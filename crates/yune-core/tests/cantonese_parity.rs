use serde_json::Value;
use yune_core::{
    Candidate, CandidateFilter, CandidateSource, DictionaryLookupFilter, ReverseLookupTranslator,
    StaticTableTranslator, TableDictionary, Translator,
};

const ORACLE: &str = include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json");
const M14_SMOKE_ORACLE: &str = include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-m14-smoke.json");
const M14_OPTIONS_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-m14-options.json");
const M14_COMPLETION_CORRECTION_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-m14-completion-correction.json");
const M14_SCHEMA_MENU_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-m14-schema-menu.json");
const M14_USERDB_ORACLE: &str = include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-m14-userdb.json");
const REVERSE_LOOKUP_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/reverse-lookup-prompt.json");

fn oracle_fixture() -> Value {
    serde_json::from_str(ORACLE).expect("TypeDuck v1.1.2 oracle fixture should be valid JSON")
}

fn m14_smoke_fixture() -> Value {
    serde_json::from_str(M14_SMOKE_ORACLE)
        .expect("TypeDuck v1.1.2 M14 smoke fixture should be valid JSON")
}

fn m14_options_fixture() -> Value {
    serde_json::from_str(M14_OPTIONS_ORACLE)
        .expect("TypeDuck v1.1.2 M14 options fixture should be valid JSON")
}

fn m14_completion_correction_fixture() -> Value {
    serde_json::from_str(M14_COMPLETION_CORRECTION_ORACLE)
        .expect("TypeDuck v1.1.2 M14 completion/correction fixture should be valid JSON")
}

fn m14_schema_menu_fixture() -> Value {
    serde_json::from_str(M14_SCHEMA_MENU_ORACLE)
        .expect("TypeDuck v1.1.2 M14 schema-menu fixture should be valid JSON")
}

fn m14_userdb_fixture() -> Value {
    serde_json::from_str(M14_USERDB_ORACLE)
        .expect("TypeDuck v1.1.2 M14 userdb fixture should be valid JSON")
}

fn reverse_lookup_fixture() -> Value {
    serde_json::from_str(REVERSE_LOOKUP_ORACLE)
        .expect("TypeDuck v1.1.2 reverse-lookup fixture should be valid JSON")
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
fn typeduck_v112_m14_smoke_fixture_is_locked() {
    let fixture = m14_smoke_fixture();
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
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_binary_smoke"
    );
    assert!(fixture["oracle"]["capture_command"]
        .as_str()
        .is_some_and(|command| command.contains("scripts/capture-typeduck-jyutping.ps1")));

    let cases = fixture["cases"]
        .as_array()
        .expect("M14 smoke cases should be an array");
    assert_eq!(cases.len(), 1);
    let case = &cases[0];
    assert_eq!(case["input"], "nei");
    assert_eq!(case["schema_id"], "jyut6ping3_mobile");
    assert_eq!(case["preedit"], "nei");
    assert_eq!(case["commit_text_preview"], "你");

    let selected_candidates = case["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array");
    assert!(
        selected_candidates.len() >= 3,
        "M14 smoke should preserve sampled dictionary panel candidates"
    );
    assert_eq!(selected_candidates[0]["text"], "你");
    assert!(selected_candidates[0]["comment"]
        .as_str()
        .is_some_and(|comment| comment.starts_with("\u{000c}\r1,你,nei5")));
}

#[test]
fn typeduck_v112_m14_option_toggle_fixtures_are_locked() {
    let fixture = m14_options_fixture();
    assert_eq!(fixture["oracle"]["engine"], "TypeDuck-HK/librime");
    assert_eq!(fixture["schema"], "mixed");
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_deploy_time_option_variants"
    );

    let combined_hou = m14_case(&fixture, "combine_candidates_default", "hou");
    let separate_hou = m14_case(&fixture, "combine_candidates_separate", "hou");
    assert_eq!(candidate_count(combined_hou), 43);
    assert_eq!(candidate_count(separate_hou), 46);
    assert_ne!(
        selected_candidate_comment(combined_hou, 0),
        selected_candidate_comment(separate_hou, 0),
        "combine_candidates and separate_candidates should capture different comment grouping"
    );

    let sentence = m14_case(&fixture, "enable_sentence_default", "ngohaigo");
    let no_sentence = m14_case(&fixture, "enable_sentence_disabled", "ngohaigo");
    assert_eq!(sentence["preedit"], "ngo hai go");
    assert_eq!(no_sentence["preedit"], "ngo haigo");
    assert_eq!(candidate_count(sentence), 49);
    assert_eq!(candidate_count(no_sentence), 48);

    let full_code_off = m14_case(&fixture, "show_full_code_default", "`cam");
    let full_code_on = m14_case(&fixture, "show_full_code_enabled", "`cam");
    assert_eq!(full_code_off["schema_id"], "jyut6ping3");
    assert_eq!(full_code_on["schema_id"], "jyut6ping3");
    assert_eq!(candidate_count(full_code_off), 6);
    assert_eq!(candidate_count(full_code_on), 6);
    assert_ne!(
        selected_candidate_comment(full_code_off, 0),
        selected_candidate_comment(full_code_on, 0),
        "show_full_code should change cangjie side-lookup comments"
    );
}

#[test]
fn typeduck_v112_m14_completion_and_correction_fixtures_are_locked() {
    let fixture = m14_completion_correction_fixture();
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_deploy_time_completion_correction_variants"
    );

    assert_eq!(
        candidate_count(m14_case(&fixture, "completion_default", "n")),
        50
    );
    assert_eq!(
        candidate_count(m14_case(&fixture, "completion_disabled", "n")),
        50
    );
    assert_eq!(
        candidate_count(m14_case(&fixture, "completion_default", "ne")),
        1
    );

    let correction_default = m14_case(&fixture, "correction_default", "nri");
    let correction_enabled = m14_case(&fixture, "correction_enabled", "nri");
    assert_eq!(candidate_count(correction_default), 50);
    assert_eq!(candidate_count(correction_enabled), 6);
    assert_ne!(
        selected_candidate_text(correction_default, 0),
        selected_candidate_text(correction_enabled, 0),
        "enable_correction should capture a different top row for nri"
    );
}

#[test]
fn typeduck_v112_m14_schema_menu_surface_fixture_is_locked() {
    let fixture = m14_schema_menu_fixture();
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_schema_list_emitted_surface"
    );
    assert!(fixture["capture"]["oracle_observable_surface"]
        .as_str()
        .is_some_and(|surface| surface.contains("RimeGetSchemaList")));

    let one_schema = m14_case(&fixture, "one_schema_default", "");
    let multi_schema = m14_case(&fixture, "mobile_multi_schema_custom", "");
    assert_eq!(one_schema["rime_get_schema_list"], true);
    assert_eq!(multi_schema["rime_get_schema_list"], true);
    assert_eq!(one_schema["schemas"].as_array().expect("schemas").len(), 1);
    assert_eq!(
        multi_schema["schemas"].as_array().expect("schemas").len(),
        9
    );
}

#[test]
fn typeduck_v112_m14_userdb_export_fixture_is_locked() {
    let fixture = m14_userdb_fixture();
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default", "dictionary_lookup", "levers"])
    );
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_userdb_levers_export_probe"
    );
    let case = &fixture["cases"]
        .as_array()
        .expect("userdb cases should be an array")[0];
    let probe = &case["probe"];
    assert_eq!(probe["levers_module_found"], true);
    assert_eq!(probe["export_function_found"], true);
    assert_eq!(probe["export_return"], 1);
    let export_text = probe["export_text"]
        .as_str()
        .expect("userdb export text should be captured");
    assert!(export_text.contains("#@/db_name\tjyut6ping3"));
    assert!(export_text.contains("\tnei5\t1"));
}

#[test]
fn typeduck_v112_reverse_lookup_prompt_fixture_is_locked() {
    let fixture = reverse_lookup_fixture();
    assert_eq!(fixture["oracle"]["engine"], "TypeDuck-HK/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "v1.1.2");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "74cb52b78fb2411137a7643f6c8bc6517acfde69"
    );
    assert_eq!(fixture["schema"], "hr6_reverse");
    assert_eq!(fixture["capture"]["schema_name"], "HR6 粵語");
    assert_eq!(fixture["capture"]["reverse_lookup_tips"], "〔HR6 粵語〕");

    let case = reverse_lookup_case(&fixture);
    assert_eq!(case["schema_id"], "hr6_reverse");
    assert_eq!(case["schema_name"], "HR6 粵語");
    assert_eq!(case["input"], "`huo");
    assert_eq!(case["preedit"], "huo〔HR6 粵語〕");
    assert_eq!(case["commit_text_preview"], "火");
    assert_eq!(case["selected_candidates"][0]["text"], "火");
    assert_eq!(case["selected_candidates"][0]["comment"], "ho; huo");
}

#[test]
fn yune_reverse_lookup_translator_joins_comments_like_v112_oracle() {
    let fixture = reverse_lookup_fixture();
    let case = reverse_lookup_case(&fixture);
    let lookup_dictionary =
        TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_fixture_rows(
            "hr6_lookup",
            &fixture["capture"]["lookup_dictionary_rows"],
        ))
        .expect("lookup dictionary rows should parse");
    let target_dictionary =
        TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_fixture_rows(
            "hr6_target",
            &fixture["capture"]["target_dictionary_rows"],
        ))
        .expect("target dictionary rows should parse");
    let translator =
        ReverseLookupTranslator::new(lookup_dictionary, Some(target_dictionary), "`", ";");
    let input = case["input"]
        .as_str()
        .expect("reverse lookup input should be a string");
    let candidates = translator.translate(input);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].text, case["selected_candidates"][0]["text"]);
    assert_eq!(
        candidates[0].comment,
        case["selected_candidates"][0]["comment"]
            .as_str()
            .expect("oracle comment should be a string")
    );
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

fn reverse_lookup_case(fixture: &Value) -> &Value {
    fixture["cases"]
        .as_array()
        .expect("reverse lookup cases should be an array")
        .first()
        .expect("reverse lookup fixture should capture one case")
}

fn m14_case<'a>(fixture: &'a Value, variant: &str, input: &str) -> &'a Value {
    fixture["cases"]
        .as_array()
        .expect("M14 cases should be an array")
        .iter()
        .find(|case| {
            case["variant"] == variant
                && (input.is_empty()
                    || case
                        .get("input")
                        .is_some_and(|case_input| case_input == input))
        })
        .unwrap_or_else(|| panic!("M14 fixture should capture variant {variant} input {input}"))
}

fn candidate_count(case: &Value) -> usize {
    case["selected_candidates"]
        .as_array()
        .expect("selected_candidates should be an array")
        .len()
}

fn selected_candidate_text(case: &Value, index: usize) -> &str {
    case["selected_candidates"]
        .as_array()
        .expect("selected_candidates should be an array")[index]["text"]
        .as_str()
        .expect("candidate text should be a string")
}

fn selected_candidate_comment(case: &Value, index: usize) -> &str {
    case["selected_candidates"]
        .as_array()
        .expect("selected_candidates should be an array")[index]["comment"]
        .as_str()
        .expect("candidate comment should be a string")
}

fn dictionary_yaml_from_fixture_rows(name: &str, rows: &Value) -> String {
    let rows = rows
        .as_array()
        .expect("dictionary rows should be an array")
        .iter()
        .map(|row| row.as_str().expect("dictionary row should be a string"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("---\nname: {name}\nversion: '0.1'\nsort: original\n...\n\n{rows}\n")
}

fn dictionary_yaml_from_oracle_comments(name: &str, comments: &[&str]) -> String {
    let rows = comments
        .iter()
        .flat_map(|comment| comment.split('\u{000c}').skip(1))
        .flat_map(|records| records.split('\r'))
        .filter_map(|record| {
            record
                .strip_prefix("1,")
                .or_else(|| record.strip_prefix("0,"))
        })
        .map(|fields| fields.split(',').collect::<Vec<_>>().join("\t"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("---\nname: {name}\nversion: '0.1'\nsort: original\n...\n\n{rows}\n")
}

#[test]
fn options_combine_candidates_show_full_code_enable_sentence_parity() {
    let fixture = m14_options_fixture();
    let form_feed_comment = vec!["xform/^/\u{000c}/".to_owned()];
    let jyutping_algebra = vec!["derive/\\d//".to_owned()];

    let combined_hou = m14_case(&fixture, "combine_candidates_default", "hou");
    let separate_hou = m14_case(&fixture, "combine_candidates_separate", "hou");
    let hou_dictionary =
        TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_oracle_comments(
            "hou_lookup",
            &[
                selected_candidate_comment(combined_hou, 0),
                selected_candidate_comment(combined_hou, 1),
            ],
        ))
        .expect("M14 hou source rows should parse");

    let combined_translator =
        StaticTableTranslator::new([("hou2", "好"), ("hou3", "好"), ("hou6", "號")])
            .with_spelling_algebra(&jyutping_algebra)
            .with_comment_format(&form_feed_comment)
            .with_combine_candidates(true);
    let mut combined_candidates = combined_translator.translate("hou");
    DictionaryLookupFilter::new(hou_dictionary.clone()).apply(&mut combined_candidates);
    assert_eq!(
        combined_candidates[0].text,
        selected_candidate_text(combined_hou, 0)
    );
    assert_eq!(
        combined_candidates[0].comment,
        selected_candidate_comment(combined_hou, 0)
    );

    let separate_translator =
        StaticTableTranslator::new([("hou2", "好"), ("hou3", "好"), ("hou6", "號")])
            .with_spelling_algebra(&jyutping_algebra)
            .with_comment_format(&form_feed_comment)
            .with_combine_candidates(false);
    let mut separate_candidates = separate_translator.translate("hou");
    DictionaryLookupFilter::new(hou_dictionary).apply(&mut separate_candidates);
    assert_eq!(
        separate_candidates[0].comment,
        selected_candidate_comment(separate_hou, 0)
    );
    assert_eq!(
        separate_candidates[1].comment,
        selected_candidate_comment(separate_hou, 1)
    );

    let cangjie_formulas = vec![
        "xform/^/\u{000b}/".to_owned(),
        "xlit|abcdefghijklmnopqrstuvwxyz~|日月金木水火土竹戈十大中一弓人心手口尸廿山女田難卜符～|"
            .to_owned(),
    ];
    let full_code_off = m14_case(&fixture, "show_full_code_default", "`cam");
    let full_code_on = m14_case(&fixture, "show_full_code_enabled", "`cam");
    let cangjie_dictionary =
        TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_oracle_comments(
            "cangjie_lookup",
            &[
                selected_candidate_comment(full_code_off, 0),
                selected_candidate_comment(full_code_off, 1),
            ],
        ))
        .expect("M14 cangjie source rows should parse");

    let short_code_translator = StaticTableTranslator::new([("am", "旦"), ("amd", "旴")])
        .with_completion(true)
        .with_affix("`c", ";")
        .with_show_full_code(false)
        .with_comment_format(&cangjie_formulas);
    let mut short_code_candidates = short_code_translator.translate("`cam");
    DictionaryLookupFilter::new(cangjie_dictionary.clone()).apply(&mut short_code_candidates);
    assert_eq!(
        short_code_candidates[0].comment,
        selected_candidate_comment(full_code_off, 0)
    );
    assert_eq!(
        short_code_candidates[1].comment,
        selected_candidate_comment(full_code_off, 1)
    );

    let full_code_translator = StaticTableTranslator::new([("am", "旦"), ("amd", "旴")])
        .with_completion(true)
        .with_affix("`c", ";")
        .with_show_full_code(true)
        .with_comment_format(&cangjie_formulas);
    let mut full_code_candidates = full_code_translator.translate("`cam");
    DictionaryLookupFilter::new(cangjie_dictionary).apply(&mut full_code_candidates);
    assert_eq!(
        full_code_candidates[0].comment,
        selected_candidate_comment(full_code_on, 0)
    );
    assert_eq!(
        full_code_candidates[1].comment,
        selected_candidate_comment(full_code_on, 1)
    );

    let sentence = m14_case(&fixture, "enable_sentence_default", "ngohaigo");
    let sentence_dictionary =
        TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_oracle_comments(
            "sentence_lookup",
            &[selected_candidate_comment(sentence, 0)],
        ))
        .expect("M14 sentence source rows should parse");
    let sentence_translator = StaticTableTranslator::new([("ngo5hai6", "我係"), ("go3", "個")])
        .with_spelling_algebra(&jyutping_algebra)
        .with_sentence(true);
    let mut sentence_candidates = sentence_translator.translate("ngohaigo");
    DictionaryLookupFilter::new(sentence_dictionary).apply(&mut sentence_candidates);
    assert_eq!(
        sentence_candidates[0].text,
        selected_candidate_text(sentence, 0)
    );
    assert_eq!(
        sentence_candidates[0].comment,
        selected_candidate_comment(sentence, 0)
    );
}

#[test]
fn completion_prediction_and_enable_completion_parity() {
    let fixture = m14_completion_correction_fixture();
    let completion_ne = m14_case(&fixture, "completion_default", "ne");
    let dictionary = TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_oracle_comments(
        "completion_lookup",
        &[selected_candidate_comment(completion_ne, 0)],
    ))
    .expect("M14 completion source rows should parse");
    let translator = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_spelling_algebra(&["derive/\\d//".to_owned()])
        .with_comment_format(&["xform/^/\u{000c}/".to_owned()]);

    let mut candidates = translator.translate("ne");
    DictionaryLookupFilter::new(dictionary).apply(&mut candidates);

    assert_eq!(candidates.len(), candidate_count(completion_ne));
    assert_eq!(
        candidates[0].text,
        selected_candidate_text(completion_ne, 0)
    );
    assert_eq!(
        candidates[0].comment,
        selected_candidate_comment(completion_ne, 0)
    );
}

#[test]
fn correction_minimal_distance_and_m_abbreviation_parity() {
    let fixture = m14_completion_correction_fixture();
    let correction_enabled = m14_case(&fixture, "correction_enabled", "nri");
    let dictionary = TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_oracle_comments(
        "correction_lookup",
        &[selected_candidate_comment(correction_enabled, 0)],
    ))
    .expect("M14 correction source rows should parse");
    let translator = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_spelling_algebra(&[
            "derive/\\d//".to_owned(),
            "derive/^nei$/nri/correction".to_owned(),
        ])
        .with_comment_format(&["xform/^/\u{000c}/".to_owned()]);

    let mut candidates = translator.translate("nri");
    DictionaryLookupFilter::new(dictionary).apply(&mut candidates);

    assert_eq!(
        candidates[0].text,
        selected_candidate_text(correction_enabled, 0)
    );
    assert_eq!(
        candidates[0].comment,
        selected_candidate_comment(correction_enabled, 0)
    );
}

#[test]
#[ignore = "blocked: implement schema-menu behavior against the captured v1.1.2 schema-list surface and M16 browser UI assertion"]
fn schema_menu_hiding_parity() {
    panic!(
        "Yune implementation does not yet pass the captured TypeDuck v1.1.2 schema-menu surface"
    );
}

#[test]
#[ignore = "blocked: implement userdb pronunciation behavior against the captured v1.1.2 levers export golden"]
fn per_entry_userdb_pronunciation_parity() {
    panic!(
        "Yune implementation does not yet pass the captured TypeDuck v1.1.2 userdb export fixture"
    );
}
