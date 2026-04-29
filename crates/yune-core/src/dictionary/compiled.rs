#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RimeChecksumComputer {
    remainder: u32,
}

impl RimeChecksumComputer {
    const POLYNOMIAL: u32 = 0xedb8_8320;

    #[must_use]
    pub const fn new(initial_remainder: u32) -> Self {
        Self {
            remainder: initial_remainder,
        }
    }

    pub fn process_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        for byte in bytes.as_ref() {
            self.remainder ^= u32::from(*byte);
            for _ in 0..8 {
                if self.remainder & 1 == 1 {
                    self.remainder = (self.remainder >> 1) ^ Self::POLYNOMIAL;
                } else {
                    self.remainder >>= 1;
                }
            }
        }
    }

    #[must_use]
    pub const fn checksum(&self) -> u32 {
        self.remainder ^ 0xffff_ffff
    }
}

#[must_use]
pub fn rime_checksum_bytes(bytes: impl AsRef<[u8]>) -> u32 {
    let mut checksum = RimeChecksumComputer::new(0);
    checksum.process_bytes(bytes);
    checksum.checksum()
}

#[must_use]
pub fn rime_dict_source_checksum<B>(
    initial_checksum: u32,
    dict_sources: impl IntoIterator<Item = B>,
    preset_vocabulary: Option<B>,
) -> u32
where
    B: AsRef<[u8]>,
{
    let mut dict_sources = dict_sources.into_iter().peekable();
    if dict_sources.peek().is_none() {
        return initial_checksum;
    }

    let mut checksum = RimeChecksumComputer::new(initial_checksum);
    for source in dict_sources {
        checksum.process_bytes(source);
    }
    if let Some(preset_vocabulary) = preset_vocabulary {
        checksum.process_bytes(preset_vocabulary);
    }
    checksum.checksum()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RimeTableBinMetadata {
    pub dict_file_checksum: u32,
    pub num_syllables: u32,
    pub num_entries: u32,
    pub string_table_size: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RimePrismBinMetadata {
    pub dict_file_checksum: u32,
    pub schema_file_checksum: u32,
    pub num_syllables: u32,
    pub num_spellings: u32,
    pub double_array_size: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RimeReverseBinMetadata {
    pub dict_file_checksum: u32,
    pub key_trie_size: u32,
    pub value_trie_size: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RimeCompiledMetadataError {
    TooShort,
    InvalidFormat,
    UnsupportedVersion,
    MissingRequiredSection,
}

pub fn parse_rime_table_bin_metadata(
    bytes: impl AsRef<[u8]>,
) -> Result<RimeTableBinMetadata, RimeCompiledMetadataError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 68)?;
    let version = parse_rime_format_version(bytes, b"Rime::Table/")?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimeCompiledMetadataError::UnsupportedVersion);
    }
    if read_u32_le(bytes, 44)? == 0 || read_u32_le(bytes, 48)? == 0 {
        return Err(RimeCompiledMetadataError::MissingRequiredSection);
    }

    Ok(RimeTableBinMetadata {
        dict_file_checksum: read_u32_le(bytes, 32)?,
        num_syllables: read_u32_le(bytes, 36)?,
        num_entries: read_u32_le(bytes, 40)?,
        string_table_size: read_u32_le(bytes, 64)?,
    })
}

pub fn parse_rime_prism_bin_metadata(
    bytes: impl AsRef<[u8]>,
) -> Result<RimePrismBinMetadata, RimeCompiledMetadataError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 320)?;
    let version = parse_rime_format_version(bytes, b"Rime::Prism/")?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimeCompiledMetadataError::UnsupportedVersion);
    }
    if read_u32_le(bytes, 52)? == 0 {
        return Err(RimeCompiledMetadataError::MissingRequiredSection);
    }

    Ok(RimePrismBinMetadata {
        dict_file_checksum: read_u32_le(bytes, 32)?,
        schema_file_checksum: read_u32_le(bytes, 36)?,
        num_syllables: read_u32_le(bytes, 40)?,
        num_spellings: read_u32_le(bytes, 44)?,
        double_array_size: read_u32_le(bytes, 48)?,
    })
}

pub fn parse_rime_reverse_bin_metadata(
    bytes: impl AsRef<[u8]>,
) -> Result<RimeReverseBinMetadata, RimeCompiledMetadataError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 64)?;
    let version = parse_rime_format_version(bytes, b"Rime::Reverse/")?;
    if !(3.0 - f64::EPSILON..=4.0 + f64::EPSILON).contains(&version) {
        return Err(RimeCompiledMetadataError::UnsupportedVersion);
    }

    Ok(RimeReverseBinMetadata {
        dict_file_checksum: read_u32_le(bytes, 32)?,
        key_trie_size: read_u32_le(bytes, 52)?,
        value_trie_size: read_u32_le(bytes, 60)?,
    })
}

fn ensure_len(bytes: &[u8], len: usize) -> Result<(), RimeCompiledMetadataError> {
    if bytes.len() < len {
        return Err(RimeCompiledMetadataError::TooShort);
    }
    Ok(())
}

pub(crate) fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, RimeCompiledMetadataError> {
    let end = offset
        .checked_add(4)
        .ok_or(RimeCompiledMetadataError::TooShort)?;
    let Some(value) = bytes.get(offset..end) else {
        return Err(RimeCompiledMetadataError::TooShort);
    };
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

pub(crate) fn read_i32_le(bytes: &[u8], offset: usize) -> Result<i32, RimeCompiledMetadataError> {
    read_u32_le(bytes, offset).map(|value| value as i32)
}

pub(crate) fn read_f32_le(bytes: &[u8], offset: usize) -> Result<f32, RimeCompiledMetadataError> {
    read_u32_le(bytes, offset).map(f32::from_bits)
}

fn parse_rime_format_version(
    bytes: &[u8],
    prefix: &[u8],
) -> Result<f64, RimeCompiledMetadataError> {
    parse_rime_format_version_for_payload(bytes, prefix)
}

pub(crate) fn parse_rime_format_version_for_payload(
    bytes: &[u8],
    prefix: &[u8],
) -> Result<f64, RimeCompiledMetadataError> {
    let Some(format) = bytes.get(..32) else {
        return Err(RimeCompiledMetadataError::TooShort);
    };
    if !format.starts_with(prefix) {
        return Err(RimeCompiledMetadataError::InvalidFormat);
    }
    let version_end = format.iter().position(|byte| *byte == 0).unwrap_or(32);
    let version = std::str::from_utf8(&format[prefix.len()..version_end])
        .ok()
        .and_then(|version| version.parse::<f64>().ok())
        .ok_or(RimeCompiledMetadataError::InvalidFormat)?;
    Ok(version)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RimePrismChecksumMetadata {
    pub dict_file_checksum: u32,
    pub schema_file_checksum: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RimeDictRebuildInput {
    pub source_available: bool,
    pub source_dict_file_checksum: u32,
    pub pack_source_checksums: Vec<u32>,
    pub schema_file_checksum: u32,
    pub table_dict_file_checksum: Option<u32>,
    pub prism: Option<RimePrismChecksumMetadata>,
    pub reverse_dict_file_checksum: Option<u32>,
    pub prebuilt_table_available: bool,
    pub prebuilt_prism_available: bool,
    pub prebuilt_reverse_available: bool,
    pub force_rebuild_table: bool,
    pub force_rebuild_prism: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RimeDictRebuildPlan {
    pub dict_file_checksum: u32,
    pub rebuild_table: bool,
    pub rebuild_prism: bool,
    pub rebuild_reverse: bool,
    pub report: RimeDictRebuildExecutionReport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RimeDictRebuildExecutionReport {
    pub table: RimeDictArtifactStatus,
    pub prism: RimeDictArtifactStatus,
    pub reverse: RimeDictArtifactStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RimeDictArtifactStatus {
    Rebuilt,
    ReusedFresh,
    ReusedPrebuilt,
    MissingSourceAndCompiled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RimeDictRebuildError {
    MissingSourceAndCompiled,
}

pub fn rime_dict_rebuild_plan(
    input: RimeDictRebuildInput,
) -> Result<RimeDictRebuildPlan, RimeDictRebuildError> {
    let dict_file_checksum = input
        .pack_source_checksums
        .last()
        .copied()
        .unwrap_or(input.source_dict_file_checksum);

    if !input.source_available {
        let Some(table_checksum) = input.table_dict_file_checksum else {
            return missing_source_and_compiled(dict_file_checksum);
        };
        if !input.prebuilt_table_available || !input.prebuilt_reverse_available {
            return missing_source_and_compiled(table_checksum);
        }
        let prism_reusable = input.prebuilt_prism_available
            && input.prism.is_some_and(|prism| {
                prism.dict_file_checksum == table_checksum
                    && prism.schema_file_checksum == input.schema_file_checksum
            });
        if !prism_reusable {
            return missing_source_and_compiled(table_checksum);
        }
        let reverse_reusable = input.reverse_dict_file_checksum == Some(table_checksum);
        if !reverse_reusable {
            return missing_source_and_compiled(table_checksum);
        }
        return Ok(RimeDictRebuildPlan {
            dict_file_checksum: table_checksum,
            rebuild_table: false,
            rebuild_prism: false,
            rebuild_reverse: false,
            report: RimeDictRebuildExecutionReport {
                table: RimeDictArtifactStatus::ReusedPrebuilt,
                prism: RimeDictArtifactStatus::ReusedPrebuilt,
                reverse: RimeDictArtifactStatus::ReusedPrebuilt,
            },
        });
    }

    let table_stale = input.table_dict_file_checksum != Some(dict_file_checksum);
    let mut rebuild_reverse = input.reverse_dict_file_checksum != Some(dict_file_checksum);
    let mut rebuild_table = table_stale || rebuild_reverse || input.force_rebuild_table;
    if input.table_dict_file_checksum.is_none() {
        rebuild_table = true;
    }
    if rebuild_table && input.reverse_dict_file_checksum.is_none() {
        rebuild_reverse = true;
    }
    if table_stale {
        rebuild_reverse = true;
    }

    let mut rebuild_prism = match input.prism {
        Some(prism) => {
            prism.dict_file_checksum != dict_file_checksum
                || prism.schema_file_checksum != input.schema_file_checksum
        }
        None => true,
    };
    if input.force_rebuild_prism {
        rebuild_prism = true;
    }

    Ok(RimeDictRebuildPlan {
        dict_file_checksum,
        rebuild_table,
        rebuild_prism,
        rebuild_reverse,
        report: RimeDictRebuildExecutionReport {
            table: artifact_status(rebuild_table),
            prism: artifact_status(rebuild_prism),
            reverse: artifact_status(rebuild_reverse),
        },
    })
}

fn artifact_status(rebuild: bool) -> RimeDictArtifactStatus {
    if rebuild {
        RimeDictArtifactStatus::Rebuilt
    } else {
        RimeDictArtifactStatus::ReusedFresh
    }
}

fn missing_source_and_compiled(
    dict_file_checksum: u32,
) -> Result<RimeDictRebuildPlan, RimeDictRebuildError> {
    let _ = RimeDictRebuildPlan {
        dict_file_checksum,
        rebuild_table: false,
        rebuild_prism: false,
        rebuild_reverse: false,
        report: RimeDictRebuildExecutionReport {
            table: RimeDictArtifactStatus::MissingSourceAndCompiled,
            prism: RimeDictArtifactStatus::MissingSourceAndCompiled,
            reverse: RimeDictArtifactStatus::MissingSourceAndCompiled,
        },
    };
    Err(RimeDictRebuildError::MissingSourceAndCompiled)
}
