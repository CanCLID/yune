#!/usr/bin/env python3
"""Shared safety policy for external benchmark output and compact evidence.

Raw benchmark output belongs outside Git worktrees.  A deliberately small,
explicitly allowlisted subset can later be curated into ``docs/reports/evidence``.
This module owns the common path, curation, and growth-policy implementation;
the adjacent command-line scripts are intentionally thin entry points.
"""

from __future__ import annotations

import csv
import hashlib
import io
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Iterable, Sequence


EVIDENCE_PREFIX = PurePosixPath("docs/reports/evidence")
DEFAULT_EVIDENCE_HOME = PurePosixPath(".yune/evidence")
MAX_NEW_FILE_BYTES = 5 * 1024 * 1024
MAX_CURATED_PACKET_BYTES = 10 * 1024 * 1024
DEFAULT_MANIFEST_NAME = "packet-manifest.csv"
RAW_EVIDENCE_BASENAMES = frozenset(
    {"m37_metrics.csv", "samples.csv", "startup_session_trace.csv"}
)
KIND_PATTERN = re.compile(r"^[a-z0-9][a-z0-9-]*$")


class EvidencePolicyError(ValueError):
    """Raised when a path or packet violates the evidence retention policy."""


def _run_git(repo_root: Path, arguments: Sequence[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo_root), *arguments],
        text=True,
        capture_output=True,
        check=False,
    )


def _canonical(path: Path) -> Path:
    try:
        return path.expanduser().resolve(strict=False)
    except (OSError, RuntimeError) as error:
        raise EvidencePolicyError(f"cannot resolve path {path}: {error}") from error


def _paths_overlap(first: Path, second: Path) -> bool:
    try:
        common = Path(os.path.commonpath((str(first), str(second))))
    except ValueError:
        return False
    return common == first or common == second


def _nearest_existing_directory(path: Path) -> Path:
    probe = path
    while not probe.exists():
        parent = probe.parent
        if parent == probe:
            raise EvidencePolicyError(f"path has no existing ancestor: {path}")
        probe = parent
    if not probe.is_dir():
        probe = probe.parent
    return probe


def git_worktree_roots(repo_root: Path) -> tuple[Path, ...]:
    """Return every worktree registered by the repository at ``repo_root``."""

    repository = _canonical(repo_root)
    completed = _run_git(repository, ["worktree", "list", "--porcelain"])
    if completed.returncode != 0:
        raise EvidencePolicyError(
            f"cannot enumerate Git worktrees from {repository}: "
            f"{completed.stderr.strip() or completed.stdout.strip()}"
        )
    roots = []
    for line in completed.stdout.splitlines():
        if line.startswith("worktree "):
            roots.append(_canonical(Path(line.removeprefix("worktree "))))
    if not roots:
        raise EvidencePolicyError(f"Git reported no worktrees for {repository}")
    return tuple(dict.fromkeys(roots))


def validate_external_output_path(repo_root: Path, candidate: Path) -> Path:
    """Resolve ``candidate`` and reject overlap with any tracked Git worktree."""

    if any(character in str(candidate) for character in ("\0", "\r", "\n")):
        raise EvidencePolicyError("output path must not contain control characters")
    resolved = _canonical(candidate)
    if resolved == Path(resolved.anchor):
        raise EvidencePolicyError(f"refusing to use a filesystem root as output: {resolved}")

    known_roots = git_worktree_roots(repo_root)
    for root in known_roots:
        if _paths_overlap(resolved, root):
            raise EvidencePolicyError(
                f"benchmark output must be disjoint from Git worktree {root}: {resolved}"
            )

    # Also reject a destination in an unrelated Git worktree.  Looking up from
    # the nearest existing ancestor covers a not-yet-created output directory.
    ancestor = _nearest_existing_directory(resolved)
    containing = _run_git(ancestor, ["rev-parse", "--show-toplevel"])
    if containing.returncode == 0:
        containing_root = _canonical(Path(containing.stdout.strip()))
        if _paths_overlap(resolved, containing_root):
            raise EvidencePolicyError(
                "benchmark output must be disjoint from containing Git worktree "
                f"{containing_root}: {resolved}"
            )
    return resolved


def default_external_output_path(
    kind: str,
    *,
    home: Path | None = None,
    timestamp: str | None = None,
    environment: dict[str, str] | None = None,
) -> Path:
    """Return a unique user-level raw-evidence destination for ``kind``."""

    if not KIND_PATTERN.fullmatch(kind):
        raise EvidencePolicyError(f"invalid evidence kind: {kind!r}")
    env = os.environ if environment is None else environment
    configured = env.get("YUNE_EVIDENCE_ROOT", "").strip()
    base = Path(configured).expanduser() if configured else (home or Path.home()) / DEFAULT_EVIDENCE_HOME
    stamp = timestamp or datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    if not re.fullmatch(r"[0-9]{8}T[0-9]{6}Z", stamp):
        raise EvidencePolicyError(f"invalid UTC evidence timestamp: {stamp!r}")
    return _canonical(base / kind / stamp)


def normalize_relative_path(value: str, *, label: str) -> PurePosixPath:
    if not value or "\\" in value or any(character in value for character in ("\0", "\r", "\n")):
        raise EvidencePolicyError(f"{label} must be a non-empty POSIX relative path: {value!r}")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise EvidencePolicyError(f"{label} must not be absolute or traverse: {value!r}")
    return path


def read_explicit_path_list(path: Path, *, label: str) -> tuple[PurePosixPath, ...]:
    entries: list[PurePosixPath] = []
    seen: set[PurePosixPath] = set()
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        entry = normalize_relative_path(line, label=f"{label} line {line_number}")
        if entry in seen:
            raise EvidencePolicyError(f"duplicate {label} entry: {entry.as_posix()}")
        seen.add(entry)
        entries.append(entry)
    if not entries and label == "allowlist":
        raise EvidencePolicyError("allowlist must name at least one file")
    return tuple(entries)


def _assert_plain_source_file(source_root: Path, relative: PurePosixPath) -> Path:
    probe = source_root
    if probe.is_symlink():
        raise EvidencePolicyError(f"source root must not be a symlink: {source_root}")
    for part in relative.parts:
        probe = probe / part
        if probe.is_symlink():
            raise EvidencePolicyError(f"allowlisted source traverses a symlink: {probe}")
    if not probe.is_file():
        raise EvidencePolicyError(f"allowlisted source is not a regular file: {probe}")
    return probe


def _assert_no_casefold_collisions(
    paths: Iterable[PurePosixPath], *, label: str
) -> None:
    observed: dict[str, PurePosixPath] = {}
    for path in paths:
        key = path.as_posix().casefold()
        previous = observed.get(key)
        if previous is not None and previous != path:
            raise EvidencePolicyError(
                f"{label} paths collide on case-insensitive filesystems: "
                f"{previous.as_posix()} / {path.as_posix()}"
            )
        observed[key] = path


def _assert_no_symlink_components(root: Path, relative: PurePosixPath) -> Path:
    probe = root
    for part in relative.parts:
        probe = probe / part
        if probe.is_symlink():
            raise EvidencePolicyError(f"evidence path traverses a symlink: {probe}")
    return probe


def _assert_absolute_path_has_no_symlink_components(path: Path, *, label: str) -> None:
    absolute = path.absolute()
    probe = Path(absolute.anchor)
    platform_root_aliases = (
        {Path("/etc"), Path("/tmp"), Path("/var")}
        if sys.platform == "darwin"
        else set()
    )
    for part in absolute.parts[1:]:
        probe = probe / part
        if probe.is_symlink():
            # macOS exposes these top-level system directories through stable
            # aliases into /private.  They are part of the platform path model,
            # not packet-controlled traversal.  Any deeper symlink still fails.
            if probe in platform_root_aliases:
                continue
            raise EvidencePolicyError(f"{label} traverses a symlink: {probe}")


def _manifest_bytes(rows: Sequence[tuple[str, int, str]]) -> bytes:
    output = io.StringIO(newline="")
    writer = csv.writer(output, lineterminator="\n")
    writer.writerow(("path", "size_bytes", "sha256"))
    writer.writerows(rows)
    return output.getvalue().encode("utf-8")


def curate_compact_evidence(
    source_root: Path,
    output_root: Path,
    allowlist_path: Path,
    *,
    manifest_name: str = DEFAULT_MANIFEST_NAME,
    fixture_exceptions: Iterable[PurePosixPath] = (),
) -> tuple[tuple[str, int, str], ...]:
    """Copy an exact allowlist into a create-new packet and bind it by SHA-256."""

    source_input = source_root.expanduser()
    _assert_absolute_path_has_no_symlink_components(source_input, label="source root")
    source = _canonical(source_input)
    output = _canonical(output_root)
    if not source.is_dir() or source.is_symlink():
        raise EvidencePolicyError(f"source root must be a plain directory: {source}")
    if output.exists():
        raise EvidencePolicyError(f"output packet must be a new path: {output}")
    if _paths_overlap(source, output):
        raise EvidencePolicyError(f"source and output roots must be disjoint: {source} / {output}")

    manifest_relative = normalize_relative_path(manifest_name, label="manifest name")
    allowlist = read_explicit_path_list(allowlist_path, label="allowlist")
    _assert_no_casefold_collisions(allowlist, label="allowlist")
    if any(
        entry.as_posix().casefold() == manifest_relative.as_posix().casefold()
        for entry in allowlist
    ):
        raise EvidencePolicyError("allowlist must not include the generated manifest")
    exceptions = set(fixture_exceptions)

    selected: list[tuple[PurePosixPath, bytes]] = []
    copied_bytes = 0
    for relative in sorted(allowlist, key=lambda item: item.as_posix()):
        if is_raw_evidence_path(relative):
            raise EvidencePolicyError(
                f"compact curator refuses raw evidence class: {relative.as_posix()}"
            )
        source_path = _assert_plain_source_file(source, relative)
        payload = source_path.read_bytes()
        if len(payload) > MAX_NEW_FILE_BYTES and relative not in exceptions:
            raise EvidencePolicyError(
                "compact curator file exceeds 5 MiB without fixture exception: "
                f"{relative.as_posix()} ({len(payload)} bytes)"
            )
        copied_bytes += len(payload)
        selected.append((relative, payload))

    rows = tuple(
        (
            relative.as_posix(),
            len(payload),
            hashlib.sha256(payload).hexdigest(),
        )
        for relative, payload in selected
    )
    manifest = _manifest_bytes(rows)
    if copied_bytes + len(manifest) > MAX_CURATED_PACKET_BYTES:
        raise EvidencePolicyError(
            "curated packet exceeds 10 MiB cap: "
            f"{copied_bytes + len(manifest)} bytes"
        )

    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix=f".{output.name}.tmp-", dir=output.parent))
    try:
        for relative, payload in selected:
            destination = temporary.joinpath(*relative.parts)
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(payload)
        manifest_path = temporary.joinpath(*manifest_relative.parts)
        manifest_path.parent.mkdir(parents=True, exist_ok=True)
        manifest_path.write_bytes(manifest)
        if output.exists():
            raise EvidencePolicyError(f"output packet appeared during curation: {output}")
        temporary.rename(output)
    except BaseException:
        if temporary.exists():
            shutil.rmtree(temporary)
        raise
    return rows


def is_raw_evidence_path(path: PurePosixPath) -> bool:
    basename = path.name.casefold()
    return basename in RAW_EVIDENCE_BASENAMES or basename.endswith(".marisa")


@dataclass(frozen=True)
class AddedEvidenceFile:
    path: PurePosixPath
    size_bytes: int
    git_mode: str = "100644"


def validate_evidence_growth(
    added_files: Iterable[AddedEvidenceFile],
    fixture_exceptions: Iterable[PurePosixPath] = (),
    post_change_packet_sizes: dict[str, int] | None = None,
) -> tuple[AddedEvidenceFile, ...]:
    """Fail closed on raw, oversized, or collectively oversized new evidence."""

    exceptions = set(fixture_exceptions)
    accepted: list[AddedEvidenceFile] = []
    packet_sizes: dict[str, int] = {}
    errors: list[str] = []
    for record in added_files:
        path = record.path
        if tuple(path.parts[:3]) != tuple(EVIDENCE_PREFIX.parts):
            continue
        if len(path.parts) < 4:
            errors.append(f"evidence file must belong to a named packet: {path}")
            continue
        if record.size_bytes < 0:
            errors.append(f"negative file size for {path}: {record.size_bytes}")
            continue
        if record.git_mode not in {"100644", "100755"}:
            errors.append(
                f"evidence path must be a regular Git file, not mode "
                f"{record.git_mode}: {path}"
            )
        if is_raw_evidence_path(path):
            errors.append(f"new raw evidence class is prohibited: {path}")
        if record.size_bytes > MAX_NEW_FILE_BYTES and path not in exceptions:
            errors.append(
                f"new evidence file exceeds 5 MiB without fixture exception: "
                f"{path} ({record.size_bytes} bytes)"
            )
        packet = path.parts[3]
        packet_sizes[packet] = packet_sizes.get(packet, 0) + record.size_bytes
        accepted.append(record)

    sizes_to_check = post_change_packet_sizes or packet_sizes
    for packet, size in sorted(sizes_to_check.items()):
        if size > MAX_CURATED_PACKET_BYTES:
            errors.append(
                f"post-change curated packet exceeds 10 MiB cap: {packet} ({size} bytes)"
            )
    if errors:
        raise EvidencePolicyError("\n".join(errors))
    return tuple(accepted)


def staged_changed_evidence(repo_root: Path) -> tuple[AddedEvidenceFile, ...]:
    """Return added, copied, modified, and renamed staged evidence destinations."""

    repository = _canonical(repo_root)
    completed = subprocess.run(
        [
            "git",
            "-C",
            str(repository),
            "diff",
            "--cached",
            "--diff-filter=ACMRT",
            "--name-only",
            "-z",
            "--",
            EVIDENCE_PREFIX.as_posix(),
        ],
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise EvidencePolicyError(
            f"cannot inspect staged evidence changes: {completed.stderr.decode(errors='replace').strip()}"
        )
    records: list[AddedEvidenceFile] = []
    for raw_path in completed.stdout.split(b"\0"):
        if not raw_path:
            continue
        value = raw_path.decode("utf-8", errors="strict")
        relative = normalize_relative_path(value, label="staged evidence path")
        size = _run_git(repository, ["cat-file", "-s", f":{relative.as_posix()}"])
        if size.returncode != 0:
            raise EvidencePolicyError(
                f"cannot read staged blob size for {relative}: {size.stderr.strip()}"
            )
        indexed = _run_git(repository, ["ls-files", "-s", "--", relative.as_posix()])
        if indexed.returncode != 0 or not indexed.stdout.strip():
            raise EvidencePolicyError(f"cannot read staged Git mode for {relative}")
        mode = indexed.stdout.split(maxsplit=1)[0]
        records.append(AddedEvidenceFile(relative, int(size.stdout.strip()), mode))
    return tuple(records)


# Compatibility alias for callers written before modifications and renames were
# included.  The implementation intentionally returns every staged destination
# that can grow or enter a prohibited raw class.
staged_added_evidence = staged_changed_evidence


def tree_changed_evidence(
    repo_root: Path, base: str, treeish: str = "HEAD"
) -> tuple[AddedEvidenceFile, ...]:
    """Return evidence destinations added, copied, modified, or renamed in a tree diff."""

    repository = _canonical(repo_root)
    completed = subprocess.run(
        [
            "git",
            "-C",
            str(repository),
            "diff",
            "--diff-filter=ACMRT",
            "--name-only",
            "-z",
            base,
            treeish,
            "--",
            EVIDENCE_PREFIX.as_posix(),
        ],
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise EvidencePolicyError(
            "cannot inspect committed evidence changes: "
            f"{completed.stderr.decode(errors='replace').strip()}"
        )
    records: list[AddedEvidenceFile] = []
    for raw_path in completed.stdout.split(b"\0"):
        if not raw_path:
            continue
        value = raw_path.decode("utf-8", errors="strict")
        relative = normalize_relative_path(value, label="committed evidence path")
        size = _run_git(
            repository,
            ["cat-file", "-s", f"{treeish}:{relative.as_posix()}"],
        )
        if size.returncode != 0:
            raise EvidencePolicyError(
                f"cannot read committed blob size for {relative}: {size.stderr.strip()}"
            )
        listed = _run_git(
            repository,
            ["ls-tree", treeish, "--", relative.as_posix()],
        )
        if listed.returncode != 0 or not listed.stdout.strip():
            raise EvidencePolicyError(f"cannot read committed Git mode for {relative}")
        mode = listed.stdout.split(maxsplit=1)[0]
        records.append(AddedEvidenceFile(relative, int(size.stdout.strip()), mode))
    return tuple(records)


def _packet_names(records: Iterable[AddedEvidenceFile]) -> tuple[str, ...]:
    return tuple(
        sorted(
            {
                record.path.parts[3]
                for record in records
                if tuple(record.path.parts[:3]) == tuple(EVIDENCE_PREFIX.parts)
                and len(record.path.parts) >= 4
            }
        )
    )


def staged_packet_sizes(
    repo_root: Path,
    records: Iterable[AddedEvidenceFile],
) -> dict[str, int]:
    """Measure complete touched packets from the post-change Git index."""

    repository = _canonical(repo_root)
    totals: dict[str, int] = {}
    for packet in _packet_names(records):
        prefix = f"{EVIDENCE_PREFIX.as_posix()}/{packet}"
        listed = subprocess.run(
            ["git", "-C", str(repository), "ls-files", "-z", "--", prefix],
            capture_output=True,
            check=False,
        )
        if listed.returncode != 0:
            raise EvidencePolicyError(
                f"cannot list indexed packet {packet}: {listed.stderr.decode(errors='replace').strip()}"
            )
        total = 0
        for raw_path in listed.stdout.split(b"\0"):
            if not raw_path:
                continue
            path = raw_path.decode("utf-8", errors="strict")
            size = _run_git(repository, ["cat-file", "-s", f":{path}"])
            if size.returncode != 0:
                raise EvidencePolicyError(
                    f"cannot read indexed blob size for {path}: {size.stderr.strip()}"
                )
            total += int(size.stdout.strip())
        totals[packet] = total
    return totals


def tree_packet_sizes(
    repo_root: Path,
    records: Iterable[AddedEvidenceFile],
    treeish: str = "HEAD",
) -> dict[str, int]:
    """Measure complete touched packets from a committed Git tree."""

    repository = _canonical(repo_root)
    totals: dict[str, int] = {}
    for packet in _packet_names(records):
        prefix = f"{EVIDENCE_PREFIX.as_posix()}/{packet}"
        listed = subprocess.run(
            ["git", "-C", str(repository), "ls-tree", "-lr", "-z", treeish, "--", prefix],
            capture_output=True,
            check=False,
        )
        if listed.returncode != 0:
            raise EvidencePolicyError(
                f"cannot list committed packet {packet}: "
                f"{listed.stderr.decode(errors='replace').strip()}"
            )
        total = 0
        for raw_record in listed.stdout.split(b"\0"):
            if not raw_record:
                continue
            metadata, separator, _raw_path = raw_record.partition(b"\t")
            if not separator:
                raise EvidencePolicyError(
                    f"unexpected Git tree record while measuring packet {packet}"
                )
            fields = metadata.split()
            if len(fields) != 4 or fields[1] != b"blob":
                continue
            total += int(fields[3])
        totals[packet] = total
    return totals


def working_tree_packet_sizes(
    repo_root: Path,
    records: Iterable[AddedEvidenceFile],
) -> dict[str, int]:
    """Measure complete touched packets for explicit working-tree validation."""

    repository = _canonical(repo_root)
    totals: dict[str, int] = {}
    for packet in _packet_names(records):
        packet_root = repository.joinpath(*EVIDENCE_PREFIX.parts, packet)
        packet_relative = PurePosixPath(*EVIDENCE_PREFIX.parts, packet)
        _assert_no_symlink_components(repository, packet_relative)
        total = 0
        if packet_root.is_dir():
            for path in packet_root.rglob("*"):
                if path.is_symlink():
                    raise EvidencePolicyError(f"curated packet contains a symlink: {path}")
                if path.is_file():
                    total += path.stat().st_size
        totals[packet] = total
    return totals


def files_from_paths(repo_root: Path, paths: Iterable[str]) -> tuple[AddedEvidenceFile, ...]:
    repository = _canonical(repo_root)
    records: list[AddedEvidenceFile] = []
    for value in paths:
        relative = normalize_relative_path(value.strip(), label="evidence path")
        full = _assert_no_symlink_components(repository, relative)
        if not full.is_file():
            raise EvidencePolicyError(f"evidence path is not a plain file: {relative}")
        records.append(AddedEvidenceFile(relative, full.stat().st_size))
    return tuple(records)
