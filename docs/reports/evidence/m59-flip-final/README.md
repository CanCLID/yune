# M59 default-ON flip — perf gate + expanded diagnostic round

This bundle is the first honest ratchet measurement of the M59 leading-single
injection with the feature **actually on** for the benchmark's luna. It gates the
flip and baselines an owner-expanded Track A input set.

## Why a fresh round was needed (the finding-#8 measurement hole)

The benchmark deploys the upstream/source luna, which carried no
`leading_syllable_reachability` flag; under the pre-flip `schema_install`
(`…unwrap_or(false)`) the injection was **off in every prior ratchet run**,
including finding #8's "straddle closed" runs and the independent verify. Those
runs certified the *no-injection* path. The default-ON flip makes the benchmark
measure the mechanism for the first time. A provenance guard now fails the
benchmark if the deployed luna and the shipped web-product luna disagree on the
reachability state (`scripts/benchmark-native-rime-inprocess.ps1`,
`Get-ReachabilityFlagState`) so this class cannot recur.

## Attribution method — separating run-noise from feature cost (write this down)

The ratchet ratio (`yune ÷ librime`) is dominated by the poet sentence model on
the long rows and sits at the microbenchmark noise floor on the short keys, so a
raw flip-off↔flip-on ratio delta cannot be attributed to the feature. The method
that earned the attribution here — the discipline M55 lacked:

1. **Same-dll isolation runs.** Build one dll; run it with the flip toggled off
   vs on (a one-line `schema_install` default flip), everything else identical.
   The only difference is the feature. Compare against the *same* machine/session
   baseline, not a historical number.
2. **Per-input m37 decomposition.** `m37_metrics.csv` is per-input. Aggregate the
   counters for a single input across a run's samples, and diff flip-off vs
   flip-on. Read the **counts** (`owned_candidates_materialized`,
   `prism_lookup_codes`, `exact_lookup_candidates`, `candidates_sorted`) *and* the
   **timings** (`upstream_sentence_model_ns`, `short_key_first_page_materialize_ns`)
   separately.
3. **Poet noise is identified by same-dll variance.** `upstream_sentence_model_ns`
   for the 59-char row across the 5 committed identical-dll runs (`run-1..5`):
   4699 / 4592 / 4539 / 4569 / 4616 M — a **3.5%** same-dll swing. Any flip-off↔on
   poet-time "delta" inside that band is noise, not the feature. The flip has no
   code path into the poet; the m37 confirms it empirically.
4. **Same-count + higher-time = artifact, not computation.** `hao`'s **+197 µs**
   post-flip (`short_key_first_page_materialize_ns` 665400 → 862400) came with
   **every count identical** flip-off↔on: `owned_candidates_materialized` 1680=1680,
   `prism_lookup_codes` 1680=1680, `exact_lookup_candidates` 1120=1120,
   `candidates_sorted` 1680=1680. A cost with no work behind it is a cache/layout/
   branch timing artifact, not a removable computation; no optimization targets it.
   (This killed three successive hypotheses — the guard probe, `assign_ordered`,
   and the walk-call — each to m37 data.) Compare `m59-flipoff-isolation/` (feature
   OFF) against `run-1/track-a-yune/m37_metrics.csv` (feature ON) to reproduce.
5. **Injection real cost is the non-poet remainder.** On the long rows the
   injection's own cost (prism + materialize deltas) is a fraction of a percent of
   the poet base; the visible ratio movement on the thin rows is the small
   real cost eating an already-thin margin plus run-noise.

## Disposition

- **CPU** was closed by the O(1) boundary skip + the range cap (walk bounded to
  the longest syllabary code); **memory** by the schema-level precedence fix
  (the injection no longer fires on jyutping keystrokes where `prefix_fallback`
  did not apply per-request — Track B back to 80 MB, restoring the M58 contract).
- Four rows (`n`, `hao`, 37-char, and the later-added 59-char row) carry an
  owner-signed injection-on ceiling = **pooled-worst of the committed
  injection-on observations × 1.05**. The first three use the five committed
  `run-1..5` observations; the 59-char addendum uses those five plus the
  independent `claude-verify` observation (`1.665 × 1.05 = 1.748`). The other
  standing rows are untouched. The bare-syllable-guard rows (`yi`, `chuang`)
  are expected to show ≈ 0 feature cost — a nonzero cost there is a regression
  signal. See `../../../decisions.md` (M59 entry) and the pre-declared protocol in
  `docs/plans/completed/m59-plan-canonical-jyutping-reachability-parity.md`
  ("Flip perf-gate protocol").

M59 final acceptance does not rebaseline this historical flip packet. Five
fresh complete rounds at final behavior source `5879405c` pass `32/32`
aggregate rows and `160/160` individual observations under the same signed
ceilings. The final reconciliation is under
`../m59-canonical-jyutping-reachability-parity/final-closeout/`.

## Gate-verdict rule (pre-declared)

A row passes iff its **median** observation across the committed injection-on runs
is ≤ its ceiling. Median absorbs single-run outliers; `spread_pct` per row is
recorded in `m55-thresholds.csv` so each row's noise is visible in the gate
artifact.

## Reproduce

```
cargo build --release -p yune-rime-api
scripts/benchmark-native-rime-inprocess.ps1 -YuneDll target/release/yune_rime_api.dll \
  -DeployProductBeforeBenchmark \
  -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru,zh,j,yi,che,chuang,b,ceshi,zhongdengchangdu,dazisudu \
  -TrackAThresholds docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv \
  -OutputRoot docs/reports/evidence/m59-flip-final/run-N
```
(Per-run `track-a-*`/`track-b-*` deploy artifacts incl. `.marisa` string tables and
the heavy raw per-iteration CSVs are intentionally NOT committed; the gate result +
summary + m37 decomposition + this README are.)
