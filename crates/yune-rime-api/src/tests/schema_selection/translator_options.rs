#[test]
fn select_schema_wires_prediction_threshold_and_never_first_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-prediction-options");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("prediction.schema.yaml"),
        "\
schema:
  schema_id: prediction
  name: Prediction
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: prediction
  enable_completion: true
  prediction_weight_threshold: 3
  prediction_never_first: true
",
    )
    .expect("prediction schema config should be written");
    fs::write(
        shared.join("prediction.dict.yaml"),
        "\
---
name: prediction
version: '0.1'
sort: by_weight
columns: [text, code, weight]
...

EXACT\tsan\t1
LOW\tsanlow\t2
HIGH\tsanhigh\t4
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
    let schema_id = CString::new("prediction").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in ['s', 'a', 'n'] {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    let texts = current_candidate_pairs(session_id)
        .into_iter()
        .map(|(text, _)| text)
        .collect::<Vec<_>>();

    assert_eq!(texts, ["EXACT", "HIGH", "san"]);
    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_prediction_never_first_applies_to_learned_prefix_predictions() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-prediction-never-first-userdb");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("prediction.schema.yaml"),
        "\
schema:
  schema_id: prediction
  name: Prediction
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: prediction
  enable_completion: true
  prediction_never_first: true
",
    )
    .expect("prediction schema config should be written");
    fs::write(
        shared.join("prediction.dict.yaml"),
        "\
---
name: prediction
version: '0.1'
sort: by_weight
columns: [text, code, weight]
...

EXACT\tsan\t1
HIGH\tsanhigh\t4
",
    )
    .expect("dictionary should be written");
    fs::write(
        user.join("prediction.userdb"),
        "\
# yune userdb
/db_name\tprediction
/db_type\tuserdb
/tick\t5
sanlearn \tLEARNED\tc=2 d=2 t=2
",
    )
    .expect("seed userdb should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("prediction").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in ['s', 'a', 'n'] {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    let texts = current_candidate_pairs(session_id)
        .into_iter()
        .map(|(text, _)| text)
        .collect::<Vec<_>>();

    assert_eq!(texts, ["EXACT", "LEARNED", "HIGH", "san"]);
    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_wires_typeduck_table_translator_comment_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-typeduck-table-comment-options");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("affix-default.schema.yaml"),
        "\
schema:
  schema_id: affix-default
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: options
  prefix: x
  suffix: z
  enable_sentence: false
",
    )
    .expect("default schema config should be written");
    fs::write(
        staging.join("affix-combined.schema.yaml"),
        "\
schema:
  schema_id: affix-combined
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: options
  prefix: x
  suffix: z
  show_full_code: true
  combine_candidates: true
  enable_sentence: false
",
    )
    .expect("combined schema config should be written");
    fs::write(
        shared.join("options.dict.yaml"),
        "\
---
name: options
version: '0.1'
sort: original
columns: [code, text, weight]
...

cam\tOPTION\t2
caam\tOPTION\t1
caa\tOTHER\t0
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
    let candidate_pairs_for = |schema: &str| {
        RimeClearComposition(session_id);
        let schema_id = CString::new(schema).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        for ch in "xcaz".chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let pairs = candidates
            .iter()
            .map(|candidate| {
                let text = unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned();
                let comment = if candidate.comment.is_null() {
                    String::new()
                } else {
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned()
                };
                (text, comment)
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        pairs
    };

    assert_eq!(
        candidate_pairs_for("affix-default")[0],
        ("OPTION".to_owned(), "~m".to_owned()),
        "affix should strip prefix/suffix and default show_full_code=false should keep short comments"
    );
    let combined_pairs = candidate_pairs_for("affix-combined");
    assert_eq!(combined_pairs[0].0, "OPTION");
    assert!(
        matches!(combined_pairs[0].1.as_str(), "cam;caam" | "caam;cam"),
        "schema combine_candidates and show_full_code should reach the installed translator: {combined_pairs:?}"
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_can_skip_namespaced_translator_load_for_optional_ui_pack() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-skip-namespaced-translator-load");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("eager.schema.yaml"),
        "\
schema:
  schema_id: eager
engine:
  translators:
    - table_translator
    - table_translator@secondary
    - echo_translator
translator:
  dictionary: primary
  enable_completion: false
secondary:
  dictionary: secondary
  prefix: \"`\"
  enable_completion: false
",
    )
    .expect("eager schema config should be written");
    fs::write(
        staging.join("keyboard.schema.yaml"),
        "\
schema:
  schema_id: keyboard
engine:
  translators:
    - table_translator
    - table_translator@secondary
    - echo_translator
translator:
  dictionary: primary
  enable_completion: false
secondary:
  dictionary: secondary
  prefix: \"`\"
  enable_completion: false
  load_translator: false
",
    )
    .expect("keyboard schema config should be written");
    fs::write(
        shared.join("primary.dict.yaml"),
        "\
---
name: primary
version: '0.1'
sort: original
columns: [code, text]
...

a\tPRIMARY
",
    )
    .expect("primary dictionary should be written");
    fs::write(
        shared.join("secondary.dict.yaml"),
        "\
---
name: secondary
version: '0.1'
sort: original
columns: [code, text]
...

b\tSECONDARY
",
    )
    .expect("secondary dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let candidate_texts_for = |schema: &str, input: &str| {
        RimeClearComposition(session_id);
        let schema_id = CString::new(schema).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        current_candidate_pairs(session_id)
            .into_iter()
            .map(|(text, _)| text)
            .collect::<Vec<_>>()
    };

    assert_eq!(candidate_texts_for("eager", "a")[0], "PRIMARY");
    assert!(
        candidate_texts_for("eager", "`b").contains(&"SECONDARY".to_owned()),
        "default namespaced translator path should remain eager"
    );
    assert_eq!(candidate_texts_for("keyboard", "a")[0], "PRIMARY");
    assert!(
        !candidate_texts_for("keyboard", "`b").contains(&"SECONDARY".to_owned()),
        "keyboard-profile opt-out should skip the optional prefixed translator"
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_tag_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-tags");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("custom-tag.schema.yaml"),
        "\
schema:
  schema_id: custom-tag
  name: Custom Tag
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: luna
  tag: custom
",
    )
    .expect("custom-tag schema config should be written");
    fs::write(
        staging.join("abc-tags.schema.yaml"),
        "\
schema:
  schema_id: abc-tags
  name: ABC Tags
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: luna
  tag: custom
  tags: [abc]
",
    )
    .expect("abc-tags schema config should be written");
    fs::write(
        staging.join("abc-extra-tags.schema.yaml"),
        "\
schema:
  schema_id: abc-extra-tags
  name: ABC Extra Tags
engine:
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
    - echo_translator
abc_segmentor:
  extra_tags: [custom]
translator:
  dictionary: luna
  tag: custom
",
    )
    .expect("abc-extra-tags schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ba\t爸\t9
ban\t班\t8
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

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);

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
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts
    };

    assert_eq!(candidate_texts_for("custom-tag"), ["b"]);
    assert_eq!(candidate_texts_for("abc-tags"), ["爸", "班", "b"]);
    assert_eq!(candidate_texts_for("abc-extra-tags"), ["爸", "班", "b"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_filter_tag_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-filter-tags");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("blocked.schema.yaml"),
        "\
schema:
  schema_id: blocked
  name: Blocked
engine:
  translators:
    - table_translator
    - echo_translator
  filters:
    - simplifier@zh_simp
translator:
  dictionary: luna
  enable_completion: false
zh_simp:
  option_name: zh_simp
  tags: [custom]
",
    )
    .expect("blocked schema config should be written");
    fs::write(
        staging.join("matched.schema.yaml"),
        "\
schema:
  schema_id: matched
  name: Matched
engine:
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
    - echo_translator
  filters:
    - simplifier@zh_simp
abc_segmentor:
  extra_tags: [custom]
translator:
  dictionary: luna
  enable_completion: false
zh_simp:
  option_name: zh_simp
  tags: [custom]
",
    )
    .expect("matched schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

tw\t臺灣\t9
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

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        let option = CString::new("zh_simp").expect("option name should be valid");
        // SAFETY: option is a valid NUL-terminated string.
        unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };
        for ch in "tw".chars() {
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
                // SAFETY: candidate text pointers are populated by `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts
    };

    assert_eq!(candidate_texts_for("blocked"), ["臺灣", "tw"]);
    assert_eq!(candidate_texts_for("matched"), ["台湾", "tw"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_script_translator_word_completion_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-script-word-completion");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("script.schema.yaml"),
        "\
schema:
  schema_id: script
  name: Script
engine:
  translators:
    - script_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
  enable_word_completion: true
",
    )
    .expect("script schema config should be written");
    fs::write(
        staging.join("r10n.schema.yaml"),
        "\
schema:
  schema_id: r10n
  name: R10n
engine:
  translators:
    - r10n_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: true
  enable_word_completion: false
",
    )
    .expect("r10n schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

ba\t爸\t9
ban\t班\t8
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

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);

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
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts
    };

    assert_eq!(candidate_texts_for("script"), ["爸", "班", "b"]);
    assert_eq!(candidate_texts_for("r10n"), ["b"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_table_translator_sentence_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-table-sentence-options");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("sentence.schema.yaml"),
        "\
schema:
  schema_id: sentence
  name: Sentence
engine:
  translators:
    - table_translator@default_table
    - table_translator@disabled_table
    - table_translator@over_table
    - echo_translator
default_table:
  dictionary: default_dict
  enable_completion: false
disabled_table:
  dictionary: disabled_dict
  enable_completion: false
  enable_sentence: false
over_table:
  dictionary: over_dict
  sentence_over_completion: true
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("default_dict.dict.yaml"),
        "\
---
name: default_dict
version: '0.1'
sort: original
columns: [code, text]
...

ba\t爸
bao\t包
",
    )
    .expect("default dictionary should be written");
    fs::write(
        shared.join("disabled_dict.dict.yaml"),
        "\
---
name: disabled_dict
version: '0.1'
sort: original
columns: [code, text]
...

ca\t擦
cao\t草
",
    )
    .expect("disabled dictionary should be written");
    fs::write(
        shared.join("over_dict.dict.yaml"),
        "\
---
name: over_dict
version: '0.1'
sort: original
columns: [code, text]
...

da\t大
dadan\t大单
",
    )
    .expect("over dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("sentence").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'o' as c_int, 0), TRUE);
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
    let first_text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8");
    let first_comment = unsafe { CStr::from_ptr(candidates[0].comment) }
        .to_str()
        .expect("candidate comment should be valid UTF-8");
    assert_eq!(first_text, "爸包");
    assert_eq!(first_comment, " ☯ ");
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    RimeClearComposition(session_id);
    assert_eq!(RimeProcessKey(session_id, 'c' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'c' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'o' as c_int, 0), TRUE);
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
    assert_eq!(texts, ["cacao"]);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    RimeClearComposition(session_id);
    assert_eq!(RimeProcessKey(session_id, 'd' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'd' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as c_int, 0), TRUE);
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
    assert_eq!(texts, ["大大", "大单", "dada"]);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_initial_quality() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-initial-quality");
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
    - table_translator@low_table
    - table_translator@high_table
    - echo_translator
low_table:
  dictionary: low
  enable_completion: false
high_table:
  dictionary: high
  enable_completion: false
  initial_quality: 10
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("low.dict.yaml"),
        "\
---
name: low
version: '0.1'
sort: original
columns: [code, text]
...

ba\t低
",
    )
    .expect("low dictionary should be written");
    fs::write(
        shared.join("high.dict.yaml"),
        "\
---
name: high
version: '0.1'
sort: original
columns: [code, text]
...

ba\t高
",
    )
    .expect("high dictionary should be written");

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
    for ch in "ba".chars() {
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
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["高", "低", "ba"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_r10n_translator_alias() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-r10n-translator");
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
    - r10n_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
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
columns: [code, text]
...

ni\t你
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
    for ch in "ni".chars() {
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
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["你", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
