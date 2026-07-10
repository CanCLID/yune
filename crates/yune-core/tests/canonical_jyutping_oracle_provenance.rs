use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/upstream-jyutping/canonical-rime-cantonese")
}

fn read_json(path: &PathBuf) -> Value {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    assert!(
        !bytes.starts_with(&[0xef, 0xbb, 0xbf]),
        "{path:?} has a BOM"
    );
    assert!(!bytes.contains(&b'\r'), "{path:?} must use LF newlines");
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("invalid JSON in {path:?}: {error}"))
}

fn sha256(path: &PathBuf) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn canonical_jyutping_being_fixture_is_external_and_source_absent() {
    let root = fixture_root();
    let fixture_path = root.join("jyutping-m59-being-whole-input.json");
    let manifest_path = root.join("oracle-manifest.json");
    let fixture = read_json(&fixture_path);
    let manifest = read_json(&manifest_path);

    assert_eq!(
        manifest["fixture_family"],
        "canonical-rime-cantonese-whole-input"
    );
    assert_eq!(
        manifest["oracle"]["engine_commit"],
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    );
    assert_eq!(
        manifest["oracle"]["schema_commit"],
        "c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0"
    );
    assert_eq!(manifest["fixtures"][0]["sha256"], sha256(&fixture_path));

    assert_eq!(fixture["milestone"], "M59");
    assert_eq!(fixture["canonical"], true);
    assert_eq!(fixture["capture"]["capture_mode"], "m59-whole-input");
    assert_eq!(fixture["capture"]["capture_date"], "2026-07-10");
    assert_eq!(fixture["capture"]["captured_all_pages"], true);
    assert_eq!(fixture["cases"][0]["input"], "being");
    assert_eq!(fixture["cases"][0]["preedit"], "bei ng");
    assert_eq!(
        fixture["cases"][0]["selected_candidates"][0]["text"],
        "畀嗯"
    );
    assert_eq!(fixture["cases"][0]["commit_text_preview"], "畀嗯");

    let repositories = fixture["capture"]["source_repositories_clean"]
        .as_object()
        .expect("source repository cleanliness map");
    assert_eq!(repositories.len(), 7);
    assert!(repositories.values().all(|clean| clean == true));
    let repository_trees = fixture["capture"]["schema_repo_trees"]
        .as_object()
        .expect("source repository tree map");
    assert_eq!(repository_trees.len(), 7);
    assert!(repository_trees.values().all(|tree| tree
        .as_str()
        .is_some_and(|value| value.len() == 40 && value.chars().all(|ch| ch.is_ascii_hexdigit()))));

    let oracle_row = &fixture["source_lexicon"]["whole_input_oracle_rows"][0];
    assert_eq!(oracle_row["input"], "being");
    assert_eq!(oracle_row["oracle_top"], "畀嗯");
    assert_eq!(oracle_row["constituents"], serde_json::json!(["畀", "嗯"]));
    assert_eq!(oracle_row["source_dictionary_exact_term_count"], 0);
    assert_eq!(oracle_row["source_vocabulary_exact_term_count"], 0);
    assert_eq!(oracle_row["source_lexicon_absent"], true);

    let source_files = fixture["source_lexicon"]["source_files"]
        .as_array()
        .expect("source lexicon file list");
    assert_eq!(source_files.len(), 7);
    assert_eq!(
        source_files
            .iter()
            .filter(|row| row["kind"] == "dictionary")
            .count(),
        6
    );
    assert_eq!(
        source_files
            .iter()
            .filter(|row| row["kind"] == "vocabulary")
            .count(),
        1
    );
    assert!(source_files.iter().all(|row| row["sha256"]
        .as_str()
        .is_some_and(|value| value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))));

    for field in ["source_dictionary_rows", "source_vocabulary_rows"] {
        let terms = fixture["source_lexicon"][field]
            .as_array()
            .unwrap_or_else(|| panic!("missing source lexicon field {field}"))
            .iter()
            .map(|row| row["term"].as_str().expect("source-row term"))
            .collect::<BTreeSet<_>>();
        assert_eq!(terms, BTreeSet::from(["嗯", "畀"]), "{field}");
    }
}

#[test]
fn canonical_jyutping_being_fixture_pins_capture_bytes() {
    let root = fixture_root();
    let fixture = read_json(&root.join("jyutping-m59-being-whole-input.json"));
    let manifest = read_json(&root.join("oracle-manifest.json"));
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = repo_root.join("scripts/capture-upstream-rime-cantonese.ps1");
    let probe = repo_root.join("scripts/oracle-rime-probe.cs");

    let script_hash = sha256(&script);
    let probe_hash = sha256(&probe);
    assert_eq!(fixture["capture"]["capture_script_sha256"], script_hash);
    assert_eq!(fixture["capture"]["probe_sha256"], probe_hash);
    assert_eq!(fixture["oracle"]["capture_script_sha256"], script_hash);
    assert_eq!(fixture["oracle"]["probe_sha256"], probe_hash);
    assert_eq!(manifest["tools"]["capture_script_sha256"], script_hash);
    assert_eq!(manifest["tools"]["probe_sha256"], probe_hash);

    let invocation = fixture["capture"]["actual_invocation"]
        .as_str()
        .expect("actual invocation");
    for required in [
        "-Inputs 'being'",
        "-CaptureMode 'm59-whole-input'",
        "-CaptureDate '2026-07-10'",
        "-ReportedCaseInput 'being'",
    ] {
        assert!(
            invocation.contains(required),
            "missing {required} in {invocation}"
        );
    }
    assert!(
        !invocation.contains(":\\"),
        "local absolute path leaked into invocation"
    );
}
