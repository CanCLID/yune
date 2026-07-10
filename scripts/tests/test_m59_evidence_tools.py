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
                    "input": input_text,
                    "rime_get_input": input_text,
                    "processed": [1] * len(input_text),
                    "page_size": 5,
                    "page_no": 0,
                    "is_last_page": pages[0]["is_last_page"],
                    "captured_all_pages": True,
                    "selected_candidates": [dict(row) for row in pages[0]["candidates"]],
                    "pages": pages,
                    "all_candidates": all_candidates,
                }
            )
        return cases

    def run_curator(self, root, pages, compose=None, metadata_mutator=None):
        pages_path = root / "pages.json"
        compose_path = root / "compose.json"
        metadata_path = root / "metadata.json"
        output_path = root / "output.json"
        pages_path.write_text(json.dumps(pages, ensure_ascii=False), encoding="utf-8")
        if compose is None:
            expected_commit = luna_curator.COMPOSITIONS[0][3]
            compose = [
                {
                    "scenario": "moboyi_compose",
                    "label": "after_select_yi",
                    "preedit": expected_commit,
                    "commit_text": expected_commit,
                }
            ]
        compose_path.write_text(json.dumps(compose, ensure_ascii=False), encoding="utf-8")
        parameters = {
            "oracle_root": "target/oracle",
            "output": "fixture.json",
            "expected_rime_dll_sha256": "a" * 64,
            "expected_rime_deployer_sha256": "b" * 64,
            "expected_luna_pinyin_commit": "c" * 40,
            "expected_prelude_commit": "d" * 40,
            "expected_essay_commit": "e" * 40,
            "expected_stroke_commit": "f" * 40,
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
            "actual_invocation": invocation,
            "effective_parameters": parameters,
        }
        if metadata_mutator is not None:
            metadata_mutator(metadata)
        metadata_path.write_text(json.dumps(metadata), encoding="utf-8")
        with contextlib.redirect_stderr(io.StringIO()):
            luna_curator.main(
                [str(pages_path), str(compose_path), str(metadata_path), str(output_path)]
            )
        return json.loads(output_path.read_text(encoding="utf-8"))

    def test_valid_complete_seven_case_capture_is_preserved(self):
        with tempfile.TemporaryDirectory() as temp:
            output = self.run_curator(Path(temp), self.valid_inputs())
        self.assertEqual(len(output["cases"]), 7)
        self.assertEqual(
            output["compositions"]["moboyi"]["final_commit"],
            luna_curator.COMPOSITIONS[0][3],
        )
        self.assertEqual(
            output["capture"]["source_row_policy"],
            "m59_lane_b_complete_order_and_partial_selection_composition",
        )
        self.assertEqual(output["capture"]["curator_version"], 5)
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

    def test_incomplete_or_malformed_capture_fails_closed(self):
        mutations = {
            "missing_case": lambda pages: pages.pop(),
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

        with tempfile.TemporaryDirectory() as temp:
            compose = [
                {
                    "scenario": "moboyi_compose",
                    "label": "after_select_yi",
                    "preedit": "莫伯洢",
                    "commit_text": None,
                }
            ]
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
            with self.assertRaises(ValueError):
                self.run_curator(
                    root,
                    self.valid_inputs(),
                    metadata_mutator=mutations["bad_hash"],
                )
            self.assertFalse(stale_output.exists())
            self.assertFalse(stale_temp.exists())


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
        luna = (SCRIPTS / "capture-m59-luna-composition.ps1").read_text(
            encoding="utf-8-sig"
        )
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
