#!/usr/bin/env python3
"""Validate repository-local links in an explicit list of current Markdown docs."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit


INLINE_LINK_RE = re.compile(
    r"\[[^\]]*\]\(\s*(<[^>]+>|[^\s)]+)(?:\s+['\"][^'\"]*['\"])?\s*\)"
)
REFERENCE_LINK_RE = re.compile(r"(?m)^\s*\[[^\]]+\]:\s*(<[^>]+>|\S+)")
FENCED_CODE_RE = re.compile(r"(?ms)^\s*(```|~~~).*?^\s*\1\s*$")
INLINE_CODE_RE = re.compile(r"`+[^`]*`+")
WINDOWS_ABSOLUTE_RE = re.compile(r"^[A-Za-z]:[/\\]")


class LinkAuditError(ValueError):
    """Raised when a current-document link violates the repository contract."""


def _is_within(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
    except ValueError:
        return False
    return True


def _reject_symlink_chain(path: Path, repo_root: Path, label: str) -> None:
    relative = path.relative_to(repo_root)
    cursor = repo_root
    for part in relative.parts:
        cursor /= part
        if cursor.is_symlink():
            raise LinkAuditError(f"{label} traverses symbolic link: {cursor}")


def _safe_repo_path(raw: str, repo_root: Path, label: str, *, must_be_markdown: bool) -> Path:
    if not raw or "\x00" in raw:
        raise LinkAuditError(f"{label} must be a non-empty repository-relative path")
    if "\\" in raw or raw.startswith("/") or WINDOWS_ABSOLUTE_RE.match(raw):
        raise LinkAuditError(f"{label} must be a safe repository-relative POSIX path: {raw}")
    if any(part in {"", ".", ".."} for part in raw.split("/")):
        raise LinkAuditError(f"{label} must be a normalized repository-relative path: {raw}")
    candidate = repo_root.joinpath(*raw.split("/"))
    try:
        resolved = candidate.resolve(strict=True)
    except FileNotFoundError as error:
        raise LinkAuditError(f"{label} is missing: {raw}") from error
    if not _is_within(resolved, repo_root):
        raise LinkAuditError(f"{label} escapes the repository: {raw}")
    _reject_symlink_chain(candidate, repo_root, label)
    if not resolved.is_file():
        raise LinkAuditError(f"{label} is not a file: {raw}")
    if must_be_markdown and resolved.suffix.lower() != ".md":
        raise LinkAuditError(f"{label} is not Markdown: {raw}")
    return resolved


def _markdown_targets(source: str) -> list[str]:
    without_fences = FENCED_CODE_RE.sub("", source)
    without_code = INLINE_CODE_RE.sub("", without_fences)
    return [
        *(match.group(1) for match in INLINE_LINK_RE.finditer(without_code)),
        *(match.group(1) for match in REFERENCE_LINK_RE.finditer(without_code)),
    ]


def _local_target(raw_target: str) -> str | None:
    target = raw_target[1:-1] if raw_target.startswith("<") and raw_target.endswith(">") else raw_target
    target = unquote(target)
    if target.startswith("#") or target.startswith("//"):
        return None
    if "\\" in target or target.startswith("/") or WINDOWS_ABSOLUTE_RE.match(target):
        return target
    parsed = urlsplit(target)
    if parsed.scheme:
        return None
    return parsed.path or None


def audit_current_doc_links(repo_root: Path, paths_file: Path) -> tuple[int, int]:
    repo_root = repo_root.resolve(strict=True)
    if not paths_file.is_file():
        raise LinkAuditError(f"--paths-from file is missing: {paths_file}")
    raw_paths = paths_file.read_text(encoding="utf-8-sig").splitlines()
    if not raw_paths:
        raise LinkAuditError("--paths-from must name at least one current Markdown document")
    if any(not line or line != line.strip() for line in raw_paths):
        raise LinkAuditError("--paths-from contains an empty or whitespace-padded path")
    if len(raw_paths) != len(set(raw_paths)):
        raise LinkAuditError("--paths-from contains duplicate paths")

    checked_links = 0
    for raw_doc in raw_paths:
        doc = _safe_repo_path(raw_doc, repo_root, "current document", must_be_markdown=True)
        for raw_target in _markdown_targets(doc.read_text(encoding="utf-8")):
            local = _local_target(raw_target)
            if local is None:
                continue
            if "\\" in local or local.startswith("/") or WINDOWS_ABSOLUTE_RE.match(local):
                raise LinkAuditError(f"{raw_doc} has unsafe local link target: {raw_target}")
            lexical = doc.parent.joinpath(*local.split("/"))
            try:
                resolved = lexical.resolve(strict=True)
            except FileNotFoundError as error:
                raise LinkAuditError(f"{raw_doc} has missing local target: {raw_target}") from error
            if not _is_within(resolved, repo_root):
                raise LinkAuditError(f"{raw_doc} has escaping local target: {raw_target}")
            _reject_symlink_chain(lexical, repo_root, f"link from {raw_doc}")
            checked_links += 1
    return len(raw_paths), checked_links


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--paths-from", required=True, type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    try:
        documents, links = audit_current_doc_links(args.repo_root, args.paths_from)
    except (LinkAuditError, OSError, UnicodeError) as error:
        print(f"current-document link audit failed: {error}", file=sys.stderr)
        return 1
    print(f"Current-document links verified: {documents} documents, {links} local targets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
