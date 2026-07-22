#!/usr/bin/env python3
"""Generate parity-centered SVGs for the canonical performance reports."""

from __future__ import annotations

import csv
import math
from pathlib import Path
from xml.sax.saxutils import escape


ROOT = Path(__file__).resolve().parent
OUT = ROOT / "visuals"
HISTORY = ROOT.parent / "history" / "performance-ratio-visuals-2026-07-14"
HISTORY_OUT = HISTORY / "visuals"
INK = "#17202a"
MUTED = "#52616f"
GRID = "#d9e2ec"
BLUE = "#2463d4"
BLUE_LIGHT = "#d9e7ff"
GOLD = "#b7791f"
GOLD_LIGHT = "#f8e7bf"
ORANGE = "#c45a1b"
ORANGE_LIGHT = "#f8dfcf"
BG = "#fbfcfe"


def rows(name: str) -> list[dict[str, str]]:
    with (ROOT / name).open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def history_rows(name: str) -> list[dict[str, str]]:
    with (HISTORY / name).open(encoding="utf-8", newline="") as handle:
        return list(csv.DictReader(handle))


def text(x: float, y: float, value: str, size: int = 14, *, anchor: str = "start", weight: int = 400, fill: str = INK, family: str = "system-ui, sans-serif") -> str:
    return f'<text x="{x:.1f}" y="{y:.1f}" text-anchor="{anchor}" font-family="{family}" font-size="{size}" font-weight="{weight}" fill="{fill}">{escape(value)}</text>'


def multiline_text(x: float, y: float, values: list[str], size: int = 18, *, anchor: str = "start", weight: int = 400, fill: str = INK, family: str = "system-ui, sans-serif") -> str:
    start_y = y - (len(values) - 1) * size * 0.55
    tspans = []
    for index, value in enumerate(values):
        dy = 0 if index == 0 else size * 1.1
        tspans.append(f'<tspan x="{x:.1f}" dy="{dy:.1f}">{escape(value)}</tspan>')
    return f'<text x="{x:.1f}" y="{start_y:.1f}" text-anchor="{anchor}" font-family="{family}" font-size="{size}" font-weight="{weight}" fill="{fill}">{"".join(tspans)}</text>'


def line(x1: float, y1: float, x2: float, y2: float, stroke: str = GRID, width: float = 1, dash: str | None = None) -> str:
    extra = f' stroke-dasharray="{dash}"' if dash else ""
    return f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{stroke}" stroke-width="{width}"{extra}/>'


def circle(x: float, y: float, radius: float, fill: str, stroke: str = "none", width: float = 1.5) -> str:
    return f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{radius}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>'


def diamond(x: float, y: float, radius: float, fill: str, stroke: str, width: float = 1.5) -> str:
    points = f"{x:.1f},{y-radius:.1f} {x+radius:.1f},{y:.1f} {x:.1f},{y+radius:.1f} {x-radius:.1f},{y:.1f}"
    return f'<polygon points="{points}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>'


def square(x: float, y: float, radius: float, fill: str, stroke: str, width: float = 1.5) -> str:
    return f'<rect x="{x-radius:.1f}" y="{y-radius:.1f}" width="{radius*2:.1f}" height="{radius*2:.1f}" fill="{fill}" stroke="{stroke}" stroke-width="{width}"/>'


def svg_start(
    width: int,
    height: int,
    title_value: str,
    desc_value: str,
    *,
    intrinsic_dimensions: bool = False,
) -> list[str]:
    dimensions = f' width="{width}" height="{height}"' if intrinsic_dimensions else ""
    return [
        f'<svg xmlns="http://www.w3.org/2000/svg"{dimensions} viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc" style="max-width:100%;height:auto;background:{BG}">',
        f'<title id="title">{escape(title_value)}</title>',
        f'<desc id="desc">{escape(desc_value)}</desc>',
        f'<rect width="{width}" height="{height}" fill="{BG}"/>',
    ]


def log_scale(left: float, right: float, minimum: float, maximum: float):
    lo = math.log2(minimum)
    hi = math.log2(maximum)
    return lambda value: left + (math.log2(value) - lo) / (hi - lo) * (right - left)


def current_mac_chart() -> None:
    data = rows("current-macos-track-a-ratios.csv")
    width, height = 1420, 930
    left, right = 250, 1010
    top, row_h = 182, 39
    scale = log_scale(left, right, 0.00390625, 2.0)
    tick_values = [0.004, 0.008, 0.016, 0.031, 0.063, 0.125, 0.250, 0.500, 1.000, 2.000]
    parts = svg_start(
        width, height,
        "Current-source macOS Track A Yune/librime ratios",
        "Seventeen logarithmic intervals show each five-run median and pooled worst. Ratio is Yune latency divided by same-run librime latency. Every value is left of the bold 1.000x parity line.",
    )
    parts += [
        text(48, 48, "Current-source macOS Track A ratios", 28, weight=700),
        text(48, 78, "Yune latency / same-run librime latency · five rounds at Yune 0111cf47", 16, fill=MUTED),
        text(left, 116, "← Yune lower latency (<1.000x)", 15, weight=600, fill=BLUE),
        text(right, 116, "librime lower latency (>1.000x) →", 15, anchor="end", weight=600, fill=ORANGE),
        text(1095, 148, "Median", 13, anchor="end", weight=700, fill=MUTED),
        text(1195, 148, "Worst", 13, anchor="end", weight=700, fill=MUTED),
        text(1365, 148, "Parity read", 13, anchor="end", weight=700, fill=MUTED),
    ]
    bottom = top + row_h * (len(data) - 1) + 22
    for value in tick_values:
        x = scale(value)
        strong = abs(value - 1.0) < 1e-9
        parts.append(line(x, 145, x, bottom, INK if strong else GRID, 3 if strong else 1, None if strong else "3 5"))
        parts.append(text(x, bottom + 28, f"{value:.3f}x" if value < 1 else f"{value:.1f}x", 12, anchor="middle", weight=700 if strong else 400, fill=INK if strong else MUTED, family="ui-monospace, monospace"))
        if strong:
            parts.append(text(x, 139, "1.000x PARITY", 13, anchor="middle", weight=800, fill=INK))
    for index, row in enumerate(data):
        y = top + index * row_h
        median = float(row["mac_median_yune_librime_ratio"])
        worst = float(row["mac_pooled_worst_yune_librime_ratio"])
        xm, xw = scale(median), scale(worst)
        if index % 2 == 0:
            parts.append(f'<rect x="36" y="{y-18:.1f}" width="1348" height="36" rx="6" fill="#f3f6fa"/>')
        parts.append(text(232, y + 5, row["label"], 14, anchor="end", weight=600))
        parts.append(line(xm, y, xw, y, BLUE, 5))
        parts.append(line(xw, y - 7, xw, y + 7, BLUE, 2))
        parts.append(circle(xm, y, 6, BLUE, INK, 1))
        parts.append(text(1095, y + 5, f"{median:.3f}x", 14, anchor="end", weight=700, family="ui-monospace, monospace"))
        parts.append(text(1195, y + 5, f"{worst:.3f}x", 14, anchor="end", weight=700, family="ui-monospace, monospace"))
        parts.append(text(1365, y + 5, "Yune lower", 13, anchor="end", weight=600, fill=BLUE))
    parts += [
        circle(60, height - 35, 6, BLUE, INK, 1),
        text(74, height - 30, "median", 13, fill=MUTED),
        line(150, height - 35, 198, height - 35, BLUE, 5),
        line(198, height - 42, 198, height - 28, BLUE, 2),
        text(210, height - 30, "median-to-pooled-worst interval", 13, fill=MUTED),
        text(1365, height - 30, "Exact values are printed at right; logarithmic axis", 12, anchor="end", fill=MUTED),
        "</svg>",
    ]
    (OUT / "current-macos-track-a-parity.svg").write_text("\n".join(parts) + "\n", encoding="utf-8")


def current_windows_chart() -> None:
    data = rows("current-windows-track-a-ratios.csv")
    width, height = 1420, 930
    left, right = 250, 1010
    top, row_h = 182, 39
    scale = log_scale(left, right, 0.00390625, 2.0)
    tick_values = [0.004, 0.008, 0.016, 0.031, 0.063, 0.125, 0.250, 0.500, 1.000, 2.000]
    parts = svg_start(
        width, height,
        "Final-M59 Windows Track A Yune/librime ratios",
        "Seventeen logarithmic intervals show each five-run median and worst observation. Ratio is Yune latency divided by same-run librime latency. Every value is left of the bold 1.000x parity line.",
    )
    parts += [
        text(48, 48, "Final-M59 Windows Track A ratios", 28, weight=700),
        text(48, 78, "Yune latency / same-run librime latency · five rounds at Yune 443cc636", 16, fill=MUTED),
        text(left, 116, "← Yune lower latency (<1.000x)", 15, weight=600, fill=BLUE),
        text(right, 116, "librime lower latency (>1.000x) →", 15, anchor="end", weight=600, fill=ORANGE),
        text(1095, 148, "Median", 13, anchor="end", weight=700, fill=MUTED),
        text(1195, 148, "Worst", 13, anchor="end", weight=700, fill=MUTED),
        text(1365, 148, "Parity read", 13, anchor="end", weight=700, fill=MUTED),
    ]
    bottom = top + row_h * (len(data) - 1) + 22
    for value in tick_values:
        x = scale(value)
        strong = abs(value - 1.0) < 1e-9
        parts.append(line(x, 145, x, bottom, INK if strong else GRID, 3 if strong else 1, None if strong else "3 5"))
        parts.append(text(x, bottom + 28, f"{value:.3f}x" if value < 1 else f"{value:.1f}x", 12, anchor="middle", weight=700 if strong else 400, fill=INK if strong else MUTED, family="ui-monospace, monospace"))
        if strong:
            parts.append(text(x, 139, "1.000x PARITY", 13, anchor="middle", weight=800, fill=INK))
    for index, row in enumerate(data):
        y = top + index * row_h
        median = float(row["windows_median_yune_librime_ratio"])
        worst = float(row["windows_worst_yune_librime_ratio"])
        xm, xw = scale(median), scale(worst)
        if index % 2 == 0:
            parts.append(f'<rect x="36" y="{y-18:.1f}" width="1348" height="36" rx="6" fill="#f3f6fa"/>')
        parts.append(text(232, y + 5, row["label"], 14, anchor="end", weight=600))
        parts.append(line(xm, y, xw, y, BLUE, 5))
        parts.append(line(xw, y - 7, xw, y + 7, BLUE, 2))
        parts.append(circle(xm, y, 6, BLUE, INK, 1))
        parts.append(text(1095, y + 5, f"{median:.3f}x", 14, anchor="end", weight=700, family="ui-monospace, monospace"))
        parts.append(text(1195, y + 5, f"{worst:.3f}x", 14, anchor="end", weight=700, family="ui-monospace, monospace"))
        parts.append(text(1365, y + 5, "Yune lower", 13, anchor="end", weight=600, fill=BLUE))
    parts += [
        circle(60, height - 35, 6, BLUE, INK, 1),
        text(74, height - 30, "median", 13, fill=MUTED),
        line(150, height - 35, 198, height - 35, BLUE, 5),
        line(198, height - 42, 198, height - 28, BLUE, 2),
        text(210, height - 30, "median-to-worst interval", 13, fill=MUTED),
        text(1365, height - 30, "Exact values are printed at right; logarithmic axis", 12, anchor="end", fill=MUTED),
        "</svg>",
    ]
    (OUT / "current-windows-track-a-parity.svg").write_text("\n".join(parts) + "\n", encoding="utf-8")


def current_native_split_chart(
    *,
    data: list[dict[str, str]],
    filename: str,
    platform_title: str,
    source_label: str,
    median_field: str,
    worst_field: str,
    part: int,
) -> None:
    start = 0 if part == 1 else 9
    stop = 9 if part == 1 else 17
    selected = data[start:stop]
    width = 760
    left, right = 180, 500
    top, row_h = 194, 50
    bottom = top + row_h * (len(selected) - 1) + 24
    height = bottom + 82
    scale = log_scale(left, right, 0.00390625, 2.0)
    ticks = [0.004, 0.016, 0.063, 0.250, 1.000, 2.000]
    parts = svg_start(
        width,
        height,
        f"{platform_title} Track A Yune/librime ratios, part {part} of 2",
        f"{len(selected)} logarithmic intervals show each five-run median and worst observation. Ratio is Yune latency divided by same-platform pinned librime latency. Exact three-decimal values are printed, and every value is left of the 1.000x parity line.",
        intrinsic_dimensions=True,
    )
    parts += [
        text(24, 46, f"{platform_title} Track A ratios — {part}/2", 28, weight=700),
        text(24, 78, f"Inputs {start + 1}–{stop} of 17 · Yune / same-platform librime · {source_label}", 17, fill=MUTED),
        text(left, 118, "← Yune lower", 17, weight=600, fill=BLUE),
        text(right, 118, "librime lower →", 17, anchor="end", weight=600, fill=ORANGE),
        text(600, 160, "Median", 17, anchor="end", weight=700, fill=MUTED),
        text(700, 160, "Worst", 17, anchor="end", weight=700, fill=MUTED),
    ]
    for value in ticks:
        x = scale(value)
        strong = abs(value - 1.0) < 1e-9
        line_top = 180 if strong else 156
        parts.append(line(x, line_top, x, bottom, INK if strong else GRID, 3 if strong else 1, None if strong else "3 5"))
        tick_label = f"{value:.3f}x" if value <= 1 else f"{value:.1f}x"
        tick_x = x + 6 if value == 2 else x
        tick_anchor = "start" if value == 2 else "middle"
        parts.append(text(tick_x, bottom + 31, tick_label, 16, anchor=tick_anchor, weight=800 if strong else 400, fill=INK if strong else MUTED, family="ui-monospace, monospace"))
        if strong:
            parts.append(text(x, 146, "1.000x", 18, anchor="middle", weight=800))
            parts.append(text(x, 173, "PARITY", 16, anchor="middle", weight=800))
    for index, row in enumerate(selected):
        y = top + index * row_h
        median = float(row[median_field])
        worst = float(row[worst_field])
        xm, xw = scale(median), scale(worst)
        if index % 2 == 0:
            parts.append(f'<rect x="12" y="{y-22:.1f}" width="724" height="44" rx="7" fill="#f3f6fa"/>')
        label_lines = [row["label"]]
        if row["label"].casefold() == "zhongdengchangdu":
            label_lines = ["zhongdeng", "changdu"]
        parts.append(multiline_text(166, y + 6, label_lines, 18, anchor="end", weight=600))
        parts.append(line(xm, y, xw, y, BLUE, 6))
        parts.append(line(xw, y - 9, xw, y + 9, BLUE, 2.5))
        parts.append(circle(xm, y, 7, BLUE, INK, 1))
        parts.append(text(600, y + 7, f"{median:.3f}x", 20, anchor="end", weight=700, family="ui-monospace, monospace"))
        parts.append(text(700, y + 7, f"{worst:.3f}x", 20, anchor="end", weight=700, family="ui-monospace, monospace"))
    parts += [
        circle(24, height - 28, 7, BLUE, INK, 1),
        text(42, height - 22, "median", 16, fill=MUTED),
        line(120, height - 28, 178, height - 28, BLUE, 6),
        line(178, height - 37, 178, height - 19, BLUE, 2.5),
        text(190, height - 22, "median-to-worst interval", 16, fill=MUTED),
        text(730, height - 22, "Exact values · log axis", 16, anchor="end", fill=MUTED),
        "</svg>",
    ]
    (OUT / filename).write_text("\n".join(parts) + "\n", encoding="utf-8")


def current_native_split_charts() -> None:
    mac = rows("current-macos-track-a-ratios.csv")
    windows = rows("current-windows-track-a-ratios.csv")
    for part in (1, 2):
        current_native_split_chart(
            data=mac,
            filename=f"current-macos-track-a-parity-{part}-of-2.svg",
            platform_title="Current-source macOS",
            source_label="Yune 0111cf47",
            median_field="mac_median_yune_librime_ratio",
            worst_field="mac_pooled_worst_yune_librime_ratio",
            part=part,
        )
        current_native_split_chart(
            data=windows,
            filename=f"current-windows-track-a-parity-{part}-of-2.svg",
            platform_title="Final-M59 Windows",
            source_label="Yune 443cc636",
            median_field="windows_median_yune_librime_ratio",
            worst_field="windows_worst_yune_librime_ratio",
            part=part,
        )


def paired_chart() -> None:
    data = history_rows("paired-587-track-a-ratios.csv")
    width, height = 1420, 930
    left, right = 250, 1010
    top, row_h = 182, 39
    scale = log_scale(left, right, 0.00390625, 2.0)
    tick_values = [0.004, 0.008, 0.016, 0.031, 0.063, 0.125, 0.250, 0.500, 1.000, 2.000]
    parts = svg_start(
        width, height,
        "Source-matched 587 Mac and Windows Yune/librime ratios",
        "Seventeen paired logarithmic points compare Mac and Windows at the same Yune commit 5879405c. Ratio is Yune latency divided by same-platform librime latency. Every point is below 1.000x parity.",
    )
    parts += [
        text(48, 48, "Source-matched Mac and Windows ratios", 28, weight=700),
        text(48, 78, "Yune 5879405c on both platforms · Yune latency / same-platform librime latency", 16, fill=MUTED),
        text(left, 116, "← Yune lower latency (<1.000x)", 15, weight=600, fill=BLUE),
        text(right, 116, "librime lower latency (>1.000x) →", 15, anchor="end", weight=600, fill=ORANGE),
        text(1110, 148, "Mac", 13, anchor="end", weight=700, fill=MUTED),
        text(1215, 148, "Windows", 13, anchor="end", weight=700, fill=MUTED),
        text(1365, 148, "Both vs parity", 13, anchor="end", weight=700, fill=MUTED),
    ]
    bottom = top + row_h * (len(data) - 1) + 22
    for value in tick_values:
        x = scale(value)
        strong = abs(value - 1.0) < 1e-9
        parts.append(line(x, 145, x, bottom, INK if strong else GRID, 3 if strong else 1, None if strong else "3 5"))
        parts.append(text(x, bottom + 28, f"{value:.3f}x" if value < 1 else f"{value:.1f}x", 12, anchor="middle", weight=700 if strong else 400, fill=INK if strong else MUTED, family="ui-monospace, monospace"))
        if strong:
            parts.append(text(x, 139, "1.000x PARITY", 13, anchor="middle", weight=800))
    for index, row in enumerate(data):
        y = top + index * row_h
        mac = float(row["mac_587_median_yune_librime_ratio"])
        win = float(row["windows_587_median_yune_librime_ratio"])
        xm, xw = scale(mac), scale(win)
        if index % 2 == 0:
            parts.append(f'<rect x="36" y="{y-18:.1f}" width="1348" height="36" rx="6" fill="#f3f6fa"/>')
        parts.append(text(232, y + 5, row["label"], 14, anchor="end", weight=600))
        parts.append(line(min(xm, xw), y, max(xm, xw), y, GRID, 3))
        parts.append(circle(xm, y, 6, BLUE, INK, 1))
        parts.append(diamond(xw, y, 7, BG, GOLD, 2.5))
        parts.append(text(1110, y + 5, f"{mac:.3f}x", 14, anchor="end", weight=700, family="ui-monospace, monospace"))
        parts.append(text(1215, y + 5, f"{win:.3f}x", 14, anchor="end", weight=700, family="ui-monospace, monospace"))
        parts.append(text(1365, y + 5, "Yune lower", 13, anchor="end", weight=600, fill=BLUE))
    parts += [
        circle(60, height - 35, 6, BLUE, INK, 1), text(74, height - 30, "Mac median", 13, fill=MUTED),
        diamond(190, height - 35, 7, BG, GOLD, 2.5), text(205, height - 30, "Windows median", 13, fill=MUTED),
        text(1365, height - 30, "Exact values are printed at right; logarithmic axis", 12, anchor="end", fill=MUTED),
        "</svg>",
    ]
    (HISTORY_OUT / "paired-587-macos-windows-parity.svg").write_text("\n".join(parts) + "\n", encoding="utf-8")


def behavior_chart() -> None:
    data = history_rows("long-input-behavior-ratios.csv")
    width, height = 1320, 470
    left, right = 250, 1080
    scale = log_scale(left, right, 0.125, 2.0)
    ticks = [0.125, 0.250, 0.500, 1.000, 2.000]
    parts = svg_start(
        width, height,
        "Historical long-input behavior normalization crosses parity",
        "For both 37- and 59-character inputs, the all-prefix Yune/librime latency ratio is below parity, but candidate-text-matched and final-key ratios are above parity. Ratio is Yune latency divided by librime latency.",
    )
    parts += [
        text(48, 48, "Historical long-input timing flips after behavior normalization", 27, weight=700),
        text(48, 78, "afb7079b behavior-strata control · Yune latency / librime latency", 16, fill=MUTED),
        text(left, 118, "← Yune lower latency", 15, weight=600, fill=BLUE),
        text(right, 118, "librime lower latency →", 15, anchor="end", weight=600, fill=ORANGE),
    ]
    for value in ticks:
        x = scale(value)
        strong = value == 1.0
        parts.append(line(x, 138, x, 360, INK if strong else GRID, 3 if strong else 1, None if strong else "3 5"))
        parts.append(text(x, 387, f"{value:.3f}x" if value < 1 else f"{value:.1f}x", 12, anchor="middle", weight=800 if strong else 400, family="ui-monospace, monospace"))
        if strong:
            parts.append(text(x, 132, "1.000x PARITY", 13, anchor="middle", weight=800))
    y_values = [205, 305]
    series = [
        ("All-prefix sum", "all_prefix_sum_ratio", BLUE, "circle"),
        ("Candidate-text-matched", "text_matched_prefix_sum_ratio", GOLD, "diamond"),
        ("Final key", "final_key_ratio", ORANGE, "square"),
    ]
    for row, y in zip(data, y_values):
        parts.append(text(225, y + 5, row["label"], 16, anchor="end", weight=700))
        points = []
        for idx, (label, field, color, shape) in enumerate(series):
            value = float(row[field])
            x = scale(value)
            offset = (-13, 0, 13)[idx]
            points.append((x, y + offset))
            if shape == "diamond":
                parts.append(diamond(x, y + offset, 7, BG, color, 2.5))
            elif shape == "square":
                parts.append(square(x, y + offset, 6, color, INK, 1))
            else:
                parts.append(circle(x, y + offset, 6, color, INK, 1))
            label_anchor = "end" if value > 1 else "start"
            label_x = x - 10 if value > 1 else x + 10
            parts.append(text(label_x, y + offset + 5, f"{value:.3f}x", 13, anchor=label_anchor, weight=700, fill=color, family="ui-monospace, monospace"))
        parts.append(line(points[0][0], points[0][1], points[1][0], points[1][1], GRID, 2))
        parts.append(line(points[1][0], points[1][1], points[2][0], points[2][1], GRID, 2))
    legend_y = 435
    x = 48
    for label, _, color, shape in series:
        if shape == "diamond":
            parts.append(diamond(x, legend_y - 5, 7, BG, color, 2.5))
        elif shape == "square":
            parts.append(square(x, legend_y - 5, 6, color, INK, 1))
        else:
            parts.append(circle(x, legend_y - 5, 6, color, INK, 1))
        parts.append(text(x + 15, legend_y, label, 13, fill=MUTED))
        x += 230
    parts += [text(1275, 435, "Logarithmic ratio axis", 12, anchor="end", fill=MUTED), "</svg>"]
    (HISTORY_OUT / "long-input-behavior-parity.svg").write_text("\n".join(parts) + "\n", encoding="utf-8")


def browser_chart() -> None:
    data = rows("browser-peer-ratios.csv")
    width, height = 760, 640
    left, right = 205, 555
    scale = log_scale(left, right, 0.5, 8.0)
    parity_x = scale(1.0)
    ticks = [0.5, 1.0, 2.0, 4.0, 8.0]
    parts = svg_start(
        width, height,
        "Dated same-schema browser snapshot with interaction withdrawals",
        "Six dated luna_pinyin browser quotients plot Yune divided by My RIME on a logarithmic axis. Input to candidate at 0.779x and commit at 0.899x are historical values withdrawn on 2026-07-21 for endpoint mismatch. Ready time, WASM, and payload rows remain dated peer observations. Jyutping is excluded because its dictionaries differ.",
        intrinsic_dimensions=True,
    )
    parts += [
        text(24, 46, "Browser peer ratio snapshot", 30, weight=700),
        text(24, 78, "2026-06-28 luna_pinyin · interaction correction 2026-07-21", 17, fill=MUTED),
        text(left, 118, "← quotient below 1.000x", 16, weight=600, fill=BLUE),
        text(right, 118, "quotient above 1.000x →", 16, anchor="end", weight=600, fill=ORANGE),
        text(730, 160, "Historical Yune / peer", 17, anchor="end", weight=700, fill=MUTED),
    ]
    top, row_h = 198, 64
    bottom = top + row_h * (len(data) - 1) + 28
    for value in ticks:
        x = scale(value)
        strong = value == 1.0
        parts.append(line(x, 156, x, bottom, INK if strong else GRID, 3 if strong else 1, None if strong else "3 5"))
        parts.append(text(x, bottom + 32, f"{value:.1f}x", 16, anchor="middle", weight=800 if strong else 400, family="ui-monospace, monospace"))
        if strong:
            parts.append(text(x, 150, "1.000x PARITY", 18, anchor="middle", weight=800))
    for index, row in enumerate(data):
        y = top + index * row_h
        ratio = float(row["yune_my_rime_ratio"])
        x = scale(ratio)
        withdrawn = row["claim_status"] == "WITHDRAWN_ENDPOINT_MISMATCH"
        if withdrawn:
            color = MUTED
            fill = "#edf1f4"
        else:
            color = BLUE if ratio < 1 else ORANGE
            fill = BLUE_LIGHT if ratio < 1 else ORANGE_LIGHT
        if index % 2 == 0:
            parts.append(f'<rect x="12" y="{y-27:.1f}" width="724" height="54" rx="7" fill="#f3f6fa"/>')
        label_lines = [row["metric"]]
        if row["metric"].casefold() == "unique encoded resources":
            label_lines = ["Unique encoded", "resources"]
        parts.append(multiline_text(190, y + 6, label_lines, 19, anchor="end", weight=600))
        rect_x = min(parity_x, x)
        rect_w = max(2, abs(x - parity_x))
        dash = ' stroke-dasharray="4 3"' if withdrawn else ""
        parts.append(f'<rect x="{rect_x:.1f}" y="{y-10:.1f}" width="{rect_w:.1f}" height="20" rx="5" fill="{fill}" stroke="{color}" stroke-width="1.5"{dash}/>')
        parts.append(circle(x, y, 7, color, INK, 1))
        parts.append(text(730, y + 7, f"{ratio:.3f}x", 20, anchor="end", weight=800, fill=color, family="ui-monospace, monospace"))
        if withdrawn:
            parts.append(text(730, y + 24, "WITHDRAWN_ENDPOINT_MISMATCH", 11, anchor="end", weight=800, fill="#9b2c2c", family="ui-monospace, monospace"))
    parts += [
        text(24, height - 24, "Withdrawn interaction values are history, not peer evidence · other rows unchanged", 15, fill=MUTED),
        "</svg>",
    ]
    (OUT / "browser-luna-peer-parity.svg").write_text("\n".join(parts) + "\n", encoding="utf-8")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    HISTORY_OUT.mkdir(parents=True, exist_ok=True)
    current_mac_chart()
    current_windows_chart()
    current_native_split_charts()
    paired_chart()
    behavior_chart()
    browser_chart()
    print("generated 7 current and 2 archived parity SVGs")


if __name__ == "__main__":
    main()
