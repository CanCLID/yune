use super::{TableDictionary, TableDictionaryAdvancedData, TableEntry};
use crate::dictionary::compiled::{parse_rime_format_version_for_payload, read_u32_le};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RimeReverseBinParseError {
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

pub fn parse_rime_reverse_bin_dictionary(
    bytes: impl AsRef<[u8]>,
) -> Result<TableDictionary, RimeReverseBinParseError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 64)?;
    let version = parse_rime_format_version_for_payload(bytes, b"Rime::Reverse/")
        .map_err(map_metadata_error)?;
    if !(3.0 - f64::EPSILON..=4.0 + f64::EPSILON).contains(&version) {
        return Err(RimeReverseBinParseError::UnsupportedVersion);
    }

    let index_count = read_u32_le(bytes, 40).map_err(map_metadata_error)?;
    let key_trie = read_u32_le(bytes, 48).map_err(map_metadata_error)?;
    let key_trie_size = read_u32_le(bytes, 52).map_err(map_metadata_error)?;
    let value_trie = read_u32_le(bytes, 56).map_err(map_metadata_error)?;
    let value_trie_size = read_u32_le(bytes, 60).map_err(map_metadata_error)?;
    if key_trie != 0 || key_trie_size != 0 || value_trie != 0 || value_trie_size != 0 {
        return Err(RimeReverseBinParseError::UnsupportedSection {
            role: "marisa reverse key/value trie".to_owned(),
        });
    }
    if index_count != 0 {
        return Err(RimeReverseBinParseError::UnsupportedSection {
            role: "reverse trie index".to_owned(),
        });
    }

    let payload = read_yune_reverse_payload(bytes)?;
    Ok(TableDictionary::with_advanced_data(
        payload.entries,
        payload.data,
    ))
}

struct ReversePayload {
    entries: Vec<TableEntry>,
    data: TableDictionaryAdvancedData,
}

fn read_yune_reverse_payload(bytes: &[u8]) -> Result<ReversePayload, RimeReverseBinParseError> {
    let marker = b"YUNE-REVERSE\0";
    let start = 64usize;
    if bytes.len() == start {
        return Err(RimeReverseBinParseError::MissingRequiredSection);
    }
    if !bytes[start..].starts_with(marker) {
        return Err(RimeReverseBinParseError::UnsupportedSection {
            role: "non-Yune reverse payload".to_owned(),
        });
    }
    let mut cursor = start
        .checked_add(marker.len())
        .ok_or(RimeReverseBinParseError::OutOfBounds)?;
    let count = read_count(bytes, cursor)?;
    cursor = cursor
        .checked_add(4)
        .ok_or(RimeReverseBinParseError::OutOfBounds)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let (code, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        let (text, next) = read_len_string(bytes, cursor)?;
        cursor = next;
        entries.push(TableEntry::new(code, text, 0.0));
    }

    let mut dict_settings = BTreeMap::new();
    let mut stems = HashMap::new();
    if cursor < bytes.len() {
        let setting_count = read_count(bytes, cursor)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeReverseBinParseError::OutOfBounds)?;
        for _ in 0..setting_count {
            let (key, next) = read_len_string(bytes, cursor)?;
            cursor = next;
            let (value, next) = read_len_string(bytes, cursor)?;
            cursor = next;
            dict_settings.insert(key, value);
        }

        let stem_count = read_count(bytes, cursor)?;
        cursor = cursor
            .checked_add(4)
            .ok_or(RimeReverseBinParseError::OutOfBounds)?;
        for _ in 0..stem_count {
            let (text, next) = read_len_string(bytes, cursor)?;
            cursor = next;
            let count = read_count(bytes, cursor)?;
            cursor = cursor
                .checked_add(4)
                .ok_or(RimeReverseBinParseError::OutOfBounds)?;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                let (stem, next) = read_len_string(bytes, cursor)?;
                cursor = next;
                values.push(stem);
            }
            stems.insert(text, values);
        }
    }

    Ok(ReversePayload {
        entries,
        data: TableDictionaryAdvancedData {
            stems,
            dict_settings,
            ..TableDictionaryAdvancedData::default()
        },
    })
}

fn read_len_string(
    bytes: &[u8],
    offset: usize,
) -> Result<(String, usize), RimeReverseBinParseError> {
    let len = read_count(bytes, offset)?;
    let start = offset
        .checked_add(4)
        .ok_or(RimeReverseBinParseError::OutOfBounds)?;
    let end = start
        .checked_add(len)
        .ok_or(RimeReverseBinParseError::InvalidLength)?;
    if end > bytes.len() {
        return Err(RimeReverseBinParseError::OutOfBounds);
    }
    let value = std::str::from_utf8(&bytes[start..end])
        .map(str::to_owned)
        .map_err(|_| RimeReverseBinParseError::InvalidUtf8)?;
    Ok((value, end))
}

fn read_count(bytes: &[u8], offset: usize) -> Result<usize, RimeReverseBinParseError> {
    let count = read_u32_le(bytes, offset).map_err(map_metadata_error)?;
    usize::try_from(count).map_err(|_| RimeReverseBinParseError::InvalidCount)
}

fn ensure_len(bytes: &[u8], len: usize) -> Result<(), RimeReverseBinParseError> {
    if bytes.len() < len {
        return Err(RimeReverseBinParseError::TooShort);
    }
    Ok(())
}

fn map_metadata_error(error: super::RimeCompiledMetadataError) -> RimeReverseBinParseError {
    match error {
        super::RimeCompiledMetadataError::TooShort => RimeReverseBinParseError::TooShort,
        super::RimeCompiledMetadataError::InvalidFormat => RimeReverseBinParseError::InvalidFormat,
        super::RimeCompiledMetadataError::UnsupportedVersion => {
            RimeReverseBinParseError::UnsupportedVersion
        }
        super::RimeCompiledMetadataError::MissingRequiredSection => {
            RimeReverseBinParseError::MissingRequiredSection
        }
    }
}
