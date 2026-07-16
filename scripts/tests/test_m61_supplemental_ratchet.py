from __future__ import annotations

import contextlib
import csv
import hashlib
import importlib.util
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]


def load_script(name: str):
    path = SCRIPTS / name
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


native_ratchet = load_script("aggregate-native-ratchet.py")
candidate_parity = load_script("check-native-candidate-parity.py")

FROZEN_SUPPLEMENTAL_BYTES = (
    b'"kind","workload","input","metric","ceiling","unit","source_value",'
    b'"spread_pct","notes"\n'
    b'"memory_peak","","","track_a_peak_working_set_bytes","125000000",'
    b'"bytes","125000000","0","M61 predeclared supplemental Track A '
    b"pooled-worst cap; frozen before accepted measurement; does not replace "
    b'm55-thresholds.csv."\n'
)
FROZEN_SUPPLEMENTAL_SHA256 = (
    "d52d064f410df36c1c22dd5523430062563a17bb9f2f63253b607d211badefd7"
)


class SupplementalRatchetTests(unittest.TestCase):
    def setUp(self):
        test_parent = SCRIPTS.parent / "target"
        test_parent.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(
            prefix="m61-supplemental-ratchet-test-", dir=test_parent
        )
        self.root = Path(self.temp.name)
        self.thresholds = self.root / "thresholds.csv"
        self.supplemental_thresholds = self.root / "m61-threshold.csv"
        self.output = self.root / "gate-verdict.csv"
        self.sidecar = self.root / "gate-verdict.provenance.json"
        self.supplemental_output = self.root / "m61-memory-verdict.csv"
        self.supplemental_sidecar = (
            self.root / "m61-memory-verdict.provenance.json"
        )
        self.write_primary_thresholds()
        self.supplemental_thresholds.write_bytes(FROZEN_SUPPLEMENTAL_BYTES)

    def tearDown(self):
        self.temp.cleanup()

    def write_primary_thresholds(
        self,
        *,
        memory_ceiling="195028378",
        memory_unit="bytes",
    ):
        rows = [
            {
                "kind": "latency_ratio",
                "workload": "key_sequence_process_with_context",
                "input": input_text,
                "metric": "yune_librime_median_ratio",
                "ceiling": "2",
                "unit": "x",
            }
            for input_text in native_ratchet.FROZEN_TRACK_A_INPUTS
        ]
        rows.extend(
            [
                {
                    "kind": "memory_peak",
                    "workload": "",
                    "input": "",
                    "metric": "track_a_peak_working_set_bytes",
                    "ceiling": memory_ceiling,
                    "unit": memory_unit,
                },
                {
                    "kind": "latency_absolute_us",
                    "workload": (
                        "track-b-product/key_sequence_process_with_context"
                    ),
                    "input": "trackb",
                    "metric": "median_us",
                    "ceiling": "100",
                    "unit": "us",
                },
            ]
        )
        with self.thresholds.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(
                handle, fieldnames=list(rows[0]), lineterminator="\n"
            )
            writer.writeheader()
            writer.writerows(rows)

    def write_run(self, number: int, memory_observed: int) -> Path:
        run = self.root / f"run-{number}"
        run.mkdir()
        receipt_fields = {
            "format_version": "1",
            "source_commit": "a" * 40,
            "source_tree": "e" * 40,
            "source_clean": "True",
            "source_content_binding_sha256": "3" * 64,
            "benchmark_script_sha256": "2" * 64,
            "benchmark_rust_source_sha256": "5" * 64,
            "cargo_lock_sha256": "6" * 64,
            "rustc_identity_sha256": "7" * 64,
            "cargo_identity_sha256": "8" * 64,
            "cargo_command": "cargo bench --no-run",
            "native_benchmark_build_command": "cargo bench --no-run",
            "cargo_target_root": "/external/native-benchmark-target",
            "native_benchmark_executable_path": (
                "/external/native-benchmark-target/native-benchmark"
            ),
            "native_benchmark_executable_sha256": "1" * 64,
        }
        receipt_path = run / "native-benchmark-build-receipt.txt"
        receipt_path.write_text(
            "".join(f"{key}={value}\n" for key, value in receipt_fields.items()),
            encoding="utf-8",
        )
        environment = {
            "source_commit": "a" * 40,
            "source_tree": "e" * 40,
            "yune_git_head": "a" * 40,
            "source_clean": "True",
            "allow_dirty": "False",
            "source_content_binding_sha256": "3" * 64,
            "measured_yune_dll_sha256": "b" * 64,
            "upstream_rime_dll_sha256": "c" * 64,
            "upstream_shared_tree_sha256": "e" * 64,
            "upstream_build_tree_sha256": "f" * 64,
            "product_schema_tree_sha256": "d" * 64,
            "native_benchmark_executable": (
                "/external/native-benchmark-target/native-benchmark"
            ),
            "native_benchmark_executable_sha256": "1" * 64,
            "native_benchmark_receipt_sha256": hashlib.sha256(
                receipt_path.read_bytes()
            ).hexdigest(),
            "native_benchmark_executable_prebuilt": "True",
            "native_benchmark_build_performed": "False",
            "benchmark_script_sha256": "2" * 64,
            "track_a_storage_mode": "byte-backed",
            "track_a_inputs": ",".join(native_ratchet.FROZEN_TRACK_A_INPUTS),
            "track_b_inputs": "trackb",
            "iterations": "9",
            "session_iterations": "60",
            "key_iterations": "80",
            "deploy_product_before_benchmark": "True",
            "skip_track_b": "False",
        }
        self.write_candidate_packet(run, environment)
        (run / "environment.txt").write_text(
            "".join(f"{key}={value}\n" for key, value in environment.items()),
            encoding="utf-8",
        )
        (run / "run-status.txt").write_text(
            "status=complete\ndate_utc=2026-01-01T00:00:00Z\ndetail=\n",
            encoding="utf-8",
        )
        (run / "memory-owner-profile.csv").write_text(
            "owner_id,mapping_mode\n"
            "poet.vocabulary,poet_bin:byte_backed:mmap\n",
            encoding="utf-8",
        )
        with (run / "summary-comparison.csv").open(
            "w", encoding="utf-8", newline=""
        ) as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=[
                    "track",
                    "workload",
                    "input",
                    "yune_librime_median_ratio",
                ],
                lineterminator="\n",
            )
            writer.writeheader()
            writer.writerows(
                {
                    "track": "track-a-comparison",
                    "workload": "key_sequence_process_with_context",
                    "input": input_text,
                    "yune_librime_median_ratio": "1",
                }
                for input_text in native_ratchet.FROZEN_TRACK_A_INPUTS
            )
        with (run / "summary.csv").open(
            "w", encoding="utf-8", newline=""
        ) as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=[
                    "engine",
                    "track",
                    "schema_id",
                    "workload",
                    "input",
                    "samples",
                    "median_private_bytes",
                ],
                lineterminator="\n",
            )
            writer.writeheader()
            writer.writerows(
                {
                    "engine": "yune",
                    "track": "track-a-comparison",
                    "schema_id": "luna_pinyin",
                    "workload": "key_sequence_process_with_context",
                    "input": input_text,
                    "samples": "80",
                    "median_private_bytes": "50000000",
                }
                for input_text in native_ratchet.FROZEN_TRACK_A_INPUTS
            )
        with (run / "threshold-check.csv").open(
            "w", encoding="utf-8", newline=""
        ) as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=[
                    "kind",
                    "workload",
                    "input",
                    "metric",
                    "observed",
                    "ceiling",
                    "unit",
                    "status",
                ],
                lineterminator="\n",
            )
            writer.writeheader()
            writer.writerows(
                [
                    {
                        "kind": "latency_ratio",
                        "workload": "key_sequence_process_with_context",
                        "input": input_text,
                        "metric": "yune_librime_median_ratio",
                        "observed": "1",
                        "ceiling": "2",
                        "unit": "x",
                        "status": "pass",
                    }
                    for input_text in native_ratchet.FROZEN_TRACK_A_INPUTS
                ]
                + [
                    {
                        "kind": "memory_peak",
                        "workload": "",
                        "input": "",
                        "metric": "track_a_peak_working_set_bytes",
                        "observed": str(memory_observed),
                        "ceiling": "195028378",
                        "unit": "bytes",
                        "status": "pass",
                    },
                    {
                        "kind": "latency_absolute_us",
                        "workload": (
                            "track-b-product/"
                            "key_sequence_process_with_context"
                        ),
                        "input": "trackb",
                        "metric": "median_us",
                        "observed": "50",
                        "ceiling": "100",
                        "unit": "us",
                        "status": "pass",
                    },
                ]
            )
        return run

    def write_candidate_packet(
        self, run: Path, environment: dict[str, str]
    ) -> None:
        inputs = run / "candidate-parity-inputs.csv"
        inputs.write_bytes(
            (
                "input\n"
                + "\n".join(native_ratchet.FROZEN_TRACK_A_INPUTS)
                + "\n"
            ).encode("utf-8")
        )
        snapshot = run / "candidate_snapshots.csv"
        rows = []
        for engine in candidate_parity.ENGINES:
            for input_text in native_ratchet.FROZEN_TRACK_A_INPUTS:
                for index in range(5):
                    rows.append(
                        {
                            "engine": engine,
                            "track": "track-a-comparison",
                            "schema_id": "luna_pinyin",
                            "input": input_text,
                            "candidate_index": str(index),
                            "candidate_count": "5",
                            "page_size": "5",
                            "page_no": "0",
                            "is_last_page": "0",
                            "highlighted_index": "0",
                            "composition_preedit": input_text,
                            "text": f"{input_text}-{index}",
                            "comment": "",
                        }
                    )
        with snapshot.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(
                handle,
                fieldnames=candidate_parity.SNAPSHOT_HEADER,
                lineterminator="\n",
            )
            writer.writeheader()
            writer.writerows(rows)
        result = candidate_parity.main(
            [
                "--snapshot-csv",
                str(snapshot),
                "--expected-inputs-csv",
                str(inputs),
                "--output-dir",
                str(run),
                "--source-commit",
                environment["source_commit"],
                "--source-tree",
                environment["source_tree"],
                "--oracle-binary-sha256",
                environment["upstream_rime_dll_sha256"],
                "--oracle-shared-tree-sha256",
                environment["upstream_shared_tree_sha256"],
                "--oracle-build-tree-sha256",
                environment["upstream_build_tree_sha256"],
            ]
        )
        if result != 0:
            raise AssertionError(f"candidate test packet failed with {result}")
        receipt = native_ratchet._read_key_value_file(
            run / "candidate-parity-verdict.txt", required=True
        )
        environment.update(
            {
                "candidate_parity_tool_sha256": receipt["tool_sha256"],
                "candidate_parity_expected_inputs_sha256": receipt[
                    "expected_inputs_sha256"
                ],
                "candidate_parity_snapshot_sha256": receipt["snapshot_sha256"],
                "candidate_parity_csv_sha256": receipt["parity_csv_sha256"],
                "candidate_parity_detail_sha256": receipt["detail_csv_sha256"],
                "candidate_parity_verdict_sha256": hashlib.sha256(
                    (run / "candidate-parity-verdict.txt").read_bytes()
                ).hexdigest(),
            }
        )

    def run_tool(
        self,
        runs: list[Path],
        *,
        include_supplemental=True,
        extra_args: list[str] | None = None,
    ) -> tuple[int, str]:
        argv = [
            "--thresholds",
            str(self.thresholds),
            "--expected-runs",
            "5",
        ]
        for run in runs:
            argv.extend(["--run", str(run)])
        argv.extend(["--output", str(self.output)])
        if include_supplemental:
            argv.extend(
                [
                    "--supplemental-thresholds",
                    str(self.supplemental_thresholds),
                    "--supplemental-output",
                    str(self.supplemental_output),
                ]
            )
        argv.extend(extra_args or [])
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            result = native_ratchet.main(argv)
        return result, stderr.getvalue()

    def test_frozen_supplemental_bytes_and_hash_are_exact(self):
        self.assertEqual(len(FROZEN_SUPPLEMENTAL_BYTES), 307)
        self.assertEqual(
            hashlib.sha256(FROZEN_SUPPLEMENTAL_BYTES).hexdigest(),
            FROZEN_SUPPLEMENTAL_SHA256,
        )

    def test_supplemental_uses_primary_observations_and_requires_all_five(self):
        runs = [
            self.write_run(index, observed)
            for index, observed in enumerate(
                [120000000, 121000000, 122000000, 123000000, 124000000],
                start=1,
            )
        ]
        primary_before = self.thresholds.read_bytes()
        result, stderr = self.run_tool(runs)
        self.assertEqual((result, stderr), (0, ""))
        self.assertEqual(self.thresholds.read_bytes(), primary_before)
        with self.output.open(encoding="utf-8") as handle:
            primary_rows = list(csv.DictReader(handle))
        with self.supplemental_output.open(encoding="utf-8") as handle:
            supplemental_rows = list(csv.DictReader(handle))
        self.assertEqual(len(primary_rows), 19)
        self.assertEqual(len(supplemental_rows), 1)
        row = supplemental_rows[0]
        self.assertEqual(row["run1_observed"], "120000000")
        self.assertEqual(row["run5_observed"], "124000000")
        self.assertEqual(row["worst_observed"], "124000000")
        self.assertEqual(row["individual_failures"], "0")
        self.assertEqual(row["verdict"], "pass")
        primary_provenance = json.loads(
            self.sidecar.read_text(encoding="utf-8")
        )
        supplemental_provenance = json.loads(
            self.supplemental_sidecar.read_text(encoding="utf-8")
        )
        self.assertEqual(
            primary_provenance["supplemental_evaluation"]["thresholds"][
                "sha256"
            ],
            FROZEN_SUPPLEMENTAL_SHA256,
        )
        self.assertEqual(
            supplemental_provenance["evaluation"],
            "m61-supplemental-memory",
        )
        for option in (
            "--thresholds",
            "--run",
            "--output",
            "--supplemental-thresholds",
            "--supplemental-output",
        ):
            index = supplemental_provenance["effective_argv"].index(option)
            self.assertFalse(
                Path(
                    supplemental_provenance["effective_argv"][index + 1]
                ).is_absolute()
            )

    def test_one_supplemental_failure_is_a_measured_red_not_a_median_pass(self):
        runs = [
            self.write_run(index, observed)
            for index, observed in enumerate(
                [120000000, 121000000, 122000000, 123000000, 126000000],
                start=1,
            )
        ]
        result, stderr = self.run_tool(runs)
        self.assertEqual((result, stderr), (1, ""))
        with self.output.open(encoding="utf-8") as handle:
            primary_memory = next(
                row
                for row in csv.DictReader(handle)
                if row["metric"] == "track_a_peak_working_set_bytes"
            )
        with self.supplemental_output.open(encoding="utf-8") as handle:
            supplemental = next(csv.DictReader(handle))
        self.assertEqual(primary_memory["verdict"], "pass")
        self.assertEqual(supplemental["median_observed"], "122000000")
        self.assertEqual(supplemental["individual_failures"], "1")
        self.assertEqual(supplemental["verdict"], "fail")

    def test_supplemental_arguments_are_paired_and_single_use(self):
        runs = [self.write_run(index, 120000000) for index in range(1, 6)]
        cases = (
            [
                "--supplemental-thresholds",
                str(self.supplemental_thresholds),
            ],
            ["--supplemental-output", str(self.supplemental_output)],
            [
                "--supplemental-thresholds",
                str(self.supplemental_thresholds),
                "--supplemental-thresholds",
                str(self.supplemental_thresholds),
                "--supplemental-output",
                str(self.supplemental_output),
            ],
            [
                "--supplemental-thresholds",
                str(self.supplemental_thresholds),
                "--supplemental-output",
                str(self.supplemental_output),
                "--supplemental-output",
                str(self.supplemental_output),
            ],
        )
        for extra_args in cases:
            with self.subTest(extra_args=extra_args):
                result, stderr = self.run_tool(
                    runs,
                    include_supplemental=False,
                    extra_args=extra_args,
                )
                self.assertEqual(result, 2)
                self.assertTrue(
                    "must be supplied together" in stderr
                    or "may appear only once" in stderr,
                    stderr,
                )
                for output in (
                    self.output,
                    self.sidecar,
                    self.supplemental_output,
                    self.supplemental_sidecar,
                ):
                    self.assertFalse(output.exists(), str(output))

    def test_modified_supplemental_bytes_are_rejected_by_frozen_hash(self):
        runs = [self.write_run(index, 120000000) for index in range(1, 6)]
        self.supplemental_thresholds.write_bytes(
            FROZEN_SUPPLEMENTAL_BYTES.replace(
                b"125000000", b"125000001", 1
            )
        )
        result, stderr = self.run_tool(runs)
        self.assertEqual(result, 2)
        self.assertIn("supplemental threshold SHA-256", stderr)
        for output in (
            self.output,
            self.sidecar,
            self.supplemental_output,
            self.supplemental_sidecar,
        ):
            self.assertFalse(output.exists(), str(output))

    def test_supplemental_threshold_structure_unit_and_tightness_are_strict(self):
        primary = native_ratchet.read_thresholds(self.thresholds)
        supplemental = native_ratchet.read_thresholds(
            self.supplemental_thresholds
        )
        self.assertEqual(
            native_ratchet._validate_supplemental_threshold(
                primary, supplemental
            ),
            supplemental[0],
        )
        valid = supplemental[0]
        bad_cases = (
            (
                "multiple",
                primary,
                [valid, valid],
            ),
            (
                "wrong-key",
                primary,
                [
                    native_ratchet.Threshold(
                        native_ratchet.MetricKey(
                            "memory_peak", "", "", "other"
                        ),
                        valid.ceiling,
                        valid.ceiling_text,
                        valid.unit,
                    )
                ],
            ),
            (
                "wrong-cap",
                primary,
                [
                    native_ratchet.Threshold(
                        valid.key,
                        native_ratchet.Decimal("125000001"),
                        "125000001",
                        valid.unit,
                    )
                ],
            ),
            (
                "wrong-unit",
                primary,
                [
                    native_ratchet.Threshold(
                        valid.key,
                        valid.ceiling,
                        valid.ceiling_text,
                        "kib",
                    )
                ],
            ),
            (
                "missing-primary-key",
                [threshold for threshold in primary if threshold.key != valid.key],
                supplemental,
            ),
            (
                "not-tighter",
                [
                    native_ratchet.Threshold(
                        threshold.key,
                        valid.ceiling
                        if threshold.key == valid.key
                        else threshold.ceiling,
                        valid.ceiling_text
                        if threshold.key == valid.key
                        else threshold.ceiling_text,
                        threshold.unit,
                    )
                    for threshold in primary
                ],
                supplemental,
            ),
        )
        for label, primary_rows, supplemental_rows in bad_cases:
            with self.subTest(label=label):
                with self.assertRaises(native_ratchet.EvidenceError):
                    native_ratchet._validate_supplemental_threshold(
                        primary_rows, supplemental_rows
                    )

    def test_any_structural_failure_invalidates_all_four_outputs(self):
        runs = [self.write_run(index, 120000000) for index in range(1, 6)]
        for output in (
            self.output,
            self.sidecar,
            self.supplemental_output,
            self.supplemental_sidecar,
        ):
            output.write_text("stale", encoding="utf-8")
            (output.parent / f".{output.name}.stale.tmp").write_text(
                "stale", encoding="utf-8"
            )
        (runs[-1] / "run-status.txt").write_text(
            "status=failed\ndate_utc=2026-01-01T00:00:00Z\ndetail=red\n",
            encoding="utf-8",
        )
        result, stderr = self.run_tool(runs)
        self.assertEqual(result, 2)
        self.assertIn("benchmark run status must be complete", stderr)
        for output in (
            self.output,
            self.sidecar,
            self.supplemental_output,
            self.supplemental_sidecar,
        ):
            self.assertFalse(output.exists(), str(output))
            self.assertFalse(
                (output.parent / f".{output.name}.stale.tmp").exists()
            )

    def test_supplemental_output_collisions_fail_before_input_mutation(self):
        runs = [self.write_run(index, 120000000) for index in range(1, 6)]
        threshold_before = self.supplemental_thresholds.read_bytes()
        self.supplemental_output = self.supplemental_thresholds
        self.supplemental_sidecar = native_ratchet._sidecar_path(
            self.supplemental_output
        )
        for safe_output in (
            self.output,
            self.sidecar,
            self.supplemental_sidecar,
        ):
            safe_output.write_text("stale", encoding="utf-8")
        result, stderr = self.run_tool(runs)
        self.assertEqual(result, 2)
        self.assertIn("collides with protected supplemental thresholds", stderr)
        self.assertEqual(
            self.supplemental_thresholds.read_bytes(), threshold_before
        )
        for safe_output in (
            self.output,
            self.sidecar,
            self.supplemental_sidecar,
        ):
            self.assertFalse(safe_output.exists(), str(safe_output))

    def test_quad_write_failure_removes_every_final_and_temporary_output(self):
        runs = [self.write_run(index, 120000000) for index in range(1, 6)]
        original_replace = native_ratchet.os.replace
        replace_count = 0

        def fail_third_replace(source, destination):
            nonlocal replace_count
            replace_count += 1
            if replace_count == 3:
                raise OSError("injected write failure")
            original_replace(source, destination)

        native_ratchet.os.replace = fail_third_replace
        try:
            result, stderr = self.run_tool(runs)
        finally:
            native_ratchet.os.replace = original_replace
        self.assertEqual(result, 2)
        self.assertIn("four-output write failed", stderr)
        for output in (
            self.output,
            self.sidecar,
            self.supplemental_output,
            self.supplemental_sidecar,
        ):
            self.assertFalse(output.exists(), str(output))
            self.assertEqual(
                list(output.parent.glob(f".{output.name}.*.tmp")), []
            )


if __name__ == "__main__":
    unittest.main()
