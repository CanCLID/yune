# M55 Phase 3R-2 Path-State Materialization Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, preserves candidate output, and keeps the standing M52 gate
green. Tier M is not met yet.

## Change

`collect_sentence_states` now builds accepted `PathState` values with exact
capacity for the output text instead of cloning the source text and extending
it. `PathState` word lengths now use a 16-slot inline buffer and spill to heap
only for unusually segmented paths.

This keeps scoring, ordering, beam width, weight arithmetic, and candidate
formatting unchanged. It is a constant-factor allocation/materialization
checkpoint on top of the Phase 3R-2 owned beam-reject checkpoint.

## Access-Volume Delta

Command:

```powershell
$env:YUNE_M55_PHASE3R_VOLUME_CSV='<workspace>\docs\reports\evidence\m55-native-match-or-beat\phase-3r-owned-inline-lengths\access-volume.csv'
cargo test -p yune-core --test upstream_luna_pinyin_parity capture_phase3r_access_volume_csv -- --ignored --nocapture
Remove-Item Env:YUNE_M55_PHASE3R_VOLUME_CSV
```

Compared with Phase 3R-1 and the first Phase 3R-2 checkpoint:

| input | metric | Phase 3R-1 | beam-reject checkpoint | this checkpoint |
| --- | --- | ---: | ---: | ---: |
| 37-char | graph entries inserted | 96 | 96 | 96 |
| 37-char | DP states created | 21,418 | 5,060 | 5,060 |
| 37-char | beam evictions | 18,058 | 1,700 | 1,700 |
| 59-char | graph entries inserted | 164 | 164 | 164 |
| 59-char | DP states created | 40,645 | 10,853 | 10,853 |
| 59-char | beam evictions | 34,286 | 4,494 | 4,494 |

The state count is unchanged from the beam-reject checkpoint. The owner reduced
here is per-state heap materialization, not graph/DP volume.

## M52 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv -FailOnRegression -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-owned-inline-lengths\m52-ratchet
```

Result: green.

| row | observed | ceiling |
| --- | ---: | ---: |
| `n` | `2.639x` | `3.050x` |
| `ni` | `2.881x` | `3.223x` |
| `hao` | `1.972x` | `2.287x` |
| 37-char Luna | `2.228x` | `3.267x` |
| 59-char Luna | `1.856x` | `2.447x` |
| Track A peak | `185,864,192 B` | `198,000,000 B` |

## Product Path

Command shape:

```powershell
target\debug\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists and matching trailing-space
commit text against the previous Phase 3R-2 product-path checkpoint.

## Remaining Work

Phase 3R-2 remains active. The checkpoint improves the 37-character row from
`2.407x` to `2.228x` in this same gate shape, but 37-character `2.228x` and
59-character `1.856x` are still above the `<=1.50x` Tier M bar.

Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
