#!/usr/bin/env python3
"""Reject staged raw, oversized, or over-cap repository evidence changes."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from evidence_retention import (
    EvidencePolicyError,
    files_from_paths,
    read_explicit_path_list,
    staged_changed_evidence,
    staged_packet_sizes,
    tree_changed_evidence,
    tree_packet_sizes,
    validate_evidence_growth,
    working_tree_packet_sizes,
)


def main(argv: list[str] | None = None) -> int:
    scripts = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=scripts.parent)
    parser.add_argument(
        "--fixture-exceptions",
        type=Path,
        default=scripts / "evidence-fixture-exceptions.txt",
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument(
        "--paths-from",
        type=Path,
        help="validate newline-delimited working-tree paths instead of staged additions",
    )
    mode.add_argument(
        "--compare-base",
        help="validate committed evidence changes between this tree-ish and --treeish",
    )
    parser.add_argument(
        "--treeish",
        default="HEAD",
        help="post-change tree for --compare-base (default: HEAD)",
    )
    args = parser.parse_args(argv)
    try:
        exceptions = read_explicit_path_list(
            args.fixture_exceptions,
            label="fixture exception",
        )
        if args.paths_from:
            values = [
                line.strip()
                for line in args.paths_from.read_text(encoding="utf-8").splitlines()
                if line.strip() and not line.lstrip().startswith("#")
            ]
            records = files_from_paths(args.repo_root, values)
            packet_sizes = working_tree_packet_sizes(args.repo_root, records)
        elif args.compare_base:
            records = tree_changed_evidence(
                args.repo_root,
                args.compare_base,
                args.treeish,
            )
            packet_sizes = tree_packet_sizes(
                args.repo_root,
                records,
                args.treeish,
            )
        else:
            records = staged_changed_evidence(args.repo_root)
            packet_sizes = staged_packet_sizes(args.repo_root, records)
        accepted = validate_evidence_growth(
            records,
            exceptions,
            post_change_packet_sizes=packet_sizes,
        )
    except (EvidencePolicyError, OSError, UnicodeError) as error:
        print(str(error), file=sys.stderr)
        return 1
    print(f"evidence growth guard passed: {len(accepted)} changed files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
