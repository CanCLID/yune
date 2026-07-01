use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::Path,
    sync::Arc,
};

use serde_json::Value;

use crate::dictionary::{parse_compact_table_bin_lookup, TableLookup};
use crate::{
    build_prism_bin, build_reverse_bin, build_table_bin,
    byte_backed_lookup_records_from_table_bin_bytes, execute_rebuild_plan,
    parse_rime_prism_bin_payload, parse_rime_prism_runtime_payload,
    parse_rime_reverse_bin_dictionary, parse_rime_table_bin_advanced_data,
    parse_rime_table_bin_advanced_data_with_options, parse_rime_table_bin_dictionary,
    parse_rime_table_bin_metadata, Candidate, CandidateSource, CompactTableByteSource,
    CompactTableStore, DartsDoubleArray, DictionaryLookupRecord, MemoryOwnerClass,
    RimeCorrectionEntry, RimeDictArtifactStatus, RimeDictRebuildExecutionReport,
    RimeDictRebuildPlan, RimeDictRebuildSources, RimePrismSpellingDescriptor,
    RimeTableBinAdvancedDataOptions, RimeTableBinParseError, RimeToleranceRule, TableDictionary,
    TableDictionaryAdvancedData, TableEncoder, TableEntry,
};

#[derive(Debug)]
struct TestPrismByteSource {
    bytes: Arc<[u8]>,
    mapping_mode: &'static str,
}

impl CompactTableByteSource for TestPrismByteSource {
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn storage_label(&self) -> &'static str {
        "byte_backed"
    }

    fn mapping_mode(&self) -> &'static str {
        self.mapping_mode
    }
}

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
fn darts_double_array_supports_non_utf8_binary_keys() {
    let double_array =
        DartsDoubleArray::build_bytes(&[(vec![0x8e, 0x2d], 7), (vec![0x8e, 0x2d, 0xe1, 0x80], 11)])
            .expect("binary keys should build");

    assert_eq!(double_array.exact_match_bytes(&[0x8e, 0x2d]), Some(7));
    assert_eq!(
        double_array.exact_match_bytes(&[0x8e, 0x2d, 0xe1, 0x80]),
        Some(11)
    );
    assert_eq!(double_array.exact_match_bytes(&[0x8e, 0x2e]), None);

    let prefixes = double_array
        .common_prefix_search_bytes(&[0x8e, 0x2d, 0xe1, 0x80, b'x'])
        .into_iter()
        .map(|matched| (matched.value, matched.length))
        .collect::<Vec<_>>();
    assert_eq!(prefixes, [(7, 2), (11, 4)]);
}

#[test]
fn build_prism_bin_round_trips_multi_syllable_toneless_spellings() {
    // Regression for a double-array construction bug: a parent node validated its
    // child slots as free but did not reserve them before recursing, so one child's
    // subtree could occupy a not-yet-assigned sibling's slot and corrupt the trie.
    // `exact_match` then returned out-of-range values for some keys. With the rich
    // Jyutping tone algebra this dropped the toneless form of multi-syllable codes
    // (e.g. "litbiu" from "lit6biu2"), so byte-backed Jyutping returned garbage
    // candidates in the browser. The expected codes are just the input syllabary
    // codes — this asserts the spelling algebra round-trips, not any oracle ranking.
    let syllabary: Vec<String> = [
        "lit6biu2",
        "siu2baan1doek3muk6niu5",
        "lit6",
        "biu2",
        "caam1haau2",
        "caam1",
        "haau2",
    ]
    .iter()
    .map(|code| (*code).to_owned())
    .collect();
    let algebra: Vec<String> = [
        "derive/^ng(?=[aeiou])//",
        "derive/^(?=[aeiou])/ng/",
        "derive/^n(?!g)/l/",
        "derive/^ng(?=\\d)/m/",
        "derive/^(g|k)w(?=o)/$1/",
        "derive/^jy?(?=[aeiou])/y/",
        "derive/^jyu/ju/",
        "derive/yu(?!ng|k)/y/",
        "derive/(g|k)u(?!ng|k)/$1wu/",
        "derive/^([zcs])/$1h/",
        "derive/eoi(?=\\d)/eoy/",
        "derive/eo/oe/",
        "derive/oe/eo/",
        "derive/aa(?=\\d)/a/",
        "derive/\\d//",
        "abbrev/^([a-z]).+$/$1/",
        "xform/1/v/",
        "xform/4/vv/",
        "xform/2/x/",
        "xform/5/xx/",
        "xform/3/q/",
        "xform/6/qq/",
    ]
    .iter()
    .map(|formula| (*formula).to_owned())
    .collect();

    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &algebra, 1, 2))
        .expect("generated prism should parse");

    for (spelling, expected_code) in [("litbiu", "lit6biu2"), ("caamhaau", "caam1haau2")] {
        let codes = prism.lookup_canonical_codes(spelling, &syllabary);
        assert!(
            codes.iter().any(|code| code.code == expected_code),
            "toneless spelling {spelling} should map to {expected_code}, got {codes:?}"
        );
    }

    // No key may resolve to an index beyond the spelling map (the collision bug
    // produced values far larger than the spelling count).
    if let Some(double_array) = prism.double_array.as_ref() {
        for spelling in ["litbiu", "lit", "biu", "caamhaau", "caam", "haau"] {
            if let Some(index) = double_array.exact_match(spelling) {
                assert!(
                    (index as usize) < prism.spelling_map.len(),
                    "exact_match({spelling}) returned out-of-range index {index} (spelling map len {})",
                    prism.spelling_map.len()
                );
            }
        }
    }
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
                (code.into_owned(), candidate.text().to_owned())
            })
            .collect::<Vec<_>>(),
        [
            ("ni".to_owned(), "exact".to_owned()),
            ("nia".to_owned(), "completion-a".to_owned()),
            ("nib".to_owned(), "completion-b".to_owned())
        ]
    );
    assert_eq!(
        lookup
            .all_codes()
            .map(|code| code.into_owned())
            .collect::<Vec<_>>(),
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
                (code.into_owned(), candidate.text().to_owned())
            })
            .collect::<Vec<_>>(),
        heap.prefix_candidates("ni")
            .map(|entry| {
                let (code, candidate) = entry.into_parts();
                (code.into_owned(), candidate.text().to_owned())
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
fn compact_table_lookup_resolves_marisa_backed_upstream_table_entries() {
    let bytes = build_marisa_table_fixture();
    let compact =
        parse_compact_table_bin_lookup(&bytes).expect("marisa-backed upstream table should parse");

    assert_eq!(compact.storage_label(), "rsmarisa_byte_backed");
    assert!(compact.has_code("a"));
    assert!(compact.has_code("xian"));
    assert!(compact.has_code("zhongguo"));
    assert!(compact.has_code("zhongguorenmin"));
    assert!(!compact.has_code("zhongguoa"));
    assert!(!compact.has_code("zhongguorenmina"));

    let single = compact
        .exact_candidates("a")
        .map(|candidate| {
            (
                candidate.text().to_owned(),
                candidate.raw_comment().to_owned(),
                candidate.raw_quality(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        single,
        [
            ("\u{554a}".to_owned(), "a".to_owned(), 11.0),
            ("\u{963f}".to_owned(), "a".to_owned(), 9.0)
        ]
    );

    let phrase = compact
        .exact_candidates("zhongguo")
        .map(|candidate| {
            (
                candidate.text().to_owned(),
                candidate.raw_comment().to_owned(),
                candidate.raw_quality(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        phrase,
        [("\u{4e2d}\u{56fd}".to_owned(), "zhongguo".to_owned(), 13.0)]
    );

    let long_phrase = compact
        .exact_candidates("zhongguorenmin")
        .map(|candidate| {
            (
                candidate.text().to_owned(),
                candidate.raw_comment().to_owned(),
                candidate.raw_quality(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        long_phrase,
        [(
            "\u{4e2d}\u{534e}\u{4eba}\u{6c11}\u{5171}\u{548c}\u{56fd}".to_owned(),
            "zhongguorenmin".to_owned(),
            21.0
        )]
    );

    let ambiguous = compact
        .exact_candidates("xian")
        .map(|candidate| {
            (
                candidate.text().to_owned(),
                candidate.raw_comment().to_owned(),
                candidate.raw_quality(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ambiguous,
        [
            ("\u{5148}".to_owned(), "xian".to_owned(), 15.0),
            ("\u{897f}\u{5b89}".to_owned(), "xian".to_owned(), 14.0)
        ]
    );

    let prefix = compact
        .prefix_candidates("zhongg")
        .map(|entry| {
            let (code, candidate) = entry.into_parts();
            (
                code.into_owned(),
                candidate.text().to_owned(),
                candidate.raw_comment().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        prefix,
        [
            (
                "zhongguo".to_owned(),
                "\u{4e2d}\u{56fd}".to_owned(),
                "zhongguo".to_owned()
            ),
            (
                "zhongguoren".to_owned(),
                "\u{4e2d}\u{56fd}\u{4eba}".to_owned(),
                "zhongguoren".to_owned()
            ),
            (
                "zhongguorenmin".to_owned(),
                "\u{4e2d}\u{534e}\u{4eba}\u{6c11}\u{5171}\u{548c}\u{56fd}".to_owned(),
                "zhongguorenmin".to_owned()
            )
        ]
    );

    let all_codes_list = compact
        .all_codes()
        .map(|code| code.into_owned())
        .collect::<Vec<_>>();
    assert!(all_codes_list.iter().any(|code| code == "xian"));
    let all_codes = all_codes_list.into_iter().collect::<HashSet<_>>();
    assert!(all_codes.contains("a"));
    assert!(all_codes.contains("xian"));
    assert!(all_codes.contains("zhongguo"));
    assert!(all_codes.contains("zhongguorenmin"));
    assert!(!all_codes.contains("zhongguorenmi"));
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
fn prism_runtime_payload_reads_lookup_storage_from_byte_source() {
    let dictionary = TableDictionary::new([
        TableEntry::new("ai", "payload-ai", 2.0),
        TableEntry::new("an", "payload-an", 3.0),
    ]);
    let compact = parse_compact_table_bin_lookup(build_table_bin(&dictionary, 0x1234_5678))
        .expect("compact table should parse");
    let bytes = Arc::<[u8]>::from(build_prism_bin(
        compact.syllabary_codes(),
        &[
            "derive/^ai$/a/".to_owned(),
            "derive/^an$/a/abbrev".to_owned(),
        ],
        0x1234_5678,
        0x8765_4321,
    ));
    let source: Arc<dyn CompactTableByteSource> = Arc::new(TestPrismByteSource {
        bytes,
        mapping_mode: "mmap",
    });
    let prism = parse_rime_prism_runtime_payload(source).expect("byte-backed prism should parse");

    let resolved = prism
        .lookup_canonical_codes("a", compact.syllabary_codes())
        .into_iter()
        .map(|code| (code.code.to_owned(), code.abbreviation))
        .collect::<Vec<_>>();
    assert_eq!(
        resolved,
        [("ai".to_owned(), false), ("an".to_owned(), true)]
    );

    let rows = prism.memory_owner_rows();
    let owner = |name: &str| {
        rows.iter()
            .find(|row| row.owner == name)
            .unwrap_or_else(|| panic!("owner row {name} should be present"))
    };
    assert_eq!(
        owner("prism.double_array_units").class,
        MemoryOwnerClass::MmapFileBacked
    );
    assert_eq!(
        owner("prism.spelling_map").class,
        MemoryOwnerClass::MmapFileBacked
    );
    assert!(owner("prism.double_array_units").estimated_bytes > 0);
    assert!(owner("prism.spelling_map").estimated_bytes > 0);
    assert_eq!(
        owner("prism.corrections_tolerance").class,
        MemoryOwnerClass::HeapOwnedRequired
    );
    assert!(owner("prism.corrections_tolerance").estimated_bytes < 128);
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
fn table_advanced_parser_can_skip_lookup_records_without_dropping_keyboard_payloads() {
    let mut stems = HashMap::new();
    stems.insert("word".to_owned(), vec!["nei".to_owned()]);
    let mut lookup_records = HashMap::new();
    lookup_records.insert(
        "word".to_owned(),
        vec![DictionaryLookupRecord {
            code: "nei".to_owned(),
            fields: vec!["word".to_owned(), "nei,1,primary".to_owned()],
        }],
    );
    let dictionary = TableDictionary::with_advanced_data(
        [TableEntry::new("nei", "word", 1.0)],
        TableDictionaryAdvancedData {
            stems,
            corrections: vec![RimeCorrectionEntry::new("leoi", "neoi")],
            tolerance_rules: vec![RimeToleranceRule::new("nei", ["lei"])],
            lookup_records,
            ..TableDictionaryAdvancedData::default()
        },
    );

    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let default_data =
        parse_rime_table_bin_advanced_data(&table_bytes).expect("advanced data should parse");
    assert_eq!(default_data.lookup_records["word"][0].code, "nei");

    let skipped = parse_rime_table_bin_advanced_data_with_options(
        &table_bytes,
        RimeTableBinAdvancedDataOptions {
            load_lookup_records: false,
            ..RimeTableBinAdvancedDataOptions::default()
        },
    )
    .expect("advanced data should parse without retaining lookup records");

    assert!(skipped.lookup_records.is_empty());
    assert_eq!(skipped.stems["word"], ["nei"]);
    assert_eq!(
        skipped.corrections,
        [RimeCorrectionEntry::new("leoi", "neoi")]
    );
    assert_eq!(
        skipped.tolerance_rules,
        [RimeToleranceRule::new("nei", ["lei"])]
    );
}

#[test]
fn byte_backed_lookup_records_decode_from_compiled_payload_without_heap_owner() {
    let dictionary = TableDictionary::parse_typeduck_lookup_dict_yaml(
        "---\n\
name: lookup\n\
version: '0.1'\n\
sort: original\n\
...\n\
\n\
ngo5hai6,1,0,,oth,ver,,,word,,,I am,,,,\tword\n\
nei5,1,0,,oth,,,,,,,you (singular),tm,nepali,hindi,kamu\tword\n",
    )
    .expect("typeduck lookup dictionary should parse");
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let byte_backed = byte_backed_lookup_records_from_table_bin_bytes(table_bytes.clone())
        .expect("compiled lookup payload should parse")
        .expect("compiled lookup payload should be present");

    assert_eq!(byte_backed.text_count(), 1);
    assert_eq!(byte_backed.record_count(), 2);
    let records = byte_backed
        .records_for_text("word")
        .expect("records should decode on demand");
    assert_eq!(records[0].code, "ngo5hai6");
    assert_eq!(
        records[0].fields,
        ["word", "ngo5hai6,1,0,,oth,ver,,,word,,,I am,,,,"]
    );
    assert_eq!(records[1].code, "nei5");
    assert!(byte_backed.records_for_text("missing").is_none());

    let advanced = parse_rime_table_bin_advanced_data_with_options(
        &table_bytes,
        RimeTableBinAdvancedDataOptions {
            load_lookup_records: true,
            byte_back_lookup_records: true,
        },
    )
    .expect("advanced data should parse without retaining lookup records");
    assert!(advanced.lookup_records.is_empty());

    let store = CompactTableStore::from_table_bin_bytes(
        table_bytes,
        advanced.with_byte_backed_lookup_records(byte_backed),
    )
    .expect("compact store should parse");
    let owner = store
        .memory_owner_rows()
        .into_iter()
        .find(|row| row.owner == "compact_table.lookup_records")
        .expect("compact lookup owner row should be present");
    assert_ne!(owner.class, MemoryOwnerClass::HeapOwnedRequired);
    assert_eq!(owner.item_count, 2);
    assert!(owner.storage.contains("byte_backed"));
}

#[test]
fn table_advanced_lookup_record_skip_still_rejects_invalid_utf8() {
    let dictionary = TableDictionary::parse_typeduck_lookup_dict_yaml(
        "---\n\
name: lookup\n\
version: '0.1'\n\
sort: original\n\
...\n\
\n\
ngo5hai6,1,0,,oth,ver,,,word,,,I am,,,,\tword\n",
    )
    .expect("typeduck lookup dictionary should parse");
    let mut table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    corrupt_first_lookup_record_code_byte(&mut table_bytes);

    let error = parse_rime_table_bin_advanced_data_with_options(
        &table_bytes,
        RimeTableBinAdvancedDataOptions {
            load_lookup_records: false,
            ..RimeTableBinAdvancedDataOptions::default()
        },
    )
    .expect_err("skip mode should validate lookup record UTF-8");

    assert_eq!(error, RimeTableBinParseError::InvalidUtf8);
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

fn corrupt_first_lookup_record_code_byte(bytes: &mut [u8]) {
    let marker = b"YUNE-LOOKUP\0";
    let marker_offset = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("fixture should contain lookup payload");
    let mut cursor = marker_offset + marker.len();
    let text_count = read_test_u32(bytes, cursor);
    assert_eq!(text_count, 1, "fixture should have one lookup text");
    cursor += 4;
    cursor = skip_test_len_string(bytes, cursor);
    let record_count = read_test_u32(bytes, cursor);
    assert_eq!(record_count, 1, "fixture should have one lookup record");
    cursor += 4;
    let code_len = read_test_u32(bytes, cursor);
    assert!(code_len > 0, "fixture code should not be empty");
    bytes[cursor + 4] = 0xff;
}

fn skip_test_len_string(bytes: &[u8], offset: usize) -> usize {
    offset + 4 + read_test_u32(bytes, offset)
}

fn read_test_u32(bytes: &[u8], offset: usize) -> usize {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("fixture should contain a u32"),
    ) as usize
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

fn build_marisa_table_fixture() -> Vec<u8> {
    let keys = [
        "a",
        "an",
        "guo",
        "min",
        "ren",
        "xi",
        "xian",
        "zhong",
        "\u{554a}",
        "\u{963f}",
        "\u{56fd}",
        "\u{4e2d}",
        "\u{4e2d}\u{56fd}",
        "\u{4e2d}\u{56fd}\u{4eba}",
        "\u{4e2d}\u{534e}\u{4eba}\u{6c11}\u{5171}\u{548c}\u{56fd}",
        "\u{5148}",
        "\u{897f}\u{5b89}",
    ];
    let mut keyset = rsmarisa::Keyset::new();
    for key in keys {
        keyset
            .push_back_str(key)
            .expect("fixture key should be accepted");
    }
    let mut trie = rsmarisa::Trie::new();
    trie.build(&mut keyset, 0);

    let mut bytes = vec![0; 68];
    put_c_string(&mut bytes, 0, b"Rime::Table/4.0");
    put_u32_le(&mut bytes, 32, 0x1234_5678);
    put_u32_le(&mut bytes, 36, 8);
    put_u32_le(&mut bytes, 40, 9);

    let syllabary_offset = bytes.len();
    bytes.resize(syllabary_offset + 4 + 8 * 4, 0);
    put_u32_le(&mut bytes, syllabary_offset, 8);
    for (index, key) in ["a", "an", "guo", "min", "ren", "xi", "xian", "zhong"]
        .into_iter()
        .enumerate()
    {
        put_u32_le(
            &mut bytes,
            syllabary_offset + 4 + index * 4,
            marisa_id(&trie, key),
        );
    }

    let a_entries = append_marisa_entry_list(
        &mut bytes,
        &[
            (marisa_id(&trie, "\u{554a}"), 11.0),
            (marisa_id(&trie, "\u{963f}"), 9.0),
        ],
    );
    let guo_entries = append_marisa_entry_list(&mut bytes, &[(marisa_id(&trie, "\u{56fd}"), 8.0)]);
    let zhong_entries =
        append_marisa_entry_list(&mut bytes, &[(marisa_id(&trie, "\u{4e2d}"), 7.0)]);
    let xian_entries =
        append_marisa_entry_list(&mut bytes, &[(marisa_id(&trie, "\u{5148}"), 15.0)]);
    let xi_an_entries =
        append_marisa_entry_list(&mut bytes, &[(marisa_id(&trie, "\u{897f}\u{5b89}"), 14.0)]);
    let zhongguo_entries =
        append_marisa_entry_list(&mut bytes, &[(marisa_id(&trie, "\u{4e2d}\u{56fd}"), 13.0)]);
    let zhongguoren_entries = append_marisa_entry_list(
        &mut bytes,
        &[(marisa_id(&trie, "\u{4e2d}\u{56fd}\u{4eba}"), 17.0)],
    );
    let zhongguorenmin_tail = append_marisa_tail_index(
        &mut bytes,
        &[(
            &[3],
            marisa_id(
                &trie,
                "\u{4e2d}\u{534e}\u{4eba}\u{6c11}\u{5171}\u{548c}\u{56fd}",
            ),
            21.0,
        )],
    );

    let second_trunk_offset = bytes.len();
    bytes.resize(second_trunk_offset + 4 + 16, 0);
    put_u32_le(&mut bytes, second_trunk_offset, 1);
    put_marisa_trunk_node(
        &mut bytes,
        second_trunk_offset + 4,
        4,
        1,
        zhongguoren_entries,
        Some(zhongguorenmin_tail),
    );

    let trunk_offset = bytes.len();
    bytes.resize(trunk_offset + 4 + 16, 0);
    put_u32_le(&mut bytes, trunk_offset, 1);
    put_marisa_trunk_node(
        &mut bytes,
        trunk_offset + 4,
        2,
        1,
        zhongguo_entries,
        Some(second_trunk_offset),
    );

    let xi_trunk_offset = bytes.len();
    bytes.resize(xi_trunk_offset + 4 + 16, 0);
    put_u32_le(&mut bytes, xi_trunk_offset, 1);
    put_marisa_trunk_node(&mut bytes, xi_trunk_offset + 4, 1, 1, xi_an_entries, None);

    let index_offset = bytes.len();
    bytes.resize(index_offset + 4 + 8 * 12, 0);
    put_u32_le(&mut bytes, index_offset, 8);
    put_marisa_head_node(&mut bytes, index_offset + 4, 2, a_entries, None);
    put_marisa_head_node(&mut bytes, index_offset + 16, 0, 0, None);
    put_marisa_head_node(&mut bytes, index_offset + 28, 1, guo_entries, None);
    put_marisa_head_node(&mut bytes, index_offset + 40, 0, 0, None);
    put_marisa_head_node(&mut bytes, index_offset + 52, 0, 0, None);
    put_marisa_head_node(&mut bytes, index_offset + 64, 0, 0, Some(xi_trunk_offset));
    put_marisa_head_node(&mut bytes, index_offset + 76, 1, xian_entries, None);
    put_marisa_head_node(
        &mut bytes,
        index_offset + 88,
        1,
        zhong_entries,
        Some(trunk_offset),
    );

    let mut writer = rsmarisa::grimoire::io::Writer::from_vec(Vec::new());
    trie.write(&mut writer)
        .expect("fixture trie should serialize");
    let payload = writer
        .into_inner()
        .expect("fixture trie payload should be returned");
    let string_table_offset = bytes.len();
    bytes.extend_from_slice(&payload);

    put_offset(&mut bytes, 44, syllabary_offset);
    put_offset(&mut bytes, 48, index_offset);
    put_offset(&mut bytes, 60, string_table_offset);
    put_u32_le(&mut bytes, 64, payload.len() as u32);

    bytes
}

fn append_marisa_tail_index(bytes: &mut Vec<u8>, entries: &[(&[i32], u32, f32)]) -> usize {
    let extra_offsets = entries
        .iter()
        .map(|(extra_ids, _, _)| append_marisa_i32_list(bytes, extra_ids))
        .collect::<Vec<_>>();
    let offset = bytes.len();
    bytes.resize(offset + 4 + entries.len() * 16, 0);
    put_u32_le(bytes, offset, entries.len() as u32);
    for (index, ((extra_ids, text_id, weight), extra_offset)) in
        entries.iter().zip(extra_offsets).enumerate()
    {
        let entry_offset = offset + 4 + index * 16;
        put_u32_le(bytes, entry_offset, extra_ids.len() as u32);
        put_offset(bytes, entry_offset + 4, extra_offset);
        put_u32_le(bytes, entry_offset + 8, *text_id);
        put_f32_le(bytes, entry_offset + 12, *weight);
    }
    offset
}

fn append_marisa_i32_list(bytes: &mut Vec<u8>, values: &[i32]) -> usize {
    let offset = bytes.len();
    bytes.resize(offset + values.len() * 4, 0);
    for (index, value) in values.iter().enumerate() {
        put_i32_le(bytes, offset + index * 4, *value);
    }
    offset
}

fn append_marisa_entry_list(bytes: &mut Vec<u8>, entries: &[(u32, f32)]) -> usize {
    let offset = bytes.len();
    bytes.resize(offset + entries.len() * 8, 0);
    for (index, (text_id, weight)) in entries.iter().enumerate() {
        let entry_offset = offset + index * 8;
        put_u32_le(bytes, entry_offset, *text_id);
        put_f32_le(bytes, entry_offset + 4, *weight);
    }
    offset
}

fn put_marisa_trunk_node(
    bytes: &mut [u8],
    offset: usize,
    key: i32,
    entry_count: u32,
    entries_offset: usize,
    next_level_offset: Option<usize>,
) {
    put_i32_le(bytes, offset, key);
    put_u32_le(bytes, offset + 4, entry_count);
    put_offset(bytes, offset + 8, entries_offset);
    if let Some(next_level_offset) = next_level_offset {
        put_offset(bytes, offset + 12, next_level_offset);
    }
}

fn put_marisa_head_node(
    bytes: &mut [u8],
    offset: usize,
    entry_count: u32,
    entries_offset: usize,
    next_level_offset: Option<usize>,
) {
    put_u32_le(bytes, offset, entry_count);
    put_offset(bytes, offset + 4, entries_offset);
    if let Some(next_level_offset) = next_level_offset {
        put_offset(bytes, offset + 8, next_level_offset);
    }
}

fn marisa_id(trie: &rsmarisa::Trie, key: &str) -> u32 {
    let mut agent = rsmarisa::Agent::new();
    agent.set_query_str(key);
    assert!(trie.lookup(&mut agent), "fixture key should exist: {key}");
    agent.key().id() as u32
}

fn put_c_string(bytes: &mut [u8], offset: usize, value: &[u8]) {
    bytes[offset..offset + value.len()].copy_from_slice(value);
}

fn put_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_i32_le(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_f32_le(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_le_bytes());
}

fn put_offset(bytes: &mut [u8], field_offset: usize, target: usize) {
    let raw = i32::try_from(target as isize - field_offset as isize)
        .expect("fixture offset should fit i32");
    put_i32_le(bytes, field_offset, raw);
}
