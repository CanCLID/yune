---
phase: 08-typescript-bridge-and-runtime-package
plan: 02
subsystem: testing
tags: [typescript, typeduck, vitest, emscripten, runtime-tests, keyboard-mapping]

requires:
  - phase: 08-typescript-bridge-and-runtime-package
    provides: Package-local TypeScript TypeDuck runtime wrapper from plan 08-01
provides:
  - Deterministic fake Emscripten Module harness for TypeDuck runtime tests
  - Vitest coverage for binding, response ownership, runtime lifecycle, operation forwarding, and key mapping
  - Package-local verification that tests run without DOM, browser APIs, Emscripten, WASM, or TypeDuck-Web
  - Response error-path coverage proving nonzero response pointers are freed exactly once
  - Cleanup guard coverage proving raw state pointers are not cleaned up twice
affects: [08-typescript-bridge-and-runtime-package, 09-browser-filesystem-persistence, 10-typeduck-web-integration]

tech-stack:
  added: []
  patterns:
    - Fake injected Emscripten Module with cwrap and UTF8ToString only
    - Package-local Vitest tests excluded from TypeScript build output
    - Response ownership tests that assert free-on-success and free-on-error
    - DOM-free key mapping tests using plain KeyboardEvent-like objects

key-files:
  created:
    - packages/yune-typeduck-runtime/test/fake-module.ts
    - packages/yune-typeduck-runtime/test/typeduck.test.ts
    - packages/yune-typeduck-runtime/test/response.test.ts
    - packages/yune-typeduck-runtime/test/keys.test.ts
  modified:
    - packages/yune-typeduck-runtime/src/module.ts
    - packages/yune-typeduck-runtime/src/response.ts

key-decisions:
  - "Kept all test infrastructure inside packages/yune-typeduck-runtime and used the existing package-local Vitest script."
  - "Made yune_typeduck_response_handled authoritative so native handled state can override the JSON envelope."
  - "Normalized malformed JSON failures into TypeDuckResponseError for deterministic wrapper errors."

patterns-established:
  - "FakeTypeDuckModule allocates deterministic numeric string/response pointers and records yune_typeduck_* calls for ownership assertions."
  - "Runtime tests inject fake response pointers into wrapper operations and assert every response-producing path frees once."
  - "Key mapping tests use TypeDuckKeyboardEventLike plain objects and ban keyCode references across source and tests."

requirements-completed:
  - TYPEDUCK-JS-01
  - TYPEDUCK-JS-02
  - TYPEDUCK-JS-03
  - TYPEDUCK-JS-04

duration: 7min
completed: 2026-05-04
---

# Phase 08 Plan 02: Deterministic TypeDuck Runtime Tests Summary

**Vitest fake-Module coverage now locks TypeDuck export binding, response pointer ownership, lifecycle cleanup guards, operation forwarding, and DOM-free browser key mapping.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-05-04T17:09:25Z
- **Completed:** 2026-05-04T17:16:31Z
- **Tasks:** 7 completed
- **Files modified:** 6

## Accomplishments

- Added `FakeTypeDuckModule`, a deterministic Emscripten Module substitute exposing only `cwrap` and `UTF8ToString`, fake string/response pointer allocation, exact export registration, and call recording.
- Added binding/runtime tests covering all 11 canonical `yune_typeduck_*` symbols, missing-export errors, init failure, operation forwarding, candidate actions, deploy/customize boolean mapping, and idempotent cleanup.
- Added response tests proving valid parse behavior, native handled authority, null pointer errors, malformed/non-object/invalid-envelope errors, and free-on-error ownership.
- Added key mapping tests for printable keys, selection digits, space spellings, editing/navigation keys, modifiers, release events, and unsupported key errors.
- Verified the package-local TypeScript build still excludes tests while `npm --prefix packages/yune-typeduck-runtime test` runs all Vitest coverage in Node without DOM or WASM dependencies.

## Task Commits

Each task was committed atomically where practical:

1. **Task 1: Add fake Emscripten Module test harness** - `05fac4e` (test)
2. **Task 2: Test module binding behavior** - `6197104` (test)
3. **Task 3: Test centralized response parsing and ownership** - `d1169cf` (test)
4. **Task 4 and 5: Test runtime init, operation forwarding, and lifecycle guards** - `6197104` (test)
5. **Task 6: Test deterministic key mapping** - `7c8bcd9` (test)
6. **Task 7: Keep tests package-local and verify package scripts** - `1acd358` (test)

**Plan metadata:** pending final docs commit

## Files Created/Modified

- `packages/yune-typeduck-runtime/test/fake-module.ts` - Fake Emscripten Module harness with export registration, pointer allocation, response freeing records, and per-symbol call capture.
- `packages/yune-typeduck-runtime/test/typeduck.test.ts` - Binding, runtime operation forwarding, candidate action, deploy/customize, and lifecycle guard tests.
- `packages/yune-typeduck-runtime/test/response.test.ts` - Central response parsing and response-free ownership tests for success and deterministic error paths.
- `packages/yune-typeduck-runtime/test/keys.test.ts` - DOM-free key mapping tests for `event.key` semantics, RIME constants, modifiers, and unsupported key errors.
- `packages/yune-typeduck-runtime/src/module.ts` - Converts thrown `cwrap` missing-export failures into `TypeDuckBindingError` with the required deterministic message.
- `packages/yune-typeduck-runtime/src/response.ts` - Makes native handled accessor authoritative and normalizes malformed JSON into `TypeDuckResponseError`.

## Verification

Executed successfully from the repository root:

```bash
npm --prefix packages/yune-typeduck-runtime run build
npm --prefix packages/yune-typeduck-runtime test
grep -q 'FakeTypeDuckModule' packages/yune-typeduck-runtime/test/fake-module.ts
grep -q 'TypeDuck adapter returned null response' packages/yune-typeduck-runtime/test/response.test.ts
grep -q 'TypeDuck runtime has been cleaned up' packages/yune-typeduck-runtime/test/typeduck.test.ts
grep -q 'Backspace' packages/yune-typeduck-runtime/test/keys.test.ts
! grep -R -q 'keyCode' packages/yune-typeduck-runtime/src packages/yune-typeduck-runtime/test
```

Results:

- `npm --prefix packages/yune-typeduck-runtime run build` passed.
- `npm --prefix packages/yune-typeduck-runtime test` passed: 3 test files, 33 tests.
- All grep verification commands passed.
- `git status --short` was clean after verification.

Task-level spot checks also passed:

- `npm --prefix packages/yune-typeduck-runtime test -- typeduck.test.ts` passed: 15 tests.
- `npm --prefix packages/yune-typeduck-runtime test -- response.test.ts` passed: 9 tests.
- `npm --prefix packages/yune-typeduck-runtime test -- keys.test.ts` passed: 9 tests.

## Decisions Made

- Kept TypeScript tests package-local under `packages/yune-typeduck-runtime/test/` and did not create root npm workspace or browser runner infrastructure.
- Used fake injected Module tests rather than Emscripten/WASM/browser tests, preserving Phase 8 boundaries and leaving real browser integration to later phases.
- Changed response parsing so `yune_typeduck_response_handled` is authoritative, matching the plan's explicit requirement when JSON and native handled disagree.
- Changed malformed JSON errors to deterministic `TypeDuckResponseError` rather than surfacing host `SyntaxError` details.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Converted missing cwrap failures into TypeDuckBindingError**
- **Found during:** Task 2 (Test module binding behavior)
- **Issue:** The 08-01 implementation let `module.cwrap` exceptions escape directly, so missing fake exports did not surface as `TypeDuckBindingError("Missing TypeDuck export: <symbol>")`.
- **Fix:** Wrapped `module.cwrap` in `bindExport` and normalized missing export failures to the planned deterministic error type/message.
- **Files modified:** `packages/yune-typeduck-runtime/src/module.ts`
- **Verification:** `npm --prefix packages/yune-typeduck-runtime test -- typeduck.test.ts`
- **Committed in:** `6197104`

**2. [Rule 1 - Bug] Made native response handled state authoritative**
- **Found during:** Task 3 (Test centralized response parsing and ownership)
- **Issue:** The 08-01 parser threw when JSON `handled` disagreed with `yune_typeduck_response_handled`, but 08-02 requires the native accessor to be authoritative.
- **Fix:** Parse the JSON envelope, then overwrite `response.handled` with `responseHandled(responsePtr) !== 0` before returning.
- **Files modified:** `packages/yune-typeduck-runtime/src/response.ts`
- **Verification:** `npm --prefix packages/yune-typeduck-runtime test -- response.test.ts`
- **Committed in:** `d1169cf`

**3. [Rule 1 - Bug] Normalized malformed JSON errors to deterministic wrapper errors**
- **Found during:** Task 3 (Test centralized response parsing and ownership)
- **Issue:** Raw `JSON.parse` failures exposed environment-specific `SyntaxError` text instead of deterministic `TypeDuckResponseError` behavior.
- **Fix:** Added a JSON parse wrapper that throws `TypeDuckResponseError("TypeDuck adapter returned malformed response JSON")` while preserving finally-based response freeing.
- **Files modified:** `packages/yune-typeduck-runtime/src/response.ts`, `packages/yune-typeduck-runtime/test/response.test.ts`
- **Verification:** `npm --prefix packages/yune-typeduck-runtime test -- response.test.ts`
- **Committed in:** `d1169cf`, tightened by `1acd358`

**4. [Rule 3 - Blocking] Installed package-local dependencies for Vitest execution**
- **Found during:** Task 2 verification
- **Issue:** `npm --prefix packages/yune-typeduck-runtime test -- typeduck.test.ts` initially failed with `sh: vitest: command not found` because package-local dependencies were not installed in the fresh worktree.
- **Fix:** Ran `npm --prefix packages/yune-typeduck-runtime install`; generated `node_modules/` remains ignored and uncommitted.
- **Files modified:** None committed.
- **Verification:** Re-ran package-local Vitest commands successfully.
- **Committed in:** Not committed; local install output only.

---

**Total deviations:** 4 auto-fixed (3 bugs, 1 blocking)
**Impact on plan:** All fixes were required for the planned deterministic tests and runtime ownership semantics. No browser, Emscripten, root workspace, or TypeDuck-Web scope was added.

## Known Stubs

None. Stub-pattern matches were intentional test helper empty call arrays/override defaults and source null checks; no placeholder UI data, TODO/FIXME, or unwired mock paths were introduced.

## Threat Flags

None. This plan added tests and deterministic error handling for the existing TypeScript runtime package; it did not introduce new network endpoints, auth paths, file access patterns, or schema changes at trust boundaries.

## Issues Encountered

- Package-local dependencies were absent in the fresh worktree, so the first Vitest invocation could not find `vitest`. A package-local `npm install` resolved the blocking verification issue without root tooling.
- The malformed JSON test initially invoked `readTypeDuckResponse` twice while asserting error type and message, which doubled the fake free record. The test was tightened to capture one thrown error and assert one free.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The TypeDuck runtime package now has deterministic Node/Vitest coverage for pointer ownership, lifecycle misuse, operation forwarding, and key mapping.
- Ready for 08-03 documentation to describe package usage, host filesystem/lifecycle responsibilities, and Phase 9/10 integration boundaries.

## Self-Check: PASSED

- Found `packages/yune-typeduck-runtime/test/fake-module.ts`.
- Found `packages/yune-typeduck-runtime/test/typeduck.test.ts`.
- Found `packages/yune-typeduck-runtime/test/response.test.ts`.
- Found `packages/yune-typeduck-runtime/test/keys.test.ts`.
- Found implementation/test commits `05fac4e`, `6197104`, `d1169cf`, `7c8bcd9`, and `1acd358` in git history.
- Confirmed no shared tracking files were modified.

---
*Phase: 08-typescript-bridge-and-runtime-package*
*Completed: 2026-05-04*
