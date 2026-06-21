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
fn fresh_session_uses_first_deployed_schema_for_schema_specific_settings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    RimeFinalize();
    let root = unique_temp_dir("fresh-session-first-deployed-schema");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: jyut6ping3\n  - schema: luna_pinyin\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("jyut6ping3.schema.yaml"),
        "schema:\n  schema_id: jyut6ping3\n  name: Jyutping\n",
    )
    .expect("first schema config should be written");
    fs::write(
        shared.join("luna_pinyin.schema.yaml"),
        "schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\n",
    )
    .expect("second schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeInitialize(&traits) };
    let _ = RimeStartMaintenance(FALSE);

    let session_id = RimeCreateSession();
    assert_ne!(session_id, 0);
    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_id = unsafe { CStr::from_ptr(status.schema_id) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_name = unsafe { CStr::from_ptr(status.schema_name) }
        .to_string_lossy()
        .into_owned();
    assert_eq!(status_schema_id, "jyut6ping3");
    assert_eq!(status_schema_name, "Jyutping");
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    let schema_id = CString::new(status_schema_id).expect("schema id should be valid");
    let mut config = empty_config();
    // SAFETY: the status schema id is a valid NUL-terminated logical schema id
    // and config points to writable storage.
    assert_eq!(unsafe { RimeSchemaOpen(schema_id.as_ptr(), &mut config) }, TRUE);
    // SAFETY: config was opened by `RimeSchemaOpen`.
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    RimeFinalize();
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeInitialize(&reset_traits) };
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
fn select_schema_loads_librime_switch_translator_candidates() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator");
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
    - switch_translator
    - echo_translator
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
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
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
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
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ 西文".to_owned()),
            ("简体".to_owned(), String::new()),
            ("繁體".to_owned(), " ✓".to_owned()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_preserves_missing_state_indices_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-missing-states");
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
    - switch_translator
    - echo_translator
switches:
  - name: ascii_mode
    states: [中文]
  - options: [simplification, traditional, emoji]
    states: [简体, ~, 表情]
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
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
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
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ ".to_owned()),
            ("简体".to_owned(), " ✓".to_owned()),
            ("表情".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 2), TRUE);
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    let emoji = CString::new("emoji").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(unsafe { RimeGetOption(session_id, emoji.as_ptr()) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_persists_librime_save_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-save-options");
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
    - switch_translator
    - echo_translator
switcher:
  save_options: [ascii_mode, simplification, traditional]
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    reset: 0
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
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);

    let mut user_config = empty_config();
    let user_id = CString::new("user").expect("config id should be valid");
    // SAFETY: config id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeUserConfigOpen(user_id.as_ptr(), &mut user_config) },
        TRUE
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/ascii_mode"),
        Some(TRUE)
    );
    // SAFETY: config owns state allocated by the shim.
    assert_eq!(unsafe { RimeConfigClose(&mut user_config) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'y' as c_int, 0), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 2), TRUE);

    let mut user_config = empty_config();
    // SAFETY: config id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeUserConfigOpen(user_id.as_ptr(), &mut user_config) },
        TRUE
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/ascii_mode"),
        Some(TRUE)
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/simplification"),
        Some(FALSE)
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/traditional"),
        Some(TRUE)
    );
    // SAFETY: config owns state allocated by the shim.
    assert_eq!(unsafe { RimeConfigClose(&mut user_config) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_restores_librime_switcher_saved_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switcher-restore-save-options");
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
switcher:
  save_options: [ascii_mode, full_shape, simplification, traditional]
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - name: full_shape
    states: [半角, 全角]
  - options: [simplification, traditional]
    states: [简体, 繁體]
",
    )
    .expect("schema config should be written");
    fs::write(
        user.join("user.yaml"),
        "\
var:
  option:
    ascii_mode: true
    full_shape: 'true'
    simplification: false
    traditional: true
",
    )
    .expect("user config should be written");

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

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );
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
fn select_schema_switch_translator_normalizes_radio_group_selection() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-radio-default");
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
    - switch_translator
    - echo_translator
switches:
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
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

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
    let candidate_pairs = candidates
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
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("简体".to_owned(), " ✓".to_owned()),
            ("繁體".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    // SAFETY: option names are valid NUL-terminated strings.
    unsafe {
        RimeSetOption(session_id, simplification.as_ptr(), TRUE);
        RimeSetOption(session_id, traditional.as_ptr(), TRUE);
    }
    RimeClearComposition(session_id);
    assert_eq!(RimeProcessKey(session_id, 'y' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_folds_and_unfolds_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-options");
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
    - switch_translator
    - echo_translator
switcher:
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
  abbreviate_options: 'true'
switches:
  - name: ascii_mode
    states: [中文, 西文]
    abbrev: [中, 西]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    abbrev: [简, 繁]
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
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, fold_options.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
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
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中/繁]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, fold_options.as_ptr()) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let composition_input = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_string_lossy()
        .into_owned();
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
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
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(composition_input, "x");
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ 西文".to_owned()),
            ("简体".to_owned(), String::new()),
            ("繁體".to_owned(), " ✓".to_owned()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_honors_librime_fold_options_default() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-options-default");
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
    - switch_translator
    - echo_translator
switcher:
  fold_options: 'true'
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
  abbreviate_options: true
switches:
  - name: ascii_mode
    states: [中文, 西文]
    abbrev: [中, 西]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    abbrev: [简, 繁]
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
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, fold_options.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
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
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中/繁]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_folds_default_radio_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-radio-default");
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
    - switch_translator
    - echo_translator
switcher:
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
switches:
  - name: ascii_mode
    states: [中文, 西文]
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
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    unsafe { RimeSetOption(session_id, fold_options.as_ptr(), TRUE) };
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
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
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中文/简体]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );
    // SAFETY: option names are valid NUL-terminated strings. Librime selects
    // the first radio option while constructing the visible switch menu.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
