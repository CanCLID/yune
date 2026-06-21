#[test]
fn schema_ascii_composer_rejects_direct_ascii_and_edits_inline_ascii() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer");
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
    - ascii_composer
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
translator:
  dictionary: luna
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

你\tni
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
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };

    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), FALSE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, ' ' as c_int, 0), TRUE);

    let mut no_commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut no_commit) }, FALSE);
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 3);
    // SAFETY: `RimeGetContext` populated a valid preedit C string.
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) };
    assert_eq!(preedit.to_str(), Ok("ni "));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_switch_key_handles_eisu_toggle() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-switch-key");
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
    - ascii_composer
  segmentors:
    - abc_segmentor
ascii_composer:
  switch_key:
    Eisu_toggle: set_ascii_mode
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
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);

    let eisu_toggle = CString::new("Eisu_toggle").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let eisu_toggle_keycode = unsafe { RimeGetKeycodeByName(eisu_toggle.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, eisu_toggle_keycode, 0), TRUE);

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_caps_lock_switch_key_clears_composition() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-caps-lock-switch-key");
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
    - ascii_composer
  segmentors:
    - abc_segmentor
ascii_composer:
  switch_key:
    Caps_Lock: clear
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
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let caps_lock = CString::new("Caps_Lock").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let caps_lock_keycode = unsafe { RimeGetKeycodeByName(caps_lock.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, caps_lock_keycode, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, caps_lock_keycode, K_RELEASE_MASK),
        FALSE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_switch_key_handles_shift_release_commit_code() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-shift-switch-key");
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
    - ascii_composer
  segmentors:
    - abc_segmentor
ascii_composer:
  switch_key:
    Shift_L: commit_code
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
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let shift_l = CString::new("Shift_L").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let shift_l_keycode = unsafe { RimeGetKeycodeByName(shift_l.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, shift_l_keycode, 0), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, shift_l_keycode, K_RELEASE_MASK),
        FALSE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` populated a valid C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    // SAFETY: commit text was allocated by `RimeGetCommit`.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_inline_ascii_mode_ends_with_composition() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-inline-ascii");
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
    - ascii_composer
  segmentors:
    - abc_segmentor
ascii_composer:
  switch_key:
    Shift_L: inline_ascii
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
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let shift_l = CString::new("Shift_L").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let shift_l_keycode = unsafe { RimeGetKeycodeByName(shift_l.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, shift_l_keycode, 0), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, shift_l_keycode, K_RELEASE_MASK),
        FALSE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 3);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    RimeClearComposition(session_id);
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_composer_switch_key_falls_back_to_default_commit_text() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-composer-default-switch-key");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
ascii_composer:
  switch_key:
    Shift_R: commit_text
",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - ascii_composer
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
translator:
  dictionary: luna
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

你\tni
尼\tni
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
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let shift_r = CString::new("Shift_R").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated C string.
    let shift_r_keycode = unsafe { RimeGetKeycodeByName(shift_r.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, shift_r_keycode, 0), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, shift_r_keycode, K_RELEASE_MASK),
        FALSE
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` populated a valid C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by `RimeGetCommit`.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_ascii_segmentor_uses_raw_tag_in_ascii_mode() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-ascii-segmentor");
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
  segmentors:
    - ascii_segmentor
    - abc_segmentor
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: luna
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

你\tni
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
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);

    let candidate_texts = || {
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let menu = context.menu;
        let candidates = if menu.num_candidates > 0 {
            // SAFETY: `RimeGetContext` populated `menu.candidates` with
            // `num_candidates` initialized entries.
            unsafe { std::slice::from_raw_parts(menu.candidates, menu.num_candidates as usize) }
                .iter()
                .map(|candidate| {
                    // SAFETY: candidate text pointers are valid C strings
                    // owned by the context until `RimeFreeContext`.
                    unsafe { CStr::from_ptr(candidate.text) }
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        candidates
    };

    assert_eq!(candidate_texts(), ["你", "ni"]);
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(candidate_texts(), ["ni"]);
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
    assert_eq!(candidate_texts(), ["你", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
