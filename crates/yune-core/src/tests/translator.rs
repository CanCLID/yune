use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
    sync::Arc,
};

use crate::poet::WeightedSentenceCodeSpan;
use crate::translator::{sentence_path_score_replaces, SentencePathScore};
use crate::{
    build_poet_bin, build_prism_bin, build_table_bin, encode_octagram_key,
    parse_rime_prism_bin_payload, parse_rime_prism_runtime_payload,
    parse_rime_table_bin_advanced_data, parse_rime_table_bin_dictionary, Candidate,
    CandidateFilter, CandidateRequest, CandidateSource, CompactTableByteSource, CompactTableStore,
    Context, DartsDoubleArray, DictionaryLookupRecord, Engine, HistoryTranslator, MemoryOwnerClass,
    OctagramGrammar, OctagramGrammarConfig, OwnedPoetBytes, PoetByteSource, PresetVocabularyEntry,
    PunctuationTranslator, ReverseLookupTranslator, RimeCorrectionEntry, RimePrismBinPayload,
    RimePrismSpellingDescriptor, RimeToleranceRule, SentenceCodeSpan, SentencePolicy,
    StaticTableTranslator, Status, TableDictionary, TableDictionaryAdvancedData, TableEntry,
    TranslationMergePolicy, TranslationResult, Translator, TranslatorScratch, UniquifierFilter,
    UpstreamSentenceModel,
};

struct DropFirstWindowFilter;

#[derive(Debug)]
struct AlgebraPrismByteSource(Arc<[u8]>);

impl CompactTableByteSource for AlgebraPrismByteSource {
    fn bytes(&self) -> &[u8] {
        &self.0
    }

    fn storage_label(&self) -> &'static str {
        "byte_backed"
    }

    fn mapping_mode(&self) -> &'static str {
        "owned_test_bytes"
    }
}

impl CandidateFilter for DropFirstWindowFilter {
    fn name(&self) -> &'static str {
        "uniquifier"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        candidates.retain(|candidate| !candidate.text.starts_with("DROP"));
    }
}

#[test]
fn reverse_lookup_translator_uses_target_dictionary_comments() {
    let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: stroke
version: "0.1"
sort: original
...

火	huo
水	shui
"#,
    )
    .expect("lookup dictionary should parse");
    let target_dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: luna
version: "0.1"
sort: original
...

火	ho
火	huo
"#,
    )
    .expect("target dictionary should parse");

    let translator =
        ReverseLookupTranslator::new(lookup_dictionary, Some(target_dictionary), "`", "");

    let unprefixed_candidates = translator.translate("huo");
    assert_eq!(unprefixed_candidates.len(), 1);
    assert_eq!(
        unprefixed_candidates[0].source,
        CandidateSource::ReverseLookup
    );
    assert_eq!(unprefixed_candidates[0].text, "火");
    assert_eq!(unprefixed_candidates[0].comment, "ho; huo");

    let candidates = translator.translate("`huo");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].source, CandidateSource::ReverseLookup);
    assert_eq!(candidates[0].text, "火");
    assert_eq!(candidates[0].comment, "ho; huo");
}

#[test]
fn bounded_static_table_request_matches_eager_top_candidates() {
    let translator = StaticTableTranslator::parse_rime_dict_yaml(
        r#"
---
name: sample
version: "0.1"
sort: by_weight
...

first	na	9
second	nb	8
third	nc	7
fourth	nd	6
fifth	ne	5
"#,
    )
    .expect("dictionary should parse")
    .with_completion(true)
    .with_sentence(false);
    let mut eager = translator.translate("n");
    eager.sort_by(|left, right| {
        right
            .quality
            .partial_cmp(&left.quality)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let bounded = translator.translate_with_context_and_request(
        "n",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(3).with_debug_full_count(true),
    );

    assert_eq!(
        bounded
            .candidates
            .iter()
            .map(|candidate| (
                candidate.text.as_str(),
                candidate.comment.as_str(),
                candidate.source.clone(),
            ))
            .collect::<Vec<_>>(),
        eager
            .iter()
            .take(3)
            .map(|candidate| (
                candidate.text.as_str(),
                candidate.comment.as_str(),
                candidate.source.clone(),
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(bounded.full_count, Some(5));
    assert!(!bounded.is_complete);
}

#[test]
fn static_table_memory_owner_rows_cover_m43_owner_set() {
    let translator =
        StaticTableTranslator::new([("ni", "你"), ("hao", "好"), ("zhong", "中"), ("guo", "國")])
            .with_sentence(true)
            .with_upstream_sentence_model(5);

    let rows = translator.memory_owner_rows();
    let owner_class = |owner: &str| {
        rows.iter()
            .find(|row| row.owner == owner)
            .map(|row| row.class)
            .expect("owner row should be present")
    };

    assert_eq!(
        owner_class("translator.entries_by_code"),
        MemoryOwnerClass::HeapOwnedGuarded
    );
    assert_eq!(
        owner_class("poet.entries_by_code"),
        MemoryOwnerClass::Shared,
        "the lazy model reports a zero-byte placeholder before first use"
    );
    assert_eq!(owner_class("poet.lookup_index"), MemoryOwnerClass::Shared);
    assert_eq!(
        owner_class("poet.abbreviation_vocabulary"),
        MemoryOwnerClass::Shared
    );
    assert_eq!(
        owner_class("translator.upstream_sentence_model_build_source"),
        MemoryOwnerClass::HeapOwnedGuarded,
        "the retained source recipe owns the pre-initialization heap"
    );

    let _ = translator.translate("nihao");
    let initialized_rows = translator.memory_owner_rows();
    let initialized_class = |owner: &str| {
        initialized_rows
            .iter()
            .find(|row| row.owner == owner)
            .map(|row| row.class)
            .expect("initialized owner row should be present")
    };
    assert_eq!(
        initialized_class("poet.entries_by_code"),
        MemoryOwnerClass::HeapOwnedReducible
    );
    assert_eq!(
        initialized_class("poet.lookup_index"),
        MemoryOwnerClass::HeapOwnedGuarded
    );
    assert_eq!(
        initialized_class("poet.abbreviation_vocabulary"),
        MemoryOwnerClass::HeapOwnedReducible
    );
}

#[test]
fn compact_table_memory_owner_rows_cover_m46_payload_owner_set() {
    let mut stems = HashMap::new();
    stems.insert("你".to_owned(), vec!["nei5".to_owned()]);
    let mut dict_settings = BTreeMap::new();
    dict_settings.insert("display.language".to_owned(), "zh-HK".to_owned());
    let mut lookup_records = HashMap::new();
    lookup_records.insert(
        "你".to_owned(),
        vec![DictionaryLookupRecord {
            code: "nei5".to_owned(),
            fields: vec!["你".to_owned(), "nei5".to_owned(), "1".to_owned()],
        }],
    );
    let dictionary = TableDictionary::with_advanced_data(
        [TableEntry::new("nei5", "你", 10.0)],
        TableDictionaryAdvancedData {
            stems,
            dict_settings,
            corrections: vec![RimeCorrectionEntry::new("nri", "nei")],
            tolerance_rules: vec![RimeToleranceRule::new("nei", ["nri"])],
            lookup_records,
            preset_vocabulary: vec![PresetVocabularyEntry::new("你好", 1.0)],
            ..TableDictionaryAdvancedData::default()
        },
    );
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, None);

    let rows = translator.memory_owner_rows();
    let owner_class = |owner: &str| {
        rows.iter()
            .find(|row| row.owner == owner)
            .map(|row| row.class)
            .expect("owner row should be present")
    };

    for owner in [
        "compact_table.candidate_text_payload",
        "compact_table.candidate_comment_payload",
        "compact_table.stems",
        "compact_table.lookup_records",
        "compact_table.corrections_tolerance",
        "compact_table.dict_settings",
        "compact_table.preset_vocabulary",
    ] {
        assert_eq!(owner_class(owner), MemoryOwnerClass::HeapOwnedRequired);
    }
}

#[test]
fn compact_table_memory_owner_rows_report_storage_backed_normal_codes() {
    let dictionary = TableDictionary::new([
        TableEntry::new("nei", "你", 10.0),
        TableEntry::new("hou", "好", 9.0),
    ]);
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, None);

    let rows = translator.memory_owner_rows();
    let owner = rows
        .iter()
        .find(|row| row.owner == "translator.normal_codes")
        .expect("normal code membership owner row should be present");

    assert_eq!(owner.class, MemoryOwnerClass::Shared);
    assert_eq!(owner.estimated_bytes, 0);
    assert_eq!(owner.storage, "compact_table.has_code");
}

#[test]
fn compact_table_memory_owner_rows_cover_parsed_prism_payload_owner_set() {
    let dictionary = TableDictionary::new([TableEntry::new("nei", "你", 10.0)]);
    let prism_payload = RimePrismBinPayload {
        dict_file_checksum: 1,
        schema_file_checksum: 2,
        num_syllables: 1,
        num_spellings: 2,
        double_array_size: 4,
        double_array: Some(DartsDoubleArray::from_units(vec![1, 2, 3, 4]).unwrap()),
        spelling_map: vec![
            vec![
                RimePrismSpellingDescriptor {
                    syllable_id: 0,
                    spelling_type: 0,
                    is_correction: false,
                    credibility: 0.0,
                    tips: "tip".to_owned(),
                },
                RimePrismSpellingDescriptor {
                    syllable_id: 0,
                    spelling_type: 2,
                    is_correction: false,
                    credibility: -0.5,
                    tips: String::new(),
                },
            ],
            Vec::new(),
        ],
        corrections: vec![RimeCorrectionEntry::new("nri", "nei")],
        tolerance_rules: vec![RimeToleranceRule::new("nei", ["nri", "lei"])],
    };
    let translator =
        StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism_payload));

    let rows = translator.memory_owner_rows();
    let owner = |name: &str| {
        rows.iter()
            .find(|row| row.owner == name)
            .unwrap_or_else(|| panic!("owner row {name} should be present"))
    };

    for name in [
        "prism.double_array_units",
        "prism.spelling_map",
        "prism.corrections_tolerance",
        "prism.tips_payload",
    ] {
        assert_eq!(owner(name).class, MemoryOwnerClass::HeapOwnedRequired);
        assert!(
            owner(name).estimated_bytes > 0,
            "owner row {name} should name retained heap bytes"
        );
    }
    assert_eq!(owner("prism.double_array_units").item_count, 4);
    assert_eq!(owner("prism.spelling_map").item_count, 2);
    assert_eq!(owner("prism.tips_payload").item_count, 1);
}

#[test]
fn reverse_lookup_memory_owner_rows_cover_m46_side_index_owner_set() {
    let dictionary = TableDictionary::new([TableEntry::new("nei", "你", 10.0)]);
    let reverse_dictionary = TableDictionary::new([TableEntry::new("ni", "你", 10.0)]);
    let translator = ReverseLookupTranslator::new(dictionary, Some(reverse_dictionary), "`", ";");

    let rows = translator.memory_owner_rows();
    let owner_class = |owner: &str| {
        rows.iter()
            .find(|row| row.owner == owner)
            .map(|row| row.class)
            .expect("owner row should be present")
    };

    assert_eq!(
        owner_class("reverse_lookup.entries"),
        MemoryOwnerClass::HeapOwnedRequired
    );
    assert_eq!(
        owner_class("reverse_lookup.comments_index"),
        MemoryOwnerClass::HeapOwnedRequired
    );
    assert_eq!(
        owner_class("reverse_lookup.config"),
        MemoryOwnerClass::HeapOwnedRequired
    );
}

#[test]
fn bounded_static_table_request_matches_typeduck_prediction_prefix_top_candidates() {
    let translator = StaticTableTranslator::parse_rime_dict_yaml(
        r#"
---
name: sample
version: "0.1"
sort: by_weight
...

exact-a	hai	100
exact-b	hai	99
exact-c	hai	98
exact-d	hai	97
exact-e	hai	96
prefix	h	90
prediction-a	hai6aa1	80
prediction-b	hai6bb1	79
ordinary-a	haia	70
ordinary-b	haib	69
ordinary-c	haic	68
ordinary-d	haid	67
ordinary-e	haie	66
"#,
    )
    .expect("dictionary should parse")
    .with_completion(true)
    .with_sentence(false)
    .with_prediction_candidate_limit(1)
    .with_prefix_fallback(true);
    let mut eager = translator.translate("hai");
    eager.sort_by(|left, right| {
        right
            .quality
            .partial_cmp(&left.quality)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let bounded = translator.translate_with_context_and_request(
        "hai",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(4).with_debug_full_count(true),
    );

    assert_eq!(
        bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        eager
            .iter()
            .take(4)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(bounded.full_count, Some(eager.len()));
    assert!(!bounded.is_complete);
}

#[test]
fn prediction_limit_ordering_respects_table_sort_policy() {
    let translator_for = |sort: &str| {
        StaticTableTranslator::parse_rime_dict_yaml(&format!(
            r#"
---
name: prediction_sort_policy
version: "0.1"
sort: {sort}
...

first	na	1
second	na	9
"#
        ))
        .expect("dictionary should parse")
        .with_completion(true)
        .with_sentence(false)
        .with_prediction_candidate_limit(1)
        .with_prefix_fallback(true)
    };
    let texts = |result: crate::TranslationResult| {
        result
            .candidates
            .into_iter()
            .map(|candidate| candidate.text)
            .collect::<Vec<_>>()
    };

    for (sort, expected) in [
        ("original", vec!["first", "second"]),
        ("by_weight", vec!["second", "first"]),
    ] {
        let translator = translator_for(sort);
        assert_eq!(
            translator
                .translate("na")
                .into_iter()
                .map(|candidate| candidate.text)
                .collect::<Vec<_>>(),
            expected,
            "complete prediction-limit path must honor sort: {sort}"
        );
        assert_eq!(
            texts(translator.translate_with_context_and_request(
                "na",
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(4).with_debug_full_count(true),
            )),
            expected,
            "bounded prediction-limit path must honor sort: {sort}"
        );
    }
}

#[test]
fn sort_original_k_way_merges_fetch_groups_and_prediction_without_reordering_a_group() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: original_fetch_groups
version: "0.1"
sort: original
...

group-a-head	g1	100
group-a-blocked-high	g1	1000
group-b-head	g2	900
group-b-tail	g2	50
prediction	hai6aa1	950
"#,
    )
    .expect("dictionary should parse");
    let syllabary = ["g1", "g2", "hai6aa1"].map(str::to_owned);
    let formulas = vec!["derive/^g[12]$/hai/".to_owned()];
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        .with_sentence(false)
        .with_prediction_candidate_limit(1);
    let expected = [
        "group-b-head",
        "prediction",
        "group-a-head",
        "group-a-blocked-high",
        "group-b-tail",
    ];

    assert_eq!(
        translator
            .translate("hai")
            .into_iter()
            .map(|candidate| candidate.text)
            .collect::<Vec<_>>(),
        expected,
        "complete lookup should merge group heads by weight while preserving each group's source order"
    );
    let bounded = translator.translate_with_context_and_request(
        "hai",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(4).with_debug_full_count(true),
    );
    assert_eq!(
        bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        &expected[..4],
        "bounded collection must retain each fetch-group head before the constrained merge"
    );
    assert_eq!(bounded.full_count, Some(expected.len()));
}

#[test]
fn sort_original_bounded_many_group_merge_is_exact_prefix_of_complete_order() {
    const GROUP_COUNT: usize = 48;
    let mut source =
        String::from("---\nname: original_many_groups\nversion: '0.1'\nsort: original\n...\n\n");
    let mut syllabary = Vec::new();
    let mut formulas = Vec::new();
    for group in 0..GROUP_COUNT {
        let code = format!("group{group:02}");
        syllabary.push(code.clone());
        formulas.push(format!("derive/^{code}$/hai/"));
        let head_weight = match group {
            0 | 1 => 500,
            2 => 900,
            _ => 300 - group,
        };
        let tail_weight = match group {
            0 => 5_000,
            1 => 4_999,
            2 => 50,
            _ => 2_000 - group,
        };
        source.push_str(&format!(
            "group-{group:02}-head\t{code}\t{head_weight}\n\
             group-{group:02}-tail\t{code}\t{tail_weight}\n"
        ));
    }
    source.push_str("prediction\thai6aa1\t800\ncompletion\thaiz\t10000\n");
    syllabary.extend(["hai6aa1".to_owned(), "haiz".to_owned()]);

    let dictionary =
        TableDictionary::parse_rime_dict_yaml(&source).expect("many-group dictionary should parse");
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("many-group prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        .with_sentence(false)
        .with_prediction_candidate_limit(1);
    let complete = translator
        .translate("hai")
        .into_iter()
        .map(|candidate| candidate.text)
        .collect::<Vec<_>>();

    assert_eq!(
        &complete[..6],
        [
            "group-02-head",
            "prediction",
            "group-00-head",
            "group-00-tail",
            "group-01-head",
            "group-01-tail",
        ],
        "equal heads must favor the first-seen group, whose later high row stays blocked until its head is emitted"
    );
    assert_eq!(
        complete.last().map(String::as_str),
        Some("completion"),
        "completion-category rows must remain behind all exact-category groups regardless of weight"
    );

    for limit in [1, 2, 3, 4, 5, 8, 13, 21, 64, complete.len() + 3, usize::MAX] {
        let bounded = translator.translate_with_context_and_request(
            "hai",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(limit).with_debug_full_count(true),
        );
        let bounded_text = bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        let expected_len = limit.min(complete.len());
        assert_eq!(
            bounded_text,
            complete[..expected_len]
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            "bounded merge at limit {limit} must be the exact complete-order prefix"
        );
        assert_eq!(bounded.full_count, Some(complete.len()));
        assert_eq!(bounded.is_complete, limit >= complete.len());
    }

    let without_debug_count = translator.translate_with_context_and_request(
        "hai",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(2),
    );
    assert_eq!(without_debug_count.full_count, None);
    assert!(
        !without_debug_count.is_complete,
        "hiding the debug count must not falsely mark a known-truncated prefix complete"
    );
}

#[test]
fn sort_original_low_prediction_does_not_jump_between_full_span_groups() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: original_full_span_groups
version: "0.1"
sort: original
...

full-a	g1	100
full-b	g2	90
prediction	hai6aa1	1
"#,
    )
    .expect("dictionary should parse");
    let syllabary = ["g1", "g2", "hai6aa1"].map(str::to_owned);
    let formulas = vec!["derive/^g[12]$/hai/".to_owned()];
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        .with_sentence(false)
        .with_prediction_candidate_limit(1);
    let expected = ["full-a", "full-b", "prediction"];

    assert_eq!(
        translator
            .translate("hai")
            .into_iter()
            .map(|candidate| candidate.text)
            .collect::<Vec<_>>(),
        expected,
        "changing canonical groups at the same consumed span must not promote a low prediction"
    );
    let bounded = translator.translate_with_context_and_request(
        "hai",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(4).with_debug_full_count(true),
    );
    assert_eq!(
        bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        expected,
        "bounded lookup must apply the same full-span prediction guard"
    );
}

#[test]
fn sort_original_prediction_requires_an_ordinary_candidate_to_lead() {
    let translator_for = |sort: &str, rows: &str| {
        let dictionary = TableDictionary::parse_rime_dict_yaml(&format!(
            "---\nname: prediction_first_guard\nversion: '0.1'\nsort: {sort}\n...\n\n{rows}"
        ))
        .expect("dictionary should parse");
        StaticTableTranslator::from_compact_dictionary(dictionary, None)
            .with_completion(true)
            .with_sentence(false)
            .with_prediction_candidate_limit(1)
    };
    let texts = |translator: &StaticTableTranslator| {
        translator
            .translate("hai")
            .into_iter()
            .map(|candidate| candidate.text)
            .collect::<Vec<_>>()
    };
    let bounded_texts = |translator: &StaticTableTranslator| {
        translator
            .translate_with_context_and_request(
                "hai",
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(4).with_debug_full_count(true),
            )
            .candidates
            .into_iter()
            .map(|candidate| candidate.text)
            .collect::<Vec<_>>()
    };

    let ordinary_then_prediction =
        translator_for("original", "ordinary\thaia\t100\nprediction\thai6aa1\t50\n");
    assert_eq!(texts(&ordinary_then_prediction), ["ordinary", "prediction"]);
    assert_eq!(
        bounded_texts(&ordinary_then_prediction),
        ["ordinary", "prediction"]
    );

    let prediction_only = translator_for("original", "prediction\thai6aa1\t50\n");
    assert!(texts(&prediction_only).is_empty());
    assert!(bounded_texts(&prediction_only).is_empty());
    let bounded_prediction_only = prediction_only.translate_with_context_and_request(
        "hai",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(4).with_debug_full_count(true),
    );
    assert_eq!(bounded_prediction_only.full_count, Some(0));
    assert!(bounded_prediction_only.is_complete);

    let by_weight_prediction_only = translator_for("by_weight", "prediction\thai6aa1\t50\n");
    assert_eq!(texts(&by_weight_prediction_only), ["prediction"]);
    assert_eq!(bounded_texts(&by_weight_prediction_only), ["prediction"]);
}

#[test]
fn reverse_lookup_translator_completion_is_opt_in() {
    let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: stroke
version: "0.1"
sort: original
...

火	huo
水	shui
"#,
    )
    .expect("lookup dictionary should parse");

    let exact_translator = ReverseLookupTranslator::new(lookup_dictionary.clone(), None, "`", "");
    assert!(exact_translator.translate("`hu").is_empty());

    let completion_translator =
        ReverseLookupTranslator::new(lookup_dictionary, None, "`", "").with_completion(true);
    let candidates = completion_translator.translate("`hu");
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].text, "火");
    assert_eq!(candidates[0].comment, "huo");
}

#[test]
fn reverse_lookup_translator_honors_librime_segment_tag() {
    let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: stroke
version: "0.1"
sort: original
...

火	huo
"#,
    )
    .expect("lookup dictionary should parse");

    let mut engine = Engine::new();
    engine.add_translator(ReverseLookupTranslator::new(
        lookup_dictionary.clone(),
        None,
        "`",
        "",
    ));
    engine.set_input("`huo");
    assert!(engine
        .context()
        .candidates
        .iter()
        .all(|candidate| candidate.source != CandidateSource::ReverseLookup));

    engine.set_segment_tags(["abc", "reverse_lookup"]);
    let reverse_candidates = engine
        .context()
        .candidates
        .iter()
        .filter(|candidate| candidate.source == CandidateSource::ReverseLookup)
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(reverse_candidates, ["火"]);

    let mut tagged_engine = Engine::new();
    tagged_engine.add_translator(
        ReverseLookupTranslator::new(lookup_dictionary, None, "`", "").with_tag("custom"),
    );
    tagged_engine.set_segment_tags(["abc", "reverse_lookup"]);
    tagged_engine.set_input("`huo");
    assert!(tagged_engine
        .context()
        .candidates
        .iter()
        .all(|candidate| candidate.source != CandidateSource::ReverseLookup));

    tagged_engine.set_segment_tags(["abc", "custom"]);
    let reverse_candidates = tagged_engine
        .context()
        .candidates
        .iter()
        .filter(|candidate| candidate.source == CandidateSource::ReverseLookup)
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(reverse_candidates, ["火"]);
}

#[test]
fn history_translator_returns_recent_commits_for_configured_input() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你"), ("hao", "好")]));
    engine.add_translator(HistoryTranslator::new("his").with_size(2));

    engine.set_input("ni");
    assert_eq!(engine.commit_highlighted(), Some("你".to_owned()));
    engine.set_input("hao");
    assert_eq!(engine.commit_highlighted(), Some("好".to_owned()));

    engine.set_input("hi");
    assert_eq!(engine.context().candidates[0].text, "hi");

    engine.set_input("his");
    let history_candidates = engine
        .context()
        .candidates
        .iter()
        .take(2)
        .map(|candidate| (candidate.text.as_str(), &candidate.source))
        .collect::<Vec<_>>();
    assert_eq!(
        history_candidates,
        [
            ("好", &CandidateSource::History),
            ("你", &CandidateSource::History)
        ]
    );

    let mut tagged_engine = Engine::new();
    tagged_engine.add_translator(StaticTableTranslator::new([("ni", "你")]));
    tagged_engine.add_translator(HistoryTranslator::new("his").with_tag("custom"));
    tagged_engine.set_input("ni");
    assert_eq!(tagged_engine.commit_highlighted(), Some("你".to_owned()));
    tagged_engine.set_input("his");
    assert!(tagged_engine
        .context()
        .candidates
        .iter()
        .all(|candidate| candidate.source != CandidateSource::History));

    tagged_engine.set_segment_tags(["abc", "custom"]);
    let history_candidates = tagged_engine
        .context()
        .candidates
        .iter()
        .filter(|candidate| candidate.source == CandidateSource::History)
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(history_candidates, ["你"]);
}

#[test]
fn punctuation_translator_offers_half_shape_candidates_before_echo() {
    let mut engine = Engine::new();
    engine.add_translator(PunctuationTranslator::default_half_shape());

    engine.process_char('.');

    assert_eq!(engine.context().composition.input, ".");
    assert_eq!(engine.context().candidates[0].text, "。");
    assert_eq!(
        engine.context().candidates[0].source,
        CandidateSource::Punctuation
    );
    assert_eq!(engine.context().candidates[1].text, ".");
}

#[test]
fn punctuation_candidate_commits_through_selection_key() {
    let mut engine = Engine::new();
    engine.add_translator(PunctuationTranslator::default_half_shape());

    let commits = engine
        .process_key_sequence(".{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["。"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("。"));
    assert!(!engine.status().is_composing);
}

#[test]
fn punctuation_translator_tracks_full_shape_option() {
    let mut engine = Engine::new();
    engine.add_translator(PunctuationTranslator::with_shape_entries(
        [("/", "、")],
        [("/", "／")],
    ));

    engine.process_char('/');
    assert_eq!(engine.context().candidates[0].text, "、");

    engine.set_option("full_shape", true);
    assert_eq!(engine.context().candidates[0].text, "／");

    engine.set_option("full_shape", false);
    assert_eq!(engine.context().candidates[0].text, "、");
}

#[test]
fn punctuation_translator_uses_symbols_as_shape_fallback() {
    let mut engine = Engine::new();
    engine.add_translator(PunctuationTranslator::with_shape_and_symbol_entries(
        [("/", "、")],
        [("/", "／")],
        [("/", "symbol-slash"), ("/fh", "©")],
    ));

    engine
        .process_key_sequence("/fh")
        .expect("keys should parse");
    assert_eq!(engine.context().candidates[0].text, "©");
    assert_eq!(engine.context().candidates[1].text, "/fh");

    engine.clear_composition();
    engine.process_char('/');
    assert_eq!(engine.context().candidates[0].text, "、");
    assert_eq!(engine.context().candidates[1].text, "/");

    engine.set_option("full_shape", true);
    assert_eq!(engine.context().candidates[0].text, "／");
    assert_eq!(engine.context().candidates[1].text, "/");
}

#[test]
fn punctuation_translator_uses_librime_shape_comments() {
    let mut engine = Engine::new();
    engine.add_translator(PunctuationTranslator::with_shape_and_symbol_entries(
        [("/", "/"), (",", "、")],
        [("/", "／")],
        [("/copyright", "©")],
    ));

    engine.process_char('/');
    assert_eq!(engine.context().candidates[0].comment, "〔半角〕");

    engine.clear_composition();
    engine.process_char(',');
    assert_eq!(engine.context().candidates[0].comment, "〔全角〕");

    engine.set_option("full_shape", true);
    engine.clear_composition();
    engine.process_char('/');
    assert_eq!(engine.context().candidates[0].comment, "〔全角〕");

    engine.clear_composition();
    engine
        .process_key_sequence("/copyright")
        .expect("keys should parse");
    assert_eq!(engine.context().candidates[0].comment, "");
}

#[test]
fn static_table_sentence_word_penalty_defaults_to_upstream_neutral() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: sentence_penalty
version: "0.1"
sort: by_weight
...

A	ab	1000
B	cd	1000
C	ef	1000
X	abc	1
Y	def	1
"#,
    )
    .expect("sentence penalty dictionary should parse");

    let translator = StaticTableTranslator::from_dictionary(dictionary).with_sentence(true);
    let candidates = translator.translate("abcdef");

    assert_eq!(candidates[0].source, CandidateSource::Sentence);
    assert_eq!(candidates[0].text, "ABC");
}

#[test]
fn static_table_sentence_word_penalty_can_opt_into_typeduck_profile_value() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: sentence_penalty
version: "0.1"
sort: by_weight
...

A	ab	1000
B	cd	1000
C	ef	1000
X	abc	1
Y	def	1
"#,
    )
    .expect("sentence penalty dictionary should parse");

    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_sentence(true)
        .with_sentence_word_penalty(21.0);
    let candidates = translator.translate("abcdef");

    assert_eq!(candidates[0].source, CandidateSource::Sentence);
    assert_eq!(candidates[0].text, "XY");
}

#[test]
fn legacy_fallback_raw_and_v2_compiled_weights_keep_sentence_and_prediction_parity() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: legacy_weight_domain_parity
version: "0.1"
sort: by_weight
...

A	ab	1000
B	cd	1000
C	ef	1000
X	abc	1
Y	def	1
HIGH	na	100
BOUNDARY	nb	10
DROP	nc	9
"#,
    )
    .expect("legacy weight-domain fixture should parse");

    let configure = |translator: StaticTableTranslator| {
        translator
            .with_completion(true)
            .with_prediction_weight_threshold(10.0)
            .with_sentence(true)
            .with_sentence_policy(SentencePolicy::LegacyFallback)
            .with_sentence_word_penalty(21.0)
    };
    let raw = configure(StaticTableTranslator::from_dictionary(dictionary.clone()));

    let table_bytes = build_table_bin(&dictionary, 0x5904_b007);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("v2 compiled metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("v2 compiled table should parse");
    let compiled = configure(StaticTableTranslator::from_compact_table_store(store, None));

    let signature = |candidates: Vec<Candidate>| {
        candidates
            .into_iter()
            .map(|candidate| (candidate.text, candidate.source))
            .collect::<Vec<_>>()
    };
    let raw_sentence = signature(raw.translate("abcdef"));
    let compiled_sentence = signature(compiled.translate("abcdef"));
    assert_eq!(compiled_sentence, raw_sentence);
    assert_eq!(
        raw_sentence[0],
        ("XY".to_owned(), CandidateSource::Sentence)
    );

    let raw_predictions = signature(raw.translate("n"));
    let compiled_predictions = signature(compiled.translate("n"));
    assert_eq!(compiled_predictions, raw_predictions);
    assert_eq!(
        raw_predictions,
        [
            ("HIGH".to_owned(), CandidateSource::Completion),
            ("BOUNDARY".to_owned(), CandidateSource::Completion),
        ],
        "the inclusive threshold and prediction order must be weight-domain invariant"
    );
}

#[test]
fn legacy_fallback_original_prediction_merge_uses_raw_weights_in_eager_and_bounded_paths() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: legacy_original_prediction_weight_domain
version: "0.1"
sort: original
...

ANCHOR	n	1
ORDINARY	n	0.25
PREDICTION	n1a	0.5
"#,
    )
    .expect("sort-original prediction fixture should parse");

    let configure = |translator: StaticTableTranslator| {
        translator
            .with_completion(true)
            .with_prediction_candidate_limit(1)
            .with_sentence(true)
            .with_sentence_policy(SentencePolicy::LegacyFallback)
    };
    let raw = configure(StaticTableTranslator::from_dictionary(dictionary.clone()));
    let table_bytes = build_table_bin(&dictionary, 0x5904_b008);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("sort-original v2 metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("sort-original v2 table should parse");
    let compiled = configure(StaticTableTranslator::from_compact_table_store(store, None));

    let signature = |candidates: &[Candidate]| {
        candidates
            .iter()
            .map(|candidate| (candidate.text.clone(), candidate.source.clone()))
            .collect::<Vec<_>>()
    };
    let expected = [
        ("ANCHOR".to_owned(), CandidateSource::Table),
        ("PREDICTION".to_owned(), CandidateSource::Completion),
        ("ORDINARY".to_owned(), CandidateSource::Table),
    ];
    let raw_eager = raw.translate("n");
    let compiled_eager = compiled.translate("n");
    assert_eq!(signature(&raw_eager), expected);
    assert_eq!(signature(&compiled_eager), expected);

    let bounded = |translator: &StaticTableTranslator| {
        translator
            .translate_with_context_and_request(
                "n",
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(3),
            )
            .candidates
    };
    assert_eq!(signature(&bounded(&raw)), expected);
    assert_eq!(signature(&bounded(&compiled)), expected);
}

#[test]
fn sentence_span_fold_prefers_exact_then_quality_and_keeps_first_tie() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::new([
        TableEntry::new("ab", "FUZZY", 1000.0),
        TableEntry::new("ax", "FIRST", 10.0),
        TableEntry::new("ax", "SECOND", 10.0),
        TableEntry::new("cd", "TAIL", 10.0),
    ]);
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&["derive/^ab$/ax/".to_owned()])
        .with_sentence(true);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let candidates = translator.translate("axcd");
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(candidates[0].source, CandidateSource::Sentence);
    assert_eq!(
        candidates[0].text, "FIRSTTAIL",
        "an exact surface beats a heavier fuzzy row and an exact tie keeps source order"
    );
    assert_eq!(metrics.sentence_entry_matches_collected, 2);
    assert_eq!(metrics.sentence_path_clones, 2);
}

#[test]
fn sentence_span_shadow_replays_nan_endpoint_comparisons_per_candidate() {
    let dictionary = TableDictionary::with_advanced_data(
        [
            TableEntry::new("ab", "OLD", 1.0),
            TableEntry::new("cd", "PATH", 1.0),
            TableEntry::new("abc", "NEW", 1.0),
            TableEntry::new("d", "NAN", f32::NAN),
            TableEntry::new("d", "GOOD", 10.0),
        ],
        TableDictionaryAdvancedData {
            sort_by_weight: Some(false),
            ..TableDictionaryAdvancedData::default()
        },
    );
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_sentence(true)
        .with_sentence_word_penalty(f32::NAN);

    let candidates = translator.translate("abcd");

    assert_eq!(candidates[0].source, CandidateSource::Sentence);
    assert_eq!(
        candidates[0].text, "NEWGOOD",
        "the first unordered local row must not hide a later row that wins against the live endpoint"
    );
}

#[test]
fn sentence_path_shadow_uses_predecessor_added_f32_rounding() {
    let predecessor = 16_777_216.0_f32;
    let candidate = SentencePathScore {
        fuzzy_pieces: 0,
        quality: predecessor + 0.5,
        raw_quality: 11.0,
    };
    let existing = SentencePathScore {
        fuzzy_pieces: 0,
        quality: predecessor,
        raw_quality: 10.0,
    };

    assert_eq!(candidate.quality, existing.quality);
    assert!(sentence_path_score_replaces(candidate, existing));
}

#[test]
fn sentence_path_shadow_uses_raw_quality_for_sub_one_weights() {
    let lower = SentencePathScore {
        fuzzy_pieces: 0,
        quality: 0.0,
        raw_quality: 0.25,
    };
    let higher = SentencePathScore {
        raw_quality: 0.5,
        ..lower
    };

    assert!(sentence_path_score_replaces(higher, lower));
    assert!(!sentence_path_score_replaces(lower, higher));
}

#[test]
fn sentence_path_shadow_keeps_the_first_exact_tie() {
    let first = SentencePathScore {
        fuzzy_pieces: 1,
        quality: 2.0,
        raw_quality: 3.0,
    };

    assert!(!sentence_path_score_replaces(first, first));
}

#[test]
fn static_table_sentence_candidate_records_m39_owner_metrics() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: sentence_metrics
version: "0.1"
sort: by_weight
...

A	ab	1000
B	cd	1000
C	ef	1000
"#,
    )
    .expect("sentence metrics dictionary should parse");
    let translator = StaticTableTranslator::from_dictionary(dictionary).with_sentence(true);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let candidates = translator.translate("abcdef");
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(candidates[0].source, CandidateSource::Sentence);
    assert_eq!(candidates[0].text, "ABC");
    assert!(metrics.sentence_candidate_calls >= 1);
    assert!(metrics.sentence_candidate_ns > 0);
    assert!(metrics.sentence_substrings_considered > 0);
    assert!(metrics.sentence_exact_lookup_calls > 0);
    assert!(metrics.sentence_exact_lookup_ns > 0);
    assert!(metrics.sentence_exact_lookup_candidates >= 3);
    assert!(metrics.sentence_entry_matches_collected >= 3);
    assert!(metrics.sentence_path_clones >= 3);
    assert!(metrics.sentence_path_replacements >= 3);
    assert!(metrics.sentence_max_live_paths >= 1);
    assert!(metrics.sentence_result_candidates >= 1);
}

#[test]
fn static_table_records_m39_prefix_and_upstream_sentence_metrics() {
    let _guard = super::m37_metrics_test_guard();
    let prefix_translator = StaticTableTranslator::new([("nei", "你")]).with_prefix_fallback(true);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let prefix_candidates = prefix_translator.translate("neix");
    let prefix_metrics = crate::m37_metrics_snapshot();

    assert_eq!(prefix_candidates[0].text, "你");
    assert!(prefix_metrics.prefix_fallback_calls > 0);
    assert!(prefix_metrics.prefix_fallback_ns > 0);
    assert!(prefix_metrics.prefix_fallback_views_visited > 0);
    assert!(prefix_metrics.prefix_fallback_candidates > 0);

    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: upstream_sentence_metrics
version: "0.1"
sort: by_weight
...

A	ab	1000
B	cd	1000
"#,
    )
    .expect("upstream sentence metrics dictionary should parse");
    let upstream_translator =
        StaticTableTranslator::from_dictionary(dictionary).with_upstream_sentence_model(10);

    crate::m37_metrics_reset();
    let upstream_candidates = upstream_translator.translate("abcd");
    let upstream_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert!(!upstream_candidates.is_empty());
    assert!(upstream_metrics.upstream_sentence_model_calls > 0);
    assert!(upstream_metrics.upstream_sentence_model_ns > 0);
    assert!(upstream_metrics.upstream_sentence_model_candidates > 0);
}

#[test]
fn bounded_request_uses_limited_upstream_sentence_model_without_full_fallback() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: bounded_upstream_sentence
version: "0.1"
sort: by_weight
...

A	ab	1000
B	cd	1000
C	ef	1000
"#,
    )
    .expect("bounded upstream sentence dictionary should parse");
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_sentence(true)
        .with_upstream_sentence_model(10);
    let context = Context::default();

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "abcdef",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(1),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(result.candidates[0].text, "ABC");
    assert!(!result.is_complete);
    assert_eq!(metrics.full_list_fallback_count, 0);
    assert_eq!(metrics.upstream_sentence_model_calls, 1);
}

#[test]
fn upstream_script_translation_keeps_sentence_ahead_of_predictive_completion_stream() {
    let dictionary = TableDictionary::new([
        TableEntry::new("ab", "AB", 100.0),
        TableEntry::new("cd", "CD", 100.0),
        TableEntry::new("abcdef", "PREDICT", 1_000.0),
    ]);
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_completion(true)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_upstream_sentence_model(10);

    let assert_page = |candidates: &[Candidate]| {
        assert_eq!(
            candidates
                .iter()
                .take(3)
                .map(|candidate| (candidate.text.as_str(), candidate.source.clone()))
                .collect::<Vec<_>>(),
            [
                ("ABCD", CandidateSource::Sentence),
                ("PREDICT", CandidateSource::Completion),
                (
                    "AB",
                    CandidateSource::PartialTable {
                        consumed: 2,
                        recompose_on_default: false,
                    },
                ),
            ],
            "a completion-only outer stream must not suppress Poet, and full-consumed predictions precede shorter phrase rows"
        );
    };

    let complete = translator.translate("abcd");
    assert_page(&complete);
    let bounded = translator.translate_with_context_and_request(
        "abcd",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(5),
    );
    assert_page(&bounded.candidates);
}

#[test]
fn legacy_sentence_policy_keeps_poet_fallback_only_when_completion_exists() {
    let dictionary = TableDictionary::new([
        TableEntry::new("ab", "AB", 100.0),
        TableEntry::new("cd", "CD", 100.0),
        TableEntry::new("abcdef", "PREDICT", 1_000.0),
    ]);
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_completion(true)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::LegacyFallback)
        .with_upstream_sentence_model(10);

    let assert_legacy = |candidates: &[Candidate]| {
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| (candidate.text.as_str(), candidate.source.clone()))
                .collect::<Vec<_>>(),
            [("PREDICT", CandidateSource::Completion)],
            "LegacyFallback must not merge Poet's sentence/phrase stream into an existing completion stream"
        );
    };

    assert_legacy(&translator.translate("abcd"));
    let bounded = translator.translate_with_context_and_request(
        "abcd",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(5),
    );
    assert_legacy(&bounded.candidates);
}

#[test]
fn upstream_script_phrase_dedup_preserves_outer_comment_formatting() {
    let build = || {
        StaticTableTranslator::from_dictionary(TableDictionary::new([
            TableEntry::new("ab", "AB", 100.0),
            TableEntry::new("c", "C", 100.0),
        ]))
        .with_prefix_fallback(true)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_upstream_sentence_model(10)
    };
    let translator = build().with_comment_format(&["xform/^ab$/META/".to_owned()]);

    let candidates = translator.translate("abc");
    let phrase = candidates
        .iter()
        .find(|candidate| {
            candidate.text == "AB"
                && matches!(
                    candidate.source,
                    CandidateSource::PartialTable { consumed: 2, .. }
                )
        })
        .expect("the authoritative Poet phrase row should remain visible");

    assert_eq!(phrase.comment, "META");

    let unformatted = build().translate("abc");
    let phrase = unformatted
        .iter()
        .find(|candidate| {
            candidate.text == "AB"
                && matches!(
                    candidate.source,
                    CandidateSource::PartialTable { consumed: 2, .. }
                )
        })
        .expect("the unformatted Poet phrase row should remain visible");
    assert_eq!(
        phrase.comment, "",
        "an ordinary outer lookup code is not ScriptTranslation display metadata"
    );
}

#[test]
fn upstream_script_dictionary_exclude_removes_words_from_the_sentence_graph() {
    let translator = StaticTableTranslator::from_dictionary(TableDictionary::new([
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("b", "B", 200.0),
        TableEntry::new("b", "X", 100.0),
        TableEntry::new("c", "C", 100.0),
    ]))
    .with_prefix_fallback(true)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_model(10)
    .with_dictionary_exclude(["B"]);

    let candidates = translator.translate("abc");
    assert!(
        candidates.iter().all(|candidate| candidate.text != "B"),
        "dictionary exclusions must cover the independent phrase stream"
    );
    assert!(
        candidates.iter().all(|candidate| candidate.text != "ABC"),
        "an excluded graph word must not survive inside a composed sentence"
    );
    assert!(candidates.iter().any(|candidate| {
        candidate.text == "AXC" && candidate.source == CandidateSource::Sentence
    }));
}

#[test]
fn upstream_script_translation_keeps_sentence_with_correction_only_exact_lookup() {
    let formulas = ["derive/^abcdx$/abcd/correction".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("ab", "AB", 100.0),
        TableEntry::new("cd", "CD", 100.0),
        TableEntry::new("abcdx", "CORRECTED", 1_000.0),
    ]);
    // Construct Poet from canonical rows before installing the correction-only
    // surface on the outer table. This mirrors the deployed compact split:
    // poet.bin stays canonical while prism/table lookup owns algebra surfaces.
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_upstream_sentence_model(10)
        .with_spelling_algebra(&formulas)
        .with_completion(false)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript);

    let result = translator.translate_with_context_and_request(
        "abcd",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(5),
    );
    assert_eq!(result.candidates[0].text, "ABCD");
    assert_eq!(result.candidates[0].source, CandidateSource::Sentence);
    assert_eq!(result.candidates[1].text, "CORRECTED");
    assert_eq!(result.candidates[1].source, CandidateSource::Table);
    assert_eq!(result.candidates[2].text, "AB");
    assert_eq!(
        result.candidates[2].source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: false,
        }
    );
}

#[test]
fn reliable_exact_system_phrase_suppresses_upstream_sentence() {
    let dictionary = TableDictionary::new([
        TableEntry::new("ab", "AB", 100.0),
        TableEntry::new("cd", "CD", 100.0),
        TableEntry::new("abcd", "EXACT", 1_000.0),
    ]);
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_upstream_sentence_model(10);

    let candidates = translator.translate("abcd");
    assert_eq!(candidates[0].text, "EXACT");
    assert_eq!(candidates[0].source, CandidateSource::Table);
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.source != CandidateSource::Sentence),
        "a reliable exact non-correction system phrase owns the translation before Poet"
    );
}

#[test]
fn upstream_script_limits_and_cutoff_are_builder_order_independent() {
    let dictionary = || {
        TableDictionary::new([
            TableEntry::new("ab", "AB", 100.0),
            TableEntry::new("cd", "CD", 100.0),
        ])
    };
    let before_model = StaticTableTranslator::from_dictionary(dictionary())
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_upstream_script_translation_limits(2, 0)
        .with_upstream_sentence_cutoff_threshold(0.0)
        .with_upstream_sentence_model(10);
    let after_model = StaticTableTranslator::from_dictionary(dictionary())
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_upstream_sentence_model(10)
        .with_upstream_script_translation_limits(2, 0)
        .with_upstream_sentence_cutoff_threshold(0.0);

    let shape = |translator: &StaticTableTranslator| {
        translator
            .translate("abcd")
            .into_iter()
            .map(|candidate| (candidate.text, candidate.source))
            .collect::<Vec<_>>()
    };
    let before = shape(&before_model);
    let after = shape(&after_model);
    assert_eq!(before, after);
    assert!(
        before
            .iter()
            .all(|(_, source)| source != &CandidateSource::Sentence),
        "max_homophones=0 must reach a model constructed after the limits builder"
    );
}

#[test]
fn bounded_engine_reuses_owned_upstream_sentence_states_for_growing_null_grammar_input() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: incremental_upstream_sentence
version: "0.1"
sort: by_weight
...

A	a	100
B	b	100
C	c	100
"#,
    )
    .expect("incremental upstream sentence dictionary should parse");
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.set_schema("luna_pinyin", "Luna Pinyin");
    engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_completion(false)
            .with_sentence(true)
            .with_upstream_sentence_model(10),
    );

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    engine
        .process_key_sequence("abc")
        .expect("key sequence should parse");
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(engine.context().candidates[0].text, "ABC");
    assert_eq!(metrics.full_list_fallback_count, 0);
    assert_eq!(metrics.upstream_sentence_model_calls, 2);
    assert_eq!(metrics.upstream_sentence_model_incremental_reuse_hits, 1);
    assert_eq!(
        metrics.upstream_sentence_model_incremental_discarded_rebuild_chars, 2,
        "the first sentence-model call builds the scratch for `ab`; the `abc` refresh should extend it"
    );
}

#[test]
fn compact_abbreviation_translator_uses_preset_vocabulary_for_full_pinyin_sentence_ranking() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        r#"
---
name: compact_full_pinyin_sentence
version: "0.1"
sort: by_weight
use_preset_vocabulary: true
...

A	jian	20000
B	li	20000
X	jian	30000
Y	li	30000
"#,
        std::iter::empty::<&str>(),
        |_| None,
        |name| (name == "essay").then(|| "AB\t20000\n".to_owned()),
    )
    .expect("dictionary should parse");
    let syllabary = ["jian", "li"].map(str::to_owned);
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_sentence(true)
        .with_spelling_algebra(&formulas)
        .with_upstream_sentence_model(10);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "jianli",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(5),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(result.candidates[0].text, "AB");
    assert_eq!(result.candidates[0].source, CandidateSource::Table);
    assert!(
        metrics.upstream_sentence_model_vocabulary_entries_considered > 0,
        "full-pinyin compact sentence lookup must consider preset phrase vocabulary"
    );
    assert_eq!(metrics.abbreviation_span_discovery_calls, 0);
    assert_eq!(metrics.abbreviation_code_span_graph_build_ns, 0);
}

#[test]
fn bounded_compact_translator_uses_prism_abbreviation_spans_for_sentence_model() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        r#"
---
name: compact_abbreviation_sentence
version: "0.1"
sort: by_weight
use_preset_vocabulary: true
...

A	chong	100
B	shang	100
C	zhu	100
D	yi	100
"#,
        std::iter::empty::<&str>(),
        |_| None,
        |name| (name == "essay").then(|| "ABCD\t1000\n".to_owned()),
    )
    .expect("dictionary should parse");
    let syllabary = ["chong", "shang", "yi", "zhu"].map(str::to_owned);
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_sentence(true)
        .with_spelling_algebra(&formulas)
        .with_upstream_sentence_model(10);
    let context = Context::default();

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let full_pinyin_result = translator.translate_with_context_and_request(
        "chongshangzhuyi",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(5),
    );
    let full_pinyin_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(full_pinyin_result.candidates[0].text, "ABCD");
    assert_eq!(
        full_pinyin_metrics.upstream_sentence_model_vocabulary_entries_considered, 1,
        "full-pinyin sentence lookup must consider the normal preset vocabulary"
    );
    assert_eq!(
        full_pinyin_metrics.abbreviation_span_discovery_calls, 0,
        "full-pinyin sentence lookup must not invoke the M42 abbreviation path"
    );
    assert_eq!(
        full_pinyin_metrics.abbreviation_code_span_graph_build_ns, 0,
        "full-pinyin sentence lookup must not record abbreviation code-span graph work"
    );
    assert_eq!(full_pinyin_metrics.abbreviation_span_discovery_ns, 0);
    assert_eq!(
        full_pinyin_metrics.abbreviation_span_candidates_considered,
        0
    );
    assert_eq!(full_pinyin_metrics.abbreviation_span_codes_emitted, 0);
    assert_eq!(full_pinyin_metrics.abbreviation_model_has_code_calls, 0);
    assert_eq!(full_pinyin_metrics.abbreviation_model_has_code_ns, 0);
    assert_eq!(full_pinyin_metrics.abbreviation_sentence_ranking_ns, 0);
    assert_eq!(full_pinyin_metrics.abbreviation_preedit_format_ns, 0);
    assert_eq!(full_pinyin_metrics.abbreviation_candidate_format_ns, 0);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "cszy",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(5),
    );
    let abbreviation_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(result.candidates[0].text, "ABCD");
    assert_eq!(result.candidates[0].source, CandidateSource::Sentence);
    assert!(result.is_complete);
    assert!(abbreviation_metrics.abbreviation_span_discovery_calls > 0);
    assert!(abbreviation_metrics.abbreviation_span_discovery_ns > 0);
    assert!(abbreviation_metrics.abbreviation_span_candidates_considered > 0);
    assert!(abbreviation_metrics.abbreviation_span_codes_emitted > 0);
    assert!(abbreviation_metrics.abbreviation_model_has_code_calls > 0);
    assert!(abbreviation_metrics.abbreviation_model_has_code_ns > 0);
    assert!(abbreviation_metrics.abbreviation_code_span_graph_build_ns > 0);
    assert!(abbreviation_metrics.abbreviation_sentence_ranking_ns > 0);
    assert!(abbreviation_metrics.abbreviation_preedit_format_ns > 0);
    assert!(abbreviation_metrics.abbreviation_candidate_format_ns > 0);
}

#[test]
fn long_luna_rows_do_not_record_m44_short_key_metrics() {
    let _guard = super::m37_metrics_test_guard();
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.set_schema("luna_pinyin", "Luna Pinyin");
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: long_luna_metrics
version: "0.1"
sort: by_weight
...

LONG	ceshiyixiachangjushuruxingnengzenyang	100
ZHONG	zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong	100
"#,
        )
        .expect("dictionary should parse")
        .with_completion(true)
        .with_sentence(false),
    );

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    engine
        .process_key_sequence("ceshiyixiachangjushuruxingnengzenyang")
        .expect("key sequence should parse");
    engine.clear_composition();
    engine
        .process_key_sequence("zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong")
        .expect("key sequence should parse");
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(metrics.short_key_candidate_rows_scanned, 0);
    assert_eq!(metrics.short_key_candidates_materialized, 0);
    assert_eq!(metrics.short_key_candidates_cloned, 0);
    assert_eq!(metrics.short_key_filter_ns, 0);
    assert_eq!(metrics.short_key_sort_rank_ns, 0);
    assert_eq!(metrics.short_key_comment_quality_ns, 0);
    assert_eq!(metrics.short_key_first_page_materialize_ns, 0);
}

#[test]
fn bounded_short_key_request_records_m44_owner_metrics() {
    let _guard = super::m37_metrics_test_guard();
    let translator = StaticTableTranslator::parse_rime_dict_yaml(
        r#"
---
name: short_key_metrics
version: "0.1"
sort: by_weight
...

H	hao	100
H2	hao	90
HA	ha	80
HAO1	haoa	70
HAO2	haob	60
"#,
    )
    .expect("dictionary should parse")
    .with_completion(true)
    .with_sentence(false);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "hao",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(3).with_debug_full_count(true),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(result.candidates[0].text, "H");
    assert!(metrics.short_key_candidate_rows_scanned > 0);
    assert!(metrics.short_key_candidates_materialized > 0);
    assert!(metrics.short_key_candidates_cloned > 0);
    assert!(metrics.short_key_filter_ns > 0);
    assert!(metrics.short_key_sort_rank_ns > 0);
    assert!(metrics.short_key_comment_quality_ns > 0);
    assert!(metrics.short_key_first_page_materialize_ns > 0);
}

#[test]
fn short_luna_key_refresh_uses_first_page_bound_and_completes_on_page_turn() {
    let _guard = super::m37_metrics_test_guard();
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.set_schema("luna_pinyin", "Luna Pinyin");
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: short_key_engine_metrics
version: "0.1"
sort: by_weight
...

H1	hao	100
H2	hao	90
H3	hao	80
H4	hao	70
H5	hao	60
H6	hao	50
H7	hao	40
"#,
        )
        .expect("dictionary should parse")
        .with_completion(false)
        .with_sentence(false),
    );

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    engine
        .process_key_sequence("hao")
        .expect("key sequence should parse");
    let refresh_metrics = crate::m37_metrics_snapshot();

    assert_eq!(engine.context().candidates.len(), 5);
    assert!(!engine.candidate_list_complete());
    assert!(refresh_metrics.candidate_request_bounded_calls > 0);
    assert_eq!(refresh_metrics.candidate_request_surplus_total, 0);
    assert_eq!(
        engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["H1", "H2", "H3", "H4", "H5"]
    );

    crate::m37_metrics_reset();
    assert!(engine.change_page(false));
    let paging_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert!(engine.candidate_list_complete());
    assert!(engine.context().candidates.len() >= 7);
    assert!(paging_metrics.candidate_request_unbounded_calls > 0);
}

#[test]
fn bounded_compact_luna_request_does_not_probe_strict_prefixes_without_prefix_fallback() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: luna_compact_prefix_probe
version: "0.1"
sort: by_weight
...

ZHONGGUO	zhongguo	100
ZHONG	zhong	90
GUO	guo	80
"#,
    )
    .expect("dictionary should parse");
    let syllabary = ["zhong", "guo"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 1, 2))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        .with_sentence(false);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "zhongguo",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(5).with_debug_full_count(true),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(result.candidates[0].text, "ZHONGGUO");
    assert_eq!(
        metrics.prism_lookup_calls, 1,
        "bounded compact Luna requests should expand the requested spelling once, not probe every strict prefix"
    );
}

#[test]
fn bounded_one_scalar_leading_reachability_scans_only_the_requested_window() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::new(
        (0..32).map(|index| TableEntry::new("n", format!("ROW{index:02}"), 100.0 - index as f32)),
    );
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "n",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(5),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(
        result
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["ROW00", "ROW01", "ROW02", "ROW03", "ROW04"]
    );
    assert_eq!(metrics.lookup_views_visited, 5);
    assert_eq!(metrics.exact_lookup_candidates, 5);
}

#[test]
fn bounded_multi_scalar_bare_exact_still_blocks_shorter_leading_family() {
    let translator = StaticTableTranslator::new([("hao", "好"), ("ha", "哈")])
        .with_leading_syllable_reachability(true)
        .with_sentence(false);

    let result = translator.translate_with_context_and_request(
        "hao",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(5),
    );

    assert_eq!(
        result
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["好"]
    );
}

#[test]
fn short_luna_key_refresh_falls_back_when_filter_surplus_underfills_first_page() {
    let _guard = super::m37_metrics_test_guard();
    let mut engine = Engine::new();
    engine.clear_translators();
    engine.set_schema("luna_pinyin", "Luna Pinyin");
    engine.add_filter(DropFirstWindowFilter);
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: short_key_underfill
version: "0.1"
sort: by_weight
...

DROP1	ni	100
DROP2	ni	99
DROP3	ni	98
DROP4	ni	97
DROP5	ni	96
DROP6	ni	95
DROP7	ni	94
A	ni	93
B	ni	92
C	ni	91
D	ni	90
E	ni	89
"#,
        )
        .expect("dictionary should parse")
        .with_completion(false)
        .with_sentence(false),
    );

    engine
        .process_key_sequence("n")
        .expect("key sequence should parse");
    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    engine
        .process_key_sequence("i")
        .expect("key sequence should parse");
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(metrics.candidate_request_bounded_calls, 1);
    assert_eq!(metrics.candidate_request_surplus_total, 2);
    assert_eq!(metrics.candidate_request_unbounded_calls, 1);
    assert!(engine.candidate_list_complete());
    assert_eq!(
        engine
            .context()
            .candidates
            .iter()
            .take(5)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["A", "B", "C", "D", "E"]
    );
}

#[test]
fn bounded_typeduck_profile_request_records_m44_track_b_owner_metrics() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: track_b_metrics
version: "0.1"
sort: by_weight
...

HA	ha	100
HAU	hau	90
HAI	hai	80
"#,
    )
    .expect("dictionary should parse");
    let syllabary = ["ha", "hau", "hai"].map(str::to_owned);
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        .with_dynamic_correction_lookup(true)
        .with_spelling_algebra(&formulas);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "h",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(3).with_debug_full_count(true),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(result.candidates[0].text, "HA");
    assert!(metrics.track_b_spelling_expansions_considered > 0);
    assert!(metrics.track_b_spelling_expansion_ns > 0);
    assert!(metrics.track_b_exact_lookup_calls > 0);
    assert!(
        metrics.track_b_exact_lookup_calls <= 1,
        "short TypeDuck prefix rows should not exact-probe every prism expansion"
    );
    assert!(metrics.track_b_exact_lookup_ns > 0);
    assert!(metrics.track_b_prefix_lookup_calls > 0);
    assert!(metrics.track_b_prefix_lookup_ns > 0);
    assert!(metrics.track_b_candidates_materialized > 0);
    assert!(metrics.track_b_first_page_materialize_ns > 0);
}

#[test]
fn bounded_typeduck_short_prefix_pruning_matches_full_translation_for_target_rows() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: track_b_prefix_parity
version: "0.1"
sort: by_weight
...

NEI	nei	100
NEI2	nei	90
NGO	ngo	100
NGO2	ngo	90
HAI	hai	100
HAU	hau	100
"#,
    )
    .expect("dictionary should parse");
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let table_bytes = build_table_bin(&dictionary, 1);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("Track B table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("Track B compact table should parse");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 1, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("Track B byte-backed prism should parse");
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_completion(true)
            .with_dynamic_correction_lookup(true)
            .with_spelling_algebra(&formulas);

    for input in ["nei", "ngo"] {
        let full = translator.translate(input);
        let bounded = translator.translate_with_context_and_request(
            input,
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(5).with_debug_full_count(true),
        );

        assert_eq!(
            bounded
                .candidates
                .iter()
                .map(|candidate| (candidate.text.as_str(), candidate.comment.as_str()))
                .collect::<Vec<_>>(),
            full.iter()
                .take(bounded.candidates.len())
                .map(|candidate| (candidate.text.as_str(), candidate.comment.as_str()))
                .collect::<Vec<_>>(),
            "bounded Track B short-prefix pruning must preserve full translation order for {input}"
        );
    }
}

#[test]
fn bounded_long_prefix_fallback_uses_the_requested_per_fetch_window() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: long_prefix_cap
version: "0.1"
sort: by_weight
...

FULL	abcdefghij	100
AB1	ab	90
AB2	ab	89
AB3	ab	88
"#,
    )
    .expect("dictionary should parse");
    let syllabary = ["ab", "cd", "ef", "gh", "ij"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 1, 5))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        .with_prefix_fallback(true)
        .with_sentence(false);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "abcdefghij",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(8).with_debug_full_count(true),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    let prefix_candidates = result
        .candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.source,
                CandidateSource::PartialTable {
                    recompose_on_default: true,
                    ..
                }
            )
        })
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        metrics.prism_lookup_calls, 1,
        "bounded prefix fallback should use the authoritative prism descriptors without a redundant sentence-alias lookup per strict prefix"
    );
    assert_eq!(
        prefix_candidates,
        ["AB1", "AB2", "AB3"],
        "a requested eight-row page must not retain an obsolete two-row per-fetch cap"
    );
    assert!(
        result.is_complete,
        "the bounded collector may prove completion when every family exhausts below K"
    );
    assert_eq!(
        result.full_count,
        Some(result.candidates.len()),
        "debug full_count must report the proven complete family"
    );

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let full = translator.translate("abcdefghij");
    let full_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(
        full_metrics.prism_lookup_calls, 1,
        "full prefix fallback should use the authoritative prism descriptors without the former lossy sentence-alias augmentation while AB1/AB2 remain unchanged"
    );
    assert!(
        full.iter().any(|candidate| candidate.text == "AB3"),
        "the unbounded page-turn path must retain rows omitted by the bounded cap"
    );
}

fn double_pinyin_shaped_algebra() -> Vec<String> {
    [
        "xform/^zh/V/",
        "xform/i?ong$/S/",
        "xform/(.)ao$/$1K/",
        "xform/^ha$/h/",
        "xlit/KVS/kvs/",
    ]
    .map(str::to_owned)
    .to_vec()
}

fn double_pinyin_reachability_dictionary() -> TableDictionary {
    TableDictionary::new([
        TableEntry::new("hao", "\u{597d}", 100.0),
        TableEntry::new("ha", "\u{54c8}", 90.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
        TableEntry::new("zhong", "\u{4e2d}", 70.0),
    ])
}

fn assert_surface_leading_single(
    translator: &StaticTableTranslator,
    input: &str,
    expected_text: &str,
    expected_raw_code: &str,
    expected_consumed: usize,
) {
    let result = translator.translate_with_context_and_request(
        input,
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(8).with_debug_full_count(true),
    );
    let leading = result
        .candidates
        .iter()
        .find(|candidate| candidate.text == expected_text)
        .unwrap_or_else(|| {
            panic!(
                "surface input {input:?} must reach {expected_text:?}; got {:?}",
                result
                    .candidates
                    .iter()
                    .map(|candidate| (&candidate.text, &candidate.comment, &candidate.source))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(
        leading.comment, expected_raw_code,
        "surface admission must not replace canonical raw-code metadata"
    );
    assert_eq!(
        leading.source,
        CandidateSource::PartialTable {
            consumed: expected_consumed,
            recompose_on_default: true,
        },
        "the partial candidate must consume the deployed surface spelling, not the canonical fetch code"
    );
}

#[test]
fn transformed_double_pinyin_surface_reaches_canonical_row_on_compact_and_heap_paths() {
    // Structural replay of the pinned diagnostic spelling sequence: hao -> hk,
    // ni -> ni, zhong -> vs. The test deliberately asserts only the first
    // surface->canonical reachability edge; whole-sentence text belongs to the
    // external oracle capture, not an invented engine golden.
    let formulas = double_pinyin_shaped_algebra();
    let syllabary = ["hao", "ha", "ni", "zhong"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("double-pinyin-shaped prism should parse");
    assert_eq!(
        prism
            .lookup_canonical_codes("hk", &syllabary)
            .into_iter()
            .map(|lookup| lookup.code)
            .collect::<Vec<_>>(),
        ["hao"],
        "the test algebra must deploy the diagnostic hao -> hk surface edge"
    );
    let compact = StaticTableTranslator::from_compact_dictionary(
        double_pinyin_reachability_dictionary(),
        Some(prism),
    )
    .with_spelling_algebra(&formulas)
    .with_leading_syllable_reachability(true)
    .with_sentence(false);
    let heap = StaticTableTranslator::new([
        ("hao", "\u{597d}"),
        ("ha", "\u{54c8}"),
        ("ni", "\u{4f60}"),
        ("zhong", "\u{4e2d}"),
    ])
    .with_spelling_algebra(&formulas)
    .with_leading_syllable_reachability(true)
    .with_sentence(false);

    for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
        assert_surface_leading_single(translator, "hknivs", "\u{597d}", "hao", 2);
        let bare = translator.translate("hk");
        assert_eq!(
            bare.iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["\u{597d}"],
            "{storage}: the alias-aware bare-syllable guard must not inject the shorter h -> ha family"
        );
    }

    let disabled_prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("control prism should parse");
    let disabled = StaticTableTranslator::from_compact_dictionary(
        double_pinyin_reachability_dictionary(),
        Some(disabled_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_leading_syllable_reachability(false)
    .with_sentence(false);
    assert!(
        disabled.translate("hknivs").is_empty(),
        "explicit-false reachability must not inject a transformed leading single"
    );
}

#[test]
fn exact_surface_duplicate_promotes_shorter_prefix_fallback_consumed_span() {
    // Product Jyutping deploys both a toneless exact (`zi2` -> `zi`) and the
    // initial abbreviation (`zi2` -> `z`). The complete merge inserts the `z`
    // family first; when both paths yield 子, the visible first duplicate must
    // retain the longer admitted `zi` span so explicit selection consumes all
    // of the input instead of committing 子 and leaving raw `i` behind.
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let translator = StaticTableTranslator::new([("zi2", "\u{5b50}")])
        .with_prefix_fallback(true)
        .with_leading_syllable_reachability(true)
        .with_spelling_algebra(&formulas)
        .with_sentence(false);
    let mut eager = translator.translate("zi");
    let mut bounded = translator
        .translate_with_context_and_request(
            "zi",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(1),
        )
        .candidates;
    UniquifierFilter.apply(&mut eager);
    UniquifierFilter.apply(&mut bounded);
    assert_eq!(bounded, eager[..bounded.len()]);
    assert_eq!(
        bounded[0].source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: false,
        },
        "a full bounded page must apply the same duplicate span promotion as the eager list"
    );

    let mut engine = Engine::new();
    engine.add_translator(translator);
    engine.process_sequence("zi");

    let target_index = engine
        .context()
        .candidates
        .iter()
        .position(|candidate| candidate.text == "\u{5b50}")
        .expect("the product-shaped exact/abbreviation collision should expose 子");
    assert_eq!(
        engine.context().candidates[target_index].source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: false,
        },
        "the earlier abbreviation row must inherit the admitted full-exact surface span"
    );
    assert_eq!(
        engine.select_candidate(target_index).as_deref(),
        Some("\u{5b50}")
    );
    assert!(
        engine.context().composition.input.is_empty(),
        "selecting 子 for exact surface `zi` must not leave a raw `i` remainder"
    );
}

#[test]
fn exact_surface_duplicate_span_promotion_uses_raw_code_not_display_comment() {
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let build = |show_full_code, comment_format: &[String]| {
        StaticTableTranslator::new([("zi2", "CHILD")])
            .with_prefix_fallback(true)
            .with_leading_syllable_reachability(true)
            .with_spelling_algebra(&formulas)
            .with_show_full_code(show_full_code)
            .with_comment_format(comment_format)
            .with_sentence(false)
    };

    for (label, translator) in [
        ("hidden", build(false, &[])),
        (
            "formatted",
            build(true, &["xform/^zi2$/DISPLAY-ONLY/".to_owned()]),
        ),
    ] {
        let mut eager = translator.translate("zi");
        let mut bounded = translator
            .translate_with_context_and_request(
                "zi",
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(1),
            )
            .candidates;
        UniquifierFilter.apply(&mut eager);
        UniquifierFilter.apply(&mut bounded);

        assert_eq!(bounded, eager[..1], "{label}");
        assert_eq!(
            bounded[0].source,
            CandidateSource::PartialTable {
                consumed: 2,
                recompose_on_default: false,
            },
            "{label}: span promotion must use the deployed raw `zi2` surface provenance even when the visible comment cannot encode it"
        );
        if label == "hidden" {
            assert!(bounded[0].comment.is_empty());
        } else {
            assert_eq!(bounded[0].comment, "DISPLAY-ONLY");
        }
    }
}

#[test]
fn cross_translator_duplicate_promotes_longest_surface_span_but_not_abbreviation_only() {
    let abbreviated = StaticTableTranslator::new([("si6", "\u{662f}")])
        .with_prefix_fallback(true)
        .with_spelling_algebra(&["abbrev/^([a-z]).+$/$1/".to_owned()])
        .with_sentence(false);
    let toneless = StaticTableTranslator::new([("si6", "\u{662f}")])
        .with_prefix_fallback(true)
        .with_spelling_algebra(&["derive/\\d//".to_owned()])
        .with_sentence(false);
    let mut merged = Engine::new();
    merged.add_translator(abbreviated);
    merged.add_translator(toneless);
    merged.add_filter(UniquifierFilter);
    merged.process_sequence("sij");
    let target_index = merged
        .context()
        .candidates
        .iter()
        .position(|candidate| candidate.text == "\u{662f}")
        .expect("both deployed surfaces should expose the shared text");
    assert_eq!(
        merged.context().candidates[target_index].source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: true,
        }
    );
    assert_eq!(
        merged.select_candidate(target_index).as_deref(),
        Some("\u{662f}")
    );
    assert_eq!(merged.context().composition.input, "j");

    let abbreviation_only = StaticTableTranslator::new([("si6", "\u{662f}")])
        .with_prefix_fallback(true)
        .with_spelling_algebra(&["abbrev/^([a-z]).+$/$1/".to_owned()])
        .with_sentence(false);
    let mut negative = Engine::new();
    negative.add_translator(abbreviation_only);
    negative.add_filter(UniquifierFilter);
    negative.process_sequence("sij");
    let target_index = negative
        .context()
        .candidates
        .iter()
        .position(|candidate| candidate.text == "\u{662f}")
        .expect("abbreviation-only control should expose the shared text");
    assert_eq!(
        negative.context().candidates[target_index].source,
        CandidateSource::PartialTable {
            consumed: 1,
            recompose_on_default: false,
        },
        "without a longer deployed surface, uniquification must not invent one"
    );

    let short_abbreviation = StaticTableTranslator::new([("si6", "\u{662f}")])
        .with_prefix_fallback(true)
        .with_spelling_algebra(&["abbrev/^([a-z]).+$/$1/".to_owned()])
        .with_sentence(false);
    let long_abbreviation = StaticTableTranslator::new([("si6", "\u{662f}")])
        .with_prefix_fallback(true)
        .with_spelling_algebra(&["abbrev/^si6$/si/".to_owned()])
        .with_sentence(false);
    assert!(long_abbreviation.translate("sij").iter().any(|candidate| {
        candidate.text == "\u{662f}"
            && candidate.source
                == CandidateSource::PartialTable {
                    consumed: 2,
                    recompose_on_default: false,
                }
    }));
    let mut two_abbreviations = Engine::new();
    two_abbreviations.add_translator(short_abbreviation);
    two_abbreviations.add_translator(long_abbreviation);
    two_abbreviations.add_filter(UniquifierFilter);
    two_abbreviations.process_sequence("sij");
    let target = two_abbreviations
        .context()
        .candidates
        .iter()
        .find(|candidate| candidate.text == "\u{662f}")
        .expect("two-abbreviation control should expose the shared text");
    assert_eq!(
        target.source,
        CandidateSource::PartialTable {
            consumed: 1,
            recompose_on_default: false,
        },
        "a longer abbreviation-only duplicate is not non-abbreviation proof and must not promote the earlier selection span"
    );
}

#[test]
fn transformed_bopomofo_surface_preserves_digits_on_byte_backed_and_heap_paths() {
    // Bopomofo keyboard spellings carry tone keys. These deliberately-shaped
    // rules prove that `cl3` remains the consumed surface (3 bytes), while the
    // canonical `hao3` comment remains available to the tone/syllable logic.
    let formulas = [
        "xform/^hao/cl/".to_owned(),
        "xform/^ni/su/".to_owned(),
        "xform/^wan/j0/".to_owned(),
    ];
    let dictionary = TableDictionary::new([
        TableEntry::new("hao3", "\u{597d}", 100.0),
        TableEntry::new("ni3", "\u{4f60}", 90.0),
        TableEntry::new("wan6", "\u{73a9}", 80.0),
    ]);
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("compiled table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed compiled table should parse");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 0x1234_5678, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed transformed prism should parse");
    let byte_backed =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_spelling_algebra(&formulas)
            .with_leading_syllable_reachability(true)
            .with_sentence(false);
    let heap = StaticTableTranslator::new([
        ("hao3", "\u{597d}"),
        ("ni3", "\u{4f60}"),
        ("wan6", "\u{73a9}"),
    ])
    .with_spelling_algebra(&formulas)
    .with_leading_syllable_reachability(true)
    .with_sentence(false);

    for translator in [&byte_backed, &heap] {
        assert_surface_leading_single(translator, "cl3su3j06", "\u{597d}", "hao3", 3);
    }
}

#[test]
fn transformed_bopomofo_upstream_script_honors_leading_single_opt_out() {
    let formulas = [
        "xform/^hao/cl/".to_owned(),
        "xform/^ni/su/".to_owned(),
        "xform/^wan/j0/".to_owned(),
    ];
    let dictionary = TableDictionary::new([
        TableEntry::new("hao3", "\u{597d}", 100.0),
        TableEntry::new("ni3", "\u{4f60}", 90.0),
        TableEntry::new("wan6", "\u{73a9}", 80.0),
        // A one-scalar phrase consuming two syllables is not the independently
        // configurable leading-single family and must remain visible.
        TableEntry::new("hao3ni3", "Q", 1.0),
    ]);
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let prism_bytes = build_prism_bin(&syllabary, &formulas, 0x5904_b001, 0x5904_b002);

    let build = |leading_syllable_reachability: bool, prefix_fallback: bool| {
        let owned_prism = parse_rime_prism_bin_payload(prism_bytes.clone())
            .expect("owned Bopomofo opt-out prism should parse");
        let owned =
            StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(owned_prism))
                .with_spelling_algebra(&formulas)
                .with_sentence(true)
                .with_sentence_policy(SentencePolicy::UpstreamScript)
                .with_prefix_fallback(prefix_fallback)
                .with_leading_syllable_reachability(leading_syllable_reachability)
                .with_upstream_sentence_model(100);

        let table_bytes = build_table_bin(&dictionary, 0x5904_b001);
        let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
            .expect("byte-backed Bopomofo opt-out table metadata should parse");
        let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
            .expect("byte-backed Bopomofo opt-out table should parse");
        let prism_source: Arc<dyn CompactTableByteSource> = Arc::new(AlgebraPrismByteSource(
            Arc::<[u8]>::from(prism_bytes.clone()),
        ));
        let byte_prism = parse_rime_prism_runtime_payload(prism_source)
            .expect("byte-backed Bopomofo opt-out prism should parse");
        let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
            store,
            Some(byte_prism),
        )
        .with_spelling_algebra(&formulas)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_prefix_fallback(prefix_fallback)
        .with_leading_syllable_reachability(leading_syllable_reachability)
        .with_upstream_sentence_model(100);

        [("owned", owned), ("byte-backed", byte_backed)]
    };

    for (storage, translator) in build(false, false) {
        let candidates = translator.translate("cl3su3j06");
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.source == CandidateSource::Sentence),
            "{storage}: explicit false must retain the full sentence; candidates were {candidates:?}"
        );
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.text != "\u{597d}"),
            "{storage}: explicit false must suppress the transformed leading single"
        );
        assert!(
            candidates.iter().any(|candidate| {
                candidate.text == "Q"
                    && matches!(
                        candidate.source,
                        CandidateSource::PartialTable { consumed: 6, .. }
                    )
            }),
            "{storage}: a one-scalar two-syllable phrase must remain visible"
        );
    }

    for (owner, translators) in [
        ("default-on", build(true, false)),
        ("prefix_fallback", build(false, true)),
    ] {
        for (storage, translator) in translators {
            assert!(
                translator.translate("cl3su3j06").iter().any(|candidate| {
                    candidate.text == "\u{597d}"
                        && matches!(
                            candidate.source,
                            CandidateSource::PartialTable { consumed: 3, .. }
                        )
                }),
                "{storage}: {owner} must retain the transformed leading single"
            );
        }
    }
}

#[test]
fn transformed_correction_only_surface_is_not_a_default_reachability_edge() {
    let formulas = ["derive/^hao$/hx/correction".to_owned()];
    let heap = StaticTableTranslator::new([("hao", "\u{597d}"), ("ni", "\u{4f60}")])
        .with_spelling_algebra(&formulas)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);
    let syllabary = ["hao", "ni"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("correction-only compact prism should parse");
    let compact = StaticTableTranslator::from_compact_dictionary(
        TableDictionary::new([
            TableEntry::new("hao", "\u{597d}", 100.0),
            TableEntry::new("ni", "\u{4f60}", 90.0),
        ]),
        Some(prism),
    )
    .with_spelling_algebra(&formulas)
    .with_leading_syllable_reachability(true)
    .with_sentence(false);

    for (storage, translator) in [("heap/source", &heap), ("compact", &compact)] {
        assert!(
            !translator
                .translate("hxni")
                .iter()
                .any(|candidate| candidate.text == "\u{597d}"),
            "{storage}: a correction-only algebra spelling must not become a default-on leading reachability edge"
        );
    }
}

#[test]
fn normal_correction_wins_a_fuzzy_collision_for_compact_and_heap_reachability() {
    let formulas = [
        "fuzz/^hao$/hx/".to_owned(),
        "derive/^hao$/hx/correction".to_owned(),
        "abbrev/^hu$/h/".to_owned(),
    ];
    let syllabary = ["hao", "hu", "ni"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("collision prism should parse");
    let descriptor = prism
        .lookup_canonical_codes("hx", &syllabary)
        .into_iter()
        .next()
        .expect("colliding surface should have one merged descriptor");
    assert!(!descriptor.abbreviation);
    assert!(
        descriptor.correction,
        "pinned Update adopts the normal correction path over the fuzzy non-correction path"
    );

    let dictionary = TableDictionary::new([
        TableEntry::new("hao", "\u{597d}", 100.0),
        TableEntry::new("hu", "\u{56de}", 95.0),
        TableEntry::new("ni", "\u{4f60}", 90.0),
    ]);
    let compact = StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(prism))
        .with_spelling_algebra(&formulas)
        .with_leading_syllable_reachability(true)
        .with_prefix_fallback(true)
        .with_sentence(false);
    let heap = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&formulas)
        .with_leading_syllable_reachability(true)
        .with_prefix_fallback(true)
        .with_sentence(false);

    let mut bare_qualities = Vec::new();
    for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
        let bare = translator.translate("hx");
        assert_eq!(
            bare.iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["\u{597d}"],
            "{storage}: the correction alias remains an ordinary exact spelling"
        );
        bare_qualities.push(bare[0].quality.to_bits());
        assert!(
            !translator
                .translate("hxni")
                .iter()
                .any(|candidate| candidate.text == "\u{597d}"),
            "{storage}: merged correction provenance must remain a guard, not an injectable leading edge"
        );
    }
    assert_eq!(
        bare_qualities[0], bare_qualities[1],
        "compact and heap collision paths must apply the same merged credibility"
    );
}

#[test]
fn spelling_credibility_ranks_in_log_domain_across_raw_and_compiled_storage() {
    // A fuzzy spelling carries credibility ln(0.5). Librime compares
    // ln(dictionary_weight) + credibility, so A's effective weight is 5 and
    // the normal B row at 7 must win. Adding ln(0.5) directly to the raw value
    // would incorrectly keep A (10 - 0.693) ahead on source/legacy storage.
    let formulas = ["fuzz/^aa1$/x/".to_owned(), "derive/^ab1$/x/".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("aa1", "A", 10.0),
        TableEntry::new("ab1", "B", 7.0),
    ]);
    let syllabary = ["aa1", "ab1"].map(str::to_owned);
    let checksum = 0x5904_b411;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_b412);

    let heap = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_spelling_algebra(&formulas)
        .with_sentence(false);
    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned credibility-domain prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(false);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("compiled credibility-domain metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("compiled credibility-domain table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("compiled credibility-domain prism should parse");
    let compiled = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(false);

    let mut quality_rows = Vec::new();
    for (storage, translator) in [("heap", heap), ("owned", owned), ("compiled", compiled)] {
        let candidates = translator.translate("x");
        assert_eq!(
            candidates
                .iter()
                .take(2)
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["B", "A"],
            "{storage}: credibility must multiply dictionary weight in the compiled log domain"
        );
        assert!(
            candidates[0].quality > candidates[1].quality,
            "{storage}: exported quality must preserve the same order"
        );
        let expected = [7.0_f32.exp(), 5.0_f32.exp()];
        for (candidate, expected) in candidates.iter().take(2).zip(expected) {
            assert!(
                (candidate.quality - expected).abs() <= expected * 0.000_01,
                "{storage}: exported quality {} must stay in the shared legacy namespace near {expected}",
                candidate.quality
            );
        }
        quality_rows.push([candidates[0].quality, candidates[1].quality]);
    }
    for row in quality_rows.iter().skip(1) {
        for (&quality, &baseline) in row.iter().zip(&quality_rows[0]) {
            assert!(
                (quality - baseline).abs() <= baseline * 0.000_01,
                "heap/compact/compiled credibility qualities must share one merge namespace"
            );
        }
    }
}

#[test]
fn raw_and_compiled_zero_weight_rows_stay_below_positive_one() {
    let dictionary = TableDictionary::new([
        TableEntry::new("yi", "ZERO", 0.0),
        TableEntry::new("yi", "ONE", 1.0),
    ]);
    let heap = StaticTableTranslator::from_dictionary(dictionary.clone()).with_sentence(false);
    let owned = StaticTableTranslator::from_compact_dictionary(dictionary.clone(), None)
        .with_sentence(false);
    let table_bytes = build_table_bin(&dictionary, 0x5904_b431);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("compiled zero-weight ordering metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("compiled zero-weight ordering table should parse");
    let compiled =
        StaticTableTranslator::from_compact_table_store(store, None).with_sentence(false);

    for (storage, translator) in [("heap", heap), ("owned", owned), ("compiled", compiled)] {
        let candidates = translator.translate("yi");
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["ONE", "ZERO"],
            "{storage}: an unweighted row must not tie a positive weight of one"
        );
        assert!(candidates[0].quality > candidates[1].quality, "{storage}");
    }
}

#[test]
fn fuzzy_spelling_wins_an_abbreviation_collision_across_storage_paths() {
    let formulas = ["fuzz/^hao$/hx/".to_owned(), "abbrev/^hao$/hx/".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("hao", "GOOD", 100.0),
        TableEntry::new("ni", "YOU", 90.0),
    ]);
    let syllabary = ["hao", "ni"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("fuzzy/abbreviation collision prism should parse");
    let descriptor = prism
        .lookup_canonical_codes("hx", &syllabary)
        .into_iter()
        .find(|descriptor| descriptor.code == "hao")
        .expect("colliding spelling should resolve to hao");
    assert!(
        !descriptor.abbreviation,
        "pinned spelling update keeps the fuzzy path over an abbreviation to the same code"
    );

    let compact = StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(prism))
        .with_spelling_algebra(&formulas)
        .with_prefix_fallback(true)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);
    let heap = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&formulas)
        .with_prefix_fallback(true)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);

    for (storage, translator) in [("compact", compact), ("heap/source", heap)] {
        let candidate = translator
            .translate("hxni")
            .into_iter()
            .find(|candidate| candidate.text == "GOOD")
            .unwrap_or_else(|| panic!("{storage}: fuzzy winner must remain an injectable prefix"));
        assert_eq!(
            candidate.source,
            CandidateSource::PartialTable {
                consumed: 2,
                recompose_on_default: true,
            },
            "{storage}: abbreviation provenance must not leak through the fuzzy winner"
        );
    }
}

#[test]
fn heap_source_mixed_abbreviations_are_not_limited_to_initial_m_or_two_syllables() {
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let translator =
        StaticTableTranslator::from_dictionary(TableDictionary::new([TableEntry::new(
            "dak1mai5si6",
            "DMS",
            100.0,
        )]))
        .with_spelling_algebra(&formulas)
        .with_sentence(false);

    for input in ["dmaisi", "dakmsi", "dakmais"] {
        assert_eq!(
            translator.translate(input)[0].text,
            "DMS",
            "heap/source must admit one abbreviated edge at any position of an arbitrary three-syllable code ({input})"
        );
    }
}

#[test]
fn pinned_correction_surface_order_matches_heap_owned_and_byte_backed_paths() {
    let oracle: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/upstream-1.17.0/m59-correction-spelling.json"
    ))
    .expect("curated correction oracle fixture should parse");
    assert_eq!(
        oracle["source_capture"]["sha256"],
        "f5e1cf58cca162c03eadf71473b80376e440c044958ead3ca25e27e36565eee9"
    );
    assert_eq!(
        oracle["source_capture"]["evidence_manifest_sha256"],
        "4cb688f0624a7c19dd7a35b506aec0f30419f62a4eee0f93911d8caf7c6dcf48"
    );
    for option in ["enable_completion", "enable_sentence", "enable_correction"] {
        assert_eq!(oracle["schema"][option], false);
    }
    let expected = oracle["candidates"]
        .as_array()
        .expect("oracle candidates should be an array")
        .iter()
        .map(|candidate| {
            (
                candidate["text"]
                    .as_str()
                    .expect("candidate text")
                    .to_owned(),
                candidate["comment"]
                    .as_str()
                    .expect("candidate comment")
                    .to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        expected,
        [
            ("\u{7c97}".to_owned(), "cu".to_owned()),
            ("\u{932f}".to_owned(), "cuo".to_owned()),
        ]
    );

    let formulas = [
        "abbrev/^chang$/c/".to_owned(),
        "derive/^cuo$/cu/correction".to_owned(),
    ];
    let dictionary = TableDictionary::new([
        TableEntry::new("cu", "\u{7c97}", 0.0),
        TableEntry::new("cuo", "\u{932f}", 0.0),
        TableEntry::new("chang", "\u{9577}", 0.0),
    ]);
    let syllabary = ["cu", "cuo", "chang"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("guard-only correction prism should parse");
    let heap = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_leading_syllable_reachability(true)
        .with_spelling_algebra(&formulas)
        .with_completion(false)
        .with_correction(false)
        .with_sentence(false);
    let compact = StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(prism))
        .with_leading_syllable_reachability(true)
        .with_spelling_algebra(&formulas)
        .with_completion(false)
        .with_correction(false)
        .with_sentence(false);
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("correction oracle table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("correction oracle table should load byte-backed");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 0x1234_5678, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("correction oracle prism should load byte-backed");
    let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_leading_syllable_reachability(true)
    .with_spelling_algebra(&formulas)
    .with_completion(false)
    .with_correction(false)
    .with_sentence(false);

    let mut quality_bits = Vec::new();
    for (storage, translator) in [
        ("heap/source", &heap),
        ("owned compact", &compact),
        ("byte-backed compact", &byte_backed),
    ] {
        let candidates = translator.translate("cu");
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| (candidate.text.clone(), candidate.comment.clone()))
                .collect::<Vec<_>>(),
            expected,
            "{storage}: deployed correction admission must match the complete pinned upstream order"
        );
        assert!(
            candidates[0].quality > candidates[1].quality,
            "{storage}: equal-weight direct `cu` must outrank correction-penalized `cuo`; candidates were {candidates:?}"
        );
        quality_bits.push(
            candidates
                .iter()
                .map(|candidate| candidate.quality.to_bits())
                .collect::<Vec<_>>(),
        );
        let bounded = translator.translate_with_context_and_request(
            "cu",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(2),
        );
        assert_eq!(
            bounded
                .candidates
                .iter()
                .map(|candidate| (candidate.text.clone(), candidate.comment.clone()))
                .collect::<Vec<_>>(),
            expected,
            "{storage}: bounded/ref lookup must preserve the complete oracle order"
        );
        let concatenated = translator.translate("cuni");
        assert!(
            concatenated
                .iter()
                .any(|candidate| candidate.text == "\u{7c97}"),
            "{storage}: the exact normal cu edge must remain reachable in a longer input"
        );
        assert!(
            !concatenated
                .iter()
                .any(|candidate| candidate.text == "\u{932f}"),
            "{storage}: the correction cuo sibling must remain exact-only"
        );
    }
    assert_eq!(
        quality_bits[0], quality_bits[1],
        "the two raw/source storage paths must retain identical correction qualities"
    );
    assert_ne!(
        quality_bits[1], quality_bits[2],
        "Rime::Table/4.0 compiles a non-positive source weight to the pinned ln(DBL_EPSILON) sentinel"
    );
}

#[test]
fn transformed_lookahead_aliases_reach_concatenated_inputs_on_heap_and_byte_backed_paths() {
    let formulas = [
        "derive/^ng(?=[aeiou])//".to_owned(),
        "derive/^n(?!g)/l/".to_owned(),
    ];
    let dictionary = TableDictionary::new([
        TableEntry::new("ngo", "\u{6211}", 100.0),
        TableEntry::new("na", "\u{90a3}", 90.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
    ]);
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("lookahead table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("lookahead byte-backed table should parse");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 0x1234_5678, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("lookahead byte-backed prism should parse");
    let byte_backed =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_leading_syllable_reachability(true)
            .with_spelling_algebra(&formulas)
            .with_sentence(false);
    let heap =
        StaticTableTranslator::new([("ngo", "\u{6211}"), ("na", "\u{90a3}"), ("ni", "\u{4f60}")])
            .with_leading_syllable_reachability(true)
            .with_spelling_algebra(&formulas)
            .with_sentence(false);

    for translator in [&byte_backed, &heap] {
        assert_surface_leading_single(translator, "oni", "\u{6211}", "ngo", 1);
        assert_surface_leading_single(translator, "lani", "\u{90a3}", "na", 2);
    }
}

#[test]
fn transformed_alias_collision_preserves_full_source_order_across_storage_paths() {
    let formulas = ["derive/^ha.$/hx/".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("hai", "\u{6d77}", 100.0),
        TableEntry::new("hao", "\u{597d}", 100.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
    ]);
    let syllabary = ["hai", "hao", "ni"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("collision prism should parse");
    let compact = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_leading_syllable_reachability(true)
        .with_spelling_algebra(&formulas)
        .with_sentence(false);
    let heap =
        StaticTableTranslator::new([("hai", "\u{6d77}"), ("hao", "\u{597d}"), ("ni", "\u{4f60}")])
            .with_leading_syllable_reachability(true)
            .with_spelling_algebra(&formulas)
            .with_sentence(false);

    for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
        assert_eq!(
            translator
                .translate("hxni")
                .into_iter()
                .map(|candidate| candidate.text)
                .collect::<Vec<_>>(),
            ["\u{6d77}", "\u{597d}"],
            "{storage}: colliding surface aliases must retain canonical source order"
        );
    }
}

#[test]
fn mixed_normal_and_correction_surface_keeps_normal_reachability_across_storage_paths() {
    let formulas = [
        "derive/^ha$/hx/".to_owned(),
        "derive/^hao$/hx/correction".to_owned(),
    ];
    let dictionary = TableDictionary::new([
        TableEntry::new("ha", "\u{54c8}", 100.0),
        TableEntry::new("hao", "\u{597d}", 90.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
    ]);
    let syllabary = ["ha", "hao", "ni"].map(str::to_owned);

    let compact_leading = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
                .expect("mixed-surface compact prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_leading_syllable_reachability(true)
    .with_sentence(false);
    let heap_leading = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_spelling_algebra(&formulas)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);
    for (storage, translator) in [
        ("compact leading", &compact_leading),
        ("heap/source leading", &heap_leading),
    ] {
        assert_surface_leading_single(translator, "hxni", "\u{54c8}", "ha", 2);
        assert!(
            !translator
                .translate("hxni")
                .iter()
                .any(|candidate| candidate.text == "\u{597d}"),
            "{storage}: a correction sibling must never become a default leading edge"
        );
    }

    let compact_fallback = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
                .expect("mixed-surface fallback prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_leading_syllable_reachability(true)
    .with_prefix_fallback(true)
    .with_sentence(false);
    let heap_fallback = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&formulas)
        .with_leading_syllable_reachability(true)
        .with_prefix_fallback(true)
        .with_sentence(false);
    for (storage, translator) in [
        ("compact prefix fallback", &compact_fallback),
        ("heap/source prefix fallback", &heap_fallback),
    ] {
        let candidates = translator.translate("hxni");
        let normal = candidates
            .iter()
            .find(|candidate| candidate.text == "\u{54c8}")
            .unwrap_or_else(|| panic!("{storage}: mapped surface hx must admit canonical ha"));
        assert_eq!(
            normal.source,
            CandidateSource::PartialTable {
                consumed: 2,
                recompose_on_default: true,
            },
            "{storage}: prefix fallback must consume the deployed surface span"
        );
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.text == "\u{597d}"),
            "{storage}: prefix fallback must filter the correction-only hao sibling"
        );
    }
}

#[test]
fn exact_normal_surface_with_correction_sibling_keeps_only_normal_reachability() {
    let formulas = ["derive/^hao$/ha/correction".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("ha", "\u{54c8}", 100.0),
        TableEntry::new("hao", "\u{597d}", 90.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
    ]);
    let syllabary = ["ha", "hao", "ni"].map(str::to_owned);

    for prefix_fallback in [false, true] {
        let compact = StaticTableTranslator::from_compact_dictionary(
            dictionary.clone(),
            Some(
                parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
                    .expect("exact-normal mixed compact prism should parse"),
            ),
        )
        .with_spelling_algebra(&formulas)
        .with_leading_syllable_reachability(true)
        .with_prefix_fallback(prefix_fallback)
        .with_sentence(false);
        let heap = StaticTableTranslator::from_dictionary(dictionary.clone())
            .with_spelling_algebra(&formulas)
            .with_leading_syllable_reachability(true)
            .with_prefix_fallback(prefix_fallback)
            .with_sentence(false);
        for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
            let candidates = translator.translate("hani");
            let normal = candidates
                .iter()
                .find(|candidate| candidate.text == "\u{54c8}")
                .unwrap_or_else(|| {
                    panic!(
                        "{storage} fallback={prefix_fallback}: exact normal ha must stay reachable"
                    )
                });
            assert_eq!(
                normal.source,
                CandidateSource::PartialTable {
                    consumed: 2,
                    recompose_on_default: true,
                }
            );
            assert!(
                !candidates
                    .iter()
                    .any(|candidate| candidate.text == "\u{597d}"),
                "{storage} fallback={prefix_fallback}: correction hao must stay exact-only"
            );
        }
    }
}

#[test]
fn direct_normal_prefix_keeps_sibling_normal_algebra_edges_across_storage_paths() {
    let formulas = ["derive/^hao$/ha/".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("ha", "\u{54c8}", 100.0),
        TableEntry::new("hao", "\u{597d}", 90.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
    ]);
    let syllabary = ["ha", "hao", "ni"].map(str::to_owned);
    let compact = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
                .expect("normal-collision compact prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_prefix_fallback(true)
    .with_sentence(false);
    let heap = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&formulas)
        .with_prefix_fallback(true)
        .with_sentence(false);

    for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
        let candidates = translator.translate("hani");
        let normal_family = candidates
            .iter()
            .filter(|candidate| matches!(candidate.text.as_str(), "\u{54c8}" | "\u{597d}"))
            .collect::<Vec<_>>();
        assert_eq!(
            normal_family
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["\u{54c8}", "\u{597d}"],
            "{storage}: the exact ha row and mapped hao sibling must each appear once in source order"
        );
        assert_eq!(
            normal_family
                .iter()
                .map(|candidate| candidate.comment.as_str())
                .collect::<Vec<_>>(),
            ["ha", "hao"],
            "{storage}: surface admission must preserve each canonical raw code"
        );
        assert!(normal_family.iter().all(|candidate| {
            candidate.source
                == CandidateSource::PartialTable {
                    consumed: 2,
                    recompose_on_default: true,
                }
        }));
    }
}

#[test]
fn correction_only_shorter_prefix_preserves_accumulated_longer_normal_prefix() {
    let formulas = ["derive/^hu$/ha/correction".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("hao", "\u{597d}", 100.0),
        TableEntry::new("hu", "\u{56de}", 90.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
    ]);
    let syllabary = ["hao", "hu", "ni"].map(str::to_owned);
    let compact = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
                .expect("correction-boundary compact prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_prefix_fallback(true)
    .with_sentence(false);
    let heap = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&formulas)
        .with_prefix_fallback(true)
        .with_sentence(false);

    for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
        let candidates = translator.translate("haoni");
        let normal = candidates
            .iter()
            .find(|candidate| candidate.text == "\u{597d}")
            .unwrap_or_else(|| panic!("{storage}: longer canonical hao prefix must survive"));
        assert_eq!(normal.comment, "hao");
        assert_eq!(
            normal.source,
            CandidateSource::PartialTable {
                consumed: 3,
                recompose_on_default: true,
            }
        );
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.text == "\u{56de}"),
            "{storage}: the shorter correction-only hu -> ha edge must stay excluded"
        );
    }
}

#[test]
fn transformed_prefix_fallback_resolves_surfaces_without_leading_reachability() {
    let formulas = ["derive/^hao$/hx/".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("hao", "\u{597d}", 100.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
    ]);
    let syllabary = ["hao", "ni"].map(str::to_owned);
    let compact = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
                .expect("prefix-only compact prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_prefix_fallback(true)
    .with_leading_syllable_reachability(false)
    .with_sentence(false);
    let heap = StaticTableTranslator::from_dictionary(dictionary)
        .with_prefix_fallback(true)
        .with_leading_syllable_reachability(false)
        .with_spelling_algebra(&formulas)
        .with_sentence(false);

    for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
        let seed = translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "translator.leading_fetch_seed")
            .expect("leading-fetch seed owner row should exist");
        if storage == "compact" {
            assert_eq!(
                seed.item_count, 0,
                "a current compact prism is the authoritative lazy surface index"
            );
        } else {
            assert!(
                seed.item_count > 0,
                "heap/source prefix fallback alone must retain the algebra surface seed"
            );
        }
        let candidates = translator.translate("hxni");
        let normal_family = candidates
            .iter()
            .filter(|candidate| candidate.text == "\u{597d}")
            .collect::<Vec<_>>();
        assert_eq!(
            normal_family.len(),
            1,
            "{storage}: mapped hao must appear exactly once"
        );
        assert_eq!(normal_family[0].comment, "hao");
        assert_eq!(
            normal_family[0].source,
            CandidateSource::PartialTable {
                consumed: 2,
                recompose_on_default: true,
            }
        );
    }
}

#[test]
fn compiled_transformed_prefix_fallback_follows_full_surface_exact_family() {
    let formulas = ["derive/\\d//".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("ne1", "\u{5462}\u{500b}", 110.0),
        TableEntry::new("nei5", "\u{4f60}", 100.0),
        TableEntry::new("nei1", "\u{5462}", 90.0),
    ]);
    let table_bytes = build_table_bin(&dictionary, 1);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("toned-alias table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("toned-alias compact table should parse");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 1, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("toned-alias byte-backed prism should parse");
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_spelling_algebra(&formulas)
            .with_prefix_fallback(true)
            .with_dynamic_correction_lookup(true)
            .with_sentence(false);

    let candidates = translator.translate("nei");
    assert_eq!(
        candidates
            .iter()
            .take(3)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["\u{4f60}", "\u{5462}", "\u{5462}\u{500b}"],
        "full-input prism aliases must stay ahead of a shorter transformed prefix even when the prefix row has more weight"
    );
    assert_eq!(
        candidates[2].source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: true,
        },
        "the transformed ne -> ne1 row remains reachable with its proper consumed span"
    );

    let bounded = translator.translate_with_context_and_request(
        "nei",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(3),
    );
    assert_eq!(
        bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["\u{4f60}", "\u{5462}", "\u{5462}\u{500b}"],
        "the product bounded path must retain ordinary prism aliases instead of falling through to a sentence"
    );
    assert!(bounded
        .candidates
        .iter()
        .all(|candidate| candidate.source != CandidateSource::Sentence));
}

#[test]
fn transformed_only_phrases_follow_canonical_prefix_reachability() {
    let formulas = ["derive/aa(?=\\d)/a/".to_owned(), "derive/\\d//".to_owned()];
    let entries = || {
        [
            TableEntry::new("zou2hap6ci3", "組合次", 300.0),
            TableEntry::new("zou2hap6", "組合", 200.0),
            // These rows are intentionally heavier than the canonical singles.
            // The admission class, not synthetic test weights, owns page order.
            TableEntry::new("zou6haa5", "做下", 190.0),
            TableEntry::new("zou3haa1", "灶蝦", 180.0),
            TableEntry::new("zou6", "做", 100.0),
            TableEntry::new("zou2", "早", 90.0),
            TableEntry::new("zou2", "組", 80.0),
            TableEntry::new("zou1", "租", 70.0),
        ]
    };
    let translator = compact_prefix_fallback_test_translator(entries(), &formulas)
        .with_completion(true)
        .with_combine_candidates(true);

    for pass in ["cold", "warm"] {
        let first_page = translator.translate_with_context_and_request(
            "zouhapci",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(6),
        );
        assert_eq!(
            first_page
                .candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["組合次", "組合", "做", "早", "組", "租"],
            "{pass} byte-backed path must keep the canonical family on page one"
        );
        assert!(
            !first_page.is_complete,
            "{pass} byte-backed path must retain the deferred phrase family after page one"
        );
    }

    let expanded = translator.translate_with_context_and_request(
        "zouhapci",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(8),
    );
    assert_eq!(
        expanded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["組合次", "組合", "做", "早", "組", "租", "做下", "灶蝦"],
        "transformed-only phrase rows must remain reachable after the canonical page"
    );
    for candidate in &expanded.candidates[6..] {
        assert_eq!(
            candidate.source,
            CandidateSource::PartialTable {
                consumed: 5,
                recompose_on_default: true,
            }
        );
    }

    let streaming = translator.translate_with_context_and_request(
        "zouhapci",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(33),
    );
    assert_eq!(
        streaming
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["組合次", "組合", "做", "早", "組", "租", "做下", "灶蝦"],
        "the cache-bypass streaming path must preserve the same transformed-phrase tiers"
    );

    let heap = StaticTableTranslator::from_dictionary(TableDictionary::new(entries()))
        .with_spelling_algebra(&formulas)
        .with_completion(true)
        .with_prefix_fallback(true)
        .with_combine_candidates(true)
        .with_sentence(false);
    for (storage, translator) in [("byte-backed compact", &translator), ("heap/source", &heap)] {
        assert_eq!(
            translator
                .translate("zouhapci")
                .iter()
                .take(8)
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["組合次", "組合", "做", "早", "組", "租", "做下", "灶蝦"],
            "{storage} eager order must match the bounded transformed-phrase tiers"
        );
    }
}

#[test]
fn bounded_transformed_phrase_tier_reserves_later_single_reachability() {
    let formulas = ["derive/^.+$/x/".to_owned()];
    for (path, deferred_rows, expect_cached) in
        [("cached", 4usize, true), ("streaming", 65usize, false)]
    {
        let mut entries = (0..deferred_rows)
            .map(|index| {
                TableEntry::new(
                    format!("a{index:03}"),
                    format!("PHRASE-{index:03}"),
                    (deferred_rows - index) as f32 + 10.0,
                )
            })
            .collect::<Vec<_>>();
        if path == "streaming" {
            entries.extend([
                TableEntry::new("zlate", "LATE-PHRASE-A", 9.0),
                TableEntry::new("zlate", "LATE-PHRASE-B", 8.0),
                TableEntry::new("zlate", "LATE-PHRASE-C", 7.0),
            ]);
        }
        entries.push(TableEntry::new("zsingle", "S", 1.0));
        let translator = compact_prefix_fallback_test_translator(entries, &formulas);

        let bounded = translator.translate_with_context_and_request(
            "xy",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(1),
        );
        assert_eq!(
            bounded
                .candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["S"],
            "{path}: a full deferred pending window must reserve the later transformed single"
        );
        assert!(
            !bounded.is_complete,
            "{path}: deferred phrases remain reachable"
        );
        assert_eq!(
            prefix_fallback_cache_owner_snapshot(&translator).1 > 0,
            expect_cached,
            "{path}: the fixture must exercise its declared collector"
        );
        assert_eq!(
            translator.translate("xy")[0].text,
            "S",
            "{path}: bounded and eager ranking must agree"
        );
    }
}

#[test]
fn bounded_transformed_phrase_tier_keeps_a_later_single_head_blocked_within_fetch() {
    let formulas = ["derive/^same$/x/".to_owned()];
    let translator = compact_prefix_fallback_test_translator(
        [
            TableEntry::new("same", "PHRASE-A", 40.0),
            TableEntry::new("same", "PHRASE-B", 30.0),
            TableEntry::new("same", "PHRASE-C", 20.0),
            TableEntry::new("same", "S", 1.0),
        ],
        &formulas,
    );

    let bounded = translator.translate_with_context_and_request(
        "xy",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(1),
    );
    assert_eq!(
        bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["PHRASE-A"],
        "an accepted transformed phrase remains the accessor head; a later single cannot leap it"
    );
    assert!(!bounded.is_complete, "deferred phrases remain reachable");
    assert_eq!(bounded.candidates, translator.translate("xy")[..1]);
}

#[test]
fn compiled_transformed_exact_completion_and_prefix_order_matches_after_uniquifier() {
    // The raw `nei` lookup emits completions before the later prism alias emits
    // its full exact. The eager path must apply the same exact/completion
    // category order as the bounded path before the proper-prefix family is
    // merged, without assuming that exacts originally formed a contiguous head.
    let formulas = ["derive/\\d//".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("ne1", "PREFIX", 110.0),
        TableEntry::new("nei1", "EXACT", 100.0),
        TableEntry::new("neix", "COMPLETION", 90.0),
    ]);
    let table_bytes = build_table_bin(&dictionary, 1);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("interleaved-order table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("interleaved-order compact table should parse");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 1, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("interleaved-order byte-backed prism should parse");
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_spelling_algebra(&formulas)
            .with_completion(true)
            .with_prefix_fallback(true)
            .with_dynamic_correction_lookup(true)
            .with_sentence(false);

    let mut eager = translator.translate("nei");
    assert!(eager.iter().any(|candidate| {
        candidate.text == "EXACT" && candidate.source == CandidateSource::Table
    }));
    assert!(eager.iter().any(|candidate| {
        candidate.text == "EXACT" && candidate.source == CandidateSource::Completion
    }));
    let mut bounded = translator
        .translate_with_context_and_request(
            "nei",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(4),
        )
        .candidates;
    UniquifierFilter.apply(&mut eager);
    UniquifierFilter.apply(&mut bounded);

    let signature = |candidates: &[Candidate]| {
        candidates
            .iter()
            .map(|candidate| (candidate.text.clone(), candidate.source.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(signature(&bounded), signature(&eager));
    assert_eq!(
        eager
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["EXACT", "PREFIX", "COMPLETION"]
    );
    assert_eq!(eager[0].source, CandidateSource::Table);
    assert_eq!(
        eager[1].source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: true,
        }
    );
}

#[test]
fn full_prediction_stays_ahead_of_prefix_family_independent_of_comment_display() {
    let build = |show_full_code, combine_candidates| {
        StaticTableTranslator::from_dictionary(TableDictionary::new([
            TableEntry::new("si6gin2", "EVENT", 200.0),
            TableEntry::new("si5gin3guk2", "PRED", 141.0),
            TableEntry::new("si4", "PREFIX", 100.0),
        ]))
        .with_completion(true)
        .with_prediction_candidate_limit(1)
        .with_prefix_fallback(true)
        .with_spelling_algebra(&["derive/\\d//".to_owned()])
        .with_combine_candidates(combine_candidates)
        .with_show_full_code(show_full_code)
        .with_sentence(false)
    };

    for (show_full_code, combine_candidates) in
        [(true, false), (false, false), (true, true), (false, true)]
    {
        let translator = build(show_full_code, combine_candidates);
        let eager = translator.translate("sigin");
        assert_eq!(
            eager
                .iter()
                .take(3)
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["EVENT", "PRED", "PREFIX"],
            "eager order must not depend on show_full_code={show_full_code} or combine_candidates={combine_candidates}"
        );

        let bounded = translator.translate_with_context_and_request(
            "sigin",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(3),
        );
        assert_eq!(
            bounded
                .candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["EVENT", "PRED", "PREFIX"],
            "bounded order must not depend on show_full_code={show_full_code} or combine_candidates={combine_candidates}"
        );
    }
}

#[test]
fn combined_duplicate_full_rows_remap_the_prefix_insertion_anchor() {
    for show_full_code in [true, false] {
        let translator = StaticTableTranslator::from_dictionary(TableDictionary::new([
            TableEntry::new("si6gin2", "EVENT", 200.0),
            TableEntry::new("si5gin3", "EVENT", 190.0),
            TableEntry::new("si5gin3guk2", "PRED", 141.0),
            TableEntry::new("si4", "PREFIX", 100.0),
        ]))
        .with_completion(true)
        .with_prediction_candidate_limit(1)
        .with_prefix_fallback(true)
        .with_spelling_algebra(&["derive/\\d//".to_owned()])
        .with_combine_candidates(true)
        .with_show_full_code(show_full_code)
        .with_sentence(false);

        let eager = translator.translate("sigin");
        assert_eq!(
            eager
                .iter()
                .take(3)
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["EVENT", "PRED", "PREFIX"],
            "eager duplicate collapse must remap the full-input anchor with show_full_code={show_full_code}"
        );

        let bounded = translator.translate_with_context_and_request(
            "sigin",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(3),
        );
        assert_eq!(
            bounded
                .candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["EVENT", "PRED", "PREFIX"],
            "bounded duplicate collapse must remap the full-input anchor with show_full_code={show_full_code}"
        );
    }
}

#[test]
fn one_scalar_prefix_fallback_owns_completion_only_input_bounded_and_eager() {
    let translator = StaticTableTranslator::new([("n", "N"), ("nxyzmore", "COMPLETION")])
        .with_completion(true)
        .with_prefix_fallback(true)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);

    let eager = translator.translate("nxyz");
    let bounded = translator
        .translate_with_context_and_request(
            "nxyz",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(4),
        )
        .candidates;
    let exact_fill = translator.translate_with_context_and_request(
        "nxyz",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(2),
    );
    let signature = |candidates: &[Candidate]| {
        candidates
            .iter()
            .map(|candidate| (candidate.text.clone(), candidate.source.clone()))
            .collect::<Vec<_>>()
    };

    assert_eq!(signature(&bounded), signature(&eager));
    assert_eq!(exact_fill.candidates.len(), 2);
    assert!(
        exact_fill.is_complete,
        "an exhaustive fallback batch that exactly fills the page has no hidden row"
    );
    assert_eq!(
        eager
            .iter()
            .filter(|candidate| candidate.text == "N")
            .count(),
        1,
        "prefix fallback must own the deployed one-scalar prefix instead of also running the leading merge"
    );
    let prefix = eager
        .iter()
        .find(|candidate| candidate.text == "N")
        .expect("the deployed n prefix should remain reachable");
    assert_eq!(
        prefix.source,
        CandidateSource::PartialTable {
            consumed: 1,
            recompose_on_default: false,
        }
    );
    assert!(eager.iter().any(|candidate| {
        candidate.text == "COMPLETION" && candidate.source == CandidateSource::Completion
    }));
    assert!(
        eager
            .iter()
            .all(|candidate| candidate.source != CandidateSource::Table),
        "nxyz intentionally has no full exact"
    );
}

#[test]
fn transformed_prefix_fallback_reaches_multi_character_rows_across_storage_paths() {
    let formulas = ["xform/^hao$/hk/".to_owned()];
    let dictionary = TableDictionary::new([TableEntry::new("hao", "\u{4f60}\u{597d}", 100.0)]);
    let syllabary = ["hao".to_owned()];
    let compact = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
                .expect("multi-character prefix compact prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_prefix_fallback(true)
    .with_leading_syllable_reachability(false)
    .with_sentence(false);
    let heap = StaticTableTranslator::from_dictionary(dictionary)
        .with_prefix_fallback(true)
        .with_leading_syllable_reachability(false)
        .with_spelling_algebra(&formulas)
        .with_sentence(false);

    let seed_items = |translator: &StaticTableTranslator| {
        translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "translator.leading_fetch_seed")
            .expect("leading-fetch seed owner row should exist")
            .item_count
    };
    assert_eq!(
        seed_items(&compact),
        0,
        "a current compact prism must resolve phrase surfaces without cloning its syllabary into the heap seed"
    );
    assert!(
        seed_items(&heap) > 0,
        "heap/source prefix fallback needs a canonical seed for phrase-only transformed rows"
    );

    for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
        let candidates = translator.translate("hkfoo");
        let phrase_family = candidates
            .iter()
            .filter(|candidate| candidate.text == "\u{4f60}\u{597d}")
            .collect::<Vec<_>>();
        assert_eq!(
            phrase_family.len(),
            1,
            "{storage}: mapped phrase row must appear exactly once"
        );
        assert_eq!(phrase_family[0].comment, "hao");
        assert_eq!(
            phrase_family[0].source,
            CandidateSource::PartialTable {
                consumed: 2,
                recompose_on_default: true,
            }
        );
    }
}

#[test]
fn compact_abbreviation_prefix_fallback_preserves_non_recomposing_descriptor() {
    let formulas = ["abbrev/^hao$/hx/".to_owned()];
    let dictionary = TableDictionary::new([TableEntry::new("hao", "\u{597d}", 100.0)]);
    let syllabary = ["hao".to_owned()];
    let translator = StaticTableTranslator::from_compact_dictionary(
        dictionary,
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
                .expect("abbreviation prefix compact prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_prefix_fallback(true)
    .with_leading_syllable_reachability(false)
    .with_sentence(false);

    let candidates = translator.translate("hxfoo");
    let abbreviation_family = candidates
        .iter()
        .filter(|candidate| candidate.text == "\u{597d}")
        .collect::<Vec<_>>();
    assert_eq!(
        abbreviation_family.len(),
        1,
        "the compact prism descriptor must not be duplicated by a lossy alias path"
    );
    assert_eq!(abbreviation_family[0].comment, "hao");
    assert_eq!(
        abbreviation_family[0].source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: false,
        },
        "the prism abbreviation bit must suppress default recomposition"
    );
}

#[test]
fn upstream_script_policy_merges_phrase_sentence_and_partial_families_across_storage_paths() {
    let dictionary = TableDictionary::new([
        TableEntry::new("bei1", "A", 100.0),
        TableEntry::new("bei2", "B", 90.0),
        TableEntry::new("ngo5", "N", 100.0),
        TableEntry::new("ng5", "M", 80.0),
        TableEntry::new("be1", "E", 1.0),
        // A prefix row with the same text as the synthesized A + M sentence
        // proves that sentence insertion cannot borrow a partial donor's source.
        TableEntry::new("bei3", "AM", 70.0),
        TableEntry::new("bei2ngo5", "BN", 80.0),
    ]);
    let vocabulary = vec![
        PresetVocabularyEntry::new("AN", 200.0),
        PresetVocabularyEntry::new("BN", 1_000.0),
    ];
    let formulas = [
        "derive/\\d//".to_owned(),
        "xform/1/v/".to_owned(),
        "xform/2/x/".to_owned(),
        "xform/5/xx/".to_owned(),
    ];
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let prism_bytes = build_prism_bin(&syllabary, &formulas, 0x5904_0001, 0x5904_0002);
    let owned_prism =
        parse_rime_prism_bin_payload(prism_bytes.clone()).expect("owned 4a prism should parse");
    let owned =
        StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(owned_prism))
            .with_spelling_algebra(&formulas)
            .with_sentence(true)
            .with_sentence_policy(SentencePolicy::UpstreamScript)
            .with_leading_syllable_reachability(true)
            .with_preset_vocabulary(vocabulary.clone())
            .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, 0x5904_0001);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed 4a table metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed 4a table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> = Arc::new(AlgebraPrismByteSource(
        Arc::<[u8]>::from(prism_bytes.clone()),
    ));
    let runtime_prism =
        parse_rime_prism_runtime_payload(prism_source).expect("byte-backed 4a prism should parse");
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        dictionary.entries().iter().cloned(),
        &vocabulary,
        &vocabulary,
        0x5904_0001,
    )));
    let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_leading_syllable_reachability(true)
    .with_preset_vocabulary(vocabulary.clone())
    .with_upstream_sentence_poet_source(poet_source, 0x5904_0001)
    .with_upstream_sentence_model(100);

    for (storage, translator) in [("owned", owned), ("byte", byte_backed)] {
        let texts = |input: &str| {
            translator
                .translate(input)
                .into_iter()
                .map(|candidate| candidate.text)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            texts("beingo"),
            ["AN", "BN", "A", "B", "AM", "E"],
            "{storage}: dynamic and explicit phrases must share upstream weight order"
        );
        let sentence = translator.translate("being");
        assert_eq!(
            sentence[0].text, "AM",
            "{storage}: one model sentence leads"
        );
        assert_eq!(
            sentence
                .iter()
                .filter(|candidate| candidate.source == CandidateSource::Sentence)
                .count(),
            1,
            "{storage}: no reliable full phrase admits exactly one sentence"
        );
        let toned = texts("beixngoxx");
        assert_eq!(toned[..2], ["BN", "B"]);
        assert!(
            !toned.iter().any(|text| text == "E"),
            "{storage}: an explicit tone spelling must not admit the shorter raw prefix"
        );

        let full = translator.translate("beingo");
        let bounded = translator.translate_with_context_and_request(
            "beingo",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(3).with_debug_full_count(true),
        );
        assert_eq!(
            bounded.candidates,
            full[..bounded.candidates.len()],
            "{storage}: bounded 4a output must be an exact prefix of eager order"
        );
        assert!(!bounded.is_complete);
        assert!(
            bounded
                .full_count
                .is_some_and(|count| count > bounded.candidates.len()),
            "{storage}: truncation must preserve a conservative paging witness"
        );
    }

    let legacy_prism =
        parse_rime_prism_bin_payload(prism_bytes).expect("legacy control prism should parse");
    let legacy = StaticTableTranslator::from_compact_dictionary(dictionary, Some(legacy_prism))
        .with_spelling_algebra(&formulas)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::LegacyFallback)
        .with_preset_vocabulary(vocabulary)
        .with_upstream_sentence_model(100);
    let legacy_texts = legacy
        .translate("beingo")
        .into_iter()
        .map(|candidate| candidate.text)
        .collect::<Vec<_>>();
    assert!(legacy_texts.contains(&"BN".to_owned()));
    assert!(
        !legacy_texts.contains(&"AN".to_owned()),
        "the legacy/TypeDuck policy must not activate the new upstream script-family merge"
    );
}

#[test]
fn upstream_script_sentence_traverses_explicit_multi_span_table_phrases() {
    let dictionary = TableDictionary::new([
        TableEntry::new("a1", "A", 1_000.0),
        TableEntry::new("b1", "B", 1_000.0),
        TableEntry::new("c1", "C", 1_000.0),
        TableEntry::new("b1c1", "YZ", 1_000_000.0),
    ]);
    // The explicit source row suppresses this same-text preset row in
    // ScriptEncoder. Its deliberately tiny weight makes the test depend on the
    // compiled multi-syllable table edge rather than accidental preset reuse.
    let vocabulary = vec![PresetVocabularyEntry::new("YZ", 0.001)];
    let formulas = ["derive/\\d//".to_owned()];
    // Phrase codes are table-trie paths, not prism syllables. Keeping `b1c1`
    // out of the prism prevents a synthetic one-edge spelling from masking the
    // consecutive-span traversal this regression owns.
    let syllabary = ["a1", "b1", "c1"].map(str::to_owned);
    let prism_bytes = build_prism_bin(&syllabary, &formulas, 0x5904_b201, 0x5904_b202);

    let build_translators = |exclude_internal_phrase: bool| {
        let owned_prism = parse_rime_prism_bin_payload(prism_bytes.clone())
            .expect("owned multi-span prism should parse");
        let owned =
            StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(owned_prism))
                .with_spelling_algebra(&formulas)
                .with_sentence(true)
                .with_sentence_policy(SentencePolicy::UpstreamScript)
                .with_preset_vocabulary(vocabulary.clone())
                .with_upstream_sentence_model(100);

        let table_bytes = build_table_bin(&dictionary, 0x5904_b201);
        let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
            .expect("byte-backed multi-span table metadata should parse");
        let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
            .expect("byte-backed multi-span table should parse");
        let prism_source: Arc<dyn CompactTableByteSource> = Arc::new(AlgebraPrismByteSource(
            Arc::<[u8]>::from(prism_bytes.clone()),
        ));
        let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
            .expect("byte-backed multi-span prism should parse");
        let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
            dictionary.entries().iter().cloned(),
            &vocabulary,
            &vocabulary,
            0x5904_b201,
        )));
        let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
            store,
            Some(runtime_prism),
        )
        .with_spelling_algebra(&formulas)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_preset_vocabulary(vocabulary.clone())
        .with_upstream_sentence_poet_source(poet_source, 0x5904_b201)
        .with_upstream_sentence_model(100);

        [("owned", owned), ("byte", byte_backed)].map(|(storage, translator)| {
            let translator = if exclude_internal_phrase {
                translator.with_dictionary_exclude(["YZ"])
            } else {
                translator
            };
            (storage, translator)
        })
    };

    for (storage, translator) in build_translators(false) {
        let candidates = translator.translate("abc");
        assert_eq!(candidates[0].source, CandidateSource::Sentence, "{storage}");
        assert_eq!(
            candidates[0].text, "AYZ",
            "{storage}: the explicit b1c1 table row must remain an internal sentence edge"
        );
        let bounded = translator.translate_with_context_and_request(
            "abc",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(2).with_debug_full_count(true),
        );
        assert_eq!(
            bounded.candidates,
            candidates[..bounded.candidates.len()],
            "{storage}: bounded internal-phrase output must be a field-for-field complete prefix"
        );
        assert!(!bounded.is_complete, "{storage}");
    }

    for (storage, translator) in build_translators(true) {
        let candidates = translator.translate("abc");
        assert_eq!(candidates[0].source, CandidateSource::Sentence, "{storage}");
        assert_eq!(
            candidates[0].text, "ABC",
            "{storage}: dictionary_exclude must still remove the internal phrase edge"
        );
    }
}

#[test]
fn upstream_script_table_tail_matches_exactly_after_three_syllables_across_storage_paths() {
    let entries = vec![
        TableEntry::new("a1b1c1d1e1", "LOW-WEIGHT-FAR-TAIL", 10.0),
        TableEntry::new("a1b1c1f1g1", "HIGH-WEIGHT-TAIL", 20.0),
        TableEntry::new("a1b1c1d1z1", "UNMATCHED-TAIL", 1_000.0),
    ];
    let owned = UpstreamSentenceModel::from_table_entries(entries.clone(), &[], 100);
    let checksum = 0x5904_e031;
    let source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        entries,
        &[],
        &[],
        checksum,
    )));
    let byte = UpstreamSentenceModel::from_poet_bin_source(source, checksum, 100)
        .expect("byte-backed packed-tail model");
    let spans = [
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(0, 1, "a1"), 0.1),
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(1, 2, "b1"), 0.2),
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(2, 3, "c1"), 0.3),
        // `match_extra_code` does not add tail credibility. If Yune keeps
        // accumulating it after librime's three-syllable table-index boundary,
        // this lower-weight family incorrectly jumps the next family.
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(3, 4, "d1"), 100.0),
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(4, 5, "e1"), 100.0),
        // The same packed tail has a farther exact surface match. Librime keeps
        // the farthest successful end rather than emitting both matches.
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(3, 5, "d1"), 100.0),
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(5, 6, "e1"), 100.0),
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(3, 5, "f1"), 0.0),
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(5, 6, "g1"), 0.0),
    ];

    for (storage, model) in [("owned", owned), ("byte", byte)] {
        let complete =
            model.ranked_script_phrase_candidates_for_weighted_code_spans("abcdef", &spans);
        assert_eq!(
            complete
                .iter()
                .map(|ranked| ranked.candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["HIGH-WEIGHT-TAIL", "LOW-WEIGHT-FAR-TAIL"],
            "{storage}: the packed tail must be exact and tail credibility must not change table order"
        );
        assert_eq!(
            complete[1].candidate.source,
            CandidateSource::Table,
            "{storage}: the farthest exact packed-tail match must consume the complete input"
        );

        let bounded = model
            .ranked_script_phrase_candidates_for_weighted_code_spans_with_limit_filtered(
                "abcdef",
                &spans,
                2,
                &|_, _| true,
            );
        assert_eq!(
            bounded, complete,
            "{storage}: K+1 bounded tail matching must equal complete order"
        );
    }
}

#[test]
fn upstream_script_policy_honors_leading_single_opt_out_across_storage_paths() {
    let dictionary = TableDictionary::new([
        TableEntry::new("bei2", "B", 100.0),
        TableEntry::new("ngo5", "N", 100.0),
        TableEntry::new("aa1", "X", 100.0),
        TableEntry::new("bei2ngo5", "Q", 80.0),
        TableEntry::new("bei2ngo5aa1", "QX", 70.0),
    ]);
    let formulas = ["derive/\\d//".to_owned()];
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let prism_bytes = build_prism_bin(&syllabary, &formulas, 0x5904_1001, 0x5904_1002);

    let owned = |leading_syllable_reachability, prefix_fallback| {
        let prism = parse_rime_prism_bin_payload(prism_bytes.clone())
            .expect("owned opt-out prism should parse");
        StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(prism))
            .with_spelling_algebra(&formulas)
            .with_sentence(true)
            .with_sentence_policy(SentencePolicy::UpstreamScript)
            .with_prefix_fallback(prefix_fallback)
            .with_leading_syllable_reachability(leading_syllable_reachability)
            .with_upstream_sentence_model(100)
    };
    let byte_backed = |leading_syllable_reachability, prefix_fallback| {
        let table_bytes = build_table_bin(&dictionary, 0x5904_1001);
        let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
            .expect("byte-backed opt-out table metadata should parse");
        let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
            .expect("byte-backed opt-out table should parse");
        let prism_source: Arc<dyn CompactTableByteSource> = Arc::new(AlgebraPrismByteSource(
            Arc::<[u8]>::from(prism_bytes.clone()),
        ));
        let prism = parse_rime_prism_runtime_payload(prism_source)
            .expect("byte-backed opt-out prism should parse");
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_spelling_algebra(&formulas)
            .with_sentence(true)
            .with_sentence_policy(SentencePolicy::UpstreamScript)
            .with_prefix_fallback(prefix_fallback)
            .with_leading_syllable_reachability(leading_syllable_reachability)
            .with_upstream_sentence_model(100)
    };

    for (storage, disabled, enabled, prefix_owned) in [
        (
            "owned",
            owned(false, false),
            owned(true, false),
            owned(false, true),
        ),
        (
            "byte",
            byte_backed(false, false),
            byte_backed(true, false),
            byte_backed(false, true),
        ),
    ] {
        let disabled = disabled.translate("beingoaa");
        assert!(
            disabled.iter().any(|candidate| candidate.text == "QX"),
            "{storage}: explicit false must retain full-input phrase output"
        );
        assert!(
            disabled.iter().all(|candidate| {
                candidate.text != "B"
                    || !matches!(candidate.source, CandidateSource::PartialTable { .. })
            }),
            "{storage}: explicit false must suppress the injected leading single"
        );
        assert!(
            disabled.iter().any(|candidate| {
                candidate.text == "Q"
                    && matches!(
                        candidate.source,
                        CandidateSource::PartialTable { consumed: 6, .. }
                    )
            }),
            "{storage}: explicit false must retain a one-scalar phrase that consumes multiple syllables"
        );

        assert!(
            enabled.translate("beingoaa").iter().any(|candidate| {
                candidate.text == "B"
                    && matches!(candidate.source, CandidateSource::PartialTable { .. })
            }),
            "{storage}: default-on reachability must retain the leading single"
        );
        assert!(
            prefix_owned.translate("beingoaa").iter().any(|candidate| {
                candidate.text == "B"
                    && matches!(candidate.source, CandidateSource::PartialTable { .. })
            }),
            "{storage}: independent prefix_fallback must retain its leading family"
        );
    }
}

#[test]
fn upstream_script_prefix_collector_excludes_dead_longer_overlap_across_storage_paths() {
    let dictionary = TableDictionary::new([
        TableEntry::new("be1", "B", 100.0),
        TableEntry::new("in1", "I", 100.0),
        TableEntry::new("be1in1", "BI", 100.0),
        TableEntry::new("bei1", "LONG", 1.0),
        TableEntry::new("ngo5", "NGO", 100.0),
        TableEntry::new("hai6", "HAI", 100.0),
        TableEntry::new("haa6", "HAA", 100.0),
        TableEntry::new("go3", "GO", 100.0),
        TableEntry::new("ngo5hai6go3", "FULL", 100.0),
        TableEntry::new("ngo5haa6", "ALT", 90.0),
    ]);
    let formulas = ["derive/aa(?=\\d)/a/".to_owned(), "derive/\\d//".to_owned()];
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let prism_bytes = build_prism_bin(&syllabary, &formulas, 0x5904_1001, 0x5904_1002);
    let owned_prism = parse_rime_prism_bin_payload(prism_bytes.clone())
        .expect("owned inverse-overlap prism should parse");
    let owned =
        StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(owned_prism))
            .with_spelling_algebra(&formulas)
            .with_sentence(true)
            .with_sentence_policy(SentencePolicy::UpstreamScript)
            .with_leading_syllable_reachability(true);

    let table_bytes = build_table_bin(&dictionary, 0x5904_1001);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed inverse-overlap table metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed inverse-overlap table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed inverse-overlap prism should parse");
    let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_leading_syllable_reachability(true);

    for (storage, translator) in [("owned", owned), ("byte", byte_backed)] {
        let candidates = translator.translate("bein");
        assert!(
            candidates.iter().any(|candidate| candidate.text == "BI"),
            "{storage}: the live be + in path must remain admitted"
        );
        assert!(
            !candidates.iter().any(|candidate| candidate.text == "LONG"),
            "{storage}: a disconnected longer bei overlap must not leak from the prefix collector"
        );
        let recomposed = translator.translate("ngohaigo");
        assert!(
            recomposed.iter().any(|candidate| candidate.text == "FULL"),
            "{storage}: the complete ngo + hai + go path must remain admitted"
        );
        assert!(
            recomposed.iter().any(|candidate| candidate.text == "ALT"),
            "{storage}: the shorter ngo + ha recognized prefix must recompose"
        );
    }
}

#[test]
fn upstream_script_policy_admits_complete_abbreviation_graphs_across_storage_paths() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::new([
        TableEntry::new("na1", "N-low", 30.0),
        TableEntry::new("nei5", "N-high", 100.0),
        TableEntry::new("nong4", "N-mid", 50.0),
        TableEntry::new("ngaam1feel", "N-lettered-tail", 1_000.0),
        TableEntry::new("dak1", "D", 20.0),
        TableEntry::new("mai5", "M", 20.0),
        TableEntry::new("si6", "S", 20.0),
        TableEntry::new("dak1mai5si6", "DMS", 500.0),
        TableEntry::new("ngo5", "NGO", 20.0),
        TableEntry::new("hai6", "HAI", 20.0),
        TableEntry::new("gam2", "GAM", 20.0),
        TableEntry::new("ngo5hai6gam2", "NHG", 500.0),
    ]);
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let prism_bytes = build_prism_bin(&syllabary, &formulas, 0x5904_2001, 0x5904_2002);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone()).expect("owned 4b prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, 0x5904_2001);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed 4b table metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed 4b table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism =
        parse_rime_prism_runtime_payload(prism_source).expect("byte-backed 4b prism should parse");
    let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_model(100);

    for (storage, translator) in [("owned", owned), ("byte", byte_backed)] {
        crate::m37_metrics_enable(true);
        crate::m37_metrics_reset();
        let initial_family = translator.translate("n");
        let cold_metrics = crate::m37_metrics_snapshot();
        crate::m37_metrics_enable(false);
        assert_eq!(
            cold_metrics.upstream_sentence_model_vocabulary_entries_considered, 0,
            "{storage}: a one-syllable root cannot match a multi-character vocabulary row"
        );
        assert_eq!(
            initial_family
                .iter()
                .take(3)
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["N-high", "N-mid", "N-low"],
            "{storage}: all same-end abbreviation codes merge by raw dictionary weight"
        );
        assert_eq!(initial_family[0].source, CandidateSource::Table);
        assert!(
            initial_family
                .iter()
                .all(|candidate| candidate.text != "N-lettered-tail"),
            "{storage}: an untoned Latin tail after a toned syllable remains a second syllable"
        );

        let residual = translator.translate("nri");
        assert_eq!(residual[0].text, "N-high", "{storage}");
        assert_eq!(
            residual[0].source,
            CandidateSource::PartialTable {
                consumed: 1,
                recompose_on_default: true,
            },
            "{storage}: the farthest recognized abbreviation prefix must retain the raw tail"
        );

        assert_eq!(
            translator.translate("dakms")[0].text,
            "DMS",
            "{storage}: mixed full spelling plus two arbitrary initials must compose"
        );
        assert_eq!(
            translator.translate("ngohaig")[0].text,
            "NHG",
            "{storage}: a final abbreviation must compose after two normal syllables"
        );
        assert_eq!(
            translator.translate("ngohg")[0].text,
            "NHG",
            "{storage}: multiple abbreviation positions must compose without an input allowlist"
        );

        for input in ["n", "nri", "ngohaig", "ngohg"] {
            let complete = translator.translate(input);
            let bounded = translator.translate_with_context_and_request(
                input,
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(2).with_debug_full_count(true),
            );
            assert_eq!(
                bounded.candidates,
                complete[..bounded.candidates.len()],
                "{storage} {input}: bounded candidates, including qualities, must equal the complete prefix"
            );
            assert!(!bounded.is_complete, "{storage} {input}");
            crate::m37_metrics_enable(true);
            crate::m37_metrics_reset();
            let cached = translator.translate_with_context_and_request(
                input,
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(2).with_debug_full_count(true),
            );
            let cached_metrics = crate::m37_metrics_snapshot();
            crate::m37_metrics_enable(false);
            assert_eq!(cached, bounded, "{storage} {input}: bounded cache hit");
            assert_eq!(
                cached_metrics.upstream_sentence_model_graph_rebuild_calls, 0,
                "{storage} {input}: a cached page window must not rebuild the sentence graph"
            );
            assert!(
                bounded
                    .full_count
                    .is_some_and(|count| count > bounded.candidates.len()),
                "{storage} {input}: bounded work must advertise an on-demand tail"
            );
        }

        let populated_cache = translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "translator.bounded_upstream_script_cache")
            .expect("bounded cache owner row should exist");
        assert!(
            (1..=64).contains(&populated_cache.item_count),
            "{storage}: bounded requests should populate the fixed-capacity page cache"
        );

        translator.clear_ephemeral_runtime_caches();
        let oversized = translator.translate_with_context_and_request(
            "n",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(65).with_debug_full_count(true),
        );
        assert!(!oversized.candidates.is_empty(), "{storage}");
        let oversized_cache = translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "translator.bounded_upstream_script_cache")
            .expect("bounded cache owner row should exist");
        assert_eq!(
            oversized_cache.item_count, 0,
            "{storage}: a request beyond the declared page limit must not be retained"
        );

        let cached_before_reconfiguration = translator.translate_with_context_and_request(
            "n",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(2).with_debug_full_count(true),
        );
        let mut scratch = TranslatorScratch::default();
        scratch.seed_for_test();
        let scratch_hit = translator.translate_with_context_and_request_with_scratch(
            "n",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(2).with_debug_full_count(true),
            &mut scratch,
        );
        assert_eq!(scratch_hit, cached_before_reconfiguration, "{storage}");
        assert!(
            scratch.is_empty(),
            "{storage}: a page-cache hit must clear reusable sentence scratch"
        );

        let reconfigured = translator.with_initial_quality(3.0);
        let reset_cache = reconfigured
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "translator.bounded_upstream_script_cache")
            .expect("bounded cache owner row should exist");
        assert_eq!(
            reset_cache.item_count, 0,
            "{storage}: translator reconfiguration must invalidate bounded results"
        );
        let reconfigured_bounded = reconfigured.translate_with_context_and_request(
            "n",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(2).with_debug_full_count(true),
        );
        assert_ne!(
            reconfigured_bounded.candidates[0].quality,
            cached_before_reconfiguration.candidates[0].quality,
            "{storage}: a warmed pre-reconfiguration quality must not leak"
        );
        let reconfigured_complete = reconfigured.translate("n");
        assert_eq!(
            reconfigured_bounded.candidates,
            reconfigured_complete[..reconfigured_bounded.candidates.len()],
            "{storage}: cache invalidation must preserve bounded/full field equality"
        );
    }
}

#[test]
fn upstream_script_bounded_collector_keeps_explicit_before_equal_generated_rows() {
    let entries = vec![
        TableEntry::new("a1", "A", 1.0),
        TableEntry::new("a1", "U", 1.0),
        TableEntry::new("b1", "B", 1.0),
        TableEntry::new("b1", "V", 1.0),
        TableEntry::new("a1b1", "EXPLICIT", 1.0),
    ];
    let vocabulary = vec![PresetVocabularyEntry::new("UV", 1.0)];
    let owned = UpstreamSentenceModel::from_table_entries(entries.clone(), &vocabulary, 100);
    let checksum = 0x5904_b403;
    let source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        entries,
        &vocabulary,
        &vocabulary,
        checksum,
    )));
    let byte = UpstreamSentenceModel::from_poet_bin_source(source, checksum, 100)
        .expect("byte-backed tie model");
    let spans = [
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(0, 1, "a1"), 0.0),
        WeightedSentenceCodeSpan::new(SentenceCodeSpan::new(1, 2, "b1"), 0.0),
    ];
    for (storage, model) in [("owned", owned), ("byte", byte)] {
        let complete = model.ranked_script_phrase_candidates_for_weighted_code_spans("ab", &spans);
        let bounded = model
            .ranked_script_phrase_candidates_for_weighted_code_spans_with_limit_filtered(
                "ab",
                &spans,
                2,
                &|_, _| true,
            );
        assert_eq!(complete[0].candidate.text, "EXPLICIT", "{storage}");
        assert_eq!(complete[1].candidate.text, "UV", "{storage}");
        assert_eq!(bounded, complete[..2], "{storage}");
    }
}

#[test]
fn upstream_script_bounded_cap_counts_only_eligible_rows() {
    let excluded = (0..8)
        .map(|index| format!("EXCLUDED{index}"))
        .collect::<Vec<_>>();
    let mut entries = vec![TableEntry::new("bb1", "B", 100.0)];
    entries.extend(
        (0..8).map(|index| TableEntry::new("aa1", format!("S{index}"), 500.0 - index as f32)),
    );
    entries.extend(
        excluded
            .iter()
            .enumerate()
            .map(|(index, text)| TableEntry::new("aa1bb1", text, 400.0 - index as f32)),
    );
    entries.extend((0..8).map(|index| {
        TableEntry::new(
            "aa1bb1",
            format!("\u{3400}HIDDEN{index}"),
            300.0 - index as f32,
        )
    }));
    entries.extend(
        (0..3).map(|index| {
            TableEntry::new("aa1bb1", format!("VISIBLE{index}"), 100.0 - index as f32)
        }),
    );
    let dictionary = TableDictionary::new(entries);
    let formulas = vec!["derive/\\d//".to_owned()];
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let prism = parse_rime_prism_bin_payload(build_prism_bin(
        &syllabary,
        &formulas,
        0x5904_b411,
        0x5904_b412,
    ))
    .expect("filtered-head prism");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_spelling_algebra(&formulas)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_charset_filter(true)
        .with_dictionary_exclude(excluded)
        .with_upstream_sentence_model(100);
    let complete = translator.translate_with_context_and_request(
        "aabb",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::unbounded(),
    );
    assert_eq!(
        complete
            .candidates
            .iter()
            .take(3)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["VISIBLE0", "VISIBLE1", "VISIBLE2"]
    );
    let bounded = translator.translate_with_context_and_request(
        "aabb",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(2),
    );
    assert_eq!(bounded.candidates, complete.candidates[..2]);
}

#[test]
fn upstream_sentence_model_is_lazy_and_builder_reconfiguration_resets_it() {
    let entries = vec![
        TableEntry::new("aa", "A", 1.0),
        TableEntry::new("bb", "B", 1.0),
    ];
    let vocabulary = vec![PresetVocabularyEntry::new("AB", 1.0)];
    let checksum = 0x5904_b423;
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        entries.clone(),
        &vocabulary,
        &vocabulary,
        checksum,
    )));
    let octagram_entries = [(encode_octagram_key("AB"), 42)];
    let double_array =
        DartsDoubleArray::build_bytes(&octagram_entries).expect("synthetic grammar should build");
    let mut grammar_bytes = vec![0; 44];
    grammar_bytes[.."Rime::Grammar/1.0".len()].copy_from_slice(b"Rime::Grammar/1.0");
    grammar_bytes[36..40].copy_from_slice(&(double_array.units().len() as u32).to_le_bytes());
    grammar_bytes[40..44].copy_from_slice(&4_i32.to_le_bytes());
    for unit in double_array.units() {
        grammar_bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let grammar = OctagramGrammar::from_bytes(&grammar_bytes, OctagramGrammarConfig::default())
        .expect("synthetic grammar should parse");
    let translator = StaticTableTranslator::new([("aa", "A"), ("bb", "B")])
        .with_sentence(true)
        .with_preset_vocabulary(vocabulary)
        .with_upstream_sentence_poet_source(poet_source, checksum)
        .with_upstream_sentence_grammar(grammar)
        .with_upstream_sentence_model(10);
    let owner = |translator: &StaticTableTranslator| {
        translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "poet.entries_by_code")
            .expect("poet owner row")
    };
    let owners = translator.memory_owner_rows();
    assert_eq!(
        owner(&translator).item_count,
        0,
        "configuration must not build the model"
    );
    assert!(owners.iter().any(|row| {
        row.owner == "translator.upstream_sentence_model_poet_source_recipe"
            && row.item_count == 1
            && row.estimated_bytes > 0
    }));
    assert!(owners.iter().any(|row| {
        row.owner == "translator.upstream_sentence_model_preset_vocabulary_recipe"
            && row.item_count == 1
            && row.estimated_bytes > 0
    }));
    assert_eq!(
        owners
            .iter()
            .filter(|row| row.owner.contains("octagram_double_array"))
            .count(),
        1,
        "the translator-owned grammar recipe must be reported before initialization"
    );
    let _ = translator.translate("aabb");
    assert!(
        owner(&translator).item_count > 0,
        "first translation should initialize the model"
    );
    let initialized_owners = translator.memory_owner_rows();
    assert!(initialized_owners
        .iter()
        .all(|row| { row.owner != "translator.upstream_sentence_model_poet_source_recipe" }));
    assert!(initialized_owners.iter().any(|row| {
        row.owner == "translator.upstream_sentence_model_preset_vocabulary_recipe"
            && row.item_count == 1
            && row.estimated_bytes > 0
    }));
    assert_eq!(
        initialized_owners
            .iter()
            .filter(|row| row.owner.contains("octagram_double_array"))
            .count(),
        2,
        "model initialization clones the translator-owned octagram trie"
    );
    let reconfigured = translator.with_dictionary_exclude(["A"]);
    assert_eq!(
        owner(&reconfigured).item_count,
        0,
        "builder changes must discard the lazy model"
    );
    let reset_owners = reconfigured.memory_owner_rows();
    assert!(reset_owners.iter().any(|row| {
        row.owner == "translator.upstream_sentence_model_poet_source_recipe" && row.item_count == 1
    }));
    assert_eq!(
        reset_owners
            .iter()
            .filter(|row| row.owner.contains("octagram_double_array"))
            .count(),
        1,
        "reset must discard the model clone but retain the translator recipe"
    );
}

#[test]
fn bounded_page_cache_rejects_results_with_more_than_64_rows() {
    let translator = StaticTableTranslator::new([("aa", "A")])
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript);
    let rows = (0..65)
        .map(|index| Candidate {
            text: format!("ROW{index:02}"),
            comment: String::new(),
            preedit: None,
            source: CandidateSource::Table,
            quality: 0.0,
        })
        .collect::<Vec<_>>();
    translator.cache_bounded_result_probe(TranslationResult::complete(rows.clone()));
    translator.cache_bounded_result_probe(TranslationResult {
        candidates: rows,
        is_complete: false,
        full_count: Some(66),
        merge_policy: crate::TranslationMergePolicy::Default,
        merge_qualities: None,
    });
    let owner = translator
        .memory_owner_rows()
        .into_iter()
        .find(|row| row.owner == "translator.bounded_upstream_script_cache")
        .expect("bounded cache owner row");
    assert_eq!(owner.item_count, 0);
}

#[test]
fn upstream_script_uses_librime_lexical_syllable_order_across_storage_paths() {
    // The compact artifact deliberately keeps B before A. Pinned librime does
    // not: EntryCollector builds a std::set syllabary, so Table::Query sees
    // A, B, C, D. The multi-row C/D chunks make that initial order observable
    // after MSVC's repeated one-element partial_sort tournaments.
    let dictionary = TableDictionary::new([
        TableEntry::new("nei5", "B", 50.0),
        TableEntry::new("na1", "A", 50.0),
        TableEntry::new("nong4", "C-high", 60.0),
        TableEntry::new("nong4", "C-tail", 50.0),
        TableEntry::new("nyun4", "D-high", 60.0),
        TableEntry::new("nyun4", "D-tail", 50.0),
    ]);
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let syllabary = ["nei5", "na1", "nong4", "nyun4"].map(str::to_owned);
    let checksum = 0x5904_b403;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_b404);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned lexical-order prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed lexical-order metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed lexical-order table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed lexical-order prism should parse");
    let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript);

    for (storage, translator) in [("owned", owned), ("byte", byte_backed)] {
        assert_eq!(
            translator
                .translate("n")
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["C-high", "D-high", "D-tail", "B", "A", "C-tail"],
            "{storage}: Yune artifact insertion order must not replace librime's lexical syllable traversal"
        );
    }
}

#[test]
fn upstream_script_reconstructed_vocabulary_uses_librime_chunk_tournament() {
    let dictionary = TableDictionary::new([
        TableEntry::new("nei5", "乙", 10.0),
        TableEntry::new("na1", "甲", 10.0),
        TableEntry::new("nong4", "丙", 10.0),
        TableEntry::new("nong4", "丁", 10.0),
        TableEntry::new("nyun4", "戊", 10.0),
        TableEntry::new("nyun4", "己", 10.0),
        TableEntry::new("ngo5", "我", 10.0),
    ]);
    let vocabulary = vec![
        PresetVocabularyEntry::new("我乙", 50.0),
        PresetVocabularyEntry::new("我甲", 50.0),
        PresetVocabularyEntry::new("我丙", 60.0),
        PresetVocabularyEntry::new("我丁", 50.0),
        PresetVocabularyEntry::new("我戊", 60.0),
        PresetVocabularyEntry::new("我己", 50.0),
    ];
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let syllabary = ["nei5", "na1", "nong4", "nyun4", "ngo5"].map(str::to_owned);
    let checksum = 0x5904_b405;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_b406);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned vocabulary-tournament prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_preset_vocabulary(vocabulary.clone())
    .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed vocabulary-tournament metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed vocabulary-tournament table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed vocabulary-tournament prism should parse");
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        dictionary.entries().iter().cloned(),
        &vocabulary,
        &[],
        checksum,
    )));
    let byte_poet = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_preset_vocabulary(vocabulary)
    .with_upstream_sentence_poet_source(poet_source, checksum)
    .with_upstream_sentence_model(100);

    for (storage, translator) in [("owned", owned), ("byte-poet", byte_poet)] {
        assert_eq!(
            translator
                .translate("ngon")
                .iter()
                .take(6)
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["我丙", "我戊", "我己", "我乙", "我甲", "我丁"],
            "{storage}: reconstructed compiler vocabulary must retain DictEntryIterator's mutable chunk residual"
        );
    }
}

#[test]
fn upstream_script_keeps_explicit_source_before_equal_generated_phrase() {
    // EntryCollector reads every explicitly coded row in pass 1, then runs
    // ScriptEncoder over blank-code phrases in pass 2. Its stable homophone
    // sort therefore retains 鈕公 before the equal-weight generated 紐公 row.
    let dictionary = TableDictionary::new([
        TableEntry::new("nau2", "鈕", 10.0),
        TableEntry::new("nau2", "紐", 10.0),
        TableEntry::new("gung1", "公", 10.0),
        TableEntry::new("nau2gung1", "鈕公", 1.0),
        TableEntry::new("", "紐公", 1.0),
    ]);
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let syllabary = ["nau2", "gung1"].map(str::to_owned);
    let checksum = 0x5904_b407;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_b408);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned source-phase prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed source-phase metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed source-phase table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed source-phase prism should parse");
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        dictionary.entries().iter().cloned(),
        &[],
        &[],
        checksum,
    )));
    let byte_poet = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_poet_source(poet_source, checksum)
    .with_upstream_sentence_model(100);

    for (storage, translator) in [("owned", owned), ("byte-poet", byte_poet)] {
        assert_eq!(
            translator
                .translate("ng")
                .iter()
                .filter(|candidate| matches!(candidate.text.as_str(), "鈕公" | "紐公"))
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["鈕公", "紐公"],
            "{storage}: pass-1 explicit rows must precede equal pass-2 ScriptEncoder rows"
        );
    }
}

#[test]
fn upstream_script_transformed_ties_keep_model_traversal_while_identity_keeps_direct_phase() {
    // The canonical M59 `ngohaig` capture exposed this distinction: on a
    // transformed abbreviation graph, equal-weight generated phrases from
    // earlier canonical code families precede a later phrase that also has a
    // direct source row. An identity graph still follows the deployed direct
    // collector phase before generated rows. Keep both storage paths and the
    // bounded page prefix on the same structural policy.
    let dictionary = TableDictionary::new([
        TableEntry::new("al", "A", 1.0),
        TableEntry::new("ba", "B", 1.0),
        TableEntry::new("am", "X", 1.0),
        TableEntry::new("bb", "Y", 1.0),
        TableEntry::new("an", "M", 1.0),
        TableEntry::new("bc", "N", 1.0),
        TableEntry::new("an", "O", 1.0),
        TableEntry::new("bc", "P", 1.0),
        TableEntry::new("an", "Q", 1.0),
        TableEntry::new("bc", "R", 1.0),
        TableEntry::new("anbc", "MN", 100.0),
    ]);
    let vocabulary = vec![
        PresetVocabularyEntry::new("AB", 100.0),
        PresetVocabularyEntry::new("XY", 100.0),
        PresetVocabularyEntry::new("OP", 100.0),
        PresetVocabularyEntry::new("QR", 100.0),
    ];
    let formulas = ["abbrev/^([a-z]).+$/$1/".to_owned()];
    let syllabary = ["al", "ba", "am", "bb", "an", "bc"].map(str::to_owned);
    let checksum = 0x5904_b409;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_b40a);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned transformed-tie prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_preset_vocabulary(vocabulary.clone())
    .with_abbreviation_preset_vocabulary(vocabulary.clone())
    .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed transformed-tie metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed transformed-tie table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed transformed-tie prism should parse");
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        dictionary.entries().iter().cloned(),
        &vocabulary,
        &vocabulary,
        checksum,
    )));
    let byte_poet = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_poet_source(poet_source, checksum)
    .with_upstream_sentence_model(100);

    for (storage, translator) in [("owned", owned), ("byte-poet", byte_poet)] {
        let transformed = translator.translate("ab");
        assert_eq!(
            transformed
                .iter()
                .filter(|candidate| matches!(candidate.text.as_str(), "AB" | "XY" | "MN"))
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["AB", "XY", "MN"],
            "{storage}: transformed equal-weight ties must retain reconstructed model traversal: {transformed:?}"
        );
        let bounded = translator.translate_with_context_and_request(
            "ab",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(5).with_debug_full_count(true),
        );
        assert_eq!(
            bounded.candidates,
            transformed[..bounded.candidates.len()],
            "{storage}: transformed bounded output must be a field-identical complete prefix"
        );

        let identity = translator.translate("anbc");
        assert_eq!(
            identity
                .iter()
                .filter(|candidate| matches!(candidate.text.as_str(), "MN" | "OP" | "QR"))
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["MN", "OP", "QR"],
            "{storage}: identity-normal ties must retain the direct collector phase"
        );
    }
}

#[test]
fn upstream_script_generated_vocabulary_uses_compiled_f32_chunk_weights() {
    let lower = 10.0_f32;
    let higher = f32::from_bits(lower.to_bits() + 1);
    assert!(higher > lower);
    assert_eq!(
        (f64::from(lower).ln() as f32).to_bits(),
        (f64::from(higher).ln() as f32).to_bits(),
        "the fixture must straddle a raw-f32 boundary that collapses in the compiled log-f32 domain"
    );

    let dictionary = TableDictionary::new([
        TableEntry::new("a1", "A", 1.0),
        TableEntry::new("a2", "X", 1.0),
        TableEntry::new("b1", "B", 1.0),
        TableEntry::new("b2", "Y", 1.0),
    ]);
    let vocabulary = vec![
        PresetVocabularyEntry::new("AB", lower),
        PresetVocabularyEntry::new("XY", higher),
    ];
    let formulas = ["derive/\\d//".to_owned()];
    let syllabary = ["a1", "a2", "b1", "b2"].map(str::to_owned);
    let checksum = 0x5904_b409;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_b410);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned f32-vocabulary prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_preset_vocabulary(vocabulary.clone())
    .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed f32-vocabulary metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed f32-vocabulary table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed f32-vocabulary prism should parse");
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        dictionary.entries().iter().cloned(),
        &vocabulary,
        &vocabulary,
        checksum,
    )));
    let byte_poet = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_preset_vocabulary(vocabulary)
    .with_upstream_sentence_poet_source(poet_source, checksum)
    .with_upstream_sentence_model(100);

    for (storage, translator) in [("owned", owned), ("byte-poet", byte_poet)] {
        assert_eq!(
            translator
                .translate("ab")
                .iter()
                .filter(|candidate| matches!(candidate.text.as_str(), "AB" | "XY"))
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["AB", "XY"],
            "{storage}: compiled equal heads retain lexical accessor order instead of raw-f64 promotion"
        );
    }
}

#[test]
fn upstream_script_model_applies_cumulative_spelling_credibility_across_storage_paths() {
    let dictionary = TableDictionary::new([
        TableEntry::new("naa5", "NAA", 10.0),
        TableEntry::new("go3", "GO", 10.0),
        TableEntry::new("hou2", "HOU", 10.0),
        TableEntry::new("ngo5", "NGO", 10.0),
        TableEntry::new("hoeng3", "HOENG", 10.0),
        TableEntry::new("naa5go3hou2", "TWO-ABBREV", 785.0),
        TableEntry::new("ngo5hoeng3", "ONE-ABBREV", 703.0),
    ]);
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    // Phrase codes are table-trie paths, not prism syllables. Keeping them out
    // prevents a synthetic one-edge phrase spelling from masking cumulative
    // credibility across the real n + go + h and ngo + h paths.
    let syllabary = ["naa5", "go3", "hou2", "ngo5", "hoeng3"].map(str::to_owned);
    let checksum = 0x5904_b401;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_b402);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned credibility prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed credibility metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed credibility table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed credibility prism should parse");
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        dictionary.entries().iter().cloned(),
        &[],
        &[],
        checksum,
    )));
    let byte_poet = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_poet_source(poet_source, checksum)
    .with_upstream_sentence_model(100);

    for (storage, translator) in [("owned", owned), ("byte-poet", byte_poet)] {
        let candidates = translator.translate("ngoh");
        let position = |text: &str| {
            candidates
                .iter()
                .position(|candidate| candidate.text == text)
                .unwrap_or_else(|| panic!("{storage}: missing {text}: {candidates:?}"))
        };
        assert!(
            position("ONE-ABBREV") < position("TWO-ABBREV"),
            "{storage}: one abbreviation gives 703 * 0.5, which must outrank two abbreviations at 785 * 0.25"
        );
    }
}

#[test]
fn upstream_script_recovers_late_phrase_only_syllables_across_storage_paths() {
    let _guard = super::m37_metrics_test_guard();
    let mut entries = vec![TableEntry::new("goeng1", "G", 100.0)];
    for index in 0..140usize {
        let first = char::from(b'a' + (index / 26) as u8);
        let second = char::from(b'a' + (index % 26) as u8);
        entries.push(TableEntry::new(
            format!("n{first}{second}1goeng1"),
            format!("decoy-{index:03}"),
            100.0,
        ));
    }
    entries.push(TableEntry::new("nzz1goeng1", "TARGET-ZERO", 0.0));
    let dictionary = TableDictionary::new(entries);
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let prism_bytes = build_prism_bin(&syllabary, &formulas, 0x5904_b301, 0x5904_b302);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned late-abbreviation prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_model(500);

    let table_bytes = build_table_bin(&dictionary, 0x5904_b301);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed late-abbreviation table metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed late-abbreviation table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed late-abbreviation prism should parse");
    let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_model(500);

    for (storage, translator) in [("owned", owned), ("byte", byte_backed)] {
        crate::m37_metrics_enable(true);
        crate::m37_metrics_reset();
        let bounded = translator.translate_with_context_and_request(
            "ng",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(5).with_debug_full_count(true),
        );
        let bounded_metrics = crate::m37_metrics_snapshot();
        crate::m37_metrics_enable(false);

        let candidates = translator.translate("ng");
        assert_eq!(
            bounded.candidates,
            candidates[..bounded.candidates.len()],
            "{storage}: page-bounded late-family rows must be a field-for-field complete prefix"
        );
        assert!(!bounded.is_complete, "{storage}");
        assert!(
            bounded_metrics.upstream_sentence_model_phrase_index_nodes_visited < 2_048,
            "{storage}: bounded typing must not traverse the complete late abbreviation graph: {bounded_metrics:?}"
        );
        assert!(
            bounded_metrics.upstream_sentence_model_graph_edges < 512,
            "{storage}: a cold bounded request must retain bounded graph edges: {bounded_metrics:?}"
        );
        let target = candidates
            .iter()
            .position(|candidate| candidate.text == "TARGET-ZERO")
            .unwrap_or_else(|| panic!("{storage}: phrase-only zero row must remain reachable"));
        assert!(
            target > 128,
            "{storage}: the fixture must prove reachability beyond the retired per-span cap"
        );
        assert_eq!(
            candidates[target].source,
            CandidateSource::Table,
            "{storage}"
        );
    }
}

#[test]
fn compiled_abbreviation_merge_preserves_librime_log_f32_tie_order() {
    let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        "---\nname: precise-abbrev\nversion: '1'\nsort: by_weight\nvocabulary: essay\n...\n\n柟\tnaam1\t5%\n恁\tnam2\t3%\n糅\tnau6\t5%\n",
        std::iter::empty::<&str>(),
        |_| None,
        |name| {
            (name == "essay").then(|| "柟\t519\n恁\t865\n糅\t519\n".to_owned())
        },
    )
    .expect("percentage dictionary should parse");
    let formulas = [
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let table_bytes = build_table_bin(&dictionary, 0x5904_b101);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("compiled table metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("compiled table should parse");
    let prism_bytes = build_prism_bin(&syllabary, &formulas, 0x5904_b101, 0x5904_b102);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("compiled abbreviation prism should parse");
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_spelling_algebra(&formulas)
            .with_sentence(true)
            .with_sentence_policy(SentencePolicy::UpstreamScript);

    for input in ["n", "nri"] {
        let candidates = translator.translate(input);
        assert_eq!(
            candidates
                .iter()
                .take(3)
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["柟", "恁", "糅"],
            "{input} must retain the pinned equal-log traversal order"
        );
    }
    assert_eq!(
        translator.translate("nri")[0].source,
        CandidateSource::PartialTable {
            consumed: 1,
            recompose_on_default: true,
        }
    );
}

#[test]
fn upstream_script_policy_bounds_long_identity_graphs_to_one_prism_walk_per_vertex() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::new(
        ('a'..='z')
            .map(|ch| TableEntry::new(ch.to_string(), ch.to_ascii_uppercase().to_string(), 100.0)),
    );
    let syllabary = ('a'..='z').map(|ch| ch.to_string()).collect::<Vec<_>>();
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 1, 2))
        .expect("identity 4a prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_upstream_sentence_model(10);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let short_sentence = translator.translate("ab");
    let short_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);
    assert_eq!(short_sentence[0].source, CandidateSource::Sentence);
    assert!(
        short_metrics.prism_lookup_calls > 1,
        "short untoned Standard input must enter the deployed SyllableGraph owner"
    );

    for input in [
        "ceshiyixiachangjushuruxingnengzenyang",
        "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
    ] {
        crate::m37_metrics_enable(true);
        crate::m37_metrics_reset();
        let identity_sentence = translator.translate(input);
        let metrics = crate::m37_metrics_snapshot();
        crate::m37_metrics_enable(false);
        assert_eq!(identity_sentence[0].source, CandidateSource::Sentence);
        assert_eq!(
            metrics.prism_lookup_calls,
            input.len() as u64,
            "37/59-byte identity input {input:?} must use one common-prefix prism walk per live vertex, not one exact lookup per substring"
        );
    }
}

#[test]
fn upstream_script_identity_graph_reuses_octagram_scratch_with_cold_weighted_order() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::new([
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("b", "B", 100.0),
        TableEntry::new("c", "C", 100.0),
    ]);
    let syllabary = ["a", "b", "c"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 1, 2))
        .expect("identity Octagram prism should parse");
    let grammar_trie = DartsDoubleArray::build_bytes(&[
        (encode_octagram_key("AB"), 420_000),
        (encode_octagram_key("C$"), 990_000),
    ])
    .expect("identity Octagram grammar trie should build");
    let mut grammar_bytes = vec![0; 44];
    grammar_bytes[.."Rime::Grammar/1.0".len()].copy_from_slice(b"Rime::Grammar/1.0");
    grammar_bytes[36..40].copy_from_slice(&(grammar_trie.units().len() as u32).to_le_bytes());
    grammar_bytes[40..44].copy_from_slice(&4_i32.to_le_bytes());
    for unit in grammar_trie.units() {
        grammar_bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let grammar = OctagramGrammar::from_bytes(&grammar_bytes, OctagramGrammarConfig::default())
        .expect("identity Octagram grammar should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_upstream_sentence_grammar(grammar)
        .with_upstream_sentence_model(10);

    let cold = translator.translate("abc");
    assert_eq!(cold[0].text, "ABC");
    assert_eq!(cold[0].source, CandidateSource::Sentence);

    let mut scratch = TranslatorScratch::default();
    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let _ = translator.translate_with_context_and_request_with_scratch(
        "ab",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(4).with_debug_full_count(true),
        &mut scratch,
    );
    let warm = translator.translate_with_context_and_request_with_scratch(
        "abc",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(4).with_debug_full_count(true),
        &mut scratch,
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(warm.candidates, cold[..warm.candidates.len()]);
    assert_eq!(metrics.upstream_sentence_model_incremental_reuse_hits, 1);
}

#[test]
fn upstream_script_untoned_graph_owns_exact_prefix_and_sentence_surfaces_across_storage_paths() {
    let dictionary = TableDictionary::new([
        // Keep the real prism syllables first so the owned and byte-backed
        // descriptor IDs share the same canonical syllabary positions.
        TableEntry::new("mo", "M", 100.0),
        TableEntry::new("bo", "B", 100.0),
        TableEntry::new("yi", "Y", 100.0),
        TableEntry::new("o", "O", 80.0),
        TableEntry::new("mao", "A", 70.0),
        TableEntry::new("mobo", "EXPLICIT", 500.0),
        TableEntry::new("moboyi", "FULL", 400.0),
        TableEntry::new("maoyi", "ABBREVIATION-OWNED", 300.0),
        TableEntry::new("maooyi", "ABBREVIATION-LEAK", 1_000.0),
        TableEntry::new("yiz", "Y-COMPLETION", 10_000.0),
    ]);
    let vocabulary = vec![PresetVocabularyEntry::new("MB", 450.0)];
    let formulas = ["abbrev/^([a-z]).+$/$1/".to_owned()];
    let syllabary = ["mo", "bo", "yi", "o", "mao"].map(str::to_owned);
    let checksum = 0x5904_e001;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_e002);
    let grammar = || {
        let entries = [(encode_octagram_key("MY"), 42)];
        let double_array =
            DartsDoubleArray::build_bytes(&entries).expect("synthetic untoned grammar");
        let mut bytes = vec![0; 44];
        bytes[.."Rime::Grammar/1.0".len()].copy_from_slice(b"Rime::Grammar/1.0");
        bytes[36..40].copy_from_slice(&(double_array.units().len() as u32).to_le_bytes());
        bytes[40..44].copy_from_slice(&4_i32.to_le_bytes());
        for unit in double_array.units() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        OctagramGrammar::from_bytes(&bytes, OctagramGrammarConfig::default())
            .expect("synthetic untoned grammar should parse")
    };

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned untoned prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_leading_syllable_reachability(true)
    .with_preset_vocabulary(vocabulary.clone())
    .with_upstream_sentence_grammar(grammar())
    .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed untoned table metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed untoned table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed untoned prism should parse");
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        dictionary.entries().iter().cloned(),
        &vocabulary,
        &vocabulary,
        checksum,
    )));
    let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_leading_syllable_reachability(true)
    .with_preset_vocabulary(vocabulary)
    .with_upstream_sentence_poet_source(poet_source, checksum)
    .with_upstream_sentence_grammar(grammar())
    .with_upstream_sentence_model(100);

    let mut head_merge_qualities = Vec::new();
    for (storage, translator) in [("owned", owned), ("byte-backed", byte_backed)] {
        let exact = translator.translate("yi");
        assert_eq!(exact[0].text, "Y", "{storage}");
        assert!(
            exact.iter().all(|candidate| candidate.text != "Y-COMPLETION"),
            "{storage}: a fully parsed exact syllable must not admit ordinary raw prefix completions"
        );
        let merge_result = translator.translate_with_context_and_request(
            "yi",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::unbounded(),
        );
        let merge_qualities = merge_result
            .merge_qualities
            .as_ref()
            .expect("UpstreamScript must retain its outer merge qualities");
        assert_eq!(
            merge_qualities.len(),
            merge_result.candidates.len(),
            "{storage}"
        );
        let head_merge_quality = merge_qualities[0];
        let expected = 100.0_f32 / 100_000_000.0;
        assert!(
            (head_merge_quality - expected).abs() <= expected * 0.000_01,
            "{storage}: normalized ScriptTranslation merge quality was {head_merge_quality}"
        );
        head_merge_qualities.push(head_merge_quality);

        let reliable = translator.translate("mobo");
        assert!(reliable
            .iter()
            .any(|candidate| candidate.text == "EXPLICIT"));
        assert!(reliable.iter().any(|candidate| candidate.text == "MB"));
        assert!(
            reliable
                .iter()
                .all(|candidate| candidate.source != CandidateSource::Sentence),
            "{storage}: a reliable full phrase suppresses Poet but not the collector phrase stream"
        );

        let proper_prefix = translator.translate("moboyix");
        assert!(proper_prefix.iter().any(|candidate| {
            candidate.text == "FULL"
                && candidate.source
                    == CandidateSource::PartialTable {
                        consumed: 6,
                        recompose_on_default: true,
                    }
        }));

        let normal = translator.translate("moyi");
        assert_eq!(
            normal[0].text, "MY",
            "{storage}: Octagram-backed Poet sentence"
        );
        assert_eq!(normal[0].source, CandidateSource::Sentence, "{storage}");
        assert!(
            normal
                .iter()
                .all(|candidate| candidate.text != "ABBREVIATION-LEAK"),
            "{storage}: a normal full path must prune the competing abbreviation path"
        );
        assert!(
            translator
                .translate("myi")
                .iter()
                .any(|candidate| candidate.text == "ABBREVIATION-OWNED"),
            "{storage}: an abbreviation-only graph remains admitted"
        );

        for input in ["yi", "mobo", "moboyix", "moyi", "myi"] {
            let complete = translator.translate(input);
            let bounded = translator.translate_with_context_and_request(
                input,
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(2).with_debug_full_count(true),
            );
            assert_eq!(
                bounded.candidates,
                complete[..bounded.candidates.len()],
                "{storage} {input}: bounded graph work must be a field-identical complete prefix"
            );
        }
    }
    assert!(
        (head_merge_qualities[0] - head_merge_qualities[1]).abs()
            <= head_merge_qualities[0] * 0.000_01,
        "owned and byte-backed ScriptTranslation merge qualities must share the normalized weight namespace: {head_merge_qualities:?}"
    );

    let direct_only = StaticTableTranslator::from_compact_dictionary(
        dictionary,
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(
                &syllabary,
                &formulas,
                checksum,
                0x5904_e002,
            ))
            .expect("direct-only untoned prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(false)
    .with_sentence_policy(SentencePolicy::UpstreamScript);
    assert!(
        direct_only
            .translate("mobo")
            .iter()
            .any(|candidate| candidate.text == "EXPLICIT"),
        "enable_sentence=false must retain graph-owned direct order"
    );
    assert!(
        direct_only
            .translate("moyi")
            .iter()
            .all(|candidate| candidate.source != CandidateSource::Sentence),
        "enable_sentence=false gates only Poet sentence generation"
    );
}

#[test]
fn upstream_script_untoned_graph_rejects_flattened_phrase_codes_as_syllable_edges() {
    let dictionary = TableDictionary::new([
        TableEntry::new("alpha", "甲", 100.0),
        TableEntry::new("beta", "乙", 90.0),
        TableEntry::new("alphabeta", "甲乙", 500.0),
    ]);
    let formulas = ["abbrev/^([a-z]).+$/$1/".to_owned()];
    // Deliberately include the flattened phrase in the compact prism. Pinned
    // librime's syllabary contains only the single-character readings; the
    // phrase must instead be reached by Table::Query traversal.
    let syllabary = ["alpha", "beta", "alphabeta"].map(str::to_owned);
    let checksum = 0x5904_e041;
    let prism_bytes = build_prism_bin(&syllabary, &formulas, checksum, 0x5904_e042);

    let owned = StaticTableTranslator::from_compact_dictionary(
        dictionary.clone(),
        Some(
            parse_rime_prism_bin_payload(prism_bytes.clone())
                .expect("owned structural prism should parse"),
        ),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_model(100);

    let table_bytes = build_table_bin(&dictionary, checksum);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed structural metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed structural table should parse");
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let runtime_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed structural prism should parse");
    let poet_source: Arc<dyn PoetByteSource> = Arc::new(OwnedPoetBytes::new(build_poet_bin(
        dictionary.entries().iter().cloned(),
        &[],
        &[],
        checksum,
    )));
    let byte_backed = StaticTableTranslator::from_compact_table_store_with_prism_runtime(
        store,
        Some(runtime_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_upstream_sentence_poet_source(poet_source, checksum)
    .with_upstream_sentence_model(100);

    for (storage, translator) in [("owned", owned), ("byte", byte_backed)] {
        let abbreviated = translator.translate("a");
        assert!(
            abbreviated.iter().any(|candidate| candidate.text == "甲"),
            "{storage}: a real single-character reading remains a syllable edge"
        );
        assert!(
            abbreviated.iter().all(|candidate| candidate.text != "甲乙"),
            "{storage}: a flattened phrase code must not masquerade as one abbreviated syllable"
        );
        assert!(
            translator
                .translate("ab")
                .iter()
                .any(|candidate| candidate.text == "甲乙"),
            "{storage}: the phrase remains reachable by concatenating its two real syllables"
        );
    }
}

#[test]
fn upstream_script_policy_preserves_dictionary_exclusions_for_model_rows_and_sentences() {
    let dictionary = TableDictionary::new([
        TableEntry::new("bei1", "A", 100.0),
        TableEntry::new("bei2", "B", 90.0),
        TableEntry::new("ngo5", "N", 100.0),
        TableEntry::new("ng5", "M", 80.0),
        TableEntry::new("bei3", "AM", 70.0),
        TableEntry::new("bei2ngo5", "BN", 80.0),
    ]);
    let formulas = [
        "derive/\\d//".to_owned(),
        "xform/1/v/".to_owned(),
        "xform/2/x/".to_owned(),
        "xform/5/xx/".to_owned(),
    ];
    let syllabary = dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("dictionary-exclude 4a prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_spelling_algebra(&formulas)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript)
        .with_preset_vocabulary([
            PresetVocabularyEntry::new("AN", 200.0),
            PresetVocabularyEntry::new("BN", 1_000.0),
        ])
        .with_dictionary_exclude(["BN", "A"])
        .with_upstream_sentence_model(100);

    assert!(
        translator
            .translate("beingo")
            .iter()
            .all(|candidate| candidate.text != "BN"),
        "model-derived phrases must honor exact dictionary exclusions"
    );
    assert!(
        translator
            .translate("being")
            .iter()
            .all(
                |candidate| candidate.source != CandidateSource::Sentence || candidate.text != "AM"
            ),
        "synthesized sentences must not reintroduce an exactly excluded component"
    );

    let containing_dictionary = TableDictionary::new([
        TableEntry::new("bei1", "XA", 100.0),
        TableEntry::new("ng5", "M", 80.0),
    ]);
    let containing_syllabary = containing_dictionary
        .entries()
        .iter()
        .map(|entry| entry.code.clone())
        .collect::<Vec<_>>();
    let containing_prism =
        parse_rime_prism_bin_payload(build_prism_bin(&containing_syllabary, &formulas, 1, 2))
            .expect("containing-text 4a prism should parse");
    let containing = StaticTableTranslator::from_compact_dictionary(
        containing_dictionary,
        Some(containing_prism),
    )
    .with_spelling_algebra(&formulas)
    .with_sentence(true)
    .with_sentence_policy(SentencePolicy::UpstreamScript)
    .with_dictionary_exclude(["A"])
    .with_upstream_sentence_model(100);
    assert!(
        containing.translate("being").iter().any(|candidate| {
            candidate.source == CandidateSource::Sentence && candidate.text == "XAM"
        }),
        "dictionary_exclude is exact text filtering, not substring censorship"
    );
}

#[test]
fn empty_or_invalid_algebra_keeps_heap_prefix_fallback_identity_tone_alias() {
    let dictionary = TableDictionary::new([
        TableEntry::new("bei2", "\u{6bd4}", 100.0),
        TableEntry::new("ni", "\u{4f60}", 80.0),
    ]);
    let cases = [
        ("empty", Vec::<String>::new()),
        ("invalid", vec!["not-a-formula".to_owned()]),
    ];

    for (label, formulas) in cases {
        let translator = StaticTableTranslator::from_dictionary(dictionary.clone())
            .with_prefix_fallback(true)
            .with_leading_syllable_reachability(false)
            .with_spelling_algebra(&formulas)
            .with_sentence(false);
        let candidates = translator.translate("beini");
        let normal = candidates
            .iter()
            .find(|candidate| candidate.text == "\u{6bd4}")
            .unwrap_or_else(|| {
                panic!("{label} algebra must retain bounded identity tone-alias fallback")
            });
        assert_eq!(normal.comment, "bei2");
        assert_eq!(
            normal.source,
            CandidateSource::PartialTable {
                consumed: 3,
                recompose_on_default: true,
            }
        );
    }
}

#[test]
fn explicit_false_spelling_algebra_keeps_surface_index_unallocated_until_enabled() {
    let formulas = double_pinyin_shaped_algebra();
    let syllabary = ["hao", "ha", "ni", "zhong"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("lazy-index prism should parse");
    let disabled = StaticTableTranslator::from_compact_dictionary(
        double_pinyin_reachability_dictionary(),
        Some(prism),
    )
    .with_leading_syllable_reachability(false)
    .with_spelling_algebra(&formulas)
    .with_sentence(false);

    let owner = |translator: &StaticTableTranslator, owner_name: &str| {
        translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == owner_name)
            .unwrap_or_else(|| panic!("{owner_name} owner row should exist"))
    };
    let before = owner(&disabled, "translator.leading_fetch_index");
    assert_eq!((before.estimated_bytes, before.item_count), (0, 0));
    let before_seed = owner(&disabled, "translator.leading_fetch_seed");
    assert_eq!(
        (before_seed.estimated_bytes, before_seed.item_count),
        (0, 0),
        "explicit-false construction must not scan or retain a syllabary seed"
    );
    assert!(disabled.translate("hknivs").is_empty());
    let after_disabled_lookup = owner(&disabled, "translator.leading_fetch_index");
    assert_eq!(
        (
            after_disabled_lookup.estimated_bytes,
            after_disabled_lookup.item_count
        ),
        (0, 0),
        "explicit-false translation must not initialize the expanded surface index"
    );

    let enabled = disabled.with_leading_syllable_reachability(true);
    let enabled_seed = owner(&enabled, "translator.leading_fetch_seed");
    assert_eq!(
        (enabled_seed.estimated_bytes, enabled_seed.item_count),
        (0, 0),
        "a checksum-current compact prism must remain the authoritative transformed-surface index"
    );
    let enabled_cold_index = owner(&enabled, "translator.leading_fetch_index");
    assert_eq!(
        (
            enabled_cold_index.estimated_bytes,
            enabled_cold_index.item_count
        ),
        (0, 0),
        "late enabling must not reconstruct a redundant syllabary seed or expanded index"
    );
    assert_surface_leading_single(&enabled, "hknivs", "\u{597d}", "hao", 2);
    let after_enabled_lookup = owner(&enabled, "translator.leading_fetch_index");
    assert_eq!(
        (
            after_enabled_lookup.estimated_bytes,
            after_enabled_lookup.item_count
        ),
        (0, 0),
        "the direct prism lookup must stay heap-index-free after transformed reachability"
    );
}

#[test]
fn no_algebra_heap_reachability_is_index_free_and_retains_bounded_tone_aliases() {
    let mut entries = vec![
        TableEntry::new("bei2", "\u{6bd4}", 100.0),
        TableEntry::new("e", "\u{4fc4}", 99.0),
    ];
    entries.extend((0..64).map(|index| {
        TableEntry::new(
            format!("phrase{index}"),
            format!("\u{8a5e}{index}"),
            50.0 - index as f32,
        )
    }));
    let dictionary = TableDictionary::new(entries);
    let translator = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_leading_syllable_reachability(true)
        .with_sentence(false);
    let owner = |translator: &StaticTableTranslator, owner_name: &str| {
        translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == owner_name)
            .unwrap_or_else(|| panic!("{owner_name} owner row should exist"))
    };

    assert_eq!(
        owner(&translator, "translator.leading_fetch_seed").item_count,
        0,
        "no-algebra heap reachability must not scan or retain a global seed"
    );
    assert_eq!(
        owner(&translator, "translator.leading_fetch_index").item_count,
        0,
        "no-algebra heap reachability must start without a global surface index"
    );
    assert_eq!(
        owner(&translator, "translator.spelling_correction_entries").item_count,
        0
    );
    assert_eq!(
        owner(&translator, "translator.spelling_correction_surfaces").item_count,
        0
    );

    assert_surface_leading_single(&translator, "beini", "\u{6bd4}", "bei2", 3);
    assert_surface_leading_single(&translator, "etail", "\u{4fc4}", "e", 1);
    assert!(
        translator.translate("xxxxxxxx").is_empty(),
        "a direct heap miss must not enumerate the dictionary"
    );
    assert_eq!(
        owner(&translator, "translator.leading_fetch_index").item_count,
        0,
        "bounded exact/tone probes must not initialize a global index"
    );

    let invalid = StaticTableTranslator::from_dictionary(dictionary)
        .with_leading_syllable_reachability(true)
        .with_spelling_algebra(&["not-a-formula".to_owned()])
        .with_sentence(false);
    assert_eq!(
        owner(&invalid, "translator.leading_fetch_seed").item_count,
        0,
        "invalid all-or-nothing algebra must remain on the index-free identity path"
    );
    assert_surface_leading_single(&invalid, "beini", "\u{6bd4}", "bei2", 3);
    assert_eq!(
        owner(&invalid, "translator.leading_fetch_index").item_count,
        0
    );
}

#[test]
fn no_algebra_compact_identity_prism_retains_bounded_tone_aliases() {
    let dictionary = TableDictionary::new([
        TableEntry::new("bei2", "\u{6bd4}", 100.0),
        TableEntry::new("be2", "\u{5564}", 95.0),
        TableEntry::new("ni", "\u{4f60}", 90.0),
    ]);
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("tone-alias table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("tone-alias compact table should parse");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &[], 0x1234_5678, 2);
    assert_eq!(
        i32::from_le_bytes(prism_bytes[56..60].try_into().expect("map pointer bytes")),
        0,
        "no-algebra control must use the upstream null-map identity prism"
    );
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed identity prism should parse");
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_leading_syllable_reachability(true)
            .with_sentence(false);

    assert_surface_leading_single(&translator, "beini", "\u{6bd4}", "bei2", 3);
    assert_surface_leading_single(&translator, "bei", "\u{5564}", "be2", 2);
    let owner = |owner_name: &str| {
        translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == owner_name)
            .unwrap_or_else(|| panic!("{owner_name} owner row should exist"))
    };
    assert_eq!(
        (
            owner("translator.leading_fetch_seed").item_count,
            owner("translator.leading_fetch_index").item_count,
        ),
        (0, 0),
        "bounded Darts tone-child probes must not allocate a global surface index"
    );
}

#[test]
fn explicit_algebra_prism_does_not_invent_an_unexposed_tone_alias() {
    let formulas = ["derive/^ni$/nx/".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("bei2", "\u{6bd4}", 100.0),
        TableEntry::new("ni", "\u{4f60}", 90.0),
    ]);
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("explicit-algebra table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("explicit-algebra compact table should parse");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 0x1234_5678, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("explicit-algebra byte-backed prism should parse");
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_spelling_algebra(&formulas)
            .with_leading_syllable_reachability(true)
            .with_sentence(false);

    assert!(
        !translator
            .translate("beini")
            .iter()
            .any(|candidate| candidate.text == "\u{6bd4}"),
        "an unrelated explicit algebra must not synthesize an undeployed bei surface from canonical bei2"
    );
}

#[test]
fn real_stroke_null_map_no_algebra_index_does_not_fan_out_to_every_spelling() {
    let schema_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/yune-web/public/schema");
    let table_bytes = fs::read(schema_root.join("stroke.table.bin"))
        .expect("product Stroke table fixture should be present");
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("product Stroke table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("product Stroke table should load as compact storage");
    let spelling_count = store.syllabary_codes().len();
    assert!(
        spelling_count >= 157_000,
        "control fixture should retain the large Stroke spelling inventory"
    );
    let boundary_37 = store
        .syllabary_codes()
        .iter()
        .find(|code| code.len() == 37 && code.is_ascii())
        .expect("tracked Stroke syllabary should retain a real 37-byte ASCII code")
        .clone();
    let long_stroke_code = store
        .syllabary_codes()
        .iter()
        .find(|code| code.len() > 59 && code.is_ascii())
        .expect("tracked Stroke syllabary should retain a real key extending beyond byte 59")
        .clone();
    let boundary_59 = long_stroke_code[..59].to_owned();
    assert_eq!(boundary_59.chars().count(), 59);
    let functional_code = store
        .syllabary_codes()
        .iter()
        .find(|code| code.len() <= 8 && code.is_ascii() && store.exact_candidate_count(code) == 1)
        .expect("tracked Stroke table should retain a bounded one-candidate control code")
        .clone();
    let prism_bytes = fs::read(schema_root.join("stroke.prism.bin"))
        .expect("product Stroke prism fixture should be present");
    assert_eq!(
        i32::from_le_bytes(prism_bytes[56..60].try_into().expect("map pointer bytes")),
        0,
        "control fixture must exercise the upstream null-map identity optimization"
    );
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("product Stroke identity prism should parse byte-backed");
    assert!(prism.has_byte_backed_identity_spelling_map());
    let boundary_37_matches =
        prism.common_prefix_canonical_codes(&boundary_37, store.syllabary_codes(), usize::MAX);
    assert!(
        boundary_37_matches
            .iter()
            .any(|(matched_len, lookup)| *matched_len == 37 && lookup.code == boundary_37),
        "the real byte-backed Stroke Darts must return the full 37-character deployed key"
    );
    let full_long_matches =
        prism.common_prefix_canonical_codes(&long_stroke_code, store.syllabary_codes(), usize::MAX);
    assert!(
        full_long_matches.iter().any(|(matched_len, lookup)| {
            *matched_len == long_stroke_code.len() && lookup.code == long_stroke_code
        }),
        "the deployed long key must prove that its Darts path extends beyond byte 59"
    );
    let expected_through_59 = full_long_matches
        .iter()
        .filter(|(matched_len, _)| *matched_len <= boundary_59.len())
        .map(|(matched_len, lookup)| (*matched_len, lookup.code.to_owned()))
        .collect::<Vec<_>>();
    let actual_through_59 = prism
        .common_prefix_canonical_codes(&boundary_59, store.syllabary_codes(), usize::MAX)
        .into_iter()
        .map(|(matched_len, lookup)| (matched_len, lookup.code.to_owned()))
        .collect::<Vec<_>>();
    assert!(!actual_through_59.is_empty());
    assert_eq!(actual_through_59, expected_through_59);
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_leading_syllable_reachability(true)
            .with_completion(false)
            .with_sentence(false);
    let owner = |owner_name: &str| {
        translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == owner_name)
            .unwrap_or_else(|| panic!("{owner_name} owner row should exist"))
    };
    assert_eq!(
        (
            owner("translator.leading_fetch_seed").estimated_bytes,
            owner("translator.leading_fetch_seed").item_count
        ),
        (0, 0),
        "the real {spelling_count}-spelling Stroke product must not clone a reachability seed"
    );
    assert_eq!(
        (
            owner("translator.leading_fetch_index").estimated_bytes,
            owner("translator.leading_fetch_index").item_count
        ),
        (0, 0)
    );
    let prism_map = owner("prism.spelling_map");
    assert_eq!(
        (prism_map.estimated_bytes, prism_map.item_count),
        (0, 0),
        "the upstream null spelling-map pointer must remain a zero-allocation identity view"
    );
    let prism_darts = owner("prism.double_array_units");
    assert_eq!(prism_darts.class, MemoryOwnerClass::HeapOwnedGuarded);
    assert!(prism_darts.estimated_bytes > 0 && prism_darts.item_count > 0);

    let functional = translator.translate(&functional_code);
    assert_eq!(
        functional.len(),
        1,
        "the compact translator must retain a bounded functional identity lookup"
    );
    assert!(!functional[0].text.is_empty());
    let invalid = "x".repeat(84);
    assert!(
        translator.translate(&invalid).is_empty(),
        "an authoritative Stroke prism miss must stay empty without a storage-prefix fallback"
    );
    assert_eq!(
        (
            owner("translator.leading_fetch_seed").estimated_bytes,
            owner("translator.leading_fetch_seed").item_count,
            owner("translator.leading_fetch_index").estimated_bytes,
            owner("translator.leading_fetch_index").item_count,
        ),
        (0, 0, 0, 0),
        "valid 37/59-char Darts traversals, a bounded functional lookup, and an 84-char miss must keep both owners at zero"
    );
}

#[test]
fn replacing_initialized_algebra_with_empty_or_invalid_formulas_resets_surface_caches() {
    let formulas = ["xform/^long$/x/".to_owned()];
    let syllabary = ["long".to_owned()];
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("reset-control prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(
        TableDictionary::new([TableEntry::new("long", "\u{9577}", 100.0)]),
        Some(prism),
    )
    .with_leading_syllable_reachability(true)
    .with_spelling_algebra(&formulas)
    .with_sentence(false);
    let owner_stats = |translator: &StaticTableTranslator, owner_name: &str| {
        let row = translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == owner_name)
            .unwrap_or_else(|| panic!("{owner_name} owner row should exist"));
        (row.estimated_bytes, row.item_count)
    };

    assert_surface_leading_single(&translator, "xy", "\u{9577}", "long", 1);
    assert_eq!(
        owner_stats(&translator, "translator.leading_fetch_index"),
        (0, 0),
        "a current compact prism must resolve the transformed surface without a redundant heap index"
    );

    let reset = translator.with_spelling_algebra(&[]);
    assert_eq!(
        owner_stats(&reset, "translator.leading_fetch_seed"),
        (0, 0),
        "empty algebra must drop the transformed seed"
    );
    assert_eq!(
        owner_stats(&reset, "translator.leading_fetch_index"),
        (0, 0),
        "empty algebra must drop the initialized transformed index"
    );
    assert_surface_leading_single(&reset, "longy", "\u{9577}", "long", 4);

    let invalid = reset.with_spelling_algebra(&["not-a-formula".to_owned()]);
    assert_eq!(
        owner_stats(&invalid, "translator.leading_fetch_seed"),
        (0, 0),
        "an invalid replacement algebra must not retain a seed"
    );
    assert_eq!(
        owner_stats(&invalid, "translator.leading_fetch_index"),
        (0, 0),
        "an invalid replacement algebra must invalidate the prior default index"
    );
    assert_surface_leading_single(&invalid, "longy", "\u{9577}", "long", 4);
}

#[test]
fn replacing_compact_algebra_does_not_reuse_stale_upstream_script_prism_edges() {
    let original_formulas = ["abbrev/^aa1$/a/".to_owned(), "abbrev/^bb1$/b/".to_owned()];
    let replacement_formulas = ["abbrev/^aa1$/x/".to_owned(), "abbrev/^bb1$/y/".to_owned()];
    let dictionary = TableDictionary::new([
        TableEntry::new("aa1", "A", 100.0),
        TableEntry::new("bb1", "B", 100.0),
        TableEntry::new("aa1bb1", "AB", 500.0),
    ]);
    let syllabary = ["aa1", "bb1", "aa1bb1"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(
        &syllabary,
        &original_formulas,
        0x5904_b421,
        0x5904_b422,
    ))
    .expect("original compact algebra prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_spelling_algebra(&original_formulas)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamScript);

    let control = translator.translate("ab");
    assert!(
        control.iter().any(|candidate| candidate.text == "AB"),
        "control must exercise the original prism-backed surface graph: {control:?}"
    );

    let replaced = translator.with_spelling_algebra(&replacement_formulas);
    assert!(
        replaced
            .translate("ab")
            .iter()
            .all(|candidate| candidate.text != "AB"),
        "a replaced algebra must not segment through the previous prism mapping"
    );
}

#[test]
fn a_mixed_valid_and_invalid_algebra_has_no_transformed_runtime_edge_or_seed() {
    let formulas = [
        "derive/^hao$/hx/".to_owned(),
        "this-is-not-a-valid-algebra-formula".to_owned(),
    ];
    let syllabary = ["hao", "ni"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 1, 2))
        .expect("identity control prism should parse");
    let compact = StaticTableTranslator::from_compact_dictionary(
        TableDictionary::new([
            TableEntry::new("hao", "\u{597d}", 100.0),
            TableEntry::new("ni", "\u{4f60}", 90.0),
        ]),
        Some(prism),
    )
    .with_leading_syllable_reachability(true)
    .with_spelling_algebra(&formulas)
    .with_sentence(false);
    let heap = StaticTableTranslator::new([("hao", "\u{597d}"), ("ni", "\u{4f60}")])
        .with_leading_syllable_reachability(true)
        .with_spelling_algebra(&formulas)
        .with_sentence(false);

    for (storage, translator) in [("compact", &compact), ("heap/source", &heap)] {
        let seed = translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "translator.leading_fetch_seed")
            .expect("leading-fetch seed owner should exist");
        assert_eq!(
            (seed.estimated_bytes, seed.item_count),
            (0, 0),
            "{storage}: an invalid sibling must clear the complete algebra before cache seeding"
        );
        assert!(
            !translator
                .translate("hxni")
                .iter()
                .any(|candidate| candidate.text == "\u{597d}"),
            "{storage}: the valid sibling must not survive as a partial transformed edge"
        );
    }
}

#[test]
fn replacing_heap_algebra_resets_initialized_untoned_dictionary_classification() {
    let translator = StaticTableTranslator::new([("bei2", "\u{6bd4}")]).with_sentence(false);
    assert!(
        !translator.untoned_dictionary(),
        "the control storage contains a tone digit"
    );

    let transformed = translator.with_spelling_algebra(&["xform/2$//".to_owned()]);
    assert!(
        transformed.untoned_dictionary(),
        "heap algebra rewrites the storage key to `bei`; the old OnceLock classification must not survive"
    );
}

#[test]
fn normalized_injection_fallback_does_not_masquerade_as_a_bare_exact() {
    let translator = StaticTableTranslator::from_compact_dictionary(
        TableDictionary::new([
            TableEntry::new("bei2", "\u{6bd4}", 100.0),
            TableEntry::new("be2", "\u{5564}", 90.0),
        ]),
        None,
    )
    .with_leading_syllable_reachability(true)
    .with_sentence(false);

    // With no deployed algebra, `bei` is only the legacy normalized injection
    // key for canonical `bei2`; the normal exact path does not serve it. It must
    // therefore not trip the bare guard and erase the shorter `be` family.
    assert_surface_leading_single(&translator, "bei", "\u{5564}", "be2", 2);
}

#[test]
fn leading_single_ordered_quality_preserves_initial_quality_across_translators() {
    let low = StaticTableTranslator::new([("ba", "低")])
        .with_initial_quality(0.0)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);
    let high = StaticTableTranslator::new([("ba", "高")])
        .with_initial_quality(10.0)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);

    let injected = |translator: &StaticTableTranslator, target: &str| {
        translator
            .translate("baba")
            .into_iter()
            .find(|candidate| candidate.text == target)
            .unwrap_or_else(|| panic!("{target} should be injected from the leading `ba` family"))
    };
    let low_candidate = injected(&low, "低");
    let high_candidate = injected(&high, "高");
    assert!(matches!(
        low_candidate.source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: true,
        }
    ));
    assert!(matches!(
        high_candidate.source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: true,
        }
    ));
    assert_eq!(
        high_candidate.quality - low_candidate.quality,
        10.0,
        "D-47's positional ordering must retain the namespace initial_quality offset"
    );
}

#[test]
fn leading_single_ordered_quality_keeps_unequal_lists_inside_their_namespace_band() {
    let low_entries = (0..20)
        .map(|index| {
            let text = char::from_u32(0x4e00 + index)
                .expect("test code point should be valid")
                .to_string();
            ("ba".to_owned(), text)
        })
        .collect::<Vec<_>>();
    let low = StaticTableTranslator::new(low_entries)
        .with_initial_quality(0.0)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);
    let high = StaticTableTranslator::new([("ba", "\u{9ad8}")])
        .with_initial_quality(10.0)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);

    let low_candidates = low.translate("baba");
    let high_candidate = high
        .translate("baba")
        .into_iter()
        .find(|candidate| candidate.text == "\u{9ad8}")
        .expect("high-priority translator should inject its leading single");
    let low_head = low_candidates
        .iter()
        .map(|candidate| candidate.quality)
        .reduce(f32::max)
        .expect("low-priority translator should inject its family");

    assert!(
        low_head < 1.0,
        "positional qualities must remain in the translator's bounded unit band"
    );
    assert!(
        high_candidate.quality > low_head,
        "twenty low-priority rows must not swamp a one-row translator with initial_quality=10"
    );
}

#[test]
fn leading_single_with_single_letter_code_recomposes_when_remainder_remains() {
    // M59 finding #5: a leading single whose dictionary code is a single letter —
    // the pinyin vowel syllables e/a/o -> 俄/阿/哦 — consumes only 1 char. The old
    // `consumed_input_len > 1` gate marked it non-recomposing, so DefaultConfirm
    // (space) committed the remainder raw (`俄luo`) instead of recomposing it to
    // `luo`. The family single must carry recompose_on_default=true whenever a
    // remainder is left (consumed < input length), independent of code length.
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: leading_single_vowel_code
version: "0.1"
sort: by_weight
...

俄羅	eluo	100
俄	e	95
額	e	90
羅	luo	80
"#,
    )
    .expect("dictionary should parse");
    let syllabary = ["e", "luo"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 1, 5))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        .with_leading_syllable_reachability(true)
        .with_sentence(false);

    let result = translator.translate_with_context_and_request(
        "eluo",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(8).with_debug_full_count(true),
    );

    let e_single = result
        .candidates
        .iter()
        .find(|candidate| candidate.text == "俄")
        .unwrap_or_else(|| {
            panic!(
                "leading single 俄 (code `e`) must be injected for `eluo`; got {:?}",
                result
                    .candidates
                    .iter()
                    .map(|candidate| candidate.text.as_str())
                    .collect::<Vec<_>>()
            )
        });
    assert!(
        matches!(
            e_single.source,
            CandidateSource::PartialTable {
                consumed: 1,
                recompose_on_default: true,
            }
        ),
        "a 1-char-code leading single with a remainder must recompose on default \
         (pre-fix `consumed_input_len > 1` wrongly set false for consumed==1); got {:?}",
        e_single.source,
    );
}

#[test]
fn bounded_leading_single_reachable_under_prediction_never_first_without_limit() {
    // M59 finding #9 (flip precondition): a schema combining
    // leading_syllable_reachability with prediction_never_first, but WITHOUT a
    // prediction limit and WITHOUT prefix_fallback (the combo the default-ON flip
    // creates for e.g. cangjie), must still take the bounded path — which carries
    // the leading-single injection (translator/mod.rs:2188). Pre-fix,
    // bounded_request_supported returned false for that combo, so the caller fell
    // to the compact Some(limit) fallback whose injection gate
    // (prefix_fallback_limit.is_none()) is false, silently dropping the leading
    // single. 俄 is the `俄 e 95` dict row, not Yune-derived.
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: reach_prediction_never_first
version: "0.1"
sort: by_weight
...

俄羅	eluo	100
俄	e	95
額	e	90
羅	luo	80
"#,
    )
    .expect("dictionary should parse");
    let syllabary = ["e", "luo"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 1, 5))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        .with_leading_syllable_reachability(true)
        .with_sentence(false)
        // The precondition combo: prediction_never_first with NO limit / NO
        // prefix_fallback — the first bounded_request_supported disjunct is
        // (false || false || false) pre-fix.
        .with_prediction_never_first(true);

    let result = translator.translate_with_context_and_request(
        "eluo",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(8).with_debug_full_count(true),
    );
    assert!(
        result
            .candidates
            .iter()
            .any(|candidate| candidate.text == "俄"),
        "leading single 俄 must reach the bounded page under prediction_never_first \
         (finding #9); pre-fix the compact Some(limit) fallback skipped the \
         injection; got {:?}",
        result
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn untoned_dictionary_classification_is_structural_not_flag_keyed() {
    // M59 finding #6 (flip precondition): the untoned-relaxation in the
    // leading-single filter admits digit-less single-char rows ONLY when the
    // backing dictionary is structurally UNTONED (no tone digit in any syllable
    // code), NOT whenever the reachability flag is on. This keeps the default-ON
    // schema-general flip from admitting digit-less/malformed rows into a TONED
    // jyutping family and shifting the M58-pinned positions (畀@6, 諮@27). Here we
    // assert the structural classifier directly.
    let untoned = StaticTableTranslator::from_compact_dictionary(
        TableDictionary::parse_rime_dict_yaml(
            "---\nname: untoned\nversion: \"0.1\"\nsort: by_weight\n...\n\n莫\tmo\t100\n伯\tbo\t90\n",
        )
        .expect("untoned dictionary should parse"),
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(
                &["mo".to_owned(), "bo".to_owned()],
                &[],
                1,
                5,
            ))
            .expect("prism should parse"),
        ),
    );
    assert!(
        untoned.untoned_dictionary(),
        "pure-alpha luna-style codes (mo/bo) must classify as untoned"
    );

    let toned = StaticTableTranslator::from_compact_dictionary(
        TableDictionary::parse_rime_dict_yaml(
            "---\nname: toned\nversion: \"0.1\"\nsort: by_weight\n...\n\n畀\tbei2\t100\n諮\tzi1\t90\n",
        )
        .expect("toned dictionary should parse"),
        Some(
            parse_rime_prism_bin_payload(build_prism_bin(
                &["bei2".to_owned(), "zi1".to_owned()],
                &[],
                1,
                5,
            ))
            .expect("prism should parse"),
        ),
    );
    assert!(
        !toned.untoned_dictionary(),
        "tone-digit codes (bei2/zi1) must classify as toned so the relaxation stays off"
    );
}

#[test]
fn toned_classified_dictionary_rejects_digitless_leading_single_under_flip() {
    // M59 finding #6 — the re-key BITES here. Same untoned leading family (`俄`
    // code `e`) as the reachability tests, but the dictionary also carries one
    // toned code (`好 hou2`), so it classifies as TONED. With the reachability flag
    // forced on (simulating the default-ON flip on a toned schema), the digit-less
    // single 俄 must NOT be admitted into the leading family: the relaxation keys
    // on the structural classification, not the flag. Pre-fix (flag-keyed) the flag
    // being on admitted 俄 and would have polluted the toned family / shifted pins.
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: toned_classified
version: "0.1"
sort: by_weight
...

俄羅	eluo	100
俄	e	95
額	e	90
羅	luo	80
好	hou2	70
"#,
    )
    .expect("dictionary should parse");
    let syllabary = ["e", "luo", "hou2"].map(str::to_owned);
    let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 1, 5))
        .expect("test prism should parse");
    let translator = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_completion(true)
        // Simulate the flip: reachability on, but the dict is structurally toned.
        .with_leading_syllable_reachability(true)
        .with_sentence(false);
    assert!(
        !translator.untoned_dictionary(),
        "the `hou2` tone-digit code must make this dictionary classify as toned"
    );

    let result = translator.translate_with_context_and_request(
        "eluo",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::unbounded(),
    );
    assert!(
        !result
            .candidates
            .iter()
            .any(|candidate| candidate.text == "俄"),
        "a toned-classified dictionary must NOT admit the digit-less leading single \
         俄 under the flip (the untoned relaxation is structure-keyed, not \
         flag-keyed; pre-fix the on-flag admitted it); got {:?}",
        result
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn bounded_request_uses_prefix_fallback_without_full_fallback() {
    let _guard = super::m37_metrics_test_guard();
    let translator = StaticTableTranslator::new([("nei", "你")]).with_prefix_fallback(true);
    let context = Context::default();

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "neix",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(1),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(result.candidates[0].text, "你");
    assert_eq!(metrics.full_list_fallback_count, 0);
    assert!(metrics.prefix_fallback_calls > 0);
}

#[test]
fn full_bounded_page_merges_a_capped_prefix_window_and_keeps_has_more() {
    let _guard = super::m37_metrics_test_guard();
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let mut entries = (0..10)
        .map(|index| TableEntry::new("az", format!("EXACT{index}"), 200.0 - index as f32))
        .collect::<Vec<_>>();
    entries.extend([
        TableEntry::new("aa", "EXACT0", 100.0),
        TableEntry::new("aa", "EXACT1", 99.0),
        TableEntry::new("ab", "EXACT2", 98.0),
        TableEntry::new("ab", "EXACT3", 97.0),
        TableEntry::new("ac", "EXACT4", 96.0),
        TableEntry::new("ac", "EXACT5", 95.0),
        TableEntry::new("ad", "PREFIX_UNIQUE", 94.0),
    ]);
    for (index, code) in (b'e'..=b'x').map(char::from).enumerate() {
        entries.push(TableEntry::new(
            format!("a{code}"),
            format!("TAIL{index}"),
            90.0 - index as f32,
        ));
    }
    let dictionary = TableDictionary::new(entries);
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("existence-probe table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("existence-probe table should load byte-backed");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 0x1234_5678, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("existence-probe prism should load byte-backed");
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_spelling_algebra(&formulas)
            .with_prefix_fallback(true)
            .with_sentence(false);
    let context = Context::default();

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let bounded = translator.translate_with_context_and_request(
        "az",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(10),
    );
    let probe_metrics = crate::m37_metrics_snapshot();

    assert_eq!(
        bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        (0..10)
            .map(|index| format!("EXACT{index}"))
            .collect::<Vec<_>>(),
        "the shared fallback merge must not displace the full exact family"
    );
    assert!(
        !bounded.is_complete,
        "the unique prefix row after duplicate texts must still advertise another page"
    );
    assert!(
        (1..=10).contains(&probe_metrics.prefix_fallback_candidates),
        "the full page must materialize only its bounded merge window"
    );
    assert!(
        (1..=40).contains(&probe_metrics.prefix_fallback_views_visited),
        "the merge must stay inside the signed four-window pending cap"
    );
    assert!(
        bounded.candidates[..6].iter().all(|candidate| matches!(
            candidate.source,
            CandidateSource::PartialTable {
                consumed: 2,
                recompose_on_default: false,
            }
        )),
        "duplicate full-page exacts must receive the same span promotion as the eager merge"
    );

    crate::m37_metrics_reset();
    let debug_count = translator.translate_with_context_and_request(
        "az",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(10).with_debug_full_count(true),
    );
    let debug_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(debug_count.candidates, bounded.candidates);
    assert!(debug_count.full_count.is_some_and(|count| count > 10));
    assert!(
        debug_metrics.prefix_fallback_views_visited <= probe_metrics.prefix_fallback_views_visited,
        "the repeated debug request may reuse the bounded prefix-window cache"
    );
}

#[test]
fn full_bounded_page_stays_incomplete_when_unique_fallback_is_beyond_fetch_cap() {
    let entries = [
        TableEntry::new("abcdefgh", "EXACT0", 200.0),
        TableEntry::new("abcdefgh", "EXACT1", 199.0),
        TableEntry::new("ab", "EXACT0", 100.0),
        TableEntry::new("ab", "EXACT1", 99.0),
        TableEntry::new("ab", "UNIQUE_AFTER_CAP", 98.0),
    ];
    let translator = compact_prefix_fallback_test_translator(entries, &[]);

    let result = translator.translate_with_context_and_request(
        "abcdefgh",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(2).with_debug_full_count(true),
    );

    assert_eq!(
        result
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["EXACT0", "EXACT1"]
    );
    assert!(!result.is_complete);
    assert!(result
        .full_count
        .is_some_and(|count| count > result.candidates.len()));
}

#[test]
fn exact_duplicate_probe_reports_complete_without_a_budget_guess() {
    let entries = [
        TableEntry::new("abcdefgh", "EXACT", 200.0),
        TableEntry::new("ab", "EXACT", 100.0),
        TableEntry::new("abcd", "EXACT", 99.0),
        TableEntry::new("abcdef", "EXACT", 98.0),
        TableEntry::new("abcdefg", "EXACT", 97.0),
    ];
    let translator = compact_prefix_fallback_test_translator(entries, &[]);

    let result = translator.translate_with_context_and_request(
        "abcdefgh",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(1),
    );

    assert_eq!(result.candidates[0].text, "EXACT");
    assert!(
        result.is_complete,
        "the current-head collector exhausts this duplicate-only family instead of guessing from a global probe cap"
    );
}

#[test]
fn exhausted_duplicate_probe_can_report_complete() {
    let entries = [
        TableEntry::new("abcdefgh", "EXACT", 200.0),
        TableEntry::new("ab", "EXACT", 100.0),
        TableEntry::new("abcd", "EXACT", 99.0),
        TableEntry::new("abcdef", "EXACT", 98.0),
    ];
    let translator = compact_prefix_fallback_test_translator(entries, &[]);

    let result = translator.translate_with_context_and_request(
        "abcdefgh",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(1),
    );

    assert_eq!(result.candidates[0].text, "EXACT");
    assert!(
        result.is_complete,
        "only an exhaustive duplicate-only probe may advertise completion"
    );
}

fn compact_prefix_fallback_test_translator(
    entries: impl IntoIterator<Item = TableEntry>,
    formulas: &[String],
) -> StaticTableTranslator {
    let dictionary = TableDictionary::new(entries);
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("prefix fallback test table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("prefix fallback test table should load byte-backed");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), formulas, 0x1234_5678, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("prefix fallback test prism should load byte-backed");
    StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
        .with_spelling_algebra(formulas)
        .with_prefix_fallback(true)
        .with_sentence(false)
}

fn original_prefix_fallback_test_translators() -> [(&'static str, StaticTableTranslator); 2] {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: original_prefix_chunks
version: "0.1"
sort: original
...

A	ca	1
a	ca	100
B	cb	10
C	cc	10
"#,
    )
    .expect("sort-original prefix dictionary should parse");
    let formulas = ["derive/^c[abc]$/x/".to_owned()];
    let syllabary = ["ca", "cb", "cc"].map(str::to_owned);
    let owned_prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &formulas, 1, 2))
        .expect("owned sort-original prefix prism should parse");
    let owned =
        StaticTableTranslator::from_compact_dictionary(dictionary.clone(), Some(owned_prism))
            .with_spelling_algebra(&formulas)
            .with_prefix_fallback(true)
            .with_sentence(false)
            .with_sentence_policy(SentencePolicy::UpstreamScript);

    let table_bytes = build_table_bin(&dictionary, 0x5904_b423);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("byte-backed sort-original prefix metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("byte-backed sort-original prefix table should parse");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 1, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let byte_prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("byte-backed sort-original prefix prism should parse");
    let byte =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(byte_prism))
            .with_spelling_algebra(&formulas)
            .with_prefix_fallback(true)
            .with_sentence(false)
            .with_sentence_policy(SentencePolicy::UpstreamScript);

    [("owned", owned), ("byte-backed", byte)]
}

#[test]
fn sort_original_prefix_fallback_uses_current_heads_and_partial_sort_residual_order() {
    for (storage, translator) in original_prefix_fallback_test_translators() {
        let complete = translator.translate("xy");
        assert_eq!(
            complete
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["B", "C", "A", "a"],
            "{storage}: the first strict visitor swap must leave the later equal head in librime's one-item partial-sort residual order, while the high `ca` tail remains blocked by its accepted head"
        );

        for limit in 1..=complete.len() {
            let bounded = translator.translate_with_context_and_request(
                "xy",
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(limit),
            );
            assert_eq!(
                bounded.candidates,
                complete[..limit],
                "{storage}: bounded {limit}-row order must be the exact prefix of the complete current-head merge"
            );
        }
    }
}

#[test]
fn bounded_current_head_merge_advances_past_cross_chunk_duplicate_heads() {
    let formulas = ["derive/^c[ab]$/x/".to_owned()];
    let entries = [
        TableEntry::new("ca", "DUP", 100.0),
        TableEntry::new("ca", "A", 80.0),
        TableEntry::new("cb", "DUP", 90.0),
        TableEntry::new("cb", "B", 70.0),
    ];
    let translator = compact_prefix_fallback_test_translator(entries, &formulas)
        .with_sentence_policy(SentencePolicy::UpstreamScript);
    let complete = translator.translate("xy");
    let bounded = translator.translate_with_context_and_request(
        "xy",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(3),
    );

    assert_eq!(
        complete
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["DUP", "A", "B"]
    );
    assert_eq!(bounded.candidates, complete);
}

#[test]
fn bounded_current_head_merge_advances_past_more_than_the_old_blocked_row_cap() {
    let duplicate_prefix_rows =
        (0..96).map(|index| TableEntry::new("a0", "DUP", 900.0 - index as f32));
    let unique_prefix_rows =
        (0..16).map(|index| TableEntry::new("a0", format!("ROW{index:02}"), 700.0 - index as f32));
    let entries = std::iter::once(TableEntry::new("a0z", "DUP", 1_000.0))
        .chain(duplicate_prefix_rows)
        .chain(unique_prefix_rows)
        .collect::<Vec<_>>();
    let translator = compact_prefix_fallback_test_translator(entries, &[])
        .with_sentence_policy(SentencePolicy::UpstreamScript);
    let complete = translator.translate("a0z");
    let bounded = translator.translate_with_context_and_request(
        "a0z",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(5),
    );

    assert_eq!(
        bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["DUP", "ROW00", "ROW01", "ROW02", "ROW03"]
    );
    assert_eq!(bounded.candidates, complete[..5]);
    assert!(!bounded.is_complete);
}

#[test]
fn legacy_prefix_fallback_bounded_fields_match_complete_after_script_declines() {
    let _guard = super::m37_metrics_test_guard();
    let duplicate_heads = (0..8).map(|index| TableEntry::new("a0", "DUP", 1_000.0 - index as f32));
    let unique_rows =
        (0..512).map(|index| TableEntry::new("a0", format!("ROW{index:04}"), 900.0 - index as f32));
    let entries = duplicate_heads.chain(unique_rows).collect::<Vec<_>>();
    let dictionary = TableDictionary::new(entries.clone());
    let prism = parse_rime_prism_bin_payload(build_prism_bin(
        &["a0".to_owned()],
        &[],
        0x5904_b421,
        0x5904_b422,
    ))
    .expect("owned duplicate-head prism");
    let owned = StaticTableTranslator::from_compact_dictionary(dictionary, Some(prism))
        .with_spelling_algebra(&[])
        .with_prefix_fallback(true)
        .with_sentence(true)
        .with_upstream_sentence_model(20);
    let byte = compact_prefix_fallback_test_translator(entries, &[])
        .with_sentence(true)
        .with_upstream_sentence_model(20);
    for (storage, translator) in [("owned", owned), ("byte", byte)] {
        let complete = translator.translate("a0z");
        crate::m37_metrics_enable(true);
        crate::m37_metrics_reset();
        let bounded = translator.translate_with_context_and_request(
            "a0z",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(5).with_debug_full_count(true),
        );
        let cold_metrics = crate::m37_metrics_snapshot();
        crate::m37_metrics_enable(false);
        assert_eq!(
            bounded
                .candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["DUP", "ROW0000", "ROW0001", "ROW0002", "ROW0003"],
            "{storage}: duplicate heads must not consume the unique page window"
        );
        assert_eq!(bounded.candidates, complete[..5], "{storage}");
        assert!(
            bounded
                .full_count
                .is_some_and(|count| count > bounded.candidates.len()),
            "{storage}: the bounded family must advertise an on-demand tail"
        );
        assert!(
            cold_metrics.owned_candidates_materialized <= 16,
            "{storage}: a cold five-row page must not materialize the 512-row family: {cold_metrics:?}"
        );
    }
}

fn prefix_fallback_cache_owner_snapshot(translator: &StaticTableTranslator) -> (usize, usize) {
    translator
        .memory_owner_rows()
        .into_iter()
        .find(|row| row.owner == "translator.prefix_fallback_window_cache")
        .map(|row| (row.estimated_bytes, row.item_count))
        .expect("prefix fallback cache owner row should exist")
}

fn candidate_shape_without_quality(
    candidates: impl IntoIterator<Item = Candidate>,
) -> Vec<(String, String, CandidateSource)> {
    candidates
        .into_iter()
        .map(|candidate| (candidate.text, candidate.comment, candidate.source))
        .collect()
}

#[test]
fn bounded_prefix_fallback_cache_preserves_cold_warm_order_and_is_owner_accounted() {
    let _guard = super::m37_metrics_test_guard();
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let entries = (b'a'..=b'y')
        .map(char::from)
        .enumerate()
        .map(|(index, suffix)| {
            TableEntry::new(
                format!("a{suffix}"),
                format!("ROW{index:02}"),
                200.0 - index as f32,
            )
        })
        .collect::<Vec<_>>();
    let dictionary = TableDictionary::new(entries);
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("prefix-window table advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("prefix-window table should load byte-backed");
    let prism_bytes = build_prism_bin(store.syllabary_codes(), &formulas, 0x1234_5678, 2);
    let prism_source: Arc<dyn CompactTableByteSource> =
        Arc::new(AlgebraPrismByteSource(Arc::<[u8]>::from(prism_bytes)));
    let prism = parse_rime_prism_runtime_payload(prism_source)
        .expect("prefix-window prism should load byte-backed");
    let translator =
        StaticTableTranslator::from_compact_table_store_with_prism_runtime(store, Some(prism))
            .with_spelling_algebra(&formulas)
            .with_prefix_fallback(true)
            .with_sentence(false);
    let context = Context::default();
    let owner = |translator: &StaticTableTranslator| {
        translator
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "translator.prefix_fallback_window_cache")
            .expect("prefix fallback cache owner row should exist")
    };
    assert_eq!(owner(&translator).item_count, 0);

    let complete = translator.translate("az");
    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let cold = translator.translate_with_context_and_request(
        "az",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(10),
    );
    let cold_metrics = crate::m37_metrics_snapshot();
    assert!(cold_metrics.prefix_fallback_views_visited > 0);
    let candidate_shape = |candidates: &[Candidate]| {
        candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.text.clone(),
                    candidate.comment.clone(),
                    candidate.source.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        candidate_shape(&cold.candidates),
        candidate_shape(&complete.into_iter().take(10).collect::<Vec<_>>()),
        "the cold bounded cache fill must preserve the complete-list prefix"
    );
    let populated_owner = owner(&translator);
    assert_eq!(
        populated_owner.item_count, 11,
        "a ten-row page caches only its exact K+1 current-head certificate"
    );
    assert!(populated_owner.estimated_bytes > 0);
    assert!(populated_owner.item_count <= 128);
    assert!(populated_owner.estimated_bytes <= 512 * 1024);

    crate::m37_metrics_reset();
    let warm = translator.translate_with_context_and_request(
        "azextra",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(10),
    );
    let warm_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);
    assert_eq!(warm.candidates, cold.candidates);
    assert_eq!(warm.is_complete, cold.is_complete);
    assert_eq!(
        warm_metrics.prefix_fallback_views_visited, 0,
        "a longer input with the same resolved leading family must reuse the raw window"
    );

    let reset = translator.with_spelling_algebra(&formulas);
    assert_eq!(
        owner(&reset).item_count,
        0,
        "reconfiguring deployed algebra must invalidate the raw prefix window"
    );
    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let rebuilt = reset.translate_with_context_and_request(
        "az",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(10),
    );
    let rebuilt_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);
    assert_eq!(rebuilt.candidates, cold.candidates);
    assert!(rebuilt_metrics.prefix_fallback_views_visited > 0);
}

#[test]
fn bounded_prefix_fallback_cache_admits_a_59_character_request() {
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let entries = (b'a'..=b'y')
        .map(char::from)
        .enumerate()
        .map(|(index, suffix)| {
            TableEntry::new(
                format!("a{suffix}"),
                format!("ROW{index:02}"),
                200.0 - index as f32,
            )
        })
        .collect::<Vec<_>>();
    let translator = compact_prefix_fallback_test_translator(entries, &formulas);
    let input = "a".repeat(59);

    let bounded = translator.translate_with_context_and_request(
        &input,
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(10),
    );
    let (bytes, items) = prefix_fallback_cache_owner_snapshot(&translator);

    assert!(!bounded.candidates.is_empty());
    assert!((1..=128).contains(&items));
    assert!((1..=512 * 1024).contains(&bytes));
}

#[test]
fn large_prefix_fallback_request_limit_caches_when_actual_rows_fit() {
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let entries = (b'a'..=b'y')
        .map(char::from)
        .enumerate()
        .map(|(index, suffix)| {
            TableEntry::new(
                format!("a{suffix}"),
                format!("ROW{index:02}"),
                200.0 - index as f32,
            )
        })
        .collect::<Vec<_>>();
    let translator = compact_prefix_fallback_test_translator(entries, &formulas);
    let complete = translator.translate("az");

    let bounded = translator.translate_with_context_and_request(
        "az",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(33),
    );

    let bounded_len = bounded.candidates.len();
    assert_eq!(
        candidate_shape_without_quality(bounded.candidates),
        candidate_shape_without_quality(complete.into_iter().take(bounded_len))
    );
    let (bytes, items) = prefix_fallback_cache_owner_snapshot(&translator);
    assert_eq!(
        items, 25,
        "cache admission is based on actual retained rows"
    );
    assert!((1..=512 * 1024).contains(&bytes));
}

#[test]
fn oversized_prefix_count_and_key_bytes_bypass_the_cache() {
    let _guard = super::m37_metrics_test_guard();
    for (first_len, prefix_count) in [(1usize, 65usize), (513usize, 64usize)] {
        let entries = (0..prefix_count)
            .map(|index| {
                let code = "a".repeat(first_len + index);
                TableEntry::new(code, format!("ROW{index:02}"), 200.0 - index as f32)
            })
            .collect::<Vec<_>>();
        let translator = compact_prefix_fallback_test_translator(entries, &[]);
        let input = "a".repeat(first_len + prefix_count);
        let complete = translator.translate(&input);

        crate::m37_metrics_enable(true);
        crate::m37_metrics_reset();
        let bounded = translator.translate_with_context_and_request(
            &input,
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(4),
        );
        let metrics = crate::m37_metrics_snapshot();
        crate::m37_metrics_enable(false);

        let bounded_len = bounded.candidates.len();
        assert_eq!(
            candidate_shape_without_quality(bounded.candidates),
            candidate_shape_without_quality(complete.into_iter().take(bounded_len)),
            "bounded uncached parity failed for first_len={first_len}, prefix_count={prefix_count}"
        );
        assert_eq!(prefix_fallback_cache_owner_snapshot(&translator), (0, 0));
        assert!(
            metrics.prefix_fallback_views_visited <= 16,
            "the uncached collector must retain the signed four-window bound for first_len={first_len}, prefix_count={prefix_count}: {metrics:?}"
        );
    }
}

#[test]
fn oversized_streaming_finishes_the_longest_span_group_before_shorter_prefixes() {
    let _guard = super::m37_metrics_test_guard();
    let longest = "a".repeat(65);
    let mut entries = vec![
        TableEntry::new(&longest, "TOP1", 300.0),
        TableEntry::new(&longest, "TOP2", 299.0),
        TableEntry::new(&longest, "TOP3", 298.0),
        TableEntry::new(&longest, "TOP4", 297.0),
    ];
    entries.extend(
        (1..65).map(|len| TableEntry::new("a".repeat(len), format!("LOW{len:02}"), len as f32)),
    );
    let syllabary = (1..=65).map(|len| "a".repeat(len)).collect::<Vec<_>>();
    let owned_prism =
        parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 0x5904_b431, 0x5904_b432))
            .expect("owned oversized-prefix prism should parse");
    let owned = StaticTableTranslator::from_compact_dictionary(
        TableDictionary::new(entries.clone()),
        Some(owned_prism),
    )
    .with_spelling_algebra(&[])
    .with_prefix_fallback(true)
    .with_sentence(false);
    let byte = compact_prefix_fallback_test_translator(entries, &[]);
    let input = "a".repeat(66);

    for (storage, translator) in [("owned", owned), ("byte", byte)] {
        let complete = translator.translate(&input);
        crate::m37_metrics_enable(true);
        crate::m37_metrics_reset();
        let bounded = translator.translate_with_context_and_request(
            &input,
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(4),
        );
        let metrics = crate::m37_metrics_snapshot();
        crate::m37_metrics_enable(false);

        assert_eq!(
            candidate_shape_without_quality(bounded.candidates),
            candidate_shape_without_quality(complete.into_iter().take(4)),
            "{storage}: bounded output must finish the longest consumed-span group before admitting shorter prefixes"
        );
        assert!(
            metrics.prefix_fallback_views_visited <= 140,
            "{storage}: the exact current-head merge should inspect one bounded head per accessor, not every accessor tail: {metrics:?}"
        );
    }
}

#[test]
fn prefix_fallback_bounding_is_selected_by_profile_policy_not_schema_identity() {
    let longest = "a".repeat(65);
    let entries = [
        TableEntry::new(&longest, "TOP1", 300.0),
        TableEntry::new(&longest, "TOP2", 299.0),
        TableEntry::new(&longest, "TOP3", 298.0),
        TableEntry::new(&longest, "TOP4", 297.0),
        TableEntry::new("a".repeat(64), "LOW64", 64.0),
        TableEntry::new("a".repeat(63), "LOW63", 63.0),
    ];
    let build = |policy, prediction_limit| {
        let translator = compact_prefix_fallback_test_translator(entries.clone(), &[])
            .with_sentence_policy(policy);
        if prediction_limit {
            translator.with_prediction_candidate_limit(1)
        } else {
            translator
        }
    };
    let standard = build(SentencePolicy::UpstreamScript, false);
    let profile = build(SentencePolicy::LegacyFallback, true);
    let legacy_without_profile_marker = build(SentencePolicy::LegacyFallback, false);
    let input = "a".repeat(66);
    let complete = standard.translate(&input);

    for (label, translator) in [
        ("upstream-script", &standard),
        (
            "legacy-without-prediction-marker",
            &legacy_without_profile_marker,
        ),
    ] {
        let bounded = translator.translate_with_context_and_request(
            &input,
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            CandidateRequest::bounded(4),
        );
        assert_eq!(
            bounded.candidates,
            complete[..4],
            "{label}: canonical exact current-head paging must not depend on a schema id"
        );
    }

    let profile_bounded = profile.translate_with_context_and_request(
        &input,
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(4),
    );
    assert_eq!(
        profile_bounded
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["TOP1", "TOP2", "LOW64", "LOW63"],
        "only the explicit LegacyFallback plus prediction marker retains TypeDuck's page-bounded long-input policy"
    );
    assert_eq!(
        profile.translate(&input),
        complete,
        "complete/page-turn collection remains exact even for the bounded profile policy"
    );
}

#[test]
fn bounded_prefix_fallback_reaches_valid_rows_after_more_than_64_filtered_aliases() {
    let _guard = super::m37_metrics_test_guard();
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let filtered = (0..80).map(|index| {
        TableEntry::new(
            format!("a{index:03}"),
            format!("\u{3400}FILTERED{index:03}"),
            400.0 - index as f32,
        )
    });
    let visible = (0..20).map(|index| {
        TableEntry::new(
            format!("az{index:03}"),
            format!("VISIBLE{index:03}"),
            200.0 - index as f32,
        )
    });
    let translator = compact_prefix_fallback_test_translator(filtered.chain(visible), &formulas);
    let request = |limit: CandidateRequest| {
        translator.translate_with_context_and_request(
            "ax",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            limit.with_filter_extended_cjk(true),
        )
    };

    let complete = request(CandidateRequest::unbounded());
    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let bounded = request(CandidateRequest::bounded(10));
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(complete.candidates.len(), 20);
    assert_eq!(
        candidate_shape_without_quality(bounded.candidates.clone()),
        candidate_shape_without_quality(complete.candidates[..10].iter().cloned())
    );
    assert!(!bounded.is_complete);
    assert!(
        metrics.lookup_views_visited <= 128,
        "the 100-row first pass plus ten locator replays must stay bounded; eager descriptor revalidation would exceed this budget: {metrics:?}"
    );
    assert!(
        metrics.prefix_fallback_views_visited <= 40,
        "the bounded collector must stop on its candidate window, not materialize every alias: {metrics:?}"
    );
    assert_eq!(prefix_fallback_cache_owner_snapshot(&translator), (0, 0));
}

#[test]
fn oversized_prefix_fallback_payload_is_request_local() {
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let entries = (b'b'..=b'u')
        .map(char::from)
        .enumerate()
        .map(|(index, suffix)| {
            TableEntry::new(
                format!("a{suffix}"),
                format!("ROW{index:02}{}", "x".repeat(32 * 1024)),
                200.0 - index as f32,
            )
        })
        .collect::<Vec<_>>();
    let translator = compact_prefix_fallback_test_translator(entries, &formulas);
    let complete = translator.translate("az");

    let bounded = translator.translate_with_context_and_request(
        "az",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(10),
    );

    let bounded_len = bounded.candidates.len();
    assert_eq!(
        candidate_shape_without_quality(bounded.candidates),
        candidate_shape_without_quality(complete.into_iter().take(bounded_len))
    );
    assert_eq!(prefix_fallback_cache_owner_snapshot(&translator), (0, 0));
}

#[test]
fn concurrent_prefix_fallback_cache_hits_and_replacements_preserve_results() {
    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let entries = [b'a', b'b']
        .into_iter()
        .flat_map(|initial| {
            (b'a'..=b'y').map(move |suffix| {
                let index = usize::from(suffix - b'a');
                TableEntry::new(
                    format!("{}{}", char::from(initial), char::from(suffix)),
                    format!("{}ROW{index:02}", char::from(initial).to_ascii_uppercase()),
                    200.0 - index as f32,
                )
            })
        })
        .collect::<Vec<_>>();
    let translator = Arc::new(compact_prefix_fallback_test_translator(entries, &formulas));
    let expected_a = translator
        .translate("az")
        .into_iter()
        .take(10)
        .map(|candidate| (candidate.text, candidate.comment, candidate.source))
        .collect::<Vec<_>>();
    let expected_b = translator
        .translate("bz")
        .into_iter()
        .take(10)
        .map(|candidate| (candidate.text, candidate.comment, candidate.source))
        .collect::<Vec<_>>();

    std::thread::scope(|scope| {
        let handles = (0..16)
            .map(|index| {
                let translator = Arc::clone(&translator);
                let (input, expected) = if index % 3 == 0 {
                    ("bz", &expected_b)
                } else {
                    ("az", &expected_a)
                };
                scope.spawn(move || {
                    let result = translator.translate_with_context_and_request(
                        input,
                        &Status::default(),
                        &HashMap::new(),
                        &Context::default(),
                        CandidateRequest::bounded(10),
                    );
                    let shape = result
                        .candidates
                        .into_iter()
                        .map(|candidate| (candidate.text, candidate.comment, candidate.source))
                        .collect::<Vec<_>>();
                    assert_eq!(&shape, expected);
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            handle
                .join()
                .expect("prefix fallback cache worker panicked");
        }
    });

    let (bytes, items) = prefix_fallback_cache_owner_snapshot(&translator);
    assert!((1..=128).contains(&items));
    assert!((1..=512 * 1024).contains(&bytes));
}

#[test]
fn heap_and_unbounded_prefix_fallback_leave_the_cache_empty() {
    let heap = StaticTableTranslator::new([("aa", "A"), ("ab", "B")])
        .with_prefix_fallback(true)
        .with_sentence(false);
    let _ = heap.translate_with_context_and_request(
        "aaz",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        CandidateRequest::bounded(1),
    );
    assert_eq!(prefix_fallback_cache_owner_snapshot(&heap), (0, 0));

    let formulas = vec!["abbrev/^([a-z]).+$/$1/".to_owned()];
    let compact = compact_prefix_fallback_test_translator(
        [
            TableEntry::new("aa", "A", 2.0),
            TableEntry::new("ab", "B", 1.0),
        ],
        &formulas,
    );
    let _ = compact.translate("az");
    assert_eq!(prefix_fallback_cache_owner_snapshot(&compact), (0, 0));
}

#[test]
fn bounded_request_uses_full_list_when_sentence_and_prefix_fallback_must_merge() {
    let _guard = super::m37_metrics_test_guard();
    let translator = StaticTableTranslator::new([("ab", "A"), ("cd", "B")])
        .with_sentence(true)
        .with_prefix_fallback(true);
    let context = Context::default();

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let result = translator.translate_with_context_and_request(
        "abcd",
        &Status::default(),
        &HashMap::new(),
        &context,
        CandidateRequest::bounded(1),
    );
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert!(result.is_complete);
    assert_eq!(result.candidates[0].text, "AB");
    assert_eq!(metrics.full_list_fallback_count, 1);
}

#[test]
fn punctuation_translator_keeps_digit_separator_literal_for_punct_number() {
    let mut engine = Engine::new();
    engine.add_translator(
        PunctuationTranslator::with_shape_entries([(".", "。")], [(".", "。")])
            .with_required_tags(["punct", "punct_number"]),
    );
    engine.set_segment_tags(["punct_number"]);

    engine.process_char('.');
    assert_eq!(engine.context().candidates[0].text, ".");
    assert_eq!(engine.context().candidates[0].comment, "〔半角〕");

    engine.set_option("full_shape", true);
    assert_eq!(engine.context().candidates[0].text, "．");
    assert_eq!(engine.context().candidates[0].comment, "〔全角〕");
}

const SORT_ORIGINAL_TEST_DICTIONARY: &str = r#"
---
name: original_order
version: "0.1"
sort: original
...

first	na	1
second	na	9
"#;

fn sort_original_test_dictionary() -> TableDictionary {
    TableDictionary::parse_rime_dict_yaml(SORT_ORIGINAL_TEST_DICTIONARY)
        .expect("dictionary should parse")
}

fn assert_na_candidate_texts(translator: &StaticTableTranslator, expected: &[&str]) {
    let texts = translator
        .translate("na")
        .into_iter()
        .map(|candidate| candidate.text)
        .collect::<Vec<_>>();
    assert_eq!(texts, expected);
}

#[test]
fn sort_original_dictionary_preserves_source_order_over_weight() {
    // GPT review P1 (2026-07-09): `sort: original` is a RIME contract — the dict
    // author's row order IS the ranking, and librime preserves it regardless of
    // weights. The tone-merge detector must not re-rank these exacts.
    let translator = StaticTableTranslator::parse_rime_dict_yaml(
        r#"
---
name: original_order
version: "0.1"
sort: original
...

first	na	1
second	na	9
"#,
    )
    .expect("dictionary should parse");
    let candidates = translator.translate("na");
    let texts: Vec<&str> = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect();
    assert_eq!(
        texts,
        ["first", "second"],
        "sort: original must keep source row order even when weights ascend"
    );
}

#[test]
fn sort_original_direct_compact_preserves_source_order_over_weight() {
    let translator =
        StaticTableTranslator::from_compact_dictionary(sort_original_test_dictionary(), None);
    assert_na_candidate_texts(&translator, &["first", "second"]);
}

#[test]
fn sort_original_owned_compact_store_preserves_source_order_over_weight() {
    let store = CompactTableStore::from_dictionary(sort_original_test_dictionary());
    assert_eq!(store.storage_label(), "owned_heap");
    let translator = StaticTableTranslator::from_compact_table_store(store, None);
    assert_na_candidate_texts(&translator, &["first", "second"]);
}

#[test]
fn sort_original_materialized_compiled_reload_preserves_source_order_over_weight() {
    let table_bytes = build_table_bin(&sort_original_test_dictionary(), 0x1234_5678);
    let relative = i32::from_le_bytes(
        table_bytes[44..48]
            .try_into()
            .expect("syllabary offset should fit"),
    );
    let syllabary_offset = usize::try_from(44isize + relative as isize)
        .expect("syllabary offset should be nonnegative");
    assert_eq!(syllabary_offset, 96);
    assert_eq!(syllabary_offset % 4, 0);
    assert_eq!(
        &table_bytes[68..syllabary_offset],
        b"YUNE-TABLE-META\0\x02\x00\x00\x00\x04\x00\x00\x00\x01\x01\x00\x00"
    );
    let reloaded =
        parse_rime_table_bin_dictionary(table_bytes).expect("compiled table should parse");
    assert!(!reloaded.sort_by_weight());
    let translator = StaticTableTranslator::from_dictionary(reloaded);
    assert_na_candidate_texts(&translator, &["first", "second"]);
}

#[test]
fn sort_original_byte_backed_compiled_reload_preserves_source_order_over_weight() {
    let table_bytes = build_table_bin(&sort_original_test_dictionary(), 0x1234_5678);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("compiled advanced data should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("compiled table store should parse");
    assert_eq!(store.storage_label(), "byte_backed");
    assert!(!store.sort_by_weight());
    let translator = StaticTableTranslator::from_compact_table_store(store, None);
    assert_na_candidate_texts(&translator, &["first", "second"]);
}

#[test]
fn sort_by_weight_compiled_path_keeps_default_order_and_v2_weight_metadata() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        &SORT_ORIGINAL_TEST_DICTIONARY.replace("sort: original", "sort: by_weight"),
    )
    .expect("dictionary should parse");
    let table_bytes = build_table_bin(&dictionary, 0x1234_5678);
    let relative = i32::from_le_bytes(
        table_bytes[44..48]
            .try_into()
            .expect("syllabary offset should fit"),
    );
    assert_eq!(44isize + relative as isize, 96);
    assert!(table_bytes
        .windows(b"YUNE-TABLE-META\0".len())
        .any(|window| window == b"YUNE-TABLE-META\0"));

    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("compiled advanced data should parse");
    assert_eq!(advanced.sort_by_weight, Some(true));
    let reloaded =
        parse_rime_table_bin_dictionary(table_bytes).expect("compiled table should parse");
    assert!(reloaded.sort_by_weight());
    let translator = StaticTableTranslator::from_dictionary(reloaded);
    assert_na_candidate_texts(&translator, &["second", "first"]);
}

#[test]
fn upstream_table_sentence_translation_keeps_one_poet_sentence_then_real_prefix_rows() {
    const INPUT: &str = "takohaeosk";
    const TARGET: &str = "\u{83ab}\u{4f2f}\u{6d22}";
    const MODEL_ONLY_PREFIX: &str = "\u{83ab}\u{5165}";
    let dictionary = TableDictionary::new([
        TableEntry::new("tak", "\u{83ab}", 1_000.0),
        TableEntry::new("oha", "\u{4f2f}", 1_000.0),
        TableEntry::new("eosk", "\u{6d22}", 1_000.0),
        TableEntry::new("oh", "\u{5165}", 900.0),
        TableEntry::new("ta", "\u{6614}", 800.0),
        TableEntry::new("t", "\u{5eff}", 700.0),
    ]);
    let vocabulary = vec![PresetVocabularyEntry::new(MODEL_ONLY_PREFIX, 10_000.0)];
    let configure = |translator: StaticTableTranslator, policy| {
        translator
            .with_completion(true)
            .with_sentence(true)
            .with_sentence_policy(policy)
            .with_preset_vocabulary(vocabulary.clone())
            .with_upstream_sentence_model(100)
    };

    let legacy = configure(
        StaticTableTranslator::from_dictionary(dictionary.clone()),
        SentencePolicy::LegacyFallback,
    );
    assert!(
        legacy
            .translate(INPUT)
            .iter()
            .any(|candidate| candidate.text == MODEL_ONLY_PREFIX),
        "the control must prove Poet reconstructed a phrase which is absent from the table"
    );

    let source = configure(
        StaticTableTranslator::from_dictionary(dictionary.clone()),
        SentencePolicy::UpstreamTable,
    );
    let table_bytes = build_table_bin(&dictionary, 0x5904_d402);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("upstream-table sentence fixture metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("upstream-table sentence byte-backed store should parse");
    let byte_backed = configure(
        StaticTableTranslator::from_compact_table_store(store, None),
        SentencePolicy::UpstreamTable,
    );

    let expected = [
        (TARGET, CandidateSource::Sentence),
        (
            "\u{83ab}",
            CandidateSource::PartialTable {
                consumed: 3,
                recompose_on_default: false,
            },
        ),
        (
            "\u{6614}",
            CandidateSource::PartialTable {
                consumed: 2,
                recompose_on_default: false,
            },
        ),
        (
            "\u{5eff}",
            CandidateSource::PartialTable {
                consumed: 1,
                recompose_on_default: false,
            },
        ),
    ];
    for (storage, translator) in [("source", source), ("byte-backed", byte_backed)] {
        let complete = translator.translate(INPUT);
        assert_eq!(
            complete
                .iter()
                .map(|candidate| (candidate.text.as_str(), candidate.source.clone()))
                .collect::<Vec<_>>(),
            expected,
            "{storage}: one best sentence must be followed only by real longest-first prefix rows"
        );
        assert!(
            complete
                .iter()
                .all(|candidate| candidate.text != MODEL_ONLY_PREFIX),
            "{storage}: model-only prefix phrases must not leak into TableTranslator output"
        );

        for limit in 1..expected.len() {
            let bounded = translator.translate_with_context_and_request(
                INPUT,
                &Status::default(),
                &HashMap::new(),
                &Context::default(),
                CandidateRequest::bounded(limit).with_debug_full_count(true),
            );
            assert_eq!(
                bounded.candidates,
                complete[..limit],
                "{storage}: bounded sentence translation must be a field-identical complete prefix at {limit}"
            );
            assert!(
                bounded.full_count.is_some_and(|count| count > limit),
                "{storage}: bounded sentence translation must retain a tail witness"
            );
        }
    }
}

fn upstream_table_lazy_fixture_dictionary() -> TableDictionary {
    TableDictionary::new([
        TableEntry::new("q", "exact", 500.0),
        TableEntry::new("qa", "a", 90.0),
        // CharsetFilter hides this row, but LazyTableTranslation's raw
        // entry_count and the 10->100 Skip boundary must still count it.
        TableEntry::new("qa", "\u{20000}", 85.0),
        TableEntry::new("qb", "b", 80.0),
        TableEntry::new("qc", "c", 70.0),
        TableEntry::new("qd", "d", 60.0),
        TableEntry::new("qe", "e", 50.0),
        TableEntry::new("qf", "f", 40.0),
        TableEntry::new("qg", "g", 30.0),
        TableEntry::new("qh", "h", 20.0),
        TableEntry::new("qi", "i", 10.0),
        // These keys first appear after the 10-key stage. The first row after
        // raw Skip is deliberately lower-weight than its sibling: librime
        // exposes j before sorting the remaining current heads and emitting k.
        TableEntry::new("qj", "j", 1.0),
        TableEntry::new("qk", "k", 1_000.0),
    ])
}

fn upstream_table_lazy_translator(translator: StaticTableTranslator) -> StaticTableTranslator {
    translator
        .with_completion(true)
        .with_sentence(true)
        .with_sentence_policy(SentencePolicy::UpstreamTable)
        .with_leading_syllable_reachability(true)
}

fn upstream_table_result(
    translator: &StaticTableTranslator,
    request: CandidateRequest,
) -> TranslationResult {
    translator.translate_with_context_and_request(
        "q",
        &Status::default(),
        &HashMap::new(),
        &Context::default(),
        request.with_filter_extended_cjk(true),
    )
}

#[test]
fn upstream_table_lazy_10_to_100_order_is_source_and_byte_backed_identical() {
    let dictionary = upstream_table_lazy_fixture_dictionary();
    let source =
        upstream_table_lazy_translator(StaticTableTranslator::from_dictionary(dictionary.clone()));

    let table_bytes = build_table_bin(&dictionary, 0x5904_d401);
    let advanced = parse_rime_table_bin_advanced_data(&table_bytes)
        .expect("upstream-table byte-backed metadata should parse");
    let store = CompactTableStore::from_table_bin_bytes(table_bytes, advanced)
        .expect("upstream-table byte-backed store should parse");
    let byte_backed = upstream_table_lazy_translator(
        StaticTableTranslator::from_compact_table_store(store, None),
    );

    let expected = [
        "exact", "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k",
    ];
    for (storage, translator) in [("source", source), ("byte-backed", byte_backed)] {
        let complete = upstream_table_result(&translator, CandidateRequest::unbounded());
        let complete_texts = complete
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(complete_texts, expected, "{storage}: complete lazy order");
        assert!(complete.is_complete, "{storage}: complete request");
        assert_eq!(
            complete.merge_policy,
            crate::TranslationMergePolicy::UpstreamTable,
            "{storage}: marked table result must request producer-head merge"
        );

        for limit in [10, 11] {
            let bounded = upstream_table_result(&translator, CandidateRequest::bounded(limit));
            let bounded_texts = bounded
                .candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>();
            assert_eq!(
                bounded_texts,
                expected[..limit],
                "{storage}: bounded output must be a complete-list prefix at limit {limit}"
            );
            assert!(
                !bounded.is_complete,
                "{storage}: tail remains at limit {limit}"
            );
        }
    }
}

#[test]
fn upstream_table_lazy_order_coexists_with_default_on_leading_reachability() {
    let dictionary = TableDictionary::new([
        TableEntry::new("q", "Q", 100.0),
        TableEntry::new("qa", "EXACT", 90.0),
        TableEntry::new("qab", "COMPLETION", 80.0),
    ]);
    let translator =
        upstream_table_lazy_translator(StaticTableTranslator::from_dictionary(dictionary));
    let translate = |request| {
        translator.translate_with_context_and_request(
            "qa",
            &Status::default(),
            &HashMap::new(),
            &Context::default(),
            request,
        )
    };

    let complete = translate(CandidateRequest::unbounded());
    let bounded = translate(CandidateRequest::bounded(3));
    let texts = |result: &TranslationResult| {
        result
            .candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(texts(&complete), ["EXACT", "COMPLETION", "Q"]);
    assert_eq!(texts(&bounded), texts(&complete));
    assert_eq!(bounded.merge_policy, TranslationMergePolicy::UpstreamTable);
}

#[test]
fn unmarked_table_does_not_activate_upstream_table_merge_policy() {
    let translator =
        StaticTableTranslator::from_dictionary(upstream_table_lazy_fixture_dictionary())
            .with_completion(true)
            .with_sentence(true);
    let result = upstream_table_result(&translator, CandidateRequest::bounded(12));
    let texts = result
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts.first().copied(), Some("exact"));
    assert_eq!(result.merge_policy, crate::TranslationMergePolicy::Default);
}
