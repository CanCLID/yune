#!/usr/bin/env python3
"""Verify exact membership, sizes, and SHA-256 values for a text evidence packet."""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path, PurePosixPath


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
WINDOWS_ABSOLUTE_RE = re.compile(r"^[A-Za-z]:[/\\]")


class PacketManifestError(ValueError):
    """Raised when packet membership or content does not match its manifest."""


@dataclass(frozen=True)
class PacketEntry:
    path: str
    bytes: int
    sha256: str


def _safe_relative_path(raw: str) -> str:
    if not raw or "\x00" in raw or "\\" in raw or raw.startswith("/") or WINDOWS_ABSOLUTE_RE.match(raw):
        raise PacketManifestError(f"unsafe packet path: {raw!r}")
    if any(part in {"", ".", ".."} for part in raw.split("/")):
        raise PacketManifestError(f"unsafe packet path: {raw!r}")
    parsed = PurePosixPath(raw)
    if raw in {".", ".."} or parsed.as_posix() != raw:
        raise PacketManifestError(f"unsafe packet path: {raw!r}")
    return parsed.as_posix()


def _parse_manifest(source: str, manifest_name: str) -> list[PacketEntry]:
    reader = csv.DictReader(io.StringIO(source, newline=""))
    if reader.fieldnames != ["path", "bytes", "sha256"]:
        raise PacketManifestError("manifest header must be exactly path,bytes,sha256")
    entries: list[PacketEntry] = []
    seen: set[str] = set()
    for row_number, row in enumerate(reader, start=2):
        if None in row:
            raise PacketManifestError(f"row {row_number} has extra columns")
        raw_path = row.get("path") or ""
        path = _safe_relative_path(raw_path)
        if path == manifest_name:
            raise PacketManifestError("packet manifest must not list itself")
        if path in seen:
            raise PacketManifestError(f"duplicate packet row: {path}")
        seen.add(path)
        raw_bytes = row.get("bytes") or ""
        if not raw_bytes.isdecimal() or (raw_bytes.startswith("0") and raw_bytes != "0"):
            raise PacketManifestError(f"row {row_number} has invalid byte size: {raw_bytes!r}")
        digest = row.get("sha256") or ""
        if not SHA256_RE.fullmatch(digest):
            raise PacketManifestError(f"row {row_number} has invalid SHA-256: {digest!r}")
        entries.append(PacketEntry(path, int(raw_bytes), digest))
    if not entries:
        raise PacketManifestError("packet manifest must contain at least one file")
    if [entry.path for entry in entries] != sorted(entry.path for entry in entries):
        raise PacketManifestError("packet manifest rows must be sorted by path")
    return entries


def _reject_symlink_chain(path: Path, root: Path) -> None:
    relative = path.relative_to(root)
    cursor = root
    for part in relative.parts:
        cursor /= part
        if cursor.is_symlink():
            raise PacketManifestError(f"packet contains or traverses symbolic link: {cursor}")


def _worktree_packet(manifest: Path, repo_root: Path) -> tuple[str, dict[str, bytes]]:
    try:
        resolved_manifest = manifest.resolve(strict=True)
    except FileNotFoundError as error:
        raise PacketManifestError(f"packet manifest is missing: {manifest}") from error
    if not resolved_manifest.is_file():
        raise PacketManifestError(f"packet manifest is not a file: {manifest}")
    try:
        resolved_manifest.relative_to(repo_root)
    except ValueError as error:
        raise PacketManifestError("packet manifest must resolve inside the repository") from error
    packet_root = manifest.parent
    _reject_symlink_chain(manifest, repo_root)
    files: dict[str, bytes] = {}
    for candidate in packet_root.rglob("*"):
        _reject_symlink_chain(candidate, repo_root)
        if candidate.is_file() and candidate != manifest:
            relative = candidate.relative_to(packet_root).as_posix()
            files[_safe_relative_path(relative)] = candidate.read_bytes()
    return manifest.read_text(encoding="utf-8-sig"), files


def _git(repo_root: Path, *args: str, text: bool = False) -> bytes | str:
    result = subprocess.run(
        ["git", *args],
        cwd=repo_root,
        capture_output=True,
        text=text,
        check=False,
    )
    if result.returncode != 0:
        stderr = result.stderr if text else result.stderr.decode("utf-8", errors="replace")
        raise PacketManifestError(f"git {' '.join(args)} failed: {stderr.strip()}")
    return result.stdout


def _tree_packet(manifest_relative: str, repo_root: Path, treeish: str) -> tuple[str, dict[str, bytes]]:
    manifest_relative = _safe_relative_path(manifest_relative)
    packet_prefix = PurePosixPath(manifest_relative).parent.as_posix()
    manifest_blob = _git(repo_root, "show", f"{treeish}:{manifest_relative}")
    listing = _git(repo_root, "ls-tree", "-rz", "-r", treeish, "--", packet_prefix)
    assert isinstance(manifest_blob, bytes) and isinstance(listing, bytes)
    files: dict[str, bytes] = {}
    for record in listing.split(b"\0"):
        if not record:
            continue
        metadata, raw_path = record.split(b"\t", 1)
        mode, object_type, _object_id = metadata.decode("ascii").split()
        path = raw_path.decode("utf-8")
        if mode == "120000":
            raise PacketManifestError(f"packet contains symbolic link in {treeish}: {path}")
        if object_type != "blob":
            raise PacketManifestError(
                f"packet contains non-blob tree entry in {treeish}: {path} ({object_type})"
            )
        if path == manifest_relative:
            continue
        try:
            relative = PurePosixPath(path).relative_to(PurePosixPath(packet_prefix)).as_posix()
        except ValueError as error:
            raise PacketManifestError(
                f"tree entry escapes packet prefix in {treeish}: {path}"
            ) from error
        files[_safe_relative_path(relative)] = _git(repo_root, "show", f"{treeish}:{path}")  # type: ignore[assignment]
    return manifest_blob.decode("utf-8-sig"), files


def verify_packet_manifest(
    manifest: Path, repo_root: Path, *, treeish: str | None = None
) -> tuple[int, int]:
    repo_root = repo_root.resolve(strict=True)
    manifest_path = manifest if manifest.is_absolute() else repo_root / manifest
    try:
        manifest_relative = _safe_relative_path(manifest_path.relative_to(repo_root).as_posix())
    except ValueError as error:
        raise PacketManifestError("packet manifest must be inside the repository") from error
    if treeish is None:
        source, files = _worktree_packet(manifest_path, repo_root)
    else:
        source, files = _tree_packet(manifest_relative, repo_root, treeish)
    entries = _parse_manifest(source, PurePosixPath(manifest_relative).name)
    expected = {entry.path for entry in entries}
    actual = set(files)
    if expected != actual:
        missing = sorted(actual - expected)
        extra = sorted(expected - actual)
        raise PacketManifestError(
            f"packet membership mismatch: unlisted_files={missing}, rows_without_files={extra}"
        )
    total_bytes = 0
    for entry in entries:
        data = files[entry.path]
        if len(data) != entry.bytes:
            raise PacketManifestError(
                f"{entry.path} byte mismatch: manifest={entry.bytes}, actual={len(data)}"
            )
        digest = hashlib.sha256(data).hexdigest()
        if digest != entry.sha256:
            raise PacketManifestError(
                f"{entry.path} SHA-256 mismatch: manifest={entry.sha256}, actual={digest}"
            )
        total_bytes += len(data)
    return len(entries), total_bytes


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--treeish")
    args = parser.parse_args(argv)
    try:
        files, total_bytes = verify_packet_manifest(
            args.manifest, args.repo_root, treeish=args.treeish
        )
    except (PacketManifestError, OSError, UnicodeError) as error:
        print(f"packet manifest verification failed: {error}", file=sys.stderr)
        return 1
    location = args.treeish or "working tree"
    print(f"Packet manifest verified from {location}: {files} files, {total_bytes} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
