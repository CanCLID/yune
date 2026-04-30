pub(crate) mod file_store;
pub(crate) mod record;
pub(crate) mod recovery;
pub(crate) mod snapshot;
pub(crate) mod store;
pub(crate) mod sync;

use std::{fs, io, os::raw::c_int, path::PathBuf};

use yune_core::{UserDb, UserDbCommitMetadata};

use crate::{resource_id::validate_user_dict_name, runtime_paths};

use self::{
    file_store::FileUserDbStore,
    record::{formula_d, UserDbRecord, UserDbValue},
    store::UserDbStore,
};

pub(crate) fn deployed_user_dict_names() -> Vec<String> {
    let user_data_dir = runtime_user_data_dir();
    let Ok(entries) = fs::read_dir(user_data_dir) else {
        return Vec::new();
    };

    let mut names = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .strip_suffix(".userdb")
                .filter(|name| validate_user_dict_name(name).is_some())
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

pub(crate) fn backup_user_dict(dict_name: &str) -> bool {
    let Ok(store) = open_store(dict_name) else {
        return false;
    };
    if !store.path().is_file() {
        return false;
    }
    let Some(snapshot) = user_dict_snapshot_path(dict_name) else {
        return false;
    };
    snapshot::write_snapshot(&store, &snapshot).is_ok()
}

pub(crate) fn restore_user_dict_snapshot(snapshot: &std::path::Path) -> bool {
    sync::restore_snapshot(snapshot).is_ok()
}

pub(crate) fn export_user_dict(dict_name: &str, export_destination: PathBuf) -> c_int {
    if export_destination.as_os_str().is_empty() {
        return -1;
    }
    let Ok(store) = open_store(dict_name) else {
        return -1;
    };
    if !store.path().is_file() || !store.validate() {
        return -1;
    }
    let mut output = String::new();
    let mut count: c_int = 0;
    for record in store.ordered_records() {
        if let Some(row) = record.to_table_row() {
            output.push_str(&row);
            count = count.saturating_add(1);
        }
    }
    if let Some(parent) = export_destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            return -1;
        }
    }
    if fs::write(export_destination, output).is_err() {
        return -1;
    }
    count
}

pub(crate) fn import_user_dict(dict_name: &str, source: PathBuf) -> c_int {
    if !source.is_file() {
        return -1;
    }
    let Ok(text) = fs::read_to_string(source) else {
        return -1;
    };
    let records = text
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.is_empty() && !line.starts_with('#')
        })
        .map(UserDbRecord::from_table_row)
        .collect::<Result<Vec<_>, _>>();
    let Ok(records) = records else {
        return -1;
    };
    let Ok(mut store) = open_store(dict_name) else {
        return -1;
    };
    if !store.begin_transaction() {
        return -1;
    }
    for record in records {
        store.update(record);
    }
    store.set_tick_to_record_max();
    if store.commit_transaction().is_err() {
        return -1;
    }
    store
        .ordered_records()
        .len()
        .try_into()
        .unwrap_or(c_int::MAX)
}

pub(crate) fn runtime_user_data_sync_dir() -> PathBuf {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    PathBuf::from(paths.user_data_sync_dir.to_string_lossy().into_owned())
}

pub(crate) fn runtime_user_data_dir() -> PathBuf {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned())
}

pub(crate) fn runtime_sync_dir() -> PathBuf {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    PathBuf::from(paths.sync_dir.to_string_lossy().into_owned())
}

pub(crate) fn runtime_user_id() -> String {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    paths.user_id.to_string_lossy().into_owned()
}

pub(crate) fn user_dict_path(dict_name: &str) -> Option<PathBuf> {
    let dict_name = validate_user_dict_name(dict_name)?;
    Some(runtime_user_data_dir().join(format!("{dict_name}.userdb")))
}

pub(crate) fn user_dict_snapshot_path(dict_name: &str) -> Option<PathBuf> {
    let dict_name = validate_user_dict_name(dict_name)?;
    Some(runtime_user_data_sync_dir().join(format!("{dict_name}.userdb.txt")))
}

pub(crate) fn open_store(dict_name: &str) -> io::Result<FileUserDbStore> {
    let Some(dict_name) = validate_user_dict_name(dict_name) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid user dictionary name",
        ));
    };
    let Some(path) = user_dict_path(&dict_name) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid user dictionary path",
        ));
    };
    let user_id = runtime_user_id();
    FileUserDbStore::open(path, dict_name, user_id)
}

pub(crate) fn load_runtime_userdb(dict_name: &str) -> io::Result<UserDb> {
    let store = open_store(dict_name)?;
    let mut userdb = UserDb::default();
    for record in store.ordered_records() {
        userdb.learn_entry(
            record.code.trim_end(),
            record.phrase,
            record.value.commits,
            record.value.dee,
            record.value.tick,
        );
    }
    Ok(userdb)
}

pub(crate) fn record_runtime_commit(
    dict_name: &str,
    event: &UserDbCommitMetadata,
) -> io::Result<UserDb> {
    let mut store = open_store(dict_name)?;
    let Some(record) = updated_record_for_commit(&store, event) else {
        return load_runtime_userdb(dict_name);
    };
    if !store.begin_transaction() {
        return Err(io::Error::other("userdb transaction already active"));
    }
    store.update(record);
    if let Err(error) = store.commit_transaction() {
        let _ = store.rollback();
        return Err(error);
    }
    load_runtime_userdb(dict_name)
}

fn updated_record_for_commit(
    store: &FileUserDbStore,
    event: &UserDbCommitMetadata,
) -> Option<UserDbRecord> {
    let mut value = store
        .get(&record_key(&event.input, &event.selected_text)?)
        .unwrap_or_default();
    if value.commits < 0 {
        value.commits = -value.commits;
    }
    value.commits = value.commits.saturating_add(1);
    let next_tick = store
        .metadata()
        .tick
        .max(value.tick)
        .max(event.tick)
        .saturating_add(1);
    value.dee = formula_d(1.0, next_tick as f64, value.dee, value.tick as f64);
    value.tick = next_tick;
    UserDbRecord::from_code_phrase(&event.input, &event.selected_text, value)
}

fn record_key(code: &str, phrase: &str) -> Option<String> {
    UserDbRecord::from_code_phrase(code, phrase, UserDbValue::default()).map(|record| record.key)
}

pub(crate) fn sync_all_user_dicts() -> bool {
    sync::sync_all_user_dicts()
}

pub(crate) fn user_dict_upgrade() -> bool {
    deployed_user_dict_names()
        .iter()
        .all(|dict_name| recovery::recover_user_dict(dict_name, None))
}
