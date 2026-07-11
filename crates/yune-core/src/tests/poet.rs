use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::poet::UpstreamSentenceScratch;
use crate::{
    build_poet_bin, encode_octagram_key, make_sentences, make_sentences_with_grammar,
    null_grammar_score, CandidateSource, DartsDoubleArray, Grammar, MemoryOwnerClass,
    OctagramGrammar, OctagramGrammarConfig, OctagramGrammarParseError, OwnedPoetBytes,
    PoetByteSource, PresetVocabularyEntry, SentenceCodeSpan, StaticTableTranslator,
    TableDictionary, TableEntry, Translator, UpstreamSentenceModel, WordGraph, WordGraphEntry,
    UPSTREAM_NO_GRAMMAR_PENALTY,
};

#[test]
fn null_grammar_score_applies_upstream_penalty() {
    assert!((UPSTREAM_NO_GRAMMAR_PENALTY - 1.0e-6_f64.ln()).abs() < f64::EPSILON);
    assert!((null_grammar_score(20.0) - (20.0 + UPSTREAM_NO_GRAMMAR_PENALTY)).abs() < f64::EPSILON);
}

#[test]
fn upstream_sentence_model_scales_raw_weights_like_librime_entries_before_null_grammar_scoring() {
    let entries = [
        TableEntry::new("a", "A", 20_000.0),
        TableEntry::new("b", "B", 20_000.0),
        TableEntry::new("a", "X", 30_000.0),
        TableEntry::new("b", "Y", 30_000.0),
    ];
    let vocabulary = [PresetVocabularyEntry::new("AB", 20_000.0)];
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10);

    let candidates = model.candidates_for_input("ab");

    assert_eq!(candidates[0].text, "AB");
    assert_eq!(candidates[0].source, CandidateSource::Table);

    let entries = [
        TableEntry::new("a", "A", 78_069.0),
        TableEntry::new("bc", "BC", 26_997.0),
        TableEntry::new("abc", "ABC", 1_679.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 10);

    let candidates = model.candidates_for_input("abc");

    assert_eq!(candidates[0].text, "ABC");
    assert_eq!(candidates[0].source, CandidateSource::Table);
}

#[test]
fn upstream_script_translation_emits_one_sentence_then_the_independent_phrase_stream() {
    let entries = [
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("a", "X", 90.0),
        TableEntry::new("b", "B", 100.0),
        TableEntry::new("c", "C", 100.0),
    ];
    let vocabulary = [
        PresetVocabularyEntry::new("AB", 1_000.0),
        PresetVocabularyEntry::new("XB", 900.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10)
        .with_script_translation_limits(1, 1);

    let candidates = model.candidates_for_input("abc");
    let actual = candidates
        .iter()
        .map(|candidate| (candidate.text.as_str(), candidate.source.clone()))
        .collect::<Vec<_>>();

    assert_eq!(
        actual,
        [
            ("ABC", CandidateSource::Sentence),
            (
                "AB",
                CandidateSource::PartialTable {
                    consumed: 2,
                    recompose_on_default: false,
                },
            ),
            (
                "XB",
                CandidateSource::PartialTable {
                    consumed: 2,
                    recompose_on_default: false,
                },
            ),
            (
                "A",
                CandidateSource::PartialTable {
                    consumed: 1,
                    recompose_on_default: false,
                },
            ),
            (
                "X",
                CandidateSource::PartialTable {
                    consumed: 1,
                    recompose_on_default: false,
                },
            ),
        ]
    );
}

#[test]
fn upstream_script_translation_reliable_full_phrase_suppresses_sentence_generation() {
    let entries = [
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("a", "X", 90.0),
        TableEntry::new("b", "B", 100.0),
    ];
    let vocabulary = [
        PresetVocabularyEntry::new("AB", 1_000.0),
        PresetVocabularyEntry::new("XB", 900.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10)
        .with_script_translation_limits(1, 1);

    let candidates = model.candidates_for_input("ab");

    assert_eq!(
        candidates
            .iter()
            .take(2)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["AB", "XB"]
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.source != CandidateSource::Sentence),
        "a reliable full exact phrase belongs to the phrase stream and suppresses Poet"
    );
}

#[test]
fn upstream_script_translation_applies_phrase_limit_after_text_deduplication() {
    let entries = [
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("a", "A", 90.0),
        TableEntry::new("a", "A", 80.0),
        TableEntry::new("a", "A", 70.0),
        TableEntry::new("a", "A", 60.0),
        TableEntry::new("a", "A", 50.0),
        TableEntry::new("a", "A", 40.0),
        TableEntry::new("a", "A", 30.0),
        TableEntry::new("a", "X", 20.0),
        TableEntry::new("b", "B", 100.0),
        TableEntry::new("c", "C", 100.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 3)
        .with_script_translation_limits(1, 1);
    let cold = model.candidates_for_input("abc");
    let mut scratch = UpstreamSentenceScratch::default();
    let incremental = model.candidates_for_input_with_limit_and_scratch("abc", 3, &mut scratch);

    for (path, candidates) in [("cold", cold), ("incremental", incremental)] {
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.text.as_str())
                .collect::<Vec<_>>(),
            ["ABC", "A", "X"],
            "{path} extraction must continue past duplicate rows to fill the visible page"
        );
    }
}

#[test]
fn singular_null_poet_keeps_first_traversed_equal_weight_path() {
    let mut graph = WordGraph::new();
    graph
        .entry(0)
        .or_default()
        .entry(1)
        .or_default()
        .push(WordGraphEntry::new("Z", 1.0));
    graph
        .entry(0)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("A", 1.0));
    graph
        .entry(1)
        .or_default()
        .entry(3)
        .or_default()
        .push(WordGraphEntry::new("Z", 1.0));
    graph
        .entry(2)
        .or_default()
        .entry(3)
        .or_default()
        .push(WordGraphEntry::new("A", 1.0));

    let sentences = crate::make_sentences(&graph, 3, 1);

    assert_eq!(sentences[0].text, "ZZ");
}

#[test]
fn owned_and_byte_backed_sentence_graphs_preserve_equal_weight_source_order() {
    let entries = vec![
        TableEntry::new("a", "Z", 100.0),
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("b", "X", 100.0),
    ];
    let owned = UpstreamSentenceModel::from_table_entries(entries.clone(), &[], 5);
    let checksum = 0x4551_5449;
    let byte_backed = UpstreamSentenceModel::from_poet_bin_source(
        Arc::new(TestMmapPoetBytes::new(build_poet_bin(
            entries,
            &[],
            &[],
            checksum,
        ))) as Arc<dyn PoetByteSource>,
        checksum,
        5,
    )
    .expect("byte-backed tie fixture should parse");

    for (storage, model) in [("owned", &owned), ("byte-backed", &byte_backed)] {
        let candidates = model.candidates_for_input("ab");
        assert_eq!(candidates[0].text, "ZX", "{storage}");
        assert_eq!(candidates[0].source, CandidateSource::Sentence, "{storage}");
    }
}

#[test]
fn singular_grammar_poet_retains_up_to_seven_last_word_states() {
    let mut graph = WordGraph::new();
    graph.entry(0).or_default().entry(1).or_default().extend([
        WordGraphEntry::new("A", 4.0),
        WordGraphEntry::new("B", 3.0),
        WordGraphEntry::new("C", 2.0),
        WordGraphEntry::new("D", 1.0),
    ]);
    graph
        .entry(1)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("X", 0.0));
    let grammar = ContextBoostGrammar {
        context: "D",
        word: "X",
        score: 100.0,
    };

    let sentences = crate::make_sentences_with_grammar(&graph, 2, 1, &grammar);

    assert_eq!(sentences[0].text, "DX");
}

#[test]
fn plural_poet_applies_default_and_accelerated_sentence_cutoff() {
    let grammar = RecordingGrammar::default();
    let graph_for = |weights: [f64; 3]| {
        let mut graph = WordGraph::new();
        graph.entry(0).or_default().entry(1).or_default().extend([
            WordGraphEntry::new("A", weights[0]),
            WordGraphEntry::new("B", weights[1]),
            WordGraphEntry::new("C", weights[2]),
        ]);
        graph
            .entry(1)
            .or_default()
            .entry(2)
            .or_default()
            .push(WordGraphEntry::new("X", 0.0));
        graph
    };

    let default_cutoff =
        crate::make_sentences_with_grammar(&graph_for([100.0, 80.0, 70.0]), 2, 3, &grammar);
    assert_eq!(
        default_cutoff
            .iter()
            .map(|sentence| sentence.text.as_str())
            .collect::<Vec<_>>(),
        ["AX"]
    );

    let accelerated =
        crate::make_sentences_with_grammar(&graph_for([100.0, 95.0, 88.0]), 2, 3, &grammar);
    assert_eq!(
        accelerated
            .iter()
            .map(|sentence| sentence.text.as_str())
            .collect::<Vec<_>>(),
        ["AX", "BX"],
        "the third row is within 10% of the second but outside the accelerated 6.67% cutoff"
    );
}

#[test]
fn script_encoder_uses_exact_five_percent_boundary_and_ignores_duplicate_codes() {
    let entries = vec![
        TableEntry::new("chang", "長", 100.0),
        TableEntry::new("ju", "句", 100.0),
        TableEntry::new("ju", "足", 5.0),
        TableEntry::new("zhu", "朱", 100.0),
        TableEntry::new("zhu", "足", 1.0),
        TableEntry::new("zu", "足", 94.0),
        TableEntry::new("zu", "足", 1_000.0),
    ];
    let vocabulary = vec![PresetVocabularyEntry::new("長足", 989.0)];
    let owned = UpstreamSentenceModel::from_table_entries(entries.clone(), &vocabulary, 10);
    let checksum = 0x5350_3543;
    let bytes = build_poet_bin(entries, &vocabulary, &vocabulary, checksum);
    let byte_backed = UpstreamSentenceModel::from_poet_bin_source(
        Arc::new(TestMmapPoetBytes::new(bytes)) as Arc<dyn PoetByteSource>,
        checksum,
        10,
    )
    .expect("byte-backed ScriptEncoder model should parse");
    let octagram = || {
        OctagramGrammar::from_bytes(
            &synthetic_octagram_gram(&[("長足", 1)]),
            OctagramGrammarConfig::default(),
        )
        .expect("synthetic octagram should parse")
    };
    let owned_octagram = owned.clone().with_grammar(octagram());
    let byte_backed_octagram = byte_backed.clone().with_grammar(octagram());

    for (storage, model) in [
        ("owned-null", &owned),
        ("byte-backed-null", &byte_backed),
        ("owned-octagram", &owned_octagram),
        ("byte-backed-octagram", &byte_backed_octagram),
    ] {
        let boundary = model.candidates_for_input("changju");
        assert_eq!(boundary[0].text, "長足", "{storage}");
        assert!(
            boundary.iter().any(|candidate| candidate.text == "長足"),
            "{storage} must admit 足/ju at exactly 5% of unique-code total weight"
        );

        let below = model.candidates_for_input("changzhu");
        assert!(
            below.iter().all(|candidate| candidate.text != "長足"),
            "{storage} must exclude 足/zhu below 5%; the duplicate 足/zu row must not inflate the total"
        );
    }
}

#[test]
fn upstream_script_translation_limits_sentence_homophones_independently_from_phrase_rows() {
    let entries = [
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("a", "X", 90.0),
        TableEntry::new("b", "B", 100.0),
        TableEntry::new("c", "C", 100.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 10)
        .with_script_translation_limits(2, 1);

    let candidates = model.candidates_for_input("abc");

    assert_eq!(candidates[0].text, "ABC");
    assert_eq!(candidates[0].source, CandidateSource::Sentence);
    assert_eq!(candidates[1].text, "A");
    assert_eq!(candidates[2].text, "X");
    assert_eq!(
        candidates
            .iter()
            .filter(|candidate| candidate.source == CandidateSource::Sentence)
            .count(),
        1,
        "max_homophones=1 excludes X from the sentence graph without hiding it from phrases"
    );

    let model = UpstreamSentenceModel::from_table_entries(
        [
            TableEntry::new("a", "A", 100.0),
            TableEntry::new("a", "X", 90.0),
            TableEntry::new("b", "B", 100.0),
            TableEntry::new("c", "C", 100.0),
        ],
        &[],
        10,
    )
    .with_script_translation_limits(2, 2);
    let candidates = model.candidates_for_input("abc");
    assert_eq!(
        candidates
            .iter()
            .filter(|candidate| candidate.source == CandidateSource::Sentence)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["ABC", "XBC"],
        "an explicit plural sentence configuration preserves the upstream beam surface"
    );
}

#[test]
fn make_sentences_keeps_weight_ordered_beam() {
    let mut graph = WordGraph::new();
    graph.entry(0).or_default().entry(2).or_default().extend([
        WordGraphEntry::new("A", 100.0),
        WordGraphEntry::new("X", 99.0),
    ]);
    graph.entry(2).or_default().entry(4).or_default().extend([
        WordGraphEntry::new("B", 99.0),
        WordGraphEntry::new("Y", 97.0),
    ]);

    let sentences = make_sentences(&graph, 4, 3)
        .into_iter()
        .map(|sentence| sentence.text)
        .collect::<Vec<_>>();

    assert_eq!(sentences, ["AB", "XB", "AY"]);
}

#[test]
fn make_sentence_excludes_the_direct_whole_input_word() {
    let mut graph = WordGraph::new();
    graph
        .entry(0)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("WHOLE", 1_000.0));
    graph
        .entry(0)
        .or_default()
        .entry(1)
        .or_default()
        .push(WordGraphEntry::new("A", 10.0));
    graph
        .entry(1)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("B", 10.0));

    let sentence = crate::make_sentence(&graph, 2).expect("composed path should exist");

    assert_eq!(sentence.text, "AB");
}

#[test]
fn upstream_sentence_model_orders_longest_code_before_weight() {
    let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        "\
---
name: sentence_model
version: '1'
sort: by_weight
use_preset_vocabulary: true
...

A\ta\t1000
B\tb\t900
C\tc\t1
",
        std::iter::empty::<&str>(),
        |_| None,
        |name| {
            (name == "essay").then(|| {
                "\
AB\t10
AC\t20
A\t1000
"
                .to_owned()
            })
        },
    )
    .expect("dictionary should parse");
    let translator =
        StaticTableTranslator::from_dictionary(dictionary).with_upstream_sentence_model(10);
    let candidates = translator.translate("abx");

    assert_eq!(candidates[0].text, "AB");
    assert_eq!(
        candidates[0].source,
        CandidateSource::PartialTable {
            consumed: 2,
            recompose_on_default: false,
        }
    );
    assert_eq!(candidates[0].commit_text_for_input("abx"), "ABx");
    assert_eq!(candidates[1].text, "A");
}

#[test]
fn upstream_sentence_model_uses_indexed_code_prefix_scan() {
    let _guard = super::m37_metrics_test_guard();
    let mut source = "\
---
name: indexed_sentence_model
version: '1'
sort: by_weight
...

A\tab\t1000
B\tcd\t1000
C\tef\t1000
"
    .to_owned();
    for index in 0..1000 {
        source.push_str(&format!("F{index}\tq{index}\t1\n"));
    }
    let dictionary =
        TableDictionary::parse_rime_dict_yaml(&source).expect("dictionary should parse");
    let model = UpstreamSentenceModel::from_dictionary(&dictionary, 10);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let candidates = model.candidates_for_input("abcdef");
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(candidates[0].text, "ABC");
    assert!(metrics.upstream_sentence_model_code_prefix_checks <= 21);
    assert!(metrics.upstream_sentence_model_table_entries_considered <= 3);
}

#[test]
fn upstream_sentence_model_prefilters_irrelevant_vocabulary_codes() {
    let _guard = super::m37_metrics_test_guard();
    let entries = [
        TableEntry::new("a", "A", 1000.0),
        TableEntry::new("b", "B", 1000.0),
        TableEntry::new("x", "X", 1000.0),
    ];
    let mut vocabulary = vec![PresetVocabularyEntry::new("AB", 1000.0)];
    for index in 0..1000 {
        vocabulary.push(PresetVocabularyEntry::new(format!("AX{index}"), 1.0));
    }
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10);

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let candidates = model.candidates_for_input("ab");
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(candidates[0].text, "AB");
    assert!(
        metrics.upstream_sentence_model_vocabulary_entries_considered <= 1,
        "sentence model should skip preset vocabulary entries whose following character codes cannot match the input: {metrics:?}"
    );
}

#[test]
fn upstream_sentence_model_memory_profile_accounts_packed_entries() {
    let repeated_code = "sharedsentencemodelcode".repeat(4);
    let entries = (0..64)
        .map(|index| TableEntry::new(&repeated_code, format!("phrase-{index:02}"), 100.0))
        .collect::<Vec<_>>();
    let old_owned_shape_lower_bound = std::mem::size_of::<Vec<TableEntry>>()
        + entries.len() * (std::mem::size_of::<String>() * 2 + std::mem::size_of::<f32>())
        + entries
            .iter()
            .map(|entry| entry.code.len() + entry.text.len())
            .sum::<usize>();

    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 10);
    let owner = model
        .memory_owner_rows()
        .into_iter()
        .find(|row| row.owner == "poet.entries_by_code")
        .expect("sentence model entry owner should be reported");

    assert_eq!(owner.item_count, 64);
    assert_eq!(owner.storage, "Vec<ModelEntry>");
    assert!(
        owner.estimated_bytes < old_owned_shape_lower_bound,
        "packed owner {} should stay below old string-heavy shape {}",
        owner.estimated_bytes,
        old_owned_shape_lower_bound
    );
}

#[test]
fn upstream_sentence_model_memory_profile_accounts_normal_vocabulary_without_prefix_index() {
    let entries = [
        TableEntry::new("a", "A", 1000.0),
        TableEntry::new("b", "B", 1000.0),
        TableEntry::new("x", "X", 1000.0),
    ];
    let vocabulary = std::iter::once(PresetVocabularyEntry::new("AB", 1000.0))
        .chain((0..128).map(|index| PresetVocabularyEntry::new(format!("AX{index}"), 1.0)))
        .collect::<Vec<_>>();
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10);
    let owners = model.memory_owner_rows();

    assert!(
        owners.iter().all(|row| !row.owner.contains("prefix_index")),
        "normal vocabulary should not reintroduce a retained prefix index: {owners:?}"
    );

    let owner = owners
        .iter()
        .find(|row| row.owner == "poet.vocabulary")
        .expect("normal sentence vocabulary owner should be reported");
    assert_eq!(owner.item_count, vocabulary.len());
    assert_eq!(owner.storage, "Vec<ModelVocabularyEntry>");
    assert!(
        owner.estimated_bytes < 1_000_000,
        "synthetic vocabulary owner should stay small without a retained prefix index: {}",
        owner.estimated_bytes
    );
}

#[test]
fn upstream_sentence_model_memory_profile_accounts_octagram_grammar_separately() {
    let entries = [
        TableEntry::new("a", "A", 1000.0),
        TableEntry::new("b", "B", 1000.0),
    ];
    let vocabulary = [PresetVocabularyEntry::new("AB", 1000.0)];

    let null_model = UpstreamSentenceModel::from_table_entries(entries.clone(), &vocabulary, 10);
    assert!(
        null_model
            .memory_owner_rows()
            .iter()
            .all(|row| row.owner != "poet.octagram_double_array"),
        "null grammar should not report octagram retained bytes"
    );

    let grammar = OctagramGrammar::from_bytes(
        &synthetic_octagram_gram(&[("AB", 42)]),
        OctagramGrammarConfig::default(),
    )
    .expect("synthetic octagram grammar should parse");
    let octagram_model =
        UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10).with_grammar(grammar);
    let owner = octagram_model
        .memory_owner_rows()
        .into_iter()
        .find(|row| row.owner == "poet.octagram_double_array")
        .expect("octagram grammar owner should be reported separately");

    assert_eq!(owner.class, crate::MemoryOwnerClass::HeapOwnedReducible);
    assert_eq!(owner.storage, "DartsDoubleArray");
    assert!(owner.item_count > 0);
    assert!(owner.estimated_bytes > 0);
}

#[test]
fn upstream_sentence_scratch_is_not_used_for_octagram_grammar_model() {
    let _guard = super::m37_metrics_test_guard();
    let entries = [
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("b", "B", 100.0),
        TableEntry::new("c", "C", 100.0),
    ];
    let grammar = OctagramGrammar::from_bytes(
        &synthetic_octagram_gram(&[("AB", 42), ("ABC$", 99)]),
        OctagramGrammarConfig::default(),
    )
    .expect("synthetic octagram grammar should parse");
    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 10).with_grammar(grammar);
    let mut scratch = UpstreamSentenceScratch::default();

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let first = model.candidates_for_input_with_limit_and_scratch("ab", 5, &mut scratch);
    let second = model.candidates_for_input_with_limit_and_scratch("abc", 5, &mut scratch);
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(first[0].text, "AB");
    assert_eq!(second[0].text, "ABC");
    assert_eq!(metrics.upstream_sentence_model_incremental_reuse_hits, 0);
    assert_eq!(metrics.upstream_sentence_model_graph_rebuild_calls, 2);
}

#[test]
fn upstream_sentence_model_reads_candidates_from_byte_backed_poet_artifact() {
    let entries = vec![
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("b", "B", 100.0),
        TableEntry::new("c", "C", 100.0),
        TableEntry::new("d", "D", 100.0),
        TableEntry::new("w", "W", 100.0),
        TableEntry::new("x", "X", 100.0),
        TableEntry::new("y", "Y", 100.0),
        TableEntry::new("z", "Z", 100.0),
    ];
    let vocabulary = vec![
        PresetVocabularyEntry::new("AB", 1_000_000.0),
        PresetVocabularyEntry::new("CD", 10.0),
    ];
    let abbreviation_vocabulary = vec![
        PresetVocabularyEntry::new("WXYZ", 1_000_000.0),
        PresetVocabularyEntry::new("WX", 10.0),
    ];
    let checksum = 0x1234_5678;
    let heap_model = UpstreamSentenceModel::from_table_entries_with_abbreviation_vocabulary(
        entries.clone(),
        &vocabulary,
        &abbreviation_vocabulary,
        10,
    );
    let bytes = build_poet_bin(entries, &vocabulary, &abbreviation_vocabulary, checksum);
    let byte_model = UpstreamSentenceModel::from_poet_bin_source(
        Arc::new(TestMmapPoetBytes::new(bytes)) as Arc<dyn PoetByteSource>,
        checksum,
        10,
    )
    .expect("byte-backed poet artifact should load");

    assert_eq!(
        byte_model.candidates_for_input("ab"),
        heap_model.candidates_for_input("ab")
    );
    let spans = [
        SentenceCodeSpan::new(0, 1, "w"),
        SentenceCodeSpan::new(1, 2, "x"),
        SentenceCodeSpan::new(2, 3, "y"),
        SentenceCodeSpan::new(3, 4, "z"),
    ];
    assert_eq!(
        byte_model.candidates_for_code_spans_with_limit("wxyz", &spans, 5),
        heap_model.candidates_for_code_spans_with_limit("wxyz", &spans, 5)
    );

    let owners = byte_model.memory_owner_rows();
    let owner = |name: &str| {
        owners
            .iter()
            .find(|row| row.owner == name)
            .unwrap_or_else(|| panic!("missing memory owner {name}: {owners:?}"))
    };
    assert_eq!(
        owner("poet.entries_by_code").class,
        MemoryOwnerClass::MmapFileBacked
    );
    assert_eq!(
        owner("poet.vocabulary").class,
        MemoryOwnerClass::MmapFileBacked
    );
    assert_eq!(
        owner("poet.abbreviation_vocabulary").class,
        MemoryOwnerClass::MmapFileBacked
    );
    assert!(
        owners.iter().all(|row| row.owner != "poet.lookup_index"),
        "byte-backed YUNE-POET/3 must use compiled index storage, not a retained heap lookup index: {owners:?}"
    );
    assert_eq!(
        owner("poet.prefix_index").class,
        MemoryOwnerClass::MmapFileBacked
    );
}

#[test]
fn upstream_sentence_model_rejects_stale_poet_artifact_checksum() {
    let bytes = build_poet_bin([TableEntry::new("a", "A", 100.0)], &[], &[], 0x1234_5678);
    let result = UpstreamSentenceModel::from_poet_bin_source(
        Arc::new(OwnedPoetBytes::new(bytes)) as Arc<dyn PoetByteSource>,
        0x8765_4321,
        10,
    );

    assert!(result.is_err());
}

#[test]
fn upstream_sentence_model_records_m40_lookup_index_counters() {
    let _guard = super::m37_metrics_test_guard();
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        "\
---
name: m40_sentence_index_metrics
version: '1'
sort: by_weight
...

A\tab\t1000
B\tcd\t1000
C\tef\t1000
Alt\tab\t900
",
    )
    .expect("dictionary should parse");

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let model = UpstreamSentenceModel::from_dictionary(&dictionary, 10);
    let build_metrics = crate::m37_metrics_snapshot();
    assert!(build_metrics.upstream_sentence_model_index_build_calls >= 1);
    assert!(build_metrics.upstream_sentence_model_index_build_ns > 0);

    crate::m37_metrics_reset();
    let candidates = model.candidates_for_input("abcdefz");
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_reset();
    let reset_metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(candidates[0].text, "A");
    assert!(metrics.upstream_sentence_model_exact_range_index_hits >= 3);
    assert!(metrics.upstream_sentence_model_exact_range_index_misses >= 1);
    assert!(metrics.upstream_sentence_model_prefix_filter_hits >= 3);
    assert!(metrics.upstream_sentence_model_prefix_filter_misses >= 1);
    assert!(metrics.upstream_sentence_model_prefix_filter_early_breaks >= 1);
    assert!(metrics.upstream_sentence_model_vocabulary_index_probes >= 3);
    assert_eq!(metrics.upstream_sentence_model_vocabulary_rows_examined, 0);
    assert!(metrics.upstream_sentence_model_graph_entries_inserted >= 3);
    assert!(metrics.upstream_sentence_model_graph_entry_text_bytes >= 3);
    assert!(metrics.upstream_sentence_model_reachable_starts_visited >= 3);
    assert!(metrics.upstream_sentence_model_unreachable_starts_skipped >= 1);
    assert!(metrics.upstream_sentence_model_phrase_index_walk_calls >= 1);
    assert!(metrics.upstream_sentence_model_phrase_index_nodes_visited >= 3);
    assert!(metrics.upstream_sentence_model_phrase_index_entry_ranges_emitted >= 3);
    assert_eq!(
        metrics.upstream_sentence_model_partition_point_fallback_calls,
        0
    );
    assert!(metrics.upstream_sentence_model_graph_rebuild_calls >= 1);
    assert!(metrics.upstream_sentence_model_graph_rebuild_ns > 0);
    assert_eq!(metrics.upstream_sentence_model_incremental_reuse_hits, 0);
    assert!(metrics.upstream_sentence_model_dp_states_created >= 3);
    assert_eq!(metrics.upstream_sentence_model_dp_beam_evictions, 0);

    assert_eq!(
        reset_metrics.upstream_sentence_model_exact_range_index_hits,
        0
    );
    assert_eq!(
        reset_metrics.upstream_sentence_model_phrase_index_walk_calls,
        0
    );
    assert_eq!(
        reset_metrics.upstream_sentence_model_graph_entries_inserted,
        0
    );
    assert_eq!(reset_metrics.upstream_sentence_model_dp_states_created, 0);
    assert_eq!(reset_metrics.upstream_sentence_model_graph_rebuild_calls, 0);
}

#[test]
fn upstream_sentence_scratch_reuses_prefix_walk_state_for_single_key_extensions() {
    let _guard = super::m37_metrics_test_guard();
    let entries = [
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("ab", "AB", 100.0),
        TableEntry::new("abc", "ABC", 100.0),
        TableEntry::new("abcd", "ABCD", 100.0),
        TableEntry::new("b", "B", 90.0),
        TableEntry::new("c", "C", 90.0),
        TableEntry::new("d", "D", 90.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 10);
    let mut scratch = UpstreamSentenceScratch::default();

    let initial = model.candidates_for_input_with_limit_and_scratch("ab", 5, &mut scratch);
    assert_eq!(initial[0].text, "AB");

    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();
    let third = model.candidates_for_input_with_limit_and_scratch("abc", 5, &mut scratch);
    let fourth = model.candidates_for_input_with_limit_and_scratch("abcd", 5, &mut scratch);
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(third[0].text, "ABC");
    assert_eq!(fourth[0].text, "ABCD");
    assert_eq!(metrics.upstream_sentence_model_incremental_reuse_hits, 2);
    assert!(
        metrics.upstream_sentence_model_code_prefix_checks <= 7,
        "single-key extension should advance cached prefix ranges instead of replaying old prefixes: {metrics:?}"
    );
    assert!(
        metrics.upstream_sentence_model_phrase_index_nodes_visited <= 4,
        "only newly advanced prefix hits should be visited during cached extension: {metrics:?}"
    );
}

#[test]
fn upstream_sentence_scratch_matches_cold_sentence_then_phrase_stream_order() {
    let entries = [
        TableEntry::new("a", "A", 100.0),
        TableEntry::new("a", "X", 90.0),
        TableEntry::new("b", "B", 100.0),
        TableEntry::new("c", "C", 100.0),
    ];
    let vocabulary = [
        PresetVocabularyEntry::new("AB", 1_000.0),
        PresetVocabularyEntry::new("XB", 900.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10)
        .with_script_translation_limits(1, 1);
    let mut scratch = UpstreamSentenceScratch::default();

    for input in ["ab", "abc"] {
        let warm = model.candidates_for_input_with_limit_and_scratch(input, 5, &mut scratch);
        let cold = model.candidates_for_input_with_limit(input, 5);
        assert_eq!(
            warm, cold,
            "incremental stream order must match cold for {input}"
        );
    }

    let actual = model
        .candidates_for_input_with_limit_and_scratch("abc", 5, &mut scratch)
        .into_iter()
        .map(|candidate| candidate.text)
        .collect::<Vec<_>>();
    assert_eq!(actual, ["ABC", "AB", "XB", "A", "X"]);
}

#[test]
fn upstream_sentence_scratch_preserves_close_f64_phrase_weight_order() {
    let previous = "é".repeat(2_047);
    let input = "é".repeat(2_048);
    let entries = [
        TableEntry::new("é", "P", 1.0),
        TableEntry::new(&input, "Z", 100.125),
        TableEntry::new(&input, "A", 100.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 2);
    let mut scratch = UpstreamSentenceScratch::default();

    let _ = model.candidates_for_input_with_limit_and_scratch(&previous, 2, &mut scratch);
    let warm = model.candidates_for_input_with_limit_and_scratch(&input, 2, &mut scratch);
    let cold = model.candidates_for_input_with_limit(&input, 2);

    assert_eq!(warm, cold, "warm extraction must retain graph f64 order");
    assert_eq!(
        warm.iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        ["Z", "A"],
        "close weights must not collapse into a lexical f32 tie at long byte offsets"
    );
}

#[test]
fn upstream_sentence_model_accepts_owned_table_entry_stream() {
    let entries = [
        crate::TableEntry::new("ab", "A", 10.0),
        crate::TableEntry::new("cd", "B", 9.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 10);

    let candidates = model.candidates_for_input("abcd");

    assert_eq!(candidates[0].text, "AB");
}

#[test]
fn upstream_sentence_model_uses_supplied_code_spans_for_abbreviation_sentences() {
    let entries = [
        crate::TableEntry::new("chong", "A", 100.0),
        crate::TableEntry::new("shang", "B", 100.0),
        crate::TableEntry::new("zhu", "C", 100.0),
        crate::TableEntry::new("yi", "D", 100.0),
    ];
    let vocabulary = [crate::PresetVocabularyEntry::new("ABCD", 1000.0)];
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10);

    assert!(model.candidates_for_input("cszy").is_empty());

    let candidates = model.candidates_for_code_spans_with_limit(
        "cszy",
        &[
            SentenceCodeSpan::new(0, 1, "chong"),
            SentenceCodeSpan::new(1, 2, "shang"),
            SentenceCodeSpan::new(2, 3, "zhu"),
            SentenceCodeSpan::new(3, 4, "yi"),
        ],
        5,
    );

    assert_eq!(candidates[0].text, "ABCD");
    assert_eq!(candidates[0].source, CandidateSource::Sentence);
}

#[test]
fn upstream_sentence_model_prefers_long_abbreviation_phrase_over_short_phrase_pairs() {
    let entries = [
        crate::TableEntry::new("c1", "A", 100.0),
        crate::TableEntry::new("s1", "B", 100.0),
        crate::TableEntry::new("z1", "C", 100.0),
        crate::TableEntry::new("y1", "D", 100.0),
        crate::TableEntry::new("c2", "W", 100.0),
        crate::TableEntry::new("s2", "X", 100.0),
        crate::TableEntry::new("z2", "Y", 100.0),
        crate::TableEntry::new("y2", "Z", 100.0),
    ];
    let vocabulary = [
        crate::PresetVocabularyEntry::new("ABCD", 1.0),
        crate::PresetVocabularyEntry::new("WX", 1_000_000.0),
        crate::PresetVocabularyEntry::new("YZ", 1_000_000.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10);

    let candidates = model.candidates_for_code_spans_with_limit(
        "cszy",
        &[
            SentenceCodeSpan::new(0, 1, "c1"),
            SentenceCodeSpan::new(0, 1, "c2"),
            SentenceCodeSpan::new(1, 2, "s1"),
            SentenceCodeSpan::new(1, 2, "s2"),
            SentenceCodeSpan::new(2, 3, "z1"),
            SentenceCodeSpan::new(2, 3, "z2"),
            SentenceCodeSpan::new(3, 4, "y1"),
            SentenceCodeSpan::new(3, 4, "y2"),
        ],
        5,
    );

    assert_eq!(candidates[0].text, "ABCD");
}

#[test]
fn upstream_sentence_model_ignores_zero_weight_character_codes_for_phrase_derivation() {
    let entries = [
        crate::TableEntry::new("a", "A", 100.0),
        crate::TableEntry::new("b", "B", 100.0),
        crate::TableEntry::new("x", "X", 0.0),
    ];
    let vocabulary = [
        crate::PresetVocabularyEntry::new("AX", 1_000_000.0),
        crate::PresetVocabularyEntry::new("AB", 1.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10);

    let candidates = model.candidates_for_code_spans_with_limit(
        "ab",
        &[
            SentenceCodeSpan::new(0, 1, "a"),
            SentenceCodeSpan::new(1, 2, "b"),
            SentenceCodeSpan::new(1, 2, "x"),
        ],
        5,
    );

    assert_eq!(candidates[0].text, "AB");
}

#[test]
fn upstream_sentence_model_prefers_abbreviation_phrase_paths_without_singletons() {
    let entries = [
        crate::TableEntry::new("a", "A", 100.0),
        crate::TableEntry::new("b", "B", 100.0),
        crate::TableEntry::new("c", "C", 100.0),
        crate::TableEntry::new("d", "D", 100.0),
        crate::TableEntry::new("w", "W", 100.0),
        crate::TableEntry::new("x", "X", 100.0),
        crate::TableEntry::new("y", "Y", 100.0),
        crate::TableEntry::new("z", "Z", 100.0),
    ];
    let vocabulary = [
        crate::PresetVocabularyEntry::new("AB", 1.0),
        crate::PresetVocabularyEntry::new("CD", 1.0),
        crate::PresetVocabularyEntry::new("WXY", 1_000_000.0),
    ];
    let model = UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10);

    let candidates = model.candidates_for_code_spans_with_limit(
        "abcd",
        &[
            SentenceCodeSpan::new(0, 1, "a"),
            SentenceCodeSpan::new(1, 2, "b"),
            SentenceCodeSpan::new(2, 3, "c"),
            SentenceCodeSpan::new(3, 4, "d"),
            SentenceCodeSpan::new(0, 1, "w"),
            SentenceCodeSpan::new(1, 2, "x"),
            SentenceCodeSpan::new(2, 3, "y"),
            SentenceCodeSpan::new(3, 4, "z"),
        ],
        5,
    );

    assert_eq!(candidates[0].text, "ABCD");
}

#[test]
fn octagram_encoder_matches_binary_key_format_facts() {
    assert_eq!(
        encode_octagram_key("A\0中䀀😀"),
        vec![0x41, 0xe0, 0x8e, 0x2d, 0xe1, 0x80, 0xe3, 0xbe, 0xbe, 0xbe]
    );
}

#[test]
fn octagram_grammar_parses_synthetic_gram_and_scores_collocations() {
    let bytes =
        synthetic_octagram_gram(&[("今天會", 500_000), ("天優", 900_000), ("會議$", 400_000)]);
    let grammar =
        OctagramGrammar::from_bytes(&bytes, OctagramGrammarConfig::default()).expect("valid gram");

    assert_eq!(grammar.query("", "會議", false), -12.0);
    assert_eq!(grammar.query("今天", "會議", false), 38.0);
    assert_eq!(grammar.query("今天", "優惠", false), 66.0);

    let rear_bytes = synthetic_octagram_gram(&[("會議$", 400_000)]);
    let rear_grammar = OctagramGrammar::from_bytes(&rear_bytes, OctagramGrammarConfig::default())
        .expect("valid rear gram");
    assert_eq!(rear_grammar.query("今天", "會議", true), 22.0);
}

#[test]
fn octagram_grammar_scores_rear_boundary_without_context() {
    let grammar = OctagramGrammar::from_bytes(
        &synthetic_octagram_gram(&[("A$", 250_000)]),
        OctagramGrammarConfig::default(),
    )
    .expect("valid synthetic gram");

    assert_eq!(grammar.query("", "A", false), -12.0);
    assert_eq!(grammar.query("", "A", true), -12.0);
}

#[test]
fn make_sentences_does_not_apply_octagram_rear_boundary_to_initial_word() {
    let grammar = OctagramGrammar::from_bytes(
        &synthetic_octagram_gram(&[("A$", 250_000)]),
        OctagramGrammarConfig::default(),
    )
    .expect("valid synthetic gram");
    let mut graph = WordGraph::new();
    graph
        .entry(0)
        .or_default()
        .entry(1)
        .or_default()
        .push(WordGraphEntry::new("A", 0.0));
    graph
        .entry(1)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("B", 0.0));

    let sentence = make_sentences_with_grammar(&graph, 2, 1, &grammar)
        .into_iter()
        .next()
        .expect("composed sentence should be produced");

    assert_eq!(sentence.text, "AB");
    assert_eq!(sentence.weight, -24.0);
}

#[test]
fn octagram_empty_context_rear_boundary_matches_librime_oracle_fixture() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/upstream-octagram/synthetic-rear-boundary-oracle.json"
    ))
    .expect("synthetic oracle fixture should parse");
    let grammar_hex = fixture["grammar_model"]["model_bytes_hex_chunks"]
        .as_array()
        .expect("fixture should include grammar byte chunks")
        .iter()
        .map(|chunk| {
            chunk
                .as_str()
                .expect("grammar byte chunk should be a string")
        })
        .collect::<String>();
    let grammar = OctagramGrammar::from_bytes(
        &decode_hex(&grammar_hex).expect("fixture grammar bytes should decode"),
        OctagramGrammarConfig::default(),
    )
    .expect("fixture grammar should parse");
    let entries = fixture["schema"]["dictionary_rows"]
        .as_array()
        .expect("fixture should include dictionary rows")
        .iter()
        .map(|row| {
            TableEntry::new(
                row["code"].as_str().expect("row code should be a string"),
                row["text"].as_str().expect("row text should be a string"),
                row["weight"]
                    .as_f64()
                    .expect("row weight should be numeric") as f32,
            )
        })
        .collect::<Vec<_>>();
    let model = UpstreamSentenceModel::from_table_entries(entries, &[], 10).with_grammar(grammar);
    let case = fixture["cases"]
        .as_array()
        .expect("fixture should include cases")
        .first()
        .expect("fixture should include one case");
    let input = case["input"]
        .as_str()
        .expect("oracle fixture input should be text");
    let expected = case["selected_candidates"]
        .as_array()
        .expect("oracle fixture should include selected candidates")
        .iter()
        .map(|candidate| {
            candidate["text"]
                .as_str()
                .expect("oracle candidate text should be a string")
                .to_owned()
        })
        .collect::<Vec<_>>();

    let actual = model
        .candidates_for_input(input)
        .into_iter()
        .take(expected.len())
        .map(|candidate| candidate.text)
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}

#[test]
fn octagram_grammar_caps_raw_prefix_matches_like_librime_lookup() {
    let context_key = encode_octagram_key("C");
    let word_key = encode_octagram_key("中中中中中");
    let mut entries = Vec::new();
    for byte_len in 1..=8 {
        let mut key = context_key.clone();
        key.extend_from_slice(&word_key[..byte_len]);
        entries.push((key, 1));
    }
    let mut ignored_key = context_key;
    ignored_key.extend_from_slice(&word_key[..10]);
    entries.push((ignored_key, 1_000_000));

    let grammar = OctagramGrammar::from_bytes(
        &synthetic_octagram_gram_from_encoded_entries(&entries),
        OctagramGrammarConfig {
            collocation_max_length: 6,
            ..OctagramGrammarConfig::default()
        },
    )
    .expect("valid synthetic gram");

    assert!(grammar.query("C", "中中中中中", false) < 0.0);
}

#[test]
fn octagram_grammar_rejects_invalid_gram_headers() {
    assert_eq!(
        OctagramGrammar::from_bytes(b"short", OctagramGrammarConfig::default()).unwrap_err(),
        OctagramGrammarParseError::ShortHeader { len: 5 }
    );

    let mut bytes = synthetic_octagram_gram(&[("今天會議", 500_000)]);
    bytes[0] = b'X';
    assert_eq!(
        OctagramGrammar::from_bytes(&bytes, OctagramGrammarConfig::default()).unwrap_err(),
        OctagramGrammarParseError::InvalidFormat
    );

    let mut bytes = synthetic_octagram_gram(&[("今天會議", 500_000)]);
    bytes[40..44].copy_from_slice(&i32::MAX.to_le_bytes());
    assert!(matches!(
        OctagramGrammar::from_bytes(&bytes, OctagramGrammarConfig::default()).unwrap_err(),
        OctagramGrammarParseError::PayloadOutOfBounds { .. }
    ));
}

#[test]
fn make_sentences_uses_octagram_grammar_to_rank_sentence_paths() {
    let grammar = OctagramGrammar::from_bytes(
        &synthetic_octagram_gram(&[("今天會議", 500_000)]),
        OctagramGrammarConfig::default(),
    )
    .expect("valid gram");
    let mut graph = WordGraph::new();
    graph
        .entry(0)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("今天", 0.0));
    graph.entry(2).or_default().entry(4).or_default().extend([
        WordGraphEntry::new("優惠", 20.0),
        WordGraphEntry::new("會議", 0.0),
    ]);

    let texts = make_sentences_with_grammar(&graph, 4, 2, &grammar)
        .into_iter()
        .map(|sentence| sentence.text)
        .collect::<Vec<_>>();

    assert_eq!(
        texts,
        ["今天會議"],
        "the lower octagram path falls outside pinned Poet's default plural cutoff"
    );
}

#[test]
fn make_sentences_passes_only_last_two_prior_words_to_grammar() {
    let grammar = RecordingGrammar::default();
    let mut graph = WordGraph::new();
    graph
        .entry(0)
        .or_default()
        .entry(1)
        .or_default()
        .push(WordGraphEntry::new("甲", 0.0));
    graph
        .entry(1)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("乙", 0.0));
    graph
        .entry(2)
        .or_default()
        .entry(3)
        .or_default()
        .push(WordGraphEntry::new("丙", 0.0));
    graph
        .entry(3)
        .or_default()
        .entry(4)
        .or_default()
        .push(WordGraphEntry::new("丁", 0.0));

    let _ = make_sentences_with_grammar(&graph, 4, 1, &grammar);
    let calls = grammar
        .calls
        .lock()
        .expect("recording grammar should not panic");

    assert!(calls
        .iter()
        .any(|(context, word, is_rear)| context == "乙丙" && word == "丁" && *is_rear));
    assert!(!calls
        .iter()
        .any(|(context, word, _)| context == "甲乙丙" && word == "丁"));
}

#[test]
fn octagram_sentence_lattice_keeps_same_text_paths_with_distinct_context() {
    let grammar = ContextBoostGrammar {
        context: "BCD",
        word: "E",
        score: 20.0,
    };
    let mut graph = WordGraph::new();
    graph
        .entry(0)
        .or_default()
        .entry(1)
        .or_default()
        .push(WordGraphEntry::new("A", 0.0));
    graph
        .entry(1)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("B", 0.0));
    graph
        .entry(2)
        .or_default()
        .entry(4)
        .or_default()
        .push(WordGraphEntry::new("CD", 0.0));
    graph
        .entry(0)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("AB", 5.0));
    graph
        .entry(2)
        .or_default()
        .entry(3)
        .or_default()
        .push(WordGraphEntry::new("C", 0.0));
    graph
        .entry(3)
        .or_default()
        .entry(4)
        .or_default()
        .push(WordGraphEntry::new("D", 0.0));
    graph
        .entry(4)
        .or_default()
        .entry(5)
        .or_default()
        .push(WordGraphEntry::new("E", 0.0));

    let sentence = make_sentences_with_grammar(&graph, 5, 1, &grammar)
        .into_iter()
        .next()
        .expect("sentence should be produced");

    assert_eq!(sentence.text, "ABCDE");
    assert_eq!(
        sentence.word_lengths,
        [2, 2, 1],
        "singular grammar keeps only the best line per last word before the next edge"
    );
}

#[test]
fn octagram_sentence_model_ignores_zero_weight_character_codes_for_normal_phrase_derivation() {
    let entries = [
        crate::TableEntry::new("a", "A", 100.0),
        crate::TableEntry::new("b", "B", 0.0),
        crate::TableEntry::new("x", "B", 100.0),
        crate::TableEntry::new("b", "C", 100.0),
    ];
    let vocabulary = [
        crate::PresetVocabularyEntry::new("AB", 1_000_000.0),
        crate::PresetVocabularyEntry::new("AC", 1.0),
    ];
    let null_model = UpstreamSentenceModel::from_table_entries(entries.clone(), &vocabulary, 10);
    let grammar = OctagramGrammar::from_bytes(
        &synthetic_octagram_gram(&[("AC", 1)]),
        OctagramGrammarConfig::default(),
    )
    .expect("valid gram");
    let octagram_model =
        UpstreamSentenceModel::from_table_entries(entries, &vocabulary, 10).with_grammar(grammar);

    assert_eq!(null_model.candidates_for_input("ab")[0].text, "AC");
    assert_eq!(octagram_model.candidates_for_input("ab")[0].text, "AC");
}

fn synthetic_octagram_gram(entries: &[(&str, u32)]) -> Vec<u8> {
    let encoded_entries = entries
        .iter()
        .map(|(key, value)| (encode_octagram_key(key), *value))
        .collect::<Vec<_>>();
    synthetic_octagram_gram_from_encoded_entries(&encoded_entries)
}

fn synthetic_octagram_gram_from_encoded_entries(entries: &[(Vec<u8>, u32)]) -> Vec<u8> {
    let double_array =
        DartsDoubleArray::build_bytes(entries).expect("synthetic gram keys should build");
    let mut bytes = vec![0; 44];
    bytes[.."Rime::Grammar/1.0".len()].copy_from_slice(b"Rime::Grammar/1.0");
    bytes[36..40].copy_from_slice(&(double_array.units().len() as u32).to_le_bytes());
    bytes[40..44].copy_from_slice(&4_i32.to_le_bytes());
    for unit in double_array.units() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}

fn decode_hex(input: &str) -> Result<Vec<u8>, String> {
    let bytes = input.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err("hex input should have even length".to_owned());
    }
    bytes
        .chunks_exact(2)
        .map(|chunk| {
            let high = decode_hex_digit(chunk[0])?;
            let low = decode_hex_digit(chunk[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn decode_hex_digit(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex digit: {byte}")),
    }
}

#[derive(Debug, Default)]
struct RecordingGrammar {
    calls: Mutex<Vec<(String, String, bool)>>,
}

impl Grammar for RecordingGrammar {
    fn query(&self, context: &str, word: &str, is_rear: bool) -> f64 {
        self.calls
            .lock()
            .expect("recording grammar should not panic")
            .push((context.to_owned(), word.to_owned(), is_rear));
        0.0
    }
}

#[derive(Debug)]
struct ContextBoostGrammar {
    context: &'static str,
    word: &'static str,
    score: f64,
}

impl Grammar for ContextBoostGrammar {
    fn query(&self, context: &str, word: &str, _is_rear: bool) -> f64 {
        if context == self.context && word == self.word {
            self.score
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
struct TestMmapPoetBytes {
    bytes: Arc<[u8]>,
}

impl TestMmapPoetBytes {
    fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }
}

impl PoetByteSource for TestMmapPoetBytes {
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn storage_label(&self) -> &'static str {
        "byte_backed"
    }

    fn mapping_mode(&self) -> &'static str {
        "mmap"
    }
}
