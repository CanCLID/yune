# Phase 7: WASM Build And Export Contract - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-02
**Phase:** 07-WASM Build And Export Contract
**Areas discussed:** WASM target contract, export retention verification, local toolchain fallback, browser constraint documentation

---

## WASM Target Contract

| Option | Description | Selected |
|--------|-------------|----------|
| Emscripten target | Use `wasm32-unknown-emscripten` as the browser build target because the adapter exposes C ABI symbols and expects Emscripten filesystem/runtime support. | ✓ |
| wasm-bindgen first | Reframe the adapter as a wasm-bindgen package before TypeDuck-Web integration. | |
| Native only | Keep only native cdylib validation and defer browser build details. | |

**User's choice:** Auto-selected recommended option.
**Notes:** The existing milestone is TypeDuck-Web-style browser integration, so Phase 7 needs an Emscripten/WASM build contract rather than a JS package or native-only continuation.

---

## Export Retention Verification

| Option | Description | Selected |
|--------|-------------|----------|
| Adapter-specific symbol list | Verify the required `yune_typeduck_*` symbols and keep the librime-shaped `RimeApi` table unchanged. | ✓ |
| Full ABI export audit | Re-audit every `Rime*` symbol and function-table member in this phase. | |
| TypeScript-driven check | Wait for the Phase 8 wrapper to prove symbol availability. | |

**User's choice:** Auto-selected recommended option.
**Notes:** Phase 7 owns symbol availability for JS callers; Phase 8 owns typed wrapper ergonomics.

---

## Local Toolchain Fallback

| Option | Description | Selected |
|--------|-------------|----------|
| Reproducible blocker plus native fallback | Attempt/document the Emscripten build, report missing tooling deterministically, and keep native adapter tests authoritative when unavailable. | ✓ |
| Require Emscripten locally | Block all Phase 7 completion unless local Emscripten builds succeed. | |
| Skip browser target until Phase 10 | Defer all Emscripten build investigation until upstream TypeDuck-Web is cloned. | |

**User's choice:** Auto-selected recommended option.
**Notes:** The requirements explicitly allow a documented local-toolchain blocker while preserving native fallback tests.

---

## Browser Constraint Documentation

| Option | Description | Selected |
|--------|-------------|----------|
| Extend adapter docs now | Document export flags, process-global lifecycle, MEMFS/IDBFS assumptions, and Phase 9/10 boundaries in the existing adapter docs. | ✓ |
| Create full browser runtime package docs | Write package-level TypeScript/browser runtime docs in Phase 7. | |
| Leave docs until E2E | Wait until TypeDuck-Web is cloned to document browser assumptions. | |

**User's choice:** Auto-selected recommended option.
**Notes:** Documentation should make host assumptions explicit before TypeScript/package work, but package-level and E2E details remain later phases.

---

## Claude's Discretion

- Exact script name, artifact inspection command, and docs layout are left to planning and execution.
- The planner should prefer small, deterministic build/export checks over npm package, bundler, browser app, or upstream TypeDuck-Web work in Phase 7.

## Deferred Ideas

- TypeScript bridge and key mapping remain Phase 8.
- Browser filesystem persistence and IDBFS orchestration remain Phase 9.
- Upstream TypeDuck-Web clone/replace/browser E2E remains Phase 10.
- AI-native frontend exposure remains deferred until this TypeDuck-Web milestone closes.
