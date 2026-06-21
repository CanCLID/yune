use std::{fs, path::Path};

use serde_json::Value;
use yune_core::{Engine, StaticTableTranslator, TableDictionary, Translator};

const FIXTURE_ROOT: &str = "tests/fixtures/upstream-1.17.0";
const BASIC_FIXTURE: &str = "bopomofo-basic.json";

#[test]
fn upstream_bopomofo_fixture_is_locked() {
    let fixture = fixture();
    assert_upstream_oracle_header(&fixture);
    assert_eq!(fixture["schema"], "bopomofo");
    assert_eq!(fixture["module_list"], serde_json::json!(["default"]));
    assert_eq!(fixture["capture"]["schema_data"], "rime/rime-bopomofo");
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "m19_bopomofo_curated_zhuyin_algebra"
    );
    assert_eq!(
        fixture["input_sequence"],
        serde_json::json!(["su3", "cl3", "j06", "w/4"])
    );
    assert_non_empty_array(&fixture, &["capture", "source_dictionary_rows"]);
    assert_non_empty_array(&fixture, &["capture", "source_vocabulary_rows"]);
    assert_non_empty_array(&fixture, &["capture", "speller_algebra_rules"]);
}

#[test]
fn yune_bopomofo_algebra_matches_upstream_first_page() {
    let fixture = fixture();
    let translator = bopomofo_translator(&fixture);

    for case in cases(&fixture) {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        let expected = selected_texts(case);
        let actual = translator
            .translate(input)
            .into_iter()
            .take(expected.len())
            .map(|candidate| candidate.text)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "first page should match for {input}");
        assert_eq!(
            case["commit_text_preview"].as_str(),
            expected.first().map(String::as_str)
        );
    }
}

#[test]
fn yune_bopomofo_engine_set_input_matches_tone_key_cases() {
    let fixture = fixture();
    for case in cases(&fixture) {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        let mut engine = bopomofo_engine(&fixture);
        engine.set_input(input);
        assert_eq!(
            engine
                .context()
                .candidates
                .iter()
                .take(selected_texts(case).len())
                .map(|candidate| candidate.text.clone())
                .collect::<Vec<_>>(),
            selected_texts(case),
            "engine candidates should match for {input}"
        );
    }
}

#[test]
#[ignore = "blocked: bopomofo digit tone keys require schema speller handling so digits are input, not numeric candidate selection"]
fn bopomofo_tone_digit_key_sequence_parity_is_blocked() {
    panic!("drive the schema-loaded speller path before enabling tone digit key-sequence parity");
}

#[test]
#[ignore = "blocked: bopomofo first-tone literal Space requires schema speller/use_space and fluency-editor behavior outside the core translator slice"]
fn bopomofo_first_tone_space_speller_parity_is_blocked() {
    panic!(
        "drive the schema-loaded speller/use_space path before enabling first-tone space parity"
    );
}

#[test]
#[ignore = "blocked: bopomofo phrase/sentence lattice parity depends on the upstream compiled language model tracked by M17"]
fn bopomofo_sentence_lattice_parity_is_blocked() {
    panic!("capture complete upstream language-model evidence before enabling Zhuyin sentences");
}

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_ROOT)
        .join(BASIC_FIXTURE);
    let fixture = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_str(&fixture).unwrap_or_else(|error| panic!("invalid JSON {path:?}: {error}"))
}

fn assert_upstream_oracle_header(fixture: &Value) {
    assert_eq!(fixture["oracle"]["engine"], "rime/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "1.17.0");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    );
}

fn cases(fixture: &Value) -> &[Value] {
    fixture["cases"]
        .as_array()
        .expect("fixture cases should be an array")
}

fn selected_texts(snapshot: &Value) -> Vec<String> {
    snapshot["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array")
        .iter()
        .map(|candidate| {
            candidate["text"]
                .as_str()
                .expect("candidate text should be a string")
                .to_owned()
        })
        .collect()
}

fn bopomofo_translator(fixture: &Value) -> StaticTableTranslator {
    let formulas = string_array(&fixture["capture"]["speller_algebra_rules"]);
    StaticTableTranslator::from_dictionary(
        TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            &dictionary_yaml_from_fixture_rows(
                "terra_pinyin",
                "by_weight",
                true,
                &fixture["capture"]["source_dictionary_rows"],
            ),
            std::iter::empty::<&str>(),
            |_| None,
            |name| {
                (name == "essay").then(|| text_rows(&fixture["capture"]["source_vocabulary_rows"]))
            },
        )
        .expect("bopomofo source rows should parse"),
    )
    .with_spelling_algebra(&formulas)
    .with_completion(true)
}

fn bopomofo_engine(fixture: &Value) -> Engine {
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.add_translator(bopomofo_translator(fixture));
    engine
}

fn dictionary_yaml_from_fixture_rows(
    name: &str,
    sort: &str,
    use_preset_vocabulary: bool,
    rows: &Value,
) -> String {
    format!(
        "---\nname: {name}\nversion: 'upstream-oracle-slice'\nsort: {sort}\nuse_preset_vocabulary: {use_preset_vocabulary}\n...\n\n{}\n",
        text_rows(rows)
    )
}

fn text_rows(rows: &Value) -> String {
    rows.as_array()
        .expect("rows should be an array")
        .iter()
        .map(|row| row.as_str().expect("row should be a string"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("value should be an array")
        .iter()
        .map(|value| value.as_str().expect("value should be a string").to_owned())
        .collect()
}

fn assert_non_empty_array(fixture: &Value, fields: &[&str]) {
    let value = fields.iter().fold(fixture, |value, field| &value[*field]);
    assert!(
        value.as_array().is_some_and(|array| !array.is_empty()),
        "fixture must include non-empty {}",
        fields.join(".")
    );
}
