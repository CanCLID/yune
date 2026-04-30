use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{
    backup_user_dict, open_store,
    record::{formula_d, UserDbRecord},
    runtime_sync_dir,
    snapshot::read_snapshot,
    store::UserDbStore,
};
use crate::resource_id::validate_user_dict_name;

pub(crate) fn sync_all_user_dicts() -> bool {
    let mut success = true;
    for dict_name in super::deployed_user_dict_names() {
        if !sync_user_dict(&dict_name) {
            success = false;
        }
    }
    success
}

pub(crate) fn sync_user_dict(dict_name: &str) -> bool {
    if validate_user_dict_name(dict_name).is_none() {
        return false;
    }
    let mut success = true;
    for snapshot in peer_user_dict_snapshots(dict_name) {
        if restore_snapshot(&snapshot).is_err() {
            success = false;
        }
    }
    backup_user_dict(dict_name) && success
}

pub(crate) fn restore_snapshot(snapshot: &Path) -> std::io::Result<()> {
    let (metadata, records) = read_snapshot(snapshot)?;
    let mut store = open_store(&metadata.db_name)?;
    let our_tick = store.metadata().tick;
    let their_tick = metadata.tick;
    let max_tick = our_tick.max(their_tick);
    if !store.begin_transaction() {
        return Err(std::io::Error::other("transaction already active"));
    }
    for remote in records {
        let merged = merge_record(&store, remote, our_tick, their_tick, max_tick);
        store.update(merged);
    }
    let mut next_metadata = store.metadata().clone();
    next_metadata.tick = max_tick;
    store.update_metadata(next_metadata);
    store.commit_transaction()
}

fn merge_record(
    store: &impl UserDbStore,
    mut remote: UserDbRecord,
    our_tick: u64,
    _their_tick: u64,
    max_tick: u64,
) -> UserDbRecord {
    let mut merged = store.get(&remote.key).unwrap_or_default();
    if merged.tick < our_tick {
        merged.dee = formula_d(0.0, our_tick as f64, merged.dee, merged.tick as f64);
    }
    if merged.commits.abs() < remote.value.commits.abs() {
        merged.commits = remote.value.commits;
    }
    merged.dee = merged.dee.max(remote.value.dee);
    merged.tick = max_tick;
    remote.value = merged;
    remote
}

fn peer_user_dict_snapshots(dict_name: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(runtime_sync_dir()) else {
        return Vec::new();
    };
    let snapshot_name = format!("{dict_name}.userdb.txt");
    let mut snapshots = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| path.join(&snapshot_name))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    snapshots.sort();
    snapshots
}
