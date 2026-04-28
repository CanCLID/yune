use super::*;

#[test]
fn state_label_apis_read_selected_schema_switches() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("state-label");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
switches:
  - name: ascii_mode
    states: [Native, Ascii]
    abbrev: [N, A]
  - name: full_shape
    states: [0, 1]
    abbrev: [H, true]
  - name: tri_state
    states: [Zero, One, Two]
  - options: [simplification, traditional]
    states: [简体, 繁體]
  - options: [0, 1]
    states: [Zero, One]
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
    let tri_state = CString::new("tri_state").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let numeric_option = CString::new("1").expect("option name should be valid");
    let missing = CString::new("missing").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    // SAFETY: option names are valid NUL-terminated strings.
    let full_label = unsafe { RimeGetStateLabel(session_id, ascii_mode.as_ptr(), TRUE) };
    assert!(!full_label.is_null());
    // SAFETY: non-null state-label pointers are process-owned C strings.
    assert_eq!(unsafe { CStr::from_ptr(full_label) }.to_str(), Ok("Ascii"));

    // SAFETY: option names are valid NUL-terminated strings.
    let abbreviated =
        unsafe { RimeGetStateLabelAbbreviated(session_id, ascii_mode.as_ptr(), TRUE, TRUE) };
    assert_eq!(abbreviated.length, 1);
    assert!(!abbreviated.str.is_null());
    // SAFETY: non-null state-label pointers are process-owned C strings.
    assert_eq!(unsafe { CStr::from_ptr(abbreviated.str) }.to_str(), Ok("A"));

    // SAFETY: option names are valid NUL-terminated strings.
    let scalar_state = unsafe { RimeGetStateLabel(session_id, full_shape.as_ptr(), TRUE) };
    assert!(!scalar_state.is_null());
    // SAFETY: non-null state-label pointers are process-owned C strings.
    assert_eq!(unsafe { CStr::from_ptr(scalar_state) }.to_str(), Ok("1"));

    // SAFETY: option names are valid NUL-terminated strings.
    let scalar_abbrev =
        unsafe { RimeGetStateLabelAbbreviated(session_id, full_shape.as_ptr(), TRUE, TRUE) };
    assert_eq!(scalar_abbrev.length, "true".len());
    assert!(!scalar_abbrev.str.is_null());
    // SAFETY: non-null state-label pointers are process-owned C strings.
    assert_eq!(
        unsafe { CStr::from_ptr(scalar_abbrev.str) }.to_str(),
        Ok("true")
    );

    // SAFETY: option names are valid NUL-terminated strings.
    let third_state = unsafe { RimeGetStateLabel(session_id, tri_state.as_ptr(), 2) };
    assert!(!third_state.is_null());
    // SAFETY: non-null state-label pointers are process-owned C strings.
    assert_eq!(unsafe { CStr::from_ptr(third_state) }.to_str(), Ok("Two"));
    assert!(unsafe { RimeGetStateLabel(session_id, ascii_mode.as_ptr(), 2) }.is_null());
    assert!(unsafe { RimeGetStateLabel(session_id, ascii_mode.as_ptr(), -1) }.is_null());

    // SAFETY: option names are valid NUL-terminated strings.
    let radio =
        unsafe { RimeGetStateLabelAbbreviated(session_id, simplification.as_ptr(), TRUE, TRUE) };
    assert_eq!(radio.length, "简".len());
    // SAFETY: `radio.str` points to a C string and `length` is within its
    // first UTF-8 scalar value.
    let radio_slice = unsafe { std::slice::from_raw_parts(radio.str.cast::<u8>(), radio.length) };
    assert_eq!(std::str::from_utf8(radio_slice), Ok("简"));

    // SAFETY: option names are valid NUL-terminated strings.
    let scalar_radio =
        unsafe { RimeGetStateLabelAbbreviated(session_id, numeric_option.as_ptr(), TRUE, FALSE) };
    assert_eq!(scalar_radio.length, "One".len());
    assert!(!scalar_radio.str.is_null());
    // SAFETY: non-null state-label pointers are process-owned C strings.
    assert_eq!(
        unsafe { CStr::from_ptr(scalar_radio.str) }.to_str(),
        Ok("One")
    );

    // SAFETY: option names are valid NUL-terminated strings.
    let hidden_radio =
        unsafe { RimeGetStateLabelAbbreviated(session_id, simplification.as_ptr(), FALSE, TRUE) };
    assert!(hidden_radio.str.is_null());
    assert_eq!(hidden_radio.length, 0);
    assert!(unsafe { RimeGetStateLabel(session_id, missing.as_ptr(), TRUE) }.is_null());
    assert!(unsafe { RimeGetStateLabel(0, ascii_mode.as_ptr(), TRUE) }.is_null());
    assert!(unsafe { RimeGetStateLabel(session_id, std::ptr::null(), TRUE) }.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn returns_context_with_preedit_and_candidate_page() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);

    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert_eq!(context.composition.cursor_pos, 2);
    assert_eq!(context.composition.sel_start, 0);
    assert_eq!(context.composition.sel_end, 2);
    // SAFETY: `RimeGetContext` returned true and populated owned C strings.
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) };
    assert_eq!(preedit.to_str(), Ok("ni"));

    assert_eq!(context.menu.page_size, 5);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.is_last_page, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    assert_eq!(context.menu.num_candidates, 1);
    assert!(!context.menu.candidates.is_null());
    // SAFETY: `context.menu.candidates` points to one initialized candidate.
    let candidate = unsafe { *context.menu.candidates };
    // SAFETY: candidate strings are valid NUL-terminated strings owned by
    // the context object.
    let candidate_text = unsafe { CStr::from_ptr(candidate.text) };
    assert_eq!(candidate_text.to_str(), Ok("ni"));
    // SAFETY: the echo candidate includes a non-null comment.
    let candidate_comment = unsafe { CStr::from_ptr(candidate.comment) };
    assert_eq!(candidate_comment.to_str(), Ok("echo"));

    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert!(context.composition.preedit.is_null());
    assert!(context.menu.candidates.is_null());
    assert_eq!(context.menu.num_candidates, 0);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rime_context_hides_candidate_entries_when_librime_hide_candidate_option_is_set() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let hide_candidate =
        CString::new("_hide_candidate").expect("option name should be a valid C string");
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, hide_candidate.as_ptr(), TRUE) };

    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert!(!context.composition.preedit.is_null());
    assert!(!context.commit_text_preview.is_null());
    assert_eq!(context.menu.page_size, 5);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.is_last_page, TRUE);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert!(context.menu.candidates.is_null());
    assert!(context.menu.select_keys.is_null());
    assert!(context.select_labels.is_null());

    // SAFETY: `RimeGetContext` returned true and populated owned C strings.
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) };
    assert_eq!(preedit.to_str(), Ok("ni"));
    // SAFETY: `RimeGetContext` returned true and populated owned C strings.
    let preview = unsafe { CStr::from_ptr(context.commit_text_preview) };
    assert_eq!(preview.to_str(), Ok("ni"));

    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert!(context.composition.preedit.is_null());
    assert!(context.commit_text_preview.is_null());
    assert_eq!(context.menu.num_candidates, 0);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rime_context_reads_librime_menu_settings_from_selected_schema() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("context-menu-settings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n  alternative_select_labels: [Alpha, Beta]\n",
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
            ("ni", "你"),
            ("ni", "尼"),
            ("ni", "呢"),
            ("ni", "泥"),
            ("ni", "拟"),
        ]));
    }
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 3), TRUE);

    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_size, 2);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    assert_eq!(context.menu.num_candidates, 2);
    assert!(!context.menu.select_keys.is_null());
    // SAFETY: `RimeGetContext` returned true and populated a select-key string.
    let select_keys = unsafe { CStr::from_ptr(context.menu.select_keys) };
    assert_eq!(select_keys.to_str(), Ok("AB"));
    assert!(!context.select_labels.is_null());
    // SAFETY: `RimeGetContext` returned true and populated one label per page slot.
    let select_labels = unsafe {
        std::slice::from_raw_parts(context.select_labels, context.menu.page_size as usize)
    };
    // SAFETY: label pointers are valid NUL-terminated strings owned by the context object.
    assert_eq!(
        unsafe { CStr::from_ptr(select_labels[0]) }.to_str(),
        Ok("Alpha")
    );
    // SAFETY: label pointers are valid NUL-terminated strings owned by the context object.
    assert_eq!(
        unsafe { CStr::from_ptr(select_labels[1]) }.to_str(),
        Ok("Beta")
    );
    // SAFETY: `context.menu.candidates` points to `num_candidates` entries.
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    // SAFETY: candidate texts are valid strings owned by the context object.
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("呢")
    );
    // SAFETY: candidate texts are valid strings owned by the context object.
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
        Ok("泥")
    );

    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert!(context.menu.select_keys.is_null());
    assert!(context.select_labels.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn rime_context_includes_librime_commit_text_preview_for_current_selection() {
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
            .add_translator(StaticTableTranslator::new([("ni", "你"), ("ni", "呢")]));
    }
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeHighlightCandidate(session_id, 1), TRUE);

    // SAFETY: `context` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert!(!context.commit_text_preview.is_null());
    // SAFETY: `RimeGetContext` returned true and populated a preview string.
    let preview = unsafe { CStr::from_ptr(context.commit_text_preview) };
    assert_eq!(preview.to_str(), Ok("呢"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert!(context.commit_text_preview.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rime_context_clear_respects_librime_versioned_tail_members() {
    let _guard = test_guard();
    let mut context = empty_context();
    let preview = CString::new("preserve-preview")
        .expect("literal should be valid")
        .into_raw();
    let label = CString::new("preserve-label")
        .expect("literal should be valid")
        .into_raw();
    let mut labels = vec![label];
    let labels_ptr = labels.as_mut_ptr();
    std::mem::forget(labels);

    context.data_size = context_data_size_before_commit_text_preview();
    context.menu.page_size = 1;
    context.commit_text_preview = preview;
    context.select_labels = labels_ptr;

    // SAFETY: `context` points to valid writable storage. Its tail members are
    // valid allocations but are outside the caller-declared version boundary.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        context.data_size,
        context_data_size_before_commit_text_preview()
    );
    assert_eq!(context.commit_text_preview, preview);
    assert_eq!(context.select_labels, labels_ptr);

    // SAFETY: the older-version context did not transfer ownership of tail
    // members to `RimeFreeContext`, so the test reclaims its own allocations.
    unsafe {
        drop(CString::from_raw(preview));
        let labels = Vec::from_raw_parts(labels_ptr, 1, 1);
        for label in labels {
            drop(CString::from_raw(label));
        }
    }
}

#[test]
fn returns_status_with_schema_and_composing_flags() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut status = empty_status();

    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: `RimeGetStatus` returned true and populated owned C strings.
    let schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    // SAFETY: `RimeGetStatus` returned true and populated owned C strings.
    let schema_name = unsafe { CStr::from_ptr(status.schema_name) };
    assert_eq!(schema_id.to_str(), Ok("default"));
    assert_eq!(schema_name.to_str(), Ok("Default"));
    assert_eq!(status.is_composing, FALSE);
    assert_eq!(status.is_ascii_mode, FALSE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);
    assert!(status.schema_id.is_null());
    assert!(status.schema_name.is_null());

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rime_status_clear_preserves_librime_struct_data_size() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let expected_data_size =
        (std::mem::size_of::<RimeStatus>() - std::mem::size_of::<i32>()) as i32;
    let mut status = empty_status();
    status.data_size = expected_data_size;

    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id + 1, &mut status) }, FALSE);
    assert_eq!(status.data_size, expected_data_size);
    assert!(status.schema_id.is_null());
    assert!(status.schema_name.is_null());

    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.data_size, expected_data_size);
    assert!(!status.schema_id.is_null());
    assert!(!status.schema_name.is_null());

    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);
    assert_eq!(status.data_size, expected_data_size);
    assert!(status.schema_id.is_null());
    assert!(status.schema_name.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rejects_invalid_context_requests() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut context = empty_context();
    context.data_size = 0;

    // SAFETY: `context` points to writable storage but has invalid
    // librime-style data_size metadata.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, FALSE);
    // SAFETY: null pointers are explicitly rejected.
    assert_eq!(
        unsafe { RimeGetContext(session_id, std::ptr::null_mut()) },
        FALSE
    );
    // SAFETY: `context` points to writable storage but has invalid
    // librime-style data_size metadata.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rejects_invalid_status_requests() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut status = empty_status();
    status.data_size = 0;

    // SAFETY: `status` points to writable storage but has invalid
    // librime-style data_size metadata.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, FALSE);
    // SAFETY: null pointers are explicitly rejected.
    assert_eq!(
        unsafe { RimeGetStatus(session_id, std::ptr::null_mut()) },
        FALSE
    );
    // SAFETY: `status` points to writable storage but has invalid
    // librime-style data_size metadata.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}
