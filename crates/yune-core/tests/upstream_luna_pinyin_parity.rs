use std::{fs, path::Path};

use serde_json::Value;
use yune_core::{
    CandidateSource, Engine, PunctuationDefinition, PunctuationProcessor, PunctuationTranslator,
    ReverseLookupTranslator, SimplifierFilter, StaticTableTranslator, TableDictionary, Translator,
};

const FIXTURE_ROOT: &str = "tests/fixtures/upstream-1.17.0";
const BASIC_FIXTURE: &str = "luna-pinyin-basic.json";
const SELECTION_FIXTURE: &str = "luna-pinyin-selection.json";
const ACTIONS_FIXTURE: &str = "luna-pinyin-actions.json";
const REVERSE_LOOKUP_FIXTURE: &str = "luna-pinyin-reverse-lookup.json";
const PUNCTUATION_FIXTURE: &str = "luna-pinyin-punctuation.json";
const M18_PUNCTUATION_FIXTURE: &str = "m18-punctuation-processor.json";
const OPTIONS_FIXTURE: &str = "luna-pinyin-options.json";
const SENTENCE_FIXTURE: &str = "luna-pinyin-sentence.json";
const LATTICE_FIXTURE: &str = "luna-pinyin-lattice.json";

#[test]
fn upstream_luna_pinyin_fixture_is_locked() {
    let fixture = fixture(BASIC_FIXTURE);
    assert_upstream_oracle_header(&fixture);
    assert_eq!(fixture["schema"], "luna_pinyin");
    assert_eq!(fixture["module_list"], serde_json::json!(["default"]));
    assert_eq!(fixture["capture"]["schema_data"], "rime/rime-luna-pinyin");
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "curated_oracle_winners"
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
    assert_eq!(inputs, ["ni", "hao", "zhong", "guo", "zhongguo"]);

    for case in cases {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        assert_eq!(case["schema_id"], "luna_pinyin");
        assert_eq!(case["schema_name"], "\u{6719}\u{6708}\u{62fc}\u{97f3}");
        assert_eq!(case["is_composing"], true);
        assert_eq!(case["is_ascii_mode"], false);
        assert_eq!(case["highlighted_candidate_index"], 0);
        assert_eq!(case["page_size"], 5);
        assert_eq!(case["page_no"], 0);
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
        assert_eq!(
            case["commit_text_preview"], selected_candidates[0]["text"],
            "commit preview should match the highlighted upstream candidate for {input}"
        );
    }

    let zhongguo = cases
        .iter()
        .find(|case| case["input"] == "zhongguo")
        .expect("zhongguo should be captured");
    assert_eq!(zhongguo["preedit"], "zhong guo");
    assert_eq!(
        zhongguo["selected_candidates"][0]["text"],
        "\u{4e2d}\u{570b}"
    );
}

#[test]
fn yune_table_translator_matches_upstream_luna_pinyin_single_code_first_page() {
    let fixture = fixture(BASIC_FIXTURE);
    let translator = StaticTableTranslator::from_dictionary(luna_dictionary_from_rows(
        &fixture["capture"]["source_dictionary_rows"],
        &fixture["capture"]["source_vocabulary_rows"],
    ));

    for case in cases(&fixture)
        .iter()
        .filter(|case| case["input"] != "zhongguo")
    {
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
    }
}

#[test]
fn zhongguo_phrase_mechanics_matches_upstream_sentence_fixture() {
    let fixture = fixture(SENTENCE_FIXTURE);
    assert_upstream_oracle_header(&fixture);
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "m17_upstream_luna_sentence_language_model"
    );

    let dictionary = m17_luna_dictionary_from_rows(&fixture);

    for case in cases(&fixture) {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        let expected = selected_texts(case);
        let mut engine = m17_luna_sentence_engine(dictionary.clone());
        engine
            .process_key_sequence(input)
            .expect("key sequence should parse");
        let actual = current_page_texts(&engine, expected.len())
            .into_iter()
            .take(expected.len())
            .collect::<Vec<_>>();
        assert_eq!(
            actual, expected,
            "first sentence page should match for {input}"
        );
        assert_eq!(
            engine.context().candidates[0].source,
            CandidateSource::Sentence,
            "top candidate should come from the M17 upstream sentence path for {input}"
        );
    }
}

#[test]
fn full_dictionary_selection_uses_all_exact_code_rows_and_essay_weights() {
    let fixture = fixture(SELECTION_FIXTURE);
    assert_upstream_oracle_header(&fixture);
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "all_rows_for_exact_code_plus_relevant_essay_rows"
    );
    assert_eq!(fixture["capture"]["tested_code"], "ni");

    let dictionary_rows = fixture["capture"]["source_dictionary_rows_all_for_code"]
        .as_array()
        .expect("selection dictionary rows should be present");
    let essay_rows = fixture["capture"]["essay_vocabulary_rows_for_candidates"]
        .as_array()
        .expect("selection essay rows should be present");
    assert!(
        dictionary_rows.len() > 5,
        "selection fixture must include rows beyond the first page"
    );
    assert!(
        !essay_rows.is_empty(),
        "selection fixture must carry essay weights for candidate ranking"
    );

    let dictionary = luna_dictionary_from_rows(
        &fixture["capture"]["source_dictionary_rows_all_for_code"],
        &fixture["capture"]["essay_vocabulary_rows_for_candidates"],
    );
    let expected = selected_texts(cases(&fixture)[0]);
    for expected_text in &expected {
        let weighted_entry = dictionary
            .entries()
            .iter()
            .find(|entry| entry.code == "ni" && entry.text == *expected_text)
            .unwrap_or_else(|| panic!("missing dictionary entry for {expected_text}"));
        assert!(
            weighted_entry.weight > 0.0,
            "selection fixture must not rank {expected_text} with default/zero essay weight"
        );
    }

    let actual = StaticTableTranslator::from_dictionary(dictionary)
        .translate("ni")
        .into_iter()
        .take(expected.len())
        .map(|candidate| candidate.text)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

#[test]
fn full_pipeline_paging_matches_upstream_action_fixture() {
    let fixture = fixture(ACTIONS_FIXTURE);
    let mut engine = luna_engine_from_rows(
        &fixture["capture"]["source_dictionary_rows_all_for_code"],
        &fixture["capture"]["essay_vocabulary_rows_for_candidates"],
    );

    engine
        .process_key_sequence("ni")
        .expect("key sequence should parse");
    assert_engine_snapshot_matches(&engine, snapshot(&fixture, "paging_ni", "page_1"), None);

    engine
        .process_key_sequence("{Page_Down}")
        .expect("key sequence should parse");
    assert_engine_snapshot_matches(&engine, snapshot(&fixture, "paging_ni", "page_2"), None);

    engine
        .process_key_sequence("{Page_Up}")
        .expect("key sequence should parse");
    assert_engine_snapshot_matches(
        &engine,
        snapshot(&fixture, "paging_ni", "page_1_again"),
        None,
    );
}

#[test]
fn full_pipeline_selection_and_space_commit_match_upstream_action_fixture() {
    let fixture = fixture(ACTIONS_FIXTURE);

    let mut select_engine = luna_engine_from_rows(
        &fixture["capture"]["source_dictionary_rows_all_for_code"],
        &fixture["capture"]["essay_vocabulary_rows_for_candidates"],
    );
    let select_commits = select_engine
        .process_key_sequence("ni2")
        .expect("key sequence should parse");
    let select_snapshot = snapshot(&fixture, "select_ni_second", "after_select_2");
    assert_eq!(select_commits, vec![commit_text(select_snapshot)]);
    assert_engine_snapshot_matches(&select_engine, select_snapshot, Some(&select_commits[0]));

    let mut space_engine = luna_engine_from_rows(
        &fixture["capture"]["source_dictionary_rows_all_for_code"],
        &fixture["capture"]["essay_vocabulary_rows_for_candidates"],
    );
    space_engine
        .process_key_sequence("ni")
        .expect("key sequence should parse");
    assert_engine_snapshot_matches(
        &space_engine,
        snapshot(&fixture, "commit_ni_space", "before_space"),
        None,
    );
    let space_commits = space_engine
        .process_key_sequence("{space}")
        .expect("key sequence should parse");
    let space_snapshot = snapshot(&fixture, "commit_ni_space", "after_space");
    assert_eq!(space_commits, vec![commit_text(space_snapshot)]);
    assert_engine_snapshot_matches(&space_engine, space_snapshot, Some(&space_commits[0]));
}

#[test]
fn reverse_lookup_candidates_match_upstream_fixture() {
    let fixture = fixture(REVERSE_LOOKUP_FIXTURE);
    let lookup_dictionary =
        TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            &dictionary_yaml_from_fixture_rows(
                "stroke",
                "by_weight",
                true,
                &fixture["capture"]["source_stroke_rows"],
            ),
            std::iter::empty::<&str>(),
            |_| None,
            |name| {
                (name == "essay").then(|| {
                    essay_txt_from_fixture_rows(
                        &fixture["capture"]["source_stroke_vocabulary_rows"],
                    )
                })
            },
        )
        .expect("stroke rows should parse");
    let target_dictionary =
        TableDictionary::parse_rime_dict_yaml(&dictionary_yaml_from_fixture_rows(
            "luna_pinyin",
            "original",
            false,
            &fixture["capture"]["source_reverse_comment_rows"],
        ))
        .expect("luna rows should parse as reverse comments");
    let comment_format = vec!["xform/; / /".to_owned()];

    for (scenario, label, input) in [
        ("reverse_lookup_h", "prefix_h", "`h"),
        ("reverse_lookup_hs", "prefix_hs", "`hs"),
        ("reverse_lookup_no_result", "no_result", "`q"),
    ] {
        let mut engine = Engine::new();
        engine.clear_translators();
        engine.add_translator(
            ReverseLookupTranslator::new(
                lookup_dictionary.clone(),
                Some(target_dictionary.clone()),
                "`",
                "'",
            )
            .with_completion(true)
            .with_comment_format(&comment_format),
        );
        engine.set_segment_tags(["abc", "reverse_lookup"]);
        engine
            .process_key_sequence(input)
            .expect("key sequence should parse");

        let expected_snapshot = snapshot(&fixture, scenario, label);
        let expected_texts = selected_texts(expected_snapshot);
        let actual_texts = engine
            .context()
            .candidates
            .iter()
            .take(expected_texts.len())
            .map(|candidate| candidate.text.clone())
            .collect::<Vec<_>>();
        assert_eq!(actual_texts, expected_texts, "{scenario}");

        for (actual, expected) in engine
            .context()
            .candidates
            .iter()
            .zip(selected_candidates(expected_snapshot))
        {
            if let Some(expected_comment) = expected["comment"].as_str() {
                assert_eq!(
                    actual.comment, expected_comment,
                    "{scenario} {}",
                    actual.text
                );
            }
        }
    }
}

#[test]
fn punctuation_candidates_match_upstream_fixture() {
    let fixture = fixture(PUNCTUATION_FIXTURE);
    let entries = &fixture["capture"]["punctuation_entries"];

    let mut period_engine = punctuation_engine(entries);
    let period_commit = period_engine
        .process_key_sequence(".{space}")
        .expect("key sequence should parse");
    let period_snapshot = snapshot(&fixture, "punctuation_period", "period_commit");
    assert_eq!(period_commit, vec![commit_text(period_snapshot)]);

    let mut symbol_engine = punctuation_engine(entries);
    symbol_engine
        .process_key_sequence("/fh")
        .expect("key sequence should parse");
    let expected_symbols = selected_texts(snapshot(&fixture, "symbol_fh", "symbols"));
    let actual_symbols = current_page_texts(&symbol_engine, expected_symbols.len());
    assert_eq!(actual_symbols, expected_symbols);

    let mut no_match_engine = punctuation_engine(entries);
    no_match_engine
        .process_key_sequence("/notasymbol")
        .expect("key sequence should parse");
    assert!(selected_texts(snapshot(&fixture, "symbol_no_match", "no_match")).is_empty());
    assert!(no_match_engine.context().candidates.is_empty());
}

#[test]
fn options_fixture_matches_supported_yune_option_paths() {
    let fixture = fixture(OPTIONS_FIXTURE);

    let mut zh_hans_engine = luna_engine_from_rows(
        &fixture["capture"]["source_dictionary_rows"],
        &fixture["capture"]["source_vocabulary_rows"],
    );
    zh_hans_engine.add_filter(SimplifierFilter::new().with_option_name("zh_hans"));
    zh_hans_engine
        .process_key_sequence("guo")
        .expect("key sequence should parse");
    let traditional = snapshot(&fixture, "option_zh_hans_single_off", "traditional_single");
    assert_eq!(
        current_page_texts(&zh_hans_engine, selected_texts(traditional).len()),
        selected_texts(traditional)
    );
    zh_hans_engine.set_option("zh_hans", true);
    let simplified = snapshot(&fixture, "option_zh_hans_single_on", "simplified_single");
    assert_eq!(
        current_page_texts(&zh_hans_engine, selected_texts(simplified).len()),
        selected_texts(simplified)
    );

    let mut full_shape_engine = punctuation_engine(&fixture["capture"]["punctuation_entries"]);
    full_shape_engine.set_option("full_shape", true);
    full_shape_engine
        .process_key_sequence("/")
        .expect("key sequence should parse");
    let full_shape = snapshot(
        &fixture,
        "option_full_shape_on",
        "full_shape_slash_snapshot",
    );
    assert_eq!(
        full_shape_engine.context().candidates[0].text,
        selected_texts(full_shape)[0]
    );
}

#[test]
fn full_sentence_lattice_parity_for_zhongguo_matches_upstream_fixture() {
    let fixture = fixture(LATTICE_FIXTURE);
    assert_upstream_oracle_header(&fixture);
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "m17_upstream_luna_sentence_lattice"
    );

    let mut engine = Engine::new();
    engine.clear_translators();
    engine.add_translator(
        StaticTableTranslator::from_dictionary(m17_luna_dictionary_from_rows(&fixture))
            .with_charset_filter(true)
            .with_upstream_sentence_model(100),
    );

    engine
        .process_key_sequence("zhongguo")
        .expect("key sequence should parse");
    assert_engine_snapshot_matches(
        &engine,
        snapshot(&fixture, "sentence_lattice_zhongguo", "page_1"),
        None,
    );

    engine
        .process_key_sequence("{Page_Down}")
        .expect("page down should parse");
    let page_2 = snapshot(&fixture, "sentence_lattice_zhongguo", "page_2");
    assert_engine_snapshot_matches(&engine, page_2, None);
    assert_highlighted_commit_preview_matches(&engine, page_2);

    engine
        .process_key_sequence("{Page_Up}")
        .expect("page up should parse");
    assert_engine_snapshot_matches(
        &engine,
        snapshot(&fixture, "sentence_lattice_zhongguo", "page_1_again"),
        None,
    );
}

#[test]
fn ascii_punct_option_processor_bypass_matches_upstream_fixture() {
    let fixture = fixture(M18_PUNCTUATION_FIXTURE);
    assert_upstream_oracle_header_for_schema(&fixture, "m18_punct");
    let mut engine = m18_punctuation_engine(&fixture);

    engine.set_option("ascii_punct", true);
    let commits = engine
        .process_key_sequence(".")
        .expect("key sequence should parse");

    let expected = snapshot(&fixture, "ascii_punct_period", "period_noop");
    assert!(commits.is_empty());
    assert_eq!(expected["processed"], 0);
    assert_engine_snapshot_matches(&engine, expected, None);
}

#[test]
fn punctuation_processor_commit_confirm_pair_and_list_match_upstream_fixture() {
    let fixture = fixture(M18_PUNCTUATION_FIXTURE);
    assert_upstream_oracle_header_for_schema(&fixture, "m18_punct");

    let mut direct_commit_engine = m18_punctuation_engine(&fixture);
    let period_commits = direct_commit_engine
        .process_key_sequence(".")
        .expect("key sequence should parse");
    let period = snapshot(&fixture, "direct_commit_period", "period_commit");
    assert_eq!(period["processed"], 1);
    assert_eq!(period_commits, vec![commit_text(period)]);
    assert_engine_snapshot_matches(
        &direct_commit_engine,
        period,
        period_commits.first().map(String::as_str),
    );

    let mut confirm_unique_engine = m18_punctuation_engine(&fixture);
    let bang_commits = confirm_unique_engine
        .process_key_sequence("!")
        .expect("key sequence should parse");
    let bang = snapshot(&fixture, "confirm_unique_bang", "bang_commit");
    assert_eq!(bang["processed"], 1);
    assert!(bang_commits.is_empty());
    assert_engine_snapshot_matches(&confirm_unique_engine, bang, None);

    let mut pair_engine = m18_punctuation_engine(&fixture);
    for label in ["open_commit", "close_commit", "open_again_commit"] {
        let commits = pair_engine
            .process_key_sequence("(")
            .expect("key sequence should parse");
        let expected = snapshot(&fixture, "pair_parenthesis", label);
        assert_eq!(expected["processed"], 1);
        assert!(commits.is_empty());
        assert_engine_snapshot_matches(&pair_engine, expected, None);
    }

    let mut slash_engine = m18_punctuation_engine(&fixture);
    for label in ["slash_candidates", "slash_next"] {
        let commits = slash_engine
            .process_key_sequence("/")
            .expect("key sequence should parse");
        let expected = snapshot(&fixture, "slash_candidates", label);
        assert_eq!(expected["processed"], 1);
        assert!(commits.is_empty());
        assert_engine_snapshot_matches(&slash_engine, expected, None);
    }
}

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(FIXTURE_ROOT)
        .join(name);
    let fixture = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_str(&fixture).unwrap_or_else(|error| panic!("invalid JSON {path:?}: {error}"))
}

fn assert_upstream_oracle_header(fixture: &Value) {
    assert_upstream_oracle_header_for_schema(fixture, "luna_pinyin");
}

fn assert_upstream_oracle_header_for_schema(fixture: &Value, schema: &str) {
    assert_eq!(fixture["oracle"]["engine"], "rime/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "1.17.0");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    );
    assert_eq!(fixture["schema"], schema);
    assert_eq!(fixture["module_list"], serde_json::json!(["default"]));
}

fn cases(fixture: &Value) -> Vec<&Value> {
    fixture["cases"]
        .as_array()
        .expect("fixture cases should be an array")
        .iter()
        .collect()
}

fn selected_candidates(snapshot: &Value) -> &[Value] {
    snapshot["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array")
}

fn selected_texts(snapshot: &Value) -> Vec<String> {
    selected_candidates(snapshot)
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

fn luna_dictionary_from_rows(dictionary_rows: &Value, vocabulary_rows: &Value) -> TableDictionary {
    TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        &dictionary_yaml_from_fixture_rows("luna_pinyin", "by_weight", true, dictionary_rows),
        std::iter::empty::<&str>(),
        |_| None,
        |name| (name == "essay").then(|| essay_txt_from_fixture_rows(vocabulary_rows)),
    )
    .expect("upstream luna_pinyin source rows should parse")
}

fn luna_engine_from_rows(dictionary_rows: &Value, vocabulary_rows: &Value) -> Engine {
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.add_translator(StaticTableTranslator::from_dictionary(
        luna_dictionary_from_rows(dictionary_rows, vocabulary_rows),
    ));
    engine
}

fn m17_luna_dictionary_from_rows(fixture: &Value) -> TableDictionary {
    TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        &dictionary_yaml_from_fixture_rows(
            "luna_pinyin",
            "by_weight",
            true,
            &fixture["capture"]["source_dictionary_rows_for_tested_codes"],
        ),
        std::iter::empty::<&str>(),
        |_| None,
        |name| {
            (name == "essay").then(|| {
                essay_txt_from_fixture_rows(
                    &fixture["capture"]["essay_vocabulary_rows_for_candidates"],
                )
            })
        },
    )
    .expect("M17 upstream sentence source rows should parse")
}

fn m17_luna_sentence_engine(dictionary: TableDictionary) -> Engine {
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_charset_filter(true)
            .with_upstream_sentence_model(100),
    );
    engine
}

fn punctuation_engine(entries: &Value) -> Engine {
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.add_translator(PunctuationTranslator::with_shape_and_symbol_entries(
        tuple_rows(&entries["half_shape"]),
        tuple_rows(&entries["full_shape"]),
        tuple_rows(&entries["symbols"]),
    ));
    engine
}

fn m18_punctuation_engine(fixture: &Value) -> Engine {
    let definitions = &fixture["capture"]["punctuation_definitions"];
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.add_translator(PunctuationTranslator::with_shape_and_symbol_entries(
        punctuation_definition_candidate_rows(&definitions["half_shape"]),
        punctuation_definition_candidate_rows(&definitions["full_shape"]),
        punctuation_definition_candidate_rows(&definitions["symbols"]),
    ));
    engine.set_punctuation_processor(PunctuationProcessor::with_shape_definitions(
        punctuation_definition_rows(&definitions["half_shape"]),
        punctuation_definition_rows(&definitions["full_shape"]),
        punctuation_definition_rows(&definitions["symbols"]),
    ));
    engine
}

fn dictionary_yaml_from_fixture_rows(
    name: &str,
    sort: &str,
    use_preset_vocabulary: bool,
    rows: &Value,
) -> String {
    let rows = rows
        .as_array()
        .expect("dictionary rows should be an array")
        .iter()
        .map(|row| row.as_str().expect("dictionary row should be a string"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "---\nname: {name}\nversion: 'upstream-oracle-slice'\nsort: {sort}\nuse_preset_vocabulary: {use_preset_vocabulary}\n...\n\n{rows}\n"
    )
}

fn essay_txt_from_fixture_rows(rows: &Value) -> String {
    rows.as_array()
        .expect("vocabulary rows should be an array")
        .iter()
        .map(|row| row.as_str().expect("vocabulary row should be a string"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn tuple_rows(rows: &Value) -> Vec<(String, String)> {
    rows.as_array()
        .expect("tuple rows should be an array")
        .iter()
        .map(|row| {
            let row = row.as_array().expect("tuple row should be an array");
            (
                row[0]
                    .as_str()
                    .expect("tuple key should be a string")
                    .to_owned(),
                row[1]
                    .as_str()
                    .expect("tuple value should be a string")
                    .to_owned(),
            )
        })
        .collect()
}

fn punctuation_definition_rows(rows: &Value) -> Vec<(String, PunctuationDefinition)> {
    rows.as_array()
        .expect("definition rows should be an array")
        .iter()
        .map(|row| {
            let key = row["key"]
                .as_str()
                .expect("definition key should be a string")
                .to_owned();
            let values = row["values"]
                .as_array()
                .expect("definition values should be an array")
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .expect("definition value should be a string")
                        .to_owned()
                })
                .collect::<Vec<_>>();
            let definition = match row["kind"]
                .as_str()
                .expect("definition kind should be a string")
            {
                "commit" => PunctuationDefinition::Commit(
                    values
                        .first()
                        .expect("commit definition should have a value")
                        .clone(),
                ),
                "confirm_unique" => PunctuationDefinition::ConfirmUnique(
                    values
                        .first()
                        .expect("confirm definition should have a value")
                        .clone(),
                ),
                "pair" => PunctuationDefinition::Pair([
                    values
                        .first()
                        .expect("pair definition should have an opening value")
                        .clone(),
                    values
                        .get(1)
                        .expect("pair definition should have a closing value")
                        .clone(),
                ]),
                "candidates" => PunctuationDefinition::Candidates(values),
                kind => panic!("unsupported punctuation definition kind: {kind}"),
            };
            (key, definition)
        })
        .collect()
}

fn punctuation_definition_candidate_rows(rows: &Value) -> Vec<(String, String)> {
    punctuation_definition_rows(rows)
        .into_iter()
        .flat_map(|(key, definition)| {
            definition
                .candidate_texts()
                .into_iter()
                .map(move |text| (key.clone(), text))
        })
        .collect()
}

fn assert_engine_snapshot_matches(
    engine: &Engine,
    expected_snapshot: &Value,
    observed_commit_text: Option<&str>,
) {
    let expected_page_size = expected_snapshot["page_size"]
        .as_u64()
        .expect("page size should be numeric") as usize;
    let expected_texts = selected_texts(expected_snapshot);

    assert_eq!(
        engine.status().is_composing,
        expected_snapshot["is_composing"]
    );
    assert_eq!(
        engine.context().composition.preedit,
        expected_snapshot["preedit"]
            .as_str()
            .unwrap_or_default()
            .replace(' ', "")
    );
    if expected_page_size > 0 {
        let expected_page_no = expected_snapshot["page_no"]
            .as_u64()
            .expect("page number should be numeric") as usize;
        let expected_highlighted = expected_snapshot["highlighted_candidate_index"]
            .as_u64()
            .expect("highlighted index should be numeric")
            as usize;
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
                .take(expected_texts.len())
                .collect::<Vec<_>>(),
            expected_texts
        );
    } else {
        assert!(expected_texts.is_empty());
        assert!(engine.context().candidates.is_empty());
    }

    if let Some(observed_commit_text) = observed_commit_text {
        assert_eq!(
            Some(observed_commit_text),
            expected_snapshot["commit_text"].as_str()
        );
    }
}

fn assert_highlighted_commit_preview_matches(engine: &Engine, expected_snapshot: &Value) {
    let actual = engine.context().candidates[engine.context().highlighted]
        .commit_text_for_input(&engine.context().composition.input);
    assert_eq!(
        Some(actual.as_str()),
        expected_snapshot["commit_text_preview"].as_str()
    );
}

fn current_page_texts(engine: &Engine, page_size: usize) -> Vec<String> {
    let page_size = page_size.max(1);
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
