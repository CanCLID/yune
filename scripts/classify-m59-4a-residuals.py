#!/usr/bin/env python3
"""Classify the remaining M59 Increment 4a Lane A strict-order residuals.

This tool is deliberately additive.  It consumes (and verifies) the untouched
strict comparator result, which must remain red and carry no exception policy.
It then proves whether the residuals are limited to the predeclared OpenCC 4c
rows plus D-48 class-3 equal-weight inversions.  It does not turn that proof
into a full D-48 acceptance verdict.
"""

from __future__ import annotations

import argparse
import collections
import csv
import hashlib
import json
import math
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from itertools import zip_longest
from pathlib import Path
from typing import Any, Iterable, Sequence


TOOL_NAME = "classify-m59-4a-residuals.py"
TOOL_VERSION = "1"
REPO_ROOT = Path(__file__).resolve().parent.parent
CLASS1_INPUTS = ("being", "beingo", "beixngoxx", "mgoi", "zijiguk")
EXACT_INPUTS = frozenset(("beixngoxx", "mgoi"))
EXPECTED_IMPORT_TABLES = (
    "jyut6ping3.chars",
    "jyut6ping3.words",
    "jyut6ping3.phrase",
    "jyut6ping3.lettered",
    "jyut6ping3.maps",
)
PATH_ARGUMENT_ROLES = {
    "--oracle": "oracle",
    "--actual": "actual",
    "--strict-comparator": "strict-comparator",
    "--opencc-inventory": "opencc-inventory",
    "--source-repository": "source-repository",
    "--dictionary-manifest": "dictionary-manifest",
    "--vocabulary": "vocabulary",
    "--opencc-source": "opencc-source",
    "--output": "output",
}


class EvidenceError(ValueError):
    """An input is malformed, ambiguous, or has invalid provenance."""


class UnownedResidual(ValueError):
    """The captures are valid but carry a residual outside the declared scope."""


@dataclass(frozen=True)
class Candidate:
    text: str
    comment: str


@dataclass(frozen=True)
class CandidateCase:
    input: str
    rows: tuple[Candidate, ...]
    page_size: int
    captured_all_pages: bool
    menu_present: bool
    termination_reason: str


@dataclass(frozen=True)
class SourceWeightRow:
    text: str
    code: str
    weight: float
    table: str
    line: int
    raw_weight: str


@dataclass(frozen=True)
class OpenCcMapping:
    key: str
    outputs: tuple[str, ...]
    code: str
    line: int
    locations: str


@dataclass(frozen=True)
class SourceWeights:
    rows: dict[tuple[str, str], tuple[SourceWeightRow, ...]]
    table_provenance: tuple[dict[str, Any], ...]

    def candidate_weight(self, candidate: Candidate, label: str) -> float:
        codes = tuple(
            code.strip() for code in candidate.comment.split(";") if code.strip()
        )
        if not codes:
            raise EvidenceError(f"{label} has no source code in its comment")
        matches: list[SourceWeightRow] = []
        for code in codes:
            matches.extend(self.rows.get((candidate.text, code), ()))
        if not matches:
            raise EvidenceError(
                f"{label} has no source-weight row for {candidate.text!r} "
                f"and codes {codes!r}"
            )
        return max(row.weight for row in matches)


def _canonical_path_key(path: Path) -> str:
    return os.path.normcase(os.path.realpath(path))


def _is_within(path: Path, root: Path) -> bool:
    try:
        Path(os.path.realpath(path)).relative_to(Path(os.path.realpath(root)))
        return True
    except ValueError:
        return False


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise EvidenceError(f"cannot hash {path}: {error}") from error
    return digest.hexdigest()


def _expected_sha256(value: str, label: str) -> str:
    normalized = value.lower()
    if re.fullmatch(r"[0-9a-f]{64}", normalized) is None:
        raise EvidenceError(f"{label} must be exactly 64 hexadecimal characters")
    return normalized


def _verify_sha256(path: Path, expected: str, label: str) -> str:
    expected = _expected_sha256(expected, f"expected {label} SHA-256")
    actual = _file_sha256(path)
    if actual != expected:
        raise EvidenceError(
            f"{label} SHA-256 mismatch: expected {expected}, actual {actual}"
        )
    return actual


def _load_json(path: Path, label: str) -> Any:
    try:
        with path.open(encoding="utf-8-sig") as handle:
            return json.load(handle)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
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
                    prefix
                    + _logical_path(Path(value[len(prefix) :]), inline_role)
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
    source_repository = dict(inputs).get("--source-repository")
    if source_repository is not None and _is_within(output, source_repository):
        raise EvidenceError("--output must be outside --source-repository")


def _cases_array(document: Any, label: str) -> list[Any]:
    if isinstance(document, list):
        return document
    if isinstance(document, dict) and isinstance(document.get("cases"), list):
        return document["cases"]
    raise EvidenceError(f"{label} must contain a top-level cases array")


def parse_capture(document: Any, label: str) -> dict[str, CandidateCase]:
    cases: dict[str, CandidateCase] = {}
    for index, raw in enumerate(_cases_array(document, label)):
        row_label = f"{label}.cases[{index}]"
        if not isinstance(raw, dict):
            raise EvidenceError(f"{row_label} must be an object")
        input_text = raw.get("input")
        if not isinstance(input_text, str) or not input_text:
            raise EvidenceError(f"{row_label}.input must be a non-empty string")
        if input_text in cases:
            raise EvidenceError(f"{label} contains duplicate case {input_text!r}")
        raw_candidates = raw.get("all_candidates")
        if not isinstance(raw_candidates, list):
            raise EvidenceError(f"{row_label}.all_candidates must be an array")
        candidates: list[Candidate] = []
        seen_texts: set[str] = set()
        for candidate_index, candidate in enumerate(raw_candidates):
            candidate_label = f"{row_label}.all_candidates[{candidate_index}]"
            if not isinstance(candidate, dict):
                raise EvidenceError(f"{candidate_label} must be an object")
            text = candidate.get("text")
            comment = candidate.get("comment", "")
            if comment is None:
                comment = ""
            if not isinstance(text, str) or not text:
                raise EvidenceError(f"{candidate_label}.text must be non-empty")
            if not isinstance(comment, str):
                raise EvidenceError(f"{candidate_label}.comment must be a string")
            if text in seen_texts:
                raise EvidenceError(
                    f"{row_label} has duplicate candidate text {text!r}; "
                    "occurrence weights would be ambiguous"
                )
            seen_texts.add(text)
            candidates.append(Candidate(text=text, comment=comment))
        complete = raw.get("captured_all_pages")
        page_size = raw.get("page_size")
        menu_present = raw.get("menu_present", True)
        termination_reason = raw.get("termination_reason")
        if complete is not True:
            raise EvidenceError(f"{row_label} must be a complete all-pages capture")
        if not isinstance(page_size, int) or isinstance(page_size, bool) or page_size <= 0:
            raise EvidenceError(f"{row_label}.page_size must be a positive integer")
        if menu_present is not True:
            raise EvidenceError(f"{row_label}.menu_present must be true")
        if termination_reason != "last_page":
            raise EvidenceError(f"{row_label}.termination_reason must be last_page")
        cases[input_text] = CandidateCase(
            input=input_text,
            rows=tuple(candidates),
            page_size=page_size,
            captured_all_pages=True,
            menu_present=True,
            termination_reason="last_page",
        )
    if tuple(cases) != CLASS1_INPUTS:
        raise EvidenceError(
            f"{label} inputs must be exactly {CLASS1_INPUTS!r} in that order; "
            f"found {tuple(cases)!r}"
        )
    return cases


def _raw_first_mismatch(oracle: Sequence[str], actual: Sequence[str]) -> int | None:
    sentinel = object()
    for index, (left, right) in enumerate(
        zip_longest(oracle, actual, fillvalue=sentinel)
    ):
        if left != right:
            return index
    return None


def _counter_difference(
    oracle: Sequence[str], actual: Sequence[str]
) -> tuple[list[str], list[str]]:
    oracle_counts = collections.Counter(oracle)
    actual_counts = collections.Counter(actual)
    missing = list((oracle_counts - actual_counts).elements())
    extra = list((actual_counts - oracle_counts).elements())
    return missing, extra


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
    if document.get("policy") != "exact":
        raise EvidenceError("strict comparator policy must remain exact")
    if document.get("inputs") != list(CLASS1_INPUTS):
        raise EvidenceError("strict comparator inputs do not match the 4a fixed set")
    if document.get("all_accepted") is not False:
        raise EvidenceError("strict comparator must remain red (all_accepted=false)")
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
    if not isinstance(raw_cases, list) or len(raw_cases) != len(CLASS1_INPUTS):
        raise EvidenceError("strict comparator cases do not match the fixed input set")
    by_input: dict[str, dict[str, Any]] = {}
    for raw in raw_cases:
        if not isinstance(raw, dict) or not isinstance(raw.get("input"), str):
            raise EvidenceError("strict comparator carries a malformed case")
        if raw["input"] in by_input:
            raise EvidenceError("strict comparator carries a duplicate case")
        by_input[raw["input"]] = raw
    if tuple(by_input) != CLASS1_INPUTS:
        raise EvidenceError("strict comparator case order does not match the fixed set")
    recomputed_verdicts: list[str] = []
    for input_text in CLASS1_INPUTS:
        raw = by_input[input_text]
        oracle = [candidate.text for candidate in oracle_cases[input_text].rows]
        actual = [candidate.text for candidate in actual_cases[input_text].rows]
        missing, extra = _counter_difference(oracle, actual)
        expected_verdict = "pass" if oracle == actual else "fail"
        recomputed_verdicts.append(expected_verdict)
        checks = {
            "oracle_count": len(oracle),
            "actual_count": len(actual),
            "raw_first_mismatch_index": _raw_first_mismatch(oracle, actual),
            "missing_count": len(missing),
            "extra_count": len(extra),
            "verdict": expected_verdict,
        }
        for field, expected in checks.items():
            if raw.get(field) != expected:
                raise EvidenceError(
                    f"strict comparator {input_text}.{field} is {raw.get(field)!r}; "
                    f"expected {expected!r}"
                )
        if raw.get("accepted_exceptions") != []:
            raise EvidenceError(
                f"strict comparator {input_text} must carry no accepted exceptions"
            )
    recomputed_all_accepted = all(
        verdict == "pass" for verdict in recomputed_verdicts
    )
    if document.get("all_accepted") != recomputed_all_accepted:
        raise EvidenceError(
            "strict comparator all_accepted disagrees with recomputed case verdicts"
        )
    if recomputed_all_accepted:
        raise EvidenceError("strict comparator must contain at least one raw-red case")
    return {
        "all_accepted": False,
        "policy": "exact",
        "exception_policy_present": provenance.get("exceptions") is not None,
    }


def parse_dictionary_manifest(text: str) -> tuple[str, str, tuple[str, ...]]:
    dictionary_name: str | None = None
    vocabulary_name: str | None = None
    sort_policy: str | None = None
    import_tables: list[str] = []
    in_header = False
    in_imports = False
    for raw_line in text.splitlines():
        line = raw_line.strip()
        if line == "---":
            in_header = True
            continue
        if line == "...":
            break
        if not in_header or not line or line.startswith("#"):
            continue
        if line.startswith("name:"):
            dictionary_name = line.split(":", 1)[1].strip().strip("\"'")
            in_imports = False
        elif line.startswith("vocabulary:"):
            vocabulary_name = line.split(":", 1)[1].strip().strip("\"'")
            in_imports = False
        elif line.startswith("sort:"):
            sort_policy = line.split(":", 1)[1].strip().strip("\"'")
            in_imports = False
        elif line == "import_tables:":
            in_imports = True
        elif in_imports and line.startswith("-"):
            import_tables.append(line[1:].strip().strip("\"'"))
        elif not raw_line.startswith((" ", "\t")):
            in_imports = False
    if dictionary_name != "jyut6ping3":
        raise EvidenceError("dictionary manifest name must be jyut6ping3")
    if vocabulary_name != "essay-cantonese":
        raise EvidenceError("dictionary manifest vocabulary must be essay-cantonese")
    if sort_policy != "by_weight":
        raise EvidenceError("dictionary manifest sort policy must be by_weight")
    if tuple(import_tables) != EXPECTED_IMPORT_TABLES:
        raise EvidenceError(
            f"dictionary imports must be exactly {EXPECTED_IMPORT_TABLES!r}"
        )
    return dictionary_name, vocabulary_name, tuple(import_tables)


def _parse_float(value: str, label: str) -> float:
    try:
        parsed = float(value)
    except ValueError as error:
        raise EvidenceError(f"{label} is not numeric: {value!r}") from error
    if not math.isfinite(parsed):
        raise EvidenceError(f"{label} must be finite")
    return parsed


def parse_vocabulary(text: str) -> dict[str, float]:
    vocabulary: dict[str, float] = {}
    comments_enabled = True
    for line_number, raw_line in enumerate(text.splitlines(), 1):
        line = raw_line.rstrip()
        if not line:
            continue
        if comments_enabled and line.startswith("#"):
            if line == "# no comment":
                comments_enabled = False
            continue
        fields = line.split("\t")
        if not fields[0]:
            raise EvidenceError(f"vocabulary line {line_number} has no text")
        weight = _parse_float(
            fields[1] if len(fields) > 1 else "0",
            f"vocabulary line {line_number} weight",
        )
        vocabulary[fields[0]] = weight
    if not vocabulary:
        raise EvidenceError("vocabulary has no rows")
    return vocabulary


def _parse_table_rows(
    table_path: Path, table_name: str, vocabulary: dict[str, float]
) -> list[SourceWeightRow]:
    try:
        lines = table_path.read_text(encoding="utf-8-sig").splitlines()
    except (OSError, UnicodeError) as error:
        raise EvidenceError(f"cannot read source table {table_path}: {error}") from error
    rows: list[SourceWeightRow] = []
    in_data = False
    sort_policy: str | None = None
    for line_number, raw_line in enumerate(lines, 1):
        if not in_data:
            stripped = raw_line.strip()
            if stripped.startswith("sort:"):
                sort_policy = stripped.split(":", 1)[1].strip().strip("\"'")
            if raw_line.strip() == "...":
                in_data = True
            continue
        if not raw_line or raw_line.startswith("#"):
            continue
        fields = raw_line.split("\t")
        if not fields[0]:
            raise EvidenceError(f"{table_name} line {line_number} has no text")
        text = fields[0]
        code = fields[1] if len(fields) > 1 else ""
        raw_weight = fields[2].strip() if len(fields) > 2 else ""
        if not raw_weight:
            weight = vocabulary.get(text, 0.0)
        elif raw_weight.endswith("%"):
            percentage = _parse_float(
                raw_weight[:-1].strip(),
                f"{table_name} line {line_number} percentage",
            )
            weight = vocabulary.get(text, 0.0) * percentage / 100.0
        else:
            weight = _parse_float(
                raw_weight, f"{table_name} line {line_number} weight"
            )
        rows.append(
            SourceWeightRow(
                text=text,
                code=code,
                weight=weight,
                table=table_name,
                line=line_number,
                raw_weight=raw_weight,
            )
        )
    if sort_policy != "by_weight":
        raise EvidenceError(f"{table_name} sort policy must be by_weight")
    if not in_data or not rows:
        raise EvidenceError(f"{table_name} has no dictionary data rows")
    return rows


def build_source_weights(
    source_repository: Path,
    import_tables: Sequence[str],
    vocabulary: dict[str, float],
) -> SourceWeights:
    grouped: dict[tuple[str, str], list[SourceWeightRow]] = {}
    provenance: list[dict[str, Any]] = []
    for table_name in import_tables:
        path = source_repository / f"{table_name}.dict.yaml"
        if not _is_within(path, source_repository) or not path.is_file():
            raise EvidenceError(f"source table is missing or escapes repository: {path}")
        rows = _parse_table_rows(path, table_name, vocabulary)
        for row in rows:
            grouped.setdefault((row.text, row.code), []).append(row)
        provenance.append(
            {
                "path": f"external/source-table/{path.name}",
                "sha256": _file_sha256(path),
                "row_count": len(rows),
            }
        )
    return SourceWeights(
        rows={key: tuple(value) for key, value in grouped.items()},
        table_provenance=tuple(provenance),
    )


def parse_opencc_inventory(
    path: Path,
    expected_commit: str,
    expected_tree: str,
    expected_manifest_sha256: str,
    expected_opencc_sha256: str,
    expected_import_tables: Sequence[str],
    opencc_source: Path,
) -> dict[tuple[str, str], OpenCcMapping]:
    try:
        with path.open(encoding="utf-8-sig", newline="") as handle:
            reader = csv.DictReader(handle)
            rows = list(reader)
    except (OSError, UnicodeError, csv.Error) as error:
        raise EvidenceError(f"cannot read OpenCC inventory {path}: {error}") from error
    required = {
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
    }
    if not rows or reader.fieldnames is None or not required.issubset(reader.fieldnames):
        raise EvidenceError("OpenCC inventory header/rows are incomplete")
    expected_imports = ";".join(expected_import_tables)
    try:
        source_lines = opencc_source.read_text(encoding="utf-8-sig").splitlines()
    except (OSError, UnicodeError) as error:
        raise EvidenceError(f"cannot read OpenCC source {opencc_source}: {error}") from error
    mappings: dict[tuple[str, str], OpenCcMapping] = {}
    for index, row in enumerate(rows, 2):
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
                    f"OpenCC inventory row {index} {field} does not match provenance"
                )
        try:
            line_number = int(row["opencc_line"])
        except ValueError as error:
            raise EvidenceError(
                f"OpenCC inventory row {index} has invalid source line"
            ) from error
        if line_number <= 0 or line_number > len(source_lines):
            raise EvidenceError(f"OpenCC inventory row {index} source line is out of range")
        source_fields = source_lines[line_number - 1].split()
        outputs = tuple(row["outputs"].split())
        if not source_fields or source_fields[0] != row["key"]:
            raise EvidenceError(f"OpenCC inventory row {index} key/source mismatch")
        if tuple(source_fields[1:]) != outputs:
            raise EvidenceError(f"OpenCC inventory row {index} output/source mismatch")
        if tuple(row["siblings"].split()) != outputs:
            raise EvidenceError(f"OpenCC inventory row {index} sibling/output mismatch")
        key = (row["key"], row["code"])
        if key in mappings:
            raise EvidenceError(f"OpenCC inventory contains duplicate mapping {key!r}")
        mappings[key] = OpenCcMapping(
            key=row["key"],
            outputs=outputs,
            code=row["code"],
            line=line_number,
            locations=row["locations"],
        )
    return mappings


def _remove_one(rows: Sequence[Candidate], text: str, label: str) -> list[Candidate]:
    result = list(rows)
    positions = [index for index, row in enumerate(result) if row.text == text]
    if len(positions) != 1:
        raise UnownedResidual(
            f"{label} expected exactly one {text!r}; found {len(positions)}"
        )
    result.pop(positions[0])
    return result


def _require_mapping(
    mappings: dict[tuple[str, str], OpenCcMapping],
    key: str,
    code: str,
    outputs: tuple[str, ...],
) -> OpenCcMapping:
    mapping = mappings.get((key, code))
    if mapping is None or mapping.outputs != outputs:
        raise EvidenceError(
            f"OpenCC inventory must carry {key!r}/{code} -> {outputs!r}"
        )
    return mapping


def _normalize_case(
    input_text: str,
    oracle: Sequence[Candidate],
    actual: Sequence[Candidate],
    mappings: dict[tuple[str, str], OpenCcMapping],
) -> tuple[list[Candidate], list[Candidate], dict[str, Any] | None]:
    oracle_texts = [row.text for row in oracle]
    actual_texts = [row.text for row in actual]
    missing, extra = _counter_difference(oracle_texts, actual_texts)
    if input_text in EXACT_INPUTS:
        if oracle_texts != actual_texts:
            raise UnownedResidual(f"{input_text} must remain strict exact")
        return list(oracle), list(actual), None
    if input_text in {"being", "beingo"}:
        mapping = _require_mapping(mappings, "祕", "bei3", ("秘", "祕"))
        if missing != ["祕"] or extra:
            raise UnownedResidual(
                f"{input_text} expected only missing 祕; missing={missing!r}, extra={extra!r}"
            )
        first, second = mapping.outputs
        first_index = oracle_texts.index(first)
        if oracle_texts[first_index : first_index + 2] != [first, second]:
            raise UnownedResidual(
                f"{input_text} oracle does not carry adjacent ordered OpenCC outputs"
            )
        if actual_texts.index(first) != first_index:
            raise UnownedResidual(
                f"{input_text} first OpenCC output moved independently"
            )
        return (
            _remove_one(oracle, second, f"{input_text} oracle"),
            list(actual),
            {
                "class": "4c-opencc-one-to-many-missing-output",
                "mapping_key": mapping.key,
                "mapping_code": mapping.code,
                "ordered_outputs": list(mapping.outputs),
                "normalized_text": second,
                "oracle_index": oracle_texts.index(second),
                "actual_index": None,
                "inventory_line": mapping.line,
                "locations": mapping.locations,
            },
        )
    if input_text == "zijiguk":
        mapping = _require_mapping(mappings, "只", "zi2", ("只", "衹"))
        if missing or extra:
            raise UnownedResidual(
                f"zijiguk expected the same raw set; missing={missing!r}, extra={extra!r}"
            )
        first, second = mapping.outputs
        oracle_first = oracle_texts.index(first)
        oracle_second = oracle_texts.index(second)
        actual_first = actual_texts.index(first)
        actual_second = actual_texts.index(second)
        if oracle_second != oracle_first + 1:
            raise UnownedResidual("zijiguk oracle OpenCC outputs are not adjacent")
        if actual_first != oracle_first or actual_second <= actual_first + 1:
            raise UnownedResidual(
                "zijiguk does not have the declared present-but-late OpenCC residue"
            )
        return (
            _remove_one(oracle, second, "zijiguk oracle"),
            _remove_one(actual, second, "zijiguk actual"),
            {
                "class": "4c-opencc-one-to-many-derived-weight-placement",
                "mapping_key": mapping.key,
                "mapping_code": mapping.code,
                "ordered_outputs": list(mapping.outputs),
                "normalized_text": second,
                "oracle_index": oracle_second,
                "actual_index": actual_second,
                "inventory_line": mapping.line,
                "locations": mapping.locations,
            },
        )
    raise EvidenceError(f"unexpected classifier input {input_text!r}")


def _weight_label(weight: float) -> str:
    return format(weight, ".12g")


def _row_text_hash(rows: Sequence[Candidate]) -> str:
    digest = hashlib.sha256()
    for row in rows:
        digest.update(row.text.encode("utf-8"))
        digest.update(b"\0")
    return digest.hexdigest()


def _inversion_report(
    input_text: str,
    oracle: Sequence[Candidate],
    actual: Sequence[Candidate],
    weights: SourceWeights,
) -> dict[str, Any]:
    oracle_texts = [row.text for row in oracle]
    actual_texts = [row.text for row in actual]
    missing, extra = _counter_difference(oracle_texts, actual_texts)
    if missing or extra:
        raise UnownedResidual(
            f"{input_text} normalized sets differ; missing={missing!r}, extra={extra!r}"
        )
    actual_by_text = {row.text: row for row in actual}
    actual_positions = {row.text: index for index, row in enumerate(actual)}
    buckets: dict[str, dict[str, Any]] = {}
    pair_lines: list[str] = []
    cross_weight: list[dict[str, Any]] = []
    cross_weight_count = 0
    inversion_count = 0
    for left_index, left in enumerate(oracle):
        for right in oracle[left_index + 1 :]:
            if actual_positions[left.text] < actual_positions[right.text]:
                continue
            inversion_count += 1
            oracle_left_weight = weights.candidate_weight(
                left, f"{input_text} oracle {left.text!r}"
            )
            oracle_right_weight = weights.candidate_weight(
                right, f"{input_text} oracle {right.text!r}"
            )
            actual_left_weight = weights.candidate_weight(
                actual_by_text[left.text], f"{input_text} actual {left.text!r}"
            )
            actual_right_weight = weights.candidate_weight(
                actual_by_text[right.text], f"{input_text} actual {right.text!r}"
            )
            equal = (
                math.isclose(
                    oracle_left_weight,
                    oracle_right_weight,
                    rel_tol=0.0,
                    abs_tol=1e-9,
                )
                and math.isclose(
                    oracle_left_weight,
                    actual_left_weight,
                    rel_tol=0.0,
                    abs_tol=1e-9,
                )
                and math.isclose(
                    oracle_right_weight,
                    actual_right_weight,
                    rel_tol=0.0,
                    abs_tol=1e-9,
                )
            )
            if not equal:
                cross_weight_count += 1
                if len(cross_weight) < 20:
                    cross_weight.append(
                        {
                            "oracle_left": left.text,
                            "oracle_right": right.text,
                            "oracle_left_weight": oracle_left_weight,
                            "oracle_right_weight": oracle_right_weight,
                            "actual_left_weight": actual_left_weight,
                            "actual_right_weight": actual_right_weight,
                        }
                    )
                continue
            weight = _weight_label(oracle_left_weight)
            bucket = buckets.setdefault(
                weight, {"weight": oracle_left_weight, "pair_count": 0, "texts": set()}
            )
            bucket["pair_count"] += 1
            bucket["texts"].update((left.text, right.text))
            pair_lines.append(f"{weight}\t{left.text}\t{right.text}\n")
    bucket_rows = []
    for bucket in sorted(buckets.values(), key=lambda item: -item["weight"]):
        texts = sorted(bucket["texts"])
        bucket_rows.append(
            {
                "weight": bucket["weight"],
                "pair_count": bucket["pair_count"],
                "participating_text_count": len(texts),
                "participating_texts": texts,
            }
        )
    digest = hashlib.sha256("".join(pair_lines).encode("utf-8")).hexdigest()
    return {
        "normalized_oracle_rows_sha256": _row_text_hash(oracle),
        "normalized_actual_rows_sha256": _row_text_hash(actual),
        "inversion_count": inversion_count,
        "equal_weight_inversion_count": inversion_count - cross_weight_count,
        "cross_weight_inversion_count": cross_weight_count,
        "cross_weight_examples": cross_weight,
        "equal_weight_inversion_pairs_sha256": digest,
        "equal_weight_buckets": bucket_rows,
    }


def classify_documents(
    oracle_cases: dict[str, CandidateCase],
    actual_cases: dict[str, CandidateCase],
    strict_summary: dict[str, Any],
    mappings: dict[tuple[str, str], OpenCcMapping],
    weights: SourceWeights,
) -> dict[str, Any]:
    case_results: list[dict[str, Any]] = []
    for input_text in CLASS1_INPUTS:
        oracle_case = oracle_cases[input_text]
        actual_case = actual_cases[input_text]
        oracle_texts = [row.text for row in oracle_case.rows]
        actual_texts = [row.text for row in actual_case.rows]
        missing, extra = _counter_difference(oracle_texts, actual_texts)
        base = {
            "input": input_text,
            "raw_oracle_count": len(oracle_texts),
            "raw_actual_count": len(actual_texts),
            "raw_first_mismatch_index": _raw_first_mismatch(
                oracle_texts, actual_texts
            ),
            "raw_missing": missing,
            "raw_extra": extra,
            "raw_strict_verdict": "pass" if oracle_texts == actual_texts else "fail",
        }
        try:
            normalized_oracle, normalized_actual, normalization = _normalize_case(
                input_text,
                oracle_case.rows,
                actual_case.rows,
                mappings,
            )
            inversion = _inversion_report(
                input_text, normalized_oracle, normalized_actual, weights
            )
            cross_weight = inversion["cross_weight_inversion_count"]
            case_verdict = "pass" if cross_weight == 0 else "fail"
            reasons = [] if case_verdict == "pass" else ["cross-weight-inversion"]
            base.update(
                {
                    "normalization": normalization,
                    "normalized_oracle_count": len(normalized_oracle),
                    "normalized_actual_count": len(normalized_actual),
                    **inversion,
                    "classification_reasons": reasons,
                    "classification_verdict": case_verdict,
                }
            )
        except UnownedResidual as error:
            base.update(
                {
                    "normalization": None,
                    "classification_reasons": [str(error)],
                    "classification_verdict": "fail",
                }
            )
        case_results.append(base)
    complete = all(row["classification_verdict"] == "pass" for row in case_results)
    return {
        "scope": {
            "milestone": "M59",
            "increment": "4a",
            "decision": "D-48",
            "classification_only": True,
            "full_d48_acceptance_claimed": False,
            "inputs": list(CLASS1_INPUTS),
            "weight_policy": {
                "dictionary_sort": "by_weight",
                "blank_weight": "exact-term vocabulary weight, otherwise zero",
                "percentage_weight": "exact-term vocabulary weight multiplied by percentage",
                "multiple_matching_rows": "maximum effective weight",
                "tie_comparison": "zero relative tolerance; 1e-9 absolute tolerance",
            },
            "normalization_policy": (
                "only the input-specific ordered OpenCC rows declared by this tool; "
                "no generic text deletion"
            ),
        },
        "raw_comparator": strict_summary,
        "raw_comparator_all_accepted": False,
        "classification_complete": complete,
        "verdict": "pass" if complete else "fail",
        "cases": case_results,
        "summary": {
            "raw_strict_passes": sum(
                row["raw_strict_verdict"] == "pass" for row in case_results
            ),
            "raw_strict_failures": sum(
                row["raw_strict_verdict"] == "fail" for row in case_results
            ),
            "classified_cases": sum(
                row["classification_verdict"] == "pass" for row in case_results
            ),
            "total_inversions_after_opencc_normalization": sum(
                row.get("inversion_count", 0) for row in case_results
            ),
            "cross_weight_inversions": sum(
                row.get("cross_weight_inversion_count", 0) for row in case_results
            ),
            "beyond_oracle_depth_disposition_used": False,
        },
    }


def _git_output(source_repository: Path, *args: str) -> str:
    try:
        completed = subprocess.run(
            ["git", "-C", str(source_repository), *args],
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
    except OSError as error:
        raise EvidenceError(f"cannot run git for source repository: {error}") from error
    if completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise EvidenceError(f"git {' '.join(args)} failed: {detail}")
    return completed.stdout.strip()


def validate_source_repository(
    source_repository: Path, expected_commit: str, expected_tree: str
) -> dict[str, Any]:
    expected_commit = expected_commit.lower()
    expected_tree = expected_tree.lower()
    if re.fullmatch(r"[0-9a-f]{40}", expected_commit) is None:
        raise EvidenceError("expected dictionary commit must be a 40-hex object id")
    if re.fullmatch(r"[0-9a-f]{40}", expected_tree) is None:
        raise EvidenceError("expected dictionary tree must be a 40-hex object id")
    commit = _git_output(source_repository, "rev-parse", "HEAD").lower()
    tree = _git_output(source_repository, "rev-parse", "HEAD^{tree}").lower()
    status = _git_output(
        source_repository, "status", "--short", "--untracked-files=all"
    )
    if commit != expected_commit or tree != expected_tree:
        raise EvidenceError(
            f"source repository identity mismatch: commit={commit}, tree={tree}"
        )
    if status:
        raise EvidenceError("source repository must be clean, including untracked files")
    return {
        "path": "external/source-repository",
        "commit": commit,
        "tree": tree,
        "clean": True,
    }


def _ensure_tracked(source_repository: Path, paths: Iterable[Path]) -> None:
    for path in paths:
        if not _is_within(path, source_repository):
            raise EvidenceError(f"source input escapes source repository: {path}")
        relative = Path(os.path.realpath(path)).relative_to(
            Path(os.path.realpath(source_repository))
        )
        _git_output(
            source_repository,
            "ls-files",
            "--error-unmatch",
            "--",
            relative.as_posix(),
        )


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
    parser.add_argument("--source-repository", required=True, type=Path)
    parser.add_argument("--expected-dictionary-commit", required=True)
    parser.add_argument("--expected-dictionary-tree", required=True)
    parser.add_argument("--dictionary-manifest", required=True, type=Path)
    parser.add_argument("--expected-dictionary-manifest-sha256", required=True)
    parser.add_argument("--vocabulary", required=True, type=Path)
    parser.add_argument("--expected-vocabulary-sha256", required=True)
    parser.add_argument("--opencc-source", required=True, type=Path)
    parser.add_argument("--expected-opencc-source-sha256", required=True)
    parser.add_argument("--output", required=True, type=Path)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    effective_args = list(sys.argv[1:] if argv is None else argv)
    args = _parser().parse_args(effective_args)
    input_paths = (
        ("--tool", Path(__file__).resolve()),
        ("--oracle", args.oracle),
        ("--actual", args.actual),
        ("--strict-comparator", args.strict_comparator),
        ("--opencc-inventory", args.opencc_inventory),
        ("--source-repository", args.source_repository),
        ("--dictionary-manifest", args.dictionary_manifest),
        ("--vocabulary", args.vocabulary),
        ("--opencc-source", args.opencc_source),
    )
    try:
        _preflight_paths(input_paths, args.output)
    except EvidenceError as error:
        print(f"M59 4a residual evidence error: {error}", file=sys.stderr)
        return 2
    try:
        hashes = {
            "oracle": _verify_sha256(
                args.oracle, args.expected_oracle_sha256, "oracle"
            ),
            "actual": _verify_sha256(
                args.actual, args.expected_actual_sha256, "actual"
            ),
            "strict_comparator": _verify_sha256(
                args.strict_comparator,
                args.expected_strict_comparator_sha256,
                "strict comparator",
            ),
            "opencc_inventory": _verify_sha256(
                args.opencc_inventory,
                args.expected_opencc_inventory_sha256,
                "OpenCC inventory",
            ),
            "dictionary_manifest": _verify_sha256(
                args.dictionary_manifest,
                args.expected_dictionary_manifest_sha256,
                "dictionary manifest",
            ),
            "vocabulary": _verify_sha256(
                args.vocabulary,
                args.expected_vocabulary_sha256,
                "vocabulary",
            ),
            "opencc_source": _verify_sha256(
                args.opencc_source,
                args.expected_opencc_source_sha256,
                "OpenCC source",
            ),
        }
        source_identity = validate_source_repository(
            args.source_repository,
            args.expected_dictionary_commit,
            args.expected_dictionary_tree,
        )
        manifest_text = args.dictionary_manifest.read_text(encoding="utf-8-sig")
        _, _, import_tables = parse_dictionary_manifest(manifest_text)
        table_paths = tuple(
            args.source_repository / f"{table}.dict.yaml" for table in import_tables
        )
        _ensure_tracked(
            args.source_repository,
            (
                args.dictionary_manifest,
                args.vocabulary,
                *table_paths,
            ),
        )
        vocabulary = parse_vocabulary(
            args.vocabulary.read_text(encoding="utf-8-sig")
        )
        source_weights = build_source_weights(
            args.source_repository, import_tables, vocabulary
        )
        mappings = parse_opencc_inventory(
            args.opencc_inventory,
            args.expected_dictionary_commit.lower(),
            args.expected_dictionary_tree.lower(),
            hashes["dictionary_manifest"],
            hashes["opencc_source"],
            import_tables,
            args.opencc_source,
        )
        oracle_document = _load_json(args.oracle, "oracle")
        actual_document = _load_json(args.actual, "actual")
        strict_document = _load_json(args.strict_comparator, "strict comparator")
        oracle_cases = parse_capture(oracle_document, "oracle")
        actual_cases = parse_capture(actual_document, "actual")
        strict_summary = validate_strict_comparator(
            strict_document,
            oracle_cases,
            actual_cases,
            hashes["oracle"],
            hashes["actual"],
        )
        result = classify_documents(
            oracle_cases,
            actual_cases,
            strict_summary,
            mappings,
            source_weights,
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
                    },
                    "source_repository": source_identity,
                    "dictionary_manifest": {
                        "path": "external/dictionary-manifest",
                        "sha256": hashes["dictionary_manifest"],
                        "import_tables": list(import_tables),
                    },
                    "vocabulary": {
                        "path": "external/vocabulary",
                        "sha256": hashes["vocabulary"],
                    },
                    "opencc_source": {
                        "path": _logical_path(args.opencc_source, "opencc-source"),
                        "sha256": hashes["opencc_source"],
                    },
                    "source_tables": list(source_weights.table_provenance),
                    "effective_argv": logical_args,
                    "effective_invocation": subprocess.list2cmdline(
                        ["python", logical_tool_path, *logical_args]
                    ),
                },
            }
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
        print(f"M59 4a residual evidence error: {detail}", file=sys.stderr)
        return 2
    return 0 if result["classification_complete"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
