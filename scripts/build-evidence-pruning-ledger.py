#!/usr/bin/env python3
"""Build a deterministic, recovery-bound ledger for bulky evidence artifacts.

This tool inventories tracked benchmark artifacts. It never removes files or
changes the Git index. A ledger row may be classified ``archive-remove`` only
when the path is inside ``docs/reports/evidence`` and no current code, test, or
current-document dependency points to it.
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import os
import posixpath
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence


LEDGER_FIELDS = (
    "path",
    "git_blob_sha1",
    "byte_size",
    "packet",
    "removal_class",
    "retained_summary_pointer",
    "recovery_commit",
)
DEPENDENCY_FIELDS = (
    "candidate_path",
    "reference_path",
    "line_number",
    "reference_scope",
    "reference_kind",
    "matched_token",
    "disposition",
)
ALLOWLIST_FIELDS = (
    "path",
    "removal_class",
    "reason",
    "retained_summary_pointer",
)
RETAIN_CLASSES = frozenset(("retain-dependency", "retain-fixture"))
TARGET_BASENAMES = frozenset(
    ("m37_metrics.csv", "samples.csv", "startup_session_trace.csv")
)
PATH_TOKEN_RE = re.compile(
    r"(?P<token>(?:[A-Za-z]:)?(?:[A-Za-z0-9_ .@{}-]+[\\/])+"
    r"(?:m37_metrics[.]csv|samples[.]csv|startup_session_trace[.]csv|"
    r"[A-Za-z0-9_.{}-]+[.]marisa))"
)
GENERIC_NAME_RE = re.compile(
    r"m37_metrics[.]csv|samples[.]csv|startup_session_trace[.]csv|"
    r"[A-Za-z0-9_.{}-]+[.]marisa(?![A-Za-z0-9_])"
)
GIT_GREP_PATTERN = (
    r"(m37_metrics[.]csv|samples[.]csv|startup_session_trace[.]csv|"
    r"[[:alnum:]_.{}-]+[.]marisa([^[:alnum:]_]|$))"
)


class LedgerError(RuntimeError):
    """Raised when the proposed ledger cannot safely authorize pruning."""


@dataclass(frozen=True)
class Blob:
    oid: str
    size: int
    path: str


@dataclass(frozen=True)
class AllowlistEntry:
    path: str
    removal_class: str
    reason: str
    retained_summary_pointer: str


def _run_git(
    repo: Path, args: Sequence[str], *, check: bool = True
) -> subprocess.CompletedProcess[bytes]:
    process = subprocess.run(
        ["git", *args],
        cwd=repo,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if check and process.returncode != 0:
        stderr = process.stderr.decode("utf-8", errors="replace").strip()
        raise LedgerError(f"git {' '.join(args)} failed: {stderr}")
    return process


def _resolve_commit(repo: Path, revision: str, label: str) -> str:
    process = _run_git(repo, ["rev-parse", "--verify", f"{revision}^{{commit}}"], check=False)
    if process.returncode != 0:
        raise LedgerError(f"{label} is not a commit present in this repository: {revision}")
    commit = process.stdout.decode("ascii").strip()
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise LedgerError(f"{label} must resolve to a SHA-1 commit, got {commit!r}")
    return commit


def _resolve_tree(repo: Path, revision: str, label: str) -> str:
    process = _run_git(repo, ["rev-parse", "--verify", f"{revision}^{{tree}}"], check=False)
    if process.returncode != 0:
        raise LedgerError(f"{label} is not a Git tree-ish present in this repository: {revision}")
    tree = process.stdout.decode("ascii").strip()
    if not re.fullmatch(r"[0-9a-f]{40}", tree):
        raise LedgerError(f"{label} must resolve to a SHA-1 tree, got {tree!r}")
    return tree


def _require_sha1_repository(repo: Path) -> None:
    object_format = _run_git(repo, ["rev-parse", "--show-object-format"]).stdout.decode(
        "ascii"
    ).strip()
    if object_format != "sha1":
        raise LedgerError(
            f"the archive contract requires Git SHA-1 blob ids, repository uses {object_format}"
        )


def _tree_blobs(repo: Path, commit: str) -> dict[str, Blob]:
    output = _run_git(repo, ["ls-tree", "-lr", "-z", commit]).stdout
    blobs: dict[str, Blob] = {}
    for raw_record in output.split(b"\0"):
        if not raw_record:
            continue
        metadata, separator, raw_path = raw_record.partition(b"\t")
        if not separator:
            raise LedgerError("unexpected git ls-tree record without a path separator")
        parts = metadata.decode("ascii").split()
        if len(parts) != 4 or parts[1] != "blob":
            continue
        _, _, oid, raw_size = parts
        path = raw_path.decode("utf-8")
        if path in blobs:
            raise LedgerError(f"duplicate tracked path in Git tree: {path}")
        if not re.fullmatch(r"[0-9a-f]{40}", oid):
            raise LedgerError(f"non-SHA-1 blob id for {path}: {oid}")
        blobs[path] = Blob(oid=oid, size=int(raw_size), path=path)
    return blobs


def _is_candidate(path: str) -> bool:
    basename = path.rsplit("/", 1)[-1]
    return basename in TARGET_BASENAMES or basename.endswith(".marisa")


def _packet(path: str) -> str:
    parts = path.split("/")
    evidence_prefix = ("docs", "reports", "evidence")
    results_prefix = ("apps", "yune-web", "e2e", "results")
    if tuple(parts[:3]) == evidence_prefix and len(parts) > 3:
        return parts[3]
    if tuple(parts[:4]) == results_prefix and len(parts) > 4:
        return parts[4]
    return "/".join(parts[: min(3, len(parts))])


def _read_allowlist(path: Path) -> dict[str, AllowlistEntry]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as error:
        raise LedgerError(f"cannot read allowlist {path}: {error}") from error
    reader = csv.DictReader(io.StringIO(text))
    if tuple(reader.fieldnames or ()) != ALLOWLIST_FIELDS:
        raise LedgerError(
            f"allowlist columns must be {','.join(ALLOWLIST_FIELDS)} in that order"
        )
    entries: dict[str, AllowlistEntry] = {}
    observed_order: list[str] = []
    for line_number, row in enumerate(reader, start=2):
        candidate_path = row["path"].strip()
        removal_class = row["removal_class"].strip()
        reason = row["reason"].strip()
        summary = row["retained_summary_pointer"].strip()
        if not candidate_path or candidate_path.startswith("/"):
            raise LedgerError(f"allowlist line {line_number} has an invalid repository path")
        if candidate_path in entries:
            raise LedgerError(f"duplicate allowlist path: {candidate_path}")
        if removal_class not in RETAIN_CLASSES:
            raise LedgerError(
                f"allowlist path {candidate_path} has unsupported class {removal_class!r}"
            )
        if not reason:
            raise LedgerError(f"allowlist path {candidate_path} requires a reason")
        entries[candidate_path] = AllowlistEntry(
            path=candidate_path,
            removal_class=removal_class,
            reason=reason,
            retained_summary_pointer=summary,
        )
        observed_order.append(candidate_path)
    if observed_order != sorted(observed_order):
        raise LedgerError("allowlist paths must be sorted lexicographically")
    return entries


def _reference_scope(path: str) -> str:
    if path.startswith("docs/reports/evidence/"):
        basename = path.rsplit("/", 1)[-1].lower()
        if path.lower().endswith(".md"):
            return "evidence-doc"
        if "manifest" in basename or basename.endswith((".sha256", ".sha256sum")):
            return "evidence-manifest"
        return "evidence-artifact"
    if path.startswith("docs/plans/completed/") or path.startswith("docs/reports/history/"):
        return "historical-doc"
    if path.startswith("docs/"):
        return "current-doc"
    if path.startswith(("apps/", "crates/", "packages/", "scripts/")):
        if "/results/" in path:
            return "historical-artifact"
        return "code-test"
    return "repo-config"


def _candidate_matches(
    token: str, reference_path: str, candidates: dict[str, Blob]
) -> tuple[str, list[str]]:
    normalized_token = posixpath.normpath(token.replace("\\", "/"))
    for repository_prefix in (
        "docs/reports/evidence/",
        "apps/yune-web/e2e/results/",
    ):
        prefix_offset = normalized_token.find(repository_prefix)
        if prefix_offset >= 0:
            repository_path = normalized_token[prefix_offset:]
            if repository_path in candidates:
                return "embedded-repository-path", [repository_path]
    if normalized_token in candidates:
        return "exact-repository-path", [normalized_token]
    relative = posixpath.normpath(
        posixpath.join(posixpath.dirname(reference_path), normalized_token)
    )
    if relative in candidates:
        return "relative-repository-path", [relative]
    suffix = normalized_token.lstrip("./")
    if "/" not in suffix:
        return "generic-output-name", []
    candidate_pool: Iterable[str] = candidates
    if reference_path.startswith("docs/reports/evidence/"):
        reference_parts = reference_path.split("/")
        if len(reference_parts) > 3:
            packet_prefix = "/".join(reference_parts[:4]) + "/"
            packet_candidates = [
                path for path in candidates if path.startswith(packet_prefix)
            ]
            if packet_candidates:
                candidate_pool = packet_candidates
    matches = sorted(path for path in candidate_pool if path.endswith(f"/{suffix}"))
    if len(matches) == 1:
        return "unique-path-suffix", matches
    if matches:
        # A generic output shape such as ``yune/results/samples.csv`` may match
        # many historical packets.  It is not a dependency on every matching
        # leaf; keep it informational until a repository path is unambiguous.
        return "ambiguous-path-suffix", []
    return "unresolved-path-reference", []


def _dependency_rows(
    repo: Path,
    commit: str,
    tracked_paths: Iterable[str],
    candidates: dict[str, Blob],
    allowlist: dict[str, AllowlistEntry],
) -> tuple[list[dict[str, str]], list[str]]:
    scan_paths = sorted(
        path
        for path in tracked_paths
        if path not in candidates
        and not path.startswith("docs/ledgers/evidence-pruning/")
    )
    grep_lines: list[str] = []
    for offset in range(0, len(scan_paths), 250):
        process = _run_git(
            repo,
            [
                "grep",
                "-n",
                "-I",
                "-E",
                GIT_GREP_PATTERN,
                commit,
                "--",
                *scan_paths[offset : offset + 250],
            ],
            check=False,
        )
        if process.returncode not in (0, 1):
            stderr = process.stderr.decode("utf-8", errors="replace").strip()
            raise LedgerError(f"git grep dependency scan failed: {stderr}")
        grep_lines.extend(process.stdout.decode("utf-8").splitlines())
    rows: list[dict[str, str]] = []
    blockers: list[str] = []
    prefix = f"{commit}:"
    for raw_line in grep_lines:
        if not raw_line.startswith(prefix):
            raise LedgerError(f"unexpected git grep result: {raw_line!r}")
        try:
            reference_path, raw_line_number, content = raw_line[len(prefix) :].split(":", 2)
        except ValueError as error:
            raise LedgerError(f"cannot parse git grep result: {raw_line!r}") from error
        if reference_path in candidates:
            continue
        scope = _reference_scope(reference_path)
        token_matches = list(PATH_TOKEN_RE.finditer(content))
        covered_names: set[str] = set()
        for token_match in token_matches:
            token = token_match.group("token")
            covered_names.add(token.rsplit("/", 1)[-1])
            kind, matched_candidates = _candidate_matches(token, reference_path, candidates)
            if not matched_candidates:
                rows.append(
                    {
                        "candidate_path": "",
                        "reference_path": reference_path,
                        "line_number": raw_line_number,
                        "reference_scope": scope,
                        "reference_kind": kind,
                        "matched_token": token,
                        "disposition": "informational",
                    }
                )
                continue
            for candidate_path in matched_candidates:
                if scope in ("historical-doc", "historical-artifact"):
                    disposition = "historical-reference"
                elif candidate_path in allowlist:
                    disposition = "retained-allowlist"
                elif scope in ("current-doc", "evidence-doc"):
                    disposition = "redirect-required"
                    blockers.append(
                        f"{candidate_path} linked by {reference_path}:{raw_line_number}"
                    )
                elif scope == "evidence-manifest":
                    disposition = "historical-manifest-reference"
                elif scope == "evidence-artifact":
                    disposition = "retained-artifact-recovery-reference"
                else:
                    disposition = "blocking-unallowlisted"
                    blockers.append(
                        f"{candidate_path} referenced by {reference_path}:{raw_line_number}"
                    )
                rows.append(
                    {
                        "candidate_path": candidate_path,
                        "reference_path": reference_path,
                        "line_number": raw_line_number,
                        "reference_scope": scope,
                        "reference_kind": kind,
                        "matched_token": token,
                        "disposition": disposition,
                    }
                )
        for generic_match in GENERIC_NAME_RE.finditer(content):
            generic_name = generic_match.group(0)
            if generic_name in covered_names:
                continue
            rows.append(
                {
                    "candidate_path": "",
                    "reference_path": reference_path,
                    "line_number": raw_line_number,
                    "reference_scope": scope,
                    "reference_kind": "generic-output-name",
                    "matched_token": generic_name,
                    "disposition": "informational",
                }
            )
    rows = list(
        {
            tuple(row[field] for field in DEPENDENCY_FIELDS): row
            for row in rows
        }.values()
    )
    rows.sort(
        key=lambda row: (
            row["candidate_path"],
            row["reference_path"],
            int(row["line_number"]),
            row["matched_token"],
            row["reference_kind"],
        )
    )
    return rows, sorted(set(blockers))


def _csv_bytes(fields: Sequence[str], rows: Iterable[dict[str, object]]) -> bytes:
    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return output.getvalue().encode("utf-8")


def _write_or_check(path: Path, content: bytes, check: bool) -> None:
    if check:
        try:
            existing = path.read_bytes()
        except OSError as error:
            raise LedgerError(f"cannot check missing output {path}: {error}") from error
        if existing != content:
            raise LedgerError(f"generated output is stale: {path}")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    temporary.write_bytes(content)
    temporary.replace(path)


def _repo_path(repo: Path, path: Path) -> Path:
    return path if path.is_absolute() else repo / path


def build(args: argparse.Namespace) -> dict[str, int | str]:
    repo = args.repo_root.resolve()
    _require_sha1_repository(repo)
    tree_commit = _resolve_commit(repo, args.treeish, "treeish")
    dependency_tree = _resolve_tree(
        repo,
        args.dependency_treeish or tree_commit,
        "dependency treeish",
    )
    recovery_commit = _resolve_commit(repo, args.recovery_commit, "recovery commit")
    tree = _tree_blobs(repo, tree_commit)
    recovery_tree = _tree_blobs(repo, recovery_commit)
    candidates = {path: blob for path, blob in tree.items() if _is_candidate(path)}
    if not candidates:
        raise LedgerError("no tracked pruning candidates matched the inventory contract")
    allowlist = _read_allowlist(_repo_path(repo, args.allowlist))
    stale_allowlist = sorted(set(allowlist) - set(candidates))
    if stale_allowlist:
        raise LedgerError(f"allowlist paths are not tracked candidates: {stale_allowlist}")
    outside_scope = sorted(
        path
        for path in candidates
        if not path.startswith("docs/reports/evidence/") and path not in allowlist
    )
    if outside_scope:
        raise LedgerError(
            "candidate paths outside docs/reports/evidence require an explicit allowlist: "
            + ", ".join(outside_scope)
        )
    summary_path = args.retained_summary
    if summary_path not in tree:
        raise LedgerError(f"retained summary pointer is not a tracked blob at treeish: {summary_path}")
    if summary_path in candidates and summary_path not in allowlist:
        raise LedgerError(f"retained summary pointer is scheduled for archive removal: {summary_path}")
    for entry in allowlist.values():
        pointer = entry.retained_summary_pointer or summary_path
        if pointer not in tree:
            raise LedgerError(
                f"allowlist summary pointer for {entry.path} is not tracked: {pointer}"
            )
        if pointer in candidates and pointer not in allowlist:
            raise LedgerError(
                f"allowlist summary pointer for {entry.path} is scheduled for archive removal: {pointer}"
            )
    recovery_failures: list[str] = []
    for candidate_path, blob in candidates.items():
        recovery_blob = recovery_tree.get(candidate_path)
        if recovery_blob is None:
            recovery_failures.append(f"missing {candidate_path}")
        elif (recovery_blob.oid, recovery_blob.size) != (blob.oid, blob.size):
            recovery_failures.append(f"blob mismatch {candidate_path}")
    if recovery_failures:
        raise LedgerError(
            "recovery commit cannot restore the inventoried bytes: "
            + "; ".join(recovery_failures[:20])
        )
    dependency_rows, blockers = _dependency_rows(
        repo,
        dependency_tree,
        _tree_blobs(repo, dependency_tree),
        candidates,
        allowlist,
    )
    if blockers:
        raise LedgerError(
            "current dependencies require retain-dependency allowlist entries: "
            + "; ".join(blockers[:20])
        )
    ledger_rows: list[dict[str, object]] = []
    for candidate_path in sorted(candidates):
        blob = candidates[candidate_path]
        entry = allowlist.get(candidate_path)
        removal_class = entry.removal_class if entry else "archive-remove"
        pointer = (
            entry.retained_summary_pointer
            if entry and entry.retained_summary_pointer
            else summary_path
        )
        ledger_rows.append(
            {
                "path": candidate_path,
                "git_blob_sha1": blob.oid,
                "byte_size": blob.size,
                "packet": _packet(candidate_path),
                "removal_class": removal_class,
                "retained_summary_pointer": pointer,
                "recovery_commit": recovery_commit,
            }
        )
    ledger_content = _csv_bytes(LEDGER_FIELDS, ledger_rows)
    dependency_content = _csv_bytes(DEPENDENCY_FIELDS, dependency_rows)
    _write_or_check(_repo_path(repo, args.ledger_out), ledger_content, args.check)
    _write_or_check(
        _repo_path(repo, args.dependency_out), dependency_content, args.check
    )
    archive_rows = [row for row in ledger_rows if row["removal_class"] == "archive-remove"]
    return {
        "tree_commit": tree_commit,
        "dependency_tree": dependency_tree,
        "recovery_commit": recovery_commit,
        "ledger_rows": len(ledger_rows),
        "archive_remove_rows": len(archive_rows),
        "archive_remove_bytes": sum(int(row["byte_size"]) for row in archive_rows),
        "retained_rows": len(ledger_rows) - len(archive_rows),
        "dependency_rows": len(dependency_rows),
    }


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--treeish", default="HEAD")
    parser.add_argument(
        "--dependency-treeish",
        help=(
            "optional post-pruning tree-ish used for dependency scanning; "
            "pass git write-tree before commit and HEAD after commit"
        ),
    )
    parser.add_argument("--recovery-commit", required=True)
    parser.add_argument("--allowlist", type=Path, required=True)
    parser.add_argument("--retained-summary", required=True)
    parser.add_argument("--ledger-out", type=Path, required=True)
    parser.add_argument("--dependency-out", type=Path, required=True)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail unless the existing outputs exactly match regenerated bytes",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        result = build(args)
    except LedgerError as error:
        print(f"evidence pruning ledger failed: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
