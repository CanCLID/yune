#!/usr/bin/env python3
"""Aggregate native benchmark runs into an executable median ratchet gate.

Each run directory must contain ``environment.txt``, ``summary-comparison.csv``
and ``threshold-check.csv``; ``external-provenance.txt`` is merged when present.
Track-A latency ratios are read from the underlying summary comparison; all
other metrics are read from the threshold check. This keeps newly signed latency
rows reproducible even when the measurement run was captured before those rows
were installed in the standing threshold file.
"""

from __future__ import annotations

import argparse
import csv
import functools
import hashlib
import io
import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation, ROUND_HALF_UP
from pathlib import Path
from typing import Any, Sequence


PROVENANCE_KEYS = (
    "source_commit",
    "source_tree",
    "yune_git_head",
    "source_clean",
    "allow_dirty",
    "source_content_binding_sha256",
    "measured_yune_dll_sha256",
    "upstream_rime_dll_sha256",
    "upstream_shared_tree_sha256",
    "upstream_build_tree_sha256",
    "product_schema_tree_sha256",
    "native_benchmark_executable_sha256",
    "benchmark_script_sha256",
    "track_a_inputs",
    "track_b_inputs",
    "iterations",
    "session_iterations",
    "key_iterations",
    "deploy_product_before_benchmark",
    "skip_track_b",
)
TOOL_NAME = "aggregate-native-ratchet.py"
TOOL_VERSION = "7"
REQUIRED_RUN_COUNT = 5
REPO_ROOT = Path(__file__).resolve().parents[1]
RUN_FILES = (
    "run-status.txt",
    "environment.txt",
    "external-provenance.txt",
    "summary-comparison.csv",
    "threshold-check.csv",
)


class EvidenceError(ValueError):
    """The benchmark evidence is incomplete or internally inconsistent."""


def _normalize_parser_failures(function):
    @functools.wraps(function)
    def wrapped(*args, **kwargs):
        try:
            return function(*args, **kwargs)
        except EvidenceError:
            raise
        except (csv.Error, UnicodeError, AttributeError, KeyError, TypeError) as error:
            source = args[0] if args else function.__name__
            raise EvidenceError(f"malformed evidence input {source}: {error}") from error

    return wrapped


@dataclass(frozen=True, order=True)
class MetricKey:
    kind: str
    workload: str
    input: str
    metric: str


@dataclass(frozen=True)
class Threshold:
    key: MetricKey
    ceiling: Decimal
    ceiling_text: str
    unit: str


@dataclass(frozen=True)
class RunEvidence:
    path: Path
    environment: dict[str, str]
    observations: dict[MetricKey, Decimal]


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


def _recorded_path(path: Path) -> str:
    resolved = path.resolve()
    try:
        recorded = resolved.relative_to(REPO_ROOT).as_posix()
        return recorded.lower() if os.name == "nt" else recorded
    except ValueError:
        return path.as_posix()


def _recorded_effective_args(effective_args: Sequence[str]) -> list[str]:
    path_options = {"--thresholds", "--run", "--output"}
    recorded: list[str] = []
    expecting_path = False
    for argument in effective_args:
        if expecting_path:
            recorded.append(_recorded_path(Path(argument)))
            expecting_path = False
            continue
        matched_inline = False
        for option in path_options:
            prefix = option + "="
            if argument.startswith(prefix):
                recorded.append(prefix + _recorded_path(Path(argument[len(prefix) :])))
                matched_inline = True
                break
        if matched_inline:
            continue
        recorded.append(argument)
        expecting_path = argument in path_options
    if expecting_path:
        raise EvidenceError("path option is missing its value in effective arguments")
    return recorded


def _protected_input_paths(
    thresholds_path: Path, run_paths: Sequence[Path]
) -> list[tuple[str, Path]]:
    protected = [("thresholds", thresholds_path)]
    for run_number, run_path in enumerate(run_paths, start=1):
        protected.append((f"run {run_number} directory", run_path))
        protected.extend(
            (f"run {run_number} {name}", run_path / name) for name in RUN_FILES
        )
    return protected


def _validate_output_destinations(
    gate_path: Path,
    sidecar_path: Path,
    thresholds_path: Path,
    run_paths: Sequence[Path],
) -> None:
    destinations = (
        ("gate verdict output", gate_path),
        ("provenance sidecar output", sidecar_path),
    )
    destination_keys = {
        label: _canonical_path_key(path) for label, path in destinations
    }
    if len(set(destination_keys.values())) != len(destination_keys):
        raise EvidenceError(
            "gate verdict and provenance sidecar paths must differ"
        )

    protected_by_key: dict[str, tuple[str, Path]] = {}
    for label, path in _protected_input_paths(thresholds_path, run_paths):
        protected_by_key.setdefault(_canonical_path_key(path), (label, path))
    for destination_label, destination_path in destinations:
        destination_key = destination_keys[destination_label]
        protected = protected_by_key.get(destination_key)
        if protected is None:
            for run_number, run_path in enumerate(run_paths, start=1):
                run_key = _canonical_path_key(run_path)
                try:
                    inside_run = os.path.commonpath((destination_key, run_key)) == run_key
                except ValueError:
                    inside_run = False
                if inside_run:
                    raise EvidenceError(
                        f"{destination_label} {destination_path} must not be inside "
                        f"protected run {run_number} directory {run_path}"
                    )
            continue
        protected_label, protected_path = protected
        raise EvidenceError(
            f"{destination_label} {destination_path} collides with protected "
            f"{protected_label} {protected_path}"
        )


def _decimal(value: str, label: str, *, positive: bool = True) -> Decimal:
    try:
        parsed = Decimal(value)
    except (InvalidOperation, ValueError) as error:
        raise EvidenceError(f"{label} is not a decimal: {value!r}") from error
    if not parsed.is_finite() or (positive and parsed <= 0):
        raise EvidenceError(f"{label} must be finite and positive: {value!r}")
    return parsed


def _decimal_text(value: Decimal) -> str:
    text = format(value, "f")
    if "." in text:
        text = text.rstrip("0").rstrip(".")
    return text or "0"


def _required_columns(reader: csv.DictReader, required: set[str], label: str) -> None:
    field_list = reader.fieldnames or []
    if any(not isinstance(field, str) or not field for field in field_list):
        raise EvidenceError(f"{label} has an empty CSV column name")
    if len(field_list) != len(set(field_list)):
        raise EvidenceError(f"{label} has duplicate CSV column names")
    fields = set(field_list)
    missing = sorted(required - fields)
    if missing:
        raise EvidenceError(f"{label} is missing columns: {missing}")


def _csv_cell(row: dict[str, str | None], column: str, label: str) -> str:
    value = row.get(column)
    if not isinstance(value, str):
        raise EvidenceError(f"{label} has a missing {column!r} field")
    return value.strip()


def _csv_rows(reader: csv.DictReader, label: str):
    try:
        for row in reader:
            if None in row:
                raise EvidenceError(f"{label} has a row with fields beyond its CSV header")
            yield row
    except csv.Error as error:
        raise EvidenceError(f"malformed CSV {label}: {error}") from error


@_normalize_parser_failures
def read_thresholds(path: Path) -> list[Threshold]:
    try:
        handle = path.open(encoding="utf-8-sig", newline="")
    except OSError as error:
        raise EvidenceError(f"cannot read thresholds {path}: {error}") from error
    with handle:
        reader = csv.DictReader(handle)
        _required_columns(
            reader,
            {"kind", "workload", "input", "metric", "ceiling", "unit"},
            str(path),
        )
        thresholds: list[Threshold] = []
        seen: set[MetricKey] = set()
        for row_number, row in enumerate(_csv_rows(reader, str(path)), start=2):
            key = MetricKey(
                _csv_cell(row, "kind", f"{path}:{row_number}"),
                _csv_cell(row, "workload", f"{path}:{row_number}"),
                _csv_cell(row, "input", f"{path}:{row_number}"),
                _csv_cell(row, "metric", f"{path}:{row_number}"),
            )
            if not all((key.kind, key.metric)):
                raise EvidenceError(f"{path}:{row_number} has an empty kind or metric")
            if key in seen:
                raise EvidenceError(f"{path}:{row_number} duplicates threshold key {key}")
            seen.add(key)
            ceiling_text = _csv_cell(row, "ceiling", f"{path}:{row_number}")
            thresholds.append(
                Threshold(
                    key,
                    _decimal(ceiling_text, f"{path}:{row_number} ceiling"),
                    ceiling_text,
                    _csv_cell(row, "unit", f"{path}:{row_number}"),
                )
            )
    if not thresholds:
        raise EvidenceError(f"threshold file {path} is empty")
    return thresholds


@_normalize_parser_failures
def _read_key_value_file(path: Path, *, required: bool) -> dict[str, str]:
    if not path.is_file() and not required:
        return {}
    try:
        lines = path.read_text(encoding="utf-8-sig").splitlines()
    except OSError as error:
        raise EvidenceError(f"cannot read environment {path}: {error}") from error
    environment: dict[str, str] = {}
    for line_number, line in enumerate(lines, start=1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if "=" not in line:
            raise EvidenceError(f"{path}:{line_number} is not key=value")
        key, value = line.split("=", 1)
        key = key.strip()
        if not key:
            raise EvidenceError(f"{path}:{line_number} has an empty key")
        if key in environment:
            raise EvidenceError(f"{path}:{line_number} duplicates environment key {key!r}")
        environment[key] = value.strip()
    return environment


def read_environment(run_path: Path) -> dict[str, str]:
    environment = _read_key_value_file(run_path / "environment.txt", required=True)
    external = _read_key_value_file(
        run_path / "external-provenance.txt", required=False
    )
    for key, value in external.items():
        existing = environment.get(key)
        if existing is not None and existing != value:
            raise EvidenceError(
                f"{run_path} provenance files disagree for {key}: {existing!r} != {value!r}"
            )
        environment[key] = value
    missing = [key for key in PROVENANCE_KEYS if not environment.get(key)]
    if missing:
        raise EvidenceError(f"{run_path} is missing strict provenance keys: {missing}")
    if re.fullmatch(r"[0-9a-fA-F]{40}", environment["source_commit"]) is None:
        raise EvidenceError(f"{run_path} source_commit must be 40 hexadecimal characters")
    for key in (
        "source_content_binding_sha256",
        "measured_yune_dll_sha256",
        "upstream_rime_dll_sha256",
        "upstream_shared_tree_sha256",
        "upstream_build_tree_sha256",
        "product_schema_tree_sha256",
        "native_benchmark_executable_sha256",
        "benchmark_script_sha256",
    ):
        if re.fullmatch(r"[0-9a-fA-F]{64}", environment[key]) is None:
            raise EvidenceError(f"{run_path} {key} must be 64 hexadecimal characters")
    yune_git_head = environment["yune_git_head"]
    if re.fullmatch(r"[0-9a-fA-F]{40}", yune_git_head) is None:
        raise EvidenceError(f"{run_path} yune_git_head must be 40 hexadecimal characters")
    if yune_git_head.lower() != environment["source_commit"].lower():
        raise EvidenceError(
            f"{run_path} yune_git_head does not match source_commit: "
            f"{yune_git_head} != {environment['source_commit']}"
        )
    if re.fullmatch(r"[0-9a-fA-F]{40}", environment["source_tree"]) is None:
        raise EvidenceError(f"{run_path} source_tree must be 40 hexadecimal characters")
    if environment["source_clean"] != "True":
        raise EvidenceError(f"{run_path} signed benchmark source must be clean")
    if environment["allow_dirty"] != "False":
        raise EvidenceError(f"{run_path} signed benchmark must not allow dirty source")
    if environment["deploy_product_before_benchmark"] != "True":
        raise EvidenceError(
            f"{run_path} must record deploy_product_before_benchmark=True"
        )
    if environment["skip_track_b"] != "False":
        raise EvidenceError(f"{run_path} must record skip_track_b=False")
    return environment


def validate_run_status(run_path: Path) -> None:
    status = _read_key_value_file(run_path / "run-status.txt", required=True)
    if status.get("status") != "complete":
        raise EvidenceError(
            f"{run_path} benchmark run status must be complete; "
            f"found {status.get('status')!r}"
        )


def _recorded_input_set(value: str, label: str) -> set[str]:
    inputs = [item.strip() for item in value.split(",")]
    if any(not item for item in inputs):
        raise EvidenceError(f"{label} contains an empty input")
    if len(inputs) != len(set(inputs)):
        raise EvidenceError(f"{label} contains duplicate inputs")
    return set(inputs)


def _validate_recorded_input_sets(
    run_path: Path,
    environment: dict[str, str],
    thresholds: Sequence[Threshold],
) -> None:
    expected_track_a = {
        threshold.key.input
        for threshold in thresholds
        if threshold.key.kind == "latency_ratio"
        and threshold.key.workload == "key_sequence_process_with_context"
        and threshold.key.input
    }
    expected_track_b = {
        threshold.key.input
        for threshold in thresholds
        if threshold.key.workload.startswith("track-b-product/")
        and threshold.key.input
    }
    expected_by_key = {
        "track_a_inputs": expected_track_a,
        "track_b_inputs": expected_track_b,
    }
    for key, expected in expected_by_key.items():
        recorded = _recorded_input_set(
            environment[key], f"{run_path} recorded {key}"
        )
        if recorded == expected:
            continue
        raise EvidenceError(
            f"{run_path} recorded {key} differs from the observed threshold set; "
            f"missing={sorted(expected - recorded)}, "
            f"extra={sorted(recorded - expected)}"
        )


@_normalize_parser_failures
def _read_threshold_check(
    path: Path, threshold_map: dict[MetricKey, Threshold]
) -> dict[MetricKey, Decimal]:
    try:
        handle = path.open(encoding="utf-8-sig", newline="")
    except OSError as error:
        raise EvidenceError(f"cannot read threshold check {path}: {error}") from error
    observations: dict[MetricKey, Decimal] = {}
    with handle:
        reader = csv.DictReader(handle)
        _required_columns(
            reader,
            {
                "kind",
                "workload",
                "input",
                "metric",
                "observed",
                "ceiling",
                "unit",
                "status",
            },
            str(path),
        )
        for row_number, row in enumerate(_csv_rows(reader, str(path)), start=2):
            key = MetricKey(
                _csv_cell(row, "kind", f"{path}:{row_number}"),
                _csv_cell(row, "workload", f"{path}:{row_number}"),
                _csv_cell(row, "input", f"{path}:{row_number}"),
                _csv_cell(row, "metric", f"{path}:{row_number}"),
            )
            if key in observations:
                raise EvidenceError(f"{path}:{row_number} duplicates metric key {key}")
            threshold = threshold_map.get(key)
            if threshold is None:
                raise EvidenceError(f"{path}:{row_number} has a row absent from thresholds: {key}")
            observed = _decimal(
                _csv_cell(row, "observed", f"{path}:{row_number}"),
                f"{path}:{row_number} observed",
            )
            row_ceiling = _decimal(
                _csv_cell(row, "ceiling", f"{path}:{row_number}"),
                f"{path}:{row_number} ceiling",
            )
            if row_ceiling != threshold.ceiling:
                raise EvidenceError(
                    f"{path}:{row_number} ceiling drift for {key}: "
                    f"run={row_ceiling}, standing={threshold.ceiling}"
                )
            if _csv_cell(row, "unit", f"{path}:{row_number}") != threshold.unit:
                raise EvidenceError(f"{path}:{row_number} unit drift for {key}")
            expected_status = "pass" if observed <= threshold.ceiling else "fail"
            if _csv_cell(row, "status", f"{path}:{row_number}").lower() != expected_status:
                raise EvidenceError(
                    f"{path}:{row_number} status does not match observed/ceiling for {key}"
                )
            observations[key] = observed
    return observations


@_normalize_parser_failures
def _read_track_a_ratios(
    path: Path, expected_keys: set[MetricKey]
) -> dict[MetricKey, Decimal]:
    try:
        handle = path.open(encoding="utf-8-sig", newline="")
    except OSError as error:
        raise EvidenceError(f"cannot read summary comparison {path}: {error}") from error
    all_observations: dict[MetricKey, Decimal] = {}
    with handle:
        reader = csv.DictReader(handle)
        _required_columns(
            reader,
            {
                "track",
                "workload",
                "input",
                "yune_librime_median_ratio",
            },
            str(path),
        )
        for row_number, row in enumerate(_csv_rows(reader, str(path)), start=2):
            if _csv_cell(row, "track", f"{path}:{row_number}") != "track-a-comparison":
                continue
            key = MetricKey(
                "latency_ratio",
                _csv_cell(row, "workload", f"{path}:{row_number}"),
                _csv_cell(row, "input", f"{path}:{row_number}"),
                "yune_librime_median_ratio",
            )
            if key in all_observations:
                raise EvidenceError(f"{path}:{row_number} duplicates Track-A ratio key {key}")
            all_observations[key] = _decimal(
                _csv_cell(
                    row, "yune_librime_median_ratio", f"{path}:{row_number}"
                ),
                f"{path}:{row_number} Track-A ratio",
            )
    if not all_observations:
        raise EvidenceError(f"{path} contains no Track-A ratios")
    missing = sorted(expected_keys - set(all_observations))
    if missing:
        raise EvidenceError(f"{path} is missing standing Track-A ratios: {missing}")
    expected_key_inputs = {
        key
        for key in expected_keys
        if key.workload == "key_sequence_process_with_context"
    }
    actual_key_inputs = {
        key
        for key in all_observations
        if key.workload == "key_sequence_process_with_context"
    }
    if actual_key_inputs != expected_key_inputs:
        missing_inputs = sorted(expected_key_inputs - actual_key_inputs)
        extra_inputs = sorted(actual_key_inputs - expected_key_inputs)
        raise EvidenceError(
            f"{path} Track-A key-sequence rows differ from standing thresholds; "
            f"missing={missing_inputs}, extra={extra_inputs}"
        )
    return {key: all_observations[key] for key in expected_keys}


def read_run(path: Path, thresholds: Sequence[Threshold]) -> RunEvidence:
    if not path.is_dir():
        raise EvidenceError(f"run directory does not exist: {path}")
    validate_run_status(path)
    threshold_map = {threshold.key: threshold for threshold in thresholds}
    checked = _read_threshold_check(path / "threshold-check.csv", threshold_map)
    expected_ratio_keys = {
        threshold.key for threshold in thresholds if threshold.key.kind == "latency_ratio"
    }
    ratios = _read_track_a_ratios(path / "summary-comparison.csv", expected_ratio_keys)

    observations: dict[MetricKey, Decimal] = {}
    for threshold in thresholds:
        key = threshold.key
        if key.kind == "latency_ratio":
            observed = ratios[key]
            checked_value = checked.get(key)
            if checked_value is not None and checked_value != observed:
                raise EvidenceError(
                    f"{path} threshold-check and summary-comparison disagree for {key}: "
                    f"{checked_value} != {observed}"
                )
            observations[key] = observed
        else:
            if key not in checked:
                raise EvidenceError(f"{path} threshold-check is missing non-ratio row {key}")
            observations[key] = checked[key]

    environment = read_environment(path)
    _validate_recorded_input_sets(path, environment, thresholds)
    return RunEvidence(path, environment, observations)


def _validate_provenance(runs: Sequence[RunEvidence]) -> dict[str, str]:
    baseline = {key: runs[0].environment[key] for key in PROVENANCE_KEYS}
    for run_number, run in enumerate(runs[1:], start=2):
        for key, expected in baseline.items():
            actual = run.environment[key]
            if actual != expected:
                raise EvidenceError(
                    f"run {run_number} provenance mismatch for {key}: "
                    f"{actual!r} != {expected!r}"
                )
    return baseline


def _median(values: Sequence[Decimal]) -> Decimal:
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return ordered[middle]
    return (ordered[middle - 1] + ordered[middle]) / Decimal(2)


def aggregate(
    thresholds: Sequence[Threshold], runs: Sequence[RunEvidence]
) -> tuple[list[dict[str, Any]], dict[str, str]]:
    if len(runs) != REQUIRED_RUN_COUNT:
        raise EvidenceError(
            f"native ratchet requires exactly {REQUIRED_RUN_COUNT} unique runs, got {len(runs)}"
        )
    resolved = [_canonical_path_key(run.path) for run in runs]
    if len(resolved) != len(set(resolved)):
        raise EvidenceError("native ratchet runs must be unique")
    provenance = _validate_provenance(runs)
    rows: list[dict[str, Any]] = []
    for threshold in thresholds:
        values = [run.observations[threshold.key] for run in runs]
        median = _median(values)
        worst = max(values)
        minimum = min(values)
        spread = ((worst - minimum) / minimum * Decimal(100)).quantize(
            Decimal("0.1"), rounding=ROUND_HALF_UP
        )
        row: dict[str, Any] = {
            "kind": threshold.key.kind,
            "workload": threshold.key.workload,
            "input": threshold.key.input,
            "metric": threshold.key.metric,
            "unit": threshold.unit,
        }
        for run_number, value in enumerate(values, start=1):
            row[f"run{run_number}_observed"] = _decimal_text(value)
        row.update(
            {
                "median_observed": _decimal_text(median),
                "worst_observed": _decimal_text(worst),
                "spread_pct": _decimal_text(spread),
                "ceiling": threshold.ceiling_text,
                "individual_failures": sum(value > threshold.ceiling for value in values),
                "verdict": "pass" if median <= threshold.ceiling else "fail",
            }
        )
        rows.append(row)
    return rows, provenance


def _gate_text(rows: Sequence[dict[str, Any]], run_count: int) -> str:
    fields = ["kind", "workload", "input", "metric", "unit"]
    fields.extend(f"run{run_number}_observed" for run_number in range(1, run_count + 1))
    fields.extend(
        [
            "median_observed",
            "worst_observed",
            "spread_pct",
            "ceiling",
            "individual_failures",
            "verdict",
        ]
    )
    handle = io.StringIO(newline="")
    writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return handle.getvalue()


def _sidecar_path(output: Path) -> Path:
    return output.with_name(f"{output.stem}.provenance.json")


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


def _write_output_pair(
    gate_path: Path,
    sidecar_path: Path,
    gate_text: str,
    sidecar: dict[str, Any],
    *,
    thresholds_path: Path,
    run_paths: Sequence[Path],
) -> None:
    if _canonical_path_key(gate_path) == _canonical_path_key(sidecar_path):
        raise EvidenceError("gate verdict and provenance sidecar paths must differ")
    sidecar_text = json.dumps(sidecar, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    outputs = (gate_path, sidecar_path)
    gate_temp: Path | None = None
    sidecar_temp: Path | None = None
    try:
        gate_temp = _write_temp(gate_path, gate_text)
        sidecar_temp = _write_temp(sidecar_path, sidecar_text)
        _validate_output_destinations(
            gate_path, sidecar_path, thresholds_path, run_paths
        )
        _remove_final_outputs(outputs)
        os.replace(sidecar_temp, sidecar_path)
        sidecar_temp = None
        os.replace(gate_temp, gate_path)
        gate_temp = None
    except Exception as error:
        cleanup_error: EvidenceError | None = None
        try:
            _invalidate_outputs(outputs, (gate_temp, sidecar_temp))
        except EvidenceError as invalidation_error:
            cleanup_error = invalidation_error
        detail = f"paired output write failed: {error}"
        if cleanup_error is not None:
            detail += f"; {cleanup_error}"
        raise EvidenceError(detail) from error


def _run_hashes(run: RunEvidence, number: int) -> dict[str, Any]:
    files: dict[str, str | None] = {}
    for name in RUN_FILES:
        path = run.path / name
        files[name] = _file_sha256(path) if path.is_file() else None
    return {
        "run": number,
        "path": _recorded_path(run.path),
        "raw_files_sha256": files,
    }


def _build_sidecar(
    *,
    thresholds_path: Path,
    runs: Sequence[RunEvidence],
    provenance: dict[str, str],
    gate_path: Path,
    gate_text: str,
    effective_args: Sequence[str],
) -> dict[str, Any]:
    tool_path = Path(__file__).resolve()
    recorded_tool_path = _recorded_path(tool_path)
    recorded_args = _recorded_effective_args(effective_args)
    tool_hash = _file_sha256(tool_path)
    return {
        "schema_version": 1,
        "tool": TOOL_NAME,
        "tool_version": TOOL_VERSION,
        "tool_path": recorded_tool_path,
        "tool_sha256": tool_hash,
        "effective_argv": recorded_args,
        "effective_invocation": subprocess.list2cmdline(
            ["python", recorded_tool_path, *recorded_args]
        ),
        "required_run_count": REQUIRED_RUN_COUNT,
        "thresholds": {
            "path": _recorded_path(thresholds_path),
            "sha256": _file_sha256(thresholds_path),
        },
        "validated_provenance": provenance,
        "runs": [_run_hashes(run, number) for number, run in enumerate(runs, start=1)],
        "gate_verdict": {
            "path": _recorded_path(gate_path),
            "sha256": hashlib.sha256(gate_text.encode("utf-8")).hexdigest(),
        },
    }


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__, allow_abbrev=False)
    parser.add_argument("--thresholds", required=True, type=Path)
    parser.add_argument("--expected-runs", required=True, type=int)
    parser.add_argument("--run", action="append", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    effective_args = list(sys.argv[1:] if argv is None else argv)
    args = _parser().parse_args(effective_args)
    sidecar = _sidecar_path(args.output)
    try:
        _validate_output_destinations(
            args.output, sidecar, args.thresholds, args.run
        )
    except (EvidenceError, OSError, UnicodeError) as error:
        print(f"native-ratchet evidence error: {error}", file=sys.stderr)
        return 2
    outputs = (args.output, sidecar)
    try:
        if args.expected_runs != REQUIRED_RUN_COUNT:
            raise EvidenceError(
                f"--expected-runs must be exactly {REQUIRED_RUN_COUNT}, got {args.expected_runs}"
            )
        if len(args.run) != REQUIRED_RUN_COUNT:
            raise EvidenceError(
                f"expected exactly {REQUIRED_RUN_COUNT} runs but received {len(args.run)}"
            )
        resolved_runs = [_canonical_path_key(path) for path in args.run]
        if len(resolved_runs) != len(set(resolved_runs)):
            raise EvidenceError("the same run directory was supplied more than once")
        thresholds = read_thresholds(args.thresholds)
        runs = [read_run(path, thresholds) for path in args.run]
        rows, provenance = aggregate(thresholds, runs)
        gate_text = _gate_text(rows, len(runs))
        provenance_sidecar = _build_sidecar(
            thresholds_path=args.thresholds,
            runs=runs,
            provenance=provenance,
            gate_path=args.output,
            gate_text=gate_text,
            effective_args=effective_args,
        )
        _write_output_pair(
            args.output,
            sidecar,
            gate_text,
            provenance_sidecar,
            thresholds_path=args.thresholds,
            run_paths=args.run,
        )
    except (EvidenceError, OSError, csv.Error, UnicodeError) as error:
        cleanup_error: EvidenceError | None = None
        try:
            _invalidate_outputs(outputs)
        except EvidenceError as invalidation_error:
            cleanup_error = invalidation_error
        detail = str(error)
        if cleanup_error is not None:
            detail += f"; {cleanup_error}"
        print(f"native-ratchet evidence error: {detail}", file=sys.stderr)
        return 2
    return 0 if all(row["verdict"] == "pass" for row in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
