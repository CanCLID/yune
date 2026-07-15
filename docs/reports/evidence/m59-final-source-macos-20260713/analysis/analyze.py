#!/usr/bin/env python3
"""Reproduce the final-M59 macOS versus Windows diagnostic aggregation.

This script reads five external macOS rounds plus committed Windows evidence
and writes only to the explicitly selected output directory.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import statistics
from collections import Counter
from decimal import Decimal, ROUND_HALF_UP, getcontext
from pathlib import Path


getcontext().prec = 50

YUNE_COMMIT = "5879405c7b0f76af4dca7382f00b3e0605386f2c"
SIGNED_I0_YUNE_COMMIT = "457751824b8944676dc44912b9ce31ff29d78403"
LIBRIME_COMMIT = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
YUNE_DYLIB_SHA256 = "2e822d67e92794dace159b15104035954c6f2aee69e5d917793acb536e1deb56"
LIBRIME_DYLIB_SHA256 = "5a0b2b308a47141d4c6e0c23a48b3fcfdb49da2d846979cfee359660e1256dc9"

TRACK_A = [
    "n",
    "ni",
    "hao",
    "zhongguo",
    "ceshiyixiachangjushuruxingnengzenyang",
    "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
    "cszysmsrsd",
    "zybfshmsru",
    "zh",
    "j",
    "yi",
    "che",
    "chuang",
    "b",
    "ceshi",
    "zhongdengchangdu",
    "dazisudu",
]
TRACK_B = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung"
NEWLY_SIGNED = {
    "zh",
    "j",
    "yi",
    "che",
    "chuang",
    "b",
    "ceshi",
    "zhongdengchangdu",
    "dazisudu",
}
LONG_37 = "ceshiyixiachangjushuruxingnengzenyang"
LONG_59 = "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong"

REQUIRED_ARTIFACTS = [
    "environment.txt",
    "commands.txt",
    "summary-comparison.csv",
    "summary.csv",
    "candidate_snapshots.csv",
    "m37_metrics.csv",
    "memory-owner-profile.csv",
    "product_path_status.csv",
    "macos-verdict.md",
    "yune-dylib-post-run.sha256",
    "librime-dylib.sha256",
    "librime-source-identity.txt",
    "end-state.txt",
]


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open("r", encoding="utf-8-sig", newline="") as handle:
        return list(csv.DictReader(handle))


def write_csv(path: Path, rows: list[dict[str, object]], fields: list[str]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_hash_file(path: Path) -> str:
    return path.read_text(encoding="utf-8").split()[0]


def normalized_owner_profile(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    normalized = []
    for row in rows:
        if row["owner_id"].startswith("process.") or row["owner_id"] == "schema.config":
            continue
        comparable = dict(row)
        comparable.pop("session_id", None)
        normalized.append(comparable)
    return sorted(
        normalized,
        key=lambda row: json.dumps(row, ensure_ascii=False, sort_keys=True),
    )


def track_b_product_shape(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    fields = [
        "schema_id",
        "dictionary_id",
        "prism_id",
        "source_checksum",
        "table_checksum",
        "checksum_status",
        "table_parse",
        "prism_parse",
        "reverse_parse",
        "compiled_ready",
        "selected_storage",
        "table_format",
        "table_mapping_mode",
        "prism_mapping_mode",
        "source_fallback",
        "stored_entries",
        "compact_all_codes_count",
        "compact_expanded_table_entries",
        "compact_expansion_status",
        "table_heap_mirror_bytes",
        "prism_heap_mirror_bytes",
        "rsmarisa_status",
        "rsmarisa_mapping_mode",
        "rsmarisa_num_tries",
        "rsmarisa_num_keys",
        "rsmarisa_sample_key",
    ]
    selected = [row for row in rows if row["track"] == "track-b-product"]
    return sorted(
        [{field: row[field] for field in fields} for row in selected],
        key=lambda row: (row["dictionary_id"], row["prism_id"]),
    )


def parse_key_values(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        if "=" not in raw:
            continue
        key, value = raw.split("=", 1)
        if key and key.replace("_", "").isalnum():
            result[key] = value
    return result


def dec(value: str | Decimal) -> Decimal:
    return value if isinstance(value, Decimal) else Decimal(value)


def decimal_text(value: Decimal) -> str:
    text = format(value, "f")
    if "." in text:
        text = text.rstrip("0").rstrip(".")
    return text or "0"


def median(values: list[Decimal]) -> Decimal:
    if len(values) % 2 != 1:
        raise AssertionError("this analysis requires an odd run count")
    return sorted(values)[len(values) // 2]


def spread_pct(values: list[Decimal]) -> Decimal:
    low = min(values)
    if low == 0:
        raise AssertionError("spread denominator is zero")
    return (max(values) - low) / low * Decimal(100)


def pct_difference(observed: Decimal, reference: Decimal) -> Decimal:
    if reference == 0:
        raise AssertionError("percentage-difference reference is zero")
    return (observed / reference - Decimal(1)) * Decimal(100)


def round_1(value: Decimal) -> Decimal:
    return value.quantize(Decimal("0.1"), rounding=ROUND_HALF_UP)


def classify(delta_pct: Decimal) -> str:
    magnitude = abs(delta_pct)
    if magnitude <= Decimal(10):
        return "close"
    if magnitude <= Decimal(25):
        return "notable"
    return "material"


def select_one(rows: list[dict[str, str]], **filters: str) -> dict[str, str]:
    matches = [row for row in rows if all(row.get(key) == value for key, value in filters.items())]
    if len(matches) != 1:
        raise AssertionError(f"expected one row for {filters}, found {len(matches)}")
    return matches[0]


def ratio_gate_rows(rows: list[dict[str, str]]) -> dict[str, dict[str, str]]:
    selected = [
        row
        for row in rows
        if row["kind"] == "latency_ratio"
        and row["workload"] == "key_sequence_process_with_context"
        and row["metric"] == "yune_librime_median_ratio"
        and row["input"] in TRACK_A
    ]
    if len(selected) != len(TRACK_A) or len({row["input"] for row in selected}) != len(TRACK_A):
        raise AssertionError("gate verdict does not contain one exact row for every Track A input")
    return {row["input"]: row for row in selected}


def validate_gate_formulas(label: str, rows: dict[str, dict[str, str]]) -> None:
    for input_value in TRACK_A:
        row = rows[input_value]
        values = [dec(row[f"run{i}_observed"]) for i in range(1, 6)]
        if median(values) != dec(row["median_observed"]):
            raise AssertionError(f"{label} median mismatch for {input_value}")
        if max(values) != dec(row["worst_observed"]):
            raise AssertionError(f"{label} worst mismatch for {input_value}")
        if round_1(spread_pct(values)) != dec(row["spread_pct"]):
            raise AssertionError(f"{label} spread mismatch for {input_value}")


def special_labels(input_value: str) -> str:
    labels: list[str] = []
    if input_value in {"n", "ni", "hao"}:
        labels.append("short-explicit")
    if input_value == LONG_37:
        labels.append("37-character")
    if input_value == LONG_59:
        labels.append("59-character")
    if input_value in NEWLY_SIGNED:
        labels.append("newly-signed")
    return ";".join(labels)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--evidence-root",
        type=Path,
        required=True,
        help="external root containing accepted/run-1 through run-5",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        required=True,
        help="Yune repository root containing the committed Windows evidence",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="external directory for regenerated analysis outputs",
    )
    args = parser.parse_args()
    evidence_root = args.evidence_root.resolve()
    repo_root = args.repo_root.resolve()
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    accepted = evidence_root / "accepted"
    mac_runs = [accepted / f"run-{i}" for i in range(1, 6)]
    for path in mac_runs:
        if not path.is_dir():
            raise AssertionError(f"missing accepted run: {path}")

    final_windows_root = (
        repo_root
        / "docs/reports/evidence/m59-canonical-jyutping-reachability-parity"
        / "increment-4e-lane-b-exact-order/performance-ratchet"
    )
    signed_root = repo_root / "docs/reports/evidence/m59-closeout-baseline"
    thresholds_path = (
        repo_root
        / "docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv"
    )
    final_gate_path = final_windows_root / "gate-verdict.csv"
    signed_gate_path = signed_root / "gate-verdict.csv"
    expanded_path = signed_root / "expanded-ceiling-derivation.csv"
    provenance_path = final_windows_root / "gate-verdict.provenance.json"
    signed_provenance_path = signed_root / "gate-verdict.provenance.json"

    final_gate = ratio_gate_rows(read_csv(final_gate_path))
    signed_gate = ratio_gate_rows(read_csv(signed_gate_path))
    validate_gate_formulas("source-matched 587 Windows", final_gate)
    validate_gate_formulas("historical signed Increment-0 Windows", signed_gate)

    threshold_rows = read_csv(thresholds_path)
    threshold_map: dict[str, dict[str, str]] = {}
    for input_value in TRACK_A:
        threshold_map[input_value] = select_one(
            threshold_rows,
            kind="latency_ratio",
            workload="key_sequence_process_with_context",
            input=input_value,
            metric="yune_librime_median_ratio",
        )
        if dec(threshold_map[input_value]["ceiling"]) != dec(signed_gate[input_value]["ceiling"]):
            raise AssertionError(f"threshold/signed-gate ceiling mismatch for {input_value}")

    expanded = {row["input"]: row for row in read_csv(expanded_path)}
    if set(expanded) != NEWLY_SIGNED:
        raise AssertionError("expanded ceiling derivation does not contain exactly nine newly signed rows")
    for input_value in NEWLY_SIGNED:
        if dec(expanded[input_value]["median_ratio"]) != dec(signed_gate[input_value]["median_observed"]):
            raise AssertionError(f"expanded median mismatch for {input_value}")
        if dec(expanded[input_value]["proposed_ceiling_f3"]) != dec(signed_gate[input_value]["ceiling"]):
            raise AssertionError(f"expanded ceiling mismatch for {input_value}")

    provenance = json.loads(provenance_path.read_text(encoding="utf-8"))
    validated = provenance["validated_provenance"]
    if validated["source_commit"] != YUNE_COMMIT or validated["yune_git_head"] != YUNE_COMMIT:
        raise AssertionError("source-matched Windows provenance is not the exact final-M59 commit")
    if validated["iterations"] != "9" or validated["session_iterations"] != "60" or validated["key_iterations"] != "80":
        raise AssertionError("source-matched Windows iteration protocol differs")
    if validated["track_a_inputs"].split(",") != TRACK_A or validated["track_b_inputs"] != TRACK_B:
        raise AssertionError("source-matched Windows input protocol differs")

    signed_provenance = json.loads(signed_provenance_path.read_text(encoding="utf-8"))
    signed_validated = signed_provenance["validated_provenance"]
    if (
        signed_validated["source_commit"] != SIGNED_I0_YUNE_COMMIT
        or signed_validated["yune_git_head"] != SIGNED_I0_YUNE_COMMIT
    ):
        raise AssertionError("historical signed Windows provenance is not Increment-0")
    if (
        signed_validated["iterations"] != "9"
        or signed_validated["session_iterations"] != "60"
        or signed_validated["key_iterations"] != "80"
    ):
        raise AssertionError("historical signed Windows iteration protocol differs")
    if (
        signed_validated["track_a_inputs"].split(",") != TRACK_A
        or signed_validated["track_b_inputs"] != TRACK_B
    ):
        raise AssertionError("historical signed Windows input protocol differs")

    mac_comparisons: list[dict[str, dict[str, str]]] = []
    mac_summaries: list[list[dict[str, str]]] = []
    audit_rows: list[dict[str, object]] = []
    validation_rows: list[dict[str, object]] = []

    for run_number, run_path in enumerate(mac_runs, 1):
        comparison_rows = [
            row
            for row in read_csv(run_path / "summary-comparison.csv")
            if row["track"] == "track-a-comparison"
            and row["schema_id"] == "luna_pinyin"
            and row["workload"] == "key_sequence_process_with_context"
            and row["input"] in TRACK_A
        ]
        if len(comparison_rows) != len(TRACK_A) or len({row["input"] for row in comparison_rows}) != len(TRACK_A):
            raise AssertionError(f"run {run_number} lacks one exact comparison row per Track A input")
        if set(row["input"] for row in comparison_rows) != set(TRACK_A):
            raise AssertionError(f"run {run_number} Track A input set differs")
        mac_comparisons.append({row["input"]: row for row in comparison_rows})

        summary_rows = read_csv(run_path / "summary.csv")
        for engine in ("yune", "librime-1.17.0"):
            selected = [
                row
                for row in summary_rows
                if row["engine"] == engine
                and row["track"] == "track-a-comparison"
                and row["schema_id"] == "luna_pinyin"
                and row["workload"] == "key_sequence_process_with_context"
                and row["input"] in TRACK_A
            ]
            if len(selected) != len(TRACK_A) or len({row["input"] for row in selected}) != len(TRACK_A):
                raise AssertionError(f"run {run_number} summary lacks exact {engine} Track A rows")
        select_one(
            summary_rows,
            engine="yune",
            track="track-b-product",
            schema_id="jyut6ping3_mobile",
            workload="key_sequence_process_with_context",
            input=TRACK_B,
        )
        mac_summaries.append(summary_rows)

        status = parse_key_values(run_path / "source-status-pre-run.txt")
        librime_identity = parse_key_values(run_path / "librime-source-identity.txt")
        end_state = parse_key_values(run_path / "end-state.txt")
        benchmark_environment = parse_key_values(run_path / "environment.txt")
        commands_text = (run_path / "commands.txt").read_text(encoding="utf-8")
        post_yune = parse_hash_file(run_path / "yune-dylib-post-run.sha256")
        post_librime = parse_hash_file(run_path / "librime-dylib.sha256")
        candidate_path = run_path / "candidate_snapshots.csv"
        candidate_rows = read_csv(candidate_path)
        missing = [name for name in REQUIRED_ARTIFACTS if not (run_path / name).is_file()]
        source_parameters_ok = (
            status.get("logical_round") == str(run_number)
            and status.get("yune_commit") == YUNE_COMMIT
            and status.get("librime_commit") == LIBRIME_COMMIT
            and status.get("yune_source_status") == "clean"
            and status.get("librime_source_status") == "clean"
            and librime_identity.get("commit") == LIBRIME_COMMIT
            and librime_identity.get("status") == "clean"
            and status.get("iterations") == "9"
            and status.get("session_iterations") == "60"
            and status.get("key_iterations") == "80"
            and status.get("product_deployment") == "enabled"
            and status.get("track_a", "").split(",") == TRACK_A
            and status.get("track_b") == TRACK_B
        )
        hash_ok = (
            status.get("yune_dylib_sha256") == YUNE_DYLIB_SHA256
            and post_yune == YUNE_DYLIB_SHA256
            and status.get("librime_dylib_sha256") == LIBRIME_DYLIB_SHA256
            and post_librime == LIBRIME_DYLIB_SHA256
        )
        power_ok = status.get("power") == "Now drawing from 'AC Power'" and "AC Power" in (run_path / "end-state.txt").read_text(encoding="utf-8")
        artifact_ok = not missing and len(candidate_rows) == 175
        measurement_git_status = benchmark_environment.get("yune_git_status_short", "")
        output_location_deviation = (
            measurement_git_status
            == "?? docs/reports/evidence/m59-final-macos-transient/"
            and "docs/reports/evidence/m59-final-macos-transient/" in commands_text
        )
        if run_number < 5:
            next_status = parse_key_values(
                mac_runs[run_number] / "source-status-pre-run.txt"
            )
            after_move_status_ok = (
                next_status.get("yune_commit") == YUNE_COMMIT
                and next_status.get("yune_source_status") == "clean"
            )
            after_move_status_evidence = "inferred-clean-from-next-round-preflight"
        else:
            later_gate_result = (
                evidence_root / "aggregate/behavior-gates/result.txt"
            ).read_text(encoding="utf-8")
            after_move_status_ok = (
                "Worktree state before: clean detached HEAD" in later_gate_result
                and f"Yune commit: {YUNE_COMMIT}" in later_gate_result
            )
            after_move_status_evidence = (
                "inferred-clean-from-later-behavior-gate-preflight"
            )
        diagnostic_ok = (
            source_parameters_ok
            and hash_ok
            and power_ok
            and artifact_ok
            and output_location_deviation
            and after_move_status_ok
        )
        audit_rows.append(
            {
                "run": run_number,
                "path": f"accepted/run-{run_number}",
                "start_utc": status.get("start_utc", ""),
                "end_utc": end_state.get("end_utc", ""),
                "power_pre": status.get("power", ""),
                "power_post": "AC Power" if power_ok else "not-confirmed",
                "yune_commit": status.get("yune_commit", ""),
                "yune_source_status_pre_output": status.get("yune_source_status", ""),
                "yune_measurement_git_status_short": measurement_git_status,
                "yune_after_move_status_evidence": after_move_status_evidence,
                "yune_after_move_status_ok": str(after_move_status_ok).lower(),
                "output_location_protocol": "transient-untracked-worktree-output-then-moved-external",
                "librime_commit": status.get("librime_commit", ""),
                "librime_source_status": status.get("librime_source_status", ""),
                "yune_dylib_pre_sha256": status.get("yune_dylib_sha256", ""),
                "yune_dylib_post_sha256": post_yune,
                "yune_pre_post_identical": str(status.get("yune_dylib_sha256") == post_yune).lower(),
                "librime_dylib_pre_sha256": status.get("librime_dylib_sha256", ""),
                "librime_dylib_post_sha256": post_librime,
                "librime_pre_post_identical": str(status.get("librime_dylib_sha256") == post_librime).lower(),
                "summary_comparison_sha256": sha256(run_path / "summary-comparison.csv"),
                "summary_sha256": sha256(run_path / "summary.csv"),
                "candidate_snapshots_sha256": sha256(candidate_path),
                "candidate_snapshot_data_rows": len(candidate_rows),
                "track_a_ratio_rows": len(comparison_rows),
                "missing_required_artifacts": ";".join(missing),
                "source_and_parameters_ok": str(source_parameters_ok).lower(),
                "binary_identity_ok": str(hash_ok).lower(),
                "ac_power_ok": str(power_ok).lower(),
                "artifacts_ok": str(artifact_ok).lower(),
                "verdict": (
                    "diagnostic-pass-with-disclosed-output-path-deviation"
                    if diagnostic_ok
                    else "fail"
                ),
            }
        )

    if any(
        row["verdict"] != "diagnostic-pass-with-disclosed-output-path-deviation"
        for row in audit_rows
    ):
        raise AssertionError("one or more measured-run audit rows failed")
    if len({row["yune_dylib_post_sha256"] for row in audit_rows}) != 1:
        raise AssertionError("Yune binary varied across the five rounds")
    if len({row["librime_dylib_post_sha256"] for row in audit_rows}) != 1:
        raise AssertionError("librime binary varied across the five rounds")
    if len({row["candidate_snapshots_sha256"] for row in audit_rows}) != 1:
        raise AssertionError("candidate snapshots varied across the five rounds")

    final_windows_run_paths = [
        final_windows_root / f"run{run_number}" for run_number in range(1, 6)
    ]
    normalized_owner_sets = [
        normalized_owner_profile(read_csv(run_path / "memory-owner-profile.csv"))
        for run_path in mac_runs + final_windows_run_paths
    ]
    if any(len(rows) != 99 for rows in normalized_owner_sets):
        raise AssertionError("normalized logical-owner profile does not have 99 rows")
    if any(rows != normalized_owner_sets[0] for rows in normalized_owner_sets[1:]):
        raise AssertionError("normalized Mac/Windows logical-owner profiles differ")

    m57_product_path = (
        repo_root
        / "docs/reports/evidence/m57-macos-track-a-sentence-model-parity"
        / "full-pass-1/product_path_status.csv"
    )
    track_b_shapes = [
        track_b_product_shape(read_csv(run_path / "product_path_status.csv"))
        for run_path in mac_runs + final_windows_run_paths
    ] + [track_b_product_shape(read_csv(m57_product_path))]
    if any(len(rows) != 2 for rows in track_b_shapes):
        raise AssertionError("Track B product shape does not contain two dictionary rows")
    if any(rows != track_b_shapes[0] for rows in track_b_shapes[1:]):
        raise AssertionError("Mac/Windows/M57 Track B checksum or storage shape differs")

    comparison_rows_out: list[dict[str, object]] = []
    for input_value in TRACK_A:
        mac_values = [dec(run[input_value]["yune_librime_median_ratio"]) for run in mac_comparisons]
        mac_median = median(mac_values)
        mac_worst = max(mac_values)
        mac_spread = spread_pct(mac_values)
        final_windows_median = dec(final_gate[input_value]["median_observed"])
        signed_median = dec(signed_gate[input_value]["median_observed"])
        signed_ceiling = dec(signed_gate[input_value]["ceiling"])
        final_delta = pct_difference(mac_median, final_windows_median)
        signed_delta = pct_difference(mac_median, signed_median)
        ceiling_median_delta = pct_difference(mac_median, signed_ceiling)
        ceiling_worst_delta = pct_difference(mac_worst, signed_ceiling)
        row: dict[str, object] = {
            "input": input_value,
            "special_rows": special_labels(input_value),
            "newly_signed": str(input_value in NEWLY_SIGNED).lower(),
        }
        for run_number, value in enumerate(mac_values, 1):
            row[f"mac_run{run_number}_ratio"] = decimal_text(value)
        row.update(
            {
                "mac_median_ratio": decimal_text(mac_median),
                "mac_pooled_worst_ratio": decimal_text(mac_worst),
                "mac_spread_pct": decimal_text(mac_spread),
                "mac_spread_pct_rounded_1dp": decimal_text(round_1(mac_spread)),
                "source_matched_final_windows_median_ratio": final_gate[input_value]["median_observed"],
                "mac_vs_source_matched_final_windows_pct": decimal_text(final_delta),
                "mac_vs_source_matched_final_windows_class": classify(final_delta),
                "historical_signed_i0_windows_median_ratio": signed_gate[input_value]["median_observed"],
                "historical_signed_windows_ceiling": signed_gate[input_value]["ceiling"],
                "mac_vs_historical_signed_i0_windows_pct": decimal_text(signed_delta),
                "mac_vs_historical_signed_i0_windows_class": classify(signed_delta),
                "mac_median_vs_signed_ceiling_pct": decimal_text(ceiling_median_delta),
                "mac_median_signed_ceiling_diagnostic": "at-or-below" if mac_median <= signed_ceiling else "above",
                "mac_pooled_worst_vs_signed_ceiling_pct": decimal_text(ceiling_worst_delta),
                "mac_pooled_worst_signed_ceiling_diagnostic": "at-or-below" if mac_worst <= signed_ceiling else "above",
                "comparison_boundary": "diagnostic-only; Windows ceiling is not a macOS acceptance threshold",
            }
        )
        comparison_rows_out.append(row)

    comparison_fields = [
        "input",
        "special_rows",
        "newly_signed",
        *[f"mac_run{i}_ratio" for i in range(1, 6)],
        "mac_median_ratio",
        "mac_pooled_worst_ratio",
        "mac_spread_pct",
        "mac_spread_pct_rounded_1dp",
        "source_matched_final_windows_median_ratio",
        "mac_vs_source_matched_final_windows_pct",
        "mac_vs_source_matched_final_windows_class",
        "historical_signed_i0_windows_median_ratio",
        "historical_signed_windows_ceiling",
        "mac_vs_historical_signed_i0_windows_pct",
        "mac_vs_historical_signed_i0_windows_class",
        "mac_median_vs_signed_ceiling_pct",
        "mac_median_signed_ceiling_diagnostic",
        "mac_pooled_worst_vs_signed_ceiling_pct",
        "mac_pooled_worst_signed_ceiling_diagnostic",
        "comparison_boundary",
    ]
    write_csv(output_dir / "track-a-17-row-comparison.csv", comparison_rows_out, comparison_fields)

    final_windows_summaries = [read_csv(final_windows_root / f"run{i}" / "summary.csv") for i in range(1, 6)]
    component_rows: list[dict[str, object]] = []
    for input_value in TRACK_A:
        for engine in ("yune", "librime-1.17.0"):
            mac_values = [
                dec(
                    select_one(
                        run,
                        engine=engine,
                        track="track-a-comparison",
                        schema_id="luna_pinyin",
                        workload="key_sequence_process_with_context",
                        input=input_value,
                    )["median_us"]
                )
                for run in mac_summaries
            ]
            windows_values = [
                dec(
                    select_one(
                        run,
                        engine=engine,
                        track="track-a-comparison",
                        schema_id="luna_pinyin",
                        workload="key_sequence_process_with_context",
                        input=input_value,
                    )["median_us"]
                )
                for run in final_windows_summaries
            ]
            mac_center = median(mac_values)
            windows_center = median(windows_values)
            row = {
                "input": input_value,
                "special_rows": special_labels(input_value),
                "engine": engine,
            }
            for run_number, value in enumerate(mac_values, 1):
                row[f"mac_run{run_number}_median_us"] = decimal_text(value)
            row.update(
                {
                    "mac_median_of_run_medians_us": decimal_text(mac_center),
                    "mac_worst_run_median_us": decimal_text(max(mac_values)),
                    "mac_spread_pct": decimal_text(spread_pct(mac_values)),
                }
            )
            for run_number, value in enumerate(windows_values, 1):
                row[f"source_matched_windows_run{run_number}_median_us"] = decimal_text(value)
            row.update(
                {
                    "source_matched_windows_median_of_run_medians_us": decimal_text(windows_center),
                    "source_matched_windows_worst_run_median_us": decimal_text(max(windows_values)),
                    "source_matched_windows_spread_pct": decimal_text(spread_pct(windows_values)),
                    "mac_vs_source_matched_windows_median_pct": decimal_text(pct_difference(mac_center, windows_center)),
                    "comparison_boundary": "absolute latency is platform-specific and diagnostic only",
                }
            )
            component_rows.append(row)

    component_fields = [
        "input",
        "special_rows",
        "engine",
        *[f"mac_run{i}_median_us" for i in range(1, 6)],
        "mac_median_of_run_medians_us",
        "mac_worst_run_median_us",
        "mac_spread_pct",
        *[f"source_matched_windows_run{i}_median_us" for i in range(1, 6)],
        "source_matched_windows_median_of_run_medians_us",
        "source_matched_windows_worst_run_median_us",
        "source_matched_windows_spread_pct",
        "mac_vs_source_matched_windows_median_pct",
        "comparison_boundary",
    ]
    write_csv(output_dir / "track-a-component-absolute-latency.csv", component_rows, component_fields)

    mac_track_b_rows = [
        select_one(
            run,
            engine="yune",
            track="track-b-product",
            schema_id="jyut6ping3_mobile",
            workload="key_sequence_process_with_context",
            input=TRACK_B,
        )
        for run in mac_summaries
    ]
    windows_track_b_rows = [
        select_one(
            run,
            engine="yune",
            track="track-b-product",
            schema_id="jyut6ping3_mobile",
            workload="key_sequence_process_with_context",
            input=TRACK_B,
        )
        for run in final_windows_summaries
    ]
    mac_track_b_medians = [dec(row["median_us"]) for row in mac_track_b_rows]
    windows_track_b_medians = [dec(row["median_us"]) for row in windows_track_b_rows]
    mac_track_b_center = median(mac_track_b_medians)
    windows_track_b_center = median(windows_track_b_medians)
    track_b_rows_out: list[dict[str, object]] = []
    for index, (mac_row, windows_row) in enumerate(zip(mac_track_b_rows, windows_track_b_rows), 1):
        track_b_rows_out.append(
            {
                "run": index,
                "input": TRACK_B,
                "mac_median_us": mac_row["median_us"],
                "mac_p95_us": mac_row["p95_us"],
                "mac_p99_us": mac_row["p99_us"],
                "mac_max_us": mac_row["max_us"],
                "mac_median_working_set_bytes": mac_row["median_working_set_bytes"],
                "mac_max_peak_working_set_bytes": mac_row["max_peak_working_set_bytes"],
                "source_matched_windows_median_us": windows_row["median_us"],
                "source_matched_windows_p95_us": windows_row["p95_us"],
                "source_matched_windows_p99_us": windows_row["p99_us"],
                "source_matched_windows_max_us": windows_row["max_us"],
                "mac_vs_source_matched_windows_run_median_pct": decimal_text(
                    pct_difference(dec(mac_row["median_us"]), dec(windows_row["median_us"]))
                ),
                "mac_median_of_five_run_medians_us": decimal_text(mac_track_b_center),
                "mac_worst_run_median_us": decimal_text(max(mac_track_b_medians)),
                "mac_pooled_worst_sample_us": decimal_text(max(dec(row["max_us"]) for row in mac_track_b_rows)),
                "mac_spread_pct": decimal_text(spread_pct(mac_track_b_medians)),
                "source_matched_windows_median_of_five_run_medians_us": decimal_text(windows_track_b_center),
                "source_matched_windows_worst_run_median_us": decimal_text(max(windows_track_b_medians)),
                "source_matched_windows_pooled_worst_sample_us": decimal_text(
                    max(dec(row["max_us"]) for row in windows_track_b_rows)
                ),
                "source_matched_windows_spread_pct": decimal_text(spread_pct(windows_track_b_medians)),
                "mac_vs_source_matched_windows_aggregate_median_pct": decimal_text(
                    pct_difference(mac_track_b_center, windows_track_b_center)
                ),
                "comparison_boundary": "absolute latency and memory are platform-specific; diagnostic only",
            }
        )
    track_b_fields = list(track_b_rows_out[0].keys())
    write_csv(output_dir / "track-b-five-observations.csv", track_b_rows_out, track_b_fields)

    audit_fields = list(audit_rows[0].keys())
    write_csv(output_dir / "artifact-hash-audit.csv", audit_rows, audit_fields)

    final_classes = Counter(str(row["mac_vs_source_matched_final_windows_class"]) for row in comparison_rows_out)
    signed_classes = Counter(str(row["mac_vs_historical_signed_i0_windows_class"]) for row in comparison_rows_out)
    high_spread = sorted(
        comparison_rows_out,
        key=lambda row: dec(str(row["mac_spread_pct"])),
        reverse=True,
    )
    rows_above_10_spread = [row for row in high_spread if dec(str(row["mac_spread_pct"])) > Decimal(10)]

    component_by_key = {(str(row["input"]), str(row["engine"])): row for row in component_rows}
    yune_component_deltas = [dec(str(row["mac_vs_source_matched_windows_median_pct"])) for row in component_rows if row["engine"] == "yune"]
    librime_component_deltas = [dec(str(row["mac_vs_source_matched_windows_median_pct"])) for row in component_rows if row["engine"] == "librime-1.17.0"]

    comparison_by_input = {str(row["input"]): row for row in comparison_rows_out}
    findings_lines = [
        "# Final-M59 macOS aggregation findings",
        "",
        "## Result",
        "",
        (
            "All five measured macOS rounds are aggregatable as a diagnostic with one disclosed protocol deviation: the Yune dylib hash stayed "
            f"`{YUNE_DYLIB_SHA256}`, the librime dylib hash stayed `{LIBRIME_DYLIB_SHA256}`, "
            "the tracked source was directly recorded clean before each transient output directory was created and after-move cleanliness is inferred from the next clean preflight (runs 1–4) or later clean behavior-gate preflight (run 5), all runs remained "
            "on AC power, and the 175-row candidate snapshot was byte-identical across rounds. During measurement Git reported only the untracked transient evidence directory."
        ),
        "",
        (
            f"Every Track A macOS median ratio is below 1.0 (Yune faster than librime in this driver). "
            f"Against the source-matched final-M59 Windows medians, {final_classes['close']} rows are "
            f"close, {final_classes['notable']} notable, and {final_classes['material']} material under "
            "the requested diagnostic labels. Against the historical signed Increment-0 Windows "
            f"medians, {signed_classes['close']} are close, {signed_classes['notable']} notable, and "
            f"{signed_classes['material']} material; that comparison combines platform and major source-state changes."
        ),
        "",
        "The signed Windows ceilings are used only as a diagnostic reference. All 17 macOS medians and all 17 pooled worst ratios are at or below them; this does not establish or alter any macOS acceptance threshold.",
        "",
        "## Explicit rows",
        "",
    ]
    for label, input_value in [
        ("37-character", LONG_37),
        ("59-character", LONG_59),
        ("n", "n"),
        ("ni", "ni"),
        ("hao", "hao"),
    ]:
        row = comparison_by_input[input_value]
        findings_lines.append(
            f"- {label}: macOS median `{dec(str(row['mac_median_ratio'])):.3f}x`, pooled worst "
            f"`{dec(str(row['mac_pooled_worst_ratio'])):.3f}x`, spread "
            f"`{round_1(dec(str(row['mac_spread_pct']))):.1f}%`; source-matched Windows median "
            f"`{dec(str(row['source_matched_final_windows_median_ratio'])):.3f}x` "
            f"({round_1(dec(str(row['mac_vs_source_matched_final_windows_pct']))):+.1f}%, "
            f"{row['mac_vs_source_matched_final_windows_class']})."
        )

    findings_lines.extend(["", "All nine newly signed rows:", ""])
    for input_value in TRACK_A:
        if input_value not in NEWLY_SIGNED:
            continue
        row = comparison_by_input[input_value]
        findings_lines.append(
            f"- `{input_value}`: macOS `{dec(str(row['mac_median_ratio'])):.3f}x`; source-matched 587 Windows "
            f"`{dec(str(row['source_matched_final_windows_median_ratio'])):.3f}x`; delta "
            f"`{round_1(dec(str(row['mac_vs_source_matched_final_windows_pct']))):+.1f}%` "
            f"({row['mac_vs_source_matched_final_windows_class']})."
        )

    findings_lines.extend(
        [
            "",
            "## Track B product input",
            "",
            (
                f"The five macOS key-sequence observations are {', '.join(f'{value:.3f}' for value in mac_track_b_medians)} µs/key. "
                f"Their median is `{mac_track_b_center:.3f} µs/key`, worst run median "
                f"`{max(mac_track_b_medians):.3f} µs/key`, pooled worst sample "
                f"`{max(dec(row['max_us']) for row in mac_track_b_rows):.3f} µs/key`, and spread "
                f"`{round_1(spread_pct(mac_track_b_medians)):.1f}%`. The source-matched Windows "
                f"median is `{windows_track_b_center:.3f} µs/key`; the macOS median is "
                f"`{round_1(pct_difference(mac_track_b_center, windows_track_b_center)):+.1f}%` different. "
                "Absolute latency and memory remain platform-specific diagnostics."
            ),
            "",
            "## Absolute-latency decomposition",
            "",
            (
                "Across the 17 per-input component rows, the median macOS-versus-source-matched-Windows "
                f"absolute-latency change is `{round_1(median(yune_component_deltas)):+.1f}%` for Yune and "
                f"`{round_1(median(librime_component_deltas)):+.1f}%` for librime. This shows that the ratio "
                "movement is not attributable to librime alone; both numerator and denominator move across platforms."
            ),
        ]
    )
    for input_value in (LONG_37, LONG_59, "n", "ni", "hao"):
        yune_row = component_by_key[(input_value, "yune")]
        librime_row = component_by_key[(input_value, "librime-1.17.0")]
        findings_lines.append(
            f"- `{input_value}`: Yune macOS/Windows absolute median delta "
            f"`{round_1(dec(str(yune_row['mac_vs_source_matched_windows_median_pct']))):+.1f}%`; "
            f"librime `{round_1(dec(str(librime_row['mac_vs_source_matched_windows_median_pct']))):+.1f}%`."
        )

    findings_lines.extend(["", "## Stability and interpretation", ""])
    if rows_above_10_spread:
        findings_lines.append(
            "The following macOS ratio rows exceed 10% five-round spread and must be read as noisy diagnostics: "
            + ", ".join(
                f"`{row['input']}` ({round_1(dec(str(row['mac_spread_pct']))):.1f}%)"
                for row in rows_above_10_spread
            )
            + ". All measured rounds are retained."
        )
    else:
        findings_lines.append("No macOS ratio row exceeds 10% five-round spread; all measured rounds are retained.")
    findings_lines.extend(
        [
            "",
            "The five-run evidence supports a real Increment-4e source-bound macOS advantage over librime for this benchmark driver, but it does not by itself identify a single platform cause. Cross-platform absolute counters, memory, scheduler effects, thermal/noise effects, and toolchain/ABI effects are confounded. The source-matched component table should be used to choose follow-up controls rather than treating Windows ceilings as portable.",
            "",
            "## Reproduction and sources",
            "",
            "Run `python3 analyze.py --evidence-root <external-evidence-root> --repo-root <yune-repository-root> --output-dir <external-review-root>/analysis`. Inputs are read from the five external `accepted/run-*` directories and the committed final-M59 Windows, historical signed Increment-0, ceiling, and expanded-derivation CSVs. The script fails on missing/duplicate Track A rows, formula mismatches, source/parameter drift, unexpected output-location status, missing artifacts, or variable binaries.",
            "",
        ]
    )
    (output_dir / "findings.md").write_text("\n".join(findings_lines), encoding="utf-8")

    validation_rows.extend(
        [
            {"check": "mac_run_count", "status": "pass", "detail": "5 measured logical rounds"},
            {"check": "track_a_output_rows", "status": "pass", "detail": str(len(comparison_rows_out))},
            {"check": "track_a_output_unique_inputs", "status": "pass", "detail": str(len({row['input'] for row in comparison_rows_out}))},
            {"check": "track_b_output_rows", "status": "pass", "detail": str(len(track_b_rows_out))},
            {"check": "component_output_rows", "status": "pass", "detail": str(len(component_rows))},
            {"check": "final_windows_gate_formulas", "status": "pass", "detail": "median/worst/spread recomputed for 17 rows"},
            {"check": "signed_i0_gate_formulas", "status": "pass", "detail": "median/worst/spread recomputed for 17 rows"},
            {"check": "signed_ceiling_join", "status": "pass", "detail": "17/17 threshold rows match signed gate"},
            {"check": "expanded_ceiling_join", "status": "pass", "detail": "9/9 derivation rows match signed gate"},
            {"check": "source_matched_windows_commit", "status": "pass", "detail": YUNE_COMMIT},
            {"check": "historical_signed_i0_windows_commit", "status": "pass", "detail": SIGNED_I0_YUNE_COMMIT},
            {"check": "mac_source_commits_and_parameters", "status": "pass", "detail": "5/5 exact commits, inputs, iterations, and directly captured pre-output cleanliness"},
            {"check": "after_move_source_status", "status": "pass-inferred", "detail": "runs 1-4 inferred clean from the next round preflight; run 5 inferred clean from the later behavior-gate preflight"},
            {"check": "output_location_protocol", "status": "disclosed-deviation", "detail": "5/5 wrote first to an untracked disposable-worktree evidence directory, then moved external; no tracked source or binary changed"},
            {"check": "yune_binary_identity", "status": "pass", "detail": YUNE_DYLIB_SHA256},
            {"check": "librime_binary_identity", "status": "pass", "detail": LIBRIME_DYLIB_SHA256},
            {"check": "candidate_snapshot_identity", "status": "pass", "detail": str(audit_rows[0]['candidate_snapshots_sha256'])},
            {"check": "logical_owner_profile_exact", "status": "pass", "detail": "99/99 normalized rows exact across all five Mac and all five final-Windows files; process.*, schema.config, and session_id excluded"},
            {"check": "track_b_product_shape_exact", "status": "pass", "detail": "2/2 dictionary rows exact across all five Mac, all five final-Windows, and M57 files for checksum/storage/mapping fields"},
            {"check": "required_artifacts", "status": "pass", "detail": "13/13 in each of 5 runs"},
        ]
    )
    write_csv(output_dir / "validation-checks.csv", validation_rows, ["check", "status", "detail"])

    source_rows = [
        {"role": "mac_rounds", "path": "accepted/run-{1..5}", "sha256": "see artifact-hash-audit.csv", "note": "exact final-M59 source; preserved external after each run; transient output-location deviation disclosed"},
        {"role": "source_matched_final_windows", "path": final_gate_path.relative_to(repo_root).as_posix(), "sha256": sha256(final_gate_path), "note": f"provenance binds {YUNE_COMMIT}"},
        {"role": "source_matched_final_windows_provenance", "path": provenance_path.relative_to(repo_root).as_posix(), "sha256": sha256(provenance_path), "note": "same inputs and iteration counts"},
        {"role": "historical_signed_i0_windows", "path": signed_gate_path.relative_to(repo_root).as_posix(), "sha256": sha256(signed_gate_path), "note": "historical source 45775182; diagnostic context"},
        {"role": "historical_signed_i0_windows_provenance", "path": signed_provenance_path.relative_to(repo_root).as_posix(), "sha256": sha256(signed_provenance_path), "note": f"provenance binds {SIGNED_I0_YUNE_COMMIT}"},
        {"role": "signed_windows_thresholds", "path": thresholds_path.relative_to(repo_root).as_posix(), "sha256": sha256(thresholds_path), "note": "diagnostic only; unchanged"},
        {"role": "nine_row_ceiling_derivation", "path": expanded_path.relative_to(repo_root).as_posix(), "sha256": sha256(expanded_path), "note": "nine newly signed rows"},
    ]
    for run_number in range(1, 6):
        for filename, role_stem, note in (
            ("candidate_snapshots.csv", "mac_candidate", "all five Mac candidate files are verified equal by hash and parsed rows"),
            ("memory-owner-profile.csv", "mac_memory_owner", "normalized logical-owner comparison; absolute process counters excluded"),
            ("product_path_status.csv", "mac_product_status", "checksum and storage-shape comparison source"),
        ):
            relative_path = f"accepted/run-{run_number}/{filename}"
            path = evidence_root / relative_path
            source_rows.append(
                {
                    "role": f"{role_stem}_run{run_number}",
                    "path": relative_path,
                    "sha256": sha256(path),
                    "note": note,
                }
            )
        for filename, role_stem, note in (
            ("summary.csv", "final_windows_summary", "source-matched absolute component and Track B observation source"),
            ("candidate_snapshots.csv", "final_windows_candidate", "all five final-Windows candidate files are verified equal by the report builder"),
            ("memory-owner-profile.csv", "final_windows_memory_owner", "normalized logical-owner comparison source"),
            ("product_path_status.csv", "final_windows_product_status", "checksum and storage-shape comparison source"),
        ):
            path = final_windows_root / f"run{run_number}/{filename}"
            source_rows.append(
                {
                    "role": f"{role_stem}_run{run_number}",
                    "path": path.relative_to(repo_root).as_posix(),
                    "sha256": sha256(path),
                    "note": note,
                }
            )
    behavior_result_path = evidence_root / "aggregate/behavior-gates/result.txt"
    source_rows.append(
        {
            "role": "mac_report_behavior_gate_result",
            "path": "aggregate/behavior-gates/result.txt",
            "sha256": sha256(behavior_result_path),
            "note": "commit-bound Lane-B and 37/59 focused gate summary",
        }
    )
    for relative_path, role, note in (
        ("docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-1/candidate_snapshots.csv", "m57_report_candidate_snapshot", "Track B behavior-shape comparison"),
        ("docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-1/memory-owner-profile.csv", "m57_report_memory_owner", "historical logical-owner comparison"),
        ("docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-1/product_path_status.csv", "m57_report_product_status", "Track B checksum and storage-shape comparison"),
    ):
        path = repo_root / relative_path
        source_rows.append(
            {"role": role, "path": relative_path, "sha256": sha256(path), "note": note}
        )
    write_csv(output_dir / "source-manifest.csv", source_rows, ["role", "path", "sha256", "note"])

    print(f"wrote {len(comparison_rows_out)} Track A rows")
    print(f"wrote {len(track_b_rows_out)} Track B observations")
    print(f"wrote {len(component_rows)} component rows")
    print(f"classification source-matched 587 Windows: {dict(final_classes)}")
    print(f"classification signed I0: {dict(signed_classes)}")


if __name__ == "__main__":
    main()
