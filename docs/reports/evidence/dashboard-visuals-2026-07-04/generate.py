#!/usr/bin/env python3
"""Generate the current Yune-vs-librime dashboard visuals (2026-07-04).

Deterministic, dependency-free SVG generator. Native numbers are read from the
M55 corrective gate run D (per-keypress context reads; the pre-corrective
run-6 numbers were batch-shaped and are superseded). Browser rows are carried
from the existing 2026-06-29 dashboard because M55 did not remeasure the
browser lane.

Re-run:
  python docs/reports/evidence/dashboard-visuals-2026-07-04/generate.py
"""
from __future__ import annotations

import csv
from pathlib import Path

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[3]
M55_FINAL = (
    ROOT
    / "docs/reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/gate-run-d"
)
SUMMARY = M55_FINAL / "summary-comparison.csv"
SUMMARY_ALL = M55_FINAL / "summary.csv"
OWNER_PROFILE = M55_FINAL / "memory-owner-profile.csv"

W = 1040
LABEL_RIGHT = 258
PLOT_LEFT = 270
PLOT_RIGHT = 740
PLOT_W = PLOT_RIGHT - PLOT_LEFT
PITCH = 34
BAR_H = 18
FONT = "Segoe UI, Arial, sans-serif"

COLORS = {
    "pass": ("#A3D576", "#386411"),
    "warn": ("#F0986E", "#804126"),
    "amber": ("#F2C879", "#9C7B1E"),
    "peer": ("#BFC6D4", "#868EA0"),
    "after": ("#7FB1D6", "#355E80"),
    "owner": ("#C7B3E0", "#5E4B86"),
}

LABELS = {
    "n": "n (short key)",
    "ni": "ni (short key)",
    "hao": "hao (short key)",
    "zhongguo": "zhongguo (common word)",
    "cszysmsrsd": "cszysmsrsd (10-char abbr)",
    "zybfshmsru": "zybfshmsru (8-char abbr)",
    "ceshiyixiachangjushuruxingnengzenyang": "37-char pinyin",
    "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong": "59-char pinyin",
}


def read_csv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="", encoding="utf-8-sig") as f:
        return list(csv.DictReader(f))


SUMMARY_ROWS = read_csv(SUMMARY)
SUMMARY_ALL_ROWS = read_csv(SUMMARY_ALL)
OWNER_ROWS = read_csv(OWNER_PROFILE)


def esc(s: str) -> str:
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def fnum(row: dict[str, str], field: str) -> float:
    return float(row[field])


def mb_from_bytes(value: float) -> float:
    return value / 1_000_000.0


def find_summary(track: str, workload: str, input_value: str = "") -> dict[str, str]:
    for row in SUMMARY_ROWS:
        if (
            row["track"] == track
            and row["workload"] == workload
            and row["input"] == input_value
        ):
            return row
    raise KeyError((track, workload, input_value))


def find_engine_summary(
    engine: str, track: str, workload: str, input_value: str = ""
) -> dict[str, str]:
    for row in SUMMARY_ALL_ROWS:
        if (
            row["engine"] == engine
            and row["track"] == track
            and row["workload"] == workload
            and row["input"] == input_value
        ):
            return row
    raise KeyError((engine, track, workload, input_value))


def owner_bytes(owner_id: str, track: str = "track-a-comparison") -> float:
    for row in OWNER_ROWS:
        if row["track"] == track and row["owner_id"] == owner_id:
            return fnum(row, "retained_estimate_bytes")
    raise KeyError((track, owner_id))


def svg_chart(
    filename: str,
    title: str,
    sub_lines: list[str],
    sections: list[dict[str, object]],
    axis_max: float,
    gridlines: list[float],
    axis_fmt,
    refs: list[tuple[float, str, bool, bool]] | None = None,
) -> None:
    def x_of(v: float) -> float:
        return PLOT_LEFT + (v / axis_max) * PLOT_W

    top_start = 54 + 16 * len(sub_lines) + 10
    y = top_start
    plot_top = top_start - 1
    body: list[str] = []
    for sec_index, sec in enumerate(sections):
        heading = sec.get("heading")
        if heading:
            body.append(
                f'  <text x="28" y="{y+13}" font-family="{FONT}" font-size="11" '
                f'font-weight="700" fill="#1F2430">{esc(str(heading))}</text>'
            )
            y += 24
        for row in sec["rows"]:  # type: ignore[index]
            label, value, color, primary, secondary = row
            fill, stroke = COLORS[color]
            bw = max(2.0, (float(value) / axis_max) * PLOT_W)
            baseline = y + 14
            body.append(
                f'  <text x="{LABEL_RIGHT}" y="{baseline}" font-family="{FONT}" '
                f'font-size="11" font-weight="500" fill="#1F2430" '
                f'text-anchor="end">{esc(str(label))}</text>'
            )
            body.append(
                f'  <rect x="{PLOT_LEFT}" y="{y}" width="{bw:.1f}" height="{BAR_H}" '
                f'rx="2" fill="{fill}" stroke="{stroke}" stroke-width="1"/>'
            )
            body.append(
                f'  <text x="{PLOT_LEFT+bw+6:.1f}" y="{baseline}" font-family="{FONT}" '
                f'font-size="11" font-weight="700" fill="#1F2430">{esc(str(primary))}</text>'
            )
            if secondary:
                off = PLOT_LEFT + bw + 14 + 7.0 * len(str(primary))
                body.append(
                    f'  <text x="{off:.1f}" y="{baseline}" font-family="{FONT}" '
                    f'font-size="10" fill="#6F768A">{esc(str(secondary))}</text>'
                )
            y += PITCH
        if sec_index != len(sections) - 1:
            y += 4
            body.append(
                f'  <line x1="28" y1="{y-2}" x2="{W-28}" y2="{y-2}" '
                f'stroke="#E6E8F0" stroke-width="1"/>'
            )
            y += 8
    plot_bottom = y + 2
    height = plot_bottom + 26

    head = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{height}" '
        f'viewBox="0 0 {W} {height}" role="img" aria-labelledby="title desc">',
        f'  <title id="title">{esc(title)}</title>',
        f'  <desc id="desc">{esc(" ".join(sub_lines))}</desc>',
        f'  <rect width="{W}" height="{height}" fill="#FCFCFD"/>',
        f'  <text x="28" y="34" font-family="{FONT}" font-size="18" '
        f'font-weight="700" fill="#1F2430">{esc(title)}</text>',
    ]
    for i, line in enumerate(sub_lines):
        head.append(
            f'  <text x="28" y="{54 + 16 * i}" font-family="{FONT}" font-size="12" '
            f'fill="#6F768A">{esc(line)}</text>'
        )
    grid = []
    for gv in gridlines:
        gx = x_of(gv)
        grid.append(
            f'  <line x1="{gx:.1f}" y1="{plot_top}" x2="{gx:.1f}" y2="{plot_bottom}" '
            f'stroke="#E6E8F0" stroke-width="1"/>'
        )
        grid.append(
            f'  <text x="{gx:.1f}" y="{plot_bottom+18}" font-family="{FONT}" '
            f'font-size="10" fill="#6F768A" text-anchor="middle">{axis_fmt(gv)}</text>'
        )
    ref_el = []
    for rv, rlabel, dashed, emph in refs or []:
        gx = x_of(rv)
        dash = ' stroke-dasharray="4 4"' if dashed else ""
        sw = "2" if emph else "1.2"
        ref_el.append(
            f'  <line x1="{gx:.1f}" y1="{plot_top-2}" x2="{gx:.1f}" y2="{plot_bottom}" '
            f'stroke="#1F2430" stroke-width="{sw}"{dash}/>'
        )
        ref_el.append(
            f'  <text x="{gx+4:.1f}" y="{plot_top-6}" font-family="{FONT}" '
            f'font-size="10" font-weight="600" fill="#1F2430">{esc(rlabel)}</text>'
        )

    out = "\n".join(head + grid + ref_el + body + ["</svg>", ""])
    (OUT / filename).write_text(out, encoding="utf-8", newline="\n")
    print(f"wrote {filename} ({height}px)")


def latency_row(input_value: str) -> tuple[str, float, str, str, str]:
    row = find_summary("track-a-comparison", "key_sequence_process_with_context", input_value)
    ratio = fnum(row, "yune_librime_median_ratio")
    gap = fnum(row, "absolute_gap_us")
    color = "pass" if ratio <= 1.0 else "warn"
    sign = "+" if gap >= 0 else ""
    return (
        LABELS[input_value],
        ratio,
        color,
        f"{ratio:.3f}x",
        f"Yune {sign}{gap:.1f} us",
    )


track_a_inputs = [
    "zhongguo",
    "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
    "cszysmsrsd",
    "zybfshmsru",
    "ceshiyixiachangjushuruxingnengzenyang",
    "hao",
    "ni",
    "n",
]

startup = find_summary("track-a-comparison", "startup_warm_shared_assets_runtime_ready")
session = find_summary("track-a-comparison", "session_create_select_destroy")

svg_chart(
    "native-track-a-latency-ratios.svg",
    "Native Track A latency: M55 final Yune vs librime 1.17.0",
    [
        "M55 corrective gate run D (context read per keypress), 2026-07-04. Bar = Yune median / same-run upstream librime median; lower is better.",
        "Rows below 1x are faster than librime in this gate. The long rows measure type-sequence then one context observation.",
    ],
    [
        {
            "heading": "Key-sequence with context latency",
            "rows": [latency_row(input_value) for input_value in track_a_inputs],
        },
        {
            "heading": "One-time lifecycle",
            "rows": [
                (
                    "startup warm runtime-ready",
                    fnum(startup, "yune_librime_median_ratio"),
                    "pass",
                    f'{fnum(startup, "yune_librime_median_ratio"):.3f}x',
                    f'{float(startup["yune_median_us"]):,.1f} vs {float(startup["librime_median_us"]):,.1f} us',
                ),
                (
                    "session create/select/destroy",
                    fnum(session, "yune_librime_median_ratio"),
                    "pass",
                    f'{fnum(session, "yune_librime_median_ratio"):.3f}x',
                    f'{float(session["yune_median_us"]):,.1f} vs {float(session["librime_median_us"]):,.1f} us',
                ),
            ],
        },
    ],
    axis_max=2.0,
    gridlines=[0.0, 0.5, 1.0, 1.5, 2.0],
    axis_fmt=lambda v: f"{v:.1f}x",
    refs=[(1.0, "1x parity", False, True), (2.0, "M55 short-key ceiling", True, False)],
)

track_a_yune_peak = max(
    fnum(row, "yune_max_peak_working_set_bytes")
    for row in SUMMARY_ROWS
    if row["track"] == "track-a-comparison"
)
track_a_librime_peak = max(
    fnum(row, "librime_max_peak_working_set_bytes")
    for row in SUMMARY_ROWS
    if row["track"] == "track-a-comparison"
)

RUN_A = (
    ROOT
    / "docs/reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/run-a-byte-backed-default"
)
RUN_A_ROWS = read_csv(RUN_A / "summary-comparison.csv")
run_a_peak = max(
    fnum(row, "yune_max_peak_working_set_bytes")
    for row in RUN_A_ROWS
    if row["track"] == "track-a-comparison"
)

svg_chart(
    "native-track-a-memory.svg",
    "Native Track A memory: corrective high-water and owners",
    [
        "M55 corrective gate run D (owned poet default), 2026-07-04. MB are decimal process proxies.",
        "The byte-backed opt-in row is from corrective run A; it fails the long-row latency ceilings until",
        "the incremental sentence scratch is ported to byte-backed storage.",
    ],
    [
        {
            "heading": "Peak working set, same-run lanes",
            "rows": [
                (
                    "Yune Track A peak (owned default)",
                    mb_from_bytes(track_a_yune_peak),
                    "warn",
                    f"{mb_from_bytes(track_a_yune_peak):.1f} MB",
                    "below 195.0 MB corrective ceiling",
                ),
                (
                    "Yune opt-in byte-backed peak",
                    mb_from_bytes(run_a_peak),
                    "after",
                    f"{mb_from_bytes(run_a_peak):.1f} MB",
                    "YUNE_POET_BYTE_BACKED=1; latency-blocked",
                ),
                (
                    "librime max peer peak",
                    mb_from_bytes(track_a_librime_peak),
                    "peer",
                    f"{mb_from_bytes(track_a_librime_peak):.1f} MB",
                    "same-run peer scale",
                ),
            ],
        },
        {
            "heading": "Named Yune owner diagnostics (owned default)",
            "rows": [
                (
                    "process unclassified lower bound",
                    mb_from_bytes(
                        owner_bytes("process.after_ready_working_set_unclassified_lower_bound")
                    ),
                    "owner",
                    f'{mb_from_bytes(owner_bytes("process.after_ready_working_set_unclassified_lower_bound")):.1f} MB',
                    "remaining process floor",
                ),
                (
                    "poet.vocabulary",
                    mb_from_bytes(owner_bytes("poet.vocabulary")),
                    "owner",
                    f'{mb_from_bytes(owner_bytes("poet.vocabulary")):.1f} MB',
                    "heap-owned; mmap-backed under the opt-in",
                ),
                (
                    "poet.entries_by_code",
                    mb_from_bytes(owner_bytes("poet.entries_by_code")),
                    "owner",
                    f'{mb_from_bytes(owner_bytes("poet.entries_by_code")):.1f} MB',
                    "heap-owned; mmap-backed under the opt-in",
                ),
                (
                    "compact_table.storage",
                    mb_from_bytes(owner_bytes("compact_table.storage")),
                    "after",
                    f'{mb_from_bytes(owner_bytes("compact_table.storage")):.1f} MB',
                    "mmap file-backed",
                ),
            ],
        },
    ],
    axis_max=200.0,
    gridlines=[0, 50, 100, 150, 200],
    axis_fmt=lambda v: f"{int(v)} MB",
    refs=[(195.028378, "corrective ceiling", True, False)],
)

track_b_key = find_engine_summary(
    "yune",
    "track-b-product",
    "key_sequence_process_with_context",
    "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung",
)
track_b_session = find_engine_summary("yune", "track-b-product", "session_create_select_destroy")
track_b_startup = find_engine_summary(
    "yune", "track-b-product", "startup_warm_shared_assets_runtime_ready"
)

svg_chart(
    "native-track-b-memory.svg",
    "Native Track B memory: M55 product guard",
    [
        "M55 corrective gate run D. TypeDuck jyut6ping3_mobile is a product guard lane, not a same-run librime comparison.",
        "The process peak includes deploy/compile transients; median rows are the steady benchmark observations.",
    ],
    [
        {
            "rows": [
                (
                    "process peak working set",
                    mb_from_bytes(fnum(track_b_key, "max_peak_working_set_bytes")),
                    "warn",
                    f'{mb_from_bytes(fnum(track_b_key, "max_peak_working_set_bytes")):.1f} MB',
                    "below 564.1 MB guard",
                ),
                (
                    "key-sequence median working set",
                    mb_from_bytes(fnum(track_b_key, "median_working_set_bytes")),
                    "after",
                    f'{mb_from_bytes(fnum(track_b_key, "median_working_set_bytes")):.1f} MB',
                    "below 280.5 MB guard",
                ),
                (
                    "key-sequence private bytes",
                    mb_from_bytes(fnum(track_b_key, "median_private_bytes")),
                    "after",
                    f'{mb_from_bytes(fnum(track_b_key, "median_private_bytes")):.1f} MB',
                    "below 200.6 MB guard",
                ),
                (
                    "session private bytes",
                    mb_from_bytes(fnum(track_b_session, "median_private_bytes")),
                    "after",
                    f'{mb_from_bytes(fnum(track_b_session, "median_private_bytes")):.1f} MB',
                    "below 195.1 MB guard",
                ),
            ]
        }
    ],
    axis_max=565.0,
    gridlines=[0, 140, 280, 420, 560],
    axis_fmt=lambda v: f"{int(v)} MB",
)

svg_chart(
    "native-track-b-latency.svg",
    "Native Track B latency: M55 product guard",
    [
        "M55 corrective gate run D. TypeDuck jyut6ping3_mobile latency rows are absolute product guardrails.",
        "No TypeDuck-vs-librime speed claim follows from this lane.",
    ],
    [
        {
            "rows": [
                (
                    "key-sequence latency",
                    fnum(track_b_key, "median_us"),
                    "after",
                    f'{float(track_b_key["median_us"]):,.1f} us',
                    "below 375.3 us guard",
                ),
                (
                    "session create/select/destroy",
                    fnum(track_b_session, "median_us"),
                    "after",
                    f'{float(track_b_session["median_us"]):,.1f} us',
                    "below 109.8k us guard",
                ),
                (
                    "startup warm runtime-ready",
                    fnum(track_b_startup, "median_us"),
                    "after",
                    f'{float(track_b_startup["median_us"]):,.1f} us',
                    "below 107.1k us guard",
                ),
            ]
        }
    ],
    axis_max=110000.0,
    gridlines=[0, 25000, 50000, 75000, 100000],
    axis_fmt=lambda v: f"{int(v/1000)}k us",
)

n_ratio = fnum(find_summary("track-a-comparison", "key_sequence_process_with_context", "n"), "yune_librime_median_ratio")
memory_ratio = track_a_yune_peak / track_a_librime_peak

svg_chart(
    "root-cause-gaps.svg",
    "Current performance gaps by lane (Yune / peer ratio)",
    [
        "Native Track A refreshed by the M55 corrective gate run D (per-keypress reads), 2026-07-04; browser rows carried from 2026-06-28/29.",
        "Lower is better. Product guard rows are omitted when there is no same-run librime peer.",
    ],
    [
        {
            "rows": [
                (
                    "Native Track A peak memory",
                    memory_ratio,
                    "amber",
                    f"{memory_ratio:.2f}x",
                    f"{mb_from_bytes(track_a_yune_peak):.1f} vs {mb_from_bytes(track_a_librime_peak):.1f} MB",
                ),
                (
                    "Browser Luna memory",
                    4.0,
                    "warn",
                    "4.00x",
                    "64.0 vs 16.0 MiB; blocker",
                ),
                (
                    "Native n latency",
                    n_ratio,
                    "amber",
                    f"{n_ratio:.3f}x",
                    "bounded-gap; not match-or-beat",
                ),
                (
                    "Browser Luna startup",
                    1.577,
                    "amber",
                    "1.58x",
                    "1000 vs 634 ms; watch",
                ),
            ]
        }
    ],
    axis_max=7.0,
    gridlines=[0, 1, 2, 3, 4, 5, 6, 7],
    axis_fmt=lambda v: f"{int(v)}x",
    refs=[(1.0, "1x parity", True, True)],
)

print("done")
