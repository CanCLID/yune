    #[test]
    fn parses_librime_compiled_table_prism_and_reverse_metadata() {
        let mut table = vec![0; 68];
        put_c_string(&mut table, 0, b"Rime::Table/4.0");
        put_u32_le(&mut table, 32, 0x1111_1111);
        put_u32_le(&mut table, 36, 7);
        put_u32_le(&mut table, 40, 11);
        put_u32_le(&mut table, 44, 0x40);
        put_u32_le(&mut table, 48, 0x44);
        put_u32_le(&mut table, 64, 13);
        assert_eq!(
            parse_rime_table_bin_metadata(&table),
            Ok(RimeTableBinMetadata {
                dict_file_checksum: 0x1111_1111,
                num_syllables: 7,
                num_entries: 11,
                string_table_size: 13,
            })
        );

        let mut prism = vec![0; 320];
        put_c_string(&mut prism, 0, b"Rime::Prism/4.0");
        put_u32_le(&mut prism, 32, 0x2222_2222);
        put_u32_le(&mut prism, 36, 0x3333_3333);
        put_u32_le(&mut prism, 40, 17);
        put_u32_le(&mut prism, 44, 19);
        put_u32_le(&mut prism, 48, 23);
        put_u32_le(&mut prism, 52, 0x50);
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Ok(RimePrismBinMetadata {
                dict_file_checksum: 0x2222_2222,
                schema_file_checksum: 0x3333_3333,
                num_syllables: 17,
                num_spellings: 19,
                double_array_size: 23,
            })
        );

        let mut reverse = vec![0; 64];
        put_c_string(&mut reverse, 0, b"Rime::Reverse/3.1");
        put_u32_le(&mut reverse, 32, 0x4444_4444);
        put_u32_le(&mut reverse, 52, 29);
        put_u32_le(&mut reverse, 60, 31);
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Ok(RimeReverseBinMetadata {
                dict_file_checksum: 0x4444_4444,
                key_trie_size: 29,
                value_trie_size: 31,
            })
        );
    }

    #[test]
    fn compiled_metadata_parser_matches_librime_load_rejection_cases() {
        let mut table = vec![0; 68];
        put_c_string(&mut table, 0, b"Rime::Table/3.0");
        assert_eq!(
            parse_rime_table_bin_metadata(&table),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut table, 0, b"Rime::Table/4.0");
        put_u32_le(&mut table, 44, 0x40);
        assert_eq!(
            parse_rime_table_bin_metadata(&table),
            Err(RimeCompiledMetadataError::MissingRequiredSection)
        );

        let mut prism = vec![0; 320];
        put_c_string(&mut prism, 0, b"Rime::Prism/3.9");
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut prism, 0, b"Rime::Prism/4.0");
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Err(RimeCompiledMetadataError::MissingRequiredSection)
        );

        let mut reverse = vec![0; 64];
        put_c_string(&mut reverse, 0, b"Rime::Reverse/2.9");
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut reverse, 0, b"Rime::Reverse/4.1");
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );

        let mut invalid = vec![0; 68];
        put_c_string(&mut invalid, 0, b"Rime::Wrong/4.0");
        assert_eq!(
            parse_rime_table_bin_metadata(&invalid),
            Err(RimeCompiledMetadataError::InvalidFormat)
        );
        assert_eq!(
            parse_rime_table_bin_metadata(&invalid[..20]),
            Err(RimeCompiledMetadataError::TooShort)
        );
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

    fn put_f32_le_extend(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }

    fn put_offset(bytes: &mut [u8], field_offset: usize, target: usize) {
        let raw = i32::try_from(target as isize - field_offset as isize)
            .expect("fixture offset should fit i32");
        put_i32_le(bytes, field_offset, raw);
    }

    fn append_c_string(bytes: &mut Vec<u8>, value: &str) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        offset
    }

    fn compiled_table_fixture() -> Vec<u8> {
        let mut bytes = vec![0; 68];
        put_c_string(&mut bytes, 0, b"Rime::Table/4.0");
        put_u32_le(&mut bytes, 32, 0x1111_1111);
        put_u32_le(&mut bytes, 36, 1);
        put_u32_le(&mut bytes, 40, 2);
        let syllabary_offset = bytes.len();
        bytes.resize(syllabary_offset + 8, 0);
        put_u32_le(&mut bytes, syllabary_offset, 1);
        let code_offset = append_c_string(&mut bytes, "ba");
        put_offset(&mut bytes, syllabary_offset + 4, code_offset);
        let index_offset = bytes.len();
        bytes.resize(index_offset + 16, 0);
        put_u32_le(&mut bytes, index_offset, 1);
        put_u32_le(&mut bytes, index_offset + 4, 2);
        let entries_offset = bytes.len();
        bytes.resize(entries_offset + 16, 0);
        let ba_offset = append_c_string(&mut bytes, "八");
        let ba2_offset = append_c_string(&mut bytes, "爸");
        put_offset(&mut bytes, entries_offset, ba_offset);
        put_f32_le(&mut bytes, entries_offset + 4, 2.0);
        put_offset(&mut bytes, entries_offset + 8, ba2_offset);
        put_f32_le(&mut bytes, entries_offset + 12, 1.0);
        put_offset(&mut bytes, index_offset + 8, entries_offset);
        put_offset(&mut bytes, 44, syllabary_offset);
        put_offset(&mut bytes, 48, index_offset);
        bytes
    }

    fn compiled_prism_fixture() -> Vec<u8> {
        let mut bytes = vec![0; 320];
        put_c_string(&mut bytes, 0, b"Rime::Prism/4.0");
        put_u32_le(&mut bytes, 32, 0x2222_2222);
        put_u32_le(&mut bytes, 36, 0x3333_3333);
        put_u32_le(&mut bytes, 40, 1);
        put_u32_le(&mut bytes, 44, 1);
        let spelling_map_offset = bytes.len();
        bytes.resize(spelling_map_offset + 12, 0);
        put_u32_le(&mut bytes, spelling_map_offset, 1);
        put_u32_le(&mut bytes, spelling_map_offset + 4, 1);
        let descriptor_offset = bytes.len();
        bytes.resize(descriptor_offset + 16, 0);
        let tips_offset = append_c_string(&mut bytes, "tip");
        put_i32_le(&mut bytes, descriptor_offset, 7);
        put_i32_le(&mut bytes, descriptor_offset + 4, (1 << 30) | 2);
        put_f32_le(&mut bytes, descriptor_offset + 8, 0.5);
        put_offset(&mut bytes, descriptor_offset + 12, tips_offset);
        put_offset(&mut bytes, spelling_map_offset + 8, descriptor_offset);
        let correction_offset = bytes.len();
        bytes.extend_from_slice(b"YUNE-CORR\0");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "bq");
        put_len_string(&mut bytes, "ba");
        let tolerance_offset = bytes.len();
        bytes.extend_from_slice(b"YUNE-TOL\0");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "bz");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "ba");
        put_offset(&mut bytes, 56, spelling_map_offset);
        put_offset(&mut bytes, 60, correction_offset);
        put_offset(&mut bytes, 64, tolerance_offset);
        bytes
    }

    fn compiled_reverse_fixture() -> Vec<u8> {
        let mut bytes = vec![0; 64];
        put_c_string(&mut bytes, 0, b"Rime::Reverse/4.0");
        put_u32_le(&mut bytes, 32, 0x4444_4444);
        bytes.extend_from_slice(b"YUNE-REVERSE\0");
        put_u32_le_extend(&mut bytes, 2);
        put_len_string(&mut bytes, "ba");
        put_len_string(&mut bytes, "八");
        put_len_string(&mut bytes, "ba");
        put_len_string(&mut bytes, "爸");
        bytes
    }

    fn compiled_table_advanced_fixture() -> Vec<u8> {
        let mut bytes = compiled_table_fixture();
        bytes.extend_from_slice(b"YUNE-TABLE-ADV\0");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "明");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "a'b");
        put_u32_le_extend(&mut bytes, 2);
        put_len_string(&mut bytes, "您好");
        put_len_string(&mut bytes, "nh");
        put_f32_le_extend(&mut bytes, 11.0);
        put_len_string(&mut bytes, "你好");
        put_len_string(&mut bytes, "nh");
        put_f32_le_extend(&mut bytes, 6.0);
        put_u32_le_extend(&mut bytes, 1);
        put_u32_le_extend(&mut bytes, 2);
        put_len_string(&mut bytes, "AaBa");
        bytes
    }

    fn put_u32_le_extend(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn put_len_string(bytes: &mut Vec<u8>, value: &str) {
        put_u32_le_extend(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    #[test]
    fn parses_compiled_table_fixture_into_dictionary_order() {
        let dictionary = parse_rime_table_bin_dictionary(compiled_table_fixture())
            .expect("compiled table should parse");
        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 2.0);
        assert_eq!(entries[1].text, "爸");
    }

    #[test]
    fn parses_compiled_table_advanced_payload_stems_vocabulary_and_encoder_entries() {
        let dictionary = parse_rime_table_bin_dictionary(compiled_table_advanced_fixture())
            .expect("advanced compiled table should parse");
        assert_eq!(dictionary.stems_for("明"), Some(&["a'b".to_owned()][..]));
        assert!(dictionary
            .entries()
            .iter()
            .any(|entry| entry.text == "您好" && entry.code == "nh" && entry.weight == 11.0));
        assert!(dictionary
            .entries()
            .iter()
            .any(|entry| entry.text == "你好" && entry.code == "nh"));
        assert!(dictionary.encoder().loaded());
    }

    #[test]
    fn parses_compiled_prism_fixture_payload() {
        let payload = parse_rime_prism_bin_payload(compiled_prism_fixture())
            .expect("compiled prism should parse");
        assert_eq!(payload.dict_file_checksum, 0x2222_2222);
        assert_eq!(payload.spelling_map.len(), 1);
        assert_eq!(payload.spelling_map[0][0].syllable_id, 7);
        assert_eq!(payload.spelling_map[0][0].spelling_type, 2);
        assert!(payload.spelling_map[0][0].is_correction);
        assert_eq!(payload.spelling_map[0][0].tips, "tip");
        assert_eq!(payload.corrections[0].observed_input, "bq");
        assert_eq!(payload.corrections[0].canonical_code, "ba");
        assert_eq!(payload.tolerance_rules[0].near_code, "bz");
        assert_eq!(
            payload.tolerance_rules[0].candidate_codes,
            ["ba".to_owned()]
        );
    }

    #[test]
    fn parses_compiled_reverse_fixture_into_dictionary() {
        let dictionary = parse_rime_reverse_bin_dictionary(compiled_reverse_fixture())
            .expect("compiled reverse should parse");
        let texts = dictionary
            .entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["八", "爸"]);
    }

    #[test]
    fn parses_compiled_reverse_dict_settings_and_stems() {
        let mut bytes = compiled_reverse_fixture();
        put_u32_le_extend(&mut bytes, 2);
        put_len_string(&mut bytes, "tail_anchor");
        put_len_string(&mut bytes, "'");
        put_len_string(&mut bytes, "rules/0/formula");
        put_len_string(&mut bytes, "AaBa");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "明");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "a'b");

        let dictionary = parse_rime_reverse_bin_dictionary(bytes)
            .expect("advanced compiled reverse should parse");
        assert_eq!(
            dictionary.dict_settings().get("tail_anchor"),
            Some(&"'".to_owned())
        );
        assert_eq!(
            dictionary.dict_settings().get("rules/0/formula"),
            Some(&"AaBa".to_owned())
        );
        assert_eq!(dictionary.stems_for("明"), Some(&["a'b".to_owned()][..]));
    }

    #[test]
    fn compiled_payload_readers_reject_malformed_bytes() {
        assert_eq!(
            parse_rime_table_bin_dictionary(&compiled_table_fixture()[..20]),
            Err(RimeTableBinParseError::TooShort)
        );
        let mut bad_version = compiled_table_fixture();
        put_c_string(&mut bad_version, 0, b"Rime::Table/3.0");
        assert_eq!(
            parse_rime_table_bin_dictionary(bad_version),
            Err(RimeTableBinParseError::UnsupportedVersion)
        );
        let mut missing_section = compiled_table_fixture();
        put_i32_le(&mut missing_section, 44, 0);
        assert_eq!(
            parse_rime_table_bin_dictionary(missing_section),
            Err(RimeTableBinParseError::MissingRequiredSection)
        );
        let mut bad_offset = compiled_table_fixture();
        put_i32_le(&mut bad_offset, 44, i32::MAX);
        assert_eq!(
            parse_rime_table_bin_dictionary(bad_offset),
            Err(RimeTableBinParseError::OutOfBounds)
        );
        let mut huge_count = compiled_table_fixture();
        let index_offset = 79;
        put_u32_le(&mut huge_count, index_offset, u32::MAX);
        assert_eq!(
            parse_rime_table_bin_dictionary(huge_count),
            Err(RimeTableBinParseError::InvalidCount)
        );
        let mut invalid_utf8 = compiled_table_fixture();
        let last = invalid_utf8.len() - 1;
        invalid_utf8[last - 1] = 0xff;
        assert_eq!(
            parse_rime_table_bin_dictionary(invalid_utf8),
            Err(RimeTableBinParseError::InvalidUtf8)
        );
        let mut unsupported = compiled_table_fixture();
        put_offset(&mut unsupported, 60, 68);
        assert!(matches!(
            parse_rime_table_bin_dictionary(unsupported),
            Err(RimeTableBinParseError::UnsupportedSection { .. })
        ));

        let mut prism_unsupported = compiled_prism_fixture();
        put_u32_le(&mut prism_unsupported, 48, 4);
        put_offset(&mut prism_unsupported, 52, 320);
        assert!(matches!(
            parse_rime_prism_bin_payload(prism_unsupported),
            Err(RimePrismBinParseError::UnsupportedSection { .. })
        ));

        let mut reverse_unsupported = compiled_reverse_fixture();
        put_u32_le(&mut reverse_unsupported, 52, 1);
        assert!(matches!(
            parse_rime_reverse_bin_dictionary(reverse_unsupported),
            Err(RimeReverseBinParseError::UnsupportedSection { .. })
        ));
    }
