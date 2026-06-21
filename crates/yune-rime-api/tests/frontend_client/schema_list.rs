#[test]
fn frontend_style_schema_list_translator_lists_and_selects_schemas() {
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
    let get_current_schema = api
        .get_current_schema
        .expect("frontend requires get_current_schema");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");

    let root = unique_temp_dir("schema-list-translator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "schema_list:\n  - schema: luna\n  - schema: bopomofo\n",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nengine:\n  translators:\n    - schema_list_translator\n    - echo_translator\n",
    )
    .expect("luna schema config should be written");
    fs::write(
        staging.join("bopomofo.schema.yaml"),
        "schema:\n  schema_id: bopomofo\n  name: Bopomofo\n",
    )
    .expect("bopomofo schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("luna").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(candidate_texts, ["Luna", "Bopomofo", "x"]);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(select_candidate_on_current_page(session_id, 1), TRUE);
    let mut schema_buffer = vec![0 as c_char; 32];
    assert_eq!(
        unsafe { get_current_schema(session_id, schema_buffer.as_mut_ptr(), schema_buffer.len()) },
        TRUE
    );
    let selected_schema = unsafe { CStr::from_ptr(schema_buffer.as_ptr()) };
    assert_eq!(selected_schema.to_str(), Ok("bopomofo"));

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_list_translator_hides_lone_schema_when_configured() {
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
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let get_schema_list = api
        .get_schema_list
        .expect("frontend requires get_schema_list");
    let free_schema_list = api
        .free_schema_list
        .expect("frontend requires free_schema_list");

    let root = unique_temp_dir("schema-list-translator-hide-lone");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "schema_list:\n  - schema: luna\n",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nswitcher:\n  hide_lone_schema: true\nengine:\n  translators:\n    - schema_list_translator\n    - echo_translator\n",
    )
    .expect("luna schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let mut schema_list = empty_schema_list();
    assert_eq!(unsafe { get_schema_list(&mut schema_list) }, TRUE);
    assert_eq!(
        schema_list.size, 1,
        "RimeGetSchemaList should still expose the single deployed schema"
    );
    unsafe { free_schema_list(&mut schema_list) };

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("luna").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(candidate_texts, ["x"]);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_list_translator_orders_by_access_time() {
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
    let get_current_schema = api
        .get_current_schema
        .expect("frontend requires get_current_schema");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");

    let root = unique_temp_dir("schema-list-translator-access-time");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "schema_list:\n  - schema: luna\n  - schema: bopomofo\n  - schema: cangjie\n",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nengine:\n  translators:\n    - schema_list_translator\n    - echo_translator\n",
    )
    .expect("luna schema config should be written");
    fs::write(
        staging.join("bopomofo.schema.yaml"),
        "schema:\n  schema_id: bopomofo\n  name: Bopomofo\n",
    )
    .expect("bopomofo schema config should be written");
    fs::write(
        staging.join("cangjie.schema.yaml"),
        "schema:\n  schema_id: cangjie\n  name: Cangjie\n",
    )
    .expect("cangjie schema config should be written");
    fs::write(
        user.join("user.yaml"),
        "var:\n  schema_access_time:\n    bopomofo: 100\n    cangjie: 200\n",
    )
    .expect("user config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("luna").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(candidate_texts, ["Luna", "Cangjie", "Bopomofo", "x"]);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(select_candidate_on_current_page(session_id, 1), TRUE);
    let mut schema_buffer = vec![0 as c_char; 32];
    assert_eq!(
        unsafe { get_current_schema(session_id, schema_buffer.as_mut_ptr(), schema_buffer.len()) },
        TRUE
    );
    let selected_schema = unsafe { CStr::from_ptr(schema_buffer.as_ptr()) };
    assert_eq!(selected_schema.to_str(), Ok("cangjie"));

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_list_translator_fix_order_uses_configured_order() {
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
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");

    let root = unique_temp_dir("schema-list-translator-fix-order");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "schema_list:\n  - schema: luna\n  - schema: bopomofo\n  - schema: cangjie\n",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nswitcher:\n  fix_schema_list_order: true\nengine:\n  translators:\n    - schema_list_translator\n    - echo_translator\n",
    )
    .expect("luna schema config should be written");
    fs::write(
        staging.join("bopomofo.schema.yaml"),
        "schema:\n  schema_id: bopomofo\n  name: Bopomofo\n",
    )
    .expect("bopomofo schema config should be written");
    fs::write(
        staging.join("cangjie.schema.yaml"),
        "schema:\n  schema_id: cangjie\n  name: Cangjie\n",
    )
    .expect("cangjie schema config should be written");
    fs::write(
        user.join("user.yaml"),
        "var:\n  schema_access_time:\n    bopomofo: 100\n    cangjie: 200\n",
    )
    .expect("user config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("luna").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'x' as c_int, 0), TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(candidate_texts, ["Luna", "Bopomofo", "Cangjie", "x"]);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
