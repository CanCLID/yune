# M55 Phase 3R-2 Owned Beam-Reject Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, preserves candidate output, and keeps the standing M52 gate
green. Tier M is not met yet.

## Change

`collect_sentence_states` now rejects a candidate before cloning path state
when the destination beam is already full and the candidate's weight is
strictly below the current worst retained state for that end position. This
does not change scoring, ordering, beam width, weight arithmetic, or candidate
formatting. It skips only states that the existing beam rule would discard.

The implementation also avoids cloning each full source-state vector while
processing a graph start: the vector is temporarily removed from the state map
and reinserted after processing. Graph edges always advance to a later end, so
this keeps the same state set.

## Access-Volume Delta

Command:

```powershell
$env:YUNE_M55_PHASE3R_VOLUME_CSV='..\..\docs\reports\evidence\m55-native-match-or-beat\phase-3r-owned-beam-reject\access-volume.csv'
cargo test -p yune-core --test upstream_luna_pinyin_parity capture_phase3r_access_volume_csv -- --ignored --nocapture
Remove-Item Env:YUNE_M55_PHASE3R_VOLUME_CSV
```

Compared with Phase 3R-1:

| input | metric | Phase 3R-1 | Phase 3R-2 |
| --- | --- | ---: | ---: |
| 37-char | graph entries inserted | 96 | 96 |
| 37-char | DP states created | 21,418 | 5,060 |
| 37-char | beam evictions | 18,058 | 1,700 |
| 59-char | graph entries inserted | 164 | 164 |
| 59-char | DP states created | 40,645 | 10,853 |
| 59-char | beam evictions | 34,286 | 4,494 |

The graph remains near the Phase 3R-1 span minimum. The reduction is in wasted
DP state construction and beam churn.

## M52 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv -FailOnRegression -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-owned-beam-reject\m52-ratchet
```

Result: green.

| row | observed | ceiling |
| --- | ---: | ---: |
| `n` | `2.894x` | `3.050x` |
| `ni` | `3.180x` | `3.223x` |
| `hao` | `2.171x` | `2.287x` |
| 37-char Luna | `2.407x` | `3.267x` |
| 59-char Luna | `1.827x` | `2.447x` |
| Track A peak | `185,860,096 B` | `198,000,000 B` |

## Product Path

Command shape:

```powershell
target\debug\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists against the prior
committed current-code product-path evidence, plus matching trailing-space
commit text.

## Remaining Work

Phase 3R-2 remains active. The 59-char row is closer to Tier M, but 37-char
`2.407x` and 59-char `1.827x` are still above the `<=1.50x` Tier M bar.
Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
