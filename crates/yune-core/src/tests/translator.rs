use std::collections::HashMap;

use crate::{
    build_prism_bin, parse_rime_prism_bin_payload, CandidateRequest, CandidateSource, Context,
    Engine, HistoryTranslator, MemoryOwnerClass, PunctuationTranslator, ReverseLookupTranslator,
    StaticTableTranslator, Status, TableDictionary, Translator,
};

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
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>(),
        eager
            .iter()
            .take(3)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(bounded.full_count, Some(5));
    assert!(!bounded.is_complete);
}

#[test]
fn static_table_memory_owner_rows_cover_m43_owner_set() {
    let translator =
        StaticTableTranslator::new([("ni", "你"), ("hao", "好"), ("zhong", "中"), ("guo", "國")])
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
        MemoryOwnerClass::HeapOwnedReducible
    );
    assert_eq!(
        owner_class("poet.lookup_index"),
        MemoryOwnerClass::HeapOwnedGuarded
    );
    assert_eq!(
        owner_class("poet.abbreviation_vocabulary"),
        MemoryOwnerClass::HeapOwnedReducible
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
        full_pinyin_metrics.upstream_sentence_model_vocabulary_entries_considered, 0,
        "full-pinyin sentence lookup must stay on the M40 model without abbreviation vocabulary"
    );
    assert_eq!(
        full_pinyin_metrics.abbreviation_span_discovery_calls, 0,
        "full-pinyin sentence lookup must not invoke the M42 abbreviation path"
    );
    assert_eq!(
        full_pinyin_metrics.abbreviation_code_span_graph_build_ns, 0,
        "full-pinyin sentence lookup must not record abbreviation code-span graph work"
    );

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
