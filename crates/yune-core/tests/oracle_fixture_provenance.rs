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
