use std::{
    ffi::c_void,
    os::raw::{c_char, c_int},
};

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
type ConfigListAppendBoolFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, Bool) -> Bool;
type ConfigListAppendIntFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, c_int) -> Bool;
type ConfigListAppendDoubleFn = unsafe extern "C" fn(*mut RimeConfig, *const c_char, f64) -> Bool;
type ConfigListAppendStringFn =
    unsafe extern "C" fn(*mut RimeConfig, *const c_char, *const c_char) -> Bool;
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
    pub config_list_append_bool: Option<ConfigListAppendBoolFn>,
    pub config_list_append_int: Option<ConfigListAppendIntFn>,
    pub config_list_append_double: Option<ConfigListAppendDoubleFn>,
    pub config_list_append_string: Option<ConfigListAppendStringFn>,
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
