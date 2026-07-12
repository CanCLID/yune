#!/usr/bin/env python3
"""Build the canonical MCP technical-report artifact for the current-main diagnosis."""

from __future__ import annotations

import csv
import glob
import hashlib
import json
import math
import sqlite3
import statistics
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path("/Users/laufei/yune-m59-post-fix-root-cause-20260711")
REPORT = ROOT / "report"
REPO = Path("/Users/laufei/Documents/GitHub/yune-m59-post-fix-root-cause")
OLD = Path("/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711/post-review-fix")
CURRENT_COMMIT = "afb7079b71f7f9353845114ff3e310c0a38b9b87"
OLD_POST_FIX_COMMIT = "89875ee2f812d070b43d12e6700407dccbb78435"
SIGNED_BASELINE_COMMIT = "457751824b8944676dc44912b9ce31ff29d78403"
PINNED_LIBRIME_COMMIT = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
YUNE_SHA256 = "3dd5a414c68f7884884c5dc172b3f0b088d1f5ae19cb983eb0eeb2f95bc6c710"
LIBRIME_SHA256 = "743acf3e3a0b64f94680a2f822b00ae42d35ce1e2ab3c8994441bc305adaf8f6"
LONG_37 = "ceshiyixiachangjushuruxingnengzenyang"
LONG_59 = "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong"
TRACK_B = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung"
NEWLY_SIGNED_INPUTS = {
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
TITLE = "M59 post-fix macOS performance root-cause diagnostic"


def now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8-sig", newline="") as handle:
        return list(csv.DictReader(handle))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def input_label(value: str) -> str:
    if value == LONG_37:
        return "37-character"
    if value == LONG_59:
        return "59-character"
    return value


def median_run_summaries(pattern: str) -> dict[str, dict[str, float]]:
    values: dict[str, list[tuple[float, float, float]]] = {}
    for path in glob.glob(pattern):
        for row in read_csv(Path(path)):
            if row.get("track") != "track-a-comparison":
                continue
            values.setdefault(row["input"], []).append(
                (
                    float(row["yune_median_us"]),
                    float(row["librime_median_us"]),
                    float(row["yune_librime_median_ratio"]),
                )
            )
    return {
        value: {
            "yune_us": statistics.median(item[0] for item in observations),
            "librime_us": statistics.median(item[1] for item in observations),
            "ratio": statistics.median(item[2] for item in observations),
        }
        for value, observations in values.items()
    }


ratio_raw = read_csv(ROOT / "analysis" / "output" / "track-a-ratio-comparison.csv")
track_a: list[dict[str, object]] = []
for order, row in enumerate(ratio_raw, 1):
    ratio = float(row["median_ratio"])
    track_a.append(
        {
            "row_order": order,
            "input": row["input"],
            "input_label": input_label(row["input"]),
            "characters": int(row["characters"]),
            "run1_ratio": float(row["run1_ratio"]),
            "run2_ratio": float(row["run2_ratio"]),
            "run3_ratio": float(row["run3_ratio"]),
            "run4_ratio": float(row["run4_ratio"]),
            "run5_ratio": float(row["run5_ratio"]),
            "median_ratio": ratio,
            "pooled_worst_ratio": float(row["pooled_worst_run_ratio"]),
            "spread_pct": float(row["spread_pct"]),
            "windows_signed_median": float(row["windows_signed_increment0_median"]),
            "windows_signed_ceiling": float(row["windows_signed_ceiling_diagnostic_only"]),
            "mac_vs_windows_signed_pct": float(row["mac_vs_windows_signed_increment0_pct"]),
            "mac_vs_windows_signed_class": row["mac_vs_windows_signed_increment0_class"],
            "windows_increment4a_median": float(row["windows_increment4a_median"]),
            "mac_vs_windows_increment4a_pct": float(row["mac_vs_windows_increment4a_pct"]),
            "mac_vs_windows_increment4a_class": row["mac_vs_windows_increment4a_class"],
            "signed_scope": "newly signed in M59 Increment-0"
            if row["input"] in NEWLY_SIGNED_INPUTS
            else "pre-existing signed row",
            "result": "Yune faster" if ratio < 1.0 else "Yune slower",
        }
    )

candidate_raw = read_csv(ROOT / "analysis" / "output" / "candidate-behavior-matrix.csv")
candidates = [
    {
        "input_label": input_label(row["input"]),
        "input": row["input"],
        "yune_first_page": row["yune_first_page_text"],
        "librime_first_page": row["librime_first_page_text"],
        "geometry_exact": row["yune_vs_librime_geometry_exact_all_five_runs"],
        "text_order_exact": row["yune_vs_librime_text_exact_all_five_runs"],
        "comments_exact": row["yune_vs_librime_comments_exact_all_five_runs"],
        "preedit_exact": row["yune_vs_librime_preedit_exact_all_five_runs"],
        "full_snapshot_exact": row["yune_vs_librime_full_exact_all_five_runs"],
        "text_mismatch_indexes": row["text_mismatch_candidate_indexes"],
        "comment_mismatch_indexes": row["comment_mismatch_candidate_indexes"],
    }
    for row in candidate_raw
    if row["track"] == "track-a-comparison"
]

prefix_raw = read_csv(
    ROOT / "analysis" / "controls" / "output" / "prefix-latency-payload-curve.csv"
)
prefix_curve = [
    {
        "input_label": input_label(row["input"]),
        "input": row["input"],
        "key_index": int(row["key_index"]),
        "prefix": row["prefix"],
        "yune_us": float(row["yune_total_us"]),
        "librime_us": float(row["librime_total_us"]),
        "yune_librime_ratio": float(row["yune_librime_total_ratio"]),
        "faster_engine": row["faster_engine"],
        "candidate_count_exact": row["candidate_count_exact"],
        "text_order_exact": row["text_fnv1a64_exact"],
        "comments_exact": row["comment_fnv1a64_exact"],
        "preedit_exact": row["preedit_fnv1a64_exact"],
        "full_snapshot_exact": row["snapshot_fnv1a64_exact"],
        "behavior_stratum": "candidate text matched"
        if row["text_fnv1a64_exact"] == "yes"
        else "candidate text differed",
        "line_style": "solid" if row["input"] == LONG_37 else "dashed",
        "chart_series": f"{input_label(row['input'])} · "
        + ("text matched" if row["text_fnv1a64_exact"] == "yes" else "text differed"),
        "log2_ratio": math.log2(float(row["yune_librime_total_ratio"])),
    }
    for row in prefix_raw
    if row["input"] in {LONG_37, LONG_59}
]

strata_raw = read_csv(
    ROOT
    / "analysis"
    / "controls"
    / "behavior-strata"
    / "long-prefix-behavior-strata.csv"
)
behavior_strata = []
for row in strata_raw:
    behavior_strata.append(
        {
            "input_label": input_label(row["input"]),
            "characters": int(row["characters"]),
            "prefixes": int(row["prefixes"]),
            "text_exact_prefixes": int(row["text_exact_prefixes"]),
            "text_different_prefixes": int(row["text_different_prefixes"]),
            "comment_exact_prefixes": int(row["comment_exact_prefixes"]),
            "preedit_exact_prefixes": int(row["preedit_exact_prefixes"]),
            "full_snapshot_exact_prefixes": int(row["full_snapshot_exact_prefixes"]),
            "all_prefix_ratio": float(row["sum_prefix_yune_librime_ratio_all"]),
            "text_matched_prefix_ratio": float(
                row["sum_prefix_yune_librime_ratio_text_exact"]
            ),
            "text_different_prefix_ratio": float(
                row["sum_prefix_yune_librime_ratio_text_different"]
            ),
            "librime_time_share_text_different_pct": float(
                row["librime_time_share_text_different_pct"]
            ),
            "yune_faster_prefixes": int(row["yune_faster_prefixes"]),
            "yune_faster_text_exact_prefixes": int(
                row["yune_faster_text_exact_prefixes"]
            ),
            "final_key_ratio": float(row["final_key_yune_librime_ratio"]),
            "boundary": row["interpretation_boundary"],
        }
    )

time_l_raw = read_csv(
    ROOT / "analysis" / "controls" / "time-l-output" / "time-l-engine-summary.csv"
)
time_l = [
    {
        "input_label": input_label(row["input"]),
        "input": row["input"],
        "driver_latency_ratio": float(row["driver_median_us_per_key_yune_librime_ratio"]),
        "cpu_ratio": float(row["whole_process_cpu_us_per_measured_key_yune_librime_ratio"]),
        "instructions_ratio": float(
            row["whole_process_instructions_per_measured_key_yune_librime_ratio"]
        ),
        "cycles_ratio": float(row["whole_process_cycles_per_measured_key_yune_librime_ratio"]),
        "cpi_ratio": float(row["whole_process_cpi_yune_librime_ratio"]),
        "max_rss_ratio": float(row["maximum_resident_set_bytes_yune_librime_ratio"]),
        "peak_footprint_ratio": float(
            row["peak_memory_footprint_bytes_yune_librime_ratio"]
        ),
        "classification": row["classification"],
        "boundary": row["caveat"],
    }
    for row in time_l_raw
]

reconciliation_raw = read_csv(
    ROOT / "analysis" / "reconciliation-delta" / "long-input-reconciliation-delta.csv"
)
reconciliation = [
    {
        "input_label": input_label(row["input"]),
        "old_ratio": float(row["old_yune_librime_ratio"]),
        "current_ratio": float(row["current_yune_librime_ratio"]),
        "ratio_change_pct": float(row["ratio_change_pct"]),
        "old_process_key_us": float(row["old_process_key_us_per_key"]),
        "current_process_key_us": float(row["current_process_key_us_per_key"]),
        "process_key_change_pct": float(row["process_key_change_pct"]),
        "old_graph_us": float(row["old_graph_us_per_key"]),
        "current_graph_us": float(row["current_graph_us_per_key"]),
        "graph_change_pct": float(row["graph_change_pct"]),
        "old_model_us": float(row["old_model_us_per_key"]),
        "current_model_us": float(row["current_model_us_per_key"]),
        "model_change_pct": float(row["model_change_pct"]),
    }
    for row in reconciliation_raw
]

allocator_raw = read_csv(
    ROOT / "analysis" / "controls" / "output" / "allocator-ratio-effects.csv"
)
allocator_effect_raw = read_csv(
    ROOT / "analysis" / "controls" / "output" / "allocator-effects.csv"
)
allocator_effects = {
    (row["engine"], row["input"]): row for row in allocator_effect_raw
}
allocator = [
    {
        "input_label": input_label(row["input"]),
        "default_ratio": float(row["default_median_ratio"]),
        "nano_off_ratio": float(row["nano-off_median_ratio"]),
        "nano_on_ratio": float(row["nano-on_median_ratio"]),
        "nano_off_vs_default_ratio_change_pct": float(
            row["nano_off_vs_default_ratio_change_pct"]
        ),
        "yune_nano_off_effect_pct": float(
            allocator_effects[("yune", row["input"])]["nano-off_median_effect_pct"]
        ),
        "librime_nano_off_effect_pct": float(
            allocator_effects[("librime", row["input"])]["nano-off_median_effect_pct"]
        ),
        "yune_nano_off_classification": allocator_effects[("yune", row["input"])][
            "nano_off_classification"
        ],
        "librime_nano_off_classification": allocator_effects[
            ("librime", row["input"])
        ]["nano_off_classification"],
        "classification": row["classification"],
    }
    for row in allocator_raw
]

api_raw = read_csv(ROOT / "analysis" / "controls" / "output" / "api-engine-ratios.csv")
api_modes = [
    {
        "input_label": input_label(row["input"]),
        "reference_ratio": float(row["reference_yune_librime_ratio"]),
        "process_only_ratio": float(row["process_only_yune_librime_ratio"]),
        "fresh_context_yune_us": float(row["context_end_yune_us"]),
        "fresh_context_librime_us": float(row["context_end_librime_us"]),
        "stable_context_yune_us": float(row["stable_context_yune_us"]),
        "stable_context_librime_us": float(row["stable_context_librime_us"]),
        "interpretation": row["interpretation"],
    }
    for row in api_raw
]

track_b_raw = read_csv(ROOT / "analysis" / "output" / "track-b-five-observations.csv")
track_b = [
    {
        "run": int(row["run"]),
        "input": TRACK_B,
        "median_us": float(row["median_us"]),
        "p95_us": float(row["p95_us"]),
        "p99_us": float(row["p99_us"]),
        "max_us": float(row["max_us"]),
        "median_working_set_mib": round(int(row["median_working_set_bytes"]) / 1048576, 1),
        "max_peak_working_set_mib": round(
            int(row["max_peak_working_set_bytes"]) / 1048576, 1
        ),
    }
    for row in track_b_raw
]

logical_raw = read_csv(
    ROOT / "analysis" / "logical-volume" / "output" / "full-input-logical-work.csv"
)
logical_volume = [
    {
        "input_label": f"{row['characters']}-character",
        "owned_candidates": int(row["owned_candidates"]),
        "byte_backed_candidates": int(row["byte_backed_candidates"]),
        "byte_vs_owned_table_entries_ratio": float(
            row["byte_vs_owned_table_entries_ratio"]
        ),
        "byte_vs_owned_graph_entries_ratio": float(
            row["byte_vs_owned_graph_entries_ratio"]
        ),
        "byte_vs_owned_dp_states_ratio": float(row["byte_vs_owned_dp_states_ratio"]),
        "scope": row["scope"],
    }
    for row in logical_raw
]

mac_absolute = median_run_summaries(
    str(ROOT / "accepted-baseline" / "run-*" / "summary-comparison.csv")
)
windows_4a_absolute = median_run_summaries(
    str(
        REPO
        / "docs"
        / "reports"
        / "evidence"
        / "m59-canonical-jyutping-reachability-parity"
        / "increment-4a-sentence-ordering"
        / "performance-ratchet"
        / "run*"
        / "summary-comparison.csv"
    )
)
cross_platform_inputs = ["n", "ni", "hao", LONG_37, LONG_59]
cross_platform = []
for value in cross_platform_inputs:
    mac = mac_absolute[value]
    win = windows_4a_absolute[value]
    cross_platform.append(
        {
            "input_label": input_label(value),
            "mac_yune_us": round(mac["yune_us"], 3),
            "windows_4a_yune_us": round(win["yune_us"], 3),
            "mac_vs_windows_yune_pct": round((mac["yune_us"] / win["yune_us"] - 1) * 100, 1),
            "mac_librime_us": round(mac["librime_us"], 3),
            "windows_4a_librime_us": round(win["librime_us"], 3),
            "mac_vs_windows_librime_pct": round(
                (mac["librime_us"] / win["librime_us"] - 1) * 100, 1
            ),
            "mac_ratio": round(mac["ratio"], 3),
            "windows_4a_ratio": round(win["ratio"], 3),
            "comparison_boundary": "different source commits and platforms; diagnostic only",
        }
    )


def owner_focus(path: Path) -> dict[str, dict[str, object]]:
    output: dict[str, dict[str, object]] = {}
    for row in read_csv(path):
        if row["track"] != "track-a-comparison" or row["module"] != "poet":
            continue
        if row["owner_id"] not in {
            "poet.entries_by_code",
            "poet.lookup_index",
            "poet.vocabulary",
            "poet.abbreviation_vocabulary",
        }:
            continue
        output[row["owner_id"]] = {
            "bytes": int(row["retained_estimate_bytes"]),
            "items": int(row["item_count"]),
            "mode": row["mapping_mode"],
        }
    return output


old_owner = owner_focus(OLD / "run-1" / "memory-owner-profile.csv")
current_owner = owner_focus(
    ROOT / "accepted-baseline" / "run-1" / "memory-owner-profile.csv"
)
model_memory = []
for owner_id in [
    "poet.entries_by_code",
    "poet.lookup_index",
    "poet.vocabulary",
    "poet.abbreviation_vocabulary",
]:
    old = old_owner[owner_id]
    current = current_owner[owner_id]
    model_memory.append(
        {
            "owner": owner_id,
            "old_items": old["items"],
            "current_items": current["items"],
            "old_retained_mib": round(int(old["bytes"]) / 1048576, 3),
            "current_retained_mib": round(int(current["bytes"]) / 1048576, 3),
            "current_mapping_mode": current["mode"],
            "boundary": "owner estimate; do not sum with whole-process RSS as disjoint causation",
        }
    )

track_b_counter_raw = read_csv(
    ROOT / "analysis" / "m57-trackb" / "track-b-counter-diff.csv"
)
track_b_counter_names = {
    "track_b_spelling_expansions_considered",
    "track_b_exact_lookup_calls",
    "track_b_prefix_lookup_calls",
    "track_b_candidates_materialized",
    "bounded_iterator_selected_total",
    "bounded_iterator_full_count_total",
    "owned_candidates_materialized",
    "candidates_sorted",
}
track_b_counters = []
for row in track_b_counter_raw:
    if row["counter"] not in track_b_counter_names:
        continue
    track_b_counters.append(
        {
            "counter": row["counter"],
            "m57_pass1": int(row["m57_full_pass_1"]),
            "current": int(row["current_value"]),
            "change_pct": float(row["change_vs_m57_full_pass_1_pct"]),
            "all_current_runs_identical": row["current_runs_consistent"],
        }
    )

track_b_candidate_audit = read_csv(
    ROOT / "analysis" / "m57-trackb" / "track-b-candidate-audit.csv"
)
track_b_product_status = read_csv(
    ROOT / "analysis" / "m57-trackb" / "track-b-product-status-diff.csv"
)
track_b_owner_status = read_csv(
    ROOT / "analysis" / "m57-trackb" / "track-b-memory-owner-diff.csv"
)


def product_status(dictionary_id: str, field: str) -> dict[str, str]:
    matches = [
        row
        for row in track_b_product_status
        if row["dictionary_id"] == dictionary_id and row["field"] == field
    ]
    if len(matches) != 1:
        raise RuntimeError(f"expected one Track B product row for {dictionary_id}/{field}")
    return matches[0]


candidate_anchor = track_b_candidate_audit[0]
current_candidate = track_b_candidate_audit[-1]
poet_owner_rows = [
    row for row in track_b_owner_status if row["owner_id"].startswith("poet.")
]
added_owner_rows = [row for row in track_b_owner_status if row["status"] == "added_current"]
changed_owner_rows = [
    row for row in track_b_owner_status if row["status"] == "changed_shape"
]
if not (
    len(track_b_candidate_audit) == 7
    and all(row["raw_sha256"] == candidate_anchor["raw_sha256"] for row in track_b_candidate_audit)
    and len(poet_owner_rows) == 6
    and all(
        row["m57_retained_bytes"] == "0"
        and row["current_retained_bytes"] == "0"
        and row["m57_item_count"] == "0"
        and row["current_item_count"] == "0"
        for row in poet_owner_rows
    )
    and len(added_owner_rows) == 12
    and len(changed_owner_rows) == 6
):
    raise RuntimeError("Track B M57 status inputs failed their expected shape")

track_b_m57_status = [
    {
        "category": "Identity",
        "item": "Exact Track B product input",
        "m57_value": TRACK_B,
        "current_value": TRACK_B,
        "status": "unchanged",
        "evidence": "M57 Track B audit and all five current runs",
    },
    {
        "category": "Candidate behavior",
        "item": "First page",
        "m57_value": candidate_anchor["texts"],
        "current_value": current_candidate["texts"],
        "status": "byte-identical across 2 + 5 observations",
        "evidence": "track-b-candidate-audit.csv",
    },
    {
        "category": "Candidate behavior",
        "item": "Raw candidate CSV SHA-256",
        "m57_value": candidate_anchor["raw_sha256"],
        "current_value": current_candidate["raw_sha256"],
        "status": "unchanged; hash covers comments and page state",
        "evidence": "track-b-candidate-audit.csv",
    },
    {
        "category": "Candidate behavior",
        "item": "Page geometry",
        "m57_value": "5 candidates; page size 5; page 0; last=false; highlighted=0",
        "current_value": "5 candidates; page size 5; page 0; last=false; highlighted=0",
        "status": "unchanged",
        "evidence": "track-b-candidate-audit.csv",
    },
    {
        "category": "Model identity",
        "item": "jyut6ping3 source/table checksum",
        "m57_value": product_status("jyut6ping3", "source_checksum")["m57_full_pass_1"],
        "current_value": product_status("jyut6ping3", "source_checksum")["current_run_1"],
        "status": "unchanged; fresh",
        "evidence": "track-b-product-status-diff.csv",
    },
    {
        "category": "Model identity",
        "item": "jyut6ping3_scolar source/table checksum",
        "m57_value": product_status("jyut6ping3_scolar", "source_checksum")["m57_full_pass_1"],
        "current_value": product_status("jyut6ping3_scolar", "source_checksum")["current_run_1"],
        "status": "unchanged; fresh",
        "evidence": "track-b-product-status-diff.csv",
    },
    {
        "category": "Storage shape",
        "item": "Both dictionary owners",
        "m57_value": "compiled-ready; byte-backed; table mmap; prism mmap; no source fallback",
        "current_value": "compiled-ready; byte-backed; table mmap; prism mmap; no source fallback",
        "status": "unchanged",
        "evidence": "track-b-product-status-diff.csv",
    },
    {
        "category": "Storage shape",
        "item": "Main dictionary entries / codes",
        "m57_value": "127143 stored/expanded; 114653 codes",
        "current_value": "127143 stored/expanded; 114653 codes",
        "status": "unchanged",
        "evidence": "track-b-product-status-diff.csv",
    },
    {
        "category": "Storage shape",
        "item": "Compiled table byte-source lengths",
        "m57_value": "15248382 / 12478842 bytes",
        "current_value": "15248410 / 12478870 bytes",
        "status": "+28 bytes each; checksums and entry counts unchanged",
        "evidence": "track-b-product-status-diff.csv",
    },
    {
        "category": "Model owner",
        "item": "POET entries/index/abbreviation owners",
        "m57_value": "6 normalized occurrences; 0 retained bytes; 0 items; mapping none",
        "current_value": "6 normalized occurrences; 0 retained bytes; 0 items; mapping none",
        "status": "unchanged; sentence model not retained for either translator",
        "evidence": "track-b-memory-owner-diff.csv",
    },
    {
        "category": "Model owner",
        "item": "Normalized non-process owner deltas",
        "m57_value": "no guarded leading/spelling rows; prior mapping sizes",
        "current_value": "12 guarded rows added at 0/48 B; 6 shape changes",
        "status": "structural/accounting and mapping changes; not a large retained-owner regression",
        "evidence": "track-b-memory-owner-diff.csv; paths/session ids excluded",
    },
    {
        "category": "Memory observation",
        "item": "Peak resident proxy",
        "m57_value": "741736448..752746496 bytes",
        "current_value": "444940288..468123648 bytes",
        "status": "same-platform observational; not additive or causal",
        "evidence": "track-b-memory-owner-diff.csv",
    },
    {
        "category": "Provenance",
        "item": "Source state",
        "m57_value": "c6749cc6 plus dirty changes later committed as a87c6b88",
        "current_value": f"clean detached {CURRENT_COMMIT}",
        "status": "descriptive comparison across source states",
        "evidence": "analysis/m57-trackb/README.md",
    },
]

findings = [
    {
        "finding": "Earlier post-fix report is stale for current main",
        "classification": "verified",
        "evidence": "37/59 ratios changed 2.428→0.399 and 1.809→0.205; graph time fell 98.7%/99.2% after reconciliation",
        "implication": "Use this current-main packet for diagnosis; retain the old packet as historical evidence only",
    },
    {
        "finding": "Long aggregate advantage is not behavior-normalized",
        "classification": "verified",
        "evidence": "Text-different prefixes consume 82.0%/90.1% of librime long-row time; text-matched subset ratios are 1.420/1.204",
        "implication": "Do not call 0.399/0.205 a pure implementation-speed win",
    },
    {
        "finding": "Short rows execute more Yune work",
        "classification": "verified",
        "evidence": "n/zh use 8.682×/4.092× instructions and 10.730×/5.061× cycles; five-round direction is stable",
        "implication": "The short-row deficit is a real work-volume problem, not just timer noise",
    },
    {
        "finding": "Short-row function owners",
        "classification": "qualitatively verified",
        "evidence": "Symbolized samples resolve compact-table/MARISA expansion plus abbreviation/sentence-model generation",
        "implication": "Prioritize these owners, but do not treat single-sample branch percentages as causal effect sizes",
    },
    {
        "finding": "Eager versus lazy page evaluation",
        "classification": "source-verified; causal magnitude likely",
        "evidence": "Yune eagerly selects/stores owned batches before exporting five; librime Menu/Translation/Uniquifier enumeration is demand-driven",
        "implication": "A behavior-locked, filter-aware page-fill prototype is the most direct librime lesson",
    },
    {
        "finding": "Translation dominates Yune latency",
        "classification": "verified",
        "evidence": "Translator accounts for about 90–99% across inspected Track A and 96.7% on Track B",
        "implication": "ABI copying, ranker, filter shell, and AI merge are not first-order targets",
    },
    {
        "finding": "API/context export",
        "classification": "verified negative result",
        "evidence": "Process-only ratios retain the deficit; fresh context export is 0.54–0.67 µs for Yune and 4.38–9.71 µs for librime",
        "implication": "Context export actually favors Yune and cannot explain the missing advantage",
    },
    {
        "finding": "M37 instrumentation tax",
        "classification": "verified small",
        "evidence": "Paired metrics-on/off tax is 1.0–2.9% on six inputs",
        "implication": "Instrumentation asymmetry is real but not the multi-fold cause",
    },
    {
        "finding": "macOS Nano allocator contribution",
        "classification": "verified partial",
        "evidence": "Nano-off lowers the stable long/medium Yune/librime ratio by about 6–14%; librime slows more",
        "implication": "macOS allocator behavior contributes, but does not explain behavior or instruction-volume gaps",
    },
    {
        "finding": "CPI disadvantage",
        "classification": "verified; owner unresolved",
        "evidence": "Yune/librime CPI is 1.236–1.405 across n, zh, 37, and 59",
        "implication": "There is a secondary per-instruction disadvantage; cache/compiler/branch/memory attribution is not proven",
    },
    {
        "finding": "Whole-process memory gap",
        "classification": "verified absolute; causality unresolved",
        "evidence": "Yune max RSS is 11.5–18.3× and peak footprint 23.5–26.4× librime in amortized controls",
        "implication": "Memory deserves its own workstream, but the packet does not prove memory causes CPI or latency",
    },
    {
        "finding": "Byte-backed POET timing",
        "classification": "rejected",
        "evidence": "The deployed opt-in path returns zero candidates versus five on all 99 prefixes",
        "implication": "Repair behavior before timing; fixture counters do not show an automatic CPU win",
    },
    {
        "finding": "Pure platform-only explanation",
        "classification": "rejected",
        "evidence": "Allocator effects coexist with deterministic payload differences and large instruction-volume differences",
        "implication": "The residual is a mixed behavior, engine-path, allocator, build, and platform problem",
    },
    {
        "finding": "Thermal/noise-only explanation",
        "classification": "rejected as primary; noise remains",
        "evidence": "No thermal/performance warning; fixed hashes and stable instruction ratios; some short latency and Track B tails remain noisy",
        "implication": "Noise affects precision but cannot account for the direction and work-volume split",
    },
    {
        "finding": "Exact current cross-platform attribution",
        "classification": "unresolved",
        "evidence": "Windows 4a is near-code but not afb7079; compiler/linker/CPU/platform and source commits differ",
        "implication": "Run the same current commit and payload on both platforms before assigning the residual to macOS",
    },
]

priorities = [
    {
        "priority": 1,
        "work": "Incremental-prefix oracle lock",
        "why": "No 37/59 full prefix snapshot is exact; aggregate advantage is concentrated in candidate-text-different prefixes",
        "required_guard": "Candidate text/order, comments, preedit, pagination, selection identity at every measured prefix",
        "risk": "behavior-critical",
    },
    {
        "priority": 2,
        "work": "Translator residual and producer attribution",
        "why": "Translation is 90–99% of elapsed, but direct leading-family and producer-specific work is incompletely timed",
        "required_guard": "Low-perturbation counters/sampling; separate punctuation, reverse, script, exact, prefix, family, formatting, filter, storage",
        "risk": "diagnostic",
    },
    {
        "priority": 3,
        "work": "Behavior-locked lazy page-fill prototype",
        "why": "Yune materializes surplus owned candidates while librime drains lazy streams to a page",
        "required_guard": "All named oracle/profile rows, pagination, dedup/filter refill, comments and preedit",
        "risk": "high behavior risk",
    },
    {
        "priority": 4,
        "work": "Short abbreviation/MARISA path",
        "why": "n/zh samples and counters identify abbreviation graph plus expanded compact-table traversal",
        "required_guard": "Resolve current n/zh page differences first; then remove duplicate scans/cache safe bounded views",
        "risk": "oracle-sensitive",
    },
    {
        "priority": 5,
        "work": "Memory and byte-backed POET behavior",
        "why": "Whole-process memory is much larger; deployed byte-backed POET currently drops all candidates",
        "required_guard": "Recover identical prefix behavior; then measure memory and CPU separately with incremental/lazy indexing",
        "risk": "behavior and memory",
    },
    {
        "priority": 6,
        "work": "Exact-current Windows/macOS matched lane",
        "why": "Near-code evidence suggests platform asymmetry, but source/build/host are confounded",
        "required_guard": "Same commit, source payload, iterations, allocator declarations, hashes, and retained red rounds",
        "risk": "cross-platform diagnostic",
    },
    {
        "priority": 7,
        "work": "Track B overfetch and owner follow-up",
        "why": "Behavior is exact but Track B materialization +95.2% and bounded selection +110.9% versus M57",
        "required_guard": "Exact M57 page/comments/page-state subset and product checksums",
        "risk": "product profile",
    },
]

setup_failures = [
    {
        "stage": "Initial warmup",
        "status": "excluded and preserved",
        "detail": "Spotlight/indexing activity was observed before measurement; unmeasured warmup retained under setup-warmup-unmeasured",
    },
    {
        "stage": "Timing control attempt 0",
        "status": "setup failure, retried",
        "detail": "Runner permission failure before measurement; preserved under controls/setup-failures/attempt-0-runner-permission",
    },
    {
        "stage": "Timing control attempt 1 allocator continuation",
        "status": "setup failure, explicit retry",
        "detail": "macOS env rejected -u syntax at case 233; completed prior families retained and allocator/later families rerun under explicit retry root",
    },
    {
        "stage": "POET storage timing",
        "status": "measured but rejected",
        "detail": "Byte-backed path emitted zero candidates on 99/99 prefixes; all 40 timing rows preserved as missing-behavior evidence",
    },
    {
        "stage": "Function profiles",
        "status": "qualitative only",
        "detail": "Yune symbol sibling includes 19–26% setup samples; librime n has only 40 engine samples; active Spotlight/UI load and no replication",
    },
    {
        "stage": "Unavailable profilers",
        "status": "skipped with limitation",
        "detail": "Full Xcode/Instruments absent; DTrace/powermetrics unavailable unattended; /usr/bin/sample and /usr/bin/time -l used",
    },
]

headline = [
    {
        "retained_rounds": 5,
        "yune_faster_rows": sum(row["median_ratio"] < 1.0 for row in track_a),
        "yune_slower_rows": sum(row["median_ratio"] >= 1.0 for row in track_a),
        "ratio_37": next(row["median_ratio"] for row in track_a if row["input"] == LONG_37),
        "text_matched_ratio_37": next(
            row["text_matched_prefix_ratio"] for row in behavior_strata if row["characters"] == 37
        ),
        "ratio_59": next(row["median_ratio"] for row in track_a if row["input"] == LONG_59),
        "text_matched_ratio_59": next(
            row["text_matched_prefix_ratio"] for row in behavior_strata if row["characters"] == 59
        ),
        "n_instruction_ratio": next(
            row["instructions_ratio"] for row in time_l if row["input"] == "n"
        ),
        "zh_instruction_ratio": next(
            row["instructions_ratio"] for row in time_l if row["input"] == "zh"
        ),
        "track_b_median_us": statistics.median(row["median_us"] for row in track_b),
        "track_b_spread_pct": round(
            (max(row["median_us"] for row in track_b) - min(row["median_us"] for row in track_b))
            / min(row["median_us"] for row in track_b)
            * 100,
            1,
        ),
    }
]

cpu_owner_raw = read_csv(ROOT / "analysis" / "cpu-samples" / "owner-tags.csv")
cpu_owners = [
    {
        "engine": row["engine"],
        "input_label": {"37-char": "37-character", "59-char": "59-character"}.get(
            row["input_label"], row["input_label"]
        ),
        "owner": row["owner_tag"],
        "samples": int(row["observed_samples"]),
        "process_key_samples": int(row["denominator_samples"]),
        "share_pct": float(row["share_pct"]),
        "boundary": row["overlap_note"],
    }
    for row in cpu_owner_raw
    if row["allocator"] == "default"
    and row["scope"] == "RimeProcessKey"
    and row["input_label"] in {"n", "zh", "37-char", "59-char"}
    and int(row["observed_samples"]) > 0
    and (
        (
            row["engine"] == "yune"
            and row["owner_tag"]
            in {
                "yune_marisa_lookup",
                "yune_abbreviation_sentence",
                "yune_sentence_word_graph",
            }
        )
        or (
            row["engine"] == "librime"
            and row["owner_tag"]
            in {
                "librime_dictionary_table_query",
                "librime_table_query_access",
                "allocator_leaf_under_primary_owner",
            }
        )
    )
]


def execute_snapshot_query(
    table_name: str, rows: list[dict[str, object]], sql: str
) -> list[dict[str, object]]:
    """Materialize the reviewed rows and execute the exact SQL exposed as provenance."""
    if not rows:
        raise RuntimeError(f"cannot materialize empty report dataset {table_name}")
    columns = list(rows[0])

    def declared_type(column: str) -> str:
        values = [row[column] for row in rows if row[column] is not None]
        if values and all(isinstance(value, (bool, int)) for value in values):
            return "INTEGER"
        if values and all(isinstance(value, (bool, int, float)) for value in values):
            return "REAL"
        return "TEXT"

    connection = sqlite3.connect(":memory:")
    column_sql = ", ".join(
        f'"{column}" {declared_type(column)}' for column in columns
    )
    connection.execute(f'CREATE TABLE "{table_name}" ({column_sql})')
    placeholders = ", ".join("?" for _ in columns)
    connection.executemany(
        f'INSERT INTO "{table_name}" VALUES ({placeholders})',
        [[row[column] for column in columns] for row in rows],
    )
    cursor = connection.execute(sql)
    output_columns = [description[0] for description in cursor.description]
    output = [dict(zip(output_columns, row)) for row in cursor.fetchall()]
    connection.close()
    return output


# Every exposed source query below is executed here against the reviewed rows.
headline = execute_snapshot_query("headline", headline, "SELECT * FROM headline")
track_a = execute_snapshot_query(
    "track_a_report", track_a, "SELECT * FROM track_a_report ORDER BY row_order"
)
prefix_curve = execute_snapshot_query(
    "prefix_curve_report",
    prefix_curve,
    "SELECT * FROM prefix_curve_report ORDER BY input_label, key_index",
)
behavior_strata = execute_snapshot_query(
    "behavior_strata_report",
    behavior_strata,
    "SELECT * FROM behavior_strata_report ORDER BY characters",
)
time_l = execute_snapshot_query(
    "time_l_report", time_l, "SELECT * FROM time_l_report ORDER BY input_label"
)
reconciliation = execute_snapshot_query(
    "reconciliation_report",
    reconciliation,
    "SELECT * FROM reconciliation_report ORDER BY input_label",
)
allocator = execute_snapshot_query(
    "allocator_report",
    allocator,
    "SELECT * FROM allocator_report ORDER BY classification, input_label",
)
api_modes = execute_snapshot_query(
    "api_report", api_modes, "SELECT * FROM api_report ORDER BY input_label"
)
cross_platform = execute_snapshot_query(
    "cross_platform_report",
    cross_platform,
    "SELECT * FROM cross_platform_report ORDER BY input_label",
)
candidates = execute_snapshot_query(
    "candidate_report",
    candidates,
    "SELECT * FROM candidate_report ORDER BY input_label",
)
cpu_owners = execute_snapshot_query(
    "cpu_owner_report",
    cpu_owners,
    "SELECT * FROM cpu_owner_report ORDER BY engine, input_label, owner",
)
model_memory = execute_snapshot_query(
    "model_memory_report",
    model_memory,
    "SELECT * FROM model_memory_report ORDER BY current_retained_mib DESC",
)
track_b = execute_snapshot_query(
    "track_b_report", track_b, "SELECT * FROM track_b_report ORDER BY run"
)
track_b_counters = execute_snapshot_query(
    "track_b_counter_report",
    track_b_counters,
    "SELECT * FROM track_b_counter_report ORDER BY counter",
)
track_b_m57_status = execute_snapshot_query(
    "track_b_m57_status_report",
    track_b_m57_status,
    "SELECT * FROM track_b_m57_status_report ORDER BY category, item",
)
logical_volume = execute_snapshot_query(
    "logical_volume_report",
    logical_volume,
    "SELECT * FROM logical_volume_report ORDER BY input_label",
)
findings = execute_snapshot_query(
    "findings_report", findings, "SELECT * FROM findings_report ORDER BY finding"
)
priorities = execute_snapshot_query(
    "priorities_report",
    priorities,
    "SELECT * FROM priorities_report ORDER BY priority",
)
setup_failures = execute_snapshot_query(
    "failures_report", setup_failures, "SELECT * FROM failures_report ORDER BY stage"
)


def write_snapshot_database(tables: dict[str, list[dict[str, object]]]) -> Path:
    """Persist the exact queryable report snapshot used by every SQL source."""
    REPORT.mkdir(parents=True, exist_ok=True)
    database = REPORT / "snapshot.sqlite"
    if database.exists():
        database.unlink()
    connection = sqlite3.connect(database)
    for table_name, rows in tables.items():
        if not rows:
            raise RuntimeError(f"cannot persist empty report dataset {table_name}")
        columns = list(rows[0])

        def declared_type(column: str) -> str:
            values = [row[column] for row in rows if row[column] is not None]
            if values and all(isinstance(value, (bool, int)) for value in values):
                return "INTEGER"
            if values and all(isinstance(value, (bool, int, float)) for value in values):
                return "REAL"
            return "TEXT"

        column_sql = ", ".join(
            f'"{column}" {declared_type(column)}' for column in columns
        )
        connection.execute(f'CREATE TABLE "{table_name}" ({column_sql})')
        placeholders = ", ".join("?" for _ in columns)
        connection.executemany(
            f'INSERT INTO "{table_name}" VALUES ({placeholders})',
            [[row[column] for column in columns] for row in rows],
        )
    connection.commit()
    connection.close()
    return database


snapshot_database = write_snapshot_database(
    {
        "headline": headline,
        "track_a_report": track_a,
        "prefix_curve_report": prefix_curve,
        "behavior_strata_report": behavior_strata,
        "time_l_report": time_l,
        "reconciliation_report": reconciliation,
        "allocator_report": allocator,
        "api_report": api_modes,
        "cross_platform_report": cross_platform,
        "candidate_report": candidates,
        "cpu_owner_report": cpu_owners,
        "model_memory_report": model_memory,
        "track_b_report": track_b,
        "track_b_counter_report": track_b_counters,
        "track_b_m57_status_report": track_b_m57_status,
        "logical_volume_report": logical_volume,
        "findings_report": findings,
        "priorities_report": priorities,
        "failures_report": setup_failures,
    }
)


source_specs = [
    (
        "headline_sql",
        "Validated diagnostic headline query",
        "external-evidence/m59-post-fix-root-cause-20260711/report/build_report_artifact.py",
        "headline",
        "SELECT * FROM headline",
        "Returns the decision-relevant current-main headline values after all fail-closed analyses pass.",
    ),
    (
        "track_a_sql",
        "Current-main five-round Track A query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/output/track-a-ratio-comparison.csv",
        "track_a_report",
        "SELECT * FROM track_a_report ORDER BY row_order",
        "Returns all 17 retained rows and both diagnostic Windows comparison contexts without filtering red results.",
    ),
    (
        "prefix_sql",
        "Five-round long-prefix trace query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/controls/output/prefix-latency-payload-curve.csv",
        "prefix_curve_report",
        "SELECT * FROM prefix_curve_report ORDER BY input_label, key_index",
        "Returns all 96 measured long-input prefixes with five-round medians and captured payload-equivalence flags.",
    ),
    (
        "behavior_strata_sql",
        "Long-prefix behavior-stratified query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/controls/behavior-strata/long-prefix-behavior-strata.csv",
        "behavior_strata_report",
        "SELECT * FROM behavior_strata_report ORDER BY characters",
        "Stratifies reconstructed long-prefix time by candidate-text hash match; full semantic equivalence is explicitly not claimed.",
    ),
    (
        "time_l_sql",
        "Whole-process hardware-counter query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/controls/time-l-output/time-l-engine-summary.csv",
        "time_l_report",
        "SELECT * FROM time_l_report ORDER BY input_label",
        "Returns five-round instruction, cycle, CPU, CPI, RSS, and footprint ratios from 40 high-iteration controls.",
    ),
    (
        "reconciliation_sql",
        "Before/after reconciliation query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/reconciliation-delta/long-input-reconciliation-delta.csv",
        "reconciliation_report",
        "SELECT * FROM reconciliation_report ORDER BY input_label",
        "Compares the earlier 89875ee2 post-fix packet with current afb7079b across five retained rounds on each side.",
    ),
    (
        "allocator_sql",
        "Allocator control query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/controls/output/allocator-ratio-effects.csv",
        "allocator_report",
        "SELECT * FROM allocator_report ORDER BY classification, input_label",
        "Returns five-round default, Nano-off, and forced-Nano ratio medians while preserving noise-sensitive short rows.",
    ),
    (
        "api_sql",
        "API-path control query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/controls/output/api-engine-ratios.csv",
        "api_report",
        "SELECT * FROM api_report ORDER BY input_label",
        "Separates process-key from fresh/stable context export for n, zh, 37, and 59.",
    ),
    (
        "cross_platform_sql",
        "Near-code Windows/macOS diagnostic query",
        "external-evidence/m59-post-fix-root-cause-20260711/report/build_report_artifact.py",
        "cross_platform_report",
        "SELECT * FROM cross_platform_report ORDER BY input_label",
        "Compares medians of five current Mac runs with five Windows Increment-4a runs; source and platform remain confounded.",
    ),
    (
        "candidate_sql",
        "Candidate behavior matrix query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/output/candidate-behavior-matrix.csv",
        "candidate_report",
        "SELECT * FROM candidate_report ORDER BY input_label",
        "Returns all 17 final-page candidate, comment, preedit, and full-snapshot equivalence flags.",
    ),
    (
        "cpu_owner_sql",
        "Process-key CPU owner query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/cpu-samples/owner-tags.csv",
        "cpu_owner_report",
        "SELECT * FROM cpu_owner_report ORDER BY engine, input_label, owner",
        "Returns selected inclusive function-family tags after excluding setup; overlapping shares are not additive or timing evidence.",
    ),
    (
        "memory_sql",
        "Track A POET owner comparison query",
        "external-evidence/m59-post-fix-root-cause-20260711/report/build_report_artifact.py",
        "model_memory_report",
        "SELECT * FROM model_memory_report ORDER BY current_retained_mib DESC",
        "Compares retained owner estimates in the earlier and current five-run packets.",
    ),
    (
        "track_b_sql",
        "Current Track B observation query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/output/track-b-five-observations.csv",
        "track_b_report",
        "SELECT * FROM track_b_report ORDER BY run",
        "Returns every retained current Track B run; Track B is a Yune product guard, not a peer benchmark.",
    ),
    (
        "track_b_counter_sql",
        "M57 Track B work-shape query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/m57-trackb/track-b-counter-diff.csv",
        "track_b_counter_report",
        "SELECT * FROM track_b_counter_report ORDER BY counter",
        "Returns focused deterministic non-time counter movements versus M57 full-pass-1.",
    ),
    (
        "track_b_m57_status_sql",
        "M57 Track B candidate, product, and owner status query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/m57-trackb",
        "track_b_m57_status_report",
        "SELECT * FROM track_b_m57_status_report ORDER BY category, item",
        "Returns exact input/page/hash identity, product checksums/storage, normalized owner deltas, and the M57 provenance boundary.",
    ),
    (
        "logical_sql",
        "Fixture logical-volume query",
        "external-evidence/m59-post-fix-root-cause-20260711/analysis/logical-volume/output/full-input-logical-work.csv",
        "logical_volume_report",
        "SELECT * FROM logical_volume_report ORDER BY input_label",
        "Returns fixture-only owned versus byte-backed logical work; debug nanoseconds are excluded.",
    ),
    (
        "findings_sql",
        "Evidence classification query",
        "external-evidence/m59-post-fix-root-cause-20260711/report/build_report_artifact.py",
        "findings_report",
        "SELECT * FROM findings_report ORDER BY finding",
        "Reconciles validated baseline, controls, source audit, CPU samples, and explicit rejection boundaries.",
    ),
    (
        "priorities_sql",
        "Future priority query",
        "external-evidence/m59-post-fix-root-cause-20260711/report/build_report_artifact.py",
        "priorities_report",
        "SELECT * FROM priorities_report ORDER BY priority",
        "Orders evidence-backed future work without creating a milestone or authorizing implementation.",
    ),
    (
        "failures_sql",
        "Setup failure and rejection inventory query",
        "external-evidence/m59-post-fix-root-cause-20260711/report/build_report_artifact.py",
        "failures_report",
        "SELECT * FROM failures_report ORDER BY stage",
        "Preserves every setup failure, explicit retry, rejected behavior lane, and profiler limitation.",
    ),
]

source_filters = {
    "headline_sql": ["One derived decision-headline row; formulas are in build_report_artifact.py."],
    "track_a_sql": ["track = track-a-comparison", "All 17 inputs and all five measured rounds retained."],
    "prefix_sql": [
        f"input IN ({LONG_37}, {LONG_59})",
        "The n/zh focus traces are excluded; every one of the 96 long-input prefixes is retained.",
    ],
    "behavior_strata_sql": ["The complete 37- and 59-character behavior-strata summary; no row filter."],
    "time_l_sql": [f"input IN (n, zh, {LONG_37}, {LONG_59})", "All five paired processes retained."],
    "reconciliation_sql": ["37- and 59-character rows only; both five-run source packets retained."],
    "allocator_sql": ["Nine designated inputs; default, Nano-off, and forced-Nano observations retained across all five rounds."],
    "api_sql": [f"input IN (n, zh, {LONG_37}, {LONG_59})", "Reference, process-only, fresh-context, and stable-context modes retained."],
    "cross_platform_sql": [f"input IN (n, ni, hao, {LONG_37}, {LONG_59})", "Fixed diagnostic subset; medians use all five runs on each platform."],
    "candidate_sql": ["track = track-a-comparison", "Track B is excluded here and reported in its dedicated M57 status table."],
    "cpu_owner_sql": [
        "allocator = default",
        "scope = RimeProcessKey",
        "input IN (n, zh, 37-char, 59-char)",
        "observed_samples > 0",
        "Yune owners IN (marisa lookup, abbreviation sentence, sentence word graph); librime owners IN (dictionary table query, table query access, allocator leaf).",
    ],
    "memory_sql": ["Track A run-1 owner profile from each source packet.", "owner_id IN (POET entries_by_code, lookup_index, vocabulary, abbreviation_vocabulary)."],
    "track_b_sql": ["All five accepted current Track B observations retained; no row filter."],
    "track_b_counter_sql": [
        "Focused eight-counter subset: Track-B spelling/exact/prefix/materialization plus bounded-selection, global owned-materialization, and sorting counters.",
        "The complete 46-row counter ledger remains in track-b-counter-diff.csv.",
    ],
    "track_b_m57_status_sql": ["Selected identity, candidate, checksum, storage, owner, memory-proxy, and provenance facts from three complete M57 audit ledgers."],
    "logical_sql": ["37- and 59-character owned-versus-byte-backed fixture rows; timing fields intentionally excluded."],
    "findings_sql": ["All builder-authored evidence classifications retained; no row filter."],
    "priorities_sql": ["All seven evidence-backed future-work priorities retained; no milestone created."],
    "failures_sql": ["All setup failures, rejected lanes, and skipped-profiler limitations retained."],
}

additional_source_files = {
    "allocator_sql": [
        "analysis/controls/output/allocator-effects.csv",
    ],
    "cross_platform_sql": [
        "accepted-baseline",
        "repository/docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-4a-sentence-ordering/performance-ratchet",
    ],
    "memory_sql": [
        "historical/m59-luna-page-order-parity-verification-20260711/post-review-fix/run-1/memory-owner-profile.csv",
        "accepted-baseline/run-1/memory-owner-profile.csv",
    ],
    "track_b_m57_status_sql": [
        "analysis/m57-trackb/track-b-candidate-audit.csv",
        "analysis/m57-trackb/track-b-product-status-diff.csv",
        "analysis/m57-trackb/track-b-memory-owner-diff.csv",
        "analysis/m57-trackb/README.md",
    ],
}


def portable_source_path(path: str) -> str:
    external_prefix = "external-evidence/m59-post-fix-root-cause-20260711/"
    old_prefix = "external-evidence/m59-luna-page-order-parity-verification-20260711/"
    if path.startswith(external_prefix):
        return path.removeprefix(external_prefix)
    if path.startswith(old_prefix):
        return (
            "historical/m59-luna-page-order-parity-verification-20260711/"
            + path.removeprefix(old_prefix)
        )
    if path.startswith("docs/"):
        return "repository/" + path
    return path

query_sources = []
manifest_sources = []
generated_at = now()
for source_id, label, path, table_name, sql, description in source_specs:
    raw_path = portable_source_path(path)
    manifest_sources.append(
        {"id": source_id, "label": label, "path": "report/snapshot.sqlite"}
    )
    query_sources.append(
        {
            "id": source_id,
            "label": label,
            "path": "report/snapshot.sqlite",
            "query": {
                "engine": "sqlite",
                "sql": sql,
                "description": description,
                "tables_used": [table_name],
                "filters": source_filters[source_id],
                "source_files": list(
                    dict.fromkeys([raw_path, *additional_source_files.get(source_id, [])])
                ),
                "executed_at": generated_at,
                "metric_definitions": [
                    "Yune/librime ratio < 1 means Yune is faster; ratio > 1 means librime is faster.",
                    "All Windows ceilings and ratios in this report are diagnostic only and retain their signed authority unchanged.",
                ],
            },
        }
    )

reference_sources = [
    {
        "id": "accepted_packet",
        "label": "Accepted current-main five-round macOS packet",
        "path": "accepted-baseline",
    },
    {
        "id": "old_post_fix_packet",
        "label": "Earlier 89875ee2 post-review-fix packet",
        "path": "historical/m59-luna-page-order-parity-verification-20260711/post-review-fix",
    },
    {
        "id": "windows_signed",
        "label": "Signed Windows M59 Increment-0 baseline",
        "path": "repository/docs/reports/evidence/m59-closeout-baseline/gate-verdict.csv",
    },
    {
        "id": "windows_4a",
        "label": "Windows M59 Increment-4a near-code diagnostic",
        "path": "repository/docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-4a-sentence-ordering/performance-ratchet",
    },
    {
        "id": "m57_macos",
        "label": "M57 macOS sentence-model and Track B evidence",
        "path": "repository/docs/reports/evidence/m57-macos-track-a-sentence-model-parity",
    },
    {
        "id": "source_audit",
        "label": "Current Yune/librime comparative source audit",
        "path": "analysis/source-audit/README.md",
    },
    {
        "id": "build_provenance",
        "label": "Build, binary, and resource provenance audit",
        "path": "analysis/build-provenance/README.md",
    },
]
manifest_sources.extend(reference_sources)

cards = [
    {
        "id": "row_split_card",
        "description": "Current-main median-of-five Track A result; this is not a signed acceptance count.",
        "dataset": "headline",
        "sourceId": "headline_sql",
        "metrics": [
            {"label": "Yune-faster rows", "field": "yune_faster_rows", "format": "number"},
            {"label": "Yune-slower rows", "field": "yune_slower_rows", "format": "number"},
        ],
    },
    {
        "id": "long_37_card",
        "description": "Aggregate sequence ratio versus candidate-text-matched reconstructed prefix ratio.",
        "dataset": "headline",
        "sourceId": "headline_sql",
        "metrics": [
            {"label": "37-char aggregate ratio", "field": "ratio_37", "format": "number"},
            {
                "label": "Text-matched prefix ratio",
                "field": "text_matched_ratio_37",
                "format": "number",
            },
        ],
    },
    {
        "id": "long_59_card",
        "description": "Aggregate sequence ratio versus candidate-text-matched reconstructed prefix ratio.",
        "dataset": "headline",
        "sourceId": "headline_sql",
        "metrics": [
            {"label": "59-char aggregate ratio", "field": "ratio_59", "format": "number"},
            {
                "label": "Text-matched prefix ratio",
                "field": "text_matched_ratio_59",
                "format": "number",
            },
        ],
    },
    {
        "id": "short_work_card",
        "description": "Whole-process instructions divided by measured keys; startup is amortized but not subtracted.",
        "dataset": "headline",
        "sourceId": "headline_sql",
        "metrics": [
            {"label": "n instruction ratio", "field": "n_instruction_ratio", "format": "number"},
            {"label": "zh instruction ratio", "field": "zh_instruction_ratio", "format": "number"},
        ],
    },
    {
        "id": "track_b_card",
        "description": "Median of five current Track B run medians; absolute macOS product-guard value.",
        "dataset": "headline",
        "sourceId": "headline_sql",
        "metrics": [
            {"label": "Track B median, µs/key", "field": "track_b_median_us", "format": "number"},
            {"label": "Run-median spread, %", "field": "track_b_spread_pct", "format": "number"},
        ],
    },
    {
        "id": "rounds_card",
        "description": "Complete logical rounds retained with one fixed Yune and one fixed librime dylib hash.",
        "dataset": "headline",
        "sourceId": "headline_sql",
        "metrics": [{"label": "Accepted rounds", "field": "retained_rounds", "format": "number"}],
    },
]

charts = [
    {
        "id": "track_a_ratio_chart",
        "title": "Current-main macOS Yune/librime latency ratio",
        "subtitle": "Six rows favor Yune, but the long-row aggregate includes behavior-different intermediate prefixes.",
        "intent": "comparison",
        "question": "Which of the 17 Track A rows favor Yune or librime on current main?",
        "rationale": "A horizontal ratio bar keeps all exact row labels visible and makes the parity line explicit.",
        "comparisonContext": {
            "baseline": "Yune/librime parity at 1.0",
            "denominator": "librime median µs/key",
            "grain": "17 Track A inputs; median of five run ratios",
            "unit": "ratio",
        },
        "type": "horizontalBar",
        "dataset": "track_a",
        "sourceId": "track_a_sql",
        "encodings": {
            "x": {"field": "input_label", "type": "nominal", "label": "Input"},
            "y": {
                "field": "median_ratio",
                "type": "quantitative",
                "label": "Yune/librime ratio",
            },
        },
        "palette": {"kind": "sequential"},
        "labels": {"values": "auto"},
        "referenceLines": [
            {"axis": "x", "value": 1.0, "color": "neutral", "label": "Parity"}
        ],
        "settings": {"orientation": "horizontal", "sort": "none", "categoryLabelPolicy": "wrap"},
        "valueFormat": "number",
        "layout": "full",
        "maxRows": 17,
    },
    {
        "id": "prefix_ratio_chart",
        "title": "Per-prefix log2 latency ratio and candidate-text agreement",
        "subtitle": "Zero is parity; the four visible series separate 37/59-character prefixes by candidate-text match.",
        "intent": "relationship",
        "question": "Where do long-sequence latency ratios coincide with candidate-text agreement?",
        "rationale": "A log2 scatter keeps first-prefix outliers from flattening the remaining observations and makes behavior strata visible rather than hover-dependent.",
        "comparisonContext": {
            "baseline": "Yune/librime parity at 1.0",
            "denominator": "librime five-round prefix median",
            "grain": "one captured first-page prefix",
            "unit": "log2 ratio",
        },
        "type": "scatter",
        "dataset": "prefix_curve",
        "sourceId": "prefix_sql",
        "encodings": {
            "x": {"field": "key_index", "type": "quantitative", "label": "Prefix key index"},
            "y": {
                "field": "log2_ratio",
                "type": "quantitative",
                "label": "log2(Yune/librime ratio)",
            },
            "color": {"field": "chart_series", "type": "nominal", "label": "Input · behavior"},
        },
        "palette": {"kind": "categorical"},
        "legend": {"position": "bottom", "sort": "spec"},
        "labels": {"values": "none"},
        "referenceLines": [
            {"axis": "y", "value": 0.0, "color": "neutral", "label": "Parity"}
        ],
        "valueFormat": "number",
        "layout": "full",
        "maxRows": 120,
    },
    {
        "id": "instruction_ratio_chart",
        "title": "Whole-process instruction ratio in high-iteration controls",
        "subtitle": "Instruction direction is stable: Yune does much more short-row work and much less aggregate long-row work.",
        "intent": "comparison",
        "question": "Does executed work volume track the current latency direction?",
        "rationale": "Four bars make the instruction-volume split legible without implying an additive causal decomposition.",
        "comparisonContext": {
            "baseline": "Yune/librime parity at 1.0",
            "denominator": "librime whole-process instructions per measured key",
            "grain": "median paired ratio across five high-iteration processes",
            "unit": "ratio",
        },
        "type": "horizontalBar",
        "dataset": "time_l",
        "sourceId": "time_l_sql",
        "encodings": {
            "x": {"field": "input_label", "type": "nominal", "label": "Input"},
            "y": {
                "field": "instructions_ratio",
                "type": "quantitative",
                "label": "Yune/librime instructions",
            },
        },
        "palette": {"kind": "sequential"},
        "labels": {"values": "all"},
        "referenceLines": [
            {"axis": "x", "value": 1.0, "color": "neutral", "label": "Parity"}
        ],
        "settings": {"orientation": "horizontal", "sort": "none"},
        "valueFormat": "number",
        "layout": "full",
        "maxRows": 4,
    },
]


def col(field: str, label: str, kind: str = "text", unit: str | None = None) -> dict[str, object]:
    value: dict[str, object] = {"field": field, "label": label, "type": kind}
    if unit is not None:
        value["unit"] = unit
    return value


tables = [
    {
        "id": "track_a_table",
        "title": "Complete 17-row current-main comparison",
        "subtitle": "Five retained Mac ratios plus signed Windows and Increment-4a diagnostics; no new gate",
        "dataset": "track_a",
        "sourceId": "track_a_sql",
        "defaultSort": {"field": "median_ratio", "direction": "desc"},
        "density": "compact",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("input", "Exact input"),
            col("signed_scope", "M59 signing scope"),
            col("run1_ratio", "Run 1", "number", "×"),
            col("run2_ratio", "Run 2", "number", "×"),
            col("run3_ratio", "Run 3", "number", "×"),
            col("run4_ratio", "Run 4", "number", "×"),
            col("run5_ratio", "Run 5", "number", "×"),
            col("median_ratio", "Mac median", "number", "×"),
            col("pooled_worst_ratio", "Mac worst", "number", "×"),
            col("spread_pct", "Spread", "number", "%"),
            col("windows_signed_median", "Signed Win median", "number", "×"),
            col("windows_signed_ceiling", "Signed Win ceiling", "number", "×"),
            col("mac_vs_windows_signed_pct", "Mac vs signed Win", "number", "%"),
            col("mac_vs_windows_signed_class", "Diagnostic class"),
            col("windows_increment4a_median", "Win 4a ratio", "number", "×"),
            col("result", "Current result"),
        ],
    },
    {
        "id": "behavior_strata_table",
        "title": "Long-prefix behavior strata",
        "subtitle": "Candidate-text match only; comments/preedit may differ and no complete snapshot is exact",
        "dataset": "behavior_strata",
        "sourceId": "behavior_strata_sql",
        "defaultSort": {"field": "input_label", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("prefixes", "Prefixes", "number"),
            col("text_exact_prefixes", "Text matched", "number"),
            col("text_different_prefixes", "Text differed", "number"),
            col("all_prefix_ratio", "All-prefix ratio", "number", "×"),
            col("text_matched_prefix_ratio", "Text-matched ratio", "number", "×"),
            col("text_different_prefix_ratio", "Text-different ratio", "number", "×"),
            col("librime_time_share_text_different_pct", "librime time in text-different", "number", "%"),
            col("yune_faster_prefixes", "Yune-faster prefixes", "number"),
            col("yune_faster_text_exact_prefixes", "Yune-faster + text matched", "number"),
            col("final_key_ratio", "Final-key ratio", "number", "×"),
        ],
    },
    {
        "id": "reconciliation_table",
        "title": "Earlier post-fix versus reconciled current main",
        "subtitle": "Descriptive five-run comparison across different commits; not a single-change causal estimate",
        "dataset": "reconciliation",
        "sourceId": "reconciliation_sql",
        "defaultSort": {"field": "input_label", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("old_ratio", "898 ratio", "number", "×"),
            col("current_ratio", "Current ratio", "number", "×"),
            col("old_process_key_us", "898 Yune µs/key", "number"),
            col("current_process_key_us", "Current Yune µs/key", "number"),
            col("process_key_change_pct", "Process-key change", "number", "%"),
            col("old_graph_us", "898 graph µs/key", "number"),
            col("current_graph_us", "Current graph µs/key", "number"),
            col("graph_change_pct", "Graph change", "number", "%"),
        ],
    },
    {
        "id": "time_l_table",
        "title": "Instruction, cycle, CPI, and memory ratios",
        "subtitle": "Whole-process counters amortized by measured keys; startup and five warmups are not subtracted",
        "dataset": "time_l",
        "sourceId": "time_l_sql",
        "defaultSort": {"field": "instructions_ratio", "direction": "desc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("driver_latency_ratio", "Driver latency", "number", "×"),
            col("instructions_ratio", "Instructions", "number", "×"),
            col("cycles_ratio", "Cycles", "number", "×"),
            col("cpu_ratio", "CPU/key", "number", "×"),
            col("cpi_ratio", "CPI", "number", "×"),
            col("max_rss_ratio", "Max RSS", "number", "×"),
            col("peak_footprint_ratio", "Peak footprint", "number", "×"),
        ],
    },
    {
        "id": "cpu_owner_table",
        "title": "Qualitative process-key function owners",
        "subtitle": "Single optimized symbolized-sibling samples; owner tags overlap and are not accepted timing shares",
        "dataset": "cpu_owners",
        "sourceId": "cpu_owner_sql",
        "defaultSort": {"field": "engine", "direction": "asc"},
        "density": "compact",
        "layout": "full",
        "columns": [
            col("engine", "Engine"),
            col("input_label", "Input"),
            col("owner", "Inclusive owner tag"),
            col("samples", "Samples", "number"),
            col("process_key_samples", "Process-key denominator", "number"),
            col("share_pct", "Observed share", "number", "%"),
        ],
    },
    {
        "id": "cross_platform_table",
        "title": "Near-code absolute latency across macOS and Windows",
        "subtitle": "Mac afb7079 versus Windows ca52ec4; source, compiler, CPU, OS, and allocator are confounded",
        "dataset": "cross_platform",
        "sourceId": "cross_platform_sql",
        "defaultSort": {"field": "input_label", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("mac_yune_us", "Mac Yune, µs/key", "number"),
            col("windows_4a_yune_us", "Win 4a Yune, µs/key", "number"),
            col("mac_vs_windows_yune_pct", "Mac vs Win Yune", "number", "%"),
            col("mac_librime_us", "Mac librime, µs/key", "number"),
            col("windows_4a_librime_us", "Win 4a librime, µs/key", "number"),
            col("mac_vs_windows_librime_pct", "Mac vs Win librime", "number", "%"),
            col("mac_ratio", "Mac ratio", "number", "×"),
            col("windows_4a_ratio", "Win 4a ratio", "number", "×"),
        ],
    },
    {
        "id": "allocator_table",
        "title": "Allocator ratio controls",
        "subtitle": "Five rounds; short rows remain noise-sensitive and are preserved",
        "dataset": "allocator",
        "sourceId": "allocator_sql",
        "defaultSort": {"field": "nano_off_vs_default_ratio_change_pct", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("default_ratio", "Default", "number", "×"),
            col("nano_off_ratio", "Nano off", "number", "×"),
            col("nano_on_ratio", "Nano forced on", "number", "×"),
            col("nano_off_vs_default_ratio_change_pct", "Nano-off ratio change", "number", "%"),
            col("yune_nano_off_effect_pct", "Yune Nano-off effect", "number", "%"),
            col("librime_nano_off_effect_pct", "librime Nano-off effect", "number", "%"),
            col("yune_nano_off_classification", "Yune effect class"),
            col("librime_nano_off_classification", "librime effect class"),
            col("classification", "Classification"),
        ],
    },
    {
        "id": "api_table",
        "title": "API-path controls",
        "subtitle": "Context export favors Yune and does not reveal a hidden process-key advantage",
        "dataset": "api_modes",
        "sourceId": "api_sql",
        "defaultSort": {"field": "input_label", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("reference_ratio", "Reference ratio", "number", "×"),
            col("process_only_ratio", "Process-only ratio", "number", "×"),
            col("fresh_context_yune_us", "Fresh context Yune, µs", "number"),
            col("fresh_context_librime_us", "Fresh context librime, µs", "number"),
            col("stable_context_yune_us", "Stable context Yune, µs", "number"),
            col("stable_context_librime_us", "Stable context librime, µs", "number"),
        ],
    },
    {
        "id": "model_memory_table",
        "title": "Track A POET owner estimates before and after reconciliation",
        "subtitle": "Owner estimates are not additive with whole-process RSS",
        "dataset": "model_memory",
        "sourceId": "memory_sql",
        "defaultSort": {"field": "current_retained_mib", "direction": "desc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("owner", "Owner"),
            col("old_items", "898 items", "number"),
            col("current_items", "Current items", "number"),
            col("old_retained_mib", "898 retained MiB", "number"),
            col("current_retained_mib", "Current retained MiB", "number"),
            col("current_mapping_mode", "Current mode"),
        ],
    },
    {
        "id": "logical_volume_table",
        "title": "Fixture-only byte-backed logical work",
        "subtitle": "Both variants return five candidates; debug nanoseconds are intentionally omitted",
        "dataset": "logical_volume",
        "sourceId": "logical_sql",
        "defaultSort": {"field": "input_label", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("owned_candidates", "Owned candidates", "number"),
            col("byte_backed_candidates", "Byte-backed candidates", "number"),
            col("byte_vs_owned_table_entries_ratio", "Table entries", "number", "×"),
            col("byte_vs_owned_graph_entries_ratio", "Graph entries", "number", "×"),
            col("byte_vs_owned_dp_states_ratio", "DP states", "number", "×"),
        ],
    },
    {
        "id": "candidate_table",
        "title": "Final-page behavior matrix for all 17 Track A inputs",
        "subtitle": "All five current rounds are deterministic; exactness is against pinned librime",
        "dataset": "candidates",
        "sourceId": "candidate_sql",
        "defaultSort": {"field": "input_label", "direction": "asc"},
        "density": "compact",
        "layout": "full",
        "columns": [
            col("input_label", "Input"),
            col("yune_first_page", "Yune first page"),
            col("librime_first_page", "librime first page"),
            col("geometry_exact", "Geometry"),
            col("text_order_exact", "Text/order"),
            col("comments_exact", "Comments"),
            col("preedit_exact", "Preedit"),
            col("full_snapshot_exact", "Full snapshot"),
        ],
    },
    {
        "id": "track_b_table",
        "title": "All five Track B observations",
        "subtitle": "macOS product guard; not a Yune/librime peer lane",
        "dataset": "track_b",
        "sourceId": "track_b_sql",
        "defaultSort": {"field": "run", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("input", "Exact input"),
            col("run", "Run", "number"),
            col("median_us", "Median, µs/key", "number"),
            col("p95_us", "P95, µs/key", "number"),
            col("p99_us", "P99, µs/key", "number"),
            col("max_us", "Max, µs/key", "number"),
            col("median_working_set_mib", "Median working set, MiB", "number"),
            col("max_peak_working_set_mib", "Peak resident, MiB", "number"),
        ],
    },
    {
        "id": "track_b_counter_table",
        "title": "Focused Track B work-shape counters versus M57 full-pass-1",
        "subtitle": "Selected 8 of 46 counters; complete ledger retained; behavior is byte-identical and current counters are deterministic",
        "dataset": "track_b_counters",
        "sourceId": "track_b_counter_sql",
        "defaultSort": {"field": "change_pct", "direction": "desc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("counter", "Counter"),
            col("m57_pass1", "M57", "number"),
            col("current", "Current", "number"),
            col("change_pct", "Change", "number", "%"),
            col("all_current_runs_identical", "Five identical?"),
        ],
    },
    {
        "id": "track_b_m57_status_table",
        "title": "Track B behavior, checksum, storage, and model-owner status versus M57",
        "subtitle": "Two historical M57 passes versus five clean current rounds; descriptive across source states",
        "dataset": "track_b_m57_status",
        "sourceId": "track_b_m57_status_sql",
        "defaultSort": {"field": "category", "direction": "asc"},
        "density": "compact",
        "layout": "full",
        "columns": [
            col("category", "Category"),
            col("item", "Item"),
            col("m57_value", "M57"),
            col("current_value", "Current"),
            col("status", "Status / boundary"),
            col("evidence", "Evidence"),
        ],
    },
    {
        "id": "finding_table",
        "title": "Evidence classification",
        "subtitle": "Verified, qualitative, likely, rejected, noise-sensitive, and unresolved claims remain separate",
        "dataset": "findings",
        "sourceId": "findings_sql",
        "defaultSort": {"field": "finding", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("finding", "Finding"),
            col("classification", "Classification"),
            col("evidence", "Evidence"),
            col("implication", "Implication"),
        ],
    },
    {
        "id": "priority_table",
        "title": "Future milestone input",
        "subtitle": "Ordered diagnostic and optimization candidates; no milestone is created here",
        "dataset": "priorities",
        "sourceId": "priorities_sql",
        "defaultSort": {"field": "priority", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("priority", "Priority", "number"),
            col("work", "Work"),
            col("why", "Why"),
            col("required_guard", "Required guard"),
            col("risk", "Risk"),
        ],
    },
    {
        "id": "failure_table",
        "title": "Setup failures, rejected lanes, and skipped profilers",
        "subtitle": "Every measured red/rejected lane is preserved; only pre-measurement setup failures were retried",
        "dataset": "setup_failures",
        "sourceId": "failures_sql",
        "defaultSort": {"field": "stage", "direction": "asc"},
        "density": "spacious",
        "layout": "full",
        "columns": [
            col("stage", "Stage"),
            col("status", "Disposition"),
            col("detail", "Detail"),
        ],
    },
]

blocks = [
    {"id": "title", "type": "markdown", "body": f"# {TITLE}"},
    {
        "id": "executive_summary",
        "type": "markdown",
        "body": f"""## Executive Summary

**Yes—this is the missing post-fix diagnosis, and it materially changes the earlier report.** The prior final-page repair report measured Yune `{OLD_POST_FIX_COMMIT[:8]}`. Current main `{CURRENT_COMMIT[:8]}` includes the reconciled Windows Increment-4a mechanism; its 37/59 sentence-graph time is about 99% lower, so the old performance conclusion is stale.

The current answer is still not “Yune now has a broad macOS advantage.” Current main wins 6 of 17 aggregate Track A rows and loses 11. The 37- and 59-character aggregate ratios look excellent at **0.399** and **0.205**, but those key-sequence metrics include every incomplete prefix. Candidate text matches librime on only 19/37 and 30/59 prefixes; candidate-text-different prefixes consume **82.0%** and **90.1%** of librime's reconstructed prefix time. On the candidate-text-matched subset, Yune is instead **1.420×** and **1.204×** librime; at the final key it is **1.713×** and **1.139×**. Comments and preedit still prevent full semantic parity, so even those subset numbers are diagnostic—not acceptance metrics.

The short-row deficit is real work, not merely macOS timer noise: Yune executes **8.682×** the instructions for `n` and **4.092×** for `zh`. Symbolized samples qualitatively locate that work in expanded MARISA traversal and abbreviation/sentence-model generation. The clearest librime lesson is demand-driven page construction: librime keeps merged translation, uniqueness, and dictionary enumeration lazy, while Yune eagerly selects, owns, filters, and stores a surplus batch before exporting five candidates. Its exact causal savings remain unmeasured, and `n`/`zh` behavior must be locked to the oracle first.

macOS contributes but does not explain everything. Disabling Nano moves stable long/medium ratios about 6–14% in Yune's favor because librime slows more. Yune also has 24–41% worse CPI and much larger whole-process memory. Compiler/linker, CPU, allocator, and source differences remain confounded because no exact-current Windows packet exists. The evidence rejects both a pure platform-only story and a pure thermal/noise story.

This report is diagnostic only. It changes no signed Windows ceiling, baseline, exception, or milestone and authorizes no performance implementation.""",
    },
    {
        "id": "headline_metrics",
        "type": "metric-strip",
        "cardIds": [
            "row_split_card",
            "long_37_card",
            "long_59_card",
            "short_work_card",
            "track_b_card",
            "rounds_card",
        ],
    },
    {
        "id": "reconciliation_heading",
        "type": "markdown",
        "sourceId": "reconciliation_sql",
        "body": f"""## The reconciliation invalidates the earlier performance read

The earlier post-review-fix packet at `{OLD_POST_FIX_COMMIT[:8]}` did verify the repaired final 37/59 page text and order, but it still measured Yune ratios **2.428** and **1.809**. The reconciled `{CURRENT_COMMIT[:8]}` packet reverses those aggregates to **0.399** and **0.205**.

The mechanism-level movement is equally large: median Yune process-key time falls from **403.648→65.954 µs/key** on the 37-character sequence and **701.133→79.219 µs/key** on the 59-character sequence. Sentence-graph time falls **358.630→4.605** and **646.191→5.328 µs/key**. Because the commits differ by the complete 4a reconciliation and review fixes, this is a descriptive before/after result—not attribution to one line—but it proves the old report is not current-main performance evidence.""",
    },
    {"id": "reconciliation_evidence", "type": "table", "tableId": "reconciliation_table", "layout": "full"},
    {
        "id": "behavior_heading",
        "type": "markdown",
        "sourceId": "behavior_strata_sql",
        "body": """## The apparent long-row win disappears on candidate-text-matched prefixes

The final repaired first pages do match librime in candidate text/order, but the benchmark is an incremental key sequence. Its aggregate includes 96 separate prefixes, and no complete captured prefix snapshot is exact because preedit—and sometimes comments—still differs.

For the 37-character input, all 12 Yune-faster prefixes have different candidate text; candidate-text-different prefixes account for 82.0% of librime time. For the 59-character input, 25 of 26 Yune-faster prefixes have different candidate text and those prefixes account for 90.1% of librime time. The largest librime spikes are therefore real measurements of a different observable path, not clean evidence that Yune implements the same work faster.

The text-matched subset is only a sensitivity analysis. It controls candidate text/order hashes, not comments, preedit, uncaptured later pages, or selection state. Its 1.420×/1.204× ratios nevertheless show why the aggregate 0.399×/0.205× result must not be used as a behavior-normalized engine claim.""",
    },
    {"id": "prefix_chart", "type": "chart", "chartId": "prefix_ratio_chart", "layout": "full"},
    {"id": "behavior_strata_evidence", "type": "table", "tableId": "behavior_strata_table", "layout": "full"},
    {
        "id": "row_split_heading",
        "type": "markdown",
        "sourceId": "track_a_sql",
        "body": f"""## Eleven Track A rows still favor librime on current-main macOS

Across the exact 17-input set, Yune wins six aggregate rows and loses eleven. The largest deficits remain tiny inputs (`j` 7.462×, `b` 6.213×, `n` 4.123×, `yi` 4.102×, `zh` 3.261×). Ratios with very small librime denominators are naturally noise-sensitive, but five-run leave-one-out checks and the independent instruction controls preserve the direction.

The signed Windows Increment-0 medians and ceilings below are diagnostic context only. Their exact source commit is `{SIGNED_BASELINE_COMMIT}`; current main is not that commit, macOS absolute counters are not Windows counters, and no row is re-baselined or excused. The table explicitly marks the nine newly signed rows (`zh`, `j`, `yi`, `che`, `chuang`, `b`, `ceshi`, `zhongdengchangdu`, and `dazisudu`).""",
    },
    {"id": "track_a_chart", "type": "chart", "chartId": "track_a_ratio_chart", "layout": "full"},
    {"id": "track_a_evidence", "type": "table", "tableId": "track_a_table", "layout": "full"},
    {
        "id": "short_work_heading",
        "type": "markdown",
        "sourceId": "time_l_sql",
        "body": """## Short rows execute four to nine times the instructions

The 40 high-iteration process controls turn the short-versus-long split into a work-volume result. Yune uses **8.682×** the instructions, **10.730×** the cycles, and **10.704×** the CPU/key for `n`; `zh` uses **4.092×**, **5.061×**, and **5.048×**. In the aggregate long sequences Yune uses only **0.417×/0.212×** the instructions—but that direction inherits the prefix-behavior mismatch described above.

Instruction ratios vary only about 0.1–0.2% across the five rounds, which makes a pure scheduling-noise explanation implausible. CPI is still 1.236–1.405× worse for Yune in every lane, so work volume is not the only issue. These are whole-process totals divided by many measured keys: loader, initialization, five warmups, and teardown are amortized but not subtracted.""",
    },
    {"id": "instruction_chart", "type": "chart", "chartId": "instruction_ratio_chart", "layout": "full"},
    {"id": "time_l_evidence", "type": "table", "tableId": "time_l_table", "layout": "full"},
    {
        "id": "function_owner_heading",
        "type": "markdown",
        "body": """## Short MARISA and abbreviation paths are observed owners, not yet measured savings

After excluding setup frames, the symbolized Yune sibling shows overlapping short-row owner families: expanded compact-table/MARISA traversal, abbreviation sentence generation, and sentence word-graph work. Long Yune samples are now predominantly MARISA/table lookup rather than the former sentence-model graph. librime's long samples stay in `ScriptTranslation → Dictionary::Lookup → Table::Query/TableQuery` with substantial allocation leaves.

This upgrades function-family ownership from speculation to qualitative observation. It does **not** quantify production savings: the Yune binary is a debuginfo sibling, the fixed two-second delay failed to synchronize after warmups, the profiles are single fixed-order captures under active Spotlight/UI load, tags overlap, and librime `n` has only 40 process-key samples. The exact librime incomplete-prefix spike owner remains unresolved because profiles aggregate every prefix without a marker.""",
    },
    {"id": "cpu_owner_evidence", "type": "table", "tableId": "cpu_owner_table", "layout": "full"},
    {
        "id": "lazy_design_heading",
        "type": "markdown",
        "body": """## librime's lazy page pipeline is the clearest design lesson

Yune's current Luna request path asks every installed translator, converts the bounded result to owned candidates, eagerly filters/deduplicates/sorts, and stores the page candidate vector. `n` selects seven candidates to export five; generic keys request/store up to twenty to export five. Three translator calls occur per key, and current counters do not partition every producer or direct leading-family scan.

librime still builds composition, syllable graphs, collectors, and table accessors, but its `Translation`, merged streams, uniquifier, dictionary entries, and `Menu::Prepare` are generator-driven. It advances far enough to construct the requested page and resumes when filtering or deduplication consumes rows. That structure plausibly explains disproportionate fixed/surplus cost on short keys.

This is the best design direction to study, not a ready fix. A page-fill prototype must be filter-aware and resumable, and must preserve candidate order, comments, preedit, pagination, selection identity, deduplication, and every named profile. Current `n` and `zh` pages already differ from librime beyond the first candidate, so behavior authority comes first.""",
    },
    {
        "id": "platform_heading",
        "type": "markdown",
        "body": """## macOS helps librime, but only part of the residual is platform-specific

The stable allocator control is real: disabling Nano slows librime more than Yune on the long/medium rows, lowering Yune/librime ratios by roughly 6–14%. The symbolized samples retain the same high-level routes with Nano off and shift allocator leaf shape, which supports allocator mechanics rather than an algorithm switch. Short allocator rows change sign and remain noise-sensitive.

The near-code Windows 4a comparison shows two different patterns. On long rows, Mac Yune is close to Windows (−1.6%/−6.0%) while Mac librime is about 44.3%/41.9% faster. On `n`/`ni`/`hao`, Mac librime is similar (about +6.5%/−3.1%/−9.0%) while Mac Yune is 52.8%/43.8%/36.8% slower. A single “librime is optimized for macOS” theory cannot explain both directions.

The comparison is not causal: Windows measured `ca52ec42`, Mac measured `afb7079b`, and CPU, compiler, linker, allocator, OS, payload metadata, and background load differ. The only valid conclusion is mixed engine-path and platform interaction; an exact-current matched lane is still required.""",
    },
    {"id": "cross_platform_evidence", "type": "table", "tableId": "cross_platform_table", "layout": "full"},
    {"id": "allocator_evidence", "type": "table", "tableId": "allocator_table", "layout": "full"},
    {"id": "api_evidence", "type": "table", "tableId": "api_table", "layout": "full"},
    {
        "id": "memory_heading",
        "type": "markdown",
        "body": """## Memory remains a separate unresolved pressure

The high-iteration controls report Yune max RSS 11.5–18.3× librime and peak footprint 23.5–26.4×. These are whole-process peaks—not per-key bytes and not proof of latency causation—but they make memory a serious independent gap and a plausible contributor to the unresolved CPI difference.

Reconciliation already removed one large owner: POET vocabulary falls from roughly 47.7 MiB/421,966 items to 0.027 MiB/193 items. Current Track A still retains about 20.4 MiB across 513,353 POET entries and the 332,604-row lookup index, in addition to mapped compact-table/prism resources and process/runtime overhead.

The deployed `YUNE_POET_BYTE_BACKED=1` control cannot be timed: it emitted zero candidates versus five on all 99 prefixes. The fixture implementation returns five candidates, but examines 13–23× more table entries, 14–22× more graph entries, and 17–19× more DP states than its owned incremental fixture. Byte-backing may reduce ownership; it is not automatically a CPU optimization and needs behavior plus incremental/lazy indexing design first.""",
    },
    {"id": "memory_evidence", "type": "table", "tableId": "model_memory_table", "layout": "full"},
    {"id": "logical_volume_evidence", "type": "table", "tableId": "logical_volume_table", "layout": "full"},
    {
        "id": "behavior_boundary_heading",
        "type": "markdown",
        "sourceId": "candidate_sql",
        "body": """## Final-page behavior is deterministic but not uniformly oracle-exact

All five current candidate snapshots and every captured non-time counter are deterministic. Final-page candidate text/order matches pinned librime on 9 of 17 Track A inputs; only `cszysmsrsd` and `zybfshmsru` match the complete captured geometry/text/comments/preedit semantics. The 37/59 final candidate text/order/comments match, while preedit spacing still differs. `n` and `zh` differ beyond their first candidate.

This is not merely a reporting caveat: optimizing a behavior-different path can preserve or amplify the wrong page. Any future performance milestone must name its oracle and lock the relevant incremental prefix/page semantics before measuring a speedup.""",
    },
    {"id": "candidate_evidence", "type": "table", "tableId": "candidate_table", "layout": "full"},
    {
        "id": "track_b_heading",
        "type": "markdown",
        "body": """## Track B behavior is stable while its work shape moved

Track B's current first page is byte-identical to both retained M57 passes, including comments and page state. Its five run medians are 265.696, 264.941, 263.880, 268.620, and 263.914 µs/key: robust median **264.941**, worst run median **268.620**, spread **1.8%**. Runs 4 and 5 preserve tail spikes of 559.840 and 642.907 µs/key, so no tail improvement is claimed.

Checksums, mmap storage, entry counts, and no-POET ownership remain stable. The internal route changed: Track-B-specific candidates materialized +95.2% and bounded selections +110.9%, while global owned candidates and sorting fell 48–51%. These counters are different definitions and must not be added. Track B remains a Yune product guard, not a Yune/librime peer lane, and its absolute Mac/Windows values are not interchangeable.""",
    },
    {"id": "track_b_evidence", "type": "table", "tableId": "track_b_table", "layout": "full"},
    {"id": "track_b_counter_evidence", "type": "table", "tableId": "track_b_counter_table", "layout": "full"},
    {"id": "track_b_m57_status_evidence", "type": "table", "tableId": "track_b_m57_status_table", "layout": "full"},
    {
        "id": "methodology_heading",
        "type": "markdown",
        "body": f"""## Scope and measurement design

The accepted packet measures clean detached Yune `{CURRENT_COMMIT}` and clean pinned librime `{PINNED_LIBRIME_COMMIT}` on a MacBook Air Mac17,3 (Apple M5, 10 cores, 16 GB), macOS 26.5.1 (25F80), APFS, AC power, 100% battery, Low Power Mode disabled, and no recorded thermal/performance warning. Rust/Cargo are 1.96.1; Command Line Tools 26.6 / Apple clang 21.0.0.

The exact official macOS in-process script ran five complete logical rounds with all 17 Track A inputs, Track B, 9 startup iterations, 60 session iterations, 80 key iterations, and product deployment. Measurement interval was `2026-07-11T21:17:44Z`–`2026-07-11T21:30:27Z`. The five exact run roots are:

- `/Users/laufei/yune-m59-post-fix-root-cause-20260711/accepted-baseline/run-1`
- `/Users/laufei/yune-m59-post-fix-root-cause-20260711/accepted-baseline/run-2`
- `/Users/laufei/yune-m59-post-fix-root-cause-20260711/accepted-baseline/run-3`
- `/Users/laufei/yune-m59-post-fix-root-cause-20260711/accepted-baseline/run-4`
- `/Users/laufei/yune-m59-post-fix-root-cause-20260711/accepted-baseline/run-5`

Every Yune dylib is SHA-256 `{YUNE_SHA256}`; every librime dylib is `{LIBRIME_SHA256}`. Post-run hashes remained identical. Both engines used byte-identical table/prism/reverse payloads within each run. The Track A prism's raw four-byte schema checksum varies between runs, but all other normalized payload bytes are identical and both engines share the per-run file. The detached measurement worktree and pinned source are clean; the primary repository retains only the user's pre-existing `.codex/config.toml` modification.

The accepted baseline began after an unmeasured setup warmup. It was not a perfectly idle host—ChatGPT/Codex and other UI processes remained active, and Spotlight rose later—but no benchmark compilation ran concurrently and the repeat packet retains every observation. Independent controls then tested metrics on/off, allocator modes, API path, prefix payload/timing, process counters, storage behavior, and qualitative profiles. No signed threshold was applied or changed.""",
    },
    {"id": "failure_evidence", "type": "table", "tableId": "failure_table", "layout": "full"},
    {
        "id": "robustness_heading",
        "type": "markdown",
        "body": """## What is proven, rejected, and unresolved

The five-run packet, 121 baseline validations, 394 timing-control checks, 246 hardware-counter checks, 108 CPU-sample structural checks, and 55 build/provenance checks establish fixed identities, deterministic outputs, the row-class split, short instruction-volume excess, allocator contribution, API negative result, and behavior-stratified long-prefix result.

They do not establish an additive causal decomposition. Yune-only internal timers have no librime equivalent; metrics-on tax is small but nested phase timers overlap. Prefix text hashes cover the captured first page, not every future page. Whole-process counters retain amortized initialization. Profiles are qualitative and have a warmup synchronization defect. Compiler/linker and exact-current Windows effects remain unpaired. Memory-to-CPI causality, the direct-family translator residual, and librime's exact spike-prefix owner remain unresolved.

Thermal/noise is not the primary explanation: no warning was recorded and instruction direction is exceptionally stable. It still limits fine-grained absolute comparisons, especially `hao`, tiny-denominator short allocator rows, Track B tails, and fixed-order profile percentages.""",
    },
    {"id": "finding_evidence", "type": "table", "tableId": "finding_table", "layout": "full"},
    {
        "id": "priorities_heading",
        "type": "markdown",
        "body": """## Future milestones should start with behavior locks

The highest-priority issue is not a micro-optimization: it is incremental-prefix comparability. The benchmark's strongest aggregate wins occur where candidate text differs. The next diagnostic should first make the governing librime page, comments, preedit, pagination, and selection identity explicit for every measured prefix.

After that, the best optimization hypothesis is a filter-aware lazy page-fill pipeline modeled on librime's demand-driven streams, supported by lower-perturbation producer and translator-residual attribution. Short MARISA/abbreviation work, memory/byte-backed behavior, exact-current cross-platform pairing, and Track B overfetch follow. Sentence-DP and ABI export are not first targets: the former is now a small long-row share with reuse active, and the latter is below about 1.2% and favors Yune in direct controls.

These are inputs to future planning only. This report neither creates a milestone nor authorizes implementation.""",
    },
    {"id": "priority_evidence", "type": "table", "tableId": "priority_table", "layout": "full"},
    {
        "id": "further_questions",
        "type": "markdown",
        "body": """## Questions the next lane must answer

- Which incremental `luna_pinyin` prefix semantics are required by the named target, and which current differences are intentional versus missed oracle parity?
- Can a resumable page-fill pipeline remove surplus materialization while preserving every filter, deduplication, pagination, comment, preedit, and selection invariant?
- Which producer/direct leading-family scans own the current translator residual after low-perturbation attribution is added?
- Does an exact-current Windows/macOS lane reproduce the short Yune slowdown and long librime speedup with identical payloads and allocator declarations?
- Which cache, compiler, branch, or memory effect explains Yune's 1.24–1.41× CPI once work volume is held comparable?
- Can byte-backed POET recover full prefix behavior and memory savings without losing incremental reuse or multiplying logical graph work?
- Which specific librime incomplete-prefix dictionary/table/allocation path produces the measured spikes when sampled with a synchronized per-prefix marker?""",
    },
]

artifact = {
    "surface": "report",
    "manifest": {
        "version": 1,
        "surface": "report",
        "title": TITLE,
        "description": "Current-main post-page-order-fix macOS root-cause diagnosis of remaining Yune/librime performance, behavior, platform, and memory gaps; diagnostic only.",
        "generatedAt": generated_at,
        "cards": cards,
        "charts": charts,
        "tables": tables,
        "sources": manifest_sources,
        "blocks": blocks,
    },
    "snapshot": {
        "version": 1,
        "generatedAt": generated_at,
        "status": "ready",
        "datasets": {
            "headline": headline,
            "track_a": track_a,
            "prefix_curve": prefix_curve,
            "behavior_strata": behavior_strata,
            "time_l": time_l,
            "reconciliation": reconciliation,
            "allocator": allocator,
            "api_modes": api_modes,
            "cross_platform": cross_platform,
            "candidates": candidates,
            "cpu_owners": cpu_owners,
            "model_memory": model_memory,
            "track_b": track_b,
            "track_b_counters": track_b_counters,
            "track_b_m57_status": track_b_m57_status,
            "logical_volume": logical_volume,
            "findings": findings,
            "priorities": priorities,
            "setup_failures": setup_failures,
        },
    },
    "sources": query_sources,
}

REPORT.mkdir(parents=True, exist_ok=True)
(REPORT / "artifact.json").write_text(
    json.dumps(artifact, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
)
(REPORT / "report-source-notes.md").write_text(
    f"""# Report source notes

- Audience: technical.
- Delivery mode: Data Analytics MCP report artifact; `artifact.json` is the canonical bounded source payload.
- Query provenance: every exposed SQL statement executes against `snapshot.sqlite`; each query declares its material selection filters and raw source file(s).
- Decision question: why current post-fix Yune does not show a broad, behavior-normalized macOS advantage over pinned librime, and which remaining gaps should inform future milestones.
- Current identities: Yune `{CURRENT_COMMIT}` / `{YUNE_SHA256}`; librime `{PINNED_LIBRIME_COMMIT}` / `{LIBRIME_SHA256}`.
- Signed Windows comparison source: Yune `{SIGNED_BASELINE_COMMIT}`; diagnostic context only.
- Supersession boundary: the performance interpretation in the earlier `{OLD_POST_FIX_COMMIT}` report is stale for current main; the historical page-parity evidence remains retained.
- Authority boundary: no signed ceiling, baseline, exception, or milestone is created or changed.

## Chart map

1. `track_a_ratio_chart` — category comparison; all 17 current median ratios; single sequential root plus parity line; supports the 6/17 versus 11/17 row-class result.
2. `prefix_ratio_chart` — 96-observation log2 scatter with four visible input/behavior series and a zero-parity line; supports spike concentration without outlier compression or hover-dependent semantics.
3. `instruction_ratio_chart` — four-row category comparison with a parity line; exact cycles/CPI/CPU/RSS context remains in the adjacent table.

## Omitted visuals

- Reconciliation, behavior strata, allocator, API, cross-platform absolute values, memory owners, Track B, and setup failures are exact-lookup questions with two to nine rows; tables are more honest than underpowered trends.
- CPU profile shares are overlapping qualitative tags with a warmup defect, so a chart would imply false precision; a labeled table is used.
- No additive waterfall is shown because the measured drivers are overlapping and do not sum to the observed gap.

## Required caveats

- Candidate-text matching is not full semantic parity; comments/preedit and uncaptured pages remain.
- The prefix sum is a diagnostic reconstruction, not an acceptance metric.
- Hardware counters are whole-process totals amortized per measured key.
- CPU samples identify function families only; they are not latency or instruction attribution.
- Windows comparisons are diagnostic and source/platform confounded.
""",
    encoding="utf-8",
)

source_manifest = []
for path in sorted(REPORT.iterdir()):
    if path.name == "source-manifest.csv" or not path.is_file():
        continue
    source_manifest.append(
        {"file": path.name, "sha256": sha256(path), "bytes": path.stat().st_size}
    )
with (REPORT / "source-manifest.csv").open("w", encoding="utf-8", newline="") as handle:
    writer = csv.DictWriter(handle, fieldnames=["file", "sha256", "bytes"])
    writer.writeheader()
    writer.writerows(source_manifest)

print(REPORT / "artifact.json")
