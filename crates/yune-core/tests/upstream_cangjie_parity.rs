use std::{collections::BTreeMap, fs, path::Path};

use serde_json::Value;
use yune_core::{Engine, StaticTableTranslator, TableDictionary, Translator};

const FIXTURE_ROOT: &str = "tests/fixtures/upstream-1.17.0";
const BASIC_FIXTURE: &str = "cangjie5-basic.json";

#[test]
fn upstream_cangjie5_fixture_is_locked() {
    let fixture = fixture();
    assert_upstream_oracle_header(&fixture);
    assert_eq!(fixture["schema"], "cangjie5");
    assert_eq!(fixture["module_list"], serde_json::json!(["default"]));
    assert_eq!(fixture["capture"]["schema_data"], "rime/rime-cangjie");
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "m19_cangjie5_curated_table_codes"
    );
    assert_eq!(
        fixture["input_sequence"],
        serde_json::json!(["a", "am", "amd"])
    );
    assert_non_empty_array(
        &fixture,
        &[
            "capture",
            "source_dictionary_import_rows",
            "cangjie5.base.dict.yaml",
        ],
    );
    assert_non_empty_array(&fixture, &["capture", "source_vocabulary_rows"]);
}

#[test]
fn yune_cangjie5_exact_code_prefix_matches_upstream_table_rows() {
    let fixture = fixture();
    let translator = cangjie_translator(&fixture);

    for case in cases(&fixture) {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        let expected = leading_table_candidates(case);
        assert!(
            !expected.is_empty(),
            "fixture should include leading exact table candidates for {input}"
        );
        let actual = translator
            .translate(input)
            .into_iter()
            .take(expected.len())
            .map(|candidate| (candidate.text, empty_to_null(candidate.comment)))
            .collect::<Vec<_>>();
        assert_eq!(
            actual, expected,
            "leading exact table candidates should match for {input}"
        );
        assert_eq!(
            case["commit_text_preview"].as_str(),
            expected.first().map(|(text, _)| text.as_str()),
            "commit preview should be the highlighted cangjie table candidate for {input}"
        );
    }
}

#[test]
fn yune_cangjie5_engine_commits_exact_code_top_candidate() {
    let fixture = fixture();
    for case in cases(&fixture) {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        let mut engine = cangjie_engine(&fixture);
        engine
            .process_key_sequence(input)
            .expect("key sequence should parse");
        let expected = leading_table_candidates(case);
        assert_eq!(
            engine
                .context()
                .candidates
                .first()
                .map(|candidate| candidate.text.as_str()),
            expected.first().map(|(text, _)| text.as_str()),
            "top cangjie candidate should match for {input}"
        );
        let commits = engine
            .process_key_sequence("{space}")
            .expect("key sequence should parse");
        assert_eq!(
            commits,
            vec![expected[0].0.clone()],
            "space should commit the oracle top exact-code row for {input}"
        );
    }
}

#[test]
#[ignore = "blocked: cangjie5 full page phrase/table-encoder interleave and no-match classification need a broader upstream compiled-language-model fixture in M17"]
fn cangjie5_phrase_encoder_full_page_parity_is_blocked() {
    panic!("enable only after the full Cangjie phrase/table-encoder oracle slice is captured");
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

fn leading_table_candidates(case: &Value) -> Vec<(String, Option<String>)> {
    case["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array")
        .iter()
        .take_while(|candidate| candidate["comment"].is_null())
        .map(|candidate| {
            (
                candidate["text"]
                    .as_str()
                    .expect("candidate text should be a string")
                    .to_owned(),
                None,
            )
        })
        .collect()
}

fn cangjie_translator(fixture: &Value) -> StaticTableTranslator {
    StaticTableTranslator::from_dictionary(cangjie_dictionary(fixture))
        .with_completion(true)
        .with_sentence(true)
        .with_comment_format(&string_array(
            &fixture["capture"]["translator_comment_format"],
        ))
        .with_show_full_code(false)
}

fn cangjie_engine(fixture: &Value) -> Engine {
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.add_translator(cangjie_translator(fixture));
    engine
}

fn cangjie_dictionary(fixture: &Value) -> TableDictionary {
    let imports = fixture["capture"]["source_dictionary_import_rows"]
        .as_object()
        .expect("import rows should be an object")
        .iter()
        .map(|(file, rows)| (file.clone(), text_rows(rows)))
        .collect::<BTreeMap<_, _>>();
    TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        cangjie_main_yaml().as_str(),
        std::iter::empty::<&str>(),
        |name| {
            let file = format!("{name}.dict.yaml");
            imports
                .get(&file)
                .map(|rows| cangjie_import_yaml(name, rows))
        },
        |name| (name == "essay").then(|| text_rows(&fixture["capture"]["source_vocabulary_rows"])),
    )
    .expect("cangjie source rows should parse")
}

fn cangjie_main_yaml() -> String {
    "\
---
name: cangjie5
version: 'upstream-oracle-slice'
sort: by_weight
use_preset_vocabulary: true
max_phrase_length: 7
min_phrase_weight: 100
columns: [text, code, stem]
import_tables:
  - cangjie5.base
  - cangjie5.stem
  - cangjie5.extended
encoder:
  exclude_patterns:
    - '^x.*$'
    - '^z.*$'
  rules:
    - length_equal: 2
      formula: 'AaAzBaBbBz'
    - length_equal: 3
      formula: 'AaAzBaBzCz'
    - length_in_range: [4, 10]
      formula: 'AaBzCaYzZz'
  tail_anchor: \"'\"
...
"
    .to_owned()
}

fn cangjie_import_yaml(name: &str, rows: &str) -> String {
    let columns = if name.ends_with(".stem") {
        "columns: [text, code, stem]\n"
    } else {
        "columns: [text, code]\n"
    };
    format!("---\nname: {name}\nversion: 'upstream-oracle-slice'\n{columns}...\n\n{rows}\n")
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

fn empty_to_null(comment: String) -> Option<String> {
    (!comment.is_empty()).then_some(comment)
}

fn assert_non_empty_array(fixture: &Value, fields: &[&str]) {
    let value = fields.iter().fold(fixture, |value, field| &value[*field]);
    assert!(
        value.as_array().is_some_and(|array| !array.is_empty()),
        "fixture must include non-empty {}",
        fields.join(".")
    );
}
