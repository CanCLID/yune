//! M59 oracle provenance for luna_pinyin leading-single reachability,
//! partial-selection composition, and complete Lane B candidate order, captured
//! from rime/librime 1.17.0 via
//! scripts/capture-m59-luna-composition.ps1 (reproducible; see the fixture's
//! `capture` block and the upstream-1.17.0 oracle-manifest). These assertions pin
//! the ORACLE's behavior, so the acceptance rows the M59 luna tests exercise
//! (crates/yune-rime-api/tests/yune_web.rs `m59_luna_*`) are oracle-grounded here,
//! never Yune-derived (conventions §7).
//!
//! Recorded divergence (NOT asserted against Yune): Yune's PRODUCT surfaces the
//! leading single earlier than librime — e.g. `zhongguo` → 中 sits at librime
//! global index 11, while Yune injects it onto page 0 — because Yune's
//! completion/sentence ordering differs from librime. D-48 uses this complete
//! capture as the exact-order oracle for the remaining M59 closure work.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const FIXTURE: &str = "tests/fixtures/upstream-1.17.0/m59-luna-leading-single-composition.json";

fn load_fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let body = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {path:?}: {error}"));
    serde_json::from_str(&body).unwrap_or_else(|error| panic!("invalid JSON {path:?}: {error}"))
}

fn ordered_text_sha256(candidates: &[Value]) -> String {
    let mut digest = Sha256::new();
    for candidate in candidates {
        let text = candidate["text"]
            .as_str()
            .expect("captured candidate text should be a string");
        let bytes = text.as_bytes();
        digest.update((bytes.len() as u64).to_be_bytes());
        digest.update(bytes);
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

#[test]
fn oracle_capture_binds_clean_deterministic_query_bytes() {
    let fixture = load_fixture();
    assert_eq!(
        fixture["oracle"]["dll_sha256"],
        "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b"
    );
    assert_eq!(
        fixture["oracle"]["deployer_sha256"],
        "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071"
    );
    let schema = &fixture["schema"];
    assert_eq!(
        schema["source_commit"],
        "18a80335c37522311f7cff02886cd81cec3b460a"
    );
    for (repository, commit, tree) in [
        (
            "rime/rime-luna-pinyin",
            "18a80335c37522311f7cff02886cd81cec3b460a",
            "0d5efcb75aa40689bf3be210a4f056db6d77b49d",
        ),
        (
            "rime/rime-prelude",
            "082425ea0684bca36474415d4a0e8db9b016487e",
            "d7e128f09ce6b1f920729ef2f848ca1294c9cb31",
        ),
        (
            "rime/rime-essay",
            "48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed",
            "4769c4ef6c5f93f450c5f36c2c9ac5e6845d37bc",
        ),
        (
            "rime/rime-stroke",
            "3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c",
            "d60c793d8d68154847923f21aa73ba90441dab32",
        ),
    ] {
        assert_eq!(
            schema["source_repositories_clean"][repository], true,
            "{repository} must be clean for an attributable oracle capture"
        );
        if repository == "rime/rime-luna-pinyin" {
            assert_eq!(schema["source_commit"], commit);
        } else {
            assert_eq!(schema["dependency_commits"][repository], commit);
        }
        assert_eq!(schema["source_git_trees"][repository], tree);
    }

    let capture = &fixture["capture"];
    assert_eq!(
        capture["source_row_policy"],
        "m59_lane_b_complete_order_and_partial_selection_composition"
    );
    assert_eq!(capture["curator_version"], 5);
    assert_eq!(
        capture["order_hash_algorithm"],
        "sha256 of repeated u64be utf8-byte-length followed by utf8 candidate text"
    );
    let queried = &fixture["capture"]["queried_data"];
    assert_eq!(queried["shared_path"], "disposable/shared");
    assert_eq!(queried["build_path"], "disposable/user/build");
    assert_eq!(
        queried["shared_tree_sha256"],
        "2280114cfc83ada16f3898e6531ff573a7b33a39f1e679695e06eaf8778f3b37"
    );
    assert_eq!(
        queried["build_tree_sha256"],
        "702c223ffbd282981b9573f881b69f6d97451fe7506ac97cbcecc74c8a4bcc1a"
    );
    assert_eq!(queried["staged_timestamp_utc"], "2000-01-01T00:00:00.500Z");
    assert_eq!(
        queried["tree_hash_algorithm"],
        "sha256 of ordinal path<TAB>file-sha256 rows joined by LF with final LF"
    );
    assert_eq!(
        queried["mutation_policy"],
        "raw shared/build hashes must remain identical before/after capture"
    );
    assert_eq!(
        queried["deployment_policy"],
        "clean disposable deploy from pinned tracked source files plus pinned upstream OpenCC"
    );
    assert_eq!(
        queried["timestamp_normalization_policy"],
        "all staged files use fixed half-second LastWriteTimeUtc verified by exact FileTimeUtc readback before deployment"
    );
    assert_eq!(
        queried["default_custom_sha256"],
        "20b8c83ef07e670c3f940214a1aa96e111c02ebf0b7e5a9614da18f8276d7d95"
    );
    assert_eq!(
        queried["opencc_tree_sha256"],
        "c5bb651dc96c8c546d1bf0c8ec60dbbae9645746378b8b3c1102450a5df143b1"
    );
}

#[test]
fn oracle_full_order_capture_is_complete_and_position_preserving() {
    let fixture = load_fixture();
    let cases = fixture["cases"]
        .as_array()
        .expect("Lane B cases should be an array");
    let expected = [
        (
            "moboyi",
            225_usize,
            "\u{83ab}",
            2_usize,
            "e8707ec2f5ed327eb976f476aee5a3a1c9f79a613d7fca6440eea3536cdac2b4",
        ),
        (
            "boyi",
            297,
            "\u{4f2f}",
            19,
            "6e568299350907049a8c4805a77d1332c9dd96a01a1c534c5c401dbff83b2144",
        ),
        (
            "yi",
            841,
            "\u{6d22}",
            155,
            "db6ed11dc979c5c8d642e226e71b694858b5c0eb86488f02e7bfb8460843aa53",
        ),
        (
            "zhonggao",
            117,
            "\u{4e2d}",
            3,
            "1571a4c4b1728f17873ce95e9dd1e3fdcb4f6870a6425ca13c51390defc5f7ba",
        ),
        (
            "zhongguo",
            125,
            "\u{4e2d}",
            11,
            "ca2b56a7b00f4be3fa43ace42d4911c67fcee2710a3611259adb7249b6d01c95",
        ),
        (
            "gao",
            164,
            "\u{9ad8}",
            0,
            "9c7473e7dd7498b99faf6b3ef76f7fb058151fd1f1b66147a3982440986bed68",
        ),
        (
            "guo",
            366,
            "\u{570b}",
            1,
            "9f37538bcd7386044b84254c836c94b9fa239adf6fd8e8954bb95b9cbf870228",
        ),
    ];
    assert_eq!(cases.len(), expected.len());

    for (case, (input, expected_count, target, target_index, expected_hash)) in
        cases.iter().zip(expected)
    {
        assert_eq!(case["input"], input);
        assert_eq!(case["rime_get_input"], input);
        assert_eq!(case["page_no"], 0);
        assert_eq!(case["captured_all_pages"], true);
        assert_eq!(case["page_size"], 5);

        let candidates = case["all_candidates"]
            .as_array()
            .unwrap_or_else(|| panic!("{input} all_candidates should be an array"));
        assert_eq!(
            candidates.len(),
            expected_count,
            "{input} order was truncated"
        );
        let pages = case["pages"]
            .as_array()
            .unwrap_or_else(|| panic!("{input} pages should be an array"));
        assert_eq!(
            pages.len(),
            expected_count.div_ceil(5),
            "{input} must have exactly enough nonempty pages for every candidate"
        );
        let mut global_index = 0_usize;
        for (page_index, page) in pages.iter().enumerate() {
            assert_eq!(page["page_no"], page_index as u64);
            assert_eq!(page["page_size"], 5);
            assert_eq!(page["is_last_page"], page_index == pages.len() - 1);
            let page_candidates = page["candidates"].as_array().unwrap_or_else(|| {
                panic!("{input} page {page_index} candidates should be an array")
            });
            let expected_page_len = (expected_count - global_index).min(5);
            assert!(
                expected_page_len > 0,
                "{input} must not contain an empty page"
            );
            assert_eq!(page_candidates.len(), expected_page_len);
            for (local_index, candidate) in page_candidates.iter().enumerate() {
                assert_eq!(candidate["index"], local_index as u64);
                assert_eq!(candidate["global_index"], global_index as u64);
                assert_eq!(candidate, &candidates[global_index]);
                global_index += 1;
            }
        }
        assert_eq!(global_index, candidates.len());
        assert_eq!(case["is_last_page"], pages[0]["is_last_page"]);
        assert_eq!(case["selected_candidates"], pages[0]["candidates"]);

        for (global_index, candidate) in candidates.iter().enumerate() {
            assert_eq!(
                candidate["global_index"].as_u64(),
                Some(global_index as u64),
                "{input} must preserve every captured position"
            );
            assert!(
                candidate["text"].is_string(),
                "{input} candidate {global_index} must retain text"
            );
        }
        assert_eq!(candidates[target_index]["text"], target);
        assert_eq!(ordered_text_sha256(candidates), expected_hash);

        let summary = &fixture["inputs"][input];
        assert_eq!(summary["target_single"], target);
        assert_eq!(summary["target_global_index"], target_index as u64);
        assert_eq!(summary["total_candidates_captured"], expected_count as u64);
        assert_eq!(summary["ordered_text_sha256"], expected_hash);
        assert_eq!(summary["captured_all_pages"], true);
    }
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
    // list at a finite global index — the historical reachability positions the
    // earlier M59 rows cite. `zhongguo` 中@11 also records an exact-order
    // divergence that remains open until the D-48 Lane B closure increment.
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
