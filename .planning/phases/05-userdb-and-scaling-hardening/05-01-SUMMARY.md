---
phase: 05-userdb-and-scaling-hardening
plan: 01
subsystem: yune-rime-api userdb storage
tags: [userdb, storage, sync, recovery, c-abi]
dependency_graph:
  requires: [USERDB-01, USERDB-02, QUAL-04]
  provides:
    - typed userdb record/value storage contracts
    - file-backed atomic transaction store
    - validated snapshot/import/export/restore paths
    - conflict-aware sync merge semantics
  affects:
    - crates/yune-rime-api/src/userdb.rs
    - crates/yune-rime-api/src/userdb/*
    - crates/yune-rime-api/src/tests/userdb.rs
    - crates/yune-rime-api/tests/frontend_client.rs
tech_stack:
  added: []
  patterns:
    - Rust file-backed compatible userdb store
    - temp-file plus rename commit path
    - C ABI facade delegating to focused modules
key_files:
  created:
    - crates/yune-rime-api/src/userdb/mod.rs
    - crates/yune-rime-api/src/userdb/record.rs
    - crates/yune-rime-api/src/userdb/store.rs
    - crates/yune-rime-api/src/userdb/file_store.rs
    - crates/yune-rime-api/src/userdb/snapshot.rs
    - crates/yune-rime-api/src/userdb/sync.rs
    - crates/yune-rime-api/src/userdb/recovery.rs
  modified:
    - crates/yune-rime-api/src/lib.rs
    - crates/yune-rime-api/src/userdb.rs
    - crates/yune-rime-api/src/tests/mod.rs
    - crates/yune-rime-api/src/tests/userdb.rs
    - crates/yune-rime-api/src/tests/levers.rs
    - crates/yune-rime-api/tests/frontend_client.rs
decisions:
  - Kept exported RimeLevers*UserDict* functions in userdb.rs and moved storage behavior behind internal userdb modules.
  - Preserved legacy plain userdb import compatibility while committing typed c/d/t records.
  - Used file snapshots and atomic rename semantics instead of adding a LevelDB dependency.
metrics:
  duration: not recorded
  completed_date: 2026-04-30
  tasks_completed: 3
  commits: 4
---

# Phase 05 Plan 01: Userdb and Scaling Hardening Summary

Typed user dictionary storage with librime-style commits/dee/tick values, validated snapshot lifecycle operations, and conflict-aware sync now backs the existing levers C ABI boundary.

## Tasks Completed

| Task | Name | Commit | Notes |
| ---- | ---- | ------ | ----- |
| RED | Add failing userdb storage contract tests | 0d35e96 | Added tests for typed c/d/t storage, invalid logical names, and interrupted temp writes. |
| 1 | Define compatible userdb storage contracts and file-backed transaction store | 80bbbef | Added record/store/file_store modules and routed ABI functions through the manager facade. |
| 2 | Implement validated snapshot, restore, sync, recovery, and rollback semantics | 7b33ea6 | Added backup/restore metadata validation, malformed snapshot fail-closed behavior, and conflict-aware sync tests. |
| 3 | Close storage lifecycle verification gates | 4e5dd57 | Fixed clippy/dead-code issues and updated frontend levers expectations for typed userdb artifacts. |

## Implementation Owner

- `crates/yune-rime-api/src/userdb.rs`: stable C ABI facade for levers/user dictionary functions.
- `crates/yune-rime-api/src/userdb/mod.rs`: internal manager facade for runtime paths, logical dictionary validation, import/export, backup/restore, sync, and upgrade entry points.
- `crates/yune-rime-api/src/userdb/record.rs`: typed userdb records and values with `commits`, `dee`, and `tick` pack/parse helpers.
- `crates/yune-rime-api/src/userdb/store.rs`: `UserDbStore` trait for metadata, ordered/prefix records, get/update/delete, transaction, rollback, and validation operations.
- `crates/yune-rime-api/src/userdb/file_store.rs`: file-backed store with atomic temp-file replacement and transaction backup/rollback.
- `crates/yune-rime-api/src/userdb/snapshot.rs`: metadata-bearing snapshot parser/writer with `db_name` validation.
- `crates/yune-rime-api/src/userdb/sync.rs`: keyed sync merge using larger absolute commits, max dee, and deterministic merge tick.
- `crates/yune-rime-api/src/userdb/recovery.rs`: fail-closed recovery/restore helpers.

## Test Owner

- `crates/yune-rime-api/src/tests/userdb.rs`: focused userdb storage, metadata, sync, invalid-name, and recovery tests.
- `crates/yune-rime-api/src/tests/levers.rs`: ABI levers file-operation coverage updated for typed userdb snapshots and exports.
- `crates/yune-rime-api/tests/frontend_client.rs`: frontend-style levers API coverage updated for typed userdb behavior.
- `crates/yune-rime-api/src/tests/resource_id.rs`: existing logical resource ID coverage remains the validation owner.

## Librime Comparison Targets

- `/Users/trenton/Projects/librime/src/rime/dict/user_db.cc`
- `/Users/trenton/Projects/librime/src/rime/dict/level_db.cc`
- `/Users/trenton/Projects/librime/src/rime/lever/user_dict_manager.cc`
- `/Users/trenton/Projects/librime/src/rime/algo/dynamics.h`

## Verification Commands Run

- `$HOME/.cargo/bin/cargo test -p yune-rime-api userdb -- --nocapture`
- `$HOME/.cargo/bin/cargo test -p yune-rime-api userdb_backup_restore -- --nocapture`
- `$HOME/.cargo/bin/cargo test -p yune-rime-api userdb_sync -- --nocapture`
- `$HOME/.cargo/bin/cargo test -p yune-rime-api userdb_recovery -- --nocapture`
- `$HOME/.cargo/bin/cargo fmt --check`
- `$HOME/.cargo/bin/cargo test -p yune-rime-api userdb`
- `$HOME/.cargo/bin/cargo test -p yune-rime-api resource_id`
- `$HOME/.cargo/bin/cargo test --workspace`
- `$HOME/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings`
- Acceptance greps from Tasks 1, 2, and 3, including no `HashSet` line-dedupe in userdb sync and no `fs::write(destination` committed overwrite path.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Normalized legacy tab-separated userdb keys during parsing**
- **Found during:** Task 1
- **Issue:** Legacy plain `.userdb` files parsed keys without librime's trailing code-space normalization, producing mismatched snapshot/export expectations and duplicate merge keys.
- **Fix:** Canonicalized keys through `UserDbRecord::from_key_value` and stored normalized record keys in `FileUserDbStore`.
- **Files modified:** `crates/yune-rime-api/src/userdb/record.rs`, `crates/yune-rime-api/src/userdb/file_store.rs`
- **Commit:** 80bbbef

**2. [Rule 2 - Missing Critical Functionality] Accepted legacy plain numeric values and metadata fallback**
- **Found during:** Task 1
- **Issue:** Legacy userdb files and snapshots without metadata or packed values could not migrate into the typed store.
- **Fix:** Added numeric legacy value parsing, filename-derived metadata fallback, and default userdb metadata.
- **Files modified:** `crates/yune-rime-api/src/userdb/record.rs`, `crates/yune-rime-api/src/userdb/file_store.rs`, `crates/yune-rime-api/src/userdb/snapshot.rs`
- **Commit:** 80bbbef

**3. [Rule 1 - Bug] Updated frontend levers expectations for typed artifacts**
- **Found during:** Task 3
- **Issue:** Full workspace tests still expected byte-for-byte plain userdb copy/export behavior after the storage contract changed to typed c/d/t records.
- **Fix:** Updated frontend-style levers test assertions to validate typed snapshot metadata, table export format, and typed imported store output.
- **Files modified:** `crates/yune-rime-api/tests/frontend_client.rs`
- **Commit:** 4e5dd57

**4. [Rule 3 - Blocking Issue] Removed clippy-blocking dead code and API nits**
- **Found during:** Task 3
- **Issue:** `cargo clippy --workspace --all-targets -- -D warnings` rejected unused compatibility wrappers, unused parse helpers, `io::ErrorKind::Other` construction, and `&PathBuf` restore arguments.
- **Fix:** Removed unused wrappers/helpers, used `io::Error::other`, changed restore to accept `&Path`, and documented intentionally broad store trait methods with `#[allow(dead_code)]`.
- **Files modified:** `crates/yune-rime-api/src/userdb.rs`, `crates/yune-rime-api/src/userdb/mod.rs`, `crates/yune-rime-api/src/userdb/file_store.rs`, `crates/yune-rime-api/src/userdb/sync.rs`, `crates/yune-rime-api/src/userdb/store.rs`, `crates/yune-rime-api/src/userdb/snapshot.rs`, `crates/yune-rime-api/src/userdb/recovery.rs`, `crates/yune-rime-api/src/userdb/record.rs`
- **Commit:** 4e5dd57

## Known Stubs

None.

## Threat Flags

None beyond the planned userdb ABI, snapshot/import, runtime file store, and sync trust boundaries already covered by the plan threat model.

## Remaining Gaps

None for this plan. Direct LevelDB storage is intentionally not added; the plan accepted a compatible Rust abstraction matching librime-observable behavior.

## Self-Check: PASSED

- Created files exist: `05-01-SUMMARY.md`, `userdb/mod.rs`, `userdb/record.rs`, `userdb/store.rs`, `userdb/file_store.rs`, `userdb/snapshot.rs`, `userdb/sync.rs`, and `userdb/recovery.rs`.
- Commit hashes found in git history: `0d35e96`, `80bbbef`, `7b33ea6`, and `4e5dd57`.
