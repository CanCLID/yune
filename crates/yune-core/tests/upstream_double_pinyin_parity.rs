use std::{fs, path::Path};

use serde_json::Value;
use yune_core::{Engine, StaticTableTranslator, TableDictionary, Translator};

const FIXTURE_ROOT: &str = "tests/fixtures/upstream-1.17.0";
const BASIC_FIXTURE: &str = "double-pinyin-basic.json";

#[test]
fn upstream_double_pinyin_fixture_is_locked() {
    let fixture = fixture();
    assert_upstream_oracle_header(&fixture);
    assert_eq!(fixture["schema"], "double_pinyin");
    assert_eq!(fixture["module_list"], serde_json::json!(["default"]));
    assert_eq!(fixture["capture"]["schema_data"], "rime/rime-double-pinyin");
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "m19_double_pinyin_curated_shuangpin_algebra"
    );
    assert_eq!(
        fixture["input_sequence"],
        serde_json::json!(["ni", "hk", "vs", "go"])
    );
    assert_non_empty_array(&fixture, &["capture", "source_dictionary_rows"]);
    assert_non_empty_array(&fixture, &["capture", "source_vocabulary_rows"]);
    assert_non_empty_array(&fixture, &["capture", "speller_algebra_rules"]);
}

#[test]
fn yune_double_pinyin_algebra_matches_upstream_first_page() {
    let fixture = fixture();
    let translator = double_pinyin_translator(&fixture);

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
            expected.first().map(String::as_str),
            "upstream commit preview should be the highlighted candidate for {input}"
        );
    }
}

#[test]
fn yune_double_pinyin_engine_actions_match_upstream_snapshots() {
    let fixture = fixture();

    let mut paging_engine = double_pinyin_engine(&fixture);
    paging_engine
        .process_key_sequence("ni")
        .expect("key sequence should parse");
    assert_snapshot_candidates(
        &paging_engine,
        snapshot(&fixture, "paging_first_input", "page_1"),
    );
    paging_engine
        .process_key_sequence("{Page_Down}")
        .expect("key sequence should parse");
    assert_snapshot_candidates(
        &paging_engine,
        snapshot(&fixture, "paging_first_input", "page_2"),
    );
    paging_engine
        .process_key_sequence("{Page_Up}")
        .expect("key sequence should parse");
    assert_snapshot_candidates(
        &paging_engine,
        snapshot(&fixture, "paging_first_input", "page_1_again"),
    );

    let mut select_engine = double_pinyin_engine(&fixture);
    let commits = select_engine
        .process_key_sequence("ni2")
        .expect("key sequence should parse");
    let select_snapshot = snapshot(&fixture, "select_first_input_second", "after_select_2");
    assert_eq!(commits, vec![commit_text(select_snapshot)]);

    let mut commit_engine = double_pinyin_engine(&fixture);
    commit_engine
        .process_key_sequence("ni")
        .expect("key sequence should parse");
    let commits = commit_engine
        .process_key_sequence("{space}")
        .expect("key sequence should parse");
    let commit_snapshot = snapshot(&fixture, "commit_first_input_space", "after_space");
    assert_eq!(commits, vec![commit_text(commit_snapshot)]);
}

#[test]
#[ignore = "blocked: Shuangpin phrase/sentence lattice parity depends on the upstream compiled language model tracked by M17"]
fn double_pinyin_sentence_lattice_parity_is_blocked() {
    panic!("capture complete upstream language-model evidence before enabling Shuangpin sentences");
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

fn snapshot<'a>(fixture: &'a Value, scenario: &str, label: &str) -> &'a Value {
    fixture["snapshots"]
        .as_array()
        .expect("fixture snapshots should be an array")
        .iter()
        .find(|snapshot| snapshot["scenario"] == scenario && snapshot["label"] == label)
        .unwrap_or_else(|| panic!("missing snapshot {scenario}/{label}"))
}

fn commit_text(snapshot: &Value) -> String {
    snapshot["commit_text"]
        .as_str()
        .expect("snapshot should contain committed text")
        .to_owned()
}

fn double_pinyin_translator(fixture: &Value) -> StaticTableTranslator {
    let formulas = string_array(&fixture["capture"]["speller_algebra_rules"]);
    StaticTableTranslator::from_dictionary(
        TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            &dictionary_yaml_from_fixture_rows(
                "luna_pinyin",
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
        .expect("double-pinyin source rows should parse"),
    )
    .with_spelling_algebra(&formulas)
    .with_completion(true)
}

fn double_pinyin_engine(fixture: &Value) -> Engine {
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.add_translator(double_pinyin_translator(fixture));
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

fn assert_snapshot_candidates(engine: &Engine, expected_snapshot: &Value) {
    let expected_page_size = expected_snapshot["page_size"]
        .as_u64()
        .expect("page size should be numeric") as usize;
    let expected_page_no = expected_snapshot["page_no"]
        .as_u64()
        .expect("page number should be numeric") as usize;
    let expected_highlighted = expected_snapshot["highlighted_candidate_index"]
        .as_u64()
        .expect("highlighted index should be numeric") as usize;
    assert_eq!(
        engine.context().highlighted % expected_page_size,
        expected_highlighted
    );
    assert_eq!(
        engine.context().highlighted / expected_page_size,
        expected_page_no
    );
    assert_eq!(
        current_page_texts(engine, expected_page_size)
            .into_iter()
            .take(selected_texts(expected_snapshot).len())
            .collect::<Vec<_>>(),
        selected_texts(expected_snapshot)
    );
}

fn current_page_texts(engine: &Engine, page_size: usize) -> Vec<String> {
    let page_start = (engine.context().highlighted / page_size) * page_size;
    engine
        .context()
        .candidates
        .iter()
        .skip(page_start)
        .take(page_size)
        .map(|candidate| candidate.text.clone())
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
