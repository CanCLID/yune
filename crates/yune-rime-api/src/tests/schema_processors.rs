use super::*;

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
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
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

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned");
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

#[test]
fn control_delete_key_removes_highlighted_candidate_like_librime_editor_delete_candidate() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let delete = CString::new("Delete").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let delete_keycode = unsafe { RimeGetKeycodeByName(delete.as_ptr()) };
    assert_eq!(delete_keycode, 0xffff);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);
    let control = CString::new("Control").expect("modifier name should be valid");
    // SAFETY: modifier name is a valid NUL-terminated string.
    let control_mask = unsafe { RimeGetModifierByName(control.as_ptr()) };
    assert_eq!(control_mask, 1 << 2);

    let session_id = RimeCreateSession();
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
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, delete_keycode, control_mask),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 3);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: `candidates` points to at least two initialized candidates.
    let second_candidate = unsafe { *context.menu.candidates.add(1) };
    // SAFETY: candidate text is a valid NUL-terminated string owned by context.
    let second_text = unsafe { CStr::from_ptr(second_candidate.text) };
    assert_eq!(second_text.to_str(), Ok("爸"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn shift_delete_key_removes_highlighted_candidate_like_librime_editor_shift_as_control_fallback() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let delete = CString::new("Delete").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let delete_keycode = unsafe { RimeGetKeycodeByName(delete.as_ptr()) };
    assert_eq!(delete_keycode, 0xffff);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);
    let shift = CString::new("Shift").expect("modifier name should be valid");
    // SAFETY: modifier name is a valid NUL-terminated string.
    let shift_mask = unsafe { RimeGetModifierByName(shift.as_ptr()) };
    assert_eq!(shift_mask, 1);

    let session_id = RimeCreateSession();
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
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, delete_keycode, shift_mask), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 3);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: `candidates` points to at least two initialized candidates.
    let second_candidate = unsafe { *context.menu.candidates.add(1) };
    // SAFETY: candidate text is a valid NUL-terminated string owned by context.
    let second_text = unsafe { CStr::from_ptr(second_candidate.text) };
    assert_eq!(second_text.to_str(), Ok("爸"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));
    }
    let sequence = CString::new("ba{Down}{Shift+Delete}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(
        unsafe { RimeGetContext(sequence_session_id, &mut context) },
        TRUE
    );
    assert_eq!(context.menu.num_candidates, 3);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: `candidates` points to at least two initialized candidates.
    let second_candidate = unsafe { *context.menu.candidates.add(1) };
    // SAFETY: candidate text is a valid NUL-terminated string owned by context.
    let second_text = unsafe { CStr::from_ptr(second_candidate.text) };
    assert_eq!(second_text.to_str(), Ok("爸"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn keypad_enter_commits_composition_like_librime_return_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_enter = CString::new("KP_Enter").expect("key name should be valid");
    let shift = CString::new("Shift").expect("modifier name should be valid");
    let kp_enter_keycode = unsafe { RimeGetKeycodeByName(kp_enter.as_ptr()) };
    let shift_mask = unsafe { RimeGetModifierByName(shift.as_ptr()) };
    assert_eq!(kp_enter_keycode, 0xff8d);
    assert_eq!(shift_mask, 1);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_enter_keycode, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{KP_Enter}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let modified_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&modified_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(modified_session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(modified_session_id, 'i' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(modified_session_id, kp_enter_keycode, shift_mask),
        FALSE
    );
    let modified_sequence =
        CString::new("{Control+KP_Enter}{Shift+KP_Enter}{Control+Shift+KP_Enter}")
            .expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(modified_session_id, modified_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and no unread commit is expected.
    assert_eq!(
        unsafe { RimeGetCommit(modified_session_id, &mut commit) },
        FALSE
    );
    assert_eq!(RimeGetCaretPos(modified_session_id), 2);
    let input = RimeGetInput(modified_session_id);
    assert!(!input.is_null());
    // SAFETY: RimeGetInput returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    let unmodified_sequence = CString::new("{KP_Enter}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(modified_session_id, unmodified_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(modified_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(modified_session_id), TRUE);
}

#[test]
fn keypad_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(RimeProcessKey(session_id, kp_2_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_2_keycode, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("{KP_2}ba{KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_keypad_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(
        RimeProcessKey(session_id, kp_2_keycode, K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_2_keycode, K_SHIFT_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("{Shift+KP_2}ba{Shift+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_ascii_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, K_SHIFT_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Shift+2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_ascii_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(
        RimeProcessKey(session_id, '2' as i32, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, K_CONTROL_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Control+2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_keypad_digits_select_candidates_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(
        RimeProcessKey(session_id, kp_2_keycode, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, kp_2_keycode, K_CONTROL_MASK),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Control+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_shift_digits_select_candidates_like_librime_selector_fallback() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let control_shift = K_CONTROL_MASK | K_SHIFT_MASK;
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(RimeProcessKey(session_id, '2' as i32, control_shift), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, kp_2_keycode, control_shift),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, control_shift), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let keypad_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&keypad_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    assert_eq!(RimeProcessKey(keypad_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(keypad_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(keypad_session_id, kp_2_keycode, control_shift),
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(keypad_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(keypad_session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Control+Shift+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage and was cleared above.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn escape_clears_composition_like_librime_editor_cancel_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let escape = CString::new("Escape").expect("key name should be valid");
    let escape_keycode = unsafe { RimeGetKeycodeByName(escape.as_ptr()) };
    assert_eq!(escape_keycode, 0xff1b);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(session_id, escape_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, escape_keycode, 0), TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    let mut context = empty_context();
    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert!(context.composition.preedit.is_null());
    assert_eq!(context.menu.num_candidates, 0);
    assert!(context.menu.candidates.is_null());
    // SAFETY: nested pointers are null after the empty context response.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{Escape}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let sequence_input = RimeGetInput(sequence_session_id);
    assert!(!sequence_input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(sequence_input) }.to_str(), Ok(""));
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        FALSE
    );
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_escape_clears_composition_like_librime_editor_cancel_fallback() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let escape = CString::new("Escape").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let escape_keycode = unsafe { RimeGetKeycodeByName(escape.as_ptr()) };
    assert_eq!(escape_keycode, 0xff1b);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, escape_keycode, K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, escape_keycode, K_SHIFT_MASK),
        TRUE
    );

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence = CString::new("ni{Shift+Escape}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let sequence_input = RimeGetInput(sequence_session_id);
    assert!(!sequence_input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(sequence_input) }.to_str(), Ok(""));
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn backspace_key_removes_input_before_caret_like_librime_editor_back() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let backspace = CString::new("BackSpace").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let backspace_keycode = unsafe { RimeGetKeycodeByName(backspace.as_ptr()) };
    assert_eq!(backspace_keycode, 0xff08);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nxi").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, backspace_keycode, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 1);
    RimeSetCaretPos(session_id, 0);
    assert_eq!(RimeProcessKey(session_id, backspace_keycode, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("nxi{Left}{BackSpace}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_backspace_key_removes_previous_input_like_librime_editor_back_syllable() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let backspace = CString::new("BackSpace").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let backspace_keycode = unsafe { RimeGetKeycodeByName(backspace.as_ptr()) };
    assert_eq!(backspace_keycode, 0xff08);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nxi").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    RimeSetCaretPos(session_id, 2);
    assert_eq!(
        RimeProcessKey(session_id, backspace_keycode, K_CONTROL_MASK),
        TRUE
    );
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 1);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence =
        CString::new("nxi{Left}{Control+BackSpace}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_backspace_key_uses_librime_editor_shift_as_control_fallback() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let backspace = CString::new("BackSpace").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let backspace_keycode = unsafe { RimeGetKeycodeByName(backspace.as_ptr()) };
    assert_eq!(backspace_keycode, 0xff08);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nxi").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    RimeSetCaretPos(session_id, 2);
    assert_eq!(
        RimeProcessKey(session_id, backspace_keycode, K_SHIFT_MASK),
        TRUE
    );
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 1);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence =
        CString::new("nxi{Left}{Shift+BackSpace}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_return_key_commits_raw_input_like_librime_fluid_editor() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, K_CONTROL_MASK),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{Control+Return}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_return_key_commits_script_text_like_librime_fluid_editor() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, K_SHIFT_MASK),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{Shift+Return}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_printable_keys_enter_input_and_shift_space_confirms_like_librime_editor() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let session_id = RimeCreateSession();
    assert_eq!(RimeProcessKey(session_id, 'A' as i32, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("Ab"));
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, K_SHIFT_MASK), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("Ab"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, K_SHIFT_MASK), FALSE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence = CString::new("{Shift+A}b{Shift+space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("Ab"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_shift_return_key_commits_selected_comment_like_librime_fluid_editor() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let return_key = CString::new("Return").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let return_keycode = unsafe { RimeGetKeycodeByName(return_key.as_ptr()) };
    assert_eq!(return_keycode, 0xff0d);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);
    let modifier_mask = K_CONTROL_MASK | K_SHIFT_MASK;

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.add_translator(CommentTranslator);
    }
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, modifier_mask),
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(
        unsafe { CStr::from_ptr(commit.text) }.to_str(),
        Ok("second-comment")
    );
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(
        RimeProcessKey(session_id, return_keycode, modifier_mask),
        FALSE
    );
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session.engine.add_translator(CommentTranslator);
    }
    let sequence =
        CString::new("ni{Down}{Control+Shift+Return}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(
        unsafe { CStr::from_ptr(commit.text) }.to_str(),
        Ok("second-comment")
    );
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn delete_key_removes_input_at_caret_like_librime_editor_delete_key() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let delete = CString::new("Delete").expect("key name should be valid");
    let delete_keycode = unsafe { RimeGetKeycodeByName(delete.as_ptr()) };
    assert_eq!(delete_keycode, 0xffff);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, delete_keycode, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ni"));
    assert_eq!(RimeGetCaretPos(session_id), 2);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeSetInput(sequence_session_id, input.as_ptr()) },
        TRUE
    );
    RimeSetCaretPos(sequence_session_id, 2);
    let sequence = CString::new("{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated a valid C string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn left_right_keys_move_caret_like_librime_navigator_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(left_keycode, 0xff51);
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };
    assert_eq!(right_keycode, 0xff53);

    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, left_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 2);
    RimeSetCaretPos(session_id, 1);
    assert_eq!(RimeProcessKey(session_id, right_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 2);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("nix{Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_left_right_keys_jump_syllable_span_like_librime_navigator_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(left_keycode, 0xff51);
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };
    assert_eq!(right_keycode, 0xff53);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence =
        CString::new("nix{Control+Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ix"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_left_right_keys_fall_back_to_control_syllable_jump_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(left_keycode, 0xff51);
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };
    assert_eq!(right_keycode, 0xff53);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_SHIFT_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, left_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence =
        CString::new("nix{Shift+Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ix"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn control_up_down_keys_jump_syllable_span_like_librime_vertical_navigator_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let up = CString::new("Up").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let up_keycode = unsafe { RimeGetKeycodeByName(up.as_ptr()) };
    assert_eq!(up_keycode, 0xff52);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, up_keycode, K_CONTROL_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, up_keycode, K_CONTROL_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(
        RimeProcessKey(session_id, down_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence =
        CString::new("nix{Control+Up}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ix"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn linear_selector_arrow_keys_follow_librime_layout_bindings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let up = CString::new("Up").expect("key name should be valid");
    let down = CString::new("Down").expect("key name should be valid");
    let left = CString::new("Left").expect("key name should be valid");
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key names are valid NUL-terminated strings.
    let up_keycode = unsafe { RimeGetKeycodeByName(up.as_ptr()) };
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };

    let session_id = RimeCreateSession();
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
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let linear = CString::new("_linear").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, linear.as_ptr(), TRUE) };

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 0);

    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 5);
    assert_eq!(RimeProcessKey(session_id, up_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 0);

    assert_eq!(RimeProcessKey(session_id, right_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 1);

    RimeSetCaretPos(session_id, 1);
    assert_eq!(RimeProcessKey(session_id, left_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 1);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let vertical_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&vertical_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let vertical = CString::new("_vertical").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(vertical_session_id, vertical.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(vertical_session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(vertical_session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(vertical_session_id, left_keycode, 0), TRUE);
    assert_eq!(current_highlighted(vertical_session_id), 1);
    assert_eq!(RimeProcessKey(vertical_session_id, right_keycode, 0), TRUE);
    assert_eq!(current_highlighted(vertical_session_id), 0);
    assert_eq!(RimeDestroySession(vertical_session_id), TRUE);

    let vertical_linear_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&vertical_linear_session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    // SAFETY: option names are valid NUL-terminated strings.
    unsafe {
        RimeSetOption(vertical_linear_session_id, vertical.as_ptr(), TRUE);
        RimeSetOption(vertical_linear_session_id, linear.as_ptr(), TRUE);
    }
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, 'b' as c_int, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, 'a' as c_int, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, left_keycode, 0),
        TRUE
    );
    assert_eq!(current_highlighted(vertical_linear_session_id), 5);
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, right_keycode, 0),
        TRUE
    );
    assert_eq!(current_highlighted(vertical_linear_session_id), 0);
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, down_keycode, 0),
        TRUE
    );
    assert_eq!(current_highlighted(vertical_linear_session_id), 1);
    assert_eq!(
        RimeProcessKey(vertical_linear_session_id, up_keycode, 0),
        TRUE
    );
    assert_eq!(current_highlighted(vertical_linear_session_id), 0);
    assert_eq!(RimeDestroySession(vertical_linear_session_id), TRUE);
}

#[test]
fn schema_selector_bindings_override_default_layout_keymap() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-selector-bindings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\nselector:\n  bindings:\n    Control+j: next_candidate\n    Down: noop\n  linear:\n    bindings:\n      Control+k: previous_page\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
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
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(current_highlighted(session_id), 0);

    assert_eq!(
        RimeProcessKey(session_id, 'j' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(current_highlighted(session_id), 1);
    assert_eq!(
        RimeProcessKey(session_id, 'j' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(current_highlighted(session_id), 2);

    let linear = CString::new("_linear").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, linear.as_ptr(), TRUE) };
    assert_eq!(
        RimeProcessKey(session_id, 'k' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(current_highlighted(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_navigator_bindings_override_default_keymap() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-navigator-bindings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nnavigator:\n  bindings:\n    Control+h: left_by_char\n    Control+l: right_by_char_no_loop\n    Left: noop\n  vertical:\n    bindings:\n      Control+j: end\n      Control+k: home\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(left_keycode, 0xff51);

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let input = CString::new("abc").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(
        RimeProcessKey(session_id, 'h' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 1);
    assert_eq!(RimeProcessKey(session_id, left_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 1);
    assert_eq!(
        RimeProcessKey(session_id, 'l' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);

    let vertical = CString::new("_vertical").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, vertical.as_ptr(), TRUE) };
    assert_eq!(
        RimeProcessKey(session_id, 'j' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(
        RimeProcessKey(session_id, 'k' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_navigator_syllable_jump_position_honors_delimiters() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-navigator-delimiter-jump");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("after.schema.yaml"),
        "\
schema:\n  schema_id: after\n  name: After\nspeller:\n  delimiter: \"'\"\n",
    )
    .expect("after schema config should be written");
    fs::write(
        staging.join("before.schema.yaml"),
        "\
schema:\n  schema_id: before\n  name: Before\nspeller:\n  delimiter: \"'\"\nnavigator:\n  syllable_jump_position: before_delimiter\n  bindings:\n    Control+h: left_by_syllable_no_loop\n    Control+l: right_by_syllable_no_loop\n",
    )
    .expect("before schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let left = CString::new("Left").expect("key name should be valid");
    let right = CString::new("Right").expect("key name should be valid");
    // SAFETY: key names are valid NUL-terminated strings.
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    let right_keycode = unsafe { RimeGetKeycodeByName(right.as_ptr()) };
    let input = CString::new("ab'cd").expect("input should be valid");

    let session_id = RimeCreateSession();
    let after_schema = CString::new("after").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, after_schema.as_ptr()) },
        TRUE
    );
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    RimeSetCaretPos(session_id, 5);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);

    let before_schema = CString::new("before").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, before_schema.as_ptr()) },
        TRUE
    );
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);

    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, left_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    RimeSetCaretPos(session_id, 5);
    assert_eq!(
        RimeProcessKey(session_id, right_keycode, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, 'h' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    RimeSetCaretPos(session_id, 5);
    assert_eq!(
        RimeProcessKey(session_id, 'l' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 5);
    RimeSetCaretPos(session_id, 4);
    assert_eq!(
        RimeProcessKey(session_id, 'h' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    assert_eq!(
        RimeProcessKey(session_id, 'l' as c_int, K_CONTROL_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 5);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn shift_up_down_keys_fall_back_to_control_syllable_jump_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let up = CString::new("Up").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let up_keycode = unsafe { RimeGetKeycodeByName(up.as_ptr()) };
    assert_eq!(up_keycode, 0xff52);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);

    let session_id = RimeCreateSession();
    assert_eq!(RimeProcessKey(session_id, up_keycode, K_SHIFT_MASK), FALSE);
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 2);
    assert_eq!(RimeProcessKey(session_id, up_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeProcessKey(session_id, down_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence = CString::new("nix{Shift+Up}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ix"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn keypad_left_right_keys_move_caret_by_char_with_librime_navigator_looping() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_left = CString::new("KP_Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_left_keycode = unsafe { RimeGetKeycodeByName(kp_left.as_ptr()) };
    assert_eq!(kp_left_keycode, 0xff96);
    let kp_right = CString::new("KP_Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_right_keycode = unsafe { RimeGetKeycodeByName(kp_right.as_ptr()) };
    assert_eq!(kp_right_keycode, 0xff98);

    let session_id = RimeCreateSession();
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 0);
    assert_eq!(RimeProcessKey(session_id, kp_left_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, kp_right_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("nix{KP_Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_keypad_left_right_keys_ignore_shift_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_left = CString::new("KP_Left").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_left_keycode = unsafe { RimeGetKeycodeByName(kp_left.as_ptr()) };
    assert_eq!(kp_left_keycode, 0xff96);
    let kp_right = CString::new("KP_Right").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_right_keycode = unsafe { RimeGetKeycodeByName(kp_right.as_ptr()) };
    assert_eq!(kp_right_keycode, 0xff98);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, kp_left_keycode, K_SHIFT_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, kp_left_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(
        RimeProcessKey(session_id, kp_right_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence =
        CString::new("nix{Shift+KP_Left}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_keypad_up_down_keys_ignore_shift_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_up = CString::new("KP_Up").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_up_keycode = unsafe { RimeGetKeycodeByName(kp_up.as_ptr()) };
    assert_eq!(kp_up_keycode, 0xff97);
    let kp_down = CString::new("KP_Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_down_keycode = unsafe { RimeGetKeycodeByName(kp_down.as_ptr()) };
    assert_eq!(kp_down_keycode, 0xff99);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, kp_up_keycode, K_SHIFT_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    RimeSetCaretPos(session_id, 0);
    assert_eq!(
        RimeProcessKey(session_id, kp_up_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(
        RimeProcessKey(session_id, kp_down_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence =
        CString::new("nix{Shift+KP_Up}{Delete}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("你"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn page_keys_move_candidate_page_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("page-key-selector");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n",
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
    let page_down_keycode = unsafe { RimeGetKeycodeByName(page_down.as_ptr()) };
    assert_eq!(page_down_keycode, 0xff56);
    let kp_page_up = CString::new("KP_Page_Up").expect("key name should be valid");
    let kp_page_up_keycode = unsafe { RimeGetKeycodeByName(kp_page_up.as_ptr()) };
    assert_eq!(kp_page_up_keycode, 0xff9a);

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
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

    assert_eq!(RimeProcessKey(session_id, page_down_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, page_down_keycode, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_size, 2);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: context.menu.candidates points to at least one candidate.
    let first_candidate = unsafe { *context.menu.candidates };
    // SAFETY: candidate text is owned by the returned context and is valid until free.
    assert_eq!(
        unsafe { CStr::from_ptr(first_candidate.text) }.to_str(),
        Ok("爸")
    );
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, kp_page_up_keycode, 0), TRUE);
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let sequence = CString::new("ba{Next}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(
        unsafe { RimeGetContext(sequence_session_id, &mut context) },
        TRUE
    );
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn up_down_keys_move_candidate_highlight_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(down_keycode, 0xff54);
    let kp_up = CString::new("KP_Up").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_up_keycode = unsafe { RimeGetKeycodeByName(kp_up.as_ptr()) };
    assert_eq!(kp_up_keycode, 0xff97);

    let session_id = RimeCreateSession();
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
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, kp_up_keycode, 0), TRUE);
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Down}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn home_end_keys_reset_candidate_highlight_like_librime_selector_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let home = CString::new("Home").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let home_keycode = unsafe { RimeGetKeycodeByName(home.as_ptr()) };
    assert_eq!(home_keycode, 0xff50);
    let kp_end = CString::new("KP_End").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_end_keycode = unsafe { RimeGetKeycodeByName(kp_end.as_ptr()) };
    assert_eq!(kp_end_keycode, 0xff9c);

    let session_id = RimeCreateSession();
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
        ]));
    }

    assert_eq!(RimeProcessKey(session_id, home_keycode, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);

    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, home_keycode, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_end_keycode, 0), TRUE);
    // SAFETY: context points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: nested pointers were allocated by RimeGetContext above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Down}{KP_End}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("八"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn home_end_keys_fall_back_to_librime_navigator_caret_movement() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let home = CString::new("Home").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let home_keycode = unsafe { RimeGetKeycodeByName(home.as_ptr()) };
    assert_eq!(home_keycode, 0xff50);
    let end = CString::new("End").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let end_keycode = unsafe { RimeGetKeycodeByName(end.as_ptr()) };
    assert_eq!(end_keycode, 0xff57);

    let session_id = RimeCreateSession();
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'x' as i32, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, home_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeProcessKey(session_id, end_keycode, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence =
        CString::new("nix{Home}{Delete}{End}{BackSpace}{space}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("i"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

#[test]
fn shift_home_end_keys_ignore_shift_like_librime_navigator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let home = CString::new("Home").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let home_keycode = unsafe { RimeGetKeycodeByName(home.as_ptr()) };
    assert_eq!(home_keycode, 0xff50);
    let end = CString::new("End").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let end_keycode = unsafe { RimeGetKeycodeByName(end.as_ptr()) };
    assert_eq!(end_keycode, 0xff57);
    let kp_end = CString::new("KP_End").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_end_keycode = unsafe { RimeGetKeycodeByName(kp_end.as_ptr()) };
    assert_eq!(kp_end_keycode, 0xff9c);

    let session_id = RimeCreateSession();
    assert_eq!(
        RimeProcessKey(session_id, home_keycode, K_SHIFT_MASK),
        FALSE
    );
    let input = CString::new("nix").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated C string.
    assert_eq!(unsafe { RimeSetInput(session_id, input.as_ptr()) }, TRUE);

    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, home_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(RimeProcessKey(session_id, end_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeProcessKey(session_id, home_keycode, K_SHIFT_MASK), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 0);
    assert_eq!(
        RimeProcessKey(session_id, kp_end_keycode, K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 3);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    let sequence = CString::new("nix{Shift+Home}{Delete}{Shift+KP_End}{BackSpace}{space}")
        .expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: successful commit text is a valid NUL-terminated string.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("i"));
    // SAFETY: commit text was allocated by RimeGetCommit.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);
}

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

#[test]
fn schema_recognizer_processor_accepts_space_for_librime_patterns() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-recognizer-processor-space");
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
    - recognizer@processor
  segmentors:
    - abc_segmentor
    - matcher
  translators:
    - reverse_lookup_translator
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
recognizer:
  use_space: 'true'
  patterns:
    reverse_lookup: \"`[a-z ]*$\"
processor:
  use_space: false
  patterns:
    reverse_lookup: \"^never$\"
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火山\thuo shan
",
    )
    .expect("reverse lookup dictionary should be written");

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
    for ch in "`huo".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    assert_eq!(RimeProcessKey(session_id, ' ' as c_int, 0), TRUE);
    let input = unsafe { CStr::from_ptr(RimeGetInput(session_id)) }
        .to_str()
        .expect("input should be valid UTF-8");
    assert_eq!(input, "`huo ");
    let mut no_commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as c_int,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut no_commit) }, FALSE);

    for ch in "shan".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

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
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(texts, ["火山".to_owned(), "`huo shan".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: processors/speller.rs; librime oracle: Speller::AutoSelectPreviousMatch non-auto ConfirmCurrentSelection + FindEarlierMatch.
#[test]
fn schema_speller_previous_match_non_auto_confirm_matches_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-speller-previous-match-non-auto");
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
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
speller:
  alphabet: abc
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
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

甲\ta\t100
乙\tb\t100
丙\tc\t100
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

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);

    let mut no_commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut no_commit) }, FALSE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("b"));

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 1);
    assert_eq!(context.composition.sel_start, 0);
    assert_eq!(context.composition.sel_end, 1);
    assert_eq!(context.menu.num_candidates, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    // SAFETY: context composition and candidate pointers are populated by `RimeGetContext`.
    assert_eq!(unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(), Ok("b"));
    let candidate = unsafe { *context.menu.candidates };
    assert_eq!(unsafe { CStr::from_ptr(candidate.text) }.to_str(), Ok("乙"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'c' as i32, 0), TRUE);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("c"));

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: processors/editor.rs, processors/navigator.rs, processors/selector.rs; librime oracle: schema-loaded composition span and candidate selection state.
#[test]
fn schema_editor_navigator_selector_spans_match_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-editor-navigator-selector-spans");
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
    - speller
    - selector
    - navigator
    - fluid_editor
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
speller:
  alphabet: abc
  delimiter: ' '
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
selector:
  bindings:
    Down: next_candidate
    Up: previous_candidate
navigator:
  bindings:
    Left: left_by_syllable
    Right: right_by_syllable
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

甲\ta\t100
乙\ta\t90
丙\ta\t80
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

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    let down = CString::new("Down").expect("key name should be valid");
    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key names are valid NUL-terminated strings.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, left_keycode, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 1);
    assert_eq!(context.composition.cursor_pos, 0);
    assert_eq!(context.composition.sel_start, 0);
    assert_eq!(context.composition.sel_end, 1);
    assert_eq!(context.menu.num_candidates, 3);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(unsafe { CStr::from_ptr(candidates[1].text) }.to_str(), Ok("乙"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    let mut no_commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut no_commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: processors/chord_composer.rs, processors/shape.rs, processors/punctuation.rs, schema_install.rs; librime oracle: chain punctuation/fallback ordering and cleanup state.
#[test]
fn schema_chord_shape_punctuation_fallback_chain_matches_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-shape-punctuation-fallback-chain");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chain.schema.yaml"),
        "\
schema:
  schema_id: chain
  name: Chain
engine:
  processors:
    - chord_composer
    - punctuator
  segmentors:
    - punct_segmentor
    - fallback_segmentor
  translators:
    - punct_translator
    - table_translator
    - echo_translator
chord_composer:
  alphabet: ab
  output_format:
    - xlit/ab/xy/
  bindings:
    Control+r: commit_raw_input
translator:
  dictionary: chain
  enable_completion: false
  enable_sentence: false
punctuator:
  half_shape:
    '.': '。'
  full_shape:
    '.': '．'
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chain.dict.yaml"),
        "\
---
name: chain
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
    let schema_id = CString::new("chain").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 2);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, ["。".to_owned(), ".".to_owned()]);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    RimeClearComposition(session_id);

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(RimeCommitComposition(session_id), TRUE);
    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("形"));
    // SAFETY: commit text was allocated by `RimeGetCommit`.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK), FALSE);

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, 'x' as i32, 0), TRUE);
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("ｘ"));
    // SAFETY: commit text was allocated by `RimeGetCommit`.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, FALSE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_candidates_expose_librime_shape_comments() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-comments");
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
    - punct_translator
    - echo_translator
punctuator:
  half_shape:
    \"/\": [\"/\", \"、\", \"©\"]
  full_shape:
    \"/\": \"／\"
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

    let candidate_comments = || {
        assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
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
        let comments = candidates
            .iter()
            .map(|candidate| {
                if candidate.comment.is_null() {
                    None
                } else {
                    Some(
                        // SAFETY: non-null candidate comment pointers are
                        // populated by `RimeGetContext`.
                        unsafe { CStr::from_ptr(candidate.comment) }
                            .to_str()
                            .expect("candidate comment should be valid UTF-8")
                            .to_owned(),
                    )
                }
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        RimeClearComposition(session_id);
        comments
    };

    assert_eq!(
        candidate_comments(),
        [
            Some("〔半角〕".to_owned()),
            Some("〔全角〕".to_owned()),
            None,
            Some("echo".to_owned())
        ]
    );

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    assert_eq!(
        candidate_comments(),
        [Some("〔全角〕".to_owned()), Some("echo".to_owned())]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punct_segmentor_tags_punctuation_exclusively() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punct-segmentor-exclusive");
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
    - abc_segmentor
    - punct_segmentor
  translators:
    - punct_translator
    - table_translator
    - echo_translator
translator:
  dictionary: luna
punctuator:
  half_shape:
    \".\": \"。\"
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

DOT\t.\t100
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

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
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
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["。".to_owned(), ".".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punct_segmentor_translates_digit_separator_as_number_punctuation() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punct-segmentor-number");
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
    - punct_segmentor
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

    let top_candidate = || {
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
        let text = unsafe { CStr::from_ptr(candidates[0].text) }
            .to_str()
            .expect("candidate text should be valid UTF-8")
            .to_owned();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        text
    };

    assert_eq!(RimeProcessKey(session_id, '1' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(top_candidate(), ".");

    RimeClearComposition(session_id);
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(top_candidate(), "．");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_fallback_segmentor_tags_unclaimed_input_as_raw() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-fallback-segmentor-raw");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("raw.schema.yaml"),
        "\
schema:
  schema_id: raw
  name: Raw
engine:
  segmentors:
    - fallback_segmentor
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: raw
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("raw.dict.yaml"),
        "\
---
name: raw
version: '0.1'
sort: original
...

Alpha\ta\t100
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
    let schema_id = CString::new("raw").expect("schema id should be valid");
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
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["a".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

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

#[test]
fn schema_punctuator_processor_commits_unique_punctuation() {
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
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("。"));
    // SAFETY: commit.text was returned by RimeGetCommit above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

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
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: RimeGetCommit populated a valid NUL-terminated C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("。"));
    // SAFETY: commit.text was returned by RimeGetCommit above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

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
            "/".to_owned(),
            vec!["A".to_owned(), "B".to_owned(), "/".to_owned()],
            0
        )
    );

    assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
    assert_eq!(
        context_state(),
        (
            "/".to_owned(),
            vec!["A".to_owned(), "B".to_owned(), "/".to_owned()],
            1
        )
    );

    assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
    assert_eq!(
        context_state(),
        (
            "/".to_owned(),
            vec!["A".to_owned(), "B".to_owned(), "/".to_owned()],
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
fn schema_punctuator_processor_commits_paired_punctuation_alternately() {
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

    let committed_pair = || {
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        assert_eq!(RimeProcessKey(session_id, '(' as i32, 0), TRUE);
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

    assert_eq!(committed_pair(), "（");
    assert_eq!(committed_pair(), "）");
    assert_eq!(committed_pair(), "（");

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };

    assert_eq!(committed_pair(), "〔");
    assert_eq!(committed_pair(), "〕");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
