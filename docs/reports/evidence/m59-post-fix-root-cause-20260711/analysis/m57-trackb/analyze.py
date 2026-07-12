#!/usr/bin/env python3
"""Reconcile M57 Track B evidence with the five accepted current rounds.

This script reads evidence only. It writes its audit products beside itself and
never writes into the Yune repository.
"""

from __future__ import annotations

import csv
import hashlib
import json
import statistics
from collections import defaultdict
from pathlib import Path


REPO = Path("/Users/laufei/Documents/GitHub/yune")
M57 = REPO / "docs/reports/evidence/m57-macos-track-a-sentence-model-parity"
CURRENT = Path(
    "/Users/laufei/yune-m59-post-fix-root-cause-20260711/accepted-baseline"
)
OUT = Path(__file__).resolve().parent
TRACK_B_INPUT = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung"

M57_PASSES = [
    ("m57-full-pass-1", M57 / "full-pass-1"),
    ("m57-full-pass-2", M57 / "full-pass-2"),
]
CURRENT_RUNS = [
    (f"current-run-{index}", CURRENT / f"run-{index}") for index in range(1, 6)
]
ALL_EVIDENCE = M57_PASSES + CURRENT_RUNS

PRODUCT_FIELDS = [
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
    "byte_source_len",
    "stored_entries",
    "compact_all_codes_count",
    "compact_expanded_table_entries",
    "compact_expansion_status",
    "table_heap_mirror_bytes",
    "prism_heap_mirror_bytes",
    "rsmarisa_status",
    "rsmarisa_mapping_mode",
]

MEMORY_FIELDS = [
    "module",
    "structure",
    "byte_class",
    "sharing_scope",
    "retained_estimate_bytes",
    "non_overlapping_reducible_bytes",
    "logical_bytes",
    "item_count",
    "mapped_file_bytes",
    "mapping_mode",
    "evidence_source",
    "notes",
]

IDENTITY_FIELDS = {
    "engine",
    "track",
    "schema_id",
    "workload",
    "input",
    "sample_index",
    "operation_count",
}


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8-sig") as handle:
        return list(csv.DictReader(handle))


def write_csv(path: Path, rows: list[dict[str, object]], fields: list[str]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_json(value: object) -> str:
    payload = json.dumps(
        value, ensure_ascii=False, separators=(",", ":"), sort_keys=True
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def percent_change(current: float, reference: float) -> str:
    if reference == 0:
        return "n/a"
    return f"{(current / reference - 1.0) * 100.0:+.1f}"


def parse_environment(path: Path) -> dict[str, str]:
    output: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            output[key] = value
    return output


def track_b_path(root: Path, name: str) -> Path:
    return root / "track-b-yune-product" / name


def candidate_audit() -> list[dict[str, object]]:
    reference_path = track_b_path(M57_PASSES[0][1], "candidate_snapshots.csv")
    reference_rows = read_csv(reference_path)
    reference_hash = sha256_file(reference_path)
    output = []
    for label, root in ALL_EVIDENCE:
        path = track_b_path(root, "candidate_snapshots.csv")
        rows = read_csv(path)
        texts = [row["text"] for row in rows]
        output.append(
            {
                "evidence": label,
                "candidate_rows": len(rows),
                "candidate_count": rows[0]["candidate_count"] if rows else "",
                "page_size": rows[0]["page_size"] if rows else "",
                "page_no": rows[0]["page_no"] if rows else "",
                "is_last_page": rows[0]["is_last_page"] if rows else "",
                "highlighted_index": rows[0]["highlighted_index"] if rows else "",
                "texts": " | ".join(texts),
                "raw_sha256": sha256_file(path),
                "normalized_sha256": sha256_json(rows),
                "byte_identical_to_m57_full_pass_1": (
                    "yes" if sha256_file(path) == reference_hash else "no"
                ),
                "row_identical_to_m57_full_pass_1": (
                    "yes" if rows == reference_rows else "no"
                ),
            }
        )
    return output


def product_status_audit() -> list[dict[str, object]]:
    datasets: dict[str, dict[str, dict[str, str]]] = {}
    for label, root in ALL_EVIDENCE:
        datasets[label] = {
            row["dictionary_id"]: row
            for row in read_csv(track_b_path(root, "product_path_status.csv"))
        }

    output = []
    dictionaries = sorted(
        set().union(*(set(dataset) for dataset in datasets.values()))
    )
    for dictionary in dictionaries:
        for field in PRODUCT_FIELDS:
            values = {
                label: datasets[label].get(dictionary, {}).get(field, "missing")
                for label, _ in ALL_EVIDENCE
            }
            m57_values = [values[label] for label, _ in M57_PASSES]
            current_values = [values[label] for label, _ in CURRENT_RUNS]
            m57_consistent = len(set(m57_values)) == 1
            current_consistent = len(set(current_values)) == 1
            if m57_consistent and current_consistent and m57_values[0] == current_values[0]:
                status = "unchanged"
            elif current_consistent:
                status = "changed_since_m57"
            else:
                status = "current_round_disagreement"
            note = ""
            if field == "byte_source_len" and status == "changed_since_m57":
                note = "compiled byte length changed; checksums/entry counts decide model identity"
            elif field in {"source_checksum", "table_checksum", "checksum_status"}:
                note = "model/checksum identity"
            elif field in {"selected_storage", "table_mapping_mode", "prism_mapping_mode"}:
                note = "storage-owner shape"
            output.append(
                {
                    "dictionary_id": dictionary,
                    "field": field,
                    "m57_full_pass_1": m57_values[0],
                    "m57_full_pass_2": m57_values[1],
                    "m57_passes_consistent": "yes" if m57_consistent else "no",
                    "current_run_1": current_values[0],
                    "current_run_2": current_values[1],
                    "current_run_3": current_values[2],
                    "current_run_4": current_values[3],
                    "current_run_5": current_values[4],
                    "current_runs_consistent": "yes" if current_consistent else "no",
                    "status": status,
                    "note": note,
                }
            )
    return output


def normalized_owner_map(path: Path) -> dict[tuple[str, int], dict[str, str]]:
    counts: defaultdict[str, int] = defaultdict(int)
    output: dict[tuple[str, int], dict[str, str]] = {}
    for row in read_csv(path):
        owner = row["owner_id"]
        counts[owner] += 1
        output[(owner, counts[owner])] = {field: row[field] for field in MEMORY_FIELDS}
    return output


def compact_owner_value(row: dict[str, str] | None) -> str:
    if row is None:
        return "missing"
    return ";".join(f"{field}={row[field]}" for field in MEMORY_FIELDS)


def numeric_range(rows: list[dict[str, str] | None], field: str) -> str:
    values = [int(row[field]) for row in rows if row is not None and row[field].isdigit()]
    if not values:
        return "missing"
    if min(values) == max(values):
        return str(values[0])
    return f"{min(values)}..{max(values)}"


def memory_owner_audit() -> list[dict[str, object]]:
    datasets = {
        label: normalized_owner_map(track_b_path(root, "memory-owner-profile.csv"))
        for label, root in ALL_EVIDENCE
    }
    keys = sorted(set().union(*(set(dataset) for dataset in datasets.values())))
    output = []
    for owner, occurrence in keys:
        m57_rows = [datasets[label].get((owner, occurrence)) for label, _ in M57_PASSES]
        current_rows = [
            datasets[label].get((owner, occurrence)) for label, _ in CURRENT_RUNS
        ]
        process_proxy = owner.startswith("process.")
        m57_values = [compact_owner_value(row) for row in m57_rows]
        current_values = [compact_owner_value(row) for row in current_rows]
        m57_consistent = len(set(m57_values)) == 1
        current_consistent = len(set(current_values)) == 1
        if process_proxy:
            status = "process_proxy_observation"
        elif all(row is None for row in m57_rows) and all(row is not None for row in current_rows):
            status = "added_current"
        elif all(row is not None for row in m57_rows) and all(row is None for row in current_rows):
            status = "removed_current"
        elif m57_consistent and current_consistent and m57_values[0] == current_values[0]:
            status = "unchanged"
        elif current_consistent:
            status = "changed_shape"
        else:
            status = "current_round_disagreement"
        output.append(
            {
                "owner_id": owner,
                "occurrence": occurrence,
                "m57_present_passes": sum(row is not None for row in m57_rows),
                "current_present_runs": sum(row is not None for row in current_rows),
                "m57_passes_consistent": "yes" if m57_consistent else "no",
                "current_runs_consistent": "yes" if current_consistent else "no",
                "m57_retained_bytes": numeric_range(m57_rows, "retained_estimate_bytes"),
                "current_retained_bytes": numeric_range(
                    current_rows, "retained_estimate_bytes"
                ),
                "m57_item_count": numeric_range(m57_rows, "item_count"),
                "current_item_count": numeric_range(current_rows, "item_count"),
                "m57_mapped_file_bytes": numeric_range(m57_rows, "mapped_file_bytes"),
                "current_mapped_file_bytes": numeric_range(
                    current_rows, "mapped_file_bytes"
                ),
                "m57_shape": m57_values[0] if m57_consistent else "varies between passes",
                "current_shape": (
                    current_values[0] if current_consistent else "varies between runs"
                ),
                "status": status,
                "interpretation": (
                    "absolute macOS process memory proxy; not an additive owner"
                    if process_proxy
                    else "normalized owner row; paths and session ids excluded"
                ),
            }
        )
    return output


def counter_sums(path: Path) -> dict[str, int]:
    rows = [
        row
        for row in read_csv(path)
        if row["track"] == "track-b-product"
        and row["workload"] == "key_sequence_process_with_context"
        and row["input"] == TRACK_B_INPUT
    ]
    if len(rows) != 80:
        raise RuntimeError(f"expected 80 Track B M37 rows in {path}, found {len(rows)}")
    fields = [
        field
        for field in rows[0]
        if field not in IDENTITY_FIELDS and not field.endswith("_ns")
    ]
    return {field: sum(int(row[field]) for row in rows) for field in fields}


def counter_audit() -> list[dict[str, object]]:
    datasets = {
        label: counter_sums(track_b_path(root, "m37_metrics.csv"))
        for label, root in ALL_EVIDENCE
    }
    fields = sorted(set().union(*(set(dataset) for dataset in datasets.values())))
    output = []
    for field in fields:
        m57_values = [datasets[label][field] for label, _ in M57_PASSES]
        current_values = [datasets[label][field] for label, _ in CURRENT_RUNS]
        if not any(m57_values) and not any(current_values):
            continue
        output.append(
            {
                "counter": field,
                "m57_full_pass_1": m57_values[0],
                "m57_full_pass_2": m57_values[1],
                "m57_passes_consistent": "yes" if len(set(m57_values)) == 1 else "no",
                "current_run_1": current_values[0],
                "current_run_2": current_values[1],
                "current_run_3": current_values[2],
                "current_run_4": current_values[3],
                "current_run_5": current_values[4],
                "current_runs_consistent": (
                    "yes" if len(set(current_values)) == 1 else "no"
                ),
                "current_value": (
                    current_values[0]
                    if len(set(current_values)) == 1
                    else "varies"
                ),
                "change_vs_m57_full_pass_1_pct": percent_change(
                    current_values[0], m57_values[0]
                ),
                "status": "unchanged" if m57_values[0] == current_values[0] else "changed",
                "interpretation": "non-timing internal work shape; not behavior",
            }
        )
    return output


def latency_observations() -> list[dict[str, object]]:
    output = []
    for label, root in ALL_EVIDENCE:
        for row in read_csv(track_b_path(root, "summary.csv")):
            output.append(
                {
                    "evidence": label,
                    "generation": "M57" if label.startswith("m57") else "current",
                    "workload": row["workload"],
                    "input": row["input"],
                    "samples": row["samples"],
                    "operations": row["operations"],
                    "median_us": row["median_us"],
                    "p95_us": row["p95_us"],
                    "p99_us": row["p99_us"],
                    "max_us": row["max_us"],
                    "median_working_set_bytes": row["median_working_set_bytes"],
                    "max_peak_working_set_bytes": row["max_peak_working_set_bytes"],
                    "median_private_bytes": row["median_private_bytes"],
                    "max_peak_pagefile_bytes": row["max_peak_pagefile_bytes"],
                }
            )
    return output


def latency_summary(observations: list[dict[str, object]]) -> list[dict[str, object]]:
    output = []
    workloads = sorted({str(row["workload"]) for row in observations})
    for workload in workloads:
        m57 = [
            row
            for row in observations
            if row["generation"] == "M57" and row["workload"] == workload
        ]
        current = [
            row
            for row in observations
            if row["generation"] == "current" and row["workload"] == workload
        ]
        m57_medians = [float(row["median_us"]) for row in m57]
        current_medians = [float(row["median_us"]) for row in current]
        current_median = statistics.median(current_medians)
        m57_center = statistics.median(m57_medians)
        output.append(
            {
                "workload": workload,
                "m57_full_pass_1_median_us": f"{m57_medians[0]:.3f}",
                "m57_full_pass_2_median_us": f"{m57_medians[1]:.3f}",
                "m57_two_pass_center_us": f"{m57_center:.3f}",
                "current_run_1_median_us": f"{current_medians[0]:.3f}",
                "current_run_2_median_us": f"{current_medians[1]:.3f}",
                "current_run_3_median_us": f"{current_medians[2]:.3f}",
                "current_run_4_median_us": f"{current_medians[3]:.3f}",
                "current_run_5_median_us": f"{current_medians[4]:.3f}",
                "current_median_of_run_medians_us": f"{current_median:.3f}",
                "current_worst_run_median_us": f"{max(current_medians):.3f}",
                "current_pooled_worst_sample_us": f"{max(float(row['max_us']) for row in current):.3f}",
                "current_median_spread_pct": f"{(max(current_medians) - min(current_medians)) / current_median * 100.0:.1f}",
                "current_vs_m57_full_pass_1_pct": percent_change(
                    current_median, m57_medians[0]
                ),
                "current_vs_m57_two_pass_center_pct": percent_change(
                    current_median, m57_center
                ),
                "interpretation": "same-host historical observation; source commits differ",
            }
        )
    return output


def manifest() -> list[dict[str, object]]:
    output = []
    for label, root in ALL_EVIDENCE:
        for name in [
            "candidate_snapshots.csv",
            "product_path_status.csv",
            "memory-owner-profile.csv",
            "m37_metrics.csv",
            "summary.csv",
            "metadata.txt",
        ]:
            path = track_b_path(root, name)
            output.append(
                {
                    "evidence": label,
                    "file": name,
                    "path": str(path),
                    "bytes": path.stat().st_size,
                    "sha256": sha256_file(path),
                }
            )
        environment_path = root / "environment.txt"
        environment = parse_environment(environment_path)
        output.append(
            {
                "evidence": label,
                "file": "environment.txt",
                "path": str(environment_path),
                "bytes": environment_path.stat().st_size,
                "sha256": sha256_file(environment_path),
            }
        )
        if label.startswith("m57"):
            expected_head = "c6749cc659a8b5b693e4bf5ed631da67933bf86d"
            if environment.get("yune_git_head") != expected_head:
                raise RuntimeError(f"unexpected M57 source head in {environment_path}")
        else:
            expected_head = "afb7079b71f7f9353845114ff3e310c0a38b9b87"
            if environment.get("yune_git_head") != expected_head:
                raise RuntimeError(f"unexpected current source head in {environment_path}")
    return output


def selected_counter(rows: list[dict[str, object]], name: str) -> dict[str, object]:
    return next(row for row in rows if row["counter"] == name)


def selected_latency(rows: list[dict[str, object]], workload: str) -> dict[str, object]:
    return next(row for row in rows if row["workload"] == workload)


def build_readme(
    candidates: list[dict[str, object]],
    products: list[dict[str, object]],
    owners: list[dict[str, object]],
    counters: list[dict[str, object]],
    latency: list[dict[str, object]],
) -> str:
    key_latency = selected_latency(latency, "key_sequence_process_with_context")
    session_latency = selected_latency(latency, "session_create_select_destroy")
    startup_latency = selected_latency(latency, "startup_warm_shared_assets_runtime_ready")
    changed_counters = [row for row in counters if row["status"] == "changed"]
    added_owners = [row for row in owners if row["status"] == "added_current"]
    changed_owners = [row for row in owners if row["status"] == "changed_shape"]
    current_candidate_hashes = {
        str(row["raw_sha256"])
        for row in candidates
        if str(row["evidence"]).startswith("current")
    }
    m57_candidate_hashes = {
        str(row["raw_sha256"])
        for row in candidates
        if str(row["evidence"]).startswith("m57")
    }
    product_changes = [row for row in products if row["status"] != "unchanged"]

    highlight_names = [
        "track_b_spelling_expansions_considered",
        "track_b_exact_lookup_calls",
        "track_b_prefix_lookup_calls",
        "track_b_candidates_materialized",
        "bounded_iterator_selected_total",
        "bounded_iterator_full_count_total",
        "owned_candidates_materialized",
        "candidates_sorted",
    ]
    counter_lines = []
    for name in highlight_names:
        row = selected_counter(counters, name)
        counter_lines.append(
            f"| `{name}` | {row['m57_full_pass_1']} | {row['current_value']} | "
            f"{row['change_vs_m57_full_pass_1_pct']}% |"
        )

    return f"""# M57-to-current Track B audit

## Answer

Track B behavior is unchanged: both M57 complete passes and all five accepted
current rounds produced the exact same five candidate rows, including comments
and page state. The raw candidate CSV SHA-256 is `{next(iter(current_candidate_hashes))}`
for all seven observations.

The underlying work and memory shape did change. All five current rounds agree
exactly on every non-timing M37 counter, so this is deterministic work-shape
movement rather than round noise. Product checksums, compiled readiness, mmap
storage, entry counts, and no-source-fallback status remain stable. The two
compiled Track B table payloads are each 28 bytes larger; their logical
checksums and entry counts are unchanged.

## Scope and grain

- Input: `{TRACK_B_INPUT}`.
- M57 reference: `full-pass-1` is the prior report's named anchor;
  `full-pass-2` is retained as a second historical observation.
- Current evidence: `accepted-baseline/run-1` through `run-5`, Yune
  `afb7079b71f7f9353845114ff3e310c0a38b9b87`.
- Schema: `jyut6ping3_mobile`; Track B is a Yune product guard, not a
  Yune-versus-librime peer benchmark.
- Each latency row contains 80 key-sequence samples, 60 session samples, and
  9 startup samples. Each key-sequence sample processes 61 keys.

## Candidate and product status

- Candidate rows: byte-identical across both M57 passes and all five current
  runs (`{len(m57_candidate_hashes)} M57 hash`, `{len(current_candidate_hashes)} current hash`).
- First page: `你個人經其實應該支援超場句子輸入先可以用`, `你個`, `你`, `呢`, `尼`.
- Both dictionaries retain the same source/table checksums:
  `jyut6ping3` = `0xf6589c0c`; `jyut6ping3_scolar` = `0x822bccba`.
- Both remain `fresh`, compiled-ready, byte-backed, mmap/mmap, with no source
  fallback and zero table/prism heap mirrors.
- `jyut6ping3` remains 127,143 stored/expanded entries and 114,653 codes;
  `jyut6ping3_scolar` remains an empty table owner in this profile.
- Product-field changes: {len(product_changes)} rows, all accounted for by the
  two 28-byte `byte_source_len` changes.

## Non-timing counter movement

| Counter | M57 pass 1 | Current (all five) | Change |
| --- | ---: | ---: | ---: |
{chr(10).join(counter_lines)}

There are {len(changed_counters)} changed non-zero counter rows in the complete
CSV. Two scopes move in opposite directions: Track-B-specific materialization
and bounded selection increase, while global owned candidates and sorting fall.
They are not additive definitions and must not be used as a single work total.
Candidate behavior remains exact despite the changed internal path.

## Normalized memory-owner shape

- {len(added_owners)} owner occurrences are newly reported: six guarded
  translator owners for each of the two translator dictionaries. Their retained
  estimates are zero or 48 bytes; they are structural/accounting additions, not
  a large retained-memory regression.
- {len(changed_owners)} non-process owner occurrences changed shape. Product
  status records both compiled tables at +28 bytes, while the owner ledger
  reflects +28 bytes for the main compact-table mapping. The two prism mapping
  pairs grew by 13,327,264 bytes in total, and the schema reload signature grew
  by 679 bytes.
- `poet.entries_by_code`, `poet.lookup_index`, and
  `poet.abbreviation_vocabulary` remain absent/shared-zero for both translators.
- Current peak resident proxy spans 444,940,288 to 468,123,648 bytes, versus
  741,736,448 to 752,746,496 bytes in the two M57 passes. This is same-platform
  observational evidence, not an additive owner total or a causal attribution.

## Latency observations

| Workload | M57 pass 1 | M57 pass 2 | Current five-run median | Current spread | Current vs M57 center |
| --- | ---: | ---: | ---: | ---: | ---: |
| Key sequence (µs/key) | {key_latency['m57_full_pass_1_median_us']} | {key_latency['m57_full_pass_2_median_us']} | {key_latency['current_median_of_run_medians_us']} | {key_latency['current_median_spread_pct']}% | {key_latency['current_vs_m57_two_pass_center_pct']}% |
| Session lifecycle (µs) | {session_latency['m57_full_pass_1_median_us']} | {session_latency['m57_full_pass_2_median_us']} | {session_latency['current_median_of_run_medians_us']} | {session_latency['current_median_spread_pct']}% | {session_latency['current_vs_m57_two_pass_center_pct']}% |
| Startup (µs) | {startup_latency['m57_full_pass_1_median_us']} | {startup_latency['m57_full_pass_2_median_us']} | {startup_latency['current_median_of_run_medians_us']} | {startup_latency['current_median_spread_pct']}% | {startup_latency['current_vs_m57_two_pass_center_pct']}% |

The current key-sequence median is stable and roughly 8% below the two-pass M57
center. Runs 4 and 5 nevertheless contain high-tail samples (pooled worst
`{key_latency['current_pooled_worst_sample_us']} µs/key`), so the median improvement
does not justify a tail-latency claim. Startup is especially unsuitable for a
source-change conclusion because the two M57 medians themselves differ sharply.

## Evidence quality and caveats

1. The current five-run source and Yune binary are clean and fixed; the Yune
   dylib hash is `3dd5a414c68f7884884c5dc172b3f0b088d1f5ae19cb983eb0eeb2f95bc6c710`.
2. M57 recorded `c6749cc6` plus dirty modifications in the exact files later
   committed by M57 closeout `a87c6b88`. It is accepted historical evidence,
   but it is weaker clean-commit provenance than the current packet and does not
   record an equivalent five-run dylib hash.
3. Source commits differ, so counter, owner, and latency movements are
   descriptive. They cannot be attributed solely to the Luna page-order repair.
4. Absolute macOS resident memory includes allocator, mappings, loader state,
   and overlap. It is not interchangeable with Windows memory counters.
5. Source/table checksums do not identify the prism artifact. The stable table
   checksums therefore do not erase the separately observed prism-shape change.
6. Comments are compared byte-for-byte through the candidate CSV hash. The
   human-readable table abbreviates them only for readability.

## Files

- `track-b-candidate-audit.csv`: seven exact candidate observations.
- `track-b-product-status-diff.csv`: complete normalized product/checksum fields.
- `track-b-memory-owner-diff.csv`: owner rows normalized without paths/session IDs.
- `track-b-counter-diff.csv`: all non-zero non-timing M37 counters.
- `track-b-latency-observations.csv`: all raw summary observations.
- `track-b-latency-summary.csv`: two-pass M57 and five-run current aggregates.
- `source-manifest.csv`: exact source paths, sizes, and hashes.
- `analyze.py`: reproducible read-only transformation.
"""


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    candidates = candidate_audit()
    products = product_status_audit()
    owners = memory_owner_audit()
    counters = counter_audit()
    observations = latency_observations()
    summary = latency_summary(observations)
    sources = manifest()

    write_csv(
        OUT / "track-b-candidate-audit.csv",
        candidates,
        list(candidates[0]),
    )
    write_csv(
        OUT / "track-b-product-status-diff.csv",
        products,
        list(products[0]),
    )
    write_csv(
        OUT / "track-b-memory-owner-diff.csv",
        owners,
        list(owners[0]),
    )
    write_csv(
        OUT / "track-b-counter-diff.csv",
        counters,
        list(counters[0]),
    )
    write_csv(
        OUT / "track-b-latency-observations.csv",
        observations,
        list(observations[0]),
    )
    write_csv(
        OUT / "track-b-latency-summary.csv",
        summary,
        list(summary[0]),
    )
    write_csv(OUT / "source-manifest.csv", sources, list(sources[0]))
    (OUT / "README.md").write_text(
        build_readme(candidates, products, owners, counters, summary),
        encoding="utf-8",
    )

    invariants = {
        "candidate_observations": len(candidates),
        "all_candidates_match_m57": all(
            row["row_identical_to_m57_full_pass_1"] == "yes" for row in candidates
        ),
        "m57_product_passes_consistent": all(
            row["m57_passes_consistent"] == "yes" for row in products
        ),
        "current_product_rounds_consistent": all(
            row["current_runs_consistent"] == "yes" for row in products
        ),
        "m57_non_timing_counters_identical": all(
            row["m57_passes_consistent"] == "yes" for row in counters
        ),
        "current_non_timing_counters_identical": all(
            row["current_runs_consistent"] == "yes" for row in counters
        ),
        "m57_non_process_owner_rows_consistent": all(
            row["m57_passes_consistent"] == "yes"
            for row in owners
            if row["status"] != "process_proxy_observation"
        ),
        "current_non_process_owner_rows_consistent": all(
            row["current_runs_consistent"] == "yes"
            for row in owners
            if row["status"] != "process_proxy_observation"
        ),
    }
    (OUT / "validation.json").write_text(
        json.dumps(invariants, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    if not all(invariants.values()):
        raise RuntimeError(f"audit invariant failed: {invariants}")
    print(json.dumps(invariants, sort_keys=True))


if __name__ == "__main__":
    main()
