use super::*;
use crate::remaining_gear_deferrals_snapshot;
use serde_json::Value;

const TYPEDUCK_V112_REVERSE_LOOKUP_PROMPT: &str =
    include_str!("../../../yune-core/tests/fixtures/typeduck-v1.1.2/reverse-lookup-prompt.json");

fn typeduck_v112_reverse_lookup_prompt_fixture() -> Value {
    serde_json::from_str(TYPEDUCK_V112_REVERSE_LOOKUP_PROMPT)
        .expect("TypeDuck v1.1.2 reverse-lookup prompt fixture should parse")
}

fn typeduck_v112_reverse_lookup_case(fixture: &Value) -> &Value {
    fixture["cases"]
        .as_array()
        .expect("reverse lookup cases should be an array")
        .first()
        .expect("reverse lookup fixture should have a case")
}

fn m21_gap_01_copy_asset(
    schema_root: &std::path::Path,
    destination_root: &std::path::Path,
    relative_path: &str,
) {
    let source = schema_root.join(relative_path);
    let destination = destination_root.join(relative_path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("M21 asset parent directory should be created");
    }
    fs::copy(&source, &destination).unwrap_or_else(|error| {
        panic!(
            "failed to copy M21 asset {} -> {}: {error}",
            source.display(),
            destination.display()
        )
    });
}

fn m21_gap_01_write_real_browser_assets(shared: &std::path::Path, staging: &std::path::Path) {
    let schema_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/typeduck-web/source/public/schema");
    for file_name in [
        "default.yaml",
        "default.custom.yaml",
        "common.yaml",
        "common.custom.yaml",
        "include.yaml",
        "template.yaml",
        "jyut6ping3_mobile.schema.yaml",
        "jyut6ping3.dict.yaml",
        "jyut6ping3_scolar.dict.yaml",
        "opencc/hk2s.json",
        "opencc/HKVariantsRev.ocd2",
        "opencc/HKVariantsRevPhrases.ocd2",
        "opencc/TSCharacters.ocd2",
        "opencc/TSPhrases.ocd2",
    ] {
        m21_gap_01_copy_asset(&schema_root, shared, file_name);
    }
    for file_name in ["default.yaml", "jyut6ping3_mobile.schema.yaml"] {
        m21_gap_01_copy_asset(&schema_root.join("build"), staging, file_name);
    }
}

fn m21_gap_01_context_snapshot(session_id: crate::RimeSessionId) -> Value {
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let preedit = static_c_string(context.composition.preedit);
    let commit_text_preview = static_c_string(context.commit_text_preview);
    let page_size = context.menu.page_size;
    let page_no = context.menu.page_no;
    let highlighted_candidate_index = context.menu.highlighted_candidate_index;
    let is_last_page = context.menu.is_last_page == TRUE;
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    serde_json::json!({
        "preedit": preedit,
        "commit_text_preview": commit_text_preview,
        "page_size": page_size,
        "page_no": page_no,
        "highlighted_candidate_index": highlighted_candidate_index,
        "is_last_page": is_last_page,
    })
}

fn m21_gap_01_capture_case(session_id: crate::RimeSessionId, input: &str) -> Value {
    RimeClearComposition(session_id);
    let processed = input
        .chars()
        .map(|key| RimeProcessKey(session_id, key as c_int, 0))
        .collect::<Vec<_>>();
    let context = m21_gap_01_context_snapshot(session_id);
    let engine_candidates =
        super::super::session_candidates_snapshot(session_id).expect("session should exist");
    let sentence_row_position = engine_candidates
        .iter()
        .position(|candidate| candidate.source == CandidateSource::Sentence);
    let fallback_gate = if sentence_row_position.is_some() {
        "fired_returned_sentence"
    } else if engine_candidates.is_empty() {
        "fired_no_sentence_returned"
    } else {
        "not_fired_base_candidates_nonempty"
    };
    let mut source_counts = std::collections::BTreeMap::<&'static str, usize>::new();
    for candidate in &engine_candidates {
        *source_counts.entry(candidate.source.as_str()).or_default() += 1;
    }
    let completion_seen = source_counts.contains_key("completion");
    let userdb_seen = source_counts.contains_key("user_table");
    let sentence_seen = source_counts.contains_key("sentence");
    let top_candidates = engine_candidates
        .iter()
        .take(50)
        .enumerate()
        .map(|(index, candidate)| {
            serde_json::json!({
                "index": index,
                "text": candidate.text.as_str(),
                "comment": candidate.comment.as_str(),
                "candidate_preedit": candidate.preedit.as_deref(),
                "source": candidate.source.as_str(),
                "quality": candidate.quality,
                "is_sentence": candidate.source == CandidateSource::Sentence,
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "input": input,
        "processed": processed,
        "preedit": context["preedit"],
        "commit_text_preview": context["commit_text_preview"],
        "page_size": context["page_size"],
        "page_no": context["page_no"],
        "highlighted_candidate_index": context["highlighted_candidate_index"],
        "is_last_page": context["is_last_page"],
        "candidate_count": engine_candidates.len(),
        "sentence_row_position": sentence_row_position,
        "source_counts": source_counts,
        "source_path_diagnostics": {
            "fallback_gate": fallback_gate,
            "completion_seen": completion_seen,
            "userdb_seen": userdb_seen,
            "sentence_seen": sentence_seen,
            "correction_seen": false,
            "correction_note": "CandidateSource does not distinguish correction rows; this corpus contains dictionary-composition inputs rather than typo probes."
        },
        "top_candidates": top_candidates,
    })
}

fn current_candidate_pairs(session_id: crate::RimeSessionId) -> Vec<(String, String)> {
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
}

// Owner: m21-plan-typeduck-web-product-comparison.md M21-GAP-01. This ignored
// probe captures the Yune real-browser-assets candidate surface with engine
// source labels; it does not add public ABI or TypeDuck-Web exports.
#[test]
#[ignore = "diagnostic: writes M21-GAP-01 Yune real-assets candidate evidence"]
fn m21_gap_01_sentence_composition_probe() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("m21-gap-01-sentence-composition");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    m21_gap_01_write_real_browser_assets(&shared, &staging);

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeInitialize(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("jyut6ping3_mobile").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let inputs = [
        "loengnincin",
        "leoicijyu",
        "ngohaigo",
        "loengjathau",
        "geijatcin",
        "gamjatheoi",
    ];
    let cases = inputs
        .iter()
        .map(|input| m21_gap_01_capture_case(session_id, input))
        .collect::<Vec<_>>();
    let evidence = serde_json::json!({
        "capture": {
            "scenario": "M21-GAP-01 Yune real-assets sentence-composition probe",
            "schema": "jyut6ping3_mobile",
            "candidate_source": "session_candidates_snapshot after RimeGetContext on the real RimeApi path",
            "asset_root": "third_party/typeduck-web/source/public/schema",
            "dictionary": "jyut6ping3.dict.yaml",
            "lookup_dictionary": "jyut6ping3_scolar.dict.yaml",
            "pinned_yune_commit": std::env::var("YUNE_M21_GAP_01_COMMIT").unwrap_or_else(|_| "unknown".to_owned()),
            "notes": [
                "fallback_gate=fired_returned_sentence when a sentence source row is present under the default sentence-only-as-fallback path",
                "fallback_gate=not_fired_base_candidates_nonempty when non-sentence candidates prevent sentence fallback",
                "correction rows are not separately encoded in CandidateSource; this corpus is not a typo/correction probe"
            ]
        },
        "inputs": inputs,
        "cases": cases,
    });
    let output =
        serde_json::to_string_pretty(&evidence).expect("M21 evidence should serialize as JSON");
    if let Ok(output_path) = std::env::var("YUNE_M21_GAP_01_OUTPUT") {
        fs::write(&output_path, output)
            .unwrap_or_else(|error| panic!("failed to write {output_path}: {error}"));
    } else {
        println!("M21_GAP_01_YUNE_JSON_BEGIN");
        println!("{output}");
        println!("M21_GAP_01_YUNE_JSON_END");
    }

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

fn dictionary_yaml_from_oracle_rows(name: &str, rows: &Value) -> String {
    let rows = rows
        .as_array()
        .expect("dictionary rows should be an array")
        .iter()
        .map(|row| row.as_str().expect("dictionary row should be a string"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("---\nname: {name}\nversion: '0.1'\nsort: original\n...\n\n{rows}\n")
}

#[test]
fn gets_and_selects_current_schema() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let schema_id = CString::new("sample_schema").expect("schema id should be valid");
    let mut buffer = vec![0 as c_char; 32];
    let mut short_buffer = vec![0 as c_char; 8];
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();
    let mut status = empty_status();

    // SAFETY: buffer points to writable storage.
    assert_eq!(
        unsafe { RimeGetCurrentSchema(session_id, buffer.as_mut_ptr(), buffer.len()) },
        TRUE
    );
    // SAFETY: `RimeGetCurrentSchema` wrote a trailing NUL into buffer.
    let current_schema = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(current_schema.to_str(), Ok("default"));

    assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
    // SAFETY: schema id is a valid nul-terminated C string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    // SAFETY: selecting a schema clears unread composition state.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
    // SAFETY: context points to writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);

    // SAFETY: buffer points to writable storage.
    assert_eq!(
        unsafe { RimeGetCurrentSchema(session_id, short_buffer.as_mut_ptr(), short_buffer.len()) },
        TRUE
    );
    // SAFETY: the raw byte view is bounded to the caller-owned buffer.
    let truncated_schema = unsafe {
        std::slice::from_raw_parts(short_buffer.as_ptr().cast::<u8>(), short_buffer.len())
    };
    assert_eq!(truncated_schema, b"sample_s");

    let mut zero_len_marker = b'?' as c_char;
    // SAFETY: librime's strncpy-based getter accepts a valid output pointer
    // with a zero-length buffer and leaves the pointed storage untouched.
    assert_eq!(
        unsafe { RimeGetCurrentSchema(session_id, &mut zero_len_marker, 0) },
        TRUE
    );
    assert_eq!(zero_len_marker, b'?' as c_char);

    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_name = unsafe { CStr::from_ptr(status.schema_name) };
    assert_eq!(status_schema_id.to_str(), Ok("sample_schema"));
    assert_eq!(status_schema_name.to_str(), Ok("sample_schema"));
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(
        unsafe { RimeGetCurrentSchema(session_id, std::ptr::null_mut(), 0) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, std::ptr::null()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeSelectSchema(session_id + 1, schema_id.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn select_schema_uses_deployed_schema_name_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("select-schema-name");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "schema:\n  schema_id: luna\n  name: Luna\n",
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
    let mut status = empty_status();

    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    // SAFETY: `RimeGetStatus` populated owned C strings.
    let status_schema_name = unsafe { CStr::from_ptr(status.schema_name) };
    assert_eq!(status_schema_id.to_str(), Ok("luna"));
    assert_eq!(status_schema_name.to_str(), Ok("Luna"));
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_switch_reset_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("select-schema-switch-reset");
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
  - name: ascii_mode
    reset: 1
  - name: full_shape
    reset: 0
  - options: [simplification, traditional]
    reset: 1
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
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    let mut status = empty_status();

    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, full_shape.as_ptr(), TRUE) };
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );

    // SAFETY: status points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_ascii_mode, TRUE);
    assert_eq!(status.is_full_shape, FALSE);
    assert_eq!(status.is_simplified, FALSE);
    assert_eq!(status.is_traditional, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_switch_translator_candidates() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator");
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
    - switch_translator
    - echo_translator
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    reset: 1
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
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ 西文".to_owned()),
            ("简体".to_owned(), String::new()),
            ("繁體".to_owned(), " ✓".to_owned()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_preserves_missing_state_indices_like_librime() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-missing-states");
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
    - switch_translator
    - echo_translator
switches:
  - name: ascii_mode
    states: [中文]
  - options: [simplification, traditional, emoji]
    states: [简体, ~, 表情]
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
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ ".to_owned()),
            ("简体".to_owned(), " ✓".to_owned()),
            ("表情".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 2), TRUE);
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    let emoji = CString::new("emoji").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(unsafe { RimeGetOption(session_id, emoji.as_ptr()) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_persists_librime_save_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-save-options");
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
    - switch_translator
    - echo_translator
switcher:
  save_options: [ascii_mode, simplification, traditional]
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    reset: 0
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
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);

    let mut user_config = empty_config();
    let user_id = CString::new("user").expect("config id should be valid");
    // SAFETY: config id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeUserConfigOpen(user_id.as_ptr(), &mut user_config) },
        TRUE
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/ascii_mode"),
        Some(TRUE)
    );
    // SAFETY: config owns state allocated by the shim.
    assert_eq!(unsafe { RimeConfigClose(&mut user_config) }, TRUE);

    assert_eq!(RimeProcessKey(session_id, 'y' as c_int, 0), TRUE);
    assert_eq!(RimeSelectCandidate(session_id, 2), TRUE);

    let mut user_config = empty_config();
    // SAFETY: config id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeUserConfigOpen(user_id.as_ptr(), &mut user_config) },
        TRUE
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/ascii_mode"),
        Some(TRUE)
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/simplification"),
        Some(FALSE)
    );
    assert_eq!(
        config_bool(&mut user_config, "var/option/traditional"),
        Some(TRUE)
    );
    // SAFETY: config owns state allocated by the shim.
    assert_eq!(unsafe { RimeConfigClose(&mut user_config) }, TRUE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_restores_librime_switcher_saved_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switcher-restore-save-options");
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
switcher:
  save_options: [ascii_mode, full_shape, simplification, traditional]
switches:
  - name: ascii_mode
    states: [中文, 西文]
    reset: 0
  - name: full_shape
    states: [半角, 全角]
  - options: [simplification, traditional]
    states: [简体, 繁體]
",
    )
    .expect("schema config should be written");
    fs::write(
        user.join("user.yaml"),
        "\
var:
  option:
    ascii_mode: true
    full_shape: 'true'
    simplification: false
    traditional: true
",
    )
    .expect("user config should be written");

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

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_normalizes_radio_group_selection() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-radio-default");
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
    - switch_translator
    - echo_translator
switches:
  - options: [simplification, traditional]
    states: [简体, 繁體]
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
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

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
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("简体".to_owned(), " ✓".to_owned()),
            ("繁體".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    // SAFETY: option names are valid NUL-terminated strings.
    unsafe {
        RimeSetOption(session_id, simplification.as_ptr(), TRUE);
        RimeSetOption(session_id, traditional.as_ptr(), TRUE);
    }
    RimeClearComposition(session_id);
    assert_eq!(RimeProcessKey(session_id, 'y' as c_int, 0), TRUE);
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_folds_and_unfolds_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-options");
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
    - switch_translator
    - echo_translator
switcher:
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
  abbreviate_options: 'true'
switches:
  - name: ascii_mode
    states: [中文, 西文]
    abbrev: [中, 西]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    abbrev: [简, 繁]
    reset: 1
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
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, fold_options.as_ptr(), TRUE) };
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中/繁]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, fold_options.as_ptr()) },
        FALSE
    );
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let composition_input = unsafe { CStr::from_ptr(context.composition.preedit) }
        .to_string_lossy()
        .into_owned();
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
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(composition_input, "x");
    assert_eq!(
        candidate_pairs,
        [
            ("中文".to_owned(), "→ 西文".to_owned()),
            ("简体".to_owned(), String::new()),
            ("繁體".to_owned(), " ✓".to_owned()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to writable storage.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_honors_librime_fold_options_default() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-options-default");
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
    - switch_translator
    - echo_translator
switcher:
  fold_options: 'true'
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
  abbreviate_options: true
switches:
  - name: ascii_mode
    states: [中文, 西文]
    abbrev: [中, 西]
    reset: 0
  - options: [simplification, traditional]
    states: [简体, 繁體]
    abbrev: [简, 繁]
    reset: 1
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
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    // SAFETY: option name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeGetOption(session_id, fold_options.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中/繁]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_switch_translator_folds_default_radio_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-switch-translator-fold-radio-default");
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
    - switch_translator
    - echo_translator
switcher:
  option_list_prefix: '['
  option_list_suffix: ']'
  option_list_separator: '/'
switches:
  - name: ascii_mode
    states: [中文, 西文]
  - options: [simplification, traditional]
    states: [简体, 繁體]
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
    let fold_options = CString::new("_fold_options").expect("option name should be valid");
    let simplification = CString::new("simplification").expect("option name should be valid");
    let traditional = CString::new("traditional").expect("option name should be valid");
    // SAFETY: option names are valid NUL-terminated strings.
    unsafe { RimeSetOption(session_id, fold_options.as_ptr(), TRUE) };
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );
    assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

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
    let candidate_pairs = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_string_lossy()
                .into_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_string_lossy()
                    .into_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(
        candidate_pairs,
        [
            ("[中文/简体]".to_owned(), String::new()),
            ("x".to_owned(), "echo".to_owned()),
        ]
    );
    // SAFETY: option names are valid NUL-terminated strings. Librime selects
    // the first radio option while constructing the visible switch menu.
    assert_eq!(
        unsafe { RimeGetOption(session_id, simplification.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, traditional.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_dictionary_packs() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-dictionary-packs");
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
  packs:
    - luna_pack
    - missing_pack
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
...

八\tba\t1
",
    )
    .expect("primary dictionary should be written");
    fs::write(
        shared.join("luna_pack.dict.yaml"),
        "\
---
name: luna_pack
version: '0.1'
sort: original
columns: [code, text, weight]
...

ba\t爸\t9
ba\t吧\t3
",
    )
    .expect("pack dictionary should be written");

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

    assert_eq!(texts, ["爸", "吧", "八", "ba"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_uses_preset_vocabulary_for_dictionary_weights() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-preset-vocabulary");
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
use_preset_vocabulary: true
...

八\tba
吧\tba\t50%
爸\tba\t1
",
    )
    .expect("dictionary should be written");
    fs::write(
        shared.join("essay.txt"),
        "\
八\t8
吧\t6
",
    )
    .expect("preset vocabulary should be written");

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

    assert_eq!(texts, ["八", "吧", "爸", "ba"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_encodes_rule_based_dictionary_and_preset_phrases() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-encoder-phrase-injection");
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
use_preset_vocabulary: true
max_phrase_length: 2
min_phrase_weight: 10
encoder:
  rules:
    - length_equal: 2
      formula: \"AaBa\"
...

你\tni\t10
好\thao\t9
您\tnin\t8
你好\t\t50%
",
    )
    .expect("dictionary should be written");
    fs::write(
        shared.join("essay.txt"),
        "\
您好\t11
你好啊\t20
低频\t9
",
    )
    .expect("preset vocabulary should be written");

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
    for ch in "nh".chars() {
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

    assert_eq!(texts, ["您好", "你好", "nh"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_namespaced_librime_table_translator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-namespaced-table-translator");
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
    - table_translator@custom_table
    - echo_translator
translator:
  dictionary: base
custom_table:
  dictionary: custom
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("base.dict.yaml"),
        "\
---
name: base
version: '0.1'
sort: by_weight
...

基\tji\t9
",
    )
    .expect("default dictionary should be written");
    fs::write(
        shared.join("custom.dict.yaml"),
        "\
---
name: custom
version: '0.1'
sort: by_weight
...

机\tji\t9
",
    )
    .expect("custom dictionary should be written");

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
    for ch in "ji".chars() {
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

    assert_eq!(texts, ["机", "ji"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_preserves_librime_translator_prescription_order() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-order");
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
    - script_translator@first_table
    - table_translator@second_table
    - echo_translator
first_table:
  dictionary: first
  enable_completion: false
second_table:
  dictionary: second
  enable_completion: false
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("first.dict.yaml"),
        "\
---
name: first
version: '0.1'
sort: original
...

先\txu\t0
",
    )
    .expect("first dictionary should be written");
    fs::write(
        shared.join("second.dict.yaml"),
        "\
---
name: second
version: '0.1'
sort: original
...

后\txu\t0
",
    )
    .expect("second dictionary should be written");

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
    for ch in "xu".chars() {
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

    assert_eq!(texts, ["先", "后", "xu"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_installs_echo_translator_only_when_declared() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-explicit-echo-translator");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("table-only.schema.yaml"),
        "\
schema:
  schema_id: table-only
  name: Table Only
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  enable_completion: false
",
    )
    .expect("table-only schema config should be written");
    fs::write(
        staging.join("table-echo.schema.yaml"),
        "\
schema:
  schema_id: table-echo
  name: Table Echo
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
",
    )
    .expect("table-echo schema config should be written");
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

    let candidate_texts_and_comments_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(RimeProcessKey(session_id, 'x' as c_int, 0), TRUE);

        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let texts_and_comments = if context.menu.num_candidates == 0 {
            Vec::new()
        } else {
            let candidates = unsafe {
                std::slice::from_raw_parts(
                    context.menu.candidates,
                    context.menu.num_candidates as usize,
                )
            };
            candidates
                .iter()
                .map(|candidate| {
                    // SAFETY: candidate string pointers are populated by `RimeGetContext`.
                    let text = unsafe { CStr::from_ptr(candidate.text) }
                        .to_str()
                        .expect("candidate text should be valid UTF-8")
                        .to_owned();
                    let comment = unsafe { CStr::from_ptr(candidate.comment) }
                        .to_str()
                        .expect("candidate comment should be valid UTF-8")
                        .to_owned();
                    (text, comment)
                })
                .collect::<Vec<_>>()
        };
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        texts_and_comments
    };

    assert!(candidate_texts_and_comments_for("table-only").is_empty());
    assert_eq!(
        candidate_texts_and_comments_for("table-echo"),
        [("x".to_owned(), "echo".to_owned())]
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_preserves_librime_filter_prescription_order() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-filter-order");
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
    - uniquifier
    - simplifier
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
columns: [code, text, weight]
...

tw\t臺\t0
tw\t台\t0
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
    let option = CString::new("simplification").expect("option name should be valid");
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

    assert_eq!(texts, ["台", "台", "tw"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_uniquifier_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-uniquifier-filter");
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
    - uniquifier
translator:
  dictionary: luna
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

ni\t你\t9
ni\t你\t8
ni\t呢\t7
ni\tni\t6
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

    assert_eq!(texts, ["你", "呢", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_single_char_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-single-char-filter");
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
    - single_char_filter
translator:
  dictionary: luna
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

ni\t你好\t9
ni\t你\t8
ni\t呢\t7
ni\t你们\t6
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

    assert_eq!(texts, ["你", "呢", "你好", "你们", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_charset_filter_alias() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-charset-filter");
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
    - cjk_minifier
translator:
  dictionary: luna
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

ni\t你\t9
ni\t㐀\t8
ni\t𠀀\t7
ni\t㍿\t6
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

    assert_eq!(candidate_texts(), ["你", "ni"]);

    let option = CString::new("extended_charset").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    assert_eq!(candidate_texts(), ["你", "㐀", "𠀀", "㍿", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_namespaced_librime_charset_filter_alias() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-namespaced-charset-filter");
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
    - cjk_minifier@charset_guard
translator:
  dictionary: luna
charset_guard:
  tags: [abc]
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

ni\t你\t9
ni\t𠀀\t8
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

#[test]
fn select_schema_applies_librime_translator_charset_filter_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-charset-filter");
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
  enable_charset_filter: true
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

ni\t你\t9
ni\t㐀\t8
ni\t𠀀\t7
ni\t㍿\t6
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

    assert_eq!(candidate_texts(), ["你", "ni"]);

    let option = CString::new("extended_charset").expect("option name should be valid");
    // SAFETY: option is a valid NUL-terminated string.
    unsafe { RimeSetOption(session_id, option.as_ptr(), TRUE) };

    assert_eq!(candidate_texts(), ["你", "㐀", "𠀀", "㍿", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_translator_completion_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-translator-completion");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("complete.schema.yaml"),
        "\
schema:
  schema_id: complete
  name: Complete
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: luna
",
    )
    .expect("completion schema config should be written");
    fs::write(
        staging.join("exact.schema.yaml"),
        "\
schema:
  schema_id: exact
  name: Exact
engine:
  translators:
    - table_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
",
    )
    .expect("exact schema config should be written");
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

    assert_eq!(candidate_texts_for("complete"), ["爸", "班", "b"]);
    assert_eq!(candidate_texts_for("exact"), ["b"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

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

#[test]
fn select_schema_loads_librime_history_translator() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-history-translator");
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
    - history_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
history:
  input: his
  size: 2
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
hao\t好
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
    for input in ["ni", "hao"] {
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), TRUE);
    }
    for ch in "his".chars() {
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

    assert_eq!(texts, ["好", "你", "his"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_treats_librime_history_translator_translator_namespace_as_history() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-history-translator-namespace");
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
    - history_translator@translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
  input: ignored
history:
  input: his
  size: 1
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
    assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), TRUE);
    for ch in "his".chars() {
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

    assert_eq!(texts, ["你", "his"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_applies_librime_history_translator_tag_option() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-history-translator-tag");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("allowed.schema.yaml"),
        "\
schema:
  schema_id: allowed
  name: Allowed
engine:
  translators:
    - table_translator
    - history_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
history:
  input: his
  tag: abc
",
    )
    .expect("allowed schema config should be written");
    fs::write(
        staging.join("blocked.schema.yaml"),
        "\
schema:
  schema_id: blocked
  name: Blocked
engine:
  translators:
    - table_translator
    - history_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
history:
  input: his
  tag: custom
",
    )
    .expect("blocked schema config should be written");
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

    let candidate_texts_for = |schema_id: &str| {
        let session_id = RimeCreateSession();
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        // SAFETY: schema id is a valid NUL-terminated string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        for ch in "ni".chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), TRUE);
        for ch in "his".chars() {
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

    assert_eq!(candidate_texts_for("allowed"), ["你", "his"]);
    assert_eq!(candidate_texts_for("blocked"), ["his"]);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_parses_librime_history_translator_numeric_scalars() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-history-translator-scalars");
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
    - history_translator
    - echo_translator
translator:
  dictionary: luna
  enable_completion: false
history:
  input: his
  size: '2'
  initial_quality: '-10.0'
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
columns: [code, text, weight]
...

his\t表\t0
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
    for input in ["ni", "hao"] {
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        assert_eq!(RimeProcessKey(session_id, XK_RETURN, 0), TRUE);
    }
    for ch in "his".chars() {
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

    assert_eq!(texts, ["表", "his", "hao", "ni"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

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

#[test]
fn select_schema_loads_librime_reverse_lookup_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-filter");
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
    - reverse_lookup_filter
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  overwrite_comment: true
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

你\tni
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

你\twq
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
            ("你".to_owned(), Some("〔WQ〕".to_owned())),
            ("ni".to_owned(), Some("echo".to_owned()))
        ]
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_typeduck_dictionary_lookup_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-dictionary-lookup-filter");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("typeduck.schema.yaml"),
        "\
schema:
  schema_id: typeduck
  name: TypeDuck
engine:
  translators:
    - table_translator
  filters:
    - dictionary_lookup_filter
translator:
  dictionary: typeduck
  enable_completion: false
dictionary_lookup_filter:
  dictionary: typeduck_lookup
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("typeduck.dict.yaml"),
        "\
---
name: typeduck
version: '0.1'
sort: original
...

word\tnei
",
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("typeduck_lookup.dict.yaml"),
        "\
---
name: typeduck_lookup
version: '0.1'
sort: original
columns: [text, code, weight, stem, source, jyutping, english]
...

word\tnei\t1\tn\tprimary\tnei\tyou
word\tlei\t2\tl\tvariant\tlei\tyou alt
",
    )
    .expect("dictionary lookup rows should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("typeduck").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "nei".chars() {
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

    assert_eq!(first_text, "word");
    assert_eq!(
        first_comment,
        "\u{000c}\r1,word,nei,1,n,primary,nei,you\r0,word,lei,2,l,variant,lei,you alt"
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_namespaced_librime_reverse_lookup_filter() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-namespaced-reverse-lookup-filter");
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
    - reverse_lookup_filter@stroke_lookup
translator:
  dictionary: luna
stroke_lookup:
  dictionary: stroke
  overwrite_comment: true
  comment_format:
    - xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/
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

你\tni
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

你\twq
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

    assert_eq!(text, "你");
    assert_eq!(comment, "WQ");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_loads_librime_reverse_lookup_filter_filter_namespace_alias() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-filter-alias");
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
    - reverse_lookup_filter@filter
translator:
  dictionary: luna
reverse_lookup:
  dictionary: stroke
  overwrite_comment: true
  comment_format:
    - xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/
filter:
  dictionary: wrong
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

你\tni
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

你\twq
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

    assert_eq!(first_text, "你");
    assert_eq!(first_comment, "WQ");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn select_schema_reverse_lookup_filter_updates_sentence_candidates() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-reverse-lookup-filter-sentence");
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
    - table_translator
  filters:
    - reverse_lookup_filter
translator:
  dictionary: sentence
  enable_completion: false
reverse_lookup:
  dictionary: stroke
  overwrite_comment: true
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("sentence.dict.yaml"),
        "\
---
name: sentence
version: '0.1'
sort: original
columns: [code, text]
...

ba\t爸
bao\t包
",
    )
    .expect("sentence dictionary should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

爸包\tbb
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
    let schema_id = CString::new("sentence").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "babao".chars() {
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

    assert_eq!(first_text, "爸包");
    assert_eq!(first_comment, "bb");

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

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
