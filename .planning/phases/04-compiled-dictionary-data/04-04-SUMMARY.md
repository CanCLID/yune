---
phase: 04-compiled-dictionary-data
plan: 04
subsystem: dictionary-data
tags: [rust, rime, compiled-dictionary, prism, table, correction, tolerance, abi-tests]

requires:
  - phase: 04-compiled-dictionary-data
    provides: bounded compiled table/prism/reverse readers and advanced dictionary contracts from plans 04-01 and 04-03
provides:
  - Correction and tolerance data models on TableDictionary and compiled prism/table payload paths
  - Schema-loaded lookup integration for correction canonicalization and tolerance expansion
  - Source/compiled correction and tolerance parity tests with malformed-section fallback coverage
affects: [05-userdb-and-learning, schema-install, dictionary-parser, translator-lookup]

tech-stack:
  added: []
  patterns:
    - bounded local fixture payloads for correction/tolerance sections
    - exact lookup before correction before tolerance expansion
    - structured fallback diagnostics for malformed or unsupported compiled correction/tolerance data

key-files:
  created:
    - .planning/phases/04-compiled-dictionary-data/04-04-SUMMARY.md
  modified:
    - crates/yune-core/src/dictionary/compiled_prism.rs
    - crates/yune-core/src/dictionary/compiled_table.rs
    - crates/yune-core/src/dictionary/source.rs
    - crates/yune-core/src/dictionary/mod.rs
    - crates/yune-core/src/lib.rs
    - crates/yune-core/src/translator/mod.rs
    - crates/yune-rime-api/src/schema_install.rs
    - crates/yune-rime-api/src/tests/dictionary_data.rs

key-decisions:
  - "Correction/tolerance data is represented as normalized dictionary metadata and merged through TableDictionary rather than creating translator-only side channels."
  - "Lookup expansion keeps the original input first, then correction canonical codes, then tolerance candidate codes so exact and completion behavior remain stable."
  - "Malformed correction/tolerance counts are capped before allocation and surface as structured invalid-count fallback diagnostics."
  - "Tests use local Rust-generated fixture bytes and ABI session calls only; no librime compiler or external process is invoked."

patterns-established:
  - "Compiled prism optional sections use YUNE-CORR and YUNE-TOL local fixture markers with checked offsets, lengths, UTF-8, and count caps."
  - "Schema installation merges prism correction/tolerance metadata into table dictionaries before constructing StaticTableTranslator."
  - "Dictionary-data compatibility tests assert source and compiled candidate text/comment ordering through RimeProcessKey and RimeGetContext."

requirements-completed: [DATA-04]

duration: 16min
completed: 2026-04-29
---

# Phase 04 Plan 04: Correction and Tolerance Lookup Data Summary

**Compiled prism/table correction and tolerance metadata now drives schema-loaded lookup parity through bounded parsers and ABI-level source/compiled tests.**

## Performance

- **Duration:** 16 min in this resumed executor session; Task 1 and Task 2 were completed before context compaction in the same plan execution.
- **Started:** 2026-04-29T18:30:10Z
- **Completed:** 2026-04-29T18:46:00Z
- **Tasks:** 3/3
- **Files modified:** 8 source/test files plus this summary

## Accomplishments

- Added normalized `RimeCorrectionEntry` and `RimeToleranceRule` metadata to the dictionary contract and exposed it through `TableDictionary`/`TableDictionaryAdvancedData`.
- Parsed correction/tolerance payloads from compiled prism/table fixture sections with structured unsupported-section errors and fail-closed malformed handling.
- Wired schema installation so parsed prism correction/tolerance metadata is merged into installed table translators.
- Extended `StaticTableTranslator` lookup so exact/completion matching is preserved and correction/tolerance expansions are deterministic.
- Added ABI-level dictionary-data tests proving source and compiled correction/tolerance parity plus malformed correction/tolerance fallback diagnostics.

## Task Commits

Each task was committed atomically:

1. **Task 1: Add correction and tolerance data structures to compiled dictionary payloads** - `fbef489` (feat)
2. **Task 2: Integrate correction and tolerance inputs into schema-loaded lookup** - `c3151dd` (feat)
3. **Formatting from Tasks 1-2** - `fab3c1d` (style)
4. **Task 3: Validate correction and tolerance behavior against schema-loaded fixtures** - `62458c2` (test)

**Plan metadata:** committed separately after this summary.

## Files Created/Modified

- `crates/yune-core/src/dictionary/source.rs` - Added correction/tolerance dictionary metadata, source-header parsing, accessors, and merge behavior.
- `crates/yune-core/src/dictionary/mod.rs` - Re-exported correction/tolerance dictionary types.
- `crates/yune-core/src/dictionary/compiled_prism.rs` - Added optional correction/tolerance payload parsing, structured unsupported errors, bounds checks, and count caps.
- `crates/yune-core/src/dictionary/compiled_table.rs` - Added correction/tolerance advanced payload parsing, count caps, and fixed head-index node sizing for multi-code fixtures.
- `crates/yune-core/src/lib.rs` - Re-exported new dictionary types and extended compiled fixture assertions.
- `crates/yune-core/src/translator/mod.rs` - Added correction/tolerance lookup data and expansion ordering to `StaticTableTranslator`.
- `crates/yune-rime-api/src/schema_install.rs` - Parsed prism payloads during compiled dictionary load and merged correction/tolerance metadata into installed dictionaries.
- `crates/yune-rime-api/src/tests/dictionary_data.rs` - Added source/compiled correction/tolerance parity tests and malformed correction/tolerance prism fallback cases.

## Decisions Made

- Used dictionary-level metadata (`TableDictionaryAdvancedData`) as the handoff point for correction/tolerance data so source, table, prism, reverse, and translator paths share one contract.
- Kept the lookup order as original code first, correction canonical code second, and tolerance candidate codes last to preserve exact/completion behavior.
- Treated unsupported or malformed correction/tolerance sections as compiled rejects that can fall back to source dictionaries, matching the Phase 4 fallback policy.
- Added explicit count caps for correction/tolerance payloads to prevent huge-count allocations before lookup or test fallback.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed compiled table head-index node stride**
- **Found during:** Task 3 (schema-loaded correction/tolerance parity tests)
- **Issue:** Multi-code compiled table fixtures exposed that the compiled table reader used an 8-byte head-index node size even though it reads an entry count, entries offset, and next-level offset (12 bytes). This caused compiled correction parity to read the wrong code group.
- **Fix:** Updated `read_head_index_entries` to use a 12-byte node stride and updated the local fixture builder to emit grouped syllabary/index entries for multiple codes.
- **Files modified:** `crates/yune-core/src/dictionary/compiled_table.rs`, `crates/yune-rime-api/src/tests/dictionary_data.rs`
- **Verification:** `cargo test -p yune-rime-api dictionary_data -- --nocapture --test-threads=1`; `cargo test -p yune-core compiled -- --nocapture`
- **Committed in:** `62458c2`

**2. [Rule 2 - Missing Critical] Added correction/tolerance count caps**
- **Found during:** Task 3 malformed-section tests and threat model T-04-04-02
- **Issue:** Huge correction/tolerance counts could attempt excessive allocation before failing on payload length.
- **Fix:** Added parser caps for correction count, tolerance rule count, and tolerance candidate count in compiled prism/table correction-tolerance readers.
- **Files modified:** `crates/yune-core/src/dictionary/compiled_prism.rs`, `crates/yune-core/src/dictionary/compiled_table.rs`
- **Verification:** Malformed huge-count ABI fallback cases pass and report `InvalidCount`.
- **Committed in:** `62458c2`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 missing critical mitigation)
**Impact on plan:** Both fixes were required for correctness and threat-model compliance. No out-of-scope LevelDB/userdb/predictive/plugin/AI behavior was added.

## Issues Encountered

- The plan's first listed cargo command has invalid Cargo syntax because Cargo accepts one test filter before `--`. Equivalent focused verification used `cargo test -p yune-core compiled -- --nocapture`.
- A failed intermediate test run poisoned the process-global test mutex, so subsequent verification ran focused/serialized tests in fresh cargo invocations.
- Existing `schema_install.rs` deferral strings contain `LevelDB/userdb`; filtered anti-pattern checks confirmed these are pre-existing deferral text rather than Phase 5 implementation.

## Verification

- `cargo test -p yune-core compiled -- --nocapture` - passed, 8 tests.
- `cargo test -p yune-core translator:: -- --nocapture` - passed, 0 matching tests.
- `cargo test -p yune-rime-api dictionary_data -- --nocapture --test-threads=1` - passed, 12 tests.
- `cargo test --workspace` - passed.
- `cargo clippy --workspace --all-targets -- -D warnings` - passed.
- Acceptance greps for correction/tolerance, ABI calls, checked parsing arithmetic, and no external command execution passed.

## Known Stubs

None. Stub scan only found pre-existing test names/fixture text for YAML placeholder parsing in `crates/yune-core/src/lib.rs`, unrelated to DATA-04 runtime behavior.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

DATA-04 is complete for the Phase 4 compiled-data scope. Phase 5 can build on the explicit boundary that correction/tolerance lookup metadata does not require LevelDB/userdb learning, predictive frequency updates, plugin translators, or AI-native ranking.

## Self-Check: PASSED

Verified summary/source files exist and task commits `fbef489`, `c3151dd`, `fab3c1d`, and `62458c2` are present in git history.

---
*Phase: 04-compiled-dictionary-data*
*Completed: 2026-04-29*
