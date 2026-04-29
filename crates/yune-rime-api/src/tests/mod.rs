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
mod lifecycle_safety;
mod resource_id;
mod runtime;
mod schema_api;
mod schema_processors;
mod schema_selection;
mod session_api;
mod userdb;

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
