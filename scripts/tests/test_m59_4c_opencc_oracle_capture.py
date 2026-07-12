import hashlib
import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / "scripts"
CAPTURE_SCRIPT = SCRIPTS / "capture-m59-opencc-convert-word.ps1"
PROBE = SCRIPTS / "oracle-rime-probe.cs"
OPENCC_SOURCE = (
    ROOT / "crates/yune-core/src/opencc/data/HKVariantsFull.txt"
)
FIXTURE_ROOT = ROOT / "crates/yune-core/tests/fixtures/upstream-1.17.0"
FIXTURE = FIXTURE_ROOT / "m59-opencc-convert-word.json"
MANIFEST = FIXTURE_ROOT / "oracle-manifest.json"
OFFICIAL_ORACLE = Path(r"C:\rime-m59-4c-oracle\dist")

EXPECTED_RIME_DLL_SHA256 = (
    "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b"
)
EXPECTED_DEPLOYER_SHA256 = (
    "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071"
)
EXPECTED_HEADER_SHA256 = (
    "85caf744b4e5405a9a1de9c7aef3affc4ae315f4ae5d7ebdd08e191a2c16dad4"
)
EXPECTED_PROBE_SHA256 = (
    "94f7deb7c3632a6c3c918536295b03d88aa8a80bbbbc9d8a26e896fb70bf07e7"
)
EXPECTED_OPENCC_SHA256 = (
    "145b561c68a697d5f2197da0c091caf4a0e9457f0a4c56cdf2ae7ad4b8ff8cc2"
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for key, item in value.items():
            yield key
            yield from strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)


class M59OpenCcOracleFixtureTests(unittest.TestCase):
    def test_checked_in_opencc_rows_define_order_and_no_exact_partial_key(self):
        mappings = {}
        for line in OPENCC_SOURCE.read_text(encoding="utf-8").splitlines():
            source, converted = line.split("\t", 1)
            self.assertNotIn(source, mappings)
            mappings[source] = converted.split(" ")
        self.assertEqual(mappings["祕"], ["秘", "祕"])
        self.assertEqual(mappings["糉"], ["粽", "糉", "糭"])
        self.assertNotIn("祕糉", mappings)

    def test_capture_script_is_ascii_safe_and_pins_every_external_input(self):
        raw = CAPTURE_SCRIPT.read_bytes()
        source = raw.decode("ascii")
        self.assertFalse(raw.startswith(b"\xef\xbb\xbf"))
        self.assertNotIn(b"\r", raw)
        self.assertTrue(raw.endswith(b"\n"))
        for expected in (
            EXPECTED_RIME_DLL_SHA256,
            EXPECTED_DEPLOYER_SHA256,
            EXPECTED_HEADER_SHA256,
            EXPECTED_PROBE_SHA256,
            EXPECTED_OPENCC_SHA256,
        ):
            self.assertIn(expected, source)
        self.assertIn("[RimeProbe]::Capture(", source)
        self.assertIn('if (-not [bool]$Case["captured_all_pages"]', source)
        self.assertIn('[string]$Case["termination_reason"] -cne "last_page"', source)
        self.assertIn("Assert-FileUnchanged", source)
        self.assertIn("Staged HKVariantsFull.txt bytes differ", source)
        self.assertIn("Output canonical path changed during capture", source)
        self.assertIn("Workspace canonical path changed during capture", source)
        self.assertIn("[System.IO.FileMode]::CreateNew", source)
        self.assertIn('filter_chain = @("simplifier", "uniquifier")', source)
        self.assertIn("  alphabet: abcde", source)
        self.assertIn("$ExactTwoSource = [string][char]0x7955", source)
        self.assertIn("$ExactThreeSource = [string][char]0x7CC9", source)
        self.assertIn("$OriginalFirstSource = [string][char]0x53EA", source)
        self.assertIn(
            "$PassThroughSource = ([string][char]0x7532) + ([string][char]0x4E59)",
            source,
        )
        self.assertNotIn("yune_core", source.lower())

    def test_fixture_is_actual_all_page_upstream_observation(self):
        fixture_raw = FIXTURE.read_bytes()
        fixture = json.loads(fixture_raw.decode("utf-8"))
        self.assertFalse(fixture_raw.startswith(b"\xef\xbb\xbf"))
        self.assertNotIn(b"\r", fixture_raw)
        self.assertNotIn(b"\x00", fixture_raw)
        self.assertTrue(fixture_raw.endswith(b"\n"))

        self.assertEqual(fixture["fixture_version"], 1)
        self.assertEqual(fixture["milestone"], "M59 Increment 4c")
        self.assertEqual(fixture["status"], "pinned_upstream_oracle_capture")
        self.assertEqual(
            fixture["source_row_policy"],
            "m59_minimal_opencc_convert_word_and_default_segmentation_oracle",
        )
        self.assertEqual(fixture["oracle"]["engine"], "rime/librime")
        self.assertEqual(fixture["oracle"]["engine_tag"], "1.17.0")
        self.assertEqual(
            fixture["oracle"]["engine_commit"],
            "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
        )
        self.assertEqual(fixture["capture"]["inputs"], ["a", "b", "c", "d", "e"])
        self.assertEqual(fixture["capture"]["page_size"], 2)
        self.assertEqual(
            fixture["capture"]["filter_chain"], ["simplifier", "uniquifier"]
        )
        self.assertEqual(
            [row["text"] for row in fixture["capture"]["table_rows"]],
            ["祕", "秘", "糉", "祕糉", "只", "甲乙"],
        )

        cases = {case["input"]: case for case in fixture["cases"]}
        self.assertEqual(list(cases), ["a", "b", "c", "d", "e"])
        self.assertTrue(all(case["captured_all_pages"] for case in cases.values()))
        self.assertTrue(
            all(case["termination_reason"] == "last_page" for case in cases.values())
        )

        self.assertEqual(
            [row["text"] for row in cases["a"]["all_candidates"]], ["秘", "祕"]
        )
        self.assertEqual(
            [row["text"] for row in cases["b"]["all_candidates"]],
            ["粽", "糉", "糭"],
        )
        self.assertEqual(
            [[row["text"] for row in page["candidates"]] for page in cases["b"]["pages"]],
            [["粽", "糉"], ["糭"]],
        )
        self.assertEqual(
            [row["global_index"] for row in cases["b"]["all_candidates"]],
            [0, 1, 2],
        )
        self.assertEqual(
            [row["text"] for row in cases["c"]["all_candidates"]], ["秘粽"]
        )
        self.assertEqual(
            [row["text"] for row in cases["d"]["all_candidates"]], ["只", "衹"]
        )
        self.assertEqual(
            [row["text"] for row in cases["e"]["all_candidates"]], ["甲乙"]
        )
        self.assertEqual(cases["a"]["commit_text_preview"], "秘")
        self.assertEqual(cases["b"]["commit_text_preview"], "粽")
        self.assertEqual(cases["c"]["commit_text_preview"], "秘粽")
        self.assertEqual(cases["d"]["commit_text_preview"], "只")
        self.assertEqual(cases["e"]["commit_text_preview"], "甲乙")

        self.assertEqual(
            fixture["tools"]["capture_script"]["sha256"], sha256(CAPTURE_SCRIPT)
        )
        self.assertEqual(fixture["tools"]["oracle_probe"]["sha256"], sha256(PROBE))
        self.assertEqual(
            fixture["data"]["opencc_dictionary"]["sha256"], sha256(OPENCC_SOURCE)
        )
        self.assertEqual(
            fixture["data"]["opencc_dictionary"]["repository_commit"],
            "c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0",
        )
        self.assertEqual(
            fixture["data"]["opencc_dictionary"]["repository_tree"],
            "eb193fb80675ffa60df3c32bf24afa7d7f68617a",
        )
        self.assertEqual(sha256(PROBE), EXPECTED_PROBE_SHA256)
        self.assertEqual(sha256(OPENCC_SOURCE), EXPECTED_OPENCC_SHA256)
        absolute_windows_path = re.compile(r"^[A-Za-z]:[\\/]")
        self.assertFalse(
            any(absolute_windows_path.match(value) for value in strings(fixture))
        )

    def test_manifest_binds_fixture_and_capture_command(self):
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        rows = [
            row
            for row in manifest["files"]
            if row["path"] == "m59-opencc-convert-word.json"
        ]
        self.assertEqual(len(rows), 1)
        row = rows[0]
        fixture = json.loads(FIXTURE.read_text(encoding="utf-8"))
        self.assertEqual(manifest["updated"], fixture["oracle"]["capture_date"])
        self.assertEqual(row["milestone"], "M59 Increment 4c")
        self.assertEqual(row["capture_date"], fixture["oracle"]["capture_date"])
        self.assertEqual(row["sha256"], sha256(FIXTURE))
        self.assertEqual(row["source_row_policy"], fixture["source_row_policy"])
        self.assertEqual(
            row["capture_command"], fixture["provenance"]["capture_command"]
        )
        self.assertIn("never overwrites", row["import_policy"])


@unittest.skipUnless(shutil.which("powershell.exe"), "Windows PowerShell is required")
class M59OpenCcOracleCaptureRuntimeTests(unittest.TestCase):
    def run_capture(self, root: Path, name: str):
        workspace = root / f"{name}-workspace"
        output = root / f"{name}.json"
        completed = subprocess.run(
            [
                shutil.which("powershell.exe"),
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                str(CAPTURE_SCRIPT),
                "-OracleBinaryRoot",
                str(OFFICIAL_ORACLE),
                "-Workspace",
                str(workspace),
                "-Output",
                str(output),
                "-CaptureDate",
                "2026-07-12",
            ],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=120,
        )
        return completed, workspace, output

    @unittest.skipUnless(
        (OFFICIAL_ORACLE / "lib/rime.dll").is_file()
        and (OFFICIAL_ORACLE / "bin/rime_deployer.exe").is_file()
        and (OFFICIAL_ORACLE / "include/rime_api.h").is_file(),
        "Pinned librime 1.17.0 oracle distribution is unavailable",
    )
    def test_two_fresh_captures_are_byte_identical_to_fixture(self):
        with tempfile.TemporaryDirectory(prefix="m59-opencc-capture-") as temp:
            root = Path(temp)
            first, first_workspace, first_output = self.run_capture(root, "first")
            second, second_workspace, second_output = self.run_capture(root, "second")
            if first.returncode != 0:
                self.fail(first.stderr or first.stdout)
            if second.returncode != 0:
                self.fail(second.stderr or second.stdout)
            self.assertTrue(first_workspace.is_dir())
            self.assertTrue(second_workspace.is_dir())
            self.assertEqual(first_output.read_bytes(), second_output.read_bytes())
            self.assertEqual(first_output.read_bytes(), FIXTURE.read_bytes())

    def test_wrong_binary_hash_fails_before_workspace_or_output_creation(self):
        with tempfile.TemporaryDirectory(prefix="m59-opencc-bad-hash-") as temp:
            root = Path(temp)
            oracle = root / "oracle"
            (oracle / "lib").mkdir(parents=True)
            (oracle / "lib/rime.dll").write_bytes(b"\x00" * 3_739_136)
            workspace = root / "workspace"
            output = root / "output.json"
            completed = subprocess.run(
                [
                    shutil.which("powershell.exe"),
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    str(CAPTURE_SCRIPT),
                    "-OracleBinaryRoot",
                    str(oracle),
                    "-Workspace",
                    str(workspace),
                    "-Output",
                    str(output),
                    "-CaptureDate",
                    "2026-07-12",
                ],
                cwd=ROOT,
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=30,
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("rime_dll SHA-256 mismatch", completed.stderr)
            self.assertFalse(workspace.exists())
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
