use super::*;

#[test]
fn select_candidate_apis_commit_current_candidates() {
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
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    assert_eq!(RimeSelectCandidate(session_id, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 1), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeSelectCandidateOnCurrentPage(session_id, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("八"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn highlight_candidate_apis_move_selection_without_commit() {
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
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    assert_eq!(RimeHighlightCandidate(session_id, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 1), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 99), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 99), FALSE);
    assert_eq!(RimeHighlightCandidate(session_id, 1), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeChangePage(session_id, FALSE), TRUE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 0), TRUE);
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 4), TRUE);
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 4), FALSE);
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 5), FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeChangePage(session_id, TRUE), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("八"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn current_page_candidate_apis_use_selected_schema_page_size_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("current-page-candidate-apis");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: '2'\n",
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
    let mut context = empty_context();
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 3), TRUE);
    assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 0), TRUE);
    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_size, 2);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: `context.menu.candidates` points to `num_candidates` entries.
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    // SAFETY: candidate text pointers are valid strings owned by the context.
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("爸")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeChangePage(session_id, FALSE), TRUE);
    assert_eq!(RimeDeleteCandidateOnCurrentPage(session_id, 0), TRUE);
    assert_eq!(RimeSelectCandidateOnCurrentPage(session_id, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("拔"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn alternative_select_keys_select_current_page_candidates_like_librime_selector() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("alternative-select-keys");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    let session_id = RimeCreateSession();
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
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
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
    let sequence = CString::new("baB").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let shifted_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&shifted_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(shifted_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(shifted_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(shifted_session_id, 'B' as i32, K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_session_id), TRUE);

    let shifted_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&shifted_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Shift+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(shifted_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_sequence_session_id), TRUE);

    let controlled_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&controlled_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(controlled_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(controlled_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(controlled_session_id, 'B' as i32, K_CONTROL_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(controlled_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(controlled_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_session_id), TRUE);

    let controlled_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&controlled_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Control+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(controlled_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_sequence_session_id), TRUE);

    let alt_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(alt_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&alt_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(alt_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(alt_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(alt_session_id, 'B' as i32, K_ALT_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(alt_session_id, &mut commit) }, FALSE);
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(alt_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(alt_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(alt_session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(alt_session_id), TRUE);

    let alt_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(alt_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&alt_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Alt+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(alt_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(alt_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(alt_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(alt_sequence_session_id), TRUE);

    let super_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(super_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&super_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(super_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(super_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(super_session_id, 'B' as i32, K_SUPER_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(super_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(super_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(super_session_id), TRUE);

    let super_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(super_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&super_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Super+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(super_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(super_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(super_sequence_session_id), TRUE);

    let released_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(released_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&released_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }

    assert_eq!(RimeProcessKey(released_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(released_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(released_session_id, 'B' as i32, K_RELEASE_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(released_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(released_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(released_session_id), TRUE);

    let released_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(released_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&released_sequence_session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    }
    let sequence = CString::new("ba{Release+B}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(released_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(released_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(released_sequence_session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn alternative_select_keys_suppress_unlisted_ascii_digit_fallback_like_librime_selector() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("alternative-select-keys-unlisted-digit");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let add_ba_translator = |session_id| {
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
    };

    let session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(session_id);

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let shifted_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_session_id);
    assert_eq!(RimeProcessKey(shifted_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(shifted_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(shifted_session_id, '2' as i32, K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(shifted_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeProcessKey(shifted_session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(sequence_session_id);
    let sequence = CString::new("ba2B").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(printable_session_id);
    assert_eq!(RimeProcessKey(printable_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(printable_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(printable_session_id, 'x' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("bax")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(printable_session_id), TRUE);

    let printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(printable_sequence_session_id);
    let sequence = CString::new("bax").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(printable_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("bax")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(printable_sequence_session_id), TRUE);

    let shifted_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_printable_session_id);
    assert_eq!(
        RimeProcessKey(shifted_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(shifted_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(shifted_printable_session_id, 'x' as i32, K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(shifted_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("bax")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_printable_session_id), TRUE);

    let shifted_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_printable_sequence_session_id);
    let sequence = CString::new("ba{Shift+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe {
            RimeSimulateKeySequence(shifted_printable_sequence_session_id, sequence.as_ptr())
        },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(shifted_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("bax")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        RimeDestroySession(shifted_printable_sequence_session_id),
        TRUE
    );

    let controlled_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_printable_session_id);
    assert_eq!(
        RimeProcessKey(controlled_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(controlled_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(controlled_printable_session_id, 'x' as i32, K_CONTROL_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(controlled_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_printable_session_id), TRUE);

    let controlled_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_printable_sequence_session_id);
    let sequence = CString::new("ba{Control+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe {
            RimeSimulateKeySequence(controlled_printable_sequence_session_id, sequence.as_ptr())
        },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(controlled_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        RimeDestroySession(controlled_printable_sequence_session_id),
        TRUE
    );

    let alt_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(alt_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(alt_printable_session_id);
    assert_eq!(
        RimeProcessKey(alt_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(alt_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(alt_printable_session_id, 'x' as i32, K_ALT_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(alt_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(alt_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(alt_printable_session_id), TRUE);

    let alt_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(alt_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(alt_printable_sequence_session_id);
    let sequence = CString::new("ba{Alt+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(alt_printable_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(alt_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(alt_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(alt_printable_sequence_session_id), TRUE);

    let super_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(super_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(super_printable_session_id);
    assert_eq!(
        RimeProcessKey(super_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(super_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(super_printable_session_id, 'x' as i32, K_SUPER_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(super_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(super_printable_session_id), TRUE);

    let super_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(super_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(super_printable_sequence_session_id);
    let sequence = CString::new("ba{Super+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(super_printable_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(super_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(super_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        RimeDestroySession(super_printable_sequence_session_id),
        TRUE
    );

    let released_printable_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(released_printable_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(released_printable_session_id);
    assert_eq!(
        RimeProcessKey(released_printable_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(released_printable_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(released_printable_session_id, 'x' as i32, K_RELEASE_MASK),
        FALSE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_printable_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(released_printable_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(released_printable_session_id), TRUE);

    let released_printable_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(released_printable_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(released_printable_sequence_session_id);
    let sequence = CString::new("ba{Release+x}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe {
            RimeSimulateKeySequence(released_printable_sequence_session_id, sequence.as_ptr())
        },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(released_printable_sequence_session_id, &mut commit) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(
        unsafe { RimeGetContext(released_printable_sequence_session_id, &mut context) },
        TRUE
    );
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        RimeDestroySession(released_printable_sequence_session_id),
        TRUE
    );

    let shifted_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_sequence_session_id);
    let sequence = CString::new("ba{Shift+2}B").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(shifted_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_sequence_session_id), TRUE);

    let controlled_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_session_id);
    assert_eq!(RimeProcessKey(controlled_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(controlled_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(controlled_session_id, '2' as i32, K_CONTROL_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_session_id), TRUE);

    let controlled_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_sequence_session_id);
    let sequence = CString::new("ba{Control+2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_sequence_session_id), TRUE);

    let controlled_shift_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_shift_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_shift_session_id);
    assert_eq!(
        RimeProcessKey(controlled_shift_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(controlled_shift_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(
            controlled_shift_session_id,
            '2' as i32,
            K_CONTROL_MASK | K_SHIFT_MASK
        ),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_shift_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_shift_session_id), TRUE);

    let controlled_shift_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_shift_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_shift_sequence_session_id);
    let sequence = CString::new("ba{Control+Shift+2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_shift_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_shift_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(
        RimeDestroySession(controlled_shift_sequence_session_id),
        TRUE
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn alternative_select_keys_keep_keypad_digit_fallback_like_librime_selector() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let kp_2 = CString::new("KP_2").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let kp_2_keycode = unsafe { RimeGetKeycodeByName(kp_2.as_ptr()) };
    assert_eq!(kp_2_keycode, 0xffb2);

    let root = unique_temp_dir("alternative-select-keys-keypad-digit");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let add_ba_translator = |session_id| {
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
    };

    let session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(session_id);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, kp_2_keycode, 0), TRUE);
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(sequence_session_id);
    let sequence = CString::new("ba{KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let shifted_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_session_id);
    assert_eq!(
        RimeProcessKey(shifted_session_id, kp_2_keycode, K_SHIFT_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(shifted_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(shifted_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(shifted_session_id, kp_2_keycode, K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_session_id), TRUE);

    let shifted_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(shifted_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(shifted_sequence_session_id);
    let sequence = CString::new("ba{Shift+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(shifted_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(shifted_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(shifted_sequence_session_id), TRUE);

    let controlled_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_session_id);
    assert_eq!(
        RimeProcessKey(controlled_session_id, kp_2_keycode, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(controlled_session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(controlled_session_id, 'a' as i32, 0), TRUE);
    assert_eq!(
        RimeProcessKey(controlled_session_id, kp_2_keycode, K_CONTROL_MASK),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_session_id), TRUE);

    let controlled_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_sequence_session_id);
    let sequence = CString::new("ba{Control+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_sequence_session_id), TRUE);

    let controlled_shift_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_shift_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_shift_session_id);
    assert_eq!(
        RimeProcessKey(
            controlled_shift_session_id,
            kp_2_keycode,
            K_CONTROL_MASK | K_SHIFT_MASK
        ),
        FALSE
    );
    assert_eq!(
        RimeProcessKey(controlled_shift_session_id, 'b' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(controlled_shift_session_id, 'a' as i32, 0),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(
            controlled_shift_session_id,
            kp_2_keycode,
            K_CONTROL_MASK | K_SHIFT_MASK
        ),
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_shift_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(controlled_shift_session_id), TRUE);

    let controlled_shift_sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(controlled_shift_sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(controlled_shift_sequence_session_id);
    let sequence = CString::new("ba{Control+Shift+KP_2}").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(controlled_shift_sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(controlled_shift_sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(
        RimeDestroySession(controlled_shift_sequence_session_id),
        TRUE
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn alternative_select_keys_beyond_page_size_are_consumed_like_librime_selector() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("alternative-select-keys-beyond-page-size");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: ABX\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let add_ba_translator = |session_id| {
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
            ("ba", "拔"),
        ]));
    };

    let session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(session_id);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'X' as i32, 0), TRUE);

    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    let mut context = empty_context();
    // SAFETY: `context` points to writable storage initialized with data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    // SAFETY: `preedit` is populated by `RimeGetContext` for active composition.
    assert_eq!(
        unsafe { CStr::from_ptr(context.composition.preedit) }.to_str(),
        Ok("ba")
    );
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeProcessKey(session_id, 'B' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let sequence_session_id = RimeCreateSession();
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(sequence_session_id, schema_id.as_ptr()) },
        TRUE
    );
    add_ba_translator(sequence_session_id);
    let sequence = CString::new("baXB").expect("sequence should be valid");
    // SAFETY: sequence is a valid NUL-terminated librime-style key sequence.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(sequence_session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(
        unsafe { RimeGetCommit(sequence_session_id, &mut commit) },
        TRUE
    );
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(RimeDestroySession(sequence_session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn delete_candidate_apis_remove_menu_items_without_commit() {
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
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));
    }
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    assert_eq!(RimeDeleteCandidate(session_id, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeDeleteCandidate(session_id, 1), TRUE);
    assert_eq!(RimeDeleteCandidate(session_id, 99), FALSE);

    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 5);
    // SAFETY: `context.menu.candidates` points to initialized candidates.
    let second_candidate = unsafe { *context.menu.candidates.add(1) };
    // SAFETY: candidate text is a valid NUL-terminated string owned by the
    // context object.
    let second_text = unsafe { CStr::from_ptr(second_candidate.text) };
    assert_eq!(second_text.to_str(), Ok("爸"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeChangePage(session_id, FALSE), TRUE);
    assert_eq!(RimeDeleteCandidateOnCurrentPage(session_id, 0), TRUE);
    assert_eq!(RimeDeleteCandidateOnCurrentPage(session_id, 5), FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.num_candidates, 5);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(RimeDeleteCandidate(0, 0), FALSE);
    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn iterates_candidate_list_from_current_context() {
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
    let mut iterator = empty_candidate_list_iterator();

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    // SAFETY: `iterator` points to valid writable storage.
    assert_eq!(
        unsafe { RimeCandidateListBegin(session_id, &mut iterator) },
        TRUE
    );
    // SAFETY: `iterator` was initialized by `RimeCandidateListBegin`.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let first_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(first_text.to_str(), Ok("八"));
    // SAFETY: current candidate includes a non-null comment.
    let first_comment = unsafe { CStr::from_ptr(iterator.candidate.comment) };
    assert_eq!(first_comment.to_str(), Ok("ba"));
    // SAFETY: `iterator` remains valid and owns the current candidate.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let second_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(second_text.to_str(), Ok("吧"));
    // SAFETY: `iterator` remains valid and owns the current candidate.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let third_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(third_text.to_str(), Ok("ba"));
    // SAFETY: `iterator` remains valid; librime leaves the current candidate
    // intact when advancing beyond the final item.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 3);
    // SAFETY: the failed advance preserves the previous candidate string.
    let preserved_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(preserved_text.to_str(), Ok("ba"));
    // SAFETY: `iterator` was initialized by this API and can be ended once.
    unsafe { RimeCandidateListEnd(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert!(iterator.candidate.text.is_null());
    assert!(iterator.candidate.comment.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn candidate_list_can_start_from_index_and_rejects_empty_menu() {
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
    let mut iterator = empty_candidate_list_iterator();

    // SAFETY: `iterator` points to valid writable storage.
    assert_eq!(
        unsafe { RimeCandidateListBegin(session_id, &mut iterator) },
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    // SAFETY: `iterator` points to valid writable storage.
    assert_eq!(
        unsafe { RimeCandidateListFromIndex(session_id, &mut iterator, 1) },
        TRUE
    );
    // SAFETY: `iterator` was initialized by `RimeCandidateListFromIndex`.
    assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(text.to_str(), Ok("吧"));
    // SAFETY: `iterator` was initialized by this API and can be ended once.
    unsafe { RimeCandidateListEnd(&mut iterator) };

    let mut negative_iterator = empty_candidate_list_iterator();
    // SAFETY: `negative_iterator` points to valid writable storage.
    assert_eq!(
        unsafe { RimeCandidateListFromIndex(session_id, &mut negative_iterator, -1) },
        TRUE
    );
    assert_eq!(negative_iterator.index, -2);
    // SAFETY: `negative_iterator` was initialized by this API. librime starts
    // one position before the requested index, so the first advance from -1
    // fails and leaves the public index at -1.
    assert_eq!(
        unsafe { RimeCandidateListNext(&mut negative_iterator) },
        FALSE
    );
    assert_eq!(negative_iterator.index, -1);
    assert!(negative_iterator.candidate.text.is_null());
    // SAFETY: the iterator remains valid after the failed negative advance.
    assert_eq!(
        unsafe { RimeCandidateListNext(&mut negative_iterator) },
        TRUE
    );
    // SAFETY: `RimeCandidateListNext` populated a valid C string.
    let first_text = unsafe { CStr::from_ptr(negative_iterator.candidate.text) };
    assert_eq!(first_text.to_str(), Ok("八"));
    // SAFETY: `negative_iterator` was initialized by this API and can be ended once.
    unsafe { RimeCandidateListEnd(&mut negative_iterator) };

    assert_eq!(RimeDestroySession(session_id), TRUE);
}
