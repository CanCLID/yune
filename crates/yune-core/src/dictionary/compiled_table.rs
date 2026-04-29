use super::{
    RimeCorrectionEntry, RimeToleranceRule, TableDictionary, TableDictionaryAdvancedData,
    TableEncoder, TableEntry,
};
use crate::dictionary::compiled::{
    parse_rime_format_version_for_payload, read_f32_le, read_i32_le, read_u32_le,
};
use std::collections::HashMap;

const MAX_CORRECTION_COUNT: usize = 4096;
const MAX_TOLERANCE_RULE_COUNT: usize = 4096;
const MAX_TOLERANCE_CANDIDATE_COUNT: usize = 64;

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
    let node_size = 12usize;
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

    let (corrections, tolerance_rules) = if cursor < bytes.len() {
        read_correction_tolerance_payload(bytes, cursor)?
    } else {
        (Vec::new(), Vec::new())
    };

    Ok(AdvancedTablePayload {
        entries,
        data: TableDictionaryAdvancedData {
            stems,
            encoder,
            corrections,
            tolerance_rules,
            ..TableDictionaryAdvancedData::default()
        },
    })
}

fn read_correction_tolerance_payload(
    bytes: &[u8],
    mut cursor: usize,
) -> Result<(Vec<RimeCorrectionEntry>, Vec<RimeToleranceRule>), RimeTableBinParseError> {
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
    Ok((corrections, tolerance_rules))
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
