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
- the Jyutping Mandarin-pinyin affix path `` `p... `` as historical M24 behavior; `M25-DOGFOOD-08` reopens that dogfood UX and requests bare `` ` `` for the web reverse-lookup trigger.

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

## Review Summary For Claude

Use this section as the high-level review map. The running ledger below remains the source of truth for exact evidence paths, owning files, and close criteria.

| Execution group | Rows | Why this grouping matters |
|---|---|---|
| Performance and responsiveness blockers | `M25-DOGFOOD-01`, `M25-DOGFOOD-03` | These decide whether the app feels usable at all. Review first for measurement quality, startup phase attribution, and whether normal typing is separated from deploy/customize/loading work. |
| Candidate/settings correctness | `M25-DOGFOOD-02`, `M25-DOGFOOD-08` | These affect user-visible IME behavior. `M25-DOGFOOD-02` must fix both the settings key path and rendered candidate cap; `M25-DOGFOOD-08` intentionally supersedes the historical M24 `` `p`` web trigger request with the new bare-grave dogfood expectation. |
| Top-control and settings layout | `M25-DOGFOOD-04`, `M25-DOGFOOD-05`, `M25-DOGFOOD-06`, `M25-DOGFOOD-10` | These should be implemented together only after the correctness blockers are understood, because they share `App.tsx`, `Toolbar.tsx`, `SchemaSwitcher.tsx`, and `Preferences.tsx` layout surfaces. |
| Control affordance polish | `M25-DOGFOOD-07`, `M25-DOGFOOD-09` | These are local component presentation changes. They can be batched if tests preserve the individual checkbox and radio close criteria. |

Claude review should focus on four questions:

1. Is the startup plan still aimed at the validated P0 suspects: debug browser WASM and forced schema invalidation before lower-level micro-optimization?
2. Does the page-size row protect both `menu/page_size` browser/runtime wiring and the candidate panel's rendered row cap?
3. Is the reverse-lookup row clear that bare `` ` `` is an M25 dogfood correction request, even though captured TypeDuck `v1.1.2` schema evidence still shows `` `p``?
4. Can the UI rows be batched by touched surfaces without losing per-row browser evidence and close criteria?

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

M25 intake began on 2026-06-21 with user-reported browser dogfooding regressions. Keep adding one row per distinct user-visible symptom using the next `M25-DOGFOOD-XX` id.

| ID | Status | Classification | User-visible issue | First repro / evidence | Owning surfaces to inspect first | Close criteria |
|---|---|---|---|---|---|---|
| M25-DOGFOOD-01 | Open | Browser integration / performance | Refreshing `http://localhost:5173/web/` still leaves `載入中 Loading...` visible for too long; the user sees no practical improvement compared with the pre-M24 app and considers this unacceptable for real users. | Reproduced in the in-app browser on 2026-06-21: reload took `47.752s`; startup marker reported `totalMs=47331`, with `runtime:initialized` consuming nearly the entire delay after assets loaded. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-01/reload-startup-repro.json`. | `third_party/typeduck-web/source/src/worker.ts`, `third_party/typeduck-web/source/src/yune-integration/adapter.ts`, `third_party/typeduck-web/source/src/yune-integration/assets.ts`, `packages/yune-typeduck-runtime/src/typeduck.ts`, `packages/yune-typeduck-runtime/src/module.ts`, `packages/yune-typeduck-runtime/src/filesystem.ts`, `crates/yune-rime-api/src/typeduck_web.rs`, schema deploy/init paths under `crates/yune-rime-api/src/`. | Close only after a measured startup optimization lands. Add a Playwright startup budget test that records phase timings, preserve `startup:complete` diagnostics, and capture before/after evidence under this issue id. Target: interactive shell visible quickly and warm reload IME readiness materially below the current ~47s baseline; any chosen budget must be written into the test and justified by local evidence. |
| M25-DOGFOOD-02 | Open | Browser integration / settings and candidate pagination | The page-size control is hard to find after M24 UI changes, its allowed range is wrong for the requested behavior, and changing it still does not cap the rendered candidate list. The user expects a visible slider/control allowing 3-10 candidates, where setting 9 shows exactly 9 visible candidates. | User screenshot showed `hai` rendering far more than 10 rows. Reproduced in the in-app browser on 2026-06-21: DOM had page-size slider `min=4`, `max=10`, `value=6`, but typing `hai` rendered `50` visible candidate rows. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-02/page-size-hai-repro.json`. | `third_party/typeduck-web/source/src/Preferences.tsx`, `third_party/typeduck-web/source/src/Inputs.tsx`, `third_party/typeduck-web/source/src/CandidatePanel.tsx`, `third_party/typeduck-web/source/src/App.tsx`, `third_party/typeduck-web/source/src/yune-integration/adapter.ts`, `crates/yune-rime-api/src/typeduck_web.rs`, `crates/yune-rime-api/src/context_api.rs`, `crates/yune-rime-api/tests/typeduck_web.rs`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`. | Close when the UI exposes an obvious 3-10 page-size control and the rendered candidate panel never exceeds the configured page size. Add/extend native `typeduck_web` coverage for `menu/page_size` at 3 and 9, add Playwright coverage that sets 3/9/10 and types `hai`, verify page navigation still works, regenerate and reverse/forward check `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, and capture browser JSON/screenshot evidence under this issue id. |
| M25-DOGFOOD-03 | Open | Browser integration / typing responsiveness | Typing in the textbox can still show the global `è¼‰å…¥ä¸­ Loading...` state and can stall for about a second when entering a character. This makes the dogfood IME feel blocked even after the page becomes visible. | User-reported during manual dogfooding on 2026-06-21. Evidence placeholder with exact report: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-03/typing-latency-user-report.json`. Needs measured browser repro with keydown-to-candidate timing. | `third_party/typeduck-web/source/src/CandidatePanel.tsx`, `third_party/typeduck-web/source/src/rime.ts`, `third_party/typeduck-web/source/src/worker.ts`, `third_party/typeduck-web/source/src/App.tsx`, `third_party/typeduck-web/source/src/Toolbar.tsx`, `third_party/typeduck-web/source/src/yune-integration/adapter.ts`, `packages/yune-typeduck-runtime/src/typeduck.ts`, `crates/yune-rime-api/src/typeduck_web.rs`, `crates/yune-core/src/engine.rs`. | Close only after per-key latency is measured and improved. Add Playwright instrumentation for keydown-to-candidate update latency on `hai`, `nei`, and a long phrase; split queue wait, worker roundtrip, Rust `process_key`, and React render time; remove global loading from normal per-key composition; ensure typing remains responsive while startup/deploy/customize is in flight; save before/after latency JSON under this issue id. |
| M25-DOGFOOD-04 | Open | UI polish / schema switcher layout | The schema list sits in its own vertical block below the three mode buttons, wasting vertical space. The user wants the schema list moved into the same top row as the three buttons. The `luna_pinyin` option is also currently shown as `普通話`, but the user expects the schema name `朙月拼音`. | User-reported during manual dogfooding on 2026-06-21. Local verification found upstream `1.17.0` `luna_pinyin.schema.yaml` uses `朙月拼音`, while the current TypeDuck-Web source and TypeDuck v1.1.2 captured schema use `普通話`. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-04/schema-switcher-toolbar-and-luna-name-report.json`. | `third_party/typeduck-web/source/src/App.tsx`, `third_party/typeduck-web/source/src/Toolbar.tsx`, `third_party/typeduck-web/source/src/SchemaSwitcher.tsx`, `third_party/typeduck-web/source/src/consts.ts`, `third_party/typeduck-web/source/src/Inputs.tsx`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`. | Close when the schema selector is part of the top control row with the three mode buttons on desktop, wraps within the same compact control band on narrow screens, and `luna_pinyin` is labeled `朙月拼音` as the primary user-facing schema name unless the row is explicitly revised with TypeDuck-v1.1.2-specific rationale. Add Playwright evidence covering desktop and mobile layout, active schema switching, and the visible Luna label; regenerate and reverse/forward check `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` if source changed. |
| M25-DOGFOOD-05 | Open | UI polish / top controls layout | The `倉頡版本 Cangjie version` control currently lives in a separate lower `Web Frontend Controls` section, but it is tightly related to schema choice. The user wants it moved to the top beside the schema selection so the page does not spend vertical space on a one-control section. | User-reported during manual dogfooding on 2026-06-21. Local verification found the control in `third_party/typeduck-web/source/src/Preferences.tsx` under `Web Frontend Controls`, using `prefs.isCangjie5` with `三代 Version 3` and `五代 Version 5` segments. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-05/cangjie-version-top-controls-report.json`. | `third_party/typeduck-web/source/src/App.tsx`, `third_party/typeduck-web/source/src/Toolbar.tsx`, `third_party/typeduck-web/source/src/SchemaSwitcher.tsx`, `third_party/typeduck-web/source/src/Preferences.tsx`, `third_party/typeduck-web/source/src/Inputs.tsx`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`. | Close when the Cangjie version segmented control is colocated with the schema selector in the top control band on desktop and wraps within that same compact band on narrow screens, the lower `Web Frontend Controls` section is removed if it has no remaining controls, and changing 三代/五代 still updates the `isCangjie5` customize path. Add Playwright evidence for layout and both Cangjie version values; regenerate and reverse/forward check `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` if source changed. |
| M25-DOGFOOD-06 | Open | UI polish / settings section order | The `顯示設定 Display controls` section is currently in the second-row right column. The user wants it in the top-row right column, meaning `顯示設定 Display controls` and `即時狀態 Live session controls` should exchange positions. | User-reported during manual dogfooding on 2026-06-21. Local verification found `Preferences.tsx` renders `即時狀態 Live session controls` before `顯示設定 Display controls`, causing Display controls to land below the top-right slot in the two-column settings grid. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-06/display-live-section-order-report.json`. | `third_party/typeduck-web/source/src/Preferences.tsx`, `third_party/typeduck-web/source/src/Inputs.tsx`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`. | Close when Display controls render before Live session controls in the settings grid, occupy the top-right column on desktop, and Live session controls move to the position Display controls previously occupied. Preserve all existing control labels, state bindings, and live-option behavior. Add Playwright desktop and narrow-viewport evidence for the section order and bounding boxes; regenerate and reverse/forward check `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` if source changed. |
| M25-DOGFOOD-07 | Open | UI polish / binary control affordance | Binary settings are currently shown as rounded rectangle pill switches that fill blue when checked. The user finds this unintuitive and wants these binary toggles to be raw checkbox-style controls instead. | User-reported during manual dogfooding on 2026-06-21 with screenshot `codex-clipboard-da4df560-7b7a-49fd-814f-4cfe5cbd6968.png`. Local verification found `Toggle` renders `input type="checkbox"` with `className="yd-switch"` in `Inputs.tsx`, and `.yd-switch` applies the pill styling in `index.css`; `.yd-check` already exists as a square checkbox style. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-07/raw-checkbox-binary-controls-report.json`. | `third_party/typeduck-web/source/src/Inputs.tsx`, `third_party/typeduck-web/source/src/index.css`, `third_party/typeduck-web/source/src/App.tsx`, `third_party/typeduck-web/source/src/ThemeSwitcher.tsx`, `third_party/typeduck-web/source/src/Preferences.tsx`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`. | Close when binary dogfood controls render as checkbox-style controls rather than pill switches, checked/unchecked states remain clear and keyboard-accessible, and no settings-panel binary control uses the blue rounded-pill affordance. Decide explicitly whether the app theme toggle remains a specialized icon switch or also moves to checkbox styling. Add Playwright evidence for checked and unchecked states across engine/session/display/frontend settings plus the inspector toggle; regenerate and reverse/forward check `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` if source changed. |
| M25-DOGFOOD-08 | Open | Browser integration / reverse lookup schema trigger | Jyutping Mandarin reverse lookup currently appears to require `` `p``. The user says this is a mistake and the luna_pinyin reverse-lookup trigger should be the single grave accent `` ` ``. While trying reverse lookup, the app is also slow and can show `執行操作時發生錯誤。如輸入法不能正常運作，請重新載入頁面。 / An error occurred while performing the operation...`; treat that performance/error symptom as related to `M25-DOGFOOD-01`/`M25-DOGFOOD-03` unless a fresh repro proves a distinct reverse-lookup crash. | User-reported during manual dogfooding on 2026-06-21. Local verification found `third_party/typeduck-web/source/schema/jyut6ping3.schema.yaml` and `jyut6ping3_mobile.schema.yaml` set `luna_pinyin` prefix to `` `p`` and alphabet regex ``^`p[a-z']*;?$``; the TypeDuck v1.1.2 captured schema also has `` `p``, so this is a new M25 web dogfood correction request rather than a continuation of the M24 assumption. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-08/reverse-lookup-bare-grave-trigger-report.json`. | `third_party/typeduck-web/source/schema/jyut6ping3.schema.yaml`, `third_party/typeduck-web/source/schema/jyut6ping3_mobile.schema.yaml`, `third_party/typeduck-web/source/src/consts.ts`, `crates/yune-rime-api/src/schema_install.rs`, `crates/yune-rime-api/tests/typeduck_web.rs`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`. | Close when the web Jyutping schema uses bare `` ` `` for luna_pinyin reverse lookup, the visible hint changes from `` `pzhe`` to a bare-grave example such as `` `zhe``, browser evidence shows typing `` `zhe`` reaches luna_pinyin reverse lookup without needing `p`, and `` `p...`` is either still handled intentionally or documented as no longer required. Add or update native/schema coverage for the affix translator trigger, capture browser JSON/screenshot evidence, and regenerate plus reverse/forward check `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` if source changed. |
| M25-DOGFOOD-09 | Open | UI polish / control affordance | The `候選排版 Candidate Menu Layout` setting currently uses a segmented control. The user wants it changed to a radio selection. | User-reported during manual dogfooding on 2026-06-21. Local verification found `Preferences.tsx` maps `CANDIDATE_MENU_LAYOUT_LABELS` into `Segment` controls for `prefs.candidateMenuLayout`; `Inputs.tsx` already provides a `Radio` component. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-09/candidate-layout-radio-report.json`. | `third_party/typeduck-web/source/src/Preferences.tsx`, `third_party/typeduck-web/source/src/Inputs.tsx`, `third_party/typeduck-web/source/src/consts.ts`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`. | Close when Candidate Menu Layout renders as radio choices, not a segmented group; both horizontal and vertical options remain labeled, keyboard-accessible, and bound to `candidateMenuLayout`; switching options still changes the candidate panel layout. Add Playwright evidence for selecting both options and capturing the resulting panel layout; regenerate and reverse/forward check `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` if source changed. |
| M25-DOGFOOD-10 | Open | UI polish / layout alignment | The `輸入法設定 IME Settings` section appears to use a different horizontal margin than the playground section above it. The user wants the settings block aligned with the content above so the page columns share the same left/right edges. | User-reported during manual dogfooding on 2026-06-21. Local verification found `App.tsx` renders the main playground inside `<main className="m-auto p-8 max-w-7xl">`, while `Preferences.tsx` wraps IME Settings in a nested `<section className="mx-auto max-w-6xl">`, making settings narrower than the preceding content stack. Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-10/ime-settings-margin-alignment-report.json`. | `third_party/typeduck-web/source/src/App.tsx`, `third_party/typeduck-web/source/src/Preferences.tsx`, `third_party/typeduck-web/source/src/index.css`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`. | Close when the IME Settings heading and settings grid align horizontally with the content above on desktop and narrow viewports. Add Playwright evidence comparing bounding boxes for the playground content and IME Settings wrapper; left and right edge differences should be within 2 px unless a documented scrollbar/device-pixel reason requires a slightly wider tolerance. Preserve the settings grid behavior from `M25-DOGFOOD-06` and regenerate plus reverse/forward check `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` if source changed. |

## Accepted Review Corrections

- `M25-DOGFOOD-01` startup work must address the two validated P0 suspects before broader micro-optimization: the browser WASM artifact is currently built from `target/wasm32-unknown-emscripten/debug`, and the TypeDuck-Web adapter forces deploy freshness by calling `invalidateDeployedSchema(...)` before `currentRuntime.deploy()`. The review evidence is tracked at `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-01/claude-review-p0-notes.json`.
- Do not treat fast asset fetches as proof that startup is network-bound. The current reproduction showed assets loading in under 100 ms while `runtime:initialized` consumed about 47 seconds, so Task 2 must re-measure release WASM and deploy reuse before setting the final warm-reload budget.

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

### Task 2: M25-DOGFOOD-01 Startup Performance Optimization

**Files:**
- Modify: `Cargo.toml`
- Modify: `scripts/typeduck-wasm-build.sh`
- Modify: `third_party/typeduck-web/source/src/worker.ts`
- Modify: `third_party/typeduck-web/source/src/yune-integration/adapter.ts`
- Modify if profiling still points there: `packages/yune-typeduck-runtime/src/typeduck.ts`
- Modify if profiling still points there: `packages/yune-typeduck-runtime/src/filesystem.ts`
- Modify if profiling still points there: `crates/yune-rime-api/src/typeduck_web.rs`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-01/`

- [ ] **Step 1: Preserve the failing startup characterization and artifact facts**

  Keep the current reload evidence as the baseline, then add a Playwright startup budget characterization that reloads `/web/`, waits for `document.documentElement.dataset.yuneLoading === "false"`, captures the latest `startup:complete` diagnostic, and writes `m25-dogfooding/M25-DOGFOOD-01/startup-before.json`. Also record the shipped `.wasm` size, whether the build came from `debug` or `release`, and whether Emscripten/wasm optimization flags were applied. The first test may fail against the current ~47s local baseline, but the final budget should be chosen after the P0 fixes below are measured.

- [ ] **Step 2: Ship release-mode browser WASM first**

  Update the TypeDuck-Web build path so the browser artifact is not copied from `target/wasm32-unknown-emscripten/debug`. Add conservative release tuning in `Cargo.toml` for the existing workspace (`opt-level = 3`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, and `strip = true` if supported by the active toolchain), update `scripts/typeduck-wasm-build.sh` to build `typeduck_web_module` with `--release`, copy from `target/wasm32-unknown-emscripten/release`, and add Emscripten optimization flags such as `-O3` or `-Oz` to the link path. Run `wasm-opt -O3` only when available and record whether it was skipped. Preserve the existing JS glue contract: `createYuneTypeduckModule`, `cwrap`, `UTF8ToString`, `FS`, and `IDBFS` must still exist after the build.

- [ ] **Step 3: Stop rebuilding/deploying fresh state on every reload**

  Remove or guard the unconditional `invalidateDeployedSchema(currentFs, currentPrepareOptions)` call before `currentRuntime.deploy()`. Do not simply delete the safety check; replace it with freshness validation based on `assetVersion`, schema/resource checksums, or another deterministic deploy stamp. Persist and reuse `/rime/build` through IDBFS when fresh. If `jyut6ping3_mobile` still spends startup time generating stable table/reverse artifacts, ship or prefer precompiled artifacts only after adding fixture-backed evidence that the resulting candidates and paging behavior remain unchanged.

- [ ] **Step 4: Re-measure after the P0 fixes before choosing deeper work**

  Rebuild the patchable TypeDuck-Web source, reload `/web/`, and save `startup-after-release-wasm.json` and `startup-after-deploy-reuse.json` as applicable. Compare `startup:complete.totalMs`, the user-visible loading duration, `.wasm` size, and deploy/cache diagnostics against `reload-startup-repro.json`. The final warm-reload budget must be written from this measured optimized baseline, not guessed from the pre-fix 47s number.

- [ ] **Step 5: Add finer `runtime:initialized` phase markers only if needed**

  If release WASM plus deploy reuse do not meet the startup budget, split `runtime:initialized` into nested markers around `TypeDuckRuntime.init`, filesystem mount/sync, schema deploy, default schema selection, dictionary/table loading, and persistence sync. The next evidence file must show which sub-phase owns the remaining delay.

- [ ] **Step 6: Keep startup latency separate from typing latency**

  Startup optimization must not hide the separate typing-stutter problem. If typing is still blocked while startup continues, keep `M25-DOGFOOD-03` open and measure it separately instead of claiming startup fixed the perceived performance problem.

- [ ] **Step 7: Prove the improvement and regenerate the TypeDuck-Web patch if source changed**

  Re-run the startup test, save the final `startup-after.json`, and compare `startup:complete.totalMs` plus the visible loading duration against the baseline evidence. The final row update must state before/after timings, the release/deploy evidence paths, and any remaining bottleneck. If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 3: M25-DOGFOOD-02 Page-Size Slider And Candidate Cap

**Files:**
- Modify: `third_party/typeduck-web/source/src/Preferences.tsx`
- Modify if needed: `third_party/typeduck-web/source/src/Inputs.tsx`
- Modify if needed: `third_party/typeduck-web/source/src/CandidatePanel.tsx`
- Modify if needed: `third_party/typeduck-web/source/src/yune-integration/adapter.ts`
- Modify if needed: `crates/yune-rime-api/src/typeduck_web.rs`
- Test: `crates/yune-rime-api/tests/typeduck_web.rs`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-02/`

- [ ] **Step 1: Add failing native page-size coverage for 3 and 9**

  Extend `typeduck_adapter_real_assets_page_size_customize_limits_context_page` or add a focused companion test that customizes `menu/page_size` to `3` and `9`, types a high-candidate input such as `hai`, and asserts `context.page_size` and `context.candidates.len()` match the requested cap.

- [ ] **Step 2: Add failing browser coverage for the visible control**

  Add a Playwright test that locates the page-size control by its Cantonese/English label, asserts the range is `3` through `10`, sets it to `9`, types `hai`, and asserts exactly 9 visible candidate rows. Repeat with `3` to protect the lower bound.

- [ ] **Step 3: Fix discoverability and the allowed range**

  Keep the control in the settings UI, but make it visibly discoverable in the candidate/display area and set its range to `3..10`. The visible value must update immediately when the slider changes.

- [ ] **Step 4: Fix the runtime cap**

  Trace the browser response after `Rime.customize({ pageSize })`. If native `typeduck_web` returns a paged `context.candidates` array but the browser renders all candidates, fix `CandidatePanel` or the adapter mapping. If native returns too many candidates for `menu/page_size`, fix `typeduck_web.rs` or `context_api.rs`. Do not hide extra rows with CSS while leaving selection/page navigation semantics wrong.

- [ ] **Step 5: Prove paging and selection still work**

  The browser test must verify first-page row count, next/previous page behavior, and digit selection after the cap is applied. Save JSON and screenshots under `M25-DOGFOOD-02`.

- [ ] **Step 6: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 4: M25-DOGFOOD-03 Typing Responsiveness And Loading-State Separation

**Files:**
- Modify: `third_party/typeduck-web/source/src/CandidatePanel.tsx`
- Modify: `third_party/typeduck-web/source/src/rime.ts`
- Modify: `third_party/typeduck-web/source/src/worker.ts`
- Modify: `third_party/typeduck-web/source/src/App.tsx`
- Modify: `third_party/typeduck-web/source/src/Toolbar.tsx`
- Modify if profiling points there: `third_party/typeduck-web/source/src/yune-integration/adapter.ts`
- Modify if profiling points there: `packages/yune-typeduck-runtime/src/typeduck.ts`
- Modify if profiling points there: `crates/yune-rime-api/src/typeduck_web.rs`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-03/`

- [ ] **Step 1: Add keypress latency instrumentation**

  Add temporary or permanent diagnostics that record `keydown` time, `Rime.processKey` queued time, worker receive time, Rust response time, page receive time, and candidate-panel render time. Save a browser JSON baseline for `h`, `ha`, `hai`, `nei`, and `jigaajiusihaa` under `M25-DOGFOOD-03`.

- [ ] **Step 2: Add a failing Playwright responsiveness test**

  Add a Playwright test that types into the real textarea using actual keypresses and asserts an explicit latency budget for keydown-to-candidate update. The first version should fail or record the current bad p95/p99 latency so the implementation has a hard target.

- [ ] **Step 3: Separate global loading state from normal composition**

  `è¼‰å…¥ä¸­ Loading...` should mean startup, schema deploy, or settings redeploy, not ordinary per-key composition. Split loading state in `App.tsx`/`Toolbar.tsx`/`rime.ts` so normal `processKey`, `stageAi`, page flip, and candidate selection do not keep the global loading indicator active. If an action needs a local pending state, show it near that control instead of blocking the textbox.

- [ ] **Step 4: Avoid queueing keystrokes behind long non-key actions**

  Inspect the `rime.ts` single-message queue. If `processKey` messages wait behind startup/customize/deploy, split high-priority key events from low-priority settings work, coalesce stale option/customize calls, or block typing until the IME is truly ready with an explicit disabled state. Do not let the user type into an apparently live IME while key events are silently delayed.

- [ ] **Step 5: Reduce the slowest measured per-key path**

  If latency is dominated by Rust `process_key`, profile `yune_typeduck_process_key` and candidate serialization. If latency is dominated by rendering, cap rendered rows through `M25-DOGFOOD-02` and avoid rebuilding expensive dictionary/detail data for every row. If latency is dominated by AI staging, ensure AI remains second-pass, cancellable/stale-result guarded, and default-off.

- [ ] **Step 6: Prove typing responsiveness after the fix**

  Re-run the browser responsiveness test and save `typing-latency-after.json` with before/after p50/p95/p99 timings. The final evidence must show the textbox accepts keypresses without a visible one-second stall and the global loading indicator does not appear for normal composition.

- [ ] **Step 7: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 5: M25-DOGFOOD-04 Compact Schema Switcher And Luna Label

**Files:**
- Modify: `third_party/typeduck-web/source/src/App.tsx`
- Modify: `third_party/typeduck-web/source/src/Toolbar.tsx`
- Modify: `third_party/typeduck-web/source/src/SchemaSwitcher.tsx`
- Modify: `third_party/typeduck-web/source/src/consts.ts`
- Modify if segment markup needs shared layout changes: `third_party/typeduck-web/source/src/Inputs.tsx`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-04/`

- [ ] **Step 1: Preserve the schema-name verification**

  Keep `schema-switcher-toolbar-and-luna-name-report.json` as the issue baseline. The implementation should treat the current web label `普通話` as a UI/source mismatch to resolve, not as proof that the user-facing label is correct. Upstream `1.17.0` names `luna_pinyin` as `朙月拼音`; TypeDuck v1.1.2 names it `普通話`, so any decision to keep `普通話` must be documented in this row with explicit TypeDuck-specific rationale.

- [ ] **Step 2: Add failing browser layout coverage**

  Add a Playwright test that loads `/web/` at a desktop viewport and asserts the schema switcher belongs to the same top control band as the ASCII/simplification/full-shape buttons. The test should check that `[data-yune-schema-switcher]` is inside or directly owned by the toolbar/top-controls component, the top edges of the three mode buttons and schema segments are aligned within one compact row, and no separate standalone schema block appears between the toolbar and status strip. Add a narrow viewport assertion that the same controls wrap within one compact band instead of becoming a separate full-width section.

- [ ] **Step 3: Move the schema switcher into the toolbar row**

  Refactor `Toolbar.tsx` to accept `activeSchema` and `setActiveSchema`, render `SchemaSwitcher` next to the three mode buttons, and keep the loading indicator in the same top control area without pushing the textarea downward. Remove the standalone `<SchemaSwitcher ... />` call from `App.tsx`. Keep accessible labels for each schema option and preserve the existing `data-yune-schema-switcher` hook for tests.

- [ ] **Step 4: Make the Luna label explicit**

  Update `SCHEMA_OPTIONS` in `consts.ts` so `luna_pinyin` uses `朙月拼音` as the primary visible `schemaName`. Keep `Luna Pinyin` or `Mandarin` only as a secondary label or tooltip if useful. Do not change engine output/ranking behavior for this UI-only row.

- [ ] **Step 5: Prove schema switching still works**

  In the browser test, switch from `jyut6ping3_mobile` to `luna_pinyin` and back, assert the visible active option changes, assert the Luna option text includes `朙月拼音`, and type a short smoke input after switching to prove the selector still drives the worker path.

- [ ] **Step 6: Capture visual evidence**

  Save desktop and narrow-viewport screenshots plus a JSON summary under `M25-DOGFOOD-04`. The JSON should include viewport size, active schema before/after switching, bounding boxes for the three mode buttons and schema switcher, and the visible text for the Luna option.

- [ ] **Step 7: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 6: M25-DOGFOOD-05 Cangjie Version In Top Controls

**Files:**
- Modify: `third_party/typeduck-web/source/src/App.tsx`
- Modify: `third_party/typeduck-web/source/src/Toolbar.tsx`
- Modify: `third_party/typeduck-web/source/src/SchemaSwitcher.tsx` if the schema row becomes the shared top-control layout
- Modify: `third_party/typeduck-web/source/src/Preferences.tsx`
- Modify if segment markup needs shared compact styling: `third_party/typeduck-web/source/src/Inputs.tsx`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-05/`

- [ ] **Step 1: Preserve the Cangjie-version baseline**

  Keep `cangjie-version-top-controls-report.json` as the issue baseline. The current control lives in `Preferences.tsx` under `Web Frontend Controls`, uses `prefs.isCangjie5`, and exposes `三代 Version 3` plus `五代 Version 5`. The implementation must move this existing control, not create a second independent setting.

- [ ] **Step 2: Add failing browser layout coverage**

  Add a Playwright test that loads `/web/` at a desktop viewport and asserts the Cangjie version control is in the same top control band as `[data-yune-schema-switcher]`. The test should assert that the `三代 Version 3` and `五代 Version 5` controls appear before the status strip and textarea, and that no lower section titled `Web Frontend Controls` remains when this was its only control. Add a narrow viewport assertion that schema selection and Cangjie version wrap within the same compact top band.

- [ ] **Step 3: Move the control into the top control component**

  Pass `isCangjie5` and `setIsCangjie5` from `App.tsx` into the top toolbar/control component created or updated by `M25-DOGFOOD-04`. Render a compact segmented control next to the schema selector with the label `倉頡版本 Cangjie version`, options `三代 Version 3` and `五代 Version 5`, and the existing `Segment` interaction pattern. Keep the control keyboard-accessible and do not add another UI framework.

- [ ] **Step 4: Remove the empty lower section**

  Delete the `Web Frontend Controls` section from `Preferences.tsx` if moving Cangjie version leaves it empty. If later rows add other frontend controls, keep those controls in an appropriately named settings group, but do not keep a one-control Cangjie section at the bottom.

- [ ] **Step 5: Prove the existing customize path still works**

  In Playwright, switch to the Cangjie schema, choose `三代 Version 3`, apply or trigger the existing settings flow, then choose `五代 Version 5`. Capture the active segment state and a worker/debug signal showing `isCangjie5` changed, or use the existing UI/evidence hook that proves `Rime.customize({ isCangjie5 })` was invoked. Do not change engine behavior or Cangjie candidate ordering in this UI row.

- [ ] **Step 6: Capture visual evidence**

  Save desktop and narrow-viewport screenshots plus a JSON summary under `M25-DOGFOOD-05`. The JSON should include viewport size, bounding boxes for schema selector and Cangjie version, active Cangjie version before/after toggling, and whether any lower `Web Frontend Controls` section remains.

- [ ] **Step 7: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 7: M25-DOGFOOD-06 Display Controls Above Live Session Controls

**Files:**
- Modify: `third_party/typeduck-web/source/src/Preferences.tsx`
- Modify if shared section markup needs a test hook: `third_party/typeduck-web/source/src/Inputs.tsx`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-06/`

- [ ] **Step 1: Preserve the section-order baseline**

  Keep `display-live-section-order-report.json` as the issue baseline. The current `Preferences.tsx` order is engine/session dogfood controls, `即時狀態 Live session controls`, `顯示設定 Display controls`, then `網頁前端 Web Frontend Controls`. The requested order swaps Display controls and Live session controls without changing the controls inside either section.

- [ ] **Step 2: Add failing browser layout coverage**

  Add a Playwright test that loads `/web/` at a desktop viewport and locates the headings `顯示設定 Display controls` and `即時狀態 Live session controls`. Assert that Display controls is visually above Live session controls and occupies the top-right settings-grid slot. Use bounding boxes rather than only DOM order, because the user complaint is about the rendered two-column layout. Add a narrow viewport assertion that Display controls still appears before Live session controls in a readable single-column flow.

- [ ] **Step 3: Swap the sections in `Preferences.tsx`**

  Move the full `顯示設定 Display controls` section above the `即時狀態 Live session controls` section. Preserve every existing Display control: display languages, page size, candidate menu layout, Chinese typeface, candidate Jyutping, and reverse code display. Preserve every existing Live session control: ASCII mode, full shape, simplification, traditionalization, extended charset, and disabled.

- [ ] **Step 4: Prove state bindings still work**

  Extend the browser test to toggle one Display control and one Live session control after the swap. Use existing UI state hooks where available, or assert the active/checked DOM state directly. This prevents a layout-only edit from accidentally detaching `prefs.displayLanguages`, `prefs.pageSize`, `prefs.isAsciiMode`, or related state setters.

- [ ] **Step 5: Capture visual evidence**

  Save desktop and narrow-viewport screenshots plus a JSON summary under `M25-DOGFOOD-06`. The JSON should include viewport size, section heading order, bounding boxes for Display controls and Live session controls, and the toggled control state checked in Step 4.

- [ ] **Step 6: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 8: M25-DOGFOOD-07 Checkbox Affordance For Binary Controls

**Files:**
- Modify: `third_party/typeduck-web/source/src/Inputs.tsx`
- Modify: `third_party/typeduck-web/source/src/index.css`
- Modify: `third_party/typeduck-web/source/src/App.tsx`
- Modify if the theme toggle is included in the change: `third_party/typeduck-web/source/src/ThemeSwitcher.tsx`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-07/`

- [ ] **Step 1: Preserve the binary-control baseline**

  Keep `raw-checkbox-binary-controls-report.json` as the issue baseline. The current shared `Toggle` component in `Inputs.tsx` renders `input type="checkbox"` with `className="yd-switch"`, and `.yd-switch` in `index.css` creates the rounded pill switch shown in the user screenshot. `.yd-check` already exists as the square checkbox styling used by checklist controls.

- [ ] **Step 2: Add failing browser affordance coverage**

  Add a Playwright test that loads `/web/`, finds representative binary settings in the preferences area, and asserts they do not use the rounded pill visual dimensions. Cover at least one engine/session toggle, one live-session toggle, one display/settings toggle if present after `M25-DOGFOOD-06`, and the Yune inspector toggle. The test should record each control's label, checked state, class list, bounding box, and computed border radius.

- [ ] **Step 3: Change the shared `Toggle` presentation to checkbox style**

  Update `Toggle` in `Inputs.tsx` to render the binary control with the checkbox-style class used by raw checkboxes, or rename the shared class so both `Toggle` and `Checkbox` use the same square checkbox affordance. Keep semantic `input type="checkbox"`, label click behavior, checked binding, `NO_AUTO_FILL`, and focus-ring accessibility. Do not change `Radio`, `Segment`, or `Range` controls.

- [ ] **Step 4: Retire or narrow `yd-switch` usage**

  Remove the rounded-pill styling from settings-panel binary controls. Update the Yune inspector toggle in `App.tsx` to use the same checkbox affordance. Decide explicitly in the row evidence whether `ThemeSwitcher.tsx` remains a specialized icon switch because it overlays sun/moon icons, or whether it also becomes a checkbox-style control. If the theme toggle remains specialized, rename its class or add a comment so `yd-switch` no longer means generic binary setting.

- [ ] **Step 5: Prove checked and unchecked states remain obvious**

  In Playwright, toggle representative controls on and off and assert both states are visible through the checkbox mark/fill, not only through surrounding text. Verify keyboard focus still lands on the input and Space toggles the checked state.

- [ ] **Step 6: Capture visual evidence**

  Save desktop and narrow-viewport screenshots plus a JSON summary under `M25-DOGFOOD-07`. The JSON should include the class names and computed styles for each checked control, unchecked control, and any explicitly exempted theme toggle.

- [ ] **Step 7: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 9: M25-DOGFOOD-08 Bare-Grave Luna Reverse Lookup Trigger

**Files:**
- Modify: `third_party/typeduck-web/source/schema/jyut6ping3.schema.yaml`
- Modify: `third_party/typeduck-web/source/schema/jyut6ping3_mobile.schema.yaml`
- Modify: `third_party/typeduck-web/source/src/consts.ts`
- Modify if schema installation rejects bare grave overlap: `crates/yune-rime-api/src/schema_install.rs`
- Test: `crates/yune-rime-api/tests/typeduck_web.rs`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-08/`

- [ ] **Step 1: Preserve the trigger baseline and superseded assumption**

  Keep `reverse-lookup-bare-grave-trigger-report.json` as the issue baseline. Current web schemas use ``luna_pinyin.prefix: "`p"`` and ``luna_pinyin: "^`p[a-z']*;?$"`` under `speller/algebra`. TypeDuck v1.1.2 captured schema data also uses `` `p``, while the new M25 dogfood request explicitly wants the bare grave accent. Treat this as an intentional web dogfood correction request and record any oracle/profile tension in the final evidence.

- [ ] **Step 2: Add failing native/schema coverage**

  Add or extend a `typeduck_web` test that installs the Jyutping web/mobile schema and proves an input beginning with bare grave, for example `` `zhe``, routes into the `luna_pinyin` affix translator. The test should also record whether `` `pzhe`` remains accepted as a compatibility alias or is no longer required. If accepting both triggers would create ambiguous segmentation, document the chosen behavior before implementation.

- [ ] **Step 3: Add failing browser coverage**

  Add a Playwright test that loads `/web/`, keeps the Jyutping schema active, types `` `zhe``, and asserts the candidate/debug state identifies the luna_pinyin reverse-lookup path without requiring the `p` prefix. Capture whether the generic operation-error toast appears; if the toast appears because the worker is blocked or slow rather than because of trigger parsing, link that finding back to `M25-DOGFOOD-01` and `M25-DOGFOOD-03` instead of broadening this row.

- [ ] **Step 4: Change the schema trigger**

  Update the `luna_pinyin` affix translator settings in `jyut6ping3.schema.yaml` and `jyut6ping3_mobile.schema.yaml` so the prefix is bare `` ` `` and the alphabet regex accepts bare-grave Mandarin pinyin input, for example `` ^`[a-z']*;?$ ``. Make the smallest schema change that does not disturb Cangjie, Loengfan, or other reverse-lookup affixes.

- [ ] **Step 5: Update user-facing hints**

  Update `SCHEMA_OPTIONS` in `consts.ts` so the Jyutping reverse-lookup example no longer says `` `pzhe -> ... ``. Use a bare-grave example such as `` `zhe -> 這（普通話反查） ``. Keep the Cangjie and Luna schema hints unchanged unless the browser test proves they are also wrong.

- [ ] **Step 6: Prove behavior and error handling**

  Run the native/schema test and browser test. Save JSON evidence with the typed input, active schema id, reverse-lookup tag/source if available, first candidates, whether the operation-error toast appeared, and per-key timing if available. If reverse lookup remains slow after the trigger fix, leave or cross-link the latency to `M25-DOGFOOD-01`/`M25-DOGFOOD-03`; open a new row only if the trigger fix exposes a distinct crash.

- [ ] **Step 7: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 10: M25-DOGFOOD-09 Candidate Layout Radio Selection

**Files:**
- Modify: `third_party/typeduck-web/source/src/Preferences.tsx`
- Modify if radio markup/styling needs adjustment: `third_party/typeduck-web/source/src/Inputs.tsx`
- Modify if labels need copy changes: `third_party/typeduck-web/source/src/consts.ts`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-09/`

- [ ] **Step 1: Preserve the control baseline**

  Keep `candidate-layout-radio-report.json` as the issue baseline. Current `Preferences.tsx` renders `候選排版 Candidate Menu Layout` as `Segment` options backed by `prefs.candidateMenuLayout`; `Inputs.tsx` already exposes `Radio`.

- [ ] **Step 2: Add failing browser coverage**

  Add a Playwright test that finds the Candidate Menu Layout field and asserts it contains radio inputs for horizontal/vertical instead of `.yd-segment` labels. Assert the accessible names include the existing labels from `CANDIDATE_MENU_LAYOUT_LABELS`.

- [ ] **Step 3: Replace the segmented control with radios**

  In `Preferences.tsx`, replace the `Segment` mapping for `candidateMenuLayout` with `Radio` components using the same `name`, labels, `prefs.candidateMenuLayout`, and `prefs.setCandidateMenuLayout`. Do not change `Segment` globally because schema, Cangjie version, and other controls may still need segmented styling.

- [ ] **Step 4: Prove layout behavior remains intact**

  In Playwright, select horizontal and vertical radio options and type a short input such as `hai`. Assert the selected radio state changes and the candidate panel uses the expected horizontal/vertical layout class or DOM shape for each option.

- [ ] **Step 5: Capture evidence**

  Save desktop and narrow-viewport screenshots plus JSON under `M25-DOGFOOD-09`. Include radio labels, selected value before/after, and candidate panel layout evidence.

- [ ] **Step 6: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

### Task 11: M25-DOGFOOD-10 IME Settings Horizontal Alignment

**Files:**
- Modify: `third_party/typeduck-web/source/src/Preferences.tsx`
- Modify if shared layout styling is introduced: `third_party/typeduck-web/source/src/App.tsx`
- Modify if a reusable class is needed: `third_party/typeduck-web/source/src/index.css`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Evidence: `third_party/typeduck-web/e2e/results/m25-dogfooding/M25-DOGFOOD-10/`

- [ ] **Step 1: Preserve the alignment baseline**

  Keep `ime-settings-margin-alignment-report.json` as the issue baseline. Current `App.tsx` renders the playground stack directly inside `main.m-auto.p-8.max-w-7xl`, while `Preferences.tsx` adds a nested `mx-auto max-w-6xl` wrapper around `輸入法設定 IME Settings`.

- [ ] **Step 2: Add failing visual-layout coverage**

  Add a Playwright test that loads `/web/` at a desktop viewport, locates the textarea or first stable playground content block above settings, locates the `輸入法設定 IME Settings` wrapper, and compares bounding boxes. Assert `Math.abs(playground.left - settings.left) <= 2` and `Math.abs(playground.right - settings.right) <= 2`.

- [ ] **Step 3: Add narrow-viewport coverage**

  Repeat the bounding-box check at a narrow mobile viewport. Assert the settings wrapper does not introduce extra side margins compared with the textarea/main playground content and that no settings heading text overflows.

- [ ] **Step 4: Remove the nested width mismatch**

  In `Preferences.tsx`, remove the nested `mx-auto max-w-6xl` constraint or replace it with a shared class that inherits the main content width. Prefer the minimal change: make the preferences root use the same available width as the content above, without changing `main` width, section card padding, or the two-column settings grid.

- [ ] **Step 5: Protect adjacent M25 layout changes**

  Run the layout checks added for `M25-DOGFOOD-06`, `M25-DOGFOOD-07`, and `M25-DOGFOOD-09` if they exist by implementation time. If they do not exist yet, capture a JSON note under `M25-DOGFOOD-10` stating that the alignment fix preserved the current section order, checkbox/radio affordances, and settings grid class names.

- [ ] **Step 6: Capture evidence**

  Save desktop and narrow-viewport screenshots plus JSON under `M25-DOGFOOD-10`. Include viewport sizes, left/right bounding-box values for the playground content and IME Settings wrapper, the pixel deltas, and the final preferences root class name.

- [ ] **Step 7: Regenerate the TypeDuck-Web patch if source changed**

  If any file under `third_party/typeduck-web/source/` changed, regenerate `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`, reverse-check it from `source/`, and forward-check it on a clean source checkout.

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
