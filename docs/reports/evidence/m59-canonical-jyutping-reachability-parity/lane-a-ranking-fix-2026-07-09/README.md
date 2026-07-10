# M59 Lane A ranking-fix — implementation + validation (2026-07-09)

Implements the owner-approved fix (scoping doc `../lane-a-ranking-fix-scoping-2026-07-09/`),
**hardened per the fable review** (detector + mutation scoped to true-exact rows; see "The
fix (hardened)" below). Execution handed to fable by the owner on 2026-07-09; the original
implementation and the model correction below are the prior session's work.

## Model correction — the "multi-spec tone-alias" premise was empirically WRONG

The scoping doc (and fable's verification) assumed a toneless syllable tone-expands into
several **lookup specs** (`exact("bei")`, `alias(bei1)`, `alias(bei2)`, …), so the fix was
framed as a *cross-spec* re-rank. An in-engine probe of the real canonical path refutes it:

```
[CFLC] specs=1 exact_alias_spec_count=1 pooled=2178 codes=[("bei","bei")]
```

For canonical `bei`, `candidates_for_lookup_codes` is called with **one** spec
(`("bei","bei")`), and `exact_candidates("bei")` returns **2178** rows — the compiled
prism/table resolves the toneless syllable to a single code whose exact candidates span
**all tone-codes** (`bei1`/`bei2`/`bei3`/`bei6`), concatenated in per-code storage order.
So the tone-grouping is **multi-code within ONE spec**, not multi-spec. The cross-spec
trigger never fired (smoke unchanged), which is how this surfaced. The committed diagnosis
and scoping docs carry dated corrections pointing here.

## The fix (hardened)

In `candidates_for_lookup_codes` (`translator/mod.rs`), after pooling the per-spec
candidates:

1. **Detector — scoped to TRUE EXACT rows** (`entry_code == lookup_code`, not
   abbreviation, not correction, not limited-prediction): walk them in pooled order and
   fire only on a **strict `raw_quality` increase**. If the exact rows are already
   weight-non-increasing, nothing is touched — that covers every `sort: by_weight`
   single-code set *by construction*, and any concatenation that happens to be already
   weight-sorted *by observation* (skip-on-non-increase is proven order-correct: internally
   non-increasing groups + non-increasing junctions ⇒ globally non-increasing).
2. **Mutation — scoped to the same rows**: stable-sort ONLY the true-exact rows by
   per-reading `raw_quality` descending and reinsert them in their original slots.
   Completions, corrections, and abbreviation rows **never move**; equal-weight ties keep
   storage order in **both** regimes (the stable sort has no text/entry-code tiebreak), so
   tie behavior does not depend on whether the detector fired.

Why the earlier justifications were replaced (fable review, verified by code lenses):

- ~~"single-code exact sets are stored weight-descending, never increase"~~ — TRUE only
  for `sort: by_weight`; `sort: original` dictionaries (including the shipped product
  jyut6ping3/cangjie dicts) make no per-code weight promise — today's files scan globally
  weight-descending, but that is a data property. The hardened detector **does not assume
  the invariant**: it conditions on the observed order itself.
- ~~"prism-less parity harnesses yield no toneless exacts"~~ — REFUTED: they DO (the
  spelling algebra puts toneless codes in the index). They are untouched because a
  prism-less table yields a **single exact spec** and those engines set
  `prediction_candidate_limit`, whose per-spec sort already delivers a weight-sorted pool
  (no strict increase, detector silent; a fired stable re-sort of a sorted list is a no-op
  anyway).
- ~~"the splice lives in a different path"~~ — the unbounded prefix-fallback and
  leading-single splices run **after this function returns, in the caller, over the
  re-sorted list**. They are safe because the mutation preserves block structure (exact
  slots stay exact slots), so both splice anchors resolve identically pre/post — verified
  by code lens and by the m59 injection suite.

Known fire-sites beyond the canonical target (named, accepted):

- **Compiled-profile alias exacts** (e.g. luna `z`/`zh` abbreviations on the page-turn
  path): alias exact rows are in-domain and may permute **among the exact rows only** —
  completions no longer move (this was wider in the unscoped version). Unpinned area;
  luna parity + m59 suites green.
- **`sort: original` single-code sets** are protected only by today's file order; if a
  future dict edit introduces a per-code weight increase, the detector will fire and
  weight-sort those exacts. This is recorded as intended-lean behavior (weight order is
  the oracle-correct order), not silent.
- **Tolerance-rule rows are EXCLUDED from the domain** (adversarial-review finding): a
  tolerance near-match spec creates rows with `entry_code == lookup_code` for a code the
  user did not type; without exclusion, a junction increase would interleave them above
  the typed-code exacts. `LookupCodeSpec::tolerance_exact` now marks them and
  `is_true_exact` skips them — pre-fix trailing placement preserved by construction.

Adversarial self-review (2 lenses on the implementation, one independently re-running the
gates): slot-reinsertion permutation and detector proven sound; the per-spec-extend
removal proven byte-equivalent when the detector is silent (incl. `record_track_b`
totals); the counter-test guard confirmed poison-proof and pattern-matching. Two findings
applied before commit: the tolerance-domain exclusion above, and the sort comparator
switched to `f32::total_cmp` (the `partial_cmp`/`unwrap_or(Equal)` idiom is not a total
order and may panic under the post-1.81 sort on NaN weights, which are constructible from
dict data; `total_cmp` is behavior-identical for non-NaN).

Owner conditions:

- **(a) byte-identity for untouched surfaces** — via the scoped domain + skip-on-sorted,
  and **verified empirically** across the full suite matrix below (the empirical gates are
  the guarantee of record; architectural reasoning above is explanation, not proof).
- **(b) sort key = `PendingLookupCandidate::raw_quality`** (per-reading weight, essay ×
  dict-%) — never the exp-saturated intermediate or the positional overwrite values.
  Polyphones whose corpus weight belongs to another reading stay low (畀 bei2 100% vs
  bei3 3%).
- **M59 injection preserved** — splice anchors consume unchanged block structure; m59
  suite green.

## Correctness gates (hardened version, run by fable)

| Gate | Result |
|---|---|
| Canonical smoke `bei` | **畀 比 被 鼻 避** (oracle head) ✓ |
| Canonical `be` | **啤 嚊 唄 𠹇** = the oracle's complete 4-row list ✓ (tail beyond oracle depth stays in storage order — see scope note) |
| Canonical `beix` | 畀 比 髀 俾 吡 (oracle head) ✓ |
| cantonese_parity | 38 passed / 3 failed — **byte-identical** failing values (`就`≠`自`, `我`≠`呢`, `nei`=[呢 我 你 外 能]@5198…) |
| upstream_luna_pinyin_parity | 14/14 |
| upstream_zhuyin_parity | 3/3 |
| upstream_cangjie_parity / cangjie5_composition / double_pinyin | green |
| yune-core `--lib` | 307/0 — **5x consecutive** (see flaky-guard note) |
| yune_web (full, incl. correction/nri + m59 injection) | 45 passed / 0 failed (2 ignored) |
| cold_start_conformance | 1/0 |
| typeduck_windows_boundary | 2/4 — **pre-existing since `5d3dba2a`, exonerated** (identical on pristine main; profile-activation repair is a separate named work item, GPT lane) |
| fmt / clippy (`-D warnings`) | clean |

**Flaky-guard fix (same commit):** `bounded_long_prefix_fallback_keeps_two_candidates_per_fetch_code`
asserts global m37/m40 lookup counters but was the ONE counter test without
`m37_metrics_test_guard()`; under the fix's slightly different test timing it raced the
guarded counter tests (`--lib` flaked ok/1-fail/2-fail; pristine main happened not to
collide, 3x green). Both racers pass in isolation; adding the standard guard makes
`--lib` stable (5x consecutive 307/0). Pre-existing race of the M56 "abuse-suite
concurrency" wart class, exposed not introduced by this change.

**Essay-load check:** the `bei1` group prints in exact essay-cantonese weight order
(碑4449 > 悲1423 > 卑1164 > …) — `essay-cantonese` is loaded and applied per reading; rows
absent from the vocabulary keep weight 0 and sink.

**Scope note on tails (exact-only mutation):** beyond the oracle's captured range the
completion tail keeps storage order (e.g. `be` row 5 is 啤把, where the unscoped prototype
had promoted the completion 畀). The captured range is the D-48 parity surface; the tail is
the owner-required reachability extension and its ordering follows the M59 injection rules.

## Re-frozen 13-input diff + disposition table

`re-diff/` in this directory: the committed 13-input oracle capture (provenance pinned,
counts cross-match the pre-fix diff), the post-fix classifier output, and the **D-48
disposition table** (5 residual classes; 2 exception rows for owner signature, 3 named
work items). Headline: `be` and `beix` are order-exact through the oracle's COMPLETE
captured lists; `bei` order-exact through 138/139 modulo one variant-sibling drop (祕)
and two equal-weight tie swaps.

## Scope — residuals → D-48 table

Fixed: the bare-syllable multi-code tone-merge (`bei`, `be`, `beix` family). Unchanged
(different mechanisms, expected): multi-syllable/phrase inputs (`being`, `beingo`,
`zijiguk`, `mgoi` — phrase/sentence ranking) and abbreviation/segmentation inputs (`n`,
`nri`, `ngohaig`, `bein` — the step-4 class). The re-frozen 13-input diff + per-class
disposition table (this directory, `re-diff/`) is the D-48 instrument of record.

## Heavy gates — BOTH RUN

- **M55 ratchet: MEDIAN GATE ALL PASS** (`ratchet-README.md` + `ratchet-run-1..6/`). The
  first implementation's whole-pool detector scan cost `ni` ~2% (median 2.695 > 2.666,
  consistent — a REAL regression, caught by the gate); fixed by scanning only recorded
  exact-block ranges, after which every row passes with margin and `ni` (2.544) lands
  below even its flip-era median (2.631). No ceiling was touched.
- **Playwright e2e (fix-bearing locally-built WASM, dev server): 7 passed / 4 skipped
  (public-demo-gated conditionals) / 1 failed** — the failure is
  `M58 yune-web TypeDuck profile reaches oracle-ranked reported candidates` (`zi` → 諮 no
  longer reachable within 4 PageDowns; browser page 4 shows completion phrases).
  **EXONERATED for this fix by A/B**: a WASM built from the identical tree with the fix
  stashed fails the same test the same way (fable, 2026-07-09, live-server rerun after a
  first attempt was voided by a dead dev server). The last browser-validated WASM
  predates the M59 flip entirely — this is the first browser validation of the whole M59
  engine line, and the regression belongs to that line (flip/corrective/c4336cd9 window),
  not to this change. Filed as a named work item: browser-lane `zi` paging regression on
  the M59 line, wants a WASM-level bisect (archived A/B artifacts make swaps cheap) and a
  browser gate added to flip-class changes. Both WASM builds archived in the session
  scratchpad; the shipped/committed browser assets are untouched by this commit.

## Browser A/B evidence (durable record, per GPT request)

Live-server A/B on the M58 Playwright pin (`zi` -> 諮 within 4 physical PageDowns),
2026-07-09: **fix-bearing WASM FAILS; pre-fix WASM (identical tree, fix stashed) FAILS
identically** — last page observed `["正常","佔","症","節目","織","折"]` (completion
phrases). SHA-256 (first 16 hex) of the archived artifacts:
- fix.wasm: `117e0e29e1f55db6`
- prefix.wasm (tree b11e7ab5 + f0ca95d6-minus-crates): `07bbe40107ca78f4`

Root cause (GPT-diagnosed, fable code-verified): `ba15e725` (M59 finding #4) made the
physical-selector forward-page completion UNCONDITIONAL while `RimeChangePage`
(lib.rs:1750-1756) retains a `jyut6ping3*` + `>2 chars` gate — so keyboard PageDown on
2-char `zi` swaps the bounded 30-row window (諮@27) for the unbounded list, while the
UI next-page button and the native M58 test (both via RimeChangePage) stay bounded and
green. The surviving lib.rs:1752 `starts_with("jyut6ping3")` gate is the same schema-ID
wart class finding #4 deleted from the selector. Repair contract + guards: GPT's named
work item (one profile-aware forward-page predicate for both paths; no schema-ID
inference; no fixture re-pin).
