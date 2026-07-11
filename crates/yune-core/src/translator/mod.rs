use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque};
use std::mem;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::comment_format::CommentFormat;
use crate::dictionary::{
    normalize_table_code, CompactTableStore, LookupCandidate, LookupCandidateEntry,
    RimePrismBinPayload, RimePrismRuntimePayload, TableLookup,
};
use crate::filter::contains_extended_cjk;
use crate::poet::{GrammarProvider, PoetByteSource, SentenceCodeSpan, UpstreamSentenceModel};
use crate::spelling_algebra::{DeployedSpellingType, ExpandedSpellingEntry, SpellingAlgebra};
use crate::{
    Candidate, CandidateRequest, CandidateSource, Context, M37SentenceCandidateMetrics,
    MemoryOwnerClass, MemoryOwnerRow, PresetVocabularyEntry, RimeCorrectionEntry,
    RimeToleranceRule, SpellingAlgebraDebug, Status, StorageDiagnosticsRow, TableDictionary,
    TableDictionaryParseError, TableEntry, TranslationResult, Translator, TranslatorScratch,
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
const PREFIX_FALLBACK_BOUNDED_CANDIDATES_PER_FETCH_CODE: usize = 2;
const PREFIX_FALLBACK_BOUNDED_REACHABILITY_CANDIDATES_PER_FETCH_CODE: usize = 3;
const PREFIX_FALLBACK_BOUNDED_REACHABILITY_MAX_INPUT_CHARS: usize = 7;
const PREFIX_FALLBACK_BOUNDED_PENDING_MULTIPLIER: usize = 4;
const PREFIX_FALLBACK_CACHE_MAX_ROWS: usize = 128;
const PREFIX_FALLBACK_CACHE_MAX_PREFIXES: usize = 64;
const PREFIX_FALLBACK_CACHE_MAX_KEY_BYTES: usize = 32 * 1024;
const PREFIX_FALLBACK_CACHE_MAX_ENTRY_BYTES: usize = 512 * 1024;
/// Yune-internal heuristic calibrated to the M21 TypeDuck v1.1.2 sentence-composition fixture
/// and the M28 follow-up upstream-Jyutping composition fixture; install only for the
/// jyut6ping3 TypeDuck profile.
pub const TYPEDUCK_SENTENCE_WORD_PENALTY: f32 = 24.0;

#[derive(Clone, Debug, PartialEq)]
struct LookupCodeSpec {
    code: String,
    lookup_code: String,
    correction_distance: Option<usize>,
    required_syllable_count: Option<usize>,
    tolerance: bool,
    spelling_correction: bool,
    spelling_credibility: f32,
}

impl LookupCodeSpec {
    fn exact(code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            lookup_code: code.clone(),
            code,
            correction_distance: None,
            required_syllable_count: None,
            tolerance: false,
            spelling_correction: false,
            spelling_credibility: 0.0,
        }
    }

    /// A tolerance-rule near-match: ranked with the exacts of its own code but
    /// excluded from the tone-merge re-rank domain (the user did not type this code).
    fn tolerance_exact(code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            lookup_code: code.clone(),
            code,
            correction_distance: None,
            required_syllable_count: None,
            tolerance: true,
            spelling_correction: false,
            spelling_credibility: 0.0,
        }
    }

    fn alias(
        code: impl Into<String>,
        lookup_code: impl Into<String>,
        spelling_correction: bool,
        spelling_credibility: f32,
    ) -> Self {
        Self {
            code: code.into(),
            lookup_code: lookup_code.into(),
            correction_distance: None,
            required_syllable_count: None,
            tolerance: false,
            spelling_correction,
            spelling_credibility,
        }
    }

    fn correction(code: impl Into<String>, distance: usize) -> Self {
        let code = code.into();
        Self {
            lookup_code: code.clone(),
            code,
            correction_distance: Some(distance),
            required_syllable_count: None,
            tolerance: false,
            spelling_correction: false,
            spelling_credibility: 0.0,
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
            tolerance: false,
            spelling_correction: false,
            spelling_credibility: 0.0,
        }
    }
}

#[derive(Clone)]
struct PendingLookupCandidate {
    fetch_group: usize,
    entry_code: String,
    lookup_code: String,
    candidate: Candidate,
    correction_distance: Option<usize>,
    spelling_abbreviation: bool,
    limited_prediction: bool,
    tolerance: bool,
    spelling_correction: bool,
    spelling_credibility: f32,
}

impl PendingLookupCandidate {
    fn raw_quality(&self) -> f32 {
        let mut quality = self.candidate.quality + self.spelling_credibility;
        if let Some(distance) = self.correction_distance {
            quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        quality
    }

    fn prediction_comparison_weight(&self) -> f32 {
        let mut weight =
            self.candidate.quality.max(f32::MIN_POSITIVE).ln() + self.spelling_credibility;
        if let Some(distance) = self.correction_distance {
            weight += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        weight
    }

    fn prediction_precedes(&self, ordinary: &Self) -> bool {
        let interpreted =
            complete_syllable_prefix_count(&self.candidate.comment, &self.lookup_code);
        let consumed = source_code_syllable_count(&ordinary.candidate.comment);
        self.prediction_comparison_weight() > ordinary.prediction_comparison_weight()
            || interpreted
                .zip(consumed)
                .is_some_and(|(interpreted, consumed)| consumed < interpreted)
    }
}

struct PendingLookupCandidateRef<'a> {
    fetch_group: usize,
    entry_code: Cow<'a, str>,
    lookup_code: &'a str,
    candidate: LookupCandidate<'a>,
    correction_distance: Option<usize>,
    spelling_abbreviation: bool,
    limited_prediction: bool,
    emission_order: usize,
    spelling_correction: bool,
    spelling_credibility: f32,
}

struct OriginalMergeGroup<T> {
    category: u8,
    candidates: VecDeque<T>,
}

#[derive(Clone, Copy, Debug)]
struct OriginalMergeHead {
    category: u8,
    group_index: usize,
    raw_quality: f32,
}

impl PartialEq for OriginalMergeHead {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for OriginalMergeHead {}

impl PartialOrd for OriginalMergeHead {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OriginalMergeHead {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap pops the greatest item. Earlier categories are complete
        // phases, while a source-earlier group wins equal-quality heads.
        other
            .category
            .cmp(&self.category)
            .then_with(|| self.raw_quality.total_cmp(&other.raw_quality))
            .then_with(|| other.group_index.cmp(&self.group_index))
    }
}

enum OriginalMergeFrontier {
    Heap(BinaryHeap<OriginalMergeHead>),
    Scan,
}

fn pop_original_merge_head<T>(
    groups: &mut [OriginalMergeGroup<T>],
    frontier: &mut OriginalMergeFrontier,
    raw_quality: &mut impl FnMut(&T) -> f32,
) -> Option<T> {
    let head = match frontier {
        OriginalMergeFrontier::Heap(heads) => heads.pop()?,
        OriginalMergeFrontier::Scan => {
            let mut best: Option<OriginalMergeHead> = None;
            for (group_index, group) in groups.iter().enumerate() {
                let Some(candidate) = group.candidates.front() else {
                    continue;
                };
                let candidate_head = OriginalMergeHead {
                    category: group.category,
                    group_index,
                    raw_quality: raw_quality(candidate),
                };
                if best.as_ref().map_or(true, |best| {
                    candidate_head.category < best.category
                        || (candidate_head.category == best.category
                            && candidate_head.raw_quality > best.raw_quality)
                }) {
                    best = Some(candidate_head);
                }
            }
            best?
        }
    };
    let group = &mut groups[head.group_index];
    let candidate = group
        .candidates
        .pop_front()
        .expect("merge head should reference a nonempty candidate group");
    if let OriginalMergeFrontier::Heap(heads) = frontier {
        if let Some(next) = group.candidates.front() {
            heads.push(OriginalMergeHead {
                category: group.category,
                group_index: head.group_index,
                raw_quality: normalize_original_merge_quality(raw_quality(next)),
            });
        }
    }
    Some(candidate)
}

fn normalize_original_merge_quality(raw_quality: f32) -> f32 {
    if raw_quality == 0.0 {
        0.0
    } else {
        raw_quality
    }
}

#[allow(clippy::too_many_arguments)]
fn order_original_grouped_candidates<T>(
    candidates: &mut Vec<T>,
    output_limit: Option<usize>,
    mut is_prediction: impl FnMut(&T) -> bool,
    mut category: impl FnMut(&T) -> u8,
    mut fetch_group: impl FnMut(&T) -> usize,
    mut raw_quality: impl FnMut(&T) -> f32,
    mut prediction_order: impl FnMut(&T, &T) -> Ordering,
    mut prediction_precedes: impl FnMut(&T, &T) -> bool,
) -> usize {
    if output_limit == Some(0) {
        candidates.clear();
        return 0;
    }

    let mut predictions = Vec::new();
    let mut ordinary = Vec::new();
    for candidate in std::mem::take(candidates) {
        if is_prediction(&candidate) {
            predictions.push(candidate);
        } else {
            ordinary.push(candidate);
        }
    }
    predictions.sort_by(&mut prediction_order);

    let mut groups: Vec<OriginalMergeGroup<T>> = Vec::new();
    let mut group_indices: HashMap<(u8, usize), usize> = HashMap::new();
    for candidate in ordinary {
        let candidate_category = category(&candidate);
        let key = fetch_group(&candidate);
        if let Some(index) = group_indices.get(&(candidate_category, key)).copied() {
            groups[index].candidates.push_back(candidate);
        } else {
            group_indices.insert((candidate_category, key), groups.len());
            groups.push(OriginalMergeGroup {
                category: candidate_category,
                candidates: VecDeque::from([candidate]),
            });
        }
    }

    // A finite/ordered f32 head key gives the heap a real total order. Preserve
    // the former strict-`>` first-seen semantics for pathological NaN table
    // weights by falling back to the old scan only for that malformed pool.
    let has_nan_quality = groups.iter().any(|group| {
        group
            .candidates
            .iter()
            .any(|candidate| raw_quality(candidate).is_nan())
    });
    let mut frontier = if has_nan_quality {
        OriginalMergeFrontier::Scan
    } else {
        let mut heads = BinaryHeap::with_capacity(groups.len());
        for (group_index, group) in groups.iter().enumerate() {
            let head = group
                .candidates
                .front()
                .expect("new candidate group should be nonempty");
            heads.push(OriginalMergeHead {
                category: group.category,
                group_index,
                raw_quality: normalize_original_merge_quality(raw_quality(head)),
            });
        }
        OriginalMergeFrontier::Heap(heads)
    };

    // ScriptTranslation does not let a limited prediction keep an otherwise
    // empty translation alive, and it always emits one ordinary candidate
    // before considering prediction interleaving.
    let Some(first) = pop_original_merge_head(&mut groups, &mut frontier, &mut raw_quality) else {
        return 0;
    };
    let mut ordinary_pops = 1;
    let remaining_total = groups
        .iter()
        .map(|group| group.candidates.len())
        .sum::<usize>()
        .saturating_add(predictions.len())
        .saturating_add(1);
    let capacity = output_limit.map_or(remaining_total, |limit| limit.min(remaining_total));
    let mut ordered = Vec::with_capacity(capacity);
    ordered.push(first);
    if output_limit.is_some_and(|limit| ordered.len() >= limit) {
        *candidates = ordered;
        return ordinary_pops;
    }

    let mut predictions = VecDeque::from(predictions);
    while let Some(next_ordinary) =
        pop_original_merge_head(&mut groups, &mut frontier, &mut raw_quality)
    {
        ordinary_pops += 1;
        while predictions
            .front()
            .is_some_and(|prediction| prediction_precedes(prediction, &next_ordinary))
        {
            ordered.push(
                predictions
                    .pop_front()
                    .expect("checked prediction queue should be nonempty"),
            );
            if output_limit.is_some_and(|limit| ordered.len() >= limit) {
                *candidates = ordered;
                return ordinary_pops;
            }
        }
        ordered.push(next_ordinary);
        if output_limit.is_some_and(|limit| ordered.len() >= limit) {
            *candidates = ordered;
            return ordinary_pops;
        }
    }
    while let Some(prediction) = predictions.pop_front() {
        ordered.push(prediction);
        if output_limit.is_some_and(|limit| ordered.len() >= limit) {
            break;
        }
    }
    *candidates = ordered;
    ordinary_pops
}

#[cfg(test)]
mod original_merge_algorithm_tests {
    use std::cmp::Ordering;

    use super::{order_original_grouped_candidates, OriginalMergeHead};

    #[derive(Debug)]
    struct TestCandidate {
        group: usize,
        category: u8,
        quality: f32,
    }

    #[test]
    fn sort_original_merge_head_equality_matches_total_order() {
        let nan = OriginalMergeHead {
            category: 0,
            group_index: 0,
            raw_quality: f32::NAN,
        };
        assert_eq!(nan, nan);

        let negative_zero = OriginalMergeHead {
            raw_quality: -0.0,
            ..nan
        };
        let positive_zero = OriginalMergeHead {
            raw_quality: 0.0,
            ..nan
        };
        assert_ne!(negative_zero, positive_zero);
        assert_ne!(negative_zero.cmp(&positive_zero), Ordering::Equal);
    }

    #[test]
    fn sort_original_bounded_heap_pops_only_requested_ordinary_prefix() {
        let mut candidates = (0..128)
            .map(|group| TestCandidate {
                group,
                category: if group == 126 {
                    1
                } else if group == 127 {
                    2
                } else {
                    0
                },
                quality: if group >= 126 {
                    1_000_000.0
                } else {
                    1_000.0 - group as f32
                },
            })
            .collect::<Vec<_>>();

        let ordinary_pops = order_original_grouped_candidates(
            &mut candidates,
            Some(5),
            |_| false,
            |candidate| candidate.category,
            |candidate| candidate.group,
            |candidate| candidate.quality,
            |_, _| Ordering::Equal,
            |_, _| false,
        );

        assert_eq!(ordinary_pops, 5);
        assert_eq!(candidates.len(), 5);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.group)
                .collect::<Vec<_>>(),
            [0, 1, 2, 3, 4]
        );
    }

    #[test]
    fn sort_original_nan_quality_uses_legacy_first_seen_scan_semantics() {
        let mut candidates = vec![
            TestCandidate {
                group: 0,
                category: 0,
                quality: 1.0,
            },
            TestCandidate {
                group: 1,
                category: 0,
                quality: f32::NAN,
            },
            TestCandidate {
                group: 2,
                category: 0,
                quality: 2.0,
            },
        ];

        order_original_grouped_candidates(
            &mut candidates,
            None,
            |_| false,
            |candidate| candidate.category,
            |candidate| candidate.group,
            |candidate| candidate.quality,
            |_, _| Ordering::Equal,
            |_, _| false,
        );

        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.group)
                .collect::<Vec<_>>(),
            [2, 0, 1]
        );
    }
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

struct BoundedPrefixFallbackCacheRequest<'input, 'request> {
    input: &'input str,
    lookup_code: &'input str,
    filter_by_charset: bool,
    existing_candidates: &'request [Candidate],
    admitted_span_candidates: &'request [Candidate],
    prefixes: &'request [LookupPrefixSpec<'input>],
    limit: usize,
    fallback_start: Option<Instant>,
}

struct LookupPrefixSpec<'a> {
    input_prefix: &'a str,
    fetch_code: String,
    consumed_lookup_len: usize,
    surface_fetch: Option<LeadingFetchCode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrefixFallbackCachePrefix {
    input_prefix: String,
    fetch_code: String,
    consumed_lookup_len: usize,
    surface_fetch: Option<LeadingFetchCode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrefixFallbackWindowCacheKey {
    prefixes: Vec<PrefixFallbackCachePrefix>,
    filter_by_charset: bool,
    pending_cap: usize,
    per_fetch_cap: usize,
}

#[derive(Clone, Debug)]
struct CachedPrefixFallbackView {
    fetch_code: String,
    input_prefix: String,
    candidate: Candidate,
    consumed_lookup_len: usize,
    surface_abbreviation: bool,
    spelling_abbreviation: bool,
    emission_order: usize,
}

#[derive(Clone, Debug)]
struct PrefixFallbackWindowCacheEntry {
    key: PrefixFallbackWindowCacheKey,
    rows: Vec<CachedPrefixFallbackView>,
    truncated: bool,
}

struct PrefixFallbackBatch {
    candidates: Vec<Candidate>,
    truncated: bool,
    owns_reachability: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PrefixFallbackProbe {
    NoPrefix,
    Found,
    Exhausted,
    Truncated,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct LeadingFetchCode {
    fetch_code: String,
    canonical_code: String,
    bare_exact: bool,
    injectable: bool,
    abbreviation: bool,
    // A direct no-algebra edge carries local identity proof: either an upstream
    // null-map prism descriptor or a bounded exact storage probe plus an exact
    // one-character row. It needs no global syllabary classification and keeps
    // Stroke's 157k inventory off the cold first-key path.
    direct_identity: bool,
}

struct LeadingFetchIndexSeed {
    canonical_codes: Vec<String>,
    fetches_canonical_storage: bool,
    max_leading_single_surface_len: usize,
}

fn sentence_piece_quality(raw_quality: f32, word_penalty: f32) -> f32 {
    raw_quality.max(1.0).ln() - word_penalty
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SentencePathScore {
    pub(crate) fuzzy_pieces: usize,
    pub(crate) quality: f32,
    pub(crate) raw_quality: f32,
}

pub(crate) fn sentence_path_score_replaces(
    candidate: SentencePathScore,
    existing: SentencePathScore,
) -> bool {
    match candidate.fuzzy_pieces.cmp(&existing.fuzzy_pieces) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => match candidate
            .quality
            .partial_cmp(&existing.quality)
            .unwrap_or(Ordering::Equal)
        {
            Ordering::Greater => true,
            Ordering::Equal => candidate.raw_quality > existing.raw_quality,
            Ordering::Less => false,
        },
    }
}

fn retain_winning_sentence_path_candidate<'a>(
    winning_candidate: &mut Option<LookupCandidate<'a>>,
    shadow_score: &mut Option<SentencePathScore>,
    predecessor: SentencePathScore,
    candidate: LookupCandidate<'a>,
    entry_code: &str,
    word_penalty: f32,
) {
    let candidate_score = SentencePathScore {
        fuzzy_pieces: predecessor.fuzzy_pieces
            + usize::from(!raw_sentence_piece_matches_input_code(
                candidate.raw_comment(),
                candidate.text(),
                entry_code,
            )),
        quality: predecessor.quality
            + sentence_piece_quality(candidate.raw_quality(), word_penalty),
        raw_quality: predecessor.raw_quality + candidate.raw_quality(),
    };
    if shadow_score.map_or(true, |existing| {
        sentence_path_score_replaces(candidate_score, existing)
    }) {
        *shadow_score = Some(candidate_score);
        *winning_candidate = Some(candidate);
    }
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

fn estimate_leading_fetch_seed_bytes(seed: &LeadingFetchIndexSeed) -> usize {
    mem::size_of::<LeadingFetchIndexSeed>()
        .saturating_add(
            seed.canonical_codes
                .capacity()
                .saturating_mul(mem::size_of::<String>()),
        )
        .saturating_add(
            seed.canonical_codes
                .iter()
                .map(String::capacity)
                .sum::<usize>(),
        )
}

fn estimate_leading_fetch_index_bytes(index: &HashMap<String, Vec<LeadingFetchCode>>) -> usize {
    mem::size_of::<HashMap<String, Vec<LeadingFetchCode>>>()
        .saturating_add(
            index
                .capacity()
                .saturating_mul(mem::size_of::<(String, Vec<LeadingFetchCode>)>()),
        )
        .saturating_add(
            index
                .iter()
                .map(|(surface, edges)| {
                    surface
                        .capacity()
                        .saturating_add(
                            edges
                                .capacity()
                                .saturating_mul(mem::size_of::<LeadingFetchCode>()),
                        )
                        .saturating_add(
                            edges
                                .iter()
                                .map(|edge| {
                                    edge.fetch_code
                                        .capacity()
                                        .saturating_add(edge.canonical_code.capacity())
                                })
                                .sum::<usize>(),
                        )
                })
                .sum::<usize>(),
        )
}

fn estimate_prefix_fallback_window_cache_bytes(entry: &PrefixFallbackWindowCacheEntry) -> usize {
    let key_bytes = entry
        .key
        .prefixes
        .iter()
        .map(|prefix| {
            prefix
                .input_prefix
                .capacity()
                .saturating_add(prefix.fetch_code.capacity())
                .saturating_add(prefix.surface_fetch.as_ref().map_or(0, |fetch| {
                    fetch
                        .fetch_code
                        .capacity()
                        .saturating_add(fetch.canonical_code.capacity())
                }))
        })
        .sum::<usize>();
    let row_bytes = entry
        .rows
        .iter()
        .map(|row| {
            row.fetch_code
                .capacity()
                .saturating_add(row.input_prefix.capacity())
                .saturating_add(row.candidate.text.capacity())
                .saturating_add(row.candidate.comment.capacity())
                .saturating_add(row.candidate.preedit.as_ref().map_or(0, String::capacity))
        })
        .sum::<usize>();
    mem::size_of::<PrefixFallbackWindowCacheEntry>()
        .saturating_add(
            entry
                .key
                .prefixes
                .capacity()
                .saturating_mul(mem::size_of::<PrefixFallbackCachePrefix>()),
        )
        .saturating_add(key_bytes)
        .saturating_add(
            entry
                .rows
                .capacity()
                .saturating_mul(mem::size_of::<CachedPrefixFallbackView>()),
        )
        .saturating_add(row_bytes)
}

fn prefix_fallback_cache_key_bytes(prefixes: &[LookupPrefixSpec<'_>]) -> usize {
    mem::size_of::<PrefixFallbackWindowCacheKey>()
        .saturating_add(
            prefixes
                .len()
                .saturating_mul(mem::size_of::<PrefixFallbackCachePrefix>()),
        )
        .saturating_add(prefixes.iter().fold(0usize, |bytes, prefix| {
            bytes
                .saturating_add(prefix.input_prefix.len())
                .saturating_add(prefix.fetch_code.len())
                .saturating_add(prefix.surface_fetch.as_ref().map_or(0, |fetch| {
                    fetch
                        .fetch_code
                        .len()
                        .saturating_add(fetch.canonical_code.len())
                }))
        }))
}

fn estimate_prefix_fallback_window_cache_key_bytes(key: &PrefixFallbackWindowCacheKey) -> usize {
    mem::size_of::<PrefixFallbackWindowCacheKey>()
        .saturating_add(
            key.prefixes
                .capacity()
                .saturating_mul(mem::size_of::<PrefixFallbackCachePrefix>()),
        )
        .saturating_add(key.prefixes.iter().fold(0usize, |bytes, prefix| {
            bytes
                .saturating_add(prefix.input_prefix.capacity())
                .saturating_add(prefix.fetch_code.capacity())
                .saturating_add(prefix.surface_fetch.as_ref().map_or(0, |fetch| {
                    fetch
                        .fetch_code
                        .capacity()
                        .saturating_add(fetch.canonical_code.capacity())
                }))
        }))
}

fn estimate_string_triple_hash_set_bytes(values: &HashSet<(String, String, String)>) -> usize {
    mem::size_of::<HashSet<(String, String, String)>>()
        .saturating_add(
            values
                .capacity()
                .saturating_mul(mem::size_of::<(String, String, String)>()),
        )
        .saturating_add(
            values
                .iter()
                .map(|(first, second, third)| {
                    first
                        .capacity()
                        .saturating_add(second.capacity())
                        .saturating_add(third.capacity())
                })
                .sum::<usize>(),
        )
}

pub struct StaticTableTranslator {
    source_entries: Option<Vec<(String, Candidate)>>,
    storage: TableStorage,
    prism_payload: Option<RimePrismRuntimePayload>,
    // The compact prism is supplied before schema algebra is installed. The
    // first algebra configuration binds that prism to the deployed formulas;
    // replacing the formulas later invalidates direct surface traversal while
    // retaining the payload for its other metadata.
    direct_prism_surface_mapping_current: bool,
    spelling_algebra_configured: bool,
    spelling_algebra_active: bool,
    /// RIME `sort:` policy of the backing dictionary; false (`sort: original`)
    /// disables the M59 tone-merge re-rank — source row order is the contract.
    sort_by_weight: bool,
    spelling_abbreviation_entries: HashSet<(String, String, String)>,
    spelling_correction_entries: HashSet<(String, String, String)>,
    spelling_correction_surfaces: HashSet<String>,
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
    // Complete-list leading-syllable single-character reachability (M59), a
    // narrow capability distinct from the broad TypeDuck `prefix_fallback`:
    // it exposes the leading syllable's single-char family on the page-turn/
    // complete path (and only *signals* its existence on the bounded typing
    // path, without materializing or reordering page 1).
    leading_syllable_reachability: bool,
    // M59 finding #6: cached structural classification of the dictionary as
    // untoned (luna `mo`) vs toned (jyutping `bei2`). The untoned-relaxation in
    // the leading-single filter is keyed on THIS (code structure), not on the
    // `leading_syllable_reachability` flag, so the default-ON schema-general flip
    // cannot admit digit-less rows into a toned family (shifting M58 pins).
    // Lazily computed once from `storage`, then cached.
    untoned_dictionary_cache: OnceLock<bool>,
    // M59 finding #8: memoized `normalized_original_code(code) -> [storage codes]`
    // index for the leading-syllable fetch. Without it every prefix boundary of
    // every keystroke rescanned the whole syllabary and allocated a String per
    // entry (~15-25k allocs/keystroke on 37/59-char rows). Built once, then O(1).
    leading_fetch_index_cache: OnceLock<HashMap<String, Vec<LeadingFetchCode>>>,
    // Source-ordered canonical codes retained across heap algebra expansion so
    // the surface index itself can stay lazy. Leading-only reachability keeps
    // just groups with a single-character candidate; heap/source prefix fallback
    // (which has no current prism to query) keeps every canonical group, including
    // phrase-only rows. The expanded surface map is allocated only on lookup.
    leading_fetch_index_seed: Option<LeadingFetchIndexSeed>,
    // M59 finding #8 (range cap): the longest normalized syllabary-code byte
    // length. A leading syllable can never be longer than this, so the
    // longest-first prefix walk only needs to consider boundaries up to it —
    // bounding the per-keystroke walk to O(max_syllable_len) instead of
    // O(input_len) and collapsing the long-input O(n^2) to O(n). Cached once.
    max_leading_prefix_len_cache: OnceLock<usize>,
    // One bounded raw prefix window is retained across incremental keystrokes.
    // The key contains the resolved deployed-surface fetch graph, so a longer
    // input that keeps the same leading family can reuse immutable raw rows
    // while recomputing span promotion, ordering, and deduplication per call.
    // A single entry bounds ownership independently of typing history.
    prefix_fallback_window_cache: Mutex<Option<Arc<PrefixFallbackWindowCacheEntry>>>,
    sentence_word_penalty: f32,
    spelling_algebra_formulas: Vec<String>,
    preset_vocabulary: Vec<PresetVocabularyEntry>,
    abbreviation_preset_vocabulary: Vec<PresetVocabularyEntry>,
    upstream_sentence_grammar: GrammarProvider,
    upstream_sentence_poet_source: Option<(Arc<dyn PoetByteSource>, u32)>,
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
            direct_prism_surface_mapping_current: false,
            spelling_algebra_configured: false,
            spelling_algebra_active: false,
            sort_by_weight: true,
            spelling_abbreviation_entries: HashSet::new(),
            spelling_correction_entries: HashSet::new(),
            spelling_correction_surfaces: HashSet::new(),
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
            leading_syllable_reachability: false,
            untoned_dictionary_cache: OnceLock::new(),
            leading_fetch_index_cache: OnceLock::new(),
            leading_fetch_index_seed: None,
            max_leading_prefix_len_cache: OnceLock::new(),
            prefix_fallback_window_cache: Mutex::new(None),
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary: Vec::new(),
            abbreviation_preset_vocabulary: Vec::new(),
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_poet_source: None,
            upstream_sentence_model: None,
        }
    }

    #[must_use]
    pub fn from_dictionary(dictionary: TableDictionary) -> Self {
        let sort_by_weight = dictionary.sort_by_weight();
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
            direct_prism_surface_mapping_current: false,
            spelling_algebra_configured: false,
            spelling_algebra_active: false,
            sort_by_weight,
            spelling_abbreviation_entries: HashSet::new(),
            spelling_correction_entries: HashSet::new(),
            spelling_correction_surfaces: HashSet::new(),
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
            leading_syllable_reachability: false,
            untoned_dictionary_cache: OnceLock::new(),
            leading_fetch_index_cache: OnceLock::new(),
            leading_fetch_index_seed: None,
            max_leading_prefix_len_cache: OnceLock::new(),
            prefix_fallback_window_cache: Mutex::new(None),
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_poet_source: None,
            upstream_sentence_model: None,
        }
    }

    #[must_use]
    pub fn from_compact_dictionary(
        dictionary: TableDictionary,
        prism_payload: Option<RimePrismBinPayload>,
    ) -> Self {
        let direct_prism_surface_mapping_current = prism_payload.is_some();
        let sort_by_weight = dictionary.sort_by_weight();
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
            direct_prism_surface_mapping_current,
            spelling_algebra_configured: false,
            spelling_algebra_active: false,
            sort_by_weight,
            spelling_abbreviation_entries: HashSet::new(),
            spelling_correction_entries: HashSet::new(),
            spelling_correction_surfaces: HashSet::new(),
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
            leading_syllable_reachability: false,
            untoned_dictionary_cache: OnceLock::new(),
            leading_fetch_index_cache: OnceLock::new(),
            leading_fetch_index_seed: None,
            max_leading_prefix_len_cache: OnceLock::new(),
            prefix_fallback_window_cache: Mutex::new(None),
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_poet_source: None,
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
        let direct_prism_surface_mapping_current = prism_payload.is_some();
        let sort_by_weight = store.sort_by_weight();
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
            direct_prism_surface_mapping_current,
            spelling_algebra_configured: false,
            spelling_algebra_active: false,
            sort_by_weight,
            spelling_abbreviation_entries: HashSet::new(),
            spelling_correction_entries: HashSet::new(),
            spelling_correction_surfaces: HashSet::new(),
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
            leading_syllable_reachability: false,
            untoned_dictionary_cache: OnceLock::new(),
            leading_fetch_index_cache: OnceLock::new(),
            leading_fetch_index_seed: None,
            max_leading_prefix_len_cache: OnceLock::new(),
            prefix_fallback_window_cache: Mutex::new(None),
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_poet_source: None,
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
        self.prefix_fallback_window_cache = Mutex::new(None);
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
        self.prefix_fallback_window_cache = Mutex::new(None);
        self.leading_fetch_index_seed = None;
        self.leading_fetch_index_cache = OnceLock::new();
        self.max_leading_prefix_len_cache = OnceLock::new();
        if self.requires_surface_fetch_index_seed() {
            let fetches_canonical_storage =
                matches!(self.storage, TableStorage::Compact(_)) && self.prism_payload.is_some();
            self.leading_fetch_index_seed =
                Some(self.capture_leading_fetch_index_seed(fetches_canonical_storage));
        }
        self
    }

    #[must_use]
    pub fn with_leading_syllable_reachability(
        mut self,
        leading_syllable_reachability: bool,
    ) -> Self {
        self.leading_syllable_reachability = leading_syllable_reachability;
        self.prefix_fallback_window_cache = Mutex::new(None);
        self.leading_fetch_index_seed = None;
        self.leading_fetch_index_cache = OnceLock::new();
        self.max_leading_prefix_len_cache = OnceLock::new();
        if self.requires_surface_fetch_index_seed() {
            let fetches_canonical_storage =
                matches!(self.storage, TableStorage::Compact(_)) && self.prism_payload.is_some();
            self.leading_fetch_index_seed =
                Some(self.capture_leading_fetch_index_seed(fetches_canonical_storage));
        }
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
    pub fn with_upstream_sentence_poet_source(
        mut self,
        source: Arc<dyn PoetByteSource>,
        dictionary_checksum: u32,
    ) -> Self {
        self.upstream_sentence_poet_source = Some((source, dictionary_checksum));
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
        let model = if let Some((source, dictionary_checksum)) =
            self.upstream_sentence_poet_source.take()
        {
            UpstreamSentenceModel::from_poet_bin_source(source, dictionary_checksum, max_candidates)
                .expect("validated poet artifact should load into sentence model")
        } else if let Some(entries) = self.source_entries.take() {
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

    fn capture_leading_fetch_index_seed(
        &self,
        fetches_canonical_storage: bool,
    ) -> LeadingFetchIndexSeed {
        // The leading-single path needs only canonical codes that own a
        // one-character candidate. Broad prefix fallback additionally admits
        // phrase rows; heap/source storage has no prism to resolve those
        // transformed surfaces lazily, so retain every canonical group there.
        // A current compact prism remains the bounded authoritative index for
        // phrase prefixes and therefore keeps this seed single-character-only.
        let include_all_candidates =
            self.prefix_fallback && !self.direct_prism_surface_mapping_current;
        let (canonical_codes, leading_single_codes) =
            if let Some(syllabary_codes) = self.storage.syllabary_codes() {
                let leading_single_codes = syllabary_codes
                    .iter()
                    .filter(|code| {
                        self.storage
                            .exact_candidates(code)
                            .any(|candidate| candidate.text().chars().count() == 1)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let canonical_codes = if include_all_candidates {
                    syllabary_codes.to_vec()
                } else {
                    leading_single_codes.clone()
                };
                (canonical_codes, leading_single_codes)
            } else {
                // Heap/source algebra rewrites storage keys. Capture the canonical
                // code from the immutable raw comment before expansion so the
                // resulting surface edge can still filter correction collisions and
                // preserve tone metadata. Track single-character ownership
                // separately so phrase groups can never widen the leading-syllable
                // boundary cap.
                let mut codes = Vec::new();
                let mut has_single_candidate = Vec::new();
                let mut index_by_code = HashMap::<String, usize>::new();
                let mut record = |storage_code: &str, raw_comment: &str, is_single: bool| {
                    let canonical = canonical_fetch_group(raw_comment);
                    let canonical = if canonical.is_empty() {
                        storage_code
                    } else {
                        canonical.as_ref()
                    };
                    if canonical.is_empty() {
                        return;
                    }
                    if let Some(index) = index_by_code.get(canonical).copied() {
                        has_single_candidate[index] |= is_single;
                    } else {
                        let canonical = canonical.to_owned();
                        index_by_code.insert(canonical.clone(), codes.len());
                        codes.push(canonical);
                        has_single_candidate.push(is_single);
                    }
                };
                if let Some(source_entries) = &self.source_entries {
                    for (storage_code, candidate) in source_entries {
                        record(
                            storage_code,
                            &candidate.comment,
                            candidate.text.chars().count() == 1,
                        );
                    }
                } else {
                    for storage_code in self.storage.all_codes() {
                        for candidate in self.storage.exact_candidates(storage_code.as_ref()) {
                            record(
                                storage_code.as_ref(),
                                candidate.raw_comment(),
                                candidate.text().chars().count() == 1,
                            );
                        }
                    }
                }
                let leading_single_codes = codes
                    .iter()
                    .zip(&has_single_candidate)
                    .filter(|(_, has_single)| **has_single)
                    .map(|(code, _)| code.clone())
                    .collect::<Vec<_>>();
                if include_all_candidates {
                    (codes, leading_single_codes)
                } else {
                    (leading_single_codes.clone(), leading_single_codes)
                }
            };
        let algebra = SpellingAlgebra::parse(&self.spelling_algebra_formulas);
        let max_leading_single_surface_len = if algebra.is_empty() {
            leading_single_codes
                .iter()
                .map(|code| code.len().max(normalized_original_code(code).len()))
                .max()
                .unwrap_or(0)
        } else {
            leading_single_codes
                .iter()
                .flat_map(|code| algebra.expand_deployed_spelling_variants(code))
                .map(|variant| variant.code.len())
                .max()
                .unwrap_or(0)
        };
        LeadingFetchIndexSeed {
            canonical_codes,
            fetches_canonical_storage,
            max_leading_single_surface_len,
        }
    }

    fn requires_surface_fetch_index_seed(&self) -> bool {
        self.spelling_algebra_active
            && (self.leading_syllable_reachability
                || (self.prefix_fallback && !self.direct_prism_surface_mapping_current))
    }

    fn deployed_algebra_leading_fetch_index(
        &self,
        seed: &LeadingFetchIndexSeed,
    ) -> HashMap<String, Vec<LeadingFetchCode>> {
        let algebra = SpellingAlgebra::parse(&self.spelling_algebra_formulas);
        let mut index: HashMap<String, Vec<LeadingFetchCode>> = HashMap::new();
        for canonical_code in &seed.canonical_codes {
            if algebra.is_empty() {
                let edge = LeadingFetchCode {
                    fetch_code: canonical_code.clone(),
                    canonical_code: canonical_code.clone(),
                    bare_exact: true,
                    injectable: true,
                    abbreviation: false,
                    direct_identity: false,
                };
                let exact_edges = index.entry(canonical_code.clone()).or_default();
                if !exact_edges.contains(&edge) {
                    exact_edges.push(edge.clone());
                }
                let normalized = normalized_original_code(canonical_code);
                if normalized.is_empty() {
                    continue;
                }
                let normalized_edge = LeadingFetchCode {
                    bare_exact: normalized == *canonical_code,
                    ..edge
                };
                let normalized_edges = index.entry(normalized).or_default();
                if !normalized_edges.contains(&normalized_edge) {
                    normalized_edges.push(normalized_edge);
                }
                continue;
            }
            for variant in algebra.expand_deployed_spelling_variants(canonical_code) {
                if variant.code.is_empty() {
                    continue;
                }
                let surface = variant.code;
                let edge = LeadingFetchCode {
                    fetch_code: if seed.fetches_canonical_storage {
                        canonical_code.clone()
                    } else {
                        surface.clone()
                    },
                    canonical_code: canonical_code.clone(),
                    bare_exact: true,
                    // A correction surface guards a complete input from a
                    // shorter-family injection, but remains a separate opt-in
                    // mechanism and cannot itself become a default reachability
                    // candidate.
                    injectable: !variant.properties.is_correction,
                    abbreviation: variant.properties.spelling_type
                        == DeployedSpellingType::Abbreviation,
                    direct_identity: false,
                };
                let edges = index.entry(surface.clone()).or_default();
                if !edges.contains(&edge) {
                    edges.push(edge);
                }
            }
        }
        index
    }

    #[must_use]
    pub fn with_spelling_algebra(mut self, formulas: &[String]) -> Self {
        let algebra = SpellingAlgebra::parse(formulas);
        self.prefix_fallback_window_cache = Mutex::new(None);
        if self.spelling_algebra_configured {
            self.direct_prism_surface_mapping_current &=
                self.spelling_algebra_formulas.as_slice() == formulas;
        }
        self.spelling_algebra_configured = true;
        self.spelling_algebra_active = !algebra.is_empty();
        self.spelling_algebra_formulas = formulas.to_vec();
        self.spelling_abbreviation_entries.clear();
        self.spelling_correction_entries.clear();
        self.spelling_correction_surfaces.clear();
        self.single_letter_sentence_guard_enabled = false;
        // Algebra replacement invalidates every surface-derived cache even when
        // the new formula list is empty or fails to parse. Keep the seed absent
        // while reachability is explicitly disabled; enabling it later rebuilds
        // from the current compact or expanded storage in source order.
        self.leading_fetch_index_seed = None;
        self.leading_fetch_index_cache = OnceLock::new();
        self.max_leading_prefix_len_cache = OnceLock::new();
        self.untoned_dictionary_cache = OnceLock::new();
        let keeps_canonical_storage =
            matches!(self.storage, TableStorage::Compact(_)) && self.prism_payload.is_some();
        if self.requires_surface_fetch_index_seed() {
            self.leading_fetch_index_seed =
                Some(self.capture_leading_fetch_index_seed(keeps_canonical_storage));
        }
        if !algebra.is_empty() {
            if keeps_canonical_storage {
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
            self.spelling_correction_entries = spelling_correction_entries(&entries);
            self.spelling_correction_surfaces = self
                .spelling_correction_entries
                .iter()
                .map(|(surface, _, _)| surface.clone())
                .collect();
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
        let mut exact = LookupCodeSpec::exact(lookup_code);
        exact.spelling_correction = self.exact_surface_is_correction_only(lookup_code);
        let mut specs = vec![exact];
        let default_off_dynamic_anchor_exists = self.dynamic_correction_lookup
            && !self.enable_correction
            && lookup_code.starts_with('m')
            && self.storage.has_code(lookup_code);
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
                // Short TypeDuck rows can expose thousands of single-letter
                // abbreviation descriptors.  Prune that family, not ordinary
                // deployed spellings whose canonical storage code differs from
                // the surface (for example `nei` -> `nei1`/`nei5`).  Treating
                // every non-identical code as an abbreviation dropped the full
                // exact family and let sentence/prefix fallback own the input.
                if track_b_short_prefix && (lookup.abbreviation || lookup.correction) {
                    continue;
                }
                if !specs.iter().any(|spec| spec.code == lookup.code)
                    && self.storage.has_code(lookup.code)
                {
                    specs.push(LookupCodeSpec::alias(
                        lookup.code.to_owned(),
                        lookup_code.to_owned(),
                        lookup.correction,
                        lookup.credibility,
                    ));
                }
            }
        }
        let allow_dynamic_near_lookup = self.dynamic_correction_lookup
            && (self.enable_correction || default_off_dynamic_anchor_exists);
        let dynamic_syllable_count = default_off_dynamic_anchor_exists
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
                        && default_off_dynamic_anchor_exists
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
                        specs.push(LookupCodeSpec::tolerance_exact(candidate_code));
                    }
                }
            }
        }
        specs
    }

    fn sentence_lookup_specs(&self, lookup_code: &str) -> Vec<LookupCodeSpec> {
        let mut exact = LookupCodeSpec::exact(lookup_code);
        exact.spelling_correction = self.exact_surface_is_correction_only(lookup_code);
        let mut specs = vec![exact];
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
            if specs.iter().any(|spec| spec.code == lookup.code)
                || !self.storage.has_code(lookup.code)
            {
                continue;
            }
            specs.push(LookupCodeSpec::alias(
                lookup.code.to_owned(),
                lookup_code.to_owned(),
                lookup.correction,
                lookup.credibility,
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

    /// True when the full input already resolves to a single-char exact — a
    /// complete syllable the exact path serves directly (`hao`→好, `mai`→買/賣).
    /// Such inputs need no leading-single composition (there is no multi-syllable
    /// remainder), so the injection machinery would walk, allocate, and produce
    /// nothing. This is the bare-syllable guard formerly buried inside the walk,
    /// lifted to the call sites so bare syllables stop paying for it every
    /// keystroke. It must NOT fire when the exact is a multi-char word (`zhonggao`,
    /// `zhongguo`→中國) — those keep the injection so their leading single stays
    /// reachable.
    fn input_serves_single_char_exact(&self, lookup_code: &str) -> bool {
        self.leading_surface_fetch_codes(lookup_code)
            .into_iter()
            .filter(|fetch| fetch.bare_exact)
            .any(|fetch| {
                self.storage
                    .exact_candidates(&fetch.fetch_code)
                    .any(|candidate| {
                        candidate.text().chars().count() == 1
                            && leading_candidate_matches_fetch(&candidate, &fetch)
                    })
            })
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

    fn is_spelling_correction_view(&self, code: &str, candidate: &LookupCandidate<'_>) -> bool {
        if self.spelling_correction_entries.is_empty() {
            return false;
        }
        self.spelling_correction_entries.contains(&(
            code.to_owned(),
            candidate.text().to_owned(),
            candidate.raw_comment().to_owned(),
        ))
    }

    fn exact_surface_is_correction_only(&self, code: &str) -> bool {
        if !self.spelling_correction_surfaces.contains(code) {
            return false;
        }
        let mut saw_correction = false;
        let mut saw_normal = false;
        for candidate in self.storage.exact_candidates(code) {
            if self.is_spelling_correction_view(code, &candidate) {
                saw_correction = true;
            } else {
                saw_normal = true;
            }
        }
        saw_correction && !saw_normal
    }

    fn lookup_candidate_order(
        &self,
        left: &PendingLookupCandidate,
        right: &PendingLookupCandidate,
    ) -> Ordering {
        let category_order = self
            .lookup_candidate_category(left)
            .cmp(&self.lookup_candidate_category(right));
        if category_order != Ordering::Equal || !self.sort_by_weight {
            return category_order;
        }
        self.lookup_candidate_weight_order(left, right)
    }

    fn lookup_candidate_weight_order(
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

    fn order_lookup_candidates(&self, candidates: &mut Vec<PendingLookupCandidate>) {
        if self.sort_by_weight {
            candidates.sort_by(|left, right| self.lookup_candidate_order(left, right));
            return;
        }

        // librime's table `sort: original` contract is per canonical code queue,
        // not a freeze of the flattened prism-alias stream. A heap merges the
        // current group heads by raw weight while keeping every group's source
        // order and source-earlier stability on equal heads. TypeDuck's selected
        // prediction stays behind the first ordinary row and uses its existing
        // entry-weight/span comparison. Partial-span candidates are produced by
        // the later prefix-fallback path, so canonical group changes never stand
        // in for consumed-span changes.
        order_original_grouped_candidates(
            candidates,
            None,
            |candidate| candidate.limited_prediction,
            |candidate| self.lookup_candidate_category(candidate),
            |candidate| candidate.fetch_group,
            PendingLookupCandidate::raw_quality,
            |left, right| self.lookup_candidate_weight_order(left, right),
            PendingLookupCandidate::prediction_precedes,
        );
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
        spelling_credibility: f32,
    ) -> Candidate {
        self.format_candidate_for_lookup(
            entry_code,
            candidate.clone(),
            lookup_code,
            correction_distance,
            spelling_credibility,
        )
    }

    fn candidate_for_lookup_view(
        &self,
        entry_code: &str,
        candidate: &LookupCandidate<'_>,
        lookup_code: &str,
        correction_distance: Option<usize>,
        spelling_credibility: f32,
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
        raw_quality += spelling_credibility;
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
        spelling_credibility: f32,
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
        let mut raw_quality = candidate.quality + spelling_credibility;
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
        // M59 finding #9 (flip precondition): `leading_syllable_reachability`
        // carries its own bounded-arm injection (:2188), so a flip schema that
        // combines it with `prediction_never_first` (and neither a prediction
        // limit nor prefix_fallback) must still take the bounded path. Without
        // this disjunct that combo falls to the compact `Some(limit)` fallback,
        // whose injection gate (`prefix_fallback_limit.is_none()`) is false, and
        // the leading single is silently dropped (source storage also
        // full-materialises per keypress). Inert today: luna already satisfies
        // `!prediction_never_first`, and jyutping leaves the flag off.
        (!self.prediction_never_first
            || self.prediction_candidate_limit.is_some()
            || self.prefix_fallback
            || self.leading_syllable_reachability)
            && !self.sentence_over_completion
            && lookup_specs
                .iter()
                .all(|spec| spec.required_syllable_count.is_none())
    }

    /// M59 finding #6: structural classification of the backing dictionary as
    /// untoned (luna `mo`) vs toned (jyutping `bei2`), cached after first use.
    /// The untoned-relaxation in the leading-single filter keys on this rather
    /// than the reachability flag, so the default-ON schema-general flip cannot
    /// admit digit-less rows into a toned family.
    pub(crate) fn untoned_dictionary(&self) -> bool {
        *self
            .untoned_dictionary_cache
            .get_or_init(|| dictionary_is_untoned(&self.storage))
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
        let mut raw_quality = candidate.candidate.raw_quality() + candidate.spelling_credibility;
        if let Some(distance) = candidate.correction_distance {
            raw_quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        raw_quality
    }

    fn lookup_candidate_ref_prediction_weight(
        &self,
        candidate: &PendingLookupCandidateRef<'_>,
    ) -> f32 {
        let mut weight = candidate
            .candidate
            .raw_quality()
            .max(f32::MIN_POSITIVE)
            .ln()
            + candidate.spelling_credibility;
        if let Some(distance) = candidate.correction_distance {
            weight += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        weight
    }

    fn lookup_prediction_ref_precedes(
        &self,
        prediction: &PendingLookupCandidateRef<'_>,
        ordinary: &PendingLookupCandidateRef<'_>,
    ) -> bool {
        let interpreted = complete_syllable_prefix_count(
            prediction.candidate.raw_comment(),
            prediction.lookup_code,
        );
        let consumed = source_code_syllable_count(ordinary.candidate.raw_comment());
        self.lookup_candidate_ref_prediction_weight(prediction)
            > self.lookup_candidate_ref_prediction_weight(ordinary)
            || interpreted
                .zip(consumed)
                .is_some_and(|(interpreted, consumed)| consumed < interpreted)
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
        let category_order = self
            .lookup_candidate_ref_category(left)
            .cmp(&self.lookup_candidate_ref_category(right));
        if category_order != Ordering::Equal {
            return category_order;
        }
        if !self.sort_by_weight {
            return left.emission_order.cmp(&right.emission_order);
        }
        self.lookup_candidate_ref_weight_order(left, right)
    }

    fn lookup_candidate_ref_weight_order(
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

    fn order_lookup_candidate_refs(
        &self,
        candidates: &mut Vec<PendingLookupCandidateRef<'_>>,
        limit: usize,
    ) {
        if self.sort_by_weight {
            candidates.sort_by(|left, right| self.lookup_candidate_ref_order(left, right));
            candidates.truncate(limit);
            return;
        }

        // Bounded collection retains only the first requested window per group.
        // Stop this shared constrained merge at the requested global prefix: the
        // old implementation rescanned every group for every retained row, built
        // the complete merged pool, and only then truncated it.
        order_original_grouped_candidates(
            candidates,
            Some(limit),
            |candidate| candidate.limited_prediction,
            |candidate| self.lookup_candidate_ref_category(candidate),
            |candidate| candidate.fetch_group,
            |candidate| self.lookup_candidate_ref_raw_quality(candidate),
            |left, right| self.lookup_candidate_ref_weight_order(left, right),
            |prediction, ordinary| self.lookup_prediction_ref_precedes(prediction, ordinary),
        );
    }

    fn materialized_quality(
        &self,
        entry_code: &str,
        lookup_code: &str,
        candidate: &LookupCandidate<'_>,
        correction_distance: Option<usize>,
        spelling_credibility: f32,
    ) -> f32 {
        let mut raw_quality = candidate.raw_quality() + spelling_credibility;
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
            right.spelling_credibility,
        )
        .partial_cmp(&self.materialized_quality(
            left.entry_code.as_ref(),
            left.lookup_code,
            &left.candidate,
            left.correction_distance,
            left.spelling_credibility,
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

    fn push_bounded_pending_by_weight_order<'a>(
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
            .max_by(|(_, left), (_, right)| self.lookup_candidate_ref_weight_order(left, right))
        else {
            return;
        };
        if self.lookup_candidate_ref_weight_order(&candidate, worst) == Ordering::Less {
            selected[worst_index] = candidate;
        }
    }

    fn push_bounded_pending_by_original_group<'a>(
        &self,
        selected: &mut Vec<PendingLookupCandidateRef<'a>>,
        retained_by_group: &mut HashMap<(u8, usize), usize>,
        candidate: PendingLookupCandidateRef<'a>,
        limit: usize,
    ) {
        let category = self.lookup_candidate_ref_category(&candidate);
        let retained = retained_by_group
            .entry((category, candidate.fetch_group))
            .or_default();
        if *retained < limit {
            *retained += 1;
            selected.push(candidate);
        }
    }

    fn bounded_candidates_for_lookup_codes(
        &self,
        request: BoundedLookupRequest<'_>,
        scratch: Option<&mut TranslatorScratch>,
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
        // NOTE: `leading_syllable_reachability` deliberately does NOT widen
        // `ordered_mode` here — that would disable luna's bounded early-stop
        // (`can_stop_after_window`) and cost typing latency. Ordering for the
        // leading-single injection is preserved locally: the bounded
        // sentence-splice path re-assigns ordered qualities after its splice, and
        // the complete/page-turn path gates on `leading_syllable_reachability`
        // for its own quality assignment.
        let ordered_mode = self.prediction_candidate_limit.is_some() || self.prefix_fallback;
        let record_short_key = self.uses_m44_short_key_metrics(lookup_code);
        let record_track_b = self.uses_m44_track_b_metrics();
        let short_key_filter_start =
            (record_short_key && crate::m37_metrics_enabled()).then(Instant::now);
        let mut short_key_rows_scanned = 0usize;
        let mut selected = Vec::new();
        let mut limited_predictions = Vec::new();
        let mut fetch_groups = HashMap::new();
        let mut retained_original_groups = HashMap::new();
        let mut emission_order = 0;
        let mut full_count = 0;
        let mut prefix_fallback_owned = false;
        let mut has_correction_lookup = lookup_specs
            .iter()
            .any(|spec| spec.correction_distance.is_some() || spec.spelling_correction);
        let can_stop_after_window = !include_full_count && !ordered_mode && self.sort_by_weight;
        let mut early_stopped = false;
        for lookup_spec in lookup_specs {
            let fetch_code = lookup_spec.code.as_str();
            let spec_lookup_code = lookup_spec.lookup_code.as_str();
            let lookup_has_exact_candidates =
                self.prediction_candidate_limit.is_some() && self.storage.has_code(fetch_code);
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
                let spelling_abbreviation =
                    self.is_spelling_abbreviation_view(spec_lookup_code, &candidate);
                let spelling_correction = lookup_spec.spelling_correction
                    || self.is_spelling_correction_view(spec_lookup_code, &candidate);
                let fetch_group = if self.sort_by_weight {
                    0
                } else {
                    intern_fetch_group(&mut fetch_groups, candidate.raw_comment())
                };
                let pending = PendingLookupCandidateRef {
                    fetch_group,
                    entry_code: Cow::Borrowed(spec_lookup_code),
                    lookup_code: spec_lookup_code,
                    candidate,
                    correction_distance: lookup_spec.correction_distance,
                    spelling_abbreviation,
                    limited_prediction: false,
                    emission_order,
                    spelling_correction,
                    spelling_credibility: lookup_spec.spelling_credibility,
                };
                if !self.sort_by_weight {
                    self.push_bounded_pending_by_original_group(
                        &mut selected,
                        &mut retained_original_groups,
                        pending,
                        limit,
                    );
                } else if ordered_mode {
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
                        let spelling_correction = lookup_spec.spelling_correction
                            || self.is_spelling_correction_view(spec_lookup_code, &candidate);
                        let fetch_group = if self.sort_by_weight {
                            0
                        } else {
                            intern_fetch_group(&mut fetch_groups, candidate.raw_comment())
                        };
                        let pending = PendingLookupCandidateRef {
                            fetch_group,
                            entry_code,
                            lookup_code: spec_lookup_code,
                            candidate,
                            correction_distance: lookup_spec.correction_distance,
                            spelling_abbreviation,
                            limited_prediction,
                            emission_order,
                            spelling_correction,
                            spelling_credibility: lookup_spec.spelling_credibility,
                        };
                        if limited_prediction && ordered_mode {
                            self.push_bounded_pending_by_weight_order(
                                &mut limited_predictions,
                                pending,
                                self.prediction_candidate_limit.unwrap_or(limit),
                            );
                        } else if !self.sort_by_weight {
                            self.push_bounded_pending_by_original_group(
                                &mut selected,
                                &mut retained_original_groups,
                                pending,
                                limit,
                            );
                            full_count += 1;
                        } else if ordered_mode {
                            self.push_bounded_pending_by_lookup_order(
                                &mut selected,
                                pending,
                                limit,
                            );
                            full_count += 1;
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
        // On the new `sort: original` path, a held prediction cannot keep an
        // otherwise empty translation alive (`ScriptTranslation::cand_count_`
        // is still zero). Preserve the legacy by-weight path's complete/bounded
        // parity; 3a does not redefine that established ordering contract.
        if self.sort_by_weight || !selected.is_empty() {
            full_count += limited_predictions.len();
            selected.extend(limited_predictions);
        }
        has_correction_lookup |= selected.iter().any(|pending| pending.spelling_correction);
        if selected.is_empty() && self.enable_sentence {
            if let Some(model) = &self.upstream_sentence_model {
                let model_start = crate::m37_metrics_enabled().then(Instant::now);
                let sentence_limit = limit.min(BOUNDED_SENTENCE_MODEL_PAGE_LIMIT);
                let mut candidates = if let Some(scratch) = scratch {
                    model.candidates_for_input_with_limit_and_scratch(
                        input,
                        sentence_limit,
                        &mut scratch.upstream_sentence,
                    )
                } else {
                    model.candidates_for_input_with_limit(input, sentence_limit)
                }
                .into_iter()
                .filter(|candidate| !filter_by_charset || !contains_extended_cjk(&candidate.text))
                .collect::<Vec<_>>();
                if let Some(start) = model_start {
                    crate::m37_record_upstream_sentence_model(start.elapsed(), candidates.len());
                }
                if !candidates.is_empty() {
                    let base_window_may_have_more = candidates.len() >= sentence_limit;
                    let mut prefix_fallback_truncated = false;
                    let mut prefix_fallback_owned = false;
                    if self.prefix_fallback && !has_correction_lookup {
                        let room = limit.saturating_sub(candidates.len());
                        if room > 0 {
                            let prefix_batch = self.prefix_fallback_candidates(
                                input,
                                lookup_code,
                                filter_by_charset,
                                &candidates,
                                &candidates,
                                Some(room),
                            );
                            prefix_fallback_owned = prefix_batch.owns_reachability;
                            prefix_fallback_truncated = prefix_batch.truncated;
                            let inserted = merge_prefix_fallback_candidates(
                                &mut candidates,
                                prefix_batch.candidates,
                                lookup_code,
                            );
                            if prefix_fallback_truncated && inserted == 0 {
                                prefix_fallback_truncated = matches!(
                                    self.prefix_fallback_has_unique_candidate(
                                        input,
                                        lookup_code,
                                        filter_by_charset,
                                        &candidates,
                                        Some(limit),
                                    ),
                                    PrefixFallbackProbe::Found | PrefixFallbackProbe::Truncated
                                );
                            }
                        } else {
                            prefix_fallback_owned = !matches!(
                                self.prefix_fallback_has_unique_candidate(
                                    input,
                                    lookup_code,
                                    filter_by_charset,
                                    &candidates,
                                    Some(limit),
                                ),
                                PrefixFallbackProbe::NoPrefix
                            );
                        }
                    }
                    if self.leading_syllable_reachability
                        && !has_correction_lookup
                        && !prefix_fallback_owned
                        && has_proper_leading_prefix(lookup_code)
                        && !self.input_serves_single_char_exact(lookup_code)
                    {
                        // Keep phrase candidates in place; append only a bounded
                        // slice of the leading-syllable family after them — enough
                        // to fill the page and signal the rest is reachable by
                        // paging. The fetch is capped (no full materialization on
                        // the typing path); the full family comes on the page-turn.
                        let insert_at = leading_single_insert_index(&candidates);
                        let want = limit.saturating_sub(insert_at).saturating_add(1);
                        let leading_singles = self.leading_single_syllable_prefix_candidates(
                            input,
                            lookup_code,
                            filter_by_charset,
                            &candidates[..insert_at],
                            Some(want),
                        );
                        // M59: only re-rank when singles were actually injected. On a
                        // bare-syllable input (`hao`, `mai`) the bare-syllable guard
                        // returns an empty family, so pre-flip this branch never ran
                        // and the phrases returned in their natural order. Post-flip
                        // the branch runs, and an unconditional
                        // `assign_ordered_candidate_qualities` was re-ranking the whole
                        // page every keystroke for nothing — the dominant `hao` flip
                        // cost (short_key_first_page_materialize +181k ns). Guarding on
                        // non-empty restores the exact pre-flip behaviour for the empty
                        // case while leaving the real-injection path untouched.
                        if !leading_singles.is_empty() {
                            candidates.splice(insert_at..insert_at, leading_singles);
                            // Preserve phrase-before-single order through the engine's
                            // global quality sort (the bounded sentence return does not
                            // otherwise assign ordered qualities).
                            self.assign_ordered_candidate_qualities(&mut candidates);
                        }
                    }
                    let merged_window_overflow = candidates.len() > limit;
                    candidates.truncate(limit);
                    let result_full_count = if prefix_fallback_truncated
                        || base_window_may_have_more
                        || merged_window_overflow
                    {
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
                    let base_window_may_have_more = candidates.len() >= abbreviation_limit;
                    let mut prefix_fallback_truncated = false;
                    let mut prefix_fallback_owned = false;
                    if self.prefix_fallback && !has_correction_lookup {
                        let room = limit.saturating_sub(candidates.len());
                        if room > 0 {
                            let prefix_batch = self.prefix_fallback_candidates(
                                input,
                                lookup_code,
                                filter_by_charset,
                                &candidates,
                                &candidates,
                                Some(room),
                            );
                            prefix_fallback_owned = prefix_batch.owns_reachability;
                            prefix_fallback_truncated = prefix_batch.truncated;
                            let inserted = merge_prefix_fallback_candidates(
                                &mut candidates,
                                prefix_batch.candidates,
                                lookup_code,
                            );
                            if prefix_fallback_truncated && inserted == 0 {
                                prefix_fallback_truncated = matches!(
                                    self.prefix_fallback_has_unique_candidate(
                                        input,
                                        lookup_code,
                                        filter_by_charset,
                                        &candidates,
                                        Some(limit),
                                    ),
                                    PrefixFallbackProbe::Found | PrefixFallbackProbe::Truncated
                                );
                            }
                        } else {
                            prefix_fallback_owned = !matches!(
                                self.prefix_fallback_has_unique_candidate(
                                    input,
                                    lookup_code,
                                    filter_by_charset,
                                    &candidates,
                                    Some(limit),
                                ),
                                PrefixFallbackProbe::NoPrefix
                            );
                        }
                    }
                    if self.leading_syllable_reachability
                        && !has_correction_lookup
                        && !prefix_fallback_owned
                        && has_proper_leading_prefix(lookup_code)
                        && !self.input_serves_single_char_exact(lookup_code)
                    {
                        // Keep phrase candidates in place; append only a bounded
                        // slice of the leading-syllable family after them — enough
                        // to fill the page and signal the rest is reachable by
                        // paging. The fetch is capped (no full materialization on
                        // the typing path); the full family comes on the page-turn.
                        let insert_at = leading_single_insert_index(&candidates);
                        let want = limit.saturating_sub(insert_at).saturating_add(1);
                        let leading_singles = self.leading_single_syllable_prefix_candidates(
                            input,
                            lookup_code,
                            filter_by_charset,
                            &candidates[..insert_at],
                            Some(want),
                        );
                        // M59: only re-rank when singles were actually injected. On a
                        // bare-syllable input (`hao`, `mai`) the bare-syllable guard
                        // returns an empty family, so pre-flip this branch never ran
                        // and the phrases returned in their natural order. Post-flip
                        // the branch runs, and an unconditional
                        // `assign_ordered_candidate_qualities` was re-ranking the whole
                        // page every keystroke for nothing — the dominant `hao` flip
                        // cost (short_key_first_page_materialize +181k ns). Guarding on
                        // non-empty restores the exact pre-flip behaviour for the empty
                        // case while leaving the real-injection path untouched.
                        if !leading_singles.is_empty() {
                            candidates.splice(insert_at..insert_at, leading_singles);
                            // Preserve phrase-before-single order through the engine's
                            // global quality sort (the bounded sentence return does not
                            // otherwise assign ordered qualities).
                            self.assign_ordered_candidate_qualities(&mut candidates);
                        }
                    }
                    let merged_window_overflow = candidates.len() > limit;
                    candidates.truncate(limit);
                    let result_full_count = if prefix_fallback_truncated
                        || base_window_may_have_more
                        || merged_window_overflow
                    {
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
                if !self.bounds_compact_fallback_expansion() {
                    return TranslationResult::complete(self.translated_candidates_for_segment(
                        input,
                        filter_by_charset,
                        segment_tags,
                    ));
                }
                let batch = self.translated_candidates_for_segment_with_prefix_fallback_limit(
                    input,
                    filter_by_charset,
                    segment_tags,
                    Some(limit),
                );
                let full_count = if batch.truncated || batch.candidates.len() > limit {
                    batch.candidates.len().saturating_add(1)
                } else {
                    batch.candidates.len()
                };
                return TranslationResult::bounded(
                    batch.candidates,
                    full_count,
                    include_full_count,
                );
            }
            if let Some(sentence) = self.sentence_candidate(input, filter_by_charset, None) {
                let candidates = vec![sentence];
                crate::m37_record_bounded_iterator(limit, candidates.len(), candidates.len());
                return TranslationResult::bounded(candidates, 1, include_full_count);
            }
            crate::m37_record_full_list_fallback();
            if !self.bounds_compact_fallback_expansion() {
                return TranslationResult::complete(self.translated_candidates_for_segment(
                    input,
                    filter_by_charset,
                    segment_tags,
                ));
            }
            let batch = self.translated_candidates_for_segment_with_prefix_fallback_limit(
                input,
                filter_by_charset,
                segment_tags,
                Some(limit),
            );
            let full_count = if batch.truncated || batch.candidates.len() > limit {
                batch.candidates.len().saturating_add(1)
            } else {
                batch.candidates.len()
            };
            return TranslationResult::bounded(batch.candidates, full_count, include_full_count);
        }
        if selected.is_empty() && self.prefix_fallback && !has_correction_lookup {
            let mut batch = self.prefix_fallback_candidates(
                input,
                lookup_code,
                filter_by_charset,
                &[],
                &[],
                Some(limit),
            );
            prefix_fallback_owned = batch.owns_reachability;
            let full_count = batch.candidates.len();
            if !batch.candidates.is_empty() {
                batch.candidates.truncate(limit);
                let result_full_count = if batch.truncated || full_count >= limit {
                    batch.candidates.len().saturating_add(1)
                } else if full_count > batch.candidates.len() {
                    full_count
                } else {
                    batch.candidates.len()
                };
                crate::m37_record_bounded_iterator(
                    limit,
                    batch.candidates.len(),
                    result_full_count,
                );
                return TranslationResult::bounded(
                    batch.candidates,
                    result_full_count,
                    include_full_count,
                );
            }
        }
        if !self.sort_by_weight || ordered_mode {
            let sort_start = (record_short_key && crate::m37_metrics_enabled()).then(Instant::now);
            self.order_lookup_candidate_refs(&mut selected, limit);
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
                    candidate.spelling_credibility,
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
        let has_multi_syllable_full_exact_candidate =
            (self.prefix_fallback && !has_correction_lookup).then(|| {
                candidates.iter().any(|candidate| {
                    candidate.source == CandidateSource::Table
                        && source_code_syllable_count(&candidate.comment)
                            .is_some_and(|count| count > 1)
                })
            });
        if self.combine_candidates {
            candidates = combine_duplicate_text_candidates(candidates);
        }
        if self.prefix_fallback && !has_correction_lookup {
            let has_multi_syllable_full_exact_candidate =
                has_multi_syllable_full_exact_candidate.unwrap_or(false);
            let full_page_without_exact = candidates.len() >= limit
                && candidates
                    .iter()
                    .all(|candidate| candidate.source != CandidateSource::Table);
            if full_page_without_exact {
                // With no exact row, the shared fallback insertion point is the
                // end of this already-full page and there is no span metadata
                // to promote. Preserve the bounded existence probe here; a
                // materialized merge cannot change any visible candidate.
                let probe = self.prefix_fallback_has_unique_candidate(
                    input,
                    lookup_code,
                    filter_by_charset,
                    &candidates,
                    Some(limit),
                );
                prefix_fallback_owned = !matches!(probe, PrefixFallbackProbe::NoPrefix);
                if matches!(
                    probe,
                    PrefixFallbackProbe::Found | PrefixFallbackProbe::Truncated
                ) {
                    full_count = full_count.max(candidates.len().saturating_add(1));
                }
            } else {
                let prefix_batch = self.prefix_fallback_candidates(
                    input,
                    lookup_code,
                    filter_by_charset,
                    &[],
                    &candidates,
                    Some(limit),
                );
                prefix_fallback_owned = prefix_batch.owns_reachability;
                let mut prefix_fallback_truncated = prefix_batch.truncated;
                let inserted = merge_prefix_fallback_candidates(
                    &mut candidates,
                    prefix_batch.candidates,
                    lookup_code,
                );
                if prefix_fallback_truncated && inserted == 0 {
                    // A bounded materialization can stop on duplicate rows even
                    // though no distinct fallback candidate exists. Reuse the
                    // cheap existence probe to distinguish an exhaustively
                    // duplicate-only family from a budget-truncated one.
                    prefix_fallback_truncated = matches!(
                        self.prefix_fallback_has_unique_candidate(
                            input,
                            lookup_code,
                            filter_by_charset,
                            &candidates,
                            Some(limit),
                        ),
                        PrefixFallbackProbe::Found | PrefixFallbackProbe::Truncated
                    );
                }
                full_count += inserted;
                let merged_window_overflow = candidates.len() > limit;
                candidates.truncate(limit);
                if prefix_fallback_truncated
                    || merged_window_overflow
                    || (prefix_fallback_owned && has_multi_syllable_full_exact_candidate)
                {
                    full_count = full_count.max(candidates.len().saturating_add(1));
                }
            }
        }
        if ordered_mode {
            self.assign_ordered_candidate_qualities(&mut candidates);
        }
        // M59 finding #1 (corrective): leading-syllable reachability must make
        // the leading-single family reachable on THIS (non-empty-`selected`)
        // bounded arm. Completion/exact-hit inputs (e.g. `zhongguo`, `zhonggao`)
        // skip the empty-`selected` sentence arm, so signalling alone leaves the
        // boundary single (中 at the first tail slot) unreachable: the bounded
        // page renders short (only the phrase completions), then when the
        // page-turn materialises the full list the boundary single lands at the
        // page-0 tail slot the highlight has already advanced past — silently
        // skipped (the pre-M59 dead-end). Inject a bounded slice of the family
        // AFTER the phrases so the bounded page is a prefix of the unbounded
        // list (page-0 tail = the leading single, page-turn is contiguous), and
        // under-advertise completeness so the remaining family pages in. The
        // fetch is capped (no full materialisation on the typing path); the full
        // family is materialised only on the unbounded/page-turn path (:2804).
        //
        // Prefix fallback is authoritative only when this input has an actual
        // deployed proper prefix. If it does not, leave the independent
        // leading-syllable mechanism available instead of creating a
        // schema-level reachability hole.
        if self.leading_syllable_reachability
            && !has_correction_lookup
            && !prefix_fallback_owned
            && has_proper_leading_prefix(lookup_code)
            && !self.input_serves_single_char_exact(lookup_code)
        {
            let insert_at = leading_single_insert_index(&candidates);
            let want = limit.saturating_sub(insert_at).saturating_add(1);
            let leading_singles = self.leading_single_syllable_prefix_candidates(
                input,
                lookup_code,
                filter_by_charset,
                &candidates,
                Some(want),
            );
            if !leading_singles.is_empty() {
                let overflows_page = candidates.len().saturating_add(leading_singles.len()) > limit;
                candidates.splice(insert_at..insert_at, leading_singles);
                self.assign_ordered_candidate_qualities(&mut candidates);
                if candidates.len() > limit {
                    candidates.truncate(limit);
                }
                if overflows_page {
                    full_count = full_count.max(candidates.len().saturating_add(1));
                } else {
                    full_count = full_count.max(candidates.len());
                }
            }
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
        let mut pooled: Vec<PendingLookupCandidate> = Vec::new();
        let mut exact_scan_ranges: Vec<(usize, usize)> = Vec::new();
        let mut fetch_groups = HashMap::new();
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
                        let fetch_group = if self.sort_by_weight {
                            0
                        } else {
                            intern_fetch_group(&mut fetch_groups, candidate.raw_comment())
                        };
                        Some(PendingLookupCandidate {
                            fetch_group,
                            entry_code: lookup_code.to_owned(),
                            lookup_code: lookup_code.to_owned(),
                            candidate: candidate.to_candidate(),
                            correction_distance: lookup_spec.correction_distance,
                            spelling_abbreviation: self
                                .is_spelling_abbreviation_view(lookup_code, &candidate),
                            limited_prediction: false,
                            tolerance: lookup_spec.tolerance,
                            spelling_correction: lookup_spec.spelling_correction
                                || self.is_spelling_correction_view(lookup_code, &candidate),
                            spelling_credibility: lookup_spec.spelling_credibility,
                        })
                    }),
            );
            let exact_rows_in_pending = pending.len();
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
                    let fetch_group = if self.sort_by_weight {
                        0
                    } else {
                        intern_fetch_group(&mut fetch_groups, candidate.raw_comment())
                    };
                    completion_candidates.push(PendingLookupCandidate {
                        fetch_group,
                        entry_code: entry_code.into_owned(),
                        lookup_code: lookup_code.to_owned(),
                        candidate: candidate.to_candidate(),
                        correction_distance: lookup_spec.correction_distance,
                        spelling_abbreviation,
                        limited_prediction,
                        tolerance: lookup_spec.tolerance,
                        spelling_correction: lookup_spec.spelling_correction
                            || self.is_spelling_correction_view(lookup_code, &candidate),
                        spelling_credibility: lookup_spec.spelling_credibility,
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
                        .sort_by(|left, right| self.lookup_candidate_weight_order(left, right));
                    limited_predictions.truncate(limit);
                    limited_predictions.extend(ordinary_completions);
                    completion_candidates = limited_predictions;
                }
                pending.extend(completion_candidates);
            }
            if self.prediction_candidate_limit.is_some() && self.sort_by_weight {
                self.order_lookup_candidates(&mut pending);
            }
            // Exact rows sit at the head of `pending` in construction order; after
            // the prediction-limit sort the block boundary blurs, so record the
            // whole spec range in that (page-turn-only) mode. The detector below
            // walks ONLY these ranges — completion tails are never scanned, which
            // keeps the per-keystroke cost of the typing path at the pre-fix level.
            let scan_len = if self.prediction_candidate_limit.is_some() {
                pending.len()
            } else {
                exact_rows_in_pending
            };
            if scan_len > 0 {
                exact_scan_ranges.push((pooled.len(), scan_len));
            }
            pooled.append(&mut pending);
        }
        if !self.sort_by_weight {
            // `sort: original` is stable within each canonical fetch code, while
            // prism aliases and the selected prediction queue are comparable by
            // their current head weight. A constrained k-way merge preserves both
            // requirements without letting a later row jump its own group head.
            self.order_lookup_candidates(&mut pooled);
        }
        // M59 Lane A: a toneless syllable resolves (in the compiled prism/table
        // storage) to a single spec whose exact_candidates span several tone-codes
        // (`bei1`/`bei2`/`bei3`/`bei6`), concatenated in per-code storage order.
        // librime ranks that set by global per-reading weight (`bei` -> 畀 比 被 鼻
        // 避); the concatenation ranks it code-grouped. Both the detector and the
        // mutation below are scoped to TRUE EXACT rows (entry_code == lookup_code, no
        // abbreviation / correction / limited-prediction): if those rows are already
        // weight-non-increasing — every `sort: by_weight` single-code set by
        // construction, and any concatenation whose junctions do not increase, which
        // is exactly the already-weight-sorted case — nothing is touched. When a
        // strict `raw_quality` increase shows the exacts are not weight-sorted,
        // stable-sort ONLY those rows by per-reading `raw_quality` (essay x dict-%;
        // never the exp-saturated or positional `quality`) and reinsert them in the
        // same slots: ties keep storage order in both regimes, and completions,
        // corrections, and abbreviation rows never move, so downstream splice anchors
        // see an unchanged block structure. `sort: original` dictionaries make no
        // per-code weight-order promise, so the detector conditions the re-rank on
        // the observed order itself rather than assuming a storage invariant.
        fn is_true_exact(pending: &PendingLookupCandidate) -> bool {
            pending.entry_code == pending.lookup_code
                && !pending.spelling_abbreviation
                && !pending.limited_prediction
                && !pending.tolerance
                && pending.correction_distance.is_none()
                && !pending.spelling_correction
        }
        let mut needs_reorder = false;
        let mut prev_exact_quality: Option<f32> = None;
        // `sort: original` dictionaries contract to source row order (librime honors
        // it regardless of weights) — the tone-merge re-rank must never fire there.
        if !self.sort_by_weight {
            exact_scan_ranges.clear();
        }
        'detector: for &(start, len) in &exact_scan_ranges {
            for pending in &pooled[start..start + len] {
                if !is_true_exact(pending) {
                    continue;
                }
                let quality = pending.raw_quality();
                if prev_exact_quality.is_some_and(|prev| quality > prev) {
                    needs_reorder = true;
                    break 'detector;
                }
                prev_exact_quality = Some(quality);
            }
        }
        if needs_reorder {
            let slots: Vec<usize> = pooled
                .iter()
                .enumerate()
                .filter(|(_, pending)| is_true_exact(pending))
                .map(|(index, _)| index)
                .collect();
            let mut exact_rows: Vec<PendingLookupCandidate> =
                slots.iter().map(|&index| pooled[index].clone()).collect();
            exact_rows.sort_by(|left, right| right.raw_quality().total_cmp(&left.raw_quality()));
            for (slot, row) in slots.into_iter().zip(exact_rows) {
                pooled[slot] = row;
            }
        }
        if self.sort_by_weight && self.prefix_fallback {
            // The bounded prefix-fallback path orders full-surface table rows
            // ahead of completion rows through `lookup_candidate_ref_category`.
            // Apply the same category order to the eager/page-turn pool.  Prism
            // aliases are appended after the raw surface spec, so without this
            // shared order a completion from the raw spec could precede a later
            // full-surface exact alias even though the bounded list did not.
            self.order_lookup_candidates(&mut pooled);
        }
        let pending_count = pooled.len();
        let candidates = pooled
            .into_iter()
            .map(|pending| {
                self.candidate_for_lookup(
                    &pending.entry_code,
                    &pending.candidate,
                    &pending.lookup_code,
                    pending.correction_distance,
                    pending.spelling_credibility,
                )
            })
            .collect::<Vec<_>>();
        if record_track_b {
            for _ in 0..pending_count {
                crate::m37_record_track_b_candidate_materialized();
            }
        }
        candidates
    }

    fn prefix_fallback_view_is_allowed(
        &self,
        prefix_spec: &LookupPrefixSpec<'_>,
        candidate: &LookupCandidate<'_>,
        filter_by_charset: bool,
    ) -> bool {
        let prefix = prefix_spec.input_prefix;
        let admitted_by_surface = prefix_spec
            .surface_fetch
            .as_ref()
            .is_some_and(|fetch| leading_candidate_matches_fetch(candidate, fetch));
        let admitted_by_raw = prefix_spec.surface_fetch.is_none()
            && original_code_allows_prefix_fallback(candidate.raw_comment(), prefix);
        !self.is_spelling_correction_view(prefix, candidate)
            && (admitted_by_surface || admitted_by_raw)
            && self.is_dictionary_text_allowed(candidate.text())
            && (!filter_by_charset || !contains_extended_cjk(candidate.text()))
    }

    fn prefix_fallback_has_unique_candidate(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        existing_candidates: &[Candidate],
        request_limit: Option<usize>,
    ) -> PrefixFallbackProbe {
        let fallback_start = crate::m37_metrics_enabled().then(Instant::now);
        let prefixes = self.valid_lookup_prefixes(lookup_code);
        if prefixes.is_empty() {
            if let Some(start) = fallback_start {
                crate::m37_record_prefix_fallback(start.elapsed(), 0, 0);
            }
            return PrefixFallbackProbe::NoPrefix;
        }
        let seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<HashSet<_>>();
        let bounded_limit = if self.bounds_compact_fallback_expansion() {
            request_limit.filter(|limit| *limit > 0)
        } else {
            None
        };
        let pending_cap = bounded_limit
            .map(|limit| {
                limit
                    .saturating_mul(PREFIX_FALLBACK_BOUNDED_PENDING_MULTIPLIER)
                    .max(limit)
            })
            .unwrap_or(usize::MAX);
        let per_fetch_cap = bounded_limit
            .map(|_| {
                if input.chars().count() <= PREFIX_FALLBACK_BOUNDED_REACHABILITY_MAX_INPUT_CHARS {
                    PREFIX_FALLBACK_BOUNDED_REACHABILITY_CANDIDATES_PER_FETCH_CODE
                } else {
                    PREFIX_FALLBACK_BOUNDED_CANDIDATES_PER_FETCH_CODE
                }
            })
            .unwrap_or(usize::MAX);
        let mut views_visited = 0usize;
        let mut truncated = false;
        for prefix_spec in &prefixes {
            let exact_start = LookupTimer::start();
            let mut exact_candidates = 0usize;
            let mut emitted_for_fetch_code = 0usize;
            let mut found_unique = false;
            for candidate in self
                .storage
                .exact_candidates(&prefix_spec.fetch_code)
                .filter(|candidate| {
                    self.prefix_fallback_view_is_allowed(prefix_spec, candidate, filter_by_charset)
                })
            {
                views_visited += 1;
                exact_candidates += 1;
                emitted_for_fetch_code += 1;
                if !seen_texts.contains(candidate.text()) {
                    found_unique = true;
                    break;
                }
                if emitted_for_fetch_code >= per_fetch_cap || views_visited >= pending_cap {
                    truncated = true;
                    break;
                }
            }
            self.storage
                .record_exact_lookup(exact_start.elapsed(), exact_candidates);
            if found_unique {
                if let Some(start) = fallback_start {
                    crate::m37_record_prefix_fallback(start.elapsed(), views_visited, 1);
                }
                return PrefixFallbackProbe::Found;
            }
            if truncated {
                break;
            }
        }
        if let Some(start) = fallback_start {
            crate::m37_record_prefix_fallback(start.elapsed(), views_visited, 0);
        }
        if truncated {
            PrefixFallbackProbe::Truncated
        } else {
            PrefixFallbackProbe::Exhausted
        }
    }

    fn build_bounded_prefix_fallback_cache_entry(
        &self,
        prefixes: &[LookupPrefixSpec<'_>],
        filter_by_charset: bool,
        pending_cap: usize,
        per_fetch_cap: usize,
        key: PrefixFallbackWindowCacheKey,
    ) -> (Arc<PrefixFallbackWindowCacheEntry>, usize) {
        let mut rows = Vec::new();
        let mut emission_order = 0usize;
        let mut views_visited = 0usize;
        let mut truncated = false;
        let mut global_truncated = false;
        for prefix_spec in prefixes {
            let exact_start = LookupTimer::start();
            let mut exact_candidates = 0usize;
            let mut emitted_for_fetch_code = 0usize;
            for candidate in self
                .storage
                .exact_candidates(&prefix_spec.fetch_code)
                .filter(|candidate| {
                    self.prefix_fallback_view_is_allowed(prefix_spec, candidate, filter_by_charset)
                })
            {
                views_visited += 1;
                exact_candidates += 1;
                emitted_for_fetch_code += 1;
                let spelling_abbreviation =
                    self.is_spelling_abbreviation_view(prefix_spec.input_prefix, &candidate);
                rows.push(CachedPrefixFallbackView {
                    fetch_code: prefix_spec.fetch_code.clone(),
                    input_prefix: prefix_spec.input_prefix.to_owned(),
                    candidate: candidate.to_candidate(),
                    consumed_lookup_len: prefix_spec.consumed_lookup_len,
                    surface_abbreviation: prefix_spec
                        .surface_fetch
                        .as_ref()
                        .is_some_and(|fetch| fetch.abbreviation),
                    spelling_abbreviation,
                    emission_order,
                });
                emission_order += 1;
                if rows.len() >= pending_cap {
                    truncated = true;
                    global_truncated = true;
                    break;
                }
                if emitted_for_fetch_code >= per_fetch_cap {
                    truncated = true;
                    break;
                }
            }
            self.storage
                .record_exact_lookup(exact_start.elapsed(), exact_candidates);
            if global_truncated {
                break;
            }
        }
        (
            Arc::new(PrefixFallbackWindowCacheEntry {
                key,
                rows,
                truncated,
            }),
            views_visited,
        )
    }

    fn materialize_bounded_prefix_fallback_entry(
        &self,
        entry: &PrefixFallbackWindowCacheEntry,
        input: &str,
        lookup_code: &str,
        existing_candidates: &[Candidate],
        admitted_span_candidates: &[Candidate],
        limit: usize,
    ) -> PrefixFallbackBatch {
        let mut seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<HashSet<_>>();
        let full_span_texts = admitted_span_candidates
            .iter()
            .filter(|candidate| candidate.source == CandidateSource::Table)
            .map(|candidate| candidate.text.as_str())
            .collect::<HashSet<_>>();
        struct CachedPendingPrefixCandidate<'a> {
            view: &'a CachedPrefixFallbackView,
            consumed_input_len: usize,
            recompose_on_default: bool,
        }
        let input_base = input.len().saturating_sub(lookup_code.len());
        let mut pending = entry
            .rows
            .iter()
            .map(|view| {
                let consumed_input_len = if full_span_texts.contains(view.candidate.text.as_str()) {
                    input.len()
                } else {
                    input_base.saturating_add(view.consumed_lookup_len)
                };
                CachedPendingPrefixCandidate {
                    view,
                    consumed_input_len,
                    recompose_on_default: consumed_input_len > 1
                        && !view.surface_abbreviation
                        && !view.spelling_abbreviation,
                }
            })
            .collect::<Vec<_>>();
        pending.sort_by(|left, right| {
            right
                .consumed_input_len
                .cmp(&left.consumed_input_len)
                .then_with(|| {
                    right
                        .view
                        .candidate
                        .quality
                        .partial_cmp(&left.view.candidate.quality)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| left.view.emission_order.cmp(&right.view.emission_order))
        });
        let pending_len = pending.len();
        let mut candidates = Vec::new();
        let mut truncated = entry.truncated;
        for (index, pending) in pending.into_iter().enumerate() {
            let mut candidate = self.format_candidate_for_lookup(
                &pending.view.fetch_code,
                pending.view.candidate.clone(),
                &pending.view.input_prefix,
                None,
                0.0,
            );
            if !seen_texts.insert(candidate.text.clone()) {
                continue;
            }
            candidate.source = CandidateSource::PartialTable {
                consumed: pending.consumed_input_len,
                recompose_on_default: pending.recompose_on_default,
            };
            candidates.push(candidate);
            if candidates.len() >= limit {
                truncated |= index + 1 < pending_len;
                break;
            }
        }
        PrefixFallbackBatch {
            candidates,
            truncated,
            owns_reachability: true,
        }
    }

    fn bounded_prefix_fallback_candidates_cached(
        &self,
        request: BoundedPrefixFallbackCacheRequest<'_, '_>,
    ) -> Option<PrefixFallbackBatch> {
        let BoundedPrefixFallbackCacheRequest {
            input,
            lookup_code,
            filter_by_charset,
            existing_candidates,
            admitted_span_candidates,
            prefixes,
            limit,
            fallback_start,
        } = request;
        let pending_cap = limit
            .saturating_mul(PREFIX_FALLBACK_BOUNDED_PENDING_MULTIPLIER)
            .max(limit);
        let per_fetch_cap =
            if input.chars().count() <= PREFIX_FALLBACK_BOUNDED_REACHABILITY_MAX_INPUT_CHARS {
                PREFIX_FALLBACK_BOUNDED_REACHABILITY_CANDIDATES_PER_FETCH_CODE
            } else {
                PREFIX_FALLBACK_BOUNDED_CANDIDATES_PER_FETCH_CODE
            };
        let key = PrefixFallbackWindowCacheKey {
            prefixes: prefixes
                .iter()
                .map(|prefix| PrefixFallbackCachePrefix {
                    input_prefix: prefix.input_prefix.to_owned(),
                    fetch_code: prefix.fetch_code.clone(),
                    consumed_lookup_len: prefix.consumed_lookup_len,
                    surface_fetch: prefix.surface_fetch.clone(),
                })
                .collect(),
            filter_by_charset,
            pending_cap,
            per_fetch_cap,
        };
        let cached = {
            let cache = self
                .prefix_fallback_window_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            cache.as_ref().filter(|entry| entry.key == key).cloned()
        };
        let mut views_visited = 0usize;
        let entry = if let Some(entry) = cached {
            entry
        } else {
            let (built, built_views) = self.build_bounded_prefix_fallback_cache_entry(
                prefixes,
                filter_by_charset,
                pending_cap,
                per_fetch_cap,
                key,
            );
            views_visited = built_views;
            if estimate_prefix_fallback_window_cache_key_bytes(&built.key)
                > PREFIX_FALLBACK_CACHE_MAX_KEY_BYTES
                || estimate_prefix_fallback_window_cache_bytes(&built)
                    > PREFIX_FALLBACK_CACHE_MAX_ENTRY_BYTES
            {
                // Keep any prior admitted entry intact. This oversized window
                // must use the borrowed bounded collector and must never become
                // retained translator state.
                return None;
            }
            let mut cache = self
                .prefix_fallback_window_cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(existing) = cache
                .as_ref()
                .filter(|entry| entry.key == built.key)
                .cloned()
            {
                existing
            } else {
                *cache = Some(Arc::clone(&built));
                built
            }
        };
        let batch = self.materialize_bounded_prefix_fallback_entry(
            &entry,
            input,
            lookup_code,
            existing_candidates,
            admitted_span_candidates,
            limit,
        );
        if let Some(start) = fallback_start {
            crate::m37_record_prefix_fallback(
                start.elapsed(),
                views_visited,
                batch.candidates.len(),
            );
        }
        Some(batch)
    }

    fn prefix_fallback_candidates(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        existing_candidates: &[Candidate],
        admitted_span_candidates: &[Candidate],
        request_limit: Option<usize>,
    ) -> PrefixFallbackBatch {
        let fallback_start = crate::m37_metrics_enabled().then(Instant::now);
        let prefixes = self.valid_lookup_prefixes(lookup_code);
        if prefixes.is_empty() {
            if let Some(start) = fallback_start {
                crate::m37_record_prefix_fallback(start.elapsed(), 0, 0);
            }
            return PrefixFallbackBatch {
                candidates: Vec::new(),
                truncated: false,
                owns_reachability: false,
            };
        };
        let bounded_limit = if self.bounds_compact_fallback_expansion() {
            request_limit.filter(|limit| *limit > 0)
        } else {
            None
        };
        if let Some(limit) = bounded_limit {
            let pending_cap = limit
                .saturating_mul(PREFIX_FALLBACK_BOUNDED_PENDING_MULTIPLIER)
                .max(limit);
            let cache_admitted = pending_cap <= PREFIX_FALLBACK_CACHE_MAX_ROWS
                && prefixes.len() <= PREFIX_FALLBACK_CACHE_MAX_PREFIXES
                && prefix_fallback_cache_key_bytes(&prefixes)
                    <= PREFIX_FALLBACK_CACHE_MAX_KEY_BYTES;
            if cache_admitted {
                if let Some(batch) = self.bounded_prefix_fallback_candidates_cached(
                    BoundedPrefixFallbackCacheRequest {
                        input,
                        lookup_code,
                        filter_by_charset,
                        existing_candidates,
                        admitted_span_candidates,
                        prefixes: &prefixes,
                        limit,
                        fallback_start,
                    },
                ) {
                    return batch;
                }
            }
        }
        let mut seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<HashSet<_>>();
        // The unbounded merge can insert a shorter-prefix family before full
        // exact rows. A later uniqueness filter keeps the first duplicate text,
        // so retain the prefix row's reachable position but promote its consumed
        // span when this same translator has already admitted the same text as a
        // full exact for the current lookup. Otherwise selecting `zi` -> 子 from
        // the earlier `z` abbreviation family leaves raw `i` behind.
        let full_span_texts = admitted_span_candidates
            .iter()
            .filter(|candidate| candidate.source == CandidateSource::Table)
            .map(|candidate| candidate.text.as_str())
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
        let mut truncated = false;
        let mut global_truncated = false;
        let output_cap = bounded_limit.unwrap_or(usize::MAX);
        let pending_cap = bounded_limit
            .map(|limit| {
                limit
                    .saturating_mul(PREFIX_FALLBACK_BOUNDED_PENDING_MULTIPLIER)
                    .max(limit)
            })
            .unwrap_or(usize::MAX);
        let per_fetch_cap = bounded_limit
            .map(|_| {
                if input.chars().count() <= PREFIX_FALLBACK_BOUNDED_REACHABILITY_MAX_INPUT_CHARS {
                    PREFIX_FALLBACK_BOUNDED_REACHABILITY_CANDIDATES_PER_FETCH_CODE
                } else {
                    PREFIX_FALLBACK_BOUNDED_CANDIDATES_PER_FETCH_CODE
                }
            })
            .unwrap_or(usize::MAX);
        for prefix_spec in &prefixes {
            let prefix = prefix_spec.input_prefix;
            let fetch_code = prefix_spec.fetch_code.as_str();
            let prefix_consumed_input_len = input
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
                    self.prefix_fallback_view_is_allowed(prefix_spec, candidate, filter_by_charset)
                })
            {
                views_visited += 1;
                exact_candidates += 1;
                let consumed_input_len = if full_span_texts.contains(candidate.text()) {
                    input.len()
                } else {
                    prefix_consumed_input_len
                };
                let recompose_on_default = consumed_input_len > 1
                    && !prefix_spec
                        .surface_fetch
                        .as_ref()
                        .is_some_and(|fetch| fetch.abbreviation)
                    && !self.is_spelling_abbreviation_view(prefix, &candidate);
                pending.push(PendingPrefixCandidate {
                    pending: PendingLookupCandidateRef {
                        fetch_group: 0,
                        entry_code: Cow::Owned(fetch_code.to_owned()),
                        lookup_code: prefix,
                        candidate,
                        correction_distance: None,
                        spelling_abbreviation: false,
                        limited_prediction: false,
                        emission_order,
                        spelling_correction: false,
                        spelling_credibility: 0.0,
                    },
                    consumed_input_len,
                    recompose_on_default,
                });
                emission_order += 1;
                emitted_for_fetch_code += 1;
                if pending.len() >= pending_cap {
                    truncated = true;
                    global_truncated = true;
                    break;
                }
                if emitted_for_fetch_code >= per_fetch_cap {
                    truncated = true;
                    break;
                }
            }
            self.storage
                .record_exact_lookup(exact_start.elapsed(), exact_candidates);
            if global_truncated {
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
        let pending_len = pending.len();
        for (index, pending) in pending.into_iter().enumerate() {
            let mut candidate = self.candidate_for_lookup_view(
                pending.pending.entry_code.as_ref(),
                &pending.pending.candidate,
                pending.pending.lookup_code,
                None,
                0.0,
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
                truncated |= index + 1 < pending_len;
                break;
            }
        }
        if let Some(start) = fallback_start {
            crate::m37_record_prefix_fallback(start.elapsed(), views_visited, candidates.len());
        }
        PrefixFallbackBatch {
            candidates,
            truncated,
            owns_reachability: true,
        }
    }

    fn leading_single_syllable_prefix_candidates(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        existing_candidates: &[Candidate],
        // `Some(n)` caps materialization to `n` rows via lazy early-stop (bounded
        // typing path). `None` materializes the full family (page-turn/complete).
        fetch_limit: Option<usize>,
    ) -> Vec<Candidate> {
        if !has_proper_leading_prefix(lookup_code) {
            return Vec::new();
        }
        // M59 increment-2 stop-the-line: when the FULL input is itself a complete
        // syllable the exact path already serves with single-char candidates
        // (`mai`→買/賣/麥/邁, `wai`→外, `xian`→先/現), skip the injection entirely.
        // The prefix walk below EXCLUDES the full input, so for this class it can
        // only ever inject a DIFFERENT, shorter-prefix family (`ma`, `wa`, `xi`);
        // spliced above the exacts at `leading_single_insert_index` and then
        // truncated to the page, that family DROPS the full-syllable exacts —
        // typing `mai` could no longer produce 買 at any page (regression from the
        // increment-1 injection). Leading-single reachability exists for
        // MULTI-syllable composition, where the full input has no single-char
        // exact of its own; those inputs (`zhongguo`→中國 phrase, `moboyi`→no
        // exact) keep the injection. Flag-gated so jyutping/prefix_fallback is
        // untouched (its canonical parity is the oracle).
        // Retained as the walk's own safety net (the unbounded/page-turn path at
        // :2933 has no call-site pre-check); the bounded per-keystroke callers gate
        // on the same predicate before ever calling in, so this is dead for them.
        if self.leading_syllable_reachability && self.input_serves_single_char_exact(lookup_code) {
            return Vec::new();
        }
        let mut seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<HashSet<_>>();
        // M59 finding #3: walk leading-syllable prefixes LONGEST-FIRST and stop at
        // the first non-empty family. The old short->long walk kept only the last
        // (longest) family but let `seen_texts` accumulate across the DISCARDED
        // shorter-prefix families, so a character reachable under a shorter
        // abbreviation prefix (e.g. 中 under `z`/`zh`) was marked seen and then
        // suppressed from the kept `zhong` family — unreachable at any page.
        // Longest-first + stop also drops the discarded fetch work.
        // M59 finding #8 (range cap): a leading syllable is never longer than the
        // longest syllabary code, so boundaries beyond that can't match — cap the
        // walk to `max_leading_prefix_len`. On a 37/59-char input this bounds the
        // per-keystroke walk to ~one syllable's worth of boundaries instead of the
        // whole input (the O(n^2) long-input cost the boundary skip only partly
        // cut). Safe: the boundary carrying the leading single always survives the
        // cap. (A dictionary with no syllabary reports max 0 via all_codes; guard
        // against that degenerate case by leaving the walk uncapped there.)
        for (end, fetches) in self.leading_prefix_fetch_groups(lookup_code) {
            let prefix = &lookup_code[..end];
            // A recognized correction-only surface is an exact syllable guard,
            // not a default injection edge. Once the longest-first walk reaches
            // it, do not fall through and inject a shorter abbreviation family.
            let has_matching_single = |fetch: &LeadingFetchCode| {
                self.storage
                    .exact_candidates(&fetch.fetch_code)
                    .any(|candidate| {
                        candidate.text().chars().count() == 1
                            && leading_candidate_matches_fetch(&candidate, fetch)
                    })
            };
            let has_valid_correction = fetches
                .iter()
                .filter(|fetch| fetch.bare_exact && !fetch.injectable)
                .any(has_matching_single);
            let has_valid_injectable = fetches
                .iter()
                .filter(|fetch| fetch.injectable)
                .any(has_matching_single);
            if has_valid_correction && !has_valid_injectable {
                return Vec::new();
            }
            // M59 finding #8 (completing the skip): this walk keeps only
            // single-char candidates, whose codes are syllabary codes.
            // `leading_syllable_fetch_codes` can surface such a code for `prefix`
            // only when a syllabary code normalizes to `prefix` — and BOTH of its
            // sources are so bounded: the prism alias lookup in
            // `sentence_lookup_specs` is restricted to `syllabary_codes`, and the
            // second loop reads the memoized index, whose keys are exactly the
            // normalized syllabary codes. So an absent index entry means no
            // single-char family lives at this boundary; skip the per-prefix,
            // per-keystroke prism lookup. Together with the bounded iterator
            // above, the former O(n^2) long-input prefix work becomes a constant
            // number of O(1) map probes per keystroke for normal dictionaries.
            // (Guarded behavior-preserving by the reachability tests, which fail
            // loudly if a reachable leading single is dropped.)
            let mut candidates = Vec::new();
            for fetch in fetches.into_iter().filter(|fetch| fetch.injectable) {
                for candidate in
                    self.storage
                        .exact_candidates(&fetch.fetch_code)
                        .filter(|candidate| {
                            candidate.text().chars().count() == 1
                                && leading_candidate_matches_fetch(candidate, &fetch)
                                && {
                                    // Toned codes (jyutping `bei2`) count syllables
                                    // by tone digit. Untoned codes (luna `mo`) have
                                    // none — accept those ONLY when the DICTIONARY
                                    // itself is untoned (M59 finding #6: keyed on
                                    // code STRUCTURE, not the reachability flag, so
                                    // the default-ON flip cannot admit digit-less
                                    // rows into a toned family and shift the M58
                                    // pins). The fetch code is already a single
                                    // leading syllable, so a single-char entry is
                                    // one syllable by construction. A byte-backed
                                    // null-map identity descriptor plus its exact
                                    // one-character row is equivalent local proof;
                                    // it avoids classifying Stroke's 157k codes on
                                    // the cold first key. Explicit/algebra maps keep
                                    // the dictionary-wide toned-family guard.
                                    let syllables =
                                        source_code_syllable_count(candidate.raw_comment());
                                    syllables == Some(1)
                                        || (syllables.is_none()
                                            && (fetch.direct_identity || self.untoned_dictionary()))
                                }
                                && self.is_dictionary_text_allowed(candidate.text())
                                && (!filter_by_charset || !contains_extended_cjk(candidate.text()))
                        })
                {
                    // `prefix` is the spelling the user actually typed. It may be
                    // transformed by the deployed prism (`hk` -> canonical `hao`,
                    // `cl3` -> canonical toned code), so the canonical raw comment
                    // cannot decide admission or how much input was consumed. The
                    // fetch index below admits only deployed surface->storage-code
                    // edges; keep the raw comment untouched for tone/syllable
                    // metadata, but use the surface spelling for positional state.
                    let candidate_prefix = prefix;
                    let consumed_input_len = input
                        .len()
                        .saturating_sub(lookup_code.len())
                        .saturating_add(candidate_prefix.len());
                    let mut materialized = self.candidate_for_lookup_view(
                        &fetch.fetch_code,
                        &candidate,
                        candidate_prefix,
                        None,
                        0.0,
                    );
                    if !seen_texts.insert(materialized.text.clone()) {
                        continue;
                    }
                    materialized.source = CandidateSource::PartialTable {
                        consumed: consumed_input_len,
                        // M59 finding #5: recompose whenever the selected single
                        // consumes a PROPER prefix (a remainder is left), not only
                        // when the code is >1 char. Single-letter vowel syllables
                        // (e→俄, a→阿, o→哦) consume 1 char, so the old `> 1` gate
                        // committed the tail raw (`俄luosi`) instead of recomposing
                        // it to `luosi`. `< input.len()` keys on "is there a
                        // remainder", independent of code length; the abbreviation
                        // guard mirrors the toned prefix-fallback path (:2437).
                        recompose_on_default: consumed_input_len < input.len()
                            && !self.is_spelling_abbreviation_view(candidate_prefix, &candidate),
                    };
                    candidates.push(materialized);
                    if fetch_limit.is_some_and(|cap| candidates.len() >= cap) {
                        break;
                    }
                }
                if fetch_limit.is_some_and(|cap| candidates.len() >= cap) {
                    break;
                }
            }
            if !candidates.is_empty() {
                return candidates;
            }
        }
        Vec::new()
    }

    fn leading_prefix_fetch_groups(
        &self,
        lookup_code: &str,
    ) -> Vec<(usize, Vec<LeadingFetchCode>)> {
        if self.leading_fetch_index_seed.is_none() && self.direct_prism_surface_mapping_current {
            if let (Some(prism), Some(syllabary_codes)) =
                (self.prism_payload.as_ref(), self.storage.syllabary_codes())
            {
                let mut groups: Vec<(usize, Vec<LeadingFetchCode>)> = Vec::new();
                let direct_identity = prism.has_byte_backed_identity_spelling_map();
                for (length, lookup) in
                    prism.common_prefix_canonical_codes(lookup_code, syllabary_codes, usize::MAX)
                {
                    if length >= lookup_code.len() {
                        continue;
                    }
                    let edge = LeadingFetchCode {
                        fetch_code: lookup.code.to_owned(),
                        canonical_code: lookup.code.to_owned(),
                        bare_exact: true,
                        injectable: !lookup.correction,
                        abbreviation: lookup.abbreviation,
                        direct_identity,
                    };
                    if let Some((_, fetches)) = groups
                        .iter_mut()
                        .find(|(group_length, _)| *group_length == length)
                    {
                        if !fetches.contains(&edge) {
                            fetches.push(edge);
                        }
                    } else {
                        groups.push((length, vec![edge]));
                    }
                }
                if !self.spelling_algebra_active {
                    for (length, lookup) in prism.trailing_ascii_digit_prefix_canonical_codes(
                        lookup_code,
                        syllabary_codes,
                        usize::MAX,
                    ) {
                        if length >= lookup_code.len() {
                            continue;
                        }
                        let edge = LeadingFetchCode {
                            fetch_code: lookup.code.to_owned(),
                            canonical_code: lookup.code.to_owned(),
                            bare_exact: false,
                            injectable: !lookup.correction,
                            abbreviation: lookup.abbreviation,
                            direct_identity,
                        };
                        if let Some((_, fetches)) = groups
                            .iter_mut()
                            .find(|(group_length, _)| *group_length == length)
                        {
                            if !fetches.contains(&edge) {
                                fetches.push(edge);
                            }
                        } else {
                            groups.push((length, vec![edge]));
                        }
                    }
                }
                groups.sort_unstable_by_key(|right| std::cmp::Reverse(right.0));
                // A checksum-validated prism miss is authoritative; do not
                // turn it into a boundary-by-boundary storage scan.
                return groups;
            }
        }

        // Algebra-backed paths cap by the indexed syllable inventory. Direct
        // heap/source identity mode has no global seed by design, so it probes
        // the finite set of boundaries in this input only.
        let has_index_seed = self.leading_fetch_index_seed.is_some();
        let max_prefix_len = if has_index_seed {
            self.max_leading_prefix_len()
        } else {
            0
        };
        if has_index_seed && max_prefix_len == 0 {
            return Vec::new();
        }
        let mut prefix_ends: Vec<usize> = lookup_code
            .char_indices()
            .map(|(index, _)| index)
            .skip(1)
            .take_while(|index| !has_index_seed || *index <= max_prefix_len)
            .collect();
        prefix_ends.sort_unstable_by(|left, right| right.cmp(left));
        prefix_ends
            .into_iter()
            .filter_map(|end| {
                let fetches = self.leading_surface_fetch_codes(&lookup_code[..end]);
                (!fetches.is_empty()).then_some((end, fetches))
            })
            .collect()
    }

    fn leading_surface_fetch_codes(&self, prefix: &str) -> Vec<LeadingFetchCode> {
        if self.leading_fetch_index_seed.is_none() {
            return self.direct_no_algebra_fetch_codes(prefix);
        }
        self.leading_fetch_index()
            .get(prefix)
            .into_iter()
            .flatten()
            .cloned()
            .collect()
    }

    fn direct_no_algebra_fetch_codes(&self, prefix: &str) -> Vec<LeadingFetchCode> {
        if self.direct_prism_surface_mapping_current {
            if let (Some(prism), Some(syllabary_codes)) =
                (self.prism_payload.as_ref(), self.storage.syllabary_codes())
            {
                let lookups = prism.lookup_canonical_codes(prefix, syllabary_codes);
                let direct_identity = prism.has_byte_backed_identity_spelling_map();
                let mut fetches = lookups
                    .into_iter()
                    .map(|lookup| LeadingFetchCode {
                        fetch_code: lookup.code.to_owned(),
                        canonical_code: lookup.code.to_owned(),
                        bare_exact: true,
                        injectable: !lookup.correction,
                        abbreviation: lookup.abbreviation,
                        direct_identity,
                    })
                    .collect::<Vec<_>>();
                if !self.spelling_algebra_active {
                    fetches.extend(
                        prism
                            .trailing_ascii_digit_prefix_canonical_codes(
                                prefix,
                                syllabary_codes,
                                usize::MAX,
                            )
                            .into_iter()
                            .filter(|(consumed, _)| *consumed == prefix.len())
                            .map(|(_, lookup)| LeadingFetchCode {
                                fetch_code: lookup.code.to_owned(),
                                canonical_code: lookup.code.to_owned(),
                                bare_exact: false,
                                injectable: !lookup.correction,
                                abbreviation: lookup.abbreviation,
                                direct_identity,
                            }),
                    );
                }
                fetches.dedup();
                return fetches;
            }
        }
        self.direct_storage_identity_fetch_codes(prefix)
    }

    fn direct_storage_identity_fetch_codes(&self, prefix: &str) -> Vec<LeadingFetchCode> {
        let mut fetches = Vec::new();
        let mut seen = HashSet::new();
        let mut storage_codes = Vec::with_capacity(11);
        storage_codes.push(prefix.to_owned());
        // The only no-algebra normalization retained by an owning regression is
        // a trailing tone digit (`bei2` -> surface `bei`). Probe that finite
        // family directly instead of enumerating every key beginning with an
        // arbitrary prefix (catastrophic on Stroke misses).
        if prefix.chars().all(|ch| ch.is_ascii_alphabetic()) {
            storage_codes.extend((0..=9).map(|tone| format!("{prefix}{tone}")));
        }
        for storage_code in storage_codes {
            for candidate in self.storage.exact_candidates(&storage_code) {
                if candidate.text().chars().count() != 1 {
                    continue;
                }
                let canonical = canonical_fetch_group(candidate.raw_comment()).into_owned();
                let edge = LeadingFetchCode {
                    fetch_code: storage_code.clone(),
                    canonical_code: canonical,
                    // A normalized tone alias is an injection edge, not an
                    // exact spelling served by the ordinary lookup path.
                    bare_exact: storage_code == prefix,
                    injectable: true,
                    abbreviation: false,
                    direct_identity: true,
                };
                if seen.insert(edge.clone()) {
                    fetches.push(edge);
                }
            }
        }
        fetches
    }

    fn leading_fetch_index(&self) -> &HashMap<String, Vec<LeadingFetchCode>> {
        self.leading_fetch_index_cache.get_or_init(|| {
            if let Some(seed) = &self.leading_fetch_index_seed {
                return self.deployed_algebra_leading_fetch_index(seed);
            }
            // Empty/invalid algebra is deliberately index-free. Compact
            // translators traverse the prism Darts directly; heap/source
            // translators probe only the input boundaries requested by the
            // current lookup. This is the Stroke 157k-spelling memory boundary.
            HashMap::new()
        })
    }

    /// Longest deployed surface that owns a single-character candidate. Broad
    /// heap/source prefix fallback may retain much longer phrase codes in the
    /// shared index, but those must never widen this leading-syllable cap.
    fn max_leading_prefix_len(&self) -> usize {
        *self.max_leading_prefix_len_cache.get_or_init(|| {
            self.leading_fetch_index_seed
                .as_ref()
                .map_or(0, |seed| seed.max_leading_single_surface_len)
        })
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
            let mut boundary_prefixes = Vec::new();
            let mut has_valid_normal = false;
            let mut has_valid_correction = false;
            let mut direct_normal_groups = HashSet::new();

            if self.storage.has_code(prefix) {
                let mut has_direct_normal = false;
                for candidate in self.storage.exact_candidates(prefix) {
                    if self.is_spelling_correction_view(prefix, &candidate) {
                        has_valid_correction = true;
                    } else if original_code_allows_prefix_fallback(candidate.raw_comment(), prefix)
                    {
                        has_direct_normal = true;
                        direct_normal_groups
                            .insert(canonical_fetch_group(candidate.raw_comment()).into_owned());
                    }
                }
                if has_direct_normal {
                    has_valid_normal = true;
                    boundary_prefixes.push(LookupPrefixSpec {
                        input_prefix: prefix,
                        fetch_code: prefix.to_owned(),
                        consumed_lookup_len: end,
                        surface_fetch: None,
                    });
                }
            }

            let mapped_fetches = if self.direct_prism_surface_mapping_current {
                self.direct_no_algebra_fetch_codes(prefix)
            } else if !self.spelling_algebra_active || self.leading_fetch_index_seed.is_some() {
                self.leading_surface_fetch_codes(prefix)
            } else {
                Vec::new()
            };

            let mut seen_mapped_fetches = HashSet::new();
            for fetch in mapped_fetches {
                if direct_normal_groups.contains(&fetch.canonical_code) {
                    continue;
                }
                if !self.storage.has_code(&fetch.fetch_code)
                    || !self
                        .storage
                        .exact_candidates(&fetch.fetch_code)
                        .any(|candidate| leading_candidate_matches_fetch(&candidate, &fetch))
                {
                    continue;
                }
                if !fetch.injectable {
                    has_valid_correction = true;
                    continue;
                }
                has_valid_normal = true;
                if seen_mapped_fetches.insert(fetch.clone()) {
                    boundary_prefixes.push(LookupPrefixSpec {
                        input_prefix: prefix,
                        fetch_code: fetch.fetch_code.clone(),
                        consumed_lookup_len: end,
                        surface_fetch: Some(fetch),
                    });
                }
            }

            if has_valid_correction && !has_valid_normal {
                break;
            }
            prefixes.extend(boundary_prefixes);
        }
        prefixes
    }

    fn assign_ordered_candidate_qualities(&self, candidates: &mut [Candidate]) {
        // Ordered modes deliberately replace raw per-entry weights to preserve
        // upstream list order. Keep the translator namespace's configured
        // offset, though: librime `initial_quality` still controls interleaving
        // across translators, including when D-47 injects a leading family.
        // Keep positional ranks in a count-independent unit band so a long
        // injected family cannot swamp another translator's initial_quality.
        let candidate_count = candidates.len();
        let denominator = candidate_count as f32 + 1.0;
        for (index, candidate) in candidates.iter_mut().enumerate() {
            let rank = (candidate_count - index) as f32 / denominator;
            candidate.quality = self.initial_quality + rank;
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
        self.translated_candidates_for_segment_with_prefix_fallback_limit(
            input,
            filter_by_charset,
            segment_tags,
            None,
        )
        .candidates
    }

    fn translated_candidates_for_segment_with_prefix_fallback_limit(
        &self,
        input: &str,
        filter_by_charset: bool,
        segment_tags: Option<&[String]>,
        prefix_fallback_limit: Option<usize>,
    ) -> PrefixFallbackBatch {
        let accepts_segment = segment_tags
            .map(|tags| self.accepts_segment_tags(tags))
            .unwrap_or_else(|| self.accepts_default_segment());
        if !accepts_segment {
            return PrefixFallbackBatch {
                candidates: Vec::new(),
                truncated: false,
                owns_reachability: false,
            };
        }

        let Some(lookup_code) = self.lookup_code(input) else {
            return PrefixFallbackBatch {
                candidates: Vec::new(),
                truncated: false,
                owns_reachability: false,
            };
        };
        let expanded_lookup_codes = self.expanded_lookup_specs(lookup_code);
        let mut candidates =
            self.candidates_for_lookup_codes(&expanded_lookup_codes, filter_by_charset);
        let has_correction_lookup = expanded_lookup_codes
            .iter()
            .any(|spec| spec.correction_distance.is_some() || spec.spelling_correction);
        if self.combine_candidates {
            candidates = combine_duplicate_text_candidates(candidates);
        }
        self.enforce_prediction_never_first(&mut candidates);

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
                }
            }
        }
        let mut sentence_over_completion_floored = false;
        if candidates.is_empty() && self.enable_sentence {
            if let Some(sentence) = self.sentence_candidate(input, filter_by_charset, None) {
                candidates.push(sentence);
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
                sentence_over_completion_floored = true;
            }
        }

        let mut prefix_fallback_truncated = false;
        let mut prefix_fallback_owned = false;
        if self.prefix_fallback && !has_correction_lookup {
            let fallback_room =
                prefix_fallback_limit.map(|limit| limit.saturating_sub(candidates.len()));
            let prefix_batch = if fallback_room == Some(0) {
                let probe = self.prefix_fallback_has_unique_candidate(
                    input,
                    lookup_code,
                    filter_by_charset,
                    &candidates,
                    prefix_fallback_limit,
                );
                PrefixFallbackBatch {
                    candidates: Vec::new(),
                    truncated: matches!(
                        probe,
                        PrefixFallbackProbe::Found | PrefixFallbackProbe::Truncated
                    ),
                    owns_reachability: !matches!(probe, PrefixFallbackProbe::NoPrefix),
                }
            } else {
                let needs_span_promotion = candidates
                    .iter()
                    .any(|candidate| candidate.source == CandidateSource::Table);
                let existing_candidates = if needs_span_promotion {
                    &[][..]
                } else {
                    candidates.as_slice()
                };
                self.prefix_fallback_candidates(
                    input,
                    lookup_code,
                    filter_by_charset,
                    existing_candidates,
                    &candidates,
                    if needs_span_promotion {
                        prefix_fallback_limit
                    } else {
                        fallback_room
                    },
                )
            };
            prefix_fallback_owned = prefix_batch.owns_reachability;
            prefix_fallback_truncated |= prefix_batch.truncated;
            let inserted = merge_prefix_fallback_candidates(
                &mut candidates,
                prefix_batch.candidates,
                lookup_code,
            );
            if prefix_fallback_truncated && inserted == 0 {
                if let Some(limit) = prefix_fallback_limit {
                    prefix_fallback_truncated = matches!(
                        self.prefix_fallback_has_unique_candidate(
                            input,
                            lookup_code,
                            filter_by_charset,
                            &candidates,
                            Some(limit),
                        ),
                        PrefixFallbackProbe::Found | PrefixFallbackProbe::Truncated
                    );
                }
            }
            if let Some(limit) = prefix_fallback_limit {
                if candidates.len() > limit {
                    prefix_fallback_truncated = true;
                    candidates.truncate(limit);
                }
            }
        }
        let mut leading_singles_inserted = false;
        // A deployed proper prefix makes prefix fallback authoritative for this
        // input. Otherwise keep the independent leading-single mechanism
        // available even when the schema enables prefix fallback; suppressing
        // it from the schema flag alone creates a bounded reachability hole.
        if self.leading_syllable_reachability && !prefix_fallback_owned {
            let insert_at = leading_single_insert_index(&candidates);
            let fetch_limit = prefix_fallback_limit
                .map(|limit| limit.saturating_sub(insert_at).saturating_add(1));
            let leading_singles = self.leading_single_syllable_prefix_candidates(
                input,
                lookup_code,
                filter_by_charset,
                &candidates,
                fetch_limit,
            );
            leading_singles_inserted = !leading_singles.is_empty();
            candidates.splice(insert_at..insert_at, leading_singles);
            if let Some(limit) = prefix_fallback_limit {
                if candidates.len() > limit {
                    prefix_fallback_truncated = true;
                    candidates.truncate(limit);
                }
            }
        }
        if self.prefix_fallback
            || self.prediction_candidate_limit.is_some()
            || leading_singles_inserted
        {
            self.assign_ordered_candidate_qualities(&mut candidates);
            // M59 finding #10: the positional overwrite above ranks by list index,
            // which clobbers the `sentence_over_completion` priority floor — the
            // floored sentence sits at the tail (pushed after the completions), so
            // it would be demoted to last, defeating the whole point of the floor.
            // Re-float it above the positional ranks. (Scoping the overwrite to the
            // injected rows would instead risk reordering the live luna
            // phrases/completions, so we preserve the all-rows ordering and just
            // restore the one deliberately-floored row. Latent today: no shipped
            // schema enables `sentence_over_completion`.)
            if sentence_over_completion_floored {
                let top = candidates
                    .iter()
                    .map(|candidate| candidate.quality)
                    .fold(f32::MIN, f32::max);
                if let Some(sentence) = candidates
                    .iter_mut()
                    .find(|candidate| candidate.source == CandidateSource::Sentence)
                {
                    sentence.quality = top + 1.0;
                }
            }
        }

        PrefixFallbackBatch {
            candidates,
            truncated: prefix_fallback_truncated,
            owns_reachability: prefix_fallback_owned,
        }
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
            if !self.bounds_compact_fallback_expansion() {
                return TranslationResult::complete(self.translated_candidates_for_segment(
                    input,
                    filter_by_charset,
                    segment_tags,
                ));
            }
            let batch = self.translated_candidates_for_segment_with_prefix_fallback_limit(
                input,
                filter_by_charset,
                segment_tags,
                Some(limit),
            );
            let full_count = if batch.truncated || batch.candidates.len() > limit {
                batch.candidates.len().saturating_add(1)
            } else {
                batch.candidates.len()
            };
            return TranslationResult::bounded(
                batch.candidates,
                full_count,
                request.include_debug_full_count,
            );
        }
        self.bounded_candidates_for_lookup_codes(
            BoundedLookupRequest {
                input,
                lookup_code,
                lookup_specs: &expanded_lookup_codes,
                filter_by_charset,
                segment_tags,
                limit,
                include_full_count: request.include_debug_full_count,
            },
            None,
        )
    }

    fn translated_candidates_for_segment_with_request_and_scratch(
        &self,
        input: &str,
        filter_by_charset: bool,
        segment_tags: Option<&[String]>,
        request: CandidateRequest,
        scratch: &mut TranslatorScratch,
    ) -> TranslationResult {
        let Some(limit) = request.limit.filter(|limit| *limit > 0) else {
            scratch.clear();
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
            scratch.clear();
            return TranslationResult::complete(Vec::new());
        }

        let Some(lookup_code) = self.lookup_code(input) else {
            scratch.clear();
            return TranslationResult::complete(Vec::new());
        };
        let expanded_lookup_codes = self.expanded_lookup_specs(lookup_code);
        if !self.bounded_request_supported(&expanded_lookup_codes) {
            scratch.clear();
            crate::m37_record_full_list_fallback();
            if !self.bounds_compact_fallback_expansion() {
                return TranslationResult::complete(self.translated_candidates_for_segment(
                    input,
                    filter_by_charset,
                    segment_tags,
                ));
            }
            let batch = self.translated_candidates_for_segment_with_prefix_fallback_limit(
                input,
                filter_by_charset,
                segment_tags,
                Some(limit),
            );
            let full_count = if batch.truncated || batch.candidates.len() > limit {
                batch.candidates.len().saturating_add(1)
            } else {
                batch.candidates.len()
            };
            return TranslationResult::bounded(
                batch.candidates,
                full_count,
                request.include_debug_full_count,
            );
        }
        self.bounded_candidates_for_lookup_codes(
            BoundedLookupRequest {
                input,
                lookup_code,
                lookup_specs: &expanded_lookup_codes,
                filter_by_charset,
                segment_tags,
                limit,
                include_full_count: request.include_debug_full_count,
            },
            Some(scratch),
        )
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
                let mut shadow_score = paths[end_pos].as_ref().map(|existing| SentencePathScore {
                    fuzzy_pieces: existing.fuzzy_pieces,
                    quality: existing.quality,
                    raw_quality: existing.raw_quality,
                });
                let exact_start = crate::m37_metrics_enabled().then(Instant::now);
                let sentence_specs = self.sentence_lookup_specs(entry_code);
                let mut winning_candidate = None;
                let mut entry_matches_examined = 0usize;
                let mut exact_candidates = 0usize;
                let predecessor_score = SentencePathScore {
                    fuzzy_pieces: path.fuzzy_pieces,
                    quality: path.quality,
                    raw_quality: path.raw_quality,
                };
                'specs: for spec in &sentence_specs {
                    for candidate in self.storage.exact_candidates(&spec.code) {
                        if !self.is_dictionary_text_allowed(candidate.text())
                            || (filter_by_charset && contains_extended_cjk(candidate.text()))
                        {
                            continue;
                        }
                        exact_candidates += 1;
                        entry_matches_examined += 1;
                        retain_winning_sentence_path_candidate(
                            &mut winning_candidate,
                            &mut shadow_score,
                            predecessor_score,
                            candidate,
                            entry_code,
                            self.sentence_word_penalty,
                        );
                        if entry_matches_examined >= max_candidates_per_span {
                            break 'specs;
                        }
                    }
                }
                if let Some(start) = exact_start {
                    sentence_metrics.exact_lookup_calls += 1;
                    sentence_metrics.exact_lookup_ns += start.elapsed();
                    sentence_metrics.exact_lookup_candidates += exact_candidates;
                }
                if is_final_segment && self.enable_completion && !entry_code.is_empty() {
                    let prefix_start = crate::m37_metrics_enabled().then(Instant::now);
                    let mut prefix_candidates = 0usize;
                    for entry in self.storage.prefix_candidates(entry_code) {
                        if entry_matches_examined >= max_candidates_per_span {
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
                        entry_matches_examined += 1;
                        retain_winning_sentence_path_candidate(
                            &mut winning_candidate,
                            &mut shadow_score,
                            predecessor_score,
                            candidate,
                            entry_code,
                            self.sentence_word_penalty,
                        );
                    }
                    if let Some(start) = prefix_start {
                        sentence_metrics.prefix_lookup_calls += 1;
                        sentence_metrics.prefix_lookup_ns += start.elapsed();
                        sentence_metrics.prefix_lookup_candidates += prefix_candidates;
                    }
                }
                sentence_metrics.entry_matches_collected +=
                    usize::from(winning_candidate.is_some());
                let Some(candidate) = winning_candidate else {
                    continue;
                };
                if is_final_segment && path.pieces.is_empty() {
                    continue;
                }
                let winning_score = shadow_score.expect("winning candidate installs a score");
                let mut next_path = path.clone();
                sentence_metrics.path_clones += 1;
                next_path.fuzzy_pieces = winning_score.fuzzy_pieces;
                next_path.quality = winning_score.quality;
                next_path.raw_quality = winning_score.raw_quality;
                next_path.pieces.push(candidate.text().to_owned());
                let replacing_empty = paths[end_pos].is_none();
                paths[end_pos] = Some(next_path);
                sentence_metrics.path_replacements += 1;
                if replacing_empty {
                    live_paths += 1;
                    max_live_paths = max_live_paths.max(live_paths);
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

fn spelling_correction_entries(
    entries: &[ExpandedSpellingEntry],
) -> HashSet<(String, String, String)> {
    entries
        .iter()
        .filter(|entry| entry.correction)
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

fn prefix_fallback_insert_index(candidates: &[Candidate], lookup_code: &str) -> usize {
    candidates
        .iter()
        .position(|candidate| {
            if candidate.source != CandidateSource::Table {
                return false;
            }
            let source_code = normalized_original_code(&candidate.comment);
            !source_code.is_empty()
                && (!lookup_code.starts_with(&source_code)
                    || source_code_syllable_count(&candidate.comment)
                        .is_some_and(|count| count <= 1))
        })
        .unwrap_or(candidates.len())
}

fn merge_prefix_fallback_candidates(
    candidates: &mut Vec<Candidate>,
    mut prefix_candidates: Vec<Candidate>,
    lookup_code: &str,
) -> usize {
    // Snapshot exact positions before promoting duplicate span metadata.  The
    // eager pool historically allowed completions to interleave with later
    // prism-alias exacts, so neither duplicate detection nor the insertion
    // point may assume the exact family is a contiguous head.
    let exact_positions = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            (candidate.source == CandidateSource::Table).then_some(index)
        })
        .collect::<Vec<_>>();
    let insert_at = exact_positions.last().map_or_else(
        || prefix_fallback_insert_index(candidates, lookup_code),
        |index| index + 1,
    );

    prefix_candidates.retain(|prefix_candidate| {
        if let Some(exact_index) = exact_positions
            .iter()
            .copied()
            .find(|index| candidates[*index].text == prefix_candidate.text)
        {
            // Preserve the exact row's order, display payload, and quality, but
            // selection must consume the longest deployed surface represented
            // by the matching fallback view.
            candidates[exact_index].source = prefix_candidate.source.clone();
            return false;
        }
        !candidates
            .iter()
            .any(|candidate| candidate.text == prefix_candidate.text)
    });

    let inserted = prefix_candidates.len();
    candidates.splice(insert_at..insert_at, prefix_candidates);
    inserted
}

fn leading_single_insert_index(candidates: &[Candidate]) -> usize {
    candidates
        .iter()
        .position(|candidate| {
            candidate.source == CandidateSource::Table && candidate.text.chars().count() == 1
        })
        .unwrap_or(candidates.len())
}

/// Whether `lookup_code` has a UTF-8 boundary strictly between its start and
/// end. A one-scalar lookup has no proper leading prefix even when that scalar
/// occupies multiple bytes.
fn has_proper_leading_prefix(lookup_code: &str) -> bool {
    lookup_code.char_indices().nth(1).is_some()
}

#[cfg(test)]
mod lookup_guard_tests {
    use super::{has_proper_leading_prefix, StaticTableTranslator};
    use crate::{TableDictionary, TableEntry};

    #[test]
    fn proper_leading_prefix_uses_unicode_scalar_boundaries() {
        assert!(!has_proper_leading_prefix(""));
        assert!(!has_proper_leading_prefix("n"));
        assert!(!has_proper_leading_prefix("你"));

        assert!(has_proper_leading_prefix("ni"));
        assert!(has_proper_leading_prefix("你a"));
        assert!(has_proper_leading_prefix("e\u{301}"));
    }

    #[test]
    fn dynamic_correction_keeps_the_default_off_m_prefix_truth_table() {
        let m_prefix = StaticTableTranslator::new([("mgoi", "甲"), ("mgo", "乙")])
            .with_dynamic_correction_lookup(true);
        let m_specs = m_prefix.expanded_lookup_specs("mgoi");
        assert_eq!(
            m_specs
                .iter()
                .map(|spec| (spec.code.as_str(), spec.correction_distance))
                .collect::<Vec<_>>(),
            [("mgoi", None), ("mgo", Some(2))]
        );

        let missing_m_anchor =
            StaticTableTranslator::new([("mgo", "乙")]).with_dynamic_correction_lookup(true);
        assert_eq!(
            missing_m_anchor
                .expanded_lookup_specs("mgoi")
                .iter()
                .map(|spec| (spec.code.as_str(), spec.correction_distance))
                .collect::<Vec<_>>(),
            [("mgoi", None)]
        );

        let non_m = StaticTableTranslator::new([("ngoi", "甲"), ("ngo", "乙")])
            .with_dynamic_correction_lookup(true);
        assert_eq!(
            non_m
                .expanded_lookup_specs("ngoi")
                .iter()
                .map(|spec| (spec.code.as_str(), spec.correction_distance))
                .collect::<Vec<_>>(),
            [("ngoi", None)]
        );

        let enabled = non_m.with_correction(true);
        assert_eq!(
            enabled
                .expanded_lookup_specs("ngoi")
                .iter()
                .map(|spec| (spec.code.as_str(), spec.correction_distance))
                .collect::<Vec<_>>(),
            [("ngoi", None), ("ngo", Some(2))]
        );
    }

    #[test]
    fn empty_or_invalid_replacement_clears_abbreviation_metadata_and_sentence_guard() {
        let formulas = ["abbrev/^hao$/h/".to_owned()];
        let translator =
            StaticTableTranslator::new([("hao", "\u{597d}")]).with_spelling_algebra(&formulas);
        assert!(translator.single_letter_sentence_guard_enabled);
        assert!(!translator.spelling_abbreviation_entries.is_empty());

        let empty = translator.with_spelling_algebra(&[]);
        assert!(!empty.single_letter_sentence_guard_enabled);
        assert!(empty.spelling_abbreviation_entries.is_empty());

        let invalid = empty.with_spelling_algebra(&["not-a-formula".to_owned()]);
        assert!(!invalid.single_letter_sentence_guard_enabled);
        assert!(invalid.spelling_abbreviation_entries.is_empty());
    }

    #[test]
    fn phrase_prefix_seed_does_not_widen_the_leading_single_surface_bound() {
        let long_phrase_code = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwx";
        let dictionary = TableDictionary::new([
            TableEntry::new("hao", "\u{597d}", 100.0),
            TableEntry::new(long_phrase_code, "\u{9577}\u{8a5e}", 90.0),
        ]);
        let formulas = ["derive/^hao$/hx/".to_owned()];
        let translator = StaticTableTranslator::from_dictionary(dictionary)
            .with_prefix_fallback(true)
            .with_leading_syllable_reachability(true)
            .with_spelling_algebra(&formulas)
            .with_sentence(false);

        assert!(
            translator
                .leading_fetch_index()
                .contains_key(long_phrase_code),
            "heap/source prefix fallback must retain the transformed phrase inventory"
        );
        assert_eq!(
            translator.max_leading_prefix_len(),
            3,
            "the long phrase surface must not widen the hao/hx leading-single bound"
        );
        assert!(
            translator
                .leading_prefix_fetch_groups(&format!("hx{long_phrase_code}"))
                .iter()
                .all(|(end, _)| *end <= 3),
            "the actual leading-prefix walk must stay inside the single-character surface bound"
        );

        let phrase_only =
            StaticTableTranslator::from_dictionary(TableDictionary::new([TableEntry::new(
                long_phrase_code,
                "\u{9577}\u{8a5e}",
                90.0,
            )]))
            .with_prefix_fallback(true)
            .with_leading_syllable_reachability(true)
            .with_spelling_algebra(&formulas)
            .with_sentence(false);
        assert!(
            phrase_only
                .leading_prefix_fetch_groups(long_phrase_code)
                .is_empty(),
            "a phrase-only algebra seed has no leading-single boundary to scan"
        );
    }
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

/// M59 finding #6: a dictionary is "untoned" (luna-style) when its syllable
/// codes carry no ASCII tone digit; toned dictionaries (jyutping `bei2`, `zi1`)
/// put a digit in every syllable. One digit proves toned, so the scan
/// early-exits on the first toned code. Prefers the deduped `syllabary_codes`
/// (the exact set the candidate `raw_comment` draws from) and falls back to
/// `all_codes` for the Heap constructors where the syllabary is absent. An empty
/// code inventory classifies as toned/false, but such a dictionary yields no
/// single-char families so the default is inert.
fn dictionary_is_untoned(storage: &TableStorage) -> bool {
    fn has_tone_digit(code: &str) -> bool {
        code.bytes().any(|byte| byte.is_ascii_digit())
    }
    match storage.syllabary_codes() {
        Some(codes) if !codes.is_empty() => !codes.iter().any(|code| has_tone_digit(code)),
        _ => {
            let mut saw_any = false;
            for code in storage.all_codes() {
                saw_any = true;
                if has_tone_digit(&code) {
                    return false;
                }
            }
            saw_any
        }
    }
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

fn canonical_fetch_group(raw_code: &str) -> Cow<'_, str> {
    let code = typeduck_rich_comment_code(raw_code).unwrap_or(raw_code);
    if code.chars().any(char::is_whitespace) {
        Cow::Owned(normalize_table_code(code))
    } else {
        Cow::Borrowed(code)
    }
}

fn leading_candidate_matches_fetch(
    candidate: &LookupCandidate<'_>,
    fetch: &LeadingFetchCode,
) -> bool {
    canonical_fetch_group(candidate.raw_comment()).as_ref() == fetch.canonical_code
}

fn intern_fetch_group(groups: &mut HashMap<String, usize>, raw_code: &str) -> usize {
    let canonical = canonical_fetch_group(raw_code);
    if let Some(index) = groups.get(canonical.as_ref()) {
        *index
    } else {
        let index = groups.len();
        groups.insert(canonical.into_owned(), index);
        index
    }
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

    fn uses_translate_scratch(&self) -> bool {
        self.upstream_sentence_model.is_some()
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

    fn translate_with_context_and_request_with_scratch(
        &self,
        input: &str,
        _status: &Status,
        options: &HashMap<String, bool>,
        context: &Context,
        request: CandidateRequest,
        scratch: &mut TranslatorScratch,
    ) -> TranslationResult {
        let filter_by_charset = (self.enable_charset_filter
            && !options.get("extended_charset").copied().unwrap_or(false))
            || request.filter_extended_cjk;
        self.translated_candidates_for_segment_with_request_and_scratch(
            input,
            filter_by_charset,
            Some(&context.segment_tags),
            request,
            scratch,
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
        let seed_bytes = self
            .leading_fetch_index_seed
            .as_ref()
            .map_or(0, estimate_leading_fetch_seed_bytes);
        let seed_items = self
            .leading_fetch_index_seed
            .as_ref()
            .map_or(0, |seed| seed.canonical_codes.len());
        rows.push(MemoryOwnerRow::new(
            "translator.leading_fetch_seed",
            MemoryOwnerClass::HeapOwnedGuarded,
            seed_bytes,
            seed_items,
            "Vec<String>",
            "source-ordered canonical groups retained for lazy algebra-backed reachability: single-character groups for leading-only use, or every heap/source group when prefix fallback has no current prism; no-algebra paths probe directly",
        ));
        rows.push(MemoryOwnerRow::new(
            "translator.spelling_correction_entries",
            MemoryOwnerClass::HeapOwnedGuarded,
            estimate_string_triple_hash_set_bytes(&self.spelling_correction_entries),
            self.spelling_correction_entries.len(),
            "HashSet<(String, String, String)>",
            "heap/source correction provenance retained per expanded candidate row",
        ));
        rows.push(MemoryOwnerRow::new(
            "translator.spelling_abbreviation_entries",
            MemoryOwnerClass::HeapOwnedGuarded,
            estimate_string_triple_hash_set_bytes(&self.spelling_abbreviation_entries),
            self.spelling_abbreviation_entries.len(),
            "HashSet<(String, String, String)>",
            "heap/source abbreviation provenance retained per expanded candidate row",
        ));
        rows.push(MemoryOwnerRow::new(
            "translator.spelling_correction_surfaces",
            MemoryOwnerClass::HeapOwnedGuarded,
            estimate_string_hash_set_bytes(&self.spelling_correction_surfaces),
            self.spelling_correction_surfaces.len(),
            "HashSet<String>",
            "heap/source correction surfaces retained for exact and prefix guards",
        ));
        let (index_bytes, index_items) =
            self.leading_fetch_index_cache
                .get()
                .map_or((0, 0), |index| {
                    (
                        estimate_leading_fetch_index_bytes(index),
                        index.values().map(Vec::len).sum(),
                    )
                });
        rows.push(MemoryOwnerRow::new(
            "translator.leading_fetch_index",
            MemoryOwnerClass::HeapOwnedGuarded,
            index_bytes,
            index_items,
            "OnceLock<HashMap<String, Vec<LeadingFetchCode>>>",
            "algebra surface spelling to canonical fetch edges, allocated lazily on first indexed reachability lookup; no-algebra paths remain direct",
        ));
        let cache = self
            .prefix_fallback_window_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        let (cache_bytes, cache_items) = cache.as_ref().map_or((0, 0), |entry| {
            (
                estimate_prefix_fallback_window_cache_bytes(entry),
                entry.rows.len(),
            )
        });
        rows.push(MemoryOwnerRow::new(
            "translator.prefix_fallback_window_cache",
            MemoryOwnerClass::HeapOwnedGuarded,
            cache_bytes,
            cache_items,
            "Mutex<Option<Arc<PrefixFallbackWindowCacheEntry>>>",
            "single bounded raw prefix window reused across incremental keystrokes; per-call span promotion, ordering, and deduplication remain live",
        ));
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
