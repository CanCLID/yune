# Phase 8: TypeScript Bridge And Runtime Package - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-04
**Phase:** 08-typescript-bridge-and-runtime-package
**Areas discussed:** Wrapper location and tooling, TypeScript API shape, Response ownership and JSON parsing, Browser key mapping, Runtime lifecycle contract

---

## Wrapper Location And Tooling

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal repository-owned TypeScript bridge package | Add a small tested TS wrapper package without a browser app scaffold. | ✓ |
| Documentation-only snippets | Keep wrapper examples in docs and defer package files. | |
| Full TypeDuck-Web/bundler scaffold | Introduce browser app/package tooling for direct app integration now. | |

**User's choice:** [auto] Selected minimal repository-owned TypeScript bridge package (recommended default).
**Notes:** The repo has no existing JS/TS tooling, so the selected option permits only the smallest deterministic TypeScript setup needed for Phase 8 while keeping TypeDuck-Web app work in Phase 10.

---

## TypeScript API Shape

| Option | Description | Selected |
|--------|-------------|----------|
| Inject a narrow Emscripten Module interface | Wrap `cwrap`/`UTF8ToString` through a typed Module dependency and fake it in tests. | ✓ |
| Bind directly to a generated browser artifact | Require real Emscripten output during wrapper tests. | |
| Expose raw `RimeApi`/librime-style calls | Build a broader frontend API around the function table. | |

**User's choice:** [auto] Selected injected narrow Emscripten Module interface (recommended default).
**Notes:** Phase 7 already verifies exported symbols. Phase 8 should test wrapper ergonomics and ownership without requiring local browser tooling.

---

## Response Ownership And JSON Parsing

| Option | Description | Selected |
|--------|-------------|----------|
| Centralized response handling path | Every operation copies/parses JSON and frees owned responses in one helper. | ✓ |
| Per-operation response handling | Each wrapper operation manually handles JSON/freeing. | |
| Caller-managed response pointers | Return raw response pointers and require callers to free. | |

**User's choice:** [auto] Selected centralized response handling path (recommended default).
**Notes:** This directly satisfies TYPEDUCK-JS-02 and mirrors the native fallback pattern in `typeduck_web.rs` tests.

---

## Browser Key Mapping

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit event-like key mapping with tests | Map narrow KeyboardEvent-like inputs to keycode/mask pairs deterministically. | ✓ |
| Pass browser key events through unmodified | Let app callers decide keycode/mask mapping ad hoc. | |
| Exhaustive cross-browser keyboard model | Attempt to cover all platform/browser edge cases in Phase 8. | |

**User's choice:** [auto] Selected explicit event-like key mapping with tests (recommended default).
**Notes:** Start with printable keys, common editing/navigation keys, page keys, selection digits, and modifiers needed for TypeDuck-Web integration. Real app/browser edge cases can be expanded in Phase 10.

---

## Runtime Lifecycle Contract

| Option | Description | Selected |
|--------|-------------|----------|
| Visible one-active-service wrapper contract | Make global lifecycle limits explicit and guard common misuse in the wrapper. | ✓ |
| Documentation-only lifecycle warning | Mention process-global behavior but do not enforce wrapper state. | |
| Multi-instance isolation | Try to support multiple simultaneous service states now. | |

**User's choice:** [auto] Selected visible one-active-service wrapper contract (recommended default).
**Notes:** Phase 7 locked the process-global constraint. Phase 8 should expose that clearly and fail deterministic operations after cleanup, while leaving true multi-instance isolation out of scope.

---

## Claude's Discretion

- Choose exact package path, TypeScript file names, and test runner during planning.
- Prefer fake-module tests over Emscripten/browser dependencies.
- Keep wrapper types and runtime guards small; avoid generated bindings and browser app scaffolding.

## Deferred Ideas

- Phase 9 owns MEMFS/IDBFS setup, asset preload, persistence sync, and recovery paths.
- Phase 10 owns upstream TypeDuck-Web checkout, librime bridge replacement, and real browser E2E.
- AI-native provider/ranking/context/memory/privacy behavior remains deferred until the TypeDuck-Web milestone produces a frontend exposure recommendation.
