# M55 ratchet gate — tone-merge fix (2026-07-09)

Command per run (the standing 17-input flip-gate invocation):
`scripts/benchmark-native-rime-inprocess.ps1 -YuneDll target/release/yune_rime_api.dll
-DeployProductBeforeBenchmark -TrackAInputs <17 inputs> -TrackAThresholds
docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`.
Quiet machine; median-of-runs verdict per the flip-gate protocol. Per-run
`track-a-*`/`track-b-*` deploy artifacts and heavy raw CSVs are not committed (flip
precedent); each run keeps `threshold-check.csv`, `summary*.csv`, `m37_metrics.csv`,
`commands.txt`, `environment.txt`.

## Runs 1-3 — first implementation (detector scanned the whole pool)

`n` 3.039/2.985/2.966 (median 2.985 ≤ 3.006 PASS, run-1 was noise) but **`ni`
2.721/2.695/2.694 — median 2.695 > 2.666 FAIL, consistent across runs**: the detector's
post-pooling scan walked the completion-heavy full list on every full-list
materialization, a real ~2% cost on the hottest short-key row (flip-era ni median was
2.631). All other rows passed. Response: optimize, not re-baseline — the owner-signed
ceilings were not touched.

## Optimization

The detector now walks only recorded exact-block ranges (`exact_scan_ranges`): exact rows
sit at the head of each spec's pending block in construction order, so the scan skips
completion tails entirely. Under `prediction_candidate_limit` the block boundary blurs
after the per-spec sort, so the whole spec range is recorded in that (page-turn-only)
mode. Semantics unchanged — true-exact rows cannot exist outside the recorded ranges.

## Runs 4-6 — shipped implementation: MEDIAN GATE ALL PASS

| row | runs | median | ceiling |
|---|---|---|---|
| n | 2.853 / 2.819 / 2.734 | **2.819** | 3.006 |
| ni | 2.595 / 2.521 / 2.544 | **2.544** | 2.666 |
| hao | 1.717 / 1.711 / 1.711 | **1.711** | 1.844 |
| 37-char | 2.141 / 2.116 / 2.069 | **2.116** | 2.339 |
| 59-char | 1.682 / 1.588 / 1.588 | **1.588** | 1.748 |
| zhongguo (win) | 0.292 / 0.291 / 0.285 | **0.291** | 0.323 |
| cszysmsrsd (win) | 0.397 / 0.387 / 0.392 | **0.392** | 0.474 |
| zybfshmsru (win) | 0.572 / 0.557 / 0.584 | **0.572** | 0.695 |
| startup ratio | 1.065 / 0.782 / 0.954 | **0.954** | 1.091 |

Every Track A memory row and every Track B latency/memory row also passes in all three
runs (see per-run `threshold-check.csv`). `ni`'s median (2.544) ends *below* its flip-era
median (2.631) — the range-scan is cheaper than the pre-fix whole-pool state.
