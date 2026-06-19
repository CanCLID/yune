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
