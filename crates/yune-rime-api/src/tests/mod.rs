use std::env;
use std::ffi::{c_void, CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_yaml::Value;
use yune_core::{Candidate, CandidateSource, StaticTableTranslator, Translator};

mod abi;
mod candidate_api;
mod config_api;
mod context_status;
mod deployment;
mod levers;
mod runtime;
mod session_api;

use super::{
    bool_from, current_log_date_marker, find_config_value, rime_get_api, rime_levers_get_api,
    RimeApi, RimeCandidateListBegin, RimeCandidateListEnd, RimeCandidateListFromIndex,
    RimeCandidateListIterator, RimeCandidateListNext, RimeChangePage, RimeCleanupAllSessions,
    RimeCleanupStaleSessions, RimeClearComposition, RimeCommit, RimeCommitComposition, RimeConfig,
    RimeConfigBeginList, RimeConfigBeginMap, RimeConfigClear, RimeConfigClose,
    RimeConfigCreateList, RimeConfigCreateMap, RimeConfigEnd, RimeConfigGetBool,
    RimeConfigGetCString, RimeConfigGetDouble, RimeConfigGetInt, RimeConfigGetItem,
    RimeConfigGetString, RimeConfigInit, RimeConfigIterator, RimeConfigListSize,
    RimeConfigLoadString, RimeConfigNext, RimeConfigOpen, RimeConfigSetBool, RimeConfigSetDouble,
    RimeConfigSetInt, RimeConfigSetItem, RimeConfigSetString, RimeConfigUpdateSignature,
    RimeContext, RimeCreateSession, RimeCustomApi, RimeDeleteCandidate,
    RimeDeleteCandidateOnCurrentPage, RimeDeployConfigFile, RimeDeploySchema, RimeDeployWorkspace,
    RimeDeployerInitialize, RimeDestroySession, RimeFinalize, RimeFindModule, RimeFindSession,
    RimeFreeCommit, RimeFreeContext, RimeFreeStatus, RimeGetCaretPos, RimeGetCommit,
    RimeGetContext, RimeGetCurrentSchema, RimeGetInput, RimeGetKeyName, RimeGetKeycodeByName,
    RimeGetModifierByName, RimeGetModifierName, RimeGetOption, RimeGetPrebuiltDataDir,
    RimeGetPrebuiltDataDirSecure, RimeGetProperty, RimeGetSchemaList, RimeGetSharedDataDir,
    RimeGetSharedDataDirSecure, RimeGetStagingDir, RimeGetStagingDirSecure, RimeGetStateLabel,
    RimeGetStateLabelAbbreviated, RimeGetStatus, RimeGetSyncDir, RimeGetSyncDirSecure,
    RimeGetUserDataDir, RimeGetUserDataDirSecure, RimeGetUserDataSyncDir, RimeGetUserId,
    RimeGetVersion, RimeHighlightCandidate, RimeHighlightCandidateOnCurrentPage, RimeInitialize,
    RimeIsMaintenancing, RimeJoinMaintenanceThread, RimeLeversApi, RimeModule,
    RimePrebuildAllSchemas, RimeProcessKey, RimeRegisterModule, RimeRunTask, RimeSchemaOpen,
    RimeSelectCandidate, RimeSelectCandidateOnCurrentPage, RimeSelectSchema, RimeSetCaretPos,
    RimeSetInput, RimeSetNotificationHandler, RimeSetOption, RimeSetProperty, RimeSetup,
    RimeSetupLogging, RimeSimulateKeySequence, RimeStartMaintenance,
    RimeStartMaintenanceOnWorkspaceChange, RimeStatus, RimeSyncUserData, RimeTraits,
    RimeUserConfigOpen, RimeUserDictIterator, FALSE, K_ALT_MASK, K_CONTROL_MASK, K_LOCK_MASK,
    K_RELEASE_MASK, K_SHIFT_MASK, K_SUPER_MASK, TRUE, XK_RETURN,
};

#[derive(Debug, PartialEq, Eq)]
struct NotificationEvent {
    context_object: usize,
    session_id: super::RimeSessionId,
    message_type: String,
    message_value: String,
}

struct CommentTranslator;

impl Translator for CommentTranslator {
    fn name(&self) -> &'static str {
        "comment_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        if input != "ni" {
            return Vec::new();
        }
        vec![
            Candidate {
                text: "你".to_owned(),
                comment: "first-comment".to_owned(),
                source: CandidateSource::Table,
                quality: 1.0,
            },
            Candidate {
                text: "呢".to_owned(),
                comment: "second-comment".to_owned(),
                source: CandidateSource::Table,
                quality: 1.0,
            },
        ]
    }
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned");
    let traits = empty_traits();
    // SAFETY: empty traits points to valid storage for the duration of the call.
    unsafe { RimeInitialize(&traits) };
    guard
}

fn notification_events() -> &'static Mutex<Vec<NotificationEvent>> {
    static NOTIFICATION_EVENTS: OnceLock<Mutex<Vec<NotificationEvent>>> = OnceLock::new();
    NOTIFICATION_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

fn current_highlighted(session_id: super::RimeSessionId) -> usize {
    super::sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .sessions
        .get(&session_id)
        .expect("session should exist")
        .engine
        .context()
        .highlighted
}

extern "C" fn record_notification(
    context_object: *mut c_void,
    session_id: super::RimeSessionId,
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

extern "C" fn sample_module_initialize() {}

extern "C" fn sample_module_finalize() {}

extern "C" fn sample_module_get_api() -> *mut RimeCustomApi {
    std::ptr::null_mut()
}

fn empty_context() -> RimeContext {
    RimeContext {
        data_size: (std::mem::size_of::<RimeContext>() - std::mem::size_of::<i32>()) as i32,
        composition: super::RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: std::ptr::null_mut(),
        },
        menu: super::RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: std::ptr::null_mut(),
            select_keys: std::ptr::null_mut(),
        },
        commit_text_preview: std::ptr::null_mut(),
        select_labels: std::ptr::null_mut(),
    }
}

fn context_data_size_before_commit_text_preview() -> i32 {
    let context = empty_context();
    let base = &context as *const RimeContext as usize;
    let member = std::ptr::addr_of!(context.commit_text_preview) as usize;
    (member - base - std::mem::size_of::<i32>()) as i32
}

fn align_up(offset: usize, alignment: usize) -> usize {
    if alignment == 0 {
        return offset;
    }
    let remainder = offset % alignment;
    if remainder == 0 {
        offset
    } else {
        offset + alignment - remainder
    }
}

fn field_offset<T, U>(base: &T, member: *const U) -> usize {
    member as usize - base as *const T as usize
}

fn empty_status() -> RimeStatus {
    RimeStatus {
        data_size: (std::mem::size_of::<RimeStatus>() - std::mem::size_of::<i32>()) as i32,
        schema_id: std::ptr::null_mut(),
        schema_name: std::ptr::null_mut(),
        is_disabled: FALSE,
        is_composing: FALSE,
        is_ascii_mode: FALSE,
        is_full_shape: FALSE,
        is_simplified: FALSE,
        is_traditional: FALSE,
        is_ascii_punct: FALSE,
    }
}

fn empty_candidate_list_iterator() -> RimeCandidateListIterator {
    RimeCandidateListIterator {
        ptr: std::ptr::null_mut(),
        index: 0,
        candidate: super::RimeCandidate {
            text: std::ptr::null_mut(),
            comment: std::ptr::null_mut(),
            reserved: std::ptr::null_mut(),
        },
    }
}

fn empty_schema_list() -> super::RimeSchemaList {
    super::RimeSchemaList {
        size: 0,
        list: std::ptr::null_mut(),
    }
}

fn empty_config() -> RimeConfig {
    RimeConfig {
        ptr: std::ptr::null_mut(),
    }
}

fn static_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn empty_config_iterator() -> RimeConfigIterator {
    RimeConfigIterator {
        list: std::ptr::null_mut(),
        map: std::ptr::null_mut(),
        index: 0,
        key: std::ptr::null(),
        path: std::ptr::null(),
    }
}

fn empty_traits() -> RimeTraits {
    RimeTraits {
        data_size: std::mem::size_of::<RimeTraits>() as i32,
        shared_data_dir: std::ptr::null(),
        user_data_dir: std::ptr::null(),
        distribution_name: std::ptr::null(),
        distribution_code_name: std::ptr::null(),
        distribution_version: std::ptr::null(),
        app_name: std::ptr::null(),
        modules: std::ptr::null(),
        min_log_level: 0,
        log_dir: std::ptr::null(),
        prebuilt_data_dir: std::ptr::null(),
        staging_dir: std::ptr::null(),
    }
}

fn traits_data_size_before_prebuilt_data_dir() -> i32 {
    let traits = empty_traits();
    let base = &traits as *const RimeTraits as usize;
    let member = std::ptr::addr_of!(traits.prebuilt_data_dir) as usize;
    (member - base - std::mem::size_of::<i32>()) as i32
}

fn config_string(config: &mut RimeConfig, key: &str) -> Option<String> {
    let key = CString::new(key).expect("key should be valid");
    let mut buffer = [0 as c_char; 128];
    // SAFETY: config, key, and output buffer are valid for the call.
    let ok =
        unsafe { RimeConfigGetString(config, key.as_ptr(), buffer.as_mut_ptr(), buffer.len()) };
    if ok == FALSE {
        return None;
    }
    // SAFETY: successful config string copies are NUL-terminated.
    Some(
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn config_bool(config: &mut RimeConfig, key: &str) -> Option<c_int> {
    let key = CString::new(key).expect("key should be valid");
    let mut output = FALSE;
    // SAFETY: config, key, and output pointer are valid for the call.
    (unsafe { RimeConfigGetBool(config, key.as_ptr(), &mut output) } == TRUE).then_some(output)
}

fn assert_librime_ctime_shape(value: &str) {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    assert_eq!(parts.len(), 5);
    assert!(["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].contains(&parts[0]));
    assert!(
        ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",]
            .contains(&parts[1])
    );
    assert!(parts[2]
        .parse::<u8>()
        .is_ok_and(|day| (1..=31).contains(&day)));
    assert_eq!(parts[3].len(), 8);
    assert_eq!(parts[3].as_bytes()[2], b':');
    assert_eq!(parts[3].as_bytes()[5], b':');
    assert!(parts[4].parse::<u16>().is_ok());
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "yune-rime-api-{name}-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn schema_key_binder_processor_toggles_options_from_bindings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-toggle");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, toggle: ascii_mode }
    - { when: composing, accept: Control+Shift+2, toggle: full_shape }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_prefers_later_same_condition_binding() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-same-condition-order");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, toggle: ascii_mode }
    - { when: always, accept: Control+Shift+1, toggle: full_shape }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_sets_and_unsets_switch_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-set-option");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
switches:
  - name: ascii_mode
  - options: [simplification, traditional]
    reset: 0
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, set_option: ascii_mode }
    - { when: always, accept: Control+Shift+2, unset_option: ascii_mode }
    - { when: always, accept: Control+Shift+3, set_option: traditional }
    - { when: always, accept: Control+Shift+4, unset_option: traditional }
    - { when: always, accept: Control+Shift+5, toggle: simplification }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(
        RimeProcessKey(session_id, '3' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '4' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(
        RimeProcessKey(session_id, '5' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_toggles_switches_by_index() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-toggle-index");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
switches:
  - name: ascii_mode
  - options: [simplification, traditional]
    reset: 0
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, toggle: '@0' }
    - { when: always, accept: Control+Shift+2, toggle: '@1' }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_redirects_send_bindings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-send");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: '/', send_sequence: 'xy' }
    - { when: composing, accept: ';', send: BackSpace }
    - { when: always, accept: ',', send: ',' }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let preedit = |session_id| {
        let mut context = empty_context();
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
        let text = unsafe { CStr::from_ptr(context.composition.preedit) }
            .to_str()
            .expect("preedit should be valid UTF-8")
            .to_owned();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        unsafe { RimeFreeContext(&mut context) };
        text
    };

    assert_eq!(RimeProcessKey(session_id, '/' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "xy");

    assert_eq!(RimeProcessKey(session_id, ';' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "x");

    assert_eq!(RimeProcessKey(session_id, ',' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "x,");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_loads_namespaced_prescription() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-namespaced-key-binder");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - key_binder@custom_processor
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: '/', send_sequence: 'xy' }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, '/' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage for the populated context.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: `RimeGetContext` populated a valid preedit C string.
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_str()
        .expect("preedit should be valid UTF-8");
    assert_eq!(preedit, "xy");
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    unsafe { RimeFreeContext(&mut context) };

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_matches_paging_condition_after_page_navigation() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-paging");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
menu:
  page_size: 2
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: has_menu, accept: ',', toggle: full_shape }
    - { when: paging, accept: ',', toggle: ascii_mode }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let page_down = CString::new("Page_Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let page_down_keycode = unsafe { RimeGetKeycodeByName(page_down.as_ptr()) };
    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ',' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeProcessKey(session_id, page_down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ',' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_reinterprets_period_paging_before_letters() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-reinterpret-period");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: has_menu, accept: period, send: Page_Down }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let preedit = |session_id| {
        let mut context = empty_context();
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
        let text = unsafe { CStr::from_ptr(context.composition.preedit) }
            .to_str()
            .expect("preedit should be valid UTF-8")
            .to_owned();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        unsafe { RimeFreeContext(&mut context) };
        text
    };

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "ba");
    assert_eq!(RimeProcessKey(session_id, 'c' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "ba.c");

    assert_eq!(RimeProcessKey(session_id, 0xff1b, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'c' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "bac");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_selects_schemas_from_bindings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    RimeSetNotificationHandler(None, std::ptr::null_mut());
    let root = unique_temp_dir("schema-key-binder-select");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
schema_list:
  - schema: alpha
  - schema: beta
  - schema: gamma
",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("alpha.schema.yaml"),
        "\
schema:
  schema_id: alpha
  name: Alpha
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, select: beta }
",
    )
    .expect("alpha schema config should be written");
    fs::write(
        staging.join("beta.schema.yaml"),
        "\
schema:
  schema_id: beta
  name: Beta
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+2, select: .next }
",
    )
    .expect("beta schema config should be written");
    fs::write(
        staging.join("gamma.schema.yaml"),
        "schema:\n  schema_id: gamma\n  name: Gamma\n",
    )
    .expect("gamma schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let alpha = CString::new("alpha").expect("schema id should be valid");
    let context_object = 0x51_usize as *mut c_void;
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, alpha.as_ptr()) },
        TRUE
    );
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
    RimeSetNotificationHandler(Some(record_notification), context_object);

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: status strings were allocated by RimeGetStatus above.
    assert_eq!(
        unsafe { CStr::from_ptr(status.schema_id) }.to_str(),
        Ok("beta")
    );
    // SAFETY: nested status allocations were returned by RimeGetStatus above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: status strings were allocated by RimeGetStatus above.
    assert_eq!(
        unsafe { CStr::from_ptr(status.schema_id) }.to_str(),
        Ok("alpha")
    );
    // SAFETY: nested status allocations were returned by RimeGetStatus above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned");
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x51,
                session_id,
                message_type: "schema".to_owned(),
                message_value: "beta/Beta".to_owned(),
            },
            NotificationEvent {
                context_object: 0x51,
                session_id,
                message_type: "schema".to_owned(),
                message_value: "alpha/Alpha".to_owned(),
            },
        ]
    );
    drop(events);

    RimeSetNotificationHandler(None, std::ptr::null_mut());
    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn sync_user_data_merges_plain_userdb_snapshots_and_backs_up_current_state() {
    let _guard = test_guard();
    let root = unique_temp_dir("rime-sync-user-data");
    let user = root.join("user");
    let peer_sync = user.join("sync").join("peer");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::create_dir_all(&peer_sync).expect("peer sync dir should be created");
    struct CurrentDirGuard(PathBuf);
    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }
    let current_dir_guard =
        CurrentDirGuard(env::current_dir().expect("current dir should be available"));
    env::set_current_dir(&root).expect("test cwd should move under temp root");

    fs::write(user.join("luna_pinyin.userdb"), "ni hao\t你好\t1\n")
        .expect("local user dict should be written");
    fs::write(user.join("default.yaml"), "config_version: '1.0'\n")
        .expect("user config should be written");
    fs::write(user.join("notes.txt"), "local notes\n").expect("text file should be written");
    fs::write(
        user.join("generated.yaml"),
        "customization: abc123\nschema:\n  name: Generated\n",
    )
    .expect("generated customized copy should be written");
    fs::write(
        peer_sync.join("luna_pinyin.userdb.txt"),
        "ni hao\t你好\t1\nzhong guo\t中国\t2\n",
    )
    .expect("peer snapshot should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    assert_eq!(RimeFindSession(session_id), TRUE);
    assert_eq!(RimeSyncUserData(), TRUE);
    assert_eq!(RimeFindSession(session_id), FALSE);

    let merged =
        fs::read_to_string(user.join("luna_pinyin.userdb")).expect("dict should be readable");
    assert_eq!(merged, "ni hao\t你好\t1\nzhong guo\t中国\t2\n");

    let installation_metadata = fs::read_to_string(user.join("installation.yaml"))
        .expect("installation metadata should be written during sync");
    let installation_metadata: Value =
        serde_yaml::from_str(&installation_metadata).expect("installation metadata should parse");
    let installation_id = find_config_value(&installation_metadata, "installation_id")
        .and_then(Value::as_str)
        .expect("installation id should be available");
    let sync_user_dir = user.join("sync").join(installation_id);
    let backup = fs::read_to_string(sync_user_dir.join("luna_pinyin.userdb.txt"))
        .expect("current user snapshot should be written");
    assert_eq!(backup, merged);

    assert_eq!(
        fs::read_to_string(sync_user_dir.join("default.yaml"))
            .expect("user config backup should be readable"),
        "config_version: '1.0'\n"
    );
    assert_eq!(
        fs::read_to_string(sync_user_dir.join("notes.txt"))
            .expect("text backup should be readable"),
        "local notes\n"
    );
    assert!(!sync_user_dir.join("generated.yaml").exists());

    let task_name = CString::new("user_dict_sync").expect("task name should be valid");
    assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);
    fs::remove_file(sync_user_dir.join("default.yaml")).expect("config backup should be removable");
    let backup_config_task =
        CString::new("backup_config_files").expect("task name should be valid");
    assert_eq!(RimeRunTask(backup_config_task.as_ptr()), TRUE);
    assert_eq!(
        fs::read_to_string(sync_user_dir.join("default.yaml"))
            .expect("task should recreate config backup"),
        "config_version: '1.0'\n"
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    drop(current_dir_guard);
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn control_delete_key_removes_highlighted_candidate_like_librime_editor_delete_candidate() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let delete = CString::new("Delete").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let delete_keycode = unsafe { RimeGetKeycodeByName(delete.as_ptr()) };
    assert_eq!(delete_keycode, 0xffff);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);
    let control = CString::new("Control").expect("modifier name should be valid");
    // SAFETY: modifier name is a valid NUL-terminated string.
    let control_mask = unsafe { RimeGetModifierByName(control.as_ptr()) };
    assert_eq!(control_mask, 1 << 2);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, delete_keycode, control_mask),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 3);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: `candidates` points to at least two initialized candidates.
    let second_candidate = unsafe { *context.menu.candidates.add(1) };
    // SAFETY: candidate text is a valid NUL-terminated string owned by context.
    let second_text = unsafe { CStr::from_ptr(second_candidate.text) };
    assert_eq!(second_text.to_str(), Ok("爸"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn shift_delete_key_removes_highlighted_candidate_like_librime_editor_shift_as_control_fallback() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let delete = CString::new("Delete").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let delete_keycode = unsafe { RimeGetKeycodeByName(delete.as_ptr()) };
    assert_eq!(delete_keycode, 0xffff);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);
    let shift = CString::new("Shift").expect("modifier name should be valid");
    // SAFETY: modifier name is a valid NUL-terminated string.
    let shift_mask = unsafe { RimeGetModifierByName(shift.as_ptr()) };
    assert_eq!(shift_mask, 1);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, delete_keycode, shift_mask), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 3);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: `candidates` points to at least two initialized candidates.
    let second_candidate = unsafe { *context.menu.candidates.add(1) };
    // SAFETY: candidate text is a valid NUL-terminated string owned by context.
    let second_text = unsafe { CStr::from_ptr(second_candidate.text) };
    assert_eq!(second_text.to_str(), Ok("爸"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));
    }
    let sequence = CString::new("ba{Down}{Shift+Delete}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(
        unsafe { RimeGetContext(sequence_session_id, &mut context) },
        TRUE
    );
    assert_eq!(context.menu.num_candidates, 3);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: `candidates` points to at least two initialized candidates.
    let second_candidate = unsafe { *context.menu.candidates.add(1) };
    // SAFETY: candidate text is a valid NUL-terminated string owned by context.
    let second_text = unsafe { CStr::from_ptr(second_candidate.text) };
    assert_eq!(second_text.to_str(), Ok("爸"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn keypad_enter_commits_composition_like_librime_return_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_enter = CString::new("KP_Enter").expect("key name should be valid");
    let shift = CString::new("Shift").expect("modifier name should be valid");
    let kp_enter_keycode = unsafe { RimeGetKeycodeByName(kp_enter.as_ptr()) };
    let shift_mask = unsafe { RimeGetModifierByName(shift.as_ptr()) };
    assert_eq!(kp_enter_keycode, 0xff8d);
    assert_eq!(shift_mask, 1);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_enter_keycode, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{KP_Enter}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let modified_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&modified_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(modified_session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(modified_session_id, 'i' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(modified_session_id, kp_enter_keycode, shift_mask),
        FALSE
    );
    let modified_sequence =
        CString::new("{Control+KP_Enter}{Shift+KP_Enter}{Control+Shift+KP_Enter}")
            .expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(modified_session_id, modified_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and no unread commit is expected.
    assert_eq!(
        unsafe { RimeGetCommit(modified_session_id, &mut commit) },
        FALSE
    );
    assert_eq!(RimeGetCaretPos(modified_session_id), 2);
    let input = RimeGetInput(modified_session_id);
    assert!(!input.is_null());
    // SAFETY: RimeGetInput returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    let unmodified_sequence = CString::new("{KP_Enter}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(modified_session_id, unmodified_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(modified_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(modified_session_id), TRUE);
}

#[test]
fn keypad_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(RimeProcessKey(session_id, kp_2_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_2_keycode, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("{KP_2}ba{KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_keypad_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(
        RimeProcessKey(session_id, kp_2_keycode, K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_2_keycode, K_SHIFT_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("{Shift+KP_2}ba{Shift+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_ascii_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, K_SHIFT_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Shift+2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_ascii_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(
        RimeProcessKey(session_id, '2' as i32, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, K_CONTROL_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Control+2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_keypad_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(
        RimeProcessKey(session_id, kp_2_keycode, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, kp_2_keycode, K_CONTROL_MASK),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Control+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_shift_digits_select_candidates_like_librime_selector_fallback() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let control_shift = K_CONTROL_MASK | K_SHIFT_MASK;
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(RimeProcessKey(session_id, '2' as i32, control_shift), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, kp_2_keycode, control_shift),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, control_shift), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let keypad_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&keypad_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(RimeProcessKey(keypad_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(keypad_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(keypad_session_id, kp_2_keycode, control_shift),
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(keypad_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(keypad_session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Control+Shift+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn escape_clears_composition_like_librime_editor_cancel_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let escape = CString::new("Escape").expect("key name should be valid");
    let escape_keycode = unsafe { RimeGetKeycodeByName(escape.as_ptr()) };
    assert_eq!(escape_keycode, 0xff1b);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(session_id, escape_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, escape_keycode, 0), TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    let mut context = empty_context();
    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    assert_eq!(context.menu.num_candidates, 0);
    assert!(context.menu.candidates.is_null());
    // SAFETY: nested pointers are null after the empty context response.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{Escape}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let sequence_input = RimeGetInput(sequence_session_id);
    assert!(!sequence_input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(sequence_input) }.to_str(), Ok(""));
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        FALSE
    );
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_escape_clears_composition_like_librime_editor_cancel_fallback() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let escape = CString::new("Escape").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let escape_keycode = unsafe { RimeGetKeycodeByName(escape.as_ptr()) };
    assert_eq!(escape_keycode, 0xff1b);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, escape_keycode, K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, escape_keycode, K_SHIFT_MASK),
        TRUE
    );

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence = CString::new("ni{Shift+Escape}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let sequence_input = RimeGetInput(sequence_session_id);
    assert!(!sequence_input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(sequence_input) }.to_str(), Ok(""));
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn backspace_key_removes_input_before_caret_like_librime_editor_back() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let backspace = CString::new("BackSpace").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let backspace_keycode = unsafe { RimeGetKeycodeByName(backspace.as_ptr()) };
    assert_eq!(backspace_keycode, 0xff08);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nxi").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, backspace_keycode, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 1);
    RimeSetCaretPos(session_id, 0);
    assert_eq!(RimeProcessKey(session_id, backspace_keycode, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("nxi{Left}{BackSpace}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_backspace_key_removes_previous_input_like_librime_editor_back_syllable() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let backspace = CString::new("BackSpace").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let backspace_keycode = unsafe { RimeGetKeycodeByName(backspace.as_ptr()) };
    assert_eq!(backspace_keycode, 0xff08);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nxi").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    RimeSetCaretPos(session_id, 2);
    assert_eq!(
        RimeProcessKey(session_id, backspace_keycode, K_CONTROL_MASK),
        TRUE
    );
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 1);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence =
        CString::new("nxi{Left}{Control+BackSpace}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_backspace_key_uses_librime_editor_shift_as_control_fallback() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let backspace = CString::new("BackSpace").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let backspace_keycode = unsafe { RimeGetKeycodeByName(backspace.as_ptr()) };
    assert_eq!(backspace_keycode, 0xff08);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nxi").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    RimeSetCaretPos(session_id, 2);
    assert_eq!(
        RimeProcessKey(session_id, backspace_keycode, K_SHIFT_MASK),
        TRUE
    );
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 1);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence =
        CString::new("nxi{Left}{Shift+BackSpace}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_return_key_commits_raw_input_like_librime_fluid_editor() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, K_CONTROL_MASK),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{Control+Return}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_return_key_commits_script_text_like_librime_fluid_editor() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, K_SHIFT_MASK),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{Shift+Return}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_printable_keys_enter_input_and_shift_space_confirms_like_librime_editor() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let session_id = RimeCreateSession();
    assert_eq!(RimeProcessKey(session_id, 'A' as i32, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("Ab"));
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, K_SHIFT_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("Ab"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, K_SHIFT_MASK), FALSE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence = CString::new("{Shift+A}b{Shift+space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("Ab"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_shift_return_key_commits_selected_comment_like_librime_fluid_editor() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);
    let modifier_mask = K_CONTROL_MASK | K_SHIFT_MASK;

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(CommentTranslator);
    }
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, modifier_mask),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(
        unsafe { CStr::from_ptr(commit.text) }.to_str(),
        Ok("second-comment")
    );
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, modifier_mask),
        FALSE
    );
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session.engine.add_translator(CommentTranslator);
    }
    let sequence =
        CString::new("ni{Down}{Control+Shift+Return}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(
        unsafe { CStr::from_ptr(commit.text) }.to_str(),
        Ok("second-comment")
    );
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn delete_key_removes_input_at_caret_like_librime_editor_delete_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let delete = CString::new("Delete").expect("key name should be valid");
    let delete_keycode = unsafe { RimeGetKeycodeByName(delete.as_ptr()) };
    assert_eq!(delete_keycode, 0xffff);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, delete_keycode, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 2);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeSetInput(sequence_session_id, input.as_ptr()) },
        TRUE
    );
    RimeSetCaretPos(sequence_session_id, 2);
    let sequence = CString::new("{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn left_right_keys_move_caret_like_librime_navigator_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(left_keycode, 0xff51);
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };
    assert_eq!(right_keycode, 0xff53);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, left_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 2);
    RimeSetCaretPos(session_id, 1);
    assert_eq!(RimeProcessKey(session_id, right_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 2);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("nix{Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_left_right_keys_jump_syllable_span_like_librime_navigator_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(left_keycode, 0xff51);
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };
    assert_eq!(right_keycode, 0xff53);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence =
        CString::new("nix{Control+Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ix"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_left_right_keys_fall_back_to_control_syllable_jump_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(left_keycode, 0xff51);
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };
    assert_eq!(right_keycode, 0xff53);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_SHIFT_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, left_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence =
        CString::new("nix{Shift+Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ix"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_up_down_keys_jump_syllable_span_like_librime_vertical_navigator_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let up = CString::new("Up").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let up_keycode = unsafe { RimeGetKeycodeByName(up.as_ptr()) };
    assert_eq!(up_keycode, 0xff52);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, up_keycode, K_CONTROL_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, up_keycode, K_CONTROL_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(
        RimeProcessKey(session_id, down_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence =
        CString::new("nix{Control+Up}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ix"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn linear_selector_arrow_keys_follow_librime_layout_bindings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let up = CString::new("Up").expect("key name should be valid");
    let down = CString::new("Down").expect("key name should be valid");
    let left = CString::new("Left").expect("key name should be valid");
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key names are valid NUL-terminated strings.
    let up_keycode = unsafe { RimeGetKeycodeByName(up.as_ptr()) };
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let linear = CString::new("_linear").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, linear.as_ptr(), TRUE) };

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 0);

    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 5);
    assert_eq!(RimeProcessKey(session_id, up_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 0);

    assert_eq!(RimeProcessKey(session_id, right_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 1);

    RimeSetCaretPos(session_id, 1);
    assert_eq!(RimeProcessKey(session_id, left_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 1);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let vertical_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&vertical_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let vertical = CString::new("_vertical").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(vertical_session_id, vertical.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(vertical_session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(vertical_session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(vertical_session_id, left_keycode, 0), TRUE);
    assert_eq!(current_highlighted(vertical_session_id), 1);
    assert_eq!(RimeProcessKey(vertical_session_id, right_keycode, 0), TRUE);
    assert_eq!(current_highlighted(vertical_session_id), 0);
    assert_eq!(RimeDestroySession(vertical_session_id), TRUE);

    let vertical_linear_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&vertical_linear_session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    // SAFETY: option names are valid NUL-terminated strings.
    unsafe {
        RimeSetOption(vertical_linear_session_id, vertical.as_ptr(), TRUE);
        RimeSetOption(vertical_linear_session_id, linear.as_ptr(), TRUE);
    }
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, 'b' as c_int, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, 'a' as c_int, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, left_keycode, 0),
        TRUE
    );
    assert_eq!(current_highlighted(vertical_linear_session_id), 5);
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, right_keycode, 0),
        TRUE
    );
    assert_eq!(current_highlighted(vertical_linear_session_id), 0);
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, down_keycode, 0),
        TRUE
    );
    assert_eq!(current_highlighted(vertical_linear_session_id), 1);
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, up_keycode, 0),
        TRUE
    );
    assert_eq!(current_highlighted(vertical_linear_session_id), 0);
    assert_eq!(RimeDestroySession(vertical_linear_session_id), TRUE);
}

#[test]
fn schema_selector_bindings_override_default_layout_keymap() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-selector-bindings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\nselector:\n  bindings:\n    Control+j: next_candidate\n    Down: noop\n  linear:\n    bindings:\n      Control+k: previous_page\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 0);

    assert_eq!(
        RimeProcessKey(session_id, 'j' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(current_highlighted(session_id), 1);
    assert_eq!(
        RimeProcessKey(session_id, 'j' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(current_highlighted(session_id), 2);

    let linear = CString::new("_linear").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, linear.as_ptr(), TRUE) };
    assert_eq!(
        RimeProcessKey(session_id, 'k' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(current_highlighted(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_navigator_bindings_override_default_keymap() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-navigator-bindings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nnavigator:\n  bindings:\n    Control+h: left_by_char\n    Control+l: right_by_char_no_loop\n    Left: noop\n  vertical:\n    bindings:\n      Control+j: end\n      Control+k: home\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(left_keycode, 0xff51);

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let input = CString::new("abc").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(
        RimeProcessKey(session_id, 'h' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 1);
    assert_eq!(RimeProcessKey(session_id, left_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 1);
    assert_eq!(
        RimeProcessKey(session_id, 'l' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);

    let vertical = CString::new("_vertical").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, vertical.as_ptr(), TRUE) };
    assert_eq!(
        RimeProcessKey(session_id, 'j' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(
        RimeProcessKey(session_id, 'k' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_navigator_syllable_jump_position_honors_delimiters() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-navigator-delimiter-jump");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("after.schema.yaml"),
        "\
schema:\n  schema_id: after\n  name: After\nspeller:\n  delimiter: \"'\"\n",
    )
    .expect("after schema config should be written");
    fs::write(
        staging.join("before.schema.yaml"),
        "\
schema:\n  schema_id: before\n  name: Before\nspeller:\n  delimiter: \"'\"\nnavigator:\n  syllable_jump_position: before_delimiter\n  bindings:\n    Control+h: left_by_syllable_no_loop\n    Control+l: right_by_syllable_no_loop\n",
    )
    .expect("before schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let left = CString::new("Left").expect("key name should be valid");
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key names are valid NUL-terminated strings.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };
    let input = CString::new("ab'cd").expect("input should be valid");

    let session_id = RimeCreateSession();
    let after_schema = CString::new("after").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, after_schema.as_ptr()) },
        TRUE
    );
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    RimeSetCaretPos(session_id, 5);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);

    let before_schema = CString::new("before").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, before_schema.as_ptr()) },
        TRUE
    );
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);

    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    RimeSetCaretPos(session_id, 5);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, 'h' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    RimeSetCaretPos(session_id, 5);
    assert_eq!(
        RimeProcessKey(session_id, 'l' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 5);
    RimeSetCaretPos(session_id, 4);
    assert_eq!(
        RimeProcessKey(session_id, 'h' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    assert_eq!(
        RimeProcessKey(session_id, 'l' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 5);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn shift_up_down_keys_fall_back_to_control_syllable_jump_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let up = CString::new("Up").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let up_keycode = unsafe { RimeGetKeycodeByName(up.as_ptr()) };
    assert_eq!(up_keycode, 0xff52);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);

    let session_id = RimeCreateSession();
    assert_eq!(RimeProcessKey(session_id, up_keycode, K_SHIFT_MASK), FALSE);
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, up_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeProcessKey(session_id, down_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence = CString::new("nix{Shift+Up}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ix"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn keypad_left_right_keys_move_caret_by_char_with_librime_navigator_looping() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_left = CString::new("KP_Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_left_keycode = unsafe { RimeGetKeycodeByName(kp_left.as_ptr()) };
    assert_eq!(kp_left_keycode, 0xff96);
    let kp_right = CString::new("KP_Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_right_keycode = unsafe { RimeGetKeycodeByName(kp_right.as_ptr()) };
    assert_eq!(kp_right_keycode, 0xff98);

    let session_id = RimeCreateSession();
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 0);
    assert_eq!(RimeProcessKey(session_id, kp_left_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, kp_right_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("nix{KP_Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_keypad_left_right_keys_ignore_shift_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_left = CString::new("KP_Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_left_keycode = unsafe { RimeGetKeycodeByName(kp_left.as_ptr()) };
    assert_eq!(kp_left_keycode, 0xff96);
    let kp_right = CString::new("KP_Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_right_keycode = unsafe { RimeGetKeycodeByName(kp_right.as_ptr()) };
    assert_eq!(kp_right_keycode, 0xff98);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, kp_left_keycode, K_SHIFT_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, kp_left_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(
        RimeProcessKey(session_id, kp_right_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence =
        CString::new("nix{Shift+KP_Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_keypad_up_down_keys_ignore_shift_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_up = CString::new("KP_Up").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_up_keycode = unsafe { RimeGetKeycodeByName(kp_up.as_ptr()) };
    assert_eq!(kp_up_keycode, 0xff97);
    let kp_down = CString::new("KP_Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_down_keycode = unsafe { RimeGetKeycodeByName(kp_down.as_ptr()) };
    assert_eq!(kp_down_keycode, 0xff99);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, kp_up_keycode, K_SHIFT_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, kp_up_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(
        RimeProcessKey(session_id, kp_down_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence =
        CString::new("nix{Shift+KP_Up}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn page_keys_move_candidate_page_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("page-key-selector");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let page_down = CString::new("Page_Down").expect("key name should be valid");
    let page_down_keycode = unsafe { RimeGetKeycodeByName(page_down.as_ptr()) };
    assert_eq!(page_down_keycode, 0xff56);
    let kp_page_up = CString::new("KP_Page_Up").expect("key name should be valid");
    let kp_page_up_keycode = unsafe { RimeGetKeycodeByName(kp_page_up.as_ptr()) };
    assert_eq!(kp_page_up_keycode, 0xff9a);

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, page_down_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, page_down_keycode, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_size, 2);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: context.menu.candidates points to at least one candidate.
    let first_candidate = unsafe { *context.menu.candidates };
    // SAFETY: candidate text is owned by the returned context and is valid until free.
    assert_eq!(
        unsafe { CStr::from_ptr(first_candidate.text) }.to_str(),
        Ok("爸")
    );
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, kp_page_up_keycode, 0), TRUE);
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let sequence = CString::new("ba{Next}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(
        unsafe { RimeGetContext(sequence_session_id, &mut context) },
        TRUE
    );
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn up_down_keys_move_candidate_highlight_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);
    let kp_up = CString::new("KP_Up").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_up_keycode = unsafe { RimeGetKeycodeByName(kp_up.as_ptr()) };
    assert_eq!(kp_up_keycode, 0xff97);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, kp_up_keycode, 0), TRUE);
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Down}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn home_end_keys_reset_candidate_highlight_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let home = CString::new("Home").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let home_keycode = unsafe { RimeGetKeycodeByName(home.as_ptr()) };
    assert_eq!(home_keycode, 0xff50);
    let kp_end = CString::new("KP_End").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_end_keycode = unsafe { RimeGetKeycodeByName(kp_end.as_ptr()) };
    assert_eq!(kp_end_keycode, 0xff9c);

    let session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, home_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);

    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, home_keycode, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_end_keycode, 0), TRUE);
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Down}{KP_End}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("八"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn home_end_keys_fall_back_to_librime_navigator_caret_movement() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let home = CString::new("Home").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let home_keycode = unsafe { RimeGetKeycodeByName(home.as_ptr()) };
    assert_eq!(home_keycode, 0xff50);
    let end = CString::new("End").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let end_keycode = unsafe { RimeGetKeycodeByName(end.as_ptr()) };
    assert_eq!(end_keycode, 0xff57);

    let session_id = RimeCreateSession();
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'x' as i32, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, home_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeProcessKey(session_id, end_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence =
        CString::new("nix{Home}{Delete}{End}{BackSpace}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("i"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_home_end_keys_ignore_shift_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let home = CString::new("Home").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let home_keycode = unsafe { RimeGetKeycodeByName(home.as_ptr()) };
    assert_eq!(home_keycode, 0xff50);
    let end = CString::new("End").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let end_keycode = unsafe { RimeGetKeycodeByName(end.as_ptr()) };
    assert_eq!(end_keycode, 0xff57);
    let kp_end = CString::new("KP_End").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_end_keycode = unsafe { RimeGetKeycodeByName(kp_end.as_ptr()) };
    assert_eq!(kp_end_keycode, 0xff9c);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, home_keycode, K_SHIFT_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, home_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeProcessKey(session_id, end_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, home_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(
        RimeProcessKey(session_id, kp_end_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence = CString::new("nix{Shift+Home}{Delete}{Shift+KP_End}{BackSpace}{space}")
        .expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("i"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn schema_ascii_composer_rejects_direct_ascii_and_edits_inline_ascii() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - ascii_composer
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
translator:
  dictionary: luna
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni
",
    )
    .expect("dictionary should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };

    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), FALSE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, ' ' as c_int, 0), TRUE);

    let mut no_commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut no_commit) }, FALSE);
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 3);
    // SAFETY: `RimeGetContext` populated a valid preedit C string.
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) };
    assert_eq!(preedit.to_str(), Ok("ni "));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_switch_key_handles_eisu_toggle() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-switch-key");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - ascii_composer
  segmentors:
    - abc_segmentor
ascii_composer:
  switch_key:
    Eisu_toggle: set_ascii_mode
",
    )
    .expect("schema config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);

    let eisu_toggle = CString::new("Eisu_toggle").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let eisu_toggle_keycode = unsafe { RimeGetKeycodeByName(eisu_toggle.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, eisu_toggle_keycode, 0), TRUE);

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_caps_lock_switch_key_clears_composition() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-caps-lock-switch-key");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - ascii_composer
  segmentors:
    - abc_segmentor
ascii_composer:
  switch_key:
    Caps_Lock: clear
",
    )
    .expect("schema config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let caps_lock = CString::new("Caps_Lock").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let caps_lock_keycode = unsafe { RimeGetKeycodeByName(caps_lock.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, caps_lock_keycode, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, caps_lock_keycode, K_RELEASE_MASK),
        FALSE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_switch_key_handles_shift_release_commit_code() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-shift-switch-key");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - ascii_composer
  segmentors:
    - abc_segmentor
ascii_composer:
  switch_key:
    Shift_L: commit_code
",
    )
    .expect("schema config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let shift_l = CString::new("Shift_L").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let shift_l_keycode = unsafe { RimeGetKeycodeByName(shift_l.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, shift_l_keycode, 0), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, shift_l_keycode, K_RELEASE_MASK),
        FALSE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` populated a valid C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    // SAFETY: commit text was allocated by `RimeGetCommit`.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_inline_ascii_mode_ends_with_composition() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-inline-ascii");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - ascii_composer
  segmentors:
    - abc_segmentor
ascii_composer:
  switch_key:
    Shift_L: inline_ascii
",
    )
    .expect("schema config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let shift_l = CString::new("Shift_L").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let shift_l_keycode = unsafe { RimeGetKeycodeByName(shift_l.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, shift_l_keycode, 0), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, shift_l_keycode, K_RELEASE_MASK),
        FALSE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 3);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    RimeClearComposition(session_id);
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_switch_key_falls_back_to_default_commit_text() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-default-switch-key");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
ascii_composer:
  switch_key:
    Shift_R: commit_text
",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - ascii_composer
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
translator:
  dictionary: luna
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni
尼\tni
",
    )
    .expect("dictionary should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let shift_r = CString::new("Shift_R").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let shift_r_keycode = unsafe { RimeGetKeycodeByName(shift_r.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, shift_r_keycode, 0), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, shift_r_keycode, K_RELEASE_MASK),
        FALSE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` populated a valid C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by `RimeGetCommit`.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_segmentor_uses_raw_tag_in_ascii_mode() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-segmentor");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - ascii_segmentor
    - abc_segmentor
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: luna
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni
",
    )
    .expect("dictionary should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let candidate_texts = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let menu = context.menu;
        let candidates = if menu.num_candidates > 0 {
            // SAFETY: `RimeGetContext` populated `menu.candidates` with
            // `num_candidates` initialized entries.
            unsafe { std::slice::from_raw_parts(menu.candidates, menu.num_candidates as usize) }
                .iter()
                .map(|candidate| {
                    // SAFETY: candidate text pointers are valid C strings
                    // owned by the context until `RimeFreeContext`.
                    unsafe { CStr::from_ptr(candidate.text) }
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        candidates
    };

    assert_eq!(candidate_texts(), ["你", "ni"]);
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(candidate_texts(), ["ni"]);
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
    assert_eq!(candidate_texts(), ["你", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn gets_and_selects_current_schema() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let schema_id = CString::new("sample_schema").expect("schema id should be valid");
    let mut buffer = vec![0 as c_char; 32];
    let mut short_buffer = vec![0 as c_char; 8];
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();
    let mut status = empty_status();

    // SAFETY: buffer points to writable storage.
    assert_eq!(
        unsafe { RimeGetCurrentSchema(session_id, buffer.as_mut_ptr(), buffer.len()) },
        TRUE
    );
    // SAFETY: `RimeGetCurrentSchema` wrote a trailing NUL into buffer.
    let current_schema = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(current_schema.to_str(), Ok("default"));

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    // SAFETY: schema id is a valid nul-terminated C string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    // SAFETY: selecting a schema clears unread composition state.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    // SAFETY: context points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);

    // SAFETY: buffer points to writable storage.
    assert_eq!(
        unsafe { RimeGetCurrentSchema(session_id, short_buffer.as_mut_ptr(), short_buffer.len()) },
        TRUE
    );
    // SAFETY: the raw byte view is bounded to the caller-owned buffer.
    let truncated_schema = unsafe {
        std::slice::from_raw_parts(short_buffer.as_ptr().cast::<u8>(), short_buffer.len())
    };
    assert_eq!(truncated_schema, b"sample_s");

    let mut zero_len_marker = b'?' as c_char;
    // SAFETY: librime's strncpy-based getter accepts a valid output pointer
    // with a zero-length buffer and leaves the pointed storage untouched.
    assert_eq!(
        unsafe { RimeGetCurrentSchema(session_id, &mut zero_len_marker, 0) },
        TRUE
    );
    assert_eq!(zero_len_marker, b'?' as c_char);

    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_name = unsafe { CStr::from_ptr(status.schema_name) };
    assert_eq!(status_schema_id.to_str(), Ok("sample_schema"));
    assert_eq!(status_schema_name.to_str(), Ok("sample_schema"));
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(
        unsafe { RimeGetCurrentSchema(session_id, std::ptr::null_mut(), 0) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, std::ptr::null()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeSelectSchema(session_id + 1, schema_id.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn select_schema_uses_deployed_schema_name_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("select-schema-name");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "schema:\n  schema_id: luna\n  name: Luna\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let mut status = empty_status();

    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_name = unsafe { CStr::from_ptr(status.schema_name) };
    assert_eq!(status_schema_id.to_str(), Ok("luna"));
    assert_eq!(status_schema_name.to_str(), Ok("Luna"));
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_switch_reset_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("select-schema-switch-reset");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
switches:
  - name: ascii_mode
    reset: 1
  - name: full_shape
    reset: 0
  - options: [simplification, traditional]
    reset: 1
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    let mut status = empty_status();

    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );

    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_ascii_mode, TRUE);
    assert_eq!(status.is_full_shape, FALSE);
    assert_eq!(status.is_simplified, FALSE);
    assert_eq!(status.is_traditional, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_switch_translator_candidates() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - switch_translator
    - echo_translator
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    reset: 1
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ 西文".to_owned()),
            ("简体".to_owned(), String::new()),
            ("繁體".to_owned(), " ✓".to_owned()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_preserves_missing_state_indices_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-missing-states");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - switch_translator
    - echo_translator
switches:
  - name: ascii_mode
    states: [中文]
  - options: [simplification, traditional, emoji]
    states: [简体, ~, 表情]
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ ".to_owned()),
            ("简体".to_owned(), " ✓".to_owned()),
            ("表情".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 2), TRUE);
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    let emoji = CString::new("emoji").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(unsafe { RimeGetOption(session_id, emoji.as_ptr()) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_persists_librime_save_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-save-options");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - switch_translator
    - echo_translator
switcher:
  save_options: [ascii_mode, simplification, traditional]
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    reset: 0
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);

    let mut user_config = empty_config();
    let user_id = CString::new("user").expect("config id should be valid");
    // SAFETY: config id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeUserConfigOpen(user_id.as_ptr(), &mut user_config) },
        TRUE
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/ascii_mode"),
        Some(TRUE)
    );
    // SAFETY: config owns state allocated by the shim.
    assert_eq!(unsafe { RimeConfigClose(&mut user_config) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'y' as c_int, 0), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 2), TRUE);

    let mut user_config = empty_config();
    // SAFETY: config id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeUserConfigOpen(user_id.as_ptr(), &mut user_config) },
        TRUE
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/ascii_mode"),
        Some(TRUE)
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/simplification"),
        Some(FALSE)
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/traditional"),
        Some(TRUE)
    );
    // SAFETY: config owns state allocated by the shim.
    assert_eq!(unsafe { RimeConfigClose(&mut user_config) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_restores_librime_switcher_saved_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switcher-restore-save-options");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
switcher:
  save_options: [ascii_mode, full_shape, simplification, traditional]
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - name: full_shape
    states: [半角, 全角]
  - options: [simplification, traditional]
    states: [简体, 繁體]
",
    )
    .expect("schema config should be written");
    fs::write(
        user.join("user.yaml"),
        "\
var:
  option:
    ascii_mode: true
    full_shape: 'true'
    simplification: false
    traditional: true
",
    )
    .expect("user config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_normalizes_radio_group_selection() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-radio-default");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - switch_translator
    - echo_translator
switches:
  - options: [simplification, traditional]
    states: [简体, 繁體]
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("简体".to_owned(), " ✓".to_owned()),
            ("繁體".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    // SAFETY: option names are valid NUL-terminated strings.
    unsafe {
        RimeSetOption(session_id, simplification.as_ptr(), TRUE);
        RimeSetOption(session_id, traditional.as_ptr(), TRUE);
    }
    RimeClearComposition(session_id);
    assert_eq!(RimeProcessKey(session_id, 'y' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_folds_and_unfolds_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-options");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - switch_translator
    - echo_translator
switcher:
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
  abbreviate_options: 'true'
switches:
  - name: ascii_mode
    states: [中文, 西文]
    abbrev: [中, 西]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    abbrev: [简, 繁]
    reset: 1
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, fold_options.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中/繁]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, fold_options.as_ptr()) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let composition_input = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_string_lossy()
        .into_owned();
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(composition_input, "x");
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ 西文".to_owned()),
            ("简体".to_owned(), String::new()),
            ("繁體".to_owned(), " ✓".to_owned()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_honors_librime_fold_options_default() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-options-default");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - switch_translator
    - echo_translator
switcher:
  fold_options: 'true'
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
  abbreviate_options: true
switches:
  - name: ascii_mode
    states: [中文, 西文]
    abbrev: [中, 西]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    abbrev: [简, 繁]
    reset: 1
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, fold_options.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中/繁]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_folds_default_radio_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-radio-default");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - switch_translator
    - echo_translator
switcher:
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
switches:
  - name: ascii_mode
    states: [中文, 西文]
  - options: [simplification, traditional]
    states: [简体, 繁體]
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    unsafe { RimeSetOption(session_id, fold_options.as_ptr(), TRUE) };
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中文/简体]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );
    // SAFETY: option names are valid NUL-terminated strings. Librime selects
    // the first radio option while constructing the visible switch menu.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_dictionary_packs() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-dictionary-packs");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  packs:
    - luna_pack
    - missing_pack
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
...

八\tba\t1
",
    )
    .expect("primary dictionary should be written");
    fs::write(
        shared.join("luna_pack.dict.yaml"),
        "\
---
name: luna_pack
version: '0.1'
sort: original
columns: [code, text, weight]
...

ba\t爸\t9
ba\t吧\t3
",
    )
    .expect("pack dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ba".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["爸", "吧", "八", "ba"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_uses_preset_vocabulary_for_dictionary_weights() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-preset-vocabulary");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
use_preset_vocabulary: true
...

八\tba
吧\tba\t50%
爸\tba\t1
",
    )
    .expect("dictionary should be written");
    fs::write(
        shared.join("essay.txt"),
        "\
八\t8
吧\t6
",
    )
    .expect("preset vocabulary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ba".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["八", "吧", "爸", "ba"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_encodes_rule_based_dictionary_and_preset_phrases() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-encoder-phrase-injection");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
use_preset_vocabulary: true
max_phrase_length: 2
min_phrase_weight: 10
encoder:
  rules:
    - length_equal: 2
      formula: \"AaBa\"
...

你\tni\t10
好\thao\t9
您\tnin\t8
你好\t\t50%
",
    )
    .expect("dictionary should be written");
    fs::write(
        shared.join("essay.txt"),
        "\
您好\t11
你好啊\t20
低频\t9
",
    )
    .expect("preset vocabulary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "nh".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["您好", "你好", "nh"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_namespaced_librime_table_translator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-namespaced-table-translator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator@custom_table
translator:
  dictionary: base
custom_table:
  dictionary: custom
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("base.dict.yaml"),
        "\
---
name: base
version: '0.1'
sort: by_weight
...

基\tji\t9
",
    )
    .expect("default dictionary should be written");
    fs::write(
        shared.join("custom.dict.yaml"),
        "\
---
name: custom
version: '0.1'
sort: by_weight
...

机\tji\t9
",
    )
    .expect("custom dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ji".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["机", "ji"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_preserves_librime_translator_prescription_order() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-order");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - script_translator@first_table
    - table_translator@second_table
first_table:
  dictionary: first
  enable_completion: false
second_table:
  dictionary: second
  enable_completion: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("first.dict.yaml"),
        "\
---
name: first
version: '0.1'
sort: original
...

先\txu\t0
",
    )
    .expect("first dictionary should be written");
    fs::write(
        shared.join("second.dict.yaml"),
        "\
---
name: second
version: '0.1'
sort: original
...

后\txu\t0
",
    )
    .expect("second dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "xu".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["先", "后", "xu"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_preserves_librime_filter_prescription_order() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-filter-order");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - uniquifier
    - simplifier
translator:
  dictionary: luna
  enable_completion: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
columns: [code, text, weight]
...

tw\t臺\t0
tw\t台\t0
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let option = CString::new("simplification").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["台", "台", "tw"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_uniquifier_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-uniquifier-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - uniquifier
translator:
  dictionary: luna
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ni\t你\t9
ni\t你\t8
ni\t呢\t7
ni\tni\t6
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["你", "呢", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_single_char_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-single-char-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - single_char_filter
translator:
  dictionary: luna
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ni\t你好\t9
ni\t你\t8
ni\t呢\t7
ni\t你们\t6
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["你", "呢", "你好", "你们", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_charset_filter_alias() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-charset-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - cjk_minifier
translator:
  dictionary: luna
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ni\t你\t9
ni\t㐀\t8
ni\t𠀀\t7
ni\t㍿\t6
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let candidate_texts = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        texts
    };

    assert_eq!(candidate_texts(), ["你", "ni"]);

    let option = CString::new("extended_charset").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    assert_eq!(candidate_texts(), ["你", "㐀", "𠀀", "㍿", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_namespaced_librime_charset_filter_alias() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-namespaced-charset-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - cjk_minifier@charset_guard
translator:
  dictionary: luna
charset_guard:
  tags: [abc]
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ni\t你\t9
ni\t𠀀\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["你", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_charset_filter_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-charset-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  enable_charset_filter: true
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ni\t你\t9
ni\t㐀\t8
ni\t𠀀\t7
ni\t㍿\t6
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let candidate_texts = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        texts
    };

    assert_eq!(candidate_texts(), ["你", "ni"]);

    let option = CString::new("extended_charset").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    assert_eq!(candidate_texts(), ["你", "㐀", "𠀀", "㍿", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_completion_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-completion");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("complete.schema.yaml"),
        "\
schema:
  schema_id: complete
  name: Complete
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
",
    )
    .expect("completion schema config should be written");
    fs::write(
        staging.join("exact.schema.yaml"),
        "\
schema:
  schema_id: exact
  name: Exact
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  enable_completion: false
",
    )
    .expect("exact schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ba\t爸\t9
ban\t班\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);

        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts
    };

    assert_eq!(candidate_texts_for("complete"), ["爸", "班", "b"]);
    assert_eq!(candidate_texts_for("exact"), ["b"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_tag_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-tags");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("custom-tag.schema.yaml"),
        "\
schema:
  schema_id: custom-tag
  name: Custom Tag
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  tag: custom
",
    )
    .expect("custom-tag schema config should be written");
    fs::write(
        staging.join("abc-tags.schema.yaml"),
        "\
schema:
  schema_id: abc-tags
  name: ABC Tags
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  tag: custom
  tags: [abc]
",
    )
    .expect("abc-tags schema config should be written");
    fs::write(
        staging.join("abc-extra-tags.schema.yaml"),
        "\
schema:
  schema_id: abc-extra-tags
  name: ABC Extra Tags
engine:
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
abc_segmentor:
  extra_tags: [custom]
translator:
  dictionary: luna
  tag: custom
",
    )
    .expect("abc-extra-tags schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ba\t爸\t9
ban\t班\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);

        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts
    };

    assert_eq!(candidate_texts_for("custom-tag"), ["b"]);
    assert_eq!(candidate_texts_for("abc-tags"), ["爸", "班", "b"]);
    assert_eq!(candidate_texts_for("abc-extra-tags"), ["爸", "班", "b"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_filter_tag_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-filter-tags");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("blocked.schema.yaml"),
        "\
schema:
  schema_id: blocked
  name: Blocked
engine:
  translators:
    - table_translator
  filters:
    - simplifier@zh_simp
translator:
  dictionary: luna
  enable_completion: false
zh_simp:
  option_name: zh_simp
  tags: [custom]
",
    )
    .expect("blocked schema config should be written");
    fs::write(
        staging.join("matched.schema.yaml"),
        "\
schema:
  schema_id: matched
  name: Matched
engine:
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
  filters:
    - simplifier@zh_simp
abc_segmentor:
  extra_tags: [custom]
translator:
  dictionary: luna
  enable_completion: false
zh_simp:
  option_name: zh_simp
  tags: [custom]
",
    )
    .expect("matched schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

tw\t臺灣\t9
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        let option = CString::new("zh_simp").expect("option name should be valid");
        // SAFETY: option is a valid NUL-terminated string.
        unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };
        for ch in "tw".chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }

        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts
    };

    assert_eq!(candidate_texts_for("blocked"), ["臺灣", "tw"]);
    assert_eq!(candidate_texts_for("matched"), ["台湾", "tw"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_script_translator_word_completion_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-script-word-completion");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("script.schema.yaml"),
        "\
schema:
  schema_id: script
  name: Script
engine:
  translators:
    - script_translator
translator:
  dictionary: luna
  enable_completion: false
  enable_word_completion: true
",
    )
    .expect("script schema config should be written");
    fs::write(
        staging.join("r10n.schema.yaml"),
        "\
schema:
  schema_id: r10n
  name: R10n
engine:
  translators:
    - r10n_translator
translator:
  dictionary: luna
  enable_completion: true
  enable_word_completion: false
",
    )
    .expect("r10n schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ba\t爸\t9
ban\t班\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);

        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts
    };

    assert_eq!(candidate_texts_for("script"), ["爸", "班", "b"]);
    assert_eq!(candidate_texts_for("r10n"), ["b"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_table_translator_sentence_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-table-sentence-options");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("sentence.schema.yaml"),
        "\
schema:
  schema_id: sentence
  name: Sentence
engine:
  translators:
    - table_translator@default_table
    - table_translator@disabled_table
    - table_translator@over_table
default_table:
  dictionary: default_dict
  enable_completion: false
disabled_table:
  dictionary: disabled_dict
  enable_completion: false
  enable_sentence: false
over_table:
  dictionary: over_dict
  sentence_over_completion: true
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("default_dict.dict.yaml"),
        "\
---
name: default_dict
version: '0.1'
sort: original
columns: [code, text]
...

ba\t爸
bao\t包
",
    )
    .expect("default dictionary should be written");
    fs::write(
        shared.join("disabled_dict.dict.yaml"),
        "\
---
name: disabled_dict
version: '0.1'
sort: original
columns: [code, text]
...

ca\t擦
cao\t草
",
    )
    .expect("disabled dictionary should be written");
    fs::write(
        shared.join("over_dict.dict.yaml"),
        "\
---
name: over_dict
version: '0.1'
sort: original
columns: [code, text]
...

da\t大
dadan\t大单
",
    )
    .expect("over dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("sentence").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'o' as c_int, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let first_text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8");
    let first_comment = unsafe { CStr::from_ptr(candidates[0].comment) }
        .to_str()
        .expect("candidate comment should be valid UTF-8");
    assert_eq!(first_text, "爸包");
    assert_eq!(first_comment, " ☯ ");
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    RimeClearComposition(session_id);
    assert_eq!(RimeProcessKey(session_id, 'c' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'c' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'o' as c_int, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, ["cacao"]);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    RimeClearComposition(session_id);
    assert_eq!(RimeProcessKey(session_id, 'd' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'd' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, ["大大", "大单", "dada"]);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_initial_quality() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-initial-quality");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator@low_table
    - table_translator@high_table
low_table:
  dictionary: low
  enable_completion: false
high_table:
  dictionary: high
  enable_completion: false
  initial_quality: 10
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("low.dict.yaml"),
        "\
---
name: low
version: '0.1'
sort: original
columns: [code, text]
...

ba\t低
",
    )
    .expect("low dictionary should be written");
    fs::write(
        shared.join("high.dict.yaml"),
        "\
---
name: high
version: '0.1'
sort: original
columns: [code, text]
...

ba\t高
",
    )
    .expect("high dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ba".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["高", "低", "ba"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_r10n_translator_alias() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-r10n-translator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - r10n_translator
translator:
  dictionary: luna
  enable_completion: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
columns: [code, text]
...

ni\t你
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["你", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_history_translator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-history-translator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
    - history_translator
translator:
  dictionary: luna
  enable_completion: false
history:
  input: his
  size: 2
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
columns: [code, text]
...

ni\t你
hao\t好
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for input in ["ni", "hao"] {
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), TRUE);
    }
    for ch in "his".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["好", "你", "his"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_treats_librime_history_translator_translator_namespace_as_history() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-history-translator-namespace");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
    - history_translator@translator
translator:
  dictionary: luna
  enable_completion: false
  input: ignored
history:
  input: his
  size: 1
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
columns: [code, text]
...

ni\t你
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), TRUE);
    for ch in "his".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["你", "his"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_history_translator_tag_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-history-translator-tag");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("allowed.schema.yaml"),
        "\
schema:
  schema_id: allowed
  name: Allowed
engine:
  translators:
    - table_translator
    - history_translator
translator:
  dictionary: luna
  enable_completion: false
history:
  input: his
  tag: abc
",
    )
    .expect("allowed schema config should be written");
    fs::write(
        staging.join("blocked.schema.yaml"),
        "\
schema:
  schema_id: blocked
  name: Blocked
engine:
  translators:
    - table_translator
    - history_translator
translator:
  dictionary: luna
  enable_completion: false
history:
  input: his
  tag: custom
",
    )
    .expect("blocked schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
columns: [code, text]
...

ni\t你
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        for ch in "ni".chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), TRUE);
        for ch in "his".chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }

        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts
    };

    assert_eq!(candidate_texts_for("allowed"), ["你", "his"]);
    assert_eq!(candidate_texts_for("blocked"), ["his"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_parses_librime_history_translator_numeric_scalars() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-history-translator-scalars");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
    - history_translator
translator:
  dictionary: luna
  enable_completion: false
history:
  input: his
  size: '2'
  initial_quality: '-10.0'
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
columns: [code, text, weight]
...

his\t表\t0
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for input in ["ni", "hao"] {
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), TRUE);
    }
    for ch in "his".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["表", "his", "hao", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_comment_format() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-comment-format");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  comment_format:
    - xlit/ab/AB/
    - xform/^/[/
    - xform/$/]/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ba\t爸\t9
ban\t班\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let comments = candidates
        .iter()
        .map(|candidate| {
            if candidate.comment.is_null() {
                return String::new();
            }
            // SAFETY: candidate comment pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.comment) }
                .to_str()
                .expect("candidate comment should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(comments, ["[BA]", "[BAn]", "echo"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_dictionary_exclude() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-dictionary-exclude");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  dictionary_exclude:
    - 爸
    - 班
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ba\t爸\t9
ban\t班\t8
bao\t包\t7
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["包", "b"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_delimiter_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-delimiter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
speller:
  delimiter: \"'\"
translator:
  dictionary: luna
  enable_completion: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ba\t爸\t9
ban\t班\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ba'".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_str()
        .expect("preedit should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["爸", "ba'"]);
    assert_eq!(preedit, "ba'");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_simplifier_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-simplifier-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - simplifier@zh_simp
translator:
  dictionary: luna
zh_simp:
  option_name: zh_simp
  tips: all
  comment_format:
    - xform/^/〔/
    - xform/$/〕/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

tw\t臺灣\t9
tw\t龍馬\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let candidate_pairs = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                let text = unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned();
                let comment = if candidate.comment.is_null() {
                    String::new()
                } else {
                    // SAFETY: candidate comment pointers are populated by `RimeGetContext`.
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned()
                };
                (text, comment)
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        texts
    };

    assert_eq!(
        candidate_pairs(),
        [
            ("臺灣".to_owned(), "tw".to_owned()),
            ("龍馬".to_owned(), "tw".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );

    let option = CString::new("zh_simp").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    assert_eq!(
        candidate_pairs(),
        [
            ("台湾".to_owned(), "〔臺灣〕".to_owned()),
            ("龙马".to_owned(), "〔龍馬〕".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_treats_librime_simplifier_filter_namespace_as_simplifier() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-simplifier-filter-namespace");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - simplifier@filter
translator:
  dictionary: luna
simplifier:
  option_name: zh_simp
  tips: all
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

tw\t臺灣\t9
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let option = CString::new("zh_simp").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8")
        .to_owned();
    let comment = unsafe { CStr::from_ptr(candidates[0].comment) }
        .to_str()
        .expect("candidate comment should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(text, "台湾");
    assert_eq!(comment, "〔臺灣〕");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_simplifier_opencc_config() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-simplifier-opencc-config");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - simplifier@zh_tw
translator:
  dictionary: luna
zh_tw:
  option_name: zh_tw
  opencc_config: t2tw.json
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

tw\t台灣\t9
tw\t裏\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let candidate_texts = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        texts
    };

    assert_eq!(candidate_texts(), ["台灣", "裏", "tw"]);

    let option = CString::new("zh_tw").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    assert_eq!(candidate_texts(), ["臺灣", "裡", "tw"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_simplifier_excluded_types() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-simplifier-excluded-types");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - simplifier@zh_simp
translator:
  dictionary: luna
zh_simp:
  option_name: zh_simp
  tips: all
  excluded_types:
    - table
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

tw\t臺灣\t9
tw\t龍馬\t8
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let option = CString::new("zh_simp").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_str()
                    .expect("candidate comment should be valid UTF-8")
                    .to_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        pairs,
        [
            ("臺灣".to_owned(), "tw".to_owned()),
            ("龍馬".to_owned(), "tw".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_reverse_lookup_translator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-translator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
  translators:
    - reverse_lookup_translator
    - table_translator
abc_segmentor:
  extra_tags: [reverse_lookup]
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
  comment_format:
    - xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/
    - xform/^/〔/
    - xform/$/〕/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

火\tho
火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("〔HO HUO〕".to_owned())),
            ("`huo".to_owned(), Some("echo".to_owned()))
        ]
    );

    RimeClearComposition(session_id);
    for ch in "huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("huo".to_owned())),
            ("火".to_owned(), Some("〔HO HUO〕".to_owned())),
            ("huo".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_matcher_segmentor_adds_librime_recognizer_pattern_tags() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-matcher-segmentor-tags");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
    - matcher
  translators:
    - reverse_lookup_translator
    - table_translator
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
recognizer:
  patterns:
    reverse_lookup: \"`[a-z]*'?$\"
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["火".to_owned(), "`huo".to_owned()]);

    RimeClearComposition(session_id);
    for ch in "huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["火".to_owned(), "huo".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_affix_segmentor_tags_librime_prefixed_lookup() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-affix-segmentor-tags");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
    - affix_segmentor@reverse_lookup
  translators:
    - reverse_lookup_translator
reverse_lookup:
  tag: reverse_lookup
  dictionary: stroke
  prefix: \"`\"
  suffix: \"'\"
  extra_tags: [lookup_extra]
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo'".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["火".to_owned(), "`huo'".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_affix_segmentor_tags_are_exclusive_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-affix-segmentor-exclusive-tags");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
    - affix_segmentor@reverse_lookup
  translators:
    - reverse_lookup_translator
    - table_translator
translator:
  dictionary: luna
reverse_lookup:
  tag: reverse_lookup
  dictionary: stroke
  prefix: \"`\"
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

桌\thuo
",
    )
    .expect("table dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["火".to_owned(), "`huo".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_matcher_segmentor_uses_librime_sorted_recognizer_pattern_tags() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-matcher-segmentor-pattern-order");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
    - matcher
  translators:
    - reverse_lookup_translator
    - table_translator
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
recognizer:
  patterns:
    z_custom: \"`[a-z]*$\"
    reverse_lookup: \"`[a-z]*$\"
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["火".to_owned(), "`huo".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_matcher_segmentor_segmentor_namespace_reads_recognizer_patterns() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-matcher-segmentor-namespace");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
    - matcher@segmentor
  translators:
    - reverse_lookup_translator
    - table_translator
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
recognizer:
  patterns:
    reverse_lookup: \"`[a-z]*'?$\"
segmentor:
  patterns:
    reverse_lookup: \"^never$\"
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["火".to_owned(), "`huo".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_recognizer_processor_accepts_space_for_librime_patterns() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-recognizer-processor-space");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - recognizer@processor
  segmentors:
    - abc_segmentor
    - matcher
  translators:
    - reverse_lookup_translator
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
recognizer:
  use_space: 'true'
  patterns:
    reverse_lookup: \"`[a-z ]*$\"
processor:
  use_space: false
  patterns:
    reverse_lookup: \"^never$\"
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火山\thuo shan
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    assert_eq!(RimeProcessKey(session_id, ' ' as c_int, 0), TRUE);
    let input = unsafe { CStr::from_ptr(RimeGetInput(session_id)) }
        .to_str()
        .expect("input should be valid UTF-8");
    assert_eq!(input, "`huo ");
    let mut no_commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as c_int,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut no_commit) }, FALSE);

    for ch in "shan".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(texts, ["火山".to_owned(), "`huo shan".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_treats_librime_reverse_lookup_translator_namespace_as_reverse_lookup() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-translator-namespace");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
  translators:
    - reverse_lookup_translator@translator
    - table_translator
abc_segmentor:
  extra_tags: [reverse_lookup]
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
  comment_format:
    - xform/^/stroke:/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

火\tho
火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("stroke:ho huo".to_owned())),
            ("`huo".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_reverse_lookup_translator_honors_librime_tag() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-translator-tag");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
  translators:
    - reverse_lookup_translator
abc_segmentor:
  extra_tags: [custom_lookup]
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
  tag: custom_lookup
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("huo".to_owned())),
            ("`huo".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_reverse_lookup_completion_when_enabled() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-completion");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
  translators:
    - reverse_lookup_translator
    - table_translator
abc_segmentor:
  extra_tags: [reverse_lookup]
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
  enable_completion: true
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

火\tho
火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
水\tshui
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`hu".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("ho huo".to_owned())),
            ("`hu".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_reverse_lookup_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - reverse_lookup_filter
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  overwrite_comment: true
  comment_format:
    - xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/
    - xform/^/〔/
    - xform/$/〕/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

你\twq
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("你".to_owned(), Some("〔WQ〕".to_owned())),
            ("ni".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_namespaced_librime_reverse_lookup_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-namespaced-reverse-lookup-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - reverse_lookup_filter@stroke_lookup
translator:
  dictionary: luna
stroke_lookup:
  dictionary: stroke
  overwrite_comment: true
  comment_format:
    - xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

你\twq
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8")
        .to_owned();
    let comment = unsafe { CStr::from_ptr(candidates[0].comment) }
        .to_str()
        .expect("candidate comment should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(text, "你");
    assert_eq!(comment, "WQ");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_reverse_lookup_filter_filter_namespace_alias() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-filter-alias");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
  filters:
    - reverse_lookup_filter@filter
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  overwrite_comment: true
  comment_format:
    - xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/
filter:
  dictionary: wrong
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

你\twq
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ni".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let first_text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8")
        .to_owned();
    let first_comment = unsafe { CStr::from_ptr(candidates[0].comment) }
        .to_str()
        .expect("candidate comment should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(first_text, "你");
    assert_eq!(first_comment, "WQ");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_reverse_lookup_filter_updates_sentence_candidates() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-filter-sentence");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("sentence.schema.yaml"),
        "\
schema:
  schema_id: sentence
  name: Sentence
engine:
  translators:
    - table_translator
  filters:
    - reverse_lookup_filter
translator:
  dictionary: sentence
  enable_completion: false
reverse_lookup:
  dictionary: stroke
  overwrite_comment: true
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("sentence.dict.yaml"),
        "\
---
name: sentence
version: '0.1'
sort: original
columns: [code, text]
...

ba\t爸
bao\t包
",
    )
    .expect("sentence dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

爸包\tbb
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("sentence").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "babao".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let first_text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8")
        .to_owned();
    let first_comment = unsafe { CStr::from_ptr(candidates[0].comment) }
        .to_str()
        .expect("candidate comment should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(first_text, "爸包");
    assert_eq!(first_comment, "bb");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_punctuator_shape_and_symbol_definitions() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - punct_translator
    - echo_translator
punctuator:
  half_shape:
    \"/\": [\"、\", \"/\"]
    \"!\": { commit: \"！\" }
    \"(\": { pair: [\"（\", \"）\"] }
  full_shape:
    \"/\": \"／\"
    \"!\": { commit: \"！\" }
    \"(\": { pair: [\"〔\", \"〕\"] }
  symbols:
    \"/\": [\"symbol-slash\"]
    \"/fh\": [\"©\", \"®\"]
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let candidate_texts = |input: &str| {
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as i32, 0), TRUE);
        }
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by
                // `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        RimeClearComposition(session_id);
        texts
    };

    assert_eq!(candidate_texts("/"), ["、", "/", "/"]);
    assert_eq!(candidate_texts("!"), ["！", "!"]);
    assert_eq!(candidate_texts("("), ["（", "）", "("]);
    assert_eq!(candidate_texts("/fh"), ["©", "®", "/fh"]);

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };

    assert_eq!(candidate_texts("/"), ["／", "/"]);
    assert_eq!(candidate_texts("!"), ["！", "!"]);
    assert_eq!(candidate_texts("("), ["〔", "〕", "("]);
    assert_eq!(candidate_texts("/fh"), ["©", "®", "/fh"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_candidates_expose_librime_shape_comments() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-comments");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - punct_translator
    - echo_translator
punctuator:
  half_shape:
    \"/\": [\"/\", \"、\", \"©\"]
  full_shape:
    \"/\": \"／\"
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let candidate_comments = || {
        assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let comments = candidates
            .iter()
            .map(|candidate| {
                if candidate.comment.is_null() {
                    None
                } else {
                    Some(
                        // SAFETY: non-null candidate comment pointers are
                        // populated by `RimeGetContext`.
                        unsafe { CStr::from_ptr(candidate.comment) }
                            .to_str()
                            .expect("candidate comment should be valid UTF-8")
                            .to_owned(),
                    )
                }
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        RimeClearComposition(session_id);
        comments
    };

    assert_eq!(
        candidate_comments(),
        [
            Some("〔半角〕".to_owned()),
            Some("〔全角〕".to_owned()),
            None,
            Some("echo".to_owned())
        ]
    );

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    assert_eq!(
        candidate_comments(),
        [Some("〔全角〕".to_owned()), Some("echo".to_owned())]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punct_segmentor_tags_punctuation_exclusively() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punct-segmentor-exclusive");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - abc_segmentor
    - punct_segmentor
  translators:
    - punct_translator
    - table_translator
    - echo_translator
translator:
  dictionary: luna
punctuator:
  half_shape:
    \".\": \"。\"
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

DOT\t.\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["。".to_owned(), ".".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punct_segmentor_translates_digit_separator_as_number_punctuation() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punct-segmentor-number");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  segmentors:
    - punct_segmentor
  translators:
    - punct_translator
    - echo_translator
punctuator:
  digit_separators: \".:\"
  half_shape:
    \".\": \"。\"
  full_shape:
    \".\": \"。\"
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let top_candidate = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let text = unsafe { CStr::from_ptr(candidates[0].text) }
            .to_str()
            .expect("candidate text should be valid UTF-8")
            .to_owned();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        text
    };

    assert_eq!(RimeProcessKey(session_id, '1' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(top_candidate(), ".");

    RimeClearComposition(session_id);
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(top_candidate(), "．");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_fallback_segmentor_tags_unclaimed_input_as_raw() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-fallback-segmentor-raw");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("raw.schema.yaml"),
        "\
schema:
  schema_id: raw
  name: Raw
engine:
  segmentors:
    - fallback_segmentor
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: raw
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("raw.dict.yaml"),
        "\
---
name: raw
version: '0.1'
sort: original
...

Alpha\ta\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("raw").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["a".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_serializes_chord_on_key_release() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: ba
  output_format:
    - xlit/ab/xy/
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

醒\tyx\t100
形\txy\t90
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let current_input = || {
        let input = RimeGetInput(session_id);
        assert!(!input.is_null());
        // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
        unsafe { CStr::from_ptr(input) }
            .to_str()
            .expect("input should be valid UTF-8")
            .to_owned()
    };

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(current_input(), "");
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(current_input(), "");
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(current_input(), "yx");

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert!(!candidates.is_empty());
    // SAFETY: candidate text pointers are populated by `RimeGetContext`.
    let top_text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(top_text, "醒");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_exposes_prompt_while_chording() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-prompt");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: ba
  algebra:
    - xlit/ab/xy/
  prompt_format:
    - xform/^(.+)$/<$1>/
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\tx\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 3);
    assert_eq!(context.composition.cursor_pos, 0);
    assert_eq!(context.composition.sel_start, 0);
    assert_eq!(context.composition.sel_end, 0);
    // SAFETY: context composition preedit was allocated by `RimeGetContext`.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("<x>")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: context composition preedit was allocated by `RimeGetContext`.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("x")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_cancels_active_chord_on_function_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-function-cancel");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: a
  output_format:
    - xlit/a/x/
  prompt_format:
    - xform/^(.+)$/<$1>/
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\tx\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: context composition preedit was allocated by `RimeGetContext`.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("<a>")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), FALSE);

    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, FALSE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK),
        FALSE
    );

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_binding_commits_raw_sequence() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-raw-binding");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: ab
  output_format:
    - xlit/ab/xy/
  bindings:
    Control+r: commit_raw_input
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\txy\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, K_RELEASE_MASK), TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("xy"));

    assert_eq!(RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ab"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_clears_raw_sequence_after_context_commit() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-context-commit-clears-raw");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: ab
  output_format:
    - xlit/ab/xy/
  bindings:
    Control+r: commit_raw_input
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\txy\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, K_RELEASE_MASK), TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("xy"));

    assert_eq!(RimeCommitComposition(session_id), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("形"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(
        RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK),
        FALSE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_clears_raw_sequence_after_direct_commit_output() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-direct-commit-clears-raw");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
chord_composer:
  alphabet: a
  output_format:
    - \"xform/^a$/ /\"
  bindings:
    Control+r: commit_raw_input
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);

    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok(" "));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(
        RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK),
        FALSE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_honors_modifier_use_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-modifiers");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");

    let schema = |schema_id: &str, use_option: &str| {
        let use_option = if use_option.is_empty() {
            String::new()
        } else {
            format!("  {use_option}: true\n")
        };
        format!(
            "\
schema:
  schema_id: {schema_id}
  name: {schema_id}
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: a
{use_option}  output_format:
    - xlit/a/x/
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
"
        )
    };
    fs::write(staging.join("plain.schema.yaml"), schema("plain", ""))
        .expect("plain schema should be written");
    fs::write(
        staging.join("control.schema.yaml"),
        schema("control", "use_control"),
    )
    .expect("control schema should be written");
    fs::write(
        staging.join("shift.schema.yaml"),
        schema("shift", "use_shift"),
    )
    .expect("shift schema should be written");
    fs::write(staging.join("alt.schema.yaml"), schema("alt", "use_alt"))
        .expect("alt schema should be written");
    fs::write(
        staging.join("super.schema.yaml"),
        schema("super", "use_super"),
    )
    .expect("super schema should be written");
    fs::write(staging.join("caps.schema.yaml"), schema("caps", "use_caps"))
        .expect("caps schema should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\tx\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let current_input = || {
        let input = RimeGetInput(session_id);
        assert!(!input.is_null());
        // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
        unsafe { CStr::from_ptr(input) }
            .to_str()
            .expect("input should be valid UTF-8")
            .to_owned()
    };

    let plain_schema = CString::new("plain").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, plain_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK | K_RELEASE_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_LOCK_MASK), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_LOCK_MASK | K_RELEASE_MASK),
        FALSE
    );
    assert_eq!(current_input(), "");

    let control_schema = CString::new("control").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, control_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    let shift_schema = CString::new("shift").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, shift_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_SHIFT_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_SHIFT_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    let alt_schema = CString::new("alt").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, alt_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_ALT_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_ALT_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    let super_schema = CString::new("super").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, super_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_SUPER_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_SUPER_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    let caps_schema = CString::new("caps").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, caps_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_LOCK_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_LOCK_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_express_editor_return_commits_raw_input() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-express-editor-return");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("fluid.schema.yaml"),
        "\
schema:
  schema_id: fluid
  name: Fluid
engine:
  processors:
    - speller
    - fluid_editor
  translators:
    - table_translator
speller:
  alphabet: in
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("fluid schema config should be written");
    fs::write(
        staging.join("express.schema.yaml"),
        "\
schema:
  schema_id: express
  name: Express
engine:
  processors:
    - speller
    - express_editor
  translators:
    - table_translator
speller:
  alphabet: in
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("express schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);

    let commit_text = |session_id| {
        let mut commit = RimeCommit {
            data_size: std::mem::size_of::<RimeCommit>() as i32,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to writable storage.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was allocated by the shim above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };

    let fluid_session = RimeCreateSession();
    let fluid_schema = CString::new("fluid").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(fluid_session, fluid_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(fluid_session, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(fluid_session, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(fluid_session, return_keycode, 0), TRUE);
    assert_eq!(commit_text(fluid_session), "你");
    assert_eq!(RimeDestroySession(fluid_session), TRUE);

    let express_session = RimeCreateSession();
    let express_schema = CString::new("express").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(express_session, express_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(express_session, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(express_session, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(express_session, return_keycode, 0), TRUE);
    assert_eq!(commit_text(express_session), "ni");
    let input = RimeGetInput(express_session);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(RimeDestroySession(express_session), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_editor_bindings_override_default_keymap() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-editor-bindings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - speller
    - fluid_editor
  translators:
    - table_translator
speller:
  alphabet: abcni
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
editor:
  bindings:
    Return: noop
    Control+r: commit_raw_input
    Control+d: delete_candidate
    Control+x: delete
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni\t100
呢\tni\t90
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);

    let commit_text = |session_id| {
        let mut commit = RimeCommit {
            data_size: std::mem::size_of::<RimeCommit>() as i32,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to writable storage.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was allocated by the shim above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, return_keycode, 0), TRUE);
    let mut empty_commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(session_id, &mut empty_commit) },
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'd' as i32, K_CONTROL_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(commit_text(session_id), "呢");

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK), TRUE);
    assert_eq!(commit_text(session_id), "ni");

    let raw_input = CString::new("abc").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeSetInput(session_id, raw_input.as_ptr()) },
        TRUE
    );
    RimeSetCaretPos(session_id, 1);
    assert_eq!(RimeProcessKey(session_id, 'x' as i32, K_CONTROL_MASK), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ac"));

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_editor_char_handler_controls_printable_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-editor-char-handler");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");

    let schema = |schema_id: &str, processor: &str, char_handler: Option<&str>| {
        let editor_config = char_handler
            .map(|handler| format!("editor:\n  char_handler: {handler}\n"))
            .unwrap_or_default();
        format!(
            "\
schema:
  schema_id: {schema_id}
  name: {schema_id}
engine:
  processors:
    - {processor}
  translators:
    - table_translator
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
{editor_config}"
        )
    };
    fs::write(
        staging.join("fluid.schema.yaml"),
        schema("fluid", "fluid_editor", None),
    )
    .expect("fluid schema config should be written");
    fs::write(
        staging.join("express.schema.yaml"),
        schema("express", "express_editor", None),
    )
    .expect("express schema config should be written");
    fs::write(
        staging.join("express_add.schema.yaml"),
        schema("express_add", "express_editor", Some("add_to_input")),
    )
    .expect("express add schema config should be written");
    fs::write(
        staging.join("fluid_direct.schema.yaml"),
        schema("fluid_direct", "fluid_editor", Some("direct_commit")),
    )
    .expect("fluid direct schema config should be written");
    fs::write(
        staging.join("fluid_noop.schema.yaml"),
        schema("fluid_noop", "fluid_editor", Some("noop")),
    )
    .expect("fluid noop schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tn\t100
泥\tni\t90
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let commit_text = |session_id| {
        let mut commit = RimeCommit {
            data_size: std::mem::size_of::<RimeCommit>() as i32,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to writable storage.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was allocated by the shim above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };
    let no_commit = |session_id| {
        let mut commit = RimeCommit {
            data_size: std::mem::size_of::<RimeCommit>() as i32,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to writable storage.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    };
    let current_input = |session_id| {
        let input = RimeGetInput(session_id);
        assert!(!input.is_null());
        // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
        unsafe { CStr::from_ptr(input) }
            .to_str()
            .expect("input should be valid UTF-8")
            .to_owned()
    };
    let create_seeded_session = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        let input = CString::new("n").expect("input should be valid");
        // SAFETY: input is a valid NUL-terminated string.
        assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
        session_id
    };

    let fluid = create_seeded_session("fluid");
    assert_eq!(RimeProcessKey(fluid, 'i' as i32, 0), TRUE);
    assert_eq!(current_input(fluid), "ni");
    no_commit(fluid);
    assert_eq!(RimeDestroySession(fluid), TRUE);

    let express = create_seeded_session("express");
    assert_eq!(RimeProcessKey(express, 'i' as i32, 0), FALSE);
    assert_eq!(commit_text(express), "你");
    assert_eq!(current_input(express), "");
    assert_eq!(RimeDestroySession(express), TRUE);

    let express_add = create_seeded_session("express_add");
    assert_eq!(RimeProcessKey(express_add, 'i' as i32, 0), TRUE);
    assert_eq!(current_input(express_add), "ni");
    no_commit(express_add);
    assert_eq!(RimeDestroySession(express_add), TRUE);

    let fluid_direct = create_seeded_session("fluid_direct");
    assert_eq!(RimeProcessKey(fluid_direct, 'i' as i32, 0), FALSE);
    assert_eq!(commit_text(fluid_direct), "你");
    assert_eq!(current_input(fluid_direct), "");
    assert_eq!(RimeDestroySession(fluid_direct), TRUE);

    let fluid_noop = create_seeded_session("fluid_noop");
    assert_eq!(RimeProcessKey(fluid_noop, 'i' as i32, 0), FALSE);
    no_commit(fluid_noop);
    assert_eq!(current_input(fluid_noop), "n");
    assert_eq!(RimeDestroySession(fluid_noop), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_processor_commits_unique_punctuation() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-processor");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - punctuator
  translators:
    - punct_translator
    - echo_translator
punctuator:
  use_space: true
  half_shape:
    \" \": { commit: \"　\" }
    \".\": \"。\"
  full_shape:
    \" \": { commit: \"□\" }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    // SAFETY: commit points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("　"));
    // SAFETY: commit.text was returned by RimeGetCommit above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    // SAFETY: commit points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("。"));
    // SAFETY: commit.text was returned by RimeGetCommit above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    // SAFETY: commit points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("□"));
    // SAFETY: commit.text was returned by RimeGetCommit above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_processor_loads_namespaced_prescriptions() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-namespaced");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - punctuator@custom_processor
  translators:
    - punct_translator@custom_translator
    - echo_translator
punctuator:
  half_shape:
    \".\": \"。\"
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("。"));
    // SAFETY: commit.text was returned by RimeGetCommit above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_processor_commits_digit_separator_after_number() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-digit-separator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - punctuator
  translators:
    - punct_translator
    - echo_translator
punctuator:
  digit_separators: \".:\"
  digit_separator_action: commit
  half_shape:
    \".\": \"。\"
  full_shape:
    \".\": \"。\"
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let read_commit = || {
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit text should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was returned by RimeGetCommit above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };

    assert_eq!(RimeProcessKey(session_id, '1' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(read_commit(), "1");

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(read_commit(), ".");

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };

    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(read_commit(), "２");

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(read_commit(), "．");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_processor_keeps_default_digit_separator_until_next_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-digit-separator-default");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - punctuator
  translators:
    - punct_translator
    - echo_translator
punctuator:
  digit_separators: \".:\"
  half_shape:
    \".\": \"。\"
  full_shape:
    \".\": \"。\"
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let read_commit = || {
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit text should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was returned by RimeGetCommit above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };

    let context_state = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let input = unsafe { CStr::from_ptr(context.composition.preedit) }
            .to_str()
            .expect("preedit should be valid UTF-8")
            .to_owned();
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by
                // `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        (input, texts)
    };

    assert_eq!(RimeProcessKey(session_id, '1' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(read_commit(), "1");

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    let mut no_commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut no_commit) }, FALSE);
    assert_eq!(context_state(), (".".to_owned(), vec![".".to_owned()]));

    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    assert_eq!(read_commit(), ".2");

    assert_eq!(RimeProcessKey(session_id, '3' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(read_commit(), "3");
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(
        context_state(),
        (".".to_owned(), vec!["。".to_owned(), ".".to_owned()])
    );

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    RimeClearComposition(session_id);

    assert_eq!(RimeProcessKey(session_id, '4' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(read_commit(), "４");
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(context_state(), (".".to_owned(), vec!["．".to_owned()]));
    assert_eq!(RimeProcessKey(session_id, '5' as i32, 0), TRUE);
    assert_eq!(read_commit(), "．５");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_processor_cycles_alternating_punctuation() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-alternating");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - punctuator
  translators:
    - punct_translator
    - echo_translator
punctuator:
  half_shape:
    \"/\": [\"A\", \"B\"]
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let context_state = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let input = unsafe { CStr::from_ptr(context.composition.preedit) }
            .to_str()
            .expect("preedit should be valid UTF-8")
            .to_owned();
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by
                // `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        let highlighted = context.menu.highlighted_candidate_index;
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        (input, texts, highlighted)
    };

    assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
    assert_eq!(
        context_state(),
        (
            "/".to_owned(),
            vec!["A".to_owned(), "B".to_owned(), "/".to_owned()],
            0
        )
    );

    assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
    assert_eq!(
        context_state(),
        (
            "/".to_owned(),
            vec!["A".to_owned(), "B".to_owned(), "/".to_owned()],
            1
        )
    );

    assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
    assert_eq!(
        context_state(),
        (
            "/".to_owned(),
            vec!["A".to_owned(), "B".to_owned(), "/".to_owned()],
            0
        )
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_processor_commits_paired_punctuation_alternately() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-pair");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - punctuator
  translators:
    - punct_translator
    - echo_translator
punctuator:
  half_shape:
    \"(\": { pair: [\"（\", \"）\"] }
  full_shape:
    \"(\": { pair: [\"〔\", \"〕\"] }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let committed_pair = || {
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        assert_eq!(RimeProcessKey(session_id, '(' as i32, 0), TRUE);
        // SAFETY: commit points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit text should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was returned by RimeGetCommit above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };

    assert_eq!(committed_pair(), "（");
    assert_eq!(committed_pair(), "）");
    assert_eq!(committed_pair(), "（");

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };

    assert_eq!(committed_pair(), "〔");
    assert_eq!(committed_pair(), "〕");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn gets_and_frees_available_schema_list() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-list");
    let shared = root.join("shared");
    let user = root.join("user");
    let prebuilt = shared.join("build");
    let staging = user.join("build");
    fs::create_dir_all(&prebuilt).expect("prebuilt dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        prebuilt.join("default.yaml"),
        "\
schema_list:
  - schema: prebuilt_only
",
    )
    .expect("prebuilt default config should be written");
    fs::write(
        staging.join("default.yaml"),
        "\
schema_list:
  - schema: luna_pinyin
  - schema: cangjie5
    case: [conditions/include_cangjie]
  - schema: hidden
    case: [conditions/include_hidden]
  - schema: missing_name
  - not_schema: ignored
conditions:
  include_cangjie: true
  include_hidden: false
",
    )
    .expect("staging default config should be written");
    fs::write(
        staging.join("luna_pinyin.schema.yaml"),
        "schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\n",
    )
    .expect("luna schema config should be written");
    fs::write(
        prebuilt.join("cangjie5.schema.yaml"),
        "schema:\n  schema_id: cangjie5\n  name: Cangjie 5\n",
    )
    .expect("cangjie schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let mut schema_list = empty_schema_list();

    // SAFETY: schema_list points to valid writable storage.
    assert_eq!(unsafe { RimeGetSchemaList(&mut schema_list) }, TRUE);
    assert_eq!(schema_list.size, 4);
    assert!(!schema_list.list.is_null());

    let mut actual = Vec::new();
    for index in 0..schema_list.size {
        // SAFETY: `RimeGetSchemaList` returned true and populated `size`
        // initialized schema-list items.
        let item = unsafe { *schema_list.list.add(index) };
        // SAFETY: schema strings are valid NUL-terminated strings owned by
        // the schema-list object.
        let schema_id = unsafe { CStr::from_ptr(item.schema_id) };
        // SAFETY: schema strings are valid NUL-terminated strings owned by
        // the schema-list object.
        let name = unsafe { CStr::from_ptr(item.name) };
        actual.push((
            schema_id.to_string_lossy().into_owned(),
            name.to_string_lossy().into_owned(),
        ));
        assert!(item.reserved.is_null());
    }
    assert_eq!(
        actual,
        vec![
            ("luna_pinyin".to_owned(), "Luna Pinyin".to_owned()),
            ("cangjie5".to_owned(), "Cangjie 5".to_owned()),
            ("hidden".to_owned(), "hidden".to_owned()),
            ("missing_name".to_owned(), "missing_name".to_owned()),
        ]
    );

    // SAFETY: nested pointers were allocated by `RimeGetSchemaList` above.
    unsafe { super::RimeFreeSchemaList(&mut schema_list) };
    assert_eq!(schema_list.size, 0);
    assert!(schema_list.list.is_null());

    // SAFETY: null pointers are explicitly rejected/no-oped.
    assert_eq!(unsafe { RimeGetSchemaList(std::ptr::null_mut()) }, FALSE);
    unsafe { super::RimeFreeSchemaList(std::ptr::null_mut()) };

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_list_returns_false_when_default_config_has_no_schema_list() {
    let _guard = test_guard();
    let root = unique_temp_dir("schema-list-empty");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(staging.join("default.yaml"), "config_version: test\n")
        .expect("default config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let mut schema_list = empty_schema_list();
    // SAFETY: schema_list points to valid writable storage.
    assert_eq!(unsafe { RimeGetSchemaList(&mut schema_list) }, FALSE);
    assert_eq!(schema_list.size, 0);
    assert!(schema_list.list.is_null());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
