from __future__ import annotations

import csv
import hashlib
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from decimal import Decimal
from pathlib import Path
from unittest import mock


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
TOOL = SCRIPTS / "check-native-candidate-parity.py"
POWERSHELL_WRAPPER = SCRIPTS / "benchmark-native-rime-inprocess.ps1"
MACOS_WRAPPER = SCRIPTS / "benchmark-native-rime-inprocess-macos.sh"


def load_script(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


candidate = load_script("check-native-candidate-parity.py")
ratchet = load_script("aggregate-native-ratchet.py")


class NativeCandidateParityTests(unittest.TestCase):
    def setUp(self) -> None:
        test_parent = REPO_ROOT / "target"
        test_parent.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(
            prefix="native-candidate-parity-test-", dir=test_parent
        )
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    @staticmethod
    def snapshot_rows(*, red: bool = False) -> list[dict[str, str]]:
        rows: list[dict[str, str]] = []
        for engine in candidate.ENGINES:
            for input_text in candidate.FROZEN_INPUTS:
                for index in range(5):
                    text = f"{input_text}-{index}"
                    if (
                        red
                        and engine == "yune"
                        and input_text == candidate.DETAIL_INPUT
                        and index >= 2
                    ):
                        text += "的"
                    rows.append(
                        {
                            "engine": engine,
                            "track": candidate.TRACK,
                            "schema_id": candidate.SCHEMA_ID,
                            "input": input_text,
                            "candidate_index": str(index),
                            "candidate_count": "5",
                            "page_size": "5",
                            "page_no": "0",
                            "is_last_page": "0",
                            "highlighted_index": "0",
                            "composition_preedit": input_text,
                            "text": text,
                            "comment": "",
                        }
                    )
        # Full accepted rounds also contain one Track B page.  The comparator
        # validates its 13-column shape but scopes equality to Track A Luna.
        for index in range(5):
            rows.append(
                {
                    "engine": "yune",
                    "track": "track-b-product",
                    "schema_id": "jyut6ping3_mobile",
                    "input": "trackb",
                    "candidate_index": str(index),
                    "candidate_count": "5",
                    "page_size": "5",
                    "page_no": "0",
                    "is_last_page": "0",
                    "highlighted_index": "0",
                    "composition_preedit": "trackb",
                    "text": f"trackb-{index}",
                    "comment": "",
                }
            )
        return rows

    def write_inputs(self, root: Path) -> Path:
        path = root / "candidate-parity-inputs.csv"
        path.write_bytes(
            ("input\n" + "\n".join(candidate.FROZEN_INPUTS) + "\n").encode(
                "utf-8"
            )
        )
        return path

    def write_snapshot(
        self,
        root: Path,
        rows: list[dict[str, str]],
        *,
        fieldnames: tuple[str, ...] = candidate.SNAPSHOT_HEADER,
    ) -> Path:
        path = root / "candidate_snapshots.csv"
        with path.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(
                handle, fieldnames=fieldnames, lineterminator="\n"
            )
            writer.writeheader()
            for row in rows:
                writer.writerow({key: row[key] for key in fieldnames})
        return path

    def run_tool(
        self,
        name: str,
        *,
        rows: list[dict[str, str]] | None = None,
        fieldnames: tuple[str, ...] = candidate.SNAPSHOT_HEADER,
    ) -> tuple[subprocess.CompletedProcess[str], Path]:
        root = self.root / name
        root.mkdir()
        inputs = self.write_inputs(root)
        snapshot = self.write_snapshot(
            root, self.snapshot_rows() if rows is None else rows, fieldnames=fieldnames
        )
        result = subprocess.run(
            [
                sys.executable,
                "-B",
                str(TOOL),
                "--snapshot-csv",
                str(snapshot),
                "--expected-inputs-csv",
                str(inputs),
                "--output-dir",
                str(root),
                "--source-commit",
                "a" * 40,
                "--source-tree",
                "b" * 40,
                "--oracle-binary-sha256",
                "c" * 64,
                "--oracle-shared-tree-sha256",
                "d" * 64,
                "--oracle-build-tree-sha256",
                "e" * 64,
            ],
            text=True,
            capture_output=True,
            check=False,
        )
        return result, root

    @staticmethod
    def read_receipt(root: Path) -> dict[str, str]:
        return ratchet._read_key_value_file(
            root / "candidate-parity-verdict.txt", required=True
        )

    def test_green_is_deterministic_exact_17_of_17(self) -> None:
        first, first_root = self.run_tool("green-1")
        second, second_root = self.run_tool("green-2")
        self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
        self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
        receipt = self.read_receipt(first_root)
        self.assertEqual(set(receipt), set(candidate.PASS_RECEIPT_KEYS))
        self.assertEqual(receipt["shape"], "PASS")
        self.assertEqual(receipt["exact_inputs"], "17/17")
        self.assertEqual(receipt["mismatches"], "none")
        self.assertEqual(receipt["verdict"], "PASS")
        for name in (
            "candidate-parity.csv",
            "zhongdengchangdu-detail.csv",
            "candidate-parity-verdict.txt",
        ):
            self.assertEqual(
                (first_root / name).read_bytes(), (second_root / name).read_bytes()
            )
        with (first_root / "candidate-parity.csv").open(
            encoding="utf-8", newline=""
        ) as handle:
            parity = list(csv.DictReader(handle))
        self.assertEqual([row["input"] for row in parity], list(candidate.FROZEN_INPUTS))
        self.assertTrue(all(row["exact_match"] == "1" for row in parity))
        with (first_root / "zhongdengchangdu-detail.csv").open(
            encoding="utf-8", newline=""
        ) as handle:
            detail = list(csv.DictReader(handle))
        self.assertEqual(len(detail), 10)
        self.assertEqual({row["engine"] for row in detail}, set(candidate.ENGINES))

    def test_preserved_shape_valid_16_of_17_red_exits_one(self) -> None:
        result, root = self.run_tool("red", rows=self.snapshot_rows(red=True))
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        receipt = self.read_receipt(root)
        self.assertEqual(receipt["shape"], "PASS")
        self.assertEqual(receipt["exact_inputs"], "16/17")
        self.assertEqual(receipt["mismatches"], "zhongdengchangdu")
        self.assertEqual(receipt["verdict"], "FAIL")
        self.assertEqual(receipt["exit_code"], "1")
        with (root / "candidate-parity.csv").open(
            encoding="utf-8", newline=""
        ) as handle:
            parity = {row["input"]: row for row in csv.DictReader(handle)}
        self.assertEqual(parity["zhongdengchangdu"]["mismatch_fields"], "text")

    def test_missing_extra_duplicate_noncontiguous_and_malformed_exit_two(self) -> None:
        base = self.snapshot_rows()
        cases: list[tuple[str, list[dict[str, str]], tuple[str, ...]]] = []
        cases.append(("missing", base[:-6] + base[-5:], candidate.SNAPSHOT_HEADER))
        extra = [dict(row) for row in base]
        unexpected = dict(extra[0])
        unexpected["input"] = "unexpected"
        extra.append(unexpected)
        cases.append(("extra", extra, candidate.SNAPSHOT_HEADER))
        duplicate = [dict(row) for row in base]
        duplicate.append(dict(duplicate[0]))
        cases.append(("duplicate", duplicate, candidate.SNAPSHOT_HEADER))
        noncontiguous = [dict(row) for row in base]
        noncontiguous[4]["candidate_index"] = "5"
        cases.append(("noncontiguous", noncontiguous, candidate.SNAPSHOT_HEADER))
        malformed_header = tuple(
            field for field in candidate.SNAPSHOT_HEADER if field != "comment"
        )
        cases.append(("malformed", base, malformed_header))
        for name, rows, header in cases:
            with self.subTest(name=name):
                result, root = self.run_tool(name, rows=rows, fieldnames=header)
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                receipt = self.read_receipt(root)
                self.assertEqual(receipt["shape"], "FAIL")
                self.assertEqual(receipt["verdict"], "FAIL")
                self.assertEqual(receipt["exit_code"], "2")
                self.assertTrue(any(key.startswith("shape_error_") for key in receipt))

    def test_expected_input_file_is_exact_and_ordered(self) -> None:
        for name, mutate in (
            ("missing-input", lambda values: values[:-1]),
            ("extra-input", lambda values: values + ["extra"]),
            ("duplicate-input", lambda values: values + [values[-1]]),
            ("reordered-input", lambda values: values[1:] + values[:1]),
        ):
            with self.subTest(name=name):
                root = self.root / name
                root.mkdir()
                values = mutate(list(candidate.FROZEN_INPUTS))
                inputs = root / "candidate-parity-inputs.csv"
                inputs.write_bytes(
                    ("input\n" + "\n".join(values) + "\n").encode("utf-8")
                )
                snapshot = self.write_snapshot(root, self.snapshot_rows())
                result = subprocess.run(
                    [
                        sys.executable,
                        str(TOOL),
                        "--snapshot-csv",
                        str(snapshot),
                        "--expected-inputs-csv",
                        str(inputs),
                        "--output-dir",
                        str(root),
                        "--source-commit",
                        "a" * 40,
                        "--source-tree",
                        "b" * 40,
                        "--oracle-binary-sha256",
                        "c" * 64,
                        "--oracle-shared-tree-sha256",
                        "d" * 64,
                        "--oracle-build-tree-sha256",
                        "e" * 64,
                    ],
                    check=False,
                )
                self.assertEqual(result.returncode, 2)

    def test_unusable_output_path_exits_two_not_behavior_red(self) -> None:
        root = self.root / "unusable-output"
        root.mkdir()
        inputs = self.write_inputs(root)
        snapshot = self.write_snapshot(root, self.snapshot_rows())
        output = root / "not-a-directory"
        output.write_text("occupied\n", encoding="utf-8")
        result = subprocess.run(
            [
                sys.executable,
                "-B",
                str(TOOL),
                "--snapshot-csv",
                str(snapshot),
                "--expected-inputs-csv",
                str(inputs),
                "--output-dir",
                str(output),
                "--source-commit",
                "a" * 40,
                "--source-tree",
                "b" * 40,
                "--oracle-binary-sha256",
                "c" * 64,
                "--oracle-shared-tree-sha256",
                "d" * 64,
                "--oracle-build-tree-sha256",
                "e" * 64,
            ],
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("cannot create output directory", result.stderr)
        self.assertNotIn("Traceback", result.stderr)

    def test_receipt_hashes_the_same_input_bytes_that_were_compared(self) -> None:
        root = self.root / "captured-input-bytes"
        root.mkdir()
        inputs = self.write_inputs(root)
        snapshot = self.write_snapshot(root, self.snapshot_rows())
        original_inputs = inputs.read_bytes()
        original_snapshot = snapshot.read_bytes()
        read_inputs = candidate.read_expected_inputs
        read_snapshot = candidate.read_snapshot

        def mutate_inputs_after_capture(payload: bytes):
            inputs.write_bytes(payload + b"extra\n")
            return read_inputs(payload)

        def mutate_snapshot_after_capture(payload: bytes):
            self.write_snapshot(root, self.snapshot_rows(red=True))
            return read_snapshot(payload)

        with mock.patch.object(
            candidate,
            "read_expected_inputs",
            side_effect=mutate_inputs_after_capture,
        ), mock.patch.object(
            candidate,
            "read_snapshot",
            side_effect=mutate_snapshot_after_capture,
        ):
            result = candidate.main(
                [
                    "--snapshot-csv",
                    str(snapshot),
                    "--expected-inputs-csv",
                    str(inputs),
                    "--output-dir",
                    str(root),
                    "--source-commit",
                    "a" * 40,
                    "--source-tree",
                    "b" * 40,
                    "--oracle-binary-sha256",
                    "c" * 64,
                    "--oracle-shared-tree-sha256",
                    "d" * 64,
                    "--oracle-build-tree-sha256",
                    "e" * 64,
                ]
            )
        self.assertEqual(result, 0)
        receipt = self.read_receipt(root)
        self.assertEqual(
            receipt["snapshot_sha256"], hashlib.sha256(original_snapshot).hexdigest()
        )
        self.assertEqual(
            receipt["expected_inputs_sha256"],
            hashlib.sha256(original_inputs).hexdigest(),
        )
        self.assertNotEqual(
            receipt["snapshot_sha256"], hashlib.sha256(snapshot.read_bytes()).hexdigest()
        )
        self.assertNotEqual(
            receipt["expected_inputs_sha256"],
            hashlib.sha256(inputs.read_bytes()).hexdigest(),
        )

    def test_wrappers_run_comparator_before_round_completion(self) -> None:
        powershell = POWERSHELL_WRAPPER.read_text(encoding="utf-8-sig")
        macos = MACOS_WRAPPER.read_text(encoding="utf-8")
        ps_invoke = powershell.index("& python @CandidateParityArgs")
        ps_red = powershell.index("$CandidateParityExitCode -ne 0", ps_invoke)
        ps_complete = powershell.index('Set-RunStatus "complete"', ps_red)
        self.assertLess(ps_invoke, ps_red)
        self.assertLess(ps_red, ps_complete)
        self.assertIn(
            "Native candidate parity check failed with exit code", powershell[ps_red:ps_complete]
        )
        self.assertIn("ExpectedCandidateParityReceiptKeys", powershell[ps_red:ps_complete])
        self.assertIn("candidate parity snapshot changed", powershell[ps_red:ps_complete])
        self.assertIn("candidate parity verdict changed", powershell[ps_red:ps_complete])
        mac_combine = macos.index("\ncombine_outputs\n")
        mac_invoke = macos.index('python3 -B "$candidate_parity_tool"', mac_combine)
        mac_validate = macos.index("\nvalidate_candidate_parity_receipt \\", mac_invoke)
        mac_done = macos.index("\nwrite_readme\n", mac_invoke)
        self.assertLess(mac_combine, mac_invoke)
        self.assertLess(mac_invoke, mac_validate)
        self.assertLess(mac_validate, mac_done)
        self.assertGreaterEqual(macos.count("validate_candidate_parity_receipt"), 3)
        self.assertIn("candidate parity packet changed during macOS measurement", macos)
        self.assertTrue(macos.startswith("#!/usr/bin/env bash\nset -euo pipefail"))

    def test_macos_receipt_validator_rejects_post_compare_mutation(self) -> None:
        result, root = self.run_tool("macos-receipt-validator")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        macos = MACOS_WRAPPER.read_text(encoding="utf-8")
        marker = "validate_candidate_parity_receipt() {\n  python3 - \"$@\" <<'PY'\n"
        validator = macos.split(marker, 1)[1].split("\nPY\n}", 1)[0]
        arguments = [
            str(root / "candidate-parity-verdict.txt"),
            str(TOOL),
            str(root / "candidate_snapshots.csv"),
            str(root / "candidate-parity-inputs.csv"),
            str(root / "candidate-parity.csv"),
            str(root / "zhongdengchangdu-detail.csv"),
            "a" * 40,
            "b" * 40,
            "c" * 64,
            "d" * 64,
            "e" * 64,
            ",".join(candidate.FROZEN_INPUTS),
        ]
        green = subprocess.run(
            [sys.executable, "-c", validator, *arguments],
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(green.returncode, 0, green.stdout + green.stderr)
        snapshot = root / "candidate_snapshots.csv"
        snapshot.write_bytes(snapshot.read_bytes() + b"\n")
        red = subprocess.run(
            [sys.executable, "-c", validator, *arguments],
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertNotEqual(red.returncode, 0)
        self.assertIn("hash mismatch for snapshot_sha256", red.stderr)

    def _candidate_environment(self, root: Path) -> dict[str, str]:
        receipt = self.read_receipt(root)
        verdict_hash = hashlib.sha256(
            (root / "candidate-parity-verdict.txt").read_bytes()
        ).hexdigest()
        return {
            "source_commit": "a" * 40,
            "source_tree": "b" * 40,
            "yune_git_head": "a" * 40,
            "source_clean": "True",
            "allow_dirty": "False",
            "source_content_binding_sha256": "1" * 64,
            "measured_yune_dll_sha256": "2" * 64,
            "upstream_rime_dll_sha256": "c" * 64,
            "upstream_shared_tree_sha256": "d" * 64,
            "upstream_build_tree_sha256": "e" * 64,
            "product_schema_tree_sha256": "3" * 64,
            "native_benchmark_executable_sha256": "4" * 64,
            "native_benchmark_receipt_sha256": "5" * 64,
            "native_benchmark_executable_prebuilt": "True",
            "native_benchmark_build_performed": "False",
            "benchmark_script_sha256": "6" * 64,
            "candidate_parity_tool_sha256": receipt["tool_sha256"],
            "candidate_parity_expected_inputs_sha256": receipt[
                "expected_inputs_sha256"
            ],
            "candidate_parity_snapshot_sha256": receipt["snapshot_sha256"],
            "candidate_parity_csv_sha256": receipt["parity_csv_sha256"],
            "candidate_parity_detail_sha256": receipt["detail_csv_sha256"],
            "candidate_parity_verdict_sha256": verdict_hash,
            "track_a_storage_mode": "owned",
            "track_a_inputs": ",".join(candidate.FROZEN_INPUTS),
            "track_b_inputs": "trackb",
            "iterations": "9",
            "session_iterations": "60",
            "key_iterations": "80",
            "deploy_product_before_benchmark": "True",
            "skip_track_b": "False",
        }

    def rebind_candidate_outputs(
        self, root: Path, environment: dict[str, str]
    ) -> None:
        receipt = self.read_receipt(root)
        bindings = (
            ("parity_csv_sha256", "candidate_parity_csv_sha256", "candidate-parity.csv"),
            (
                "detail_csv_sha256",
                "candidate_parity_detail_sha256",
                "zhongdengchangdu-detail.csv",
            ),
        )
        for receipt_key, environment_key, name in bindings:
            digest = hashlib.sha256((root / name).read_bytes()).hexdigest()
            receipt[receipt_key] = digest
            environment[environment_key] = digest
        (root / "candidate-parity-verdict.txt").write_bytes(
            candidate.receipt_bytes(
                [(key, receipt[key]) for key in candidate.PASS_RECEIPT_KEYS], []
            )
        )
        environment["candidate_parity_verdict_sha256"] = hashlib.sha256(
            (root / "candidate-parity-verdict.txt").read_bytes()
        ).hexdigest()

    def test_five_round_receipts_require_uniform_inputs_tool_outputs_and_identity(self) -> None:
        roots: list[Path] = []
        environments: list[dict[str, str]] = []
        for index in range(1, 6):
            result, root = self.run_tool(f"aggregate-{index}")
            self.assertEqual(result.returncode, 0)
            environment = self._candidate_environment(root)
            ratchet._validate_candidate_parity_receipt(root, environment)
            roots.append(root)
            environments.append(environment)
        runs = [
            ratchet.RunEvidence(root, environment, {}, Decimal("1"))
            for root, environment in zip(roots, environments)
        ]
        ratchet._validate_provenance(runs)

        drift_cases = (
            "candidate_parity_expected_inputs_sha256",
            "candidate_parity_tool_sha256",
            "candidate_parity_csv_sha256",
            "source_commit",
            "upstream_build_tree_sha256",
        )
        for key in drift_cases:
            with self.subTest(key=key):
                changed = [dict(environment) for environment in environments]
                changed[-1][key] = ("f" * 40 if key == "source_commit" else "f" * 64)
                changed_runs = [
                    ratchet.RunEvidence(root, environment, {}, Decimal("1"))
                    for root, environment in zip(roots, changed)
                ]
                with self.assertRaises(ratchet.EvidenceError):
                    ratchet._validate_provenance(changed_runs)

        (roots[-1] / "candidate-parity.csv").write_text("tampered\n", encoding="utf-8")
        with self.assertRaisesRegex(ratchet.EvidenceError, "output hash mismatch"):
            ratchet._validate_candidate_parity_receipt(
                roots[-1], environments[-1]
            )

    def test_aggregator_rejects_hash_consistent_semantic_contradictions(self) -> None:
        parity_result, parity_root = self.run_tool("contradictory-parity")
        self.assertEqual(parity_result.returncode, 0)
        parity_environment = self._candidate_environment(parity_root)
        with (parity_root / "candidate-parity.csv").open(
            encoding="utf-8", newline=""
        ) as handle:
            parity_rows = list(csv.DictReader(handle))
        parity_rows[0]["yune_texts_json"] = '["different","page","but","still","pass"]'
        parity_rows[0]["yune_page_sha256"] = "f" * 64
        with (parity_root / "candidate-parity.csv").open(
            "w", encoding="utf-8", newline=""
        ) as handle:
            writer = csv.DictWriter(
                handle, fieldnames=candidate.PARITY_HEADER, lineterminator="\n"
            )
            writer.writeheader()
            writer.writerows(parity_rows)
        self.rebind_candidate_outputs(parity_root, parity_environment)
        with self.assertRaisesRegex(ratchet.EvidenceError, "contradicts exact_match"):
            ratchet._validate_candidate_parity_receipt(
                parity_root, parity_environment
            )

        detail_result, detail_root = self.run_tool("contradictory-detail")
        self.assertEqual(detail_result.returncode, 0)
        detail_environment = self._candidate_environment(detail_root)
        with (detail_root / "zhongdengchangdu-detail.csv").open(
            encoding="utf-8", newline=""
        ) as handle:
            detail_rows = list(csv.DictReader(handle))
        yune_row = next(row for row in detail_rows if row["engine"] == "yune")
        yune_row["text"] = "different"
        with (detail_root / "zhongdengchangdu-detail.csv").open(
            "w", encoding="utf-8", newline=""
        ) as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=candidate.SNAPSHOT_HEADER,
                lineterminator="\n",
            )
            writer.writeheader()
            writer.writerows(detail_rows)
        self.rebind_candidate_outputs(detail_root, detail_environment)
        with self.assertRaisesRegex(ratchet.EvidenceError, "detail CSV differs"):
            ratchet._validate_candidate_parity_receipt(
                detail_root, detail_environment
            )


if __name__ == "__main__":
    unittest.main()
