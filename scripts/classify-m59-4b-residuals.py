#!/usr/bin/env python3
"""Classify the M59 Increment 4b abbreviation/segmentation residual.

The raw exact comparator remains the authoritative, visibly-red record.  This
tool independently verifies that record, then applies one narrowly derived
normalization: candidate surfaces belonging to the checked-in, same-code
OpenCC one-to-many inventory are represented by their declared mapping group
and stable-deduplicated.  The five contract rows must have exact normalized
text/position order, preedit, and commit preview.  No exception policy is read
or produced, and every residual outside that declared 4c surface fails closed.
"""

from __future__ import annotations

import argparse
import collections
import csv
import difflib
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from itertools import zip_longest
from pathlib import Path
from typing import Any, Iterable, Sequence


TOOL_NAME = "classify-m59-4b-residuals.py"
TOOL_VERSION = "1"
REPO_ROOT = Path(__file__).resolve().parent.parent
CLASS4_INPUTS = ("n", "nri", "ngohaig", "ngohaigo", "bein")
EXPECTED_IMPORT_TABLES = (
    "jyut6ping3.chars",
    "jyut6ping3.words",
    "jyut6ping3.phrase",
    "jyut6ping3.lettered",
    "jyut6ping3.maps",
)
PINNED_LIBRIME_COMMIT = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
PINNED_RIME_CANTONESE_COMMIT = "c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0"
PINNED_RIME_CANTONESE_TREE = "eb193fb80675ffa60df3c32bf24afa7d7f68617a"
PINNED_DICTIONARY_MANIFEST_SHA256 = (
    "4301001fb7bb52d5d1a9c032c519ac18ba50677e926e01006e34a48788385efa"
)
PINNED_LANE_A_ORACLE_SHA256 = (
    "56e7aafabcfac7eb7d3b209d5929eff88c65aeaa1b98fee0464e6b1fcda8c1ca"
)
PINNED_OPENCC_INVENTORY_SHA256 = (
    "01522f437038a3591d3a3b92cbdace2cced1b1e9076e566ca40662c736afcaf1"
)
PINNED_OPENCC_SOURCE_SHA256 = (
    "145b561c68a697d5f2197da0c091caf4a0e9457f0a4c56cdf2ae7ad4b8ff8cc2"
)
PINNED_INVENTORY_GENERATOR_SHA256 = (
    "5dc88c282a7dd8dec6d4c43a5be08e90d6b6b7276c383d5decd704e0c7b363f7"
)
PINNED_CAPTURE_SCRIPT_SHA256 = (
    "c2614bc7f068d89903d7b0af3856f286f07194e35f505806251aa2ba887aa45c"
)
PINNED_CAPTURE_PROBE_SHA256 = (
    "94f7deb7c3632a6c3c918536295b03d88aa8a80bbbbc9d8a26e896fb70bf07e7"
)
PINNED_SOURCE_SHARED_TREE_SHA256 = (
    "b76c540136b54162df6cc00a2b9159719fb7c207c99e5a6c9625abbe08aa9e5c"
)
PINNED_STAGED_SHARED_TREE_SHA256 = (
    "a169531b04449c38875eca42c14ce2ddb95d32037f2178de603e0d80c47adfd8"
)
PINNED_DEFAULT_YAML_OVERLAY_SHA256 = (
    "ab0beef16410765c1b7157a27f406990c7a6f6330e4e0c76d95e2b44b4050f7f"
)
PINNED_COMPARATOR_VERSION = "5"
PINNED_COMPARATOR_SHA256 = (
    "d20eccc78822dd612eefd39966586a5c87cd5bbe8be4386634a20c52c139f612"
)
CODE_RE = re.compile(r"(?<![A-Za-z0-9])([a-z]+[1-6])(?=$|[^A-Za-z0-9])")
PATH_ARGUMENT_ROLES = {
    "--oracle": "oracle",
    "--actual": "actual",
    "--strict-comparator": "strict-comparator",
    "--opencc-inventory": "opencc-inventory",
    "--opencc-source": "opencc-source",
    "--output": "output",
}


class EvidenceError(ValueError):
    """An evidence input is malformed, ambiguous, or provenance-invalid."""


@dataclass(frozen=True)
class Candidate:
    text: str
    comment: str
    global_index: int


@dataclass(frozen=True)
class CandidateCase:
    input: str
    rows: tuple[Candidate, ...]
    page_size: int
    captured_all_pages: bool
    menu_present: bool
    termination_reason: str | None
    preedit: str
    commit_text_preview: str


@dataclass(frozen=True)
class OpenCcMapping:
    key: str
    outputs: tuple[str, ...]
    code: str
    line: int
    locations: str

    @property
    def identity(self) -> tuple[int, str, str]:
        return (self.line, self.key, self.code)


@dataclass(frozen=True)
class NormalizedRow:
    identity: tuple[str, ...]
    text: str
    code: str | None
    inventory_line: int | None


def _canonical_path_key(path: Path) -> str:
    return os.path.normcase(os.path.realpath(path))


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise EvidenceError(f"cannot hash {path}: {error}") from error
    return digest.hexdigest()


def _read_bytes(path: Path, label: str) -> bytes:
    try:
        return path.read_bytes()
    except OSError as error:
        raise EvidenceError(f"cannot read {label} {path}: {error}") from error


def _expected_sha256(value: Any, label: str) -> str:
    if not isinstance(value, str):
        raise EvidenceError(f"{label} must be a string")
    normalized = value.lower()
    if re.fullmatch(r"[0-9a-f]{64}", normalized) is None:
        raise EvidenceError(f"{label} must be exactly 64 hexadecimal characters")
    return normalized


def _expected_object_id(value: Any, label: str) -> str:
    if not isinstance(value, str):
        raise EvidenceError(f"{label} must be a string")
    normalized = value.lower()
    if re.fullmatch(r"[0-9a-f]{40}", normalized) is None:
        raise EvidenceError(f"{label} must be exactly 40 hexadecimal characters")
    return normalized


def _verify_sha256(path: Path, expected: str, label: str) -> str:
    expected = _expected_sha256(expected, f"expected {label} SHA-256")
    actual = _file_sha256(path)
    if actual != expected:
        raise EvidenceError(
            f"{label} SHA-256 mismatch: expected {expected}, actual {actual}"
        )
    return actual


def _read_verified_bytes(
    path: Path, expected: str, label: str
) -> tuple[bytes, str]:
    expected = _expected_sha256(expected, f"expected {label} SHA-256")
    data = _read_bytes(path, label)
    actual = hashlib.sha256(data).hexdigest()
    if actual != expected:
        raise EvidenceError(
            f"{label} SHA-256 mismatch: expected {expected}, actual {actual}"
        )
    return data, actual


def _load_json_bytes(data: bytes, path: Path, label: str) -> Any:
    try:
        return json.loads(data.decode("utf-8-sig"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise EvidenceError(f"cannot read {label} JSON {path}: {error}") from error


def _load_json(path: Path, label: str) -> Any:
    return _load_json_bytes(_read_bytes(path, f"{label} JSON"), path, label)


def _logical_path(path: Path, role: str) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return f"external/{role}"


def _logical_argv(values: Sequence[str]) -> list[str]:
    result: list[str] = []
    index = 0
    while index < len(values):
        value = values[index]
        role = PATH_ARGUMENT_ROLES.get(value)
        if role is not None:
            if index + 1 >= len(values):
                raise EvidenceError(f"{value} has no path argument")
            result.extend((value, _logical_path(Path(values[index + 1]), role)))
            index += 2
            continue
        matched = False
        for flag, inline_role in PATH_ARGUMENT_ROLES.items():
            prefix = flag + "="
            if value.startswith(prefix):
                result.append(
                    prefix + _logical_path(Path(value[len(prefix) :]), inline_role)
                )
                matched = True
                break
        if not matched:
            result.append(value)
        index += 1
    return result


def _preflight_paths(inputs: Sequence[tuple[str, Path]], output: Path) -> None:
    output_key = _canonical_path_key(output)
    for role, path in inputs:
        if output_key == _canonical_path_key(path):
            raise EvidenceError(f"--output must not alias {role}: {output}")


def _cases_array(document: Any, label: str) -> list[Any]:
    if isinstance(document, list):
        return document
    if isinstance(document, dict) and isinstance(document.get("cases"), list):
        return document["cases"]
    capture = document.get("capture") if isinstance(document, dict) else None
    if isinstance(capture, dict) and isinstance(capture.get("cases"), list):
        return capture["cases"]
    raise EvidenceError(f"{label} must contain a cases array")


def _required_string(raw: dict[str, Any], field: str, label: str) -> str:
    value = raw.get(field)
    if not isinstance(value, str):
        raise EvidenceError(f"{label}.{field} must be a string")
    return value


def _parse_case(raw: Any, label: str, *, oracle: bool) -> CandidateCase:
    if not isinstance(raw, dict):
        raise EvidenceError(f"{label} must be an object")
    input_text = raw.get("input")
    if not isinstance(input_text, str) or not input_text:
        raise EvidenceError(f"{label}.input must be a non-empty string")
    raw_candidates = raw.get("all_candidates")
    if not isinstance(raw_candidates, list):
        raise EvidenceError(f"{label}.all_candidates must be an array")
    page_size = raw.get("page_size")
    if not isinstance(page_size, int) or isinstance(page_size, bool) or page_size <= 0:
        raise EvidenceError(f"{label}.page_size must be a positive integer")
    if raw.get("captured_all_pages") is not True:
        raise EvidenceError(f"{label} must be a complete all-pages capture")
    if raw.get("menu_present", True) is not True:
        raise EvidenceError(f"{label}.menu_present must be true")
    termination_reason = raw.get("termination_reason")
    allowed_termination = {None, "last_page"} if oracle else {"last_page"}
    if termination_reason not in allowed_termination:
        raise EvidenceError(
            f"{label}.termination_reason must be "
            + ("absent/last_page" if oracle else "last_page")
        )
    rows: list[Candidate] = []
    for index, candidate in enumerate(raw_candidates):
        candidate_label = f"{label}.all_candidates[{index}]"
        if not isinstance(candidate, dict):
            raise EvidenceError(f"{candidate_label} must be an object")
        text = candidate.get("text")
        comment = candidate.get("comment", "")
        global_index = candidate.get("global_index")
        page_index = candidate.get("index")
        if not isinstance(text, str) or not text:
            raise EvidenceError(f"{candidate_label}.text must be non-empty")
        if comment is None:
            comment = ""
        if not isinstance(comment, str):
            raise EvidenceError(f"{candidate_label}.comment must be a string")
        if (
            not isinstance(global_index, int)
            or isinstance(global_index, bool)
            or global_index != index
        ):
            raise EvidenceError(
                f"{candidate_label}.global_index must equal raw row position"
            )
        if (
            not isinstance(page_index, int)
            or isinstance(page_index, bool)
            or page_index != index % page_size
        ):
            raise EvidenceError(
                f"{candidate_label}.index must equal global_index modulo page_size"
            )
        rows.append(Candidate(text=text, comment=comment, global_index=global_index))
    return CandidateCase(
        input=input_text,
        rows=tuple(rows),
        page_size=page_size,
        captured_all_pages=True,
        menu_present=True,
        termination_reason=termination_reason,
        preedit=_required_string(raw, "preedit", label),
        commit_text_preview=_required_string(raw, "commit_text_preview", label),
    )


def parse_capture(
    document: Any, label: str, *, oracle: bool
) -> dict[str, CandidateCase]:
    parsed_all: dict[str, CandidateCase] = {}
    raw_inputs: list[str] = []
    for index, raw in enumerate(_cases_array(document, label)):
        case = _parse_case(raw, f"{label}.cases[{index}]", oracle=oracle)
        if case.input in parsed_all:
            raise EvidenceError(f"{label} contains duplicate case {case.input!r}")
        parsed_all[case.input] = case
        raw_inputs.append(case.input)
    missing = [value for value in CLASS4_INPUTS if value not in parsed_all]
    if missing:
        raise EvidenceError(f"{label} is missing fixed class-4 inputs: {missing}")
    if not oracle and tuple(raw_inputs) != CLASS4_INPUTS:
        raise EvidenceError(
            f"actual inputs must be exactly {CLASS4_INPUTS!r} in that order; "
            f"found {tuple(raw_inputs)!r}"
        )
    declared_inputs = document.get("inputs") if isinstance(document, dict) else None
    if declared_inputs is not None:
        if not isinstance(declared_inputs, list) or not all(
            isinstance(value, str) for value in declared_inputs
        ):
            raise EvidenceError(f"{label}.inputs must be an array of strings")
        if declared_inputs != raw_inputs:
            raise EvidenceError(f"{label}.inputs must match case order exactly")
    return {value: parsed_all[value] for value in CLASS4_INPUTS}


def validate_capture_provenance(
    oracle_document: Any,
    actual_document: Any,
    oracle_sha256: str,
    *,
    expected_yune_commit: str,
    expected_yune_tree: str,
    expected_yune_dll_sha256: str,
) -> dict[str, Any]:
    if not isinstance(oracle_document, dict):
        raise EvidenceError("oracle document must be an object with provenance")
    oracle = oracle_document.get("oracle")
    schema = oracle_document.get("schema")
    if not isinstance(oracle, dict) or not isinstance(schema, dict):
        raise EvidenceError("oracle engine/schema provenance is missing")
    expected_oracle = {
        "engine": "rime/librime",
        "version": "1.17.0",
        "commit": PINNED_LIBRIME_COMMIT,
    }
    for field, expected in expected_oracle.items():
        if oracle.get(field) != expected:
            raise EvidenceError(f"oracle {field} does not match the pinned lane")
    if schema.get("source_repo") != "rime/rime-cantonese":
        raise EvidenceError("oracle schema source_repo must be rime/rime-cantonese")
    if schema.get("source_commit") != PINNED_RIME_CANTONESE_COMMIT:
        raise EvidenceError("oracle schema source_commit does not match the pinned lane")

    if not isinstance(actual_document, dict):
        raise EvidenceError("actual document must be an object with provenance")
    capture = actual_document.get("capture")
    if not isinstance(capture, dict):
        raise EvidenceError("actual capture provenance is missing")
    if capture.get("engine") != "yune":
        raise EvidenceError("actual capture engine must be yune")
    if capture.get("schema_id") != "jyut6ping3":
        raise EvidenceError("actual capture schema_id must be jyut6ping3")
    if capture.get("source_clean") is not True or capture.get("source_dirty") is not False:
        raise EvidenceError("actual capture source must be recorded clean")
    if capture.get("source_status_short") != []:
        raise EvidenceError("actual capture source_status_short must be empty")
    expected_yune_commit = _expected_object_id(
        expected_yune_commit, "expected Yune commit"
    )
    expected_yune_tree = _expected_object_id(expected_yune_tree, "expected Yune tree")
    expected_yune_dll_sha256 = _expected_sha256(
        expected_yune_dll_sha256, "expected Yune DLL SHA-256"
    )
    source_commit = _expected_object_id(
        capture.get("source_commit"), "actual source commit"
    )
    source_tree = _expected_object_id(capture.get("source_tree"), "actual source tree")
    if source_commit != expected_yune_commit or source_tree != expected_yune_tree:
        raise EvidenceError(
            "actual capture source identity does not match the explicitly expected "
            "Yune commit/tree"
        )
    if capture.get("oracle_capture_sha256") != oracle_sha256:
        raise EvidenceError("actual capture does not bind the supplied oracle SHA-256")
    dll_sha256 = _expected_sha256(
        capture.get("yune_dll_sha256"), "actual Yune DLL SHA-256"
    )
    if dll_sha256 != expected_yune_dll_sha256:
        raise EvidenceError(
            "actual capture DLL does not match the explicitly expected SHA-256"
        )
    pinned_hash_fields = {
        "capture_script_sha256": PINNED_CAPTURE_SCRIPT_SHA256,
        "probe_sha256": PINNED_CAPTURE_PROBE_SHA256,
        "source_shared_tree_sha256": PINNED_SOURCE_SHARED_TREE_SHA256,
        "staged_shared_tree_sha256": PINNED_STAGED_SHARED_TREE_SHA256,
        "default_yaml_overlay_sha256": PINNED_DEFAULT_YAML_OVERLAY_SHA256,
    }
    for field, expected in pinned_hash_fields.items():
        if capture.get(field) != expected:
            raise EvidenceError(f"actual capture {field} does not match the pinned lane")
    if capture.get("modules") != ["default"]:
        raise EvidenceError("actual capture modules must be exactly ['default']")
    if (
        capture.get("schema_list_narrowed") is not True
        or capture.get("narrow_schema_list_switch_used") is not False
        or capture.get("schema_list_narrowing_source") != "default_yaml_overlay"
    ):
        raise EvidenceError("actual capture must use the pinned narrowed default.yaml lane")
    expected_options = {
        "ascii_mode": False,
        "full_shape": False,
        "ascii_punct": False,
        "zh_hans": False,
    }
    if capture.get("runtime_options") != expected_options:
        raise EvidenceError("actual capture runtime options do not match the canonical lane")
    effective = capture.get("effective_parameters")
    if not isinstance(effective, dict):
        raise EvidenceError("actual capture effective_parameters are missing")
    expected_effective = {
        "schema_id": "jyut6ping3",
        "inputs": list(CLASS4_INPUTS),
        "inputs_source": "explicit",
        "schema_list_narrowed": True,
        "narrow_schema_list_switch_used": False,
        "schema_list_narrowing_source": "default_yaml_overlay",
        "runtime_options": expected_options,
        "expected_yune_dll_sha256": expected_yune_dll_sha256,
        "allow_dirty": False,
        "keep_work_root": False,
    }
    for field, expected in expected_effective.items():
        if effective.get(field) != expected:
            raise EvidenceError(
                f"actual capture effective_parameters.{field} does not match the lane"
            )
    return {
        "oracle_engine_commit": PINNED_LIBRIME_COMMIT,
        "oracle_schema_commit": PINNED_RIME_CANTONESE_COMMIT,
        "actual_source_commit": source_commit,
        "actual_source_tree": source_tree,
        "actual_yune_dll_sha256": dll_sha256,
        "actual_source_clean": True,
        "capture_script_sha256": PINNED_CAPTURE_SCRIPT_SHA256,
        "capture_probe_sha256": PINNED_CAPTURE_PROBE_SHA256,
        "source_shared_tree_sha256": PINNED_SOURCE_SHARED_TREE_SHA256,
        "staged_shared_tree_sha256": PINNED_STAGED_SHARED_TREE_SHA256,
        "default_yaml_overlay_sha256": PINNED_DEFAULT_YAML_OVERLAY_SHA256,
    }


def _raw_first_mismatch(left: Sequence[str], right: Sequence[str]) -> int | None:
    sentinel = object()
    for index, (left_row, right_row) in enumerate(
        zip_longest(left, right, fillvalue=sentinel)
    ):
        if left_row != right_row:
            return index
    return None


def _counter_difference(
    oracle: Sequence[str], actual: Sequence[str]
) -> tuple[list[str], list[str]]:
    oracle_counts = collections.Counter(oracle)
    actual_counts = collections.Counter(actual)
    missing: list[str] = []
    extra: list[str] = []
    for text in sorted(oracle_counts):
        missing.extend([text] * max(0, oracle_counts[text] - actual_counts[text]))
    for text in sorted(actual_counts):
        extra.extend([text] * max(0, actual_counts[text] - oracle_counts[text]))
    return missing, extra


def _row_hash(rows: Iterable[str]) -> str:
    digest = hashlib.sha256()
    for row in rows:
        encoded = row.encode("utf-8")
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
    return digest.hexdigest()


def _diff_opcodes(oracle: Sequence[str], actual: Sequence[str]) -> list[dict[str, Any]]:
    matcher = difflib.SequenceMatcher(a=oracle, b=actual, autojunk=False)
    return [
        {
            "tag": tag,
            "oracle_start": oracle_start,
            "oracle_end": oracle_end,
            "actual_start": actual_start,
            "actual_end": actual_end,
        }
        for tag, oracle_start, oracle_end, actual_start, actual_end in matcher.get_opcodes()
        if tag != "equal"
    ]


def validate_strict_comparator(
    document: Any,
    oracle_cases: dict[str, CandidateCase],
    actual_cases: dict[str, CandidateCase],
    oracle_sha256: str,
    actual_sha256: str,
) -> dict[str, Any]:
    if not isinstance(document, dict):
        raise EvidenceError("strict comparator must be an object")
    if document.get("tool") != "compare-candidate-order.py":
        raise EvidenceError("strict comparator tool identity is not recognized")
    if document.get("tool_version") != PINNED_COMPARATOR_VERSION:
        raise EvidenceError("strict comparator tool_version does not match the pinned tool")
    if document.get("tool_sha256") != PINNED_COMPARATOR_SHA256:
        raise EvidenceError("strict comparator tool_sha256 does not match the pinned tool")
    comparator_path = REPO_ROOT / "scripts" / "compare-candidate-order.py"
    if _file_sha256(comparator_path) != PINNED_COMPARATOR_SHA256:
        raise EvidenceError("checked-in strict comparator bytes no longer match the pin")
    if document.get("policy") != "exact":
        raise EvidenceError("strict comparator policy must remain exact")
    if document.get("inputs") != list(CLASS4_INPUTS):
        raise EvidenceError("strict comparator inputs do not match the fixed 4b set")
    provenance = document.get("provenance")
    if not isinstance(provenance, dict):
        raise EvidenceError("strict comparator provenance is missing")
    for role, expected in (("oracle", oracle_sha256), ("actual", actual_sha256)):
        record = provenance.get(role)
        if not isinstance(record, dict) or record.get("sha256") != expected:
            raise EvidenceError(
                f"strict comparator embedded {role} SHA-256 does not match its input"
            )
    if provenance.get("exceptions") is not None:
        raise EvidenceError("strict comparator must not apply an exception policy")
    raw_cases = document.get("cases")
    if not isinstance(raw_cases, list) or len(raw_cases) != len(CLASS4_INPUTS):
        raise EvidenceError("strict comparator cases do not match the fixed input set")
    if [raw.get("input") if isinstance(raw, dict) else None for raw in raw_cases] != list(
        CLASS4_INPUTS
    ):
        raise EvidenceError("strict comparator case order does not match the fixed input set")

    verdicts: list[str] = []
    for raw in raw_cases:
        input_text = raw["input"]
        oracle_case = oracle_cases[input_text]
        actual_case = actual_cases[input_text]
        oracle_texts = [row.text for row in oracle_case.rows]
        actual_texts = [row.text for row in actual_case.rows]
        missing, extra = _counter_difference(oracle_texts, actual_texts)
        exact = oracle_texts == actual_texts
        failures: list[str] = []
        if not oracle_case.captured_all_pages:
            failures.append("oracle-incomplete")
        if not actual_case.captured_all_pages:
            failures.append("actual-incomplete")
        if oracle_case.page_size != actual_case.page_size:
            failures.append("page-size")
        if oracle_case.menu_present != actual_case.menu_present:
            failures.append("menu-presence")
        if missing:
            failures.append("under-admission")
        if extra:
            failures.append("over-admission")
        if not exact:
            failures.append("order")
        verdict = "pass" if not failures else "fail"
        verdicts.append(verdict)
        expected_fields = {
            "policy": "exact",
            "oracle_count": len(oracle_texts),
            "actual_count": len(actual_texts),
            "oracle_captured_all_pages": True,
            "actual_captured_all_pages": True,
            "page_size": {
                "oracle": oracle_case.page_size,
                "actual": actual_case.page_size,
            },
            "menu_present": {"oracle": True, "actual": True},
            "termination_reason": {
                "oracle": oracle_case.termination_reason,
                "actual": actual_case.termination_reason,
            },
            "raw_first_mismatch_index": _raw_first_mismatch(
                oracle_texts, actual_texts
            ),
            "order_matches_after_signed_exceptions": exact,
            "missing_count": len(missing),
            "extra_count": len(extra),
            "missing_examples": missing[:20],
            "extra_examples": extra[:20],
            "oracle_rows_sha256": _row_hash(oracle_texts),
            "actual_rows_sha256": _row_hash(actual_texts),
            "diff_opcodes": _diff_opcodes(oracle_texts, actual_texts),
            "failure_classes": failures,
            "accepted_exceptions": [],
            "used_replacements": [],
            "used_tail": None,
            "verdict": verdict,
        }
        for field, expected in expected_fields.items():
            if raw.get(field) != expected:
                raise EvidenceError(
                    f"strict comparator {input_text}.{field} is {raw.get(field)!r}; "
                    f"expected {expected!r}"
                )
    all_accepted = all(verdict == "pass" for verdict in verdicts)
    if document.get("all_accepted") != all_accepted:
        raise EvidenceError(
            "strict comparator all_accepted disagrees with recomputed case verdicts"
        )
    if all_accepted:
        raise EvidenceError("strict comparator must remain raw-red before Increment 4c")
    return {
        "tool": "compare-candidate-order.py",
        "policy": "exact",
        "inputs": list(CLASS4_INPUTS),
        "all_accepted": False,
        "exception_policy_present": False,
    }


def parse_opencc_inventory(
    path: Path,
    opencc_source: Path,
    *,
    expected_commit: str,
    expected_tree: str,
    expected_manifest_sha256: str,
    expected_opencc_sha256: str,
    inventory_bytes: bytes | None = None,
    opencc_source_bytes: bytes | None = None,
) -> tuple[OpenCcMapping, ...]:
    expected_commit = _expected_object_id(expected_commit, "dictionary commit")
    expected_tree = _expected_object_id(expected_tree, "dictionary tree")
    expected_manifest_sha256 = _expected_sha256(
        expected_manifest_sha256, "dictionary manifest SHA-256"
    )
    expected_opencc_sha256 = _expected_sha256(
        expected_opencc_sha256, "OpenCC source SHA-256"
    )
    if inventory_bytes is None:
        inventory_bytes = _read_bytes(path, "OpenCC inventory")
    if opencc_source_bytes is None:
        opencc_source_bytes = _read_bytes(opencc_source, "OpenCC source")
    try:
        inventory_text = inventory_bytes.decode("utf-8-sig")
        reader = csv.DictReader(io.StringIO(inventory_text, newline=""))
        rows = list(reader)
    except (UnicodeError, csv.Error) as error:
        raise EvidenceError(f"cannot read OpenCC inventory {path}: {error}") from error
    required = (
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
    )
    if not rows or reader.fieldnames is None or tuple(reader.fieldnames) != required:
        raise EvidenceError("OpenCC inventory header/rows are not the exact contract")
    try:
        source_lines = opencc_source_bytes.decode("utf-8-sig").splitlines()
    except UnicodeError as error:
        raise EvidenceError(f"cannot read OpenCC source {opencc_source}: {error}") from error
    actual_opencc_sha256 = hashlib.sha256(opencc_source_bytes).hexdigest()
    if actual_opencc_sha256 != expected_opencc_sha256:
        raise EvidenceError(
            "OpenCC source SHA-256 mismatch: "
            f"expected {expected_opencc_sha256}, actual {actual_opencc_sha256}"
        )

    expected_imports = ";".join(EXPECTED_IMPORT_TABLES)
    mappings: list[OpenCcMapping] = []
    seen_identities: set[tuple[int, str, str]] = set()
    memberships: set[tuple[str, str]] = set()
    previous_sort_key: tuple[int, str] | None = None
    for csv_index, row in enumerate(rows, 2):
        if None in row or any(not isinstance(row.get(field), str) for field in required):
            raise EvidenceError(f"OpenCC inventory row {csv_index} is malformed")
        pins = {
            "dictionary_commit": expected_commit,
            "dictionary_tree": expected_tree,
            "dictionary_manifest_sha256": expected_manifest_sha256,
            "dictionary_import_tables": expected_imports,
            "opencc_sha256": expected_opencc_sha256,
        }
        for field, expected in pins.items():
            if row.get(field) != expected:
                raise EvidenceError(
                    f"OpenCC inventory row {csv_index} {field} does not match provenance"
                )
        try:
            line_number = int(row["opencc_line"])
        except ValueError as error:
            raise EvidenceError(
                f"OpenCC inventory row {csv_index} has invalid source line"
            ) from error
        if line_number <= 0 or line_number > len(source_lines):
            raise EvidenceError(
                f"OpenCC inventory row {csv_index} source line is out of range"
            )
        source_fields = source_lines[line_number - 1].split()
        outputs = tuple(row["outputs"].split())
        if len(outputs) < 2 or len(outputs) != len(set(outputs)):
            raise EvidenceError(
                f"OpenCC inventory row {csv_index} must declare unique one-to-many outputs"
            )
        if not source_fields or source_fields[0] != row["key"]:
            raise EvidenceError(f"OpenCC inventory row {csv_index} key/source mismatch")
        if tuple(source_fields[1:]) != outputs:
            raise EvidenceError(
                f"OpenCC inventory row {csv_index} output/source mismatch"
            )
        if tuple(row["siblings"].split()) != outputs:
            raise EvidenceError(
                f"OpenCC inventory row {csv_index} sibling/output mismatch"
            )
        code = row["code"]
        if CODE_RE.fullmatch(code) is None:
            raise EvidenceError(f"OpenCC inventory row {csv_index} code is invalid")
        locations = row["locations"]
        location_surfaces = {
            location.split("@", 1)[0]
            for location in locations.split(";")
            if "@" in location
        }
        if not set(outputs).issubset(location_surfaces):
            raise EvidenceError(
                f"OpenCC inventory row {csv_index} lacks sibling source locations"
            )
        identity = (line_number, row["key"], code)
        if identity in seen_identities:
            raise EvidenceError(f"OpenCC inventory duplicates mapping {identity!r}")
        seen_identities.add(identity)
        sort_key = (line_number, code)
        if previous_sort_key is not None and sort_key < previous_sort_key:
            raise EvidenceError("OpenCC inventory rows are not deterministically sorted")
        previous_sort_key = sort_key
        for output in outputs:
            membership = (output, code)
            if membership in memberships:
                raise EvidenceError(
                    f"OpenCC inventory ambiguously repeats membership {membership!r}"
                )
            memberships.add(membership)
        mappings.append(
            OpenCcMapping(
                key=row["key"],
                outputs=outputs,
                code=code,
                line=line_number,
                locations=locations,
            )
        )
    return tuple(mappings)


def _membership_index(
    mappings: Sequence[OpenCcMapping],
) -> dict[str, tuple[OpenCcMapping, ...]]:
    by_surface: dict[str, list[OpenCcMapping]] = {}
    for mapping in mappings:
        for output in mapping.outputs:
            by_surface.setdefault(output, []).append(mapping)
    return {key: tuple(value) for key, value in by_surface.items()}


def _candidate_codes(comment: str) -> set[str]:
    if "\r" not in comment:
        return {
            token
            for token in re.split(r"[\s;]+", comment.strip())
            if CODE_RE.fullmatch(token) is not None
        }
    codes: set[str] = set()
    for record in comment.split("\r"):
        normalized = record.lstrip("\x0c\n")
        fields = normalized.split(",")
        if (
            len(fields) >= 3
            and fields[0] in {"0", "1"}
            and CODE_RE.fullmatch(fields[2]) is not None
        ):
            codes.add(fields[2])
    return codes


def _candidate_mapping(
    candidate: Candidate, memberships: dict[str, tuple[OpenCcMapping, ...]]
) -> OpenCcMapping | None:
    possible = memberships.get(candidate.text, ())
    if not possible:
        return None
    codes = _candidate_codes(candidate.comment)
    matches = [mapping for mapping in possible if mapping.code in codes]
    if len(matches) > 1:
        identities = [mapping.identity for mapping in matches]
        raise EvidenceError(
            f"candidate {candidate.text!r} at {candidate.global_index} has ambiguous "
            f"OpenCC code provenance: {identities!r}"
        )
    return matches[0] if matches else None


def _candidate_source_surfaces(
    candidate: Candidate, mapping: OpenCcMapping
) -> tuple[str, ...]:
    sources: list[str] = []
    if "\r" in candidate.comment:
        for record in candidate.comment.split("\r"):
            fields = record.lstrip("\x0c\n").split(",")
            if (
                len(fields) >= 3
                and fields[0] in {"0", "1"}
                and fields[2] == mapping.code
                and fields[1] in mapping.outputs
                and fields[1] not in sources
            ):
                sources.append(fields[1])
    if not sources:
        sources.append(candidate.text)
    return tuple(sources)


def normalize_case(
    case: CandidateCase, mappings: Sequence[OpenCcMapping]
) -> tuple[tuple[NormalizedRow, ...], list[dict[str, Any]], list[dict[str, Any]]]:
    memberships = _membership_index(mappings)
    normalized: list[NormalizedRow] = []
    groups_by_identity: dict[tuple[int, str, str], dict[str, Any]] = {}
    unmatched_inventory_surfaces: list[dict[str, Any]] = []
    for candidate in case.rows:
        mapping = _candidate_mapping(candidate, memberships)
        possible = memberships.get(candidate.text, ())
        if mapping is None:
            if possible:
                unmatched_inventory_surfaces.append(
                    {
                        "raw_index": candidate.global_index,
                        "text": candidate.text,
                        "observed_codes": sorted(_candidate_codes(candidate.comment)),
                        "declared_codes": sorted({item.code for item in possible}),
                    }
                )
            normalized.append(
                NormalizedRow(
                    identity=("text", candidate.text),
                    text=candidate.text,
                    code=None,
                    inventory_line=None,
                )
            )
            continue
        identity = mapping.identity
        group = groups_by_identity.get(identity)
        occurrence = {
            "raw_index": candidate.global_index,
            "text": candidate.text,
            "code": mapping.code,
            "source_surfaces": list(_candidate_source_surfaces(candidate, mapping)),
        }
        if group is not None:
            group["occurrences"].append(occurrence)
            duplicate_surfaces = sorted(
                set(occurrence["source_surfaces"])
                & set(group["claimed_source_surfaces"])
            )
            if duplicate_surfaces:
                excess = dict(occurrence)
                excess["duplicate_source_surfaces"] = duplicate_surfaces
                group["excess_occurrences"].append(excess)
                normalized.append(
                    NormalizedRow(
                        identity=(
                            "opencc-excess-multiplicity",
                            str(mapping.line),
                            mapping.key,
                            mapping.code,
                            str(candidate.global_index),
                            candidate.text,
                        ),
                        text=candidate.text,
                        code=mapping.code,
                        inventory_line=mapping.line,
                    )
                )
            else:
                group["claimed_source_surfaces"].extend(
                    occurrence["source_surfaces"]
                )
            continue
        normalized_index = len(normalized)
        groups_by_identity[identity] = {
            "inventory_line": mapping.line,
            "mapping_key": mapping.key,
            "code": mapping.code,
            "ordered_outputs": list(mapping.outputs),
            "normalized_text": mapping.outputs[0],
            "normalized_index": normalized_index,
            "max_collapsible_occurrences": len(mapping.outputs),
            "occurrences": [occurrence],
            "excess_occurrences": [],
            "claimed_source_surfaces": list(occurrence["source_surfaces"]),
        }
        normalized.append(
            NormalizedRow(
                identity=(
                    "opencc",
                    str(mapping.line),
                    mapping.key,
                    mapping.code,
                ),
                text=mapping.outputs[0],
                code=mapping.code,
                inventory_line=mapping.line,
            )
        )
    groups = [groups_by_identity[key] for key in sorted(groups_by_identity)]
    return tuple(normalized), groups, unmatched_inventory_surfaces


def _normalized_hash(rows: Sequence[NormalizedRow]) -> str:
    digest = hashlib.sha256()
    for position, row in enumerate(rows):
        fields = (str(position), *row.identity)
        for field in fields:
            encoded = field.encode("utf-8")
            digest.update(len(encoded).to_bytes(8, "big"))
            digest.update(encoded)
    return digest.hexdigest()


def _group_report(
    oracle_groups: Sequence[dict[str, Any]], actual_groups: Sequence[dict[str, Any]]
) -> list[dict[str, Any]]:
    def identity(group: dict[str, Any]) -> tuple[int, str, str]:
        return (group["inventory_line"], group["mapping_key"], group["code"])

    oracle_by_id = {identity(group): group for group in oracle_groups}
    actual_by_id = {identity(group): group for group in actual_groups}
    result: list[dict[str, Any]] = []
    for key in sorted(set(oracle_by_id) | set(actual_by_id)):
        oracle = oracle_by_id.get(key)
        actual = actual_by_id.get(key)
        template = oracle or actual
        result.append(
            {
                "inventory_line": key[0],
                "mapping_key": key[1],
                "code": key[2],
                "ordered_outputs": template["ordered_outputs"],
                "normalized_text": template["normalized_text"],
                "oracle_normalized_index": (
                    oracle["normalized_index"] if oracle is not None else None
                ),
                "actual_normalized_index": (
                    actual["normalized_index"] if actual is not None else None
                ),
                "max_collapsible_occurrences": template[
                    "max_collapsible_occurrences"
                ],
                "oracle_claimed_source_surfaces": (
                    oracle["claimed_source_surfaces"] if oracle else []
                ),
                "actual_claimed_source_surfaces": (
                    actual["claimed_source_surfaces"] if actual else []
                ),
                "oracle_occurrences": oracle["occurrences"] if oracle else [],
                "actual_occurrences": actual["occurrences"] if actual else [],
                "oracle_excess_occurrences": (
                    oracle["excess_occurrences"] if oracle else []
                ),
                "actual_excess_occurrences": (
                    actual["excess_occurrences"] if actual else []
                ),
            }
        )
    return result


def _sequence_mismatch_example(
    oracle: Sequence[NormalizedRow], actual: Sequence[NormalizedRow]
) -> dict[str, Any] | None:
    index = _raw_first_mismatch(
        [row.identity for row in oracle], [row.identity for row in actual]
    )
    if index is None:
        return None
    return {
        "index": index,
        "oracle": (
            {"text": oracle[index].text, "identity": list(oracle[index].identity)}
            if index < len(oracle)
            else None
        ),
        "actual": (
            {"text": actual[index].text, "identity": list(actual[index].identity)}
            if index < len(actual)
            else None
        ),
    }


def classify_documents(
    oracle_cases: dict[str, CandidateCase],
    actual_cases: dict[str, CandidateCase],
    strict_summary: dict[str, Any],
    mappings: Sequence[OpenCcMapping],
) -> dict[str, Any]:
    case_results: list[dict[str, Any]] = []
    for input_text in CLASS4_INPUTS:
        oracle_case = oracle_cases[input_text]
        actual_case = actual_cases[input_text]
        oracle_texts = [row.text for row in oracle_case.rows]
        actual_texts = [row.text for row in actual_case.rows]
        missing, extra = _counter_difference(oracle_texts, actual_texts)
        normalized_oracle, oracle_groups, oracle_unmatched = normalize_case(
            oracle_case, mappings
        )
        normalized_actual, actual_groups, actual_unmatched = normalize_case(
            actual_case, mappings
        )
        oracle_identities = [row.identity for row in normalized_oracle]
        actual_identities = [row.identity for row in normalized_actual]
        oracle_multiplicity_valid = all(
            not group["excess_occurrences"] for group in oracle_groups
        )
        actual_multiplicity_valid = all(
            not group["excess_occurrences"] for group in actual_groups
        )
        multiplicity_valid = (
            oracle_multiplicity_valid and actual_multiplicity_valid
        )
        normalized_exact = oracle_identities == actual_identities and multiplicity_valid
        preedit_exact = oracle_case.preedit == actual_case.preedit
        preview_exact = (
            oracle_case.commit_text_preview == actual_case.commit_text_preview
        )
        failures: list[str] = []
        if not preedit_exact:
            failures.append("preedit-segmentation-mismatch")
        if not preview_exact:
            failures.append("commit-preview-mismatch")
        if not multiplicity_valid:
            failures.append(
                "opencc-source-surface-multiplicity-exceeds-declared-multiset"
            )
        if not normalized_exact:
            failures.append("residual-outside-declared-opencc-4c-surface")
        raw_exact = oracle_texts == actual_texts
        case_results.append(
            {
                "input": input_text,
                "raw": {
                    "oracle_count": len(oracle_texts),
                    "actual_count": len(actual_texts),
                    "first_mismatch_index": _raw_first_mismatch(
                        oracle_texts, actual_texts
                    ),
                    "missing_count": len(missing),
                    "extra_count": len(extra),
                    "missing": missing,
                    "extra": extra,
                    "oracle_rows_sha256": _row_hash(oracle_texts),
                    "actual_rows_sha256": _row_hash(actual_texts),
                    "diff_opcodes": _diff_opcodes(oracle_texts, actual_texts),
                    "strict_verdict": "pass" if raw_exact else "fail",
                },
                "segmentation": {
                    "oracle_preedit": oracle_case.preedit,
                    "actual_preedit": actual_case.preedit,
                    "preedit_exact": preedit_exact,
                    "oracle_commit_text_preview": oracle_case.commit_text_preview,
                    "actual_commit_text_preview": actual_case.commit_text_preview,
                    "commit_text_preview_exact": preview_exact,
                },
                "normalization": {
                    "oracle_count": len(normalized_oracle),
                    "actual_count": len(normalized_actual),
                    "oracle_text_position_sha256": _normalized_hash(
                        normalized_oracle
                    ),
                    "actual_text_position_sha256": _normalized_hash(
                        normalized_actual
                    ),
                    "oracle_multiplicity_within_declared_outputs": (
                        oracle_multiplicity_valid
                    ),
                    "actual_multiplicity_within_declared_outputs": (
                        actual_multiplicity_valid
                    ),
                    "text_position_order_exact": normalized_exact,
                    "first_mismatch": _sequence_mismatch_example(
                        normalized_oracle, normalized_actual
                    ),
                    "declared_opencc_groups": _group_report(
                        oracle_groups, actual_groups
                    ),
                    "oracle_inventory_surfaces_with_unmatched_code": oracle_unmatched,
                    "actual_inventory_surfaces_with_unmatched_code": actual_unmatched,
                },
                "raw_difference_attribution": (
                    "none"
                    if raw_exact
                    else (
                        "declared-opencc-4c-surface-only"
                        if normalized_exact
                        else "unowned"
                    )
                ),
                "classification_reasons": failures,
                "classification_verdict": "pass" if not failures else "fail",
            }
        )
    complete = all(row["classification_verdict"] == "pass" for row in case_results)
    return {
        "scope": {
            "milestone": "M59",
            "increment": "4b",
            "decision": "D-48",
            "classification_only": True,
            "full_d48_acceptance_claimed": False,
            "increment_4c_waived": False,
            "inputs": list(CLASS4_INPUTS),
            "normalization_policy": (
                "map only inventory-proven exact candidate surfaces whose observed "
                "comment carries the inventory code; represent each mapping/code as "
                "one logical row at its first occurrence; stable-deduplicate later "
                "siblings only when structured capture provenance or the visible "
                "surface identifies a distinct declared output; fail repeated source "
                "surfaces; require exact normalized text/position order"
            ),
            "segmentation_policy": "preedit and commit preview remain strict exact",
            "exception_policy": None,
        },
        "raw_comparator": strict_summary,
        "raw_comparator_all_accepted": False,
        "classification_complete": complete,
        "classification_status": "complete" if complete else "incomplete",
        "cases": case_results,
        "summary": {
            "raw_strict_passes": sum(
                row["raw"]["strict_verdict"] == "pass" for row in case_results
            ),
            "raw_strict_failures": sum(
                row["raw"]["strict_verdict"] == "fail" for row in case_results
            ),
            "normalized_text_position_passes": sum(
                row["normalization"]["text_position_order_exact"]
                for row in case_results
            ),
            "preedit_segmentation_passes": sum(
                row["segmentation"]["preedit_exact"] for row in case_results
            ),
            "commit_preview_passes": sum(
                row["segmentation"]["commit_text_preview_exact"]
                for row in case_results
            ),
            "unowned_residual_cases": sum(
                row["raw_difference_attribution"] == "unowned"
                for row in case_results
            ),
            "opencc_inventory_mapping_count": len(mappings),
            "exception_used": False,
            "beyond_oracle_depth_used": False,
        },
    }


def _write_temp(output: Path, text: str) -> Path:
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temp_name = tempfile.mkstemp(
        prefix=f".{output.name}.", suffix=".tmp", dir=output.parent
    )
    temp_path = Path(temp_name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as handle:
            handle.write(text)
            handle.flush()
            os.fsync(handle.fileno())
    except Exception:
        temp_path.unlink(missing_ok=True)
        raise
    return temp_path


def _invalidate_output(output: Path, temp: Path | None = None) -> None:
    errors: list[str] = []
    for path in (temp, output):
        if path is None:
            continue
        try:
            path.unlink(missing_ok=True)
        except OSError as error:
            errors.append(f"cannot remove {path}: {error}")
    if errors:
        raise EvidenceError("output invalidation failed: " + "; ".join(errors))


def write_json_atomic(output: Path, result: dict[str, Any]) -> None:
    text = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    temp: Path | None = None
    try:
        temp = _write_temp(output, text)
        os.replace(temp, output)
        temp = None
    except Exception as error:
        cleanup_error: EvidenceError | None = None
        try:
            _invalidate_output(output, temp)
        except EvidenceError as invalidation_error:
            cleanup_error = invalidation_error
        detail = f"atomic output write failed: {error}"
        if cleanup_error is not None:
            detail += f"; {cleanup_error}"
        raise EvidenceError(detail) from error


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--oracle", required=True, type=Path)
    parser.add_argument("--expected-oracle-sha256", required=True)
    parser.add_argument("--actual", required=True, type=Path)
    parser.add_argument("--expected-actual-sha256", required=True)
    parser.add_argument("--strict-comparator", required=True, type=Path)
    parser.add_argument("--expected-strict-comparator-sha256", required=True)
    parser.add_argument("--opencc-inventory", required=True, type=Path)
    parser.add_argument("--expected-opencc-inventory-sha256", required=True)
    parser.add_argument("--opencc-source", required=True, type=Path)
    parser.add_argument("--expected-opencc-source-sha256", required=True)
    parser.add_argument("--expected-dictionary-commit", required=True)
    parser.add_argument("--expected-dictionary-tree", required=True)
    parser.add_argument("--expected-dictionary-manifest-sha256", required=True)
    parser.add_argument("--expected-yune-commit", required=True)
    parser.add_argument("--expected-yune-tree", required=True)
    parser.add_argument("--expected-yune-dll-sha256", required=True)
    parser.add_argument("--output", required=True, type=Path)
    return parser


def _validate_pinned_cli_contract(args: argparse.Namespace) -> None:
    expected = {
        "--expected-oracle-sha256": (
            args.expected_oracle_sha256,
            PINNED_LANE_A_ORACLE_SHA256,
            "SHA-256",
        ),
        "--expected-opencc-inventory-sha256": (
            args.expected_opencc_inventory_sha256,
            PINNED_OPENCC_INVENTORY_SHA256,
            "SHA-256",
        ),
        "--expected-opencc-source-sha256": (
            args.expected_opencc_source_sha256,
            PINNED_OPENCC_SOURCE_SHA256,
            "SHA-256",
        ),
        "--expected-dictionary-commit": (
            args.expected_dictionary_commit,
            PINNED_RIME_CANTONESE_COMMIT,
            "object id",
        ),
        "--expected-dictionary-tree": (
            args.expected_dictionary_tree,
            PINNED_RIME_CANTONESE_TREE,
            "object id",
        ),
        "--expected-dictionary-manifest-sha256": (
            args.expected_dictionary_manifest_sha256,
            PINNED_DICTIONARY_MANIFEST_SHA256,
            "SHA-256",
        ),
    }
    for flag, (observed, pinned, kind) in expected.items():
        normalized = (
            _expected_sha256(observed, flag)
            if kind == "SHA-256"
            else _expected_object_id(observed, flag)
        )
        if normalized != pinned:
            raise EvidenceError(
                f"{flag} must equal the checked-in M59 4b {kind}: {pinned}"
            )
    generator = REPO_ROOT / "scripts" / "inventory-opencc-same-code.ps1"
    if _file_sha256(generator) != PINNED_INVENTORY_GENERATOR_SHA256:
        raise EvidenceError("checked-in OpenCC inventory generator bytes changed")
    capture_script = REPO_ROOT / "scripts" / "capture-yune-candidate-order.ps1"
    if _file_sha256(capture_script) != PINNED_CAPTURE_SCRIPT_SHA256:
        raise EvidenceError("checked-in Yune capture script bytes changed")
    capture_probe = REPO_ROOT / "scripts" / "oracle-rime-probe.cs"
    if _file_sha256(capture_probe) != PINNED_CAPTURE_PROBE_SHA256:
        raise EvidenceError("checked-in capture probe bytes changed")


def _verify_input_files_unchanged(
    inputs: Sequence[tuple[str, Path, str]], hashes: dict[str, str]
) -> None:
    for key, path, label in inputs:
        actual = _file_sha256(path)
        expected = hashes[key]
        if actual != expected:
            raise EvidenceError(
                f"{label} changed during classification: expected SHA-256 "
                f"{expected}, actual {actual}"
            )


def main(argv: Sequence[str] | None = None) -> int:
    effective_args = list(sys.argv[1:] if argv is None else argv)
    args = _parser().parse_args(effective_args)
    input_paths = (
        ("--tool", Path(__file__).resolve()),
        ("--oracle", args.oracle),
        ("--actual", args.actual),
        ("--strict-comparator", args.strict_comparator),
        ("--opencc-inventory", args.opencc_inventory),
        ("--opencc-source", args.opencc_source),
    )
    try:
        _preflight_paths(input_paths, args.output)
    except EvidenceError as error:
        print(f"M59 4b residual evidence error: {error}", file=sys.stderr)
        return 2
    try:
        _validate_pinned_cli_contract(args)
        verified_inputs = (
            ("oracle", args.oracle, args.expected_oracle_sha256, "oracle"),
            ("actual", args.actual, args.expected_actual_sha256, "actual"),
            (
                "strict_comparator",
                args.strict_comparator,
                args.expected_strict_comparator_sha256,
                "strict comparator",
            ),
            (
                "opencc_inventory",
                args.opencc_inventory,
                args.expected_opencc_inventory_sha256,
                "OpenCC inventory",
            ),
            (
                "opencc_source",
                args.opencc_source,
                args.expected_opencc_source_sha256,
                "OpenCC source",
            ),
        )
        input_bytes: dict[str, bytes] = {}
        hashes: dict[str, str] = {}
        for key, path, expected, label in verified_inputs:
            data, actual = _read_verified_bytes(path, expected, label)
            input_bytes[key] = data
            hashes[key] = actual
        mappings = parse_opencc_inventory(
            args.opencc_inventory,
            args.opencc_source,
            expected_commit=args.expected_dictionary_commit,
            expected_tree=args.expected_dictionary_tree,
            expected_manifest_sha256=args.expected_dictionary_manifest_sha256,
            expected_opencc_sha256=hashes["opencc_source"],
            inventory_bytes=input_bytes["opencc_inventory"],
            opencc_source_bytes=input_bytes["opencc_source"],
        )
        oracle_document = _load_json_bytes(
            input_bytes["oracle"], args.oracle, "oracle"
        )
        actual_document = _load_json_bytes(
            input_bytes["actual"], args.actual, "actual"
        )
        strict_document = _load_json_bytes(
            input_bytes["strict_comparator"],
            args.strict_comparator,
            "strict comparator",
        )
        capture_provenance = validate_capture_provenance(
            oracle_document,
            actual_document,
            hashes["oracle"],
            expected_yune_commit=args.expected_yune_commit,
            expected_yune_tree=args.expected_yune_tree,
            expected_yune_dll_sha256=args.expected_yune_dll_sha256,
        )
        oracle_cases = parse_capture(oracle_document, "oracle", oracle=True)
        actual_cases = parse_capture(actual_document, "actual", oracle=False)
        strict_summary = validate_strict_comparator(
            strict_document,
            oracle_cases,
            actual_cases,
            hashes["oracle"],
            hashes["actual"],
        )
        result = classify_documents(
            oracle_cases, actual_cases, strict_summary, mappings
        )
        tool_path = Path(__file__).resolve()
        logical_args = _logical_argv(effective_args)
        logical_tool_path = _logical_path(tool_path, "tool")
        result.update(
            {
                "tool": TOOL_NAME,
                "tool_version": TOOL_VERSION,
                "tool_sha256": _file_sha256(tool_path),
                "provenance": {
                    "oracle": {
                        "path": _logical_path(args.oracle, "oracle"),
                        "sha256": hashes["oracle"],
                    },
                    "actual": {
                        "path": _logical_path(args.actual, "actual"),
                        "sha256": hashes["actual"],
                    },
                    "strict_comparator": {
                        "path": _logical_path(
                            args.strict_comparator, "strict-comparator"
                        ),
                        "sha256": hashes["strict_comparator"],
                        "raw_red_preserved": True,
                        "exceptions": None,
                    },
                    "opencc_inventory": {
                        "path": _logical_path(
                            args.opencc_inventory, "opencc-inventory"
                        ),
                        "sha256": hashes["opencc_inventory"],
                        "mapping_count": len(mappings),
                        "dictionary_commit": args.expected_dictionary_commit.lower(),
                        "dictionary_tree": args.expected_dictionary_tree.lower(),
                        "dictionary_manifest_sha256": (
                            args.expected_dictionary_manifest_sha256.lower()
                        ),
                        "dictionary_import_tables": list(EXPECTED_IMPORT_TABLES),
                        "generator_path": "scripts/inventory-opencc-same-code.ps1",
                        "generator_sha256": PINNED_INVENTORY_GENERATOR_SHA256,
                    },
                    "opencc_source": {
                        "path": _logical_path(args.opencc_source, "opencc-source"),
                        "sha256": hashes["opencc_source"],
                    },
                    "capture_identity": capture_provenance,
                    "effective_argv": logical_args,
                    "effective_invocation": subprocess.list2cmdline(
                        ["python", logical_tool_path, *logical_args]
                    ),
                },
            }
        )
        _verify_input_files_unchanged(
            tuple((key, path, label) for key, path, _, label in verified_inputs),
            hashes,
        )
        write_json_atomic(args.output, result)
    except (EvidenceError, OSError, UnicodeError) as error:
        cleanup_error: EvidenceError | None = None
        try:
            _invalidate_output(args.output)
        except EvidenceError as invalidation_error:
            cleanup_error = invalidation_error
        detail = str(error)
        if cleanup_error is not None:
            detail += f"; {cleanup_error}"
        print(f"M59 4b residual evidence error: {detail}", file=sys.stderr)
        return 2
    return 0 if result["classification_complete"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
