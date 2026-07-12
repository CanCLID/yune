#!/usr/bin/env python3
"""Fail-closed acceptance classifier for M59 Increment 4c.

The supplied strict comparator is the authoritative raw record.  This tool
does not normalize it and accepts no exception policy: all thirteen canonical
Lane A rows must already be exact.  It then reconciles the complete checked-in
OpenCC same-code inventory to its pinned source, verifies the external
ConvertWord oracle fixture and its manifest row, and proves every inventory
group visible in the Lane A oracle remains at the same text, order, and
position in Yune's complete all-page capture.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import importlib.util
import io
import json
import os
import re
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType
from typing import Any, Iterable, Sequence


TOOL_NAME = "classify-m59-4c-residuals.py"
TOOL_VERSION = "1"
REPO_ROOT = Path(__file__).resolve().parent.parent
CANONICAL_INPUTS = (
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
)
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
PINNED_OPENCC_FIXTURE_SHA256 = (
    "fafdb3b6ae5f7ac77d797dcc282c359e315669d2bb44bfca804cf7b7c56f8437"
)
PINNED_ORACLE_MANIFEST_SHA256 = (
    "7f765e004c340dc9002a542ca6f136395a289393e307359d96c80468da1add3f"
)
PINNED_COMPARATOR_VERSION = "5"
PINNED_COMPARATOR_SHA256 = (
    "d20eccc78822dd612eefd39966586a5c87cd5bbe8be4386634a20c52c139f612"
)
PINNED_INVENTORY_GENERATOR_SHA256 = (
    "5dc88c282a7dd8dec6d4c43a5be08e90d6b6b7276c383d5decd704e0c7b363f7"
)
PINNED_OPENCC_CAPTURE_SCRIPT_SHA256 = (
    "90c37c321247852bea03920b1ec573760c9f04e2286cc8bdf57531c3824f5a9a"
)
PINNED_CANDIDATE_CAPTURE_SCRIPT_SHA256 = (
    "c2614bc7f068d89903d7b0af3856f286f07194e35f505806251aa2ba887aa45c"
)
PINNED_CAPTURE_PROBE_SHA256 = (
    "94f7deb7c3632a6c3c918536295b03d88aa8a80bbbbc9d8a26e896fb70bf07e7"
)
PINNED_DEFAULT_YAML_OVERLAY_SHA256 = (
    "ab0beef16410765c1b7157a27f406990c7a6f6330e4e0c76d95e2b44b4050f7f"
)
CODE_RE = re.compile(r"(?<![A-Za-z0-9])([a-z]+[1-6])(?=$|[^A-Za-z0-9])")
PATH_ARGUMENT_ROLES = {
    "--oracle": "oracle",
    "--actual": "actual",
    "--strict-comparator": "strict-comparator",
    "--opencc-inventory": "opencc-inventory",
    "--opencc-source": "opencc-source",
    "--opencc-oracle-fixture": "opencc-oracle-fixture",
    "--oracle-manifest": "oracle-manifest",
    "--output": "output",
}


class EvidenceError(ValueError):
    """An evidence input is missing, ambiguous, or contract-invalid."""


@dataclass(frozen=True)
class Candidate:
    text: str
    comment: str
    index: int
    global_index: int


@dataclass(frozen=True)
class CaptureCase:
    input: str
    candidates: tuple[Candidate, ...]
    page_size: int
    pages: tuple[dict[str, Any], ...]
    preedit: str
    commit_text_preview: str
    control: dict[str, Any]


@dataclass(frozen=True)
class OpenCcMapping:
    source_line: int
    key: str
    outputs: tuple[str, ...]
    code: str
    locations: str

    @property
    def identity(self) -> tuple[int, str, str]:
        return (self.source_line, self.key, self.code)


def _canonical_path_key(path: Path) -> str:
    return os.path.normcase(os.path.realpath(path))


def _read_bytes(path: Path, label: str) -> bytes:
    try:
        return path.read_bytes()
    except OSError as error:
        raise EvidenceError(f"cannot read {label} {path}: {error}") from error


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _file_sha256(path: Path) -> str:
    return _sha256_bytes(_read_bytes(path, "file"))


def _expected_hash(value: Any, label: str, length: int) -> str:
    if not isinstance(value, str) or re.fullmatch(
        rf"[0-9a-fA-F]{{{length}}}", value
    ) is None:
        raise EvidenceError(f"{label} must be exactly {length} hexadecimal characters")
    return value.lower()


def _expected_sha256(value: Any, label: str) -> str:
    return _expected_hash(value, label, 64)


def _expected_object_id(value: Any, label: str) -> str:
    return _expected_hash(value, label, 40)


def _load_json_bytes(data: bytes, path: Path, label: str) -> Any:
    try:
        return json.loads(data.decode("utf-8-sig"))
    except (UnicodeError, json.JSONDecodeError) as error:
        raise EvidenceError(f"cannot read {label} JSON {path}: {error}") from error


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
    seen: dict[str, str] = {}
    for role, path in inputs:
        key = _canonical_path_key(path)
        if key in seen:
            raise EvidenceError(f"{role} aliases {seen[key]}: {path}")
        seen[key] = role
    output_key = _canonical_path_key(output)
    if output_key in seen:
        raise EvidenceError(f"--output aliases {seen[output_key]}: {output}")
    if output.exists() or output.is_symlink():
        raise EvidenceError(f"--output already exists; create-new policy forbids overwrite: {output}")


def _required_string(raw: dict[str, Any], field: str, label: str) -> str:
    value = raw.get(field)
    if not isinstance(value, str):
        raise EvidenceError(f"{label}.{field} must be a string")
    return value


def _parse_candidate(raw: Any, label: str, expected_global_index: int) -> Candidate:
    if not isinstance(raw, dict):
        raise EvidenceError(f"{label} must be an object")
    text = raw.get("text")
    comment = raw.get("comment", "")
    index = raw.get("index")
    global_index = raw.get("global_index")
    if not isinstance(text, str) or not text:
        raise EvidenceError(f"{label}.text must be a non-empty string")
    if comment is None:
        comment = ""
    if not isinstance(comment, str):
        raise EvidenceError(f"{label}.comment must be a string")
    for value, field in ((index, "index"), (global_index, "global_index")):
        if not isinstance(value, int) or isinstance(value, bool) or value < 0:
            raise EvidenceError(f"{label}.{field} must be a non-negative integer")
    if global_index != expected_global_index:
        raise EvidenceError(
            f"{label}.global_index must equal row position {expected_global_index}"
        )
    return Candidate(text, comment, index, global_index)


CONTROL_FIELDS = (
    "schema_id",
    "input",
    "rime_get_input",
    "processed",
    "is_composing",
    "is_ascii_mode",
    "preedit",
    "commit_text_preview",
    "highlighted_candidate_index",
    "page_size",
    "page_no",
    "num_candidates",
    "is_last_page",
    "candidate_pointer_null",
    "menu_present",
    "captured_all_pages",
    "termination_reason",
)


def _parse_case(raw: Any, label: str) -> CaptureCase:
    if not isinstance(raw, dict):
        raise EvidenceError(f"{label} must be an object")
    input_text = raw.get("input")
    if not isinstance(input_text, str) or not input_text:
        raise EvidenceError(f"{label}.input must be a non-empty string")
    if "pagination_error" in raw:
        raise EvidenceError(f"{label} must not carry pagination_error")
    page_size = raw.get("page_size")
    if not isinstance(page_size, int) or isinstance(page_size, bool) or page_size <= 0:
        raise EvidenceError(f"{label}.page_size must be a positive integer")
    if raw.get("captured_all_pages") is not True:
        raise EvidenceError(f"{label} must be a complete all-pages capture")
    if raw.get("menu_present") is not True:
        raise EvidenceError(f"{label}.menu_present must be true")
    if raw.get("termination_reason") != "last_page":
        raise EvidenceError(f"{label}.termination_reason must be last_page")
    raw_candidates = raw.get("all_candidates")
    if not isinstance(raw_candidates, list) or not raw_candidates:
        raise EvidenceError(f"{label}.all_candidates must be a non-empty array")
    candidates = tuple(
        _parse_candidate(item, f"{label}.all_candidates[{index}]", index)
        for index, item in enumerate(raw_candidates)
    )
    for candidate in candidates:
        if candidate.index != candidate.global_index % page_size:
            raise EvidenceError(
                f"{label}.all_candidates[{candidate.global_index}].index is inconsistent"
            )

    raw_pages = raw.get("pages")
    if not isinstance(raw_pages, list) or not raw_pages:
        raise EvidenceError(f"{label}.pages must be a non-empty array")
    expected_page_count = (len(candidates) + page_size - 1) // page_size
    if len(raw_pages) != expected_page_count:
        raise EvidenceError(
            f"{label}.pages count must be {expected_page_count}, found {len(raw_pages)}"
        )
    pages: list[dict[str, Any]] = []
    for page_no, page in enumerate(raw_pages):
        page_label = f"{label}.pages[{page_no}]"
        if not isinstance(page, dict):
            raise EvidenceError(f"{page_label} must be an object")
        if set(page) != {"page_no", "page_size", "is_last_page", "candidates"}:
            raise EvidenceError(f"{page_label} fields are not the exact page contract")
        if page.get("page_no") != page_no or page.get("page_size") != page_size:
            raise EvidenceError(f"{page_label} page number/size is inconsistent")
        if page.get("is_last_page") is not (page_no == expected_page_count - 1):
            raise EvidenceError(f"{page_label}.is_last_page is inconsistent")
        page_candidates = page.get("candidates")
        if not isinstance(page_candidates, list):
            raise EvidenceError(f"{page_label}.candidates must be an array")
        start = page_no * page_size
        expected_slice = candidates[start : start + page_size]
        parsed_page = tuple(
            _parse_candidate(item, f"{page_label}.candidates[{index}]", start + index)
            for index, item in enumerate(page_candidates)
        )
        if parsed_page != expected_slice:
            raise EvidenceError(f"{page_label}.candidates do not reproduce all_candidates")
        pages.append(page)

    selected = raw.get("selected_candidates")
    if not isinstance(selected, list):
        raise EvidenceError(f"{label}.selected_candidates must be an array")
    parsed_selected = tuple(
        _parse_candidate(item, f"{label}.selected_candidates[{index}]", index)
        for index, item in enumerate(selected)
    )
    if parsed_selected != candidates[:page_size]:
        raise EvidenceError(f"{label}.selected_candidates must reproduce the first page")
    control = {field: raw.get(field) for field in CONTROL_FIELDS}
    if any(field not in raw for field in CONTROL_FIELDS):
        missing = [field for field in CONTROL_FIELDS if field not in raw]
        raise EvidenceError(f"{label} lacks control fields: {missing}")
    return CaptureCase(
        input=input_text,
        candidates=candidates,
        page_size=page_size,
        pages=tuple(pages),
        preedit=_required_string(raw, "preedit", label),
        commit_text_preview=_required_string(raw, "commit_text_preview", label),
        control=control,
    )


def parse_capture(document: Any, label: str) -> dict[str, CaptureCase]:
    if not isinstance(document, dict) or not isinstance(document.get("cases"), list):
        raise EvidenceError(f"{label} must be an object with a cases array")
    raw_cases = document["cases"]
    raw_inputs: list[str] = []
    parsed: dict[str, CaptureCase] = {}
    for index, raw in enumerate(raw_cases):
        case = _parse_case(raw, f"{label}.cases[{index}]")
        if case.input in parsed:
            raise EvidenceError(f"{label} contains duplicate input {case.input!r}")
        parsed[case.input] = case
        raw_inputs.append(case.input)
    if tuple(raw_inputs) != CANONICAL_INPUTS:
        missing = [value for value in CANONICAL_INPUTS if value not in parsed]
        extra = [value for value in raw_inputs if value not in CANONICAL_INPUTS]
        raise EvidenceError(
            f"{label} inputs must be exactly the canonical 13 in order; "
            f"missing={missing}, extra={extra}, observed={raw_inputs}"
        )
    declared = document.get("inputs")
    if declared is not None and declared != raw_inputs:
        raise EvidenceError(f"{label}.inputs must exactly match case order")
    return parsed


def _candidate_codes(comment: str) -> set[str]:
    if "\r" not in comment:
        return {
            token
            for token in re.split(r"[\s;]+", comment.strip())
            if CODE_RE.fullmatch(token) is not None
        }
    codes: set[str] = set()
    for record in comment.split("\r"):
        fields = record.lstrip("\x0c\n").split(",")
        if len(fields) >= 3 and fields[0] in {"0", "1"} and CODE_RE.fullmatch(fields[2]):
            codes.add(fields[2])
    return codes


def parse_opencc_inventory(
    inventory_bytes: bytes,
    source_bytes: bytes,
) -> tuple[tuple[OpenCcMapping, ...], dict[str, Any]]:
    try:
        reader = csv.DictReader(
            io.StringIO(inventory_bytes.decode("utf-8-sig"), newline="")
        )
        rows = list(reader)
        source_lines = source_bytes.decode("utf-8-sig").splitlines()
    except (UnicodeError, csv.Error) as error:
        raise EvidenceError(f"cannot parse OpenCC source/inventory: {error}") from error
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
    if reader.fieldnames is None or tuple(reader.fieldnames) != required:
        raise EvidenceError("OpenCC inventory header is not the exact contract")
    if len(rows) != 83:
        raise EvidenceError(f"OpenCC inventory must contain exactly 83 rows, found {len(rows)}")
    parsed_source: dict[int, tuple[str, tuple[str, ...]]] = {}
    for line_no, line in enumerate(source_lines, 1):
        fields = line.split()
        if len(fields) < 2 or len(fields[1:]) != len(set(fields[1:])):
            raise EvidenceError(f"OpenCC source line {line_no} is malformed")
        parsed_source[line_no] = (fields[0], tuple(fields[1:]))
    if len(parsed_source) != 65:
        raise EvidenceError(
            f"OpenCC source must contain exactly 65 mapping keys, found {len(parsed_source)}"
        )

    expected_pins = {
        "dictionary_commit": PINNED_RIME_CANTONESE_COMMIT,
        "dictionary_tree": PINNED_RIME_CANTONESE_TREE,
        "dictionary_manifest_sha256": PINNED_DICTIONARY_MANIFEST_SHA256,
        "dictionary_import_tables": ";".join(EXPECTED_IMPORT_TABLES),
        "opencc_sha256": PINNED_OPENCC_SOURCE_SHA256,
    }
    mappings: list[OpenCcMapping] = []
    seen: set[tuple[int, str, str]] = set()
    memberships: set[tuple[str, str]] = set()
    previous: tuple[int, str] | None = None
    represented_lines: set[int] = set()
    for csv_index, row in enumerate(rows, 2):
        if None in row or any(not isinstance(row.get(field), str) for field in required):
            raise EvidenceError(f"OpenCC inventory row {csv_index} is malformed")
        for field, expected in expected_pins.items():
            if row[field] != expected:
                raise EvidenceError(
                    f"OpenCC inventory row {csv_index} {field} does not match provenance"
                )
        try:
            source_line = int(row["opencc_line"])
        except ValueError as error:
            raise EvidenceError(f"OpenCC inventory row {csv_index} line is invalid") from error
        if source_line not in parsed_source:
            raise EvidenceError(f"OpenCC inventory row {csv_index} line is out of range")
        source_key, source_outputs = parsed_source[source_line]
        outputs = tuple(row["outputs"].split())
        if row["key"] != source_key or outputs != source_outputs:
            raise EvidenceError(f"OpenCC inventory row {csv_index} mapping/order mismatch")
        if tuple(row["siblings"].split()) != outputs:
            raise EvidenceError(f"OpenCC inventory row {csv_index} sibling/order mismatch")
        code = row["code"]
        if CODE_RE.fullmatch(code) is None:
            raise EvidenceError(f"OpenCC inventory row {csv_index} code is invalid")
        location_surfaces = [
            item.split("@", 1)[0]
            for item in row["locations"].split(";")
            if "@" in item
        ]
        if location_surfaces != list(outputs):
            raise EvidenceError(
                f"OpenCC inventory row {csv_index} locations must preserve output order"
            )
        identity = (source_line, source_key, code)
        if identity in seen:
            raise EvidenceError(f"OpenCC inventory duplicates mapping {identity!r}")
        seen.add(identity)
        sort_key = (source_line, code)
        if previous is not None and sort_key < previous:
            raise EvidenceError("OpenCC inventory rows are not deterministically sorted")
        previous = sort_key
        for output in outputs:
            membership = (output, code)
            if membership in memberships:
                raise EvidenceError(f"OpenCC inventory duplicates membership {membership!r}")
            memberships.add(membership)
        represented_lines.add(source_line)
        mappings.append(OpenCcMapping(source_line, source_key, outputs, code, row["locations"]))
    if len(represented_lines) != 64:
        raise EvidenceError(
            f"OpenCC inventory must reconcile 64 of 65 mapping keys, found {len(represented_lines)}"
        )
    unrepresented = sorted(set(parsed_source) - represented_lines)
    if len(unrepresented) != 1:
        raise EvidenceError("OpenCC inventory must leave exactly one no-same-code mapping key")
    missing_line = unrepresented[0]
    missing_key, missing_outputs = parsed_source[missing_line]
    return tuple(mappings), {
        "inventory_rows": len(mappings),
        "source_mapping_keys": len(parsed_source),
        "represented_mapping_keys": len(represented_lines),
        "unrepresented_mapping_keys": 1,
        "unrepresented_mapping": {
            "source_line": missing_line,
            "key": missing_key,
            "outputs": list(missing_outputs),
            "reason": "no exact-code sibling group in pinned dictionary inventory",
        },
        "mapping_mismatches": 0,
        "order_mismatches": 0,
    }


def validate_opencc_fixture(document: Any, manifest: Any, fixture_sha256: str) -> dict[str, Any]:
    if not isinstance(document, dict) or not isinstance(manifest, dict):
        raise EvidenceError("OpenCC oracle fixture/manifest must be objects")
    if document.get("fixture_version") != 1 or document.get("milestone") != "M59 Increment 4c":
        raise EvidenceError("OpenCC oracle fixture identity is invalid")
    oracle = document.get("oracle")
    tools = document.get("tools")
    data = document.get("data")
    capture = document.get("capture")
    provenance = document.get("provenance")
    if not all(isinstance(value, dict) for value in (oracle, tools, data, capture, provenance)):
        raise EvidenceError("OpenCC oracle fixture provenance is incomplete")
    if (
        oracle.get("engine") != "rime/librime"
        or oracle.get("engine_tag") != "1.17.0"
        or oracle.get("engine_commit") != PINNED_LIBRIME_COMMIT
    ):
        raise EvidenceError("OpenCC oracle fixture does not bind pinned librime")
    dictionary = data.get("opencc_dictionary")
    if not isinstance(dictionary, dict):
        raise EvidenceError("OpenCC oracle fixture dictionary provenance is missing")
    expected_dictionary = {
        "repository_commit": PINNED_RIME_CANTONESE_COMMIT,
        "repository_tree": PINNED_RIME_CANTONESE_TREE,
        "dictionary_manifest_sha256": PINNED_DICTIONARY_MANIFEST_SHA256,
        "sha256": PINNED_OPENCC_SOURCE_SHA256,
        "staged_byte_identical": True,
    }
    for field, expected in expected_dictionary.items():
        if dictionary.get(field) != expected:
            raise EvidenceError(f"OpenCC oracle fixture dictionary {field} is invalid")
    capture_tool = tools.get("capture_script")
    probe = tools.get("oracle_probe")
    if not isinstance(capture_tool, dict) or not isinstance(probe, dict):
        raise EvidenceError("OpenCC oracle fixture tool provenance is missing")
    if capture_tool.get("sha256") != PINNED_OPENCC_CAPTURE_SCRIPT_SHA256:
        raise EvidenceError("OpenCC oracle fixture capture script hash is invalid")
    if probe.get("sha256") != PINNED_CAPTURE_PROBE_SHA256:
        raise EvidenceError("OpenCC oracle fixture probe hash is invalid")
    if capture.get("inputs") != ["a", "b", "c", "d", "e"]:
        raise EvidenceError("OpenCC oracle fixture inputs are invalid")
    if capture.get("filter_chain") != ["simplifier", "uniquifier"]:
        raise EvidenceError("OpenCC oracle fixture filter chain is invalid")
    if capture.get("captured_all_pages_required") is not True:
        raise EvidenceError("OpenCC oracle fixture must require all pages")
    cases = document.get("cases")
    if not isinstance(cases, list) or [case.get("input") for case in cases if isinstance(case, dict)] != ["a", "b", "c", "d", "e"]:
        raise EvidenceError("OpenCC oracle fixture cases are missing/duplicate/out of order")
    observed: list[list[str]] = []
    for index, raw_case in enumerate(cases):
        case = _parse_case(raw_case, f"OpenCC fixture.cases[{index}]")
        observed.append([candidate.text for candidate in case.candidates])
    if observed != [["秘", "祕"], ["粽", "糉", "糭"], ["秘粽"], ["只", "衹"], ["甲乙"]]:
        raise EvidenceError("OpenCC oracle fixture ConvertWord observations changed")

    if (
        manifest.get("fixture_family") != "upstream-core"
        or not isinstance(manifest.get("oracle"), dict)
        or manifest["oracle"].get("engine_commit") != PINNED_LIBRIME_COMMIT
    ):
        raise EvidenceError("oracle manifest identity is invalid")
    files = manifest.get("files")
    if not isinstance(files, list):
        raise EvidenceError("oracle manifest files must be an array")
    rows = [
        row
        for row in files
        if isinstance(row, dict) and row.get("path") == "m59-opencc-convert-word.json"
    ]
    if len(rows) != 1:
        raise EvidenceError("oracle manifest must contain exactly one OpenCC fixture row")
    row = rows[0]
    expected_manifest_row = {
        "milestone": "M59 Increment 4c",
        "capture_date": "2026-07-12",
        "sha256": fixture_sha256,
        "source_row_policy": "m59_minimal_opencc_convert_word_and_default_segmentation_oracle",
    }
    for field, expected in expected_manifest_row.items():
        if row.get(field) != expected:
            raise EvidenceError(f"oracle manifest OpenCC row {field} is invalid")
    return {
        "fixture_cases": 5,
        "whole_word_one_to_many": True,
        "stable_dedup": True,
        "partial_segmentation_default_only": True,
        "pass_through_control": True,
        "manifest_rows": 1,
    }


def validate_capture_provenance(
    oracle_document: Any,
    actual_document: Any,
    *,
    oracle_sha256: str,
    expected_yune_commit: str,
    expected_yune_tree: str,
    expected_yune_dll_sha256: str,
) -> dict[str, Any]:
    if not isinstance(oracle_document, dict) or not isinstance(actual_document, dict):
        raise EvidenceError("oracle and actual captures must be objects")
    oracle_capture = oracle_document.get("capture")
    oracle = oracle_document.get("oracle")
    schema = oracle_document.get("schema")
    if not all(isinstance(value, dict) for value in (oracle_capture, oracle, schema)):
        raise EvidenceError("Lane A oracle provenance is incomplete")
    if (
        oracle_document.get("canonical") is not True
        or oracle_capture.get("engine") != "rime/librime"
        or oracle_capture.get("version") != "1.17.0"
        or oracle_capture.get("librime_commit") != PINNED_LIBRIME_COMMIT
        or oracle.get("commit") != PINNED_LIBRIME_COMMIT
        or schema.get("source_repo") != "rime/rime-cantonese"
        or schema.get("source_commit") != PINNED_RIME_CANTONESE_COMMIT
    ):
        raise EvidenceError("Lane A oracle does not bind the canonical pinned lane")
    if oracle_capture.get("inputs") != list(CANONICAL_INPUTS):
        raise EvidenceError("Lane A oracle capture inputs are not the canonical 13")
    if oracle_capture.get("captured_all_pages") is not True:
        raise EvidenceError("Lane A oracle provenance must require all pages")

    actual = actual_document.get("capture")
    if not isinstance(actual, dict):
        raise EvidenceError("actual capture provenance is missing")
    expected_yune_commit = _expected_object_id(expected_yune_commit, "expected Yune commit")
    expected_yune_tree = _expected_object_id(expected_yune_tree, "expected Yune tree")
    expected_yune_dll_sha256 = _expected_sha256(expected_yune_dll_sha256, "expected Yune DLL SHA-256")
    expected_actual = {
        "engine": "yune",
        "source_commit": expected_yune_commit,
        "source_tree": expected_yune_tree,
        "source_clean": True,
        "source_dirty": False,
        "source_status_short": [],
        "schema_id": "jyut6ping3",
        "modules": ["default"],
        "yune_dll_sha256": expected_yune_dll_sha256,
        "probe_sha256": PINNED_CAPTURE_PROBE_SHA256,
        "capture_script_sha256": PINNED_CANDIDATE_CAPTURE_SCRIPT_SHA256,
        "oracle_capture_sha256": oracle_sha256,
        "default_yaml_overlay_sha256": PINNED_DEFAULT_YAML_OVERLAY_SHA256,
        "schema_list_narrowed": True,
        "narrow_schema_list_switch_used": False,
        "schema_list_narrowing_source": "default_yaml_overlay",
        "runtime_options": {
            "ascii_mode": False,
            "full_shape": False,
            "ascii_punct": False,
            "zh_hans": False,
        },
    }
    for field, expected in expected_actual.items():
        if actual.get(field) != expected:
            raise EvidenceError(f"actual capture {field} does not match the canonical lane")
    for field in ("source_shared_tree_sha256", "staged_shared_tree_sha256"):
        _expected_sha256(actual.get(field), f"actual capture {field}")
    effective = actual.get("effective_parameters")
    if not isinstance(effective, dict):
        raise EvidenceError("actual capture effective_parameters are missing")
    expected_effective = {
        "schema_id": "jyut6ping3",
        "inputs": list(CANONICAL_INPUTS),
        "inputs_source": "oracle_cases",
        "schema_list_narrowed": True,
        "narrow_schema_list_switch_used": False,
        "schema_list_narrowing_source": "default_yaml_overlay",
        "runtime_options": expected_actual["runtime_options"],
        "expected_yune_dll_sha256": expected_yune_dll_sha256,
        "allow_dirty": False,
        "keep_work_root": False,
    }
    for field, expected in expected_effective.items():
        if effective.get(field) != expected:
            raise EvidenceError(f"actual effective_parameters.{field} is invalid")
    return {
        "oracle_engine_commit": PINNED_LIBRIME_COMMIT,
        "oracle_schema_commit": PINNED_RIME_CANTONESE_COMMIT,
        "actual_source_commit": expected_yune_commit,
        "actual_source_tree": expected_yune_tree,
        "actual_yune_dll_sha256": expected_yune_dll_sha256,
        "actual_source_clean": True,
        "source_shared_tree_sha256": actual["source_shared_tree_sha256"],
        "staged_shared_tree_sha256": actual["staged_shared_tree_sha256"],
    }


def _load_comparator() -> ModuleType:
    path = REPO_ROOT / "scripts/compare-candidate-order.py"
    if _file_sha256(path) != PINNED_COMPARATOR_SHA256:
        raise EvidenceError("checked-in comparator bytes no longer match the pin")
    spec = importlib.util.spec_from_file_location("m59_compare_candidate_order", path)
    if spec is None or spec.loader is None:
        raise EvidenceError("cannot load pinned comparator module")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except Exception as error:  # pragma: no cover - defensive loader boundary
        raise EvidenceError(f"cannot load pinned comparator module: {error}") from error
    return module


def validate_strict_comparator(
    document: Any,
    oracle_document: Any,
    actual_document: Any,
    oracle_sha256: str,
    actual_sha256: str,
) -> dict[str, Any]:
    if not isinstance(document, dict):
        raise EvidenceError("strict comparator must be an object")
    expected_top = {
        "tool",
        "tool_version",
        "tool_sha256",
        "policy",
        "inputs",
        "all_accepted",
        "cases",
        "provenance",
    }
    if set(document) != expected_top:
        raise EvidenceError("strict comparator top-level fields are not the exact contract")
    if (
        document.get("tool") != "compare-candidate-order.py"
        or document.get("tool_version") != PINNED_COMPARATOR_VERSION
        or document.get("tool_sha256") != PINNED_COMPARATOR_SHA256
        or document.get("policy") != "exact"
        or document.get("inputs") != list(CANONICAL_INPUTS)
    ):
        raise EvidenceError("strict comparator identity/policy/input scope is invalid")
    provenance = document.get("provenance")
    if not isinstance(provenance, dict) or provenance.get("exceptions") is not None:
        raise EvidenceError("strict comparator must not carry an exception policy")
    if provenance.get("oracle", {}).get("sha256") != oracle_sha256:
        raise EvidenceError("strict comparator oracle hash does not match the supplied oracle")
    if provenance.get("actual", {}).get("sha256") != actual_sha256:
        raise EvidenceError("strict comparator actual hash does not match the supplied capture")
    if (
        provenance.get("tool_version") != PINNED_COMPARATOR_VERSION
        or provenance.get("tool_sha256") != PINNED_COMPARATOR_SHA256
    ):
        raise EvidenceError("strict comparator embedded tool provenance is invalid")

    comparator = _load_comparator()
    try:
        recomputed = comparator.compare_documents(
            oracle_document,
            actual_document,
            policy="exact",
            selected_inputs=CANONICAL_INPUTS,
            exception_policy=None,
        )
    except Exception as error:
        raise EvidenceError(f"pinned comparator rejected supplied captures: {error}") from error
    for field in ("tool", "tool_version", "tool_sha256", "policy", "inputs", "all_accepted", "cases"):
        if document.get(field) != recomputed.get(field):
            raise EvidenceError(f"strict comparator {field} differs from exact recomputation")
    if document.get("all_accepted") is not True:
        raise EvidenceError("strict comparator must be exact all_accepted for all 13 inputs")
    cases = document.get("cases")
    if not isinstance(cases, list) or len(cases) != len(CANONICAL_INPUTS):
        raise EvidenceError("strict comparator must contain exactly 13 cases")
    for input_text, case in zip(CANONICAL_INPUTS, cases):
        if not isinstance(case, dict) or case.get("input") != input_text:
            raise EvidenceError("strict comparator case order is invalid")
        required_pass = {
            "verdict": "pass",
            "raw_first_mismatch_index": None,
            "order_matches_after_signed_exceptions": True,
            "missing_count": 0,
            "extra_count": 0,
            "diff_opcodes": [],
            "failure_classes": [],
            "accepted_exceptions": [],
            "used_replacements": [],
            "used_tail": None,
        }
        for field, expected in required_pass.items():
            if case.get(field) != expected:
                raise EvidenceError(
                    f"strict comparator {input_text}.{field} is not exact-pass"
                )
    return {
        "tool": "compare-candidate-order.py",
        "tool_version": PINNED_COMPARATOR_VERSION,
        "tool_sha256": PINNED_COMPARATOR_SHA256,
        "policy": "exact",
        "inputs": list(CANONICAL_INPUTS),
        "all_accepted": True,
        "exception_policy_present": False,
    }


def _page_projection(case: CaptureCase) -> list[dict[str, Any]]:
    return [
        {
            "page_no": page["page_no"],
            "page_size": page["page_size"],
            "is_last_page": page["is_last_page"],
            "candidates": [
                {
                    "index": candidate["index"],
                    "global_index": candidate["global_index"],
                    "text": candidate["text"],
                }
                for candidate in page["candidates"]
            ],
        }
        for page in case.pages
    ]


def _opencc_occurrences(
    oracle_cases: dict[str, CaptureCase],
    actual_cases: dict[str, CaptureCase],
    mappings: Sequence[OpenCcMapping],
) -> tuple[list[dict[str, Any]], set[tuple[int, str, str]]]:
    occurrences: list[dict[str, Any]] = []
    applicable: set[tuple[int, str, str]] = set()
    for mapping in mappings:
        outputs = mapping.outputs
        for input_text in CANONICAL_INPUTS:
            oracle_rows = oracle_cases[input_text].candidates
            actual_rows = actual_cases[input_text].candidates
            matched_indices = {
                index
                for index, candidate in enumerate(oracle_rows)
                if candidate.text in outputs and mapping.code in _candidate_codes(candidate.comment)
            }
            if not matched_indices:
                continue
            starts: list[int] = []
            consumed: set[int] = set()
            for index in sorted(matched_indices):
                if index in consumed or oracle_rows[index].text != outputs[0]:
                    continue
                end = index + len(outputs)
                if end > len(oracle_rows):
                    continue
                oracle_slice = oracle_rows[index:end]
                if tuple(row.text for row in oracle_slice) != outputs:
                    continue
                if any(mapping.code not in _candidate_codes(row.comment) for row in oracle_slice):
                    continue
                actual_slice = actual_rows[index:end]
                if tuple(row.text for row in actual_slice) != outputs:
                    raise EvidenceError(
                        f"OpenCC mapping {mapping.identity!r} changed text/order/position "
                        f"for {input_text!r} at {index}"
                    )
                if any(mapping.code not in _candidate_codes(row.comment) for row in actual_slice):
                    raise EvidenceError(
                        f"OpenCC mapping {mapping.identity!r} lost code provenance "
                        f"for {input_text!r} at {index}"
                    )
                positions = list(range(index, end))
                starts.append(index)
                consumed.update(positions)
                occurrences.append(
                    {
                        "input": input_text,
                        "inventory_identity": {
                            "source_line": mapping.source_line,
                            "key": mapping.key,
                            "code": mapping.code,
                        },
                        "outputs": list(outputs),
                        "positions": positions,
                        "oracle_text_order": list(outputs),
                        "actual_text_order": list(outputs),
                        "exact_text_order_position": True,
                    }
                )
            if consumed != matched_indices or not starts:
                raise EvidenceError(
                    f"OpenCC mapping {mapping.identity!r} has an incomplete/noncontiguous "
                    f"oracle occurrence for {input_text!r}: matched={sorted(matched_indices)}, "
                    f"consumed={sorted(consumed)}"
                )
            applicable.add(mapping.identity)
    return occurrences, applicable


def _comment_comparison_summary(
    oracle_cases: dict[str, CaptureCase],
    actual_cases: dict[str, CaptureCase],
) -> dict[str, Any]:
    """Describe comment drift without widening D-48's acceptance surface."""

    mismatches: list[list[Any]] = []
    affected_inputs: list[str] = []
    total_candidates = 0
    for input_text in CANONICAL_INPUTS:
        oracle_rows = oracle_cases[input_text].candidates
        actual_rows = actual_cases[input_text].candidates
        if len(oracle_rows) != len(actual_rows):
            raise EvidenceError(
                f"{input_text!r} comment comparison requires equal candidate counts"
            )
        input_affected = False
        for index, (oracle, actual) in enumerate(zip(oracle_rows, actual_rows)):
            total_candidates += 1
            if oracle.comment == actual.comment:
                continue
            mismatches.append(
                [input_text, index, oracle.comment, actual.comment]
            )
            input_affected = True
        if input_affected:
            affected_inputs.append(input_text)
    encoded = json.dumps(
        mismatches,
        ensure_ascii=False,
        separators=(",", ":"),
    ).encode("utf-8")
    return {
        "acceptance_gating": False,
        "acceptance_scope": (
            "D-48 Lane A accepts candidate text/order/position; canonical comment "
            "byte identity is not claimed. OpenCC-owned rows still require code "
            "provenance independently."
        ),
        "total_candidates_compared": total_candidates,
        "mismatch_count": len(mismatches),
        "affected_case_count": len(affected_inputs),
        "affected_inputs": affected_inputs,
        "ordered_mismatch_tuple_fields": [
            "input",
            "candidate_index",
            "oracle_comment",
            "actual_comment",
        ],
        "ordered_mismatch_tuple_encoding": (
            "UTF-8 compact JSON array, ensure_ascii=false, no terminal newline"
        ),
        "ordered_mismatch_tuples_sha256": hashlib.sha256(encoded).hexdigest(),
    }


def classify_documents(
    oracle_cases: dict[str, CaptureCase],
    actual_cases: dict[str, CaptureCase],
    mappings: Sequence[OpenCcMapping],
    inventory_summary: dict[str, Any],
    strict_summary: dict[str, Any],
    fixture_summary: dict[str, Any],
) -> dict[str, Any]:
    cases: list[dict[str, Any]] = []
    for input_text in CANONICAL_INPUTS:
        oracle = oracle_cases[input_text]
        actual = actual_cases[input_text]
        if tuple(row.text for row in oracle.candidates) != tuple(
            row.text for row in actual.candidates
        ):
            raise EvidenceError(f"{input_text!r} candidate text/order is not exact")
        if oracle.control != actual.control:
            differing = [
                field
                for field in CONTROL_FIELDS
                if oracle.control.get(field) != actual.control.get(field)
            ]
            raise EvidenceError(f"{input_text!r} control fields differ: {differing}")
        if _page_projection(oracle) != _page_projection(actual):
            raise EvidenceError(f"{input_text!r} page text/order/position is not exact")
        cases.append(
            {
                "input": input_text,
                "oracle_count": len(oracle.candidates),
                "actual_count": len(actual.candidates),
                "page_size": oracle.page_size,
                "page_count": len(oracle.pages),
                "termination_reason": "last_page",
                "preedit": oracle.preedit,
                "commit_text_preview": oracle.commit_text_preview,
                "exact_count_pages_termination_preedit_commit": True,
                "verdict": "pass",
            }
        )
    comment_summary = _comment_comparison_summary(oracle_cases, actual_cases)
    occurrences, applicable = _opencc_occurrences(oracle_cases, actual_cases, mappings)
    if len(applicable) != 5 or len(occurrences) != 14:
        raise EvidenceError(
            "captured oracle OpenCC applicability changed: expected 5 mappings/14 "
            f"occurrences, found {len(applicable)}/{len(occurrences)}"
        )
    applicable_rows = sum(len(row["outputs"]) for row in occurrences)
    if applicable_rows != 28:
        raise EvidenceError(f"expected 28 inventory-owned captured rows, found {applicable_rows}")
    return {
        "schema_version": 1,
        "milestone": "M59 Increment 4c",
        "classification_complete": True,
        "verdict": "pass",
        "policy": {
            "candidate_order": "strict-exact",
            "opencc_inventory": "complete-pinned-source-reconciliation",
            "exceptions": None,
            "beyond_oracle_depth": False,
            "opencc_residual": None,
        },
        "summary": {
            "canonical_inputs": len(CANONICAL_INPUTS),
            "exact_cases": len(cases),
            "oracle_candidates": sum(row["oracle_count"] for row in cases),
            "actual_candidates": sum(row["actual_count"] for row in cases),
            "opencc_inventory_rows": inventory_summary["inventory_rows"],
            "opencc_source_mapping_keys": inventory_summary["source_mapping_keys"],
            "opencc_inventory_mapping_keys": inventory_summary["represented_mapping_keys"],
            "applicable_inventory_mappings": len(applicable),
            "applicable_inventory_occurrences": len(occurrences),
            "applicable_inventory_rows": applicable_rows,
            "mapping_mismatches": 0,
            "order_mismatches": 0,
            "position_mismatches": 0,
            "opencc_residuals": 0,
            "exceptions": 0,
            "beyond_oracle_depth": 0,
            "comment_mismatches_non_gating": comment_summary["mismatch_count"],
        },
        "strict_comparator": strict_summary,
        "comment_field_comparison": comment_summary,
        "opencc_oracle_fixture": fixture_summary,
        "inventory_reconciliation": inventory_summary,
        "applicable_inventory_occurrences": occurrences,
        "cases": cases,
    }


def _write_json_create_new(output: Path, result: dict[str, Any]) -> None:
    text = json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary: Path | None = None
    try:
        descriptor, temporary_name = tempfile.mkstemp(
            prefix=f".{output.name}.tmp.", dir=output.parent
        )
        temporary = Path(temporary_name)
        with os.fdopen(descriptor, "wb") as handle:
            handle.write(text.encode("utf-8"))
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, output)
        except FileExistsError as error:
            raise EvidenceError(
                f"--output appeared during publication; create-new policy forbids overwrite: {output}"
            ) from error
        except OSError as error:
            raise EvidenceError(f"cannot atomically publish output {output}: {error}") from error
    finally:
        if temporary is not None:
            try:
                temporary.unlink(missing_ok=True)
            except OSError:
                pass


def write_json_create_new(output: Path, result: dict[str, Any]) -> None:
    if output.exists() or output.is_symlink():
        raise EvidenceError(f"--output already exists; create-new policy forbids overwrite: {output}")
    _write_json_create_new(output, result)


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
    parser.add_argument("--opencc-oracle-fixture", required=True, type=Path)
    parser.add_argument("--expected-opencc-oracle-fixture-sha256", required=True)
    parser.add_argument("--oracle-manifest", required=True, type=Path)
    parser.add_argument("--expected-oracle-manifest-sha256", required=True)
    parser.add_argument("--expected-yune-commit", required=True)
    parser.add_argument("--expected-yune-tree", required=True)
    parser.add_argument("--expected-yune-dll-sha256", required=True)
    parser.add_argument("--output", required=True, type=Path)
    return parser


def _validate_pinned_cli_contract(args: argparse.Namespace) -> None:
    expected_pins = {
        "--expected-oracle-sha256": (
            args.expected_oracle_sha256,
            PINNED_LANE_A_ORACLE_SHA256,
        ),
        "--expected-opencc-inventory-sha256": (
            args.expected_opencc_inventory_sha256,
            PINNED_OPENCC_INVENTORY_SHA256,
        ),
        "--expected-opencc-source-sha256": (
            args.expected_opencc_source_sha256,
            PINNED_OPENCC_SOURCE_SHA256,
        ),
        "--expected-opencc-oracle-fixture-sha256": (
            args.expected_opencc_oracle_fixture_sha256,
            PINNED_OPENCC_FIXTURE_SHA256,
        ),
        "--expected-oracle-manifest-sha256": (
            args.expected_oracle_manifest_sha256,
            PINNED_ORACLE_MANIFEST_SHA256,
        ),
    }
    for flag, (actual, expected) in expected_pins.items():
        if _expected_sha256(actual, flag) != expected:
            raise EvidenceError(f"{flag} must equal the pinned M59 value {expected}")
    checked_tools = {
        "scripts/compare-candidate-order.py": PINNED_COMPARATOR_SHA256,
        "scripts/inventory-opencc-same-code.ps1": PINNED_INVENTORY_GENERATOR_SHA256,
        "scripts/capture-m59-opencc-convert-word.ps1": PINNED_OPENCC_CAPTURE_SCRIPT_SHA256,
        "scripts/capture-yune-candidate-order.ps1": PINNED_CANDIDATE_CAPTURE_SCRIPT_SHA256,
        "scripts/oracle-rime-probe.cs": PINNED_CAPTURE_PROBE_SHA256,
    }
    for relative, expected in checked_tools.items():
        if _file_sha256(REPO_ROOT / relative) != expected:
            raise EvidenceError(f"checked-in tool bytes changed: {relative}")


def _verify_unchanged(paths: dict[str, Path], hashes: dict[str, str]) -> None:
    for role, path in paths.items():
        if _file_sha256(path) != hashes[role]:
            raise EvidenceError(f"{role} changed during classification")


def main(argv: Sequence[str] | None = None) -> int:
    raw_argv = list(sys.argv[1:] if argv is None else argv)
    args = _parser().parse_args(raw_argv)
    input_paths = {
        "oracle": args.oracle,
        "actual": args.actual,
        "strict_comparator": args.strict_comparator,
        "opencc_inventory": args.opencc_inventory,
        "opencc_source": args.opencc_source,
        "opencc_oracle_fixture": args.opencc_oracle_fixture,
        "oracle_manifest": args.oracle_manifest,
    }
    try:
        _preflight_paths(list(input_paths.items()), args.output)
        _validate_pinned_cli_contract(args)
        expected_hashes = {
            "oracle": _expected_sha256(args.expected_oracle_sha256, "expected oracle SHA-256"),
            "actual": _expected_sha256(args.expected_actual_sha256, "expected actual SHA-256"),
            "strict_comparator": _expected_sha256(
                args.expected_strict_comparator_sha256,
                "expected strict comparator SHA-256",
            ),
            "opencc_inventory": _expected_sha256(
                args.expected_opencc_inventory_sha256,
                "expected OpenCC inventory SHA-256",
            ),
            "opencc_source": _expected_sha256(
                args.expected_opencc_source_sha256,
                "expected OpenCC source SHA-256",
            ),
            "opencc_oracle_fixture": _expected_sha256(
                args.expected_opencc_oracle_fixture_sha256,
                "expected OpenCC oracle fixture SHA-256",
            ),
            "oracle_manifest": _expected_sha256(
                args.expected_oracle_manifest_sha256,
                "expected oracle manifest SHA-256",
            ),
        }
        input_bytes = {role: _read_bytes(path, role) for role, path in input_paths.items()}
        actual_hashes = {role: _sha256_bytes(data) for role, data in input_bytes.items()}
        for role, expected in expected_hashes.items():
            if actual_hashes[role] != expected:
                raise EvidenceError(
                    f"{role} SHA-256 mismatch: expected {expected}, actual {actual_hashes[role]}"
                )
        oracle_document = _load_json_bytes(input_bytes["oracle"], args.oracle, "oracle")
        actual_document = _load_json_bytes(input_bytes["actual"], args.actual, "actual")
        strict_document = _load_json_bytes(
            input_bytes["strict_comparator"], args.strict_comparator, "strict comparator"
        )
        fixture_document = _load_json_bytes(
            input_bytes["opencc_oracle_fixture"],
            args.opencc_oracle_fixture,
            "OpenCC oracle fixture",
        )
        manifest_document = _load_json_bytes(
            input_bytes["oracle_manifest"], args.oracle_manifest, "oracle manifest"
        )
        oracle_cases = parse_capture(oracle_document, "oracle")
        actual_cases = parse_capture(actual_document, "actual")
        capture_identity = validate_capture_provenance(
            oracle_document,
            actual_document,
            oracle_sha256=actual_hashes["oracle"],
            expected_yune_commit=args.expected_yune_commit,
            expected_yune_tree=args.expected_yune_tree,
            expected_yune_dll_sha256=args.expected_yune_dll_sha256,
        )
        strict_summary = validate_strict_comparator(
            strict_document,
            oracle_document,
            actual_document,
            actual_hashes["oracle"],
            actual_hashes["actual"],
        )
        mappings, inventory_summary = parse_opencc_inventory(
            input_bytes["opencc_inventory"], input_bytes["opencc_source"]
        )
        fixture_summary = validate_opencc_fixture(
            fixture_document, manifest_document, actual_hashes["opencc_oracle_fixture"]
        )
        result = classify_documents(
            oracle_cases,
            actual_cases,
            mappings,
            inventory_summary,
            strict_summary,
            fixture_summary,
        )
        result["tool"] = {
            "name": TOOL_NAME,
            "version": TOOL_VERSION,
            "sha256": _file_sha256(Path(__file__)),
        }
        result["provenance"] = {
            role: {
                "path": _logical_path(path, role.replace("_", "-")),
                "sha256": actual_hashes[role],
            }
            for role, path in input_paths.items()
        }
        result["provenance"]["capture_identity"] = capture_identity
        result["provenance"]["effective_argv"] = _logical_argv(raw_argv)
        result["provenance"]["output_policy"] = "utf8-no-bom-lf-create-new-atomic-hardlink"
        _verify_unchanged(input_paths, actual_hashes)
        write_json_create_new(args.output, result)
        return 0
    except EvidenceError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
