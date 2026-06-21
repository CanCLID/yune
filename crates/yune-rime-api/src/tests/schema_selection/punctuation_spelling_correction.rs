#[test]
fn select_schema_loads_librime_punctuator_shape_and_symbol_definitions() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator");
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
    \"/\": [\"、\", \"/\"]
    \"!\": { commit: \"！\" }
    \"(\": { pair: [\"（\", \"）\"] }
  full_shape:
    \"/\": \"／\"
    \"!\": { commit: \"！\" }
    \"(\": { pair: [\"〔\", \"〕\"] }
  symbols:
    \"/\": [\"symbol-slash\"]
    \"/fh\": [\"©\", \"®\"]
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

    let candidate_texts = |input: &str| {
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as i32, 0), TRUE);
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
                // SAFETY: candidate text pointers are populated by
                // `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        RimeClearComposition(session_id);
        texts
    };

    assert_eq!(candidate_texts("/"), ["、", "/", "/"]);
    assert_eq!(candidate_texts("!"), ["！", "!"]);
    assert_eq!(candidate_texts("("), ["（", "）", "("]);
    assert_eq!(candidate_texts("/fh"), ["©", "®", "/fh"]);

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };

    assert_eq!(candidate_texts("/"), ["／", "/"]);
    assert_eq!(candidate_texts("!"), ["！", "!"]);
    assert_eq!(candidate_texts("("), ["〔", "〕", "("]);
    assert_eq!(candidate_texts("/fh"), ["©", "®", "/fh"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: yune-core spelling_algebra.rs and translator/mod.rs; librime oracle: schema-loaded speller/algebra generated spellings and candidate penalty ordering from Phase 03 distribution comparison.
#[test]
fn schema_spelling_algebra_generated_spellings_match_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-spelling-algebra-generated-spellings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("algebra.schema.yaml"),
        "\
schema:
  schema_id: algebra
  name: Algebra
engine:
  translators:
    - table_translator
    - echo_translator
speller:
  algebra:
    - xform/^lue$/lve/
    - derive/^nv$/nu/
    - fuzz/^bing$/pin/
    - abbrev/^chang$/c/
    - derive/^cuo$/cu/correction
    - erase/^gone$/
    - xlit/zyx/abc/
translator:
  dictionary: algebra
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("algebra.dict.yaml"),
        "\
---
name: algebra
version: '0.1'
sort: original
columns: [code, text, weight]
...

lue	略	0
nv	女	0
bing	病	0
pin	平	0
chang	长	0
cuo	错	0
zyx	照	0
gone	删	0
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
    let schema_id = CString::new("algebra").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let candidate_pairs_for = |input: &str| {
        RimeClearComposition(session_id);
        for ch in input.chars() {
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
        candidate_pairs_for("lve"),
        [
            ("略".to_owned(), "lue".to_owned()),
            ("lve".to_owned(), "echo".to_owned())
        ]
    );
    assert_eq!(
        candidate_pairs_for("nu"),
        [
            ("女".to_owned(), "nv".to_owned()),
            ("nu".to_owned(), "echo".to_owned())
        ]
    );
    assert_eq!(
        candidate_pairs_for("abc"),
        [
            ("照".to_owned(), "zyx".to_owned()),
            ("abc".to_owned(), "echo".to_owned())
        ]
    );
    assert_eq!(
        candidate_pairs_for("gone"),
        [("gone".to_owned(), "echo".to_owned())]
    );
    assert_eq!(
        candidate_pairs_for("pin"),
        [
            ("平".to_owned(), "pin".to_owned()),
            ("病".to_owned(), "bing".to_owned()),
            ("pin".to_owned(), "echo".to_owned())
        ]
    );
    assert_eq!(
        candidate_pairs_for("c"),
        [
            ("长".to_owned(), "chang".to_owned()),
            ("c".to_owned(), "echo".to_owned())
        ]
    );
    assert_eq!(
        candidate_pairs_for("cu"),
        [
            ("错".to_owned(), "cuo".to_owned()),
            ("cu".to_owned(), "echo".to_owned())
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: schema_install.rs and yune-core spelling_algebra.rs; fork oracle:
// TypeDuck v1.1.2 jyut6ping3_mobile with the real 127k-row jyut6ping3_scolar
// dictionary keeps Cantonese ng->m fuzzy algebra for single-letter `m`.
#[test]
fn schema_large_cantonese_dictionary_keeps_fork_fuzzy_algebra() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-large-cantonese-fuzzy-algebra");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("cantonese_large.schema.yaml"),
        "\
schema:
  schema_id: cantonese_large
  name: Cantonese Large
engine:
  translators:
    - table_translator
    - echo_translator
speller:
  algebra:
    - derive/^ng(?=\\d)/m/
    - derive/\\d//
translator:
  dictionary: cantonese_large
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");

    let mut dictionary = "\
---
name: cantonese_large
version: '0.1'
sort: original
columns: [code, text, weight]
...

"
    .to_owned();
    dictionary.push_str("m4\t\u{5514}\t1\n");
    dictionary.push_str("ng5\t\u{4e94}\t1\n");
    for index in 0..50_000 {
        dictionary.push_str(&format!("zz{index}\tFILLER{index}\t0\n"));
    }
    fs::write(shared.join("cantonese_large.dict.yaml"), dictionary)
        .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("cantonese_large").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'm' as c_int, 0), TRUE);
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let candidate_pairs = candidates
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

    assert!(
        candidate_pairs.contains(&("\u{5514}".to_owned(), "m4".to_owned())),
        "large dictionary should keep the directly keyed real Cantonese row: {candidate_pairs:?}"
    );
    assert!(
        candidate_pairs.contains(&("\u{4e94}".to_owned(), "ng5".to_owned())),
        "large dictionary should keep fork ng->m fuzzy algebra for real Cantonese rows: {candidate_pairs:?}"
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: yune-core spelling_algebra.rs and translator/mod.rs; librime oracle: YAML-backed correction penalties participate in lookup ranking without compiled prism/table/reverse payloads.
#[test]
fn schema_tolerance_lookup_yaml_backed_ranking_matches_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-tolerance-yaml-backed-ranking");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("tolerance.schema.yaml"),
        "\
schema:
  schema_id: tolerance
  name: Tolerance
engine:
  translators:
    - table_translator
    - echo_translator
speller:
  algebra:
    - derive/^cuo$/cu/correction
    - fuzz/^bing$/pin/
translator:
  dictionary: tolerance
  enable_completion: false
  enable_sentence: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("tolerance.dict.yaml"),
        "\
---
name: tolerance
version: '0.1'
sort: original
columns: [code, text, weight]
...

cuo	错	0
cu	粗	0
bing	病	0
pin	平	0
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
    let schema_id = CString::new("tolerance").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    for ch in "cu".chars() {
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
    let ranked = candidates
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
        ranked,
        [
            ("粗".to_owned(), "cu".to_owned()),
            ("错".to_owned(), "cuo".to_owned()),
            ("cu".to_owned(), "echo".to_owned())
        ]
    );

    RimeClearComposition(session_id);
    for ch in "pin".chars() {
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
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(texts, ["平".to_owned(), "病".to_owned(), "pin".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: schema_install.rs and yune-core translator/mod.rs; TypeDuck v1.1.2 oracle: translator/enable_correction is independent of enable_completion and gates dictionary correction lookup.
#[test]
fn schema_dictionary_correction_gate_is_independent_of_completion() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-dictionary-correction-gate");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    for (schema_id, enable_completion, enable_correction) in [
        ("correction_disabled", false, false),
        ("correction_enabled", true, true),
    ] {
        fs::write(
            staging.join(format!("{schema_id}.schema.yaml")),
            format!(
                "\
schema:
  schema_id: {schema_id}
  name: {schema_id}
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: correction
  enable_completion: {enable_completion}
  enable_sentence: false
  enable_correction: {enable_correction}
"
            ),
        )
        .expect("schema config should be written");
    }
    fs::write(
        shared.join("correction.dict.yaml"),
        "\
---
name: correction
version: '0.1'
sort: by_weight
correction: [bq=>ba]
...

八\tba\t2
爸\tba\t1
把\tbaa\t100
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
    let schema_id = CString::new("correction_disabled").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "bq".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    assert_eq!(
        current_candidate_pairs(session_id),
        [("bq".to_owned(), "echo".to_owned())],
        "correction dictionary entries should not leak when enable_correction is false"
    );

    RimeClearComposition(session_id);
    let schema_id = CString::new("correction_enabled").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "bq".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    let ranked = current_candidate_pairs(session_id);
    assert_eq!(
        ranked[..2],
        [
            ("八".to_owned(), "ba".to_owned()),
            ("爸".to_owned(), "ba".to_owned())
        ],
        "enable_correction should recover dictionary corrections independently of completion"
    );
    assert!(
        ranked.iter().all(|(text, _)| text != "把"),
        "corrected lookup should not expand completion rows from the canonical spelling"
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: schema_install.rs and context_api.rs; TypeDuck v1.1.2 oracle: include:/letter_to_tone maps v/x/q tone letters to numeric Jyutping in preedit only.
#[test]
fn schema_translator_preedit_format_applies_letter_to_tone_without_changing_input() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-preedit-format-letter-to-tone");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("jyut.schema.yaml"),
        "\
schema:
  schema_id: jyut
  name: Jyut
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: jyut
  enable_completion: false
  enable_sentence: false
  preedit_format:
    - xform/([aeiouymngptk])vv/${1}4/
    - xform/([aeiouymngptk])xx/${1}5/
    - xform/([aeiouymngptk])qq/${1}6/
    - xform/([aeiouymngptk])v/${1}1/
    - xform/([aeiouymngptk])x/${1}2/
    - xform/([aeiouymngptk])q/${1}3/
speller:
  algebra:
    - xform/4/vv/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("jyut.dict.yaml"),
        "\
---
name: jyut
version: '0.1'
sort: by_weight
...

tone-four\tnei4\t1
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
    let schema_id = CString::new("jyut").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "neivv".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    let input_ptr = RimeGetInput(session_id);
    assert!(!input_ptr.is_null());
    let raw_input = unsafe { CStr::from_ptr(input_ptr) }
        .to_str()
        .expect("raw input should be UTF-8");
    assert_eq!(raw_input, "neivv");

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_str()
        .expect("preedit should be valid UTF-8")
        .to_owned();
    assert_eq!(context.composition.length, 4);
    assert_eq!(context.composition.cursor_pos, 4);
    assert_eq!(context.composition.sel_end, 4);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(preedit, "nei4");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: schema_install.rs and context_api.rs; TypeDuck v1.1.2 oracle: partial `q` input keeps raw preedit even when completion candidates exist.
#[test]
fn schema_translator_preedit_format_leaves_partial_letter_tone_completion_raw() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-preedit-format-partial-letter-tone");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("jyut.schema.yaml"),
        "\
schema:
  schema_id: jyut
  name: Jyut
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: jyut
  enable_sentence: false
  preedit_format:
    - xform/([aeiouymngptk])qq/${1}6/
    - xform/([aeiouymngptk])q/${1}3/
speller:
  algebra:
    - xform/6/qq/
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("jyut.dict.yaml"),
        "\
---
name: jyut
version: '0.1'
sort: by_weight
...

tone-six\tnei6\t1
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
    let schema_id = CString::new("jyut").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "neiq".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let preedit = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_str()
        .expect("preedit should be valid UTF-8")
        .to_owned();
    assert!(
        context.menu.num_candidates > 0,
        "partial tone-letter input should still expose candidates"
    );
    assert_eq!(context.composition.length, 4);
    assert_eq!(context.composition.cursor_pos, 4);
    assert_eq!(context.composition.sel_end, 4);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(preedit, "neiq");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
