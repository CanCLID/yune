import contextlib
import copy
import csv
import hashlib
import importlib.util
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent


def load_script(filename, module_name):
    spec = importlib.util.spec_from_file_location(module_name, SCRIPTS / filename)
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


classifier = load_script(
    "classify-m59-4b-residuals.py", "m59_4b_residual_classifier"
)
comparator = load_script("compare-candidate-order.py", "m59_4b_raw_comparator")


class M59Increment4bClassifierTests(unittest.TestCase):
    PREEDITS = {
        "n": "n",
        "nri": "nri",
        "ngohaig": "ngo hai g",
        "ngohaigo": "ngo hai go",
        "bein": "be in",
    }
    PREVIEWS = {
        "n": "\u6211",
        "nri": "\u6211ri",
        "ngohaig": "\u6211\u4fc2\u5649",
        "ngohaigo": "\u6211\u7e6b\u500b",
        "bein": "\u5564In",
    }

    @staticmethod
    def candidate(text, comment, index):
        return classifier.Candidate(text=text, comment=comment, global_index=index)

    def case(self, input_text, values, *, preedit=None, preview=None):
        rows = tuple(
            self.candidate(text, comment, index)
            for index, (text, comment) in enumerate(values)
        )
        return classifier.CandidateCase(
            input=input_text,
            rows=rows,
            page_size=5,
            captured_all_pages=True,
            menu_present=True,
            termination_reason="last_page",
            preedit=self.PREEDITS[input_text] if preedit is None else preedit,
            commit_text_preview=(
                self.PREVIEWS[input_text] if preview is None else preview
            ),
        )

    @staticmethod
    def mappings():
        return (
            classifier.OpenCcMapping(
                key="\u50de",
                outputs=("\u507d", "\u50de"),
                code="ngai6",
                line=1,
                locations="\u507d@chars:1;\u50de@chars:2",
            ),
            classifier.OpenCcMapping(
                key="\u81e5",
                outputs=("\u5367", "\u81e5"),
                code="ngo6",
                line=45,
                locations="\u5367@chars:3;\u81e5@chars:4",
            ),
        )

    def documents(self):
        oracle = {
            "n": self.case(
                "n",
                [
                    ("\u6211", "ngo5"),
                    ("\u507d", "ngai6"),
                    ("\u50de", "ngai6"),
                    ("\u4f60", "nei5"),
                ],
            ),
            "nri": self.case(
                "nri",
                [
                    ("\u6211", "ngo5"),
                    ("\u507d", "ngai6"),
                    ("\u50de", "ngai6"),
                    ("\u4f60", "nei5"),
                ],
            ),
            "ngohaig": self.case(
                "ngohaig",
                [
                    ("\u6211\u4fc2\u5649", "ngo5 hai6 gam2"),
                    ("\u5367", "ngo6"),
                    ("\u81e5", "ngo6"),
                    ("\u6211", "ngo5"),
                ],
            ),
            "ngohaigo": self.case(
                "ngohaigo",
                [
                    ("\u6211\u7e6b\u500b", "ngo5 hai6 go3"),
                    ("\u5367", "ngo6"),
                    ("\u81e5", "ngo6"),
                    ("\u6211", "ngo5"),
                ],
            ),
            "bein": self.case(
                "bein", [("\u5564In", "be1 in1"), ("\u5564", "be1")]
            ),
        }
        actual = {
            "n": self.case(
                "n",
                [
                    ("\u6211", "ngo5"),
                    ("\u507d", "ngai6"),
                    ("\u507d", "\r1,\u50de,ngai6,1"),
                    ("\u4f60", "nei5"),
                ],
            ),
            "nri": self.case(
                "nri",
                [
                    ("\u6211", "ngo5"),
                    ("\u507d", "ngai6"),
                    ("\u4f60", "nei5"),
                ],
            ),
            "ngohaig": self.case(
                "ngohaig",
                [
                    ("\u6211\u4fc2\u5649", "ngo5 hai6 gam2"),
                    ("\u5367", "ngo6"),
                    ("\u6211", "ngo5"),
                    ("\u5367", "\r1,\u81e5,ngo6,1"),
                ],
            ),
            "ngohaigo": self.case(
                "ngohaigo",
                [
                    ("\u6211\u7e6b\u500b", "ngo5 hai6 go3"),
                    ("\u5367", "ngo6"),
                    ("\u6211", "ngo5"),
                ],
            ),
            "bein": self.case(
                "bein", [("\u5564In", "be1 in1"), ("\u5564", "be1")]
            ),
        }
        return oracle, actual

    def test_only_declared_opencc_surface_is_normalized_and_raw_red_is_preserved(self):
        oracle, actual = self.documents()
        result = classifier.classify_documents(
            oracle,
            actual,
            {"policy": "exact", "all_accepted": False},
            self.mappings(),
        )
        self.assertTrue(result["classification_complete"])
        self.assertFalse(result["raw_comparator_all_accepted"])
        self.assertFalse(result["scope"]["increment_4c_waived"])
        self.assertIsNone(result["scope"]["exception_policy"])
        self.assertEqual(result["summary"]["raw_strict_passes"], 1)
        self.assertEqual(result["summary"]["raw_strict_failures"], 4)
        self.assertEqual(result["summary"]["normalized_text_position_passes"], 5)
        n = result["cases"][0]
        self.assertEqual(
            n["raw_difference_attribution"], "declared-opencc-4c-surface-only"
        )
        group = n["normalization"]["declared_opencc_groups"][0]
        self.assertEqual(group["oracle_normalized_index"], 1)
        self.assertEqual(group["actual_normalized_index"], 1)
        self.assertEqual(len(group["oracle_occurrences"]), 2)
        self.assertEqual(len(group["actual_occurrences"]), 2)
        self.assertEqual(
            group["actual_claimed_source_surfaces"], ["\u507d", "\u50de"]
        )

    def test_equal_arity_repeated_surface_is_not_normalized_away(self):
        oracle, actual = self.documents()
        oracle["n"] = self.case(
            "n",
            [("\u507d", "ngai6"), ("\u50de", "ngai6")],
        )
        actual["n"] = self.case(
            "n",
            [("\u507d", "ngai6"), ("\u507d", "ngai6")],
        )

        result = classifier.classify_documents(
            oracle,
            actual,
            {"policy": "exact", "all_accepted": False},
            self.mappings(),
        )

        self.assertFalse(result["classification_complete"])
        n = result["cases"][0]
        self.assertEqual(n["raw_difference_attribution"], "unowned")
        group = n["normalization"]["declared_opencc_groups"][0]
        self.assertEqual(group["actual_claimed_source_surfaces"], ["\u507d"])
        self.assertEqual(len(group["actual_excess_occurrences"]), 1)
        self.assertEqual(
            group["actual_excess_occurrences"][0]["duplicate_source_surfaces"],
            ["\u507d"],
        )
        self.assertIn(
            "opencc-source-surface-multiplicity-exceeds-declared-multiset",
            n["classification_reasons"],
        )

    def test_single_declared_surface_remains_allowed_for_deferred_4c(self):
        for surface in ("\u507d", "\u50de"):
            with self.subTest(surface=surface):
                oracle, actual = self.documents()
                oracle["n"] = self.case(
                    "n",
                    [("\u507d", "ngai6"), ("\u50de", "ngai6")],
                )
                actual["n"] = self.case("n", [(surface, "ngai6")])

                result = classifier.classify_documents(
                    oracle,
                    actual,
                    {"policy": "exact", "all_accepted": False},
                    self.mappings(),
                )

                self.assertTrue(result["classification_complete"])
                group = result["cases"][0]["normalization"][
                    "declared_opencc_groups"
                ][0]
                self.assertEqual(group["actual_claimed_source_surfaces"], [surface])
                self.assertEqual(group["actual_excess_occurrences"], [])

    def test_excess_duplicate_multiplicity_is_not_normalized_away(self):
        oracle, actual = self.documents()
        actual["n"] = self.case(
            "n",
            [
                ("\u507d", "ngai6"),
                ("\u507d", "ngai6"),
                ("\u507d", "ngai6"),
            ],
        )
        oracle["n"] = self.case(
            "n",
            [("\u507d", "ngai6"), ("\u50de", "ngai6")],
        )

        result = classifier.classify_documents(
            oracle,
            actual,
            {"policy": "exact", "all_accepted": False},
            self.mappings(),
        )

        self.assertFalse(result["classification_complete"])
        n = result["cases"][0]
        self.assertEqual(n["raw_difference_attribution"], "unowned")
        self.assertFalse(
            n["normalization"][
                "actual_multiplicity_within_declared_outputs"
            ]
        )
        self.assertIn(
            "opencc-source-surface-multiplicity-exceeds-declared-multiset",
            n["classification_reasons"],
        )
        group = n["normalization"]["declared_opencc_groups"][0]
        self.assertEqual(group["max_collapsible_occurrences"], 2)
        self.assertEqual(len(group["actual_occurrences"]), 3)
        self.assertEqual(len(group["actual_excess_occurrences"]), 2)

    def test_nonvariant_order_residual_fails_closed(self):
        oracle, actual = self.documents()
        rows = list(actual["nri"].rows)
        rows[0], rows[-1] = rows[-1], rows[0]
        rows = [
            classifier.Candidate(row.text, row.comment, index)
            for index, row in enumerate(rows)
        ]
        actual["nri"] = classifier.CandidateCase(
            **{**actual["nri"].__dict__, "rows": tuple(rows)}
        )
        result = classifier.classify_documents(
            oracle,
            actual,
            {"policy": "exact", "all_accepted": False},
            self.mappings(),
        )
        self.assertFalse(result["classification_complete"])
        nri = result["cases"][1]
        self.assertEqual(nri["raw_difference_attribution"], "unowned")
        self.assertIn(
            "residual-outside-declared-opencc-4c-surface",
            nri["classification_reasons"],
        )

    def test_preedit_and_commit_preview_are_strict_segmentation_gates(self):
        oracle, actual = self.documents()
        actual["bein"] = self.case(
            "bein",
            [("\u5564In", "be1 in1"), ("\u5564", "be1")],
            preedit="bein",
            preview="wrong",
        )
        result = classifier.classify_documents(
            oracle,
            actual,
            {"policy": "exact", "all_accepted": False},
            self.mappings(),
        )
        self.assertFalse(result["classification_complete"])
        bein = result["cases"][-1]
        self.assertEqual(
            bein["classification_reasons"],
            ["preedit-segmentation-mismatch", "commit-preview-mismatch"],
        )

    def test_inventory_surface_with_wrong_code_is_not_normalized(self):
        oracle, actual = self.documents()
        actual["nri"] = self.case(
            "nri",
            [
                ("\u6211", "ngo5"),
                ("\u507d", "not-a-jyutping-code"),
                ("\u4f60", "nei5"),
            ],
        )
        result = classifier.classify_documents(
            oracle,
            actual,
            {"policy": "exact", "all_accepted": False},
            self.mappings(),
        )
        nri = result["cases"][1]
        self.assertEqual(nri["raw_difference_attribution"], "unowned")
        unmatched = nri["normalization"][
            "actual_inventory_surfaces_with_unmatched_code"
        ]
        self.assertEqual(unmatched[0]["text"], "\u507d")

    def test_ambiguous_same_surface_code_provenance_is_structural_failure(self):
        case = self.case("n", [("\u920e", "gau1; ngau1")])
        mappings = (
            classifier.OpenCcMapping(
                key="\u9264",
                outputs=("\u920e", "\u9264"),
                code="gau1",
                line=61,
                locations="x",
            ),
            classifier.OpenCcMapping(
                key="\u9264",
                outputs=("\u920e", "\u9264"),
                code="ngau1",
                line=61,
                locations="x",
            ),
        )
        with self.assertRaisesRegex(classifier.EvidenceError, "ambiguous OpenCC"):
            classifier.normalize_case(case, mappings)

    def test_candidate_codes_use_structured_records_not_translation_text(self):
        self.assertEqual(
            classifier._candidate_codes("ngo5 hai6 gam2"),
            {"ngo5", "hai6", "gam2"},
        )
        rich = "\x0c\r1,\u507d,ngai6,1,0,,adj,,,,,,,translation gau1"
        self.assertEqual(classifier._candidate_codes(rich), {"ngai6"})

    def capture_document(self, *, oracle=False, extra=False, bad_index=False):
        inputs = list(classifier.CLASS4_INPUTS)
        if extra:
            inputs.append("extra")
        cases = []
        for input_text in inputs:
            canonical = input_text if input_text in self.PREEDITS else "extra"
            cases.append(
                {
                    "input": input_text,
                    "page_size": 5,
                    "menu_present": True,
                    "termination_reason": None if oracle else "last_page",
                    "captured_all_pages": True,
                    "preedit": self.PREEDITS.get(canonical, canonical),
                    "commit_text_preview": self.PREVIEWS.get(canonical, canonical),
                    "all_candidates": [
                        {
                            "text": canonical,
                            "comment": "ngo5",
                            "index": 0,
                            "global_index": 1 if bad_index and input_text == "n" else 0,
                        }
                    ],
                }
            )
        return {"inputs": inputs, "cases": cases}

    def test_actual_capture_accepts_exact_five_only_and_checks_positions(self):
        parsed = classifier.parse_capture(
            self.capture_document(), "actual", oracle=False
        )
        self.assertEqual(tuple(parsed), classifier.CLASS4_INPUTS)
        with self.assertRaisesRegex(classifier.EvidenceError, "actual inputs must be exactly"):
            classifier.parse_capture(
                self.capture_document(extra=True), "actual", oracle=False
            )
        with self.assertRaisesRegex(classifier.EvidenceError, "global_index"):
            classifier.parse_capture(
                self.capture_document(bad_index=True), "actual", oracle=False
            )

    def test_full_lane_oracle_may_contain_other_rows_but_selects_only_contract_five(self):
        document = self.capture_document(oracle=True, extra=True)
        parsed = classifier.parse_capture(document, "oracle", oracle=True)
        self.assertEqual(tuple(parsed), classifier.CLASS4_INPUTS)
        self.assertNotIn("extra", parsed)

    def comparator_documents(self):
        oracle_cases, actual_cases = self.documents()

        def document(cases, oracle):
            rows = []
            for input_text in classifier.CLASS4_INPUTS:
                case = cases[input_text]
                rows.append(
                    {
                        "input": input_text,
                        "page_size": 5,
                        "menu_present": True,
                        "termination_reason": None if oracle else "last_page",
                        "captured_all_pages": True,
                        "preedit": case.preedit,
                        "commit_text_preview": case.commit_text_preview,
                        "all_candidates": [
                            {
                                "text": candidate.text,
                                "comment": candidate.comment,
                                "index": index % 5,
                                "global_index": index,
                            }
                            for index, candidate in enumerate(case.rows)
                        ],
                    }
                )
            return {"inputs": list(classifier.CLASS4_INPUTS), "cases": rows}

        return document(oracle_cases, True), document(actual_cases, False)

    def test_raw_comparator_is_recomputed_and_must_be_exception_free(self):
        oracle_document, actual_document = self.comparator_documents()
        oracle_cases = classifier.parse_capture(
            oracle_document, "oracle", oracle=True
        )
        actual_cases = classifier.parse_capture(
            actual_document, "actual", oracle=False
        )
        strict = comparator.compare_documents(
            oracle_document,
            actual_document,
            policy="exact",
            selected_inputs=classifier.CLASS4_INPUTS,
        )
        strict["provenance"] = {
            "oracle": {"sha256": "a" * 64},
            "actual": {"sha256": "b" * 64},
            "exceptions": None,
        }
        summary = classifier.validate_strict_comparator(
            strict, oracle_cases, actual_cases, "a" * 64, "b" * 64
        )
        self.assertFalse(summary["all_accepted"])
        strict["provenance"]["exceptions"] = {"sha256": "c" * 64}
        with self.assertRaisesRegex(classifier.EvidenceError, "must not apply"):
            classifier.validate_strict_comparator(
                strict, oracle_cases, actual_cases, "a" * 64, "b" * 64
            )

    def test_raw_comparator_tampering_is_rejected(self):
        oracle_document, actual_document = self.comparator_documents()
        oracle_cases = classifier.parse_capture(
            oracle_document, "oracle", oracle=True
        )
        actual_cases = classifier.parse_capture(
            actual_document, "actual", oracle=False
        )
        strict = comparator.compare_documents(
            oracle_document,
            actual_document,
            policy="exact",
            selected_inputs=classifier.CLASS4_INPUTS,
        )
        strict["provenance"] = {
            "oracle": {"sha256": "a" * 64},
            "actual": {"sha256": "b" * 64},
            "exceptions": None,
        }
        strict["cases"][0]["diff_opcodes"] = []
        with self.assertRaisesRegex(classifier.EvidenceError, "diff_opcodes"):
            classifier.validate_strict_comparator(
                strict, oracle_cases, actual_cases, "a" * 64, "b" * 64
            )

    def write_inventory(self, root, *, source_text="\u50de \u507d \u50de\n"):
        source = root / "HKVariantsFull.txt"
        source.write_text(source_text, encoding="utf-8")
        source_sha = hashlib.sha256(source.read_bytes()).hexdigest()
        inventory = root / "inventory.csv"
        fields = [
            "dictionary_commit",
            "dictionary_tree",
            "dictionary_manifest_sha256",
            "dictionary_import_tables",
            "opencc_sha256",
            "opencc_line",
            "key",
            "outputs",
            "code",
            "siblings",
            "locations",
        ]
        with inventory.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
            writer.writeheader()
            writer.writerow(
                {
                    "dictionary_commit": "1" * 40,
                    "dictionary_tree": "2" * 40,
                    "dictionary_manifest_sha256": "3" * 64,
                    "dictionary_import_tables": ";".join(
                        classifier.EXPECTED_IMPORT_TABLES
                    ),
                    "opencc_sha256": source_sha,
                    "opencc_line": "1",
                    "key": "\u50de",
                    "outputs": "\u507d \u50de",
                    "code": "ngai6",
                    "siblings": "\u507d \u50de",
                    "locations": "\u507d@chars:1;\u50de@chars:2",
                }
            )
        return inventory, source, source_sha

    def test_inventory_is_bound_to_source_and_dictionary_provenance(self):
        with tempfile.TemporaryDirectory() as temp:
            inventory, source, source_sha = self.write_inventory(Path(temp))
            mappings = classifier.parse_opencc_inventory(
                inventory,
                source,
                expected_commit="1" * 40,
                expected_tree="2" * 40,
                expected_manifest_sha256="3" * 64,
                expected_opencc_sha256=source_sha,
            )
            self.assertEqual(mappings[0].outputs, ("\u507d", "\u50de"))
            source.write_text("\u50de \u507d\n", encoding="utf-8")
            with self.assertRaisesRegex(classifier.EvidenceError, "SHA-256 mismatch"):
                classifier.parse_opencc_inventory(
                    inventory,
                    source,
                    expected_commit="1" * 40,
                    expected_tree="2" * 40,
                    expected_manifest_sha256="3" * 64,
                    expected_opencc_sha256=source_sha,
                )

    @staticmethod
    def pinned_contract_args():
        return SimpleNamespace(
            expected_oracle_sha256=classifier.PINNED_LANE_A_ORACLE_SHA256,
            expected_opencc_inventory_sha256=(
                classifier.PINNED_OPENCC_INVENTORY_SHA256
            ),
            expected_opencc_source_sha256=classifier.PINNED_OPENCC_SOURCE_SHA256,
            expected_dictionary_commit=classifier.PINNED_RIME_CANTONESE_COMMIT,
            expected_dictionary_tree=classifier.PINNED_RIME_CANTONESE_TREE,
            expected_dictionary_manifest_sha256=(
                classifier.PINNED_DICTIONARY_MANIFEST_SHA256
            ),
        )

    def test_checked_in_capture_script_and_probe_bytes_are_verified(self):
        filenames = (
            "inventory-opencc-same-code.ps1",
            "capture-yune-candidate-order.ps1",
            "oracle-rime-probe.cs",
        )
        mutations = (
            (
                "capture-yune-candidate-order.ps1",
                "Yune capture script bytes changed",
            ),
            ("oracle-rime-probe.cs", "capture probe bytes changed"),
        )
        for mutated_name, message in mutations:
            with (
                self.subTest(mutated_name=mutated_name),
                tempfile.TemporaryDirectory() as temp,
            ):
                root = Path(temp)
                scripts = root / "scripts"
                scripts.mkdir()
                for filename in filenames:
                    (scripts / filename).write_bytes(
                        (SCRIPTS / filename).read_bytes()
                    )
                with mock.patch.object(classifier, "REPO_ROOT", root):
                    classifier._validate_pinned_cli_contract(
                        self.pinned_contract_args()
                    )
                    mutated = scripts / mutated_name
                    mutated.write_bytes(mutated.read_bytes() + b"\nmutation\n")
                    with self.assertRaisesRegex(classifier.EvidenceError, message):
                        classifier._validate_pinned_cli_contract(
                            self.pinned_contract_args()
                        )

    def test_atomic_json_is_deterministic_utf8_with_one_lf(self):
        result = {"verdict": "pass", "text": "\u507d\u50de"}
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            first_path = Path(first) / "result.json"
            second_path = Path(second) / "result.json"
            classifier.write_json_atomic(first_path, result)
            classifier.write_json_atomic(second_path, result)
            first_bytes = first_path.read_bytes()
            self.assertEqual(first_bytes, second_path.read_bytes())
            self.assertFalse(first_bytes.startswith(b"\xef\xbb\xbf"))
            self.assertNotIn(b"\r", first_bytes)
            self.assertTrue(first_bytes.endswith(b"\n"))
            self.assertFalse(first_bytes.endswith(b"\n\n"))

    @staticmethod
    def write_json(path, document):
        path.write_text(
            json.dumps(document, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
            newline="\n",
        )

    def real_cli_fixture(self, root):
        oracle = (
            REPO_ROOT
            / "docs/reports/evidence/m59-canonical-jyutping-reachability-parity"
            / "increment-1-executable-evidence/lane-a-oracle.json"
        )
        inventory = (
            REPO_ROOT
            / "docs/reports/evidence/m59-canonical-jyutping-reachability-parity"
            / "increment-2-profile-paging/opencc-same-code-inventory.csv"
        )
        opencc = REPO_ROOT / "crates/yune-core/src/opencc/data/HKVariantsFull.txt"
        oracle_document = json.loads(oracle.read_text(encoding="utf-8-sig"))
        mappings = classifier.parse_opencc_inventory(
            inventory,
            opencc,
            expected_commit=classifier.PINNED_RIME_CANTONESE_COMMIT,
            expected_tree=classifier.PINNED_RIME_CANTONESE_TREE,
            expected_manifest_sha256=(
                classifier.PINNED_DICTIONARY_MANIFEST_SHA256
            ),
            expected_opencc_sha256=classifier.PINNED_OPENCC_SOURCE_SHA256,
        )
        memberships = classifier._membership_index(mappings)
        cases = []
        for input_text in classifier.CLASS4_INPUTS:
            case = copy.deepcopy(
                next(
                    row
                    for row in oracle_document["cases"]
                    if row["input"] == input_text
                )
            )
            for index, row in enumerate(case["all_candidates"]):
                candidate = classifier.Candidate(
                    text=row["text"],
                    comment=row.get("comment", ""),
                    global_index=index,
                )
                mapping = classifier._candidate_mapping(candidate, memberships)
                if mapping is not None:
                    source_text = row["text"]
                    row["text"] = mapping.outputs[0]
                    if source_text != row["text"]:
                        row["comment"] = (
                            f"\r1,{source_text},{mapping.code},1"
                        )
            cases.append(case)
        yune_commit = "4" * 40
        yune_tree = "5" * 40
        yune_dll = "6" * 64
        runtime_options = {
            "ascii_mode": False,
            "full_shape": False,
            "ascii_punct": False,
            "zh_hans": False,
        }
        actual_document = {
            "capture": {
                "engine": "yune",
                "source_commit": yune_commit,
                "source_tree": yune_tree,
                "source_clean": True,
                "source_dirty": False,
                "source_status_short": [],
                "schema_id": "jyut6ping3",
                "modules": ["default"],
                "yune_dll_sha256": yune_dll,
                "probe_sha256": classifier.PINNED_CAPTURE_PROBE_SHA256,
                "capture_script_sha256": (
                    classifier.PINNED_CAPTURE_SCRIPT_SHA256
                ),
                "oracle_capture_sha256": (
                    classifier.PINNED_LANE_A_ORACLE_SHA256
                ),
                "source_shared_tree_sha256": (
                    classifier.PINNED_SOURCE_SHARED_TREE_SHA256
                ),
                "staged_shared_tree_sha256": (
                    classifier.PINNED_STAGED_SHARED_TREE_SHA256
                ),
                "default_yaml_overlay_sha256": (
                    classifier.PINNED_DEFAULT_YAML_OVERLAY_SHA256
                ),
                "schema_list_narrowed": True,
                "narrow_schema_list_switch_used": False,
                "schema_list_narrowing_source": "default_yaml_overlay",
                "runtime_options": runtime_options,
                "effective_parameters": {
                    "schema_id": "jyut6ping3",
                    "inputs": list(classifier.CLASS4_INPUTS),
                    "inputs_source": "explicit",
                    "schema_list_narrowed": True,
                    "narrow_schema_list_switch_used": False,
                    "schema_list_narrowing_source": "default_yaml_overlay",
                    "runtime_options": runtime_options,
                    "expected_yune_dll_sha256": yune_dll,
                    "allow_dirty": False,
                    "keep_work_root": False,
                },
            },
            "inputs": list(classifier.CLASS4_INPUTS),
            "cases": cases,
        }
        actual = root / "actual.json"
        self.write_json(actual, actual_document)
        oracle_sha = hashlib.sha256(oracle.read_bytes()).hexdigest()
        actual_sha = hashlib.sha256(actual.read_bytes()).hexdigest()
        strict_document = comparator.compare_documents(
            oracle_document,
            actual_document,
            policy="exact",
            selected_inputs=classifier.CLASS4_INPUTS,
        )
        strict_document["provenance"] = {
            "oracle": {"sha256": oracle_sha},
            "actual": {"sha256": actual_sha},
            "exceptions": None,
        }
        strict = root / "strict.json"
        self.write_json(strict, strict_document)
        output = root / "classification.json"
        args = [
            "--oracle",
            str(oracle),
            "--expected-oracle-sha256",
            classifier.PINNED_LANE_A_ORACLE_SHA256,
            "--actual",
            str(actual),
            "--expected-actual-sha256",
            actual_sha,
            "--strict-comparator",
            str(strict),
            "--expected-strict-comparator-sha256",
            hashlib.sha256(strict.read_bytes()).hexdigest(),
            "--opencc-inventory",
            str(inventory),
            "--expected-opencc-inventory-sha256",
            classifier.PINNED_OPENCC_INVENTORY_SHA256,
            "--opencc-source",
            str(opencc),
            "--expected-opencc-source-sha256",
            classifier.PINNED_OPENCC_SOURCE_SHA256,
            "--expected-dictionary-commit",
            classifier.PINNED_RIME_CANTONESE_COMMIT,
            "--expected-dictionary-tree",
            classifier.PINNED_RIME_CANTONESE_TREE,
            "--expected-dictionary-manifest-sha256",
            classifier.PINNED_DICTIONARY_MANIFEST_SHA256,
            "--expected-yune-commit",
            yune_commit,
            "--expected-yune-tree",
            yune_tree,
            "--expected-yune-dll-sha256",
            yune_dll,
            "--output",
            str(output),
        ]
        return args, actual, strict, output

    def test_real_pinned_inputs_complete_an_end_to_end_cli_success(self):
        with tempfile.TemporaryDirectory() as temp:
            args, _, _, output = self.real_cli_fixture(Path(temp))
            self.assertEqual(classifier.main(args), 0)
            result = json.loads(output.read_text(encoding="utf-8"))
            self.assertTrue(result["classification_complete"])
            self.assertEqual(result["summary"]["raw_strict_failures"], 4)
            self.assertEqual(
                result["summary"]["normalized_text_position_passes"], 5
            )

    def test_hash_valid_malformed_capture_invalidates_stale_output(self):
        with tempfile.TemporaryDirectory() as temp:
            args, actual, _, output = self.real_cli_fixture(Path(temp))
            document = json.loads(actual.read_text(encoding="utf-8"))
            del document["capture"]["source_commit"]
            self.write_json(actual, document)
            actual_sha = hashlib.sha256(actual.read_bytes()).hexdigest()
            args[args.index("--expected-actual-sha256") + 1] = actual_sha
            output.write_text("stale", encoding="utf-8")
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = classifier.main(args)
            self.assertEqual(exit_code, 2)
            self.assertIn("actual source commit", stderr.getvalue())
            self.assertFalse(output.exists())

    def test_input_mutation_during_classification_invalidates_stale_output(self):
        with tempfile.TemporaryDirectory() as temp:
            args, actual, _, output = self.real_cli_fixture(Path(temp))
            output.write_text("stale", encoding="utf-8")
            original_classify = classifier.classify_documents

            def classify_then_mutate(*classify_args):
                result = original_classify(*classify_args)
                actual.write_bytes(actual.read_bytes() + b"\n")
                return result

            stderr = io.StringIO()
            with (
                mock.patch.object(
                    classifier,
                    "classify_documents",
                    side_effect=classify_then_mutate,
                ),
                contextlib.redirect_stderr(stderr),
            ):
                exit_code = classifier.main(args)

            self.assertEqual(exit_code, 2)
            self.assertIn("actual changed during classification", stderr.getvalue())
            self.assertFalse(output.exists())

    def test_unowned_residual_writes_red_output_and_exits_one(self):
        with tempfile.TemporaryDirectory() as temp:
            args, actual, strict, output = self.real_cli_fixture(Path(temp))
            actual_document = json.loads(actual.read_text(encoding="utf-8"))
            bein = next(
                row for row in actual_document["cases"] if row["input"] == "bein"
            )
            bein["all_candidates"][0], bein["all_candidates"][1] = (
                bein["all_candidates"][1],
                bein["all_candidates"][0],
            )
            for index, row in enumerate(bein["all_candidates"]):
                row["global_index"] = index
                row["index"] = index
            self.write_json(actual, actual_document)
            actual_sha = hashlib.sha256(actual.read_bytes()).hexdigest()
            args[args.index("--expected-actual-sha256") + 1] = actual_sha
            oracle = Path(args[args.index("--oracle") + 1])
            oracle_document = json.loads(oracle.read_text(encoding="utf-8-sig"))
            strict_document = comparator.compare_documents(
                oracle_document,
                actual_document,
                policy="exact",
                selected_inputs=classifier.CLASS4_INPUTS,
            )
            strict_document["provenance"] = {
                "oracle": {
                    "sha256": classifier.PINNED_LANE_A_ORACLE_SHA256
                },
                "actual": {"sha256": actual_sha},
                "exceptions": None,
            }
            self.write_json(strict, strict_document)
            args[args.index("--expected-strict-comparator-sha256") + 1] = (
                hashlib.sha256(strict.read_bytes()).hexdigest()
            )
            self.assertEqual(classifier.main(args), 1)
            result = json.loads(output.read_text(encoding="utf-8"))
            self.assertFalse(result["classification_complete"])
            self.assertEqual(result["summary"]["unowned_residual_cases"], 1)

    def test_cli_hash_failure_invalidates_stale_output(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            paths = {}
            for name in ("oracle", "actual", "strict", "inventory", "opencc"):
                paths[name] = root / f"{name}.txt"
                paths[name].write_text("{}", encoding="utf-8")
            output = root / "result.json"
            output.write_text("stale", encoding="utf-8")
            args = [
                "--oracle",
                str(paths["oracle"]),
                "--expected-oracle-sha256",
                classifier.PINNED_LANE_A_ORACLE_SHA256,
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
                classifier.PINNED_OPENCC_INVENTORY_SHA256,
                "--opencc-source",
                str(paths["opencc"]),
                "--expected-opencc-source-sha256",
                classifier.PINNED_OPENCC_SOURCE_SHA256,
                "--expected-dictionary-commit",
                classifier.PINNED_RIME_CANTONESE_COMMIT,
                "--expected-dictionary-tree",
                classifier.PINNED_RIME_CANTONESE_TREE,
                "--expected-dictionary-manifest-sha256",
                classifier.PINNED_DICTIONARY_MANIFEST_SHA256,
                "--expected-yune-commit",
                "4" * 40,
                "--expected-yune-tree",
                "5" * 40,
                "--expected-yune-dll-sha256",
                "6" * 64,
                "--output",
                str(output),
            ]
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr):
                exit_code = classifier.main(args)
            self.assertEqual(exit_code, 2)
            self.assertIn("SHA-256 mismatch", stderr.getvalue())
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
