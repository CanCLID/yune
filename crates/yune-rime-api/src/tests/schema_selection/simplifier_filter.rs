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
