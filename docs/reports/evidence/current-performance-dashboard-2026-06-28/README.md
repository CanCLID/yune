# Current Performance Dashboard Evidence

Date: 2026-06-28

This bundle normalizes the current benchmark rows used by the three dashboard-style reports.

## Interaction Correction

Correction date: 2026-07-21

`WITHDRAWN_ENDPOINT_MISMATCH` applies only to the Luna first-candidate
`74 ms`/`95 ms` and commit `107 ms`/`119 ms` interpretations. Yune stopped on
the earlier candidate-bearing `n` diagnostic while My RIME waited for complete
`ni`; commit timing inherited those unequal starts. The normalized CSV and SVG
retain the historical numbers with an explicit overlay, and the source
comparator packet is unchanged. The ready-time, WASM, and resource-payload
observations are not withdrawn.

## Source Inputs

- Native Track A peer rows: `native-current-benchmark/summary.csv`, freshly captured for this dashboard pass with `-DeployProductBeforeBenchmark`.
- Browser peer rows: `apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/current-dashboard/summary.csv`, freshly captured for this dashboard pass.
- Browser input-latency suite: `apps/yune-web/e2e/results/web03-latency-regression-fix/local-browser-latency/samples.json`, the latest checked-in rebuilt public-demo WEB-03 latency bundle.

## Normalized Tables

- `current-native-track-a.csv`
- `current-browser-peer-comparator.csv`
- `current-yune-browser-input-latency.csv`
- `current-root-cause-gaps.csv`

## Visuals

- `visuals/current-native-latency-ratios.svg`
- `visuals/current-memory-peaks.svg`
- `visuals/current-browser-peer-latency.svg`
- `visuals/current-browser-memory-payload.svg`
- `visuals/current-yune-browser-input-latency.svg`
- `visuals/current-root-cause-gaps.svg`

## Comparison Notes

`luna_pinyin` remains comparable for the retained startup, WASM, and payload
observations. Its interaction endpoints are mismatched and withdrawn. Browser
Jyutping rows are retained as guard evidence only because My RIME uses a
Cantonese-only Jyutping dictionary while Yune ships TypeDuck's larger
multilingual `jyut6ping3` profile.
