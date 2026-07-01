use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::mem;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::comment_format::CommentFormat;
use crate::dictionary::{
    normalize_table_code, CompactTableStore, LookupCandidate, LookupCandidateEntry,
    RimePrismBinPayload, RimePrismRuntimePayload, TableLookup,
};
use crate::filter::contains_extended_cjk;
use crate::poet::{GrammarProvider, SentenceCodeSpan, UpstreamSentenceModel};
use crate::spelling_algebra::{ExpandedSpellingEntry, SpellingAlgebra};
use crate::{
    Candidate, CandidateRequest, CandidateSource, Context, M37SentenceCandidateMetrics,
    MemoryOwnerClass, MemoryOwnerRow, PresetVocabularyEntry, RimeCorrectionEntry,
    RimeToleranceRule, SpellingAlgebraDebug, Status, StorageDiagnosticsRow, TableDictionary,
    TableDictionaryParseError, TableEntry, TranslationResult, Translator,
};

const TYPEDUCK_CORRECTION_CREDIBILITY: f32 = -16.118_095; // log(1e-7)
const TYPEDUCK_CORRECTION_MAX_DISTANCE: usize = 4;
const DEFAULT_SENTENCE_WORD_PENALTY: f32 = 0.0;
const BOUNDED_SENTENCE_MODEL_PAGE_LIMIT: usize = 5;
const MAX_ABBREVIATION_SENTENCE_INPUT_BYTES: usize = 16;
const MAX_ABBREVIATION_SENTENCE_SPAN_BYTES: usize = 6;
const MAX_ABBREVIATION_SENTENCE_CODES_PER_SPAN: usize = 128;
const MAX_ABBREVIATION_SENTENCE_TOTAL_SPANS: usize = 4096;
const MAX_SENTENCE_ALIAS_LOOKUP_BYTES: usize = 12;
const MAX_SENTENCE_ALIAS_LOOKUP_CODES: usize = 64;
const MAX_SENTENCE_CANDIDATES_PER_SPAN: usize = 6;
const MAX_PREFIX_FALLBACK_CANDIDATES: usize = 64;
const MAX_PREFIX_FALLBACK_PENDING_CANDIDATES: usize = 256;
const MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE: usize = 2;
/// Yune-internal heuristic calibrated to the M21 TypeDuck v1.1.2 sentence-composition fixture
/// and the M28 follow-up upstream-Jyutping composition fixture; install only for the
/// jyut6ping3 TypeDuck profile.
pub const TYPEDUCK_SENTENCE_WORD_PENALTY: f32 = 24.0;

#[derive(Clone, Debug, Eq, PartialEq)]
struct LookupCodeSpec {
    code: String,
    lookup_code: String,
    correction_distance: Option<usize>,
    required_syllable_count: Option<usize>,
}

impl LookupCodeSpec {
    fn exact(code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            lookup_code: code.clone(),
            code,
            correction_distance: None,
            required_syllable_count: None,
        }
    }

    fn alias(code: impl Into<String>, lookup_code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            lookup_code: lookup_code.into(),
            correction_distance: None,
            required_syllable_count: None,
        }
    }

    fn correction(code: impl Into<String>, distance: usize) -> Self {
        let code = code.into();
        Self {
            lookup_code: code.clone(),
            code,
            correction_distance: Some(distance),
            required_syllable_count: None,
        }
    }

    fn correction_with_syllable_count(
        code: impl Into<String>,
        distance: usize,
        syllable_count: usize,
    ) -> Self {
        let code = code.into();
        Self {
            lookup_code: code.clone(),
            code,
            correction_distance: Some(distance),
            required_syllable_count: Some(syllable_count),
        }
    }
}

#[derive(Clone)]
struct PendingLookupCandidate {
    entry_code: String,
    lookup_code: String,
    candidate: Candidate,
    correction_distance: Option<usize>,
    spelling_abbreviation: bool,
    limited_prediction: bool,
}

impl PendingLookupCandidate {
    fn raw_quality(&self) -> f32 {
        let mut quality = self.candidate.quality;
        if let Some(distance) = self.correction_distance {
            quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        quality
    }
}

struct PendingLookupCandidateRef<'a> {
    entry_code: Cow<'a, str>,
    lookup_code: &'a str,
    candidate: LookupCandidate<'a>,
    correction_distance: Option<usize>,
    spelling_abbreviation: bool,
    limited_prediction: bool,
    emission_order: usize,
}

struct BoundedLookupRequest<'a> {
    input: &'a str,
    lookup_code: &'a str,
    lookup_specs: &'a [LookupCodeSpec],
    filter_by_charset: bool,
    segment_tags: Option<&'a [String]>,
    limit: usize,
    include_full_count: bool,
}

struct LookupPrefixSpec<'a> {
    input_prefix: &'a str,
    fetch_code: String,
    consumed_lookup_len: usize,
}

fn sentence_piece_quality(raw_quality: f32, word_penalty: f32) -> f32 {
    raw_quality.max(1.0).ln() - word_penalty
}

#[derive(Default)]
pub struct EchoTranslator;

impl Translator for EchoTranslator {
    fn name(&self) -> &'static str {
        "echo_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }
        vec![Candidate {
            text: input.to_owned(),
            comment: "echo".to_owned(),
            preedit: None,
            source: CandidateSource::Echo,
            quality: 0.0,
        }]
    }
}

enum TableStorage {
    Heap(BTreeMap<String, Vec<Candidate>>),
    Compact(Box<CompactTableStore>),
}

struct LookupTimer(Option<Instant>);

impl LookupTimer {
    fn start() -> Self {
        Self(crate::m37_metrics_enabled().then(Instant::now))
    }

    fn elapsed(&self) -> Duration {
        self.0
            .map_or_else(Duration::default, |start| start.elapsed())
    }
}

impl TableStorage {
    fn has_code(&self, code: &str) -> bool {
        match self {
            Self::Heap(entries) => entries.has_code(code),
            Self::Compact(store) => store.has_code(code),
        }
    }

    fn exact_candidates<'a>(
        &'a self,
        code: &'a str,
    ) -> Box<dyn Iterator<Item = LookupCandidate<'a>> + 'a> {
        match self {
            Self::Heap(entries) => Box::new(entries.exact_candidates(code)),
            Self::Compact(store) => Box::new(store.exact_candidates(code)),
        }
    }

    fn prefix_candidates<'a>(
        &'a self,
        prefix: &'a str,
    ) -> Box<dyn Iterator<Item = LookupCandidateEntry<'a>> + 'a> {
        match self {
            Self::Heap(entries) => Box::new(entries.prefix_candidates(prefix)),
            Self::Compact(store) => Box::new(store.prefix_candidates(prefix)),
        }
    }

    fn all_codes(&self) -> Box<dyn Iterator<Item = Cow<'_, str>> + '_> {
        match self {
            Self::Heap(entries) => Box::new(entries.all_codes()),
            Self::Compact(store) => Box::new(store.all_codes()),
        }
    }

    fn code_count(&self) -> usize {
        match self {
            Self::Heap(entries) => entries.len(),
            Self::Compact(store) => store.code_count(),
        }
    }

    fn record_exact_lookup(&self, duration: Duration, candidates: usize) {
        match self {
            Self::Heap(_) => crate::m37_record_heap_exact_lookup(duration, candidates),
            Self::Compact(store) => {
                if store.is_marisa_backed() {
                    crate::m37_record_rsmarisa_exact_lookup(duration, candidates);
                } else {
                    crate::m37_record_no_marisa_compact_exact_lookup(duration, candidates);
                }
            }
        }
    }

    fn record_prefix_lookup(&self, duration: Duration, candidates: usize) {
        match self {
            Self::Heap(_) => crate::m37_record_heap_prefix_lookup(duration, candidates),
            Self::Compact(store) => {
                if store.is_marisa_backed() {
                    crate::m37_record_rsmarisa_prefix_lookup(duration, candidates);
                } else {
                    crate::m37_record_no_marisa_compact_prefix_lookup(duration, candidates);
                }
            }
        }
    }

    fn syllabary_codes(&self) -> Option<&[String]> {
        match self {
            Self::Heap(_) => None,
            Self::Compact(store) => Some(store.syllabary_codes()),
        }
    }

    fn table_entry_iter(&self) -> Box<dyn Iterator<Item = TableEntry> + '_> {
        match self {
            Self::Heap(entries) => Box::new(entries.iter().flat_map(|(code, candidates)| {
                candidates
                    .iter()
                    .map(move |candidate| TableEntry::new(code, &candidate.text, candidate.quality))
            })),
            Self::Compact(store) => Box::new(store.all_codes().flat_map(|code| {
                let code = code.into_owned();
                store
                    .exact_candidates(&code)
                    .map(|candidate| {
                        TableEntry::new(&code, candidate.text(), candidate.raw_quality())
                    })
                    .collect::<Vec<_>>()
            })),
        }
    }

    fn owned_entries(&self) -> Vec<(String, Candidate)> {
        match self {
            Self::Heap(entries) => entries
                .iter()
                .flat_map(|(code, candidates)| {
                    candidates
                        .iter()
                        .map(move |candidate| (code.clone(), candidate.clone()))
                })
                .collect(),
            Self::Compact(store) => store
                .all_codes()
                .flat_map(|code| {
                    let code = code.into_owned();
                    store
                        .exact_candidates(&code)
                        .map(|candidate| {
                            (
                                code.clone(),
                                Candidate {
                                    text: candidate.text().to_owned(),
                                    comment: candidate.raw_comment().to_owned(),
                                    preedit: None,
                                    source: candidate.source_hint(),
                                    quality: candidate.raw_quality(),
                                },
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
        }
    }

    fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        match self {
            Self::Heap(entries) => vec![MemoryOwnerRow::new(
                "translator.entries_by_code",
                MemoryOwnerClass::HeapOwnedGuarded,
                estimate_entries_by_code_bytes(entries),
                entries.values().map(Vec::len).sum(),
                "BTreeMap<String, Vec<Candidate>>",
                "heap dictionary rows used by source-YAML and small test translators",
            )],
            Self::Compact(store) => {
                let mut rows = vec![MemoryOwnerRow::new(
                    "translator.entries_by_code",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "compact_table",
                    "compact storage path does not retain a translator BTreeMap",
                )];
                rows.extend(store.memory_owner_rows());
                rows
            }
        }
    }

    fn storage_diagnostics(&self) -> Vec<StorageDiagnosticsRow> {
        match self {
            Self::Heap(entries) => vec![StorageDiagnosticsRow::new(
                "translator.entries_by_code",
                "owned_heap",
                "owned_heap",
                false,
                0,
                entries.values().map(Vec::len).sum(),
            )],
            Self::Compact(store) => vec![StorageDiagnosticsRow::new(
                "compact_table.storage",
                store.storage_label(),
                store.mapping_mode(),
                store.is_marisa_backed(),
                store.byte_source_len(),
                store.stored_entry_count(),
            )],
        }
    }
}

enum NormalCodeIndex {
    Eager(HashSet<String>),
    StorageBackedCompact,
}

impl NormalCodeIndex {
    fn eager(codes: HashSet<String>) -> Self {
        Self::Eager(codes)
    }

    fn contains(&self, storage: &TableStorage, code: &str) -> bool {
        match self {
            Self::Eager(codes) => codes.contains(code),
            Self::StorageBackedCompact => storage.has_code(code),
        }
    }

    fn memory_owner_row(&self, storage: &TableStorage) -> MemoryOwnerRow {
        match self {
            Self::Eager(codes) => MemoryOwnerRow::new(
                "translator.normal_codes",
                MemoryOwnerClass::HeapOwnedReducible,
                estimate_string_hash_set_bytes(codes),
                codes.len(),
                "HashSet<String>",
                "normal code membership retained for spelling correction gating",
            ),
            Self::StorageBackedCompact => MemoryOwnerRow::new(
                "translator.normal_codes",
                MemoryOwnerClass::Shared,
                0,
                storage.code_count(),
                "compact_table.has_code",
                "normal code membership delegated to compact storage lookup",
            ),
        }
    }
}

fn estimate_string_hash_set_bytes(values: &HashSet<String>) -> usize {
    mem::size_of::<HashSet<String>>()
        .saturating_add(values.capacity().saturating_mul(mem::size_of::<String>()))
        .saturating_add(values.iter().map(String::capacity).sum::<usize>())
}

fn estimate_entries_by_code_bytes(entries: &BTreeMap<String, Vec<Candidate>>) -> usize {
    mem::size_of::<BTreeMap<String, Vec<Candidate>>>().saturating_add(
        entries
            .iter()
            .map(|(code, candidates)| {
                code.capacity()
                    .saturating_add(mem::size_of::<(String, Vec<Candidate>)>())
                    .saturating_add(
                        candidates
                            .capacity()
                            .saturating_mul(mem::size_of::<Candidate>()),
                    )
                    .saturating_add(
                        candidates
                            .iter()
                            .map(estimate_candidate_bytes)
                            .sum::<usize>(),
                    )
            })
            .sum::<usize>(),
    )
}

fn estimate_candidate_bytes(candidate: &Candidate) -> usize {
    candidate
        .text
        .capacity()
        .saturating_add(candidate.comment.capacity())
        .saturating_add(candidate.preedit.as_ref().map_or(0, String::capacity))
}

fn estimate_table_entries_bytes(entries: &[TableEntry]) -> usize {
    mem::size_of_val(entries).saturating_add(
        entries
            .iter()
            .map(|entry| entry.code.capacity().saturating_add(entry.text.capacity()))
            .sum::<usize>(),
    )
}

fn estimate_string_vec_hash_map_bytes(values: &HashMap<String, Vec<String>>) -> usize {
    mem::size_of::<HashMap<String, Vec<String>>>()
        .saturating_add(
            values
                .capacity()
                .saturating_mul(mem::size_of::<(String, Vec<String>)>()),
        )
        .saturating_add(
            values
                .iter()
                .map(|(key, list)| {
                    key.capacity()
                        .saturating_add(list.capacity().saturating_mul(mem::size_of::<String>()))
                        .saturating_add(list.iter().map(String::capacity).sum::<usize>())
                })
                .sum::<usize>(),
        )
}

pub struct StaticTableTranslator {
    source_entries: Option<Vec<(String, Candidate)>>,
    storage: TableStorage,
    prism_payload: Option<RimePrismRuntimePayload>,
    spelling_abbreviation_entries: HashSet<(String, String, String)>,
    normal_codes: NormalCodeIndex,
    enable_completion: bool,
    enable_correction: bool,
    dynamic_correction_lookup: bool,
    enable_charset_filter: bool,
    enable_sentence: bool,
    sentence_over_completion: bool,
    tags: Vec<String>,
    delimiters: String,
    initial_quality: f32,
    comment_format: CommentFormat,
    preedit_format: CommentFormat,
    dictionary_exclude: HashSet<String>,
    corrections: Vec<RimeCorrectionEntry>,
    tolerance_rules: Vec<RimeToleranceRule>,
    combine_candidates: bool,
    prefix: String,
    suffix: String,
    show_full_code: bool,
    single_letter_sentence_guard_enabled: bool,
    prediction_weight_threshold: Option<f32>,
    prediction_never_first: bool,
    prediction_candidate_limit: Option<usize>,
    prefix_fallback: bool,
    sentence_word_penalty: f32,
    spelling_algebra_formulas: Vec<String>,
    preset_vocabulary: Vec<PresetVocabularyEntry>,
    abbreviation_preset_vocabulary: Vec<PresetVocabularyEntry>,
    upstream_sentence_grammar: GrammarProvider,
    upstream_sentence_model: Option<UpstreamSentenceModel>,
}

impl StaticTableTranslator {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        let entries: Vec<(String, Candidate)> = entries
            .into_iter()
            .map(|(code, text)| {
                let code = code.into();
                let text = text.into();
                (
                    code.clone(),
                    Candidate {
                        text,
                        comment: code,
                        preedit: None,
                        source: CandidateSource::Table,
                        quality: 0.0,
                    },
                )
            })
            .collect();
        let entries_by_code = entries_by_code(&entries);
        let normal_codes = NormalCodeIndex::eager(normal_codes(&entries));
        Self {
            source_entries: Some(entries),
            storage: TableStorage::Heap(entries_by_code),
            prism_payload: None,
            spelling_abbreviation_entries: HashSet::new(),
            normal_codes,
            enable_completion: false,
            enable_correction: false,
            dynamic_correction_lookup: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_over_completion: false,
            tags: vec!["abc".to_owned()],
            delimiters: " ".to_owned(),
            initial_quality: 0.0,
            comment_format: CommentFormat::default(),
            preedit_format: CommentFormat::default(),
            dictionary_exclude: HashSet::new(),
            corrections: Vec::new(),
            tolerance_rules: Vec::new(),
            combine_candidates: false,
            prefix: String::new(),
            suffix: String::new(),
            show_full_code: true,
            single_letter_sentence_guard_enabled: false,
            prediction_weight_threshold: None,
            prediction_never_first: false,
            prediction_candidate_limit: None,
            prefix_fallback: false,
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary: Vec::new(),
            abbreviation_preset_vocabulary: Vec::new(),
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_model: None,
        }
    }

    #[must_use]
    pub fn from_dictionary(dictionary: TableDictionary) -> Self {
        let preset_vocabulary = dictionary.preset_vocabulary_entries().to_vec();
        let abbreviation_preset_vocabulary: Vec<PresetVocabularyEntry> = Vec::new();
        let corrections = dictionary.corrections().to_vec();
        let tolerance_rules = dictionary.tolerance_rules().to_vec();
        let entries: Vec<(String, Candidate)> = dictionary
            .entries
            .into_iter()
            .map(|entry| {
                let candidate = Candidate {
                    text: entry.text,
                    comment: entry.code.clone(),
                    preedit: None,
                    source: CandidateSource::Table,
                    quality: entry.weight,
                };
                (entry.code, candidate)
            })
            .collect();
        let entries_by_code = entries_by_code(&entries);
        let normal_codes = NormalCodeIndex::eager(normal_codes(&entries));
        Self {
            source_entries: Some(entries),
            storage: TableStorage::Heap(entries_by_code),
            prism_payload: None,
            spelling_abbreviation_entries: HashSet::new(),
            normal_codes,
            enable_completion: false,
            enable_correction: false,
            dynamic_correction_lookup: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_over_completion: false,
            tags: vec!["abc".to_owned()],
            delimiters: " ".to_owned(),
            initial_quality: 0.0,
            comment_format: CommentFormat::default(),
            preedit_format: CommentFormat::default(),
            dictionary_exclude: HashSet::new(),
            corrections,
            tolerance_rules,
            combine_candidates: false,
            prefix: String::new(),
            suffix: String::new(),
            show_full_code: true,
            single_letter_sentence_guard_enabled: false,
            prediction_weight_threshold: None,
            prediction_never_first: false,
            prediction_candidate_limit: None,
            prefix_fallback: false,
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_model: None,
        }
    }

    #[must_use]
    pub fn from_compact_dictionary(
        dictionary: TableDictionary,
        prism_payload: Option<RimePrismBinPayload>,
    ) -> Self {
        let preset_vocabulary = dictionary.preset_vocabulary_entries().to_vec();
        let abbreviation_preset_vocabulary: Vec<PresetVocabularyEntry> = Vec::new();
        let corrections = dictionary.corrections().to_vec();
        let tolerance_rules = dictionary.tolerance_rules().to_vec();
        let normal_codes = NormalCodeIndex::StorageBackedCompact;
        Self {
            source_entries: None,
            storage: TableStorage::Compact(Box::new(CompactTableStore::from_dictionary(
                dictionary,
            ))),
            prism_payload: prism_payload.map(RimePrismRuntimePayload::from),
            spelling_abbreviation_entries: HashSet::new(),
            normal_codes,
            enable_completion: false,
            enable_correction: false,
            dynamic_correction_lookup: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_over_completion: false,
            tags: vec!["abc".to_owned()],
            delimiters: " ".to_owned(),
            initial_quality: 0.0,
            comment_format: CommentFormat::default(),
            preedit_format: CommentFormat::default(),
            dictionary_exclude: HashSet::new(),
            corrections,
            tolerance_rules,
            combine_candidates: false,
            prefix: String::new(),
            suffix: String::new(),
            show_full_code: true,
            single_letter_sentence_guard_enabled: false,
            prediction_weight_threshold: None,
            prediction_never_first: false,
            prediction_candidate_limit: None,
            prefix_fallback: false,
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_model: None,
        }
    }

    #[must_use]
    pub fn from_compact_table_store(
        store: CompactTableStore,
        prism_payload: Option<RimePrismBinPayload>,
    ) -> Self {
        Self::from_compact_table_store_with_prism_runtime(
            store,
            prism_payload.map(RimePrismRuntimePayload::from),
        )
    }

    #[must_use]
    pub fn from_compact_table_store_with_prism_runtime(
        store: CompactTableStore,
        prism_payload: Option<RimePrismRuntimePayload>,
    ) -> Self {
        let advanced = store.advanced_data();
        let preset_vocabulary = advanced.preset_vocabulary.clone();
        let abbreviation_preset_vocabulary: Vec<PresetVocabularyEntry> = Vec::new();
        let corrections = advanced.corrections.clone();
        let tolerance_rules = advanced.tolerance_rules.clone();
        crate::memory_probe_mark(
            "m47:compact_table:after_storage_backed_normal_codes:index=compact_table_has_code",
        );
        let normal_codes = NormalCodeIndex::StorageBackedCompact;
        Self {
            source_entries: None,
            storage: TableStorage::Compact(Box::new(store)),
            prism_payload,
            spelling_abbreviation_entries: HashSet::new(),
            normal_codes,
            enable_completion: false,
            enable_correction: false,
            dynamic_correction_lookup: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_over_completion: false,
            tags: vec!["abc".to_owned()],
            delimiters: " ".to_owned(),
            initial_quality: 0.0,
            comment_format: CommentFormat::default(),
            preedit_format: CommentFormat::default(),
            dictionary_exclude: HashSet::new(),
            corrections,
            tolerance_rules,
            combine_candidates: false,
            prefix: String::new(),
            suffix: String::new(),
            show_full_code: true,
            single_letter_sentence_guard_enabled: false,
            prediction_weight_threshold: None,
            prediction_never_first: false,
            prediction_candidate_limit: None,
            prefix_fallback: false,
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_model: None,
        }
    }

    #[must_use]
    pub fn with_completion(mut self, enable_completion: bool) -> Self {
        self.enable_completion = enable_completion;
        self
    }

    #[must_use]
    pub fn with_correction(mut self, enable_correction: bool) -> Self {
        self.enable_correction = enable_correction;
        self
    }

    #[must_use]
    pub fn with_dynamic_correction_lookup(mut self, dynamic_correction_lookup: bool) -> Self {
        self.dynamic_correction_lookup = dynamic_correction_lookup;
        self
    }

    #[must_use]
    pub fn with_charset_filter(mut self, enable_charset_filter: bool) -> Self {
        self.enable_charset_filter = enable_charset_filter;
        self
    }

    #[must_use]
    pub fn with_sentence(mut self, enable_sentence: bool) -> Self {
        self.enable_sentence = enable_sentence;
        self
    }

    #[must_use]
    pub fn with_sentence_word_penalty(mut self, sentence_word_penalty: f32) -> Self {
        self.sentence_word_penalty = sentence_word_penalty;
        self
    }

    #[must_use]
    pub fn with_sentence_over_completion(mut self, sentence_over_completion: bool) -> Self {
        self.sentence_over_completion = sentence_over_completion;
        self
    }

    #[must_use]
    pub fn with_delimiters(mut self, delimiters: impl Into<String>) -> Self {
        self.delimiters = delimiters.into();
        if self.delimiters.is_empty() {
            self.delimiters = " ".to_owned();
        }
        self
    }

    #[must_use]
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        if self.tags.is_empty() {
            self.tags.push("abc".to_owned());
        }
        self
    }

    #[must_use]
    pub fn with_initial_quality(mut self, initial_quality: f32) -> Self {
        self.initial_quality = initial_quality;
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self
    }

    #[must_use]
    pub fn with_preedit_format(mut self, formulas: &[String]) -> Self {
        self.preedit_format = CommentFormat::parse(formulas);
        self
    }

    #[must_use]
    pub fn with_dictionary_exclude(
        mut self,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.dictionary_exclude = words.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn with_combine_candidates(mut self, combine_candidates: bool) -> Self {
        self.combine_candidates = combine_candidates;
        self
    }

    #[must_use]
    pub fn with_affix(mut self, prefix: impl Into<String>, suffix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self.suffix = suffix.into();
        self
    }

    #[must_use]
    pub fn with_show_full_code(mut self, show_full_code: bool) -> Self {
        self.show_full_code = show_full_code;
        self
    }

    #[must_use]
    pub fn with_prediction_weight_threshold(mut self, threshold: f32) -> Self {
        self.prediction_weight_threshold = Some(threshold);
        self
    }

    #[must_use]
    pub fn with_prediction_never_first(mut self, prediction_never_first: bool) -> Self {
        self.prediction_never_first = prediction_never_first;
        self
    }

    #[must_use]
    pub fn with_prediction_candidate_limit(mut self, limit: usize) -> Self {
        self.prediction_candidate_limit = Some(limit);
        self
    }

    #[must_use]
    pub fn with_prefix_fallback(mut self, prefix_fallback: bool) -> Self {
        self.prefix_fallback = prefix_fallback;
        self
    }

    #[must_use]
    pub fn with_preset_vocabulary(
        mut self,
        vocabulary: impl IntoIterator<Item = PresetVocabularyEntry>,
    ) -> Self {
        self.preset_vocabulary = vocabulary.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_abbreviation_preset_vocabulary(
        mut self,
        vocabulary: impl IntoIterator<Item = PresetVocabularyEntry>,
    ) -> Self {
        self.abbreviation_preset_vocabulary = vocabulary.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_upstream_sentence_model(mut self, max_candidates: usize) -> Self {
        let abbreviation_vocabulary = if self.abbreviation_preset_vocabulary.is_empty() {
            self.preset_vocabulary.as_slice()
        } else {
            self.abbreviation_preset_vocabulary.as_slice()
        };
        let build_abbreviation_model = matches!(self.storage, TableStorage::Compact(_))
            && self.prism_payload.is_some()
            && self.single_letter_sentence_guard_enabled
            && !abbreviation_vocabulary.is_empty();
        let model = if let Some(entries) = self.source_entries.take() {
            let table_entries = entries
                .into_iter()
                .map(|(code, candidate)| TableEntry::new(code, candidate.text, candidate.quality))
                .collect::<Vec<_>>();
            UpstreamSentenceModel::from_table_entries(
                table_entries,
                &self.preset_vocabulary,
                max_candidates,
            )
        } else {
            let full_pinyin_vocabulary = self.preset_vocabulary.as_slice();
            if build_abbreviation_model {
                UpstreamSentenceModel::from_table_entries_with_abbreviation_vocabulary(
                    self.storage.table_entry_iter(),
                    full_pinyin_vocabulary,
                    abbreviation_vocabulary,
                    max_candidates,
                )
            } else {
                UpstreamSentenceModel::from_table_entries(
                    self.storage.table_entry_iter(),
                    full_pinyin_vocabulary,
                    max_candidates,
                )
            }
        };
        self.upstream_sentence_model =
            Some(model.with_grammar(self.upstream_sentence_grammar.clone()));
        self
    }

    #[must_use]
    pub fn with_upstream_sentence_grammar(mut self, grammar: impl Into<GrammarProvider>) -> Self {
        let grammar = grammar.into();
        self.upstream_sentence_grammar = grammar.clone();
        if let Some(model) = self.upstream_sentence_model.take() {
            self.upstream_sentence_model = Some(model.with_grammar(grammar));
        }
        self
    }

    #[must_use]
    pub fn with_corrections(
        mut self,
        corrections: impl IntoIterator<Item = RimeCorrectionEntry>,
    ) -> Self {
        self.corrections = corrections.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_tolerance_rules(
        mut self,
        tolerance_rules: impl IntoIterator<Item = RimeToleranceRule>,
    ) -> Self {
        self.tolerance_rules = tolerance_rules.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_spelling_algebra(mut self, formulas: &[String]) -> Self {
        self.spelling_algebra_formulas = formulas.to_vec();
        let algebra = SpellingAlgebra::parse(formulas);
        if !algebra.is_empty() {
            if matches!(self.storage, TableStorage::Compact(_)) && self.prism_payload.is_some() {
                self.single_letter_sentence_guard_enabled = formulas
                    .iter()
                    .any(|formula| formula_is_abbreviation(formula));
                return self;
            }
            let source_entries = self
                .source_entries
                .take()
                .unwrap_or_else(|| self.storage.owned_entries());
            let (entries, normal_codes, has_single_letter_abbreviations) =
                algebra.expand_entries_with_normal_codes(source_entries);
            self.spelling_abbreviation_entries = spelling_abbreviation_entries(&entries);
            let entries = entries
                .into_iter()
                .map(|entry| (entry.code, entry.candidate))
                .collect::<Vec<_>>();
            self.storage = TableStorage::Heap(entries_by_code_from_entries(entries));
            self.normal_codes = NormalCodeIndex::eager(normal_codes);
            self.single_letter_sentence_guard_enabled = has_single_letter_abbreviations;
        } else if self.source_entries.is_some() && !self.storage.has_code("") {
            let source_entries = self.source_entries.take().unwrap_or_default();
            if !source_entries.is_empty() {
                self.storage = TableStorage::Heap(entries_by_code_from_entries(source_entries));
            }
        }
        self
    }

    fn lookup_code<'a>(&self, input: &'a str) -> Option<&'a str> {
        let mut code = if self.prefix.is_empty() {
            input
        } else {
            input.strip_prefix(&self.prefix)?
        };
        if !self.suffix.is_empty() {
            code = code.strip_suffix(&self.suffix).unwrap_or(code);
        }
        Some(code.trim_end_matches(|ch| self.delimiters.contains(ch)))
    }

    fn accepts_default_segment(&self) -> bool {
        self.tags.iter().any(|tag| tag == "abc")
    }

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        self.tags
            .iter()
            .any(|tag| segment_tags.iter().any(|segment_tag| segment_tag == tag))
    }

    fn bounds_compact_fallback_expansion(&self) -> bool {
        matches!(self.storage, TableStorage::Compact(_)) && self.prism_payload.is_some()
    }

    fn expanded_lookup_specs(&self, lookup_code: &str) -> Vec<LookupCodeSpec> {
        let mut specs = vec![LookupCodeSpec::exact(lookup_code)];
        let has_exact_lookup = self.storage.has_code(lookup_code);
        if let (Some(prism), Some(syllabary_codes)) =
            (self.prism_payload.as_ref(), self.storage.syllabary_codes())
        {
            let track_b_short_prefix = self.uses_m44_track_b_short_prefix_lookup(lookup_code);
            let prism_start = crate::m37_metrics_enabled().then(Instant::now);
            let lookups = prism.lookup_canonical_codes(lookup_code, syllabary_codes);
            if let Some(start) = prism_start {
                let elapsed = start.elapsed();
                crate::m37_record_prism_lookup(elapsed, lookups.len());
                if self.dynamic_correction_lookup {
                    crate::m37_record_track_b_spelling_expansion(elapsed, lookups.len());
                }
            }
            for lookup in lookups {
                if track_b_short_prefix && lookup.code != lookup_code {
                    continue;
                }
                if !lookup.correction
                    && !specs.iter().any(|spec| spec.code == lookup.code)
                    && self.storage.has_code(lookup.code)
                {
                    specs.push(LookupCodeSpec::alias(
                        lookup.code.to_owned(),
                        lookup_code.to_owned(),
                    ));
                }
            }
        }
        let allow_dynamic_near_lookup = self.dynamic_correction_lookup
            && (self.enable_correction || (has_exact_lookup && lookup_code.starts_with('m')));
        let dynamic_syllable_count = (!self.enable_correction && has_exact_lookup)
            .then(|| self.exact_lookup_min_syllable_count(lookup_code))
            .flatten();
        if self.enable_correction || allow_dynamic_near_lookup {
            let correction_start = crate::m37_metrics_enabled().then(Instant::now);
            let mut dynamic_codes_considered = 0;
            let mut corrections = Vec::new();
            if self.enable_correction {
                for correction in &self.corrections {
                    if correction.observed_input != lookup_code
                        || !self.normal_code_contains(&correction.canonical_code)
                    {
                        continue;
                    }
                    let distance = typeduck_restricted_distance(
                        &correction.canonical_code,
                        lookup_code,
                        TYPEDUCK_CORRECTION_MAX_DISTANCE,
                    );
                    if distance == 0 || distance > TYPEDUCK_CORRECTION_MAX_DISTANCE {
                        continue;
                    }
                    corrections.push((correction.canonical_code.clone(), distance));
                }
            }
            if allow_dynamic_near_lookup {
                for canonical_code in self.storage.all_codes() {
                    dynamic_codes_considered += 1;
                    let canonical_code = canonical_code.into_owned();
                    if canonical_code == lookup_code {
                        continue;
                    }
                    if typeduck_length_distance_lower_bound(&canonical_code, lookup_code)
                        > TYPEDUCK_CORRECTION_MAX_DISTANCE
                    {
                        continue;
                    }
                    if !self.enable_correction
                        && has_exact_lookup
                        && !lookup_code.starts_with(&canonical_code)
                    {
                        continue;
                    }
                    if !self.lookup_code_has_non_abbreviation_candidate(&canonical_code) {
                        continue;
                    }
                    if dynamic_syllable_count.is_some_and(|syllable_count| {
                        !self.lookup_code_has_syllable_count(&canonical_code, syllable_count)
                    }) {
                        continue;
                    }
                    let distance = typeduck_restricted_distance(
                        &canonical_code,
                        lookup_code,
                        TYPEDUCK_CORRECTION_MAX_DISTANCE,
                    );
                    if distance == 0 || distance > TYPEDUCK_CORRECTION_MAX_DISTANCE {
                        continue;
                    }
                    corrections.push((canonical_code, distance));
                }
            }
            let correction_candidates = corrections.len();
            if let Some(min_distance) = corrections.iter().map(|(_, distance)| *distance).min() {
                for (code, distance) in corrections {
                    if distance == min_distance && !specs.iter().any(|spec| spec.code == code) {
                        if let Some(syllable_count) = dynamic_syllable_count {
                            specs.push(LookupCodeSpec::correction_with_syllable_count(
                                code,
                                distance,
                                syllable_count,
                            ));
                        } else {
                            specs.push(LookupCodeSpec::correction(code, distance));
                        }
                    }
                }
            }
            if let Some(start) = correction_start {
                crate::m37_record_dynamic_correction(
                    start.elapsed(),
                    dynamic_codes_considered,
                    correction_candidates,
                );
            }
        }
        for rule in &self.tolerance_rules {
            if rule.near_code == lookup_code {
                for candidate_code in &rule.candidate_codes {
                    if !specs.iter().any(|spec| &spec.code == candidate_code) {
                        specs.push(LookupCodeSpec::exact(candidate_code));
                    }
                }
            }
        }
        specs
    }

    fn sentence_lookup_specs(&self, lookup_code: &str) -> Vec<LookupCodeSpec> {
        let mut specs = vec![LookupCodeSpec::exact(lookup_code)];
        if lookup_code.len() > MAX_SENTENCE_ALIAS_LOOKUP_BYTES {
            return specs;
        }
        let (Some(prism), Some(syllabary_codes)) =
            (self.prism_payload.as_ref(), self.storage.syllabary_codes())
        else {
            return specs;
        };
        let prism_start = crate::m37_metrics_enabled().then(Instant::now);
        let lookups = prism.lookup_canonical_codes_with_limit(
            lookup_code,
            syllabary_codes,
            MAX_SENTENCE_ALIAS_LOOKUP_CODES,
        );
        if let Some(start) = prism_start {
            crate::m37_record_prism_lookup(start.elapsed(), lookups.len());
        }
        for lookup in lookups {
            if lookup.correction
                || specs.iter().any(|spec| spec.code == lookup.code)
                || !self.storage.has_code(lookup.code)
            {
                continue;
            }
            specs.push(LookupCodeSpec::alias(
                lookup.code.to_owned(),
                lookup_code.to_owned(),
            ));
        }
        specs
    }

    fn abbreviation_sentence_candidates(
        &self,
        model: &UpstreamSentenceModel,
        input: &str,
        limit: usize,
        filter_by_charset: bool,
    ) -> Vec<Candidate> {
        let Some((spans, preedit)) = self.abbreviation_sentence_spans(model, input) else {
            return Vec::new();
        };
        let format_start = crate::m37_metrics_enabled().then(Instant::now);
        let mut candidates = model
            .candidates_for_code_spans_with_limit(input, &spans, limit)
            .into_iter()
            .filter(|candidate| !filter_by_charset || !contains_extended_cjk(&candidate.text))
            .collect::<Vec<_>>();
        for candidate in &mut candidates {
            candidate.preedit = Some(preedit.clone());
        }
        if let Some(start) = format_start {
            crate::m37_record_abbreviation_candidate_format(start.elapsed());
        }
        candidates
    }

    fn abbreviation_sentence_spans(
        &self,
        model: &UpstreamSentenceModel,
        input: &str,
    ) -> Option<(Vec<SentenceCodeSpan>, String)> {
        if !self.single_letter_sentence_guard_enabled
            || input.is_empty()
            || input.len() > MAX_ABBREVIATION_SENTENCE_INPUT_BYTES
            || !input.is_ascii()
        {
            return None;
        }
        let prism = self.prism_payload.as_ref()?;
        let syllabary_codes = self.storage.syllabary_codes()?;
        let discovery_start = crate::m37_metrics_enabled().then(Instant::now);
        let mut candidates_considered = 0usize;
        let mut codes_emitted = 0usize;
        let boundaries = input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
            .collect::<Vec<_>>();
        let mut spans = Vec::new();
        let mut saw_abbreviation = false;
        for (start_index, start) in boundaries.iter().copied().enumerate() {
            if start >= input.len() {
                continue;
            }
            for end in boundaries.iter().copied().skip(start_index + 1) {
                if end - start > MAX_ABBREVIATION_SENTENCE_SPAN_BYTES {
                    break;
                }
                let spelling = &input[start..end];
                let prism_start = crate::m37_metrics_enabled().then(Instant::now);
                let lookups = prism.lookup_canonical_codes(spelling, syllabary_codes);
                if let Some(start) = prism_start {
                    crate::m37_record_prism_lookup(start.elapsed(), lookups.len());
                }
                let mut codes = Vec::new();
                for lookup in lookups {
                    candidates_considered += 1;
                    if lookup.correction || !(lookup.abbreviation || lookup.code == spelling) {
                        continue;
                    }
                    let has_code_start = crate::m37_metrics_enabled().then(Instant::now);
                    let has_code = model.has_code(lookup.code);
                    if let Some(start) = has_code_start {
                        crate::m37_record_abbreviation_model_has_code(start.elapsed());
                    }
                    if !has_code {
                        continue;
                    }
                    if lookup.abbreviation {
                        saw_abbreviation = true;
                    }
                    codes.push(lookup.code.to_owned());
                }
                codes.sort();
                codes.dedup();
                codes.truncate(MAX_ABBREVIATION_SENTENCE_CODES_PER_SPAN);
                for code in codes {
                    spans.push(SentenceCodeSpan::new(start, end, code));
                    codes_emitted += 1;
                    if spans.len() >= MAX_ABBREVIATION_SENTENCE_TOTAL_SPANS {
                        break;
                    }
                }
                if spans.len() >= MAX_ABBREVIATION_SENTENCE_TOTAL_SPANS {
                    break;
                }
            }
            if spans.len() >= MAX_ABBREVIATION_SENTENCE_TOTAL_SPANS {
                break;
            }
        }
        let result = if !saw_abbreviation || spans.is_empty() {
            None
        } else {
            let preedit_start = crate::m37_metrics_enabled().then(Instant::now);
            let preedit = abbreviation_preedit_from_spans(input, &boundaries, &spans);
            if let Some(start) = preedit_start {
                crate::m37_record_abbreviation_preedit_format(start.elapsed());
            }
            preedit.map(|preedit| (spans, preedit))
        };
        if let Some(start) = discovery_start {
            crate::m37_record_abbreviation_span_discovery(
                start.elapsed(),
                candidates_considered,
                codes_emitted,
            );
        }
        result
    }

    fn exact_lookup_min_syllable_count(&self, lookup_code: &str) -> Option<usize> {
        self.storage
            .exact_candidates(lookup_code)
            .filter_map(|candidate| {
                raw_candidate_syllable_count(candidate.raw_comment(), candidate.text())
            })
            .min()
    }

    fn lookup_code_has_syllable_count(&self, lookup_code: &str, syllable_count: usize) -> bool {
        self.storage.exact_candidates(lookup_code).any(|candidate| {
            raw_candidate_syllable_count(candidate.raw_comment(), candidate.text())
                == Some(syllable_count)
        })
    }

    fn lookup_code_has_non_abbreviation_candidate(&self, lookup_code: &str) -> bool {
        self.storage
            .exact_candidates(lookup_code)
            .any(|candidate| !self.is_spelling_abbreviation_view(lookup_code, &candidate))
    }

    fn is_dictionary_text_allowed(&self, text: &str) -> bool {
        self.dictionary_exclude.is_empty() || !self.dictionary_exclude.contains(text)
    }

    fn is_limited_prediction_view(
        &self,
        lookup_code: &str,
        candidate: &LookupCandidate<'_>,
    ) -> bool {
        self.prediction_candidate_limit.is_some()
            && complete_syllable_prefix_count(candidate.raw_comment(), lookup_code).is_some()
    }

    fn is_completion_candidate_view_allowed(
        &self,
        lookup_has_exact_candidates: bool,
        limited_prediction: bool,
        candidate: &LookupCandidate<'_>,
    ) -> bool {
        if self.prediction_candidate_limit.is_some()
            && lookup_has_exact_candidates
            && !limited_prediction
        {
            return false;
        }
        let threshold_applies = limited_prediction || self.prediction_candidate_limit.is_none();
        !threshold_applies
            || self
                .prediction_weight_threshold
                .map_or(true, |threshold| candidate.raw_quality() >= threshold)
    }

    fn is_spelling_abbreviation_view(&self, code: &str, candidate: &LookupCandidate<'_>) -> bool {
        if self.spelling_abbreviation_entries.is_empty() {
            return false;
        }
        self.spelling_abbreviation_entries.contains(&(
            code.to_owned(),
            candidate.text().to_owned(),
            candidate.raw_comment().to_owned(),
        ))
    }

    fn lookup_candidate_order(
        &self,
        left: &PendingLookupCandidate,
        right: &PendingLookupCandidate,
    ) -> Ordering {
        self.lookup_candidate_category(left)
            .cmp(&self.lookup_candidate_category(right))
            .then_with(|| {
                right
                    .raw_quality()
                    .partial_cmp(&left.raw_quality())
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.entry_code.cmp(&right.entry_code))
            .then_with(|| left.candidate.text.cmp(&right.candidate.text))
    }

    fn lookup_candidate_category(&self, candidate: &PendingLookupCandidate) -> u8 {
        if candidate.spelling_abbreviation {
            1
        } else if candidate.entry_code.as_ref() != candidate.lookup_code
            && !candidate.limited_prediction
        {
            2
        } else {
            0
        }
    }

    fn enforce_prediction_never_first(&self, candidates: &mut [Candidate]) {
        if !self.prediction_never_first {
            return;
        }
        let Some(best_non_prediction_quality) = candidates
            .iter()
            .filter(|candidate| candidate.source != CandidateSource::Completion)
            .map(|candidate| candidate.quality)
            .max_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal))
        else {
            return;
        };
        let capped_quality =
            best_non_prediction_quality - 1.0e-6 * best_non_prediction_quality.abs().max(1.0);
        for candidate in candidates {
            if candidate.source == CandidateSource::Completion
                && candidate.quality >= capped_quality
            {
                candidate.quality = capped_quality;
            }
        }
    }

    fn candidate_for_lookup(
        &self,
        entry_code: &str,
        candidate: &Candidate,
        lookup_code: &str,
        correction_distance: Option<usize>,
    ) -> Candidate {
        self.format_candidate_for_lookup(
            entry_code,
            candidate.clone(),
            lookup_code,
            correction_distance,
        )
    }

    fn candidate_for_lookup_view(
        &self,
        entry_code: &str,
        candidate: &LookupCandidate<'_>,
        lookup_code: &str,
        correction_distance: Option<usize>,
    ) -> Candidate {
        let materialize_start = crate::m37_metrics_enabled().then(Instant::now);
        let text = candidate.text().to_owned();
        let source_hint = candidate.source_hint();
        let mut raw_quality = candidate.raw_quality();
        if let Some(start) = materialize_start {
            crate::m37_record_owned_candidate_materialization(start.elapsed());
        }

        let comment_code = if self.show_full_code {
            candidate.raw_comment().to_owned()
        } else if entry_code == lookup_code {
            String::new()
        } else {
            entry_code
                .strip_prefix(lookup_code)
                .filter(|suffix| !suffix.is_empty())
                .map_or_else(
                    || candidate.raw_comment().to_owned(),
                    |suffix| format!("~{suffix}"),
                )
        };
        let comment = if comment_code.is_empty() {
            String::new()
        } else {
            self.comment_format.apply(&comment_code)
        };
        let preedit = if entry_code == lookup_code {
            let preedit = self.preedit_format.apply(lookup_code);
            (preedit != lookup_code).then_some(preedit)
        } else {
            None
        };
        if let Some(distance) = correction_distance {
            raw_quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        let mut source = source_hint;
        let mut quality = raw_quality.exp() + self.initial_quality;
        if entry_code != lookup_code {
            source = CandidateSource::Completion;
            quality -= 1.0;
        }

        Candidate {
            text,
            comment,
            preedit,
            source,
            quality,
        }
    }

    fn format_candidate_for_lookup(
        &self,
        entry_code: &str,
        mut candidate: Candidate,
        lookup_code: &str,
        correction_distance: Option<usize>,
    ) -> Candidate {
        let comment_code = if self.show_full_code {
            candidate.comment.clone()
        } else if entry_code == lookup_code {
            String::new()
        } else {
            entry_code
                .strip_prefix(lookup_code)
                .filter(|suffix| !suffix.is_empty())
                .map_or_else(|| candidate.comment.clone(), |suffix| format!("~{suffix}"))
        };
        candidate.comment = if comment_code.is_empty() {
            String::new()
        } else {
            self.comment_format.apply(&comment_code)
        };
        if entry_code == lookup_code {
            let preedit = self.preedit_format.apply(lookup_code);
            if preedit != lookup_code {
                candidate.preedit = Some(preedit);
            }
        }
        let mut raw_quality = candidate.quality;
        if let Some(distance) = correction_distance {
            raw_quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        candidate.quality = raw_quality.exp() + self.initial_quality;
        if entry_code != lookup_code {
            candidate.source = CandidateSource::Completion;
            candidate.quality -= 1.0;
        }
        candidate
    }

    fn bounded_request_supported(&self, lookup_specs: &[LookupCodeSpec]) -> bool {
        (!self.prediction_never_first
            || self.prediction_candidate_limit.is_some()
            || self.prefix_fallback)
            && !self.sentence_over_completion
            && lookup_specs
                .iter()
                .all(|spec| spec.required_syllable_count.is_none())
    }

    fn uses_m44_short_key_metrics(&self, lookup_code: &str) -> bool {
        !self.dynamic_correction_lookup && is_m44_track_a_short_key_prefix(lookup_code)
    }

    fn uses_m44_track_b_metrics(&self) -> bool {
        self.dynamic_correction_lookup
    }

    fn uses_m44_track_b_short_prefix_lookup(&self, lookup_code: &str) -> bool {
        self.dynamic_correction_lookup && is_m44_track_b_short_key_prefix(lookup_code)
    }

    fn lookup_candidate_ref_raw_quality(&self, candidate: &PendingLookupCandidateRef<'_>) -> f32 {
        let mut raw_quality = candidate.candidate.raw_quality();
        if let Some(distance) = candidate.correction_distance {
            raw_quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        raw_quality
    }

    fn lookup_candidate_ref_category(&self, candidate: &PendingLookupCandidateRef<'_>) -> u8 {
        if candidate.spelling_abbreviation {
            1
        } else if candidate.entry_code.as_ref() != candidate.lookup_code
            && !candidate.limited_prediction
        {
            2
        } else {
            0
        }
    }

    fn lookup_candidate_ref_order(
        &self,
        left: &PendingLookupCandidateRef<'_>,
        right: &PendingLookupCandidateRef<'_>,
    ) -> Ordering {
        self.lookup_candidate_ref_category(left)
            .cmp(&self.lookup_candidate_ref_category(right))
            .then_with(|| {
                self.lookup_candidate_ref_raw_quality(right)
                    .partial_cmp(&self.lookup_candidate_ref_raw_quality(left))
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.entry_code.as_ref().cmp(right.entry_code.as_ref()))
            .then_with(|| left.candidate.text().cmp(right.candidate.text()))
            .then_with(|| left.emission_order.cmp(&right.emission_order))
    }

    fn materialized_quality(
        &self,
        entry_code: &str,
        lookup_code: &str,
        candidate: &LookupCandidate<'_>,
        correction_distance: Option<usize>,
    ) -> f32 {
        let mut raw_quality = candidate.raw_quality();
        if let Some(distance) = correction_distance {
            raw_quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        let mut quality = raw_quality.exp() + self.initial_quality;
        if entry_code != lookup_code {
            quality -= 1.0;
        }
        quality
    }

    fn bounded_candidate_order(
        &self,
        left: &PendingLookupCandidateRef<'_>,
        right: &PendingLookupCandidateRef<'_>,
    ) -> Ordering {
        self.materialized_quality(
            right.entry_code.as_ref(),
            right.lookup_code,
            &right.candidate,
            right.correction_distance,
        )
        .partial_cmp(&self.materialized_quality(
            left.entry_code.as_ref(),
            left.lookup_code,
            &left.candidate,
            left.correction_distance,
        ))
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.emission_order.cmp(&right.emission_order))
    }

    fn push_bounded_pending<'a>(
        &self,
        selected: &mut Vec<PendingLookupCandidateRef<'a>>,
        candidate: PendingLookupCandidateRef<'a>,
        limit: usize,
    ) {
        if selected.len() < limit {
            selected.push(candidate);
            return;
        }
        let Some((worst_index, worst)) = selected
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| self.bounded_candidate_order(left, right))
        else {
            return;
        };
        if self.bounded_candidate_order(&candidate, worst) == Ordering::Less {
            selected[worst_index] = candidate;
        }
    }

    fn push_bounded_pending_by_lookup_order<'a>(
        &self,
        selected: &mut Vec<PendingLookupCandidateRef<'a>>,
        candidate: PendingLookupCandidateRef<'a>,
        limit: usize,
    ) {
        if selected.len() < limit {
            selected.push(candidate);
            return;
        }
        let Some((worst_index, worst)) = selected
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| self.lookup_candidate_ref_order(left, right))
        else {
            return;
        };
        if self.lookup_candidate_ref_order(&candidate, worst) == Ordering::Less {
            selected[worst_index] = candidate;
        }
    }

    fn bounded_candidates_for_lookup_codes(
        &self,
        request: BoundedLookupRequest<'_>,
    ) -> TranslationResult {
        let BoundedLookupRequest {
            input,
            lookup_code,
            lookup_specs,
            filter_by_charset,
            segment_tags,
            limit,
            include_full_count,
        } = request;
        let ordered_mode = self.prediction_candidate_limit.is_some() || self.prefix_fallback;
        let record_short_key = self.uses_m44_short_key_metrics(lookup_code);
        let record_track_b = self.uses_m44_track_b_metrics();
        let short_key_filter_start =
            (record_short_key && crate::m37_metrics_enabled()).then(Instant::now);
        let mut short_key_rows_scanned = 0usize;
        let mut selected = Vec::new();
        let mut limited_predictions = Vec::new();
        let mut emission_order = 0;
        let mut full_count = 0;
        let mut has_full_exact_candidate = false;
        let has_correction_lookup = lookup_specs
            .iter()
            .any(|spec| spec.correction_distance.is_some());
        let can_stop_after_window = !include_full_count && !ordered_mode;
        let mut early_stopped = false;
        for lookup_spec in lookup_specs {
            let fetch_code = lookup_spec.code.as_str();
            let spec_lookup_code = lookup_spec.lookup_code.as_str();
            let lookup_has_exact_candidates = self.storage.has_code(fetch_code);
            let exact_start = LookupTimer::start();
            let mut exact_candidates = 0;
            for candidate in self
                .storage
                .exact_candidates(fetch_code)
                .filter(|candidate| {
                    self.is_dictionary_text_allowed(candidate.text())
                        && lookup_spec.required_syllable_count.map_or(true, |count| {
                            raw_candidate_syllable_count(candidate.raw_comment(), candidate.text())
                                == Some(count)
                        })
                        && (!filter_by_charset || !contains_extended_cjk(candidate.text()))
                })
            {
                exact_candidates += 1;
                full_count += 1;
                has_full_exact_candidate = true;
                let spelling_abbreviation =
                    self.is_spelling_abbreviation_view(spec_lookup_code, &candidate);
                let pending = PendingLookupCandidateRef {
                    entry_code: Cow::Borrowed(spec_lookup_code),
                    lookup_code: spec_lookup_code,
                    candidate,
                    correction_distance: lookup_spec.correction_distance,
                    spelling_abbreviation,
                    limited_prediction: false,
                    emission_order,
                };
                if ordered_mode {
                    self.push_bounded_pending_by_lookup_order(&mut selected, pending, limit);
                } else {
                    self.push_bounded_pending(&mut selected, pending, limit);
                }
                emission_order += 1;
                if can_stop_after_window && selected.len() >= limit {
                    early_stopped = true;
                    break;
                }
            }
            if record_short_key {
                short_key_rows_scanned += exact_candidates;
            }
            let exact_elapsed = exact_start.elapsed();
            self.storage
                .record_exact_lookup(exact_elapsed, exact_candidates);
            if record_track_b {
                crate::m37_record_track_b_exact_lookup(exact_elapsed);
            }
            if lookup_spec.correction_distance.is_none()
                && self.enable_completion
                && !spec_lookup_code.is_empty()
                && fetch_code == spec_lookup_code
                && !(can_stop_after_window && selected.len() >= limit)
            {
                let prefix_start = LookupTimer::start();
                let mut prefix_candidates = 0;
                for entry in self.storage.prefix_candidates(spec_lookup_code) {
                    let (entry_code, candidate) = entry.into_parts();
                    if entry_code == spec_lookup_code {
                        continue;
                    }
                    let limited_prediction =
                        self.is_limited_prediction_view(spec_lookup_code, &candidate);
                    if self.is_dictionary_text_allowed(candidate.text())
                        && self.is_completion_candidate_view_allowed(
                            lookup_has_exact_candidates,
                            limited_prediction,
                            &candidate,
                        )
                        && (!filter_by_charset || !contains_extended_cjk(candidate.text()))
                    {
                        prefix_candidates += 1;
                        let spelling_abbreviation =
                            self.is_spelling_abbreviation_view(entry_code.as_ref(), &candidate);
                        let pending = PendingLookupCandidateRef {
                            entry_code,
                            lookup_code: spec_lookup_code,
                            candidate,
                            correction_distance: lookup_spec.correction_distance,
                            spelling_abbreviation,
                            limited_prediction,
                            emission_order,
                        };
                        if ordered_mode {
                            if limited_prediction {
                                self.push_bounded_pending_by_lookup_order(
                                    &mut limited_predictions,
                                    pending,
                                    self.prediction_candidate_limit.unwrap_or(limit),
                                );
                            } else {
                                self.push_bounded_pending_by_lookup_order(
                                    &mut selected,
                                    pending,
                                    limit,
                                );
                                full_count += 1;
                            }
                        } else {
                            self.push_bounded_pending(&mut selected, pending, limit);
                            full_count += 1;
                        }
                        emission_order += 1;
                        if can_stop_after_window && selected.len() >= limit {
                            early_stopped = true;
                            break;
                        }
                    }
                }
                if record_short_key {
                    short_key_rows_scanned += prefix_candidates;
                }
                let prefix_elapsed = prefix_start.elapsed();
                self.storage
                    .record_prefix_lookup(prefix_elapsed, prefix_candidates);
                if record_track_b {
                    crate::m37_record_track_b_prefix_lookup(prefix_elapsed);
                }
            }
            if can_stop_after_window && selected.len() >= limit {
                early_stopped = true;
                break;
            }
        }
        if let Some(start) = short_key_filter_start {
            crate::m37_record_short_key_filter(start.elapsed());
            crate::m37_record_short_key_candidate_rows_scanned(short_key_rows_scanned);
        }
        full_count += limited_predictions.len();
        for candidate in limited_predictions {
            self.push_bounded_pending_by_lookup_order(&mut selected, candidate, limit);
        }
        if selected.is_empty() && self.enable_sentence {
            if let Some(model) = &self.upstream_sentence_model {
                let model_start = crate::m37_metrics_enabled().then(Instant::now);
                let mut candidates = model
                    .candidates_for_input_with_limit(
                        input,
                        limit.min(BOUNDED_SENTENCE_MODEL_PAGE_LIMIT),
                    )
                    .into_iter()
                    .filter(|candidate| {
                        !filter_by_charset || !contains_extended_cjk(&candidate.text)
                    })
                    .collect::<Vec<_>>();
                if let Some(start) = model_start {
                    crate::m37_record_upstream_sentence_model(start.elapsed(), candidates.len());
                }
                if !candidates.is_empty() {
                    if self.prefix_fallback && !has_correction_lookup {
                        let mut prefix_candidates = self.prefix_fallback_candidates(
                            input,
                            lookup_code,
                            filter_by_charset,
                            &candidates,
                        );
                        prefix_candidates.truncate(limit.saturating_sub(candidates.len()));
                        candidates.extend(prefix_candidates);
                    }
                    candidates.truncate(limit);
                    let result_full_count = if candidates.len() >= limit {
                        limit.saturating_add(1)
                    } else {
                        candidates.len()
                    };
                    crate::m37_record_bounded_iterator(limit, candidates.len(), result_full_count);
                    return TranslationResult::bounded(
                        candidates,
                        result_full_count,
                        include_full_count,
                    );
                }
                let abbreviation_start = crate::m37_metrics_enabled().then(Instant::now);
                let abbreviation_limit = limit.min(BOUNDED_SENTENCE_MODEL_PAGE_LIMIT);
                let mut candidates = self.abbreviation_sentence_candidates(
                    model,
                    input,
                    abbreviation_limit,
                    filter_by_charset,
                );
                if let Some(start) = abbreviation_start {
                    crate::m37_record_upstream_sentence_model(start.elapsed(), candidates.len());
                }
                if !candidates.is_empty() {
                    if self.prefix_fallback && !has_correction_lookup {
                        let mut prefix_candidates = self.prefix_fallback_candidates(
                            input,
                            lookup_code,
                            filter_by_charset,
                            &candidates,
                        );
                        prefix_candidates.truncate(limit.saturating_sub(candidates.len()));
                        candidates.extend(prefix_candidates);
                    }
                    candidates.truncate(limit);
                    let result_full_count = if candidates.len() >= abbreviation_limit {
                        limit.saturating_add(1)
                    } else {
                        candidates.len()
                    };
                    crate::m37_record_bounded_iterator(limit, candidates.len(), result_full_count);
                    return TranslationResult::bounded(
                        candidates,
                        result_full_count,
                        include_full_count,
                    );
                }
            }
            if self.prefix_fallback && !has_correction_lookup {
                crate::m37_record_full_list_fallback();
                return TranslationResult::complete(self.translated_candidates_for_segment(
                    input,
                    filter_by_charset,
                    segment_tags,
                ));
            }
            if let Some(sentence) = self.sentence_candidate(input, filter_by_charset, None) {
                let candidates = vec![sentence];
                crate::m37_record_bounded_iterator(limit, candidates.len(), candidates.len());
                return TranslationResult::bounded(candidates, 1, include_full_count);
            }
            crate::m37_record_full_list_fallback();
            return TranslationResult::complete(self.translated_candidates_for_segment(
                input,
                filter_by_charset,
                segment_tags,
            ));
        }
        if selected.is_empty() && self.prefix_fallback && !has_correction_lookup {
            let mut candidates =
                self.prefix_fallback_candidates(input, lookup_code, filter_by_charset, &[]);
            let full_count = candidates.len();
            if !candidates.is_empty() {
                candidates.truncate(limit);
                let result_full_count = if full_count > candidates.len() {
                    full_count
                } else {
                    candidates.len()
                };
                crate::m37_record_bounded_iterator(limit, candidates.len(), result_full_count);
                return TranslationResult::bounded(
                    candidates,
                    result_full_count,
                    include_full_count,
                );
            }
        }
        if ordered_mode {
            let sort_start = (record_short_key && crate::m37_metrics_enabled()).then(Instant::now);
            selected.sort_by(|left, right| self.lookup_candidate_ref_order(left, right));
            if let Some(start) = sort_start {
                crate::m37_record_short_key_sort_rank(start.elapsed());
            }
        } else {
            let sort_start = (record_short_key && crate::m37_metrics_enabled()).then(Instant::now);
            selected.sort_by(|left, right| self.bounded_candidate_order(left, right));
            if let Some(start) = sort_start {
                crate::m37_record_short_key_sort_rank(start.elapsed());
            }
        }
        let materialized_count = selected.len();
        let materialize_start = ((record_short_key || record_track_b)
            && crate::m37_metrics_enabled())
        .then(Instant::now);
        let comment_quality_start =
            (record_short_key && crate::m37_metrics_enabled()).then(Instant::now);
        let mut candidates = selected
            .into_iter()
            .map(|candidate| {
                self.candidate_for_lookup_view(
                    candidate.entry_code.as_ref(),
                    &candidate.candidate,
                    candidate.lookup_code,
                    candidate.correction_distance,
                )
            })
            .collect::<Vec<_>>();
        if let Some(start) = comment_quality_start {
            crate::m37_record_short_key_comment_quality(start.elapsed());
        }
        if record_short_key {
            crate::m37_record_short_key_candidates_cloned(materialized_count);
            for _ in 0..materialized_count {
                crate::m37_record_short_key_candidate_materialized();
            }
        }
        if record_track_b {
            for _ in 0..materialized_count {
                crate::m37_record_track_b_candidate_materialized();
            }
        }
        if self.combine_candidates {
            candidates = combine_duplicate_text_candidates(candidates);
        }
        let prefix_fallback_applies = self.prefix_fallback
            && !has_correction_lookup
            && (candidates.is_empty() || has_full_exact_candidate);
        if prefix_fallback_applies {
            if candidates.len() < limit {
                let mut prefix_candidates = self.prefix_fallback_candidates(
                    input,
                    lookup_code,
                    filter_by_charset,
                    &candidates,
                );
                full_count += prefix_candidates.len();
                prefix_candidates.truncate(limit - candidates.len());
                candidates.extend(prefix_candidates);
            } else if include_full_count || full_count <= candidates.len() {
                full_count += self
                    .prefix_fallback_candidates(input, lookup_code, filter_by_charset, &candidates)
                    .len();
            }
        }
        if ordered_mode {
            Self::assign_ordered_candidate_qualities(&mut candidates);
        }
        if let Some(start) = materialize_start {
            if record_short_key {
                crate::m37_record_short_key_first_page_materialize(start.elapsed());
            }
            if record_track_b {
                crate::m37_record_track_b_first_page_materialize(start.elapsed());
            }
        }
        crate::m37_record_bounded_iterator(limit, candidates.len(), full_count);
        let result_full_count = if early_stopped {
            full_count.max(candidates.len().saturating_add(1))
        } else {
            full_count
        };
        TranslationResult::bounded(candidates, result_full_count, include_full_count)
    }

    fn candidates_for_lookup_codes(
        &self,
        lookup_specs: &[LookupCodeSpec],
        filter_by_charset: bool,
    ) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        let record_track_b = self.uses_m44_track_b_metrics();
        for lookup_spec in lookup_specs {
            let fetch_code = lookup_spec.code.as_str();
            let lookup_code = lookup_spec.lookup_code.as_str();
            let mut pending = Vec::new();
            let lookup_has_exact_candidates = self.storage.has_code(fetch_code);
            let exact_start = LookupTimer::start();
            let mut exact_candidates = 0;
            pending.extend(
                self.storage
                    .exact_candidates(fetch_code)
                    .filter_map(|candidate| {
                        if !self.is_dictionary_text_allowed(candidate.text())
                            || !lookup_spec.required_syllable_count.map_or(true, |count| {
                                raw_candidate_syllable_count(
                                    candidate.raw_comment(),
                                    candidate.text(),
                                ) == Some(count)
                            })
                            || (filter_by_charset && contains_extended_cjk(candidate.text()))
                        {
                            return None;
                        }
                        exact_candidates += 1;
                        Some(PendingLookupCandidate {
                            entry_code: lookup_code.to_owned(),
                            lookup_code: lookup_code.to_owned(),
                            candidate: candidate.to_candidate(),
                            correction_distance: lookup_spec.correction_distance,
                            spelling_abbreviation: self
                                .is_spelling_abbreviation_view(lookup_code, &candidate),
                            limited_prediction: false,
                        })
                    }),
            );
            let exact_elapsed = exact_start.elapsed();
            self.storage
                .record_exact_lookup(exact_elapsed, exact_candidates);
            if record_track_b {
                crate::m37_record_track_b_exact_lookup(exact_elapsed);
            }
            if lookup_spec.correction_distance.is_none()
                && self.enable_completion
                && !lookup_code.is_empty()
                && fetch_code == lookup_code
            {
                let prefix_start = LookupTimer::start();
                let mut prefix_lookup_candidates = 0;
                let mut completion_candidates = Vec::new();
                for entry in self.storage.prefix_candidates(lookup_code) {
                    let (entry_code, candidate) = entry.into_parts();
                    if !entry_code.starts_with(lookup_code) {
                        break;
                    }
                    if entry_code == lookup_code {
                        continue;
                    }
                    let limited_prediction =
                        self.is_limited_prediction_view(lookup_code, &candidate);
                    if !self.is_dictionary_text_allowed(candidate.text())
                        || !self.is_completion_candidate_view_allowed(
                            lookup_has_exact_candidates,
                            limited_prediction,
                            &candidate,
                        )
                        || (filter_by_charset && contains_extended_cjk(candidate.text()))
                    {
                        continue;
                    }
                    prefix_lookup_candidates += 1;
                    let spelling_abbreviation =
                        self.is_spelling_abbreviation_view(entry_code.as_ref(), &candidate);
                    completion_candidates.push(PendingLookupCandidate {
                        entry_code: entry_code.into_owned(),
                        lookup_code: lookup_code.to_owned(),
                        candidate: candidate.to_candidate(),
                        correction_distance: lookup_spec.correction_distance,
                        spelling_abbreviation,
                        limited_prediction,
                    });
                }
                let prefix_elapsed = prefix_start.elapsed();
                self.storage
                    .record_prefix_lookup(prefix_elapsed, prefix_lookup_candidates);
                if record_track_b {
                    crate::m37_record_track_b_prefix_lookup(prefix_elapsed);
                }
                if let Some(limit) = self.prediction_candidate_limit {
                    let mut limited_predictions = Vec::new();
                    let mut ordinary_completions = Vec::new();
                    for candidate in completion_candidates {
                        if candidate.limited_prediction {
                            limited_predictions.push(candidate);
                        } else {
                            ordinary_completions.push(candidate);
                        }
                    }
                    limited_predictions
                        .sort_by(|left, right| self.lookup_candidate_order(left, right));
                    limited_predictions.truncate(limit);
                    limited_predictions.extend(ordinary_completions);
                    completion_candidates = limited_predictions;
                }
                pending.extend(completion_candidates);
            }
            if self.prediction_candidate_limit.is_some() {
                pending.sort_by(|left, right| self.lookup_candidate_order(left, right));
            }
            let pending_count = pending.len();
            candidates.extend(pending.into_iter().map(|pending| {
                self.candidate_for_lookup(
                    &pending.entry_code,
                    &pending.candidate,
                    &pending.lookup_code,
                    pending.correction_distance,
                )
            }));
            if record_track_b {
                for _ in 0..pending_count {
                    crate::m37_record_track_b_candidate_materialized();
                }
            }
        }
        candidates
    }

    fn prefix_fallback_candidates(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        existing_candidates: &[Candidate],
    ) -> Vec<Candidate> {
        let fallback_start = crate::m37_metrics_enabled().then(Instant::now);
        let prefixes = self.valid_lookup_prefixes(lookup_code);
        if prefixes.is_empty() {
            if let Some(start) = fallback_start {
                crate::m37_record_prefix_fallback(start.elapsed(), 0, 0);
            }
            return Vec::new();
        };
        let mut seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<HashSet<_>>();
        let mut candidates = Vec::new();
        struct PendingPrefixCandidate<'a> {
            pending: PendingLookupCandidateRef<'a>,
            consumed_input_len: usize,
            recompose_on_default: bool,
        }
        let mut pending = Vec::new();
        let mut emission_order = 0;
        let mut views_visited = 0;
        let bound_expansion = self.bounds_compact_fallback_expansion();
        let output_cap = if bound_expansion {
            MAX_PREFIX_FALLBACK_CANDIDATES
        } else {
            usize::MAX
        };
        let pending_cap = if bound_expansion {
            MAX_PREFIX_FALLBACK_PENDING_CANDIDATES
        } else {
            usize::MAX
        };
        let per_fetch_cap = if bound_expansion {
            MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE
        } else {
            usize::MAX
        };
        for prefix_spec in &prefixes {
            let prefix = prefix_spec.input_prefix;
            let fetch_code = prefix_spec.fetch_code.as_str();
            let consumed_input_len = input
                .len()
                .saturating_sub(lookup_code.len())
                .saturating_add(prefix_spec.consumed_lookup_len);
            let exact_start = LookupTimer::start();
            let mut exact_candidates = 0;
            let mut emitted_for_fetch_code = 0usize;
            for candidate in self
                .storage
                .exact_candidates(fetch_code)
                .filter(|candidate| {
                    self.is_dictionary_text_allowed(candidate.text())
                        && original_code_allows_prefix_fallback(candidate.raw_comment(), prefix)
                        && (!filter_by_charset || !contains_extended_cjk(candidate.text()))
                })
            {
                views_visited += 1;
                exact_candidates += 1;
                let recompose_on_default = consumed_input_len > 1
                    && !self.is_spelling_abbreviation_view(prefix, &candidate);
                pending.push(PendingPrefixCandidate {
                    pending: PendingLookupCandidateRef {
                        entry_code: Cow::Owned(fetch_code.to_owned()),
                        lookup_code: prefix,
                        candidate,
                        correction_distance: None,
                        spelling_abbreviation: false,
                        limited_prediction: false,
                        emission_order,
                    },
                    consumed_input_len,
                    recompose_on_default,
                });
                emission_order += 1;
                emitted_for_fetch_code += 1;
                if emitted_for_fetch_code >= per_fetch_cap {
                    break;
                }
                if pending.len() >= pending_cap {
                    break;
                }
            }
            self.storage
                .record_exact_lookup(exact_start.elapsed(), exact_candidates);
            if pending.len() >= pending_cap {
                break;
            }
        }
        pending.sort_by(|left, right| {
            right
                .consumed_input_len
                .cmp(&left.consumed_input_len)
                .then_with(|| {
                    self.lookup_candidate_ref_raw_quality(&right.pending)
                        .partial_cmp(&self.lookup_candidate_ref_raw_quality(&left.pending))
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| {
                    left.pending
                        .emission_order
                        .cmp(&right.pending.emission_order)
                })
        });
        for pending in pending {
            let mut candidate = self.candidate_for_lookup_view(
                pending.pending.entry_code.as_ref(),
                &pending.pending.candidate,
                pending.pending.lookup_code,
                None,
            );
            if !seen_texts.insert(candidate.text.clone()) {
                continue;
            }
            candidate.source = CandidateSource::PartialTable {
                consumed: pending.consumed_input_len,
                recompose_on_default: pending.recompose_on_default,
            };
            candidates.push(candidate);
            if candidates.len() >= output_cap {
                break;
            }
        }
        if let Some(start) = fallback_start {
            crate::m37_record_prefix_fallback(start.elapsed(), views_visited, candidates.len());
        }
        candidates
    }

    fn valid_lookup_prefixes<'a>(&self, lookup_code: &'a str) -> Vec<LookupPrefixSpec<'a>> {
        let mut boundaries = lookup_code
            .char_indices()
            .map(|(index, _)| index)
            .filter(|index| *index > 0)
            .collect::<Vec<_>>();
        boundaries.reverse();
        let mut prefixes = Vec::new();
        for end in boundaries {
            let prefix = &lookup_code[..end];
            let mut seen_fetch_codes = HashSet::new();
            for spec in self.sentence_lookup_specs(prefix) {
                if !self.storage.has_code(&spec.code) || !seen_fetch_codes.insert(spec.code.clone())
                {
                    continue;
                }
                prefixes.push(LookupPrefixSpec {
                    input_prefix: prefix,
                    fetch_code: spec.code,
                    consumed_lookup_len: end,
                });
            }
        }
        prefixes
    }

    fn assign_ordered_candidate_qualities(candidates: &mut [Candidate]) {
        let base = candidates.len() as f32 + 1.0;
        for (index, candidate) in candidates.iter_mut().enumerate() {
            candidate.quality = base - index as f32;
        }
    }

    fn translated_candidates(&self, input: &str, filter_by_charset: bool) -> Vec<Candidate> {
        self.translated_candidates_for_segment(input, filter_by_charset, None)
    }

    fn translated_candidates_for_segment(
        &self,
        input: &str,
        filter_by_charset: bool,
        segment_tags: Option<&[String]>,
    ) -> Vec<Candidate> {
        let accepts_segment = segment_tags
            .map(|tags| self.accepts_segment_tags(tags))
            .unwrap_or_else(|| self.accepts_default_segment());
        if !accepts_segment {
            return Vec::new();
        }

        let Some(lookup_code) = self.lookup_code(input) else {
            return Vec::new();
        };
        let expanded_lookup_codes = self.expanded_lookup_specs(lookup_code);
        let mut candidates =
            self.candidates_for_lookup_codes(&expanded_lookup_codes, filter_by_charset);
        let has_correction_lookup = expanded_lookup_codes
            .iter()
            .any(|spec| spec.correction_distance.is_some());
        let has_full_exact_candidate = candidates
            .iter()
            .any(|candidate| candidate.source == CandidateSource::Table);
        if self.combine_candidates {
            candidates = combine_duplicate_text_candidates(candidates);
        }
        self.enforce_prediction_never_first(&mut candidates);

        let mut used_sentence = false;
        if candidates.is_empty() {
            if let Some(model) = &self.upstream_sentence_model {
                let model_start = crate::m37_metrics_enabled().then(Instant::now);
                candidates = model
                    .candidates_for_input(input)
                    .into_iter()
                    .filter(|candidate| {
                        !filter_by_charset || !contains_extended_cjk(&candidate.text)
                    })
                    .collect();
                if let Some(start) = model_start {
                    crate::m37_record_upstream_sentence_model(start.elapsed(), candidates.len());
                }
                used_sentence = !candidates.is_empty();
                if candidates.is_empty() {
                    let abbreviation_start = crate::m37_metrics_enabled().then(Instant::now);
                    candidates = self.abbreviation_sentence_candidates(
                        model,
                        input,
                        usize::MAX,
                        filter_by_charset,
                    );
                    if let Some(start) = abbreviation_start {
                        crate::m37_record_upstream_sentence_model(
                            start.elapsed(),
                            candidates.len(),
                        );
                    }
                    used_sentence = !candidates.is_empty();
                }
            }
        }
        if candidates.is_empty() && self.enable_sentence {
            if let Some(sentence) = self.sentence_candidate(input, filter_by_charset, None) {
                candidates.push(sentence);
                used_sentence = true;
            }
        } else if self.sentence_over_completion
            && candidates
                .first()
                .is_some_and(|candidate| candidate.source == CandidateSource::Completion)
        {
            let priority_floor = candidates
                .iter()
                .map(|candidate| candidate.quality)
                .max_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
            if let Some(sentence) =
                self.sentence_candidate(input, filter_by_charset, priority_floor)
            {
                candidates.push(sentence);
            }
        }

        if self.prefix_fallback && !has_correction_lookup {
            // Full exact rows may coexist with a valid leading prefix; prefix lookup plus
            // the existing candidate set keeps the fallback benign for normal full matches.
            let should_add_prefix_fallback =
                candidates.is_empty() || used_sentence || has_full_exact_candidate;
            if should_add_prefix_fallback {
                let prefix_candidates = self.prefix_fallback_candidates(
                    input,
                    lookup_code,
                    filter_by_charset,
                    &candidates,
                );
                candidates.extend(prefix_candidates);
            }
        }
        if self.prefix_fallback || self.prediction_candidate_limit.is_some() {
            Self::assign_ordered_candidate_qualities(&mut candidates);
        }

        candidates
    }

    fn translated_candidates_for_segment_with_request(
        &self,
        input: &str,
        filter_by_charset: bool,
        segment_tags: Option<&[String]>,
        request: CandidateRequest,
    ) -> TranslationResult {
        let Some(limit) = request.limit.filter(|limit| *limit > 0) else {
            crate::m37_record_full_list_fallback();
            return TranslationResult::complete(self.translated_candidates_for_segment(
                input,
                filter_by_charset,
                segment_tags,
            ));
        };
        let accepts_segment = segment_tags
            .map(|tags| self.accepts_segment_tags(tags))
            .unwrap_or_else(|| self.accepts_default_segment());
        if !accepts_segment {
            return TranslationResult::complete(Vec::new());
        }

        let Some(lookup_code) = self.lookup_code(input) else {
            return TranslationResult::complete(Vec::new());
        };
        let expanded_lookup_codes = self.expanded_lookup_specs(lookup_code);
        if !self.bounded_request_supported(&expanded_lookup_codes) {
            crate::m37_record_full_list_fallback();
            return TranslationResult::complete(self.translated_candidates_for_segment(
                input,
                filter_by_charset,
                segment_tags,
            ));
        }
        self.bounded_candidates_for_lookup_codes(BoundedLookupRequest {
            input,
            lookup_code,
            lookup_specs: &expanded_lookup_codes,
            filter_by_charset,
            segment_tags,
            limit,
            include_full_count: request.include_debug_full_count,
        })
    }

    fn sentence_candidate(
        &self,
        input: &str,
        filter_by_charset: bool,
        priority_floor: Option<f32>,
    ) -> Option<Candidate> {
        let sentence_start = crate::m37_metrics_enabled().then(Instant::now);
        let mut sentence_metrics = M37SentenceCandidateMetrics::default();
        if input.is_empty() {
            record_sentence_candidate_metrics(sentence_start, sentence_metrics, 0);
            return None;
        }

        #[derive(Clone)]
        struct SentencePath {
            fuzzy_pieces: usize,
            quality: f32,
            raw_quality: f32,
            pieces: Vec<String>,
        }

        let mut paths: Vec<Option<SentencePath>> = vec![None; input.len() + 1];
        paths[0] = Some(SentencePath {
            fuzzy_pieces: 0,
            quality: 0.0,
            raw_quality: 0.0,
            pieces: Vec::new(),
        });
        let mut live_paths = 1usize;
        let mut max_live_paths = 1usize;
        let max_candidates_per_span = if self.bounds_compact_fallback_expansion() {
            MAX_SENTENCE_CANDIDATES_PER_SPAN
        } else {
            usize::MAX
        };
        for pos in input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
        {
            let Some(path) = paths.get(pos).and_then(Clone::clone) else {
                continue;
            };
            for end in input[pos..]
                .char_indices()
                .skip(1)
                .map(|(offset, _)| pos + offset)
                .chain(std::iter::once(input.len()))
            {
                let entry_code = &input[pos..end];
                sentence_metrics.substrings_considered += 1;
                let is_final_segment = end == input.len();
                // In abbreviation-bearing schemas, generated one-letter aliases are lookup
                // shortcuts, not stable interior sentence boundaries.
                if !is_final_segment
                    && self.single_letter_sentence_guard_enabled
                    && entry_code.len() == 1
                {
                    continue;
                }
                let exact_start = crate::m37_metrics_enabled().then(Instant::now);
                let sentence_specs = self.sentence_lookup_specs(entry_code);
                let mut entry_matches = Vec::new();
                'specs: for spec in &sentence_specs {
                    for candidate in self.storage.exact_candidates(&spec.code) {
                        if !self.is_dictionary_text_allowed(candidate.text())
                            || (filter_by_charset && contains_extended_cjk(candidate.text()))
                        {
                            continue;
                        }
                        entry_matches.push(candidate);
                        if entry_matches.len() >= max_candidates_per_span {
                            break 'specs;
                        }
                    }
                }
                if let Some(start) = exact_start {
                    sentence_metrics.exact_lookup_calls += 1;
                    sentence_metrics.exact_lookup_ns += start.elapsed();
                    sentence_metrics.exact_lookup_candidates += entry_matches.len();
                }
                if is_final_segment && self.enable_completion && !entry_code.is_empty() {
                    let prefix_start = crate::m37_metrics_enabled().then(Instant::now);
                    let mut prefix_candidates = 0usize;
                    for entry in self.storage.prefix_candidates(entry_code) {
                        if entry_matches.len() >= max_candidates_per_span {
                            break;
                        }
                        let (completion_code, candidate) = entry.into_parts();
                        if !completion_code.starts_with(entry_code) {
                            break;
                        }
                        if completion_code == entry_code {
                            continue;
                        }
                        prefix_candidates += 1;
                        entry_matches.push(candidate);
                    }
                    if let Some(start) = prefix_start {
                        sentence_metrics.prefix_lookup_calls += 1;
                        sentence_metrics.prefix_lookup_ns += start.elapsed();
                        sentence_metrics.prefix_lookup_candidates += prefix_candidates;
                    }
                }
                sentence_metrics.entry_matches_collected += entry_matches.len();
                if entry_matches.is_empty() {
                    continue;
                }
                let mut end_pos = pos + entry_code.len();
                while end_pos < input.len() {
                    let Some(ch) = input[end_pos..].chars().next() else {
                        break;
                    };
                    if !self.delimiters.contains(ch) {
                        break;
                    }
                    end_pos += ch.len_utf8();
                }
                for candidate in entry_matches {
                    let mut next_path = path.clone();
                    sentence_metrics.path_clones += 1;
                    if !raw_sentence_piece_matches_input_code(
                        candidate.raw_comment(),
                        candidate.text(),
                        entry_code,
                    ) {
                        next_path.fuzzy_pieces += 1;
                    }
                    next_path.quality +=
                        sentence_piece_quality(candidate.raw_quality(), self.sentence_word_penalty);
                    next_path.raw_quality += candidate.raw_quality();
                    next_path.pieces.push(candidate.text().to_owned());
                    if is_final_segment && next_path.pieces.len() <= 1 {
                        continue;
                    }
                    let replace = match paths[end_pos].as_ref() {
                        Some(existing) => {
                            match next_path.fuzzy_pieces.cmp(&existing.fuzzy_pieces) {
                                Ordering::Less => true,
                                Ordering::Greater => false,
                                Ordering::Equal => match next_path
                                    .quality
                                    .partial_cmp(&existing.quality)
                                    .unwrap_or(Ordering::Equal)
                                {
                                    Ordering::Greater => true,
                                    Ordering::Equal => next_path.raw_quality > existing.raw_quality,
                                    Ordering::Less => false,
                                },
                            }
                        }
                        None => true,
                    };
                    if replace {
                        let replacing_empty = paths[end_pos].is_none();
                        paths[end_pos] = Some(next_path);
                        sentence_metrics.path_replacements += 1;
                        if replacing_empty {
                            live_paths += 1;
                            max_live_paths = max_live_paths.max(live_paths);
                        }
                    }
                }
            }
        }

        sentence_metrics.max_live_paths = max_live_paths;
        let Some(path) = paths[input.len()].take() else {
            record_sentence_candidate_metrics(sentence_start, sentence_metrics, 0);
            return None;
        };
        if path.pieces.len() <= 1 {
            record_sentence_candidate_metrics(sentence_start, sentence_metrics, 0);
            return None;
        }
        let quality = priority_floor
            .map(|floor| floor + 1.0)
            .unwrap_or(path.quality.max(1.0) + self.initial_quality);
        let candidate = Candidate {
            text: path.pieces.join(""),
            comment: " ☯ ".to_owned(),
            preedit: None,
            source: CandidateSource::Sentence,
            quality,
        };
        record_sentence_candidate_metrics(sentence_start, sentence_metrics, 1);
        Some(candidate)
    }

    pub fn parse_rime_dict_yaml(input: &str) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml(input).map(Self::from_dictionary)
    }

    pub fn parse_rime_dict_yaml_with_imports(
        input: &str,
        import_loader: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml_with_imports(input, import_loader)
            .map(Self::from_dictionary)
    }

    pub fn parse_rime_dict_yaml_with_imports_and_packs(
        input: &str,
        packs: impl IntoIterator<Item = impl AsRef<str>>,
        import_loader: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml_with_imports_and_packs(input, packs, import_loader)
            .map(Self::from_dictionary)
    }

    pub fn parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        input: &str,
        packs: impl IntoIterator<Item = impl AsRef<str>>,
        import_loader: impl FnMut(&str) -> Option<String>,
        vocabulary_loader: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            input,
            packs,
            import_loader,
            vocabulary_loader,
        )
        .map(Self::from_dictionary)
    }
}

pub(crate) fn is_m44_track_a_short_key_prefix(input: &str) -> bool {
    matches!(input, "h" | "ha" | "hao" | "n" | "ni")
}

fn is_m44_track_b_short_key_prefix(input: &str) -> bool {
    matches!(
        input,
        "h" | "ha" | "hai" | "hau" | "n" | "ne" | "nei" | "ng" | "ngo"
    )
}

fn record_sentence_candidate_metrics(
    start: Option<Instant>,
    mut record: M37SentenceCandidateMetrics,
    result_candidates: usize,
) {
    if let Some(start) = start {
        record.duration = start.elapsed();
        record.result_candidates = result_candidates;
        crate::m37_record_sentence_candidate_metrics(record);
    }
}

fn entries_by_code(entries: &[(String, Candidate)]) -> BTreeMap<String, Vec<Candidate>> {
    let mut indexed = BTreeMap::<String, Vec<Candidate>>::new();
    for (code, candidate) in entries {
        indexed
            .entry(code.clone())
            .or_default()
            .push(candidate.clone());
    }
    indexed
}

fn entries_by_code_from_entries(
    entries: impl IntoIterator<Item = (String, Candidate)>,
) -> BTreeMap<String, Vec<Candidate>> {
    let mut indexed = BTreeMap::<String, Vec<Candidate>>::new();
    for (code, candidate) in entries {
        indexed.entry(code).or_default().push(candidate);
    }
    indexed
}

fn spelling_abbreviation_entries(
    entries: &[ExpandedSpellingEntry],
) -> HashSet<(String, String, String)> {
    entries
        .iter()
        .filter(|entry| entry.abbreviation)
        .map(|entry| {
            (
                entry.code.clone(),
                entry.candidate.text.clone(),
                entry.candidate.comment.clone(),
            )
        })
        .collect()
}

fn normal_codes(entries: &[(String, Candidate)]) -> HashSet<String> {
    entries.iter().map(|(code, _)| code.clone()).collect()
}

fn complete_syllable_prefix_count(raw_code: &str, lookup_code: &str) -> Option<usize> {
    let mut normalized = String::new();
    let mut syllables = 0;
    for ch in raw_code.chars() {
        if ch.is_ascii_digit() {
            syllables += 1;
            if normalized == lookup_code {
                return Some(syllables);
            }
            if normalized.len() >= lookup_code.len() {
                return None;
            }
        } else if ch.is_ascii_alphabetic() {
            normalized.push(ch.to_ascii_lowercase());
        }
    }
    None
}

fn original_code_allows_prefix_fallback(raw_code: &str, lookup_code: &str) -> bool {
    let normalized = normalized_original_code(raw_code);
    let lookup = lookup_code
        .chars()
        .map(|ch| ch.to_ascii_lowercase())
        .collect::<String>();
    normalized == lookup || (lookup.len() == 1 && normalized.starts_with(&lookup))
}

fn raw_sentence_piece_matches_input_code(raw_comment: &str, _text: &str, entry_code: &str) -> bool {
    if raw_comment.is_empty() {
        return true;
    }
    let normalized = normalized_original_code(raw_comment);
    normalized == entry_code
}

fn source_code_syllable_count(raw_code: &str) -> Option<usize> {
    let code = typeduck_rich_comment_code(raw_code).unwrap_or(raw_code);
    let count = code.chars().filter(char::is_ascii_digit).count();
    (count > 0).then_some(count)
}

fn raw_candidate_syllable_count(raw_comment: &str, text: &str) -> Option<usize> {
    source_code_syllable_count(raw_comment).or_else(|| {
        let count = text.chars().count();
        (count > 0).then_some(count)
    })
}

fn normalized_original_code(raw_code: &str) -> String {
    typeduck_rich_comment_code(raw_code)
        .unwrap_or(raw_code)
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn typeduck_rich_comment_code(raw_code: &str) -> Option<&str> {
    let normalized = raw_code.trim_start_matches(['\u{000b}', '\u{000c}', '\r']);
    let mut fields = normalized.split(',');
    let _rank = fields.next()?;
    let _text = fields.next()?;
    let code = fields.next()?.trim();
    (!code.is_empty()).then_some(code)
}

fn typeduck_restricted_distance(left: &str, right: &str, threshold: usize) -> usize {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let left_len = left.len();
    let right_len = right.len();
    let mut distance = vec![0; (left_len + 1) * (right_len + 1)];
    let index = |left_index: usize, right_index: usize| left_index * (right_len + 1) + right_index;

    for left_index in 1..=left_len {
        distance[index(left_index, 0)] = left_index * 2;
    }
    for right_index in 1..=right_len {
        distance[index(0, right_index)] = right_index * 2;
    }

    for left_index in 1..=left_len {
        let mut row_min = threshold + 1;
        for right_index in 1..=right_len {
            distance[index(left_index, right_index)] = [
                distance[index(left_index - 1, right_index)] + 2,
                distance[index(left_index, right_index - 1)] + 2,
                distance[index(left_index - 1, right_index - 1)]
                    + typeduck_substitution_cost(left[left_index - 1], right[right_index - 1]),
            ]
            .into_iter()
            .min()
            .expect("distance candidates should be non-empty");
            if left_index > 1
                && right_index > 1
                && left[left_index - 2] == right[right_index - 1]
                && left[left_index - 1] == right[right_index - 2]
            {
                distance[index(left_index, right_index)] = distance[index(left_index, right_index)]
                    .min(distance[index(left_index - 2, right_index - 2)] + 2);
            }
            row_min = row_min.min(distance[index(left_index, right_index)]);
        }
        if row_min > threshold {
            return row_min;
        }
    }

    distance[index(left_len, right_len)]
}

fn typeduck_length_distance_lower_bound(left: &str, right: &str) -> usize {
    left.len().abs_diff(right.len()) * 2
}

fn typeduck_substitution_cost(left: u8, right: u8) -> usize {
    if left == right {
        return 0;
    }
    if typeduck_keyboard_neighbors(left, right) {
        1
    } else {
        4
    }
}

fn typeduck_keyboard_neighbors(left: u8, right: u8) -> bool {
    match left {
        b'1' => matches!(right, b'2' | b'q' | b'w'),
        b'2' => matches!(right, b'1' | b'3' | b'q' | b'w' | b'e'),
        b'3' => matches!(right, b'2' | b'4' | b'w' | b'e' | b'r'),
        b'4' => matches!(right, b'3' | b'5' | b'e' | b'r' | b't'),
        b'5' => matches!(right, b'4' | b'6' | b'r' | b't' | b'y'),
        b'6' => matches!(right, b'5' | b'7' | b't' | b'y' | b'u'),
        b'7' => matches!(right, b'6' | b'8' | b'y' | b'u' | b'i'),
        b'8' => matches!(right, b'7' | b'9' | b'u' | b'i' | b'o'),
        b'9' => matches!(right, b'8' | b'0' | b'i' | b'o' | b'p'),
        b'0' => matches!(right, b'9' | b'-' | b'o' | b'p' | b'['),
        b'-' => matches!(right, b'0' | b'=' | b'p' | b'[' | b']'),
        b'=' => matches!(right, b'-' | b'[' | b']' | b'\\'),
        b'q' => matches!(right, b'w'),
        b'w' => matches!(right, b'q' | b'e'),
        b'e' => matches!(right, b'w' | b'r'),
        b'r' => matches!(right, b'e' | b't'),
        b't' => matches!(right, b'r' | b'y'),
        b'y' => matches!(right, b't' | b'u'),
        b'u' => matches!(right, b'y' | b'i'),
        b'i' => matches!(right, b'u' | b'o'),
        b'o' => matches!(right, b'i' | b'p'),
        b'p' => matches!(right, b'o' | b'['),
        b'[' => matches!(right, b'p' | b']'),
        b']' => matches!(right, b'[' | b'\\'),
        b'\\' => matches!(right, b']'),
        b'a' => matches!(right, b's'),
        b's' => matches!(right, b'a' | b'd'),
        b'd' => matches!(right, b's' | b'f'),
        b'f' => matches!(right, b'd' | b'g'),
        b'g' => matches!(right, b'f' | b'h'),
        b'h' => matches!(right, b'g' | b'j'),
        b'j' => matches!(right, b'h' | b'k'),
        b'k' => matches!(right, b'j' | b'l'),
        b'l' => matches!(right, b'k' | b';'),
        b';' => matches!(right, b'l' | b'\''),
        b'\'' => matches!(right, b';'),
        b'z' => matches!(right, b'x'),
        b'x' => matches!(right, b'z' | b'c'),
        b'c' => matches!(right, b'x' | b'v'),
        b'v' => matches!(right, b'c' | b'b'),
        b'b' => matches!(right, b'v' | b'n'),
        b'n' => matches!(right, b'b' | b'm'),
        b'm' => matches!(right, b'n' | b','),
        b',' => matches!(right, b'm' | b'.'),
        b'.' => matches!(right, b',' | b'/'),
        b'/' => matches!(right, b'.'),
        _ => false,
    }
}

fn combine_duplicate_text_candidates(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut index_by_text = HashMap::<String, usize>::new();
    let mut combined = Vec::<Candidate>::new();
    for candidate in candidates {
        if let Some(index) = index_by_text.get(&candidate.text).copied() {
            let existing = &mut combined[index];
            existing.comment = combine_lookup_comments(&existing.comment, &candidate.comment);
            if candidate.quality > existing.quality {
                existing.quality = candidate.quality;
            }
        } else {
            index_by_text.insert(candidate.text.clone(), combined.len());
            combined.push(candidate);
        }
    }
    combined
}

fn combine_lookup_comments(existing: &str, next: &str) -> String {
    let (prefix, existing_lookup, had_separator) = split_comment_prefix(existing);
    let (_, next_lookup, next_had_separator) = split_comment_prefix(next);
    let mut codes = split_lookup_codes(existing_lookup);
    for code in split_lookup_codes(next_lookup) {
        if !codes.iter().any(|existing| existing == &code) {
            codes.push(code);
        }
    }
    if codes.is_empty() {
        return existing.to_owned();
    }
    if had_separator || next_had_separator || !prefix.is_empty() {
        format!("{prefix}\u{000c}{}", codes.join(";"))
    } else {
        codes.join(";")
    }
}

fn split_comment_prefix(comment: &str) -> (&str, &str, bool) {
    comment
        .split_once('\u{000c}')
        .map_or(("", comment, false), |(prefix, lookup)| {
            (prefix, lookup, true)
        })
}

fn split_lookup_codes(comment: &str) -> Vec<String> {
    comment
        .split(['\u{000c}', ';', ' ', '\t'])
        .filter(|code| !code.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

impl Translator for StaticTableTranslator {
    fn name(&self) -> &'static str {
        "static_table_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        self.translated_candidates(input, false)
    }

    fn translate_with_state(
        &self,
        input: &str,
        _status: &Status,
        options: &HashMap<String, bool>,
    ) -> Vec<Candidate> {
        let filter_by_charset = self.enable_charset_filter
            && !options.get("extended_charset").copied().unwrap_or(false);
        self.translated_candidates(input, filter_by_charset)
    }

    fn translate_with_context(
        &self,
        input: &str,
        _status: &Status,
        options: &HashMap<String, bool>,
        context: &Context,
    ) -> Vec<Candidate> {
        let filter_by_charset = self.enable_charset_filter
            && !options.get("extended_charset").copied().unwrap_or(false);
        self.translated_candidates_for_segment(
            input,
            filter_by_charset,
            Some(&context.segment_tags),
        )
    }

    fn translate_with_context_and_request(
        &self,
        input: &str,
        _status: &Status,
        options: &HashMap<String, bool>,
        context: &Context,
        request: CandidateRequest,
    ) -> TranslationResult {
        let filter_by_charset = (self.enable_charset_filter
            && !options.get("extended_charset").copied().unwrap_or(false))
            || request.filter_extended_cjk;
        self.translated_candidates_for_segment_with_request(
            input,
            filter_by_charset,
            Some(&context.segment_tags),
            request,
        )
    }

    fn spelling_algebra_debug(&self, input: &str) -> Option<SpellingAlgebraDebug> {
        if self.spelling_algebra_formulas.is_empty() {
            return None;
        }
        let lookup_code = (!input.is_empty())
            .then(|| self.lookup_code(input).map(ToOwned::to_owned))
            .flatten();
        let mut expanded_codes = lookup_code.as_deref().map_or_else(Vec::new, |code| {
            self.expanded_lookup_specs(code)
                .into_iter()
                .map(|spec| spec.code)
                .collect::<Vec<_>>()
        });
        expanded_codes.sort();
        expanded_codes.dedup();
        Some(SpellingAlgebraDebug {
            translator: self.name().to_owned(),
            input: input.to_owned(),
            lookup_code,
            formulas: self.spelling_algebra_formulas.clone(),
            expanded_codes,
        })
    }

    fn prediction_weight_threshold(&self) -> Option<f32> {
        self.prediction_weight_threshold
    }

    fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let mut rows = self.storage.memory_owner_rows();
        rows.push(self.normal_codes.memory_owner_row(&self.storage));
        if let Some(prism_payload) = &self.prism_payload {
            rows.extend(prism_payload.memory_owner_rows());
        }
        if let Some(model) = &self.upstream_sentence_model {
            rows.extend(model.memory_owner_rows());
        } else {
            rows.extend([
                MemoryOwnerRow::new(
                    "poet.entries_by_code",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "none",
                    "upstream sentence model not retained for this translator",
                ),
                MemoryOwnerRow::new(
                    "poet.lookup_index",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "none",
                    "upstream sentence model not retained for this translator",
                ),
                MemoryOwnerRow::new(
                    "poet.abbreviation_vocabulary",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "none",
                    "upstream sentence model not retained for this translator",
                ),
            ]);
        }
        rows
    }

    fn storage_diagnostics(&self) -> Vec<StorageDiagnosticsRow> {
        self.storage.storage_diagnostics()
    }
}

struct ReverseLookupData {
    entries: Vec<TableEntry>,
    reverse_comments: HashMap<String, Vec<String>>,
}

enum ReverseLookupStorage {
    Ready(ReverseLookupData),
    Lazy {
        loaded: Mutex<Option<ReverseLookupData>>,
        loader: Box<dyn Fn() -> Option<(TableDictionary, Option<TableDictionary>)> + Send + Sync>,
    },
}

pub struct ReverseLookupTranslator {
    storage: ReverseLookupStorage,
    prefix: String,
    suffix: String,
    tag: String,
    enable_completion: bool,
    comment_format: CommentFormat,
    spelling_algebra_formulas: Vec<String>,
}

impl ReverseLookupTranslator {
    #[must_use]
    pub fn new(
        dictionary: TableDictionary,
        reverse_dictionary: Option<TableDictionary>,
        prefix: impl Into<String>,
        suffix: impl Into<String>,
    ) -> Self {
        Self {
            storage: ReverseLookupStorage::Ready(ReverseLookupData::from_dictionaries(
                dictionary,
                reverse_dictionary,
            )),
            prefix: prefix.into(),
            suffix: suffix.into(),
            tag: "reverse_lookup".to_owned(),
            enable_completion: false,
            comment_format: CommentFormat::default(),
            spelling_algebra_formulas: Vec::new(),
        }
    }

    #[must_use]
    pub fn new_lazy(
        loader: impl Fn() -> Option<(TableDictionary, Option<TableDictionary>)> + Send + Sync + 'static,
        prefix: impl Into<String>,
        suffix: impl Into<String>,
    ) -> Self {
        Self {
            storage: ReverseLookupStorage::Lazy {
                loaded: Mutex::new(None),
                loader: Box::new(loader),
            },
            prefix: prefix.into(),
            suffix: suffix.into(),
            tag: "reverse_lookup".to_owned(),
            enable_completion: false,
            comment_format: CommentFormat::default(),
            spelling_algebra_formulas: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    #[must_use]
    pub fn with_completion(mut self, enable_completion: bool) -> Self {
        self.enable_completion = enable_completion;
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self
    }

    #[must_use]
    pub fn with_spelling_algebra(mut self, formulas: &[String]) -> Self {
        self.spelling_algebra_formulas = formulas.to_vec();
        if matches!(self.storage, ReverseLookupStorage::Ready(_)) {
            if let ReverseLookupStorage::Ready(data) = &mut self.storage {
                data.apply_spelling_algebra(formulas);
            }
        }
        self
    }

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        segment_tags
            .iter()
            .any(|segment_tag| segment_tag == &self.tag)
    }

    fn with_data<T>(&self, f: impl FnOnce(&ReverseLookupData) -> T) -> Option<T> {
        match &self.storage {
            ReverseLookupStorage::Ready(data) => Some(f(data)),
            ReverseLookupStorage::Lazy { loaded, loader } => {
                let mut loaded = loaded
                    .lock()
                    .expect("reverse lookup lazy data should not be poisoned");
                if loaded.is_none() {
                    if let Some((dictionary, reverse_dictionary)) = loader() {
                        let mut data =
                            ReverseLookupData::from_dictionaries(dictionary, reverse_dictionary);
                        data.apply_spelling_algebra(&self.spelling_algebra_formulas);
                        *loaded = Some(data);
                    }
                }
                loaded.as_ref().map(f)
            }
        }
    }
}

impl ReverseLookupData {
    fn from_dictionaries(
        dictionary: TableDictionary,
        reverse_dictionary: Option<TableDictionary>,
    ) -> Self {
        let mut reverse_comments: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(reverse_dictionary) = reverse_dictionary {
            let comment_format = reverse_dictionary
                .dict_settings()
                .get("comment_format")
                .cloned();
            for entry in &reverse_dictionary.entries {
                let comment = comment_format.as_ref().map_or_else(
                    || entry.code.clone(),
                    |format| format.replace("$comment", &entry.code),
                );
                reverse_comments
                    .entry(entry.text.clone())
                    .or_default()
                    .push(comment);
            }
        }

        Self {
            entries: dictionary.entries,
            reverse_comments,
        }
    }

    fn apply_spelling_algebra(&mut self, formulas: &[String]) {
        let algebra = SpellingAlgebra::parse(formulas);
        if algebra.is_empty() {
            return;
        }
        let entries = std::mem::take(&mut self.entries)
            .into_iter()
            .map(|entry| {
                let code = entry.code;
                let candidate = Candidate {
                    text: entry.text,
                    comment: String::new(),
                    preedit: None,
                    source: CandidateSource::ReverseLookup,
                    quality: entry.weight,
                };
                (code, candidate)
            })
            .collect::<Vec<_>>();
        let (expanded, _, _) = algebra.expand_entries_with_normal_codes(entries);
        self.entries = expanded
            .into_iter()
            .map(|entry| TableEntry::new(entry.code, entry.candidate.text, entry.candidate.quality))
            .collect();
    }

    fn memory_owner_rows(&self, storage_label: &'static str) -> Vec<MemoryOwnerRow> {
        vec![
            MemoryOwnerRow::new(
                "reverse_lookup.entries",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_table_entries_bytes(&self.entries),
                self.entries.len(),
                storage_label,
                "retained reverse-lookup dictionary entries; required when the reverse translator is loaded",
            ),
            MemoryOwnerRow::new(
                "reverse_lookup.comments_index",
                MemoryOwnerClass::HeapOwnedRequired,
                estimate_string_vec_hash_map_bytes(&self.reverse_comments),
                self.reverse_comments.values().map(Vec::len).sum(),
                storage_label,
                "retained reverse-comment side index used to join dictionary-panel lookup comments",
            ),
        ]
    }
}

impl ReverseLookupStorage {
    fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        match self {
            Self::Ready(data) => data.memory_owner_rows("ready_heap"),
            Self::Lazy { loaded, .. } => {
                let loaded = loaded
                    .lock()
                    .expect("reverse lookup lazy data should not be poisoned");
                loaded.as_ref().map_or_else(
                    || {
                        vec![
                            MemoryOwnerRow::new(
                                "reverse_lookup.entries",
                                MemoryOwnerClass::SharedOrOverlapping,
                                0,
                                0,
                                "lazy_unloaded",
                                "lazy reverse-lookup dictionary is not retained until used",
                            ),
                            MemoryOwnerRow::new(
                                "reverse_lookup.comments_index",
                                MemoryOwnerClass::SharedOrOverlapping,
                                0,
                                0,
                                "lazy_unloaded",
                                "lazy reverse-comment side index is not retained until used",
                            ),
                        ]
                    },
                    |data| data.memory_owner_rows("lazy_loaded_heap"),
                )
            }
        }
    }
}

impl Translator for ReverseLookupTranslator {
    fn name(&self) -> &'static str {
        "reverse_lookup_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }

        let start = if !self.prefix.is_empty() && input.starts_with(&self.prefix) {
            self.prefix.len()
        } else {
            0
        };
        let has_prefix = start > 0;
        let mut code = &input[start..];
        if !self.suffix.is_empty() && code.ends_with(&self.suffix) {
            code = &code[..code.len() - self.suffix.len()];
        }
        let code = normalize_table_code(code);
        if code.is_empty() {
            return Vec::new();
        }

        self.with_data(|data| {
            data.entries
                .iter()
                .filter(|entry| {
                    if self.enable_completion {
                        entry.code.starts_with(&code)
                    } else {
                        entry.code == code
                    }
                })
                .map(|entry| {
                    let comment = data
                        .reverse_comments
                        .get(&entry.text)
                        .filter(|comments| !comments.is_empty())
                        .map(|comments| self.comment_format.apply(&comments.join("; ")))
                        .unwrap_or_else(|| entry.code.clone());
                    let quality = if self.enable_completion && has_prefix && entry.code == code {
                        entry.weight + 1_000_000.0
                    } else {
                        entry.weight
                    };
                    Candidate {
                        text: entry.text.clone(),
                        comment,
                        preedit: None,
                        source: CandidateSource::ReverseLookup,
                        quality,
                    }
                })
                .collect()
        })
        .unwrap_or_default()
    }

    fn translate_with_context(
        &self,
        input: &str,
        _status: &Status,
        _options: &HashMap<String, bool>,
        context: &Context,
    ) -> Vec<Candidate> {
        if !self.accepts_segment_tags(&context.segment_tags) {
            return Vec::new();
        }
        self.translate(input)
    }

    fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let mut rows = self.storage.memory_owner_rows();
        rows.push(MemoryOwnerRow::new(
            "reverse_lookup.config",
            MemoryOwnerClass::HeapOwnedRequired,
            self.prefix
                .capacity()
                .saturating_add(self.suffix.capacity())
                .saturating_add(self.tag.capacity())
                .saturating_add(mem::size_of::<CommentFormat>()),
            1,
            "ReverseLookupTranslator",
            "prefix/suffix/tag/comment-format state for the reverse lookup translator",
        ));
        rows
    }
}

pub struct HistoryTranslator {
    input: String,
    size: usize,
    initial_quality: f32,
    tag: String,
}

impl HistoryTranslator {
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            size: 1,
            initial_quality: 1000.0,
            tag: "abc".to_owned(),
        }
    }

    #[must_use]
    pub const fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub const fn with_initial_quality(mut self, initial_quality: f32) -> Self {
        self.initial_quality = initial_quality;
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        if self.tag.is_empty() {
            self.tag = "abc".to_owned();
        }
        self
    }

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        segment_tags
            .iter()
            .any(|segment_tag| segment_tag == &self.tag)
    }
}

impl Translator for HistoryTranslator {
    fn name(&self) -> &'static str {
        "history_translator"
    }

    fn translate(&self, _input: &str) -> Vec<Candidate> {
        Vec::new()
    }

    fn translate_with_context(
        &self,
        input: &str,
        _status: &Status,
        _options: &HashMap<String, bool>,
        context: &Context,
    ) -> Vec<Candidate> {
        if !self.accepts_segment_tags(&context.segment_tags)
            || self.input.is_empty()
            || self.input != input
        {
            return Vec::new();
        }

        context
            .commit_history
            .iter()
            .rev()
            .filter(|record| record.candidate_type != "thru")
            .take(self.size)
            .map(|record| Candidate {
                text: record.text.clone(),
                comment: String::new(),
                preedit: None,
                source: CandidateSource::History,
                quality: self.initial_quality,
            })
            .collect()
    }
}

impl StaticTableTranslator {
    fn normal_code_contains(&self, code: &str) -> bool {
        self.normal_codes.contains(&self.storage, code)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SwitchTranslatorSwitch {
    Toggle {
        option_name: String,
        states: [String; 2],
        abbrev: [Option<String>; 2],
    },
    Radio {
        options: Vec<String>,
        states: Vec<String>,
        abbrev: Vec<Option<String>>,
    },
}

impl SwitchTranslatorSwitch {
    #[must_use]
    pub fn toggle(
        option_name: impl Into<String>,
        state0: impl Into<String>,
        state1: impl Into<String>,
    ) -> Self {
        Self::Toggle {
            option_name: option_name.into(),
            states: [state0.into(), state1.into()],
            abbrev: [None, None],
        }
    }

    #[must_use]
    pub fn radio(
        options: impl IntoIterator<Item = impl Into<String>>,
        states: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::Radio {
            options: options.into_iter().map(Into::into).collect(),
            states: states.into_iter().map(Into::into).collect(),
            abbrev: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_abbrev(
        mut self,
        abbrev: impl IntoIterator<Item = Option<impl Into<String>>>,
    ) -> Self {
        match &mut self {
            Self::Toggle { abbrev: values, .. } => {
                for (index, value) in abbrev.into_iter().take(2).enumerate() {
                    values[index] = value.map(Into::into);
                }
            }
            Self::Radio { abbrev: values, .. } => {
                *values = abbrev
                    .into_iter()
                    .map(|value| value.map(Into::into))
                    .collect();
            }
        }
        self
    }
}

pub struct SwitchTranslator {
    switches: Vec<SwitchTranslatorSwitch>,
    folded_options: FoldedSwitchOptions,
}

impl SwitchTranslator {
    #[must_use]
    pub fn new(switches: impl IntoIterator<Item = SwitchTranslatorSwitch>) -> Self {
        Self {
            switches: switches.into_iter().collect(),
            folded_options: FoldedSwitchOptions::default(),
        }
    }

    #[must_use]
    pub fn with_folded_options(mut self, folded_options: FoldedSwitchOptions) -> Self {
        self.folded_options = folded_options;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FoldedSwitchOptions {
    pub prefix: String,
    pub suffix: String,
    pub separator: String,
    pub abbreviate_options: bool,
}

impl Default for FoldedSwitchOptions {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            suffix: String::new(),
            separator: " ".to_owned(),
            abbreviate_options: false,
        }
    }
}

impl Translator for SwitchTranslator {
    fn name(&self) -> &'static str {
        "switch_translator"
    }

    fn translate(&self, _input: &str) -> Vec<Candidate> {
        Vec::new()
    }

    fn translate_with_state(
        &self,
        input: &str,
        _status: &Status,
        runtime_options: &HashMap<String, bool>,
    ) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        for the_switch in &self.switches {
            match the_switch {
                SwitchTranslatorSwitch::Toggle {
                    option_name,
                    states,
                    ..
                } => {
                    let current_state = runtime_options.get(option_name).copied().unwrap_or(false);
                    let current_index = usize::from(current_state);
                    candidates.push(Candidate {
                        text: states[current_index].clone(),
                        comment: format!("→ {}", states[1 - current_index]),
                        preedit: None,
                        source: CandidateSource::Switch,
                        quality: 0.5,
                    });
                }
                SwitchTranslatorSwitch::Radio {
                    options, states, ..
                } => {
                    if options.is_empty() || states.is_empty() {
                        continue;
                    }
                    let selected_index = options
                        .iter()
                        .position(|option| options_get_bool(runtime_options, option))
                        .unwrap_or(0);
                    for (option_index, state) in states.iter().enumerate().take(options.len()) {
                        if state.is_empty() {
                            continue;
                        }
                        candidates.push(Candidate {
                            text: state.clone(),
                            comment: if option_index == selected_index {
                                " ✓".to_owned()
                            } else {
                                String::new()
                            },
                            preedit: None,
                            source: CandidateSource::Switch,
                            quality: 0.5,
                        });
                    }
                }
            }
        }
        if options_get_bool(runtime_options, "_fold_options") {
            let labels = self.folded_option_labels(runtime_options);
            if labels.len() > 1 {
                return vec![Candidate {
                    text: format!(
                        "{}{}{}",
                        self.folded_options.prefix,
                        labels.join(&self.folded_options.separator),
                        self.folded_options.suffix
                    ),
                    comment: String::new(),
                    preedit: None,
                    source: CandidateSource::Unfold,
                    quality: 0.5,
                }];
            }
        }
        candidates
    }
}

impl SwitchTranslator {
    fn folded_option_labels(&self, runtime_options: &HashMap<String, bool>) -> Vec<String> {
        let mut labels = Vec::new();
        for the_switch in &self.switches {
            match the_switch {
                SwitchTranslatorSwitch::Toggle {
                    option_name,
                    states,
                    abbrev,
                } => {
                    let current_state =
                        usize::from(runtime_options.get(option_name).copied().unwrap_or(false));
                    if !states
                        .get(current_state)
                        .is_some_and(|state| !state.is_empty())
                    {
                        continue;
                    }
                    labels.push(folded_state_label(
                        &states[current_state],
                        abbrev.get(current_state).and_then(Option::as_deref),
                        self.folded_options.abbreviate_options,
                    ));
                }
                SwitchTranslatorSwitch::Radio {
                    options,
                    states,
                    abbrev,
                } => {
                    let selected_index = options
                        .iter()
                        .position(|option| options_get_bool(runtime_options, option))
                        .unwrap_or(0);
                    if !states
                        .get(selected_index)
                        .is_some_and(|state| !state.is_empty())
                    {
                        continue;
                    }
                    labels.push(folded_state_label(
                        &states[selected_index],
                        abbrev.get(selected_index).and_then(Option::as_deref),
                        self.folded_options.abbreviate_options,
                    ));
                }
            }
        }
        labels
    }
}

pub struct SchemaListTranslator {
    entries: Vec<(String, String)>,
    hide_lone_schema: bool,
}

impl SchemaListTranslator {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(schema_id, schema_name)| (schema_id.into(), schema_name.into()))
                .collect(),
            hide_lone_schema: false,
        }
    }

    #[must_use]
    pub const fn with_hide_lone_schema(mut self, hide_lone_schema: bool) -> Self {
        self.hide_lone_schema = hide_lone_schema;
        self
    }
}

impl Translator for SchemaListTranslator {
    fn name(&self) -> &'static str {
        "schema_list_translator"
    }

    fn translate(&self, _input: &str) -> Vec<Candidate> {
        Vec::new()
    }

    fn translate_with_status(&self, input: &str, status: &Status) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }
        if self.hide_lone_schema && self.entries.is_empty() {
            return Vec::new();
        }

        let mut candidates = vec![Candidate {
            text: status.schema_name.clone(),
            comment: String::new(),
            preedit: None,
            source: CandidateSource::Schema,
            quality: 0.5,
        }];
        candidates.extend(
            self.entries
                .iter()
                .filter(|(schema_id, _)| schema_id != &status.schema_id)
                .map(|(_, schema_name)| Candidate {
                    text: schema_name.clone(),
                    comment: String::new(),
                    preedit: None,
                    source: CandidateSource::Schema,
                    quality: 0.5,
                }),
        );
        candidates
    }
}

fn folded_state_label(state: &str, abbrev: Option<&str>, abbreviate: bool) -> String {
    if !abbreviate {
        return state.to_owned();
    }
    if let Some(abbrev) = abbrev {
        return abbrev.to_owned();
    }
    state.chars().next().into_iter().collect()
}

fn abbreviation_preedit_from_spans(
    input: &str,
    boundaries: &[usize],
    spans: &[SentenceCodeSpan],
) -> Option<String> {
    let mut raw_spans_by_start = vec![Vec::<usize>::new(); boundaries.len()];
    for span in spans {
        let Ok(start_index) = boundaries.binary_search(&span.start) else {
            continue;
        };
        if boundaries.binary_search(&span.end).is_err() {
            continue;
        }
        raw_spans_by_start[start_index].push(span.end);
    }
    for ends in &mut raw_spans_by_start {
        ends.sort_unstable();
        ends.dedup();
    }

    let mut coverable = vec![false; boundaries.len()];
    if let Some(last) = coverable.last_mut() {
        *last = true;
    }
    for start_index in (0..boundaries.len().saturating_sub(1)).rev() {
        coverable[start_index] = raw_spans_by_start[start_index].iter().any(|end| {
            boundaries
                .binary_search(end)
                .is_ok_and(|end_index| coverable[end_index])
        });
    }
    if !coverable.first().copied().unwrap_or(false) {
        return None;
    }

    let mut pieces = Vec::new();
    let mut start_index = 0usize;
    while boundaries[start_index] < input.len() {
        let start = boundaries[start_index];
        let end = raw_spans_by_start[start_index]
            .iter()
            .copied()
            .rev()
            .find(|end| {
                boundaries
                    .binary_search(end)
                    .is_ok_and(|end_index| coverable[end_index])
            })?;
        pieces.push(input[start..end].to_owned());
        start_index = boundaries.binary_search(&end).ok()?;
    }
    Some(pieces.join(" "))
}

fn formula_is_abbreviation(formula: &str) -> bool {
    formula.starts_with("abbrev/") || formula.contains("/abbrev")
}

fn options_get_bool(options: &HashMap<String, bool>, option: &str) -> bool {
    options.get(option).copied().unwrap_or(false)
}
