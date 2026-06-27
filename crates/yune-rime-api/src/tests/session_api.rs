use super::*;

#[test]
fn finalize_clears_sessions_but_preserves_notification_handler() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    RimeSetNotificationHandler(None, std::ptr::null_mut());
    notification_events_lock().clear();
    let context_object = 0x7c_usize as *mut c_void;
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");

    RimeSetNotificationHandler(Some(record_notification), context_object);
    let old_session_id = RimeCreateSession();
    assert_ne!(old_session_id, 0);
    RimeFinalize();
    assert_eq!(RimeFindSession(old_session_id), FALSE);
    assert_eq!(RimeCreateSession(), 0);

    let traits = empty_traits();
    // SAFETY: traits points to valid storage.
    unsafe { RimeInitialize(&traits) };
    let new_session_id = RimeCreateSession();
    assert_ne!(new_session_id, 0);
    // SAFETY: option is a valid NUL-terminated C string.
    unsafe { RimeSetOption(new_session_id, ascii_mode.as_ptr(), TRUE) };

    let events = notification_events_lock();
    assert_eq!(
        *events,
        vec![NotificationEvent {
            context_object: 0x7c,
            session_id: new_session_id,
            message_type: "option".to_owned(),
            message_value: "ascii_mode".to_owned(),
        }]
    );
    drop(events);

    RimeSetNotificationHandler(None, std::ptr::null_mut());
    assert_eq!(RimeDestroySession(new_session_id), TRUE);
}

#[test]
fn creates_finds_and_destroys_sessions() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let session_id = RimeCreateSession();

    assert_ne!(session_id, 0);
    assert_eq!(RimeFindSession(session_id), TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);
    assert_eq!(RimeFindSession(session_id), FALSE);
}

#[test]
fn processes_ascii_keys_and_returns_unread_commit_once() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text` with a
    // valid NUL-terminated C string owned by the commit object.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert!(commit.text.is_null());
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn accumulates_unread_commit_text_like_librime_session() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };

    for ch in "ni hao ".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as i32, 0), TRUE);
    }

    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text` with a
    // valid NUL-terminated C string owned by the commit object.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("nihao"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn rime_commit_clear_preserves_librime_struct_data_size() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let expected_data_size =
        (std::mem::size_of::<RimeCommit>() - std::mem::size_of::<i32>()) as i32;
    let mut commit = RimeCommit {
        data_size: expected_data_size,
        text: std::ptr::null_mut(),
    };

    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    assert_eq!(commit.data_size, expected_data_size);
    assert!(commit.text.is_null());

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    assert_eq!(commit.data_size, expected_data_size);

    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
    assert_eq!(commit.data_size, expected_data_size);
    assert!(commit.text.is_null());

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn process_key_commits_numeric_candidate_selection() {
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

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text` with a
    // valid NUL-terminated C string owned by the commit object.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("吧"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn commits_composition_explicitly_and_returns_unread_commit() {
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
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    assert_eq!(RimeCommitComposition(session_id), FALSE);
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeCommitComposition(session_id), TRUE);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("你"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn clears_composition_without_creating_commit() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    RimeClearComposition(session_id);
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn gets_and_sets_input_and_caret_position() {
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
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let mut context = empty_context();

    assert_eq!(RimeGetInput(0), std::ptr::null());
    assert_eq!(RimeGetCaretPos(0), 0);
    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeGetCaretPos(session_id), 2);

    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    let input = unsafe { CStr::from_ptr(input) };
    assert_eq!(input.to_str(), Ok("ni"));

    RimeSetCaretPos(session_id, 1);
    assert_eq!(RimeGetCaretPos(session_id), 1);
    RimeSetCaretPos(session_id, 10);
    assert_eq!(RimeGetCaretPos(session_id), 2);

    let new_input = CString::new("ni").expect("literal should not contain NUL");
    // SAFETY: `new_input` is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeSetInput(session_id, new_input.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeGetCaretPos(session_id), 2);
    // SAFETY: `context` points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 2);
    // SAFETY: `context.menu.candidates` points to initialized candidates.
    let first_candidate = unsafe { *context.menu.candidates };
    // SAFETY: candidate text is a valid NUL-terminated string owned by the
    // context object.
    let first_candidate_text = unsafe { CStr::from_ptr(first_candidate.text) };
    assert_eq!(first_candidate_text.to_str(), Ok("你"));
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: null pointers are explicitly rejected.
    assert_eq!(unsafe { RimeSetInput(session_id, std::ptr::null()) }, FALSE);
    // SAFETY: `new_input` is a valid NUL-terminated C string.
    assert_eq!(
        unsafe { RimeSetInput(session_id + 1, new_input.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn get_context_uses_page_snapshot_without_full_candidate_clone() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    yune_core::m37_metrics_enable(true);
    yune_core::m37_metrics_reset();
    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.clear_translators();
        session.engine.add_translator(StaticTableTranslator::new([
            ("ba", "candidate-00"),
            ("ba", "candidate-01"),
            ("ba", "candidate-02"),
            ("ba", "candidate-03"),
            ("ba", "candidate-04"),
            ("ba", "candidate-05"),
            ("ba", "candidate-06"),
        ]));
    }
    let mut context = empty_context();

    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    // SAFETY: `context` points to writable storage initialized with a positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 5);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    let metrics = yune_core::m37_metrics_snapshot();
    yune_core::m37_metrics_enable(false);
    assert_eq!(metrics.context_full_snapshot_candidates_cloned, 0);
    assert_eq!(metrics.context_page_snapshot_candidates_cloned, 5);
    assert_eq!(metrics.abi_candidates_exported, 5);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn m37_metrics_exports_snapshot_json_for_loaded_benchmarks() {
    crate::yune_m37_metrics_enable(TRUE);
    crate::yune_m37_metrics_reset();
    yune_core::m37_record_abi_candidates_exported(3);
    yune_core::m37_record_owned_candidate_materialization(std::time::Duration::from_nanos(13));
    yune_core::m37_record_prism_lookup(std::time::Duration::from_nanos(29), 2);
    yune_core::m37_record_abi_c_string_allocation_duration(11, std::time::Duration::from_nanos(31));
    yune_core::m37_record_sentence_candidate(std::time::Duration::from_nanos(17), 1);
    yune_core::m37_record_upstream_sentence_model_index_build(std::time::Duration::from_nanos(19));
    yune_core::m37_record_upstream_sentence_model_lookup_index(
        yune_core::M40SentenceLookupMetrics {
            exact_range_index_hits: 2,
            exact_range_index_misses: 1,
            prefix_filter_hits: 3,
            prefix_filter_misses: 1,
            prefix_filter_early_breaks: 1,
            reachable_starts_visited: 2,
            unreachable_starts_skipped: 1,
            phrase_index_walk_calls: 2,
            phrase_index_nodes_visited: 3,
            phrase_index_entry_ranges_emitted: 2,
            partition_point_fallback_calls: 0,
            graph_rebuild_duration: std::time::Duration::from_nanos(23),
            incremental_reuse_hits: 0,
            incremental_extend_duration: std::time::Duration::ZERO,
            incremental_discarded_rebuild_chars: 4,
        },
    );

    let json = crate::yune_m37_metrics_snapshot_json();
    assert!(!json.is_null());
    // SAFETY: the snapshot export returns a valid NUL-terminated string.
    let json_text = unsafe { CStr::from_ptr(json) }
        .to_str()
        .expect("snapshot JSON should be UTF-8");
    assert!(json_text.contains("\"abi_candidates_exported\":3"));
    assert!(json_text.contains("\"owned_candidates_materialized\":1"));
    assert!(json_text.contains("\"owned_candidate_materialization_ns\":13"));
    assert!(json_text.contains("\"prism_lookup_calls\":1"));
    assert!(json_text.contains("\"prism_lookup_ns\":29"));
    assert!(json_text.contains("\"prism_lookup_codes\":2"));
    assert!(json_text.contains("\"abi_c_string_allocations\":1"));
    assert!(json_text.contains("\"abi_c_string_bytes\":11"));
    assert!(json_text.contains("\"abi_c_string_allocation_ns\":31"));
    assert!(json_text.contains("\"sentence_candidate_calls\":1"));
    assert!(json_text.contains("\"sentence_candidate_ns\":17"));
    assert!(json_text.contains("\"prefix_fallback_calls\":0"));
    assert!(json_text.contains("\"upstream_sentence_model_calls\":0"));
    assert!(json_text.contains("\"upstream_sentence_model_code_prefix_checks\":0"));
    assert!(json_text.contains("\"upstream_sentence_model_index_build_calls\":1"));
    assert!(json_text.contains("\"upstream_sentence_model_index_build_ns\":19"));
    assert!(json_text.contains("\"upstream_sentence_model_exact_range_index_hits\":2"));
    assert!(json_text.contains("\"upstream_sentence_model_exact_range_index_misses\":1"));
    assert!(json_text.contains("\"upstream_sentence_model_prefix_filter_hits\":3"));
    assert!(json_text.contains("\"upstream_sentence_model_prefix_filter_misses\":1"));
    assert!(json_text.contains("\"upstream_sentence_model_prefix_filter_early_breaks\":1"));
    assert!(json_text.contains("\"upstream_sentence_model_reachable_starts_visited\":2"));
    assert!(json_text.contains("\"upstream_sentence_model_unreachable_starts_skipped\":1"));
    assert!(json_text.contains("\"upstream_sentence_model_phrase_index_walk_calls\":2"));
    assert!(json_text.contains("\"upstream_sentence_model_phrase_index_nodes_visited\":3"));
    assert!(json_text.contains("\"upstream_sentence_model_phrase_index_entry_ranges_emitted\":2"));
    assert!(json_text.contains("\"upstream_sentence_model_partition_point_fallback_calls\":0"));
    assert!(json_text.contains("\"upstream_sentence_model_graph_rebuild_calls\":1"));
    assert!(json_text.contains("\"upstream_sentence_model_graph_rebuild_ns\":23"));
    assert!(json_text.contains("\"upstream_sentence_model_incremental_reuse_hits\":0"));
    assert!(json_text.contains("\"upstream_sentence_model_incremental_extend_ns\":0"));
    assert!(json_text.contains("\"upstream_sentence_model_incremental_discarded_rebuild_chars\":4"));
    for field in [
        "abbreviation_span_discovery_calls",
        "abbreviation_span_discovery_ns",
        "abbreviation_span_candidates_considered",
        "abbreviation_span_codes_emitted",
        "abbreviation_model_has_code_calls",
        "abbreviation_model_has_code_ns",
        "abbreviation_code_span_graph_build_ns",
        "abbreviation_sentence_ranking_ns",
        "abbreviation_preedit_format_ns",
        "abbreviation_candidate_format_ns",
        "short_key_candidate_rows_scanned",
        "short_key_candidates_materialized",
        "short_key_candidates_cloned",
        "short_key_filter_ns",
        "short_key_sort_rank_ns",
        "short_key_comment_quality_ns",
        "short_key_first_page_materialize_ns",
        "track_b_spelling_expansions_considered",
        "track_b_spelling_expansion_ns",
        "track_b_exact_lookup_calls",
        "track_b_exact_lookup_ns",
        "track_b_prefix_lookup_calls",
        "track_b_prefix_lookup_ns",
        "track_b_candidates_materialized",
        "track_b_first_page_materialize_ns",
    ] {
        assert!(
            crate::M37_METRIC_FIELDS.contains(&field),
            "M44 metric {field} must be listed for m37_metrics.csv export"
        );
        assert!(
            json_text.contains(&format!("\"{field}\":0")),
            "M44 metric {field} must be exported in the snapshot JSON: {json_text}"
        );
    }
    // SAFETY: `json` is owned by the metrics export.
    unsafe { crate::yune_m37_metrics_free_string(json) };
    crate::yune_m37_metrics_enable(FALSE);
}

#[test]
fn m43_memory_owner_profile_exports_required_session_rows() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    assert_ne!(session_id, 0);
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session.engine.clear_translators();
        session.engine.add_translator(
            StaticTableTranslator::new([
                ("ni", "你"),
                ("hao", "好"),
                ("zhong", "中"),
                ("guo", "國"),
            ])
            .with_upstream_sentence_model(5),
        );
    }

    let json = crate::yune_m43_memory_owner_profile_json();
    assert!(!json.is_null());
    // SAFETY: the owner-profile export returns a valid NUL-terminated string.
    let json_text = unsafe { CStr::from_ptr(json) }
        .to_str()
        .expect("owner profile JSON should be UTF-8");
    for owner in [
        "translator.entries_by_code",
        "poet.entries_by_code",
        "poet.lookup_index",
        "poet.abbreviation_vocabulary",
        "schema.config",
        "schema.processors",
        "session.userdb",
        "runtime.session_state",
    ] {
        assert!(json_text.contains(owner), "missing owner row {owner}");
    }
    assert!(json_text.contains("heap_owned_reducible"));
    assert!(json_text.contains("heap_owned_guarded"));
    assert!(json_text.contains("overlap_estimate"));
    // SAFETY: `json` is owned by the metrics/string export surface.
    unsafe { crate::yune_m37_metrics_free_string(json) };

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn cleanup_stale_sessions_matches_librime_activity_lifespan() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let stale_session_id = RimeCreateSession();
    let refreshed_session_id = RimeCreateSession();
    let fresh_session_id = RimeCreateSession();
    let stale_time = crate::session_activity_now().saturating_sub(crate::SESSION_LIFESPAN_SECS + 1);

    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        registry
            .sessions
            .get_mut(&stale_session_id)
            .expect("stale session should exist")
            .last_active_time = stale_time;
        registry
            .sessions
            .get_mut(&refreshed_session_id)
            .expect("refreshed session should exist")
            .last_active_time = stale_time;
    }

    assert_eq!(RimeFindSession(refreshed_session_id), TRUE);
    RimeCleanupStaleSessions();

    assert_eq!(RimeFindSession(stale_session_id), FALSE);
    assert_eq!(RimeFindSession(refreshed_session_id), TRUE);
    assert_eq!(RimeFindSession(fresh_session_id), TRUE);

    RimeCleanupAllSessions();
}

#[test]
fn rejects_unknown_sessions_and_modified_keys() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();

    assert_eq!(RimeProcessKey(0, 'a' as i32, 0), FALSE);
    assert_eq!(RimeProcessKey(session_id + 1, 'a' as i32, 0), FALSE);
    assert_eq!(
        RimeProcessKey(session_id, 'a' as i32, K_CONTROL_MASK),
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 1 << 3), FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
}
