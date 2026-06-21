    #[test]
    fn parses_rime_dict_yaml_default_columns_and_weight_order() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

巴	ba	3193
爸	ba	3625
八	ba	6677
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "爸");
        assert_eq!(entries[2].text, "巴");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 6677.0);
    }

    #[test]
    fn parses_rime_dict_yaml_custom_columns_for_shape_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: cangjie_sample
version: "0.1"
sort: original
columns:
  - text
  - code
  - stem
...

明	ab
晭	abgr	ab'gr
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries[0].text, "明");
        assert_eq!(entries[0].code, "ab");
        assert_eq!(entries[1].text, "晭");
        assert_eq!(entries[1].code, "abgr");
        assert_eq!(
            dictionary.stems().get("晭").cloned(),
            Some(vec!["ab'gr".to_owned()])
        );
    }

    #[test]
    fn parses_rime_dict_yaml_stem_columns_like_librime_entry_collector() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stem_sample
version: "0.1"
sort: original
columns: [text, code, stem]
...

明	ab	a'b
明	ab	a'b
明	ac	a'c
未编码		ignored
"#,
        )
        .expect("dictionary should parse");

        assert_eq!(
            dictionary.stems().get("明").cloned(),
            Some(vec!["a'b".to_owned(), "a'c".to_owned()])
        );
        assert_eq!(
            dictionary.stems_for("明"),
            Some(&["a'b".to_owned(), "a'c".to_owned()][..])
        );
        assert!(!dictionary.stems().contains_key("未编码"));
        assert_eq!(dictionary.stems_for("未编码"), None);
    }

    #[test]
    fn parses_rime_dict_yaml_reverse_dict_settings_as_read_only_contract() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: reverse_settings_sample
version: "0.1"
dict_settings:
  use_rule_based_encoder: true
  tail_anchor: "'"
  rules:
    - length_equal: 2
      formula: "AaBa"
...

明	ab	1
"#,
        )
        .expect("dictionary should parse");

        assert_eq!(
            dictionary.dict_settings().get("use_rule_based_encoder"),
            Some(&"true".to_owned())
        );
        assert_eq!(
            dictionary.dict_settings().get("tail_anchor"),
            Some(&"'".to_owned())
        );
        assert_eq!(
            dictionary.dict_settings().get("rules/0/length_equal"),
            Some(&"2".to_owned())
        );
        assert_eq!(
            dictionary.dict_settings().get("rules/0/formula"),
            Some(&"AaBa".to_owned())
        );
    }

    #[test]
    fn parses_rime_dict_yaml_encoder_settings_like_librime_dict_settings() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: encoder_sample
version: "0.1"
sort: original
encoder:
  exclude_patterns:
    - '^x.*$'
  rules:
    - length_equal: 2
      formula: "AaAzBaBz"
    - length_in_range: [3, 5]
      formula: "AaBaCaZz"
  tail_anchor: "'"
...

甲	abc
乙	def
"#,
        )
        .expect("dictionary should parse");

        let encoder = dictionary.encoder();
        assert!(encoder.loaded());
        assert_eq!(encoder.max_phrase_length(), 5);
        assert_eq!(encoder.rules().len(), 2);
        assert_eq!(encoder.rules()[0].min_word_length, 2);
        assert_eq!(encoder.rules()[0].max_word_length, 2);
        assert_eq!(encoder.rules()[1].min_word_length, 3);
        assert_eq!(encoder.rules()[1].max_word_length, 5);
        assert!(encoder.is_code_excluded("xyz"));
        assert!(!encoder.is_code_excluded("axyz"));
        assert_eq!(
            encoder.encode(&["zyx'wvu'tsr", "qpo'nmlk'jih", "gfedcba"]),
            Some("zqga".to_owned())
        );
    }

    #[test]
    fn parses_rime_dict_yaml_rule_encoder_phrase_entries_like_librime_entry_collector() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            r#"
---
name: encoder_phrase_sample
version: "0.1"
sort: by_weight
use_preset_vocabulary: true
max_phrase_length: 2
min_phrase_weight: 10
encoder:
  rules:
    - length_equal: 2
      formula: "AaBa"
...

你	ni	10
好	hao	9
您	nin	8
你好		50%
"#,
            std::iter::empty::<&str>(),
            |_| None,
            |name| {
                (name == "essay").then(|| {
                    "\
你好\t12
您好\t11
你好啊\t20
低频\t9
"
                    .to_owned()
                })
            },
        )
        .expect("rule-based encoder phrases should parse");

        let entries = dictionary.entries();
        let encoded_source_phrase = entries
            .iter()
            .find(|entry| entry.text == "你好")
            .expect("source phrase should be encoded");
        assert_eq!(encoded_source_phrase.code, "nh");
        assert_eq!(encoded_source_phrase.weight, 6.0);
        assert!(!entries
            .iter()
            .any(|entry| entry.text == "你好" && entry.code.is_empty()));

        let injected_phrase = entries
            .iter()
            .find(|entry| entry.text == "您好")
            .expect("preset phrase should be injected when all characters are encodable");
        assert_eq!(injected_phrase.code, "nh");
        assert_eq!(injected_phrase.weight, 11.0);
        assert!(!entries.iter().any(|entry| entry.text == "你好啊"));
        assert!(!entries.iter().any(|entry| entry.text == "低频"));
    }

    #[test]
    fn table_encoder_parses_librime_formula_settings() {
        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAzBaBz")
            .expect("librime encoder formula should parse");
        encoder
            .add_length_equal_rule(3, "AaBaCaBz")
            .expect("librime encoder formula should parse");
        encoder
            .add_length_in_range_rule(4, 9, "AaBaCaZz")
            .expect("librime encoder formula should parse");

        assert!(encoder.loaded());
        assert_eq!(encoder.max_phrase_length(), 9);
        assert_eq!(encoder.rules().len(), 3);
        assert_eq!(encoder.rules()[0].min_word_length, 2);
        assert_eq!(encoder.rules()[0].max_word_length, 2);
        assert_eq!(
            encoder.rules()[0].coords,
            [
                CodeCoords {
                    char_index: 0,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 0,
                    code_index: -1
                },
                CodeCoords {
                    char_index: 1,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 1,
                    code_index: -1
                },
            ]
        );
        assert_eq!(
            encoder.rules()[2].coords,
            [
                CodeCoords {
                    char_index: 0,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 1,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 2,
                    code_index: 0
                },
                CodeCoords {
                    char_index: -1,
                    code_index: -1
                },
            ]
        );
    }

    #[test]
    fn table_encoder_matches_librime_raw_code_encoding_cases() {
        let code2 = ["abc", "def"];
        let code3 = ["abc", "def", "ghi"];

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAbBaBb")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("abde".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_in_range_rule(3, 5, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code3), Some("acdfgi".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("acdf".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAbZyZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("abef".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAaBbBbZzZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("aaeeff".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_in_range_rule(3, 5, "AzAzByByZaZa")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code3), Some("cceegg".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaBaYaZaZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("adf".to_owned()));
    }

    #[test]
    fn table_encoder_honors_librime_exclude_patterns_and_tail_anchor() {
        let mut encoder = TableEncoder::new();
        encoder
            .set_exclude_patterns(["^x.*$"])
            .expect("exclude regex should compile");
        assert!(encoder.is_code_excluded("x"));
        assert!(encoder.is_code_excluded("xyz"));
        assert!(!encoder.is_code_excluded("XYZ"));
        assert!(!encoder.is_code_excluded("ax"));

        let code = ["zyx'wvu'tsr", "qpo'nmlk'jih", "gfedcba"];

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zxqoga".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAbAcAzBwBxByBz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zyxuqpo".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAbAcAdAzBaBxByBz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zyxwuqpo".to_owned()));
    }

    #[test]
    fn parses_rime_dict_yaml_inline_custom_columns() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_columns_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	八	10
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].code, "ba");
        assert_eq!(entries[1].text, "吧");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_header_scalars() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: quoted_header_sample
version: "0.1"
sort: 'original'
columns:
  - 'code'
  - "text"
  - 'weight'
...

ba	八	1
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, 9.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_scalars_with_comments() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: commented_header_sample
version: "0.1" # dictionary version
sort: original # preserve source order
columns:
  - code # input code
  - text # candidate text
  - weight # absolute weight
...

ba	八	1
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, 9.0);
    }

    #[test]
    fn parses_rime_dict_yaml_block_lists_after_commented_keys() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: commented_list_key_primary
version: "0.1"
sort: original
columns: # dictionary field order
  - code
  - text
  - weight
import_tables: # extra tables
  - secondary
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
columns: # imported field order
  - code
  - text
  - weight
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts comments after block-list mapping keys");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
        assert_eq!(entries[1].weight, 2.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_columns_with_trailing_comment() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: commented_inline_columns_sample
version: "0.1"
sort: original
columns: [code, text, weight] # inline RIME config comment
...

ba	八	10
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "吧");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_empty_required_header_scalars() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: ""
version: ''
sort: original
...

八	ba	1
"#,
        )
        .expect("quoted empty required metadata is a present YAML scalar");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_text_only_entries_for_later_encoding() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: text_only_sample
version: "0.1"
sort: original
columns: [text, weight]
...

你好	10
你	1
"#,
        )
        .expect("dictionary with text-only entries should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "你好");
        assert_eq!(entries[0].code, "");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "你");
        assert_eq!(entries[1].code, "");
        assert_eq!(entries[1].weight, 1.0);
    }

    #[test]
    fn parses_rime_dict_yaml_preserves_raw_text_column_whitespace() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: spaced_text_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	 八 	10
"#,
        )
        .expect("RIME dictionary text fields should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, " 八 ");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_weight_numeric_prefixes() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: weight_prefix_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	八	10oops
ba	吧	-2.5x
ba	巴	abc
ba	把	50%
"#,
        )
        .expect("dictionary with librime-style row weights should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, -2.5);
        assert_eq!(entries[2].text, "巴");
        assert_eq!(entries[2].weight, 0.0);
        assert_eq!(entries[3].text, "把");
        assert_eq!(entries[3].weight, 0.0);
    }

    #[test]
    fn parses_rime_dict_yaml_no_comment_marker_as_literal_hash_entries() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: no_comment_sample
version: "0.1"
sort: original
columns: [text, code, weight]
...

# skipped comment
# no comment
#hash	ha	1
#literal	li	2
"#,
        )
        .expect("RIME dictionary '# no comment' marker should allow literal hash entries");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "#hash");
        assert_eq!(entries[0].code, "ha");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "#literal");
        assert_eq!(entries[1].code, "li");
        assert_eq!(entries[1].weight, 2.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_keys_with_space_before_colon() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name : spaced_colon_primary
version : "0.1"
sort : original
columns : [code, text, weight]
import_tables : [secondary]
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name : secondary
version : "0.1"
sort : original
columns : [code, text, weight]
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts whitespace before mapping-key colons");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_header_keys() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
"name": quoted_key_primary
'version': "0.1"
"sort": original
'columns': [code, text, weight]
"import_tables": [secondary]
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
'name': secondary
"version": "0.1"
"sort": original
'columns': [code, text, weight]
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts quoted dictionary header mapping keys");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_null_columns_as_default_columns() {
        for columns_header in ["columns:", "columns: null", "columns: ~"] {
            let dictionary = TableDictionary::parse_rime_dict_yaml(&format!(
                r#"
---
name: null_columns_sample
version: "0.1"
sort: original
{columns_header}
...

八	ba	10
"#
            ))
            .expect("null columns should use the default RIME column order");

            let entries = dictionary.entries();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].text, "八");
            assert_eq!(entries[0].code, "ba");
            assert_eq!(entries[0].weight, 10.0);
        }
    }

    #[test]
    fn parses_rime_dict_yaml_scalar_columns_as_explicit_empty_list() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: scalar_columns_sample
version: "0.1"
sort: original
columns: text
...

八	ba	10
"#,
        )
        .expect("scalar columns are non-null but not a ConfigList in librime");

        assert!(dictionary.entries().is_empty());
    }

    #[test]
    fn parses_rime_dict_yaml_null_column_items_as_placeholders() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: null_column_item_sample
version: "0.1"
sort: original
columns:
  -
  - text
  - code
  - ''
  - weight
...

ignored	八	ba	ignored	10
"#,
        )
        .expect("YAML-null column items should still occupy a column position");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_null_column_items_as_placeholders() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_null_column_item_sample
version: "0.1"
sort: original
columns: [, text, code, '', weight]
...

ignored	八	ba	ignored	10
"#,
        )
        .expect("inline YAML-null column items should still occupy column positions");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_quoted_commas_as_single_column_items() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_quoted_comma_column_sample
version: "0.1"
sort: original
columns: ['ignored,placeholder', text, code, weight]
...

ignored	八	ba	10
"#,
        )
        .expect("quoted commas in YAML flow lists should not split column items");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_without_document_start() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
name: no_document_start_sample
version: "0.1"
sort: original
...

八	ba	10
"#,
        )
        .expect("librime loads dictionary headers as YAML streams without requiring '---'");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_with_utf8_bom() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            "\u{feff}name: bom_header_sample\nversion: \"0.1\"\nsort: original\n...\n\n八\tba\t10\n",
        )
        .expect("yaml-cpp accepts a leading UTF-8 BOM before the dictionary header");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_import_tables_with_primary_sorting() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
import_tables:
  - primary
  - secondary
...

八	ba	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	爸	9
ba	吧	3
"#
                    .to_owned()
                })
            },
        )
        .expect("dictionary imports should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "八");
    }

    #[test]
    fn parses_rime_dict_yaml_schema_packs_as_optional_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_and_packs(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
...

爸	ba	1
八	ba	2
"#,
            ["pack", "missing_pack", "broken_pack"],
            |name| match name {
                "pack" => Some(
                    r#"
---
name: pack
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	爸	9
ba	吧	3
"#
                    .to_owned(),
                ),
                "broken_pack" => Some("name: broken\n".to_owned()),
                _ => None,
            },
        )
        .expect("dictionary packs should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "八");
        assert_eq!(entries[3].text, "爸");
        assert_eq!(entries[3].weight, 1.0);
    }

    #[test]
    fn parses_rime_dict_yaml_preset_vocabulary_weights() {
        let mut requested_vocabulary = Vec::new();
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
vocabulary: custom
import_tables:
  - secondary
...

八	ba
吧	ba	50%
白	bai	7
"#,
            std::iter::empty::<&str>(),
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

爸	ba
"#
                    .to_owned()
                })
            },
            |name| {
                requested_vocabulary.push(name.to_owned());
                (name == "custom").then(|| {
                    "\
八\t8
吧\t6
爸\t9
"
                    .to_owned()
                })
            },
        )
        .expect("dictionary with preset vocabulary weights should parse");

        let entries = dictionary.entries();
        assert_eq!(requested_vocabulary, ["custom"]);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[0].weight, 9.0);
        assert_eq!(entries[1].text, "八");
        assert_eq!(entries[1].weight, 8.0);
        assert_eq!(entries[2].text, "吧");
        assert_eq!(entries[2].weight, 3.0);
        assert_eq!(entries[3].text, "白");
        assert_eq!(entries[3].weight, 7.0);
    }

    #[test]
    fn parses_rime_dict_yaml_skips_null_import_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [null, ~, secondary, 'null']
...

八	ba	1
"#,
            |name| match name {
                "secondary" => Some(
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

吧	ba	2
"#
                    .to_owned(),
                ),
                "null" => Some(
                    r#"
---
name: 'null'
version: "0.1"
sort: original
...

爸	ba	3
"#
                    .to_owned(),
                ),
                _ => None,
            },
        )
        .expect("YAML-null import tables should be skipped like librime config nodes");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "爸");
    }

    #[test]
    fn parses_rime_dict_yaml_unescapes_quoted_import_table_names() {
        let mut requested_imports = Vec::new();
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: escaped_import_sample
version: "0.1"
sort: original
import_tables: ['sec''ondary', "third\"table", "hex\x5ftable", "unicode\u005ftable", "long\U0000005ftable"]
...

primary	pri	1
"#,
            |table| {
                requested_imports.push(table.to_owned());
                match table {
                    "sec'ondary" => Some(
                        r#"
---
name: "sec'ondary"
version: "0.1"
...

single quote	sq	2
"#
                        .to_owned(),
                    ),
                    "third\"table" => Some(
                        r#"
---
name: 'third"table'
version: "0.1"
...

double quote	dq	3
"#
                        .to_owned(),
                    ),
                    "hex_table" => Some(
                        r#"
---
name: hex_table
version: "0.1"
...

hex escape	he	4
"#
                        .to_owned(),
                    ),
                    "unicode_table" => Some(
                        r#"
---
name: unicode_table
version: "0.1"
...

unicode escape	ue	5
"#
                        .to_owned(),
                    ),
                    "long_table" => Some(
                        r#"
---
name: long_table
version: "0.1"
...

long unicode escape	le	6
"#
                        .to_owned(),
                    ),
                    _ => None,
                }
            },
        )
        .expect("quoted YAML import table names should be unescaped like yaml-cpp scalars");

        assert_eq!(
            requested_imports,
            [
                "sec'ondary",
                "third\"table",
                "hex_table",
                "unicode_table",
                "long_table"
            ]
        );
        let entries = dictionary.entries();
        assert_eq!(entries[0].text, "primary");
        assert_eq!(entries[1].text, "single quote");
        assert_eq!(entries[2].text, "double quote");
        assert_eq!(entries[3].text, "hex escape");
        assert_eq!(entries[4].text, "unicode escape");
        assert_eq!(entries[5].text, "long unicode escape");
    }

    #[test]
    fn parses_rime_dict_yaml_skips_collection_import_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [[ignored, missing], {name: skipped}, secondary, '[literal]']
...

八	ba	1
"#,
            |name| match name {
                "secondary" => Some(
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

吧	ba	2
"#
                    .to_owned(),
                ),
                "[literal]" => Some(
                    r#"
---
name: '[literal]'
version: "0.1"
sort: original
...

爸	ba	3
"#
                    .to_owned(),
                ),
                other => panic!("non-scalar import table should be skipped, got {other}"),
            },
        )
        .expect("non-scalar import table items should be skipped like librime config nodes");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "爸");
    }

    #[test]
    fn parses_rime_dict_yaml_drops_duplicate_word_code_definitions() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [secondary]
...

八	ba	1
八	ba	99
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

八	ba	88
吧	ba	3
"#
                    .to_owned()
                })
            },
        )
        .expect("dictionary imports should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_preserves_duplicate_phrase_code_definitions() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: phrase_duplicate_sample
version: "0.1"
sort: original
...

你好	ni hao	1
你好	ni hao	2
你	ni	3
你	ni	4
"#,
        )
        .expect("dictionary with duplicate phrase code definitions should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "你好");
        assert_eq!(entries[0].code, "nihao");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "你好");
        assert_eq!(entries[1].code, "nihao");
        assert_eq!(entries[1].weight, 2.0);
        assert_eq!(entries[2].text, "你");
        assert_eq!(entries[2].code, "ni");
        assert_eq!(entries[2].weight, 3.0);
    }

    #[test]
    fn rejects_rime_dict_yaml_with_incomplete_header() {
        let missing_name = TableDictionary::parse_rime_dict_yaml(
            r#"
---
version: "0.1"
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary without a name should be rejected");
        assert_eq!(
            missing_name.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let missing_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary without a version should be rejected");
        assert_eq!(
            missing_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let commented_blank_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
version: # dictionary version is missing
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with a blank commented version should be rejected");
        assert_eq!(
            commented_blank_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let null_name = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: null
version: "0.1"
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with YAML null name should be rejected");
        assert_eq!(
            null_name.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let null_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
version: ~
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with YAML null version should be rejected");
        assert_eq!(
            null_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );
    }
