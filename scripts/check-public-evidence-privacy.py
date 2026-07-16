#!/usr/bin/env python3
"""Fail closed when curated public evidence contains private machine data."""

from __future__ import annotations

import argparse
import re
import sys
import unicodedata
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence
from urllib.parse import unquote


REPO_ROOT = Path(__file__).resolve().parents[1]
TOOL_VERSION = "1"

GENERIC_PATTERNS = (
    (
        "user_profile_path",
        re.compile(
            r"(?i)(?:"
            r"\b[a-z]:[\\/](?:users|documents[ ]and[ ]settings)[\\/]"
            r"[^\\/\s\"'<>:]+"
            r"|/(?:users|home)/[^/\s\"'<>:]+"
            r")"
        ),
    ),
    (
        "email",
        re.compile(
            r"(?i)(?<![a-z0-9.!#$%&'*+/=?^_`{|}~-])"
            r"[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@"
            r"[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?"
            r"(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)+"
        ),
    ),
    (
        "secret_assignment",
        re.compile(
            r"(?i)\b(?:api[_-]?key|access[_-]?token|auth[_-]?token|"
            r"refresh[_-]?token|client[_-]?secret|secret|password|passwd|"
            r"authorization)\b\s*[:=]\s*[\"']?[^\s,;\"']{4,}"
        ),
    ),
    (
        "bearer_token",
        re.compile(r"(?i)\bbearer\s+[a-z0-9._~+/=-]{8,}"),
    ),
    (
        "known_token_prefix",
        re.compile(
            r"\b(?:"
            r"AKIA[0-9A-Z]{16}"
            r"|gh[pousr]_[A-Za-z0-9_]{20,}"
            r"|github_pat_[A-Za-z0-9_]{20,}"
            r"|sk-[A-Za-z0-9_-]{20,}"
            r"|xox[baprs]-[A-Za-z0-9-]{10,}"
            r")\b"
        ),
    ),
    (
        "jwt",
        re.compile(
            r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}"
            r"(?:\.[A-Za-z0-9_-]{8,})?\b"
        ),
    ),
    (
        "private_key",
        re.compile(r"-----BEGIN (?:[A-Z0-9 ]+ )?PRIVATE KEY-----"),
    ),
)


@dataclass(frozen=True)
class PrivacyError(Exception):
    category: str
    file_index: int | None = None
    line: int | None = None


def _read_required_lines(path: Path, category: str) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8-sig")
    except (OSError, UnicodeError) as error:
        raise PrivacyError(category) from error
    if "\x00" in text:
        raise PrivacyError(category)
    lines = text.splitlines()
    if not lines:
        raise PrivacyError(category)
    if any(not line.strip() for line in lines):
        raise PrivacyError(category)
    stripped = [line.strip() for line in lines]
    if len(stripped) != len(set(stripped)):
        raise PrivacyError(category)
    return stripped


def _resolve_public_paths(entries: Sequence[str]) -> list[Path]:
    paths: list[Path] = []
    seen: set[Path] = set()
    for index, entry in enumerate(entries, start=1):
        if "\\" in entry:
            raise PrivacyError("invalid_allowlist_path", index)
        relative = Path(entry)
        if relative.is_absolute() or ".." in relative.parts:
            raise PrivacyError("invalid_allowlist_path", index)
        try:
            resolved = (REPO_ROOT / relative).resolve(strict=True)
            resolved.relative_to(REPO_ROOT)
        except (OSError, RuntimeError, ValueError) as error:
            raise PrivacyError("invalid_allowlist_path", index) from error
        if not resolved.is_file():
            raise PrivacyError("invalid_allowlist_path", index)
        if resolved in seen:
            raise PrivacyError("duplicate_allowlist_path", index)
        seen.add(resolved)
        paths.append(resolved)
    if not paths:
        raise PrivacyError("empty_allowlist")
    return paths


def _normalized_forms(value: str) -> set[str]:
    decoded = unquote(value)
    folded = unicodedata.normalize("NFKC", decoded).casefold()
    slash = re.sub(r"/+", "/", folded.replace("\\", "/"))
    backslash = re.sub(r"\\+", r"\\", folded.replace("/", "\\"))
    return {form for form in (folded, slash, backslash) if form}


def _line_number(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def _scan_text(
    text: str,
    forbidden_forms: Sequence[set[str]],
    file_index: int,
) -> None:
    content_forms = _normalized_forms(text)
    for literal_forms in forbidden_forms:
        if any(
            literal in content
            for literal in literal_forms
            for content in content_forms
        ):
            raise PrivacyError("forbidden_literal", file_index)
    for category, pattern in GENERIC_PATTERNS:
        match = pattern.search(text)
        if match is not None:
            raise PrivacyError(
                category, file_index, _line_number(text, match.start())
            )


def check(paths_from: Path, forbid_literal_file: Path) -> tuple[int, int]:
    path_entries = _read_required_lines(paths_from, "invalid_paths_file")
    forbidden_literals = _read_required_lines(
        forbid_literal_file, "invalid_forbidden_literal_file"
    )
    forbidden_forms = [_normalized_forms(value) for value in forbidden_literals]
    if any(not forms for forms in forbidden_forms):
        raise PrivacyError("invalid_forbidden_literal_file")
    public_paths = _resolve_public_paths(path_entries)
    for file_index, path in enumerate(public_paths, start=1):
        try:
            text = path.read_text(encoding="utf-8-sig")
        except (OSError, UnicodeError) as error:
            raise PrivacyError("unreadable_public_file", file_index) from error
        if "\x00" in text:
            raise PrivacyError("unreadable_public_file", file_index)
        _scan_text(text, forbidden_forms, file_index)
    return len(public_paths), len(forbidden_literals)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--paths-from", required=True, type=Path)
    parser.add_argument("--forbid-literal-file", required=True, type=Path)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        file_count, literal_count = check(
            args.paths_from, args.forbid_literal_file
        )
    except PrivacyError as error:
        details = [f"category={error.category}"]
        if error.file_index is not None:
            details.append(f"file_index={error.file_index}")
        if error.line is not None:
            details.append(f"line={error.line}")
        print(
            "public-evidence-privacy: fail " + " ".join(details),
            file=sys.stderr,
        )
        return 2
    print(
        "public-evidence-privacy: pass "
        f"files={file_count} forbidden_literals={literal_count} "
        f"tool_version={TOOL_VERSION}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
