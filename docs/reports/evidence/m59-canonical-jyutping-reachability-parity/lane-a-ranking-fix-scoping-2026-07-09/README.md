# M59 Lane A ranking-fix — blast-radius scoping (owner/fable checkpoint)

Brief: `../lane-a-ranking-diagnosis-2026-07-09/README.md`. **This increment pins every
named surface's PRE-FIX state and gives the fix design + a per-surface disposition
PREDICTION for sign-off. NO engine code.**

> **CORRECTED 2026-07-09** after owner sign-off + fable's 4-lens verification. The first
> draft mis-attributed the TypeDuck surfaces to a "bounded product path" and claimed the
> fix "cannot touch the product." **Both are false** and are corrected below; the
> pre-fix pins themselves reproduced exactly.

## 0. Quality mechanism (was the blocker; now resolved)

The CLI `quality:0` was a red herring: `RimeCandidate` has no quality field, so
`copy_candidate` hardcodes `quality: 0` (`yune-cli/src/rime_frontend.rs:404-411`) — an
ABI placeholder, not the engine's ordering key. Internally: (1)
`format_candidate_for_lookup` computes `quality = raw_quality.exp() + initial_quality`
(`mod.rs:1601-1605`), which **saturates to `+inf`** at essay-scale weights; (2)
`assign_ordered_candidate_qualities` (`mod.rs:2878-2883`) then **overwrites it
positionally** (`(len+1) - index`), preserving the concatenation order, and fires on the
canonical Standard path because `leading_syllable_reachability` defaults ON; (3)
`engine.rs:1491` sorts by that positional quality → a no-op re-order. So the visible order
is fixed entirely by the spec-concatenation in `candidates_for_lookup_codes`. *(The 3
red rows' qualities `5198,5197,5196…` are exactly this positional `base - index`, which is
itself proof they run the full-list path.)*

## 1. Pre-fix state — pinned (current `main` @ `280d51a0`)

Measured 2026-07-09; no engine changes since. Two surprises the "pin first" step caught:
three cantonese_parity rows are already RED (the documented "3 pre-existing fails"), and
one is a pin the note assumed green.

| Surface | Command | Pre-fix state |
|---|---|---|
| Canonical bare syllable (the bug) | `yune-cli … --schema jyut6ping3 --sequence "bei"` (staged rime-cantonese) | `bei` page 0 = `碑 悲 卑 陂 蓖` (code-grouped); 畀 @~28. **What the fix changes.** |
| in-process `beingo`→畀@6 | `cantonese_parity::m58_current_yune_web_profile_reaches_beingo_bei_at_typeduck_rank` | **GREEN** — 畀 reachable @6, leading `俾我`. Preserve. |
| in-process fixture positions | `cantonese_parity::m58_profile_reachability_fixture_records_typeduck_v112_positions` | **GREEN** (asserts fixture values only). |
| in-process `zi`→諮@27 | `cantonese_parity::m58_current_yune_web_profile_reaches_zi_advice_at_typeduck_rank` | **RED (pre-existing)** — leads `就`, expected `自`; never reaches the 諮@27 check. |
| in-process `nei` golden | `cantonese_parity::m21_closeout_rows_match_typeduck_v112_real_dictionary_goldens` | **RED (pre-existing)** — `[呢 我 你 外 能]` (positional qualities 5198,5197,…) vs oracle `[你 呢 尼 妮 彌]`. |
| in-process prediction boundary | `cantonese_parity::m21_prediction_limit_preserves_m14_short_completion_boundary` | **RED (pre-existing)** — `我` vs `呢`. |
| cantonese_parity suite | `cargo test -p yune-core --test cantonese_parity` | **38 passed, 3 failed** (the 3 above). |
| luna parity | `cargo test -p yune-core --test upstream_luna_pinyin_parity` | **GREEN — 14 passed, 13 ignored.** |
| M59 injection-ordering | `cargo test -p yune-rime-api --test yune_web -- m59 reach` | **GREEN — 7 passed.** |
| **Playwright e2e (REAL toned product)** | `apps/yune-web/e2e/yune-web.spec.ts:1642-1698` (`@smoke`) | Pins over the deployed product: `beingo`→畀 at **page-turn 1, row 0**; `zi` first = `自`, 諮 not in first 6, 諮 reachable at **page 4**; `zijiguk`→諮 @ page 64 + select. **Not run this increment** — the true product surface. |
| M55 ratchet baseline | `…/m55-native-match-or-beat/thresholds/m55-thresholds.csv` | 23 rows recorded. |

**Correction — path attribution.** The in-process cantonese_parity engines are **not the
TypeDuck product**: `yune_web_jyut6ping3_mobile_engine` builds via
`StaticTableTranslator::from_dictionary` and `engine.set_schema(...)` **resets the profile
to Standard** (`engine.rs:309-313`), and being `from_dictionary` it is **prism-less** →
`expanded_lookup_specs` (prism-gated, `mod.rs:1086`) returns a **single spec**, no
tone-alias expansion. So all six in-process rows above run the **Standard full-list path
on single-spec lists**. The real multi-spec toned product is exercised only by the
Playwright e2e.

## 2. Fix design (Option A, approved conditionally)

Re-order the concatenation **inside `candidates_for_lookup_codes`** so the pooled
tone-variant candidates rank by per-reading weight, matching the oracle. **Not** "populate
`quality`" — that field is overwritten positionally and the exp-intermediate saturates to
`+inf` (§0).

**Conditions (owner, binding):**
- **(a) Strict scope to the multi-spec tone-alias merge.** Re-rank only when
  `expanded_lookup_specs` produced >1 exact-code spec from prism expansion. Single-spec
  lists (every prism-less parity harness) must be **byte-identical** — a no-op *by
  construction*, not by luck.
- **(b) Sort key = per-spec `PendingLookupCandidate::raw_quality`** through the existing
  tier comparator `lookup_candidate_order` (category → raw_quality desc → …) — **never**
  the exp-saturated/positional `quality`. Per-reading weight = `essay_weight(text) ×
  dict-%` for that `(text,code)` reading (already carried in `raw_quality`), so polyphones
  whose corpus weight belongs to another reading stay low (`費 bei3 3%`→`fai3`; `脾 bei2
  0%`→`pei4`; `輩 bei3 0%`→`bui3`).
- Preserve the **M59 leading-single injection** phrase-before-single ordering — the
  re-rank must touch only the exact-code tone-variant merge, not the injection splice.
- **Essay-load check:** `%`-weights parse to `0.0` and become non-zero only via the essay
  multiply — confirm the canonical deploy actually loads `essay-cantonese` for every
  re-ranked row; rows absent from the vocabulary stay weight-0 (would sink, not surface).

**(c) Validation is EMPIRICAL, not architectural** — run, do not reason:
- `cantonese_parity` → still **38/3** with **byte-identical failing values** (`就`/`呢-list`/`我`).
- `yune_web` **full jyutping set** (not just the `m59 reach` filter — the correction / `nri`
  multi-spec tests too).
- `upstream_zhuyin_parity`.
- **M55 ratchet** with `-DeployProductBeforeBenchmark` (M38); standing ceilings, no re-baseline.
- **Playwright e2e** `yune-web.spec.ts:1642-1698` — browser evidence per CLAUDE.md; the
  toned product is multi-spec, so `beingo`→畀@page-1-row-0 and `zi`→諮@page-4 are the pins
  that can actually move.

## 3. Per-surface disposition PREDICTION (empirical gate in the last column)

| Surface | Path / spec shape | Prediction | Why | Gate |
|---|---|---|---|---|
| Canonical bare syllable | full-list, **multi-spec** | **WILL MOVE (intended)** | pooled tone-variants re-ranked by per-reading weight → `bei`=畀比被鼻避. | lane-a-runner diff over the 13-input capture; head=畀比被鼻避. |
| in-process `beingo`→畀@6 (green) | full-list Standard, **single-spec (prism-less)** | **BYTE-IDENTICAL (stays green)** | scoped fix skips single-spec lists (condition a). | cantonese_parity green. |
| in-process `zi`/`nei`/pred-limit (RED) | full-list Standard, **single-spec** | **BYTE-IDENTICAL (stays red, same values)** | same — no multi-spec merge fires. **Empirical gate, not structural** (they ARE on the fix's path). | 38/3 with identical `就`/`我`/`呢-list`. |
| luna parity (green) | luna dict is UNTONED | **BYTE-IDENTICAL** | `untoned_dictionary()` true → single-tone codes, no tone-alias multi-spec. Watch fuzzy/correction multi-spec rows. | `upstream_luna_pinyin_parity` 14/14. |
| zhuyin parity | toned, may multi-spec | **VERIFY** | bopomofo tone marks could tone-expand. | `upstream_zhuyin_parity` unchanged. |
| M59 injection (green) | page-turn + injection splice | **UNAFFECTED IF splice preserved** | re-rank must not disturb phrase-before-single (condition). | `yune_web` full jyutping incl. m59 reach. |
| **Playwright e2e — REAL product** | full-list, **multi-spec (prism, toned)** | **CAN MOVE — must validate** | the toned product tone-expands (`zi`→zi1..zi6), so the re-rank fires here; `beingo`→畀@page-1-row-0 / `zi`→諮@page-4 are the pins at risk. | Playwright `@smoke` green (browser evidence). |
| M55 ratchet | typing = bounded path | **GREEN (predicted)** | fix on full-list/page-turn, not the bounded typing path. | ratchet green, `-DeployProductBeforeBenchmark`. |

## 4. Sign-off status + open items

- **Ask 1 (quality mechanism):** RESOLVED + folded into the diagnosis doc (§0 here).
- **Ask 2 (Option A):** approved **conditionally** — conditions (a)/(b)/(c) above are binding.
- **Ask 3:** the 3 pre-existing TypeDuck-golden fails are a **separate product-lane
  ranking work item**; because they are full-list-path, "same values pre/post" is an
  **empirical gate** the fix must run, not a structural given.
- **Ask 4:** residual order-only rows after the fix → owner as the **D-48 disposition
  table** (per-input, order-exact through the captured range).

Fix increment proceeds under the conditions above; every row's "Gate" is a required
pre-merge check.

## CORRECTION (2026-07-09, fix increment): scoping model revised multi-spec → multi-code

The fix design above frames the re-rank as a cross-SPEC tone-alias merge. Empirically the
canonical path delivers ONE spec with multi-CODE exacts (see the fix README's model
correction). Option A survives in hardened form: detector + mutation scoped to true-exact
rows (entry_code == lookup_code, no abbreviation/correction/limited-prediction),
stable-sorted by per-reading raw weight only, reinserted in-slot — completions and
correction rows never move, ties keep storage order in both regimes. The §3 disposition
predictions were re-validated empirically against the hardened fix (all pinned suites
byte-identical; canonical heads = oracle):
`../lane-a-ranking-fix-2026-07-09/README.md`.
