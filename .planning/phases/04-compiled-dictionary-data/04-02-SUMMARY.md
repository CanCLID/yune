---
phase: 04-compiled-dictionary-data
plan: 02
subsystem: data
tags: [rust, rime, dictionary, deployment, checksums]

requires:
  - phase: 04-compiled-dictionary-data
    provides: Plan 04-01 compiled table/prism/reverse payload readers and runtime selection paths
provides:
  - Extended table/prism/reverse rebuild planning with pack checksum chaining and partial artifact reports
  - Workspace deployment execution for deterministic Rust-generated dictionary artifacts
  - Deployment coverage for freshness, pack changes, forced rebuild flags, prebuilt reuse, and fail-closed dictionary IDs
affects: [runtime-selection, schema-deployment, dictionary-data]

tech-stack:
  added: []
  patterns:
    - Checksum-driven rebuild planning with explicit artifact status reports
    - Validated logical resource IDs before dictionary source/prebuilt/staging path construction
    - Deterministic local compiled fixture serialization without external compiler commands

key-files:
  created:
    - .planning/phases/04-compiled-dictionary-data/04-02-SUMMARY.md
  modified:
    - crates/yune-core/src/dictionary/compiled.rs
    - crates/yune-core/src/dictionary/mod.rs
    - crates/yune-core/src/lib.rs
    - crates/yune-rime-api/src/deployment.rs
    - crates/yune-rime-api/src/tests/mod.rs
    - crates/yune-rime-api/src/tests/deployment.rs

key-decisions:
  - "Use RimeDictRebuildExecutionReport statuses for table, prism, and reverse artifacts so deployment tests can assert partial rebuild/reuse behavior."
  - "Compute deployment freshness from source/schema/pack checksums rather than mtimes, while normalizing generated schema build metadata out of prism signatures."
  - "Keep rebuild execution entirely in Rust by emitting the supported local compiled formats accepted by existing Plan 04-01 readers."

patterns-established:
  - "Deployment dictionary IDs and pack IDs are logical resource IDs validated before any filesystem path is constructed."
  - "Existing staging artifacts take precedence for freshness; prebuilt artifacts are fallback inputs when staging artifacts are missing."
  - "Source dictionaries are parsed with imports, packs, and vocabulary loaders before writing table/reverse artifacts."

requirements-completed: [DATA-02]

duration: 2h 10m
completed: 2026-04-29
---

# Phase 04 Plan 02: Deterministic Dictionary Rebuild Execution Summary

**Checksum-driven table/prism/reverse rebuild execution in Rust with explicit partial reports, pack chaining, prebuilt reuse, and fail-closed deployment validation.**

## Performance

- **Duration:** 2h 10m
- **Started:** 2026-04-29T15:28:00Z
- **Completed:** 2026-04-29T17:38:01Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- Extended core rebuild planning so table, prism, and reverse artifacts each report `Rebuilt`, `ReusedFresh`, `ReusedPrebuilt`, or `MissingSourceAndCompiled`.
- Added pack checksum chaining and source-unavailable prebuilt fallback semantics to the core planner.
- Integrated workspace deployment with deterministic Rust artifact writers for `.table.bin`, `.prism.bin`, and `.reverse.bin` outputs.
- Added deployment-visible rebuild reports and tests covering fresh reuse, pack changes, forced prism rebuilds, prebuilt reuse, unsafe IDs, and missing source/compiled failures.
- Verified no external compiler execution is used for rebuilds.

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend rebuild planning for reverse artifacts, pack checksum chaining, and partial reports**
   - `cbef863` test(04-02): add failing rebuild planner coverage
   - `c7524f1` feat(04-02): extend rebuild planner reports
2. **Task 2: Execute deterministic dictionary artifact rebuilds during workspace deployment**
   - `987d7da` test(04-02): add failing deployment report accessor coverage
   - `a805de1` feat(04-02): rebuild dictionary artifacts during deployment
   - `06fbf56` feat(04-02): stabilize deployment dictionary rebuilds
3. **Task 3: Cover deployment rebuild decisions with local artifact tests**
   - `3ed7877` test(04-02): cover deployment dictionary freshness
   - `32cf913` test(04-02): cover deployment rebuild decisions

**Plan metadata:** pending final docs commit.

_Note: TDD tasks have multiple commits (test then implementation/fix coverage)._ 

## Files Created/Modified

- `crates/yune-core/src/dictionary/compiled.rs` - Extended checksum/rebuild model with pack inputs, reverse planning, prebuilt fallback, partial report statuses, and explicit missing-source errors.
- `crates/yune-core/src/dictionary/mod.rs` - Re-exported new rebuild report/status types.
- `crates/yune-core/src/lib.rs` - Re-exported new core types and added focused rebuild planner tests.
- `crates/yune-rime-api/src/deployment.rs` - Integrated dictionary artifact discovery, validated path construction, metadata freshness checks, deterministic artifact writers, report tracking, and schema signature normalization.
- `crates/yune-rime-api/src/tests/mod.rs` - Imported deployment report accessor for tests.
- `crates/yune-rime-api/src/tests/deployment.rs` - Added deployment rebuild tests for fresh reuse, pack changes, forced rebuild, prebuilt reuse, and failure paths.

## Decisions Made

- Used one `RimeDictRebuildExecutionReport` containing per-artifact statuses instead of opaque booleans, because deployment needs observable partial rebuild outcomes.
- Made deployment schema checksums stable by removing generated `__build_info` before checksum calculation, preventing prism rebuilds solely due to deployment metadata.
- Allowed prism freshness metadata to come from either metadata headers or the supported local payload parser, because locally generated prism artifacts intentionally omit unsupported Darts sections while remaining runtime-readable.
- Kept all dictionary and pack resolution inside validated runtime roots by using `validate_data_resource_id` before constructing `{id}.dict.yaml` and compiled artifact paths.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Prevented local prism artifacts from rebuilding forever**
- **Found during:** Task 2 (deployment rebuild execution)
- **Issue:** Locally generated prism artifacts were runtime-readable but failed the metadata freshness check requiring the unsupported double-array header field, so every subsequent workspace update reported prism as `Rebuilt` instead of `ReusedFresh`.
- **Fix:** Added `prism_checksum_metadata` to extract checksum metadata from the supported local prism payload parser when the header-only metadata parser rejects the artifact.
- **Files modified:** `crates/yune-rime-api/src/deployment.rs`
- **Verification:** `cargo test -p yune-rime-api workspace_update_rebuilds_source_dictionary_artifacts_and_reuses_fresh_outputs -- --nocapture`; `cargo test -p yune-rime-api deployment -- --nocapture`
- **Committed in:** `06fbf56`

**2. [Rule 1 - Bug] Stabilized schema checksum inputs across deployment runs**
- **Found during:** Task 2 (deployment rebuild execution)
- **Issue:** The prism schema checksum could include generated `__build_info`, causing checksum drift between source and deployed schema YAML.
- **Fix:** Normalized schema config signatures by removing top-level `__build_info` before serialization.
- **Files modified:** `crates/yune-rime-api/src/deployment.rs`
- **Verification:** Focused deployment freshness test and full deployment test suite pass.
- **Committed in:** `06fbf56`

**3. [Rule 1 - Bug] Fixed generated table artifacts for duplicate codes**
- **Found during:** Task 2 (deployment rebuild execution)
- **Issue:** Generated table artifacts wrote duplicate syllable/head nodes for entries sharing the same code, which made local table parsing fail for realistic dictionaries containing multiple candidates per code.
- **Fix:** Grouped table entries by code before serializing syllabary and head index sections.
- **Files modified:** `crates/yune-rime-api/src/deployment.rs`
- **Verification:** Focused deployment freshness test parses generated `luna.table.bin` successfully.
- **Committed in:** `06fbf56`

**4. [Rule 2 - Missing Critical] Loaded imports/packs during artifact rebuild execution**
- **Found during:** Task 3 (deployment rebuild decision coverage)
- **Issue:** Deployment checksum planning accounted for packs, but table/reverse writers initially parsed only the primary source dictionary, so rebuilt artifacts could omit pack entries.
- **Fix:** Added workspace dictionary loading through `parse_rime_dict_yaml_with_imports_packs_and_vocabulary` with validated shared-data loaders.
- **Files modified:** `crates/yune-rime-api/src/deployment.rs`
- **Verification:** `workspace_update_rebuilds_after_pack_changes_and_honors_force_flags` asserts pack entries appear in generated table artifacts and pack updates rebuild artifacts.
- **Committed in:** `06fbf56`, covered by `32cf913`

---

**Total deviations:** 4 auto-fixed (3 Rule 1 bugs, 1 Rule 2 missing critical functionality)
**Impact on plan:** All fixes were necessary for correctness of planned rebuild/freshness behavior; no architectural changes or user decisions were required.

## Issues Encountered

- `cargo test -p yune-core dictionary::compiled` is a valid plan command but matches zero tests in this workspace; the focused rebuild planner tests are matched by `rime_dict_rebuild` and pass.
- A formatting check initially revealed unrelated pre-existing formatting differences in files outside the current task; those formatting-only changes were reverted to avoid broad unrelated commits.
- Deployment schema freshness tests that rewrote schema files in-place exposed existing mtime-based config deployment behavior; tests use explicit deployed-file removal where necessary to exercise forced rebuild configuration deterministically.

## Verification

- `cargo test -p yune-core dictionary::compiled -- --nocapture` - passed (0 matched tests)
- `cargo test -p yune-core rime_dict_rebuild -- --nocapture` - passed (3 tests)
- `cargo test -p yune-rime-api deployment -- --nocapture` - passed (35 tests)
- `cargo test --workspace` - passed
- `cargo clippy --workspace --all-targets -- -D warnings` - passed

## Known Stubs

None. Stub scan only found pre-existing parser tests that intentionally use the word `placeholder` for null-column parsing fixtures.

## Threat Flags

None. New deployment trust-boundary behavior was covered by the plan threat model: dictionary/pack resource ID validation, checksum-driven rebuilds, partial reporting, bounded parser reuse, and no external compiler execution.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 04-02 is ready for verifier review. Deployment now produces deterministic local compiled artifacts and exposes partial rebuild reports for downstream runtime/schema work. Remaining broader compiled-format limitations from Plan 04-01 still apply to unsupported full MARISA/Darts sections, but this plan's local fixture-compatible execution path is covered.

## Self-Check: PASSED

- FOUND: `.planning/phases/04-compiled-dictionary-data/04-02-SUMMARY.md`
- FOUND commits: `cbef863`, `c7524f1`, `987d7da`, `a805de1`, `06fbf56`, `3ed7877`, `32cf913`
- Working tree clean before final summary commit except for this summary file.

---
*Phase: 04-compiled-dictionary-data*
*Completed: 2026-04-29*
