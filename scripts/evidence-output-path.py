#!/usr/bin/env python3
"""Resolve safe external benchmark evidence destinations."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from evidence_retention import (
    EvidencePolicyError,
    default_external_output_path,
    validate_external_output_path,
)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    default = subparsers.add_parser("default")
    default.add_argument("--repo-root", type=Path, required=True)
    default.add_argument("--kind", required=True)
    default.add_argument("--timestamp")

    validate = subparsers.add_parser("validate")
    validate.add_argument("--repo-root", type=Path, required=True)
    validate.add_argument("--path", type=Path, required=True)

    args = parser.parse_args(argv)
    try:
        if args.command == "default":
            candidate = default_external_output_path(
                args.kind,
                timestamp=args.timestamp,
            )
        else:
            candidate = args.path
        print(validate_external_output_path(args.repo_root, candidate))
    except EvidencePolicyError as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    sys.exit(main())
