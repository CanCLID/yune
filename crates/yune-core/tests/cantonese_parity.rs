use serde_json::Value;
use yune_core::{
    Candidate, CandidateFilter, CandidateSource, DictionaryLookupFilter, TableDictionary,
};

const ORACLE: &str = include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json");

fn oracle_fixture() -> Value {
    serde_json::from_str(ORACLE).expect("TypeDuck v1.1.2 oracle fixture should be valid JSON")
}

#[test]
fn typeduck_v112_jyutping_oracle_fixture_is_locked() {
    let fixture = oracle_fixture();
    assert_eq!(fixture["oracle"]["engine"], "TypeDuck-HK/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "v1.1.2");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "74cb52b78fb2411137a7643f6c8bc6517acfde69"
    );
    assert_eq!(fixture["schema"], "jyut6ping3_mobile");
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default", "dictionary_lookup"])
    );

    let cases = fixture["cases"]
        .as_array()
        .expect("oracle cases should be an array");
    let inputs = cases
        .iter()
        .map(|case| {
            case["input"]
                .as_str()
                .expect("case input should be a string")
        })
        .collect::<Vec<_>>();
    assert_eq!(inputs, ["nei", "hou", "zyu", "haau"]);

    for case in cases {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        assert_eq!(case["schema_id"], "jyut6ping3_mobile");
        assert!(case["schema_name"]
            .as_str()
            .is_some_and(|schema_name| !schema_name.is_empty()));
        assert_eq!(case["is_composing"], true);
        assert_eq!(case["is_ascii_mode"], false);
        assert_eq!(case["preedit"], input);
        assert_eq!(case["highlighted_candidate_index"], 0);
        assert_eq!(case["page_size"], 50);
        assert_eq!(case["page_no"], 0);
        assert_eq!(case["is_last_page"], true);
        assert_eq!(
            case["processed"]
                .as_array()
                .expect("processed keys should be an array")
                .len(),
            input.len()
        );

        let selected_candidates = case["selected_candidates"]
            .as_array()
            .expect("selected candidates should be an array");
        assert!(
            selected_candidates.len() >= 3,
            "case {input} should preserve sampled dictionary panel candidates"
        );
        for candidate in selected_candidates {
            let comment = candidate["comment"]
                .as_str()
                .expect("candidate comment should be a string");
            assert!(
                comment.starts_with("\u{000c}\r1,"),
                "case {input} candidate comment should start with the TypeDuck panel marker"
            );
        }
    }
}

#[test]
fn yune_dictionary_lookup_filter_replays_typeduck_comment_payloads() {
    let fixture = oracle_fixture();
    let hou_case = fixture["cases"]
        .as_array()
        .expect("oracle cases should be an array")
        .iter()
        .find(|case| case["input"] == "hou")
        .expect("hou case should be captured");
    let selected_candidates = hou_case["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array");

    for candidate in selected_candidates.iter().take(2) {
        let expected_comment = candidate["comment"]
            .as_str()
            .expect("candidate comment should be a string");
        assert!(
            expected_comment.contains("\r0,"),
            "hou sample should include alternate pronunciation records"
        );
        let dictionary_yaml = dictionary_yaml_from_typeduck_comment(expected_comment);
        let dictionary = TableDictionary::parse_rime_dict_yaml(&dictionary_yaml)
            .expect("TypeDuck comment records should round-trip as dictionary rows");
        let (text, code) = primary_text_and_code(expected_comment);
        let mut candidates = vec![Candidate {
            text,
            comment: code,
            source: CandidateSource::Table,
            quality: 1.0,
        }];

        DictionaryLookupFilter::new(dictionary).apply(&mut candidates);

        assert_eq!(candidates[0].comment, expected_comment);
    }
}

fn dictionary_yaml_from_typeduck_comment(comment: &str) -> String {
    let rows = typeduck_comment_records(comment)
        .into_iter()
        .map(|record| record.join("\t"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("---\nname: typeduck_oracle\nversion: '0.1'\nsort: original\n...\n\n{rows}\n")
}

fn primary_text_and_code(comment: &str) -> (String, String) {
    let primary = typeduck_comment_records(comment)
        .into_iter()
        .next()
        .expect("comment should contain a primary record");
    (
        primary
            .first()
            .expect("primary record should contain text")
            .clone(),
        primary
            .get(1)
            .expect("primary record should contain code")
            .clone(),
    )
}

fn typeduck_comment_records(comment: &str) -> Vec<Vec<String>> {
    comment
        .trim_start_matches('\u{000c}')
        .split('\r')
        .filter(|record| !record.is_empty())
        .map(|record| {
            let (marker, fields) = record
                .split_once(',')
                .expect("TypeDuck comment record should have a marker");
            assert!(
                marker == "1" || marker == "0",
                "TypeDuck comment marker should be primary or alternate"
            );
            fields.split(',').map(ToOwned::to_owned).collect()
        })
        .collect()
}

#[test]
#[ignore = "blocked: capture v1.1.2 goldens for combine_candidates, show_full_code, and enable_sentence toggles before enabling"]
fn options_combine_candidates_show_full_code_enable_sentence_parity() {
    panic!("missing dedicated TypeDuck v1.1.2 option-toggle oracle fixture");
}

#[test]
#[ignore = "blocked: capture v1.1.2 completion/prediction and enable_completion option goldens before enabling"]
fn completion_prediction_and_enable_completion_parity() {
    panic!("missing dedicated TypeDuck v1.1.2 completion/prediction oracle fixture");
}

#[test]
#[ignore = "blocked: capture v1.1.2 correction goldens for minimal distance and m-abbreviation penalties before enabling"]
fn correction_minimal_distance_and_m_abbreviation_parity() {
    panic!("missing dedicated TypeDuck v1.1.2 correction oracle fixture");
}

#[test]
#[ignore = "blocked: capture v1.1.2 schema-menu hiding goldens for hide-lone-schema and hide-caret behavior before enabling"]
fn schema_menu_hiding_parity() {
    panic!("missing dedicated TypeDuck v1.1.2 schema-menu oracle fixture");
}

#[test]
#[ignore = "blocked: capture v1.1.2 userdb fixtures with per-entry pronunciations before enabling"]
fn per_entry_userdb_pronunciation_parity() {
    panic!("missing dedicated TypeDuck v1.1.2 userdb pronunciation oracle fixture");
}
