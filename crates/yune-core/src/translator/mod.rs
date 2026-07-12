use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque};
use std::mem;
use std::ops::ControlFlow;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::comment_format::CommentFormat;
use crate::dictionary::{
    normalize_table_code, CompactTableStore, LookupCandidate, LookupCandidateEntry,
    RimePrismBinPayload, RimePrismRuntimePayload, TableEntryWeightDomain, TableLookup,
    LIBRIME_NON_POSITIVE_COMPILED_LOG_WEIGHT_BITS,
};
use crate::filter::contains_extended_cjk;
use crate::poet::{
    upstream_script_raw_candidate_quality, GrammarProvider, PoetByteSource,
    RankedScriptPhraseCandidate, SentenceCodeSpan, UpstreamSentenceModel, WeightedSentenceCodeSpan,
};
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
const BOUNDED_UPSTREAM_SCRIPT_CACHE_CAPACITY: usize = 64;
const BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_LIMIT: usize = 64;
const BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_INPUT_BYTES: usize = 256;
const BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_ENTRY_BYTES: usize = 128 * 1024;
const BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_TOTAL_BYTES: usize = 512 * 1024;
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
const PREFIX_FALLBACK_PROFILE_LONG_PENDING_MULTIPLIER: usize = 2;
const PREFIX_FALLBACK_CACHE_MAX_ROWS: usize = 128;
const PREFIX_FALLBACK_CACHE_MAX_PREFIXES: usize = 64;
const PREFIX_FALLBACK_CACHE_MAX_KEY_BYTES: usize = 32 * 1024;
const PREFIX_FALLBACK_CACHE_MAX_ROW_BYTES: usize = 16 * 1024;
const PREFIX_FALLBACK_CACHE_MAX_ENTRY_BYTES: usize = 512 * 1024;
/// Yune-internal heuristic calibrated to the M21 TypeDuck v1.1.2 sentence-composition fixture
/// and the M28 follow-up upstream-Jyutping composition fixture; install only for the
/// jyut6ping3 TypeDuck profile.
pub const TYPEDUCK_SENTENCE_WORD_PENALTY: f32 = 24.0;

/// Selects how a dictionary translator participates in script-translation
/// sentence ordering. The legacy mode preserves the historical fallback-only
/// behavior. `UpstreamScript` makes the deployed surface graph authoritative
/// for a Standard, toned/transformed `script_translator` with
/// `enable_sentence: true`. Untoned identity dictionaries retain the legacy
/// fast path until M59 4e owns their complete merge/order semantics.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SentencePolicy {
    #[default]
    LegacyFallback,
    UpstreamScript,
}

#[derive(Clone, Debug, PartialEq)]
struct LookupCodeSpec {
    code: String,
    lookup_code: String,
    correction_distance: Option<usize>,
    required_syllable_count: Option<usize>,
    tolerance: bool,
    spelling_correction: bool,
    spelling_abbreviation: bool,
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
            spelling_abbreviation: false,
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
            spelling_abbreviation: false,
            spelling_credibility: 0.0,
        }
    }

    fn alias(
        code: impl Into<String>,
        lookup_code: impl Into<String>,
        spelling_abbreviation: bool,
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
            spelling_abbreviation,
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
            spelling_abbreviation: false,
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
            spelling_abbreviation: false,
            spelling_credibility: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
struct SurfaceCodeChoice {
    code: String,
    syllable_id: usize,
    normal: bool,
    abbreviation: bool,
    credibility: f32,
}

#[derive(Clone, Debug, Default)]
struct SurfaceCodeChoices {
    choices: Vec<SurfaceCodeChoice>,
    indexes: HashMap<String, usize>,
}

impl SurfaceCodeChoices {
    fn merge(&mut self, choice: SurfaceCodeChoice) {
        if let Some(index) = self.indexes.get(&choice.code).copied() {
            let existing = &mut self.choices[index];
            // Mirrors deployed `SpellingProperties::Update`: a normal/fuzzy
            // route outranks an abbreviation to the same canonical code. Keep
            // the first traversal position for equal provenance.
            if !choice.abbreviation {
                existing.abbreviation = false;
            }
            existing.credibility = existing.credibility.max(choice.credibility);
            existing.normal |= choice.normal;
            existing.syllable_id = existing.syllable_id.min(choice.syllable_id);
            return;
        }
        self.indexes.insert(choice.code.clone(), self.choices.len());
        self.choices.push(choice);
    }
}

#[derive(Clone, Debug)]
struct SurfaceSyllable {
    start: usize,
    end: usize,
    choices: Vec<SurfaceCodeChoice>,
}

#[derive(Clone, Debug)]
struct ConcatenatedSurfaceCode {
    code: String,
    abbreviation: bool,
    credibility: f32,
}

struct UpstreamScriptDirectChunk<'a> {
    code: &'a ConcatenatedSurfaceCode,
    rows: Box<dyn Iterator<Item = LookupCandidate<'a>> + 'a>,
    head: Option<(Candidate, f64)>,
}

#[derive(Clone, Debug)]
struct UpstreamScriptSurfaceGraph {
    boundaries: Vec<usize>,
    edges_by_start: Vec<Vec<SurfaceSyllable>>,
    direct_edges_by_start: Vec<Vec<SurfaceSyllable>>,
    leading_prefix_syllables: Vec<SurfaceSyllable>,
    one_syllable_prefix_ends: Vec<usize>,
    interpreted_end: usize,
}

impl UpstreamScriptSurfaceGraph {
    fn leading_syllables(&self) -> &[SurfaceSyllable] {
        &self.leading_prefix_syllables
    }

    fn is_one_syllable_prefix_end(&self, end: usize) -> bool {
        self.one_syllable_prefix_ends.binary_search(&end).is_ok()
    }

    fn preedit(&self, input: &str, delimiter: char) -> Option<String> {
        let interpreted_index = self.boundaries.binary_search(&self.interpreted_end).ok()?;
        let mut coverable = vec![false; self.boundaries.len()];
        coverable[interpreted_index] = true;
        for start_index in (0..interpreted_index).rev() {
            coverable[start_index] = self.edges_by_start[start_index].iter().any(|syllable| {
                self.boundaries
                    .binary_search(&syllable.end)
                    .is_ok_and(|end_index| coverable[end_index])
            });
        }
        if !coverable[0] {
            return None;
        }

        let mut pieces = Vec::new();
        let mut start_index = 0usize;
        while start_index < interpreted_index {
            let syllable = self.edges_by_start[start_index]
                .iter()
                .filter(|syllable| {
                    self.boundaries
                        .binary_search(&syllable.end)
                        .is_ok_and(|end_index| coverable[end_index])
                })
                .max_by_key(|syllable| syllable.end)?;
            pieces.push(input[syllable.start..syllable.end].to_owned());
            start_index = self.boundaries.binary_search(&syllable.end).ok()?;
        }
        let mut preedit = pieces.join(&delimiter.to_string());
        preedit.push_str(&input[self.interpreted_end..]);
        Some(preedit)
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
    fn owns_full_input_span(&self) -> bool {
        self.entry_code == self.lookup_code || self.limited_prediction
    }

    fn comparison_weight(&self, weight_domain: TableEntryWeightDomain) -> f32 {
        let mut quality = table_comparison_weight(self.candidate.quality, weight_domain)
            + self.spelling_credibility;
        if let Some(distance) = self.correction_distance {
            quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        quality
    }

    fn prediction_comparison_weight(&self, weight_domain: TableEntryWeightDomain) -> f32 {
        self.comparison_weight(weight_domain)
    }

    fn prediction_precedes(&self, ordinary: &Self, weight_domain: TableEntryWeightDomain) -> bool {
        let interpreted =
            complete_syllable_prefix_count(&self.candidate.comment, &self.lookup_code);
        let consumed = source_code_syllable_count(&ordinary.candidate.comment);
        self.prediction_comparison_weight(weight_domain)
            > ordinary.prediction_comparison_weight(weight_domain)
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

impl PendingLookupCandidateRef<'_> {
    fn owns_full_input_span(&self) -> bool {
        self.entry_code.as_ref() == self.lookup_code || self.limited_prediction
    }
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

#[derive(Clone, Copy)]
struct BoundedLookupRequest<'a> {
    input: &'a str,
    lookup_code: &'a str,
    lookup_specs: &'a [LookupCodeSpec],
    filter_by_charset: bool,
    segment_tags: Option<&'a [String]>,
    limit: usize,
    include_full_count: bool,
}

struct LegacyBoundedSentenceFallbackRequest<'a> {
    model: &'a UpstreamSentenceModel,
    input: &'a str,
    lookup_code: &'a str,
    filter_by_charset: bool,
    limit: usize,
    include_full_count: bool,
    has_correction_lookup: bool,
    scratch: Option<&'a mut TranslatorScratch>,
}

struct BoundedPrefixFallbackCacheRequest<'input, 'request> {
    input: &'input str,
    lookup_code: &'input str,
    filter_by_charset: bool,
    existing_candidates: &'request [Candidate],
    admitted_span_candidates: &'request [PrefixFallbackSpanView],
    prefixes: &'request [LookupPrefixSpec<'input>],
    limit: usize,
    fallback_start: Option<Instant>,
}

struct BoundedPrefixFallbackStreamingRequest<'input, 'request> {
    input: &'input str,
    lookup_code: &'input str,
    filter_by_charset: bool,
    existing_candidates: &'request [Candidate],
    admitted_span_candidates: &'request [PrefixFallbackSpanView],
    limit: usize,
}

#[derive(Clone)]
struct LookupPrefixSpec<'a> {
    input_prefix: &'a str,
    fetch_code: String,
    consumed_lookup_len: usize,
    surface_fetch: Option<LeadingFetchCode>,
}

struct BoundedPrefixStreamRow {
    fetch_code: String,
    fetch_order: usize,
    candidate_index: usize,
    text: String,
    consumed_lookup_len: usize,
    consumed_input_len: usize,
    recompose_on_default: bool,
    deferred_surface_phrase: bool,
    raw_quality: f32,
}

struct BoundedPrefixStreamChunk<'input> {
    prefix: LookupPrefixSpec<'input>,
    fetch_order: usize,
    next_candidate_index: usize,
    head: Option<BoundedPrefixStreamRow>,
}

fn bounded_prefix_stream_head_strictly_precedes(
    candidate: &BoundedPrefixStreamRow,
    current: &BoundedPrefixStreamRow,
) -> bool {
    match candidate
        .deferred_surface_phrase
        .cmp(&current.deferred_surface_phrase)
        .then_with(|| {
            current
                .consumed_input_len
                .cmp(&candidate.consumed_input_len)
        }) {
        Ordering::Less => true,
        Ordering::Greater => false,
        Ordering::Equal => candidate.raw_quality > current.raw_quality,
    }
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
}

#[derive(Clone, Debug)]
struct PrefixFallbackWindowCacheEntry {
    key: PrefixFallbackWindowCacheKey,
    rows: Vec<CachedPrefixFallbackView>,
    truncated: bool,
}

#[derive(Clone, Debug)]
struct PrefixFallbackBatch {
    candidates: Vec<Candidate>,
    truncated: bool,
    owns_reachability: bool,
    span_promotions: HashMap<String, CandidateSource>,
}

#[derive(Clone, Debug)]
struct PrefixFallbackSpanView {
    text: String,
    raw_comment: String,
    spelling_abbreviation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BoundedUpstreamScriptCacheKey {
    input: String,
    filter_by_charset: bool,
    limit: usize,
    include_full_count: bool,
}

#[derive(Clone, Debug)]
struct BoundedUpstreamScriptCacheEntry {
    key: BoundedUpstreamScriptCacheKey,
    result: TranslationResult,
}

#[derive(Debug, Default)]
struct BoundedUpstreamScriptCache {
    entries: VecDeque<BoundedUpstreamScriptCacheEntry>,
}

impl BoundedUpstreamScriptCache {
    fn get(
        &mut self,
        input: &str,
        filter_by_charset: bool,
        limit: usize,
        include_full_count: bool,
    ) -> Option<TranslationResult> {
        let index = self.entries.iter().position(|entry| {
            entry.key.input == input
                && entry.key.filter_by_charset == filter_by_charset
                && entry.key.limit == limit
                && entry.key.include_full_count == include_full_count
        })?;
        let entry = self
            .entries
            .remove(index)
            .expect("located bounded upstream-script cache entry should exist");
        let result = entry.result.clone();
        self.entries.push_back(entry);
        Some(result)
    }

    fn insert(
        &mut self,
        input: &str,
        filter_by_charset: bool,
        limit: usize,
        include_full_count: bool,
        result: TranslationResult,
    ) {
        if input.len() > BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_INPUT_BYTES
            || limit > BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_LIMIT
            || result.candidates.len() > BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_LIMIT
            || result.is_complete
        {
            return;
        }
        let entry_bytes = bounded_upstream_script_cache_entry_bytes(input, &result);
        if entry_bytes > BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_ENTRY_BYTES {
            return;
        }
        if let Some(index) = self.entries.iter().position(|entry| {
            entry.key.input == input
                && entry.key.filter_by_charset == filter_by_charset
                && entry.key.limit == limit
                && entry.key.include_full_count == include_full_count
        }) {
            self.entries.remove(index);
        }
        while self.entries.len() >= BOUNDED_UPSTREAM_SCRIPT_CACHE_CAPACITY
            || self.entry_payload_bytes().saturating_add(entry_bytes)
                > BOUNDED_UPSTREAM_SCRIPT_CACHE_MAX_TOTAL_BYTES
        {
            if self.entries.pop_front().is_none() {
                break;
            }
        }
        self.entries.push_back(BoundedUpstreamScriptCacheEntry {
            key: BoundedUpstreamScriptCacheKey {
                input: input.to_owned(),
                filter_by_charset,
                limit,
                include_full_count,
            },
            result,
        });
    }

    fn entry_payload_bytes(&self) -> usize {
        self.entries
            .iter()
            .map(|entry| bounded_upstream_script_cache_entry_bytes(&entry.key.input, &entry.result))
            .sum()
    }

    fn estimated_retained_bytes(&self) -> usize {
        mem::size_of::<Self>()
            .saturating_add(
                self.entries
                    .capacity()
                    .saturating_mul(mem::size_of::<BoundedUpstreamScriptCacheEntry>()),
            )
            .saturating_add(self.entries.iter().fold(0usize, |bytes, entry| {
                bytes
                    .saturating_add(entry.key.input.capacity())
                    .saturating_add(
                        entry
                            .result
                            .candidates
                            .capacity()
                            .saturating_mul(mem::size_of::<Candidate>()),
                    )
                    .saturating_add(entry.result.candidates.iter().fold(
                        0usize,
                        |candidate_bytes, candidate| {
                            candidate_bytes
                                .saturating_add(candidate.text.capacity())
                                .saturating_add(candidate.comment.capacity())
                                .saturating_add(
                                    candidate.preedit.as_ref().map_or(0, String::capacity),
                                )
                        },
                    ))
            }))
    }
}

fn bounded_upstream_script_cache_entry_bytes(input: &str, result: &TranslationResult) -> usize {
    mem::size_of::<BoundedUpstreamScriptCacheEntry>()
        .saturating_add(input.len())
        .saturating_add(
            result
                .candidates
                .capacity()
                .saturating_mul(mem::size_of::<Candidate>()),
        )
        .saturating_add(
            result
                .candidates
                .iter()
                .map(estimate_candidate_bytes)
                .sum::<usize>(),
        )
}

fn order_current_head_chunks<T>(
    candidates: &mut Vec<T>,
    output_limit: Option<usize>,
    mut chunk_key: impl FnMut(&T) -> usize,
    mut strictly_precedes: impl FnMut(&T, &T) -> bool,
) {
    let mut chunk_indices: HashMap<usize, usize> = HashMap::new();
    let mut chunks: Vec<VecDeque<T>> = Vec::new();
    for candidate in std::mem::take(candidates) {
        let key = chunk_key(&candidate);
        if let Some(index) = chunk_indices.get(&key).copied() {
            chunks[index].push_back(candidate);
        } else {
            chunk_indices.insert(key, chunks.len());
            chunks.push(VecDeque::from([candidate]));
        }
    }

    let total = chunks.iter().map(VecDeque::len).sum::<usize>();
    let mut ordered = Vec::with_capacity(output_limit.map_or(total, |limit| limit.min(total)));
    let mut active = 0usize;
    while active < chunks.len() {
        for visitor in active + 1..chunks.len() {
            if chunks[visitor]
                .front()
                .zip(chunks[active].front())
                .is_some_and(|(candidate, current)| strictly_precedes(candidate, current))
            {
                // librime's DictEntryIterator repeatedly performs a one-item
                // partial_sort. Swap for every strict visitor so the residual
                // chunk permutation, including later equal-head ties, matches
                // that tournament rather than a conventional stable heap.
                chunks.swap(active, visitor);
            }
        }
        ordered.push(
            chunks[active]
                .pop_front()
                .expect("active prefix chunk should be nonempty"),
        );
        if chunks[active].is_empty() {
            active += 1;
        }
        if output_limit.is_some_and(|limit| ordered.len() >= limit) {
            break;
        }
    }
    *candidates = ordered;
}

fn retain_profile_bounded_prefix_row<T>(
    rows: &mut Vec<T>,
    row: T,
    cap: usize,
    deferred_row_count: &mut usize,
    is_deferred: impl Fn(&T) -> bool,
) -> bool {
    let row_is_deferred = is_deferred(&row);
    if rows.len() < cap {
        *deferred_row_count += usize::from(row_is_deferred);
        rows.push(row);
    } else if !row_is_deferred && *deferred_row_count > 0 {
        if let Some(index) = rows.iter().rposition(&is_deferred) {
            rows.remove(index);
            *deferred_row_count -= 1;
            rows.push(row);
        }
    }
    rows.len() >= cap && *deferred_row_count == 0
}

struct LookupCandidateBatch {
    candidates: Vec<Candidate>,
    full_input_anchor: Option<usize>,
    has_reliable_exact_system_phrase: bool,
    prefix_fallback_span_views: Vec<PrefixFallbackSpanView>,
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

fn table_log_weight(weight: f32, domain: TableEntryWeightDomain) -> f32 {
    match domain {
        // Positive source dictionary weights are absolute weights and enter
        // ranking in log space. Preserve the long-standing zero/negative
        // source-score convention used by lightweight and test dictionaries:
        // zero is the neutral log score (quality 1), while a negative value is
        // already a penalty. Compiled tables encode non-positive source rows
        // explicitly and take the NaturalLog arm below.
        TableEntryWeightDomain::Raw if weight > 0.0 => weight.ln(),
        TableEntryWeightDomain::Raw => weight,
        TableEntryWeightDomain::NaturalLog => weight,
    }
}

fn table_comparison_weight(weight: f32, domain: TableEntryWeightDomain) -> f32 {
    match domain {
        TableEntryWeightDomain::NaturalLog => weight,
        TableEntryWeightDomain::Raw if weight > 0.0 => weight.ln(),
        // Raw zero is an unweighted dictionary row and must remain below a
        // positive weight of one. Offset non-positive source penalties from
        // the smallest positive log so their relative credibility still
        // participates without tying zero against ln(1).
        TableEntryWeightDomain::Raw => f32::MIN_POSITIVE.ln() + weight,
    }
}

fn table_raw_weight(weight: f32, domain: TableEntryWeightDomain) -> f32 {
    match domain {
        TableEntryWeightDomain::Raw => weight,
        TableEntryWeightDomain::NaturalLog => weight.exp(),
    }
}

fn sentence_piece_quality(weight: f32, word_penalty: f32, domain: TableEntryWeightDomain) -> f32 {
    table_log_weight(weight, domain).max(0.0) - word_penalty
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
    weight_domain: TableEntryWeightDomain,
) {
    let candidate_score = SentencePathScore {
        fuzzy_pieces: predecessor.fuzzy_pieces
            + usize::from(!raw_sentence_piece_matches_input_code(
                candidate.raw_comment(),
                candidate.text(),
                entry_code,
            )),
        quality: predecessor.quality
            + sentence_piece_quality(candidate.raw_quality(), word_penalty, weight_domain),
        raw_quality: predecessor.raw_quality
            + table_raw_weight(candidate.raw_quality(), weight_domain),
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

    fn has_strict_code_prefix(&self, prefix: &str) -> bool {
        match self {
            Self::Heap(entries) => entries
                .range(prefix.to_owned()..)
                .map(|(code, _)| code)
                .take_while(|code| code.starts_with(prefix))
                .any(|code| code.len() > prefix.len()),
            Self::Compact(store) => store.has_strict_code_prefix(prefix),
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

fn estimate_source_entry_build_bytes(entries: &[(String, Candidate)]) -> usize {
    mem::size_of_val(entries).saturating_add(entries.iter().fold(0usize, |bytes, (code, row)| {
        bytes
            .saturating_add(code.capacity())
            .saturating_add(estimate_candidate_bytes(row))
    }))
}

fn estimate_preset_vocabulary_build_bytes(entries: &[PresetVocabularyEntry]) -> usize {
    mem::size_of_val(entries).saturating_add(
        entries
            .iter()
            .map(|entry| entry.text.capacity())
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
    entry_weight_domain: TableEntryWeightDomain,
    spelling_abbreviation_entries: HashSet<(String, String, String)>,
    spelling_correction_entries: HashSet<(String, String, String)>,
    spelling_correction_surfaces: HashSet<String>,
    normal_codes: NormalCodeIndex,
    enable_completion: bool,
    enable_correction: bool,
    dynamic_correction_lookup: bool,
    enable_charset_filter: bool,
    enable_sentence: bool,
    sentence_policy: SentencePolicy,
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
    // Fixed-capacity immutable page prefixes for Standard ScriptTranslation.
    // Complete lists remain on-demand and are never retained here.
    bounded_upstream_script_cache: Mutex<BoundedUpstreamScriptCache>,
    // One bounded raw prefix window is retained across incremental keystrokes.
    // The key contains the resolved deployed-surface fetch graph, so a longer
    // input that keeps the same leading family can reuse immutable raw rows
    // while recomputing span promotion, ordering, and deduplication per call.
    // A single entry bounds ownership independently of typing history.
    prefix_fallback_window_cache: Mutex<Option<Arc<PrefixFallbackWindowCacheEntry>>>,
    sentence_word_penalty: f32,
    spelling_algebra: SpellingAlgebra,
    spelling_algebra_formulas: Vec<String>,
    preset_vocabulary: Vec<PresetVocabularyEntry>,
    abbreviation_preset_vocabulary: Vec<PresetVocabularyEntry>,
    upstream_sentence_grammar: GrammarProvider,
    upstream_sentence_poet_source: Option<(Arc<dyn PoetByteSource>, u32)>,
    upstream_sentence_cutoff_threshold: f64,
    upstream_script_translation_limits: Option<(usize, usize)>,
    // Sentence/abbreviation indexing is the expensive part of installing a
    // Standard script translator. Keep the immutable build inputs on the
    // translator and construct the model on its first translation/warmup.
    // D-32 may then share this initialized translator across sessions; the
    // model is still owned by, and dies with, that fingerprinted translator.
    upstream_sentence_model_max_candidates: Option<usize>,
    upstream_sentence_model: OnceLock<UpstreamSentenceModel>,
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
            // This convenience constructor stores the historical translator
            // score directly. The Raw domain's non-positive convention keeps
            // zero as the neutral score exported as weight 1.
            entry_weight_domain: TableEntryWeightDomain::Raw,
            spelling_abbreviation_entries: HashSet::new(),
            spelling_correction_entries: HashSet::new(),
            spelling_correction_surfaces: HashSet::new(),
            normal_codes,
            enable_completion: false,
            enable_correction: false,
            dynamic_correction_lookup: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_policy: SentencePolicy::LegacyFallback,
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
            bounded_upstream_script_cache: Mutex::new(BoundedUpstreamScriptCache::default()),
            prefix_fallback_window_cache: Mutex::new(None),
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra: SpellingAlgebra::default(),
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary: Vec::new(),
            abbreviation_preset_vocabulary: Vec::new(),
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_poet_source: None,
            upstream_sentence_cutoff_threshold: 0.1,
            upstream_script_translation_limits: None,
            upstream_sentence_model_max_candidates: None,
            upstream_sentence_model: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn from_dictionary(dictionary: TableDictionary) -> Self {
        let sort_by_weight = dictionary.sort_by_weight();
        let entry_weight_domain = dictionary.entry_weight_domain();
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
            entry_weight_domain,
            spelling_abbreviation_entries: HashSet::new(),
            spelling_correction_entries: HashSet::new(),
            spelling_correction_surfaces: HashSet::new(),
            normal_codes,
            enable_completion: false,
            enable_correction: false,
            dynamic_correction_lookup: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_policy: SentencePolicy::LegacyFallback,
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
            bounded_upstream_script_cache: Mutex::new(BoundedUpstreamScriptCache::default()),
            prefix_fallback_window_cache: Mutex::new(None),
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra: SpellingAlgebra::default(),
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_poet_source: None,
            upstream_sentence_cutoff_threshold: 0.1,
            upstream_script_translation_limits: None,
            upstream_sentence_model_max_candidates: None,
            upstream_sentence_model: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn from_compact_dictionary(
        dictionary: TableDictionary,
        prism_payload: Option<RimePrismBinPayload>,
    ) -> Self {
        let direct_prism_surface_mapping_current = prism_payload.is_some();
        let sort_by_weight = dictionary.sort_by_weight();
        let entry_weight_domain = dictionary.entry_weight_domain();
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
            entry_weight_domain,
            spelling_abbreviation_entries: HashSet::new(),
            spelling_correction_entries: HashSet::new(),
            spelling_correction_surfaces: HashSet::new(),
            normal_codes,
            enable_completion: false,
            enable_correction: false,
            dynamic_correction_lookup: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_policy: SentencePolicy::LegacyFallback,
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
            bounded_upstream_script_cache: Mutex::new(BoundedUpstreamScriptCache::default()),
            prefix_fallback_window_cache: Mutex::new(None),
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra: SpellingAlgebra::default(),
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_poet_source: None,
            upstream_sentence_cutoff_threshold: 0.1,
            upstream_script_translation_limits: None,
            upstream_sentence_model_max_candidates: None,
            upstream_sentence_model: OnceLock::new(),
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
        let entry_weight_domain = store.entry_weight_domain();
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
            entry_weight_domain,
            spelling_abbreviation_entries: HashSet::new(),
            spelling_correction_entries: HashSet::new(),
            spelling_correction_surfaces: HashSet::new(),
            normal_codes,
            enable_completion: false,
            enable_correction: false,
            dynamic_correction_lookup: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_policy: SentencePolicy::LegacyFallback,
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
            bounded_upstream_script_cache: Mutex::new(BoundedUpstreamScriptCache::default()),
            prefix_fallback_window_cache: Mutex::new(None),
            sentence_word_penalty: DEFAULT_SENTENCE_WORD_PENALTY,
            spelling_algebra: SpellingAlgebra::default(),
            spelling_algebra_formulas: Vec::new(),
            preset_vocabulary,
            abbreviation_preset_vocabulary,
            upstream_sentence_grammar: GrammarProvider::default(),
            upstream_sentence_poet_source: None,
            upstream_sentence_cutoff_threshold: 0.1,
            upstream_script_translation_limits: None,
            upstream_sentence_model_max_candidates: None,
            upstream_sentence_model: OnceLock::new(),
        }
    }

    fn reset_bounded_translation_caches(&mut self) {
        self.bounded_upstream_script_cache = Mutex::new(BoundedUpstreamScriptCache::default());
        self.prefix_fallback_window_cache = Mutex::new(None);
    }

    #[must_use]
    pub fn with_completion(mut self, enable_completion: bool) -> Self {
        self.enable_completion = enable_completion;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_correction(mut self, enable_correction: bool) -> Self {
        self.enable_correction = enable_correction;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_dynamic_correction_lookup(mut self, dynamic_correction_lookup: bool) -> Self {
        self.dynamic_correction_lookup = dynamic_correction_lookup;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_charset_filter(mut self, enable_charset_filter: bool) -> Self {
        self.enable_charset_filter = enable_charset_filter;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_sentence(mut self, enable_sentence: bool) -> Self {
        self.enable_sentence = enable_sentence;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_sentence_policy(mut self, policy: SentencePolicy) -> Self {
        self.sentence_policy = policy;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_sentence_word_penalty(mut self, sentence_word_penalty: f32) -> Self {
        self.sentence_word_penalty = sentence_word_penalty;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_sentence_over_completion(mut self, sentence_over_completion: bool) -> Self {
        self.sentence_over_completion = sentence_over_completion;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_delimiters(mut self, delimiters: impl Into<String>) -> Self {
        self.delimiters = delimiters.into();
        if self.delimiters.is_empty() {
            self.delimiters = " ".to_owned();
        }
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        if self.tags.is_empty() {
            self.tags.push("abc".to_owned());
        }
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_initial_quality(mut self, initial_quality: f32) -> Self {
        self.initial_quality = initial_quality;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_preedit_format(mut self, formulas: &[String]) -> Self {
        self.preedit_format = CommentFormat::parse(formulas);
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_dictionary_exclude(
        mut self,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.dictionary_exclude = words.into_iter().map(Into::into).collect();
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_combine_candidates(mut self, combine_candidates: bool) -> Self {
        self.combine_candidates = combine_candidates;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_affix(mut self, prefix: impl Into<String>, suffix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self.suffix = suffix.into();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_show_full_code(mut self, show_full_code: bool) -> Self {
        self.show_full_code = show_full_code;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_prediction_weight_threshold(mut self, threshold: f32) -> Self {
        self.prediction_weight_threshold = Some(threshold);
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_prediction_never_first(mut self, prediction_never_first: bool) -> Self {
        self.prediction_never_first = prediction_never_first;
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_prediction_candidate_limit(mut self, limit: usize) -> Self {
        self.prediction_candidate_limit = Some(limit);
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_prefix_fallback(mut self, prefix_fallback: bool) -> Self {
        self.prefix_fallback = prefix_fallback;
        self.reset_bounded_translation_caches();
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
        self.reset_bounded_translation_caches();
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
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_abbreviation_preset_vocabulary(
        mut self,
        vocabulary: impl IntoIterator<Item = PresetVocabularyEntry>,
    ) -> Self {
        self.abbreviation_preset_vocabulary = vocabulary.into_iter().collect();
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_upstream_sentence_poet_source(
        mut self,
        source: Arc<dyn PoetByteSource>,
        dictionary_checksum: u32,
    ) -> Self {
        self.upstream_sentence_poet_source = Some((source, dictionary_checksum));
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_upstream_sentence_model(mut self, max_candidates: usize) -> Self {
        self.upstream_sentence_model_max_candidates = Some(max_candidates.max(1));
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
        self
    }

    fn build_upstream_sentence_model(&self, max_candidates: usize) -> UpstreamSentenceModel {
        let abbreviation_vocabulary = if self.abbreviation_preset_vocabulary.is_empty() {
            self.preset_vocabulary.as_slice()
        } else {
            self.abbreviation_preset_vocabulary.as_slice()
        };
        let build_abbreviation_model = matches!(self.storage, TableStorage::Compact(_))
            && self.prism_payload.is_some()
            && self.single_letter_sentence_guard_enabled
            && !abbreviation_vocabulary.is_empty();
        // Every standard Rime::Table/4.0 stores natural-log weights, regardless
        // of whether its index is Marisa-backed. Source dictionaries and
        // identifiable legacy Yune tables retain raw weights. The explicit
        // domain prevents both double-logging external non-Marisa tables and
        // misreading source/legacy rows as compiled logs.
        let natural_log_table_weights =
            self.entry_weight_domain == TableEntryWeightDomain::NaturalLog;
        let model = if let Some((source, dictionary_checksum)) = &self.upstream_sentence_poet_source
        {
            UpstreamSentenceModel::from_poet_bin_source(
                Arc::clone(source),
                *dictionary_checksum,
                max_candidates,
            )
            .expect("validated poet artifact should load into sentence model")
        } else if let Some(entries) = &self.source_entries {
            let table_entries = entries
                .iter()
                .map(|(code, candidate)| TableEntry::new(code, &candidate.text, candidate.quality))
                .collect::<Vec<_>>();
            if natural_log_table_weights {
                UpstreamSentenceModel::from_natural_log_table_entries(
                    table_entries,
                    &self.preset_vocabulary,
                    max_candidates,
                )
            } else {
                UpstreamSentenceModel::from_table_entries(
                    table_entries,
                    &self.preset_vocabulary,
                    max_candidates,
                )
            }
        } else {
            let full_pinyin_vocabulary = self.preset_vocabulary.as_slice();
            if build_abbreviation_model && natural_log_table_weights {
                UpstreamSentenceModel::from_natural_log_table_entries_with_abbreviation_vocabulary(
                    self.storage.table_entry_iter(),
                    full_pinyin_vocabulary,
                    abbreviation_vocabulary,
                    max_candidates,
                )
            } else if build_abbreviation_model {
                UpstreamSentenceModel::from_table_entries_with_abbreviation_vocabulary(
                    self.storage.table_entry_iter(),
                    full_pinyin_vocabulary,
                    abbreviation_vocabulary,
                    max_candidates,
                )
            } else if natural_log_table_weights {
                UpstreamSentenceModel::from_natural_log_table_entries(
                    self.storage.table_entry_iter(),
                    full_pinyin_vocabulary,
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
        let mut model = model
            .with_grammar(self.upstream_sentence_grammar.clone())
            .with_sentence_cutoff_threshold(self.upstream_sentence_cutoff_threshold)
            .with_excluded_texts(self.dictionary_exclude.iter().cloned());
        if let Some((max_sentences, max_homophones)) = self.upstream_script_translation_limits {
            model = model.with_script_translation_limits(max_sentences, max_homophones);
        }
        model
    }

    fn has_upstream_sentence_model(&self) -> bool {
        self.upstream_sentence_model_max_candidates.is_some()
    }

    fn upstream_sentence_model(&self) -> Option<&UpstreamSentenceModel> {
        let max_candidates = self.upstream_sentence_model_max_candidates?;
        Some(
            self.upstream_sentence_model
                .get_or_init(|| self.build_upstream_sentence_model(max_candidates)),
        )
    }

    fn reset_upstream_sentence_model(&mut self) {
        self.upstream_sentence_model = OnceLock::new();
    }

    #[must_use]
    pub fn with_upstream_sentence_grammar(mut self, grammar: impl Into<GrammarProvider>) -> Self {
        let grammar = grammar.into();
        self.upstream_sentence_grammar = grammar.clone();
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_upstream_script_translation_limits(
        mut self,
        max_sentences: usize,
        max_homophones: usize,
    ) -> Self {
        self.upstream_script_translation_limits = Some((max_sentences, max_homophones));
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_upstream_sentence_cutoff_threshold(mut self, threshold: f64) -> Self {
        self.upstream_sentence_cutoff_threshold = threshold;
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_corrections(
        mut self,
        corrections: impl IntoIterator<Item = RimeCorrectionEntry>,
    ) -> Self {
        self.corrections = corrections.into_iter().collect();
        self.reset_bounded_translation_caches();
        self
    }

    #[must_use]
    pub fn with_tolerance_rules(
        mut self,
        tolerance_rules: impl IntoIterator<Item = RimeToleranceRule>,
    ) -> Self {
        self.tolerance_rules = tolerance_rules.into_iter().collect();
        self.reset_bounded_translation_caches();
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
            && !self.direct_prism_surface_mapping_current
            && (self.leading_syllable_reachability || self.prefix_fallback)
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
        self.spelling_algebra = algebra.clone();
        self.reset_upstream_sentence_model();
        self.reset_bounded_translation_caches();
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
            // A lazy sentence model must retain the same canonical pre-algebra
            // recipe the former eager builder consumed at this point. Derived,
            // abbreviated, and correction surfaces belong only to the outer
            // table lookup; rebuilding Poet from expanded storage would promote
            // those aliases into genuine sentence phrases. Schema installation
            // configures the UpstreamScript owner before algebra and the direct
            // builder may configure the model itself first, so cover both orders.
            let retain_canonical_sentence_recipe = self.upstream_sentence_poet_source.is_none()
                && (self.has_upstream_sentence_model()
                    || (self.enable_sentence
                        && self.sentence_policy == SentencePolicy::UpstreamScript));
            let source_entries = if retain_canonical_sentence_recipe {
                self.source_entries.clone()
            } else {
                self.source_entries.take()
            }
            .unwrap_or_else(|| self.storage.owned_entries());
            let (entries, normal_codes, has_single_letter_abbreviations) =
                algebra.expand_entries_with_normal_codes(source_entries, self.entry_weight_domain);
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
            let retain_canonical_sentence_recipe = self.upstream_sentence_poet_source.is_none()
                && (self.has_upstream_sentence_model()
                    || (self.enable_sentence
                        && self.sentence_policy == SentencePolicy::UpstreamScript));
            let source_entries = if retain_canonical_sentence_recipe {
                self.source_entries.clone()
            } else {
                self.source_entries.take()
            }
            .unwrap_or_default();
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
                        lookup.abbreviation,
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
        if lookup_code.len() > MAX_SENTENCE_ALIAS_LOOKUP_BYTES
            || !self.direct_prism_surface_mapping_current
        {
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
                lookup.abbreviation,
                lookup.correction,
                lookup.credibility,
            ));
        }
        specs
    }

    fn upstream_script_surface_segmentation(
        &self,
        input: &str,
    ) -> Option<UpstreamScriptSurfaceGraph> {
        if self.sentence_policy != SentencePolicy::UpstreamScript
            || input.is_empty()
            || !self.direct_prism_surface_mapping_current
        {
            return None;
        }
        // A direct table code is already served by the ordinary exact path. In
        // particular, do not make identity-spelling schemas rescan every UTF-8
        // substring merely because their script policy is enabled.
        if self.storage.has_code(input) || self.untoned_dictionary() {
            return None;
        }
        let (Some(prism), Some(syllabary_codes)) =
            (self.prism_payload.as_ref(), self.storage.syllabary_codes())
        else {
            return None;
        };
        let boundaries = input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
            .collect::<Vec<_>>();
        // `false` is a normal/fuzzy path and `true` is an abbreviation path.
        // This is the part of librime's SpellingType ladder that changes the
        // class-4 surface: a normal/fuzzy path to the farthest vertex prunes
        // abbreviation edges, while a farthest vertex reachable only through
        // abbreviation keeps them.  Credibility ranks candidates after graph
        // construction; it does not choose the graph path.
        let mut vertex_abbreviation = vec![None::<bool>; boundaries.len()];
        let mut edges_by_start = vec![Vec::<SurfaceSyllable>::new(); boundaries.len()];
        let mut derived_syllable_choices = HashMap::<String, SurfaceCodeChoices>::new();
        vertex_abbreviation[0] = Some(false);

        for start_index in 0..boundaries.len().saturating_sub(1) {
            let Some(path_abbreviation) = vertex_abbreviation[start_index] else {
                continue;
            };
            let start = boundaries[start_index];
            for end_index in start_index + 1..boundaries.len() {
                let end = boundaries[end_index];
                if end - start > MAX_SENTENCE_ALIAS_LOOKUP_BYTES {
                    break;
                }
                let spelling = &input[start..end];
                let prism_start = crate::m37_metrics_enabled().then(Instant::now);
                // Script translation consumes the complete deployed spelling
                // family.  In particular a one-letter abbreviation such as
                // `n` can fan out to substantially more than the ordinary
                // sentence-alias cap; truncating that descriptor family makes
                // its order depend on prism storage position instead of raw
                // dictionary weight.
                let lookups = prism.lookup_canonical_codes(spelling, syllabary_codes);
                if let Some(start) = prism_start {
                    crate::m37_record_prism_lookup(start.elapsed(), lookups.len());
                }
                let mut choices = derived_syllable_choices
                    .get(spelling)
                    .cloned()
                    .unwrap_or_default();
                for lookup in lookups {
                    if lookup.correction {
                        continue;
                    }
                    if lookup.abbreviation && end < input.len() {
                        for syllable in source_code_syllables(lookup.code).skip(1) {
                            for variant in self
                                .spelling_algebra
                                .expand_deployed_spelling_variants(syllable)
                            {
                                if variant.properties.is_correction
                                    || variant.code.is_empty()
                                    || !surface_spelling_occurs_after(input, end, &variant.code)
                                {
                                    continue;
                                }
                                let abbreviation = variant.properties.spelling_type
                                    == DeployedSpellingType::Abbreviation;
                                derived_syllable_choices
                                    .entry(variant.code)
                                    .or_default()
                                    .merge(SurfaceCodeChoice {
                                        code: syllable.to_owned(),
                                        // The flattened descriptor's ID belongs
                                        // to the complete phrase, not this
                                        // recovered suffix syllable. Keep it
                                        // after real prism syllable IDs; a later
                                        // direct descriptor merge supplies the
                                        // canonical ID when one exists.
                                        syllable_id: usize::MAX,
                                        normal: variant.properties.spelling_type
                                            == DeployedSpellingType::Normal,
                                        abbreviation,
                                        credibility: variant.properties.credibility as f32,
                                    });
                            }
                        }
                    }
                    let (choice_code, choice_syllable_id) =
                        if source_code_syllable_count(lookup.code) == Some(1) {
                            (lookup.code, lookup.syllable_id)
                        } else if lookup.abbreviation {
                            // Yune's compact prism is built from flattened table
                            // codes, whereas librime's prism descriptors point at
                            // syllable IDs and Table::Query walks the remaining
                            // syllables through the dictionary trie. Recover that
                            // first live syllable from a multi-syllable abbreviation
                            // descriptor so phrase-only readings remain traversable
                            // without admitting the whole phrase as a one-span row.
                            let Some(prefix) = first_toned_syllable_prefix(lookup.code) else {
                                continue;
                            };
                            if self.storage.prefix_candidates(prefix).next().is_none() {
                                continue;
                            }
                            (prefix, usize::MAX)
                        } else {
                            continue;
                        };
                    choices.merge(SurfaceCodeChoice {
                        code: choice_code.to_owned(),
                        syllable_id: choice_syllable_id,
                        normal: lookup.normal,
                        abbreviation: lookup.abbreviation,
                        credibility: lookup.credibility,
                    });
                }
                if choices.choices.is_empty() {
                    continue;
                }
                // librime's `Syllabary` is a `std::set<string>` and
                // `SyllableGraph::indices` is traversed by the resulting
                // lexical syllable IDs. Yune's compact artifacts retain full
                // source codes in source order, so their descriptor IDs are
                // not the upstream traversal key (phrase-only recovery also
                // has no descriptor ID at all). Canonical-code order restores
                // the schema-generic Table::Query chunk order for both native
                // librime artifacts and Yune-rebuilt compact artifacts.
                choices
                    .choices
                    .sort_by(|left, right| left.code.cmp(&right.code));
                let edge_abbreviation = choices.choices.iter().all(|choice| choice.abbreviation);
                let syllable = SurfaceSyllable {
                    start,
                    end,
                    choices: choices.choices,
                };
                edges_by_start[start_index].push(syllable);
                let next_abbreviation = path_abbreviation || edge_abbreviation;
                vertex_abbreviation[end_index] = Some(
                    vertex_abbreviation[end_index]
                        .map_or(next_abbreviation, |current| current && next_abbreviation),
                );
            }
        }

        // librime's script translation owns the farthest recognized prefix,
        // not only a graph that reaches the end of the raw input.  This is the
        // load-bearing distinction for `nri`: `n` remains a live abbreviation
        // segment and `ri` remains raw for default recomposition.
        let interpreted_index = vertex_abbreviation.iter().rposition(Option::is_some)?;
        if interpreted_index == 0 {
            return None;
        }
        let allow_abbreviation = vertex_abbreviation[interpreted_index]?;

        // `TableTranslator::MakeSentence` separately retains normal-spelling
        // dictionary prefixes that start at zero, even when that prefix does
        // not participate in a path to the farthest syllabifier vertex. Keep
        // that collector surface apart from the pruned sentence graph so a
        // dead prefix can still be offered for manual composition without
        // becoming a sentence-model span.
        let unpruned_edges_by_start = edges_by_start.clone();
        let unpruned_leading_prefix_syllables = unpruned_edges_by_start
            .first()
            .into_iter()
            .flatten()
            .filter_map(|syllable| {
                let mut syllable = syllable.clone();
                syllable.choices.retain(|choice| choice.normal);
                (!syllable.choices.is_empty()).then_some(syllable)
            })
            .collect::<Vec<_>>();

        // Mirror Syllabifier::BuildSyllableGraph's reverse `good` walk.  Edges
        // that cannot reach the farthest vertex are stale.  If a normal/fuzzy
        // path reaches that vertex, abbreviation choices and abbreviation-only
        // vertices are worse spellings and are removed.  This is what keeps
        // `bein` on `be in` and `ngohaigo` on `ngo hai go`, while preserving
        // the required abbreviation graph for `n`, `nri`, and `ngohaig`.
        let mut good = vec![false; boundaries.len()];
        good[interpreted_index] = true;
        for start_index in (0..interpreted_index).rev() {
            if vertex_abbreviation[start_index].is_none()
                || (vertex_abbreviation[start_index] == Some(true) && !allow_abbreviation)
            {
                edges_by_start[start_index].clear();
                continue;
            }
            edges_by_start[start_index].retain_mut(|syllable| {
                let Some(end_index) = boundaries
                    .binary_search(&syllable.end)
                    .ok()
                    .filter(|end_index| good[*end_index])
                else {
                    return false;
                };
                let _ = end_index;
                if !allow_abbreviation {
                    syllable.choices.retain(|choice| !choice.abbreviation);
                }
                !syllable.choices.is_empty()
            });
            if !edges_by_start[start_index].is_empty() {
                good[start_index] = true;
            }
        }
        if !good[0] {
            return None;
        }
        let selected_first = edges_by_start
            .first()
            .and_then(|syllables| syllables.iter().max_by_key(|syllable| syllable.end));
        let selected_first_is_raw_spelling = selected_first.is_some_and(|syllable| {
            let spelling = &input[syllable.start..syllable.end];
            syllable
                .choices
                .iter()
                .any(|choice| normalized_original_code(&choice.code) == spelling)
        });
        let eligible_leading_prefix_syllables = selected_first
            .map(|selected| {
                unpruned_leading_prefix_syllables
                    .into_iter()
                    // A disconnected shorter prefix remains available for
                    // manual composition. A longer overlapping spelling is
                    // not a prefix of the viable first segment and must not
                    // leak into inputs such as `bein` (live `be`, dead `bei`).
                    .filter(|syllable| syllable.end <= selected.end)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        // Reachability ownership follows surface syllables, including normal
        // transformed spellings such as Bopomofo `cl3 -> hao3`. The independent
        // prefix collector below remains raw-spelling-only; conflating the two
        // lets its explicit-false control leak transformed leading singles.
        let mut one_syllable_prefix_ends = eligible_leading_prefix_syllables
            .iter()
            .map(|syllable| syllable.end)
            .collect::<Vec<_>>();
        one_syllable_prefix_ends.sort_unstable();
        one_syllable_prefix_ends.dedup();
        let leading_prefix_syllables = if selected_first_is_raw_spelling {
            eligible_leading_prefix_syllables
        } else {
            Vec::new()
        };
        let mut direct_edges_by_start = edges_by_start.clone();
        if let Some(selected) = selected_first {
            let selected_end_index = boundaries.binary_search(&selected.end).ok()?;
            // Longest-recognized-prefix recomposition is allowed to branch
            // after the viable first syllable. This retains a normal `ha`
            // phrase below live `ngo` even when a later abbreviation cannot
            // survive the reverse-good walk. It deliberately does not restore
            // alternate first-syllable overlaps; those are owned by the
            // shorter-only collector rule above.
            for start_index in selected_end_index..interpreted_index {
                for syllable in &unpruned_edges_by_start[start_index] {
                    if syllable.end > boundaries[interpreted_index] {
                        continue;
                    }
                    let mut syllable = syllable.clone();
                    if !allow_abbreviation {
                        syllable.choices.retain(|choice| !choice.abbreviation);
                    }
                    if !syllable.choices.is_empty() {
                        direct_edges_by_start[start_index].push(syllable);
                    }
                }
            }
        }
        Some(UpstreamScriptSurfaceGraph {
            interpreted_end: boundaries[interpreted_index],
            boundaries,
            edges_by_start,
            direct_edges_by_start,
            leading_prefix_syllables,
            one_syllable_prefix_ends,
        })
    }

    fn upstream_script_model_spans(
        graph: &UpstreamScriptSurfaceGraph,
    ) -> Vec<WeightedSentenceCodeSpan> {
        graph
            .direct_edges_by_start
            .iter()
            .flatten()
            .filter(|syllable| syllable.end <= graph.interpreted_end)
            .flat_map(|syllable| {
                syllable.choices.iter().map(|choice| {
                    WeightedSentenceCodeSpan::new(
                        SentenceCodeSpan::new(syllable.start, syllable.end, choice.code.clone()),
                        choice.credibility,
                    )
                })
            })
            .collect()
    }

    fn advance_upstream_script_direct_chunk<'a>(
        &'a self,
        input: &str,
        end: usize,
        filter_by_charset: bool,
        graph: &UpstreamScriptSurfaceGraph,
        chunk: &mut UpstreamScriptDirectChunk<'a>,
    ) {
        let surface = &input[..end];
        let weight_domain = self.entry_weight_domain;
        chunk.head = None;
        for lookup in chunk.rows.by_ref() {
            if !self.upstream_script_text_at_end_allowed(
                lookup.text(),
                end,
                input.len(),
                graph,
                filter_by_charset,
            ) {
                continue;
            }
            let raw_quality = lookup.raw_quality();
            let mut candidate = self.candidate_for_lookup_view(
                &chunk.code.code,
                &lookup,
                surface,
                None,
                chunk.code.credibility,
            );
            candidate.quality = upstream_script_raw_candidate_quality(
                end,
                raw_quality,
                chunk.code.credibility,
                weight_domain,
            );
            candidate.source = if end < input.len() {
                CandidateSource::PartialTable {
                    consumed: end,
                    recompose_on_default: true,
                }
            } else {
                CandidateSource::Table
            };
            let comparison_weight = match weight_domain {
                TableEntryWeightDomain::Raw => f64::from(raw_quality).max(f64::EPSILON).ln(),
                TableEntryWeightDomain::NaturalLog => f64::from(raw_quality),
            } + f64::from(chunk.code.credibility);
            chunk.head = Some((candidate, comparison_weight));
            return;
        }
    }

    fn upstream_script_ranked_family<'a>(
        &'a self,
        input: &str,
        end: usize,
        codes: &'a [ConcatenatedSurfaceCode],
        filter_by_charset: bool,
        visible_limit: Option<usize>,
        graph: &UpstreamScriptSurfaceGraph,
    ) -> Vec<RankedScriptPhraseCandidate> {
        let mut chunks = Vec::<UpstreamScriptDirectChunk<'a>>::new();
        for code in codes {
            let mut chunk = UpstreamScriptDirectChunk {
                code,
                rows: self.storage.exact_candidates(&code.code),
                head: None,
            };
            self.advance_upstream_script_direct_chunk(
                input,
                end,
                filter_by_charset,
                graph,
                &mut chunk,
            );
            if chunk.head.is_some() {
                chunks.push(chunk);
            }
        }

        // DictEntryIterator::Sort uses MSVC `partial_sort(first, first + 1,
        // last)`.  With a one-item heap this keeps the first equal head, but a
        // later strictly better head swaps its whole chunk into the active
        // slot.  Replaying that mutable chunk order is load-bearing for the
        // complete `n` family; a conventional stable heap does not reproduce
        // the later equal-weight ties after such a swap.
        let mut active = 0usize;
        let mut emitted = 0usize;
        let mut seen = HashSet::new();
        let mut family = Vec::new();
        while active < chunks.len() {
            for index in active + 1..chunks.len() {
                if chunks[index]
                    .head
                    .as_ref()
                    .zip(chunks[active].head.as_ref())
                    .is_some_and(|(candidate, current)| candidate.1 > current.1)
                {
                    // MSVC's one-item partial-sort heap moves the old active
                    // chunk into every strictly-better visitor's slot.  A
                    // single final swap with the best chunk leaves a different
                    // residual permutation and therefore diverges at later
                    // equal-weight ties.
                    chunks.swap(active, index);
                }
            }
            let (candidate, _) = chunks[active]
                .head
                .take()
                .expect("active script chunk should be nonempty");
            self.advance_upstream_script_direct_chunk(
                input,
                end,
                filter_by_charset,
                graph,
                &mut chunks[active],
            );
            if seen.insert(candidate.text.clone()) {
                family.push(RankedScriptPhraseCandidate {
                    candidate,
                    code_order: format!("\0{emitted:020}"),
                });
                emitted += 1;
            }
            if chunks[active].head.is_none() {
                active += 1;
            }
            if visible_limit.is_some_and(|limit| family.len() >= limit) {
                break;
            }
        }
        family
    }

    fn upstream_script_direct_families(
        &self,
        input: &str,
        graph: &UpstreamScriptSurfaceGraph,
        filter_by_charset: bool,
        visible_limit: Option<usize>,
    ) -> Option<(Vec<RankedScriptPhraseCandidate>, bool)> {
        let mut codes_by_boundary =
            vec![Vec::<ConcatenatedSurfaceCode>::new(); graph.boundaries.len()];
        let mut code_indexes_by_boundary =
            vec![HashMap::<String, usize>::new(); graph.boundaries.len()];
        let mut exact_codes_by_boundary =
            vec![Vec::<ConcatenatedSurfaceCode>::new(); graph.boundaries.len()];
        let mut exact_code_indexes_by_boundary =
            vec![HashMap::<String, usize>::new(); graph.boundaries.len()];
        codes_by_boundary[0].push(ConcatenatedSurfaceCode {
            code: String::new(),
            abbreviation: false,
            credibility: 0.0,
        });
        code_indexes_by_boundary[0].insert(String::new(), 0);
        let mut families = Vec::<(usize, Vec<RankedScriptPhraseCandidate>)>::new();

        for start_index in 0..graph.boundaries.len().saturating_sub(1) {
            let prefixes = codes_by_boundary[start_index].clone();
            if prefixes.is_empty() {
                continue;
            }
            for syllable in &graph.direct_edges_by_start[start_index] {
                if syllable.end > graph.interpreted_end {
                    continue;
                }
                let end_index = graph.boundaries.binary_search(&syllable.end).ok()?;
                let destination = &mut codes_by_boundary[end_index];
                let destination_indexes = &mut code_indexes_by_boundary[end_index];
                let exact_destination = &mut exact_codes_by_boundary[end_index];
                let exact_destination_indexes = &mut exact_code_indexes_by_boundary[end_index];
                for prefix in &prefixes {
                    for choice in &syllable.choices {
                        let mut code = String::with_capacity(prefix.code.len() + choice.code.len());
                        code.push_str(&prefix.code);
                        code.push_str(&choice.code);
                        let has_exact = self.storage.has_code(&code);
                        let has_strict_prefix = self.storage.has_strict_code_prefix(&code);
                        if !has_exact && !has_strict_prefix {
                            continue;
                        }
                        let abbreviation = prefix.abbreviation || choice.abbreviation;
                        let credibility = prefix.credibility + choice.credibility;
                        if has_exact {
                            if let Some(index) = exact_destination_indexes.get(&code).copied() {
                                let existing = &mut exact_destination[index];
                                if !abbreviation {
                                    existing.abbreviation = false;
                                }
                                existing.credibility = existing.credibility.max(credibility);
                            } else {
                                exact_destination_indexes
                                    .insert(code.clone(), exact_destination.len());
                                exact_destination.push(ConcatenatedSurfaceCode {
                                    code: code.clone(),
                                    abbreviation,
                                    credibility,
                                });
                            }
                        }
                        if has_strict_prefix {
                            if let Some(index) = destination_indexes.get(&code).copied() {
                                let existing = &mut destination[index];
                                if !abbreviation {
                                    existing.abbreviation = false;
                                }
                                existing.credibility = existing.credibility.max(credibility);
                            } else {
                                destination_indexes.insert(code.clone(), destination.len());
                                destination.push(ConcatenatedSurfaceCode {
                                    code,
                                    abbreviation,
                                    credibility,
                                });
                            }
                        }
                    }
                }
            }
        }

        for (end_index, exact_codes) in exact_codes_by_boundary.iter().enumerate().skip(1) {
            let end = graph.boundaries[end_index];
            if end > graph.interpreted_end || exact_codes.is_empty() {
                continue;
            }
            let codes = exact_codes.clone();
            let family = self.upstream_script_ranked_family(
                input,
                end,
                &codes,
                filter_by_charset,
                visible_limit,
                graph,
            );
            families.push((end, family));
        }

        // Prefix collectors in librime's `MakeSentence` are independent of
        // the graph's reverse-good walk. Add only their normal-spelling rows;
        // live graph families remain first and deduplication below makes this
        // a no-op unless the prefix was disconnected from the farthest path.
        for syllable in graph.leading_syllables() {
            let codes = syllable
                .choices
                .iter()
                .map(|choice| ConcatenatedSurfaceCode {
                    code: choice.code.clone(),
                    abbreviation: false,
                    credibility: choice.credibility,
                })
                .collect::<Vec<_>>();
            let family = self.upstream_script_ranked_family(
                input,
                syllable.end,
                &codes,
                filter_by_charset,
                visible_limit,
                graph,
            );
            families.push((syllable.end, family));
        }

        let has_full_exact = families
            .iter()
            .any(|(end, family)| *end == input.len() && !family.is_empty());
        families.sort_by_key(|(end, _)| std::cmp::Reverse(*end));
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        for (_, family) in families {
            for ranked in family {
                if seen.insert(ranked.candidate.text.clone()) {
                    candidates.push(ranked);
                    if visible_limit.is_some_and(|limit| candidates.len() >= limit) {
                        return Some((candidates, has_full_exact));
                    }
                }
            }
        }
        Some((candidates, has_full_exact))
    }

    fn upstream_script_translation(
        &self,
        input: &str,
        filter_by_charset: bool,
        limit: Option<usize>,
    ) -> Option<PrefixFallbackBatch> {
        let graph = self.upstream_script_surface_segmentation(input)?;
        let visible_limit = limit.map(|limit| limit.saturating_add(1));
        // Each direct family counts distinct eligible rows before applying its
        // K+1 cap. The first K+1 unique rows of the concatenated family union
        // must therefore occur within one of those retained family prefixes.
        let (direct, _) =
            self.upstream_script_direct_families(input, &graph, filter_by_charset, visible_limit)?;
        let spans = Self::upstream_script_model_spans(&graph);
        let model_candidates = if let Some(visible) = visible_limit {
            let eligible = |text: &str, end: usize| {
                self.upstream_script_text_at_end_allowed(
                    text,
                    end,
                    input.len(),
                    &graph,
                    filter_by_charset,
                )
            };
            self.upstream_sentence_model().map_or_else(Vec::new, |model| {
                model.ranked_script_phrase_candidates_for_weighted_code_spans_with_limit_filtered(
                    input,
                    &spans,
                    visible,
                    &eligible,
                )
            })
        } else {
            self.upstream_sentence_model()
                .map(|model| {
                    model.ranked_script_phrase_candidates_for_weighted_code_spans(input, &spans)
                })
                .unwrap_or_default()
        };
        let mut candidate_indices = HashMap::<String, usize>::new();
        let mut candidates = Vec::<RankedScriptPhraseCandidate>::new();
        for ranked in model_candidates {
            if !self.upstream_script_partial_candidate_allowed(&ranked.candidate, &graph)
                || !self.is_dictionary_text_allowed(&ranked.candidate.text)
                || (filter_by_charset && contains_extended_cjk(&ranked.candidate.text))
                || candidate_indices.contains_key(&ranked.candidate.text)
            {
                continue;
            }
            candidate_indices.insert(ranked.candidate.text.clone(), candidates.len());
            candidates.push(ranked);
        }
        // Yune's compact table intentionally omits preset-vocabulary phrases
        // that librime's DictCompiler materializes into the deployed Table.
        // The model reconstructs that complete table surface and now applies
        // DictEntryIterator's collector order, so it owns ordering. Enrich a
        // reconstructed row with direct-table metadata when available, and add
        // only direct rows that the reconstruction genuinely lacks.
        for mut direct in direct {
            if !self.upstream_script_partial_candidate_allowed(&direct.candidate, &graph) {
                continue;
            }
            if let Some(index) = candidate_indices.get(&direct.candidate.text).copied() {
                let ranked = &mut candidates[index];
                direct.candidate.quality = ranked.candidate.quality;
                ranked.candidate = direct.candidate;
                continue;
            }
            candidate_indices.insert(direct.candidate.text.clone(), candidates.len());
            candidates.push(direct);
        }
        for ranked in &mut candidates {
            if let CandidateSource::PartialTable { consumed, .. } = ranked.candidate.source {
                ranked.candidate.source = CandidateSource::PartialTable {
                    consumed,
                    recompose_on_default: true,
                };
            }
        }
        candidates.sort_by(|left, right| {
            right
                .candidate
                .quality
                .partial_cmp(&left.candidate.quality)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.code_order.cmp(&right.code_order))
        });
        let has_full_phrase = candidates.iter().any(|ranked| {
            !matches!(
                ranked.candidate.source,
                CandidateSource::PartialTable { .. }
            )
        });
        if !has_full_phrase {
            // No score-safe prefix certificate exists for an unseen table-trie
            // path. Record the exact cold fallback explicitly instead of
            // presenting it as bounded work; the retained Candidate window is
            // still capped below and warm requests use the page cache.
            crate::m37_record_full_list_fallback();
            let model_limit = limit.map(|limit| limit.saturating_add(1)).unwrap_or(100);
            let sentence = self.upstream_sentence_model().and_then(|model| {
                // Do not cap the table-trie walk by discovery count. A late
                // path can own a higher-weight sentence than every earlier BFS
                // state, so such a budget cannot preserve the complete prefix.
                // This full fallback is reached only when no direct full phrase
                // exists; ordinary bounded typing stays on the direct K+1 path.
                let candidates = model
                    .candidates_for_weighted_surface_code_spans_with_limit_excluding(
                        input,
                        &spans,
                        model_limit,
                        &self.dictionary_exclude,
                    );
                candidates.into_iter().find(|candidate| {
                    candidate.source == CandidateSource::Sentence
                        && self.is_dictionary_text_allowed(&candidate.text)
                        && (!filter_by_charset || !contains_extended_cjk(&candidate.text))
                })
            });
            if let Some(sentence) = sentence {
                candidates.insert(
                    0,
                    RankedScriptPhraseCandidate {
                        candidate: sentence,
                        code_order: String::new(),
                    },
                );
            }
        }
        let mut candidates = candidates
            .into_iter()
            .map(|ranked| ranked.candidate)
            .collect::<Vec<_>>();
        let mut seen = HashSet::new();
        candidates.retain(|candidate| seen.insert(candidate.text.clone()));
        if candidates.is_empty() {
            return None;
        }
        if let Some(preedit) = graph.preedit(input, self.delimiters.chars().next().unwrap_or(' ')) {
            let preedit = self.preedit_format.apply(&preedit);
            for candidate in &mut candidates {
                candidate.preedit = Some(preedit.clone());
            }
        }
        self.assign_upstream_script_candidate_qualities(&mut candidates);
        // A bounded upstream-script surface is intentionally a page window.
        // Even when that window happens to contain every currently visible row,
        // forward navigation must request the complete exact graph rather than
        // treating the bounded work budget as authoritative.
        let mut truncated = limit.is_some();
        if let Some(limit) = limit {
            if candidates.len() > limit {
                candidates.truncate(limit);
                truncated = true;
            }
        }
        let batch = PrefixFallbackBatch {
            candidates,
            truncated,
            owns_reachability: true,
            span_promotions: HashMap::new(),
        };
        Some(batch)
    }

    fn upstream_script_partial_candidate_allowed(
        &self,
        candidate: &Candidate,
        graph: &UpstreamScriptSurfaceGraph,
    ) -> bool {
        if self.leading_syllable_reachability || self.prefix_fallback {
            return true;
        }
        let CandidateSource::PartialTable { consumed, .. } = candidate.source else {
            return true;
        };
        let consumes_one_leading_syllable = graph.is_one_syllable_prefix_end(consumed);
        !consumes_one_leading_syllable || candidate.text.chars().count() != 1
    }

    fn upstream_script_text_at_end_allowed(
        &self,
        text: &str,
        end: usize,
        input_len: usize,
        graph: &UpstreamScriptSurfaceGraph,
        filter_by_charset: bool,
    ) -> bool {
        if !self.is_dictionary_text_allowed(text)
            || (filter_by_charset && contains_extended_cjk(text))
        {
            return false;
        }
        // The script policy owns sentence/phrase ordering, but it must not
        // silently re-enable M59's independently configurable leading-single
        // reachability. Preserve full-input rows, sentences, and multi-character
        // partial phrases. A separately enabled broad prefix_fallback continues
        // to own its historical reachability surface.
        if self.leading_syllable_reachability || self.prefix_fallback {
            return true;
        }
        if end >= input_len {
            return true;
        }
        let consumes_one_leading_syllable = graph.is_one_syllable_prefix_end(end);
        !consumes_one_leading_syllable || text.chars().count() != 1
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
            .filter(|candidate| {
                self.is_dictionary_text_allowed(&candidate.text)
                    && (!filter_by_charset || !contains_extended_cjk(&candidate.text))
            })
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
            || self.prediction_weight_threshold.map_or(true, |threshold| {
                table_raw_weight(candidate.raw_quality(), self.entry_weight_domain) >= threshold
            })
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

    fn is_reliable_exact_system_phrase(
        &self,
        lookup_spec: &LookupCodeSpec,
        candidate: &LookupCandidate<'_>,
    ) -> bool {
        lookup_spec.correction_distance.is_none()
            && !lookup_spec.tolerance
            && !lookup_spec.spelling_correction
            && !self.is_spelling_abbreviation_view(&lookup_spec.lookup_code, candidate)
            && !self.is_spelling_correction_view(&lookup_spec.lookup_code, candidate)
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
                    .comparison_weight(self.entry_weight_domain)
                    .partial_cmp(&left.comparison_weight(self.entry_weight_domain))
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
        // current group heads by compiled-domain weight while keeping every group's source
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
            |candidate| candidate.comparison_weight(self.entry_weight_domain),
            |left, right| self.lookup_candidate_weight_order(left, right),
            |prediction, ordinary| {
                prediction.prediction_precedes(ordinary, self.entry_weight_domain)
            },
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

    fn lookup_materialization_log_quality(
        &self,
        stored_quality: f32,
        spelling_credibility: f32,
        correction_distance: Option<usize>,
    ) -> f32 {
        // Candidate qualities live in Yune's historical outer `exp(score)`
        // namespace. Reconstruct the effective raw dictionary weight after
        // applying a live prism credibility, then use that raw value as the
        // score passed to the outer exponent. This keeps source-expanded,
        // compact-prism, and v2 NaturalLog tables in one merge namespace.
        let mut log_quality = match self.entry_weight_domain {
            // Preserve DictCompiler's exact non-positive sentinel in score
            // space. Reconstructing its epsilon raw weight and then applying
            // the outer exponent rounds both direct and correction-penalized
            // rows to 1.0, erasing librime's observable order.
            TableEntryWeightDomain::NaturalLog
                if stored_quality.to_bits() == LIBRIME_NON_POSITIVE_COMPILED_LOG_WEIGHT_BITS =>
            {
                stored_quality + spelling_credibility
            }
            TableEntryWeightDomain::NaturalLog => (stored_quality + spelling_credibility).exp(),
            // Raw/Yune-authored tables historically expose their numeric entry
            // as a log score (`10 -> exp(10)`). Preserve that contract for an
            // ordinary row and for heap algebra, where credibility is already
            // baked into the expanded stored weight. A live compact-prism edge
            // carries nonzero credibility separately and must apply it in the
            // raw dictionary's log domain.
            TableEntryWeightDomain::Raw if spelling_credibility == 0.0 => stored_quality,
            // Zero/negative source values are already historical scores, not
            // absolute weights. Apply a live prism credibility directly so a
            // correction penalty remains below its neutral direct sibling.
            TableEntryWeightDomain::Raw if stored_quality <= 0.0 => {
                stored_quality + spelling_credibility
            }
            TableEntryWeightDomain::Raw => {
                (table_log_weight(stored_quality, self.entry_weight_domain) + spelling_credibility)
                    .exp()
            }
        };
        if let Some(distance) = correction_distance {
            log_quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        log_quality
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
        let log_quality = self.lookup_materialization_log_quality(
            candidate.raw_quality(),
            spelling_credibility,
            correction_distance,
        );
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
        let mut source = source_hint;
        let mut quality = log_quality.exp() + self.initial_quality;
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
        let log_quality = self.lookup_materialization_log_quality(
            candidate.quality,
            spelling_credibility,
            correction_distance,
        );
        candidate.quality = log_quality.exp() + self.initial_quality;
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

    fn lookup_candidate_ref_comparison_weight(
        &self,
        candidate: &PendingLookupCandidateRef<'_>,
    ) -> f32 {
        let mut log_quality =
            table_comparison_weight(candidate.candidate.raw_quality(), self.entry_weight_domain)
                + candidate.spelling_credibility;
        if let Some(distance) = candidate.correction_distance {
            log_quality += TYPEDUCK_CORRECTION_CREDIBILITY * distance as f32;
        }
        log_quality
    }

    fn lookup_candidate_ref_prediction_weight(
        &self,
        candidate: &PendingLookupCandidateRef<'_>,
    ) -> f32 {
        let mut weight =
            table_comparison_weight(candidate.candidate.raw_quality(), self.entry_weight_domain)
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
                self.lookup_candidate_ref_comparison_weight(right)
                    .partial_cmp(&self.lookup_candidate_ref_comparison_weight(left))
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
            |candidate| self.lookup_candidate_ref_comparison_weight(candidate),
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
        let log_quality = self.lookup_materialization_log_quality(
            candidate.raw_quality(),
            spelling_credibility,
            correction_distance,
        );
        let mut quality = log_quality.exp() + self.initial_quality;
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

    fn legacy_bounded_upstream_sentence_fallback(
        &self,
        request: LegacyBoundedSentenceFallbackRequest<'_>,
    ) -> Option<TranslationResult> {
        let LegacyBoundedSentenceFallbackRequest {
            model,
            input,
            lookup_code,
            filter_by_charset,
            limit,
            include_full_count,
            has_correction_lookup,
            scratch,
        } = request;
        let sentence_limit = limit.min(BOUNDED_SENTENCE_MODEL_PAGE_LIMIT);
        let model_start = crate::m37_metrics_enabled().then(Instant::now);
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
        .filter(|candidate| {
            self.is_dictionary_text_allowed(&candidate.text)
                && (!filter_by_charset || !contains_extended_cjk(&candidate.text))
        })
        .collect::<Vec<_>>();
        if let Some(start) = model_start {
            crate::m37_record_upstream_sentence_model(start.elapsed(), candidates.len());
        }
        if candidates.is_empty() {
            let abbreviation_start = crate::m37_metrics_enabled().then(Instant::now);
            candidates = self.abbreviation_sentence_candidates(
                model,
                input,
                sentence_limit,
                filter_by_charset,
            );
            if let Some(start) = abbreviation_start {
                crate::m37_record_upstream_sentence_model(start.elapsed(), candidates.len());
            }
        }
        if candidates.is_empty() {
            return None;
        }

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
                    &[],
                    Some(room),
                );
                prefix_fallback_owned = prefix_batch.owns_reachability;
                prefix_fallback_truncated = prefix_batch.truncated;
                let inserted = merge_prefix_fallback_candidates_with_full_input_anchor(
                    &mut candidates,
                    &prefix_batch.span_promotions,
                    prefix_batch.candidates,
                    lookup_code,
                    None,
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
            let insert_at = leading_single_insert_index(&candidates);
            let want = limit.saturating_sub(insert_at).saturating_add(1);
            let leading_singles = self.leading_single_syllable_prefix_candidates(
                input,
                lookup_code,
                filter_by_charset,
                &candidates[..insert_at],
                Some(want),
            );
            if !leading_singles.is_empty() {
                candidates.splice(insert_at..insert_at, leading_singles);
                self.assign_mode_stable_ordered_candidate_qualities(&mut candidates);
            }
        }
        self.assign_mode_stable_ordered_candidate_qualities(&mut candidates);
        let merged_window_overflow = candidates.len() > limit;
        candidates.truncate(limit);
        let full_count =
            if prefix_fallback_truncated || base_window_may_have_more || merged_window_overflow {
                limit.saturating_add(1)
            } else {
                candidates.len()
            };
        crate::m37_record_bounded_iterator(limit, candidates.len(), full_count);
        Some(TranslationResult::bounded(
            candidates,
            full_count,
            include_full_count,
        ))
    }

    fn bounded_candidates_for_lookup_codes(
        &self,
        request: BoundedLookupRequest<'_>,
        mut scratch: Option<&mut TranslatorScratch>,
    ) -> TranslationResult {
        if let Some(result) = self.cached_bounded_upstream_script_result(
            request.input,
            request.filter_by_charset,
            request.limit,
            request.include_full_count,
        ) {
            if let Some(scratch) = scratch.as_deref_mut() {
                scratch.clear();
            }
            return result;
        }
        let result = self.bounded_candidates_for_lookup_codes_uncached(request, scratch);
        self.cache_bounded_upstream_script_result(
            request.input,
            request.filter_by_charset,
            request.limit,
            request.include_full_count,
            result.clone(),
        );
        result
    }

    fn cached_bounded_upstream_script_result(
        &self,
        input: &str,
        filter_by_charset: bool,
        limit: usize,
        include_full_count: bool,
    ) -> Option<TranslationResult> {
        if !self.caches_bounded_translation_results() {
            return None;
        }
        self.bounded_upstream_script_cache
            .lock()
            .expect("bounded upstream-script cache should not be poisoned")
            .get(input, filter_by_charset, limit, include_full_count)
    }

    fn cache_bounded_upstream_script_result(
        &self,
        input: &str,
        filter_by_charset: bool,
        limit: usize,
        include_full_count: bool,
        result: TranslationResult,
    ) {
        if !self.caches_bounded_translation_results() {
            return;
        }
        self.bounded_upstream_script_cache
            .lock()
            .expect("bounded upstream-script cache should not be poisoned")
            .insert(input, filter_by_charset, limit, include_full_count, result);
    }

    fn caches_bounded_translation_results(&self) -> bool {
        (self.sentence_policy == SentencePolicy::UpstreamScript && self.enable_sentence)
            || (self.sentence_policy == SentencePolicy::LegacyFallback && self.prefix_fallback)
    }

    #[cfg(test)]
    pub(crate) fn cache_bounded_result_probe(&self, result: TranslationResult) {
        self.bounded_upstream_script_cache
            .lock()
            .expect("bounded upstream-script cache should not be poisoned")
            .insert("probe", false, 2, false, result);
    }

    fn bounded_candidates_for_lookup_codes_uncached(
        &self,
        request: BoundedLookupRequest<'_>,
        mut scratch: Option<&mut TranslatorScratch>,
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
        let mut has_reliable_exact_system_phrase = false;
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
                if self.has_upstream_sentence_model() {
                    has_reliable_exact_system_phrase |=
                        self.is_reliable_exact_system_phrase(lookup_spec, &candidate);
                }
                let spelling_abbreviation = lookup_spec.spelling_abbreviation
                    || self.is_spelling_abbreviation_view(spec_lookup_code, &candidate);
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
                        let spelling_abbreviation = lookup_spec.spelling_abbreviation
                            || self.is_spelling_abbreviation_view(entry_code.as_ref(), &candidate);
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
        if self.sentence_policy == SentencePolicy::LegacyFallback
            && self.enable_sentence
            && selected.is_empty()
        {
            if let Some(model) = self.upstream_sentence_model() {
                if let Some(result) = self.legacy_bounded_upstream_sentence_fallback(
                    LegacyBoundedSentenceFallbackRequest {
                        model,
                        input,
                        lookup_code,
                        filter_by_charset,
                        limit,
                        include_full_count,
                        has_correction_lookup,
                        scratch: scratch.as_deref_mut(),
                    },
                ) {
                    return result;
                }
            }
        }
        if selected.is_empty() && self.enable_sentence && !self.has_upstream_sentence_model() {
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
        if selected.is_empty()
            && self.prefix_fallback
            && !has_correction_lookup
            && !self.has_upstream_sentence_model()
        {
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
                // The direct fallback fast path bypasses the common ordered
                // tail below. Export positional qualities here as well so the
                // Engine's global quality merge preserves the fallback's
                // longest-consumed-span order instead of re-sorting the rows
                // by source dictionary weight.
                self.assign_mode_stable_ordered_candidate_qualities(&mut batch.candidates);
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
        let full_input_indices = if self.combine_candidates {
            selected
                .iter()
                .enumerate()
                .filter_map(|(index, candidate)| candidate.owns_full_input_span().then_some(index))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let prefix_fallback_span_views = selected
            .iter()
            .filter(|candidate| {
                candidate.owns_full_input_span()
                    && candidate.candidate.source_hint() == CandidateSource::Table
            })
            .map(|candidate| PrefixFallbackSpanView {
                text: candidate.candidate.text().to_owned(),
                raw_comment: candidate.candidate.raw_comment().to_owned(),
                spelling_abbreviation: candidate.spelling_abbreviation,
            })
            .collect::<Vec<_>>();
        let mut full_input_anchor = if self.combine_candidates {
            full_input_indices.last().copied()
        } else {
            selected
                .iter()
                .rposition(PendingLookupCandidateRef::owns_full_input_span)
        };
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
            (candidates, full_input_anchor) =
                combine_duplicate_text_candidates_with_full_input_anchor(
                    candidates,
                    &full_input_indices,
                );
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
                    &candidates,
                    &prefix_fallback_span_views,
                    Some(limit),
                );
                prefix_fallback_owned = prefix_batch.owns_reachability;
                let mut prefix_fallback_truncated = prefix_batch.truncated;
                let inserted = merge_prefix_fallback_candidates_with_full_input_anchor(
                    &mut candidates,
                    &prefix_batch.span_promotions,
                    prefix_batch.candidates,
                    lookup_code,
                    full_input_anchor,
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
            self.assign_mode_stable_ordered_candidate_qualities(&mut candidates);
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
                self.assign_mode_stable_ordered_candidate_qualities(&mut candidates);
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
        if self.sentence_policy == SentencePolicy::UpstreamScript
            && self.enable_sentence
            && !has_reliable_exact_system_phrase
        {
            if let Some(model) = self.upstream_sentence_model() {
                let sentence_limit = limit.min(BOUNDED_SENTENCE_MODEL_PAGE_LIMIT);
                let model_start = crate::m37_metrics_enabled().then(Instant::now);
                let mut upstream_candidates = if let Some(scratch) = scratch.as_mut() {
                    model.candidates_for_input_with_limit_and_scratch(
                        input,
                        sentence_limit,
                        &mut scratch.upstream_sentence,
                    )
                } else {
                    model.candidates_for_input_with_limit(input, sentence_limit)
                }
                .into_iter()
                .filter(|candidate| {
                    self.is_dictionary_text_allowed(&candidate.text)
                        && (!filter_by_charset || !contains_extended_cjk(&candidate.text))
                })
                .collect::<Vec<_>>();
                if let Some(start) = model_start {
                    crate::m37_record_upstream_sentence_model(
                        start.elapsed(),
                        upstream_candidates.len(),
                    );
                }
                if upstream_candidates.is_empty() {
                    let abbreviation_start = crate::m37_metrics_enabled().then(Instant::now);
                    upstream_candidates = self.abbreviation_sentence_candidates(
                        model,
                        input,
                        sentence_limit,
                        filter_by_charset,
                    );
                    if let Some(start) = abbreviation_start {
                        crate::m37_record_upstream_sentence_model(
                            start.elapsed(),
                            upstream_candidates.len(),
                        );
                    }
                }
                if !upstream_candidates.is_empty() {
                    let upstream_window_may_have_more = upstream_candidates.len() >= sentence_limit;
                    candidates = merge_upstream_script_translation_candidates(
                        input,
                        upstream_candidates,
                        candidates,
                        usize::MAX,
                        self.initial_quality,
                        !self.comment_format.is_empty(),
                        !self.preedit_format.is_empty(),
                    );
                    let merged_window_overflow = candidates.len() > limit;
                    candidates.truncate(limit);
                    if upstream_window_may_have_more || merged_window_overflow {
                        full_count = full_count.max(limit.saturating_add(1));
                    } else {
                        full_count = full_count.max(candidates.len());
                    }
                }
            }
        }
        if candidates.is_empty() && self.enable_sentence {
            if let Some(sentence) = self.sentence_candidate(input, filter_by_charset, None) {
                candidates.push(sentence);
                full_count = full_count.max(1);
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
    ) -> LookupCandidateBatch {
        self.candidates_for_lookup_codes_with_completion(lookup_specs, filter_by_charset, true)
    }

    fn candidates_for_lookup_codes_with_completion(
        &self,
        lookup_specs: &[LookupCodeSpec],
        filter_by_charset: bool,
        include_completion: bool,
    ) -> LookupCandidateBatch {
        let mut pooled: Vec<PendingLookupCandidate> = Vec::new();
        let mut exact_scan_ranges: Vec<(usize, usize)> = Vec::new();
        let mut fetch_groups = HashMap::new();
        let mut has_reliable_exact_system_phrase = false;
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
                        has_reliable_exact_system_phrase |=
                            self.is_reliable_exact_system_phrase(lookup_spec, &candidate);
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
                            spelling_abbreviation: lookup_spec.spelling_abbreviation
                                || self.is_spelling_abbreviation_view(lookup_code, &candidate),
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
                && include_completion
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
                    let spelling_abbreviation = lookup_spec.spelling_abbreviation
                        || self.is_spelling_abbreviation_view(entry_code.as_ref(), &candidate);
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
                let quality = pending.comparison_weight(self.entry_weight_domain);
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
            exact_rows.sort_by(|left, right| {
                right
                    .comparison_weight(self.entry_weight_domain)
                    .total_cmp(&left.comparison_weight(self.entry_weight_domain))
            });
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
        let full_input_indices = if self.combine_candidates {
            pooled
                .iter()
                .enumerate()
                .filter_map(|(index, candidate)| candidate.owns_full_input_span().then_some(index))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let mut full_input_anchor = if self.combine_candidates {
            full_input_indices.last().copied()
        } else {
            pooled
                .iter()
                .rposition(PendingLookupCandidate::owns_full_input_span)
        };
        let prefix_fallback_span_views = pooled
            .iter()
            .filter(|pending| {
                pending.owns_full_input_span() && pending.candidate.source == CandidateSource::Table
            })
            .map(|pending| PrefixFallbackSpanView {
                text: pending.candidate.text.clone(),
                raw_comment: pending.candidate.comment.clone(),
                spelling_abbreviation: pending.spelling_abbreviation,
            })
            .collect();
        let mut candidates = pooled
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
        if self.combine_candidates {
            (candidates, full_input_anchor) =
                combine_duplicate_text_candidates_with_full_input_anchor(
                    candidates,
                    &full_input_indices,
                );
        }
        if record_track_b {
            for _ in 0..pending_count {
                crate::m37_record_track_b_candidate_materialized();
            }
        }
        LookupCandidateBatch {
            candidates,
            full_input_anchor,
            has_reliable_exact_system_phrase,
            prefix_fallback_span_views,
        }
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

    fn prefix_fallback_view_is_deferred_surface_phrase(
        prefix_spec: &LookupPrefixSpec<'_>,
        candidate: &LookupCandidate<'_>,
    ) -> bool {
        // A non-abbreviation algebra surface may expose a phrase at a shorter
        // spelling than its normalized raw code (`zouha` -> `zou6 haa5`). Keep
        // that new reachability, but rank it after raw-compatible prefix
        // families so it cannot evict canonical singles from a bounded page.
        // Abbreviations carry their own deployed ordering semantics and are not
        // part of this transformed-phrase fallback tier.
        prefix_spec
            .surface_fetch
            .as_ref()
            .is_some_and(|fetch| !fetch.abbreviation)
            && candidate.text().chars().nth(1).is_some()
            && !original_code_allows_prefix_fallback(
                candidate.raw_comment(),
                prefix_spec.input_prefix,
            )
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
        let mut saw_prefix = false;
        let mut views_visited = 0usize;
        let mut truncated = false;
        let mut found_unique = false;
        let _: ControlFlow<()> = self.visit_valid_lookup_prefixes(lookup_code, |prefix_spec| {
            saw_prefix = true;
            let exact_start = LookupTimer::start();
            let mut exact_candidates = 0usize;
            let mut emitted_for_fetch_code = 0usize;
            for candidate in self
                .storage
                .exact_candidates(&prefix_spec.fetch_code)
                .filter(|candidate| {
                    self.prefix_fallback_view_is_allowed(&prefix_spec, candidate, filter_by_charset)
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
            if found_unique || truncated {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        if found_unique {
            if let Some(start) = fallback_start {
                crate::m37_record_prefix_fallback(start.elapsed(), views_visited, 1);
            }
            return PrefixFallbackProbe::Found;
        }
        if let Some(start) = fallback_start {
            crate::m37_record_prefix_fallback(start.elapsed(), views_visited, 0);
        }
        if !saw_prefix {
            PrefixFallbackProbe::NoPrefix
        } else if truncated {
            PrefixFallbackProbe::Truncated
        } else {
            PrefixFallbackProbe::Exhausted
        }
    }

    fn build_bounded_prefix_fallback_cache_entry(
        &self,
        input: &str,
        lookup_code: &str,
        prefixes: &[LookupPrefixSpec<'_>],
        filter_by_charset: bool,
        pending_cap: usize,
        key: PrefixFallbackWindowCacheKey,
    ) -> (Arc<PrefixFallbackWindowCacheEntry>, usize) {
        let full_span_texts: HashSet<&str> = HashSet::new();
        let blocked_texts = HashSet::new();
        let mut selected_texts = HashSet::new();
        let (stream_rows, views_visited, exhausted) = self
            .bounded_prefix_stream_rows_from_prefixes(
                input,
                lookup_code,
                filter_by_charset,
                &full_span_texts,
                &blocked_texts,
                &mut selected_texts,
                prefixes.to_vec(),
                pending_cap,
            );
        let rows = stream_rows
            .into_iter()
            .filter_map(|row| {
                let prefix_spec = prefixes.get(row.fetch_order)?;
                let candidate = self
                    .storage
                    .exact_candidates(&row.fetch_code)
                    .nth(row.candidate_index)?;
                let spelling_abbreviation =
                    self.is_spelling_abbreviation_view(prefix_spec.input_prefix, &candidate);
                let candidate = candidate.to_candidate();
                Some(CachedPrefixFallbackView {
                    fetch_code: row.fetch_code,
                    input_prefix: prefix_spec.input_prefix.to_owned(),
                    candidate,
                    consumed_lookup_len: row.consumed_lookup_len,
                    surface_abbreviation: prefix_spec
                        .surface_fetch
                        .as_ref()
                        .is_some_and(|fetch| fetch.abbreviation),
                    spelling_abbreviation,
                })
            })
            .collect();
        (
            Arc::new(PrefixFallbackWindowCacheEntry {
                key,
                rows,
                truncated: !exhausted,
            }),
            views_visited,
        )
    }

    fn advance_bounded_prefix_stream_chunk(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        full_span_texts: &HashSet<&str>,
        _blocked_texts: &HashSet<String>,
        chunk: &mut BoundedPrefixStreamChunk<'_>,
    ) -> usize {
        let exact_start = LookupTimer::start();
        let mut exact_candidates = 0usize;
        let mut views_visited = 0usize;
        chunk.head = None;
        for (candidate_index, candidate) in self
            .storage
            .exact_candidates(&chunk.prefix.fetch_code)
            .enumerate()
            .skip(chunk.next_candidate_index)
        {
            chunk.next_candidate_index = candidate_index.saturating_add(1);
            if !self.prefix_fallback_view_is_allowed(&chunk.prefix, &candidate, filter_by_charset) {
                continue;
            }
            views_visited += 1;
            exact_candidates += 1;
            let consumed_input_len = if full_span_texts.contains(candidate.text()) {
                input.len()
            } else {
                input
                    .len()
                    .saturating_sub(lookup_code.len())
                    .saturating_add(chunk.prefix.consumed_lookup_len)
            };
            let spelling_abbreviation =
                self.is_spelling_abbreviation_view(chunk.prefix.input_prefix, &candidate);
            chunk.head = Some(BoundedPrefixStreamRow {
                fetch_code: chunk.prefix.fetch_code.clone(),
                fetch_order: chunk.fetch_order,
                candidate_index,
                text: candidate.text().to_owned(),
                consumed_lookup_len: chunk.prefix.consumed_lookup_len,
                consumed_input_len,
                recompose_on_default: consumed_input_len > 1
                    && !chunk
                        .prefix
                        .surface_fetch
                        .as_ref()
                        .is_some_and(|fetch| fetch.abbreviation)
                    && !spelling_abbreviation,
                deferred_surface_phrase: Self::prefix_fallback_view_is_deferred_surface_phrase(
                    &chunk.prefix,
                    &candidate,
                ),
                raw_quality: candidate.raw_quality(),
            });
            break;
        }
        self.storage
            .record_exact_lookup(exact_start.elapsed(), exact_candidates);
        views_visited
    }

    #[allow(clippy::too_many_arguments)]
    fn push_bounded_prefix_stream_chunks<'input>(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        full_span_texts: &HashSet<&str>,
        blocked_texts: &HashSet<String>,
        prefixes: impl IntoIterator<Item = LookupPrefixSpec<'input>>,
        next_fetch_order: &mut usize,
        chunks: &mut Vec<BoundedPrefixStreamChunk<'input>>,
    ) -> usize {
        let mut views_visited = 0usize;
        for prefix in prefixes {
            let fetch_order = *next_fetch_order;
            *next_fetch_order = next_fetch_order.saturating_add(1);
            let mut chunk = BoundedPrefixStreamChunk {
                prefix,
                fetch_order,
                next_candidate_index: 0,
                head: None,
            };
            views_visited += self.advance_bounded_prefix_stream_chunk(
                input,
                lookup_code,
                filter_by_charset,
                full_span_texts,
                blocked_texts,
                &mut chunk,
            );
            if chunk.head.is_some() {
                chunks.push(chunk);
            }
        }
        views_visited
    }

    #[allow(clippy::too_many_arguments)]
    fn drain_bounded_prefix_stream_chunks(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        full_span_texts: &HashSet<&str>,
        blocked_texts: &HashSet<String>,
        selected_texts: &mut HashSet<String>,
        chunks: &mut Vec<BoundedPrefixStreamChunk<'_>>,
        rows: &mut Vec<BoundedPrefixStreamRow>,
        target: usize,
        allow_deferred: bool,
    ) -> usize {
        let mut views_visited = 0usize;
        while !chunks.is_empty() && rows.len() < target {
            for visitor in 1..chunks.len() {
                if chunks[visitor]
                    .head
                    .as_ref()
                    .zip(chunks[0].head.as_ref())
                    .is_some_and(|(candidate, current)| {
                        bounded_prefix_stream_head_strictly_precedes(candidate, current)
                    })
                {
                    chunks.swap(0, visitor);
                }
            }
            if !allow_deferred
                && chunks[0]
                    .head
                    .as_ref()
                    .is_some_and(|row| row.deferred_surface_phrase)
            {
                break;
            }
            let row = chunks[0]
                .head
                .take()
                .expect("active bounded-prefix chunk has a head");
            views_visited += self.advance_bounded_prefix_stream_chunk(
                input,
                lookup_code,
                filter_by_charset,
                full_span_texts,
                blocked_texts,
                &mut chunks[0],
            );
            if chunks[0].head.is_none() {
                chunks.remove(0);
            }
            if selected_texts.insert(row.text.clone()) {
                rows.push(row);
            }
        }
        views_visited
    }

    #[allow(clippy::too_many_arguments)]
    fn bounded_prefix_stream_rows_from_prefixes(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        full_span_texts: &HashSet<&str>,
        blocked_texts: &HashSet<String>,
        selected_texts: &mut HashSet<String>,
        prefixes: Vec<LookupPrefixSpec<'_>>,
        target: usize,
    ) -> (Vec<BoundedPrefixStreamRow>, usize, bool) {
        let mut chunks = Vec::new();
        let mut next_fetch_order = 0usize;
        let mut views_visited = self.push_bounded_prefix_stream_chunks(
            input,
            lookup_code,
            filter_by_charset,
            full_span_texts,
            blocked_texts,
            prefixes,
            &mut next_fetch_order,
            &mut chunks,
        );
        let mut rows = Vec::new();
        views_visited += self.drain_bounded_prefix_stream_chunks(
            input,
            lookup_code,
            filter_by_charset,
            full_span_texts,
            blocked_texts,
            selected_texts,
            &mut chunks,
            &mut rows,
            target,
            true,
        );
        (rows, views_visited, chunks.is_empty())
    }

    #[allow(clippy::too_many_arguments)]
    fn bounded_prefix_stream_rows_incremental<'input>(
        &self,
        input: &'input str,
        lookup_code: &'input str,
        filter_by_charset: bool,
        full_span_texts: &HashSet<&str>,
        blocked_texts: &HashSet<String>,
        selected_texts: &mut HashSet<String>,
        target: usize,
    ) -> (Vec<BoundedPrefixStreamRow>, usize, bool, bool) {
        let mut chunks = Vec::new();
        let mut current_span = Vec::<LookupPrefixSpec<'input>>::new();
        let mut next_fetch_order = 0usize;
        let mut rows = Vec::new();
        let mut views_visited = 0usize;
        let mut saw_prefix = false;
        let traversal = self.visit_valid_lookup_prefixes(lookup_code, |prefix| {
            saw_prefix = true;
            if current_span
                .first()
                .is_some_and(|current| current.consumed_lookup_len != prefix.consumed_lookup_len)
            {
                views_visited += self.push_bounded_prefix_stream_chunks(
                    input,
                    lookup_code,
                    filter_by_charset,
                    full_span_texts,
                    blocked_texts,
                    current_span.drain(..),
                    &mut next_fetch_order,
                    &mut chunks,
                );
                views_visited += self.drain_bounded_prefix_stream_chunks(
                    input,
                    lookup_code,
                    filter_by_charset,
                    full_span_texts,
                    blocked_texts,
                    selected_texts,
                    &mut chunks,
                    &mut rows,
                    target,
                    false,
                );
                if rows.len() >= target {
                    return ControlFlow::Break(());
                }
            }
            current_span.push(prefix);
            ControlFlow::Continue(())
        });
        let visited_all = matches!(traversal, ControlFlow::Continue(()));
        if visited_all {
            views_visited += self.push_bounded_prefix_stream_chunks(
                input,
                lookup_code,
                filter_by_charset,
                full_span_texts,
                blocked_texts,
                current_span,
                &mut next_fetch_order,
                &mut chunks,
            );
            views_visited += self.drain_bounded_prefix_stream_chunks(
                input,
                lookup_code,
                filter_by_charset,
                full_span_texts,
                blocked_texts,
                selected_texts,
                &mut chunks,
                &mut rows,
                target,
                true,
            );
        }
        (
            rows,
            views_visited,
            visited_all && chunks.is_empty(),
            saw_prefix,
        )
    }

    fn bounded_profile_prefix_fallback_candidates(
        &self,
        request: BoundedPrefixFallbackStreamingRequest<'_, '_>,
    ) -> (PrefixFallbackBatch, usize, bool) {
        let BoundedPrefixFallbackStreamingRequest {
            input,
            lookup_code,
            filter_by_charset,
            existing_candidates,
            admitted_span_candidates,
            limit,
        } = request;
        let mut seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<HashSet<_>>();
        seen_texts.extend(
            admitted_span_candidates
                .iter()
                .map(|candidate| candidate.text.clone()),
        );
        let per_fetch_cap =
            if input.chars().count() <= PREFIX_FALLBACK_BOUNDED_REACHABILITY_MAX_INPUT_CHARS {
                limit.min(PREFIX_FALLBACK_BOUNDED_REACHABILITY_CANDIDATES_PER_FETCH_CODE)
            } else if input.len() <= MAX_ABBREVIATION_SENTENCE_INPUT_BYTES {
                limit
            } else {
                limit.min(PREFIX_FALLBACK_BOUNDED_CANDIDATES_PER_FETCH_CODE)
            };
        let pending_multiplier = if input.len() <= MAX_ABBREVIATION_SENTENCE_INPUT_BYTES {
            PREFIX_FALLBACK_BOUNDED_PENDING_MULTIPLIER
        } else {
            PREFIX_FALLBACK_PROFILE_LONG_PENDING_MULTIPLIER
        };
        let pending_cap = limit.saturating_mul(pending_multiplier).max(limit);
        let input_base = input.len().saturating_sub(lookup_code.len());
        let mut pending = Vec::<BoundedPrefixStreamRow>::new();
        let mut fetch_rows = Vec::<BoundedPrefixStreamRow>::with_capacity(per_fetch_cap);
        let mut emission_order = 0usize;
        let mut views_visited = 0usize;
        let mut deferred_row_count = 0usize;
        let mut truncated = false;
        let mut saw_prefix = false;
        let _: ControlFlow<()> = self.visit_valid_lookup_prefixes(lookup_code, |prefix| {
            saw_prefix = true;
            fetch_rows.clear();
            let mut deferred_fetch_row_count = 0usize;
            let exact_start = LookupTimer::start();
            let mut exact_candidates = 0usize;
            for (candidate_index, candidate) in self
                .storage
                .exact_candidates(&prefix.fetch_code)
                .enumerate()
            {
                if !self.prefix_fallback_view_is_allowed(&prefix, &candidate, filter_by_charset) {
                    continue;
                }
                views_visited += 1;
                exact_candidates += 1;
                if fetch_rows.iter().any(|row| row.text == candidate.text()) {
                    continue;
                }
                let consumed_input_len = input_base.saturating_add(prefix.consumed_lookup_len);
                let spelling_abbreviation =
                    self.is_spelling_abbreviation_view(prefix.input_prefix, &candidate);
                let row = BoundedPrefixStreamRow {
                    fetch_code: prefix.fetch_code.clone(),
                    fetch_order: emission_order,
                    candidate_index,
                    text: candidate.text().to_owned(),
                    consumed_lookup_len: prefix.consumed_lookup_len,
                    consumed_input_len,
                    recompose_on_default: consumed_input_len > 1
                        && !prefix
                            .surface_fetch
                            .as_ref()
                            .is_some_and(|fetch| fetch.abbreviation)
                        && !spelling_abbreviation,
                    deferred_surface_phrase: Self::prefix_fallback_view_is_deferred_surface_phrase(
                        &prefix, &candidate,
                    ),
                    raw_quality: candidate.raw_quality(),
                };
                emission_order += 1;
                let saturated = retain_profile_bounded_prefix_row(
                    &mut fetch_rows,
                    row,
                    per_fetch_cap,
                    &mut deferred_fetch_row_count,
                    |row| row.deferred_surface_phrase,
                );
                truncated |= fetch_rows.len() >= per_fetch_cap;
                if saturated {
                    break;
                }
            }
            self.storage
                .record_exact_lookup(exact_start.elapsed(), exact_candidates);
            let mut saturated = false;
            for row in fetch_rows.drain(..) {
                saturated = retain_profile_bounded_prefix_row(
                    &mut pending,
                    row,
                    pending_cap,
                    &mut deferred_row_count,
                    |row| row.deferred_surface_phrase,
                );
                truncated |= pending.len() >= pending_cap;
                if saturated {
                    break;
                }
            }
            if saturated {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        pending.sort_by(|left, right| {
            left.deferred_surface_phrase
                .cmp(&right.deferred_surface_phrase)
                .then_with(|| right.consumed_input_len.cmp(&left.consumed_input_len))
                .then_with(|| {
                    right
                        .raw_quality
                        .partial_cmp(&left.raw_quality)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| left.fetch_order.cmp(&right.fetch_order))
        });
        let pending_len = pending.len();
        let mut candidates = Vec::new();
        for (index, row) in pending.into_iter().enumerate() {
            let Some(candidate_view) = self
                .storage
                .exact_candidates(&row.fetch_code)
                .nth(row.candidate_index)
            else {
                continue;
            };
            let mut candidate = self.candidate_for_lookup_view(
                &row.fetch_code,
                &candidate_view,
                &lookup_code[..row.consumed_lookup_len],
                None,
                0.0,
            );
            if !seen_texts.insert(candidate.text.clone()) {
                continue;
            }
            candidate.source = CandidateSource::PartialTable {
                consumed: row.consumed_input_len,
                recompose_on_default: row.recompose_on_default,
            };
            candidates.push(candidate);
            if candidates.len() >= limit {
                truncated |= index + 1 < pending_len;
                break;
            }
        }
        (
            PrefixFallbackBatch {
                candidates,
                truncated,
                owns_reachability: saw_prefix,
                span_promotions: HashMap::new(),
            },
            views_visited,
            saw_prefix,
        )
    }

    fn bounded_prefix_fallback_candidates_streaming(
        &self,
        request: BoundedPrefixFallbackStreamingRequest<'_, '_>,
    ) -> (PrefixFallbackBatch, usize, bool) {
        let BoundedPrefixFallbackStreamingRequest {
            input,
            lookup_code,
            filter_by_charset,
            existing_candidates,
            admitted_span_candidates,
            limit,
        } = request;

        let mut seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<HashSet<_>>();
        seen_texts.extend(
            admitted_span_candidates
                .iter()
                .map(|candidate| candidate.text.clone()),
        );
        let full_span_texts: HashSet<&str> = HashSet::new();
        let blocked_texts = seen_texts.clone();
        let mut selected_texts = blocked_texts.clone();
        let target = limit.saturating_add(1);
        let (mut pending, views_visited, exhausted, saw_prefix) = self
            .bounded_prefix_stream_rows_incremental(
                input,
                lookup_code,
                filter_by_charset,
                &full_span_texts,
                &blocked_texts,
                &mut selected_texts,
                target,
            );
        let mut truncated = !exhausted || pending.len() > limit;
        pending.truncate(limit);
        let pending_len = pending.len();
        let mut candidates = Vec::new();
        for (index, pending) in pending.into_iter().enumerate() {
            let Some(candidate_view) = self
                .storage
                .exact_candidates(&pending.fetch_code)
                .nth(pending.candidate_index)
            else {
                continue;
            };
            let mut candidate = self.candidate_for_lookup_view(
                &pending.fetch_code,
                &candidate_view,
                &lookup_code[..pending.consumed_lookup_len],
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
        (
            PrefixFallbackBatch {
                candidates,
                truncated,
                owns_reachability: saw_prefix,
                span_promotions: HashMap::new(),
            },
            views_visited,
            saw_prefix,
        )
    }

    fn materialize_bounded_prefix_fallback_entry(
        &self,
        entry: &PrefixFallbackWindowCacheEntry,
        input: &str,
        lookup_code: &str,
        existing_candidates: &[Candidate],
        _admitted_span_candidates: &[PrefixFallbackSpanView],
        limit: usize,
    ) -> PrefixFallbackBatch {
        let mut seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<HashSet<_>>();
        let full_span_texts: HashSet<&str> = HashSet::new();
        struct CachedPendingPrefixCandidate<'a> {
            view: &'a CachedPrefixFallbackView,
            consumed_input_len: usize,
            recompose_on_default: bool,
        }
        let input_base = input.len().saturating_sub(lookup_code.len());
        let pending = entry
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
            span_promotions: HashMap::new(),
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
        let pending_cap = limit.saturating_add(1);
        let per_fetch_cap = pending_cap;
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
                input,
                lookup_code,
                prefixes,
                filter_by_charset,
                pending_cap,
                key,
            );
            views_visited = built_views;
            if built.rows.len() > PREFIX_FALLBACK_CACHE_MAX_ROWS
                || estimate_prefix_fallback_window_cache_key_bytes(&built.key)
                    > PREFIX_FALLBACK_CACHE_MAX_KEY_BYTES
                || built.rows.iter().any(|row| {
                    estimate_candidate_bytes(&row.candidate) > PREFIX_FALLBACK_CACHE_MAX_ROW_BYTES
                })
                || estimate_prefix_fallback_window_cache_bytes(&built)
                    > PREFIX_FALLBACK_CACHE_MAX_ENTRY_BYTES
            {
                // Keep any prior admitted entry intact. The already-built
                // bounded rows remain request-local; materialize them once
                // instead of discarding the work and rescanning through the
                // streaming fallback.
                let batch = self.materialize_bounded_prefix_fallback_entry(
                    &built,
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
                return Some(batch);
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

    fn direct_prism_prefix_family_exceeds_cache_limit(
        &self,
        lookup_code: &str,
        limit: usize,
    ) -> bool {
        if !self.direct_prism_surface_mapping_current {
            return false;
        }
        let (Some(prism), Some(syllabary_codes)) =
            (self.prism_payload.as_ref(), self.storage.syllabary_codes())
        else {
            return false;
        };
        let mut boundaries = lookup_code
            .char_indices()
            .map(|(index, _)| index)
            .filter(|index| *index > 0)
            .collect::<Vec<_>>();
        boundaries.reverse();
        let mut descriptor_count = 0usize;
        for end in boundaries {
            let outcome: ControlFlow<()> =
                prism.visit_canonical_codes(&lookup_code[..end], syllabary_codes, |_| {
                    descriptor_count = descriptor_count.saturating_add(1);
                    if descriptor_count > limit {
                        ControlFlow::Break(())
                    } else {
                        ControlFlow::Continue(())
                    }
                });
            if matches!(outcome, ControlFlow::Break(())) {
                return true;
            }
        }
        false
    }

    fn prefix_fallback_span_promotions(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        admitted_span_candidates: &[PrefixFallbackSpanView],
    ) -> HashMap<String, CandidateSource> {
        if admitted_span_candidates.is_empty() {
            return HashMap::new();
        }
        let prefixes = self.valid_lookup_prefixes(lookup_code);
        let mut promotions = HashMap::new();
        for candidate in admitted_span_candidates {
            let mut best: Option<&LookupPrefixSpec<'_>> = None;
            for prefix in &prefixes {
                let admitted_by_surface = prefix.surface_fetch.as_ref().is_some_and(|fetch| {
                    canonical_fetch_group(&candidate.raw_comment).as_ref() == fetch.canonical_code
                });
                let admitted_by_raw = prefix.surface_fetch.is_none()
                    && original_code_allows_prefix_fallback(
                        &candidate.raw_comment,
                        prefix.input_prefix,
                    );
                if !(admitted_by_surface || admitted_by_raw)
                    || !self.is_dictionary_text_allowed(&candidate.text)
                    || (filter_by_charset && contains_extended_cjk(&candidate.text))
                {
                    continue;
                }
                if best.map_or(true, |current| {
                    prefix.consumed_lookup_len > current.consumed_lookup_len
                }) {
                    best = Some(prefix);
                }
            }
            let Some(prefix) = best else {
                continue;
            };
            let spelling_abbreviation = candidate.spelling_abbreviation
                || self.spelling_abbreviation_entries.contains(&(
                    prefix.input_prefix.to_owned(),
                    candidate.text.clone(),
                    candidate.raw_comment.clone(),
                ));
            promotions.insert(
                candidate.text.clone(),
                CandidateSource::PartialTable {
                    consumed: input.len(),
                    recompose_on_default: input.len() > 1
                        && !prefix
                            .surface_fetch
                            .as_ref()
                            .is_some_and(|fetch| fetch.abbreviation)
                        && !spelling_abbreviation,
                },
            );
        }
        promotions
    }

    fn prefix_fallback_candidates(
        &self,
        input: &str,
        lookup_code: &str,
        filter_by_charset: bool,
        existing_candidates: &[Candidate],
        admitted_span_candidates: &[PrefixFallbackSpanView],
        request_limit: Option<usize>,
    ) -> PrefixFallbackBatch {
        let fallback_start = crate::m37_metrics_enabled().then(Instant::now);
        let span_promotions = self.prefix_fallback_span_promotions(
            input,
            lookup_code,
            filter_by_charset,
            admitted_span_candidates,
        );
        let bounded_limit = if self.bounds_compact_fallback_expansion() {
            request_limit.filter(|limit| *limit > 0)
        } else {
            None
        };
        if let Some(limit) = bounded_limit {
            if self.sentence_policy == SentencePolicy::LegacyFallback
                && self.prediction_candidate_limit.is_some()
            {
                // The TypeDuck profile keeps its historical page-bounded
                // prediction stream; complete/page-turn requests still use the
                // exact current-head collector below. Standard UpstreamScript
                // schemas, including M59's canonical D-48 lane, never enter
                // this profile-policy path.
                let (mut batch, views_visited, saw_prefix) = self
                    .bounded_profile_prefix_fallback_candidates(
                        BoundedPrefixFallbackStreamingRequest {
                            input,
                            lookup_code,
                            filter_by_charset,
                            existing_candidates,
                            admitted_span_candidates,
                            limit,
                        },
                    );
                if let Some(start) = fallback_start {
                    crate::m37_record_prefix_fallback(
                        start.elapsed(),
                        views_visited,
                        batch.candidates.len(),
                    );
                }
                batch.owns_reachability = saw_prefix;
                batch.span_promotions = span_promotions;
                return batch;
            }
            let mut cache_prefixes = Vec::new();
            let cache_probe_complete = if self.direct_prism_prefix_family_exceeds_cache_limit(
                lookup_code,
                PREFIX_FALLBACK_CACHE_MAX_PREFIXES,
            ) {
                false
            } else {
                matches!(
                    self.visit_valid_lookup_prefixes(lookup_code, |prefix| {
                        cache_prefixes.push(prefix);
                        if cache_prefixes.len() > PREFIX_FALLBACK_CACHE_MAX_PREFIXES {
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    }),
                    ControlFlow::Continue(())
                )
            };
            if cache_probe_complete && cache_prefixes.is_empty() {
                if let Some(start) = fallback_start {
                    crate::m37_record_prefix_fallback(start.elapsed(), 0, 0);
                }
                return PrefixFallbackBatch {
                    candidates: Vec::new(),
                    truncated: false,
                    owns_reachability: false,
                    span_promotions,
                };
            }
            let cache_admitted = span_promotions.is_empty()
                && existing_candidates.is_empty()
                && cache_probe_complete
                && prefix_fallback_cache_key_bytes(&cache_prefixes)
                    <= PREFIX_FALLBACK_CACHE_MAX_KEY_BYTES;
            if cache_admitted {
                if let Some(mut batch) = self.bounded_prefix_fallback_candidates_cached(
                    BoundedPrefixFallbackCacheRequest {
                        input,
                        lookup_code,
                        filter_by_charset,
                        existing_candidates,
                        admitted_span_candidates,
                        prefixes: &cache_prefixes,
                        limit,
                        fallback_start,
                    },
                ) {
                    batch.span_promotions = span_promotions;
                    return batch;
                }
            }
            let (mut batch, views_visited, saw_prefix) = self
                .bounded_prefix_fallback_candidates_streaming(
                    BoundedPrefixFallbackStreamingRequest {
                        input,
                        lookup_code,
                        filter_by_charset,
                        existing_candidates,
                        admitted_span_candidates,
                        limit,
                    },
                );
            if !saw_prefix {
                if let Some(start) = fallback_start {
                    crate::m37_record_prefix_fallback(start.elapsed(), views_visited, 0);
                }
                return PrefixFallbackBatch {
                    candidates: Vec::new(),
                    truncated: false,
                    owns_reachability: false,
                    span_promotions,
                };
            }
            if let Some(start) = fallback_start {
                crate::m37_record_prefix_fallback(
                    start.elapsed(),
                    views_visited,
                    batch.candidates.len(),
                );
            }
            batch.span_promotions = span_promotions;
            return batch;
        }
        let prefixes = self.valid_lookup_prefixes(lookup_code);
        if prefixes.is_empty() {
            if let Some(start) = fallback_start {
                crate::m37_record_prefix_fallback(start.elapsed(), 0, 0);
            }
            return PrefixFallbackBatch {
                candidates: Vec::new(),
                truncated: false,
                owns_reachability: false,
                span_promotions,
            };
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
        seen_texts.extend(
            admitted_span_candidates
                .iter()
                .map(|candidate| candidate.text.clone()),
        );
        let full_span_texts: HashSet<&str> = HashSet::new();
        let mut candidates = Vec::new();
        struct PendingPrefixCandidate<'a> {
            pending: PendingLookupCandidateRef<'a>,
            fetch_order: usize,
            consumed_input_len: usize,
            recompose_on_default: bool,
            deferred_surface_phrase: bool,
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
        for (fetch_order, prefix_spec) in prefixes.iter().enumerate() {
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
                let deferred_surface_phrase =
                    Self::prefix_fallback_view_is_deferred_surface_phrase(prefix_spec, &candidate);
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
                    fetch_order,
                    consumed_input_len,
                    recompose_on_default,
                    deferred_surface_phrase,
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
        order_current_head_chunks(
            &mut pending,
            None,
            |row| row.fetch_order,
            |candidate, current| match candidate
                .deferred_surface_phrase
                .cmp(&current.deferred_surface_phrase)
                .then_with(|| {
                    current
                        .consumed_input_len
                        .cmp(&candidate.consumed_input_len)
                }) {
                Ordering::Less => true,
                Ordering::Greater => false,
                Ordering::Equal => {
                    self.lookup_candidate_ref_comparison_weight(&candidate.pending)
                        > self.lookup_candidate_ref_comparison_weight(&current.pending)
                }
            },
        );
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
            span_promotions,
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

    fn visit_valid_lookup_prefixes<'a, B>(
        &self,
        lookup_code: &'a str,
        mut visitor: impl FnMut(LookupPrefixSpec<'a>) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        let mut boundaries = lookup_code
            .char_indices()
            .map(|(index, _)| index)
            .filter(|index| *index > 0)
            .collect::<Vec<_>>();
        boundaries.reverse();
        for end in boundaries {
            let prefix = &lookup_code[..end];
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
                    visitor(LookupPrefixSpec {
                        input_prefix: prefix,
                        fetch_code: prefix.to_owned(),
                        consumed_lookup_len: end,
                        surface_fetch: None,
                    })?;
                }
            }

            {
                let mut seen_mapped_fetches = HashSet::new();
                let mut visit_fetch = |fetch: LeadingFetchCode| {
                    if direct_normal_groups.contains(&fetch.canonical_code) {
                        return ControlFlow::Continue(());
                    }
                    if self.direct_prism_surface_mapping_current {
                        // A checksum-current compact prism binds every descriptor
                        // to the table syllabary. Prove at least one normal (or
                        // correction-only) group for this boundary before claiming
                        // ownership, then let the consumer's exact scan filter any
                        // malformed sibling descriptor. This avoids re-running the
                        // same binary search for thousands of aliases.
                        let boundary_class_proven = if fetch.injectable {
                            has_valid_normal
                        } else {
                            has_valid_correction
                        };
                        if !boundary_class_proven && !self.storage.has_code(&fetch.fetch_code) {
                            return ControlFlow::Continue(());
                        }
                    } else if !self.storage.has_code(&fetch.fetch_code)
                        || !self
                            .storage
                            .exact_candidates(&fetch.fetch_code)
                            .any(|candidate| leading_candidate_matches_fetch(&candidate, &fetch))
                    {
                        return ControlFlow::Continue(());
                    }
                    if !fetch.injectable {
                        has_valid_correction = true;
                        return ControlFlow::Continue(());
                    }
                    has_valid_normal = true;
                    if seen_mapped_fetches.insert(fetch.clone()) {
                        visitor(LookupPrefixSpec {
                            input_prefix: prefix,
                            fetch_code: fetch.fetch_code.clone(),
                            consumed_lookup_len: end,
                            surface_fetch: Some(fetch),
                        })?;
                    }
                    ControlFlow::Continue(())
                };

                if self.direct_prism_surface_mapping_current {
                    if let (Some(prism), Some(syllabary_codes)) =
                        (self.prism_payload.as_ref(), self.storage.syllabary_codes())
                    {
                        let direct_identity = prism.has_byte_backed_identity_spelling_map();
                        prism.visit_canonical_codes(prefix, syllabary_codes, |lookup| {
                            visit_fetch(LeadingFetchCode {
                                fetch_code: lookup.code.to_owned(),
                                canonical_code: lookup.code.to_owned(),
                                bare_exact: true,
                                injectable: !lookup.correction,
                                abbreviation: lookup.abbreviation,
                                direct_identity,
                            })
                        })?;
                        if !self.spelling_algebra_active {
                            for (_, lookup) in prism
                                .trailing_ascii_digit_prefix_canonical_codes(
                                    prefix,
                                    syllabary_codes,
                                    usize::MAX,
                                )
                                .into_iter()
                                .filter(|(consumed, _)| *consumed == prefix.len())
                            {
                                visit_fetch(LeadingFetchCode {
                                    fetch_code: lookup.code.to_owned(),
                                    canonical_code: lookup.code.to_owned(),
                                    bare_exact: false,
                                    injectable: !lookup.correction,
                                    abbreviation: lookup.abbreviation,
                                    direct_identity,
                                })?;
                            }
                        }
                    } else {
                        for fetch in self.direct_storage_identity_fetch_codes(prefix) {
                            visit_fetch(fetch)?;
                        }
                    }
                } else if !self.spelling_algebra_active || self.leading_fetch_index_seed.is_some() {
                    for fetch in self.leading_surface_fetch_codes(prefix) {
                        visit_fetch(fetch)?;
                    }
                }
            }

            if has_valid_correction && !has_valid_normal {
                break;
            }
        }
        ControlFlow::Continue(())
    }

    fn valid_lookup_prefixes<'a>(&self, lookup_code: &'a str) -> Vec<LookupPrefixSpec<'a>> {
        let mut prefixes = Vec::new();
        let _: ControlFlow<()> = self.visit_valid_lookup_prefixes(lookup_code, |prefix| {
            prefixes.push(prefix);
            ControlFlow::Continue(())
        });
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

    fn assign_mode_stable_ordered_candidate_qualities(&self, candidates: &mut [Candidate]) {
        if self.sentence_policy == SentencePolicy::LegacyFallback && self.prefix_fallback {
            // A bounded legacy fallback must not materialize its complete tail
            // merely to learn a count-dependent denominator. Positional ranks
            // in this open unit band depend only on the visible index, so the
            // bounded and complete shared prefix is field-identical.
            for (index, candidate) in candidates.iter_mut().enumerate() {
                candidate.quality = self.initial_quality + 1.0 / (index as f32 + 2.0);
            }
        } else {
            self.assign_ordered_candidate_qualities(candidates);
        }
    }

    fn assign_upstream_script_candidate_qualities(&self, candidates: &mut [Candidate]) {
        // Page-bounded and complete upstream ScriptTranslation lists must use
        // identical qualities for their shared prefix so downstream producer
        // merges cannot distinguish the typing window from the on-demand full
        // list. A reciprocal positional rank stays inside the translator's
        // open unit band without depending on how many tail rows were fetched.
        for (index, candidate) in candidates.iter_mut().enumerate() {
            candidate.quality = self.initial_quality + 1.0 / (index as f32 + 2.0);
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
                span_promotions: HashMap::new(),
            };
        }

        let Some(lookup_code) = self.lookup_code(input) else {
            return PrefixFallbackBatch {
                candidates: Vec::new(),
                truncated: false,
                owns_reachability: false,
                span_promotions: HashMap::new(),
            };
        };
        if let Some(batch) =
            self.upstream_script_translation(input, filter_by_charset, prefix_fallback_limit)
        {
            return batch;
        }
        let expanded_lookup_codes = self.expanded_lookup_specs(lookup_code);
        let LookupCandidateBatch {
            mut candidates,
            full_input_anchor,
            has_reliable_exact_system_phrase,
            prefix_fallback_span_views,
        } = self.candidates_for_lookup_codes(&expanded_lookup_codes, filter_by_charset);
        let has_correction_lookup = expanded_lookup_codes
            .iter()
            .any(|spec| spec.correction_distance.is_some() || spec.spelling_correction);
        self.enforce_prediction_never_first(&mut candidates);

        let independent_upstream_script =
            self.sentence_policy == SentencePolicy::UpstreamScript && self.enable_sentence;
        let mut upstream_script_candidates = Vec::<Candidate>::new();
        if !has_reliable_exact_system_phrase
            && (independent_upstream_script
                || (self.sentence_policy == SentencePolicy::LegacyFallback
                    && candidates.is_empty()))
        {
            if let Some(model) = self.upstream_sentence_model() {
                let model_start = crate::m37_metrics_enabled().then(Instant::now);
                let mut upstream_candidates = model
                    .candidates_for_input(input)
                    .into_iter()
                    .filter(|candidate| {
                        self.is_dictionary_text_allowed(&candidate.text)
                            && (!filter_by_charset || !contains_extended_cjk(&candidate.text))
                    })
                    .collect::<Vec<_>>();
                if let Some(start) = model_start {
                    crate::m37_record_upstream_sentence_model(
                        start.elapsed(),
                        upstream_candidates.len(),
                    );
                }
                if upstream_candidates.is_empty() {
                    let abbreviation_start = crate::m37_metrics_enabled().then(Instant::now);
                    upstream_candidates = self.abbreviation_sentence_candidates(
                        model,
                        input,
                        usize::MAX,
                        filter_by_charset,
                    );
                    if let Some(start) = abbreviation_start {
                        crate::m37_record_upstream_sentence_model(
                            start.elapsed(),
                            upstream_candidates.len(),
                        );
                    }
                }
                if independent_upstream_script {
                    upstream_script_candidates = upstream_candidates;
                } else {
                    // LegacyFallback retains the historical fallback-only
                    // contract: consult Poet only when the ordinary lookup
                    // stream is empty, and do not merge it into an existing
                    // completion/correction stream.
                    candidates = upstream_candidates;
                }
            }
        }
        let mut sentence_over_completion_floored = false;
        if candidates.is_empty() && upstream_script_candidates.is_empty() && self.enable_sentence {
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
                    span_promotions: self.prefix_fallback_span_promotions(
                        input,
                        lookup_code,
                        filter_by_charset,
                        &prefix_fallback_span_views,
                    ),
                }
            } else {
                self.prefix_fallback_candidates(
                    input,
                    lookup_code,
                    filter_by_charset,
                    candidates.as_slice(),
                    &prefix_fallback_span_views,
                    fallback_room,
                )
            };
            prefix_fallback_owned = prefix_batch.owns_reachability;
            prefix_fallback_truncated |= prefix_batch.truncated;
            let inserted = merge_prefix_fallback_candidates_with_full_input_anchor(
                &mut candidates,
                &prefix_batch.span_promotions,
                prefix_batch.candidates,
                lookup_code,
                full_input_anchor,
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
            self.assign_mode_stable_ordered_candidate_qualities(&mut candidates);
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
        if independent_upstream_script && !upstream_script_candidates.is_empty() {
            candidates = merge_upstream_script_translation_candidates(
                input,
                upstream_script_candidates,
                candidates,
                usize::MAX,
                self.initial_quality,
                !self.comment_format.is_empty(),
                !self.preedit_format.is_empty(),
            );
        }

        PrefixFallbackBatch {
            candidates,
            truncated: prefix_fallback_truncated,
            owns_reachability: prefix_fallback_owned,
            span_promotions: HashMap::new(),
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

        if let Some(result) = self.cached_bounded_upstream_script_result(
            input,
            filter_by_charset,
            limit,
            request.include_debug_full_count,
        ) {
            return result;
        }

        if let Some(batch) = self.upstream_script_translation(input, filter_by_charset, Some(limit))
        {
            let full_count = batch.candidates.len().saturating_add(1);
            crate::m37_record_bounded_iterator(limit, batch.candidates.len(), full_count);
            let result = TranslationResult::bounded(
                batch.candidates,
                full_count,
                request.include_debug_full_count,
            );
            self.cache_bounded_upstream_script_result(
                input,
                filter_by_charset,
                limit,
                request.include_debug_full_count,
                result.clone(),
            );
            return result;
        }

        let Some(lookup_code) = self.lookup_code(input) else {
            let result = TranslationResult::complete(Vec::new());
            self.cache_bounded_upstream_script_result(
                input,
                filter_by_charset,
                limit,
                request.include_debug_full_count,
                result.clone(),
            );
            return result;
        };
        let expanded_lookup_codes = self.expanded_lookup_specs(lookup_code);
        if !self.bounded_request_supported(&expanded_lookup_codes) {
            crate::m37_record_full_list_fallback();
            if !self.bounds_compact_fallback_expansion() {
                let result = TranslationResult::complete(self.translated_candidates_for_segment(
                    input,
                    filter_by_charset,
                    segment_tags,
                ));
                self.cache_bounded_upstream_script_result(
                    input,
                    filter_by_charset,
                    limit,
                    request.include_debug_full_count,
                    result.clone(),
                );
                return result;
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
            let result = TranslationResult::bounded(
                batch.candidates,
                full_count,
                request.include_debug_full_count,
            );
            self.cache_bounded_upstream_script_result(
                input,
                filter_by_charset,
                limit,
                request.include_debug_full_count,
                result.clone(),
            );
            return result;
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

        if let Some(result) = self.cached_bounded_upstream_script_result(
            input,
            filter_by_charset,
            limit,
            request.include_debug_full_count,
        ) {
            scratch.clear();
            return result;
        }

        if let Some(batch) = self.upstream_script_translation(input, filter_by_charset, Some(limit))
        {
            scratch.clear();
            let full_count = batch.candidates.len().saturating_add(1);
            crate::m37_record_bounded_iterator(limit, batch.candidates.len(), full_count);
            let result = TranslationResult::bounded(
                batch.candidates,
                full_count,
                request.include_debug_full_count,
            );
            self.cache_bounded_upstream_script_result(
                input,
                filter_by_charset,
                limit,
                request.include_debug_full_count,
                result.clone(),
            );
            return result;
        }

        let Some(lookup_code) = self.lookup_code(input) else {
            scratch.clear();
            let result = TranslationResult::complete(Vec::new());
            self.cache_bounded_upstream_script_result(
                input,
                filter_by_charset,
                limit,
                request.include_debug_full_count,
                result.clone(),
            );
            return result;
        };
        let expanded_lookup_codes = self.expanded_lookup_specs(lookup_code);
        if !self.bounded_request_supported(&expanded_lookup_codes) {
            scratch.clear();
            crate::m37_record_full_list_fallback();
            if !self.bounds_compact_fallback_expansion() {
                let result = TranslationResult::complete(self.translated_candidates_for_segment(
                    input,
                    filter_by_charset,
                    segment_tags,
                ));
                self.cache_bounded_upstream_script_result(
                    input,
                    filter_by_charset,
                    limit,
                    request.include_debug_full_count,
                    result.clone(),
                );
                return result;
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
            let result = TranslationResult::bounded(
                batch.candidates,
                full_count,
                request.include_debug_full_count,
            );
            self.cache_bounded_upstream_script_result(
                input,
                filter_by_charset,
                limit,
                request.include_debug_full_count,
                result.clone(),
            );
            return result;
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
                            self.entry_weight_domain,
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
                            self.entry_weight_domain,
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

fn merge_prefix_fallback_candidates_with_full_input_anchor(
    candidates: &mut Vec<Candidate>,
    span_promotions: &HashMap<String, CandidateSource>,
    mut prefix_candidates: Vec<Candidate>,
    lookup_code: &str,
    full_input_anchor: Option<usize>,
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
    let insert_after = exact_positions
        .last()
        .copied()
        .into_iter()
        .chain(full_input_anchor.filter(|index| *index < candidates.len()))
        .max();
    let insert_at = insert_after.map_or_else(
        || prefix_fallback_insert_index(candidates, lookup_code),
        |index| index + 1,
    );

    for exact_index in &exact_positions {
        if let Some(source) = span_promotions.get(&candidates[*exact_index].text) {
            candidates[*exact_index].source = source.clone();
        }
    }

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

fn merge_upstream_script_translation_candidates(
    input: &str,
    upstream_candidates: Vec<Candidate>,
    outer_candidates: Vec<Candidate>,
    limit: usize,
    initial_quality: f32,
    preserve_formatted_comment: bool,
    preserve_formatted_preedit: bool,
) -> Vec<Candidate> {
    let mut sentences = Vec::new();
    let mut phrase_stream = Vec::new();
    for candidate in upstream_candidates {
        if candidate.source == CandidateSource::Sentence {
            sentences.push(candidate);
        } else {
            phrase_stream.push(candidate);
        }
    }
    for candidate in &mut phrase_stream {
        let CandidateSource::PartialTable { consumed, .. } = candidate.source else {
            continue;
        };
        let Some(outer) = outer_candidates.iter().find(|outer| {
            outer.text == candidate.text
                && matches!(
                    outer.source,
                    CandidateSource::PartialTable {
                        consumed: outer_consumed,
                        ..
                    } if outer_consumed == consumed
                )
        }) else {
            continue;
        };
        // Keep the model row's oracle span/order, but retain schema formatting
        // already applied by the ordinary translator path.
        if preserve_formatted_comment {
            candidate.comment.clone_from(&outer.comment);
        }
        if preserve_formatted_preedit {
            candidate.preedit.clone_from(&outer.preedit);
        }
    }
    phrase_stream.extend(outer_candidates);
    phrase_stream.sort_by(|left, right| {
        script_translation_consumed_len(right, input.len())
            .cmp(&script_translation_consumed_len(left, input.len()))
    });

    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for candidate in sentences.into_iter().chain(phrase_stream) {
        if !seen.insert(candidate.text.clone()) {
            continue;
        }
        candidates.push(candidate);
        if candidates.len() >= limit {
            break;
        }
    }
    let candidate_count = candidates.len();
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.quality = initial_quality + (candidate_count - index) as f32;
    }
    candidates
}

fn script_translation_consumed_len(candidate: &Candidate, input_len: usize) -> usize {
    match &candidate.source {
        CandidateSource::PartialTable { consumed, .. } => *consumed,
        CandidateSource::Sentence => usize::MAX,
        CandidateSource::Table | CandidateSource::Completion => input_len,
        _ => 0,
    }
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
    let mut count = 0usize;
    let mut alphabetic_after_last_tone = false;
    for ch in code.chars() {
        if ch.is_ascii_digit() {
            count += 1;
            alphabetic_after_last_tone = false;
        } else if count > 0 && ch.is_ascii_alphabetic() {
            // Source table whitespace is normalized out of compiled codes.
            // A Latin tail after a completed toned syllable still represents
            // another syllable (`ngaam1 feel`), even when that tail carries no
            // tone digit. Treating digit count alone as the segmentation count
            // incorrectly admits such two-syllable lettered rows into bare `n`.
            alphabetic_after_last_tone = true;
        }
    }
    count += usize::from(alphabetic_after_last_tone);
    (count > 0).then_some(count)
}

fn first_toned_syllable_prefix(code: &str) -> Option<&str> {
    let mut syllables = source_code_syllables(code);
    let first = syllables.next()?;
    syllables.next().map(|_| first)
}

fn source_code_syllables(code: &str) -> impl Iterator<Item = &str> {
    code.split_inclusive(|ch: char| ch.is_ascii_digit())
        .filter(|syllable| !syllable.is_empty())
}

fn surface_spelling_occurs_after(input: &str, after: usize, spelling: &str) -> bool {
    input.get(after..).is_some_and(|suffix| {
        suffix
            .char_indices()
            .any(|(offset, _)| suffix[offset..].starts_with(spelling))
    })
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

fn combine_duplicate_text_candidates_with_full_input_anchor(
    candidates: Vec<Candidate>,
    full_input_indices: &[usize],
) -> (Vec<Candidate>, Option<usize>) {
    let mut index_by_text = HashMap::<String, usize>::new();
    let mut combined = Vec::<Candidate>::new();
    let mut full_input_indices = full_input_indices.iter().copied().peekable();
    let mut full_input_anchor: Option<usize> = None;
    for (source_index, candidate) in candidates.into_iter().enumerate() {
        let owns_full_input = full_input_indices.peek().copied() == Some(source_index);
        if owns_full_input {
            full_input_indices.next();
        }
        let combined_index = if let Some(index) = index_by_text.get(&candidate.text).copied() {
            let existing = &mut combined[index];
            existing.comment = combine_lookup_comments(&existing.comment, &candidate.comment);
            if candidate.quality > existing.quality {
                existing.quality = candidate.quality;
            }
            index
        } else {
            let index = combined.len();
            index_by_text.insert(candidate.text.clone(), index);
            combined.push(candidate);
            index
        };
        if owns_full_input {
            full_input_anchor =
                Some(full_input_anchor.map_or(combined_index, |anchor| anchor.max(combined_index)));
        }
    }
    debug_assert!(full_input_indices.next().is_none());
    (combined, full_input_anchor)
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
        self.has_upstream_sentence_model()
    }

    fn active_upstream_script_translation(&self, context: &Context) -> bool {
        self.sentence_policy == SentencePolicy::UpstreamScript
            && self.enable_sentence
            && self.has_upstream_sentence_model()
            && self.initial_quality == 0.0
            && self.accepts_segment_tags(&context.segment_tags)
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

    fn clear_ephemeral_runtime_caches(&self) {
        self.bounded_upstream_script_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entries
            .clear();
        *self
            .prefix_fallback_window_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
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
        let bounded_script_cache = self
            .bounded_upstream_script_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        rows.push(MemoryOwnerRow::new(
            "translator.bounded_upstream_script_cache",
            MemoryOwnerClass::HeapOwnedGuarded,
            bounded_script_cache.estimated_retained_bytes(),
            bounded_script_cache.entries.len(),
            "Mutex<VecDeque<BoundedUpstreamScriptCacheEntry>>",
            "ephemeral LRU of at most 64 bounded page prefixes, limited to 64 visible rows, 128 KiB per entry, and 512 KiB total payload; cleared at runtime finalize while immutable translator assets stay warm",
        ));
        drop(bounded_script_cache);
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
        let (lazy_source_bytes, lazy_source_items) =
            self.source_entries.as_ref().map_or((0, 0), |entries| {
                (estimate_source_entry_build_bytes(entries), entries.len())
            });
        rows.push(MemoryOwnerRow::new(
            "translator.upstream_sentence_model_build_source",
            MemoryOwnerClass::HeapOwnedGuarded,
            lazy_source_bytes,
            lazy_source_items,
            "Option<Vec<(String, Candidate)>>",
            if self.has_upstream_sentence_model() {
                "source-order rows retained as the lazy sentence-model build recipe; construction occurs on first translation/warmup and resets after relevant builder reconfiguration"
            } else {
                "source rows are not configured as a sentence-model build recipe"
            },
        ));
        rows.push(MemoryOwnerRow::new(
            "translator.upstream_sentence_model_preset_vocabulary_recipe",
            MemoryOwnerClass::HeapOwnedGuarded,
            estimate_preset_vocabulary_build_bytes(&self.preset_vocabulary),
            self.preset_vocabulary.len(),
            "Vec<PresetVocabularyEntry>",
            "translator-owned normal preset-vocabulary recipe; the initialized model owns its separately packed runtime index",
        ));
        rows.push(MemoryOwnerRow::new(
            "translator.upstream_sentence_model_abbreviation_vocabulary_recipe",
            MemoryOwnerClass::HeapOwnedGuarded,
            estimate_preset_vocabulary_build_bytes(&self.abbreviation_preset_vocabulary),
            self.abbreviation_preset_vocabulary.len(),
            "Vec<PresetVocabularyEntry>",
            "translator-owned abbreviation-vocabulary recipe; the initialized model owns its separately packed runtime index",
        ));
        let model_initialized = self.upstream_sentence_model.get().is_some();
        for mut grammar in self.upstream_sentence_grammar.memory_owner_rows() {
            grammar.owner = format!(
                "translator.upstream_sentence_model_grammar_recipe.{}",
                grammar.owner
            );
            grammar.notes = format!(
                "translator-owned grammar recipe; model initialization clones its owned runtime trie: {}",
                grammar.notes
            );
            rows.push(grammar);
        }
        if self.has_upstream_sentence_model() && !model_initialized {
            if let Some((source, _)) = &self.upstream_sentence_poet_source {
                rows.push(MemoryOwnerRow::new(
                    "translator.upstream_sentence_model_poet_source_recipe",
                    if source.mapping_mode() == "mmap" {
                        MemoryOwnerClass::MmapFileBacked
                    } else {
                        MemoryOwnerClass::HeapOwnedGuarded
                    },
                    source.bytes().len(),
                    1,
                    source.storage_label(),
                    "lazy Poet byte source reported only before model initialization; afterward poet storage reports the same shared source to avoid double counting",
                ));
            }
        }
        if let Some(model) = self.upstream_sentence_model.get() {
            rows.extend(model.memory_owner_rows());
        } else {
            let lazy_note = if self.has_upstream_sentence_model() {
                "configured but not initialized; first translation/warmup owns model construction"
            } else {
                "upstream sentence model not configured for this translator"
            };
            rows.extend([
                MemoryOwnerRow::new(
                    "poet.entries_by_code",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "none",
                    lazy_note,
                ),
                MemoryOwnerRow::new(
                    "poet.lookup_index",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "none",
                    lazy_note,
                ),
                MemoryOwnerRow::new(
                    "poet.abbreviation_vocabulary",
                    MemoryOwnerClass::Shared,
                    0,
                    0,
                    "none",
                    lazy_note,
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
        let (expanded, _, _) =
            algebra.expand_entries_with_normal_codes(entries, TableEntryWeightDomain::Raw);
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
