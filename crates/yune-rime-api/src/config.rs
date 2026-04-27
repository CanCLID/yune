use std::{
    ffi::{c_void, CStr, CString},
    os::raw::{c_char, c_int},
    ptr,
};

use serde_yaml::{Mapping, Number, Value};

use crate::{bool_from, Bool, RimeConfig, RimeConfigIterator, FALSE, TRUE};

pub(crate) struct ConfigState {
    pub(crate) root: Value,
    pub(crate) cstring_borrows: Vec<CString>,
}

pub(crate) struct ConfigIteratorState {
    pub(crate) entries: Vec<(String, String)>,
    pub(crate) key_cache: Option<CString>,
    pub(crate) path_cache: Option<CString>,
}
impl Default for ConfigState {
    fn default() -> Self {
        Self {
            root: Value::Mapping(Mapping::new()),
            cstring_borrows: Vec::new(),
        }
    }
}

pub(crate) unsafe fn config_state_mut(config: *mut RimeConfig) -> Option<&'static mut ConfigState> {
    if config.is_null() {
        return None;
    }
    // SAFETY: callers promise `config` points to valid RimeConfig storage.
    let ptr = unsafe { (*config).ptr };
    if ptr.is_null() {
        return None;
    }
    // SAFETY: non-null config pointers are created by `RimeConfigInit`.
    Some(unsafe { &mut *ptr.cast::<ConfigState>() })
}

pub(crate) unsafe fn config_lookup(config: *mut RimeConfig, key: *const c_char) -> Option<Value> {
    let key = unsafe { c_string_key(key) }?;
    unsafe { config_lookup_key(config, &key) }
}

pub(crate) unsafe fn config_lookup_key(config: *mut RimeConfig, key: &str) -> Option<Value> {
    let state = unsafe { config_state_mut(config) }?;
    find_config_value(&state.root, key).cloned()
}

pub(crate) unsafe fn config_string_value(
    config: *mut RimeConfig,
    key: *const c_char,
) -> Option<String> {
    config_scalar_string(&unsafe { config_lookup(config, key) }?)
}

pub(crate) unsafe fn config_set(config: *mut RimeConfig, key: *const c_char, value: Value) -> Bool {
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };
    let Some(state) = (unsafe { config_state_mut(config) }) else {
        return FALSE;
    };
    state.cstring_borrows.clear();
    bool_from(set_config_value(&mut state.root, &key, value))
}

pub(crate) fn config_scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(if *value { "true" } else { "false" }.to_owned()),
        Value::Number(value) => Some(config_number_string(value)),
        _ => None,
    }
}

pub(crate) fn config_scalar_bool(value: &Value) -> Option<bool> {
    match value {
        Value::Bool(value) => Some(*value),
        Value::String(value) if value.eq_ignore_ascii_case("true") => Some(true),
        Value::String(value) if value.eq_ignore_ascii_case("false") => Some(false),
        _ => None,
    }
}

pub(crate) fn config_scalar_int(value: &Value) -> Option<c_int> {
    match value {
        Value::Number(value) => value
            .as_i64()
            .and_then(|number| c_int::try_from(number).ok()),
        Value::String(value) => parse_config_int(value),
        _ => None,
    }
}

pub(crate) fn parse_config_int(value: &str) -> Option<c_int> {
    if value.is_empty() {
        return None;
    }
    if let Some(hex) = value.strip_prefix("0x") {
        if let Ok(parsed) = u32::from_str_radix(hex, 16) {
            return c_int::try_from(parsed).ok();
        }
    }
    parse_config_i64_prefix(value).and_then(|number| c_int::try_from(number).ok())
}

pub(crate) fn parse_config_i64_prefix(value: &str) -> Option<i64> {
    let trimmed = value.trim_start();
    if trimmed.is_empty() {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let mut end = 0usize;
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        end = 1;
    }
    let digit_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if end == digit_start {
        return None;
    }
    trimmed[..end].parse::<i64>().ok()
}

pub(crate) fn config_scalar_double(value: &Value) -> Option<f64> {
    match value {
        Value::Number(value) => value.as_f64(),
        Value::String(value) if !value.is_empty() => parse_config_f64_prefix(value),
        _ => None,
    }
}

pub(crate) fn parse_config_f64_prefix(value: &str) -> Option<f64> {
    let trimmed = value.trim_start();
    for end in (1..=trimmed.len()).rev() {
        if !trimmed.is_char_boundary(end) {
            continue;
        }
        if let Ok(number) = trimmed[..end].parse::<f64>() {
            return Some(number);
        }
    }
    None
}

pub(crate) fn config_number_string(value: &Number) -> String {
    if let Some(number) = value.as_i64() {
        number.to_string()
    } else if let Some(number) = value.as_u64() {
        number.to_string()
    } else if let Some(number) = value.as_f64() {
        number.to_string()
    } else {
        String::new()
    }
}

pub(crate) unsafe fn c_string_key(key: *const c_char) -> Option<String> {
    if key.is_null() {
        return None;
    }
    // SAFETY: callers promise `key` is a valid NUL-terminated C string.
    Some(
        unsafe { CStr::from_ptr(key) }
            .to_string_lossy()
            .into_owned(),
    )
}
pub(crate) unsafe fn config_iterator_begin(
    iterator: *mut RimeConfigIterator,
    entries: Vec<(String, String)>,
    is_list: bool,
) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }

    let state = Box::into_raw(Box::new(ConfigIteratorState {
        entries,
        key_cache: None,
        path_cache: None,
    }))
    .cast::<c_void>();

    // SAFETY: `iterator` is non-null and points to caller-owned writable
    // storage; the boxed state is released by `RimeConfigEnd`.
    unsafe {
        (*iterator).list = if is_list { state } else { ptr::null_mut() };
        (*iterator).map = if is_list { ptr::null_mut() } else { state };
        (*iterator).index = -1;
        (*iterator).key = ptr::null();
        (*iterator).path = ptr::null();
    }
    TRUE
}

pub(crate) unsafe fn reset_config_iterator_for_begin(iterator: *mut RimeConfigIterator) {
    if iterator.is_null() {
        return;
    }
    // SAFETY: `iterator` is non-null and points to caller-owned writable
    // storage. This mirrors librime's pre-lookup field reset.
    unsafe {
        (*iterator).list = ptr::null_mut();
        (*iterator).map = ptr::null_mut();
        (*iterator).index = -1;
        (*iterator).key = ptr::null();
        (*iterator).path = ptr::null();
    }
}

pub(crate) fn config_child_path(root_path: &str, child_key: &str) -> String {
    if root_path.is_empty() || root_path == "/" {
        child_key.to_owned()
    } else {
        format!("{root_path}/{child_key}")
    }
}

pub(crate) fn find_config_value<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    if key.is_empty() {
        return Some(root);
    }

    let mut current = root;
    for segment in config_path_segments(key) {
        if let Some(index) = list_index_for_read(segment, current) {
            let Value::Sequence(sequence) = current else {
                return None;
            };
            current = sequence.get(index)?;
        } else {
            let Value::Mapping(mapping) = current else {
                return None;
            };
            current = mapping.get(Value::String(segment.to_owned()))?;
        }
    }
    Some(current)
}

pub(crate) fn find_config_value_mut<'a>(root: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    if key.is_empty() {
        return Some(root);
    }

    let mut current = root;
    for segment in config_path_segments(key) {
        if let Some(index) = list_index_for_read(segment, current) {
            let Value::Sequence(sequence) = current else {
                return None;
            };
            current = sequence.get_mut(index)?;
        } else {
            let Value::Mapping(mapping) = current else {
                return None;
            };
            current = mapping.get_mut(Value::String(segment.to_owned()))?;
        }
    }
    Some(current)
}

pub(crate) fn set_config_value(root: &mut Value, key: &str, value: Value) -> bool {
    if key.is_empty() {
        *root = value;
        return true;
    }

    let segments = config_path_segments(key);
    let Some((last, parents)) = segments.split_last() else {
        *root = value;
        return true;
    };

    let mut current = root;
    for (parent_index, segment) in parents.iter().enumerate() {
        if is_list_item_reference(segment) && current.is_null() {
            *current = Value::Sequence(Vec::new());
        }
        if let Some((index, insert)) = list_index_for_write(segment, current) {
            let Value::Sequence(sequence) = current else {
                return false;
            };
            let next_segment = if parent_index + 1 == parents.len() {
                *last
            } else {
                parents[parent_index + 1]
            };
            if insert {
                if index > sequence.len() {
                    sequence.resize(index, Value::Null);
                }
                sequence.insert(index, empty_config_container_for_next(next_segment));
            } else if index >= sequence.len() {
                sequence.resize(index + 1, Value::Null);
            }
            current = &mut sequence[index];
            if current.is_null() {
                *current = empty_config_container_for_next(next_segment);
            }
        } else if is_list_item_reference(segment) {
            return false;
        } else {
            if current.is_null() {
                *current = Value::Mapping(Mapping::new());
            }
            let Value::Mapping(mapping) = current else {
                return false;
            };
            let next_segment = if parent_index + 1 == parents.len() {
                *last
            } else {
                parents[parent_index + 1]
            };
            current = mapping
                .entry(Value::String((*segment).to_owned()))
                .or_insert_with(|| empty_config_container_for_next(next_segment));
        }
    }

    if is_list_item_reference(last) && current.is_null() {
        *current = Value::Sequence(Vec::new());
    }
    if let Some((index, insert)) = list_index_for_write(last, current) {
        let Value::Sequence(sequence) = current else {
            return false;
        };
        if insert {
            if index > sequence.len() {
                sequence.resize(index, Value::Null);
            }
            sequence.insert(index, value);
            true
        } else {
            if index >= sequence.len() {
                sequence.resize(index + 1, Value::Null);
            }
            sequence[index] = value;
            true
        }
    } else if is_list_item_reference(last) {
        false
    } else {
        if current.is_null() {
            *current = Value::Mapping(Mapping::new());
        }
        let Value::Mapping(mapping) = current else {
            return false;
        };
        mapping.insert(Value::String((*last).to_owned()), value);
        true
    }
}

pub(crate) fn config_path_segments(key: &str) -> Vec<&str> {
    let trimmed = key.trim_start_matches('/');
    if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    }
}

pub(crate) fn ensure_mapping(value: &mut Value) -> &mut Value {
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    value
}

pub(crate) fn empty_config_container_for_next(next_segment: &str) -> Value {
    if is_list_item_reference(next_segment) {
        Value::Sequence(Vec::new())
    } else {
        Value::Mapping(Mapping::new())
    }
}

pub(crate) fn list_index_for_read(segment: &str, current: &Value) -> Option<usize> {
    let Value::Sequence(sequence) = current else {
        return None;
    };
    list_index(segment, sequence.len()).map(|(index, _)| index)
}

pub(crate) fn list_index_for_write(segment: &str, current: &Value) -> Option<(usize, bool)> {
    let Value::Sequence(sequence) = current else {
        return None;
    };
    list_index(segment, sequence.len())
}

pub(crate) fn list_index(segment: &str, len: usize) -> Option<(usize, bool)> {
    let mut rest = segment.strip_prefix('@')?;
    if !rest
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphanumeric)
    {
        return None;
    }
    let mut index = 0usize;
    let mut insert = false;

    if let Some(after_next) = rest.strip_prefix("next") {
        rest = after_next;
        index = len;
    } else if let Some(after_before) = rest.strip_prefix("before") {
        rest = after_before;
        insert = true;
    } else if let Some(after_after) = rest.strip_prefix("after") {
        rest = after_after;
        index = 1;
        insert = true;
    }

    if let Some(after_space) = rest.strip_prefix(' ') {
        rest = after_space;
    }

    if rest.strip_prefix("last").is_some() {
        index = index.checked_add(len)?;
        index = index.saturating_sub(1);
    } else {
        let digits_len = rest
            .bytes()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits_len > 0 {
            index = index.checked_add(rest[..digits_len].parse::<usize>().ok()?)?;
        }
    }

    Some((index, insert))
}

pub(crate) fn is_list_item_reference(segment: &str) -> bool {
    segment
        .strip_prefix('@')
        .and_then(|rest| rest.as_bytes().first().copied())
        .is_some_and(|byte| byte.is_ascii_alphanumeric())
}
