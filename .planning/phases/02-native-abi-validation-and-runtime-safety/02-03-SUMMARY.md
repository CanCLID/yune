---
phase: 02-native-abi-validation-and-runtime-safety
plan: 03
subsystem: native-abi-runtime-safety
tags: [rust, rime, abi, path-traversal, resource-id-validation]

requires:
  - phase: 01-cli-frontend-surrogate
    provides: CLI/frontend surrogate and librime compatibility fixtures used by workspace verification
provides:
  - Shared logical resource-ID validation helpers for config, data, schema config, and user dictionary names
  - Runtime boundary validation before config/data/userdb filesystem joins
  - Focused and integration tests for unsafe resource-ID rejection across C ABI, schema YAML, deployment, and userdb paths
affects: [phase-02, abi-runtime-safety, schema-loading, deployment, userdb]

tech-stack:
  added: []
  patterns:
    - Pure string validation before runtime-root Path::join calls
    - ABI fail-closed conventions for unsafe logical IDs
    - Separate config API, runtime config, schema config, data, and userdict validators

key-files:
  created:
    - crates/yune-rime-api/src/resource_id.rs
    - crates/yune-rime-api/src/tests/resource_id.rs
  modified:
    - crates/yune-rime-api/src/lib.rs
    - crates/yune-rime-api/src/config_api.rs
    - crates/yune-rime-api/src/config_compiler.rs
    - crates/yune-rime-api/src/deployment.rs
    - crates/yune-rime-api/src/schema_install.rs
    - crates/yune-rime-api/src/userdb.rs
    - crates/yune-rime-api/src/tests/mod.rs
    - crates/yune-rime-api/src/tests/resource_id.rs

key-decisions:
  - "Validate logical resource IDs with explicit platform-independent string checks rather than Path::components so Unix runs still reject Windows drive prefixes and backslashes."
  - "Keep explicit userdb import/export/restore file path parameters as arbitrary paths while validating only the derived logical dictionary names joined into runtime roots."
  - "Preserve .schema logical config IDs for compiler/deployment references, while routing direct schema opens through schema-specific normalization."

patterns-established:
  - "Fail closed at ABI/resource boundaries using existing conventions: FALSE, -1, None, or Value::Null."
  - "Validate schema YAML-controlled dictionary/import/pack/vocabulary IDs before selected runtime data path lookups."

requirements-completed: [ABI-03]

duration: continued-session
completed: 2026-04-29T04:09:49Z
---

# Phase 02 Plan 03: Resource-ID Runtime Safety Summary

**Logical resource-ID validation now blocks traversal and filesystem syntax across RIME config, schema data, deployment, and userdb runtime path construction.**

## Performance

- **Duration:** continued session after context compaction
- **Started:** prior session before compaction
- **Completed:** 2026-04-29T04:09:49Z
- **Tasks:** 3/3
- **Files modified:** 9

## Accomplishments

- Added `resource_id` validators for config resources, runtime config resources, schema config resources, data resources, config API opens, and user dictionary names.
- Added focused tests covering safe logical IDs and unsafe traversal, separators, absolute paths, Windows drive prefixes, tilde paths, NUL, and invalid userdb suffixes.
- Wired validation into C ABI config/schema opens, runtime config/data selectors, deployment file handling, schema dictionary/import/pack/vocabulary loading, config compiler external references, and userdb backup/restore/export/import/sync paths.
- Preserved compatibility for safe fixture names and explicit import/export/restore file path arguments while rejecting unsafe logical names before runtime-root joins.
- Verified formatting, focused tests, package tests, and the full workspace test suite.

## Task Commits

Each task was committed atomically; TDD tasks include RED and GREEN commits:

1. **Task 1: Define and test shared logical resource-ID validation**
   - `f7002d1` test(02-03): add failing tests for resource ID validation
   - `fd13b5a` feat(02-03): implement shared resource ID validation
2. **Task 2: Integrate validation into config, schema data, deployment, and userdb paths**
   - `5735ce9` test(02-03): add failing resource ID boundary tests
   - `2f6ba96` feat(02-03): validate resource IDs at runtime boundaries
3. **Task 3: Verify package and workspace safety gates**
   - `5b780a3` fix(02-03): preserve schema config resource IDs

**Plan metadata:** committed separately by the final summary commit.

## Files Created/Modified

- `crates/yune-rime-api/src/resource_id.rs` - Shared logical resource-ID validators and platform-independent rejection logic.
- `crates/yune-rime-api/src/tests/resource_id.rs` - Focused and boundary tests for resource-ID validation and fail-closed ABI behavior.
- `crates/yune-rime-api/src/tests/mod.rs` - Registered the resource-ID test module.
- `crates/yune-rime-api/src/lib.rs` - Validates runtime config/data IDs before selecting paths or loading roots.
- `crates/yune-rime-api/src/config_api.rs` - Validates C ABI config/schema IDs before opening runtime configs.
- `crates/yune-rime-api/src/config_compiler.rs` - Handles optional validated config references in compiler include/patch loading.
- `crates/yune-rime-api/src/deployment.rs` - Validates deployment logical filenames, schema IDs, dependencies, and workspace schema IDs before joins.
- `crates/yune-rime-api/src/schema_install.rs` - Validates schema YAML dictionary/import/pack/vocabulary IDs before runtime data reads.
- `crates/yune-rime-api/src/userdb.rs` - Validates user dictionary logical names before backup, restore-derived destination, export, import, sync, and snapshot joins.

## Decisions Made

- Used explicit string checks for separators, traversal values, NUL, tilde prefixes, and Windows drive prefixes to keep validation deterministic across platforms.
- Added separate validators for runtime config IDs, schema config IDs, and direct config API IDs so schema opens can normalize `schema_id` to `<schema_id>.schema` without breaking compiler/deployment references.
- Kept explicit file path parameters for userdb import/export/restore as file paths, matching plan scope; only derived dictionary names are treated as logical resource IDs.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated config compiler for optional validated config IDs**
- **Found during:** Task 2 (integration)
- **Issue:** Changing `normalize_config_resource_id` to return `Option<String>` broke config compiler reference loading at compile time.
- **Fix:** Propagated invalid external config references as `None` before shared-data path construction.
- **Files modified:** `crates/yune-rime-api/src/config_compiler.rs`
- **Verification:** `cargo test -p yune-rime-api config_api -- --nocapture`, `cargo test -p yune-rime-api`, and `cargo test --workspace`.
- **Committed in:** `2f6ba96`

**2. [Rule 1 - Bug] Preserved schema config logical IDs during compiler/deployment references**
- **Found during:** Task 3 (package verification)
- **Issue:** Full package tests showed `deploy_schema_expands_librime_key_binder_import_preset` regressed because general config normalization stripped `.schema`, changing deployment/compiler behavior for schema config references.
- **Fix:** Adjusted general config validation to strip only terminal `.yaml`, added a direct config API validator for non-schema opens, and kept schema-specific normalization in `validate_schema_config_resource_id`.
- **Files modified:** `crates/yune-rime-api/src/resource_id.rs`, `crates/yune-rime-api/src/config_api.rs`, `crates/yune-rime-api/src/tests/resource_id.rs`
- **Verification:** `cargo test -p yune-rime-api tests::deployment::deploy_schema_expands_librime_key_binder_import_preset -- --nocapture --test-threads=1`, `cargo test -p yune-rime-api`, and `cargo test --workspace`.
- **Committed in:** `5b780a3`

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes were required to keep the planned validation changes compiling and compatible with existing safe schema deployment behavior. No D-07 schema semantics or userdb storage compatibility work was added.

## Issues Encountered

- `cargo` was not on the default PATH in this environment. Verification commands were run with `PATH="/Users/trenton/.cargo/bin:$PATH"`.
- The plan verification command `cargo test -p yune-rime-api config_api userdb -- --nocapture` is not valid Cargo syntax with multiple test filters; equivalent filters were run separately for `config_api` and `userdb`.
- `cargo fmt --check` identified import formatting after the final validation adjustment; `cargo fmt` fixed it and subsequent format checks passed.

## Verification

- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo fmt --check` - passed
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test -p yune-rime-api resource_id -- --nocapture` - passed, 11 focused resource-ID tests
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test -p yune-rime-api config_api -- --nocapture` - passed, 18 filtered tests
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test -p yune-rime-api userdb -- --nocapture` - passed, 5 filtered tests
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test -p yune-rime-api` - passed, 233 unit tests, 33 frontend client tests, doc tests
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test --workspace` - passed, including yune-cli, yune-core, yune-rime-api, yune-schema, integration tests, and doc tests

## Known Stubs

None found in files created or modified by this plan.

## Threat Flags

None. The plan intentionally touched existing filesystem trust boundaries and added validation before joins; no new network endpoints, auth paths, file access patterns outside the plan threat model, or schema trust boundaries were introduced.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- ABI-03 resource-ID safety is complete for config, schema/dictionary/custom-settings inputs, deployment logical filenames, and userdb logical dictionary names.
- Future D-07 work can build on these validators without treating this plan as schema semantic validation, compiled dictionary behavior, or userdb storage compatibility.

## Self-Check: PASSED

- Created files exist: `crates/yune-rime-api/src/resource_id.rs`, `crates/yune-rime-api/src/tests/resource_id.rs`.
- Modified integration files are present and verified by the package/workspace test suites.
- Task commits exist: `f7002d1`, `fd13b5a`, `5735ce9`, `2f6ba96`, `5b780a3`.

---
*Phase: 02-native-abi-validation-and-runtime-safety*
*Completed: 2026-04-29T04:09:49Z*
