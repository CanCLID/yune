#!/usr/bin/env python3
"""Build the portable technical report for the source-current macOS packet."""

from __future__ import annotations

import csv
import json
import sqlite3
from collections import Counter
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
AGG = ROOT / "aggregate"
OUT = ROOT / "report"
TITLE = "Source-current M59 macOS Yune vs librime verification"
GENERATED_AT = "2026-07-15T02:57:03Z"
LONG_37 = "ceshiyixiachangjushuruxingnengzenyang"
LONG_59 = "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong"
WINDOWS_ZHONGDENG_EVIDENCE = "aggregate/windows-zhongdengchangdu-evidence.csv"


def read_csv(name: str) -> list[dict[str, str]]:
    with (AGG / name).open(encoding="utf-8-sig", newline="") as handle:
        return list(csv.DictReader(handle))


def num(value: str) -> int | float:
    parsed = float(value)
    return int(parsed) if parsed.is_integer() else parsed


def truth(value: str) -> str:
    return "yes" if value.lower() == "true" else "no"


def ratio_text(value: int | float) -> str:
    return f"{float(value):.3f}x"


def us_text(value: int | float) -> str:
    return f"{float(value):.3f} µs/key"


def bytes_text(value: int | float) -> str:
    return f"{int(value):,} B"


def percent_text(value: int | float) -> str:
    return f"{float(value):.1f}%"


def table(
    table_id: str,
    title: str,
    subtitle: str,
    dataset: str,
    source_id: str,
    columns: list[tuple[str, str, str, str | None]],
    sort_field: str,
) -> dict[str, object]:
    rendered_columns = []
    for field, label, kind, unit in columns:
        column: dict[str, object] = {"field": field, "label": label, "type": kind}
        if unit:
            column["unit"] = unit
        rendered_columns.append(column)
    return {
        "id": table_id,
        "title": title,
        "subtitle": subtitle,
        "dataset": dataset,
        "sourceId": source_id,
        "defaultSort": {"field": sort_field, "direction": "asc"},
        "density": "compact",
        "layout": "full",
        "columns": rendered_columns,
    }


def source(source_id: str, label: str, path: str, description: str) -> dict[str, object]:
    table_name = {
        "headline_source": "headline",
        "track_a_source": "track_a",
        "track_b_source": "track_b",
        "candidate_source": "candidates",
        "model_source": "model_summary",
        "audit_source": "run_audit",
    }[source_id]
    source_files = [path]
    if source_id == "candidate_source":
        source_files.append(WINDOWS_ZHONGDENG_EVIDENCE)
    return {
        "id": source_id,
        "label": label,
        "path": "report/snapshot.sqlite",
        "query": {
            "engine": "sqlite",
            "language": "sql",
            "sql": f'SELECT * FROM "{table_name}"',
            "description": description,
            "tables_used": [table_name],
            "source_files": source_files,
            "filters": ["Five complete measured rounds; no measured round removed."],
            "metric_definitions": [
                "A Yune/librime ratio below 1 means Yune is faster on the same Mac.",
                "Close means an absolute Mac-vs-Windows ratio difference within 10%; notable is over 10% through 25%; material is over 25%.",
            ],
            "executed_at": "2026-07-14T22:06:16Z",
        },
    }


def write_sqlite(path: Path, datasets: dict[str, list[dict[str, object]]]) -> None:
    if path.exists():
        path.unlink()
    connection = sqlite3.connect(path)
    try:
        for table_name, rows in datasets.items():
            if not rows:
                continue
            columns = list(rows[0])
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
            connection.execute(f'CREATE TABLE "{table_name}" ({", ".join(definitions)})')
            placeholders = ",".join("?" for _ in columns)
            connection.executemany(
                f'INSERT INTO "{table_name}" VALUES ({placeholders})',
                [[row[column] for column in columns] for row in rows],
            )
        connection.commit()
    finally:
        connection.close()


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    generated_at = GENERATED_AT

    raw_a = read_csv("track-a-17-row-comparison.csv")
    track_a = []
    for order, row in enumerate(raw_a, 1):
        label = "37-character" if row["input"] == LONG_37 else "59-character" if row["input"] == LONG_59 else row["input"]
        run_ratios = {f"run{i}_ratio": num(row[f"mac_run{i}_ratio"]) for i in range(1, 6)}
        mac_median_ratio = num(row["mac_median_ratio"])
        mac_pooled_worst_ratio = num(row["mac_pooled_worst_ratio"])
        spread_pct = round(num(row["mac_spread_pct"]), 1)
        final_windows_443_ratio = num(row["final_m59_windows_443_median_ratio"])
        mac_vs_windows_443_pct = round(num(row["mac_vs_final_m59_windows_443_pct"]), 1)
        signed_i0_ratio = num(row["signed_i0_windows_median_ratio"])
        signed_ceiling = num(row["signed_windows_ceiling"])
        track_a.append({
            "row_order": order,
            "input_label": label,
            "input": row["input"],
            "new_m59_row": truth(row["newly_signed"]),
            **run_ratios,
            **{f"run{i}_ratio_display": ratio_text(run_ratios[f"run{i}_ratio"]) for i in range(1, 6)},
            "mac_median_ratio": mac_median_ratio,
            "mac_median_ratio_display": ratio_text(mac_median_ratio),
            "mac_pooled_worst_ratio": mac_pooled_worst_ratio,
            "mac_pooled_worst_ratio_display": ratio_text(mac_pooled_worst_ratio),
            "spread_pct": spread_pct,
            "spread_pct_display": percent_text(spread_pct),
            "final_windows_443_ratio": final_windows_443_ratio,
            "final_windows_443_ratio_display": ratio_text(final_windows_443_ratio),
            "mac_vs_windows_443_pct": mac_vs_windows_443_pct,
            "mac_vs_windows_443_pct_display": percent_text(mac_vs_windows_443_pct),
            "class": row["mac_vs_final_m59_windows_443_class"],
            "signed_i0_ratio": signed_i0_ratio,
            "signed_i0_ratio_display": ratio_text(signed_i0_ratio),
            "signed_ceiling": signed_ceiling,
            "signed_ceiling_display": ratio_text(signed_ceiling),
            "ceiling_diagnostic": row["signed_ceiling_diagnostic"],
            "chart_label": f"{label} — {ratio_text(mac_median_ratio)}",
        })

    classes = Counter(row["class"] for row in track_a)
    by_input = {row["input"]: row for row in track_a}
    raw_components = read_csv("track-a-component-macos.csv")
    component_map = {(row["input"], row["engine"]): row for row in raw_components}

    raw_b = read_csv("track-b-five-observations.csv")
    track_b = []
    for row in raw_b:
        values = {
            "mac_median_us": num(row["mac_median_us"]),
            "mac_p95_us": num(row["mac_p95_us"]),
            "mac_p99_us": num(row["mac_p99_us"]),
            "mac_max_us": num(row["mac_max_us"]),
            "mac_median_rss_bytes": int(row["mac_median_working_set_bytes"]),
            "mac_peak_rss_bytes": int(row["mac_max_peak_working_set_bytes"]),
            "final_windows_443_median_us": num(row["final_m59_windows_443_run_median_us"]),
            "signed_i0_median_us": num(row["signed_i0_windows_run_median_us"]),
        }
        track_b.append({
            "run": int(row["run"]),
            **values,
            **{f"{field}_display": us_text(value) for field, value in values.items() if field.endswith("_us")},
            "mac_median_rss_bytes_display": bytes_text(values["mac_median_rss_bytes"]),
            "mac_peak_rss_bytes_display": bytes_text(values["mac_peak_rss_bytes"]),
        })

    raw_candidates = read_csv("current-yune-librime-candidate-comparison.csv")
    candidates = []
    for order, row in enumerate(raw_candidates, 1):
        label = "37-character" if row["input"] == LONG_37 else "59-character" if row["input"] == LONG_59 else row["input"]
        candidates.append({
            "row_order": order,
            "input_label": label,
            "input": row["input"],
            "yune_page": row["yune_texts"],
            "librime_page": row["librime_texts"],
            "text_order_match": truth(row["text_order_match"]),
            "geometry_match": truth(row["page_geometry_match"]),
            "comments_match": truth(row["comments_match"]),
            "classification": "cross-platform Yune engine-path discrepancy" if row["input"] == "zhongdengchangdu" else "exact",
        })

    audit = []
    for row in read_csv("artifact-hash-audit.csv"):
        audit.append({
            "run": int(row["run"]),
            "path": f"accepted/run-{row['run']}",
            "start_utc": row["start_utc"],
            "end_utc": row["end_utc"],
            "source_clean": "yes" if row["yune_source_clean"] == "True" and row["librime_seed_clean"] == "True" else "no",
            "ac_start_end": "yes" if row["ac_power_start_end"] == "True" else "no",
            "yune_hash": row["yune_dylib_sha256"],
            "librime_hash": row["librime_dylib_sha256"],
            "artifacts_complete": "yes" if row["required_artifacts_present"] == "True" else "no",
            "verdict": row["verdict"],
        })

    owners = read_csv("memory-owner-m57-comparison.csv")
    products = read_csv("product-status-m57-comparison.csv")
    candidate_m57 = read_csv("candidate-m57-comparison.csv")
    owner_exact = sum(row["normalized_shape_match"] == "True" for row in owners)
    owner_current_only = sum(row["present_current"] == "True" and row["present_m57"] == "False" for row in owners)
    owner_m57_only = sum(row["present_current"] == "False" and row["present_m57"] == "True" for row in owners)
    product_exact = sum(row["normalized_shape_match"] == "True" for row in products)
    m57_candidate_exact = sum(
        row["text_order_match"] == "True" and row["page_geometry_match"] == "True" and row["comments_match"] == "True"
        for row in candidate_m57
    )
    model_summary = [
        {"measure": "Current Track A candidate pages exact vs librime", "result": "16 / 17", "interpretation": "37/59 exact; zhongdengchangdu is the one deterministic cross-platform Yune mismatch."},
        {"measure": "Current candidate pages exact vs M57", "result": f"{m57_candidate_exact} / {len(candidate_m57)}", "interpretation": "Differences reflect expected source evolution; Track B remains exact."},
        {"measure": "Normalized owner shapes exact vs M57", "result": f"{owner_exact} / {len(owners)}", "interpretation": f"{owner_current_only} current-only and {owner_m57_only} M57-only; new bounded caches/provenance owners remain byte-backed."},
        {"measure": "Product/checksum rows exact vs M57", "result": f"{product_exact} / {len(products)}", "interpretation": "Luna exact; Track B differs only by byte_source_len while checksums/readiness/storage/mmap modes remain exact."},
        {"measure": "Candidate/owner/product shape across current rounds", "result": "5 / 5 identical", "interpretation": "Deterministic logical shape across every retained round."},
    ]

    ratio_37 = by_input[LONG_37]
    ratio_59 = by_input[LONG_59]
    y37 = component_map[(LONG_37, "yune")]
    l37 = component_map[(LONG_37, "librime-1.17.0")]
    y59 = component_map[(LONG_59, "yune")]
    l59 = component_map[(LONG_59, "librime-1.17.0")]
    b_medians = [float(row["mac_median_us"]) for row in track_b]
    ratios_37_text = ", ".join(f"{ratio_37[f'run{i}_ratio']:.3f}" for i in range(1, 6))
    ratios_59_text = ", ".join(f"{ratio_59[f'run{i}_ratio']:.3f}" for i in range(1, 6))

    headline = [{
        "track_a_yune_faster_rows": sum(row["mac_median_ratio"] < 1 for row in track_a),
        "track_a_rows": 17,
        "pooled_worst_below_one_rows": sum(row["mac_pooled_worst_ratio"] < 1 for row in track_a),
        "stable_binary_rounds": 5,
        "close_rows": classes["close"],
        "notable_rows": classes["notable"],
        "material_rows": classes["material"],
        "ratio_37": ratio_37["mac_median_ratio"],
        "ratio_59": ratio_59["mac_median_ratio"],
        "ratio_37_display": ratio_text(ratio_37["mac_median_ratio"]),
        "ratio_59_display": ratio_text(ratio_59["mac_median_ratio"]),
        "track_b_median_us": sorted(b_medians)[2],
        "track_b_median_display": us_text(sorted(b_medians)[2]),
        "candidate_exact_rows": 16,
    }]

    track_a_first = track_a[:9]
    track_a_second = track_a[9:]
    candidates_first = candidates[:9]
    candidates_second = candidates[9:]
    parity_anchor = {"chart_label": "1.000x PARITY anchor", "mac_median_ratio": 1.0}
    track_a_chart_first = [
        {"chart_label": row["chart_label"], "mac_median_ratio": row["mac_median_ratio"]}
        for row in track_a_first
    ] + [parity_anchor]
    track_a_chart_second = [
        {"chart_label": row["chart_label"], "mac_median_ratio": row["mac_median_ratio"]}
        for row in track_a_second
    ] + [parity_anchor]

    sources = [
        source("headline_source", "Validated headline aggregation", "aggregate/validation-checks.csv", "Validated five-round headline metrics."),
        source("track_a_source", "Complete 17-row ratio comparison", "aggregate/track-a-17-row-comparison.csv", "Five retained macOS ratios joined to final-Windows and signed Increment-0 diagnostics."),
        source("track_b_source", "Track B five observations", "aggregate/track-b-five-observations.csv", "Five retained product-input observations and platform-specific diagnostics."),
        source("candidate_source", "Current Yune/librime candidate comparison", "aggregate/current-yune-librime-candidate-comparison.csv", "Complete-input page-zero candidate text, order, geometry, and comment comparison."),
        source("model_source", "M57 candidate and owner comparisons", "aggregate/memory-owner-m57-comparison.csv", "Normalized candidate, logical-owner, and product/checksum comparison to M57."),
        source("audit_source", "Five-run artifact and identity audit", "aggregate/artifact-hash-audit.csv", "Commit, source cleanliness, power, required artifact, and binary identity audit."),
    ]

    cards = [
        {"id": "track_a_card", "description": "Same-Mac current-source comparison; lower than 1 favors Yune.", "dataset": "headline", "sourceId": "headline_source", "metrics": [
            {"label": "Yune-faster Track A rows", "field": "track_a_yune_faster_rows", "format": "number"},
            {"label": "Total Track A rows", "field": "track_a_rows", "format": "number"},
        ]},
        {"id": "worst_card", "description": "Every retained row stays in the same direction even at its pooled worst.", "dataset": "headline", "sourceId": "headline_source", "metrics": [
            {"label": "Pooled-worst ratios below 1", "field": "pooled_worst_below_one_rows", "format": "number"},
            {"label": "Fixed-binary rounds", "field": "stable_binary_rounds", "format": "number"},
        ]},
        {"id": "long_card", "description": "Current-source long-input median ratios on the same Mac.", "dataset": "headline", "sourceId": "track_a_source", "metrics": [
            {"label": "37-character ratio", "field": "ratio_37_display", "format": "number"},
            {"label": "59-character ratio", "field": "ratio_59_display", "format": "number"},
        ]},
        {"id": "track_b_card", "description": "Absolute product-input value; not a portable Windows threshold.", "dataset": "headline", "sourceId": "track_b_source", "metrics": [
            {"label": "Track B median", "field": "track_b_median_display", "format": "number"},
        ]},
        {"id": "candidate_card", "description": "Complete-input Track A page-zero exactness versus same-run librime.", "dataset": "headline", "sourceId": "candidate_source", "metrics": [
            {"label": "Exact candidate pages", "field": "candidate_exact_rows", "format": "number"},
            {"label": "Total pages", "field": "track_a_rows", "format": "number"},
        ]},
    ]

    charts = []
    for part, dataset, measured_rows in [
        ("1 of 2", "track_a_chart_first", 9),
        ("2 of 2", "track_a_chart_second", 8),
    ]:
        chart_id = "median_ratio_chart_first" if part == "1 of 2" else "median_ratio_chart_second"
        charts.append({
            "id": chart_id,
            "title": f"Current-source macOS Track A ratios — part {part}",
            "subtitle": f"{measured_rows} measured inputs plus a non-observation 1.000x parity anchor; each label prints the exact five-round median",
            "intent": "comparison",
            "question": "Does the current-source same-Mac Yune/librime direction hold across every Track A input?",
            "rationale": "Two compact horizontal panels preserve readable labels at narrow widths while keeping the 1.000x comparison boundary in the plotted domain.",
            "comparisonContext": {
                "baseline": "1.000x means equal median latency",
                "denominator": "same-round pinned librime median latency",
                "grain": f"{measured_rows} of 17 Track A inputs; median of five reported ratios",
                "unit": "Yune/librime ratio",
            },
            "type": "horizontalBar",
            "dataset": dataset,
            "sourceId": "track_a_source",
            "encodings": {
                "x": {"field": "chart_label", "type": "nominal", "label": "Input and exact median ratio"},
                "y": {"field": "mac_median_ratio", "type": "quantitative", "label": "Median Yune/librime ratio"},
            },
            "palette": {"kind": "sequential"},
            "labels": {"values": "none"},
            "referenceLines": [{"axis": "y", "value": 1.0, "color": "neutral", "label": "1.000x parity"}],
            "settings": {"orientation": "horizontal", "sort": "none", "categoryLabelPolicy": "wrap"},
            "valueFormat": "number",
            "layout": "full",
            "maxRows": measured_rows + 1,
        })

    track_a_columns = [
        ("row_order", "#", "number", None), ("input_label", "Input", "text", None), ("input", "Exact input", "text", None), ("new_m59_row", "New M59 row", "text", None),
        *[(f"run{i}_ratio_display", f"Run {i}", "text", None) for i in range(1, 6)],
        ("mac_median_ratio_display", "Mac median", "text", None), ("mac_pooled_worst_ratio_display", "Mac worst", "text", None), ("spread_pct_display", "Spread", "text", None),
        ("final_windows_443_ratio_display", "Final Win 443", "text", None), ("mac_vs_windows_443_pct_display", "Mac vs Win 443", "text", None), ("class", "Class", "text", None),
        ("signed_i0_ratio_display", "Signed I0", "text", None), ("signed_ceiling_display", "Signed ceiling", "text", None), ("ceiling_diagnostic", "Ceiling diagnostic", "text", None),
    ]
    candidate_columns = [
        ("row_order", "#", "number", None), ("input_label", "Input", "text", None), ("yune_page", "Yune page", "text", None), ("librime_page", "librime page", "text", None),
        ("text_order_match", "Text/order", "text", None), ("geometry_match", "Geometry", "text", None), ("comments_match", "Comments", "text", None), ("classification", "Classification", "text", None),
    ]
    tables = [
        table("track_a_table_first", "Complete comparison — rows 1–9 of 17", "Exact three-decimal ratios; Windows values and signed ceilings are diagnostic, source-mismatched references", "track_a_first", "track_a_source", track_a_columns, "row_order"),
        table("track_a_table_second", "Complete comparison — rows 10–17 of 17", "Exact three-decimal ratios; no Track A row is hidden", "track_a_second", "track_a_source", track_a_columns, "row_order"),
        table("track_b_table", "Track B product input", "All five Mac observations; absolute cross-platform latency and memory are platform-specific", "track_b", "track_b_source", [
            ("run", "Run", "number", None), ("mac_median_us_display", "Mac median", "text", None), ("mac_p95_us_display", "Mac p95", "text", None), ("mac_p99_us_display", "Mac p99", "text", None), ("mac_max_us_display", "Mac max", "text", None),
            ("final_windows_443_median_us_display", "Final Win 443", "text", None), ("signed_i0_median_us_display", "Signed I0", "text", None), ("mac_median_rss_bytes_display", "Mac median RSS", "text", None), ("mac_peak_rss_bytes_display", "Mac peak RSS", "text", None),
        ], "run"),
        table("candidate_table_first", "Candidate-page comparison — rows 1–9 of 17", "Complete-input page zero; 37/59 are exact", "candidates_first", "candidate_source", candidate_columns, "row_order"),
        table("candidate_table_second", "Candidate-page comparison — rows 10–17 of 17", "No candidate row is hidden; zhongdengchangdu is the one deterministic cross-platform engine-path mismatch", "candidates_second", "candidate_source", candidate_columns, "row_order"),
        table("model_table", "Candidate and model-owner comparison to M57", "Normalized logical shape only; nonportable process counters and paths excluded", "model_summary", "model_source", [
            ("measure", "Measure", "text", None), ("result", "Result", "text", None), ("interpretation", "Interpretation", "text", None),
        ], "measure"),
        table("audit_table", "Five-run provenance and fixed-binary audit", "All complete logical rounds are retained and all required artifacts are present", "run_audit", "audit_source", [
            ("run", "Run", "number", None), ("path", "External path", "text", None), ("start_utc", "Start UTC", "text", None), ("end_utc", "End UTC", "text", None),
            ("source_clean", "Sources clean", "text", None), ("ac_start_end", "AC start/end", "text", None), ("yune_hash", "Yune SHA-256", "text", None), ("librime_hash", "librime SHA-256", "text", None), ("artifacts_complete", "Artifacts complete", "text", None), ("verdict", "Verdict", "text", None),
        ], "run"),
    ]

    blocks = [
        {"id": "title", "type": "markdown", "body": f"# {TITLE}"},
        {"id": "technical_summary", "type": "markdown", "body": "## Technical Summary\n\nThe source-current macOS result is directionally strong: Yune is faster than pinned librime for all 17 Track A median ratios, and every retained pooled-worst ratio also stays below 1.0×. The 37-character median is 0.019× and the 59-character median is 0.008×; Track B's five-run median is 5.607 µs/key. Both measured dylibs stayed byte-identical across all five rounds.\n\nThis packet does **not** isolate a pure platform effect. Mac measures Yune 0111cf47, while final-M59 Windows is 443cc636 and the signed Increment-0 baseline is 45775182. Substantial UI activity at round boundaries is also a material confounder. The evidence supports the same-Mac Yune/librime direction, but source, platform/toolchain, and workload-noise shares cannot be separated.\n\nBehavior is exact versus same-run librime for 16/17 complete-input Track A pages, including both 37/59 pages. The zhongdengchangdu suffix mismatch is deterministic and is also present in earlier Windows M59 evidence, so it is a cross-platform Yune engine-path discrepancy—not macOS noise or a macOS-only defect. No threshold was changed or treated as portable."},
        {"id": "headline_metrics", "type": "metric-strip", "cardIds": ["track_a_card", "worst_card", "long_card", "track_b_card", "candidate_card"]},
        {"id": "key_findings", "type": "markdown", "body": f"## Every retained Track A row favors Yune on this Mac\n\nAll 17 median ratios and all 17 pooled-worst ratios are below 1.0×. The chart shows the same-Mac direction only; it deliberately does not visualize the source-mismatched Windows deltas as causal. Against final-Windows 443 ratios, the diagnostic classes are {classes['close']} close, {classes['notable']} notable, and {classes['material']} material; against signed Increment-0, all 17 differences are material. Because neither Windows reference uses the same Yune source, those labels describe difference rather than platform causality. The exact five observations and references remain in the table immediately after the chart."},
        {"id": "median_ratio_chart_first_block", "type": "chart", "chartId": "median_ratio_chart_first", "layout": "full"},
        {"id": "median_ratio_chart_second_block", "type": "chart", "chartId": "median_ratio_chart_second", "layout": "full"},
        {"id": "track_a_first_block", "type": "table", "tableId": "track_a_table_first", "layout": "full"},
        {"id": "track_a_second_block", "type": "table", "tableId": "track_a_table_second", "layout": "full"},
        {"id": "long_findings", "type": "markdown", "body": f"## The 37/59 paths show a large same-Mac advantage\n\nThe 37-character ratios are {ratios_37_text}; median {ratio_37['mac_median_ratio']:.3f}× and worst {ratio_37['mac_pooled_worst_ratio']:.3f}×. Median absolute latency is {num(y37['median_of_five_us']):.3f} µs/key for Yune versus {num(l37['median_of_five_us']):.3f} for librime.\n\nThe 59-character ratios are {ratios_59_text}; median {ratio_59['mac_median_ratio']:.3f}× and worst {ratio_59['mac_pooled_worst_ratio']:.3f}×. Median absolute latency is {num(y59['median_of_five_us']):.3f} µs/key for Yune versus {num(l59['median_of_five_us']):.3f} for librime. Both candidate pages exactly match same-run librime, so the prior page-order confounder is absent from these current measurements."},
        {"id": "track_b_findings", "type": "markdown", "body": "## Track B remains behavior-stable and fast on this Mac\n\nThe five Mac medians are 5.483, 5.253, 5.607, 5.663, and 6.445 µs/key; median 5.607, worst run median 6.445, pooled worst sample 7.520, and spread 22.7%. The candidate page exactly matches M57. Final-Windows 443 has a 16.900 µs/key median and signed Increment-0 has 315.646 µs/key, but those absolute comparisons are platform-specific diagnostics, not portable acceptance rows."},
        {"id": "track_b_block", "type": "table", "tableId": "track_b_table", "layout": "full"},
        {"id": "candidate_findings", "type": "markdown", "body": "## Candidate and model-owner shape rule out a broad Mac path split\n\nCandidate files, normalized owner shape, and normalized product/checksum shape are identical across all five current Mac rounds. Versus M57, 11/17 shared candidate pages are fully exact; the differences are expected source evolution, and Track B remains exact. Owner comparison finds 48 exact normalized shapes, 23 current-only bounded cache/provenance owners, and 2 M57-only Track B index shapes; the replacements remain byte-backed/mmap and have smaller index bytes. Luna product status is exact; both Track B rows retain the same checksums, readiness, storage, and mmap modes, differing only in byte_source_len.\n\nThe one current oracle gap is zhongdengchangdu: Yune emits 中等長度 | 中等 | 中的 | 種的 | 重的 while librime emits 中等長度 | 中等 | 中 | 種 | 重. `windows-zhongdengchangdu-evidence.csv` records the same mismatch in all 15 Windows M59 performance-ratchet runs for increments 4c, 4d, and 4e, including each exact repository source path. This is a cross-platform engine behavior issue rather than a Mac-specific performance discrepancy."},
        {"id": "candidate_first_block", "type": "table", "tableId": "candidate_table_first", "layout": "full"},
        {"id": "candidate_second_block", "type": "table", "tableId": "candidate_table_second", "layout": "full"},
        {"id": "model_block", "type": "table", "tableId": "model_table", "layout": "full"},
        {"id": "scope", "type": "markdown", "body": "## Scope, data, and metric definitions\n\nYune is clean detached source 0111cf47c09bfe7a4a3d55a1832f35a55bc59435. Oracle is clean detached librime 33e78140250125871856cdc5b42ddc6a5fcd3cd4. Each of five rounds uses the exact 17 Track A inputs and exact Track B input, 9 startup iterations, 60 session iterations, 80 key iterations, and product deployment. Ratio is Yune median µs/key divided by librime median µs/key. Median is the third sorted run ratio, pooled worst is the largest run ratio, and spread is (max-min)/min. Ratios and published spreads use the benchmark's three-decimal observations to match the signed Windows method. Full-precision sensitivity checks show the high tiny-ratio spreads are primarily genuine round-to-round variance: cszysmsrsd is 83.5% and zybfshmsru is 78.7% before rounding, while the 59-character row is 36.0%. Rounding changes magnitude in either direction; it is not the main explanation.\n\nWindows ceilings are immutable historical diagnostics. macOS memory and absolute latency are not interchangeable with Windows counters. The current Mac source is newer than both Windows references, so cross-platform deltas cannot be attributed to operating system alone."},
        {"id": "audit_block", "type": "table", "tableId": "audit_table", "layout": "full"},
        {"id": "methodology", "type": "markdown", "body": "## Methodology and reproducibility\n\nA release Yune dylib and pinned librime dylib were built once, then five logical rounds ran through the macOS in-process driver. The driver still performs sequential no-change build checks before its timed lanes; no compilation ran concurrently with measurement. Every round retained its raw summaries, candidate snapshots, M37 metrics, memory-owner profile, product status, environment and command receipts, and pre/post hashes. No measured round was retried or discarded.\n\nThe repository driver incorrectly rejects a truly external output root. A preserved external adapter changes exactly two path lines: the fixed clean worktree root and accepted external evidence root. Measured source and benchmark logic are unchanged. The independent protocol review recomputed all 13,600 Track A and 400 Track B raw samples, all 85 Track A ratios, Windows joins, and the aggregate table exactly.\n\nFrom the complete external packet root, reproduce the derived data and portable report with:\n\n```sh\npython3 aggregate/analyze.py \\\n  --evidence-root /absolute/path/to/yune-m59-current-macos-20260714 \\\n  --repo-root /absolute/path/to/yune\npython3 report/build_report_artifact.py\nnode report/deliver_report.mjs \\\n  --plugin-root /absolute/path/to/data-analytics-plugin \\\n  --input report/artifact.json \\\n  --output report/report.html \\\n  > report/delivery-receipt.json\n```"},
        {"id": "limitations", "type": "markdown", "body": "## Noise prevents a clean causal platform attribution\n\nThe Mac stayed on AC with Low Power Mode disabled and no thermal warning. No `tmutil status` receipt was captured; the boundary process snapshots show `backupd` at 0.0% except for 0.1% at run 2 end. Spotlight activity after checkout/build settled before round 1. However, point-in-time receipts record substantial UI activity around round boundaries—especially round 4 at start (Codex Renderer 67.4%, WindowServer 43.4%, Chrome 16.0%, Granola 13.3%). All rounds are retained, and the fixed-binary same-Mac direction survives the worst ratio for every row, but this is not a clean quiet-machine platform-isolation experiment.\n\nConsequently, the packet does not separate source evolution, macOS/toolchain behavior, and workload noise. It also captures complete-input page zero rather than every prefix and later page for all 17 inputs. These limits affect fine row ranking and causal explanation, not binary identity or deterministic candidate/model shape."},
        {"id": "recommendations", "type": "markdown", "body": "## Recommended next steps\n\n1. Treat the current-source native macOS Track A direction as established: Yune has a large same-Mac advantage over pinned librime on all 17 named Track A rows. Track B remains a separate Yune-only absolute product guard with no librime peer.\n2. Keep signed Windows ceilings unchanged and diagnostic only. Do not re-baseline from this Mac packet.\n3. Track zhongdengchangdu as a cross-platform candidate-oracle gap after M59; do not classify it as a Mac performance defect.\n4. If causal platform attribution matters, rerun only a tightly controlled source-matched Mac/Windows pair with fixed binaries and continuous quiet-machine monitoring; do not infer that split from this packet.\n5. Keep Apple memory work separate and use Apple-native footprint counters rather than Windows working-set/private/pagefile rows."},
        {"id": "further_questions", "type": "markdown", "body": "## Further questions\n\n- Which post-443 source changes account for the remaining Mac-versus-final-Windows ratio movement?\n- Under a continuously quiet source-matched run, how much row-level spread remains?\n- What implementation change should make zhongdengchangdu's fallback candidates exactly match librime without widening M59 scope?"},
    ]

    datasets = {
        "headline": headline,
        "track_a": track_a,
        "track_a_first": track_a_first,
        "track_a_second": track_a_second,
        "track_a_chart_first": track_a_chart_first,
        "track_a_chart_second": track_a_chart_second,
        "track_b": track_b,
        "candidates": candidates,
        "candidates_first": candidates_first,
        "candidates_second": candidates_second,
        "model_summary": model_summary,
        "run_audit": audit,
    }
    write_sqlite(OUT / "snapshot.sqlite", datasets)

    artifact = {
        "surface": "report",
        "manifest": {
            "version": 1, "surface": "report", "title": TITLE,
            "description": "Technical five-round source-current macOS performance, behavior, and provenance diagnostic.",
            "generatedAt": generated_at, "blocks": blocks, "cards": cards,
            "charts": charts, "tables": tables,
            "sources": [{"id": item["id"], "label": item["label"], "path": item["path"]} for item in sources],
        },
        "snapshot": {
            "version": 1, "status": "ready", "generatedAt": generated_at,
            "datasets": datasets,
        },
        "sources": sources,
    }
    (OUT / "artifact.json").write_text(json.dumps(artifact, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (OUT / "report-source-notes.md").write_text(
        "# Report source notes\n\n"
        "Audience: technical. Required sections are mapped directly to technical summary, findings, scope/definitions, methodology, limitations/robustness, recommendations, and further questions.\n\n"
        "Chart map: the Track A finding uses two single-series horizontal bar charts (9 and 8 measured rows) of mac_median_ratio by a compact input-plus-exact-ratio label. Each chart adds a clearly named synthetic 1.000x parity anchor solely to keep the comparison boundary in view. The charts support only the same-Mac direction; Windows deltas remain tabular because their sources do not match. Palette policy is single-root preferred with no redundant series legend. Final surface is report/report.html.\n\n"
        "Reproduction: run `python3 aggregate/analyze.py --evidence-root /absolute/path/to/yune-m59-current-macos-20260714 --repo-root /absolute/path/to/yune`, then this builder, then `node report/deliver_report.mjs --plugin-root /absolute/path/to/data-analytics-plugin --input report/artifact.json --output report/report.html`. The packet-local adapter delegates to the canonical portable-report delivery and verification pipeline, adding only a fail-closed desktop-width containment rule for the packaged reader's `100vw` header. The final receipt must report both 1440 px and 390 px viewports plus a passed source interaction.\n",
        encoding="utf-8",
    )
    print(f"wrote {OUT / 'artifact.json'}")


if __name__ == "__main__":
    main()
