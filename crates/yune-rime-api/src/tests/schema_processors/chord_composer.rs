#[test]
fn schema_chord_composer_serializes_chord_on_key_release() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: ba
  output_format:
    - xlit/ab/xy/
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

醒\tyx\t100
形\txy\t90
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let current_input = || {
        let input = RimeGetInput(session_id);
        assert!(!input.is_null());
        // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
        unsafe { CStr::from_ptr(input) }
            .to_str()
            .expect("input should be valid UTF-8")
            .to_owned()
    };

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(current_input(), "");
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(current_input(), "");
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(current_input(), "yx");

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
    assert!(!candidates.is_empty());
    // SAFETY: candidate text pointers are populated by `RimeGetContext`.
    let top_text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(top_text, "醒");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_exposes_prompt_while_chording() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-prompt");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: ba
  algebra:
    - xlit/ab/xy/
  prompt_format:
    - xform/^(.+)$/<$1>/
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\tx\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 3);
    assert_eq!(context.composition.cursor_pos, 0);
    assert_eq!(context.composition.sel_start, 0);
    assert_eq!(context.composition.sel_end, 0);
    // SAFETY: context composition preedit was allocated by `RimeGetContext`.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("<x>")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: context composition preedit was allocated by `RimeGetContext`.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("x")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_cancels_active_chord_on_function_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-function-cancel");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: a
  output_format:
    - xlit/a/x/
  prompt_format:
    - xform/^(.+)$/<$1>/
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\tx\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: context composition preedit was allocated by `RimeGetContext`.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("<a>")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), FALSE);

    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, FALSE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK),
        FALSE
    );

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_binding_commits_raw_sequence() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-raw-binding");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: ab
  output_format:
    - xlit/ab/xy/
  bindings:
    Control+r: commit_raw_input
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\txy\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, K_RELEASE_MASK), TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("xy"));

    assert_eq!(RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ab"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_clears_raw_sequence_after_context_commit() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-context-commit-clears-raw");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: ab
  output_format:
    - xlit/ab/xy/
  bindings:
    Control+r: commit_raw_input
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\txy\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, K_RELEASE_MASK), TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("xy"));

    assert_eq!(RimeCommitComposition(session_id), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("形"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(
        RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK),
        FALSE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_clears_raw_sequence_after_direct_commit_output() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-direct-commit-clears-raw");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chord.schema.yaml"),
        "\
schema:
  schema_id: chord
  name: Chord
engine:
  processors:
    - chord_composer
chord_composer:
  alphabet: a
  output_format:
    - \"xform/^a$/ /\"
  bindings:
    Control+r: commit_raw_input
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
    let schema_id = CString::new("chord").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);

    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok(" "));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(
        RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK),
        FALSE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_chord_composer_honors_modifier_use_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-composer-modifiers");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");

    let schema = |schema_id: &str, use_option: &str| {
        let use_option = if use_option.is_empty() {
            String::new()
        } else {
            format!("  {use_option}: true\n")
        };
        format!(
            "\
schema:
  schema_id: {schema_id}
  name: {schema_id}
engine:
  processors:
    - chord_composer
  translators:
    - table_translator
chord_composer:
  alphabet: a
{use_option}  output_format:
    - xlit/a/x/
translator:
  dictionary: chord
  enable_completion: false
  enable_sentence: false
"
        )
    };
    fs::write(staging.join("plain.schema.yaml"), schema("plain", ""))
        .expect("plain schema should be written");
    fs::write(
        staging.join("control.schema.yaml"),
        schema("control", "use_control"),
    )
    .expect("control schema should be written");
    fs::write(
        staging.join("shift.schema.yaml"),
        schema("shift", "use_shift"),
    )
    .expect("shift schema should be written");
    fs::write(staging.join("alt.schema.yaml"), schema("alt", "use_alt"))
        .expect("alt schema should be written");
    fs::write(
        staging.join("super.schema.yaml"),
        schema("super", "use_super"),
    )
    .expect("super schema should be written");
    fs::write(staging.join("caps.schema.yaml"), schema("caps", "use_caps"))
        .expect("caps schema should be written");
    fs::write(
        shared.join("chord.dict.yaml"),
        "\
---
name: chord
version: '0.1'
sort: original
...

形\tx\t100
",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let current_input = || {
        let input = RimeGetInput(session_id);
        assert!(!input.is_null());
        // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
        unsafe { CStr::from_ptr(input) }
            .to_str()
            .expect("input should be valid UTF-8")
            .to_owned()
    };

    let plain_schema = CString::new("plain").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, plain_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK | K_RELEASE_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_LOCK_MASK), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_LOCK_MASK | K_RELEASE_MASK),
        FALSE
    );
    assert_eq!(current_input(), "");

    let control_schema = CString::new("control").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, control_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    let shift_schema = CString::new("shift").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, shift_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_SHIFT_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_SHIFT_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    let alt_schema = CString::new("alt").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, alt_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_ALT_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_ALT_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    let super_schema = CString::new("super").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, super_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_SUPER_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_SUPER_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    let caps_schema = CString::new("caps").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, caps_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_LOCK_MASK), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_LOCK_MASK | K_RELEASE_MASK),
        TRUE
    );
    assert_eq!(current_input(), "x");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
