use std::{fs, path::Path};

use serde_json::Value;

fn fixture_root(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn oracle_fixture_roots_have_machine_readable_provenance() {
    assert_manifest(
        "upstream-1.17.0",
        "upstream-core",
        "rime/librime",
        "1.17.0",
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
        false,
    );
    assert_manifest(
        "typeduck-v1.1.2",
        "typeduck-profile",
        "TypeDuck-HK/librime",
        "v1.1.2",
        "74cb52b78fb2411137a7643f6c8bc6517acfde69",
        true,
    );
}

#[test]
fn upstream_luna_pinyin_fixtures_have_non_circular_source_provenance() {
    let root = fixture_root("upstream-1.17.0");
    let mut fixture_files = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read fixture entry: {error}"))
                .path()
        })
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("luna-pinyin-") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    fixture_files.sort();
    assert_eq!(
        fixture_files.len(),
        6,
        "M12 closeout should keep the full upstream luna_pinyin fixture set checked in"
    );

    for path in fixture_files {
        let fixture = read_json(&path);
        assert_luna_fixture_header(&path, &fixture);
        assert_no_local_absolute_paths(&path, &fixture);
        assert_policy_specific_provenance(&path, &fixture);
    }
}

#[test]
fn typeduck_v112_m14_fixtures_have_non_circular_source_provenance() {
    let root = fixture_root("typeduck-v1.1.2");
    let mut fixture_files = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read fixture entry: {error}"))
                .path()
        })
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("jyut6ping3-m14-") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    fixture_files.sort();
    assert_eq!(
        fixture_files.len(),
        5,
        "M14 should keep the five TypeDuck v1.1.2 fixture files checked in"
    );

    for path in fixture_files {
        let fixture = read_json(&path);
        assert_typeduck_v112_fixture_header(&path, &fixture);
        assert_no_local_absolute_paths(&path, &fixture);
    }
}

#[test]
fn typeduck_v112_m21_sentence_composition_fixture_has_non_circular_source_provenance() {
    let root = fixture_root("typeduck-v1.1.2");
    let path = root.join("jyut6ping3-m21-sentence-composition.json");
    assert!(
        path.is_file(),
        "M21-GAP-01 should check in the TypeDuck sentence-composition fixture"
    );
    let fixture = read_json(&path);
    assert_typeduck_v112_fixture_header(&path, &fixture);
    assert_no_local_absolute_paths(&path, &fixture);
    assert_eq!(
        fixture["capture"]["source_row_policy"], "typeduck_v112_binary_smoke",
        "{path:?}"
    );
    assert_eq!(fixture["schema"], "jyut6ping3_mobile", "{path:?}");
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default", "dictionary_lookup"]),
        "{path:?}"
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
        ]),
        "{path:?}"
    );
    for case in fixture["cases"]
        .as_array()
        .unwrap_or_else(|| panic!("{path:?} cases should be an array"))
    {
        let input = case["input"]
            .as_str()
            .unwrap_or_else(|| panic!("{path:?} case input should be a string"));
        let top_comment = case["selected_candidates"][0]["comment"]
            .as_str()
            .unwrap_or_else(|| panic!("{path:?} {input} top comment should be a string"));
        assert!(
            top_comment.contains(",composition,"),
            "{path:?} {input} should preserve the oracle composition row"
        );
    }
}

#[test]
fn typeduck_v112_fork_parity_fixtures_have_non_circular_source_provenance() {
    let root = fixture_root("typeduck-v1.1.2");
    let mut fixture_files = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read fixture entry: {error}"))
                .path()
        })
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("jyut6ping3-fork-parity-") && name.ends_with(".json")
                })
        })
        .collect::<Vec<_>>();
    fixture_files.sort();
    assert_eq!(
        fixture_files.len(),
        4,
        "FORK-PARITY should keep the captured TypeDuck fork fixtures checked in"
    );

    let path = root.join("jyut6ping3-fork-parity-01-real-dictionary-fuzzy.json");
    assert!(
        path.is_file(),
        "FORK-PARITY-01 should check in the TypeDuck real-dictionary fixture"
    );
    let fixture = read_json(&path);
    assert_typeduck_v112_fixture_header(&path, &fixture);
    assert_no_local_absolute_paths(&path, &fixture);
    assert_eq!(
        fixture["capture"]["source_row_policy"],
        "typeduck_v112_real_mobile_translator_and_scolar_lookup_fuzzy",
        "{path:?}"
    );
    assert_eq!(
        fixture["capture"]["translator_dictionary_file"], "TypeDuck-HK/schema/jyut6ping3.dict.yaml",
        "{path:?}"
    );
    assert_eq!(
        fixture["capture"]["lookup_dictionary_file"],
        "TypeDuck-HK/schema/jyut6ping3_scolar.dict.yaml",
        "{path:?}"
    );
    for key in ["translator_dictionary", "lookup_dictionary"] {
        let dictionary_count = fixture["capture"]["source_row_counts"][key]
            .as_u64()
            .unwrap_or_else(|| panic!("{key} source row count should be numeric"));
        assert!(
            dictionary_count > 50_000,
            "{path:?} must prove the production-sized {key} path"
        );
    }
    assert_non_empty_array(
        &path,
        &fixture,
        &["capture", "source_translator_rows_for_candidates"],
    );
    assert_non_empty_array(
        &path,
        &fixture,
        &["capture", "source_lookup_rows_for_candidates"],
    );
    assert_non_empty_array(&path, &fixture, &["capture", "speller_algebra_rules"]);

    let path = root.join("jyut6ping3-fork-parity-02-prefer-user-phrase.json");
    assert!(
        path.is_file(),
        "FORK-PARITY-02 should check in the TypeDuck PreferUserPhrase fixture"
    );
    let fixture = read_json(&path);
    assert_typeduck_v112_fixture_header(&path, &fixture);
    assert_no_local_absolute_paths(&path, &fixture);
    assert_eq!(
        fixture["capture"]["source_row_policy"], "typeduck_v112_prefer_user_phrase_weighted_gate",
        "{path:?}"
    );
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default", "dictionary_lookup", "levers"]),
        "{path:?}"
    );
    assert_eq!(fixture["cases"][0]["probe"]["import_return"], 1, "{path:?}");
    assert!(
        fixture["cases"][0]["probe"]["captures"]
            .as_array()
            .is_some_and(|captures| !captures.is_empty()),
        "{path:?} must include captured candidate snapshots"
    );

    let path = root.join("jyut6ping3-fork-parity-06-letter-to-tone.json");
    assert!(
        path.is_file(),
        "FORK-PARITY-06 should check in the TypeDuck letter_to_tone fixture"
    );
    let fixture = read_json(&path);
    assert_typeduck_v112_fixture_header(&path, &fixture);
    assert_no_local_absolute_paths(&path, &fixture);
    assert_eq!(
        fixture["capture"]["source_row_policy"], "typeduck_v112_letter_to_tone_preedit",
        "{path:?}"
    );
    assert_eq!(fixture["schema"], "jyut6ping3_mobile", "{path:?}");
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default", "dictionary_lookup"]),
        "{path:?}"
    );
    assert_eq!(
        fixture["capture"]["input_sequence"],
        serde_json::json!(["neiv", "neivv", "neix", "neixx", "neiq", "neiqq"]),
        "{path:?}"
    );

    let path = root.join("jyut6ping3-fork-parity-07-state-labels.json");
    assert!(
        path.is_file(),
        "FORK-PARITY-07 should check in the TypeDuck full-shape state-label fixture"
    );
    let fixture = read_json(&path);
    assert_typeduck_v112_fixture_header(&path, &fixture);
    assert_no_local_absolute_paths(&path, &fixture);
    assert_eq!(
        fixture["capture"]["source_row_policy"], "typeduck_v112_full_shape_state_labels",
        "{path:?}"
    );
    assert_eq!(fixture["schema"], "jyut6ping3_mobile", "{path:?}");
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default", "dictionary_lookup"]),
        "{path:?}"
    );
    assert_eq!(
        fixture["capture"]["deployed_schema_file"], "jyut6ping3_mobile.schema.yaml",
        "{path:?}"
    );
    let labels = fixture["cases"][0]["labels"]
        .as_array()
        .unwrap_or_else(|| panic!("{path:?} should capture full_shape state labels"));
    assert_eq!(labels.len(), 2, "{path:?}");
    assert_eq!(
        labels
            .iter()
            .map(|row| row["label"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["\u{534a}\u{5f62}", "\u{5168}\u{5f62}"],
        "{path:?}"
    );
}

fn assert_manifest(
    fixture_dir: &str,
    expected_family: &str,
    expected_engine: &str,
    expected_tag: &str,
    expected_commit: &str,
    expected_profile_only: bool,
) {
    let root = fixture_root(fixture_dir);
    assert!(
        root.join("README.md").is_file(),
        "{fixture_dir} must include a human-readable README.md"
    );

    let manifest_path = root.join("oracle-manifest.json");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display()));
    let manifest: Value = serde_json::from_str(&manifest)
        .unwrap_or_else(|error| panic!("invalid JSON {}: {error}", manifest_path.display()));

    assert_eq!(manifest["fixture_family"], expected_family);
    assert_eq!(manifest["oracle"]["engine"], expected_engine);
    assert_eq!(manifest["oracle"]["engine_tag"], expected_tag);
    assert_eq!(manifest["oracle"]["engine_commit"], expected_commit);
    assert_eq!(manifest["profile_only"], expected_profile_only);
    assert!(
        manifest["oracle"]["canonical_repository"]
            .as_str()
            .is_some_and(|url| url.starts_with("https://github.com/")),
        "{fixture_dir} must identify a canonical GitHub oracle repository"
    );
}

fn assert_typeduck_v112_fixture_header(path: &Path, fixture: &Value) {
    assert_eq!(
        fixture["oracle"]["engine"], "TypeDuck-HK/librime",
        "{path:?}"
    );
    assert_eq!(fixture["oracle"]["engine_tag"], "v1.1.2", "{path:?}");
    assert_eq!(
        fixture["oracle"]["engine_commit"], "74cb52b78fb2411137a7643f6c8bc6517acfde69",
        "{path:?}"
    );
    assert!(
        fixture["oracle"]["canonical_repository"]
            .as_str()
            .is_some_and(|url| url == "https://github.com/TypeDuck-HK/librime"),
        "{path:?} must identify the TypeDuck fork repository"
    );
    assert!(
        fixture["oracle"]["release_url"]
            .as_str()
            .is_some_and(|url| url == "https://github.com/TypeDuck-HK/librime/releases/tag/v1.1.2"),
        "{path:?} must identify the TypeDuck v1.1.2 release"
    );
    assert!(
        fixture["oracle"]["capture_date"]
            .as_str()
            .is_some_and(|date| !date.is_empty()),
        "{path:?} must include a capture date"
    );
    assert!(
        fixture["oracle"]["capture_command"]
            .as_str()
            .is_some_and(|command| command.contains("scripts/capture-typeduck-jyutping.ps1")),
        "{path:?} must include the TypeDuck capture command"
    );
    assert_eq!(
        fixture["oracle"]["schema"], "TypeDuck-HK/schema",
        "{path:?}"
    );
    assert!(
        fixture["oracle"]["schema_commit"]
            .as_str()
            .is_some_and(|commit| commit.len() == 40),
        "{path:?} must include the pinned TypeDuck schema commit"
    );
    assert!(
        matches!(
            fixture["schema"].as_str(),
            Some("jyut6ping3_mobile" | "jyut6ping3" | "mixed")
        ),
        "{path:?} must name a TypeDuck jyut6ping3 schema target"
    );
    let modules = fixture["module_list"]
        .as_array()
        .unwrap_or_else(|| panic!("{path:?} must include module_list"));
    assert!(
        modules.starts_with(&[
            serde_json::json!("default"),
            serde_json::json!("dictionary_lookup")
        ]),
        "{path:?} must load default + dictionary_lookup first"
    );
    assert!(
        modules.iter().all(|module| matches!(
            module.as_str(),
            Some("default" | "dictionary_lookup" | "levers")
        )),
        "{path:?} must not load unexpected oracle modules"
    );
    assert_eq!(
        fixture["capture"]["schema_data"], "TypeDuck-HK/schema",
        "{path:?}"
    );
    assert!(
        fixture["capture"]["schema_data_commit"]
            .as_str()
            .is_some_and(|commit| commit.len() == 40),
        "{path:?} must include the pinned schema data commit"
    );
    assert!(
        fixture["capture"]["source_row_policy"]
            .as_str()
            .is_some_and(|policy| !policy.is_empty()),
        "{path:?} must include a source row policy"
    );
    assert!(
        fixture.get("input_sequence").is_some() || fixture.get("scenarios").is_some(),
        "{path:?} must include input_sequence or scenarios"
    );
}

fn read_json(path: &Path) -> Value {
    let body = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&body)
        .unwrap_or_else(|error| panic!("invalid JSON {}: {error}", path.display()))
}

fn assert_luna_fixture_header(path: &Path, fixture: &Value) {
    assert_eq!(fixture["oracle"]["engine"], "rime/librime", "{path:?}");
    assert_eq!(fixture["oracle"]["engine_tag"], "1.17.0", "{path:?}");
    assert_eq!(
        fixture["oracle"]["engine_commit"], "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
        "{path:?}"
    );
    assert!(
        fixture["oracle"]["release_url"]
            .as_str()
            .is_some_and(|url| url == "https://github.com/rime/librime/releases/tag/1.17.0"),
        "{path:?} must identify the official upstream release"
    );
    assert!(
        fixture["oracle"]["capture_date"]
            .as_str()
            .is_some_and(|date| !date.is_empty()),
        "{path:?} must include a capture date"
    );
    assert!(
        fixture["oracle"]["capture_command"]
            .as_str()
            .is_some_and(|command| command.contains("scripts/capture-upstream-luna-pinyin.ps1")),
        "{path:?} must include a reproducible capture command"
    );
    assert_eq!(fixture["schema"], "luna_pinyin", "{path:?}");
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default"]),
        "{path:?}"
    );
    assert_eq!(
        fixture["capture"]["schema_data"], "rime/rime-luna-pinyin",
        "{path:?}"
    );
    assert!(
        fixture["capture"]["schema_data_commit"]
            .as_str()
            .is_some_and(|commit| commit.len() == 40),
        "{path:?} must include the pinned schema data commit"
    );

    let dependencies = fixture["capture"]["dependency_repositories"]
        .as_object()
        .unwrap_or_else(|| panic!("{path:?} must include dependency repository commits"));
    for repo in ["rime/rime-prelude", "rime/rime-essay", "rime/rime-stroke"] {
        assert!(
            dependencies
                .get(repo)
                .and_then(Value::as_str)
                .is_some_and(|commit| commit.len() == 40),
            "{path:?} must include {repo}"
        );
    }
    assert!(
        fixture["capture"]["source_row_policy"]
            .as_str()
            .is_some_and(|policy| !policy.is_empty()),
        "{path:?} must include a source row policy"
    );
    assert!(
        fixture.get("input_sequence").is_some() || fixture.get("scenarios").is_some(),
        "{path:?} must include input_sequence or scenarios"
    );
}

fn assert_policy_specific_provenance(path: &Path, fixture: &Value) {
    match fixture["capture"]["source_row_policy"]
        .as_str()
        .expect("source row policy should be a string")
    {
        "curated_oracle_winners" => {
            assert_non_empty_array(path, fixture, &["capture", "source_dictionary_rows"]);
            assert_non_empty_array(path, fixture, &["capture", "source_vocabulary_rows"]);
        }
        "all_rows_for_exact_code_plus_relevant_essay_rows" => {
            assert_eq!(fixture["capture"]["tested_code"], "ni", "{path:?}");
            assert_eq!(
                fixture["capture"]["source_dictionary_file"],
                "rime-luna-pinyin/luna_pinyin.dict.yaml",
                "{path:?}"
            );
            assert_eq!(
                fixture["capture"]["essay_vocabulary_file"], "rime-essay/essay.txt",
                "{path:?}"
            );
            assert_non_empty_array(
                path,
                fixture,
                &["capture", "source_dictionary_rows_all_for_code"],
            );
            assert_non_empty_array(
                path,
                fixture,
                &["capture", "essay_vocabulary_rows_for_candidates"],
            );
            let dictionary_count = fixture["capture"]["source_row_counts"]["dictionary"]
                .as_u64()
                .expect("dictionary source row count should be numeric");
            let essay_count = fixture["capture"]["source_row_counts"]["essay"]
                .as_u64()
                .expect("essay source row count should be numeric");
            assert!(
                dictionary_count > 5,
                "{path:?} must include competitors beyond page one"
            );
            assert!(essay_count > 0, "{path:?} must include essay weights");

            let essay_terms = fixture["capture"]["essay_vocabulary_rows_for_candidates"]
                .as_array()
                .expect("essay rows should be an array")
                .iter()
                .map(|row| {
                    row.as_str()
                        .expect("essay row should be a string")
                        .split('\t')
                        .next()
                        .expect("essay row should include a term")
                        .to_owned()
                })
                .collect::<std::collections::HashSet<_>>();
            let absent_terms = fixture["capture"]["essay_row_absent"]
                .as_array()
                .expect("essay absent rows should be an array")
                .iter()
                .filter_map(|row| row["text"].as_str())
                .collect::<std::collections::HashSet<_>>();
            for candidate in fixture["cases"][0]["selected_candidates"]
                .as_array()
                .expect("selection case candidates should be an array")
            {
                let text = candidate["text"]
                    .as_str()
                    .expect("candidate text should be a string");
                assert!(
                    essay_terms.contains(text) || absent_terms.contains(text),
                    "{path:?} candidate {text} must have an essay row or explicit absence"
                );
            }
        }
        "action_sequence_oracle_snapshots" => {
            assert_non_empty_array(
                path,
                fixture,
                &["capture", "source_dictionary_rows_all_for_code"],
            );
            assert_non_empty_array(
                path,
                fixture,
                &["capture", "essay_vocabulary_rows_for_candidates"],
            );
            assert_snapshot(path, fixture, "paging_ni", "page_2");
            assert_snapshot(path, fixture, "select_ni_second", "after_select_2");
            assert_snapshot(path, fixture, "commit_ni_space", "after_space");
        }
        "curated_reverse_lookup_rows" => {
            assert_non_empty_array(path, fixture, &["capture", "source_stroke_rows"]);
            assert_non_empty_array(path, fixture, &["capture", "source_stroke_vocabulary_rows"]);
            assert_non_empty_array(path, fixture, &["capture", "source_reverse_comment_rows"]);
            assert_snapshot(path, fixture, "reverse_lookup_no_result", "no_result");
        }
        "curated_symbols_from_pinned_prelude" => {
            assert_non_empty_array(path, fixture, &["capture", "source_symbol_lines"]);
            assert_non_empty_array(
                path,
                fixture,
                &["capture", "punctuation_entries", "half_shape"],
            );
            assert_non_empty_array(
                path,
                fixture,
                &["capture", "punctuation_entries", "symbols"],
            );
            assert_snapshot(path, fixture, "punctuation_period", "period_commit");
            assert_snapshot(path, fixture, "symbol_fh", "symbols");
        }
        "option_action_sequence_oracle_snapshots" => {
            assert_non_empty_array(path, fixture, &["capture", "source_dictionary_rows"]);
            assert_non_empty_array(path, fixture, &["capture", "source_vocabulary_rows"]);
            assert_non_empty_array(
                path,
                fixture,
                &["capture", "punctuation_entries", "full_shape"],
            );
            assert_snapshot(path, fixture, "option_zh_hans_on", "simplified");
            assert_snapshot(
                path,
                fixture,
                "option_zh_hans_single_on",
                "simplified_single",
            );
            assert_snapshot(
                path,
                fixture,
                "option_ascii_punct_on",
                "ascii_period_snapshot",
            );
            assert_snapshot(
                path,
                fixture,
                "option_full_shape_on",
                "full_shape_slash_snapshot",
            );
        }
        policy => panic!("{path:?} has unknown source row policy {policy}"),
    }
}

fn assert_non_empty_array(path: &Path, fixture: &Value, fields: &[&str]) {
    let value = fields.iter().fold(fixture, |value, field| &value[*field]);
    assert!(
        value.as_array().is_some_and(|array| !array.is_empty()),
        "{path:?} must include non-empty {}",
        fields.join(".")
    );
}

fn assert_snapshot(path: &Path, fixture: &Value, scenario: &str, label: &str) {
    assert!(
        fixture["snapshots"]
            .as_array()
            .expect("snapshots should be an array")
            .iter()
            .any(|snapshot| snapshot["scenario"] == scenario && snapshot["label"] == label),
        "{path:?} must include snapshot {scenario}/{label}"
    );
}

fn assert_no_local_absolute_paths(path: &Path, value: &Value) {
    match value {
        Value::String(text) => {
            assert!(
                !text.contains(":\\"),
                "{path:?} must not include local absolute Windows paths: {text}"
            );
            assert!(
                !text.contains("/target/upstream-oracle/"),
                "{path:?} must not include absolute target oracle cache paths: {text}"
            );
        }
        Value::Array(values) => {
            for value in values {
                assert_no_local_absolute_paths(path, value);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                assert_no_local_absolute_paths(path, value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}
