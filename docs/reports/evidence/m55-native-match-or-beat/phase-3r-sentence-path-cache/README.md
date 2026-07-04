# M55 Phase 3R Sentence-Path Cache Checkpoint

Date: 2026-07-04

Verdict: green checkpoint, not M55 closeout. The strict M55 ratchet is green,
product-path candidate bytes are unchanged, and the owned/default 59-character
row improves to near the Tier M bar. Tier M is still not met because the
37-character row and short keys remain above their target ratios.

## Change

The owned null-grammar incremental sentence scratch now retains per-end
`SentencePath` lists alongside retained DP `PathState` buckets. On single-key
growth, Yune converts only the newly reached end bucket to sentence paths and
then runs the existing candidate merge/rank step over the cached per-end lists.

This does not change scoring, beam width, candidate ordering, entry weights,
public ABI, byte-backed poet default-off status, or output bytes. Grammar-backed
Octagram scoring remains outside this scratch path.

## Strict M55 Ratchet

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-3r-sentence-path-cache\m55-ratchet -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv -FailOnRegression
```

Result: green.

| row | observed | ceiling | status |
| --- | ---: | ---: | --- |
| `n` | `2.667x` | `3.050x` | pass |
| `ni` | `2.979x` | `3.223x` | pass |
| `hao` | `2.026x` | `2.287x` | pass |
| 37-char Luna | `2.001x` | `3.267x` | pass |
| 59-char Luna | `1.519x` | `2.447x` | pass |
| `zhongguo` | `0.260x` | `0.325x` | pass |
| `cszysmsrsd` | `0.388x` | `0.532x` | pass |
| `zybfshmsru` | `0.570x` | `0.770x` | pass |
| startup ready | `0.916x` | `1.101x` | pass |
| session create/select/destroy | `21,958.700 us` | `25,533.310 us` | pass |
| Track A peak | `185,520,128 B` | `198,000,000 B` | pass |
| Track B product key sequence | `313.461 us` | `375.253 us` | pass |

Compared with the previous incremental prefix-state checkpoint, the long Luna
rows move from `2.010x` / `1.570x` to `2.001x` / `1.519x`. The win rows stay
below `1.00x`, Track B remains within its absolute guard, and Track A memory
stays in the default-owned M52-era shape.

## Product Path

Command shape:

```powershell
target\release\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Rows: 37-char benchmark, 59-char benchmark, `jianli`, `biancheng`, and
`zhongguo`.

Result: green. `product-path-candidate-parity-2026-07-04.json` records
byte-identical final composing-event candidate lists and matching trailing-space
commit text against the previous incremental prefix-state checkpoint.

The release CLI was rebuilt before this evidence.

## Access-Volume Follow-Up

The refreshed product-path diagnostic in
`../phase-3r-incremental-product-access-volume/access-volume.csv` shows the
sentence-path conversion owner dropped from `27,900 ns` / `57,800 ns` to
`2,400 ns` / `3,800 ns` for the 37-/59-character owned rows. Candidate merge
and ranking is now the remaining measured owned extractor cost at `62,900 ns` /
`127,300 ns`.

## Remaining Work

Phase 3R-2 remains active. This checkpoint removes most repeated sentence-path
conversion work during key-by-key long-row growth, but the 37-character row is
still above `<=1.50x`, the 59-character row is just above `<=1.50x`, and the
short-key Tier M targets are still unmet.

Byte-backed poet consumption stays default-off until the full M55 ratchet is
green twice with byte-backing enabled and Track A memory remains at or below
the evidence-revised `125 MB` bar.
