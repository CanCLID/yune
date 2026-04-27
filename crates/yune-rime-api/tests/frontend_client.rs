use std::{
    ffi::{CStr, CString},
    fs, mem,
    os::raw::{c_char, c_int},
    path::PathBuf,
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{
    rime_get_api, RimeCandidate, RimeCandidateListIterator, RimeCommit, RimeComposition,
    RimeConfig, RimeConfigIterator, RimeContext, RimeMenu, RimeStatus, RimeTraits, FALSE, TRUE,
};

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

fn empty_config() -> RimeConfig {
    RimeConfig {
        ptr: ptr::null_mut(),
    }
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
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned")
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

#[test]
fn frontend_style_api_table_can_read_in_memory_configs() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let config_init = api.config_init.expect("frontend requires config_init");
    let config_load_string = api
        .config_load_string
        .expect("frontend requires config_load_string");
    let config_get_bool = api
        .config_get_bool
        .expect("frontend requires config_get_bool");
    let config_get_int = api
        .config_get_int
        .expect("frontend requires config_get_int");
    let config_get_double = api
        .config_get_double
        .expect("frontend requires config_get_double");
    let config_get_string = api
        .config_get_string
        .expect("frontend requires config_get_string");
    let config_get_cstring = api
        .config_get_cstring
        .expect("frontend requires config_get_cstring");
    let config_list_size = api
        .config_list_size
        .expect("frontend requires config_list_size");
    let config_begin_list = api
        .config_begin_list
        .expect("frontend requires config_begin_list");
    let config_begin_map = api
        .config_begin_map
        .expect("frontend requires config_begin_map");
    let config_next = api.config_next.expect("frontend requires config_next");
    let config_end = api.config_end.expect("frontend requires config_end");
    let config_close = api.config_close.expect("frontend requires config_close");

    let mut config = empty_config();
    let yaml = CString::new(
        "\
schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\nswitches:\n  - name: ascii_mode\n  - name: full_shape\nmenu:\n  page_size: 9\n  alternative_select_keys: ABC\nweights:\n  bias: 0.75\nenabled: true\n",
    )
    .expect("yaml should not contain NUL");
    let enabled_key = CString::new("enabled").expect("literal should not contain NUL");
    let page_size_key = CString::new("menu/page_size").expect("literal should not contain NUL");
    let bias_key = CString::new("weights/bias").expect("literal should not contain NUL");
    let schema_name_key = CString::new("schema/name").expect("literal should not contain NUL");
    let schema_id_key = CString::new("schema/schema_id").expect("literal should not contain NUL");
    let switches_key = CString::new("switches").expect("literal should not contain NUL");
    let menu_key = CString::new("menu").expect("literal should not contain NUL");
    let missing_key = CString::new("missing").expect("literal should not contain NUL");

    assert_eq!(unsafe { config_init(&mut config) }, TRUE);
    assert!(!config.ptr.is_null());
    assert_eq!(unsafe { config_init(&mut config) }, FALSE);
    assert_eq!(
        unsafe { config_load_string(&mut config, yaml.as_ptr()) },
        TRUE
    );

    let mut enabled = FALSE;
    let mut page_size: c_int = 0;
    let mut bias = 0.0;
    let mut schema_name_buffer = vec![0 as c_char; 16];
    assert_eq!(
        unsafe { config_get_bool(&mut config, enabled_key.as_ptr(), &mut enabled) },
        TRUE
    );
    assert_eq!(enabled, TRUE);
    assert_eq!(
        unsafe { config_get_int(&mut config, page_size_key.as_ptr(), &mut page_size) },
        TRUE
    );
    assert_eq!(page_size, 9);
    assert_eq!(
        unsafe { config_get_double(&mut config, bias_key.as_ptr(), &mut bias) },
        TRUE
    );
    assert_eq!(bias, 0.75);
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                schema_name_key.as_ptr(),
                schema_name_buffer.as_mut_ptr(),
                schema_name_buffer.len(),
            )
        },
        TRUE
    );
    let schema_name = unsafe { CStr::from_ptr(schema_name_buffer.as_ptr()) };
    assert_eq!(schema_name.to_str(), Ok("Luna Pinyin"));
    let schema_id = unsafe { config_get_cstring(&mut config, schema_id_key.as_ptr()) };
    assert!(!schema_id.is_null());
    let schema_id = unsafe { CStr::from_ptr(schema_id) };
    assert_eq!(schema_id.to_str(), Ok("luna_pinyin"));
    assert_eq!(
        unsafe { config_list_size(&mut config, switches_key.as_ptr()) },
        2
    );

    let mut iterator = empty_config_iterator();
    assert_eq!(
        unsafe { config_begin_list(&mut iterator, &mut config, switches_key.as_ptr()) },
        TRUE
    );
    assert_eq!(iterator.index, -1);
    assert!(!iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(unsafe { config_next(&mut iterator) }, TRUE);
    assert_eq!(iterator.index, 0);
    assert_eq!(unsafe { CStr::from_ptr(iterator.key) }.to_str(), Ok("@0"));
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("switches/@0")
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, TRUE);
    assert_eq!(iterator.index, 1);
    assert_eq!(unsafe { CStr::from_ptr(iterator.key) }.to_str(), Ok("@1"));
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("switches/@1")
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 2);
    unsafe { config_end(&mut iterator) };
    assert!(iterator.list.is_null());
    assert!(iterator.key.is_null());

    assert_eq!(
        unsafe { config_begin_map(&mut iterator, &mut config, menu_key.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("alternative_select_keys")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/alternative_select_keys")
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("page_size")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/page_size")
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 2);
    unsafe { config_end(&mut iterator) };

    assert_eq!(
        unsafe { config_begin_list(&mut iterator, &mut config, missing_key.as_ptr()) },
        FALSE
    );
    assert!(iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(iterator.index, -1);
    assert!(iterator.key.is_null());
    assert!(iterator.path.is_null());

    assert_eq!(unsafe { config_close(&mut config) }, TRUE);
    assert!(config.ptr.is_null());
}

#[test]
fn frontend_style_api_table_can_mutate_in_memory_configs() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let config_init = api.config_init.expect("frontend requires config_init");
    let config_load_string = api
        .config_load_string
        .expect("frontend requires config_load_string");
    let config_set_bool = api
        .config_set_bool
        .expect("frontend requires config_set_bool");
    let config_set_int = api
        .config_set_int
        .expect("frontend requires config_set_int");
    let config_set_double = api
        .config_set_double
        .expect("frontend requires config_set_double");
    let config_set_string = api
        .config_set_string
        .expect("frontend requires config_set_string");
    let config_get_bool = api
        .config_get_bool
        .expect("frontend requires config_get_bool");
    let config_get_int = api
        .config_get_int
        .expect("frontend requires config_get_int");
    let config_get_double = api
        .config_get_double
        .expect("frontend requires config_get_double");
    let config_get_string = api
        .config_get_string
        .expect("frontend requires config_get_string");
    let config_create_list = api
        .config_create_list
        .expect("frontend requires config_create_list");
    let config_create_map = api
        .config_create_map
        .expect("frontend requires config_create_map");
    let config_list_size = api
        .config_list_size
        .expect("frontend requires config_list_size");
    let config_get_item = api
        .config_get_item
        .expect("frontend requires config_get_item");
    let config_set_item = api
        .config_set_item
        .expect("frontend requires config_set_item");
    let config_clear = api.config_clear.expect("frontend requires config_clear");
    let config_close = api.config_close.expect("frontend requires config_close");

    let mut source = empty_config();
    let mut item = empty_config();
    let mut destination = empty_config();
    let schema_key = CString::new("schema").expect("literal should not contain NUL");
    let schema_name_key = CString::new("schema/name").expect("literal should not contain NUL");
    let copied_schema_key = CString::new("copied/schema").expect("literal should not contain NUL");
    let copied_name_key =
        CString::new("copied/schema/name").expect("literal should not contain NUL");
    let page_size_key = CString::new("menu/page_size").expect("literal should not contain NUL");
    let bias_key = CString::new("weights/bias").expect("literal should not contain NUL");
    let enabled_key = CString::new("enabled").expect("literal should not contain NUL");
    let switches_key = CString::new("switches").expect("literal should not contain NUL");
    let menu_key = CString::new("menu").expect("literal should not contain NUL");
    let name_value = CString::new("Default").expect("literal should not contain NUL");
    let replacement_value = CString::new("Modified").expect("literal should not contain NUL");
    let yaml = CString::new(
        "\
schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\n",
    )
    .expect("yaml should not contain NUL");

    assert_eq!(unsafe { config_init(&mut destination) }, TRUE);
    assert_eq!(
        unsafe {
            config_set_string(
                &mut destination,
                schema_name_key.as_ptr(),
                name_value.as_ptr(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { config_set_int(&mut destination, page_size_key.as_ptr(), 7) },
        TRUE
    );
    assert_eq!(
        unsafe { config_set_double(&mut destination, bias_key.as_ptr(), 1.25) },
        TRUE
    );
    assert_eq!(
        unsafe { config_set_bool(&mut destination, enabled_key.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { config_create_list(&mut destination, switches_key.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { config_create_map(&mut destination, menu_key.as_ptr()) },
        TRUE
    );

    let mut output = vec![0 as c_char; 32];
    let mut int_output: c_int = 0;
    let mut double_output = 0.0;
    let mut bool_output = FALSE;
    assert_eq!(
        unsafe {
            config_get_string(
                &mut destination,
                schema_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Default")
    );
    assert_eq!(
        unsafe { config_get_int(&mut destination, page_size_key.as_ptr(), &mut int_output) },
        FALSE
    );
    assert_eq!(
        unsafe { config_get_double(&mut destination, bias_key.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 1.25);
    assert_eq!(
        unsafe { config_get_bool(&mut destination, enabled_key.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, TRUE);
    assert_eq!(
        unsafe { config_list_size(&mut destination, switches_key.as_ptr()) },
        0
    );

    assert_eq!(
        unsafe { config_load_string(&mut source, yaml.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { config_get_item(&mut source, schema_key.as_ptr(), &mut item) },
        TRUE
    );
    assert!(!item.ptr.is_null());
    assert_eq!(
        unsafe { config_set_item(&mut destination, copied_schema_key.as_ptr(), &mut item) },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut destination,
                copied_name_key.as_ptr(),
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

    assert_eq!(
        unsafe {
            config_set_string(
                &mut item,
                schema_name_key.as_ptr(),
                replacement_value.as_ptr(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut destination,
                copied_name_key.as_ptr(),
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

    assert_eq!(
        unsafe { config_clear(&mut destination, copied_name_key.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut destination,
                copied_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        FALSE
    );

    assert_eq!(unsafe { config_close(&mut source) }, TRUE);
    assert_eq!(unsafe { config_close(&mut item) }, TRUE);
    assert_eq!(unsafe { config_close(&mut destination) }, TRUE);
}

#[test]
fn frontend_style_api_table_can_update_config_signatures() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let config_init = api.config_init.expect("frontend requires config_init");
    let config_update_signature = api
        .config_update_signature
        .expect("frontend requires config_update_signature");
    let config_get_string = api
        .config_get_string
        .expect("frontend requires config_get_string");
    let config_close = api.config_close.expect("frontend requires config_close");

    let distribution_code_name =
        CString::new("frontend-test").expect("distribution code name should be valid");
    let distribution_version =
        CString::new("2026.04").expect("distribution version should be valid");
    let mut traits = empty_traits();
    traits.distribution_code_name = distribution_code_name.as_ptr();
    traits.distribution_version = distribution_version.as_ptr();
    unsafe { setup(&traits) };

    let mut config = empty_config();
    let signer = CString::new("frontend-client").expect("signer should be valid");
    let generator_key =
        CString::new("signature/generator").expect("literal should not contain NUL");
    let distribution_code_name_key =
        CString::new("signature/distribution_code_name").expect("literal should not contain NUL");
    let distribution_version_key =
        CString::new("signature/distribution_version").expect("literal should not contain NUL");
    let rime_version_key =
        CString::new("signature/rime_version").expect("literal should not contain NUL");
    let modified_time_key =
        CString::new("signature/modified_time").expect("literal should not contain NUL");
    let mut output = vec![0 as c_char; 64];

    assert_eq!(unsafe { config_init(&mut config) }, TRUE);
    assert_eq!(
        unsafe { config_update_signature(&mut config, signer.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                generator_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("frontend-client")
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                distribution_code_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("frontend-test")
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                distribution_version_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("2026.04")
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                rime_version_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert!(unsafe { CStr::from_ptr(output.as_ptr()) }
        .to_str()
        .is_ok_and(|value| value.starts_with("yune-rime-api ")));
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                modified_time_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert!(unsafe { CStr::from_ptr(output.as_ptr()) }
        .to_str()
        .is_ok_and(|value| value.len() >= 20 && value.contains(':') && !value.ends_with('\n')));
    assert_eq!(
        unsafe { config_update_signature(&mut config, ptr::null()) },
        FALSE
    );

    assert_eq!(unsafe { config_close(&mut config) }, TRUE);
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
}

#[test]
fn frontend_style_api_table_can_drive_basic_composition_flow() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    assert_eq!(
        api.data_size,
        (mem::size_of_val(api) - mem::size_of::<i32>()) as i32
    );

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let find_session = api.find_session.expect("frontend requires find_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let get_input = api.get_input.expect("frontend requires get_input");
    let get_status = api.get_status.expect("frontend requires get_status");
    let free_status = api.free_status.expect("frontend requires free_status");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(find_session(session_id), TRUE);
    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let input = get_input(session_id);
    assert!(!input.is_null());
    let input = unsafe { CStr::from_ptr(input) };
    assert_eq!(input.to_str(), Ok("ni"));

    let mut status = empty_status();
    assert_eq!(unsafe { get_status(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    let schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    assert_eq!(schema_id.to_str(), Ok("default"));
    assert_eq!(unsafe { free_status(&mut status) }, TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert_eq!(context.menu.page_size, 5);
    assert_eq!(context.menu.num_candidates, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    let first_candidate = unsafe { *context.menu.candidates };
    let first_candidate_text = unsafe { CStr::from_ptr(first_candidate.text) };
    assert_eq!(first_candidate_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(select_candidate_on_current_page(session_id, 0), TRUE);

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    let commit_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(commit_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_simulate_key_sequences() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let simulate_key_sequence = api
        .simulate_key_sequence
        .expect("frontend requires simulate_key_sequence");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let sequence = CString::new("ni{space}").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { simulate_key_sequence(session_id, sequence.as_ptr()) },
        TRUE
    );

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    let commit_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(commit_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_edit_input_and_caret() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let get_version = api.get_version.expect("frontend requires get_version");
    let version = get_version();
    assert!(!version.is_null());
    let version = unsafe { CStr::from_ptr(version) };
    assert_eq!(version.to_str(), Ok("yune-rime-api 0.1.0"));

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let get_input = api.get_input.expect("frontend requires get_input");
    let get_caret_pos = api.get_caret_pos.expect("frontend requires get_caret_pos");
    let set_caret_pos = api.set_caret_pos.expect("frontend requires set_caret_pos");
    let set_input = api.set_input.expect("frontend requires set_input");

    assert!(get_input(0).is_null());
    assert_eq!(get_caret_pos(0), 0);

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let input = CString::new("nihao").expect("literal should not contain NUL");
    assert_eq!(unsafe { set_input(session_id, input.as_ptr()) }, TRUE);
    assert_eq!(get_caret_pos(session_id), 5);

    let current_input = get_input(session_id);
    assert!(!current_input.is_null());
    let current_input = unsafe { CStr::from_ptr(current_input) };
    assert_eq!(current_input.to_str(), Ok("nihao"));

    set_caret_pos(session_id, 2);
    assert_eq!(get_caret_pos(session_id), 2);
    set_caret_pos(session_id, 99);
    assert_eq!(get_caret_pos(session_id), 5);

    assert_eq!(unsafe { set_input(session_id, ptr::null()) }, FALSE);
    assert_eq!(unsafe { set_input(session_id + 1, input.as_ptr()) }, FALSE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_commit_clear_and_delete_composition() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let commit_composition = api
        .commit_composition
        .expect("frontend requires commit_composition");
    let clear_composition = api
        .clear_composition
        .expect("frontend requires clear_composition");
    let delete_candidate = api
        .delete_candidate
        .expect("frontend requires delete_candidate");
    let delete_candidate_on_current_page = api
        .delete_candidate_on_current_page
        .expect("frontend requires delete_candidate_on_current_page");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    assert_eq!(commit_composition(0), FALSE);
    assert_eq!(delete_candidate(0, 0), FALSE);

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(commit_composition(session_id), FALSE);
    assert_eq!(delete_candidate(session_id, 0), FALSE);

    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(commit_composition(session_id), TRUE);

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    let committed_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(committed_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(process_key(session_id, 'h' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    clear_composition(session_id);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(delete_candidate(session_id, 1), FALSE);
    assert_eq!(delete_candidate_on_current_page(session_id, 0), TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_manage_runtime_state() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let set_option = api.set_option.expect("frontend requires set_option");
    let get_option = api.get_option.expect("frontend requires get_option");
    let set_property = api.set_property.expect("frontend requires set_property");
    let get_property = api.get_property.expect("frontend requires get_property");
    let get_current_schema = api
        .get_current_schema
        .expect("frontend requires get_current_schema");
    let select_schema = api.select_schema.expect("frontend requires select_schema");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let get_status = api.get_status.expect("frontend requires get_status");
    let free_status = api.free_status.expect("frontend requires free_status");

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let ascii_mode = CString::new("ascii_mode").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { get_option(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    unsafe { set_option(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(unsafe { get_option(session_id, ascii_mode.as_ptr()) }, TRUE);

    let mut status = empty_status();
    assert_eq!(unsafe { get_status(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_ascii_mode, TRUE);
    assert_eq!(unsafe { free_status(&mut status) }, TRUE);

    let property = CString::new("client_app").expect("literal should not contain NUL");
    let property_value = CString::new("frontend_client").expect("literal should not contain NUL");
    let mut property_buffer = vec![0 as c_char; 32];
    assert_eq!(
        unsafe {
            get_property(
                session_id,
                property.as_ptr(),
                property_buffer.as_mut_ptr(),
                property_buffer.len(),
            )
        },
        FALSE
    );
    unsafe { set_property(session_id, property.as_ptr(), property_value.as_ptr()) };
    assert_eq!(
        unsafe {
            get_property(
                session_id,
                property.as_ptr(),
                property_buffer.as_mut_ptr(),
                property_buffer.len(),
            )
        },
        TRUE
    );
    let copied_property = unsafe { CStr::from_ptr(property_buffer.as_ptr()) };
    assert_eq!(copied_property.to_str(), Ok("frontend_client"));

    let mut schema_buffer = vec![0 as c_char; 32];
    assert_eq!(
        unsafe { get_current_schema(session_id, schema_buffer.as_mut_ptr(), schema_buffer.len()) },
        TRUE
    );
    let current_schema = unsafe { CStr::from_ptr(schema_buffer.as_ptr()) };
    assert_eq!(current_schema.to_str(), Ok("default"));

    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let schema_id = CString::new("sample_schema").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { get_current_schema(session_id, schema_buffer.as_mut_ptr(), schema_buffer.len()) },
        TRUE
    );
    let selected_schema = unsafe { CStr::from_ptr(schema_buffer.as_ptr()) };
    assert_eq!(selected_schema.to_str(), Ok("sample_schema"));

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_read_runtime_paths() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let root = unique_temp_dir("runtime-paths");
    let shared = root.join("shared");
    let user = root.join("user");
    let prebuilt = root.join("prebuilt");
    let staging = root.join("stage");
    let sync = root.join("sync");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        user.join("installation.yaml"),
        format!(
            "installation_id: frontend-user\nsync_dir: '{}'\n",
            sync.to_string_lossy()
        ),
    )
    .expect("installation metadata should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let prebuilt_c = CString::new(prebuilt.to_string_lossy().as_ref()).expect("path is valid");
    let staging_c = CString::new(staging.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    traits.prebuilt_data_dir = prebuilt_c.as_ptr();
    traits.staging_dir = staging_c.as_ptr();
    unsafe { setup(&traits) };

    let get_shared_data_dir = api
        .get_shared_data_dir
        .expect("frontend requires get_shared_data_dir");
    let get_user_data_dir = api
        .get_user_data_dir
        .expect("frontend requires get_user_data_dir");
    let get_prebuilt_data_dir = api
        .get_prebuilt_data_dir
        .expect("frontend requires get_prebuilt_data_dir");
    let get_staging_dir = api
        .get_staging_dir
        .expect("frontend requires get_staging_dir");
    let get_sync_dir = api.get_sync_dir.expect("frontend requires get_sync_dir");
    let get_user_id = api.get_user_id.expect("frontend requires get_user_id");
    let get_shared_data_dir_s = api
        .get_shared_data_dir_s
        .expect("frontend requires get_shared_data_dir_s");
    let get_user_data_dir_s = api
        .get_user_data_dir_s
        .expect("frontend requires get_user_data_dir_s");
    let get_prebuilt_data_dir_s = api
        .get_prebuilt_data_dir_s
        .expect("frontend requires get_prebuilt_data_dir_s");
    let get_staging_dir_s = api
        .get_staging_dir_s
        .expect("frontend requires get_staging_dir_s");
    let get_sync_dir_s = api
        .get_sync_dir_s
        .expect("frontend requires get_sync_dir_s");
    let get_user_data_sync_dir = api
        .get_user_data_sync_dir
        .expect("frontend requires get_user_data_sync_dir");

    let shared_path = shared.to_string_lossy();
    let user_path = user.to_string_lossy();
    let prebuilt_path = prebuilt.to_string_lossy();
    let staging_path = staging.to_string_lossy();
    let sync_path = sync.to_string_lossy();
    let user_sync_path = sync.join("frontend-user");
    let user_sync_path = user_sync_path.to_string_lossy();

    let raw_shared = unsafe { CStr::from_ptr(get_shared_data_dir()) };
    assert_eq!(raw_shared.to_str(), Ok(shared_path.as_ref()));
    let raw_user = unsafe { CStr::from_ptr(get_user_data_dir()) };
    assert_eq!(raw_user.to_str(), Ok(user_path.as_ref()));
    let raw_prebuilt = unsafe { CStr::from_ptr(get_prebuilt_data_dir()) };
    assert_eq!(raw_prebuilt.to_str(), Ok(prebuilt_path.as_ref()));
    let raw_staging = unsafe { CStr::from_ptr(get_staging_dir()) };
    assert_eq!(raw_staging.to_str(), Ok(staging_path.as_ref()));
    let raw_sync = unsafe { CStr::from_ptr(get_sync_dir()) };
    assert_eq!(raw_sync.to_str(), Ok(sync_path.as_ref()));
    let raw_user_id = unsafe { CStr::from_ptr(get_user_id()) };
    assert_eq!(raw_user_id.to_str(), Ok("frontend-user"));

    let mut buffer = vec![0 as c_char; 256];
    unsafe { get_shared_data_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_shared = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_shared.to_str(), Ok(shared_path.as_ref()));

    unsafe { get_user_data_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_user = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_user.to_str(), Ok(user_path.as_ref()));

    unsafe { get_prebuilt_data_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_prebuilt = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_prebuilt.to_str(), Ok(prebuilt_path.as_ref()));

    unsafe { get_staging_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_staging = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_staging.to_str(), Ok(staging_path.as_ref()));

    unsafe { get_sync_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_sync.to_str(), Ok(sync_path.as_ref()));

    unsafe { get_user_data_sync_dir(buffer.as_mut_ptr(), buffer.len()) };
    let copied_user_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_user_sync.to_str(), Ok(user_sync_path.as_ref()));

    let mut short_buffer = vec![0 as c_char; 8];
    unsafe { get_sync_dir_s(short_buffer.as_mut_ptr(), short_buffer.len()) };
    let truncated_sync = unsafe {
        std::slice::from_raw_parts(short_buffer.as_ptr().cast::<u8>(), short_buffer.len())
    };
    assert_eq!(truncated_sync, &sync_path.as_bytes()[..short_buffer.len()]);

    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_read_schema_state_labels() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let root = unique_temp_dir("state-label");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
switches:
  - name: ascii_mode
    states: [Native, Ascii]
    abbrev: [N, A]
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
    unsafe { setup(&traits) };

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let select_schema = api.select_schema.expect("frontend requires select_schema");
    let get_state_label = api
        .get_state_label
        .expect("frontend requires get_state_label");
    let get_state_label_abbreviated = api
        .get_state_label_abbreviated
        .expect("frontend requires get_state_label_abbreviated");

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("luna").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let ascii_mode = CString::new("ascii_mode").expect("literal should not contain NUL");
    let full_label = unsafe { get_state_label(session_id, ascii_mode.as_ptr(), TRUE) };
    assert!(!full_label.is_null());
    let full_label = unsafe { CStr::from_ptr(full_label) };
    assert_eq!(full_label.to_str(), Ok("Ascii"));

    let abbreviated =
        unsafe { get_state_label_abbreviated(session_id, ascii_mode.as_ptr(), TRUE, TRUE) };
    assert_eq!(abbreviated.length, 1);
    assert!(!abbreviated.str.is_null());
    let abbreviated_label = unsafe { CStr::from_ptr(abbreviated.str) };
    assert_eq!(abbreviated_label.to_str(), Ok("A"));

    let simplification = CString::new("simplification").expect("literal should not contain NUL");
    let radio =
        unsafe { get_state_label_abbreviated(session_id, simplification.as_ptr(), TRUE, TRUE) };
    assert_eq!(radio.length, "简".len());
    assert!(!radio.str.is_null());
    let radio_slice = unsafe { std::slice::from_raw_parts(radio.str.cast::<u8>(), radio.length) };
    assert_eq!(std::str::from_utf8(radio_slice), Ok("简"));

    let hidden_radio =
        unsafe { get_state_label_abbreviated(session_id, simplification.as_ptr(), FALSE, TRUE) };
    assert!(hidden_radio.str.is_null());
    assert_eq!(hidden_radio.length, 0);

    let missing = CString::new("missing").expect("literal should not contain NUL");
    assert!(unsafe { get_state_label(session_id, missing.as_ptr(), TRUE) }.is_null());
    assert!(unsafe { get_state_label(0, ascii_mode.as_ptr(), TRUE) }.is_null());
    assert!(unsafe { get_state_label(session_id, ptr::null(), TRUE) }.is_null());

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_iterate_candidates() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let candidate_list_begin = api
        .candidate_list_begin
        .expect("frontend requires candidate_list_begin");
    let candidate_list_next = api
        .candidate_list_next
        .expect("frontend requires candidate_list_next");
    let candidate_list_end = api
        .candidate_list_end
        .expect("frontend requires candidate_list_end");

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let mut iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_begin(session_id, &mut iterator) },
        TRUE
    );
    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, TRUE);

    let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    let comment = unsafe { CStr::from_ptr(iterator.candidate.comment) };
    assert_eq!(comment.to_str(), Ok("echo"));

    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 1);
    let preserved_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(preserved_text.to_str(), Ok("ni"));

    unsafe { candidate_list_end(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert!(iterator.candidate.text.is_null());
    assert!(iterator.candidate.comment.is_null());

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_iterate_candidates_from_index() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let candidate_list_from_index = api
        .candidate_list_from_index
        .expect("frontend requires candidate_list_from_index");
    let candidate_list_next = api
        .candidate_list_next
        .expect("frontend requires candidate_list_next");
    let candidate_list_end = api
        .candidate_list_end
        .expect("frontend requires candidate_list_end");

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let mut empty_iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut empty_iterator, 0) },
        FALSE
    );
    assert!(empty_iterator.ptr.is_null());

    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let mut iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut iterator, 0) },
        TRUE
    );
    assert_eq!(iterator.index, -1);
    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, TRUE);

    let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    unsafe { candidate_list_end(&mut iterator) };

    let mut past_end_iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut past_end_iterator, 1) },
        TRUE
    );
    assert_eq!(past_end_iterator.index, 0);
    assert_eq!(
        unsafe { candidate_list_next(&mut past_end_iterator) },
        FALSE
    );
    assert_eq!(past_end_iterator.index, 1);
    assert!(past_end_iterator.candidate.text.is_null());
    unsafe { candidate_list_end(&mut past_end_iterator) };

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}
