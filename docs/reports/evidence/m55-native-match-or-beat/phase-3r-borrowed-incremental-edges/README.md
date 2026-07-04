# M55 Phase 3R Borrowed Incremental Edge Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The owned/default path remains
byte-backed-off, candidate output is unchanged, and the strict M55 ratchet is
green for this run. Tier M is still not met.

## Change

The incremental owned sentence graph extension now stores temporary graph edges
as borrowed text references. Table and vocabulary text is cloned only when a
beam-accepted `PathState` is materialized, instead of cloning every temporary
`WordGraphEntry` before DP decides whether that edge contributes to the kept
states.

The full rebuild path is unchanged. This is an incremental graph materialization
reduction only. It does not change scoring, beam width, candidate ordering,
entry weights, public ABI, byte-backed poet default-off status, or output bytes.

## Strict M55 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-borrowed-incremental-edges\m55-ratchet -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression
```

Result: green.

| row | observed | ceiling | status |
| --- | ---: | ---: | --- |
| `n` | `2.765x` | `3.050x` | pass |
| `ni` | `2.986x` | `3.223x` | pass |
| `hao` | `2.076x` | `2.287x` | pass |
| 37-char Luna | `2.173x` | `3.267x` | pass |
| 59-char Luna | `1.728x` | `2.447x` | pass |
| `zhongguo` | `0.274x` | `0.325x` | pass |
| `cszysmsrsd` | `0.411x` | `0.532x` | pass |
| `zybfshmsru` | `0.590x` | `0.770x` | pass |
| startup ready | `0.974x` | `1.101x` | pass |
| session create/select/destroy | `22,766.000 us` | `25,533.310 us` | pass |
| Track A peak | `185,991,168 B` | `198,000,000 B` | pass |
| Track B product key sequence | `310.318 us` | `375.253 us` | pass |

Compared with the previous incremental vocabulary new-edge checkpoint, the
logical graph volume is unchanged for the long rows: `401` / `667` graph edges,
`233` / `353` exact table entries considered, and `168` / `314` vocabulary
entries considered. The new checkpoint avoids temporary `String` clones for
those graph edges before DP acceptance. The same-run headline ratios remain
strict-green but should not be treated as a new best: the previous checkpoint
reported `2.119x` / `1.645x` for the long Luna rows.

## Product Path

Command shape:

```powershell
target\release\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists and matching trailing-space
commit text against the previous incremental vocabulary new-edge checkpoint.

The release CLI was rebuilt before this evidence.

## Remaining Work

Phase 3R-2 remains active. This checkpoint removes one avoidable temporary
materialization point, but both long rows remain above the `<=1.50x` Tier M bar.

Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
