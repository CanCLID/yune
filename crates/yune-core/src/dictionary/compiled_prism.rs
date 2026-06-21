use super::{RimeCorrectionEntry, RimeToleranceRule};
use crate::dictionary::compiled::{
    parse_rime_format_version_for_payload, read_f32_le, read_i32_le, read_u32_le,
};
use crate::dictionary::double_array::DartsDoubleArray;

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
