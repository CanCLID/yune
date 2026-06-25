use std::collections::HashMap;

mod ai;
mod comment_format;
mod dictionary;
mod engine;
mod filter;
mod key;
mod m37_metrics;
mod poet;
mod punctuation;
mod spelling_algebra;
mod state;
#[cfg(test)]
mod tests;
mod translator;
mod userdb;
pub use ai::{
    memory_store_file_name, memory_store_snapshot_file_name, validate_memory_store_id,
    AiCandidateProvider, AiContextProvider, AiContextSnapshot, AiDecision, AiMemoryEntry,
    AiMemoryRecordResult, AiMemorySkipReason, AiMemorySnapshotError, AiOffReason, AiPrivacyPolicy,
    AiProviderKind, AiResult, AiWorker, EngineAiContextProvider, LocalModelProvider,
    LocalModelRule, MemoryStore, MockAiProvider, StagedAiCandidates, LOCAL_MODEL_PROVIDER_NAME,
    MEMORY_STORE_FILE_SUFFIX, MEMORY_STORE_SNAPSHOT_SUFFIX,
};
use comment_format::CommentFormat;
pub use dictionary::{
    build_prism_bin, build_reverse_bin, build_table_bin, execute_rebuild_plan,
    parse_rime_prism_bin_metadata, parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_reverse_bin_metadata, parse_rime_table_bin_advanced_data,
    parse_rime_table_bin_dictionary, parse_rime_table_bin_metadata, rime_checksum_bytes,
    rime_dict_rebuild_plan, rime_dict_source_checksum, rime_table_bin_dict_file_checksum,
    CodeCoords, CompactTableByteSource, CompactTableStore, DartsDoubleArray, DartsDoubleArrayError,
    DartsMatch, DictionaryLookupRecord, PresetVocabularyEntry, RimeChecksumComputer,
    RimeCompiledMetadataError, RimeCorrectionEntry, RimeDictArtifactStatus, RimeDictRebuildError,
    RimeDictRebuildExecuteError, RimeDictRebuildExecutionReport, RimeDictRebuildInput,
    RimeDictRebuildPlan, RimeDictRebuildSources, RimePrismBinMetadata, RimePrismBinParseError,
    RimePrismBinPayload, RimePrismChecksumMetadata, RimePrismSpellingDescriptor,
    RimeReverseBinMetadata, RimeReverseBinParseError, RimeTableBinMetadata, RimeTableBinParseError,
    RimeToleranceRule, TableDictionary, TableDictionaryAdvancedData, TableDictionaryParseError,
    TableEncoder, TableEncoderFormulaError, TableEncodingRule, TableEntry,
};
pub use engine::Engine;
pub use filter::{
    CharsetFilter, DictionaryLookupFilter, ReverseLookupFilter, SimplifierFilter, SingleCharFilter,
    TaggedFilter, UniquifierFilter,
};
pub use key::{parse_key_sequence, KeyCode, KeyEvent, KeyModifiers, KeySequenceParseError};
pub use m37_metrics::{
    m37_metrics_enable, m37_metrics_enabled, m37_metrics_reset, m37_metrics_snapshot,
    m37_record_abi_candidates_exported, m37_record_abi_free_context, m37_record_abi_get_context,
    m37_record_ai_merge, m37_record_candidate_sort, m37_record_candidates_sorted,
    m37_record_candidates_stored, m37_record_context_full_snapshot_clone,
    m37_record_context_page_snapshot_clone, m37_record_filter_pipeline, m37_record_lookup_view,
    m37_record_owned_candidate_materialized, m37_record_process_key, m37_record_ranker_pipeline,
    m37_record_translator, m37_record_userdb_merge, M37MetricsSnapshot,
};
pub use poet::{
    make_sentence, make_sentences, null_grammar_score, Grammar, NullGrammar, SentencePath,
    UpstreamSentenceModel, WordGraph, WordGraphEntry, UPSTREAM_NO_GRAMMAR_PENALTY,
};
pub use punctuation::{PunctuationDefinition, PunctuationProcessor, PunctuationTranslator};
pub use state::{
    AiConfidence, AiContext, AiStagingDebug, Candidate, CandidateSource, CommitRecord, Composition,
    Context, EngineInspectorSnapshot, FilterAuditRecord, PageSnapshot, PrivacyClass, SegmentDebug,
    Snapshot, SpellingAlgebraDebug, Status,
};
pub use translator::{
    EchoTranslator, FoldedSwitchOptions, HistoryTranslator, ReverseLookupTranslator,
    SchemaListTranslator, StaticTableTranslator, SwitchTranslator, SwitchTranslatorSwitch,
    TYPEDUCK_SENTENCE_WORD_PENALTY,
};
pub use userdb::{
    BackdatedScanPolicy, UserDb, UserDbCommitMetadata, UserDbLearnedEntry, UserDbLearningUpdate,
    UserDbLookupRequest, UserDbLookupResult, UserDbValue,
};

pub trait Translator: Send + Sync {
    fn name(&self) -> &'static str;

    fn translate(&self, input: &str) -> Vec<Candidate>;

    fn translate_with_status(&self, input: &str, _status: &Status) -> Vec<Candidate> {
        self.translate(input)
    }

    fn translate_with_state(
        &self,
        input: &str,
        status: &Status,
        _options: &HashMap<String, bool>,
    ) -> Vec<Candidate> {
        self.translate_with_status(input, status)
    }

    fn translate_with_context(
        &self,
        input: &str,
        status: &Status,
        options: &HashMap<String, bool>,
        _context: &Context,
    ) -> Vec<Candidate> {
        self.translate_with_state(input, status, options)
    }

    fn translate_with_context_and_request(
        &self,
        input: &str,
        status: &Status,
        options: &HashMap<String, bool>,
        context: &Context,
        _request: CandidateRequest,
    ) -> TranslationResult {
        TranslationResult::complete(self.translate_with_context(input, status, options, context))
    }

    fn spelling_algebra_debug(&self, _input: &str) -> Option<SpellingAlgebraDebug> {
        None
    }

    fn prediction_weight_threshold(&self) -> Option<f32> {
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateRequest {
    pub limit: Option<usize>,
    pub filter_extended_cjk: bool,
    pub include_debug_full_count: bool,
}

impl CandidateRequest {
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            limit: None,
            filter_extended_cjk: false,
            include_debug_full_count: false,
        }
    }

    #[must_use]
    pub const fn bounded(limit: usize) -> Self {
        Self {
            limit: Some(limit),
            filter_extended_cjk: false,
            include_debug_full_count: false,
        }
    }

    #[must_use]
    pub const fn with_filter_extended_cjk(mut self, filter_extended_cjk: bool) -> Self {
        self.filter_extended_cjk = filter_extended_cjk;
        self
    }

    #[must_use]
    pub const fn with_debug_full_count(mut self, include_debug_full_count: bool) -> Self {
        self.include_debug_full_count = include_debug_full_count;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TranslationResult {
    pub candidates: Vec<Candidate>,
    pub is_complete: bool,
    pub full_count: Option<usize>,
}

impl TranslationResult {
    #[must_use]
    pub fn complete(candidates: Vec<Candidate>) -> Self {
        Self {
            candidates,
            is_complete: true,
            full_count: None,
        }
    }

    #[must_use]
    pub fn bounded(
        candidates: Vec<Candidate>,
        full_count: usize,
        include_full_count: bool,
    ) -> Self {
        Self {
            is_complete: candidates.len() >= full_count,
            candidates,
            full_count: include_full_count.then_some(full_count),
        }
    }
}

pub trait CandidateRanker: Send + Sync {
    fn name(&self) -> &'static str;

    fn try_rerank(&self, context: &Context, candidates: &[Candidate]) -> RerankResult;
}

pub trait CandidateFilter: Send + Sync {
    fn name(&self) -> &'static str;

    fn apply(&self, candidates: &mut Vec<Candidate>);

    fn apply_with_options(
        &self,
        candidates: &mut Vec<Candidate>,
        _options: &HashMap<String, bool>,
    ) {
        self.apply(candidates);
    }

    fn apply_with_context(
        &self,
        candidates: &mut Vec<Candidate>,
        options: &HashMap<String, bool>,
        _context: &Context,
    ) {
        self.apply_with_options(candidates, options);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RerankResult {
    Pending,
    Ready(Vec<Candidate>),
}

pub struct MockAiRanker {
    preferred_texts: Vec<String>,
}

impl MockAiRanker {
    #[must_use]
    pub fn new(preferred_texts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            preferred_texts: preferred_texts.into_iter().map(Into::into).collect(),
        }
    }
}

impl CandidateRanker for MockAiRanker {
    fn name(&self) -> &'static str {
        "mock_ai_ranker"
    }

    fn try_rerank(&self, _context: &Context, candidates: &[Candidate]) -> RerankResult {
        if self.preferred_texts.is_empty() || candidates.is_empty() {
            return RerankResult::Pending;
        }

        let mut ranked = candidates.to_vec();
        ranked.sort_by_key(|candidate| {
            self.preferred_texts
                .iter()
                .position(|text| text == &candidate.text)
                .unwrap_or(self.preferred_texts.len())
        });
        RerankResult::Ready(ranked)
    }
}

#[cfg(test)]
#[path = "tests/facade_tests.rs"]
mod facade_tests;
