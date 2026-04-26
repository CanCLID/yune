use std::{
    collections::{HashMap, HashSet},
    ffi::{c_void, CStr, CString},
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr, slice,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_yaml::{Mapping, Number, Value};
use yune_core::{parse_key_sequence, Engine, KeyCode, KeyEvent, KeyModifiers};

pub type RimeSessionId = usize;
pub type Bool = c_int;
pub type RimeNotificationHandler = extern "C" fn(
    context_object: *mut c_void,
    session_id: RimeSessionId,
    message_type: *const c_char,
    message_value: *const c_char,
);

pub const FALSE: Bool = 0;
pub const TRUE: Bool = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeTraits {
    pub data_size: c_int,
    pub shared_data_dir: *const c_char,
    pub user_data_dir: *const c_char,
    pub distribution_name: *const c_char,
    pub distribution_code_name: *const c_char,
    pub distribution_version: *const c_char,
    pub app_name: *const c_char,
    pub modules: *const *const c_char,
    pub min_log_level: c_int,
    pub log_dir: *const c_char,
    pub prebuilt_data_dir: *const c_char,
    pub staging_dir: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCommit {
    pub data_size: c_int,
    pub text: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeComposition {
    pub length: c_int,
    pub cursor_pos: c_int,
    pub sel_start: c_int,
    pub sel_end: c_int,
    pub preedit: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCandidate {
    pub text: *mut c_char,
    pub comment: *mut c_char,
    pub reserved: *mut std::ffi::c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeMenu {
    pub page_size: c_int,
    pub page_no: c_int,
    pub is_last_page: Bool,
    pub highlighted_candidate_index: c_int,
    pub num_candidates: c_int,
    pub candidates: *mut RimeCandidate,
    pub select_keys: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeContext {
    pub data_size: c_int,
    pub composition: RimeComposition,
    pub menu: RimeMenu,
    pub commit_text_preview: *mut c_char,
    pub select_labels: *mut *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeStatus {
    pub data_size: c_int,
    pub schema_id: *mut c_char,
    pub schema_name: *mut c_char,
    pub is_disabled: Bool,
    pub is_composing: Bool,
    pub is_ascii_mode: Bool,
    pub is_full_shape: Bool,
    pub is_simplified: Bool,
    pub is_traditional: Bool,
    pub is_ascii_punct: Bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCandidateListIterator {
    pub ptr: *mut c_void,
    pub index: c_int,
    pub candidate: RimeCandidate,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeSchemaListItem {
    pub schema_id: *mut c_char,
    pub name: *mut c_char,
    pub reserved: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeSchemaList {
    pub size: usize,
    pub list: *mut RimeSchemaListItem,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeStringSlice {
    pub str: *const c_char,
    pub length: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeConfig {
    pub ptr: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeConfigIterator {
    pub list: *mut c_void,
    pub map: *mut c_void,
    pub index: c_int,
    pub key: *const c_char,
    pub path: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCustomApi {
    pub data_size: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeModule {
    pub data_size: c_int,
    pub module_name: *const c_char,
    pub initialize: Option<extern "C" fn()>,
    pub finalize: Option<extern "C" fn()>,
    pub get_api: Option<extern "C" fn() -> *mut RimeCustomApi>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCustomSettings {
    pub placeholder: c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeSwitcherSettings {
    pub placeholder: c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeSchemaInfo {
    pub placeholder: c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeUserDictIterator {
    pub ptr: *mut c_void,
    pub i: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RimeLeversApi {
    pub data_size: c_int,
    pub custom_settings_init:
        Option<unsafe extern "C" fn(*const c_char, *const c_char) -> *mut RimeCustomSettings>,
    pub custom_settings_destroy: Option<unsafe extern "C" fn(*mut RimeCustomSettings)>,
    pub load_settings: Option<unsafe extern "C" fn(*mut RimeCustomSettings) -> Bool>,
    pub save_settings: Option<unsafe extern "C" fn(*mut RimeCustomSettings) -> Bool>,
    pub customize_bool:
        Option<unsafe extern "C" fn(*mut RimeCustomSettings, *const c_char, Bool) -> Bool>,
    pub customize_int:
        Option<unsafe extern "C" fn(*mut RimeCustomSettings, *const c_char, c_int) -> Bool>,
    pub customize_double:
        Option<unsafe extern "C" fn(*mut RimeCustomSettings, *const c_char, f64) -> Bool>,
    pub customize_string:
        Option<unsafe extern "C" fn(*mut RimeCustomSettings, *const c_char, *const c_char) -> Bool>,
    pub is_first_run: Option<unsafe extern "C" fn(*mut RimeCustomSettings) -> Bool>,
    pub settings_is_modified: Option<unsafe extern "C" fn(*mut RimeCustomSettings) -> Bool>,
    pub settings_get_config:
        Option<unsafe extern "C" fn(*mut RimeCustomSettings, *mut RimeConfig) -> Bool>,
    pub switcher_settings_init: Option<extern "C" fn() -> *mut RimeSwitcherSettings>,
    pub get_available_schema_list:
        Option<unsafe extern "C" fn(*mut RimeSwitcherSettings, *mut RimeSchemaList) -> Bool>,
    pub get_selected_schema_list:
        Option<unsafe extern "C" fn(*mut RimeSwitcherSettings, *mut RimeSchemaList) -> Bool>,
    pub schema_list_destroy: Option<unsafe extern "C" fn(*mut RimeSchemaList)>,
    pub get_schema_id: Option<unsafe extern "C" fn(*mut RimeSchemaInfo) -> *const c_char>,
    pub get_schema_name: Option<unsafe extern "C" fn(*mut RimeSchemaInfo) -> *const c_char>,
    pub get_schema_version: Option<unsafe extern "C" fn(*mut RimeSchemaInfo) -> *const c_char>,
    pub get_schema_author: Option<unsafe extern "C" fn(*mut RimeSchemaInfo) -> *const c_char>,
    pub get_schema_description: Option<unsafe extern "C" fn(*mut RimeSchemaInfo) -> *const c_char>,
    pub get_schema_file_path: Option<unsafe extern "C" fn(*mut RimeSchemaInfo) -> *const c_char>,
    pub select_schemas: Option<
        unsafe extern "C" fn(*mut RimeSwitcherSettings, *const *const c_char, c_int) -> Bool,
    >,
    pub get_hotkeys: Option<unsafe extern "C" fn(*mut RimeSwitcherSettings) -> *const c_char>,
    pub set_hotkeys: Option<unsafe extern "C" fn(*mut RimeSwitcherSettings, *const c_char) -> Bool>,
    pub user_dict_iterator_init: Option<unsafe extern "C" fn(*mut RimeUserDictIterator) -> Bool>,
    pub user_dict_iterator_destroy: Option<unsafe extern "C" fn(*mut RimeUserDictIterator)>,
    pub next_user_dict: Option<unsafe extern "C" fn(*mut RimeUserDictIterator) -> *const c_char>,
    pub backup_user_dict: Option<unsafe extern "C" fn(*const c_char) -> Bool>,
    pub restore_user_dict: Option<unsafe extern "C" fn(*const c_char) -> Bool>,
    pub export_user_dict: Option<unsafe extern "C" fn(*const c_char, *const c_char) -> c_int>,
    pub import_user_dict: Option<unsafe extern "C" fn(*const c_char, *const c_char) -> c_int>,
    pub customize_item: Option<
        unsafe extern "C" fn(*mut RimeCustomSettings, *const c_char, *mut RimeConfig) -> Bool,
    >,
}

type SetupFn = unsafe extern "C" fn(*const RimeTraits);
type SetNotificationHandlerFn = extern "C" fn(Option<RimeNotificationHandler>, *mut c_void);
type NoArgBoolFn = extern "C" fn() -> Bool;
type ConfigOpenFn = unsafe extern "C" fn(*const c_char, *mut RimeConfig) -> Bool;
type ConfigCloseFn = unsafe extern "C" fn(*mut RimeConfig) -> Bool;
type ConfigGetBoolFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, *mut Bool) -> Bool;
type ConfigGetIntFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, *mut c_int) -> Bool;
type ConfigGetDoubleFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, *mut f64) -> Bool;
type ConfigGetStringFn =
    unsafe extern "C" fn(*mut RimeConfig, *const c_char, *mut c_char, usize) -> Bool;
type ConfigGetCStringFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char) -> *const c_char;
type ConfigIteratorBeginFn =
    unsafe extern "C" fn(*mut RimeConfigIterator, *mut RimeConfig, *const c_char) -> Bool;
type ConfigIteratorNextFn = unsafe extern "C" fn(*mut RimeConfigIterator) -> Bool;
type ConfigIteratorEndFn = unsafe extern "C" fn(*mut RimeConfigIterator);
type ConfigSetBoolFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, Bool) -> Bool;
type ConfigSetIntFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, c_int) -> Bool;
type ConfigSetDoubleFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, f64) -> Bool;
type ConfigSetStringFn =
    unsafe extern "C" fn(*mut RimeConfig, *const c_char, *const c_char) -> Bool;
type ConfigItemFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, *mut RimeConfig) -> Bool;
type ConfigKeyFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char) -> Bool;
type ConfigListSizeFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char) -> usize;
type ProtoFn = extern "C" fn(RimeSessionId, *mut c_void);
type GetStateLabelFn = unsafe extern "C" fn(RimeSessionId, *const c_char, Bool) -> *const c_char;
type GetStateLabelAbbreviatedFn =
    unsafe extern "C" fn(RimeSessionId, *const c_char, Bool, Bool) -> RimeStringSlice;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RimeApi {
    pub data_size: c_int,
    pub setup: Option<SetupFn>,
    pub set_notification_handler: Option<SetNotificationHandlerFn>,
    pub initialize: Option<SetupFn>,
    pub finalize: Option<extern "C" fn()>,
    pub start_maintenance: Option<extern "C" fn(Bool) -> Bool>,
    pub is_maintenance_mode: Option<NoArgBoolFn>,
    pub join_maintenance_thread: Option<extern "C" fn()>,
    pub deployer_initialize: Option<SetupFn>,
    pub prebuild: Option<NoArgBoolFn>,
    pub deploy: Option<NoArgBoolFn>,
    pub deploy_schema: Option<extern "C" fn(*const c_char) -> Bool>,
    pub deploy_config_file: Option<extern "C" fn(*const c_char, *const c_char) -> Bool>,
    pub sync_user_data: Option<NoArgBoolFn>,
    pub create_session: Option<extern "C" fn() -> RimeSessionId>,
    pub find_session: Option<extern "C" fn(RimeSessionId) -> Bool>,
    pub destroy_session: Option<extern "C" fn(RimeSessionId) -> Bool>,
    pub cleanup_stale_sessions: Option<extern "C" fn()>,
    pub cleanup_all_sessions: Option<extern "C" fn()>,
    pub process_key: Option<extern "C" fn(RimeSessionId, c_int, c_int) -> Bool>,
    pub commit_composition: Option<extern "C" fn(RimeSessionId) -> Bool>,
    pub clear_composition: Option<extern "C" fn(RimeSessionId)>,
    pub get_commit: Option<unsafe extern "C" fn(RimeSessionId, *mut RimeCommit) -> Bool>,
    pub free_commit: Option<unsafe extern "C" fn(*mut RimeCommit) -> Bool>,
    pub get_context: Option<unsafe extern "C" fn(RimeSessionId, *mut RimeContext) -> Bool>,
    pub free_context: Option<unsafe extern "C" fn(*mut RimeContext) -> Bool>,
    pub get_status: Option<unsafe extern "C" fn(RimeSessionId, *mut RimeStatus) -> Bool>,
    pub free_status: Option<unsafe extern "C" fn(*mut RimeStatus) -> Bool>,
    pub set_option: Option<unsafe extern "C" fn(RimeSessionId, *const c_char, Bool)>,
    pub get_option: Option<unsafe extern "C" fn(RimeSessionId, *const c_char) -> Bool>,
    pub set_property: Option<unsafe extern "C" fn(RimeSessionId, *const c_char, *const c_char)>,
    pub get_property:
        Option<unsafe extern "C" fn(RimeSessionId, *const c_char, *mut c_char, usize) -> Bool>,
    pub get_schema_list: Option<unsafe extern "C" fn(*mut RimeSchemaList) -> Bool>,
    pub free_schema_list: Option<unsafe extern "C" fn(*mut RimeSchemaList)>,
    pub get_current_schema: Option<unsafe extern "C" fn(RimeSessionId, *mut c_char, usize) -> Bool>,
    pub select_schema: Option<unsafe extern "C" fn(RimeSessionId, *const c_char) -> Bool>,
    pub schema_open: Option<ConfigOpenFn>,
    pub config_open: Option<ConfigOpenFn>,
    pub config_close: Option<ConfigCloseFn>,
    pub config_get_bool: Option<ConfigGetBoolFn>,
    pub config_get_int: Option<ConfigGetIntFn>,
    pub config_get_double: Option<ConfigGetDoubleFn>,
    pub config_get_string: Option<ConfigGetStringFn>,
    pub config_get_cstring: Option<ConfigGetCStringFn>,
    pub config_update_signature:
        Option<unsafe extern "C" fn(*mut RimeConfig, *const c_char) -> Bool>,
    pub config_begin_map: Option<ConfigIteratorBeginFn>,
    pub config_next: Option<ConfigIteratorNextFn>,
    pub config_end: Option<ConfigIteratorEndFn>,
    pub simulate_key_sequence: Option<unsafe extern "C" fn(RimeSessionId, *const c_char) -> Bool>,
    pub register_module: Option<unsafe extern "C" fn(*mut RimeModule) -> Bool>,
    pub find_module: Option<unsafe extern "C" fn(*const c_char) -> *mut RimeModule>,
    pub run_task: Option<extern "C" fn(*const c_char) -> Bool>,
    pub get_shared_data_dir: Option<extern "C" fn() -> *const c_char>,
    pub get_user_data_dir: Option<extern "C" fn() -> *const c_char>,
    pub get_sync_dir: Option<extern "C" fn() -> *const c_char>,
    pub get_user_id: Option<extern "C" fn() -> *const c_char>,
    pub get_user_data_sync_dir: Option<unsafe extern "C" fn(*mut c_char, usize)>,
    pub config_init: Option<unsafe extern "C" fn(*mut RimeConfig) -> Bool>,
    pub config_load_string: Option<unsafe extern "C" fn(*mut RimeConfig, *const c_char) -> Bool>,
    pub config_set_bool: Option<ConfigSetBoolFn>,
    pub config_set_int: Option<ConfigSetIntFn>,
    pub config_set_double: Option<ConfigSetDoubleFn>,
    pub config_set_string: Option<ConfigSetStringFn>,
    pub config_get_item: Option<ConfigItemFn>,
    pub config_set_item: Option<ConfigItemFn>,
    pub config_clear: Option<ConfigKeyFn>,
    pub config_create_list: Option<ConfigKeyFn>,
    pub config_create_map: Option<ConfigKeyFn>,
    pub config_list_size: Option<ConfigListSizeFn>,
    pub config_begin_list: Option<ConfigIteratorBeginFn>,
    pub get_input: Option<extern "C" fn(RimeSessionId) -> *const c_char>,
    pub get_caret_pos: Option<extern "C" fn(RimeSessionId) -> usize>,
    pub select_candidate: Option<extern "C" fn(RimeSessionId, usize) -> Bool>,
    pub get_version: Option<extern "C" fn() -> *const c_char>,
    pub set_caret_pos: Option<extern "C" fn(RimeSessionId, usize)>,
    pub select_candidate_on_current_page: Option<extern "C" fn(RimeSessionId, usize) -> Bool>,
    pub candidate_list_begin:
        Option<unsafe extern "C" fn(RimeSessionId, *mut RimeCandidateListIterator) -> Bool>,
    pub candidate_list_next: Option<unsafe extern "C" fn(*mut RimeCandidateListIterator) -> Bool>,
    pub candidate_list_end: Option<unsafe extern "C" fn(*mut RimeCandidateListIterator)>,
    pub user_config_open: Option<ConfigOpenFn>,
    pub candidate_list_from_index:
        Option<unsafe extern "C" fn(RimeSessionId, *mut RimeCandidateListIterator, c_int) -> Bool>,
    pub get_prebuilt_data_dir: Option<extern "C" fn() -> *const c_char>,
    pub get_staging_dir: Option<extern "C" fn() -> *const c_char>,
    pub commit_proto: Option<ProtoFn>,
    pub context_proto: Option<ProtoFn>,
    pub status_proto: Option<ProtoFn>,
    pub get_state_label: Option<GetStateLabelFn>,
    pub delete_candidate: Option<extern "C" fn(RimeSessionId, usize) -> Bool>,
    pub delete_candidate_on_current_page: Option<extern "C" fn(RimeSessionId, usize) -> Bool>,
    pub get_state_label_abbreviated: Option<GetStateLabelAbbreviatedFn>,
    pub set_input: Option<unsafe extern "C" fn(RimeSessionId, *const c_char) -> Bool>,
    pub get_shared_data_dir_s: Option<unsafe extern "C" fn(*mut c_char, usize)>,
    pub get_user_data_dir_s: Option<unsafe extern "C" fn(*mut c_char, usize)>,
    pub get_prebuilt_data_dir_s: Option<unsafe extern "C" fn(*mut c_char, usize)>,
    pub get_staging_dir_s: Option<unsafe extern "C" fn(*mut c_char, usize)>,
    pub get_sync_dir_s: Option<unsafe extern "C" fn(*mut c_char, usize)>,
    pub highlight_candidate: Option<extern "C" fn(RimeSessionId, usize) -> Bool>,
    pub highlight_candidate_on_current_page: Option<extern "C" fn(RimeSessionId, usize) -> Bool>,
    pub change_page: Option<extern "C" fn(RimeSessionId, Bool) -> Bool>,
}

const XK_BACKSPACE: c_int = 0xff08;
const XK_RETURN: c_int = 0xff0d;
const DEFAULT_PAGE_SIZE: usize = 5;
const RIME_VERSION_BYTES: &[u8] =
    concat!("yune-rime-api ", env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[derive(Default)]
struct SessionRegistry {
    next_id: RimeSessionId,
    sessions: HashMap<RimeSessionId, SessionState>,
}

impl SessionRegistry {
    fn create_session(&mut self) -> RimeSessionId {
        self.next_id = self.next_id.saturating_add(1).max(1);
        let session_id = self.next_id;
        self.sessions.insert(session_id, SessionState::default());
        session_id
    }
}

#[derive(Default)]
struct SessionState {
    engine: Engine,
    unread_commit: Option<String>,
    input_buffer: Option<CString>,
}

struct CandidateListState {
    candidates: Vec<yune_core::Candidate>,
}

struct ConfigState {
    root: Value,
    cstring_cache: Option<CString>,
}

struct ConfigIteratorState {
    entries: Vec<(String, String)>,
    key_cache: Option<CString>,
    path_cache: Option<CString>,
}

struct UserDictListState {
    names: Vec<CString>,
}

struct LeverCustomSettings {
    config_id: String,
    generator_id: String,
    config: ConfigState,
    custom_config: ConfigState,
    modified: bool,
}

struct LeverSchemaInfo {
    schema_id: CString,
    name: CString,
    version: Option<CString>,
    author: Option<CString>,
    description: Option<CString>,
    file_path: Option<CString>,
}

struct StateLabel {
    value: String,
    length: usize,
}

#[derive(Clone, Copy)]
enum ConfigOpenKind {
    Deployed,
    User,
}

impl Default for ConfigState {
    fn default() -> Self {
        Self {
            root: Value::Mapping(Mapping::new()),
            cstring_cache: None,
        }
    }
}

struct RuntimePaths {
    shared_data_dir: CString,
    user_data_dir: CString,
    prebuilt_data_dir: CString,
    staging_dir: CString,
    sync_dir: CString,
    user_id: CString,
    user_data_sync_dir: CString,
    distribution_code_name: CString,
    distribution_version: CString,
    backup_config_files: bool,
}

struct RuntimePathArgs<'a> {
    shared_data_dir: &'a str,
    user_data_dir: &'a str,
    prebuilt_data_dir: &'a str,
    staging_dir: &'a str,
    sync_dir: &'a str,
    user_id: &'a str,
    distribution: (&'a str, &'a str),
    backup_config_files: bool,
}

#[derive(Default)]
struct NotificationState {
    handler: Option<RimeNotificationHandler>,
    context_object: usize,
}

#[derive(Default)]
struct ModuleRegistry {
    modules_by_name: HashMap<String, usize>,
}

#[derive(Default)]
struct InstallationSettings {
    loaded: bool,
    installation_id: Option<String>,
    sync_dir: Option<String>,
    backup_config_files: Option<bool>,
}

impl Default for RuntimePaths {
    fn default() -> Self {
        Self::new(RuntimePathArgs {
            shared_data_dir: ".",
            user_data_dir: ".",
            prebuilt_data_dir: "build",
            staging_dir: "build",
            sync_dir: "sync",
            user_id: "unknown",
            distribution: ("", ""),
            backup_config_files: true,
        })
    }
}

impl RuntimePaths {
    fn new(args: RuntimePathArgs<'_>) -> Self {
        let user_data_sync_dir = path_join(args.sync_dir, args.user_id);
        Self {
            shared_data_dir: cstring_from_lossless_str(args.shared_data_dir),
            user_data_dir: cstring_from_lossless_str(args.user_data_dir),
            prebuilt_data_dir: cstring_from_lossless_str(args.prebuilt_data_dir),
            staging_dir: cstring_from_lossless_str(args.staging_dir),
            sync_dir: cstring_from_lossless_str(args.sync_dir),
            user_id: cstring_from_lossless_str(args.user_id),
            user_data_sync_dir: cstring_from_lossless_str(&user_data_sync_dir),
            distribution_code_name: cstring_from_lossless_str(args.distribution.0),
            distribution_version: cstring_from_lossless_str(args.distribution.1),
            backup_config_files: args.backup_config_files,
        }
    }

    unsafe fn from_traits(traits: *const RimeTraits) -> Option<Self> {
        if traits.is_null() {
            return None;
        }

        // SAFETY: callers promise that `traits`, when non-null, points to a
        // valid `RimeTraits` object whose optional C strings are NUL-terminated.
        let traits = unsafe { &*traits };
        let shared_data_dir =
            optional_c_string(traits.shared_data_dir).unwrap_or_else(|| ".".to_owned());
        let user_data_dir =
            optional_c_string(traits.user_data_dir).unwrap_or_else(|| ".".to_owned());
        let prebuilt_data_dir = optional_c_string(traits.prebuilt_data_dir)
            .unwrap_or_else(|| path_join(&shared_data_dir, "build"));
        let staging_dir = optional_c_string(traits.staging_dir)
            .unwrap_or_else(|| path_join(&user_data_dir, "build"));
        let distribution_code_name =
            optional_c_string(traits.distribution_code_name).unwrap_or_default();
        let distribution_version =
            optional_c_string(traits.distribution_version).unwrap_or_default();
        let installation = read_installation_settings(&user_data_dir);
        let sync_dir = if let Some(sync_dir) = installation.sync_dir {
            sync_dir
        } else if installation.loaded {
            path_join(&user_data_dir, "sync")
        } else {
            "sync".to_owned()
        };
        let user_id = installation
            .installation_id
            .unwrap_or_else(|| "unknown".to_owned());
        let backup_config_files = installation.backup_config_files.unwrap_or(true);

        Some(Self::new(RuntimePathArgs {
            shared_data_dir: &shared_data_dir,
            user_data_dir: &user_data_dir,
            prebuilt_data_dir: &prebuilt_data_dir,
            staging_dir: &staging_dir,
            sync_dir: &sync_dir,
            user_id: &user_id,
            distribution: (&distribution_code_name, &distribution_version),
            backup_config_files,
        }))
    }
}

fn read_installation_settings(user_data_dir: &str) -> InstallationSettings {
    let path = Path::new(user_data_dir).join("installation.yaml");
    let Ok(text) = fs::read_to_string(path) else {
        return InstallationSettings::default();
    };
    let Ok(Value::Mapping(root)) = serde_yaml::from_str::<Value>(&text) else {
        return InstallationSettings::default();
    };

    InstallationSettings {
        loaded: true,
        installation_id: root
            .get(Value::String("installation_id".to_owned()))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        sync_dir: root
            .get(Value::String("sync_dir".to_owned()))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        backup_config_files: root
            .get(Value::String("backup_config_files".to_owned()))
            .and_then(Value::as_bool),
    }
}

fn sessions() -> &'static Mutex<SessionRegistry> {
    static SESSIONS: OnceLock<Mutex<SessionRegistry>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(SessionRegistry::default()))
}

fn runtime_paths() -> &'static Mutex<RuntimePaths> {
    static RUNTIME_PATHS: OnceLock<Mutex<RuntimePaths>> = OnceLock::new();
    RUNTIME_PATHS.get_or_init(|| Mutex::new(RuntimePaths::default()))
}

fn notification_state() -> &'static Mutex<NotificationState> {
    static NOTIFICATION_STATE: OnceLock<Mutex<NotificationState>> = OnceLock::new();
    NOTIFICATION_STATE.get_or_init(|| Mutex::new(NotificationState::default()))
}

fn module_registry() -> &'static Mutex<ModuleRegistry> {
    static MODULE_REGISTRY: OnceLock<Mutex<ModuleRegistry>> = OnceLock::new();
    MODULE_REGISTRY.get_or_init(|| Mutex::new(ModuleRegistry::default()))
}

fn switcher_selection_registry() -> &'static Mutex<HashMap<usize, Option<Vec<String>>>> {
    static SWITCHER_SELECTION_REGISTRY: OnceLock<Mutex<HashMap<usize, Option<Vec<String>>>>> =
        OnceLock::new();
    SWITCHER_SELECTION_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn levers_module() -> *mut RimeModule {
    static LEVERS_MODULE: OnceLock<usize> = OnceLock::new();
    *LEVERS_MODULE.get_or_init(|| {
        Box::into_raw(Box::new(RimeModule {
            data_size: (std::mem::size_of::<RimeModule>() - std::mem::size_of::<c_int>()) as c_int,
            module_name: c"levers".as_ptr(),
            initialize: None,
            finalize: None,
            get_api: Some(rime_levers_get_api),
        })) as usize
    }) as *mut RimeModule
}

fn levers_api_entry() -> *mut RimeLeversApi {
    static LEVERS_API: OnceLock<usize> = OnceLock::new();
    *LEVERS_API.get_or_init(|| Box::into_raw(Box::new(build_levers_api())) as usize)
        as *mut RimeLeversApi
}

fn build_levers_api() -> RimeLeversApi {
    RimeLeversApi {
        data_size: (std::mem::size_of::<RimeLeversApi>() - std::mem::size_of::<c_int>()) as c_int,
        custom_settings_init: Some(RimeLeversCustomSettingsInit),
        custom_settings_destroy: Some(RimeLeversCustomSettingsDestroy),
        load_settings: Some(RimeLeversLoadSettings),
        save_settings: Some(RimeLeversSaveSettings),
        customize_bool: Some(RimeLeversCustomizeBool),
        customize_int: Some(RimeLeversCustomizeInt),
        customize_double: Some(RimeLeversCustomizeDouble),
        customize_string: Some(RimeLeversCustomizeString),
        is_first_run: Some(RimeLeversIsFirstRun),
        settings_is_modified: Some(RimeLeversSettingsIsModified),
        settings_get_config: Some(RimeLeversSettingsGetConfig),
        switcher_settings_init: Some(RimeSwitcherSettingsInit),
        get_available_schema_list: Some(RimeLeversGetAvailableSchemaList),
        get_selected_schema_list: Some(RimeLeversGetSelectedSchemaList),
        schema_list_destroy: Some(RimeLeversSchemaListDestroy),
        get_schema_id: Some(RimeLeversGetSchemaId),
        get_schema_name: Some(RimeLeversGetSchemaName),
        get_schema_version: Some(RimeLeversGetSchemaVersion),
        get_schema_author: Some(RimeLeversGetSchemaAuthor),
        get_schema_description: Some(RimeLeversGetSchemaDescription),
        get_schema_file_path: Some(RimeLeversGetSchemaFilePath),
        select_schemas: Some(RimeLeversSelectSchemas),
        get_hotkeys: Some(RimeLeversGetHotkeys),
        set_hotkeys: Some(RimeLeversSetHotkeys),
        user_dict_iterator_init: Some(RimeLeversUserDictIteratorInit),
        user_dict_iterator_destroy: Some(RimeLeversUserDictIteratorDestroy),
        next_user_dict: Some(RimeLeversNextUserDict),
        backup_user_dict: Some(RimeLeversBackupUserDict),
        restore_user_dict: Some(RimeLeversRestoreUserDict),
        export_user_dict: Some(RimeLeversExportUserDict),
        import_user_dict: Some(RimeLeversImportUserDict),
        customize_item: Some(RimeLeversCustomizeItem),
    }
}

fn state_label_cache() -> &'static Mutex<Option<CString>> {
    static STATE_LABEL_CACHE: OnceLock<Mutex<Option<CString>>> = OnceLock::new();
    STATE_LABEL_CACHE.get_or_init(|| Mutex::new(None))
}

fn switcher_hotkeys_cache() -> &'static Mutex<Option<CString>> {
    static SWITCHER_HOTKEYS_CACHE: OnceLock<Mutex<Option<CString>>> = OnceLock::new();
    SWITCHER_HOTKEYS_CACHE.get_or_init(|| Mutex::new(None))
}

fn api_entry() -> *mut RimeApi {
    static API: OnceLock<usize> = OnceLock::new();
    *API.get_or_init(|| Box::into_raw(Box::new(build_rime_api())) as usize) as *mut RimeApi
}

fn build_rime_api() -> RimeApi {
    RimeApi {
        data_size: (std::mem::size_of::<RimeApi>() - std::mem::size_of::<c_int>()) as c_int,
        setup: Some(RimeSetup),
        set_notification_handler: Some(RimeSetNotificationHandler),
        initialize: Some(RimeInitialize),
        finalize: Some(RimeFinalize),
        start_maintenance: Some(RimeStartMaintenance),
        is_maintenance_mode: Some(RimeIsMaintenancing),
        join_maintenance_thread: Some(RimeJoinMaintenanceThread),
        deployer_initialize: Some(RimeDeployerInitialize),
        prebuild: Some(RimePrebuildAllSchemas),
        deploy: Some(RimeDeployWorkspace),
        deploy_schema: Some(RimeDeploySchema),
        deploy_config_file: Some(RimeDeployConfigFile),
        sync_user_data: Some(RimeSyncUserData),
        create_session: Some(RimeCreateSession),
        find_session: Some(RimeFindSession),
        destroy_session: Some(RimeDestroySession),
        cleanup_stale_sessions: Some(RimeCleanupStaleSessions),
        cleanup_all_sessions: Some(RimeCleanupAllSessions),
        process_key: Some(RimeProcessKey),
        commit_composition: Some(RimeCommitComposition),
        clear_composition: Some(RimeClearComposition),
        get_commit: Some(RimeGetCommit),
        free_commit: Some(RimeFreeCommit),
        get_context: Some(RimeGetContext),
        free_context: Some(RimeFreeContext),
        get_status: Some(RimeGetStatus),
        free_status: Some(RimeFreeStatus),
        set_option: Some(RimeSetOption),
        get_option: Some(RimeGetOption),
        set_property: Some(RimeSetProperty),
        get_property: Some(RimeGetProperty),
        get_schema_list: Some(RimeGetSchemaList),
        free_schema_list: Some(RimeFreeSchemaList),
        get_current_schema: Some(RimeGetCurrentSchema),
        select_schema: Some(RimeSelectSchema),
        schema_open: Some(RimeSchemaOpen),
        config_open: Some(RimeConfigOpen),
        config_close: Some(RimeConfigClose),
        config_get_bool: Some(RimeConfigGetBool),
        config_get_int: Some(RimeConfigGetInt),
        config_get_double: Some(RimeConfigGetDouble),
        config_get_string: Some(RimeConfigGetString),
        config_get_cstring: Some(RimeConfigGetCString),
        config_update_signature: Some(RimeConfigUpdateSignature),
        config_begin_map: Some(RimeConfigBeginMap),
        config_next: Some(RimeConfigNext),
        config_end: Some(RimeConfigEnd),
        simulate_key_sequence: Some(RimeSimulateKeySequence),
        register_module: Some(RimeRegisterModule),
        find_module: Some(RimeFindModule),
        run_task: Some(RimeRunTask),
        get_shared_data_dir: Some(RimeGetSharedDataDir),
        get_user_data_dir: Some(RimeGetUserDataDir),
        get_sync_dir: Some(RimeGetSyncDir),
        get_user_id: Some(RimeGetUserId),
        get_user_data_sync_dir: Some(RimeGetUserDataSyncDir),
        config_init: Some(RimeConfigInit),
        config_load_string: Some(RimeConfigLoadString),
        config_set_bool: Some(RimeConfigSetBool),
        config_set_int: Some(RimeConfigSetInt),
        config_set_double: Some(RimeConfigSetDouble),
        config_set_string: Some(RimeConfigSetString),
        config_get_item: Some(RimeConfigGetItem),
        config_set_item: Some(RimeConfigSetItem),
        config_clear: Some(RimeConfigClear),
        config_create_list: Some(RimeConfigCreateList),
        config_create_map: Some(RimeConfigCreateMap),
        config_list_size: Some(RimeConfigListSize),
        config_begin_list: Some(RimeConfigBeginList),
        get_input: Some(RimeGetInput),
        get_caret_pos: Some(RimeGetCaretPos),
        select_candidate: Some(RimeSelectCandidate),
        get_version: Some(RimeGetVersion),
        set_caret_pos: Some(RimeSetCaretPos),
        select_candidate_on_current_page: Some(RimeSelectCandidateOnCurrentPage),
        candidate_list_begin: Some(RimeCandidateListBegin),
        candidate_list_next: Some(RimeCandidateListNext),
        candidate_list_end: Some(RimeCandidateListEnd),
        user_config_open: Some(RimeUserConfigOpen),
        candidate_list_from_index: Some(RimeCandidateListFromIndex),
        get_prebuilt_data_dir: Some(RimeGetPrebuiltDataDir),
        get_staging_dir: Some(RimeGetStagingDir),
        commit_proto: None,
        context_proto: None,
        status_proto: None,
        get_state_label: Some(RimeGetStateLabel),
        delete_candidate: Some(RimeDeleteCandidate),
        delete_candidate_on_current_page: Some(RimeDeleteCandidateOnCurrentPage),
        get_state_label_abbreviated: Some(RimeGetStateLabelAbbreviated),
        set_input: Some(RimeSetInput),
        get_shared_data_dir_s: Some(RimeGetSharedDataDirSecure),
        get_user_data_dir_s: Some(RimeGetUserDataDirSecure),
        get_prebuilt_data_dir_s: Some(RimeGetPrebuiltDataDirSecure),
        get_staging_dir_s: Some(RimeGetStagingDirSecure),
        get_sync_dir_s: Some(RimeGetSyncDirSecure),
        highlight_candidate: Some(RimeHighlightCandidate),
        highlight_candidate_on_current_page: Some(RimeHighlightCandidateOnCurrentPage),
        change_page: Some(RimeChangePage),
    }
}

#[must_use]
pub const fn bool_from(value: bool) -> Bool {
    if value {
        TRUE
    } else {
        FALSE
    }
}

unsafe fn levers_schema_info_ptr(
    info: *mut RimeSchemaInfo,
    getter: impl FnOnce(&LeverSchemaInfo) -> Option<*const c_char>,
) -> *const c_char {
    if info.is_null() {
        return ptr::null();
    }

    // SAFETY: callers pass the opaque pointer stored in a levers schema-list
    // item's `reserved` field. That pointer is allocated as `LeverSchemaInfo`
    // and remains valid until the schema list is destroyed.
    let info = unsafe { &*info.cast::<LeverSchemaInfo>() };
    getter(info).unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn rime_get_api() -> *mut RimeApi {
    api_entry()
}

#[no_mangle]
pub extern "C" fn rime_levers_get_api() -> *mut RimeCustomApi {
    levers_api_entry().cast::<RimeCustomApi>()
}

/// Stores process-wide runtime traits for later path queries.
///
/// # Safety
///
/// `traits` must be either null or a valid pointer to a `RimeTraits` object.
/// Any non-null string pointers in the traits object must be valid
/// NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn RimeSetup(traits: *const RimeTraits) {
    if let Some(paths) = unsafe { RuntimePaths::from_traits(traits) } {
        *runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned") = paths;
    }
}

#[no_mangle]
pub extern "C" fn RimeSetNotificationHandler(
    handler: Option<RimeNotificationHandler>,
    context_object: *mut c_void,
) {
    let mut state = notification_state()
        .lock()
        .expect("notification state should not be poisoned");
    state.handler = handler;
    state.context_object = context_object as usize;
}

/// Registers a process-wide module pointer by its module name.
///
/// # Safety
///
/// `module` must be either null or point to a valid `RimeModule` whose
/// `module_name`, when non-null, is a valid NUL-terminated C string. The caller
/// retains ownership and must keep the module storage alive while it may be
/// returned by `RimeFindModule`.
#[no_mangle]
pub unsafe extern "C" fn RimeRegisterModule(module: *mut RimeModule) -> Bool {
    if module.is_null() {
        return FALSE;
    }

    // SAFETY: callers promise `module` points to a valid RimeModule.
    let module_ref = unsafe { &*module };
    if module_ref.module_name.is_null() {
        return FALSE;
    }

    // SAFETY: callers promise `module_name` is a valid NUL-terminated C string.
    let module_name = unsafe { CStr::from_ptr(module_ref.module_name) }
        .to_string_lossy()
        .into_owned();
    module_registry()
        .lock()
        .expect("module registry should not be poisoned")
        .modules_by_name
        .insert(module_name, module as usize);
    TRUE
}

/// Finds a registered process-wide module by name.
///
/// # Safety
///
/// `module_name` must be either null or point to a valid NUL-terminated C
/// string.
#[no_mangle]
pub unsafe extern "C" fn RimeFindModule(module_name: *const c_char) -> *mut RimeModule {
    if module_name.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: callers promise `module_name` is a valid NUL-terminated C string.
    let module_name = unsafe { CStr::from_ptr(module_name) }.to_string_lossy();
    let registered = module_registry()
        .lock()
        .expect("module registry should not be poisoned")
        .modules_by_name
        .get(module_name.as_ref())
        .copied();
    if let Some(module) = registered {
        return module as *mut RimeModule;
    }
    if module_name == "levers" {
        return levers_module();
    }
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn RimeSwitcherSettingsInit() -> *mut RimeSwitcherSettings {
    let settings = Box::into_raw(Box::new(RimeSwitcherSettings { placeholder: 0 }));
    switcher_selection_registry()
        .lock()
        .expect("switcher selection registry should not be poisoned")
        .insert(settings as usize, None);
    settings
}

/// Initializes levers custom settings for a deployed config id.
///
/// # Safety
///
/// `config_id` and `generator_id` must be valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomSettingsInit(
    config_id: *const c_char,
    generator_id: *const c_char,
) -> *mut RimeCustomSettings {
    let Some(config_id) = (unsafe { c_string_key(config_id) }) else {
        return ptr::null_mut();
    };
    let Some(generator_id) = (unsafe { c_string_key(generator_id) }) else {
        return ptr::null_mut();
    };

    Box::into_raw(Box::new(LeverCustomSettings {
        config_id,
        generator_id,
        config: ConfigState::default(),
        custom_config: ConfigState {
            root: Value::Null,
            cstring_cache: None,
        },
        modified: false,
    }))
    .cast::<RimeCustomSettings>()
}

/// Releases levers custom settings storage.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomSettingsDestroy(settings: *mut RimeCustomSettings) {
    if settings.is_null() {
        return;
    }
    // SAFETY: settings pointers are allocated by `RimeLeversCustomSettingsInit`.
    unsafe { drop(Box::from_raw(settings.cast::<LeverCustomSettings>())) };
}

/// Loads deployed and user custom config data for levers custom settings.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversLoadSettings(settings: *mut RimeCustomSettings) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };

    settings.config.root = load_runtime_config_root(&settings.config_id, ConfigOpenKind::Deployed);
    settings.config.cstring_cache = None;
    settings.modified = false;

    let path = custom_config_path(&settings.config_id);
    let loaded = fs::read_to_string(path)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok());
    match loaded {
        Some(root) => {
            settings.custom_config.root = root;
            settings.custom_config.cstring_cache = None;
            TRUE
        }
        None => {
            settings.custom_config.root = Value::Null;
            settings.custom_config.cstring_cache = None;
            FALSE
        }
    }
}

/// Saves modified levers custom settings to `<config>.custom.yaml`.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSaveSettings(settings: *mut RimeCustomSettings) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    if !settings.modified {
        return FALSE;
    }

    write_config_signature(
        &mut settings.custom_config.root,
        "customization",
        &settings.generator_id,
    );
    let path = custom_config_path(&settings.config_id);
    let Some(parent) = path.parent() else {
        return FALSE;
    };
    if fs::create_dir_all(parent).is_err() {
        return FALSE;
    }
    let Ok(yaml) = serde_yaml::to_string(&settings.custom_config.root) else {
        return FALSE;
    };
    if fs::write(path, yaml).is_err() {
        return FALSE;
    }

    settings.modified = false;
    TRUE
}

/// Writes a boolean levers custom setting under the literal `patch` key.
///
/// # Safety
///
/// `settings` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeBool(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: Bool,
) -> Bool {
    unsafe { levers_customize_value(settings, key, Value::Bool(value != FALSE)) }
}

/// Writes an integer levers custom setting under the literal `patch` key.
///
/// # Safety
///
/// `settings` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeInt(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: c_int,
) -> Bool {
    unsafe { levers_customize_value(settings, key, Value::Number(Number::from(value))) }
}

/// Writes a floating-point levers custom setting under the literal `patch` key.
///
/// # Safety
///
/// `settings` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeDouble(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: f64,
) -> Bool {
    let Ok(value) = serde_yaml::to_value(value) else {
        return FALSE;
    };
    unsafe { levers_customize_value(settings, key, value) }
}

/// Writes a string levers custom setting under the literal `patch` key.
///
/// # Safety
///
/// `settings`, `key`, and `value` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeString(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: *const c_char,
) -> Bool {
    let Some(value) = (unsafe { c_string_key(value) }) else {
        return FALSE;
    };
    unsafe { levers_customize_value(settings, key, Value::String(value)) }
}

/// Writes a list/map config item as a levers custom setting.
///
/// # Safety
///
/// `settings` and `key` must be valid pointers. `value` may be null or
/// uninitialized, in which case a null item is written.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeItem(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: *mut RimeConfig,
) -> Bool {
    let item = if value.is_null() {
        Value::Null
    } else {
        match unsafe { config_state_mut(value) } {
            Some(value_state) => value_state.root.clone(),
            None => Value::Null,
        }
    };
    unsafe { levers_customize_value(settings, key, item) }
}

/// Reports whether the custom settings file has not yet been customized.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversIsFirstRun(settings: *mut RimeCustomSettings) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    let root = fs::read_to_string(custom_config_path(&settings.config_id))
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok());
    bool_from(
        root.as_ref()
            .and_then(|root| find_config_value(root, "customization"))
            .and_then(Value::as_mapping)
            .is_none(),
    )
}

/// Reports whether custom settings have unsaved mutations.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSettingsIsModified(settings: *mut RimeCustomSettings) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    bool_from(settings.modified)
}

/// Copies the loaded deployed config into a caller-owned `RimeConfig`.
///
/// # Safety
///
/// `settings` and `config` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSettingsGetConfig(
    settings: *mut RimeCustomSettings,
    config: *mut RimeConfig,
) -> Bool {
    if config.is_null() {
        return FALSE;
    }
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    unsafe { install_config_root(config, settings.config.root.clone()) }
}

/// Returns the deployed schema list through the librime levers module API.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `list` must be null or point to writable schema-list storage.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetAvailableSchemaList(
    settings: *mut RimeSwitcherSettings,
    list: *mut RimeSchemaList,
) -> Bool {
    if settings.is_null() || list.is_null() {
        return FALSE;
    }

    clear_schema_list(list);
    populate_levers_schema_list(list, deployed_levers_schema_infos())
}

/// Returns the deployed switcher selection through the librime levers module API.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `list` must be null or point to writable schema-list storage.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSelectedSchemaList(
    settings: *mut RimeSwitcherSettings,
    list: *mut RimeSchemaList,
) -> Bool {
    if settings.is_null() || list.is_null() {
        return FALSE;
    }

    clear_schema_list(list);
    let selected_schema_ids = switcher_selection_registry()
        .lock()
        .expect("switcher selection registry should not be poisoned")
        .get(&(settings as usize))
        .cloned()
        .flatten()
        .unwrap_or_else(deployed_selected_schema_ids);
    populate_schema_id_list(list, selected_schema_ids)
}

/// Returns the schema id from a levers schema-info pointer.
///
/// # Safety
///
/// `info` must be either null or a pointer returned in a levers available
/// schema-list item's `reserved` field while that list is still alive.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaId(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe { levers_schema_info_ptr(info, |info| Some(info.schema_id.as_ptr())) }
}

/// Returns the schema name from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaName(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe { levers_schema_info_ptr(info, |info| Some(info.name.as_ptr())) }
}

/// Returns the schema version from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaVersion(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe {
        levers_schema_info_ptr(info, |info| {
            info.version.as_ref().map(|value| value.as_ptr())
        })
    }
}

/// Returns the schema author from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaAuthor(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe {
        levers_schema_info_ptr(info, |info| {
            info.author.as_ref().map(|value| value.as_ptr())
        })
    }
}

/// Returns the schema description from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaDescription(
    info: *mut RimeSchemaInfo,
) -> *const c_char {
    unsafe {
        levers_schema_info_ptr(info, |info| {
            info.description.as_ref().map(|value| value.as_ptr())
        })
    }
}

/// Returns the schema config file path from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaFilePath(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe {
        levers_schema_info_ptr(info, |info| {
            info.file_path.as_ref().map(|value| value.as_ptr())
        })
    }
}

/// Selects schema IDs on the opaque switcher settings object.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `schema_id_list` must point to `count` valid NUL-terminated
/// strings when `count` is positive.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSelectSchemas(
    settings: *mut RimeSwitcherSettings,
    schema_id_list: *const *const c_char,
    count: c_int,
) -> Bool {
    if settings.is_null() || count < 0 || (count > 0 && schema_id_list.is_null()) {
        return FALSE;
    }

    let mut selected_schema_ids = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        // SAFETY: callers promise `schema_id_list` has `count` readable entries
        // when count is positive.
        let schema_id = unsafe { *schema_id_list.add(index) };
        let Some(schema_id) = (unsafe { c_string_key(schema_id) }) else {
            return FALSE;
        };
        selected_schema_ids.push(schema_id);
    }

    switcher_selection_registry()
        .lock()
        .expect("switcher selection registry should not be poisoned")
        .insert(settings as usize, Some(selected_schema_ids));
    TRUE
}

/// Returns switcher hotkeys from the deployed default config.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetHotkeys(
    settings: *mut RimeSwitcherSettings,
) -> *const c_char {
    if settings.is_null() {
        return ptr::null();
    }

    let mut cache = switcher_hotkeys_cache()
        .lock()
        .expect("switcher hotkeys cache should not be poisoned");
    *cache = deployed_switcher_hotkeys().map(|hotkeys| cstring_from_lossless_str(&hotkeys));
    cache
        .as_ref()
        .map(|hotkeys| hotkeys.as_ptr())
        .unwrap_or(ptr::null())
}

/// Matches librime's currently unimplemented switcher hotkey mutation path.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `hotkeys`, when non-null, must point to a valid C string.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSetHotkeys(
    _settings: *mut RimeSwitcherSettings,
    _hotkeys: *const c_char,
) -> Bool {
    FALSE
}

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

    // SAFETY: `iterator` is non-null and owned by the caller; if it already
    // holds state from this shim, release it before replacing it.
    unsafe { clear_user_dict_iterator(iterator) };

    let names = deployed_user_dict_names()
        .into_iter()
        .map(|name| cstring_from_lossless_str(&name))
        .collect::<Vec<_>>();
    if names.is_empty() {
        return FALSE;
    }

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
    let Some(dict_name) = snapshot_dict_name(&snapshot) else {
        return FALSE;
    };
    let destination = user_dict_path(&dict_name);
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
    if dict_name.is_empty() || text_file.is_empty() {
        return -1;
    }

    let source = user_dict_path(&dict_name);
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
    if dict_name.is_empty() || text_file.is_empty() {
        return -1;
    }

    let source = PathBuf::from(text_file);
    if !source.is_file() {
        return -1;
    }
    let Ok(entry_count) = count_text_user_dict_entries(&source) else {
        return -1;
    };
    let destination = user_dict_path(&dict_name);
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

/// Frees schema-list storage returned by levers schema-list APIs.
///
/// # Safety
///
/// `list` follows the same ownership rules as `RimeFreeSchemaList`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSchemaListDestroy(list: *mut RimeSchemaList) {
    // SAFETY: ownership rules match `RimeFreeSchemaList`.
    unsafe { RimeFreeSchemaList(list) };
}

#[no_mangle]
pub extern "C" fn RimeSetupLogging(_app_name: *const c_char) {}

/// Initializes the runtime using the same trait handling as `RimeSetup`.
///
/// # Safety
///
/// `traits` follows the same preconditions as `RimeSetup`.
#[no_mangle]
pub unsafe extern "C" fn RimeInitialize(traits: *const RimeTraits) {
    // SAFETY: forwarded preconditions are identical to `RimeSetup`.
    unsafe { RimeSetup(traits) };
}

#[no_mangle]
pub extern "C" fn RimeFinalize() {
    RimeCleanupAllSessions();
    RimeSetNotificationHandler(None, ptr::null_mut());
}

#[no_mangle]
pub extern "C" fn RimeStartMaintenance(_full_check: Bool) -> Bool {
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeStartMaintenanceOnWorkspaceChange() -> Bool {
    RimeStartMaintenance(FALSE)
}

#[no_mangle]
pub extern "C" fn RimeIsMaintenancing() -> Bool {
    FALSE
}

#[no_mangle]
pub extern "C" fn RimeJoinMaintenanceThread() {}

/// Initializes deployer state using the same trait handling as `RimeSetup`.
///
/// # Safety
///
/// `traits` follows the same preconditions as `RimeSetup`.
#[no_mangle]
pub unsafe extern "C" fn RimeDeployerInitialize(traits: *const RimeTraits) {
    // SAFETY: forwarded preconditions are identical to `RimeSetup`.
    unsafe { RimeSetup(traits) };
}

#[no_mangle]
pub extern "C" fn RimePrebuildAllSchemas() -> Bool {
    bool_from(prebuild_all_schemas())
}

#[no_mangle]
pub extern "C" fn RimeDeployWorkspace() -> Bool {
    if !run_installation_update() {
        return FALSE;
    }
    if !workspace_update() {
        return FALSE;
    }
    if !user_dict_upgrade() {
        return FALSE;
    }
    if !cleanup_trash() {
        return FALSE;
    }
    notify(0, "deploy", "start");
    notify(0, "deploy", "success");
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeDeploySchema(schema_file: *const c_char) -> Bool {
    let Some(schema_file) = optional_c_string(schema_file) else {
        return FALSE;
    };
    bool_from(deploy_schema_file(&schema_file))
}

#[no_mangle]
pub extern "C" fn RimeDeployConfigFile(
    file_name: *const c_char,
    version_key: *const c_char,
) -> Bool {
    let Some(file_name) = optional_c_string(file_name) else {
        return FALSE;
    };
    let Some(version_key) = optional_c_string(version_key) else {
        return FALSE;
    };
    bool_from(deploy_config_file(&file_name, &version_key))
}

#[no_mangle]
pub extern "C" fn RimeSyncUserData() -> Bool {
    RimeCleanupAllSessions();
    let installation_synced = run_installation_update();
    let configs_synced = backup_config_files();
    let user_dicts_synced = sync_all_user_dicts();
    bool_from(installation_synced && configs_synced && user_dicts_synced)
}

#[no_mangle]
pub extern "C" fn RimeRunTask(task_name: *const c_char) -> Bool {
    let Some(task_name) = optional_c_string(task_name) else {
        return FALSE;
    };
    if task_name == "user_dict_sync" {
        return bool_from(sync_all_user_dicts());
    }
    if task_name == "backup_config_files" {
        return bool_from(backup_config_files());
    }
    if task_name == "installation_update" {
        return bool_from(run_installation_update());
    }
    if task_name == "cleanup_trash" {
        return bool_from(cleanup_trash());
    }
    if task_name == "workspace_update" {
        return bool_from(workspace_update());
    }
    if task_name == "user_dict_upgrade" {
        return bool_from(user_dict_upgrade());
    }
    if task_name == "prebuild_all_schemas" {
        return bool_from(prebuild_all_schemas());
    }
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeGetVersion() -> *const c_char {
    RIME_VERSION_BYTES.as_ptr().cast::<c_char>()
}

#[no_mangle]
pub extern "C" fn RimeGetSharedDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.shared_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetUserDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.user_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetPrebuiltDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.prebuilt_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetStagingDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.staging_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetSyncDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.sync_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetUserId() -> *const c_char {
    runtime_path_ptr(|paths| &paths.user_id)
}

/// Copies the shared data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSharedDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.shared_data_dir, dir, buffer_size);
}

/// Copies the user data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetUserDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.user_data_dir, dir, buffer_size);
}

/// Copies the prebuilt data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetPrebuiltDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.prebuilt_data_dir, dir, buffer_size);
}

/// Copies the staging directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStagingDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.staging_dir, dir, buffer_size);
}

/// Copies the sync directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSyncDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.sync_dir, dir, buffer_size);
}

/// Copies the user-specific sync directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetUserDataSyncDir(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.user_data_sync_dir, dir, buffer_size);
}

#[no_mangle]
pub extern "C" fn RimeCreateSession() -> RimeSessionId {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .create_session()
}

#[no_mangle]
pub extern "C" fn RimeFindSession(session_id: RimeSessionId) -> Bool {
    bool_from(
        session_id != 0
            && sessions()
                .lock()
                .expect("session registry should not be poisoned")
                .sessions
                .contains_key(&session_id),
    )
}

#[no_mangle]
pub extern "C" fn RimeDestroySession(session_id: RimeSessionId) -> Bool {
    bool_from(
        session_id != 0
            && sessions()
                .lock()
                .expect("session registry should not be poisoned")
                .sessions
                .remove(&session_id)
                .is_some(),
    )
}

#[no_mangle]
pub extern "C" fn RimeCleanupAllSessions() {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .sessions
        .clear();
}

#[no_mangle]
pub extern "C" fn RimeCleanupStaleSessions() {}

#[no_mangle]
pub extern "C" fn RimeProcessKey(session_id: RimeSessionId, keycode: c_int, mask: c_int) -> Bool {
    if session_id == 0 || mask != 0 {
        return FALSE;
    }
    let Some(key_event) = key_event_from_rime_keycode(keycode) else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };

    let was_composing = !session.engine.context().composition.input.is_empty();
    if let Some(commit) = session.engine.process_key_event(key_event) {
        session.unread_commit = Some(commit);
        return TRUE;
    }

    bool_from(matches!(key_event.code, KeyCode::Character(ch) if ch != ' ') || was_composing)
}

#[no_mangle]
pub extern "C" fn RimeCommitComposition(session_id: RimeSessionId) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    let Some(commit) = session.engine.commit_composition() else {
        return FALSE;
    };

    session.unread_commit = Some(commit);
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeClearComposition(session_id: RimeSessionId) {
    if session_id == 0 {
        return;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    if let Some(session) = registry.sessions.get_mut(&session_id) {
        session.engine.clear_composition();
    }
}

#[no_mangle]
pub extern "C" fn RimeGetInput(session_id: RimeSessionId) -> *const c_char {
    if session_id == 0 {
        return ptr::null();
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return ptr::null();
    };
    let Ok(input) = CString::new(session.engine.context().composition.input.as_str()) else {
        return ptr::null();
    };
    session.input_buffer = Some(input);
    session
        .input_buffer
        .as_ref()
        .map_or(ptr::null(), |input| input.as_ptr())
}

/// Sets the current raw composition input for a session.
///
/// # Safety
///
/// `input` must be either null or a valid NUL-terminated C string. Null input
/// is rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeSetInput(session_id: RimeSessionId, input: *const c_char) -> Bool {
    if session_id == 0 || input.is_null() {
        return FALSE;
    }

    // SAFETY: `input` is non-null and caller promises a valid NUL-terminated C
    // string.
    let Ok(input) = unsafe { CStr::from_ptr(input) }.to_str() else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    session.engine.set_input(input);
    session.input_buffer = None;
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeGetCaretPos(session_id: RimeSessionId) -> usize {
    if session_id == 0 {
        return 0;
    }

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    registry
        .sessions
        .get(&session_id)
        .map_or(0, |session| session.engine.context().composition.caret)
}

#[no_mangle]
pub extern "C" fn RimeSetCaretPos(session_id: RimeSessionId, caret_pos: usize) {
    if session_id == 0 {
        return;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    if let Some(session) = registry.sessions.get_mut(&session_id) {
        session.engine.set_caret_pos(caret_pos);
    }
}

/// Sets a session-scoped runtime option.
///
/// # Safety
///
/// `option` must be either null or point to a valid, nul-terminated C string.
/// Null option names are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeSetOption(
    session_id: RimeSessionId,
    option: *const c_char,
    value: Bool,
) {
    if option.is_null() {
        return;
    }
    // SAFETY: callers promise that `option` is a valid nul-terminated string.
    let option = unsafe { CStr::from_ptr(option) }
        .to_string_lossy()
        .into_owned();
    if with_session(session_id, |session| {
        session.engine.set_option(option.clone(), value != FALSE);
        true
    }) == TRUE
    {
        let message_value = if value != FALSE {
            option
        } else {
            format!("!{option}")
        };
        notify(session_id, "option", &message_value);
    }
}

/// Returns the current value of a session-scoped runtime option.
///
/// # Safety
///
/// `option` must be either null or point to a valid, nul-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeGetOption(session_id: RimeSessionId, option: *const c_char) -> Bool {
    if option.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `option` is a valid nul-terminated string.
    let option = unsafe { CStr::from_ptr(option) };
    with_session(session_id, |session| {
        session.engine.get_option(&option.to_string_lossy())
    })
}

/// Sets a session-scoped string property.
///
/// # Safety
///
/// `property` and `value` must be either null or point to valid,
/// nul-terminated C strings. Null inputs are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeSetProperty(
    session_id: RimeSessionId,
    property: *const c_char,
    value: *const c_char,
) {
    if property.is_null() || value.is_null() {
        return;
    }
    // SAFETY: callers promise that both pointers are valid nul-terminated
    // strings.
    let property = unsafe { CStr::from_ptr(property) }
        .to_string_lossy()
        .into_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    if with_session(session_id, |session| {
        session.engine.set_property(property.clone(), value.clone());
        true
    }) == TRUE
    {
        notify(session_id, "property", &format!("{property}={value}"));
    }
}

/// Copies a session-scoped string property into caller-provided storage.
///
/// # Safety
///
/// `property` must point to a valid nul-terminated C string, and `value` must
/// point to writable storage of `buffer_size` bytes. Null or empty values are
/// rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeGetProperty(
    session_id: RimeSessionId,
    property: *const c_char,
    value: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if property.is_null() || value.is_null() || buffer_size == 0 {
        return FALSE;
    }
    // SAFETY: callers promise that `property` is a valid nul-terminated string.
    let property = unsafe { CStr::from_ptr(property) };

    with_session(session_id, |session| {
        let Some(property_value) = session.engine.get_property(&property.to_string_lossy()) else {
            return false;
        };
        if property_value.is_empty() {
            return false;
        }
        copy_c_string_to_buffer(property_value, value, buffer_size);
        true
    })
}

/// Copies the current session schema id into caller-provided storage.
///
/// # Safety
///
/// `schema_id` must point to writable storage of `buffer_size` bytes. Null or
/// empty buffers are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCurrentSchema(
    session_id: RimeSessionId,
    schema_id: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if schema_id.is_null() || buffer_size == 0 {
        return FALSE;
    }

    with_session(session_id, |session| {
        let current_schema = session.engine.status().schema_id;
        copy_c_string_to_buffer(&current_schema, schema_id, buffer_size);
        true
    })
}

/// Selects the active schema id for a session.
///
/// # Safety
///
/// `schema_id` must be either null or point to a valid nul-terminated C string.
/// Null schema ids are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeSelectSchema(
    session_id: RimeSessionId,
    schema_id: *const c_char,
) -> Bool {
    if schema_id.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `schema_id` is a valid nul-terminated
    // string.
    let schema_id = unsafe { CStr::from_ptr(schema_id) }
        .to_string_lossy()
        .into_owned();

    let selected = with_session(session_id, |session| {
        session.engine.set_schema(schema_id.clone(), schema_id);
        session.engine.clear_composition();
        session.input_buffer = None;
        session.unread_commit = None;
        true
    });
    if selected == TRUE {
        let status = sessions()
            .lock()
            .expect("session registry should not be poisoned")
            .sessions
            .get(&session_id)
            .map(|session| session.engine.status());
        if let Some(status) = status {
            notify(
                session_id,
                "schema",
                &format!("{}/{}", status.schema_id, status.schema_name),
            );
        }
    }
    selected
}

/// Returns the currently available schema list.
///
/// # Safety
///
/// `schema_list` must be either null or point to writable storage. When this
/// function returns `TRUE`, the caller must release nested allocations with
/// `RimeFreeSchemaList`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSchemaList(schema_list: *mut RimeSchemaList) -> Bool {
    if schema_list.is_null() {
        return FALSE;
    }

    clear_schema_list(schema_list);
    populate_schema_list(schema_list, deployed_schema_list_entries())
}

fn populate_schema_list(schema_list: *mut RimeSchemaList, entries: Vec<(String, String)>) -> Bool {
    if entries.is_empty() {
        return FALSE;
    }

    let mut list = Vec::with_capacity(entries.len());
    for (schema_id, name) in entries {
        let Ok(schema_id) = CString::new(schema_id) else {
            free_schema_list_items(&mut list);
            return FALSE;
        };
        let Ok(name) = CString::new(name) else {
            free_schema_list_items(&mut list);
            return FALSE;
        };
        list.push(RimeSchemaListItem {
            schema_id: schema_id.into_raw(),
            name: name.into_raw(),
            reserved: ptr::null_mut(),
        });
    }
    let size = list.len();
    let list_ptr = list.as_mut_ptr();
    std::mem::forget(list);

    // SAFETY: `schema_list` is non-null and points to caller-owned writable
    // storage; `list_ptr` owns `size` initialized schema-list items.
    unsafe {
        (*schema_list).size = size;
        (*schema_list).list = list_ptr;
    }
    TRUE
}

fn populate_levers_schema_list(
    schema_list: *mut RimeSchemaList,
    entries: Vec<LeverSchemaInfo>,
) -> Bool {
    if entries.is_empty() {
        return FALSE;
    }

    let mut list = Vec::with_capacity(entries.len());
    for entry in entries {
        let schema_id = entry.schema_id.as_c_str().to_owned().into_raw();
        let name = entry.name.as_c_str().to_owned().into_raw();
        let info = Box::into_raw(Box::new(entry)).cast::<c_void>();
        list.push(RimeSchemaListItem {
            schema_id,
            name,
            reserved: info,
        });
    }
    let size = list.len();
    let list_ptr = list.as_mut_ptr();
    std::mem::forget(list);

    // SAFETY: `schema_list` is non-null and points to caller-owned writable
    // storage; `list_ptr` owns `size` initialized schema-list items.
    unsafe {
        (*schema_list).size = size;
        (*schema_list).list = list_ptr;
    }
    TRUE
}

fn populate_schema_id_list(schema_list: *mut RimeSchemaList, schema_ids: Vec<String>) -> Bool {
    if schema_ids.is_empty() {
        return FALSE;
    }

    let mut list = Vec::with_capacity(schema_ids.len());
    for schema_id in schema_ids {
        let Ok(schema_id) = CString::new(schema_id) else {
            free_schema_list_items(&mut list);
            return FALSE;
        };
        list.push(RimeSchemaListItem {
            schema_id: schema_id.into_raw(),
            name: ptr::null_mut(),
            reserved: ptr::null_mut(),
        });
    }
    let size = list.len();
    let list_ptr = list.as_mut_ptr();
    std::mem::forget(list);

    // SAFETY: `schema_list` is non-null and points to caller-owned writable
    // storage; `list_ptr` owns `size` initialized schema-list items.
    unsafe {
        (*schema_list).size = size;
        (*schema_list).list = list_ptr;
    }
    TRUE
}

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
    let config_id = format!("{schema_id}.schema");
    open_runtime_config(&config_id, ConfigOpenKind::Deployed, config)
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
    // SAFETY: `yaml` is non-null and caller promises a valid C string.
    let Ok(yaml) = unsafe { CStr::from_ptr(yaml) }.to_str() else {
        return FALSE;
    };
    // SAFETY: `config` is non-null and writable.
    if unsafe { (*config).ptr.is_null() && RimeConfigInit(config) == FALSE } {
        return FALSE;
    }
    let Ok(root) = serde_yaml::from_str::<Value>(yaml) else {
        return FALSE;
    };
    // SAFETY: `config` now owns a valid config state.
    let Some(state) = (unsafe { config_state_mut(config) }) else {
        return FALSE;
    };
    state.root = root;
    state.cstring_cache = None;
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
    let Value::Bool(found) = found else {
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
    let Some(found) = found
        .as_i64()
        .and_then(|number| c_int::try_from(number).ok())
    else {
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
    let Some(found) = found.as_f64() else {
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
    if value.is_null() || buffer_size == 0 {
        return FALSE;
    }
    let Some(found) = (unsafe { config_string_value(config, key) }) else {
        return FALSE;
    };
    copy_c_string_to_buffer(&found, value, buffer_size);
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
    state.cstring_cache = Some(value);
    state
        .cstring_cache
        .as_ref()
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

    let modified_time = SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |_| "0".to_owned(),
        |duration| duration.as_secs().to_string(),
    );
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
    state.cstring_cache = None;
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
    unsafe { config_set(config, key, Value::Bool(value != FALSE)) }
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
    unsafe { config_set(config, key, Value::Number(Number::from(value))) }
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
    let Ok(value) = serde_yaml::to_value(value) else {
        return FALSE;
    };
    unsafe { config_set(config, key, value) }
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
    destination.cstring_cache = None;
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
    state.cstring_cache = None;
    bool_from(remove_config_value(&mut state.root, &key))
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
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };
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
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };
    let Some(found) = (unsafe { config_lookup_key(config, &key) }) else {
        return FALSE;
    };
    let Value::Mapping(mapping) = found else {
        return FALSE;
    };

    let entries = mapping
        .iter()
        .filter_map(|(entry_key, _)| match entry_key {
            Value::String(entry_key) => {
                let path = config_child_path(&key, entry_key);
                Some((entry_key.clone(), path))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
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

/// Returns a switch state label from the selected schema config.
///
/// # Safety
///
/// `option_name` must be either null or a valid NUL-terminated C string. The
/// returned pointer is process-owned and remains valid until the next
/// state-label query.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStateLabel(
    session_id: RimeSessionId,
    option_name: *const c_char,
    state: Bool,
) -> *const c_char {
    // SAFETY: forwarded preconditions match `RimeGetStateLabelAbbreviated`.
    unsafe { RimeGetStateLabelAbbreviated(session_id, option_name, state, FALSE).str }
}

/// Returns a switch state label slice from the selected schema config.
///
/// # Safety
///
/// `option_name` must be either null or a valid NUL-terminated C string. The
/// returned pointer is process-owned and remains valid until the next
/// state-label query.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStateLabelAbbreviated(
    session_id: RimeSessionId,
    option_name: *const c_char,
    state: Bool,
    abbreviated: Bool,
) -> RimeStringSlice {
    let Some(option_name) = (unsafe { c_string_key(option_name) }) else {
        return empty_string_slice();
    };
    let Some(label) = state_label_for_session(
        session_id,
        &option_name,
        state != FALSE,
        abbreviated != FALSE,
    ) else {
        return empty_string_slice();
    };
    let Ok(cached_label) = CString::new(label.value) else {
        return empty_string_slice();
    };

    let mut cache = state_label_cache()
        .lock()
        .expect("state label cache should not be poisoned");
    *cache = Some(cached_label);
    let Some(cached_label) = cache.as_ref() else {
        return empty_string_slice();
    };
    RimeStringSlice {
        str: cached_label.as_ptr(),
        length: label.length,
    }
}

/// Processes a librime-style key sequence against a session.
///
/// # Safety
///
/// `key_sequence` must be either null or point to a valid, nul-terminated C
/// string. Null or unparsable sequences are rejected without mutating the
/// session.
#[no_mangle]
pub unsafe extern "C" fn RimeSimulateKeySequence(
    session_id: RimeSessionId,
    key_sequence: *const c_char,
) -> Bool {
    if session_id == 0 || key_sequence.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `key_sequence` is a valid nul-terminated
    // string.
    let Ok(key_sequence) = unsafe { CStr::from_ptr(key_sequence) }.to_str() else {
        return FALSE;
    };
    let Ok(key_events) = parse_key_sequence(key_sequence) else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };

    for key_event in key_events {
        if let Some(commit) = session.engine.process_key_event(key_event) {
            session.unread_commit = Some(commit);
        }
    }
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeSelectCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    commit_selected_candidate(session_id, |session| session.engine.select_candidate(index))
}

#[no_mangle]
pub extern "C" fn RimeSelectCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    commit_selected_candidate(session_id, |session| {
        session.engine.select_candidate_on_current_page(index)
    })
}

#[no_mangle]
pub extern "C" fn RimeDeleteCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    with_session(session_id, |session| session.engine.delete_candidate(index))
}

#[no_mangle]
pub extern "C" fn RimeDeleteCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    with_session(session_id, |session| {
        session.engine.delete_candidate_on_current_page(index)
    })
}

#[no_mangle]
pub extern "C" fn RimeHighlightCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    with_session(session_id, |session| {
        session.engine.highlight_candidate(index)
    })
}

#[no_mangle]
pub extern "C" fn RimeHighlightCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    with_session(session_id, |session| {
        session.engine.highlight_candidate_on_current_page(index)
    })
}

#[no_mangle]
pub extern "C" fn RimeChangePage(session_id: RimeSessionId, backward: Bool) -> Bool {
    with_session(session_id, |session| {
        session.engine.change_page(backward != FALSE)
    })
}

/// Copies the unread commit text for a session into a caller-provided commit.
///
/// # Safety
///
/// `commit` must be either null or a valid, writable pointer to a `RimeCommit`.
/// When this function returns `TRUE`, the caller must release `commit.text` by
/// passing the same commit object to `RimeFreeCommit`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCommit(session_id: RimeSessionId, commit: *mut RimeCommit) -> Bool {
    if commit.is_null() {
        return FALSE;
    }

    clear_commit(commit);

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    let Some(text) = session.unread_commit.take() else {
        return FALSE;
    };
    let Ok(text) = CString::new(text) else {
        return FALSE;
    };

    // SAFETY: `commit` is non-null and points to caller-owned writable storage.
    unsafe {
        (*commit).data_size = std::mem::size_of::<RimeCommit>() as c_int;
        (*commit).text = text.into_raw();
    }
    TRUE
}

/// Copies the current composition and first candidate page into caller storage.
///
/// # Safety
///
/// `context` must be either null or a valid, writable pointer to a
/// `RimeContext` initialized with a positive `data_size`. When this function
/// returns `TRUE`, the caller must release nested strings and candidate memory
/// by passing the same context object to `RimeFreeContext`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetContext(
    session_id: RimeSessionId,
    context: *mut RimeContext,
) -> Bool {
    if context.is_null() {
        return FALSE;
    }
    // SAFETY: `context` is non-null and points to caller-owned storage.
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    clear_context(context);

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get(&session_id) else {
        return FALSE;
    };

    let snapshot = session.engine.snapshot();
    let composition = snapshot.context.composition;
    if !composition.input.is_empty() {
        let Ok(preedit) = CString::new(composition.preedit) else {
            return FALSE;
        };
        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `preedit` is converted into owned C storage for the caller.
        unsafe {
            (*context).composition.length = composition.input.len() as c_int;
            (*context).composition.cursor_pos = composition.caret as c_int;
            (*context).composition.sel_start = 0;
            (*context).composition.sel_end = composition.input.len() as c_int;
            (*context).composition.preedit = preedit.into_raw();
        }
    }

    let candidates = snapshot.context.candidates;
    if !candidates.is_empty() {
        let highlighted = snapshot.context.highlighted;
        let page_no = highlighted / DEFAULT_PAGE_SIZE;
        let page_start = page_no * DEFAULT_PAGE_SIZE;
        let page_end = (page_start + DEFAULT_PAGE_SIZE).min(candidates.len());
        let page_candidates = &candidates[page_start..page_end];

        let mut rime_candidates = Vec::with_capacity(page_candidates.len());
        for candidate in page_candidates {
            let Ok(text) = CString::new(candidate.text.as_str()) else {
                free_rime_candidates(&mut rime_candidates);
                return FALSE;
            };
            let comment = if candidate.comment.is_empty() {
                ptr::null_mut()
            } else {
                let Ok(comment) = CString::new(candidate.comment.as_str()) else {
                    free_rime_candidates(&mut rime_candidates);
                    return FALSE;
                };
                comment.into_raw()
            };
            rime_candidates.push(RimeCandidate {
                text: text.into_raw(),
                comment,
                reserved: ptr::null_mut(),
            });
        }

        let num_candidates = rime_candidates.len();
        let candidates_ptr = rime_candidates.as_mut_ptr();
        std::mem::forget(rime_candidates);

        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `candidates_ptr` owns `num_candidates` initialized entries.
        unsafe {
            (*context).menu.page_size = DEFAULT_PAGE_SIZE as c_int;
            (*context).menu.page_no = page_no as c_int;
            (*context).menu.is_last_page = bool_from(page_end == candidates.len());
            (*context).menu.highlighted_candidate_index =
                (highlighted - page_start).min(num_candidates.saturating_sub(1)) as c_int;
            (*context).menu.num_candidates = num_candidates as c_int;
            (*context).menu.candidates = candidates_ptr;
        }
    }

    TRUE
}

/// Copies current session status into caller storage.
///
/// # Safety
///
/// `status` must be either null or a valid, writable pointer to a
/// `RimeStatus` initialized with a positive `data_size`. When this function
/// returns `TRUE`, the caller must release nested strings by passing the same
/// status object to `RimeFreeStatus`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStatus(session_id: RimeSessionId, status: *mut RimeStatus) -> Bool {
    if status.is_null() {
        return FALSE;
    }
    // SAFETY: `status` is non-null and points to caller-owned storage.
    if unsafe { (*status).data_size } <= 0 {
        return FALSE;
    }

    clear_status(status);

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get(&session_id) else {
        return FALSE;
    };
    let snapshot = session.engine.status();
    let Ok(schema_id) = CString::new(snapshot.schema_id) else {
        return FALSE;
    };
    let Ok(schema_name) = CString::new(snapshot.schema_name) else {
        return FALSE;
    };

    // SAFETY: `status` is non-null and points to caller-owned writable storage;
    // schema strings are converted into owned C storage for the caller.
    unsafe {
        (*status).schema_id = schema_id.into_raw();
        (*status).schema_name = schema_name.into_raw();
        (*status).is_disabled = bool_from(snapshot.is_disabled);
        (*status).is_composing = bool_from(snapshot.is_composing);
        (*status).is_ascii_mode = bool_from(snapshot.is_ascii_mode);
        (*status).is_full_shape = bool_from(snapshot.is_full_shape);
        (*status).is_simplified = bool_from(snapshot.is_simplified);
        (*status).is_traditional = bool_from(snapshot.is_traditional);
        (*status).is_ascii_punct = bool_from(snapshot.is_ascii_punct);
    }
    TRUE
}

/// Initializes an iterator over the current candidate list from the first item.
///
/// # Safety
///
/// `iterator` must be either null or a valid, writable pointer to a
/// `RimeCandidateListIterator`. When this function returns `TRUE`, the caller
/// must eventually pass the same iterator to `RimeCandidateListEnd`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListBegin(
    session_id: RimeSessionId,
    iterator: *mut RimeCandidateListIterator,
) -> Bool {
    // SAFETY: forwarded preconditions are identical to
    // `RimeCandidateListFromIndex` with a zero start index.
    unsafe { RimeCandidateListFromIndex(session_id, iterator, 0) }
}

/// Initializes an iterator over the current candidate list from `index`.
///
/// # Safety
///
/// `iterator` must be either null or a valid, writable pointer to a
/// `RimeCandidateListIterator`. When this function returns `TRUE`, the caller
/// must eventually pass the same iterator to `RimeCandidateListEnd`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListFromIndex(
    session_id: RimeSessionId,
    iterator: *mut RimeCandidateListIterator,
    index: c_int,
) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get(&session_id) else {
        return FALSE;
    };
    let candidates = session.engine.context().candidates.clone();
    if candidates.is_empty() {
        return FALSE;
    }

    let state = Box::new(CandidateListState { candidates });
    // SAFETY: `iterator` is non-null and points to caller-owned writable
    // storage. The boxed state is released by `RimeCandidateListEnd`.
    unsafe {
        (*iterator).ptr = Box::into_raw(state).cast::<c_void>();
        (*iterator).index = index.saturating_sub(1);
        (*iterator).candidate = RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        };
    }
    TRUE
}

/// Advances a candidate list iterator and copies the current candidate.
///
/// # Safety
///
/// `iterator` must be either null or a valid pointer previously initialized by
/// `RimeCandidateListBegin` or `RimeCandidateListFromIndex`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListNext(iterator: *mut RimeCandidateListIterator) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }
    // SAFETY: `iterator` is non-null and points to caller-owned storage.
    let state = unsafe { (*iterator).ptr.cast::<CandidateListState>().as_ref() };
    let Some(state) = state else {
        return FALSE;
    };

    // SAFETY: `iterator` is non-null and any current candidate strings were
    // allocated by this API during an earlier successful `Next`.
    unsafe {
        free_candidate_fields(&mut (*iterator).candidate);
        (*iterator).index = (*iterator).index.saturating_add(1);
        if (*iterator).index < 0 {
            return FALSE;
        }
    }

    // SAFETY: index was checked non-negative above.
    let candidate_index = unsafe { (*iterator).index as usize };
    let Some(candidate) = state.candidates.get(candidate_index) else {
        return FALSE;
    };
    let Ok(text) = CString::new(candidate.text.as_str()) else {
        return FALSE;
    };
    let comment = if candidate.comment.is_empty() {
        ptr::null_mut()
    } else {
        let Ok(comment) = CString::new(candidate.comment.as_str()) else {
            return FALSE;
        };
        comment.into_raw()
    };

    // SAFETY: `iterator` is non-null and points to caller-owned writable
    // storage; strings are now owned by the iterator until next/end.
    unsafe {
        (*iterator).candidate = RimeCandidate {
            text: text.into_raw(),
            comment,
            reserved: ptr::null_mut(),
        };
    }
    TRUE
}

/// Frees a candidate list iterator initialized by this API.
///
/// # Safety
///
/// `iterator` must be either null or a valid pointer. Any non-null nested
/// pointers must have been returned by candidate-list iterator APIs.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListEnd(iterator: *mut RimeCandidateListIterator) {
    if iterator.is_null() {
        return;
    }

    // SAFETY: `iterator` is non-null and nested pointers are owned by this API
    // when populated by candidate-list iterator calls.
    unsafe {
        if !(*iterator).ptr.is_null() {
            drop(Box::from_raw((*iterator).ptr.cast::<CandidateListState>()));
        }
        free_candidate_fields(&mut (*iterator).candidate);
        (*iterator).ptr = ptr::null_mut();
        (*iterator).index = 0;
        (*iterator).candidate = RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        };
    }
}

/// Frees nested allocations populated by `RimeGetContext`.
///
/// # Safety
///
/// `context` must be either null or a valid, writable pointer to a
/// `RimeContext`. Nested pointers, when non-null, must have been returned by
/// `RimeGetContext` and not already freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeContext(context: *mut RimeContext) -> Bool {
    if context.is_null() {
        return FALSE;
    }
    // SAFETY: `context` is non-null and points to caller-owned storage.
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    free_context_fields(context);
    clear_context(context);
    TRUE
}

/// Frees nested allocations populated by `RimeGetStatus`.
///
/// # Safety
///
/// `status` must be either null or a valid, writable pointer to a
/// `RimeStatus`. Nested pointers, when non-null, must have been returned by
/// `RimeGetStatus` and not already freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeStatus(status: *mut RimeStatus) -> Bool {
    if status.is_null() {
        return FALSE;
    }
    // SAFETY: `status` is non-null and points to caller-owned storage.
    if unsafe { (*status).data_size } <= 0 {
        return FALSE;
    }

    free_status_fields(status);
    clear_status(status);
    TRUE
}

/// Frees a commit object populated by `RimeGetCommit`.
///
/// # Safety
///
/// `commit` must be either null or a valid, writable pointer to a `RimeCommit`.
/// If `commit.text` is non-null, it must be a pointer previously returned by
/// `RimeGetCommit` and not already freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeCommit(commit: *mut RimeCommit) -> Bool {
    if commit.is_null() {
        return FALSE;
    }

    // SAFETY: `commit` is non-null and any non-null `text` pointer is owned by
    // this API because it was returned from `CString::into_raw` in `RimeGetCommit`.
    unsafe {
        if !(*commit).text.is_null() {
            drop(CString::from_raw((*commit).text));
        }
    }
    clear_commit(commit);
    TRUE
}

fn key_event_from_rime_keycode(keycode: c_int) -> Option<KeyEvent> {
    let code = match keycode {
        XK_BACKSPACE => KeyCode::Backspace,
        XK_RETURN => KeyCode::Return,
        0x20..=0x7e => KeyCode::Character(char::from_u32(keycode as u32)?),
        _ => return None,
    };

    Some(KeyEvent {
        code,
        modifiers: KeyModifiers::default(),
    })
}

fn commit_selected_candidate(
    session_id: RimeSessionId,
    select: impl FnOnce(&mut SessionState) -> Option<String>,
) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    let Some(commit) = select(session) else {
        return FALSE;
    };

    session.unread_commit = Some(commit);
    TRUE
}

fn with_session(session_id: RimeSessionId, action: impl FnOnce(&mut SessionState) -> bool) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };

    bool_from(action(session))
}

fn optional_c_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }

    // SAFETY: callers validate that non-null optional runtime trait strings are
    // valid NUL-terminated C strings before reaching this helper.
    Some(
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn cstring_from_lossless_str(value: &str) -> CString {
    CString::new(value).expect("values derived from C strings or literals cannot contain NUL bytes")
}

fn path_join(base: &str, child: &str) -> String {
    Path::new(base).join(child).to_string_lossy().into_owned()
}

fn open_runtime_config(config_id: &str, kind: ConfigOpenKind, config: *mut RimeConfig) -> Bool {
    if config.is_null() {
        return FALSE;
    }

    let root = load_runtime_config_root(config_id, kind);
    unsafe { install_config_root(config, root) }
}

unsafe fn install_config_root(config: *mut RimeConfig, root: Value) -> Bool {
    if config.is_null() {
        return FALSE;
    }
    let state = Box::new(ConfigState {
        root,
        cstring_cache: None,
    });

    // SAFETY: `config` is non-null and points to caller-owned writable storage.
    // If it already owns config state from this shim, replace it to avoid
    // leaking when callers reuse a `RimeConfig` slot.
    unsafe {
        if !(*config).ptr.is_null() {
            drop(Box::from_raw((*config).ptr.cast::<ConfigState>()));
        }
        (*config).ptr = Box::into_raw(state).cast::<c_void>();
    }
    TRUE
}

fn load_runtime_config_root(config_id: &str, kind: ConfigOpenKind) -> Value {
    let resource_id = normalize_config_resource_id(config_id);
    let selected_path = selected_runtime_config_path(&resource_id, kind);

    selected_path
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
        .unwrap_or(Value::Null)
}

fn runtime_config_roots(kind: ConfigOpenKind) -> Vec<String> {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    match kind {
        ConfigOpenKind::Deployed => vec![
            paths.staging_dir.to_string_lossy().into_owned(),
            paths.prebuilt_data_dir.to_string_lossy().into_owned(),
        ],
        ConfigOpenKind::User => vec![paths.user_data_dir.to_string_lossy().into_owned()],
    }
}

fn selected_runtime_config_path(resource_id: &str, kind: ConfigOpenKind) -> Option<PathBuf> {
    let roots = runtime_config_roots(kind);
    roots
        .iter()
        .map(|root| config_file_path(root, resource_id))
        .find(|path| path.exists())
        .or_else(|| {
            roots
                .first()
                .map(|root| config_file_path(root, resource_id))
        })
}

fn existing_runtime_config_path(resource_id: &str, kind: ConfigOpenKind) -> Option<PathBuf> {
    runtime_config_roots(kind)
        .iter()
        .map(|root| config_file_path(root, resource_id))
        .find(|path| path.exists())
}

fn normalize_config_resource_id(config_id: &str) -> String {
    config_id
        .strip_suffix(".yaml")
        .unwrap_or(config_id)
        .to_owned()
}

fn config_file_path(root: &str, resource_id: &str) -> PathBuf {
    Path::new(root).join(format!("{resource_id}.yaml"))
}

fn deployed_schema_list_entries() -> Vec<(String, String)> {
    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Some(schema_list) = find_config_value(&default_config, "schema_list") else {
        return Vec::new();
    };
    let Value::Sequence(schema_list) = schema_list else {
        return Vec::new();
    };

    schema_list
        .iter()
        .filter_map(|entry| deployed_schema_list_entry(&default_config, entry))
        .map(|schema_id| {
            let schema_config =
                load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
            let name = find_config_value(&schema_config, "schema/name")
                .and_then(Value::as_str)
                .unwrap_or(&schema_id)
                .to_owned();
            (schema_id, name)
        })
        .collect()
}

fn deployed_levers_schema_infos() -> Vec<LeverSchemaInfo> {
    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Some(schema_list) = find_config_value(&default_config, "schema_list") else {
        return Vec::new();
    };
    let Value::Sequence(schema_list) = schema_list else {
        return Vec::new();
    };

    schema_list
        .iter()
        .filter_map(|entry| deployed_schema_list_entry(&default_config, entry))
        .map(|schema_id| {
            let resource_id = normalize_config_resource_id(&format!("{schema_id}.schema"));
            let file_path = existing_runtime_config_path(&resource_id, ConfigOpenKind::Deployed);
            let schema_config = load_runtime_config_root(&resource_id, ConfigOpenKind::Deployed);
            levers_schema_info(schema_id, schema_config, file_path)
        })
        .collect()
}

fn levers_schema_info(
    schema_id: String,
    schema_config: Value,
    file_path: Option<PathBuf>,
) -> LeverSchemaInfo {
    let name = find_config_value(&schema_config, "schema/name")
        .and_then(Value::as_str)
        .unwrap_or(&schema_id)
        .to_owned();
    let version = find_config_value(&schema_config, "schema/version")
        .and_then(Value::as_str)
        .map(cstring_from_lossless_str);
    let author =
        levers_schema_author(&schema_config).map(|author| cstring_from_lossless_str(&author));
    let description = find_config_value(&schema_config, "schema/description")
        .and_then(Value::as_str)
        .map(cstring_from_lossless_str);
    let file_path =
        file_path.map(|path| cstring_from_lossless_str(path.to_string_lossy().as_ref()));

    LeverSchemaInfo {
        schema_id: cstring_from_lossless_str(&schema_id),
        name: cstring_from_lossless_str(&name),
        version,
        author,
        description,
        file_path,
    }
}

fn levers_schema_author(schema_config: &Value) -> Option<String> {
    let author = find_config_value(schema_config, "schema/author")?;
    match author {
        Value::Sequence(authors) => {
            let joined = authors
                .iter()
                .filter_map(Value::as_str)
                .filter(|author| !author.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        Value::String(author) if !author.is_empty() => Some(author.clone()),
        _ => None,
    }
}

fn deployed_selected_schema_ids() -> Vec<String> {
    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Some(schema_list) = find_config_value(&default_config, "schema_list") else {
        return Vec::new();
    };
    let Value::Sequence(schema_list) = schema_list else {
        return Vec::new();
    };

    schema_list
        .iter()
        .filter_map(|entry| {
            let Value::Mapping(entry) = entry else {
                return None;
            };
            entry
                .get(Value::String("schema".to_owned()))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn deployed_switcher_hotkeys() -> Option<String> {
    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Value::Sequence(hotkeys) = find_config_value(&default_config, "switcher/hotkeys")? else {
        return None;
    };

    let hotkeys = hotkeys
        .iter()
        .filter_map(Value::as_str)
        .filter(|hotkey| !hotkey.is_empty())
        .collect::<Vec<_>>();
    if hotkeys.is_empty() {
        None
    } else {
        Some(hotkeys.join(", "))
    }
}

fn deployed_user_dict_names() -> Vec<String> {
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

fn runtime_user_data_sync_dir() -> PathBuf {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    PathBuf::from(paths.user_data_sync_dir.to_string_lossy().into_owned())
}

fn user_dict_path(dict_name: &str) -> PathBuf {
    runtime_user_data_dir().join(format!("{dict_name}.userdb"))
}

fn user_dict_snapshot_path(dict_name: &str) -> PathBuf {
    runtime_user_data_sync_dir().join(format!("{dict_name}.userdb.txt"))
}

fn backup_plain_user_dict(dict_name: &str) -> bool {
    if dict_name.is_empty() {
        return false;
    }

    let source = user_dict_path(dict_name);
    if !source.is_file() {
        return false;
    }
    let snapshot = user_dict_snapshot_path(dict_name);
    if let Some(parent) = snapshot.parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    fs::copy(source, snapshot).is_ok()
}

fn sync_all_user_dicts() -> bool {
    let mut success = true;
    for dict_name in deployed_user_dict_names() {
        if !sync_plain_user_dict(&dict_name) {
            success = false;
        }
    }
    success
}

fn run_installation_update() -> bool {
    let (user_data_dir, current_sync_dir, distribution_code_name, distribution_version) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
            paths.sync_dir.to_string_lossy().into_owned(),
            paths.distribution_code_name.to_string_lossy().into_owned(),
            paths.distribution_version.to_string_lossy().into_owned(),
        )
    };

    if fs::create_dir_all(&user_data_dir).is_err() {
        return false;
    }

    let installation_path = user_data_dir.join("installation.yaml");
    let mut root = fs::read_to_string(&installation_path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .and_then(|value| match value {
            Value::Mapping(root) => Some(root),
            _ => None,
        })
        .unwrap_or_default();

    let installation_key = Value::String("installation_id".to_owned());
    let sync_key = Value::String("sync_dir".to_owned());
    let backup_key = Value::String("backup_config_files".to_owned());

    let existing_installation_id = root
        .get(&installation_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let installation_id = existing_installation_id.unwrap_or_else(|| {
        let generated = generate_installation_id();
        root.insert(installation_key.clone(), Value::String(generated.clone()));
        root.insert(
            Value::String("install_time".to_owned()),
            Value::String(current_unix_time_string()),
        );
        generated
    });

    let sync_dir = root
        .get(&sync_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            if current_sync_dir != "sync" {
                current_sync_dir
            } else {
                user_data_dir.join("sync").to_string_lossy().into_owned()
            }
        });
    let backup_config_files = root
        .get(&backup_key)
        .and_then(Value::as_bool)
        .unwrap_or(true);

    if root.contains_key(Value::String("install_time".to_owned())) {
        root.insert(
            Value::String("update_time".to_owned()),
            Value::String(current_unix_time_string()),
        );
    }
    if !distribution_code_name.is_empty() {
        root.insert(
            Value::String("distribution_code_name".to_owned()),
            Value::String(distribution_code_name.clone()),
        );
    }
    if !distribution_version.is_empty() {
        root.insert(
            Value::String("distribution_version".to_owned()),
            Value::String(distribution_version.clone()),
        );
    }
    root.insert(
        Value::String("rime_version".to_owned()),
        Value::String(
            String::from_utf8_lossy(&RIME_VERSION_BYTES[..RIME_VERSION_BYTES.len() - 1])
                .into_owned(),
        ),
    );

    let yaml = match serde_yaml::to_string(&Value::Mapping(root)) {
        Ok(yaml) => yaml,
        Err(_) => return false,
    };
    if fs::write(&installation_path, yaml).is_err() {
        return false;
    }

    let user_data_sync_dir = path_join(&sync_dir, &installation_id);
    let mut paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    paths.user_id = cstring_from_lossless_str(&installation_id);
    paths.sync_dir = cstring_from_lossless_str(&sync_dir);
    paths.user_data_sync_dir = cstring_from_lossless_str(&user_data_sync_dir);
    paths.backup_config_files = backup_config_files;
    true
}

fn generate_installation_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("yune-{nanos}-{}", std::process::id())
}

fn current_unix_time_string() -> String {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |_| "0".to_owned(),
        |duration| duration.as_secs().to_string(),
    )
}

fn backup_config_files() -> bool {
    let (user_data_dir, backup_enabled) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
            paths.backup_config_files,
        )
    };
    if !backup_enabled {
        return true;
    }

    let Ok(entries) = fs::read_dir(&user_data_dir) else {
        return false;
    };

    let backup_dir = runtime_user_data_sync_dir();
    let mut success = true;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !should_backup_config_file(&path) {
            continue;
        }
        if fs::create_dir_all(&backup_dir).is_err() {
            return false;
        }
        let destination = backup_dir.join(entry.file_name());
        if fs::copy(&path, destination).is_err() {
            success = false;
        }
    }
    success
}

fn cleanup_trash() -> bool {
    let user_data_dir = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned())
    };
    let Ok(entries) = fs::read_dir(&user_data_dir) else {
        return false;
    };

    let trash_dir = user_data_dir.join("trash");
    let mut success = true;
    let mut trash_created = false;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !should_cleanup_trash_file(&path) {
            continue;
        }
        if !trash_created {
            if fs::create_dir_all(&trash_dir).is_err() {
                return false;
            }
            trash_created = true;
        }
        if fs::rename(&path, trash_dir.join(entry.file_name())).is_err() {
            success = false;
        }
    }
    success
}

fn workspace_update() -> bool {
    if !deploy_config_file("default.yaml", "config_version") {
        return false;
    }

    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Some(schema_ids) = workspace_schema_ids(&default_config) else {
        return false;
    };

    let mut built = HashSet::new();
    let mut success = true;
    for schema_id in schema_ids {
        if !workspace_update_schema(&schema_id, false, &mut built) {
            success = false;
        }
    }

    write_last_build_time() && success
}

fn workspace_schema_ids(default_config: &Value) -> Option<Vec<String>> {
    let Value::Sequence(schema_list) = find_config_value(default_config, "schema_list")? else {
        return None;
    };
    Some(
        schema_list
            .iter()
            .filter_map(|entry| {
                let Value::Mapping(entry) = entry else {
                    return None;
                };
                entry
                    .get(Value::String("schema".to_owned()))
                    .and_then(Value::as_str)
                    .filter(|schema_id| !schema_id.is_empty())
                    .map(ToOwned::to_owned)
            })
            .collect(),
    )
}

fn workspace_update_schema(
    schema_id: &str,
    as_dependency: bool,
    built: &mut HashSet<String>,
) -> bool {
    if !built.insert(schema_id.to_owned()) {
        return true;
    }

    let schema_file = format!("{schema_id}.schema.yaml");
    if !deploy_schema_file(&schema_file) {
        return as_dependency;
    }

    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    for dependency_id in schema_dependencies(&schema_config) {
        if !workspace_update_schema(&dependency_id, true, built) {
            return false;
        }
    }
    true
}

fn schema_dependencies(schema_config: &Value) -> Vec<String> {
    let Some(Value::Sequence(dependencies)) =
        find_config_value(schema_config, "schema/dependencies")
    else {
        return Vec::new();
    };
    dependencies
        .iter()
        .filter_map(Value::as_str)
        .filter(|dependency| !dependency.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn write_last_build_time() -> bool {
    let user_data_dir = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned())
    };
    if fs::create_dir_all(&user_data_dir).is_err() {
        return false;
    }

    let user_config_path = user_data_dir.join("user.yaml");
    let mut user_config = fs::read_to_string(&user_config_path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| Value::Mapping(Mapping::new()));
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let timestamp = c_int::try_from(timestamp).unwrap_or(c_int::MAX);
    if !set_config_value(
        &mut user_config,
        "var/last_build_time",
        Value::Number(Number::from(timestamp)),
    ) {
        return false;
    }
    let Ok(yaml) = serde_yaml::to_string(&user_config) else {
        return false;
    };
    fs::write(user_config_path, yaml).is_ok()
}

fn user_dict_upgrade() -> bool {
    true
}

fn deploy_config_file(file_name: &str, version_key: &str) -> bool {
    if file_name.is_empty() || version_key.is_empty() {
        return false;
    }

    let (shared_data_dir, staging_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned()),
            PathBuf::from(paths.staging_dir.to_string_lossy().into_owned()),
        )
    };
    let source = shared_data_dir.join(file_name);
    let destination = staging_dir.join(file_name);
    if !source.is_file() {
        return false;
    }
    if source == destination {
        return true;
    }
    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    fs::copy(source, destination).is_ok()
}

fn deploy_schema_file(schema_file: &str) -> bool {
    if schema_file.is_empty() {
        return false;
    }

    let shared_data_dir = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned())
    };
    let source = shared_data_dir.join(schema_file);
    if !source.is_file() {
        return false;
    }

    let schema_config = match fs::read_to_string(source)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
    {
        Some(schema_config) => schema_config,
        None => return false,
    };
    let Some(schema_id) = find_config_value(&schema_config, "schema/schema_id")
        .and_then(Value::as_str)
        .filter(|schema_id| !schema_id.is_empty())
    else {
        return false;
    };

    deploy_config_file(&format!("{schema_id}.schema.yaml"), "schema/version")
}

fn prebuild_all_schemas() -> bool {
    let shared_data_dir = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned())
    };
    let Ok(entries) = fs::read_dir(&shared_data_dir) else {
        return false;
    };

    let mut success = true;
    for entry in entries {
        let Ok(entry) = entry else {
            success = false;
            continue;
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
            continue;
        };
        if file_name.ends_with(".schema.yaml") && !deploy_schema_file(file_name) {
            success = false;
        }
    }
    success
}

fn should_cleanup_trash_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
        return false;
    };
    file_name == "rime.log"
        || file_name.ends_with(".bin")
        || file_name.ends_with(".reverse.kct")
        || file_name.ends_with(".userdb.kct.old")
        || file_name.ends_with(".userdb.kct.snapshot")
}

fn should_backup_config_file(path: &Path) -> bool {
    let extension = path.extension().and_then(|extension| extension.to_str());
    match extension {
        Some("txt") => true,
        Some("yaml") => !is_generated_customized_copy(path),
        _ => false,
    }
}

fn is_generated_customized_copy(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
        return false;
    };
    if file_name.ends_with(".custom.yaml") {
        return false;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(Value::Mapping(root)) = serde_yaml::from_str::<Value>(&text) else {
        return false;
    };
    matches!(
        root.get(Value::String("customization".to_owned())),
        Some(Value::String(_))
    )
}

fn sync_plain_user_dict(dict_name: &str) -> bool {
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
    let destination = user_dict_path(dict_name);
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

fn deployed_schema_list_entry(default_config: &Value, entry: &Value) -> Option<String> {
    let Value::Mapping(entry) = entry else {
        return None;
    };
    if !schema_list_entry_conditions_match(default_config, entry) {
        return None;
    }
    entry
        .get(Value::String("schema".to_owned()))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn schema_list_entry_conditions_match(default_config: &Value, entry: &Mapping) -> bool {
    let Some(conditions) = entry.get(Value::String("case".to_owned())) else {
        return true;
    };
    let Value::Sequence(conditions) = conditions else {
        return true;
    };

    conditions.iter().all(|condition| {
        let Some(path) = condition.as_str() else {
            return false;
        };
        find_config_value(default_config, path)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    })
}

fn state_label_for_session(
    session_id: RimeSessionId,
    option_name: &str,
    state: bool,
    abbreviated: bool,
) -> Option<StateLabel> {
    if session_id == 0 {
        return None;
    }
    let schema_id = {
        let registry = sessions()
            .lock()
            .expect("session registry should not be poisoned");
        registry
            .sessions
            .get(&session_id)
            .map(|session| session.engine.status().schema_id)?
    };
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    switch_state_label(&schema_config, option_name, state, abbreviated)
}

fn switch_state_label(
    schema_config: &Value,
    option_name: &str,
    state: bool,
    abbreviated: bool,
) -> Option<StateLabel> {
    let switches = find_config_value(schema_config, "switches")?;
    let Value::Sequence(switches) = switches else {
        return None;
    };

    for the_switch in switches {
        let Value::Mapping(switch_map) = the_switch else {
            continue;
        };
        if switch_string_field(switch_map, "name").is_some_and(|name| name == option_name) {
            return label_from_switch(switch_map, usize::from(state), abbreviated);
        }

        let Some(options) = switch_map.get(Value::String("options".to_owned())) else {
            continue;
        };
        let Value::Sequence(options) = options else {
            continue;
        };
        for (option_index, option) in options.iter().enumerate() {
            if matches!(option, Value::String(name) if name == option_name) {
                return state
                    .then(|| label_from_switch(switch_map, option_index, abbreviated))
                    .flatten();
            }
        }
    }
    None
}

fn switch_string_field<'a>(switch_map: &'a Mapping, key: &str) -> Option<&'a str> {
    switch_map
        .get(Value::String(key.to_owned()))
        .and_then(Value::as_str)
}

fn label_from_switch(
    switch_map: &Mapping,
    state_index: usize,
    abbreviated: bool,
) -> Option<StateLabel> {
    if abbreviated {
        if let Some(label) = label_list_value(switch_map, "abbrev", state_index) {
            let length = label.len();
            return Some(StateLabel {
                value: label,
                length,
            });
        }
    }

    let label = label_list_value(switch_map, "states", state_index)?;
    let length = if abbreviated {
        first_unicode_byte_length(&label)
    } else {
        label.len()
    };
    Some(StateLabel {
        value: label,
        length,
    })
}

fn label_list_value(switch_map: &Mapping, key: &str, state_index: usize) -> Option<String> {
    let Value::Sequence(values) = switch_map.get(Value::String(key.to_owned()))? else {
        return None;
    };
    values.get(state_index)?.as_str().map(ToOwned::to_owned)
}

fn first_unicode_byte_length(value: &str) -> usize {
    value.chars().next().map_or(0, |first| first.len_utf8())
}

fn empty_string_slice() -> RimeStringSlice {
    RimeStringSlice {
        str: ptr::null(),
        length: 0,
    }
}

fn runtime_path_ptr(select: impl FnOnce(&RuntimePaths) -> &CString) -> *const c_char {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    select(&paths).as_ptr()
}

fn notify(session_id: RimeSessionId, message_type: &str, message_value: &str) {
    let (handler, context_object) = {
        let state = notification_state()
            .lock()
            .expect("notification state should not be poisoned");
        let Some(handler) = state.handler else {
            return;
        };
        (handler, state.context_object)
    };
    let Ok(message_type) = CString::new(message_type) else {
        return;
    };
    let Ok(message_value) = CString::new(message_value) else {
        return;
    };
    handler(
        context_object as *mut c_void,
        session_id,
        message_type.as_ptr(),
        message_value.as_ptr(),
    );
}

fn copy_runtime_path_to_buffer(
    select: impl FnOnce(&RuntimePaths) -> &CString,
    output: *mut c_char,
    buffer_size: usize,
) {
    if output.is_null() || buffer_size == 0 {
        return;
    }

    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    let value = select(&paths).to_string_lossy();
    copy_c_string_to_buffer(&value, output, buffer_size);
}

fn clear_commit(commit: *mut RimeCommit) {
    // SAFETY: callers only pass non-null pointers to this helper; fields are
    // plain integers/pointers and assigning null mirrors librime's clear macro.
    unsafe {
        (*commit).data_size = 0;
        (*commit).text = ptr::null_mut();
    }
}

fn clear_context(context: *mut RimeContext) {
    // SAFETY: callers only pass non-null pointers to this helper; this mirrors
    // librime's versioned struct clear by preserving `data_size`.
    unsafe {
        (*context).composition = RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: ptr::null_mut(),
        };
        (*context).menu = RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        };
        (*context).commit_text_preview = ptr::null_mut();
        (*context).select_labels = ptr::null_mut();
    }
}

fn clear_status(status: *mut RimeStatus) {
    // SAFETY: callers only pass non-null pointers to this helper; this mirrors
    // librime's versioned struct clear by preserving `data_size`.
    unsafe {
        (*status).schema_id = ptr::null_mut();
        (*status).schema_name = ptr::null_mut();
        (*status).is_disabled = FALSE;
        (*status).is_composing = FALSE;
        (*status).is_ascii_mode = FALSE;
        (*status).is_full_shape = FALSE;
        (*status).is_simplified = FALSE;
        (*status).is_traditional = FALSE;
        (*status).is_ascii_punct = FALSE;
    }
}

fn clear_schema_list(schema_list: *mut RimeSchemaList) {
    // SAFETY: callers only pass non-null pointers to this helper; fields are
    // plain integers/pointers and assigning null mirrors librime cleanup.
    unsafe {
        (*schema_list).size = 0;
        (*schema_list).list = ptr::null_mut();
    }
}

unsafe fn clear_user_dict_iterator(iterator: *mut RimeUserDictIterator) {
    if iterator.is_null() {
        return;
    }
    // SAFETY: `iterator` is non-null and any non-null state pointer is owned by
    // this shim after successful iterator initialization.
    unsafe {
        if !(*iterator).ptr.is_null() {
            drop(Box::from_raw((*iterator).ptr.cast::<UserDictListState>()));
        }
        (*iterator).ptr = ptr::null_mut();
        (*iterator).i = 0;
    }
}

fn free_context_fields(context: *mut RimeContext) {
    // SAFETY: `context` is non-null and nested pointers are owned by this API
    // when populated by `RimeGetContext`.
    unsafe {
        if !(*context).composition.preedit.is_null() {
            drop(CString::from_raw((*context).composition.preedit));
        }
        if !(*context).menu.candidates.is_null() && (*context).menu.num_candidates > 0 {
            let num_candidates = (*context).menu.num_candidates as usize;
            let mut candidates =
                Vec::from_raw_parts((*context).menu.candidates, num_candidates, num_candidates);
            free_rime_candidates(&mut candidates);
        }
        if !(*context).menu.select_keys.is_null() {
            drop(CString::from_raw((*context).menu.select_keys));
        }
        if !(*context).commit_text_preview.is_null() {
            drop(CString::from_raw((*context).commit_text_preview));
        }
        if !(*context).select_labels.is_null() {
            let page_size = (*context).menu.page_size.max(0) as usize;
            let labels = Vec::from_raw_parts((*context).select_labels, page_size, page_size);
            for label in labels {
                if !label.is_null() {
                    drop(CString::from_raw(label));
                }
            }
        }
    }
}

fn free_schema_list_fields(schema_list: *mut RimeSchemaList) {
    // SAFETY: `schema_list` is non-null and nested pointers are owned by this
    // API when populated by `RimeGetSchemaList`.
    unsafe {
        if (*schema_list).list.is_null() {
            return;
        }
        let size = (*schema_list).size;
        let mut list = Vec::from_raw_parts((*schema_list).list, size, size);
        free_schema_list_items(&mut list);
    }
}

fn free_schema_list_items(list: &mut [RimeSchemaListItem]) {
    for item in list {
        if !item.schema_id.is_null() {
            // SAFETY: schema ids are allocated by `CString::into_raw` in
            // `RimeGetSchemaList` and are released at most once here.
            unsafe { drop(CString::from_raw(item.schema_id)) };
            item.schema_id = ptr::null_mut();
        }
        if !item.name.is_null() {
            // SAFETY: names are allocated by `CString::into_raw` in
            // `RimeGetSchemaList` and are released at most once here.
            unsafe { drop(CString::from_raw(item.name)) };
            item.name = ptr::null_mut();
        }
        if !item.reserved.is_null() {
            // SAFETY: levers available-schema lists store opaque
            // `LeverSchemaInfo` boxes in `reserved`; other schema-list APIs
            // keep this field null.
            unsafe { drop(Box::from_raw(item.reserved.cast::<LeverSchemaInfo>())) };
            item.reserved = ptr::null_mut();
        }
    }
}

fn free_status_fields(status: *mut RimeStatus) {
    // SAFETY: `status` is non-null and nested pointers are owned by this API
    // when populated by `RimeGetStatus`.
    unsafe {
        if !(*status).schema_id.is_null() {
            drop(CString::from_raw((*status).schema_id));
        }
        if !(*status).schema_name.is_null() {
            drop(CString::from_raw((*status).schema_name));
        }
    }
}

fn free_rime_candidates(candidates: &mut Vec<RimeCandidate>) {
    for mut candidate in candidates.drain(..) {
        free_candidate_fields(&mut candidate);
    }
}

fn free_candidate_fields(candidate: &mut RimeCandidate) {
    if !candidate.text.is_null() {
        // SAFETY: candidate text pointers were returned by CString::into_raw
        // while populating a RimeContext or candidate-list iterator.
        unsafe {
            drop(CString::from_raw(candidate.text));
        }
        candidate.text = ptr::null_mut();
    }
    if !candidate.comment.is_null() {
        // SAFETY: candidate comment pointers were returned by CString::into_raw
        // while populating a RimeContext or candidate-list iterator.
        unsafe {
            drop(CString::from_raw(candidate.comment));
        }
        candidate.comment = ptr::null_mut();
    }
    candidate.reserved = ptr::null_mut();
}

fn copy_c_string_to_buffer(value: &str, output: *mut c_char, buffer_size: usize) {
    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(buffer_size.saturating_sub(1));
    // SAFETY: callers pass writable storage of `buffer_size` bytes, and
    // `copy_len` is bounded to leave room for a trailing NUL.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), output, copy_len);
        slice::from_raw_parts_mut(output.cast::<u8>(), buffer_size)[copy_len] = 0;
    }
}

unsafe fn config_state_mut(config: *mut RimeConfig) -> Option<&'static mut ConfigState> {
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

unsafe fn config_lookup(config: *mut RimeConfig, key: *const c_char) -> Option<Value> {
    let key = unsafe { c_string_key(key) }?;
    unsafe { config_lookup_key(config, &key) }
}

unsafe fn config_lookup_key(config: *mut RimeConfig, key: &str) -> Option<Value> {
    let state = unsafe { config_state_mut(config) }?;
    find_config_value(&state.root, key).cloned()
}

unsafe fn config_string_value(config: *mut RimeConfig, key: *const c_char) -> Option<String> {
    match unsafe { config_lookup(config, key) }? {
        Value::String(value) => Some(value),
        _ => None,
    }
}

unsafe fn config_set(config: *mut RimeConfig, key: *const c_char, value: Value) -> Bool {
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };
    let Some(state) = (unsafe { config_state_mut(config) }) else {
        return FALSE;
    };
    state.cstring_cache = None;
    bool_from(set_config_value(&mut state.root, &key, value))
}

unsafe fn c_string_key(key: *const c_char) -> Option<String> {
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

unsafe fn levers_custom_settings_mut(
    settings: *mut RimeCustomSettings,
) -> Option<&'static mut LeverCustomSettings> {
    if settings.is_null() {
        return None;
    }
    // SAFETY: levers custom settings pointers are allocated by
    // `RimeLeversCustomSettingsInit`.
    Some(unsafe { &mut *settings.cast::<LeverCustomSettings>() })
}

unsafe fn levers_customize_value(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: Value,
) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };

    let Value::Mapping(root) = ensure_mapping(&mut settings.custom_config.root) else {
        return FALSE;
    };
    let patch_key = Value::String("patch".to_owned());
    if !matches!(root.get(&patch_key), Some(Value::Mapping(_))) {
        root.insert(patch_key.clone(), Value::Mapping(Mapping::new()));
    }
    let Some(Value::Mapping(patch)) = root.get_mut(&patch_key) else {
        return FALSE;
    };
    patch.insert(Value::String(key), value);
    settings.custom_config.cstring_cache = None;
    settings.modified = true;
    TRUE
}

fn custom_config_path(config_id: &str) -> PathBuf {
    let config_name = config_id.strip_suffix(".schema").unwrap_or(config_id);
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    Path::new(paths.user_data_dir.to_string_lossy().as_ref())
        .join(format!("{config_name}.custom.yaml"))
}

fn write_config_signature(root: &mut Value, key: &str, generator: &str) {
    let modified_time = SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |_| "0".to_owned(),
        |duration| duration.as_secs().to_string(),
    );
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

    for (path, value) in [
        (format!("{key}/generator"), generator.to_owned()),
        (format!("{key}/modified_time"), modified_time),
        (
            format!("{key}/distribution_code_name"),
            distribution_code_name,
        ),
        (format!("{key}/distribution_version"), distribution_version),
        (format!("{key}/rime_version"), rime_version),
    ] {
        let _ = set_config_value(root, &path, Value::String(value));
    }
}

unsafe fn config_iterator_begin(
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

fn config_child_path(root_path: &str, child_key: &str) -> String {
    if root_path.is_empty() || root_path == "/" {
        child_key.to_owned()
    } else {
        format!("{root_path}/{child_key}")
    }
}

fn find_config_value<'a>(root: &'a Value, key: &str) -> Option<&'a Value> {
    if key.is_empty() {
        return Some(root);
    }

    let mut current = root;
    for segment in key.split('/').filter(|segment| !segment.is_empty()) {
        if let Some(index) = list_index(segment) {
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

fn set_config_value(root: &mut Value, key: &str, value: Value) -> bool {
    if key.is_empty() {
        *root = value;
        return true;
    }

    let segments = key
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let Some((last, parents)) = segments.split_last() else {
        *root = value;
        return true;
    };

    let mut current = root;
    for segment in parents {
        let Value::Mapping(mapping) = ensure_mapping(current) else {
            return false;
        };
        current = mapping
            .entry(Value::String((*segment).to_owned()))
            .or_insert_with(|| Value::Mapping(Mapping::new()));
    }

    if let Some(index) = list_index(last) {
        let Value::Sequence(sequence) = current else {
            return false;
        };
        if index == sequence.len() {
            sequence.push(value);
            true
        } else if let Some(slot) = sequence.get_mut(index) {
            *slot = value;
            true
        } else {
            false
        }
    } else {
        let Value::Mapping(mapping) = ensure_mapping(current) else {
            return false;
        };
        mapping.insert(Value::String((*last).to_owned()), value);
        true
    }
}

fn remove_config_value(root: &mut Value, key: &str) -> bool {
    if key.is_empty() {
        *root = Value::Null;
        return true;
    }

    let segments = key
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let Some((last, parents)) = segments.split_last() else {
        return false;
    };

    let mut current = root;
    for segment in parents {
        let Value::Mapping(mapping) = current else {
            return false;
        };
        let Some(next) = mapping.get_mut(Value::String((*segment).to_owned())) else {
            return false;
        };
        current = next;
    }

    if let Some(index) = list_index(last) {
        let Value::Sequence(sequence) = current else {
            return false;
        };
        if index < sequence.len() {
            sequence.remove(index);
            true
        } else {
            false
        }
    } else {
        let Value::Mapping(mapping) = current else {
            return false;
        };
        mapping.remove(Value::String((*last).to_owned())).is_some()
    }
}

fn ensure_mapping(value: &mut Value) -> &mut Value {
    if !matches!(value, Value::Mapping(_)) {
        *value = Value::Mapping(Mapping::new());
    }
    value
}

fn list_index(segment: &str) -> Option<usize> {
    segment.strip_prefix('@')?.parse().ok()
}

#[cfg(test)]
mod tests {
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
        bool_from, find_config_value, rime_get_api, RimeApi, RimeCandidateListBegin,
        RimeCandidateListEnd, RimeCandidateListFromIndex, RimeCandidateListIterator,
        RimeCandidateListNext, RimeChangePage, RimeCleanupAllSessions, RimeCleanupStaleSessions,
        RimeClearComposition, RimeCommit, RimeCommitComposition, RimeConfig, RimeConfigBeginList,
        RimeConfigBeginMap, RimeConfigClear, RimeConfigClose, RimeConfigCreateList,
        RimeConfigCreateMap, RimeConfigEnd, RimeConfigGetBool, RimeConfigGetCString,
        RimeConfigGetDouble, RimeConfigGetInt, RimeConfigGetItem, RimeConfigGetString,
        RimeConfigInit, RimeConfigIterator, RimeConfigListSize, RimeConfigLoadString,
        RimeConfigNext, RimeConfigOpen, RimeConfigSetBool, RimeConfigSetDouble, RimeConfigSetInt,
        RimeConfigSetItem, RimeConfigSetString, RimeConfigUpdateSignature, RimeContext,
        RimeCreateSession, RimeCustomApi, RimeDeleteCandidate, RimeDeleteCandidateOnCurrentPage,
        RimeDeployConfigFile, RimeDeploySchema, RimeDeployWorkspace, RimeDeployerInitialize,
        RimeDestroySession, RimeFinalize, RimeFindModule, RimeFindSession, RimeFreeCommit,
        RimeFreeContext, RimeFreeStatus, RimeGetCaretPos, RimeGetCommit, RimeGetContext,
        RimeGetCurrentSchema, RimeGetInput, RimeGetOption, RimeGetPrebuiltDataDir,
        RimeGetPrebuiltDataDirSecure, RimeGetProperty, RimeGetSchemaList, RimeGetSharedDataDir,
        RimeGetSharedDataDirSecure, RimeGetStagingDir, RimeGetStagingDirSecure, RimeGetStateLabel,
        RimeGetStateLabelAbbreviated, RimeGetStatus, RimeGetSyncDir, RimeGetSyncDirSecure,
        RimeGetUserDataDir, RimeGetUserDataDirSecure, RimeGetUserDataSyncDir, RimeGetUserId,
        RimeGetVersion, RimeHighlightCandidate, RimeHighlightCandidateOnCurrentPage,
        RimeInitialize, RimeIsMaintenancing, RimeJoinMaintenanceThread, RimeLeversApi, RimeModule,
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
        let radio = unsafe {
            RimeGetStateLabelAbbreviated(session_id, simplification.as_ptr(), TRUE, TRUE)
        };
        assert_eq!(radio.length, "简".len());
        // SAFETY: `radio.str` points to a C string and `length` is within its
        // first UTF-8 scalar value.
        let radio_slice =
            unsafe { std::slice::from_raw_parts(radio.str.cast::<u8>(), radio.length) };
        assert_eq!(std::str::from_utf8(radio_slice), Ok("简"));

        // SAFETY: option names are valid NUL-terminated strings.
        let hidden_radio = unsafe {
            RimeGetStateLabelAbbreviated(session_id, simplification.as_ptr(), FALSE, TRUE)
        };
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
        assert_eq!(
            unsafe { RimeConfigBeginList(&mut iterator, &mut config, missing.as_ptr()) },
            FALSE
        );
        assert!(iterator.list.is_null());

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
        let distribution_code =
            CString::new("yune-test").expect("distribution code should be valid");
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

        RimeSetupLogging(task_name.as_ptr());
        assert_eq!(RimeStartMaintenance(TRUE), TRUE);
        assert_eq!(RimeStartMaintenanceOnWorkspaceChange(), TRUE);
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
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
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
        // SAFETY: config was opened by the config API.
        assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

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
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
        let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
        let schema_file = CString::new("luna.schema.yaml").expect("schema file should be valid");
        let invalid_schema =
            CString::new("invalid.schema.yaml").expect("schema file should be valid");
        let missing_schema =
            CString::new("missing.schema.yaml").expect("schema file should be valid");
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
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
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
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
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
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
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
    fn registers_and_finds_modules_by_name() {
        let _guard = test_guard();
        super::module_registry()
            .lock()
            .expect("module registry should not be poisoned")
            .modules_by_name
            .clear();
        let module_name = CString::new("sample_module_abi").expect("module name should be valid");
        let replacement_name =
            CString::new("sample_module_abi").expect("module name should be valid");
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
        fs::remove_dir_all(user.join("luna_pinyin.userdb"))
            .expect("user dict dir should be removed");
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
            fs::read_to_string(user.join("luna_pinyin.userdb"))
                .expect("user dict should be readable")
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
            fs::read_to_string(user.join("luna_pinyin.userdb"))
                .expect("user dict should be readable")
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
        let installation_metadata: Value = serde_yaml::from_str(&installation_metadata)
            .expect("installation metadata should parse");
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
        fs::remove_file(sync_user_dir.join("default.yaml"))
            .expect("config backup should be removable");
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

        let custom_bool_key =
            CString::new("switches/@0/reset").expect("custom key should be valid");
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
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
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
        let property_value =
            CString::new("sample_console").expect("property value should be valid");
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
            unsafe {
                RimeGetCurrentSchema(session_id, short_buffer.as_mut_ptr(), short_buffer.len())
            },
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
}
