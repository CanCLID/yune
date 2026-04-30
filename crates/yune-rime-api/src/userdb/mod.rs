pub(crate) mod file_store;
pub(crate) mod record;
pub(crate) mod recovery;
pub(crate) mod snapshot;
pub(crate) mod store;
pub(crate) mod sync;

use std::{fs, io, os::raw::c_int, path::PathBuf};

use crate::{resource_id::validate_user_dict_name, runtime_paths};

use self::{
    file_store::FileUserDbStore,
    record::{UserDbRecord, UserDbValue},
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
    sync::restore_snapshot(&snapshot.to_path_buf()).is_ok()
}

pub(crate) fn export_user_dict(dict_name: &str, destination: PathBuf) -> c_int {
    if destination.as_os_str().is_empty() {
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
    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            return -1;
        }
    }
    if fs::write(destination, output).is_err() {
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

pub(crate) fn sync_all_user_dicts() -> bool {
    sync::sync_all_user_dicts()
}

pub(crate) fn user_dict_upgrade() -> bool {
    deployed_user_dict_names()
        .iter()
        .all(|dict_name| recovery::recover_user_dict(dict_name, None))
}

pub(crate) fn record_from_key_value(key: &str, value: UserDbValue) -> io::Result<UserDbRecord> {
    UserDbRecord::from_key_value(key, value)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid userdb record"))
}
