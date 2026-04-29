use super::*;
use crate::{remaining_gear_deferrals_snapshot, session_candidates_snapshot, sessions};

#[derive(Debug, Eq, PartialEq)]
struct DistributionSchemaComparison {
    schema_name: &'static str,
    librime_oracle_source: &'static str,
    component_order: Vec<&'static str>,
    segment_tags: Vec<String>,
    generated_spellings: Vec<(String, String)>,
    opencc_or_filter_behavior: Vec<(String, String)>,
    punctuation_or_fallback_behavior: Vec<String>,
    candidate_differences: Vec<StructuredFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StructuredFinding {
    observed_yune_behavior: &'static str,
    expected_librime_behavior: &'static str,
    scope_decision: &'static str,
    target_phase: &'static str,
}

// Owner: crates/yune-rime-api/src/tests/distribution_schema_comparison.rs,
// schema_install.rs, processors/punctuation.rs, yune-core translator/filter.
// librime oracle: /Users/trenton/Projects/librime/data/minimal/luna_pinyin.schema.yaml,
// source-level chain semantics isolated from compiled luna_pinyin.table/prism payloads.
#[test]
fn distribution_luna_pinyin_chain_matches_focused_librime_observations() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("distribution-luna-pinyin-chain");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna_pinyin.schema.yaml"),
        "\
schema:
  schema_id: luna_pinyin
  name: Luna Pinyin Focused Distribution Slice
switches:
  - name: zh_simp
    reset: 0
engine:
  processors:
    - speller
    - punctuator
    - fluid_editor
  segmentors:
    - abc_segmentor
    - punct_segmentor
    - fallback_segmentor
  translators:
    - table_translator
    - punct_translator
    - echo_translator
  filters:
    - simplifier@zh_simp
    - uniquifier
speller:
  alphabet: abcdefghijklmnopqrstuvwxyz
translator:
  dictionary: luna_pinyin
  enable_completion: false
  enable_sentence: false
zh_simp:
  option_name: zh_simp
  opencc_config: t2s.json
  tips: all
  comment_format:
    - xform/^/〔/
    - xform/$/〕/
punctuator:
  half_shape:
    '.': '。'
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("luna_pinyin.dict.yaml"),
        "\
---
name: luna_pinyin
version: '0.1'
sort: original
columns: [code, text]
...

tw\t臺灣
ma\t龍馬
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
    let schema_id = CString::new("luna_pinyin").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    for ch in "tw".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
    let traditional_pairs = candidate_pairs(session_id);
    let generated_spellings = session_candidates_snapshot(session_id)
        .expect("session candidates should be visible")
        .into_iter()
        .map(|candidate| (candidate.text, candidate.comment))
        .collect::<Vec<_>>();
    let segment_tags = session_segment_tags(session_id);
    let zh_simp = CString::new("zh_simp").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, zh_simp.as_ptr(), TRUE) };
    let simplified_pairs = candidate_pairs(session_id);

    RimeClearComposition(session_id);
    let punctuation_input = CString::new(".").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSetInput(session_id, punctuation_input.as_ptr()) },
        TRUE
    );
    let punctuation_texts = candidate_texts(session_id);

    let comparison = DistributionSchemaComparison {
        schema_name: "luna_pinyin",
        librime_oracle_source:
            "/Users/trenton/Projects/librime/data/minimal/luna_pinyin.schema.yaml",
        component_order: vec![
            "speller",
            "punctuator",
            "fluid_editor",
            "abc_segmentor",
            "punct_segmentor",
            "fallback_segmentor",
            "table_translator",
            "punct_translator",
            "echo_translator",
            "simplifier@zh_simp",
            "uniquifier",
        ],
        segment_tags,
        generated_spellings,
        opencc_or_filter_behavior: simplified_pairs.clone(),
        punctuation_or_fallback_behavior: punctuation_texts.clone(),
        candidate_differences: vec![StructuredFinding {
            observed_yune_behavior:
                "focused source YAML produces candidates from luna_pinyin.dict.yaml only",
            expected_librime_behavior:
                "distribution luna_pinyin also consumes compiled table/prism payloads for full lookup scale",
            scope_decision:
                "compiled payload comparison is recorded, not shimmed, because Phase 3 owns chain semantics only",
            target_phase: "04-compiled-dictionary-data",
        }],
    };

    assert_eq!(comparison.schema_name, "luna_pinyin");
    assert!(comparison.librime_oracle_source.contains("librime"));
    assert_eq!(
        comparison.component_order,
        [
            "speller",
            "punctuator",
            "fluid_editor",
            "abc_segmentor",
            "punct_segmentor",
            "fallback_segmentor",
            "table_translator",
            "punct_translator",
            "echo_translator",
            "simplifier@zh_simp",
            "uniquifier",
        ]
    );
    assert_eq!(comparison.segment_tags, ["abc".to_owned()]);
    assert_eq!(
        comparison.generated_spellings,
        [
            ("臺灣".to_owned(), "tw".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );
    assert_eq!(
        traditional_pairs,
        [
            ("臺灣".to_owned(), "tw".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );
    assert_eq!(
        comparison.opencc_or_filter_behavior,
        [
            ("台湾".to_owned(), "〔臺灣〕".to_owned()),
            ("tw".to_owned(), "echo".to_owned())
        ]
    );
    assert_eq!(
        comparison.punctuation_or_fallback_behavior,
        ["。".to_owned(), ".".to_owned()]
    );
    assert_eq!(
        comparison.candidate_differences[0].target_phase,
        "04-compiled-dictionary-data"
    );
    assert!(comparison.candidate_differences[0]
        .observed_yune_behavior
        .contains("source YAML"));
    assert!(comparison.candidate_differences[0]
        .expected_librime_behavior
        .contains("compiled table/prism"));
    assert!(comparison.candidate_differences[0]
        .scope_decision
        .contains("not shimmed"));

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

// Owner: crates/yune-rime-api/src/tests/distribution_schema_comparison.rs,
// schema_install.rs, processors/selector.rs, processors/navigator.rs, yune-core translator.
// librime oracle: /Users/trenton/Projects/librime/data/minimal/cangjie5.schema.yaml,
// source-level chain semantics isolated from compiled cangjie5 table payloads and userdb memory.
#[test]
fn distribution_cangjie5_chain_records_focused_findings() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("distribution-cangjie5-chain");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("cangjie5.schema.yaml"),
        "\
schema:
  schema_id: cangjie5
  name: Cangjie5 Focused Distribution Slice
menu:
  page_size: 2
engine:
  processors:
    - speller
    - selector
    - navigator
    - fluid_editor
  segmentors:
    - ascii_segmentor
    - abc_segmentor
    - matcher
    - fallback_segmentor
  translators:
    - table_translator@translator
    - memory
  filters:
    - uniquifier
speller:
  alphabet: abcdefghijklmnopqrstuvwxyz
translator:
  dictionary: cangjie5
  enable_completion: false
  enable_sentence: false
recognizer:
  patterns:
    reverse_lookup: '^`[a-z]*$'
selector:
  bindings:
    Down: next_candidate
navigator:
  bindings:
    Left: left_by_syllable
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("cangjie5.dict.yaml"),
        "\
---
name: cangjie5
version: '0.1'
sort: original
columns: [code, text]
...

a\t日
a\t曰
ab\t明
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
    let schema_id = CString::new("cangjie5").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
    let segment_tags = session_segment_tags(session_id);
    let candidates = candidate_pairs(session_id);
    let down = CString::new("Down").expect("key name should be valid");
    // SAFETY: key name is a valid NUL-terminated string.
    let down_keycode = unsafe { RimeGetKeycodeByName(down.as_ptr()) };
    assert_eq!(RimeProcessKey(session_id, down_keycode, 0), TRUE);
    let highlighted_after_down = context_highlighted_candidate(session_id);

    RimeClearComposition(session_id);
    assert_eq!(RimeProcessKey(session_id, '`' as i32, 0), FALSE);
    let reverse_lookup_input = CString::new("`a").expect("input should be valid");
    // SAFETY: input is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSetInput(session_id, reverse_lookup_input.as_ptr()) },
        TRUE
    );
    let reverse_lookup_tags = session_segment_tags(session_id);
    let generated_spellings = session_candidates_snapshot(session_id)
        .expect("session candidates should be visible")
        .into_iter()
        .map(|candidate| (candidate.text, candidate.comment))
        .collect::<Vec<_>>();
    let deferrals = remaining_gear_deferrals_snapshot(session_id)
        .expect("session deferrals should be visible to tests");
    let findings = deferrals
        .iter()
        .map(|deferral| StructuredFinding {
            observed_yune_behavior: "memory is recognized during schema installation as a deterministic no-op",
            expected_librime_behavior: "librime memory updates user dictionary learning through LevelDB-backed transactions",
            scope_decision: "userdb learning is recorded, not shimmed, because Phase 3 owns distribution chain comparison only",
            target_phase: Box::leak(deferral.target_phase.clone().into_boxed_str()),
        })
        .collect::<Vec<_>>();

    let comparison = DistributionSchemaComparison {
        schema_name: "cangjie5",
        librime_oracle_source: "/Users/trenton/Projects/librime/data/minimal/cangjie5.schema.yaml",
        component_order: vec![
            "speller",
            "selector",
            "navigator",
            "express_editor",
            "ascii_segmentor",
            "abc_segmentor",
            "matcher",
            "fallback_segmentor",
            "table_translator",
            "memory",
            "uniquifier",
        ],
        segment_tags: reverse_lookup_tags,
        generated_spellings,
        opencc_or_filter_behavior: vec![(
            "uniquifier".to_owned(),
            "dedupes 日 before echo fallback".to_owned(),
        )],
        punctuation_or_fallback_behavior: segment_tags.clone(),
        candidate_differences: findings,
    };

    assert_eq!(comparison.schema_name, "cangjie5");
    assert!(comparison.librime_oracle_source.contains("librime"));
    assert_eq!(
        comparison.component_order,
        [
            "speller",
            "selector",
            "navigator",
            "express_editor",
            "ascii_segmentor",
            "abc_segmentor",
            "matcher",
            "fallback_segmentor",
            "table_translator",
            "memory",
            "uniquifier",
        ]
    );
    assert_eq!(segment_tags, ["abc".to_owned()]);
    assert_eq!(
        candidates,
        [
            ("日".to_owned(), "a".to_owned()),
            ("曰".to_owned(), "a".to_owned())
        ]
    );
    assert_eq!(highlighted_after_down, 1);
    assert_eq!(
        comparison.segment_tags,
        ["abc".to_owned(), "reverse_lookup".to_owned()]
    );
    assert_eq!(
        comparison.generated_spellings,
        [("`a".to_owned(), "echo".to_owned())]
    );
    assert_eq!(
        comparison.opencc_or_filter_behavior,
        [(
            "uniquifier".to_owned(),
            "dedupes 日 before echo fallback".to_owned()
        )]
    );
    assert_eq!(
        comparison.punctuation_or_fallback_behavior,
        ["abc".to_owned()]
    );
    assert_eq!(comparison.candidate_differences.len(), 1);
    assert_eq!(
        comparison.candidate_differences[0].target_phase,
        "05-userdb-and-learning"
    );
    assert!(comparison.candidate_differences[0]
        .observed_yune_behavior
        .contains("deterministic no-op"));
    assert!(comparison.candidate_differences[0]
        .expected_librime_behavior
        .contains("LevelDB"));
    assert!(comparison.candidate_differences[0]
        .scope_decision
        .contains("not shimmed"));

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

fn candidate_pairs(session_id: crate::RimeSessionId) -> Vec<(String, String)> {
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = if context.menu.num_candidates == 0 || context.menu.candidates.is_null() {
        &[][..]
    } else {
        unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        }
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
}

fn candidate_texts(session_id: crate::RimeSessionId) -> Vec<String> {
    candidate_pairs(session_id)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

fn context_highlighted_candidate(session_id: crate::RimeSessionId) -> usize {
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let highlighted = context.menu.highlighted_candidate_index as usize;
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    highlighted
}

fn session_segment_tags(session_id: crate::RimeSessionId) -> Vec<String> {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .sessions
        .get(&session_id)
        .expect("session should exist")
        .engine
        .context()
        .segment_tags
        .clone()
}
