# Cross-platform dashboard visuals (2026-07-05)

Windows standing gate vs the macOS post-M57 verification lane, preserved for
the consolidated [`performance dashboard`](../../yune-vs-librime-performance.md).

## Files

- `native-track-a-latency-windows-vs-macos.svg` / `.png` — Track A per-key
  latency ratio (Yune / same-run librime 1.17.0) for all ten dimensions, Windows
  (M55 corrective gate run D) beside macOS (M57 full-pass-1). The pass-1 to
  pass-2 spread is drawn as a whisker on the four `†` noise rows.
- `generate.py` — deterministic generator.

## What the numbers are

- **Windows** rows come from
  [`m55-native-match-or-beat/corrective-2026-07-04/gate-run-d/summary-comparison.csv`](../m55-native-match-or-beat/corrective-2026-07-04/gate-run-d/summary-comparison.csv)
  (the standing gate).
- **macOS** rows come from
  [`m57-macos-track-a-sentence-model-parity/full-pass-1/summary-comparison.csv`](../m57-macos-track-a-sentence-model-parity/full-pass-1/summary-comparison.csv),
  with `full-pass-2` giving the whisker range.

## Honest reading (baked into the chart)

- It is a **two-machine** comparison — a Windows x86 desktop vs an Apple Silicon
  MacBook Air, different CPU and compiler — so the ratio is machine-specific,
  not an OS effect.
- Yune's own absolute per-key latency is comparable across the two machines
  (37-char `572 us` Win vs `513 us` mac); the sentence-row ratio widens on macOS
  only because librime is faster there (37-char librime `299 us` Win vs
  `175 us` mac).
- The four `†` rows (`hao`, `ni`, `n`, `zhongguo`) have a noise-affected macOS
  ratio: `hao`/`ni`/`n` because librime's absolute is at the microbenchmark
  noise floor, `zhongguo` because librime's own median swings run-to-run
  (`111-264 us`). The whisker shows the pass-1 to pass-2 spread; treat those
  rows as directional only.

## Regenerating

Requires matplotlib (via uv):

```sh
uv venv .viz
uv pip install --python .viz/bin/python matplotlib
.viz/bin/python docs/reports/evidence/dashboard-visuals-2026-07-05-cross-platform/generate.py
```

The committed `.svg`/`.png` are the artifacts; consumers do not need matplotlib.
The single-platform Windows visuals remain in
[`../dashboard-visuals-2026-07-04/`](../dashboard-visuals-2026-07-04/) (a
dependency-free hand-SVG generator).
