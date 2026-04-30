---
phase: 05-userdb-and-scaling-hardening
plan: 02
subsystem: runtime-userdb
tags: [rust, rime-api, userdb, learning, predictive-lookup, candidate-ranking]

requires:
  - phase: 05-userdb-and-scaling-hardening
    provides: Plan 05-01 file-backed userdb storage, typed record/value format, recovery/sync/import/export helpers
provides:
  - Storage-agnostic core userdb learning contracts and predictive lookup
  - Commit metadata capture before composition clear in core engine flow
  - ABI/session userdb persistence through schema-selected dictionary stores
  - Frontend-style rime_get_api proof that learned entries survive session recreation
affects: [runtime, candidate-ranking, rime-api, userdb, schema-install]

tech-stack:
  added: []
  patterns:
    - Core owns typed userdb learning and quality contracts without filesystem paths or ABI pointers
    - yune-rime-api owns store selection, logical dictionary validation, and persistence adapters
    - Session append-commit seam consumes pending core learning events after normal commit paths

key-files:
  created:
    - crates/yune-core/src/userdb.rs
  modified:
    - crates/yune-core/src/engine.rs
    - crates/yune-core/src/state.rs
    - crates/yune-core/src/lib.rs
    - crates/yune-rime-api/src/session.rs
    - crates/yune-rime-api/src/lib.rs
    - crates/yune-rime-api/src/schema_selection.rs
    - crates/yune-rime-api/src/schema_install.rs
    - crates/yune-rime-api/src/userdb.rs
    - crates/yune-rime-api/src/userdb/mod.rs
    - crates/yune-rime-api/src/tests/userdb.rs
    - crates/yune-rime-api/tests/frontend_client.rs

key-decisions:
  - "Classic learning remains commit-driven: Engine emits typed pending userdb events before clearing composition, and session persistence consumes them only through normal commit paths."
  - "Core userdb remains storage-neutral; yune-rime-api bridges core events to the Plan 05-01 store and validates logical dictionary names before storage selection."
  - "Schema-selected userdb loading is keyed by the table/script translator dictionary, not the schema id fallback, so unrelated schema tests do not inherit persisted session learning."
  - "HistoryTranslator, CandidateRanker, MockAiRanker, and AI memory are not substitutes for classic userdb learning; userdb candidates enter before optional rankers."

patterns-established:
  - "Pending learning event seam: Engine::take_pending_userdb_learning transfers commit metadata to the runtime layer after append_unread_commit."
  - "Runtime userdb adapter: userdb facade delegates load_runtime_userdb and record_runtime_commit to the manager/store modules."
  - "Explicit backdated scope: BackdatedScanPolicy documents current runtime commit/composition scanning and excludes history translator or AI ranker memory."

requirements-completed: [USERDB-03, QUAL-04]

duration: 2h 20m
completed: 2026-04-30
---

# Phase 05 Plan 02: Classic Userdb Learning and Predictive Runtime Flow Summary

**Commit-driven classic userdb learning with persisted frequency updates, predictive lookup, and ABI/session candidate influence through schema-selected dictionary stores**

## Performance

- **Duration:** 2h 20m
- **Started:** 2026-04-30T01:48:00Z
- **Completed:** 2026-04-30T04:08:29Z
- **Tasks:** 3/3
- **Files modified:** 12

## Accomplishments

- Added `crates/yune-core/src/userdb.rs` with storage-agnostic commit metadata, learned entries, frequency/dee values, lookup requests/results, librime-style dynamic formulas, and explicit backdated scan policy.
- Extended core engine commit flow so candidate commits record input, selected text, candidate source/type, segment span, and tick metadata before `clear_composition()`.
- Added runtime persistence wiring so normal ABI/session commit paths persist pending learning events into the Plan 05-01 userdb store and reload learned entries for later candidate lookup.
- Added deterministic exact and predictive userdb candidates to core candidate refresh before filters and optional rankers.
- Added focused core, API/session, and frontend-style tests proving learning, persistence, session recreation, predictive lookup, and non-substitution by history/AI/rankers.

## Task Commits

1. **Task 1: Add core classic learning and predictive lookup contracts**
   - `c1e598f` test: add failing core userdb learning tests
   - `76229a1` feat: add core userdb learning contracts
2. **Task 2: Wire session persistence and candidate ordering through the ABI runtime flow**
   - `175bb78` test: add failing ABI userdb learning tests
   - `c58c241` feat: wire ABI userdb learning persistence
3. **Task 3: Close learning and predictive verification gates**
   - `086023a` test: close userdb verification gates

_Note: TDD tasks intentionally have separate RED and GREEN commits._

## Files Created/Modified

- `crates/yune-core/src/userdb.rs` - Core userdb contracts, learning update logic, predictive lookup, quality formulas, and backdated scan policy/tests.
- `crates/yune-core/src/engine.rs` - Captures userdb learning metadata before composition clear and injects userdb lookup candidates before filters/rankers.
- `crates/yune-core/src/state.rs` - Adds `CandidateSource::UserTable` and richer commit history metadata.
- `crates/yune-core/src/lib.rs` - Thin facade export for core userdb types.
- `crates/yune-rime-api/src/session.rs` - Session-owned active user dictionary state plus load/persist helpers for pending learning events.
- `crates/yune-rime-api/src/lib.rs` - Narrow append-commit seam invokes pending userdb persistence after normal commits.
- `crates/yune-rime-api/src/schema_selection.rs` - Clears/reloads session userdb state around schema application.
- `crates/yune-rime-api/src/schema_install.rs` - Selects active user dictionary from validated table/script translator dictionary IDs.
- `crates/yune-rime-api/src/userdb.rs` - Thin facade wrappers for runtime userdb load/update helpers.
- `crates/yune-rime-api/src/userdb/mod.rs` - Store adapter for loading core `UserDb` entries and recording commit updates into file-backed userdb records.
- `crates/yune-rime-api/src/tests/userdb.rs` - Focused session persistence/reload/predictive tests plus existing userdb hardening coverage.
- `crates/yune-rime-api/tests/frontend_client.rs` - Frontend-style `rime_get_api` flow proving commit, session recreation, and learned candidate observation.

## Implementation and Test Owners

- **Core implementation owners:** `crates/yune-core/src/userdb.rs`, `crates/yune-core/src/engine.rs`, `crates/yune-core/src/state.rs`.
- **Runtime/API implementation owners:** `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/userdb.rs`, `crates/yune-rime-api/src/userdb/mod.rs`, `crates/yune-rime-api/src/schema_selection.rs`, `crates/yune-rime-api/src/schema_install.rs`.
- **Facade owners:** `crates/yune-core/src/lib.rs`, `crates/yune-rime-api/src/lib.rs`; both remain module/export or orchestration boundaries rather than implementation-heavy modules.
- **Test owners:** core userdb tests in `crates/yune-core/src/userdb.rs`, API/session tests in `crates/yune-rime-api/src/tests/userdb.rs`, frontend-style ABI tests in `crates/yune-rime-api/tests/frontend_client.rs`.
- **Librime comparison targets:** `/Users/trenton/Projects/librime/src/rime/gear/memory.cc`, `/Users/trenton/Projects/librime/src/rime/gear/table_translator.cc`, `/Users/trenton/Projects/librime/src/rime/dict/user_dictionary.cc`, `/Users/trenton/Projects/librime/src/rime/algo/dynamics.h`.

## Decisions Made

- Core records learning metadata before composition clearing and exposes a pending event instead of coupling core to storage.
- Runtime persistence validates/selects logical dictionary IDs via existing resource validation and Plan 05-01 userdb store APIs.
- Userdb candidate injection is done from engine refresh using core `UserDb` state, preserving the normal translator/filter/ranker ordering contract.
- Session userdb loading is tied to the schema dictionary configured for table/script translators; this avoids accidental learning bleed from the default schema id into tests or schemas without dictionaries.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added runtime userdb load/update facade wrappers**
- **Found during:** Task 2 (ABI/session persistence)
- **Issue:** The Plan 05-01 manager was private behind `crates/yune-rime-api/src/userdb.rs`, so session code could not safely reach storage helpers without breaking the facade boundary.
- **Fix:** Added thin facade wrappers and manager helpers for loading `UserDb` from records and recording pending commit updates.
- **Files modified:** `crates/yune-rime-api/src/userdb.rs`, `crates/yune-rime-api/src/userdb/mod.rs`
- **Verification:** `cargo test -p yune-rime-api userdb -- --test-threads=1`
- **Committed in:** `c58c241`

**2. [Rule 1 - Bug] Scoped session userdb loading to configured dictionary names**
- **Found during:** Task 3 (workspace verification)
- **Issue:** Loading userdb by schema id could leak learned entries into sessions whose translator dictionary differed from schema id, altering unrelated candidate selection tests.
- **Fix:** Reset session userdb during schema application and set the active user dictionary only after validated table/script translator dictionary selection.
- **Files modified:** `crates/yune-rime-api/src/session.rs`, `crates/yune-rime-api/src/schema_selection.rs`, `crates/yune-rime-api/src/schema_install.rs`
- **Verification:** Full verification sequence including workspace tests and clippy passed.
- **Committed in:** `086023a`

---

**Total deviations:** 2 auto-fixed (1 missing critical, 1 bug)
**Impact on plan:** Both fixes were required to preserve facade boundaries, trust-boundary validation, and existing deterministic candidate behavior.

## Issues Encountered

- Initial core test module creation did not run tests until `mod userdb;` was added to the core facade. This was caught during RED and included in the failing-test commit sequence.
- Userdb quality needed explicit boosting to outrank high-weight table candidates when learned exact duplicates are present. This preserves the plan requirement that learned/frequency candidates influence deterministic order before optional rankers.
- Workspace tests exposed a session-scoping issue for dictionary-backed schemas; resolved by selecting userdb stores from validated translator dictionary IDs.

## Verification

Passed:

- `$HOME/.cargo/bin/cargo fmt --check`
- `$HOME/.cargo/bin/cargo test -p yune-core userdb`
- `$HOME/.cargo/bin/cargo test -p yune-rime-api userdb -- --test-threads=1`
- `$HOME/.cargo/bin/cargo test -p yune-rime-api --test frontend_client userdb -- --test-threads=1`
- `$HOME/.cargo/bin/cargo test --workspace -- --test-threads=1`
- `$HOME/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings`

## Known Stubs

None found in plan-created or plan-modified runtime/userdb paths. Existing dictionary parser tests in `crates/yune-core/src/lib.rs` mention placeholder columns as test data only and are unrelated to this plan's runtime userdb behavior.

## Threat Flags

None beyond the plan threat model. The trust-boundary changes are the planned runtime/session commit-to-userdb store path, core-to-API storage handoff, and learned candidate output path; mitigations are covered by typed metadata, logical dictionary validation, and focused ABI tests.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- USERDB-03 and QUAL-04 coverage is in place for commit-driven learning, persistence, predictive lookup, and ABI replay.
- Future work can refine quality calibration against more librime fixtures, but the deterministic learning/persistence seam is now established.

---
*Phase: 05-userdb-and-scaling-hardening*
*Completed: 2026-04-30*
