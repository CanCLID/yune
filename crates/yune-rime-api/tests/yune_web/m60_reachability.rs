use super::*;

use yune_rime_api::schema_reachability_audit_json;

const FIXTURE_ROOT: &str = "crates/yune-rime-api/tests/fixtures/m60-schema-reachability-audit";
const AUDIT_SCHEMA: &str = concat!(
    "crates/yune-rime-api/tests/fixtures/m60-schema-reachability-audit/",
    "shared/audit.schema.yaml"
);
const MIRROR_SCHEMA: &str = concat!(
    "crates/yune-rime-api/tests/fixtures/m60-schema-reachability-audit/",
    "shared/audit_mirror.schema.yaml"
);
const UNRESOLVED_SCHEMA: &str = concat!(
    "crates/yune-rime-api/tests/fixtures/m60-schema-reachability-audit/",
    "shared/unresolved.schema.yaml"
);
const EXPECTED: &str =
    include_str!("../fixtures/m60-schema-reachability-audit/expected-tuples.json");

#[test]
fn m60_namespaced_reachability_audit_matches_real_deploy() {
    let _guard = test_guard();
    let fixture_root = repo_root().join(FIXTURE_ROOT);
    let shared = fixture_root.join("shared");
    let user = fixture_root.join("user");
    let audited = schema_reachability_audit_json(
        &repo_root(),
        &shared,
        &user,
        &[PathBuf::from(AUDIT_SCHEMA), PathBuf::from(MIRROR_SCHEMA)],
    )
    .expect("the authored M60 fixture should audit");
    let audited: Value = serde_json::from_str(&audited).expect("audit output should be JSON");
    let expected: Value = serde_json::from_str(EXPECTED).expect("expected tuples should be JSON");
    assert_eq!(audited["version"], expected["version"]);

    let actual_tuples = audited["tuples"]
        .as_array()
        .expect("audit output should contain tuples");
    assert_eq!(
        actual_tuples.len(),
        5,
        "fixture should audit all dictionary arms"
    );
    let expected_schemas = expected["schemas"]
        .as_array()
        .expect("expected fixture should contain schemas");
    for expected_schema in expected_schemas {
        let schema_asset = expected_schema["schemaAsset"]
            .as_str()
            .expect("expected schema asset should be text");
        let actual = actual_tuples
            .iter()
            .filter(|tuple| tuple["schemaAsset"] == schema_asset)
            .collect::<Vec<_>>();
        assert!(
            !actual.is_empty(),
            "{schema_asset} should emit at least one tuple"
        );
        assert!(actual.iter().all(|tuple| {
            tuple["schemaId"] == expected_schema["schemaId"]
                && tuple["assetHashes"] == expected_schema["assetHashes"]
                && tuple["sourceTrace"] == expected_schema["sourceTrace"]
        }));
        let projected = actual
            .iter()
            .map(|tuple| {
                json!({
                    "component": tuple["component"],
                    "namespace": tuple["namespace"],
                    "configPath": tuple["configPath"],
                    "effective": tuple["effective"],
                    "settingAsset": tuple["settingAsset"],
                    "translatorArm": tuple["translatorArm"],
                    "runtimeArm": tuple["runtimeArm"],
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(Value::Array(projected), expected_schema["tuples"]);
    }
    let false_tuples = actual_tuples
        .iter()
        .filter(|tuple| tuple["effective"] == false)
        .collect::<Vec<_>>();
    assert_eq!(false_tuples.len(), 3);
    let shared_false = false_tuples
        .iter()
        .filter(|tuple| {
            tuple["settingAsset"]
                == "crates/yune-rime-api/tests/fixtures/m60-schema-reachability-audit/shared/template.yaml"
                && tuple["configPath"] == "shared/leading_syllable_reachability"
        })
        .map(|tuple| tuple["schemaAsset"].as_str().unwrap_or_default())
        .collect::<BTreeSet<_>>();
    assert_eq!(shared_false, BTreeSet::from([AUDIT_SCHEMA, MIRROR_SCHEMA]));

    let runtime = YuneWebRuntime::create_with_schema("m60-reachability-audit", "audit");
    copy_dir_contents(&shared, &runtime.shared);
    copy_dir_contents(&user, &runtime.user);
    deploy_public_demo_schema(&runtime, "audit");
    deploy_public_demo_schema(&runtime, "audit_mirror");
    let deployed: Value = serde_yaml::from_str(
        &fs::read_to_string(runtime.user.join("build/audit.schema.yaml"))
            .expect("audit schema should deploy"),
    )
    .expect("deployed audit schema should parse");
    for (path, expected_value) in [
        ("/foo/leading_syllable_reachability", false),
        ("/bar/leading_syllable_reachability", true),
        ("/baz/leading_syllable_reachability", true),
        ("/shared/leading_syllable_reachability", false),
    ] {
        assert_eq!(
            deployed.pointer(path).and_then(config_bool_like),
            Some(expected_value),
            "production deployment and the audit must agree at {path}"
        );
    }
    let mirror: Value = serde_yaml::from_str(
        &fs::read_to_string(runtime.user.join("build/audit_mirror.schema.yaml"))
            .expect("mirror schema should deploy"),
    )
    .expect("deployed mirror schema should parse");
    assert_eq!(
        mirror
            .pointer("/shared/leading_syllable_reachability")
            .and_then(config_bool_like),
        Some(false)
    );
    drop(runtime);
    reset_rime();

    let unresolved = schema_reachability_audit_json(
        &repo_root(),
        &shared,
        &user,
        &[PathBuf::from(UNRESOLVED_SCHEMA)],
    )
    .expect_err("a required reference in the relevant deployment closure must fail");
    assert!(
        unresolved.contains("unresolved or unsupported deployment directive"),
        "unexpected unresolved-reference error: {unresolved}"
    );
}
