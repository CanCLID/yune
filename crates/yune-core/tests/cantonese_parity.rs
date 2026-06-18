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
fn yune_dictionary_lookup_filter_emits_oracle_bytes_from_source_rows() {
    let fixture = oracle_fixture();

    assert_source_rows_emit_oracle_comment(
        &fixture,
        "nei",
        0,
        "你",
        "nei5",
        &["你\tnei5\t1\t0\t\toth\t\t\t\t\t\t\tyou (singular)\tتم\tतपाईं\tआप\tkamu"],
    );
    assert_source_rows_emit_oracle_comment(
        &fixture,
        "nei",
        1,
        "呢",
        "nei1",
        &[
            "呢\tnei1\t2\t0\t\toth\tver\t\t\t這\t\t\tthis\t\t\t\t",
            "呢\tne1\t1\t0\t\tpart\t\t\t\t\t\t\t(how about)\t(particle)\t\t(particle)\t(imbuhan kata)",
            "呢\tni1\t2\t0\t\toth\tver\t\t\t這\t\t\tthis\t\t\t\t",
        ],
    );
    assert_source_rows_emit_oracle_comment(
        &fixture,
        "hou",
        0,
        "好",
        "hou2",
        &[
            "好\thou2\t1\t0\t\tadj\t\t\t\t\t\t\tgood; very\tبہت\tधेरै\tबहुत\tsangat",
            "好\thou3\t2\t0\t\tv\t\t\t\t\t\t\tlike\tجیسے\tजस्तै\tपसंद\tsuka",
        ],
    );
    assert_source_rows_emit_oracle_comment(
        &fixture,
        "hou",
        1,
        "好",
        "hou3",
        &[
            "好\thou2\t1\t0\t\tadj\t\t\t\t\t\t\tgood; very\tبہت\tधेरै\tबहुत\tsangat",
            "好\thou3\t2\t0\t\tv\t\t\t\t\t\t\tlike\tجیسے\tजस्तै\tपसंद\tsuka",
        ],
    );
}

fn assert_source_rows_emit_oracle_comment(
    fixture: &Value,
    input: &str,
    index: i64,
    text: &str,
    code: &str,
    source_rows: &[&str],
) {
    let expected_comment = oracle_candidate_comment(fixture, input, index);
    let dictionary_yaml = dictionary_yaml_from_source_rows(source_rows);
    let dictionary = TableDictionary::parse_rime_dict_yaml(&dictionary_yaml)
        .expect("TypeDuck source rows should parse as dictionary rows");
    let mut candidates = vec![Candidate {
        text: text.to_owned(),
        comment: code.to_owned(),
        source: CandidateSource::Table,
        quality: 1.0,
    }];

    DictionaryLookupFilter::new(dictionary).apply(&mut candidates);

    assert_eq!(candidates[0].comment, expected_comment);
}

fn oracle_candidate_comment<'a>(fixture: &'a Value, input: &str, index: i64) -> &'a str {
    let case = fixture["cases"]
        .as_array()
        .expect("oracle cases should be an array")
        .iter()
        .find(|case| case["input"] == input)
        .expect("input should be captured");
    case["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array")
        .iter()
        .find(|candidate| candidate["index"] == index)
        .expect("candidate index should be captured")["comment"]
        .as_str()
        .expect("candidate comment should be a string")
}

fn dictionary_yaml_from_source_rows(rows: &[&str]) -> String {
    let rows = rows.join("\n");
    format!("---\nname: typeduck_oracle\nversion: '0.1'\nsort: original\n...\n\n{rows}\n")
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
