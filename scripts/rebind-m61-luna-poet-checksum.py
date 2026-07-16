#!/usr/bin/env python3
"""Bind a generated M61 Luna POET sidecar to the restored upstream table.

The benchmark deploy-prep process builds a YUNE-POET/3 sidecar next to a
Yune-generated table and therefore keys the sidecar with that table's declared
checksum.  The benchmark then restores the pinned upstream compact table.  For
the one pinned upstream Luna source/table pair, the runtime deliberately keys
POET validation with the raw source checksum instead.

This helper is intentionally narrow.  It accepts only that pinned pair, checks
the generated sidecar is still keyed to the restored table checksum, and writes
a create-new copy whose only changed bytes are the four-byte POET checksum
field.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import struct
import sys
from pathlib import Path


POET_MAGIC = b"YUNE-POET/3\0"
POET_HEADER_LEN = 24
POET_SECTION_COUNT = 20
POET_SECTION_DIRECTORY_ENTRY_LEN = 20
POET_SECTION_STRIDES = {
    1: 16,
    2: 1,
    3: 1,
    4: 8,
    5: 20,
    6: 1,
    7: 4,
    8: 12,
    9: 1,
    10: 20,
    11: 1,
    12: 4,
    13: 12,
    14: 1,
    15: 12,
    16: 1,
    17: 12,
    18: 1,
    19: 8,
    20: 32,
}

TABLE_FORMAT_FIELD = b"Rime::Table/4.0" + bytes(17)
TABLE_HEADER_LEN = 68
PINNED_LUNA_SOURCE_CHECKSUM = 0xB3D4_E98E
PINNED_LUNA_TABLE_CHECKSUM = 0x29D5_6C89
PINNED_LUNA_STRING_TABLE_SIZE = 1_574_520


class PreparationError(RuntimeError):
    """Input does not match the exact M61 benchmark-preparation contract."""


def read_u32(data: bytes, offset: int, label: str) -> int:
    if offset < 0 or offset + 4 > len(data):
        raise PreparationError(f"{label} is truncated")
    return struct.unpack_from("<I", data, offset)[0]


def rime_raw_source_checksum(data: bytes) -> int:
    """Match yune_core::rime_dict_source_checksum(0, [data], None)."""

    remainder = 0
    for byte in data:
        remainder ^= byte
        for _ in range(8):
            if remainder & 1:
                remainder = (remainder >> 1) ^ 0xEDB8_8320
            else:
                remainder >>= 1
    return remainder ^ 0xFFFF_FFFF


def validate_pinned_source_and_table(source: bytes, table: bytes) -> tuple[int, int]:
    source_checksum = rime_raw_source_checksum(source)
    if len(table) < TABLE_HEADER_LEN:
        raise PreparationError("restored table header is truncated")
    if table[:32] != TABLE_FORMAT_FIELD:
        raise PreparationError("restored table is not exact Rime::Table/4.0 format")

    table_checksum = read_u32(table, 32, "restored table checksum")
    syllabary_offset = read_u32(table, 44, "restored table syllabary offset")
    index_offset = read_u32(table, 48, "restored table index offset")
    string_table_size = read_u32(table, 64, "restored table string-table size")
    if syllabary_offset == 0 or index_offset == 0:
        raise PreparationError("restored table is missing a required compiled section")

    observed = (source_checksum, table_checksum, string_table_size)
    expected = (
        PINNED_LUNA_SOURCE_CHECKSUM,
        PINNED_LUNA_TABLE_CHECKSUM,
        PINNED_LUNA_STRING_TABLE_SIZE,
    )
    if observed != expected:
        raise PreparationError(
            "unknown or stale Luna source/table pair: "
            f"source=0x{source_checksum:08x}, table=0x{table_checksum:08x}, "
            f"string_table_size={string_table_size}"
        )
    return source_checksum, table_checksum


def validate_generated_poet(poet: bytes, expected_table_checksum: int) -> int:
    if len(poet) < POET_HEADER_LEN:
        raise PreparationError("generated POET header is truncated")
    if poet[: len(POET_MAGIC)] != POET_MAGIC:
        raise PreparationError("generated sidecar is not YUNE-POET/3")

    poet_checksum = read_u32(poet, 12, "generated POET checksum")
    if poet_checksum != expected_table_checksum:
        raise PreparationError(
            "generated POET is not keyed to the restored table checksum: "
            f"expected=0x{expected_table_checksum:08x}, "
            f"actual=0x{poet_checksum:08x}"
        )

    section_count = read_u32(poet, 16, "generated POET section count")
    section_directory_offset = read_u32(
        poet, 20, "generated POET section-directory offset"
    )
    if section_count != POET_SECTION_COUNT:
        raise PreparationError(
            "generated POET has an unknown section count: "
            f"expected={POET_SECTION_COUNT}, actual={section_count}"
        )
    directory_len = section_count * POET_SECTION_DIRECTORY_ENTRY_LEN
    directory_end = section_directory_offset + directory_len
    if (
        section_directory_offset < POET_HEADER_LEN
        or directory_end != len(poet)
    ):
        raise PreparationError("generated POET has an invalid section directory")

    seen: set[int] = set()
    nonempty_ranges: list[tuple[int, int, int]] = []
    for index in range(section_count):
        entry_offset = (
            section_directory_offset + index * POET_SECTION_DIRECTORY_ENTRY_LEN
        )
        section_id, offset, length, count, stride = struct.unpack_from(
            "<IIIII", poet, entry_offset
        )
        expected_stride = POET_SECTION_STRIDES.get(section_id)
        if expected_stride is None or section_id in seen:
            raise PreparationError("generated POET has duplicate or unknown sections")
        seen.add(section_id)
        if stride != expected_stride or length != count * stride:
            raise PreparationError(
                f"generated POET section {section_id} has an invalid shape"
            )
        end = offset + length
        if offset < POET_HEADER_LEN or end > section_directory_offset:
            raise PreparationError(
                f"generated POET section {section_id} is out of bounds"
            )
        if length:
            nonempty_ranges.append((offset, end, section_id))

    if seen != set(POET_SECTION_STRIDES):
        raise PreparationError("generated POET is missing required sections")
    nonempty_ranges.sort()
    for left, right in zip(nonempty_ranges, nonempty_ranges[1:]):
        if left[1] > right[0]:
            raise PreparationError(
                "generated POET payload sections overlap: "
                f"{left[2]} and {right[2]}"
            )
    return poet_checksum


def write_create_new(path: Path, data: bytes) -> None:
    if not path.parent.is_dir():
        raise PreparationError(f"output parent is not a directory: {path.parent}")
    descriptor: int | None = None
    created = False
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        created = True
        with os.fdopen(descriptor, "wb") as handle:
            descriptor = None
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
    except FileExistsError as error:
        raise PreparationError(f"output POET path already exists: {path}") from error
    except OSError as error:
        if descriptor is not None:
            os.close(descriptor)
        if created:
            path.unlink(missing_ok=True)
        raise PreparationError(f"could not write output POET: {error}") from error


def prepare_sidecar(
    dictionary_source_path: Path,
    restored_table_path: Path,
    generated_poet_path: Path,
    output_poet_path: Path,
) -> list[str]:
    input_paths = [dictionary_source_path, restored_table_path, generated_poet_path]
    for path in input_paths:
        if not path.is_file():
            raise PreparationError(f"required input is not a file: {path}")

    generated_resolved = generated_poet_path.resolve(strict=True)
    output_resolved = output_poet_path.parent.resolve(strict=True) / output_poet_path.name
    if generated_resolved == output_resolved:
        raise PreparationError("generated and output POET paths must be different")

    source = dictionary_source_path.read_bytes()
    table = restored_table_path.read_bytes()
    generated_poet = generated_poet_path.read_bytes()
    source_checksum, table_checksum = validate_pinned_source_and_table(source, table)
    original_poet_checksum = validate_generated_poet(generated_poet, table_checksum)

    rebound_poet = bytearray(generated_poet)
    struct.pack_into("<I", rebound_poet, 12, source_checksum)
    rebound_bytes = bytes(rebound_poet)
    changed_offsets = [
        index
        for index, (before, after) in enumerate(zip(generated_poet, rebound_bytes))
        if before != after
    ]
    if changed_offsets != [12, 13, 14, 15]:
        raise PreparationError(
            "POET preparation did not change exactly checksum bytes 12 through 15"
        )
    if (
        rebound_bytes[:12] != generated_poet[:12]
        or rebound_bytes[16:] != generated_poet[16:]
    ):
        raise PreparationError("POET preparation did not preserve the generated payload")

    write_create_new(output_poet_path, rebound_bytes)
    written = output_poet_path.read_bytes()
    if written != rebound_bytes:
        output_poet_path.unlink(missing_ok=True)
        raise PreparationError("output POET did not round-trip byte-for-byte")
    validate_generated_poet(
        written[:12] + struct.pack("<I", table_checksum) + written[16:],
        table_checksum,
    )

    return [
        "status=pass",
        f"source_checksum=0x{source_checksum:08x}",
        f"restored_table_checksum=0x{table_checksum:08x}",
        f"generated_poet_checksum=0x{original_poet_checksum:08x}",
        f"output_poet_checksum=0x{read_u32(written, 12, 'output POET checksum'):08x}",
        "changed_byte_range=12:16",
        f"generated_poet_sha256={hashlib.sha256(generated_poet).hexdigest()}",
        f"output_poet_sha256={hashlib.sha256(written).hexdigest()}",
        f"poet_bytes={len(written)}",
    ]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dictionary-source", required=True, type=Path)
    parser.add_argument("--restored-table", required=True, type=Path)
    parser.add_argument("--generated-poet", required=True, type=Path)
    parser.add_argument("--output-poet", required=True, type=Path)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        receipt = prepare_sidecar(
            args.dictionary_source,
            args.restored_table,
            args.generated_poet,
            args.output_poet,
        )
    except (OSError, PreparationError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print("\n".join(receipt))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
