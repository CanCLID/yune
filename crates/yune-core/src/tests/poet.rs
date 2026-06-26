use crate::{
    make_sentence, make_sentences, null_grammar_score, CandidateSource, StaticTableTranslator,
    TableDictionary, Translator, UpstreamSentenceModel, WordGraph, WordGraphEntry,
    UPSTREAM_NO_GRAMMAR_PENALTY,
};

#[test]
fn null_grammar_score_applies_upstream_penalty() {
    assert!((UPSTREAM_NO_GRAMMAR_PENALTY - 1.0e-6_f64.ln()).abs() < f64::EPSILON);
    assert!((null_grammar_score(20.0) - (20.0 + UPSTREAM_NO_GRAMMAR_PENALTY)).abs() < f64::EPSILON);
}

#[test]
fn make_sentence_prefers_single_phrase_when_penalty_outweighs_shorter_path() {
    let mut graph = WordGraph::new();
    graph
        .entry(0)
        .or_default()
        .entry(4)
        .or_default()
        .push(WordGraphEntry::new("AB", "abcd", 100.0));
    graph
        .entry(0)
        .or_default()
        .entry(2)
        .or_default()
        .push(WordGraphEntry::new("A", "ab", 10.0));
    graph
        .entry(2)
        .or_default()
        .entry(4)
        .or_default()
        .push(WordGraphEntry::new("B", "cd", 9.0));

    let sentence = make_sentence(&graph, 4).expect("sentence should be available");

    assert_eq!(sentence.text, "AB");
    assert_eq!(sentence.word_lengths, [4]);
}

#[test]
fn make_sentences_keeps_weight_ordered_beam() {
    let mut graph = WordGraph::new();
    graph.entry(0).or_default().entry(2).or_default().extend([
        WordGraphEntry::new("A", "ab", 10.0),
        WordGraphEntry::new("X", "ab", 9.0),
    ]);
    graph.entry(2).or_default().entry(4).or_default().extend([
        WordGraphEntry::new("B", "cd", 9.0),
        WordGraphEntry::new("Y", "cd", 7.0),
    ]);

    let sentences = make_sentences(&graph, 4, 3)
        .into_iter()
        .map(|sentence| sentence.text)
        .collect::<Vec<_>>();

    assert_eq!(sentences, ["AB", "XB", "AY"]);
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

    assert_eq!(candidates[0].text, "ABC");
    assert!(metrics.upstream_sentence_model_exact_range_index_hits >= 3);
    assert!(metrics.upstream_sentence_model_exact_range_index_misses >= 1);
    assert!(metrics.upstream_sentence_model_prefix_filter_hits >= 3);
    assert!(metrics.upstream_sentence_model_prefix_filter_misses >= 1);
    assert!(metrics.upstream_sentence_model_prefix_filter_early_breaks >= 1);
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

    assert_eq!(
        reset_metrics.upstream_sentence_model_exact_range_index_hits,
        0
    );
    assert_eq!(
        reset_metrics.upstream_sentence_model_phrase_index_walk_calls,
        0
    );
    assert_eq!(reset_metrics.upstream_sentence_model_graph_rebuild_calls, 0);
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
