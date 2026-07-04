# M55 Phase 3R-2 DP State Vector Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, candidate output is unchanged, and the standing M52 gate stays
green. Tier M is still not met.

## Change

`collect_sentence_states` now keeps per-end DP state lists in a byte-indexed
vector during one sentence rebuild, then converts the non-empty entries back to
the existing `BTreeMap` return shape. This removes hot-loop map
remove/insert/lookup work without changing scoring, beam width, ordering,
weight arithmetic, candidate formatting, or public ABI.

## Access-Volume Delta

Command:

```powershell
$env:YUNE_M55_PHASE3R_VOLUME_CSV='..\..\docs\reports\evidence\m55-native-match-or-beat\phase-3r-dp-vector\access-volume.csv'
cargo test -p yune-core --test upstream_luna_pinyin_parity capture_phase3r_access_volume_csv -- --ignored --nocapture
Remove-Item Env:YUNE_M55_PHASE3R_VOLUME_CSV
```

Compared with the previous Phase 3R-2 checkpoint:

| input | storage | previous rebuild ns | this checkpoint rebuild ns | DP states created | beam evictions |
| --- | --- | ---: | ---: | ---: | ---: |
| 37-char | owned | 145,000 | 125,000 | 5,060 | 1,700 |
| 37-char | byte-backed | 220,200 | 201,000 | 5,060 | 1,700 |
| 59-char | owned | 119,600 | 108,200 | 10,853 | 4,494 |
| 59-char | byte-backed | 284,200 | 270,100 | 10,853 | 4,494 |

The state and graph-entry counts are unchanged. The owner reduced here is
container overhead while propagating the existing DP states.

## M52 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-dp-vector\m52-ratchet -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv -FailOnRegression
```

Result: green.

| row | observed | ceiling |
| --- | ---: | ---: |
| `n` | `2.809x` | `3.050x` |
| `ni` | `3.086x` | `3.223x` |
| `hao` | `2.118x` | `2.287x` |
| 37-char Luna | `2.337x` | `3.267x` |
| 59-char Luna | `1.811x` | `2.447x` |
| Track A peak | `185,511,936 B` | `198,000,000 B` |

## Product Path

Command shape:

```powershell
target\release\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists and matching trailing-space
commit text against the previous Phase 3R-2 product-path checkpoint.

The release CLI was used for this evidence because the debug CLI full app-schema
frontend command exceeded local timeouts while deploying the large schema tree.
The release command exercises the same frontend ABI/product path.

## Remaining Work

Phase 3R-2 remains active. This checkpoint is a small default-owned DP
constant-factor reduction, but the long Luna rows remain above the `<=1.50x`
Tier M bar. Byte-backed poet consumption stays default-off until the full M55
ratchet is green twice with byte-backing enabled and Track A memory remains at
or below the evidence-revised `125 MB` bar.
