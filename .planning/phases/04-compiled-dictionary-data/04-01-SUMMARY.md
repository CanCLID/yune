---
phase: 04-compiled-dictionary-data
plan: 01
subsystem: dictionary-data
tags: [compiled-dictionary, schema-install, bounded-parser, resource-id-validation]
dependency_graph:
  requires: [phase-03-schema-pipeline-depth]
  provides: [DATA-01, compiled-dictionary-readers, compiled-source-runtime-selection]
  affects: [yune-core, yune-rime-api]
tech_stack:
  added: [bounded-little-endian-readers, rime-offsetptr-parsing, schema-loaded-regression-fixtures]
  patterns: [fail-closed-binary-parsing, explicit-load-outcome, source-fallback]
key_files:
  created:
    - crates/yune-core/src/dictionary/compiled_table.rs
    - crates/yune-core/src/dictionary/compiled_prism.rs
    - crates/yune-core/src/dictionary/compiled_reverse.rs
    - crates/yune-rime-api/src/tests/dictionary_data.rs
  modified:
    - crates/yune-core/src/dictionary/compiled.rs
    - crates/yune-core/src/dictionary/mod.rs
    - crates/yune-core/src/lib.rs
    - crates/yune-rime-api/src/schema_install.rs
    - crates/yune-rime-api/src/tests/mod.rs
decisions:
  - Keep compiled binary layout parsing in yune-core byte-slice readers and keep schema_install.rs responsible only for validated resource selection.
  - Treat valid source-only dictionaries as normal runtime behavior without adding remaining-gear deferrals; record fallback diagnostics for stale, invalid, or unsupported compiled artifacts.
  - Preserve fail-closed behavior for unsupported MARISA/Darts sections through structured UnsupportedSection errors rather than silently ignoring them.
metrics:
  duration: continued session
  completed_date: 2026-04-29
  tasks_completed: 3
  files_changed: 10
---

# Phase 04 Plan 01: Compiled Dictionary Data Summary

Compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` readers now feed schema dictionary installation with fresh-compiled preference, source fallback, explicit no-usable-path failure reporting, and schema-loaded regression coverage.

## What Changed

- Added bounded core readers for focused RIME compiled dictionary payloads:
  - `parse_rime_table_bin_dictionary`
  - `parse_rime_prism_bin_payload`
  - `parse_rime_reverse_bin_dictionary`
- Exported parser APIs and structured parse errors through `yune-core` dictionary/public exports.
- Reworked schema dictionary loading from an `Option<TableDictionary>` path into `DictionaryLoadOutcome` with compiled, source-fallback, and no-usable-path outcomes.
- Validated dictionary, import, pack, vocabulary, table, prism, and reverse resource names as logical resource IDs before runtime path lookup.
- Added session-level tests proving compiled preference, source fallback, malformed compiled fallback, explicit no-usable-path behavior, resource-ID rejection, and source/compiled candidate ordering parity.

## Task Results

| Task | Name | Commit | Result |
| --- | --- | --- | --- |
| 1 | Add bounded compiled table/prism/reverse readers in the dictionary layer | 8c026c0 | Completed |
| 2 | Wire schema installation to prefer compiled payloads with source fallback and explicit failure | 4834a4d | Completed |
| 3 | Add schema-loaded compiled/source fallback and ordering parity tests | 307eabe | Completed |
| Fix | Satisfy compiled table clippy gate | c980580 | Completed |
| Fix | Avoid deferrals for missing compiled dictionaries | 6ec1fda | Completed |

## Verification

Passed:

- `$HOME/.cargo/bin/cargo test -p yune-core dictionary:: -- --nocapture`
  - Passes, but the filter currently selects 0 tests because the relevant tests live under the crate-level test module.
- `$HOME/.cargo/bin/cargo test -p yune-core compiled_ -- --nocapture`
  - 6 compiled-reader and metadata tests passed.
- `$HOME/.cargo/bin/cargo test -p yune-rime-api dictionary_data -- --nocapture`
  - 5 schema-loaded dictionary data tests passed.
- `$HOME/.cargo/bin/cargo test --workspace -- --test-threads=1`
  - Workspace tests passed serially.
- `$HOME/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings`
  - Clippy passed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Returned `InvalidCount` for oversized compiled table index counts**
- **Found during:** Task 1 verification
- **Issue:** A malformed table fixture with a huge node count returned `OutOfBounds` rather than the structured `InvalidCount` expected for count overflow/huge allocation requests.
- **Fix:** Classified table head-index total-size overflow past the payload as `InvalidCount`.
- **Files modified:** `crates/yune-core/src/dictionary/compiled_table.rs`, `crates/yune-core/src/lib.rs`
- **Commit:** `8c026c0`

**2. [Rule 3 - Blocking] Cargo was not on PATH**
- **Found during:** Task 1 verification
- **Issue:** `cargo` was unavailable through the default shell PATH in this worktree agent.
- **Fix:** Ran verification with `$HOME/.cargo/bin/cargo`.
- **Files modified:** None
- **Commit:** None

**3. [Rule 1 - Bug] Fixed clippy needless range loop in compiled table parser**
- **Found during:** Overall clippy verification
- **Issue:** Clippy rejected indexing `syllables` with a loop variable in `read_head_index_entries`.
- **Fix:** Iterated with `syllables.iter().enumerate().take(count)` and passed the syllable reference directly.
- **Files modified:** `crates/yune-core/src/dictionary/compiled_table.rs`
- **Commit:** `c980580`

**4. [Rule 1 - Bug] Avoided remaining-gear deferrals for normal source-only dictionaries**
- **Found during:** Workspace verification
- **Issue:** Recording `dictionary_source_fallback` for missing compiled artifacts changed an existing distribution comparison test from one finding to two, because source-only dictionaries are normal behavior and should not count as remaining gear deferrals.
- **Fix:** `record_dictionary_source_fallback` now returns without recording when the compiled reject reason is `Missing`; stale, invalid, and unsupported compiled artifacts remain inspectable.
- **Files modified:** `crates/yune-rime-api/src/schema_install.rs`
- **Commit:** `6ec1fda`

## Known Stubs

None introduced for this plan. Stub-pattern scan only found pre-existing source-dictionary placeholder parser tests in `crates/yune-core/src/lib.rs`.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: binary-parser | `crates/yune-core/src/dictionary/compiled_table.rs` | New `.table.bin` byte parser at the compiled-bytes trust boundary; mitigated with checked arithmetic, bounded strings, relative-offset checks, and structured errors. |
| threat_flag: binary-parser | `crates/yune-core/src/dictionary/compiled_prism.rs` | New `.prism.bin` byte parser at the compiled-bytes trust boundary; mitigated with checked reads and unsupported-section errors. |
| threat_flag: binary-parser | `crates/yune-core/src/dictionary/compiled_reverse.rs` | New `.reverse.bin` byte parser at the compiled-bytes trust boundary; mitigated with checked reads and explicit unsupported MARISA section rejection. |
| threat_flag: resource-lookup | `crates/yune-rime-api/src/schema_install.rs` | New compiled/source runtime selection path crosses schema YAML into runtime file lookup; mitigated with `validate_data_resource_id` before each lookup and traversal/absolute/drive-prefix tests. |

## Limitations

- The compiled readers intentionally implement the first usable bounded subset for local fixture-compatible payloads. Full MARISA string tables, Darts double arrays, and multi-level phrase indexes are rejected as structured `UnsupportedSection` errors for future expansion.
- Compiled freshness compares against the primary source dictionary YAML when present; imported tables, packs, and vocabulary still flow through the source parser fallback path but are not yet included in compiled checksum reconstruction.

## Self-Check: PASSED

Verified created files exist:

- `crates/yune-core/src/dictionary/compiled_table.rs`
- `crates/yune-core/src/dictionary/compiled_prism.rs`
- `crates/yune-core/src/dictionary/compiled_reverse.rs`
- `crates/yune-rime-api/src/tests/dictionary_data.rs`
- `.planning/phases/04-compiled-dictionary-data/04-01-SUMMARY.md`

Verified commits exist:

- `8c026c0`
- `4834a4d`
- `307eabe`
- `c980580`
- `6ec1fda`
