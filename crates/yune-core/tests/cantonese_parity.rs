use serde_json::Value;
use yune_core::{
    Candidate, CandidateFilter, CandidateSource, DictionaryLookupFilter, Engine,
    ReverseLookupTranslator, SchemaListTranslator, StaticTableTranslator, Status, TableDictionary,
    Translator, UserDb,
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
const M21_SENTENCE_COMPOSITION_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-m21-sentence-composition.json");
const FORK_PARITY_01_REAL_DICTIONARY_FUZZY_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-fork-parity-01-real-dictionary-fuzzy.json");
const FORK_PARITY_02_PREFER_USER_PHRASE_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-fork-parity-02-prefer-user-phrase.json");
const FORK_PARITY_06_LETTER_TO_TONE_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-fork-parity-06-letter-to-tone.json");
const FORK_PARITY_07_STATE_LABELS_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-fork-parity-07-state-labels.json");
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

fn m21_sentence_composition_fixture() -> Value {
    serde_json::from_str(M21_SENTENCE_COMPOSITION_ORACLE)
        .expect("TypeDuck v1.1.2 M21 sentence-composition fixture should be valid JSON")
}

fn fork_parity_01_real_dictionary_fuzzy_fixture() -> Value {
    serde_json::from_str(FORK_PARITY_01_REAL_DICTIONARY_FUZZY_ORACLE)
        .expect("TypeDuck v1.1.2 FORK-PARITY-01 fixture should be valid JSON")
}

fn fork_parity_02_prefer_user_phrase_fixture() -> Value {
    serde_json::from_str(FORK_PARITY_02_PREFER_USER_PHRASE_ORACLE)
        .expect("TypeDuck v1.1.2 FORK-PARITY-02 fixture should be valid JSON")
}

fn fork_parity_06_letter_to_tone_fixture() -> Value {
    serde_json::from_str(FORK_PARITY_06_LETTER_TO_TONE_ORACLE)
        .expect("TypeDuck v1.1.2 FORK-PARITY-06 fixture should be valid JSON")
}

fn fork_parity_07_state_labels_fixture() -> Value {
    serde_json::from_str(FORK_PARITY_07_STATE_LABELS_ORACLE)
        .expect("TypeDuck v1.1.2 FORK-PARITY-07 fixture should be valid JSON")
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
fn typeduck_v112_m21_sentence_composition_fixture_is_locked() {
    let fixture = m21_sentence_composition_fixture();
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
    assert_eq!(
        fixture["capture"]["input_sequence"],
        serde_json::json!([
            "loengnincin",
            "leoicijyu",
            "ngohaigo",
            "loengjathau",
            "geijatcin",
            "gamjatheoi"
        ])
    );

    let expected_top = [
        ("loengnincin", "兩年前", "loeng nin cin"),
        ("leoicijyu", "類似如", "leoi ci jyu"),
        ("ngohaigo", "我係個", "ngo hai go"),
        ("loengjathau", "兩日後", "loeng jat hau"),
        ("geijatcin", "機日前", "gei jat cin"),
        ("gamjatheoi", "今日去", "gam jat heoi"),
    ];
    for (input, text, preedit) in expected_top {
        let case = m21_sentence_case(&fixture, input);
        assert_eq!(case["preedit"], preedit, "input {input}");
        assert_eq!(case["commit_text_preview"], text, "input {input}");
        assert_eq!(selected_candidate_text(case, 0), text, "input {input}");
        assert!(selected_candidate_comment(case, 0).contains(",composition,"));
    }
}

#[test]
fn typeduck_v112_fork_parity_06_letter_to_tone_fixture_is_locked() {
    let fixture = fork_parity_06_letter_to_tone_fixture();
    assert_eq!(fixture["oracle"]["engine"], "TypeDuck-HK/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "v1.1.2");
    assert_eq!(fixture["schema"], "jyut6ping3_mobile");
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_letter_to_tone_preedit"
    );
    assert_eq!(
        fixture["capture"]["oracle_observable_surface"],
        "RimeGetContext composition preedit maps TypeDuck v/x/q tone letters to Jyutping tone digits while RimeGetInput preserves raw letters"
    );

    let expected = [
        ("neiv", "nei1", 1),
        ("neivv", "nei4", 8),
        ("neix", "neix", 0),
        ("neixx", "nei5", 8),
        ("neiq", "neiq", 20),
        ("neiqq", "nei6", 3),
    ];
    let cases = fixture["cases"]
        .as_array()
        .expect("FORK-PARITY-06 cases should be an array");
    assert_eq!(cases.len(), expected.len());
    for (input, preedit, selected_count) in expected {
        let case = fork_parity_06_case(&fixture, input);
        assert_eq!(case["input"], input);
        assert_eq!(case["rime_get_input"], input);
        assert_eq!(case["preedit"], preedit);
        assert_eq!(candidate_count(case), selected_count);
        assert_eq!(case["schema_id"], "jyut6ping3_mobile");
        assert_eq!(case["is_composing"], true);
    }
}

#[test]
fn typeduck_v112_fork_parity_07_state_labels_fixture_is_locked() {
    let fixture = fork_parity_07_state_labels_fixture();
    assert_eq!(fixture["oracle"]["engine"], "TypeDuck-HK/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "v1.1.2");
    assert_eq!(fixture["schema"], "jyut6ping3_mobile");
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_full_shape_state_labels"
    );
    assert_eq!(
        fixture["capture"]["oracle_observable_surface"],
        "RimeGetStateLabel full_shape returns TypeDuck Traditional labels"
    );
    assert_eq!(
        fixture["capture"]["deployed_schema_file"],
        "jyut6ping3_mobile.schema.yaml"
    );

    let cases = fixture["cases"]
        .as_array()
        .expect("FORK-PARITY-07 cases should be an array");
    assert_eq!(cases.len(), 1);
    let case = &cases[0];
    assert_eq!(case["variant"], "state_labels_mobile");
    assert_eq!(case["schema_id"], "jyut6ping3_mobile");

    let labels = case["labels"]
        .as_array()
        .expect("FORK-PARITY-07 labels should be an array");
    assert_eq!(labels.len(), 2);
    for (row, state, label, abbrev, upstream_label) in [
        (
            &labels[0],
            0,
            "\u{534a}\u{5f62}",
            "\u{534a}",
            "\u{534a}\u{89d2}",
        ),
        (
            &labels[1],
            1,
            "\u{5168}\u{5f62}",
            "\u{5168}",
            "\u{5168}\u{89d2}",
        ),
    ] {
        assert_eq!(row["option"], "full_shape");
        assert_eq!(row["state"], state);
        assert_eq!(row["label"], label);
        assert_eq!(row["abbreviated_label"], abbrev);
        assert_eq!(row["abbreviated_length"], abbrev.len());
        assert_ne!(row["label"], upstream_label);
    }
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
fn typeduck_v112_fork_parity_01_real_dictionary_fuzzy_fixture_is_locked() {
    let fixture = fork_parity_01_real_dictionary_fuzzy_fixture();
    assert_eq!(fixture["oracle"]["engine"], "TypeDuck-HK/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "v1.1.2");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "74cb52b78fb2411137a7643f6c8bc6517acfde69"
    );
    assert_eq!(fixture["schema"], "jyut6ping3_mobile");
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_real_mobile_translator_and_scolar_lookup_fuzzy"
    );
    assert_eq!(
        fixture["capture"]["translator_dictionary_file"],
        "TypeDuck-HK/schema/jyut6ping3.dict.yaml"
    );
    assert_eq!(
        fixture["capture"]["lookup_dictionary_file"],
        "TypeDuck-HK/schema/jyut6ping3_scolar.dict.yaml"
    );
    assert_eq!(
        fixture["capture"]["source_row_counts"]["translator_dictionary"], 127144,
        "FORK-PARITY-01 must stay tied to the real production-sized translator dictionary"
    );
    assert_eq!(
        fixture["capture"]["source_row_counts"]["lookup_dictionary"], 127144,
        "FORK-PARITY-01 must stay tied to the real production-sized lookup dictionary"
    );
    assert!(fixture["capture"]["speller_algebra_rules"]
        .as_array()
        .expect("speller algebra should be captured")
        .iter()
        .any(|rule| rule == "derive/^ng(?=\\d)/m/"));

    let case = &fixture["cases"]
        .as_array()
        .expect("cases should be an array")[0];
    assert_eq!(case["input"], "m");
    assert_eq!(case["preedit"], "m");
    assert_eq!(case["commit_text_preview"], "\u{5514}");
    let candidates = case["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array");
    assert_eq!(candidates[0]["text"], "\u{5514}");
    assert!(candidates[0]["comment"]
        .as_str()
        .is_some_and(|comment| comment.contains(",m4,")));
    assert_eq!(candidates[1]["text"], "\u{4e94}");
    assert!(candidates[1]["comment"]
        .as_str()
        .is_some_and(|comment| comment.contains(",ng5,")));
}

#[test]
fn typeduck_v112_fork_parity_02_prefer_user_phrase_fixture_is_locked() {
    let fixture = fork_parity_02_prefer_user_phrase_fixture();
    assert_eq!(fixture["oracle"]["engine"], "TypeDuck-HK/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "v1.1.2");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "74cb52b78fb2411137a7643f6c8bc6517acfde69"
    );
    assert_eq!(fixture["schema"], "jyut6ping3_mobile");
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default", "dictionary_lookup", "levers"])
    );
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_prefer_user_phrase_weighted_gate"
    );
    assert_eq!(
        fixture["cases"]
            .as_array()
            .expect("FORK-PARITY-02 cases should be an array")
            .len(),
        3
    );

    let low = prefer_user_phrase_case(&fixture, "equal_code_low_commit_user_phrase");
    assert_schema_custom_enables_userdb(low);
    assert_eq!(low["import_text"], "YUNELOW\tnei5\t1\n");
    assert_eq!(low["probe"]["import_function_found"], true);
    assert_eq!(low["probe"]["import_return"], 1);
    let low_capture = prefer_user_phrase_capture(low);
    assert_eq!(low_capture["input"], "nei");
    assert_eq!(low_capture["preedit"], "nei");
    assert_eq!(low_capture["commit_text_preview"], "\u{4f60}");
    assert_eq!(selected_candidate_text(low_capture, 0), "\u{4f60}");
    assert_eq!(selected_candidate_text(low_capture, 8), "YUNELOW");
    assert_eq!(selected_candidate_comment(low_capture, 8), "\u{000c}nei5");

    let high = prefer_user_phrase_case(&fixture, "equal_code_high_commit_user_phrase");
    assert_schema_custom_enables_userdb(high);
    assert_eq!(high["import_text"], "YUNEHIGH\tnei5\t100000000\n");
    assert_eq!(high["probe"]["import_return"], 1);
    let high_capture = prefer_user_phrase_capture(high);
    assert_eq!(high_capture["input"], "nei");
    assert_eq!(selected_candidate_text(high_capture, 0), "\u{4f60}");
    assert_eq!(selected_candidate_text(high_capture, 4), "YUNEHIGH");
    assert_eq!(selected_candidate_comment(high_capture, 4), "\u{000c}nei5");

    let longer = prefer_user_phrase_case(&fixture, "longer_code_user_phrase");
    assert_schema_custom_enables_userdb(longer);
    assert_eq!(longer["import_text"], "YUNELONG\tnei5 hou2\t1\n");
    assert_eq!(longer["probe"]["import_return"], 1);
    let longer_capture = prefer_user_phrase_capture(longer);
    assert_eq!(longer_capture["input"], "neihou");
    assert_eq!(
        selected_candidate_text(longer_capture, 0),
        "\u{4f60}\u{597d}"
    );
    assert_eq!(selected_candidate_text(longer_capture, 1), "YUNELONG");
    assert_eq!(
        selected_candidate_comment(longer_capture, 1),
        "\u{000c}nei5 hou2"
    );
}

#[test]
fn yune_userdb_same_code_low_weight_phrase_does_not_preempt_table_candidate() {
    let fixture = fork_parity_02_prefer_user_phrase_fixture();
    assert_yune_prefer_user_phrase_case(
        prefer_user_phrase_case(&fixture, "equal_code_low_commit_user_phrase"),
        StaticTableTranslator::new([
            ("nei", "\u{4f60}"),
            ("nei", "\u{5462}"),
            ("nei", "\u{5c3c}"),
            ("nei", "\u{59ae}"),
            ("nei", "\u{5f4c}"),
            ("nei", "\u{59b3}"),
            ("nei", "\u{60a8}"),
            ("nei", "\u{81a9}"),
            ("nei", "\u{990c}"),
        ]),
        "YUNELOW",
    );
    assert_yune_prefer_user_phrase_case(
        prefer_user_phrase_case(&fixture, "equal_code_high_commit_user_phrase"),
        StaticTableTranslator::new([
            ("nei", "\u{4f60}"),
            ("nei", "\u{5462}"),
            ("nei", "\u{5c3c}"),
            ("nei", "\u{59ae}"),
            ("nei", "\u{5f4c}"),
            ("nei", "\u{59b3}"),
            ("nei", "\u{60a8}"),
            ("nei", "\u{81a9}"),
            ("nei", "\u{990c}"),
        ]),
        "YUNEHIGH",
    );
    assert_yune_prefer_user_phrase_case(
        prefer_user_phrase_case(&fixture, "longer_code_user_phrase"),
        StaticTableTranslator::new([
            ("neihou", "\u{4f60}\u{597d}"),
            ("nei", "\u{4f60}"),
            ("nei", "\u{5462}"),
            ("nei", "\u{5c3c}"),
        ]),
        "YUNELONG",
    );
}

fn assert_yune_prefer_user_phrase_case(
    case: &Value,
    translator: StaticTableTranslator,
    imported_text: &str,
) {
    let oracle_capture = prefer_user_phrase_capture(case);
    let oracle_user_phrase_index = candidate_index(oracle_capture, imported_text);
    let (text, code, commits) = imported_userdb_entry(case);

    let mut userdb = UserDb::default();
    userdb.learn_entry(
        code,
        text,
        commits,
        (f64::from(commits) + 1.0) / 100_000_000.0,
        0,
    );

    let mut engine = Engine::new();
    engine.add_translator(translator);
    engine.set_userdb(userdb);
    engine.set_input(
        oracle_capture["input"]
            .as_str()
            .expect("oracle capture input should be a string"),
    );

    let candidates = &engine.context().candidates;
    assert_eq!(
        candidates[0].text,
        selected_candidate_text(oracle_capture, 0)
    );
    let yune_user_phrase_index = candidates
        .iter()
        .position(|candidate| candidate.text == imported_text)
        .expect("Yune should expose the imported user phrase");
    assert_eq!(yune_user_phrase_index, oracle_user_phrase_index);
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
        preedit: None,
        source: CandidateSource::Table,
        quality: 1.0,
    }];

    DictionaryLookupFilter::new(dictionary.clone()).apply(&mut candidates);

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

fn prefer_user_phrase_case<'a>(fixture: &'a Value, variant: &str) -> &'a Value {
    fixture["cases"]
        .as_array()
        .expect("FORK-PARITY-02 cases should be an array")
        .iter()
        .find(|case| case["variant"] == variant)
        .unwrap_or_else(|| panic!("FORK-PARITY-02 fixture should capture variant {variant}"))
}

fn assert_schema_custom_enables_userdb(case: &Value) {
    assert!(case["schema_custom_patch_lines"]
        .as_array()
        .expect("schema custom patch lines should be captured")
        .iter()
        .any(|line| line == "translator/enable_user_dict: true"));
}

fn prefer_user_phrase_capture(case: &Value) -> &Value {
    &case["probe"]["captures"]
        .as_array()
        .expect("FORK-PARITY-02 captures should be an array")[0]
}

fn candidate_index(case: &Value, text: &str) -> usize {
    case["selected_candidates"]
        .as_array()
        .expect("selected_candidates should be an array")
        .iter()
        .position(|candidate| candidate["text"] == text)
        .unwrap_or_else(|| panic!("candidate {text} should be captured"))
}

fn imported_userdb_entry(case: &Value) -> (&str, &str, i32) {
    let import_text = case["import_text"]
        .as_str()
        .expect("import text should be a string")
        .trim_end();
    let fields = import_text.split('\t').collect::<Vec<_>>();
    assert_eq!(fields.len(), 3, "import text should be phrase/code/commits");
    let commits = fields[2]
        .parse::<i32>()
        .expect("import commits should fit in i32");
    (fields[0], fields[1], commits)
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

fn fork_parity_06_case<'a>(fixture: &'a Value, input: &str) -> &'a Value {
    fixture["cases"]
        .as_array()
        .expect("FORK-PARITY-06 cases should be an array")
        .iter()
        .find(|case| case["variant"] == "letter_to_tone_mobile" && case["input"] == input)
        .unwrap_or_else(|| panic!("FORK-PARITY-06 fixture should capture input {input}"))
}

fn m21_sentence_case<'a>(fixture: &'a Value, input: &str) -> &'a Value {
    fixture["cases"]
        .as_array()
        .expect("M21 sentence-composition cases should be an array")
        .iter()
        .find(|case| case["input"] == input)
        .unwrap_or_else(|| panic!("M21 sentence-composition fixture should capture {input}"))
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

fn typeduck_public_schema_asset(relative_path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/typeduck-web/source/public/schema")
        .join(relative_path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn jyut6ping3_mobile_spelling_algebra() -> Vec<String> {
    [
        "derive/^ng(?=[aeiou])//",
        "derive/^(?=[aeiou])/ng/",
        "derive/^n(?!g)/l/",
        "derive/^ng(?=\\d)/m/",
        "derive/^(g|k)w(?=o)/$1/",
        "derive/^jy?(?=[aeiou])/y/",
        "derive/^jyu/ju/",
        "derive/yu(?!ng|k)/y/",
        "derive/(g|k)u(?!ng|k)/$1wu/",
        "derive/^([zcs])/$1h/",
        "derive/eoi(?=\\d)/eoy/",
        "derive/eo/oe/",
        "derive/oe/eo/",
        "derive/aa(?=\\d)/a/",
        "derive/\\d//",
        "abbrev/^([a-z]).+$/$1/",
        "xform/1/v/",
        "xform/4/vv/",
        "xform/2/x/",
        "xform/5/xx/",
        "xform/3/q/",
        "xform/6/qq/",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[test]
fn m21_sentence_composition_matches_typeduck_v112_real_dictionary_goldens() {
    let fixture = m21_sentence_composition_fixture();
    let translator_dictionary = TableDictionary::parse_rime_dict_yaml(
        &typeduck_public_schema_asset("jyut6ping3.dict.yaml"),
    )
    .expect("production jyut6ping3 dictionary should parse");
    let lookup_dictionary = TableDictionary::parse_typeduck_lookup_dict_yaml(
        &typeduck_public_schema_asset("jyut6ping3_scolar.dict.yaml"),
    )
    .expect("production jyut6ping3_scolar lookup dictionary should parse");
    let translator = StaticTableTranslator::from_dictionary(translator_dictionary)
        .with_completion(true)
        .with_sentence(true)
        .with_spelling_algebra(&jyut6ping3_mobile_spelling_algebra())
        .with_comment_format(&["xform/^/\u{000c}/".to_owned()]);

    for input in [
        "loengnincin",
        "leoicijyu",
        "ngohaigo",
        "loengjathau",
        "geijatcin",
        "gamjatheoi",
    ] {
        let expected = m21_sentence_case(&fixture, input);
        let mut candidates = translator.translate(input);
        DictionaryLookupFilter::new(lookup_dictionary.clone()).apply(&mut candidates);
        let first = candidates
            .first()
            .unwrap_or_else(|| panic!("input {input} should produce candidates"));
        assert_eq!(
            first.source,
            CandidateSource::Sentence,
            "input {input} should stay on the dictionary sentence path"
        );
        assert_eq!(
            first.text,
            selected_candidate_text(expected, 0),
            "input {input}"
        );
        let expected_composition_record = selected_candidate_comment(expected, 0)
            .split('\r')
            .take(2)
            .collect::<Vec<_>>()
            .join("\r");
        assert!(
            first.comment.starts_with(&expected_composition_record),
            "input {input} should preserve the oracle composition row; got {:?}",
            first.comment
        );
    }
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
    let mgoi_enabled = m14_case(&fixture, "correction_enabled", "mgoi");
    let dictionary = TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_oracle_comments(
        "correction_lookup",
        &[
            selected_candidate_comment(correction_enabled, 0),
            selected_candidate_comment(mgoi_enabled, 0),
        ],
    ))
    .expect("M14 correction source rows should parse");
    let translator = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_spelling_algebra(&[
            "derive/\\d//".to_owned(),
            "derive/^nei$/nri/correction".to_owned(),
        ])
        .with_comment_format(&["xform/^/\u{000c}/".to_owned()]);

    let mut candidates = translator.translate("nri");
    DictionaryLookupFilter::new(dictionary.clone()).apply(&mut candidates);

    assert_eq!(
        candidates[0].text,
        selected_candidate_text(correction_enabled, 0)
    );
    assert_eq!(
        candidates[0].comment,
        selected_candidate_comment(correction_enabled, 0)
    );

    let mut mgoi_candidates = translator.translate("mgoi");
    DictionaryLookupFilter::new(dictionary).apply(&mut mgoi_candidates);

    assert_eq!(
        mgoi_candidates[0].text,
        selected_candidate_text(mgoi_enabled, 0)
    );
    assert_eq!(
        mgoi_candidates[0].comment,
        selected_candidate_comment(mgoi_enabled, 0)
    );
}

#[test]
fn letter_to_tone_preedit_uses_typeduck_oracle_rows_without_rewriting_input() {
    let fixture = fork_parity_06_letter_to_tone_fixture();
    let lookup_inputs = ["neiv", "neivv", "neixx", "neiqq"];
    let lookup_cases = lookup_inputs
        .iter()
        .map(|input| fork_parity_06_case(&fixture, input))
        .collect::<Vec<_>>();
    let comments = lookup_cases
        .iter()
        .flat_map(|case| {
            case["selected_candidates"]
                .as_array()
                .expect("selected_candidates should be an array")
        })
        .map(|candidate| {
            candidate["comment"]
                .as_str()
                .expect("candidate comment should be a string")
        })
        .collect::<Vec<_>>();
    let dictionary = TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_oracle_comments(
        "letter_to_tone_lookup",
        &comments,
    ))
    .expect("FORK-PARITY-06 source rows should parse");
    let tone_to_letter = vec![
        "xform/1/v/".to_owned(),
        "xform/4/vv/".to_owned(),
        "xform/2/x/".to_owned(),
        "xform/5/xx/".to_owned(),
        "xform/3/q/".to_owned(),
        "xform/6/qq/".to_owned(),
    ];
    let letter_to_tone = vec![
        "xform/([aeiouymngptk])vv/${1}4/".to_owned(),
        "xform/([aeiouymngptk])xx/${1}5/".to_owned(),
        "xform/([aeiouymngptk])qq/${1}6/".to_owned(),
        "xform/([aeiouymngptk])v/${1}1/".to_owned(),
        "xform/([aeiouymngptk])x/${1}2/".to_owned(),
        "xform/([aeiouymngptk])q/${1}3/".to_owned(),
    ];
    let translator = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_spelling_algebra(&tone_to_letter)
        .with_comment_format(&["xform/^/\u{000c}/".to_owned()])
        .with_preedit_format(&letter_to_tone);

    for case in lookup_cases {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        let mut candidates = translator.translate(input);
        DictionaryLookupFilter::new(dictionary.clone()).apply(&mut candidates);

        assert_eq!(candidates.len(), candidate_count(case), "input {input}");
        let expected_index = candidates
            .iter()
            .position(|candidate| {
                candidate.text == selected_candidate_text(case, 0)
                    && candidate.comment == selected_candidate_comment(case, 0)
            })
            .unwrap_or_else(|| panic!("input {input} should include the captured top row"));
        assert_eq!(
            candidates[expected_index].preedit.as_deref(),
            case["preedit"].as_str(),
            "input {input}"
        );
    }

    assert!(translator.translate("neix").is_empty());
}

#[test]
fn schema_menu_hiding_uses_typeduck_schema_list_surface() {
    let fixture = m14_schema_menu_fixture();
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_schema_list_emitted_surface"
    );
    assert_eq!(
        fixture["capture"]["oracle_observable_surface"],
        "RimeGetSchemaList emits selected schema rows; hide_lone_schema/hide_caret are switcher/frontend decoration, not candidate ABI rows"
    );
    assert_eq!(
        fixture["capture"]["m16_delivery"],
        "assert TypeDuck-Web UI behavior against one-schema and multi-schema emitted lists"
    );

    let one_schema = m14_case(&fixture, "one_schema_default", "");
    let multi_schema = m14_case(&fixture, "mobile_multi_schema_custom", "");
    assert_eq!(one_schema["rime_get_schema_list"], true);
    assert_eq!(multi_schema["rime_get_schema_list"], true);
    assert_eq!(one_schema["schemas"].as_array().expect("schemas").len(), 1);
    assert!(
        multi_schema["schemas"].as_array().expect("schemas").len() > 1,
        "multi-schema oracle case should keep the switcher visible"
    );

    let one_schema_row = &one_schema["schemas"]
        .as_array()
        .expect("one-schema fixture should contain schema rows")[0];
    let one_schema_status = schema_status(one_schema_row);
    // The TypeDuck oracle exposes schema-list cardinality at the ABI; the
    // frontend candidate/menu behavior is asserted through the rime-api
    // frontend-style test for this fork-parity item.
    let hidden_candidates = SchemaListTranslator::new(Vec::<(String, String)>::new())
        .with_hide_lone_schema(true)
        .translate_with_status("x", &one_schema_status);
    assert!(
        hidden_candidates.is_empty(),
        "Yune should suppress the schema-list candidate when the fork option hides a lone schema"
    );

    let visible_one_schema_candidates = SchemaListTranslator::new(Vec::<(String, String)>::new())
        .translate_with_status("x", &one_schema_status);
    assert_eq!(visible_one_schema_candidates.len(), 1);
    assert_eq!(
        visible_one_schema_candidates[0].text,
        one_schema_row["name"]
            .as_str()
            .expect("schema name should be captured")
    );

    let multi_schema_rows = multi_schema["schemas"]
        .as_array()
        .expect("multi-schema fixture should contain schema rows");
    let multi_schema_status = schema_status(&multi_schema_rows[0]);
    let multi_schema_entries = multi_schema_rows
        .iter()
        .skip(1)
        .map(|schema| {
            (
                schema["schema_id"]
                    .as_str()
                    .expect("schema id should be captured"),
                schema["name"]
                    .as_str()
                    .expect("schema name should be captured"),
            )
        })
        .collect::<Vec<_>>();
    let multi_candidates = SchemaListTranslator::new(multi_schema_entries)
        .with_hide_lone_schema(true)
        .translate_with_status("x", &multi_schema_status);
    assert_eq!(multi_candidates.len(), multi_schema_rows.len());
    assert!(multi_candidates
        .iter()
        .all(|candidate| candidate.source == CandidateSource::Schema));
}

fn schema_status(schema: &Value) -> Status {
    Status {
        schema_id: schema["schema_id"]
            .as_str()
            .expect("schema id should be captured")
            .to_owned(),
        schema_name: schema["name"]
            .as_str()
            .expect("schema name should be captured")
            .to_owned(),
        ..Status::default()
    }
}

#[test]
fn per_entry_userdb_pronunciation_parity() {
    let fixture = m14_userdb_fixture();
    let case = &fixture["cases"]
        .as_array()
        .expect("userdb cases should be an array")[0];
    assert_eq!(case["training_input"], "nei");
    let probe = &case["probe"];
    assert_eq!(probe["training_input"], "nei");
    assert_eq!(probe["commit_text"], "\u{4f60}");
    let export_text = probe["export_text"]
        .as_str()
        .expect("userdb export text should be captured");
    assert!(
        export_text.contains("\u{4f60}\tnei5\t1"),
        "TypeDuck v1.1.2 oracle should export the selected row with its full pronunciation code"
    );

    let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
        "---\n\
name: jyut6ping3_lookup\n\
version: '1'\n\
sort: original\n\
columns: [text, code, weight, stem, source, english]\n\
...\n\
\n\
\u{4f60}\tnei5\t1\t0\toth\tyou\n",
    )
    .expect("lookup dictionary should parse");
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("nei", "\u{4f60}")]));
    engine.add_filter(DictionaryLookupFilter::new(lookup_dictionary));
    engine.set_input("nei");

    assert_eq!(engine.commit_composition(), Some("\u{4f60}".to_owned()));
    let event = engine
        .take_pending_userdb_learning()
        .expect("classic commit should stage userdb learning");
    assert_eq!(event.input, "nei");
    assert_eq!(event.code, "nei5");

    let mut userdb = UserDb::default();
    userdb.record_commit(&event);
    assert_eq!(userdb.entries()[0].code, "nei5 ");
    assert_eq!(userdb.entries()[0].text, "\u{4f60}");
}
