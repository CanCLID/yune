---
status: issues_found
phase: 05-userdb-and-scaling-hardening
reviewed: 2026-04-30T00:00:00Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - crates/yune-core/src/engine.rs
  - crates/yune-core/src/lib.rs
  - crates/yune-core/src/state.rs
  - crates/yune-core/src/userdb.rs
  - crates/yune-rime-api/src/lib.rs
  - crates/yune-rime-api/src/schema_install.rs
  - crates/yune-rime-api/src/schema_selection.rs
  - crates/yune-rime-api/src/session.rs
  - crates/yune-rime-api/src/userdb.rs
  - crates/yune-rime-api/src/userdb/file_store.rs
  - crates/yune-rime-api/src/userdb/mod.rs
  - crates/yune-rime-api/src/userdb/record.rs
  - crates/yune-rime-api/src/userdb/recovery.rs
  - crates/yune-rime-api/src/userdb/snapshot.rs
  - crates/yune-rime-api/src/userdb/store.rs
  - crates/yune-rime-api/src/userdb/sync.rs
findings:
  total: 2
  critical: 1
  warning: 1
  info: 0
---

# Phase 05: Code Review Report

**Reviewed:** 2026-04-30T00:00:00Z
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Reviewed the Phase 05 userdb and scaling hardening changes across `yune-core` and the RIME API shim. The implementation adds in-memory userdb learning, file-backed `.userdb` persistence, levers import/export/backup/restore, schema-driven user dictionary selection, and sync snapshot merge handling.

Two production correctness issues were found. One is a process-crashing Unicode caret bug in the ABI-reachable engine editing path. The other corrupts local sync identity by copying peer snapshot metadata into the local user dictionary store.

## Critical Issues

### CR-01: Non-ASCII input can panic through byte-index caret movement

**File:** `crates/yune-core/src/engine.rs:537-652`

**Issue:** `Composition::caret` is stored as a byte offset, but `set_caret_pos`, `move_caret_left`, and `move_caret_right` can place it on a non-UTF-8 character boundary. `backspace` and `delete_at_caret` then pass that offset to `String::remove`, which panics unless the index is a character boundary. These paths are reachable from the C ABI through `RimeSetInput`, `RimeSetCaretPos`, `RimeProcessKey(... XK_LEFT ...)`, Backspace, and Delete. A frontend can therefore crash the process with valid UTF-8 input such as `é` or `你` by moving or setting the caret into the middle of the multi-byte character before deleting.

**Fix:** Clamp externally supplied and internally moved caret positions to valid UTF-8 boundaries, and make deletion step by character boundaries rather than raw bytes. For example:

```rust
fn previous_char_boundary(input: &str, caret: usize) -> Option<usize> {
    input[..caret].char_indices().last().map(|(index, _)| index)
}

fn next_char_boundary(input: &str, caret: usize) -> Option<usize> {
    input[caret..]
        .chars()
        .next()
        .map(|ch| caret + ch.len_utf8())
}

fn clamp_to_char_boundary(input: &str, caret: usize) -> usize {
    let mut caret = caret.min(input.len());
    while caret > 0 && !input.is_char_boundary(caret) {
        caret -= 1;
    }
    caret
}

pub fn set_caret_pos(&mut self, caret_pos: usize) {
    self.context.composition.caret = clamp_to_char_boundary(
        &self.context.composition.input,
        caret_pos,
    );
}

pub fn move_caret_left(&mut self) -> bool {
    let Some(previous) = previous_char_boundary(
        &self.context.composition.input,
        self.context.composition.caret,
    ) else {
        return false;
    };
    self.context.composition.caret = previous;
    true
}

pub fn move_caret_right(&mut self) -> bool {
    let Some(next) = next_char_boundary(
        &self.context.composition.input,
        self.context.composition.caret,
    ) else {
        return false;
    };
    self.context.composition.caret = next;
    true
}

fn backspace(&mut self) -> Option<String> {
    let Some(remove_at) = previous_char_boundary(
        &self.context.composition.input,
        self.context.composition.caret,
    ) else {
        return None;
    };
    self.context.composition.input.remove(remove_at);
    self.context.composition.caret = remove_at;
    self.context.composition.preedit = self.context.composition.input.clone();
    self.refresh_candidates();
    None
}

fn delete_at_caret(&mut self) -> Option<String> {
    let caret = clamp_to_char_boundary(
        &self.context.composition.input,
        self.context.composition.caret,
    );
    if caret < self.context.composition.input.len() {
        self.context.composition.caret = caret;
        self.context.composition.input.remove(caret);
        self.context.composition.preedit = self.context.composition.input.clone();
        self.refresh_candidates();
    }
    None
}
```

Add regression coverage for `RimeSetInput`/`RimeSetCaretPos` plus Backspace/Delete on multi-byte UTF-8 input.

## Warnings

### WR-01: Sync restore overwrites local user identity with peer snapshot identity

**File:** `crates/yune-rime-api/src/userdb/sync.rs:50-55`

**Issue:** `restore_snapshot` merges a peer snapshot into the local store, then copies `metadata.user_id` from that peer snapshot into the local store metadata. During `RimeSyncUserData`, `sync_user_dict` restores peer snapshots and then calls `backup_user_dict`; because the local store now carries the peer `user_id`, the newly written local snapshot can advertise the peer identity instead of the current installation. This corrupts sync metadata and can make future backups indistinguishable from the remote source.

**Fix:** Preserve the local store's `user_id` when merging peer snapshots. If an existing store has an empty/unknown identity, initialize it from the runtime installation id rather than from peer metadata.

```rust
pub(crate) fn restore_snapshot(snapshot: &Path) -> std::io::Result<()> {
    let (metadata, records) = read_snapshot(snapshot)?;
    let mut store = open_store(&metadata.db_name)?;
    let our_tick = store.metadata().tick;
    let max_tick = our_tick.max(metadata.tick);
    if !store.begin_transaction() {
        return Err(std::io::Error::other("transaction already active"));
    }
    for remote in records {
        let merged = merge_record(&store, remote, our_tick, metadata.tick, max_tick);
        store.update(merged);
    }
    let mut next_metadata = store.metadata().clone();
    next_metadata.tick = max_tick;
    // Do not copy metadata.user_id from the peer snapshot.
    store.update_metadata(next_metadata);
    store.commit_transaction()
}
```

Add a sync test assertion that the local backup contains the current `installation_id` in `/user_id` after merging a peer snapshot whose `/user_id` differs.

---

_Reviewed: 2026-04-30T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
