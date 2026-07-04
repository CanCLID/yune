# M55 Phase 3R-2 Owned Exact New-Span Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, candidate output is unchanged, and the strict M55 ratchet is
green for this run. Tier M is still not met.

## Change

The incremental owned sentence graph extension now materializes exact table
entries only for spans whose end extends beyond the previous input. Old spans
still participate in reachability through retained sentence scratch state and
vocabulary paths, but their exact table rows are not re-materialized into the
temporary graph for every new keypress.

This is a graph-volume reduction only. It does not change scoring, beam width,
candidate ordering, entry weights, public ABI, byte-backed poet default-off
status, or output bytes.

## Strict M55 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-exact-new-span\m55-ratchet -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression
```

Result: green.

| row | observed | ceiling | status |
| --- | ---: | ---: | --- |
| `n` | `2.833x` | `3.050x` | pass |
| `ni` | `3.107x` | `3.223x` | pass |
| `hao` | `2.119x` | `2.287x` | pass |
| 37-char Luna | `2.214x` | `3.267x` | pass |
| 59-char Luna | `1.708x` | `2.447x` | pass |
| `zhongguo` | `0.274x` | `0.325x` | pass |
| `cszysmsrsd` | `0.394x` | `0.532x` | pass |
| `zybfshmsru` | `0.568x` | `0.770x` | pass |
| startup ready | `1.042x` | `1.101x` | pass |
| session create/select/destroy | `22,798.400 us` | `25,533.310 us` | pass |
| Track A peak | `185,675,776 B` | `198,000,000 B` | pass |
| Track B product key sequence | `331.133 us` | `375.253 us` | pass |

Compared with the previous index-range checkpoint, the long Luna rows move from
`2.223x` / `1.730x` to `2.214x` / `1.708x`. Track B remains green but is slower
than the previous `316.844 us` run, so this checkpoint does not tighten the
Track B guard.

The long-row access-volume counters show exact table entries considered drop
from `4125` to `233` for the 37-char row and from `11023` to `353` for the
59-char row. Graph edges stay at `401` and `667`; vocabulary entries considered
stay at `3950` and `11673`.

## Product Path

Command shape:

```powershell
target\release\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists and matching trailing-space
commit text against the previous index-range checkpoint.

The release CLI was rebuilt before this evidence.

## Remaining Work

Phase 3R-2 remains active. This checkpoint is a small same-run owned-path
reduction, but both long rows remain above the `<=1.50x` Tier M bar.

Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
