# Lane A ranking diagnosis — 2026-07-09 (fable-CONFIRMED)

Diagnosis-only increment per the fable seed
(`../lane-a-diff-2026-07-08/README.md` "Seed for the NEXT increment"). **No engine
code changed.** The seed's weight-collapse hypothesis is REFUTED; the true root cause
is demonstrated below and **independently confirmed by fable** (repro: `quality:0`
across all candidates, `畀`@rank 28, concatenation locus verified in code).

## TL;DR

- **Seed hypothesis REFUTED.** "The essay-vocabulary percentage-weight chain is not
  applying on the canonical dict build" is **wrong** — the weights ARE applied.
- **Root cause (candidate merge/rank, not weight loading):** for a toneless syllable,
  Yune looks up each tone-variant (`bei1`, `bei2`, `bei3`, `bei6`) as a separate
  lookup spec and **concatenates the specs in code order**; downstream quality is
  assigned **positionally** (`assign_ordered_candidate_qualities`, `mod.rs:2878-2883`),
  which preserves that concatenation, so nothing re-ranks by weight and the entire
  low-frequency `bei1` group precedes high-frequency `畀`(bei2)/`被`(bei6). (The CLI
  `quality:0` is an ABI display placeholder, `rime_frontend.rs:404-411` — NOT the
  engine's ordering key; see §Root cause item 4.) librime pools all tone-variants and
  ranks by global weight.
- **Step 4 (`nri`→0 / `ngohaig` 46-vs-2050): independent segmentation/fuzzy gap** —
  file separately; not part of the ranking fix.
- **No fix.** The fix touches the shared candidate path; a blast-radius scoping note
  (below) is the owner/fable checkpoint before any code.

## The seed hypothesis is refuted (weights ARE applied)

`bei1`-family essay-cantonese weights (empty weight column ⇒ 100% of essay frequency):

| char | 碑 | 悲 | 卑 | 陂 | 蓖 | 羆 | 萆 | 鵯 | 犤 | 庳 | 椑 | 詖 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| essay wt | 4449 | 1423 | 1164 | 736 | 597 | 576 | 540 | 515 | 443 | 411 | 409 | 339 |

Yune's real-path `bei1` group prints in **exactly essay-weight-descending** order,
including `羆`(576) **before** `犤`(443) despite `犤` being *earlier in file order* — a
signal only weight-sorting can produce. So the percentage-weight chain works. (fable's
compiled-table lens independently reproduced this and refuted the collapse hypothesis.)

## Root cause, with verified code locus

`crates/yune-core/src/translator/mod.rs`:

1. `expanded_lookup_specs("bei")` (~L1083) → `[exact("bei"), alias(bei1→bei),
   alias(bei2→bei), alias(bei3→bei), alias(bei6→bei)]`, in prism/code order.
2. `candidates_for_lookup_codes` (~L2345) **loops specs in that order** and
   `candidates.extend(pending)` — appends each spec's candidates to one global vec.
   **Within-spec order comes from STORAGE order** — `sort_rime_table_entries`
   (`dictionary/source.rs:1029`) sorts entries `code.cmp(…).then(weight desc)`, i.e.
   **code-primary then weight-descending**, so within one code the rows are already
   weight-desc. **Edit (fable):** the per-spec runtime re-sort at
   `candidates_for_lookup_codes` (~L2457) is **gated on
   `self.prediction_candidate_limit.is_some()`**, so on lanes where it does not fire
   the within-group weight-desc order is supplied by the storage sort, not that line.
3. Caller `translated_candidates_for_segment_with_prefix_fallback_limit` (~L2903) runs
   only `combine_duplicate_text_candidates` (dedup keeps the **first / lowest-code**
   position — why multi-tone `蓖` shows comment `bei1;bei3;bei6` at its `bei1` slot)
   and `enforce_prediction_never_first`. Neither ranks by weight.
4. **Quality mechanism — CORRECTED 2026-07-09 (owner/fable, verified).** The earlier
   "`engine.rs:1491` is a no-op because `quality==0`" framing was imprecise. The full
   chain:
   - The CLI `quality:0` is an **ABI placeholder** — `RimeCandidate` carries no quality
     field, so `copy_candidate` hardcodes `quality: 0` (`yune-cli/src/rime_frontend.rs:404-411`).
     It is NOT the engine's internal ordering key; reading it told us nothing.
   - Internally, `format_candidate_for_lookup` computes `quality = raw_quality.exp() +
     initial_quality` (`mod.rs:1601-1605`), which **saturates to `+inf`** at essay-scale
     weights (e.g. `820637.exp()`) → uniform, useless for ranking.
   - `assign_ordered_candidate_qualities` (`mod.rs:2878-2883`) then **overwrites quality
     positionally** (`(len+1) - index`), preserving the concatenation order. It fires on
     the canonical Standard path because `leading_syllable_reachability` defaults ON.
   - `engine.rs:1491` sorts by that positional quality → a **no-op re-order** (already
     sorted).
   Net: the visible order is fixed entirely by the spec-concatenation in
   `candidates_for_lookup_codes`; nothing downstream re-ranks by weight. **The fix must
   re-order the concatenation using per-spec `raw_quality` BEFORE the positional
   stamping** (see the scoping increment's Option A conditions).

Net = `[bei1 ↓wt][bei2 ↓wt][bei3][bei6 ↓wt]` — tone-grouped.

## Real-path trace (the demonstration)

Warm CLI, staged rime-cantonese + `lane-a-runner/default.yaml`, page size 5:

- **Page 0 = `碑 悲 卑 陂 蓖`** (all `bei1`) — matches `../lane-a-diff-2026-07-08/classified.json`.
- Highest-weight `畀` (essay 820637) is buried **~28 positions** in (top of the `bei2`
  group); `被`(bei6, 259308) ~99 in. The CLI reports `quality: 0` for every candidate —
  but that is the ABI placeholder (item 4), not the engine's ordering key.
- Oracle (`../phase-1/canonical-rime-cantonese-capture.json`): `畀 比 被 鼻 避` at ranks
  0-4 = global weight descending (820637 > 676300 > 259308 > 20467 > 6051), `bei2`/`bei6`
  interleaved, **no tone grouping**. Head is strictly monotone in essay weight; `碑`
  (Yune's #0) is oracle #6.

## Step 4 — `nri`→0 / `ngohaig` 46-vs-2050 (independent, file separately)

`nri` → **0 candidates**, preedit stays `nri` (unsegmented); `nri` is not a resolvable
jyutping syllable and has no exact code. Oracle `nri` = 1309 (== `n`; librime treats the
trailing `ri` via fuzzy/segmentation). `ngohaig` 46 vs 2050 = a longer-input
lattice/segmentation divergence. Both are segmentation/fuzzy behavior, orthogonal to the
tone-merge ranking — a separate named work item, NOT part of the ranking fix.

## Fix spec (NOT implemented — see scoping checkpoint below)

Mechanism: pool the tone-variant specs and **re-order the concatenation inside
`candidates_for_lookup_codes` by per-spec `raw_quality`, BEFORE the positional stamping**
(`assign_ordered_candidate_qualities`). NOT "materialize weight into `quality`" — that
field is overwritten positionally and the exp-intermediate saturates to `+inf` (item 4);
the sort key must be `PendingLookupCandidate::raw_quality` through the existing tier
comparator `lookup_candidate_order`. **Edit (fable), load-bearing:** rank by the
**per-reading weight = `essay_weight(text) × dict-percentage` for that specific
`(text, code)` reading**, NOT a char-global / essay-max weight. Otherwise polyphones whose
large corpus weight belongs to a *different* reading jump up incorrectly — e.g. `費 bei3 3%`
(dominant reading `fai3`), `脾 bei2 0%` (`pei4`), `輩 bei3 0%` (`bui3`). Yune's per-spec
entries already carry the correct per-reading `raw_quality` (that is why the within-`bei1`
order is right); the fix is to make the **cross-spec** ranking use it. Full conditions +
disposition: `../lane-a-ranking-fix-scoping-2026-07-09/README.md`.

## Blast-radius scoping (owner/fable checkpoint — REQUIRED before any fix)

The full-list candidate path is **shared by every schema/profile**, so a cross-spec
re-rank must be scoped and dispositioned before landing. Surfaces to pin first:

1. **TypeDuck `jyut6ping3` PRODUCT profile — frozen pins (D-48).** The grandfathered
   `畀@6` / `諮@27` fixture pins must NOT move (D-48 §3: TypeDuck lane frozen, do not
   extend/delete). Determine whether the product reaches page 0 via this same
   concatenation path or via `prefix_fallback` / `leading_syllable_reachability`
   (different ordering machinery), and scope so the product ordering is untouched.
2. **luna_pinyin fixtures** — `upstream_luna_pinyin_parity` (14/14) must stay green.
   luna codes are toneless (no tone expansion) but fuzzy/correction still create
   multiple specs, so a cross-spec re-rank can reorder luna candidates.
3. **M55 perf ratchet** — the change alters the full-list/page-turn path; re-run with
   `-DeployProductBeforeBenchmark` (M38 lesson: compiled product bytes may shift), and
   confirm the standing ceilings stay green (no re-baseline).
4. **M59 injection-ordering tests** — `m59_luna_*` phrase-before-single ordering,
   `sentence_over_completion` floor, bare-syllable guard, and the jyutping
   `zijiguk`→諮 / `beingo`→畀 reachability pins must all survive a cross-spec re-rank.

Disposition each surface (unchanged / re-derived-with-justification / owner-signed
exception) before writing engine code. Residual order-only rows go to the owner as the
D-48 disposition table.

## Adversarial verification (2026-07-09)

A four-lens pass (compiled-table / ranking-locus / oracle-semantics / fix-scope) plus
fable's independent review agree: weights applied (collapse refuted); the effective
ranking failure is `quality:0` defeating the `engine.rs:1491` sort on the full-list
path; the oracle target is pure global weight but must use the **per-reading** weight;
and the fix has real blast radius (this note).

## CORRECTION (2026-07-09, fix increment): the tone-grouping is multi-CODE, not multi-spec

An in-engine probe of the real canonical path showed `bei` reaches
`candidates_for_lookup_codes` as **one spec** whose `exact_candidates` span all tone-codes
(2178 rows, `bei1`..`bei6` in per-code storage order) — the compiled prism resolves the
toneless syllable to a single code. The "per-tone lookup specs concatenated in spec order"
model above is therefore wrong on the compiled canonical path (alias-spec expansion exists
but does not occur there); the concatenation is per-CODE within one spec's exact set. The
mechanism conclusion (code-grouped concatenation, no global weight re-rank, positional
quality freeze) is unchanged. Fix + hardened mechanism:
`../lane-a-ranking-fix-2026-07-09/README.md`.
