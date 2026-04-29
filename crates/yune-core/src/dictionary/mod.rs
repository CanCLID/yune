mod compiled;
mod compiled_prism;
mod compiled_reverse;
mod compiled_table;
mod encoder;
mod source;

pub use compiled::{
    parse_rime_prism_bin_metadata, parse_rime_reverse_bin_metadata, parse_rime_table_bin_metadata,
    rime_checksum_bytes, rime_dict_rebuild_plan, rime_dict_source_checksum, RimeChecksumComputer,
    RimeCompiledMetadataError, RimeDictArtifactStatus, RimeDictRebuildError,
    RimeDictRebuildExecutionReport, RimeDictRebuildInput, RimeDictRebuildPlan,
    RimePrismBinMetadata, RimePrismChecksumMetadata, RimeReverseBinMetadata, RimeTableBinMetadata,
};
pub use compiled_prism::{
    parse_rime_prism_bin_payload, RimePrismBinParseError, RimePrismBinPayload,
    RimePrismSpellingDescriptor,
};
pub use compiled_reverse::{parse_rime_reverse_bin_dictionary, RimeReverseBinParseError};
pub use compiled_table::{
    parse_rime_table_bin_dictionary, rime_table_bin_dict_file_checksum, RimeTableBinParseError,
};
pub use encoder::{CodeCoords, TableEncoder, TableEncoderFormulaError, TableEncodingRule};
pub(crate) use source::normalize_table_code;
pub use source::{
    RimeCorrectionEntry, RimeToleranceRule, TableDictionary, TableDictionaryAdvancedData,
    TableDictionaryParseError, TableEntry,
};
