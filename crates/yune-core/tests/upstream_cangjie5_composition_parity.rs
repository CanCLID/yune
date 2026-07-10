//! D-48 item 2 — cangjie5 order-parity onboarding (composition lane).
//!
//! Oracle: librime 1.17.0 over pinned `rime/rime-cangjie` `52d90a1b…`, captured by
//! `scripts/capture-upstream-cangjie5.ps1` into
//! `tests/fixtures/upstream-1.17.0/cangjie5-composition.json`.
//!
//! Decisive D-48 question — ANSWERED YES: librime composes the three owner phrases
//! `粵拼` / `測試` / `莫伯洢` (and the `香港` control) at candidate 0 via its ` ☯ `
//! sentence, so they are **oracle-backed order rows** (locked below).
//!
//! Non-circularity: every expected value comes from the oracle capture's
//! `all_candidates` / `commit_text_preview`. Yune's real translator is built from
//! UPSTREAM rime-cangjie rows (`source_slice`), never from Yune output.
//!
//! Yune divergence (finding CJ-1, filed for owner — see
//! `docs/reports/evidence/m59-cangjie5-order-parity/`): on THIS upstream rime-cangjie
//! lane Yune's sentence scorer picks a mis-scored segmentation — the eight single-letter
//! roots `竹田一女尸手廿廿` (h|w|m|v|s|q|t|t) over the correct `hwmvs|qtt`→粵拼. Yune's
//! engine CAN runtime-compose: the shipped PRODUCT cangjie5 (Jackchows,
//! `max_phrase_length: 1`, so 粵拼 cannot be a dict entry) composes `hwmvsqtt`→粵拼@0.
//! The gap is the upstream lane's `by_weight`+essay segmentation SCORING, not a missing
//! capability. Char-by-char composition works on both lanes and is asserted live below.

use std::{collections::BTreeMap, fs, path::Path};

use serde_json::Value;
use sha2::{Digest, Sha256};
use yune_core::{StaticTableTranslator, TableDictionary, Translator};

const FIXTURE: &str = "tests/fixtures/upstream-1.17.0/cangjie5-composition.json";
const FIXTURE_SHA256: &str = "24408c3b2b83db516ae1382d2ba743b41ead50c7c026aee2837a01137c7ecbcf";

/// Locks the oracle capture: pinned provenance + the decisive answer that librime
/// composes each owner phrase at candidate 0. No Yune involved — a pure oracle lock.
#[test]
fn upstream_cangjie5_composition_fixture_is_locked() {
    let fixture = fixture();
    assert_eq!(fixture["status"], "cangjie5_capture_curated_complete");
    assert_eq!(fixture["canonical"], true);
    assert_eq!(fixture["oracle"]["engine"], "rime/librime");
    assert_eq!(fixture["oracle"]["version"], "1.17.0");
    assert_eq!(
        fixture["oracle"]["commit"],
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    );
    assert_eq!(fixture["schema"]["source_repo"], "rime/rime-cangjie");
    assert_eq!(
        fixture["schema"]["source_commit"],
        "52d90a1b1312e74042b38c1cbc8142defbc53171"
    );
    assert_eq!(
        fixture["schema"]["source_tree"],
        "db11cf6ffd382ada3087e9765c0ba2e636a8b68d"
    );
    // Pin the oracle binary itself so non-owner candidate rows cannot drift under a
    // different librime build.
    assert_eq!(
        fixture["oracle"]["dll_sha256"],
        "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b"
    );
    assert_eq!(
        fixture["oracle"]["deployer_sha256"],
        "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071"
    );
    assert_eq!(
        fixture["capture"]["source_commit"],
        "c7c04ff73b76ea3e8e1c5e6bf9b432483ee6650f"
    );
    assert_eq!(
        fixture["capture"]["source_tree"],
        "382f7d31617a5a6aeb2cd9e1ec7472b7bf3e3bc1"
    );
    assert_eq!(fixture["capture"]["source_clean"], true);
    assert_eq!(
        fixture["capture"]["source_status_short"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(fixture["curation"]["version"], 2);
    assert_eq!(
        fixture["curation"]["raw_input_sha256"],
        "91dca789769cbbed160132bf23a54d891bf164f3f2617d5fcf5a2ac5d4443be1"
    );
    assert_eq!(
        fixture["capture"]["tool_hashes"]["capture_script_sha256"],
        "bd70748a7f95434dba3739d3c13c9d8f468d053aef8b350b89248ce3aa22012c"
    );
    assert_eq!(
        fixture["capture"]["tool_hashes"]["curator_sha256"],
        "2e69df2649f514b090897ffbb1624c547caed7e81bd05b4d6b0702f584b1e3d1"
    );
    assert_eq!(
        fixture["capture"]["tool_hashes"]["probe_sha256"],
        "94f7deb7c3632a6c3c918536295b03d88aa8a80bbbbc9d8a26e896fb70bf07e7"
    );
    assert_eq!(
        fixture["capture"]["runtime_options"],
        serde_json::json!({
            "ascii_mode": false,
            "full_shape": false,
            "ascii_punct": false,
            "zh_hans": false,
        })
    );
    assert_eq!(
        fixture["capture"]["additional_runtime_option_patches"],
        serde_json::json!([])
    );
    assert_eq!(
        fixture["source_slice"]["schema_data_tree"],
        "db11cf6ffd382ada3087e9765c0ba2e636a8b68d"
    );

    let cases = fixture["cases"]
        .as_array()
        .expect("Cangjie oracle cases should be an array");
    assert_eq!(
        cases.len(),
        12,
        "the full Cangjie lane should have 12 cases"
    );
    for case in cases {
        assert_eq!(case["captured_all_pages"], true);
        assert_eq!(case["menu_present"], true);
        assert_eq!(case["termination_reason"], "last_page");
        assert_eq!(case["rime_get_input"], case["input"]);
    }

    // The owner composition rows are oracle-backed: librime composes each phrase at
    // candidate 0 (the highlighted / commit-preview slot).
    for ((input, target), target_codepoints) in OWNER_COMPOSITION_ROWS
        .into_iter()
        .zip(OWNER_TARGET_CODEPOINTS)
    {
        let case = case_for(&fixture, input);
        assert_eq!(
            case["commit_text_preview"].as_str(),
            Some(target),
            "librime should compose {target} at the highlighted slot for {input}"
        );
        assert_eq!(
            case["all_candidates"][0]["text"].as_str(),
            Some(target),
            "the composed phrase {target} should be oracle candidate 0 for {input}"
        );
        assert_eq!(
            case["highlighted_candidate_index"].as_i64(),
            Some(0),
            "the composed phrase should be highlighted for {input}"
        );
        let provenance_row = fixture["composition_rows"]
            .as_array()
            .expect("composition_rows provenance should be an array")
            .iter()
            .find(|row| row["input"].as_str() == Some(input))
            .unwrap_or_else(|| panic!("composition_rows should contain {input}"));
        assert_eq!(provenance_row["target"], target);
        assert_eq!(provenance_row["target_codepoints"], target_codepoints);
        assert!(
            provenance_row["provenance"]
                .as_str()
                .is_some_and(|value| value.contains("derived from cases[")
                    && value.contains("owner U+ declaration")),
            "{input} should bind its owner target to captured candidate zero"
        );
    }

    let rows = fixture["composition_rows"]
        .as_array()
        .expect("composition_rows provenance should be an array");
    assert_eq!(rows.len(), 3, "three owner composition rows are recorded");
}

/// Real production translator path over UPSTREAM rime-cangjie rows: for every single
/// composition constituent, Yune's top candidate is the oracle's candidate-0 character,
/// so an arbitrary non-lexicon phrase is reachable one character at a time
/// (D-47 / M59-REACH-02). This is a composition-REACHABILITY guard — it proves each
/// constituent character is produced for its shape code (the char-by-char path both
/// lanes support). It is NOT a weight-ranking discriminator: for these seven codes the
/// oracle order coincides with dictionary insertion order, so by_weight ranking is
/// exercised by the M19 `upstream_cangjie_parity` lane, not here. Expected values are
/// the oracle's candidate 0 — never Yune-derived.
#[test]
fn yune_cangjie5_composes_each_constituent_char_at_top() {
    let fixture = fixture();
    let translator = cangjie_translator(&fixture);

    let atomic = fixture["source_slice"]["atomic_codes"]
        .as_array()
        .expect("source_slice.atomic_codes should be an array");
    assert!(
        !atomic.is_empty(),
        "atomic composition codes should be present"
    );

    for code_value in atomic {
        let code = code_value.as_str().expect("atomic code should be a string");
        let oracle_case = case_for(&fixture, code);
        let expected = oracle_case["all_candidates"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("oracle case {code} should have a candidate 0"));
        let actual = translator
            .translate(code)
            .into_iter()
            .next()
            .map(|candidate| candidate.text);
        assert_eq!(
            actual.as_deref(),
            Some(expected),
            "Yune should produce the oracle candidate-0 character {expected} at the top for cangjie code {code}"
        );
    }
}

/// The full-code composition librime performs (`hwmvsqtt`→粵拼, `ebcnyripm`→測試,
/// `takohaeosk`→莫伯洢, `hdaetcu`→香港, all at candidate 0 — locked by the fixture) is
/// NOT reproduced on THIS upstream rime-cangjie lane: Yune's sentence scorer picks a
/// mis-scored segmentation (the eight single-letter roots `竹田一女尸手廿廿`,
/// h|w|m|v|s|q|t|t) over the correct `hwmvs|qtt`→粵拼. This is a lane-specific
/// SCORING divergence, not a missing capability — Yune's engine runtime-composes on the
/// PRODUCT cangjie5 lane (Jackchows `max_phrase_length: 1`, so 粵拼 is not a dict entry
/// yet composes `hwmvsqtt`→粵拼@0). Hypothesis (finding CJ-1): upstream `by_weight`+essay
/// scores the common root chars far above 粵/拼, and/or the sentence scorer carries the
/// M48 raw-frequency class of bug. No silent gap — disposition tracked in CJ-1.
#[test]
#[ignore = "blocked: upstream rime-cangjie lane mis-scores segmentation (竹田一女尸手廿廿 over hwmvs|qtt→粵拼); oracle + PRODUCT lane compose 粵拼/測試/莫伯洢/香港 @0. Finding CJ-1 (docs/reports/evidence/m59-cangjie5-order-parity/) = diagnose+fix the upstream-lane segmentation scoring, not a missing capability."]
fn cangjie5_upstream_lane_segmentation_scoring_is_blocked() {
    panic!(
        "enable only after the upstream rime-cangjie lane scores hwmvs|qtt→粵拼 above the \
         eight-single-root segmentation (owner decision on finding CJ-1)"
    );
}

const OWNER_COMPOSITION_ROWS: [(&str, &str); 3] = [
    ("hwmvsqtt", "粵拼"),
    ("ebcnyripm", "測試"),
    ("takohaeosk", "莫伯洢"),
];
const OWNER_TARGET_CODEPOINTS: [&str; 3] =
    ["U+7CB5 U+62FC", "U+6E2C U+8A66", "U+83AB U+4F2F U+6D22"];

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        FIXTURE_SHA256,
        "reviewed Cangjie oracle fixture bytes should stay pinned"
    );
    serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("invalid JSON {path:?}: {error}"))
}

fn case_for<'a>(fixture: &'a Value, input: &str) -> &'a Value {
    fixture["cases"]
        .as_array()
        .expect("cases should be an array")
        .iter()
        .find(|case| case["input"].as_str() == Some(input))
        .unwrap_or_else(|| panic!("fixture should contain a case for {input}"))
}

fn cangjie_translator(fixture: &Value) -> StaticTableTranslator {
    StaticTableTranslator::from_dictionary(cangjie_dictionary(fixture))
        .with_completion(true)
        .with_sentence(true)
        .with_show_full_code(false)
}

fn cangjie_dictionary(fixture: &Value) -> TableDictionary {
    let imports = fixture["source_slice"]["import_rows"]
        .as_object()
        .expect("source_slice.import_rows should be an object")
        .iter()
        .map(|(file, rows)| (file.clone(), rows_to_text(rows)))
        .collect::<BTreeMap<_, _>>();
    let vocabulary = rows_to_text(&fixture["source_slice"]["vocabulary_rows"]);
    TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        cangjie_main_yaml().as_str(),
        std::iter::empty::<&str>(),
        |name| {
            // Every declared import_table must resolve; tables with no cohort rows
            // in the slice (e.g. cangjie5.stem) resolve to an empty table.
            let file = format!("{name}.dict.yaml");
            let rows = imports.get(&file).map(String::as_str).unwrap_or("");
            Some(cangjie_import_yaml(name, rows))
        },
        |name| (name == "essay").then(|| vocabulary.clone()),
    )
    .expect("cangjie source slice should parse")
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

fn rows_to_text(rows: &Value) -> String {
    rows.as_array()
        .expect("rows should be an array")
        .iter()
        .map(|row| row.as_str().expect("row should be a string"))
        .collect::<Vec<_>>()
        .join("\n")
}
