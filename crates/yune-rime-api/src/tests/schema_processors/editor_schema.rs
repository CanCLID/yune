#[test]
fn schema_express_editor_return_commits_raw_input() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-express-editor-return");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("fluid.schema.yaml"),
        "\
schema:
  schema_id: fluid
  name: Fluid
engine:
  processors:
    - speller
    - fluid_editor
  translators:
    - table_translator
speller:
  alphabet: in
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("fluid schema config should be written");
    fs::write(
        staging.join("express.schema.yaml"),
        "\
schema:
  schema_id: express
  name: Express
engine:
  processors:
    - speller
    - express_editor
  translators:
    - table_translator
speller:
  alphabet: in
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("express schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni\t100
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

    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);

    let commit_text = |session_id| {
        let mut commit = RimeCommit {
            data_size: std::mem::size_of::<RimeCommit>() as i32,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to writable storage.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was allocated by the shim above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };

    let fluid_session = RimeCreateSession();
    let fluid_schema = CString::new("fluid").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(fluid_session, fluid_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(fluid_session, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(fluid_session, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(fluid_session, return_keycode, 0), TRUE);
    assert_eq!(commit_text(fluid_session), "你");
    assert_eq!(RimeDestroySession(fluid_session), TRUE);

    let express_session = RimeCreateSession();
    let express_schema = CString::new("express").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(express_session, express_schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(express_session, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(express_session, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(express_session, return_keycode, 0), TRUE);
    assert_eq!(commit_text(express_session), "ni");
    let input = RimeGetInput(express_session);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(RimeDestroySession(express_session), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_editor_bindings_override_default_keymap() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-editor-bindings");
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
    - speller
    - fluid_editor
  translators:
    - table_translator
speller:
  alphabet: abcni
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
editor:
  bindings:
    Return: noop
    Control+r: commit_raw_input
    Control+d: delete_candidate
    Control+x: delete
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tni\t100
呢\tni\t90
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

    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);

    let commit_text = |session_id| {
        let mut commit = RimeCommit {
            data_size: std::mem::size_of::<RimeCommit>() as i32,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to writable storage.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was allocated by the shim above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, return_keycode, 0), TRUE);
    let mut empty_commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(session_id, &mut empty_commit) },
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'd' as i32, K_CONTROL_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(commit_text(session_id), "呢");

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK), TRUE);
    assert_eq!(commit_text(session_id), "ni");

    let raw_input = CString::new("abc").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeSetInput(session_id, raw_input.as_ptr()) },
        TRUE
    );
    RimeSetCaretPos(session_id, 1);
    assert_eq!(RimeProcessKey(session_id, 'x' as i32, K_CONTROL_MASK), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ac"));

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_editor_char_handler_controls_printable_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-editor-char-handler");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");

    let schema = |schema_id: &str, processor: &str, char_handler: Option<&str>| {
        let editor_config = char_handler
            .map(|handler| format!("editor:\n  char_handler: {handler}\n"))
            .unwrap_or_default();
        format!(
            "\
schema:
  schema_id: {schema_id}
  name: {schema_id}
engine:
  processors:
    - {processor}
  translators:
    - table_translator
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
{editor_config}"
        )
    };
    fs::write(
        staging.join("fluid.schema.yaml"),
        schema("fluid", "fluid_editor", None),
    )
    .expect("fluid schema config should be written");
    fs::write(
        staging.join("express.schema.yaml"),
        schema("express", "express_editor", None),
    )
    .expect("express schema config should be written");
    fs::write(
        staging.join("express_add.schema.yaml"),
        schema("express_add", "express_editor", Some("add_to_input")),
    )
    .expect("express add schema config should be written");
    fs::write(
        staging.join("fluid_direct.schema.yaml"),
        schema("fluid_direct", "fluid_editor", Some("direct_commit")),
    )
    .expect("fluid direct schema config should be written");
    fs::write(
        staging.join("fluid_noop.schema.yaml"),
        schema("fluid_noop", "fluid_editor", Some("noop")),
    )
    .expect("fluid noop schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: original
...

你\tn\t100
泥\tni\t90
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

    let commit_text = |session_id| {
        let mut commit = RimeCommit {
            data_size: std::mem::size_of::<RimeCommit>() as i32,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to writable storage.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
        let text = unsafe { CStr::from_ptr(commit.text) }
            .to_str()
            .expect("commit should be valid UTF-8")
            .to_owned();
        // SAFETY: commit.text was allocated by the shim above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        text
    };
    let no_commit = |session_id| {
        let mut commit = RimeCommit {
            data_size: std::mem::size_of::<RimeCommit>() as i32,
            text: std::ptr::null_mut(),
        };
        // SAFETY: commit points to writable storage.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    };
    let current_input = |session_id| {
        let input = RimeGetInput(session_id);
        assert!(!input.is_null());
        // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
        unsafe { CStr::from_ptr(input) }
            .to_str()
            .expect("input should be valid UTF-8")
            .to_owned()
    };
    let create_seeded_session = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        let input = CString::new("n").expect("input should be valid");
        // SAFETY: input is a valid NUL-terminated string.
        assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
        session_id
    };

    let fluid = create_seeded_session("fluid");
    assert_eq!(RimeProcessKey(fluid, 'i' as i32, 0), TRUE);
    assert_eq!(current_input(fluid), "ni");
    no_commit(fluid);
    assert_eq!(RimeDestroySession(fluid), TRUE);

    let express = create_seeded_session("express");
    assert_eq!(RimeProcessKey(express, 'i' as i32, 0), FALSE);
    assert_eq!(commit_text(express), "你");
    assert_eq!(current_input(express), "");
    assert_eq!(RimeDestroySession(express), TRUE);

    let express_add = create_seeded_session("express_add");
    assert_eq!(RimeProcessKey(express_add, 'i' as i32, 0), TRUE);
    assert_eq!(current_input(express_add), "ni");
    no_commit(express_add);
    assert_eq!(RimeDestroySession(express_add), TRUE);

    let fluid_direct = create_seeded_session("fluid_direct");
    assert_eq!(RimeProcessKey(fluid_direct, 'i' as i32, 0), FALSE);
    assert_eq!(commit_text(fluid_direct), "你");
    assert_eq!(current_input(fluid_direct), "");
    assert_eq!(RimeDestroySession(fluid_direct), TRUE);

    let fluid_noop = create_seeded_session("fluid_noop");
    assert_eq!(RimeProcessKey(fluid_noop, 'i' as i32, 0), FALSE);
    no_commit(fluid_noop);
    assert_eq!(current_input(fluid_noop), "n");
    assert_eq!(RimeDestroySession(fluid_noop), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
