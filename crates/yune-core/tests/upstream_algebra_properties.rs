use std::{fs, path::Path, sync::Arc};

use serde_json::Value;
use sha2::{Digest, Sha256};
use yune_core::{
    build_prism_bin, parse_rime_prism_bin_payload, parse_rime_prism_runtime_payload,
    CompactTableByteSource, MemoryOwnerClass, RimePrismBinPayload, RimePrismRuntimePayload,
};

const FIXTURE: &str = "tests/fixtures/upstream-1.17.0/m59-algebra-properties.json";
const MANIFEST: &str = "tests/fixtures/upstream-1.17.0/oracle-manifest.json";
const FIXTURE_SHA256: &str = "597ab7939c2b2f7f790749106546812fc5dee24d20327e7212fe4d9583510ba0";
const CORRECTION_FIXTURE: &str = "tests/fixtures/upstream-1.17.0/m59-correction-spelling.json";
const CORRECTION_FIXTURE_SHA256: &str =
    "3c0bac5072f122d64398c0e51dc02e1d4edd17ee685f99cdee76a5ef83dc77da";

#[derive(Debug)]
struct UpstreamPrismByteSource(Arc<[u8]>);

impl CompactTableByteSource for UpstreamPrismByteSource {
    fn bytes(&self) -> &[u8] {
        &self.0
    }

    fn storage_label(&self) -> &'static str {
        "m59-upstream-prism-fixture"
    }

    fn mapping_mode(&self) -> &'static str {
        "mmap"
    }
}

#[test]
fn upstream_algebra_property_fixture_is_pinned_to_librime_1_17_0() {
    let fixture = fixture();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = root.join(FIXTURE);
    assert_eq!(sha256(&fixture_path), FIXTURE_SHA256);

    let manifest_path = root.join(MANIFEST);
    let manifest = read_json(&manifest_path);
    let manifest_row = manifest["files"]
        .as_array()
        .expect("oracle manifest files should be an array")
        .iter()
        .find(|row| row["path"] == "m59-algebra-properties.json")
        .expect("oracle manifest should register the algebra-property fixture");
    assert_eq!(manifest_row["sha256"], FIXTURE_SHA256);
    assert_eq!(manifest_row["milestone"], "M59 Increment 3b");

    let decoder = &fixture["capture"]["independent_decoder"];
    assert_eq!(decoder["path"], "scripts/decode-m59-algebra-prisms.py");
    assert_eq!(
        decoder["sha256"],
        "3cc69624efe65d6ab518768d8556fb2d331ea2b2405d6e4120c051757300bdb7"
    );
    let repo_root = root
        .parent()
        .and_then(Path::parent)
        .expect("crate should live two levels below repository root");
    assert_eq!(
        sha256(&repo_root.join("scripts/decode-m59-algebra-prisms.py")),
        decoder["sha256"]
            .as_str()
            .expect("decoder SHA should be a string")
    );
    let replay = &fixture["capture"]["replay_script"];
    let replay_path = repo_root.join(
        replay["path"]
            .as_str()
            .expect("replay script path should be text"),
    );
    assert_eq!(
        fs::metadata(&replay_path)
            .expect("replay script should exist")
            .len(),
        replay["bytes"]
            .as_u64()
            .expect("replay script byte count should be nonnegative")
    );
    assert_eq!(
        sha256(&replay_path),
        replay["sha256"]
            .as_str()
            .expect("replay script SHA should be text")
    );

    let source_rows = fixture["capture"]["source_files"]
        .as_array()
        .expect("replay source files should be an array");
    let expected_source_paths = [
        "shared/default.yaml",
        "shared/m59_alg_collision.dict.yaml",
        "shared/m59_alg_collision.schema.yaml",
        "shared/m59_alg_preserve.dict.yaml",
        "shared/m59_alg_preserve.schema.yaml",
        "shared/m59_alg_erase.dict.yaml",
        "shared/m59_alg_erase.schema.yaml",
        "shared/m59_alg_cumulative.dict.yaml",
        "shared/m59_alg_cumulative.schema.yaml",
    ];
    assert_eq!(source_rows.len(), expected_source_paths.len());
    let source_root = repo_root.join(
        fixture["capture"]["checked_in_source_root"]
            .as_str()
            .expect("checked-in source root should be text"),
    );
    for (row, expected_path) in source_rows.iter().zip(expected_source_paths) {
        assert_eq!(row["path"], expected_path);
        let checkout = fs::read(source_root.join(expected_path)).unwrap_or_else(|error| {
            panic!("failed to read replay source {expected_path}: {error}")
        });
        assert_eq!(
            checkout.len() as u64,
            row["checkout_bytes"]
                .as_u64()
                .expect("checkout byte count should be nonnegative")
        );
        assert_eq!(
            sha256_bytes(&checkout),
            row["checkout_sha256"]
                .as_str()
                .expect("checkout SHA should be text")
        );
        let materialized = match row["materialization"]
            .as_str()
            .expect("materialization should be text")
        {
            "identity" => checkout,
            "append_terminal_lf" => {
                let mut bytes = checkout;
                bytes.push(b'\n');
                bytes
            }
            policy => panic!("unsupported replay materialization {policy}"),
        };
        assert_eq!(
            materialized.len() as u64,
            row["bytes"]
                .as_u64()
                .expect("materialized byte count should be nonnegative")
        );
        assert_eq!(
            sha256_bytes(&materialized),
            row["sha256"]
                .as_str()
                .expect("materialized SHA should be text")
        );
        let expected_timestamp = if expected_path == "shared/default.yaml"
            || expected_path.contains("m59_alg_cumulative")
        {
            1_783_703_132
        } else {
            1_783_700_253
        };
        assert_eq!(
            row["librime_timestamp_epoch_seconds"].as_i64(),
            Some(expected_timestamp)
        );
    }

    assert_eq!(fixture["oracle"]["engine"], "rime/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "1.17.0");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    );
    assert_eq!(
        fixture["oracle"]["rime_dll"]["sha256"],
        "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b"
    );
    assert_eq!(
        fixture["oracle"]["rime_deployer"]["sha256"],
        "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071"
    );
    assert_eq!(fixture["oracle"]["capture_date"], "2026-07-10");

    let cases = cases(&fixture);
    assert_eq!(cases.len(), 4);
    assert_eq!(
        case(cases, "same_surface_same_syllable_collision")["artifact"]["sha256"],
        "b335c754e54ae5b30f713bc4a0cc29e853f51639eeed4541f8f57a9598992b88"
    );
    assert_eq!(
        case(cases, "ordinary_xform_preserves_properties")["artifact"]["sha256"],
        "66f07687794f39e22c2233832dbec2865f5712de914037c90055a767c5f01a86"
    );
    assert_eq!(
        case(cases, "cumulative_credibility_uses_double_until_emission")["artifact"]["sha256"],
        "7b1fa33c638b1a9a15071b3221927d0559d3f79cbe300fca4e52c0a2cc56906d"
    );
    let erase = case(cases, "partial_match_erase_is_inert");
    assert_eq!(
        erase["artifact"]["sha256"],
        "d0941d5b6950e278bcdbbc2b3c7677697696bd4f9e8e8dc43f4dc87db261ae9d"
    );
    assert_eq!(erase["artifact"]["spelling_map_offset_raw"], 0);
    assert_eq!(erase["artifact"]["spelling_map_offset_is_null"], true);

    for fixture_case in cases {
        let artifact = &fixture_case["artifact"];
        let path = artifact["path"]
            .as_str()
            .expect("artifact path should be text");
        let row = manifest["files"]
            .as_array()
            .expect("oracle manifest files should be an array")
            .iter()
            .find(|row| row["path"] == path)
            .unwrap_or_else(|| panic!("oracle manifest should register {path}"));
        assert_eq!(row["sha256"], artifact["sha256"]);
        assert_eq!(row["observation_fixture"], "m59-algebra-properties.json");
    }
}

#[test]
fn correction_spelling_fixture_and_preserved_text_packet_are_hash_bound() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_root
        .parent()
        .and_then(Path::parent)
        .expect("crate should live two levels below repository root");
    let fixture_path = crate_root.join(CORRECTION_FIXTURE);
    assert_eq!(sha256(&fixture_path), CORRECTION_FIXTURE_SHA256);
    let fixture = read_json(&fixture_path);

    let manifest = read_json(&crate_root.join(MANIFEST));
    let row = manifest["files"]
        .as_array()
        .expect("oracle manifest files should be an array")
        .iter()
        .find(|row| row["path"] == "m59-correction-spelling.json")
        .expect("oracle manifest should register the correction fixture");
    assert_eq!(row["sha256"], CORRECTION_FIXTURE_SHA256);
    assert_eq!(row["milestone"], "M59 Increment 3b");
    assert_eq!(
        row["source_evidence_manifest_sha256"],
        "4cb688f0624a7c19dd7a35b506aec0f30419f62a4eee0f93911d8caf7c6dcf48"
    );

    let source_capture = &fixture["source_capture"];
    let source_path = repo_root.join(
        source_capture["path"]
            .as_str()
            .expect("preserved capture path should be text"),
    );
    assert_eq!(
        sha256(&source_path),
        source_capture["sha256"]
            .as_str()
            .expect("preserved capture SHA should be text")
    );
    let subset_path = repo_root.join(
        source_capture["repo_text_subset_manifest"]
            .as_str()
            .expect("subset manifest path should be text"),
    );
    assert_eq!(
        sha256(&subset_path),
        row["repo_text_subset_manifest_sha256"]
            .as_str()
            .expect("subset manifest SHA should be text")
    );
    let decoder_path = repo_root.join(
        source_capture["decoder_path"]
            .as_str()
            .expect("decoder path should be text"),
    );
    assert_eq!(
        sha256(&decoder_path),
        source_capture["decoder_sha256"]
            .as_str()
            .expect("decoder SHA should be text")
    );
    assert_eq!(row["decoder_sha256"], source_capture["decoder_sha256"]);
}

#[test]
fn yune_generated_prism_matches_upstream_same_surface_collision_merge() {
    let fixture = fixture();
    let case = case(cases(&fixture), "same_surface_same_syllable_collision");
    let upstream = upstream_prism(case);
    assert_surface(&upstream, "hao", &case["surfaces"]["hao"]);
    assert_surface(&upstream, "hx", &case["surfaces"]["hx"]);
    let runtime = upstream_runtime_prism(case);
    assert_runtime_surface(&runtime, case, "hao", &case["surfaces"]["hao"]);
    assert_runtime_surface(&runtime, case, "hx", &case["surfaces"]["hx"]);

    let payload = generated_prism(case);

    assert_surface(&payload, "hao", &case["surfaces"]["hao"]);
    assert_surface(&payload, "hx", &case["surfaces"]["hx"]);
}

#[test]
fn yune_generated_prism_preserves_properties_through_ordinary_xform() {
    let fixture = fixture();
    let case = case(cases(&fixture), "ordinary_xform_preserves_properties");
    let upstream = upstream_prism(case);
    let runtime = upstream_runtime_prism(case);
    for surface in ["bei", "bz", "cei", "cz", "dei", "dz", "bx", "cx", "dx"] {
        assert_surface(&upstream, surface, &case["surfaces"][surface]);
        assert_runtime_surface(&runtime, case, surface, &case["surfaces"][surface]);
    }

    let payload = generated_prism(case);

    for surface in ["bei", "bz", "cei", "cz", "dei", "dz", "bx", "cx", "dx"] {
        assert_surface(&payload, surface, &case["surfaces"][surface]);
    }
}

#[test]
fn yune_generated_prism_accumulates_deployed_credibility_as_double() {
    let fixture = fixture();
    let case = case(
        cases(&fixture),
        "cumulative_credibility_uses_double_until_emission",
    );
    let upstream = upstream_prism(case);
    let runtime = upstream_runtime_prism(case);
    for surface in ["bei", "fi", "ci", "di"] {
        assert_surface(&upstream, surface, &case["surfaces"][surface]);
        assert_runtime_surface(&runtime, case, surface, &case["surfaces"][surface]);
    }

    let payload = generated_prism(case);
    for surface in ["bei", "fi", "ci", "di"] {
        assert_surface(&payload, surface, &case["surfaces"][surface]);
    }
    assert_eq!(
        case["surfaces"]["di"]["descriptors"][0]["credibility_f32_bits"],
        "0xC11E74AF",
        "the pinned cumulative path must distinguish double accumulation from the one-ULP-lower stepwise-f32 result"
    );
}

#[test]
fn yune_generated_prism_keeps_partial_match_erase_inert() {
    let fixture = fixture();
    let case = case(cases(&fixture), "partial_match_erase_is_inert");
    let upstream = upstream_prism(case);
    assert_surface(&upstream, "hao", &case["surfaces"]["hao"]);
    assert_surface(&upstream, "ho", &case["surfaces"]["ho"]);

    let payload = generated_prism(case);
    let double_array = payload
        .double_array
        .as_ref()
        .expect("Yune-generated prism should carry its double array");

    assert_eq!(double_array.exact_match("hao"), Some(0));
    assert_eq!(double_array.exact_match("ho"), None);
    assert_eq!(payload.num_syllables, 1);
    assert_eq!(payload.num_spellings, 1);

    assert_surface(&payload, "hao", &case["surfaces"]["hao"]);
    assert_surface(&payload, "ho", &case["surfaces"]["ho"]);
}

#[test]
fn upstream_identity_prism_runtime_path_stays_byte_backed_and_implicit() {
    let fixture = fixture();
    let case = case(cases(&fixture), "partial_match_erase_is_inert");
    let runtime = upstream_runtime_prism(case);
    let syllabary = string_array(&case["syllabary"]);

    let hao = runtime.lookup_canonical_codes("hao", &syllabary);
    assert_eq!(hao.len(), 1);
    assert_eq!(hao[0].code, "hao");
    assert!(!hao[0].abbreviation);
    assert!(!hao[0].correction);
    assert_eq!(hao[0].credibility.to_bits(), 0);
    assert!(runtime.lookup_canonical_codes("ho", &syllabary).is_empty());

    assert_identity_runtime_owner_rows(&runtime);
}

#[test]
fn identity_prism_rejects_spelling_count_larger_than_its_darts_storage() {
    let fixture = fixture();
    let case = case(cases(&fixture), "partial_match_erase_is_inert");
    let mut bytes = upstream_prism_bytes(case);
    let large_count = 157_000_u32;
    bytes[40..44].copy_from_slice(&large_count.to_le_bytes());
    bytes[44..48].copy_from_slice(&large_count.to_le_bytes());
    let source: Arc<dyn CompactTableByteSource> =
        Arc::new(UpstreamPrismByteSource(Arc::from(bytes)));
    assert!(matches!(
        parse_rime_prism_runtime_payload(source),
        Err(yune_core::RimePrismBinParseError::InvalidCount)
    ));
}

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    read_json(&path)
}

fn read_json(path: &Path) -> Value {
    let json = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&json)
        .unwrap_or_else(|error| panic!("invalid JSON fixture {}: {error}", path.display()))
}

fn sha256(path: &Path) -> String {
    let bytes =
        fs::read(path).unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn cases(fixture: &Value) -> &[Value] {
    fixture["cases"]
        .as_array()
        .expect("fixture cases should be an array")
}

fn case<'a>(cases: &'a [Value], id: &str) -> &'a Value {
    cases
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("missing fixture case {id}"))
}

fn generated_prism(case: &Value) -> RimePrismBinPayload {
    let syllabary = string_array(&case["syllabary"]);
    let formulas = string_array(&case["formulas"]);
    let bytes = build_prism_bin(&syllabary, &formulas, 0x1111_1111, 0x2222_2222);
    parse_rime_prism_bin_payload(&bytes).expect("Yune-generated prism should parse")
}

fn upstream_prism(case: &Value) -> RimePrismBinPayload {
    let bytes = upstream_prism_bytes(case);
    let relative = case["artifact"]["path"]
        .as_str()
        .expect("artifact path should be a string");
    parse_rime_prism_bin_payload(bytes)
        .unwrap_or_else(|error| panic!("production parser rejected {relative}: {error:?}"))
}

fn upstream_runtime_prism(case: &Value) -> RimePrismRuntimePayload {
    let bytes = upstream_prism_bytes(case);
    let source: Arc<dyn CompactTableByteSource> =
        Arc::new(UpstreamPrismByteSource(Arc::from(bytes)));
    parse_rime_prism_runtime_payload(source)
        .expect("production runtime parser should accept the external upstream prism")
}

fn upstream_prism_bytes(case: &Value) -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        Path::new(FIXTURE)
            .parent()
            .expect("fixture should have a parent"),
    );
    let artifact = &case["artifact"];
    let relative = artifact["path"]
        .as_str()
        .expect("artifact path should be a string");
    let path = root.join(relative);
    assert_eq!(
        sha256(&path),
        artifact["sha256"]
            .as_str()
            .expect("artifact SHA should be a string"),
        "external upstream prism hash differs for {}",
        path.display()
    );
    let bytes = fs::read(&path).unwrap_or_else(|error| {
        panic!("failed to read upstream prism {}: {error}", path.display())
    });
    assert_eq!(
        bytes.len() as u64,
        artifact["bytes"]
            .as_u64()
            .expect("artifact byte count should be numeric")
    );
    bytes
}

fn assert_identity_runtime_owner_rows(runtime: &RimePrismRuntimePayload) {
    let rows = runtime.memory_owner_rows();
    let row = |owner: &str| {
        rows.iter()
            .find(|row| row.owner == owner)
            .unwrap_or_else(|| panic!("missing runtime owner row {owner}"))
    };
    let double_array = row("prism.double_array_units");
    assert_eq!(double_array.class, MemoryOwnerClass::MmapFileBacked);
    assert!(double_array.estimated_bytes > 0);
    assert!(double_array.item_count > 0);

    let spelling_map = row("prism.spelling_map");
    assert_eq!(spelling_map.class, MemoryOwnerClass::MmapFileBacked);
    assert_eq!(spelling_map.estimated_bytes, 0);
    assert_eq!(spelling_map.item_count, 0);
    let tips = row("prism.tips_payload");
    assert_eq!((tips.estimated_bytes, tips.item_count), (0, 0));
}

fn string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("fixture field should be an array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("fixture array value should be a string")
                .to_owned()
        })
        .collect()
}

fn assert_surface(payload: &RimePrismBinPayload, surface: &str, expected: &Value) {
    let index = payload
        .double_array
        .as_ref()
        .and_then(|array| array.exact_match(surface));
    let expected_present = expected["present"]
        .as_bool()
        .expect("surface present flag should be boolean");
    assert_eq!(
        index.is_some(),
        expected_present,
        "surface presence differs for {surface}"
    );
    let Some(index) = index else {
        return;
    };
    assert_eq!(
        u64::from(index),
        expected["spelling_index"]
            .as_u64()
            .expect("present surface should carry spelling_index"),
        "spelling index differs for {surface}"
    );
    let actual = payload
        .spelling_map
        .get(index as usize)
        .expect("spelling index should resolve");
    let expected_rows = expected["descriptors"]
        .as_array()
        .expect("present surface should carry descriptor rows");
    assert_eq!(
        actual.len(),
        expected_rows.len(),
        "descriptor count differs for {surface}: actual={actual:?}"
    );
    for (row, (actual, expected)) in actual.iter().zip(expected_rows).enumerate() {
        assert_descriptor(actual, expected, surface, row);
    }
}

fn assert_runtime_surface(
    runtime: &RimePrismRuntimePayload,
    case: &Value,
    surface: &str,
    expected: &Value,
) {
    let syllabary = string_array(&case["syllabary"]);
    let actual = runtime.lookup_canonical_codes(surface, &syllabary);
    let expected_rows = expected["descriptors"]
        .as_array()
        .expect("surface observation should carry descriptor rows");
    assert_eq!(
        actual.len(),
        expected_rows.len(),
        "runtime descriptor count differs for external surface {surface}"
    );
    for (row, (actual, expected)) in actual.iter().zip(expected_rows).enumerate() {
        let syllable_id = expected["syllable_id"]
            .as_u64()
            .expect("descriptor syllable id should be nonnegative")
            as usize;
        assert_eq!(
            actual.code, syllabary[syllable_id],
            "runtime canonical code differs for {surface} row {row}"
        );
        assert_eq!(
            actual.abbreviation,
            expected["spelling_type"] == 2,
            "runtime abbreviation flag differs for {surface} row {row}"
        );
        assert_eq!(
            actual.correction,
            expected["is_correction"]
                .as_bool()
                .expect("descriptor correction should be boolean"),
            "runtime correction flag differs for {surface} row {row}"
        );
        assert_eq!(
            format!("0x{:08X}", actual.credibility.to_bits()),
            expected["credibility_f32_bits"]
                .as_str()
                .expect("descriptor credibility bits should be a string"),
            "runtime credibility differs for {surface} row {row}"
        );
    }
}

fn assert_descriptor(
    actual: &yune_core::RimePrismSpellingDescriptor,
    expected: &Value,
    surface: &str,
    row: usize,
) {
    assert_eq!(
        i64::from(actual.syllable_id),
        expected["syllable_id"]
            .as_i64()
            .expect("descriptor syllable_id should be an integer"),
        "syllable id differs for {surface} row {row}"
    );
    assert_eq!(
        i64::from(actual.spelling_type),
        expected["spelling_type"]
            .as_i64()
            .expect("descriptor spelling_type should be an integer"),
        "spelling type differs for {surface} row {row}"
    );
    assert_eq!(
        actual.is_correction,
        expected["is_correction"]
            .as_bool()
            .expect("descriptor correction flag should be boolean"),
        "correction flag differs for {surface} row {row}"
    );
    assert_eq!(
        format!("0x{:08X}", actual.credibility.to_bits()),
        expected["credibility_f32_bits"]
            .as_str()
            .expect("descriptor credibility bits should be a string"),
        "credibility bits differ for {surface} row {row}"
    );
    assert_eq!(
        actual.tips,
        expected["tips"]
            .as_str()
            .expect("descriptor tips should be a string"),
        "tips differ for {surface} row {row}"
    );
}
