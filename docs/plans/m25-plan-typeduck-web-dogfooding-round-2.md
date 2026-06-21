# M25 TypeDuck-Web Dogfooding Round 2 Implementation Plan

> **Status:** Intake - **Milestone:** M25 (TypeDuck-Web dogfooding round 2) - **Created:** 2026-06-21 - **Type:** active issue ledger / future execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Intake note:** This plan starts the second manual dogfooding loop for the internal TypeDuck-Web playground. Add the user's upcoming feedback as `M25-DOGFOOD-*` rows here before implementing fixes.

**Goal:** Capture, classify, and later execute the second real dogfooding feedback batch for the internal TypeDuck-Web playground without reopening completed M24 rows or weakening oracle-backed compatibility claims.

**Architecture:** M25 treats `third_party/typeduck-web/` as the browser dogfooding surface and keeps the browser app on Vite + React + Tailwind CSS + small local components. Browser and UI defects are fixed with Playwright evidence from the local `/web/` app; engine-output or ranking defects require pinned TypeDuck `v1.1.2` or upstream `1.17.0` fixtures before changing engine behavior. TypeDuck-Web source edits must be regenerated into the tracked patch before they count as landed.

**Tech Stack:** TypeDuck-Web React/TypeScript, Vite/Bun, Tailwind CSS, small local React components, Playwright, `@yune-ime/typeduck-runtime`, `yune-rime-api` C ABI, `yune-core`, TypeDuck `v1.1.2` oracle fixtures, upstream librime `1.17.0` oracle fixtures.

---

## Scope

M25 is the second dogfooding and hardening loop for the internal browser playground:

- **In scope:** manual play feedback from `http://localhost:5173/web/`, first-run and reload behavior, schema switching, settings ergonomics, candidate panel layout, dictionary/detail panel readability, input scenarios, inspector/status usefulness, and local dogfood UI polish.
- **In scope with oracle evidence:** any change to candidate text, candidate set, candidate order, segmentation, correction, prediction, reverse lookup, dictionary lookup payloads, or commit behavior.
- **In scope for frontend stack maintenance:** keep the dogfood demo on Vite + React + Tailwind CSS + small local components only. Do not add DaisyUI back and do not add another UI framework.
- **Out of scope:** editing the separately cloned or deployed `TypeDuck-HK/TypeDuck-Web` product, treating `typeduck.hk/web` as the hard oracle, broad design-system work, widening the default `RimeApi`, or adding unsupported controls that only appear to work.

## Relationship To M24

M24 is closed and archived at `docs/plans/archive/m24-plan-typeduck-web-dogfooding.md`. Do not edit M24 rows for new feedback.

Use M24 as the baseline for:

- local Tailwind-only component stack,
- `third_party/typeduck-web/e2e/results/m24-dogfooding/` as historical evidence only,
- TypeDuck-Web patch discipline,
- the `jigaajiusihaa` TypeDuck `v1.1.2` ordering fixture,
- the `menu/page_size` customize key,
- the Jyutping Mandarin-pinyin affix path `` `p... ``.

M25 evidence belongs under `third_party/typeduck-web/e2e/results/m25-dogfooding/<issue-id>/`.

## Classification Rules

Classify every report before editing code:

| Classification | Use when | First evidence |
|---|---|---|
| Browser integration | Worker/runtime/assets/settings wiring fails or drifts from the intended browser contract. | Browser console logs, worker diagnostics, state JSON, screenshot. |
| UI polish | The rendered app is confusing, cramped, mislabeled, inaccessible, or inefficient, but engine output is correct. | Screenshot plus the exact interaction path. |
| Engine correctness | Candidate output, ranking, commit text, segmentation, correction, prediction, or reverse lookup seems wrong. | Pinned oracle fixture or a row marked blocked until fixture capture. |
| Unsupported / N/A | The report asks for behavior intentionally not exposed in the dogfood playground. | Short rationale and, if useful, a UI copy/docs change. |
| Future product integration | The report belongs to the real TypeDuck-Web product, not the internal Yune harness. | Product-track note; do not edit `third_party/typeduck-web/source/` unless the harness also needs it. |
| Needs triage | The symptom is not reproducible or does not yet identify the layer. | Screenshot/state capture and a narrow reproduction attempt. |

## Evidence Rules

- Save browser evidence under `third_party/typeduck-web/e2e/results/m25-dogfooding/<issue-id>/`.
- For every browser-visible fix, capture a screenshot or JSON/state snapshot from the real local `/web/` app.
- For every engine-output fix, add or extend a pinned oracle fixture before implementation.
- Do not use `https://www.typeduck.hk/web/` as a hard oracle. It is a useful feel target only.
- Keep completed M9/M13/M16/M20/M22/M24 gates green unless a row explicitly changes a supported contract with fresh evidence.

## Patch-Layer Rule

`third_party/typeduck-web/source/` is gitignored in the Yune repository. Local edits there are allowed for development, but a M25 row is not closed until the matching tracked artifacts are updated.

Before closing any row that changes TypeDuck-Web source:

1. Regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` from the patched upstream checkout.
2. Reverse-check from `third_party/typeduck-web/source/`:

   ```powershell
   git apply --reverse --check ..\patches\yune-typeduck-runtime.patch
   ```

3. Forward-check the patch on a clean source checkout reset to `third_party/typeduck-web/typeduck-web.lock.json`.
4. Stage only the tracked artifacts for the slice: the patch, Yune-owned integration files, Playwright tests/evidence, Rust/runtime files, docs, and lock metadata when the upstream source pin changes.

## Running Issue Ledger

No M25 feedback rows have been captured yet. When feedback arrives, append one row per distinct user-visible symptom using the next `M25-DOGFOOD-XX` id.

| ID | Status | Classification | User-visible issue | First repro / evidence | Owning surfaces to inspect first | Close criteria |
|---|---|---|---|---|---|---|

## Intake Task

### Task 1: Convert User Feedback Into M25 Rows

**Files:**
- Modify: `docs/plans/m25-plan-typeduck-web-dogfooding-round-2.md`
- Create evidence as needed: `third_party/typeduck-web/e2e/results/m25-dogfooding/<issue-id>/`

- [ ] **Step 1: Split the feedback into distinct symptoms**

  Treat each independently reproducible behavior as one row. If two comments share one root cause but have different user-visible symptoms, keep separate rows until triage proves they should close together.

- [ ] **Step 2: Assign stable ids**

  Use `M25-DOGFOOD-01`, `M25-DOGFOOD-02`, and so on. Do not renumber rows after they are referenced by evidence, commits, or tests.

- [ ] **Step 3: Classify each row before implementation**

  Use one of the classifications in this plan. Mark ambiguous reports as `Needs triage`; do not force them into browser or engine buckets prematurely.

- [ ] **Step 4: Name the first evidence path**

  For browser reports, record the intended evidence directory, for example:

  ```text
  third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-01/
  ```

  For engine reports, record the fixture family first: `typeduck-v1.1.2` for TypeDuck profile behavior or `upstream-1.17.0` for default upstream behavior.

- [ ] **Step 5: Name the owning surfaces**

  List the files or test families to inspect first. For TypeDuck-Web source changes, include the `source/` file and the tracked patch requirement.

- [ ] **Step 6: Write close criteria that can be verified**

  Each row needs concrete close criteria: owning test, evidence path, patch regeneration if applicable, and the focused gate to run.

## Execution Order After Intake

When the feedback list exists, execute in this order unless the ledger explicitly says otherwise:

1. Reproduce and capture evidence for all `Needs triage` and `Browser integration` rows.
2. Fix runtime/browser correctness rows before broad UI polish.
3. Capture or reuse pinned oracle fixtures before any `Engine correctness` implementation.
4. Batch adjacent UI polish only when it touches the same local components and does not blur issue ownership.
5. Regenerate and reverse/forward check the TypeDuck-Web patch after every source-changing slice.
6. Close ledger rows as evidence lands; do not wait until the end to update row status.
7. Run focused gates for touched layers, then broad closeout gates if the batch changes shared behavior.

## Closeout Gates

Before M25 can be archived:

```powershell
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
npm.cmd --prefix third_party/typeduck-web/source run build
git diff --check
```

Run the real TypeDuck-Web Playwright tests for every closed browser-visible row. If source files under `third_party/typeduck-web/source/` changed, also run the patch reverse/forward checks from this plan.

## Archive Rule

Archive this plan only after all M25 rows are `Closed`, `Deferred`, or `Rejected` with evidence/rationale. Update `docs/roadmap.md` and `docs/requirements.md` only for durable milestone status or new requirements, not for every intake row.
