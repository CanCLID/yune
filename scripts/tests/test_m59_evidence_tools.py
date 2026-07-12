from __future__ import annotations

import csv
import contextlib
import hashlib
import importlib.util
import io
import json
import os
import shutil
import subprocess
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


candidate_order = load_script("compare-candidate-order.py")
native_ratchet = load_script("aggregate-native-ratchet.py")
luna_curator = load_script("curate-m59-luna-composition.py")
residual_classifier = load_script("classify-m59-4a-residuals.py")


def capture(rows_by_input, *, complete=True):
    return {
        "cases": [
            {
                "input": input_text,
                "page_size": 5,
                "captured_all_pages": complete,
                "all_candidates": [{"text": row} for row in rows],
            }
            for input_text, rows in rows_by_input.items()
        ]
    }


def no_menu_capture(input_text="x"):
    return {
        "cases": [
            {
                "input": input_text,
                "page_size": 0,
                "page_no": 0,
                "num_candidates": 0,
                "is_last_page": False,
                "candidate_pointer_null": True,
                "menu_present": False,
                "termination_reason": "no_menu",
                "captured_all_pages": True,
                "selected_candidates": [],
                "pages": [],
                "all_candidates": [],
            }
        ]
    }


def empty_nonterminal_capture(input_text="x"):
    return {
        "cases": [
            {
                "input": input_text,
                "page_size": 5,
                "page_no": 0,
                "num_candidates": 0,
                "is_last_page": False,
                "menu_present": True,
                "termination_reason": "empty_nonterminal_page",
                "pagination_error": "empty_nonterminal_page_at_page_0",
                "captured_all_pages": False,
                "selected_candidates": [],
                "pages": [
                    {
                        "page_no": 0,
                        "page_size": 5,
                        "is_last_page": False,
                        "candidates": [],
                    }
                ],
                "all_candidates": [],
            }
        ]
    }


class CandidateOrderTests(unittest.TestCase):
    def compare(self, oracle_rows, actual_rows, **kwargs):
        return candidate_order.compare_documents(
            capture({"x": oracle_rows}),
            capture({"x": actual_rows}),
            policy=kwargs.pop("policy", "exact"),
            **kwargs,
        )

    def test_exact_comparison_preserves_duplicate_rows(self):
        result = self.compare(["a", "dup", "dup", "b"], ["a", "dup", "dup", "b"])
        self.assertTrue(result["all_accepted"])
        self.assertEqual(result["cases"][0]["verdict"], "pass")

    def test_dropped_duplicate_is_under_admission_not_a_match(self):
        result = self.compare(["a", "dup", "dup", "b"], ["a", "dup", "b"])
        row = result["cases"][0]
        self.assertFalse(result["all_accepted"])
        self.assertEqual(row["missing_count"], 1)
        self.assertEqual(row["raw_first_mismatch_index"], 2)
        self.assertIn("under-admission", row["failure_classes"])

    def test_same_multiset_in_different_order_fails_positionally(self):
        result = self.compare(["a", "b", "c"], ["b", "a", "c"])
        row = result["cases"][0]
        self.assertEqual(row["missing_count"], 0)
        self.assertEqual(row["extra_count"], 0)
        self.assertEqual(row["raw_first_mismatch_index"], 0)
        self.assertIn("order", row["failure_classes"])

    def test_empty_actual_list_is_full_under_admission(self):
        result = self.compare(["a", "b", "c"], [])
        row = result["cases"][0]
        self.assertEqual(row["missing_count"], 3)
        self.assertEqual(row["raw_first_mismatch_index"], 0)
        self.assertEqual(row["verdict"], "fail")

    def test_explicit_no_menu_is_complete_shape_but_under_admission(self):
        result = candidate_order.compare_documents(
            capture({"x": ["a", "b"]}), no_menu_capture(), policy="exact"
        )
        row = result["cases"][0]
        self.assertEqual(row["missing_count"], 2)
        self.assertIn("under-admission", row["failure_classes"])
        self.assertIn("menu-presence", row["failure_classes"])
        self.assertNotIn("actual-incomplete", row["failure_classes"])
        self.assertFalse(row["menu_present"]["actual"])
        self.assertEqual(row["termination_reason"]["actual"], "no_menu")

        empty_match = candidate_order.compare_documents(
            no_menu_capture(), no_menu_capture(), policy="exact"
        )
        self.assertTrue(empty_match["all_accepted"])

    def test_empty_nonterminal_page_remains_incomplete(self):
        result = candidate_order.compare_documents(
            capture({"x": ["a"]}), empty_nonterminal_capture(), policy="exact"
        )
        row = result["cases"][0]
        self.assertIn("actual-incomplete", row["failure_classes"])
        self.assertIn("under-admission", row["failure_classes"])
        self.assertEqual(
            row["termination_reason"]["actual"], "empty_nonterminal_page"
        )

    def test_invalid_no_menu_shapes_are_structural_failures(self):
        mutations = {
            "positive_page_size": lambda case: case.__setitem__("page_size", 5),
            "float_page_size": lambda case: case.__setitem__("page_size", 0.0),
            "incomplete": lambda case: case.__setitem__("captured_all_pages", False),
            "wrong_reason": lambda case: case.__setitem__(
                "termination_reason", "last_page"
            ),
            "candidate": lambda case: case["all_candidates"].append({"text": "x"}),
            "page": lambda case: case["pages"].append({}),
            "selected": lambda case: case["selected_candidates"].append({"text": "x"}),
            "nonzero_count": lambda case: case.__setitem__("num_candidates", 1),
            "missing_page_no": lambda case: case.pop("page_no"),
            "true_last_page": lambda case: case.__setitem__("is_last_page", True),
            "missing_candidate_pointer_marker": lambda case: case.pop(
                "candidate_pointer_null"
            ),
            "false_candidate_pointer_marker": lambda case: case.__setitem__(
                "candidate_pointer_null", False
            ),
            "pagination_error": lambda case: case.__setitem__(
                "pagination_error", "page_down_not_handled"
            ),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label):
                malformed = no_menu_capture()
                mutate(malformed["cases"][0])
                with self.assertRaises(candidate_order.EvidenceError):
                    candidate_order.compare_documents(
                        capture({"x": ["a"]}), malformed, policy="exact"
                    )

    def test_termination_reason_state_machine_rejects_false_green_shapes(self):
        invalid_mutations = {
            "unknown_reason": lambda case: case.__setitem__(
                "termination_reason", "looks_done"
            ),
            "last_page_incomplete": lambda case: (
                case.__setitem__("termination_reason", "last_page"),
                case.__setitem__("captured_all_pages", False),
            ),
            "last_page_with_error": lambda case: (
                case.__setitem__("termination_reason", "last_page"),
                case.__setitem__("pagination_error", "contradiction"),
            ),
            "max_pages_complete": lambda case: (
                case.__setitem__("termination_reason", "max_pages"),
                case.__setitem__("pagination_error", "max_pages_reached_2000"),
            ),
            "max_pages_without_error": lambda case: (
                case.__setitem__("termination_reason", "max_pages"),
                case.__setitem__("captured_all_pages", False),
            ),
            "incomplete_reason_without_error": lambda case: (
                case.__setitem__("termination_reason", "page_did_not_advance"),
                case.__setitem__("captured_all_pages", False),
            ),
            "complete_legacy_with_error": lambda case: case.__setitem__(
                "pagination_error", "contradiction"
            ),
        }
        for label, mutate in invalid_mutations.items():
            with self.subTest(label=label):
                malformed = capture({"x": ["a"]})
                mutate(malformed["cases"][0])
                with self.assertRaises(candidate_order.EvidenceError):
                    candidate_order.compare_documents(
                        malformed, capture({"x": ["a"]}), policy="exact"
                    )

        valid_last_page = capture({"x": ["a"]})
        valid_last_page["cases"][0]["termination_reason"] = "last_page"
        self.assertTrue(
            candidate_order.compare_documents(
                valid_last_page, valid_last_page, policy="exact"
            )["all_accepted"]
        )

        valid_incomplete = capture({"x": ["a"]}, complete=False)
        valid_incomplete["cases"][0].update(
            {
                "termination_reason": "page_did_not_advance",
                "pagination_error": "page_down_did_not_advance_at_page_0",
            }
        )
        result = candidate_order.compare_documents(
            valid_incomplete, valid_incomplete, policy="exact"
        )
        self.assertFalse(result["all_accepted"])
        self.assertIn("actual-incomplete", result["cases"][0]["failure_classes"])

    def test_oracle_prefix_requires_input_specific_owner_signed_tail(self):
        unsigned = self.compare(["a", "b"], ["a", "b", "tail"], policy="oracle-prefix")
        self.assertFalse(unsigned["all_accepted"])
        self.assertIn(
            "unsigned-beyond-oracle-depth", unsigned["cases"][0]["failure_classes"]
        )

        policy = {
            "schema_version": 1,
            "decision_id": "D-48",
            "owner_signed": True,
            "owner_decision_date": "2026-07-09",
            "allowed_tails": [
                {
                    "input": "x",
                    "start": 2,
                    "actual": ["tail"],
                    "class": "beyond-oracle-depth",
                    "reason": "owner signed this input-specific tail",
                    "owner_signed": True,
                    "owner_decision_date": "2026-07-09",
                }
            ],
        }
        accepted = self.compare(
            ["a", "b"],
            ["a", "b", "tail"],
            policy="oracle-prefix",
            exception_policy=policy,
        )
        self.assertTrue(accepted["all_accepted"])
        self.assertEqual(accepted["cases"][0]["verdict"], "signed-exception")
        self.assertIn("beyond-oracle-depth", accepted["cases"][0]["accepted_exceptions"])

        per_tail_unsigned = json.loads(json.dumps(policy))
        per_tail_unsigned["allowed_tails"][0].pop("owner_signed")
        with self.assertRaises(candidate_order.EvidenceError):
            self.compare(
                ["a", "b"],
                ["a", "b", "tail"],
                policy="oracle-prefix",
                exception_policy=per_tail_unsigned,
            )

        interleaved = self.compare(
            ["a", "b"], ["a", "tail", "b"], policy="oracle-prefix"
        )
        self.assertFalse(interleaved["all_accepted"])
        self.assertIn("order", interleaved["cases"][0]["failure_classes"])

    def test_declared_equal_length_replacement_is_fail_closed(self):
        policy = {
            "schema_version": 1,
            "decision_id": "D-48",
            "owner_signed": True,
            "owner_decision_date": "2026-07-09",
            "allowed_replacements": [
                {
                    "input": "x",
                    "start": 1,
                    "oracle": ["b", "c"],
                    "actual": ["c", "b"],
                    "class": "equal-weight-tie",
                    "reason": "owner signed",
                }
            ],
        }
        result = self.compare(
            ["a", "b", "c", "d"],
            ["a", "c", "b", "d"],
            exception_policy=policy,
        )
        self.assertTrue(result["all_accepted"])
        self.assertEqual(result["cases"][0]["verdict"], "signed-exception")

        with self.assertRaises(candidate_order.EvidenceError):
            self.compare(
                ["a", "b", "c", "d"],
                ["a", "wrong", "b", "d"],
                exception_policy=policy,
            )

        wrong_class = dict(policy)
        wrong_class["allowed_replacements"] = [dict(policy["allowed_replacements"][0])]
        wrong_class["allowed_replacements"][0]["class"] = "new-unsigned-class"
        with self.assertRaises(candidate_order.EvidenceError):
            self.compare(
                ["a", "b", "c", "d"],
                ["a", "c", "b", "d"],
                exception_policy=wrong_class,
            )

        missing_owner_metadata = dict(policy)
        missing_owner_metadata.pop("owner_signed")
        with self.assertRaises(candidate_order.EvidenceError):
            self.compare(
                ["a", "b", "c", "d"],
                ["a", "c", "b", "d"],
                exception_policy=missing_owner_metadata,
            )

    def test_incomplete_and_missing_input_evidence_fails(self):
        result = candidate_order.compare_documents(
            capture({"x": ["a"]}),
            capture({"x": ["a"]}, complete=False),
            policy="exact",
        )
        self.assertFalse(result["all_accepted"])
        self.assertIn("actual-incomplete", result["cases"][0]["failure_classes"])

        with self.assertRaises(candidate_order.EvidenceError):
            candidate_order.compare_documents(
                capture({"x": ["a"]}), capture({"y": ["a"]}), policy="exact"
            )

    def test_page_size_and_explicit_global_positions_are_checked(self):
        oracle = capture({"x": ["a", "b"]})
        actual = capture({"x": ["a", "b"]})
        actual["cases"][0]["page_size"] = 9
        result = candidate_order.compare_documents(oracle, actual, policy="exact")
        self.assertIn("page-size", result["cases"][0]["failure_classes"])

        oracle["cases"][0]["all_candidates"][0]["global_index"] = 1
        with self.assertRaises(candidate_order.EvidenceError):
            candidate_order.compare_documents(oracle, capture({"x": ["a", "b"]}), policy="exact")

        missing_page_size = capture({"x": ["a"]})
        del missing_page_size["cases"][0]["page_size"]
        with self.assertRaises(candidate_order.EvidenceError):
            candidate_order.compare_documents(
                missing_page_size, capture({"x": ["a"]}), policy="exact"
            )

        zero_page_size = capture({"x": ["a"]})
        zero_page_size["cases"][0]["page_size"] = 0
        with self.assertRaises(candidate_order.EvidenceError):
            candidate_order.compare_documents(
                zero_page_size, capture({"x": ["a"]}), policy="exact"
            )

    def test_cli_exit_codes_distinguish_behavior_and_structural_failures(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle = root / "oracle.json"
            actual = root / "actual.json"
            output_json = root / "diff.json"
            output_csv = root / "diff.csv"
            exceptions = root / "exceptions.json"
            oracle.write_text(json.dumps(capture({"x": ["a", "b"]})), encoding="utf-8")
            actual.write_text(json.dumps(capture({"x": ["a"]})), encoding="utf-8")
            exceptions.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "decision_id": "D-48",
                        "owner_signed": True,
                        "owner_decision_date": "2026-07-09",
                    }
                ),
                encoding="utf-8",
            )
            args = [
                "--oracle",
                str(oracle),
                "--actual",
                str(actual),
                "--policy",
                "exact",
                "--exceptions",
                str(exceptions),
                "--output-json",
                str(output_json),
                "--output-csv",
                str(output_csv),
            ]
            self.assertEqual(candidate_order.main(args), 1)
            self.assertTrue(output_json.is_file())
            result = json.loads(output_json.read_text(encoding="utf-8"))
            provenance = result["provenance"]
            self.assertEqual(
                provenance["oracle"]["sha256"], candidate_order._file_sha256(oracle)
            )
            self.assertEqual(
                provenance["actual"]["sha256"], candidate_order._file_sha256(actual)
            )
            self.assertEqual(
                provenance["exceptions"]["sha256"],
                candidate_order._file_sha256(exceptions),
            )
            self.assertEqual(result["tool_version"], candidate_order.TOOL_VERSION)
            self.assertEqual(provenance["oracle"]["path"], "external/oracle")
            self.assertEqual(provenance["actual"]["path"], "external/actual")
            self.assertEqual(provenance["exceptions"]["path"], "external/exceptions")
            self.assertEqual(provenance["tool_path"], "scripts/compare-candidate-order.py")
            self.assertIn("external/output-json", provenance["effective_argv"])
            self.assertIn("external/output-csv", provenance["effective_argv"])
            serialized_provenance = json.dumps(provenance)
            self.assertNotIn(str(root), serialized_provenance)
            self.assertNotIn("Users", serialized_provenance)
            self.assertNotIn(":\\", serialized_provenance)
            self.assertNotIn("\\", provenance["effective_invocation"])
            with output_csv.open(encoding="utf-8", newline="") as handle:
                csv_row = next(csv.DictReader(handle))
            self.assertEqual(
                csv_row["effective_invocation"], provenance["effective_invocation"]
            )
            self.assertNotIn("Users", csv_row["effective_invocation"])

            self.assertEqual(
                candidate_order._logical_path(
                    SCRIPTS / "compare-candidate-order.py", "tool"
                ),
                "scripts/compare-candidate-order.py",
            )

            actual.write_text(json.dumps(capture({"y": ["a"]})), encoding="utf-8")
            stale_json_temp = root / f".{output_json.name}.stale.tmp"
            stale_csv_temp = root / f".{output_csv.name}.stale.tmp"
            stale_json_temp.write_text("stale", encoding="utf-8")
            stale_csv_temp.write_text("stale", encoding="utf-8")
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(candidate_order.main(args), 2)
            self.assertFalse(output_json.exists())
            self.assertFalse(output_csv.exists())
            self.assertFalse(stale_json_temp.exists())
            self.assertFalse(stale_csv_temp.exists())

            output_json.mkdir()
            output_csv.write_text("stale sibling", encoding="utf-8")
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                self.assertEqual(candidate_order.main(args), 2)
            self.assertTrue(output_json.is_dir())
            self.assertFalse(output_csv.exists())
            self.assertIn("paired-output invalidation failed", stderr.getvalue())

    def test_cli_preflight_rejects_all_input_output_aliases_without_mutation(self):
        input_roles = ("oracle", "actual", "exceptions")
        output_roles = ("output_json", "output_csv")
        for output_role in output_roles:
            for input_role in input_roles:
                for canonical_equivalent in (False, True):
                    label = (
                        f"{output_role}_{input_role}_"
                        f"{'canonical' if canonical_equivalent else 'direct'}"
                    )
                    with self.subTest(label=label), tempfile.TemporaryDirectory() as temp:
                        root = Path(temp)
                        alias_parent = root / "alias"
                        alias_parent.mkdir()
                        oracle = root / "oracle.json"
                        actual = root / "actual.json"
                        exceptions = root / "exceptions.json"
                        documents = {
                            oracle: json.dumps(capture({"x": ["a"]})),
                            actual: json.dumps(capture({"x": ["a"]})),
                            exceptions: json.dumps(
                                {
                                    "schema_version": 1,
                                    "decision_id": "D-48",
                                    "owner_signed": True,
                                    "owner_decision_date": "2026-07-09",
                                }
                            ),
                        }
                        for path, content in documents.items():
                            path.write_text(content, encoding="utf-8")
                        input_paths = {
                            "oracle": oracle,
                            "actual": actual,
                            "exceptions": exceptions,
                        }
                        outputs = {
                            "output_json": root / "result.json",
                            "output_csv": root / "result.csv",
                        }
                        target = input_paths[input_role]
                        outputs[output_role] = (
                            alias_parent / ".." / target.name
                            if canonical_equivalent
                            else target
                        )
                        other_output_role = (
                            "output_csv"
                            if output_role == "output_json"
                            else "output_json"
                        )
                        other_output = outputs[other_output_role]
                        other_output.write_bytes(b"preflight must preserve this output")
                        input_snapshots = {
                            path: path.read_bytes() for path in input_paths.values()
                        }
                        args = [
                            "--oracle",
                            str(oracle),
                            "--actual",
                            str(actual),
                            "--policy",
                            "exact",
                            "--exceptions",
                            str(exceptions),
                            "--output-json",
                            str(outputs["output_json"]),
                            "--output-csv",
                            str(outputs["output_csv"]),
                        ]
                        with contextlib.redirect_stderr(io.StringIO()):
                            self.assertEqual(candidate_order.main(args), 2)
                        for path, snapshot in input_snapshots.items():
                            self.assertEqual(path.read_bytes(), snapshot)
                        self.assertEqual(
                            other_output.read_bytes(),
                            b"preflight must preserve this output",
                        )

    def test_cli_preflight_rejects_output_aliases_before_other_structural_errors(self):
        for canonical_equivalent in (False, True):
            for invalid_actual in (False, True):
                with self.subTest(
                    canonical_equivalent=canonical_equivalent,
                    invalid_actual=invalid_actual,
                ), tempfile.TemporaryDirectory() as temp:
                    root = Path(temp)
                    alias_parent = root / "alias"
                    alias_parent.mkdir()
                    oracle = root / "oracle.json"
                    actual = root / "actual.json"
                    collision_output = root / "collision.json"
                    oracle.write_text(
                        json.dumps(capture({"x": ["a"]})), encoding="utf-8"
                    )
                    actual.write_text(
                        "not valid JSON"
                        if invalid_actual
                        else json.dumps(capture({"x": ["a"]})),
                        encoding="utf-8",
                    )
                    collision_output.write_bytes(b"preflight sentinel")
                    output_csv = (
                        alias_parent / ".." / collision_output.name
                        if canonical_equivalent
                        else collision_output
                    )
                    snapshots = {
                        oracle: oracle.read_bytes(),
                        actual: actual.read_bytes(),
                        collision_output: collision_output.read_bytes(),
                    }
                    args = [
                        "--oracle",
                        str(oracle),
                        "--actual",
                        str(actual),
                        "--policy",
                        "exact",
                        "--output-json",
                        str(collision_output),
                        "--output-csv",
                        str(output_csv),
                    ]
                    stderr = io.StringIO()
                    with contextlib.redirect_stderr(stderr):
                        self.assertEqual(candidate_order.main(args), 2)
                    self.assertIn("canonically different", stderr.getvalue())
                    for path, snapshot in snapshots.items():
                        self.assertEqual(path.read_bytes(), snapshot)

    def test_cli_preflight_input_alias_wins_before_invalid_json_and_preserves_all(self):
        for canonical_equivalent in (False, True):
            with self.subTest(
                canonical_equivalent=canonical_equivalent
            ), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                alias_parent = root / "alias"
                alias_parent.mkdir()
                oracle = root / "oracle.json"
                actual = root / "actual.json"
                output_csv = root / "result.csv"
                oracle.write_text(
                    json.dumps(capture({"x": ["a"]})), encoding="utf-8"
                )
                actual.write_text("not valid JSON", encoding="utf-8")
                output_csv.write_bytes(b"preflight must not clean sibling output")
                output_json = (
                    alias_parent / ".." / oracle.name
                    if canonical_equivalent
                    else oracle
                )
                snapshots = {
                    oracle: oracle.read_bytes(),
                    actual: actual.read_bytes(),
                    output_csv: output_csv.read_bytes(),
                }
                args = [
                    "--oracle",
                    str(oracle),
                    "--actual",
                    str(actual),
                    "--policy",
                    "exact",
                    "--output-json",
                    str(output_json),
                    "--output-csv",
                    str(output_csv),
                ]
                stderr = io.StringIO()
                with contextlib.redirect_stderr(stderr):
                    self.assertEqual(candidate_order.main(args), 2)
                self.assertIn("must not alias --oracle", stderr.getvalue())
                for path, snapshot in snapshots.items():
                    self.assertEqual(path.read_bytes(), snapshot)


class LunaCuratorTests(unittest.TestCase):
    def valid_inputs(self):
        cases = []
        for input_text, target in luna_curator.TARGETS.items():
            texts = [target]
            if input_text == "moboyi":
                texts.extend(["a", "b", "c", "d", "tail"])
            all_candidates = []
            pages = []
            for page_no, start in enumerate(range(0, len(texts), 5)):
                page_candidates = []
                for local_index, text in enumerate(texts[start : start + 5]):
                    candidate = {
                        "index": local_index,
                        "text": text,
                        "global_index": start + local_index,
                    }
                    page_candidates.append(candidate)
                    all_candidates.append(dict(candidate))
                pages.append(
                    {
                        "page_no": page_no,
                        "page_size": 5,
                        "is_last_page": start + 5 >= len(texts),
                        "candidates": page_candidates,
                    }
                )
            cases.append(
                {
                    "schema_id": "luna_pinyin",
                    "input": input_text,
                    "rime_get_input": input_text,
                    "processed": [1] * len(input_text),
                    "page_size": 5,
                    "page_no": 0,
                    "is_last_page": pages[0]["is_last_page"],
                    "captured_all_pages": True,
                    "termination_reason": "last_page",
                    "selected_candidates": [dict(row) for row in pages[0]["candidates"]],
                    "pages": pages,
                    "all_candidates": all_candidates,
                }
            )
        return cases

    def valid_composition(self):
        expected_commit = luna_curator.COMPOSITIONS[0][3]
        expected_preedits = {
            "moboyi_page0": "mo bo yi",
            "after_select_mo": expected_commit[0] + "bo yi",
            "bo_pd1": expected_commit[0] + "bo yi",
            "bo_pd2": expected_commit[0] + "bo yi",
            "bo_pd3": expected_commit[0] + "boyi",
            "after_select_bo": expected_commit[:2] + "yi",
            **{
                f"yi_pd{index}": expected_commit[:2] + "yi"
                for index in range(1, 32)
            },
            "after_select_yi": None,
        }
        compose = []
        for index, label in enumerate(luna_curator.COMPOSITION_LABELS):
            final = label == "after_select_yi"
            if label.startswith("bo_pd") or label.startswith("yi_pd"):
                page_no = int(label.rsplit("pd", 1)[1])
            else:
                page_no = 0
            page_size = 0 if final else 5
            selected_candidates = [
                {"index": local_index, "text": f"candidate-{label}-{local_index}"}
                for local_index in range(page_size)
            ]
            selection_targets = {
                "moboyi_page0": (2, expected_commit[0]),
                "bo_pd3": (4, expected_commit[1]),
                "yi_pd31": (0, expected_commit[2]),
            }
            if label in selection_targets:
                target_index, target_text = selection_targets[label]
                selected_candidates[target_index]["text"] = target_text
            snapshot = {
                "schema_id": "luna_pinyin",
                "scenario": "moboyi_compose",
                "label": label,
                "rime_get_input": "" if final else "moboyi",
                "is_composing": not final,
                "is_ascii_mode": False,
                "is_full_shape": False,
                "is_simplified": False,
                "is_ascii_punct": False,
                "page_no": page_no,
                "page_size": page_size,
                "is_last_page": False,
                "highlighted_candidate_index": 0,
                "selected_candidates": selected_candidates,
                "preedit": expected_preedits[label],
                "commit_text": expected_commit if final else None,
            }
            if index > 0:
                snapshot["processed"] = 1
            compose.append(snapshot)
        return compose

    def run_curator(self, root, pages, compose=None, metadata_mutator=None):
        pages_path = root / "pages.json"
        compose_path = root / "compose.json"
        metadata_path = root / "metadata.json"
        output_path = root / "output.json"
        pages_path.write_bytes((json.dumps(pages, ensure_ascii=False) + "\n").encode("utf-8"))
        if compose is None:
            compose = self.valid_composition()
        compose_path.write_bytes(
            (json.dumps(compose, ensure_ascii=False) + "\n").encode("utf-8")
        )
        parameters = {
            "oracle_root": "target/oracle",
            "output": "fixture.json",
            "expected_rime_dll_sha256": "a" * 64,
            "expected_rime_deployer_sha256": "b" * 64,
            "expected_luna_pinyin_commit": "c" * 40,
            "expected_prelude_commit": "d" * 40,
            "expected_essay_commit": "e" * 40,
            "expected_stroke_commit": "f" * 40,
            "schema_id": "luna_pinyin",
            "modules": ["default"],
            "inputs": list(luna_curator.EXPECTED_INPUTS),
            "page_policy": luna_curator.PAGE_POLICY,
            "runtime_options": dict(luna_curator.RUNTIME_OPTIONS),
            "runtime_options_source": luna_curator.RUNTIME_OPTIONS_SOURCE,
            "additional_runtime_option_patches": [],
            "serialization": dict(luna_curator.CANONICAL_JSON_SERIALIZATION),
        }
        invocation = " ".join(
            [
                "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-m59-luna-composition.ps1",
                "-OracleRoot 'target/oracle'",
                "-Output 'fixture.json'",
                f"-ExpectedRimeDllSha256 '{'a' * 64}'",
                f"-ExpectedRimeDeployerSha256 '{'b' * 64}'",
                f"-ExpectedLunaPinyinCommit '{'c' * 40}'",
                f"-ExpectedPreludeCommit '{'d' * 40}'",
                f"-ExpectedEssayCommit '{'e' * 40}'",
                f"-ExpectedStrokeCommit '{'f' * 40}'",
            ]
        )
        metadata = {
            "rime_dll_sha256": "a" * 64,
            "rime_deployer_sha256": "b" * 64,
            "schema_source_repo": "rime/rime-luna-pinyin",
            "schema_source_commit": "c" * 40,
            "dependency_commits": {
                "rime/rime-prelude": "d" * 40,
                "rime/rime-essay": "e" * 40,
                "rime/rime-stroke": "f" * 40,
            },
            "source_repositories_clean": {
                repo: True for repo in luna_curator.SOURCE_REPOSITORIES
            },
            "source_git_trees": {
                repo: format(index, "x") * 40
                for index, repo in enumerate(
                    sorted(luna_curator.SOURCE_REPOSITORIES), start=1
                )
            },
            "queried_data": {
                "shared_path": "disposable/shared",
                "build_path": "disposable/user/build",
                "shared_tree_sha256": "1" * 64,
                "build_tree_sha256": "2" * 64,
                "tree_hash_algorithm": luna_curator.TREE_HASH_ALGORITHM,
                "mutation_policy": luna_curator.QUERY_MUTATION_POLICY,
                "deployment_policy": luna_curator.DEPLOYMENT_POLICY,
                "timestamp_normalization_policy": luna_curator.TIMESTAMP_NORMALIZATION_POLICY,
                "staged_timestamp_utc": luna_curator.STAGED_TIMESTAMP_UTC,
                "default_custom_sha256": "3" * 64,
                "opencc_tree_sha256": "4" * 64,
            },
            "tool_source": {
                "repository": "yune",
                "commit": "7" * 40,
                "git_tree": "8" * 40,
                "clean": True,
                "dirty": False,
                "status_short": [],
            },
            "tool_hashes": {
                "capture_script_sha256": luna_curator._file_sha256(
                    SCRIPTS / "capture-m59-luna-composition.ps1"
                ),
                "curator_sha256": luna_curator._file_sha256(
                    SCRIPTS / "curate-m59-luna-composition.py"
                ),
                "probe_sha256": luna_curator._file_sha256(
                    SCRIPTS / "oracle-rime-probe.cs"
                ),
            },
            "schema_id": "luna_pinyin",
            "modules": ["default"],
            "inputs": list(luna_curator.EXPECTED_INPUTS),
            "input_count": len(luna_curator.EXPECTED_INPUTS),
            "page_sizes_observed": [5],
            "captured_all_pages": True,
            "page_policy": luna_curator.PAGE_POLICY,
            "runtime_options": dict(luna_curator.RUNTIME_OPTIONS),
            "runtime_options_source": luna_curator.RUNTIME_OPTIONS_SOURCE,
            "additional_runtime_option_patches": [],
            "serialization": dict(luna_curator.CANONICAL_JSON_SERIALIZATION),
            "commands": {
                "deploy": "rime_deployer.exe --build disposable/user disposable/shared "
                "disposable/user/build",
                "capture": invocation,
                "curate": "python scripts/curate-m59-luna-composition.py "
                "'disposable/raw/pages.json' 'disposable/raw/compose.json' "
                "'disposable/raw/metadata.json' 'fixture.json'",
            },
            "actual_invocation": invocation,
            "effective_parameters": parameters,
            "curator_effective_parameters": {
                "pages": "disposable/raw/pages.json",
                "composition": "disposable/raw/compose.json",
                "metadata": "disposable/raw/metadata.json",
                "output": "fixture.json",
            },
            "output_provenance": {
                "path": "fixture.json",
                "existed_before_capture": False,
                "write_policy": luna_curator.WRITE_POLICY,
                "generated_by": "scripts/curate-m59-luna-composition.py",
                "raw_paths": dict(luna_curator.RAW_PATHS),
            },
        }
        if metadata_mutator is not None:
            metadata_mutator(metadata)
        metadata_path.write_bytes((json.dumps(metadata) + "\n").encode("utf-8"))
        with contextlib.redirect_stderr(io.StringIO()):
            luna_curator.main(
                [str(pages_path), str(compose_path), str(metadata_path), str(output_path)]
            )
        return json.loads(output_path.read_text(encoding="utf-8"))

    def test_valid_complete_seven_case_capture_is_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            pages = self.valid_inputs()
            composition = self.valid_composition()
            output = self.run_curator(root, pages, composition)
            output_bytes = (root / "output.json").read_bytes()
        self.assertEqual(len(output["cases"]), 7)
        self.assertEqual(output["cases"], pages)
        self.assertEqual(output["composition_snapshots"], composition)
        self.assertEqual(output["capture"]["inputs"], list(luna_curator.EXPECTED_INPUTS))
        self.assertEqual(output["capture"]["input_count"], 7)
        self.assertEqual(output["capture"]["page_sizes_observed"], [5])
        self.assertTrue(output["capture"]["captured_all_pages"])
        self.assertEqual(
            output["capture"]["runtime_options"],
            dict(luna_curator.RUNTIME_OPTIONS),
        )
        self.assertEqual(
            output["capture"]["runtime_options_source"],
            luna_curator.RUNTIME_OPTIONS_SOURCE,
        )
        self.assertEqual(output["capture"]["additional_runtime_option_patches"], [])
        self.assertEqual(
            output["capture"]["serialization"],
            luna_curator.CANONICAL_JSON_SERIALIZATION,
        )
        self.assertTrue(output["capture"]["tool_source"]["clean"])
        self.assertFalse(output["capture"]["tool_source"]["dirty"])
        self.assertEqual(output["capture"]["tool_source"]["status_short"], [])
        self.assertEqual(
            set(output["capture"]["tool_hashes"]),
            {"capture_script_sha256", "curator_sha256", "probe_sha256"},
        )
        self.assertEqual(
            output["capture"]["commands"]["capture"],
            output["capture"]["actual_invocation"],
        )
        self.assertEqual(
            output["capture"]["curator_effective_parameters"]["output"],
            "fixture.json",
        )
        self.assertFalse(output_bytes.startswith(b"\xef\xbb\xbf"))
        self.assertNotIn(b"\r", output_bytes)
        self.assertNotIn(b"\x00", output_bytes)
        self.assertTrue(output_bytes.endswith(b"\n"))
        self.assertFalse(output_bytes.endswith(b"\n\n"))
        self.assertEqual(
            output["compositions"]["moboyi"]["final_commit"],
            luna_curator.COMPOSITIONS[0][3],
        )
        self.assertEqual(
            output["capture"]["source_row_policy"],
            "m59_lane_b_complete_order_and_partial_selection_composition",
        )
        self.assertEqual(
            output["capture"]["curator_version"], luna_curator.CURATOR_VERSION
        )
        self.assertEqual(
            output["capture"]["order_hash_algorithm"],
            "sha256 of repeated u64be utf8-byte-length followed by utf8 candidate text",
        )
        self.assertEqual(
            output["capture"]["note"],
            "Complete Lane B candidate text/order/position capture plus partial-selection "
            "composition provenance for M59 D-48. PRIMARY case: moboyi -> the non-lexicon "
            "phrase 莫伯洢. Current Yune order divergences remain open until the owning closure "
            "increments land.",
        )
        self.assertRegex(output["inputs"]["moboyi"]["ordered_text_sha256"], r"^[0-9a-f]{64}$")

    def test_ordered_text_hash_has_literal_framed_order_vector(self):
        rows = [{"text": text} for text in ["中", "a", "中", "ab"]]
        self.assertEqual(
            luna_curator._ordered_text_sha256(rows),
            "71864f51172669108316ef6ecd3574f1bae97aa1c258a0210bd23bf7425617f2",
        )
        reversed_rows = list(reversed(rows))
        self.assertEqual(
            luna_curator._ordered_text_sha256(reversed_rows),
            "ac0876c6fcc27043f4d43b27addcbab919021a87519d111e92769c4f50933d65",
        )
        self.assertNotEqual(
            luna_curator._ordered_text_sha256([{"text": "ab"}, {"text": "c"}]),
            luna_curator._ordered_text_sha256([{"text": "a"}, {"text": "bc"}]),
        )

    def test_identical_curation_is_byte_identical(self):
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            first_root = Path(first)
            second_root = Path(second)
            self.run_curator(first_root, self.valid_inputs())
            self.run_curator(second_root, self.valid_inputs())
            self.assertEqual(
                (first_root / "output.json").read_bytes(),
                (second_root / "output.json").read_bytes(),
            )

    def test_raw_json_inputs_must_use_the_declared_canonical_bytes(self):
        invalid_payloads = {
            "bom": b"\xef\xbb\xbf{}\n",
            "crlf": b"{}\r\n",
            "missing_terminal_lf": b"{}",
            "double_terminal_lf": b"{}\n\n",
            "nul": b'{"value":"\\u0000"}\x00\n',
            "invalid_utf8": b'"\xff"\n',
        }
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            valid = root / "valid.json"
            valid.write_bytes(b"{}\n")
            self.assertEqual(luna_curator._load_canonical_json(valid, "valid"), {})
            for label, payload in invalid_payloads.items():
                with self.subTest(label=label):
                    path = root / f"{label}.json"
                    path.write_bytes(payload)
                    with self.assertRaises(ValueError):
                        luna_curator._load_canonical_json(path, label)

    def test_incomplete_or_malformed_capture_fails_closed(self):
        mutations = {
            "missing_case": lambda pages: pages.pop(),
            "wrong_case_order": lambda pages: pages.__setitem__(
                slice(0, 2), [pages[1], pages[0]]
            ),
            "zero_page_size": lambda pages: pages[0].__setitem__("page_size", 0),
            "non_string_candidate": lambda pages: pages[0]["all_candidates"][0].__setitem__(
                "text", 1
            ),
            "missing_target": lambda pages: pages[0]["all_candidates"][0].__setitem__(
                "text", "not-target"
            ),
            "bad_global_index": lambda pages: pages[0]["all_candidates"][0].__setitem__(
                "global_index", 1
            ),
            "boolean_global_index": lambda pages: pages[0]["all_candidates"][0].__setitem__(
                "global_index", False
            ),
            "non_contiguous_page": lambda pages: pages[0]["pages"][0].__setitem__(
                "page_no", 1
            ),
            "boolean_page_number": lambda pages: pages[0]["pages"][0].__setitem__(
                "page_no", False
            ),
            "bad_local_index": lambda pages: pages[0]["pages"][0]["candidates"][0].__setitem__(
                "index", 1
            ),
            "boolean_local_index": lambda pages: pages[0]["pages"][0]["candidates"][0].__setitem__(
                "index", False
            ),
            "bad_last_page": lambda pages: pages[0]["pages"][0].__setitem__(
                "is_last_page", True
            ),
            "short_non_final_page": lambda pages: pages[0]["pages"][0]["candidates"].pop(),
            "oversized_final_page": lambda pages: pages[1]["pages"][0]["candidates"].extend(
                [
                    {"index": index, "text": str(index), "global_index": index}
                    for index in range(1, 6)
                ]
            ),
            "flat_page_disagreement": lambda pages: pages[0]["pages"][0]["candidates"][
                0
            ].__setitem__("text", "different"),
            "input_mismatch": lambda pages: pages[0].__setitem__("rime_get_input", "other"),
            "wrong_schema": lambda pages: pages[0].__setitem__("schema_id", "other"),
            "wrong_termination": lambda pages: pages[0].__setitem__(
                "termination_reason", "page_did_not_advance"
            ),
            "contradictory_pagination_error": lambda pages: pages[0].__setitem__(
                "pagination_error", "page_down_did_not_advance_at_page_0"
            ),
            "bad_initial_page": lambda pages: pages[0].__setitem__("page_no", 1),
            "boolean_initial_page": lambda pages: pages[0].__setitem__("page_no", False),
            "bad_processed_key": lambda pages: pages[0]["processed"].__setitem__(0, False),
            "missing_top_last_page": lambda pages: pages[0].pop("is_last_page"),
            "mismatched_top_last_page": lambda pages: pages[0].__setitem__(
                "is_last_page", True
            ),
            "non_boolean_top_last_page": lambda pages: pages[1].__setitem__(
                "is_last_page", 1
            ),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as temp:
                pages = self.valid_inputs()
                mutate(pages)
                with self.assertRaises(ValueError):
                    self.run_curator(Path(temp), pages)

        composition_mutations = {
            "missing_step": lambda rows: rows.pop(10),
            "reordered_steps": lambda rows: rows.__setitem__(
                slice(2, 4), [rows[3], rows[2]]
            ),
            "duplicate_step": lambda rows: rows.insert(4, dict(rows[3])),
            "unprocessed_step": lambda rows: rows[5].__setitem__("processed", 0),
            "float_processed_step": lambda rows: rows[5].__setitem__("processed", 1.0),
            "wrong_page_number": lambda rows: rows[3].__setitem__("page_no", 999),
            "wrong_page_size": lambda rows: rows[3].__setitem__("page_size", 4),
            "wrong_highlight": lambda rows: rows[3].__setitem__(
                "highlighted_candidate_index", 1
            ),
            "wrong_selection_target": lambda rows: rows[4]["selected_candidates"][
                4
            ].__setitem__("text", "wrong"),
            "wrong_intermediate_preedit": lambda rows: rows[5].__setitem__(
                "preedit", "wrong"
            ),
            "missing_final_commit": lambda rows: rows[-1].__setitem__(
                "commit_text", None
            ),
        }
        for label, mutate in composition_mutations.items():
            with self.subTest(composition=label), tempfile.TemporaryDirectory() as temp:
                compose = self.valid_composition()
                mutate(compose)
                with self.assertRaises(ValueError):
                    self.run_curator(Path(temp), self.valid_inputs(), compose)

    def test_metadata_identities_and_invocation_are_fail_closed(self):
        mutations = {
            "bad_hash": lambda metadata: metadata.__setitem__("rime_dll_sha256", "bad"),
            "wrong_schema_repo": lambda metadata: metadata.__setitem__(
                "schema_source_repo", "example/wrong"
            ),
            "missing_dependency": lambda metadata: metadata["dependency_commits"].pop(
                "rime/rime-stroke"
            ),
            "wrong_commit_shape": lambda metadata: metadata.__setitem__(
                "schema_source_commit", "c" * 39
            ),
            "mismatched_effective_hash": lambda metadata: metadata[
                "effective_parameters"
            ].__setitem__("expected_rime_dll_sha256", "9" * 64),
            "mismatched_invocation": lambda metadata: metadata.__setitem__(
                "actual_invocation", "capture"
            ),
            "dirty_source": lambda metadata: metadata[
                "source_repositories_clean"
            ].__setitem__("rime/rime-essay", False),
            "missing_source_tree": lambda metadata: metadata["source_git_trees"].pop(
                "rime/rime-stroke"
            ),
            "queried_build_bad_hash": lambda metadata: metadata[
                "queried_data"
            ].__setitem__("build_tree_sha256", "bad"),
            "queried_tree_bad_hash": lambda metadata: metadata[
                "queried_data"
            ].__setitem__("shared_tree_sha256", "bad"),
            "guid_shared_path": lambda metadata: metadata["queried_data"].__setitem__(
                "shared_path", "C:/Temp/m59-luna-capture-guid/shared"
            ),
            "unstable_timestamp_policy": lambda metadata: metadata[
                "queried_data"
            ].__setitem__("staged_timestamp_utc", "2026-07-09T22:00:00Z"),
            "dirty_tool_source": lambda metadata: metadata["tool_source"].update(
                {"clean": False, "dirty": True, "status_short": [" M scripts/tool.py"]}
            ),
            "wrong_tool_hash": lambda metadata: metadata["tool_hashes"].__setitem__(
                "probe_sha256", "9" * 64
            ),
            "wrong_input_order": lambda metadata: metadata.__setitem__(
                "inputs", list(reversed(luna_curator.EXPECTED_INPUTS))
            ),
            "wrong_page_size_summary": lambda metadata: metadata.__setitem__(
                "page_sizes_observed", [6]
            ),
            "runtime_option_enabled": lambda metadata: metadata[
                "runtime_options"
            ].__setitem__("ascii_mode", True),
            "wrong_runtime_option_source": lambda metadata: metadata.__setitem__(
                "runtime_options_source", "invented"
            ),
            "runtime_option_patch": lambda metadata: metadata.__setitem__(
                "additional_runtime_option_patches", ["ascii_mode"]
            ),
            "bom_serialization": lambda metadata: metadata["serialization"].__setitem__(
                "bom", True
            ),
            "wrong_capture_command": lambda metadata: metadata["commands"].__setitem__(
                "capture", "capture"
            ),
            "wrong_curator_output": lambda metadata: metadata[
                "curator_effective_parameters"
            ].__setitem__("output", "other.json"),
            "wrong_output_provenance": lambda metadata: metadata[
                "output_provenance"
            ].__setitem__("path", "other.json"),
            "preexisting_output_provenance": lambda metadata: metadata[
                "output_provenance"
            ].__setitem__("existed_before_capture", True),
            "extra_metadata_field": lambda metadata: metadata.__setitem__(
                "undeclared", True
            ),
        }
        for label, mutate in mutations.items():
            with self.subTest(label=label), tempfile.TemporaryDirectory() as temp:
                with self.assertRaises(ValueError):
                    self.run_curator(
                        Path(temp), self.valid_inputs(), metadata_mutator=mutate
                    )

        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            stale_output = root / "output.json"
            stale_temp = root / ".output.json.stale.tmp"
            stale_output.write_text("stale", encoding="utf-8")
            stale_temp.write_text("stale", encoding="utf-8")
            output_snapshot = stale_output.read_bytes()
            temp_snapshot = stale_temp.read_bytes()
            with self.assertRaises(FileExistsError):
                self.run_curator(
                    root,
                    self.valid_inputs(),
                    metadata_mutator=mutations["bad_hash"],
                )
            self.assertEqual(stale_output.read_bytes(), output_snapshot)
            self.assertEqual(stale_temp.read_bytes(), temp_snapshot)


class CaptureContractTests(unittest.TestCase):
    def test_rime_capture_hard_fails_on_input_and_paging_drift(self):
        source = (SCRIPTS / "oracle-rime-probe.cs").read_text(encoding="utf-8")
        self.assertIn("RimeProcessKey did not handle input key", source)
        self.assertIn("rimeInput = CurrentInput(session, ctx);", source)
        self.assertIn('EntryPoint = "rime_get_api"', source)
        self.assertIn("const int GetInputSlot = 69;", source)
        self.assertIn("RimeGetInput mismatch while capturing", source)
        self.assertIn("if (thisPageNo != pageIndex)", source)
        self.assertIn("Page_Down was not handled while capturing", source)
        self.assertIn("bool noMenu = pageIndex == 0 &&", source)
        self.assertIn("ctx.menu.page_size == 0", source)
        self.assertIn("ctx.menu.num_candidates == 0", source)
        self.assertIn("ctx.menu.page_no == 0", source)
        self.assertIn("ctx.menu.is_last_page == 0", source)
        self.assertIn("ctx.menu.candidates == IntPtr.Zero", source)
        self.assertIn(
            'result["candidate_pointer_null"] = firstPageCandidatePointerNull;', source
        )
        self.assertIn('terminationReason = "no_menu";', source)
        self.assertIn('paginationError = "empty_nonterminal_page_at_page_"', source)
        self.assertIn('terminationReason = "empty_nonterminal_page";', source)
        self.assertLess(source.index("if (noMenu)"), source.index("pages.Add(pageRec)"))
        self.assertLess(
            source.index("if (seenPageNos.Contains(thisPageNo))"),
            source.index("if (thisPageNo != pageIndex)"),
        )

    def test_generalized_schema_capture_is_pinned_create_new_and_date_explicit(self):
        source = (SCRIPTS / "capture-upstream-schema.ps1").read_text(
            encoding="utf-8-sig"
        )
        self.assertIn('[ValidatePattern(\'^\\d{4}-\\d{2}-\\d{2}$\')]', source)
        self.assertIn("[string]$CaptureDate", source)
        self.assertIn("[string]$ExpectedRimeDllSha256", source)
        self.assertIn("[string]$ExpectedRimeDeployerSha256", source)
        self.assertIn("[string]$ExpectedSchemaDataCommit", source)
        self.assertIn("[string[]]$ExpectedDependencyCommit", source)
        self.assertIn("[System.IO.FileMode]::CreateNew", source)
        self.assertIn("Output must not already exist", source)
        self.assertIn("Upstream repository must be clean for capture", source)
        self.assertIn("Upstream repository commit mismatch", source)
        self.assertIn("Assert-GitRepositoryStateUnchanged", source)
        self.assertIn("rime.dll SHA-256 mismatch", source)
        self.assertIn("rime_deployer.exe SHA-256 mismatch", source)
        self.assertIn("capture_date: captureDate", source)
        self.assertIn("capture_command: captureCommand", source)
        self.assertIn("-CaptureDate $(Quote-CommandArg $CaptureDate)", source)
        self.assertIn("-Output $(Quote-CommandArg $EvidenceOutput)", source)
        self.assertIn("-ExpectedSchemaDataCommit", source)
        self.assertNotIn("capture_date: '2026-06-21'", source)
        self.assertLess(
            source.index("if (Test-Path -LiteralPath $Output)"),
            source.index("foreach ($Dir in @($Shared, $User))"),
        )

    def test_generalized_schema_capture_versions_whole_input_scenarios(self):
        source = (SCRIPTS / "capture-upstream-schema.ps1").read_text(
            encoding="utf-8-sig"
        )
        self.assertIn('[ValidateSet("m19-component", "m59-whole-input")]', source)
        self.assertIn('$CaptureMode -eq "m19-component"', source)
        self.assertIn('"tone_key_2_after_first_input"', source)
        self.assertIn('"before_tone_key_2"', source)
        self.assertIn('"after_tone_key_2"', source)
        self.assertIn("digits remain real schema key events", source)
        self.assertIn("effective_scenarios: effectiveScenarios", source)
        self.assertIn("whole_input_oracle_rows", source)
        self.assertIn("source_lexicon_absent", source)
        self.assertIn("termsWithCharacters(terms)", source)
        self.assertIn("oracle terms plus Unicode-scalar constituents", source)
        self.assertIn('$DependencyArguments = Quote-CommandArg ($DependencyRepo -join ",")', source)
        self.assertIn('$InputArguments = Quote-CommandArg ($InputSequence -join ",")', source)

    def test_m59_whole_input_fixtures_match_manifest_and_preserve_components(self):
        fixture_root = (
            SCRIPTS.parent / "crates/yune-core/tests/fixtures/upstream-1.17.0"
        )
        manifest = json.loads(
            (fixture_root / "oracle-manifest.json").read_text(encoding="utf-8")
        )
        manifest_by_path = {entry["path"]: entry for entry in manifest["files"]}
        expected = {
            "double-pinyin-m59-whole-input.json": (
                "hknivs",
                "好逆鐘",
                "33f373436769d0be0a719bafb6d0c2367e4295c4ed8f26ecda528adf043bf62d",
            ),
            "bopomofo-m59-whole-input.json": (
                "cl3su3j06",
                "好你玩",
                "3f563a940f5d0437b809307d6162e6e1f8ad63e3faf430e1641480fcce667dff",
            ),
        }
        for fixture_name, (input_text, oracle_top, expected_sha) in expected.items():
            with self.subTest(fixture=fixture_name):
                path = fixture_root / fixture_name
                payload = path.read_bytes()
                self.assertEqual(hashlib.sha256(payload).hexdigest(), expected_sha)
                fixture = json.loads(payload.decode("utf-8"))
                manifest_row = manifest_by_path[fixture_name]
                self.assertEqual(manifest_row["sha256"], expected_sha)
                self.assertEqual(
                    manifest_row["capture_command"],
                    fixture["oracle"]["capture_command"],
                )
                self.assertIn(
                    f"-Output 'crates/yune-core/tests/fixtures/upstream-1.17.0/{fixture_name}'",
                    fixture["oracle"]["capture_command"],
                )
                self.assertEqual(fixture["cases"][0]["input"], input_text)
                self.assertEqual(
                    fixture["cases"][0]["all_candidates"][0]["text"], oracle_top
                )
                proof = fixture["capture"]["whole_input_oracle_rows"][0]
                self.assertEqual(proof["oracle_top"], oracle_top)
                self.assertEqual(proof["source_dictionary_exact_term_count"], 0)
                self.assertEqual(proof["source_vocabulary_exact_term_count"], 0)
                self.assertTrue(proof["source_lexicon_absent"])
                dictionary_terms = {
                    row.split("\t", 1)[0]
                    for row in fixture["capture"]["source_dictionary_rows"]
                }
                self.assertTrue(
                    set(oracle_top).issubset(dictionary_terms),
                    "every oracle-top constituent must retain external source-row provenance",
                )
                self.assertEqual(
                    fixture["capture"]["effective_scenarios"],
                    ["paging_first_input", "commit_first_input_space"],
                )

        component_hashes = {
            "double-pinyin-basic.json": (
                "2f17053131d73028f315229fe7f22df226fc4b67f3b224e19ce99ed2bf864d24"
            ),
            "bopomofo-basic.json": (
                "3288e14306c3fc1cfe53e10f0bb743afa02e514d3bafe652f544e423f1047c70"
            ),
        }
        for fixture_name, expected_sha in component_hashes.items():
            self.assertEqual(
                hashlib.sha256((fixture_root / fixture_name).read_bytes()).hexdigest(),
                expected_sha,
                f"historical component fixture changed: {fixture_name}",
            )

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_generalized_schema_capture_rejects_existing_output_before_mutation(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle_root = root / "missing-oracle"
            output = root / "capture.json"
            sentinel = b"existing-output-must-survive"
            output.write_bytes(sentinel)
            completed = subprocess.run(
                [
                    powershell,
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    str(SCRIPTS / "capture-upstream-schema.ps1"),
                    "-OracleRoot",
                    str(oracle_root),
                    "-SchemaId",
                    "double_pinyin",
                    "-Output",
                    str(output),
                    "-CaptureDate",
                    "2026-07-10",
                ],
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=60,
            )
            combined = completed.stdout + completed.stderr
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("Output must not already exist", combined)
            self.assertNotIn("Missing required upstream oracle input", combined)
            self.assertEqual(output.read_bytes(), sentinel)
            self.assertFalse(oracle_root.exists())

    def test_lane_b_capture_roots_are_unique_and_marker_verified(self):
        luna_bytes = (SCRIPTS / "capture-m59-luna-composition.ps1").read_bytes()
        self.assertTrue(
            all(byte < 0x80 for byte in luna_bytes),
            "Windows PowerShell capture scripts that claim pure-ASCII must not "
            "embed encoding-sensitive literals or comments",
        )
        luna = luna_bytes.decode("ascii")
        yune = (SCRIPTS / "capture-yune-candidate-order.ps1").read_text(
            encoding="utf-8-sig"
        )
        self.assertIn('[guid]::NewGuid().ToString("N")', luna)
        self.assertIn("Refusing to reuse an existing Lane-B capture root", luna)
        self.assertIn("invalid marker", luna)
        self.assertIn("Assert-Git-Clean", luna)
        self.assertIn("SharedTreeSha256BeforeCapture", luna)
        self.assertIn("BuildTreeSha256AfterCapture", luna)
        self.assertIn("raw shared/build hashes must remain identical", luna)
        self.assertIn("clean disposable deploy from pinned tracked source files", luna)
        self.assertIn("Copy-PinnedRimeData", luna)
        self.assertIn("& $RimeDeployer --build $User $Shared $Build", luna)
        self.assertIn("FromUnixTimeSeconds(946684800)", luna)
        self.assertIn("AddMilliseconds(500)", luna)
        self.assertIn("LastWriteTimeUtc.Ticks -ne $StagedTimestampUtc.Ticks", luna)
        self.assertIn('shared_path = "disposable/shared"', luna)
        self.assertIn(
            '$pagingInputs = @("moboyi", "boyi", "yi", "zhonggao", "zhongguo", '
            '"gao", "guo")',
            luna,
        )
        self.assertIn("Lane-B capture did not preserve the declared seven-input order", luna)
        self.assertIn("Get-RimeCaptureRuntimeOptionProvenance", luna)
        self.assertIn(
            '"RimeProbe.CaptureWithIdentity+CaptureScenariosWithIdentity/'
            'CaptureRuntimeOptionPolicy"',
            luna,
        )
        self.assertIn("runtime_options = $RuntimeOptions", luna)
        self.assertIn("runtime_options_source = $RuntimeOptionsSource", luna)
        self.assertIn("additional_runtime_option_patches", luna)
        self.assertIn("status_short = @($ToolState.status_short)", luna)
        self.assertIn("capture_script_sha256 = $CaptureScriptSha256", luna)
        self.assertIn("curator_sha256 = $CuratorSha256", luna)
        self.assertIn("probe_sha256 = $ProbeSha256", luna)
        self.assertIn("Assert-GitStateUnchanged $RepoRoot", luna)
        self.assertIn("Write-Utf8NoBom $PagesRaw", luna)
        self.assertIn("Write-Utf8NoBom $ComposeRaw", luna)
        self.assertIn("Write-Utf8NoBom $MetadataRaw", luna)
        self.assertNotIn("Set-Content -LiteralPath $PagesRaw -Encoding UTF8", luna)
        self.assertIn("Assert-CanonicalJsonFile $Output", luna)
        self.assertIn("actual_invocation = $EffectiveInvocation", luna)
        self.assertIn("effective_parameters = $EffectiveParameters", luna)
        self.assertIn(
            '$DefaultOutput = Join-Path $RepoRoot "target/m59-luna-leading-single-composition.json"',
            luna,
        )
        self.assertIn("function Assert-LaneBOutputPreflight", luna)
        self.assertIn("Output must not already exist", luna)
        self.assertIn("Output must not be inside or equal to OracleRoot", luna)
        self.assertIn("Output must not alias protected input", luna)
        self.assertIn("Output must not be inside protected input", luna)
        self.assertIn("GetFinalPathNameByHandle", luna)
        self.assertIn("existing_fixture = $DefaultFixture", luna)
        self.assertIn(
            "$Output = Assert-LaneBOutputPreflight $Output $OracleRoot "
            "$ProtectedCaptureInputs",
            luna,
        )
        self.assertLess(
            luna.index("$Output = Assert-LaneBOutputPreflight"),
            luna.index("$ToolState = Git-State $RepoRoot"),
        )
        self.assertLess(
            luna.index("$Output = Assert-LaneBOutputPreflight"),
            luna.index("New-Item -ItemType Directory -Path $WorkRoot"),
        )
        self.assertIn("existed_before_capture = $false", luna)
        manifest = json.loads(
            (
                SCRIPTS.parent
                / "crates/yune-core/tests/fixtures/upstream-1.17.0/oracle-manifest.json"
            ).read_text(encoding="utf-8")
        )
        lane_b_manifest = next(
            row
            for row in manifest["files"]
            if row["path"] == "m59-luna-leading-single-composition.json"
        )
        lane_b_fixture = json.loads(
            (
                SCRIPTS.parent
                / "crates/yune-core/tests/fixtures/upstream-1.17.0/"
                "m59-luna-leading-single-composition.json"
            ).read_text(encoding="utf-8")
        )
        self.assertEqual(
            lane_b_manifest["capture_command"],
            lane_b_fixture["capture"]["actual_invocation"],
        )
        self.assertIn("-Output 'target/", lane_b_manifest["capture_command"])
        self.assertNotIn(
            "-Output crates/yune-core/tests/fixtures/",
            lane_b_manifest["capture_command"],
        )
        self.assertIn("never overwrites", lane_b_manifest["import_policy"])
        curator = (SCRIPTS / "curate-m59-luna-composition.py").read_text(
            encoding="utf-8"
        )
        self.assertIn("os.link(temp_path, output)", curator)
        self.assertNotIn("os.replace(temp_path, output)", curator)
        self.assertIn('[guid]::NewGuid().ToString("N")', yune)
        self.assertIn("Capture work root already exists", yune)
        self.assertIn("invalid marker", yune)

    def test_yune_capture_invocation_and_schema_narrowing_provenance_are_truthful(self):
        source = (SCRIPTS / "capture-yune-candidate-order.ps1").read_text(
            encoding="utf-8-sig"
        )
        invocation_block = source.split("$Invocation = @(", 1)[1].split("\n)", 1)[0]
        self.assertNotIn("-Inputs", invocation_block)
        self.assertIn(
            'if ($InputsWereProvided) { $Invocation += "-Inputs $(Quote-CommandArg '
            '($Inputs -join \',\'))" }',
            source,
        )
        self.assertIn(
            'inputs_source = if ($InputsWereProvided) { "explicit" } else { "oracle_cases" }',
            source,
        )
        self.assertIn("$StagedSchemaList = @(Get-TopLevelSchemaList", source)
        self.assertIn("$SchemaListState = Resolve-SchemaListNarrowing", source)
        self.assertLess(
            source.index("$StagedSchemaList = @(Get-TopLevelSchemaList"),
            source.index("$EffectiveParameters[\"schema_list_narrowed\"]"),
        )
        self.assertIn("if ($SchemaListIndexes.Count -ne 1)", source)
        self.assertIn("if ($Entries.Count -eq 0)", source)
        self.assertIn("if (-not $SeenEntries.Add($Matches.schema))", source)
        for value in (
            '"none"',
            '"generated_narrow_schema_list_switch"',
            '"default_yaml_overlay"',
            '"source_default_yaml"',
        ):
            self.assertIn(value, source)
        self.assertIn("schema_list_narrowed = $SchemaListNarrowed", source)
        self.assertIn(
            '$EffectiveParameters["schema_list_narrowed"] = $SchemaListNarrowed',
            source,
        )
        self.assertGreaterEqual(
            source.count(
                "narrow_schema_list_switch_used = $NarrowSchemaListSwitchUsed"
            ),
            2,
        )
        self.assertIn(
            "schema_list_narrowing_source = $SchemaListNarrowingSource", source
        )
        self.assertIn(
            '$EffectiveParameters["schema_list_narrowing_source"] = '
            "$SchemaListNarrowingSource",
            source,
        )

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_lane_b_output_preflight_rejects_existing_alias_and_containment(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle = root / "oracle"
            tools_dir = root / "tools"
            oracle.mkdir()
            tools_dir.mkdir()
            oracle_sentinel = oracle / "source.txt"
            tool_file = tools_dir / "capture.ps1"
            fixture = root / "fixture.json"
            existing_output = root / "existing.json"
            oracle_sentinel.write_bytes(b"oracle-input")
            tool_file.write_bytes(b"tool-input")
            fixture.write_bytes(b"fixture-input")
            existing_output.write_bytes(b"existing-output")
            command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_LANE_B_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @(
    "Get-CanonicalLaneBPath",
    "Test-LaneBPathWithinOrEqual",
    "Assert-LaneBOutputPreflight"
)) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
$protected = [ordered]@{
    tool_directory = $env:YUNE_LANE_B_TOOLS_TEST
    capture_script = $env:YUNE_LANE_B_TOOL_FILE_TEST
    existing_fixture = $env:YUNE_LANE_B_FIXTURE_TEST
}
function Try-Preflight([string]$candidate) {
    try {
        $resolved = Assert-LaneBOutputPreflight `
            $candidate `
            $env:YUNE_LANE_B_ORACLE_TEST `
            $protected
        return [ordered]@{ accepted = $true; resolved = $resolved; error = $null }
    }
    catch {
        return [ordered]@{ accepted = $false; resolved = $null; error = $_.Exception.Message }
    }
}
[ordered]@{
    existing = Try-Preflight $env:YUNE_LANE_B_EXISTING_OUTPUT_TEST
    oracle_child = Try-Preflight $env:YUNE_LANE_B_ORACLE_CHILD_TEST
    tool_child = Try-Preflight $env:YUNE_LANE_B_TOOL_CHILD_TEST
    fixture_alias = Try-Preflight $env:YUNE_LANE_B_FIXTURE_TEST
    safe = Try-Preflight $env:YUNE_LANE_B_SAFE_OUTPUT_TEST
} | ConvertTo-Json -Depth 5 -Compress
"""
            safe_output = root / "fresh" / "lane-b.json"
            oracle_child = oracle / "lane-b.json"
            tool_child = tools_dir / "lane-b.json"
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_LANE_B_CAPTURE_SCRIPT_TEST": str(
                        SCRIPTS / "capture-m59-luna-composition.ps1"
                    ),
                    "YUNE_LANE_B_ORACLE_TEST": str(oracle),
                    "YUNE_LANE_B_TOOLS_TEST": str(tools_dir),
                    "YUNE_LANE_B_TOOL_FILE_TEST": str(tool_file),
                    "YUNE_LANE_B_FIXTURE_TEST": str(fixture),
                    "YUNE_LANE_B_EXISTING_OUTPUT_TEST": str(existing_output),
                    "YUNE_LANE_B_ORACLE_CHILD_TEST": str(oracle_child),
                    "YUNE_LANE_B_TOOL_CHILD_TEST": str(tool_child),
                    "YUNE_LANE_B_SAFE_OUTPUT_TEST": str(safe_output),
                }
            )
            completed = subprocess.run(
                [powershell, "-NoProfile", "-Command", command],
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
                env=environment,
                timeout=60,
            )
            if completed.returncode != 0:
                self.fail(completed.stderr or completed.stdout)
            result = json.loads(completed.stdout.strip().splitlines()[-1])
            self.assertFalse(result["existing"]["accepted"])
            self.assertIn("must not already exist", result["existing"]["error"])
            self.assertFalse(result["oracle_child"]["accepted"])
            self.assertIn("inside or equal to OracleRoot", result["oracle_child"]["error"])
            self.assertFalse(result["tool_child"]["accepted"])
            self.assertIn("inside protected input", result["tool_child"]["error"])
            self.assertFalse(result["fixture_alias"]["accepted"])
            self.assertIn("must not already exist", result["fixture_alias"]["error"])
            self.assertTrue(result["safe"]["accepted"])
            self.assertFalse(safe_output.exists())
            self.assertFalse(oracle_child.exists())
            self.assertFalse(tool_child.exists())
            self.assertEqual(existing_output.read_bytes(), b"existing-output")
            self.assertEqual(oracle_sentinel.read_bytes(), b"oracle-input")
            self.assertEqual(tool_file.read_bytes(), b"tool-input")
            self.assertEqual(fixture.read_bytes(), b"fixture-input")

    def test_capture_runtime_options_are_single_sourced_and_serialized(self):
        probe = (SCRIPTS / "oracle-rime-probe.cs").read_text(encoding="utf-8")
        capture = (SCRIPTS / "capture-yune-candidate-order.ps1").read_text(
            encoding="utf-8-sig"
        )
        policy = probe.split(
            "CaptureRuntimeOptionPolicy = new RuntimeOptionSetting[] {", 1
        )[1].split("};", 1)[0]
        expected_settings = (
            'new RuntimeOptionSetting("ascii_mode", false)',
            'new RuntimeOptionSetting("full_shape", false)',
            'new RuntimeOptionSetting("ascii_punct", false)',
            'new RuntimeOptionSetting("zh_hans", false)',
        )
        positions = [policy.index(setting) for setting in expected_settings]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(policy.count("new RuntimeOptionSetting("), 4)
        self.assertIn(
            "return (RuntimeOptionSetting[])CaptureRuntimeOptionPolicy.Clone();",
            probe,
        )
        capture_with_identity = probe.split(
            "public static List<Dictionary<string, object>> CaptureWithIdentity(", 1
        )[1].split(
            "public static List<Dictionary<string, object>> CaptureScenarios(", 1
        )[0]
        self.assertIn(
            "foreach (var option in CaptureRuntimeOptionPolicy)",
            capture_with_identity,
        )
        self.assertIn(
            "RimeSetOption(session, U8(option.name, ptrs), option.enabled ? 1 : 0);",
            capture_with_identity,
        )
        for option_name in ("ascii_mode", "full_shape", "ascii_punct", "zh_hans"):
            self.assertNotIn(f'U8("{option_name}", ptrs)', capture_with_identity)
        scenario_initialization = probe.split(
            "public static List<Dictionary<string, object>> CaptureScenariosWithIdentity(",
            1,
        )[1].split("foreach (var action in scenario.actions", 1)[0]
        self.assertIn("foreach (var option in CaptureRuntimeOptionPolicy)", scenario_initialization)
        self.assertIn(
            "RimeSetOption(session, U8(option.name, ptrs), option.enabled ? 1 : 0);",
            scenario_initialization,
        )
        for option_name in ("ascii_mode", "full_shape", "ascii_punct", "zh_hans"):
            self.assertNotIn(f'U8("{option_name}", ptrs)', scenario_initialization)
        self.assertIn(
            "foreach ($Option in [RimeProbe]::GetCaptureRuntimeOptions())", capture
        )
        self.assertIn(
            '$EffectiveParameters["runtime_options"] = $RuntimeOptions', capture
        )
        self.assertIn(
            '$EffectiveParameters["runtime_options_source"] = $RuntimeOptionsSource',
            capture,
        )
        self.assertIn("runtime_options = $RuntimeOptions", capture)
        self.assertIn("runtime_options_source = $RuntimeOptionsSource", capture)

    def test_raw_capture_generators_canonicalize_only_final_json_text(self):
        contracts = (
            (
                "capture-upstream-rime-cantonese.ps1",
                "Write-NewUtf8NoBom $Output $EvidenceJson",
                "function Write-NewUtf8NoBom",
            ),
            (
                "capture-yune-candidate-order.ps1",
                "Write-Utf8NoBom $Output $EvidenceJson",
                "function Write-Utf8NoBom",
            ),
        )
        for script_name, final_write, writer_definition in contracts:
            with self.subTest(script=script_name):
                source = (SCRIPTS / script_name).read_text(encoding="utf-8-sig")
                self.assertIn("function ConvertTo-CanonicalJsonText", source)
                self.assertIn(
                    '$Json = $Json.Replace("`r`n", "`n").Replace("`r", "`n")',
                    source,
                )
                self.assertIn('return $Json.TrimEnd([char]10) + "`n"', source)
                self.assertIn("$EvidenceJson = ConvertTo-CanonicalJsonText $Evidence", source)
                self.assertIn(final_write, source)
                self.assertEqual(source.count("ConvertTo-CanonicalJsonText"), 2)
                writer_body = source.split(writer_definition, 1)[1].split("\n}", 1)[0]
                self.assertNotIn("Replace(\"`r`n\"", writer_body)
                if script_name == "capture-yune-candidate-order.ps1":
                    self.assertIn("Write-Utf8NoBom $Marker $MarkerText", source)
                    self.assertIn("Write-Utf8NoBom $DefaultYaml $Narrowed", source)

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_raw_capture_json_writers_emit_deterministic_utf8_lf(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        contracts = (
            ("capture-upstream-rime-cantonese.ps1", "Write-NewUtf8NoBom"),
            ("capture-yune-candidate-order.ps1", "Write-Utf8NoBom"),
            ("capture-m59-luna-composition.ps1", "Write-Utf8NoBom"),
        )
        generated = {}
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            for script_name, writer_name in contracts:
                output_one = root / f"{script_name}.one.json"
                output_two = root / f"{script_name}.two.json"
                command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_JSON_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @("ConvertTo-CanonicalJsonText", $env:YUNE_JSON_WRITER_TEST)) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
$payload = [ordered]@{
    schema_version = 1
    message = "line one`r`nline two"
    runtime_options = [ordered]@{
        ascii_mode = $false
        full_shape = $false
    }
    rows = @("first", "second")
}
$first = ConvertTo-CanonicalJsonText $payload
$second = ConvertTo-CanonicalJsonText $payload
& $env:YUNE_JSON_WRITER_TEST $env:YUNE_JSON_OUTPUT_ONE_TEST $first
& $env:YUNE_JSON_WRITER_TEST $env:YUNE_JSON_OUTPUT_TWO_TEST $second
"""
                environment = os.environ.copy()
                environment.update(
                    {
                        "YUNE_JSON_SCRIPT_TEST": str(SCRIPTS / script_name),
                        "YUNE_JSON_WRITER_TEST": writer_name,
                        "YUNE_JSON_OUTPUT_ONE_TEST": str(output_one),
                        "YUNE_JSON_OUTPUT_TWO_TEST": str(output_two),
                    }
                )
                completed = subprocess.run(
                    [powershell, "-NoProfile", "-Command", command],
                    check=False,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    env=environment,
                    timeout=60,
                )
                if completed.returncode != 0:
                    self.fail(completed.stderr or completed.stdout)
                first_bytes = output_one.read_bytes()
                second_bytes = output_two.read_bytes()
                self.assertEqual(first_bytes, second_bytes)
                self.assertFalse(first_bytes.startswith(b"\xef\xbb\xbf"))
                self.assertNotIn(b"\r", first_bytes)
                self.assertTrue(first_bytes.endswith(b"\n"))
                self.assertFalse(first_bytes.endswith(b"\n\n"))
                parsed = json.loads(first_bytes.decode("utf-8"))
                self.assertEqual(parsed["message"], "line one\r\nline two")
                self.assertEqual(parsed["rows"], ["first", "second"])
                generated[script_name] = first_bytes
        self.assertEqual(
            generated["capture-upstream-rime-cantonese.ps1"],
            generated["capture-yune-candidate-order.ps1"],
        )
        self.assertEqual(
            generated["capture-upstream-rime-cantonese.ps1"],
            generated["capture-m59-luna-composition.ps1"],
        )

    def test_upstream_lane_a_raw_capture_contract_is_explicit_and_safe(self):
        source = (SCRIPTS / "capture-upstream-rime-cantonese.ps1").read_text(
            encoding="utf-8-sig"
        )
        self.assertIn('[string]$EvidenceMilestone = "M58"', source)
        self.assertIn("[switch]$AllowDirty", source)
        defaults = source.split(
            "if ($null -eq $Inputs -or $Inputs.Count -eq 0) {", 1
        )[1].split("}", 1)[0]
        expected_inputs = [
            "be",
            "bei",
            "bein",
            "being",
            "beingo",
            "beix",
            "beixngoxx",
            "ngohaig",
            "ngohaigo",
            "n",
            "nri",
            "mgoi",
            "zijiguk",
        ]
        positions = [defaults.index(f'"{value}"') for value in expected_inputs]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(defaults.count('"'), 2 * len(expected_inputs))

        helper_definition = source.index("function Assert-UpstreamOutputPreflight")
        first_normalization = source.index("$InputsWereProvided =")
        preflight_call = source.index(
            "$CanonicalOutput = Assert-UpstreamOutputPreflight"
        )
        first_delete = source.index("Remove-Item -LiteralPath $Dir -Recurse -Force")
        dirty_guard = source.index(
            'if ($DirtySources.Count -gt 0 -and -not $AllowDirty.IsPresent)'
        )
        self.assertLess(helper_definition, preflight_call)
        self.assertLess(source.index("function Write-NewUtf8NoBom"), first_normalization)
        self.assertLess(preflight_call, first_delete)
        self.assertLess(dirty_guard, first_delete)
        self.assertIn("Output must not already exist", source)
        self.assertIn(
            "Recreated Shared/User roots must be strict descendants of OracleRoot",
            source,
        )
        self.assertIn(
            "Recreated Shared/User roots must resolve to their exact expected "
            "OracleRoot leaf paths",
            source,
        )
        self.assertIn(
            "Recreated Shared/User roots must be distinct and non-nested", source
        )
        self.assertIn("Output must not be inside or equal to OracleRoot", source)
        self.assertIn("Output must not be inside a recreated Shared/User root", source)
        self.assertIn("GetFinalPathNameByHandle", source)
        self.assertIn("[System.IO.FileMode]::CreateNew", source)

        for contract_field in (
            "librime_commit",
            "source_commit",
            "source_clean",
            "source_dirty",
            "source_status_short",
            "inputs_source",
            "page_sizes_observed",
            "captured_all_pages",
            "runtime_options",
            "runtime_options_source",
            "additional_runtime_option_patches",
            "rime_dll_sha256",
            "rime_deployer_sha256",
            "schema_repo_commits",
            "source_repositories_clean",
            "capture_script_sha256",
            "probe_sha256",
            "actual_invocation",
            "effective_parameters",
            "output_provenance",
        ):
            self.assertIn(contract_field, source)
        self.assertIn('$AdditionalRuntimeOptionPatches = @()', source)
        self.assertIn(
            'runtime_option_patches_scope = "legacy alias: no additional overrides '
            'beyond runtime_options"',
            source,
        )
        self.assertIn('return "external/$Role"', source)
        self.assertIn(
            'if ($PSBoundParameters.ContainsKey("EvidenceMilestone"))', source
        )
        self.assertIn("Write-NewUtf8NoBom $Output", source)
        self.assertGreater(
            source.index('Assert-GitStateUnchanged $RepoRoot "yune"'),
            source.index("$Cases = [RimeProbe]::Capture"),
        )
        self.assertLess(
            source.index('Assert-GitStateUnchanged $RepoRoot "yune"'),
            source.index("$Evidence = [ordered]@{"),
        )
        for binary_guard in (
            'Assert-FileSha256Unchanged $PSCommandPath "capture script" '
            "$CaptureScriptSha256",
            'Assert-FileSha256Unchanged $ProbeSource "oracle probe source" '
            "$ProbeSha256",
            'Assert-FileSha256Unchanged $RimeDll "rime.dll" $ActualRimeDllSha256',
            'Assert-FileSha256Unchanged $RimeDeployer "rime_deployer.exe" '
            "$ActualRimeDeployerSha256",
        ):
            self.assertGreater(source.index(binary_guard), source.index("$Cases = [RimeProbe]::Capture"))
            self.assertLess(source.index(binary_guard), source.index("$Evidence = [ordered]@{"))

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_upstream_binary_hash_revalidation_fails_on_replacement(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / "rime.dll"
            binary.write_bytes(b"pinned-binary-before-capture")
            command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @("File-Sha256", "Assert-FileSha256Unchanged")) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
$before = File-Sha256 $env:YUNE_UPSTREAM_BINARY_TEST
Assert-FileSha256Unchanged $env:YUNE_UPSTREAM_BINARY_TEST "rime.dll" $before
[System.IO.File]::WriteAllBytes(
    $env:YUNE_UPSTREAM_BINARY_TEST,
    [System.Text.Encoding]::UTF8.GetBytes("replacement-during-capture")
)
$replacementRejected = $false
try {
    Assert-FileSha256Unchanged $env:YUNE_UPSTREAM_BINARY_TEST "rime.dll" $before
}
catch {
    $replacementRejected = $_.Exception.Message -like "Binary changed during capture:*"
}
[ordered]@{
    unchanged_accepted = $true
    replacement_rejected = $replacementRejected
} | ConvertTo-Json -Compress
"""
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST": str(
                        SCRIPTS / "capture-upstream-rime-cantonese.ps1"
                    ),
                    "YUNE_UPSTREAM_BINARY_TEST": str(binary),
                }
            )
            completed = subprocess.run(
                [powershell, "-NoProfile", "-Command", command],
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
                env=environment,
                timeout=60,
            )
            if completed.returncode != 0:
                self.fail(completed.stderr or completed.stdout)
            result = json.loads(completed.stdout.strip().splitlines()[-1])
            self.assertTrue(result["unchanged_accepted"])
            self.assertTrue(result["replacement_rejected"])

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_upstream_dirty_capture_tool_hash_revalidation_rejects_in_place_change(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            tool = Path(temp) / "capture-tool.ps1"
            tool.write_text("# already-dirty capture tool before capture\n", encoding="utf-8")
            command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @("File-Sha256", "Assert-FileSha256Unchanged")) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
$statusBefore = " M scripts/capture-upstream-rime-cantonese.ps1"
$before = File-Sha256 $env:YUNE_UPSTREAM_DIRTY_TOOL_TEST
[System.IO.File]::WriteAllText(
    $env:YUNE_UPSTREAM_DIRTY_TOOL_TEST,
    "# changed while still dirty`n",
    [System.Text.UTF8Encoding]::new($false)
)
$statusAfter = " M scripts/capture-upstream-rime-cantonese.ps1"
$mutationRejected = $false
try {
    Assert-FileSha256Unchanged `
        $env:YUNE_UPSTREAM_DIRTY_TOOL_TEST `
        "capture script" `
        $before
}
catch {
    $mutationRejected = $_.Exception.Message -like "Binary changed during capture:*"
}
[ordered]@{
    git_status_unchanged = $statusBefore -ceq $statusAfter
    mutation_rejected = $mutationRejected
} | ConvertTo-Json -Compress
"""
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST": str(
                        SCRIPTS / "capture-upstream-rime-cantonese.ps1"
                    ),
                    "YUNE_UPSTREAM_DIRTY_TOOL_TEST": str(tool),
                }
            )
            completed = subprocess.run(
                [powershell, "-NoProfile", "-Command", command],
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
                env=environment,
                timeout=60,
            )
            if completed.returncode != 0:
                self.fail(completed.stderr or completed.stdout)
            result = json.loads(completed.stdout.strip().splitlines()[-1])
            self.assertTrue(result["git_status_unchanged"])
            self.assertTrue(result["mutation_rejected"])

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_upstream_runtime_option_provenance_matches_probe_runtime(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        command = r"""
$ErrorActionPreference = "Stop"
Add-Type -Path $env:YUNE_PROBE_SOURCE_TEST
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
$functionAst = $ast.Find({
    param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -eq "Get-RimeCaptureRuntimeOptionProvenance"
}, $true)
if ($null -eq $functionAst) { throw "missing runtime-option provenance helper" }
Invoke-Expression $functionAst.Extent.Text
$provenance = Get-RimeCaptureRuntimeOptionProvenance
[ordered]@{
    options = $provenance.runtime_options
    order = @($provenance.runtime_options.Keys)
    source = $provenance.runtime_options_source
} | ConvertTo-Json -Depth 4 -Compress
"""
        environment = os.environ.copy()
        environment.update(
            {
                "YUNE_PROBE_SOURCE_TEST": str(SCRIPTS / "oracle-rime-probe.cs"),
                "YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST": str(
                    SCRIPTS / "capture-upstream-rime-cantonese.ps1"
                ),
            }
        )
        completed = subprocess.run(
            [powershell, "-NoProfile", "-Command", command],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            env=environment,
            timeout=60,
        )
        if completed.returncode != 0:
            self.fail(completed.stderr or completed.stdout)
        result = json.loads(completed.stdout.strip().splitlines()[-1])
        expected = ["ascii_mode", "full_shape", "ascii_punct", "zh_hans"]
        self.assertEqual(result["order"], expected)
        self.assertEqual(list(result["options"]), expected)
        self.assertTrue(all(value is False for value in result["options"].values()))
        self.assertEqual(
            result["source"],
            "RimeProbe.CaptureWithIdentity/CaptureRuntimeOptionPolicy",
        )

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_upstream_capture_script_reaches_defined_preflight_before_missing_inputs(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle_root = root / "oracle"
            output = root / "capture.json"
            completed = subprocess.run(
                [
                    powershell,
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    str(SCRIPTS / "capture-upstream-rime-cantonese.ps1"),
                    "-OracleRoot",
                    str(oracle_root),
                    "-Output",
                    str(output),
                    "-EvidenceMilestone",
                    "M59",
                    "-AllowDirty",
                ],
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=60,
            )
            combined = completed.stdout + completed.stderr
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn(
                "Missing required upstream rime-cantonese capture input", combined
            )
            self.assertNotIn("Assert-UpstreamOutputPreflight", combined)
            self.assertFalse(output.exists())
            self.assertFalse((oracle_root / "m58-rime-cantonese-shared").exists())
            self.assertFalse((oracle_root / "m58-rime-cantonese-user").exists())

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_upstream_output_preflight_preserves_recreated_roots(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle_root = root / "oracle"
            shared = oracle_root / "m58-rime-cantonese-shared"
            user = oracle_root / "m58-rime-cantonese-user"
            shared.mkdir(parents=True)
            user.mkdir()
            (shared / "sentinel.bin").write_bytes(b"shared-sentinel")
            (user / "sentinel.bin").write_bytes(b"user-sentinel")
            existing_output = root / "existing.json"
            existing_output.write_bytes(b"output-sentinel")
            junction = root / "shared-junction"
            junction_result = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(junction), str(shared)],
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if junction_result.returncode != 0 or not junction.exists():
                self.skipTest("junction creation is required for canonical-path coverage")

            cases = [
                {"name": "safe", "output": str(root / "safe" / "capture.json")},
                {"name": "existing", "output": str(existing_output)},
                {
                    "name": "inside_oracle_root",
                    "output": str(oracle_root / "capture.json"),
                },
                {"name": "inside_shared", "output": str(shared / "capture.json")},
                {"name": "inside_user", "output": str(user / "capture.json")},
                {
                    "name": "inside_shared_junction",
                    "output": str(junction / "capture.json"),
                },
            ]
            cases_path = root / "cases.json"
            cases_path.write_text(json.dumps(cases), encoding="utf-8")
            snapshots = {
                shared / "sentinel.bin": b"shared-sentinel",
                user / "sentinel.bin": b"user-sentinel",
                existing_output: b"output-sentinel",
            }
            initially_missing = {
                Path(case["output"])
                for case in cases
                if not Path(case["output"]).exists()
            }
            command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @(
    "Get-CanonicalCapturePath",
    "Test-CapturePathWithinOrEqual",
            "Assert-UpstreamOutputPreflight"
)) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
$caseDocument = Get-Content -LiteralPath $env:YUNE_UPSTREAM_PREFLIGHT_CASES_TEST -Raw -Encoding UTF8 | ConvertFrom-Json
$results = @(
    foreach ($case in $caseDocument) {
        try {
            $null = Assert-UpstreamOutputPreflight `
                ([string]$case.output) `
                $env:YUNE_UPSTREAM_ORACLE_ROOT_TEST `
                $env:YUNE_UPSTREAM_SHARED_TEST `
                $env:YUNE_UPSTREAM_USER_TEST
            [pscustomobject]@{ name = $case.name; accepted = $true }
        }
        catch {
            [pscustomobject]@{ name = $case.name; accepted = $false }
        }
    }
)
$results | ConvertTo-Json -Depth 4 -Compress
"""
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST": str(
                        SCRIPTS / "capture-upstream-rime-cantonese.ps1"
                    ),
                    "YUNE_UPSTREAM_PREFLIGHT_CASES_TEST": str(cases_path),
                    "YUNE_UPSTREAM_ORACLE_ROOT_TEST": str(oracle_root),
                    "YUNE_UPSTREAM_SHARED_TEST": str(shared),
                    "YUNE_UPSTREAM_USER_TEST": str(user),
                }
            )
            try:
                completed = subprocess.run(
                    [powershell, "-NoProfile", "-Command", command],
                    check=False,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    env=environment,
                    timeout=60,
                )
                if completed.returncode != 0:
                    self.fail(completed.stderr or completed.stdout)
                results = {
                    row["name"]: row
                    for row in json.loads(completed.stdout.strip().splitlines()[-1])
                }
                self.assertTrue(results["safe"]["accepted"])
                for name in (
                    "existing",
                    "inside_oracle_root",
                    "inside_shared",
                    "inside_user",
                    "inside_shared_junction",
                ):
                    self.assertFalse(results[name]["accepted"])
                for path, snapshot in snapshots.items():
                    self.assertEqual(path.read_bytes(), snapshot)
                for path in initially_missing:
                    self.assertFalse(path.exists(), str(path))
            finally:
                if junction.exists():
                    junction.rmdir()

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_upstream_preflight_rejects_recreated_root_junction_escape(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle_root = root / "oracle"
            oracle_root.mkdir()
            external_victim = root / "external-victim"
            external_victim.mkdir()
            victim_sentinel = external_victim / "do-not-delete.bin"
            victim_sentinel.write_bytes(b"external-victim-sentinel")
            user = oracle_root / "m58-rime-cantonese-user"
            user.mkdir()
            user_sentinel = user / "do-not-delete.bin"
            user_sentinel.write_bytes(b"user-sentinel")
            escaped_shared = oracle_root / "m58-rime-cantonese-shared"
            junction_result = subprocess.run(
                [
                    "cmd",
                    "/c",
                    "mklink",
                    "/J",
                    str(escaped_shared),
                    str(external_victim),
                ],
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if junction_result.returncode != 0 or not escaped_shared.exists():
                self.skipTest("junction creation is required for escape coverage")
            output = root / "capture.json"
            command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @(
    "Get-CanonicalCapturePath",
    "Test-CapturePathWithinOrEqual",
    "Assert-UpstreamOutputPreflight"
)) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
try {
    $null = Assert-UpstreamOutputPreflight `
        $env:YUNE_UPSTREAM_OUTPUT_TEST `
        $env:YUNE_UPSTREAM_ORACLE_ROOT_TEST `
        $env:YUNE_UPSTREAM_SHARED_TEST `
        $env:YUNE_UPSTREAM_USER_TEST
    [ordered]@{ accepted = $true; error = $null } | ConvertTo-Json -Compress
}
catch {
    [ordered]@{ accepted = $false; error = $_.Exception.Message } | ConvertTo-Json -Compress
}
"""
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST": str(
                        SCRIPTS / "capture-upstream-rime-cantonese.ps1"
                    ),
                    "YUNE_UPSTREAM_OUTPUT_TEST": str(output),
                    "YUNE_UPSTREAM_ORACLE_ROOT_TEST": str(oracle_root),
                    "YUNE_UPSTREAM_SHARED_TEST": str(escaped_shared),
                    "YUNE_UPSTREAM_USER_TEST": str(user),
                }
            )
            try:
                completed = subprocess.run(
                    [powershell, "-NoProfile", "-Command", command],
                    check=False,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    env=environment,
                    timeout=60,
                )
                if completed.returncode != 0:
                    self.fail(completed.stderr or completed.stdout)
                result = json.loads(completed.stdout.strip().splitlines()[-1])
                self.assertFalse(result["accepted"])
                self.assertIn("exact expected OracleRoot leaf paths", result["error"])
                self.assertEqual(
                    victim_sentinel.read_bytes(), b"external-victim-sentinel"
                )
                self.assertEqual(user_sentinel.read_bytes(), b"user-sentinel")
                self.assertFalse(output.exists())
            finally:
                if escaped_shared.exists():
                    escaped_shared.rmdir()

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_upstream_preflight_rejects_internal_source_junction(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            oracle_root = root / "oracle"
            protected_source = oracle_root / "schema-src" / "rime-cantonese"
            protected_source.mkdir(parents=True)
            source_sentinel = protected_source / "do-not-delete.yaml"
            source_sentinel.write_bytes(b"protected-schema-source")
            user = oracle_root / "m58-rime-cantonese-user"
            user.mkdir()
            user_sentinel = user / "do-not-delete.bin"
            user_sentinel.write_bytes(b"user-sentinel")
            redirected_shared = oracle_root / "m58-rime-cantonese-shared"
            junction_result = subprocess.run(
                [
                    "cmd",
                    "/c",
                    "mklink",
                    "/J",
                    str(redirected_shared),
                    str(protected_source),
                ],
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if junction_result.returncode != 0 or not redirected_shared.exists():
                self.skipTest("junction creation is required for internal-source coverage")
            output = root / "capture.json"
            command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @(
    "Get-CanonicalCapturePath",
    "Test-CapturePathWithinOrEqual",
    "Assert-UpstreamOutputPreflight"
)) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
try {
    $null = Assert-UpstreamOutputPreflight `
        $env:YUNE_UPSTREAM_OUTPUT_TEST `
        $env:YUNE_UPSTREAM_ORACLE_ROOT_TEST `
        $env:YUNE_UPSTREAM_SHARED_TEST `
        $env:YUNE_UPSTREAM_USER_TEST
    [ordered]@{ accepted = $true; error = $null } | ConvertTo-Json -Compress
}
catch {
    [ordered]@{ accepted = $false; error = $_.Exception.Message } | ConvertTo-Json -Compress
}
"""
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_UPSTREAM_CAPTURE_SCRIPT_TEST": str(
                        SCRIPTS / "capture-upstream-rime-cantonese.ps1"
                    ),
                    "YUNE_UPSTREAM_OUTPUT_TEST": str(output),
                    "YUNE_UPSTREAM_ORACLE_ROOT_TEST": str(oracle_root),
                    "YUNE_UPSTREAM_SHARED_TEST": str(redirected_shared),
                    "YUNE_UPSTREAM_USER_TEST": str(user),
                }
            )
            try:
                completed = subprocess.run(
                    [powershell, "-NoProfile", "-Command", command],
                    check=False,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    env=environment,
                    timeout=60,
                )
                if completed.returncode != 0:
                    self.fail(completed.stderr or completed.stdout)
                result = json.loads(completed.stdout.strip().splitlines()[-1])
                self.assertFalse(result["accepted"])
                self.assertIn("exact expected OracleRoot leaf paths", result["error"])
                self.assertEqual(
                    source_sentinel.read_bytes(), b"protected-schema-source"
                )
                self.assertEqual(user_sentinel.read_bytes(), b"user-sentinel")
                self.assertFalse(output.exists())
            finally:
                if redirected_shared.exists():
                    redirected_shared.rmdir()

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_capture_runtime_option_provenance_matches_probe_policy_runtime(self):
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        command = r"""
$ErrorActionPreference = "Stop"
Add-Type -Path $env:YUNE_PROBE_SOURCE_TEST
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
$functionAst = $ast.Find({
    param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -eq "Get-RimeCaptureRuntimeOptionProvenance"
}, $true)
if ($null -eq $functionAst) { throw "missing runtime-option provenance helper" }
Invoke-Expression $functionAst.Extent.Text
$probePolicy = @(
    foreach ($option in [RimeProbe]::GetCaptureRuntimeOptions()) {
        [ordered]@{ name = $option.name; enabled = $option.enabled }
    }
)
$provenance = Get-RimeCaptureRuntimeOptionProvenance
[ordered]@{
    probe_policy = $probePolicy
    probe_source = [RimeProbe]::CaptureRuntimeOptionsSource
    shared_probe_source = [RimeProbe]::SharedCaptureRuntimeOptionsSource
    serialized_options = $provenance.runtime_options
    serialized_order = @($provenance.runtime_options.Keys)
    serialized_source = $provenance.runtime_options_source
} | ConvertTo-Json -Depth 5 -Compress
"""
        for script_name in (
            "capture-yune-candidate-order.ps1",
            "capture-m59-luna-composition.ps1",
        ):
            with self.subTest(script=script_name):
                environment = os.environ.copy()
                environment.update(
                    {
                        "YUNE_PROBE_SOURCE_TEST": str(SCRIPTS / "oracle-rime-probe.cs"),
                        "YUNE_CAPTURE_SCRIPT_TEST": str(SCRIPTS / script_name),
                    }
                )
                completed = subprocess.run(
                    [powershell, "-NoProfile", "-Command", command],
                    check=False,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    env=environment,
                    timeout=60,
                )
                if completed.returncode != 0:
                    self.fail(completed.stderr or completed.stdout)
                result = json.loads(completed.stdout.strip().splitlines()[-1])
                expected_order = [
                    "ascii_mode",
                    "full_shape",
                    "ascii_punct",
                    "zh_hans",
                ]
                self.assertEqual(
                    [entry["name"] for entry in result["probe_policy"]], expected_order
                )
                self.assertTrue(
                    all(entry["enabled"] is False for entry in result["probe_policy"])
                )
                self.assertEqual(result["serialized_order"], expected_order)
                self.assertEqual(list(result["serialized_options"]), expected_order)
                self.assertTrue(
                    all(
                        value is False
                        for value in result["serialized_options"].values()
                    )
                )
                expected_probe_source = (
                    "RimeProbe.CaptureWithIdentity/CaptureRuntimeOptionPolicy"
                )
                expected_shared_source = (
                    "RimeProbe.CaptureWithIdentity+CaptureScenariosWithIdentity/"
                    "CaptureRuntimeOptionPolicy"
                )
                self.assertEqual(result["probe_source"], expected_probe_source)
                self.assertEqual(result["shared_probe_source"], expected_shared_source)
                expected_source = (
                    expected_shared_source
                    if script_name == "capture-m59-luna-composition.ps1"
                    else expected_probe_source
                )
                self.assertEqual(result["serialized_source"], expected_source)

    def test_yune_capture_path_preflight_precedes_workspace_mutation(self):
        source = (SCRIPTS / "capture-yune-candidate-order.ps1").read_text(
            encoding="utf-8-sig"
        )
        preflight = source.index("$PathPreflight = Assert-CapturePathPreflight")
        mutation = source.index(
            "New-Item -ItemType Directory -Force -Path $WorkRoot, $User, $Build, $Bin"
        )
        self.assertLess(preflight, mutation)
        self.assertIn("GetFinalPathNameByHandle", source)
        self.assertIn("Alternate data stream paths are not allowed", source)
        self.assertIn("Output must not already exist", source)
        self.assertIn("Output must not be inside SharedDataDir", source)
        self.assertIn("WorkRoot must not be inside SharedDataDir", source)
        self.assertIn("WorkRoot must not be inside or equal to Output", source)
        for protected_name in (
            "YuneDll",
            "OracleCapture",
            "DefaultYamlOverlay",
            "ProbeSource",
            "CaptureScript",
        ):
            self.assertIn(protected_name, source)
        self.assertIn("$OutputUnderWorkRoot -and -not $KeepWorkRoot", source)

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_yune_capture_schema_list_provenance_helpers_runtime(self):
        script_path = SCRIPTS / "capture-yune-candidate-order.ps1"
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            fixtures = {
                "narrow": "schema_list:\n  - schema: jyut6ping3\nmenu:\n  page_size: 5\n",
                "wide": (
                    "schema_list:\n  - schema: jyut6ping3\n"
                    "  - schema: luna_pinyin\nmenu:\n  page_size: 5\n"
                ),
                "malformed": "schema_list:\n  - {schema: jyut6ping3}\n",
                "duplicate": (
                    "schema_list:\n  - schema: jyut6ping3\n"
                    "  - schema: jyut6ping3\n"
                ),
            }
            fixture_paths = {}
            for name, content in fixtures.items():
                path = root / f"{name}.yaml"
                path.write_text(content, encoding="utf-8", newline="\n")
                fixture_paths[name] = str(path)

            command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @("Get-TopLevelSchemaList", "Resolve-SchemaListNarrowing")) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
$narrow = @(Get-TopLevelSchemaList $env:YUNE_NARROW_DEFAULT_TEST)
$wide = @(Get-TopLevelSchemaList $env:YUNE_WIDE_DEFAULT_TEST)
$source = Resolve-SchemaListNarrowing $narrow "jyut6ping3" $false $false
$overlay = Resolve-SchemaListNarrowing $narrow "jyut6ping3" $false $true
$generated = Resolve-SchemaListNarrowing $narrow "jyut6ping3" $true $false
$wideSource = Resolve-SchemaListNarrowing $wide "jyut6ping3" $false $false
$wideOverlay = Resolve-SchemaListNarrowing $wide "jyut6ping3" $false $true
function Is-Rejected([string]$Path) {
    try {
        $null = @(Get-TopLevelSchemaList $Path)
        return $false
    }
    catch {
        return $true
    }
}
[ordered]@{
    source = $source
    overlay = $overlay
    generated = $generated
    wide_source = $wideSource
    wide_overlay = $wideOverlay
    malformed_rejected = Is-Rejected $env:YUNE_MALFORMED_DEFAULT_TEST
    duplicate_rejected = Is-Rejected $env:YUNE_DUPLICATE_DEFAULT_TEST
} | ConvertTo-Json -Depth 5 -Compress
"""
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_CAPTURE_SCRIPT_TEST": str(script_path),
                    "YUNE_NARROW_DEFAULT_TEST": fixture_paths["narrow"],
                    "YUNE_WIDE_DEFAULT_TEST": fixture_paths["wide"],
                    "YUNE_MALFORMED_DEFAULT_TEST": fixture_paths["malformed"],
                    "YUNE_DUPLICATE_DEFAULT_TEST": fixture_paths["duplicate"],
                }
            )
            completed = subprocess.run(
                [powershell, "-NoProfile", "-Command", command],
                check=False,
                capture_output=True,
                text=True,
                encoding="utf-8",
                env=environment,
                timeout=30,
            )
            if completed.returncode != 0:
                self.fail(completed.stderr or completed.stdout)
            result = json.loads(completed.stdout.strip().splitlines()[-1])

        expected_narrowing = {
            "source": "source_default_yaml",
            "overlay": "default_yaml_overlay",
            "generated": "generated_narrow_schema_list_switch",
        }
        for key, expected_source in expected_narrowing.items():
            self.assertTrue(result[key]["schema_list_narrowed"])
            self.assertEqual(
                result[key]["schema_list_narrowing_source"], expected_source
            )
            self.assertEqual(
                result[key]["narrow_schema_list_switch_used"], key == "generated"
            )
        for key in ("wide_source", "wide_overlay"):
            self.assertFalse(result[key]["schema_list_narrowed"])
            self.assertFalse(result[key]["narrow_schema_list_switch_used"])
            self.assertEqual(result[key]["schema_list_narrowing_source"], "none")
        self.assertTrue(result["malformed_rejected"])
        self.assertTrue(result["duplicate_rejected"])

    @unittest.skipUnless(shutil.which("powershell"), "Windows PowerShell is required")
    def test_yune_capture_path_preflight_preserves_inputs_and_creates_nothing(self):
        script_path = SCRIPTS / "capture-yune-candidate-order.ps1"
        powershell = shutil.which("powershell")
        self.assertIsNotNone(powershell)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            shared = root / "shared"
            shared.mkdir()
            protected = {
                "yune_dll": root / "yune.dll",
                "oracle_capture": root / "oracle.json",
                "default_yaml_overlay": root / "overlay.yaml",
                "probe_source": root / "probe.cs",
                "capture_script": root / "capture.ps1",
            }
            for index, path in enumerate(protected.values(), start=1):
                path.write_bytes(f"protected-sentinel-{index}".encode())
            (shared / "default.yaml").write_bytes(b"shared-default-sentinel")
            nested_source = shared / "nested" / "source.bin"
            nested_source.parent.mkdir()
            nested_source.write_bytes(b"shared-nested-sentinel")
            existing_output = root / "existing-output.json"
            existing_output.write_bytes(b"existing-output-sentinel")

            junction = root / "shared-junction"
            junction_result = subprocess.run(
                ["cmd", "/c", "mklink", "/J", str(junction), str(shared)],
                check=False,
                capture_output=True,
                text=True,
                timeout=30,
            )
            if junction_result.returncode != 0 or not junction.exists():
                self.skipTest("junction creation is required for canonical-path coverage")

            common = {
                **{key: str(value) for key, value in protected.items()},
                "shared_data_dir": str(shared),
                "keep_work_root": False,
            }
            cases = [
                {
                    **common,
                    "name": "safe",
                    "output": str(root / "results" / "safe.json"),
                    "work_root": str(root / "work" / "safe"),
                },
                {
                    **common,
                    "name": "existing_output",
                    "output": str(existing_output),
                    "work_root": str(root / "work" / "existing-output"),
                },
                {
                    **common,
                    "name": "output_alias_yune_dll_default_stream",
                    "output": str(protected["yune_dll"]) + "::$DATA",
                    "work_root": str(root / "work" / "output-ads"),
                },
            ]
            for protected_name, protected_path in protected.items():
                cases.append(
                    {
                        **common,
                        "name": f"output_alias_{protected_name}",
                        "output": str(protected_path),
                        "work_root": str(root / "work" / f"alias-{protected_name}"),
                    }
                )
            cases.extend(
                [
                    {
                        **common,
                        "name": "output_under_shared",
                        "output": str(shared / "new-output.json"),
                        "work_root": str(root / "work" / "output-under-shared"),
                    },
                    {
                        **common,
                        "name": "output_under_shared_junction",
                        "output": str(junction / "new-output.json"),
                        "work_root": str(root / "work" / "output-under-junction"),
                    },
                    {
                        **common,
                        "name": "workroot_under_shared",
                        "output": str(root / "results" / "workroot-under-shared.json"),
                        "work_root": str(shared / "new-work"),
                    },
                    {
                        **common,
                        "name": "workroot_under_shared_junction",
                        "output": str(root / "results" / "workroot-under-junction.json"),
                        "work_root": str(junction / "new-work"),
                    },
                    {
                        **common,
                        "name": "output_under_disposable_workroot",
                        "output": str(root / "disposable" / "blocked" / "result.json"),
                        "work_root": str(root / "disposable" / "blocked"),
                    },
                    {
                        **common,
                        "name": "output_under_kept_workroot",
                        "output": str(root / "disposable" / "kept" / "result.json"),
                        "work_root": str(root / "disposable" / "kept"),
                        "keep_work_root": True,
                    },
                    {
                        **common,
                        "name": "workroot_under_future_output",
                        "output": str(root / "future-output" / "blocked"),
                        "work_root": str(
                            root / "future-output" / "blocked" / "nested-work"
                        ),
                    },
                    {
                        **common,
                        "name": "workroot_under_future_output_kept",
                        "output": str(root / "future-output" / "kept"),
                        "work_root": str(
                            root / "future-output" / "kept" / "nested-work"
                        ),
                        "keep_work_root": True,
                    },
                ]
            )
            cases_path = root / "preflight-cases.json"
            cases_path.write_text(json.dumps(cases), encoding="utf-8")
            protected_snapshots = {
                path: path.read_bytes() for path in protected.values()
            }
            protected_snapshots[existing_output] = existing_output.read_bytes()

            def shared_snapshot():
                return {
                    path.relative_to(shared).as_posix(): path.read_bytes()
                    for path in sorted(shared.rglob("*"))
                    if path.is_file()
                }

            shared_before = shared_snapshot()
            initially_missing_outputs = {
                Path(case["output"])
                for case in cases
                if not Path(case["output"]).exists()
            }
            initially_missing_workroots = {
                Path(case["work_root"])
                for case in cases
                if not Path(case["work_root"]).exists()
            }

            command = r"""
$ErrorActionPreference = "Stop"
$tokens = $null
$errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $env:YUNE_CAPTURE_SCRIPT_TEST,
    [ref]$tokens,
    [ref]$errors
)
if ($errors.Count -ne 0) { throw "capture script parse failed" }
foreach ($name in @(
    "Get-CanonicalWindowsPath",
    "Test-CanonicalPathWithinOrEqual",
    "Assert-CapturePathPreflight"
)) {
    $functionAst = $ast.Find({
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq $name
    }, $true)
    if ($null -eq $functionAst) { throw "missing helper $name" }
    Invoke-Expression $functionAst.Extent.Text
}
$caseDocument = Get-Content -LiteralPath $env:YUNE_PREFLIGHT_CASES_TEST -Raw -Encoding UTF8 | ConvertFrom-Json
$results = @(
    foreach ($case in $caseDocument) {
        try {
            $null = Assert-CapturePathPreflight `
                ([string]$case.output) `
                ([string]$case.work_root) `
                ([string]$case.shared_data_dir) `
                ([string]$case.yune_dll) `
                ([string]$case.oracle_capture) `
                ([string]$case.default_yaml_overlay) `
                ([string]$case.probe_source) `
                ([string]$case.capture_script) `
                ([bool]$case.keep_work_root)
            [pscustomobject]@{ name = $case.name; accepted = $true; error = $null }
        }
        catch {
            [pscustomobject]@{ name = $case.name; accepted = $false; error = $_.Exception.Message }
        }
    }
)
$results | ConvertTo-Json -Depth 5 -Compress
"""
            environment = os.environ.copy()
            environment.update(
                {
                    "YUNE_CAPTURE_SCRIPT_TEST": str(script_path),
                    "YUNE_PREFLIGHT_CASES_TEST": str(cases_path),
                }
            )
            try:
                completed = subprocess.run(
                    [powershell, "-NoProfile", "-Command", command],
                    check=False,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    env=environment,
                    timeout=60,
                )
                if completed.returncode != 0:
                    self.fail(completed.stderr or completed.stdout)
                results = {
                    row["name"]: row
                    for row in json.loads(completed.stdout.strip().splitlines()[-1])
                }
                self.assertTrue(results["safe"]["accepted"])
                self.assertTrue(results["output_under_kept_workroot"]["accepted"])
                for case in cases:
                    if case["name"] not in {"safe", "output_under_kept_workroot"}:
                        self.assertFalse(results[case["name"]]["accepted"], case["name"])
                for path, snapshot in protected_snapshots.items():
                    self.assertEqual(path.read_bytes(), snapshot)
                self.assertEqual(shared_snapshot(), shared_before)
                for path in initially_missing_outputs | initially_missing_workroots:
                    self.assertFalse(path.exists(), str(path))
            finally:
                if junction.exists():
                    junction.rmdir()


class NativeRatchetTests(unittest.TestCase):
    provenance = {
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
        "native_benchmark_receipt_sha256": "4" * 64,
        "native_benchmark_executable_prebuilt": "False",
        "native_benchmark_build_performed": "True",
        "benchmark_script_sha256": "2" * 64,
        "track_a_inputs": "x",
        "track_b_inputs": "trackb",
        "iterations": "9",
        "session_iterations": "60",
        "key_iterations": "80",
        "deploy_product_before_benchmark": "True",
        "skip_track_b": "False",
    }

    def setUp(self):
        test_parent = SCRIPTS.parent / "target"
        test_parent.mkdir(parents=True, exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(
            prefix="m59-native-ratchet-test-", dir=test_parent
        )
        self.root = Path(self.temp.name)
        self.thresholds = self.root / "thresholds.csv"
        self.output = self.root / "gate-verdict.csv"
        self.sidecar = self.root / "gate-verdict.provenance.json"
        self.write_thresholds()

    def tearDown(self):
        self.temp.cleanup()

    def write_thresholds(self, *, include_absolute=False, duplicate=False):
        rows = [
            {
                "kind": "latency_ratio",
                "workload": "key_sequence_process_with_context",
                "input": "x",
                "metric": "yune_librime_median_ratio",
                "ceiling": "1.5",
                "unit": "x",
            },
            {
                "kind": "latency_absolute_us",
                "workload": "track-b-product/key_sequence_process_with_context",
                "input": "trackb",
                "metric": "median_us",
                "ceiling": "100",
                "unit": "us",
            },
        ]
        if include_absolute:
            rows.append(
                {
                    "kind": "latency_absolute_us",
                    "workload": "session_create_select_destroy",
                    "input": "",
                    "metric": "median_us",
                    "ceiling": "100",
                    "unit": "us",
                }
            )
        if duplicate:
            rows.append(dict(rows[0]))
        with self.thresholds.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=list(rows[0]), lineterminator="\n")
            writer.writeheader()
            writer.writerows(rows)

    def write_run(
        self,
        number,
        observed,
        *,
        provenance_override=None,
        receipt_override=None,
        include_ratio_check=True,
        checked_observed=None,
    ):
        run = self.root / f"run-{number}"
        run.mkdir()
        environment = dict(self.provenance)
        builder_run = str(number).endswith("1")
        environment["native_benchmark_executable_prebuilt"] = (
            "False" if builder_run else "True"
        )
        environment["native_benchmark_build_performed"] = (
            "True" if builder_run else "False"
        )
        if provenance_override:
            environment.update(provenance_override)
        receipt_fields = {
            "format_version": "1",
            "source_commit": environment["source_commit"],
            "source_tree": environment["source_tree"],
            "source_clean": environment["source_clean"],
            "source_content_binding_sha256": environment[
                "source_content_binding_sha256"
            ],
            "benchmark_script_sha256": environment["benchmark_script_sha256"],
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
            "native_benchmark_executable_sha256": environment[
                "native_benchmark_executable_sha256"
            ],
        }
        if receipt_override:
            receipt_fields.update(receipt_override)
        receipt_text = "".join(
            f"{key}={value}\n" for key, value in receipt_fields.items()
        )
        receipt_path = run / "native-benchmark-build-receipt.txt"
        receipt_path.write_text(receipt_text, encoding="utf-8")
        if (
            not provenance_override
            or "native_benchmark_receipt_sha256" not in provenance_override
        ):
            environment["native_benchmark_receipt_sha256"] = hashlib.sha256(
                receipt_path.read_bytes()
            ).hexdigest()
        (run / "run-status.txt").write_text(
            "status=complete\ndate_utc=2026-01-01T00:00:00Z\ndetail=\n",
            encoding="utf-8",
        )
        (run / "environment.txt").write_text(
            "".join(f"{key}={value}\n" for key, value in environment.items()),
            encoding="utf-8",
        )
        with (run / "summary-comparison.csv").open(
            "w", encoding="utf-8", newline=""
        ) as handle:
            fields = ["track", "workload", "input", "yune_librime_median_ratio"]
            writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
            writer.writeheader()
            writer.writerow(
                {
                    "track": "track-a-comparison",
                    "workload": "key_sequence_process_with_context",
                    "input": "x",
                    "yune_librime_median_ratio": str(observed),
                }
            )
        with (run / "threshold-check.csv").open(
            "w", encoding="utf-8", newline=""
        ) as handle:
            fields = [
                "kind",
                "workload",
                "input",
                "metric",
                "observed",
                "ceiling",
                "unit",
                "status",
            ]
            writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
            writer.writeheader()
            if include_ratio_check:
                checked = observed if checked_observed is None else checked_observed
                writer.writerow(
                    {
                        "kind": "latency_ratio",
                        "workload": "key_sequence_process_with_context",
                        "input": "x",
                        "metric": "yune_librime_median_ratio",
                        "observed": str(checked),
                        "ceiling": "1.5",
                        "unit": "x",
                        "status": "pass" if checked <= 1.5 else "fail",
                    }
                )
            writer.writerow(
                {
                    "kind": "latency_absolute_us",
                    "workload": "track-b-product/key_sequence_process_with_context",
                    "input": "trackb",
                    "metric": "median_us",
                    "observed": "50",
                    "ceiling": "100",
                    "unit": "us",
                    "status": "pass",
                }
            )
        return run

    def run_tool(
        self,
        runs,
        expected=5,
        *,
        thresholds=None,
        output=None,
        return_stderr=False,
    ):
        thresholds = self.thresholds if thresholds is None else thresholds
        output = self.output if output is None else output
        argv = [
            "--thresholds",
            str(thresholds),
            "--expected-runs",
            str(expected),
            "--output",
            str(output),
        ]
        for run in runs:
            argv.extend(["--run", str(run)])
        stderr = io.StringIO()
        with contextlib.redirect_stderr(stderr):
            result = native_ratchet.main(argv)
        return (result, stderr.getvalue()) if return_stderr else result

    def test_five_run_median_passes_while_preserving_an_outlier(self):
        runs = [
            self.write_run(index, value, include_ratio_check=False)
            for index, value in enumerate([1, 1, 2, 1, 1], start=1)
        ]
        self.assertEqual(self.run_tool(runs), 0)
        with self.output.open(encoding="utf-8") as handle:
            row = next(csv.DictReader(handle))
        self.assertEqual(row["median_observed"], "1")
        self.assertEqual(row["worst_observed"], "2")
        self.assertEqual(row["individual_failures"], "1")
        self.assertEqual(row["verdict"], "pass")
        raw_provenance = self.sidecar.read_text(encoding="utf-8")
        provenance = json.loads(raw_provenance)
        self.assertEqual(provenance["required_run_count"], 5)
        self.assertEqual(provenance["tool_version"], native_ratchet.TOOL_VERSION)
        self.assertEqual(
            provenance["tool_path"], "scripts/aggregate-native-ratchet.py"
        )
        self.assertIn(
            "python scripts/aggregate-native-ratchet.py",
            provenance["effective_invocation"],
        )
        self.assertNotIn(
            str(SCRIPTS.parent.resolve()).lower(), raw_provenance.lower()
        )
        self.assertEqual(
            provenance["thresholds"]["path"],
            native_ratchet._recorded_path(self.thresholds),
        )
        self.assertEqual(
            provenance["runs"][0]["path"],
            native_ratchet._recorded_path(runs[0]),
        )
        self.assertEqual(
            provenance["gate_verdict"]["path"],
            native_ratchet._recorded_path(self.output),
        )
        for option in ("--thresholds", "--run", "--output"):
            option_index = provenance["effective_argv"].index(option)
            self.assertFalse(
                Path(provenance["effective_argv"][option_index + 1]).is_absolute()
            )
        self.assertEqual(
            provenance["thresholds"]["sha256"],
            native_ratchet._file_sha256(self.thresholds),
        )
        self.assertEqual(len(provenance["runs"]), 5)
        self.assertEqual(
            provenance["runs"][0]["raw_files_sha256"]["summary-comparison.csv"],
            native_ratchet._file_sha256(runs[0] / "summary-comparison.csv"),
        )
        self.assertEqual(
            provenance["validated_provenance"]["measured_yune_dll_sha256"],
            "b" * 64,
        )
        self.assertEqual(
            provenance["validated_provenance"]["native_benchmark_mode_sequence"],
            "build,reuse,reuse,reuse,reuse",
        )
        self.assertEqual(
            provenance["validated_provenance"]["native_benchmark_builder_run"],
            "1",
        )
        self.assertEqual(
            [
                (
                    row["native_benchmark_executable_prebuilt"],
                    row["native_benchmark_build_performed"],
                )
                for row in provenance["runs"]
            ],
            [("False", "True")] + [("True", "False")] * 4,
        )
        self.assertEqual(
            {
                row["native_benchmark_executable_sha256"]
                for row in provenance["runs"]
            },
            {"1" * 64},
        )
        self.assertEqual(
            {
                row["native_benchmark_receipt_sha256"]
                for row in provenance["runs"]
            },
            {provenance["runs"][0]["native_benchmark_receipt_sha256"]},
        )
        self.assertIn("--thresholds", provenance["effective_invocation"])

    def test_effective_argv_normalizes_absolute_in_repo_path_values(self):
        threshold = (
            SCRIPTS.parent
            / "docs/reports/evidence/m55-native-match-or-beat/thresholds/"
            "m55-thresholds.csv"
        ).resolve()
        run = (
            SCRIPTS.parent
            / "docs/reports/evidence/m59-closeout-baseline/"
            "m59-i0-fixed-45775182-r1"
        ).resolve()
        output = (SCRIPTS.parent / "target/portable-gate-verdict.csv").resolve()
        if os.name == "nt":
            threshold = Path(str(threshold).swapcase())
            run = Path(str(run).swapcase())
            output = Path(str(output).swapcase())
        recorded = native_ratchet._recorded_effective_args(
            [
                "--thresholds",
                str(threshold),
                "--expected-runs",
                "5",
                f"--run={run}",
                "--output",
                str(output),
            ]
        )
        serialized = json.dumps(recorded).lower()
        self.assertNotIn(str(SCRIPTS.parent.resolve()).lower(), serialized)
        self.assertEqual(
            recorded[recorded.index("--thresholds") + 1],
            "docs/reports/evidence/m55-native-match-or-beat/thresholds/"
            "m55-thresholds.csv",
        )
        self.assertIn(
            "--run=docs/reports/evidence/m59-closeout-baseline/"
            "m59-i0-fixed-45775182-r1",
            recorded,
        )
        self.assertEqual(
            recorded[recorded.index("--output") + 1],
            "target/portable-gate-verdict.csv",
        )

    def test_abbreviated_path_options_are_rejected_before_output(self):
        runs = [self.write_run(index, 1) for index in range(1, 6)]
        command = [
            sys.executable,
            "-B",
            str(SCRIPTS / "aggregate-native-ratchet.py"),
            "--thresh",
            str(self.thresholds),
            "--expected-runs",
            "5",
        ]
        for run in runs:
            command.extend(["--r", str(run)])
        command.extend(["--out", str(self.output)])
        result = subprocess.run(command, check=False, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("the following arguments are required", result.stderr)
        self.assertIn("--thresholds", result.stderr)
        self.assertIn("--run", result.stderr)
        self.assertIn("--output", result.stderr)
        self.assertFalse(self.output.exists())
        self.assertFalse(self.sidecar.exists())

    def test_median_failure_writes_gate_and_returns_nonzero(self):
        runs = [
            self.write_run(index, value)
            for index, value in enumerate([2, 2, 1, 2, 1], start=1)
        ]
        self.assertEqual(self.run_tool(runs), 1)
        with self.output.open(encoding="utf-8") as handle:
            row = next(csv.DictReader(handle))
        self.assertEqual(row["median_observed"], "2")
        self.assertEqual(row["verdict"], "fail")

    def test_one_four_and_six_runs_are_structural_failures(self):
        all_runs = [self.write_run(index, 1) for index in range(1, 7)]
        for count in (1, 4, 6):
            with self.subTest(count=count):
                self.assertEqual(self.run_tool(all_runs[:count]), 2)
        self.assertEqual(self.run_tool(all_runs[:4], expected=4), 2)

    def test_duplicate_path_and_structural_failure_invalidate_output_pair(self):
        runs = [self.write_run(index, 1) for index in range(1, 6)]
        self.output.write_text("stale", encoding="utf-8")
        self.sidecar.write_text("stale", encoding="utf-8")
        stale_gate_temp = self.root / f".{self.output.name}.stale.tmp"
        stale_sidecar_temp = self.root / f".{self.sidecar.name}.stale.tmp"
        stale_gate_temp.write_text("stale", encoding="utf-8")
        stale_sidecar_temp.write_text("stale", encoding="utf-8")
        self.assertEqual(
            self.run_tool([runs[0], runs[0], runs[1], runs[2], runs[4]]), 2
        )
        self.assertFalse(self.output.exists())
        self.assertFalse(self.sidecar.exists())
        self.assertFalse(stale_gate_temp.exists())
        self.assertFalse(stale_sidecar_temp.exists())

        self.output.mkdir()
        self.sidecar.write_text("stale sibling", encoding="utf-8")
        result, stderr = self.run_tool(
            [runs[0], runs[0], runs[1], runs[2], runs[4]], return_stderr=True
        )
        self.assertEqual(result, 2)
        self.assertTrue(self.output.is_dir())
        self.assertFalse(self.sidecar.exists())
        self.assertIn("paired-output invalidation failed", stderr)

    def test_output_collisions_fail_before_mutation_for_every_input_path(self):
        runs = [self.write_run(index, 1) for index in range(1, 6)]
        existing_inputs = [self.thresholds]
        absent_input_slots = []
        for run in runs:
            for name in native_ratchet.RUN_FILES:
                path = run / name
                if path.exists():
                    existing_inputs.append(path)
                else:
                    absent_input_slots.append(path)
        snapshots = {path: path.read_bytes() for path in existing_inputs}

        cases = [
            ("thresholds", self.thresholds),
            (
                "thresholds-relative-alias",
                Path(os.path.relpath(self.thresholds, Path.cwd())),
            ),
            ("run-directory", runs[0]),
            ("inside-run-directory", runs[0] / "new-gate-verdict.csv"),
        ]
        cases.extend(
            (f"run-input-{name}", runs[0] / name)
            for name in native_ratchet.RUN_FILES
        )
        if os.name == "nt":
            cases.append(
                ("thresholds-case-alias", Path(str(self.thresholds).swapcase()))
            )

        for label, output in cases:
            with self.subTest(label=label):
                sidecar = native_ratchet._sidecar_path(output)
                sidecar_before = (
                    sidecar.read_bytes() if sidecar.is_file() else None
                )
                result, stderr = self.run_tool(
                    runs, output=output, return_stderr=True
                )
                self.assertEqual(result, 2)
                self.assertTrue(
                    "collides with protected" in stderr
                    or "must not be inside protected run" in stderr,
                    stderr,
                )
                for path, snapshot in snapshots.items():
                    self.assertEqual(path.read_bytes(), snapshot, str(path))
                for path in absent_input_slots:
                    self.assertFalse(path.exists(), str(path))
                if sidecar_before is None:
                    self.assertFalse(sidecar.exists(), str(sidecar))
                else:
                    self.assertEqual(sidecar.read_bytes(), sidecar_before)

    def test_sidecar_collision_with_thresholds_fails_before_mutation(self):
        runs = [self.write_run(index, 1) for index in range(1, 6)]
        sidecar_thresholds = self.sidecar
        sidecar_thresholds.write_bytes(self.thresholds.read_bytes())
        threshold_snapshot = sidecar_thresholds.read_bytes()

        result, stderr = self.run_tool(
            runs,
            thresholds=sidecar_thresholds,
            output=self.output,
            return_stderr=True,
        )

        self.assertEqual(result, 2)
        self.assertIn("provenance sidecar output", stderr)
        self.assertIn("collides with protected thresholds", stderr)
        self.assertEqual(sidecar_thresholds.read_bytes(), threshold_snapshot)
        self.assertFalse(self.output.exists())

    def test_hard_exit_cannot_publish_gate_without_provenance_sidecar(self):
        runs = [self.write_run(index, 1) for index in range(1, 6)]
        code = f"""
import os
import runpy
from pathlib import Path

namespace = runpy.run_path({str(SCRIPTS / 'aggregate-native-ratchet.py')!r})
real_replace = os.replace
replace_count = 0

def replace_then_crash(source, destination):
    global replace_count
    real_replace(source, destination)
    replace_count += 1
    if replace_count == 1:
        os._exit(77)

namespace['os'].replace = replace_then_crash
namespace['_write_output_pair'](
    Path({str(self.output)!r}),
    Path({str(self.sidecar)!r}),
    'verdict\\n',
    {{'schema_version': 1}},
    thresholds_path=Path({str(self.thresholds)!r}),
    run_paths=[Path(value) for value in {[str(run) for run in runs]!r}],
)
"""
        result = subprocess.run(
            [sys.executable, "-B", "-c", code],
            cwd=SCRIPTS.parent,
            check=False,
        )
        self.assertEqual(result.returncode, 77)
        self.assertFalse(
            self.output.exists(),
            "gate verdict is the commit marker and must publish last",
        )
        self.assertTrue(self.sidecar.is_file())
        self.assertEqual(
            json.loads(self.sidecar.read_text(encoding="utf-8")),
            {"schema_version": 1},
        )

    def test_output_destinations_are_revalidated_at_publication_boundary(self):
        runs = [self.write_run(index, 1) for index in range(1, 6)]
        events = []
        original_validate = native_ratchet._validate_output_destinations
        original_remove = native_ratchet._remove_final_outputs

        def validate(*args, **kwargs):
            events.append("validate")
            return original_validate(*args, **kwargs)

        def remove(*args, **kwargs):
            events.append("remove")
            return original_remove(*args, **kwargs)

        native_ratchet._validate_output_destinations = validate
        native_ratchet._remove_final_outputs = remove
        try:
            native_ratchet._write_output_pair(
                self.output,
                self.sidecar,
                "verdict\n",
                {"schema_version": 1},
                thresholds_path=self.thresholds,
                run_paths=runs,
            )
        finally:
            native_ratchet._validate_output_destinations = original_validate
            native_ratchet._remove_final_outputs = original_remove

        self.assertEqual(events, ["validate", "remove"])
        self.assertTrue(self.sidecar.is_file())
        self.assertTrue(self.output.is_file())

    def test_recorded_input_sets_are_bound_to_observed_threshold_rows(self):
        bad_cases = [
            ("track-a-mismatch", {"track_a_inputs": "other"}),
            ("track-a-duplicate", {"track_a_inputs": "x,x"}),
            ("track-a-empty", {"track_a_inputs": "x,"}),
            ("track-b-mismatch", {"track_b_inputs": "other"}),
            ("track-b-duplicate", {"track_b_inputs": "trackb,trackb"}),
        ]
        for case_number, (label, override) in enumerate(bad_cases, start=1):
            with self.subTest(label=label):
                runs = [
                    self.write_run(
                        case_number * 10 + index,
                        1,
                        provenance_override=override,
                    )
                    for index in range(1, 6)
                ]
                result, stderr = self.run_tool(runs, return_stderr=True)
                self.assertEqual(result, 2)
                self.assertIn("recorded track_", stderr)
                self.assertFalse(self.output.exists())
                self.assertFalse(self.sidecar.exists())

    def test_provenance_mismatch_is_structural_failure(self):
        runs = [self.write_run(index, 1) for index in range(1, 5)]
        runs.append(
            self.write_run(5, 1, provenance_override={"iterations": "999"})
        )
        result, stderr = self.run_tool(runs, return_stderr=True)
        self.assertEqual(result, 2)
        self.assertIn("run 5 provenance mismatch for iterations", stderr)
        self.assertFalse(self.output.exists())
        self.assertFalse(self.sidecar.exists())

    def test_native_benchmark_build_reuse_sequence_is_exact(self):
        bad_cases = (
            (
                "run-1-reused",
                1,
                {
                    "native_benchmark_executable_prebuilt": "True",
                    "native_benchmark_build_performed": "False",
                },
            ),
            (
                "run-2-rebuilt",
                2,
                {
                    "native_benchmark_executable_prebuilt": "False",
                    "native_benchmark_build_performed": "True",
                },
            ),
        )
        for case_number, (label, bad_index, override) in enumerate(bad_cases, start=1):
            with self.subTest(label=label):
                runs = []
                for index in range(1, 6):
                    runs.append(
                        self.write_run(
                            case_number * 10 + index,
                            1,
                            provenance_override=override if index == bad_index else None,
                        )
                    )
                result, stderr = self.run_tool(runs, return_stderr=True)
                self.assertEqual(result, 2)
                self.assertIn("run 1 built once", stderr)
                self.assertFalse(self.output.exists())
                self.assertFalse(self.sidecar.exists())

    def test_native_benchmark_receipt_must_match_packet_and_environment(self):
        bad_cases = (
            (
                "packet-hash",
                {"native_benchmark_receipt_sha256": "9" * 64},
                None,
                "receipt SHA does not match packet bytes",
            ),
            (
                "source-binding",
                None,
                {"source_content_binding_sha256": "9" * 64},
                "receipt mismatch for source_content_binding_sha256",
            ),
            (
                "executable-path-binding",
                None,
                {
                    "native_benchmark_executable_path": (
                        "/external/native-benchmark-target/other-benchmark"
                    )
                },
                "receipt mismatch for native_benchmark_executable_path",
            ),
        )
        for case_number, (
            label,
            provenance_override,
            receipt_override,
            message,
        ) in enumerate(bad_cases, start=1):
            with self.subTest(label=label):
                runs = [
                    self.write_run(case_number * 10 + index, 1)
                    for index in range(1, 5)
                ]
                runs.append(
                    self.write_run(
                        case_number * 10 + 5,
                        1,
                        provenance_override=provenance_override,
                        receipt_override=receipt_override,
                    )
                )
                result, stderr = self.run_tool(runs, return_stderr=True)
                self.assertEqual(result, 2)
                self.assertIn(message, stderr)
                self.assertFalse(self.output.exists())
                self.assertFalse(self.sidecar.exists())

    def test_native_benchmark_receipt_hash_must_match_across_all_runs(self):
        runs = [self.write_run(index, 1) for index in range(1, 5)]
        runs.append(
            self.write_run(
                5,
                1,
                receipt_override={"cargo_command": "cargo bench --no-run --frozen"},
            )
        )
        result, stderr = self.run_tool(runs, return_stderr=True)
        self.assertEqual(result, 2)
        self.assertIn(
            "run 5 provenance mismatch for native_benchmark_receipt_sha256",
            stderr,
        )
        self.assertFalse(self.output.exists())
        self.assertFalse(self.sidecar.exists())

    def test_incomplete_or_failed_run_status_is_structural_failure(self):
        for case_number, status in enumerate(("in-progress", "failed"), start=1):
            with self.subTest(status=status):
                runs = [
                    self.write_run(case_number * 10 + index, 1)
                    for index in range(1, 6)
                ]
                (runs[-1] / "run-status.txt").write_text(
                    f"status={status}\ndate_utc=2026-01-01T00:00:00Z\ndetail=boom\n",
                    encoding="utf-8",
                )
                result, stderr = self.run_tool(runs, return_stderr=True)
                self.assertEqual(result, 2)
                self.assertIn("benchmark run status must be complete", stderr)
                self.assertFalse(self.output.exists())
                self.assertFalse(self.sidecar.exists())

    def test_summary_and_threshold_check_disagreement_is_structural(self):
        runs = [self.write_run(index, 1) for index in range(1, 5)]
        runs.append(self.write_run(5, 1, checked_observed=1.1))
        self.assertEqual(self.run_tool(runs), 2)

    def test_hash_shape_git_head_and_required_benchmark_mode_are_enforced(self):
        bad_cases = [
            {"measured_yune_dll_sha256": "not-a-hash"},
            {"yune_git_head": "f" * 40},
            {"source_clean": "False"},
            {"allow_dirty": "True"},
            {"deploy_product_before_benchmark": "False"},
            {"skip_track_b": "True"},
        ]
        for case_number, override in enumerate(bad_cases, start=1):
            with self.subTest(override=override):
                runs = [
                    self.write_run(case_number * 10 + index, 1)
                    for index in range(1, 5)
                ]
                runs.append(
                    self.write_run(
                        case_number * 10 + 5,
                        1,
                        provenance_override=override,
                    )
                )
                self.assertEqual(self.run_tool(runs), 2)

    def test_missing_non_ratio_row_and_duplicate_threshold_fail(self):
        self.write_thresholds(include_absolute=True)
        runs = [self.write_run(index, 1) for index in range(1, 6)]
        self.assertEqual(self.run_tool(runs), 2)

        self.write_thresholds(duplicate=True)
        self.assertEqual(self.run_tool(runs), 2)

    def test_short_csv_row_is_evidence_error_and_invalidates_pair(self):
        runs = [self.write_run(index, 1) for index in range(1, 6)]
        malformed = runs[0] / "summary-comparison.csv"
        malformed.write_text(
            "track,workload,input,yune_librime_median_ratio\n"
            "track-a-comparison,key_sequence_process_with_context\n",
            encoding="utf-8",
        )
        thresholds = native_ratchet.read_thresholds(self.thresholds)
        with self.assertRaises(native_ratchet.EvidenceError):
            native_ratchet.read_run(runs[0], thresholds)
        self.output.write_text("stale", encoding="utf-8")
        self.sidecar.write_text("stale", encoding="utf-8")
        result, stderr = self.run_tool(runs, return_stderr=True)
        self.assertEqual(result, 2)
        self.assertIn("missing 'input' field", stderr)
        self.assertFalse(self.output.exists())
        self.assertFalse(self.sidecar.exists())


class ResidualClassifierTests(unittest.TestCase):
    @staticmethod
    def row(text, comment):
        return residual_classifier.Candidate(text=text, comment=comment)

    def case(self, input_text, rows):
        return residual_classifier.CandidateCase(
            input=input_text,
            rows=tuple(rows),
            page_size=5,
            captured_all_pages=True,
            menu_present=True,
            termination_reason="last_page",
        )

    def owned_documents(self):
        row = self.row
        oracle = {
            "being": self.case(
                "being",
                [
                    row("頭", "tau4"),
                    row("秘", "bei3"),
                    row("祕", "bei3"),
                    row("甲", "gaap3"),
                    row("乙", "jyut3"),
                ],
            ),
            "beingo": self.case(
                "beingo",
                [
                    row("首", "sau2"),
                    row("秘", "bei3"),
                    row("祕", "bei3"),
                    row("丙", "bing2"),
                    row("丁", "ding1"),
                ],
            ),
            "beixngoxx": self.case("beixngoxx", [row("準", "zeon2")]),
            "mgoi": self.case("mgoi", [row("該", "goi1")]),
            "zijiguk": self.case(
                "zijiguk",
                [
                    row("先", "sin1"),
                    row("只", "zi2"),
                    row("衹", "zi2"),
                    row("戊", "mou6"),
                    row("己", "gei2"),
                ],
            ),
        }
        actual = {
            "being": self.case(
                "being",
                [
                    row("頭", "tau4"),
                    row("秘", "bei3"),
                    row("乙", "jyut3"),
                    row("甲", "gaap3"),
                ],
            ),
            "beingo": self.case(
                "beingo",
                [
                    row("首", "sau2"),
                    row("秘", "bei3"),
                    row("丁", "ding1"),
                    row("丙", "bing2"),
                ],
            ),
            "beixngoxx": self.case("beixngoxx", [row("準", "zeon2")]),
            "mgoi": self.case("mgoi", [row("該", "goi1")]),
            "zijiguk": self.case(
                "zijiguk",
                [
                    row("先", "sin1"),
                    row("只", "zi2"),
                    row("己", "gei2"),
                    row("戊", "mou6"),
                    row("衹", "zi2"),
                ],
            ),
        }
        return oracle, actual

    def weights(self, overrides=None):
        values = {
            ("甲", "gaap3"): 10.0,
            ("乙", "jyut3"): 10.0,
            ("丙", "bing2"): 20.0,
            ("丁", "ding1"): 20.0,
            ("戊", "mou6"): 30.0,
            ("己", "gei2"): 30.0,
        }
        values.update(overrides or {})
        rows = {
            key: (
                residual_classifier.SourceWeightRow(
                    text=key[0],
                    code=key[1],
                    weight=weight,
                    table="test",
                    line=index,
                    raw_weight=str(weight),
                ),
            )
            for index, (key, weight) in enumerate(values.items(), 1)
        }
        return residual_classifier.SourceWeights(rows=rows, table_provenance=())

    @staticmethod
    def mappings():
        return {
            ("祕", "bei3"): residual_classifier.OpenCcMapping(
                key="祕",
                outputs=("秘", "祕"),
                code="bei3",
                line=36,
                locations="秘@chars:2;祕@chars:1",
            ),
            ("只", "zi2"): residual_classifier.OpenCcMapping(
                key="只",
                outputs=("只", "衹"),
                code="zi2",
                line=4,
                locations="只@chars:3;衹@chars:4",
            ),
        }

    def test_declared_opencc_and_equal_weight_residuals_classify_without_accepting_d48(self):
        oracle, actual = self.owned_documents()
        result = residual_classifier.classify_documents(
            oracle,
            actual,
            {"all_accepted": False, "policy": "exact"},
            self.mappings(),
            self.weights(),
        )
        self.assertTrue(result["classification_complete"])
        self.assertEqual(result["classification_status"], "complete")
        self.assertFalse(result["raw_comparator_all_accepted"])
        self.assertFalse(result["scope"]["full_d48_acceptance_claimed"])
        self.assertEqual(result["summary"]["raw_strict_passes"], 2)
        self.assertEqual(result["summary"]["raw_strict_failures"], 3)
        self.assertEqual(
            result["summary"]["total_inversions_after_opencc_normalization"], 3
        )
        self.assertEqual(result["summary"]["cross_weight_inversions"], 0)
        self.assertFalse(
            result["summary"]["beyond_oracle_depth_disposition_used"]
        )

    def test_cross_weight_residual_emits_semantic_failure(self):
        oracle, actual = self.owned_documents()
        result = residual_classifier.classify_documents(
            oracle,
            actual,
            {"all_accepted": False, "policy": "exact"},
            self.mappings(),
            self.weights({("乙", "jyut3"): 9.0}),
        )
        self.assertFalse(result["classification_complete"])
        self.assertEqual(result["classification_status"], "incomplete")
        being = next(row for row in result["cases"] if row["input"] == "being")
        self.assertEqual(being["cross_weight_inversion_count"], 1)
        self.assertEqual(being["classification_reasons"], ["cross-weight-inversion"])

    def test_unexpected_exact_case_difference_is_unowned_not_normalized(self):
        oracle, actual = self.owned_documents()
        actual["mgoi"] = self.case("mgoi", [self.row("別", "bit6")])
        result = residual_classifier.classify_documents(
            oracle,
            actual,
            {"all_accepted": False, "policy": "exact"},
            self.mappings(),
            self.weights(),
        )
        mgoi = next(row for row in result["cases"] if row["input"] == "mgoi")
        self.assertEqual(mgoi["classification_verdict"], "fail")
        self.assertIn("must remain strict exact", mgoi["classification_reasons"][0])

    @staticmethod
    def capture_document(duplicates=False):
        cases = []
        for input_text in residual_classifier.CLASS1_INPUTS:
            candidates = [{"text": input_text, "comment": "x1"}]
            if duplicates and input_text == "being":
                candidates.append({"text": input_text, "comment": "x2"})
            cases.append(
                {
                    "input": input_text,
                    "page_size": 5,
                    "menu_present": True,
                    "termination_reason": "last_page",
                    "captured_all_pages": True,
                    "all_candidates": candidates,
                }
            )
        return {"cases": cases}

    def test_duplicate_candidate_text_fails_closed_as_weight_ambiguous(self):
        with self.assertRaisesRegex(
            residual_classifier.EvidenceError, "occurrence weights would be ambiguous"
        ):
            residual_classifier.parse_capture(
                self.capture_document(duplicates=True), "oracle"
            )

    def test_null_capture_comment_is_normalized_to_empty_string(self):
        document = self.capture_document()
        document["cases"][0]["all_candidates"][0]["comment"] = None
        parsed = residual_classifier.parse_capture(document, "oracle")
        self.assertEqual(parsed["being"].rows[0].comment, "")

    def test_source_table_accepts_encoder_phrase_without_explicit_code(self):
        with tempfile.TemporaryDirectory() as temp:
            table = Path(temp) / "phrase.dict.yaml"
            table.write_text(
                "---\nname: phrase\nsort: by_weight\n...\n\n詞\n有碼\tcode\n",
                encoding="utf-8",
            )
            rows = residual_classifier._parse_table_rows(
                table, "phrase", {"詞": 9.0, "有碼": 8.0}
            )
            self.assertEqual(
                [(row.text, row.code, row.weight) for row in rows],
                [("詞", "", 9.0), ("有碼", "code", 8.0)],
            )

    def test_dictionary_manifest_requires_by_weight_policy(self):
        imports = "\n".join(
            f"  - {name}" for name in residual_classifier.EXPECTED_IMPORT_TABLES
        )
        manifest = (
            "---\nname: jyut6ping3\nsort: original\n"
            "vocabulary: essay-cantonese\nimport_tables:\n"
            f"{imports}\n...\n"
        )
        with self.assertRaisesRegex(
            residual_classifier.EvidenceError, "sort policy must be by_weight"
        ):
            residual_classifier.parse_dictionary_manifest(manifest)

    def test_strict_comparator_must_stay_red_and_exception_free(self):
        document = self.capture_document()
        oracle = residual_classifier.parse_capture(document, "oracle")
        actual = residual_classifier.parse_capture(document, "actual")
        cases = []
        for input_text in residual_classifier.CLASS1_INPUTS:
            cases.append(
                {
                    "input": input_text,
                    "oracle_count": 1,
                    "actual_count": 1,
                    "raw_first_mismatch_index": None,
                    "missing_count": 0,
                    "extra_count": 0,
                    "accepted_exceptions": [],
                    "verdict": "pass",
                }
            )
        comparator = {
            "tool": "compare-candidate-order.py",
            "policy": "exact",
            "inputs": list(residual_classifier.CLASS1_INPUTS),
            "all_accepted": False,
            "provenance": {
                "oracle": {"sha256": "a" * 64},
                "actual": {"sha256": "b" * 64},
                "exceptions": {"sha256": "c" * 64},
            },
            "cases": cases,
        }
        with self.assertRaisesRegex(
            residual_classifier.EvidenceError, "must not apply an exception policy"
        ):
            residual_classifier.validate_strict_comparator(
                comparator, oracle, actual, "a" * 64, "b" * 64
            )
        comparator["provenance"]["exceptions"] = None
        with self.assertRaisesRegex(
            residual_classifier.EvidenceError,
            "all_accepted disagrees with recomputed case verdicts",
        ):
            residual_classifier.validate_strict_comparator(
                comparator, oracle, actual, "a" * 64, "b" * 64
            )

    def test_atomic_json_is_deterministic_utf8_and_single_lf(self):
        result = {"verdict": "pass", "text": "祕衹"}
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            first_path = Path(first) / "result.json"
            second_path = Path(second) / "result.json"
            residual_classifier.write_json_atomic(first_path, result)
            residual_classifier.write_json_atomic(second_path, result)
            first_bytes = first_path.read_bytes()
            self.assertEqual(first_bytes, second_path.read_bytes())
            self.assertFalse(first_bytes.startswith(b"\xef\xbb\xbf"))
            self.assertNotIn(b"\r", first_bytes)
            self.assertNotIn(b"\x00", first_bytes)
            self.assertTrue(first_bytes.endswith(b"\n"))
            self.assertFalse(first_bytes.endswith(b"\n\n"))

    def test_atomic_json_replaces_existing_output(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "result.json"
            output.write_text("stale", encoding="utf-8")
            residual_classifier.write_json_atomic(output, {"verdict": "pass"})
            self.assertEqual(
                json.loads(output.read_text(encoding="utf-8")),
                {"verdict": "pass"},
            )

    def required_cli_args(self, root, output):
        paths = {}
        for name in (
            "oracle",
            "actual",
            "strict",
            "inventory",
            "manifest",
            "vocabulary",
            "opencc",
        ):
            path = root / f"{name}.txt"
            path.write_text("{}", encoding="utf-8")
            paths[name] = path
        source = root / "source"
        source.mkdir()
        return [
            "--oracle",
            str(paths["oracle"]),
            "--expected-oracle-sha256",
            "0" * 64,
            "--actual",
            str(paths["actual"]),
            "--expected-actual-sha256",
            "0" * 64,
            "--strict-comparator",
            str(paths["strict"]),
            "--expected-strict-comparator-sha256",
            "0" * 64,
            "--opencc-inventory",
            str(paths["inventory"]),
            "--expected-opencc-inventory-sha256",
            "0" * 64,
            "--source-repository",
            str(source),
            "--expected-dictionary-commit",
            "1" * 40,
            "--expected-dictionary-tree",
            "2" * 40,
            "--dictionary-manifest",
            str(paths["manifest"]),
            "--expected-dictionary-manifest-sha256",
            "0" * 64,
            "--vocabulary",
            str(paths["vocabulary"]),
            "--expected-vocabulary-sha256",
            "0" * 64,
            "--opencc-source",
            str(paths["opencc"]),
            "--expected-opencc-source-sha256",
            "0" * 64,
            "--output",
            str(output),
        ]

    def test_cli_hash_mismatch_invalidates_stale_output(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            output = root / "result.json"
            output.write_text("stale", encoding="utf-8")
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                result = residual_classifier.main(self.required_cli_args(root, output))
            self.assertEqual(result, 2)
            self.assertIn("SHA-256 mismatch", stderr.getvalue())
            self.assertFalse(output.exists())

    def test_cli_output_alias_is_rejected_without_mutating_input(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            args = self.required_cli_args(root, root / "result.json")
            oracle = Path(args[args.index("--oracle") + 1])
            original = oracle.read_bytes()
            args[args.index("--output") + 1] = str(oracle)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                result = residual_classifier.main(args)
            self.assertEqual(result, 2)
            self.assertIn("must not alias --oracle", stderr.getvalue())
            self.assertEqual(oracle.read_bytes(), original)

    def test_output_must_not_alias_classifier_tool(self):
        with self.assertRaisesRegex(
            residual_classifier.EvidenceError, "must not alias --tool"
        ):
            residual_classifier._preflight_paths(
                [("--tool", Path(residual_classifier.__file__))],
                Path(residual_classifier.__file__),
            )


if __name__ == "__main__":
    unittest.main()
