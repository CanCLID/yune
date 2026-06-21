#[test]
fn schema_punctuator_processor_handles_commit_and_unique_preview_punctuation() {
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
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: RimeGetContext populated a valid NUL-terminated preedit string.
    let text = unsafe { CStr::from_ptr(context.composition.preedit) };
    assert_eq!(text.to_str(), Ok("。"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    RimeClearComposition(session_id);

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
fn schema_punctuator_processor_loads_namespaced_prescriptions() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-namespaced");
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
    - punctuator@custom_processor
  translators:
    - punct_translator@custom_translator
    - echo_translator
punctuator:
  half_shape:
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

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: RimeGetContext populated a valid NUL-terminated preedit string.
    let text = unsafe { CStr::from_ptr(context.composition.preedit) };
    assert_eq!(text.to_str(), Ok("。"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

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
    assert_eq!(read_commit(), "２");

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
    assert_eq!(read_commit(), "４");
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
            "A".to_owned(),
            vec!["A".to_owned(), "B".to_owned()],
            0
        )
    );

    assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
    assert_eq!(
        context_state(),
        (
            "B".to_owned(),
            vec!["A".to_owned(), "B".to_owned()],
            1
        )
    );

    assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
    assert_eq!(
        context_state(),
        (
            "A".to_owned(),
            vec!["A".to_owned(), "B".to_owned()],
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
fn schema_punctuator_processor_previews_paired_punctuation_alternately() {
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

    let pair_preview = || {
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        assert_eq!(RimeProcessKey(session_id, '(' as i32, 0), TRUE);
        // SAFETY: commit points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        // SAFETY: RimeGetContext populated a valid NUL-terminated preedit string.
        let text = unsafe { CStr::from_ptr(context.composition.preedit) }
            .to_str()
            .expect("preedit text should be valid UTF-8")
            .to_owned();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        text
    };

    assert_eq!(pair_preview(), "（");
    assert_eq!(pair_preview(), "（）");
    assert_eq!(pair_preview(), "（）（");
    RimeClearComposition(session_id);

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };

    assert_eq!(pair_preview(), "〔");
    assert_eq!(pair_preview(), "〔〕");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
