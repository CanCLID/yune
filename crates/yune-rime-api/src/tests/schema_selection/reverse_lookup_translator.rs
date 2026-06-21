#[test]
fn select_schema_loads_librime_reverse_lookup_translator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-translator");
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
  translators:
    - reverse_lookup_translator
    - table_translator
    - echo_translator
abc_segmentor:
  extra_tags: [reverse_lookup]
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
  comment_format:
    - xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/
    - xform/^/〔/
    - xform/$/〕/
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

火\tho
火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
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
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("〔HO; HUO〕".to_owned())),
            ("`huo".to_owned(), Some("echo".to_owned()))
        ]
    );

    RimeClearComposition(session_id);
    for ch in "huo".chars() {
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
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("huo".to_owned())),
            ("火".to_owned(), Some("〔HO; HUO〕".to_owned())),
            ("huo".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_affix_prompt_matches_typeduck_v112_reverse_lookup_fixture_commit_preview() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let fixture = typeduck_v112_reverse_lookup_prompt_fixture();
    let case = typeduck_v112_reverse_lookup_case(&fixture);
    let root = unique_temp_dir("schema-reverse-lookup-prompt");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");

    let schema_name = fixture["capture"]["schema_name"]
        .as_str()
        .expect("oracle fixture should capture schema name");
    let tips = fixture["capture"]["reverse_lookup_tips"]
        .as_str()
        .expect("oracle fixture should capture reverse lookup tips");
    fs::write(
        staging.join("hr6_reverse.schema.yaml"),
        format!(
            "\
schema:
  schema_id: hr6_reverse
  name: \"{schema_name}\"
engine:
  segmentors:
    - abc_segmentor
    - affix_segmentor@reverse_lookup
  translators:
    - reverse_lookup_translator
    - table_translator
translator:
  dictionary: hr6_target
reverse_lookup:
  tag: reverse_lookup
  dictionary: hr6_lookup
  prefix: \"`\"
  suffix: \";\"
  tips: \"{tips}\"
"
        ),
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("hr6_lookup.dict.yaml"),
        dictionary_yaml_from_oracle_rows(
            "hr6_lookup",
            &fixture["capture"]["lookup_dictionary_rows"],
        ),
    )
    .expect("reverse lookup dictionary should be written");
    fs::write(
        shared.join("hr6_target.dict.yaml"),
        dictionary_yaml_from_oracle_rows(
            "hr6_target",
            &fixture["capture"]["target_dictionary_rows"],
        ),
    )
    .expect("target dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new(case["schema_id"].as_str().expect("schema id should exist"))
        .expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in case["input"]
        .as_str()
        .expect("oracle input should exist")
        .chars()
    {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_str()
        .expect("preedit should be valid UTF-8")
        .to_owned();
    let commit_text_preview = unsafe { CStr::from_ptr(context.commit_text_preview) }
        .to_str()
        .expect("commit preview should be valid UTF-8")
        .to_owned();
    let composition_length = context.composition.length as usize;
    let cursor_pos = context.composition.cursor_pos as usize;
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert!(!candidates.is_empty());
    let first_text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8")
        .to_owned();
    let first_comment = unsafe { CStr::from_ptr(candidates[0].comment) }
        .to_str()
        .expect("candidate comment should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        preedit,
        case["preedit"]
            .as_str()
            .expect("oracle preedit should exist")
    );
    assert_eq!(composition_length, preedit.len());
    assert_eq!(
        cursor_pos,
        case["input"]
            .as_str()
            .expect("oracle input should exist")
            .trim_start_matches('`')
            .len()
    );
    assert_eq!(
        commit_text_preview,
        case["commit_text_preview"]
            .as_str()
            .expect("oracle commit preview should exist")
    );
    assert_eq!(
        first_text,
        case["selected_candidates"][0]["text"]
            .as_str()
            .expect("oracle candidate text should exist")
    );
    assert_eq!(
        first_comment,
        case["selected_candidates"][0]["comment"]
            .as_str()
            .expect("oracle candidate comment should exist")
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_matcher_segmentor_adds_librime_recognizer_pattern_tags() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-matcher-segmentor-tags");
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
    - matcher
  translators:
    - reverse_lookup_translator
    - table_translator
    - echo_translator
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
recognizer:
  patterns:
    reverse_lookup: \"`[a-z]*'?$\"
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

火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
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

    assert_eq!(texts, ["火".to_owned(), "`huo".to_owned()]);

    RimeClearComposition(session_id);
    for ch in "huo".chars() {
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

    assert_eq!(texts, ["火".to_owned(), "huo".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_affix_segmentor_tags_librime_prefixed_lookup() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-affix-segmentor-tags");
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
    - affix_segmentor@reverse_lookup
  translators:
    - reverse_lookup_translator
    - echo_translator
reverse_lookup:
  tag: reverse_lookup
  dictionary: stroke
  prefix: \"`\"
  suffix: \"'\"
  extra_tags: [lookup_extra]
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

火\thuo
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
    for ch in "`huo'".chars() {
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

    assert_eq!(texts, ["火".to_owned(), "`huo'".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_affix_segmentor_tags_are_exclusive_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-affix-segmentor-exclusive-tags");
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
    - affix_segmentor@reverse_lookup
  translators:
    - reverse_lookup_translator
    - table_translator
    - echo_translator
translator:
  dictionary: luna
reverse_lookup:
  tag: reverse_lookup
  dictionary: stroke
  prefix: \"`\"
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

桌\thuo
",
    )
    .expect("table dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
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

    assert_eq!(texts, ["火".to_owned(), "`huo".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_matcher_segmentor_uses_librime_sorted_recognizer_pattern_tags() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-matcher-segmentor-pattern-order");
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
    - matcher
  translators:
    - reverse_lookup_translator
    - table_translator
    - echo_translator
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
recognizer:
  patterns:
    z_custom: \"`[a-z]*$\"
    reverse_lookup: \"`[a-z]*$\"
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

火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
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

    assert_eq!(texts, ["火".to_owned(), "`huo".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_matcher_segmentor_segmentor_namespace_reads_recognizer_patterns() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-matcher-segmentor-namespace");
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
    - matcher@segmentor
  translators:
    - reverse_lookup_translator
    - table_translator
    - echo_translator
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
recognizer:
  patterns:
    reverse_lookup: \"`[a-z]*'?$\"
segmentor:
  patterns:
    reverse_lookup: \"^never$\"
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

火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
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

    assert_eq!(texts, ["火".to_owned(), "`huo".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_treats_librime_reverse_lookup_translator_namespace_as_reverse_lookup() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-translator-namespace");
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
  translators:
    - reverse_lookup_translator@translator
    - table_translator
    - echo_translator
abc_segmentor:
  extra_tags: [reverse_lookup]
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
  comment_format:
    - xform/^/stroke:/
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

火\tho
火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
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
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("stroke:ho; huo".to_owned())),
            ("`huo".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_reverse_lookup_translator_honors_librime_tag() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-translator-tag");
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
  translators:
    - reverse_lookup_translator
    - echo_translator
abc_segmentor:
  extra_tags: [custom_lookup]
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
  tag: custom_lookup
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

火\thuo
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
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("huo".to_owned())),
            ("`huo".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_reverse_lookup_completion_when_enabled() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-completion");
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
  translators:
    - reverse_lookup_translator
    - table_translator
    - echo_translator
abc_segmentor:
  extra_tags: [reverse_lookup]
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
  enable_completion: true
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

火\tho
火\thuo
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火\thuo
水\tshui
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
    for ch in "`hu".chars() {
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
    let texts_and_comments = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                None
            } else {
                Some(
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned(),
                )
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(
        texts_and_comments,
        [
            ("火".to_owned(), Some("ho; huo".to_owned())),
            ("`hu".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
