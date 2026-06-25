use crate::{
    Candidate, CandidateFilter, CandidateSource, CharsetFilter, DictionaryLookupFilter, Engine,
    ReverseLookupFilter, ReverseLookupTranslator, RimeCorrectionEntry, SimplifierFilter,
    SingleCharFilter, StaticTableTranslator, TableDictionary, TableDictionaryAdvancedData,
    TableEntry, TaggedFilter, Translator, UniquifierFilter,
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
            preedit: None,
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "好".to_owned(),
            comment: String::new(),
            preedit: None,
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "你".to_owned(),
            comment: String::new(),
            preedit: None,
            source: CandidateSource::Completion,
            quality: 0.5,
        },
        Candidate {
            text: "你好".to_owned(),
            comment: " ☯ ".to_owned(),
            preedit: None,
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
        preedit: None,
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
            preedit: None,
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "word".to_owned(),
            comment: "lei5".to_owned(),
            preedit: None,
            source: CandidateSource::Completion,
            quality: 0.5,
        },
        Candidate {
            text: "word".to_owned(),
            comment: "history".to_owned(),
            preedit: None,
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
fn dictionary_lookup_filter_emits_typeduck_panel_comments_from_code_first_rows() {
    let dictionary_yaml = format!(
        "---\n\
name: typeduck_lookup\n\
version: \"0.1\"\n\
sort: original\n\
...\n\
\n\
nei5,1,0,,oth,,,,,,,you (singular),tm,nepali,hindi,kamu\t{}\n\
ne1,1,0,,part,,,,,,,(how about),(particle),,(particle),(imbuhan kata)\t{}\n\
nei1,2,0,,oth,ver,,,this,,,this,,,,\t{}\n\
ni1,2,0,,oth,ver,,,this,,,this,,,,\t{}\n",
        "\u{4f60}", "\u{5462}", "\u{5462}", "\u{5462}",
    );
    let dictionary = TableDictionary::parse_typeduck_lookup_dict_yaml(&dictionary_yaml)
        .expect("TypeDuck code-first lookup rows should parse");
    let filter = DictionaryLookupFilter::new(dictionary);

    let mut candidates = vec![
        Candidate {
            text: "\u{4f60}".to_owned(),
            comment: "\u{000c}nei5".to_owned(),
            preedit: None,
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "\u{5462}".to_owned(),
            comment: "\u{000c}nei1".to_owned(),
            preedit: None,
            source: CandidateSource::Table,
            quality: 1.0,
        },
    ];
    filter.apply(&mut candidates);

    assert_eq!(
        candidates[0].comment,
        format!(
            "\u{000c}\r1,{},nei5,1,0,,oth,,,,,,,you (singular),tm,nepali,hindi,kamu",
            "\u{4f60}",
        )
    );
    assert_eq!(
        candidates[1].comment,
        format!(
            "\u{000c}\r1,{},nei1,2,0,,oth,ver,,,this,,,this,,,,\
\r0,{},ne1,1,0,,part,,,,,,,(how about),(particle),,(particle),(imbuhan kata)\
\r0,{},ni1,2,0,,oth,ver,,,this,,,this,,,,",
            "\u{5462}", "\u{5462}", "\u{5462}",
        )
    );
}

#[test]
fn dictionary_lookup_filter_marks_combined_lookup_codes_as_primary() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: typeduck_lookup
version: "0.1"
sort: original
columns: [text, code, weight, stem, source, english]
...

好	hou2	1	0	adj	good; very
好	hou3	2	0	v	like
"#,
    )
    .expect("dictionary lookup rows should parse");
    let filter = DictionaryLookupFilter::new(dictionary);

    let mut candidates = vec![Candidate {
        text: "好".to_owned(),
        comment: "\u{000c}hou2;hou3".to_owned(),
        preedit: None,
        source: CandidateSource::Table,
        quality: 1.0,
    }];
    filter.apply(&mut candidates);

    assert_eq!(
        candidates[0].comment,
        "\u{000c}\r1,好,hou2,1,0,adj,good; very\r1,好,hou3,2,0,v,like"
    );
}

#[test]
fn dictionary_lookup_filter_preserves_side_lookup_comment_prefix() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: typeduck_lookup
version: "0.1"
sort: original
...

旴	heoi1	1	0	rare
"#,
    )
    .expect("dictionary lookup rows should parse");
    let filter = DictionaryLookupFilter::new(dictionary);

    let mut candidates = vec![Candidate {
        text: "旴".to_owned(),
        comment: "\u{000b}～木".to_owned(),
        preedit: None,
        source: CandidateSource::Completion,
        quality: -1.0,
    }];
    filter.apply(&mut candidates);

    assert_eq!(
        candidates[0].comment,
        "\u{000b}～木\u{000c}\r1,旴,heoi1,1,0,rare"
    );
}

#[test]
fn dictionary_lookup_filter_expands_sentence_candidates_into_segment_records() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: typeduck_sentence_lookup
version: "0.1"
sort: original
columns: [text, code, weight, stem, source, english]
...

我係個	ngo5hai6go3	1	0	composition
我係	ngo5hai6	1	0	oth	I am
個	go3	1	0	mw	item of
"#,
    )
    .expect("dictionary lookup rows should parse");
    let filter = DictionaryLookupFilter::new(dictionary);

    let mut candidates = vec![Candidate {
        text: "我係個".to_owned(),
        comment: " ☯ ".to_owned(),
        preedit: None,
        source: CandidateSource::Sentence,
        quality: 1.0,
    }];
    filter.apply(&mut candidates);

    assert_eq!(
        candidates[0].comment,
        "\u{000c}\r1,我係個,ngo5hai6go3,1,0,composition\r1,我係,ngo5hai6,1,0,oth,I am\r1,個,go3,1,0,mw,item of"
    );
}

#[test]
fn typeduck_lookup_parser_rejects_regular_table_rows() {
    let dictionary_yaml = r#"
---
name: regular_lookup
version: "0.1"
sort: original
...

word	nei5	1	n	primary
"#;

    assert!(TableDictionary::parse_typeduck_lookup_dict_yaml(dictionary_yaml).is_err());
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
fn static_table_translator_prefix_fallback_preserves_full_match() {
    let translator =
        StaticTableTranslator::new([("a", "prefix"), ("ab", "full")]).with_prefix_fallback(true);

    let candidates = translator.translate("ab");
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(texts, ["full", "prefix"]);
    assert!(matches!(
        candidates[1].source,
        CandidateSource::PartialTable { consumed: 1, .. }
    ));
}

#[test]
fn static_table_translator_completion_can_filter_by_prediction_weight_threshold() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        "\
---
name: prediction_threshold
version: '1'
sort: by_weight
...

LOW\tsanlow\t2
HIGH\tsanhigh\t4
",
    )
    .expect("dictionary should parse");
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_completion(true)
            .with_prediction_weight_threshold(3.0),
    );
    engine.set_input("san");

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["HIGH", "san"]);
}

#[test]
fn static_table_translator_prediction_never_first_keeps_exact_candidate_ahead() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        "\
---
name: prediction_never_first
version: '1'
sort: by_weight
...

EXACT\tsan\t1
LONG\tsantai\t4
",
    )
    .expect("dictionary should parse");
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_completion(true)
            .with_prediction_never_first(true),
    );
    engine.set_input("san");

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["EXACT", "LONG", "san"]);
}

#[test]
fn static_table_translator_completion_surfaces_long_cantonese_prediction() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        "\
---
name: cantonese_prediction
version: '1'
sort: by_weight
...

身體\tsantai\t3
身體健康\tsantaiginhong\t5
",
    )
    .expect("dictionary should parse");
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_completion(true)
            .with_prediction_never_first(true),
    );
    engine.set_input("santai");

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["身體", "身體健康", "santai"]);
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
fn static_table_translator_sentence_allows_literal_single_letter_interior_segments() {
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::new([("ni", "N"), ("a", "A"), ("hao", "H")]).with_sentence(true),
    );
    engine
        .process_key_sequence("niahao")
        .expect("key sequence should parse");

    let candidates = &engine.context().candidates;
    assert_eq!(candidates[0].text, "NAH");
    assert_eq!(candidates[0].source, CandidateSource::Sentence);
}

#[test]
fn static_table_translator_sentence_skips_single_letter_segments_in_abbreviation_schemas() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        "\
---
name: abbreviation_collision
version: '1'
sort: original
...

我\tngo5\t10
係\thai6\t9
個\tgo3\t8
官\tng\t20
O\to\t20
",
    )
    .expect("dictionary should parse");
    let formulas = vec![
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
    ];
    let mut engine = Engine::new();
    engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_spelling_algebra(&formulas)
            .with_completion(true)
            .with_sentence(true),
    );
    engine
        .process_key_sequence("ngohaigo")
        .expect("key sequence should parse");

    assert_eq!(engine.context().candidates[0].text, "我係個");
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
fn static_table_translator_sentence_uses_spelling_algebra_expanded_codes() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        r#"
---
name: jyut6ping3_sentence
version: "0.1"
sort: original
...

我	ngo5	10
係	hai6	9
個	go3	8
嘅	ge3	7
家	gaa1	6
"#,
    )
    .expect("dictionary should parse");
    let formulas = vec![
        "derive/^ng(?=[aeiou])//".to_owned(),
        "derive/^(?=[aeiou])/ng/".to_owned(),
        "derive/^n(?!g)/l/".to_owned(),
        "derive/^ng(?=\\d)/m/".to_owned(),
        "derive/^(g|k)w(?=o)/$1/".to_owned(),
        "derive/^jy?(?=[aeiou])/y/".to_owned(),
        "derive/^jyu/ju/".to_owned(),
        "derive/yu(?!ng|k)/y/".to_owned(),
        "derive/(g|k)u(?!ng|k)/$1wu/".to_owned(),
        "derive/^([zcs])/$1h/".to_owned(),
        "derive/eoi(?=\\d)/eoy/".to_owned(),
        "derive/eo/oe/".to_owned(),
        "derive/oe/eo/".to_owned(),
        "derive/aa(?=\\d)/a/".to_owned(),
        "derive/\\d//".to_owned(),
        "abbrev/^([a-z]).+$/$1/".to_owned(),
        "xform/1/v/".to_owned(),
        "xform/4/vv/".to_owned(),
        "xform/2/x/".to_owned(),
        "xform/5/xx/".to_owned(),
        "xform/3/q/".to_owned(),
        "xform/6/qq/".to_owned(),
    ];
    let translator = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&formulas)
        .with_completion(true)
        .with_sentence(true);
    assert_eq!(translator.translate("ngo")[0].text, "我");
    assert_eq!(translator.translate("hai")[0].text, "係");
    assert_eq!(translator.translate("go")[0].text, "個");

    let mut engine = Engine::new();
    engine.add_translator(translator);
    engine
        .process_key_sequence("ngohaigo")
        .expect("key sequence should parse");

    assert_eq!(engine.context().candidates[0].text, "我係個");

    engine.clear_composition();
    engine
        .process_key_sequence("ngohaig")
        .expect("key sequence should parse");

    assert_eq!(engine.context().candidates[0].text, "我係個");
    assert_eq!(engine.process_char(' ').as_deref(), Some("我係個"));
}

#[test]
fn static_table_translator_expands_cantonese_lookahead_spelling_algebra() {
    let dictionary = TableDictionary::parse_rime_dict_yaml(
        "\
---
name: jyut6ping3_lookahead
version: '0.1'
sort: original
columns: [code, text, weight]
...

ng5\t\u{4e94}\t1
noi5\t\u{5167}\t1
oi3\t\u{611b}\t1
",
    )
    .expect("dictionary should parse");
    let formulas = vec![
        "derive/^ng(?=\\d)/m/".to_owned(),
        "derive/^n(?!g)/l/".to_owned(),
        "derive/^(?=[aeiou])/ng/".to_owned(),
        "derive/\\d//".to_owned(),
    ];
    let translator =
        StaticTableTranslator::from_dictionary(dictionary).with_spelling_algebra(&formulas);

    assert!(translator
        .translate("m")
        .iter()
        .any(|candidate| candidate.text == "\u{4e94}"));
    assert!(translator
        .translate("loi")
        .iter()
        .any(|candidate| candidate.text == "\u{5167}"));
    assert!(translator
        .translate("ngoi")
        .iter()
        .any(|candidate| candidate.text == "\u{611b}"));
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
fn static_table_translator_comment_format_decodes_rime_control_escapes() {
    let formulas = vec!["xform/^/\\f/".to_owned(), "xform/$/\\r1/".to_owned()];
    let translator =
        StaticTableTranslator::new([("nei", "\u{4f60}")]).with_comment_format(&formulas);

    let candidates = translator.translate("nei");

    assert_eq!(candidates[0].comment, "\u{000c}nei\r1");
    assert_ne!(candidates[0].comment, "\\fnei\\r1");
}

#[test]
fn static_table_translator_combines_duplicate_text_candidates_with_lookup_codes() {
    let formulas = vec!["xform/^/\u{000c}/".to_owned()];
    let translator = StaticTableTranslator::new([("hou2", "好"), ("hou3", "好"), ("hou6", "號")])
        .with_spelling_algebra(&["derive/\\d//".to_owned()])
        .with_combine_candidates(true)
        .with_comment_format(&formulas);

    let candidates = translator.translate("hou");

    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    let comments = candidates
        .iter()
        .map(|candidate| candidate.comment.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["好", "號"]);
    assert_eq!(comments, ["\u{000c}hou2;hou3", "\u{000c}hou6"]);
}

#[test]
fn static_table_translator_supports_affix_lookup_and_cangjie_short_code_comments() {
    let formulas = vec![
        "xform/^/\u{000b}/".to_owned(),
        "xlit|abcdefghijklmnopqrstuvwxyz~|日月金木水火土竹戈十大中一弓人心手口尸廿山女田難卜符～|"
            .to_owned(),
    ];

    let short_code = StaticTableTranslator::new([("am", "旦"), ("amd", "旴")])
        .with_completion(true)
        .with_affix("`c", ";")
        .with_show_full_code(false)
        .with_comment_format(&formulas);
    let short_candidates = short_code.translate("`cam");
    let short_comments = short_candidates
        .iter()
        .map(|candidate| candidate.comment.as_str())
        .collect::<Vec<_>>();
    assert_eq!(short_comments, ["", "\u{000b}～木"]);

    let full_code = StaticTableTranslator::new([("am", "旦"), ("amd", "旴")])
        .with_completion(true)
        .with_affix("`c", ";")
        .with_show_full_code(true)
        .with_comment_format(&formulas);
    let full_candidates = full_code.translate("`cam");
    let full_comments = full_candidates
        .iter()
        .map(|candidate| candidate.comment.as_str())
        .collect::<Vec<_>>();
    assert_eq!(full_comments, ["\u{000b}日一", "\u{000b}日一木"]);
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
fn static_table_translator_gates_dictionary_corrections_independently_of_completion() {
    let dictionary = TableDictionary::with_advanced_data(
        [TableEntry::new("ba", "八", 0.0)],
        TableDictionaryAdvancedData {
            corrections: vec![RimeCorrectionEntry::new("bq", "ba")],
            ..TableDictionaryAdvancedData::default()
        },
    );

    let disabled = StaticTableTranslator::from_dictionary(dictionary.clone())
        .with_completion(false)
        .translate("bq");
    assert!(
        disabled.is_empty(),
        "dictionary corrections should stay off unless enable_correction is true"
    );

    let enabled = StaticTableTranslator::from_dictionary(dictionary)
        .with_completion(false)
        .with_correction(true)
        .translate("bq");
    assert_eq!(enabled[0].text, "八");
    assert_eq!(enabled[0].comment, "ba");
}

#[test]
fn static_table_translator_discards_non_minimal_dictionary_corrections() {
    let dictionary = TableDictionary::with_advanced_data(
        [
            TableEntry::new("aaa", "正", 0.0),
            TableEntry::new("aa", "近", 20.0),
            TableEntry::new("a", "遠", 100.0),
        ],
        TableDictionaryAdvancedData {
            corrections: vec![
                RimeCorrectionEntry::new("aaa", "aa"),
                RimeCorrectionEntry::new("aaa", "a"),
            ],
            ..TableDictionaryAdvancedData::default()
        },
    );

    let candidates = StaticTableTranslator::from_dictionary(dictionary)
        .with_correction(true)
        .translate("aaa");
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["正", "近"]);
}

#[test]
fn static_table_translator_dictionary_corrections_target_normal_derived_codes() {
    let dictionary = TableDictionary::with_advanced_data(
        [TableEntry::new("ba", "八", 0.0)],
        TableDictionaryAdvancedData {
            corrections: vec![RimeCorrectionEntry::new("bqq", "bq")],
            ..TableDictionaryAdvancedData::default()
        },
    );

    let candidates = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&["derive/^ba$/bq/".to_owned()])
        .with_correction(true)
        .translate("bqq");
    assert_eq!(candidates[0].text, "八");
    assert_eq!(candidates[0].comment, "ba");
}

#[test]
fn static_table_translator_dictionary_corrections_reject_non_normal_derived_codes() {
    let dictionary = TableDictionary::with_advanced_data(
        [TableEntry::new("ba", "八", 0.0)],
        TableDictionaryAdvancedData {
            corrections: vec![RimeCorrectionEntry::new("bqq", "bq")],
            ..TableDictionaryAdvancedData::default()
        },
    );

    let candidates = StaticTableTranslator::from_dictionary(dictionary)
        .with_spelling_algebra(&["fuzz/^ba$/bq/".to_owned()])
        .with_correction(true)
        .translate("bqq");
    assert!(
        candidates.is_empty(),
        "dictionary corrections should not target fuzzy spelling-algebra derived codes"
    );
}

#[test]
fn static_table_translator_scales_dictionary_correction_penalty_by_distance() {
    let dictionary = TableDictionary::with_advanced_data(
        [
            TableEntry::new("aaa", "正", 0.0),
            TableEntry::new("aa", "近", 20.0),
        ],
        TableDictionaryAdvancedData {
            corrections: vec![RimeCorrectionEntry::new("aaa", "aa")],
            ..TableDictionaryAdvancedData::default()
        },
    );

    let candidates = StaticTableTranslator::from_dictionary(dictionary)
        .with_correction(true)
        .translate("aaa");
    assert_eq!(candidates[0].text, "正");
    assert_eq!(candidates[1].text, "近");
    assert!(candidates[0].quality > candidates[1].quality);
    let expected_correction_quality = (20.0_f32 + (-16.118_095_f32 * 2.0)).exp();
    assert!((candidates[1].quality - expected_correction_quality).abs() < f32::EPSILON);
}

#[test]
fn static_table_translator_dictionary_corrections_do_not_expand_completion_rows() {
    let dictionary = TableDictionary::with_advanced_data(
        [
            TableEntry::new("ba", "八", 0.0),
            TableEntry::new("baa", "把", 100.0),
        ],
        TableDictionaryAdvancedData {
            corrections: vec![RimeCorrectionEntry::new("bq", "ba")],
            ..TableDictionaryAdvancedData::default()
        },
    );

    let candidates = StaticTableTranslator::from_dictionary(dictionary)
        .with_completion(true)
        .with_correction(true)
        .translate("bq");
    let texts = candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(texts, ["八"]);
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
fn simplifier_filter_uses_opencc_hk2s_source_data() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("hk", "枱粧一口吃個")]));
    engine.add_filter(
        SimplifierFilter::new()
            .with_option_name("zh_hans")
            .with_opencc_config("hk2s.json"),
    );

    engine
        .process_key_sequence("hk")
        .expect("keys should parse");
    engine.set_option("zh_hans", true);

    assert_eq!(engine.context().candidates[0].text, "台妆一口吃个");
}

#[test]
fn simplifier_filter_uses_opencc_t2hkf_source_data() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("hk", "檯溫兌說")]));
    engine.add_filter(
        SimplifierFilter::new()
            .with_option_name("variants_hk")
            .with_opencc_config("t2hkf.json"),
    );

    engine
        .process_key_sequence("hk")
        .expect("keys should parse");
    engine.set_option("variants_hk", true);

    assert_eq!(engine.context().candidates[0].text, "枱温兑説");
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
            preedit: None,
            source: CandidateSource::Table,
            quality: 1.0,
        },
        Candidate {
            text: "龍".to_owned(),
            comment: String::new(),
            preedit: None,
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
        preedit: None,
        source: CandidateSource::Table,
        quality: 1.0,
    }];
    filter.apply(&mut candidates);
    assert_eq!(candidates[0].comment, "〔WQ〕");
}
