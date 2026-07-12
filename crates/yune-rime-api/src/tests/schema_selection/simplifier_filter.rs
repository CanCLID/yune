#[test]
fn select_schema_loads_librime_simplifier_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-simplifier-filter");
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
    - table_translator
    - echo_translator
  filters:
    - simplifier@zh_simp
translator:
  dictionary: luna
zh_simp:
  option_name: zh_simp
  tips: all
  comment_format:
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
sort: by_weight
columns: [code, text, weight]
...

tw\t臺灣\t9
tw\t龍馬\t8
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
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let candidate_pairs = || {
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
                let text = unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned();
                let comment = if candidate.comment.is_null() {
                    String::new()
                } else {
                    // SAFETY: candidate comment pointers are populated by `RimeGetContext`.
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
        texts
    };

    assert_eq!(
        candidate_pairs(),
        [
            ("臺灣".to_owned(), "tw".to_owned()),
            ("龍馬".to_owned(), "tw".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );

    let option = CString::new("zh_simp").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    assert_eq!(
        candidate_pairs(),
        [
            ("台湾".to_owned(), "〔臺灣〕".to_owned()),
            ("龙马".to_owned(), "〔龍馬〕".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_treats_librime_simplifier_filter_namespace_as_simplifier() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-simplifier-filter-namespace");
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
    - table_translator
  filters:
    - simplifier@filter
translator:
  dictionary: luna
simplifier:
  option_name: zh_simp
  tips: all
",
    )
    .expect("schema config should be written");
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

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let option = CString::new("zh_simp").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

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
    let text = unsafe { CStr::from_ptr(candidates[0].text) }
        .to_str()
        .expect("candidate text should be valid UTF-8")
        .to_owned();
    let comment = unsafe { CStr::from_ptr(candidates[0].comment) }
        .to_str()
        .expect("candidate comment should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(text, "台湾");
    assert_eq!(comment, "〔臺灣〕");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_simplifier_opencc_config() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-simplifier-opencc-config");
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
    - table_translator
    - echo_translator
  filters:
    - simplifier@zh_tw
translator:
  dictionary: luna
zh_tw:
  option_name: zh_tw
  opencc_config: t2tw.json
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

tw\t台灣\t9
tw\t裏\t8
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
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let candidate_texts = || {
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
        texts
    };

    assert_eq!(candidate_texts(), ["台灣", "裏", "tw"]);

    let option = CString::new("zh_tw").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    assert_eq!(candidate_texts(), ["臺灣", "裡", "tw"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_upstream_output_standard_simplifiers() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-output-standard-simplifiers");
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
switches:
  - options: [ noop, variants_hk, trad_tw, simplification ]
    states: [ 傳統漢字, 香港傳統漢字, 臺灣傳統漢字, 大陆简化汉字 ]
    reset: 1
engine:
  translators:
    - table_translator
    - echo_translator
  filters:
    - simplifier
    - simplifier@variants_hk
    - simplifier@trad_tw
translator:
  dictionary: luna
simplifier:
  option_name: simplification
  opencc_config: t2s.json
variants_hk:
  option_name: variants_hk
  opencc_config: t2hkf.json
trad_tw:
  option_name: trad_tw
  opencc_config: t2tw.json
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---
name: luna
version: '0.1'
sort: by_weight
columns: [code, text, weight]
...

std\t檯台灣個\t9
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
    for ch in "std".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let candidate_texts = || {
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
        texts
    };

    let variants_hk = CString::new("variants_hk").expect("option name should be valid");
    let trad_tw = CString::new("trad_tw").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");

    assert_eq!(candidate_texts(), ["枱台灣個", "std"]);

    // SAFETY: option names are valid NUL-terminated strings.
    unsafe {
        RimeSetOption(session_id, variants_hk.as_ptr(), FALSE);
        RimeSetOption(session_id, trad_tw.as_ptr(), TRUE);
    }
    assert_eq!(candidate_texts(), ["檯臺灣個", "std"]);

    // SAFETY: option names are valid NUL-terminated strings.
    unsafe {
        RimeSetOption(session_id, trad_tw.as_ptr(), FALSE);
        RimeSetOption(session_id, simplification.as_ptr(), TRUE);
    }
    assert_eq!(candidate_texts(), ["台台湾个", "std"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_simplifier_excluded_types() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-simplifier-excluded-types");
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
    - table_translator
    - echo_translator
  filters:
    - simplifier@zh_simp
translator:
  dictionary: luna
zh_simp:
  option_name: zh_simp
  tips: all
  excluded_types:
    - table
",
    )
    .expect("schema config should be written");
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
tw\t龍馬\t8
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
    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let option = CString::new("zh_simp").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

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

    assert_eq!(
        pairs,
        [
            ("臺灣".to_owned(), "tw".to_owned()),
            ("龍馬".to_owned(), "tw".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_matches_librime_opencc_convert_word_for_source_and_compiled_tables() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-opencc-convert-word");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");

    let dictionary_yaml = |name: &str| {
        format!(
            "---\nname: {name}\nversion: '0.1'\nsort: original\ncolumns: [text, code, weight]\n...\n\n祕\ta\t10\n秘\ta\t9\n糉\tb\t8\n祕糉\tc\t7\n只\td\t6\n甲乙\te\t5\n"
        )
    };
    let schema_yaml = |schema_id: &str, dictionary_id: &str| {
        format!(
            "schema:\n  schema_id: {schema_id}\n  name: {schema_id}\nswitches:\n  - name: variants_hk\n    reset: 1\nmenu:\n  page_size: 10\nengine:\n  translators:\n    - table_translator\n  filters:\n    - simplifier@variants_hk\n    - uniquifier\ntranslator:\n  dictionary: {dictionary_id}\n  enable_completion: false\n  enable_sentence: false\nvariants_hk:\n  option_name: variants_hk\n  opencc_config: t2hkf.json\n"
        )
    };

    fs::write(
        staging.join("opencc_source.schema.yaml"),
        schema_yaml("opencc_source", "opencc_source"),
    )
    .expect("source schema should be written");
    fs::write(
        shared.join("opencc_source.dict.yaml"),
        dictionary_yaml("opencc_source"),
    )
    .expect("source dictionary should be written");

    fs::write(
        staging.join("opencc_compiled.schema.yaml"),
        schema_yaml("opencc_compiled", "opencc_compiled"),
    )
    .expect("compiled schema should be written");
    let compiled_dictionary = yune_core::TableDictionary::parse_rime_dict_yaml(
        &dictionary_yaml("opencc_compiled"),
    )
    .expect("compiled dictionary source should parse");
    let compiled_checksum = 0x5904_c001;
    fs::write(
        shared.join("opencc_compiled.table.bin"),
        yune_core::build_table_bin(&compiled_dictionary, compiled_checksum),
    )
    .expect("compiled table should be written");
    fs::write(
        shared.join("opencc_compiled.prism.bin"),
        yune_core::build_prism_bin(
            &[
                "a".to_owned(),
                "b".to_owned(),
                "c".to_owned(),
                "d".to_owned(),
                "e".to_owned(),
            ],
            &[],
            compiled_checksum,
            0,
        ),
    )
    .expect("compiled prism should be written");
    fs::write(
        shared.join("opencc_compiled.reverse.bin"),
        yune_core::build_reverse_bin(&compiled_dictionary, compiled_checksum),
    )
    .expect("compiled reverse should be written");
    assert!(
        !shared.join("opencc_compiled.dict.yaml").exists(),
        "compiled-path coverage must not silently fall back to source YAML"
    );

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let oracle_fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../yune-core/tests/fixtures/upstream-1.17.0/m59-opencc-convert-word.json"
    )))
    .expect("pinned librime OpenCC fixture should parse");
    let cases = oracle_fixture["cases"]
        .as_array()
        .expect("oracle fixture should contain cases")
        .iter()
        .map(|case| {
            let input = case["input"]
                .as_str()
                .expect("oracle input should be a string")
                .to_owned();
            let expected = case["all_candidates"]
                .as_array()
                .expect("oracle case should contain all-page candidates")
                .iter()
                .map(|candidate| {
                    candidate["text"]
                        .as_str()
                        .expect("oracle candidate text should be a string")
                        .to_owned()
                })
                .collect::<Vec<_>>();
            (input, expected)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        cases.iter().map(|(input, _)| input.as_str()).collect::<Vec<_>>(),
        ["a", "b", "c", "d", "e"],
        "the ABI test must execute every captured upstream case"
    );
    for schema in ["opencc_source", "opencc_compiled"] {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        let variants_hk = CString::new("variants_hk").expect("option name should be valid");
        // SAFETY: option name is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeGetOption(session_id, variants_hk.as_ptr()) },
            TRUE,
            "{schema} must activate variants_hk through its declared reset"
        );

        for (input, expected) in &cases {
            RimeClearComposition(session_id);
            for ch in input.chars() {
                assert_eq!(
                    RimeProcessKey(session_id, ch as c_int, 0),
                    TRUE,
                    "{schema} should process {input}"
                );
            }
            let actual = current_candidate_pairs(session_id)
                .into_iter()
                .map(|(text, _)| text)
                .collect::<Vec<_>>();
            assert_eq!(
                actual,
                *expected,
                "ordered ConvertWord candidates for {schema}/{input}"
            );
        }

        if schema == "opencc_compiled" {
            let diagnostics = crate::session_web_diagnostics_snapshot(session_id)
                .expect("compiled session diagnostics should exist");
            let storage = diagnostics
                .storage
                .iter()
                .find(|row| row.owner == "compact_table.storage")
                .expect("compiled schema should use compact table storage");
            assert_eq!(storage.selected_storage, "byte_backed");
            assert!(
                !remaining_gear_deferrals_snapshot(session_id)
                    .expect("compiled session should exist")
                    .iter()
                    .any(|deferral| deferral.gear == "dictionary_source_fallback"),
                "compiled-path coverage must not pass through source fallback"
            );
        }

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
