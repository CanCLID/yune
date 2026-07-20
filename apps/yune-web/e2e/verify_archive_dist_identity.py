#!/usr/bin/env python3
"""Prove that an extracted public directory contains exactly one tar archive's files."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tarfile
from pathlib import Path, PurePosixPath


class ArchiveDistIdentityError(ValueError):
    """Raised when the archive or extracted directory is unsafe or differs."""


def _sha256_stream(stream) -> str:
    digest = hashlib.sha256()
    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
        digest.update(chunk)
    return digest.hexdigest()


def _plain_dist_files(dist_root: Path) -> dict[str, tuple[int, str]]:
    if dist_root.is_symlink() or not dist_root.is_dir():
        raise ArchiveDistIdentityError("dist root must be a plain directory")

    files: dict[str, tuple[int, str]] = {}
    pending = [(dist_root, PurePosixPath())]
    while pending:
        directory, relative_root = pending.pop()
        with os.scandir(directory) as entries:
            for entry in entries:
                relative = relative_root / entry.name
                if entry.is_symlink():
                    raise ArchiveDistIdentityError(
                        f"dist tree contains a symbolic link: {relative.as_posix()}"
                    )
                if entry.is_dir(follow_symlinks=False):
                    pending.append((Path(entry.path), relative))
                elif entry.is_file(follow_symlinks=False):
                    path = Path(entry.path)
                    with path.open("rb") as stream:
                        files[relative.as_posix()] = (
                            entry.stat(follow_symlinks=False).st_size,
                            _sha256_stream(stream),
                        )
                else:
                    raise ArchiveDistIdentityError(
                        f"dist tree contains a non-file entry: {relative.as_posix()}"
                    )
    return files


def _normalized_member(member: tarfile.TarInfo) -> str | None:
    relative = PurePosixPath(member.name)
    normalized = relative.as_posix()
    if normalized == "." and member.isdir():
        return None
    if (
        relative.is_absolute()
        or ".." in relative.parts
        or "\\" in member.name
        or not (member.isfile() or member.isdir())
        or normalized == "."
    ):
        raise ArchiveDistIdentityError(f"unsafe archive member: {member.name!r}")
    return normalized


def verify_archive_dist_identity(
    archive: Path, dist_root: Path, expected_archive_sha256: str
) -> dict[str, object]:
    if archive.is_symlink() or not archive.is_file():
        raise ArchiveDistIdentityError("archive must be a plain file")

    with archive.open("rb") as stream:
        archive_sha256 = _sha256_stream(stream)
    if archive_sha256 != expected_archive_sha256:
        raise ArchiveDistIdentityError(
            "archive changed after its certified SHA-256 was selected"
        )
    expected_files: dict[str, tuple[int, str]] = {}
    seen: set[str] = set()
    with tarfile.open(archive, "r:gz") as bundle:
        for member in bundle.getmembers():
            normalized = _normalized_member(member)
            if normalized is None:
                continue
            if normalized in seen:
                raise ArchiveDistIdentityError(
                    f"duplicate normalized archive member: {normalized}"
                )
            seen.add(normalized)
            if member.isfile():
                extracted = bundle.extractfile(member)
                if extracted is None:
                    raise ArchiveDistIdentityError(
                        f"archive file could not be read: {normalized}"
                    )
                with extracted:
                    digest = _sha256_stream(extracted)
                expected_files[normalized] = (member.size, digest)

    actual_files = _plain_dist_files(dist_root)
    if set(actual_files) != set(expected_files):
        missing = sorted(set(expected_files) - set(actual_files))
        extra = sorted(set(actual_files) - set(expected_files))
        raise ArchiveDistIdentityError(
            f"archive/dist file sets differ; missing={missing!r}, extra={extra!r}"
        )
    for relative, expected in expected_files.items():
        if actual_files[relative] != expected:
            raise ArchiveDistIdentityError(
                f"archive/dist bytes differ: {relative}"
            )
    return {
        "version": "yune-web-archive-dist-identity-v1",
        "archiveSha256": archive_sha256,
        "fileCount": len(expected_files),
        "status": "matched",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--dist", type=Path, required=True)
    parser.add_argument("--expected-archive-sha256", required=True)
    parser.add_argument("--receipt", type=Path)
    args = parser.parse_args()

    if not all(character in "0123456789abcdef" for character in args.expected_archive_sha256) or len(
        args.expected_archive_sha256
    ) != 64:
        parser.error("--expected-archive-sha256 must be a full lowercase SHA-256")
    result = verify_archive_dist_identity(
        args.archive, args.dist, args.expected_archive_sha256
    )
    payload = f"{json.dumps(result, indent=2, sort_keys=True)}\n"
    if args.receipt is not None:
        args.receipt.parent.mkdir(parents=True, exist_ok=True)
        with args.receipt.open("x", encoding="utf-8") as receipt:
            receipt.write(payload)
    print(payload, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
