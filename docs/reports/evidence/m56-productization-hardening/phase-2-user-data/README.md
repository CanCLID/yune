# M56 Phase 2 User-Data Lifecycle Evidence

Date: 2026-07-04

## Disposition

- Learning and same-process persistence: existing `userdb_learning_persists_session_commits_and_reloads_candidates`.
- Full reinitialize persistence: added `userdb_learning_survives_full_rime_reinitialize`.
- Corrupt committed store: added `userdb_corrupt_committed_store_fails_closed_but_keeps_table_candidates`. Current Yune-defined policy is fail-closed without merging corrupt rows; table candidates remain available. Automatic recovery requires a validated snapshot.
- Interrupted temp write recovery: existing `userdb_recovery_interrupted_temp_write_keeps_last_committed_store_readable`.
- Format freeze and migration baseline: added `userdb_file_store_format_freeze_and_bad_import_rollback`, pinning the current text store header, metadata fields, packed value format, ordering, and bad-import rollback.
- Snapshot backup/restore and local sync: existing `userdb_backup_restore_exports_typed_metadata_and_records`, `levers_user_dict_file_operations_handle_plain_userdb_files`, and `userdb_sync_merges_plain_snapshots_and_backs_up_current_state`.
- Partial sync failure: added `userdb_sync_partial_peer_failure_merges_valid_snapshots_and_reports_failure`.
- Logical-name validation and null/degenerate user-dict levers basics remain covered by existing userdb/resource-id tests; broader ABI abuse is Phase 3.

## Commands Run

```powershell
cargo test -p yune-rime-api userdb:: -- --nocapture
```

Result: 12 passed, 0 failed.

## Named Blocked Rows

None for Phase 2. Learning/ranking, format, recovery, and sync behavior are Yune-defined contract rows unless a future milestone adds direct librime learning oracle captures.
