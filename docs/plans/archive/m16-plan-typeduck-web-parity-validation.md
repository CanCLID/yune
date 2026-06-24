# M16 — TypeDuck-Web Fork-Parity Validation Implementation Plan

> **Status:** Finished · **Milestone:** M16 (TypeDuck-Web fork parity — validate) · **Closed:** 2026-06-19 · **Type:** execution plan

> **For agentic workers:** implement **task-by-task**; the deliverable is **committed real-browser evidence** that the TypeDuck-Web example behaves like the fork for all captured target behaviors. Depends on **M15** (behaviors implemented) and **M14** (goldens + userdb spike). A green written gate without browser evidence does not close M16.

**Goal:** Prove in a real browser that TypeDuck-Web, driven by Yune, is **fork-like for captured target behaviors exposed by the current app** plus the M13 AI layer — with deploy-only, UI-only, and userdb-inspection gaps explicitly listed.

**Architecture:** Extend the existing HR-5 Playwright harness ([e2e/yune-typeduck.spec.ts](../../apps/yune-web/e2e/yune-typeduck.spec.ts)) with parity scenarios; assert behavior against the M14 v1.1.2 goldens where applicable. Reuse the M12/M13 oracle-measured, non-circular discipline.

**Tech stack:** Playwright browser E2E, the patched TypeDuck-Web app + worker, `cargo`/`npm` gates.

## Non-goals

- New behaviors (M15) or new goldens (M14).
- The upstream language model or broad-upstream depth (Track 2 / M17–M19).

## Tasks

### Task 1 — Browser parity matrix (TYPEDUCK-PARITY-07)

- [x] Add E2E scenarios driving browser-exposed behavior with real `jyut6ping3_mobile` assets: default combined candidates, `enable_sentence` (`ngohaigo` → 我係個), completion, correction evidence, simplification, and the M13 AI scenarios. Capture screenshots + state JSON per scenario.
- [x] Where a behavior has an M14 golden and the browser exposes the same surface, assert the browser output matches it.
- **Acceptance:** browser-supported parity scenarios pass with zero console warning/error entries; evidence committed under `e2e/results/m16-parity-final-pass/`.

### Task 2 — Schema-menu surface (TYPEDUCK-PARITY-02 close-out)

- [x] Per the M14 finding, validate `hide_lone_schema`/`hide_caret` against the oracle-observable surface — if it was scoped to UI in M14, assert the TypeDuck-Web schema-selector visibility state directly.
- **Acceptance:** documented as a browser/UI gap: current TypeDuck-Web exposes no schema-selector DOM control; M14 `RimeGetSchemaList` remains the oracle evidence.

### Task 3 — Activate parity tests in the workspace

- [x] Confirm the M15-activated `cantonese_parity` tests are green in `cargo test --workspace`; any deferred fork-only behavior carries an explicit `#[ignore]` + blocker.
- **Acceptance:** workspace tests green; no silent gaps.

### Task 4 — userdb-pronunciation resolution (TYPEDUCK-PARITY-03 close-out)

- [x] Resolve per the M14 spike: either land the native inspection/validation path, or **explicitly list** it as the documented uncapturable fork-only gap in the parity statement.
- **Acceptance:** the gap is closed or explicitly enumerated — not implied as covered.

### Task 5 — Docs + verification gate

- [x] Flip roadmap M14/M15/M16 items and `TYPEDUCK-PARITY-01…07` to `Done` (or documented-deferral) as evidence lands; update the traceability table + coverage; add a `decisions.md` note if the parity outcome changes D-27's framing.
- [x] Run the full gate (each command separately):

```powershell
cargo fmt
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web   # keep the M9/M13 web ABI gate green
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/typeduck-wasm-build.sh                  # reproducible WASM/TypeDuck-Web worker build, or an actionable blocker
npm --prefix packages/yune-typeduck-runtime test
npm --prefix packages/yune-typeduck-runtime run build
git diff --check
```

- [x] Run the browser E2E (the core M16 proof) following `apps/yune-web/e2e/yune-browser-smoke.md` (install deps, build/serve the patched app + worker). Run Playwright from the e2e directory, which owns `playwright.config.ts`: `npm --prefix apps/yune-web/e2e install`, then `npx --prefix apps/yune-web/e2e playwright test yune-typeduck.spec.ts`. Commit screenshots + state JSON + `browser-run.log`.
- **Acceptance:** all gates pass; real-browser parity evidence committed.

## Completion criteria

- **Done = TypeDuck-Web is fork-like for current app-exposed captured behavior (plus M13 AI), with deploy-only/UI/userdb gaps explicitly listed**.
- The `cantonese_parity` ignored tests are activated where the engine path is implemented (or carry documented deferrals); committed real-browser evidence covers app-exposed parity behavior with zero console warning/error entries.
- `cargo test --workspace`, clippy `-D warnings`, TS runtime test/build, and the Playwright E2E pass.

## Outcome

- Browser-covered and asserted against M14 where applicable: default combined `hou` candidates, `ngohaigo` sentence top candidate, `ne` completion, `ngohaigo` simplification, M13 AI off/on/disable/commit safety, and the existing M9 smoke flows.
- Browser evidence recorded but not claimed as full native parity: disabling Auto-composition for full `ngohaigo`, `nri` correction default/enabled state.
- Explicit browser/userdb inspection limits: deploy-only `common:/separate_candidates`, deploy-only `common:/show_full_code` cangjie side lookup, schema-menu UI hiding, correction UI detail, and per-entry userdb pronunciation inspection. M14/M15 retain oracle-backed engine/userdb evidence for these.

## Review checklist

- [x] Parity claim is scoped to **captured app-exposed** behaviors; uncapturable fork-only/browser gaps are listed, not implied covered.
- [x] Browser evidence is committed and non-circular (asserts against M14 v1.1.2 goldens where the browser exposes the same surface).
- [x] No regression to M9/M13 web gates or classic input.
- [x] Docs/requirements reflect only the evidence that landed.
