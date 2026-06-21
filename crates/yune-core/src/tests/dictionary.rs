use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
};

use serde_json::Value;

use crate::{
    build_prism_bin, build_reverse_bin, build_table_bin, execute_rebuild_plan,
    parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_table_bin_dictionary, parse_rime_table_bin_metadata, DartsDoubleArray,
    RimeCorrectionEntry, RimeDictArtifactStatus, RimeDictRebuildExecutionReport,
    RimeDictRebuildPlan, RimeDictRebuildSources, RimePrismSpellingDescriptor, RimeToleranceRule,
    TableDictionary, TableDictionaryAdvancedData, TableEncoder, TableEntry,
};

#[test]
fn darts_double_array_exact_and_prefix_search_round_trip_inserted_keys() {
    let double_array =
        DartsDoubleArray::build(&[("a", 0), ("an", 3), ("ang", 4), ("ba", 7), ("bai", 9)])
            .expect("keys should build");

    assert_eq!(double_array.exact_match("a"), Some(0));
    assert_eq!(double_array.exact_match("ang"), Some(4));
    assert_eq!(double_array.exact_match("bai"), Some(9));
    assert_eq!(double_array.exact_match("ban"), None);

    let prefixes = double_array
        .common_prefix_search("angry")
        .into_iter()
        .map(|matched| (matched.value, matched.length))
        .collect::<Vec<_>>();
    assert_eq!(prefixes, [(0, 1), (3, 2), (4, 3)]);

    let reparsed =
        DartsDoubleArray::from_units(double_array.units().to_vec()).expect("units should parse");
    assert_eq!(reparsed.exact_match("bai"), Some(9));
}

#[test]
fn generated_prism_bin_round_trips_spelling_map_and_double_array() {
    let syllabary = vec![
        "a".to_owned(),
        "ai".to_owned(),
        "an".to_owned(),
        "eng".to_owned(),
    ];
    let algebra = vec![
        "derive/^ai$/a/abbrev".to_owned(),
        "derive/^an$/a/fuzz".to_owned(),
        "derive/^eng$/en/correction".to_owned(),
    ];

    let bytes = build_prism_bin(&syllabary, &algebra, 0x1111_1111, 0x2222_2222);
    let payload = parse_rime_prism_bin_payload(&bytes).expect("generated prism should parse");

    assert_eq!(payload.dict_file_checksum, 0x1111_1111);
    assert_eq!(payload.schema_file_checksum, 0x2222_2222);
    assert_eq!(payload.num_syllables, 4);

    let double_array = payload
        .double_array
        .as_ref()
        .expect("generated prism should include a double-array");
    let a_index = double_array
        .exact_match("a")
        .expect("a spelling should be indexed") as usize;
    assert_eq!(
        payload.spelling_map[a_index],
        vec![
            RimePrismSpellingDescriptor {
                syllable_id: 0,
                spelling_type: 0,
                is_correction: false,
                credibility: 0.0,
                tips: String::new(),
            },
            RimePrismSpellingDescriptor {
                syllable_id: 1,
                spelling_type: 2,
                is_correction: false,
                credibility: -std::f32::consts::LN_2,
                tips: String::new(),
            },
            RimePrismSpellingDescriptor {
                syllable_id: 2,
                spelling_type: 1,
                is_correction: false,
                credibility: -std::f32::consts::LN_2,
                tips: String::new(),
            },
        ]
    );

    let en_index = double_array
        .exact_match("en")
        .expect("correction spelling should be indexed") as usize;
    assert_eq!(
        payload.spelling_map[en_index],
        vec![RimePrismSpellingDescriptor {
            syllable_id: 3,
            spelling_type: 0,
            is_correction: true,
            credibility: -std::f32::consts::LN_10 * 2.0,
            tips: String::new(),
        }]
    );
}

#[test]
fn table_and_reverse_writers_round_trip_through_existing_readers() {
    let dictionary = sample_dictionary();

    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let metadata = parse_rime_table_bin_metadata(&table_bytes).expect("metadata should parse");
    assert_eq!(metadata.dict_file_checksum, 0x1234_5678);
    let table_round_trip =
        parse_rime_table_bin_dictionary(&table_bytes).expect("table should round-trip");
    assert_eq!(table_round_trip.entries(), dictionary.entries());
    assert_eq!(
        table_round_trip.stems_for("明"),
        Some(&["m'ing".to_owned()][..])
    );
    assert_eq!(
        table_round_trip.corrections(),
        [RimeCorrectionEntry::new("uen", "un")]
    );
    assert_eq!(
        table_round_trip.tolerance_rules(),
        [RimeToleranceRule::new("en", ["eng"])]
    );
    assert!(table_round_trip.encoder().loaded());

    let reverse_bytes = build_reverse_bin(&dictionary, 0x1234_5678);
    let reverse_round_trip =
        parse_rime_reverse_bin_dictionary(&reverse_bytes).expect("reverse should round-trip");
    assert_eq!(
        code_text_pairs(reverse_round_trip.entries()),
        code_text_pairs(dictionary.entries())
    );
    assert_eq!(
        reverse_round_trip.dict_settings().get("tail_anchor"),
        Some(&"'".to_owned())
    );
    assert_eq!(
        reverse_round_trip.stems_for("明"),
        Some(&["m'ing".to_owned()][..])
    );
}

#[test]
fn rebuild_plan_executor_writes_only_requested_artifacts() {
    let root = std::env::temp_dir().join(format!("yune-m18-rebuild-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("old temp dir should be removable");
    }
    fs::create_dir_all(&root).expect("temp dir should be created");

    let dictionary = sample_dictionary();
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let plan = RimeDictRebuildPlan {
        dict_file_checksum: 0x4444_4444,
        rebuild_table: true,
        rebuild_prism: false,
        rebuild_reverse: true,
        report: RimeDictRebuildExecutionReport {
            table: RimeDictArtifactStatus::Rebuilt,
            prism: RimeDictArtifactStatus::ReusedFresh,
            reverse: RimeDictArtifactStatus::Rebuilt,
        },
    };
    let sources = RimeDictRebuildSources {
        artifact_stem: "sample",
        table_dictionary: &dictionary,
        reverse_dictionary: &dictionary,
        syllabary: &syllabary,
        algebra_formulas: &[],
        schema_file_checksum: 0x5555_5555,
    };

    let report = execute_rebuild_plan(&plan, &sources, &root).expect("plan should execute");
    assert_eq!(report, plan.report);
    assert!(root.join("sample.table.bin").is_file());
    assert!(!root.join("sample.prism.bin").exists());
    assert!(root.join("sample.reverse.bin").is_file());

    fs::remove_dir_all(&root).expect("temp dir should be removed");
}

#[test]
fn upstream_luna_pinyin_prism_fixture_parses_real_darts_double_array() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("upstream-1.17.0");
    let manifest_path = root.join("m18-luna-pinyin-prism.json");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("failed to read {manifest_path:?}: {error}"));
    let manifest = serde_json::from_str::<Value>(&manifest)
        .unwrap_or_else(|error| panic!("invalid JSON {manifest_path:?}: {error}"));
    let binary_file = manifest["capture"]["binary_file"]
        .as_str()
        .expect("manifest should name binary file");
    let bytes = fs::read(root.join(binary_file)).expect("upstream prism binary should be readable");

    let payload = parse_rime_prism_bin_payload(&bytes).expect("upstream prism should parse");
    let metadata = &manifest["capture"]["expected_metadata"];
    assert_eq!(
        payload.dict_file_checksum,
        metadata["dict_file_checksum"].as_u64().unwrap() as u32
    );
    assert_eq!(
        payload.schema_file_checksum,
        metadata["schema_file_checksum"].as_u64().unwrap() as u32
    );
    assert_eq!(
        payload.num_syllables,
        metadata["num_syllables"].as_u64().unwrap() as u32
    );
    assert_eq!(
        payload.num_spellings,
        metadata["num_spellings"].as_u64().unwrap() as u32
    );
    assert_eq!(
        payload.double_array_size,
        metadata["double_array_size"].as_u64().unwrap() as u32
    );

    let double_array = payload
        .double_array
        .as_ref()
        .expect("upstream prism should include Darts units");
    for expected in manifest["capture"]["exact_matches"]
        .as_array()
        .expect("exact matches should be listed")
    {
        let spelling = expected["spelling"]
            .as_str()
            .expect("spelling should be a string");
        let spelling_index = expected["spelling_index"]
            .as_u64()
            .expect("spelling index should be numeric") as u32;
        assert_eq!(double_array.exact_match(spelling), Some(spelling_index));
        assert!(
            !payload.spelling_map[spelling_index as usize].is_empty(),
            "{spelling} should have parsed spelling descriptors"
        );
    }
}

fn code_text_pairs(entries: &[TableEntry]) -> Vec<(&str, &str)> {
    entries
        .iter()
        .map(|entry| (entry.code.as_str(), entry.text.as_str()))
        .collect()
}

fn sample_dictionary() -> TableDictionary {
    let mut stems = HashMap::new();
    stems.insert("明".to_owned(), vec!["m'ing".to_owned()]);
    let mut dict_settings = BTreeMap::new();
    dict_settings.insert("tail_anchor".to_owned(), "'".to_owned());
    let mut encoder = TableEncoder::new();
    encoder
        .add_length_equal_rule(2, "AaBa")
        .expect("encoder rule should parse");

    TableDictionary::with_advanced_data(
        [
            TableEntry::new("ming", "明", 3.0),
            TableEntry::new("an", "安", 2.0),
        ],
        TableDictionaryAdvancedData {
            stems,
            dict_settings,
            encoder,
            corrections: vec![RimeCorrectionEntry::new("uen", "un")],
            tolerance_rules: vec![RimeToleranceRule::new("en", ["eng"])],
        },
    )
}
