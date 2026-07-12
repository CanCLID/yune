import contextlib
import copy
import csv
import dataclasses
import hashlib
import importlib.util
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = REPO_ROOT / "scripts"


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


classifier = load_module(
    "m59_4c_residual_classifier",
    SCRIPTS / "classify-m59-4c-residuals.py",
)
comparator = load_module(
    "m59_4c_strict_comparator",
    SCRIPTS / "compare-candidate-order.py",
)


class M59Increment4cResidualClassifierTests(unittest.TestCase):
    ORACLE = (
        REPO_ROOT
        / "docs/reports/evidence/m59-canonical-jyutping-reachability-parity"
        / "increment-1-executable-evidence/lane-a-oracle.json"
    )
    INVENTORY = (
        REPO_ROOT
        / "docs/reports/evidence/m59-canonical-jyutping-reachability-parity"
        / "increment-2-profile-paging/opencc-same-code-inventory.csv"
    )
    OPENCC = REPO_ROOT / "crates/yune-core/src/opencc/data/HKVariantsFull.txt"
    FIXTURE = (
        REPO_ROOT
        / "crates/yune-core/tests/fixtures/upstream-1.17.0"
        / "m59-opencc-convert-word.json"
    )
    MANIFEST = (
        REPO_ROOT
        / "crates/yune-core/tests/fixtures/upstream-1.17.0/oracle-manifest.json"
    )
    YUNE_COMMIT = "4" * 40
    YUNE_TREE = "5" * 40
    YUNE_DLL = "6" * 64

    @classmethod
    def setUpClass(cls):
        cls.oracle_document = json.loads(cls.ORACLE.read_text(encoding="utf-8-sig"))
        cls.oracle_sha256 = hashlib.sha256(cls.ORACLE.read_bytes()).hexdigest()
        cls.inventory_bytes = cls.INVENTORY.read_bytes()
        cls.opencc_bytes = cls.OPENCC.read_bytes()
        cls.mappings, cls.inventory_summary = classifier.parse_opencc_inventory(
            cls.inventory_bytes, cls.opencc_bytes
        )

    @staticmethod
    def write_json(path, document):
        path.write_text(
            json.dumps(document, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
            newline="\n",
        )

    def make_actual_document(self):
        runtime_options = {
            "ascii_mode": False,
            "full_shape": False,
            "ascii_punct": False,
            "zh_hans": False,
        }
        return {
            "capture": {
                "engine": "yune",
                "source_commit": self.YUNE_COMMIT,
                "source_tree": self.YUNE_TREE,
                "source_clean": True,
                "source_dirty": False,
                "source_status_short": [],
                "schema_id": "jyut6ping3",
                "modules": ["default"],
                "yune_dll_sha256": self.YUNE_DLL,
                "probe_sha256": classifier.PINNED_CAPTURE_PROBE_SHA256,
                "capture_script_sha256": (
                    classifier.PINNED_CANDIDATE_CAPTURE_SCRIPT_SHA256
                ),
                "oracle_capture_sha256": self.oracle_sha256,
                "source_shared_tree_sha256": "7" * 64,
                "staged_shared_tree_sha256": "8" * 64,
                "default_yaml_overlay_sha256": (
                    classifier.PINNED_DEFAULT_YAML_OVERLAY_SHA256
                ),
                "schema_list_narrowed": True,
                "narrow_schema_list_switch_used": False,
                "schema_list_narrowing_source": "default_yaml_overlay",
                "runtime_options": runtime_options,
                "effective_parameters": {
                    "schema_id": "jyut6ping3",
                    "inputs": list(classifier.CANONICAL_INPUTS),
                    "inputs_source": "oracle_cases",
                    "schema_list_narrowed": True,
                    "narrow_schema_list_switch_used": False,
                    "schema_list_narrowing_source": "default_yaml_overlay",
                    "runtime_options": runtime_options,
                    "expected_yune_dll_sha256": self.YUNE_DLL,
                    "allow_dirty": False,
                    "keep_work_root": False,
                },
            },
            "inputs": list(classifier.CANONICAL_INPUTS),
            "cases": copy.deepcopy(self.oracle_document["cases"]),
        }

    def make_strict_document(self, actual_document, actual_sha256):
        document = comparator.compare_documents(
            self.oracle_document,
            actual_document,
            policy="exact",
            selected_inputs=classifier.CANONICAL_INPUTS,
            exception_policy=None,
        )
        document["provenance"] = {
            "oracle": {"path": "oracle.json", "sha256": self.oracle_sha256},
            "actual": {"path": "actual.json", "sha256": actual_sha256},
            "exceptions": None,
            "tool_path": "scripts/compare-candidate-order.py",
            "tool_version": classifier.PINNED_COMPARATOR_VERSION,
            "tool_sha256": classifier.PINNED_COMPARATOR_SHA256,
            "effective_argv": [],
            "effective_invocation": "test",
        }
        return document

    def make_cli_fixture(self, root):
        actual = root / "actual.json"
        strict = root / "strict.json"
        output = root / "classification.json"
        actual_document = self.make_actual_document()
        self.write_json(actual, actual_document)
        actual_sha = hashlib.sha256(actual.read_bytes()).hexdigest()
        strict_document = self.make_strict_document(actual_document, actual_sha)
        self.write_json(strict, strict_document)
        strict_sha = hashlib.sha256(strict.read_bytes()).hexdigest()
        args = [
            "--oracle",
            str(self.ORACLE),
            "--expected-oracle-sha256",
            classifier.PINNED_LANE_A_ORACLE_SHA256,
            "--actual",
            str(actual),
            "--expected-actual-sha256",
            actual_sha,
            "--strict-comparator",
            str(strict),
            "--expected-strict-comparator-sha256",
            strict_sha,
            "--opencc-inventory",
            str(self.INVENTORY),
            "--expected-opencc-inventory-sha256",
            classifier.PINNED_OPENCC_INVENTORY_SHA256,
            "--opencc-source",
            str(self.OPENCC),
            "--expected-opencc-source-sha256",
            classifier.PINNED_OPENCC_SOURCE_SHA256,
            "--opencc-oracle-fixture",
            str(self.FIXTURE),
            "--expected-opencc-oracle-fixture-sha256",
            classifier.PINNED_OPENCC_FIXTURE_SHA256,
            "--oracle-manifest",
            str(self.MANIFEST),
            "--expected-oracle-manifest-sha256",
            classifier.PINNED_ORACLE_MANIFEST_SHA256,
            "--expected-yune-commit",
            self.YUNE_COMMIT,
            "--expected-yune-tree",
            self.YUNE_TREE,
            "--expected-yune-dll-sha256",
            self.YUNE_DLL,
            "--output",
            str(output),
        ]
        return args, actual, strict, output

    def test_real_pinned_inputs_complete_end_to_end_cli_success(self):
        with tempfile.TemporaryDirectory() as temp:
            args, _, _, output = self.make_cli_fixture(Path(temp))
            self.assertEqual(classifier.main(args), 0)
            result = json.loads(output.read_text(encoding="utf-8"))
            self.assertTrue(result["classification_complete"])
            self.assertEqual(result["verdict"], "pass")
            self.assertEqual(result["summary"]["canonical_inputs"], 13)
            self.assertEqual(result["summary"]["exact_cases"], 13)
            self.assertEqual(result["summary"]["oracle_candidates"], 5705)
            self.assertEqual(result["summary"]["actual_candidates"], 5705)
            self.assertEqual(result["summary"]["opencc_inventory_rows"], 83)
            self.assertEqual(result["summary"]["opencc_inventory_mapping_keys"], 64)
            self.assertEqual(result["summary"]["opencc_source_mapping_keys"], 65)
            self.assertEqual(result["summary"]["applicable_inventory_mappings"], 5)
            self.assertEqual(result["summary"]["applicable_inventory_occurrences"], 14)
            self.assertEqual(result["summary"]["applicable_inventory_rows"], 28)
            self.assertEqual(result["summary"]["opencc_residuals"], 0)
            self.assertEqual(result["summary"]["exceptions"], 0)
            self.assertEqual(result["summary"]["beyond_oracle_depth"], 0)
            self.assertEqual(result["summary"]["comment_mismatches_non_gating"], 0)
            comments = result["comment_field_comparison"]
            self.assertFalse(comments["acceptance_gating"])
            self.assertEqual(comments["total_candidates_compared"], 5705)
            self.assertEqual(comments["mismatch_count"], 0)
            self.assertEqual(comments["affected_case_count"], 0)
            self.assertEqual(comments["affected_inputs"], [])
            self.assertEqual(len(result["applicable_inventory_occurrences"]), 14)

    def test_create_new_output_refuses_overwrite_and_preserves_bytes(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "result.json"
            classifier.write_json_create_new(output, {"verdict": "pass", "text": "祕"})
            first = output.read_bytes()
            self.assertFalse(first.startswith(b"\xef\xbb\xbf"))
            self.assertNotIn(b"\r", first)
            self.assertTrue(first.endswith(b"\n"))
            with self.assertRaisesRegex(classifier.EvidenceError, "create-new"):
                classifier.write_json_create_new(output, {"verdict": "changed"})
            self.assertEqual(output.read_bytes(), first)

    def test_atomic_publish_failure_leaves_no_output_or_temporary_file(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / "result.json"
            with (
                mock.patch.object(classifier.os, "link", side_effect=OSError("boom")),
                self.assertRaisesRegex(classifier.EvidenceError, "atomically publish"),
            ):
                classifier.write_json_create_new(output, {"verdict": "pass"})
            self.assertFalse(output.exists())
            self.assertEqual(list(root.glob(".result.json.tmp.*")), [])

    def test_cli_existing_output_is_not_removed_or_replaced(self):
        with tempfile.TemporaryDirectory() as temp:
            args, _, _, output = self.make_cli_fixture(Path(temp))
            output.write_bytes(b"owner bytes\n")
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                self.assertEqual(classifier.main(args), 2)
            self.assertIn("create-new", stderr.getvalue())
            self.assertEqual(output.read_bytes(), b"owner bytes\n")

    def test_changed_hash_fails_before_output_creation(self):
        with tempfile.TemporaryDirectory() as temp:
            args, actual, _, output = self.make_cli_fixture(Path(temp))
            actual.write_bytes(actual.read_bytes() + b"\n")
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                self.assertEqual(classifier.main(args), 2)
            self.assertIn("actual SHA-256 mismatch", stderr.getvalue())
            self.assertFalse(output.exists())

    def test_input_mutation_during_classification_fails_without_output(self):
        with tempfile.TemporaryDirectory() as temp:
            args, actual, _, output = self.make_cli_fixture(Path(temp))
            original = classifier.classify_documents

            def classify_then_mutate(*values):
                result = original(*values)
                actual.write_bytes(actual.read_bytes() + b"\n")
                return result

            stderr = io.StringIO()
            with (
                mock.patch.object(
                    classifier, "classify_documents", side_effect=classify_then_mutate
                ),
                contextlib.redirect_stderr(stderr),
            ):
                self.assertEqual(classifier.main(args), 2)
            self.assertIn("actual changed during classification", stderr.getvalue())
            self.assertFalse(output.exists())

    def test_input_and_output_aliases_are_rejected(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            first = root / "first"
            first.write_text("x", encoding="utf-8")
            with self.assertRaisesRegex(classifier.EvidenceError, "aliases"):
                classifier._preflight_paths([("one", first), ("two", first)], root / "out")
            with self.assertRaisesRegex(classifier.EvidenceError, "output aliases"):
                classifier._preflight_paths([("one", first)], first)

    def test_capture_rejects_missing_extra_and_duplicate_canonical_inputs(self):
        for mutation, message in (
            ("missing", "canonical 13"),
            ("extra", "canonical 13"),
            ("duplicate", "duplicate input"),
        ):
            with self.subTest(mutation=mutation):
                document = copy.deepcopy(self.oracle_document)
                if mutation == "missing":
                    document["cases"].pop()
                elif mutation == "extra":
                    document["cases"][-1]["input"] = "extra"
                else:
                    document["cases"].append(copy.deepcopy(document["cases"][0]))
                with self.assertRaisesRegex(classifier.EvidenceError, message):
                    classifier.parse_capture(document, "mutated")

    def test_capture_rejects_page_position_and_termination_tampering(self):
        mutations = (
            (lambda case: case.__setitem__("termination_reason", "pagination_error"), "last_page"),
            (
                lambda case: case["all_candidates"][0].__setitem__("global_index", 1),
                "global_index",
            ),
            (lambda case: case["pages"].pop(), "pages count"),
        )
        for mutate, message in mutations:
            with self.subTest(message=message):
                document = copy.deepcopy(self.oracle_document)
                mutate(document["cases"][1])
                with self.assertRaisesRegex(classifier.EvidenceError, message):
                    classifier.parse_capture(document, "mutated")

    def test_inventory_exactly_reconciles_83_rows_to_64_of_65_keys(self):
        summary = self.inventory_summary
        self.assertEqual(summary["inventory_rows"], 83)
        self.assertEqual(summary["source_mapping_keys"], 65)
        self.assertEqual(summary["represented_mapping_keys"], 64)
        self.assertEqual(summary["unrepresented_mapping_keys"], 1)
        self.assertEqual(summary["mapping_mismatches"], 0)
        self.assertEqual(summary["order_mismatches"], 0)
        self.assertEqual(summary["unrepresented_mapping"]["source_line"], 3)

    def mutate_inventory(self, mutation):
        text = self.inventory_bytes.decode("utf-8-sig")
        reader = csv.DictReader(io.StringIO(text, newline=""))
        rows = list(reader)
        mutation(rows)
        output = io.StringIO(newline="")
        writer = csv.DictWriter(output, fieldnames=reader.fieldnames, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)
        return output.getvalue().encode("utf-8")

    def test_inventory_rejects_extra_duplicate_row_and_mapping_order_tampering(self):
        mutations = (
            (lambda rows: rows.append(copy.deepcopy(rows[0])), "exactly 83"),
            (
                lambda rows: rows[0].__setitem__(
                    "outputs", " ".join(reversed(rows[0]["outputs"].split()))
                ),
                "mapping/order mismatch",
            ),
            (
                lambda rows: rows[0].__setitem__(
                    "locations", ";".join(reversed(rows[0]["locations"].split(";")))
                ),
                "locations must preserve output order",
            ),
        )
        for mutation, message in mutations:
            with self.subTest(message=message):
                mutated = self.mutate_inventory(mutation)
                with self.assertRaisesRegex(classifier.EvidenceError, message):
                    classifier.parse_opencc_inventory(mutated, self.opencc_bytes)

    def test_inventory_rejects_changed_source_and_provenance(self):
        source = self.opencc_bytes.replace("僞\t偽 僞".encode(), "僞\t僞 偽".encode(), 1)
        with self.assertRaisesRegex(classifier.EvidenceError, "mapping/order mismatch"):
            classifier.parse_opencc_inventory(self.inventory_bytes, source)
        inventory = self.mutate_inventory(
            lambda rows: rows[0].__setitem__("dictionary_commit", "0" * 40)
        )
        with self.assertRaisesRegex(classifier.EvidenceError, "does not match provenance"):
            classifier.parse_opencc_inventory(inventory, self.opencc_bytes)

    def test_opencc_fixture_and_manifest_are_jointly_verified(self):
        fixture = json.loads(self.FIXTURE.read_text(encoding="utf-8-sig"))
        manifest = json.loads(self.MANIFEST.read_text(encoding="utf-8-sig"))
        summary = classifier.validate_opencc_fixture(
            fixture, manifest, classifier.PINNED_OPENCC_FIXTURE_SHA256
        )
        self.assertEqual(summary["fixture_cases"], 5)
        self.assertTrue(summary["whole_word_one_to_many"])
        self.assertTrue(summary["stable_dedup"])
        self.assertTrue(summary["partial_segmentation_default_only"])

    def test_opencc_fixture_rejects_changed_observation_and_manifest_duplicates(self):
        fixture = json.loads(self.FIXTURE.read_text(encoding="utf-8-sig"))
        manifest = json.loads(self.MANIFEST.read_text(encoding="utf-8-sig"))
        fixture["cases"][0]["all_candidates"][0]["text"] = "changed"
        fixture["cases"][0]["selected_candidates"][0]["text"] = "changed"
        fixture["cases"][0]["pages"][0]["candidates"][0]["text"] = "changed"
        with self.assertRaisesRegex(classifier.EvidenceError, "observations changed"):
            classifier.validate_opencc_fixture(
                fixture, manifest, classifier.PINNED_OPENCC_FIXTURE_SHA256
            )
        fixture = json.loads(self.FIXTURE.read_text(encoding="utf-8-sig"))
        row = next(
            item
            for item in manifest["files"]
            if item.get("path") == "m59-opencc-convert-word.json"
        )
        manifest["files"].append(copy.deepcopy(row))
        with self.assertRaisesRegex(classifier.EvidenceError, "exactly one"):
            classifier.validate_opencc_fixture(
                fixture, manifest, classifier.PINNED_OPENCC_FIXTURE_SHA256
            )

    def test_strict_comparator_rejects_any_exception_policy_or_semantic_tamper(self):
        actual = self.make_actual_document()
        actual_bytes = (json.dumps(actual, ensure_ascii=False) + "\n").encode("utf-8")
        actual_sha = hashlib.sha256(actual_bytes).hexdigest()
        strict = self.make_strict_document(actual, actual_sha)
        strict["provenance"]["exceptions"] = {"owner_signed": True}
        with self.assertRaisesRegex(classifier.EvidenceError, "exception policy"):
            classifier.validate_strict_comparator(
                strict, self.oracle_document, actual, self.oracle_sha256, actual_sha
            )
        strict = self.make_strict_document(actual, actual_sha)
        strict["cases"][0]["accepted_exceptions"] = ["class-2"]
        with self.assertRaisesRegex(classifier.EvidenceError, "differs from exact recomputation"):
            classifier.validate_strict_comparator(
                strict, self.oracle_document, actual, self.oracle_sha256, actual_sha
            )

    def test_strict_comparator_rejects_red_all_accepted_record(self):
        actual = self.make_actual_document()
        actual["cases"][0]["all_candidates"][0]["text"] = "changed"
        actual["cases"][0]["selected_candidates"][0]["text"] = "changed"
        actual["cases"][0]["pages"][0]["candidates"][0]["text"] = "changed"
        actual_bytes = (json.dumps(actual, ensure_ascii=False) + "\n").encode("utf-8")
        actual_sha = hashlib.sha256(actual_bytes).hexdigest()
        strict = self.make_strict_document(actual, actual_sha)
        self.assertFalse(strict["all_accepted"])
        with self.assertRaisesRegex(classifier.EvidenceError, "all_accepted"):
            classifier.validate_strict_comparator(
                strict, self.oracle_document, actual, self.oracle_sha256, actual_sha
            )

    def test_classification_rejects_preedit_and_inventory_position_tampering(self):
        oracle_cases = classifier.parse_capture(self.oracle_document, "oracle")
        actual_cases = dict(oracle_cases)
        actual_cases["be"] = dataclasses.replace(
            actual_cases["be"],
            preedit="changed",
            control={**actual_cases["be"].control, "preedit": "changed"},
        )
        fixture_summary = {
            "fixture_cases": 5,
            "whole_word_one_to_many": True,
            "stable_dedup": True,
            "partial_segmentation_default_only": True,
            "pass_through_control": True,
            "manifest_rows": 1,
        }
        strict_summary = {"all_accepted": True}
        with self.assertRaisesRegex(classifier.EvidenceError, "control fields differ"):
            classifier.classify_documents(
                oracle_cases,
                actual_cases,
                self.mappings,
                self.inventory_summary,
                strict_summary,
                fixture_summary,
            )

        actual_cases = dict(oracle_cases)
        target = actual_cases["n"]
        rows = list(target.candidates)
        rows[71] = dataclasses.replace(rows[71], text="changed")
        actual_cases["n"] = dataclasses.replace(target, candidates=tuple(rows))
        with self.assertRaisesRegex(classifier.EvidenceError, "candidate text/order"):
            classifier.classify_documents(
                oracle_cases,
                actual_cases,
                self.mappings,
                self.inventory_summary,
                strict_summary,
                fixture_summary,
            )

    def test_opencc_occurrence_requires_actual_code_provenance(self):
        oracle_cases = classifier.parse_capture(self.oracle_document, "oracle")
        actual_cases = dict(oracle_cases)
        target = actual_cases["n"]
        rows = list(target.candidates)
        rows[71] = dataclasses.replace(rows[71], comment="wrong1")
        actual_cases["n"] = dataclasses.replace(target, candidates=tuple(rows))
        with self.assertRaisesRegex(classifier.EvidenceError, "lost code provenance"):
            classifier._opencc_occurrences(oracle_cases, actual_cases, self.mappings)

    def test_comment_comparison_is_deterministic_and_explicitly_non_gating(self):
        oracle_cases = classifier.parse_capture(self.oracle_document, "oracle")
        actual_cases = dict(oracle_cases)
        target = actual_cases["be"]
        rows = list(target.candidates)
        oracle_comment = rows[0].comment
        rows[0] = dataclasses.replace(rows[0], comment="changed comment")
        actual_cases["be"] = dataclasses.replace(target, candidates=tuple(rows))

        summary = classifier._comment_comparison_summary(
            oracle_cases, actual_cases
        )
        expected_tuples = [["be", 0, oracle_comment, "changed comment"]]
        expected_hash = hashlib.sha256(
            json.dumps(
                expected_tuples,
                ensure_ascii=False,
                separators=(",", ":"),
            ).encode("utf-8")
        ).hexdigest()
        self.assertFalse(summary["acceptance_gating"])
        self.assertEqual(summary["total_candidates_compared"], 5705)
        self.assertEqual(summary["mismatch_count"], 1)
        self.assertEqual(summary["affected_case_count"], 1)
        self.assertEqual(summary["affected_inputs"], ["be"])
        self.assertEqual(summary["ordered_mismatch_tuples_sha256"], expected_hash)

        fixture_summary = {
            "fixture_cases": 5,
            "whole_word_one_to_many": True,
            "stable_dedup": True,
            "partial_segmentation_default_only": True,
            "pass_through_control": True,
            "manifest_rows": 1,
        }
        result = classifier.classify_documents(
            oracle_cases,
            actual_cases,
            self.mappings,
            self.inventory_summary,
            {"all_accepted": True},
            fixture_summary,
        )
        self.assertEqual(result["verdict"], "pass")
        self.assertEqual(result["summary"]["comment_mismatches_non_gating"], 1)
        self.assertEqual(
            result["comment_field_comparison"]["ordered_mismatch_tuples_sha256"],
            expected_hash,
        )

    def test_capture_provenance_rejects_dirty_or_wrong_source_identity(self):
        actual = self.make_actual_document()
        actual["capture"]["source_clean"] = False
        with self.assertRaisesRegex(classifier.EvidenceError, "source_clean"):
            classifier.validate_capture_provenance(
                self.oracle_document,
                actual,
                oracle_sha256=self.oracle_sha256,
                expected_yune_commit=self.YUNE_COMMIT,
                expected_yune_tree=self.YUNE_TREE,
                expected_yune_dll_sha256=self.YUNE_DLL,
            )


if __name__ == "__main__":
    unittest.main()
