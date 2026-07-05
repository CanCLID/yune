# M56 Phase 0 User-Data Ledger

Read-only inventory completed from `84a3c0dedcfed53887427f0d50af5366d5e61f78`.

## Commands

- `rg -n "userdb|UserDb|snapshot|sync|recover|corrupt|truncate|learn|commit|migration|format" crates\yune-rime-api\src\userdb crates\yune-rime-api\src\userdb.rs crates\yune-rime-api\src\levers.rs crates\yune-rime-api\src\tests\userdb.rs`

## Ledger

| Behavior | Definition kind | Current implementation | Existing coverage | Gap / disposition |
| --- | --- | --- | --- | --- |
| Runtime commit records selected candidate metadata before clearing composition | Yune-defined over existing core metadata; oracle capture not present | `crates/yune-core/src/userdb.rs` stages `UserDbCommitMetadata`; ABI runtime path persists via `crates/yune-rime-api/src/userdb/mod.rs::record_runtime_commit` | Core tests in `crates/yune-core/src/userdb.rs`; ABI test `userdb_learning_persists_session_commits_and_reloads_candidates` | Tested, but support contract should state the persistence guarantee. |
| Learned same-code candidate ranking effect | Mixed: librime learning semantics are oracle-defined, current Yune threshold behavior is Yune-defined where no oracle fixture exists | `UserDb::learn_entry` and translator/userdb merge in core; runtime load in `load_runtime_userdb` | `userdb_learning_persists_session_commits_and_reloads_candidates`; TypeDuck parity has same-code low-weight non-preemption coverage | Phase 2 must either add oracle capture for learning effect or document the Yune-defined ranking contract. |
| Longer-code predictive userdb candidate preempts shorter table duplicate | Yune-defined TypeDuck/product behavior | Core userdb predictive query and translator merge | Core `predictive_userdb_longer_code_preempts_shorter_table_candidate`; ABI `userdb_learning_persists_session_commits_and_reloads_candidates` | Tested; contract row needed. |
| Persistence across session recreate in same process | Yune-defined | `record_runtime_commit` writes `*.userdb`; schema selection reloads via `load_runtime_userdb` | ABI `userdb_learning_persists_session_commits_and_reloads_candidates` destroys/recreates session and reloads candidates | Tested. |
| Persistence across process restart / full RIME reinitialize | Yune-defined product lifecycle | Same store load path should work after `RimeFinalize`/`RimeInitialize` or fresh process | No direct process-restart/full-finalize test identified in `src/tests/userdb.rs` | Phase 2 gap: add restart-style test using fresh runtime dirs and reinitialization. |
| Store format parse and write | Yune-defined format freeze | `file_store.rs` writes text header plus metadata and `code<TAB>phrase<TAB>c/d/t` values; no explicit format-version row | Backup/restore and runtime commit tests assert representative rows | Phase 2 gap: add format-freeze test or explicit migration/format contract. |
| Store transaction atomicity and interrupted temp write recovery | Yune-defined | `FileUserDbStore::commit_transaction` writes `*.userdb.tmp` then renames; `recovery.rs::recover_user_dict` removes stale temp when current store opens | `userdb_recovery_interrupted_temp_write_keeps_last_committed_store_readable` | Tested; contract row needed. |
| Corrupt/truncated committed store recovery | Yune-defined | `FileUserDbStore::open` returns `InvalidData`; `recover_user_dict` can restore only when a validated snapshot is passed | No direct corrupt committed-store recovery test identified | Phase 2 gap: add corrupt/truncated store recovery test with bounded data-loss policy. |
| Snapshot backup metadata and records | Yune-defined/librime-shaped text surface | `snapshot.rs::write_snapshot`; levers backup/export surfaces call manager | `userdb_backup_restore_exports_typed_metadata_and_records`; export/import part of `levers_user_dict_file_operations_handle_plain_userdb_files` | Tested; contract row should cite snapshot shape. |
| Snapshot restore validation | Yune-defined safety behavior | `snapshot.rs::read_snapshot` validates metadata and names; `recovery.rs::restore_validated_snapshot` requires snapshot db name to resolve safely | Malformed snapshot row in `userdb_rejects_malformed_logical_names_before_store_creation`; restore coverage in backup/export tests | Mostly tested; Phase 2 should add explicit invalid metadata/truncated snapshot row if absent. |
| Sync merge from peer snapshots | Yune-defined local sync surface, no cloud feature | `sync.rs::sync_all_user_dicts`, `sync_user_dict`, `merge_record`, current backup after merge | `userdb_sync_merges_plain_snapshots_and_backs_up_current_state` | Tested; contract row needed to keep local-only scope clear. |
| User-dict iterator lists and caches `.userdb` entries | Librime-shaped levers ABI behavior with Yune-owned storage | `crates/yune-rime-api/src/userdb.rs` implements the user-dict levers exports; `api_table.rs` wires them into `RimeLeversApi`. `crates/yune-rime-api/src/levers.rs` owns custom settings, not these user-dict helpers. | `levers_user_dict_iterator_lists_userdb_entries` | Tested; Phase 3 ABI guard still needed for null/degenerate handles. |
| Levers user-dict backup/restore/export/import file operations | Librime-shaped ABI behavior with Yune-owned storage | `RimeLeversBackupUserDict`, `Restore`, `Export`, `Import` live in `src/userdb.rs`; file operations route through manager/sync modules | `levers_user_dict_file_operations_handle_plain_userdb_files` | Tested for happy path; Phase 3 abuse suite should cover invalid/null paths and Phase 2 should pin malformed import/snapshot rollback and export format. |
| Levers custom settings user-data files `*.custom.yaml` | Librime-shaped levers behavior; adjacent user-data lifecycle context, not user-dict storage | `crates/yune-rime-api/src/levers.rs` load/save/customize APIs | Existing levers custom-settings tests in `src/tests/levers.rs` | Tested separately; exclude from user-dict gaps except as frontend-visible user-data context. |
| Logical user-dict name validation | Yune-defined security behavior | `resource_id.rs::validate_user_dict_name` enforced in `userdb/mod.rs`, userdb APIs, snapshots | `userdb_rejects_malformed_logical_names_before_store_creation`; `resource_id.rs` tests | Tested; support contract should cite logical-id boundary. |

## Phase 2 Gap Summary

1. No direct process-restart/full-reinitialize userdb persistence test was found.
2. No explicit committed-store corrupt/truncated recovery test was found; current recovery is strongest for interrupted temp writes and snapshot restore.
3. Malformed import/snapshot rollback and multi-peer sync conflict/partial-failure behavior are not fully pinned.
4. The store/snapshot text format has no explicit format-version row or migration policy. M56 must freeze current format or record a named blocked migration row.
5. Learning/ranking behavior is mostly tested as Yune-defined behavior, not oracle-captured librime learning. The support contract must say which claims are Yune-defined unless new oracle captures are added.
6. Levers/file-operation invalid-path/null-pointer abuse belongs to Phase 3, but the behavior rows are user-data lifecycle rows and must be dispositioned there.

## Final M56 Disposition

| Gap | Final disposition |
| --- | --- |
| Process-restart/full-reinitialize persistence | Tested. `userdb_learning_persists_across_full_rime_reinitialize` records a learned entry, finalizes/reinitializes over the same user directory, and verifies the learned candidate remains visible. |
| Corrupt/truncated committed store recovery | Tested and contracted. Corrupt committed stores fail closed for learned data while ordinary table candidates remain usable. |
| Malformed import/snapshot rollback and partial sync failure | Tested and contracted. Malformed user-data inputs report failure without treating a partial operation as fully successful, and valid local data remains usable. |
| Store/snapshot format version and migration policy | Tested and contracted. The current text store/snapshot format is frozen; future format changes require explicit migration tests and contract updates. |
| Learning/ranking definition kind | Contracted as Yune-defined product behavior unless a future row adds an oracle fixture. |
| Levers/file-operation invalid/null abuse | Tested in the Phase 3 ABI abuse suite and covered by the guarded export inventory. |
