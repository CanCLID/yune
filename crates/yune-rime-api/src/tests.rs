use std::env;
use std::ffi::{c_void, CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_int};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_yaml::Value;
use yune_core::{Candidate, CandidateSource, StaticTableTranslator, Translator};

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
    RimeUserConfigOpen, RimeUserDictIterator, FALSE, K_ALT_MASK, K_CONTROL_MASK, K_RELEASE_MASK,
    K_SHIFT_MASK, K_SUPER_MASK, TRUE,
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

#[test]
fn rime_frontend_struct_layout_matches_librime_header() {
    let int_size = std::mem::size_of::<c_int>();
    let ptr_size = std::mem::size_of::<*const c_char>();
    let ptr_align = std::mem::align_of::<*const c_char>();

    let traits = empty_traits();
    let traits_shared_data_dir = align_up(int_size, ptr_align);
    let traits_min_log_level = traits_shared_data_dir + ptr_size * 7;
    let traits_log_dir = align_up(traits_min_log_level + int_size, ptr_align);
    assert_eq!(
        field_offset(&traits, std::ptr::addr_of!(traits.data_size)),
        0
    );
    assert_eq!(
        field_offset(&traits, std::ptr::addr_of!(traits.shared_data_dir)),
        traits_shared_data_dir
    );
    assert_eq!(
        field_offset(&traits, std::ptr::addr_of!(traits.modules)),
        traits_shared_data_dir + ptr_size * 6
    );
    assert_eq!(
        field_offset(&traits, std::ptr::addr_of!(traits.min_log_level)),
        traits_min_log_level
    );
    assert_eq!(
        field_offset(&traits, std::ptr::addr_of!(traits.log_dir)),
        traits_log_dir
    );
    assert_eq!(
        field_offset(&traits, std::ptr::addr_of!(traits.prebuilt_data_dir)),
        traits_log_dir + ptr_size
    );
    assert_eq!(
        field_offset(&traits, std::ptr::addr_of!(traits.staging_dir)),
        traits_log_dir + ptr_size * 2
    );
    assert_eq!(
        std::mem::size_of::<RimeTraits>(),
        align_up(traits_log_dir + ptr_size * 3, ptr_align)
    );

    let composition = super::RimeComposition {
        length: 0,
        cursor_pos: 0,
        sel_start: 0,
        sel_end: 0,
        preedit: std::ptr::null_mut(),
    };
    let composition_preedit = align_up(int_size * 4, ptr_align);
    assert_eq!(
        field_offset(&composition, std::ptr::addr_of!(composition.length)),
        0
    );
    assert_eq!(
        field_offset(&composition, std::ptr::addr_of!(composition.cursor_pos)),
        int_size
    );
    assert_eq!(
        field_offset(&composition, std::ptr::addr_of!(composition.sel_start)),
        int_size * 2
    );
    assert_eq!(
        field_offset(&composition, std::ptr::addr_of!(composition.sel_end)),
        int_size * 3
    );
    assert_eq!(
        field_offset(&composition, std::ptr::addr_of!(composition.preedit)),
        composition_preedit
    );
    assert_eq!(
        std::mem::size_of::<super::RimeComposition>(),
        align_up(composition_preedit + ptr_size, ptr_align)
    );

    let candidate = super::RimeCandidate {
        text: std::ptr::null_mut(),
        comment: std::ptr::null_mut(),
        reserved: std::ptr::null_mut(),
    };
    assert_eq!(
        field_offset(&candidate, std::ptr::addr_of!(candidate.text)),
        0
    );
    assert_eq!(
        field_offset(&candidate, std::ptr::addr_of!(candidate.comment)),
        ptr_size
    );
    assert_eq!(
        field_offset(&candidate, std::ptr::addr_of!(candidate.reserved)),
        ptr_size * 2
    );
    assert_eq!(std::mem::size_of::<super::RimeCandidate>(), ptr_size * 3);

    let menu = super::RimeMenu {
        page_size: 0,
        page_no: 0,
        is_last_page: FALSE,
        highlighted_candidate_index: 0,
        num_candidates: 0,
        candidates: std::ptr::null_mut(),
        select_keys: std::ptr::null_mut(),
    };
    let menu_candidates = align_up(int_size * 5, ptr_align);
    assert_eq!(field_offset(&menu, std::ptr::addr_of!(menu.page_size)), 0);
    assert_eq!(
        field_offset(&menu, std::ptr::addr_of!(menu.highlighted_candidate_index)),
        int_size * 3
    );
    assert_eq!(
        field_offset(&menu, std::ptr::addr_of!(menu.candidates)),
        menu_candidates
    );
    assert_eq!(
        field_offset(&menu, std::ptr::addr_of!(menu.select_keys)),
        menu_candidates + ptr_size
    );
    assert_eq!(
        std::mem::size_of::<super::RimeMenu>(),
        align_up(menu_candidates + ptr_size * 2, ptr_align)
    );

    let commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let commit_text = align_up(int_size, ptr_align);
    assert_eq!(
        field_offset(&commit, std::ptr::addr_of!(commit.data_size)),
        0
    );
    assert_eq!(
        field_offset(&commit, std::ptr::addr_of!(commit.text)),
        commit_text
    );
    assert_eq!(
        std::mem::size_of::<RimeCommit>(),
        align_up(commit_text + ptr_size, ptr_align)
    );

    let context = empty_context();
    let context_composition = align_up(int_size, std::mem::align_of::<super::RimeComposition>());
    let context_menu = align_up(
        context_composition + std::mem::size_of::<super::RimeComposition>(),
        std::mem::align_of::<super::RimeMenu>(),
    );
    let context_commit_preview = align_up(
        context_menu + std::mem::size_of::<super::RimeMenu>(),
        ptr_align,
    );
    assert_eq!(
        field_offset(&context, std::ptr::addr_of!(context.data_size)),
        0
    );
    assert_eq!(
        field_offset(&context, std::ptr::addr_of!(context.composition)),
        context_composition
    );
    assert_eq!(
        field_offset(&context, std::ptr::addr_of!(context.menu)),
        context_menu
    );
    assert_eq!(
        field_offset(&context, std::ptr::addr_of!(context.commit_text_preview)),
        context_commit_preview
    );
    assert_eq!(
        field_offset(&context, std::ptr::addr_of!(context.select_labels)),
        context_commit_preview + ptr_size
    );
    assert_eq!(
        std::mem::size_of::<RimeContext>(),
        align_up(context_commit_preview + ptr_size * 2, ptr_align)
    );

    let status = empty_status();
    let status_schema_id = align_up(int_size, ptr_align);
    let status_disabled = status_schema_id + ptr_size * 2;
    assert_eq!(
        field_offset(&status, std::ptr::addr_of!(status.data_size)),
        0
    );
    assert_eq!(
        field_offset(&status, std::ptr::addr_of!(status.schema_id)),
        status_schema_id
    );
    assert_eq!(
        field_offset(&status, std::ptr::addr_of!(status.schema_name)),
        status_schema_id + ptr_size
    );
    assert_eq!(
        field_offset(&status, std::ptr::addr_of!(status.is_disabled)),
        status_disabled
    );
    assert_eq!(
        field_offset(&status, std::ptr::addr_of!(status.is_ascii_punct)),
        status_disabled + int_size * 6
    );
    assert_eq!(
        std::mem::size_of::<RimeStatus>(),
        align_up(status_disabled + int_size * 7, ptr_align)
    );

    let iterator = empty_candidate_list_iterator();
    let iterator_index = align_up(ptr_size, std::mem::align_of::<c_int>());
    let iterator_candidate = align_up(
        iterator_index + int_size,
        std::mem::align_of::<super::RimeCandidate>(),
    );
    assert_eq!(field_offset(&iterator, std::ptr::addr_of!(iterator.ptr)), 0);
    assert_eq!(
        field_offset(&iterator, std::ptr::addr_of!(iterator.index)),
        iterator_index
    );
    assert_eq!(
        field_offset(&iterator, std::ptr::addr_of!(iterator.candidate)),
        iterator_candidate
    );
    assert_eq!(
        std::mem::size_of::<RimeCandidateListIterator>(),
        align_up(
            iterator_candidate + std::mem::size_of::<super::RimeCandidate>(),
            std::mem::align_of::<RimeCandidateListIterator>(),
        )
    );

    let config = empty_config();
    assert_eq!(field_offset(&config, std::ptr::addr_of!(config.ptr)), 0);
    assert_eq!(std::mem::size_of::<RimeConfig>(), ptr_size);

    let config_iterator = empty_config_iterator();
    let config_iterator_index = ptr_size * 2;
    let config_iterator_key = align_up(config_iterator_index + int_size, ptr_align);
    assert_eq!(
        field_offset(&config_iterator, std::ptr::addr_of!(config_iterator.list)),
        0
    );
    assert_eq!(
        field_offset(&config_iterator, std::ptr::addr_of!(config_iterator.map)),
        ptr_size
    );
    assert_eq!(
        field_offset(&config_iterator, std::ptr::addr_of!(config_iterator.index)),
        config_iterator_index
    );
    assert_eq!(
        field_offset(&config_iterator, std::ptr::addr_of!(config_iterator.key)),
        config_iterator_key
    );
    assert_eq!(
        field_offset(&config_iterator, std::ptr::addr_of!(config_iterator.path)),
        config_iterator_key + ptr_size
    );
    assert_eq!(
        std::mem::size_of::<RimeConfigIterator>(),
        align_up(config_iterator_key + ptr_size * 2, ptr_align)
    );

    let schema_list_item = super::RimeSchemaListItem {
        schema_id: std::ptr::null_mut(),
        name: std::ptr::null_mut(),
        reserved: std::ptr::null_mut(),
    };
    assert_eq!(
        field_offset(
            &schema_list_item,
            std::ptr::addr_of!(schema_list_item.schema_id)
        ),
        0
    );
    assert_eq!(
        field_offset(&schema_list_item, std::ptr::addr_of!(schema_list_item.name)),
        ptr_size
    );
    assert_eq!(
        field_offset(
            &schema_list_item,
            std::ptr::addr_of!(schema_list_item.reserved)
        ),
        ptr_size * 2
    );
    assert_eq!(
        std::mem::size_of::<super::RimeSchemaListItem>(),
        ptr_size * 3
    );

    let schema_list = empty_schema_list();
    let usize_size = std::mem::size_of::<usize>();
    let usize_align = std::mem::align_of::<usize>();
    let schema_list_items = align_up(usize_size, ptr_align);
    assert_eq!(
        field_offset(&schema_list, std::ptr::addr_of!(schema_list.size)),
        0
    );
    assert_eq!(
        field_offset(&schema_list, std::ptr::addr_of!(schema_list.list)),
        schema_list_items
    );
    assert_eq!(
        std::mem::size_of::<super::RimeSchemaList>(),
        align_up(
            schema_list_items + ptr_size,
            std::mem::align_of::<super::RimeSchemaList>(),
        )
    );

    let string_slice = super::RimeStringSlice {
        str: std::ptr::null(),
        length: 0,
    };
    let string_slice_length = align_up(ptr_size, usize_align);
    assert_eq!(
        field_offset(&string_slice, std::ptr::addr_of!(string_slice.str)),
        0
    );
    assert_eq!(
        field_offset(&string_slice, std::ptr::addr_of!(string_slice.length)),
        string_slice_length
    );
    assert_eq!(
        std::mem::size_of::<super::RimeStringSlice>(),
        align_up(
            string_slice_length + usize_size,
            std::mem::align_of::<super::RimeStringSlice>(),
        )
    );

    let custom_api = RimeCustomApi { data_size: 0 };
    assert_eq!(
        field_offset(&custom_api, std::ptr::addr_of!(custom_api.data_size)),
        0
    );
    assert_eq!(std::mem::size_of::<RimeCustomApi>(), int_size);

    let module = RimeModule {
        data_size: 0,
        module_name: std::ptr::null(),
        initialize: None,
        finalize: None,
        get_api: None,
    };
    let function_size = std::mem::size_of::<Option<extern "C" fn()>>();
    let function_align = std::mem::align_of::<Option<extern "C" fn()>>();
    let module_name = align_up(int_size, ptr_align);
    let module_initialize = align_up(module_name + ptr_size, function_align);
    assert_eq!(
        field_offset(&module, std::ptr::addr_of!(module.data_size)),
        0
    );
    assert_eq!(
        field_offset(&module, std::ptr::addr_of!(module.module_name)),
        module_name
    );
    assert_eq!(
        field_offset(&module, std::ptr::addr_of!(module.initialize)),
        module_initialize
    );
    assert_eq!(
        field_offset(&module, std::ptr::addr_of!(module.finalize)),
        module_initialize + function_size
    );
    assert_eq!(
        field_offset(&module, std::ptr::addr_of!(module.get_api)),
        module_initialize + function_size * 2
    );
    assert_eq!(
        std::mem::size_of::<RimeModule>(),
        align_up(
            module_initialize + function_size * 3,
            std::mem::align_of::<RimeModule>(),
        )
    );

    let custom_settings = super::RimeCustomSettings { placeholder: 0 };
    let switcher_settings = super::RimeSwitcherSettings { placeholder: 0 };
    let schema_info = super::RimeSchemaInfo { placeholder: 0 };
    assert_eq!(
        field_offset(
            &custom_settings,
            std::ptr::addr_of!(custom_settings.placeholder)
        ),
        0
    );
    assert_eq!(
        field_offset(
            &switcher_settings,
            std::ptr::addr_of!(switcher_settings.placeholder)
        ),
        0
    );
    assert_eq!(
        field_offset(&schema_info, std::ptr::addr_of!(schema_info.placeholder)),
        0
    );
    assert_eq!(std::mem::size_of::<super::RimeCustomSettings>(), 1);
    assert_eq!(std::mem::size_of::<super::RimeSwitcherSettings>(), 1);
    assert_eq!(std::mem::size_of::<super::RimeSchemaInfo>(), 1);

    let user_dict_iterator = RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 0,
    };
    let user_dict_iterator_index = align_up(ptr_size, usize_align);
    assert_eq!(
        field_offset(
            &user_dict_iterator,
            std::ptr::addr_of!(user_dict_iterator.ptr)
        ),
        0
    );
    assert_eq!(
        field_offset(
            &user_dict_iterator,
            std::ptr::addr_of!(user_dict_iterator.i)
        ),
        user_dict_iterator_index
    );
    assert_eq!(
        std::mem::size_of::<RimeUserDictIterator>(),
        align_up(
            user_dict_iterator_index + usize_size,
            std::mem::align_of::<RimeUserDictIterator>(),
        )
    );
}

#[test]
fn rime_api_function_table_layout_matches_librime_header() {
    let int_size = std::mem::size_of::<c_int>();
    let fn_size = std::mem::size_of::<Option<extern "C" fn()>>();
    let fn_align = std::mem::align_of::<Option<extern "C" fn()>>();
    let table_start = align_up(int_size, fn_align);
    let api = unsafe { &*rime_get_api() };

    macro_rules! assert_api_slot {
        ($field:ident, $index:expr) => {
            assert_eq!(
                field_offset(api, std::ptr::addr_of!((*api).$field)),
                table_start + fn_size * $index,
                "RimeApi.{} must match librime rime_api.h slot {}",
                stringify!($field),
                $index
            );
        };
    }

    assert_eq!(field_offset(api, std::ptr::addr_of!((*api).data_size)), 0);
    assert_api_slot!(setup, 0);
    assert_api_slot!(set_notification_handler, 1);
    assert_api_slot!(initialize, 2);
    assert_api_slot!(finalize, 3);
    assert_api_slot!(start_maintenance, 4);
    assert_api_slot!(is_maintenance_mode, 5);
    assert_api_slot!(join_maintenance_thread, 6);
    assert_api_slot!(deployer_initialize, 7);
    assert_api_slot!(prebuild, 8);
    assert_api_slot!(deploy, 9);
    assert_api_slot!(deploy_schema, 10);
    assert_api_slot!(deploy_config_file, 11);
    assert_api_slot!(sync_user_data, 12);
    assert_api_slot!(create_session, 13);
    assert_api_slot!(find_session, 14);
    assert_api_slot!(destroy_session, 15);
    assert_api_slot!(cleanup_stale_sessions, 16);
    assert_api_slot!(cleanup_all_sessions, 17);
    assert_api_slot!(process_key, 18);
    assert_api_slot!(commit_composition, 19);
    assert_api_slot!(clear_composition, 20);
    assert_api_slot!(get_commit, 21);
    assert_api_slot!(free_commit, 22);
    assert_api_slot!(get_context, 23);
    assert_api_slot!(free_context, 24);
    assert_api_slot!(get_status, 25);
    assert_api_slot!(free_status, 26);
    assert_api_slot!(set_option, 27);
    assert_api_slot!(get_option, 28);
    assert_api_slot!(set_property, 29);
    assert_api_slot!(get_property, 30);
    assert_api_slot!(get_schema_list, 31);
    assert_api_slot!(free_schema_list, 32);
    assert_api_slot!(get_current_schema, 33);
    assert_api_slot!(select_schema, 34);
    assert_api_slot!(schema_open, 35);
    assert_api_slot!(config_open, 36);
    assert_api_slot!(config_close, 37);
    assert_api_slot!(config_get_bool, 38);
    assert_api_slot!(config_get_int, 39);
    assert_api_slot!(config_get_double, 40);
    assert_api_slot!(config_get_string, 41);
    assert_api_slot!(config_get_cstring, 42);
    assert_api_slot!(config_update_signature, 43);
    assert_api_slot!(config_begin_map, 44);
    assert_api_slot!(config_next, 45);
    assert_api_slot!(config_end, 46);
    assert_api_slot!(simulate_key_sequence, 47);
    assert_api_slot!(register_module, 48);
    assert_api_slot!(find_module, 49);
    assert_api_slot!(run_task, 50);
    assert_api_slot!(get_shared_data_dir, 51);
    assert_api_slot!(get_user_data_dir, 52);
    assert_api_slot!(get_sync_dir, 53);
    assert_api_slot!(get_user_id, 54);
    assert_api_slot!(get_user_data_sync_dir, 55);
    assert_api_slot!(config_init, 56);
    assert_api_slot!(config_load_string, 57);
    assert_api_slot!(config_set_bool, 58);
    assert_api_slot!(config_set_int, 59);
    assert_api_slot!(config_set_double, 60);
    assert_api_slot!(config_set_string, 61);
    assert_api_slot!(config_get_item, 62);
    assert_api_slot!(config_set_item, 63);
    assert_api_slot!(config_clear, 64);
    assert_api_slot!(config_create_list, 65);
    assert_api_slot!(config_create_map, 66);
    assert_api_slot!(config_list_size, 67);
    assert_api_slot!(config_begin_list, 68);
    assert_api_slot!(get_input, 69);
    assert_api_slot!(get_caret_pos, 70);
    assert_api_slot!(select_candidate, 71);
    assert_api_slot!(get_version, 72);
    assert_api_slot!(set_caret_pos, 73);
    assert_api_slot!(select_candidate_on_current_page, 74);
    assert_api_slot!(candidate_list_begin, 75);
    assert_api_slot!(candidate_list_next, 76);
    assert_api_slot!(candidate_list_end, 77);
    assert_api_slot!(user_config_open, 78);
    assert_api_slot!(candidate_list_from_index, 79);
    assert_api_slot!(get_prebuilt_data_dir, 80);
    assert_api_slot!(get_staging_dir, 81);
    assert_api_slot!(commit_proto, 82);
    assert_api_slot!(context_proto, 83);
    assert_api_slot!(status_proto, 84);
    assert_api_slot!(get_state_label, 85);
    assert_api_slot!(delete_candidate, 86);
    assert_api_slot!(delete_candidate_on_current_page, 87);
    assert_api_slot!(get_state_label_abbreviated, 88);
    assert_api_slot!(set_input, 89);
    assert_api_slot!(get_shared_data_dir_s, 90);
    assert_api_slot!(get_user_data_dir_s, 91);
    assert_api_slot!(get_prebuilt_data_dir_s, 92);
    assert_api_slot!(get_staging_dir_s, 93);
    assert_api_slot!(get_sync_dir_s, 94);
    assert_api_slot!(highlight_candidate, 95);
    assert_api_slot!(highlight_candidate_on_current_page, 96);
    assert_api_slot!(change_page, 97);
    assert_eq!(
        std::mem::size_of::<RimeApi>(),
        align_up(table_start + fn_size * 98, fn_align)
    );
    assert_eq!(
        api.data_size,
        (std::mem::size_of::<RimeApi>() - std::mem::size_of::<c_int>()) as c_int
    );

    let levers_api = unsafe { &*rime_levers_get_api().cast::<RimeLeversApi>() };
    macro_rules! assert_levers_slot {
        ($field:ident, $index:expr) => {
            assert_eq!(
                field_offset(levers_api, std::ptr::addr_of!((*levers_api).$field)),
                table_start + fn_size * $index,
                "RimeLeversApi.{} must match librime rime_levers_api.h slot {}",
                stringify!($field),
                $index
            );
        };
    }

    assert_eq!(
        field_offset(levers_api, std::ptr::addr_of!((*levers_api).data_size)),
        0
    );
    assert_levers_slot!(custom_settings_init, 0);
    assert_levers_slot!(custom_settings_destroy, 1);
    assert_levers_slot!(load_settings, 2);
    assert_levers_slot!(save_settings, 3);
    assert_levers_slot!(customize_bool, 4);
    assert_levers_slot!(customize_int, 5);
    assert_levers_slot!(customize_double, 6);
    assert_levers_slot!(customize_string, 7);
    assert_levers_slot!(is_first_run, 8);
    assert_levers_slot!(settings_is_modified, 9);
    assert_levers_slot!(settings_get_config, 10);
    assert_levers_slot!(switcher_settings_init, 11);
    assert_levers_slot!(get_available_schema_list, 12);
    assert_levers_slot!(get_selected_schema_list, 13);
    assert_levers_slot!(schema_list_destroy, 14);
    assert_levers_slot!(get_schema_id, 15);
    assert_levers_slot!(get_schema_name, 16);
    assert_levers_slot!(get_schema_version, 17);
    assert_levers_slot!(get_schema_author, 18);
    assert_levers_slot!(get_schema_description, 19);
    assert_levers_slot!(get_schema_file_path, 20);
    assert_levers_slot!(select_schemas, 21);
    assert_levers_slot!(get_hotkeys, 22);
    assert_levers_slot!(set_hotkeys, 23);
    assert_levers_slot!(user_dict_iterator_init, 24);
    assert_levers_slot!(user_dict_iterator_destroy, 25);
    assert_levers_slot!(next_user_dict, 26);
    assert_levers_slot!(backup_user_dict, 27);
    assert_levers_slot!(restore_user_dict, 28);
    assert_levers_slot!(export_user_dict, 29);
    assert_levers_slot!(import_user_dict, 30);
    assert_levers_slot!(customize_item, 31);
    assert_eq!(
        std::mem::size_of::<RimeLeversApi>(),
        align_up(table_start + fn_size * 32, fn_align)
    );
    assert_eq!(
        levers_api.data_size,
        (std::mem::size_of::<RimeLeversApi>() - std::mem::size_of::<c_int>()) as c_int
    );
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
    let linefeed = CString::new("Linefeed").expect("key name should be valid");
    let clear = CString::new("Clear").expect("key name should be valid");
    let pause = CString::new("Pause").expect("key name should be valid");
    let sys_req = CString::new("Sys_Req").expect("key name should be valid");
    let left = CString::new("Left").expect("key name should be valid");
    let prior = CString::new("Prior").expect("key name should be valid");
    let next = CString::new("Next").expect("key name should be valid");
    let begin = CString::new("Begin").expect("key name should be valid");
    let cancel = CString::new("Cancel").expect("key name should be valid");
    let break_key = CString::new("Break").expect("key name should be valid");
    let hebrew_switch = CString::new("Hebrew_switch").expect("key name should be valid");
    let mode_switch = CString::new("Mode_switch").expect("key name should be valid");
    let num_lock = CString::new("Num_Lock").expect("key name should be valid");
    let kp_enter = CString::new("KP_Enter").expect("key name should be valid");
    let kp_page_up = CString::new("KP_Page_Up").expect("key name should be valid");
    let kp_prior = CString::new("KP_Prior").expect("key name should be valid");
    let kp_page_down = CString::new("KP_Page_Down").expect("key name should be valid");
    let kp_next = CString::new("KP_Next").expect("key name should be valid");
    let kp_9 = CString::new("KP_9").expect("key name should be valid");
    let kp_equal = CString::new("KP_Equal").expect("key name should be valid");
    let f1 = CString::new("F1").expect("key name should be valid");
    let f12 = CString::new("F12").expect("key name should be valid");
    let f13 = CString::new("F13").expect("key name should be valid");
    let f24 = CString::new("F24").expect("key name should be valid");
    let f35 = CString::new("F35").expect("key name should be valid");
    let shift_l = CString::new("Shift_L").expect("key name should be valid");
    let control_r = CString::new("Control_R").expect("key name should be valid");
    let caps_lock = CString::new("Caps_Lock").expect("key name should be valid");
    let alt_l = CString::new("Alt_L").expect("key name should be valid");
    let hyper_r = CString::new("Hyper_R").expect("key name should be valid");
    let void_symbol = CString::new("VoidSymbol").expect("key name should be valid");
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
    let kappa = CString::new("kappa").expect("key name should be valid");
    let kra = CString::new("kra").expect("key name should be valid");
    let rcedilla = CString::new("Rcedilla").expect("key name should be valid");
    let eng = CString::new("ENG").expect("key name should be valid");
    let umacron = CString::new("umacron").expect("key name should be valid");
    let overline = CString::new("overline").expect("key name should be valid");
    let kana_fullstop = CString::new("kana_fullstop").expect("key name should be valid");
    let kana_middledot = CString::new("kana_middledot").expect("key name should be valid");
    let kana_tu = CString::new("kana_tu").expect("key name should be valid");
    let kana_chi = CString::new("kana_CHI").expect("key name should be valid");
    let kana_ti = CString::new("kana_TI").expect("key name should be valid");
    let kana_hu = CString::new("kana_HU").expect("key name should be valid");
    let semivoicedsound = CString::new("semivoicedsound").expect("key name should be valid");
    let arabic_comma = CString::new("Arabic_comma").expect("key name should be valid");
    let arabic_hamza = CString::new("Arabic_hamza").expect("key name should be valid");
    let arabic_ha = CString::new("Arabic_ha").expect("key name should be valid");
    let arabic_heh = CString::new("Arabic_heh").expect("key name should be valid");
    let arabic_sukun = CString::new("Arabic_sukun").expect("key name should be valid");
    let serbian_dje = CString::new("Serbian_dje").expect("key name should be valid");
    let ukrainian_ie = CString::new("Ukrainian_ie").expect("key name should be valid");
    let ukranian_je = CString::new("Ukranian_je").expect("key name should be valid");
    let cyrillic_je = CString::new("Cyrillic_je").expect("key name should be valid");
    let serbian_je = CString::new("Serbian_je").expect("key name should be valid");
    let byelorussian_shortu =
        CString::new("Byelorussian_shortu").expect("key name should be valid");
    let cyrillic_dzhe = CString::new("Cyrillic_dzhe").expect("key name should be valid");
    let serbian_dze = CString::new("Serbian_dze").expect("key name should be valid");
    let cyrillic_yu = CString::new("Cyrillic_yu").expect("key name should be valid");
    let cyrillic_ha = CString::new("Cyrillic_ha").expect("key name should be valid");
    let cyrillic_hardsign = CString::new("Cyrillic_hardsign").expect("key name should be valid");
    let cyrillic_yu_upper = CString::new("Cyrillic_YU").expect("key name should be valid");
    let cyrillic_hardsign_upper =
        CString::new("Cyrillic_HARDSIGN").expect("key name should be valid");
    let greek_alphaaccent = CString::new("Greek_ALPHAaccent").expect("key name should be valid");
    let greek_iotadieresis = CString::new("Greek_IOTAdieresis").expect("key name should be valid");
    let greek_iotadiaeresis =
        CString::new("Greek_IOTAdiaeresis").expect("key name should be valid");
    let greek_lambda_upper = CString::new("Greek_LAMBDA").expect("key name should be valid");
    let greek_lamda_upper = CString::new("Greek_LAMDA").expect("key name should be valid");
    let greek_omega_upper = CString::new("Greek_OMEGA").expect("key name should be valid");
    let greek_lambda = CString::new("Greek_lambda").expect("key name should be valid");
    let greek_lamda = CString::new("Greek_lamda").expect("key name should be valid");
    let greek_finalsmallsigma =
        CString::new("Greek_finalsmallsigma").expect("key name should be valid");
    let greek_omega = CString::new("Greek_omega").expect("key name should be valid");
    let leftradical = CString::new("leftradical").expect("key name should be valid");
    let topvertsummationconnector =
        CString::new("topvertsummationconnector").expect("key name should be valid");
    let lessthanequal = CString::new("lessthanequal").expect("key name should be valid");
    let infinity = CString::new("infinity").expect("key name should be valid");
    let leftarrow = CString::new("leftarrow").expect("key name should be valid");
    let blank = CString::new("blank").expect("key name should be valid");
    let lowrightcorner = CString::new("lowrightcorner").expect("key name should be valid");
    let vertbar = CString::new("vertbar").expect("key name should be valid");
    let emspace = CString::new("emspace").expect("key name should be valid");
    let ellipsis = CString::new("ellipsis").expect("key name should be valid");
    let trademark = CString::new("trademark").expect("key name should be valid");
    let leftsinglequotemark =
        CString::new("leftsinglequotemark").expect("key name should be valid");
    let dagger = CString::new("dagger").expect("key name should be valid");
    let cursor = CString::new("cursor").expect("key name should be valid");
    let leftcaret = CString::new("leftcaret").expect("key name should be valid");
    let overbar = CString::new("overbar").expect("key name should be valid");
    let circle = CString::new("circle").expect("key name should be valid");
    let righttack = CString::new("righttack").expect("key name should be valid");
    let hebrew_doublelowline =
        CString::new("hebrew_doublelowline").expect("key name should be valid");
    let hebrew_aleph = CString::new("hebrew_aleph").expect("key name should be valid");
    let hebrew_beth = CString::new("hebrew_beth").expect("key name should be valid");
    let hebrew_samekh = CString::new("hebrew_samekh").expect("key name should be valid");
    let hebrew_finalzadi = CString::new("hebrew_finalzadi").expect("key name should be valid");
    let hebrew_qoph = CString::new("hebrew_qoph").expect("key name should be valid");
    let hebrew_taw = CString::new("hebrew_taw").expect("key name should be valid");
    let thai_kokai = CString::new("Thai_kokai").expect("key name should be valid");
    let thai_dodek = CString::new("Thai_dodek").expect("key name should be valid");
    let thai_sarauu = CString::new("Thai_sarauu").expect("key name should be valid");
    let thai_maihanakat_maitho =
        CString::new("Thai_maihanakat_maitho").expect("key name should be valid");
    let thai_baht = CString::new("Thai_baht").expect("key name should be valid");
    let thai_leksun = CString::new("Thai_leksun").expect("key name should be valid");
    let thai_lekkao = CString::new("Thai_lekkao").expect("key name should be valid");
    let hangul_kiyeog = CString::new("Hangul_Kiyeog").expect("key name should be valid");
    let hangul_hieuh = CString::new("Hangul_Hieuh").expect("key name should be valid");
    let hangul_a = CString::new("Hangul_A").expect("key name should be valid");
    let hangul_i = CString::new("Hangul_I").expect("key name should be valid");
    let hangul_j_kiyeog = CString::new("Hangul_J_Kiyeog").expect("key name should be valid");
    let hangul_j_hieuh = CString::new("Hangul_J_Hieuh").expect("key name should be valid");
    let hangul_sunkyeongeumpieub =
        CString::new("Hangul_SunkyeongeumPieub").expect("key name should be valid");
    let hangul_j_yeorinhieuh =
        CString::new("Hangul_J_YeorinHieuh").expect("key name should be valid");
    let korean_won = CString::new("Korean_Won").expect("key name should be valid");
    let oe_upper = CString::new("OE").expect("key name should be valid");
    let oe_lower = CString::new("oe").expect("key name should be valid");
    let ydiaeresis_upper = CString::new("Ydiaeresis").expect("key name should be valid");
    let ecu_sign = CString::new("EcuSign").expect("key name should be valid");
    let rupee_sign = CString::new("RupeeSign").expect("key name should be valid");
    let euro_sign = CString::new("EuroSign").expect("key name should be valid");
    let ibm_3270_duplicate = CString::new("3270_Duplicate").expect("key name should be valid");
    let ibm_3270_erase_input = CString::new("3270_EraseInput").expect("key name should be valid");
    let ibm_3270_cursor_blink = CString::new("3270_CursorBlink").expect("key name should be valid");
    let ibm_3270_enter = CString::new("3270_Enter").expect("key name should be valid");
    let iso_lock = CString::new("ISO_Lock").expect("key name should be valid");
    let iso_level3_shift = CString::new("ISO_Level3_Shift").expect("key name should be valid");
    let iso_level5_shift = CString::new("ISO_Level5_Shift").expect("key name should be valid");
    let iso_last_group_lock =
        CString::new("ISO_Last_Group_Lock").expect("key name should be valid");
    let iso_left_tab = CString::new("ISO_Left_Tab").expect("key name should be valid");
    let iso_fast_cursor_down =
        CString::new("ISO_Fast_Cursor_Down").expect("key name should be valid");
    let iso_enter = CString::new("ISO_Enter").expect("key name should be valid");
    let dead_grave = CString::new("dead_grave").expect("key name should be valid");
    let dead_horn = CString::new("dead_horn").expect("key name should be valid");
    let dead_stroke = CString::new("dead_stroke").expect("key name should be valid");
    let accessx_enable = CString::new("AccessX_Enable").expect("key name should be valid");
    let audible_bell_enable = CString::new("AudibleBell_Enable").expect("key name should be valid");
    let first_virtual_screen =
        CString::new("First_Virtual_Screen").expect("key name should be valid");
    let pointer_left = CString::new("Pointer_Left").expect("key name should be valid");
    let pointer_button_dflt =
        CString::new("Pointer_Button_Dflt").expect("key name should be valid");
    let pointer_dblclick5 = CString::new("Pointer_DblClick5").expect("key name should be valid");
    let pointer_enable_keys = CString::new("Pointer_EnableKeys").expect("key name should be valid");
    let pointer_dflt_btn_prev =
        CString::new("Pointer_DfltBtnPrev").expect("key name should be valid");
    let pointer_drag5 = CString::new("Pointer_Drag5").expect("key name should be valid");
    let multi_key = CString::new("Multi_key").expect("key name should be valid");
    let henkan = CString::new("Henkan").expect("key name should be valid");
    let henkan_mode = CString::new("Henkan_Mode").expect("key name should be valid");
    let hiragana_katakana = CString::new("Hiragana_Katakana").expect("key name should be valid");
    let eisu_toggle = CString::new("Eisu_toggle").expect("key name should be valid");
    let hangul = CString::new("Hangul").expect("key name should be valid");
    let hangul_romaja = CString::new("Hangul_Romaja").expect("key name should be valid");
    let codeinput = CString::new("Codeinput").expect("key name should be valid");
    let multiple_candidate = CString::new("MultipleCandidate").expect("key name should be valid");
    let hangul_special = CString::new("Hangul_Special").expect("key name should be valid");
    let missing = CString::new("NoSuchKey").expect("key name should be valid");

    assert_eq!(unsafe { RimeGetKeycodeByName(space.as_ptr()) }, 0x20);
    assert_eq!(unsafe { RimeGetKeycodeByName(backspace.as_ptr()) }, 0xff08);
    assert_eq!(unsafe { RimeGetKeycodeByName(linefeed.as_ptr()) }, 0xff0a);
    assert_eq!(unsafe { RimeGetKeycodeByName(clear.as_ptr()) }, 0xff0b);
    assert_eq!(unsafe { RimeGetKeycodeByName(pause.as_ptr()) }, 0xff13);
    assert_eq!(unsafe { RimeGetKeycodeByName(sys_req.as_ptr()) }, 0xff15);
    assert_eq!(unsafe { RimeGetKeycodeByName(left.as_ptr()) }, 0xff51);
    assert_eq!(unsafe { RimeGetKeycodeByName(prior.as_ptr()) }, 0xff55);
    assert_eq!(unsafe { RimeGetKeycodeByName(next.as_ptr()) }, 0xff56);
    assert_eq!(unsafe { RimeGetKeycodeByName(begin.as_ptr()) }, 0xff58);
    assert_eq!(unsafe { RimeGetKeycodeByName(cancel.as_ptr()) }, 0xff69);
    assert_eq!(unsafe { RimeGetKeycodeByName(break_key.as_ptr()) }, 0xff6b);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_switch.as_ptr()) },
        0xff7e
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(mode_switch.as_ptr()) },
        0xff7e
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(num_lock.as_ptr()) }, 0xff7f);
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
    assert_eq!(unsafe { RimeGetKeycodeByName(f13.as_ptr()) }, 0xffca);
    assert_eq!(unsafe { RimeGetKeycodeByName(f24.as_ptr()) }, 0xffd5);
    assert_eq!(unsafe { RimeGetKeycodeByName(f35.as_ptr()) }, 0xffe0);
    assert_eq!(unsafe { RimeGetKeycodeByName(shift_l.as_ptr()) }, 0xffe1);
    assert_eq!(unsafe { RimeGetKeycodeByName(control_r.as_ptr()) }, 0xffe4);
    assert_eq!(unsafe { RimeGetKeycodeByName(caps_lock.as_ptr()) }, 0xffe5);
    assert_eq!(unsafe { RimeGetKeycodeByName(alt_l.as_ptr()) }, 0xffe9);
    assert_eq!(unsafe { RimeGetKeycodeByName(hyper_r.as_ptr()) }, 0xffee);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(void_symbol.as_ptr()) },
        0x00ff_ffff
    );
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
    assert_eq!(unsafe { RimeGetKeycodeByName(kappa.as_ptr()) }, 0x3a2);
    assert_eq!(unsafe { RimeGetKeycodeByName(kra.as_ptr()) }, 0x3a2);
    assert_eq!(unsafe { RimeGetKeycodeByName(rcedilla.as_ptr()) }, 0x3a3);
    assert_eq!(unsafe { RimeGetKeycodeByName(eng.as_ptr()) }, 0x3bd);
    assert_eq!(unsafe { RimeGetKeycodeByName(umacron.as_ptr()) }, 0x3fe);
    assert_eq!(unsafe { RimeGetKeycodeByName(overline.as_ptr()) }, 0x47e);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(kana_fullstop.as_ptr()) },
        0x4a1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(kana_middledot.as_ptr()) },
        0x4a5
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(kana_tu.as_ptr()) }, 0x4af);
    assert_eq!(unsafe { RimeGetKeycodeByName(kana_chi.as_ptr()) }, 0x4c1);
    assert_eq!(unsafe { RimeGetKeycodeByName(kana_ti.as_ptr()) }, 0x4c1);
    assert_eq!(unsafe { RimeGetKeycodeByName(kana_hu.as_ptr()) }, 0x4cc);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(semivoicedsound.as_ptr()) },
        0x4df
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(arabic_comma.as_ptr()) },
        0x5ac
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(arabic_hamza.as_ptr()) },
        0x5c1
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(arabic_ha.as_ptr()) }, 0x5e7);
    assert_eq!(unsafe { RimeGetKeycodeByName(arabic_heh.as_ptr()) }, 0x5e7);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(arabic_sukun.as_ptr()) },
        0x5f2
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(serbian_dje.as_ptr()) }, 0x6a1);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ukrainian_ie.as_ptr()) },
        0x6a4
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(ukranian_je.as_ptr()) }, 0x6a4);
    assert_eq!(unsafe { RimeGetKeycodeByName(cyrillic_je.as_ptr()) }, 0x6a8);
    assert_eq!(unsafe { RimeGetKeycodeByName(serbian_je.as_ptr()) }, 0x6a8);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(byelorussian_shortu.as_ptr()) },
        0x6ae
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(cyrillic_dzhe.as_ptr()) },
        0x6af
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(serbian_dze.as_ptr()) }, 0x6af);
    assert_eq!(unsafe { RimeGetKeycodeByName(cyrillic_yu.as_ptr()) }, 0x6c0);
    assert_eq!(unsafe { RimeGetKeycodeByName(cyrillic_ha.as_ptr()) }, 0x6c8);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(cyrillic_hardsign.as_ptr()) },
        0x6df
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(cyrillic_yu_upper.as_ptr()) },
        0x6e0
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(cyrillic_hardsign_upper.as_ptr()) },
        0x6ff
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_alphaaccent.as_ptr()) },
        0x7a1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_iotadieresis.as_ptr()) },
        0x7a5
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_iotadiaeresis.as_ptr()) },
        0x7a5
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_lambda_upper.as_ptr()) },
        0x7cb
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_lamda_upper.as_ptr()) },
        0x7cb
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_omega_upper.as_ptr()) },
        0x7d9
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_lambda.as_ptr()) },
        0x7eb
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(greek_lamda.as_ptr()) }, 0x7eb);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_finalsmallsigma.as_ptr()) },
        0x7f3
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(greek_omega.as_ptr()) }, 0x7f9);
    assert_eq!(unsafe { RimeGetKeycodeByName(leftradical.as_ptr()) }, 0x8a1);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(topvertsummationconnector.as_ptr()) },
        0x8b3
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(lessthanequal.as_ptr()) },
        0x8bc
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(infinity.as_ptr()) }, 0x8c2);
    assert_eq!(unsafe { RimeGetKeycodeByName(leftarrow.as_ptr()) }, 0x8fb);
    assert_eq!(unsafe { RimeGetKeycodeByName(blank.as_ptr()) }, 0x9df);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(lowrightcorner.as_ptr()) },
        0x9ea
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(vertbar.as_ptr()) }, 0x9f8);
    assert_eq!(unsafe { RimeGetKeycodeByName(emspace.as_ptr()) }, 0xaa1);
    assert_eq!(unsafe { RimeGetKeycodeByName(ellipsis.as_ptr()) }, 0xaae);
    assert_eq!(unsafe { RimeGetKeycodeByName(trademark.as_ptr()) }, 0xac9);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(leftsinglequotemark.as_ptr()) },
        0xad0
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(dagger.as_ptr()) }, 0xaf1);
    assert_eq!(unsafe { RimeGetKeycodeByName(cursor.as_ptr()) }, 0xaff);
    assert_eq!(unsafe { RimeGetKeycodeByName(leftcaret.as_ptr()) }, 0xba3);
    assert_eq!(unsafe { RimeGetKeycodeByName(overbar.as_ptr()) }, 0xbc0);
    assert_eq!(unsafe { RimeGetKeycodeByName(circle.as_ptr()) }, 0xbcf);
    assert_eq!(unsafe { RimeGetKeycodeByName(righttack.as_ptr()) }, 0xbfc);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_doublelowline.as_ptr()) },
        0xcdf
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_aleph.as_ptr()) },
        0xce0
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(hebrew_beth.as_ptr()) }, 0xce1);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_samekh.as_ptr()) },
        0xcf1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_finalzadi.as_ptr()) },
        0xcf5
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(hebrew_qoph.as_ptr()) }, 0xcf7);
    assert_eq!(unsafe { RimeGetKeycodeByName(hebrew_taw.as_ptr()) }, 0xcfa);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_kokai.as_ptr()) }, 0xda1);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_dodek.as_ptr()) }, 0xdb4);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_sarauu.as_ptr()) }, 0xdd9);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(thai_maihanakat_maitho.as_ptr()) },
        0xdde
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_baht.as_ptr()) }, 0xddf);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_leksun.as_ptr()) }, 0xdf0);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_lekkao.as_ptr()) }, 0xdf9);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_kiyeog.as_ptr()) },
        0xea1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_hieuh.as_ptr()) },
        0xebe
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(hangul_a.as_ptr()) }, 0xebf);
    assert_eq!(unsafe { RimeGetKeycodeByName(hangul_i.as_ptr()) }, 0xed3);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_j_kiyeog.as_ptr()) },
        0xed4
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_j_hieuh.as_ptr()) },
        0xeee
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_sunkyeongeumpieub.as_ptr()) },
        0xef1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_j_yeorinhieuh.as_ptr()) },
        0xefa
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(korean_won.as_ptr()) }, 0xeff);
    assert_eq!(unsafe { RimeGetKeycodeByName(oe_upper.as_ptr()) }, 0x13bc);
    assert_eq!(unsafe { RimeGetKeycodeByName(oe_lower.as_ptr()) }, 0x13bd);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ydiaeresis_upper.as_ptr()) },
        0x13be
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(ecu_sign.as_ptr()) }, 0x20a0);
    assert_eq!(unsafe { RimeGetKeycodeByName(rupee_sign.as_ptr()) }, 0x20a8);
    assert_eq!(unsafe { RimeGetKeycodeByName(euro_sign.as_ptr()) }, 0x20ac);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ibm_3270_duplicate.as_ptr()) },
        0xfd01
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ibm_3270_erase_input.as_ptr()) },
        0xfd07
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ibm_3270_cursor_blink.as_ptr()) },
        0xfd0f
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ibm_3270_enter.as_ptr()) },
        0xfd1e
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(iso_lock.as_ptr()) }, 0xfe01);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_level3_shift.as_ptr()) },
        0xfe03
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_level5_shift.as_ptr()) },
        0x00ff_ffff
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_last_group_lock.as_ptr()) },
        0xfe0f
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_left_tab.as_ptr()) },
        0xfe20
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_fast_cursor_down.as_ptr()) },
        0xfe2f
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(iso_enter.as_ptr()) }, 0xfe34);
    assert_eq!(unsafe { RimeGetKeycodeByName(dead_grave.as_ptr()) }, 0xfe50);
    assert_eq!(unsafe { RimeGetKeycodeByName(dead_horn.as_ptr()) }, 0xfe62);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(dead_stroke.as_ptr()) },
        0x00ff_ffff
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(accessx_enable.as_ptr()) },
        0xfe70
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(audible_bell_enable.as_ptr()) },
        0xfe7a
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(first_virtual_screen.as_ptr()) },
        0xfed0
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_left.as_ptr()) },
        0xfee0
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_button_dflt.as_ptr()) },
        0xfee8
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_dblclick5.as_ptr()) },
        0xfef3
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_enable_keys.as_ptr()) },
        0xfef9
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_dflt_btn_prev.as_ptr()) },
        0xfefc
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_drag5.as_ptr()) },
        0xfefd
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(multi_key.as_ptr()) }, 0xff20);
    assert_eq!(unsafe { RimeGetKeycodeByName(henkan.as_ptr()) }, 0xff23);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(henkan_mode.as_ptr()) },
        0xff23
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hiragana_katakana.as_ptr()) },
        0xff27
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(eisu_toggle.as_ptr()) },
        0xff30
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(hangul.as_ptr()) }, 0xff31);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_romaja.as_ptr()) },
        0xff36
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(codeinput.as_ptr()) }, 0xff37);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(multiple_candidate.as_ptr()) },
        0xff3d
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_special.as_ptr()) },
        0xff3f
    );
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
        static_c_string(RimeGetKeyName(0xff0a)).as_deref(),
        Some("Linefeed")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff0b)).as_deref(),
        Some("Clear")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff13)).as_deref(),
        Some("Pause")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff15)).as_deref(),
        Some("Sys_Req")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff51)).as_deref(),
        Some("Left")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff55)).as_deref(),
        Some("Page_Up")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff56)).as_deref(),
        Some("Next")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff58)).as_deref(),
        Some("Begin")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff60)).as_deref(),
        Some("Select")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff69)).as_deref(),
        Some("Cancel")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff6b)).as_deref(),
        Some("Break")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff7e)).as_deref(),
        Some("Arabic_switch")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff7f)).as_deref(),
        Some("Num_Lock")
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
        static_c_string(RimeGetKeyName(0xffca)).as_deref(),
        Some("F13")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffd5)).as_deref(),
        Some("F24")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe0)).as_deref(),
        Some("F35")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe1)).as_deref(),
        Some("Shift_L")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe4)).as_deref(),
        Some("Control_R")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe5)).as_deref(),
        Some("Caps_Lock")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe9)).as_deref(),
        Some("Alt_L")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffee)).as_deref(),
        Some("Hyper_R")
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
    assert_eq!(
        static_c_string(RimeGetKeyName(0x3a2)).as_deref(),
        Some("kappa")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x3a3)).as_deref(),
        Some("Rcedilla")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x3bd)).as_deref(),
        Some("ENG")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x3fe)).as_deref(),
        Some("umacron")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x47e)).as_deref(),
        Some("overline")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4a1)).as_deref(),
        Some("kana_fullstop")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4a5)).as_deref(),
        Some("kana_conjunctive")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4af)).as_deref(),
        Some("kana_tsu")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4c1)).as_deref(),
        Some("kana_CHI")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4cc)).as_deref(),
        Some("kana_FU")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4df)).as_deref(),
        Some("semivoicedsound")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x5ac)).as_deref(),
        Some("Arabic_comma")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x5c1)).as_deref(),
        Some("Arabic_hamza")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x5e7)).as_deref(),
        Some("Arabic_ha")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x5f2)).as_deref(),
        Some("Arabic_sukun")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6a1)).as_deref(),
        Some("Serbian_dje")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6a4)).as_deref(),
        Some("Ukrainian_ie")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6a8)).as_deref(),
        Some("Cyrillic_je")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6ae)).as_deref(),
        Some("Byelorussian_shortu")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6af)).as_deref(),
        Some("Cyrillic_dzhe")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6c0)).as_deref(),
        Some("Cyrillic_yu")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6df)).as_deref(),
        Some("Cyrillic_hardsign")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6e0)).as_deref(),
        Some("Cyrillic_YU")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6ff)).as_deref(),
        Some("Cyrillic_HARDSIGN")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7a1)).as_deref(),
        Some("Greek_ALPHAaccent")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7a5)).as_deref(),
        Some("Greek_IOTAdiaeresis")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7cb)).as_deref(),
        Some("Greek_LAMBDA")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7d9)).as_deref(),
        Some("Greek_OMEGA")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7eb)).as_deref(),
        Some("Greek_lambda")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7f3)).as_deref(),
        Some("Greek_finalsmallsigma")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7f9)).as_deref(),
        Some("Greek_omega")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8a1)).as_deref(),
        Some("leftradical")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8b3)).as_deref(),
        Some("topvertsummationconnector")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8bc)).as_deref(),
        Some("lessthanequal")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8c2)).as_deref(),
        Some("infinity")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8fb)).as_deref(),
        Some("leftarrow")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x9df)).as_deref(),
        Some("blank")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x9ea)).as_deref(),
        Some("lowrightcorner")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x9f8)).as_deref(),
        Some("vertbar")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xaa1)).as_deref(),
        Some("emspace")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xaae)).as_deref(),
        Some("ellipsis")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xac9)).as_deref(),
        Some("trademark")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xad0)).as_deref(),
        Some("leftsinglequotemark")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xaf1)).as_deref(),
        Some("dagger")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xaff)).as_deref(),
        Some("cursor")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xba3)).as_deref(),
        Some("leftcaret")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xbc0)).as_deref(),
        Some("overbar")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xbcf)).as_deref(),
        Some("circle")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xbfc)).as_deref(),
        Some("righttack")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcdf)).as_deref(),
        Some("hebrew_doublelowline")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xce0)).as_deref(),
        Some("hebrew_aleph")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xce1)).as_deref(),
        Some("hebrew_bet")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcf1)).as_deref(),
        Some("hebrew_samech")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcf5)).as_deref(),
        Some("hebrew_finalzade")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcf7)).as_deref(),
        Some("hebrew_kuf")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcfa)).as_deref(),
        Some("hebrew_taf")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xda1)).as_deref(),
        Some("Thai_kokai")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdb4)).as_deref(),
        Some("Thai_dodek")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdd9)).as_deref(),
        Some("Thai_sarauu")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdde)).as_deref(),
        Some("Thai_maihanakat_maitho")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xddf)).as_deref(),
        Some("Thai_baht")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdf0)).as_deref(),
        Some("Thai_leksun")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdf9)).as_deref(),
        Some("Thai_lekkao")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xea1)).as_deref(),
        Some("Hangul_Kiyeog")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xebe)).as_deref(),
        Some("Hangul_Hieuh")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xebf)).as_deref(),
        Some("Hangul_A")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xed3)).as_deref(),
        Some("Hangul_I")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xed4)).as_deref(),
        Some("Hangul_J_Kiyeog")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xeee)).as_deref(),
        Some("Hangul_J_Hieuh")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xef1)).as_deref(),
        Some("Hangul_SunkyeongeumPieub")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xefa)).as_deref(),
        Some("Hangul_J_YeorinHieuh")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xeff)).as_deref(),
        Some("Korean_Won")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x13bc)).as_deref(),
        Some("OE")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x13bd)).as_deref(),
        Some("oe")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x13be)).as_deref(),
        Some("Ydiaeresis")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x20a0)).as_deref(),
        Some("EcuSign")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x20a8)).as_deref(),
        Some("RupeeSign")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x20ac)).as_deref(),
        Some("EuroSign")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfd01)).as_deref(),
        Some("3270_Duplicate")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfd07)).as_deref(),
        Some("3270_EraseInput")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfd0f)).as_deref(),
        Some("3270_CursorBlink")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfd1e)).as_deref(),
        Some("3270_Enter")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe01)).as_deref(),
        Some("ISO_Lock")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe03)).as_deref(),
        Some("ISO_Level3_Shift")
    );
    assert_eq!(static_c_string(RimeGetKeyName(0xfe11)), None);
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe0f)).as_deref(),
        Some("ISO_Last_Group_Lock")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe20)).as_deref(),
        Some("ISO_Left_Tab")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe2f)).as_deref(),
        Some("ISO_Fast_Cursor_Down")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe34)).as_deref(),
        Some("ISO_Enter")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe50)).as_deref(),
        Some("dead_grave")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe62)).as_deref(),
        Some("dead_horn")
    );
    assert_eq!(static_c_string(RimeGetKeyName(0xfe63)), None);
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe70)).as_deref(),
        Some("AccessX_Enable")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe7a)).as_deref(),
        Some("AudibleBell_Enable")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfed0)).as_deref(),
        Some("First_Virtual_Screen")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfed5)).as_deref(),
        Some("Terminate_Server")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfee0)).as_deref(),
        Some("Pointer_Left")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfee8)).as_deref(),
        Some("Pointer_Button_Dflt")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfef3)).as_deref(),
        Some("Pointer_DblClick5")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfef9)).as_deref(),
        Some("Pointer_EnableKeys")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfefc)).as_deref(),
        Some("Pointer_DfltBtnPrev")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfefd)).as_deref(),
        Some("Pointer_Drag5")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff20)).as_deref(),
        Some("Multi_key")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff23)).as_deref(),
        Some("Henkan")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff27)).as_deref(),
        Some("Hiragana_Katakana")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff30)).as_deref(),
        Some("Eisu_toggle")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff31)).as_deref(),
        Some("Hangul")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff36)).as_deref(),
        Some("Hangul_Romaja")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff37)).as_deref(),
        Some("Codeinput")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff3d)).as_deref(),
        Some("MultipleCandidate")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff3f)).as_deref(),
        Some("Hangul_Special")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x00ff_ffff)).as_deref(),
        Some("VoidSymbol")
    );
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
fn config_load_string_initializes_before_librime_parse_failure() {
    let _guard = test_guard();
    let mut config = empty_config();
    let invalid_yaml = CString::new("schema: [unterminated").expect("yaml should be valid");
    let key = CString::new("schema/name").expect("key should be valid");
    let value = CString::new("Fallback").expect("value should be valid");
    let mut output = vec![0 as c_char; 16];

    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, invalid_yaml.as_ptr()) },
        FALSE
    );
    assert!(!config.ptr.is_null());
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(&mut config, key.as_ptr(), output.as_mut_ptr(), output.len())
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Fallback")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
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
    let modified_time =
        config_string(&mut config, "signature/modified_time").expect("signature time exists");
    assert_librime_ctime_shape(&modified_time);
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
    assert_eq!(iterator.index, 2);
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
        Ok("alternative_select_keys")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/alternative_select_keys")
    );
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("page_size")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/page_size")
    );
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 2);
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
fn config_paths_preserve_librime_empty_segments_after_leading_slashes() {
    let _guard = test_guard();
    let mut config = empty_config();
    let yaml = CString::new(
        r#"
foo:
  "":
    bar: empty
  bar: collapsed
"#,
    )
    .expect("yaml should be valid");
    let empty_segment = CString::new("foo//bar").expect("key should be valid");
    let collapsed = CString::new("foo/bar").expect("key should be valid");
    let leading_slash = CString::new("/foo//bar").expect("key should be valid");
    let triple_slash = CString::new("foo///bar").expect("key should be valid");
    let value = CString::new("written").expect("value should be valid");
    let mut output = vec![0 as c_char; 16];

    // SAFETY: config and YAML pointers are valid for the call.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                empty_segment.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("empty")
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                collapsed.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("collapsed")
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                leading_slash.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("empty")
    );

    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, triple_slash.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "foo///bar").as_deref(),
        Some("written")
    );
    assert_eq!(
        config_string(&mut config, "foo//bar").as_deref(),
        Some("empty")
    );
    assert_eq!(
        config_string(&mut config, "foo/bar").as_deref(),
        Some("collapsed")
    );
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
    let wrapped_hex = CString::new("wrapped_hex").expect("key should be valid");
    let invalid_int = CString::new("invalid_int").expect("key should be valid");
    let double_suffix = CString::new("double_suffix").expect("key should be valid");
    let exponent_suffix = CString::new("exponent_suffix").expect("key should be valid");
    let invalid_double = CString::new("invalid_double").expect("key should be valid");
    let mut int_output = 0;
    let mut double_output = 0.0;

    let yaml = CString::new(
            "\
decimal_suffix: '42abc'\nsigned_spaced: '  -7ms'\nmalformed_hex_suffix: '0x10tail'\nmalformed_hex_empty: '0x'\nspaced_hex: ' 0x10'\nwrapped_hex: '0xffffffff'\ninvalid_int: abc42\ndouble_suffix: '  2.5ms'\nexponent_suffix: '1e2hz'\ninvalid_double: hz1.5\n",
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
        unsafe { RimeConfigGetInt(&mut config, wrapped_hex.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, -1);
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
    // SAFETY: the raw byte view is bounded to the caller-owned buffer.
    let truncated_shared = unsafe {
        std::slice::from_raw_parts(short_buffer.as_ptr().cast::<u8>(), short_buffer.len())
    };
    assert_eq!(truncated_shared, b"/tmp/yune-");

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
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeInitialize(&traits) };
}

#[test]
fn setup_respects_rime_traits_data_size_for_newer_path_fields() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let shared = CString::new("/tmp/yune-shared-old-traits").expect("path should be valid");
    let user = CString::new("/tmp/yune-user-old-traits").expect("path should be valid");
    let prebuilt = CString::new("/tmp/yune-prebuilt-ignored").expect("path should be valid");
    let staging = CString::new("/tmp/yune-staging-ignored").expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared.as_ptr();
    traits.user_data_dir = user.as_ptr();
    traits.prebuilt_data_dir = prebuilt.as_ptr();
    traits.staging_dir = staging.as_ptr();
    traits.data_size = traits_data_size_before_prebuilt_data_dir();

    // SAFETY: traits points to valid storage and all strings live for the call.
    unsafe { RimeSetup(&traits) };

    // SAFETY: runtime path getters return stable process-owned C strings.
    let shared_dir = unsafe { CStr::from_ptr(RimeGetSharedDataDir()) };
    assert_eq!(shared_dir.to_str(), Ok("/tmp/yune-shared-old-traits"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_dir = unsafe { CStr::from_ptr(RimeGetUserDataDir()) };
    assert_eq!(user_dir.to_str(), Ok("/tmp/yune-user-old-traits"));
    // SAFETY: `prebuilt_data_dir` was outside `data_size`, so librime derives
    // it from the provided shared data directory.
    let prebuilt_dir = unsafe { CStr::from_ptr(RimeGetPrebuiltDataDir()) };
    assert_eq!(
        prebuilt_dir.to_str(),
        Ok("/tmp/yune-shared-old-traits/build")
    );
    // SAFETY: `staging_dir` was outside `data_size`, so librime derives it
    // from the provided user data directory.
    let staging_dir = unsafe { CStr::from_ptr(RimeGetStagingDir()) };
    assert_eq!(staging_dir.to_str(), Ok("/tmp/yune-user-old-traits/build"));

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
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
    let distribution_name = CString::new("Yune Test").expect("distribution name should be valid");
    let distribution_code = CString::new("yune-test").expect("distribution code should be valid");
    let distribution_version =
        CString::new("2026.04").expect("distribution version should be valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    traits.distribution_name = distribution_name.as_ptr();
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
        find_config_value(&metadata, "distribution_name").and_then(Value::as_str),
        Some("Yune Test")
    );
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
    let unknown_task = CString::new("no_such_task").expect("task should be valid");
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
    assert_eq!(RimeRunTask(unknown_task.as_ptr()), FALSE);
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
        shared.join("luna_pinyin.schema.yaml"),
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
    fs::write(
        user.join("ignored_missing_name.schema.yaml"),
        "schema:\n  schema_id: ignored_missing_name\n",
    )
    .expect("invalid schema config should be written");
    fs::write(
        user.join("terra_pinyin.schema.yaml"),
        "\
schema:
  schema_id: terra_pinyin
  name: Terra Pinyin
  version: ''
  description: ''
",
    )
    .expect("user schema config should be written");

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
    assert_eq!(schema_list.size, 2);
    // SAFETY: the levers API populated one schema-list item.
    let item = unsafe { *schema_list.list };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let schema_id = unsafe { CStr::from_ptr(item.schema_id) };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let name = unsafe { CStr::from_ptr(item.name) };
    assert_eq!(schema_id.to_str(), Ok("luna_pinyin"));
    assert_eq!(name.to_str(), Ok("Luna Pinyin"));
    assert!(!item.reserved.is_null());
    // SAFETY: the second item is in bounds because size is 2.
    let user_item = unsafe { *schema_list.list.add(1) };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let user_schema_id = unsafe { CStr::from_ptr(user_item.schema_id) };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let user_name = unsafe { CStr::from_ptr(user_item.name) };
    assert_eq!(user_schema_id.to_str(), Ok("terra_pinyin"));
    assert_eq!(user_name.to_str(), Ok("Terra Pinyin"));
    assert!(!user_item.reserved.is_null());

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
        shared.join("luna_pinyin.schema.yaml").to_string_lossy()
    );
    let user_schema_info = user_item.reserved.cast();
    // SAFETY: empty optional schema metadata should be exposed as null, matching
    // librime's schema-info getters.
    assert!(unsafe { get_schema_version(user_schema_info) }.is_null());
    // SAFETY: same as above.
    assert!(unsafe { get_schema_description(user_schema_info) }.is_null());
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
    fs::write(
        staging.join("default.yaml"),
        "\
schema_list:
  - schema: terra_pinyin
",
    )
    .expect("default config should be rewritten");
    let mut stale_selected_list = empty_schema_list();
    // SAFETY: existing settings keep their initialized selected-schema state.
    assert_eq!(
        unsafe { get_selected(settings, &mut stale_selected_list) },
        TRUE
    );
    assert_eq!(stale_selected_list.size, 2);
    // SAFETY: the levers API populated two selected schema-list items.
    let stale_first = unsafe { *stale_selected_list.list };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    assert_eq!(
        unsafe { CStr::from_ptr(stale_first.schema_id) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: stale_selected_list was populated by the levers API above.
    unsafe { destroy(&mut stale_selected_list) };
    fs::write(
        shared.join("new_schema.schema.yaml"),
        "\
schema:
  schema_id: new_schema
  name: New Schema
",
    )
    .expect("new schema config should be written");
    let mut stale_available_list = empty_schema_list();
    // SAFETY: existing settings keep their initialized available-schema state.
    assert_eq!(
        unsafe { get_available(settings, &mut stale_available_list) },
        TRUE
    );
    assert_eq!(stale_available_list.size, 2);
    // SAFETY: stale_available_list was populated by the levers API above.
    unsafe { destroy(&mut stale_available_list) };
    let new_settings = (api
        .switcher_settings_init
        .expect("switcher settings init should be available"))();
    assert!(!new_settings.is_null());
    let mut refreshed_available_list = empty_schema_list();
    // SAFETY: a new settings object sees the updated schema directory snapshot.
    assert_eq!(
        unsafe { get_available(new_settings, &mut refreshed_available_list) },
        TRUE
    );
    assert_eq!(refreshed_available_list.size, 3);
    // SAFETY: the second item is in bounds because size is 3 and shared-dir
    // schemas sort before user-dir schemas.
    let refreshed_available_second = unsafe { *refreshed_available_list.list.add(1) };
    // SAFETY: schema-list ids are valid NUL-terminated strings.
    assert_eq!(
        unsafe { CStr::from_ptr(refreshed_available_second.schema_id) }.to_str(),
        Ok("new_schema")
    );
    // SAFETY: refreshed_available_list was populated by the levers API above.
    unsafe { destroy(&mut refreshed_available_list) };
    let mut refreshed_selected_list = empty_schema_list();
    // SAFETY: a new settings object sees the updated deployed default config.
    assert_eq!(
        unsafe { get_selected(new_settings, &mut refreshed_selected_list) },
        TRUE
    );
    assert_eq!(refreshed_selected_list.size, 1);
    // SAFETY: the levers API populated one selected schema-list item.
    let refreshed_first = unsafe { *refreshed_selected_list.list };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    assert_eq!(
        unsafe { CStr::from_ptr(refreshed_first.schema_id) }.to_str(),
        Ok("terra_pinyin")
    );
    // SAFETY: refreshed_selected_list was populated by the levers API above.
    unsafe { destroy(&mut refreshed_selected_list) };
    // SAFETY: new_settings was allocated by this shim's switcher init function.
    unsafe { drop(Box::from_raw(new_settings)) };

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

    // SAFETY: librime's levers API treats negative counts like an empty
    // selection because its signed loop never executes.
    assert_eq!(
        unsafe { select_schemas(settings, std::ptr::null(), -1) },
        TRUE
    );
    let mut negative_count_selected_list = empty_schema_list();
    // SAFETY: settings and selected list output are valid.
    assert_eq!(
        unsafe { get_selected(settings, &mut negative_count_selected_list) },
        FALSE
    );
    assert_eq!(negative_count_selected_list.size, 0);
    assert!(negative_count_selected_list.list.is_null());

    // Restore a non-empty override before the remaining null-input checks.
    // SAFETY: settings, schema_ids, and each C string are valid for the call.
    assert_eq!(
        unsafe { select_schemas(settings, schema_ids.as_ptr(), schema_ids.len() as i32) },
        TRUE
    );
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
    assert_eq!(empty_iterator.i, 7);

    fs::write(user.join("cached.userdb"), "").expect("cached user dict should be written");
    let mut cached_iterator = super::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 0,
    };
    // SAFETY: iterator points to writable storage.
    assert_eq!(unsafe { iterator_init(&mut cached_iterator) }, TRUE);
    assert!(!cached_iterator.ptr.is_null());
    assert_eq!(cached_iterator.i, 0);
    fs::remove_file(user.join("cached.userdb")).expect("cached user dict should be removed");
    // SAFETY: librime leaves an existing iterator untouched when a re-scan
    // finds no user dictionaries.
    assert_eq!(unsafe { iterator_init(&mut cached_iterator) }, FALSE);
    assert!(!cached_iterator.ptr.is_null());
    assert_eq!(cached_iterator.i, 0);
    // SAFETY: cached_iterator still owns the previous snapshot.
    let cached = unsafe { next_user_dict(&mut cached_iterator) };
    assert!(!cached.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(unsafe { CStr::from_ptr(cached) }.to_str(), Ok("cached"));
    // SAFETY: cached_iterator was initialized by this shim.
    unsafe { iterator_destroy(&mut cached_iterator) };

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
    let customization_modified_time = find_config_value(&saved_root, "customization/modified_time")
        .and_then(Value::as_str)
        .expect("customization signature should include modified time");
    assert_librime_ctime_shape(customization_modified_time);

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
    fs::write(
        staging.join("default.yaml"),
        "\
switcher:
  hotkeys:
    - Alt+space
",
    )
    .expect("updated default config should be written");
    // SAFETY: settings is a valid pointer returned by the shim. librime keeps
    // the hotkeys loaded into the switcher settings object at init time.
    let original_hotkeys = unsafe { get_hotkeys(settings) };
    assert!(!original_hotkeys.is_null());
    // SAFETY: returned pointer is valid while the settings object is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(original_hotkeys) }.to_str(),
        Ok("Control+grave, F4")
    );
    let new_settings = super::RimeSwitcherSettingsInit();
    assert!(!new_settings.is_null());
    // SAFETY: new_settings is a valid pointer returned after the config update.
    let updated_hotkeys = unsafe { get_hotkeys(new_settings) };
    assert!(!updated_hotkeys.is_null());
    // SAFETY: returned pointer is valid while the new settings object is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(updated_hotkeys) }.to_str(),
        Ok("Alt+space")
    );
    // SAFETY: null settings are rejected without dereferencing.
    assert!(unsafe { get_hotkeys(std::ptr::null_mut()) }.is_null());

    let new_hotkeys = CString::new("Alt+space").expect("hotkeys should be valid");
    // SAFETY: settings and hotkeys are valid pointers; mutation is currently unsupported.
    assert_eq!(
        unsafe { set_hotkeys(settings, new_hotkeys.as_ptr()) },
        FALSE
    );

    // SAFETY: new_settings was allocated by this shim's switcher init function.
    unsafe { drop(Box::from_raw(new_settings)) };
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
    assert_eq!(RimeStartMaintenance(TRUE), TRUE);
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
fn sync_user_data_emits_librime_deploy_notifications() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("sync-notification-events");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(user.join("default.yaml"), "config_version: test\n")
        .expect("user config should be written");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
    let context_object = 0x6b_usize as *mut c_void;

    RimeSetNotificationHandler(Some(record_notification), context_object);
    assert_eq!(RimeSyncUserData(), TRUE);

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned");
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x6b,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "start".to_owned(),
            },
            NotificationEvent {
                context_object: 0x6b,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "success".to_owned(),
            },
        ]
    );
    drop(events);

    RimeSetNotificationHandler(None, std::ptr::null_mut());
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn finalize_clears_sessions_but_preserves_notification_handler() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    RimeSetNotificationHandler(None, std::ptr::null_mut());
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
    let context_object = 0x7c_usize as *mut c_void;
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");

    RimeSetNotificationHandler(Some(record_notification), context_object);
    let old_session_id = RimeCreateSession();
    assert_ne!(old_session_id, 0);
    RimeFinalize();
    assert_eq!(RimeFindSession(old_session_id), FALSE);
    assert_eq!(RimeCreateSession(), 0);

    let traits = empty_traits();
    // SAFETY: traits points to valid storage.
    unsafe { RimeInitialize(&traits) };
    let new_session_id = RimeCreateSession();
    assert_ne!(new_session_id, 0);
    // SAFETY: option is a valid NUL-terminated C string.
    unsafe { RimeSetOption(new_session_id, ascii_mode.as_ptr(), TRUE) };

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned");
    assert_eq!(
        *events,
        vec![NotificationEvent {
            context_object: 0x7c,
            session_id: new_session_id,
            message_type: "option".to_owned(),
            message_value: "ascii_mode".to_owned(),
        }]
    );
    drop(events);

    RimeSetNotificationHandler(None, std::ptr::null_mut());
    assert_eq!(RimeDestroySession(new_session_id), TRUE);
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
fn accumulates_unread_commit_text_like_librime_session() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    for ch in "ni hao ".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as i32, 0), TRUE);
    }

    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text` with a
    // valid NUL-terminated C string owned by the commit object.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("nihao"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rime_commit_clear_preserves_librime_struct_data_size() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let expected_data_size =
        (std::mem::size_of::<RimeCommit>() - std::mem::size_of::<i32>()) as i32;
    let mut commit = RimeCommit {
        data_size: expected_data_size,
        text: std::ptr::null_mut(),
    };

    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    assert_eq!(commit.data_size, expected_data_size);
    assert!(commit.text.is_null());

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    assert_eq!(commit.data_size, expected_data_size);

    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(commit.data_size, expected_data_size);
    assert!(commit.text.is_null());

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
    assert_eq!(RimeHighlightCandidate(session_id, 99), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 99), FALSE);
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
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 4), TRUE);
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 4), FALSE);
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 5), FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
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
fn current_page_candidate_apis_use_selected_schema_page_size_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("current-page-candidate-apis");
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
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let mut context = empty_context();
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 3), TRUE);
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 0), TRUE);
    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_size, 2);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: `context.menu.candidates` points to `num_candidates` entries.
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    // SAFETY: candidate text pointers are valid strings owned by the context.
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("爸")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeChangePage(session_id, FALSE), TRUE);
    assert_eq!(RimeDeleteCandidateOnCurrentPage(session_id, 0), TRUE);
    assert_eq!(RimeSelectCandidateOnCurrentPage(session_id, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("拔"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn alternative_select_keys_select_current_page_candidates_like_librime_selector() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("alternative-select-keys");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    let session_id = RimeCreateSession();
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
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
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
    let sequence = CString::new("baB").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let shifted_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&shifted_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(shifted_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(shifted_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(shifted_session_id, 'B' as i32, K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_session_id), TRUE);

    let shifted_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&shifted_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Shift+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(shifted_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_sequence_session_id), TRUE);

    let controlled_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&controlled_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(controlled_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(controlled_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(controlled_session_id, 'B' as i32, K_CONTROL_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(controlled_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(controlled_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_session_id), TRUE);

    let controlled_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&controlled_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Control+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(controlled_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_sequence_session_id), TRUE);

    let alt_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(alt_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&alt_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(alt_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(alt_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(alt_session_id, 'B' as i32, K_ALT_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(alt_session_id, &mut commit) }, FALSE);
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(alt_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(alt_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(alt_session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(alt_session_id), TRUE);

    let alt_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(alt_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&alt_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Alt+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(alt_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(alt_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(alt_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(alt_sequence_session_id), TRUE);

    let super_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(super_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&super_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(super_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(super_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(super_session_id, 'B' as i32, K_SUPER_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(super_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(super_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(super_session_id), TRUE);

    let super_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(super_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&super_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Super+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(super_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(super_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(super_sequence_session_id), TRUE);

    let released_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(released_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&released_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(released_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(released_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(released_session_id, 'B' as i32, K_RELEASE_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(released_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(released_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(released_session_id), TRUE);

    let released_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(released_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&released_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Release+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(released_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(released_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(released_sequence_session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn alternative_select_keys_suppress_unlisted_ascii_digit_fallback_like_librime_selector() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("alternative-select-keys-unlisted-digit");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let add_ba_translator = |session_id| {
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
    };

    let session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(session_id);

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let shifted_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_session_id);
    assert_eq!(RimeProcessKey(shifted_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(shifted_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(shifted_session_id, '2' as i32, K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(shifted_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(shifted_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(sequence_session_id);
    let sequence = CString::new("ba2B").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(printable_session_id);
    assert_eq!(RimeProcessKey(printable_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(printable_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(printable_session_id, 'x' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("bax")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(printable_session_id), TRUE);

    let printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(printable_sequence_session_id);
    let sequence = CString::new("bax").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(printable_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("bax")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(printable_sequence_session_id), TRUE);

    let shifted_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_printable_session_id);
    assert_eq!(
        RimeProcessKey(shifted_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(shifted_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(shifted_printable_session_id, 'x' as i32, K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(shifted_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("bax")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_printable_session_id), TRUE);

    let shifted_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_printable_sequence_session_id);
    let sequence = CString::new("ba{Shift+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe {
            RimeSimulateKeySequence(shifted_printable_sequence_session_id, sequence.as_ptr())
        },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(shifted_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("bax")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        RimeDestroySession(shifted_printable_sequence_session_id),
        TRUE
    );

    let controlled_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_printable_session_id);
    assert_eq!(
        RimeProcessKey(controlled_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(controlled_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(controlled_printable_session_id, 'x' as i32, K_CONTROL_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(controlled_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_printable_session_id), TRUE);

    let controlled_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_printable_sequence_session_id);
    let sequence = CString::new("ba{Control+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe {
            RimeSimulateKeySequence(controlled_printable_sequence_session_id, sequence.as_ptr())
        },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(controlled_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        RimeDestroySession(controlled_printable_sequence_session_id),
        TRUE
    );

    let alt_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(alt_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(alt_printable_session_id);
    assert_eq!(
        RimeProcessKey(alt_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(alt_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(alt_printable_session_id, 'x' as i32, K_ALT_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(alt_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(alt_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(alt_printable_session_id), TRUE);

    let alt_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(alt_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(alt_printable_sequence_session_id);
    let sequence = CString::new("ba{Alt+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(alt_printable_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(alt_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(alt_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(alt_printable_sequence_session_id), TRUE);

    let super_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(super_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(super_printable_session_id);
    assert_eq!(
        RimeProcessKey(super_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(super_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(super_printable_session_id, 'x' as i32, K_SUPER_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(super_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(super_printable_session_id), TRUE);

    let super_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(super_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(super_printable_sequence_session_id);
    let sequence = CString::new("ba{Super+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(super_printable_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(super_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        RimeDestroySession(super_printable_sequence_session_id),
        TRUE
    );

    let released_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(released_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(released_printable_session_id);
    assert_eq!(
        RimeProcessKey(released_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(released_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(released_printable_session_id, 'x' as i32, K_RELEASE_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(released_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(released_printable_session_id), TRUE);

    let released_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(released_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(released_printable_sequence_session_id);
    let sequence = CString::new("ba{Release+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe {
            RimeSimulateKeySequence(released_printable_sequence_session_id, sequence.as_ptr())
        },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(released_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        RimeDestroySession(released_printable_sequence_session_id),
        TRUE
    );

    let shifted_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_sequence_session_id);
    let sequence = CString::new("ba{Shift+2}B").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(shifted_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_sequence_session_id), TRUE);

    let controlled_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_session_id);
    assert_eq!(RimeProcessKey(controlled_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(controlled_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(controlled_session_id, '2' as i32, K_CONTROL_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_session_id), TRUE);

    let controlled_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_sequence_session_id);
    let sequence = CString::new("ba{Control+2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_sequence_session_id), TRUE);

    let controlled_shift_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_shift_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_shift_session_id);
    assert_eq!(
        RimeProcessKey(controlled_shift_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(controlled_shift_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(
            controlled_shift_session_id,
            '2' as i32,
            K_CONTROL_MASK | K_SHIFT_MASK
        ),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_shift_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_shift_session_id), TRUE);

    let controlled_shift_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_shift_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_shift_sequence_session_id);
    let sequence = CString::new("ba{Control+Shift+2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_shift_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_shift_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(
        RimeDestroySession(controlled_shift_sequence_session_id),
        TRUE
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn alternative_select_keys_keep_keypad_digit_fallback_like_librime_selector() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let root = unique_temp_dir("alternative-select-keys-keypad-digit");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let add_ba_translator = |session_id| {
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
    };

    let session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(session_id);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_2_keycode, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(sequence_session_id);
    let sequence = CString::new("ba{KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let shifted_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_session_id);
    assert_eq!(
        RimeProcessKey(shifted_session_id, kp_2_keycode, K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(shifted_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(shifted_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(shifted_session_id, kp_2_keycode, K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_session_id), TRUE);

    let shifted_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_sequence_session_id);
    let sequence = CString::new("ba{Shift+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(shifted_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_sequence_session_id), TRUE);

    let controlled_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_session_id);
    assert_eq!(
        RimeProcessKey(controlled_session_id, kp_2_keycode, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(controlled_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(controlled_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(controlled_session_id, kp_2_keycode, K_CONTROL_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_session_id), TRUE);

    let controlled_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_sequence_session_id);
    let sequence = CString::new("ba{Control+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_sequence_session_id), TRUE);

    let controlled_shift_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_shift_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_shift_session_id);
    assert_eq!(
        RimeProcessKey(
            controlled_shift_session_id,
            kp_2_keycode,
            K_CONTROL_MASK | K_SHIFT_MASK
        ),
        FALSE
    );
    assert_eq!(
        RimeProcessKey(controlled_shift_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(controlled_shift_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(
            controlled_shift_session_id,
            kp_2_keycode,
            K_CONTROL_MASK | K_SHIFT_MASK
        ),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_shift_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_shift_session_id), TRUE);

    let controlled_shift_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_shift_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_shift_sequence_session_id);
    let sequence = CString::new("ba{Control+Shift+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_shift_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_shift_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(
        RimeDestroySession(controlled_shift_sequence_session_id),
        TRUE
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn alternative_select_keys_beyond_page_size_are_consumed_like_librime_selector() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("alternative-select-keys-beyond-page-size");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: ABX\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let add_ba_translator = |session_id| {
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
            ("ba", "拔"),
        ]));
    };

    let session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(session_id);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'X' as i32, 0), TRUE);

    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeProcessKey(session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(sequence_session_id);
    let sequence = CString::new("baXB").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
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
fn rime_context_hides_candidate_entries_when_librime_hide_candidate_option_is_set() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let hide_candidate =
        CString::new("_hide_candidate").expect("option name should be a valid C string");
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, hide_candidate.as_ptr(), TRUE) };

    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert!(!context.composition.preedit.is_null());
    assert!(!context.commit_text_preview.is_null());
    assert_eq!(context.menu.page_size, 5);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.is_last_page, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert!(context.menu.candidates.is_null());
    assert!(context.menu.select_keys.is_null());
    assert!(context.select_labels.is_null());

    // SAFETY: `RimeGetContext` returned true and populated owned C strings.
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) };
    assert_eq!(preedit.to_str(), Ok("ni"));
    // SAFETY: `RimeGetContext` returned true and populated owned C strings.
    let preview = unsafe { CStr::from_ptr(context.commit_text_preview) };
    assert_eq!(preview.to_str(), Ok("ni"));

    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert!(context.composition.preedit.is_null());
    assert!(context.commit_text_preview.is_null());
    assert_eq!(context.menu.num_candidates, 0);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rime_context_reads_librime_menu_settings_from_selected_schema() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("context-menu-settings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n  alternative_select_labels: [Alpha, Beta]\n",
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
    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ni", "你"),
            ("ni", "尼"),
            ("ni", "呢"),
            ("ni", "泥"),
            ("ni", "拟"),
        ]));
    }
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 3), TRUE);

    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_size, 2);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    assert_eq!(context.menu.num_candidates, 2);
    assert!(!context.menu.select_keys.is_null());
    // SAFETY: `RimeGetContext` returned true and populated a select-key string.
    let select_keys = unsafe { CStr::from_ptr(context.menu.select_keys) };
    assert_eq!(select_keys.to_str(), Ok("AB"));
    assert!(!context.select_labels.is_null());
    // SAFETY: `RimeGetContext` returned true and populated one label per page slot.
    let select_labels = unsafe {
        std::slice::from_raw_parts(context.select_labels, context.menu.page_size as usize)
    };
    // SAFETY: label pointers are valid NUL-terminated strings owned by the context object.
    assert_eq!(
        unsafe { CStr::from_ptr(select_labels[0]) }.to_str(),
        Ok("Alpha")
    );
    // SAFETY: label pointers are valid NUL-terminated strings owned by the context object.
    assert_eq!(
        unsafe { CStr::from_ptr(select_labels[1]) }.to_str(),
        Ok("Beta")
    );
    // SAFETY: `context.menu.candidates` points to `num_candidates` entries.
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    // SAFETY: candidate texts are valid strings owned by the context object.
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("呢")
    );
    // SAFETY: candidate texts are valid strings owned by the context object.
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
        Ok("泥")
    );

    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert!(context.menu.select_keys.is_null());
    assert!(context.select_labels.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn rime_context_includes_librime_commit_text_preview_for_current_selection() {
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
            .add_translator(StaticTableTranslator::new([("ni", "你"), ("ni", "呢")]));
    }
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 1), TRUE);

    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert!(!context.commit_text_preview.is_null());
    // SAFETY: `RimeGetContext` returned true and populated a preview string.
    let preview = unsafe { CStr::from_ptr(context.commit_text_preview) };
    assert_eq!(preview.to_str(), Ok("呢"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert!(context.commit_text_preview.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rime_context_clear_respects_librime_versioned_tail_members() {
    let _guard = test_guard();
    let mut context = empty_context();
    let preview = CString::new("preserve-preview")
        .expect("literal should be valid")
        .into_raw();
    let label = CString::new("preserve-label")
        .expect("literal should be valid")
        .into_raw();
    let mut labels = vec![label];
    let labels_ptr = labels.as_mut_ptr();
    std::mem::forget(labels);

    context.data_size = context_data_size_before_commit_text_preview();
    context.menu.page_size = 1;
    context.commit_text_preview = preview;
    context.select_labels = labels_ptr;

    // SAFETY: `context` points to valid writable storage. Its tail members are
    // valid allocations but are outside the caller-declared version boundary.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        context.data_size,
        context_data_size_before_commit_text_preview()
    );
    assert_eq!(context.commit_text_preview, preview);
    assert_eq!(context.select_labels, labels_ptr);

    // SAFETY: the older-version context did not transfer ownership of tail
    // members to `RimeFreeContext`, so the test reclaims its own allocations.
    unsafe {
        drop(CString::from_raw(preview));
        let labels = Vec::from_raw_parts(labels_ptr, 1, 1);
        for label in labels {
            drop(CString::from_raw(label));
        }
    }
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
    // SAFETY: `iterator` remains valid and owns the current candidate.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let third_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(third_text.to_str(), Ok("ba"));
    // SAFETY: `iterator` remains valid; librime leaves the current candidate
    // intact when advancing beyond the final item.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 3);
    // SAFETY: the failed advance preserves the previous candidate string.
    let preserved_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(preserved_text.to_str(), Ok("ba"));
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

    let mut negative_iterator = empty_candidate_list_iterator();
    // SAFETY: `negative_iterator` points to valid writable storage.
    assert_eq!(
        unsafe { RimeCandidateListFromIndex(session_id, &mut negative_iterator, -1) },
        TRUE
    );
    assert_eq!(negative_iterator.index, -2);
    // SAFETY: `negative_iterator` was initialized by this API. librime starts
    // one position before the requested index, so the first advance from -1
    // fails and leaves the public index at -1.
    assert_eq!(
        unsafe { RimeCandidateListNext(&mut negative_iterator) },
        FALSE
    );
    assert_eq!(negative_iterator.index, -1);
    assert!(negative_iterator.candidate.text.is_null());
    // SAFETY: the iterator remains valid after the failed negative advance.
    assert_eq!(
        unsafe { RimeCandidateListNext(&mut negative_iterator) },
        TRUE
    );
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let first_text = unsafe { CStr::from_ptr(negative_iterator.candidate.text) };
    assert_eq!(first_text.to_str(), Ok("八"));
    // SAFETY: `negative_iterator` was initialized by this API and can be ended once.
    unsafe { RimeCandidateListEnd(&mut negative_iterator) };

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
fn rime_status_clear_preserves_librime_struct_data_size() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let expected_data_size =
        (std::mem::size_of::<RimeStatus>() - std::mem::size_of::<i32>()) as i32;
    let mut status = empty_status();
    status.data_size = expected_data_size;

    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id + 1, &mut status) }, FALSE);
    assert_eq!(status.data_size, expected_data_size);
    assert!(status.schema_id.is_null());
    assert!(status.schema_name.is_null());

    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.data_size, expected_data_size);
    assert!(!status.schema_id.is_null());
    assert!(!status.schema_name.is_null());

    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);
    assert_eq!(status.data_size, expected_data_size);
    assert!(status.schema_id.is_null());
    assert!(status.schema_name.is_null());

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

    let mut zero_len_marker = b'!' as c_char;
    // SAFETY: librime's strncpy-based getter accepts a valid output pointer
    // with a zero-length buffer and reports the non-empty property as present.
    assert_eq!(
        unsafe { RimeGetProperty(session_id, property.as_ptr(), &mut zero_len_marker, 0,) },
        TRUE
    );
    assert_eq!(zero_len_marker, b'!' as c_char);

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
    // SAFETY: the raw byte view is bounded to the caller-owned buffer.
    let truncated_value = unsafe {
        std::slice::from_raw_parts(short_buffer.as_ptr().cast::<u8>(), short_buffer.len())
    };
    assert_eq!(truncated_value, b"sample_");

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
    assert_eq!(read_commit(), "2");

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
    assert_eq!(read_commit(), "4");
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
fn deploy_schema_expands_librime_punctuator_import_preset() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-import-preset");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("symbols.yaml"),
        "\
punctuator:
  half_shape:
    \"/\": \"、\"
  symbols:
    \"/fh\": [\"©\", \"®\"]
",
    )
    .expect("preset config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - punct_translator
    - echo_translator
punctuator:
  import_preset: symbols
  half_shape:
    \"/\": \"schema-slash\"
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };

    let config_file = CString::new("luna.schema.yaml").expect("file name should be valid");
    let version_key = CString::new("schema/version").expect("version key should be valid");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("luna.schema.yaml"))
            .expect("staged schema should be readable"),
    )
    .expect("staged schema should parse");
    let half_shape = find_config_value(&staged, "punctuator/half_shape")
        .and_then(Value::as_mapping)
        .expect("half-shape punctuation map should be staged");
    assert_eq!(
        half_shape
            .get(Value::String("/".to_owned()))
            .and_then(Value::as_str),
        Some("schema-slash")
    );
    let symbols = find_config_value(&staged, "punctuator/symbols")
        .and_then(Value::as_mapping)
        .expect("symbol punctuation map should be staged");
    let slash_fh = symbols
        .get(Value::String("/fh".to_owned()))
        .and_then(Value::as_sequence)
        .expect("imported /fh symbol list should be staged");
    assert_eq!(slash_fh.first().and_then(Value::as_str), Some("©"));
    assert!(
        find_config_value(&staged, "__build_info/timestamps/symbols")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

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

    assert_eq!(candidate_texts("/"), ["schema-slash", "/"]);
    assert_eq!(candidate_texts("/fh"), ["©", "®", "/fh"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_schema_expands_librime_key_binder_import_preset() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-import-preset");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("key_bindings.yaml"),
        "\
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, toggle: ascii_mode }
",
    )
    .expect("preset config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
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
  import_preset: key_bindings
  bindings:
    - { when: always, accept: Control+Shift+2, toggle: full_shape }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };

    let config_file = CString::new("luna.schema.yaml").expect("file name should be valid");
    let version_key = CString::new("schema/version").expect("version key should be valid");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("luna.schema.yaml"))
            .expect("staged schema should be readable"),
    )
    .expect("staged schema should parse");
    let bindings = find_config_value(&staged, "key_binder/bindings")
        .and_then(Value::as_sequence)
        .expect("key bindings should be staged");
    assert_eq!(
        bindings
            .first()
            .and_then(|binding| find_config_value(binding, "toggle"))
            .and_then(Value::as_str),
        Some("ascii_mode")
    );
    assert_eq!(
        bindings
            .get(1)
            .and_then(|binding| find_config_value(binding, "toggle"))
            .and_then(Value::as_str),
        Some("full_shape")
    );
    assert!(
        find_config_value(&staged, "__build_info/timestamps/key_bindings")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

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
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
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
    let noop_named_sequence = CString::new("{Tab}").expect("key sequence should be valid");
    let noop_control_sequence = CString::new("{Linefeed}{Clear}{Pause}{Scroll_Lock}{Sys_Req}")
        .expect("key sequence should be valid");
    let noop_misc_sequence = CString::new(
        "{Begin}{Select}{Print}{Execute}{Insert}{Undo}{Redo}{Menu}{Find}{Cancel}{Help}{Break}",
    )
    .expect("key sequence should be valid");
    let noop_switch_sequence = CString::new(
        "{Arabic_switch}{Greek_switch}{Hangul_switch}{Hebrew_switch}{ISO_Group_Shift}{Mode_switch}{kana_switch}{script_switch}{Num_Lock}",
    )
    .expect("key sequence should be valid");
    let noop_function_sequence =
        CString::new("{F1}{Alt+F4}{F12}{F13}{F35}").expect("key sequence should be valid");
    let noop_modifier_key_sequence = CString::new(
        "{Shift_L}{Shift_R}{Control_L}{Control_R}{Caps_Lock}{Shift_Lock}{Meta_L}{Meta_R}{Alt_L}{Alt_R}{Super_L}{Super_R}{Hyper_L}{Release+Hyper_R}",
    )
    .expect("key sequence should be valid");
    let noop_iso_key_sequence = CString::new(
        "{ISO_Lock}{ISO_Level2_Latch}{ISO_Level3_Shift}{ISO_Level3_Latch}{ISO_Level3_Lock}{ISO_Group_Latch}{ISO_Group_Lock}{ISO_Next_Group}{ISO_Next_Group_Lock}{ISO_Prev_Group}{ISO_Prev_Group_Lock}{ISO_First_Group}{ISO_First_Group_Lock}{ISO_Last_Group}{ISO_Last_Group_Lock}{ISO_Left_Tab}{ISO_Move_Line_Up}{ISO_Move_Line_Down}{ISO_Partial_Line_Up}{ISO_Partial_Line_Down}{ISO_Partial_Space_Left}{ISO_Partial_Space_Right}{ISO_Set_Margin_Left}{ISO_Set_Margin_Right}{ISO_Release_Margin_Left}{ISO_Release_Margin_Right}{ISO_Release_Both_Margins}{ISO_Fast_Cursor_Left}{ISO_Fast_Cursor_Right}{ISO_Fast_Cursor_Up}{ISO_Fast_Cursor_Down}{ISO_Continuous_Underline}{ISO_Discontinuous_Underline}{ISO_Emphasize}{ISO_Center_Object}{Release+ISO_Enter}",
    )
    .expect("key sequence should be valid");
    let noop_xkb_key_sequence = CString::new(concat!(
        "{dead_grave}{dead_acute}{dead_circumflex}{dead_tilde}{dead_macron}",
        "{dead_breve}{dead_abovedot}{dead_diaeresis}{dead_abovering}",
        "{dead_doubleacute}{dead_caron}{dead_cedilla}{dead_ogonek}",
        "{dead_iota}{dead_voiced_sound}{dead_semivoiced_sound}{dead_belowdot}",
        "{dead_hook}{dead_horn}{AccessX_Enable}{AccessX_Feedback_Enable}",
        "{RepeatKeys_Enable}{SlowKeys_Enable}{BounceKeys_Enable}",
        "{StickyKeys_Enable}{MouseKeys_Enable}{MouseKeys_Accel_Enable}",
        "{Overlay1_Enable}{Overlay2_Enable}{AudibleBell_Enable}",
        "{First_Virtual_Screen}{Prev_Virtual_Screen}{Next_Virtual_Screen}",
        "{Last_Virtual_Screen}{Terminate_Server}{Pointer_Left}{Pointer_Right}",
        "{Pointer_Up}{Pointer_Down}{Pointer_UpLeft}{Pointer_UpRight}",
        "{Pointer_DownLeft}{Pointer_DownRight}{Pointer_Button_Dflt}",
        "{Pointer_Button1}{Pointer_Button2}{Pointer_Button3}{Pointer_Button4}",
        "{Pointer_Button5}{Pointer_DblClick_Dflt}{Pointer_DblClick1}",
        "{Pointer_DblClick2}{Pointer_DblClick3}{Pointer_DblClick4}",
        "{Pointer_DblClick5}{Pointer_Drag_Dflt}{Pointer_Drag1}",
        "{Pointer_Drag2}{Pointer_Drag3}{Pointer_Drag4}{Pointer_EnableKeys}",
        "{Pointer_Accelerate}{Pointer_DfltBtnNext}{Pointer_DfltBtnPrev}",
        "{Release+Pointer_Drag5}",
    ))
    .expect("key sequence should be valid");
    let noop_input_method_key_sequence = CString::new(concat!(
        "{Multi_key}{Kanji}{Muhenkan}{Henkan}{Henkan_Mode}{Romaji}",
        "{Hiragana}{Katakana}{Hiragana_Katakana}{Zenkaku}{Hankaku}",
        "{Zenkaku_Hankaku}{Touroku}{Massyo}{Kana_Lock}{Kana_Shift}",
        "{Eisu_Shift}{Eisu_toggle}{Hangul}{Hangul_Start}{Hangul_End}",
        "{Hangul_Hanja}{Hangul_Jamo}{Hangul_Romaja}{Codeinput}",
        "{Hangul_Jeonja}{Hangul_Banja}{Hangul_PreHanja}{Hangul_PostHanja}",
        "{SingleCandidate}{MultipleCandidate}{PreviousCandidate}",
        "{Release+Hangul_Special}",
    ))
    .expect("key sequence should be valid");
    let noop_keypad_sequence = CString::new(concat!(
        "{KP_Space}{KP_Tab}{KP_F1}{KP_F2}{KP_F3}{KP_F4}{KP_Begin}",
        "{KP_Insert}{KP_Delete}{KP_Multiply}{KP_Add}{KP_Separator}",
        "{KP_Subtract}{KP_Decimal}{KP_Divide}{Release+KP_Equal}",
    ))
    .expect("key sequence should be valid");
    let noop_latin1_key_sequence =
        CString::new("{nobreakspace}{yen}{ETH}{Eth}{THORN}{Thorn}{division}{Release+ydiaeresis}")
            .expect("key sequence should be valid");
    let noop_latin2_key_sequence =
        CString::new("{Aogonek}{breve}{Lstroke}{Scaron}{Dstroke}{Odoubleacute}{Release+abovedot}")
            .expect("key sequence should be valid");
    let noop_latin3_key_sequence = CString::new(
        "{Hstroke}{Hcircumflex}{Iabovedot}{Gbreve}{Jcircumflex}{Scircumflex}{Release+scircumflex}",
    )
    .expect("key sequence should be valid");
    let noop_latin4_key_sequence = CString::new(
        "{kappa}{kra}{Rcedilla}{Itilde}{Lcedilla}{ENG}{Amacron}{Umacron}{Release+umacron}",
    )
    .expect("key sequence should be valid");
    let noop_kana_key_sequence = CString::new(
        "{overline}{kana_fullstop}{kana_conjunctive}{kana_middledot}{kana_tu}{kana_TI}{kana_HU}{voicedsound}{Release+semivoicedsound}",
    )
    .expect("key sequence should be valid");
    let noop_arabic_key_sequence = CString::new(
        "{Arabic_comma}{Arabic_semicolon}{Arabic_question_mark}{Arabic_hamza}{Arabic_hamzaonyeh}{Arabic_tatweel}{Arabic_ha}{Arabic_heh}{Release+Arabic_sukun}",
    )
    .expect("key sequence should be valid");
    let noop_cyrillic_key_sequence = CString::new(
        "{Serbian_dje}{Ukrainian_ie}{Ukranian_je}{Cyrillic_je}{Serbian_je}{Byelorussian_shortu}{Cyrillic_dzhe}{Serbian_dze}{numerosign}{Cyrillic_yu}{Cyrillic_hardsign}{Cyrillic_YU}{Release+Cyrillic_HARDSIGN}",
    )
    .expect("key sequence should be valid");
    let noop_greek_key_sequence = CString::new(
        "{Greek_ALPHAaccent}{Greek_IOTAdieresis}{Greek_IOTAdiaeresis}{Greek_LAMBDA}{Greek_LAMDA}{Greek_OMEGA}{Greek_lambda}{Greek_lamda}{Greek_finalsmallsigma}{Release+Greek_omega}",
    )
    .expect("key sequence should be valid");
    let noop_technical_key_sequence = CString::new(
        "{leftradical}{topleftradical}{topvertsummationconnector}{lessthanequal}{infinity}{leftarrow}{blank}{lowrightcorner}{Release+vertbar}",
    )
    .expect("key sequence should be valid");
    let noop_publishing_key_sequence = CString::new(
        "{emspace}{enspace}{signifblank}{ellipsis}{trademark}{leftsinglequotemark}{telephone}{leftcaret}{overbar}{Release+righttack}",
    )
    .expect("key sequence should be valid");
    let noop_hebrew_key_sequence = CString::new(
        "{hebrew_doublelowline}{hebrew_aleph}{hebrew_bet}{hebrew_beth}{hebrew_samech}{hebrew_samekh}{hebrew_kuf}{hebrew_qoph}{Release+hebrew_taw}",
    )
    .expect("key sequence should be valid");
    let named_ascii_sequence =
        CString::new("{exclam}{space}").expect("key sequence should be valid");
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

    // SAFETY: noop_named_sequence is a valid C string; librime parses known
    // key-table names such as Tab even when the engine does not handle them.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_named_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: Tab is a parsed no-op and should leave the context empty after
    // the previous commit.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_control_sequence is a valid C string; librime parses these
    // adjacent key-table names even when the engine ignores their events.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_control_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored named keys should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_misc_sequence is a valid C string; librime accepts these
    // named function keys in simulated sequences even when no processor handles
    // them in the active session.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_misc_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored named keys should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_switch_sequence is a valid C string; librime accepts
    // mode-switch aliases and Num_Lock as known key-table names.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_switch_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored named keys should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_function_sequence is a valid C string; librime accepts F1
    // through F35 via its key table even when the active engine ignores them.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_function_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored function keys should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_modifier_key_sequence is a valid C string; librime parses
    // physical modifier key names as key-table names even when ignored.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_modifier_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored modifier-key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_iso_key_sequence is a valid C string; librime parses the
    // ISO key-name block through its key table even when no processor handles
    // the resulting key events.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_iso_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored ISO key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_xkb_key_sequence is a valid C string; librime parses the
    // XKB/dead-key block through its key table even when no processor handles
    // the resulting key events.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_xkb_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored XKB key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_input_method_key_sequence is a valid C string; librime
    // parses input-method key names through its key table even when ignored.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_input_method_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored input-method key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_keypad_sequence is a valid C string; librime parses keypad
    // key-table names through its key table even when ignored by the engine.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_keypad_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored keypad key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_latin1_key_sequence is a valid C string; librime parses
    // Latin-1 key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_latin1_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Latin-1 key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_latin2_key_sequence is a valid C string; librime parses
    // Latin-2 key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_latin2_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Latin-2 key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_latin3_key_sequence is a valid C string; librime parses
    // Latin-3 key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_latin3_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Latin-3 key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_latin4_key_sequence is a valid C string; librime parses
    // Latin-4 key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_latin4_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Latin-4 key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_kana_key_sequence is a valid C string; librime parses
    // kana key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_kana_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored kana key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_arabic_key_sequence is a valid C string; librime parses
    // Arabic key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_arabic_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Arabic key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_cyrillic_key_sequence is a valid C string; librime parses
    // Cyrillic key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_cyrillic_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Cyrillic key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_greek_key_sequence is a valid C string; librime parses
    // Greek key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_greek_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Greek key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_technical_key_sequence is a valid C string; librime parses
    // technical-symbol key-table names even though the default editor/speller
    // ignore non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_technical_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored technical key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_publishing_key_sequence is a valid C string; librime parses
    // publishing/APL key-table names even though the default editor/speller
    // ignore non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_publishing_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored publishing/APL key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_hebrew_key_sequence is a valid C string; librime parses
    // Hebrew key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_hebrew_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Hebrew key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: named_ascii_sequence is a valid C string; librime parses ASCII
    // symbolic key names through its key table as printable key events.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, named_ascii_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("!"));
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
fn cleanup_stale_sessions_matches_librime_activity_lifespan() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let stale_session_id = RimeCreateSession();
    let refreshed_session_id = RimeCreateSession();
    let fresh_session_id = RimeCreateSession();
    let stale_time = super::session_activity_now().saturating_sub(super::SESSION_LIFESPAN_SECS + 1);

    {
        let mut registry = super::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        registry
            .sessions
            .get_mut(&stale_session_id)
            .expect("stale session should exist")
            .last_active_time = stale_time;
        registry
            .sessions
            .get_mut(&refreshed_session_id)
            .expect("refreshed session should exist")
            .last_active_time = stale_time;
    }

    assert_eq!(RimeFindSession(refreshed_session_id), TRUE);
    RimeCleanupStaleSessions();

    assert_eq!(RimeFindSession(stale_session_id), FALSE);
    assert_eq!(RimeFindSession(refreshed_session_id), TRUE);
    assert_eq!(RimeFindSession(fresh_session_id), TRUE);

    RimeCleanupAllSessions();
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
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 1 << 3), FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}
