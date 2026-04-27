use std::{
    collections::{HashMap, HashSet},
    ffi::{c_void, CStr, CString},
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use regex::Regex;
use serde_yaml::{Mapping, Number, Value};
use yune_core::{
    parse_key_sequence, CharsetFilter, Engine, HistoryTranslator, KeyCode, KeyEvent, KeyModifiers,
    PunctuationTranslator, ReverseLookupFilter, ReverseLookupTranslator, SimplifierFilter,
    SingleCharFilter, StaticTableTranslator, TableDictionary, TaggedFilter, UniquifierFilter,
};

mod abi;
mod config;
mod config_compiler;
mod key_table;
pub use abi::*;
use config::*;
use config_compiler::*;
pub use key_table::*;

const XK_BACKSPACE: c_int = 0xff08;
const XK_ESCAPE: c_int = 0xff1b;
const XK_RETURN: c_int = 0xff0d;
const XK_DELETE: c_int = 0xffff;
const XK_HOME: c_int = 0xff50;
const XK_LEFT: c_int = 0xff51;
const XK_UP: c_int = 0xff52;
const XK_RIGHT: c_int = 0xff53;
const XK_DOWN: c_int = 0xff54;
const XK_PAGE_UP: c_int = 0xff55;
const XK_PAGE_DOWN: c_int = 0xff56;
const XK_END: c_int = 0xff57;
const XK_KP_ENTER: c_int = 0xff8d;
const XK_KP_HOME: c_int = 0xff95;
const XK_KP_LEFT: c_int = 0xff96;
const XK_KP_UP: c_int = 0xff97;
const XK_KP_RIGHT: c_int = 0xff98;
const XK_KP_DOWN: c_int = 0xff99;
const XK_KP_PAGE_UP: c_int = 0xff9a;
const XK_KP_PAGE_DOWN: c_int = 0xff9b;
const XK_KP_END: c_int = 0xff9c;
const XK_KP_0: c_int = 0xffb0;
const XK_KP_9: c_int = 0xffb9;
const K_SHIFT_MASK: c_int = 1 << 0;
const K_CONTROL_MASK: c_int = 1 << 2;
#[cfg(test)]
const K_ALT_MASK: c_int = 1 << 3;
#[cfg(test)]
const K_SUPER_MASK: c_int = 1 << 26;
#[cfg(test)]
const K_RELEASE_MASK: c_int = 1 << 30;
const DEFAULT_PAGE_SIZE: usize = 5;
const SESSION_LIFESPAN_SECS: u64 = 5 * 60;
const RIME_VERSION_BYTES: &[u8] =
    concat!("yune-rime-api ", env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[derive(Default)]
struct SessionRegistry {
    next_id: RimeSessionId,
    sessions: HashMap<RimeSessionId, SessionState>,
}

impl SessionRegistry {
    fn create_session(&mut self) -> RimeSessionId {
        if !service_started().load(Ordering::SeqCst) {
            return 0;
        }

        self.next_id = self.next_id.saturating_add(1).max(1);
        let session_id = self.next_id;
        self.sessions.insert(session_id, SessionState::new());
        session_id
    }

    fn get_session_mut(&mut self, session_id: RimeSessionId) -> Option<&mut SessionState> {
        if session_id == 0 || !service_started().load(Ordering::SeqCst) {
            return None;
        }

        let session = self.sessions.get_mut(&session_id)?;
        session.activate();
        Some(session)
    }

    fn find_session(&mut self, session_id: RimeSessionId) -> bool {
        self.get_session_mut(session_id).is_some()
    }

    fn cleanup_stale_sessions(&mut self) {
        let now = session_activity_now();
        self.sessions.retain(|_, session| {
            now.saturating_sub(session.last_active_time) <= SESSION_LIFESPAN_SECS
        });
    }
}

struct SessionState {
    engine: Engine,
    unread_commit: Option<String>,
    input_buffer: Option<CString>,
    key_binder: Option<KeyBinderProcessor>,
    ascii_composer_enabled: bool,
    ascii_segmentor_enabled: bool,
    punctuation_processor: Option<PunctuationProcessor>,
    recognizer_processor: Option<RecognizerProcessor>,
    base_segment_tags: Vec<String>,
    affix_segmentors: Vec<AffixSegmentor>,
    matcher_segmentor: Option<MatcherSegmentor>,
    paging: bool,
    last_active_time: u64,
}

impl SessionState {
    fn new() -> Self {
        Self {
            engine: Engine::default(),
            unread_commit: None,
            input_buffer: None,
            key_binder: None,
            ascii_composer_enabled: false,
            ascii_segmentor_enabled: false,
            punctuation_processor: None,
            recognizer_processor: None,
            base_segment_tags: vec!["abc".to_owned()],
            affix_segmentors: Vec::new(),
            matcher_segmentor: None,
            paging: false,
            last_active_time: session_activity_now(),
        }
    }

    fn activate(&mut self) {
        self.last_active_time = session_activity_now();
    }
}

struct KeyBinderProcessor {
    bindings: HashMap<KeyEvent, Vec<KeyBinding>>,
    redirecting: bool,
    last_key: Option<char>,
}

struct KeyBinding {
    condition: KeyBindingCondition,
    action: KeyBindingAction,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum KeyBindingCondition {
    Always,
    Composing,
    HasMenu,
    Paging,
}

#[derive(Clone)]
enum KeyBindingAction {
    Send(Vec<KeyEvent>),
    Toggle(String),
    SetOption { option: String, value: bool },
    SelectSchema(String),
}

struct KeyBindingSwitchOption {
    options: Vec<String>,
    option_index: usize,
    reset_index: usize,
}

enum KeyBindingSwitchTarget {
    Toggle(String),
    Radio(KeyBindingSwitchOption),
}

struct MatcherSegmentor {
    patterns: Vec<MatcherPattern>,
}

struct AffixSegmentor {
    tag: String,
    prefix: String,
    suffix: String,
    extra_tags: Vec<String>,
}

struct RecognizerProcessor {
    use_space: bool,
    patterns: Vec<MatcherPattern>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AsciiComposerProcessResult {
    Noop,
    Accepted,
    Rejected,
}

struct MatcherPattern {
    tag: String,
    pattern: Regex,
}

struct PunctuationProcessor {
    use_space: bool,
    digit_separators: String,
    digit_separator_commit: bool,
    half_shape_alternating_counts: HashMap<String, usize>,
    full_shape_alternating_counts: HashMap<String, usize>,
    symbol_alternating_counts: HashMap<String, usize>,
    half_shape_unique_commits: HashMap<String, String>,
    full_shape_unique_commits: HashMap<String, String>,
    symbol_unique_commits: HashMap<String, String>,
    half_shape_pairs: HashMap<String, [String; 2]>,
    full_shape_pairs: HashMap<String, [String; 2]>,
    symbol_pairs: HashMap<String, [String; 2]>,
    pair_oddness: HashMap<String, usize>,
    pending_digit_separator: Option<String>,
}

enum PunctuationProcessResult {
    Accepted,
    Commit(String),
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

struct CandidateListState {
    candidates: Vec<yune_core::Candidate>,
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

#[derive(Clone)]
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

struct ContextMenuSettings {
    page_size: usize,
    select_keys: Option<String>,
    select_labels: Vec<String>,
}

type SwitcherAvailableSchemaRegistry = HashMap<usize, Option<Vec<LeverSchemaInfo>>>;

#[derive(Clone, Copy)]
enum ConfigOpenKind {
    Deployed,
    User,
}

struct RuntimePaths {
    shared_data_dir: CString,
    user_data_dir: CString,
    prebuilt_data_dir: CString,
    staging_dir: CString,
    sync_dir: CString,
    user_id: CString,
    user_data_sync_dir: CString,
    distribution_name: CString,
    distribution_code_name: CString,
    distribution_version: CString,
    app_name: CString,
    log_dir: CString,
    backup_config_files: bool,
}

struct RuntimePathArgs<'a> {
    shared_data_dir: &'a str,
    user_data_dir: &'a str,
    prebuilt_data_dir: &'a str,
    staging_dir: &'a str,
    sync_dir: &'a str,
    user_id: &'a str,
    distribution: (&'a str, &'a str, &'a str),
    app_name: &'a str,
    log_dir: &'a str,
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
            distribution: ("", "", ""),
            app_name: "",
            log_dir: "",
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
            distribution_name: cstring_from_lossless_str(args.distribution.0),
            distribution_code_name: cstring_from_lossless_str(args.distribution.1),
            distribution_version: cstring_from_lossless_str(args.distribution.2),
            app_name: cstring_from_lossless_str(args.app_name),
            log_dir: cstring_from_lossless_str(args.log_dir),
            backup_config_files: args.backup_config_files,
        }
    }

    unsafe fn from_traits(traits: *const RimeTraits) -> Option<Self> {
        if traits.is_null() {
            return None;
        }

        // SAFETY: callers promise that `traits`, when non-null, points to at
        // least the leading `data_size` field of a `RimeTraits` object.
        let data_size = unsafe { (*traits).data_size };
        let provided_string = |member: *const *const c_char| {
            if rime_struct_has_member(traits, data_size, member) {
                // SAFETY: the field is covered by `data_size`; callers promise
                // that provided non-null strings are NUL-terminated.
                unsafe { optional_c_string(*member) }
            } else {
                None
            }
        };

        let shared_data_dir = provided_string(unsafe { ptr::addr_of!((*traits).shared_data_dir) })
            .unwrap_or_else(|| ".".to_owned());
        let user_data_dir = provided_string(unsafe { ptr::addr_of!((*traits).user_data_dir) })
            .unwrap_or_else(|| ".".to_owned());
        let prebuilt_data_dir =
            provided_string(unsafe { ptr::addr_of!((*traits).prebuilt_data_dir) })
                .unwrap_or_else(|| path_join(&shared_data_dir, "build"));
        let staging_dir = provided_string(unsafe { ptr::addr_of!((*traits).staging_dir) })
            .unwrap_or_else(|| path_join(&user_data_dir, "build"));
        let distribution_name =
            provided_string(unsafe { ptr::addr_of!((*traits).distribution_name) })
                .unwrap_or_default();
        let distribution_code_name =
            provided_string(unsafe { ptr::addr_of!((*traits).distribution_code_name) })
                .unwrap_or_default();
        let distribution_version =
            provided_string(unsafe { ptr::addr_of!((*traits).distribution_version) })
                .unwrap_or_default();
        let app_name =
            provided_string(unsafe { ptr::addr_of!((*traits).app_name) }).unwrap_or_default();
        let log_dir =
            provided_string(unsafe { ptr::addr_of!((*traits).log_dir) }).unwrap_or_default();
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
            distribution: (
                &distribution_name,
                &distribution_code_name,
                &distribution_version,
            ),
            app_name: &app_name,
            log_dir: &log_dir,
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

fn service_started() -> &'static AtomicBool {
    static SERVICE_STARTED: AtomicBool = AtomicBool::new(false);
    &SERVICE_STARTED
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

fn switcher_hotkeys_registry() -> &'static Mutex<HashMap<usize, Option<CString>>> {
    static SWITCHER_HOTKEYS_REGISTRY: OnceLock<Mutex<HashMap<usize, Option<CString>>>> =
        OnceLock::new();
    SWITCHER_HOTKEYS_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn switcher_available_schema_registry() -> &'static Mutex<SwitcherAvailableSchemaRegistry> {
    static SWITCHER_AVAILABLE_SCHEMA_REGISTRY: OnceLock<Mutex<SwitcherAvailableSchemaRegistry>> =
        OnceLock::new();
    SWITCHER_AVAILABLE_SCHEMA_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
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

fn non_empty_cstring_ptr(value: &CString) -> Option<*const c_char> {
    if value.as_bytes().is_empty() {
        None
    } else {
        Some(value.as_ptr())
    }
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
    switcher_available_schema_registry()
        .lock()
        .expect("switcher available schema registry should not be poisoned")
        .insert(settings as usize, Some(deployed_levers_schema_infos()));
    switcher_selection_registry()
        .lock()
        .expect("switcher selection registry should not be poisoned")
        .insert(settings as usize, Some(deployed_selected_schema_ids()));
    switcher_hotkeys_registry()
        .lock()
        .expect("switcher hotkeys registry should not be poisoned")
        .insert(
            settings as usize,
            deployed_switcher_hotkeys().map(|hotkeys| cstring_from_lossless_str(&hotkeys)),
        );
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
            cstring_borrows: Vec::new(),
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
    settings.config.cstring_borrows.clear();
    settings.modified = false;

    let path = custom_config_path(&settings.config_id);
    let loaded = fs::read_to_string(path)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok());
    match loaded {
        Some(root) => {
            settings.custom_config.root = root;
            settings.custom_config.cstring_borrows.clear();
            TRUE
        }
        None => {
            settings.custom_config.root = Value::Null;
            settings.custom_config.cstring_borrows.clear();
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
    let available_schema_infos = switcher_available_schema_registry()
        .lock()
        .expect("switcher available schema registry should not be poisoned")
        .get(&(settings as usize))
        .cloned()
        .flatten()
        .unwrap_or_else(deployed_levers_schema_infos);
    populate_levers_schema_list(list, available_schema_infos)
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
    unsafe { levers_schema_info_ptr(info, |info| non_empty_cstring_ptr(&info.schema_id)) }
}

/// Returns the schema name from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaName(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe { levers_schema_info_ptr(info, |info| non_empty_cstring_ptr(&info.name)) }
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
            info.version.as_ref().and_then(non_empty_cstring_ptr)
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
            info.author.as_ref().and_then(non_empty_cstring_ptr)
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
            info.description.as_ref().and_then(non_empty_cstring_ptr)
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
            info.file_path.as_ref().and_then(non_empty_cstring_ptr)
        })
    }
}

/// Selects schema IDs on the opaque switcher settings object.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `schema_id_list` must point to `count` valid NUL-terminated
/// strings when `count` is positive; non-positive counts select an empty list.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSelectSchemas(
    settings: *mut RimeSwitcherSettings,
    schema_id_list: *const *const c_char,
    count: c_int,
) -> Bool {
    if settings.is_null() || (count > 0 && schema_id_list.is_null()) {
        return FALSE;
    }

    let count = usize::try_from(count).unwrap_or(0);
    let mut selected_schema_ids = Vec::with_capacity(count);
    for index in 0..count {
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

    switcher_hotkeys_registry()
        .lock()
        .expect("switcher hotkeys registry should not be poisoned")
        .get(&(settings as usize))
        .and_then(Option::as_ref)
        .map_or(ptr::null(), |hotkeys| hotkeys.as_ptr())
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
pub extern "C" fn RimeSetupLogging(app_name: *const c_char) {
    let Some(app_name) = optional_c_string(app_name) else {
        return;
    };
    runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned")
        .app_name = cstring_from_lossless_str(&app_name);
}

/// Initializes the runtime using the same trait handling as `RimeSetup`.
///
/// # Safety
///
/// `traits` follows the same preconditions as `RimeSetup`.
#[no_mangle]
pub unsafe extern "C" fn RimeInitialize(traits: *const RimeTraits) {
    // SAFETY: forwarded preconditions are identical to `RimeSetup`.
    unsafe { RimeSetup(traits) };
    service_started().store(true, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn RimeFinalize() {
    RimeCleanupAllSessions();
    service_started().store(false, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn RimeStartMaintenance(full_check: Bool) -> Bool {
    let _ = clean_old_log_files();
    if !run_installation_update() {
        return FALSE;
    }
    if full_check == FALSE && !detect_modifications() {
        return FALSE;
    }
    notify(0, "deploy", "start");
    let success = run_workspace_maintenance_tasks();
    notify(0, "deploy", if success { "success" } else { "failure" });
    bool_from(success)
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
    if !run_workspace_maintenance_tasks() {
        return FALSE;
    }
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
    notify(0, "deploy", "start");
    let installation_synced = run_installation_update();
    let configs_synced = backup_config_files();
    let user_dicts_synced = sync_all_user_dicts();
    let success = installation_synced && configs_synced && user_dicts_synced;
    notify(0, "deploy", if success { "success" } else { "failure" });
    bool_from(success)
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
    if task_name == "clean_old_log_files" {
        return bool_from(clean_old_log_files());
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
    FALSE
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
    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    bool_from(registry.find_session(session_id))
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
pub extern "C" fn RimeCleanupStaleSessions() {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .cleanup_stale_sessions();
}

#[no_mangle]
pub extern "C" fn RimeProcessKey(session_id: RimeSessionId, keycode: c_int, mask: c_int) -> Bool {
    if session_id == 0
        || (mask != 0
            && !((mask == K_CONTROL_MASK
                && (matches!(
                    keycode,
                    XK_BACKSPACE | XK_DELETE | XK_LEFT | XK_RIGHT | XK_UP | XK_DOWN | XK_RETURN
                ) || (('0' as c_int)..=('9' as c_int)).contains(&keycode)
                    || (XK_KP_0..=XK_KP_9).contains(&keycode)))
                || (mask == K_SHIFT_MASK && keycode == XK_RETURN)
                || (mask == K_SHIFT_MASK && keycode == XK_BACKSPACE)
                || (mask == K_SHIFT_MASK && keycode == XK_DELETE)
                || (mask == K_SHIFT_MASK && keycode == XK_ESCAPE)
                || (mask == K_SHIFT_MASK
                    && matches!(keycode, XK_LEFT | XK_RIGHT | XK_UP | XK_DOWN))
                || (mask == K_SHIFT_MASK
                    && matches!(keycode, XK_KP_LEFT | XK_KP_RIGHT | XK_KP_UP | XK_KP_DOWN))
                || (mask == K_SHIFT_MASK && (XK_KP_0..=XK_KP_9).contains(&keycode))
                || (mask == K_SHIFT_MASK
                    && matches!(keycode, XK_HOME | XK_END | XK_KP_HOME | XK_KP_END))
                || (mask == K_SHIFT_MASK && (0x20..=0x7e).contains(&keycode))
                || (mask == (K_CONTROL_MASK | K_SHIFT_MASK)
                    && (keycode == XK_RETURN
                        || (('0' as c_int)..=('9' as c_int)).contains(&keycode)
                        || (XK_KP_0..=XK_KP_9).contains(&keycode)))))
    {
        return FALSE;
    }
    let Some(key_event) = key_event_from_rime_keycode(keycode, mask) else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };

    match process_ascii_composer_processor(session, key_event) {
        AsciiComposerProcessResult::Noop => {}
        AsciiComposerProcessResult::Accepted => {
            update_session_segment_tags(session);
            return TRUE;
        }
        AsciiComposerProcessResult::Rejected => return FALSE,
    }

    if let Some(commits) = process_key_binder_processor(session_id, session, key_event) {
        for commit in commits {
            append_unread_commit(session, commit);
        }
        return TRUE;
    }

    let was_composing = !session.engine.context().composition.input.is_empty();
    if !was_composing
        && (mask == K_CONTROL_MASK || mask == (K_CONTROL_MASK | K_SHIFT_MASK))
        && matches!(
            key_event.code,
            KeyCode::Character('0'..='9') | KeyCode::KeypadDigit('0'..='9')
        )
    {
        return FALSE;
    }
    match key_event.code {
        KeyCode::PreviousPage => {
            let page_size = session_menu_page_size(session);
            if session.engine.change_page_by(page_size, true) {
                session.paging = true;
            }
        }
        KeyCode::NextPage => {
            let page_size = session_menu_page_size(session);
            if session.engine.change_page_by(page_size, false) {
                session.paging = true;
            }
        }
        _ => {
            if let Some(commit) = process_session_key_event(session_id, session, key_event) {
                append_unread_commit(session, commit);
                return TRUE;
            }
        }
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
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    let Some(commit) = session.engine.commit_composition() else {
        return FALSE;
    };

    session.paging = false;
    update_session_segment_tags(session);
    append_unread_commit(session, commit);
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
    if let Some(session) = registry.get_session_mut(session_id) {
        session.engine.clear_composition();
        session.paging = false;
        update_session_segment_tags(session);
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
    let Some(session) = registry.get_session_mut(session_id) else {
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
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    session.engine.set_input(input);
    session.input_buffer = None;
    update_session_segment_tags(session);
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeGetCaretPos(session_id: RimeSessionId) -> usize {
    if session_id == 0 {
        return 0;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    registry
        .get_session_mut(session_id)
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
    if let Some(session) = registry.get_session_mut(session_id) {
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
        update_session_segment_tags(session);
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
/// point to writable storage of `buffer_size` bytes. Null pointers and empty
/// property values are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeGetProperty(
    session_id: RimeSessionId,
    property: *const c_char,
    value: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if property.is_null() || value.is_null() {
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
        copy_c_string_with_strncpy_semantics(property_value, value, buffer_size);
        true
    })
}

/// Copies the current session schema id into caller-provided storage.
///
/// # Safety
///
/// `schema_id` must point to writable storage of `buffer_size` bytes. Null
/// buffers are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCurrentSchema(
    session_id: RimeSessionId,
    schema_id: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if schema_id.is_null() {
        return FALSE;
    }

    with_session(session_id, |session| {
        let current_schema = session.engine.status().schema_id;
        copy_c_string_with_strncpy_semantics(&current_schema, schema_id, buffer_size);
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
        apply_schema_to_session(session, &schema_id);
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

fn apply_schema_to_session(session: &mut SessionState, schema_id: &str) {
    let schema_name = deployed_schema_name(schema_id);
    session.engine.set_schema(schema_id.to_owned(), schema_name);
    session.engine.reset_translators();
    session.engine.reset_filters();
    session.key_binder = None;
    session.ascii_composer_enabled = false;
    session.ascii_segmentor_enabled = false;
    session.punctuation_processor = None;
    session.recognizer_processor = None;
    session.paging = false;
    apply_schema_switch_resets(session, schema_id);
    install_schema_segment_tags(session, schema_id);
    install_schema_ascii_composer_processor(session, schema_id);
    install_schema_recognizer_processor(session, schema_id);
    install_schema_key_binder_processor(session, schema_id);
    install_schema_punctuation_processor(session, schema_id);
    install_schema_translator_chain(session, schema_id);
    install_schema_filter_chain(session, schema_id);
    session.engine.clear_composition();
    session.input_buffer = None;
    session.unread_commit = None;
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
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };

    for key_event in key_events {
        if let Some(commit) = process_session_key_event(session_id, session, key_event) {
            append_unread_commit(session, commit);
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
        let global_index = candidate_index_on_current_page(session, index)?;
        session.engine.select_candidate(global_index)
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
        let Some(global_index) = candidate_index_on_current_page(session, index) else {
            return false;
        };
        session.engine.delete_candidate(global_index)
    })
}

#[no_mangle]
pub extern "C" fn RimeHighlightCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    with_session(session_id, |session| {
        highlight_candidate_clamped_like_librime(session, index)
    })
}

#[no_mangle]
pub extern "C" fn RimeHighlightCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    with_session(session_id, |session| {
        let Some(global_index) = candidate_index_on_current_page(session, index) else {
            return false;
        };
        highlight_candidate_clamped_like_librime(session, global_index)
    })
}

#[no_mangle]
pub extern "C" fn RimeChangePage(session_id: RimeSessionId, backward: Bool) -> Bool {
    with_session(session_id, |session| {
        if session.engine.context().candidates.is_empty() {
            return false;
        }

        let page_size = session_menu_page_size(session);
        let current_index = session.engine.context().highlighted;
        let next_index = if backward != FALSE {
            current_index.saturating_sub(page_size)
        } else {
            current_index + page_size
        };
        let changed = highlight_candidate_clamped_like_librime(session, next_index);
        if changed {
            session.paging = true;
        }
        changed
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
    let Some(session) = registry.get_session_mut(session_id) else {
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

    let (snapshot, hide_candidate) = {
        let mut registry = sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let Some(session) = registry.get_session_mut(session_id) else {
            return FALSE;
        };
        (
            session.engine.snapshot(),
            session.engine.get_option("_hide_candidate"),
        )
    };
    let menu_settings = context_menu_settings(&snapshot.status.schema_id);
    let select_keys = match menu_settings.select_keys.as_deref() {
        Some(select_keys) => match CString::new(select_keys) {
            Ok(select_keys) => Some(select_keys),
            Err(_) => return FALSE,
        },
        None => None,
    };
    let composition = snapshot.context.composition;
    if !composition.input.is_empty() {
        let Ok(preedit) = CString::new(composition.preedit) else {
            return FALSE;
        };
        let commit_text_preview = if unsafe { context_has_commit_text_preview(context) } {
            let preview = snapshot
                .context
                .candidates
                .get(snapshot.context.highlighted)
                .map_or(composition.input.as_str(), |candidate| {
                    candidate.text.as_str()
                });
            match CString::new(preview) {
                Ok(preview) => Some(preview),
                Err(_) => return FALSE,
            }
        } else {
            None
        };
        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `preedit` is converted into owned C storage for the caller.
        unsafe {
            (*context).composition.length = composition.input.len() as c_int;
            (*context).composition.cursor_pos = composition.caret as c_int;
            (*context).composition.sel_start = 0;
            (*context).composition.sel_end = composition.input.len() as c_int;
            (*context).composition.preedit = preedit.into_raw();
            if let Some(commit_text_preview) = commit_text_preview {
                (*context).commit_text_preview = commit_text_preview.into_raw();
            }
        }
    }

    let candidates = snapshot.context.candidates;
    if !candidates.is_empty() {
        let highlighted = snapshot.context.highlighted;
        let page_size = menu_settings.page_size;
        let page_no = highlighted / page_size;
        let page_start = page_no * page_size;
        let page_end = (page_start + page_size).min(candidates.len());
        let page_candidates = &candidates[page_start..page_end];

        if hide_candidate {
            // SAFETY: `context` is non-null and points to caller-owned writable
            // storage. librime still exposes menu metadata while hiding entries.
            unsafe {
                (*context).menu.page_size =
                    c_int::try_from(page_size).expect("menu page size should fit in c_int");
                (*context).menu.page_no = page_no as c_int;
                (*context).menu.is_last_page = bool_from(page_end == candidates.len());
                (*context).menu.highlighted_candidate_index = (highlighted - page_start)
                    .min(page_candidates.len().saturating_sub(1))
                    as c_int;
                (*context).menu.num_candidates = 0;
            }
            return TRUE;
        }

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

        let select_labels = if unsafe { context_has_select_labels(context) }
            && menu_settings.select_labels.len() >= page_size
        {
            let mut labels = Vec::with_capacity(page_size);
            for label in menu_settings.select_labels.iter().take(page_size) {
                let Ok(label) = CString::new(label.as_str()) else {
                    free_rime_candidates(&mut rime_candidates);
                    return FALSE;
                };
                labels.push(label);
            }
            let mut labels = labels
                .into_iter()
                .map(CString::into_raw)
                .collect::<Vec<_>>();
            let labels_ptr = labels.as_mut_ptr();
            std::mem::forget(labels);
            Some(labels_ptr)
        } else {
            None
        };
        let num_candidates = rime_candidates.len();
        let candidates_ptr = rime_candidates.as_mut_ptr();
        std::mem::forget(rime_candidates);

        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `candidates_ptr` owns `num_candidates` initialized entries.
        unsafe {
            (*context).menu.page_size =
                c_int::try_from(page_size).expect("menu page size should fit in c_int");
            (*context).menu.page_no = page_no as c_int;
            (*context).menu.is_last_page = bool_from(page_end == candidates.len());
            (*context).menu.highlighted_candidate_index =
                (highlighted - page_start).min(num_candidates.saturating_sub(1)) as c_int;
            (*context).menu.num_candidates = num_candidates as c_int;
            (*context).menu.candidates = candidates_ptr;
            if let Some(select_keys) = select_keys {
                (*context).menu.select_keys = select_keys.into_raw();
            }
            if let Some(select_labels) = select_labels {
                (*context).select_labels = select_labels;
            }
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

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
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

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
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

    // SAFETY: `iterator` is non-null and points to caller-owned storage.
    let next_index = unsafe { (*iterator).index.saturating_add(1) };
    if next_index < 0 {
        // SAFETY: librime still advances the iterator index on failed lookup.
        unsafe {
            (*iterator).index = next_index;
        }
        return FALSE;
    }

    let candidate_index = next_index as usize;
    let Some(candidate) = state.candidates.get(candidate_index) else {
        // SAFETY: librime leaves the current candidate intact when advancing
        // past the end but still exposes the advanced iterator index.
        unsafe {
            (*iterator).index = next_index;
        }
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
    // storage; existing strings were allocated by this API during an earlier
    // successful `Next`, and new strings are owned until next/end.
    unsafe {
        free_candidate_fields(&mut (*iterator).candidate);
        (*iterator).index = next_index;
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

fn key_event_from_rime_keycode(keycode: c_int, mask: c_int) -> Option<KeyEvent> {
    let code = match keycode {
        XK_BACKSPACE => KeyCode::Backspace,
        XK_DELETE => KeyCode::Delete,
        XK_ESCAPE => KeyCode::Escape,
        XK_LEFT => KeyCode::MoveCaretLeft,
        XK_RIGHT => KeyCode::MoveCaretRight,
        XK_UP if mask == K_CONTROL_MASK || mask == K_SHIFT_MASK => KeyCode::MoveCaretLeftBySyllable,
        XK_DOWN if mask == K_CONTROL_MASK || mask == K_SHIFT_MASK => {
            KeyCode::MoveCaretRightBySyllable
        }
        XK_KP_LEFT => KeyCode::MoveCaretLeftByChar,
        XK_KP_RIGHT => KeyCode::MoveCaretRightByChar,
        XK_UP | XK_KP_UP => KeyCode::PreviousCandidate,
        XK_DOWN | XK_KP_DOWN => KeyCode::NextCandidate,
        XK_HOME | XK_KP_HOME => KeyCode::Home,
        XK_END | XK_KP_END => KeyCode::End,
        XK_PAGE_UP | XK_KP_PAGE_UP => KeyCode::PreviousPage,
        XK_PAGE_DOWN | XK_KP_PAGE_DOWN => KeyCode::NextPage,
        XK_RETURN => KeyCode::Return,
        XK_KP_ENTER => KeyCode::KeypadEnter,
        XK_KP_0..=XK_KP_9 => {
            KeyCode::KeypadDigit(char::from_u32(('0' as u32) + (keycode - XK_KP_0) as u32)?)
        }
        0x20..=0x7e => KeyCode::Character(char::from_u32(keycode as u32)?),
        _ => return None,
    };
    let modifiers = match mask {
        0 => KeyModifiers::default(),
        K_SHIFT_MASK => KeyModifiers {
            shift: true,
            ..KeyModifiers::default()
        },
        K_CONTROL_MASK => KeyModifiers {
            control: true,
            ..KeyModifiers::default()
        },
        combined if combined == (K_CONTROL_MASK | K_SHIFT_MASK) => KeyModifiers {
            control: true,
            shift: true,
            ..KeyModifiers::default()
        },
        _ => return None,
    };

    Some(KeyEvent { code, modifiers })
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
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    let Some(commit) = select(session) else {
        return FALSE;
    };

    session.paging = false;
    update_session_segment_tags(session);
    append_unread_commit(session, commit);
    TRUE
}

fn append_unread_commit(session: &mut SessionState, commit: String) {
    match &mut session.unread_commit {
        Some(buffer) => buffer.push_str(&commit),
        None => session.unread_commit = Some(commit),
    }
}

fn session_menu_page_size(session: &SessionState) -> usize {
    context_menu_settings(&session.engine.status().schema_id).page_size
}

fn install_schema_translator_chain(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(translators)) =
        find_config_value(&schema_config, "engine/translators")
    else {
        return;
    };
    let mut punctuation_translator_installed = false;

    for translator in translators.iter().filter_map(Value::as_str) {
        let (component_name, name_space) = schema_component_prescription(translator);
        match component_name {
            "punct_translator" => {
                if !punctuation_translator_installed {
                    install_schema_punctuation_translator_from_config(session, &schema_config);
                    punctuation_translator_installed = true;
                }
            }
            "table_translator" | "script_translator" | "r10n_translator" => {
                install_schema_dictionary_translator_from_config(
                    session,
                    &schema_config,
                    component_name,
                    name_space.unwrap_or("translator"),
                );
            }
            "reverse_lookup_translator" => install_schema_reverse_lookup_translator_from_config(
                session,
                &schema_config,
                match name_space {
                    Some("translator") | None => "reverse_lookup",
                    Some(name_space) => name_space,
                },
            ),
            "history_translator" => install_schema_history_translator_from_config(
                session,
                &schema_config,
                match name_space {
                    Some("translator") | None => "history",
                    Some(name_space) => name_space,
                },
            ),
            _ => {}
        }
    }
}

fn schema_component_prescription(component: &str) -> (&str, Option<&str>) {
    let Some((component_name, name_space)) = component.split_once('@') else {
        return (component, None);
    };
    if component_name.is_empty() || name_space.is_empty() {
        (component, None)
    } else {
        (component_name, Some(name_space))
    }
}

fn install_schema_dictionary_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    component_name: &str,
    name_space: &str,
) {
    let Some(dictionary) = load_schema_table_dictionary(schema_config, name_space) else {
        return;
    };
    let enable_charset_filter = find_config_value(
        schema_config,
        &format!("{name_space}/enable_charset_filter"),
    )
    .and_then(config_scalar_bool)
    .unwrap_or(false);
    let enable_sentence =
        find_config_value(schema_config, &format!("{name_space}/enable_sentence"))
            .and_then(config_scalar_bool)
            .unwrap_or(true);
    let sentence_over_completion = find_config_value(
        schema_config,
        &format!("{name_space}/sentence_over_completion"),
    )
    .and_then(config_scalar_bool)
    .unwrap_or(false);
    let mut enable_completion =
        find_config_value(schema_config, &format!("{name_space}/enable_completion"))
            .and_then(config_scalar_bool)
            .unwrap_or(true);
    if matches!(component_name, "script_translator" | "r10n_translator") {
        if let Some(enable_word_completion) = find_config_value(
            schema_config,
            &format!("{name_space}/enable_word_completion"),
        )
        .and_then(config_scalar_bool)
        {
            enable_completion = enable_word_completion;
        }
    }
    let delimiters = find_config_value(schema_config, &format!("{name_space}/delimiter"))
        .or_else(|| find_config_value(schema_config, "speller/delimiter"))
        .and_then(config_scalar_string)
        .unwrap_or_else(|| " ".to_owned());
    let tags = schema_translator_tags(schema_config, name_space);
    let initial_quality =
        find_config_value(schema_config, &format!("{name_space}/initial_quality"))
            .and_then(config_scalar_f32)
            .unwrap_or(0.0);
    let comment_format = schema_comment_format(schema_config, name_space);
    let dictionary_exclude =
        schema_string_list(schema_config, &format!("{name_space}/dictionary_exclude"));
    session.engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_completion(enable_completion)
            .with_charset_filter(enable_charset_filter)
            .with_sentence(enable_sentence)
            .with_sentence_over_completion(sentence_over_completion)
            .with_delimiters(delimiters)
            .with_tags(tags)
            .with_initial_quality(initial_quality)
            .with_comment_format(&comment_format)
            .with_dictionary_exclude(dictionary_exclude),
    );
}

fn install_schema_reverse_lookup_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let Some(dictionary) = load_schema_table_dictionary(schema_config, name_space) else {
        return;
    };
    let target_namespace = find_config_value(schema_config, &format!("{name_space}/target"))
        .and_then(config_scalar_string)
        .filter(|target| !target.is_empty())
        .unwrap_or_else(|| "translator".to_owned());
    let reverse_dictionary = load_schema_table_dictionary(schema_config, &target_namespace);
    let prefix = find_config_value(schema_config, &format!("{name_space}/prefix"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let suffix = find_config_value(schema_config, &format!("{name_space}/suffix"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let tag = find_config_value(schema_config, &format!("{name_space}/tag"))
        .and_then(config_scalar_string)
        .unwrap_or_else(|| "reverse_lookup".to_owned());
    let enable_completion =
        find_config_value(schema_config, &format!("{name_space}/enable_completion"))
            .and_then(config_scalar_bool)
            .unwrap_or(false);
    let comment_format = schema_comment_format(schema_config, name_space);

    session.engine.add_translator(
        ReverseLookupTranslator::new(dictionary, reverse_dictionary, prefix, suffix)
            .with_tag(tag)
            .with_completion(enable_completion)
            .with_comment_format(&comment_format),
    );
}

fn install_schema_history_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let input = find_config_value(schema_config, &format!("{name_space}/input"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let size = find_config_value(schema_config, &format!("{name_space}/size"))
        .and_then(config_scalar_int)
        .and_then(|size| usize::try_from(size).ok())
        .unwrap_or(1);
    let initial_quality =
        find_config_value(schema_config, &format!("{name_space}/initial_quality"))
            .and_then(config_scalar_double)
            .map(|quality| quality as f32)
            .unwrap_or(1000.0);
    let tag = find_config_value(schema_config, &format!("{name_space}/tag"))
        .and_then(config_scalar_string)
        .unwrap_or_else(|| "abc".to_owned());

    session.engine.add_translator(
        HistoryTranslator::new(input)
            .with_size(size)
            .with_initial_quality(initial_quality)
            .with_tag(tag),
    );
}

fn install_schema_filter_chain(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(filters)) = find_config_value(&schema_config, "engine/filters") else {
        return;
    };
    for filter in filters.iter().filter_map(Value::as_str) {
        let (filter_name, name_space) = schema_component_prescription(filter);
        match filter_name {
            "reverse_lookup_filter" => install_schema_reverse_lookup_filter_from_config(
                session,
                &schema_config,
                match name_space {
                    Some("filter") | None => "reverse_lookup",
                    Some(name_space) => name_space,
                },
            ),
            "simplifier" => install_schema_simplifier_filter_from_config(
                session,
                &schema_config,
                match name_space {
                    Some("filter") | None => "simplifier",
                    Some(name_space) => name_space,
                },
            ),
            "uniquifier" => session.engine.add_filter(UniquifierFilter),
            "single_char_filter" => session.engine.add_filter(SingleCharFilter),
            "charset_filter" | "cjk_minifier" => {
                let tags = schema_filter_tags(&schema_config, name_space.unwrap_or(filter_name));
                session
                    .engine
                    .add_filter(TaggedFilter::new(CharsetFilter, tags));
            }
            _ => {}
        }
    }
}

fn schema_filter_tags(schema_config: &Value, name_space: &str) -> Vec<String> {
    schema_string_list(schema_config, &format!("{name_space}/tags"))
}

fn install_schema_reverse_lookup_filter_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let Some(reverse_dictionary) = load_schema_table_dictionary(schema_config, name_space) else {
        return;
    };

    let overwrite_comment =
        find_config_value(schema_config, &format!("{name_space}/overwrite_comment"))
            .and_then(config_scalar_bool)
            .unwrap_or(false);
    let append_comment = find_config_value(schema_config, &format!("{name_space}/append_comment"))
        .and_then(config_scalar_bool)
        .unwrap_or(false);
    let comment_format = schema_comment_format(schema_config, name_space);

    let tags = schema_filter_tags(schema_config, name_space);
    session.engine.add_filter(TaggedFilter::new(
        ReverseLookupFilter::new(reverse_dictionary)
            .with_overwrite_comment(overwrite_comment)
            .with_append_comment(append_comment)
            .with_comment_format(&comment_format),
        tags,
    ));
}

fn install_schema_simplifier_filter_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let option_name = find_config_value(schema_config, &format!("{name_space}/option_name"))
        .and_then(config_scalar_string)
        .filter(|option_name| !option_name.is_empty())
        .unwrap_or_else(|| "simplification".to_owned());
    let tips = find_config_value(schema_config, &format!("{name_space}/tips"))
        .or_else(|| find_config_value(schema_config, &format!("{name_space}/tip")))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let opencc_config = find_config_value(schema_config, &format!("{name_space}/opencc_config"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let show_in_comment =
        find_config_value(schema_config, &format!("{name_space}/show_in_comment"))
            .and_then(config_scalar_bool)
            .unwrap_or(false);
    let inherit_comment =
        find_config_value(schema_config, &format!("{name_space}/inherit_comment"))
            .and_then(config_scalar_bool)
            .unwrap_or(true);
    let comment_format = schema_comment_format(schema_config, name_space);
    let excluded_types = schema_string_list(schema_config, &format!("{name_space}/excluded_types"));

    let tags = schema_filter_tags(schema_config, name_space);
    session.engine.add_filter(TaggedFilter::new(
        SimplifierFilter::new()
            .with_option_name(option_name)
            .with_opencc_config(opencc_config)
            .with_tips(tips)
            .with_show_in_comment(show_in_comment)
            .with_inherit_comment(inherit_comment)
            .with_comment_format(&comment_format)
            .with_excluded_types(excluded_types),
        tags,
    ));
}

fn load_schema_table_dictionary(
    schema_config: &Value,
    name_space: &str,
) -> Option<TableDictionary> {
    let dictionary_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
        .and_then(config_scalar_string)
        .filter(|dictionary_name| !dictionary_name.is_empty())?;
    let dictionary_path = selected_runtime_data_path(&format!("{dictionary_name}.dict.yaml"))?;
    let dictionary_yaml = fs::read_to_string(dictionary_path).ok()?;
    let packs = schema_dictionary_packs(schema_config, name_space);
    TableDictionary::parse_rime_dict_yaml_with_imports_and_packs(
        &dictionary_yaml,
        packs,
        |import_table| {
            selected_runtime_data_path(&format!("{import_table}.dict.yaml"))
                .and_then(|path| fs::read_to_string(path).ok())
        },
    )
    .ok()
}

fn schema_dictionary_packs(schema_config: &Value, name_space: &str) -> Vec<String> {
    let Some(Value::Sequence(packs)) =
        find_config_value(schema_config, &format!("{name_space}/packs"))
    else {
        return Vec::new();
    };
    packs.iter().filter_map(config_scalar_string).collect()
}

fn schema_comment_format(schema_config: &Value, name_space: &str) -> Vec<String> {
    schema_string_list(schema_config, &format!("{name_space}/comment_format"))
}

fn schema_translator_tags(schema_config: &Value, name_space: &str) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(tag) = find_config_value(schema_config, &format!("{name_space}/tag"))
        .and_then(config_scalar_string)
    {
        tags.push(tag);
    }
    tags.extend(schema_string_list(
        schema_config,
        &format!("{name_space}/tags"),
    ));
    if tags.is_empty() {
        tags.push("abc".to_owned());
    }
    tags
}

fn schema_string_list(schema_config: &Value, key: &str) -> Vec<String> {
    let Some(Value::Sequence(formulas)) = find_config_value(schema_config, key) else {
        return Vec::new();
    };
    formulas.iter().filter_map(config_scalar_string).collect()
}

fn config_scalar_f32(value: &Value) -> Option<f32> {
    config_scalar_double(value).map(|number| number as f32)
}

fn install_schema_segment_tags(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let mut tags = vec!["abc".to_owned()];
    session.affix_segmentors.clear();
    session.matcher_segmentor = None;
    session.ascii_segmentor_enabled = false;

    if let Some(Value::Sequence(segmentors)) =
        find_config_value(&schema_config, "engine/segmentors")
    {
        session.ascii_segmentor_enabled = segmentors
            .iter()
            .filter_map(Value::as_str)
            .map(schema_component_prescription)
            .any(|(component_name, _)| component_name == "ascii_segmentor");
        if segmentors
            .iter()
            .filter_map(Value::as_str)
            .map(schema_component_prescription)
            .any(|(component_name, _)| component_name == "abc_segmentor")
        {
            tags.extend(schema_string_list(
                &schema_config,
                "abc_segmentor/extra_tags",
            ));
        }
        session.affix_segmentors = load_schema_affix_segmentors(&schema_config, segmentors);
        session.matcher_segmentor = load_schema_matcher_segmentor(&schema_config, segmentors);
    }
    session.base_segment_tags = tags;
    update_session_segment_tags(session);
}

fn install_schema_ascii_composer_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(processors)) = find_config_value(&schema_config, "engine/processors")
    else {
        return;
    };
    session.ascii_composer_enabled = processors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .any(|(component_name, _)| component_name == "ascii_composer");
}

fn install_schema_recognizer_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(processors)) = find_config_value(&schema_config, "engine/processors")
    else {
        return;
    };
    let Some(name_space) = processors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .find_map(|(component_name, name_space)| {
            (component_name == "recognizer")
                .then(|| {
                    let name_space = name_space.unwrap_or("recognizer");
                    if name_space == "processor" {
                        "recognizer"
                    } else {
                        name_space
                    }
                })
                .filter(|name_space| !name_space.is_empty())
        })
    else {
        return;
    };
    let patterns = load_schema_recognizer_patterns(&schema_config, name_space);
    if patterns.is_empty() {
        return;
    }
    let use_space = find_config_value(&schema_config, &format!("{name_space}/use_space"))
        .and_then(config_scalar_bool)
        .unwrap_or(false);
    session.recognizer_processor = Some(RecognizerProcessor {
        use_space,
        patterns,
    });
}

fn load_schema_matcher_segmentor(
    schema_config: &Value,
    segmentors: &[Value],
) -> Option<MatcherSegmentor> {
    let name_space = segmentors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .find_map(|(component_name, name_space)| {
            (component_name == "matcher")
                .then(|| {
                    let name_space = name_space.unwrap_or("recognizer");
                    if name_space == "segmentor" {
                        "recognizer"
                    } else {
                        name_space
                    }
                })
                .filter(|name_space| !name_space.is_empty())
        })?;
    let patterns = load_schema_recognizer_patterns(schema_config, name_space);
    (!patterns.is_empty()).then_some(MatcherSegmentor { patterns })
}

fn load_schema_affix_segmentors(
    schema_config: &Value,
    segmentors: &[Value],
) -> Vec<AffixSegmentor> {
    segmentors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .filter_map(|(component_name, name_space)| {
            if component_name != "affix_segmentor" {
                return None;
            }
            let name_space = name_space.unwrap_or("segmentor");
            if name_space.is_empty() {
                return None;
            }
            let prefix = find_config_value(schema_config, &format!("{name_space}/prefix"))
                .and_then(config_scalar_string)
                .unwrap_or_default();
            if prefix.is_empty() {
                return None;
            }
            let tag = find_config_value(schema_config, &format!("{name_space}/tag"))
                .and_then(config_scalar_string)
                .filter(|tag| !tag.is_empty())
                .unwrap_or_else(|| "abc".to_owned());
            let suffix = find_config_value(schema_config, &format!("{name_space}/suffix"))
                .and_then(config_scalar_string)
                .unwrap_or_default();
            let extra_tags = schema_string_list(schema_config, &format!("{name_space}/extra_tags"));
            Some(AffixSegmentor {
                tag,
                prefix,
                suffix,
                extra_tags,
            })
        })
        .collect()
}

fn load_schema_recognizer_patterns(schema_config: &Value, name_space: &str) -> Vec<MatcherPattern> {
    let Some(Value::Mapping(patterns)) =
        find_config_value(schema_config, &format!("{name_space}/patterns"))
    else {
        return Vec::new();
    };
    let mut patterns = patterns
        .iter()
        .filter_map(|(tag, pattern)| {
            let tag = config_scalar_string(tag)?;
            let pattern = config_scalar_string(pattern)?;
            Regex::new(&pattern)
                .ok()
                .map(|pattern| MatcherPattern { tag, pattern })
        })
        .collect::<Vec<_>>();
    patterns.sort_by(|left, right| left.tag.cmp(&right.tag));
    patterns
}

fn update_session_segment_tags(session: &mut SessionState) {
    let input = session.engine.context().composition.input.as_str();
    if session.ascii_segmentor_enabled && session.engine.status().is_ascii_mode && !input.is_empty()
    {
        let raw_tags = vec!["raw".to_owned()];
        if session.engine.context().segment_tags != raw_tags {
            session.engine.set_segment_tags(raw_tags);
        }
        return;
    }
    let mut tags = session.base_segment_tags.clone();
    for affix_segmentor in &session.affix_segmentors {
        if affix_segmentor.matches(input) {
            let mut affix_tags = vec![affix_segmentor.tag.clone()];
            for extra_tag in &affix_segmentor.extra_tags {
                if !affix_tags.iter().any(|existing| existing == extra_tag) {
                    affix_tags.push(extra_tag.clone());
                }
            }
            if session.engine.context().segment_tags != affix_tags {
                session.engine.set_segment_tags(affix_tags);
            }
            return;
        }
    }
    if let Some(matcher) = &session.matcher_segmentor {
        if let Some(tag) = matcher.match_tag(input) {
            if !tags.iter().any(|existing| existing == tag) {
                tags.push(tag.to_owned());
            }
        }
    }
    if session.engine.context().segment_tags != tags {
        session.engine.set_segment_tags(tags);
    }
}

impl AffixSegmentor {
    fn matches(&self, input: &str) -> bool {
        let Some(mut code) = input.strip_prefix(&self.prefix) else {
            return false;
        };
        if code.is_empty() {
            return false;
        }
        if !self.suffix.is_empty() {
            code = code.strip_suffix(&self.suffix).unwrap_or(code);
        }
        !code.is_empty()
    }
}

impl MatcherSegmentor {
    fn match_tag(&self, input: &str) -> Option<&str> {
        if input.is_empty() {
            return None;
        }
        self.patterns
            .iter()
            .find(|pattern| recognizer_pattern_matches(pattern, input))
            .map(|pattern| pattern.tag.as_str())
    }
}

fn recognizer_patterns_match(patterns: &[MatcherPattern], input: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| recognizer_pattern_matches(pattern, input))
}

fn recognizer_pattern_matches(pattern: &MatcherPattern, input: &str) -> bool {
    pattern
        .pattern
        .find(input)
        .is_some_and(|matched| matched.start() == 0 && matched.end() == input.len())
}

fn install_schema_punctuation_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
) {
    let half_shape_entries = punctuation_entries_from_config(schema_config, "half_shape");
    let full_shape_entries = punctuation_entries_from_config(schema_config, "full_shape");
    let symbol_entries = punctuation_entries_from_config(schema_config, "symbols");
    if half_shape_entries.is_empty() && full_shape_entries.is_empty() && symbol_entries.is_empty() {
        return;
    }
    session
        .engine
        .add_translator(PunctuationTranslator::with_shape_and_symbol_entries(
            half_shape_entries,
            full_shape_entries,
            symbol_entries,
        ));
}

fn install_schema_key_binder_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    if !schema_engine_processors_include(&schema_config, "key_binder") {
        return;
    }
    let Some(Value::Sequence(bindings)) = find_config_value(&schema_config, "key_binder/bindings")
    else {
        return;
    };

    let mut processor = KeyBinderProcessor {
        bindings: HashMap::new(),
        redirecting: false,
        last_key: None,
    };
    for binding in bindings {
        let Value::Mapping(binding) = binding else {
            continue;
        };
        let Some(condition) = binding
            .get(Value::String("when".to_owned()))
            .and_then(config_scalar_string)
            .and_then(|condition| key_binding_condition(&condition))
        else {
            continue;
        };
        let Some(accept) = binding
            .get(Value::String("accept".to_owned()))
            .and_then(config_scalar_string)
        else {
            continue;
        };
        let Some(key_event) = parse_single_key_binding_event(&accept) else {
            continue;
        };
        let action = if let Some(send) = binding
            .get(Value::String("send".to_owned()))
            .and_then(config_scalar_string)
        {
            let Some(target) = parse_single_key_binding_event(&send) else {
                continue;
            };
            KeyBindingAction::Send(vec![target])
        } else if let Some(send_sequence) = binding
            .get(Value::String("send_sequence".to_owned()))
            .and_then(config_scalar_string)
        {
            let Ok(targets) = parse_key_sequence(&send_sequence) else {
                continue;
            };
            KeyBindingAction::Send(targets)
        } else if let Some(toggle) = binding
            .get(Value::String("toggle".to_owned()))
            .and_then(config_scalar_string)
        {
            KeyBindingAction::Toggle(toggle)
        } else if let Some(option) = binding
            .get(Value::String("set_option".to_owned()))
            .and_then(config_scalar_string)
        {
            KeyBindingAction::SetOption {
                option,
                value: true,
            }
        } else if let Some(option) = binding
            .get(Value::String("unset_option".to_owned()))
            .and_then(config_scalar_string)
        {
            KeyBindingAction::SetOption {
                option,
                value: false,
            }
        } else if let Some(schema) = binding
            .get(Value::String("select".to_owned()))
            .and_then(config_scalar_string)
        {
            KeyBindingAction::SelectSchema(schema)
        } else {
            continue;
        };
        insert_key_binding(
            processor.bindings.entry(key_event).or_default(),
            KeyBinding { condition, action },
        );
    }

    if !processor.bindings.is_empty() {
        session.key_binder = Some(processor);
    }
}

fn insert_key_binding(bindings: &mut Vec<KeyBinding>, binding: KeyBinding) {
    let rank = key_binding_condition_rank(binding.condition);
    let insertion_index = bindings
        .iter()
        .position(|existing| key_binding_condition_rank(existing.condition) >= rank)
        .unwrap_or(bindings.len());
    bindings.insert(insertion_index, binding);
}

fn key_binding_condition(condition: &str) -> Option<KeyBindingCondition> {
    match condition {
        "always" => Some(KeyBindingCondition::Always),
        "composing" => Some(KeyBindingCondition::Composing),
        "has_menu" => Some(KeyBindingCondition::HasMenu),
        "paging" => Some(KeyBindingCondition::Paging),
        _ => None,
    }
}

fn key_binding_condition_rank(condition: KeyBindingCondition) -> usize {
    match condition {
        KeyBindingCondition::Paging => 1,
        KeyBindingCondition::HasMenu => 2,
        KeyBindingCondition::Composing => 3,
        KeyBindingCondition::Always => 4,
    }
}

fn parse_single_key_binding_event(pattern: &str) -> Option<KeyEvent> {
    let sequence = if pattern.chars().count() == 1 {
        pattern.to_owned()
    } else {
        format!("{{{pattern}}}")
    };
    let mut events = parse_key_sequence(&sequence).ok()?;
    (events.len() == 1).then(|| events.remove(0))
}

fn install_schema_punctuation_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    if !schema_engine_processors_include(&schema_config, "punctuator")
        || !schema_engine_translators_include(&schema_config, "punct_translator")
    {
        return;
    }

    let processor = PunctuationProcessor {
        use_space: find_config_value(&schema_config, "punctuator/use_space")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
        digit_separators: find_config_value(&schema_config, "punctuator/digit_separators")
            .and_then(config_scalar_string)
            .unwrap_or_else(|| ".:".to_owned()),
        digit_separator_commit: find_config_value(
            &schema_config,
            "punctuator/digit_separator_action",
        )
        .and_then(config_scalar_string)
        .is_some_and(|action| action == "commit"),
        half_shape_alternating_counts: punctuation_alternating_counts_from_config(
            &schema_config,
            "half_shape",
        ),
        full_shape_alternating_counts: punctuation_alternating_counts_from_config(
            &schema_config,
            "full_shape",
        ),
        symbol_alternating_counts: punctuation_alternating_counts_from_config(
            &schema_config,
            "symbols",
        ),
        half_shape_unique_commits: punctuation_unique_commits_from_config(
            &schema_config,
            "half_shape",
        ),
        full_shape_unique_commits: punctuation_unique_commits_from_config(
            &schema_config,
            "full_shape",
        ),
        symbol_unique_commits: punctuation_unique_commits_from_config(&schema_config, "symbols"),
        half_shape_pairs: punctuation_pairs_from_config(&schema_config, "half_shape"),
        full_shape_pairs: punctuation_pairs_from_config(&schema_config, "full_shape"),
        symbol_pairs: punctuation_pairs_from_config(&schema_config, "symbols"),
        pair_oddness: HashMap::new(),
        pending_digit_separator: None,
    };
    if processor.half_shape_unique_commits.is_empty()
        && processor.full_shape_unique_commits.is_empty()
        && processor.symbol_unique_commits.is_empty()
        && processor.half_shape_alternating_counts.is_empty()
        && processor.full_shape_alternating_counts.is_empty()
        && processor.symbol_alternating_counts.is_empty()
        && processor.half_shape_pairs.is_empty()
        && processor.full_shape_pairs.is_empty()
        && processor.symbol_pairs.is_empty()
    {
        return;
    }
    session.punctuation_processor = Some(processor);
}

fn schema_engine_translators_include(schema_config: &Value, translator_name: &str) -> bool {
    !schema_engine_translator_namespaces(schema_config, translator_name, translator_name).is_empty()
}

fn schema_engine_translator_namespaces(
    schema_config: &Value,
    translator_name: &str,
    default_name_space: &str,
) -> Vec<String> {
    let Some(Value::Sequence(translators)) = find_config_value(schema_config, "engine/translators")
    else {
        return Vec::new();
    };
    schema_engine_component_namespaces(translators, translator_name, default_name_space)
}

fn schema_engine_component_namespaces(
    components: &[Value],
    component_name: &str,
    default_name_space: &str,
) -> Vec<String> {
    let component_prefix = format!("{component_name}@");
    components
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|component| {
            if component == component_name {
                Some(default_name_space.to_owned())
            } else {
                component
                    .strip_prefix(&component_prefix)
                    .filter(|name_space| !name_space.is_empty())
                    .map(ToOwned::to_owned)
            }
        })
        .collect()
}

fn schema_engine_processors_include(schema_config: &Value, processor_name: &str) -> bool {
    let Some(Value::Sequence(processors)) = find_config_value(schema_config, "engine/processors")
    else {
        return false;
    };
    processors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .any(|(component_name, _)| component_name == processor_name)
}

fn apply_schema_switch_resets(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(switches)) = find_config_value(&schema_config, "switches") else {
        return;
    };

    for the_switch in switches {
        let Value::Mapping(switch_map) = the_switch else {
            continue;
        };
        let Some(reset_value) = switch_reset_value(switch_map) else {
            continue;
        };

        if let Some(option_name) = switch_scalar_field(switch_map, "name") {
            session.engine.set_option(option_name, reset_value != 0);
            continue;
        }

        let Some(Value::Sequence(options)) = switch_map.get(Value::String("options".to_owned()))
        else {
            continue;
        };
        for (option_index, option) in options.iter().enumerate() {
            let Some(option_name) = config_scalar_string(option) else {
                continue;
            };
            session
                .engine
                .set_option(option_name, option_index as c_int == reset_value);
        }
    }
}

fn switch_reset_value(switch_map: &Mapping) -> Option<c_int> {
    let reset = switch_map.get(Value::String("reset".to_owned()))?;
    match reset {
        Value::Null | Value::Sequence(_) | Value::Mapping(_) => None,
        scalar => Some(config_scalar_int(scalar).unwrap_or(0)),
    }
}

fn switch_scalar_field(switch_map: &Mapping, key: &str) -> Option<String> {
    switch_map
        .get(Value::String(key.to_owned()))
        .and_then(config_scalar_string)
}

fn punctuation_entries_from_config(schema_config: &Value, shape: &str) -> Vec<(String, String)> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    for (key, definition) in mapping {
        let Some(key) = config_scalar_string(key) else {
            continue;
        };
        append_punctuation_definition(&mut entries, &key, definition);
    }
    entries
}

fn punctuation_unique_commits_from_config(
    schema_config: &Value,
    shape: &str,
) -> HashMap<String, String> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return HashMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, definition)| {
            let key = config_scalar_string(key)?;
            let text = punctuation_unique_commit(definition)?;
            Some((key, text))
        })
        .collect()
}

fn punctuation_alternating_counts_from_config(
    schema_config: &Value,
    shape: &str,
) -> HashMap<String, usize> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return HashMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, definition)| {
            let key = config_scalar_string(key)?;
            let Value::Sequence(values) = definition else {
                return None;
            };
            let count = values.iter().filter_map(config_scalar_string).count();
            (count > 0).then_some((key, count))
        })
        .collect()
}

fn punctuation_pairs_from_config(
    schema_config: &Value,
    shape: &str,
) -> HashMap<String, [String; 2]> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return HashMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, definition)| {
            let key = config_scalar_string(key)?;
            let pair = punctuation_pair(definition)?;
            Some((key, pair))
        })
        .collect()
}

fn punctuation_unique_commit(definition: &Value) -> Option<String> {
    if let Some(text) = config_scalar_string(definition) {
        return Some(text);
    }
    let Value::Mapping(mapping) = definition else {
        return None;
    };
    mapping
        .get(Value::String("commit".to_owned()))
        .and_then(config_scalar_string)
}

fn punctuation_pair(definition: &Value) -> Option<[String; 2]> {
    let Value::Mapping(mapping) = definition else {
        return None;
    };
    let Some(Value::Sequence(pair)) = mapping.get(Value::String("pair".to_owned())) else {
        return None;
    };
    if pair.len() != 2 {
        return None;
    }
    let first = config_scalar_string(&pair[0])?;
    let second = config_scalar_string(&pair[1])?;
    Some([first, second])
}

fn append_punctuation_definition(
    entries: &mut Vec<(String, String)>,
    key: &str,
    definition: &Value,
) {
    if let Some(text) = config_scalar_string(definition) {
        entries.push((key.to_owned(), text));
        return;
    }

    match definition {
        Value::Sequence(values) => {
            for value in values {
                if let Some(text) = config_scalar_string(value) {
                    entries.push((key.to_owned(), text));
                }
            }
        }
        Value::Mapping(mapping) => {
            let commit_key = Value::String("commit".to_owned());
            if let Some(text) = mapping.get(&commit_key).and_then(config_scalar_string) {
                entries.push((key.to_owned(), text));
                return;
            }

            let pair_key = Value::String("pair".to_owned());
            let Some(Value::Sequence(pair)) = mapping.get(&pair_key) else {
                return;
            };
            if pair.len() != 2 {
                return;
            }
            for value in pair {
                if let Some(text) = config_scalar_string(value) {
                    entries.push((key.to_owned(), text));
                }
            }
        }
        _ => {}
    }
}

fn process_session_key_event(
    session_id: RimeSessionId,
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<String> {
    if let Some(commits) = process_key_binder_processor(session_id, session, key_event) {
        update_session_segment_tags(session);
        return (!commits.is_empty()).then(|| commits.concat());
    }
    if process_recognizer_processor(session, key_event) {
        update_session_segment_tags(session);
        return None;
    }
    if let Some(result) = process_punctuation_processor(session, key_event) {
        let commit = match result {
            PunctuationProcessResult::Accepted => None,
            PunctuationProcessResult::Commit(commit) => Some(session.engine.record_commit(commit)),
        };
        update_session_segment_tags(session);
        return commit;
    }
    if let Some(commit) = process_alternative_select_key(session, key_event) {
        update_session_segment_tags(session);
        return commit;
    }
    let before_input = session.engine.context().composition.input.clone();
    let before_highlighted = session.engine.context().highlighted;
    let commit = session.engine.process_key_event(key_event);
    update_key_binding_paging_state(session, key_event, &before_input, before_highlighted);
    update_session_segment_tags(session);
    commit
}

fn process_ascii_composer_processor(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> AsciiComposerProcessResult {
    if !session.ascii_composer_enabled
        || key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.hyper
        || key_event.modifiers.meta
        || key_event.modifiers.release
    {
        return AsciiComposerProcessResult::Noop;
    }
    if key_event.modifiers.shift && matches!(key_event.code, KeyCode::Character(' ')) {
        return AsciiComposerProcessResult::Noop;
    }
    if !session.engine.status().is_ascii_mode {
        return AsciiComposerProcessResult::Noop;
    }
    let KeyCode::Character(ch) = key_event.code else {
        return AsciiComposerProcessResult::Noop;
    };
    if !(('\u{20}'..'\u{80}').contains(&ch)) {
        return AsciiComposerProcessResult::Noop;
    }
    if session.engine.context().composition.input.is_empty() {
        return AsciiComposerProcessResult::Rejected;
    }

    let mut input = session.engine.context().composition.input.clone();
    input.push(ch);
    session.engine.set_input(input);
    AsciiComposerProcessResult::Accepted
}

fn process_recognizer_processor(session: &mut SessionState, key_event: KeyEvent) -> bool {
    if key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.release
    {
        return false;
    }
    let KeyCode::Character(ch) = key_event.code else {
        return false;
    };
    if !((ch == ' '
        && session
            .recognizer_processor
            .as_ref()
            .is_some_and(|processor| processor.use_space))
        || (ch > '\u{20}' && ch < '\u{80}'))
    {
        return false;
    }
    let Some(processor) = &session.recognizer_processor else {
        return false;
    };

    let mut input = session.engine.context().composition.input.clone();
    input.push(ch);
    if !recognizer_patterns_match(&processor.patterns, &input) {
        return false;
    }
    session.engine.set_input(input);
    true
}

fn process_key_binder_processor(
    session_id: RimeSessionId,
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<Vec<String>> {
    {
        let Some(processor) = session.key_binder.as_mut() else {
            return None;
        };
        if processor.redirecting {
            return None;
        }
        if reinterpret_key_binding_paging_key(processor, &mut session.engine, key_event) {
            return None;
        }
    }

    let Some(processor) = session.key_binder.as_ref() else {
        return None;
    };
    let Some(bindings) = processor.bindings.get(&key_event) else {
        return None;
    };
    let Some(binding_index) = bindings
        .iter()
        .position(|binding| key_binding_condition_matches(session, binding.condition))
    else {
        return None;
    };

    let action = bindings[binding_index].action.clone();
    match action {
        KeyBindingAction::Send(events) => {
            Some(redirect_key_binding_events(session_id, session, events))
        }
        KeyBindingAction::Toggle(option) => {
            toggle_key_binding_option(session, &option);
            Some(Vec::new())
        }
        KeyBindingAction::SetOption { option, value } => {
            set_key_binding_option(session, &option, value);
            Some(Vec::new())
        }
        KeyBindingAction::SelectSchema(schema) => {
            select_key_binding_schema(session_id, session, &schema);
            Some(Vec::new())
        }
    }
}

fn reinterpret_key_binding_paging_key(
    processor: &mut KeyBinderProcessor,
    engine: &mut Engine,
    key_event: KeyEvent,
) -> bool {
    if key_event.modifiers.release {
        return false;
    }

    let ch = if key_event.modifiers.is_empty() {
        match key_event.code {
            KeyCode::Character(ch) => Some(ch),
            _ => None,
        }
    } else {
        None
    };

    if ch == Some('.') && matches!(processor.last_key, Some('.') | Some(',')) {
        processor.last_key = None;
        return false;
    }

    let mut reinterpreted = false;
    if processor.last_key == Some('.') && matches!(ch, Some('a'..='z')) {
        let input = &engine.context().composition.input;
        if !input.is_empty() && !input.ends_with('.') {
            engine.process_char('.');
            reinterpreted = true;
        }
    }

    processor.last_key = ch;
    reinterpreted
}

fn redirect_key_binding_events(
    session_id: RimeSessionId,
    session: &mut SessionState,
    events: Vec<KeyEvent>,
) -> Vec<String> {
    if let Some(processor) = session.key_binder.as_mut() {
        processor.redirecting = true;
    }
    let mut commits = Vec::new();
    for event in events {
        if let Some(commit) = process_session_key_event(session_id, session, event) {
            commits.push(commit);
        }
    }
    if let Some(processor) = session.key_binder.as_mut() {
        processor.redirecting = false;
    }
    commits
}

fn key_binding_condition_matches(session: &SessionState, condition: KeyBindingCondition) -> bool {
    match condition {
        KeyBindingCondition::Always => true,
        KeyBindingCondition::Composing => !session.engine.context().composition.input.is_empty(),
        KeyBindingCondition::HasMenu => {
            !session.engine.status().is_ascii_mode
                && !session.engine.context().candidates.is_empty()
        }
        KeyBindingCondition::Paging => {
            session.paging && !session.engine.context().composition.input.is_empty()
        }
    }
}

fn update_key_binding_paging_state(
    session: &mut SessionState,
    key_event: KeyEvent,
    before_input: &str,
    before_highlighted: usize,
) {
    let context = session.engine.context();
    if context.composition.input.is_empty() {
        session.paging = false;
        return;
    }
    if context.composition.input != before_input {
        session.paging = false;
    }
    if matches!(
        key_event.code,
        KeyCode::PreviousPage
            | KeyCode::NextPage
            | KeyCode::PreviousCandidate
            | KeyCode::NextCandidate
    ) && context.highlighted != before_highlighted
    {
        session.paging = true;
    }
}

fn toggle_key_binding_option(session: &mut SessionState, option: &str) {
    if let Some(the_option) = key_binding_switch_by_index(session, option) {
        match the_option {
            KeyBindingSwitchTarget::Toggle(option) => {
                session
                    .engine
                    .set_option(option.clone(), !session.engine.get_option(&option));
            }
            KeyBindingSwitchTarget::Radio(the_option) => {
                toggle_key_binding_radio_option(session, &the_option);
            }
        }
        return;
    }

    if let Some(the_option) = key_binding_switch_option(session, option) {
        toggle_key_binding_radio_option(session, &the_option);
        return;
    }

    session
        .engine
        .set_option(option, !session.engine.get_option(option));
}

fn toggle_key_binding_radio_option(
    session: &mut SessionState,
    the_option: &KeyBindingSwitchOption,
) {
    let selected_index = the_option
        .options
        .iter()
        .position(|option| session.engine.get_option(option));
    let next_index = selected_index
        .map(|index| (index + 1) % the_option.options.len())
        .unwrap_or(the_option.option_index);
    select_key_binding_radio_option(session, &the_option.options, next_index);
}

fn set_key_binding_option(session: &mut SessionState, option: &str, value: bool) {
    if let Some(the_option) = key_binding_switch_option(session, option) {
        if value {
            select_key_binding_radio_option(session, &the_option.options, the_option.option_index);
        } else if session.engine.get_option(option) {
            select_key_binding_radio_option(session, &the_option.options, the_option.reset_index);
        }
        return;
    }

    session.engine.set_option(option, value);
}

fn select_key_binding_schema(session_id: RimeSessionId, session: &mut SessionState, schema: &str) {
    let selected_schema = if schema == ".next" {
        next_key_binding_schema(session)
    } else {
        Some(schema.to_owned())
    };
    let Some(selected_schema) = selected_schema else {
        return;
    };
    apply_schema_to_session(session, &selected_schema);
    let status = session.engine.status();
    notify(
        session_id,
        "schema",
        &format!("{}/{}", status.schema_id, status.schema_name),
    );
}

fn next_key_binding_schema(session: &SessionState) -> Option<String> {
    let current_schema = &session.engine.status().schema_id;
    deployed_selected_schema_ids()
        .into_iter()
        .find(|schema_id| schema_id != current_schema)
}

fn select_key_binding_radio_option(
    session: &mut SessionState,
    options: &[String],
    selected_index: usize,
) {
    if selected_index >= options.len() {
        return;
    }
    for (option_index, option) in options.iter().enumerate() {
        session
            .engine
            .set_option(option.clone(), option_index == selected_index);
    }
}

fn key_binding_switch_option(
    session: &SessionState,
    option_name: &str,
) -> Option<KeyBindingSwitchOption> {
    let schema_id = &session.engine.status().schema_id;
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Value::Sequence(switches) = find_config_value(&schema_config, "switches")? else {
        return None;
    };

    for the_switch in switches {
        let Value::Mapping(switch_map) = the_switch else {
            continue;
        };
        let Some(Value::Sequence(options)) = switch_map.get(Value::String("options".to_owned()))
        else {
            continue;
        };
        let options = options
            .iter()
            .filter_map(config_scalar_string)
            .collect::<Vec<_>>();
        let Some(option_index) = options.iter().position(|option| option == option_name) else {
            continue;
        };
        let reset_index = switch_reset_value(switch_map)
            .and_then(|reset| usize::try_from(reset).ok())
            .unwrap_or(0);
        return Some(KeyBindingSwitchOption {
            options,
            option_index,
            reset_index,
        });
    }
    None
}

fn key_binding_switch_by_index(
    session: &SessionState,
    option_name: &str,
) -> Option<KeyBindingSwitchTarget> {
    let switch_index = option_name.strip_prefix('@')?.parse::<usize>().ok()?;
    let schema_id = &session.engine.status().schema_id;
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Value::Sequence(switches) = find_config_value(&schema_config, "switches")? else {
        return None;
    };
    let Value::Mapping(switch_map) = switches.get(switch_index)? else {
        return None;
    };

    if let Some(option_name) = switch_scalar_field(switch_map, "name") {
        return Some(KeyBindingSwitchTarget::Toggle(option_name));
    }

    let Some(Value::Sequence(options)) = switch_map.get(Value::String("options".to_owned())) else {
        return None;
    };
    let options = options
        .iter()
        .filter_map(config_scalar_string)
        .collect::<Vec<_>>();
    if options.is_empty() {
        return None;
    }
    let reset_index = switch_reset_value(switch_map)
        .and_then(|reset| usize::try_from(reset).ok())
        .unwrap_or(0);
    Some(KeyBindingSwitchTarget::Radio(KeyBindingSwitchOption {
        options,
        option_index: 0,
        reset_index,
    }))
}

fn process_punctuation_processor(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<PunctuationProcessResult> {
    if key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.release
        || session.engine.get_option("ascii_punct")
    {
        return None;
    }

    let KeyCode::Character(ch) = key_event.code else {
        return None;
    };
    if !ch.is_ascii() || ch.is_ascii_control() {
        return None;
    }

    if let Some(result) = process_pending_digit_separator(session, ch, &ch.to_string()) {
        return Some(result);
    }

    let use_space = session.punctuation_processor.as_ref()?.use_space;
    if ch == ' ' && !use_space && !session.engine.context().composition.input.is_empty() {
        return None;
    }

    let key = ch.to_string();
    if let Some(result) = process_digit_separator(session, ch, &key) {
        return Some(result);
    }

    if let Some(count) = active_alternating_punct_count(session, &key) {
        let highlighted = session.engine.context().highlighted;
        let next_index = (highlighted + 1) % count;
        session.engine.highlight_candidate(next_index);
        return Some(PunctuationProcessResult::Accepted);
    }

    if let Some(commit) = active_pair_commit(session, &key) {
        return Some(PunctuationProcessResult::Commit(commit));
    }

    let processor = session.punctuation_processor.as_ref()?;
    let shape_entries = if session.engine.status().is_full_shape {
        &processor.full_shape_unique_commits
    } else {
        &processor.half_shape_unique_commits
    };

    shape_entries
        .get(&key)
        .or_else(|| processor.symbol_unique_commits.get(&key))
        .cloned()
        .map(PunctuationProcessResult::Commit)
}

fn process_digit_separator(
    session: &mut SessionState,
    ch: char,
    key: &str,
) -> Option<PunctuationProcessResult> {
    let is_digit_separator = session
        .punctuation_processor
        .as_ref()
        .is_some_and(|processor| processor.digit_separators.contains(ch));
    if !is_digit_separator
        || !session.engine.context().composition.input.is_empty()
        || !session
            .engine
            .context()
            .last_commit
            .as_deref()
            .is_some_and(ends_with_ascii_digit)
        || !active_punctuation_definition_exists(session, key)
    {
        return None;
    }

    let full_shape = session.engine.status().is_full_shape;
    let digit_separator_commit = session
        .punctuation_processor
        .as_ref()
        .is_some_and(|processor| processor.digit_separator_commit);
    let punct = shape_formatted_ascii_text(key, full_shape);
    if digit_separator_commit {
        return Some(PunctuationProcessResult::Commit(punct));
    }

    if let Some(processor) = session.punctuation_processor.as_mut() {
        processor.pending_digit_separator = Some(key.to_owned());
    }
    session
        .engine
        .set_punctuation_composition(key.to_owned(), punct);
    Some(PunctuationProcessResult::Accepted)
}

fn process_pending_digit_separator(
    session: &mut SessionState,
    ch: char,
    key: &str,
) -> Option<PunctuationProcessResult> {
    let pending = session
        .punctuation_processor
        .as_ref()
        .and_then(|processor| processor.pending_digit_separator.as_deref())?;
    if session.engine.context().composition.input != pending {
        if let Some(processor) = session.punctuation_processor.as_mut() {
            processor.pending_digit_separator = None;
        }
        return None;
    }

    if ch.is_ascii_digit() || ch == ' ' {
        let commit = shape_formatted_ascii_text(
            &format!("{pending}{ch}"),
            session.engine.status().is_full_shape,
        );
        if let Some(processor) = session.punctuation_processor.as_mut() {
            processor.pending_digit_separator = None;
        }
        return Some(PunctuationProcessResult::Commit(commit));
    }

    if key == pending {
        if let Some(processor) = session.punctuation_processor.as_mut() {
            processor.pending_digit_separator = None;
        }
        session.engine.set_input(key.to_owned());
        return Some(PunctuationProcessResult::Accepted);
    }

    None
}

fn active_punctuation_definition_exists(session: &SessionState, key: &str) -> bool {
    let Some(processor) = session.punctuation_processor.as_ref() else {
        return false;
    };
    let (shape_unique_commits, shape_alternating_counts, shape_pairs) =
        if session.engine.status().is_full_shape {
            (
                &processor.full_shape_unique_commits,
                &processor.full_shape_alternating_counts,
                &processor.full_shape_pairs,
            )
        } else {
            (
                &processor.half_shape_unique_commits,
                &processor.half_shape_alternating_counts,
                &processor.half_shape_pairs,
            )
        };

    shape_unique_commits.contains_key(key)
        || shape_alternating_counts.contains_key(key)
        || shape_pairs.contains_key(key)
        || processor.symbol_unique_commits.contains_key(key)
        || processor.symbol_alternating_counts.contains_key(key)
        || processor.symbol_pairs.contains_key(key)
}

fn ends_with_ascii_digit(text: &str) -> bool {
    text.as_bytes()
        .last()
        .is_some_and(|byte| byte.is_ascii_digit())
}

fn shape_formatted_ascii_text(text: &str, full_shape: bool) -> String {
    if !full_shape {
        return text.to_owned();
    }
    text.chars()
        .map(|ch| match ch {
            ' ' => '\u{3000}',
            '!'..='~' => char::from_u32(ch as u32 + 0xfee0)
                .expect("printable ASCII has a full-shape compatibility form"),
            _ => ch,
        })
        .collect()
}

fn active_pair_commit(session: &mut SessionState, key: &str) -> Option<String> {
    let processor = session.punctuation_processor.as_mut()?;
    let is_full_shape = session.engine.status().is_full_shape;
    let shape_name = if is_full_shape {
        "full_shape"
    } else {
        "half_shape"
    };
    let shape_pairs = if is_full_shape {
        &processor.full_shape_pairs
    } else {
        &processor.half_shape_pairs
    };
    let (pair_name, pair) = shape_pairs
        .get(key)
        .map(|pair| (shape_name, pair))
        .or_else(|| {
            processor
                .symbol_pairs
                .get(key)
                .map(|pair| ("symbols", pair))
        })?;

    let oddness_key = format!("{pair_name}:{key}");
    let oddness = processor.pair_oddness.entry(oddness_key).or_insert(0);
    let commit = pair[*oddness % 2].clone();
    *oddness = 1 - (*oddness % 2);
    Some(commit)
}

fn active_alternating_punct_count(session: &SessionState, key: &str) -> Option<usize> {
    let context = session.engine.context();
    if context.composition.input != key || context.candidates.is_empty() {
        return None;
    }

    let processor = session.punctuation_processor.as_ref()?;
    let shape_counts = if session.engine.status().is_full_shape {
        &processor.full_shape_alternating_counts
    } else {
        &processor.half_shape_alternating_counts
    };
    shape_counts
        .get(key)
        .or_else(|| processor.symbol_alternating_counts.get(key))
        .copied()
        .filter(|count| *count > 0)
        .map(|count| count.min(context.candidates.len()))
}

fn process_alternative_select_key(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<Option<String>> {
    if key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.release
        || session.engine.context().candidates.is_empty()
    {
        return None;
    }
    let KeyCode::Character(ch) = key_event.code else {
        return None;
    };
    if !ch.is_ascii() || !('\u{20}'..='\u{7e}').contains(&ch) {
        return None;
    }

    let menu_settings = context_menu_settings(&session.engine.status().schema_id);
    let select_keys = menu_settings.select_keys.as_deref()?;
    let Some(index) = select_keys
        .bytes()
        .position(|select_key| select_key == ch as u8)
    else {
        return ch.is_ascii_digit().then_some(None);
    };
    if index >= menu_settings.page_size {
        return Some(None);
    }

    let page_start =
        (session.engine.context().highlighted / menu_settings.page_size) * menu_settings.page_size;
    Some(session.engine.select_candidate(page_start + index))
}

fn candidate_index_on_current_page(session: &SessionState, index: usize) -> Option<usize> {
    let page_size = session_menu_page_size(session);
    if index >= page_size || session.engine.context().candidates.is_empty() {
        return None;
    }

    let page_start = (session.engine.context().highlighted / page_size) * page_size;
    Some(page_start + index)
}

fn highlight_candidate_clamped_like_librime(session: &mut SessionState, index: usize) -> bool {
    let candidate_count = session.engine.context().candidates.len();
    if candidate_count == 0 {
        return false;
    }

    let new_index = index.min(candidate_count - 1);
    if new_index == session.engine.context().highlighted {
        return false;
    }

    session.engine.highlight_candidate(new_index)
}

fn with_session(session_id: RimeSessionId, action: impl FnOnce(&mut SessionState) -> bool) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
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

fn session_activity_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
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
        cstring_borrows: Vec::new(),
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

fn selected_runtime_data_path(file_name: &str) -> Option<PathBuf> {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    [
        paths.staging_dir.to_string_lossy().into_owned(),
        paths.prebuilt_data_dir.to_string_lossy().into_owned(),
        paths.shared_data_dir.to_string_lossy().into_owned(),
    ]
    .into_iter()
    .map(|root| Path::new(&root).join(file_name))
    .find(|path| path.is_file())
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
        .filter_map(deployed_schema_list_entry)
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

fn deployed_schema_name(schema_id: &str) -> String {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    find_config_value(&schema_config, "schema/name")
        .and_then(Value::as_str)
        .unwrap_or(schema_id)
        .to_owned()
}

fn deployed_levers_schema_infos() -> Vec<LeverSchemaInfo> {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    let roots = [
        paths.shared_data_dir.to_string_lossy().into_owned(),
        paths.user_data_dir.to_string_lossy().into_owned(),
    ];
    drop(paths);

    let mut seen = HashSet::new();
    let mut infos = Vec::new();
    for root in roots {
        for path in schema_file_paths_in_dir(&root) {
            let Some((schema_id, schema_config)) = levers_schema_config_from_file(&path) else {
                continue;
            };
            if !seen.insert(schema_id.clone()) {
                continue;
            }
            infos.push(levers_schema_info(schema_id, schema_config, Some(path)));
        }
    }
    infos
}

fn schema_file_paths_in_dir(root: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().ends_with(".schema.yaml"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn levers_schema_config_from_file(path: &Path) -> Option<(String, Value)> {
    let yaml = fs::read_to_string(path).ok()?;
    let schema_config = serde_yaml::from_str::<Value>(&yaml).ok()?;
    let schema_id = find_config_value(&schema_config, "schema/schema_id")?
        .as_str()?
        .to_owned();
    find_config_value(&schema_config, "schema/name")?.as_str()?;
    Some((schema_id, schema_config))
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
    let (
        user_data_dir,
        current_sync_dir,
        distribution_name,
        distribution_code_name,
        distribution_version,
    ) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
            paths.sync_dir.to_string_lossy().into_owned(),
            paths.distribution_name.to_string_lossy().into_owned(),
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
    if !distribution_name.is_empty() {
        root.insert(
            Value::String("distribution_name".to_owned()),
            Value::String(distribution_name.clone()),
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

fn clean_old_log_files() -> bool {
    let (app_name, log_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            paths.app_name.to_string_lossy().into_owned(),
            PathBuf::from(paths.log_dir.to_string_lossy().into_owned()),
        )
    };
    if app_name.is_empty() || log_dir.as_os_str().is_empty() {
        return true;
    }

    let Ok(entries) = fs::read_dir(&log_dir) else {
        return true;
    };
    let entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    let mut files_in_use = HashSet::new();
    for path in &entries {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            continue;
        };
        if !metadata.file_type().is_symlink() {
            continue;
        }
        let Ok(target) = fs::read_link(path) else {
            continue;
        };
        let Some(target_file_name) = target.file_name().and_then(|file_name| file_name.to_str())
        else {
            continue;
        };
        if target_file_name.starts_with(&app_name) && target_file_name.ends_with(".log") {
            files_in_use.insert(target_file_name.to_owned());
        }
    }

    let today = current_log_date_marker();
    let mut success = true;
    for path in entries {
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            success = false;
            continue;
        };
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
            continue;
        };
        if !file_name.starts_with(&app_name) || !file_name.ends_with(".log") {
            continue;
        }
        if file_name.contains(&today) {
            continue;
        }
        if files_in_use.contains(file_name) {
            continue;
        }
        if fs::remove_file(&path).is_err() {
            success = false;
        }
    }
    success
}

fn detect_modifications() -> bool {
    let (shared_data_dir, user_data_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned()),
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
        )
    };

    let Some(last_modified) =
        latest_workspace_modified_time([user_data_dir.as_path(), shared_data_dir.as_path()])
    else {
        return true;
    };
    last_modified > user_last_build_time(&user_data_dir)
}

fn latest_workspace_modified_time<const N: usize>(data_dirs: [&Path; N]) -> Option<u64> {
    let mut last_modified = 0;
    for data_dir in data_dirs {
        last_modified = last_modified.max(file_modified_secs(data_dir)?);
        if data_dir.is_dir() {
            let entries = fs::read_dir(data_dir).ok()?;
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if !path.is_file() || !is_workspace_yaml_file(&path) {
                    continue;
                }
                last_modified = last_modified.max(file_modified_secs(&path)?);
            }
        }
    }
    Some(last_modified)
}

fn file_modified_secs(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn user_last_build_time(user_data_dir: &Path) -> u64 {
    let user_config_path = user_data_dir.join("user.yaml");
    fs::read_to_string(user_config_path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .and_then(|root| {
            find_config_value(&root, "var/last_build_time")
                .and_then(Value::as_i64)
                .and_then(|value| u64::try_from(value).ok())
        })
        .unwrap_or(0)
}

fn is_workspace_yaml_file(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("yaml")
        && path.file_name().and_then(|file_name| file_name.to_str()) != Some("user.yaml")
}

fn current_log_date_marker() -> String {
    let days_since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() / 86_400);
    let (year, month, day) = civil_from_days(days_since_epoch as i64);
    format!(".{year:04}{month:02}{day:02}")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn workspace_update() -> bool {
    if !deploy_config_file("default.yaml", "config_version") {
        return false;
    }
    let _ = symlink_prebuilt_dictionaries();

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

fn run_workspace_maintenance_tasks() -> bool {
    workspace_update() && user_dict_upgrade() && cleanup_trash()
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
    if !as_dependency {
        for dependency_id in schema_dependencies(&schema_config) {
            if !workspace_update_schema(&dependency_id, true, built) {
                return false;
            }
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

fn symlink_prebuilt_dictionaries() -> bool {
    let (shared_data_dir, user_data_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned()),
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
        )
    };
    if !shared_data_dir.is_dir() || !user_data_dir.is_dir() {
        return false;
    }
    let Ok(shared_data_dir) = shared_data_dir.canonicalize() else {
        return false;
    };
    if user_data_dir
        .canonicalize()
        .is_ok_and(|user_data_dir| user_data_dir == shared_data_dir)
    {
        return false;
    }

    let Ok(entries) = fs::read_dir(&user_data_dir) else {
        return false;
    };
    let mut success = true;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            success = false;
            continue;
        };
        if !metadata.file_type().is_symlink() {
            continue;
        }

        let target_path = path.canonicalize();
        let bad_link = target_path.is_err();
        let linked_to_shared_data = target_path
            .ok()
            .and_then(|target_path| target_path.parent().map(Path::to_path_buf))
            .is_some_and(|parent| parent == shared_data_dir);
        if (bad_link || linked_to_shared_data) && fs::remove_file(&path).is_err() {
            success = false;
        }
    }
    success
}

fn deploy_config_file(file_name: &str, version_key: &str) -> bool {
    if file_name.is_empty() || version_key.is_empty() {
        return false;
    }

    let (shared_data_dir, user_data_dir, staging_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned()),
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
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
    let user_copy = user_data_dir.join(file_name);
    let trash_dir = user_data_dir.join("trash");
    let _ = trash_deprecated_user_copy(&source, &user_copy, version_key, &trash_dir);
    if !deployed_config_needs_update(&destination, file_name, &shared_data_dir, &user_data_dir) {
        return true;
    }
    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    match deployed_config_yaml_with_build_info(&source, file_name, &shared_data_dir, &user_data_dir)
    {
        Some(yaml) => fs::write(destination, yaml).is_ok(),
        None => fs::copy(source, destination).is_ok(),
    }
}

fn deployed_config_needs_update(
    destination: &Path,
    file_name: &str,
    shared_data_dir: &Path,
    user_data_dir: &Path,
) -> bool {
    let root = match fs::read_to_string(destination)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
    {
        Some(root) => root,
        None => return true,
    };
    let Some(Value::Mapping(timestamps)) = find_config_value(&root, "__build_info/timestamps")
    else {
        return true;
    };
    let resource_id = normalize_config_resource_id(file_name);
    if !timestamps.contains_key(Value::String(resource_id.clone())) {
        return true;
    }
    let custom_resource_id = custom_patch_resource_id(&resource_id);
    if source_uses_auto_custom_patch(&shared_data_dir.join(file_name))
        && !timestamps.contains_key(Value::String(custom_resource_id.clone()))
        && config_resource_path(shared_data_dir, user_data_dir, &custom_resource_id).exists()
    {
        return true;
    }
    for (resource_id, recorded_time) in timestamps {
        let Some(resource_id) = resource_id.as_str() else {
            return true;
        };
        let Some(recorded_time) = recorded_time.as_i64() else {
            return true;
        };
        let source = config_resource_path(shared_data_dir, user_data_dir, resource_id);
        if !source.exists() {
            if recorded_time != 0 {
                return true;
            }
            continue;
        }
        let Some(source_time) = source_modified_secs(&source) else {
            return true;
        };
        if recorded_time != i64::from(source_time) {
            return true;
        }
    }
    false
}

fn deployed_config_yaml_with_build_info(
    source: &Path,
    file_name: &str,
    shared_data_dir: &Path,
    user_data_dir: &Path,
) -> Option<String> {
    let mut root = fs::read_to_string(source)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())?;
    let resource_id = normalize_config_resource_id(file_name);
    let timestamp = source_modified_secs(source).unwrap_or(0);
    let mut patch_dependencies = Vec::new();
    let apply_auto_custom_patch =
        apply_config_directives(&mut root, shared_data_dir, &mut patch_dependencies)?;
    apply_legacy_preset_config_plugins(
        &mut root,
        &resource_id,
        shared_data_dir,
        &mut patch_dependencies,
    )?;
    set_build_info(&mut root, &resource_id, timestamp)?;

    if apply_auto_custom_patch {
        let custom_resource_id = custom_patch_resource_id(&resource_id);
        let custom_path = user_data_dir.join(format!("{custom_resource_id}.yaml"));
        if let Some(custom_root) = fs::read_to_string(&custom_path)
            .ok()
            .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
        {
            apply_custom_patch(
                &mut root,
                &custom_root,
                shared_data_dir,
                &mut patch_dependencies,
            )?;
            set_build_info(
                &mut root,
                &custom_resource_id,
                source_modified_secs(&custom_path).unwrap_or(0),
            )?;
        } else {
            set_build_info(&mut root, &custom_resource_id, 0)?;
        }
    }
    for (resource_id, timestamp) in patch_dependencies {
        set_build_info(&mut root, &resource_id, timestamp)?;
    }
    serde_yaml::to_string(&root).ok()
}

fn custom_patch_resource_id(resource_id: &str) -> String {
    let base = resource_id.strip_suffix(".schema").unwrap_or(resource_id);
    format!("{base}.custom")
}

fn config_resource_path(
    shared_data_dir: &Path,
    user_data_dir: &Path,
    resource_id: &str,
) -> PathBuf {
    let root = if resource_id.ends_with(".custom") {
        user_data_dir
    } else {
        shared_data_dir
    };
    root.join(format!("{resource_id}.yaml"))
}

fn trash_deprecated_user_copy(
    shared_copy: &Path,
    user_copy: &Path,
    version_key: &str,
    trash_dir: &Path,
) -> bool {
    if !shared_copy.exists()
        || !user_copy.exists()
        || paths_equivalent(shared_copy, user_copy).unwrap_or(false)
    {
        return false;
    }

    let mut shared_version = config_string_from_file(shared_copy, version_key).unwrap_or_default();
    let _ = remove_version_suffix(&mut shared_version, ".minimal");
    let mut user_version = config_string_from_file(user_copy, version_key).unwrap_or_default();
    let is_customized_user_copy = remove_version_suffix(&mut user_version, ".custom.");
    if compare_version_strings(&shared_version, &user_version).is_gt()
        || (shared_version == user_version && is_customized_user_copy)
    {
        if fs::create_dir_all(trash_dir).is_err() {
            return false;
        }
        return fs::rename(
            user_copy,
            trash_dir.join(user_copy.file_name().unwrap_or_default()),
        )
        .is_ok();
    }
    false
}

fn paths_equivalent(left: &Path, right: &Path) -> Option<bool> {
    Some(left.canonicalize().ok()? == right.canonicalize().ok()?)
}

fn config_string_from_file(path: &Path, key: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .and_then(|root| {
            find_config_value(&root, key)
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn remove_version_suffix(version: &mut String, suffix: &str) -> bool {
    let Some(index) = version.find(suffix) else {
        return false;
    };
    version.truncate(index);
    true
}

fn compare_version_strings(left: &str, right: &str) -> std::cmp::Ordering {
    let mut left_parts = left.split('.');
    let mut right_parts = right.split('.');
    loop {
        match (left_parts.next(), right_parts.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (Some(part), None) => {
                return compare_version_part(part, "0").then(std::cmp::Ordering::Greater);
            }
            (None, Some(part)) => {
                return compare_version_part("0", part).then(std::cmp::Ordering::Less);
            }
            (Some(left), Some(right)) => {
                let ordering = compare_version_part(left, right);
                if !ordering.is_eq() {
                    return ordering;
                }
            }
        }
    }
}

fn compare_version_part(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.parse::<u64>(), right.parse::<u64>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
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

fn deployed_schema_list_entry(entry: &Value) -> Option<String> {
    let Value::Mapping(entry) = entry else {
        return None;
    };
    entry
        .get(Value::String("schema".to_owned()))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
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
        let mut registry = sessions()
            .lock()
            .expect("session registry should not be poisoned");
        registry
            .get_session_mut(session_id)
            .map(|session| session.engine.status().schema_id)?
    };
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    switch_state_label(&schema_config, option_name, state, abbreviated)
}

fn context_menu_settings(schema_id: &str) -> ContextMenuSettings {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let page_size = find_config_value(&schema_config, "menu/page_size")
        .and_then(Value::as_i64)
        .and_then(|page_size| c_int::try_from(page_size).ok())
        .filter(|page_size| *page_size > 0)
        .map(|page_size| page_size as usize)
        .unwrap_or(DEFAULT_PAGE_SIZE);
    let select_keys = find_config_value(&schema_config, "menu/alternative_select_keys")
        .and_then(Value::as_str)
        .filter(|select_keys| !select_keys.is_empty())
        .map(ToOwned::to_owned);
    let select_labels = match find_config_value(&schema_config, "menu/alternative_select_labels") {
        Some(Value::Sequence(labels)) => labels
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    };

    ContextMenuSettings {
        page_size,
        select_keys,
        select_labels,
    }
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
    copy_c_string_with_strncpy_semantics(&value, output, buffer_size);
}

fn clear_commit(commit: *mut RimeCommit) {
    // SAFETY: callers only pass non-null pointers to this helper; fields are
    // plain pointers and assigning null mirrors librime's clear macro while
    // preserving the self-versioned struct's `data_size` field.
    unsafe {
        (*commit).text = ptr::null_mut();
    }
}

fn clear_context(context: *mut RimeContext) {
    // SAFETY: callers only pass non-null pointers to this helper; this mirrors
    // librime's versioned struct clear by preserving `data_size` and only
    // clearing members covered by the caller-provided version.
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
        if context_has_commit_text_preview(context) {
            (*context).commit_text_preview = ptr::null_mut();
        }
        if context_has_select_labels(context) {
            (*context).select_labels = ptr::null_mut();
        }
    }
}

unsafe fn context_has_commit_text_preview(context: *const RimeContext) -> bool {
    // SAFETY: callers pass a valid `RimeContext` pointer; `addr_of!` computes a
    // field address without creating an intermediate reference.
    unsafe {
        rime_struct_has_member(
            context,
            (*context).data_size,
            ptr::addr_of!((*context).commit_text_preview),
        )
    }
}

unsafe fn context_has_select_labels(context: *const RimeContext) -> bool {
    // SAFETY: callers pass a valid `RimeContext` pointer; `addr_of!` computes a
    // field address without creating an intermediate reference.
    unsafe {
        rime_struct_has_member(
            context,
            (*context).data_size,
            ptr::addr_of!((*context).select_labels),
        )
    }
}

fn rime_struct_has_member<T, U>(object: *const T, data_size: c_int, member: *const U) -> bool {
    let Ok(data_size) = usize::try_from(data_size) else {
        return false;
    };
    let bytes_after_data_size = std::mem::size_of::<c_int>().saturating_add(data_size);
    let member_offset = (member as usize).saturating_sub(object as usize);
    bytes_after_data_size > member_offset
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
        if context_has_commit_text_preview(context) && !(*context).commit_text_preview.is_null() {
            drop(CString::from_raw((*context).commit_text_preview));
        }
        if context_has_select_labels(context) && !(*context).select_labels.is_null() {
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

fn copy_c_string_with_strncpy_semantics(value: &str, output: *mut c_char, buffer_size: usize) {
    if buffer_size == 0 {
        return;
    }

    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(buffer_size);
    // SAFETY: callers pass writable storage of `buffer_size` bytes; `copy_len`
    // is bounded by `buffer_size`, and the zero-fill mirrors `strncpy` for
    // source strings shorter than the destination buffer.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), output, copy_len);
        if copy_len < buffer_size {
            ptr::write_bytes(output.add(copy_len), 0, buffer_size - copy_len);
        }
    }
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
    settings.custom_config.cstring_borrows.clear();
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

#[cfg(unix)]
fn librime_signature_modified_time() -> String {
    // librime's Signature::Sign stores a trimmed ctime(3) string.
    let now = unsafe { libc::time(ptr::null_mut()) };
    let mut buffer = [0 as c_char; 64];
    let written = unsafe { libc::ctime_r(&now, buffer.as_mut_ptr()) };
    if written.is_null() {
        return "0".to_owned();
    }
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .trim()
        .to_owned()
}

#[cfg(not(unix))]
fn librime_signature_modified_time() -> String {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |_| "0".to_owned(),
        |duration| duration.as_secs().to_string(),
    )
}

#[cfg(test)]
mod tests;
