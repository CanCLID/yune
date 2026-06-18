use crate::{
    CandidateSource, Engine, HistoryTranslator, PunctuationTranslator, ReverseLookupTranslator,
    StaticTableTranslator, TableDictionary, Translator,
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
