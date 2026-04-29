use std::{
    collections::HashSet,
    ffi::c_void,
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
};

use crate::{
    bool_from, clear_user_dict_iterator, cstring_from_lossless_str, optional_c_string,
    resource_id::validate_user_dict_name, runtime_paths, Bool, RimeUserDictIterator,
    UserDictListState, FALSE, TRUE,
};

/// Initializes an iterator over user dictionary names found in `user_data_dir`.
///
/// # Safety
///
/// `iterator` must be null or point to writable `RimeUserDictIterator` storage.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversUserDictIteratorInit(
    iterator: *mut RimeUserDictIterator,
) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }

    let names = deployed_user_dict_names()
        .into_iter()
        .map(|name| cstring_from_lossless_str(&name))
        .collect::<Vec<_>>();
    if names.is_empty() {
        return FALSE;
    }

    // SAFETY: `iterator` is non-null and owned by the caller; if it already
    // holds state from this shim, release it before replacing it. librime does
    // not touch an existing iterator when a new scan finds no dictionaries.
    unsafe { clear_user_dict_iterator(iterator) };

    let state = Box::into_raw(Box::new(UserDictListState { names })).cast::<c_void>();
    // SAFETY: `iterator` is non-null and points to writable storage.
    unsafe {
        (*iterator).ptr = state;
        (*iterator).i = 0;
    }
    TRUE
}

/// Releases a user dictionary iterator initialized by the levers API.
///
/// # Safety
///
/// `iterator` must be null or point to `RimeUserDictIterator` storage.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversUserDictIteratorDestroy(iterator: *mut RimeUserDictIterator) {
    if iterator.is_null() {
        return;
    }
    // SAFETY: ownership rules match `RimeLeversUserDictIteratorInit`.
    unsafe { clear_user_dict_iterator(iterator) };
}

/// Returns the next user dictionary name from an initialized iterator.
///
/// # Safety
///
/// `iterator` must be null or point to a `RimeUserDictIterator` initialized by
/// `RimeLeversUserDictIteratorInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversNextUserDict(
    iterator: *mut RimeUserDictIterator,
) -> *const c_char {
    if iterator.is_null() {
        return ptr::null();
    }

    // SAFETY: `iterator` is non-null and points to caller-owned storage.
    let state_ptr = unsafe { (*iterator).ptr };
    if state_ptr.is_null() {
        return ptr::null();
    }
    // SAFETY: non-null iterator state pointers are allocated by this shim.
    let state = unsafe { &*state_ptr.cast::<UserDictListState>() };
    // SAFETY: `iterator` is non-null and readable.
    let index = unsafe { (*iterator).i };
    let Some(name) = state.names.get(index) else {
        return ptr::null();
    };
    // SAFETY: `iterator` is non-null and writable.
    unsafe {
        (*iterator).i = (*iterator).i.saturating_add(1);
    }
    name.as_ptr()
}

/// Backs up a plain file-backed user dictionary into the user sync directory.
///
/// # Safety
///
/// `dict_name` must be null or point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversBackupUserDict(dict_name: *const c_char) -> Bool {
    let Some(dict_name) = optional_c_string(dict_name) else {
        return FALSE;
    };
    bool_from(backup_plain_user_dict(&dict_name))
}

/// Restores a plain user dictionary snapshot into the user data directory.
///
/// # Safety
///
/// `snapshot_file` must be null or point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversRestoreUserDict(snapshot_file: *const c_char) -> Bool {
    let Some(snapshot_file) = optional_c_string(snapshot_file) else {
        return FALSE;
    };
    let snapshot = PathBuf::from(snapshot_file);
    if !snapshot.is_file() {
        return FALSE;
    }
    let Some(dict_name) =
        snapshot_dict_name(&snapshot).and_then(|name| validate_user_dict_name(&name))
    else {
        return FALSE;
    };
    let Some(destination) = user_dict_path(&dict_name) else {
        return FALSE;
    };
    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            return FALSE;
        }
    }
    bool_from(fs::copy(snapshot, destination).is_ok())
}

/// Exports a plain file-backed user dictionary to a text file.
///
/// # Safety
///
/// `dict_name` and `text_file` must be null or point to valid NUL-terminated C
/// strings.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversExportUserDict(
    dict_name: *const c_char,
    text_file: *const c_char,
) -> c_int {
    let Some(dict_name) = optional_c_string(dict_name) else {
        return -1;
    };
    let Some(text_file) = optional_c_string(text_file) else {
        return -1;
    };
    if text_file.is_empty() {
        return -1;
    }

    let Some(source) = user_dict_path(&dict_name) else {
        return -1;
    };
    if !source.is_file() {
        return -1;
    }
    let Ok(entry_count) = count_text_user_dict_entries(&source) else {
        return -1;
    };
    let destination = PathBuf::from(text_file);
    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            return -1;
        }
    }
    if fs::copy(source, destination).is_err() {
        return -1;
    }
    entry_count
}

/// Imports a text file as a plain file-backed user dictionary.
///
/// # Safety
///
/// `dict_name` and `text_file` must be null or point to valid NUL-terminated C
/// strings.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversImportUserDict(
    dict_name: *const c_char,
    text_file: *const c_char,
) -> c_int {
    let Some(dict_name) = optional_c_string(dict_name) else {
        return -1;
    };
    let Some(text_file) = optional_c_string(text_file) else {
        return -1;
    };
    if text_file.is_empty() {
        return -1;
    }

    let source = PathBuf::from(text_file);
    if !source.is_file() {
        return -1;
    }
    let Ok(entry_count) = count_text_user_dict_entries(&source) else {
        return -1;
    };
    let Some(destination) = user_dict_path(&dict_name) else {
        return -1;
    };
    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            return -1;
        }
    }
    if fs::copy(source, destination).is_err() {
        return -1;
    }
    entry_count
}

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
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn runtime_user_data_dir() -> PathBuf {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned())
}

pub(crate) fn runtime_user_data_sync_dir() -> PathBuf {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    PathBuf::from(paths.user_data_sync_dir.to_string_lossy().into_owned())
}

fn user_dict_path(dict_name: &str) -> Option<PathBuf> {
    let dict_name = validate_user_dict_name(dict_name)?;
    Some(runtime_user_data_dir().join(format!("{dict_name}.userdb")))
}

fn user_dict_snapshot_path(dict_name: &str) -> Option<PathBuf> {
    let dict_name = validate_user_dict_name(dict_name)?;
    Some(runtime_user_data_sync_dir().join(format!("{dict_name}.userdb.txt")))
}

pub(crate) fn backup_plain_user_dict(dict_name: &str) -> bool {
    let Some(source) = user_dict_path(dict_name) else {
        return false;
    };
    if !source.is_file() {
        return false;
    }
    let Some(snapshot) = user_dict_snapshot_path(dict_name) else {
        return false;
    };
    if let Some(parent) = snapshot.parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    fs::copy(source, snapshot).is_ok()
}

pub(crate) fn sync_all_user_dicts() -> bool {
    let mut success = true;
    for dict_name in deployed_user_dict_names() {
        if !sync_plain_user_dict(&dict_name) {
            success = false;
        }
    }
    success
}

pub(crate) fn user_dict_upgrade() -> bool {
    true
}

fn sync_plain_user_dict(dict_name: &str) -> bool {
    if validate_user_dict_name(dict_name).is_none() {
        return false;
    }
    let mut success = true;
    for snapshot in peer_user_dict_snapshots(dict_name) {
        if merge_plain_user_dict_snapshot(dict_name, &snapshot).is_err() {
            success = false;
        }
    }
    backup_plain_user_dict(dict_name) && success
}

fn peer_user_dict_snapshots(dict_name: &str) -> Vec<PathBuf> {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    let sync_dir = PathBuf::from(paths.sync_dir.to_string_lossy().into_owned());
    drop(paths);

    let Ok(entries) = fs::read_dir(sync_dir) else {
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

fn merge_plain_user_dict_snapshot(dict_name: &str, snapshot: &Path) -> Result<(), std::io::Error> {
    let Some(destination) = user_dict_path(dict_name) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid user dictionary name",
        ));
    };
    if !destination.is_file() {
        fs::copy(snapshot, destination)?;
        return Ok(());
    }

    let destination_text = fs::read_to_string(&destination)?;
    let snapshot_text = fs::read_to_string(snapshot)?;
    let mut seen = destination_text
        .lines()
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();
    let mut merged = destination_text;
    for line in snapshot_text.lines() {
        if line.trim().is_empty() || !seen.insert(line.to_owned()) {
            continue;
        }
        if !merged.is_empty() && !merged.ends_with('\n') {
            merged.push('\n');
        }
        merged.push_str(line);
        merged.push('\n');
    }
    fs::write(destination, merged)
}

fn snapshot_dict_name(snapshot_file: &Path) -> Option<String> {
    snapshot_file
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .and_then(|file_name| file_name.strip_suffix(".userdb.txt"))
        .filter(|dict_name| !dict_name.is_empty())
        .map(ToOwned::to_owned)
}

fn count_text_user_dict_entries(path: &Path) -> Result<c_int, std::io::Error> {
    let contents = fs::read_to_string(path)?;
    Ok(contents
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.is_empty() && !line.starts_with('#')
        })
        .count()
        .try_into()
        .unwrap_or(c_int::MAX))
}
