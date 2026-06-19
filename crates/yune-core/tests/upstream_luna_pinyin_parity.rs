use std::{fs, path::Path};

use serde_json::Value;
use yune_core::{StaticTableTranslator, TableDictionary, Translator};

const FIXTURE: &str = "tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json";

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let fixture = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_str(&fixture).unwrap_or_else(|error| panic!("invalid JSON {path:?}: {error}"))
}

#[test]
fn upstream_luna_pinyin_fixture_is_locked() {
    let fixture = fixture();
    assert_eq!(fixture["oracle"]["engine"], "rime/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "1.17.0");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    );
    assert_eq!(fixture["schema"], "luna_pinyin");
    assert_eq!(fixture["module_list"], serde_json::json!(["default"]));
    assert_eq!(fixture["capture"]["schema_data"], "rime/rime-luna-pinyin");

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
        assert_eq!(case["schema_name"], "朙月拼音");
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
    assert_eq!(zhongguo["selected_candidates"][0]["text"], "中國");
}

#[test]
fn yune_table_translator_matches_upstream_luna_pinyin_single_code_first_page() {
    let fixture = fixture();
    let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        &dictionary_yaml_from_fixture_rows(
            "luna_pinyin",
            &fixture["capture"]["source_dictionary_rows"],
        ),
        std::iter::empty::<&str>(),
        |_| None,
        |name| {
            (name == "essay")
                .then(|| essay_txt_from_fixture_rows(&fixture["capture"]["source_vocabulary_rows"]))
        },
    )
    .expect("upstream luna_pinyin source rows should parse");
    let translator = StaticTableTranslator::from_dictionary(dictionary);

    for case in fixture["cases"]
        .as_array()
        .expect("oracle cases should be an array")
        .iter()
        .filter(|case| case["input"] != "zhongguo")
    {
        let input = case["input"]
            .as_str()
            .expect("case input should be a string");
        let expected = case["selected_candidates"]
            .as_array()
            .expect("selected candidates should be an array")
            .iter()
            .map(|candidate| {
                candidate["text"]
                    .as_str()
                    .expect("candidate text should be a string")
            })
            .collect::<Vec<_>>();
        let actual = translator
            .translate(input)
            .into_iter()
            .take(expected.len())
            .map(|candidate| candidate.text)
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "first page should match for {input}");
    }
}

fn dictionary_yaml_from_fixture_rows(name: &str, rows: &Value) -> String {
    let rows = rows
        .as_array()
        .expect("dictionary rows should be an array")
        .iter()
        .map(|row| row.as_str().expect("dictionary row should be a string"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "---\nname: {name}\nversion: 'upstream-oracle-slice'\nsort: by_weight\nuse_preset_vocabulary: true\n...\n\n{rows}\n"
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
