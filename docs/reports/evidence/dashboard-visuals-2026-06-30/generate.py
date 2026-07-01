#!/usr/bin/env python3
"""Generate the current Yune-vs-librime dashboard visuals (2026-06-30).

Deterministic, dependency-free SVG generator. Numbers are transcribed from the
checked-in evidence CSVs:
- Native Track A: m52-track-a-guardrails-and-disposition/final-native-benchmark/
  summary-comparison.csv + memory-owner-profile.csv (M52 final, 2026-06-30).
- Native Track B: current-performance-dashboard-2026-06-29/current-native-track-b.csv
  (carried; M52 ran -SkipTrackB).
- Root-cause gaps: refreshed to M52 native ratios; browser ratios carried.

Re-run: python docs/reports/evidence/dashboard-visuals-2026-06-30/generate.py
Browser visuals are carried unchanged from
current-performance-dashboard-2026-06-29/visuals/.
"""
import os

OUT = os.path.dirname(os.path.abspath(__file__))

W = 1040
LABEL_RIGHT = 258
PLOT_LEFT = 270
PLOT_RIGHT = 720
PLOT_W = PLOT_RIGHT - PLOT_LEFT
FIRST_TOP = 79
PITCH = 34
BAR_H = 18
FONT = "Segoe UI, Arial, sans-serif"

COLORS = {
    "pass":   ("#A3D576", "#386411"),  # green: at/under target
    "warn":   ("#F0986E", "#804126"),  # orange: over strict gate / concern
    "amber":  ("#F2C879", "#9C7B1E"),  # amber: watch
    "peer":   ("#BFC6D4", "#868EA0"),  # grey: reference / peer
    "after":  ("#7FB1D6", "#355E80"),  # blue: improved current value
    "owner":  ("#C7B3E0", "#5E4B86"),  # purple: owner breakdown
}


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def svg_chart(filename, title, sub_lines, sections, axis_max, gridlines,
              axis_fmt, refs=None):
    """sections: list of {heading?, rows:[(label, value, color, primary, secondary)]}.
    sub_lines: list of subtitle lines (each kept short enough for the canvas).
    All sections share one numeric axis."""
    def x_of(v):
        return PLOT_LEFT + (v / axis_max) * PLOT_W

    top_start = 54 + 16 * len(sub_lines) + 10
    y = top_start
    plot_top = top_start - 1
    body = []
    for si, sec in enumerate(sections):
        if sec.get("heading"):
            body.append(
                f'  <text x="28" y="{y+13}" font-family="{FONT}" font-size="11" '
                f'font-weight="700" fill="#1F2430">{esc(sec["heading"])}</text>'
            )
            y += 24
        for row in sec["rows"]:
            label, value, color, primary, secondary = row
            fill, stroke = COLORS[color]
            bw = max(2.0, (value / axis_max) * PLOT_W)
            baseline = y + 14
            body.append(
                f'  <text x="{LABEL_RIGHT}" y="{baseline}" font-family="{FONT}" '
                f'font-size="11" font-weight="500" fill="#1F2430" '
                f'text-anchor="end">{esc(label)}</text>'
            )
            body.append(
                f'  <rect x="{PLOT_LEFT}" y="{y}" width="{bw:.1f}" height="{BAR_H}" '
                f'rx="2" fill="{fill}" stroke="{stroke}" stroke-width="1"/>'
            )
            body.append(
                f'  <text x="{PLOT_LEFT+bw+6:.1f}" y="{baseline}" font-family="{FONT}" '
                f'font-size="11" font-weight="700" fill="#1F2430">{esc(primary)}</text>'
            )
            if secondary:
                off = PLOT_LEFT + bw + 6 + 8 + 7.0 * len(primary)
                body.append(
                    f'  <text x="{off:.1f}" y="{baseline}" font-family="{FONT}" '
                    f'font-size="10" fill="#6F768A">{esc(secondary)}</text>'
                )
            y += PITCH
        if si != len(sections) - 1:
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
    for (rv, rlabel, dashed, emph) in (refs or []):
        gx = x_of(rv)
        dash = ' stroke-dasharray="4 4"' if dashed else ''
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
    with open(os.path.join(OUT, filename), "w", encoding="utf-8", newline="\n") as f:
        f.write(out)
    print(f"wrote {filename} ({height}px)")


# ------------------------------------------------------------ Track A latency
svg_chart(
    "native-track-a-latency-ratios.svg",
    "Native Track A latency across all input dimensions: Yune vs librime 1.17.0",
    ["Bar = Yune median / same-run upstream librime median; lower is better. Left of the 1x line means Yune is faster.",
     "Green = Yune faster, coral = Yune slower; the us figure is the absolute per-call gap (the honest magnitude).",
     "Per-key rows sorted fastest-first. Short/pinyin rows from M52; zhongguo + abbreviation rows carried from the 2026-06-29 suite."],
    [
        {"heading": "Per-key input latency", "rows": [
            ("zhongguo (common word)", 0.277, "pass", "0.28x", "Yune -120 us"),
            ("cszysmsrsd (10-char abbr)", 0.425, "pass", "0.43x", "Yune -701 us"),
            ("zybfshmsru (8-char abbr)", 0.637, "pass", "0.64x", "Yune -312 us"),
            ("hao (short key)", 2.146, "warn", "2.15x", "Yune +13 us"),
            ("59-char pinyin", 2.247, "warn", "2.25x", "Yune +858 us"),
            ("n (short key)", 2.818, "warn", "2.82x", "Yune +39 us"),
            ("37-char pinyin", 3.053, "warn", "3.05x", "Yune +602 us; ceiling"),
            ("ni (short key)", 3.143, "warn", "3.14x", "Yune +31 us; ceiling"),
        ]},
        {"heading": "One-time lifecycle (near parity, run-noisy)", "rows": [
            ("startup warm runtime-ready", 1.113, "peer", "1.11x", "M52 run; 0.80x on the 2026-06-29 run"),
            ("session create/select/destroy", 1.001, "peer", "1.00x", "parity"),
        ]},
    ],
    axis_max=3.6,
    gridlines=[0.0, 0.9, 1.8, 2.7, 3.6],
    axis_fmt=lambda v: f"{v:.1f}x",
    refs=[(1.0, "1x parity", False, True), (3.0, "3x gate", True, False)],
)

# ------------------------------------------------------------ Track A memory
svg_chart(
    "native-track-a-memory.svg",
    "Native Track A memory: peak high-water and named owners",
    ["M52 final, 2026-06-30. luna_pinyin comparison lane, peak working set in MB.",
     "Top: Yune peak vs same-run librime peer. Bottom: named Yune owners that make up the 188.4 MB peak."],
    [
        {"heading": "Peak working set, same-run lanes", "rows": [
            ("Yune Track A peak", 188.4, "warn", "188.4 MB", "comparison lane high-water"),
            ("librime max peer peak", 17.3, "peer", "17.3 MB", "same-run peer scale"),
        ]},
        {"heading": "Named Yune owners (subset of the 188.4 MB)", "rows": [
            ("process unclassified lower bound", 105.6, "owner", "105.6 MB", ""),
            ("poet.vocabulary", 53.6, "owner", "53.6 MB", "full upstream Luna preset vocab"),
            ("poet.entries_by_code", 18.7, "owner", "18.7 MB", "sentence-model entries"),
            ("poet.lookup_index", 2.7, "owner", "2.7 MB", "M40 index"),
        ]},
    ],
    axis_max=200.0,
    gridlines=[0, 50, 100, 150, 200],
    axis_fmt=lambda v: f"{int(v)} MB",
)

# ------------------------------------------------------------ Track B memory
svg_chart(
    "native-track-b-memory.svg",
    "Native Track B memory: TypeDuck jyut6ping3 product path",
    ["Native TypeDuck jyut6ping3 product lane, working set / private in MB. No librime peer (TypeDuck is a fork).",
     "Current 2026-06-29 after M47 byte-backing; prior 2026-06-28 shown as 'was'."],
    [{"rows": [
        ("peak working set (process)", 504.4, "warn", "504.4 MB", "flat; deploy/compile transient"),
        ("median working set (key seq)", 255.0, "after", "255.0 MB", "was 440.1"),
        ("private bytes (key seq)", 183.9, "after", "183.9 MB", "was 420.0"),
        ("private bytes (session)", 178.8, "after", "178.8 MB", "was 405.8"),
    ]}],
    axis_max=560.0,
    gridlines=[0, 140, 280, 420, 560],
    axis_fmt=lambda v: f"{int(v)} MB",
)

# ------------------------------------------------------------ Track B latency
svg_chart(
    "native-track-b-latency.svg",
    "Native Track B lifecycle latency: TypeDuck jyut6ping3 product path",
    ["Native TypeDuck jyut6ping3 product lane, one-time lifecycle medians in us; M47 byte-backing cut both.",
     "Per-key processing is 336.49 us (was 341.67), essentially flat - too small to plot on this scale."],
    [{"rows": [
        ("startup warm runtime-ready", 90795.0, "after", "90,795 us", "was 124,505"),
        ("session create/select/destroy", 92093.1, "after", "92,093 us", "was 141,590"),
    ]}],
    axis_max=150000.0,
    gridlines=[0, 50000, 100000, 150000],
    axis_fmt=lambda v: f"{int(v/1000)}k us",
)

# ------------------------------------------------------------ Root-cause gaps
svg_chart(
    "root-cause-gaps.svg",
    "Current performance gaps by lane (Yune / peer ratio)",
    ["Native Track A refreshed by M52, 2026-06-30; browser rows carried from 2026-06-28.",
     "Lower is better; dashed line = 1.0x peer parity. Jyutping browser row is guard-only (different dictionary)."],
    [{"rows": [
        ("Native Track A peak memory", 10.90, "amber", "10.90x", "188.4 vs 17.3 MB; watch"),
        ("Browser Luna memory", 4.000, "warn", "4.00x", "64.0 vs 16.0 MiB; blocker"),
        ("Native 37-char latency", 3.053, "warn", "3.05x", "ceiling"),
        ("Native ni latency", 3.143, "warn", "3.14x", "ceiling"),
        ("Browser Jyutping memory (guard)", 2.353, "peer", "2.35x", "160 vs 68 MiB; diff dict"),
        ("Browser Luna startup", 1.577, "amber", "1.58x", "1000 vs 634 ms; watch"),
    ]}],
    axis_max=12.0,
    gridlines=[0, 3, 6, 9, 12],
    axis_fmt=lambda v: f"{int(v)}x",
    refs=[(1.0, "1x parity", True, True)],
)

print("done")
