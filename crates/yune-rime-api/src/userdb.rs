#[path = "userdb/mod.rs"]
mod manager;

use std::{
    ffi::c_void,
    io,
    os::raw::{c_char, c_int},
    path::PathBuf,
    ptr,
};

use yune_core::{UserDb, UserDbCommitMetadata};

use crate::{
    bool_from, clear_user_dict_iterator, cstring_from_lossless_str, optional_c_string, Bool,
    RimeUserDictIterator, UserDictListState, FALSE, TRUE,
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

    let names = manager::deployed_user_dict_names()
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

/// Backs up a file-backed user dictionary into the user sync directory.
///
/// # Safety
///
/// `dict_name` must be null or point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversBackupUserDict(dict_name: *const c_char) -> Bool {
    let Some(dict_name) = optional_c_string(dict_name) else {
        return FALSE;
    };
    bool_from(manager::backup_user_dict(&dict_name))
}

/// Restores a user dictionary snapshot into the user data directory.
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
    bool_from(manager::restore_user_dict_snapshot(&snapshot))
}

/// Exports a file-backed user dictionary to a text file.
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

    manager::export_user_dict(&dict_name, PathBuf::from(text_file))
}

/// Imports a text file as a file-backed user dictionary.
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

    manager::import_user_dict(&dict_name, PathBuf::from(text_file))
}

pub(crate) fn runtime_user_data_sync_dir() -> PathBuf {
    manager::runtime_user_data_sync_dir()
}

pub(crate) fn load_runtime_userdb(dict_name: &str) -> io::Result<UserDb> {
    manager::load_runtime_userdb(dict_name)
}

pub(crate) fn record_runtime_commit(
    dict_name: &str,
    event: &UserDbCommitMetadata,
) -> io::Result<UserDb> {
    manager::record_runtime_commit(dict_name, event)
}

pub(crate) fn sync_all_user_dicts() -> bool {
    manager::sync_all_user_dicts()
}

pub(crate) fn user_dict_upgrade() -> bool {
    manager::user_dict_upgrade()
}
