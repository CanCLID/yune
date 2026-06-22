use crate::{
    make_sentence, make_sentences, null_grammar_score, CandidateSource, StaticTableTranslator,
    TableDictionary, Translator, WordGraph, WordGraphEntry, UPSTREAM_NO_GRAMMAR_PENALTY,
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
