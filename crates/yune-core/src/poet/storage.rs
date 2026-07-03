use std::collections::HashMap;
use std::fmt;
use std::str;

use crate::{PresetVocabularyEntry, TableEntry};

use super::{
    build_model_vocabulary_index, compare_model_entry_by_code, pack_owned_model_entries,
    ModelEntry, ModelStringPool, ModelStringRange, ModelVocabularyEntry, OwnedModelEntry,
};

const MAGIC: &[u8; 12] = b"YUNE-POET/1\0";
const HEADER_LEN: usize = 24;
const SECTION_DIR_ENTRY_LEN: usize = 20;

const SECTION_ENTRIES: u32 = 1;
const SECTION_ENTRY_TEXT_POOL: u32 = 2;
const SECTION_ENTRY_CODE_POOL: u32 = 3;
const SECTION_ENTRY_CODE_RANGES: u32 = 4;
const SECTION_VOCABULARY: u32 = 5;
const SECTION_VOCABULARY_TEXT_POOL: u32 = 6;
const SECTION_VOCABULARY_CHARS: u32 = 7;
const SECTION_VOCABULARY_FIRST_CODES: u32 = 8;
const SECTION_VOCABULARY_FIRST_CODE_TEXT_POOL: u32 = 9;
const SECTION_ABBREVIATION_VOCABULARY: u32 = 10;
const SECTION_ABBREVIATION_VOCABULARY_TEXT_POOL: u32 = 11;
const SECTION_ABBREVIATION_VOCABULARY_CHARS: u32 = 12;
const SECTION_ABBREVIATION_VOCABULARY_FIRST_CODES: u32 = 13;
const SECTION_ABBREVIATION_FIRST_CODE_TEXT_POOL: u32 = 14;
const SECTION_CHARACTER_CODES: u32 = 15;
const SECTION_CHARACTER_CODE_TEXT_POOL: u32 = 16;
const SECTION_ABBREVIATION_CHARACTER_CODES: u32 = 17;
const SECTION_ABBREVIATION_CHARACTER_CODE_TEXT_POOL: u32 = 18;

const ENTRY_STRIDE: u32 = 16;
const RANGE_STRIDE: u32 = 8;
const VOCABULARY_STRIDE: u32 = 20;
const FIRST_CODE_STRIDE: u32 = 12;
const CHAR_CODE_STRIDE: u32 = 12;
const U32_STRIDE: u32 = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoetBinSectionSummary {
    pub id: u32,
    pub offset: u32,
    pub len: u32,
    pub count: u32,
    pub stride: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoetBinSummary {
    pub dictionary_checksum: u32,
    pub entries: u32,
    pub vocabulary_entries: u32,
    pub abbreviation_vocabulary_entries: u32,
    pub sections: Vec<PoetBinSectionSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoetBinParseError {
    TooShort,
    UnsupportedVersion,
    ChecksumMismatch { expected: u32, actual: u32 },
    InvalidSectionDirectory,
    DuplicateSection(u32),
    MissingSection(u32),
    SectionOutOfBounds(u32),
    InvalidSectionShape(u32),
    InvalidUtf8(u32),
    InvalidChar(u32),
}

impl fmt::Display for PoetBinParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort => write!(f, "poet bin is too short"),
            Self::UnsupportedVersion => write!(f, "unsupported poet bin version"),
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "poet bin checksum mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidSectionDirectory => write!(f, "invalid poet bin section directory"),
            Self::DuplicateSection(id) => write!(f, "duplicate poet bin section {id}"),
            Self::MissingSection(id) => write!(f, "missing poet bin section {id}"),
            Self::SectionOutOfBounds(id) => write!(f, "poet bin section {id} is out of bounds"),
            Self::InvalidSectionShape(id) => write!(f, "invalid poet bin section {id} shape"),
            Self::InvalidUtf8(id) => write!(f, "invalid UTF-8 in poet bin section {id}"),
            Self::InvalidChar(id) => write!(f, "invalid char scalar in poet bin section {id}"),
        }
    }
}

impl std::error::Error for PoetBinParseError {}

#[must_use]
pub fn build_poet_bin(
    entries: impl IntoIterator<Item = TableEntry>,
    vocabulary: &[PresetVocabularyEntry],
    abbreviation_vocabulary: &[PresetVocabularyEntry],
    dictionary_checksum: u32,
) -> Vec<u8> {
    let compiled = compile_poet_inputs(entries, vocabulary, abbreviation_vocabulary);
    let mut sections = Vec::new();
    let mut bytes = vec![0; HEADER_LEN];
    bytes[..MAGIC.len()].copy_from_slice(MAGIC);
    put_u32(&mut bytes, 12, dictionary_checksum);
    put_u32(&mut bytes, 16, 0);
    put_u32(&mut bytes, 20, 0);

    append_section(
        &mut bytes,
        &mut sections,
        SECTION_ENTRIES,
        ENTRY_STRIDE,
        encode_entries(&compiled.entries_by_code),
    );
    append_section(
        &mut bytes,
        &mut sections,
        SECTION_ENTRY_TEXT_POOL,
        1,
        compiled.entry_texts.bytes.as_bytes().to_vec(),
    );
    append_section(
        &mut bytes,
        &mut sections,
        SECTION_ENTRY_CODE_POOL,
        1,
        compiled.entry_codes.bytes.as_bytes().to_vec(),
    );
    append_section(
        &mut bytes,
        &mut sections,
        SECTION_ENTRY_CODE_RANGES,
        RANGE_STRIDE,
        encode_ranges(&compiled.entry_codes.ranges),
    );
    append_vocabulary_sections(
        &mut bytes,
        &mut sections,
        &compiled.vocabulary,
        &compiled.vocabulary_first_codes,
        VocabularySectionIds {
            rows: SECTION_VOCABULARY,
            text_pool: SECTION_VOCABULARY_TEXT_POOL,
            chars: SECTION_VOCABULARY_CHARS,
            first_codes: SECTION_VOCABULARY_FIRST_CODES,
            first_code_text_pool: SECTION_VOCABULARY_FIRST_CODE_TEXT_POOL,
        },
    );
    append_vocabulary_sections(
        &mut bytes,
        &mut sections,
        &compiled.abbreviation_vocabulary,
        &compiled.abbreviation_vocabulary_first_codes,
        VocabularySectionIds {
            rows: SECTION_ABBREVIATION_VOCABULARY,
            text_pool: SECTION_ABBREVIATION_VOCABULARY_TEXT_POOL,
            chars: SECTION_ABBREVIATION_VOCABULARY_CHARS,
            first_codes: SECTION_ABBREVIATION_VOCABULARY_FIRST_CODES,
            first_code_text_pool: SECTION_ABBREVIATION_FIRST_CODE_TEXT_POOL,
        },
    );
    append_character_code_sections(
        &mut bytes,
        &mut sections,
        &compiled.character_codes,
        SECTION_CHARACTER_CODES,
        SECTION_CHARACTER_CODE_TEXT_POOL,
    );
    append_character_code_sections(
        &mut bytes,
        &mut sections,
        &compiled.abbreviation_character_codes,
        SECTION_ABBREVIATION_CHARACTER_CODES,
        SECTION_ABBREVIATION_CHARACTER_CODE_TEXT_POOL,
    );

    let section_dir_offset = bytes.len();
    for section in &sections {
        put_u32_extend(&mut bytes, section.id);
        put_u32_extend(&mut bytes, section.offset);
        put_u32_extend(&mut bytes, section.len);
        put_u32_extend(&mut bytes, section.count);
        put_u32_extend(&mut bytes, section.stride);
    }
    put_u32(&mut bytes, 16, sections.len() as u32);
    put_u32(&mut bytes, 20, section_dir_offset as u32);
    bytes
}

pub fn parse_poet_bin_summary(
    bytes: &[u8],
    expected_dictionary_checksum: u32,
) -> Result<PoetBinSummary, PoetBinParseError> {
    let dictionary_checksum = parse_poet_bin_dictionary_checksum(bytes)?;
    if dictionary_checksum != expected_dictionary_checksum {
        return Err(PoetBinParseError::ChecksumMismatch {
            expected: expected_dictionary_checksum,
            actual: dictionary_checksum,
        });
    }
    let section_count = read_u32(bytes, 16).ok_or(PoetBinParseError::TooShort)? as usize;
    let section_dir_offset = read_u32(bytes, 20).ok_or(PoetBinParseError::TooShort)? as usize;
    let section_dir_len = section_count
        .checked_mul(SECTION_DIR_ENTRY_LEN)
        .ok_or(PoetBinParseError::InvalidSectionDirectory)?;
    let section_dir_end = section_dir_offset
        .checked_add(section_dir_len)
        .ok_or(PoetBinParseError::InvalidSectionDirectory)?;
    if section_dir_offset < HEADER_LEN || section_dir_end > bytes.len() {
        return Err(PoetBinParseError::InvalidSectionDirectory);
    }

    let mut sections = Vec::with_capacity(section_count);
    for index in 0..section_count {
        let offset = section_dir_offset + index * SECTION_DIR_ENTRY_LEN;
        let section = PoetBinSectionSummary {
            id: read_u32(bytes, offset).ok_or(PoetBinParseError::InvalidSectionDirectory)?,
            offset: read_u32(bytes, offset + 4)
                .ok_or(PoetBinParseError::InvalidSectionDirectory)?,
            len: read_u32(bytes, offset + 8).ok_or(PoetBinParseError::InvalidSectionDirectory)?,
            count: read_u32(bytes, offset + 12)
                .ok_or(PoetBinParseError::InvalidSectionDirectory)?,
            stride: read_u32(bytes, offset + 16)
                .ok_or(PoetBinParseError::InvalidSectionDirectory)?,
        };
        if sections
            .iter()
            .any(|existing: &PoetBinSectionSummary| existing.id == section.id)
        {
            return Err(PoetBinParseError::DuplicateSection(section.id));
        }
        validate_section_bounds(bytes, &section)?;
        sections.push(section);
    }

    let entries = required_section(&sections, SECTION_ENTRIES)?;
    let vocabulary = required_section(&sections, SECTION_VOCABULARY)?;
    let abbreviation_vocabulary = required_section(&sections, SECTION_ABBREVIATION_VOCABULARY)?;

    validate_entries(bytes, &sections)?;
    validate_vocabulary(
        bytes,
        &sections,
        SECTION_VOCABULARY,
        SECTION_VOCABULARY_TEXT_POOL,
        SECTION_VOCABULARY_CHARS,
        SECTION_VOCABULARY_FIRST_CODES,
        SECTION_VOCABULARY_FIRST_CODE_TEXT_POOL,
    )?;
    validate_vocabulary(
        bytes,
        &sections,
        SECTION_ABBREVIATION_VOCABULARY,
        SECTION_ABBREVIATION_VOCABULARY_TEXT_POOL,
        SECTION_ABBREVIATION_VOCABULARY_CHARS,
        SECTION_ABBREVIATION_VOCABULARY_FIRST_CODES,
        SECTION_ABBREVIATION_FIRST_CODE_TEXT_POOL,
    )?;
    validate_character_codes(
        bytes,
        &sections,
        SECTION_CHARACTER_CODES,
        SECTION_CHARACTER_CODE_TEXT_POOL,
    )?;
    validate_character_codes(
        bytes,
        &sections,
        SECTION_ABBREVIATION_CHARACTER_CODES,
        SECTION_ABBREVIATION_CHARACTER_CODE_TEXT_POOL,
    )?;

    Ok(PoetBinSummary {
        dictionary_checksum,
        entries: entries.count,
        vocabulary_entries: vocabulary.count,
        abbreviation_vocabulary_entries: abbreviation_vocabulary.count,
        sections,
    })
}

pub fn parse_poet_bin_dictionary_checksum(bytes: &[u8]) -> Result<u32, PoetBinParseError> {
    if bytes.len() < HEADER_LEN {
        return Err(PoetBinParseError::TooShort);
    }
    if &bytes[..MAGIC.len()] != MAGIC {
        return Err(PoetBinParseError::UnsupportedVersion);
    }
    read_u32(bytes, 12).ok_or(PoetBinParseError::TooShort)
}

struct CompiledPoetInputs {
    entries_by_code: Vec<ModelEntry>,
    entry_texts: ModelStringPool,
    entry_codes: ModelStringPool,
    vocabulary: Vec<ModelVocabularyEntry>,
    vocabulary_first_codes: Vec<(String, usize)>,
    abbreviation_vocabulary: Vec<ModelVocabularyEntry>,
    abbreviation_vocabulary_first_codes: Vec<(String, usize)>,
    character_codes: HashMap<char, Vec<String>>,
    abbreviation_character_codes: HashMap<char, Vec<String>>,
}

fn compile_poet_inputs(
    entries: impl IntoIterator<Item = TableEntry>,
    vocabulary: &[PresetVocabularyEntry],
    abbreviation_vocabulary: &[PresetVocabularyEntry],
) -> CompiledPoetInputs {
    let mut owned_entries = Vec::new();
    let mut character_codes: HashMap<char, Vec<String>> = HashMap::new();
    let mut abbreviation_character_codes: HashMap<char, Vec<String>> = HashMap::new();
    for entry in entries {
        if entry.code.is_empty() {
            continue;
        }
        let owned = OwnedModelEntry {
            text: entry.text,
            code: entry.code,
            weight: entry.weight,
        };
        let mut chars = owned.text.chars();
        if let Some(ch) = chars.next() {
            if chars.next().is_none() {
                character_codes
                    .entry(ch)
                    .or_default()
                    .push(owned.code.clone());
                if owned.weight > 0.0 {
                    abbreviation_character_codes
                        .entry(ch)
                        .or_default()
                        .push(owned.code.clone());
                }
            }
        }
        owned_entries.push(owned);
    }
    for codes in character_codes.values_mut() {
        codes.sort();
        codes.dedup();
    }
    for codes in abbreviation_character_codes.values_mut() {
        codes.sort();
        codes.dedup();
    }
    owned_entries.sort_by(compare_model_entry_by_code);
    let (entries_by_code, entry_texts, entry_codes) = pack_owned_model_entries(owned_entries);
    let (vocabulary, vocabulary_first_codes) =
        build_model_vocabulary_index(vocabulary, &character_codes);
    let (abbreviation_vocabulary, abbreviation_vocabulary_first_codes) =
        build_model_vocabulary_index(abbreviation_vocabulary, &abbreviation_character_codes);
    CompiledPoetInputs {
        entries_by_code,
        entry_texts,
        entry_codes,
        vocabulary,
        vocabulary_first_codes,
        abbreviation_vocabulary,
        abbreviation_vocabulary_first_codes,
        character_codes,
        abbreviation_character_codes,
    }
}

fn append_section(
    bytes: &mut Vec<u8>,
    sections: &mut Vec<PoetBinSectionSummary>,
    id: u32,
    stride: u32,
    payload: Vec<u8>,
) {
    let offset = bytes.len() as u32;
    let len = payload.len() as u32;
    let count = if stride == 1 {
        len
    } else {
        len.checked_div(stride)
            .expect("poet section stride is nonzero")
    };
    bytes.extend_from_slice(&payload);
    sections.push(PoetBinSectionSummary {
        id,
        offset,
        len,
        count,
        stride,
    });
}

#[derive(Clone, Copy)]
struct VocabularySectionIds {
    rows: u32,
    text_pool: u32,
    chars: u32,
    first_codes: u32,
    first_code_text_pool: u32,
}

fn append_vocabulary_sections(
    bytes: &mut Vec<u8>,
    sections: &mut Vec<PoetBinSectionSummary>,
    vocabulary: &[ModelVocabularyEntry],
    first_codes: &[(String, usize)],
    ids: VocabularySectionIds,
) {
    let encoded = encode_vocabulary(vocabulary);
    append_section(bytes, sections, ids.rows, VOCABULARY_STRIDE, encoded.rows);
    append_section(bytes, sections, ids.text_pool, 1, encoded.text_pool);
    append_section(bytes, sections, ids.chars, U32_STRIDE, encoded.chars);
    let encoded_first_codes = encode_first_codes(first_codes);
    append_section(
        bytes,
        sections,
        ids.first_codes,
        FIRST_CODE_STRIDE,
        encoded_first_codes.rows,
    );
    append_section(
        bytes,
        sections,
        ids.first_code_text_pool,
        1,
        encoded_first_codes.text_pool,
    );
}

fn append_character_code_sections(
    bytes: &mut Vec<u8>,
    sections: &mut Vec<PoetBinSectionSummary>,
    character_codes: &HashMap<char, Vec<String>>,
    rows_id: u32,
    text_pool_id: u32,
) {
    let encoded = encode_character_codes(character_codes);
    append_section(bytes, sections, rows_id, CHAR_CODE_STRIDE, encoded.rows);
    append_section(bytes, sections, text_pool_id, 1, encoded.text_pool);
}

fn encode_entries(entries: &[ModelEntry]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(entries.len() * ENTRY_STRIDE as usize);
    for entry in entries {
        put_u32_extend(&mut bytes, entry.text.start);
        put_u32_extend(&mut bytes, entry.text.end);
        put_u32_extend(&mut bytes, entry.code_id);
        put_u32_extend(&mut bytes, entry.weight.to_bits());
    }
    bytes
}

fn encode_ranges(ranges: &[ModelStringRange]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(ranges.len() * RANGE_STRIDE as usize);
    for range in ranges {
        put_u32_extend(&mut bytes, range.start);
        put_u32_extend(&mut bytes, range.end);
    }
    bytes
}

struct EncodedVocabulary {
    rows: Vec<u8>,
    text_pool: Vec<u8>,
    chars: Vec<u8>,
}

fn encode_vocabulary(vocabulary: &[ModelVocabularyEntry]) -> EncodedVocabulary {
    let mut rows = Vec::with_capacity(vocabulary.len() * VOCABULARY_STRIDE as usize);
    let mut text_pool = Vec::new();
    let mut chars = Vec::new();
    for entry in vocabulary {
        let text_start = text_pool.len() as u32;
        text_pool.extend_from_slice(entry.text.as_bytes());
        let text_end = text_pool.len() as u32;
        let chars_start = chars.len() as u32 / U32_STRIDE;
        for ch in &entry.chars {
            put_u32_extend(&mut chars, *ch as u32);
        }
        put_u32_extend(&mut rows, text_start);
        put_u32_extend(&mut rows, text_end);
        put_u32_extend(&mut rows, chars_start);
        put_u32_extend(&mut rows, entry.chars.len() as u32);
        put_u32_extend(&mut rows, entry.weight.to_bits());
    }
    EncodedVocabulary {
        rows,
        text_pool,
        chars,
    }
}

struct EncodedTextIndexRows {
    rows: Vec<u8>,
    text_pool: Vec<u8>,
}

fn encode_first_codes(first_codes: &[(String, usize)]) -> EncodedTextIndexRows {
    let mut rows = Vec::with_capacity(first_codes.len() * FIRST_CODE_STRIDE as usize);
    let mut text_pool = Vec::new();
    for (code, index) in first_codes {
        let start = text_pool.len() as u32;
        text_pool.extend_from_slice(code.as_bytes());
        let end = text_pool.len() as u32;
        put_u32_extend(&mut rows, start);
        put_u32_extend(&mut rows, end);
        put_u32_extend(&mut rows, *index as u32);
    }
    EncodedTextIndexRows { rows, text_pool }
}

fn encode_character_codes(character_codes: &HashMap<char, Vec<String>>) -> EncodedTextIndexRows {
    let mut rows = Vec::with_capacity(character_codes.len() * CHAR_CODE_STRIDE as usize);
    let mut text_pool = Vec::new();
    let mut character_codes = character_codes.iter().collect::<Vec<_>>();
    character_codes.sort_by_key(|(ch, _)| **ch);
    for (ch, codes) in character_codes {
        let start = text_pool.len() as u32;
        for code in codes {
            put_len_string(&mut text_pool, code);
        }
        let end = text_pool.len() as u32;
        put_u32_extend(&mut rows, *ch as u32);
        put_u32_extend(&mut rows, start);
        put_u32_extend(&mut rows, end);
    }
    EncodedTextIndexRows { rows, text_pool }
}

fn validate_entries(
    bytes: &[u8],
    sections: &[PoetBinSectionSummary],
) -> Result<(), PoetBinParseError> {
    let entries = required_section(sections, SECTION_ENTRIES)?;
    require_stride(entries, ENTRY_STRIDE)?;
    let text_pool = section_bytes(bytes, required_section(sections, SECTION_ENTRY_TEXT_POOL)?)?;
    let code_pool = section_bytes(bytes, required_section(sections, SECTION_ENTRY_CODE_POOL)?)?;
    let code_ranges = required_section(sections, SECTION_ENTRY_CODE_RANGES)?;
    require_stride(code_ranges, RANGE_STRIDE)?;
    for index in 0..entries.count as usize {
        let row = row_offset(entries, index);
        let text_start =
            read_u32(bytes, row).ok_or(PoetBinParseError::SectionOutOfBounds(SECTION_ENTRIES))?;
        let text_end = read_u32(bytes, row + 4)
            .ok_or(PoetBinParseError::SectionOutOfBounds(SECTION_ENTRIES))?;
        let code_id = read_u32(bytes, row + 8)
            .ok_or(PoetBinParseError::SectionOutOfBounds(SECTION_ENTRIES))?;
        validate_text_range(text_pool, text_start, text_end, SECTION_ENTRY_TEXT_POOL)?;
        if code_id >= code_ranges.count {
            return Err(PoetBinParseError::InvalidSectionShape(SECTION_ENTRIES));
        }
    }
    for index in 0..code_ranges.count as usize {
        let row = row_offset(code_ranges, index);
        let start = read_u32(bytes, row).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_ENTRY_CODE_RANGES,
        ))?;
        let end = read_u32(bytes, row + 4).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_ENTRY_CODE_RANGES,
        ))?;
        validate_text_range(code_pool, start, end, SECTION_ENTRY_CODE_POOL)?;
    }
    Ok(())
}

fn validate_vocabulary(
    bytes: &[u8],
    sections: &[PoetBinSectionSummary],
    vocabulary_id: u32,
    text_pool_id: u32,
    chars_id: u32,
    first_codes_id: u32,
    first_code_text_pool_id: u32,
) -> Result<(), PoetBinParseError> {
    let vocabulary = required_section(sections, vocabulary_id)?;
    require_stride(vocabulary, VOCABULARY_STRIDE)?;
    let text_pool = section_bytes(bytes, required_section(sections, text_pool_id)?)?;
    let chars = required_section(sections, chars_id)?;
    require_stride(chars, U32_STRIDE)?;
    for index in 0..vocabulary.count as usize {
        let row = row_offset(vocabulary, index);
        let text_start =
            read_u32(bytes, row).ok_or(PoetBinParseError::SectionOutOfBounds(vocabulary_id))?;
        let text_end =
            read_u32(bytes, row + 4).ok_or(PoetBinParseError::SectionOutOfBounds(vocabulary_id))?;
        let chars_start =
            read_u32(bytes, row + 8).ok_or(PoetBinParseError::SectionOutOfBounds(vocabulary_id))?;
        let chars_count = read_u32(bytes, row + 12)
            .ok_or(PoetBinParseError::SectionOutOfBounds(vocabulary_id))?;
        validate_text_range(text_pool, text_start, text_end, text_pool_id)?;
        validate_char_range(bytes, chars, chars_start, chars_count, chars_id)?;
    }

    let first_codes = required_section(sections, first_codes_id)?;
    require_stride(first_codes, FIRST_CODE_STRIDE)?;
    let first_code_text_pool =
        section_bytes(bytes, required_section(sections, first_code_text_pool_id)?)?;
    for index in 0..first_codes.count as usize {
        let row = row_offset(first_codes, index);
        let code_start =
            read_u32(bytes, row).ok_or(PoetBinParseError::SectionOutOfBounds(first_codes_id))?;
        let code_end = read_u32(bytes, row + 4)
            .ok_or(PoetBinParseError::SectionOutOfBounds(first_codes_id))?;
        let vocabulary_index = read_u32(bytes, row + 8)
            .ok_or(PoetBinParseError::SectionOutOfBounds(first_codes_id))?;
        validate_text_range(
            first_code_text_pool,
            code_start,
            code_end,
            first_code_text_pool_id,
        )?;
        if vocabulary_index >= vocabulary.count {
            return Err(PoetBinParseError::InvalidSectionShape(first_codes_id));
        }
    }
    Ok(())
}

fn validate_character_codes(
    bytes: &[u8],
    sections: &[PoetBinSectionSummary],
    rows_id: u32,
    text_pool_id: u32,
) -> Result<(), PoetBinParseError> {
    let rows = required_section(sections, rows_id)?;
    require_stride(rows, CHAR_CODE_STRIDE)?;
    let text_pool = section_bytes(bytes, required_section(sections, text_pool_id)?)?;
    for index in 0..rows.count as usize {
        let row = row_offset(rows, index);
        let scalar = read_u32(bytes, row).ok_or(PoetBinParseError::SectionOutOfBounds(rows_id))?;
        let start =
            read_u32(bytes, row + 4).ok_or(PoetBinParseError::SectionOutOfBounds(rows_id))?;
        let end = read_u32(bytes, row + 8).ok_or(PoetBinParseError::SectionOutOfBounds(rows_id))?;
        char::from_u32(scalar).ok_or(PoetBinParseError::InvalidChar(rows_id))?;
        validate_len_string_range(text_pool, start, end, text_pool_id)?;
    }
    Ok(())
}

fn validate_char_range(
    bytes: &[u8],
    section: &PoetBinSectionSummary,
    start: u32,
    count: u32,
    section_id: u32,
) -> Result<(), PoetBinParseError> {
    let end = start
        .checked_add(count)
        .ok_or(PoetBinParseError::InvalidSectionShape(section_id))?;
    if end > section.count {
        return Err(PoetBinParseError::InvalidSectionShape(section_id));
    }
    for index in start..end {
        let offset = row_offset(section, index as usize);
        let scalar =
            read_u32(bytes, offset).ok_or(PoetBinParseError::SectionOutOfBounds(section_id))?;
        char::from_u32(scalar).ok_or(PoetBinParseError::InvalidChar(section_id))?;
    }
    Ok(())
}

fn validate_text_range(
    pool: &[u8],
    start: u32,
    end: u32,
    section_id: u32,
) -> Result<(), PoetBinParseError> {
    let start = start as usize;
    let end = end as usize;
    if start > end || end > pool.len() {
        return Err(PoetBinParseError::InvalidSectionShape(section_id));
    }
    str::from_utf8(&pool[start..end]).map_err(|_| PoetBinParseError::InvalidUtf8(section_id))?;
    Ok(())
}

fn validate_len_string_range(
    pool: &[u8],
    start: u32,
    end: u32,
    section_id: u32,
) -> Result<(), PoetBinParseError> {
    let mut cursor = start as usize;
    let end = end as usize;
    if cursor > end || end > pool.len() {
        return Err(PoetBinParseError::InvalidSectionShape(section_id));
    }
    while cursor < end {
        let Some(len) = read_u32(pool, cursor) else {
            return Err(PoetBinParseError::InvalidSectionShape(section_id));
        };
        cursor += 4;
        let string_end = cursor
            .checked_add(len as usize)
            .ok_or(PoetBinParseError::InvalidSectionShape(section_id))?;
        if string_end > end {
            return Err(PoetBinParseError::InvalidSectionShape(section_id));
        }
        str::from_utf8(&pool[cursor..string_end])
            .map_err(|_| PoetBinParseError::InvalidUtf8(section_id))?;
        cursor = string_end;
    }
    if cursor != end {
        return Err(PoetBinParseError::InvalidSectionShape(section_id));
    }
    Ok(())
}

fn validate_section_bounds(
    bytes: &[u8],
    section: &PoetBinSectionSummary,
) -> Result<(), PoetBinParseError> {
    if section.stride == 0 {
        return Err(PoetBinParseError::InvalidSectionShape(section.id));
    }
    if section.stride != 1 && section.len % section.stride != 0 {
        return Err(PoetBinParseError::InvalidSectionShape(section.id));
    }
    if section.stride != 1 && section.count != section.len / section.stride {
        return Err(PoetBinParseError::InvalidSectionShape(section.id));
    }
    if section.stride == 1 && section.count != section.len {
        return Err(PoetBinParseError::InvalidSectionShape(section.id));
    }
    let start = section.offset as usize;
    let end = start
        .checked_add(section.len as usize)
        .ok_or(PoetBinParseError::SectionOutOfBounds(section.id))?;
    if start < HEADER_LEN || end > bytes.len() {
        return Err(PoetBinParseError::SectionOutOfBounds(section.id));
    }
    Ok(())
}

fn required_section(
    sections: &[PoetBinSectionSummary],
    id: u32,
) -> Result<&PoetBinSectionSummary, PoetBinParseError> {
    sections
        .iter()
        .find(|section| section.id == id)
        .ok_or(PoetBinParseError::MissingSection(id))
}

fn require_stride(section: &PoetBinSectionSummary, expected: u32) -> Result<(), PoetBinParseError> {
    if section.stride == expected {
        Ok(())
    } else {
        Err(PoetBinParseError::InvalidSectionShape(section.id))
    }
}

fn section_bytes<'a>(
    bytes: &'a [u8],
    section: &PoetBinSectionSummary,
) -> Result<&'a [u8], PoetBinParseError> {
    let start = section.offset as usize;
    let end = start
        .checked_add(section.len as usize)
        .ok_or(PoetBinParseError::SectionOutOfBounds(section.id))?;
    bytes
        .get(start..end)
        .ok_or(PoetBinParseError::SectionOutOfBounds(section.id))
}

fn row_offset(section: &PoetBinSectionSummary, index: usize) -> usize {
    section.offset as usize + index * section.stride as usize
}

fn put_len_string(bytes: &mut Vec<u8>, value: &str) {
    put_u32_extend(bytes, value.len() as u32);
    bytes.extend_from_slice(value.as_bytes());
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u32_extend(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let raw = bytes.get(offset..end)?;
    Some(u32::from_le_bytes(raw.try_into().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_poet_bin() -> Vec<u8> {
        build_poet_bin(
            [
                TableEntry::new("ni", "你", 10.0),
                TableEntry::new("hao", "好", 8.0),
                TableEntry::new("nihao", "你好", 4.0),
            ],
            &[PresetVocabularyEntry::new("你好", 5.0)],
            &[PresetVocabularyEntry::new("你好", 5.0)],
            0xAABBCCDD,
        )
    }

    #[test]
    fn poet_bin_summary_validates_versioned_sections() {
        let bytes = sample_poet_bin();
        let summary = parse_poet_bin_summary(&bytes, 0xAABBCCDD).expect("poet bin should parse");

        assert_eq!(summary.dictionary_checksum, 0xAABBCCDD);
        assert_eq!(summary.entries, 3);
        assert_eq!(summary.vocabulary_entries, 1);
        assert_eq!(summary.abbreviation_vocabulary_entries, 1);
        assert_eq!(summary.sections.len(), 18);
    }

    #[test]
    fn poet_bin_rejects_wrong_version() {
        let mut bytes = sample_poet_bin();
        bytes[0] = b'X';

        assert_eq!(
            parse_poet_bin_summary(&bytes, 0xAABBCCDD),
            Err(PoetBinParseError::UnsupportedVersion),
        );
    }

    #[test]
    fn poet_bin_rejects_truncated_artifact() {
        let bytes = sample_poet_bin();
        let truncated = &bytes[..bytes.len() - 3];

        assert!(matches!(
            parse_poet_bin_summary(truncated, 0xAABBCCDD),
            Err(PoetBinParseError::InvalidSectionDirectory)
                | Err(PoetBinParseError::SectionOutOfBounds(_))
        ));
    }

    #[test]
    fn poet_bin_rejects_checksum_mismatch() {
        let bytes = sample_poet_bin();

        assert_eq!(
            parse_poet_bin_summary(&bytes, 0x01020304),
            Err(PoetBinParseError::ChecksumMismatch {
                expected: 0x01020304,
                actual: 0xAABBCCDD,
            }),
        );
    }
}
