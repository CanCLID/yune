#!/usr/bin/env python3
"""Fail closed on native Track A candidate-page drift.

The native benchmark wrappers write a combined candidate snapshot.  This tool
validates the frozen M61 Track A page-zero shape, compares the same nine fields
for pinned librime and Yune, and emits a source/oracle-bound receipt.  Exit 1 is
reserved for a shape-valid behavior mismatch; malformed evidence exits 2.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import os
import re
import sys
import tempfile
from collections import defaultdict
from pathlib import Path
from typing import Sequence


TOOL_NAME = "check-native-candidate-parity.py"
FORMAT_VERSION = "1"
TRACK = "track-a-comparison"
SCHEMA_ID = "luna_pinyin"
ENGINES = ("librime-1.17.0", "yune")
DETAIL_INPUT = "zhongdengchangdu"
FROZEN_INPUTS = (
    "n",
    "ni",
    "hao",
    "zhongguo",
    "ceshiyixiachangjushuruxingnengzenyang",
    "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
    "cszysmsrsd",
    "zybfshmsru",
    "zh",
    "j",
    "yi",
    "che",
    "chuang",
    "b",
    "ceshi",
    DETAIL_INPUT,
    "dazisudu",
)
SNAPSHOT_HEADER = (
    "engine",
    "track",
    "schema_id",
    "input",
    "candidate_index",
    "candidate_count",
    "page_size",
    "page_no",
    "is_last_page",
    "highlighted_index",
    "composition_preedit",
    "text",
    "comment",
)
COMPARE_FIELDS = (
    "candidate_index",
    "candidate_count",
    "page_size",
    "page_no",
    "is_last_page",
    "highlighted_index",
    "composition_preedit",
    "text",
    "comment",
)
PARITY_HEADER = (
    "input",
    "oracle_rows",
    "yune_rows",
    "exact_match",
    "mismatch_fields",
    "oracle_texts_json",
    "yune_texts_json",
    "oracle_page_sha256",
    "yune_page_sha256",
)
PASS_RECEIPT_KEYS = (
    "format_version",
    "tool",
    "tool_sha256",
    "snapshot_sha256",
    "expected_inputs_sha256",
    "expected_inputs",
    "source_commit",
    "source_tree",
    "oracle_binary_sha256",
    "oracle_shared_tree_sha256",
    "oracle_build_tree_sha256",
    "track",
    "schema_id",
    "engines",
    "parity_csv_sha256",
    "detail_csv_sha256",
    "shape",
    "exact_inputs",
    "mismatches",
    "verdict",
    "exit_code",
)


class ShapeError(ValueError):
    """The candidate packet cannot support a behavior comparison."""


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _require_hex(value: str, length: int, label: str) -> str:
    if re.fullmatch(rf"[0-9a-fA-F]{{{length}}}", value) is None:
        raise ShapeError(f"{label} must be {length} hexadecimal characters")
    return value.lower()


def read_expected_inputs(payload: bytes) -> tuple[str, ...]:
    try:
        handle = io.StringIO(payload.decode("utf-8-sig"), newline="")
        reader = csv.DictReader(handle)
        if tuple(reader.fieldnames or ()) != ("input",):
            raise ShapeError(
                f"expected-input header must be exactly ['input'], got {reader.fieldnames!r}"
            )
        rows = list(reader)
    except (UnicodeError, csv.Error) as error:
        raise ShapeError(f"cannot read expected inputs: {error}") from error
    values: list[str] = []
    for row_number, row in enumerate(rows, start=2):
        if None in row or set(row) != {"input"}:
            raise ShapeError(f"expected-input row {row_number} is malformed")
        value = row.get("input")
        if not isinstance(value, str) or not value or value != value.strip():
            raise ShapeError(f"expected-input row {row_number} has an invalid input")
        values.append(value)
    if len(values) != len(set(values)):
        raise ShapeError("expected inputs contain a duplicate")
    observed = tuple(values)
    if observed != FROZEN_INPUTS:
        raise ShapeError(
            "expected inputs differ from the frozen M61 17-input order: "
            f"observed={observed!r}"
        )
    return observed


def read_snapshot(payload: bytes) -> list[dict[str, str]]:
    try:
        handle = io.StringIO(payload.decode("utf-8-sig"), newline="")
        reader = csv.DictReader(handle)
        if tuple(reader.fieldnames or ()) != SNAPSHOT_HEADER:
            raise ShapeError(
                "candidate snapshot header must be the exact 13-column contract; "
                f"got {reader.fieldnames!r}"
            )
        rows = list(reader)
    except (UnicodeError, csv.Error) as error:
        raise ShapeError(f"cannot read candidate snapshot: {error}") from error
    for row_number, row in enumerate(rows, start=2):
        if None in row or set(row) != set(SNAPSHOT_HEADER):
            raise ShapeError(f"candidate snapshot row {row_number} is malformed")
        if any(not isinstance(row[field], str) for field in SNAPSHOT_HEADER):
            raise ShapeError(f"candidate snapshot row {row_number} has a missing cell")
    return rows


def validate_pages(
    rows: Sequence[dict[str, str]], expected_inputs: Sequence[str]
) -> dict[tuple[str, str], list[dict[str, str]]]:
    groups: dict[tuple[str, str], list[dict[str, str]]] = defaultdict(list)
    expected_set = set(expected_inputs)
    for row_number, row in enumerate(rows, start=2):
        is_track_a = row["track"] == TRACK
        is_luna = row["schema_id"] == SCHEMA_ID
        if not is_track_a and not is_luna:
            # The combined wrapper packet may also contain the Track B product
            # page.  It is intentionally outside this Track A parity contract.
            continue
        if not (is_track_a and is_luna):
            raise ShapeError(
                f"row {row_number} partially matches the Track A identity: "
                f"track={row['track']!r}, schema_id={row['schema_id']!r}"
            )
        engine = row["engine"]
        input_text = row["input"]
        if engine not in ENGINES:
            raise ShapeError(f"row {row_number} has unexpected Track A engine {engine!r}")
        if input_text not in expected_set:
            raise ShapeError(
                f"row {row_number} has unexpected Track A input {input_text!r}"
            )
        groups[(engine, input_text)].append(row)

    expected_groups = {
        (engine, input_text)
        for engine in ENGINES
        for input_text in expected_inputs
    }
    actual_groups = set(groups)
    missing = sorted(expected_groups - actual_groups)
    extra = sorted(actual_groups - expected_groups)
    if missing:
        raise ShapeError(f"missing Track A page groups: {missing!r}")
    if extra:
        raise ShapeError(f"unexpected Track A page groups: {extra!r}")
    if sum(len(group) for group in groups.values()) != 170:
        raise ShapeError(
            "expected exactly 170 Track A snapshot rows "
            f"(17 inputs x 2 engines x 5 candidates), got "
            f"{sum(len(group) for group in groups.values())}"
        )

    fixed_geometry = {
        "candidate_count": "5",
        "page_size": "5",
        "page_no": "0",
        "is_last_page": "0",
        "highlighted_index": "0",
    }
    for engine, input_text in sorted(expected_groups):
        group = groups[(engine, input_text)]
        if len(group) != 5:
            raise ShapeError(
                f"{engine}/{input_text} must contain exactly five page-zero rows; "
                f"got {len(group)}"
            )
        indices: list[int] = []
        for row in group:
            try:
                index = int(row["candidate_index"])
            except ValueError as error:
                raise ShapeError(
                    f"{engine}/{input_text} has invalid candidate_index "
                    f"{row['candidate_index']!r}"
                ) from error
            if str(index) != row["candidate_index"]:
                raise ShapeError(
                    f"{engine}/{input_text} candidate_index is not canonical: "
                    f"{row['candidate_index']!r}"
                )
            indices.append(index)
            for field, expected in fixed_geometry.items():
                if row[field] != expected:
                    raise ShapeError(
                        f"{engine}/{input_text}/{index} has {field}={row[field]!r}; "
                        f"expected {expected!r}"
                    )
            if not row["composition_preedit"]:
                raise ShapeError(
                    f"{engine}/{input_text}/{index} has empty composition_preedit"
                )
            if not row["text"]:
                raise ShapeError(f"{engine}/{input_text}/{index} has empty text")
        if sorted(indices) != list(range(5)) or len(set(indices)) != 5:
            raise ShapeError(
                f"{engine}/{input_text} candidate indices must be unique, "
                f"contiguous 0..4; got {indices!r}"
            )
        group.sort(key=lambda row: int(row["candidate_index"]))
    return groups


def page_hash(rows: Sequence[dict[str, str]]) -> str:
    payload = [[row[field] for field in COMPARE_FIELDS] for row in rows]
    encoded = json.dumps(
        payload, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def compare_pages(
    groups: dict[tuple[str, str], list[dict[str, str]]],
    expected_inputs: Sequence[str],
) -> tuple[list[dict[str, str]], list[dict[str, str]], list[str]]:
    parity_rows: list[dict[str, str]] = []
    detail_rows: list[dict[str, str]] = []
    mismatches: list[str] = []
    for input_text in expected_inputs:
        oracle = groups[(ENGINES[0], input_text)]
        yune = groups[(ENGINES[1], input_text)]
        mismatch_fields = [
            field
            for field in COMPARE_FIELDS
            if [row[field] for row in oracle] != [row[field] for row in yune]
        ]
        exact = not mismatch_fields
        if not exact:
            mismatches.append(input_text)
        parity_rows.append(
            {
                "input": input_text,
                "oracle_rows": str(len(oracle)),
                "yune_rows": str(len(yune)),
                "exact_match": "1" if exact else "0",
                "mismatch_fields": ",".join(mismatch_fields),
                "oracle_texts_json": json.dumps(
                    [row["text"] for row in oracle],
                    ensure_ascii=False,
                    separators=(",", ":"),
                ),
                "yune_texts_json": json.dumps(
                    [row["text"] for row in yune],
                    ensure_ascii=False,
                    separators=(",", ":"),
                ),
                "oracle_page_sha256": page_hash(oracle),
                "yune_page_sha256": page_hash(yune),
            }
        )
        if input_text == DETAIL_INPUT:
            detail_rows.extend(oracle)
            detail_rows.extend(yune)
    return parity_rows, detail_rows, mismatches


def csv_bytes(
    fieldnames: Sequence[str], rows: Sequence[dict[str, str]]
) -> bytes:
    output = io.StringIO(newline="")
    writer = csv.DictWriter(
        output, fieldnames=fieldnames, lineterminator="\n", extrasaction="raise"
    )
    writer.writeheader()
    writer.writerows(rows)
    return output.getvalue().encode("utf-8")


def receipt_bytes(fields: Sequence[tuple[str, str]], errors: Sequence[str]) -> bytes:
    lines = [f"{key}={value}" for key, value in fields]
    lines.extend(
        f"shape_error_{index}={error.replace(chr(13), ' ').replace(chr(10), ' ')}"
        for index, error in enumerate(errors, start=1)
    )
    return ("\n".join(lines) + "\n").encode("utf-8")


def write_atomic(path: Path, payload: bytes) -> None:
    descriptor, temporary = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    try:
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Check the frozen native Track A candidate-page parity contract",
        allow_abbrev=False,
    )
    parser.add_argument("--snapshot-csv", required=True, type=Path)
    parser.add_argument("--expected-inputs-csv", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--source-tree", required=True)
    parser.add_argument("--oracle-binary-sha256", required=True)
    parser.add_argument("--oracle-shared-tree-sha256", required=True)
    parser.add_argument("--oracle-build-tree-sha256", required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        args.output_dir.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        print(
            f"candidate-parity evidence error: cannot create output directory: {error}",
            file=sys.stderr,
        )
        return 2
    parity_path = args.output_dir / "candidate-parity.csv"
    detail_path = args.output_dir / "zhongdengchangdu-detail.csv"
    verdict_path = args.output_dir / "candidate-parity-verdict.txt"
    for path in (parity_path, detail_path, verdict_path):
        if path.exists():
            print(f"candidate-parity evidence error: refusing to overwrite {path}", file=sys.stderr)
            return 2

    errors: list[str] = []
    snapshot_payload: bytes | None = None
    expected_inputs_payload: bytes | None = None
    expected_inputs: tuple[str, ...] = ()
    parity_rows: list[dict[str, str]] = []
    detail_rows: list[dict[str, str]] = []
    mismatches: list[str] = []
    source_commit = args.source_commit.lower()
    source_tree = args.source_tree.lower()
    oracle_binary = args.oracle_binary_sha256.lower()
    oracle_shared = args.oracle_shared_tree_sha256.lower()
    oracle_build = args.oracle_build_tree_sha256.lower()
    try:
        snapshot_payload = args.snapshot_csv.read_bytes()
        expected_inputs_payload = args.expected_inputs_csv.read_bytes()
        source_commit = _require_hex(source_commit, 40, "source commit")
        source_tree = _require_hex(source_tree, 40, "source tree")
        oracle_binary = _require_hex(oracle_binary, 64, "oracle binary SHA-256")
        oracle_shared = _require_hex(
            oracle_shared, 64, "oracle shared-tree SHA-256"
        )
        oracle_build = _require_hex(
            oracle_build, 64, "oracle build-tree SHA-256"
        )
        expected_inputs = read_expected_inputs(expected_inputs_payload)
        rows = read_snapshot(snapshot_payload)
        groups = validate_pages(rows, expected_inputs)
        parity_rows, detail_rows, mismatches = compare_pages(
            groups, expected_inputs
        )
    except (ShapeError, OSError) as error:
        errors.append(str(error))

    parity_payload = csv_bytes(PARITY_HEADER, parity_rows)
    detail_payload = csv_bytes(SNAPSHOT_HEADER, detail_rows)
    exit_code = 2 if errors else (1 if mismatches else 0)
    shape = "FAIL" if errors else "PASS"
    exact_count = 0 if errors else len(expected_inputs) - len(mismatches)
    verdict = "PASS" if exit_code == 0 else "FAIL"
    try:
        tool_sha256 = sha256_file(Path(__file__).resolve())
        snapshot_sha256 = (
            hashlib.sha256(snapshot_payload).hexdigest()
            if snapshot_payload is not None
            else "unavailable"
        )
        expected_inputs_sha256 = (
            hashlib.sha256(expected_inputs_payload).hexdigest()
            if expected_inputs_payload is not None
            else "unavailable"
        )
    except OSError as error:
        errors.append(f"cannot hash input/tool bytes: {error}")
        exit_code = 2
        shape = "FAIL"
        exact_count = 0
        verdict = "FAIL"
        tool_sha256 = "unavailable"
        snapshot_sha256 = "unavailable"
        expected_inputs_sha256 = "unavailable"

    fields = (
        ("format_version", FORMAT_VERSION),
        ("tool", TOOL_NAME),
        ("tool_sha256", tool_sha256),
        ("snapshot_sha256", snapshot_sha256),
        ("expected_inputs_sha256", expected_inputs_sha256),
        ("expected_inputs", ",".join(expected_inputs)),
        ("source_commit", source_commit),
        ("source_tree", source_tree),
        ("oracle_binary_sha256", oracle_binary),
        ("oracle_shared_tree_sha256", oracle_shared),
        ("oracle_build_tree_sha256", oracle_build),
        ("track", TRACK),
        ("schema_id", SCHEMA_ID),
        ("engines", ",".join(ENGINES)),
        ("parity_csv_sha256", hashlib.sha256(parity_payload).hexdigest()),
        ("detail_csv_sha256", hashlib.sha256(detail_payload).hexdigest()),
        ("shape", shape),
        ("exact_inputs", f"{exact_count}/{len(FROZEN_INPUTS)}"),
        ("mismatches", ",".join(mismatches) if mismatches else "none"),
        ("verdict", verdict),
        ("exit_code", str(exit_code)),
    )
    try:
        write_atomic(parity_path, parity_payload)
        write_atomic(detail_path, detail_payload)
        write_atomic(verdict_path, receipt_bytes(fields, errors))
    except OSError as error:
        print(f"candidate-parity evidence error: cannot write outputs: {error}", file=sys.stderr)
        return 2
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
