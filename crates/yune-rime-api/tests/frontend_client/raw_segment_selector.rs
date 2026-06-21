#[test]
fn frontend_style_raw_segment_selector_does_not_select_candidates() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
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
    let select_schema = api.select_schema.expect("frontend requires select_schema");
    let get_input = api.get_input.expect("frontend requires get_input");
    let get_commit = api.get_commit.expect("frontend requires get_commit");

    let root = unique_temp_dir("schema-raw-selector");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("raw.schema.yaml"),
        "\
schema:\n  schema_id: raw\n  name: Raw\nengine:\n  segmentors:\n    - fallback_segmentor\n  translators:\n    - echo_translator\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("raw").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(process_key(session_id, '1' as c_int, 0), TRUE);

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("a1"));
    assert_eq!(destroy_session(session_id), TRUE);

    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
