# M14 — TypeDuck v1.1.2 Cantonese Golden Capture Implementation Plan

> **Status:** Active · **Milestone:** M14 (TypeDuck-Web fork parity — capture) · **Updated:** 2026-06-19 · **Type:** execution plan

> **For agentic workers:** implement **task-by-task**; steps use checkbox (`- [ ]`). M14 only **captures oracle goldens** — it does **not** implement Yune behavior (that is M15) and does **not** un-ignore the `cantonese_parity` tests. Every golden must be captured from the TypeDuck-HK v1.1.2 binary, never derived from Yune (non-circular).

**Goal:** Capture reproducible, provenance-stamped oracle goldens from the **TypeDuck-HK/librime v1.1.2** binary for the five blocked Cantonese behaviors, so M15 has concrete targets to implement against. Resolve, via a feasibility spike, whether per-entry userdb pronunciations are capturable at all.

**Architecture:** Reuse the **scenario-capable** `scripts/oracle-rime-probe.cs` (it already has `CaptureScenarios` / `set_option` / multi-step snapshot support) but its traits hardcode upstream identity (`distribution_version = "1.17.0"`, `app_name = "rime.yune_upstream_oracle_probe"`, [oracle-rime-probe.cs:144-157](../../scripts/oracle-rime-probe.cs)). **Parameterize the oracle identity** (or add a thin v1.1.2 wrapper) so it loads the v1.1.2 `rime.dll` with the correct modules (`default`, `dictionary_lookup`), distribution name/version, and provenance — mirroring how the existing v1.1.2 fixtures were captured ([fixtures/typeduck-v1.1.2/README.md](../../crates/yune-core/tests/fixtures/typeduck-v1.1.2/README.md)). Goldens land under `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` with an `oracle` provenance block (engine `TypeDuck-HK/librime`, tag `v1.1.2`, commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`).

**Tech stack:** C# P/Invoke probe + PowerShell capture wrapper, the v1.1.2 oracle binary under `target/typeduck-oracle/v1.1.2/` (local scratch, gitignored), JSON fixtures, the `oracle_fixture_provenance` guard test.

## Non-goals
- Implementing any behavior (M15) or un-ignoring `cantonese_parity` tests.
- The upstream language model — `jyut6ping3` is dictionary-driven (see D-27).
- Committing the v1.1.2 binary; only provenance + JSON goldens are checked in.

## Current state
- `oracle-rime-probe.cs` has scenario/option-toggle/snapshot capture but upstream identity.
- v1.1.2 oracle (binary + `jyut6ping3` schema) is reproducible under `target/typeduck-oracle/v1.1.2/`; the existing v1.1.2 fixtures (`jyut6ping3-mobile-comments.json`, `reverse-lookup-prompt.json`) prove the capture pattern works.
- Five blocked tests await goldens: [cantonese_parity.rs:260-287](../../crates/yune-core/tests/cantonese_parity.rs) (`options_combine_candidates_show_full_code_enable_sentence_parity`, `completion_prediction_and_enable_completion_parity`, `correction_minimal_distance_and_m_abbreviation_parity`, `schema_menu_hiding_parity`, `per_entry_userdb_pronunciation_parity`).

## Tasks

### Task 1 — v1.1.2 capture wrapper (TYPEDUCK-PARITY-01 infra)
- [ ] Parameterize `oracle-rime-probe.cs` oracle identity (modules, distribution name/version, app name) **or** add `scripts/capture-typeduck-jyutping.ps1` + a v1.1.2 wrapper that loads `target/typeduck-oracle/v1.1.2/.../rime.dll` with modules `default` + `dictionary_lookup`.
- [ ] Add a `jyut6ping3` fixture composer that stamps each fixture's `oracle` block: engine `TypeDuck-HK/librime`, tag `v1.1.2`, commit `74cb52b…`, schema `TypeDuck-HK/schema` commit, capture command, input/action sequence, source-row policy.
- **Acceptance:** a smoke capture (e.g. `nei` → 你/呢/尼) reproduces the existing `jyut6ping3-mobile-comments.json` shape with v1.1.2 provenance.

### Task 2 — Option-toggle goldens (TYPEDUCK-PARITY-01)
- [ ] Capture `combine_candidates`, `show_full_code`, and `enable_sentence` via `set_option` on/off + input + snapshot, at multiple input lengths (e.g. `ngohaigo` for sentence; homophone inputs for combine; cangjie inputs for show_full_code). Record both option states per scenario.
- **Acceptance:** `luna…`-style fixture(s) under `typeduck-v1.1.2/` with before/after-toggle candidate lists captured from v1.1.2.

### Task 3 — Completion + correction goldens (TYPEDUCK-PARITY-01)
- [ ] `enable_completion`: partial-code inputs (e.g. `n`, `ne`) → completion candidates with rank order.
- [ ] correction: deliberate 1-edit typos and `m`-abbreviation inputs → corrected candidates with rank order.
- **Acceptance:** fixtures capturing candidate lists for partial/typo inputs in both option states.

### Task 4 — Schema-menu oracle surface (TYPEDUCK-PARITY-02)
- [ ] **First identify the oracle-observable surface** for `hide_lone_schema`/`hide_caret` — config API read-back, the schema-list/switcher API, or TypeDuck-Web UI state — then capture **emitted behavior**. Do **not** rely on static config inspection alone (it would be circular). If no oracle-observable surface exists at the ABI, record that and scope this to a TypeDuck-Web UI assertion in M16.
- **Acceptance:** either captured emitted-behavior goldens, or a documented finding that this is a UI-only surface deferred to the M16 browser matrix.

### Task 5 — userdb-pronunciation feasibility spike (TYPEDUCK-PARITY-03)
- [ ] The standard RIME C ABI exposes no userdb introspection, but the **levers** ABI has user-dict export/import/backup hooks ([abi.rs:222](../../crates/yune-rime-api/src/abi.rs)). Spike: seed/import a userdb state into the v1.1.2 oracle, type to learn per-entry pronunciations, then export/snapshot to observe the per-entry `(code, freq)` state and subsequent prediction ranking.
- [ ] If capturable → produce the golden. If not → document the **precise** blocker and mark `per_entry_userdb_pronunciation_parity` a fork-only deferral (not silently dropped).
- **Acceptance:** a captured golden **or** a documented, reproducible blocker.

### Task 6 — Provenance guard
- [ ] Extend `oracle_fixture_provenance` to scan the new `typeduck-v1.1.2/` goldens (engine/tag/commit, schema commits, capture command, source-row policy; reject local absolute `target/` paths).
- **Acceptance:** `cargo test -p yune-core --test oracle_fixture_provenance` passes over all new fixtures.

## Completion criteria
- Goldens captured from the v1.1.2 binary for combine_candidates / show_full_code / enable_sentence / completion / correction, each provenance-stamped and non-circular.
- Schema-menu surface identified (captured or scoped to M16 UI).
- The userdb-pronunciation spike resolved: golden **or** documented blocker.
- Provenance guard green; no v1.1.2 binary committed; only JSON + provenance checked in.
- The five `cantonese_parity` tests remain `#[ignore]`d (M15 un-ignores them as behavior lands).

## Review checklist
- [ ] Oracle identity is genuinely v1.1.2 (modules/distribution/commit), not upstream-flavored.
- [ ] Every golden carries provenance; none derived from Yune.
- [ ] Schema-menu capture proves emitted behavior, not echoed static config.
- [ ] userdb spike either captures or documents a precise blocker — no silent gap.
