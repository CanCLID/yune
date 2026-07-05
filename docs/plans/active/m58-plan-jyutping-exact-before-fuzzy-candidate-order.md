# M58 Jyutping Exact-Before-Fuzzy Candidate Order Repair Plan

> **For agentic workers:** execute one phase at a time. This plan is written as
> a review packet: before implementing, have another reviewer challenge the
> root-cause chain, the fix, and the oracle gates. The named product target is
> the TypeDuck `jyut6ping3` profile against TypeDuck-HK/librime `v1.1.2`; that
> fork is the correctness oracle for every candidate-order claim here.

> **Status:** Draft for review (v2, amended 2026-07-05 after review). -
> **Track:** Engine behavioral correctness (TypeDuck/Jyutping product lane). -
> **Created:** 2026-07-05. - **Type:** bug-fix milestone. No ABI widening, no new
> performance claim.

> **Amendment note (v2).** Review found the v1 root-cause chain wrong in two
> load-bearing ways, both re-verified against the code and the checked-in
> fixtures: (1) 畀 is not buried by the flood — the fallback candidates already
> sort `consumed_input_len`-descending so the exact `bei` codes sort *ahead* of
> the `b`/`be` flood; 畀 is cut by a **per-fetch cap of 2**
> (`MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE`) that emits at most two
> characters per toned code, and only on the compiled product path. (2) The
> oracle does **not** universally suppress same-initial fuzzy: with correction
> off it *does* emit a same-initial completion flood for an unparseable head
> (`nri` → 我/你/外/能/內/呢/男/女…, pinned green by the existing
> `m21_nri_prefix_fallback_matches_typeduck_v112_real_dictionary_goldens`
> fixture). The target behavior is **parse-state-conditional**, and the v1 "no
> fuzzy ever" goal would regress that golden. Sections below carry the corrected
> design.

**Goal:** Match the TypeDuck/librime `v1.1.2` oracle candidate output for
multi-syllable Jyutping composition. The oracle is **parse-state-conditional**:

- When the composition has a **complete leading-syllable parse** (`m`;
  `beingo`→`bei`; `ngohaig`→`ngo`), the oracle emits exact word/sentence/syllable
  matches only — including the legitimate leading-syllable commit (我 = `ngo5`
  for `ngohaig`) — and **no** same-initial fuzzy characters.
- When the head is **incomplete / not a syllable** (`nri`→`n`, correction off),
  the oracle emits the same-initial completion flood (我/你/外/能/內/呢/男/女…).
  This is real oracle behavior, pinned green by the existing `m21_nri` fixture,
  and must not be regressed.

Two independent Yune defects break the complete-leading-parse case: (a) Yune
appends a same-initial fuzzy flood even when the leading syllable is complete
(`b`-initial 不/本/部/報 for `beingo`; `n`/`ng`-initial 你/能/男/女 for
`ngohaig`) — the oracle shows none there; and (b) a per-fetch cap of 2 truncates
each exact toned code so less-common exact characters (畀 = `bei2`, the third
`bei2` character) are never emitted. Fix **both** so the complete-leading-parse
candidate set/order matches the oracle, **without** regressing the `nri`
incomplete-head flood. This is a comparability/correctness repair, not a
reordering-of-fuzzy exercise.

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

1. **Same-initial flood.** From page 1 onward the list mixes in `b`-initial
   characters with unrelated finals (`bat`/`bun`/`biu`/`bou`/`baa`/`bo`/`bin`/
   `bit` → 不/本/表/部/巴/波/邊/別…) that are **not `bei`**. The two `be`-syllable
   characters 啤 (`be1`) / 唄 (`be6`) are the exact matches of the `be` leading
   parse, *not* part of that flood — it is the single-letter `b` prefix that
   pulls in the rest. The oracle shows none of the `b`-final-mismatch flood for a
   complete leading parse.
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

There are **two independent defects**. A fix that addresses only one leaves the
bug half-open (remove the flood and 畀 is still capped; lift the cap and the
flood still appears).

### Defect A — the same-initial flood (which prefixes the fallback admits)

- `valid_lookup_prefixes(lookup_code)` emits **every character-boundary
  prefix** of the input, longest-first. For `beingo` that includes the
  degenerate partial-syllable prefixes `bein`, `bei`, `be`, and the
  single-letter `b`.
- `original_code_allows_prefix_fallback(raw_code, lookup_code)` is
  `normalized == lookup || (lookup.len() == 1 && normalized.starts_with(lookup))`
  (verified at `translator/mod.rs:2881`). The first disjunct is **exact-equal**;
  the `starts_with` disjunct fires **only for a single-letter prefix**. So:
  - the 1-char `b` prefix admits **any `b`-initial code** (`bat`, `bun`, `bou`,
    `biu`, `baa`, `bo`, `bin`, …) — this is the flood;
  - the 2-char `be` prefix admits **only exact `be`-syllable codes** (啤 `be1`,
    唄 `be6`), *not* all `be`-initial codes. (The v1 plan wrongly said the `be`
    prefix admits `be`-initial codes; it does not — multi-char prefixes take the
    exact-equal disjunct.)

So the flood is driven by the **single-letter** `b` prefix's `starts_with`
branch, not by `be`. This branch fires both mid-composition (the multi-syllable
bug) and for a whole-input single letter (`b` alone). **Neither is assumed
correct — both are measured against the oracle**, and the oracle is
parse-state-conditional (Oracle Evidence): for `beingo` the head `bei` is a
complete parse and the oracle shows no `b` flood, so the flood is a divergence
*there*; for an unparseable head (`nri`) the oracle *does* flood, so the
`starts_with` branch cannot simply be deleted.

### Defect B — the per-fetch cap hides less-common exact characters

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
So the target is **not** "no fuzzy ever":

| Head state | Example | Oracle candidate set |
| --- | --- | --- |
| Complete leading syllable | `m`, `ng`, `ne`, `beingo`→`bei`, `ngohaig`→`ngo` | exact matches only, no same-initial fuzzy |
| Incomplete / not a syllable | `n`, `nri`→`n` (correction off) | same-initial completion flood (kept) |

The same `jyut6ping3-m14-completion-correction.json` also pins the **complete
syllable** `ng` → **19 candidates, all `ng`-syllable exact characters** (五 `ng5`,
午 `ng5`, 誤 `ng6`, 吳 `ng4`, 伍, 吾, 悟, 晤 …), `is_last_page:true` — no
`n`/`ng`-initial flood, and *not truncated to two*. This checked-in fixture is
direct evidence for **both** M58 legs: a complete syllable shows exact-only
(Leg A), and the oracle emits the **full** exact-syllable set, so a per-fetch cap
of 2 is an under-count (Leg B). Contrast the bare `n` case (not a syllable) in
the same fixture → 50-candidate flood, `is_last_page:false`.

The M58 fix must **suppress the flood only where the head is a complete leading
parse**, and must leave the `nri` flood intact. A global change to the
single-letter `starts_with` branch (as the v1 plan proposed) would break the
`m21_nri` golden and is rejected.

Direct same-input comparison on this machine (`ngohaig`, `jyut6ping3_mobile`):

| Source | Candidate set |
| --- | --- |
| Oracle (v1.1.2) | 我係個 · 我係 · 我喺 · 我 (exact only) |
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
2. **Defect A (flood):** for a composition whose head is a complete leading
   parse, `valid_lookup_prefixes` still offers the degenerate single-letter `b`
   prefix, and `original_code_allows_prefix_fallback`'s single-letter
   `starts_with` branch admits all `b`-initial codes — a flood the oracle does
   not show for that head state. (For an unparseable head like `nri` the oracle
   *does* flood, so the branch is correct there and must be preserved.)
3. **Defect B (cap):** `MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE = 2`
   truncates each toned code on the compiled product path, so less-common exact
   characters (畀, third `bei2`) are never emitted. This is *not* a positional
   "burying" — the sort already places exact `bei*` ahead of the flood; the cap
   drops 畀 at emission regardless of the flood.
4. Consequence: even after Defect A is fixed, 畀 stays unreachable until Defect B
   is fixed. Both must land together.

Falsify or confirm each step before changing behavior; verify against the
TypeDuck `v1.1.2` oracle order for the same inputs, and confirm Defect B on the
compiled path (it does not reproduce on Owned storage).

## Proposed Fix

`prefix_fallback` is **Yune-owned behavior, not a TypeDuck schema setting.** Yune
auto-enables it for the TypeDuck jyut6ping3 profile
([`schema_install.rs:332`](../../../crates/yune-rime-api/src/schema_install.rs):
`.unwrap_or(is_typeduck_jyut6ping3_profile)`); the upstream TypeDuck schema at
commit `1bed1ae` has no `prefix_fallback` key. So the fix is to **constrain an
over-broad Yune compatibility shim** to match the TypeDuck oracle — not to fix a
misread YAML feature.

The repair has **two legs** (one per defect). Both must land together, and both
are gated on the parse-state-conditional oracle model — the single-letter flood
is correct behavior for an unparseable head (`nri`) and must be preserved.

**Leg A — suppress the flood only when a complete leading parse exists.**
In `valid_lookup_prefixes` (or a guard at its call site), drop the strictly
shorter *degenerate* (non-syllable) leading prefixes **when the composition has
a complete-syllable leading parse**. The parse must be the one Yune's
sentence/segmentor already computes for the input (the same parse that yields 我
for `ngohaig`), **not** "any prefix that could be a syllable in isolation":

- `beingo`: leading parse `bei` exists → drop the degenerate `b` prefix. Keep
  `bei` (and any longer complete leading parse). Result: only exact `bei*` codes.
- `ngohaig`: leading parse `ngo` exists → drop `n`/`ng`. (`ng` *is* a syllable in
  isolation, so an "is-a-syllable" test would wrongly keep it and re-admit the
  flood — the criterion is the *parse*, hence the segmentor.)
- `nri`: **no** complete-syllable leading parse (`n`/`nr`/`nri` are not
  syllables) → the single-letter `n` prefix stays → the completion flood is
  emitted, matching the `m21_nri` oracle golden. **Do not touch this path.**

Do **not** globally restrict `original_code_allows_prefix_fallback`'s
single-letter `starts_with` branch to "entire input is one letter" — that was
the v1 proposal and it breaks the `nri` golden (whose head is `n` under a
3-letter input). The conditional lives in prefix *selection* (which prefixes are
offered), not in the admit predicate.

**Leg B — emit the full exact leading-parse set (fix the cap).**
For the complete-leading-parse fetch codes, raise/lift
`MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE` so the exact set matches the
oracle capture (the third `bei2` character 畀 must emit). Options, in order of
preference: (a) exempt the recognized leading-parse codes from the per-fetch cap
entirely; (b) if the Track B latency guard binds, cap at the oracle-observed max
per code rather than the fixed 2. **Re-prove the Track B latency guard either
way** — Leg A removes the `b`/`be` flood work, which should free budget, but the
ratchet must be re-run (Win Bars names the exact command). This defect only
reproduces on the compiled Compact+prism path, so the regression test must run
there (Diagnostic Evidence, Defect B path caveat).

Result: for `beingo`, Yune contributes only exact `bei`-syllable characters —
the **full** set, including 畀 — matching the oracle's exact-only output; no
`b`/`be` fuzzy; and `nri` still floods.

Assert the fix at the **prefix-spec level**, not just on final candidates: the
Phase 1 diagnostic records the specs `valid_lookup_prefixes` emits, and the test
asserts `beingo` emits no `b` spec (but does emit `bei`), `ngohaig` emits no
`n`/`ng` spec (but does emit `ngo`), and **`nri` still emits the `n` spec**.

Completion sub-question (Scope): decide, with the oracle capture, whether
`enable_completion` should inject longer `zi`-prefixed syllables ahead of exact
`zi` characters, or be suppressed. Do not change completion behavior without a
TypeDuck `v1.1.2` capture that shows the expected order.

## Decided Calls

- **No ABI widening.** No `RimeApi`/profile-slot/`yune_web_*` change.
- **Oracle-first.** Expected candidate order comes from TypeDuck-HK/librime
  `v1.1.2` on the `jyut6ping3` profile, captured into a checked-in fixture —
  never derived from Yune's current output.
- **Oracle is parse-state-conditional, not "no fuzzy ever."** Complete leading
  syllable → exact only; incomplete/unparseable head → same-initial completion
  flood (the `nri` golden). The fix suppresses the flood *only* in the first
  case. Any change must keep `m21_nri` green.
- **Single-letter views are not assumed correct.** Capture the oracle for
  `b`, `m`, `ng`, and `z` and match it. The checked-in `m` → 唔/五 (exact only,
  not all `m`-initial) shows a single letter that is itself a complete syllable
  gets exact-only; a single letter that is *not* a complete syllable and is the
  whole unparseable input (bare `b`?) needs its own capture — do not assume.
- **Two defects, both fixed together.** Leg A (conditional flood suppression) and
  Leg B (lift the per-fetch cap for leading-parse codes). Neither alone makes 畀
  reachable.
- **Defect B is compiled-path-only.** Reproduce and test it on the Compact+prism
  product path; Owned-storage unit tests do not trip the cap.
- **Product lane only.** This is the TypeDuck/Jyutping product path; the
  upstream `luna_pinyin` Track A lane is out of scope unless a fixture shows the
  same defect there.

## Win Bars

M58 closes when:

1. For `beingo`, Yune's candidate set **matches the full TypeDuck `v1.1.2`
   capture** — exact word/sentence/syllable matches only, no `b`-initial fuzzy —
   **and 畀 (third `bei2`) is present**, proving *both* legs landed (Leg A removed
   the flood; Leg B lifted the cap). Verified on the compiled Compact+prism
   product path, not Owned storage.
2. For the `諮議局` input (pinned by its exact ASCII key sequence), Yune's
   candidate set matches the oracle capture — no `z`-initial fuzzy ahead of /
   crowding out the exact `zi` characters (including 諮).
3. Single-letter input (`b`, `m`, `ng`) matches the oracle capture. The oracle
   shows exact-syllable-only for a single letter that is itself a syllable (`m` →
   唔/五); capture bare `b` (not a syllable) and match whatever it shows — do not
   assume.
4. **The `nri` incomplete-head flood is preserved.**
   `m21_nri_prefix_fallback_matches_typeduck_v112_real_dictionary_goldens`
   (`cantonese_parity.rs:1409`) stays green — the fix must not suppress the
   oracle-correct completion flood for an unparseable head.
5. Oracle-driven candidate tests cover both reported inputs, a complete-syllable
   single-letter case, the `nri` incomplete-head case, and one non-Cantonese
   control; they assert the candidate set/order from captured TypeDuck bytes, not
   from Yune. Defect-B coverage runs on the compiled product path.
6. **No Track B latency regression, re-proven by re-running the standing native
   ratchet** — the `benchmark-native-rime-inprocess` command in
   [roadmap.md §Current Guardrails](../../roadmap.md) with its
   `-TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`,
   `-TrackAThresholds …/m55-thresholds.csv -FailOnRegression` (macOS:
   `scripts/benchmark-native-rime-inprocess-macos.sh`) — plus existing
   `cantonese_parity` rows still passing.
7. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   and the focused tests pass.

Close partial/no-go if the full oracle capture shows TypeDuck itself emits the
same same-initial fuzzy set **for a complete-leading-parse head** (then that part
is not a bug), or if lifting the per-fetch cap for leading-parse codes cannot
hold the Track B latency ratchet, or if matching the oracle cannot be achieved
without regressing a genuine oracle-backed view (leading-syllable commit,
completion, or the `nri` flood).

## Scope

In scope: `valid_lookup_prefixes` / `original_code_allows_prefix_fallback`
prefix selection (Leg A, parse-state-conditional); the per-fetch cap
`MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE` for leading-parse codes (Leg B);
the prefix-fallback append order; oracle capture + tests for the reported inputs;
re-proving the Track B latency ratchet; a decision on completion ordering in long
compositions.

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

### Phase 1: Instrument prefix selection
- [ ] Add a dev-only diagnostic (test helper or metric) that reports, per
      input, the prefixes `valid_lookup_prefixes` emits and which candidates
      each admits, so the partial-prefix contribution is visible.

### Phase 2: Fix prefix selection (Leg A) and the per-fetch cap (Leg B)
- [ ] **Leg A.** For a composition that has a complete-syllable leading parse
      (per Yune's own segmentor, not "is-a-syllable-in-isolation" — so `ng`
      cannot sneak in for `ngohaig`), drop the strictly-shorter degenerate
      leading prefixes so the single-letter `starts_with` flood is not offered.
      **Leave the single-letter path untouched when there is no complete leading
      parse** (`nri` → `n`), so its flood still matches the oracle.
- [ ] **Leg B.** Raise/lift `MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE` for
      the recognized leading-parse fetch codes so the full exact set (incl 畀)
      emits, matching the oracle capture; keep the cap on non-leading-parse codes
      unless the capture says otherwise.
- [ ] Add real-path tests asserting Yune's candidate set matches the captured
      oracle for `beingo` (incl 畀), `ngohaig`, and `諮議局`, matches the
      single-letter oracle captures (`b`, `m`, `ng`), **and that `nri` still
      matches its incomplete-head flood golden**. The `beingo`/畀 (Defect B) test
      **must run on the compiled Compact+prism product path** — assert 畀 is
      absent on the pre-fix product build and present after — because Owned
      storage does not trip the cap.

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

- Are **both** defects addressed? Leg A (flood) alone leaves 畀 capped; Leg B
  (cap) alone leaves the flood. 畀 reachability proves both landed.
- Is Defect B tested on the **compiled Compact+prism product path**? Owned
  storage sets `per_fetch_cap = usize::MAX` and will pass without reproducing the
  bug — a green Owned-storage test is a false pass here.
- Does the flood suppression stay **conditional on the parse state**? `nri`
  (`m21_nri` golden) must still flood; a global change to the single-letter
  `starts_with` branch breaks it.
- Does "leading parse" mean the segmentor's parse, not "is-a-syllable-in-
  isolation"? (Otherwise `ng` re-admits the `ngohaig` flood.)
- Is completion the dominant contributor for some inputs (`zi`) rather than
  prefix-fallback? The fix must handle both without over-reaching.
- Are the expectations captured from TypeDuck `v1.1.2` (all pages,
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
