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
//! `docs/reports/evidence/m59-cangjie5-order-parity/`): Yune's full-code ☯-sentence
//! renders the concatenated code as root glyphs instead of segmenting+composing, so
//! the one-shot full-code composition is unsupported. Char-by-char composition (the
//! D-47 guarantee) DOES work and is asserted live below.

use std::{collections::BTreeMap, fs, path::Path};

use serde_json::Value;
use yune_core::{StaticTableTranslator, TableDictionary, Translator};

const FIXTURE: &str = "tests/fixtures/upstream-1.17.0/cangjie5-composition.json";

/// Locks the oracle capture: pinned provenance + the decisive answer that librime
/// composes each owner phrase at candidate 0. No Yune involved — a pure oracle lock.
#[test]
fn upstream_cangjie5_composition_fixture_is_locked() {
    let fixture = fixture();
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

    // The owner composition rows are oracle-backed: librime composes each phrase at
    // candidate 0 (the highlighted / commit-preview slot).
    for (input, target) in OWNER_COMPOSITION_ROWS {
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
    }

    let rows = fixture["composition_rows"]
        .as_array()
        .expect("composition_rows provenance should be an array");
    assert_eq!(rows.len(), 3, "three owner composition rows are recorded");
}

/// Real production translator path over UPSTREAM rime-cangjie rows: for every single
/// composition constituent, Yune's top candidate is the oracle's candidate-0 character,
/// so an arbitrary non-lexicon phrase is reachable one character at a time
/// (D-47 / M59-REACH-02). This is a composition-REACHABILITY guard — it proves the
/// constituent character is produced (not dropped or replaced by a code glyph), which
/// is the CJ-1 failure mode. It is NOT a weight-ranking discriminator: for these seven
/// codes the oracle order coincides with dictionary insertion order, so by_weight
/// ranking is exercised by the M19 `upstream_cangjie_parity` lane, not here. Expected
/// values are the oracle's candidate 0 — never Yune-derived.
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

/// The full-code one-shot ☯-sentence composition librime performs
/// (`hwmvsqtt`→粵拼, `ebcnyripm`→測試, `takohaeosk`→莫伯洢, `hdaetcu`→香港, all at
/// candidate 0 — locked by the fixture) is NOT reproduced by Yune's real path:
/// Yune renders the concatenated code as root glyphs instead of segmenting and
/// composing. This is the M17-blocked cangjie phrase/table-encoder/sentence
/// interleave area. Char-by-char composition works
/// (`yune_cangjie5_composes_each_constituent_char_at_top`). No silent gap — owner
/// disposition tracked in the CJ-1 finding.
#[test]
#[ignore = "blocked: Yune cangjie full-code ☯-sentence segmentation composition gap; oracle composes 粵拼/測試/莫伯洢/香港 @0 (fixture cangjie5-composition.json). Finding CJ-1 (docs/reports/evidence/m59-cangjie5-order-parity/). Char-by-char composition is supported and asserted separately."]
fn cangjie5_full_code_sentence_composition_is_blocked() {
    panic!(
        "enable only after Yune composes the full concatenated cangjie code into the oracle's \
         ☯-sentence phrase (owner decision on finding CJ-1)"
    );
}

const OWNER_COMPOSITION_ROWS: [(&str, &str); 3] = [
    ("hwmvsqtt", "粵拼"),
    ("ebcnyripm", "測試"),
    ("takohaeosk", "莫伯洢"),
];

fn fixture() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("invalid JSON {path:?}: {error}"))
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
