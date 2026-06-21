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
