#!/usr/bin/env python3
"""Write the curated packet manifest after all report files are finalized."""

from __future__ import annotations

import csv
import hashlib
from pathlib import Path


ROOT = Path(__file__).resolve().parent
MANIFEST = ROOT / "packet-manifest.csv"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


rows = []
for path in sorted(ROOT.rglob("*")):
    if path == MANIFEST or not path.is_file():
        continue
    rows.append(
        {
            "file": path.relative_to(ROOT).as_posix(),
            "sha256": sha256(path),
            "bytes": path.stat().st_size,
        }
    )

with MANIFEST.open("w", encoding="utf-8", newline="") as handle:
    writer = csv.DictWriter(handle, fieldnames=["file", "sha256", "bytes"])
    writer.writeheader()
    writer.writerows(rows)

print(f"{MANIFEST}: {len(rows)} files")
