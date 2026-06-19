# M15 — TypeDuck Dictionary-Driven Feature Parity Implementation Plan

> **Status:** Active · **Milestone:** M15 (TypeDuck-Web fork parity — implement) · **Updated:** 2026-06-19 · **Type:** execution plan

> **For agentic workers:** implement **task-by-task**; each behavior is implemented/refined to pass its **M14-captured golden** through Yune's real engine path, then its `cantonese_parity` test is **un-ignored**. Non-circular: the expected bytes come from the M14 v1.1.2 goldens, never from Yune. Depends on **M14** goldens existing.

**Goal:** Make Yune's real engine reproduce the TypeDuck `jyut6ping3` behaviors captured in M14, so each blocked `cantonese_parity` test ([cantonese_parity.rs:260-287](../../crates/yune-core/tests/cantonese_parity.rs)) flips from `#[ignore]` to active-and-green. `jyut6ping3` is dictionary-driven — **no language model** (D-27).

**Architecture:** Own each behavior in its implementation module with a matching test, per CONVENTIONS. Most behaviors have partial scaffolding to refine; `combine_candidates` and `show_full_code` are from scratch. No change to the upstream `RimeApi` table or `RimeCandidate` ABI.

**Tech stack:** Rust (`yune-core` translator/filter/engine), the M14 fixtures, `cargo` test/clippy.

## Non-goals
- Capturing goldens (M14) or the browser E2E (M16).
- The upstream language model (Track 2 / M17); `enable_sentence` here is dictionary/word-boundary driven.
- Any change to default upstream behavior or the `RimeApi` table.

## Tasks

### Task 1 — `combine_candidates` (TYPEDUCK-PARITY-04)
- [ ] Implement candidate grouping/dedup so homophones with the same text but different codes coalesce into one row with all pronunciations in the comment (config `translator/combine_candidates`, [common.yaml] default `separate_candidates`). Owning module: translator post-processing or a dedicated filter; matching test.
- **Acceptance:** the M14 combine_candidates golden passes through the real engine in both option states; from-scratch logic, not a test-only shim.

### Task 2 — `show_full_code` (TYPEDUCK-PARITY-04)
- [ ] Implement cangjie preedit algebra for the side-lookup path so the full input code renders in preedit (jyut6ping3 cangjie sub-lookup). Owning module: the cangjie `table_translator` preedit-format path; matching test.
- **Acceptance:** the M14 show_full_code golden passes; classic jyutping preedit unchanged.

### Task 3 — `enable_sentence` refine (TYPEDUCK-PARITY-05)
- [ ] Refine the existing Viterbi sentence path `sentence_candidate()` ([translator/mod.rs:360-473](../../crates/yune-core/src/translator/mod.rs)) so word-boundary segmentation + phrase-table lookup reproduces the v1.1.2 sentence output (e.g. `ngohaigo` → 我係個).
- **Acceptance:** the M14 enable_sentence golden passes through the real engine; no language-model dependency introduced.

### Task 4 — Completion ranking (TYPEDUCK-PARITY-05)
- [ ] Improve the existing prefix-search completion ([translator/mod.rs:288-313](../../crates/yune-core/src/translator/mod.rs), fixed `-1.0` penalty) so partial-code candidate generation and rank order match the M14 completion golden.
- **Acceptance:** the M14 completion golden passes; completion off ⇒ output unchanged.

### Task 5 — Correction/tolerance tuning (TYPEDUCK-PARITY-05)
- [ ] Tune the correction/tolerance ranking (the fixed `SPELLING_ALGEBRA_CORRECTION_PENALTY`, [spelling_algebra.rs:76-81](../../crates/yune-core/src/spelling_algebra.rs); correction/tolerance application in [translator/mod.rs:186-245](../../crates/yune-core/src/translator/mod.rs)) so minimal-distance + m-abbreviation corrections match the M14 correction golden.
- **Acceptance:** the M14 correction golden passes; correction off ⇒ output unchanged.

### Task 6 — OpenCC `hk2s` coverage (TYPEDUCK-PARITY-06)
- [ ] Expand `SimplifierFilter` ([filter/mod.rs:220-551](../../crates/yune-core/src/filter/mod.rs), ~60 hardcoded char pairs) to the full `hk2s` conversion data the jyut6ping3 simplifier needs — embed or load the conversion table; do not hardcode per character.
- **Acceptance:** the jyut6ping3 simplification toggle matches v1.1.2 for the captured set; serves upstream too.

### Task 7 — Activate the parity tests
- [ ] As each behavior passes its golden, **un-ignore** the corresponding `cantonese_parity` test (lines 260-287) and assert against the M14 fixture (non-circular). Leave any M14-deferred behavior (e.g. uncapturable userdb pronunciations) `#[ignore]`d with the documented blocker.
- **Acceptance:** the previously-blocked tests are active and green (or carry a documented fork-only deferral).

## Completion criteria
- combine_candidates, show_full_code, enable_sentence, completion, correction, and OpenCC `hk2s` reproduce their M14 v1.1.2 goldens through Yune's real engine.
- The `cantonese_parity` tests are activated (or carry documented fork-only deferrals); no test-only shims.
- `cargo fmt`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` pass.

## Review checklist
- [ ] Each behavior owns an implementation module + test; `lib.rs` stays a facade.
- [ ] Expected bytes come from M14 v1.1.2 goldens, never from Yune.
- [ ] Option-off paths are unchanged (no regression to classic jyutping or upstream).
- [ ] No `RimeApi`/`RimeCandidate`/upstream-default behavior change.
