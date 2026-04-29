---
phase: 03-schema-pipeline-depth
plan: 04
subsystem: schema-pipeline-depth
tags: [rust, rime, schema-selection, spelling-algebra, opencc]

requires:
  - phase: 03-schema-pipeline-depth
    provides: Distribution comparison findings from plan 03-03
provides:
  - Schema-loaded focused tests for spelling algebra generated spellings and penalty ranking
  - YAML-backed correction/tolerance ranking boundary coverage
  - OpenCC/simplifier filter-chain integration coverage for limited built-in maps
affects: [04-compiled-dictionary-data, schema-pipeline-depth]

tech-stack:
  added: []
  patterns:
    - ABI-facing schema-loaded temp runtime fixtures
    - Structured deferral to Phase 4 for compiled correction/tolerance and OpenCC data parity gaps

key-files:
  created:
    - .planning/phases/03-schema-pipeline-depth/03-04-SUMMARY.md
  modified:
    - crates/yune-rime-api/src/tests/schema_selection.rs

decisions:
  - "Used schema-loaded ABI fixtures for spelling algebra, YAML-backed correction/tolerance ranking, and OpenCC filter-chain behavior rather than direct core-only tests."
  - "No compiled payload readers, LevelDB/userdb, plugin, or AI-native implementations were added."
  - "OpenCC assertions cover filter-chain tag gating and limited built-in maps only; full conversion-data parity remains out of Phase 3."

metrics:
  duration: 28min
  tasks: 3
  files_modified: 2
  completed: 2026-04-29T10:18:26Z

requirements-completed: [SCHEMA-05]
---

# Phase 03 Plan 04: Spelling, Correction/Tolerance, and OpenCC Boundaries Summary

**Schema-loaded tests lock targeted spelling algebra, YAML-backed correction/tolerance ranking, and limited OpenCC filter-chain semantics without crossing compiled-data boundaries.**

## Performance

- **Duration:** 28 min
- **Started:** 2026-04-29T09:50:00Z
- **Completed:** 2026-04-29T10:18:26Z
- **Tasks:** 3
- **Files modified:** 2 for this plan, including this summary

## Accomplishments

- Added focused ABI-facing tests in `crates/yune-rime-api/src/tests/schema_selection.rs` for:
  - `schema_spelling_algebra_generated_spellings_match_librime`
  - `schema_tolerance_lookup_yaml_backed_ranking_matches_librime`
  - `schema_opencc_filter_chain_integration_matches_librime_limited_maps`
- Confirmed existing `yune-core` spelling algebra and translator behavior already covered the targeted YAML-backed formulas and penalties needed by Phase 3 comparisons.
- Confirmed existing simplifier/OpenCC implementation already supported the required limited map and tag-gated filter-chain semantics when represented by schema YAML.
- Preserved existing uncommitted changes from plans 03-01, 03-02, and 03-03 and did not overwrite their files beyond appending this plan's schema-selection tests.
- Created this summary without committing, per user instruction.

## Implemented spelling algebra increments

No production code increment was required after adding the focused schema-loaded tests. Existing implementation already supports the represented RIME formula family used by Phase 3 comparisons:

- `xform` replacement for generated spellings such as `lue -> lve`.
- `derive` generated spellings that keep original entries and add generated lookup forms.
- `fuzz` and `abbrev` penalties that participate in candidate ordering.
- `derive/.../correction` penalty for YAML-backed correction behavior.
- `erase` removal for full-code matches.
- `xlit` transliteration for generated lookup forms.

The new schema-loaded test exercises these through `RimeSelectSchema` and `RimeProcessKey`, not direct core-only setup.

## Correction/tolerance boundaries

YAML-backed correction/tolerance behavior is covered where it can be represented through source dictionary entries and `speller/algebra` formulas:

- Exact `cu` table candidate ranks ahead of correction-derived `cuo -> cu` candidate.
- Correction-derived candidate remains available without compiled prism/table/reverse payload consumption.
- Fuzzy spelling `bing -> pin` ranks below exact `pin` but above echo fallback.

Compiled-payload-dependent correction/tolerance behavior remains out of Phase 3. Any behavior requiring `.prism.bin`, `.table.bin`, `.reverse.bin`, or compiled correction/tolerance data belongs to `04-compiled-dictionary-data`.

## OpenCC/filter-chain boundaries

OpenCC/filter-chain coverage is limited to schema-visible simplifier integration:

- `simplifier@zh_simp` is installed from schema YAML.
- `opencc_config: t2s.json` selects the existing limited built-in traditional-to-simplified map.
- Filter tags gate conversion to the intended schema segment, so reverse lookup candidates tagged outside that filter are not converted.
- Comment tips and formatting are verified for converted table candidates.

Full OpenCC conversion-data parity is not claimed. The built-in maps are intentionally limited and do not consume OpenCC conversion data files; full conversion-data parity remains outside Phase 3 and belongs with future data compatibility work.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Split invalid combined Cargo filter command**
- **Found during:** Task 1 verification
- **Issue:** The exact plan command `cargo test -p yune-rime-api distribution_schema_comparison schema_selection -- --nocapture` is invalid Cargo syntax because Cargo accepts one test filter argument.
- **Fix:** Ran `distribution_schema_comparison` and `schema_selection` as separate focused test invocations.
- **Files modified:** None
- **Verification:** Both separate focused commands passed.
- **Commit:** Not committed per user instruction.

**2. [Rule 1 - Bug] Aligned new focused test expectations with ABI-visible echo fallback and tag semantics**
- **Found during:** Task 1 and Task 2 verification
- **Issue:** Initial focused assertions omitted echo fallback candidates and used matcher tags in a way that kept the simplifier tag active during reverse lookup.
- **Fix:** Updated expected candidate lists to include echo fallback and used affix segmentor tags to make OpenCC tag-gating explicit.
- **Files modified:** `crates/yune-rime-api/src/tests/schema_selection.rs`
- **Verification:** Focused schema-selection tests passed.
- **Commit:** Not committed per user instruction.

---

**Total deviations:** 2 auto-fixed. No architecture changes were required.

## Verification

Passed:

- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" -p yune-rime-api distribution_schema_comparison -- --nocapture`
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" -p yune-rime-api schema_selection -- --nocapture`
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo fmt --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" --all --check`
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo test --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" --workspace`
- `PATH="/Users/trenton/.cargo/bin:$PATH" cargo clippy --manifest-path "/Users/trenton/Projects/yune/Cargo.toml" --workspace --all-targets -- -D warnings`

Command caveat:

- The exact combined focused test command from the plan failed due Cargo CLI syntax; equivalent separate test-filter invocations passed.

## Known Stubs

None introduced by this plan. The OpenCC coverage intentionally uses limited built-in maps and documents the full conversion-data boundary instead of stubbing data-file parity.

## Threat Flags

None. This plan adds ABI tests and a planning summary only; it does not introduce new network endpoints, auth paths, persistent runtime file access, compiled payload readers, or raw filesystem path construction.

## User Setup Required

None.

## Next Phase Readiness

- Phase 4 should implement compiled dictionary/prism/reverse and compiled correction/tolerance data before claiming broader correction/tolerance parity.
- Full OpenCC conversion data parity remains future data compatibility work; Phase 3 only verifies filter-chain semantics and limited built-in maps.

## Self-Check: PASSED

- Found modified file: `crates/yune-rime-api/src/tests/schema_selection.rs`
- Found created summary: `.planning/phases/03-schema-pipeline-depth/03-04-SUMMARY.md`
- Verified focused test names are present exactly once each.
- Verified no commits were created because the user explicitly requested no commits.

---
*Phase: 03-schema-pipeline-depth*
*Completed: 2026-04-29*
