use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::ops::Range;
use std::str;
use std::sync::Arc;

use crate::{PresetVocabularyEntry, TableEntry};

use super::{
    build_model_vocabulary_index, build_script_encoder_character_codes,
    compare_model_entry_by_code, pack_owned_model_entries, script_encoder_phrase_vocabulary,
    EntryWeightDomain, ModelEntry, ModelStringPool, ModelStringRange, ModelVocabularyEntry,
    OwnedModelEntry,
};

// Version 3 changes the compiled ScriptEncoder character-code map: readings
// below five percent of a character's total weight are no longer admitted.
// Keep this in the artifact identity so an otherwise checksum-fresh v2 file
// cannot preserve the pre-M59 false phrase surface after an upgrade.
const MAGIC: &[u8; 12] = b"YUNE-POET/3\0";
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
const SECTION_ENTRY_ROW_RANGES: u32 = 19;
const SECTION_CODE_PREFIX_INDEX: u32 = 20;

const ENTRY_STRIDE: u32 = 16;
const RANGE_STRIDE: u32 = 8;
const PREFIX_INDEX_STRIDE: u32 = 32;
const VOCABULARY_STRIDE: u32 = 20;
const FIRST_CODE_STRIDE: u32 = 12;
const CHAR_CODE_STRIDE: u32 = 12;
const U32_STRIDE: u32 = 4;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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
    append_section(
        &mut bytes,
        &mut sections,
        SECTION_ENTRY_ROW_RANGES,
        RANGE_STRIDE,
        encode_entry_row_ranges(&compiled.entry_row_ranges),
    );
    append_section(
        &mut bytes,
        &mut sections,
        SECTION_CODE_PREFIX_INDEX,
        PREFIX_INDEX_STRIDE,
        encode_prefix_index(&compiled.entry_code_prefix_index),
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
    validate_entry_row_ranges(bytes, &sections)?;
    validate_prefix_index(bytes, &sections)?;
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

pub trait PoetByteSource: fmt::Debug + Send + Sync {
    fn bytes(&self) -> &[u8];

    fn storage_label(&self) -> &'static str;

    fn mapping_mode(&self) -> &'static str;
}

#[derive(Clone, Debug)]
pub struct OwnedPoetBytes {
    bytes: Arc<[u8]>,
}

impl OwnedPoetBytes {
    #[must_use]
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }
}

impl PoetByteSource for OwnedPoetBytes {
    fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn storage_label(&self) -> &'static str {
        "byte_backed"
    }

    fn mapping_mode(&self) -> &'static str {
        "owned_bytes"
    }
}

#[derive(Debug)]
pub(super) struct ByteBackedPoetStore {
    source: Arc<dyn PoetByteSource>,
    sections: PoetBinSections,
}

#[derive(Clone, Debug)]
struct PoetBinSections {
    entries: PoetBinSectionSummary,
    entry_text_pool: PoetBinSectionSummary,
    entry_code_pool: PoetBinSectionSummary,
    entry_code_ranges: PoetBinSectionSummary,
    entry_row_ranges: PoetBinSectionSummary,
    prefix_index: PoetBinSectionSummary,
    vocabulary: VocabularySections,
    abbreviation_vocabulary: VocabularySections,
    character_codes: CharacterCodeSections,
    abbreviation_character_codes: CharacterCodeSections,
}

#[derive(Clone, Debug)]
struct VocabularySections {
    rows: PoetBinSectionSummary,
    text_pool: PoetBinSectionSummary,
    chars: PoetBinSectionSummary,
    first_codes: PoetBinSectionSummary,
    first_code_text_pool: PoetBinSectionSummary,
}

#[derive(Clone, Debug)]
struct CharacterCodeSections {
    rows: PoetBinSectionSummary,
    text_pool: PoetBinSectionSummary,
}

#[derive(Clone, Copy, Debug)]
struct VocabularyRow {
    text_start: u32,
    text_end: u32,
    chars_start: u32,
    chars_count: u32,
    weight: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct VocabularyCharsRange {
    pub(super) start: u32,
    pub(super) count: u32,
}

#[derive(Clone, Debug)]
pub(super) struct ByteBackedCodeSpan {
    pub(super) end: usize,
    pub(super) end_index: usize,
    pub(super) entries: Range<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ByteBackedPrefixState {
    hash: u64,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ByteBackedPrefixWalk {
    pub(super) spans: Vec<ByteBackedCodeSpan>,
    pub(super) prefix_hits: usize,
    pub(super) prefix_misses: usize,
    pub(super) prefix_early_breaks: usize,
    pub(super) exact_range_misses: usize,
    pub(super) nodes_visited: usize,
    pub(super) entry_ranges_emitted: usize,
}

#[derive(Clone, Copy, Debug)]
struct PrefixIndexRow {
    hash: u64,
    prefix_start: u32,
    prefix_end: u32,
    entry_start: u32,
    entry_end: u32,
}

impl ByteBackedPoetStore {
    pub(super) fn from_source(
        source: Arc<dyn PoetByteSource>,
        expected_dictionary_checksum: u32,
    ) -> Result<Self, PoetBinParseError> {
        let summary = parse_poet_bin_summary(source.bytes(), expected_dictionary_checksum)?;
        let sections = PoetBinSections::from_summary(&summary)?;
        Ok(Self { source, sections })
    }

    pub(super) fn storage_label(&self) -> &'static str {
        self.source.storage_label()
    }

    pub(super) fn mapping_mode(&self) -> &'static str {
        self.source.mapping_mode()
    }

    pub(super) fn entry_count(&self) -> usize {
        self.sections.entries.count as usize
    }

    pub(super) fn vocabulary_count(&self) -> usize {
        self.sections.vocabulary.rows.count as usize
    }

    pub(super) fn abbreviation_vocabulary_count(&self) -> usize {
        self.sections.abbreviation_vocabulary.rows.count as usize
    }

    pub(super) fn entry_code_id(&self, index: usize) -> u32 {
        read_u32(self.bytes(), self.entry_row_offset(index) + 8)
            .expect("poet entry code ids are validated during parse")
    }

    pub(super) fn entry_code(&self, index: usize) -> &str {
        let code_id = self.entry_code_id(index);
        let range_offset = row_offset(&self.sections.entry_code_ranges, code_id as usize);
        let start = read_u32(self.bytes(), range_offset)
            .expect("poet entry code ranges are validated during parse");
        let end = read_u32(self.bytes(), range_offset + 4)
            .expect("poet entry code ranges are validated during parse");
        self.text_range(&self.sections.entry_code_pool, start, end)
    }

    pub(super) fn entry_text(&self, index: usize) -> &str {
        let row = self.entry_row_offset(index);
        let start = read_u32(self.bytes(), row).expect("poet entry text is validated during parse");
        let end =
            read_u32(self.bytes(), row + 4).expect("poet entry text is validated during parse");
        self.text_range(&self.sections.entry_text_pool, start, end)
    }

    pub(super) fn entry_weight(&self, index: usize) -> f32 {
        let bits = read_u32(self.bytes(), self.entry_row_offset(index) + 12)
            .expect("poet entry weight is validated during parse");
        f32::from_bits(bits)
    }

    pub(super) fn entries_for_code_range(&self, code: &str) -> Option<Range<usize>> {
        let row = self.prefix_index_row(code.as_bytes())?;
        if row.entry_start == row.entry_end {
            return None;
        }
        Some(row.entry_start as usize..row.entry_end as usize)
    }

    pub(super) fn has_entry_code_prefix(&self, prefix: &str) -> bool {
        self.prefix_index_row(prefix.as_bytes()).is_some()
    }

    pub(super) fn root_prefix_state(&self) -> ByteBackedPrefixState {
        ByteBackedPrefixState { hash: FNV_OFFSET }
    }

    pub(super) fn advance_prefix_state(
        &self,
        state: ByteBackedPrefixState,
        appended: &[u8],
        full_prefix: &[u8],
    ) -> Option<(ByteBackedPrefixState, Range<usize>)> {
        let hash = extend_prefix_hash(state.hash, appended);
        let row = self.prefix_index_row_with_hash(full_prefix, hash)?;
        Some((
            ByteBackedPrefixState { hash },
            row.entry_start as usize..row.entry_end as usize,
        ))
    }

    pub(super) fn walk_from_prefix_index(
        &self,
        input: &str,
        boundaries: &[usize],
        start_index: usize,
    ) -> ByteBackedPrefixWalk {
        let mut walk = ByteBackedPrefixWalk::default();
        let start = boundaries[start_index];
        let mut previous_end = start;
        let mut state = self.root_prefix_state();
        for (end_index, end) in boundaries.iter().copied().enumerate().skip(start_index + 1) {
            let prefix = &input.as_bytes()[start..end];
            let appended = &input.as_bytes()[previous_end..end];
            let Some((next_state, entries)) = self.advance_prefix_state(state, appended, prefix)
            else {
                walk.prefix_misses += 1;
                walk.prefix_early_breaks += 1;
                break;
            };
            state = next_state;
            previous_end = end;
            walk.prefix_hits += 1;
            walk.nodes_visited += 1;
            if !entries.is_empty() {
                walk.entry_ranges_emitted += 1;
                walk.spans.push(ByteBackedCodeSpan {
                    end,
                    end_index,
                    entries,
                });
            } else {
                walk.exact_range_misses += 1;
            }
        }
        walk
    }

    pub(super) fn vocabulary_indices_for_first_code(
        &self,
        abbreviation: bool,
        code: &str,
    ) -> Vec<usize> {
        let sections = self.vocabulary_sections(abbreviation);
        let start = self.first_code_lower_bound(sections, code, 0, sections.first_codes.count);
        let end = self.first_code_upper_bound(sections, code, start, sections.first_codes.count);
        (start..end)
            .map(|index| self.first_code_vocabulary_index(sections, index as usize))
            .collect()
    }

    pub(super) fn vocabulary_text(&self, abbreviation: bool, index: usize) -> &str {
        let sections = self.vocabulary_sections(abbreviation);
        let row = self.vocabulary_row(sections, index);
        self.text_range(&sections.text_pool, row.text_start, row.text_end)
    }

    pub(super) fn vocabulary_weight(&self, abbreviation: bool, index: usize) -> f32 {
        self.vocabulary_row(self.vocabulary_sections(abbreviation), index)
            .weight
    }

    pub(super) fn vocabulary_chars_into(
        &self,
        abbreviation: bool,
        index: usize,
        out: &mut Vec<char>,
    ) {
        out.clear();
        let sections = self.vocabulary_sections(abbreviation);
        let row = self.vocabulary_row(sections, index);
        out.reserve(row.chars_count as usize);
        for char_index in row.chars_start..row.chars_start + row.chars_count {
            let offset = row_offset(&sections.chars, char_index as usize);
            let scalar = read_u32(self.bytes(), offset)
                .expect("poet vocabulary chars are validated during parse");
            out.push(
                char::from_u32(scalar).expect("poet vocabulary chars are validated during parse"),
            );
        }
    }

    pub(super) fn vocabulary_second_char(&self, abbreviation: bool, index: usize) -> Option<char> {
        let range = self.vocabulary_chars_range(abbreviation, index);
        (range.count >= 2).then(|| self.vocabulary_char_at(abbreviation, range, 1))
    }

    pub(super) fn vocabulary_chars_range(
        &self,
        abbreviation: bool,
        index: usize,
    ) -> VocabularyCharsRange {
        let row = self.vocabulary_row(self.vocabulary_sections(abbreviation), index);
        VocabularyCharsRange {
            start: row.chars_start,
            count: row.chars_count,
        }
    }

    pub(super) fn vocabulary_char_at(
        &self,
        abbreviation: bool,
        range: VocabularyCharsRange,
        index: usize,
    ) -> char {
        debug_assert!(index < range.count as usize);
        let sections = self.vocabulary_sections(abbreviation);
        let offset = row_offset(&sections.chars, range.start as usize + index);
        let scalar = read_u32(self.bytes(), offset)
            .expect("poet vocabulary chars are validated during parse");
        char::from_u32(scalar).expect("poet vocabulary chars are validated during parse")
    }

    pub(super) fn character_codes(&self, abbreviation: bool, ch: char) -> Vec<&str> {
        let sections = if abbreviation {
            &self.sections.abbreviation_character_codes
        } else {
            &self.sections.character_codes
        };
        let Some(row_index) = self.character_code_row_index(sections, ch) else {
            return Vec::new();
        };
        let row = row_offset(&sections.rows, row_index);
        let start = read_u32(self.bytes(), row + 4)
            .expect("poet character code ranges are validated during parse");
        let end = read_u32(self.bytes(), row + 8)
            .expect("poet character code ranges are validated during parse");
        self.len_strings(&sections.text_pool, start, end)
    }

    pub(super) fn has_normal_character_code(&self, code: &str) -> bool {
        let sections = &self.sections.character_codes;
        for row_index in 0..sections.rows.count as usize {
            let row = row_offset(&sections.rows, row_index);
            let start = read_u32(self.bytes(), row + 4)
                .expect("poet character code ranges are validated during parse");
            let end = read_u32(self.bytes(), row + 8)
                .expect("poet character code ranges are validated during parse");
            if self.len_string_range_contains(&sections.text_pool, start, end, code.as_bytes()) {
                return true;
            }
        }
        false
    }

    pub(super) fn memory_owner_rows(&self) -> Vec<crate::MemoryOwnerRow> {
        vec![
            crate::MemoryOwnerRow::new(
                "poet.entries_by_code",
                poet_byte_source_class(self.source.as_ref()),
                self.entries_payload_bytes(),
                self.entry_count(),
                format!("poet_bin:{}:{}", self.storage_label(), self.mapping_mode()),
                "sentence model entries served from YUNE-POET/3 bytes",
            ),
            crate::MemoryOwnerRow::new(
                "poet.prefix_index",
                poet_byte_source_class(self.source.as_ref()),
                section_len(&self.sections.entry_row_ranges)
                    .saturating_add(section_len(&self.sections.prefix_index)),
                self.sections.prefix_index.count as usize,
                format!("poet_bin:{}:{}", self.storage_label(), self.mapping_mode()),
                "compiled prefix and exact-row ranges served from YUNE-POET/3 bytes",
            ),
            crate::MemoryOwnerRow::new(
                "poet.vocabulary",
                poet_byte_source_class(self.source.as_ref()),
                self.vocabulary_payload_bytes(false),
                self.vocabulary_count(),
                format!("poet_bin:{}:{}", self.storage_label(), self.mapping_mode()),
                "normal preset vocabulary served from YUNE-POET/3 bytes",
            ),
            crate::MemoryOwnerRow::new(
                "poet.abbreviation_vocabulary",
                poet_byte_source_class(self.source.as_ref()),
                self.vocabulary_payload_bytes(true),
                self.abbreviation_vocabulary_count(),
                format!("poet_bin:{}:{}", self.storage_label(), self.mapping_mode()),
                "abbreviation preset vocabulary served from YUNE-POET/3 bytes",
            ),
        ]
    }

    fn entries_payload_bytes(&self) -> usize {
        section_len(&self.sections.entries)
            .saturating_add(section_len(&self.sections.entry_text_pool))
            .saturating_add(section_len(&self.sections.entry_code_pool))
            .saturating_add(section_len(&self.sections.entry_code_ranges))
            .saturating_add(section_len(&self.sections.character_codes.rows))
            .saturating_add(section_len(&self.sections.character_codes.text_pool))
            .saturating_add(section_len(
                &self.sections.abbreviation_character_codes.rows,
            ))
            .saturating_add(section_len(
                &self.sections.abbreviation_character_codes.text_pool,
            ))
    }

    fn vocabulary_payload_bytes(&self, abbreviation: bool) -> usize {
        let sections = self.vocabulary_sections(abbreviation);
        section_len(&sections.rows)
            .saturating_add(section_len(&sections.text_pool))
            .saturating_add(section_len(&sections.chars))
            .saturating_add(section_len(&sections.first_codes))
            .saturating_add(section_len(&sections.first_code_text_pool))
    }

    fn bytes(&self) -> &[u8] {
        self.source.bytes()
    }

    fn entry_row_offset(&self, index: usize) -> usize {
        row_offset(&self.sections.entries, index)
    }

    fn vocabulary_sections(&self, abbreviation: bool) -> &VocabularySections {
        if abbreviation {
            &self.sections.abbreviation_vocabulary
        } else {
            &self.sections.vocabulary
        }
    }

    fn vocabulary_row(&self, sections: &VocabularySections, index: usize) -> VocabularyRow {
        let row = row_offset(&sections.rows, index);
        VocabularyRow {
            text_start: read_u32(self.bytes(), row)
                .expect("poet vocabulary text is validated during parse"),
            text_end: read_u32(self.bytes(), row + 4)
                .expect("poet vocabulary text is validated during parse"),
            chars_start: read_u32(self.bytes(), row + 8)
                .expect("poet vocabulary chars are validated during parse"),
            chars_count: read_u32(self.bytes(), row + 12)
                .expect("poet vocabulary chars are validated during parse"),
            weight: f32::from_bits(
                read_u32(self.bytes(), row + 16)
                    .expect("poet vocabulary weight is validated during parse"),
            ),
        }
    }

    fn first_code_vocabulary_index(&self, sections: &VocabularySections, index: usize) -> usize {
        let row = row_offset(&sections.first_codes, index);
        read_u32(self.bytes(), row + 8)
            .expect("poet vocabulary first-code index is validated during parse") as usize
    }

    fn first_code_lower_bound(
        &self,
        sections: &VocabularySections,
        value: &str,
        start: u32,
        end: u32,
    ) -> u32 {
        let mut low = start;
        let mut high = end;
        while low < high {
            let mid = low + (high - low) / 2;
            if self.first_code(sections, mid as usize) < value {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    fn first_code_upper_bound(
        &self,
        sections: &VocabularySections,
        value: &str,
        start: u32,
        end: u32,
    ) -> u32 {
        let mut low = start;
        let mut high = end;
        while low < high {
            let mid = low + (high - low) / 2;
            if self.first_code(sections, mid as usize) <= value {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low
    }

    fn first_code(&self, sections: &VocabularySections, index: usize) -> &str {
        let row = row_offset(&sections.first_codes, index);
        let start = read_u32(self.bytes(), row)
            .expect("poet vocabulary first codes are validated during parse");
        let end = read_u32(self.bytes(), row + 4)
            .expect("poet vocabulary first codes are validated during parse");
        self.text_range(&sections.first_code_text_pool, start, end)
    }

    fn character_code_row_index(
        &self,
        sections: &CharacterCodeSections,
        ch: char,
    ) -> Option<usize> {
        let target = ch as u32;
        let mut low = 0usize;
        let mut high = sections.rows.count as usize;
        while low < high {
            let mid = low + (high - low) / 2;
            let row = row_offset(&sections.rows, mid);
            let scalar = read_u32(self.bytes(), row)
                .expect("poet character codes are validated during parse");
            match scalar.cmp(&target) {
                std::cmp::Ordering::Less => low = mid + 1,
                std::cmp::Ordering::Equal => return Some(mid),
                std::cmp::Ordering::Greater => high = mid,
            }
        }
        None
    }

    fn prefix_index_row(&self, prefix: &[u8]) -> Option<PrefixIndexRow> {
        let hash = hash_prefix_key(prefix);
        self.prefix_index_row_with_hash(prefix, hash)
    }

    fn prefix_index_row_with_hash(&self, prefix: &[u8], hash: u64) -> Option<PrefixIndexRow> {
        let mut low = 0usize;
        let mut high = self.sections.prefix_index.count as usize;
        while low < high {
            let mid = low + (high - low) / 2;
            let row = self.prefix_index_row_at(mid);
            match row.hash.cmp(&hash) {
                Ordering::Less => low = mid + 1,
                Ordering::Equal => high = mid,
                Ordering::Greater => high = mid,
            }
        }
        let mut index = low;
        while index < self.sections.prefix_index.count as usize {
            let row = self.prefix_index_row_at(index);
            if row.hash != hash {
                break;
            }
            if self.prefix_index_key(row) == prefix {
                return Some(row);
            }
            index += 1;
        }
        None
    }

    fn prefix_index_row_at(&self, index: usize) -> PrefixIndexRow {
        let row = row_offset(&self.sections.prefix_index, index);
        PrefixIndexRow {
            hash: read_u64(self.bytes(), row).expect("poet prefix index is validated during parse"),
            prefix_start: read_u32(self.bytes(), row + 8)
                .expect("poet prefix index is validated during parse"),
            prefix_end: read_u32(self.bytes(), row + 12)
                .expect("poet prefix index is validated during parse"),
            entry_start: read_u32(self.bytes(), row + 24)
                .expect("poet prefix index is validated during parse"),
            entry_end: read_u32(self.bytes(), row + 28)
                .expect("poet prefix index is validated during parse"),
        }
    }

    fn prefix_index_key(&self, row: PrefixIndexRow) -> &[u8] {
        let pool = section_bytes(self.bytes(), &self.sections.entry_code_pool)
            .expect("poet code pool is validated during parse");
        &pool[row.prefix_start as usize..row.prefix_end as usize]
    }

    fn text_range(&self, section: &PoetBinSectionSummary, start: u32, end: u32) -> &str {
        let pool = section_bytes(self.bytes(), section)
            .expect("poet text pools are validated during parse");
        str::from_utf8(&pool[start as usize..end as usize])
            .expect("poet text ranges are validated during parse")
    }

    fn len_strings(&self, section: &PoetBinSectionSummary, start: u32, end: u32) -> Vec<&str> {
        let pool = section_bytes(self.bytes(), section)
            .expect("poet len-string pools are validated during parse");
        let mut cursor = start as usize;
        let end = end as usize;
        let mut strings = Vec::new();
        while cursor < end {
            let len = read_u32(pool, cursor)
                .expect("poet len-string lengths are validated during parse")
                as usize;
            cursor += 4;
            let string_end = cursor + len;
            strings.push(
                str::from_utf8(&pool[cursor..string_end])
                    .expect("poet len strings are validated during parse"),
            );
            cursor = string_end;
        }
        strings
    }

    fn len_string_range_contains(
        &self,
        section: &PoetBinSectionSummary,
        start: u32,
        end: u32,
        needle: &[u8],
    ) -> bool {
        let pool = section_bytes(self.bytes(), section)
            .expect("poet len-string pools are validated during parse");
        let mut cursor = start as usize;
        let end = end as usize;
        while cursor < end {
            let len = read_u32(pool, cursor)
                .expect("poet len-string lengths are validated during parse")
                as usize;
            cursor += 4;
            let string_end = cursor + len;
            if &pool[cursor..string_end] == needle {
                return true;
            }
            cursor = string_end;
        }
        false
    }
}

impl PoetBinSections {
    fn from_summary(summary: &PoetBinSummary) -> Result<Self, PoetBinParseError> {
        let sections = &summary.sections;
        Ok(Self {
            entries: required_section(sections, SECTION_ENTRIES)?.clone(),
            entry_text_pool: required_section(sections, SECTION_ENTRY_TEXT_POOL)?.clone(),
            entry_code_pool: required_section(sections, SECTION_ENTRY_CODE_POOL)?.clone(),
            entry_code_ranges: required_section(sections, SECTION_ENTRY_CODE_RANGES)?.clone(),
            entry_row_ranges: required_section(sections, SECTION_ENTRY_ROW_RANGES)?.clone(),
            prefix_index: required_section(sections, SECTION_CODE_PREFIX_INDEX)?.clone(),
            vocabulary: VocabularySections {
                rows: required_section(sections, SECTION_VOCABULARY)?.clone(),
                text_pool: required_section(sections, SECTION_VOCABULARY_TEXT_POOL)?.clone(),
                chars: required_section(sections, SECTION_VOCABULARY_CHARS)?.clone(),
                first_codes: required_section(sections, SECTION_VOCABULARY_FIRST_CODES)?.clone(),
                first_code_text_pool: required_section(
                    sections,
                    SECTION_VOCABULARY_FIRST_CODE_TEXT_POOL,
                )?
                .clone(),
            },
            abbreviation_vocabulary: VocabularySections {
                rows: required_section(sections, SECTION_ABBREVIATION_VOCABULARY)?.clone(),
                text_pool: required_section(sections, SECTION_ABBREVIATION_VOCABULARY_TEXT_POOL)?
                    .clone(),
                chars: required_section(sections, SECTION_ABBREVIATION_VOCABULARY_CHARS)?.clone(),
                first_codes: required_section(
                    sections,
                    SECTION_ABBREVIATION_VOCABULARY_FIRST_CODES,
                )?
                .clone(),
                first_code_text_pool: required_section(
                    sections,
                    SECTION_ABBREVIATION_FIRST_CODE_TEXT_POOL,
                )?
                .clone(),
            },
            character_codes: CharacterCodeSections {
                rows: required_section(sections, SECTION_CHARACTER_CODES)?.clone(),
                text_pool: required_section(sections, SECTION_CHARACTER_CODE_TEXT_POOL)?.clone(),
            },
            abbreviation_character_codes: CharacterCodeSections {
                rows: required_section(sections, SECTION_ABBREVIATION_CHARACTER_CODES)?.clone(),
                text_pool: required_section(
                    sections,
                    SECTION_ABBREVIATION_CHARACTER_CODE_TEXT_POOL,
                )?
                .clone(),
            },
        })
    }
}

fn section_len(section: &PoetBinSectionSummary) -> usize {
    section.len as usize
}

fn poet_byte_source_class(source: &dyn PoetByteSource) -> crate::MemoryOwnerClass {
    if source.mapping_mode() == "mmap" {
        crate::MemoryOwnerClass::MmapFileBacked
    } else {
        crate::MemoryOwnerClass::HeapOwnedGuarded
    }
}

struct CompiledPoetInputs {
    entries_by_code: Vec<ModelEntry>,
    entry_texts: ModelStringPool,
    entry_codes: ModelStringPool,
    entry_row_ranges: Vec<(u32, u32)>,
    entry_code_prefix_index: Vec<PrefixIndexBuildRow>,
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
    // YUNE-POET/3 is compiled from source/Yune-owned raw weights. Unlike a
    // librime `.table.bin` reconstructed into an owned runtime model, this
    // byte-backed sidecar must therefore remain in the Raw weight domain.
    let entries = entries
        .into_iter()
        .map(|entry| OwnedModelEntry {
            text: entry.text,
            code: entry.code,
            weight: entry.weight,
        })
        .collect::<Vec<_>>();
    let script_vocabulary =
        script_encoder_phrase_vocabulary(&entries, vocabulary, EntryWeightDomain::Raw);
    let mut owned_entries = Vec::new();
    let mut script_encoder_codes = Vec::new();
    let mut abbreviation_character_codes: HashMap<char, Vec<String>> = HashMap::new();
    for owned in entries {
        if owned.code.is_empty() {
            continue;
        }
        let mut chars = owned.text.chars();
        if let Some(ch) = chars.next() {
            if chars.next().is_none() {
                script_encoder_codes.push((ch, owned.code.clone(), owned.weight));
                if EntryWeightDomain::Raw.has_positive_raw_weight(owned.weight) {
                    abbreviation_character_codes
                        .entry(ch)
                        .or_default()
                        .push(owned.code.clone());
                }
            }
        }
        owned_entries.push(owned);
    }
    let character_codes =
        build_script_encoder_character_codes(script_encoder_codes, EntryWeightDomain::Raw);
    for codes in abbreviation_character_codes.values_mut() {
        codes.sort();
        codes.dedup();
    }
    owned_entries.sort_by(compare_model_entry_by_code);
    let (entries_by_code, entry_texts, entry_codes) = pack_owned_model_entries(owned_entries);
    let entry_row_ranges = build_entry_row_ranges(&entries_by_code, entry_codes.ranges.len());
    let entry_code_prefix_index = build_prefix_index(&entry_codes, &entry_row_ranges);
    let (vocabulary, vocabulary_first_codes) =
        build_model_vocabulary_index(&script_vocabulary, &character_codes);
    let (abbreviation_vocabulary, abbreviation_vocabulary_first_codes) =
        build_model_vocabulary_index(abbreviation_vocabulary, &abbreviation_character_codes);
    CompiledPoetInputs {
        entries_by_code,
        entry_texts,
        entry_codes,
        entry_row_ranges,
        entry_code_prefix_index,
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

fn encode_entry_row_ranges(ranges: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(ranges.len() * RANGE_STRIDE as usize);
    for (start, end) in ranges {
        put_u32_extend(&mut bytes, *start);
        put_u32_extend(&mut bytes, *end);
    }
    bytes
}

#[derive(Clone, Debug)]
struct PrefixIndexBuildRow {
    hash: u64,
    prefix_start: u32,
    prefix_end: u32,
    code_start: u32,
    code_end: u32,
    entry_start: u32,
    entry_end: u32,
}

#[derive(Clone, Debug)]
struct PrefixIndexAccumulator {
    prefix_start: u32,
    prefix_end: u32,
    code_start: u32,
    code_end: u32,
    entry_start: u32,
    entry_end: u32,
}

fn build_entry_row_ranges(entries: &[ModelEntry], code_count: usize) -> Vec<(u32, u32)> {
    let mut ranges = vec![(0, 0); code_count];
    let mut start = 0usize;
    while start < entries.len() {
        let code_id = entries[start].code_id as usize;
        let mut end = start + 1;
        while end < entries.len() && entries[end].code_id as usize == code_id {
            end += 1;
        }
        ranges[code_id] = (
            u32::try_from(start).expect("poet entry row start exceeds u32"),
            u32::try_from(end).expect("poet entry row end exceeds u32"),
        );
        start = end;
    }
    ranges
}

fn build_prefix_index(
    entry_codes: &ModelStringPool,
    entry_row_ranges: &[(u32, u32)],
) -> Vec<PrefixIndexBuildRow> {
    let mut prefixes = BTreeMap::<String, PrefixIndexAccumulator>::new();
    for (code_id, range) in entry_codes.ranges.iter().copied().enumerate() {
        let code = entry_codes.string(range);
        for prefix_end in code
            .char_indices()
            .map(|(offset, ch)| offset + ch.len_utf8())
        {
            let prefix = &code[..prefix_end];
            let absolute_prefix_end = range.start + prefix_end as u32;
            let (entry_start, entry_end) = if prefix_end == code.len() {
                entry_row_ranges[code_id]
            } else {
                (0, 0)
            };
            prefixes
                .entry(prefix.to_owned())
                .and_modify(|existing| {
                    existing.code_start = existing.code_start.min(code_id as u32);
                    existing.code_end = existing.code_end.max(code_id as u32 + 1);
                    if entry_start != entry_end {
                        existing.entry_start = entry_start;
                        existing.entry_end = entry_end;
                    }
                })
                .or_insert(PrefixIndexAccumulator {
                    prefix_start: range.start,
                    prefix_end: absolute_prefix_end,
                    code_start: code_id as u32,
                    code_end: code_id as u32 + 1,
                    entry_start,
                    entry_end,
                });
        }
    }
    let mut rows = prefixes
        .into_values()
        .map(|row| PrefixIndexBuildRow {
            hash: hash_prefix_key(
                entry_codes
                    .string(ModelStringRange {
                        start: row.prefix_start,
                        end: row.prefix_end,
                    })
                    .as_bytes(),
            ),
            prefix_start: row.prefix_start,
            prefix_end: row.prefix_end,
            code_start: row.code_start,
            code_end: row.code_end,
            entry_start: row.entry_start,
            entry_end: row.entry_end,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.hash.cmp(&right.hash).then_with(|| {
            let left_key = entry_codes
                .string(ModelStringRange {
                    start: left.prefix_start,
                    end: left.prefix_end,
                })
                .as_bytes();
            let right_key = entry_codes
                .string(ModelStringRange {
                    start: right.prefix_start,
                    end: right.prefix_end,
                })
                .as_bytes();
            left_key.cmp(right_key)
        })
    });
    rows
}

fn encode_prefix_index(rows: &[PrefixIndexBuildRow]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(rows.len() * PREFIX_INDEX_STRIDE as usize);
    for row in rows {
        put_u64_extend(&mut bytes, row.hash);
        put_u32_extend(&mut bytes, row.prefix_start);
        put_u32_extend(&mut bytes, row.prefix_end);
        put_u32_extend(&mut bytes, row.code_start);
        put_u32_extend(&mut bytes, row.code_end);
        put_u32_extend(&mut bytes, row.entry_start);
        put_u32_extend(&mut bytes, row.entry_end);
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

fn validate_entry_row_ranges(
    bytes: &[u8],
    sections: &[PoetBinSectionSummary],
) -> Result<(), PoetBinParseError> {
    let entries = required_section(sections, SECTION_ENTRIES)?;
    let code_ranges = required_section(sections, SECTION_ENTRY_CODE_RANGES)?;
    let row_ranges = required_section(sections, SECTION_ENTRY_ROW_RANGES)?;
    require_stride(row_ranges, RANGE_STRIDE)?;
    if row_ranges.count != code_ranges.count {
        return Err(PoetBinParseError::InvalidSectionShape(
            SECTION_ENTRY_ROW_RANGES,
        ));
    }
    let mut previous_end = 0u32;
    for index in 0..row_ranges.count as usize {
        let row = row_offset(row_ranges, index);
        let start = read_u32(bytes, row).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_ENTRY_ROW_RANGES,
        ))?;
        let end = read_u32(bytes, row + 4).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_ENTRY_ROW_RANGES,
        ))?;
        if start > end || end > entries.count || start < previous_end {
            return Err(PoetBinParseError::InvalidSectionShape(
                SECTION_ENTRY_ROW_RANGES,
            ));
        }
        previous_end = end;
    }
    Ok(())
}

fn validate_prefix_index(
    bytes: &[u8],
    sections: &[PoetBinSectionSummary],
) -> Result<(), PoetBinParseError> {
    let prefix_index = required_section(sections, SECTION_CODE_PREFIX_INDEX)?;
    require_stride(prefix_index, PREFIX_INDEX_STRIDE)?;
    let code_pool = section_bytes(bytes, required_section(sections, SECTION_ENTRY_CODE_POOL)?)?;
    let code_ranges = required_section(sections, SECTION_ENTRY_CODE_RANGES)?;
    let entries = required_section(sections, SECTION_ENTRIES)?;
    let mut previous_key: Option<&[u8]> = None;
    for index in 0..prefix_index.count as usize {
        let row = row_offset(prefix_index, index);
        let hash = read_u64(bytes, row).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_CODE_PREFIX_INDEX,
        ))?;
        let prefix_start = read_u32(bytes, row + 8).ok_or(
            PoetBinParseError::SectionOutOfBounds(SECTION_CODE_PREFIX_INDEX),
        )?;
        let prefix_end = read_u32(bytes, row + 12).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_CODE_PREFIX_INDEX,
        ))?;
        let code_start = read_u32(bytes, row + 16).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_CODE_PREFIX_INDEX,
        ))?;
        let code_end = read_u32(bytes, row + 20).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_CODE_PREFIX_INDEX,
        ))?;
        let entry_start = read_u32(bytes, row + 24).ok_or(
            PoetBinParseError::SectionOutOfBounds(SECTION_CODE_PREFIX_INDEX),
        )?;
        let entry_end = read_u32(bytes, row + 28).ok_or(PoetBinParseError::SectionOutOfBounds(
            SECTION_CODE_PREFIX_INDEX,
        ))?;
        validate_text_range(
            code_pool,
            prefix_start,
            prefix_end,
            SECTION_CODE_PREFIX_INDEX,
        )?;
        if prefix_start == prefix_end
            || code_start >= code_end
            || code_end > code_ranges.count
            || entry_start > entry_end
            || entry_end > entries.count
        {
            return Err(PoetBinParseError::InvalidSectionShape(
                SECTION_CODE_PREFIX_INDEX,
            ));
        }
        let key = &code_pool[prefix_start as usize..prefix_end as usize];
        if hash != hash_prefix_key(key) {
            return Err(PoetBinParseError::InvalidSectionShape(
                SECTION_CODE_PREFIX_INDEX,
            ));
        }
        if previous_key.is_some_and(|previous| {
            let previous_hash = hash_prefix_key(previous);
            previous_hash > hash || (previous_hash == hash && previous >= key)
        }) {
            return Err(PoetBinParseError::InvalidSectionShape(
                SECTION_CODE_PREFIX_INDEX,
            ));
        }
        previous_key = Some(key);
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

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let raw = bytes.get(offset..end)?;
    Some(u64::from_le_bytes(raw.try_into().ok()?))
}

fn put_u64_extend(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn hash_prefix_key(bytes: &[u8]) -> u64 {
    extend_prefix_hash(FNV_OFFSET, bytes)
}

fn extend_prefix_hash(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
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

        assert_eq!(&bytes[..12], b"YUNE-POET/3\0");
        assert_eq!(summary.dictionary_checksum, 0xAABBCCDD);
        assert_eq!(summary.entries, 3);
        assert_eq!(summary.vocabulary_entries, 0);
        assert_eq!(summary.abbreviation_vocabulary_entries, 1);
        assert!(summary.sections.iter().any(|section| section.id == 19));
        assert!(summary.sections.iter().any(|section| section.id == 20));
        assert_eq!(
            summary
                .sections
                .iter()
                .find(|section| section.id == 20)
                .map(|section| section.stride),
            Some(32)
        );
        assert_eq!(summary.sections.len(), 20);
    }

    #[test]
    fn byte_backed_prefix_advance_verifies_full_key_bytes_after_hashing() {
        let storage = ByteBackedPoetStore::from_source(
            Arc::new(OwnedPoetBytes::new(sample_poet_bin())) as Arc<dyn PoetByteSource>,
            0xAABBCCDD,
        )
        .expect("poet bin should parse");
        let root = storage.root_prefix_state();

        assert!(storage.advance_prefix_state(root, b"ni", b"ni").is_some());
        assert!(
            storage.advance_prefix_state(root, b"ni", b"hao").is_none(),
            "a caller-supplied rolling hash match must not bypass full-key comparison"
        );
    }

    #[test]
    fn poet_bin_rejects_legacy_artifacts() {
        for magic in [b"YUNE-POET/1\0", b"YUNE-POET/2\0"] {
            let mut bytes = sample_poet_bin();
            bytes[..12].copy_from_slice(magic);

            assert_eq!(
                parse_poet_bin_summary(&bytes, 0xAABBCCDD),
                Err(PoetBinParseError::UnsupportedVersion),
            );
        }
    }

    #[test]
    fn poet_bin_rejects_legacy_v2_artifact_without_script_encoder_filtering() {
        let mut bytes = sample_poet_bin();
        bytes[..12].copy_from_slice(b"YUNE-POET/2\0");

        assert_eq!(
            parse_poet_bin_summary(&bytes, 0xAABBCCDD),
            Err(PoetBinParseError::UnsupportedVersion),
        );
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
