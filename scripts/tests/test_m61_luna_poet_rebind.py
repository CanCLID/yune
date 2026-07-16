from __future__ import annotations

import shutil
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
TOOL = SCRIPTS / "rebind-m61-luna-poet-checksum.py"
PINNED_SOURCE = (
    REPO_ROOT / "apps" / "yune-web" / "public" / "schema" / "luna_pinyin.dict.yaml"
)
TABLE_FORMAT_FIELD = b"Rime::Table/4.0" + bytes(17)
TABLE_CHECKSUM = 0x29D5_6C89
SOURCE_CHECKSUM = 0xB3D4_E98E
STRING_TABLE_SIZE = 1_574_520
SECTION_STRIDES = {
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


def table_fixture(
    *,
    checksum: int = TABLE_CHECKSUM,
    string_table_size: int = STRING_TABLE_SIZE,
    format_field: bytes = TABLE_FORMAT_FIELD,
) -> bytes:
    data = bytearray(68)
    data[:32] = format_field
    struct.pack_into("<I", data, 32, checksum)
    struct.pack_into("<I", data, 44, 68)
    struct.pack_into("<I", data, 48, 72)
    struct.pack_into("<I", data, 60, 76)
    struct.pack_into("<I", data, 64, string_table_size)
    return bytes(data)


def poet_fixture(*, checksum: int = TABLE_CHECKSUM) -> bytes:
    data = bytearray(24)
    data[:12] = b"YUNE-POET/3\0"
    struct.pack_into("<I", data, 12, checksum)
    struct.pack_into("<I", data, 16, len(SECTION_STRIDES))
    struct.pack_into("<I", data, 20, 24)
    for section_id, stride in SECTION_STRIDES.items():
        data.extend(struct.pack("<IIIII", section_id, 24, 0, 0, stride))
    return bytes(data)


class M61LunaPoetRebindTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.source = self.root / "luna_pinyin.dict.yaml"
        self.table = self.root / "luna_pinyin.table.bin"
        self.generated = self.root / "generated.poet.bin"
        self.output = self.root / "luna_pinyin.poet.bin"
        shutil.copy2(PINNED_SOURCE, self.source)
        self.table.write_bytes(table_fixture())
        self.generated.write_bytes(poet_fixture())

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def run_tool(
        self, *, output: Path | None = None
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                "-B",
                str(TOOL),
                "--dictionary-source",
                str(self.source),
                "--restored-table",
                str(self.table),
                "--generated-poet",
                str(self.generated),
                "--output-poet",
                str(output or self.output),
            ],
            text=True,
            capture_output=True,
            check=False,
        )

    def assert_rejected(self, expected_error: str) -> None:
        result = self.run_tool()
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(expected_error, result.stderr)
        self.assertFalse(self.output.exists(), "a rejected input must not create output")

    def test_rebinds_only_checksum_field_for_exact_pinned_pair(self) -> None:
        result = self.run_tool()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        before = self.generated.read_bytes()
        after = self.output.read_bytes()
        self.assertEqual(before[:12], after[:12])
        self.assertEqual(before[16:], after[16:])
        self.assertEqual(struct.unpack_from("<I", before, 12)[0], TABLE_CHECKSUM)
        self.assertEqual(struct.unpack_from("<I", after, 12)[0], SOURCE_CHECKSUM)
        self.assertEqual(len(before), len(after))
        for receipt in (
            "status=pass",
            "source_checksum=0xb3d4e98e",
            "restored_table_checksum=0x29d56c89",
            "generated_poet_checksum=0x29d56c89",
            "output_poet_checksum=0xb3d4e98e",
            "changed_byte_range=12:16",
        ):
            self.assertIn(receipt, result.stdout)
        self.assertFalse((SCRIPTS / "__pycache__").exists())

    def test_refuses_unknown_source_or_table_pair(self) -> None:
        self.source.write_bytes(self.source.read_bytes() + b"\n")
        self.assert_rejected("unknown or stale Luna source/table pair")

        shutil.copy2(PINNED_SOURCE, self.source)
        self.table.write_bytes(table_fixture(checksum=0xB967_CFEF))
        self.assert_rejected("unknown or stale Luna source/table pair")

        self.table.write_bytes(table_fixture(string_table_size=0))
        self.assert_rejected("unknown or stale Luna source/table pair")

    def test_refuses_malformed_table_headers(self) -> None:
        self.table.write_bytes(b"short")
        self.assert_rejected("restored table header is truncated")

        self.table.write_bytes(table_fixture(format_field=b"Rime::Table/5.0" + bytes(17)))
        self.assert_rejected("not exact Rime::Table/4.0 format")

        malformed = bytearray(table_fixture())
        struct.pack_into("<I", malformed, 44, 0)
        self.table.write_bytes(malformed)
        self.assert_rejected("missing a required compiled section")

    def test_refuses_wrong_or_legacy_poet_identity(self) -> None:
        self.generated.write_bytes(poet_fixture(checksum=SOURCE_CHECKSUM))
        self.assert_rejected("not keyed to the restored table checksum")

        legacy = bytearray(poet_fixture())
        legacy[:12] = b"YUNE-POET/2\0"
        self.generated.write_bytes(legacy)
        self.assert_rejected("not YUNE-POET/3")

    def test_refuses_malformed_poet_sections(self) -> None:
        self.generated.write_bytes(poet_fixture()[:20])
        self.assert_rejected("generated POET header is truncated")

        bad_directory = bytearray(poet_fixture())
        struct.pack_into("<I", bad_directory, 20, 25)
        self.generated.write_bytes(bad_directory)
        self.assert_rejected("invalid section directory")

        duplicate = bytearray(poet_fixture())
        struct.pack_into("<I", duplicate, 24 + 20, 1)
        self.generated.write_bytes(duplicate)
        self.assert_rejected("duplicate or unknown sections")

        bad_stride = bytearray(poet_fixture())
        struct.pack_into("<I", bad_stride, 24 + 16, 15)
        self.generated.write_bytes(bad_stride)
        self.assert_rejected("section 1 has an invalid shape")

    def test_output_is_create_new_and_input_is_never_modified(self) -> None:
        original = self.generated.read_bytes()
        self.output.write_bytes(b"owned")
        result = self.run_tool()
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("already exists", result.stderr)
        self.assertEqual(self.output.read_bytes(), b"owned")
        self.assertEqual(self.generated.read_bytes(), original)

        self.output.unlink()
        result = self.run_tool(output=self.generated)
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("must be different", result.stderr)
        self.assertEqual(self.generated.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
