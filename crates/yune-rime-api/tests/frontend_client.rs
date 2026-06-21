use std::{
    ffi::{c_void, CStr, CString},
    fs, mem,
    os::raw::{c_char, c_int},
    path::PathBuf,
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{
    rime_get_api, RimeCandidate, RimeCandidateListIterator, RimeCommit, RimeComposition,
    RimeConfig, RimeConfigIterator, RimeContext, RimeCustomApi, RimeLeversApi, RimeMenu,
    RimeModule, RimeSchemaList, RimeSessionId, RimeStatus, RimeTraits, RimeUserDictIterator, FALSE,
    TRUE,
};

use serde_yaml::Value;

#[derive(Debug, PartialEq, Eq)]
struct NotificationEvent {
    context_object: usize,
    session_id: RimeSessionId,
    message_type: String,
    message_value: String,
}

fn empty_context() -> RimeContext {
    RimeContext {
        data_size: (mem::size_of::<RimeContext>() - mem::size_of::<i32>()) as i32,
        composition: RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: ptr::null_mut(),
        },
        menu: RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        },
        commit_text_preview: ptr::null_mut(),
        select_labels: ptr::null_mut(),
    }
}

fn empty_status() -> RimeStatus {
    RimeStatus {
        data_size: (mem::size_of::<RimeStatus>() - mem::size_of::<i32>()) as i32,
        schema_id: ptr::null_mut(),
        schema_name: ptr::null_mut(),
        is_disabled: FALSE,
        is_composing: FALSE,
        is_ascii_mode: FALSE,
        is_full_shape: FALSE,
        is_simplified: FALSE,
        is_traditional: FALSE,
        is_ascii_punct: FALSE,
    }
}

fn empty_traits() -> RimeTraits {
    RimeTraits {
        data_size: mem::size_of::<RimeTraits>() as i32,
        shared_data_dir: ptr::null(),
        user_data_dir: ptr::null(),
        distribution_name: ptr::null(),
        distribution_code_name: ptr::null(),
        distribution_version: ptr::null(),
        app_name: ptr::null(),
        modules: ptr::null(),
        min_log_level: 0,
        log_dir: ptr::null(),
        prebuilt_data_dir: ptr::null(),
        staging_dir: ptr::null(),
    }
}

fn empty_commit() -> RimeCommit {
    RimeCommit {
        data_size: (mem::size_of::<RimeCommit>() - mem::size_of::<i32>()) as i32,
        text: ptr::null_mut(),
    }
}

fn empty_candidate_list_iterator() -> RimeCandidateListIterator {
    RimeCandidateListIterator {
        ptr: ptr::null_mut(),
        index: 0,
        candidate: RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        },
    }
}

fn empty_schema_list() -> RimeSchemaList {
    RimeSchemaList {
        size: 0,
        list: ptr::null_mut(),
    }
}

fn empty_config() -> RimeConfig {
    RimeConfig {
        ptr: ptr::null_mut(),
    }
}

fn empty_user_dict_iterator() -> RimeUserDictIterator {
    RimeUserDictIterator {
        ptr: ptr::null_mut(),
        i: 0,
    }
}

fn yaml_mapping_value<'a>(mapping: &'a serde_yaml::Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_owned()))
}

fn empty_config_iterator() -> RimeConfigIterator {
    RimeConfigIterator {
        list: ptr::null_mut(),
        map: ptr::null_mut(),
        index: 0,
        key: ptr::null(),
        path: ptr::null(),
    }
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned");
    let api = unsafe { &mut *rime_get_api() };
    let initialize = api
        .initialize
        .expect("frontend requires initialize for test setup");
    let traits = empty_traits();
    unsafe { initialize(&traits) };
    guard
}

fn notification_events() -> &'static Mutex<Vec<NotificationEvent>> {
    static NOTIFICATION_EVENTS: OnceLock<Mutex<Vec<NotificationEvent>>> = OnceLock::new();
    NOTIFICATION_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

extern "C" fn record_notification(
    context_object: *mut c_void,
    session_id: RimeSessionId,
    message_type: *const c_char,
    message_value: *const c_char,
) {
    // SAFETY: the shim invokes handlers with valid NUL-terminated message
    // strings for the duration of the callback.
    let message_type = unsafe { CStr::from_ptr(message_type) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: same as above.
    let message_value = unsafe { CStr::from_ptr(message_value) }
        .to_string_lossy()
        .into_owned();
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .push(NotificationEvent {
            context_object: context_object as usize,
            session_id,
            message_type,
            message_value,
        });
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "yune-rime-api-frontend-{label}-{}-{nanos}",
        std::process::id()
    ))
}

extern "C" fn frontend_module_initialize() {}

extern "C" fn frontend_module_finalize() {}

extern "C" fn frontend_module_get_api() -> *mut RimeCustomApi {
    ptr::null_mut()
}

include!("frontend_client/api_table_modules.rs");
include!("frontend_client/levers_userdb.rs");
include!("frontend_client/runtime_config_deploy.rs");
include!("frontend_client/config_values_signatures.rs");
include!("frontend_client/composition_dictionary_userdb.rs");
include!("frontend_client/schema_list.rs");
include!("frontend_client/speller.rs");
include!("frontend_client/raw_segment_selector.rs");
include!("frontend_client/input_editing_runtime.rs");
