---
phase: 06-real-frontend-validation-and-benchmark
plan: 01
subsystem: testing
tags: [rust, rime-abi, dynamic-loader, ffi, frontend-validation]

requires:
  - phase: 02-native-abi-validation-and-runtime-safety
    provides: dynamic-loader validation pattern and ABI lifecycle safety gates
  - phase: 05-userdb-and-scaling-hardening
    provides: runtime/session safety context for frontend-sensitive validation
provides:
  - Dynamic-loader-backed native host lifecycle validation harness
  - Deterministic frontend host trace and mismatch capture contracts
  - Sanitized native host lifecycle trace fixture for later frontend validation plans
affects: [06-real-frontend-validation-and-benchmark, TypeDuck-Web validation, Squirrel/macOS validation, Linux frontend scoping, frontend benchmarks]

tech-stack:
  added: []
  patterns:
    - Cargo integration-test helper module under crates/yune-rime-api/tests/frontend_hosts
    - Deterministic JSON fixture rendering without new dependencies
    - Host-shaped ABI validation through Cargo-built cdylib and rime_get_api

key-files:
  created:
    - crates/yune-rime-api/tests/frontend_hosts.rs
    - crates/yune-rime-api/tests/frontend_hosts/mod.rs
    - crates/yune-rime-api/tests/frontend_hosts/native.rs
    - fixtures/frontend-traces/native-host-lifecycle.json
  modified:
    - crates/yune-rime-api/tests/dynamic_loader.rs

key-decisions:
  - "Native frontend validation remains anchored at the Cargo-built yune-rime-api cdylib boundary and delegates through the resolved RimeApi table."
  - "Trace fixtures use logical resource IDs and deterministic event names rather than local paths, timestamps, PIDs, or pointer addresses."
  - "Missing required RimeApi function pointers are represented as blocker-capable mismatch records before any unchecked call."

patterns-established:
  - "FrontendHostTrace: deterministic ordered-call, notification, free-pair, stale-session, and mismatch capture format for frontend-shaped validation."
  - "Native host lifecycle scenario: reusable dynamic RimeApi-table driver covering setup, initialize, deploy/maintenance, schema selection, session lifecycle, context/status/commit/free, notifications, repeated initialize/finalize, stale sessions, and teardown."

requirements-completed:
  - FRONTEND-VALIDATION-01
  - FRONTEND-VALIDATION-05

duration: 11min
completed: 2026-05-01
---

# Phase 06 Plan 01: Native Host Trace Harness Summary

**Dynamic-loader-backed RimeApi native host lifecycle trace with deterministic mismatch capture and sanitized fixture output**

## Performance

- **Duration:** 11 min
- **Started:** 2026-05-01T11:29:18Z
- **Completed:** 2026-05-01T11:40:47Z
- **Tasks:** 3/3
- **Files modified:** 5

## Accomplishments

- Added a Cargo-visible frontend host validation test entrypoint plus shared trace/mismatch contracts for frontend-observed ABI/runtime gaps.
- Implemented a native host-shaped lifecycle scenario driven through the borrowed dynamic `RimeApi` table resolved from the Cargo-built cdylib.
- Locked a sanitized baseline trace fixture that records ordered calls, required function availability, notifications, free-pair observations, stale-session checks, and match/blocker-capable mismatch fields.

## Task Commits

Each task was committed atomically:

1. **Task 1: Define shared frontend host trace and mismatch contracts** - `be3953b` (test)
2. **Task 2: Implement native host-shaped lifecycle scenario through the dynamic RimeApi table** - `1f632a4` (test)
3. **Task 3: Wire the dynamic loader test to the host harness and lock the baseline trace fixture** - `075f505` (test)

**Plan metadata:** committed after summary creation.

_Note: TDD-tagged tasks were implemented as test/harness commits because this plan's deliverables are integration-test validation assets._

## Files Created/Modified

- `crates/yune-rime-api/tests/frontend_hosts.rs` - Cargo-visible integration-test entrypoint that validates fixture sanitization.
- `crates/yune-rime-api/tests/frontend_hosts/mod.rs` - Shared deterministic trace, mismatch, sanitizer, required-function, and ABI struct helper contracts.
- `crates/yune-rime-api/tests/frontend_hosts/native.rs` - Native host-shaped lifecycle scenario against a borrowed dynamic `RimeApi` table.
- `crates/yune-rime-api/tests/dynamic_loader.rs` - Dynamic-loader test now resolves `rime_get_api` from the Cargo-built cdylib and delegates lifecycle validation to the native host scenario.
- `fixtures/frontend-traces/native-host-lifecycle.json` - Sanitized baseline trace fixture synchronized with the dynamic-loader native host scenario.

## Decisions Made

- Kept validation at the RIME ABI boundary by preserving the `libloading` cdylib discovery and `rime_get_api` resolution path before running the host scenario.
- Used dependency-free deterministic JSON rendering to avoid adding non-MSRV-reviewed dependencies.
- Represented required-function absence as blocker-capable mismatch data before calls are made, while mandatory tests still fail fast when a required pointer is absent.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Used absolute Cargo path after shell PATH omitted cargo**
- **Found during:** Task 1 (Define shared frontend host trace and mismatch contracts)
- **Issue:** `cargo` was not available on the shell PATH, blocking verification commands.
- **Fix:** Used the installed Cargo binary at `$HOME/.cargo/bin/cargo` for all verification gates.
- **Files modified:** None
- **Verification:** `cargo 1.95.0` resolved at `$HOME/.cargo/bin/cargo`; focused and plan-level verification commands ran successfully.
- **Committed in:** Not applicable (environment-only fix)

**2. [Rule 3 - Blocking] Disambiguated integration-test helper module paths**
- **Found during:** Task 3 (Wire the dynamic loader test to the host harness and lock the baseline trace fixture)
- **Issue:** Rust reported module ambiguity because both `tests/frontend_hosts.rs` and `tests/frontend_hosts/mod.rs` existed.
- **Fix:** Added explicit `#[path = "frontend_hosts/mod.rs"] mod frontend_hosts;` declarations in integration-test entrypoints.
- **Files modified:** `crates/yune-rime-api/tests/dynamic_loader.rs`, `crates/yune-rime-api/tests/frontend_hosts.rs`
- **Verification:** `cargo test -p yune-rime-api --test dynamic_loader -- --nocapture` passed.
- **Committed in:** `075f505`

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both fixes were necessary to complete planned verification. No production code or out-of-scope frontend implementation was added.

## Issues Encountered

- `cargo fmt --check` initially required formatting updates to the new Rust test helpers; `cargo fmt` was run and `cargo fmt --check` then passed.
- The baseline fixture intentionally records `deploy_schema` as `false` while overall classification remains `match`, because the lifecycle includes deploy/start behavior and successful runtime schema selection through logical `dynamic_schema`.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None - scan of modified files found no TODO/FIXME/placeholder or hardcoded empty UI data stubs.

## Threat Flags

None - new security-relevant surface is test-only FFI validation and is covered by the plan threat model mitigations for dynamic library loading, RimeApi null function pointers, free pairing, stale sessions, logical IDs, and sanitized committed traces.

## Verification

- `cargo test -p yune-rime-api dynamic_loader --test dynamic_loader -- --nocapture` - passed during Task 1.
- `cargo test -p yune-rime-api dynamic_loader_harness_loads_cargo_cdylib_and_api_table --test dynamic_loader -- --nocapture` - passed during Task 2.
- `cargo test -p yune-rime-api --test dynamic_loader -- --nocapture` - passed during Task 3 and plan-level verification.
- `cargo fmt --check` - passed after formatting.
- Fixture sanitizer gate checked absence of `/tmp/`, `/var/`, user home path, `target/debug`, `target/release`, `0x`, `timestamp`, `duration`, and `process_id` - passed.

## Next Phase Readiness

Ready for Plan 06-02 TypeDuck-Web validation to reuse the `FrontendHostTrace`/mismatch format and native host baseline fixture for runnable reproduction, minimized call-sequence fixture, or documented blocker outputs.

## Self-Check: PASSED

- Verified created files exist: `crates/yune-rime-api/tests/frontend_hosts.rs`, `crates/yune-rime-api/tests/frontend_hosts/mod.rs`, `crates/yune-rime-api/tests/frontend_hosts/native.rs`, `fixtures/frontend-traces/native-host-lifecycle.json`, and this summary.
- Verified task commits exist in git history: `be3953b`, `1f632a4`, `075f505`.

---
*Phase: 06-real-frontend-validation-and-benchmark*
*Completed: 2026-05-01*
