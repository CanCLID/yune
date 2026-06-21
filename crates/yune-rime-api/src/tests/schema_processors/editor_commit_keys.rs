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
