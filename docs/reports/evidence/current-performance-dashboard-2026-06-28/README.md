# Current Performance Dashboard Evidence

Date: 2026-06-28

This bundle normalizes the current benchmark rows used by the three dashboard-style reports.

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

`luna_pinyin` is the fair cross-engine lane. Browser Jyutping rows are retained as guard evidence only because My RIME uses a Cantonese-only Jyutping dictionary while Yune ships TypeDuck's larger multilingual `jyut6ping3` profile.
