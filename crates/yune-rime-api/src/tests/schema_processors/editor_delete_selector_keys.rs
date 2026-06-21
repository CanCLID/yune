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
