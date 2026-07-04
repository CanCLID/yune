# M55 Phase 3R-2 Incremental Sentence Scratch Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, candidate output is unchanged, and the strict M55 ratchet is
green for this run. Tier M is still not met.

## Change

Bounded engine refresh now keeps per-translator scratch state for the upstream
sentence model. The scratch is used only for the null-grammar owned poet path
when the current input extends the previous sentence-model input with the same
candidate limit. It reuses prior DP states and builds only the new suffix edges,
then applies the existing scoring, beam width, ordering, and weight arithmetic.

The scratch is cleared or bypassed for unbounded translation, non-owned poet
storage, grammar-backed models such as Octagram, incompatible request shapes,
non-extending input, and translators that do not opt in. Public C ABI is
unchanged.

The strict M55 run initially exposed an existing Track B product guard problem:
the TypeDuck filter pipeline had drifted above the `375.253 us` ceiling even
before this scratch checkpoint. The owner was byte-backed
`dictionary_lookup_filter` comment construction. The final code adds a bounded
hot comment cache only for byte-backed lookup-filter records and records that
cache as a guarded heap owner when populated.

## Access Volume

Command:

```powershell
$env:YUNE_M55_PHASE3R_VOLUME_CSV='..\..\docs\reports\evidence\m55-native-match-or-beat\phase-3r-incremental-scratch\access-volume.csv'
cargo test -p yune-core --test upstream_luna_pinyin_parity capture_phase3r_access_volume_csv -- --ignored --nocapture
Remove-Item Env:YUNE_M55_PHASE3R_VOLUME_CSV
```

This capture is a direct final-input model run, so it does not show session
reuse. It confirms the final-keypress graph/state volume is still the same
shape as the previous DP-vector checkpoint: 37-character owned creates `5,060`
DP states, and 59-character owned creates `10,853` DP states.

The full benchmark M37 metrics show the new cross-keypress behavior:

| input | sentence calls | incremental reuse hits | first full rebuild chars |
| --- | ---: | ---: | ---: |
| 37-char Luna | 27 | 26 | 11 |
| 59-char Luna | 52 | 51 | 8 |

## Strict M55 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-incremental-scratch\m55-ratchet -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression
```

Result: green.

| row | observed | ceiling | status |
| --- | ---: | ---: | --- |
| `n` | `2.824x` | `3.050x` | pass |
| `ni` | `3.123x` | `3.223x` | pass |
| `hao` | `2.129x` | `2.287x` | pass |
| 37-char Luna | `2.249x` | `3.267x` | pass |
| 59-char Luna | `1.774x` | `2.447x` | pass |
| `zhongguo` | `0.280x` | `0.325x` | pass |
| `cszysmsrsd` | `0.398x` | `0.532x` | pass |
| `zybfshmsru` | `0.592x` | `0.770x` | pass |
| startup ready | `0.878x` | `1.101x` | pass |
| session create/select/destroy | `21,971.600 us` | `25,533.310 us` | pass |
| Track A peak | `186,105,856 B` | `198,000,000 B` | pass |
| Track B product key sequence | `309.382 us` | `375.253 us` | pass |

Track B filter-pipeline evidence from `m37_metrics.csv`:

| run | filter pipeline over 61-key operation | Track B median |
| --- | ---: | ---: |
| pre-cache failed run | `3,246,600 ns` | `383.772 us` |
| final green run | `393,400 ns` | `309.382 us` |

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

The release CLI was rebuilt before this evidence.

## Remaining Work

Phase 3R-2 remains active. This checkpoint reduces the long Luna rows from the
previous DP-vector checkpoint's `2.337x` / `1.811x` to `2.249x` / `1.774x`,
but both rows remain above the `<=1.50x` Tier M bar.

Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
