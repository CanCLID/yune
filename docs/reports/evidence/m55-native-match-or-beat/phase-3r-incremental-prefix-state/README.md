# M55 Phase 3R Incremental Prefix-State Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, candidate output is unchanged, and the strict M55 ratchet is
green for this run. Tier M is still not met.

## Change

The owned null-grammar incremental sentence scratch now carries per-start
prefix-walk state and exact first-code spans across key-by-key input growth.
For a single-character extension, graph construction advances each reachable
start through only the new suffix instead of replaying the complete
start-to-current-end prefix walk. Cached exact spans are still replayed for
preset-vocabulary phrase derivation, so sentence candidates stay byte-identical.

The full rebuild path is unchanged. This is an incremental prefix-walk
reduction only. It does not change scoring, beam width, candidate ordering,
entry weights, public ABI, byte-backed poet default-off status, or output bytes.

## Strict M55 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-incremental-prefix-state\m55-ratchet -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression
```

Result: green.

| row | observed | ceiling | status |
| --- | ---: | ---: | --- |
| `n` | `2.708x` | `3.050x` | pass |
| `ni` | `2.952x` | `3.223x` | pass |
| `hao` | `2.039x` | `2.287x` | pass |
| 37-char Luna | `2.010x` | `3.267x` | pass |
| 59-char Luna | `1.570x` | `2.447x` | pass |
| `zhongguo` | `0.274x` | `0.325x` | pass |
| `cszysmsrsd` | `0.401x` | `0.532x` | pass |
| `zybfshmsru` | `0.589x` | `0.770x` | pass |
| startup ready | `1.034x` | `1.101x` | pass |
| session create/select/destroy | `22,804.200 us` | `25,533.310 us` | pass |
| Track A peak | `186,085,376 B` | `198,000,000 B` | pass |
| Track B product key sequence | `324.395 us` | `375.253 us` | pass |

Compared with the previous borrowed incremental-edge checkpoint, the long Luna
rows move from `2.173x` / `1.728x` to `2.010x` / `1.570x`. Prefix checks on the
two long rows drop from `2170` / `5076` to `382` / `958`; exact first-code span
hits and final graph edges remain `821` / `1840` and `401` / `667`, preserving
the vocabulary derivation surface required for unchanged candidates.

## Product Path

Command shape:

```powershell
target\release\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists and matching trailing-space
commit text against the previous borrowed incremental-edge checkpoint.

The release CLI was rebuilt before this evidence.

## Remaining Work

Phase 3R-2 remains active. This checkpoint removes most repeated prefix-walk
work during key-by-key long-row growth, but both long rows remain above the
`<=1.50x` Tier M bar.

Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
