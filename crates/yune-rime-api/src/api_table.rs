use std::{
    ffi::CString,
    os::raw::{c_char, c_int},
    sync::{Mutex, OnceLock},
};

use crate::*;

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

pub(crate) fn state_label_cache() -> &'static Mutex<Option<CString>> {
    static STATE_LABEL_CACHE: OnceLock<Mutex<Option<CString>>> = OnceLock::new();
    STATE_LABEL_CACHE.get_or_init(|| Mutex::new(None))
}

fn api_entry() -> *mut RimeApi {
    static API: OnceLock<usize> = OnceLock::new();
    *API.get_or_init(|| Box::into_raw(Box::new(build_rime_api())) as usize) as *mut RimeApi
}

fn typeduck_profile_api_entry() -> *mut RimeTypeDuckProfileApi {
    static API: OnceLock<usize> = OnceLock::new();
    *API.get_or_init(|| Box::into_raw(Box::new(build_typeduck_profile_api())) as usize)
        as *mut RimeTypeDuckProfileApi
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RimeTypeDuckProfileApi {
    pub upstream: RimeApi,
    pub config_list_append_bool:
        Option<unsafe extern "C" fn(*mut RimeConfig, *const c_char, Bool) -> Bool>,
    pub config_list_append_int:
        Option<unsafe extern "C" fn(*mut RimeConfig, *const c_char, c_int) -> Bool>,
    pub config_list_append_double:
        Option<unsafe extern "C" fn(*mut RimeConfig, *const c_char, f64) -> Bool>,
    pub config_list_append_string:
        Option<unsafe extern "C" fn(*mut RimeConfig, *const c_char, *const c_char) -> Bool>,
}

fn build_typeduck_profile_api() -> RimeTypeDuckProfileApi {
    let mut upstream = build_rime_api();
    upstream.data_size =
        (std::mem::size_of::<RimeTypeDuckProfileApi>() - std::mem::size_of::<c_int>()) as c_int;
    RimeTypeDuckProfileApi {
        upstream,
        config_list_append_bool: Some(RimeConfigListAppendBool),
        config_list_append_int: Some(RimeConfigListAppendInt),
        config_list_append_double: Some(RimeConfigListAppendDouble),
        config_list_append_string: Some(RimeConfigListAppendString),
    }
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

#[no_mangle]
pub extern "C" fn rime_get_api() -> *mut RimeApi {
    api_entry()
}

#[no_mangle]
pub extern "C" fn rime_get_typeduck_profile_api() -> *mut RimeTypeDuckProfileApi {
    typeduck_profile_api_entry()
}

#[no_mangle]
pub extern "C" fn rime_levers_get_api() -> *mut RimeCustomApi {
    levers_api_entry().cast::<RimeCustomApi>()
}
