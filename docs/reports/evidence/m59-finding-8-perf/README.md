# M59 finding #8 — per-keystroke syllabary-scan memoization + ratchet straddle disposition

## The fix
`StaticTableTranslator::leading_syllable_fetch_codes` rescanned the whole
syllabary (~424 entries) and allocated a `String` per entry via
`normalized_original_code` on **every prefix boundary of every keystroke** — the
longest-first walk (finding #3) tries many empty prefixes before the first hit on
long inputs, so a 37/59-char row paid this repeatedly (~15–25k allocs/keystroke).
It now does an O(1) lookup into a `normalized_original_code(code) -> [storage
codes]` index built **once** at construction (`OnceLock`), iterating the same
source in the same order — the emitted fetch codes are byte-identical, so the
change is behavior-preserving (m59 4/4, reach 5/5, `upstream_luna_pinyin_parity`
14/14, `yune-core --lib` 306/0, bare-syllable guard intact).

## Disposition: the straddle is CLOSED — robust green across 3 fresh runs

Standing ceilings: `docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`
(**unchanged — no re-baseline**). Three fresh runs, all rows pass; nothing was
discarded (**no run-until-green**). `n` is blank in run-1 only because the default
`TrackAInputs` omits it; runs 2–3 add it and it passes.

Track A latency ratios (Yune ÷ librime median; lower is better):

| row | ceiling | run-1 | run-2 | run-3 |
|---|---|---|---|---|
| `n` | 2.890 | (not run) | 2.681 | 2.748 |
| `ni` | 2.666 | 2.554 | 2.486 | 2.554 |
| `hao` | 1.731 | 1.689 | 1.625 | 1.660 |
| 37-char `ceshiyixiacha…` | 2.094 | 2.045 | 1.931 | 1.927 |
| 59-char `zhegeyinqing…` | 1.625 | 1.612 | 1.505 | 1.511 |
| `zhongguo` (win, must be <1.00) | 0.323 | 0.267 | 0.256 | 0.250 |
| `cszysmsrsd` (win) | 0.474 | 0.392 | 0.378 | 0.397 |
| `zybfshmsru` (win) | 0.695 | 0.577 | 0.576 | 0.567 |
| startup | 1.091 | 0.735 | 1.054 | 0.907 |

Track A peak working set (186.2 MB vs 195.0 MB ceiling), `session_create…`
median, and every Track B regression-guard row pass in all three runs
(`threshold-check.csv` per run: `non-pass=[]`).

## Why this is the straddle closing, not luck
The straddle rows were the long inputs. fable's committed pre-#8 independent run
measured **37-char 2.165 (FAIL) / 59-char 1.653 (FAIL)**; post-memoization the
worst of three runs is **37-char 2.045 / 59-char 1.612**, both under ceiling, with
runs 2–3 comfortably lower (1.93/1.51). The memoization removes the exact
per-keystroke allocation those rows paid.

## Honest caveats (tightest rows)
- **59-char run-1 = 1.612 vs 1.625 (0.8% margin)** — the one tight row; runs 2–3
  sit ~7% under (1.505/1.511), so 1.612 reads as a high-variance sample, not the
  typical. It passes in all three, but this row has the least headroom.
- **startup run-2 = 1.054 vs 1.091** — startup is flagged run-noisy in the
  thresholds; passes all three (0.735 / 1.054 / 0.907).

## CORRECTION (2026-07-07) — the perf disposition above measured the WRONG config

The three "straddle CLOSED" runs and fable's independent run all deployed the
**source** product schema (`apps/yune-web/source/public/schema/luna_pinyin.schema.yaml`),
which carries **no** `leading_syllable_reachability` flag. Under the pre-flip read
(`schema_install` `…unwrap_or(false)`) that means the benchmark ran luna with the
**injection OFF** — the memoized `leading_syllable_fetch_codes` path never
executed in any of those runs. So:

- The "37/59-char row paid ~15–25k allocs/keystroke" narrative is **not what the
  benchmark measured** — those rows ran the no-injection path.
- The gap between fable's failing pre-#8 run (37-char **2.165**) and the passing
  post-#8 runs (**2.045/1.931/1.927**) was **run-to-run variance on injection-off
  luna**, not the memoization. The straddle was **not** closed by #8; it was a
  different (injection-off) workload that happened to land green.

When the M59 **default-ON flip** aligned the source schema to the shipped public
schema (injection ON), the benchmark measured the injection cost for the first
time and it **failed** three ceilings (37-char 2.550/2.577, 59-char 1.995/2.028,
zhongguo 0.423/0.421 — `m59-flip-ratchet/run-1..2`).

**What stands:** the memoization is still a correct, behavior-preserving change
(the deploy-path correctness suite is valid) and is the O(1) index the
**finding #8 completion skip** then builds on — the walk now skips the per-prefix
prism lookup on non-syllabary boundaries (`translator/mod.rs`
`leading_single_syllable_prefix_candidates`), collapsing the O(n²) long-input
cost. The real perf disposition for the injection lives with that skip + the
`m59-flip-ratchet` runs, **not** this section's original claim.

## Reproduce
```
cargo build --release -p yune-rime-api
scripts/benchmark-native-rime-inprocess.ps1 \
  -YuneDll target/release/yune_rime_api.dll -DeployProductBeforeBenchmark \
  -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru \
  -TrackAThresholds docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv \
  -OutputRoot docs/reports/evidence/m59-finding-8-perf/run-N
```
(Per-run `track-a-*`/`track-b-*` work subdirs — deploy artifacts incl. `.marisa`
string tables — and the heavy raw per-iteration CSVs are intentionally NOT
committed; only the gate result + summary + provenance are kept.)
