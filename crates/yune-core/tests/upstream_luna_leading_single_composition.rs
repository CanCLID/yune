//! M59 finding #7: oracle provenance for luna_pinyin leading-single reachability
//! and partial-selection composition, captured from rime/librime 1.17.0 via
//! scripts/capture-m59-luna-composition.ps1 (reproducible; see the fixture's
//! `capture` block and the upstream-1.17.0 oracle-manifest). These assertions pin
//! the ORACLE's behavior, so the acceptance rows the M59 luna tests exercise
//! (crates/yune-rime-api/tests/yune_web.rs `m59_luna_*`) are oracle-grounded here,
//! never Yune-derived (conventions §7).
//!
//! Recorded divergence (NOT asserted against Yune): Yune's PRODUCT surfaces the
//! leading single earlier than librime — e.g. `zhongguo` → 中 sits at librime
//! global index 11, while Yune injects it onto page 0 — because Yune's
//! completion/sentence ordering differs from librime. M59 asserts REACHABILITY +
//! partial-selection recompose, not candidate-position parity with librime.

use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const FIXTURE: &str = "tests/fixtures/upstream-1.17.0/m59-luna-leading-single-composition.json";

fn load_fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let body = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_str(&body).unwrap_or_else(|error| panic!("invalid JSON {path:?}: {error}"))
}

#[test]
fn oracle_provenance_header_is_librime_1_17_0() {
    let fixture = load_fixture();
    assert_eq!(fixture["oracle"]["engine"], "rime/librime");
    assert_eq!(fixture["oracle"]["engine_tag"], "1.17.0");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    );
    assert_eq!(fixture["oracle"]["schema"], "luna_pinyin");
}

fn composition_preedits<'a>(fixture: &'a Value, phrase: &str) -> Vec<&'a str> {
    fixture["compositions"][phrase]["chain"]
        .as_array()
        .unwrap_or_else(|| panic!("composition chain for {phrase} should be an array"))
        .iter()
        .filter_map(|step| step["preedit"].as_str())
        .collect()
}

#[test]
fn oracle_composes_moboyi_to_the_non_lexicon_phrase_by_partial_selection() {
    // PRIMARY M59 case: librime composes 莫伯洢 (NOT in the lexicon) from `moboyi`
    // by selecting the leading single at each step — the selected char accumulates
    // in the preedit and the remainder recomposes, committing the full phrase only
    // after the last selection. The rare 洢 (yi) sits deep (oracle index 155) but
    // is reachable; this is the oracle provenance the m59 moboyi acceptance rides.
    let fixture = load_fixture();
    assert_eq!(
        fixture["compositions"]["moboyi"]["final_commit"], "\u{83ab}\u{4f2f}\u{6d22}",
        "librime must compose 莫伯洢 from moboyi by partial single-character selection"
    );
    let preedits = composition_preedits(&fixture, "moboyi");
    assert!(
        preedits.contains(&"\u{83ab}bo yi"),
        "selecting 莫 must recompose the remainder to `bo yi`; chain={preedits:?}"
    );
    assert!(
        preedits.contains(&"\u{83ab}\u{4f2f}yi"),
        "selecting 伯 must recompose the remainder to `yi`; chain={preedits:?}"
    );
}

#[test]
fn oracle_reaches_each_leading_single_at_captured_position() {
    // Every acceptance input's leading single is reachable in librime's candidate
    // list at a finite global index — the oracle positions the M59 reachability
    // rows cite. (zhongguo 中@11 documents the position Yune's product diverges
    // from; both are reachable, which is what M59 requires.)
    let fixture = load_fixture();
    for (input, expected_index) in [
        // PRIMARY moboyi -> 莫伯洢 chain: 莫, 伯, and the rare 洢 (deep but reachable).
        ("moboyi", 2_u64),
        ("boyi", 19),
        ("yi", 155),
        // zhongguo/zhonggao class + guo/gao remainders.
        ("zhonggao", 3),
        ("zhongguo", 11),
        ("gao", 0),
        ("guo", 1),
    ] {
        let row = &fixture["inputs"][input];
        assert_eq!(
            row["target_global_index"].as_u64(),
            Some(expected_index),
            "librime `{input}` leading single must sit at oracle global index {expected_index}"
        );
        assert!(
            row["target_global_index"].as_u64().unwrap()
                < row["total_unique_captured"].as_u64().unwrap(),
            "`{input}` single must be within the captured page range (reachable)"
        );
    }
}
