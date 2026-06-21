#[test]
fn select_schema_applies_librime_translator_comment_format() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-comment-format");
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
translator:
  dictionary: luna
  comment_format:
    - xlit/ab/AB/
    - xform/^/[/
    - xform/$/]/
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

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
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
    let comments = candidates
        .iter()
        .map(|candidate| {
            if candidate.comment.is_null() {
                return String::new();
            }
            // SAFETY: candidate comment pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.comment) }
                .to_str()
                .expect("candidate comment should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(comments, ["[BA]", "[BAn]", "echo"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_dictionary_exclude() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-dictionary-exclude");
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
translator:
  dictionary: luna
  dictionary_exclude:
    - 爸
    - 班
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

ba\t爸\t9
ban\t班\t8
bao\t包\t7
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

    assert_eq!(texts, ["包", "b"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_delimiter_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-delimiter");
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
speller:
  delimiter: \"'\"
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

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "ba'".chars() {
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
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_str()
        .expect("preedit should be valid UTF-8")
        .to_owned();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    assert_eq!(texts, ["爸", "ba'"]);
    assert_eq!(preedit, "ba'");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
