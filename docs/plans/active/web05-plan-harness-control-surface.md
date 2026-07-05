# WEB-05 Harness Control Surface Implementation Plan

> **For agentic workers:** If a plan-execution sub-skill is available, use it;
> otherwise execute the checkboxes directly, in order. Steps use checkbox
> (`- [ ]`) syntax for tracking.

> **Status:** In progress - Phase 0 controls ledger committed (2026-07-05, `docs/reports/evidence/web05-control-surface/controls-ledger.md`: 14 surface rows, 2 named deferred rows); running on the Windows machine in parallel with M57 (macOS). Phase 1 next. - **Track:** Web harness (`apps/yune-web`). - **Created:** 2026-07-03 - **Type:** dogfooding/observability slice (app plumbing + UI only; no engine contract change, no default-behavior change).

**Goal:** Make the `yune-web` playground the shared debugging surface for the
external Windows/iOS frontend teams: **every engine control and diagnostic the
existing web ABI already exposes** becomes reachable and observable in the
harness, so an engine bug can be reproduced with a URL + a few clicks instead
of a native debug build. When M56 lands its new diagnostics (staleness,
recovery, crash policy), this surface is where they become visible — but this
plan is **not** blocked on M56.

**Origin:** this is the "surface all controls" follow-up explicitly carved out
of M21 as its own future milestone (see the M20/M21 scope boundary: M20 built
the playground, M21 dispositioned the product-comparison ledger; surfacing all
controls was deferred as a new milestone).

## Decided Calls

- **No engine changes** (`crates/` untouched) and **no
  `packages/yune-web-runtime` changes, period.** A control that would require
  either becomes a named deferred row in the ledger (engine-lane or
  runtime-lane), not scope creep — the WEB-04 precedent: work through the
  existing adapter/worker seams only.
- **Default behavior unchanged.** Every new control starts at the current
  default; the negative control (existing e2e suite green, unchanged
  candidates on default settings) is part of the gate.
- **Public demo stays product-shaped.** Debug controls are dev-harness-only;
  the public demo build must not grow debug surface (the WEB-04 review's
  public-demo-leak lesson: `SCHEMA_OPTIONS`-style registries render unfiltered
  in the demo build). **The gating mechanism, named precisely — there is no
  single "dev flag" today:** the demo is identified by
  `import.meta.env.VITE_YUNE_PUBLIC_DEMO === "1"` (inline checks exist in
  `src/SchemaSwitcher.tsx` and `src/hooks.ts`) and by the esbuild define
  `YUNE_PUBLIC_DEMO_BUILD` set in `public-demo/build.mjs`. Hoist one shared
  `IS_PUBLIC_DEMO` constant into `src/consts.ts` from the `VITE_` check and
  gate every new control on it. Do **not** use `import.meta.env.DEV` — that
  would hide the controls from the production *harness* build, which is the
  opposite of the goal.
- **Browser-visible claims need real-browser (Playwright) evidence**, per the
  standing repo rule.

## Current Starting Point

Verified repo facts (2026-07-03):

- The worker exposes an action-based protocol (`apps/yune-web/src/worker.ts`:
  deploy, schema select, option set, userdb import, memory snapshots,
  diagnostics via `postMessage({type: "diagnostic", source: ...})`), consumed
  through `apps/yune-web/src/rime.ts` listeners and surfaced in
  `App.tsx`/`Preferences.tsx`.
- WEB-04 established the diagnostic-surface pattern end to end: worker
  diagnostic → `grammarDiagnosticChanged` listener → `<html>` dataset +
  inspector metric with `data-*` attributes → Playwright assertions. Reuse
  this pattern; do not invent a parallel one.
- The engine-side web ABI is the 14 `yune_web_*` exports
  (`crates/yune-rime-api/src/web_runtime.rs`) — read-only reference for what
  is *reachable*; changing it is out of scope (a new export requires
  `scripts/yune-web-exports.txt` and is engine-lane work).
- Subtree guides override root for this area: `apps/yune-web/AGENTS.md`;
  browser smoke expectations: `apps/yune-web/e2e/yune-browser-smoke.md`.
- UI text goes through `apps/yune-web/src/uiText.ts` (per-language tables) —
  the WEB-04 review flagged hardcoded English literals as a defect pattern;
  all new labels are uiText entries from the start.

## Phase 0: Controls Ledger (read-only)

- [ ] Enumerate every control/diagnostic reachable through the existing
  seams: worker actions, `yune_web_*` exports and their option/config
  parameters, schema switches (`switches:` in the deployed schemas), engine
  options (`set_option` names used anywhere in the repo), deploy/redeploy
  triggers, userdb tools, memory/status snapshots, and existing diagnostics.
- [ ] Commit `docs/reports/evidence/web05-control-surface/controls-ledger.md`
  with one row per control: name, seam, currently surfaced? (where),
  disposition (`surface` / `already-surfaced` / `engine-lane-deferred` with
  the missing export named).
- [ ] Phase gate: ledger committed; typecheck untouched.

## Phase 1: Surface The Controls

Work the ledger top-down; for each `surface` row:

- [ ] Add the control to the playground UI (Preferences/inspector as
  appropriate), wired through the existing worker action protocol; labels via
  `uiText.ts` (all languages); state observable via `data-*` attributes
  following the WEB-04 diagnostic pattern.
- [ ] Keep dev-only controls out of the public demo build (build-flag gate),
  and keep every default identical to today.
- [ ] Group related controls (schema/options/deploy/userdb/diagnostics) —
  one inspector section per group, not one-off widgets.
- [ ] Gate per batch — exact invocations (`apps/yune-web` has no vitest
  dependency or `test` script of its own; the e2e suite is the separate
  `apps/yune-web/e2e` subpackage and needs the dev server on
  `localhost:5173`):

  ```powershell
  npm.cmd --prefix apps/yune-web run typecheck
  # focused vitest for new pure logic, run from apps/yune-web:
  ..\..\packages\yune-web-runtime\node_modules\.bin\vitest.cmd run yune-integration/<new>.test.ts
  # e2e (dev server running):
  npm.cmd --prefix apps/yune-web/e2e run test:e2e
  ```

## Phase 2: Evidence + Closeout

- [ ] Playwright evidence: for each control group, a test that flips the
  control and asserts the observable engine effect (option state, candidate
  change, diagnostic value), plus the **negative control**: default settings
  produce byte-identical candidate behavior to the pre-WEB-05 harness for the
  standing smoke inputs. **Both the baseline and post-change runs must be
  captured against the same built WASM commit** — if M55/M56 engine work lands
  mid-milestone, re-capture the baseline on the new WASM before attributing
  any diff to WEB-05.
- [ ] Public-demo build evidence: the demo contains no new debug surface
  (assert absence via the built bundle or a demo-mode Playwright pass).
- [ ] Record evidence under `docs/reports/evidence/web05-control-surface/`;
  update roadmap/requirements/milestone-history; move this plan to
  `docs/plans/completed/`.

## Definition Of Done

The controls ledger is fully dispositioned (surfaced, already-surfaced, or
named engine-lane-deferred); every surfaced control has Playwright evidence of
effect; defaults and the public demo are proven unchanged; no `crates/` or
`packages/yune-web-runtime` diffs.

## Proposed Requirement IDs (closeout only)

(House style per `docs/requirements.md`: `<MILESTONE>-<TOPIC>-<NN>`, e.g.
`WEB04-OCTAGRAM-01`.)

- **WEB05-SURFACE-01**: A committed controls ledger enumerates every engine
  control/diagnostic reachable through existing seams, fully dispositioned
  (surfaced / already-surfaced / engine-lane-deferred / runtime-lane-deferred).
- **WEB05-SURFACE-02**: Every `surface`-row control is operable in the dev
  harness with localized labels and observable `data-*` state, defaults
  unchanged, gated out of the public demo via the shared `IS_PUBLIC_DEMO`
  constant.
- **WEB05-SURFACE-03**: Playwright evidence covers each control group plus a
  same-WASM-commit default-behavior negative control; the public demo gains no
  debug surface.

## Review Prompt

> Please review `docs/plans/active/web05-plan-harness-control-surface.md` as
> the WEB-05 plan. Focus on: whether the ledger-first structure keeps scope
> honest (surface only what existing seams reach; engine gaps deferred by
> name); whether the WEB-04 diagnostic pattern is correctly reused; whether
> default-behavior and public-demo protection gates are sufficient; and
> whether it can truly run in parallel with M55/M56 without lane mixing.
