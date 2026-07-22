# Current Performance Dashboard Evidence - 2026-06-29

This folder contains normalized CSVs and SVGs used by the current performance
and root-cause reports. Native rows were refreshed from the M50 final native
benchmark; browser rows are carried from the 2026-06-28 Playwright dashboard.

Fresh native source:
`../m50-track-a-launch-readiness/final-native-benchmark/`.

## Interaction Correction

Correction date: 2026-07-21

The carried Luna first-candidate `74 ms`/`95 ms` and commit
`107 ms`/`119 ms` interpretations now carry
`WITHDRAWN_ENDPOINT_MISMATCH`. Yune stopped on the earlier candidate-bearing
`n` diagnostic while My RIME waited for complete `ni`; commit timing inherited
those unequal starts. The normalized CSV and SVG retain the historical values
with the overlay, and the underlying 2026-06-28 comparator packet remains
unchanged. Ready time, WASM, and resource payload are not withdrawn.

## Files

- `current-native-track-a.csv` - refreshed `luna_pinyin` peer rows. Track A is
  still measured partial: `n` passes, while `ni`, the 37-character row, and
  peak memory remain blockers.
- `current-native-track-b.csv` - refreshed product-path guard rows for
  `jyut6ping3_mobile`; not a fair librime peer comparison.
- `current-root-cause-gaps.csv` - ranked remaining gaps after M50 final.
- `current-browser-peer-comparator.csv` - carried forward from 2026-06-28 with
  the dated interaction-only withdrawal overlay.
- `current-yune-browser-input-latency.csv` - carried forward from 2026-06-28.
- `visuals/current-native-latency-ratios.svg` - regenerated from M50 final.
- `visuals/current-memory-peaks.svg` - regenerated from M50 final native memory
  plus carried browser memory rows.
- `visuals/current-root-cause-gaps.svg` - regenerated from M50 final gap ranks.
- `visuals/current-browser-peer-latency.svg`,
  `visuals/current-browser-memory-payload.svg`, and
  `visuals/current-yune-browser-input-latency.svg` - carried forward from
  2026-06-28; the peer-latency SVG has the dated interaction withdrawal
  overlay.

## Scope Notes

Native Track A values are for upstream `luna_pinyin` against same-run upstream
librime `1.17.0`. They are not browser, product package, deployment, TypeDuck
keyboard-profile memory, or iOS-device claims.
