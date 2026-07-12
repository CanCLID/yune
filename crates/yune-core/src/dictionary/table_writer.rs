use super::{
    compiled_table::{
        RIME_TABLE_HEADER_LEN, YUNE_TABLE_METADATA_MARKER, YUNE_TABLE_METADATA_PAYLOAD_LEN,
        YUNE_TABLE_METADATA_VERSION, YUNE_TABLE_SORT_BY_WEIGHT, YUNE_TABLE_SORT_ORIGINAL,
        YUNE_TABLE_WEIGHT_NATURAL_LOG,
    },
    RimeCorrectionEntry, RimeToleranceRule, TableDictionary, TableEncodingRule,
    TableEntryWeightDomain,
};

pub fn build_table_bin(dict: &TableDictionary, dict_file_checksum: u32) -> Vec<u8> {
    let mut entries_by_code: Vec<(&str, Vec<usize>)> = Vec::new();
    for (entry_index, entry) in dict.entries().iter().enumerate() {
        if let Some((_, entries)) = entries_by_code
            .iter_mut()
            .find(|(code, _)| *code == entry.code)
        {
            entries.push(entry_index);
        } else {
            entries_by_code.push((&entry.code, vec![entry_index]));
        }
    }

    let mut bytes = vec![0; RIME_TABLE_HEADER_LEN];
    put_c_string(&mut bytes, 0, b"Rime::Table/4.0");
    put_u32_le(&mut bytes, 32, dict_file_checksum);
    put_u32_le(&mut bytes, 36, entries_by_code.len() as u32);
    put_u32_le(&mut bytes, 40, dict.entries().len() as u32);

    append_table_metadata(&mut bytes, dict.sort_by_weight());

    let syllabary_offset = bytes.len();
    bytes.resize(syllabary_offset + 4 + entries_by_code.len() * 4, 0);
    put_u32_le(&mut bytes, syllabary_offset, entries_by_code.len() as u32);
    let code_offsets = entries_by_code
        .iter()
        .map(|(code, _)| append_c_string(&mut bytes, code))
        .collect::<Vec<_>>();
    for (index, offset) in code_offsets.into_iter().enumerate() {
        put_offset(&mut bytes, syllabary_offset + 4 + index * 4, offset);
    }

    let index_offset = bytes.len();
    bytes.resize(index_offset + 4 + entries_by_code.len() * 16, 0);
    put_u32_le(&mut bytes, index_offset, entries_by_code.len() as u32);
    for (index, (_, entries)) in entries_by_code.iter().enumerate() {
        let node_offset = index_offset + 4 + index * 16;
        put_u32_le(&mut bytes, node_offset, entries.len() as u32);
        let entry_offset = bytes.len();
        bytes.resize(entry_offset + entries.len() * 8, 0);
        for (row_index, entry_index) in entries.iter().copied().enumerate() {
            let entry = &dict.entries()[entry_index];
            let current_entry_offset = entry_offset + row_index * 8;
            let text_offset = append_c_string(&mut bytes, &entry.text);
            put_offset(&mut bytes, current_entry_offset, text_offset);
            let compiled_weight = match dict.entry_weight_domain() {
                TableEntryWeightDomain::Raw => {
                    dict.compiler_weight(entry_index).max(f64::EPSILON).ln() as f32
                }
                TableEntryWeightDomain::NaturalLog => entry.weight,
            };
            put_f32_le(&mut bytes, current_entry_offset + 4, compiled_weight);
        }
        put_offset(&mut bytes, node_offset + 4, entry_offset);
    }
    put_offset(&mut bytes, 44, syllabary_offset);
    put_offset(&mut bytes, 48, index_offset);

    append_advanced_payload(&mut bytes, dict);
    bytes
}

fn append_table_metadata(bytes: &mut Vec<u8>, sort_by_weight: bool) {
    // `Rime::Table/4.0` readers follow the syllabary offset at header field 44,
    // so this optional gap is invisible to librime and legacy Yune readers.
    // v2 declares both sort policy and the standard natural-log table weight
    // domain. Emitting it for by-weight tables too makes new Yune artifacts
    // distinguishable from the legacy raw-weight writer without changing the
    // Rime::Table/4.0 ABI or any offset-following reader.
    bytes.extend_from_slice(YUNE_TABLE_METADATA_MARKER);
    put_u32_le_extend(bytes, YUNE_TABLE_METADATA_VERSION);
    put_u32_le_extend(bytes, YUNE_TABLE_METADATA_PAYLOAD_LEN as u32);
    bytes.extend_from_slice(&[
        if sort_by_weight {
            YUNE_TABLE_SORT_BY_WEIGHT
        } else {
            YUNE_TABLE_SORT_ORIGINAL
        },
        YUNE_TABLE_WEIGHT_NATURAL_LOG,
        0,
        0,
    ]);
    debug_assert_eq!(bytes.len() % 4, 0);
}

fn append_advanced_payload(bytes: &mut Vec<u8>, dict: &TableDictionary) {
    bytes.extend_from_slice(b"YUNE-TABLE-ADV\0");

    put_u32_le_extend(bytes, dict.stems().len() as u32);
    let mut stems = dict.stems().iter().collect::<Vec<_>>();
    stems.sort_by(|left, right| left.0.cmp(right.0));
    for (text, values) in stems {
        put_len_string(bytes, text);
        put_u32_le_extend(bytes, values.len() as u32);
        for stem in values {
            put_len_string(bytes, stem);
        }
    }

    put_u32_le_extend(bytes, 0);

    let rules = dict
        .encoder()
        .rules()
        .iter()
        .filter(|rule| rule.min_word_length == rule.max_word_length)
        .collect::<Vec<_>>();
    put_u32_le_extend(bytes, rules.len() as u32);
    for rule in rules {
        put_u32_le_extend(bytes, rule.min_word_length as u32);
        put_len_string(bytes, &formula_from_rule(rule));
    }

    append_correction_tolerance_payload(bytes, dict.corrections(), dict.tolerance_rules());
    append_lookup_record_payload(bytes, dict);
}

fn append_correction_tolerance_payload(
    bytes: &mut Vec<u8>,
    corrections: &[RimeCorrectionEntry],
    tolerance_rules: &[RimeToleranceRule],
) {
    bytes.extend_from_slice(b"YUNE-CORR-TOL\0");
    put_u32_le_extend(bytes, corrections.len() as u32);
    for correction in corrections {
        put_len_string(bytes, &correction.observed_input);
        put_len_string(bytes, &correction.canonical_code);
    }
    put_u32_le_extend(bytes, tolerance_rules.len() as u32);
    for rule in tolerance_rules {
        put_len_string(bytes, &rule.near_code);
        put_u32_le_extend(bytes, rule.candidate_codes.len() as u32);
        for candidate in &rule.candidate_codes {
            put_len_string(bytes, candidate);
        }
    }
}

fn append_lookup_record_payload(bytes: &mut Vec<u8>, dict: &TableDictionary) {
    bytes.extend_from_slice(b"YUNE-LOOKUP\0");
    let mut records_by_text = dict.lookup_records.iter().collect::<Vec<_>>();
    records_by_text.sort_by(|left, right| left.0.cmp(right.0));
    put_u32_le_extend(bytes, records_by_text.len() as u32);
    for (text, records) in records_by_text {
        put_len_string(bytes, text);
        put_u32_le_extend(bytes, records.len() as u32);
        for record in records {
            put_len_string(bytes, &record.code);
            put_u32_le_extend(bytes, record.fields.len() as u32);
            for field in &record.fields {
                put_len_string(bytes, field);
            }
        }
    }
}

fn formula_from_rule(rule: &TableEncodingRule) -> String {
    let mut formula = String::new();
    for coords in &rule.coords {
        formula.push(index_to_formula_char(coords.char_index, b'A'));
        formula.push(index_to_formula_char(coords.code_index, b'a'));
    }
    formula
}

fn index_to_formula_char(index: isize, lower: u8) -> char {
    let byte = if index >= 0 {
        lower + u8::try_from(index).expect("formula index should fit")
    } else {
        lower + 26 - u8::try_from(-index).expect("formula index should fit")
    };
    char::from(byte)
}

pub(crate) fn put_c_string(bytes: &mut [u8], offset: usize, value: &[u8]) {
    bytes[offset..offset + value.len()].copy_from_slice(value);
}

pub(crate) fn put_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_i32_le(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_f32_le(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_le_bytes());
}

pub(crate) fn put_u32_le_extend(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn put_offset(bytes: &mut [u8], field_offset: usize, target: usize) {
    let raw = i32::try_from(target as isize - field_offset as isize)
        .expect("compiled artifact offset should fit i32");
    put_i32_le(bytes, field_offset, raw);
}

pub(crate) fn append_c_string(bytes: &mut Vec<u8>, value: &str) -> usize {
    let offset = bytes.len();
    bytes.extend_from_slice(value.as_bytes());
    bytes.push(0);
    offset
}

pub(crate) fn put_len_string(bytes: &mut Vec<u8>, value: &str) {
    put_u32_le_extend(bytes, value.len() as u32);
    bytes.extend_from_slice(value.as_bytes());
}
