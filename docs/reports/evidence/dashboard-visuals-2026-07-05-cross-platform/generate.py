#!/usr/bin/env python3
"""Cross-platform Yune-vs-librime Track A latency visual (2026-07-05).

Windows standing gate (M55 corrective gate run D) vs the macOS post-M57
verification lane (m57 full-pass-1, with the pass-1<->pass-2 spread drawn as a
whisker on the librime-noise rows). Two different machines (Windows x86 desktop
vs Apple Silicon MacBook Air), so the ratio is machine-specific, not an OS
effect.

Requires matplotlib. Install with uv:
  uv venv .viz && uv pip install --python .viz/bin/python matplotlib
  .viz/bin/python docs/reports/evidence/dashboard-visuals-2026-07-05-cross-platform/generate.py
The committed .svg/.png are the artifacts; consumers do not need matplotlib.
"""
from __future__ import annotations

import csv
from pathlib import Path

import matplotlib
matplotlib.use("svg")
import matplotlib.pyplot as plt
from matplotlib import font_manager

OUT = Path(__file__).resolve().parent
ROOT = OUT.parents[3]
WIN = ROOT / "docs/reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/gate-run-d/summary-comparison.csv"
MAC1 = ROOT / "docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-1/summary-comparison.csv"
MAC2 = ROOT / "docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-2/summary-comparison.csv"

# a CJK-free chart; use the default sans stack
plt.rcParams.update({
    "font.family": "sans-serif",
    "font.sans-serif": ["Helvetica Neue", "Arial", "DejaVu Sans"],
    "svg.fonttype": "none",
    "axes.edgecolor": "#c3c2b7",
})

WIN_C, MAC_C = "#2a78d6", "#eb6834"


def load(path: Path):
    rows = {}
    for r in csv.DictReader(path.open(encoding="utf-8-sig")):
        rows[r["input"] or r["workload"]] = r
    return rows


win, mac1, mac2 = load(WIN), load(MAC1), load(MAC2)

# (key, label, noise) ordered by Windows ratio ascending (best win at top)
DIMS = [
    ("zhongguo", "zhongguo (common word) †", True),
    ("cszysmsrsd", "cszysmsrsd (10-char abbr)", False),
    ("zybfshmsru", "zybfshmsru (8-char abbr)", False),
    ("session_create_select_destroy", "session lifecycle", False),
    ("startup_warm_shared_assets_runtime_ready", "startup (runtime-ready)", False),
    ("zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong", "59-char pinyin", False),
    ("hao", "hao (short key) †", True),
    ("ceshiyixiachangjushuruxingnengzenyang", "37-char pinyin", False),
    ("ni", "ni (short key) †", True),
    ("n", "n (short key) †", True),
]


def rat(rows, k):
    return float(rows[k]["yune_librime_median_ratio"])


labels = [d[1] for d in DIMS]
win_r = [rat(win, d[0]) for d in DIMS]
mac_r = [rat(mac1, d[0]) for d in DIMS]
mac_lo = [min(rat(mac1, d[0]), rat(mac2, d[0])) for d in DIMS]
mac_hi = [max(rat(mac1, d[0]), rat(mac2, d[0])) for d in DIMS]
noise = [d[2] for d in DIMS]

n = len(DIMS)
y = list(range(n))
h = 0.38

fig, ax = plt.subplots(figsize=(11, 6.6))
ax.barh([v + h / 2 for v in y], win_r, height=h, color=WIN_C, label="Windows (gate run D)", zorder=3)
ax.barh([v - h / 2 for v in y], mac_r, height=h, color=MAC_C, label="macOS post-M57 (full-pass-1)", zorder=3)

# noise whisker: pass-1<->pass-2 spread on the macOS bar
for i in range(n):
    if noise[i]:
        yy = y[i] - h / 2
        ax.plot([mac_lo[i], mac_hi[i]], [yy, yy], color="#7a2f16", lw=1.6, zorder=4)
        for xv in (mac_lo[i], mac_hi[i]):
            ax.plot([xv, xv], [yy - 0.08, yy + 0.08], color="#7a2f16", lw=1.6, zorder=4)

# value labels
for i in range(n):
    ax.text(win_r[i] + 0.05, y[i] + h / 2, f"{win_r[i]:.2f}x", va="center", ha="left", fontsize=8.5, color="#0b0b0b")
    hi = mac_hi[i] if noise[i] else mac_r[i]
    ax.text(hi + 0.05, y[i] - h / 2, f"{mac_r[i]:.2f}x", va="center", ha="left", fontsize=8.5, color="#0b0b0b")

ax.axvline(1.0, color="#0b0b0b", lw=1.3, zorder=2)
ax.text(1.0, n - 0.35, " 1x parity", va="bottom", ha="left", fontsize=9, color="#0b0b0b")
ax.axvspan(0, 1.0, color="#1baf7a", alpha=0.06, zorder=0)  # "faster than librime" band

ax.set_yticks(y)
ax.set_yticklabels(labels, fontsize=9.5)
ax.set_xlim(0, 4.7)
ax.set_xlabel("Yune median / same-run librime 1.17.0 median  (lower is better; < 1x = faster than librime)", fontsize=9.5)
ax.set_xticks([0, 1, 2, 3, 4])
ax.set_xticklabels(["0", "1x", "2x", "3x", "4x"], fontsize=9)
ax.grid(axis="x", color="#e1e0d9", lw=0.8, zorder=0)
for s in ("top", "right", "left"):
    ax.spines[s].set_visible(False)
ax.tick_params(length=0)
ax.invert_yaxis()

fig.text(0.055, 0.955, "Native Track A latency ratio: Windows vs macOS  (Yune / librime 1.17.0)", fontsize=14, fontweight="bold", color="#0b0b0b")
fig.text(0.055, 0.923, "Per keypress with context read. Two different machines (Windows x86 desktop vs Apple Silicon MacBook Air), so the ratio is machine-specific, not an OS effect.", fontsize=9, color="#52514e")
fig.text(0.055, 0.902, "Dagger rows: macOS ratio noise-affected (whisker = pass-1 to pass-2). hao/ni/n librime sits at the microbenchmark noise floor; zhongguo librime swings run-to-run (111-264 us).", fontsize=9, color="#52514e")
ax.legend(loc="lower right", frameon=False, fontsize=9.5)

fig.subplots_adjust(left=0.24, right=0.97, top=0.86, bottom=0.1)
fig.savefig(OUT / "native-track-a-latency-windows-vs-macos.svg")
fig.savefig(OUT / "native-track-a-latency-windows-vs-macos.png", dpi=150)
print("wrote native-track-a-latency-windows-vs-macos.svg + .png")
