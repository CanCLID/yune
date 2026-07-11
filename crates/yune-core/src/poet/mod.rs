use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::mem;
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::{
    Candidate, CandidateSource, MemoryOwnerClass, MemoryOwnerRow, PresetVocabularyEntry,
    TableDictionary, TableEntry,
};

mod index;
mod octagram;
mod storage;

use index::{SentenceLookupIndex, SentenceLookupSource, SentencePrefixState};
pub use octagram::{
    encode_octagram_key, OctagramGrammar, OctagramGrammarConfig, OctagramGrammarParseError,
};
pub use storage::{
    build_poet_bin, parse_poet_bin_dictionary_checksum, parse_poet_bin_summary, OwnedPoetBytes,
    PoetBinParseError, PoetBinSectionSummary, PoetBinSummary, PoetByteSource,
};
use storage::{ByteBackedPoetStore, VocabularyCharsRange};

/// Upstream `grammar.h` null-grammar penalty (`ln(1e-6)`) used when no `.gram`
/// language model is configured.
pub const UPSTREAM_NO_GRAMMAR_PENALTY: f64 = -13.815510557964274;
const UPSTREAM_DICT_ENTRY_WEIGHT_SCALE: f64 = 18.420680743952367;

const DEFAULT_SENTENCE_CUTOFF_THRESHOLD: f64 = 0.1;
const SINGULAR_GRAMMAR_BEAM_WIDTH: usize = 7;
// Retain enough direct phrase rows to fill the shipped first-page window. The
// configured `max_homophones` separately controls how many of these rows enter
// Poet; this value is not librime's BeamSearch kMaxLineCandidates.
const MIN_WORD_GRAPH_PHRASE_ROWS_PER_SPAN: usize = 7;
const MAX_DERIVED_PHRASE_CODES_PER_VOCABULARY_ENTRY: usize = 16;
type CharacterCodeCache = HashMap<char, Arc<[String]>>;
const ABBREVIATION_VOCABULARY_RAW_SPAN_BONUS: f64 = 500_000.0;

#[derive(Clone, Copy)]
struct ByteBackedVocabularyChars<'a> {
    storage: &'a ByteBackedPoetStore,
    abbreviation: bool,
    range: VocabularyCharsRange,
}

impl ByteBackedVocabularyChars<'_> {
    fn char_at(&self, index: usize) -> char {
        self.storage
            .vocabulary_char_at(self.abbreviation, self.range, index)
    }

    fn len(&self) -> usize {
        self.range.count as usize
    }
}

struct ByteBackedPhraseDerivation<'a, 'b> {
    chars: ByteBackedVocabularyChars<'a>,
    input: &'b str,
    codes: &'b mut Vec<String>,
    character_code_cache: &'b mut CharacterCodeCache,
}

pub trait Grammar {
    fn query(&self, context: &str, word: &str, is_rear: bool) -> f64;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NullGrammar;

impl Grammar for NullGrammar {
    fn query(&self, _context: &str, _word: &str, _is_rear: bool) -> f64 {
        UPSTREAM_NO_GRAMMAR_PENALTY
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GrammarProvider {
    Null(NullGrammar),
    Octagram(OctagramGrammar),
}

impl Default for GrammarProvider {
    fn default() -> Self {
        Self::Null(NullGrammar)
    }
}

impl Grammar for GrammarProvider {
    fn query(&self, context: &str, word: &str, is_rear: bool) -> f64 {
        match self {
            Self::Null(grammar) => grammar.query(context, word, is_rear),
            Self::Octagram(grammar) => grammar.query(context, word, is_rear),
        }
    }
}

impl From<OctagramGrammar> for GrammarProvider {
    fn from(grammar: OctagramGrammar) -> Self {
        Self::Octagram(grammar)
    }
}

impl GrammarProvider {
    fn scoring_grammar(&self) -> Option<&dyn Grammar> {
        match self {
            Self::Null(_) => None,
            Self::Octagram(grammar) => Some(grammar),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WordGraphEntry {
    pub text: String,
    pub weight: f64,
}

impl WordGraphEntry {
    #[must_use]
    pub fn new(text: impl Into<String>, weight: f64) -> Self {
        Self {
            text: text.into(),
            weight,
        }
    }
}

pub type WordGraph = BTreeMap<usize, BTreeMap<usize, Vec<WordGraphEntry>>>;

#[derive(Clone, Copy, Debug, PartialEq)]
struct BorrowedWordGraphEntry<'a> {
    text: &'a str,
    weight: f64,
}

type BorrowedWordGraph<'a> = BTreeMap<usize, BTreeMap<usize, Vec<BorrowedWordGraphEntry<'a>>>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SentenceCodeSpan {
    pub start: usize,
    pub end: usize,
    pub code: String,
}

impl SentenceCodeSpan {
    #[must_use]
    pub fn new(start: usize, end: usize, code: impl Into<String>) -> Self {
        Self {
            start,
            end,
            code: code.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SentencePath {
    pub text: String,
    pub weight: f64,
    pub word_lengths: Vec<usize>,
}

#[must_use]
pub fn null_grammar_score(entry_weight: f64) -> f64 {
    entry_weight + NullGrammar.query("", "", false)
}

fn upstream_dictionary_weight(raw_weight: f64) -> f64 {
    let weight = if raw_weight > 0.0 {
        raw_weight.ln()
    } else {
        f64::EPSILON.ln()
    };
    weight - UPSTREAM_DICT_ENTRY_WEIGHT_SCALE
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum EntryWeightDomain {
    #[default]
    Raw,
    NaturalLog,
}

// `DictCompiler` serializes every non-positive source weight as
// `ln(DBL_EPSILON)`, narrowed to `float`. The compiled bytes cannot distinguish
// that sentinel from a positive source value that rounds to the same float, so
// preserve librime's ordinary zero-weight exclusion at the exact stored value.
const LIBRIME_NON_POSITIVE_COMPILED_LOG_WEIGHT_BITS: u32 = 0xc210_2cb3;

impl EntryWeightDomain {
    fn dictionary_weight(self, weight: f32) -> f64 {
        match self {
            Self::Raw => upstream_dictionary_weight(f64::from(weight)),
            Self::NaturalLog => f64::from(weight) - UPSTREAM_DICT_ENTRY_WEIGHT_SCALE,
        }
    }

    fn raw_weight(self, weight: f32) -> f64 {
        match self {
            Self::Raw => f64::from(weight),
            Self::NaturalLog => f64::from(weight).exp(),
        }
    }

    fn has_positive_raw_weight(self, weight: f32) -> bool {
        match self {
            Self::Raw => weight > 0.0,
            // `ln(1) == 0`, negative logs for sub-one positive weights, and
            // positive infinity are valid. NaN, negative infinity, and
            // DictCompiler's finite non-positive sentinel are not.
            Self::NaturalLog => {
                weight > f32::NEG_INFINITY
                    && weight.to_bits() != LIBRIME_NON_POSITIVE_COMPILED_LOG_WEIGHT_BITS
            }
        }
    }
}

fn log_sum_exp(weights: impl IntoIterator<Item = f64>) -> f64 {
    let weights = weights.into_iter().collect::<Vec<_>>();
    let max_weight = weights
        .iter()
        .copied()
        .filter(|weight| !weight.is_nan())
        .fold(f64::NEG_INFINITY, f64::max);
    if !max_weight.is_finite() {
        return max_weight;
    }
    max_weight
        + weights
            .into_iter()
            .filter(|weight| !weight.is_nan())
            .map(|weight| (weight - max_weight).exp())
            .sum::<f64>()
            .ln()
}

fn compiled_log_rounding_interval(weight: f32) -> (f64, f64) {
    if !weight.is_finite() {
        return (f64::from(weight), f64::from(weight));
    }
    let bits = weight.to_bits();
    let previous = if weight == 0.0 {
        f32::from_bits(0x8000_0001)
    } else if weight.is_sign_positive() {
        f32::from_bits(bits - 1)
    } else {
        f32::from_bits(bits + 1)
    };
    let next = if weight == 0.0 {
        f32::from_bits(1)
    } else if weight.is_sign_positive() {
        f32::from_bits(bits + 1)
    } else {
        f32::from_bits(bits - 1)
    };
    let weight = f64::from(weight);
    let previous = f64::from(previous);
    let next = f64::from(next);
    let lower = if previous.is_finite() {
        (previous + weight) * 0.5
    } else {
        weight - (next - weight) * 0.5
    };
    let upper = if next.is_finite() {
        (weight + next) * 0.5
    } else {
        weight + (weight - previous) * 0.5
    };
    (lower, upper)
}

fn build_script_encoder_character_codes(
    entries: impl IntoIterator<Item = (char, String, f32)>,
    weight_domain: EntryWeightDomain,
) -> HashMap<char, Vec<String>> {
    // EntryCollector::TranslateWord excludes pronunciations below five percent
    // of the word's total weight before ScriptEncoder expands preset phrases.
    // Keeping this filter in the shared owned/compiled input builder prevents
    // false phrase codes (for example 足/ju at 0%) without duplicating essay
    // entries into the main table.
    let mut weighted_codes = HashMap::<char, BTreeMap<String, f32>>::new();
    for (ch, code, weight) in entries {
        weighted_codes
            .entry(ch)
            .or_default()
            .entry(code)
            .or_insert(weight);
    }

    weighted_codes
        .into_iter()
        .map(|(ch, codes)| {
            let codes = match weight_domain {
                EntryWeightDomain::Raw => {
                    let total_weight = codes.values().copied().sum::<f32>();
                    let minimum_weight = total_weight * 0.05;
                    codes
                        .into_iter()
                        .filter_map(|(code, weight)| (weight >= minimum_weight).then_some(code))
                        .collect()
                }
                EntryWeightDomain::NaturalLog => {
                    // librime stores every collected word entry as
                    // `ln(raw_weight)` in `.table.bin`, while ScriptEncoder's
                    // separate phrase expansion applies its five-percent rule
                    // to the raw collector weights. The compiled table narrows
                    // each logarithm to `f32`, so its exact source value is only
                    // known to lie inside that float's rounding bin.
                    // Retain a reading when any source value represented by the
                    // compiled bytes could have met the 5% boundary. This is the
                    // narrowest comparison that preserves librime's inclusive
                    // exact-boundary behavior without inventing a free-form
                    // tolerance or reconstructing large raw weights.
                    let bounded_codes = codes
                        .into_iter()
                        .filter_map(|(code, weight)| {
                            (!weight.is_nan()).then(|| {
                                let (lower, upper) = compiled_log_rounding_interval(weight);
                                (code, lower, upper)
                            })
                        })
                        .collect::<Vec<_>>();
                    bounded_codes
                        .iter()
                        .enumerate()
                        .filter_map(|(candidate_index, (code, _, candidate_upper))| {
                            let maximum_share_total =
                                log_sum_exp(bounded_codes.iter().enumerate().map(
                                    |(index, (_, lower, _))| {
                                        if index == candidate_index {
                                            *candidate_upper
                                        } else {
                                            *lower
                                        }
                                    },
                                ));
                            (*candidate_upper >= maximum_share_total + 0.05_f64.ln())
                                .then(|| code.clone())
                        })
                        .collect()
                }
            };
            (ch, codes)
        })
        .collect()
}

fn build_model_vocabulary_index(
    vocabulary: &[PresetVocabularyEntry],
    character_codes: &HashMap<char, Vec<String>>,
) -> (Vec<ModelVocabularyEntry>, Vec<(String, usize)>) {
    let vocabulary = vocabulary
        .iter()
        .filter_map(|entry| {
            let chars = entry.text.chars().collect::<Vec<_>>();
            (chars.len() > 1).then(|| ModelVocabularyEntry {
                text: entry.text.clone(),
                chars,
                weight: entry.weight,
            })
        })
        .collect::<Vec<_>>();
    let mut first_codes = Vec::new();
    for (index, entry) in vocabulary.iter().enumerate() {
        let Some(first_char) = entry.chars.first() else {
            continue;
        };
        let Some(codes) = character_codes.get(first_char) else {
            continue;
        };
        for code in codes {
            first_codes.push((code.clone(), index));
        }
    }
    first_codes.sort();
    first_codes.dedup();
    (vocabulary, first_codes)
}

fn vocabulary_indices_for_first_code<'a>(
    vocabulary_first_codes: &'a [(String, usize)],
    code: &str,
) -> &'a [(String, usize)] {
    let start =
        vocabulary_first_codes.partition_point(|(entry_code, _)| entry_code.as_str() < code);
    let end = vocabulary_first_codes[start..]
        .partition_point(|(entry_code, _)| entry_code.as_str() == code)
        + start;
    &vocabulary_first_codes[start..end]
}

#[must_use]
pub fn make_sentence(graph: &WordGraph, total_length: usize) -> Option<SentencePath> {
    make_sentences(graph, total_length, 1).into_iter().next()
}

#[must_use]
pub fn make_sentence_with_grammar(
    graph: &WordGraph,
    total_length: usize,
    grammar: &dyn Grammar,
) -> Option<SentencePath> {
    make_sentences_with_grammar(graph, total_length, 1, grammar)
        .into_iter()
        .next()
}

#[must_use]
pub fn make_sentences(
    graph: &WordGraph,
    total_length: usize,
    max_sentences: usize,
) -> Vec<SentencePath> {
    if max_sentences == 0 {
        return Vec::new();
    }

    make_sentences_by_end(
        graph,
        max_sentences,
        usize::MAX,
        DEFAULT_SENTENCE_CUTOFF_THRESHOLD,
        total_length,
        None,
        true,
    )
    .remove(&total_length)
    .unwrap_or_default()
}

#[must_use]
pub fn make_sentences_with_grammar(
    graph: &WordGraph,
    total_length: usize,
    max_sentences: usize,
    grammar: &dyn Grammar,
) -> Vec<SentencePath> {
    if max_sentences == 0 {
        return Vec::new();
    }

    make_sentences_by_end(
        graph,
        max_sentences,
        usize::MAX,
        DEFAULT_SENTENCE_CUTOFF_THRESHOLD,
        total_length,
        Some(grammar),
        true,
    )
    .remove(&total_length)
    .unwrap_or_default()
}

fn make_sentences_by_end(
    graph: &WordGraph,
    max_sentences: usize,
    max_homophones: usize,
    sentence_cutoff_threshold: f64,
    total_length: usize,
    grammar: Option<&dyn Grammar>,
    skip_direct_full_word: bool,
) -> BTreeMap<usize, Vec<SentencePath>> {
    if max_sentences == 0 {
        return BTreeMap::new();
    }

    collect_sentence_states(
        graph,
        max_sentences,
        max_homophones,
        total_length,
        grammar,
        skip_direct_full_word,
    )
    .into_iter()
    .filter(|(end, _)| *end > 0)
    .map(|(end, states)| {
        (
            end,
            sentence_paths_from_states(states, max_sentences, sentence_cutoff_threshold),
        )
    })
    .collect()
}

fn sentence_paths_vec_by_end_from_states(
    states_by_end: &[Vec<PathState>],
    max_sentences: usize,
    sentence_cutoff_threshold: f64,
) -> Vec<Vec<SentencePath>> {
    states_by_end
        .iter()
        .map(|states| {
            if states.is_empty() {
                Vec::new()
            } else {
                sentence_paths_from_states(states.clone(), max_sentences, sentence_cutoff_threshold)
            }
        })
        .collect()
}

fn make_abbreviation_sentences_by_end(
    graph: &WordGraph,
    max_sentences: usize,
    total_length: usize,
    grammar: Option<&dyn Grammar>,
) -> BTreeMap<usize, Vec<SentencePath>> {
    if max_sentences == 0 {
        return BTreeMap::new();
    }

    collect_abbreviation_sentence_states(graph, max_sentences, total_length, grammar)
        .into_iter()
        .filter(|(end, _)| *end > 0)
        .map(|(end, states)| {
            (
                end,
                abbreviation_sentence_paths_from_states(states, max_sentences, grammar.is_some()),
            )
        })
        .collect()
}

fn collect_sentence_states(
    graph: &WordGraph,
    max_sentences: usize,
    max_homophones: usize,
    total_length: usize,
    grammar: Option<&dyn Grammar>,
    skip_direct_full_word: bool,
) -> BTreeMap<usize, Vec<PathState>> {
    collect_sentence_state_vec(
        graph,
        max_sentences,
        max_homophones,
        total_length,
        grammar,
        skip_direct_full_word,
    )
    .into_iter()
    .enumerate()
    .filter(|(_, states)| !states.is_empty())
    .collect()
}

fn collect_sentence_state_vec(
    graph: &WordGraph,
    max_sentences: usize,
    max_homophones: usize,
    total_length: usize,
    grammar: Option<&dyn Grammar>,
    skip_direct_full_word: bool,
) -> Vec<Vec<PathState>> {
    let record_metrics = cfg!(debug_assertions) && crate::m37_metrics_enabled();
    let mut dp_states_created = 0usize;
    let mut dp_beam_evictions = 0usize;
    let search = SentenceSearchMode::for_options(max_sentences, grammar.is_some());
    let mut states_by_end = vec![Vec::<PathState>::new(); total_length.saturating_add(1)];
    if let Some(start_states) = states_by_end.first_mut() {
        start_states.push(PathState::default());
    }
    for (start, edges) in graph {
        if *start > total_length {
            continue;
        };
        let source_states = std::mem::take(&mut states_by_end[*start]);
        if source_states.is_empty() {
            continue;
        }
        for (end, entries) in edges {
            if *end > total_length {
                continue;
            }
            if skip_direct_full_word && *start == 0 && *end == total_length {
                continue;
            }
            for source in &source_states {
                for entry in entries.iter().take(max_homophones) {
                    let candidate_weight = if let Some(grammar) = grammar {
                        source.weight
                            + entry.weight
                            + grammar.query(
                                &source.grammar_context(),
                                &entry.text,
                                *end == total_length,
                            )
                    } else {
                        source.weight + null_grammar_score(entry.weight)
                    };
                    let next = source.extended(&entry.text, candidate_weight, end - start, grammar);
                    if record_metrics {
                        dp_states_created += 1;
                    }
                    let evicted = insert_sentence_state(&mut states_by_end[*end], next, search);
                    if record_metrics && evicted {
                        dp_beam_evictions += 1;
                    }
                }
            }
        }
        states_by_end[*start] = source_states;
    }

    if record_metrics {
        crate::m37_record_upstream_sentence_model_dp(dp_states_created, dp_beam_evictions);
    }
    states_by_end
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SentenceSearchMode {
    SingularNull,
    SingularGrammar,
    Plural { beam_width: usize },
}

impl SentenceSearchMode {
    fn for_options(max_sentences: usize, has_grammar: bool) -> Self {
        match (max_sentences, has_grammar) {
            (1, false) => Self::SingularNull,
            (1, true) => Self::SingularGrammar,
            _ => Self::Plural {
                beam_width: max_sentences.saturating_mul(3),
            },
        }
    }

    const fn beam_width(self) -> usize {
        match self {
            Self::SingularNull => 1,
            Self::SingularGrammar => SINGULAR_GRAMMAR_BEAM_WIDTH,
            Self::Plural { beam_width } => beam_width,
        }
    }
}

fn collect_abbreviation_sentence_states(
    graph: &WordGraph,
    max_sentences: usize,
    total_length: usize,
    grammar: Option<&dyn Grammar>,
) -> BTreeMap<usize, Vec<PathState>> {
    let mut states: BTreeMap<usize, Vec<PathState>> = BTreeMap::new();
    states.insert(0, vec![PathState::default()]);
    for (start, edges) in graph {
        let Some(source_states) = states.get(start).cloned() else {
            continue;
        };
        for (end, entries) in edges {
            for source in &source_states {
                for entry in entries {
                    let mut next = source.clone();
                    if let Some(grammar) = grammar {
                        next.weight += entry.weight
                            + grammar.query(
                                &source.grammar_context(),
                                &entry.text,
                                *end == total_length,
                            );
                        next.push_word(&entry.text);
                    } else {
                        next.weight += null_grammar_score(entry.weight);
                    }
                    next.text.push_str(&entry.text);
                    next.word_lengths.push(end - start);
                    insert_abbreviation_state(
                        states.entry(*end).or_default(),
                        next,
                        max_sentences * 3,
                    );
                }
            }
        }
    }

    states
}

fn sentence_paths_from_states(
    mut states: Vec<PathState>,
    max_sentences: usize,
    sentence_cutoff_threshold: f64,
) -> Vec<SentencePath> {
    // All search modes retain traversal order for equal weights. `sort_by` is
    // stable, so this final ranking does not add Yune-specific lexical or
    // partition tie-breakers to pinned Poet's CompareWeight contract.
    states.sort_by(|left, right| {
        right
            .weight
            .partial_cmp(&left.weight)
            .unwrap_or(Ordering::Equal)
    });
    let mut paths = Vec::new();
    for state in states {
        if paths
            .iter()
            .any(|path: &SentencePath| path.text == state.text)
        {
            continue;
        }
        let path = SentencePath {
            text: state.text,
            weight: state.weight,
            word_lengths: state.word_lengths.into_vec(),
        };
        if max_sentences > 1 {
            if let Some(previous) = paths.last() {
                let denominator = previous.weight.abs();
                let relative_gap = if denominator == 0.0 {
                    if path.weight == previous.weight {
                        0.0
                    } else {
                        f64::INFINITY
                    }
                } else {
                    (path.weight - previous.weight).abs() / denominator
                };
                let accepted_after_first = paths.len().saturating_sub(1) as i32;
                let accelerated_threshold = sentence_cutoff_threshold
                    * (1.0 - 1.0 / max_sentences as f64).powi(accepted_after_first);
                if relative_gap > accelerated_threshold {
                    break;
                }
            }
        }
        paths.push(path);
        if paths.len() == max_sentences {
            break;
        }
    }
    paths
}

fn abbreviation_sentence_paths_from_states(
    mut states: Vec<PathState>,
    max_sentences: usize,
    dedupe_text: bool,
) -> Vec<SentencePath> {
    states.sort_by(compare_abbreviation_path_state);
    if !dedupe_text {
        return states
            .into_iter()
            .take(max_sentences)
            .map(|state| SentencePath {
                text: state.text,
                weight: state.weight,
                word_lengths: state.word_lengths.into_vec(),
            })
            .collect();
    }
    let mut paths = Vec::new();
    for state in states {
        if paths
            .iter()
            .any(|path: &SentencePath| path.text == state.text)
        {
            continue;
        }
        paths.push(SentencePath {
            text: state.text,
            weight: state.weight,
            word_lengths: state.word_lengths.into_vec(),
        });
        if paths.len() == max_sentences {
            break;
        }
    }
    paths
}

#[derive(Clone, Debug, Default)]
struct PathState {
    text: String,
    weight: f64,
    word_lengths: PathWordLengths,
    recent_words: Vec<String>,
}

impl PathState {
    fn grammar_context(&self) -> String {
        self.recent_words.concat()
    }

    fn extended(
        &self,
        word: &str,
        weight: f64,
        word_length: usize,
        grammar: Option<&dyn Grammar>,
    ) -> Self {
        let mut text = String::with_capacity(self.text.len() + word.len());
        text.push_str(&self.text);
        text.push_str(word);

        let word_lengths = self.word_lengths.extended(word_length);

        let mut recent_words = if grammar.is_some() {
            self.recent_words.clone()
        } else {
            Vec::new()
        };
        if grammar.is_some() {
            push_recent_word(&mut recent_words, word);
        }

        Self {
            text,
            weight,
            word_lengths,
            recent_words,
        }
    }

    fn push_word(&mut self, word: &str) {
        push_recent_word(&mut self.recent_words, word);
    }
}

const PATH_WORD_LENGTHS_INLINE_CAPACITY: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
enum PathWordLengths {
    Inline {
        len: u8,
        values: [usize; PATH_WORD_LENGTHS_INLINE_CAPACITY],
    },
    Heap(Vec<usize>),
}

impl Default for PathWordLengths {
    fn default() -> Self {
        Self::Inline {
            len: 0,
            values: [0; PATH_WORD_LENGTHS_INLINE_CAPACITY],
        }
    }
}

impl PathWordLengths {
    fn len(&self) -> usize {
        match self {
            Self::Inline { len, .. } => usize::from(*len),
            Self::Heap(values) => values.len(),
        }
    }

    fn as_slice(&self) -> &[usize] {
        match self {
            Self::Inline { len, values } => &values[..usize::from(*len)],
            Self::Heap(values) => values,
        }
    }

    fn push(&mut self, word_length: usize) {
        match self {
            Self::Inline { len, values }
                if usize::from(*len) < PATH_WORD_LENGTHS_INLINE_CAPACITY =>
            {
                values[usize::from(*len)] = word_length;
                *len += 1;
            }
            Self::Inline { len, values } => {
                let mut heap = Vec::with_capacity(usize::from(*len) + 1);
                heap.extend_from_slice(&values[..usize::from(*len)]);
                heap.push(word_length);
                *self = Self::Heap(heap);
            }
            Self::Heap(values) => values.push(word_length),
        }
    }

    fn extended(&self, word_length: usize) -> Self {
        let mut next = self.clone();
        next.push(word_length);
        next
    }

    fn into_vec(self) -> Vec<usize> {
        match self {
            Self::Inline { len, values } => values[..usize::from(len)].to_vec(),
            Self::Heap(values) => values,
        }
    }
}

impl PartialOrd for PathWordLengths {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathWordLengths {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

fn push_recent_word(recent_words: &mut Vec<String>, word: &str) {
    if recent_words.len() == 2 {
        recent_words.remove(0);
    }
    recent_words.push(word.to_owned());
}

fn insert_sentence_state(
    states: &mut Vec<PathState>,
    candidate: PathState,
    search: SentenceSearchMode,
) -> bool {
    let duplicate_index = match search {
        SentenceSearchMode::SingularNull => states.first().map(|_| 0),
        SentenceSearchMode::SingularGrammar => {
            let candidate_last_word = candidate.recent_words.last();
            states
                .iter()
                .position(|existing| existing.recent_words.last() == candidate_last_word)
        }
        SentenceSearchMode::Plural { .. } => states
            .iter()
            .position(|existing| existing.text == candidate.text),
    };
    if let Some(existing_index) = duplicate_index {
        // DynamicProgramming, singular grammar's last-word map, and plural
        // text-hash dedup all replace only on a strictly higher weight. Equal
        // weights retain the first line encountered in graph traversal.
        if candidate.weight <= states[existing_index].weight {
            return false;
        }
        states.remove(existing_index);
    }

    let index = states
        .iter()
        .position(|existing| candidate.weight > existing.weight)
        .unwrap_or(states.len());
    states.insert(index, candidate);
    if states.len() > search.beam_width() {
        states.pop();
        return true;
    }
    false
}

fn insert_abbreviation_state(states: &mut Vec<PathState>, candidate: PathState, beam_width: usize) {
    if let Some(existing_index) = states
        .iter()
        .position(|existing| has_same_future_grammar_state(existing, &candidate))
    {
        if compare_abbreviation_path_state(&candidate, &states[existing_index]) == Ordering::Less {
            states.remove(existing_index);
        } else {
            return;
        }
    }
    let index = states
        .binary_search_by(|existing| compare_abbreviation_path_state(existing, &candidate))
        .unwrap_or_else(|index| index);
    states.insert(index, candidate);
    if states.len() > beam_width {
        states.pop();
    }
}

fn has_same_future_grammar_state(left: &PathState, right: &PathState) -> bool {
    left.text == right.text && left.recent_words == right.recent_words
}

fn compare_abbreviation_path_state(left: &PathState, right: &PathState) -> Ordering {
    left.word_lengths
        .len()
        .cmp(&right.word_lengths.len())
        .then_with(|| {
            singleton_word_count(left.word_lengths.as_slice())
                .cmp(&singleton_word_count(right.word_lengths.as_slice()))
        })
        .then_with(|| right.word_lengths.cmp(&left.word_lengths))
        .then_with(|| {
            right
                .weight
                .partial_cmp(&left.weight)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.text.cmp(&right.text))
}

fn singleton_word_count(word_lengths: &[usize]) -> usize {
    word_lengths.iter().filter(|length| **length == 1).count()
}

fn abbreviation_synthesized_sentence(
    graph: &WordGraph,
    first_end: usize,
    total_end: usize,
    grammar: Option<&dyn Grammar>,
) -> Option<SentencePath> {
    let mut segments = vec![(0usize, first_end)];
    segments.extend(abbreviation_best_suffix_partition(
        graph, first_end, total_end,
    )?);
    let mut text = String::new();
    let mut weight = 0.0;
    let mut word_lengths = Vec::with_capacity(segments.len());
    let mut recent_words = Vec::<String>::new();
    for (start, end) in segments {
        let entry = graph.get(&start)?.get(&end)?.first()?;
        text.push_str(&entry.text);
        if let Some(grammar) = grammar {
            weight +=
                entry.weight + grammar.query(&recent_words.concat(), &entry.text, end == total_end);
        } else {
            weight += null_grammar_score(entry.weight);
        }
        word_lengths.push(end - start);
        if grammar.is_some() {
            if recent_words.len() == 2 {
                recent_words.remove(0);
            }
            recent_words.push(entry.text.clone());
        }
    }
    Some(SentencePath {
        text,
        weight,
        word_lengths,
    })
}

fn abbreviation_best_suffix_partition(
    graph: &WordGraph,
    start: usize,
    total_end: usize,
) -> Option<Vec<(usize, usize)>> {
    if start == total_end {
        return Some(Vec::new());
    }
    let mut candidates = Vec::new();
    collect_abbreviation_suffix_partitions(
        graph,
        start,
        total_end,
        &mut Vec::new(),
        &mut candidates,
    );
    candidates
        .into_iter()
        .min_by(|left, right| compare_abbreviation_partition(left, right))
}

fn collect_abbreviation_suffix_partitions(
    graph: &WordGraph,
    start: usize,
    total_end: usize,
    current: &mut Vec<(usize, usize)>,
    candidates: &mut Vec<Vec<(usize, usize)>>,
) {
    if start == total_end {
        candidates.push(current.clone());
        return;
    }
    for len in 1..=4 {
        let end = start + len;
        if end > total_end {
            break;
        }
        if !graph
            .get(&start)
            .is_some_and(|edges| edges.contains_key(&end))
        {
            continue;
        }
        current.push((start, end));
        collect_abbreviation_suffix_partitions(graph, end, total_end, current, candidates);
        current.pop();
    }
}

fn compare_abbreviation_partition(left: &[(usize, usize)], right: &[(usize, usize)]) -> Ordering {
    let left_lengths = partition_lengths(left);
    let right_lengths = partition_lengths(right);
    left.len()
        .cmp(&right.len())
        .then_with(|| {
            singleton_word_count(&left_lengths).cmp(&singleton_word_count(&right_lengths))
        })
        .then_with(|| partition_spread(&left_lengths).cmp(&partition_spread(&right_lengths)))
        .then_with(|| left_lengths.cmp(&right_lengths))
}

fn partition_lengths(partition: &[(usize, usize)]) -> Vec<usize> {
    partition.iter().map(|(start, end)| end - start).collect()
}

fn partition_spread(lengths: &[usize]) -> usize {
    let Some(min) = lengths.iter().min() else {
        return 0;
    };
    let Some(max) = lengths.iter().max() else {
        return 0;
    };
    max - min
}

#[derive(Clone, Debug)]
pub struct UpstreamSentenceModel {
    storage: PoetModelStorage,
    lookup_index: SentenceLookupIndex,
    max_candidates: usize,
    // ScriptTranslator configuration. These are intentionally independent of
    // the visible candidate window: upstream defaults to one sentence and one
    // homophone per graph span, then drains an unbounded phrase iterator.
    max_sentences: usize,
    max_homophones: usize,
    sentence_cutoff_threshold: f64,
    excluded_texts: HashSet<String>,
    grammar: GrammarProvider,
}

impl Default for UpstreamSentenceModel {
    fn default() -> Self {
        Self {
            storage: PoetModelStorage::default(),
            lookup_index: SentenceLookupIndex::default(),
            max_candidates: 1,
            max_sentences: 1,
            max_homophones: 1,
            sentence_cutoff_threshold: DEFAULT_SENTENCE_CUTOFF_THRESHOLD,
            excluded_texts: HashSet::new(),
            grammar: GrammarProvider::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct UpstreamSentenceScratch {
    input: String,
    max_candidates: usize,
    states_by_end: Vec<Vec<PathState>>,
    sentence_paths_by_end: Vec<Vec<SentencePath>>,
    phrase_candidates: Vec<Candidate>,
    prefix_states_by_start: Vec<Option<SentencePrefixState>>,
    exact_spans_by_start: Vec<Vec<CachedSentenceCodeSpan>>,
}

impl UpstreamSentenceScratch {
    pub(crate) fn clear(&mut self) {
        self.input.clear();
        self.max_candidates = 0;
        self.states_by_end.clear();
        self.sentence_paths_by_end.clear();
        self.phrase_candidates.clear();
        self.prefix_states_by_start.clear();
        self.exact_spans_by_start.clear();
    }

    fn is_ready_for(&self, input: &str, max_candidates: usize) -> bool {
        !self.input.is_empty()
            && self.max_candidates == max_candidates
            && input.starts_with(&self.input)
            && self.states_by_end.len() == self.input.len().saturating_add(1)
            && self.sentence_paths_by_end.len() == self.states_by_end.len()
            && self.prefix_states_by_start.len() == self.states_by_end.len()
            && self.exact_spans_by_start.len() == self.states_by_end.len()
            && input.is_char_boundary(self.input.len())
    }
}

#[derive(Clone, Debug)]
struct CachedSentenceCodeSpan {
    end: usize,
    end_index: usize,
    entries: Range<usize>,
}

#[derive(Clone, Debug, Default)]
enum PoetModelStorage {
    #[default]
    Empty,
    Owned(Box<OwnedPoetModelStorage>),
    ByteBacked(Arc<ByteBackedPoetStore>),
}

#[derive(Clone, Debug, Default)]
struct OwnedPoetModelStorage {
    entries_by_code: Vec<ModelEntry>,
    entry_texts: ModelStringPool,
    entry_codes: ModelStringPool,
    entry_weight_domain: EntryWeightDomain,
    vocabulary: Vec<ModelVocabularyEntry>,
    vocabulary_first_codes: Vec<(String, usize)>,
    abbreviation_vocabulary: Vec<ModelVocabularyEntry>,
    abbreviation_vocabulary_first_codes: Vec<(String, usize)>,
    character_codes: HashMap<char, Vec<String>>,
    abbreviation_character_codes: HashMap<char, Vec<String>>,
}

impl SentenceLookupSource for PoetModelStorage {
    fn entry_count(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Owned(storage) => storage.entries_by_code.len(),
            Self::ByteBacked(storage) => storage.entry_count(),
        }
    }

    fn entry_code_id(&self, index: usize) -> u32 {
        match self {
            Self::Empty => 0,
            Self::Owned(storage) => storage.entries_by_code[index].code_id,
            Self::ByteBacked(storage) => storage.entry_code_id(index),
        }
    }

    fn entry_code(&self, index: usize) -> &str {
        match self {
            Self::Empty => "",
            Self::Owned(storage) => storage.entries_by_code[index].code(&storage.entry_codes),
            Self::ByteBacked(storage) => storage.entry_code(index),
        }
    }
}

impl SentenceLookupSource for OwnedPoetModelStorage {
    fn entry_count(&self) -> usize {
        self.entries_by_code.len()
    }

    fn entry_code_id(&self, index: usize) -> u32 {
        self.entries_by_code[index].code_id
    }

    fn entry_code(&self, index: usize) -> &str {
        self.entries_by_code[index].code(&self.entry_codes)
    }
}

impl PoetModelStorage {
    fn entry_text(&self, index: usize) -> &str {
        match self {
            Self::Empty => "",
            Self::Owned(storage) => storage.entries_by_code[index].text(&storage.entry_texts),
            Self::ByteBacked(storage) => storage.entry_text(index),
        }
    }

    fn entry_weight(&self, index: usize) -> f32 {
        match self {
            Self::Empty => 0.0,
            Self::Owned(storage) => storage.entries_by_code[index].weight,
            Self::ByteBacked(storage) => storage.entry_weight(index),
        }
    }

    fn entries_for_code_range(
        &self,
        lookup_index: &SentenceLookupIndex,
        code: &str,
    ) -> Option<Range<usize>> {
        match self {
            Self::ByteBacked(storage) => storage.entries_for_code_range(code),
            _ => lookup_index.entries_for_code_range(self, code),
        }
    }

    fn vocabulary_indices_for_first_code(&self, abbreviation: bool, code: &str) -> Vec<usize> {
        match self {
            Self::Empty => Vec::new(),
            Self::Owned(storage) => vocabulary_indices_for_first_code(
                storage.vocabulary_first_codes(abbreviation),
                code,
            )
            .iter()
            .map(|(_, index)| *index)
            .collect(),
            Self::ByteBacked(storage) => {
                storage.vocabulary_indices_for_first_code(abbreviation, code)
            }
        }
    }

    fn vocabulary_text(&self, abbreviation: bool, index: usize) -> &str {
        match self {
            Self::Empty => "",
            Self::Owned(storage) => storage.vocabulary(abbreviation)[index].text.as_str(),
            Self::ByteBacked(storage) => storage.vocabulary_text(abbreviation, index),
        }
    }

    fn vocabulary_weight(&self, abbreviation: bool, index: usize) -> f32 {
        match self {
            Self::Empty => 0.0,
            Self::Owned(storage) => storage.vocabulary(abbreviation)[index].weight,
            Self::ByteBacked(storage) => storage.vocabulary_weight(abbreviation, index),
        }
    }

    fn vocabulary_chars_into(&self, abbreviation: bool, index: usize, out: &mut Vec<char>) {
        out.clear();
        match self {
            Self::Empty => {}
            Self::Owned(storage) => {
                out.extend_from_slice(&storage.vocabulary(abbreviation)[index].chars)
            }
            Self::ByteBacked(storage) => storage.vocabulary_chars_into(abbreviation, index, out),
        }
    }

    fn character_codes(&self, abbreviation: bool, ch: char) -> Vec<&str> {
        match self {
            Self::Empty => Vec::new(),
            Self::Owned(storage) => storage
                .character_codes(abbreviation)
                .get(&ch)
                .map(|codes| codes.iter().map(String::as_str).collect())
                .unwrap_or_default(),
            Self::ByteBacked(storage) => storage.character_codes(abbreviation, ch),
        }
    }

    fn memory_owner_rows(&self, lookup_index: &SentenceLookupIndex) -> Vec<MemoryOwnerRow> {
        match self {
            Self::Empty => Vec::new(),
            Self::Owned(storage) => storage.memory_owner_rows(lookup_index),
            Self::ByteBacked(storage) => storage.memory_owner_rows(),
        }
    }
}

impl OwnedPoetModelStorage {
    fn dictionary_weight(&self, weight: f32) -> f64 {
        self.entry_weight_domain.dictionary_weight(weight)
    }

    fn raw_weight(&self, weight: f32) -> f64 {
        self.entry_weight_domain.raw_weight(weight)
    }

    fn vocabulary(&self, abbreviation: bool) -> &[ModelVocabularyEntry] {
        if abbreviation {
            &self.abbreviation_vocabulary
        } else {
            &self.vocabulary
        }
    }

    fn vocabulary_first_codes(&self, abbreviation: bool) -> &[(String, usize)] {
        if abbreviation {
            &self.abbreviation_vocabulary_first_codes
        } else {
            &self.vocabulary_first_codes
        }
    }

    fn character_codes(&self, abbreviation: bool) -> &HashMap<char, Vec<String>> {
        if abbreviation {
            &self.abbreviation_character_codes
        } else {
            &self.character_codes
        }
    }

    fn normal_phrase_character_codes(
        &self,
        _grammar: &GrammarProvider,
    ) -> &HashMap<char, Vec<String>> {
        // ScriptEncoder's five-percent pronunciation filter is independent of
        // whether Poet later scores the graph with null or octagram grammar.
        &self.character_codes
    }

    fn memory_owner_rows(&self, lookup_index: &SentenceLookupIndex) -> Vec<MemoryOwnerRow> {
        vec![
            MemoryOwnerRow::new(
                "poet.entries_by_code",
                MemoryOwnerClass::HeapOwnedReducible,
                estimate_model_entries_bytes(
                    &self.entries_by_code,
                    &self.entry_texts,
                    &self.entry_codes,
                ),
                self.entries_by_code.len(),
                "Vec<ModelEntry>",
                "sentence model entries cloned from table rows",
            ),
            MemoryOwnerRow::new(
                "poet.lookup_index",
                MemoryOwnerClass::HeapOwnedGuarded,
                lookup_index.estimated_retained_bytes(),
                lookup_index.range_count(),
                "SentenceLookupIndex",
                "sorted code-range index used by M40 sentence lookup",
            ),
            MemoryOwnerRow::new(
                "poet.vocabulary",
                MemoryOwnerClass::HeapOwnedReducible,
                estimate_model_vocabulary_bytes(&self.vocabulary).saturating_add(
                    estimate_string_usize_pairs_bytes(&self.vocabulary_first_codes),
                ),
                self.vocabulary.len(),
                "Vec<ModelVocabularyEntry>",
                "normal preset vocabulary used by upstream sentence graph",
            ),
            MemoryOwnerRow::new(
                "poet.abbreviation_vocabulary",
                MemoryOwnerClass::HeapOwnedReducible,
                estimate_model_vocabulary_bytes(&self.abbreviation_vocabulary).saturating_add(
                    estimate_string_usize_pairs_bytes(&self.abbreviation_vocabulary_first_codes),
                ),
                self.abbreviation_vocabulary.len(),
                "Vec<ModelVocabularyEntry>",
                "abbreviation-only vocabulary used by M42 guard rows",
            ),
        ]
    }
}

impl UpstreamSentenceModel {
    #[must_use]
    pub fn from_dictionary(dictionary: &TableDictionary, max_candidates: usize) -> Self {
        Self::from_entries(
            dictionary.entries(),
            dictionary.preset_vocabulary_entries(),
            max_candidates,
        )
    }

    #[must_use]
    pub fn from_entries(
        entries: &[TableEntry],
        vocabulary: &[PresetVocabularyEntry],
        max_candidates: usize,
    ) -> Self {
        Self::from_model_entries(
            entries.iter().map(ModelEntry::from_table_entry),
            vocabulary,
            vocabulary,
            max_candidates,
            EntryWeightDomain::Raw,
        )
    }

    #[must_use]
    pub fn from_table_entries(
        entries: impl IntoIterator<Item = TableEntry>,
        vocabulary: &[PresetVocabularyEntry],
        max_candidates: usize,
    ) -> Self {
        Self::from_table_entries_with_abbreviation_vocabulary(
            entries,
            vocabulary,
            vocabulary,
            max_candidates,
        )
    }

    #[must_use]
    pub fn from_table_entries_with_abbreviation_vocabulary(
        entries: impl IntoIterator<Item = TableEntry>,
        vocabulary: &[PresetVocabularyEntry],
        abbreviation_vocabulary: &[PresetVocabularyEntry],
        max_candidates: usize,
    ) -> Self {
        Self::from_model_entries(
            entries.into_iter().map(ModelEntry::from_owned_table_entry),
            vocabulary,
            abbreviation_vocabulary,
            max_candidates,
            EntryWeightDomain::Raw,
        )
    }

    #[must_use]
    pub(crate) fn from_natural_log_table_entries(
        entries: impl IntoIterator<Item = TableEntry>,
        vocabulary: &[PresetVocabularyEntry],
        max_candidates: usize,
    ) -> Self {
        Self::from_natural_log_table_entries_with_abbreviation_vocabulary(
            entries,
            vocabulary,
            vocabulary,
            max_candidates,
        )
    }

    #[must_use]
    pub(crate) fn from_natural_log_table_entries_with_abbreviation_vocabulary(
        entries: impl IntoIterator<Item = TableEntry>,
        vocabulary: &[PresetVocabularyEntry],
        abbreviation_vocabulary: &[PresetVocabularyEntry],
        max_candidates: usize,
    ) -> Self {
        Self::from_model_entries(
            entries.into_iter().map(ModelEntry::from_owned_table_entry),
            vocabulary,
            abbreviation_vocabulary,
            max_candidates,
            EntryWeightDomain::NaturalLog,
        )
    }

    pub fn from_poet_bin_source(
        source: Arc<dyn PoetByteSource>,
        expected_dictionary_checksum: u32,
        max_candidates: usize,
    ) -> Result<Self, PoetBinParseError> {
        let storage = PoetModelStorage::ByteBacked(Arc::new(ByteBackedPoetStore::from_source(
            source,
            expected_dictionary_checksum,
        )?));
        Ok(Self {
            storage,
            lookup_index: SentenceLookupIndex::default(),
            max_candidates: max_candidates.max(1),
            max_sentences: 1,
            max_homophones: 1,
            sentence_cutoff_threshold: DEFAULT_SENTENCE_CUTOFF_THRESHOLD,
            excluded_texts: HashSet::new(),
            grammar: GrammarProvider::default(),
        })
    }

    fn from_model_entries(
        entries: impl IntoIterator<Item = OwnedModelEntry>,
        vocabulary: &[PresetVocabularyEntry],
        abbreviation_vocabulary: &[PresetVocabularyEntry],
        max_candidates: usize,
        entry_weight_domain: EntryWeightDomain,
    ) -> Self {
        let mut owned_entries = Vec::new();
        let mut script_encoder_codes = Vec::new();
        let mut abbreviation_character_codes: HashMap<char, Vec<String>> = HashMap::new();
        for entry in entries {
            if entry.code.is_empty() {
                continue;
            }
            let mut chars = entry.text.chars();
            if let Some(ch) = chars.next() {
                if chars.next().is_none() {
                    script_encoder_codes.push((ch, entry.code.clone(), entry.weight));
                    if entry_weight_domain.has_positive_raw_weight(entry.weight) {
                        abbreviation_character_codes
                            .entry(ch)
                            .or_default()
                            .push(entry.code.clone());
                    }
                }
            }
            owned_entries.push(entry);
        }
        let character_codes =
            build_script_encoder_character_codes(script_encoder_codes, entry_weight_domain);
        for codes in abbreviation_character_codes.values_mut() {
            codes.sort();
            codes.dedup();
        }
        owned_entries.sort_by(compare_model_entry_by_code);
        let (entries_by_code, entry_texts, entry_codes) = pack_owned_model_entries(owned_entries);
        let (vocabulary, vocabulary_first_codes) =
            build_model_vocabulary_index(vocabulary, &character_codes);
        let (abbreviation_vocabulary, abbreviation_vocabulary_first_codes) =
            build_model_vocabulary_index(abbreviation_vocabulary, &abbreviation_character_codes);
        let storage = PoetModelStorage::Owned(Box::new(OwnedPoetModelStorage {
            entries_by_code,
            entry_texts,
            entry_codes,
            entry_weight_domain,
            vocabulary,
            vocabulary_first_codes,
            abbreviation_vocabulary,
            abbreviation_vocabulary_first_codes,
            character_codes,
            abbreviation_character_codes,
        }));
        let index_start = crate::m37_metrics_enabled().then(Instant::now);
        let lookup_index = SentenceLookupIndex::build(&storage);
        if let Some(index_start) = index_start {
            crate::m37_record_upstream_sentence_model_index_build(index_start.elapsed());
        }
        Self {
            storage,
            lookup_index,
            max_candidates: max_candidates.max(1),
            max_sentences: 1,
            max_homophones: 1,
            sentence_cutoff_threshold: DEFAULT_SENTENCE_CUTOFF_THRESHOLD,
            excluded_texts: HashSet::new(),
            grammar: GrammarProvider::default(),
        }
    }

    #[must_use]
    pub fn with_grammar(mut self, grammar: impl Into<GrammarProvider>) -> Self {
        self.grammar = grammar.into();
        self
    }

    #[must_use]
    pub fn with_script_translation_limits(
        mut self,
        max_sentences: usize,
        max_homophones: usize,
    ) -> Self {
        self.max_sentences = max_sentences.clamp(1, 100);
        self.max_homophones = max_homophones;
        self
    }

    #[must_use]
    pub fn with_sentence_cutoff_threshold(mut self, sentence_cutoff_threshold: f64) -> Self {
        self.sentence_cutoff_threshold = sentence_cutoff_threshold;
        self
    }

    #[must_use]
    pub fn with_excluded_texts(
        mut self,
        texts: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.excluded_texts = texts.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn candidates_for_input(&self, input: &str) -> Vec<Candidate> {
        self.candidates_for_input_with_limit(input, self.max_candidates)
    }

    #[must_use]
    pub fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let mut rows = self.storage.memory_owner_rows(&self.lookup_index);
        if let GrammarProvider::Octagram(grammar) = &self.grammar {
            rows.push(grammar.memory_owner_row());
        }
        rows
    }

    #[must_use]
    pub fn candidates_for_input_with_limit(
        &self,
        input: &str,
        max_candidates: usize,
    ) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }

        let graph = self.word_graph_for_input(input);
        self.candidates_for_graph_with_limit(input, &graph, max_candidates)
    }

    #[must_use]
    pub fn candidates_for_input_with_limit_and_scratch(
        &self,
        input: &str,
        max_candidates: usize,
        scratch: &mut UpstreamSentenceScratch,
    ) -> Vec<Candidate> {
        if input.is_empty() {
            scratch.clear();
            return Vec::new();
        }

        let max_candidates = max_candidates.max(1).min(self.max_candidates);
        if self.grammar.scoring_grammar().is_some() {
            scratch.clear();
            return self.candidates_for_input_with_limit(input, max_candidates);
        }

        let PoetModelStorage::Owned(storage) = &self.storage else {
            scratch.clear();
            return self.candidates_for_input_with_limit(input, max_candidates);
        };

        if !scratch.is_ready_for(input, max_candidates) {
            return self.rebuild_owned_scratch(input, max_candidates, scratch);
        }
        if scratch.input.len() == input.len() {
            return self.candidates_for_cached_sentence_paths_with_limit(
                input,
                scratch,
                max_candidates,
                Duration::ZERO,
            );
        }

        match self.extend_owned_scratch(storage, input, max_candidates, scratch) {
            Some(candidates) => candidates,
            None => self.rebuild_owned_scratch(input, max_candidates, scratch),
        }
    }

    #[must_use]
    pub fn candidates_for_code_spans_with_limit(
        &self,
        input: &str,
        spans: &[SentenceCodeSpan],
        max_candidates: usize,
    ) -> Vec<Candidate> {
        if input.is_empty() || spans.is_empty() {
            return Vec::new();
        }

        let graph = self.word_graph_for_code_spans(input, spans);
        self.candidates_for_abbreviation_graph_with_limit(input, &graph, max_candidates)
    }

    #[must_use]
    pub fn has_code(&self, code: &str) -> bool {
        self.entries_for_code_range(code).is_some()
    }

    fn candidates_for_graph_with_limit(
        &self,
        input: &str,
        graph: &WordGraph,
        max_candidates: usize,
    ) -> Vec<Candidate> {
        let max_candidates = max_candidates.max(1).min(self.max_candidates);
        // ScriptTranslation exposes only full-input Poet winners. Intermediate
        // DP states remain internal; visible partial rows come from the
        // independent dictionary/ScriptEncoder phrase stream below.
        let sentences_by_end = make_sentences_by_end(
            graph,
            self.max_sentences,
            self.max_homophones,
            self.sentence_cutoff_threshold,
            input.len(),
            self.grammar.scoring_grammar(),
            true,
        );
        let phrases = Self::phrase_candidates_for_graph(
            input,
            graph,
            max_candidates.saturating_add(self.max_sentences),
        );
        self.merge_script_translation_candidates(
            sentences_by_end
                .get(&input.len())
                .map(Vec::as_slice)
                .unwrap_or_default(),
            &phrases,
            max_candidates,
        )
    }

    fn phrase_candidates_for_graph(input: &str, graph: &WordGraph, limit: usize) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        let mut seen = HashMap::new();
        let Some(phrases) = graph.get(&0) else {
            return candidates;
        };
        for (end, entries) in phrases.iter().rev() {
            for entry in entries {
                // ScriptTranslation's phrase iterator is text-distinct. Apply
                // the bound to visible rows, not raw dictionary records: an
                // early run of duplicate rows must not hide a later unique
                // phrase and leave the page short after the final merge.
                if seen.insert(entry.text.clone(), ()).is_some() {
                    continue;
                }
                candidates.push(Candidate {
                    text: entry.text.clone(),
                    comment: String::new(),
                    preedit: None,
                    source: if *end == input.len() {
                        CandidateSource::Table
                    } else {
                        CandidateSource::PartialTable {
                            consumed: *end,
                            recompose_on_default: false,
                        }
                    },
                    quality: 0.0,
                });
                if candidates.len() >= limit {
                    return candidates;
                }
            }
        }
        candidates
    }

    fn merge_script_translation_candidates(
        &self,
        sentences: &[SentencePath],
        phrases: &[Candidate],
        max_candidates: usize,
    ) -> Vec<Candidate> {
        let max_candidates = max_candidates.max(1).min(self.max_candidates);
        let has_reliable_full_phrase = phrases
            .first()
            .is_some_and(|candidate| candidate.source == CandidateSource::Table);
        let mut candidates = Vec::new();
        let mut seen = HashMap::new();
        if !has_reliable_full_phrase {
            // A reliable exact whole-input phrase suppresses Poet entirely.
            // Otherwise configured sentence winners precede every phrase row.
            for sentence in sentences.iter().take(self.max_sentences) {
                if seen.insert(sentence.text.clone(), ()).is_some() {
                    continue;
                }
                candidates.push(Candidate {
                    text: sentence.text.clone(),
                    comment: String::new(),
                    preedit: None,
                    source: CandidateSource::Sentence,
                    quality: 0.0,
                });
            }
        }
        for phrase in phrases {
            if seen.insert(phrase.text.clone(), ()).is_some() {
                continue;
            }
            candidates.push(phrase.clone());
            if candidates.len() >= max_candidates {
                break;
            }
        }
        candidates.truncate(max_candidates);
        let candidate_count = candidates.len();
        for (index, candidate) in candidates.iter_mut().enumerate() {
            candidate.quality = (candidate_count - index) as f32;
        }
        candidates
    }

    fn candidates_for_cached_sentence_paths_with_limit(
        &self,
        input: &str,
        scratch: &UpstreamSentenceScratch,
        max_candidates: usize,
        path_duration: Duration,
    ) -> Vec<Candidate> {
        let record_candidate_extraction = cfg!(debug_assertions) && crate::m37_metrics_enabled();
        let (state_bucket_count, states_ranked) = if record_candidate_extraction {
            (
                scratch
                    .states_by_end
                    .iter()
                    .filter(|states| !states.is_empty())
                    .count(),
                scratch.states_by_end.iter().map(Vec::len).sum(),
            )
        } else {
            (0, 0)
        };
        let merge_start = record_candidate_extraction.then(Instant::now);
        let sentences = scratch
            .sentence_paths_by_end
            .get(input.len())
            .map(Vec::as_slice)
            .unwrap_or_default();
        let candidates = self.merge_script_translation_candidates(
            sentences,
            &scratch.phrase_candidates,
            max_candidates,
        );
        if let Some(merge_start) = merge_start {
            crate::m37_record_upstream_sentence_model_candidate_extraction(
                state_bucket_count,
                states_ranked,
                path_duration,
                merge_start.elapsed(),
            );
        }
        candidates
    }

    fn rebuild_owned_scratch(
        &self,
        input: &str,
        max_candidates: usize,
        scratch: &mut UpstreamSentenceScratch,
    ) -> Vec<Candidate> {
        scratch.clear();
        let graph = self.word_graph_for_input(input);
        scratch.states_by_end = collect_sentence_state_vec(
            &graph,
            self.max_sentences,
            self.max_homophones,
            input.len(),
            self.grammar.scoring_grammar(),
            false,
        );
        scratch.phrase_candidates = Self::phrase_candidates_for_graph(
            input,
            &graph,
            max_candidates.saturating_add(self.max_sentences),
        );
        if let PoetModelStorage::Owned(storage) = &self.storage {
            let (prefix_states, exact_spans) =
                self.prefix_state_cache_for_input_end_owned(storage, input);
            scratch.prefix_states_by_start = prefix_states;
            scratch.exact_spans_by_start = exact_spans;
        }
        let path_start = crate::m37_metrics_enabled().then(Instant::now);
        scratch.sentence_paths_by_end = sentence_paths_vec_by_end_from_states(
            &scratch.states_by_end,
            self.max_sentences,
            self.sentence_cutoff_threshold,
        );
        let path_duration = path_start.map_or(Duration::ZERO, |start| start.elapsed());
        scratch.input = input.to_owned();
        scratch.max_candidates = max_candidates;
        self.candidates_for_cached_sentence_paths_with_limit(
            input,
            scratch,
            max_candidates,
            path_duration,
        )
    }

    fn candidates_for_abbreviation_graph_with_limit(
        &self,
        input: &str,
        graph: &WordGraph,
        max_candidates: usize,
    ) -> Vec<Candidate> {
        let ranking_start = crate::m37_metrics_enabled().then(Instant::now);
        let max_candidates = max_candidates.max(1).min(self.max_candidates);
        let sentence_limit = max_candidates.saturating_mul(4).min(self.max_candidates);
        let sentences_by_end = make_abbreviation_sentences_by_end(
            graph,
            sentence_limit,
            input.len(),
            self.grammar.scoring_grammar(),
        );
        let total_end = input.len();
        let first_segment_end = sentences_by_end
            .get(&total_end)
            .and_then(|sentences| sentences.first())
            .and_then(|sentence| sentence.word_lengths.first())
            .copied()
            .filter(|end| *end < total_end);

        let mut ranked = Vec::<RankedSentence>::new();
        if let Some(sentence) = first_segment_end
            .and_then(|end| {
                abbreviation_synthesized_sentence(
                    graph,
                    end,
                    total_end,
                    self.grammar.scoring_grammar(),
                )
            })
            .or_else(|| {
                sentences_by_end
                    .get(&total_end)
                    .and_then(|sentences| sentences.first().cloned())
            })
        {
            ranked.push(RankedSentence {
                end: total_end,
                sentence,
            });
        }
        if let Some(end) = first_segment_end {
            if let Some(sentences) = sentences_by_end.get(&end) {
                ranked.extend(
                    sentences
                        .iter()
                        .cloned()
                        .map(|sentence| RankedSentence { end, sentence }),
                );
            }
        }
        if ranked.len() < max_candidates {
            for (end, sentences) in sentences_by_end.iter().rev() {
                if *end == total_end || Some(*end) == first_segment_end {
                    continue;
                }
                ranked.extend(sentences.iter().cloned().map(|sentence| RankedSentence {
                    end: *end,
                    sentence,
                }));
                if ranked.len() >= sentence_limit {
                    break;
                }
            }
        }

        ranked.sort_by(compare_ranked_abbreviation_sentence);
        let mut seen = HashMap::new();
        let mut candidates = Vec::new();
        for item in ranked {
            if seen.insert(item.sentence.text.clone(), ()).is_some() {
                continue;
            }
            let source = if item.end < total_end {
                CandidateSource::PartialTable {
                    consumed: item.end,
                    recompose_on_default: false,
                }
            } else {
                CandidateSource::Sentence
            };
            candidates.push(Candidate {
                text: item.sentence.text,
                comment: String::new(),
                preedit: None,
                source,
                quality: 0.0,
            });
            if candidates.len() >= max_candidates {
                break;
            }
        }
        let base_quality = candidates.len() as f32;
        for (index, candidate) in candidates.iter_mut().enumerate() {
            candidate.quality = base_quality - index as f32;
        }
        if let Some(start) = ranking_start {
            crate::m37_record_abbreviation_sentence_ranking(start.elapsed());
        }
        candidates
    }

    fn word_graph_for_input(&self, input: &str) -> WordGraph {
        if let PoetModelStorage::Owned(storage) = &self.storage {
            return self.word_graph_for_input_owned(storage, input);
        }
        if let PoetModelStorage::ByteBacked(storage) = &self.storage {
            return self.word_graph_for_input_byte_backed(storage, input);
        }

        let rebuild_start = crate::m37_metrics_enabled().then(Instant::now);
        let entry_limit = self.word_graph_entry_limit();
        let mut graph = WordGraph::new();
        let boundaries = input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
            .collect::<Vec<_>>();
        let mut reachable = vec![false; boundaries.len()];
        if let Some(first) = reachable.first_mut() {
            *first = true;
        }
        let mut code_prefix_checks = 0usize;
        let mut table_entries_considered = 0usize;
        let mut vocabulary_entries_considered = 0usize;
        let mut graph_edges = 0usize;
        let record_volume_metrics = cfg!(debug_assertions) && crate::m37_metrics_enabled();
        let mut seen_code_spans = record_volume_metrics.then(HashMap::<&str, usize>::new);
        let mut vocabulary_chars = Vec::new();
        let mut vocabulary_indices_cache = HashMap::<&str, Vec<usize>>::new();
        let mut character_code_cache = CharacterCodeCache::new();
        let mut lookup_metrics = crate::M40SentenceLookupMetrics::default();
        for (start_index, start) in boundaries.iter().copied().enumerate() {
            if start >= input.len() {
                continue;
            }
            if !reachable[start_index] {
                lookup_metrics.unreachable_starts_skipped += 1;
                continue;
            }
            lookup_metrics.reachable_starts_visited += 1;
            let suffix = &input[start..];
            lookup_metrics.phrase_index_walk_calls += 1;
            let walk = self
                .lookup_index
                .walk_from(&self.storage, input, &boundaries, start_index);
            code_prefix_checks += walk.prefix_hits + walk.prefix_misses;
            lookup_metrics.prefix_filter_hits += walk.prefix_hits;
            lookup_metrics.prefix_filter_misses += walk.prefix_misses;
            lookup_metrics.prefix_filter_early_breaks += walk.prefix_early_breaks;
            lookup_metrics.exact_range_index_misses += walk.exact_range_misses;
            lookup_metrics.phrase_index_nodes_visited += walk.nodes_visited;
            lookup_metrics.phrase_index_entry_ranges_emitted += walk.entry_ranges_emitted;
            for span in walk.spans {
                let code = &input[start..span.end];
                if let Some(seen_code_spans) = seen_code_spans.as_mut() {
                    let derivations = seen_code_spans.entry(code).or_default();
                    if *derivations > 0 {
                        lookup_metrics.code_span_rederivations += 1;
                    }
                    *derivations += 1;
                }
                lookup_metrics.exact_range_index_hits += 1;
                let (bounded_entries, scanned) = collect_distinct_word_graph_entries(
                    span.entries.clone().map(|entry_index| {
                        (
                            self.storage.entry_text(entry_index),
                            match &self.storage {
                                PoetModelStorage::Owned(storage) => storage
                                    .dictionary_weight(self.storage.entry_weight(entry_index)),
                                _ => upstream_dictionary_weight(f64::from(
                                    self.storage.entry_weight(entry_index),
                                )),
                            },
                        )
                    }),
                    entry_limit,
                );
                table_entries_considered += scanned;
                let inserted_edge = !bounded_entries.is_empty();
                for entry in bounded_entries {
                    if record_volume_metrics {
                        lookup_metrics.graph_entry_text_bytes += entry.text.len();
                    }
                    graph
                        .entry(start)
                        .or_default()
                        .entry(span.end)
                        .or_default()
                        .push(entry);
                    graph_edges += 1;
                    if record_volume_metrics {
                        lookup_metrics.graph_entries_inserted += 1;
                    }
                }
                if inserted_edge {
                    reachable[span.end_index] = true;
                }
                let vocabulary_entries = vocabulary_indices_cache
                    .entry(code)
                    .or_insert_with(|| self.storage.vocabulary_indices_for_first_code(false, code));
                if record_volume_metrics {
                    lookup_metrics.vocabulary_index_probes += 1;
                    lookup_metrics.vocabulary_rows_examined += vocabulary_entries.len();
                }
                for index in vocabulary_entries.iter().copied() {
                    let phrase_codes = match &self.storage {
                        PoetModelStorage::ByteBacked(storage) => {
                            let chars = ByteBackedVocabularyChars {
                                storage,
                                abbreviation: false,
                                range: storage.vocabulary_chars_range(false, index),
                            };
                            if !self.vocabulary_entry_matches_input_prefix_byte_backed(
                                chars,
                                suffix,
                                code,
                                &mut character_code_cache,
                            ) {
                                continue;
                            }
                            self.derive_matching_phrase_codes_byte_backed(
                                chars,
                                suffix,
                                code,
                                &mut character_code_cache,
                            )
                        }
                        _ => {
                            self.storage
                                .vocabulary_chars_into(false, index, &mut vocabulary_chars);
                            if !self.vocabulary_entry_matches_input_prefix(
                                &vocabulary_chars,
                                suffix,
                                code,
                                &mut character_code_cache,
                            ) {
                                continue;
                            }
                            self.derive_matching_phrase_codes(
                                &vocabulary_chars,
                                suffix,
                                code,
                                &mut character_code_cache,
                            )
                        }
                    };
                    vocabulary_entries_considered += 1;
                    let vocabulary_text = self.storage.vocabulary_text(false, index).to_owned();
                    let vocabulary_weight = upstream_dictionary_weight(f64::from(
                        self.storage.vocabulary_weight(false, index),
                    ));
                    for phrase_code in phrase_codes {
                        let end = start + phrase_code.len();
                        if record_volume_metrics {
                            lookup_metrics.graph_entry_text_bytes += vocabulary_text.len();
                        }
                        graph
                            .entry(start)
                            .or_default()
                            .entry(end)
                            .or_default()
                            .push(WordGraphEntry::new(
                                vocabulary_text.clone(),
                                vocabulary_weight,
                            ));
                        graph_edges += 1;
                        if record_volume_metrics {
                            lookup_metrics.graph_entries_inserted += 1;
                        }
                        if let Ok(end_index) = boundaries.binary_search(&end) {
                            reachable[end_index] = true;
                        }
                    }
                }
            }
        }
        for edges in graph.values_mut() {
            for entries in edges.values_mut() {
                entries.retain(|entry| !self.excluded_texts.contains(&entry.text));
                sort_dedup_truncate_word_graph_entries(entries, entry_limit);
            }
        }
        crate::m37_record_upstream_sentence_model_scan(
            code_prefix_checks,
            table_entries_considered,
            vocabulary_entries_considered,
            graph_edges,
        );
        if let Some(rebuild_start) = rebuild_start {
            let elapsed = rebuild_start.elapsed();
            lookup_metrics.graph_rebuild_duration = elapsed;
            lookup_metrics.incremental_discarded_rebuild_chars = input.chars().count();
            crate::m37_record_upstream_sentence_model_lookup_index(lookup_metrics);
        }
        graph
    }

    fn word_graph_entry_limit(&self) -> usize {
        MIN_WORD_GRAPH_PHRASE_ROWS_PER_SPAN.max(self.max_homophones)
    }

    fn word_graph_for_input_byte_backed(
        &self,
        storage: &ByteBackedPoetStore,
        input: &str,
    ) -> WordGraph {
        let rebuild_start = crate::m37_metrics_enabled().then(Instant::now);
        let entry_limit = self.word_graph_entry_limit();
        let mut graph = WordGraph::new();
        let boundaries = input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
            .collect::<Vec<_>>();
        let mut reachable = vec![false; boundaries.len()];
        if let Some(first) = reachable.first_mut() {
            *first = true;
        }
        let mut code_prefix_checks = 0usize;
        let mut table_entries_considered = 0usize;
        let mut vocabulary_entries_considered = 0usize;
        let mut graph_edges = 0usize;
        let record_volume_metrics = cfg!(debug_assertions) && crate::m37_metrics_enabled();
        let mut seen_code_spans = record_volume_metrics.then(HashMap::<&str, usize>::new);
        let mut character_code_cache = CharacterCodeCache::new();
        let mut vocabulary_indices_cache = HashMap::<&str, Vec<usize>>::new();
        let mut lookup_metrics = crate::M40SentenceLookupMetrics::default();
        for (start_index, start) in boundaries.iter().copied().enumerate() {
            if start >= input.len() {
                continue;
            }
            if !reachable[start_index] {
                lookup_metrics.unreachable_starts_skipped += 1;
                continue;
            }
            lookup_metrics.reachable_starts_visited += 1;
            let suffix = &input[start..];
            lookup_metrics.phrase_index_walk_calls += 1;
            let walk = storage.walk_from_prefix_index(input, &boundaries, start_index);
            code_prefix_checks += walk.prefix_hits + walk.prefix_misses;
            lookup_metrics.prefix_filter_hits += walk.prefix_hits;
            lookup_metrics.prefix_filter_misses += walk.prefix_misses;
            lookup_metrics.prefix_filter_early_breaks += walk.prefix_early_breaks;
            lookup_metrics.exact_range_index_misses += walk.exact_range_misses;
            lookup_metrics.phrase_index_nodes_visited += walk.nodes_visited;
            lookup_metrics.phrase_index_entry_ranges_emitted += walk.entry_ranges_emitted;
            for span in walk.spans {
                lookup_metrics.exact_range_index_hits += 1;
                let code = &input[start..span.end];
                if let Some(seen_code_spans) = seen_code_spans.as_mut() {
                    let derivations = seen_code_spans.entry(code).or_default();
                    if *derivations > 0 {
                        lookup_metrics.code_span_rederivations += 1;
                    }
                    *derivations += 1;
                }
                let (bounded_entries, scanned) = collect_distinct_word_graph_entries(
                    span.entries.clone().map(|entry_index| {
                        (
                            storage.entry_text(entry_index),
                            upstream_dictionary_weight(f64::from(
                                storage.entry_weight(entry_index),
                            )),
                        )
                    }),
                    entry_limit,
                );
                table_entries_considered += scanned;
                let inserted_edge = !bounded_entries.is_empty();
                for entry in bounded_entries {
                    if record_volume_metrics {
                        lookup_metrics.graph_entry_text_bytes += entry.text.len();
                    }
                    graph
                        .entry(start)
                        .or_default()
                        .entry(span.end)
                        .or_default()
                        .push(entry);
                    graph_edges += 1;
                    if record_volume_metrics {
                        lookup_metrics.graph_entries_inserted += 1;
                    }
                }
                if inserted_edge {
                    reachable[span.end_index] = true;
                }
                let vocabulary_entries = vocabulary_indices_cache
                    .entry(code)
                    .or_insert_with(|| storage.vocabulary_indices_for_first_code(false, code));
                if record_volume_metrics {
                    lookup_metrics.vocabulary_index_probes += 1;
                    lookup_metrics.vocabulary_rows_examined += vocabulary_entries.len();
                }
                for index in vocabulary_entries.iter().copied() {
                    let chars = ByteBackedVocabularyChars {
                        storage,
                        abbreviation: false,
                        range: storage.vocabulary_chars_range(false, index),
                    };
                    if !self.vocabulary_entry_matches_input_prefix_byte_backed(
                        chars,
                        suffix,
                        code,
                        &mut character_code_cache,
                    ) {
                        continue;
                    }
                    vocabulary_entries_considered += 1;
                    let phrase_codes = self.derive_matching_phrase_codes_byte_backed(
                        chars,
                        suffix,
                        code,
                        &mut character_code_cache,
                    );
                    let vocabulary_text = storage.vocabulary_text(false, index).to_owned();
                    let vocabulary_weight = upstream_dictionary_weight(f64::from(
                        storage.vocabulary_weight(false, index),
                    ));
                    for phrase_code in phrase_codes {
                        let end = start + phrase_code.len();
                        if record_volume_metrics {
                            lookup_metrics.graph_entry_text_bytes += vocabulary_text.len();
                        }
                        graph
                            .entry(start)
                            .or_default()
                            .entry(end)
                            .or_default()
                            .push(WordGraphEntry::new(
                                vocabulary_text.clone(),
                                vocabulary_weight,
                            ));
                        graph_edges += 1;
                        if record_volume_metrics {
                            lookup_metrics.graph_entries_inserted += 1;
                        }
                        if let Ok(end_index) = boundaries.binary_search(&end) {
                            reachable[end_index] = true;
                        }
                    }
                }
            }
        }
        for edges in graph.values_mut() {
            for entries in edges.values_mut() {
                entries.retain(|entry| !self.excluded_texts.contains(&entry.text));
                sort_dedup_truncate_word_graph_entries(entries, entry_limit);
            }
        }
        crate::m37_record_upstream_sentence_model_scan(
            code_prefix_checks,
            table_entries_considered,
            vocabulary_entries_considered,
            graph_edges,
        );
        if let Some(rebuild_start) = rebuild_start {
            let elapsed = rebuild_start.elapsed();
            lookup_metrics.graph_rebuild_duration = elapsed;
            lookup_metrics.incremental_discarded_rebuild_chars = input.chars().count();
            crate::m37_record_upstream_sentence_model_lookup_index(lookup_metrics);
        }
        graph
    }

    fn word_graph_for_input_owned(
        &self,
        storage: &OwnedPoetModelStorage,
        input: &str,
    ) -> WordGraph {
        let rebuild_start = crate::m37_metrics_enabled().then(Instant::now);
        let entry_limit = self.word_graph_entry_limit();
        let mut graph = WordGraph::new();
        let boundaries = input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
            .collect::<Vec<_>>();
        let mut reachable = vec![false; boundaries.len()];
        if let Some(first) = reachable.first_mut() {
            *first = true;
        }
        let mut code_prefix_checks = 0usize;
        let mut table_entries_considered = 0usize;
        let mut vocabulary_entries_considered = 0usize;
        let mut graph_edges = 0usize;
        let record_volume_metrics = cfg!(debug_assertions) && crate::m37_metrics_enabled();
        let mut seen_code_spans = record_volume_metrics.then(HashMap::<&str, usize>::new);
        let mut lookup_metrics = crate::M40SentenceLookupMetrics::default();
        for (start_index, start) in boundaries.iter().copied().enumerate() {
            if start >= input.len() {
                continue;
            }
            if !reachable[start_index] {
                lookup_metrics.unreachable_starts_skipped += 1;
                continue;
            }
            lookup_metrics.reachable_starts_visited += 1;
            let suffix = &input[start..];
            lookup_metrics.phrase_index_walk_calls += 1;
            let walk = self
                .lookup_index
                .walk_from(storage, input, &boundaries, start_index);
            code_prefix_checks += walk.prefix_hits + walk.prefix_misses;
            lookup_metrics.prefix_filter_hits += walk.prefix_hits;
            lookup_metrics.prefix_filter_misses += walk.prefix_misses;
            lookup_metrics.prefix_filter_early_breaks += walk.prefix_early_breaks;
            lookup_metrics.exact_range_index_misses += walk.exact_range_misses;
            lookup_metrics.phrase_index_nodes_visited += walk.nodes_visited;
            lookup_metrics.phrase_index_entry_ranges_emitted += walk.entry_ranges_emitted;
            for span in walk.spans {
                let code = &input[start..span.end];
                if let Some(seen_code_spans) = seen_code_spans.as_mut() {
                    let derivations = seen_code_spans.entry(code).or_default();
                    if *derivations > 0 {
                        lookup_metrics.code_span_rederivations += 1;
                    }
                    *derivations += 1;
                }
                lookup_metrics.exact_range_index_hits += 1;
                let entries = &storage.entries_by_code[span.entries.clone()];
                let (bounded_entries, scanned) = collect_distinct_word_graph_entries(
                    entries.iter().map(|entry| {
                        (
                            entry.text(&storage.entry_texts),
                            storage.dictionary_weight(entry.weight),
                        )
                    }),
                    entry_limit,
                );
                table_entries_considered += scanned;
                let inserted_edge = !bounded_entries.is_empty();
                for entry in bounded_entries {
                    if record_volume_metrics {
                        lookup_metrics.graph_entry_text_bytes += entry.text.len();
                    }
                    graph
                        .entry(start)
                        .or_default()
                        .entry(span.end)
                        .or_default()
                        .push(entry);
                    graph_edges += 1;
                    if record_volume_metrics {
                        lookup_metrics.graph_entries_inserted += 1;
                    }
                }
                if inserted_edge {
                    reachable[span.end_index] = true;
                }
                let vocabulary_entries =
                    vocabulary_indices_for_first_code(&storage.vocabulary_first_codes, code);
                if record_volume_metrics {
                    lookup_metrics.vocabulary_index_probes += 1;
                    lookup_metrics.vocabulary_rows_examined += vocabulary_entries.len();
                }
                for (_, index) in vocabulary_entries {
                    let vocabulary_entry = &storage.vocabulary[*index];
                    if !self.vocabulary_entry_matches_input_prefix_owned(
                        storage,
                        vocabulary_entry,
                        suffix,
                        code,
                    ) {
                        continue;
                    }
                    vocabulary_entries_considered += 1;
                    for phrase_code in self.derive_matching_phrase_codes_owned(
                        storage,
                        vocabulary_entry,
                        suffix,
                        code,
                    ) {
                        let end = start + phrase_code.len();
                        if record_volume_metrics {
                            lookup_metrics.graph_entry_text_bytes += vocabulary_entry.text.len();
                        }
                        graph
                            .entry(start)
                            .or_default()
                            .entry(end)
                            .or_default()
                            .push(WordGraphEntry::new(
                                vocabulary_entry.text.clone(),
                                upstream_dictionary_weight(f64::from(vocabulary_entry.weight)),
                            ));
                        graph_edges += 1;
                        if record_volume_metrics {
                            lookup_metrics.graph_entries_inserted += 1;
                        }
                        if let Ok(end_index) = boundaries.binary_search(&end) {
                            reachable[end_index] = true;
                        }
                    }
                }
            }
        }
        for edges in graph.values_mut() {
            for entries in edges.values_mut() {
                entries.retain(|entry| !self.excluded_texts.contains(&entry.text));
                sort_dedup_truncate_word_graph_entries(entries, entry_limit);
            }
        }
        crate::m37_record_upstream_sentence_model_scan(
            code_prefix_checks,
            table_entries_considered,
            vocabulary_entries_considered,
            graph_edges,
        );
        if let Some(rebuild_start) = rebuild_start {
            let elapsed = rebuild_start.elapsed();
            lookup_metrics.graph_rebuild_duration = elapsed;
            lookup_metrics.incremental_discarded_rebuild_chars = input.chars().count();
            crate::m37_record_upstream_sentence_model_lookup_index(lookup_metrics);
        }
        graph
    }

    fn prefix_state_cache_for_input_end_owned(
        &self,
        storage: &OwnedPoetModelStorage,
        input: &str,
    ) -> (
        Vec<Option<SentencePrefixState>>,
        Vec<Vec<CachedSentenceCodeSpan>>,
    ) {
        let boundaries = input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
            .collect::<Vec<_>>();
        let mut prefix_states = vec![None; boundaries.len()];
        let mut exact_spans = vec![Vec::new(); boundaries.len()];
        let root = self.lookup_index.root_prefix_state();
        for (start_index, start) in boundaries.iter().copied().enumerate() {
            if start == input.len() {
                prefix_states[start_index] = Some(root);
                continue;
            }
            let mut state = Some(root);
            for (end_index, end) in boundaries.iter().copied().enumerate().skip(start_index + 1) {
                let code = &input[start..end];
                state = state.and_then(|current| {
                    self.lookup_index
                        .advance_prefix_state(storage, current, code)
                });
                if let Some(current) = state {
                    if let Some(span) = self
                        .lookup_index
                        .span_for_prefix_state(storage, current, code, end, end_index)
                    {
                        exact_spans[start_index].push(CachedSentenceCodeSpan {
                            end: span.end,
                            end_index: span.end_index,
                            entries: span.entries,
                        });
                    }
                }
                if state.is_none() {
                    break;
                }
            }
            prefix_states[start_index] = state;
        }
        (prefix_states, exact_spans)
    }

    fn extend_owned_scratch(
        &self,
        storage: &OwnedPoetModelStorage,
        input: &str,
        max_candidates: usize,
        scratch: &mut UpstreamSentenceScratch,
    ) -> Option<Vec<Candidate>> {
        let previous_len = scratch.input.len();
        let entry_limit = self.word_graph_entry_limit();
        if previous_len >= input.len() || !input.starts_with(&scratch.input) {
            scratch.clear();
            return None;
        }
        let boundaries = input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
            .collect::<Vec<_>>();
        let previous_end_index = boundaries.binary_search(&previous_len).ok()?;
        if boundaries.len() != previous_end_index.saturating_add(2) {
            return None;
        }
        if scratch.prefix_states_by_start.len() != previous_end_index.saturating_add(1) {
            return None;
        }
        if scratch.exact_spans_by_start.len() != previous_end_index.saturating_add(1) {
            return None;
        }
        scratch
            .states_by_end
            .resize_with(input.len().saturating_add(1), Vec::new);

        let rebuild_start = crate::m37_metrics_enabled().then(Instant::now);
        let extend_start = rebuild_start;
        let mut graph = BorrowedWordGraph::new();
        let mut reachable = vec![false; boundaries.len()];
        for (index, boundary) in boundaries.iter().copied().enumerate() {
            if boundary <= previous_len
                && scratch
                    .states_by_end
                    .get(boundary)
                    .is_some_and(|states| !states.is_empty())
            {
                reachable[index] = true;
            }
        }

        let mut code_prefix_checks = 0usize;
        let mut table_entries_considered = 0usize;
        let mut vocabulary_entries_considered = 0usize;
        let mut graph_edges = 0usize;
        let record_volume_metrics = cfg!(debug_assertions) && crate::m37_metrics_enabled();
        let root_prefix_state = self.lookup_index.root_prefix_state();
        let mut next_prefix_states = vec![None; boundaries.len()];
        let mut next_exact_spans = vec![Vec::new(); boundaries.len()];
        let mut seen_code_spans = record_volume_metrics.then(HashMap::<&str, usize>::new);
        let mut lookup_metrics = crate::M40SentenceLookupMetrics::default();

        for (start_index, start) in boundaries.iter().copied().enumerate() {
            if start >= input.len() {
                continue;
            }
            if !reachable[start_index] {
                lookup_metrics.unreachable_starts_skipped += 1;
                continue;
            }
            lookup_metrics.reachable_starts_visited += 1;
            let suffix = &input[start..];
            lookup_metrics.phrase_index_walk_calls += 1;
            let mut spans = scratch
                .exact_spans_by_start
                .get(start_index)
                .cloned()
                .unwrap_or_default();
            let previous_prefix_state = if start == previous_len {
                Some(root_prefix_state)
            } else {
                scratch
                    .prefix_states_by_start
                    .get(start_index)
                    .copied()
                    .flatten()
            };
            let end = input.len();
            let end_index = boundaries.len() - 1;
            let code = &input[start..end];
            if let Some(prefix_state) = previous_prefix_state {
                match self
                    .lookup_index
                    .advance_prefix_state(storage, prefix_state, code)
                {
                    Some(prefix_state) => {
                        next_prefix_states[start_index] = Some(prefix_state);
                        lookup_metrics.prefix_filter_hits += 1;
                        lookup_metrics.phrase_index_nodes_visited += 1;
                        code_prefix_checks += 1;
                        match self.lookup_index.span_for_prefix_state(
                            storage,
                            prefix_state,
                            code,
                            end,
                            end_index,
                        ) {
                            Some(span) => spans.push(CachedSentenceCodeSpan {
                                end: span.end,
                                end_index: span.end_index,
                                entries: span.entries,
                            }),
                            None => lookup_metrics.exact_range_index_misses += 1,
                        }
                    }
                    None => {
                        lookup_metrics.prefix_filter_misses += 1;
                        lookup_metrics.prefix_filter_early_breaks += 1;
                        code_prefix_checks += 1;
                    }
                }
            } else {
                lookup_metrics.prefix_filter_misses += 1;
                lookup_metrics.prefix_filter_early_breaks += 1;
                code_prefix_checks += 1;
            }
            for span in &spans {
                let code = &input[start..span.end];
                if let Some(seen_code_spans) = seen_code_spans.as_mut() {
                    let derivations = seen_code_spans.entry(code).or_default();
                    if *derivations > 0 {
                        lookup_metrics.code_span_rederivations += 1;
                    }
                    *derivations += 1;
                }
                lookup_metrics.exact_range_index_hits += 1;
                lookup_metrics.phrase_index_entry_ranges_emitted += 1;
                if span.end > previous_len {
                    let entries = &storage.entries_by_code[span.entries.clone()];
                    let mut seen_entries = HashMap::new();
                    let mut bounded_entries = Vec::new();
                    let mut scanned = 0usize;
                    for entry in entries {
                        scanned += 1;
                        let text = entry.text(&storage.entry_texts);
                        if seen_entries.insert(text, ()).is_some() {
                            continue;
                        }
                        bounded_entries.push(BorrowedWordGraphEntry {
                            text,
                            weight: storage.dictionary_weight(entry.weight),
                        });
                        if bounded_entries.len() >= entry_limit {
                            break;
                        }
                    }
                    table_entries_considered += scanned;
                    let inserted_edge = !bounded_entries.is_empty();
                    for entry in bounded_entries {
                        if record_volume_metrics {
                            lookup_metrics.graph_entry_text_bytes += entry.text.len();
                        }
                        graph
                            .entry(start)
                            .or_default()
                            .entry(span.end)
                            .or_default()
                            .push(entry);
                        graph_edges += 1;
                        if record_volume_metrics {
                            lookup_metrics.graph_entries_inserted += 1;
                        }
                    }
                    if inserted_edge {
                        reachable[span.end_index] = true;
                    }
                }
                let vocabulary_entries =
                    vocabulary_indices_for_first_code(&storage.vocabulary_first_codes, code);
                if record_volume_metrics {
                    lookup_metrics.vocabulary_index_probes += 1;
                    lookup_metrics.vocabulary_rows_examined += vocabulary_entries.len();
                }
                let minimum_new_phrase_code_len = previous_len.saturating_sub(start);
                for (_, index) in vocabulary_entries {
                    let vocabulary_entry = &storage.vocabulary[*index];
                    if !self.vocabulary_entry_matches_input_prefix_owned_after(
                        storage,
                        vocabulary_entry,
                        suffix,
                        code,
                        minimum_new_phrase_code_len,
                    ) {
                        continue;
                    }
                    vocabulary_entries_considered += 1;
                    for phrase_code in self.derive_matching_phrase_codes_owned_after(
                        storage,
                        vocabulary_entry,
                        suffix,
                        code,
                        minimum_new_phrase_code_len,
                    ) {
                        let end = start + phrase_code.len();
                        let Ok(end_index) = boundaries.binary_search(&end) else {
                            continue;
                        };
                        if end <= previous_len {
                            reachable[end_index] = true;
                            continue;
                        }
                        if record_volume_metrics {
                            lookup_metrics.graph_entry_text_bytes += vocabulary_entry.text.len();
                        }
                        graph
                            .entry(start)
                            .or_default()
                            .entry(end)
                            .or_default()
                            .push(BorrowedWordGraphEntry {
                                text: vocabulary_entry.text.as_str(),
                                weight: upstream_dictionary_weight(f64::from(
                                    vocabulary_entry.weight,
                                )),
                            });
                        graph_edges += 1;
                        if record_volume_metrics {
                            lookup_metrics.graph_entries_inserted += 1;
                        }
                        reachable[end_index] = true;
                    }
                }
            }
            next_exact_spans[start_index] = spans;
        }
        next_prefix_states[boundaries.len() - 1] = Some(root_prefix_state);
        next_exact_spans[boundaries.len() - 1] = Vec::new();
        for edges in graph.values_mut() {
            for entries in edges.values_mut() {
                entries.retain(|entry| !self.excluded_texts.contains(entry.text));
                sort_dedup_truncate_borrowed_word_graph_entries(entries, entry_limit);
            }
        }

        let record_metrics = cfg!(debug_assertions) && crate::m37_metrics_enabled();
        let mut dp_states_created = 0usize;
        let mut dp_beam_evictions = 0usize;
        let search = SentenceSearchMode::for_options(self.max_sentences, false);
        for (start, edges) in &graph {
            if *start > input.len() {
                continue;
            }
            if scratch
                .states_by_end
                .get(*start)
                .map_or(true, Vec::is_empty)
            {
                continue;
            }
            for (end, entries) in edges {
                if *end <= previous_len || *end > input.len() {
                    continue;
                }
                let (source_slice, destination_slice) = scratch.states_by_end.split_at_mut(*end);
                let source_states = &source_slice[*start];
                if source_states.is_empty() {
                    continue;
                }
                let destination = &mut destination_slice[0];
                for source in source_states {
                    for entry in entries.iter().take(self.max_homophones) {
                        let candidate_weight = source.weight + null_grammar_score(entry.weight);
                        let next = source.extended(entry.text, candidate_weight, end - start, None);
                        if record_metrics {
                            dp_states_created += 1;
                        }
                        let evicted = insert_sentence_state(destination, next, search);
                        if record_metrics && evicted {
                            dp_beam_evictions += 1;
                        }
                    }
                }
            }
        }
        if record_metrics {
            crate::m37_record_upstream_sentence_model_dp(dp_states_created, dp_beam_evictions);
        }

        crate::m37_record_upstream_sentence_model_scan(
            code_prefix_checks,
            table_entries_considered,
            vocabulary_entries_considered,
            graph_edges,
        );
        if let Some(rebuild_start) = rebuild_start {
            let elapsed = rebuild_start.elapsed();
            lookup_metrics.graph_rebuild_duration = elapsed;
            lookup_metrics.incremental_reuse_hits = 1;
            if let Some(extend_start) = extend_start {
                lookup_metrics.incremental_extend_duration = extend_start.elapsed();
            }
            crate::m37_record_upstream_sentence_model_lookup_index(lookup_metrics);
        }

        for candidate in &mut scratch.phrase_candidates {
            // Yesterday's whole-input phrase becomes a partial prefix after one
            // more key. Retain it for ordering/recomposition while adding only
            // the newly reachable start-zero edges from the incremental graph.
            if candidate.source == CandidateSource::Table {
                candidate.source = CandidateSource::PartialTable {
                    consumed: previous_len,
                    recompose_on_default: false,
                };
            }
        }
        let mut phrase_candidates = Vec::new();
        if let Some(phrases) = graph.get(&0) {
            for (end, entries) in phrases.iter().rev() {
                for entry in entries {
                    phrase_candidates.push(Candidate {
                        text: entry.text.to_owned(),
                        comment: String::new(),
                        preedit: None,
                        source: if *end == input.len() {
                            CandidateSource::Table
                        } else {
                            CandidateSource::PartialTable {
                                consumed: *end,
                                recompose_on_default: false,
                            }
                        },
                        // Ordering is carried by the graph traversal itself:
                        // newly reached ends are longer than every retained old
                        // row, and each edge remains in its f64 weight order.
                        // The final merge assigns visible positional quality.
                        quality: 0.0,
                    });
                }
            }
        }
        phrase_candidates.append(&mut scratch.phrase_candidates);
        let mut seen_phrases = HashMap::new();
        phrase_candidates
            .retain(|candidate| seen_phrases.insert(candidate.text.clone(), ()).is_none());
        phrase_candidates.truncate(
            max_candidates
                .saturating_add(self.max_sentences)
                .saturating_add(1),
        );
        scratch.phrase_candidates = phrase_candidates;

        scratch.input = input.to_owned();
        scratch.prefix_states_by_start = next_prefix_states;
        scratch.exact_spans_by_start = next_exact_spans;
        scratch
            .sentence_paths_by_end
            .resize_with(scratch.states_by_end.len(), Vec::new);
        let path_start = crate::m37_metrics_enabled().then(Instant::now);
        if let Some(states) = scratch.states_by_end.get(input.len()) {
            scratch.sentence_paths_by_end[input.len()] = sentence_paths_from_states(
                states.clone(),
                self.max_sentences,
                self.sentence_cutoff_threshold,
            );
        }
        let path_duration = path_start.map_or(Duration::ZERO, |start| start.elapsed());
        Some(self.candidates_for_cached_sentence_paths_with_limit(
            input,
            scratch,
            max_candidates,
            path_duration,
        ))
    }

    fn word_graph_for_code_spans(&self, input: &str, spans: &[SentenceCodeSpan]) -> WordGraph {
        let rebuild_start = crate::m37_metrics_enabled().then(Instant::now);
        let entry_limit = self.word_graph_entry_limit();
        let mut graph = WordGraph::new();
        let boundaries = input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
            .collect::<Vec<_>>();
        let mut spans_by_start = vec![Vec::new(); boundaries.len()];
        for span in spans {
            if span.start >= span.end
                || span.end > input.len()
                || !input.is_char_boundary(span.start)
                || !input.is_char_boundary(span.end)
                || span.code.is_empty()
            {
                continue;
            }
            let Ok(start_index) = boundaries.binary_search(&span.start) else {
                continue;
            };
            let Ok(end_index) = boundaries.binary_search(&span.end) else {
                continue;
            };
            spans_by_start[start_index].push(InputCodeSpan {
                end: span.end,
                end_index,
                code: span.code.as_str(),
            });
        }
        for spans in &mut spans_by_start {
            spans.sort_by(|left, right| {
                left.end
                    .cmp(&right.end)
                    .then_with(|| left.code.cmp(right.code))
            });
            spans.dedup_by(|left, right| left.end == right.end && left.code == right.code);
        }

        let mut reachable = vec![false; boundaries.len()];
        if let Some(first) = reachable.first_mut() {
            *first = true;
        }
        let mut table_entries_considered = 0usize;
        let mut vocabulary_entries_considered = 0usize;
        let mut graph_edges = 0usize;
        let mut vocabulary_chars = Vec::new();
        let mut lookup_metrics = crate::M40SentenceLookupMetrics::default();
        for (start_index, start) in boundaries.iter().copied().enumerate() {
            if start >= input.len() {
                continue;
            }
            if !reachable[start_index] {
                lookup_metrics.unreachable_starts_skipped += 1;
                continue;
            }
            lookup_metrics.reachable_starts_visited += 1;
            for span in &spans_by_start[start_index] {
                lookup_metrics.phrase_index_walk_calls += 1;
                lookup_metrics.phrase_index_nodes_visited += 1;
                let Some(entries) = self.entries_for_code_range(span.code) else {
                    lookup_metrics.exact_range_index_misses += 1;
                    lookup_metrics.partition_point_fallback_calls += 1;
                    continue;
                };
                lookup_metrics.exact_range_index_hits += 1;
                let (bounded_entries, scanned) = collect_distinct_word_graph_entries(
                    entries.clone().map(|entry_index| {
                        (
                            self.storage.entry_text(entry_index),
                            match &self.storage {
                                PoetModelStorage::Owned(storage) => {
                                    storage.raw_weight(self.storage.entry_weight(entry_index))
                                }
                                _ => f64::from(self.storage.entry_weight(entry_index)),
                            },
                        )
                    }),
                    entry_limit,
                );
                table_entries_considered += scanned;
                let inserted_edge = !bounded_entries.is_empty();
                for entry in bounded_entries {
                    graph
                        .entry(start)
                        .or_default()
                        .entry(span.end)
                        .or_default()
                        .push(entry);
                    graph_edges += 1;
                }
                if inserted_edge {
                    reachable[span.end_index] = true;
                }
                let vocabulary_entries = self
                    .storage
                    .vocabulary_indices_for_first_code(true, span.code);
                for index in vocabulary_entries {
                    self.storage
                        .vocabulary_chars_into(true, index, &mut vocabulary_chars);
                    vocabulary_entries_considered += 1;
                    for (_phrase_code, phrase_end, phrase_end_index) in self
                        .derive_matching_phrase_codes_from_spans(
                            &vocabulary_chars,
                            &spans_by_start,
                            *span,
                        )
                    {
                        graph
                            .entry(start)
                            .or_default()
                            .entry(phrase_end)
                            .or_default()
                            .push(WordGraphEntry::new(
                                self.storage.vocabulary_text(true, index).to_owned(),
                                f64::from(self.storage.vocabulary_weight(true, index))
                                    + ABBREVIATION_VOCABULARY_RAW_SPAN_BONUS
                                        * (phrase_end - start).pow(2) as f64,
                            ));
                        graph_edges += 1;
                        reachable[phrase_end_index] = true;
                    }
                }
            }
        }
        for edges in graph.values_mut() {
            for entries in edges.values_mut() {
                entries.retain(|entry| !self.excluded_texts.contains(&entry.text));
                sort_dedup_truncate_word_graph_entries(entries, entry_limit);
            }
        }
        crate::m37_record_upstream_sentence_model_scan(
            0,
            table_entries_considered,
            vocabulary_entries_considered,
            graph_edges,
        );
        if let Some(rebuild_start) = rebuild_start {
            let elapsed = rebuild_start.elapsed();
            lookup_metrics.graph_rebuild_duration = elapsed;
            lookup_metrics.incremental_discarded_rebuild_chars = input.chars().count();
            crate::m37_record_upstream_sentence_model_lookup_index(lookup_metrics);
            crate::m37_record_abbreviation_code_span_graph_build(elapsed);
        }
        graph
    }

    fn entries_for_code_range(&self, code: &str) -> Option<Range<usize>> {
        self.storage
            .entries_for_code_range(&self.lookup_index, code)
    }

    fn derive_matching_phrase_codes(
        &self,
        chars: &[char],
        input: &str,
        first_code: &str,
        character_code_cache: &mut CharacterCodeCache,
    ) -> Vec<String> {
        let mut codes = Vec::new();
        let mut current = first_code.to_owned();
        self.derive_matching_phrase_codes_from(
            chars,
            input,
            1,
            &mut current,
            &mut codes,
            character_code_cache,
        );
        codes.sort();
        codes.dedup();
        codes
    }

    fn derive_matching_phrase_codes_owned(
        &self,
        storage: &OwnedPoetModelStorage,
        entry: &ModelVocabularyEntry,
        input: &str,
        first_code: &str,
    ) -> Vec<String> {
        self.derive_matching_phrase_codes_owned_after(storage, entry, input, first_code, 0)
    }

    fn derive_matching_phrase_codes_owned_after(
        &self,
        storage: &OwnedPoetModelStorage,
        entry: &ModelVocabularyEntry,
        input: &str,
        first_code: &str,
        minimum_code_len: usize,
    ) -> Vec<String> {
        let mut codes = Vec::new();
        let mut current = first_code.to_owned();
        derive_matching_phrase_codes_from_owned_after(
            storage.normal_phrase_character_codes(&self.grammar),
            &entry.chars,
            input,
            1,
            &mut current,
            &mut codes,
            minimum_code_len,
        );
        codes.sort();
        codes.dedup();
        codes
    }

    fn derive_matching_phrase_codes_byte_backed(
        &self,
        chars: ByteBackedVocabularyChars<'_>,
        input: &str,
        first_code: &str,
        character_code_cache: &mut CharacterCodeCache,
    ) -> Vec<String> {
        let mut codes = Vec::new();
        let mut current = first_code.to_owned();
        let mut derivation = ByteBackedPhraseDerivation {
            chars,
            input,
            codes: &mut codes,
            character_code_cache,
        };
        self.derive_matching_phrase_codes_from_byte_backed(
            &mut derivation,
            1,
            first_code.len(),
            &mut current,
        );
        codes.sort();
        codes.dedup();
        codes
    }

    fn vocabulary_entry_matches_input_prefix(
        &self,
        chars: &[char],
        input: &str,
        first_code: &str,
        character_code_cache: &mut CharacterCodeCache,
    ) -> bool {
        self.vocabulary_chars_match_input_prefix_from(
            chars,
            input,
            1,
            first_code.len(),
            character_code_cache,
        )
    }

    fn vocabulary_entry_matches_input_prefix_owned(
        &self,
        storage: &OwnedPoetModelStorage,
        entry: &ModelVocabularyEntry,
        input: &str,
        first_code: &str,
    ) -> bool {
        self.vocabulary_entry_matches_input_prefix_owned_after(storage, entry, input, first_code, 0)
    }

    fn vocabulary_entry_matches_input_prefix_owned_after(
        &self,
        storage: &OwnedPoetModelStorage,
        entry: &ModelVocabularyEntry,
        input: &str,
        first_code: &str,
        minimum_code_len: usize,
    ) -> bool {
        self.vocabulary_chars_match_input_prefix_from_owned(
            storage,
            &entry.chars,
            input,
            1,
            first_code.len(),
            minimum_code_len,
        )
    }

    fn vocabulary_entry_matches_input_prefix_byte_backed(
        &self,
        chars: ByteBackedVocabularyChars<'_>,
        input: &str,
        first_code: &str,
        character_code_cache: &mut CharacterCodeCache,
    ) -> bool {
        self.vocabulary_chars_match_input_prefix_from_byte_backed(
            chars,
            1,
            input,
            first_code.len(),
            character_code_cache,
        )
    }

    fn vocabulary_chars_match_input_prefix_from(
        &self,
        chars: &[char],
        input: &str,
        index: usize,
        offset: usize,
        character_code_cache: &mut CharacterCodeCache,
    ) -> bool {
        if index == chars.len() {
            return offset <= input.len();
        }
        if offset >= input.len() {
            return false;
        }
        let Some(remaining) = input.get(offset..) else {
            return false;
        };
        let next_codes =
            self.normal_phrase_character_codes_cached(chars[index], character_code_cache);
        next_codes.iter().any(|next_code| {
            remaining.starts_with(next_code)
                && self.vocabulary_chars_match_input_prefix_from(
                    chars,
                    input,
                    index + 1,
                    offset + next_code.len(),
                    character_code_cache,
                )
        })
    }

    fn vocabulary_chars_match_input_prefix_from_owned(
        &self,
        storage: &OwnedPoetModelStorage,
        chars: &[char],
        input: &str,
        index: usize,
        offset: usize,
        minimum_code_len: usize,
    ) -> bool {
        if index == chars.len() {
            return offset <= input.len() && offset > minimum_code_len;
        }
        if offset >= input.len() {
            return false;
        }
        let Some(remaining) = input.get(offset..) else {
            return false;
        };
        let Some(next_codes) = storage
            .normal_phrase_character_codes(&self.grammar)
            .get(&chars[index])
        else {
            return false;
        };
        next_codes.iter().any(|next_code| {
            remaining.starts_with(next_code)
                && self.vocabulary_chars_match_input_prefix_from_owned(
                    storage,
                    chars,
                    input,
                    index + 1,
                    offset + next_code.len(),
                    minimum_code_len,
                )
        })
    }

    fn vocabulary_chars_match_input_prefix_from_byte_backed(
        &self,
        chars: ByteBackedVocabularyChars<'_>,
        index: usize,
        input: &str,
        offset: usize,
        character_code_cache: &mut CharacterCodeCache,
    ) -> bool {
        if index == chars.len() {
            return offset <= input.len();
        }
        if offset >= input.len() {
            return false;
        }
        let Some(remaining) = input.get(offset..) else {
            return false;
        };
        let ch = chars.char_at(index);
        let next_codes = self.normal_phrase_character_codes_cached(ch, character_code_cache);
        next_codes.iter().any(|next_code| {
            remaining.starts_with(next_code)
                && self.vocabulary_chars_match_input_prefix_from_byte_backed(
                    chars,
                    index + 1,
                    input,
                    offset + next_code.len(),
                    character_code_cache,
                )
        })
    }

    fn derive_matching_phrase_codes_from(
        &self,
        chars: &[char],
        input: &str,
        index: usize,
        current: &mut String,
        codes: &mut Vec<String>,
        character_code_cache: &mut CharacterCodeCache,
    ) {
        if index == chars.len() {
            if input.starts_with(current.as_str()) {
                codes.push(current.clone());
            }
            return;
        }
        let next_codes =
            self.normal_phrase_character_codes_cached(chars[index], character_code_cache);
        for next_code in next_codes.iter() {
            let original_len = current.len();
            current.push_str(next_code);
            if input.starts_with(current.as_str()) {
                self.derive_matching_phrase_codes_from(
                    chars,
                    input,
                    index + 1,
                    current,
                    codes,
                    character_code_cache,
                );
            }
            current.truncate(original_len);
        }
    }

    fn derive_matching_phrase_codes_from_byte_backed(
        &self,
        derivation: &mut ByteBackedPhraseDerivation<'_, '_>,
        index: usize,
        offset: usize,
        current: &mut String,
    ) {
        if index == derivation.chars.len() {
            derivation.codes.push(current.clone());
            return;
        }
        if offset >= derivation.input.len() {
            return;
        }
        let Some(remaining) = derivation.input.get(offset..) else {
            return;
        };
        let ch = derivation.chars.char_at(index);
        let next_codes =
            self.normal_phrase_character_codes_cached(ch, derivation.character_code_cache);
        for next_code in next_codes.iter() {
            if !remaining.starts_with(next_code) {
                continue;
            }
            let original_len = current.len();
            current.push_str(next_code);
            self.derive_matching_phrase_codes_from_byte_backed(
                derivation,
                index + 1,
                offset + next_code.len(),
                current,
            );
            current.truncate(original_len);
        }
    }

    fn normal_phrase_character_codes(&self, ch: char) -> Vec<&str> {
        // The compiled normal-code section carries ScriptEncoder's 5% filter;
        // the abbreviation section is reserved for abbreviation traversal.
        self.storage.character_codes(false, ch)
    }

    fn normal_phrase_character_codes_cached(
        &self,
        ch: char,
        cache: &mut CharacterCodeCache,
    ) -> Arc<[String]> {
        if let Some(codes) = cache.get(&ch) {
            return Arc::clone(codes);
        }
        let codes: Arc<[String]> = self
            .normal_phrase_character_codes(ch)
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>()
            .into();
        cache.insert(ch, Arc::clone(&codes));
        codes
    }

    fn derive_matching_phrase_codes_from_spans(
        &self,
        chars: &[char],
        spans_by_start: &[Vec<InputCodeSpan<'_>>],
        first_span: InputCodeSpan<'_>,
    ) -> Vec<(String, usize, usize)> {
        let mut codes = Vec::new();
        self.derive_matching_phrase_span_codes_from(
            chars,
            spans_by_start,
            PhraseSpanCodeState {
                index: 1,
                start_index: first_span.end_index,
                end: first_span.end,
                code: first_span.code.to_owned(),
            },
            &mut codes,
        );
        codes.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        codes.dedup();
        codes
    }

    fn derive_matching_phrase_span_codes_from(
        &self,
        chars: &[char],
        spans_by_start: &[Vec<InputCodeSpan<'_>>],
        state: PhraseSpanCodeState,
        codes: &mut Vec<(String, usize, usize)>,
    ) {
        if codes.len() >= MAX_DERIVED_PHRASE_CODES_PER_VOCABULARY_ENTRY {
            return;
        }
        if state.index == chars.len() {
            codes.push((state.code, state.end, state.start_index));
            return;
        }
        let next_codes = self.storage.character_codes(true, chars[state.index]);
        let Some(spans) = spans_by_start.get(state.start_index) else {
            return;
        };
        for span in spans {
            if !next_codes.contains(&span.code) {
                continue;
            }
            self.derive_matching_phrase_span_codes_from(
                chars,
                spans_by_start,
                PhraseSpanCodeState {
                    index: state.index + 1,
                    start_index: span.end_index,
                    end: span.end,
                    code: format!("{}{}", state.code, span.code),
                },
                codes,
            );
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct InputCodeSpan<'a> {
    end: usize,
    end_index: usize,
    code: &'a str,
}

#[derive(Clone, Debug)]
struct PhraseSpanCodeState {
    index: usize,
    start_index: usize,
    end: usize,
    code: String,
}

#[derive(Clone, Debug)]
struct RankedSentence {
    end: usize,
    sentence: SentencePath,
}

fn compare_ranked_abbreviation_sentence(left: &RankedSentence, right: &RankedSentence) -> Ordering {
    right
        .end
        .cmp(&left.end)
        .then_with(|| {
            left.sentence
                .word_lengths
                .len()
                .cmp(&right.sentence.word_lengths.len())
        })
        .then_with(|| {
            singleton_word_count(&left.sentence.word_lengths)
                .cmp(&singleton_word_count(&right.sentence.word_lengths))
        })
        .then_with(|| right.sentence.word_lengths.cmp(&left.sentence.word_lengths))
        .then_with(|| {
            right
                .sentence
                .weight
                .partial_cmp(&left.sentence.weight)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.sentence.text.cmp(&right.sentence.text))
}

#[derive(Clone, Debug, PartialEq)]
struct ModelVocabularyEntry {
    text: String,
    chars: Vec<char>,
    weight: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ModelStringRange {
    start: u32,
    end: u32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct ModelStringPool {
    bytes: Box<str>,
    ranges: Box<[ModelStringRange]>,
}

impl ModelStringPool {
    fn string(&self, range: ModelStringRange) -> &str {
        &self.bytes[range.start as usize..range.end as usize]
    }

    fn indexed(&self, index: u32) -> &str {
        self.string(self.ranges[index as usize])
    }

    fn estimated_retained_bytes(&self) -> usize {
        mem::size_of::<Self>()
            .saturating_add(self.bytes.len())
            .saturating_add(
                self.ranges
                    .len()
                    .saturating_mul(mem::size_of::<ModelStringRange>()),
            )
    }
}

fn pack_owned_model_entries(
    entries: Vec<OwnedModelEntry>,
) -> (Vec<ModelEntry>, ModelStringPool, ModelStringPool) {
    let mut model_entries = Vec::with_capacity(entries.len());
    let mut text_bytes = String::new();
    let mut code_bytes = String::new();
    let mut code_ranges = Vec::<ModelStringRange>::new();
    for entry in entries {
        let text_start =
            u32::try_from(text_bytes.len()).expect("sentence model text pool exceeds u32");
        text_bytes.push_str(&entry.text);
        let text_end =
            u32::try_from(text_bytes.len()).expect("sentence model text pool exceeds u32");
        let new_code = match code_ranges.last() {
            Some(range) => code_bytes[range.start as usize..range.end as usize] != entry.code,
            None => true,
        };
        if new_code {
            let code_start =
                u32::try_from(code_bytes.len()).expect("sentence model code pool exceeds u32");
            code_bytes.push_str(&entry.code);
            let code_end =
                u32::try_from(code_bytes.len()).expect("sentence model code pool exceeds u32");
            code_ranges.push(ModelStringRange {
                start: code_start,
                end: code_end,
            });
        }
        model_entries.push(ModelEntry {
            text: ModelStringRange {
                start: text_start,
                end: text_end,
            },
            code_id: u32::try_from(code_ranges.len() - 1)
                .expect("sentence model code id exceeds u32"),
            weight: entry.weight,
        });
    }
    (
        model_entries,
        ModelStringPool {
            bytes: text_bytes.into_boxed_str(),
            ranges: Box::default(),
        },
        ModelStringPool {
            bytes: code_bytes.into_boxed_str(),
            ranges: code_ranges.into_boxed_slice(),
        },
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ModelEntry {
    text: ModelStringRange,
    code_id: u32,
    weight: f32,
}

impl ModelEntry {
    fn from_table_entry(entry: &TableEntry) -> OwnedModelEntry {
        OwnedModelEntry {
            text: entry.text.clone(),
            code: entry.code.clone(),
            weight: entry.weight,
        }
    }

    fn from_owned_table_entry(entry: TableEntry) -> OwnedModelEntry {
        OwnedModelEntry {
            text: entry.text,
            code: entry.code,
            weight: entry.weight,
        }
    }

    fn text<'a>(&self, pool: &'a ModelStringPool) -> &'a str {
        pool.string(self.text)
    }

    fn code<'a>(&self, pool: &'a ModelStringPool) -> &'a str {
        pool.indexed(self.code_id)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct OwnedModelEntry {
    text: String,
    code: String,
    weight: f32,
}

fn estimate_model_entries_bytes(
    entries: &[ModelEntry],
    texts: &ModelStringPool,
    codes: &ModelStringPool,
) -> usize {
    mem::size_of::<Vec<ModelEntry>>()
        .saturating_add(entries.len().saturating_mul(mem::size_of::<ModelEntry>()))
        .saturating_add(texts.estimated_retained_bytes())
        .saturating_add(codes.estimated_retained_bytes())
}

fn estimate_model_vocabulary_bytes(entries: &[ModelVocabularyEntry]) -> usize {
    mem::size_of::<Vec<ModelVocabularyEntry>>()
        .saturating_add(
            entries
                .len()
                .saturating_mul(mem::size_of::<ModelVocabularyEntry>()),
        )
        .saturating_add(
            entries
                .iter()
                .map(|entry| {
                    entry.text.capacity().saturating_add(
                        entry
                            .chars
                            .capacity()
                            .saturating_mul(mem::size_of::<char>()),
                    )
                })
                .sum::<usize>(),
        )
}

fn estimate_string_usize_pairs_bytes(values: &[(String, usize)]) -> usize {
    mem::size_of_val(values).saturating_add(
        values
            .iter()
            .map(|(value, _)| value.capacity())
            .sum::<usize>(),
    )
}

fn collect_distinct_word_graph_entries<'a>(
    entries: impl IntoIterator<Item = (&'a str, f64)>,
    limit: usize,
) -> (Vec<WordGraphEntry>, usize) {
    let mut seen = HashMap::new();
    let mut collected = Vec::new();
    let mut scanned = 0usize;
    for (text, weight) in entries {
        scanned += 1;
        if seen.insert(text, ()).is_some() {
            continue;
        }
        collected.push(WordGraphEntry::new(text, weight));
        if collected.len() >= limit {
            break;
        }
    }
    (collected, scanned)
}

fn sort_dedup_truncate_word_graph_entries(entries: &mut Vec<WordGraphEntry>, limit: usize) {
    entries.sort_by(compare_word_graph_entry);
    let mut seen = HashMap::new();
    entries.retain(|entry| seen.insert(entry.text.clone(), ()).is_none());
    entries.truncate(limit);
}

fn sort_dedup_truncate_borrowed_word_graph_entries(
    entries: &mut Vec<BorrowedWordGraphEntry<'_>>,
    limit: usize,
) {
    entries.sort_by(compare_borrowed_word_graph_entry);
    let mut seen = HashMap::new();
    entries.retain(|entry| seen.insert(entry.text, ()).is_none());
    entries.truncate(limit);
}

fn compare_word_graph_entry(left: &WordGraphEntry, right: &WordGraphEntry) -> Ordering {
    right
        .weight
        .partial_cmp(&left.weight)
        .unwrap_or(Ordering::Equal)
}

fn compare_borrowed_word_graph_entry(
    left: &BorrowedWordGraphEntry<'_>,
    right: &BorrowedWordGraphEntry<'_>,
) -> Ordering {
    right
        .weight
        .partial_cmp(&left.weight)
        .unwrap_or(Ordering::Equal)
}

fn compare_model_entry_by_code(left: &OwnedModelEntry, right: &OwnedModelEntry) -> Ordering {
    left.code
        .cmp(&right.code)
        .then_with(|| compare_model_entry(left, right))
}

fn compare_model_entry(left: &OwnedModelEntry, right: &OwnedModelEntry) -> Ordering {
    right
        .weight
        .partial_cmp(&left.weight)
        .unwrap_or(Ordering::Equal)
}

fn derive_matching_phrase_codes_from_owned_after(
    character_codes: &HashMap<char, Vec<String>>,
    chars: &[char],
    input: &str,
    index: usize,
    current: &mut String,
    codes: &mut Vec<String>,
    minimum_code_len: usize,
) {
    if index == chars.len() {
        if current.len() > minimum_code_len && input.starts_with(current.as_str()) {
            codes.push(current.clone());
        }
        return;
    }
    let Some(next_codes) = character_codes.get(&chars[index]) else {
        return;
    };
    for next_code in next_codes {
        let original_len = current.len();
        current.push_str(next_code);
        if input.starts_with(current.as_str()) {
            derive_matching_phrase_codes_from_owned_after(
                character_codes,
                chars,
                input,
                index + 1,
                current,
                codes,
                minimum_code_len,
            );
        }
        current.truncate(original_len);
    }
}
