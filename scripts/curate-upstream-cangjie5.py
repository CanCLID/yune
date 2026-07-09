#!/usr/bin/env python3
"""Augment the upstream cangjie5 composition capture with the minimal upstream
source slice needed to drive Yune's real translator NON-CIRCULARLY.

D-48 item 2: the oracle capture (cangjie5-composition.json) records librime's
candidate lists. To assert Yune's real production path against them without
Yune-derived expected values, the char-by-char composition test builds Yune's
translator from UPSTREAM rime-cangjie rows (never Yune output). This script
embeds the exact-code cohorts for the seven single-character composition
constituents plus their rime-essay preset-vocabulary weights, so the fixture is
self-contained and byte-content-verifiably regenerable.

Reads rows straight from the pinned rime-cangjie source and rime-essay/essay.txt
as UTF-8 files (never a native stdout pipe -> no CJK codepage corruption).

Usage: python scripts/curate-upstream-cangjie5.py <fixture.json> <rime-cangjie-dir> <essay.txt>
"""
import json
import sys

# The seven single cangjie codes whose top candidate is the character that the
# owner composition rows are built from (hwmvs=粵, qtt=拼, ebcn=測, yripm=試,
# tak=莫, oha=伯, eosk=洢). These, selected one at a time, compose the phrases.
ATOMIC_CODES = ["hwmvs", "qtt", "ebcn", "yripm", "tak", "oha", "eosk"]
DICT_FILES = ["cangjie5.base.dict.yaml", "cangjie5.stem.dict.yaml", "cangjie5.extended.dict.yaml"]
# Composed-phrase + control characters, so the preset-vocabulary slice is complete.
PHRASE_CHARS = list("粵拼測試莫伯洢香港中文粤")


def dict_body_rows(path):
    """Yield (text, code, raw_line) for entry rows after the YAML header (...)."""
    with open(path, encoding="utf-8") as handle:
        in_body = False
        for line in handle:
            line = line.rstrip("\n").rstrip("\r")
            if not in_body:
                if line.strip() == "...":
                    in_body = True
                continue
            if not line or line.startswith("#"):
                continue
            fields = line.split("\t")
            if len(fields) < 2 or not fields[0] or not fields[1]:
                continue
            yield fields[0], fields[1], line


def main():
    fixture_path, cangjie_dir, essay_path = sys.argv[1], sys.argv[2], sys.argv[3]
    with open(fixture_path, encoding="utf-8") as handle:
        fixture = json.load(handle)

    atomic = set(ATOMIC_CODES)
    import_rows = {}
    cohort_chars = set(PHRASE_CHARS)
    for file_name in DICT_FILES:
        path = f"{cangjie_dir}/{file_name}"
        rows = []
        for text, code, _line in dict_body_rows(path):
            if code in atomic:
                # Preserve exactly two columns (text, code) for base/extended and
                # (text, code, stem) for stem, matching the upstream row shape.
                rows.append("\t".join(_line.split("\t")))
                cohort_chars.add(text)
        if rows:
            import_rows[file_name] = rows

    # rime-essay preset vocabulary weights for every character in the slice.
    essay_weights = {}
    with open(essay_path, encoding="utf-8") as handle:
        for line in handle:
            line = line.rstrip("\n").rstrip("\r")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) >= 2 and parts[0] not in essay_weights:
                essay_weights[parts[0]] = parts[1]
    vocabulary_rows = [
        f"{char}\t{essay_weights[char]}" for char in sorted(cohort_chars) if char in essay_weights
    ]

    fixture["source_slice"] = {
        "policy": "d48_cangjie5_exact_code_cohorts_for_char_by_char_composition",
        "schema_data": "rime/rime-cangjie",
        "schema_data_commit": fixture["schema"]["source_commit"],
        "vocabulary": "essay",
        "essay_vocabulary_file": "rime-essay/essay.txt",
        "atomic_codes": ATOMIC_CODES,
        "note": (
            "Exact-code cohorts (rows whose code equals the pin) for the seven "
            "single-character composition constituents, plus rime-essay weights. "
            "Upstream rows only; expected candidate values come from the oracle "
            "cases, never from Yune."
        ),
        "import_rows": import_rows,
        "vocabulary_rows": vocabulary_rows,
    }

    with open(fixture_path, "w", encoding="utf-8", newline="\n") as handle:
        json.dump(fixture, handle, ensure_ascii=False, indent=2)
        handle.write("\n")
    print(f"augmented {fixture_path}: {sum(len(v) for v in import_rows.values())} import rows, "
          f"{len(vocabulary_rows)} vocabulary rows")


if __name__ == "__main__":
    main()
