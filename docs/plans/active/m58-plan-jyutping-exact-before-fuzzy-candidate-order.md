# M58 Jyutping Exact-Before-Fuzzy Candidate Order Repair Plan

> **For agentic workers:** execute one phase at a time. This plan is written as
> a review packet: before implementing, have another reviewer challenge the
> root-cause chain, the fix, and the oracle gates. The named product target is
> the TypeDuck `jyut6ping3` profile against TypeDuck-HK/librime `v1.1.2`; that
> fork is the correctness oracle for every candidate-order claim here.

> **Status:** Draft for review. - **Track:** Engine behavioral correctness
> (TypeDuck/Jyutping product lane). - **Created:** 2026-07-05. - **Type:**
> bug-fix milestone. No ABI widening, no new performance claim.

**Goal:** Match the TypeDuck/librime `v1.1.2` oracle candidate output for
multi-syllable Jyutping composition. The oracle emits only exact
word/sentence/syllable matches — including the legitimate leading-complete-
syllable commit (e.g. 我 = `ngo5` for `ngohaig`) — and **does not emit
same-initial fuzzy characters at all**. Yune currently appends a flood of
same-initial fuzzy characters (`b`-initial 不/本/部/報 for `beingo`;
`n`/`ng`-initial 你/能/男/女 for `ngohaig`); remove that divergence so Yune's
candidate set and order match the oracle. This is a comparability/correctness
repair, not a reordering-of-fuzzy exercise.

## Problem Statement

Typing a multi-syllable word and picking characters one at a time is broken:
less-common exact-syllable characters are unreachable because same-initial
fuzzy characters crowd the candidate list.

Reproduced on this machine (`jyut6ping3_mobile`, `yune-cli frontend`, page
size 6), input `beingo` (畀我), paging with `=`:

| Page | Candidates (text/code) |
| --- | --- |
| 0 | `俾我`(bei2ngo5) `比`(bei2) `被`(bei6) `備`(bei6) `俾`(bei2) `悲`(bei1) |
| 1 | `秘`(bei3) `臂`(bei3) `卑`(bei1) · **`啤`(be1) `唄`(be6) `不`(bat1)** |
| 2 | **`本`(bun2) `表`(biu2) `部`(bou6) `報`(bou3) `巴`(baa1) `不過`(bat1gwo3)** |
| 3 | **`波`(bo1) `邊`(bin1) `保`(bou2) `班`(baan1) `變`(bin3) `別`(bit6)** |

From page 1 onward the list mixes in `be`- and `b`-initial characters
(`bat`/`bun`/`biu`/`bou`/`baa`/`bo`/`bin`/`bit`) that are **not `bei`**. The
character the user wants — 畀 (`bei2`), which *does* appear when the input is
just `bei` — is crowded out and never shown for `beingo`, so the user must type
`bei` alone, commit 畀, then type `ngo` separately.

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

The defect is which prefixes the fallback uses:

- `valid_lookup_prefixes(lookup_code)` emits **every character-boundary
  prefix** of the input, longest-first. For `beingo` that includes the
  degenerate partial-syllable prefixes `bein`, `bei`, `be`, and the
  single-letter `b`.
- `original_code_allows_prefix_fallback(raw_code, lookup_code)` has a special
  case: `normalized == lookup || (lookup.len() == 1 &&
  normalized.starts_with(lookup))`. For the 1-char prefix `b` this admits **any
  `b`-initial code** (`bat`, `bun`, `bou`, `biu`, `baa`, `bo`, `bin`, …), and
  the 2-char `be` prefix admits `be`-initial codes.

So the single-letter/partial-syllable prefixes pull in every same-initial
character. The single-letter `starts_with` rule fires both mid-composition
(the multi-syllable bug above) and for a whole-input single letter (`b` yields
`不`/`本`/`比`/`表`/`部`/`報`). **Neither is assumed correct — both are measured
against the oracle.** The oracle's `m` → 唔/五 (exact only, not all `m`-initial)
shows the whole-input single-letter flood is likely also a divergence, not a
feature; the fix constrains this Yune shim to whatever the oracle capture shows
for each single letter (`b`, `m`, `ng`, `z`).

Relevant symbols:

- `translator/mod.rs`: `bounded_candidates_for_lookup_codes`,
  `prefix_fallback_candidates`, `valid_lookup_prefixes`,
  `original_code_allows_prefix_fallback`, `complete_syllable_prefix_count`,
  `is_completion_candidate_view_allowed`.
- Schema toggles: `apps/yune-web/public/schema/jyut6ping3.schema.yaml` and
  `jyut6ping3_mobile.schema.yaml` (`prefix_fallback`, `enable_completion`,
  `prediction_never_first`).

## Oracle Evidence (the target behavior)

Confirmed against the checked-in TypeDuck v1.1.2 oracle captures under
`crates/yune-core/tests/fixtures/typeduck-v1.1.2/` (captured from
TypeDuck-HK/librime `v1.1.2` commit `74cb52b`, schema commit `1bed1ae`):

- `jyut6ping3-fork-parity-01-real-dictionary-fuzzy.json` — input `m` → **2
  candidates**: 唔 (`m4`), 五 (`m5`). Only exact `m`-syllable characters; no
  flood of `m`-initial fuzzy characters.
- `jyut6ping3-windows-boundary-ngohaig.json` — input `ngohaig` → 我係個
  (`ngo5hai6go3`), 我係, 我喺, 我 (`ngo5`). Exact sentence/word matches plus the
  legitimate leading-syllable commit 我; no `ng`/`g`-initial fuzzy.

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
2. `valid_lookup_prefixes` wrongly also offers **partial-syllable** prefixes
   (`b`, `be`) that are not syllable boundaries.
3. `original_code_allows_prefix_fallback`'s single-letter `starts_with` branch
   then admits all same-initial characters for the `b` prefix.
4. Those fuzzy characters are appended after the exact matches and fill the
   bounded limit, burying less-common exact-syllable characters.

Falsify or confirm each step before changing behavior; verify against the
TypeDuck `v1.1.2` oracle order for the same inputs.

## Proposed Fix

`prefix_fallback` is **Yune-owned behavior, not a TypeDuck schema setting.** Yune
auto-enables it for the TypeDuck jyut6ping3 profile
([`schema_install.rs:332`](../../../crates/yune-rime-api/src/schema_install.rs):
`.unwrap_or(is_typeduck_jyut6ping3_profile)`); the upstream TypeDuck schema at
commit `1bed1ae` has no `prefix_fallback` key. So the fix is to **constrain an
over-broad Yune compatibility shim** to match the TypeDuck oracle — not to fix a
misread YAML feature.

Preferred repair:

1. In `valid_lookup_prefixes` (or a guard at its call site), for a multi-code
   composition emit fallback prefixes only at the **oracle-recognized leading
   syllable parse** of the composition — not every character boundary, and not
   every prefix that merely *could* be a syllable in isolation. For `beingo`
   the leading parse is `bei` (and any longer complete leading parse), never
   `b`/`be`; for `ngohaig` the leading parse is `ngo`, never `n`/`ng`. Note `ng`
   is itself a valid syllable, so the criterion must be the *parse*, not
   "is-a-syllable" — otherwise `ng` sneaks the flood back in.
2. Restrict the single-letter `starts_with` branch in
   `original_code_allows_prefix_fallback` to the case where the **entire input**
   is that single letter — and only if the oracle capture shows that view is
   real (see Decided Calls; the checked-in `m` → 唔/五 says it may not be).
3. Result: `beingo` contributes only exact `bei`-syllable characters (the full
   set, including 畀), matching the oracle's exact-only output; no `b`/`be` fuzzy.

Assert the fix at the **prefix-spec level**, not just on final candidates: the
Phase 1 diagnostic records the specs `valid_lookup_prefixes` emits, and the test
asserts `beingo` emits no `b`/`be` spec and `ngohaig` emits no `n`/`ng` spec.

Completion sub-question (Scope): decide, with the oracle capture, whether
`enable_completion` should inject longer `zi`-prefixed syllables ahead of exact
`zi` characters, or be suppressed. Do not change completion behavior without a
TypeDuck `v1.1.2` capture that shows the expected order.

## Decided Calls

- **No ABI widening.** No `RimeApi`/profile-slot/`yune_web_*` change.
- **Oracle-first.** Expected candidate order comes from TypeDuck-HK/librime
  `v1.1.2` on the `jyut6ping3` profile, captured into a checked-in fixture —
  never derived from Yune's current output.
- **Single-letter views are not assumed correct.** Capture the oracle for
  `b`, `m`, `ng`, and `z` and match it. The checked-in `m` → 唔/五 (exact only,
  not all `m`-initial) shows "single letter → all same-initial characters" is
  not an oracle-backed invariant — it may itself need a fix, so it is not a
  preserve-as-is call.
- **Product lane only.** This is the TypeDuck/Jyutping product path; the
  upstream `luna_pinyin` Track A lane is out of scope unless a fixture shows the
  same defect there.

## Win Bars

M58 closes when:

1. For `beingo`, Yune's candidate set **matches the full TypeDuck `v1.1.2`
   capture** — exact word/sentence/syllable matches only, no `be`/`b`-initial
   fuzzy — so 畀 is reachable.
2. For the `諮議局` input, Yune's candidate set matches the oracle capture — no
   `z`-initial fuzzy ahead of / crowding out the exact `zi` characters
   (including 諮).
3. Single-letter input (`b`, `m`, …) matches the oracle capture. Note the
   oracle shows exact-syllable-only even for a single letter (`m` → 唔/五, not
   all `m`-initial characters), so **do not assume** the current "single letter
   → all same-initial characters" view is correct — capture the oracle and match
   it (this may also need a fix).
4. Oracle-driven candidate tests cover both reported inputs plus a single-letter
   case and one non-Cantonese control; they assert the candidate set/order from
   captured TypeDuck bytes, not from Yune.
5. No Track B product regression (latency guard rows and existing
   `cantonese_parity` rows still pass where fixtures are present).
6. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   and the focused tests pass.

Close partial/no-go if the full oracle capture shows TypeDuck itself emits the
same same-initial fuzzy set (then this is not a bug), or if matching the
oracle's exact-only candidate set cannot be achieved without regressing a
genuine oracle-backed view (e.g. leading-syllable commit or completion).

## Scope

In scope: `valid_lookup_prefixes` / `original_code_allows_prefix_fallback`
prefix selection; the prefix-fallback append order; oracle capture + tests for
the two reported inputs; a decision on completion ordering in long
compositions.

Out of scope: ABI changes; `luna_pinyin` Track A behavior; performance
rebaselining; broad translator refactors.

## Phases

### Phase 0: Reproduce and freeze
- [ ] Record the failing candidate order for `beingo` and `諮議局` (paged) and
      the single-letter `b` control under
      `docs/reports/evidence/m58-jyutping-exact-before-fuzzy/phase-0/`.
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
      it serializes whatever `CaptureWithIdentity` returns.
- [ ] With the paginated harness, capture the **full** TypeDuck `v1.1.2`
      candidate list (all pages) for `beingo`, `ngohaig`, `b`, `m`, `ng`, and the
      exact ASCII key sequence for `諮議局`, into checked-in oracle fixtures.
      This is a Windows-session task (the capture runs against
      TypeDuck-HK/librime `v1.1.2`). **Reject any captured row where
      `captured_all_pages != true` or `pagination_error` is present** — that
      means the pager did not reach the last page and the list is incomplete.
      If the capture is unavailable, block the milestone — do not proceed on
      Yune-defined expectations.

### Phase 1: Instrument prefix selection
- [ ] Add a dev-only diagnostic (test helper or metric) that reports, per
      input, the prefixes `valid_lookup_prefixes` emits and which candidates
      each admits, so the partial-prefix contribution is visible.

### Phase 2: Fix prefix selection
- [ ] For multi-code inputs, restrict prefix-fallback to the **oracle-recognized
      leading syllable parse** — not every character boundary, and not
      "prefix-could-be-a-syllable-in-isolation" (so `ng` cannot sneak in for
      `ngohaig`); constrain the single-letter `starts_with` branch to match the
      single-letter oracle captures (`b`/`m`/`ng`/`z`).
- [ ] Add real-path tests asserting Yune's candidate set matches the captured
      oracle for `beingo`, `ngohaig`, and `諮議局`, and matches the single-letter
      oracle captures (`b`, `m`, `ng`).

### Phase 3: Completion ordering decision
- [ ] With the oracle capture, decide whether completion candidates
      (`zi` → `zing`/`zik`) precede or follow exact-syllable characters in a
      first-syllable/long-composition view; implement to match, with a test.

### Phase 4: Re-verify and close
- [ ] Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
      warnings`, focused tests, and the Track B guard.
- [ ] Write `docs/reports/evidence/m58-jyutping-exact-before-fuzzy/` with
      before/after candidate captures and the oracle comparison.
- [ ] Update roadmap/requirements/milestone-history on closeout; move this plan
      to `plans/completed/`.

## Review Checklist For Claude

- Is the root cause really the partial-syllable prefix, or is completion the
  dominant contributor for some inputs? (The `beingo` capture points at
  prefix-fallback; the `zi` capture points at completion — the fix must handle
  both without over-reaching.)
- Does restricting to the oracle-recognized leading parse break any legitimate
  multi-syllable prefix-commit case the oracle capture shows?
- Does the single-letter behavior match the oracle capture (`b`/`m`/`ng`/`z`) —
  rather than being preserved by assumption?
- Are the expectations captured from TypeDuck `v1.1.2`, not asserted from
  Yune's current output?
- Any risk to Track B latency guard rows from changing the fallback set size?

## Non-Goals

- Do not change `luna_pinyin` Track A behavior.
- Do not change single-letter behavior in either direction without the oracle
  capture — match `b`/`m`/`ng`/`z` to what TypeDuck actually shows.
- Do not assert candidate order from Yune; assert it from the TypeDuck oracle.
