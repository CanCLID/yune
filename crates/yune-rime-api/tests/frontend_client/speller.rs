#[test]
fn frontend_style_schema_speller_gates_spelling_input() {
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
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");

    let root = unique_temp_dir("schema-speller");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("spelling.schema.yaml"),
        "\
schema:\n  schema_id: spelling\n  name: Spelling\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: ab\n  initials: a\n  finals: b\n  delimiter: \"'\"\n  use_space: true\ntranslator:\n  dictionary: spelling\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("spelling.dict.yaml"),
        "\
---\nname: spelling\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nab\tAB\t1\n",
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
    let schema_id = CString::new("spelling").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(process_key(session_id, 'b' as i32, 0), FALSE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ab"));

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 2);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("AB")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(process_key(session_id, 'b' as i32, 0), FALSE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ab"));

    assert_eq!(process_key(session_id, ' ' as i32, 0), TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ab "));

    assert_eq!(process_key(session_id, 'c' as i32, 0), FALSE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ab "));

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_full_shape_formats_commits_and_unhandled_ascii_keys() {
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
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");
    let set_option = api.set_option.expect("frontend requires set_option");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");
    let get_input = api.get_input.expect("frontend requires get_input");

    let root = unique_temp_dir("schema-full-shape");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("shape.schema.yaml"),
        "\
schema:\n  schema_id: shape\n  name: Shape\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: ab\ntranslator:\n  dictionary: shape\n  enable_completion: false\n  enable_sentence: false\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("shape.dict.yaml"),
        "\
---\nname: shape\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nab\tABC\t1\n",
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
    let schema_id = CString::new("shape").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    unsafe { set_option(session_id, full_shape.as_ptr(), TRUE) };

    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(select_candidate_on_current_page(session_id, 0), TRUE);
    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(commit.text) }.to_str(),
        Ok("ＡＢＣ")
    );
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    assert_eq!(process_key(session_id, '?' as i32, 0), TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("？"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_speller_auto_clear_modes() {
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

    let root = unique_temp_dir("schema-speller-auto-clear");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("auto_clear.schema.yaml"),
        "\
schema:\n  schema_id: auto_clear\n  name: Auto Clear\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: abxy\n  auto_clear: auto\ntranslator:\n  dictionary: auto_clear\n  enable_sentence: false\n",
    )
    .expect("auto_clear schema config should be written");
    fs::write(
        staging.join("manual_clear.schema.yaml"),
        "\
schema:\n  schema_id: manual_clear\n  name: Manual Clear\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: abxy\n  auto_clear: manual\ntranslator:\n  dictionary: auto_clear\n  enable_sentence: false\n",
    )
    .expect("manual_clear schema config should be written");
    fs::write(
        staging.join("max_clear.schema.yaml"),
        "\
schema:\n  schema_id: max_clear\n  name: Max Clear\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: abxy\n  auto_clear: max_length\n  max_code_length: 2\ntranslator:\n  dictionary: auto_clear\n  enable_sentence: false\n",
    )
    .expect("max_clear schema config should be written");
    fs::write(
        shared.join("auto_clear.dict.yaml"),
        "\
---\nname: auto_clear\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nab\tAB\t1\n",
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

    let schema_id = CString::new("auto_clear").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'x' as i32, 0), TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    let schema_id = CString::new("manual_clear").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'x' as i32, 0), TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("x"));
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("a"));

    let schema_id = CString::new("max_clear").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'x' as i32, 0), TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("x"));
    assert_eq!(process_key(session_id, 'y' as i32, 0), TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("xy"));
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("a"));

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_speller_auto_selects_at_max_code_length() {
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
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let root = unique_temp_dir("schema-speller-auto-select-max-code");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("auto_select_max.schema.yaml"),
        "\
schema:\n  schema_id: auto_select_max\n  name: Auto Select Max\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: abc\n  max_code_length: 2\ntranslator:\n  dictionary: auto_select_max\n  enable_sentence: false\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("auto_select_max.dict.yaml"),
        "\
---\nname: auto_select_max\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nab\tAB\t1\n",
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
    let schema_id = CString::new("auto_select_max").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

    assert_eq!(process_key(session_id, 'c' as i32, 0), TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("AB"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("c"));

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_speller_auto_selects_unique_table_candidate() {
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
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let root = unique_temp_dir("schema-speller-auto-select-unique");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("auto_select_unique.schema.yaml"),
        "\
schema:\n  schema_id: auto_select_unique\n  name: Auto Select Unique\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: ab\n  auto_select: true\ntranslator:\n  dictionary: auto_select_unique\n  enable_completion: false\n  enable_sentence: false\n",
    )
    .expect("unique schema config should be written");
    fs::write(
        staging.join("auto_select_ambiguous.schema.yaml"),
        "\
schema:\n  schema_id: auto_select_ambiguous\n  name: Auto Select Ambiguous\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: ab\n  auto_select: true\ntranslator:\n  dictionary: auto_select_ambiguous\n  enable_completion: false\n  enable_sentence: false\n",
    )
    .expect("ambiguous schema config should be written");
    fs::write(
        shared.join("auto_select_unique.dict.yaml"),
        "\
---\nname: auto_select_unique\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nab\tAB\t1\n",
    )
    .expect("unique dictionary should be written");
    fs::write(
        shared.join("auto_select_ambiguous.dict.yaml"),
        "\
---\nname: auto_select_ambiguous\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nab\tAB\t1\nab\tAlt\t1\n",
    )
    .expect("ambiguous dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let unique_session_id = create_session();
    assert_ne!(unique_session_id, 0);
    let unique_schema_id =
        CString::new("auto_select_unique").expect("unique schema id should be valid");
    assert_eq!(
        unsafe { select_schema(unique_session_id, unique_schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(unique_session_id, 'a' as i32, 0), TRUE);
    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(unique_session_id, &mut commit) }, FALSE);
    assert_eq!(process_key(unique_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(unsafe { get_commit(unique_session_id, &mut commit) }, TRUE);
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("AB"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    let input = get_input(unique_session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(destroy_session(unique_session_id), TRUE);

    let ambiguous_session_id = create_session();
    assert_ne!(ambiguous_session_id, 0);
    let ambiguous_schema_id =
        CString::new("auto_select_ambiguous").expect("ambiguous schema id should be valid");
    assert_eq!(
        unsafe { select_schema(ambiguous_session_id, ambiguous_schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(ambiguous_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(process_key(ambiguous_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(
        unsafe { get_commit(ambiguous_session_id, &mut commit) },
        FALSE
    );
    let input = get_input(ambiguous_session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ab"));
    let mut context = empty_context();
    assert_eq!(
        unsafe { get_context(ambiguous_session_id, &mut context) },
        TRUE
    );
    assert_eq!(context.menu.num_candidates, 3);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("AB")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
        Ok("Alt")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);
    assert_eq!(destroy_session(ambiguous_session_id), TRUE);

    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_speller_auto_select_pattern_gates_unique_candidate() {
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
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let root = unique_temp_dir("schema-speller-auto-select-pattern");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("auto_select_pattern.schema.yaml"),
        "\
schema:\n  schema_id: auto_select_pattern\n  name: Auto Select Pattern\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: abc\n  auto_select: true\n  auto_select_pattern: ab\n  max_code_length: 3\ntranslator:\n  dictionary: auto_select_pattern\n  enable_completion: false\n  enable_sentence: false\n",
    )
    .expect("pattern schema config should be written");
    fs::write(
        shared.join("auto_select_pattern.dict.yaml"),
        "\
---\nname: auto_select_pattern\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nab\tAB\t1\nac\tAC\t1\n",
    )
    .expect("pattern dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let matching_session_id = create_session();
    assert_ne!(matching_session_id, 0);
    let schema_id = CString::new("auto_select_pattern").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(matching_session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(matching_session_id, 'a' as i32, 0), TRUE);
    let mut commit = empty_commit();
    assert_eq!(
        unsafe { get_commit(matching_session_id, &mut commit) },
        FALSE
    );
    assert_eq!(process_key(matching_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(
        unsafe { get_commit(matching_session_id, &mut commit) },
        TRUE
    );
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("AB"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    let input = get_input(matching_session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(destroy_session(matching_session_id), TRUE);

    let nonmatching_session_id = create_session();
    assert_ne!(nonmatching_session_id, 0);
    assert_eq!(
        unsafe { select_schema(nonmatching_session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(nonmatching_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(process_key(nonmatching_session_id, 'c' as i32, 0), TRUE);
    assert_eq!(
        unsafe { get_commit(nonmatching_session_id, &mut commit) },
        FALSE
    );
    let input = get_input(nonmatching_session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ac"));
    assert_eq!(destroy_session(nonmatching_session_id), TRUE);

    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_speller_algebra_expands_table_lookup_spellings() {
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

    let root = unique_temp_dir("schema-speller-algebra");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("algebra.schema.yaml"),
        "\
schema:\n  schema_id: algebra\n  name: Algebra\nengine:\n  processors:\n    - speller\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: abcdegilnopuvxyz\n  algebra:\n    - xlit/zyx/abc/\n    - xform/^lue$/lve/\n    - derive/^nv$/nu/\n    - fuzz/^bing$/pin/\n    - erase/^gone$/\ntranslator:\n  dictionary: algebra\n  enable_completion: false\n  enable_sentence: false\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("algebra.dict.yaml"),
        "\
---\nname: algebra\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nlue\t略\t1\nnv\t女\t1\nbing\t病\t1\npin\t平\t1\nzyx\t照\t1\ngone\t删\t1\n",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let lve_session_id = create_session();
    assert_ne!(lve_session_id, 0);
    let schema_id = CString::new("algebra").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(lve_session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "lve".chars() {
        assert_eq!(process_key(lve_session_id, ch as i32, 0), TRUE);
    }
    let mut context = empty_context();
    assert_eq!(unsafe { get_context(lve_session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("略")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].comment) }.to_str(),
        Ok("lue")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);
    assert_eq!(destroy_session(lve_session_id), TRUE);

    let nu_session_id = create_session();
    assert_ne!(nu_session_id, 0);
    assert_eq!(
        unsafe { select_schema(nu_session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "nu".chars() {
        assert_eq!(process_key(nu_session_id, ch as i32, 0), TRUE);
    }
    let mut context = empty_context();
    assert_eq!(unsafe { get_context(nu_session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("女")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].comment) }.to_str(),
        Ok("nv")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);
    assert_eq!(destroy_session(nu_session_id), TRUE);

    let pin_session_id = create_session();
    assert_ne!(pin_session_id, 0);
    assert_eq!(
        unsafe { select_schema(pin_session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "pin".chars() {
        assert_eq!(process_key(pin_session_id, ch as i32, 0), TRUE);
    }
    let mut context = empty_context();
    assert_eq!(unsafe { get_context(pin_session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert!(candidates.len() >= 2);
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("平")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
        Ok("病")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);
    assert_eq!(destroy_session(pin_session_id), TRUE);

    let abc_session_id = create_session();
    assert_ne!(abc_session_id, 0);
    assert_eq!(
        unsafe { select_schema(abc_session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "abc".chars() {
        assert_eq!(process_key(abc_session_id, ch as i32, 0), TRUE);
    }
    let mut context = empty_context();
    assert_eq!(unsafe { get_context(abc_session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("照")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].comment) }.to_str(),
        Ok("zyx")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);
    assert_eq!(destroy_session(abc_session_id), TRUE);

    let gone_session_id = create_session();
    assert_ne!(gone_session_id, 0);
    assert_eq!(
        unsafe { select_schema(gone_session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "gone".chars() {
        assert_eq!(process_key(gone_session_id, ch as i32, 0), TRUE);
    }
    let mut context = empty_context();
    assert_eq!(unsafe { get_context(gone_session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert!(candidates
        .iter()
        .all(|candidate| unsafe { CStr::from_ptr(candidate.text).to_str() != Ok("删") }));
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);
    assert_eq!(destroy_session(gone_session_id), TRUE);

    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_schema_speller_auto_selects_previous_match_with_express_editor() {
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
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let root = unique_temp_dir("schema-speller-auto-select-previous-match");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("auto_select_previous.schema.yaml"),
        "\
schema:\n  schema_id: auto_select_previous\n  name: Auto Select Previous\nengine:\n  processors:\n    - speller\n    - express_editor\n  translators:\n    - table_translator\n    - echo_translator\nspeller:\n  alphabet: abc\n  auto_select: true\ntranslator:\n  dictionary: auto_select_previous\n  enable_completion: false\n  enable_sentence: false\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("auto_select_previous.dict.yaml"),
        "\
---\nname: auto_select_previous\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nab\tAB\t1\nab\tAlt\t1\n",
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
    let schema_id = CString::new("auto_select_previous").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);
    assert_eq!(process_key(session_id, 'c' as i32, 0), TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("AB"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    let input = get_input(session_id);
    assert!(!input.is_null());
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("c"));
    assert_eq!(destroy_session(session_id), TRUE);

    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
