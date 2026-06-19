# M16 — TypeDuck-Web Fork-Parity Validation Implementation Plan

> **Status:** Active · **Milestone:** M16 (TypeDuck-Web fork parity — validate) · **Updated:** 2026-06-19 · **Type:** execution plan

> **For agentic workers:** implement **task-by-task**; the deliverable is **committed real-browser evidence** that the TypeDuck-Web example behaves like the fork for all captured target behaviors. Depends on **M15** (behaviors implemented) and **M14** (goldens + userdb spike). A green written gate without browser evidence does not close M16.

**Goal:** Prove in a real browser that TypeDuck-Web, driven by Yune, is **fork-like for all captured target behaviors** (combine_candidates, show_full_code, enable_sentence, completion, correction, simplification) **plus the M13 AI layer** — with any uncapturable fork-only gap explicitly listed.

**Architecture:** Extend the existing HR-5 Playwright harness ([e2e/yune-typeduck.spec.ts](../../third_party/typeduck-web/e2e/yune-typeduck.spec.ts)) with parity scenarios; assert behavior against the M14 v1.1.2 goldens where applicable. Reuse the M12/M13 oracle-measured, non-circular discipline.

**Tech stack:** Playwright browser E2E, the patched TypeDuck-Web app + worker, `cargo`/`npm` gates.

## Non-goals
- New behaviors (M15) or new goldens (M14).
- The upstream language model or broad-upstream depth (Track 2 / M17–M19).

## Tasks

### Task 1 — Browser parity matrix (TYPEDUCK-PARITY-07)
- [ ] Add E2E scenarios driving each parity behavior with real `jyut6ping3_mobile` assets: combine_candidates on/off, show_full_code (cangjie sub-lookup), enable_sentence (e.g. `ngohaigo` → 我係個), completion, correction, and the simplification toggle. Capture screenshots + state JSON per scenario.
- [ ] Where a behavior has an M14 golden, assert the browser output matches it.
- **Acceptance:** all parity scenarios pass with zero console warning/error entries; evidence committed under `e2e/results/`.

### Task 2 — Schema-menu surface (TYPEDUCK-PARITY-02 close-out)
- [ ] Per the M14 finding, validate `hide_lone_schema`/`hide_caret` against the oracle-observable surface — if it was scoped to UI in M14, assert the TypeDuck-Web schema-selector visibility state directly.
- **Acceptance:** schema-menu behavior validated (golden-backed or browser-UI-asserted), or documented as a fork-only gap.

### Task 3 — Activate parity tests in the workspace
- [ ] Confirm the M15-activated `cantonese_parity` tests are green in `cargo test --workspace`; any deferred fork-only behavior carries an explicit `#[ignore]` + blocker.
- **Acceptance:** workspace tests green; no silent gaps.

### Task 4 — userdb-pronunciation resolution (TYPEDUCK-PARITY-03 close-out)
- [ ] Resolve per the M14 spike: either land the native inspection/validation path, or **explicitly list** it as the documented uncapturable fork-only gap in the parity statement.
- **Acceptance:** the gap is closed or explicitly enumerated — not implied as covered.

### Task 5 — Docs + verification gate
- [ ] Flip roadmap M14/M15/M16 items and `TYPEDUCK-PARITY-01…07` to `Done` (or documented-deferral) as evidence lands; update the traceability table + coverage; add a `decisions.md` note if the parity outcome changes D-27's framing.
- [ ] Run the full gate (each command separately):

```powershell
cargo fmt
cargo test -p yune-core --test cantonese_parity
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix packages/yune-typeduck-runtime test
npm --prefix packages/yune-typeduck-runtime run build
git diff --check
```

- [ ] Run the browser E2E (the core M16 proof) per `e2e/yune-browser-smoke.md`, then `npx playwright test yune-typeduck.spec.ts`; commit screenshots + state JSON + `browser-run.log`.
- **Acceptance:** all gates pass; real-browser parity evidence committed.

## Completion criteria
- **Done = TypeDuck-Web is fork-like for all captured target behaviors (plus M13 AI), with any uncapturable fork-only gap explicitly listed** (e.g. userdb pronunciations if the M14 spike proved them uncapturable).
- The `cantonese_parity` ignored tests are activated (or carry documented deferrals); committed real-browser evidence covers each parity behavior with zero console warning/error entries.
- `cargo test --workspace`, clippy `-D warnings`, TS runtime test/build, and the Playwright E2E pass.

## Review checklist
- [ ] Parity claim is scoped to **captured** behaviors; uncapturable fork-only gaps are listed, not implied covered.
- [ ] Browser evidence is committed and non-circular (asserts against M14 v1.1.2 goldens).
- [ ] No regression to M9/M13 web gates or classic input.
- [ ] Docs/requirements reflect only the evidence that landed.
