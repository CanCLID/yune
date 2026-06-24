# M28 TypeDuck Partial Candidate Selection Implementation Plan

> **Status:** Complete - **Milestone:** M28 (TypeDuck partial candidate selection) - **Closed:** 2026-06-22 - **Type:** execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement TypeDuck-profile segment-aware partial candidate selection so selecting a candidate for a prefix commits only the consumed span and keeps the remaining input composing.

**Architecture:** This is engine correctness work, not TypeDuck-Web UI polish and not M27 performance work. Capture TypeDuck-HK/librime `v1.1.2` oracle behavior first, then add native engine/API tests, then change `yune-core` candidate span and commit handling. The oracle capture is capture-not-confirm: the user's `測試一下長句子` flow is the feel target, but TypeDuck v1.1.2 is authoritative if it diverges. Browser evidence is a final integration proof only after the engine behavior is fixture-backed.

**Tech Stack:** Rust (`yune-core` Engine/candidate state, `cantonese_parity`, `yune-rime-api` typeduck_web tests), TypeDuck v1.1.2 oracle fixture capture, TypeDuck-Web Playwright smoke only as final surface evidence.

**Closeout:** Completed on 2026-06-22. M28 classified the issue as missing support, captured TypeDuck-HK/librime `v1.1.2` oracle behavior, implemented segment-aware partial commit/recomposition, preserved FORK-PARITY-03 learning behavior, added native/API/browser evidence, and left one-row sentence continuation/ranking as a separate future oracle-backed scope. Evidence lives under `apps/yune-web/e2e/results/m28-partial-selection/`, with the fixture at `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json`.

---

## Why This Milestone Exists

Manual TypeDuck-Web dogfooding found that typing `caksijathaacoenggeoizi`, then selecting the first character candidate `測`, can commit `測sijathaacoenggeoizi`. That is not a browser insertion bug. The current engine commit path in `crates/yune-core/src/engine.rs` uses:

```rust
let segment_start = 0;
let segment_end = input.len();
let text = candidate.commit_text_for_input(&input);
self.clear_composition();
```

That means candidate selection consumes the whole input and clears composition even when the chosen candidate only covers a prefix such as `cak`. M28 must verify whether this is a regression or a long-missing feature, capture the TypeDuck v1.1.2 behavior, and implement the engine behavior against that oracle.

## Scope

In scope:

- Confirm whether partial candidate selection ever worked in Yune by inspecting `git log -p` for `commit_candidate`, `Candidate`, and segment metadata.
- Capture TypeDuck v1.1.2 oracle behavior for `caksijathaacoenggeoizi` partial selection without pre-filling expected answers from the observed Yune bug or the user's feel target.
- Add fixture-backed native tests for prefix candidate selection, remaining preedit/input, context candidates, and the captured final oracle flow, while recording whether the user feel target `測試一下長句子` is reachable.
- Add candidate consumed-span metadata or an equivalent segment ownership model in `yune-core`.
- Change `Engine::commit_candidate` so explicit selection commits only the candidate's consumed span and recomposes the remaining input.
- Preserve FORK-PARITY-03 userdb pronunciation recovery: whole-sentence commits still record full primary codes, and only true partial commits record the consumed span.
- Add a TypeDuck-Web browser smoke after the native behavior is correct.

Out of scope:

- M27 startup/runtime performance work.
- Treating browser Playwright evidence as the hard oracle.
- Changing upstream `luna_pinyin` behavior without upstream `1.17.0` fixtures.
- Rewriting the full segmenter/selector architecture beyond what this partial-selection fixture requires.
- Changing candidate text/order/comment behavior beyond the captured oracle fixture.

## Acceptance Gates

- `M28-PARTIAL-01`: A checked-in note or evidence file states whether this is a regression or previously missing support, based on `git log -p` over `commit_candidate` and candidate span metadata.
- `M28-PARTIAL-02`: A TypeDuck v1.1.2 oracle fixture captures `caksijathaacoenggeoizi` selection behavior, including first selection commit text, remaining preedit/input, candidate list after selection, and the path to complete the oracle flow. If v1.1.2 diverges from the user's `測試一下長句子` feel target, keep the v1.1.2 values and document the divergence.
- `M28-PARTIAL-03`: Native tests fail before the implementation and pass after it. Tests must cover `yune-core` engine behavior, the `yune-rime-api` TypeDuck-Web/native frontend path, and FORK-PARITY-03 userdb pronunciation recovery for whole-sentence versus true partial commits.
- `M28-PARTIAL-04`: Browser evidence proves the internal TypeDuck-Web playground can type `caksijathaacoenggeoizi`, select `測`, and continue selection without inserting raw `sijathaacoenggeoizi`.
- `M28-PARTIAL-05`: Full compatibility gates pass: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p yune-core --test cantonese_parity`, `cargo test -p yune-rime-api --test typeduck_web`, `cargo test --workspace`, TypeDuck-Web build/evidence if source changes, patch checks if source changes, and `git diff --check`.

## Files And Responsibilities

- `crates/yune-core/src/engine.rs`: partial commit behavior, remaining composition, userdb event segment metadata.
- `crates/yune-core/src/state.rs`: candidate consumed-span metadata if needed.
- `crates/yune-core/src/translator/mod.rs`: set candidate consumed-code span when dictionary/sentence candidates are produced.
- `crates/yune-core/tests/cantonese_parity.rs`: fixture-backed TypeDuck oracle assertion.
- `crates/yune-core/tests/fixtures/typeduck-v1.1.2/`: new M28 oracle fixture.
- `crates/yune-rime-api/tests/typeduck_web.rs` or focused module under `crates/yune-rime-api/tests/typeduck_web/`: frontend-shaped regression test.
- `apps/yune-web/e2e/yune-typeduck.spec.ts`: final browser smoke only after native tests pass.
- `apps/yune-web/e2e/results/m28-partial-selection/`: evidence folder.

## Implementation Tasks

### Task 0 - Confirm Regression Versus Missing Feature

**Files:**

- Read: `crates/yune-core/src/engine.rs`
- Read: `crates/yune-core/src/state.rs`
- Create: `apps/yune-web/e2e/results/m28-partial-selection/history-classification.md`

- [x] Step 0.1: Inspect history.

Run:

```powershell
git log -p -- crates/yune-core/src/engine.rs crates/yune-core/src/state.rs
git log -L 1043,1072:crates/yune-core/src/engine.rs
```

Expected:

- `history-classification.md` records whether full-input commit was introduced recently or has existed since the engine split.
- If this is a regression, record the last known good commit and the first bad commit.
- If this is missing support, record that M28 is feature completion, not a regression fix.

### Task 1 - Capture The TypeDuck v1.1.2 Oracle

**Files:**

- Create: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json`
- Create: `apps/yune-web/e2e/results/m28-partial-selection/oracle-capture.md`

- [x] Step 1.1: Capture the partial-selection flow from TypeDuck v1.1.2.

Capture the real oracle behavior for:

```text
input: caksijathaacoenggeoizi
action 1: select candidate text 測
captured after action 1: committed text, remaining preedit/input, candidates for remaining input
user feel target: 測試一下長句子
```

Required fixture content:

| Field | Rule |
| --- | --- |
| `fixture` | Use `jyut6ping3-m28-partial-selection`. |
| `oracle` | Use `TypeDuck-HK/librime v1.1.2` with provenance matching existing typeduck fixtures. |
| `capture_rule` | State that TypeDuck v1.1.2 values are authoritative and user-provided text is only a feel target. |
| `input` | Use `caksijathaacoenggeoizi`. |
| `selection_request` | Record the requested candidate text `測`, the actual candidate index selected in v1.1.2, and the candidate row before selection. Do not assume index 0. |
| `captured_commit` | Copy the actual commit text produced by v1.1.2 after the first selection. |
| `captured_remaining_input` / `captured_remaining_preedit` | Copy the actual remaining input and preedit after the first selection, or mark the exact field as `oracle_surface_unavailable` with the observable fallback. |
| `captured_next_candidates` | Copy the next candidate page after the first selection. |
| `captured_final_flow` | Record the actual sequence needed to complete the oracle flow. If the final oracle text differs from `測試一下長句子`, record the divergence instead of rewriting the fixture to match the user feel target. |
| `raw_tail_guard` | Record whether v1.1.2 ever commits raw `sijathaacoenggeoizi` after selecting `測`; this should be captured evidence, not inferred. |

Acceptance:

- The fixture contains captured oracle fields, not guessed values.
- The fixture does not use pre-filled `expected_commit`, `expected_remaining_input`, or `final_expected_text` values copied from the user's report.
- If v1.1.2 diverges from the user's expected flow, the fixture keeps the v1.1.2 values and `oracle-capture.md` explains the divergence.
- Any field the oracle cannot expose is marked `oracle_surface_unavailable` with the exact reason and a fallback observable surface.

### Task 2 - Add Failing Native Tests

**Files:**

- Modify: `crates/yune-core/tests/cantonese_parity.rs`
- Modify: `crates/yune-rime-api/tests/typeduck_web.rs` or a focused typeduck_web module
- Read: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json`

- [x] Step 2.1: Add a `cantonese_parity` test.

The test must:

- Load `jyut6ping3_mobile` TypeDuck profile assets.
- Type `caksijathaacoenggeoizi`.
- Select the candidate whose text is `測`.
- Assert committed text matches the fixture's captured first-selection commit, not a hardcoded user expectation.
- Assert the remaining input/preedit matches the fixture's captured remaining state, not a guessed byte span.
- Assert the next candidate list is for the remaining input, not for an empty composition.
- Assert FORK-PARITY-03 remains intact: whole-sentence commits preserve full primary codes, while true partial commits record only the consumed span needed by userdb learning.

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_partial_selection
```

Expected before implementation:

- The test fails by committing raw tail text or clearing composition.

- [x] Step 2.2: Add a frontend-shaped `typeduck_web` test.

The test must use the same runtime/API path that TypeDuck-Web uses for candidate selection and assert the same commit/preedit behavior.

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_web -- m28_partial_selection
```

Expected before implementation:

- The test fails for the same engine behavior, proving the bug is not browser-only.

### Task 3 - Implement Segment-Aware Partial Commit

**Files:**

- Modify: `crates/yune-core/src/state.rs`
- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-core/src/engine.rs`

- [x] Step 3.1: Add consumed-span metadata to candidates.

Expected shape:

- Candidate metadata can identify the code/input span consumed by the candidate.
- The consumed span is produced by translator/segment metadata in `translator/mod.rs`, not guessed from UTF-8 byte length or candidate text length.
- Exact whole-input candidates still consume the full input.
- Prefix candidates consume only the matched prefix.
- If a candidate lacks explicit span metadata, default behavior must be conservative and covered by tests.

- [x] Step 3.2: Recompose the remaining input after explicit selection.

Expected behavior:

- `commit_candidate` uses the candidate consumed span instead of `input.len()`.
- For true partial commits, it records userdb learning with the consumed segment.
- For whole-sentence commits, it preserves the full primary-code learning behavior required by FORK-PARITY-03.
- It removes the consumed prefix from composition.
- It reruns the normal speller/segmentor/translator/filter pipeline for the remaining input.
- It keeps the frontend context usable for continued selection.

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_partial_selection
cargo test -p yune-rime-api --test typeduck_web -- m28_partial_selection
```

Expected:

- Both tests pass.

### Task 4 - Add Browser Evidence

**Files:**

- Modify if needed: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Patch if source changed: `apps/yune-web/patches/yune-web-runtime.patch`
- Evidence: `apps/yune-web/e2e/results/m28-partial-selection/browser-partial-selection.json`

- [x] Step 4.1: Add a browser smoke.

The browser test must:

- Load `/web/`.
- Type `caksijathaacoenggeoizi`.
- Select `測`.
- Assert the textarea is not `測sijathaacoenggeoizi`.
- Continue candidate selection through the fixture-backed oracle flow; also record whether the user feel target `測試一下長句子` is reachable.
- Save candidate/preedit snapshots before and after the first selection.

Run:

```powershell
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M28 PARTIAL" --workers=1
```

Expected:

- Browser evidence passes only after native tests pass.
- If TypeDuck-Web source changed, regenerate and reverse/forward-check `apps/yune-web/patches/yune-web-runtime.patch`.

### Task 5 - Close With Full Gates And Docs

**Files:**

- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Archive when complete: `docs/plans/archive/m28-plan-typeduck-partial-selection.md`

- [x] Step 5.1: Run verification.

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
npm.cmd --prefix apps/yune-web/source run build
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M28 PARTIAL" --workers=1
git diff --check
```

Expected:

- All gates pass.
- Roadmap and requirements mark M28 complete with fixture and evidence paths.
- This plan moves to `docs/plans/archive/` only after the gates pass.
