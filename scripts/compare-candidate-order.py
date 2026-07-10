#!/usr/bin/env python3
"""Compare raw paged candidate captures without losing row multiplicity.

The input JSON must contain a top-level ``cases`` array (or be that array).
Each case must carry ``input``, ``all_candidates`` and
``captured_all_pages``. Candidate order is compared row-for-row; duplicate
text rows are deliberately retained.
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
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import date
from pathlib import Path
from typing import Any, Iterable, Sequence


TOOL_VERSION = "4"
TOOL_NAME = "compare-candidate-order.py"


class EvidenceError(ValueError):
    """The evidence shape is incomplete, ambiguous, or inconsistent."""


@dataclass(frozen=True)
class CandidateCase:
    input: str
    rows: tuple[str, ...]
    captured_all_pages: bool
    page_size: int


def _load_json(path: Path) -> Any:
    try:
        with path.open(encoding="utf-8-sig") as handle:
            return json.load(handle)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise EvidenceError(f"cannot read JSON {path}: {error}") from error


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise EvidenceError(f"cannot hash {path}: {error}") from error
    return digest.hexdigest()


def _canonical_path_key(path: Path) -> str:
    return os.path.normcase(os.path.realpath(path))


def _cases_array(document: Any, label: str) -> list[Any]:
    if isinstance(document, list):
        return document
    if isinstance(document, dict) and isinstance(document.get("cases"), list):
        return document["cases"]
    capture = document.get("capture") if isinstance(document, dict) else None
    if isinstance(capture, dict) and isinstance(capture.get("cases"), list):
        return capture["cases"]
    raise EvidenceError(f"{label} must contain a cases array")


def _candidate_text(candidate: Any, label: str) -> str:
    if isinstance(candidate, str):
        return candidate
    if isinstance(candidate, dict) and isinstance(candidate.get("text"), str):
        return candidate["text"]
    raise EvidenceError(f"{label} has a candidate row without string text")


def parse_cases(document: Any, label: str) -> dict[str, CandidateCase]:
    parsed: dict[str, CandidateCase] = {}
    for index, raw_case in enumerate(_cases_array(document, label)):
        case_label = f"{label}.cases[{index}]"
        if not isinstance(raw_case, dict):
            raise EvidenceError(f"{case_label} must be an object")
        input_text = raw_case.get("input")
        if not isinstance(input_text, str) or not input_text:
            raise EvidenceError(f"{case_label}.input must be a non-empty string")
        if input_text in parsed:
            raise EvidenceError(f"{label} contains duplicate input case {input_text!r}")
        candidates = raw_case.get("all_candidates")
        if not isinstance(candidates, list):
            raise EvidenceError(f"{case_label}.all_candidates must be an array")
        complete = raw_case.get("captured_all_pages")
        if not isinstance(complete, bool):
            raise EvidenceError(f"{case_label}.captured_all_pages must be boolean")
        page_size = raw_case.get("page_size")
        if not isinstance(page_size, int) or isinstance(page_size, bool) or page_size <= 0:
            raise EvidenceError(f"{case_label}.page_size must be a positive integer")
        rows_list: list[str] = []
        for candidate_index, candidate in enumerate(candidates):
            candidate_label = f"{case_label}.all_candidates[{candidate_index}]"
            if isinstance(candidate, dict) and "global_index" in candidate:
                global_index = candidate["global_index"]
                if not isinstance(global_index, int) or global_index != candidate_index:
                    raise EvidenceError(
                        f"{candidate_label}.global_index must equal its raw row position"
                    )
            rows_list.append(_candidate_text(candidate, candidate_label))
        rows = tuple(rows_list)
        parsed[input_text] = CandidateCase(input_text, rows, complete, page_size)
    if not parsed:
        raise EvidenceError(f"{label} contains no cases")
    return parsed


def _split_inputs(values: Sequence[str] | None) -> list[str] | None:
    if not values:
        return None
    result: list[str] = []
    for value in values:
        result.extend(part.strip() for part in value.split(",") if part.strip())
    if len(result) != len(set(result)):
        raise EvidenceError("--input contains a duplicate value")
    return result


def _load_exception_policy(document: Any | None) -> dict[str, list[dict[str, Any]]]:
    if document is None:
        return {"replacements": [], "tails": []}
    if not isinstance(document, dict) or document.get("schema_version") != 1:
        raise EvidenceError("exception policy must be an object with schema_version 1")
    if document.get("decision_id") != "D-48":
        raise EvidenceError("exception policy decision_id must be D-48")
    if document.get("owner_signed") is not True:
        raise EvidenceError("exception policy must record owner_signed=true")
    owner_decision_date = document.get("owner_decision_date")
    if not isinstance(owner_decision_date, str) or not owner_decision_date:
        raise EvidenceError("exception policy must record owner_decision_date")
    try:
        parsed_owner_date = date.fromisoformat(owner_decision_date)
    except ValueError as error:
        raise EvidenceError("exception policy owner_decision_date must be YYYY-MM-DD") from error
    if parsed_owner_date.isoformat() != owner_decision_date:
        raise EvidenceError("exception policy owner_decision_date must be canonical YYYY-MM-DD")
    replacements = document.get("allowed_replacements", [])
    if not isinstance(replacements, list):
        raise EvidenceError("exception policy allowed_replacements must be an array")
    normalized_replacements: list[dict[str, Any]] = []
    for index, replacement in enumerate(replacements):
        label = f"allowed_replacements[{index}]"
        if not isinstance(replacement, dict):
            raise EvidenceError(f"{label} must be an object")
        input_text = replacement.get("input")
        start = replacement.get("start")
        oracle = replacement.get("oracle")
        actual = replacement.get("actual")
        reason = replacement.get("reason")
        exception_class = replacement.get("class")
        if not isinstance(input_text, str) or not input_text:
            raise EvidenceError(f"{label}.input must be a non-empty string")
        if not isinstance(start, int) or isinstance(start, bool) or start < 0:
            raise EvidenceError(f"{label}.start must be a non-negative integer")
        if not isinstance(oracle, list) or not all(isinstance(row, str) for row in oracle):
            raise EvidenceError(f"{label}.oracle must be an array of strings")
        if not isinstance(actual, list) or not all(isinstance(row, str) for row in actual):
            raise EvidenceError(f"{label}.actual must be an array of strings")
        if not oracle or len(oracle) != len(actual):
            raise EvidenceError(f"{label} must replace equal-length non-empty row slices")
        if not isinstance(reason, str) or not reason:
            raise EvidenceError(f"{label}.reason must be a non-empty string")
        if not isinstance(exception_class, str) or not exception_class:
            raise EvidenceError(f"{label}.class must be a non-empty string")
        if exception_class != "equal-weight-tie":
            raise EvidenceError(
                f"{label}.class must be the already signed equal-weight-tie class"
            )
        normalized_replacements.append(
            {
                "input": input_text,
                "start": start,
                "oracle": tuple(oracle),
                "actual": tuple(actual),
                "reason": reason,
                "class": exception_class,
            }
        )
    tails = document.get("allowed_tails", [])
    if not isinstance(tails, list):
        raise EvidenceError("exception policy allowed_tails must be an array")
    normalized_tails: list[dict[str, Any]] = []
    seen_tail_inputs: set[str] = set()
    for index, tail in enumerate(tails):
        label = f"allowed_tails[{index}]"
        if not isinstance(tail, dict):
            raise EvidenceError(f"{label} must be an object")
        input_text = tail.get("input")
        start = tail.get("start")
        actual = tail.get("actual")
        reason = tail.get("reason")
        exception_class = tail.get("class")
        tail_owner_signed = tail.get("owner_signed")
        tail_owner_decision_date = tail.get("owner_decision_date")
        if not isinstance(input_text, str) or not input_text:
            raise EvidenceError(f"{label}.input must be a non-empty string")
        if input_text in seen_tail_inputs:
            raise EvidenceError(f"exception policy duplicates a tail for {input_text!r}")
        seen_tail_inputs.add(input_text)
        if not isinstance(start, int) or isinstance(start, bool) or start < 0:
            raise EvidenceError(f"{label}.start must be a non-negative integer")
        if not isinstance(actual, list) or not actual or not all(
            isinstance(row, str) for row in actual
        ):
            raise EvidenceError(f"{label}.actual must be a non-empty array of strings")
        if not isinstance(reason, str) or not reason:
            raise EvidenceError(f"{label}.reason must be a non-empty string")
        if exception_class != "beyond-oracle-depth":
            raise EvidenceError(
                f"{label}.class must be beyond-oracle-depth"
            )
        if tail_owner_signed is not True:
            raise EvidenceError(
                f"{label} must record owner_signed=true for this specific input tail"
            )
        if tail_owner_decision_date != owner_decision_date:
            raise EvidenceError(
                f"{label}.owner_decision_date must match the signed D-48 policy date"
            )
        normalized_tails.append(
            {
                "input": input_text,
                "start": start,
                "actual": tuple(actual),
                "reason": reason,
                "class": exception_class,
                "owner_decision_date": tail_owner_decision_date,
            }
        )
    return {"replacements": normalized_replacements, "tails": normalized_tails}


def _apply_allowed_replacements(
    input_text: str,
    oracle_rows: tuple[str, ...],
    actual_rows: tuple[str, ...],
    replacements: Sequence[dict[str, Any]],
) -> tuple[tuple[str, ...], list[dict[str, Any]]]:
    applicable = sorted(
        (replacement for replacement in replacements if replacement["input"] == input_text),
        key=lambda replacement: replacement["start"],
    )
    previous_end = -1
    for replacement in applicable:
        start = replacement["start"]
        end = start + len(replacement["oracle"])
        if start < previous_end:
            raise EvidenceError(f"exception policy has overlapping replacements for {input_text!r}")
        previous_end = end
        if oracle_rows[start:end] != replacement["oracle"]:
            raise EvidenceError(
                f"exception policy oracle slice no longer matches {input_text!r} at {start}"
            )
        if actual_rows[start:end] != replacement["actual"]:
            raise EvidenceError(
                f"exception policy actual slice no longer matches {input_text!r} at {start}"
            )

    normalized = list(actual_rows)
    used: list[dict[str, Any]] = []
    for replacement in reversed(applicable):
        start = replacement["start"]
        end = start + len(replacement["actual"])
        normalized[start:end] = replacement["oracle"]
        used.append(
            {
                "class": replacement["class"],
                "start": start,
                "reason": replacement["reason"],
                "oracle": list(replacement["oracle"]),
                "actual": list(replacement["actual"]),
            }
        )
    used.reverse()
    return tuple(normalized), used


def _apply_allowed_tail(
    input_text: str,
    oracle_rows: tuple[str, ...],
    actual_rows: tuple[str, ...],
    tails: Sequence[dict[str, Any]],
) -> dict[str, Any] | None:
    applicable = [tail for tail in tails if tail["input"] == input_text]
    if not applicable:
        return None
    tail = applicable[0]
    expected_start = len(oracle_rows)
    if tail["start"] != expected_start:
        raise EvidenceError(
            f"exception policy tail start for {input_text!r} must equal current oracle "
            f"depth {expected_start}, got {tail['start']}"
        )
    if actual_rows[expected_start:] != tail["actual"]:
        raise EvidenceError(
            f"exception policy tail no longer matches {input_text!r} at {expected_start}"
        )
    return {
        "class": tail["class"],
        "start": tail["start"],
        "reason": tail["reason"],
        "actual": list(tail["actual"]),
        "owner_signed": True,
        "owner_decision_date": tail["owner_decision_date"],
    }


def _first_mismatch(expected: Sequence[str], actual: Sequence[str]) -> int | None:
    for index, (expected_row, actual_row) in enumerate(zip(expected, actual)):
        if expected_row != actual_row:
            return index
    if len(expected) != len(actual):
        return min(len(expected), len(actual))
    return None


def _expanded_counter_difference(
    left: collections.Counter[str], right: collections.Counter[str], limit: int = 20
) -> list[str]:
    rows: list[str] = []
    for text in sorted(left):
        rows.extend([text] * max(0, left[text] - right[text]))
        if len(rows) >= limit:
            return rows[:limit]
    return rows


def _row_hash(rows: Iterable[str]) -> str:
    digest = hashlib.sha256()
    for row in rows:
        encoded = row.encode("utf-8")
        digest.update(len(encoded).to_bytes(8, "big"))
        digest.update(encoded)
    return digest.hexdigest()


def _diff_opcodes(expected: Sequence[str], actual: Sequence[str]) -> list[dict[str, Any]]:
    matcher = difflib.SequenceMatcher(a=expected, b=actual, autojunk=False)
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


def compare_documents(
    oracle_document: Any,
    actual_document: Any,
    *,
    policy: str,
    selected_inputs: Sequence[str] | None = None,
    exception_policy: Any | None = None,
) -> dict[str, Any]:
    if policy not in {"exact", "oracle-prefix"}:
        raise EvidenceError(f"unknown comparison policy {policy!r}")
    oracle_cases = parse_cases(oracle_document, "oracle")
    actual_cases = parse_cases(actual_document, "actual")
    exceptions = _load_exception_policy(exception_policy)
    replacements = exceptions["replacements"]
    tails = exceptions["tails"]
    if policy != "oracle-prefix" and tails:
        raise EvidenceError("allowed_tails require the oracle-prefix comparison policy")

    if selected_inputs is None:
        if set(oracle_cases) != set(actual_cases):
            missing = sorted(set(oracle_cases) - set(actual_cases))
            extra = sorted(set(actual_cases) - set(oracle_cases))
            raise EvidenceError(f"capture input sets differ; missing={missing}, extra={extra}")
        inputs = list(oracle_cases)
    else:
        inputs = list(selected_inputs)
        missing_oracle = [value for value in inputs if value not in oracle_cases]
        missing_actual = [value for value in inputs if value not in actual_cases]
        if missing_oracle or missing_actual:
            raise EvidenceError(
                f"selected inputs are missing; oracle={missing_oracle}, actual={missing_actual}"
            )
    exception_inputs = {replacement["input"] for replacement in replacements}
    exception_inputs.update(tail["input"] for tail in tails)
    unknown_exception_inputs = sorted(exception_inputs - set(inputs))
    if unknown_exception_inputs:
        raise EvidenceError(
            f"exception policy references inputs outside comparison scope: {unknown_exception_inputs}"
        )

    results: list[dict[str, Any]] = []
    for input_text in inputs:
        oracle_case = oracle_cases[input_text]
        actual_case = actual_cases[input_text]
        normalized_actual, used_exceptions = _apply_allowed_replacements(
            input_text, oracle_case.rows, actual_case.rows, replacements
        )
        oracle_counter = collections.Counter(oracle_case.rows)
        actual_counter = collections.Counter(actual_case.rows)
        missing_count = sum((oracle_counter - actual_counter).values())
        extra_count = sum((actual_counter - oracle_counter).values())
        raw_first_mismatch = _first_mismatch(oracle_case.rows, actual_case.rows)

        if policy == "exact":
            order_matches = normalized_actual == oracle_case.rows
        else:
            order_matches = (
                len(normalized_actual) >= len(oracle_case.rows)
                and normalized_actual[: len(oracle_case.rows)] == oracle_case.rows
            )

        failure_classes: list[str] = []
        if not oracle_case.captured_all_pages:
            failure_classes.append("oracle-incomplete")
        if not actual_case.captured_all_pages:
            failure_classes.append("actual-incomplete")
        if oracle_case.page_size != actual_case.page_size:
            failure_classes.append("page-size")
        if missing_count:
            failure_classes.append("under-admission")
        if extra_count and policy == "exact":
            failure_classes.append("over-admission")
        if not order_matches:
            failure_classes.append("order")

        accepted_exceptions = [exception["class"] for exception in used_exceptions]
        trailing_extra = (
            policy == "oracle-prefix"
            and order_matches
            and len(actual_case.rows) > len(oracle_case.rows)
        )
        used_tail: dict[str, Any] | None = None
        if trailing_extra:
            used_tail = _apply_allowed_tail(
                input_text, oracle_case.rows, actual_case.rows, tails
            )
            if used_tail is None:
                failure_classes.append("unsigned-beyond-oracle-depth")
            else:
                accepted_exceptions.append(used_tail["class"])
        elif any(tail["input"] == input_text for tail in tails):
            raise EvidenceError(
                f"exception policy declares a beyond-oracle-depth tail for {input_text!r}, "
                "but the current capture has no matching trailing rows"
            )

        accepted = not failure_classes
        if accepted_exceptions and accepted:
            verdict = "signed-exception"
        elif accepted:
            verdict = "pass"
        else:
            verdict = "fail"

        results.append(
            {
                "input": input_text,
                "policy": policy,
                "oracle_count": len(oracle_case.rows),
                "actual_count": len(actual_case.rows),
                "oracle_captured_all_pages": oracle_case.captured_all_pages,
                "actual_captured_all_pages": actual_case.captured_all_pages,
                "page_size": {
                    "oracle": oracle_case.page_size,
                    "actual": actual_case.page_size,
                },
                "raw_first_mismatch_index": raw_first_mismatch,
                "order_matches_after_signed_exceptions": order_matches,
                "missing_count": missing_count,
                "extra_count": extra_count,
                "missing_examples": _expanded_counter_difference(
                    oracle_counter, actual_counter
                ),
                "extra_examples": _expanded_counter_difference(
                    actual_counter, oracle_counter
                ),
                "oracle_rows_sha256": _row_hash(oracle_case.rows),
                "actual_rows_sha256": _row_hash(actual_case.rows),
                "diff_opcodes": _diff_opcodes(oracle_case.rows, actual_case.rows),
                "failure_classes": failure_classes,
                "accepted_exceptions": accepted_exceptions,
                "used_replacements": used_exceptions,
                "used_tail": used_tail,
                "verdict": verdict,
            }
        )

    return {
        "tool": TOOL_NAME,
        "tool_version": TOOL_VERSION,
        "tool_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "policy": policy,
        "inputs": inputs,
        "all_accepted": all(result["verdict"] != "fail" for result in results),
        "cases": results,
    }


def _csv_text(result: dict[str, Any]) -> str:
    fields = [
        "tool_version",
        "tool_sha256",
        "oracle_sha256",
        "actual_sha256",
        "exception_sha256",
        "effective_invocation",
        "input",
        "policy",
        "oracle_count",
        "actual_count",
        "oracle_captured_all_pages",
        "actual_captured_all_pages",
        "raw_first_mismatch_index",
        "missing_count",
        "extra_count",
        "failure_classes",
        "accepted_exceptions",
        "verdict",
    ]
    handle = io.StringIO(newline="")
    writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    provenance = result["provenance"]
    for case in result["cases"]:
        row = {field: case.get(field, "") for field in fields}
        row.update(
            {
                "tool_version": result["tool_version"],
                "tool_sha256": result["tool_sha256"],
                "oracle_sha256": provenance["oracle"]["sha256"],
                "actual_sha256": provenance["actual"]["sha256"],
                "exception_sha256": (
                    provenance["exceptions"]["sha256"]
                    if provenance["exceptions"] is not None
                    else ""
                ),
                "effective_invocation": provenance["effective_invocation"],
            }
        )
        row["failure_classes"] = ";".join(case["failure_classes"])
        row["accepted_exceptions"] = ";".join(case["accepted_exceptions"])
        writer.writerow(row)
    return handle.getvalue()


def _write_temp(path: Path, content: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, raw_temp = tempfile.mkstemp(
        prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
    )
    temp_path = Path(raw_temp)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
    except Exception:
        temp_path.unlink(missing_ok=True)
        raise
    return temp_path


def _remove_final_outputs(paths: Sequence[Path]) -> None:
    errors: list[str] = []
    for path in paths:
        try:
            path.unlink(missing_ok=True)
        except OSError as error:
            errors.append(f"cannot remove final {path}: {error}")
    if errors:
        raise EvidenceError("paired-output cleanup failed: " + "; ".join(errors))


def _invalidate_outputs(
    paths: Sequence[Path], extra_temps: Sequence[Path | None] = ()
) -> None:
    errors: list[str] = []
    candidates: list[tuple[str, Path]] = [("final", path) for path in paths]
    for path in paths:
        if path.parent.is_dir():
            try:
                candidates.extend(
                    ("temporary", stale_temp)
                    for stale_temp in path.parent.glob(f".{path.name}.*.tmp")
                )
            except OSError as error:
                errors.append(f"cannot enumerate temporary outputs for {path}: {error}")
    candidates.extend(
        ("temporary", temp_path)
        for temp_path in extra_temps
        if temp_path is not None
    )
    seen: set[str] = set()
    for kind, candidate in candidates:
        key = _canonical_path_key(candidate)
        if key in seen:
            continue
        seen.add(key)
        try:
            candidate.unlink(missing_ok=True)
        except OSError as error:
            errors.append(f"cannot remove {kind} {candidate}: {error}")
    if errors:
        raise EvidenceError("paired-output invalidation failed: " + "; ".join(errors))


def write_outputs(json_path: Path, csv_path: Path, result: dict[str, Any]) -> None:
    if json_path.resolve() == csv_path.resolve():
        raise EvidenceError("--output-json and --output-csv must be different paths")
    json_text = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    csv_text = _csv_text(result)
    temp_json: Path | None = None
    temp_csv: Path | None = None
    outputs = (json_path, csv_path)
    try:
        temp_json = _write_temp(json_path, json_text)
        temp_csv = _write_temp(csv_path, csv_text)
        _remove_final_outputs(outputs)
        os.replace(temp_json, json_path)
        temp_json = None
        os.replace(temp_csv, csv_path)
        temp_csv = None
    except Exception as error:
        cleanup_error: EvidenceError | None = None
        try:
            _invalidate_outputs(outputs, (temp_json, temp_csv))
        except EvidenceError as invalidation_error:
            cleanup_error = invalidation_error
        detail = f"paired output write failed: {error}"
        if cleanup_error is not None:
            detail += f"; {cleanup_error}"
        raise EvidenceError(detail) from error


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--oracle", required=True, type=Path)
    parser.add_argument("--actual", required=True, type=Path)
    parser.add_argument("--policy", choices=["exact", "oracle-prefix"], required=True)
    parser.add_argument(
        "--input",
        action="append",
        help="Input to compare; repeat or pass comma-separated values. Default: all oracle cases.",
    )
    parser.add_argument("--exceptions", type=Path)
    parser.add_argument("--output-json", required=True, type=Path)
    parser.add_argument("--output-csv", required=True, type=Path)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    effective_args = list(sys.argv[1:] if argv is None else argv)
    args = _parser().parse_args(effective_args)
    outputs = (args.output_json, args.output_csv)
    try:
        oracle = _load_json(args.oracle)
        actual = _load_json(args.actual)
        exceptions = _load_json(args.exceptions) if args.exceptions else None
        result = compare_documents(
            oracle,
            actual,
            policy=args.policy,
            selected_inputs=_split_inputs(args.input),
            exception_policy=exceptions,
        )
        tool_path = Path(__file__).resolve()
        tool_sha256 = _file_sha256(tool_path)
        result["tool_sha256"] = tool_sha256
        result["provenance"] = {
            "oracle": {
                "path": str(args.oracle.resolve()),
                "sha256": _file_sha256(args.oracle),
            },
            "actual": {
                "path": str(args.actual.resolve()),
                "sha256": _file_sha256(args.actual),
            },
            "exceptions": (
                {
                    "path": str(args.exceptions.resolve()),
                    "sha256": _file_sha256(args.exceptions),
                }
                if args.exceptions is not None
                else None
            ),
            "tool_path": str(tool_path),
            "tool_version": TOOL_VERSION,
            "tool_sha256": tool_sha256,
            "effective_argv": effective_args,
            "effective_invocation": subprocess.list2cmdline(
                ["python", str(tool_path), *effective_args]
            ),
        }
        write_outputs(args.output_json, args.output_csv, result)
    except (EvidenceError, OSError) as error:
        cleanup_error: EvidenceError | None = None
        try:
            _invalidate_outputs(outputs)
        except EvidenceError as invalidation_error:
            cleanup_error = invalidation_error
        detail = str(error)
        if cleanup_error is not None:
            detail += f"; {cleanup_error}"
        print(f"candidate-order evidence error: {detail}", file=sys.stderr)
        return 2
    return 0 if result["all_accepted"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
