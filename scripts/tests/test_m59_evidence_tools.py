from __future__ import annotations

import csv
import contextlib
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
        self.assertIn(
            "-Output target/m59-luna-leading-single-composition.json",
            lane_b_manifest["capture_command"],
        )
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
        "yune_git_head": "a" * 40,
        "measured_yune_dll_sha256": "b" * 64,
        "upstream_rime_dll_sha256": "c" * 64,
        "product_schema_tree_sha256": "d" * 64,
        "track_a_inputs": "x",
        "track_b_inputs": "trackb",
        "iterations": "9",
        "session_iterations": "60",
        "key_iterations": "80",
        "deploy_product_before_benchmark": "True",
        "skip_track_b": "False",
    }

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
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
            }
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
        include_ratio_check=True,
        checked_observed=None,
    ):
        run = self.root / f"run-{number}"
        run.mkdir()
        environment = dict(self.provenance)
        if provenance_override:
            environment.update(provenance_override)
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
        return run

    def run_tool(self, runs, expected=5, *, return_stderr=False):
        argv = [
            "--thresholds",
            str(self.thresholds),
            "--expected-runs",
            str(expected),
            "--output",
            str(self.output),
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
        provenance = json.loads(self.sidecar.read_text(encoding="utf-8"))
        self.assertEqual(provenance["required_run_count"], 5)
        self.assertEqual(provenance["tool_version"], native_ratchet.TOOL_VERSION)
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
        self.assertIn("--thresholds", provenance["effective_invocation"])

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

    def test_provenance_mismatch_is_structural_failure(self):
        runs = [self.write_run(index, 1) for index in range(1, 5)]
        runs.append(
            self.write_run(5, 1, provenance_override={"source_commit": "e" * 40})
        )
        self.assertEqual(self.run_tool(runs), 2)

    def test_summary_and_threshold_check_disagreement_is_structural(self):
        runs = [self.write_run(index, 1) for index in range(1, 5)]
        runs.append(self.write_run(5, 1, checked_observed=1.1))
        self.assertEqual(self.run_tool(runs), 2)

    def test_hash_shape_git_head_and_required_benchmark_mode_are_enforced(self):
        bad_cases = [
            {"measured_yune_dll_sha256": "not-a-hash"},
            {"yune_git_head": "f" * 40},
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


if __name__ == "__main__":
    unittest.main()
