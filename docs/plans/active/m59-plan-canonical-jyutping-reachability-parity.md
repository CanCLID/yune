# M59 Canonical Jyutping Composition Reachability & Parity Plan

> **For agentic workers:** execute one phase at a time. **Phase 0 is blocking
> and urgent: `main` is currently RED on the standing M55 ratchet** (two
> consistent failing runs recorded by `c4336cd9`'s own evidence). No other M59
> work lands before the gate is green again. This plan supersedes the earlier
> draft `m59-plan-composition-single-character-reachability.md` (deleted): the
> owner re-prioritized on 2026-07-06 — **the canonical lane (Yune +
> `rime/rime-cantonese` versus upstream `rime/librime 1.17.0` +
> `rime/rime-cantonese`) is the primary target**; the TypeDuck profile lane
> becomes a regression guard, not the acceptance driver.

> **Status:** Draft for review. - **Track:** Engine behavioral correctness
> (canonical Jyutping lane first; TypeDuck profile guarded). - **Created:**
> 2026-07-06. - **Type:** perf-corrective + canonical parity milestone. No ABI
> widening, no new performance claim.

## Owner Requirement (verbatim, 2026-07-05/06)

> When the candidate word list does not offer what I want, I should always be
> able to pick the single characters one by one to compose the final word. I
> should be able to select any single character to achieve any character
> combination matching the input syllable.

Plus the 2026-07-06 priority: *"I want to make sure that the original behavior,
i.e. yune+rime-cantonese matches the librime+rime-cantonese behavior. TypeDuck
behavior matters but we should prioritize the canonical comparison first."*

Engineering translation, canonical lane, any composition length:

1. Every dictionary single character whose code matches the **leading complete
   syllable** must be reachable by paging, in the oracle's order — **no
   promotion to earlier pages** (owner-explicit).
2. Selecting it commits its span and recomposes the remainder (M28 semantics).
3. Acceptance numbers come from the **canonical capture**
   (`phase-1/canonical-rime-cantonese-capture.json`), never from the TypeDuck
   phase-2b captures and never from Yune itself.

## Canonical Oracle Facts (verified from the committed M58 phase-1 capture)

Oracle: upstream `rime/librime 1.17.0` (`33e78140…`) + pinned
`rime/rime-cantonese` (`c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0`), page size
**5**, `captured_all_pages: true` on every row cited:

| Input | Total | First page | Named reachability rows |
| --- | ---: | --- | --- |
| `bei` | 139 (all singles) | 畀 比 被 鼻 避 | 匕 @ **31** |
| `beingo` | 142 (3 phrase rows + the same 139-single family) | 比我 被我 畀我 畀 比 | 畀 @ 3 (first page, oracle-placed); 匕 @ **34** |
| `zijiguk` | 416 (386 singles) | **諮議局** 子怡 自已 旨意 之二 | 諮 @ **227** |

Notable: canonical weights differ sharply from the TypeDuck profile (畀 is the
**top** bei2 single here; 諮議局 is candidate **#1** for `zijiguk` — the
owner's original word). The complete leading-syllable family appears
mid-composition identical to the bare-syllable list, phrase rows prepended —
the owner requirement **is** canonical oracle parity; nothing needs inventing.

**Comparator note (139, not 136 — resolves the review's High finding):** the
`bei` family is **139 single-character rows**. A tone-ignored "romanization
first syllable == `bei`" classifier matches only **136** of them — it drops
`啤/be1`, `唄/be6`, `𠹇/be1`, which the oracle nonetheless returns in the `bei`
list. For the **canonical lane the comparator is the oracle's own ordered
single-character list (all 139), not a romanization filter** — otherwise the
win bar would silently not require those 3 reachable. The romanization
classifier is retained only for the **profile** lane (where `bei` legitimately
spans bei1–bei6). Note also that the two captures store romanization
differently — canonical is plain `"bei2"`, the TypeDuck fixtures are CSV
`"1,比,bei2,…"` — so the diff harness and any classifier reused from `c4336cd9`
(CSV-shaped) must handle both formats.

## Current State (what exists, what is broken)

- **Yune has no canonical Jyutping lane.** M58 captured the canonical oracle
  (phase-1) but its "Yune canonical diff" phase was skipped — no staged
  canonical schema, no Yune-side run, no diff has ever been made — every
  committed `cantonese_parity` oracle is a `typeduck-v1.1.2` fixture
  (shipped-dict provenance is confirmed in Phase 1; see below).
- **`c4336cd9` delivered profile-lane reachability** via the right mechanism
  family: fixed caps deleted, bounded typing requests limit-driven (per-fetch
  3, pending 4×limit, output = limit), unbounded complete-list path uncapped
  plus `leading_single_syllable_prefix_candidates` injection, selector
  page-turn completion. **What is actually test-backed** (resolves the review's
  Medium finding): `beingo` → 畀@6 is guarded (`cantonese_parity.rs` ~`:1922`,
  `:1947`) and `zijiguk` → 諮 is guarded *as reachable by paging on the
  byte-backed product path* (`yune_web.rs` ~`:3000`, `:3117`, both green).
  **Not test-backed:** `beingo` → 匕 has **no** Yune-side guard (no `匕` /
  `U+5315` assertion exists in any `*.rs`), and the `諮@34` index is a
  *profile* number from an **uncommitted scratch capture** (the committed
  profile capture has bare-`zi` 諮@27, and the `zijiguk` test asserts
  reachability, not an index). Add a `beingo` → 匕 reachability guard and stop
  citing `匕@20` / `諮@34` as established until pinned.
- **But it broke the standing gate.** Two consistent ratchet runs fail Track A
  **luna** rows — `ni` 3.95–4.03x (ceiling 2.666), `hao` 3.33–3.43x (1.731),
  `zhongguo` win row 0.855–0.899 (ceiling 0.323; still <1.00x but ~3× worse) —
  plus Track B 359–362 µs (ceiling 347.975). Recorded as a "residual" and
  pushed. Per the M55 corrective's standing rule, threshold breaches are **not
  landable**; this is Phase 0.
- **Regression owner (hypothesis — requires measured attribution before any
  fix; resolves the review's Medium finding):** the bounded-path rewrite keys
  on `bounds_compact_fallback_expansion()`, which has **7 call sites**
  (`translator/mod.rs` `:1028` def, `:1938`, `:1964`, `:2292`, `:2712`,
  `:2783`, `:2851`) — *every* Compact+prism translator, and it gates more than
  the prefix-fallback path (e.g. `max_candidates_per_span` at `:2851`, the
  sentence path). So luna's bounded path plausibly changed even though luna
  never uses prefix-fallback — **but this is not yet attributed.** Phase 0 must
  localize the regression per call site (m37 metrics / bisection over the
  `c4336cd9` diff) **before** choosing a fix; the likely-but-unconfirmed remedy
  is to scope the new bounded machinery to prefix-fallback-enabled translators
  and restore luna's prior path.
- New `starts_with("jyut6ping3")` schema-string gates were added in
  `engine.rs` (~`:1429`) and `processors/selector.rs` (~`:183`) — the wart
  class the conventions warn about grew; cleanup is in scope (typed config).

## Phases

### Phase 0 — BLOCKING: restore the standing ratchet to green
- [ ] Root-cause the Track A luna regression from `c4336cd9` (start with the
      scoping hypothesis above, but attribute per call site before fixing; the
      committed `long-composition-corrective-ratchet*/m37_metrics.csv` runs are
      the profiling base). Fix without losing the delivered profile-lane
      reachability behavior (the guarded rows — `beingo` → 畀@6 and `zijiguk` →
      諮 reachable — plus the M58 anchors stay green).
- [ ] Fix or explain the Track B miss (359–362 vs 348 µs) the same way — the
      ceiling stands; no re-baselining.
- [ ] Ratchet green **twice** on fresh evidence leaves with the full standing
      command (`-TrackAInputs` full list, `-TrackBInputs`,
      `-DeployProductBeforeBenchmark`, `-FailOnRegression`).
- [ ] **Policy note (owner-backed):** "no new regression versus the recorded
      residual" is rejected as a gate policy — that is the M55 erosion pattern.
      Green means under the standing ceilings.

### Phase 1 — Stand up the canonical validation lane
- [ ] Stage pinned `rime/rime-cantonese` @ `c99b16e4…` (the capture's commit)
      as a Yune-loadable shared-data dir for **validation only** — the shipped
      product and the schema-id split decision remain untouched and
      sign-off-gated (D-31 amendment). `import_tables` is supported
      (`dictionary/source.rs:753`), **but that is necessary, not sufficient**
      (review Medium/low): verify the full lane — schema YAML semantics,
      spelling algebra + prism build, filters/options, segmentation, and
      dictionary payload/`import_tables` merge — actually loads and deploys, and
      stop with a named blocker if any rime-cantonese feature is missing.
- [ ] **Confirm the shipped `jyut6ping3.dict.yaml` provenance first.** It stores
      plain `char\tcode\tweight` (no multilingual CSV comments), which looks
      rime-cantonese-derived rather than TypeDuck-derived; if it already *is*
      the pinned rime-cantonese dict, the canonical lane is closer than "never
      diffed" implies and Phase 1's scope shrinks accordingly. Record the
      finding either way.
- [ ] **Defuse the profile-predicate landmine before any diff:**
      `is_typeduck_jyut6ping3_profile` (`schema_install.rs`) matches
      `schema_id jyut6ping3*` + dictionary `jyut6ping3` — the canonical lane
      would silently inherit the TypeDuck-calibrated shims (21.0 sentence
      penalty, prediction-limit-1, correction wiring, fallback tuning).
      Disambiguate via typed translator config / an explicit lane marker (M23
      pattern; the M58 phase-3 blast-radius audit is the reference). Which
      shims the canonical lane *should* run is decided by the Phase 2 diff,
      not assumed.
- [ ] Mirror the capture options exactly (page size **5**, option set) on the
      Yune side; record provenance both sides.

### Phase 2 — The canonical diff (the phase M58 skipped)
- [ ] Run Yune over the staged canonical lane for every phase-1 captured input
      (compiled path where applicable) and diff ordered output against the
      capture, classified per direction: **reachability** (row exists in
      capture, unreachable in Yune at any page), **admission overage/underage**
      (set membership), **order-only**. The diff — not any prior model — is
      the Phase 3 spec.
- [ ] Freeze the diff as committed evidence before fixing anything.

### Phase 3 — Fix canonical behavior per the diff
- [ ] Implement the narrowest fixes the diff proves, reusing the `c4336cd9`
      mechanism family where it fits (limit-driven bounded caps, uncapped
      complete-list, leading-syllable injection) — scoped so luna/Track A is
      untouched.
- [ ] Win bars (canonical numbers, from the capture):
      1. `beingo` first page = 比我 被我 畀我 畀 比 (page size 5; no
         promotion);
      2. the complete **139-single bei family** reachable as an ordered
         subsequence; named row **匕 @ 34**;
      3. `zijiguk`: **諮議局 first**; named row **諮 @ 227** reachable by
         paging, selectable, remainder recomposes;
      4. one ≥15-char control with a large leading family (depth×length case);
      5. comparators pinned: for the **canonical lane the classifier is the
         oracle's own ordered single-character list from the capture (all 139
         for `bei`), not a romanization filter** (the romanization filter drops
         `啤/唄/𠹇` and yields 136 — see the Comparator note); comparator = that
         oracle list as an ordered subsequence of Yune's paged output;
         full-list equality only where the diff shows it achievable — every
         remaining divergence recorded and named, none silently passed.
- [ ] Page-driven assertions only (no initial-materialization-window pins);
      compiled-path tests where caps/limits are storage-gated.

### Phase 4 — Guards, cleanup, close
- [ ] TypeDuck profile lane demoted to **regression guards**: existing M58 +
      `c4336cd9` tests and the profile e2e gates stay green, unchanged; no new
      profile-driven acceptance criteria.
- [ ] Replace the `starts_with("jyut6ping3")` string gates in `engine.rs` /
      `selector.rs` with typed config (no schema-string gates in core).
- [ ] Gates: `cargo fmt --check`; `cargo clippy --workspace --all-targets --
      -D warnings`; `cargo test --workspace` (serial re-run of named suites per
      AGENTS.md discipline); WEB-03 long-input tripwire; the M55 ratchet green
      on a fresh leaf; the first-page-turn materialization guard (budget p95 ≤
      50 ms on the standing bench machine, tighten-only, owner sign-off
      required to loosen — canonical `zijiguk` completes 416+ rows in one
      materialization, so this measures the real worst case).
- [ ] Close: evidence README (before/after diffs, canonical + profile),
      roadmap/requirements/milestone-history updates; record follow-up IDs in
      the roadmap deferred section — `M59-FU-TAIL-ADMISSION` (profile fuzzy
      tail: Yune 192 rows vs oracle 124 for profile `zijiguk`) and
      `M59-FU-CANONICAL-PRODUCT` (shipping the canonical lane in the product /
      schema-id split — still gated on owner sign-off); move this plan to
      `plans/completed/`.

## Decided Calls

- **Canonical lane is the acceptance driver; TypeDuck profile is a guard.**
  Acceptance numbers come from `canonical-rime-cantonese-capture.json` only.
- **Phase 0 blocks everything.** Red main is standing debt; ceilings are the
  gate; no residual-tolerance policy.
- **No promotion, ever** (owner-explicit): oracle order preserved; the fix is
  reachability by paging.
- **Validation lane, not product shipping.** The schema-id split /
  canonical-in-product decision stays separately gated (D-31 amendment).
- **Diff-first.** Phase 3 fixes only what the frozen Phase 2 diff proves;
  directional assertions where full equality is not proven achievable.
- **No input-length gates; no fixed constants bounding the reachable set; no
  new schema-string gates in core.**
- **Plan-before-code.** Any uncommitted working-tree drafts are reconciled or
  reverted with a recorded decision before M59 work starts.

## Review Checklist For Claude

- Was Phase 0 actually closed green (twice, fresh leaves, full command) before
  any canonical work landed?
- Does the canonical lane run free of TypeDuck-calibrated shims unless the
  diff proves the oracle behavior needs an equivalent (typed-config
  installed, not schema-string-gated)?
- Are the win-bar numbers (匕@34, 諮@227, first pages, 139-single family)
  asserted from the committed canonical capture bytes with the pinned
  comparators?
- Are the actual profile-lane guards (`beingo` → 畀@6, `zijiguk` → 諮
  reachable, the M58 anchors, e2e) still green and unchanged — and was the new
  `beingo` → 匕 guard added rather than assumed?
- Is every remaining canonical divergence named in the closeout, none silently
  passed?
- Were the string gates replaced with typed config, and no new ones added?

## Non-Goals

- No TypeDuck-profile acceptance criteria (guards only).
- No canonical-lane product shipping or schema-id rename in M59.
- No re-ranking/promotion beyond oracle order in either lane.
- No re-baselining of the M55 ratchet ceilings.
- No sentence-path cap changes; no admission changes to the profile fuzzy tail
  (deferred as `M59-FU-TAIL-ADMISSION`).
