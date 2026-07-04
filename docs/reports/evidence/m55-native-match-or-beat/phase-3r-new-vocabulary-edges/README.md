# M55 Phase 3R-2 Incremental Vocabulary New-Edge Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, candidate output is unchanged, and the strict M55 ratchet is
green for this run. Tier M is still not met.

## Change

The incremental owned sentence graph extension now applies the previous-input
boundary to vocabulary phrase-code matching. Vocabulary entries whose matching
phrase codes only end at or before the previous input are skipped for new graph
edge derivation; retained scratch states already cover those old endings.

The full rebuild path is unchanged. This is an incremental graph-volume
reduction only. It does not change scoring, beam width, candidate ordering,
entry weights, public ABI, byte-backed poet default-off status, or output bytes.

## Strict M55 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-new-vocabulary-edges\m55-ratchet -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression
```

Result: green.

| row | observed | ceiling | status |
| --- | ---: | ---: | --- |
| `n` | `2.852x` | `3.050x` | pass |
| `ni` | `3.115x` | `3.223x` | pass |
| `hao` | `2.155x` | `2.287x` | pass |
| 37-char Luna | `2.119x` | `3.267x` | pass |
| 59-char Luna | `1.645x` | `2.447x` | pass |
| `zhongguo` | `0.271x` | `0.325x` | pass |
| `cszysmsrsd` | `0.401x` | `0.532x` | pass |
| `zybfshmsru` | `0.597x` | `0.770x` | pass |
| startup ready | `1.019x` | `1.101x` | pass |
| session create/select/destroy | `22,441.600 us` | `25,533.310 us` | pass |
| Track A peak | `185,622,528 B` | `198,000,000 B` | pass |
| Track B product key sequence | `316.616 us` | `375.253 us` | pass |

Compared with the previous exact-new-span checkpoint, the long Luna rows move
from `2.214x` / `1.708x` to `2.119x` / `1.645x`. Track B remains green and
moves from `331.133 us` to `316.616 us` in this same-run evidence.

The long-row access-volume counters show vocabulary entries considered drop
from `3950` to `168` for the 37-char row and from `11673` to `314` for the
59-char row. Graph edges stay at `401` and `667`; exact table entries stay at
`233` and `353`.

## Product Path

Command shape:

```powershell
target\release\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists and matching trailing-space
commit text against the previous exact-new-span checkpoint.

The release CLI was rebuilt before this evidence.

## Remaining Work

Phase 3R-2 remains active. This checkpoint is a meaningful same-run owned-path
reduction, but both long rows remain above the `<=1.50x` Tier M bar.

Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
