#!/usr/bin/env python3
"""Create a deterministic, explicitly allowlisted compact evidence packet."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from evidence_retention import (
    EvidencePolicyError,
    curate_compact_evidence,
    read_explicit_path_list,
)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--allowlist", type=Path, required=True)
    parser.add_argument("--manifest-name", default="packet-manifest.csv")
    parser.add_argument(
        "--fixture-exceptions",
        type=Path,
        help="optional exact source-relative paths allowed to exceed 5 MiB",
    )
    args = parser.parse_args(argv)
    try:
        exceptions = (
            read_explicit_path_list(
                args.fixture_exceptions,
                label="fixture exception",
            )
            if args.fixture_exceptions
            else ()
        )
        rows = curate_compact_evidence(
            args.source_root,
            args.output_root,
            args.allowlist,
            manifest_name=args.manifest_name,
            fixture_exceptions=exceptions,
        )
    except (EvidencePolicyError, OSError) as error:
        parser.error(str(error))
    print(f"curated {len(rows)} files into {args.output_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
