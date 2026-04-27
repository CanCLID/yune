use std::env;
use std::ffi::{c_void, CStr, CString};
use std::fs;
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_yaml::Value;
use yune_core::StaticTableTranslator;

use super::{
    bool_from, current_log_date_marker, find_config_value, rime_get_api, RimeApi,
    RimeCandidateListBegin, RimeCandidateListEnd, RimeCandidateListFromIndex,
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
    RimeUserConfigOpen, FALSE, TRUE,
};

#[derive(Debug, PartialEq, Eq)]
struct NotificationEvent {
    context_object: usize,
    session_id: super::RimeSessionId,
    message_type: String,
    message_value: String,
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned")
}

fn notification_events() -> &'static Mutex<Vec<NotificationEvent>> {
    static NOTIFICATION_EVENTS: OnceLock<Mutex<Vec<NotificationEvent>>> = OnceLock::new();
    NOTIFICATION_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
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
fn maps_bool_to_rime_bool() {
    assert_eq!(bool_from(true), TRUE);
    assert_eq!(bool_from(false), FALSE);
}

#[test]
fn key_table_exposes_librime_style_modifier_and_key_name_lookup() {
    let shift = CString::new("Shift").expect("modifier name should be valid");
    let control = CString::new("Control").expect("modifier name should be valid");
    let alt = CString::new("Alt").expect("modifier name should be valid");
    let unknown = CString::new("NoSuchModifier").expect("modifier name should be valid");

    assert_eq!(unsafe { RimeGetModifierByName(shift.as_ptr()) }, 1);
    assert_eq!(unsafe { RimeGetModifierByName(control.as_ptr()) }, 1 << 2);
    assert_eq!(unsafe { RimeGetModifierByName(alt.as_ptr()) }, 1 << 3);
    assert_eq!(unsafe { RimeGetModifierByName(unknown.as_ptr()) }, 0);
    assert_eq!(unsafe { RimeGetModifierByName(std::ptr::null()) }, 0);

    assert_eq!(
        static_c_string(RimeGetModifierName(1 << 2)).as_deref(),
        Some("Control")
    );
    assert_eq!(
        static_c_string(RimeGetModifierName((1 << 2) | (1 << 3))).as_deref(),
        Some("Control")
    );
    assert_eq!(static_c_string(RimeGetModifierName(1 << 13)), None);

    let space = CString::new("space").expect("key name should be valid");
    let backspace = CString::new("BackSpace").expect("key name should be valid");
    let left = CString::new("Left").expect("key name should be valid");
    let kp_enter = CString::new("KP_Enter").expect("key name should be valid");
    let kp_page_up = CString::new("KP_Page_Up").expect("key name should be valid");
    let kp_prior = CString::new("KP_Prior").expect("key name should be valid");
    let kp_page_down = CString::new("KP_Page_Down").expect("key name should be valid");
    let kp_next = CString::new("KP_Next").expect("key name should be valid");
    let kp_9 = CString::new("KP_9").expect("key name should be valid");
    let kp_equal = CString::new("KP_Equal").expect("key name should be valid");
    let f1 = CString::new("F1").expect("key name should be valid");
    let f12 = CString::new("F12").expect("key name should be valid");
    let nobreakspace = CString::new("nobreakspace").expect("key name should be valid");
    let yen = CString::new("yen").expect("key name should be valid");
    let eth = CString::new("Eth").expect("key name should be valid");
    let thorn = CString::new("thorn").expect("key name should be valid");
    let ydiaeresis = CString::new("ydiaeresis").expect("key name should be valid");
    let aogonek = CString::new("Aogonek").expect("key name should be valid");
    let lcaron = CString::new("Lcaron").expect("key name should be valid");
    let racute = CString::new("Racute").expect("key name should be valid");
    let tcedilla = CString::new("tcedilla").expect("key name should be valid");
    let abovedot = CString::new("abovedot").expect("key name should be valid");
    let hstroke = CString::new("Hstroke").expect("key name should be valid");
    let gbreve = CString::new("gbreve").expect("key name should be valid");
    let scircumflex = CString::new("scircumflex").expect("key name should be valid");
    let missing = CString::new("NoSuchKey").expect("key name should be valid");

    assert_eq!(unsafe { RimeGetKeycodeByName(space.as_ptr()) }, 0x20);
    assert_eq!(unsafe { RimeGetKeycodeByName(backspace.as_ptr()) }, 0xff08);
    assert_eq!(unsafe { RimeGetKeycodeByName(left.as_ptr()) }, 0xff51);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_enter.as_ptr()) }, 0xff8d);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_page_up.as_ptr()) }, 0xff9a);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_prior.as_ptr()) }, 0xff9a);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(kp_page_down.as_ptr()) },
        0xff9b
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_next.as_ptr()) }, 0xff9b);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_9.as_ptr()) }, 0xffb9);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_equal.as_ptr()) }, 0xffbd);
    assert_eq!(unsafe { RimeGetKeycodeByName(f1.as_ptr()) }, 0xffbe);
    assert_eq!(unsafe { RimeGetKeycodeByName(f12.as_ptr()) }, 0xffc9);
    assert_eq!(unsafe { RimeGetKeycodeByName(nobreakspace.as_ptr()) }, 0xa0);
    assert_eq!(unsafe { RimeGetKeycodeByName(yen.as_ptr()) }, 0xa5);
    assert_eq!(unsafe { RimeGetKeycodeByName(eth.as_ptr()) }, 0xd0);
    assert_eq!(unsafe { RimeGetKeycodeByName(thorn.as_ptr()) }, 0xfe);
    assert_eq!(unsafe { RimeGetKeycodeByName(ydiaeresis.as_ptr()) }, 0xff);
    assert_eq!(unsafe { RimeGetKeycodeByName(aogonek.as_ptr()) }, 0x1a1);
    assert_eq!(unsafe { RimeGetKeycodeByName(lcaron.as_ptr()) }, 0x1a5);
    assert_eq!(unsafe { RimeGetKeycodeByName(racute.as_ptr()) }, 0x1c0);
    assert_eq!(unsafe { RimeGetKeycodeByName(tcedilla.as_ptr()) }, 0x1fe);
    assert_eq!(unsafe { RimeGetKeycodeByName(abovedot.as_ptr()) }, 0x1ff);
    assert_eq!(unsafe { RimeGetKeycodeByName(hstroke.as_ptr()) }, 0x2a1);
    assert_eq!(unsafe { RimeGetKeycodeByName(gbreve.as_ptr()) }, 0x2bb);
    assert_eq!(unsafe { RimeGetKeycodeByName(scircumflex.as_ptr()) }, 0x2fe);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(missing.as_ptr()) },
        0x00ff_ffff
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(std::ptr::null()) },
        0x00ff_ffff
    );

    assert_eq!(
        static_c_string(RimeGetKeyName(0x20)).as_deref(),
        Some("space")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff08)).as_deref(),
        Some("BackSpace")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff51)).as_deref(),
        Some("Left")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff8d)).as_deref(),
        Some("KP_Enter")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff9a)).as_deref(),
        Some("KP_Page_Up")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff9b)).as_deref(),
        Some("KP_Next")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffb9)).as_deref(),
        Some("KP_9")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffbd)).as_deref(),
        Some("KP_Equal")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffbe)).as_deref(),
        Some("F1")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffc9)).as_deref(),
        Some("F12")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xa0)).as_deref(),
        Some("nobreakspace")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xa5)).as_deref(),
        Some("yen")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xd0)).as_deref(),
        Some("ETH")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xde)).as_deref(),
        Some("THORN")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff)).as_deref(),
        Some("ydiaeresis")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1a1)).as_deref(),
        Some("Aogonek")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1a5)).as_deref(),
        Some("Lcaron")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1c0)).as_deref(),
        Some("Racute")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1fe)).as_deref(),
        Some("tcedilla")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1ff)).as_deref(),
        Some("abovedot")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x2a1)).as_deref(),
        Some("Hstroke")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x2bb)).as_deref(),
        Some("gbreve")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x2fe)).as_deref(),
        Some("scircumflex")
    );
    assert_eq!(static_c_string(RimeGetKeyName(0x00ff_ffff)), None);
}

#[test]
fn rime_get_api_exposes_current_function_table() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let api = rime_get_api();
    assert!(!api.is_null());
    // SAFETY: `rime_get_api` returns a process-lifetime pointer to an
    // initialized function table.
    let api = unsafe { &*api };
    assert_eq!(
        api.data_size,
        (std::mem::size_of::<RimeApi>() - std::mem::size_of::<i32>()) as i32
    );

    let create_session = api.create_session.expect("session API should be present");
    let find_session = api.find_session.expect("session API should be present");
    let process_key = api.process_key.expect("input API should be present");
    let get_commit = api.get_commit.expect("commit API should be present");
    let free_commit = api.free_commit.expect("commit API should be present");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("cleanup API should be present");

    assert!(api.schema_open.is_some());
    assert!(api.config_open.is_some());
    assert!(api.user_config_open.is_some());
    assert!(api.config_init.is_some());
    assert!(api.config_load_string.is_some());
    assert!(api.config_get_string.is_some());
    assert!(api.config_get_item.is_some());
    assert!(api.config_set_item.is_some());
    assert!(api.config_update_signature.is_some());
    assert!(api.config_begin_map.is_some());
    assert!(api.config_begin_list.is_some());
    assert!(api.config_next.is_some());
    assert!(api.config_end.is_some());
    assert!(api.commit_proto.is_none());
    assert!(api.get_state_label.is_some());
    assert!(api.get_state_label_abbreviated.is_some());

    let session_id = create_session();
    assert_eq!(find_session(session_id), TRUE);
    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, ' ' as i32, 0), TRUE);

    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    // SAFETY: `get_commit` returned true and populated a valid C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    cleanup_all_sessions();
    assert_eq!(find_session(session_id), FALSE);
}

#[test]
fn config_load_string_and_scalar_accessors_work() {
    let _guard = test_guard();
    let mut config = empty_config();
    let yaml = CString::new(
            "\
schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\nswitches:\n  - name: ascii_mode\nmenu:\n  page_size: 9\nspeller:\n  algebra:\n    - xform/^([nl])ue$/$1ve/\nweights:\n  bias: 0.75\nenabled: true\n",
        )
        .expect("yaml should be valid");
    let mut enabled = FALSE;
    let mut page_size = 0;
    let mut bias = 0.0;
    let mut name_buffer = vec![0 as c_char; 16];

    // SAFETY: config points to writable storage and yaml is a valid C string.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );
    // SAFETY: keys and output pointers are valid for each call.
    assert_eq!(
        unsafe {
            RimeConfigGetBool(
                &mut config,
                CString::new("enabled").unwrap().as_ptr(),
                &mut enabled,
            )
        },
        TRUE
    );
    assert_eq!(enabled, TRUE);
    // SAFETY: keys and output pointers are valid for each call.
    assert_eq!(
        unsafe {
            RimeConfigGetInt(
                &mut config,
                CString::new("menu/page_size").unwrap().as_ptr(),
                &mut page_size,
            )
        },
        TRUE
    );
    assert_eq!(page_size, 9);
    // SAFETY: keys and output pointers are valid for each call.
    assert_eq!(
        unsafe {
            RimeConfigGetDouble(
                &mut config,
                CString::new("weights/bias").unwrap().as_ptr(),
                &mut bias,
            )
        },
        TRUE
    );
    assert_eq!(bias, 0.75);
    // SAFETY: keys and output pointers are valid for each call.
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                CString::new("schema/name").unwrap().as_ptr(),
                name_buffer.as_mut_ptr(),
                name_buffer.len(),
            )
        },
        TRUE
    );
    // SAFETY: the config API NUL-terminates successful string copies.
    assert_eq!(
        unsafe { CStr::from_ptr(name_buffer.as_ptr()) }.to_str(),
        Ok("Luna Pinyin")
    );
    // SAFETY: key is valid and the returned pointer is borrowed from config.
    let schema_id = unsafe {
        RimeConfigGetCString(
            &mut config,
            CString::new("schema/schema_id").unwrap().as_ptr(),
        )
    };
    assert!(!schema_id.is_null());
    // SAFETY: non-null pointer returned by the config API is a valid C string.
    assert_eq!(
        unsafe { CStr::from_ptr(schema_id) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: key and config are valid.
    assert_eq!(
        unsafe { RimeConfigListSize(&mut config, CString::new("switches").unwrap().as_ptr()) },
        1
    );
    // SAFETY: config was initialized by the API and is still open.
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
    assert!(config.ptr.is_null());
}

#[test]
fn config_open_apis_load_runtime_yaml_files() {
    let _guard = test_guard();
    let root = unique_temp_dir("config-open");
    let shared = root.join("shared");
    let user = root.join("user");
    let prebuilt = shared.join("build");
    let staging = user.join("build");
    fs::create_dir_all(&prebuilt).expect("prebuilt dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        prebuilt.join("default.yaml"),
        "schema:\n  name: Prebuilt Default\nmenu:\n  page_size: 5\n",
    )
    .expect("prebuilt config should be written");
    fs::write(
        staging.join("default.yaml"),
        "schema:\n  name: Staging Default\nmenu:\n  page_size: 7\n",
    )
    .expect("staging config should be written");
    fs::write(
        staging.join("luna.schema.yaml"),
        "schema:\n  schema_id: luna\n  name: Luna\n",
    )
    .expect("schema config should be written");
    fs::write(user.join("user.yaml"), "var:\n  option: custom\n")
        .expect("user config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let mut config = empty_config();
    let default_id = CString::new("default").expect("config id should be valid");
    let default_file_id = CString::new("default.yaml").expect("config id should be valid");
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let user_id = CString::new("user").expect("config id should be valid");
    let missing_id = CString::new("missing").expect("config id should be valid");

    // SAFETY: config ids and output config pointers are valid.
    assert_eq!(
        unsafe { RimeConfigOpen(default_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "schema/name").as_deref(),
        Some("Staging Default")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    // SAFETY: config ids and output config pointers are valid.
    assert_eq!(
        unsafe { RimeConfigOpen(default_file_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "schema/name").as_deref(),
        Some("Staging Default")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    // SAFETY: schema id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeSchemaOpen(schema_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "schema/name").as_deref(),
        Some("Luna")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    // SAFETY: config id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeUserConfigOpen(user_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "var/option").as_deref(),
        Some("custom")
    );

    // SAFETY: missing files still create a null config object, mirroring
    // librime's component-backed open behavior.
    assert_eq!(
        unsafe { RimeConfigOpen(missing_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(config_string(&mut config, "schema/name"), None);
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    let api = rime_get_api();
    assert!(!api.is_null());
    // SAFETY: function table pointer has process lifetime.
    let api = unsafe { &*api };
    assert!(api.schema_open.is_some());
    assert!(api.config_open.is_some());
    assert!(api.user_config_open.is_some());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn state_label_apis_read_selected_schema_switches() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("state-label");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
            staging.join("luna.schema.yaml"),
            "\
switches:\n  - name: ascii_mode\n    states: [Native, Ascii]\n    abbrev: [N, A]\n  - options: [simplification, traditional]\n    states: [简体, 繁體]\n",
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
    let missing = CString::new("missing").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    // SAFETY: option names are valid NUL-terminated strings.
    let full_label = unsafe { RimeGetStateLabel(session_id, ascii_mode.as_ptr(), TRUE) };
    assert!(!full_label.is_null());
    // SAFETY: non-null state-label pointers are process-owned C strings.
    assert_eq!(unsafe { CStr::from_ptr(full_label) }.to_str(), Ok("Ascii"));

    // SAFETY: option names are valid NUL-terminated strings.
    let abbreviated =
        unsafe { RimeGetStateLabelAbbreviated(session_id, ascii_mode.as_ptr(), TRUE, TRUE) };
    assert_eq!(abbreviated.length, 1);
    assert!(!abbreviated.str.is_null());
    // SAFETY: non-null state-label pointers are process-owned C strings.
    assert_eq!(unsafe { CStr::from_ptr(abbreviated.str) }.to_str(), Ok("A"));

    // SAFETY: option names are valid NUL-terminated strings.
    let radio =
        unsafe { RimeGetStateLabelAbbreviated(session_id, simplification.as_ptr(), TRUE, TRUE) };
    assert_eq!(radio.length, "简".len());
    // SAFETY: `radio.str` points to a C string and `length` is within its
    // first UTF-8 scalar value.
    let radio_slice = unsafe { std::slice::from_raw_parts(radio.str.cast::<u8>(), radio.length) };
    assert_eq!(std::str::from_utf8(radio_slice), Ok("简"));

    // SAFETY: option names are valid NUL-terminated strings.
    let hidden_radio =
        unsafe { RimeGetStateLabelAbbreviated(session_id, simplification.as_ptr(), FALSE, TRUE) };
    assert!(hidden_radio.str.is_null());
    assert_eq!(hidden_radio.length, 0);
    assert!(unsafe { RimeGetStateLabel(session_id, missing.as_ptr(), TRUE) }.is_null());
    assert!(unsafe { RimeGetStateLabel(0, ascii_mode.as_ptr(), TRUE) }.is_null());
    assert!(unsafe { RimeGetStateLabel(session_id, std::ptr::null(), TRUE) }.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn config_update_signature_writes_runtime_metadata() {
    let _guard = test_guard();
    let distribution_code_name =
        CString::new("yune-test").expect("distribution code name should be valid");
    let distribution_version =
        CString::new("2026.04").expect("distribution version should be valid");
    let mut traits = empty_traits();
    traits.distribution_code_name = distribution_code_name.as_ptr();
    traits.distribution_version = distribution_version.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let mut config = empty_config();
    let signer = CString::new("unit-test").expect("signer should be valid");
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigUpdateSignature(&mut config, signer.as_ptr()) },
        TRUE
    );

    assert_eq!(
        config_string(&mut config, "signature/generator").as_deref(),
        Some("unit-test")
    );
    assert_eq!(
        config_string(&mut config, "signature/distribution_code_name").as_deref(),
        Some("yune-test")
    );
    assert_eq!(
        config_string(&mut config, "signature/distribution_version").as_deref(),
        Some("2026.04")
    );
    assert!(config_string(&mut config, "signature/rime_version")
        .as_deref()
        .is_some_and(|value| value.starts_with("yune-rime-api ")));
    assert!(config_string(&mut config, "signature/modified_time")
        .and_then(|value| value.parse::<u64>().ok())
        .is_some());
    assert_eq!(
        unsafe { RimeConfigUpdateSignature(&mut config, std::ptr::null()) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
}

#[test]
fn config_iterators_expose_list_and_map_paths() {
    let _guard = test_guard();
    let mut config = empty_config();
    let yaml = CString::new(
            "\
switches:\n  - name: ascii_mode\n  - name: full_shape\nmenu:\n  page_size: 9\n  alternative_select_keys: ABC\n",
        )
        .expect("yaml should be valid");
    let switches = CString::new("switches").expect("key should be valid");
    let menu = CString::new("menu").expect("key should be valid");
    let missing = CString::new("missing").expect("key should be valid");
    let mut iterator = empty_config_iterator();

    // SAFETY: config points to writable storage and yaml is a valid C string.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );

    // SAFETY: iterator, config, and key pointers are valid.
    assert_eq!(
        unsafe { RimeConfigBeginList(&mut iterator, &mut config, switches.as_ptr()) },
        TRUE
    );
    assert_eq!(iterator.index, -1);
    assert!(!iterator.list.is_null());
    assert!(iterator.map.is_null());

    // SAFETY: iterator was initialized by RimeConfigBeginList.
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(iterator.index, 0);
    // SAFETY: iterator fields point to NUL-terminated strings owned by the iterator.
    assert_eq!(unsafe { CStr::from_ptr(iterator.key) }.to_str(), Ok("@0"));
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("switches/@0")
    );
    // SAFETY: same iterator remains valid.
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(iterator.index, 1);
    assert_eq!(unsafe { CStr::from_ptr(iterator.key) }.to_str(), Ok("@1"));
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("switches/@1")
    );
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, FALSE);
    // SAFETY: iterator was initialized by this API and can be ended once.
    unsafe { RimeConfigEnd(&mut iterator) };
    assert!(iterator.list.is_null());
    assert!(iterator.key.is_null());

    // SAFETY: iterator, config, and key pointers are valid.
    assert_eq!(
        unsafe { RimeConfigBeginMap(&mut iterator, &mut config, menu.as_ptr()) },
        TRUE
    );
    // SAFETY: iterator was initialized by RimeConfigBeginMap.
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("page_size")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/page_size")
    );
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("alternative_select_keys")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/alternative_select_keys")
    );
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, FALSE);
    unsafe { RimeConfigEnd(&mut iterator) };

    // SAFETY: missing/non-container paths should fail without initializing.
    iterator.list = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.map = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.index = 8;
    iterator.key = switches.as_ptr();
    iterator.path = switches.as_ptr();
    assert_eq!(
        unsafe { RimeConfigBeginList(&mut iterator, &mut config, missing.as_ptr()) },
        FALSE
    );
    assert!(iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(iterator.index, -1);
    assert!(iterator.key.is_null());
    assert!(iterator.path.is_null());

    iterator.list = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.map = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.index = 4;
    iterator.key = switches.as_ptr();
    iterator.path = switches.as_ptr();
    assert_eq!(
        unsafe { RimeConfigBeginMap(&mut iterator, &mut config, missing.as_ptr()) },
        FALSE
    );
    assert!(iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(iterator.index, -1);
    assert!(iterator.key.is_null());
    assert!(iterator.path.is_null());

    // librime performs the basic null-argument checks before clearing the
    // caller-visible iterator state.
    iterator.list = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.map = std::ptr::null_mut();
    iterator.index = 3;
    iterator.key = switches.as_ptr();
    iterator.path = switches.as_ptr();
    assert_eq!(
        unsafe { RimeConfigBeginList(&mut iterator, std::ptr::null_mut(), switches.as_ptr()) },
        FALSE
    );
    assert!(!iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(iterator.index, 3);
    assert_eq!(iterator.key, switches.as_ptr());
    assert_eq!(iterator.path, switches.as_ptr());

    iterator.list = std::ptr::null_mut();
    iterator.map = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.index = 5;
    iterator.key = menu.as_ptr();
    iterator.path = menu.as_ptr();
    assert_eq!(
        unsafe { RimeConfigBeginMap(&mut iterator, std::ptr::null_mut(), menu.as_ptr()) },
        FALSE
    );
    assert!(iterator.list.is_null());
    assert!(!iterator.map.is_null());
    assert_eq!(iterator.index, 5);
    assert_eq!(iterator.key, menu.as_ptr());
    assert_eq!(iterator.path, menu.as_ptr());

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_set_create_clear_and_close_work() {
    let _guard = test_guard();
    let mut config = empty_config();
    let schema_name = CString::new("schema/name").expect("key should be valid");
    let name = CString::new("Default").expect("value should be valid");
    let schema_id = CString::new("schema/schema_id").expect("key should be valid");
    let page_size = CString::new("menu/page_size").expect("key should be valid");
    let bias = CString::new("weights/bias").expect("key should be valid");
    let enabled = CString::new("enabled").expect("key should be valid");
    let switches = CString::new("switches").expect("key should be valid");
    let menu = CString::new("menu").expect("key should be valid");
    let mut output = vec![0 as c_char; 32];
    let mut int_output = 0;
    let mut double_output = 0.0;
    let mut bool_output = FALSE;

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    // SAFETY: all keys and values are valid C strings.
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, schema_name.as_ptr(), name.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, page_size.as_ptr(), 7) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetDouble(&mut config, bias.as_ptr(), 1.25) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetBool(&mut config, enabled.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigCreateList(&mut config, switches.as_ptr()) },
        TRUE
    );

    // SAFETY: all keys and output pointers are valid.
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                schema_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Default")
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, page_size.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 7);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, bias.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 1.25);
    assert_eq!(
        unsafe { RimeConfigGetBool(&mut config, enabled.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, TRUE);
    assert_eq!(
        unsafe { RimeConfigListSize(&mut config, switches.as_ptr()) },
        0
    );

    assert_eq!(
        unsafe { RimeConfigClear(&mut config, schema_name.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                schema_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigCreateMap(&mut config, menu.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, page_size.as_ptr(), &mut int_output) },
        FALSE
    );
    assert_eq!(
        unsafe {
            RimeConfigSetString(
                &mut config,
                schema_id.as_ptr(),
                CString::new("default").unwrap().as_ptr(),
            )
        },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_get_string_allows_zero_length_buffers() {
    let _guard = test_guard();
    let mut config = empty_config();
    let key = CString::new("schema/name").expect("key should be valid");
    let value = CString::new("Default").expect("value should be valid");
    let missing = CString::new("schema/missing").expect("key should be valid");
    let mut output = 42 as c_char;

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigGetString(&mut config, key.as_ptr(), &mut output, 0) },
        TRUE
    );
    assert_eq!(output, 42 as c_char);
    assert_eq!(
        unsafe { RimeConfigGetString(&mut config, missing.as_ptr(), &mut output, 0) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigGetString(&mut config, key.as_ptr(), std::ptr::null_mut(), 0) },
        FALSE
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_get_string_uses_librime_strncpy_copy_semantics() {
    let _guard = test_guard();
    let mut config = empty_config();
    let long_key = CString::new("schema/name").expect("key should be valid");
    let long_value = CString::new("Default").expect("value should be valid");
    let short_key = CString::new("schema/id").expect("key should be valid");
    let short_value = CString::new("yo").expect("value should be valid");
    let mut truncated = [b'!' as c_char; 4];
    let mut padded = [b'!' as c_char; 4];

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, long_key.as_ptr(), long_value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, short_key.as_ptr(), short_value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                long_key.as_ptr(),
                truncated.as_mut_ptr(),
                truncated.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                short_key.as_ptr(),
                padded.as_mut_ptr(),
                padded.len(),
            )
        },
        TRUE
    );

    let truncated_bytes =
        unsafe { std::slice::from_raw_parts(truncated.as_ptr().cast::<u8>(), truncated.len()) };
    let padded_bytes =
        unsafe { std::slice::from_raw_parts(padded.as_ptr().cast::<u8>(), padded.len()) };
    assert_eq!(truncated_bytes, b"Defa");
    assert_eq!(padded_bytes, b"yo\0\0");
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_set_rejects_child_paths_under_existing_scalar_nodes() {
    let _guard = test_guard();
    let mut config = empty_config();
    let scalar = CString::new("zergs/going").expect("key should be valid");
    let child = CString::new("zergs/going/home").expect("key should be valid");
    let root = CString::new("").expect("key should be valid");
    let root_scalar = CString::new("root").expect("value should be valid");
    let root_child = CString::new("child").expect("key should be valid");
    let value = CString::new("home").expect("value should be valid");
    let mut output = vec![0 as c_char; 16];

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetBool(&mut config, scalar.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, child.as_ptr(), value.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                scalar.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("true")
    );

    assert_eq!(
        unsafe { RimeConfigClear(&mut config, scalar.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, child.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, root.as_ptr(), root_scalar.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, root_child.as_ptr(), value.as_ptr()) },
        FALSE
    );
    assert_eq!(config_string(&mut config, "").as_deref(), Some("root"));
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_scalar_access_matches_librime_string_backed_values() {
    let _guard = test_guard();
    let mut config = empty_config();
    let page_size = CString::new("menu/page_size").expect("key should be valid");
    let enabled = CString::new("enabled").expect("key should be valid");
    let bias = CString::new("weights/bias").expect("key should be valid");
    let hex = CString::new("hex").expect("key should be valid");
    let flag = CString::new("flag").expect("key should be valid");
    let decimal = CString::new("decimal").expect("key should be valid");
    let floating = CString::new("floating").expect("key should be valid");
    let native_bool = CString::new("native_bool").expect("key should be valid");
    let native_int = CString::new("native_int").expect("key should be valid");
    let mut int_output = 0;
    let mut double_output = 0.0;
    let mut bool_output = TRUE;

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, page_size.as_ptr(), 7) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetBool(&mut config, enabled.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetDouble(&mut config, bias.as_ptr(), 1.25) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "menu/page_size").as_deref(),
        Some("7")
    );
    assert_eq!(
        config_string(&mut config, "enabled").as_deref(),
        Some("true")
    );
    assert_eq!(
        config_string(&mut config, "weights/bias").as_deref(),
        Some("1.250000")
    );
    // SAFETY: config and key pointers are valid.
    let borrowed = unsafe { RimeConfigGetCString(&mut config, page_size.as_ptr()) };
    assert!(!borrowed.is_null());
    // SAFETY: a non-null config C string is owned by the config cache.
    assert_eq!(unsafe { CStr::from_ptr(borrowed) }.to_str(), Ok("7"));
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    let yaml = CString::new(
        "\
hex: '0x10'\nflag: 'FALSE'\ndecimal: '42'\nfloating: '1.5'\nnative_bool: true\nnative_int: 8\n",
    )
    .expect("yaml should be valid");
    // SAFETY: config points to writable storage and yaml is a valid C string.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, hex.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 16);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, decimal.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 42);
    assert_eq!(
        unsafe { RimeConfigGetBool(&mut config, flag.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, FALSE);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, floating.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 1.5);
    assert_eq!(
        config_string(&mut config, "native_bool").as_deref(),
        Some("true")
    );
    assert_eq!(
        config_string(&mut config, "native_int").as_deref(),
        Some("8")
    );

    // SAFETY: native serde scalars remain readable through typed access.
    assert_eq!(
        unsafe { RimeConfigGetBool(&mut config, native_bool.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, TRUE);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, native_int.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 8);

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_numeric_getters_accept_librime_stoi_stod_prefixes() {
    let _guard = test_guard();
    let mut config = empty_config();
    let decimal_suffix = CString::new("decimal_suffix").expect("key should be valid");
    let signed_spaced = CString::new("signed_spaced").expect("key should be valid");
    let malformed_hex_suffix = CString::new("malformed_hex_suffix").expect("key should be valid");
    let malformed_hex_empty = CString::new("malformed_hex_empty").expect("key should be valid");
    let spaced_hex = CString::new("spaced_hex").expect("key should be valid");
    let invalid_int = CString::new("invalid_int").expect("key should be valid");
    let double_suffix = CString::new("double_suffix").expect("key should be valid");
    let exponent_suffix = CString::new("exponent_suffix").expect("key should be valid");
    let invalid_double = CString::new("invalid_double").expect("key should be valid");
    let mut int_output = 0;
    let mut double_output = 0.0;

    let yaml = CString::new(
            "\
decimal_suffix: '42abc'\nsigned_spaced: '  -7ms'\nmalformed_hex_suffix: '0x10tail'\nmalformed_hex_empty: '0x'\nspaced_hex: ' 0x10'\ninvalid_int: abc42\ndouble_suffix: '  2.5ms'\nexponent_suffix: '1e2hz'\ninvalid_double: hz1.5\n",
        )
        .expect("yaml should be valid");
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );

    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, decimal_suffix.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 42);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, signed_spaced.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, -7);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, malformed_hex_suffix.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 0);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, malformed_hex_empty.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 0);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, spaced_hex.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 0);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, invalid_int.as_ptr(), &mut int_output) },
        FALSE
    );

    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, double_suffix.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 2.5);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, exponent_suffix.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 100.0);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, invalid_double.as_ptr(), &mut double_output) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_get_cstring_keeps_previous_read_only_borrows_alive() {
    let _guard = test_guard();
    let mut config = empty_config();
    let name_key = CString::new("schema/name").expect("key should be valid");
    let name_value = CString::new("Luna Pinyin").expect("value should be valid");
    let id_key = CString::new("schema/schema_id").expect("key should be valid");
    let id_value = CString::new("luna_pinyin").expect("value should be valid");

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, name_key.as_ptr(), name_value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, id_key.as_ptr(), id_value.as_ptr()) },
        TRUE
    );

    let name = unsafe { RimeConfigGetCString(&mut config, name_key.as_ptr()) };
    let schema_id = unsafe { RimeConfigGetCString(&mut config, id_key.as_ptr()) };
    assert!(!name.is_null());
    assert!(!schema_id.is_null());
    assert_eq!(unsafe { CStr::from_ptr(name) }.to_str(), Ok("Luna Pinyin"));
    assert_eq!(
        unsafe { CStr::from_ptr(schema_id) }.to_str(),
        Ok("luna_pinyin")
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_set_supports_librime_list_key_paths() {
    let _guard = test_guard();
    let mut config = empty_config();
    let list = CString::new("list").expect("key should be valid");
    let next_id = CString::new("list/@next/id").expect("key should be valid");
    let last_value = CString::new("list/@last/value").expect("key should be valid");
    let before_first_id = CString::new("list/@before 0/id").expect("key should be valid");
    let first_value = CString::new("list/@0/value").expect("key should be valid");
    let after_last_id = CString::new("list/@after last/id").expect("key should be valid");
    let before_last_id = CString::new("list/@before last/id").expect("key should be valid");
    let value_at_0 = CString::new("list/@0/value").expect("key should be valid");
    let value_at_1 = CString::new("list/@1/value").expect("key should be valid");
    let value_at_2 = CString::new("list/@2/value").expect("key should be valid");
    let value_at_3 = CString::new("list/@3/value").expect("key should be valid");
    let last_id = CString::new("list/@last/id").expect("key should be valid");
    let mut output = 0;

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, next_id.as_ptr(), 1) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, last_value.as_ptr(), 100) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, next_id.as_ptr(), 2) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, last_value.as_ptr(), 200) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 2);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, value_at_0.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 100);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, value_at_1.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 200);

    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, before_first_id.as_ptr(), 3) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, first_value.as_ptr(), 50) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, after_last_id.as_ptr(), 4) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, last_value.as_ptr(), 400) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 4);
    for (path, expected) in [
        (&value_at_0, 50),
        (&value_at_1, 100),
        (&value_at_2, 200),
        (&value_at_3, 400),
    ] {
        assert_eq!(
            unsafe { RimeConfigGetInt(&mut config, path.as_ptr(), &mut output) },
            TRUE
        );
        assert_eq!(output, expected);
    }

    assert_eq!(
        unsafe { RimeConfigCreateList(&mut config, list.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, after_last_id.as_ptr(), 5) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 1);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, last_id.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 5);

    assert_eq!(
        unsafe { RimeConfigCreateList(&mut config, list.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, before_last_id.as_ptr(), 6) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 1);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, last_id.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 6);

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_list_references_follow_librime_strtoul_parsing() {
    let _guard = test_guard();
    let mut config = empty_config();
    let first_id = CString::new("list/@0/id").expect("key should be valid");
    let second_id = CString::new("list/@1/id").expect("key should be valid");
    let malformed_first = CString::new("list/@bogus/id").expect("key should be valid");
    let trailing_first = CString::new("list/@0bogus/id").expect("key should be valid");
    let trailing_after = CString::new("list/@after bogus/id").expect("key should be valid");
    let last_with_suffix = CString::new("list/@last bogus/id").expect("key should be valid");
    let list_value = CString::new("list/@/id").expect("key should be valid");
    let id = CString::new("id").expect("value should be valid");
    let mut output = 0;

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, first_id.as_ptr(), 10) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, second_id.as_ptr(), 20) },
        TRUE
    );

    for path in [&malformed_first, &trailing_first] {
        assert_eq!(
            unsafe { RimeConfigGetInt(&mut config, path.as_ptr(), &mut output) },
            TRUE
        );
        assert_eq!(output, 10);
    }
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, trailing_after.as_ptr(), 30) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, second_id.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 30);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, last_with_suffix.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 20);

    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, list_value.as_ptr(), id.as_ptr()) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_clear_uses_librime_null_write_semantics() {
    let _guard = test_guard();
    let mut config = empty_config();
    let list = CString::new("list").expect("key should be valid");
    let next_id = CString::new("list/@next/id").expect("key should be valid");
    let first_item = CString::new("list/@0").expect("key should be valid");
    let first_id = CString::new("list/@0/id").expect("key should be valid");
    let second_id = CString::new("list/@1/id").expect("key should be valid");
    let mut output = 0;

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, next_id.as_ptr(), 1) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, next_id.as_ptr(), 2) },
        TRUE
    );

    assert_eq!(
        unsafe { RimeConfigClear(&mut config, first_item.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 2);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, first_id.as_ptr(), &mut output) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, second_id.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 2);

    assert_eq!(
        unsafe { RimeConfigClear(&mut config, second_id.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 2);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, second_id.as_ptr(), &mut output) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_get_and_set_item_copy_subtrees() {
    let _guard = test_guard();
    let mut source = empty_config();
    let mut item = empty_config();
    let mut destination = empty_config();
    let yaml = CString::new(
            "\
schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\nswitches:\n  - name: ascii_mode\n  - name: full_shape\n",
        )
        .expect("yaml should be valid");
    let schema = CString::new("schema").expect("key should be valid");
    let copied_schema = CString::new("copied/schema").expect("key should be valid");
    let copied_name = CString::new("copied/schema/name").expect("key should be valid");
    let source_name = CString::new("schema/name").expect("key should be valid");
    let missing = CString::new("missing").expect("key should be valid");
    let mut output = vec![0 as c_char; 32];

    // SAFETY: config pointers and YAML string are valid.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut source, yaml.as_ptr()) },
        TRUE
    );
    // SAFETY: source, key, and destination item pointers are valid.
    assert_eq!(
        unsafe { RimeConfigGetItem(&mut source, schema.as_ptr(), &mut item) },
        TRUE
    );
    assert!(!item.ptr.is_null());
    // SAFETY: configs and keys are valid; item was initialized by get_item.
    assert_eq!(unsafe { RimeConfigInit(&mut destination) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetItem(&mut destination, copied_schema.as_ptr(), &mut item) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut destination,
                copied_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Luna Pinyin")
    );

    assert_eq!(
        unsafe {
            RimeConfigSetString(
                &mut item,
                source_name.as_ptr(),
                CString::new("Modified").unwrap().as_ptr(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut destination,
                copied_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Luna Pinyin")
    );

    // SAFETY: missing items copy as null configs and null values can be set.
    assert_eq!(
        unsafe { RimeConfigGetItem(&mut source, missing.as_ptr(), &mut item) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetItem(&mut destination, copied_schema.as_ptr(), &mut item) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut destination,
                copied_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut source) }, TRUE);
    assert_eq!(unsafe { RimeConfigClose(&mut item) }, TRUE);
    assert_eq!(unsafe { RimeConfigClose(&mut destination) }, TRUE);
}

#[test]
fn setup_and_initialize_expose_runtime_metadata_paths() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let shared = CString::new("/tmp/yune-shared").expect("path should be valid");
    let user = CString::new("/tmp/yune-user").expect("path should be valid");
    let staging = CString::new("/tmp/yune-stage").expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared.as_ptr();
    traits.user_data_dir = user.as_ptr();
    traits.staging_dir = staging.as_ptr();
    let mut buffer = vec![0 as c_char; 64];
    let mut short_buffer = vec![0 as c_char; 10];

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };

    let version = RimeGetVersion();
    assert!(!version.is_null());
    // SAFETY: version is a static NUL-terminated C string.
    let version = unsafe { CStr::from_ptr(version) };
    assert_eq!(version.to_str(), Ok("yune-rime-api 0.1.0"));

    // SAFETY: runtime path getters return stable process-owned C strings.
    let shared_dir = unsafe { CStr::from_ptr(RimeGetSharedDataDir()) };
    assert_eq!(shared_dir.to_str(), Ok("/tmp/yune-shared"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_dir = unsafe { CStr::from_ptr(RimeGetUserDataDir()) };
    assert_eq!(user_dir.to_str(), Ok("/tmp/yune-user"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let prebuilt_dir = unsafe { CStr::from_ptr(RimeGetPrebuiltDataDir()) };
    assert_eq!(prebuilt_dir.to_str(), Ok("/tmp/yune-shared/build"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let staging_dir = unsafe { CStr::from_ptr(RimeGetStagingDir()) };
    assert_eq!(staging_dir.to_str(), Ok("/tmp/yune-stage"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let sync_dir = unsafe { CStr::from_ptr(RimeGetSyncDir()) };
    assert_eq!(sync_dir.to_str(), Ok("sync"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_id = unsafe { CStr::from_ptr(RimeGetUserId()) };
    assert_eq!(user_id.to_str(), Ok("unknown"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetPrebuiltDataDirSecure(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_prebuilt = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_prebuilt.to_str(), Ok("/tmp/yune-shared/build"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetSharedDataDirSecure(short_buffer.as_mut_ptr(), short_buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the short buffer.
    let truncated_shared = unsafe { CStr::from_ptr(short_buffer.as_ptr()) };
    assert_eq!(truncated_shared.to_str(), Ok("/tmp/yune"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetUserDataDirSecure(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_user = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_user.to_str(), Ok("/tmp/yune-user"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetStagingDirSecure(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_staging = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_staging.to_str(), Ok("/tmp/yune-stage"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetSyncDirSecure(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_sync.to_str(), Ok("sync"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetUserDataSyncDir(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_user_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_user_sync.to_str(), Ok("sync/unknown"));

    let prebuilt = CString::new("/tmp/yune-prebuilt").expect("path should be valid");
    traits.prebuilt_data_dir = prebuilt.as_ptr();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeInitialize(&traits) };
    // SAFETY: runtime path getters return stable process-owned C strings.
    let prebuilt_dir = unsafe { CStr::from_ptr(RimeGetPrebuiltDataDir()) };
    assert_eq!(prebuilt_dir.to_str(), Ok("/tmp/yune-prebuilt"));

    RimeFinalize();
}

#[test]
fn setup_reads_existing_installation_metadata() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("installation-metadata");
    let user = root.join("user");
    let sync = root.join("cloud-sync");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        user.join("installation.yaml"),
        format!(
            "\
installation_id: device-123
sync_dir: {}
backup_config_files: false
",
            sync.to_string_lossy()
        ),
    )
    .expect("installation metadata should be written");
    fs::write(user.join("default.yaml"), "config_version: '1.0'\n")
        .expect("user config should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_id = unsafe { CStr::from_ptr(RimeGetUserId()) };
    assert_eq!(user_id.to_str(), Ok("device-123"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let sync_dir = unsafe { CStr::from_ptr(RimeGetSyncDir()) };
    assert_eq!(sync_dir.to_str(), Ok(sync.to_string_lossy().as_ref()));

    let mut buffer = vec![0 as c_char; 256];
    // SAFETY: buffer points to writable storage.
    unsafe { RimeGetUserDataSyncDir(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let user_sync_dir = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(
        user_sync_dir.to_str(),
        Ok(sync.join("device-123").to_string_lossy().as_ref())
    );

    let backup_config_task =
        CString::new("backup_config_files").expect("task name should be valid");
    assert_eq!(RimeRunTask(backup_config_task.as_ptr()), TRUE);
    assert!(!sync.join("device-123").join("default.yaml").exists());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn installation_update_creates_metadata_and_refreshes_runtime_paths() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("installation-update");
    let user = root.join("user");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let distribution_code = CString::new("yune-test").expect("distribution code should be valid");
    let distribution_version =
        CString::new("2026.04").expect("distribution version should be valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    traits.distribution_code_name = distribution_code.as_ptr();
    traits.distribution_version = distribution_version.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let task_name = CString::new("installation_update").expect("task name should be valid");
    assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);

    let metadata = fs::read_to_string(user.join("installation.yaml"))
        .expect("installation metadata should be written");
    let metadata: Value =
        serde_yaml::from_str(&metadata).expect("installation metadata should parse");
    let installation_id = find_config_value(&metadata, "installation_id")
        .and_then(Value::as_str)
        .expect("installation id should be recorded");
    assert!(installation_id.starts_with("yune-"));
    assert_eq!(
        find_config_value(&metadata, "distribution_code_name").and_then(Value::as_str),
        Some("yune-test")
    );
    assert_eq!(
        find_config_value(&metadata, "distribution_version").and_then(Value::as_str),
        Some("2026.04")
    );
    assert!(find_config_value(&metadata, "rime_version").is_some());

    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_id = unsafe { CStr::from_ptr(RimeGetUserId()) };
    assert_eq!(user_id.to_str(), Ok(installation_id));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let sync_dir = unsafe { CStr::from_ptr(RimeGetSyncDir()) };
    assert_eq!(
        sync_dir.to_str(),
        Ok(user.join("sync").to_string_lossy().as_ref())
    );
    let mut buffer = vec![0 as c_char; 256];
    // SAFETY: buffer points to writable storage.
    unsafe { RimeGetUserDataSyncDir(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let user_sync_dir = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(
        user_sync_dir.to_str(),
        Ok(user
            .join("sync")
            .join(installation_id)
            .to_string_lossy()
            .as_ref())
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn maintenance_and_deployment_shims_are_deterministic() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deployer-shims");
    let shared_path = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared_path).expect("shared dir should be created");
    fs::write(
        shared_path.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared_path.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n  version: test\n",
    )
    .expect("shared schema should be written");
    let shared =
        CString::new(shared_path.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let schema_file = CString::new("default.schema.yaml").expect("file should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let task_name = CString::new("workspace_update").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };
    RimeSetupLogging(task_name.as_ptr());
    assert_eq!(RimeStartMaintenance(TRUE), TRUE);
    assert_eq!(RimeStartMaintenanceOnWorkspaceChange(), FALSE);
    assert!(user.join("build").join("default.yaml").is_file());
    assert!(user.join("build").join("default.schema.yaml").is_file());
    assert_eq!(RimeIsMaintenancing(), FALSE);
    RimeJoinMaintenanceThread();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    // SAFETY: runtime path getters return stable process-owned C strings.
    let shared_dir = unsafe { CStr::from_ptr(RimeGetSharedDataDir()) };
    assert_eq!(
        shared_dir.to_str(),
        Ok(shared_path.to_string_lossy().as_ref())
    );

    assert_eq!(RimePrebuildAllSchemas(), TRUE);
    assert_eq!(RimeDeployWorkspace(), TRUE);
    assert_eq!(RimeDeploySchema(schema_file.as_ptr()), TRUE);
    assert_eq!(RimeDeploySchema(std::ptr::null()), FALSE);
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), std::ptr::null()),
        FALSE
    );
    assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);
    assert_eq!(RimeRunTask(std::ptr::null()), FALSE);

    let session_id = RimeCreateSession();
    assert_eq!(RimeFindSession(session_id), TRUE);
    RimeCleanupStaleSessions();
    assert_eq!(RimeFindSession(session_id), TRUE);
    assert_eq!(RimeSyncUserData(), TRUE);
    assert_eq!(RimeFindSession(session_id), FALSE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_copies_shared_yaml_to_staging() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-file");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(shared.join("build")).expect("prebuilt dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: '2.0'\ndeployed_value: from_shared\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("build").join("default.yaml"),
        "config_version: '1.0'\ndeployed_value: from_prebuilt\n",
    )
    .expect("prebuilt config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    assert!(user.join("build").join("default.yaml").is_file());

    let config_id = CString::new("default").expect("config id should be valid");
    let mut config = empty_config();
    // SAFETY: config id and config pointer are valid for the call.
    assert_eq!(
        unsafe { RimeConfigOpen(config_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "deployed_value").as_deref(),
        Some("from_shared")
    );
    assert!(config_string(&mut config, "__build_info/rime_version")
        .as_deref()
        .is_some_and(|version| version.starts_with("yune-rime-api ")));
    // SAFETY: config was opened by the config API.
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    let staged_root: Value = serde_yaml::from_str(
        &fs::read_to_string(user.join("build").join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should be valid YAML");
    assert!(
        find_config_value(&staged_root, "__build_info/timestamps/default")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let missing = CString::new("missing.yaml").expect("file should be valid");
    assert_eq!(
        RimeDeployConfigFile(missing.as_ptr(), version_key.as_ptr()),
        FALSE
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_uses_build_info_freshness() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-freshness");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    let source = shared.join("default.yaml");
    let destination = staging.join("default.yaml");
    fs::write(
        &source,
        "config_version: '2.0'\ndeployed_value: from_shared\n",
    )
    .expect("shared config should be written");

    let mut staged_root: Value = serde_yaml::from_str(
        "config_version: '2.0'\ndeployed_value: already_staged\nlocal_marker: keep\n",
    )
    .expect("staged config should parse");
    super::set_build_info(
        &mut staged_root,
        "default",
        super::source_modified_secs(&source).expect("source mtime should be readable"),
    )
    .expect("build info should be stamped");
    fs::write(
        &destination,
        serde_yaml::to_string(&staged_root).expect("staged config should serialize"),
    )
    .expect("staged config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let unchanged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&unchanged, "deployed_value").and_then(Value::as_str),
        Some("already_staged")
    );
    assert_eq!(
        find_config_value(&unchanged, "local_marker").and_then(Value::as_str),
        Some("keep")
    );

    let mut stale_root = unchanged;
    super::set_build_info(&mut stale_root, "default", 0).expect("build info should be updated");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale_root).expect("stale config should serialize"),
    )
    .expect("stale config should be written");

    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt config should be readable"),
    )
    .expect("rebuilt config should parse");
    assert_eq!(
        find_config_value(&rebuilt, "deployed_value").and_then(Value::as_str),
        Some("from_shared")
    );
    assert!(find_config_value(&rebuilt, "local_marker").is_none());
    assert!(
        find_config_value(&rebuilt, "__build_info/timestamps/default")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_custom_patch_and_tracks_freshness() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-custom-patch");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
schema:
  name: Base
menu:
  page_size: 5
  options:
    - alpha
switches:
  - name: ascii_mode
    reset: false
schema_list: []
translator:
  dictionary: base
  settings:
    existing: yes
",
    )
    .expect("shared config should be written");
    fs::write(
        user.join("default.custom.yaml"),
        "\
patch:
  schema/name/+: ' Extended'
  menu/page_size: 9
  menu/options/+: [beta, gamma]
  switches/@0/reset: true
  schema_list/@next: {schema: luna_pinyin}
  translator/+:
    enable_user_dict: true
    settings:
      new: yes
  new/value: patched
",
    )
    .expect("custom patch should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("default.yaml");
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(9)
    );
    assert_eq!(
        find_config_value(&staged, "schema/name").and_then(Value::as_str),
        Some("Base Extended")
    );
    assert_eq!(
        find_config_value(&staged, "menu/options/@0").and_then(Value::as_str),
        Some("alpha")
    );
    assert_eq!(
        find_config_value(&staged, "menu/options/@1").and_then(Value::as_str),
        Some("beta")
    );
    assert_eq!(
        find_config_value(&staged, "menu/options/@2").and_then(Value::as_str),
        Some("gamma")
    );
    assert_eq!(
        find_config_value(&staged, "switches/@0/reset").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "schema_list/@0/schema").and_then(Value::as_str),
        Some("luna_pinyin")
    );
    assert_eq!(
        find_config_value(&staged, "new/value").and_then(Value::as_str),
        Some("patched")
    );
    assert_eq!(
        find_config_value(&staged, "translator/dictionary").and_then(Value::as_str),
        Some("base")
    );
    assert_eq!(
        find_config_value(&staged, "translator/enable_user_dict").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/existing").and_then(Value::as_str),
        Some("yes")
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/new").and_then(Value::as_str),
        Some("yes")
    );
    assert!(
        find_config_value(&staged, "__build_info/timestamps/default.custom")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    fs::write(
        user.join("default.custom.yaml"),
        "\
patch:
  menu/page_size: 7
",
    )
    .expect("custom patch should be updated");
    let mut stale = staged;
    super::set_build_info(&mut stale, "default.custom", 0)
        .expect("custom build info should be marked stale");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale).expect("stale staged config should serialize"),
    )
    .expect("stale staged config should be written");

    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt config should be readable"),
    )
    .expect("rebuilt config should parse");
    assert_eq!(
        find_config_value(&rebuilt, "menu/page_size").and_then(Value::as_i64),
        Some(7)
    );
    assert!(find_config_value(&rebuilt, "new/value").is_none());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_supports_librime_list_position_references() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-list-positions");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
units:
  - marine
  - zealot
__patch:
  - units/@before 0: probe
  - units/@after 1: medic
  - units/@last: carrier
  - units/@after last: arbiter
  - sparse/@3: observer
",
    )
    .expect("shared config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    let units = ["probe", "marine", "medic", "carrier", "arbiter"];
    for (index, unit) in units.iter().enumerate() {
        assert_eq!(
            find_config_value(&staged, &format!("units/@{index}")).and_then(Value::as_str),
            Some(*unit)
        );
    }
    assert!(find_config_value(&staged, "sparse/@0").is_some_and(Value::is_null));
    assert_eq!(
        find_config_value(&staged, "sparse/@3").and_then(Value::as_str),
        Some("observer")
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_explicit_root_patch_without_auto_custom_patch() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-explicit-patch");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
menu:
  page_size: 5
__patch:
  - menu/page_size: 8
  - explicit/value: patched
",
    )
    .expect("shared config should be written");
    fs::write(
        user.join("default.custom.yaml"),
        "\
patch:
  menu/page_size: 9
  custom_only/value: ignored
",
    )
    .expect("custom patch should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("default.yaml");
    let mut staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        find_config_value(&staged, "explicit/value").and_then(Value::as_str),
        Some("patched")
    );
    assert!(find_config_value(&staged, "custom_only/value").is_none());
    assert!(find_config_value(&staged, "__patch").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/default.custom").is_none());

    super::set_config_value(
        &mut staged,
        "local_marker",
        Value::String("fresh".to_owned()),
    );
    fs::write(
        &destination,
        serde_yaml::to_string(&staged).expect("fresh staged config should serialize"),
    )
    .expect("fresh staged config should be written");

    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let unchanged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("unchanged config should be readable"),
    )
    .expect("unchanged config should parse");
    assert_eq!(
        find_config_value(&unchanged, "local_marker").and_then(Value::as_str),
        Some("fresh")
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_local_root_patch_reference() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-local-patch-ref");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
menu:
  page_size: 5
schema_list:
  - schema: base
__patch:
  - local/patch
  - :/local/extra_patch
  - local/missing?
local:
  patch:
    menu/page_size: 8
    schema_list/@next: {schema: patched}
  extra_patch:
    local_marker: patched
",
    )
    .expect("shared config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        find_config_value(&staged, "schema_list/@1/schema").and_then(Value::as_str),
        Some("patched")
    );
    assert_eq!(
        find_config_value(&staged, "local_marker").and_then(Value::as_str),
        Some("patched")
    );
    assert!(find_config_value(&staged, "__patch").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/default.custom").is_none());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_external_root_patch_reference() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-external-patch-ref");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    let patch_source = shared.join("patches.yaml");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
menu:
  page_size: 5
schema_list:
  - schema: base
__patch:
  - patches.yaml:/preset/patch
  - missing:/patch?
",
    )
    .expect("shared config should be written");
    fs::write(
        &patch_source,
        "\
preset:
  patch:
    menu/page_size: 8
    schema_list/@next: {schema: external}
    external_marker: patched
",
    )
    .expect("patch config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("default.yaml");
    let mut staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        find_config_value(&staged, "schema_list/@1/schema").and_then(Value::as_str),
        Some("external")
    );
    assert_eq!(
        find_config_value(&staged, "external_marker").and_then(Value::as_str),
        Some("patched")
    );
    assert!(find_config_value(&staged, "__patch").is_none());
    assert!(
        find_config_value(&staged, "__build_info/timestamps/patches")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );
    assert_eq!(
        find_config_value(&staged, "__build_info/timestamps/missing").and_then(Value::as_i64),
        Some(0)
    );
    assert!(find_config_value(&staged, "__build_info/timestamps/default.custom").is_none());

    super::set_config_value(
        &mut staged,
        "local_marker",
        Value::String("fresh".to_owned()),
    );
    fs::write(
        &destination,
        serde_yaml::to_string(&staged).expect("fresh staged config should serialize"),
    )
    .expect("fresh staged config should be written");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let unchanged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("unchanged config should be readable"),
    )
    .expect("unchanged config should parse");
    assert_eq!(
        find_config_value(&unchanged, "local_marker").and_then(Value::as_str),
        Some("fresh")
    );

    let mut stale = unchanged;
    super::set_build_info(&mut stale, "patches", 0).expect("patch timestamp should be updated");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale).expect("stale staged config should serialize"),
    )
    .expect("stale staged config should be written");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt config should be readable"),
    )
    .expect("rebuilt config should parse");
    assert!(find_config_value(&rebuilt, "local_marker").is_none());
    assert_eq!(
        find_config_value(&rebuilt, "external_marker").and_then(Value::as_str),
        Some("patched")
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_external_root_include_reference() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-external-include-ref");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
__include: base.yaml:/
config_version: '2.0'
menu:
  page_size: 8
schema_list/+:
  - schema: override
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("base.yaml"),
        "\
config_version: '1.0'
menu:
  page_size: 5
  alternative_select_keys: ABC
schema_list:
  - schema: base
",
    )
    .expect("included config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("default.yaml");
    let mut staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "config_version").and_then(Value::as_str),
        Some("2.0")
    );
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        find_config_value(&staged, "menu/alternative_select_keys").and_then(Value::as_str),
        Some("ABC")
    );
    assert_eq!(
        find_config_value(&staged, "schema_list/@1/schema").and_then(Value::as_str),
        Some("override")
    );
    assert!(find_config_value(&staged, "__include").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/base")
        .and_then(Value::as_i64)
        .is_some_and(|timestamp| timestamp > 0));

    super::set_config_value(
        &mut staged,
        "local_marker",
        Value::String("fresh".to_owned()),
    );
    fs::write(
        &destination,
        serde_yaml::to_string(&staged).expect("fresh staged config should serialize"),
    )
    .expect("fresh staged config should be written");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let unchanged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("unchanged config should be readable"),
    )
    .expect("unchanged config should parse");
    assert_eq!(
        find_config_value(&unchanged, "local_marker").and_then(Value::as_str),
        Some("fresh")
    );

    let mut stale = unchanged;
    super::set_build_info(&mut stale, "base", 0).expect("base timestamp should be updated");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale).expect("stale staged config should serialize"),
    )
    .expect("stale staged config should be written");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt config should be readable"),
    )
    .expect("rebuilt config should parse");
    assert!(find_config_value(&rebuilt, "local_marker").is_none());
    assert_eq!(
        find_config_value(&rebuilt, "schema_list/@1/schema").and_then(Value::as_str),
        Some("override")
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_nested_external_include_references() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-nested-include-ref");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
translator:
  __include: base.yaml:/translator
  enable_user_dict: true
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("base.yaml"),
        "\
translator:
  dictionary: base
  settings:
    __include: settings.yaml:/settings
    option: base
",
    )
    .expect("base config should be written");
    fs::write(
        shared.join("settings.yaml"),
        "\
settings:
  fuzzy: true
",
    )
    .expect("settings config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "translator/dictionary").and_then(Value::as_str),
        Some("base")
    );
    assert_eq!(
        find_config_value(&staged, "translator/enable_user_dict").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/option").and_then(Value::as_str),
        Some("base")
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/fuzzy").and_then(Value::as_bool),
        Some(true)
    );
    assert!(find_config_value(&staged, "translator/__include").is_none());
    assert!(find_config_value(&staged, "translator/settings/__include").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/base")
        .and_then(Value::as_i64)
        .is_some_and(|timestamp| timestamp > 0));
    assert!(
        find_config_value(&staged, "__build_info/timestamps/settings")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_merges_include_directives_into_list_nodes() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-include-list-merge");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
combined_units:
  __include: units.yaml:/base_units
  __append:
    - medic
    - goliath
all_units:
  __patch:
    - __append:
        - scv
        - marine
    - __append:
        - firebat
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("units.yaml"),
        "\
base_units:
  - scv
  - marine
",
    )
    .expect("included config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "combined_units/@0").and_then(Value::as_str),
        Some("scv")
    );
    assert_eq!(
        find_config_value(&staged, "combined_units/@3").and_then(Value::as_str),
        Some("goliath")
    );
    assert_eq!(
        find_config_value(&staged, "all_units/@0").and_then(Value::as_str),
        Some("scv")
    );
    assert_eq!(
        find_config_value(&staged, "all_units/@2").and_then(Value::as_str),
        Some("firebat")
    );
    assert!(find_config_value(&staged, "__build_info/timestamps/units")
        .and_then(Value::as_i64)
        .is_some_and(|timestamp| timestamp > 0));
    assert_eq!(
        find_config_value(&staged, "combined_units/@1").and_then(Value::as_str),
        Some("marine")
    );
    assert!(find_config_value(&staged, "combined_units/__include").is_none());
    assert!(find_config_value(&staged, "combined_units/__append").is_none());
    assert!(find_config_value(&staged, "all_units/__patch").is_none());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_expands_include_directives_inside_patch_values() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-patch-value-include");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
combined_units:
  - probe
  - zealot
__patch:
  combined_units/+:
    __include: units.yaml:/terran_units
literal_units:
  __patch:
    __append:
      __include: units.yaml:/zerg_units
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("units.yaml"),
        "\
terran_units:
  - scv
  - marine
zerg_units:
  - drone
  - zergling
",
    )
    .expect("included config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "combined_units/@0").and_then(Value::as_str),
        Some("probe")
    );
    assert_eq!(
        find_config_value(&staged, "combined_units/@3").and_then(Value::as_str),
        Some("marine")
    );
    assert_eq!(
        find_config_value(&staged, "literal_units/@0").and_then(Value::as_str),
        Some("drone")
    );
    assert_eq!(
        find_config_value(&staged, "literal_units/@1").and_then(Value::as_str),
        Some("zergling")
    );
    assert!(find_config_value(&staged, "__patch").is_none());
    assert!(find_config_value(&staged, "literal_units/__patch").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/units")
        .and_then(Value::as_i64)
        .is_some_and(|timestamp| timestamp > 0));

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_nested_patch_directives() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-nested-patch");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
menu:
  page_size: 5
translator:
  dictionary: base
  settings:
    option: base
    __patch:
      - option: patched
      - patches.yaml:/translator_settings_patch
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("patches.yaml"),
        "\
translator_settings_patch:
  fuzzy: true
",
    )
    .expect("patch config should be written");
    fs::write(
        user.join("default.custom.yaml"),
        "\
patch:
  menu/page_size: 9
",
    )
    .expect("custom patch should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "translator/dictionary").and_then(Value::as_str),
        Some("base")
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/option").and_then(Value::as_str),
        Some("patched")
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/fuzzy").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(9)
    );
    assert!(find_config_value(&staged, "translator/settings/__patch").is_none());
    assert!(
        find_config_value(&staged, "__build_info/timestamps/patches")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );
    assert!(
        find_config_value(&staged, "__build_info/timestamps/default.custom")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_trashes_deprecated_user_copy() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-trash");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: '2.0'\nsource: shared\n",
    )
    .expect("shared config should be written");
    fs::write(
        user.join("default.yaml"),
        "config_version: '1.0'\nsource: deprecated\n",
    )
    .expect("deprecated user config should be written");
    fs::write(
        shared.join("symbols.yaml"),
        "config_version: '2.0.minimal'\nsource: shared\n",
    )
    .expect("minimal shared config should be written");
    fs::write(
        user.join("symbols.yaml"),
        "config_version: '2.0.custom.123'\nsource: customized\n",
    )
    .expect("customized user config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let default_file = CString::new("default.yaml").expect("file should be valid");
    let symbols_file = CString::new("symbols.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(default_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    assert_eq!(
        RimeDeployConfigFile(symbols_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );

    assert!(!user.join("default.yaml").exists());
    let trashed_default = fs::read_to_string(user.join("trash").join("default.yaml"))
        .expect("deprecated user copy should be trashed");
    assert_eq!(
        trashed_default,
        "config_version: '1.0'\nsource: deprecated\n"
    );
    assert!(!user.join("symbols.yaml").exists());
    let trashed_symbols = fs::read_to_string(user.join("trash").join("symbols.yaml"))
        .expect("customized user copy should be trashed");
    assert_eq!(
        trashed_symbols,
        "config_version: '2.0.custom.123'\nsource: customized\n"
    );
    let staged_default = fs::read_to_string(user.join("build").join("default.yaml"))
        .expect("shared config should be staged");
    let staged_default: Value =
        serde_yaml::from_str(&staged_default).expect("staged default should be valid YAML");
    assert_eq!(
        find_config_value(&staged_default, "source").and_then(Value::as_str),
        Some("shared")
    );
    assert!(
        find_config_value(&staged_default, "__build_info/timestamps/default")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_schema_validates_and_copies_shared_schema_to_staging() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-schema");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(shared.join("build")).expect("prebuilt dir should be created");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
  version: '2.0'
",
    )
    .expect("shared schema should be written");
    fs::write(
        shared.join("build").join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Old Luna
  version: '1.0'
",
    )
    .expect("prebuilt schema should be written");
    fs::write(
        shared.join("invalid.schema.yaml"),
        "schema:\n  name: Missing Id\n",
    )
    .expect("invalid schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let schema_file = CString::new("luna.schema.yaml").expect("schema file should be valid");
    let invalid_schema = CString::new("invalid.schema.yaml").expect("schema file should be valid");
    let missing_schema = CString::new("missing.schema.yaml").expect("schema file should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeDeploySchema(schema_file.as_ptr()), TRUE);
    assert!(user.join("build").join("luna.schema.yaml").is_file());

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let mut config = empty_config();
    // SAFETY: schema id and config pointer are valid for the call.
    assert_eq!(
        unsafe { RimeSchemaOpen(schema_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "schema/name").as_deref(),
        Some("Luna")
    );
    assert_eq!(
        config_string(&mut config, "schema/version").as_deref(),
        Some("2.0")
    );
    // SAFETY: config was opened by the config API.
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    assert_eq!(RimeDeploySchema(invalid_schema.as_ptr()), FALSE);
    assert_eq!(RimeDeploySchema(missing_schema.as_ptr()), FALSE);
    assert_eq!(RimeDeploySchema(std::ptr::null()), FALSE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn prebuild_all_schemas_deploys_shared_schema_files() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("prebuild-all-schemas");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(shared.join("build")).expect("prebuilt dir should be created");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
  version: '2.0'
",
    )
    .expect("luna schema should be written");
    fs::write(
        shared.join("terra.schema.yaml"),
        "\
schema:
  schema_id: terra
  name: Terra
  version: '3.0'
",
    )
    .expect("terra schema should be written");
    fs::write(shared.join("notes.yaml"), "schema_id: ignored\n")
        .expect("non-schema yaml should be written");
    fs::write(
        shared.join("build").join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Old Luna
  version: '1.0'
",
    )
    .expect("prebuilt luna schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let task_name = CString::new("prebuild_all_schemas").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimePrebuildAllSchemas(), TRUE);
    assert!(user.join("build").join("luna.schema.yaml").is_file());
    assert!(user.join("build").join("terra.schema.yaml").is_file());
    assert!(!user.join("build").join("notes.yaml").exists());

    let luna_id = CString::new("luna").expect("schema id should be valid");
    let mut luna = empty_config();
    // SAFETY: schema id and config pointer are valid for the call.
    assert_eq!(unsafe { RimeSchemaOpen(luna_id.as_ptr(), &mut luna) }, TRUE);
    assert_eq!(
        config_string(&mut luna, "schema/name").as_deref(),
        Some("Luna")
    );
    // SAFETY: config was opened by the config API.
    assert_eq!(unsafe { RimeConfigClose(&mut luna) }, TRUE);

    fs::remove_file(user.join("build").join("terra.schema.yaml"))
        .expect("staged terra schema should be removable");
    assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);
    assert!(user.join("build").join("terra.schema.yaml").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn workspace_update_deploys_default_schemas_and_dependencies() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-update");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '1.0'
schema_list:
  - schema: luna
  - schema: terra
    case:
      - disabled_flag
",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
  dependencies:
    - luna_ext
",
    )
    .expect("luna schema should be written");
    fs::write(
        shared.join("luna_ext.schema.yaml"),
        "\
schema:
  schema_id: luna_ext
  name: Luna Extension
",
    )
    .expect("dependency schema should be written");
    fs::write(
        shared.join("terra.schema.yaml"),
        "\
schema:
  schema_id: terra
  name: Terra
",
    )
    .expect("terra schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let workspace_task = CString::new("workspace_update").expect("task should be valid");
    let user_dict_task = CString::new("user_dict_upgrade").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    assert_eq!(RimeRunTask(user_dict_task.as_ptr()), TRUE);
    for file_name in [
        "default.yaml",
        "luna.schema.yaml",
        "luna_ext.schema.yaml",
        "terra.schema.yaml",
    ] {
        assert!(user.join("build").join(file_name).is_file());
    }

    let luna_ext_id = CString::new("luna_ext").expect("schema id should be valid");
    let mut luna_ext = empty_config();
    // SAFETY: schema id and config pointer are valid for the call.
    assert_eq!(
        unsafe { RimeSchemaOpen(luna_ext_id.as_ptr(), &mut luna_ext) },
        TRUE
    );
    assert_eq!(
        config_string(&mut luna_ext, "schema/name").as_deref(),
        Some("Luna Extension")
    );
    // SAFETY: config was opened by the config API.
    assert_eq!(unsafe { RimeConfigClose(&mut luna_ext) }, TRUE);

    let user_yaml = fs::read_to_string(user.join("user.yaml"))
        .expect("workspace update should write user config");
    let user_config: Value =
        serde_yaml::from_str(&user_yaml).expect("user config should be valid yaml");
    assert!(
        find_config_value(&user_config, "var/last_build_time")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            > 0
    );

    fs::write(user.join("stale.bin"), "stale").expect("trash fixture should be written");
    assert_eq!(RimeDeployWorkspace(), TRUE);
    assert!(user.join("trash").join("stale.bin").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[cfg(unix)]
#[test]
fn workspace_update_removes_legacy_shared_data_symlinks() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-symlinks");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n",
    )
    .expect("default schema should be written");
    fs::write(shared.join("legacy.table.bin"), "prebuilt")
        .expect("shared prebuilt fixture should be written");
    fs::write(user.join("local.table.bin"), "local").expect("local fixture should be written");
    std::os::unix::fs::symlink(
        shared.join("legacy.table.bin"),
        user.join("legacy.table.bin"),
    )
    .expect("shared symlink should be created");
    std::os::unix::fs::symlink(
        user.join("local.table.bin"),
        user.join("local-link.table.bin"),
    )
    .expect("local symlink should be created");
    std::os::unix::fs::symlink(
        shared.join("missing.table.bin"),
        user.join("missing.table.bin"),
    )
    .expect("dangling symlink should be created");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let workspace_task = CString::new("workspace_update").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);

    assert!(fs::symlink_metadata(user.join("legacy.table.bin")).is_err());
    assert!(fs::symlink_metadata(user.join("missing.table.bin")).is_err());
    assert!(user.join("local-link.table.bin").exists());
    assert!(user.join("build").join("default.schema.yaml").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn cleanup_trash_moves_librime_deployer_artifacts() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("cleanup-trash");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n",
    )
    .expect("shared schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let cleanup_task = CString::new("cleanup_trash").expect("task name should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    for file_name in [
        "rime.log",
        "build.bin",
        "luna_pinyin.reverse.kct",
        "luna_pinyin.userdb.kct.old",
        "luna_pinyin.userdb.kct.snapshot",
    ] {
        fs::write(user.join(file_name), file_name).expect("cleanup fixture should be written");
    }
    fs::write(user.join("default.yaml"), "schema_list: []\n")
        .expect("kept config should be written");

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(cleanup_task.as_ptr()), TRUE);

    for file_name in [
        "rime.log",
        "build.bin",
        "luna_pinyin.reverse.kct",
        "luna_pinyin.userdb.kct.old",
        "luna_pinyin.userdb.kct.snapshot",
    ] {
        assert!(!user.join(file_name).exists());
        assert!(user.join("trash").join(file_name).is_file());
    }
    assert!(user.join("default.yaml").is_file());

    fs::write(user.join("stale.bin"), "stale").expect("deploy fixture should be written");
    assert_eq!(RimeDeployWorkspace(), TRUE);
    assert!(user.join("trash").join("stale.bin").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn clean_old_log_files_removes_stale_app_logs() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("clean-old-log-files");
    let shared = root.join("shared");
    let user = root.join("user");
    let logs = root.join("logs");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::create_dir_all(&logs).expect("log dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n",
    )
    .expect("default schema should be written");
    let today_log = format!("rime_test{}.log", current_log_date_marker());
    for file_name in [
        "rime_test.20000101.log",
        "rime_test.20000102.log",
        "other_app.20000101.log",
        "rime_test.20000101.txt",
        &today_log,
    ] {
        fs::write(logs.join(file_name), file_name).expect("log fixture should be written");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink("rime_test.20000102.log", logs.join("rime_test.INFO"))
        .expect("active log symlink should be created");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let logs_c = CString::new(logs.to_string_lossy().as_ref()).expect("path should be valid");
    let app_name = CString::new("rime_test").expect("app name should be valid");
    let cleanup_task = CString::new("clean_old_log_files").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    traits.app_name = app_name.as_ptr();
    traits.log_dir = logs_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };
    assert_eq!(RimeRunTask(cleanup_task.as_ptr()), TRUE);

    assert!(!logs.join("rime_test.20000101.log").exists());
    assert!(logs.join("rime_test.20000102.log").is_file());
    assert!(logs.join("other_app.20000101.log").is_file());
    assert!(logs.join("rime_test.20000101.txt").is_file());
    assert!(logs.join(today_log).is_file());

    fs::write(logs.join("rime_test.19991231.log"), "stale")
        .expect("maintenance log fixture should be written");
    assert_eq!(RimeStartMaintenance(TRUE), TRUE);
    assert!(!logs.join("rime_test.19991231.log").exists());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn maintenance_on_workspace_change_detects_yaml_modifications() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("detect-modifications");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n",
    )
    .expect("default schema should be written");
    fs::write(
        user.join("user.yaml"),
        "var:\n  last_build_time: 2147483647\n",
    )
    .expect("user config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };
    assert_eq!(RimeStartMaintenanceOnWorkspaceChange(), FALSE);

    fs::write(user.join("user.yaml"), "var:\n  last_build_time: 0\n")
        .expect("user config should be updated");
    assert_eq!(RimeStartMaintenanceOnWorkspaceChange(), TRUE);
    assert!(user.join("build").join("default.yaml").is_file());
    assert!(user.join("build").join("default.schema.yaml").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn registers_and_finds_modules_by_name() {
    let _guard = test_guard();
    super::module_registry()
        .lock()
        .expect("module registry should not be poisoned")
        .modules_by_name
        .clear();
    let module_name = CString::new("sample_module_abi").expect("module name should be valid");
    let replacement_name = CString::new("sample_module_abi").expect("module name should be valid");
    let missing_name = CString::new("missing_module_abi").expect("module name should be valid");
    let mut module = RimeModule {
        data_size: std::mem::size_of::<RimeModule>() as i32,
        module_name: module_name.as_ptr(),
        initialize: Some(sample_module_initialize),
        finalize: Some(sample_module_finalize),
        get_api: Some(sample_module_get_api),
    };
    let mut replacement = RimeModule {
        data_size: std::mem::size_of::<RimeModule>() as i32,
        module_name: replacement_name.as_ptr(),
        initialize: None,
        finalize: None,
        get_api: None,
    };
    let mut unnamed = RimeModule {
        data_size: std::mem::size_of::<RimeModule>() as i32,
        module_name: std::ptr::null(),
        initialize: None,
        finalize: None,
        get_api: None,
    };

    // SAFETY: module names point to valid NUL-terminated strings and the
    // module storage lives through the lookups below.
    assert_eq!(unsafe { RimeRegisterModule(&mut module) }, TRUE);
    // SAFETY: lookup names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeFindModule(module_name.as_ptr()) },
        std::ptr::addr_of_mut!(module)
    );
    // SAFETY: lookup name is a valid NUL-terminated string.
    assert!(unsafe { RimeFindModule(missing_name.as_ptr()) }.is_null());

    // SAFETY: replacement module uses the same valid NUL-terminated name.
    assert_eq!(unsafe { RimeRegisterModule(&mut replacement) }, TRUE);
    // SAFETY: lookup name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeFindModule(replacement_name.as_ptr()) },
        std::ptr::addr_of_mut!(replacement)
    );

    // SAFETY: null inputs are explicitly rejected without dereferencing.
    assert_eq!(unsafe { RimeRegisterModule(std::ptr::null_mut()) }, FALSE);
    // SAFETY: unnamed points to a valid module with a null module_name.
    assert_eq!(unsafe { RimeRegisterModule(&mut unnamed) }, FALSE);
    // SAFETY: null lookup names are explicitly rejected without dereferencing.
    assert!(unsafe { RimeFindModule(std::ptr::null()) }.is_null());

    super::module_registry()
        .lock()
        .expect("module registry should not be poisoned")
        .modules_by_name
        .clear();
}

#[test]
fn built_in_levers_module_exposes_available_schema_list() {
    let _guard = test_guard();
    super::module_registry()
        .lock()
        .expect("module registry should not be poisoned")
        .modules_by_name
        .clear();
    let root = unique_temp_dir("levers-schema-list");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
show_extra_schema: false
schema_list:
  - schema: luna_pinyin
  - schema: extra_schema
    case:
      - show_extra_schema
",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("luna_pinyin.schema.yaml"),
        "\
schema:
  schema_id: luna_pinyin
  name: Luna Pinyin
  version: '1.0'
  author:
    - Author One
    - Author Two
  description: Sample schema
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

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    assert!(module.get_api.is_some());
    let get_api = module.get_api.expect("levers get_api should be set");
    let api = get_api().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    assert_eq!(
        api.data_size,
        (std::mem::size_of::<RimeLeversApi>() - std::mem::size_of::<i32>()) as i32
    );
    assert!(api.custom_settings_init.is_some());
    assert!(api.custom_settings_destroy.is_some());
    assert!(api.load_settings.is_some());
    assert!(api.save_settings.is_some());
    assert!(api.customize_bool.is_some());
    assert!(api.customize_int.is_some());
    assert!(api.customize_double.is_some());
    assert!(api.customize_string.is_some());
    assert!(api.customize_item.is_some());
    assert!(api.is_first_run.is_some());
    assert!(api.settings_is_modified.is_some());
    assert!(api.settings_get_config.is_some());
    assert!(api.switcher_settings_init.is_some());
    assert!(api.get_available_schema_list.is_some());
    assert!(api.get_selected_schema_list.is_some());
    assert!(api.schema_list_destroy.is_some());
    assert!(api.get_schema_id.is_some());
    assert!(api.get_schema_name.is_some());
    assert!(api.get_schema_version.is_some());
    assert!(api.get_schema_author.is_some());
    assert!(api.get_schema_description.is_some());
    assert!(api.get_schema_file_path.is_some());
    assert!(api.select_schemas.is_some());
    assert!(api.user_dict_iterator_init.is_some());
    assert!(api.user_dict_iterator_destroy.is_some());
    assert!(api.next_user_dict.is_some());
    assert!(api.backup_user_dict.is_some());
    assert!(api.restore_user_dict.is_some());
    assert!(api.export_user_dict.is_some());
    assert!(api.import_user_dict.is_some());

    let settings = (api
        .switcher_settings_init
        .expect("switcher settings init should be available"))();
    assert!(!settings.is_null());
    let mut schema_list = empty_schema_list();
    let get_available = api
        .get_available_schema_list
        .expect("available schema list should be available");
    // SAFETY: settings and schema_list are valid for the call.
    assert_eq!(unsafe { get_available(settings, &mut schema_list) }, TRUE);
    assert_eq!(schema_list.size, 1);
    // SAFETY: the levers API populated one schema-list item.
    let item = unsafe { *schema_list.list };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let schema_id = unsafe { CStr::from_ptr(item.schema_id) };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let name = unsafe { CStr::from_ptr(item.name) };
    assert_eq!(schema_id.to_str(), Ok("luna_pinyin"));
    assert_eq!(name.to_str(), Ok("Luna Pinyin"));
    assert!(!item.reserved.is_null());

    let get_schema_id = api.get_schema_id.expect("schema id getter should be set");
    let get_schema_name = api
        .get_schema_name
        .expect("schema name getter should be set");
    let get_schema_version = api
        .get_schema_version
        .expect("schema version getter should be set");
    let get_schema_author = api
        .get_schema_author
        .expect("schema author getter should be set");
    let get_schema_description = api
        .get_schema_description
        .expect("schema description getter should be set");
    let get_schema_file_path = api
        .get_schema_file_path
        .expect("schema file path getter should be set");
    let schema_info = item.reserved.cast();
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_id(schema_info)) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_name(schema_info)) }.to_str(),
        Ok("Luna Pinyin")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_version(schema_info)) }.to_str(),
        Ok("1.0")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_author(schema_info)) }.to_str(),
        Ok("Author One\nAuthor Two")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_description(schema_info)) }.to_str(),
        Ok("Sample schema")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    let file_path = unsafe { CStr::from_ptr(get_schema_file_path(schema_info)) };
    assert_eq!(
        file_path.to_string_lossy(),
        staging.join("luna_pinyin.schema.yaml").to_string_lossy()
    );
    // SAFETY: null schema info is explicitly rejected.
    assert!(unsafe { get_schema_id(std::ptr::null_mut()) }.is_null());

    let mut selected_list = empty_schema_list();
    let get_selected = api
        .get_selected_schema_list
        .expect("selected schema list should be available");
    // SAFETY: settings and selected_list are valid for the call.
    assert_eq!(unsafe { get_selected(settings, &mut selected_list) }, TRUE);
    assert_eq!(selected_list.size, 2);
    // SAFETY: the levers API populated two selected schema-list items.
    let selected_first = unsafe { *selected_list.list };
    // SAFETY: the second item is in bounds because size is 2.
    let selected_second = unsafe { *selected_list.list.add(1) };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    let selected_first_id = unsafe { CStr::from_ptr(selected_first.schema_id) };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    let selected_second_id = unsafe { CStr::from_ptr(selected_second.schema_id) };
    assert_eq!(selected_first_id.to_str(), Ok("luna_pinyin"));
    assert_eq!(selected_second_id.to_str(), Ok("extra_schema"));
    assert!(selected_first.name.is_null());
    assert!(selected_first.reserved.is_null());
    assert!(selected_second.name.is_null());
    assert!(selected_second.reserved.is_null());

    let destroy = api
        .schema_list_destroy
        .expect("schema-list destroy should be available");
    // SAFETY: selected_list was populated by the levers API above.
    unsafe { destroy(&mut selected_list) };
    assert_eq!(selected_list.size, 0);
    assert!(selected_list.list.is_null());

    let select_schemas = api
        .select_schemas
        .expect("select_schemas should be available");
    let selected_luna = CString::new("luna_pinyin").expect("schema id should be valid");
    let selected_terra = CString::new("terra_pinyin").expect("schema id should be valid");
    let schema_ids = [selected_terra.as_ptr(), selected_luna.as_ptr()];
    // SAFETY: settings, schema_ids, and each C string are valid for the call.
    assert_eq!(
        unsafe { select_schemas(settings, schema_ids.as_ptr(), schema_ids.len() as i32) },
        TRUE
    );
    let mut overridden_selected_list = empty_schema_list();
    // SAFETY: settings and selected list output are valid.
    assert_eq!(
        unsafe { get_selected(settings, &mut overridden_selected_list) },
        TRUE
    );
    assert_eq!(overridden_selected_list.size, 2);
    // SAFETY: the levers API populated two selected schema-list items.
    let overridden_first = unsafe { *overridden_selected_list.list };
    // SAFETY: the second item is in bounds because size is 2.
    let overridden_second = unsafe { *overridden_selected_list.list.add(1) };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    let overridden_first_id = unsafe { CStr::from_ptr(overridden_first.schema_id) };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    let overridden_second_id = unsafe { CStr::from_ptr(overridden_second.schema_id) };
    assert_eq!(overridden_first_id.to_str(), Ok("terra_pinyin"));
    assert_eq!(overridden_second_id.to_str(), Ok("luna_pinyin"));
    assert!(overridden_first.name.is_null());
    assert!(overridden_first.reserved.is_null());
    assert!(overridden_second.name.is_null());
    assert!(overridden_second.reserved.is_null());
    // SAFETY: null settings and null schema arrays are rejected.
    assert_eq!(
        unsafe { select_schemas(std::ptr::null_mut(), schema_ids.as_ptr(), 1) },
        FALSE
    );
    assert_eq!(
        unsafe { select_schemas(settings, std::ptr::null(), 1) },
        FALSE
    );
    // SAFETY: overridden_selected_list was populated by the levers API above.
    unsafe { destroy(&mut overridden_selected_list) };
    assert_eq!(overridden_selected_list.size, 0);
    assert!(overridden_selected_list.list.is_null());

    // SAFETY: schema_list was populated by the levers API above.
    unsafe { destroy(&mut schema_list) };
    assert_eq!(schema_list.size, 0);
    assert!(schema_list.list.is_null());
    // SAFETY: settings was allocated by this shim's switcher init function.
    unsafe { drop(Box::from_raw(settings)) };

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn levers_user_dict_iterator_lists_userdb_entries() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-user-dicts");
    let user = root.join("user");
    fs::create_dir_all(user.join("luna_pinyin.userdb"))
        .expect("leveldb-style user dict dir should be created");
    fs::write(user.join("essay.userdb"), "").expect("user dict file should be written");
    fs::write(user.join("legacy.userdb.txt"), "")
        .expect("plain legacy user dict should not match current userdb extension");
    fs::write(user.join("default.yaml"), "").expect("unrelated file should be ignored");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    let iterator_init = api
        .user_dict_iterator_init
        .expect("user dict iterator init should be available");
    let iterator_destroy = api
        .user_dict_iterator_destroy
        .expect("user dict iterator destroy should be available");
    let next_user_dict = api
        .next_user_dict
        .expect("next user dict should be available");

    let mut iterator = super::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 0,
    };
    // SAFETY: iterator points to writable storage.
    assert_eq!(unsafe { iterator_init(&mut iterator) }, TRUE);
    assert!(!iterator.ptr.is_null());
    assert_eq!(iterator.i, 0);

    // SAFETY: iterator was initialized by the levers API.
    let first = unsafe { next_user_dict(&mut iterator) };
    assert!(!first.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(unsafe { CStr::from_ptr(first) }.to_str(), Ok("essay"));
    // SAFETY: iterator remains initialized.
    let second = unsafe { next_user_dict(&mut iterator) };
    assert!(!second.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(
        unsafe { CStr::from_ptr(second) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: iterator is exhausted but valid.
    assert!(unsafe { next_user_dict(&mut iterator) }.is_null());

    // SAFETY: iterator was initialized by this shim.
    unsafe { iterator_destroy(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert_eq!(iterator.i, 0);

    // SAFETY: null inputs are explicitly rejected/no-oped.
    assert_eq!(unsafe { iterator_init(std::ptr::null_mut()) }, FALSE);
    assert!(unsafe { next_user_dict(std::ptr::null_mut()) }.is_null());
    unsafe { iterator_destroy(std::ptr::null_mut()) };

    fs::remove_file(user.join("essay.userdb")).expect("user dict file should be removed");
    fs::remove_dir_all(user.join("luna_pinyin.userdb")).expect("user dict dir should be removed");
    let mut empty_iterator = super::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 7,
    };
    // SAFETY: iterator points to writable storage; no .userdb entries remain.
    assert_eq!(unsafe { iterator_init(&mut empty_iterator) }, FALSE);
    assert!(empty_iterator.ptr.is_null());
    assert_eq!(empty_iterator.i, 0);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn levers_user_dict_file_operations_handle_plain_userdb_files() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-user-dict-files");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    struct CurrentDirGuard(PathBuf);
    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }
    let current_dir_guard =
        CurrentDirGuard(env::current_dir().expect("current dir should be available"));
    env::set_current_dir(&root).expect("test cwd should move under temp root");
    fs::write(
        user.join("luna_pinyin.userdb"),
        "# comment\nni hao\t你好\t1\n\nzhong guo\t中国\t2\n",
    )
    .expect("plain user dict should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    let backup_user_dict = api
        .backup_user_dict
        .expect("backup user dict should be available");
    let restore_user_dict = api
        .restore_user_dict
        .expect("restore user dict should be available");
    let export_user_dict = api
        .export_user_dict
        .expect("export user dict should be available");
    let import_user_dict = api
        .import_user_dict
        .expect("import user dict should be available");

    let dict_name = CString::new("luna_pinyin").expect("dict name is valid");
    // SAFETY: dict name is a valid NUL-terminated string.
    assert_eq!(unsafe { backup_user_dict(dict_name.as_ptr()) }, TRUE);
    let snapshot = root
        .join("sync")
        .join("unknown")
        .join("luna_pinyin.userdb.txt");
    assert_eq!(
        fs::read_to_string(&snapshot).expect("snapshot should be readable"),
        fs::read_to_string(user.join("luna_pinyin.userdb")).expect("user dict should be readable")
    );

    let export_path = root.join("luna_export.tsv");
    let export_path_c =
        CString::new(export_path.to_string_lossy().as_ref()).expect("path is valid");
    // SAFETY: pointers are valid NUL-terminated strings.
    assert_eq!(
        unsafe { export_user_dict(dict_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    assert_eq!(
        fs::read_to_string(&export_path).expect("export should be readable"),
        fs::read_to_string(user.join("luna_pinyin.userdb")).expect("user dict should be readable")
    );

    fs::write(&export_path, "xin\t新\t3\nci\t词\t4\n").expect("import file should be updated");
    let imported_name = CString::new("imported").expect("dict name is valid");
    // SAFETY: pointers are valid NUL-terminated strings.
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    assert_eq!(
        fs::read_to_string(user.join("imported.userdb")).expect("import should be readable"),
        "xin\t新\t3\nci\t词\t4\n"
    );

    let snapshot_c = CString::new(snapshot.to_string_lossy().as_ref()).expect("path is valid");
    fs::remove_file(user.join("luna_pinyin.userdb"))
        .expect("user dict should be removable before restore");
    // SAFETY: snapshot path is a valid NUL-terminated string.
    assert_eq!(unsafe { restore_user_dict(snapshot_c.as_ptr()) }, TRUE);
    assert!(user.join("luna_pinyin.userdb").is_file());

    let missing = CString::new("missing").expect("dict name is valid");
    // SAFETY: null and missing inputs are explicitly rejected.
    assert_eq!(unsafe { backup_user_dict(std::ptr::null()) }, FALSE);
    assert_eq!(unsafe { backup_user_dict(missing.as_ptr()) }, FALSE);
    assert_eq!(unsafe { restore_user_dict(std::ptr::null()) }, FALSE);
    assert_eq!(
        unsafe { export_user_dict(std::ptr::null(), export_path_c.as_ptr()) },
        -1
    );
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), std::ptr::null()) },
        -1
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    drop(current_dir_guard);
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
fn levers_custom_settings_load_modify_and_save_custom_yaml() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-custom-settings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna_pinyin.schema.yaml"),
        "\
schema:
  schema_id: luna_pinyin
  name: Luna Pinyin
menu:
  page_size: 5
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    traits.distribution_code_name = c"test_dist".as_ptr();
    traits.distribution_version = c"2026.04".as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };

    let config_id = CString::new("luna_pinyin.schema").expect("config id should be valid");
    let generator = CString::new("yune_test").expect("generator should be valid");
    let init = api
        .custom_settings_init
        .expect("custom settings init should be available");
    let destroy = api
        .custom_settings_destroy
        .expect("custom settings destroy should be available");
    let load = api
        .load_settings
        .expect("load_settings should be available");
    let save = api
        .save_settings
        .expect("save_settings should be available");
    let is_first_run = api.is_first_run.expect("is_first_run should be available");
    let is_modified = api
        .settings_is_modified
        .expect("settings_is_modified should be available");
    let get_config = api
        .settings_get_config
        .expect("settings_get_config should be available");

    // SAFETY: config id and generator are valid C strings.
    let settings = unsafe { init(config_id.as_ptr(), generator.as_ptr()) };
    assert!(!settings.is_null());
    // SAFETY: settings is valid for each call.
    assert_eq!(unsafe { load(settings) }, FALSE);
    assert_eq!(unsafe { is_first_run(settings) }, TRUE);
    assert_eq!(unsafe { is_modified(settings) }, FALSE);

    let mut loaded_config = empty_config();
    // SAFETY: settings and config output are valid.
    assert_eq!(unsafe { get_config(settings, &mut loaded_config) }, TRUE);
    assert_eq!(
        config_string(&mut loaded_config, "schema/name").as_deref(),
        Some("Luna Pinyin")
    );

    let custom_bool_key = CString::new("switches/@0/reset").expect("custom key should be valid");
    let custom_int_key = CString::new("menu/page_size").expect("custom key should be valid");
    let custom_double_key = CString::new("weights/bias").expect("custom key should be valid");
    let custom_string_key = CString::new("schema/name").expect("custom key should be valid");
    let custom_string_value = CString::new("Custom Luna").expect("value should be valid");
    let customize_bool = api
        .customize_bool
        .expect("customize_bool should be available");
    let customize_int = api
        .customize_int
        .expect("customize_int should be available");
    let customize_double = api
        .customize_double
        .expect("customize_double should be available");
    let customize_string = api
        .customize_string
        .expect("customize_string should be available");
    // SAFETY: settings and keys are valid for each customization call.
    assert_eq!(
        unsafe { customize_bool(settings, custom_bool_key.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { customize_int(settings, custom_int_key.as_ptr(), 9) },
        TRUE
    );
    assert_eq!(
        unsafe { customize_double(settings, custom_double_key.as_ptr(), 0.25) },
        TRUE
    );
    assert_eq!(
        unsafe {
            customize_string(
                settings,
                custom_string_key.as_ptr(),
                custom_string_value.as_ptr(),
            )
        },
        TRUE
    );

    let mut item_config = empty_config();
    let item_yaml = CString::new("- Control+grave\n- F4\n").expect("yaml should be valid");
    // SAFETY: item_config and YAML string are valid.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut item_config, item_yaml.as_ptr()) },
        TRUE
    );
    let customize_item = api
        .customize_item
        .expect("customize_item should be available");
    let item_key = CString::new("switcher/hotkeys").expect("item key should be valid");
    // SAFETY: settings, key, and item config are valid.
    assert_eq!(
        unsafe { customize_item(settings, item_key.as_ptr(), &mut item_config) },
        TRUE
    );
    assert_eq!(unsafe { is_modified(settings) }, TRUE);
    assert_eq!(unsafe { save(settings) }, TRUE);
    assert_eq!(unsafe { is_modified(settings) }, FALSE);
    assert_eq!(unsafe { save(settings) }, FALSE);
    assert_eq!(unsafe { is_first_run(settings) }, FALSE);

    let saved = fs::read_to_string(user.join("luna_pinyin.custom.yaml"))
        .expect("custom settings should be saved without .schema suffix");
    let saved_root: Value = serde_yaml::from_str(&saved).expect("saved YAML should parse");
    let patch = find_config_value(&saved_root, "patch")
        .and_then(Value::as_mapping)
        .expect("patch map should be present");
    assert_eq!(
        patch
            .get(Value::String("switches/@0/reset".to_owned()))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        patch
            .get(Value::String("menu/page_size".to_owned()))
            .and_then(Value::as_i64),
        Some(9)
    );
    assert_eq!(
        patch
            .get(Value::String("weights/bias".to_owned()))
            .and_then(Value::as_f64),
        Some(0.25)
    );
    assert_eq!(
        patch
            .get(Value::String("schema/name".to_owned()))
            .and_then(Value::as_str),
        Some("Custom Luna")
    );
    assert!(matches!(
        patch.get(Value::String("switcher/hotkeys".to_owned())),
        Some(Value::Sequence(values)) if values.len() == 2
    ));
    assert_eq!(
        find_config_value(&saved_root, "customization/generator").and_then(Value::as_str),
        Some("yune_test")
    );
    assert_eq!(
        find_config_value(&saved_root, "customization/distribution_code_name")
            .and_then(Value::as_str),
        Some("test_dist")
    );

    // SAFETY: configs and settings were initialized by this API.
    unsafe {
        assert_eq!(RimeConfigClose(&mut loaded_config), TRUE);
        assert_eq!(RimeConfigClose(&mut item_config), TRUE);
        destroy(settings);
    }
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn levers_hotkeys_are_read_from_deployed_default_config() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-hotkeys");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
switcher:
  hotkeys:
    - Control+grave
    - F4
    - ''
",
    )
    .expect("default config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let settings = super::RimeSwitcherSettingsInit();
    assert!(!settings.is_null());
    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    let get_hotkeys = api.get_hotkeys.expect("get_hotkeys should be available");
    let set_hotkeys = api.set_hotkeys.expect("set_hotkeys should be available");

    // SAFETY: settings is a valid pointer returned by the shim.
    let hotkeys = unsafe { get_hotkeys(settings) };
    assert!(!hotkeys.is_null());
    // SAFETY: get_hotkeys returns a process-owned NUL-terminated C string.
    assert_eq!(
        unsafe { CStr::from_ptr(hotkeys) }.to_str(),
        Ok("Control+grave, F4")
    );
    // SAFETY: null settings are rejected without dereferencing.
    assert!(unsafe { get_hotkeys(std::ptr::null_mut()) }.is_null());

    let new_hotkeys = CString::new("Alt+space").expect("hotkeys should be valid");
    // SAFETY: settings and hotkeys are valid pointers; mutation is currently unsupported.
    assert_eq!(
        unsafe { set_hotkeys(settings, new_hotkeys.as_ptr()) },
        FALSE
    );

    // SAFETY: settings was allocated by this shim's switcher init function.
    unsafe { drop(Box::from_raw(settings)) };
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn notification_handler_receives_runtime_events_and_can_be_cleared() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("notification-events");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: sample_schema\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("sample_schema.schema.yaml"),
        "schema:\n  schema_id: sample_schema\n  name: Sample\n",
    )
    .expect("shared schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
    let session_id = RimeCreateSession();
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let property = CString::new("client_app").expect("property name should be valid");
    let property_value = CString::new("sample_console").expect("property value should be valid");
    let schema_id = CString::new("sample_schema").expect("schema id should be valid");
    let context_object = 0x5a_usize as *mut c_void;

    RimeSetNotificationHandler(Some(record_notification), context_object);
    // SAFETY: option, property, value, and schema strings are valid
    // NUL-terminated C strings.
    unsafe {
        RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE);
        RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE);
        RimeSetProperty(session_id, property.as_ptr(), property_value.as_ptr());
        assert_eq!(RimeSelectSchema(session_id, schema_id.as_ptr()), TRUE);
    }
    assert_eq!(RimeDeployWorkspace(), TRUE);

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned");
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x5a,
                session_id,
                message_type: "option".to_owned(),
                message_value: "ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id,
                message_type: "option".to_owned(),
                message_value: "!ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id,
                message_type: "property".to_owned(),
                message_value: "client_app=sample_console".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id,
                message_type: "schema".to_owned(),
                message_value: "sample_schema/sample_schema".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "start".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "success".to_owned(),
            },
        ]
    );
    drop(events);

    RimeSetNotificationHandler(None, std::ptr::null_mut());
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(
        notification_events()
            .lock()
            .expect("notification events should not be poisoned")
            .len(),
        6
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn creates_finds_and_destroys_sessions() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let session_id = RimeCreateSession();

    assert_ne!(session_id, 0);
    assert_eq!(RimeFindSession(session_id), TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);
    assert_eq!(RimeFindSession(session_id), FALSE);
}

#[test]
fn processes_ascii_keys_and_returns_unread_commit_once() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text` with a
    // valid NUL-terminated C string owned by the commit object.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert!(commit.text.is_null());
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn process_key_commits_numeric_candidate_selection() {
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
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text` with a
    // valid NUL-terminated C string owned by the commit object.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn select_candidate_apis_commit_current_candidates() {
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
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    assert_eq!(RimeSelectCandidate(session_id, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 1), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeSelectCandidateOnCurrentPage(session_id, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("八"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn highlight_candidate_apis_move_selection_without_commit() {
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
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    assert_eq!(RimeHighlightCandidate(session_id, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 1), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeChangePage(session_id, FALSE), TRUE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 0), TRUE);
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 5), FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeChangePage(session_id, TRUE), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("八"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn delete_candidate_apis_remove_menu_items_without_commit() {
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
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    assert_eq!(RimeDeleteCandidate(session_id, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeDeleteCandidate(session_id, 1), TRUE);
    assert_eq!(RimeDeleteCandidate(session_id, 99), FALSE);

    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 5);
    // SAFETY: `context.menu.candidates` points to initialized candidates.
    let second_candidate = unsafe { *context.menu.candidates.add(1) };
    // SAFETY: candidate text is a valid NUL-terminated string owned by the
    // context object.
    let second_text = unsafe { CStr::from_ptr(second_candidate.text) };
    assert_eq!(second_text.to_str(), Ok("爸"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeChangePage(session_id, FALSE), TRUE);
    assert_eq!(RimeDeleteCandidateOnCurrentPage(session_id, 0), TRUE);
    assert_eq!(RimeDeleteCandidateOnCurrentPage(session_id, 5), FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.num_candidates, 5);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDeleteCandidate(0, 0), FALSE);
    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn commits_composition_explicitly_and_returns_unread_commit() {
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
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    assert_eq!(RimeCommitComposition(session_id), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeCommitComposition(session_id), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("你"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn clears_composition_without_creating_commit() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    RimeClearComposition(session_id);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn gets_and_sets_input_and_caret_position() {
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
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let mut context = empty_context();

    assert_eq!(RimeGetInput(0), std::ptr::null());
    assert_eq!(RimeGetCaretPos(0), 0);
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 2);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    let input = unsafe { CStr::from_ptr(input) };
    assert_eq!(input.to_str(), Ok("ni"));

    RimeSetCaretPos(session_id, 1);
    assert_eq!(RimeGetCaretPos(session_id), 1);
    RimeSetCaretPos(session_id, 10);
    assert_eq!(RimeGetCaretPos(session_id), 2);

    let new_input = CString::new("ni").expect("literal should not contain NUL");
    // SAFETY: `new_input` is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeSetInput(session_id, new_input.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: `context.menu.candidates` points to initialized candidates.
    let first_candidate = unsafe { *context.menu.candidates };
    // SAFETY: candidate text is a valid NUL-terminated string owned by the
    // context object.
    let first_candidate_text = unsafe { CStr::from_ptr(first_candidate.text) };
    assert_eq!(first_candidate_text.to_str(), Ok("你"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: null pointers are explicitly rejected.
    assert_eq!(unsafe { RimeSetInput(session_id, std::ptr::null()) }, FALSE);
    // SAFETY: `new_input` is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeSetInput(session_id + 1, new_input.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn returns_context_with_preedit_and_candidate_page() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);

    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert_eq!(context.composition.cursor_pos, 2);
    assert_eq!(context.composition.sel_start, 0);
    assert_eq!(context.composition.sel_end, 2);
    // SAFETY: `RimeGetContext` returned true and populated owned C strings.
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) };
    assert_eq!(preedit.to_str(), Ok("ni"));

    assert_eq!(context.menu.page_size, 5);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.is_last_page, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    assert_eq!(context.menu.num_candidates, 1);
    assert!(!context.menu.candidates.is_null());
    // SAFETY: `context.menu.candidates` points to one initialized candidate.
    let candidate = unsafe { *context.menu.candidates };
    // SAFETY: candidate strings are valid NUL-terminated strings owned by
    // the context object.
    let candidate_text = unsafe { CStr::from_ptr(candidate.text) };
    assert_eq!(candidate_text.to_str(), Ok("ni"));
    // SAFETY: the echo candidate includes a non-null comment.
    let candidate_comment = unsafe { CStr::from_ptr(candidate.comment) };
    assert_eq!(candidate_comment.to_str(), Ok("echo"));

    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert!(context.composition.preedit.is_null());
    assert!(context.menu.candidates.is_null());
    assert_eq!(context.menu.num_candidates, 0);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn iterates_candidate_list_from_current_context() {
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
    let mut iterator = empty_candidate_list_iterator();

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    // SAFETY: `iterator` points to valid writable storage.
    assert_eq!(
        unsafe { RimeCandidateListBegin(session_id, &mut iterator) },
        TRUE
    );
    // SAFETY: `iterator` was initialized by `RimeCandidateListBegin`.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let first_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(first_text.to_str(), Ok("八"));
    // SAFETY: current candidate includes a non-null comment.
    let first_comment = unsafe { CStr::from_ptr(iterator.candidate.comment) };
    assert_eq!(first_comment.to_str(), Ok("ba"));
    // SAFETY: `iterator` remains valid and owns the current candidate.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let second_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(second_text.to_str(), Ok("吧"));
    // SAFETY: `iterator` was initialized by this API and can be ended once.
    unsafe { RimeCandidateListEnd(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert!(iterator.candidate.text.is_null());
    assert!(iterator.candidate.comment.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn candidate_list_can_start_from_index_and_rejects_empty_menu() {
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
    let mut iterator = empty_candidate_list_iterator();

    // SAFETY: `iterator` points to valid writable storage.
    assert_eq!(
        unsafe { RimeCandidateListBegin(session_id, &mut iterator) },
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    // SAFETY: `iterator` points to valid writable storage.
    assert_eq!(
        unsafe { RimeCandidateListFromIndex(session_id, &mut iterator, 1) },
        TRUE
    );
    // SAFETY: `iterator` was initialized by `RimeCandidateListFromIndex`.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(text.to_str(), Ok("吧"));
    // SAFETY: `iterator` was initialized by this API and can be ended once.
    unsafe { RimeCandidateListEnd(&mut iterator) };

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn returns_status_with_schema_and_composing_flags() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut status = empty_status();

    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: `RimeGetStatus` returned true and populated owned C strings.
    let schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    // SAFETY: `RimeGetStatus` returned true and populated owned C strings.
    let schema_name = unsafe { CStr::from_ptr(status.schema_name) };
    assert_eq!(schema_id.to_str(), Ok("default"));
    assert_eq!(schema_name.to_str(), Ok("Default"));
    assert_eq!(status.is_composing, FALSE);
    assert_eq!(status.is_ascii_mode, FALSE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);
    assert!(status.schema_id.is_null());
    assert!(status.schema_name.is_null());

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn sets_and_gets_runtime_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let custom_toggle = CString::new("custom_toggle").expect("option name should be valid");
    let mut status = empty_status();

    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    // SAFETY: option names are valid nul-terminated C strings.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    // SAFETY: option names are valid nul-terminated C strings.
    unsafe { RimeSetOption(session_id, custom_toggle.as_ptr(), TRUE) };

    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, custom_toggle.as_ptr()) },
        TRUE
    );
    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_ascii_mode, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    // SAFETY: option names are valid nul-terminated C strings.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    assert_eq!(unsafe { RimeGetOption(0, ascii_mode.as_ptr()) }, FALSE);
    assert_eq!(
        unsafe { RimeGetOption(session_id, std::ptr::null()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn sets_and_gets_runtime_properties() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let property = CString::new("client_app").expect("property name should be valid");
    let value = CString::new("sample_console").expect("property value should be valid");
    let empty_value = CString::new("").expect("property value should be valid");
    let mut buffer = vec![0 as c_char; 32];

    // SAFETY: property name is valid and buffer points to writable storage.
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        FALSE
    );

    // SAFETY: property name and value are valid nul-terminated C strings.
    unsafe { RimeSetProperty(session_id, property.as_ptr(), value.as_ptr()) };
    // SAFETY: property name is valid and buffer points to writable storage.
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        TRUE
    );
    // SAFETY: `RimeGetProperty` returned true and wrote a trailing NUL into
    // the caller-owned buffer.
    let copied_value = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_value.to_str(), Ok("sample_console"));

    let mut short_buffer = vec![0 as c_char; 7];
    // SAFETY: property name is valid and buffer points to writable storage.
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                short_buffer.as_mut_ptr(),
                short_buffer.len(),
            )
        },
        TRUE
    );
    // SAFETY: the shim always NUL-terminates non-empty buffers.
    let truncated_value = unsafe { CStr::from_ptr(short_buffer.as_ptr()) };
    assert_eq!(truncated_value.to_str(), Ok("sample"));

    // SAFETY: empty properties are accepted on set but rejected on get, as
    // librime treats empty property values as absent.
    unsafe { RimeSetProperty(session_id, property.as_ptr(), empty_value.as_ptr()) };
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        FALSE
    );
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                std::ptr::null_mut(),
                buffer.len(),
            )
        },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetProperty(session_id, std::ptr::null(), buffer.as_mut_ptr(), 0) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
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
    // SAFETY: `RimeGetCurrentSchema` wrote a trailing NUL into buffer.
    let truncated_schema = unsafe { CStr::from_ptr(short_buffer.as_ptr()) };
    assert_eq!(truncated_schema.to_str(), Ok("sample_"));

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
    assert_eq!(schema_list.size, 3);
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

#[test]
fn simulates_librime_style_key_sequences() {
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
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{space}").expect("key sequence should be valid");
    let invalid_sequence =
        CString::new("x{Unknown}").expect("key sequence should be valid C string");
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    // SAFETY: sequence is a valid nul-terminated C string.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("你"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    // SAFETY: invalid sequence is a valid C string but should fail parsing.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, invalid_sequence.as_ptr()) },
        FALSE
    );
    // SAFETY: parse failures should not partially apply the leading `x`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: null and invalid sessions are explicitly rejected.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, std::ptr::null()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id + 1, sequence.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rejects_invalid_context_requests() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut context = empty_context();
    context.data_size = 0;

    // SAFETY: `context` points to writable storage but has invalid
    // librime-style data_size metadata.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, FALSE);
    // SAFETY: null pointers are explicitly rejected.
    assert_eq!(
        unsafe { RimeGetContext(session_id, std::ptr::null_mut()) },
        FALSE
    );
    // SAFETY: `context` points to writable storage but has invalid
    // librime-style data_size metadata.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rejects_invalid_status_requests() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut status = empty_status();
    status.data_size = 0;

    // SAFETY: `status` points to writable storage but has invalid
    // librime-style data_size metadata.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, FALSE);
    // SAFETY: null pointers are explicitly rejected.
    assert_eq!(
        unsafe { RimeGetStatus(session_id, std::ptr::null_mut()) },
        FALSE
    );
    // SAFETY: `status` points to writable storage but has invalid
    // librime-style data_size metadata.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rejects_unknown_sessions_and_modified_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();

    assert_eq!(RimeProcessKey(0, 'a' as i32, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id + 1, 'a' as i32, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 1), FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}
