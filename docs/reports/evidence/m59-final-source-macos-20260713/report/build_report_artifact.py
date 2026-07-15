#!/usr/bin/env python3
"""Build the bounded MCP technical report for the M59 Increment-4e macOS diagnostic."""

from __future__ import annotations

import argparse
import csv
import json
import sqlite3
from collections import Counter
from pathlib import Path


TITLE = "M59 Increment-4e macOS Yune vs librime performance verification"
GENERATED_AT = "2026-07-13T21:13:22Z"
YUNE_COMMIT = "5879405c7b0f76af4dca7382f00b3e0605386f2c"
LIBRIME_COMMIT = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
LONG_37 = "ceshiyixiachangjushuruxingnengzenyang"
LONG_59 = "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong"
TRACK_B = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung"
EXPLICIT_INPUTS = {"n", "ni", "hao", LONG_37, LONG_59}


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8-sig", newline="") as handle:
        return list(csv.DictReader(handle))


def number(value: str) -> int | float:
    parsed = float(value)
    return int(parsed) if parsed.is_integer() else parsed


def input_label(value: str) -> str:
    if value == LONG_37:
        return "37-character"
    if value == LONG_59:
        return "59-character"
    return value


def page_rows(
    rows: list[dict[str, str]], track: str, input_value: str, engine: str
) -> list[dict[str, str]]:
    selected = [
        row
        for row in rows
        if row["track"] == track
        and row["input"] == input_value
        and row["engine"] == engine
    ]
    return sorted(selected, key=lambda row: int(row["candidate_index"]))


def comparable_page(rows: list[dict[str, str]]) -> list[dict[str, str]]:
    return [{key: value for key, value in row.items() if key != "engine"} for row in rows]


def select_owner(
    rows: list[dict[str, str]], track: str, owner_id: str
) -> dict[str, str] | None:
    selected = [
        row
        for row in rows
        if row["track"] == track and row["owner_id"] == owner_id
    ]
    if not selected:
        return None
    values = {(row["item_count"], row["retained_estimate_bytes"]) for row in selected}
    if len(values) != 1:
        raise AssertionError(f"inconsistent owner rows for {track} / {owner_id}")
    return selected[0]


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


def write_sqlite(path: Path, datasets: dict[str, list[dict[str, object]]]) -> None:
    if path.exists():
        path.unlink()
    connection = sqlite3.connect(path)
    try:
        for table, rows in datasets.items():
            if not rows:
                continue
            columns = list(rows[0])
            for row in rows:
                if list(row) != columns:
                    raise AssertionError(f"dataset {table} has inconsistent columns")
            definitions = []
            for column in columns:
                values = [row[column] for row in rows if row[column] is not None]
                if values and all(isinstance(value, (int, bool)) for value in values):
                    data_type = "INTEGER"
                elif values and all(isinstance(value, (int, float, bool)) for value in values):
                    data_type = "REAL"
                else:
                    data_type = "TEXT"
                definitions.append(f'"{column}" {data_type}')
            connection.execute(f'CREATE TABLE "{table}" ({", ".join(definitions)})')
            placeholders = ",".join("?" for _ in columns)
            connection.executemany(
                f'INSERT INTO "{table}" VALUES ({placeholders})',
                [[row[column] for column in columns] for row in rows],
            )
        connection.commit()
    finally:
        connection.close()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--evidence-root",
        type=Path,
        required=True,
        help="external root containing accepted/ raw runs and behavior-gate evidence",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        required=True,
        help="Yune repository root containing committed comparison evidence",
    )
    parser.add_argument(
        "--analysis-root",
        type=Path,
        required=True,
        help="external directory containing regenerated analysis CSVs",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        required=True,
        help="external directory for the rebuilt report artifact",
    )
    args = parser.parse_args()
    evidence = args.evidence_root.resolve()
    repo = args.repo_root.resolve()
    aggregate = args.analysis_root.resolve()
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)

    raw_track_a = read_csv(aggregate / "track-a-17-row-comparison.csv")
    track_a: list[dict[str, object]] = []
    for order, row in enumerate(raw_track_a, 1):
        track_a.append(
            {
                "row_order": order,
                "input": row["input"],
                "input_label": input_label(row["input"]),
                "special_rows": row["special_rows"] or "standard",
                "newly_signed": row["newly_signed"],
                "run1_ratio": number(row["mac_run1_ratio"]),
                "run2_ratio": number(row["mac_run2_ratio"]),
                "run3_ratio": number(row["mac_run3_ratio"]),
                "run4_ratio": number(row["mac_run4_ratio"]),
                "run5_ratio": number(row["mac_run5_ratio"]),
                "mac_median_ratio": number(row["mac_median_ratio"]),
                "mac_worst_ratio": number(row["mac_pooled_worst_ratio"]),
                "mac_spread_pct": number(row["mac_spread_pct_rounded_1dp"]),
                "final_windows_median_ratio": number(
                    row["source_matched_final_windows_median_ratio"]
                ),
                "mac_vs_final_windows_pct": round(
                    number(row["mac_vs_source_matched_final_windows_pct"]), 1
                ),
                "final_windows_class": row[
                    "mac_vs_source_matched_final_windows_class"
                ],
                "signed_i0_windows_median_ratio": number(
                    row["historical_signed_i0_windows_median_ratio"]
                ),
                "signed_windows_ceiling": number(
                    row["historical_signed_windows_ceiling"]
                ),
                "mac_vs_signed_i0_pct": round(
                    number(row["mac_vs_historical_signed_i0_windows_pct"]), 1
                ),
                "signed_i0_class": row[
                    "mac_vs_historical_signed_i0_windows_class"
                ],
                "signed_ceiling_diagnostic": row[
                    "mac_pooled_worst_signed_ceiling_diagnostic"
                ],
                "result": "Yune faster" if number(row["mac_median_ratio"]) < 1 else "librime faster",
            }
        )

    platform_delta = sorted(
        [dict(row) for row in track_a],
        key=lambda row: float(row["mac_vs_final_windows_pct"]),
    )
    for rank, row in enumerate(platform_delta, 1):
        row["chart_order"] = rank

    raw_components = read_csv(aggregate / "track-a-component-absolute-latency.csv")
    components: list[dict[str, object]] = []
    for row in raw_components:
        if row["input"] not in EXPLICIT_INPUTS:
            continue
        components.append(
            {
                "input_order": next(
                    index
                    for index, value in enumerate(
                        ["n", "ni", "hao", LONG_37, LONG_59], 1
                    )
                    if value == row["input"]
                ),
                "input": row["input"],
                "input_label": input_label(row["input"]),
                "engine": row["engine"],
                "mac_median_us": number(row["mac_median_of_run_medians_us"]),
                "mac_worst_run_median_us": number(row["mac_worst_run_median_us"]),
                "mac_spread_pct": round(number(row["mac_spread_pct"]), 1),
                "final_windows_median_us": number(
                    row["source_matched_windows_median_of_run_medians_us"]
                ),
                "mac_vs_final_windows_pct": round(
                    number(row["mac_vs_source_matched_windows_median_pct"]), 1
                ),
                "boundary": "Absolute latency is platform-specific; diagnostic only.",
            }
        )

    raw_track_b = read_csv(aggregate / "track-b-five-observations.csv")
    track_b: list[dict[str, object]] = []
    for row in raw_track_b:
        track_b.append(
            {
                "run": int(row["run"]),
                "input": row["input"],
                "mac_median_us": number(row["mac_median_us"]),
                "mac_p95_us": number(row["mac_p95_us"]),
                "mac_p99_us": number(row["mac_p99_us"]),
                "mac_max_us": number(row["mac_max_us"]),
                "mac_median_working_set_bytes": int(
                    row["mac_median_working_set_bytes"]
                ),
                "mac_peak_working_set_bytes": int(
                    row["mac_max_peak_working_set_bytes"]
                ),
                "final_windows_median_us": number(
                    row["source_matched_windows_median_us"]
                ),
                "run_delta_pct": round(
                    number(row["mac_vs_source_matched_windows_run_median_pct"]), 1
                ),
                "mac_five_run_median_us": number(
                    row["mac_median_of_five_run_medians_us"]
                ),
                "mac_worst_run_median_us": number(
                    row["mac_worst_run_median_us"]
                ),
                "mac_pooled_worst_us": number(row["mac_pooled_worst_sample_us"]),
                "mac_spread_pct": round(number(row["mac_spread_pct"]), 1),
                "final_windows_five_run_median_us": number(
                    row["source_matched_windows_median_of_five_run_medians_us"]
                ),
                "mac_vs_final_windows_pct": round(
                    number(
                        row["mac_vs_source_matched_windows_aggregate_median_pct"]
                    ),
                    1,
                ),
                "candidate_shape": "Exact versus M57 Mac and source-matched 587 Windows",
                "boundary": "Absolute latency and memory are platform-specific.",
            }
        )

    raw_audit = read_csv(aggregate / "artifact-hash-audit.csv")
    run_audit: list[dict[str, object]] = []
    for row in raw_audit:
        run_audit.append(
            {
                "run": int(row["run"]),
                "path": f"accepted/run-{row['run']}",
                "start_utc": row["start_utc"],
                "end_utc": row["end_utc"],
                "power": "AC",
                "yune_commit": row["yune_commit"],
                "librime_commit": row["librime_commit"],
                "source_status": (
                    "pre-run clean; after-move inferred clean; transient output during"
                    if row["yune_source_status_pre_output"] == "clean"
                    and row["yune_after_move_status_ok"] == "true"
                    and row["librime_source_status"] == "clean"
                    else "source cleanliness not confirmed"
                ),
                "after_move_evidence": row["yune_after_move_status_evidence"],
                "output_protocol": row["output_location_protocol"],
                "yune_hash_stable": row["yune_pre_post_identical"],
                "librime_hash_stable": row["librime_pre_post_identical"],
                "candidate_rows": int(row["candidate_snapshot_data_rows"]),
                "track_a_rows": int(row["track_a_ratio_rows"]),
                "verdict": row["verdict"],
            }
        )

    mac_candidate_sets = [
        read_csv(evidence / f"accepted/run-{run}/candidate_snapshots.csv")
        for run in range(1, 6)
    ]
    windows_candidate_root = (
        repo
        / "docs/reports/evidence/m59-canonical-jyutping-reachability-parity"
        / "increment-4e-lane-b-exact-order/performance-ratchet"
    )
    windows_candidate_sets = [
        read_csv(windows_candidate_root / f"run{run}/candidate_snapshots.csv")
        for run in range(1, 6)
    ]
    if any(rows != mac_candidate_sets[0] for rows in mac_candidate_sets[1:]):
        raise AssertionError("parsed Mac candidate rows vary across rounds")
    if any(rows != windows_candidate_sets[0] for rows in windows_candidate_sets[1:]):
        raise AssertionError("parsed source-matched 587 Windows candidate rows vary across rounds")
    mac_candidates = mac_candidate_sets[0]
    windows_candidates = windows_candidate_sets[0]
    if mac_candidates != windows_candidates:
        raise AssertionError("parsed final Mac and source-matched 587 Windows candidate rows differ")
    candidate_summary: list[dict[str, object]] = []
    for order, input_value in enumerate(
        [str(row["input"]) for row in track_a], 1
    ):
        yune = page_rows(
            mac_candidates, "track-a-comparison", input_value, "yune"
        )
        librime = page_rows(
            mac_candidates,
            "track-a-comparison",
            input_value,
            "librime-1.17.0",
        )
        exact = comparable_page(yune) == comparable_page(librime)
        mismatch = []
        for yune_row, librime_row in zip(yune, librime):
            if {k: v for k, v in yune_row.items() if k != "engine"} != {
                k: v for k, v in librime_row.items() if k != "engine"
            }:
                mismatch.append(
                    f"{yune_row['candidate_index']}: {yune_row['text']} vs {librime_row['text']}"
                )
        candidate_summary.append(
            {
                "row_order": order,
                "input": input_value,
                "input_label": input_label(input_value),
                "yune_page": " | ".join(row["text"] for row in yune),
                "librime_page": " | ".join(row["text"] for row in librime),
                "full_page0_exact": "yes" if exact else "no",
                "mismatch": "; ".join(mismatch) if mismatch else "none",
                "mac_vs_final_windows": "exact",
                "scope": "Complete-input page 0 snapshot",
            }
        )

    m57_candidates = read_csv(
        repo
        / "docs/reports/evidence/m57-macos-track-a-sentence-model-parity"
        / "full-pass-1/candidate_snapshots.csv"
    )
    current_track_b_page = page_rows(
        mac_candidates, "track-b-product", TRACK_B, "yune"
    )
    m57_track_b_page = page_rows(m57_candidates, "track-b-product", TRACK_B, "yune")
    if comparable_page(current_track_b_page) != comparable_page(m57_track_b_page):
        raise AssertionError("Track B page differs from M57")

    current_owner_sets = [
        read_csv(evidence / f"accepted/run-{run}/memory-owner-profile.csv")
        for run in range(1, 6)
    ]
    windows_owner_sets = [
        read_csv(windows_candidate_root / f"run{run}/memory-owner-profile.csv")
        for run in range(1, 6)
    ]
    normalized_owner_sets = [
        normalized_owner_profile(rows)
        for rows in current_owner_sets + windows_owner_sets
    ]
    if any(len(rows) != 99 for rows in normalized_owner_sets):
        raise AssertionError("normalized logical-owner profile does not have 99 rows")
    if any(rows != normalized_owner_sets[0] for rows in normalized_owner_sets[1:]):
        raise AssertionError("normalized Mac/Windows logical-owner profiles differ")
    current_owner = current_owner_sets[0]
    windows_owner = windows_owner_sets[0]
    m57_owner = read_csv(
        repo
        / "docs/reports/evidence/m57-macos-track-a-sentence-model-parity"
        / "full-pass-1/memory-owner-profile.csv"
    )
    owner_specs = [
        ("poet.entries_by_code", "Luna sentence entries"),
        ("poet.lookup_index", "Luna lookup index"),
        ("poet.vocabulary", "Luna general vocabulary"),
        ("poet.abbreviation_vocabulary", "Luna abbreviation vocabulary"),
        ("poet.normal_character_code_index", "Luna normal-character index"),
    ]
    model_owner: list[dict[str, object]] = []
    for order, (owner_id, label) in enumerate(owner_specs, 1):
        mac = select_owner(current_owner, "track-a-comparison", owner_id)
        win = select_owner(windows_owner, "track-a-comparison", owner_id)
        old = select_owner(m57_owner, "track-a-comparison", owner_id)
        if mac is None or win is None:
            raise AssertionError(f"missing current owner {owner_id}")
        if mac["item_count"] != win["item_count"]:
            raise AssertionError(f"Mac/Windows owner count differs for {owner_id}")
        model_owner.append(
            {
                "row_order": order,
                "measure": label,
                "current_mac": int(mac["item_count"]),
                "final_windows": int(win["item_count"]),
                "m57_mac": int(old["item_count"]) if old else 0,
                "current_mac_windows_exact": "yes",
                "interpretation": (
                    "M57 did not report this owner."
                    if old is None
                    else "Source-state comparison; profile schema changed."
                ),
            }
        )

    mac_product_sets = [
        read_csv(evidence / f"accepted/run-{run}/product_path_status.csv")
        for run in range(1, 6)
    ]
    win_product_sets = [
        read_csv(windows_candidate_root / f"run{run}/product_path_status.csv")
        for run in range(1, 6)
    ]
    m57_product = read_csv(
        repo
        / "docs/reports/evidence/m57-macos-track-a-sentence-model-parity"
        / "full-pass-1/product_path_status.csv"
    )
    track_b_shapes = [
        track_b_product_shape(rows)
        for rows in mac_product_sets + win_product_sets + [m57_product]
    ]
    if any(len(rows) != 2 for rows in track_b_shapes):
        raise AssertionError("Track B product shape does not contain two dictionary rows")
    if any(rows != track_b_shapes[0] for rows in track_b_shapes[1:]):
        raise AssertionError("Mac/Windows/M57 Track B checksum or storage shape differs")
    mac_product = mac_product_sets[0]
    win_product = win_product_sets[0]
    mac_luna = next(
        row for row in mac_product if row["track"] == "track-a-comparison"
    )
    win_luna = next(
        row for row in win_product if row["track"] == "track-a-comparison"
    )
    model_owner.insert(
        0,
        {
            "row_order": 0,
            "measure": "Luna source / table checksum",
            "current_mac": f"{mac_luna['source_checksum']} / {mac_luna['table_checksum']}",
            "final_windows": f"{win_luna['source_checksum']} / {win_luna['table_checksum']}",
            "m57_mac": "0xb3d4e98e / 0x29d56c89",
            "current_mac_windows_exact": "target-specific accepted pairs",
            "interpretation": "Known platform build artifact; storage mode and logical counts match.",
        },
    )
    model_owner.append(
        {
            "row_order": 6,
            "measure": "Track B POET entries",
            "current_mac": 0,
            "final_windows": 0,
            "m57_mac": 0,
            "current_mac_windows_exact": "yes",
            "interpretation": "No upstream sentence model is configured for Track B.",
        }
    )

    classes = Counter(str(row["final_windows_class"]) for row in track_a)
    by_input = {str(row["input"]): row for row in track_a}
    headline = [
        {
            "track_a_rows": 17,
            "yune_faster_rows": sum(row["result"] == "Yune faster" for row in track_a),
            "librime_faster_rows": sum(
                row["result"] == "librime faster" for row in track_a
            ),
            "close_rows": classes["close"],
            "notable_rows": classes["notable"],
            "material_rows": classes["material"],
            "ratio_37": by_input[LONG_37]["mac_median_ratio"],
            "delta_37_pct": by_input[LONG_37]["mac_vs_final_windows_pct"],
            "ratio_59": by_input[LONG_59]["mac_median_ratio"],
            "delta_59_pct": by_input[LONG_59]["mac_vs_final_windows_pct"],
            "track_b_median_us": track_b[0]["mac_five_run_median_us"],
            "track_b_delta_pct": track_b[0]["mac_vs_final_windows_pct"],
            "stable_binary_rounds": 5,
            "candidate_exact_inputs": sum(
                row["full_page0_exact"] == "yes" for row in candidate_summary
            ),
        }
    ]

    behavior_result = (
        evidence / "aggregate/behavior-gates/result.txt"
    ).read_text(encoding="utf-8")
    if f"Yune commit: {YUNE_COMMIT}" not in behavior_result:
        raise AssertionError("behavior-gate result has the wrong Yune commit")
    if behavior_result.count("result: ok; 1 passed, 0 failed") != 2:
        raise AssertionError("behavior-gate result does not contain two passing gates")
    for test_name in (
        "m59_lane_b_product_matches_complete_pinned_librime_order",
        "m59_luna_long_sentence_page_order_matches_pinned_oracle_on_byte_backed_product",
    ):
        if test_name not in behavior_result:
            raise AssertionError(f"behavior-gate result lacks {test_name}")

    behavior_gates = [
        {
            "gate": "Complete Lane-B pinned order",
            "test": "m59_lane_b_product_matches_complete_pinned_librime_order",
            "result": "1 passed / 0 failed",
            "coverage": "7 inputs; 430 pages; 2,135 candidates; every captured page",
            "elapsed_seconds": 9.09,
            "commit": YUNE_COMMIT,
        },
        {
            "gate": "37/59 page order and recomposition",
            "test": "m59_luna_long_sentence_page_order_matches_pinned_oracle_on_byte_backed_product",
            "result": "1 passed / 0 failed",
            "coverage": "Deployed page 0 plus partial selection/recomposition",
            "elapsed_seconds": 15.52,
            "commit": YUNE_COMMIT,
        },
    ]

    datasets = {
        "headline": headline,
        "track_a": track_a,
        "platform_delta": platform_delta,
        "components": components,
        "track_b": track_b,
        "candidate_summary": candidate_summary,
        "model_owner": model_owner,
        "run_audit": run_audit,
        "behavior_gates": behavior_gates,
    }
    write_sqlite(output / "snapshot.sqlite", datasets)

    source_specs = {
        "headline_sql": (
            "Report headline metrics",
            "SELECT * FROM headline",
            ["headline"],
            ["One derived row from the validated Increment-4e source-bound aggregation."],
            [
                "A Yune/librime ratio below 1 means Yune is faster.",
                "Close means absolute Mac-vs-source-matched 587 Windows ratio difference <=10%; notable >10% and <=25%; material >25%.",
            ],
            [
                "analysis/track-a-17-row-comparison.csv",
                "analysis/track-b-five-observations.csv",
            ],
        ),
        "track_a_sql": (
            "Complete 17-row ratio comparison",
            "SELECT * FROM track_a ORDER BY row_order",
            ["track_a"],
            [
                "Exactly 17 Track A inputs; five complete Mac rounds; no measured round removed."
            ],
            [
                "Mac median is the third sorted ratio; worst is max; spread is (max-min)/min.",
                "Signed Windows ceilings are diagnostic only and are not portable Mac acceptance limits.",
            ],
            ["analysis/track-a-17-row-comparison.csv"],
        ),
        "platform_delta_sql": (
            "Source-matched cross-platform ratio difference",
            "SELECT * FROM platform_delta ORDER BY mac_vs_final_windows_pct",
            ["platform_delta"],
            [
                "Source commit, inputs, iteration counts, and product deployment match across platforms; the Mac output-location deviation is disclosed."
            ],
            [
                "Difference is (Mac median ratio / source-matched 587 Windows median ratio - 1) * 100.",
                "Negative means the Yune/librime ratio is lower on Mac; positive means it is higher.",
            ],
            [
                "analysis/track-a-17-row-comparison.csv",
                "docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-4e-lane-b-exact-order/performance-ratchet/gate-verdict.csv",
            ],
        ),
        "components_sql": (
            "Absolute component scaling for explicit rows",
            "SELECT * FROM components ORDER BY input_order, engine",
            ["components"],
            ["n, ni, hao, 37-character, and 59-character inputs; both engines."],
            [
                "Each platform value is the median of five run medians in microseconds per key.",
                "Absolute cross-platform latency is diagnostic and platform-specific.",
            ],
            ["analysis/track-a-component-absolute-latency.csv"],
        ),
        "track_b_sql": (
            "Track B product observations",
            "SELECT * FROM track_b ORDER BY run",
            ["track_b"],
            ["Five complete Mac observations for the exact product input."],
            [
                "Track B median is the third sorted run median.",
                "Absolute Mac/Windows latency and memory counters are diagnostic only.",
            ],
            ["analysis/track-b-five-observations.csv"],
        ),
        "candidate_sql": (
            "Complete-input page-zero candidate comparison",
            "SELECT * FROM candidate_summary ORDER BY row_order",
            ["candidate_summary"],
            ["Track A complete-input page 0 only; not every prefix or later page."],
            [
                "Full exactness compares page geometry, preedit, candidate text, and comments.",
                "Parsed Mac candidate evidence is identical to source-matched 587 Windows.",
            ],
            [
                *[
                    f"accepted/run-{run}/candidate_snapshots.csv"
                    for run in range(1, 6)
                ],
                *[
                    "docs/reports/evidence/m59-canonical-jyutping-reachability-parity/"
                    f"increment-4e-lane-b-exact-order/performance-ratchet/run{run}/candidate_snapshots.csv"
                    for run in range(1, 6)
                ],
                "docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-1/candidate_snapshots.csv",
            ],
        ),
        "model_sql": (
            "Model-owner and checksum comparison",
            "SELECT * FROM model_owner ORDER BY row_order",
            ["model_owner"],
            [
                "Logical owners only; process.* and path/metadata-bearing schema.config rows are excluded, and session_id is removed."
            ],
            [
                "All 99 remaining owner rows are exact across all five Mac and all five source-matched 587 Windows files; the table displays named decision-bearing owners.",
                "Mac and Windows checksum pairs are target-specific accepted build artifacts.",
                "Track B checksum/storage/mapping fields are exact across all five Mac files, all five source-matched 587 Windows files, and M57 full-pass-1.",
            ],
            [
                *[
                    f"accepted/run-{run}/memory-owner-profile.csv"
                    for run in range(1, 6)
                ],
                *[
                    f"accepted/run-{run}/product_path_status.csv"
                    for run in range(1, 6)
                ],
                *[
                    "docs/reports/evidence/m59-canonical-jyutping-reachability-parity/"
                    f"increment-4e-lane-b-exact-order/performance-ratchet/run{run}/memory-owner-profile.csv"
                    for run in range(1, 6)
                ],
                *[
                    "docs/reports/evidence/m59-canonical-jyutping-reachability-parity/"
                    f"increment-4e-lane-b-exact-order/performance-ratchet/run{run}/product_path_status.csv"
                    for run in range(1, 6)
                ],
                "docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-1/memory-owner-profile.csv",
                "docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-1/product_path_status.csv",
            ],
        ),
        "run_audit_sql": (
            "Five-run provenance and hash audit",
            "SELECT * FROM run_audit ORDER BY run",
            ["run_audit"],
            ["Five measured logical rounds under AC power; all are retained."],
            [
                "Pre-run Yune source cleanliness is directly captured. After-move cleanliness is inferred from the next preflight for runs 1–4 and the later behavior-gate preflight for run 5. During measurement only the transient untracked evidence directory appeared. Each round has 17 Track A rows, 175 candidate rows, and identical pre/post binary hashes."
            ],
            ["analysis/artifact-hash-audit.csv"],
        ),
        "behavior_sql": (
            "Focused behavior gate results",
            "SELECT * FROM behavior_gates",
            ["behavior_gates"],
            ["Two commit-bound focused gates selected by the efficient diagnostic sequence."],
            [
                "Pass counts are copied from complete cargo test logs and independently checked."
            ],
            ["analysis/behavior-gates/result.txt"],
        ),
    }
    sources = []
    manifest_sources = []
    for source_id, (
        label,
        sql,
        tables,
        filters,
        metric_definitions,
        source_files,
    ) in source_specs.items():
        manifest_sources.append(
            {"id": source_id, "label": label, "path": "report/snapshot.sqlite"}
        )
        sources.append(
            {
                "id": source_id,
                "label": label,
                "path": "report/snapshot.sqlite",
                "query": {
                    "engine": "sqlite",
                    "language": "sql",
                    "sql": sql,
                    "description": label,
                    "tables_used": tables,
                    "filters": filters,
                    "metric_definitions": metric_definitions,
                    "source_files": source_files,
                    "executed_at": GENERATED_AT,
                },
            }
        )

    cards = [
        {
            "id": "track_a_card",
            "description": "Increment-4e source-bound Mac median-of-five result for all Track A inputs.",
            "dataset": "headline",
            "sourceId": "headline_sql",
            "metrics": [
                {
                    "label": "Yune-faster rows",
                    "field": "yune_faster_rows",
                    "format": "number",
                },
                {
                    "label": "librime-faster rows",
                    "field": "librime_faster_rows",
                    "format": "number",
                },
            ],
        },
        {
            "id": "classification_card",
            "description": "Requested diagnostic classes versus source-matched 587 Windows.",
            "dataset": "headline",
            "sourceId": "headline_sql",
            "metrics": [
                {
                    "label": "Material rows",
                    "field": "material_rows",
                    "format": "number",
                },
                {"label": "Close", "field": "close_rows", "format": "number"},
                {
                    "label": "Notable",
                    "field": "notable_rows",
                    "format": "number",
                },
            ],
        },
        {
            "id": "long_37_card",
            "description": "Increment-4e source-bound Mac ratio; lower than 1 favors Yune.",
            "dataset": "headline",
            "sourceId": "headline_sql",
            "metrics": [
                {
                    "label": "37-char Mac ratio",
                    "field": "ratio_37",
                    "format": "number",
                },
                {
                    "label": "vs source-matched 587 Windows, %",
                    "field": "delta_37_pct",
                    "format": "number",
                    "signed": True,
                },
            ],
        },
        {
            "id": "long_59_card",
            "description": "Increment-4e source-bound Mac ratio; lower than 1 favors Yune.",
            "dataset": "headline",
            "sourceId": "headline_sql",
            "metrics": [
                {
                    "label": "59-char Mac ratio",
                    "field": "ratio_59",
                    "format": "number",
                },
                {
                    "label": "vs source-matched 587 Windows, %",
                    "field": "delta_59_pct",
                    "format": "number",
                    "signed": True,
                },
            ],
        },
        {
            "id": "track_b_card",
            "description": "Absolute Mac product-guard value; not a Windows acceptance metric.",
            "dataset": "headline",
            "sourceId": "headline_sql",
            "metrics": [
                {
                    "label": "Track B median, µs/key",
                    "field": "track_b_median_us",
                    "format": "number",
                },
                {
                    "label": "vs source-matched 587 Windows, %",
                    "field": "track_b_delta_pct",
                    "format": "number",
                    "signed": True,
                },
            ],
        },
    ]

    charts = [
        {
            "id": "platform_delta_chart",
            "title": "macOS versus source-matched 587 Windows Yune/librime ratio difference",
            "subtitle": "n, ni, and hao favor macOS; 37/59 show stronger librime scaling, while every Mac ratio remains below 1.0×.",
            "intent": "comparison",
            "question": "Which Increment-4e source-bound Yune/librime ratios differ most between macOS and Windows?",
            "rationale": "A signed horizontal comparison preserves all 17 labels and makes the zero reference explicit.",
            "comparisonContext": {
                "baseline": "0% means equal Yune/librime ratios",
                "denominator": "source-matched 587 Windows ratio",
                "grain": "17 Track A inputs; median of five ratios per platform",
                "unit": "percent difference",
            },
            "type": "horizontalBar",
            "dataset": "platform_delta",
            "sourceId": "platform_delta_sql",
            "encodings": {
                "x": {
                    "field": "input_label",
                    "type": "nominal",
                    "label": "Input",
                },
                "y": {
                    "field": "mac_vs_final_windows_pct",
                    "type": "quantitative",
                    "label": "Mac versus source-matched 587 Windows, %",
                },
            },
            "palette": {"kind": "sequential"},
            "labels": {"values": "auto"},
            "referenceLines": [
                {
                    "axis": "y",
                    "value": 0,
                    "color": "neutral",
                    "label": "Same ratio",
                }
            ],
            "settings": {
                "orientation": "horizontal",
                "sort": "none",
                "categoryLabelPolicy": "wrap",
            },
            "valueFormat": "number",
            "layout": "full",
            "maxRows": 17,
        }
    ]

    tables = [
        {
            "id": "track_a_table",
            "title": "Complete 17-row Increment-4e source-bound comparison",
            "subtitle": "Five retained Mac ratios, source-matched 587 Windows, and historical signed-ceiling diagnostics",
            "dataset": "track_a",
            "sourceId": "track_a_sql",
            "defaultSort": {"field": "row_order", "direction": "asc"},
            "density": "compact",
            "layout": "full",
            "columns": [
                {"field": "row_order", "label": "#", "type": "number"},
                {"field": "input_label", "label": "Input", "type": "text"},
                {"field": "input", "label": "Exact input", "type": "text"},
                {"field": "newly_signed", "label": "New M59 row", "type": "text"},
                {"field": "run1_ratio", "label": "Run 1", "type": "number", "unit": "×"},
                {"field": "run2_ratio", "label": "Run 2", "type": "number", "unit": "×"},
                {"field": "run3_ratio", "label": "Run 3", "type": "number", "unit": "×"},
                {"field": "run4_ratio", "label": "Run 4", "type": "number", "unit": "×"},
                {"field": "run5_ratio", "label": "Run 5", "type": "number", "unit": "×"},
                {
                    "field": "mac_median_ratio",
                    "label": "Mac median",
                    "type": "number",
                    "unit": "×",
                },
                {
                    "field": "mac_worst_ratio",
                    "label": "Mac worst",
                    "type": "number",
                    "unit": "×",
                },
                {
                    "field": "mac_spread_pct",
                    "label": "Spread",
                    "type": "number",
                    "unit": "%",
                },
                {
                    "field": "final_windows_median_ratio",
                    "label": "587 Win median",
                    "type": "number",
                    "unit": "×",
                },
                {
                    "field": "mac_vs_final_windows_pct",
                    "label": "Mac vs final Win",
                    "type": "number",
                    "unit": "%",
                },
                {
                    "field": "final_windows_class",
                    "label": "Class",
                    "type": "text",
                },
                {
                    "field": "signed_i0_windows_median_ratio",
                    "label": "Signed I0 median",
                    "type": "number",
                    "unit": "×",
                },
                {
                    "field": "signed_windows_ceiling",
                    "label": "Signed ceiling",
                    "type": "number",
                    "unit": "×",
                },
                {
                    "field": "mac_vs_signed_i0_pct",
                    "label": "Mac vs signed I0",
                    "type": "number",
                    "unit": "%",
                },
                {
                    "field": "signed_ceiling_diagnostic",
                    "label": "Ceiling diagnostic",
                    "type": "text",
                },
            ],
        },
        {
            "id": "component_table",
            "title": "Absolute component scaling on explicit rows",
            "subtitle": "Both engines move across platforms; absolute microseconds are diagnostic only",
            "dataset": "components",
            "sourceId": "components_sql",
            "defaultSort": {"field": "input_order", "direction": "asc"},
            "density": "compact",
            "layout": "full",
            "columns": [
                {"field": "input_order", "label": "#", "type": "number"},
                {"field": "input_label", "label": "Input", "type": "text"},
                {"field": "engine", "label": "Engine", "type": "text"},
                {
                    "field": "mac_median_us",
                    "label": "Mac median",
                    "type": "number",
                    "unit": "µs/key",
                },
                {
                    "field": "final_windows_median_us",
                    "label": "587 Win median",
                    "type": "number",
                    "unit": "µs/key",
                },
                {
                    "field": "mac_vs_final_windows_pct",
                    "label": "Mac vs final Win",
                    "type": "number",
                    "unit": "%",
                },
                {
                    "field": "mac_spread_pct",
                    "label": "Mac spread",
                    "type": "number",
                    "unit": "%",
                },
            ],
        },
        {
            "id": "track_b_table",
            "title": "Track B product input: five Mac observations",
            "subtitle": "Candidate page remains exact versus M57 and source-matched 587 Windows",
            "dataset": "track_b",
            "sourceId": "track_b_sql",
            "defaultSort": {"field": "run", "direction": "asc"},
            "density": "compact",
            "layout": "full",
            "columns": [
                {"field": "run", "label": "Run", "type": "number"},
                {
                    "field": "mac_median_us",
                    "label": "Mac median",
                    "type": "number",
                    "unit": "µs/key",
                },
                {
                    "field": "mac_p95_us",
                    "label": "Mac p95",
                    "type": "number",
                    "unit": "µs/key",
                },
                {
                    "field": "mac_max_us",
                    "label": "Mac max",
                    "type": "number",
                    "unit": "µs/key",
                },
                {
                    "field": "final_windows_median_us",
                    "label": "587 Win median",
                    "type": "number",
                    "unit": "µs/key",
                },
                {
                    "field": "run_delta_pct",
                    "label": "Mac vs final Win",
                    "type": "number",
                    "unit": "%",
                },
                {
                    "field": "mac_median_working_set_bytes",
                    "label": "Mac RSS",
                    "type": "number",
                    "unit": "B",
                },
                {
                    "field": "mac_peak_working_set_bytes",
                    "label": "Mac peak RSS",
                    "type": "number",
                    "unit": "B",
                },
            ],
        },
        {
            "id": "candidate_table",
            "title": "Complete-input page-zero candidate comparison",
            "subtitle": "587 Mac parsed snapshots equal source-matched 587 Windows; 16/17 Track A pages equal librime",
            "dataset": "candidate_summary",
            "sourceId": "candidate_sql",
            "defaultSort": {"field": "row_order", "direction": "asc"},
            "density": "compact",
            "layout": "full",
            "columns": [
                {"field": "row_order", "label": "#", "type": "number"},
                {"field": "input_label", "label": "Input", "type": "text"},
                {"field": "yune_page", "label": "Yune page", "type": "text"},
                {"field": "librime_page", "label": "librime page", "type": "text"},
                {
                    "field": "full_page0_exact",
                    "label": "Full page exact",
                    "type": "text",
                },
                {"field": "mismatch", "label": "Mismatch", "type": "text"},
                {
                    "field": "mac_vs_final_windows",
                    "label": "Mac vs final Win",
                    "type": "text",
                },
            ],
        },
        {
            "id": "model_table",
            "title": "Candidate-model and logical-owner shape",
            "subtitle": "Logical counts match source-matched 587 Windows; M57 is a source-state comparison",
            "dataset": "model_owner",
            "sourceId": "model_sql",
            "defaultSort": {"field": "row_order", "direction": "asc"},
            "density": "compact",
            "layout": "full",
            "columns": [
                {"field": "row_order", "label": "#", "type": "number"},
                {"field": "measure", "label": "Measure", "type": "text"},
                {"field": "current_mac", "label": "Current Mac", "type": "text"},
                {
                    "field": "final_windows",
                    "label": "587 Windows",
                    "type": "text",
                },
                {"field": "m57_mac", "label": "M57 Mac", "type": "text"},
                {
                    "field": "current_mac_windows_exact",
                    "label": "Current cross-platform",
                    "type": "text",
                },
                {
                    "field": "interpretation",
                    "label": "Interpretation",
                    "type": "text",
                },
            ],
        },
        {
            "id": "run_audit_table",
            "title": "Five-run provenance and fixed-binary audit",
            "subtitle": "Every complete logical round is retained",
            "dataset": "run_audit",
            "sourceId": "run_audit_sql",
            "defaultSort": {"field": "run", "direction": "asc"},
            "density": "compact",
            "layout": "full",
            "columns": [
                {"field": "run", "label": "Run", "type": "number"},
                {"field": "path", "label": "External path", "type": "text"},
                {"field": "start_utc", "label": "Start UTC", "type": "text"},
                {"field": "end_utc", "label": "End UTC", "type": "text"},
                {"field": "power", "label": "Power", "type": "text"},
                {"field": "source_status", "label": "Sources", "type": "text"},
                {
                    "field": "output_protocol",
                    "label": "Output location",
                    "type": "text",
                },
                {
                    "field": "after_move_evidence",
                    "label": "After-move status basis",
                    "type": "text",
                },
                {
                    "field": "yune_hash_stable",
                    "label": "Yune hash",
                    "type": "text",
                },
                {
                    "field": "librime_hash_stable",
                    "label": "librime hash",
                    "type": "text",
                },
                {
                    "field": "candidate_rows",
                    "label": "Candidate rows",
                    "type": "number",
                },
                {"field": "verdict", "label": "Verdict", "type": "text"},
            ],
        },
        {
            "id": "behavior_table",
            "title": "Commit-bound focused behavior gates",
            "subtitle": "Both load-bearing checks pass at the measured Yune commit",
            "dataset": "behavior_gates",
            "sourceId": "behavior_sql",
            "defaultSort": {"field": "gate", "direction": "asc"},
            "density": "compact",
            "layout": "full",
            "columns": [
                {"field": "gate", "label": "Gate", "type": "text"},
                {"field": "result", "label": "Result", "type": "text"},
                {"field": "coverage", "label": "Coverage", "type": "text"},
                {
                    "field": "elapsed_seconds",
                    "label": "Elapsed",
                    "type": "number",
                    "unit": "s",
                },
                {"field": "commit", "label": "Commit", "type": "text"},
            ],
        },
    ]

    blocks = [
        {"id": "title", "type": "markdown", "body": f"# {TITLE}"},
        {
            "id": "technical_summary",
            "type": "markdown",
            "body": (
                "## Technical Summary\n\n"
                f"At M59 Increment-4e engine source {YUNE_COMMIT}, the old macOS performance deficit does not reproduce. "
                "Yune is faster than pinned librime on all 17 Track A median ratios, from 0.006× to 0.471×. "
                "The earlier afb7079b Mac packet, where Yune won only 6/17 aggregate rows and had behavior-confounded long-input timing, is historical and is superseded for performance at 5879405c. "
                "Final M59 behavior source 443cc636 postdates this Mac capture, and current main contains later WEB-03 engine changes through 7f758fba. No Mac rerun at either later source is claimed.\n\n"
                "Against the source-matched 587 Windows packet, the requested diagnostic labels are 7 close, 6 notable, and 4 material. "
                "The four material rows are n, ni, and hao in Mac's favor, plus the 37-character ratio in Windows' favor. "
                "These are ratio differences, not failures: every Mac median and pooled worst remains below 1.0× and below the signed Windows ceiling diagnostically.\n\n"
                "Behavior and logical model shape do not indicate a Mac-only engine defect. Parsed candidate evidence is identical to source-matched 587 Windows; 16/17 complete-input Track A page-zero snapshots match librime. "
                "The sole mismatch, zhongdengchangdu positions 2–4, is identical on Windows. Both focused commit-bound behavior gates pass, including all captured Lane-B pages and deployed 37/59 page order/recomposition.\n\n"
                "One literal protocol requirement was not met: the unmodified benchmark script only accepts an output root under the repository, so each round first wrote an untracked transient evidence directory in the disposable worktree and was moved external after completion. Pre-run source cleanliness is directly captured; after-move cleanliness is inferred from the next preflight for runs 1–4 and the later behavior-gate preflight for run 5. Both dylibs stayed fixed, but this packet is diagnostic evidence with a disclosed output-location deviation, not a fully protocol-conforming acceptance packet.\n\n"
                "The evidence therefore supports a real Increment-4e source-bound Yune advantage on macOS and does not identify a continuing macOS engine-path discrepancy. "
                "Residual cross-platform movement is consistent with combined platform/toolchain/allocator/scheduler component scaling plus measurement noise; the evidence does not isolate their shares. No performance fix is authorized or indicated by this diagnostic."
            ),
        },
        {
            "id": "headline_metrics",
            "type": "metric-strip",
            "cardIds": [
                "track_a_card",
                "classification_card",
                "long_37_card",
                "long_59_card",
                "track_b_card",
            ],
        },
        {
            "id": "key_findings",
            "type": "markdown",
            "body": (
                "## Key Findings\n\n"
                "The cross-platform ratios are not moving as one block. On n, ni, and hao, Yune's Mac absolute latency is about 31–34% lower than Windows while librime is roughly flat or slower, so the ratio materially favors Mac. "
                "On the 37/59 sequences both engines are faster on Mac, but librime scales more strongly, making the Mac ratio 31.8% and 20.0% higher than source-matched 587 Windows even though Yune remains about 34× and 83× faster in the measured driver.\n\n"
                "Across all 17 inputs, the median absolute Mac-versus-Windows change is -30.9% for Yune and -31.2% for librime. "
                "That symmetry is evidence against a single librime-only explanation. The row-level shape instead points to workload-dependent native-library and platform scaling."
            ),
        },
        {
            "id": "platform_delta_chart_block",
            "type": "chart",
            "chartId": "platform_delta_chart",
            "layout": "full",
        },
        {
            "id": "track_a_table_block",
            "type": "table",
            "tableId": "track_a_table",
            "layout": "full",
        },
        {
            "id": "long_findings",
            "type": "markdown",
            "body": (
                "## 37/59 Findings\n\n"
                "The 37-character ratios are 0.029, 0.029, 0.026, 0.029, and 0.029; median and worst are both 0.029×. "
                "The 59-character ratios are 0.013, 0.012, 0.012, 0.012, and 0.015; median 0.012× and worst 0.015×. "
                "Unlike the historical Mac packet, these values are no longer contradicted by the repaired final page: both page-zero snapshots exactly match librime's one-sentence-then-phrases shape, and the focused recomposition gate passes.\n\n"
                "The benchmark still does not capture every prefix/page for all 17 inputs. The complete Lane-B fixture supplies all captured pages for its seven Luna inputs, while the 37/59 gate supplies deployed page-zero and partial-selection coverage. "
                "The result is strong evidence for the named M59 surfaces, not a universal every-prefix parity claim."
            ),
        },
        {
            "id": "component_table_block",
            "type": "table",
            "tableId": "component_table",
            "layout": "full",
        },
        {
            "id": "track_b_findings",
            "type": "markdown",
            "body": (
                "## Track B Product Input\n\n"
                "The five Mac medians are 12.352, 12.491, 12.936, 12.811, and 14.480 µs/key. "
                "Median is 12.811, worst run median 14.480, pooled worst sample 16.296, and spread 17.2%. "
                "The source-matched 587 Windows median is 17.177 µs/key, so the Mac median is 25.4% lower; that absolute comparison is platform-specific and is not a portable threshold.\n\n"
                "Candidate text, comments, geometry, page state, checksums, and absent/shared-zero POET status match M57 and source-matched 587 Windows. "
                "The elevated fifth observation and larger Mac RSS reinforce the noise/allocator/counter boundary rather than a behavioral path split."
            ),
        },
        {
            "id": "track_b_table_block",
            "type": "table",
            "tableId": "track_b_table",
            "layout": "full",
        },
        {
            "id": "candidate_model_findings",
            "type": "markdown",
            "body": (
                "## Candidate and Model-Owner Differences\n\n"
                "All five Mac candidate files are byte-identical, and their parsed rows are exactly identical to all five source-matched 587 Windows files. "
                "Track A page-zero behavior is exact on 16/17 inputs. The only mismatch is zhongdengchangdu positions 2–4: Yune emits 中的 / 種的 / 重的 while librime emits 中 / 種 / 重. "
                "Because source-matched 587 Windows has the same mismatch, it is a cross-platform engine behavior gap, not a Mac defect.\n\n"
                "The accepted Mac Luna checksum pair remains 0xb3d4e98e / 0x29d56c89; Windows has its independently accepted 0x16ad0e3e / 0xb967cfef pair. "
                "Logical shape is otherwise exact: 513,353 sentence entries, 332,604 lookup-index rows, vocabulary 193, abbreviation vocabulary 11, and normal-character index 423. "
                "Excluding process counters and the tiny path/metadata-bearing schema.config row, Mac and Windows logical owner profiles are exact. Absolute RSS/private/pagefile counters are not interchangeable."
            ),
        },
        {
            "id": "candidate_table_block",
            "type": "table",
            "tableId": "candidate_table",
            "layout": "full",
        },
        {
            "id": "model_table_block",
            "type": "table",
            "tableId": "model_table",
            "layout": "full",
        },
        {
            "id": "root_cause",
            "type": "markdown",
            "body": (
                "## Why the Old macOS Deficit Does Not Reproduce\n\n"
                "The strongest evidence points to source state rather than a mysterious post-M59 operating-system fix, but the measured commit range does not isolate a causal share. Increment 4e replaced the un-toned Luna path with the structure-driven UpstreamScript surface graph: one common-prefix prism walk per live vertex, pinned table-trunk/tail traversal, bounded sentence scratch reuse, and page-local position separated from outer merge quality. "
                "That measured mechanism coincides with Yune's Mac latency falling from tens or hundreds of microseconds per key in the historical packet to roughly 3–6 µs/key here. The commit range changes interacting owners, so the evidence does not assign the gain to one line or one optimization.\n\n"
                "librime remains valuable as a design oracle: demand-driven candidate streams, resumable state, and bounded page work are still useful patterns. "
                "But the Increment-4e source-bound measurement no longer presents a Mac latency deficit requiring a speculative fix. Memory ownership and any future Apple footprint work should remain separate diagnostics."
            ),
        },
        {
            "id": "scope",
            "type": "markdown",
            "body": (
                "## Scope, Data, and Definitions\n\n"
                f"Measured Yune tracked source was directly recorded clean and detached at {YUNE_COMMIT} before every round; after-move cleanliness is inferred from subsequent clean preflights. Oracle was clean and detached at librime {LIBRIME_COMMIT}. During measurement, Git reported only the transient untracked evidence directory. "
                "All five runs use 9 startup iterations, 60 session iterations, 80 key iterations, product deployment, the exact 17 Track A inputs, and the exact Track B input. "
                "Ratio means Yune median µs/key divided by librime median µs/key; below 1 favors Yune. Median is the third sorted observation, worst is the largest, and spread is (max-min)/min.\n\n"
                "The 587 Windows comparison uses the same Yune source, inputs, and benchmark shape. The signed Increment-0 packet and retained ceilings are historical diagnostics only. "
                "Close/notable/material labels describe absolute Mac-vs-Windows ratio difference of <=10%, >10–25%, and >25%. They are not acceptance thresholds."
            ),
        },
        {
            "id": "run_audit_block",
            "type": "table",
            "tableId": "run_audit_table",
            "layout": "full",
        },
        {
            "id": "methodology",
            "type": "markdown",
            "body": (
                "## Methodology\n\n"
                "The Yune release dylib was built once before measurement. The official macOS in-process benchmark script was then invoked for five logical rounds. Its per-round `cargo build --release` check was a no-op for the prebuilt measured dylib (0.08–0.09 seconds), and every pre/post dylib hash was checked. The script's `cargo bench` calls did compile the Yune crate and benchmark harness for roughly 24–30 seconds before each deploy, Track A Yune, Track A librime, and Track B lane. Those builds were sequential rather than concurrent with the lane process; measurement loaded pre-copied, hash-fixed Yune/librime dylibs. "
                "Because the unmodified script enforces an output root under the repository, each run wrote first to an untracked transient directory in the disposable worktree and was moved outside only after completion. This violates the literal all-generated-output-external requirement even though it never changed tracked source. No measured red or noisy round was removed.\n\n"
                "Aggregation joins rows by kind, workload, input, and metric rather than file order. It independently recomputes Windows gate formulas, validates all 17 ceiling joins and all nine expanded derivations, verifies the exact commits and measurement parameters, and fails closed on missing artifacts, unexpected measurement-time Git status, or variable binaries. "
                "Candidate rows are normalized before cross-platform comparison. Logical owner rows are compared separately from process-level counters."
            ),
        },
        {
            "id": "behavior_table_block",
            "type": "table",
            "tableId": "behavior_table",
            "layout": "full",
        },
        {
            "id": "limitations",
            "type": "markdown",
            "body": (
                "## Limitations and Robustness\n\n"
                "The machine stayed on AC with Low Power Mode disabled and recorded no thermal or performance warning. A setup warmup was explicitly excluded before measured data. "
                "There was a long user pause between rounds 2 and 3, and transient Chrome, Claude, duetexpertd, CloudKit, and UI activity delayed later starts. Round 5 is visibly noisier; it is retained. "
                "Quiet-machine state was not continuously observed: the available point-in-time workload snapshots show these bursts, so thermal/noise attribution remains bounded. The script-owned sequential benchmark-harness compilations occurred immediately before their lanes with no separately recorded cooldown; although no compilation ran concurrently with a measured lane and the loaded dylibs stayed fixed, compilation heat and fixed lane order are additional noise/bias boundaries. All ratio rows exceed 10% relative spread because several ratios are very small and reported to 0.001 precision, while short rows also show scheduler sensitivity. This limits fine ranking but cannot erase the all-rows-below-1 result.\n\n"
                "No setup failure invalidated a measured round. The efficient sequence intentionally skipped a new universal all-17/all-prefix/all-page capture, allocator toggles, and hardware-counter profiling because snapshots matched source-matched 587 Windows and both focused behavior gates passed. "
                "A strictly conforming rerun was not performed because the exact unmodified script cannot target an external output root; such a rerun first requires explicit external-output support in the harness. Mac RSS is not interchangeable with Windows working-set/private/pagefile counters, and no Apple phys_footprint claim is made."
            ),
        },
        {
            "id": "recommendations",
            "type": "markdown",
            "body": (
                "## Recommended Next Steps\n\n"
                "1. Treat the old Mac latency deficit as resolved for the 5879405c measured path; update canonical docs to make the afb7079b packet historical.\n"
                "2. Keep the signed Windows ceilings unchanged and keep this Mac comparison diagnostic.\n"
                "3. Do not implement a performance fix from this packet. If a product decision needs finer attribution, profile one short row and one long row with fixed binaries and low-perturbation instruction/cycle counters.\n"
                "4. Track the zhongdengchangdu candidate-shape mismatch as a cross-platform oracle gap, not a Mac performance issue.\n"
                "5. Before any strict future Mac packet, add explicit external-output-root support to the benchmark harness under separately authorized work; do not change the harness in this diagnostic.\n"
                "6. Investigate Apple memory with phys_footprint only in a separate platform-memory gate; do not reuse Windows counters."
            ),
        },
        {
            "id": "further_questions",
            "type": "markdown",
            "body": (
                "## Further Questions\n\n"
                "- Why does librime scale more strongly than Yune on the long rows but not on n/ni/hao across these two platforms?\n"
                "- How much of the fifth-round Track B elevation is scheduler/UI noise versus sustained thermal state?\n"
                "- Would an all-prefix/all-page macOS capture reveal any behavior outside the named M59 gates?\n"
                "- Can the remaining Track A memory gap be reduced without changing the accepted candidate graph or signed latency ratchet?"
            ),
        },
        {
            "id": "sources",
            "type": "markdown",
            "body": (
                "## Sources\n\n"
                "The bounded report snapshot is built from the five Mac runs preserved externally after capture, the committed Increment-4e Windows performance packet, the signed Increment-0 packet and unchanged thresholds, M57 Mac model evidence, and the two focused commit-bound test logs. The Mac output-location deviation is recorded in the run audit. "
                "Every native chart, card, and table exposes its exact SQLite query and source-file list."
            ),
        },
    ]

    manifest = {
        "version": 1,
        "surface": "report",
        "title": TITLE,
        "description": "Technical Increment-4e source-bound macOS cross-platform performance and behavior diagnostic.",
        "generatedAt": GENERATED_AT,
        "blocks": blocks,
        "cards": cards,
        "charts": charts,
        "tables": tables,
        "sources": manifest_sources,
    }
    snapshot = {
        "version": 1,
        "status": "ready",
        "generatedAt": GENERATED_AT,
        "datasets": datasets,
    }
    artifact = {
        "surface": "report",
        "manifest": manifest,
        "snapshot": snapshot,
        "sources": sources,
    }
    (output / "artifact.json").write_text(
        json.dumps(artifact, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {output / 'artifact.json'}")
    print(f"datasets={len(datasets)} rows={sum(len(rows) for rows in datasets.values())}")


if __name__ == "__main__":
    main()
