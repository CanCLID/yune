use super::{
    DictionaryLookupRecord, RimeCorrectionEntry, RimeToleranceRule, TableDictionary,
    TableDictionaryAdvancedData, TableEncoder, TableEntry, TableEntryWeightDomain,
};
use crate::dictionary::compiled::{
    parse_rime_format_version_for_payload, read_f32_le, read_i32_le, read_u32_le,
};
use crate::dictionary::query_table::{LookupCandidate, LookupCandidateEntry, TableLookup};
use crate::dictionary::source::{
    ByteBackedDictionaryLookupRecords, DictionaryLookupByteSource, DictionaryLookupByteStoreError,
};
use crate::CandidateSource;
use crate::{MemoryOwnerClass, MemoryOwnerRow};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::mem;
use std::ops::Range;
use std::sync::Arc;

const MAX_CORRECTION_COUNT: usize = 4096;
const MAX_TOLERANCE_RULE_COUNT: usize = 4096;
const MAX_TOLERANCE_CANDIDATE_COUNT: usize = 64;
const MAX_LOOKUP_TEXT_COUNT: usize = 1_000_000;
const MAX_LOOKUP_RECORD_COUNT: usize = 1_000_000;
const MAX_LOOKUP_FIELD_COUNT: usize = 64;
pub(super) const RIME_TABLE_HEADER_LEN: usize = 68;
pub(super) const YUNE_TABLE_METADATA_MARKER: &[u8] = b"YUNE-TABLE-META\0";
const YUNE_TABLE_METADATA_PREFIX: &[u8] = b"YUNE-TABLE-META";
pub(super) const YUNE_TABLE_METADATA_VERSION: u32 = 2;
pub(super) const YUNE_TABLE_METADATA_PAYLOAD_LEN: usize = 4;
pub(super) const YUNE_TABLE_SORT_BY_WEIGHT: u8 = 0;
pub(super) const YUNE_TABLE_SORT_ORIGINAL: u8 = 1;
pub(super) const YUNE_TABLE_WEIGHT_NATURAL_LOG: u8 = 1;
const YUNE_TABLE_METADATA_V1: u32 = 1;
const YUNE_TABLE_ADVANCED_MARKER: &[u8] = b"YUNE-TABLE-ADV\0";

/// Allocation-free indexed access to canonical syllabary codes.
pub trait CanonicalCodeSequence {
    fn len(&self) -> usize;

    fn get(&self, index: usize) -> Option<&str>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> CanonicalCodeSequence for T
where
    T: AsRef<[String]> + ?Sized,
{
    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn get(&self, index: usize) -> Option<&str> {
        self.as_ref().get(index).map(String::as_str)
    }
}

/// Canonical syllabary codes stored in one UTF-8 buffer with cumulative offsets.
#[derive(Debug)]
pub struct PackedSyllabaryCodes {
    utf8: Box<str>,
    offsets: Box<[u32]>,
}

/// A borrowed canonical code yielded without recreating its source `String`.
#[derive(Debug, Eq, PartialEq)]
pub struct PackedCodeRef<'a>(&'a str);

impl<'a> PackedCodeRef<'a> {
    #[must_use]
    pub fn as_str(&self) -> &'a str {
        self.0
    }

    /// Preserves the former `&String::clone()` call shape for callers that
    /// explicitly need to retain a code after the packed store is moved. The
    /// iterator itself remains a borrowed `&str` view and prism hot paths use
    /// indexed borrowed access without allocating.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn clone(&self) -> String {
        self.0.to_owned()
    }
}

impl std::ops::Deref for PackedCodeRef<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl AsRef<str> for PackedCodeRef<'_> {
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl Default for PackedSyllabaryCodes {
    fn default() -> Self {
        Self {
            utf8: String::new().into_boxed_str(),
            offsets: vec![0].into_boxed_slice(),
        }
    }
}

impl PackedSyllabaryCodes {
    fn try_from_codes<'a>(
        codes: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, RimeTableBinParseError> {
        let mut utf8 = String::new();
        let mut offsets = vec![0];
        for code in codes {
            let current = *offsets
                .last()
                .expect("packed syllabary always starts with offset zero");
            let next = checked_packed_offset(current, code.len())?;
            utf8.push_str(code);
            offsets.push(next);
        }
        Ok(Self {
            utf8: utf8.into_boxed_str(),
            offsets: offsets.into_boxed_slice(),
        })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&str> {
        let start = usize::try_from(*self.offsets.get(index)?).ok()?;
        let end = usize::try_from(*self.offsets.get(index.checked_add(1)?)?).ok()?;
        self.utf8.get(start..end)
    }

    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = PackedCodeRef<'_>> + DoubleEndedIterator + '_ {
        (0..self.len()).map(|index| {
            PackedCodeRef(
                self.get(index)
                    .expect("packed syllabary offsets are validated during construction"),
            )
        })
    }

    #[must_use]
    pub fn to_vec(&self) -> Vec<String> {
        self.iter().map(|code| code.as_str().to_owned()).collect()
    }

    fn heap_bytes(&self) -> usize {
        self.utf8
            .len()
            .saturating_add(self.offsets.len().saturating_mul(mem::size_of::<u32>()))
    }
}

impl CanonicalCodeSequence for PackedSyllabaryCodes {
    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Option<&str> {
        self.get(index)
    }
}

fn checked_packed_offset(current: u32, code_len: usize) -> Result<u32, RimeTableBinParseError> {
    let code_len = u32::try_from(code_len).map_err(|_| RimeTableBinParseError::InvalidLength)?;
    current
        .checked_add(code_len)
        .ok_or(RimeTableBinParseError::InvalidLength)
}

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

    fn marisa_string_table(
        &self,
        offset: usize,
        size: usize,
    ) -> Result<Box<dyn CompactMarisaStringTable>, RimeTableBinParseError> {
        SafeReadMarisaStringTable::from_bytes(self.bytes(), offset, size)
            .map(|table| Box::new(table) as Box<dyn CompactMarisaStringTable>)
    }
}

pub trait CompactMarisaStringTable: fmt::Debug + Send + Sync {
    fn get(&self, id: u32) -> Option<String>;

    fn num_keys(&self) -> usize;

    fn keys_for_ids(&self, ids: &[u32]) -> Result<Vec<String>, RimeTableBinParseError> {
        ids.iter()
            .map(|id| {
                (usize::try_from(*id).ok().filter(|id| *id < self.num_keys()))
                    .and_then(|_| self.get(*id))
                    .ok_or(RimeTableBinParseError::InvalidCount)
            })
            .collect()
    }

    fn mapping_mode(&self) -> &'static str;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RimeTableBinAdvancedDataOptions {
    pub load_lookup_records: bool,
    pub byte_back_lookup_records: bool,
}

impl Default for RimeTableBinAdvancedDataOptions {
    fn default() -> Self {
        Self {
            load_lookup_records: true,
            byte_back_lookup_records: false,
        }
    }
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

#[derive(Debug)]
struct CompactLookupByteSource {
    source: Arc<dyn CompactTableByteSource>,
}

impl DictionaryLookupByteSource for CompactLookupByteSource {
    fn bytes(&self) -> &[u8] {
        self.source.bytes()
    }

    fn storage_label(&self) -> &'static str {
        self.source.storage_label()
    }

    fn mapping_mode(&self) -> &'static str {
        self.source.mapping_mode()
    }
}

#[derive(Debug)]
pub struct CompactTableStore {
    syllabary_codes: PackedSyllabaryCodes,
    storage: CompactTableStorage,
    advanced: TableDictionaryAdvancedData,
    entry_weight_domain: TableEntryWeightDomain,
}

#[derive(Debug)]
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
    MarisaBacked {
        string_table: Box<dyn CompactMarisaStringTable>,
        source: Arc<dyn CompactTableByteSource>,
        index_offset: usize,
        entry_count: usize,
        syllable_ids_by_code: HashMap<String, usize>,
    },
}

#[derive(Clone, Copy, Debug)]
struct MarisaTableLayout {
    syllabary_offset: usize,
    index_offset: usize,
    string_table_offset: usize,
    string_table_size: usize,
    expected_syllable_count: usize,
    entry_weight_domain: TableEntryWeightDomain,
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

struct SafeReadMarisaStringTable {
    trie: rsmarisa::Trie,
    payload_range: Range<usize>,
    num_keys: usize,
}

impl fmt::Debug for SafeReadMarisaStringTable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SafeReadMarisaStringTable")
            .field("payload_range", &self.payload_range)
            .field("num_keys", &self.num_keys)
            .finish_non_exhaustive()
    }
}

impl SafeReadMarisaStringTable {
    fn from_bytes(
        bytes: &[u8],
        offset: usize,
        size: usize,
    ) -> Result<Self, RimeTableBinParseError> {
        let end = offset
            .checked_add(size)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let payload = bytes
            .get(offset..end)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let mut trie = rsmarisa::Trie::new();
        let mut reader = rsmarisa::grimoire::io::Reader::from_reader(std::io::Cursor::new(payload));
        trie.read(&mut reader)
            .map_err(|_| RimeTableBinParseError::InvalidFormat)?;
        let num_keys = trie.num_keys();
        Ok(Self {
            trie,
            payload_range: offset..end,
            num_keys,
        })
    }
}

impl CompactMarisaStringTable for SafeReadMarisaStringTable {
    fn get(&self, id: u32) -> Option<String> {
        if id as usize >= self.num_keys {
            return None;
        }
        let mut agent = rsmarisa::Agent::new();
        agent.set_query_id(id as usize);
        self.trie.reverse_lookup(&mut agent);
        Some(agent.key().as_str().to_owned())
    }

    fn num_keys(&self) -> usize {
        self.num_keys
    }

    fn keys_for_ids(&self, ids: &[u32]) -> Result<Vec<String>, RimeTableBinParseError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut positions = HashMap::<usize, Vec<usize>>::new();
        for (output_index, id) in ids.iter().copied().enumerate() {
            let id = usize::try_from(id).map_err(|_| RimeTableBinParseError::InvalidCount)?;
            if id >= self.num_keys {
                return Err(RimeTableBinParseError::InvalidCount);
            }
            positions.entry(id).or_default().push(output_index);
        }
        let mut output = vec![None; ids.len()];
        let requested_ids = positions.keys().copied().collect::<HashSet<_>>();
        let mut resolved_ids = HashSet::new();
        let mut agent = rsmarisa::Agent::new();
        agent.set_query_str("");
        while self.trie.predictive_search(&mut agent) {
            let id = agent.key().id();
            if requested_ids.contains(&id) && !resolved_ids.insert(id) {
                return Err(RimeTableBinParseError::InvalidCount);
            }
            let Some(output_indices) = positions.remove(&id) else {
                continue;
            };
            let key = std::str::from_utf8(agent.key().as_bytes())
                .map_err(|_| RimeTableBinParseError::InvalidUtf8)?
                .to_owned();
            for output_index in output_indices {
                output[output_index] = Some(key.clone());
            }
            if positions.is_empty() {
                break;
            }
        }
        if !positions.is_empty() {
            return Err(RimeTableBinParseError::InvalidCount);
        }
        output
            .into_iter()
            .map(|key| key.ok_or(RimeTableBinParseError::InvalidCount))
            .collect()
    }

    fn mapping_mode(&self) -> &'static str {
        "read_from_byte_source"
    }
}

impl CompactTableStore {
    #[must_use]
    pub(crate) fn from_dictionary(dictionary: TableDictionary) -> Self {
        let entry_weight_domain = dictionary.entry_weight_domain();
        let advanced = dictionary.advanced_data();
        Self::from_entries_and_advanced(dictionary.entries, advanced, entry_weight_domain)
    }

    fn from_entries_and_advanced(
        entries: Vec<TableEntry>,
        advanced: TableDictionaryAdvancedData,
        entry_weight_domain: TableEntryWeightDomain,
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
            syllabary_codes: PackedSyllabaryCodes::try_from_codes(
                syllabary_codes.iter().map(String::as_str),
            )
            .expect("in-memory syllabary aggregate length should fit u32"),
            storage: CompactTableStorage::Owned {
                code_groups,
                entries: compact_entries,
            },
            advanced,
            entry_weight_domain,
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
        mut advanced: TableDictionaryAdvancedData,
    ) -> Result<Self, RimeTableBinParseError> {
        let bytes = source.bytes();
        ensure_len(bytes, 68)?;
        let version = parse_rime_format_version_for_payload(bytes, b"Rime::Table/")
            .map_err(map_metadata_error)?;
        if version < 4.0 - f64::EPSILON {
            return Err(RimeTableBinParseError::UnsupportedVersion);
        }
        let expected_syllable_count =
            usize::try_from(read_u32_le(bytes, 36).map_err(map_metadata_error)?)
                .map_err(|_| RimeTableBinParseError::InvalidCount)?;

        let index_offset =
            read_offset_ptr(bytes, 48)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
        let syllabary_offset =
            read_offset_ptr(bytes, 44)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
        // Bind legacy weight-domain provenance to the table bytes. The
        // separately supplied advanced data may have been merged with source
        // metadata or constructed by a caller, so it cannot carry provenance
        // for this byte source.
        let yune_payload_present =
            yune_advanced_marker_offset(bytes, total_index_end(bytes, index_offset)?)?.is_some();
        let table_metadata = read_table_metadata(bytes, syllabary_offset, yune_payload_present)?;
        advanced.sort_by_weight = table_metadata.sort_by_weight;
        let string_table_offset = read_offset_ptr(bytes, 60)?;
        let string_table_size = read_u32_le(bytes, 64).map_err(map_metadata_error)?;
        if string_table_offset.is_some() || string_table_size != 0 {
            let string_table_offset =
                string_table_offset.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
            if string_table_size == 0 {
                return Err(RimeTableBinParseError::InvalidLength);
            }
            return Self::from_marisa_table_bin_byte_source(
                source,
                advanced,
                MarisaTableLayout {
                    syllabary_offset,
                    index_offset,
                    string_table_offset,
                    string_table_size: string_table_size as usize,
                    expected_syllable_count,
                    entry_weight_domain: table_metadata.entry_weight_domain,
                },
            );
        }

        let syllable_refs = read_syllabary_refs(bytes, syllabary_offset, expected_syllable_count)?;
        let syllabary_codes = PackedSyllabaryCodes::try_from_codes(
            syllable_refs
                .iter()
                .map(|reference| reference.as_str(bytes)),
        )?;
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
            entry_weight_domain: table_metadata.entry_weight_domain,
        })
    }

    fn from_marisa_table_bin_byte_source(
        source: Arc<dyn CompactTableByteSource>,
        advanced: TableDictionaryAdvancedData,
        layout: MarisaTableLayout,
    ) -> Result<Self, RimeTableBinParseError> {
        let string_table =
            source.marisa_string_table(layout.string_table_offset, layout.string_table_size)?;
        let syllable_ids = read_marisa_syllabary_ids(
            source.bytes(),
            layout.syllabary_offset,
            layout.expected_syllable_count,
        )?;
        let syllabary_codes = string_table.keys_for_ids(&syllable_ids)?;
        let syllable_ids_by_code = syllabary_codes
            .iter()
            .enumerate()
            .map(|(index, code)| (code.clone(), index))
            .collect::<HashMap<_, _>>();
        let entry_count =
            usize::try_from(read_u32_le(source.bytes(), 40).map_err(map_metadata_error)?)
                .map_err(|_| RimeTableBinParseError::InvalidCount)?;
        validate_marisa_head_index(source.bytes(), layout.index_offset, syllabary_codes.len())?;
        let syllabary_codes =
            PackedSyllabaryCodes::try_from_codes(syllabary_codes.iter().map(String::as_str))?;

        Ok(Self {
            syllabary_codes,
            storage: CompactTableStorage::MarisaBacked {
                string_table,
                source,
                index_offset: layout.index_offset,
                entry_count,
                syllable_ids_by_code,
            },
            advanced,
            entry_weight_domain: layout.entry_weight_domain,
        })
    }

    #[must_use]
    pub fn syllabary_codes(&self) -> &PackedSyllabaryCodes {
        &self.syllabary_codes
    }

    pub(crate) fn has_strict_code_prefix(&self, prefix: &str) -> bool {
        match &self.storage {
            CompactTableStorage::Owned { code_groups, .. } => {
                let mut index = self.group_index(prefix).unwrap_or_else(|index| index);
                if code_groups
                    .get(index)
                    .is_some_and(|group| group.code == prefix)
                {
                    index += 1;
                }
                code_groups
                    .get(index)
                    .is_some_and(|group| group.code.starts_with(prefix))
            }
            CompactTableStorage::ByteBacked {
                source,
                code_groups,
                ..
            } => {
                let bytes = source.bytes();
                let mut index = self.group_index(prefix).unwrap_or_else(|index| index);
                if code_groups
                    .get(index)
                    .is_some_and(|group| group.code.as_str(bytes) == prefix)
                {
                    index += 1;
                }
                code_groups
                    .get(index)
                    .is_some_and(|group| group.code.as_str(bytes).starts_with(prefix))
            }
            CompactTableStorage::MarisaBacked { .. } => self
                .prefix_candidates(prefix)
                .any(|entry| entry.into_parts().0.len() > prefix.len()),
        }
    }

    #[must_use]
    pub fn exact_candidate_count(&self, code: &str) -> usize {
        self.exact_candidates(code).count()
    }

    #[must_use]
    pub fn code_count(&self) -> usize {
        match &self.storage {
            CompactTableStorage::Owned { code_groups, .. } => code_groups.len(),
            CompactTableStorage::ByteBacked { code_groups, .. } => code_groups.len(),
            CompactTableStorage::MarisaBacked {
                syllable_ids_by_code,
                ..
            } => syllable_ids_by_code.len(),
        }
    }

    #[must_use]
    pub fn distinct_code_count(&self) -> usize {
        self.all_codes()
            .map(|code| code.into_owned())
            .collect::<BTreeSet<_>>()
            .len()
    }

    #[must_use]
    pub fn expanded_entry_count(&self) -> usize {
        self.all_codes()
            .map(|code| self.exact_candidate_count(code.as_ref()))
            .sum()
    }

    #[must_use]
    pub fn advanced_data(&self) -> TableDictionaryAdvancedData {
        self.advanced.clone()
    }

    /// RIME table ordering policy. Compiled tables without Yune's optional
    /// header-gap metadata retain the upstream `by_weight` default.
    #[must_use]
    pub fn sort_by_weight(&self) -> bool {
        self.advanced.sort_by_weight.unwrap_or(true)
    }

    #[must_use]
    pub(crate) fn entry_weight_domain(&self) -> TableEntryWeightDomain {
        self.entry_weight_domain
    }

    #[must_use]
    pub fn to_table_dictionary(&self) -> TableDictionary {
        let entries = self
            .all_codes()
            .flat_map(|code| {
                let code = code.into_owned();
                self.exact_candidates(&code)
                    .map(|candidate| {
                        TableEntry::new(&code, candidate.text(), candidate.raw_quality())
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut dictionary = TableDictionary::with_advanced_data(entries, self.advanced.clone());
        dictionary.set_entry_weight_domain(self.entry_weight_domain);
        dictionary
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
            CompactTableStorage::MarisaBacked { .. } => Err(0),
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

    fn marisa_exact_entry_cursors(
        &self,
        code: &str,
    ) -> Option<(
        &dyn CompactMarisaStringTable,
        &[u8],
        Vec<MarisaEntryCursorSpec>,
    )> {
        let CompactTableStorage::MarisaBacked {
            string_table,
            source,
            index_offset,
            syllable_ids_by_code,
            ..
        } = &self.storage
        else {
            return None;
        };
        let cursors = segment_marisa_code_paths(code, syllable_ids_by_code)
            .into_iter()
            .filter_map(|syllable_ids| {
                marisa_entry_cursor_for_syllable_ids(source.bytes(), *index_offset, &syllable_ids)
            })
            .collect::<Vec<_>>();
        (!cursors.is_empty()).then_some((string_table.as_ref(), source.bytes(), cursors))
    }

    #[must_use]
    pub fn storage_label(&self) -> &'static str {
        match &self.storage {
            CompactTableStorage::Owned { .. } => "owned_heap",
            CompactTableStorage::ByteBacked { source, .. } => source.storage_label(),
            CompactTableStorage::MarisaBacked { .. } => "rsmarisa_byte_backed",
        }
    }

    #[must_use]
    pub fn mapping_mode(&self) -> &'static str {
        match &self.storage {
            CompactTableStorage::Owned { .. } => "owned_heap",
            CompactTableStorage::ByteBacked { source, .. } => source.mapping_mode(),
            CompactTableStorage::MarisaBacked { source, .. } => source.mapping_mode(),
        }
    }

    #[must_use]
    pub fn stored_entry_count(&self) -> usize {
        match &self.storage {
            CompactTableStorage::Owned { entries, .. } => entries.len(),
            CompactTableStorage::ByteBacked { entries, .. } => entries.len(),
            CompactTableStorage::MarisaBacked { entry_count, .. } => *entry_count,
        }
    }

    #[must_use]
    pub fn byte_source_len(&self) -> usize {
        match &self.storage {
            CompactTableStorage::Owned { .. } => 0,
            CompactTableStorage::ByteBacked { source, .. } => source.bytes().len(),
            CompactTableStorage::MarisaBacked { source, .. } => source.bytes().len(),
        }
    }

    #[must_use]
    pub fn is_marisa_backed(&self) -> bool {
        matches!(self.storage, CompactTableStorage::MarisaBacked { .. })
    }

    #[must_use]
    pub fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let mut rows = vec![MemoryOwnerRow::new(
            "compact_table.syllabary_codes",
            MemoryOwnerClass::HeapOwnedReducible,
            self.syllabary_codes.heap_bytes(),
            self.syllabary_codes.len(),
            "Box<str> + Box<[u32]>",
            "packed canonical code UTF-8 buffer and cumulative offsets retained for prism lookup",
        )];
        rows.extend(self.storage.memory_owner_rows());
        rows.extend(self.storage.payload_owner_rows());
        rows.extend(advanced_memory_owner_rows(&self.advanced));
        rows
    }
}

impl CompactTableStorage {
    fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        match self {
            Self::Owned {
                code_groups,
                entries,
            } => vec![
                MemoryOwnerRow::new(
                    "compact_table.syllable_ids_by_code",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "none",
                    "owned compact storage does not retain the rsmarisa syllable map",
                ),
                MemoryOwnerRow::new(
                    "compact_table.storage",
                    MemoryOwnerClass::HeapOwnedGuarded,
                    estimate_owned_storage_bytes(code_groups, entries),
                    code_groups.len().saturating_add(entries.len()),
                    "owned_heap",
                    "owned compact table records; not a mapped file",
                ),
            ],
            Self::ByteBacked {
                source,
                code_groups,
                entries,
            } => vec![
                MemoryOwnerRow::new(
                    "compact_table.syllable_ids_by_code",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "none",
                    "byte-backed compact storage does not retain the rsmarisa syllable map",
                ),
                MemoryOwnerRow::new(
                    "compact_table.storage",
                    byte_source_class(source.as_ref()),
                    source.bytes().len(),
                    code_groups.len().saturating_add(entries.len()),
                    format!("{}:{}", source.storage_label(), source.mapping_mode()),
                    "table bytes are excluded from heap-owned branch triggers when mapped",
                ),
            ],
            Self::MarisaBacked {
                source,
                entry_count,
                syllable_ids_by_code,
                ..
            } => vec![
                MemoryOwnerRow::new(
                    "compact_table.syllable_ids_by_code",
                    MemoryOwnerClass::HeapOwnedReducible,
                    estimate_string_usize_map_bytes(syllable_ids_by_code),
                    syllable_ids_by_code.len(),
                    "HashMap<String, usize>",
                    "rsmarisa lookup side map retained on heap",
                ),
                MemoryOwnerRow::new(
                    "compact_table.storage",
                    byte_source_class(source.as_ref()),
                    source.bytes().len(),
                    *entry_count,
                    format!("{}:{}", source.storage_label(), source.mapping_mode()),
                    "rsmarisa table bytes are excluded from heap-owned branch triggers when mapped",
                ),
            ],
        }
    }

    fn payload_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let (class, storage, text_bytes, comment_bytes, entry_count) = match self {
            Self::Owned {
                code_groups,
                entries,
            } => {
                let text_bytes = entries
                    .iter()
                    .map(|entry| estimate_owned_string_bytes(&entry.text))
                    .sum();
                let comment_bytes = code_groups
                    .iter()
                    .map(|group| group.code.capacity().saturating_mul(group.entries.len()))
                    .sum();
                (
                    MemoryOwnerClass::HeapOwnedRequired,
                    "owned_heap",
                    text_bytes,
                    comment_bytes,
                    entries.len(),
                )
            }
            Self::ByteBacked {
                code_groups,
                entries,
                ..
            } => {
                let text_bytes = entries.iter().map(|entry| entry.text.len).sum();
                let comment_bytes = code_groups
                    .iter()
                    .map(|group| group.code.len.saturating_mul(group.entries.len()))
                    .sum();
                (
                    MemoryOwnerClass::SharedOrOverlapping,
                    "table_bin_byte_refs",
                    text_bytes,
                    comment_bytes,
                    entries.len(),
                )
            }
            Self::MarisaBacked {
                source,
                entry_count,
                ..
            } => (
                MemoryOwnerClass::SharedOrOverlapping,
                source.storage_label(),
                source.bytes().len(),
                0,
                *entry_count,
            ),
        };
        vec![
            MemoryOwnerRow::new(
                "compact_table.candidate_text_payload",
                class,
                text_bytes,
                entry_count,
                storage,
                "candidate text payload; byte-backed rows overlap compact_table.storage",
            ),
            MemoryOwnerRow::new(
                "compact_table.candidate_comment_payload",
                class,
                comment_bytes,
                entry_count,
                storage,
                "candidate raw comments/code payload; byte-backed rows overlap compact_table.storage",
            ),
        ]
    }
}

fn byte_source_class(source: &dyn CompactTableByteSource) -> MemoryOwnerClass {
    if source.mapping_mode() == "mmap" {
        MemoryOwnerClass::MmapFileBacked
    } else {
        MemoryOwnerClass::HeapOwnedGuarded
    }
}

fn estimate_owned_string_bytes(value: &str) -> usize {
    mem::size_of::<String>().saturating_add(value.len())
}

fn estimate_string_usize_map_bytes(values: &HashMap<String, usize>) -> usize {
    mem::size_of::<HashMap<String, usize>>()
        .saturating_add(
            values
                .capacity()
                .saturating_mul(mem::size_of::<(String, usize)>()),
        )
        .saturating_add(values.keys().map(|value| value.capacity()).sum::<usize>())
}

fn estimate_owned_storage_bytes(
    code_groups: &[CompactCodeGroup],
    entries: &[CompactTableEntry],
) -> usize {
    mem::size_of::<CompactTableStorage>()
        .saturating_add(
            code_groups
                .len()
                .saturating_mul(mem::size_of::<CompactCodeGroup>()),
        )
        .saturating_add(
            entries
                .len()
                .saturating_mul(mem::size_of::<CompactTableEntry>()),
        )
        .saturating_add(
            code_groups
                .iter()
                .map(|group| group.code.capacity())
                .sum::<usize>(),
        )
        .saturating_add(
            entries
                .iter()
                .map(|entry| estimate_owned_string_bytes(&entry.text))
                .sum(),
        )
}

fn advanced_memory_owner_rows(advanced: &TableDictionaryAdvancedData) -> Vec<MemoryOwnerRow> {
    let lookup_record_row = if let Some(records) = &advanced.byte_backed_lookup_records {
        MemoryOwnerRow::new(
            "compact_table.lookup_records",
            byte_backed_lookup_owner_class(records),
            records
                .payload_bytes()
                .saturating_add(records.estimated_index_bytes()),
            records.record_count(),
            format!(
                "byte_backed_lookup_payload({}; {}; index_bytes={})",
                records.storage_label(),
                records.mapping_mode(),
                records.estimated_index_bytes()
            ),
            "dictionary lookup records retained as an indexed compiled payload instead of an eager HashMap",
        )
    } else if advanced.lookup_records.is_empty() {
        MemoryOwnerRow::new(
            "compact_table.lookup_records",
            MemoryOwnerClass::Shared,
            0,
            0,
            "none",
            "no lookup-record payload retained for this compact table",
        )
    } else {
        MemoryOwnerRow::new(
            "compact_table.lookup_records",
            MemoryOwnerClass::HeapOwnedRequired,
            estimate_lookup_records_bytes(&advanced.lookup_records),
            advanced.lookup_records.values().map(Vec::len).sum(),
            "HashMap<String, Vec<DictionaryLookupRecord>>",
            "retained dictionary lookup records required by TypeDuck dictionary panels",
        )
    };
    vec![
        MemoryOwnerRow::new(
            "compact_table.stems",
            MemoryOwnerClass::HeapOwnedRequired,
            estimate_string_vec_map_bytes(&advanced.stems),
            advanced.stems.values().map(Vec::len).sum(),
            "HashMap<String, Vec<String>>",
            "retained phrase-code stems needed for lookup records and dictionary panels",
        ),
        lookup_record_row,
        MemoryOwnerRow::new(
            "compact_table.corrections_tolerance",
            MemoryOwnerClass::HeapOwnedRequired,
            estimate_correction_tolerance_bytes(
                &advanced.corrections,
                &advanced.tolerance_rules,
            ),
            advanced
                .corrections
                .len()
                .saturating_add(advanced.tolerance_rules.len()),
            "Vec<RimeCorrectionEntry> + Vec<RimeToleranceRule>",
            "retained correction/tolerance payload required by TypeDuck fuzzy input behavior",
        ),
        MemoryOwnerRow::new(
            "compact_table.dict_settings",
            MemoryOwnerClass::HeapOwnedRequired,
            estimate_string_btree_map_bytes(&advanced.dict_settings),
            advanced.dict_settings.len(),
            "BTreeMap<String, String>",
            "retained dictionary settings parsed from deployed dictionary metadata",
        ),
        MemoryOwnerRow::new(
            "compact_table.preset_vocabulary",
            MemoryOwnerClass::HeapOwnedRequired,
            estimate_preset_vocabulary_bytes(&advanced.preset_vocabulary),
            advanced.preset_vocabulary.len(),
            "Vec<PresetVocabularyEntry>",
            "retained preset vocabulary weights when the selected dictionary includes vocabulary packs",
        ),
    ]
}

fn byte_backed_lookup_owner_class(records: &ByteBackedDictionaryLookupRecords) -> MemoryOwnerClass {
    match records.mapping_mode() {
        "mmap" => MemoryOwnerClass::SharedOrOverlapping,
        "owned_bytes" => MemoryOwnerClass::HeapOwnedGuarded,
        _ => MemoryOwnerClass::SharedOrOverlapping,
    }
}

fn estimate_string_vec_map_bytes(values: &HashMap<String, Vec<String>>) -> usize {
    mem::size_of::<HashMap<String, Vec<String>>>()
        .saturating_add(
            values
                .capacity()
                .saturating_mul(mem::size_of::<(String, Vec<String>)>()),
        )
        .saturating_add(
            values
                .iter()
                .map(|(key, list)| {
                    key.capacity()
                        .saturating_add(list.capacity().saturating_mul(mem::size_of::<String>()))
                        .saturating_add(list.iter().map(String::capacity).sum::<usize>())
                })
                .sum::<usize>(),
        )
}

fn estimate_string_btree_map_bytes(values: &BTreeMap<String, String>) -> usize {
    mem::size_of::<BTreeMap<String, String>>().saturating_add(
        values
            .iter()
            .map(|(key, value)| {
                mem::size_of::<(String, String)>()
                    .saturating_add(key.capacity())
                    .saturating_add(value.capacity())
            })
            .sum::<usize>(),
    )
}

fn estimate_lookup_records_bytes(values: &HashMap<String, Vec<DictionaryLookupRecord>>) -> usize {
    mem::size_of::<HashMap<String, Vec<DictionaryLookupRecord>>>()
        .saturating_add(
            values
                .capacity()
                .saturating_mul(mem::size_of::<(String, Vec<DictionaryLookupRecord>)>()),
        )
        .saturating_add(
            values
                .iter()
                .map(|(text, records)| {
                    text.capacity()
                        .saturating_add(
                            records
                                .capacity()
                                .saturating_mul(mem::size_of::<DictionaryLookupRecord>()),
                        )
                        .saturating_add(
                            records
                                .iter()
                                .map(|record| {
                                    record
                                        .code
                                        .capacity()
                                        .saturating_add(
                                            record
                                                .fields
                                                .capacity()
                                                .saturating_mul(mem::size_of::<String>()),
                                        )
                                        .saturating_add(
                                            record
                                                .fields
                                                .iter()
                                                .map(String::capacity)
                                                .sum::<usize>(),
                                        )
                                })
                                .sum::<usize>(),
                        )
                })
                .sum::<usize>(),
        )
}

fn estimate_correction_tolerance_bytes(
    corrections: &[RimeCorrectionEntry],
    tolerance_rules: &[RimeToleranceRule],
) -> usize {
    mem::size_of_val(corrections)
        .saturating_add(
            corrections
                .iter()
                .map(|entry| {
                    entry
                        .observed_input
                        .capacity()
                        .saturating_add(entry.canonical_code.capacity())
                })
                .sum::<usize>(),
        )
        .saturating_add(mem::size_of_val(tolerance_rules))
        .saturating_add(
            tolerance_rules
                .iter()
                .map(|rule| {
                    rule.near_code
                        .capacity()
                        .saturating_add(
                            rule.candidate_codes
                                .capacity()
                                .saturating_mul(mem::size_of::<String>()),
                        )
                        .saturating_add(
                            rule.candidate_codes
                                .iter()
                                .map(String::capacity)
                                .sum::<usize>(),
                        )
                })
                .sum::<usize>(),
        )
}

fn estimate_preset_vocabulary_bytes(values: &[super::PresetVocabularyEntry]) -> usize {
    mem::size_of_val(values).saturating_add(
        values
            .iter()
            .map(|entry| entry.text.capacity())
            .sum::<usize>(),
    )
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

enum MarisaEntryCursorSpec {
    EntryList {
        entries_offset: usize,
        entry_count: usize,
    },
    Tail {
        tail_offset: usize,
        tail_count: usize,
        extra_ids: Vec<usize>,
    },
}

enum MarisaActiveEntryCursor {
    EntryList {
        entries_offset: usize,
        entry_count: usize,
        cursor: usize,
    },
    Tail {
        tail_offset: usize,
        tail_count: usize,
        cursor: usize,
        extra_ids: Vec<usize>,
    },
}

impl From<MarisaEntryCursorSpec> for MarisaActiveEntryCursor {
    fn from(spec: MarisaEntryCursorSpec) -> Self {
        match spec {
            MarisaEntryCursorSpec::EntryList {
                entries_offset,
                entry_count,
            } => Self::EntryList {
                entries_offset,
                entry_count,
                cursor: 0,
            },
            MarisaEntryCursorSpec::Tail {
                tail_offset,
                tail_count,
                extra_ids,
            } => Self::Tail {
                tail_offset,
                tail_count,
                cursor: 0,
                extra_ids,
            },
        }
    }
}

impl MarisaActiveEntryCursor {
    fn next_candidate<'a>(
        &mut self,
        string_table: &'a dyn CompactMarisaStringTable,
        bytes: &'a [u8],
        code: &'a str,
    ) -> Option<LookupCandidate<'a>> {
        match self {
            Self::EntryList {
                entries_offset,
                entry_count,
                cursor,
            } => {
                if *cursor >= *entry_count {
                    return None;
                }
                let entry_offset = entries_offset.checked_add(*cursor * 8)?;
                *cursor += 1;
                let text_id = read_u32_le(bytes, entry_offset).ok()?;
                let weight = read_f32_le(bytes, entry_offset + 4).ok()?;
                let text = string_table.get(text_id)?;
                Some(LookupCandidate::new(
                    text,
                    code,
                    weight,
                    CandidateSource::Table,
                ))
            }
            Self::Tail {
                tail_offset,
                tail_count,
                cursor,
                extra_ids,
            } => loop {
                if *cursor >= *tail_count {
                    return None;
                }
                let entry_offset = tail_offset.checked_add(4 + *cursor * 16)?;
                *cursor += 1;
                let extra = read_marisa_tail_extra_ids(bytes, entry_offset)?;
                if extra != *extra_ids {
                    continue;
                }
                let text_id = read_u32_le(bytes, entry_offset + 8).ok()?;
                let weight = read_f32_le(bytes, entry_offset + 12).ok()?;
                let text = string_table.get(text_id)?;
                return Some(LookupCandidate::new(
                    text,
                    code,
                    weight,
                    CandidateSource::Table,
                ));
            },
        }
    }
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
    Marisa {
        string_table: &'a dyn CompactMarisaStringTable,
        bytes: &'a [u8],
        code: &'a str,
        cursors: std::vec::IntoIter<MarisaEntryCursorSpec>,
        current: Option<MarisaActiveEntryCursor>,
    },
}

impl<'a> Iterator for CompactExactCandidates<'a> {
    type Item = LookupCandidate<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            CompactExactCandidatesInner::Empty => None,
            CompactExactCandidatesInner::Owned { code, inner } => inner.next().map(|entry| {
                LookupCandidate::new(&entry.text, *code, entry.weight, CandidateSource::Table)
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
            CompactExactCandidatesInner::Marisa {
                string_table,
                bytes,
                code,
                cursors,
                current,
            } => loop {
                if let Some(cursor) = current {
                    if let Some(candidate) = cursor.next_candidate(*string_table, bytes, code) {
                        return Some(candidate);
                    }
                    *current = None;
                }
                *current = Some(cursors.next()?.into());
            },
        }
    }
}

pub(crate) struct CompactPrefixCandidates<'a> {
    prefix: &'a str,
    store: &'a CompactTableStore,
    group_index: usize,
    current: CompactPrefixCurrent<'a>,
    done: bool,
    marisa_stack: Vec<MarisaTraversalFrame>,
    marisa_pending: Vec<MarisaPendingCandidate>,
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

struct MarisaPendingCandidate {
    code: String,
    text: String,
    weight: f32,
}

enum MarisaTraversalFrame {
    Node {
        ids: Vec<usize>,
        code: String,
        node_offset: usize,
        level: MarisaNodeLevel,
    },
    Entries {
        code: String,
        entries_offset: usize,
        entry_count: usize,
        cursor: usize,
    },
    Tail {
        ids: Vec<usize>,
        code: String,
        tail_offset: usize,
        tail_count: usize,
        cursor: usize,
    },
}

#[derive(Clone, Copy)]
enum MarisaNodeLevel {
    Head,
    Trunk,
}

impl<'a> Iterator for CompactPrefixCandidates<'a> {
    type Item = LookupCandidateEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if matches!(self.store.storage, CompactTableStorage::MarisaBacked { .. }) {
                return self.next_marisa_prefix_candidate();
            }
            match &mut self.current {
                CompactPrefixCurrent::None => {}
                CompactPrefixCurrent::Owned { code, entries } => {
                    if let Some(entry) = entries.next() {
                        return Some(LookupCandidateEntry::new(
                            *code,
                            LookupCandidate::new(
                                &entry.text,
                                *code,
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
                CompactTableStorage::MarisaBacked { .. } => unreachable!(),
            }
        }
    }
}

impl<'a> CompactPrefixCandidates<'a> {
    fn next_marisa_prefix_candidate(&mut self) -> Option<LookupCandidateEntry<'a>> {
        let CompactTableStorage::MarisaBacked {
            string_table,
            source,
            ..
        } = &self.store.storage
        else {
            return None;
        };
        loop {
            if let Some(candidate) = self.marisa_pending.pop() {
                let raw_comment = candidate.code.clone();
                return Some(LookupCandidateEntry::new(
                    candidate.code,
                    LookupCandidate::new(
                        candidate.text,
                        raw_comment,
                        candidate.weight,
                        CandidateSource::Table,
                    ),
                ));
            }
            let frame = self.marisa_stack.pop()?;
            match frame {
                MarisaTraversalFrame::Node {
                    ids,
                    code,
                    node_offset,
                    level,
                } => {
                    if !marisa_code_prefix_compatible(&code, self.prefix) {
                        continue;
                    }
                    push_marisa_child_frames(
                        &mut self.marisa_stack,
                        MarisaChildFrameInput {
                            bytes: source.bytes(),
                            syllabary_codes: &self.store.syllabary_codes,
                            ids: &ids,
                            code: &code,
                            node_offset,
                            level,
                            prefix: self.prefix,
                        },
                    );
                    if code.starts_with(self.prefix) {
                        push_marisa_node_entry_frame(
                            &mut self.marisa_stack,
                            source.bytes(),
                            code,
                            node_offset,
                            level,
                        );
                    }
                }
                MarisaTraversalFrame::Entries {
                    code,
                    entries_offset,
                    entry_count,
                    mut cursor,
                } => {
                    if cursor >= entry_count {
                        continue;
                    }
                    let Some(entry_offset) = entries_offset.checked_add(cursor * 8) else {
                        continue;
                    };
                    cursor += 1;
                    if cursor < entry_count {
                        self.marisa_stack.push(MarisaTraversalFrame::Entries {
                            code: code.clone(),
                            entries_offset,
                            entry_count,
                            cursor,
                        });
                    }
                    let Some(text_id) = read_u32_le(source.bytes(), entry_offset).ok() else {
                        continue;
                    };
                    let Some(weight) = read_f32_le(source.bytes(), entry_offset + 4).ok() else {
                        continue;
                    };
                    let Some(text) = string_table.get(text_id) else {
                        continue;
                    };
                    let raw_comment = code.clone();
                    return Some(LookupCandidateEntry::new(
                        code,
                        LookupCandidate::new(text, raw_comment, weight, CandidateSource::Table),
                    ));
                }
                MarisaTraversalFrame::Tail {
                    ids,
                    code,
                    tail_offset,
                    tail_count,
                    mut cursor,
                } => {
                    while cursor < tail_count {
                        let Some(entry_offset) = tail_offset.checked_add(4 + cursor * 16) else {
                            break;
                        };
                        cursor += 1;
                        let Some(extra_ids) =
                            read_marisa_tail_extra_ids(source.bytes(), entry_offset)
                        else {
                            continue;
                        };
                        let Some(full_code) = marisa_code_string_with_extra_ids(
                            &code,
                            &self.store.syllabary_codes,
                            &extra_ids,
                        ) else {
                            continue;
                        };
                        if !full_code.starts_with(self.prefix) {
                            continue;
                        }
                        let Some(text_id) = read_u32_le(source.bytes(), entry_offset + 8).ok()
                        else {
                            continue;
                        };
                        let Some(weight) = read_f32_le(source.bytes(), entry_offset + 12).ok()
                        else {
                            continue;
                        };
                        let Some(text) = string_table.get(text_id) else {
                            continue;
                        };
                        let raw_comment = full_code.clone();
                        if cursor < tail_count {
                            self.marisa_stack.push(MarisaTraversalFrame::Tail {
                                ids,
                                code,
                                tail_offset,
                                tail_count,
                                cursor,
                            });
                        }
                        return Some(LookupCandidateEntry::new(
                            full_code,
                            LookupCandidate::new(text, raw_comment, weight, CandidateSource::Table),
                        ));
                    }
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
    Marisa {
        syllabary_codes: &'a PackedSyllabaryCodes,
        bytes: &'a [u8],
        stack: Vec<MarisaTraversalFrame>,
        pending: Vec<String>,
    },
}

impl<'a> Iterator for CompactAllCodes<'a> {
    type Item = Cow<'a, str>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            CompactAllCodesInner::Owned(inner) => {
                inner.next().map(|group| Cow::Borrowed(group.code.as_str()))
            }
            CompactAllCodesInner::ByteBacked { bytes, inner } => inner
                .next()
                .map(|group| Cow::Borrowed(group.code.as_str(bytes))),
            CompactAllCodesInner::Marisa {
                syllabary_codes,
                bytes,
                stack,
                pending,
            } => loop {
                if let Some(code) = pending.pop() {
                    return Some(Cow::Owned(code));
                }
                let frame = stack.pop()?;
                match frame {
                    MarisaTraversalFrame::Node {
                        ids,
                        code,
                        node_offset,
                        level,
                    } => {
                        if marisa_node_has_entries(bytes, node_offset, level) {
                            pending.push(code.clone());
                        }
                        push_marisa_child_frames(
                            stack,
                            MarisaChildFrameInput {
                                bytes,
                                syllabary_codes,
                                ids: &ids,
                                code: &code,
                                node_offset,
                                level,
                                prefix: "",
                            },
                        );
                    }
                    MarisaTraversalFrame::Entries { .. } => {}
                    MarisaTraversalFrame::Tail {
                        ids,
                        code,
                        tail_offset,
                        tail_count,
                        mut cursor,
                    } => {
                        while cursor < tail_count {
                            let Some(entry_offset) = tail_offset.checked_add(4 + cursor * 16)
                            else {
                                break;
                            };
                            cursor += 1;
                            let Some(extra_ids) = read_marisa_tail_extra_ids(bytes, entry_offset)
                            else {
                                continue;
                            };
                            if let Some(full_code) = marisa_code_string_with_extra_ids(
                                &code,
                                syllabary_codes,
                                &extra_ids,
                            ) {
                                if cursor < tail_count {
                                    stack.push(MarisaTraversalFrame::Tail {
                                        ids,
                                        code,
                                        tail_offset,
                                        tail_count,
                                        cursor,
                                    });
                                }
                                return Some(Cow::Owned(full_code));
                            }
                        }
                    }
                }
            },
        }
    }
}

impl TableLookup for CompactTableStore {
    type ExactCandidates<'a> = CompactExactCandidates<'a>;
    type PrefixCandidates<'a> = CompactPrefixCandidates<'a>;
    type AllCodes<'a> = CompactAllCodes<'a>;

    fn has_code(&self, code: &str) -> bool {
        if matches!(self.storage, CompactTableStorage::MarisaBacked { .. }) {
            return self
                .marisa_exact_entry_cursors(code)
                .is_some_and(|(_, bytes, cursors)| {
                    cursors.into_iter().any(|cursor| match cursor {
                        MarisaEntryCursorSpec::EntryList { entry_count, .. } => entry_count > 0,
                        MarisaEntryCursorSpec::Tail {
                            tail_offset,
                            tail_count,
                            ref extra_ids,
                        } => marisa_tail_has_extra_code(bytes, tail_offset, tail_count, extra_ids),
                    })
                });
        }
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
        } else if let Some((string_table, bytes, cursors)) = self.marisa_exact_entry_cursors(code) {
            CompactExactCandidatesInner::Marisa {
                string_table,
                bytes,
                code,
                cursors: cursors.into_iter(),
                current: None,
            }
        } else {
            CompactExactCandidatesInner::Empty
        };
        CompactExactCandidates { inner }
    }

    fn prefix_candidates<'a>(&'a self, prefix: &'a str) -> Self::PrefixCandidates<'a> {
        if let CompactTableStorage::MarisaBacked {
            source,
            index_offset,
            ..
        } = &self.storage
        {
            return CompactPrefixCandidates {
                prefix,
                store: self,
                group_index: 0,
                current: CompactPrefixCurrent::None,
                done: false,
                marisa_stack: marisa_initial_prefix_frames(
                    source.bytes(),
                    *index_offset,
                    &self.syllabary_codes,
                    prefix,
                ),
                marisa_pending: Vec::new(),
            };
        }
        CompactPrefixCandidates {
            prefix,
            store: self,
            group_index: self.group_index(prefix).unwrap_or_else(|index| index),
            current: CompactPrefixCurrent::None,
            done: false,
            marisa_stack: Vec::new(),
            marisa_pending: Vec::new(),
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
            CompactTableStorage::MarisaBacked {
                source,
                index_offset,
                ..
            } => CompactAllCodesInner::Marisa {
                syllabary_codes: &self.syllabary_codes,
                bytes: source.bytes(),
                stack: marisa_initial_prefix_frames(
                    source.bytes(),
                    *index_offset,
                    &self.syllabary_codes,
                    "",
                ),
                pending: Vec::new(),
            },
        };
        CompactAllCodes { inner }
    }
}

pub fn parse_rime_table_bin_advanced_data(
    bytes: impl AsRef<[u8]>,
) -> Result<TableDictionaryAdvancedData, RimeTableBinParseError> {
    parse_rime_table_bin_advanced_data_with_options(
        bytes,
        RimeTableBinAdvancedDataOptions::default(),
    )
}

pub fn parse_rime_table_bin_advanced_data_with_options(
    bytes: impl AsRef<[u8]>,
    options: RimeTableBinAdvancedDataOptions,
) -> Result<TableDictionaryAdvancedData, RimeTableBinParseError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 68)?;
    let syllabary_offset =
        read_offset_ptr(bytes, 44)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let index_offset =
        read_offset_ptr(bytes, 48)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let mut advanced =
        read_yune_table_advanced_payload(bytes, total_index_end(bytes, index_offset)?, options)?;
    if !advanced.entries.is_empty() {
        return Err(RimeTableBinParseError::UnsupportedSection {
            role: "byte-backed advanced table entries".to_owned(),
        });
    }
    let table_metadata = read_table_metadata(bytes, syllabary_offset, advanced.present)?;
    advanced.data.sort_by_weight = table_metadata.sort_by_weight;
    Ok(advanced.data)
}

pub fn byte_backed_lookup_records_from_table_bin_bytes(
    bytes: impl Into<Arc<[u8]>>,
) -> Result<Option<ByteBackedDictionaryLookupRecords>, RimeTableBinParseError> {
    byte_backed_lookup_records_from_table_bin_byte_source(Arc::new(OwnedCompactTableBytes::new(
        bytes,
    )))
}

pub fn byte_backed_lookup_records_from_table_bin_byte_source(
    source: Arc<dyn CompactTableByteSource>,
) -> Result<Option<ByteBackedDictionaryLookupRecords>, RimeTableBinParseError> {
    let source: Arc<dyn DictionaryLookupByteSource> = Arc::new(CompactLookupByteSource { source });
    byte_backed_lookup_records_from_table_lookup_source(source)
}

fn byte_backed_lookup_records_from_table_lookup_source(
    source: Arc<dyn DictionaryLookupByteSource>,
) -> Result<Option<ByteBackedDictionaryLookupRecords>, RimeTableBinParseError> {
    let Some(payload_offset) = lookup_record_payload_offset_for_table_bin(source.bytes())? else {
        return Ok(None);
    };
    ByteBackedDictionaryLookupRecords::from_lookup_payload(source, payload_offset)
        .map(Some)
        .map_err(map_lookup_byte_store_error)
}

fn lookup_record_payload_offset_for_table_bin(
    bytes: &[u8],
) -> Result<Option<usize>, RimeTableBinParseError> {
    ensure_len(bytes, 68)?;
    let index_offset =
        read_offset_ptr(bytes, 48)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let advanced_offset = total_index_end(bytes, index_offset)?;
    let Some(marker_offset) = yune_advanced_marker_offset(bytes, advanced_offset)? else {
        return Ok(None);
    };

    let mut cursor = marker_offset
        .checked_add(YUNE_TABLE_ADVANCED_MARKER.len())
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let stem_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    for _ in 0..stem_count {
        cursor = skip_len_string(bytes, cursor)?;
        let value_count = read_count(bytes, cursor)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        for _ in 0..value_count {
            cursor = skip_len_string(bytes, cursor)?;
        }
    }

    let entry_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    for _ in 0..entry_count {
        cursor = skip_len_string(bytes, cursor)?;
        cursor = skip_len_string(bytes, cursor)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
    }

    let rule_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    for _ in 0..rule_count {
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        cursor = skip_len_string(bytes, cursor)?;
    }

    if cursor < bytes.len() {
        let (_, _, next) = read_correction_tolerance_payload(bytes, cursor)?;
        cursor = next;
    }

    if cursor >= bytes.len() {
        return Ok(None);
    }
    if bytes[cursor..].starts_with(b"YUNE-LOOKUP\0") {
        Ok(Some(cursor))
    } else {
        Err(RimeTableBinParseError::UnsupportedSection {
            role: "lookup record payload".to_owned(),
        })
    }
}

fn map_lookup_byte_store_error(error: DictionaryLookupByteStoreError) -> RimeTableBinParseError {
    match error {
        DictionaryLookupByteStoreError::UnsupportedSection => {
            RimeTableBinParseError::UnsupportedSection {
                role: "lookup record payload".to_owned(),
            }
        }
        DictionaryLookupByteStoreError::OutOfBounds => RimeTableBinParseError::OutOfBounds,
        DictionaryLookupByteStoreError::InvalidCount => RimeTableBinParseError::InvalidCount,
        DictionaryLookupByteStoreError::InvalidUtf8 => RimeTableBinParseError::InvalidUtf8,
    }
}

fn read_syllabary_refs(
    bytes: &[u8],
    offset: usize,
    expected_count: usize,
) -> Result<Vec<ByteStringRef>, RimeTableBinParseError> {
    let count = read_count(bytes, offset)?;
    if count != expected_count {
        return Err(RimeTableBinParseError::InvalidCount);
    }
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

fn read_marisa_syllabary_ids(
    bytes: &[u8],
    offset: usize,
    expected_count: usize,
) -> Result<Vec<u32>, RimeTableBinParseError> {
    let count = read_count(bytes, offset)?;
    if count != expected_count {
        return Err(RimeTableBinParseError::InvalidCount);
    }
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

    let mut ids = Vec::with_capacity(count);
    for index in 0..count {
        let field_offset = start
            .checked_add(
                index
                    .checked_mul(4)
                    .ok_or(RimeTableBinParseError::InvalidCount)?,
            )
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        ids.push(read_u32_le(bytes, field_offset).map_err(map_metadata_error)?);
    }
    Ok(ids)
}

fn validate_marisa_head_index(
    bytes: &[u8],
    offset: usize,
    syllable_count: usize,
) -> Result<(), RimeTableBinParseError> {
    let count = read_count(bytes, offset)?;
    if count > syllable_count {
        return Err(RimeTableBinParseError::InvalidCount);
    }
    let start = offset
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let total = count
        .checked_mul(12)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimeTableBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }
    Ok(())
}

fn segment_marisa_code_paths(
    code: &str,
    syllable_ids_by_code: &HashMap<String, usize>,
) -> Vec<Vec<usize>> {
    if code.is_empty() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    let mut current = Vec::new();
    collect_marisa_code_paths(code, 0, syllable_ids_by_code, &mut current, &mut paths);
    paths
}

fn collect_marisa_code_paths(
    code: &str,
    offset: usize,
    syllable_ids_by_code: &HashMap<String, usize>,
    current: &mut Vec<usize>,
    paths: &mut Vec<Vec<usize>>,
) {
    if offset == code.len() {
        paths.push(current.clone());
        return;
    }
    let Some(tail) = code.get(offset..) else {
        return;
    };
    let mut matches = syllable_ids_by_code
        .iter()
        .filter(|(syllable, _)| tail.starts_with(syllable.as_str()))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        right
            .0
            .len()
            .cmp(&left.0.len())
            .then_with(|| left.0.cmp(right.0))
    });
    for (syllable, syllable_id) in matches {
        current.push(*syllable_id);
        collect_marisa_code_paths(
            code,
            offset + syllable.len(),
            syllable_ids_by_code,
            current,
            paths,
        );
        current.pop();
    }
}

fn marisa_entry_cursor_for_syllable_ids(
    bytes: &[u8],
    index_offset: usize,
    syllable_ids: &[usize],
) -> Option<MarisaEntryCursorSpec> {
    let first = *syllable_ids.first()?;
    let head_count = read_count(bytes, index_offset).ok()?;
    if first >= head_count {
        return None;
    }
    let head_node = index_offset.checked_add(4 + first.checked_mul(12)?)?;
    if syllable_ids.len() == 1 {
        let (entries_offset, entry_count) = read_marisa_head_entry_list(bytes, head_node)?;
        return Some(MarisaEntryCursorSpec::EntryList {
            entries_offset,
            entry_count,
        });
    }
    let mut trunk_index = read_offset_ptr(bytes, head_node + 8).ok().flatten()?;
    for depth in 1..syllable_ids.len() {
        let trunk_node = find_marisa_trunk_node(bytes, trunk_index, syllable_ids[depth])?;
        if depth == syllable_ids.len() - 1 {
            let (entries_offset, entry_count) = read_marisa_trunk_entry_list(bytes, trunk_node)?;
            return Some(MarisaEntryCursorSpec::EntryList {
                entries_offset,
                entry_count,
            });
        }
        if depth == 2 {
            let tail_offset = read_offset_ptr(bytes, trunk_node + 12).ok().flatten()?;
            let tail_count = read_count(bytes, tail_offset).ok()?;
            return Some(MarisaEntryCursorSpec::Tail {
                tail_offset,
                tail_count,
                extra_ids: syllable_ids[3..].to_vec(),
            });
        }
        trunk_index = read_offset_ptr(bytes, trunk_node + 12).ok().flatten()?;
    }
    None
}

fn read_marisa_head_entry_list(bytes: &[u8], node_offset: usize) -> Option<(usize, usize)> {
    read_marisa_entry_list(bytes, node_offset, node_offset + 4)
}

fn read_marisa_trunk_entry_list(bytes: &[u8], node_offset: usize) -> Option<(usize, usize)> {
    read_marisa_entry_list(bytes, node_offset + 4, node_offset + 8)
}

fn read_marisa_entry_list(
    bytes: &[u8],
    count_offset: usize,
    pointer_offset: usize,
) -> Option<(usize, usize)> {
    let count = read_count(bytes, count_offset).ok()?;
    if count == 0 {
        return Some((0, 0));
    }
    let entries_offset = read_offset_ptr(bytes, pointer_offset).ok().flatten()?;
    let total = count.checked_mul(8)?.checked_add(entries_offset)?;
    (total <= bytes.len()).then_some((entries_offset, count))
}

fn find_marisa_trunk_node(bytes: &[u8], index_offset: usize, key: usize) -> Option<usize> {
    let key = i32::try_from(key).ok()?;
    let count = read_count(bytes, index_offset).ok()?;
    let start = index_offset.checked_add(4)?;
    let total = count.checked_mul(16)?.checked_add(start)?;
    if total > bytes.len() {
        return None;
    }
    let mut low = 0usize;
    let mut high = count;
    while low < high {
        let middle = low + (high - low) / 2;
        let node_offset = start.checked_add(middle.checked_mul(16)?)?;
        let node_key = read_i32_le(bytes, node_offset).ok()?;
        match node_key.cmp(&key) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Equal => return Some(node_offset),
            std::cmp::Ordering::Greater => high = middle,
        }
    }
    None
}

fn marisa_initial_prefix_frames(
    bytes: &[u8],
    index_offset: usize,
    syllabary_codes: &PackedSyllabaryCodes,
    prefix: &str,
) -> Vec<MarisaTraversalFrame> {
    let Ok(head_count) = read_count(bytes, index_offset) else {
        return Vec::new();
    };
    let mut frames = Vec::new();
    for syllable_id in (0..head_count).rev() {
        let Some(code) = syllabary_codes.get(syllable_id) else {
            continue;
        };
        if !marisa_code_prefix_compatible(code, prefix) {
            continue;
        }
        let Some(node_offset) = index_offset.checked_add(4 + syllable_id.saturating_mul(12)) else {
            continue;
        };
        frames.push(MarisaTraversalFrame::Node {
            ids: vec![syllable_id],
            code: code.to_owned(),
            node_offset,
            level: MarisaNodeLevel::Head,
        });
    }
    frames
}

fn marisa_code_prefix_compatible(code: &str, prefix: &str) -> bool {
    code.starts_with(prefix) || prefix.starts_with(code)
}

fn marisa_code_string_with_extra_ids(
    base: &str,
    syllabary_codes: &PackedSyllabaryCodes,
    extra_ids: &[usize],
) -> Option<String> {
    let mut code = String::from(base);
    for id in extra_ids {
        code.push_str(syllabary_codes.get(*id)?);
    }
    Some(code)
}

fn marisa_node_has_entries(bytes: &[u8], node_offset: usize, level: MarisaNodeLevel) -> bool {
    let entry_count_offset = match level {
        MarisaNodeLevel::Head => node_offset,
        MarisaNodeLevel::Trunk => node_offset + 4,
    };
    read_count(bytes, entry_count_offset).is_ok_and(|count| count > 0)
}

fn push_marisa_node_entry_frame(
    stack: &mut Vec<MarisaTraversalFrame>,
    bytes: &[u8],
    code: String,
    node_offset: usize,
    level: MarisaNodeLevel,
) {
    let entry_list = match level {
        MarisaNodeLevel::Head => read_marisa_head_entry_list(bytes, node_offset),
        MarisaNodeLevel::Trunk => read_marisa_trunk_entry_list(bytes, node_offset),
    };
    let Some((entries_offset, entry_count)) = entry_list else {
        return;
    };
    if entry_count == 0 {
        return;
    }
    stack.push(MarisaTraversalFrame::Entries {
        code,
        entries_offset,
        entry_count,
        cursor: 0,
    });
}

struct MarisaChildFrameInput<'a> {
    bytes: &'a [u8],
    syllabary_codes: &'a PackedSyllabaryCodes,
    ids: &'a [usize],
    code: &'a str,
    node_offset: usize,
    level: MarisaNodeLevel,
    prefix: &'a str,
}

fn push_marisa_child_frames(
    stack: &mut Vec<MarisaTraversalFrame>,
    input: MarisaChildFrameInput<'_>,
) {
    let next_level_offset = match input.level {
        MarisaNodeLevel::Head => input.node_offset + 8,
        MarisaNodeLevel::Trunk => input.node_offset + 12,
    };
    let Some(next_index) = read_offset_ptr(input.bytes, next_level_offset)
        .ok()
        .flatten()
    else {
        return;
    };
    if input.ids.len() >= 3 {
        let Ok(tail_count) = read_count(input.bytes, next_index) else {
            return;
        };
        if marisa_code_prefix_compatible(input.code, input.prefix) {
            stack.push(MarisaTraversalFrame::Tail {
                ids: input.ids.to_vec(),
                code: input.code.to_owned(),
                tail_offset: next_index,
                tail_count,
                cursor: 0,
            });
        }
        return;
    }

    let Ok(child_count) = read_count(input.bytes, next_index) else {
        return;
    };
    let Some(start) = next_index.checked_add(4) else {
        return;
    };
    for index in (0..child_count).rev() {
        let Some(child_offset) = start.checked_add(index.saturating_mul(16)) else {
            continue;
        };
        let Some(key) = read_i32_le(input.bytes, child_offset)
            .ok()
            .and_then(|key| usize::try_from(key).ok())
        else {
            continue;
        };
        let Some(child_code_part) = input.syllabary_codes.get(key) else {
            continue;
        };
        let mut child_code = String::with_capacity(input.code.len() + child_code_part.len());
        child_code.push_str(input.code);
        child_code.push_str(child_code_part);
        if !marisa_code_prefix_compatible(&child_code, input.prefix) {
            continue;
        }
        let mut child_ids = input.ids.to_vec();
        child_ids.push(key);
        stack.push(MarisaTraversalFrame::Node {
            ids: child_ids,
            code: child_code,
            node_offset: child_offset,
            level: MarisaNodeLevel::Trunk,
        });
    }
}

fn read_marisa_tail_extra_ids(bytes: &[u8], long_entry_offset: usize) -> Option<Vec<usize>> {
    let count = read_count(bytes, long_entry_offset).ok()?;
    let values_offset = read_offset_ptr(bytes, long_entry_offset + 4)
        .ok()
        .flatten()?;
    let total = count.checked_mul(4)?.checked_add(values_offset)?;
    if total > bytes.len() {
        return None;
    }
    let mut ids = Vec::with_capacity(count);
    for index in 0..count {
        let offset = values_offset.checked_add(index.checked_mul(4)?)?;
        let id = read_i32_le(bytes, offset).ok()?;
        ids.push(usize::try_from(id).ok()?);
    }
    Some(ids)
}

fn marisa_tail_has_extra_code(
    bytes: &[u8],
    tail_offset: usize,
    tail_count: usize,
    extra_ids: &[usize],
) -> bool {
    (0..tail_count).any(|index| {
        let Some(entry_offset) = tail_offset.checked_add(4 + index * 16) else {
            return false;
        };
        read_marisa_tail_extra_ids(bytes, entry_offset).is_some_and(|extra| extra == extra_ids)
    })
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
    let mut advanced = read_yune_table_advanced_payload(
        bytes,
        total_index_end(bytes, index_offset)?,
        RimeTableBinAdvancedDataOptions::default(),
    )?;
    let table_metadata = read_table_metadata(bytes, syllabary_offset, advanced.present)?;
    advanced.data.sort_by_weight = table_metadata.sort_by_weight;
    entries.extend(advanced.entries);
    let mut dictionary = TableDictionary::with_advanced_data(entries, advanced.data);
    dictionary.set_entry_weight_domain(table_metadata.entry_weight_domain);
    Ok(dictionary)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParsedTableMetadata {
    sort_by_weight: Option<bool>,
    entry_weight_domain: TableEntryWeightDomain,
}

fn read_table_metadata(
    bytes: &[u8],
    syllabary_offset: usize,
    yune_payload_present: bool,
) -> Result<ParsedTableMetadata, RimeTableBinParseError> {
    if let Some(metadata) = read_declared_table_metadata(bytes, syllabary_offset)? {
        return Ok(metadata);
    }
    Ok(ParsedTableMetadata {
        sort_by_weight: None,
        entry_weight_domain: if yune_payload_present {
            TableEntryWeightDomain::Raw
        } else {
            TableEntryWeightDomain::NaturalLog
        },
    })
}

fn read_declared_table_metadata(
    bytes: &[u8],
    syllabary_offset: usize,
) -> Result<Option<ParsedTableMetadata>, RimeTableBinParseError> {
    if syllabary_offset < RIME_TABLE_HEADER_LEN || syllabary_offset > bytes.len() {
        return Err(RimeTableBinParseError::OutOfBounds);
    }
    let gap = &bytes[RIME_TABLE_HEADER_LEN..syllabary_offset];
    if gap.is_empty() {
        return Ok(None);
    }
    if !gap.starts_with(YUNE_TABLE_METADATA_MARKER) {
        if gap.starts_with(YUNE_TABLE_METADATA_PREFIX) {
            return Err(RimeTableBinParseError::UnsupportedSection {
                role: "malformed Yune table metadata marker".to_owned(),
            });
        }
        // External `Rime::Table/4.0` producers may use this offset-followed gap
        // for their own padding. Its absence from Yune's namespace is legacy
        // `by_weight`, not a reason to reject an otherwise valid table.
        return Ok(None);
    }

    let mut cursor = YUNE_TABLE_METADATA_MARKER.len();
    let version = read_u32_le(gap, cursor).map_err(map_metadata_error)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let payload_len = usize::try_from(read_u32_le(gap, cursor).map_err(map_metadata_error)?)
        .map_err(|_| RimeTableBinParseError::InvalidLength)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    if payload_len != YUNE_TABLE_METADATA_PAYLOAD_LEN {
        return Err(RimeTableBinParseError::InvalidLength);
    }
    let payload_end = cursor
        .checked_add(payload_len)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    if payload_end != gap.len() {
        return Err(RimeTableBinParseError::InvalidLength);
    }
    let payload = gap
        .get(cursor..payload_end)
        .ok_or(RimeTableBinParseError::InvalidLength)?;
    match version {
        YUNE_TABLE_METADATA_V1 => {
            if payload != [YUNE_TABLE_SORT_ORIGINAL, 0, 0, 0] {
                return Err(RimeTableBinParseError::UnsupportedSection {
                    role: "invalid Yune table metadata v1 payload".to_owned(),
                });
            }
            Ok(Some(ParsedTableMetadata {
                sort_by_weight: Some(false),
                entry_weight_domain: TableEntryWeightDomain::Raw,
            }))
        }
        YUNE_TABLE_METADATA_VERSION => {
            if payload[2..] != [0, 0] {
                return Err(RimeTableBinParseError::UnsupportedSection {
                    role: "nonzero Yune table metadata reserved bytes".to_owned(),
                });
            }
            if payload[1] != YUNE_TABLE_WEIGHT_NATURAL_LOG {
                return Err(RimeTableBinParseError::UnsupportedSection {
                    role: format!("Yune table weight domain {}", payload[1]),
                });
            }
            let sort_by_weight = match payload[0] {
                YUNE_TABLE_SORT_BY_WEIGHT => true,
                YUNE_TABLE_SORT_ORIGINAL => false,
                policy => {
                    return Err(RimeTableBinParseError::UnsupportedSection {
                        role: format!("Yune table sort policy {policy}"),
                    });
                }
            };
            Ok(Some(ParsedTableMetadata {
                sort_by_weight: Some(sort_by_weight),
                entry_weight_domain: TableEntryWeightDomain::NaturalLog,
            }))
        }
        _ => Err(RimeTableBinParseError::UnsupportedSection {
            role: format!("Yune table metadata version {version}"),
        }),
    }
}

/// Byte length needed to inspect the optional Yune metadata gap without
/// reading a table's dictionary payload.
pub fn rime_table_bin_metadata_probe_len(
    header: impl AsRef<[u8]>,
) -> Result<usize, RimeTableBinParseError> {
    let header = header.as_ref();
    ensure_len(header, RIME_TABLE_HEADER_LEN)?;
    let raw = read_i32_le(header, 44).map_err(map_metadata_error)?;
    if raw == 0 {
        return Err(RimeTableBinParseError::MissingRequiredSection);
    }
    44usize
        .checked_add_signed(raw as isize)
        .ok_or(RimeTableBinParseError::OutOfBounds)
}

/// Classifies a recognized Yune metadata gap from a header-sized prefix.
/// `None` means the table has no declared Yune weight domain and needs the
/// legacy payload classifier (or a conservative source-backed rebuild).
pub fn rime_table_bin_declared_weight_upgrade(
    prefix: impl AsRef<[u8]>,
) -> Result<Option<bool>, RimeTableBinParseError> {
    let prefix = prefix.as_ref();
    let syllabary_offset = rime_table_bin_metadata_probe_len(prefix)?;
    Ok(read_declared_table_metadata(prefix, syllabary_offset)?
        .map(|metadata| metadata.entry_weight_domain == TableEntryWeightDomain::Raw))
}

/// Whether an identifiable table was emitted by Yune's pre-v2 raw-weight
/// writer and should be rebuilt when its source dictionary is available.
pub fn rime_table_bin_requires_weight_upgrade(
    bytes: impl AsRef<[u8]>,
) -> Result<bool, RimeTableBinParseError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, RIME_TABLE_HEADER_LEN)?;
    let syllabary_offset =
        read_offset_ptr(bytes, 44)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    if let Some(metadata) = read_declared_table_metadata(bytes, syllabary_offset)? {
        return Ok(metadata.entry_weight_domain == TableEntryWeightDomain::Raw);
    }
    let index_offset =
        read_offset_ptr(bytes, 48)?.ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let advanced = read_yune_table_advanced_payload(
        bytes,
        total_index_end(bytes, index_offset)?,
        RimeTableBinAdvancedDataOptions {
            load_lookup_records: false,
            byte_back_lookup_records: false,
        },
    )?;
    Ok(
        read_table_metadata(bytes, syllabary_offset, advanced.present)?.entry_weight_domain
            == TableEntryWeightDomain::Raw,
    )
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
    present: bool,
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
    options: RimeTableBinAdvancedDataOptions,
) -> Result<AdvancedTablePayload, RimeTableBinParseError> {
    let marker = YUNE_TABLE_ADVANCED_MARKER;
    let Some(marker_offset) = yune_advanced_marker_offset(bytes, offset)? else {
        return Ok(AdvancedTablePayload {
            entries: Vec::new(),
            data: TableDictionaryAdvancedData::default(),
            present: false,
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
        if options.load_lookup_records {
            if options.byte_back_lookup_records {
                skip_lookup_record_payload(bytes, cursor)?;
                HashMap::new()
            } else {
                read_lookup_record_payload(bytes, cursor)?
            }
        } else {
            skip_lookup_record_payload(bytes, cursor)?;
            HashMap::new()
        }
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
            byte_backed_lookup_records: None,
            ..TableDictionaryAdvancedData::default()
        },
        present: true,
    })
}

fn yune_advanced_marker_offset(
    bytes: &[u8],
    offset: usize,
) -> Result<Option<usize>, RimeTableBinParseError> {
    let Some(tail) = bytes.get(offset..) else {
        return Ok(None);
    };
    let marker_position = tail
        .windows(YUNE_TABLE_ADVANCED_MARKER.len())
        .rposition(|window| window == YUNE_TABLE_ADVANCED_MARKER);
    let Some(marker_position) = marker_position else {
        return Ok(None);
    };
    let marker_offset = offset
        .checked_add(marker_position)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;

    match validate_yune_advanced_payload(bytes, marker_offset) {
        Ok(()) => Ok(Some(marker_offset)),
        Err(error) if table_claims_yune_metadata(bytes) => Err(error),
        // A standard table may contain the marker bytes in candidate text.
        // Without Yune header metadata, only a complete, tail-anchored payload
        // establishes legacy Yune provenance.
        Err(_) => Ok(None),
    }
}

fn table_claims_yune_metadata(bytes: &[u8]) -> bool {
    let Ok(Some(syllabary_offset)) = read_offset_ptr(bytes, 44) else {
        return false;
    };
    bytes
        .get(RIME_TABLE_HEADER_LEN..syllabary_offset)
        .is_some_and(|gap| gap.starts_with(YUNE_TABLE_METADATA_PREFIX))
}

fn validate_yune_advanced_payload(
    bytes: &[u8],
    marker_offset: usize,
) -> Result<(), RimeTableBinParseError> {
    let mut cursor = marker_offset
        .checked_add(YUNE_TABLE_ADVANCED_MARKER.len())
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    let stem_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    for _ in 0..stem_count {
        cursor = skip_len_string(bytes, cursor)?;
        let value_count = read_count(bytes, cursor)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        for _ in 0..value_count {
            cursor = skip_len_string(bytes, cursor)?;
        }
    }

    let entry_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    for _ in 0..entry_count {
        cursor = skip_len_string(bytes, cursor)?;
        cursor = skip_len_string(bytes, cursor)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        if cursor > bytes.len() {
            return Err(RimeTableBinParseError::OutOfBounds);
        }
    }

    let rule_count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeTableBinParseError::OutOfBounds)?;
    for _ in 0..rule_count {
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        if cursor > bytes.len() {
            return Err(RimeTableBinParseError::OutOfBounds);
        }
        cursor = skip_len_string(bytes, cursor)?;
    }

    if cursor == bytes.len() {
        return Ok(());
    }
    let (_, _, next_cursor) = read_correction_tolerance_payload(bytes, cursor)?;
    if next_cursor >= bytes.len() {
        return Err(RimeTableBinParseError::InvalidLength);
    }
    skip_lookup_record_payload(bytes, next_cursor)
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
    crate::memory_probe_mark(format!(
        "m47:compact_table:after_lookup_record_payload_parse:lookup_texts={}:lookup_records={}",
        lookup_records.len(),
        lookup_records.values().map(Vec::len).sum::<usize>()
    ));
    Ok(lookup_records)
}

fn skip_lookup_record_payload(
    bytes: &[u8],
    mut cursor: usize,
) -> Result<(), RimeTableBinParseError> {
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

    let mut record_total = 0usize;
    for _ in 0..text_count {
        cursor = skip_len_string(bytes, cursor)?;
        let record_count = read_count(bytes, cursor)?;
        if record_count > MAX_LOOKUP_RECORD_COUNT {
            return Err(RimeTableBinParseError::InvalidCount);
        }
        record_total = record_total
            .checked_add(record_count)
            .ok_or(RimeTableBinParseError::InvalidCount)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;

        for _ in 0..record_count {
            cursor = skip_len_string(bytes, cursor)?;
            let field_count = read_count(bytes, cursor)?;
            if field_count > MAX_LOOKUP_FIELD_COUNT {
                return Err(RimeTableBinParseError::InvalidCount);
            }
            cursor = cursor
                .checked_add(4)
                .ok_or(RimeTableBinParseError::OutOfBounds)?;

            for _ in 0..field_count {
                cursor = skip_len_string(bytes, cursor)?;
            }
        }
    }
    if cursor != bytes.len() {
        return Err(RimeTableBinParseError::UnsupportedSection {
            role: "trailing table payload".to_owned(),
        });
    }
    crate::memory_probe_mark(format!(
        "m47:compact_table:after_lookup_record_payload_skip:lookup_texts={text_count}:lookup_records={record_total}"
    ));
    Ok(())
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

fn skip_len_string(bytes: &[u8], offset: usize) -> Result<usize, RimeTableBinParseError> {
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
    std::str::from_utf8(&bytes[start..end]).map_err(|_| RimeTableBinParseError::InvalidUtf8)?;
    Ok(end)
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

#[cfg(test)]
mod marisa_key_enumeration_tests {
    use super::{CompactMarisaStringTable, RimeTableBinParseError, SafeReadMarisaStringTable};

    fn serialized_trie(keys: &[&str]) -> Vec<u8> {
        let mut keyset = rsmarisa::Keyset::new();
        for key in keys {
            keyset
                .push_back_str(key)
                .expect("test key should be accepted");
        }
        let mut trie = rsmarisa::Trie::new();
        trie.build(&mut keyset, 0);
        let mut writer = rsmarisa::grimoire::io::Writer::from_vec(Vec::new());
        trie.write(&mut writer).expect("test trie should serialize");
        writer.into_inner().expect("serialized bytes should return")
    }

    #[test]
    fn predictive_enumeration_returns_complete_marisa_id_order() {
        let bytes = serialized_trie(&["zeta", "alpha", "middle", "å»£"]);
        let table = SafeReadMarisaStringTable::from_bytes(&bytes, 0, bytes.len())
            .expect("serialized trie should parse");
        let requested = [2, 0, 2, 1];
        let enumerated = table
            .keys_for_ids(&requested)
            .expect("predictive enumeration should resolve requested ids");
        let reverse_lookup_order = requested
            .into_iter()
            .map(|id| table.get(id).expect("id should reverse-resolve"))
            .collect::<Vec<_>>();
        assert_eq!(enumerated, reverse_lookup_order);
        assert_eq!(
            table.keys_for_ids(&[table.num_keys() as u32]),
            Err(RimeTableBinParseError::InvalidCount)
        );
    }

    #[derive(Debug)]
    struct MissingIdTable;

    impl CompactMarisaStringTable for MissingIdTable {
        fn get(&self, id: u32) -> Option<String> {
            (id != 1).then(|| format!("key-{id}"))
        }

        fn num_keys(&self) -> usize {
            3
        }

        fn mapping_mode(&self) -> &'static str {
            "test-corrupt"
        }
    }

    #[test]
    fn default_enumeration_fails_closed_on_missing_id() {
        assert_eq!(
            MissingIdTable.keys_for_ids(&[0, 1, 2]),
            Err(RimeTableBinParseError::InvalidCount)
        );
    }
}

#[cfg(test)]
mod packed_syllabary_tests {
    use super::{
        checked_packed_offset, CompactTableStore, PackedSyllabaryCodes, RimeTableBinParseError,
    };
    use crate::{MemoryOwnerClass, TableDictionary, TableEntry};
    use std::mem;

    #[test]
    fn packed_syllabary_codes_preserve_order_boundaries_and_owner_accounting() {
        let source = ["", "a", "廣", "a", "é", "e\u{301}"];
        let packed = PackedSyllabaryCodes::try_from_codes(source).expect("fixture should pack");

        assert_eq!(packed.len(), source.len());
        assert_eq!(
            packed.iter().map(|code| code.as_str()).collect::<Vec<_>>(),
            source
        );
        assert_eq!(packed.get(0), Some(""));
        assert_eq!(packed.get(2), Some("廣"));
        assert_eq!(packed.get(4), Some("é"));
        assert_eq!(packed.get(5), Some("e\u{301}"));
        assert_eq!(packed.get(source.len()), None);
        assert_eq!(packed.to_vec(), source.map(str::to_owned));
        assert_eq!(packed.offsets.first().copied(), Some(0));
        assert!(
            packed
                .offsets
                .windows(2)
                .all(|window| window[0] <= window[1]),
            "empty codes may repeat an offset but offsets must never decrease"
        );
        assert_eq!(
            packed.offsets.last().copied(),
            Some(source.iter().map(|code| code.len() as u32).sum())
        );
        assert_eq!(
            packed.offsets.last().copied(),
            u32::try_from(packed.utf8.len()).ok()
        );
        assert_eq!(
            checked_packed_offset(u32::MAX, 1),
            Err(RimeTableBinParseError::InvalidLength)
        );
        if usize::BITS > u32::BITS {
            assert_eq!(
                checked_packed_offset(0, u32::MAX as usize + 1),
                Err(RimeTableBinParseError::InvalidLength)
            );
        }

        let store = CompactTableStore::from_dictionary(TableDictionary::new([
            TableEntry::new("", "empty", 1.0),
            TableEntry::new("a", "latin", 1.0),
            TableEntry::new("廣", "unicode", 1.0),
            TableEntry::new("a", "duplicate-code-row", 0.5),
            TableEntry::new("e\u{301}", "combining", 1.0),
        ]));
        let codes = store.syllabary_codes();
        assert_eq!(
            codes.iter().map(|code| code.as_str()).collect::<Vec<_>>(),
            vec!["", "a", "廣", "e\u{301}"]
        );
        let owner = store
            .memory_owner_rows()
            .into_iter()
            .find(|row| row.owner == "compact_table.syllabary_codes")
            .expect("packed syllabary owner row should exist");
        let expected_bytes = codes.utf8.len() + codes.offsets.len() * mem::size_of::<u32>();
        assert_eq!(owner.class, MemoryOwnerClass::HeapOwnedReducible);
        assert_eq!(owner.estimated_bytes, expected_bytes);
        assert_eq!(owner.item_count, codes.len());
        assert_eq!(owner.item_count, 4);
        assert_eq!(owner.storage, "Box<str> + Box<[u32]>");
    }
}
