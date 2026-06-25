use super::{
    DictionaryLookupRecord, RimeCorrectionEntry, RimeToleranceRule, TableDictionary,
    TableDictionaryAdvancedData, TableEncoder, TableEntry,
};
use crate::dictionary::compiled::{
    parse_rime_format_version_for_payload, read_f32_le, read_i32_le, read_u32_le,
};
use crate::dictionary::query_table::{LookupCandidate, LookupCandidateEntry, TableLookup};
use crate::CandidateSource;
use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;

const MAX_CORRECTION_COUNT: usize = 4096;
const MAX_TOLERANCE_RULE_COUNT: usize = 4096;
const MAX_TOLERANCE_CANDIDATE_COUNT: usize = 64;
const MAX_LOOKUP_TEXT_COUNT: usize = 1_000_000;
const MAX_LOOKUP_RECORD_COUNT: usize = 1_000_000;
const MAX_LOOKUP_FIELD_COUNT: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RimeTableBinParseError {
    TooShort,
    InvalidFormat,
    UnsupportedVersion,
    MissingRequiredSection,
    OutOfBounds,
    InvalidLength,
    InvalidCount,
    InvalidUtf8,
    UnsupportedSection { role: String },
}

pub trait CompactTableByteSource: fmt::Debug + Send + Sync {
    fn bytes(&self) -> &[u8];

    fn storage_label(&self) -> &'static str;

    fn mapping_mode(&self) -> &'static str;
}

#[derive(Clone, Debug)]
pub(crate) struct OwnedCompactTableBytes {
    bytes: Arc<[u8]>,
}

impl OwnedCompactTableBytes {
    fn new(bytes: impl Into<Arc<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
        }
    }
}

impl CompactTableByteSource for OwnedCompactTableBytes {
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

#[derive(Clone, Debug)]
pub struct CompactTableStore {
    syllabary_codes: Vec<String>,
    storage: CompactTableStorage,
    advanced: TableDictionaryAdvancedData,
}

#[derive(Clone, Debug)]
enum CompactTableStorage {
    Owned {
        code_groups: Vec<CompactCodeGroup>,
        entries: Vec<CompactTableEntry>,
    },
    ByteBacked {
        source: Arc<dyn CompactTableByteSource>,
        code_groups: Vec<ByteBackedCodeGroup>,
        entries: Vec<ByteBackedTableEntry>,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct CompactCodeGroup {
    code: String,
    entries: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct CompactTableEntry {
    text: String,
    weight: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ByteStringRef {
    offset: usize,
    len: usize,
}

impl ByteStringRef {
    fn as_str(self, bytes: &[u8]) -> &str {
        std::str::from_utf8(&bytes[self.offset..self.offset + self.len])
            .expect("compiled table string refs are validated during parse")
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ByteBackedCodeGroup {
    code: ByteStringRef,
    entries: Range<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct ByteBackedTableEntry {
    text: ByteStringRef,
    weight: f32,
}

impl CompactTableStore {
    #[must_use]
    pub(crate) fn from_dictionary(dictionary: TableDictionary) -> Self {
        let advanced = dictionary.advanced_data();
        Self::from_entries_and_advanced(dictionary.entries, advanced)
    }

    fn from_entries_and_advanced(
        entries: Vec<TableEntry>,
        advanced: TableDictionaryAdvancedData,
    ) -> Self {
        let mut syllabary_codes = Vec::<String>::new();
        for entry in &entries {
            if !syllabary_codes.iter().any(|code| code == &entry.code) {
                syllabary_codes.push(entry.code.clone());
            }
        }

        let mut grouped = entries.into_iter().fold(
            Vec::<(String, Vec<CompactTableEntry>)>::new(),
            |mut groups, entry| {
                if let Some((_, group_entries)) =
                    groups.iter_mut().find(|(code, _)| code == &entry.code)
                {
                    group_entries.push(CompactTableEntry {
                        text: entry.text,
                        weight: entry.weight,
                    });
                } else {
                    groups.push((
                        entry.code,
                        vec![CompactTableEntry {
                            text: entry.text,
                            weight: entry.weight,
                        }],
                    ));
                }
                groups
            },
        );
        grouped.sort_by(|left, right| left.0.cmp(&right.0));

        let mut code_groups = Vec::with_capacity(grouped.len());
        let mut compact_entries = Vec::new();
        for (code, entries) in grouped {
            let start = compact_entries.len();
            compact_entries.extend(entries);
            let end = compact_entries.len();
            code_groups.push(CompactCodeGroup {
                code,
                entries: start..end,
            });
        }

        Self {
            syllabary_codes,
            storage: CompactTableStorage::Owned {
                code_groups,
                entries: compact_entries,
            },
            advanced,
        }
    }

    pub fn from_table_bin_bytes(
        bytes: impl Into<Arc<[u8]>>,
        advanced: TableDictionaryAdvancedData,
    ) -> Result<Self, RimeTableBinParseError> {
        Self::from_table_bin_byte_source(Arc::new(OwnedCompactTableBytes::new(bytes)), advanced)
    }

    pub fn from_table_bin_byte_source(
        source: Arc<dyn CompactTableByteSource>,
        advanced: TableDictionaryAdvancedData,
    ) -> Result<Self, RimeTableBinParseError> {
        let bytes = source.bytes();
        ensure_len(bytes, 68)?;
        let version = parse_rime_format_version_for_payload(bytes, b"Rime::Table/")
            .map_err(map_metadata_error)?;
        if version < 4.0 - f64::EPSILON {
            return Err(RimeTableBinParseError::UnsupportedVersion);
        }

        let syllabary_offset =
            read_offset_ptr(bytes, 44)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
        let index_offset =
            read_offset_ptr(bytes, 48)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
        let string_table_offset = read_offset_ptr(bytes, 60)?;
        let string_table_size = read_u32_le(bytes, 64).map_err(map_metadata_error)?;
        if string_table_offset.is_some() || string_table_size != 0 {
            return Err(RimeTableBinParseError::UnsupportedSection {
                role: "marisa string_table".to_owned(),
            });
        }

        let syllable_refs = read_syllabary_refs(bytes, syllabary_offset)?;
        let syllabary_codes = syllable_refs
            .iter()
            .map(|reference| reference.as_str(bytes).to_owned())
            .collect::<Vec<_>>();
        let (code_groups, entries) =
            read_byte_backed_head_index_entries(bytes, index_offset, &syllable_refs)?;

        Ok(Self {
            syllabary_codes,
            storage: CompactTableStorage::ByteBacked {
                source,
                code_groups,
                entries,
            },
            advanced,
        })
    }

    #[must_use]
    pub(crate) fn syllabary_codes(&self) -> &[String] {
        &self.syllabary_codes
    }

    #[must_use]
    pub fn advanced_data(&self) -> TableDictionaryAdvancedData {
        self.advanced.clone()
    }

    #[must_use]
    pub fn to_table_dictionary(&self) -> TableDictionary {
        let entries = self
            .all_codes()
            .flat_map(|code| {
                self.exact_candidates(code).map(move |candidate| {
                    TableEntry::new(code, candidate.text(), candidate.raw_quality())
                })
            })
            .collect::<Vec<_>>();
        TableDictionary::with_advanced_data(entries, self.advanced.clone())
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn corrections(&self) -> &[RimeCorrectionEntry] {
        &self.advanced.corrections
    }

    #[must_use]
    #[cfg(test)]
    pub(crate) fn tolerance_rules(&self) -> &[RimeToleranceRule] {
        &self.advanced.tolerance_rules
    }

    fn group_index(&self, code: &str) -> Result<usize, usize> {
        match &self.storage {
            CompactTableStorage::Owned { code_groups, .. } => {
                code_groups.binary_search_by(|group| group.code.as_str().cmp(code))
            }
            CompactTableStorage::ByteBacked {
                source,
                code_groups,
                ..
            } => {
                let bytes = source.bytes();
                code_groups.binary_search_by(|group| group.code.as_str(bytes).cmp(code))
            }
        }
    }

    fn exact_entries(&self, code: &str) -> Option<(&str, &[CompactTableEntry])> {
        let CompactTableStorage::Owned {
            code_groups,
            entries,
        } = &self.storage
        else {
            return None;
        };
        let index = self.group_index(code).ok()?;
        let group = &code_groups[index];
        Some((&group.code, &entries[group.entries.clone()]))
    }

    fn byte_backed_exact_entries(
        &self,
        code: &str,
    ) -> Option<(
        &dyn CompactTableByteSource,
        &ByteBackedCodeGroup,
        &[ByteBackedTableEntry],
    )> {
        let CompactTableStorage::ByteBacked {
            source,
            code_groups,
            entries,
        } = &self.storage
        else {
            return None;
        };
        let index = self.group_index(code).ok()?;
        let group = &code_groups[index];
        Some((source.as_ref(), group, &entries[group.entries.clone()]))
    }

    #[must_use]
    pub fn storage_label(&self) -> &'static str {
        match &self.storage {
            CompactTableStorage::Owned { .. } => "owned_heap",
            CompactTableStorage::ByteBacked { source, .. } => source.storage_label(),
        }
    }

    #[must_use]
    pub fn mapping_mode(&self) -> &'static str {
        match &self.storage {
            CompactTableStorage::Owned { .. } => "owned_heap",
            CompactTableStorage::ByteBacked { source, .. } => source.mapping_mode(),
        }
    }

    #[must_use]
    pub fn stored_entry_count(&self) -> usize {
        match &self.storage {
            CompactTableStorage::Owned { entries, .. } => entries.len(),
            CompactTableStorage::ByteBacked { entries, .. } => entries.len(),
        }
    }

    #[must_use]
    pub fn byte_source_len(&self) -> usize {
        match &self.storage {
            CompactTableStorage::Owned { .. } => 0,
            CompactTableStorage::ByteBacked { source, .. } => source.bytes().len(),
        }
    }
}

#[cfg(test)]
pub(crate) fn parse_compact_table_bin_lookup(
    bytes: impl AsRef<[u8]>,
) -> Result<CompactTableStore, RimeTableBinParseError> {
    let bytes = Arc::<[u8]>::from(bytes.as_ref());
    let advanced = parse_rime_table_bin_advanced_data(bytes.as_ref())?;
    CompactTableStore::from_table_bin_bytes(bytes, advanced)
}

pub(crate) struct CompactExactCandidates<'a> {
    inner: CompactExactCandidatesInner<'a>,
}

enum CompactExactCandidatesInner<'a> {
    Empty,
    Owned {
        code: &'a str,
        inner: std::slice::Iter<'a, CompactTableEntry>,
    },
    ByteBacked {
        bytes: &'a [u8],
        code: ByteStringRef,
        inner: std::slice::Iter<'a, ByteBackedTableEntry>,
    },
}

impl<'a> Iterator for CompactExactCandidates<'a> {
    type Item = LookupCandidate<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            CompactExactCandidatesInner::Empty => None,
            CompactExactCandidatesInner::Owned { code, inner } => inner.next().map(|entry| {
                LookupCandidate::new(&entry.text, code, entry.weight, CandidateSource::Table)
            }),
            CompactExactCandidatesInner::ByteBacked { bytes, code, inner } => {
                let code = code.as_str(bytes);
                inner.next().map(|entry| {
                    LookupCandidate::new(
                        entry.text.as_str(bytes),
                        code,
                        entry.weight,
                        CandidateSource::Table,
                    )
                })
            }
        }
    }
}

pub(crate) struct CompactPrefixCandidates<'a> {
    prefix: &'a str,
    store: &'a CompactTableStore,
    group_index: usize,
    current: CompactPrefixCurrent<'a>,
    done: bool,
}

enum CompactPrefixCurrent<'a> {
    None,
    Owned {
        code: &'a str,
        entries: std::slice::Iter<'a, CompactTableEntry>,
    },
    ByteBacked {
        bytes: &'a [u8],
        code: ByteStringRef,
        entries: std::slice::Iter<'a, ByteBackedTableEntry>,
    },
}

impl<'a> Iterator for CompactPrefixCandidates<'a> {
    type Item = LookupCandidateEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.current {
                CompactPrefixCurrent::None => {}
                CompactPrefixCurrent::Owned { code, entries } => {
                    if let Some(entry) = entries.next() {
                        return Some(LookupCandidateEntry::new(
                            code,
                            LookupCandidate::new(
                                &entry.text,
                                code,
                                entry.weight,
                                CandidateSource::Table,
                            ),
                        ));
                    }
                    self.current = CompactPrefixCurrent::None;
                }
                CompactPrefixCurrent::ByteBacked {
                    bytes,
                    code,
                    entries,
                } => {
                    let code = code.as_str(bytes);
                    if let Some(entry) = entries.next() {
                        return Some(LookupCandidateEntry::new(
                            code,
                            LookupCandidate::new(
                                entry.text.as_str(bytes),
                                code,
                                entry.weight,
                                CandidateSource::Table,
                            ),
                        ));
                    }
                    self.current = CompactPrefixCurrent::None;
                }
            }

            match &self.store.storage {
                CompactTableStorage::Owned {
                    code_groups,
                    entries,
                } => {
                    if self.done || self.group_index >= code_groups.len() {
                        return None;
                    }
                    let group = &code_groups[self.group_index];
                    self.group_index += 1;
                    if !group.code.starts_with(self.prefix) {
                        self.done = true;
                        return None;
                    }
                    self.current = CompactPrefixCurrent::Owned {
                        code: &group.code,
                        entries: entries[group.entries.clone()].iter(),
                    };
                }
                CompactTableStorage::ByteBacked {
                    source,
                    code_groups,
                    entries,
                } => {
                    if self.done || self.group_index >= code_groups.len() {
                        return None;
                    }
                    let bytes = source.bytes();
                    let group = &code_groups[self.group_index];
                    self.group_index += 1;
                    if !group.code.as_str(bytes).starts_with(self.prefix) {
                        self.done = true;
                        return None;
                    }
                    self.current = CompactPrefixCurrent::ByteBacked {
                        bytes,
                        code: group.code,
                        entries: entries[group.entries.clone()].iter(),
                    };
                }
            }
        }
    }
}

pub(crate) struct CompactAllCodes<'a> {
    inner: CompactAllCodesInner<'a>,
}

enum CompactAllCodesInner<'a> {
    Owned(std::slice::Iter<'a, CompactCodeGroup>),
    ByteBacked {
        bytes: &'a [u8],
        inner: std::slice::Iter<'a, ByteBackedCodeGroup>,
    },
}

impl<'a> Iterator for CompactAllCodes<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            CompactAllCodesInner::Owned(inner) => inner.next().map(|group| group.code.as_str()),
            CompactAllCodesInner::ByteBacked { bytes, inner } => {
                inner.next().map(|group| group.code.as_str(bytes))
            }
        }
    }
}

impl TableLookup for CompactTableStore {
    type ExactCandidates<'a> = CompactExactCandidates<'a>;
    type PrefixCandidates<'a> = CompactPrefixCandidates<'a>;
    type AllCodes<'a> = CompactAllCodes<'a>;

    fn has_code(&self, code: &str) -> bool {
        self.group_index(code).is_ok()
    }

    fn exact_candidates<'a>(&'a self, code: &'a str) -> Self::ExactCandidates<'a> {
        let inner = if let Some((code, entries)) = self.exact_entries(code) {
            CompactExactCandidatesInner::Owned {
                code,
                inner: entries.iter(),
            }
        } else if let Some((source, group, entries)) = self.byte_backed_exact_entries(code) {
            CompactExactCandidatesInner::ByteBacked {
                bytes: source.bytes(),
                code: group.code,
                inner: entries.iter(),
            }
        } else {
            CompactExactCandidatesInner::Empty
        };
        CompactExactCandidates { inner }
    }

    fn prefix_candidates<'a>(&'a self, prefix: &'a str) -> Self::PrefixCandidates<'a> {
        CompactPrefixCandidates {
            prefix,
            store: self,
            group_index: self.group_index(prefix).unwrap_or_else(|index| index),
            current: CompactPrefixCurrent::None,
            done: false,
        }
    }

    fn all_codes(&self) -> Self::AllCodes<'_> {
        let inner = match &self.storage {
            CompactTableStorage::Owned { code_groups, .. } => {
                CompactAllCodesInner::Owned(code_groups.iter())
            }
            CompactTableStorage::ByteBacked {
                source,
                code_groups,
                ..
            } => CompactAllCodesInner::ByteBacked {
                bytes: source.bytes(),
                inner: code_groups.iter(),
            },
        };
        CompactAllCodes { inner }
    }
}

pub fn parse_rime_table_bin_advanced_data(
    bytes: impl AsRef<[u8]>,
) -> Result<TableDictionaryAdvancedData, RimeTableBinParseError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 68)?;
    let index_offset =
        read_offset_ptr(bytes, 48)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let advanced = read_yune_table_advanced_payload(bytes, total_index_end(bytes, index_offset)?)?;
    if !advanced.entries.is_empty() {
        return Err(RimeTableBinParseError::UnsupportedSection {
            role: "byte-backed advanced table entries".to_owned(),
        });
    }
    Ok(advanced.data)
}

fn read_syllabary_refs(
    bytes: &[u8],
    offset: usize,
) -> Result<Vec<ByteStringRef>, RimeTableBinParseError> {
    let count = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let total = count
        .checked_mul(4)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimeTableBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }

    let mut syllables = Vec::with_capacity(count);
    for index in 0..count {
        let field_offset = start
            .checked_add(
                index
                    .checked_mul(4)
                    .ok_or(RimeTableBinParseError::InvalidCount)?,
            )
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        syllables.push(read_string_ref_type(bytes, field_offset)?);
    }
    Ok(syllables)
}

fn read_byte_backed_head_index_entries(
    bytes: &[u8],
    offset: usize,
    syllables: &[ByteStringRef],
) -> Result<(Vec<ByteBackedCodeGroup>, Vec<ByteBackedTableEntry>), RimeTableBinParseError> {
    let count = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let node_size = 16usize;
    let total = count
        .checked_mul(node_size)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimeTableBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimeTableBinParseError::InvalidCount);
    }
    if count > syllables.len() {
        return Err(RimeTableBinParseError::InvalidCount);
    }

    let mut grouped = Vec::<(ByteStringRef, Vec<ByteBackedTableEntry>)>::new();
    for (index, syllable) in syllables.iter().copied().enumerate().take(count) {
        let node_offset = start
            .checked_add(
                index
                    .checked_mul(node_size)
                    .ok_or(RimeTableBinParseError::InvalidCount)?,
            )
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let entry_count = read_count(bytes, node_offset)?;
        let entries_offset = read_offset_ptr(bytes, node_offset + 4)?;
        let next_level = read_offset_ptr(bytes, node_offset + 8)?;
        if next_level.is_some() {
            return Err(RimeTableBinParseError::UnsupportedSection {
                role: "multi-level phrase index".to_owned(),
            });
        }
        let Some(entries_offset) = entries_offset else {
            if entry_count == 0 {
                continue;
            }
            return Err(RimeTableBinParseError::MissingRequiredSection);
        };
        grouped.push((
            syllable,
            read_byte_backed_entry_list(bytes, entries_offset, entry_count)?,
        ));
    }

    grouped.sort_by(|left, right| left.0.as_str(bytes).cmp(right.0.as_str(bytes)));
    let mut code_groups = Vec::with_capacity(grouped.len());
    let mut entries = Vec::new();
    for (code, group_entries) in grouped {
        let start = entries.len();
        entries.extend(group_entries);
        let end = entries.len();
        code_groups.push(ByteBackedCodeGroup {
            code,
            entries: start..end,
        });
    }
    Ok((code_groups, entries))
}

fn read_byte_backed_entry_list(
    bytes: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<ByteBackedTableEntry>, RimeTableBinParseError> {
    let entry_size = 8usize;
    let total = count
        .checked_mul(entry_size)
        .and_then(|len| offset.checked_add(len))
        .ok_or(RimeTableBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let entry_offset = offset
            .checked_add(
                index
                    .checked_mul(entry_size)
                    .ok_or(RimeTableBinParseError::InvalidCount)?,
            )
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let text = read_string_ref_type(bytes, entry_offset)?;
        let weight = read_f32_le(bytes, entry_offset + 4).map_err(map_metadata_error)?;
        entries.push(ByteBackedTableEntry { text, weight });
    }
    Ok(entries)
}

fn read_string_ref_type(
    bytes: &[u8],
    offset: usize,
) -> Result<ByteStringRef, RimeTableBinParseError> {
    let string_offset =
        read_offset_ptr(bytes, offset)?.ok_or(RimeTableBinParseError::OutOfBounds)?;
    read_c_string_ref(bytes, string_offset)
}

fn read_c_string_ref(bytes: &[u8], offset: usize) -> Result<ByteStringRef, RimeTableBinParseError> {
    if offset >= bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }
    let end = bytes[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|position| offset + position)
        .ok_or(RimeTableBinParseError::InvalidLength)?;
    std::str::from_utf8(&bytes[offset..end]).map_err(|_| RimeTableBinParseError::InvalidUtf8)?;
    Ok(ByteStringRef {
        offset,
        len: end - offset,
    })
}

#[must_use]
pub fn rime_table_bin_dict_file_checksum(bytes: impl AsRef<[u8]>) -> Option<u32> {
    read_u32_le(bytes.as_ref(), 32).ok()
}

pub fn parse_rime_table_bin_dictionary(
    bytes: impl AsRef<[u8]>,
) -> Result<TableDictionary, RimeTableBinParseError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 68)?;
    let version = parse_rime_format_version_for_payload(bytes, b"Rime::Table/")
        .map_err(map_metadata_error)?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimeTableBinParseError::UnsupportedVersion);
    }

    let syllabary_offset =
        read_offset_ptr(bytes, 44)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let index_offset =
        read_offset_ptr(bytes, 48)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let string_table_offset = read_offset_ptr(bytes, 60)?;
    let string_table_size = read_u32_le(bytes, 64).map_err(map_metadata_error)?;
    if string_table_offset.is_some() || string_table_size != 0 {
        return Err(RimeTableBinParseError::UnsupportedSection {
            role: "marisa string_table".to_owned(),
        });
    }

    let syllables = read_syllabary(bytes, syllabary_offset)?;
    let mut entries = read_head_index_entries(bytes, index_offset, &syllables)?;
    let advanced = read_yune_table_advanced_payload(bytes, total_index_end(bytes, index_offset)?)?;
    entries.extend(advanced.entries);
    Ok(TableDictionary::with_advanced_data(entries, advanced.data))
}

fn read_syllabary(bytes: &[u8], offset: usize) -> Result<Vec<String>, RimeTableBinParseError> {
    let count = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let total = count
        .checked_mul(4)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimeTableBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }

    let mut syllables = Vec::with_capacity(count);
    for index in 0..count {
        let field_offset = start
            .checked_add(
                index
                    .checked_mul(4)
                    .ok_or(RimeTableBinParseError::InvalidCount)?,
            )
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        syllables.push(read_string_type(bytes, field_offset)?);
    }
    Ok(syllables)
}

fn read_head_index_entries(
    bytes: &[u8],
    offset: usize,
    syllables: &[String],
) -> Result<Vec<TableEntry>, RimeTableBinParseError> {
    let count = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let node_size = 16usize;
    let total = count
        .checked_mul(node_size)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimeTableBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimeTableBinParseError::InvalidCount);
    }
    if count > syllables.len() {
        return Err(RimeTableBinParseError::InvalidCount);
    }

    let mut entries = Vec::new();
    for (index, syllable) in syllables.iter().enumerate().take(count) {
        let node_offset = start
            .checked_add(
                index
                    .checked_mul(node_size)
                    .ok_or(RimeTableBinParseError::InvalidCount)?,
            )
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let entry_count = read_count(bytes, node_offset)?;
        let entries_offset = read_offset_ptr(bytes, node_offset + 4)?
            .ok_or(RimeTableBinParseError::MissingRequiredSection)?;
        let next_level = read_offset_ptr(bytes, node_offset + 8)?;
        if next_level.is_some() {
            return Err(RimeTableBinParseError::UnsupportedSection {
                role: "multi-level phrase index".to_owned(),
            });
        }
        entries.extend(read_entry_list(
            bytes,
            entries_offset,
            entry_count,
            syllable,
        )?);
    }
    Ok(entries)
}

struct AdvancedTablePayload {
    entries: Vec<TableEntry>,
    data: TableDictionaryAdvancedData,
}

fn total_index_end(bytes: &[u8], offset: usize) -> Result<usize, RimeTableBinParseError> {
    let count = read_count(bytes, offset)?;
    offset
        .checked_add(4)
        .and_then(|start| start.checked_add(count.checked_mul(16)?))
        .ok_or(RimeTableBinParseError::InvalidCount)
}

fn read_yune_table_advanced_payload(
    bytes: &[u8],
    offset: usize,
) -> Result<AdvancedTablePayload, RimeTableBinParseError> {
    let marker = b"YUNE-TABLE-ADV\0";
    let Some(marker_offset) = bytes
        .get(offset..)
        .and_then(|tail| {
            tail.windows(marker.len())
                .position(|window| window == marker)
        })
        .map(|position| offset + position)
    else {
        return Ok(AdvancedTablePayload {
            entries: Vec::new(),
            data: TableDictionaryAdvancedData::default(),
        });
    };

    let mut cursor = marker_offset
        .checked_add(marker.len())
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let stem_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let mut stems = HashMap::new();
    for _ in 0..stem_count {
        let (text, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let count = read_count(bytes, cursor)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            let (stem, next) = read_len_string(bytes, cursor)?;
            cursor = next;
            values.push(stem);
        }
        stems.insert(text, values);
    }

    let entry_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let (text, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let (code, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let weight = read_f32_le(bytes, cursor).map_err(map_metadata_error)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        entries.push(TableEntry::new(code, text, weight));
    }

    let rule_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let mut encoder = TableEncoder::new();
    for _ in 0..rule_count {
        let length = read_count(bytes, cursor)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let (formula, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        encoder
            .add_length_equal_rule(length, &formula)
            .map_err(|_| RimeTableBinParseError::InvalidLength)?;
    }

    let (corrections, tolerance_rules, next_cursor) = if cursor < bytes.len() {
        read_correction_tolerance_payload(bytes, cursor)?
    } else {
        (Vec::new(), Vec::new(), cursor)
    };
    cursor = next_cursor;
    let lookup_records = if cursor < bytes.len() {
        read_lookup_record_payload(bytes, cursor)?
    } else {
        HashMap::new()
    };

    Ok(AdvancedTablePayload {
        entries,
        data: TableDictionaryAdvancedData {
            stems,
            encoder,
            corrections,
            tolerance_rules,
            lookup_records,
            ..TableDictionaryAdvancedData::default()
        },
    })
}

fn read_correction_tolerance_payload(
    bytes: &[u8],
    mut cursor: usize,
) -> Result<(Vec<RimeCorrectionEntry>, Vec<RimeToleranceRule>, usize), RimeTableBinParseError> {
    if !bytes[cursor..].starts_with(b"YUNE-CORR-TOL\0") {
        return Err(RimeTableBinParseError::UnsupportedSection {
            role: "correction/tolerance payload".to_owned(),
        });
    }
    cursor = cursor
        .checked_add(b"YUNE-CORR-TOL\0".len())
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let correction_count = read_count(bytes, cursor)?;
    if correction_count > MAX_CORRECTION_COUNT {
        return Err(RimeTableBinParseError::InvalidCount);
    }
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let mut corrections = Vec::with_capacity(correction_count);
    for _ in 0..correction_count {
        let (observed_input, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let (canonical_code, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        corrections.push(RimeCorrectionEntry::new(observed_input, canonical_code));
    }

    let tolerance_count = read_count(bytes, cursor)?;
    if tolerance_count > MAX_TOLERANCE_RULE_COUNT {
        return Err(RimeTableBinParseError::InvalidCount);
    }
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let mut tolerance_rules = Vec::with_capacity(tolerance_count);
    for _ in 0..tolerance_count {
        let (near_code, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let candidate_count = read_count(bytes, cursor)?;
        if candidate_count > MAX_TOLERANCE_CANDIDATE_COUNT {
            return Err(RimeTableBinParseError::InvalidCount);
        }
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let mut candidate_codes = Vec::with_capacity(candidate_count);
        for _ in 0..candidate_count {
            let (candidate_code, next) = read_len_string(bytes, cursor)?;
            cursor = next;
            candidate_codes.push(candidate_code);
        }
        tolerance_rules.push(RimeToleranceRule::new(near_code, candidate_codes));
    }
    Ok((corrections, tolerance_rules, cursor))
}

fn read_lookup_record_payload(
    bytes: &[u8],
    mut cursor: usize,
) -> Result<HashMap<String, Vec<DictionaryLookupRecord>>, RimeTableBinParseError> {
    if !bytes[cursor..].starts_with(b"YUNE-LOOKUP\0") {
        return Err(RimeTableBinParseError::UnsupportedSection {
            role: "lookup record payload".to_owned(),
        });
    }
    cursor = cursor
        .checked_add(b"YUNE-LOOKUP\0".len())
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let text_count = read_count(bytes, cursor)?;
    if text_count > MAX_LOOKUP_TEXT_COUNT {
        return Err(RimeTableBinParseError::InvalidCount);
    }
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;

    let mut lookup_records = HashMap::with_capacity(text_count);
    for _ in 0..text_count {
        let (text, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let record_count = read_count(bytes, cursor)?;
        if record_count > MAX_LOOKUP_RECORD_COUNT {
            return Err(RimeTableBinParseError::InvalidCount);
        }
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;

        let mut records = Vec::with_capacity(record_count);
        for _ in 0..record_count {
            let (code, next) = read_len_string(bytes, cursor)?;
            cursor = next;
            let field_count = read_count(bytes, cursor)?;
            if field_count > MAX_LOOKUP_FIELD_COUNT {
                return Err(RimeTableBinParseError::InvalidCount);
            }
            cursor = cursor
                .checked_add(4)
                .ok_or(RimeTableBinParseError::OutOfBounds)?;

            let mut fields = Vec::with_capacity(field_count);
            for _ in 0..field_count {
                let (field, next) = read_len_string(bytes, cursor)?;
                cursor = next;
                fields.push(field);
            }
            records.push(DictionaryLookupRecord { code, fields });
        }
        lookup_records.insert(text, records);
    }
    if cursor != bytes.len() {
        return Err(RimeTableBinParseError::UnsupportedSection {
            role: "trailing table payload".to_owned(),
        });
    }
    Ok(lookup_records)
}

fn read_entry_list(
    bytes: &[u8],
    offset: usize,
    count: usize,
    code: &str,
) -> Result<Vec<TableEntry>, RimeTableBinParseError> {
    let entry_size = 8usize;
    let total = count
        .checked_mul(entry_size)
        .and_then(|len| offset.checked_add(len))
        .ok_or(RimeTableBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }

    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let entry_offset = offset
            .checked_add(
                index
                    .checked_mul(entry_size)
                    .ok_or(RimeTableBinParseError::InvalidCount)?,
            )
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let text = read_string_type(bytes, entry_offset)?;
        let weight = read_f32_le(bytes, entry_offset + 4).map_err(map_metadata_error)?;
        entries.push(TableEntry::new(code, text, weight));
    }
    Ok(entries)
}

fn read_string_type(bytes: &[u8], offset: usize) -> Result<String, RimeTableBinParseError> {
    let string_offset =
        read_offset_ptr(bytes, offset)?.ok_or(RimeTableBinParseError::OutOfBounds)?;
    read_c_string(bytes, string_offset)
}

fn read_c_string(bytes: &[u8], offset: usize) -> Result<String, RimeTableBinParseError> {
    if offset >= bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }
    let end = bytes[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|position| offset + position)
        .ok_or(RimeTableBinParseError::InvalidLength)?;
    std::str::from_utf8(&bytes[offset..end])
        .map(str::to_owned)
        .map_err(|_| RimeTableBinParseError::InvalidUtf8)
}

fn read_len_string(bytes: &[u8], offset: usize) -> Result<(String, usize), RimeTableBinParseError> {
    let len = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let end = start
        .checked_add(len)
        .ok_or(RimeTableBinParseError::InvalidLength)?;
    if end > bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }
    let value = std::str::from_utf8(&bytes[start..end])
        .map(str::to_owned)
        .map_err(|_| RimeTableBinParseError::InvalidUtf8)?;
    Ok((value, end))
}

fn read_offset_ptr(
    bytes: &[u8],
    field_offset: usize,
) -> Result<Option<usize>, RimeTableBinParseError> {
    let raw = read_i32_le(bytes, field_offset).map_err(map_metadata_error)?;
    if raw == 0 {
        return Ok(None);
    }
    let target = field_offset
        .checked_add_signed(raw as isize)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    if target >= bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }
    Ok(Some(target))
}

fn read_count(bytes: &[u8], offset: usize) -> Result<usize, RimeTableBinParseError> {
    let count = read_u32_le(bytes, offset).map_err(map_metadata_error)?;
    usize::try_from(count).map_err(|_| RimeTableBinParseError::InvalidCount)
}

fn ensure_len(bytes: &[u8], len: usize) -> Result<(), RimeTableBinParseError> {
    if bytes.len() < len {
        return Err(RimeTableBinParseError::TooShort);
    }
    Ok(())
}

fn map_metadata_error(error: super::RimeCompiledMetadataError) -> RimeTableBinParseError {
    match error {
        super::RimeCompiledMetadataError::TooShort => RimeTableBinParseError::TooShort,
        super::RimeCompiledMetadataError::InvalidFormat => RimeTableBinParseError::InvalidFormat,
        super::RimeCompiledMetadataError::UnsupportedVersion => {
            RimeTableBinParseError::UnsupportedVersion
        }
        super::RimeCompiledMetadataError::MissingRequiredSection => {
            RimeTableBinParseError::MissingRequiredSection
        }
    }
}
