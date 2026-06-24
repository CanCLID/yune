use super::*;
use crate::remaining_gear_deferrals_snapshot;
use crate::{begin_startup_trace, finish_startup_trace};
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
        .join("../../apps/yune-web/source/public/schema");
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

include!("schema_selection/typeduck_profile_probe.rs");
include!("schema_selection/lifecycle_switching.rs");
include!("schema_selection/dictionaries_translators.rs");
include!("schema_selection/filters_options.rs");
include!("schema_selection/translator_options.rs");
include!("schema_selection/history_translator.rs");
include!("schema_selection/translator_format_options.rs");
include!("schema_selection/simplifier_filter.rs");
include!("schema_selection/reverse_lookup_translator.rs");
include!("schema_selection/reverse_lookup_filters.rs");
include!("schema_selection/punctuation_spelling_correction.rs");
include!("schema_selection/deferred_oracles.rs");
