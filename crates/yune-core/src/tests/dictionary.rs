use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
};

use serde_json::Value;

use crate::dictionary::{parse_compact_table_bin_lookup, TableLookup};
use crate::{
    build_prism_bin, build_reverse_bin, build_table_bin, execute_rebuild_plan,
    parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_table_bin_dictionary, parse_rime_table_bin_metadata, Candidate, CandidateSource,
    DartsDoubleArray, RimeCorrectionEntry, RimeDictArtifactStatus, RimeDictRebuildExecutionReport,
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
fn heap_table_lookup_exposes_exact_prefix_and_all_code_queries() {
    let mut lookup = BTreeMap::<String, Vec<Candidate>>::new();
    for (code, text, quality) in [
        ("ni", "exact", 2.0),
        ("nia", "completion-a", 4.0),
        ("nib", "completion-b", 3.0),
        ("hao", "other", 1.0),
    ] {
        lookup.entry(code.to_owned()).or_default().push(Candidate {
            text: text.to_owned(),
            comment: code.to_owned(),
            preedit: None,
            source: CandidateSource::Table,
            quality,
        });
    }

    assert!(lookup.has_code("ni"));
    assert!(!lookup.has_code("n"));
    assert_eq!(
        lookup
            .exact_candidates("ni")
            .map(|candidate| candidate.text().to_owned())
            .collect::<Vec<_>>(),
        ["exact"]
    );
    assert_eq!(
        lookup
            .prefix_candidates("ni")
            .map(|entry| {
                let (code, candidate) = entry.into_parts();
                (code.to_owned(), candidate.text().to_owned())
            })
            .collect::<Vec<_>>(),
        [
            ("ni".to_owned(), "exact".to_owned()),
            ("nia".to_owned(), "completion-a".to_owned()),
            ("nib".to_owned(), "completion-b".to_owned())
        ]
    );
    assert_eq!(
        lookup.all_codes().collect::<Vec<_>>(),
        ["hao", "ni", "nia", "nib"]
    );
}

#[test]
fn compact_table_lookup_matches_heap_exact_prefix_and_all_code_queries() {
    let dictionary = TableDictionary::with_advanced_data(
        [
            TableEntry::new("ni", "exact", 2.0),
            TableEntry::new("nia", "completion-a", 4.0),
            TableEntry::new("nib", "completion-b", 3.0),
            TableEntry::new("hao", "other", 1.0),
        ],
        TableDictionaryAdvancedData {
            corrections: vec![RimeCorrectionEntry::new("uen", "un")],
            tolerance_rules: vec![RimeToleranceRule::new("en", ["eng"])],
            ..TableDictionaryAdvancedData::default()
        },
    );
    let heap = table_dictionary_heap_lookup(&dictionary);
    let bytes = build_table_bin(&dictionary, 0x1234_5678);
    let compact = parse_compact_table_bin_lookup(&bytes).expect("compact table should parse");

    assert!(compact.has_code("ni"));
    assert!(!compact.has_code("n"));
    assert_eq!(
        compact
            .exact_candidates("ni")
            .map(|candidate| (
                candidate.text().to_owned(),
                candidate.raw_comment().to_owned(),
                candidate.raw_quality()
            ))
            .collect::<Vec<_>>(),
        heap.exact_candidates("ni")
            .map(|candidate| (
                candidate.text().to_owned(),
                candidate.raw_comment().to_owned(),
                candidate.raw_quality()
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        compact
            .prefix_candidates("ni")
            .map(|entry| {
                let (code, candidate) = entry.into_parts();
                (code.to_owned(), candidate.text().to_owned())
            })
            .collect::<Vec<_>>(),
        heap.prefix_candidates("ni")
            .map(|entry| {
                let (code, candidate) = entry.into_parts();
                (code.to_owned(), candidate.text().to_owned())
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        compact.all_codes().collect::<Vec<_>>(),
        heap.all_codes().collect::<Vec<_>>()
    );
    assert_eq!(
        compact.corrections(),
        [RimeCorrectionEntry::new("uen", "un")]
    );
    assert_eq!(
        compact.tolerance_rules(),
        [RimeToleranceRule::new("en", ["eng"])]
    );
}

#[test]
fn prism_lookup_resolves_spelling_to_table_payload_codes() {
    let dictionary = TableDictionary::new([
        TableEntry::new("ai", "payload-ai", 2.0),
        TableEntry::new("an", "payload-an", 3.0),
    ]);
    let compact = parse_compact_table_bin_lookup(build_table_bin(&dictionary, 0x1234_5678))
        .expect("compact table should parse");
    let prism = parse_rime_prism_bin_payload(build_prism_bin(
        compact.syllabary_codes(),
        &[
            "derive/^ai$/a/".to_owned(),
            "derive/^an$/a/abbrev".to_owned(),
        ],
        0x1234_5678,
        0x8765_4321,
    ))
    .expect("prism should parse");

    let resolved = prism
        .lookup_canonical_codes("a", compact.syllabary_codes())
        .into_iter()
        .map(|code| (code.code.to_owned(), code.abbreviation))
        .collect::<Vec<_>>();
    assert_eq!(
        resolved,
        [("ai".to_owned(), false), ("an".to_owned(), true)]
    );

    let texts = resolved
        .iter()
        .flat_map(|(code, _)| compact.exact_candidates(code))
        .map(|candidate| candidate.text().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["payload-ai", "payload-an"]);
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
fn table_writer_preserves_typeduck_lookup_records_for_compiled_path() {
    let dictionary = TableDictionary::parse_typeduck_lookup_dict_yaml(
        "---\n\
name: lookup\n\
version: '0.1'\n\
sort: original\n\
...\n\
\n\
ngo5hai6,1,0,,oth,ver,,,我是,,,I am,,,,\t我係\n",
    )
    .expect("typeduck lookup dictionary should parse");

    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let reparsed = parse_rime_table_bin_dictionary(&table_bytes).expect("table should round-trip");
    let records = reparsed
        .lookup_records_for("我係")
        .expect("compiled dictionary should preserve TypeDuck lookup rows");

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].code, "ngo5hai6");
    assert_eq!(
        records[0].fields,
        ["我係", "ngo5hai6,1,0,,oth,ver,,,我是,,,I am,,,,"]
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
        prism_artifact_stem: "sample",
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
fn rebuild_plan_executor_writes_prism_to_configured_stem() {
    let root = std::env::temp_dir().join(format!("yune-m36-prism-stem-{}", std::process::id()));
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
        rebuild_table: false,
        rebuild_prism: true,
        rebuild_reverse: false,
        report: RimeDictRebuildExecutionReport {
            table: RimeDictArtifactStatus::ReusedFresh,
            prism: RimeDictArtifactStatus::Rebuilt,
            reverse: RimeDictArtifactStatus::ReusedFresh,
        },
    };
    let sources = RimeDictRebuildSources {
        artifact_stem: "sample",
        prism_artifact_stem: "sample_mobile",
        table_dictionary: &dictionary,
        reverse_dictionary: &dictionary,
        syllabary: &syllabary,
        algebra_formulas: &[],
        schema_file_checksum: 0x5555_5555,
    };

    execute_rebuild_plan(&plan, &sources, &root).expect("plan should execute");
    assert!(!root.join("sample.prism.bin").exists());
    assert!(root.join("sample_mobile.prism.bin").is_file());

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

#[test]
fn upstream_luna_pinyin_prism_fixture_does_not_contain_candidate_payloads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("upstream-1.17.0");
    let bytes = fs::read(root.join("m18-luna-pinyin-prism.bin"))
        .expect("upstream prism binary should be readable");

    let payload = parse_rime_prism_bin_payload(&bytes).expect("upstream prism should parse");
    let double_array = payload
        .double_array
        .as_ref()
        .expect("upstream prism should include Darts units");
    let spelling_index = double_array
        .exact_match("ni")
        .expect("ni spelling should be indexed") as usize;
    assert!(
        !payload.spelling_map[spelling_index].is_empty(),
        "the prism maps spellings to syllable descriptors"
    );

    // A lazy prism walk still needs the table payload for candidate text/comment/order.
    for candidate_text in ["\u{4f60}", "\u{597d}", "\u{4e2d}\u{56fd}"] {
        assert!(
            !bytes
                .windows(candidate_text.len())
                .any(|window| window == candidate_text.as_bytes()),
            "candidate text {candidate_text:?} should not be stored in the prism"
        );
    }
}

fn code_text_pairs(entries: &[TableEntry]) -> Vec<(&str, &str)> {
    entries
        .iter()
        .map(|entry| (entry.code.as_str(), entry.text.as_str()))
        .collect()
}

fn table_dictionary_heap_lookup(dictionary: &TableDictionary) -> BTreeMap<String, Vec<Candidate>> {
    let mut lookup = BTreeMap::<String, Vec<Candidate>>::new();
    for entry in dictionary.entries() {
        lookup
            .entry(entry.code.clone())
            .or_default()
            .push(Candidate {
                text: entry.text.clone(),
                comment: entry.code.clone(),
                preedit: None,
                source: CandidateSource::Table,
                quality: entry.weight,
            });
    }
    lookup
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
            ..TableDictionaryAdvancedData::default()
        },
    )
}
