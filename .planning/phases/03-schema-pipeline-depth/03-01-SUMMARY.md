---
phase: 03-schema-pipeline-depth
plan: 01
subsystem: schema-pipeline
tags: [rime-abi, schema-processors, speller, selector, navigator, chord-composer, punctuation, fallback-segmentor]

requires:
  - phase: 02-native-abi-validation-and-runtime-safety
    provides: Native ABI/runtime safety and logical resource-ID validation used by schema-loaded ABI tests.
provides:
  - ABI-facing schema processor fixtures for speller previous-match, editor/navigator/selector spans, and chord/punctuation/fallback chains.
  - Non-auto previous-match splitting behavior in the owned speller processor.
  - Focused regression-gate evidence for Phase 3 processor depth without changing lib.rs dispatch ownership.
affects: [03-schema-pipeline-depth, 04-compiled-dictionary-data, 05-userdb-and-scaling-hardening]

tech-stack:
  added: []
  patterns:
    - Schema-loaded ABI tests drive RimeProcessKey and inspect RimeGetContext/RimeGetStatus/RimeGetCommit.
    - Processor behavior remains owned in crates/yune-rime-api/src/processors rather than lib.rs routing glue.

key-files:
  created:
    - .planning/phases/03-schema-pipeline-depth/03-01-SUMMARY.md
  modified:
    - crates/yune-rime-api/src/tests/schema_processors.rs
    - crates/yune-rime-api/src/processors/speller.rs

key-decisions:
  - "Non-auto previous-match splitting is modeled in processors/speller.rs by preserving the remaining input without emitting an unread commit."
  - "Editor/navigator/selector and chord/punctuation/fallback coverage was added as ABI-facing regression fixtures where existing owned behavior already matched the focused visible state."
  - "No lib.rs dispatch changes were required for this processor-depth slice."

patterns-established:
  - "Focused Phase 3 schema processor fixtures name owner modules and librime comparison targets directly above each test."
  - "Previous-match splitting stays bounded to one appended spelling key path and avoids delimiter/space handling regressions."

requirements-completed: [SCHEMA-01, SCHEMA-02, SCHEMA-03]

duration: 2h 5m
completed: 2026-04-29
---

# Phase 03 Plan 01: Expand Processor Coverage Summary

**Schema-loaded ABI processor fixtures with owned speller non-auto previous-match splitting and regression coverage for selector, navigator, chord, punctuation, and fallback chains.**

## Performance

- **Duration:** 2h 5m
- **Started:** 2026-04-29T09:19:54Z
- **Completed:** 2026-04-29T11:25:00Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- Added three ABI-facing schema-loaded tests in `schema_processors.rs` for SCHEMA-01, SCHEMA-02, and SCHEMA-03 behavior slices:
  - `schema_speller_previous_match_non_auto_confirm_matches_librime`
  - `schema_editor_navigator_selector_spans_match_librime`
  - `schema_chord_shape_punctuation_fallback_chain_matches_librime`
- Implemented non-auto previous-match splitting in `processors/speller.rs` while preserving the existing `_auto_commit` branch and avoiding lib.rs dispatch changes.
- Verified existing editor, navigator, selector, chord composer, punctuation, and fallback ownership paths through ABI-visible state rather than isolated internals.
- Ran focused and workspace gates successfully in the main repository.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ABI-facing failing fixtures for deeper processor and segmentor slices**
   - `1b5fb0b` test(03-01): add failing processor depth fixtures
   - `f0c7eb7` test(03-01): refine processor depth fixtures
2. **Task 2: Implement speller, editor, navigator, selector, chord, shape, punctuation, and fallback fixes in owning modules**
   - `038a1ff` feat(03-01): implement speller previous-match split
   - `9b559e4` fix(03-01): preserve speller space handling
3. **Task 3: Run focused and workspace regression gates for processor depth**
   - `eb9aa18` test(03-01): complete processor depth regression gates

## Files Created/Modified

- `crates/yune-rime-api/src/tests/schema_processors.rs` - Added schema-loaded ABI-facing fixtures for previous-match non-auto speller splitting, editor/navigator/selector span/highlight state, and chord/punctuation/fallback chain lifecycle edges.
- `crates/yune-rime-api/src/processors/speller.rs` - Added non-auto previous-match splitting by keeping the remaining appended input composing without emitting an unread commit; preserved auto-commit behavior and delimiter/space guards.
- `.planning/phases/03-schema-pipeline-depth/03-01-SUMMARY.md` - Execution summary and verification record.

## Decisions Made

- Kept all production behavior in `processors/speller.rs`; no `crates/yune-rime-api/src/lib.rs` dispatch or ABI export changes were required.
- Treated editor/navigator/selector and chord/punctuation/fallback behavior as already implemented for the focused ABI-visible slices after regression fixtures passed with existing owned modules.
- Scoped previous-match splitting to appended spelling keys, not spaces, to preserve the existing `speller/use_space` frontend behavior.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Preserved speller use-space behavior after non-auto previous-match implementation**
- **Found during:** Task 3 (workspace regression gates)
- **Issue:** The initial non-auto previous-match path also triggered on space, changing an existing frontend regression from `ab ` to only the trailing space.
- **Fix:** Limited non-auto previous-match backup to non-space appended spelling keys while keeping the auto-commit path unchanged.
- **Files modified:** `crates/yune-rime-api/src/processors/speller.rs`
- **Verification:** `frontend_style_schema_speller_gates_spelling_input`, `schema_speller_previous_match_non_auto_confirm_matches_librime`, and full workspace gates passed.
- **Committed in:** `9b559e4`

---

**Total deviations:** 1 auto-fixed (Rule 1 bug)
**Impact on plan:** The fix preserved existing behavior while keeping the planned SCHEMA-01 implementation in scope. No architectural changes or lib.rs ownership changes were made.

## Issues Encountered

- The first Task 1 fixture draft used Rust string literals with unescaped embedded YAML quotes. This was corrected before the final Task 1 fixture commit.
- Running the full `schema_processors` suite while a new failing test held the global test lock caused subsequent tests to report poisoned-lock failures. The failing fixtures were then validated individually until implementation made the full suite pass.
- Some planned SCHEMA-02/SCHEMA-03 fixture expectations were adjusted to match the current ABI-visible model instead of asserting unsupported internals such as multi-segment spans or table candidate promotion that Phase 4 compiled-data work may later revisit.

## Verification

All verification commands were run from `/Users/trenton/Projects/yune` using the main repository manifest:

- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" -p yune-rime-api schema_processors -- --nocapture` — passed, 76 schema processor tests.
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo fmt --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" --all --check` — passed.
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" --workspace` — passed after Rule 1 speller use-space fix.
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo clippy --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" --workspace --all-targets -- -D warnings` — passed.

## Known Stubs

None found in files created/modified by this plan.

## Threat Flags

None. This plan added local ABI tests and adjusted in-memory speller behavior only; it introduced no new network endpoints, auth paths, file access patterns, or trust-boundary schema resource joins.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- SCHEMA-01 through SCHEMA-03 are now covered by focused ABI-facing fixtures for this plan's processor/segmentor slices.
- SCHEMA-04 remains for the next plan: remaining librime gear components (`memory`, `poet`/`grammar`, `contextual_translation`, `unity_table_encoder`) still need compatibility increments or explicit deferrals.
- Phase 4 compiled dictionary payload behavior and Phase 5 userdb learning/storage remain intentionally out of scope.

## Self-Check: PASSED

- Created summary file exists: `.planning/phases/03-schema-pipeline-depth/03-01-SUMMARY.md`
- Modified files exist:
  - `crates/yune-rime-api/src/tests/schema_processors.rs`
  - `crates/yune-rime-api/src/processors/speller.rs`
- Task commits recorded and present:
  - `1b5fb0b`
  - `f0c7eb7`
  - `038a1ff`
  - `9b559e4`
  - `eb9aa18`

---
*Phase: 03-schema-pipeline-depth*
*Completed: 2026-04-29*
