use super::*;

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

    let composition = crate::RimeComposition {
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
        std::mem::size_of::<crate::RimeComposition>(),
        align_up(composition_preedit + ptr_size, ptr_align)
    );

    let candidate = crate::RimeCandidate {
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
    assert_eq!(std::mem::size_of::<crate::RimeCandidate>(), ptr_size * 3);

    let menu = crate::RimeMenu {
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
        std::mem::size_of::<crate::RimeMenu>(),
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
    let context_composition = align_up(int_size, std::mem::align_of::<crate::RimeComposition>());
    let context_menu = align_up(
        context_composition + std::mem::size_of::<crate::RimeComposition>(),
        std::mem::align_of::<crate::RimeMenu>(),
    );
    let context_commit_preview = align_up(
        context_menu + std::mem::size_of::<crate::RimeMenu>(),
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
        std::mem::align_of::<crate::RimeCandidate>(),
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
            iterator_candidate + std::mem::size_of::<crate::RimeCandidate>(),
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

    let schema_list_item = crate::RimeSchemaListItem {
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
        std::mem::size_of::<crate::RimeSchemaListItem>(),
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
        std::mem::size_of::<crate::RimeSchemaList>(),
        align_up(
            schema_list_items + ptr_size,
            std::mem::align_of::<crate::RimeSchemaList>(),
        )
    );

    let string_slice = crate::RimeStringSlice {
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
        std::mem::size_of::<crate::RimeStringSlice>(),
        align_up(
            string_slice_length + usize_size,
            std::mem::align_of::<crate::RimeStringSlice>(),
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

    let custom_settings = crate::RimeCustomSettings { placeholder: 0 };
    let switcher_settings = crate::RimeSwitcherSettings { placeholder: 0 };
    let schema_info = crate::RimeSchemaInfo { placeholder: 0 };
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
    assert_eq!(std::mem::size_of::<crate::RimeCustomSettings>(), 1);
    assert_eq!(std::mem::size_of::<crate::RimeSwitcherSettings>(), 1);
    assert_eq!(std::mem::size_of::<crate::RimeSchemaInfo>(), 1);

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

    assert_eq!(field_offset(api, std::ptr::addr_of!(api.data_size)), 0);
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
        field_offset(levers_api, std::ptr::addr_of!(levers_api.data_size)),
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
