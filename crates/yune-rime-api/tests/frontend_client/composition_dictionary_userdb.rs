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
fn frontend_style_userdb_learning_survives_session_recreation() {
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
    let commit_composition = api
        .commit_composition
        .expect("frontend requires commit_composition");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");

    let root = unique_temp_dir("userdb-learning");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("learn.schema.yaml"),
        "schema:\n  schema_id: learn\n  name: Learn\nengine:\n  translators:\n    - table_translator\n    - echo_translator\ntranslator:\n  dictionary: learn\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("learn.dict.yaml"),
        "---\nname: learn\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nni\t你\t10\nni hao\t你好\t9\n",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let schema_id = CString::new("learn").expect("schema id should be valid");
    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as c_int, 0), TRUE);
    assert_eq!(commit_composition(session_id), TRUE);
    assert_eq!(destroy_session(session_id), TRUE);
    let store_path = user.join("learn.userdb");
    let stored = fs::read_to_string(&store_path).expect("store should be readable");
    fs::write(&store_path, format!("{stored}ni hao \t你好\tc=1 d=1 t=1\n"))
        .expect("predictive store entry should be appended");

    let reloaded_session = create_session();
    assert_ne!(reloaded_session, 0);
    assert_eq!(
        unsafe { select_schema(reloaded_session, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(reloaded_session, 'n' as c_int, 0), TRUE);
    assert_eq!(process_key(reloaded_session, 'i' as c_int, 0), TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(reloaded_session, &mut context) }, TRUE);
    assert!(context.menu.num_candidates >= 2);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_values = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    assert!(candidate_values.iter().any(|(text, _)| text == "你"));
    assert!(candidate_values
        .iter()
        .any(|(text, comment)| text == "你好" && comment == "~hao"));
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(reloaded_session), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_page_schema_dictionary_candidates() {
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
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");
    let highlight_candidate = api
        .highlight_candidate
        .expect("frontend requires highlight_candidate");
    let highlight_candidate_on_current_page = api
        .highlight_candidate_on_current_page
        .expect("frontend requires highlight_candidate_on_current_page");
    let change_page = api.change_page.expect("frontend requires change_page");
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");

    let root = unique_temp_dir("schema-dictionary-paging");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n  alternative_select_labels: [Alpha, Beta]\nengine:\n  translators:\n    - table_translator\n    - echo_translator\ntranslator:\n  dictionary: frontend\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("frontend.dict.yaml"),
        "\
---\nname: frontend\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nba\t八\t10\nba\t吧\t9\nba\t爸\t8\nba\t巴\t7\nba\t把\t6\nba\t拔\t5\n",
    )
    .expect("dictionary should be written");

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
    assert_eq!(highlight_candidate(session_id, 0), FALSE);
    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(highlight_candidate(session_id, 3), TRUE);

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_size, 2);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    assert_eq!(context.menu.num_candidates, 2);
    assert_eq!(
        unsafe { CStr::from_ptr(context.menu.select_keys) }.to_str(),
        Ok("AB")
    );
    assert!(!context.select_labels.is_null());
    let select_labels = unsafe {
        std::slice::from_raw_parts(context.select_labels, context.menu.page_size as usize)
    };
    assert_eq!(
        unsafe { CStr::from_ptr(select_labels[0]) }.to_str(),
        Ok("Alpha")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(select_labels[1]) }.to_str(),
        Ok("Beta")
    );
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
        Ok("巴")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(highlight_candidate_on_current_page(session_id, 0), TRUE);
    assert_eq!(change_page(session_id, FALSE), TRUE);
    assert_eq!(select_candidate_on_current_page(session_id, 1), TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("拔"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
}

#[test]
fn frontend_style_schema_dictionary_loads_import_tables() {
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

    let root = unique_temp_dir("schema-dictionary-imports");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("importing.schema.yaml"),
        "\
schema:\n  schema_id: importing\n  name: Importing\nengine:\n  translators:\n    - table_translator\n    - echo_translator\ntranslator:\n  dictionary: primary\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("primary.dict.yaml"),
        "\
---\nname: primary\nversion: '1'\nsort: by_weight\nimport_tables: [secondary]\n...\n八\tba\t1\n",
    )
    .expect("primary dictionary should be written");
    fs::write(
        shared.join("secondary.dict.yaml"),
        "\
---\nname: secondary\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nba\t爸\t9\n",
    )
    .expect("imported dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("importing").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 3);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("爸")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
        Ok("八")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
}
