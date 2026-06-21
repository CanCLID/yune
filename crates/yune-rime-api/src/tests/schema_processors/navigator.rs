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
