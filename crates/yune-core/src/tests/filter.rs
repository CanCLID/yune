use crate::{
    Candidate, CandidateFilter, CandidateSource, CharsetFilter, DictionaryLookupFilter, Engine,
    ReverseLookupFilter, ReverseLookupTranslator, SimplifierFilter, SingleCharFilter,
    StaticTableTranslator, TableDictionary, TaggedFilter, Translator, UniquifierFilter,
};

#[test]
fn reverse_lookup_filter_updates_comments_like_librime() {
    let reverse_dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: stroke
version: "0.1"
sort: original
...

你	wq
好	vb
"#,
    )
    .expect("reverse lookup dictionary should parse");

    let default_filter = ReverseLookupFilter::new(reverse_dictionary.clone());
    let mut candidates = vec![
        Candidate {
            text: "你".to_owned(),
            comment: "ni".to_owned(),
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "好".to_owned(),
            comment: String::new(),
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "你".to_owned(),
            comment: String::new(),
            source: CandidateSource::Completion,
            quality: 0.5,
        },
        Candidate {
            text: "你好".to_owned(),
            comment: " ☯ ".to_owned(),
            source: CandidateSource::Sentence,
            quality: 2.0,
        },
    ];
    default_filter.apply(&mut candidates);
    assert_eq!(candidates[0].comment, "ni");
    assert_eq!(candidates[1].comment, "vb");
    assert_eq!(candidates[2].comment, "wq");
    assert_eq!(candidates[3].comment, " ☯ ");

    let mut sentence_candidates = vec![Candidate {
        text: "你好".to_owned(),
        comment: " ☯ ".to_owned(),
        source: CandidateSource::Sentence,
        quality: 2.0,
    }];
    ReverseLookupFilter::new(
        TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: sentence_codes
version: "0.1"
sort: original
...

你好	wh
"#,
        )
        .expect("sentence reverse lookup dictionary should parse"),
    )
    .with_overwrite_comment(true)
    .apply(&mut sentence_candidates);
    assert_eq!(sentence_candidates[0].comment, "wh");

    let mut overwrite_engine = Engine::new();
    overwrite_engine.add_translator(StaticTableTranslator::new([("ni", "你")]));
    overwrite_engine.add_filter(
        ReverseLookupFilter::new(reverse_dictionary.clone()).with_overwrite_comment(true),
    );
    overwrite_engine
        .process_key_sequence("ni")
        .expect("keys should parse");
    assert_eq!(overwrite_engine.context().candidates[0].comment, "wq");

    let mut append_engine = Engine::new();
    append_engine.add_translator(StaticTableTranslator::new([("ni", "你")]));
    append_engine
        .add_filter(ReverseLookupFilter::new(reverse_dictionary).with_append_comment(true));
    append_engine
        .process_key_sequence("ni")
        .expect("keys should parse");
    assert_eq!(append_engine.context().candidates[0].comment, "ni; wq");
}

#[test]
fn dictionary_lookup_filter_emits_typeduck_panel_comments_from_source_rows() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: typeduck_lookup
version: "0.1"
sort: original
columns: [text, code, weight, stem, source, jyutping, english]
...

word	nei5	1	n	primary	nei5	you
word	lei5	2	l	variant	lei5	you alt
"#,
    )
    .expect("dictionary lookup rows should parse");
    let filter = DictionaryLookupFilter::new(dictionary);

    let mut candidates = vec![
        Candidate {
            text: "word".to_owned(),
            comment: "nei5".to_owned(),
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "word".to_owned(),
            comment: "lei5".to_owned(),
            source: CandidateSource::Completion,
            quality: 0.5,
        },
        Candidate {
            text: "word".to_owned(),
            comment: "history".to_owned(),
            source: CandidateSource::History,
            quality: 2.0,
        },
    ];
    filter.apply(&mut candidates);

    assert_eq!(
        candidates[0].comment,
        "\u{000c}\r1,word,nei5,1,n,primary,nei5,you\r0,word,lei5,2,l,variant,lei5,you alt"
    );
    assert_eq!(
        candidates[1].comment,
        "\u{000c}\r1,word,lei5,2,l,variant,lei5,you alt\r0,word,nei5,1,n,primary,nei5,you"
    );
    assert_eq!(candidates[2].comment, "history");
}

#[test]
fn uniquifier_filter_removes_later_duplicate_candidate_texts() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你"), ("ni", "呢")]));
    engine.add_translator(StaticTableTranslator::new([("ni", "你"), ("ni", "ni")]));
    engine.add_filter(UniquifierFilter);

    engine
        .process_key_sequence("ni")
        .expect("keys should parse");

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["你", "呢", "ni"]);
}

#[test]
fn single_char_filter_moves_table_single_characters_before_phrases() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ni", "你好"),
        ("ni", "你"),
        ("ni", "呢"),
        ("ni", "你们"),
    ]));
    engine.add_filter(SingleCharFilter);

    engine
        .process_key_sequence("ni")
        .expect("keys should parse");

    let candidates = engine.context().candidates.iter().collect::<Vec<_>>();
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let sources = candidates
        .iter()
        .map(|candidate| candidate.source.clone())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["你", "呢", "你好", "你们", "ni"]);
    assert_eq!(
        sources,
        [
            CandidateSource::Table,
            CandidateSource::Table,
            CandidateSource::Table,
            CandidateSource::Table,
            CandidateSource::Echo,
        ]
    );
}

#[test]
fn charset_filter_removes_extended_cjk_until_option_enabled() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ni", "你"),
        ("ni", "㐀"),
        ("ni", "𠀀"),
        ("ni", "㍿"),
    ]));
    engine.add_filter(CharsetFilter);

    engine
        .process_key_sequence("ni")
        .expect("keys should parse");

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["你", "ni"]);

    engine.set_option("extended_charset", true);
    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["你", "㐀", "𠀀", "㍿", "ni"]);
}

#[test]
fn static_table_translator_charset_filter_matches_librime_option() {
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::new([("ni", "你"), ("ni", "㐀"), ("ni", "𠀀"), ("ni", "㍿")])
            .with_charset_filter(true),
    );

    engine
        .process_key_sequence("ni")
        .expect("keys should parse");

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["你", "ni"]);

    engine.set_option("extended_charset", true);
    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["你", "㐀", "𠀀", "㍿", "ni"]);
}

#[test]
fn static_table_translator_trims_trailing_librime_delimiters() {
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("ban", "班")]).with_delimiters("'"),
    );

    engine.process_char('b');
    engine.process_char('a');
    engine.process_char('\'');

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["爸", "ba'"]);
    assert_eq!(engine.context().composition.preedit, "ba'");
}

#[test]
fn static_table_translator_completion_matches_librime_option() {
    let mut exact_engine = Engine::new();
    exact_engine.add_translator(StaticTableTranslator::new([("ba", "爸"), ("ban", "班")]));
    exact_engine.process_char('b');

    let exact_texts = exact_engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(exact_texts, ["b"]);

    let mut completion_engine = Engine::new();
    completion_engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("ban", "班")]).with_completion(true),
    );
    completion_engine.process_char('b');

    let candidates = &completion_engine.context().candidates;
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let sources = candidates
        .iter()
        .map(|candidate| candidate.source.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["爸", "班", "b"]);
    assert_eq!(sources, ["completion", "completion", "echo"]);
}

#[test]
fn static_table_translator_honors_librime_segment_tags() {
    let mut custom_tag_engine = Engine::new();
    custom_tag_engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("ban", "班")])
            .with_completion(true)
            .with_tags(["custom"]),
    );
    custom_tag_engine.process_char('b');

    let texts = custom_tag_engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["b"]);

    custom_tag_engine.set_segment_tags(["abc", "custom"]);
    let texts = custom_tag_engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["爸", "班", "b"]);

    let mut abc_tag_engine = Engine::new();
    abc_tag_engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("ban", "班")])
            .with_completion(true)
            .with_tags(["custom", "abc"]),
    );
    abc_tag_engine.process_char('b');

    let texts = abc_tag_engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["爸", "班", "b"]);
}

#[test]
fn static_table_translator_sentence_fallback_matches_librime_option() {
    let mut disabled_engine = Engine::new();
    disabled_engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("bao", "包")]).with_sentence(false),
    );
    disabled_engine
        .process_key_sequence("babao")
        .expect("key sequence should parse");

    let texts = disabled_engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["babao"]);

    let mut enabled_engine = Engine::new();
    enabled_engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("bao", "包")])
            .with_sentence(true)
            .with_delimiters("'"),
    );
    enabled_engine
        .process_key_sequence("ba'bao")
        .expect("key sequence should parse");

    let candidates = &enabled_engine.context().candidates;
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let sources = candidates
        .iter()
        .map(|candidate| candidate.source.as_str())
        .collect::<Vec<_>>();
    let comments = candidates
        .iter()
        .map(|candidate| candidate.comment.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["爸包", "ba'bao"]);
    assert_eq!(sources, ["sentence", "echo"]);
    assert_eq!(comments[0], " ☯ ");
}

#[test]
fn static_table_translator_sentence_over_completion_prioritizes_sentence() {
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("baban", "巴班")])
            .with_completion(true)
            .with_sentence_over_completion(true),
    );
    engine
        .process_key_sequence("baba")
        .expect("key sequence should parse");

    let candidates = &engine.context().candidates;
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let sources = candidates
        .iter()
        .map(|candidate| candidate.source.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["爸爸", "巴班", "baba"]);
    assert_eq!(sources, ["sentence", "completion", "echo"]);
}

#[test]
fn static_table_translator_initial_quality_participates_in_candidate_order() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "低")]));
    engine.add_translator(StaticTableTranslator::new([("ba", "高")]).with_initial_quality(10.0));

    engine
        .process_key_sequence("ba")
        .expect("key sequence should parse");

    let candidates = &engine.context().candidates;
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["高", "低", "ba"]);
    assert_eq!(candidates[0].quality, 11.0);
    assert_eq!(candidates[1].quality, 1.0);
}

#[test]
fn static_table_translator_applies_librime_comment_format() {
    let formulas = vec![
        "xlit/ab/AB/".to_owned(),
        "xform/^/[/".to_owned(),
        "xform/$/]/".to_owned(),
    ];
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("ban", "班")])
            .with_completion(true)
            .with_comment_format(&formulas),
    );

    engine.process_char('b');

    let comments = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.comment.as_str())
        .collect::<Vec<_>>();
    assert_eq!(comments, ["[BA]", "[BAn]", "echo"]);
}

#[test]
fn static_table_translator_applies_librime_dictionary_exclude() {
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::new([("ba", "爸"), ("ban", "班"), ("bao", "包")])
            .with_completion(true)
            .with_dictionary_exclude(["爸", "班"]),
    );

    engine.process_char('b');

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let sources = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.source.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["包", "b"]);
    assert_eq!(sources, ["completion", "echo"]);
}

#[test]
fn static_table_translator_expands_librime_spelling_algebra() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: algebra
version: "0.1"
sort: original
...

略	lue	0
女	nv	0
病	bing	0
平	pin	0
长	chang	0
错	cuo	0
照	zyx	0
删	gone	0
"#,
    )
    .expect("dictionary should parse");
    let formulas = vec![
        "xlit/zyx/abc/".to_owned(),
        "xform/^lue$/lve/".to_owned(),
        "derive/^nv$/nu/".to_owned(),
        "fuzz/^bing$/pin/".to_owned(),
        "abbrev/^chang$/c/".to_owned(),
        "derive/^cuo$/cu/correction".to_owned(),
        "erase/^gone$/".to_owned(),
    ];
    let translator =
        StaticTableTranslator::from_dictionary(dictionary).with_spelling_algebra(&formulas);

    assert_eq!(translator.translate("lue").len(), 0);

    let lve = translator.translate("lve");
    assert_eq!(lve[0].text, "略");
    assert_eq!(lve[0].comment, "lue");

    let nv = translator.translate("nv");
    assert_eq!(nv[0].text, "女");
    assert_eq!(nv[0].comment, "nv");

    let nu = translator.translate("nu");
    assert_eq!(nu[0].text, "女");
    assert_eq!(nu[0].comment, "nv");

    let pin = translator.translate("pin");
    let exact = pin
        .iter()
        .find(|candidate| candidate.text == "平")
        .expect("normal spelling candidate should be present");
    let fuzzy = pin
        .iter()
        .find(|candidate| candidate.text == "病")
        .expect("fuzzy spelling candidate should be present");
    assert_eq!(exact.quality, 1.0);
    assert!((fuzzy.quality - 0.5).abs() < f32::EPSILON);
    assert!(exact.quality > fuzzy.quality);

    let abbreviation = translator.translate("c");
    assert_eq!(abbreviation[0].text, "长");
    assert!((abbreviation[0].quality - 0.5).abs() < f32::EPSILON);

    let correction = translator.translate("cu");
    assert_eq!(correction[0].text, "错");
    assert!((correction[0].quality - 0.01).abs() < 0.000_001);

    let transliterated = translator.translate("abc");
    assert_eq!(transliterated[0].text, "照");
    assert_eq!(transliterated[0].comment, "zyx");
    assert_eq!(translator.translate("zyx").len(), 0);
    assert_eq!(translator.translate("gone").len(), 0);
}

#[test]
fn simplifier_filter_converts_text_when_option_enabled() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("tw", "臺灣"), ("tw", "龍馬")]));
    engine.add_filter(SimplifierFilter::new());

    engine
        .process_key_sequence("tw")
        .expect("keys should parse");

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["臺灣", "龍馬", "tw"]);

    engine.set_option("simplification", true);
    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let comments = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.comment.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["台湾", "龙马", "tw"]);
    assert_eq!(comments, ["tw", "tw", "echo"]);
}

#[test]
fn simplifier_filter_honors_custom_option_name() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("tw", "臺灣")]));
    engine.add_filter(SimplifierFilter::new().with_option_name("zh_simp"));

    engine
        .process_key_sequence("tw")
        .expect("keys should parse");

    engine.set_option("simplification", true);
    assert_eq!(engine.context().candidates[0].text, "臺灣");

    engine.set_option("zh_simp", true);
    assert_eq!(engine.context().candidates[0].text, "台湾");
}

#[test]
fn tagged_filter_matches_librime_filter_tags() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("tw", "臺灣")]));
    engine.add_filter(TaggedFilter::new(SimplifierFilter::new(), ["custom"]));

    engine
        .process_key_sequence("tw")
        .expect("keys should parse");
    engine.set_option("simplification", true);

    assert_eq!(engine.context().candidates[0].text, "臺灣");

    engine.set_segment_tags(["abc", "custom"]);
    assert_eq!(engine.context().candidates[0].text, "台湾");
}

#[test]
fn simplifier_filter_honors_librime_opencc_config() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("tw", "台灣"), ("tw", "裏")]));
    engine.add_filter(
        SimplifierFilter::new()
            .with_option_name("zh_tw")
            .with_opencc_config("t2tw.json"),
    );

    engine
        .process_key_sequence("tw")
        .expect("keys should parse");

    engine.set_option("simplification", true);
    assert_eq!(engine.context().candidates[0].text, "台灣");

    engine.set_option("zh_tw", true);
    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["臺灣", "裡", "tw"]);
}

#[test]
fn simplifier_filter_shows_librime_tip_comments() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("tw", "臺"), ("tw", "臺灣")]));
    engine.add_filter(SimplifierFilter::new().with_tips("char"));

    engine
        .process_key_sequence("tw")
        .expect("keys should parse");
    engine.set_option("simplification", true);

    assert_eq!(engine.context().candidates[0].text, "台");
    assert_eq!(engine.context().candidates[0].comment, "〔臺〕");
    assert_eq!(engine.context().candidates[1].text, "台湾");
    assert_eq!(engine.context().candidates[1].comment, "tw");

    let mut all_tips_engine = Engine::new();
    let formulas = vec!["xform/^/[/".to_owned(), "xform/$/]/".to_owned()];
    all_tips_engine.add_translator(StaticTableTranslator::new([("tw", "臺灣")]));
    all_tips_engine.add_filter(
        SimplifierFilter::new()
            .with_tips("all")
            .with_comment_format(&formulas),
    );

    all_tips_engine
        .process_key_sequence("tw")
        .expect("keys should parse");
    all_tips_engine.set_option("simplification", true);

    assert_eq!(all_tips_engine.context().candidates[0].text, "台湾");
    assert_eq!(all_tips_engine.context().candidates[0].comment, "[臺灣]");
}

#[test]
fn simplifier_filter_can_show_converted_text_in_comment() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("tw", "臺灣")]));
    engine.add_filter(
        SimplifierFilter::new()
            .with_tips("all")
            .with_show_in_comment(true),
    );

    engine
        .process_key_sequence("tw")
        .expect("keys should parse");
    engine.set_option("simplification", true);

    assert_eq!(engine.context().candidates[0].text, "臺灣");
    assert_eq!(engine.context().candidates[0].comment, "台湾");
}

#[test]
fn simplifier_filter_honors_librime_excluded_types() {
    let filter = SimplifierFilter::new().with_excluded_types(["table".to_owned()]);
    let mut options = std::collections::HashMap::new();
    options.insert("simplification".to_owned(), true);
    let mut candidates = vec![
        Candidate {
            text: "臺灣".to_owned(),
            comment: "tw".to_owned(),
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "龍".to_owned(),
            comment: String::new(),
            source: CandidateSource::Punctuation,
            quality: 1.0,
        },
    ];

    filter.apply_with_options(&mut candidates, &options);

    assert_eq!(candidates[0].text, "臺灣");
    assert_eq!(candidates[0].comment, "tw");
    assert_eq!(candidates[1].text, "龙");
}

#[test]
fn reverse_lookup_comment_format_applies_projection_formulas() {
    let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: stroke
version: "0.1"
sort: original
...

你	wq
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

你	ni
"#,
    )
    .expect("target dictionary should parse");
    let formulas = vec![
        "xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/".to_owned(),
        "xform/^/〔/".to_owned(),
        "xform/$/〕/".to_owned(),
    ];

    let translator =
        ReverseLookupTranslator::new(lookup_dictionary, Some(target_dictionary), "", "")
            .with_comment_format(&formulas);
    let candidates = translator.translate("wq");
    assert_eq!(candidates[0].comment, "〔NI〕");

    let reverse_dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: stroke
version: "0.1"
sort: original
...

你	wq
"#,
    )
    .expect("reverse lookup dictionary should parse");
    let filter = ReverseLookupFilter::new(reverse_dictionary).with_comment_format(&formulas);
    let mut candidates = vec![Candidate {
        text: "你".to_owned(),
        comment: String::new(),
        source: CandidateSource::Table,
        quality: 1.0,
    }];
    filter.apply(&mut candidates);
    assert_eq!(candidates[0].comment, "〔WQ〕");
}
