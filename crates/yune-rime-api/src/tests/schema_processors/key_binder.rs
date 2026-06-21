#[test]
fn schema_key_binder_processor_toggles_options_from_bindings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-toggle");
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
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, toggle: ascii_mode }
    - { when: composing, accept: Control+Shift+2, toggle: full_shape }
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
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_prefers_later_same_condition_binding() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-same-condition-order");
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
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, toggle: ascii_mode }
    - { when: always, accept: Control+Shift+1, toggle: full_shape }
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
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );
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
fn schema_key_binder_processor_sets_and_unsets_switch_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-set-option");
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
    - key_binder
  translators:
    - echo_translator
switches:
  - name: ascii_mode
  - options: [simplification, traditional]
    reset: 0
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, set_option: ascii_mode }
    - { when: always, accept: Control+Shift+2, unset_option: ascii_mode }
    - { when: always, accept: Control+Shift+3, set_option: traditional }
    - { when: always, accept: Control+Shift+4, unset_option: traditional }
    - { when: always, accept: Control+Shift+5, toggle: simplification }
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
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(
        RimeProcessKey(session_id, '3' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '4' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(
        RimeProcessKey(session_id, '5' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
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
fn schema_key_binder_processor_toggles_switches_by_index() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-toggle-index");
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
    - key_binder
  translators:
    - echo_translator
switches:
  - name: ascii_mode
  - options: [simplification, traditional]
    reset: 0
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, toggle: '@0' }
    - { when: always, accept: Control+Shift+2, toggle: '@1' }
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
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
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
fn schema_key_binder_processor_redirects_send_bindings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-send");
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
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: '/', send_sequence: 'xy' }
    - { when: composing, accept: ';', send: BackSpace }
    - { when: always, accept: ',', send: ',' }
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

    let preedit = |session_id| {
        let mut context = empty_context();
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
        let text = unsafe { CStr::from_ptr(context.composition.preedit) }
            .to_str()
            .expect("preedit should be valid UTF-8")
            .to_owned();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        unsafe { RimeFreeContext(&mut context) };
        text
    };

    assert_eq!(RimeProcessKey(session_id, '/' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "xy");

    assert_eq!(RimeProcessKey(session_id, ';' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "x");

    assert_eq!(RimeProcessKey(session_id, ',' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "x,");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_loads_namespaced_prescription() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-namespaced-key-binder");
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
    - key_binder@custom_processor
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: '/', send_sequence: 'xy' }
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

    assert_eq!(RimeProcessKey(session_id, '/' as c_int, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage for the populated context.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: `RimeGetContext` populated a valid preedit C string.
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_str()
        .expect("preedit should be valid UTF-8");
    assert_eq!(preedit, "xy");
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    unsafe { RimeFreeContext(&mut context) };

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_matches_paging_condition_after_page_navigation() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-paging");
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
menu:
  page_size: 2
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: has_menu, accept: ',', toggle: full_shape }
    - { when: paging, accept: ',', toggle: ascii_mode }
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

    let page_down = CString::new("Page_Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let page_down_keycode = unsafe { RimeGetKeycodeByName(page_down.as_ptr()) };
    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ',' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeProcessKey(session_id, page_down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ',' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_reinterprets_period_paging_before_letters() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-reinterpret-period");
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
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: has_menu, accept: period, send: Page_Down }
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

    let preedit = |session_id| {
        let mut context = empty_context();
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
        let text = unsafe { CStr::from_ptr(context.composition.preedit) }
            .to_str()
            .expect("preedit should be valid UTF-8")
            .to_owned();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        unsafe { RimeFreeContext(&mut context) };
        text
    };

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "ba");
    assert_eq!(RimeProcessKey(session_id, 'c' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "ba.c");

    assert_eq!(RimeProcessKey(session_id, 0xff1b, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'c' as c_int, 0), TRUE);
    assert_eq!(preedit(session_id), "bac");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_key_binder_processor_selects_schemas_from_bindings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    RimeSetNotificationHandler(None, std::ptr::null_mut());
    let root = unique_temp_dir("schema-key-binder-select");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
schema_list:
  - schema: alpha
  - schema: beta
  - schema: gamma
",
    )
    .expect("default config should be written");
    fs::write(
        staging.join("alpha.schema.yaml"),
        "\
schema:
  schema_id: alpha
  name: Alpha
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, select: beta }
",
    )
    .expect("alpha schema config should be written");
    fs::write(
        staging.join("beta.schema.yaml"),
        "\
schema:
  schema_id: beta
  name: Beta
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+2, select: .next }
",
    )
    .expect("beta schema config should be written");
    fs::write(
        staging.join("gamma.schema.yaml"),
        "schema:\n  schema_id: gamma\n  name: Gamma\n",
    )
    .expect("gamma schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let alpha = CString::new("alpha").expect("schema id should be valid");
    let context_object = 0x51_usize as *mut c_void;
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, alpha.as_ptr()) },
        TRUE
    );
    notification_events_lock().clear();
    RimeSetNotificationHandler(Some(record_notification), context_object);

    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: status strings were allocated by RimeGetStatus above.
    assert_eq!(
        unsafe { CStr::from_ptr(status.schema_id) }.to_str(),
        Ok("beta")
    );
    // SAFETY: nested status allocations were returned by RimeGetStatus above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: status strings were allocated by RimeGetStatus above.
    assert_eq!(
        unsafe { CStr::from_ptr(status.schema_id) }.to_str(),
        Ok("alpha")
    );
    // SAFETY: nested status allocations were returned by RimeGetStatus above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    let events = notification_events_lock();
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x51,
                session_id,
                message_type: "schema".to_owned(),
                message_value: "beta/Beta".to_owned(),
            },
            NotificationEvent {
                context_object: 0x51,
                session_id,
                message_type: "schema".to_owned(),
                message_value: "alpha/Alpha".to_owned(),
            },
        ]
    );
    drop(events);

    RimeSetNotificationHandler(None, std::ptr::null_mut());
    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
