use super::{CompactTableByteSource, RimeCorrectionEntry, RimeToleranceRule};
use crate::dictionary::compiled::{
    parse_rime_format_version_for_payload, read_f32_le, read_i32_le, read_u32_le,
};
use crate::dictionary::double_array::DartsDoubleArray;
use crate::{MemoryOwnerClass, MemoryOwnerRow};
use std::mem;
use std::sync::Arc;

const MAX_CORRECTION_COUNT: usize = 4096;
const MAX_TOLERANCE_RULE_COUNT: usize = 4096;
const MAX_TOLERANCE_CANDIDATE_COUNT: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct RimePrismBinPayload {
    pub dict_file_checksum: u32,
    pub schema_file_checksum: u32,
    pub num_syllables: u32,
    pub num_spellings: u32,
    pub double_array_size: u32,
    pub double_array: Option<DartsDoubleArray>,
    pub spelling_map: Vec<Vec<RimePrismSpellingDescriptor>>,
    pub corrections: Vec<RimeCorrectionEntry>,
    pub tolerance_rules: Vec<RimeToleranceRule>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RimePrismSpellingDescriptor {
    pub syllable_id: i32,
    pub spelling_type: i32,
    pub is_correction: bool,
    pub credibility: f32,
    pub tips: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrismLookupCode<'a> {
    pub code: &'a str,
    pub abbreviation: bool,
    pub correction: bool,
    pub credibility: f32,
}

impl RimePrismBinPayload {
    #[must_use]
    pub fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let descriptor_count = self.spelling_map.iter().map(Vec::len).sum::<usize>();
        let tip_count = self
            .spelling_map
            .iter()
            .flatten()
            .filter(|descriptor| !descriptor.tips.is_empty())
            .count();
        vec![
            MemoryOwnerRow::new(
                "prism.double_array_units",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_double_array_units_bytes(self.double_array.as_ref()),
                self.double_array
                    .as_ref()
                    .map_or(0, |double_array| double_array.units().len()),
                "DartsDoubleArray Vec<u32>",
                "parsed prism double-array units retained on heap for spelling lookup",
            ),
            MemoryOwnerRow::new(
                "prism.spelling_map",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_spelling_map_bytes(&self.spelling_map, self.spelling_map.capacity()),
                descriptor_count,
                "Vec<Vec<RimePrismSpellingDescriptor>>",
                "parsed prism spelling descriptor vectors retained on heap",
            ),
            MemoryOwnerRow::new(
                "prism.corrections_tolerance",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_correction_tolerance_bytes(
                    &self.corrections,
                    self.corrections.capacity(),
                    &self.tolerance_rules,
                    self.tolerance_rules.capacity(),
                ),
                self.corrections
                    .len()
                    .saturating_add(self.tolerance_rules.len()),
                "Vec<RimeCorrectionEntry> + Vec<RimeToleranceRule>",
                "parsed prism correction/tolerance payload retained on heap",
            ),
            MemoryOwnerRow::new(
                "prism.tips_payload",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_tips_payload_bytes(&self.spelling_map),
                tip_count,
                "String payloads in RimePrismSpellingDescriptor",
                "parsed prism descriptor tips string payload retained on heap when present",
            ),
        ]
    }

    #[must_use]
    pub fn lookup_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a [String],
    ) -> Vec<PrismLookupCode<'a>> {
        self.lookup_canonical_codes_with_limit(spelling, syllabary_codes, usize::MAX)
    }

    #[must_use]
    pub fn lookup_canonical_codes_with_limit<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a [String],
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        let Some(spelling_index) = self
            .double_array
            .as_ref()
            .and_then(|double_array| double_array.exact_match(spelling))
        else {
            return Vec::new();
        };
        self.spelling_map
            .get(spelling_index as usize)
            .into_iter()
            .flatten()
            .filter_map(|descriptor| {
                let syllable_index = usize::try_from(descriptor.syllable_id).ok()?;
                let code = syllabary_codes.get(syllable_index)?;
                Some(PrismLookupCode {
                    code,
                    abbreviation: descriptor.spelling_type == 2,
                    correction: descriptor.is_correction,
                    credibility: descriptor.credibility,
                })
            })
            .take(limit)
            .collect()
    }
}

#[derive(Debug)]
pub struct RimePrismRuntimePayload {
    storage: RimePrismRuntimeStorage,
}

#[derive(Debug)]
enum RimePrismRuntimeStorage {
    Owned(RimePrismBinPayload),
    ByteBacked(ByteBackedRimePrismPayload),
}

#[derive(Debug)]
struct ByteBackedRimePrismPayload {
    source: Arc<dyn CompactTableByteSource>,
    double_array: Option<ByteBackedPrismDoubleArray>,
    spelling_map: ByteBackedPrismSpellingMap,
    corrections: Vec<RimeCorrectionEntry>,
    tolerance_rules: Vec<RimeToleranceRule>,
}

#[derive(Clone, Copy, Debug)]
struct ByteBackedPrismDoubleArray {
    offset: usize,
    unit_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct ByteBackedPrismSpellingMap {
    offset: usize,
    spelling_count: usize,
    descriptor_count: usize,
    raw_descriptor_bytes: usize,
    tips_payload_bytes: usize,
    tip_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct RuntimePrismSpellingDescriptor {
    syllable_id: i32,
    spelling_type: i32,
    is_correction: bool,
    credibility: f32,
}

impl From<RimePrismBinPayload> for RimePrismRuntimePayload {
    fn from(payload: RimePrismBinPayload) -> Self {
        Self {
            storage: RimePrismRuntimeStorage::Owned(payload),
        }
    }
}

impl RimePrismRuntimePayload {
    #[must_use]
    pub fn corrections(&self) -> &[RimeCorrectionEntry] {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => &payload.corrections,
            RimePrismRuntimeStorage::ByteBacked(payload) => &payload.corrections,
        }
    }

    #[must_use]
    pub fn tolerance_rules(&self) -> &[RimeToleranceRule] {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => &payload.tolerance_rules,
            RimePrismRuntimeStorage::ByteBacked(payload) => &payload.tolerance_rules,
        }
    }

    #[must_use]
    pub fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => payload.memory_owner_rows(),
            RimePrismRuntimeStorage::ByteBacked(payload) => payload.memory_owner_rows(),
        }
    }

    #[must_use]
    pub fn lookup_canonical_codes<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a [String],
    ) -> Vec<PrismLookupCode<'a>> {
        self.lookup_canonical_codes_with_limit(spelling, syllabary_codes, usize::MAX)
    }

    #[must_use]
    pub fn lookup_canonical_codes_with_limit<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a [String],
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        match &self.storage {
            RimePrismRuntimeStorage::Owned(payload) => {
                payload.lookup_canonical_codes_with_limit(spelling, syllabary_codes, limit)
            }
            RimePrismRuntimeStorage::ByteBacked(payload) => {
                payload.lookup_canonical_codes_with_limit(spelling, syllabary_codes, limit)
            }
        }
    }
}

impl ByteBackedRimePrismPayload {
    fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let source_class = prism_byte_source_class(self.source.as_ref());
        let source_label = format!(
            "{}:{}",
            self.source.storage_label(),
            self.source.mapping_mode()
        );
        vec![
            MemoryOwnerRow::new(
                "prism.double_array_units",
                source_class,
                self.double_array
                    .map_or(0, ByteBackedPrismDoubleArray::byte_len),
                self.double_array
                    .map_or(0, |double_array| double_array.unit_count),
                source_label.clone(),
                "prism double-array units are read directly from the byte source",
            ),
            MemoryOwnerRow::new(
                "prism.spelling_map",
                source_class,
                self.spelling_map.byte_len(),
                self.spelling_map.descriptor_count,
                source_label.clone(),
                "prism spelling descriptors are read lazily from the byte source",
            ),
            MemoryOwnerRow::new(
                "prism.corrections_tolerance",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_correction_tolerance_bytes(
                    &self.corrections,
                    self.corrections.capacity(),
                    &self.tolerance_rules,
                    self.tolerance_rules.capacity(),
                ),
                self.corrections
                    .len()
                    .saturating_add(self.tolerance_rules.len()),
                "Vec<RimeCorrectionEntry> + Vec<RimeToleranceRule>",
                "parsed prism correction/tolerance payload retained on heap",
            ),
            MemoryOwnerRow::new(
                "prism.tips_payload",
                source_class,
                self.spelling_map.tips_payload_bytes,
                self.spelling_map.tip_count,
                source_label,
                "prism descriptor tips remain in the byte source",
            ),
        ]
    }

    fn lookup_canonical_codes_with_limit<'a>(
        &self,
        spelling: &str,
        syllabary_codes: &'a [String],
        limit: usize,
    ) -> Vec<PrismLookupCode<'a>> {
        let Some(spelling_index) = self
            .double_array
            .as_ref()
            .and_then(|double_array| double_array.exact_match(self.source.bytes(), spelling))
        else {
            return Vec::new();
        };
        let Ok(spelling_index) = usize::try_from(spelling_index) else {
            return Vec::new();
        };
        let Some((descriptor_offset, descriptor_count)) = self
            .spelling_map
            .descriptor_header(self.source.bytes(), spelling_index)
        else {
            return Vec::new();
        };
        let mut lookups = Vec::new();
        for index in 0..descriptor_count {
            let Some(descriptor) =
                read_runtime_spelling_descriptor(self.source.bytes(), descriptor_offset, index)
            else {
                return Vec::new();
            };
            let Some(syllable_index) = usize::try_from(descriptor.syllable_id).ok() else {
                continue;
            };
            let Some(code) = syllabary_codes.get(syllable_index) else {
                continue;
            };
            lookups.push(PrismLookupCode {
                code,
                abbreviation: descriptor.spelling_type == 2,
                correction: descriptor.is_correction,
                credibility: descriptor.credibility,
            });
            if lookups.len() == limit {
                break;
            }
        }
        lookups
    }
}

impl ByteBackedPrismDoubleArray {
    const HAS_LEAF: u32 = 1 << 8;
    const VALUE_MASK: u32 = (1 << 31) - 1;
    const LABEL_MASK: u32 = (1 << 31) | 0xff;

    const fn byte_len(self) -> usize {
        self.unit_count.saturating_mul(mem::size_of::<u32>())
    }

    fn unit(self, bytes: &[u8], index: usize) -> Option<u32> {
        if index >= self.unit_count {
            return None;
        }
        let offset = self.offset.checked_add(index.checked_mul(4)?)?;
        let chunk = bytes.get(offset..offset.checked_add(4)?)?;
        Some(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
    }

    fn exact_match(self, bytes: &[u8], key: &str) -> Option<u32> {
        let mut node_pos = 0usize;
        let mut unit = self.unit(bytes, node_pos)?;
        for byte in key.bytes() {
            node_pos ^= usize::try_from(Self::offset(unit)).ok()? ^ usize::from(byte);
            unit = self.unit(bytes, node_pos)?;
            if Self::label(unit) != u32::from(byte) {
                return None;
            }
        }
        if !Self::has_leaf(unit) {
            return None;
        }
        let leaf_pos = node_pos ^ usize::try_from(Self::offset(unit)).ok()?;
        self.unit(bytes, leaf_pos).map(Self::value)
    }

    const fn has_leaf(unit: u32) -> bool {
        (unit & Self::HAS_LEAF) != 0
    }

    const fn value(unit: u32) -> u32 {
        unit & Self::VALUE_MASK
    }

    const fn label(unit: u32) -> u32 {
        unit & Self::LABEL_MASK
    }

    const fn offset(unit: u32) -> u32 {
        (unit >> 10) << ((unit & (1 << 9)) >> 6)
    }
}

impl ByteBackedPrismSpellingMap {
    const ITEM_SIZE: usize = 8;
    const DESCRIPTOR_SIZE: usize = 16;

    const fn byte_len(self) -> usize {
        4usize
            .saturating_add(self.spelling_count.saturating_mul(Self::ITEM_SIZE))
            .saturating_add(self.raw_descriptor_bytes)
            .saturating_add(self.tips_payload_bytes)
    }

    fn descriptor_header(self, bytes: &[u8], index: usize) -> Option<(usize, usize)> {
        if index >= self.spelling_count {
            return None;
        }
        let start = self.offset.checked_add(4)?;
        let item_offset = start.checked_add(index.checked_mul(Self::ITEM_SIZE)?)?;
        let descriptor_count = read_count(bytes, item_offset).ok()?;
        let descriptor_offset = read_offset_ptr(bytes, item_offset.checked_add(4)?).ok()??;
        Some((descriptor_offset, descriptor_count))
    }
}

fn prism_byte_source_class(source: &dyn CompactTableByteSource) -> MemoryOwnerClass {
    if source.mapping_mode() == "mmap" {
        MemoryOwnerClass::MmapFileBacked
    } else {
        MemoryOwnerClass::HeapOwnedGuarded
    }
}

fn estimate_double_array_units_bytes(double_array: Option<&DartsDoubleArray>) -> usize {
    double_array.map_or(0, |double_array| {
        mem::size_of::<DartsDoubleArray>().saturating_add(
            double_array
                .units_capacity()
                .saturating_mul(mem::size_of::<u32>()),
        )
    })
}

fn estimate_spelling_map_bytes(
    map: &[Vec<RimePrismSpellingDescriptor>],
    outer_capacity: usize,
) -> usize {
    mem::size_of::<Vec<Vec<RimePrismSpellingDescriptor>>>()
        .saturating_add(
            outer_capacity.saturating_mul(mem::size_of::<Vec<RimePrismSpellingDescriptor>>()),
        )
        .saturating_add(
            map.iter()
                .map(|descriptors| {
                    descriptors
                        .capacity()
                        .saturating_mul(mem::size_of::<RimePrismSpellingDescriptor>())
                })
                .sum::<usize>(),
        )
}

fn estimate_correction_tolerance_bytes(
    corrections: &[RimeCorrectionEntry],
    correction_capacity: usize,
    tolerance_rules: &[RimeToleranceRule],
    tolerance_rule_capacity: usize,
) -> usize {
    mem::size_of::<Vec<RimeCorrectionEntry>>()
        .saturating_add(correction_capacity.saturating_mul(mem::size_of::<RimeCorrectionEntry>()))
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
        .saturating_add(mem::size_of::<Vec<RimeToleranceRule>>())
        .saturating_add(tolerance_rule_capacity.saturating_mul(mem::size_of::<RimeToleranceRule>()))
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

fn estimate_tips_payload_bytes(map: &[Vec<RimePrismSpellingDescriptor>]) -> usize {
    map.iter()
        .flatten()
        .map(|descriptor| descriptor.tips.capacity())
        .sum()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RimePrismBinParseError {
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

pub fn parse_rime_prism_bin_payload(
    bytes: impl AsRef<[u8]>,
) -> Result<RimePrismBinPayload, RimePrismBinParseError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 320)?;
    let version = parse_rime_format_version_for_payload(bytes, b"Rime::Prism/")
        .map_err(map_metadata_error)?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimePrismBinParseError::UnsupportedVersion);
    }
    let double_array_offset = read_offset_ptr(bytes, 52)?;
    let spelling_map_offset =
        read_offset_ptr(bytes, 56)?.ok_or(RimePrismBinParseError::MissingRequiredSection)?;
    let correction_offset = read_yune_payload_offset(bytes, 60, b"YUNE-CORR\0")?;
    let tolerance_offset = read_yune_payload_offset(bytes, 64, b"YUNE-TOL\0")?;
    let double_array_size = read_u32_le(bytes, 48).map_err(map_metadata_error)?;
    let double_array = read_double_array(bytes, double_array_offset, double_array_size)?;

    Ok(RimePrismBinPayload {
        dict_file_checksum: read_u32_le(bytes, 32).map_err(map_metadata_error)?,
        schema_file_checksum: read_u32_le(bytes, 36).map_err(map_metadata_error)?,
        num_syllables: read_u32_le(bytes, 40).map_err(map_metadata_error)?,
        num_spellings: read_u32_le(bytes, 44).map_err(map_metadata_error)?,
        double_array_size,
        double_array,
        spelling_map: read_spelling_map(bytes, spelling_map_offset)?,
        corrections: correction_offset
            .map(|offset| read_corrections(bytes, offset))
            .transpose()?
            .unwrap_or_default(),
        tolerance_rules: tolerance_offset
            .map(|offset| read_tolerance_rules(bytes, offset))
            .transpose()?
            .unwrap_or_default(),
    })
}

pub fn parse_rime_prism_runtime_payload(
    source: Arc<dyn CompactTableByteSource>,
) -> Result<RimePrismRuntimePayload, RimePrismBinParseError> {
    let bytes = source.bytes();
    ensure_len(bytes, 320)?;
    let version = parse_rime_format_version_for_payload(bytes, b"Rime::Prism/")
        .map_err(map_metadata_error)?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimePrismBinParseError::UnsupportedVersion);
    }
    let double_array_offset = read_offset_ptr(bytes, 52)?;
    let spelling_map_offset =
        read_offset_ptr(bytes, 56)?.ok_or(RimePrismBinParseError::MissingRequiredSection)?;
    let correction_offset = read_yune_payload_offset(bytes, 60, b"YUNE-CORR\0")?;
    let tolerance_offset = read_yune_payload_offset(bytes, 64, b"YUNE-TOL\0")?;
    let double_array_size = read_u32_le(bytes, 48).map_err(map_metadata_error)?;
    let double_array =
        read_byte_backed_double_array(bytes, double_array_offset, double_array_size)?;
    let spelling_map = read_byte_backed_spelling_map(bytes, spelling_map_offset)?;
    let corrections = correction_offset
        .map(|offset| read_corrections(bytes, offset))
        .transpose()?
        .unwrap_or_default();
    let tolerance_rules = tolerance_offset
        .map(|offset| read_tolerance_rules(bytes, offset))
        .transpose()?
        .unwrap_or_default();

    Ok(RimePrismRuntimePayload {
        storage: RimePrismRuntimeStorage::ByteBacked(ByteBackedRimePrismPayload {
            source: Arc::clone(&source),
            double_array,
            spelling_map,
            corrections,
            tolerance_rules,
        }),
    })
}

fn read_double_array(
    bytes: &[u8],
    offset: Option<usize>,
    size: u32,
) -> Result<Option<DartsDoubleArray>, RimePrismBinParseError> {
    let Some(offset) = offset else {
        if size == 0 {
            return Ok(None);
        }
        return Err(RimePrismBinParseError::MissingRequiredSection);
    };
    if size == 0 {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    let size = usize::try_from(size).map_err(|_| RimePrismBinParseError::InvalidCount)?;
    let byte_len = size
        .checked_mul(4)
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    let end = offset
        .checked_add(byte_len)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if end > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    let units = bytes[offset..end]
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect::<Vec<_>>();
    DartsDoubleArray::from_units(units)
        .map(Some)
        .map_err(|_| RimePrismBinParseError::InvalidCount)
}

fn read_byte_backed_double_array(
    bytes: &[u8],
    offset: Option<usize>,
    size: u32,
) -> Result<Option<ByteBackedPrismDoubleArray>, RimePrismBinParseError> {
    let Some(offset) = offset else {
        if size == 0 {
            return Ok(None);
        }
        return Err(RimePrismBinParseError::MissingRequiredSection);
    };
    if size == 0 {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    let unit_count = usize::try_from(size).map_err(|_| RimePrismBinParseError::InvalidCount)?;
    let byte_len = unit_count
        .checked_mul(4)
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    let end = offset
        .checked_add(byte_len)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if end > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    Ok(Some(ByteBackedPrismDoubleArray { offset, unit_count }))
}

fn read_corrections(
    bytes: &[u8],
    offset: usize,
) -> Result<Vec<RimeCorrectionEntry>, RimePrismBinParseError> {
    let payload = bytes
        .get(offset..)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if !payload.starts_with(b"YUNE-CORR\0") {
        return Err(RimePrismBinParseError::UnsupportedSection {
            role: "correction payload".to_owned(),
        });
    }
    let mut cursor = offset
        .checked_add(b"YUNE-CORR\0".len())
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let count = read_count(bytes, cursor)?;
    if count > MAX_CORRECTION_COUNT {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    cursor = cursor
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let mut corrections = Vec::with_capacity(count);
    for _ in 0..count {
        let (observed_input, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let (canonical_code, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        corrections.push(RimeCorrectionEntry::new(observed_input, canonical_code));
    }
    Ok(corrections)
}

fn read_tolerance_rules(
    bytes: &[u8],
    offset: usize,
) -> Result<Vec<RimeToleranceRule>, RimePrismBinParseError> {
    let payload = bytes
        .get(offset..)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if !payload.starts_with(b"YUNE-TOL\0") {
        return Err(RimePrismBinParseError::UnsupportedSection {
            role: "tolerance payload".to_owned(),
        });
    }
    let mut cursor = offset
        .checked_add(b"YUNE-TOL\0".len())
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let count = read_count(bytes, cursor)?;
    if count > MAX_TOLERANCE_RULE_COUNT {
        return Err(RimePrismBinParseError::InvalidCount);
    }
    cursor = cursor
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let mut rules = Vec::with_capacity(count);
    for _ in 0..count {
        let (near_code, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let candidate_count = read_count(bytes, cursor)?;
        if candidate_count > MAX_TOLERANCE_CANDIDATE_COUNT {
            return Err(RimePrismBinParseError::InvalidCount);
        }
        cursor = cursor
            .checked_add(4)
            .ok_or(RimePrismBinParseError::OutOfBounds)?;
        let mut candidate_codes = Vec::with_capacity(candidate_count);
        for _ in 0..candidate_count {
            let (candidate_code, next) = read_len_string(bytes, cursor)?;
            cursor = next;
            candidate_codes.push(candidate_code);
        }
        rules.push(RimeToleranceRule::new(near_code, candidate_codes));
    }
    Ok(rules)
}

fn read_spelling_map(
    bytes: &[u8],
    offset: usize,
) -> Result<Vec<Vec<RimePrismSpellingDescriptor>>, RimePrismBinParseError> {
    let count = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let item_size = 8usize;
    let total = count
        .checked_mul(item_size)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }

    let mut map = Vec::with_capacity(count);
    for index in 0..count {
        let item_offset = start
            .checked_add(
                index
                    .checked_mul(item_size)
                    .ok_or(RimePrismBinParseError::InvalidCount)?,
            )
            .ok_or(RimePrismBinParseError::OutOfBounds)?;
        let descriptor_count = read_count(bytes, item_offset)?;
        let descriptor_offset = read_offset_ptr(bytes, item_offset + 4)?
            .ok_or(RimePrismBinParseError::MissingRequiredSection)?;
        map.push(read_spelling_descriptors(
            bytes,
            descriptor_offset,
            descriptor_count,
        )?);
    }
    Ok(map)
}

fn read_byte_backed_spelling_map(
    bytes: &[u8],
    offset: usize,
) -> Result<ByteBackedPrismSpellingMap, RimePrismBinParseError> {
    let spelling_count = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let total = spelling_count
        .checked_mul(ByteBackedPrismSpellingMap::ITEM_SIZE)
        .and_then(|len| start.checked_add(len))
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }

    let mut descriptor_count = 0usize;
    let mut raw_descriptor_bytes = 0usize;
    let mut tips_payload_bytes = 0usize;
    let mut tip_count = 0usize;
    for index in 0..spelling_count {
        let item_offset = start
            .checked_add(
                index
                    .checked_mul(ByteBackedPrismSpellingMap::ITEM_SIZE)
                    .ok_or(RimePrismBinParseError::InvalidCount)?,
            )
            .ok_or(RimePrismBinParseError::OutOfBounds)?;
        let descriptors = read_count(bytes, item_offset)?;
        let descriptor_offset = read_offset_ptr(bytes, item_offset + 4)?
            .ok_or(RimePrismBinParseError::MissingRequiredSection)?;
        validate_spelling_descriptor_range(bytes, descriptor_offset, descriptors)?;
        descriptor_count = descriptor_count
            .checked_add(descriptors)
            .ok_or(RimePrismBinParseError::InvalidCount)?;
        raw_descriptor_bytes = raw_descriptor_bytes
            .checked_add(
                descriptors
                    .checked_mul(ByteBackedPrismSpellingMap::DESCRIPTOR_SIZE)
                    .ok_or(RimePrismBinParseError::InvalidCount)?,
            )
            .ok_or(RimePrismBinParseError::InvalidCount)?;
        for descriptor_index in 0..descriptors {
            let descriptor_base = descriptor_offset
                .checked_add(
                    descriptor_index
                        .checked_mul(ByteBackedPrismSpellingMap::DESCRIPTOR_SIZE)
                        .ok_or(RimePrismBinParseError::InvalidCount)?,
                )
                .ok_or(RimePrismBinParseError::OutOfBounds)?;
            if let Some(tip_len) = read_string_payload_len(bytes, descriptor_base + 12)? {
                if tip_len > 0 {
                    tip_count = tip_count.saturating_add(1);
                    tips_payload_bytes = tips_payload_bytes.saturating_add(tip_len);
                }
            }
        }
    }

    Ok(ByteBackedPrismSpellingMap {
        offset,
        spelling_count,
        descriptor_count,
        raw_descriptor_bytes,
        tips_payload_bytes,
        tip_count,
    })
}

fn read_spelling_descriptors(
    bytes: &[u8],
    offset: usize,
    count: usize,
) -> Result<Vec<RimePrismSpellingDescriptor>, RimePrismBinParseError> {
    let descriptor_size = 16usize;
    let total = count
        .checked_mul(descriptor_size)
        .and_then(|len| offset.checked_add(len))
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }

    let mut descriptors = Vec::with_capacity(count);
    for index in 0..count {
        let descriptor_offset = offset
            .checked_add(
                index
                    .checked_mul(descriptor_size)
                    .ok_or(RimePrismBinParseError::InvalidCount)?,
            )
            .ok_or(RimePrismBinParseError::OutOfBounds)?;
        let packed_type = read_i32_le(bytes, descriptor_offset + 4).map_err(map_metadata_error)?;
        descriptors.push(RimePrismSpellingDescriptor {
            syllable_id: read_i32_le(bytes, descriptor_offset).map_err(map_metadata_error)?,
            spelling_type: packed_type & !(1 << 30),
            is_correction: packed_type & (1 << 30) != 0,
            credibility: read_f32_le(bytes, descriptor_offset + 8).map_err(map_metadata_error)?,
            tips: read_string(bytes, descriptor_offset + 12)?,
        });
    }
    Ok(descriptors)
}

fn validate_spelling_descriptor_range(
    bytes: &[u8],
    offset: usize,
    count: usize,
) -> Result<(), RimePrismBinParseError> {
    let total = count
        .checked_mul(ByteBackedPrismSpellingMap::DESCRIPTOR_SIZE)
        .and_then(|len| offset.checked_add(len))
        .ok_or(RimePrismBinParseError::InvalidCount)?;
    if total > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    Ok(())
}

fn read_runtime_spelling_descriptor(
    bytes: &[u8],
    offset: usize,
    index: usize,
) -> Option<RuntimePrismSpellingDescriptor> {
    let descriptor_offset =
        offset.checked_add(index.checked_mul(ByteBackedPrismSpellingMap::DESCRIPTOR_SIZE)?)?;
    let packed_type = read_i32_le(bytes, descriptor_offset.checked_add(4)?).ok()?;
    Some(RuntimePrismSpellingDescriptor {
        syllable_id: read_i32_le(bytes, descriptor_offset).ok()?,
        spelling_type: packed_type & !(1 << 30),
        is_correction: packed_type & (1 << 30) != 0,
        credibility: read_f32_le(bytes, descriptor_offset.checked_add(8)?).ok()?,
    })
}

fn read_string(bytes: &[u8], offset: usize) -> Result<String, RimePrismBinParseError> {
    let Some(string_offset) = read_offset_ptr(bytes, offset)? else {
        return Ok(String::new());
    };
    if string_offset >= bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    let end = bytes[string_offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|position| string_offset + position)
        .ok_or(RimePrismBinParseError::InvalidLength)?;
    std::str::from_utf8(&bytes[string_offset..end])
        .map(str::to_owned)
        .map_err(|_| RimePrismBinParseError::InvalidUtf8)
}

fn read_string_payload_len(
    bytes: &[u8],
    offset: usize,
) -> Result<Option<usize>, RimePrismBinParseError> {
    let Some(string_offset) = read_offset_ptr(bytes, offset)? else {
        return Ok(None);
    };
    if string_offset >= bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    let len = bytes[string_offset..]
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(RimePrismBinParseError::InvalidLength)?;
    std::str::from_utf8(&bytes[string_offset..string_offset + len])
        .map_err(|_| RimePrismBinParseError::InvalidUtf8)?;
    Ok(Some(len))
}

fn read_len_string(bytes: &[u8], offset: usize) -> Result<(String, usize), RimePrismBinParseError> {
    let len = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    let end = start
        .checked_add(len)
        .ok_or(RimePrismBinParseError::InvalidLength)?;
    if end > bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    let value = std::str::from_utf8(&bytes[start..end])
        .map(str::to_owned)
        .map_err(|_| RimePrismBinParseError::InvalidUtf8)?;
    Ok((value, end))
}

fn read_offset_ptr(
    bytes: &[u8],
    field_offset: usize,
) -> Result<Option<usize>, RimePrismBinParseError> {
    let raw = read_i32_le(bytes, field_offset).map_err(map_metadata_error)?;
    if raw == 0 {
        return Ok(None);
    }
    let target = field_offset
        .checked_add_signed(raw as isize)
        .ok_or(RimePrismBinParseError::OutOfBounds)?;
    if target >= bytes.len() {
        return Err(RimePrismBinParseError::OutOfBounds);
    }
    Ok(Some(target))
}

fn read_yune_payload_offset(
    bytes: &[u8],
    field_offset: usize,
    marker: &[u8],
) -> Result<Option<usize>, RimePrismBinParseError> {
    let raw = read_i32_le(bytes, field_offset).map_err(map_metadata_error)?;
    if raw == 0 {
        return Ok(None);
    }
    let Some(target) = field_offset.checked_add_signed(raw as isize) else {
        return Ok(None);
    };
    if target >= bytes.len() {
        return Ok(None);
    }
    if bytes[target..].starts_with(marker) || bytes[target..].starts_with(b"YUNE-") {
        Ok(Some(target))
    } else {
        Ok(None)
    }
}

fn read_count(bytes: &[u8], offset: usize) -> Result<usize, RimePrismBinParseError> {
    let count = read_u32_le(bytes, offset).map_err(map_metadata_error)?;
    usize::try_from(count).map_err(|_| RimePrismBinParseError::InvalidCount)
}

fn ensure_len(bytes: &[u8], len: usize) -> Result<(), RimePrismBinParseError> {
    if bytes.len() < len {
        return Err(RimePrismBinParseError::TooShort);
    }
    Ok(())
}

fn map_metadata_error(error: super::RimeCompiledMetadataError) -> RimePrismBinParseError {
    match error {
        super::RimeCompiledMetadataError::TooShort => RimePrismBinParseError::TooShort,
        super::RimeCompiledMetadataError::InvalidFormat => RimePrismBinParseError::InvalidFormat,
        super::RimeCompiledMetadataError::UnsupportedVersion => {
            RimePrismBinParseError::UnsupportedVersion
        }
        super::RimeCompiledMetadataError::MissingRequiredSection => {
            RimePrismBinParseError::MissingRequiredSection
        }
    }
}
