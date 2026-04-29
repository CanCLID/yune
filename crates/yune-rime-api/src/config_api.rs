use std::{
    ffi::{c_void, CStr, CString},
    os::raw::{c_char, c_int},
    ptr,
};

use serde_yaml::{Mapping, Value};

use crate::{
    bool_from, c_string_key, clear_schema_list, config_child_path, config_iterator_begin,
    config_lookup, config_lookup_key, config_scalar_bool, config_scalar_double, config_scalar_int,
    config_set, config_state_mut, config_string_value, copy_c_string_with_strncpy_semantics,
    find_config_value, free_schema_list_fields, librime_signature_modified_time,
    open_runtime_config, reset_config_iterator_for_begin,
    resource_id::{validate_config_api_resource_id, validate_schema_config_resource_id},
    runtime_paths, set_config_value, Bool, ConfigIteratorState, ConfigOpenKind, ConfigState,
    RimeConfig, RimeConfigIterator, RimeSchemaList, FALSE, RIME_VERSION_BYTES, TRUE,
};

/// Opens a deployed schema config from `<schema_id>.schema.yaml`.
///
/// # Safety
///
/// `schema_id` must be a valid NUL-terminated C string and `config` must point
/// to writable `RimeConfig` storage.
#[no_mangle]
pub unsafe extern "C" fn RimeSchemaOpen(schema_id: *const c_char, config: *mut RimeConfig) -> Bool {
    let Some(schema_id) = (unsafe { c_string_key(schema_id) }) else {
        return FALSE;
    };
    let Some(schema_id) = validate_schema_config_resource_id(&schema_id) else {
        return FALSE;
    };
    open_runtime_config(&schema_id, ConfigOpenKind::Deployed, config)
}

/// Opens a deployed config from `<config_id>.yaml`, checking staging before
/// prebuilt data.
///
/// # Safety
///
/// `config_id` must be a valid NUL-terminated C string and `config` must point
/// to writable `RimeConfig` storage.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigOpen(config_id: *const c_char, config: *mut RimeConfig) -> Bool {
    let Some(config_id) = (unsafe { c_string_key(config_id) }) else {
        return FALSE;
    };
    let Some(config_id) = validate_config_api_resource_id(&config_id) else {
        return FALSE;
    };
    open_runtime_config(&config_id, ConfigOpenKind::Deployed, config)
}

/// Opens a user-specific config from `<config_id>.yaml` in the user data dir.
///
/// # Safety
///
/// `config_id` must be a valid NUL-terminated C string and `config` must point
/// to writable `RimeConfig` storage.
#[no_mangle]
pub unsafe extern "C" fn RimeUserConfigOpen(
    config_id: *const c_char,
    config: *mut RimeConfig,
) -> Bool {
    let Some(config_id) = (unsafe { c_string_key(config_id) }) else {
        return FALSE;
    };
    open_runtime_config(&config_id, ConfigOpenKind::User, config)
}

/// Frees nested allocations populated by `RimeGetSchemaList`.
///
/// # Safety
///
/// `schema_list` must be either null or a valid pointer. Nested pointers, when
/// non-null, must have been returned by `RimeGetSchemaList` and not already
/// freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeSchemaList(schema_list: *mut RimeSchemaList) {
    if schema_list.is_null() {
        return;
    }

    free_schema_list_fields(schema_list);
    clear_schema_list(schema_list);
}

/// Initializes an empty in-memory config object.
///
/// # Safety
///
/// `config` must be either null or point to writable `RimeConfig` storage. The
/// caller owns the returned config and must release it with `RimeConfigClose`.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigInit(config: *mut RimeConfig) -> Bool {
    if config.is_null() {
        return FALSE;
    }
    // SAFETY: `config` is non-null and points to caller-owned storage.
    if unsafe { !(*config).ptr.is_null() } {
        return FALSE;
    }

    let state = Box::new(ConfigState::default());
    // SAFETY: `config` is non-null and writable.
    unsafe {
        (*config).ptr = Box::into_raw(state).cast::<c_void>();
    }
    TRUE
}

/// Loads YAML text into an in-memory config object.
///
/// # Safety
///
/// `config` must point to writable `RimeConfig` storage and `yaml` must be a
/// valid NUL-terminated C string. If `config` is uninitialized, it is
/// initialized before loading.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigLoadString(
    config: *mut RimeConfig,
    yaml: *const c_char,
) -> Bool {
    if config.is_null() || yaml.is_null() {
        return FALSE;
    }
    // SAFETY: `config` is non-null and writable.
    if unsafe { (*config).ptr.is_null() && RimeConfigInit(config) == FALSE } {
        return FALSE;
    }
    // SAFETY: `yaml` is non-null and caller promises a valid C string.
    let Ok(yaml) = unsafe { CStr::from_ptr(yaml) }.to_str() else {
        return FALSE;
    };
    let Ok(root) = serde_yaml::from_str::<Value>(yaml) else {
        return FALSE;
    };
    // SAFETY: `config` now owns a valid config state.
    let Some(state) = (unsafe { config_state_mut(config) }) else {
        return FALSE;
    };
    state.root = root;
    state.cstring_borrows.clear();
    TRUE
}

/// Releases an in-memory config object.
///
/// # Safety
///
/// `config`, when non-null, must point to a `RimeConfig` previously initialized
/// by this API and not already closed.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigClose(config: *mut RimeConfig) -> Bool {
    if config.is_null() {
        return FALSE;
    }
    // SAFETY: `config` is non-null and points to caller-owned storage.
    let ptr = unsafe { (*config).ptr };
    if ptr.is_null() {
        return FALSE;
    }
    // SAFETY: `ptr` was returned by `Box::into_raw` in `RimeConfigInit`.
    unsafe {
        drop(Box::from_raw(ptr.cast::<ConfigState>()));
        (*config).ptr = ptr::null_mut();
    }
    TRUE
}

/// Reads a boolean config value.
///
/// # Safety
///
/// `config`, `key`, and `value` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigGetBool(
    config: *mut RimeConfig,
    key: *const c_char,
    value: *mut Bool,
) -> Bool {
    if value.is_null() {
        return FALSE;
    }
    let Some(found) = (unsafe { config_lookup(config, key) }) else {
        return FALSE;
    };
    let Some(found) = config_scalar_bool(&found) else {
        return FALSE;
    };
    // SAFETY: `value` is non-null and caller promises writable storage.
    unsafe {
        *value = bool_from(found);
    }
    TRUE
}

/// Reads an integer config value.
///
/// # Safety
///
/// `config`, `key`, and `value` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigGetInt(
    config: *mut RimeConfig,
    key: *const c_char,
    value: *mut c_int,
) -> Bool {
    if value.is_null() {
        return FALSE;
    }
    let Some(found) = (unsafe { config_lookup(config, key) }) else {
        return FALSE;
    };
    let Some(found) = config_scalar_int(&found) else {
        return FALSE;
    };
    // SAFETY: `value` is non-null and caller promises writable storage.
    unsafe {
        *value = found;
    }
    TRUE
}

/// Reads a floating-point config value.
///
/// # Safety
///
/// `config`, `key`, and `value` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigGetDouble(
    config: *mut RimeConfig,
    key: *const c_char,
    value: *mut f64,
) -> Bool {
    if value.is_null() {
        return FALSE;
    }
    let Some(found) = (unsafe { config_lookup(config, key) }) else {
        return FALSE;
    };
    let Some(found) = config_scalar_double(&found) else {
        return FALSE;
    };
    // SAFETY: `value` is non-null and caller promises writable storage.
    unsafe {
        *value = found;
    }
    TRUE
}

/// Copies a string config value into caller-provided storage.
///
/// # Safety
///
/// `config`, `key`, and `value` must be valid pointers, and `value` must point
/// to writable storage of `buffer_size` bytes.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigGetString(
    config: *mut RimeConfig,
    key: *const c_char,
    value: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if value.is_null() {
        return FALSE;
    }
    let Some(found) = (unsafe { config_string_value(config, key) }) else {
        return FALSE;
    };
    copy_c_string_with_strncpy_semantics(&found, value, buffer_size);
    TRUE
}

/// Returns a borrowed string pointer cached on the config object.
///
/// # Safety
///
/// `config` and `key` must be valid pointers. The returned pointer remains
/// valid until the next mutable config operation or `RimeConfigClose`.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigGetCString(
    config: *mut RimeConfig,
    key: *const c_char,
) -> *const c_char {
    let Some(value) = (unsafe { config_string_value(config, key) }) else {
        return ptr::null();
    };
    let Ok(value) = CString::new(value) else {
        return ptr::null();
    };
    // SAFETY: `config` points to a valid config state.
    let Some(state) = (unsafe { config_state_mut(config) }) else {
        return ptr::null();
    };
    state.cstring_borrows.push(value);
    state
        .cstring_borrows
        .last()
        .map_or(ptr::null(), |value| value.as_ptr())
}

/// Updates a config signature block with librime-style deployment metadata.
///
/// # Safety
///
/// `config` must point to an initialized `RimeConfig`, and `signer` must be a
/// valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigUpdateSignature(
    config: *mut RimeConfig,
    signer: *const c_char,
) -> Bool {
    if signer.is_null() {
        return FALSE;
    }
    // SAFETY: `signer` is non-null and caller promises a valid C string.
    let signer = unsafe { CStr::from_ptr(signer) }
        .to_string_lossy()
        .into_owned();
    let Some(state) = (unsafe { config_state_mut(config) }) else {
        return FALSE;
    };

    let modified_time = librime_signature_modified_time();
    let rime_version =
        String::from_utf8_lossy(&RIME_VERSION_BYTES[..RIME_VERSION_BYTES.len() - 1]).into_owned();
    let (distribution_code_name, distribution_version) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            paths.distribution_code_name.to_string_lossy().into_owned(),
            paths.distribution_version.to_string_lossy().into_owned(),
        )
    };

    let updates = [
        ("signature/generator", signer),
        ("signature/modified_time", modified_time),
        ("signature/distribution_code_name", distribution_code_name),
        ("signature/distribution_version", distribution_version),
        ("signature/rime_version", rime_version),
    ];
    for (key, value) in updates {
        if !set_config_value(&mut state.root, key, Value::String(value)) {
            return FALSE;
        }
    }
    state.cstring_borrows.clear();
    TRUE
}

/// Writes a boolean config value.
///
/// # Safety
///
/// `config` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigSetBool(
    config: *mut RimeConfig,
    key: *const c_char,
    value: Bool,
) -> Bool {
    let value = if value != FALSE { "true" } else { "false" };
    unsafe { config_set(config, key, Value::String(value.to_owned())) }
}

/// Writes an integer config value.
///
/// # Safety
///
/// `config` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigSetInt(
    config: *mut RimeConfig,
    key: *const c_char,
    value: c_int,
) -> Bool {
    unsafe { config_set(config, key, Value::String(value.to_string())) }
}

/// Writes a floating-point config value.
///
/// # Safety
///
/// `config` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigSetDouble(
    config: *mut RimeConfig,
    key: *const c_char,
    value: f64,
) -> Bool {
    unsafe { config_set(config, key, Value::String(format!("{value:.6}"))) }
}

/// Writes a string config value.
///
/// # Safety
///
/// `config`, `key`, and `value` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigSetString(
    config: *mut RimeConfig,
    key: *const c_char,
    value: *const c_char,
) -> Bool {
    if value.is_null() {
        return FALSE;
    }
    // SAFETY: `value` is non-null and caller promises a valid C string.
    let value = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    unsafe { config_set(config, key, Value::String(value)) }
}

/// Copies a config subtree into another in-memory config object.
///
/// # Safety
///
/// `config`, `key`, and `value` must be valid pointers. If `value` is
/// uninitialized, it is initialized before receiving the copied item.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigGetItem(
    config: *mut RimeConfig,
    key: *const c_char,
    value: *mut RimeConfig,
) -> Bool {
    if value.is_null() {
        return FALSE;
    }
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };
    let Some(source) = (unsafe { config_state_mut(config) }) else {
        return FALSE;
    };
    let item = find_config_value(&source.root, &key)
        .cloned()
        .unwrap_or(Value::Null);
    // SAFETY: `value` is non-null and points to caller-owned storage.
    if unsafe { (*value).ptr.is_null() && RimeConfigInit(value) == FALSE } {
        return FALSE;
    }
    let Some(destination) = (unsafe { config_state_mut(value) }) else {
        return FALSE;
    };

    destination.root = item;
    destination.cstring_borrows.clear();
    TRUE
}

/// Writes a config subtree from another in-memory config object.
///
/// # Safety
///
/// `config` and `key` must be valid pointers. `value` may be null or
/// uninitialized, in which case a null item is written at `key`.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigSetItem(
    config: *mut RimeConfig,
    key: *const c_char,
    value: *mut RimeConfig,
) -> Bool {
    let item = if value.is_null() {
        Value::Null
    } else {
        // SAFETY: `value` is non-null. A null inner pointer represents a null
        // item for compatibility with librime's deprecated config API.
        match unsafe { config_state_mut(value) } {
            Some(value_state) => value_state.root.clone(),
            None => Value::Null,
        }
    };
    unsafe { config_set(config, key, item) }
}

/// Clears a config value by path.
///
/// # Safety
///
/// `config` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigClear(config: *mut RimeConfig, key: *const c_char) -> Bool {
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };
    let Some(state) = (unsafe { config_state_mut(config) }) else {
        return FALSE;
    };
    state.cstring_borrows.clear();
    bool_from(set_config_value(&mut state.root, &key, Value::Null))
}

/// Creates an empty list at a config path.
///
/// # Safety
///
/// `config` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigCreateList(config: *mut RimeConfig, key: *const c_char) -> Bool {
    unsafe { config_set(config, key, Value::Sequence(Vec::new())) }
}

/// Creates an empty map at a config path.
///
/// # Safety
///
/// `config` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigCreateMap(config: *mut RimeConfig, key: *const c_char) -> Bool {
    unsafe { config_set(config, key, Value::Mapping(Mapping::new())) }
}

/// Returns the size of a list at a config path.
///
/// # Safety
///
/// `config` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigListSize(config: *mut RimeConfig, key: *const c_char) -> usize {
    let Some(found) = (unsafe { config_lookup(config, key) }) else {
        return 0;
    };
    match found {
        Value::Sequence(sequence) => sequence.len(),
        _ => 0,
    }
}

/// Initializes an iterator over a config list.
///
/// # Safety
///
/// `iterator`, `config`, and `key` must be valid pointers. The iterator must be
/// released with `RimeConfigEnd` after a successful begin call.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigBeginList(
    iterator: *mut RimeConfigIterator,
    config: *mut RimeConfig,
    key: *const c_char,
) -> Bool {
    if iterator.is_null() || config.is_null() || key.is_null() {
        return FALSE;
    }
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };
    // librime clears caller-visible iterator state before attempting lookup, so
    // stale fields are not left behind when the requested path is not a list.
    unsafe { reset_config_iterator_for_begin(iterator) };
    let Some(found) = (unsafe { config_lookup_key(config, &key) }) else {
        return FALSE;
    };
    let Value::Sequence(sequence) = found else {
        return FALSE;
    };

    let entries = sequence
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let entry_key = format!("@{index}");
            let path = config_child_path(&key, &entry_key);
            (entry_key, path)
        })
        .collect::<Vec<_>>();
    unsafe { config_iterator_begin(iterator, entries, true) }
}

/// Initializes an iterator over a config map.
///
/// # Safety
///
/// `iterator`, `config`, and `key` must be valid pointers. The iterator must be
/// released with `RimeConfigEnd` after a successful begin call.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigBeginMap(
    iterator: *mut RimeConfigIterator,
    config: *mut RimeConfig,
    key: *const c_char,
) -> Bool {
    if iterator.is_null() || config.is_null() || key.is_null() {
        return FALSE;
    }
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };
    // Match librime's begin behavior: a failed map lookup still resets the
    // iterator object after the basic pointer checks pass.
    unsafe { reset_config_iterator_for_begin(iterator) };
    let Some(found) = (unsafe { config_lookup_key(config, &key) }) else {
        return FALSE;
    };
    let Value::Mapping(mapping) = found else {
        return FALSE;
    };

    let mut entries = mapping
        .iter()
        .filter_map(|(entry_key, _)| match entry_key {
            Value::String(entry_key) => {
                let path = config_child_path(&key, entry_key);
                Some((entry_key.clone(), path))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    // librime stores config maps in std::map, so public map iteration is
    // lexical by key rather than YAML insertion order.
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    unsafe { config_iterator_begin(iterator, entries, false) }
}

/// Advances a config iterator and exposes its current key and full path.
///
/// # Safety
///
/// `iterator` must be a valid pointer previously initialized by
/// `RimeConfigBeginList` or `RimeConfigBeginMap`.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigNext(iterator: *mut RimeConfigIterator) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }

    // SAFETY: callers promise `iterator` is valid; begin stores exactly one of
    // these pointers when initialization succeeds.
    let state_ptr = unsafe {
        if !(*iterator).list.is_null() {
            (*iterator).list
        } else {
            (*iterator).map
        }
    };
    if state_ptr.is_null() {
        return FALSE;
    }

    // SAFETY: non-null iterator state pointers are created by
    // `config_iterator_begin`.
    let state = unsafe { &mut *state_ptr.cast::<ConfigIteratorState>() };
    // SAFETY: `iterator` is non-null and points to writable storage.
    let next_index = unsafe { (*iterator).index.saturating_add(1) };
    if next_index < 0 {
        return FALSE;
    }
    let Some((key, path)) = state.entries.get(next_index as usize) else {
        // librime increments the public iterator index before checking for
        // exhaustion, so failed end-of-container calls expose the advanced
        // value.
        unsafe {
            (*iterator).index = next_index;
        }
        return FALSE;
    };
    let Ok(key_cache) = CString::new(key.as_str()) else {
        return FALSE;
    };
    let Ok(path_cache) = CString::new(path.as_str()) else {
        return FALSE;
    };
    state.key_cache = Some(key_cache);
    state.path_cache = Some(path_cache);

    // SAFETY: cache pointers remain valid until the next iterator mutation or
    // `RimeConfigEnd`.
    unsafe {
        (*iterator).index = next_index;
        (*iterator).key = state
            .key_cache
            .as_ref()
            .map_or(ptr::null(), |value| value.as_ptr());
        (*iterator).path = state
            .path_cache
            .as_ref()
            .map_or(ptr::null(), |value| value.as_ptr());
    }
    TRUE
}

/// Releases a config iterator initialized by this API.
///
/// # Safety
///
/// `iterator` must be either null or a valid iterator object. Non-null nested
/// state pointers must have been returned by this API.
#[no_mangle]
pub unsafe extern "C" fn RimeConfigEnd(iterator: *mut RimeConfigIterator) {
    if iterator.is_null() {
        return;
    }
    // SAFETY: `iterator` is non-null and any state pointers were allocated by
    // `config_iterator_begin`.
    unsafe {
        if !(*iterator).list.is_null() {
            drop(Box::from_raw(
                (*iterator).list.cast::<ConfigIteratorState>(),
            ));
        }
        if !(*iterator).map.is_null() {
            drop(Box::from_raw((*iterator).map.cast::<ConfigIteratorState>()));
        }
        *iterator = RimeConfigIterator {
            list: ptr::null_mut(),
            map: ptr::null_mut(),
            index: 0,
            key: ptr::null(),
            path: ptr::null(),
        };
    }
}
