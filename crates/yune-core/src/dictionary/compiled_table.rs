use super::{TableDictionary, TableEntry};
use crate::dictionary::compiled::{parse_rime_format_version_for_payload, read_f32_le, read_i32_le, read_u32_le};

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

    let syllabary_offset = read_offset_ptr(bytes, 44)?
        .ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let index_offset = read_offset_ptr(bytes, 48)?
        .ok_or(RimeTableBinParseError::MissingRequiredSection)?;
    let string_table_offset = read_offset_ptr(bytes, 60)?;
    let string_table_size = read_u32_le(bytes, 64).map_err(map_metadata_error)?;
    if string_table_offset.is_some() || string_table_size != 0 {
        return Err(RimeTableBinParseError::UnsupportedSection {
            role: "marisa string_table".to_owned(),
        });
    }

    let syllables = read_syllabary(bytes, syllabary_offset)?;
    let entries = read_head_index_entries(bytes, index_offset, &syllables)?;
    Ok(TableDictionary::new(entries))
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
            .checked_add(index.checked_mul(4).ok_or(RimeTableBinParseError::InvalidCount)?)
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
    let node_size = 8usize;
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
            .checked_add(index.checked_mul(node_size).ok_or(RimeTableBinParseError::InvalidCount)?)
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
            .checked_add(index.checked_mul(entry_size).ok_or(RimeTableBinParseError::InvalidCount)?)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let text = read_string_type(bytes, entry_offset)?;
        let weight = read_f32_le(bytes, entry_offset + 4).map_err(map_metadata_error)?;
        entries.push(TableEntry::new(code, text, weight));
    }
    Ok(entries)
}

fn read_string_type(bytes: &[u8], offset: usize) -> Result<String, RimeTableBinParseError> {
    let string_offset = read_offset_ptr(bytes, offset)?.ok_or(RimeTableBinParseError::OutOfBounds)?;
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

fn read_offset_ptr(bytes: &[u8], field_offset: usize) -> Result<Option<usize>, RimeTableBinParseError> {
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
