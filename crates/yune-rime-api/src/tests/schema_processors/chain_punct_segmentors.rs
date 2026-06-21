// Owner: processors/editor.rs, processors/navigator.rs, processors/selector.rs; librime oracle: schema-loaded composition span and candidate selection state.
#[test]
fn schema_editor_navigator_selector_spans_match_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-editor-navigator-selector-spans");
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
menu:
  page_size: 2
engine:
  processors:
    - speller
    - selector
    - navigator
    - fluid_editor
  segmentors:
    - abc_segmentor
  translators:
    - table_translator
speller:
  alphabet: abc
  delimiter: ' '
translator:
  dictionary: luna
  enable_completion: false
  enable_sentence: false
selector:
  bindings:
    Down: next_candidate
    Up: previous_candidate
navigator:
  bindings:
    Left: left_by_syllable
    Right: right_by_syllable
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

甲\ta\t100
乙\ta\t90
丙\ta\t80
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

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    let down = CString::new("Down").expect("key name should be valid");
    let left = CString::new("Left").expect("key name should be valid");
    // SAFETY: key names are valid NUL-terminated strings.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    let left_keycode = unsafe { RimeGetKeycodeByName(left.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, left_keycode, 0), TRUE);

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 1);
    assert_eq!(context.composition.cursor_pos, 0);
    assert_eq!(context.composition.sel_start, 0);
    assert_eq!(context.composition.sel_end, 1);
    assert_eq!(context.menu.num_candidates, 2);
    assert_eq!(context.menu.page_no, 0);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
        Ok("乙")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    let mut no_commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut no_commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: processors/chord_composer.rs, processors/shape.rs, processors/punctuation.rs, schema_install.rs; librime oracle: chain punctuation/fallback ordering and cleanup state.
#[test]
fn schema_chord_shape_punctuation_fallback_chain_matches_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-chord-shape-punctuation-fallback-chain");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("chain.schema.yaml"),
        "\
schema:
  schema_id: chain
  name: Chain
engine:
  processors:
    - chord_composer
  segmentors:
    - punct_segmentor
    - fallback_segmentor
  translators:
    - table_translator
    - punct_translator
    - echo_translator
chord_composer:
  alphabet: ab
  output_format:
    - xlit/ab/xy/
  bindings:
    Control+r: commit_raw_input
translator:
  dictionary: chain
  enable_completion: false
  enable_sentence: false
  initial_quality: 3.0
punctuator:
  half_shape:
    '.': '。'
  full_shape:
    '.': '．'
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("chain.dict.yaml"),
        "\
---
name: chain
version: '0.1'
sort: original
...

形\txy\t100
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
    let schema_id = CString::new("chain").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 2);
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
    assert_eq!(texts, ["。".to_owned(), ".".to_owned()]);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    RimeClearComposition(session_id);

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'a' as i32, K_RELEASE_MASK), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'b' as i32, K_RELEASE_MASK), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.num_candidates, 1);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].text) }.to_str(),
        Ok("xy")
    );
    assert!(!candidates[0].comment.is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[0].comment) }.to_str(),
        Ok("echo")
    );
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    RimeClearComposition(session_id);
    let input = RimeGetInput(session_id);
    assert!(!input.is_null());
    // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok(""));
    assert_eq!(
        RimeProcessKey(session_id, 'r' as i32, K_CONTROL_MASK),
        FALSE
    );

    let mut status = empty_status();
    // SAFETY: status points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, FALSE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punctuator_candidates_expose_librime_shape_comments() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-comments");
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
    - punct_translator
    - echo_translator
punctuator:
  half_shape:
    \"/\": [\"/\", \"、\", \"©\"]
  full_shape:
    \"/\": \"／\"
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
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let candidate_comments = || {
        assert_eq!(RimeProcessKey(session_id, '/' as i32, 0), TRUE);
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
                    None
                } else {
                    Some(
                        // SAFETY: non-null candidate comment pointers are
                        // populated by `RimeGetContext`.
                        unsafe { CStr::from_ptr(candidate.comment) }
                            .to_str()
                            .expect("candidate comment should be valid UTF-8")
                            .to_owned(),
                    )
                }
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        RimeClearComposition(session_id);
        comments
    };

    assert_eq!(
        candidate_comments(),
        [
            Some("〔半角〕".to_owned()),
            Some("〔全角〕".to_owned()),
            None,
            Some("echo".to_owned())
        ]
    );

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    assert_eq!(
        candidate_comments(),
        [Some("〔全角〕".to_owned()), Some("echo".to_owned())]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punct_segmentor_tags_punctuation_exclusively() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punct-segmentor-exclusive");
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
    - punct_segmentor
  translators:
    - punct_translator
    - table_translator
    - echo_translator
translator:
  dictionary: luna
punctuator:
  half_shape:
    \".\": \"。\"
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

DOT\t.\t100
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

    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
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

    assert_eq!(texts, ["。".to_owned(), ".".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_punct_segmentor_translates_digit_separator_as_number_punctuation() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punct-segmentor-number");
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
    - punct_segmentor
  translators:
    - punct_translator
    - echo_translator
punctuator:
  digit_separators: \".:\"
  half_shape:
    \".\": \"。\"
  full_shape:
    \".\": \"。\"
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
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let top_candidate = || {
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
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        text
    };

    assert_eq!(RimeProcessKey(session_id, '1' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(top_candidate(), ".");

    RimeClearComposition(session_id);
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, '.' as i32, 0), TRUE);
    assert_eq!(top_candidate(), "．");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn schema_fallback_segmentor_tags_unclaimed_input_as_raw() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-fallback-segmentor-raw");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("raw.schema.yaml"),
        "\
schema:
  schema_id: raw
  name: Raw
engine:
  segmentors:
    - fallback_segmentor
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: raw
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("raw.dict.yaml"),
        "\
---
name: raw
version: '0.1'
sort: original
...

Alpha\ta\t100
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
    let schema_id = CString::new("raw").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
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

    assert_eq!(texts, ["a".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
