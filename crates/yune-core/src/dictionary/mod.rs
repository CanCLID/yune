mod compiled;
mod compiled_prism;
mod compiled_reverse;
mod compiled_table;
mod double_array;
mod encoder;
mod prism_writer;
mod rebuild_executor;
mod reverse_writer;
mod source;
mod table_writer;

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
pub use double_array::{DartsDoubleArray, DartsDoubleArrayError, DartsMatch};
pub use encoder::{CodeCoords, TableEncoder, TableEncoderFormulaError, TableEncodingRule};
pub use prism_writer::build_prism_bin;
pub use rebuild_executor::{
    execute_rebuild_plan, RimeDictRebuildExecuteError, RimeDictRebuildSources,
};
pub use reverse_writer::build_reverse_bin;
pub(crate) use source::normalize_table_code;
pub use source::{
    DictionaryLookupRecord, PresetVocabularyEntry, RimeCorrectionEntry, RimeToleranceRule,
    TableDictionary, TableDictionaryAdvancedData, TableDictionaryParseError, TableEntry,
};
pub use table_writer::build_table_bin;
