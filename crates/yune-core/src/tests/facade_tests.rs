use super::{
    parse_key_sequence, parse_rime_prism_bin_metadata, parse_rime_prism_bin_payload,
    parse_rime_reverse_bin_dictionary, parse_rime_reverse_bin_metadata,
    parse_rime_table_bin_dictionary, parse_rime_table_bin_metadata, rime_checksum_bytes,
    rime_dict_rebuild_plan, rime_dict_source_checksum, CodeCoords, KeyCode, RimeChecksumComputer,
    RimeCompiledMetadataError, RimeDictArtifactStatus, RimeDictRebuildError,
    RimeDictRebuildExecutionReport, RimeDictRebuildInput, RimeDictRebuildPlan,
    RimePrismBinMetadata, RimePrismBinParseError, RimePrismChecksumMetadata,
    RimeReverseBinMetadata, RimeReverseBinParseError, RimeTableBinMetadata, RimeTableBinParseError,
    TableDictionary, TableEncoder,
};

include!("facade_tests/key_sequences.rs");
include!("facade_tests/dictionary_artifacts.rs");
include!("facade_tests/compiled_payloads.rs");
include!("facade_tests/dictionary_yaml.rs");
