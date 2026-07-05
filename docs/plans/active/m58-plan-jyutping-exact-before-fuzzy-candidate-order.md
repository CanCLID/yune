# M58 Jyutping Exact-Before-Fuzzy Candidate Order Repair Plan

> **For agentic workers:** execute one phase at a time. This plan is written as
> a review packet: before implementing, have another reviewer challenge the
> root-cause chain, the fix, and the oracle gates. The named product target is
> the TypeDuck `jyut6ping3` profile against TypeDuck-HK/librime `v1.1.2`; that
> fork is the correctness oracle for every candidate-order claim here.

> **Status:** Draft for review (v2, amended 2026-07-05 after two review rounds
> + code/fixture verification). - **Track:** Engine behavioral correctness
> (TypeDuck/Jyutping product lane). - **Created:** 2026-07-05. - **Type:**
> bug-fix milestone. No ABI widening, no new performance claim.

> **Amendment note (v2).** Review + a code/fixture verification pass corrected the
> v1 chain in three load-bearing ways:
>
> 1. **畀 is cut by a cap, not buried.** Fallback candidates sort
>    `consumed_input_len`-descending, so the exact `bei` codes sort *ahead* of any
>    flood; 畀 is dropped by the **per-fetch cap of 2**
>    (`MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE`), which fires only on the
>    compiled product path. The oracle emits the **full** exact set per syllable
>    (checked-in `ng`→19, `hou6`→13 chars — never two), so this cap is the
>    **solid, oracle-confirmed core bug** (Defect B). Fixing it makes 畀 reachable
>    regardless of any flood.
> 2. **The oracle is *not* "no fuzzy ever."** It emits same-initial floods (a)
>    for an unparseable head (`nri` correction-off → 我/你/外/能…, pinned green by
>    `m21_nri_...`), **and** (b) even for a *complete* leading syllable in some
>    option/page configs — verified: `m` in `jyut6ping3-m21-closeout.json` case 2
>    yields 20 exact `m`/`ng` candidates then a 30-candidate cross-final `m*`
>    flood (`is_last_page:false`), while the same `m` in `fork-parity-01` is
>    exact-only. So whether Yune's `beingo` `b`-flood is an oracle *divergence* is
>    **not yet established** — our `beingo`/`ngohaig` captures are page-0-only
>    (`is_last_page:false`). Defect A (flood suppression) is therefore
>    **contingent** on the Phase 0 full paginated capture, not assumed.
> 3. **Leg A's boundary is the lookup path, not the segmentor.** Segmentors emit
>    only whole-input type tags (`abc`/`raw`/`punct`/…; `SegmentDebug` spans
>    `0..input.len()`), *not* syllable boundaries — so the leading-parse helper
>    must reuse `valid_lookup_prefixes`/`sentence_lookup_specs` (prism
>    `lookup_canonical_codes` + `storage.has_code`), the same path that yields 我
>    for `ngohaig`. Do **not** use `context.segment_tags`.
>
> Sections below carry the corrected design.

**Goal:** Make Yune's TypeDuck/Jyutping candidate output match the TypeDuck/librime
`v1.1.2` oracle for multi-syllable composition, in two parts of differing
confidence:

- **Core (confirmed): emit the full exact leading-parse set.** For a composition
  with a complete-syllable leading parse (`beingo`→`bei`; `ngohaig`→`ngo`), Yune
  currently caps each exact toned code at two characters
  (`MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE`, compiled path only), so
  less-common exact characters (畀 = third `bei2`) are unreachable. The oracle
  emits the full exact set per syllable (`ng`→19, `hou6`→13). Lift the cap for
  leading-parse codes so the full exact set — including 畀 — emits.
- **Contingent (capture-gated): suppress the same-initial flood only where the
  oracle does.** Yune appends a same-initial flood (`b`-initial 不/本/部/報 for
  `beingo`). The oracle is parse-state- **and option/page-** conditional: it
  shows a flood for an unparseable head (`nri`) and *sometimes* for a complete
  syllable too (`m` case above). So the flood is a divergence only where the
  **full paginated oracle capture with the product's real options** shows the
  oracle does not emit it. Phase 0 decides whether Leg A is in scope; the `nri`
  flood (`m21_nri` golden) must not be regressed either way.

This is a comparability/correctness repair, not a reordering-of-fuzzy exercise.

## Problem Statement

Typing a multi-syllable word and picking characters one at a time is broken:
less-common exact-syllable characters are unreachable, and same-initial fuzzy
characters the oracle does not show appear. These are two separate defects (a
per-fetch cap and a same-initial flood), not one — see below.

Reproduced on this machine (`jyut6ping3_mobile`, `yune-cli frontend`, page
size 6), input `beingo` (畀我), paging with `=`:

| Page | Candidates (text/code) |
| --- | --- |
| 0 | `俾我`(bei2ngo5) `比`(bei2) `被`(bei6) `備`(bei6) `俾`(bei2) `悲`(bei1) |
| 1 | `秘`(bei3) `臂`(bei3) `卑`(bei1) · **`啤`(be1) `唄`(be6) `不`(bat1)** |
| 2 | **`本`(bun2) `表`(biu2) `部`(bou6) `報`(bou3) `巴`(baa1) `不過`(bat1gwo3)** |
| 3 | **`波`(bo1) `邊`(bin1) `保`(bou2) `班`(baan1) `變`(bin3) `別`(bit6)** |

Two things are wrong here, and they are **independent** (verified against
`translator/mod.rs`, see Diagnostic Evidence):

1. **Same-initial flood (bug-status capture-gated).** From page 1 onward the list
   mixes in `b`-initial characters with unrelated finals (`bat`/`bun`/`biu`/`bou`/
   `baa`/`bo`/`bin`/`bit` → 不/本/表/部/巴/波/邊/別…) that are **not `bei`**. The
   two `be`-syllable characters 啤 (`be1`) / 唄 (`be6`) are the exact matches of
   the `be` leading parse, *not* part of that flood — it is the single-letter `b`
   prefix that pulls in the rest. Whether the oracle *also* shows this tail for
   `beingo` is unconfirmed (a complete syllable can flood in some configs — see
   Oracle Evidence, the `m` case), so this is fixed only if the full capture says
   so.
2. **Per-fetch cap hides 畀.** The exact `bei` characters are **not** positionally
   crowded out — the fallback candidates sort `consumed_input_len`-descending, so
   every `bei*` code sorts ahead of the `b`/`be` flood (see page 0–1: 比 被 備 俾
   悲 秘 臂 卑 all precede 啤/唄/不). The reason 畀 never appears is that each
   toned code emits **exactly two** characters and then stops: `bei2`→比,俾;
   `bei6`→被,備; `bei1`→悲,卑; `bei3`→秘,臂. 畀 is the *third* `bei2` character, so
   the per-fetch cap of 2 truncates it. (For bare `bei`, 畀 *does* appear — the
   full input has an exact multi-tone lookup that flows through the primary
   exact/sentence path, not the capped prefix-fallback loop; Phase 1 must confirm
   this is the path difference.) So the user must type `bei` alone, commit 畀,
   then type `ngo` separately.

Second reported case: `諮議局` (typed syllable-by-syllable; not a lexicon word)
shows `z`-initial fuzzy (`就` zau6, `在` zoi6, `主` zyu2) before the exact `zi`
characters are exhausted, so 諮 (`zi1`) is unreachable the same way. (For the
single syllable `zi`, `enable_completion` additionally injects longer
`zi`-prefixed syllables — `政` zing3, `即` zik1, `接` zip3 — ahead of exact
`zi` characters; see Scope.)

## Diagnostic Evidence (root cause)

The candidate list for a bounded lookup is assembled in
[`translator/mod.rs`](../../../crates/yune-core/src/translator/mod.rs): the
sentence/exact candidates are produced first, then **prefix-fallback candidates
are appended to fill the limit** (`bounded_candidates_for_lookup_codes`, the
`self.prefix_fallback && !has_correction_lookup` branch appends
`prefix_fallback_candidates(...)`). `jyut6ping3_mobile` sets `prefix_fallback:
true`, `enable_completion: true`, `prediction_never_first: true`.

There are **two independent mechanisms**. Defect B (the cap) is the confirmed
core bug and makes 畀 unreachable on its own — fixing it is the primary win.
Defect A (the flood) is a separate mechanism whose bug-status is capture-gated
(the oracle sometimes floods a complete syllable). Removing the flood does not
surface 畀; lifting the cap does.

### Defect A — the same-initial flood (contingent; mechanism to confirm in Phase 1)

The flood comes from a **single-letter prefix expansion**, in two layers
(verified reading of `translator/mod.rs`):

- `valid_lookup_prefixes(lookup_code)` (`:2328`) scans every character-boundary
  prefix longest-first (`boundaries.reverse()`, `:2334`) and keeps a prefix only
  when `sentence_lookup_specs(prefix)` (`:1164`) yields a canonical code passing
  `storage.has_code(&spec.code)` (`:2340`). `sentence_lookup_specs` resolves a
  spelling through the prism (`prism.lookup_canonical_codes_with_limit`, `:1175`,
  `compiled_prism.rs:109`). For the single letter `b`, the prism expansion
  returns the b-initial canonical syllable codes (`bat`, `bun`, `bou`, `bo`, …),
  all of which pass `has_code` — so the `b` prefix survives as **many** fetch
  specs. (Direct evidence the single-letter expansion is real: Yune reproduces
  the oracle's `n`→50-char flood, pinned by `m21_nri` — a bare consonant must be
  expanding to same-initial codes for that to happen.)
- The fetched characters are then admitted by
  `original_code_allows_prefix_fallback` (`:2881`):
  `normalized == lookup || (lookup.len() == 1 && normalized.starts_with(lookup))`.
  The `starts_with` disjunct fires **only for a single-letter prefix**, so `b`
  admits any b-initial code; a multi-char prefix like `be` takes the exact-equal
  disjunct and admits **only** exact `be`-syllable codes (啤 `be1`, 唄 `be6`),
  *not* all `be`-initial. (The v1 plan wrongly said `be` admits `be`-initial.)

So the flood is the **single-letter** prefix expansion (here `b`), not `be`.
Whether it is an oracle **divergence** for `beingo` is **not yet established** —
the oracle emits the analogous flood for `m` in some configs (Oracle Evidence)
and our `beingo`/`ngohaig` captures are page-0-only. Phase 1 must instrument the
exact specs `valid_lookup_prefixes` emits for `beingo`, and Phase 0 must capture
the full oracle output, before this is treated as a bug. The `starts_with` branch
cannot simply be deleted (it is oracle-correct for `nri`).

### Defect B — the per-fetch cap hides less-common exact characters (confirmed core bug)

This is the **solid, oracle-confirmed** part of M58: the oracle emits the full
exact-syllable set (checked-in `ng`→19 chars, `hou6`→13 chars per single toned
code; never two), so capping Yune's exact leading-parse output at two per code
makes valid characters unreachable regardless of any flood.
Independently of the flood, `bounded_candidates_for_lookup_codes` caps each
fetch code:

- `per_fetch_cap = MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE` (**= 2**,
  `translator/mod.rs:36`) when `bound_expansion` is true; the inner exact-lookup
  loop does `emitted_for_fetch_code += 1; if emitted_for_fetch_code >=
  per_fetch_cap { break; }` (`mod.rs:2274–2277`). So each toned code (`bei2`,
  `bei6`, `bei1`, `bei3`) contributes **at most two** characters; 畀 (the third
  `bei2` character) is dropped at emission.
- The candidates are then `pending.sort_by(...)` ordered by
  `consumed_input_len` **descending** (`mod.rs:2288`), so the exact `bei*` codes
  already sort ahead of the `b`/`be` flood. **Removing the flood does not surface
  畀** — only lifting/raising the per-fetch cap for the exact leading-parse codes
  does.
- **Path caveat (critical for testing).** `bound_expansion =
  bounds_compact_fallback_expansion()` is
  `matches!(self.storage, TableStorage::Compact(_)) && self.prism_payload.is_some()`
  (`mod.rs:1029–1031`) — i.e. the cap fires **only on the compiled product path**
  (Compact table + prism). On the default Owned `StaticTableTranslator` used by
  most unit tests, `per_fetch_cap = usize::MAX` and 畀 emits fine, so the bug
  **does not reproduce there**. Any regression test for Defect B must exercise
  the compiled/deployed path (Compact+prism, e.g. via the CLI frontend over a
  deployed schema), or it will pass without ever hitting the cap.

Relevant symbols:

- `translator/mod.rs`: `bounded_candidates_for_lookup_codes`,
  `prefix_fallback_candidates`, `valid_lookup_prefixes`,
  `original_code_allows_prefix_fallback` (`:2881`),
  `complete_syllable_prefix_count`, `is_completion_candidate_view_allowed`,
  `bounds_compact_fallback_expansion` (`:1029`),
  `MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE` (`:36`), the per-fetch break
  (`:2274`), and the `consumed_input_len`-descending sort (`:2288`).
- Schema toggles: `apps/yune-web/public/schema/jyut6ping3.schema.yaml` and
  `jyut6ping3_mobile.schema.yaml` (`prefix_fallback`, `enable_completion`,
  `prediction_never_first`).

## Oracle Evidence (the target behavior)

Confirmed against the checked-in TypeDuck v1.1.2 oracle captures under
`crates/yune-core/tests/fixtures/typeduck-v1.1.2/` (captured from
TypeDuck-HK/librime `v1.1.2` commit `74cb52b`, schema commit `1bed1ae`):

- `jyut6ping3-fork-parity-01-real-dictionary-fuzzy.json` — input `m` → 唔
  (`m4`) and 五 (fixture comment `ng5`; 五 also carries a colloquial `m5`
  reading). No flood of unrelated `m`-initial characters. **Caveat:** this
  capture is `is_last_page:false` (a partial leading page), so treat the exact
  `m` set as unconfirmed until Phase 0 re-captures it paginated.
- `jyut6ping3-windows-boundary-ngohaig.json` — input `ngohaig` → 我係個
  (`ngo5hai6go3`), 我係, 我喺, 我 (`ngo5`). Exact sentence/word matches plus the
  legitimate leading-syllable commit 我; no `ng`/`g`-initial fuzzy.

**Counter-example — the oracle *does* flood for an unparseable head.** The same
fixture family also pins `nri` with correction off
(`jyut6ping3-m14-completion-correction.json`, `correction_default`): the oracle
returns 我 你 外 能 內 呢 男 女 安 屋 愛 案 眼 呀 … (`page_size` 50,
`is_last_page` false) — a full same-initial completion flood. This is pinned
green today by `m21_nri_prefix_fallback_matches_typeduck_v112_real_dictionary_goldens`
([`cantonese_parity.rs:1409`](../../../crates/yune-core/tests/cantonese_parity.rs)).
So the target is **not** "no fuzzy ever". Two things the oracle always does, and
one it does *conditionally*:

1. **Full exact set first (always).** `ng` → **19 candidates, all `ng`-syllable
   exact characters** (五 `ng5`, 午, 誤 `ng6`, 吳 `ng4`, 伍, 吾, 悟, 晤 …),
   `is_last_page:true`; `hou` → 43–46 exact `hou*`/`ho*`. The oracle never
   truncates the exact set to two per code — **direct evidence for Leg B** (the
   `MAX_..._PER_FETCH_CODE = 2` cap is an under-count).
2. **Flood for an unparseable head (always).** `nri` correction-off → 50-char
   `n`/`ng`-initial flood (`is_last_page:false`), pinned by `m21_nri`. Bare `n`
   floods the same way. This must be **kept**.
3. **Flood for a *complete* syllable too — conditionally (the falsifier).** `m`
   in `jyut6ping3-m21-closeout.json` case 2 (variant `default_combined`,
   `page_size` 50) → **20 exact `m`/`ng` candidates then a 30-candidate
   cross-final `m*` flood** (mou/min/ming/man/mei/maai/mong/muk…,
   `is_last_page:false`) — yet the *same* `m` in `fork-parity-01` is exact-only.
   So "complete leading parse → exact only" is **false in general**: a
   same-initial completion tail can follow the exact set depending on
   options/page-fill.

| Head state | Oracle exact set | Same-initial flood? |
| --- | --- | --- |
| Complete syllable (`ng`, `ne`, `hou`, `beingo`→`bei`) | full, first, uncapped | **config-dependent** — capture decides (`m` shows it can appear) |
| Incomplete / not a syllable (`n`, `nri`) | n/a | yes — **kept** (`m21_nri`) |

**Consequence for the design.** Leg B (emit the full exact set) is
unconditionally right — 畀 must be reachable. Leg A (suppress the flood) is only
correct where the **full paginated capture of `beingo` with the product's real
options** shows the oracle does *not* emit the same-initial tail there. Do not
infer it from the page-0-only `ngohaig` fixture below or from the v1 "no fuzzy"
assumption; the `m` case shows that inference is unsafe. A global change to the
single-letter `starts_with` branch would break `m21_nri` and is rejected.

Direct same-input comparison on this machine (`ngohaig`, `jyut6ping3_mobile`) —
**page 0 only** (`is_last_page:false`; later pages not yet captured, so the
"exact only" claim is provisional pending Phase 0):

| Source | Candidate set (page 0) |
| --- | --- |
| Oracle (v1.1.2) | 我係個 · 我係 · 我喺 · 我 (exact only, page 0) |
| Yune (current) | 我係個 · 我係 · 我喺 · 我 · then a flood: 俄 餓 娥 鵝 卧 猗 · 五 午 · **你**(nei5) **能**(nang4) **男**(naam4) **女**(neoi5) **內**(noi6) **呢**(ne1) |

Yune reproduces the oracle's exact matches first, then appends `n`/`ng`-initial
characters with unrelated finals (`nei`/`nang`/`naam`/`neoi`/`noi`/`ne`) that
the oracle never shows. The fix is to stop at the exact set.

The checked-in fixtures captured only the leading page(s). Phase 0 must capture
the **full** oracle candidate list (all pages) for `beingo`, `ngohaig`, and the
`諮議局` input from TypeDuck v1.1.2 (harness:
`scripts/capture-typeduck-jyutping.ps1`, Windows) so the fix is asserted against
the complete oracle set, not an inferred one.

## Current Root-Cause Hypothesis

1. Prefix-fallback is meant to let a user commit the **oracle-recognized
   leading parse** of a multi-syllable input (e.g. `bei` of `beingo` → 畀/俾…).
2. **Defect B (cap) — confirmed core:**
   `MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE = 2` truncates each toned code
   on the compiled product path, so less-common exact characters (畀, third
   `bei2`) are never emitted. This is *not* positional "burying" — the sort
   already places exact `bei*` ahead of the flood; the cap drops 畀 at emission.
   The oracle emits the full exact set (`ng`→19, `hou6`→13), so this is a
   definite divergence.
3. **Defect A (flood) — contingent:** for a complete-parse head,
   `valid_lookup_prefixes` still admits the single-letter prefix expansion (via
   the prism + `starts_with`), producing a same-initial flood. Whether the oracle
   shows this flood for `beingo` is **unconfirmed** — it *does* for `nri`
   (unparseable head, kept) and *sometimes* for a complete syllable (`m` case).
   Phase 0's full capture decides whether this is a divergence to fix or oracle
   behavior to match.
4. Consequence: fixing Defect B alone makes 畀 reachable. Leg A is added only if
   the capture confirms the flood is a divergence.

Falsify or confirm each step before changing behavior; verify against the
**full paginated** TypeDuck `v1.1.2` oracle output (product options) for the same
inputs, and confirm Defect B on the compiled path (it does not reproduce on Owned
storage).

## Proposed Fix

`prefix_fallback` is **Yune-owned behavior, not a TypeDuck schema setting.** Yune
auto-enables it for the TypeDuck jyut6ping3 profile
([`schema_install.rs:332`](../../../crates/yune-rime-api/src/schema_install.rs):
`.unwrap_or(is_typeduck_jyut6ping3_profile)`); the upstream TypeDuck schema at
commit `1bed1ae` has no `prefix_fallback` key. So the fix is to **constrain an
over-broad Yune compatibility shim** to match the TypeDuck oracle — not to fix a
misread YAML feature.

The repair has **two legs of differing confidence**. Leg B is unconditional; Leg
A is gated on the Phase 0 capture. Sequence them: land Leg B first (it fixes the
user-facing 畀 bug on its own), then decide Leg A from the capture.

**Leg B (do first, unconditional) — emit the full exact leading-parse set.**
For the recognized leading-parse fetch codes, raise the per-fetch cap so the full
exact set matches the oracle (the third `bei2` character 畀 must emit).
**Default: bounded, not unlimited.** Raise the cap for leading-parse codes to the
oracle-observed max, not `usize::MAX` — the fixtures bound it: the max exact set
for a single toned code among complete-syllable inputs is **13** (`hou6`; `ng6`
is 9), and the global max across multi-syllable prediction contexts is **26**
(`san1`). A bounded cap of ~16 covers every observed complete-syllable exact set;
budget up to ~30 only if the same code path also serves multi-syllable
prediction. Fully exempting leading-parse codes is a fallback if the capture
shows a larger set, but the **Track B latency ratchet is the decider** (Win Bars
names the command) — a bounded raise is the latency-safe default. This defect
reproduces **only on the compiled Compact+prism path**, so the regression test
must run there (Defect B path caveat).

**Leg A (capture-gated) — suppress the flood only where the oracle does.**
*Only if* the Phase 0 full paginated `beingo` capture (product options) shows the
oracle does **not** emit the same-initial tail: drop the strictly-shorter
*degenerate* (non-syllable) leading prefix expansion **when the composition has a
complete-syllable leading parse**. If the capture shows the oracle *does* emit the
tail (as `m` does in one config), Leg A is **out of scope** — Yune's flood is not
a divergence, and only Leg B is the fix.

- **Leading-parse boundary — reuse the lookup path, not the segmentor.** Segment
  tags are whole-input type labels (`abc`/`raw`/…), *not* syllable boundaries.
  Decide "does this composition have a complete-syllable leading parse, and what
  is it" by reusing `valid_lookup_prefixes` (`translator/mod.rs:2328`) /
  `sentence_lookup_specs` (`:1164`) — prism `lookup_canonical_codes_with_limit`
  (`compiled_prism.rs:109`) gated by `storage.has_code` — the same path that
  yields 我 for `ngohaig`. A non-empty `valid_lookup_prefixes` result whose
  longest surviving prefix is shorter than the whole input *is* the leading
  parse; `prefixes[0]` (longest-first) is it. Do **not** use
  `context.segment_tags`, and do **not** reuse `complete_syllable_prefix_count`
  (that is a prediction-view classifier, `:2862`, not segmentation).
- `beingo`: leading parse `bei` → drop the single-letter `b` expansion, keep the
  `bei*` codes. `ngohaig`: leading parse `ngo` → drop the `n`/`ng` expansion.
  (`ng` *is* a syllable in isolation, so an "is-a-syllable" test would wrongly
  keep it — the criterion is the lookup-path parse, not isolated syllable-hood.)
- `nri`: **no** complete-syllable leading parse (`valid_lookup_prefixes` yields
  no shorter surviving parse) → the single-letter `n` expansion stays → flood
  emitted, matching `m21_nri`. **Do not touch this path.**

Do **not** globally restrict `original_code_allows_prefix_fallback`'s
single-letter `starts_with` branch to "entire input is one letter" — that was the
v1 proposal and it breaks the `nri` golden (head `n` under a 3-letter input). The
conditional lives in prefix *selection* (which prefixes are offered), not in the
admit predicate.

Result: after Leg B, `beingo` emits the **full** exact `bei*` set including 畀
(reachable). After Leg A (if in scope), the `b` flood is gone and the output
matches the full `beingo` capture; `nri` still floods either way.

If Leg A is in scope, assert it at the **prefix-spec level**, not just on final
candidates: the Phase 1 diagnostic records the specs `valid_lookup_prefixes`
emits, and the test asserts `beingo` emits no single-letter `b` expansion (but
does emit `bei`), `ngohaig` emits no `n`/`ng` expansion (but does emit `ngo`),
and **`nri` still emits the `n` expansion**.

Completion sub-question (Scope): decide, with the oracle capture, whether
`enable_completion` should inject longer `zi`-prefixed syllables ahead of exact
`zi` characters, or be suppressed. Do not change completion behavior without a
TypeDuck `v1.1.2` capture that shows the expected order.

## Decided Calls

- **No ABI widening.** No `RimeApi`/profile-slot/`yune_web_*` change.
- **Oracle-first.** Expected candidate order comes from TypeDuck-HK/librime
  `v1.1.2` on the `jyut6ping3` profile, captured into a checked-in fixture —
  never derived from Yune's current output.
- **Oracle floods are conditional, not "no fuzzy ever."** It floods for an
  unparseable head (`nri`) and *sometimes* for a complete syllable
  (`m` in `m21-closeout` case 2 → exact + 30-char `m*` tail). So the exact-only
  claim for a complete parse is **option/page-dependent** and must be captured
  full-page, never assumed. Any change must keep `m21_nri` green.
- **Leg B is the confirmed core; Leg A is capture-gated.** Lifting the per-fetch
  cap (Leg B) makes 畀 reachable and is unconditional. Suppressing the flood
  (Leg A) is done **only if** the full `beingo` capture shows the oracle emits no
  same-initial tail; otherwise Yune's flood is not a divergence and Leg A is
  skipped.
- **Leg B raise is bounded.** Raise the cap to the oracle-observed max
  (~13 per single toned code, ~26 across multi-syllable), not `usize::MAX`; the
  Track B ratchet decides the bound.
- **Defect B is compiled-path-only.** Reproduce and test it on the Compact+prism
  product path; Owned-storage unit tests do not trip the cap.
- **Product lane only.** This is the TypeDuck/Jyutping product path; the
  upstream `luna_pinyin` Track A lane is out of scope unless a fixture shows the
  same defect there.

## Win Bars

M58 closes when:

1. **(Leg B, unconditional) 畀 is reachable for `beingo`** — the full exact
   `bei*` set (including the third `bei2` = 畀) is emitted, matching the oracle's
   full exact set, on the compiled Compact+prism product path (not Owned
   storage). The cap raise is **bounded** (per Leg B sizing), and the exact-set
   count matches the `beingo` capture.
2. **The `beingo` (and `ngohaig`) full paginated oracle captures exist** (all
   pages, product options, `captured_all_pages:true`) and the Leg A disposition
   is recorded: either (a) the oracle shows no same-initial tail → Leg A landed
   and Yune emits no single-letter `b` flood; or (b) the oracle *does* show the
   tail → Leg A is out of scope and this is documented, with Yune's output still
   matching the capture.
3. For the `諮議局` / `zi` inputs (pinned by exact ASCII key sequence), Yune's
   candidate set matches the full oracle capture (completion ordering per Phase 3).
4. **The `nri` incomplete-head flood is preserved.**
   `m21_nri_prefix_fallback_matches_typeduck_v112_real_dictionary_goldens`
   (`cantonese_parity.rs:1409`) stays green.
5. Oracle-driven candidate tests cover the reported inputs, the `nri`
   incomplete-head case, and one non-Cantonese control; they assert the candidate
   set/order from captured TypeDuck bytes (full pages), not from Yune. Defect-B
   coverage runs on the compiled product path.
6. **No Track B latency regression, re-proven by re-running the standing native
   ratchet** — the `benchmark-native-rime-inprocess` command in
   [roadmap.md §Current Guardrails](../../roadmap.md) with its
   `-TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`,
   `-TrackAThresholds …/m55-thresholds.csv -FailOnRegression` (macOS:
   `scripts/benchmark-native-rime-inprocess-macos.sh`) — plus existing
   `cantonese_parity` rows still passing. Mandatory because Leg B changes the
   fallback set size.
7. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   and the focused tests pass.

Note: Leg A being out of scope (case 2b) is a **valid close**, not a failure —
Leg B alone fixes the user-facing bug. Close partial/no-go only if 畀 cannot be
made reachable within the Track B latency ratchet, or if matching the full oracle
capture would regress a genuine oracle-backed view (leading-syllable commit,
completion, or the `nri` flood).

## Scope

In scope: the per-fetch cap `MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE` for
leading-parse codes (Leg B, unconditional); `valid_lookup_prefixes` /
`original_code_allows_prefix_fallback` prefix selection (Leg A, **capture-gated**
— only if Phase 0 shows the flood is an oracle divergence); the full paginated
oracle capture + tests for the reported inputs; re-proving the Track B latency
ratchet; a decision on completion ordering in long compositions.

Out of scope: ABI changes; `luna_pinyin` Track A behavior; performance
rebaselining (beyond re-proving the standing Track B ratchet holds); broad
translator refactors; the `nri`/incomplete-head completion flood (oracle-correct,
preserved).

## Phases

### Phase 0: Reproduce and freeze
- [ ] Record the failing Yune candidate order (paged, on the compiled
      Compact+prism product path) for `beingo`, `諮議局`, and the intermediates
      `be`/`bein`/`being` and `zi`, plus the single-letter `b` control, under
      `docs/reports/evidence/m58-jyutping-exact-before-fuzzy/phase-0/`. Capture
      the `beingo` per-toned-code fingerprint (exactly two of each `bei*`, 畀
      absent) so Defect B is pinned before the fix, and confirm 畀 *is* present
      for bare `bei` (path difference from the Diagnostic Evidence).
- [ ] **Pagination groundwork is implemented in the probe — compile-verify and
      confirm it on Windows.** `scripts/oracle-rime-probe.cs` previously read
      `RimeGetContext` once (leading page only), so the checked-in fixtures are
      single-page (e.g. `ngohaig` shows 4 with `is_last_page:false`).
      `CaptureWithIdentity` now loops on `Page_Down` (`0xff56`), re-reading the
      context per page and emitting `pages` / `all_candidates` /
      `captured_all_pages` (the page-0 fields — `selected_candidates`,
      `page_size`, `is_last_page` — are kept for back-compat). This code is
      **untested off-Windows** (needs `csc`/`Add-Type` + the librime DLL): the
      Windows session must confirm it compiles and captures full lists before
      the capture is trusted. `capture-typeduck-jyutping.ps1` needs no change —
      it serializes whatever `CaptureWithIdentity` returns. **Note:** there is no
      `-Fixture M58`/preset mode wired into `capture-typeduck-jyutping.ps1` for
      these inputs — the executor must add one (preferred) or record the exact
      internal capture/serialize invocation used for each input, so the capture
      is reproducible rather than ad-hoc.
- [ ] With the paginated harness, capture the **full** TypeDuck `v1.1.2`
      candidate list (all pages) for `beingo`, `ngohaig`, `b`, `m`, `ng`, `zi`,
      **`nri` (correction off — re-capture paginated to confirm the flood is the
      full oracle set, not just the checked-in page)**, and the exact ASCII key
      sequence for `諮議局` (pin the literal keystrokes in the fixture, not the
      rendered characters), into checked-in oracle fixtures. This is a
      Windows-session task (the capture runs against TypeDuck-HK/librime
      `v1.1.2`). **Reject any captured row where `captured_all_pages != true` or
      `pagination_error` is present** — that means the pager did not reach the
      last page and the list is incomplete. (Exception: a legitimately empty
      candidate list has no pages to turn; none of the M58 inputs above are
      empty, so an empty result here is itself a capture error to investigate,
      not an accepted row.) If the capture is unavailable, block the milestone —
      do not proceed on Yune-defined expectations.
- [ ] **Capture with the product's real options and reach the last page.** The
      same input can be exact-only or flood depending on `page_size`,
      `enable_completion`, and the option variant — verified: `m` is exact-only in
      `fork-parity-01` but emits a 30-char cross-final `m*` completion tail in
      `jyut6ping3-m21-closeout.json` case 2 (`default_combined`, `page_size` 50).
      So capture `beingo`/`ngohaig` under the **shipping** `jyut6ping3_mobile`
      options and page all the way to `is_last_page:true`; a page-0-only capture
      cannot be used to conclude the oracle "shows no flood." This capture is what
      decides whether Leg A is in scope (Proposed Fix).

### Phase 1: Instrument prefix selection
- [ ] Add a dev-only diagnostic (test helper or metric) that reports, per
      input, the prefixes `valid_lookup_prefixes` emits and which candidates
      each admits, so the partial-prefix contribution is visible.

### Phase 2: Fix the per-fetch cap (Leg B, do first) then the flood (Leg A, gated)
- [ ] **Leg B (unconditional).** Raise `MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE`
      for the recognized leading-parse fetch codes so the full exact set (incl 畀)
      emits, matching the oracle capture. **Bounded, not `usize::MAX`:** raise to
      the oracle-observed max (fixtures bound it at ~13 per single toned code,
      ~26 across multi-syllable prediction) — pick a bound (~16, up to ~30) the
      Track B ratchet holds. Keep the cap on non-leading-parse codes unless the
      capture says otherwise.
- [ ] **Leg A (only if Phase 0 says so).** If the full `beingo` capture shows the
      oracle emits **no** same-initial tail: for a composition with a
      complete-syllable leading parse — decided by reusing `valid_lookup_prefixes`
      / `sentence_lookup_specs` (the lookup path that yields 我 for `ngohaig`),
      **not** `context.segment_tags` and **not** `complete_syllable_prefix_count`
      — drop the strictly-shorter single-letter prefix expansion so its flood is
      not offered. **Leave the single-letter path untouched when there is no
      complete leading parse** (`nri` → `n`), so its flood still matches the
      oracle. If the capture shows the oracle *does* emit the tail, skip Leg A and
      document it.
- [ ] Add real-path tests. Always: `beingo` emits 畀 (Leg B) on the compiled
      Compact+prism product path (assert 畀 absent pre-fix, present post-fix —
      Owned storage does not trip the cap); `nri` still matches its incomplete-
      head flood golden; one non-Cantonese control. If Leg A landed: `beingo`
      matches the full capture with no single-letter `b` flood, and the prefix-spec
      assertion (Proposed Fix). Assert candidate sets from the full captured
      TypeDuck bytes, never from Yune.

### Phase 3: Completion ordering decision
- [ ] With the oracle capture, decide whether completion candidates
      (`zi` → `zing`/`zik`) precede or follow exact-syllable characters in a
      first-syllable/long-composition view; implement to match, with a test.

### Phase 4: Re-verify and close
- [ ] Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
      warnings`, focused tests, and **re-prove the Track B latency ratchet**
      (the `benchmark-native-rime-inprocess` command with `-TrackBInputs …`,
      `-TrackAThresholds …/m55-thresholds.csv -FailOnRegression`; macOS uses
      `scripts/benchmark-native-rime-inprocess-macos.sh`). Lifting the per-fetch
      cap (Leg B) changes fallback set size, so this gate is mandatory, not
      optional.
- [ ] Write `docs/reports/evidence/m58-jyutping-exact-before-fuzzy/` with
      before/after candidate captures and the oracle comparison.
- [ ] Update roadmap/requirements/milestone-history on closeout; move this plan
      to `plans/completed/`.

## Review Checklist For Claude

- Is Leg B (the confirmed core) tested on the **compiled Compact+prism product
  path**? Owned storage sets `per_fetch_cap = usize::MAX` and passes without
  reproducing the bug — a green Owned-storage test is a false pass here.
- Is the Leg B cap raise **bounded** to the oracle-observed max, not `usize::MAX`?
- Is Leg A treated as **capture-gated**? The `m` falsifier shows a complete
  syllable can flood; so "oracle shows no `b` flood for `beingo`" must come from a
  **full-page** capture with product options, not from the page-0-only `ngohaig`
  fixture or the v1 assumption. If the capture shows a tail, Leg A is skipped.
- If Leg A lands, does the flood suppression stay **conditional** so `nri`
  (`m21_nri`) still floods? A global change to the single-letter `starts_with`
  branch breaks it.
- Does "leading parse" mean the **lookup-path** parse (`valid_lookup_prefixes` /
  `sentence_lookup_specs`), not `context.segment_tags` (whole-input type tags,
  no syllable boundaries) and not `complete_syllable_prefix_count` (a
  prediction-view classifier)?
- Are expectations captured from TypeDuck `v1.1.2` (**all pages**,
  `captured_all_pages:true`, no `pagination_error`), not asserted from Yune?
- Was the Track B latency ratchet **re-run** after Leg B changed the fallback set
  size — not assumed to still hold?

## Non-Goals

- Do not change `luna_pinyin` Track A behavior.
- Do not change single-letter behavior in either direction without the oracle
  capture — match `b`/`m`/`ng`/`z` to what TypeDuck actually shows.
- Do not suppress the incomplete-head completion flood (`nri`); it is
  oracle-correct and pinned by `m21_nri`.
- Do not assert candidate order from Yune; assert it from the TypeDuck oracle.
