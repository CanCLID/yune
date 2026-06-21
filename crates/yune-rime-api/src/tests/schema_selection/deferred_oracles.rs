// Owner: schema_install.rs and yune-core filter/mod.rs; librime oracle: simplifier filter-chain tag gating and limited built-in OpenCC config behavior from Phase 03 comparison.
#[test]
fn schema_opencc_filter_chain_integration_matches_librime_limited_maps() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-opencc-filter-chain-limited-maps");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("opencc.schema.yaml"),
        "\
schema:
  schema_id: opencc
  name: OpenCC Limited Maps
switches:
  - name: zh_simp
    reset: 1
engine:
  segmentors:
    - abc_segmentor
    - affix_segmentor@reverse_lookup
  translators:
    - table_translator
    - reverse_lookup_translator
    - echo_translator
  filters:
    - simplifier@zh_simp
abc_segmentor:
  extra_tags: [simplify]
translator:
  dictionary: opencc
  enable_completion: false
  enable_sentence: false
  tags: [simplify]
reverse_lookup:
  dictionary: reverse
  prefix: '`'
  tag: reverse_lookup
zh_simp:
  option_name: zh_simp
  opencc_config: t2s.json
  tips: all
  tags: [simplify]
  comment_format:
    - xform/^/〔/
    - xform/$/〕/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("opencc.dict.yaml"),
        "\
---
name: opencc
version: '0.1'
sort: original
columns: [code, text]
...

tw	臺灣
ma	龍馬
",
    )
    .expect("dictionary should be written");
    fs::write(
        shared.join("reverse.dict.yaml"),
        "\
---
name: reverse
version: '0.1'
sort: original
columns: [code, text]
...

ma	龍馬
",
    )
    .expect("reverse dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("opencc").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    let abc_pairs = {
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
        abc_pairs,
        [
            ("台湾".to_owned(), "〔臺灣〕".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );

    RimeClearComposition(session_id);
    for ch in "`ma".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    let reverse_lookup_pairs = {
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
        reverse_lookup_pairs,
        [
            ("龍馬".to_owned(), "ma".to_owned()),
            ("`ma".to_owned(), "echo".to_owned())
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_selection_recognizes_memory_or_defers_learning() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-memory-deferral");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("memory.schema.yaml"),
        "\
schema:
  schema_id: memory
  name: Memory Deferral
engine:
  translators:
    - table_translator
    - memory
    - history_translator
    - echo_translator
translator:
  dictionary: memory
history:
  input: zz
  size: 1
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("memory.dict.yaml"),
        "\
---
name: memory
version: '0.1'
sort: original
columns: [code, text]
...

ni\t你
ni\t尼
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
    let schema_id = CString::new("memory").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let deferrals = remaining_gear_deferrals_snapshot(session_id)
        .expect("session deferrals should be visible to tests");
    assert_eq!(deferrals.len(), 1);
    assert_eq!(deferrals[0].gear, "memory");
    assert_eq!(
        deferrals[0].observed_librime_role,
        "user dictionary memory and learning"
    );
    assert_eq!(
        deferrals[0].current_yune_behavior,
        "recognized during schema installation as a deterministic no-op"
    );
    assert_eq!(
        deferrals[0].scope_decision,
        "deferred because LevelDB/userdb learning is outside Phase 3"
    );
    assert_eq!(deferrals[0].target_phase, "05-userdb-and-learning");

    for ch in "ni".chars() {
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
    assert_eq!(texts, ["你".to_owned(), "尼".to_owned(), "ni".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_selection_defers_poet_grammar_contextual_translation() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-poet-grammar-contextual-deferral");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("grammar.schema.yaml"),
        "\
schema:
  schema_id: grammar
  name: Grammar Deferral
engine:
  translators:
    - table_translator
    - echo_translator
  filters:
    - poet
    - grammar
    - contextual_translation
translator:
  dictionary: grammar
grammar:
  language: wanxiang-lts-zh-hans
poet:
  vocab: poem
contextual_translation:
  max_homophones: 3
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("grammar.dict.yaml"),
        "\
---
name: grammar
version: '0.1'
sort: original
columns: [code, text]
...

shi\t是
shi\t时
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
    let schema_id = CString::new("grammar").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let deferrals = remaining_gear_deferrals_snapshot(session_id)
        .expect("session deferrals should be visible to tests");
    let gears = deferrals
        .iter()
        .map(|deferral| deferral.gear.as_str())
        .collect::<Vec<_>>();
    assert_eq!(gears, ["poet", "grammar", "contextual_translation"]);
    assert!(deferrals
        .iter()
        .all(|deferral| deferral.target_phase == "04-compiled-dictionary-data"));
    assert!(deferrals
        .iter()
        .all(|deferral| deferral.current_yune_behavior
            == "recognized during schema installation as a deterministic no-op"));
    assert!(deferrals.iter().any(|deferral| {
        deferral.gear == "poet" && deferral.scope_decision.contains("plugin/model behavior")
    }));
    assert!(deferrals.iter().any(|deferral| {
        deferral.gear == "grammar" && deferral.scope_decision.contains("plugin/model behavior")
    }));
    assert!(deferrals.iter().any(|deferral| {
        deferral.gear == "contextual_translation"
            && deferral
                .scope_decision
                .contains("compiled reverse/context data")
    }));

    for ch in "shi".chars() {
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
    assert_eq!(texts, ["是".to_owned(), "时".to_owned(), "shi".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_selection_defers_unity_table_encoder_payloads() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-unity-table-encoder-deferral");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("unity.schema.yaml"),
        "\
schema:
  schema_id: unity
  name: Unity Table Deferral
engine:
  translators:
    - table_translator
    - unity_table_encoder
    - echo_translator
translator:
  dictionary: unity
unity_table_encoder:
  dictionary: unity
  enable_encoder: true
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("unity.dict.yaml"),
        "\
---
name: unity
version: '0.1'
sort: original
columns: [code, text]
...

ma\t马
ma\t吗
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
    let schema_id = CString::new("unity").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let deferrals = remaining_gear_deferrals_snapshot(session_id)
        .expect("session deferrals should be visible to tests");
    assert_eq!(deferrals.len(), 1);
    assert_eq!(deferrals[0].gear, "unity_table_encoder");
    assert_eq!(
        deferrals[0].observed_librime_role,
        "encodes phrases into UniTE table data"
    );
    assert_eq!(
        deferrals[0].current_yune_behavior,
        "recognized during schema installation as a deterministic no-op"
    );
    assert_eq!(
        deferrals[0].scope_decision,
        "deferred because compiled UniTE/table payload support is outside Phase 3"
    );
    assert_eq!(deferrals[0].target_phase, "04-compiled-dictionary-data");

    for ch in "ma".chars() {
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
    assert_eq!(texts, ["马".to_owned(), "吗".to_owned(), "ma".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
