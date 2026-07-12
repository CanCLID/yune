#!/usr/bin/env python3
"""Independent fail-closed validation for the post-fix root-cause artifact."""

from __future__ import annotations

import csv
import hashlib
import json
import math
import sqlite3
import statistics
import subprocess
from pathlib import Path


ROOT = Path("/Users/laufei/yune-m59-post-fix-root-cause-20260711")
REPORT = ROOT / "report"
WORKTREE = Path("/Users/laufei/Documents/GitHub/yune-m59-post-fix-root-cause")
PRIMARY = Path("/Users/laufei/Documents/GitHub/yune")
LIBRIME = Path("/Users/laufei/librime-m59-pinned")
CURRENT_COMMIT = "afb7079b71f7f9353845114ff3e310c0a38b9b87"
LIBRIME_COMMIT = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
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


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8-sig", newline="") as handle:
        return list(csv.DictReader(handle))


def git(path: Path, *args: str) -> str:
    return subprocess.check_output(["git", "-C", str(path), *args], text=True).strip()


def resolve_portable_path(path: str) -> Path:
    if path.startswith("repository/"):
        return WORKTREE / path.removeprefix("repository/")
    if path.startswith("historical/m59-luna-page-order-parity-verification-20260711/"):
        return Path("/Users/laufei/yune-m59-luna-page-order-parity-verification-20260711") / path.removeprefix(
            "historical/m59-luna-page-order-parity-verification-20260711/"
        )
    return ROOT / path


checks: list[dict[str, object]] = []


def check(name: str, condition: bool, detail: str) -> None:
    checks.append({"check": name, "status": "pass" if condition else "fail", "detail": detail})
    if not condition:
        raise RuntimeError(f"{name}: {detail}")


artifact_path = REPORT / "artifact.json"
artifact = json.loads(artifact_path.read_text(encoding="utf-8"))
manifest = artifact["manifest"]
datasets = artifact["snapshot"]["datasets"]

check("surface", artifact["surface"] == "report", artifact["surface"])
check("snapshot_ready", artifact["snapshot"]["status"] == "ready", artifact["snapshot"]["status"])
check("title", manifest["title"] == "M59 post-fix macOS performance root-cause diagnostic", manifest["title"])
check("first_title_block", manifest["blocks"][0]["body"] == f"# {manifest['title']}", manifest["blocks"][0]["body"])
check("executive_summary_second", manifest["blocks"][1]["body"].startswith("## Executive Summary"), manifest["blocks"][1]["id"])
check("chart_count", len(manifest["charts"]) == 3, f"charts={len(manifest['charts'])}")
check("table_count", len(manifest["tables"]) == 17, f"tables={len(manifest['tables'])}")
check("payload_bound", artifact_path.stat().st_size < 3_000_000, f"bytes={artifact_path.stat().st_size}")
check("dataset_count_bound", len(datasets) <= 50, f"datasets={len(datasets)}")
check("row_bound", all(len(rows) <= 2000 for rows in datasets.values()), str({k: len(v) for k, v in datasets.items()}))

source_ids = {source["id"] for source in manifest["sources"]}
check("unique_source_ids", len(source_ids) == len(manifest["sources"]), f"sources={len(source_ids)}")
asset_ids = {
    item["id"] for family in ["cards", "charts", "tables"] for item in manifest[family]
}
check("unique_asset_ids", len(asset_ids) == sum(len(manifest[x]) for x in ["cards", "charts", "tables"]), f"assets={len(asset_ids)}")
for family in ["cards", "charts", "tables"]:
    for item in manifest[family]:
        check(f"source_{family}_{item['id']}", item.get("sourceId") in source_ids, str(item.get("sourceId")))
        check(f"dataset_{family}_{item['id']}", item["dataset"] in datasets, item["dataset"])
for block in manifest["blocks"]:
    if "sourceId" in block:
        check(f"block_source_{block['id']}", block["sourceId"] in source_ids, block["sourceId"])
    if block["type"] == "chart":
        check(f"block_chart_{block['id']}", block["chartId"] in {x["id"] for x in manifest["charts"]}, block["chartId"])
    if block["type"] == "table":
        check(f"block_table_{block['id']}", block["tableId"] in {x["id"] for x in manifest["tables"]}, block["tableId"])

expected_counts = {
    "headline": 1,
    "track_a": 17,
    "prefix_curve": 96,
    "behavior_strata": 2,
    "time_l": 4,
    "reconciliation": 2,
    "allocator": 9,
    "api_modes": 4,
    "cross_platform": 5,
    "candidates": 17,
    "model_memory": 4,
    "track_b": 5,
    "track_b_counters": 8,
    "track_b_m57_status": 13,
    "logical_volume": 2,
    "findings": 15,
    "priorities": 7,
    "setup_failures": 6,
}
for name, expected in expected_counts.items():
    check(f"dataset_count_{name}", len(datasets[name]) == expected, f"observed={len(datasets[name])} expected={expected}")
check("cpu_owner_nonempty", len(datasets["cpu_owners"]) >= 20, f"rows={len(datasets['cpu_owners'])}")

source_to_dataset = {
    "headline_sql": "headline",
    "track_a_sql": "track_a",
    "prefix_sql": "prefix_curve",
    "behavior_strata_sql": "behavior_strata",
    "time_l_sql": "time_l",
    "reconciliation_sql": "reconciliation",
    "allocator_sql": "allocator",
    "api_sql": "api_modes",
    "cross_platform_sql": "cross_platform",
    "candidate_sql": "candidates",
    "cpu_owner_sql": "cpu_owners",
    "memory_sql": "model_memory",
    "track_b_sql": "track_b",
    "track_b_counter_sql": "track_b_counters",
    "track_b_m57_status_sql": "track_b_m57_status",
    "logical_sql": "logical_volume",
    "findings_sql": "findings",
    "priorities_sql": "priorities",
    "failures_sql": "setup_failures",
}
query_source_by_id = {source["id"]: source for source in artifact["sources"]}
check(
    "query_source_set",
    set(query_source_by_id) == set(source_to_dataset),
    str(sorted(query_source_by_id)),
)
for source_id, dataset_name in source_to_dataset.items():
    source = query_source_by_id[source_id]
    check(
        f"query_portable_path_{source_id}",
        not Path(source["path"]).is_absolute() and ".." not in Path(source["path"]).parts,
        source["path"],
    )
    database = resolve_portable_path(source["path"])
    check(f"query_database_{source_id}", database.is_file(), str(database))
    check(
        f"query_filters_{source_id}",
        bool(source["query"].get("filters")),
        str(source["query"].get("filters")),
    )
    raw_inputs = source["query"].get("source_files", [])
    check(
        f"query_raw_sources_{source_id}",
        bool(raw_inputs)
        and all(
            not Path(path).is_absolute()
            and ".." not in Path(path).parts
            and resolve_portable_path(path).exists()
            for path in raw_inputs
        ),
        str(raw_inputs),
    )
    connection = sqlite3.connect(database)
    result = connection.execute(source["query"]["sql"]).fetchall()
    connection.close()
    check(
        f"query_row_count_{source_id}",
        len(result) == len(datasets[dataset_name]),
        f"query={len(result)} dataset={len(datasets[dataset_name])}",
    )

for source in manifest["sources"]:
    check(
        f"manifest_source_path_{source['id']}",
        not Path(source["path"]).is_absolute()
        and ".." not in Path(source["path"]).parts
        and resolve_portable_path(source["path"]).exists(),
        source["path"],
    )

track_a = datasets["track_a"]
check("track_a_faster_count", sum(row["median_ratio"] < 1 for row in track_a) == 6, str(sum(row["median_ratio"] < 1 for row in track_a)))
check("track_a_slower_count", sum(row["median_ratio"] >= 1 for row in track_a) == 11, str(sum(row["median_ratio"] >= 1 for row in track_a)))
by_input = {row["input"]: row for row in track_a}
check(
    "newly_signed_rows",
    {
        row["input"]
        for row in track_a
        if row["signed_scope"] == "newly signed in M59 Increment-0"
    }
    == NEWLY_SIGNED_INPUTS,
    str(sorted(row["input"] for row in track_a if row["signed_scope"] == "newly signed in M59 Increment-0")),
)
for input_value, expected in [(LONG_37, 0.399), (LONG_59, 0.205), ("n", 4.123), ("zh", 3.261)]:
    check(f"ratio_{len(input_value) if len(input_value)>3 else input_value}", math.isclose(by_input[input_value]["median_ratio"], expected, abs_tol=0.0005), str(by_input[input_value]["median_ratio"]))

strata = {row["characters"]: row for row in datasets["behavior_strata"]}
expected_strata = {
    37: (19, 18, 1.420, 0.230, 82.0, 0, 1.713),
    59: (30, 29, 1.204, 0.127, 90.1, 1, 1.139),
}
for chars, expected in expected_strata.items():
    row = strata[chars]
    observed = (
        row["text_exact_prefixes"],
        row["text_different_prefixes"],
        row["text_matched_prefix_ratio"],
        row["text_different_prefix_ratio"],
        row["librime_time_share_text_different_pct"],
        row["yune_faster_text_exact_prefixes"],
        row["final_key_ratio"],
    )
    check(f"strata_{chars}", observed == expected, f"observed={observed} expected={expected}")
    check(f"no_full_snapshot_{chars}", row["full_snapshot_exact_prefixes"] == 0, str(row["full_snapshot_exact_prefixes"]))

time_l = {row["input"]: row for row in datasets["time_l"]}
for input_value, instructions, cycles, cpi in [
    ("n", 8.682, 10.730, 1.236),
    ("zh", 4.092, 5.061, 1.237),
    (LONG_37, 0.417, 0.585, 1.405),
    (LONG_59, 0.212, 0.293, 1.383),
]:
    row = time_l[input_value]
    check(f"time_l_{len(input_value) if len(input_value)>3 else input_value}", (row["instructions_ratio"], row["cycles_ratio"], row["cpi_ratio"]) == (instructions, cycles, cpi), str(row))

recon = {row["input_label"]: row for row in datasets["reconciliation"]}
for label, expected in [
    ("37-character", (2.428, 0.399, 358.630, 4.605)),
    ("59-character", (1.809, 0.205, 646.191, 5.328)),
]:
    row = recon[label]
    observed = (row["old_ratio"], row["current_ratio"], row["old_graph_us"], row["current_graph_us"])
    check(f"reconciliation_{label}", observed == expected, f"observed={observed}")

candidate_rows = datasets["candidates"]
check("candidate_text_exact_count", sum(row["text_order_exact"] == "yes" for row in candidate_rows) == 9, str(sum(row["text_order_exact"] == "yes" for row in candidate_rows)))
check("candidate_full_exact_count", sum(row["full_snapshot_exact"] == "yes" for row in candidate_rows) == 2, str(sum(row["full_snapshot_exact"] == "yes" for row in candidate_rows)))

track_b = datasets["track_b"]
check("track_b_exact_input", {row["input"] for row in track_b} == {TRACK_B}, str({row["input"] for row in track_b}))
track_b_medians = [row["median_us"] for row in track_b]
check("track_b_median", statistics.median(track_b_medians) == 264.941, str(statistics.median(track_b_medians)))
check("track_b_worst", max(row["max_us"] for row in track_b) == 642.907, str(max(row["max_us"] for row in track_b)))

track_b_status = {row["item"]: row for row in datasets["track_b_m57_status"]}
check(
    "track_b_candidate_hash",
    track_b_status["Raw candidate CSV SHA-256"]["current_value"]
    == "5fe17ccb53dd8ee40d9ceeb00dc9c7aab0cc30e3dfdf8216914cf675b7e597e9",
    track_b_status["Raw candidate CSV SHA-256"]["current_value"],
)
check(
    "track_b_main_checksum",
    track_b_status["jyut6ping3 source/table checksum"]["current_value"] == "0xf6589c0c",
    track_b_status["jyut6ping3 source/table checksum"]["current_value"],
)
check(
    "track_b_scolar_checksum",
    track_b_status["jyut6ping3_scolar source/table checksum"]["current_value"] == "0x822bccba",
    track_b_status["jyut6ping3_scolar source/table checksum"]["current_value"],
)
check(
    "track_b_poet_owner_zero",
    "0 retained bytes" in track_b_status["POET entries/index/abbreviation owners"]["current_value"],
    track_b_status["POET entries/index/abbreviation owners"]["current_value"],
)

allocator_rows = {row["input_label"]: row for row in datasets["allocator"]}
check(
    "allocator_37_per_engine",
    allocator_rows["37-character"]["yune_nano_off_effect_pct"] == 7.0
    and allocator_rows["37-character"]["librime_nano_off_effect_pct"] == 19.4,
    str(allocator_rows["37-character"]),
)
check(
    "allocator_59_per_engine",
    allocator_rows["59-character"]["yune_nano_off_effect_pct"] == 4.4
    and allocator_rows["59-character"]["librime_nano_off_effect_pct"] == 22.9,
    str(allocator_rows["59-character"]),
)

chart_by_id = {chart["id"]: chart for chart in manifest["charts"]}
check(
    "horizontal_reference_axes",
    all(
        chart_by_id[chart_id]["referenceLines"][0]["axis"] == "x"
        for chart_id in ["track_a_ratio_chart", "instruction_ratio_chart"]
    ),
    str({chart_id: chart_by_id[chart_id]["referenceLines"] for chart_id in ["track_a_ratio_chart", "instruction_ratio_chart"]}),
)
check(
    "prefix_chart_behavior_visible",
    chart_by_id["prefix_ratio_chart"]["type"] == "scatter"
    and chart_by_id["prefix_ratio_chart"]["encodings"]["color"]["field"] == "chart_series"
    and chart_by_id["prefix_ratio_chart"]["encodings"]["y"]["field"] == "log2_ratio",
    str(chart_by_id["prefix_ratio_chart"]),
)
check(
    "charts_do_not_promise_unsupported_tooltips",
    all("tooltip" not in chart["encodings"] for chart in manifest["charts"]),
    str([chart["id"] for chart in manifest["charts"] if "tooltip" in chart["encodings"]]),
)

table_by_id = {table["id"]: table for table in manifest["tables"]}
check(
    "track_b_counter_subset_disclosed",
    "Focused" in table_by_id["track_b_counter_table"]["title"]
    and "Selected 8 of 46" in table_by_id["track_b_counter_table"]["subtitle"],
    table_by_id["track_b_counter_table"]["title"]
    + " | "
    + table_by_id["track_b_counter_table"]["subtitle"],
)
check(
    "cpu_owner_grouped_default",
    table_by_id["cpu_owner_table"]["defaultSort"]
    == {"field": "engine", "direction": "asc"},
    str(table_by_id["cpu_owner_table"]["defaultSort"]),
)

methodology = next(block["body"] for block in manifest["blocks"] if block["id"] == "methodology_heading")
for run in range(1, 6):
    run_path = str(ROOT / "accepted-baseline" / f"run-{run}")
    check(f"methodology_run_path_{run}", run_path in methodology, run_path)
row_split_body = next(block["body"] for block in manifest["blocks"] if block["id"] == "row_split_heading")
check(
    "signed_source_commit_visible",
    "457751824b8944676dc44912b9ce31ff29d78403" in row_split_body,
    row_split_body,
)

check("worktree_commit", git(WORKTREE, "rev-parse", "HEAD") == CURRENT_COMMIT, git(WORKTREE, "rev-parse", "HEAD"))
check("worktree_clean", git(WORKTREE, "status", "--porcelain=v1", "--untracked-files=all") == "", git(WORKTREE, "status", "--porcelain=v1", "--untracked-files=all"))
check("librime_commit", git(LIBRIME, "rev-parse", "HEAD") == LIBRIME_COMMIT, git(LIBRIME, "rev-parse", "HEAD"))
check("librime_clean", git(LIBRIME, "status", "--porcelain=v1", "--untracked-files=all") == "", git(LIBRIME, "status", "--porcelain=v1", "--untracked-files=all"))
primary_status = git(PRIMARY, "status", "--porcelain=v1", "--untracked-files=all")
check("primary_only_user_config_dirty", primary_status == "M .codex/config.toml", primary_status)

check("yune_hash", sha256(ROOT / "accepted-baseline" / "frozen-binaries" / "libyune_rime_api.dylib") == YUNE_SHA256, sha256(ROOT / "accepted-baseline" / "frozen-binaries" / "libyune_rime_api.dylib"))
check("librime_hash", sha256(ROOT / "accepted-baseline" / "frozen-binaries" / "librime.dylib") == LIBRIME_SHA256, sha256(ROOT / "accepted-baseline" / "frozen-binaries" / "librime.dylib"))

for run in range(1, 6):
    run_path = ROOT / "accepted-baseline" / f"run-{run}"
    check(f"run_{run}_exists", run_path.is_dir(), str(run_path))
    recorded = (run_path / "yune-dylib-post-run.sha256").read_text(encoding="utf-8").split()[0]
    check(f"run_{run}_yune_hash", recorded == YUNE_SHA256, recorded)

validation_receipts = [
    (ROOT / "analysis" / "output" / "validation-checks.csv", 121),
    (ROOT / "analysis" / "controls" / "output" / "validation-checks.csv", 394),
    (ROOT / "analysis" / "controls" / "time-l-output" / "time-l-validation.csv", 246),
]
for path, expected in validation_receipts:
    rows = read_csv(path)
    check(f"receipt_{path.parent.name}_{path.name}", len(rows) == expected and all(row["status"] == "pass" for row in rows), f"rows={len(rows)} failures={sum(row['status']!='pass' for row in rows)}")

json_receipts = [
    (ROOT / "analysis" / "build-provenance" / "validation.json", 55, "build"),
    (ROOT / "analysis" / "cpu-samples" / "validation.json", 108, "cpu"),
    (ROOT / "analysis" / "logical-volume" / "output" / "validation.json", 30, "simple"),
    (ROOT / "analysis" / "controls" / "behavior-strata" / "validation.json", 8, "simple"),
]
for path, expected, shape in json_receipts:
    receipt = json.loads(path.read_text(encoding="utf-8"))
    if shape == "build":
        passed = receipt["overall_passed"] and len(receipt["checks"]) == expected and all(
            row["passed"] for row in receipt["checks"]
        )
        observed = len(receipt["checks"])
    elif shape == "cpu":
        passed = (
            receipt["overall_structural_validation_passed"]
            and receipt["checks_passed"] == expected
            and receipt["checks_total"] == expected
        )
        observed = receipt["checks_total"]
    else:
        passed = receipt["status"] in {"pass", "PASS"} and receipt["checks"] == expected
        observed = receipt["checks"]
    check(f"json_receipt_{path.parent.name}", passed, str(observed))

def finite(value: object) -> bool:
    if isinstance(value, float):
        return math.isfinite(value)
    if isinstance(value, dict):
        return all(finite(v) for v in value.values())
    if isinstance(value, list):
        return all(finite(v) for v in value)
    return True


check("all_numbers_finite", finite(artifact), "recursive finite-number check")

validation = {
    "status": "pass",
    "assessment": "ready_to_validate_with_mcp",
    "checks": len(checks),
    "failed": 0,
    "artifact_sha256": sha256(artifact_path),
    "artifact_bytes": artifact_path.stat().st_size,
    "claim_boundaries": [
        "diagnostic only; no threshold or baseline authority",
        "candidate-text matching is not full semantic parity",
        "prefix sums are reconstructed sensitivity results",
        "whole-process counters retain amortized startup",
        "CPU samples are qualitative only",
        "Windows comparison is source/platform confounded",
    ],
}
(REPORT / "report-validation.json").write_text(json.dumps(validation, indent=2) + "\n", encoding="utf-8")
with (REPORT / "validation-checks.csv").open("w", encoding="utf-8", newline="") as handle:
    writer = csv.DictWriter(handle, fieldnames=["check", "status", "detail"])
    writer.writeheader()
    writer.writerows(checks)

print(json.dumps(validation, indent=2))
