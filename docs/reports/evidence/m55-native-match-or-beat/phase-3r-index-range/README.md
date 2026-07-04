# M55 Phase 3R-2 Owned Lookup Index Range Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, candidate output is unchanged, and the strict M55 ratchet is
green for this run. Tier M is still not met.

## Change

The owned sentence lookup walk now carries the exact entry range it already
found while scanning a code prefix. Graph construction consumes that range
directly instead of doing a second exact lookup for every emitted span.

This is an index access reduction only. It does not change scoring, beam width,
candidate ordering, entry weights, graph materialization rules, public ABI, or
byte-backed poet default-off status.

## Strict M55 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-index-range\m55-ratchet -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression
```

Result: green.

| row | observed | ceiling | status |
| --- | ---: | ---: | --- |
| `n` | `2.925x` | `3.050x` | pass |
| `ni` | `3.100x` | `3.223x` | pass |
| `hao` | `2.174x` | `2.287x` | pass |
| 37-char Luna | `2.223x` | `3.267x` | pass |
| 59-char Luna | `1.730x` | `2.447x` | pass |
| `zhongguo` | `0.282x` | `0.325x` | pass |
| `cszysmsrsd` | `0.399x` | `0.532x` | pass |
| `zybfshmsru` | `0.588x` | `0.770x` | pass |
| startup ready | `0.558x` | `1.101x` | pass |
| session create/select/destroy | `22,954.900 us` | `25,533.310 us` | pass |
| Track A peak | `185,905,152 B` | `198,000,000 B` | pass |
| Track B product key sequence | `316.844 us` | `375.253 us` | pass |

Compared with the previous incremental-scratch checkpoint, the long Luna rows
move from `2.249x` / `1.774x` to `2.223x` / `1.730x`. Track B remains green
but is slightly slower than the previous `309.382 us` run, so this checkpoint
does not tighten the Track B guard.

## Product Path

Command shape:

```powershell
target\release\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists and matching trailing-space
commit text against the previous incremental-scratch checkpoint.

The release CLI was rebuilt before this evidence.

## Remaining Work

Phase 3R-2 remains active. This checkpoint is a small same-run owned-path
reduction, but both long rows remain above the `<=1.50x` Tier M bar.

Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
