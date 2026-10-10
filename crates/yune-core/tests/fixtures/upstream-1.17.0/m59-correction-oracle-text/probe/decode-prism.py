#!/usr/bin/env python3
"""Independently decode the tiny upstream M59 Prism/4.0 oracle fixtures.

This decoder intentionally uses only the Python standard library.  It does not
invoke Yune or share Yune's Rust parser, so the checked-in JSON observations can
serve as an external-byte oracle for the production parser and prism writer.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import struct
import sys
from pathlib import Path
from typing import Any


FORMAT = b"Rime::Prism/4.0"
METADATA_SIZE = 320
CORRECTION_MASK = 1 << 30
LABEL_MASK = (1 << 31) | 0xFF
VALUE_MASK = (1 << 31) - 1
DEFAULT_FIXTURE = Path(
    "crates/yune-core/tests/fixtures/upstream-1.17.0/"
    "m59-algebra-properties.json"
)


class DecodeError(ValueError):
    """The preserved prism does not satisfy the pinned binary contract."""


def _range(data: bytes | bytearray, offset: int, size: int, role: str) -> None:
    if offset < 0 or size < 0 or offset + size > len(data):
        raise DecodeError(f"{role} is out of bounds: offset={offset}, size={size}")


def _u32(data: bytes | bytearray, offset: int, role: str) -> int:
    _range(data, offset, 4, role)
    return struct.unpack_from("<I", data, offset)[0]


def _i32(data: bytes | bytearray, offset: int, role: str) -> int:
    _range(data, offset, 4, role)
    return struct.unpack_from("<i", data, offset)[0]


def _f32(data: bytes | bytearray, offset: int, role: str) -> float:
    _range(data, offset, 4, role)
    return struct.unpack_from("<f", data, offset)[0]


def _relative_pointer(
    data: bytes | bytearray, field_offset: int, role: str
) -> tuple[int, int | None]:
    raw = _i32(data, field_offset, f"{role} relative pointer")
    if raw == 0:
        return raw, None
    target = field_offset + raw
    if target < 0 or target >= len(data):
        raise DecodeError(
            f"{role} relative pointer is out of bounds: field={field_offset}, "
            f"raw={raw}, target={target}"
        )
    return raw, target


def _c_string(data: bytes | bytearray, offset: int, role: str) -> str:
    if offset < 0 or offset >= len(data):
        raise DecodeError(f"{role} string starts out of bounds: {offset}")
    end = data.find(b"\0", offset)
    if end < 0:
        raise DecodeError(f"{role} string is not NUL terminated")
    try:
        return bytes(data[offset:end]).decode("utf-8")
    except UnicodeDecodeError as error:
        raise DecodeError(f"{role} string is not UTF-8") from error


def _darts_offset(unit: int) -> int:
    return (unit >> 10) << ((unit & (1 << 9)) >> 6)


def _darts_label(unit: int) -> int:
    return unit & LABEL_MASK


def _darts_has_leaf(unit: int) -> bool:
    return ((unit >> 8) & 1) == 1


def _darts_exact_match(units: list[int], key: bytes) -> int | None:
    if not units or not key:
        return None
    node = 0
    unit = units[node]
    for byte in key:
        node ^= _darts_offset(unit) ^ byte
        if node >= len(units):
            return None
        unit = units[node]
        if _darts_label(unit) != byte:
            return None
    if not _darts_has_leaf(unit):
        return None
    leaf = node ^ _darts_offset(unit)
    if leaf >= len(units):
        return None
    return units[leaf] & VALUE_MASK


def _descriptor(data: bytes | bytearray, offset: int) -> dict[str, Any]:
    _range(data, offset, 16, "spelling descriptor")
    packed = _u32(data, offset + 4, "spelling descriptor packed type")
    credibility = _f32(data, offset + 8, "spelling descriptor credibility")
    _, tips_offset = _relative_pointer(data, offset + 12, "spelling descriptor tips")
    tips = "" if tips_offset is None else _c_string(data, tips_offset, "descriptor tips")
    credibility_bits = _u32(data, offset + 8, "spelling descriptor credibility bits")
    return {
        "syllable_id": _i32(data, offset, "spelling descriptor syllable id"),
        "spelling_type": packed & ~CORRECTION_MASK,
        "is_correction": bool(packed & CORRECTION_MASK),
        "credibility": credibility,
        "credibility_f32_bits": f"0x{credibility_bits:08X}",
        "tips": tips,
    }


class Prism:
    """Minimal independent reader for the observable upstream prism fields."""

    def __init__(self, data: bytes) -> None:
        if len(data) < METADATA_SIZE:
            raise DecodeError(f"prism is shorter than {METADATA_SIZE} bytes")
        header = data[:32].split(b"\0", 1)[0]
        if header != FORMAT:
            raise DecodeError(f"unexpected prism format: {header!r}")

        self.data = data
        self.dict_file_checksum = _u32(data, 32, "dictionary checksum")
        self.schema_file_checksum = _u32(data, 36, "schema checksum")
        self.num_syllables = _u32(data, 40, "syllable count")
        self.num_spellings = _u32(data, 44, "spelling count")
        self.double_array_size = _u32(data, 48, "double-array unit count")
        _, double_array_offset = _relative_pointer(data, 52, "double array")
        if double_array_offset is None or self.double_array_size == 0:
            raise DecodeError("nonempty mini-prism must contain a double array")
        double_array_bytes = self.double_array_size * 4
        _range(data, double_array_offset, double_array_bytes, "double array")
        self.units = list(
            struct.unpack_from(
                f"<{self.double_array_size}I", data, double_array_offset
            )
        )

        self.spelling_map_raw, self.spelling_map_offset = _relative_pointer(
            data, 56, "spelling map"
        )
        self.spelling_map = self._read_spelling_map()

    def _read_spelling_map(self) -> list[list[dict[str, Any]]] | None:
        if self.spelling_map_offset is None:
            if self.num_spellings > self.num_syllables:
                raise DecodeError(
                    "identity spelling map has more spellings than syllables"
                )
            return None
        count = _u32(self.data, self.spelling_map_offset, "spelling-map count")
        if count != self.num_spellings:
            raise DecodeError(
                f"spelling-map count {count} != metadata count {self.num_spellings}"
            )
        item_start = self.spelling_map_offset + 4
        _range(self.data, item_start, count * 8, "spelling-map items")
        result: list[list[dict[str, Any]]] = []
        for index in range(count):
            item = item_start + index * 8
            descriptor_count = _u32(
                self.data, item, f"spelling-map item {index} descriptor count"
            )
            _, descriptor_offset = _relative_pointer(
                self.data, item + 4, f"spelling-map item {index} descriptors"
            )
            if descriptor_offset is None:
                raise DecodeError(f"spelling-map item {index} has a null descriptor list")
            _range(
                self.data,
                descriptor_offset,
                descriptor_count * 16,
                f"spelling-map item {index} descriptors",
            )
            result.append(
                [
                    _descriptor(self.data, descriptor_offset + row * 16)
                    for row in range(descriptor_count)
                ]
            )
        return result

    def exact_match(self, surface: str) -> int | None:
        try:
            key = surface.encode("utf-8")
        except UnicodeEncodeError as error:
            raise DecodeError(f"surface cannot be encoded as UTF-8: {surface!r}") from error
        return _darts_exact_match(self.units, key)

    def observe(self, surface: str) -> dict[str, Any]:
        spelling_index = self.exact_match(surface)
        if spelling_index is None:
            return {"present": False, "descriptors": []}
        if spelling_index >= self.num_spellings:
            raise DecodeError(
                f"Darts value {spelling_index} for {surface!r} exceeds spelling count"
            )
        if self.spelling_map is None:
            descriptors = [
                {
                    "syllable_id": spelling_index,
                    "spelling_type": 0,
                    "is_correction": False,
                    "credibility": 0.0,
                    "credibility_f32_bits": "0x00000000",
                    "tips": "",
                }
            ]
        else:
            descriptors = copy.deepcopy(self.spelling_map[spelling_index])
        return {
            "present": True,
            "spelling_index": spelling_index,
            "descriptors": descriptors,
        }

    def artifact_metadata(self, path: str) -> dict[str, Any]:
        return {
            "path": path,
            "bytes": len(self.data),
            "sha256": hashlib.sha256(self.data).hexdigest(),
            "dict_file_checksum": self.dict_file_checksum,
            "schema_file_checksum": self.schema_file_checksum,
            "num_syllables": self.num_syllables,
            "num_spellings": self.num_spellings,
            "double_array_size": self.double_array_size,
            "spelling_map_offset_raw": self.spelling_map_raw,
            "spelling_map_offset_is_null": self.spelling_map_offset is None,
        }


def _resolved_artifact(fixture_path: Path, relative: str) -> Path:
    root = fixture_path.parent.resolve()
    path = (root / relative).resolve()
    try:
        path.relative_to(root)
    except ValueError as error:
        raise DecodeError(f"artifact escapes fixture root: {relative}") from error
    return path


def _updated_fixture(fixture_path: Path, fixture: dict[str, Any]) -> dict[str, Any]:
    updated = copy.deepcopy(fixture)
    for case in updated["cases"]:
        relative = case["artifact"]["path"]
        artifact_path = _resolved_artifact(fixture_path, relative)
        prism = Prism(artifact_path.read_bytes())
        case["artifact"] = prism.artifact_metadata(relative)
        case["surfaces"] = {
            surface: prism.observe(surface) for surface in case["surfaces"]
        }
    return updated


def _canonical_json(value: dict[str, Any]) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def _self_test() -> None:
    pointer = bytearray(16)
    struct.pack_into("<i", pointer, 4, 6)
    assert _relative_pointer(pointer, 4, "self-test") == (6, 10)
    struct.pack_into("<i", pointer, 4, 0)
    assert _relative_pointer(pointer, 4, "self-test") == (0, None)
    struct.pack_into("<i", pointer, 4, 32)
    try:
        _relative_pointer(pointer, 4, "self-test")
    except DecodeError:
        pass
    else:
        raise AssertionError("relative-pointer bounds self-test did not fail")

    units = [0] * 98
    units[0] = 1 << 10
    units[96] = (1 << 10) | (1 << 8) | ord("a")
    units[97] = 7
    assert _darts_exact_match(units, b"a") == 7
    assert _darts_exact_match(units, b"b") is None
    units[96] ^= 1
    assert _darts_exact_match(units, b"a") is None

    descriptor = bytearray(32)
    struct.pack_into("<iI f i", descriptor, 0, 7, 2 | CORRECTION_MASK, -0.5, 8)
    descriptor[20:24] = b"tip\0"
    decoded = _descriptor(descriptor, 0)
    assert decoded == {
        "syllable_id": 7,
        "spelling_type": 2,
        "is_correction": True,
        "credibility": -0.5,
        "credibility_f32_bits": "0xBF000000",
        "tips": "tip",
    }
    struct.pack_into("<i", descriptor, 12, 64)
    try:
        _descriptor(descriptor, 0)
    except DecodeError:
        pass
    else:
        raise AssertionError("descriptor pointer self-test did not fail")


def _verify_script_binding(fixture: dict[str, Any]) -> None:
    decoder = fixture.get("capture", {}).get("independent_decoder", {})
    expected = decoder.get("sha256")
    if not expected:
        return
    actual = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    if actual != expected:
        raise DecodeError(
            f"decoder SHA-256 mismatch: fixture={expected}, current={actual}"
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, default=DEFAULT_FIXTURE)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--verify", action="store_true")
    mode.add_argument("--write", action="store_true")
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run decoder unit checks only; still requires --verify or --write",
    )
    args = parser.parse_args()

    _self_test()
    if args.self_test:
        print("m59 algebra prism decoder self-test: PASS")
        return 0

    fixture_path = args.fixture.resolve()
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    if args.verify:
        _verify_script_binding(fixture)
    updated = _updated_fixture(fixture_path, fixture)
    rendered = _canonical_json(updated)
    if args.write:
        fixture_path.write_text(rendered, encoding="utf-8", newline="")
        print(f"updated {fixture_path}")
        return 0
    current = fixture_path.read_text(encoding="utf-8")
    if current != rendered:
        raise DecodeError(
            "fixture observations do not reproduce; run this command with --write"
        )
    print(f"verified {fixture_path}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (DecodeError, OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
