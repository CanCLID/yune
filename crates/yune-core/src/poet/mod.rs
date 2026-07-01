use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use std::time::Instant;

use crate::{
    Candidate, CandidateSource, MemoryOwnerClass, MemoryOwnerRow, PresetVocabularyEntry,
    TableDictionary, TableEntry,
};

mod index;
mod octagram;

use index::SentenceLookupIndex;
pub use octagram::{
    encode_octagram_key, OctagramGrammar, OctagramGrammarConfig, OctagramGrammarParseError,
};

/// Upstream `grammar.h` null-grammar penalty (`ln(1e-6)`) used when no `.gram`
/// language model is configured.
pub const UPSTREAM_NO_GRAMMAR_PENALTY: f64 = -13.815510557964274;
const UPSTREAM_DICT_ENTRY_WEIGHT_SCALE: f64 = 18.420680743952367;

const CODE_LENGTH_QUALITY_BAND: f32 = 1_000.0;
const MAX_WORD_GRAPH_ENTRIES_PER_SPAN: usize = 7;
const MAX_DERIVED_PHRASE_CODES_PER_VOCABULARY_ENTRY: usize = 16;
const ABBREVIATION_VOCABULARY_RAW_SPAN_BONUS: f64 = 500_000.0;

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
    make_sentences_with_grammar(graph, total_length, max_sentences, &NullGrammar)
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

    make_sentences_by_end(graph, max_sentences, total_length, grammar)
        .remove(&total_length)
        .unwrap_or_default()
}

fn make_sentences_by_end(
    graph: &WordGraph,
    max_sentences: usize,
    total_length: usize,
    grammar: &dyn Grammar,
) -> BTreeMap<usize, Vec<SentencePath>> {
    if max_sentences == 0 {
        return BTreeMap::new();
    }

    collect_sentence_states(graph, max_sentences, total_length, grammar)
        .into_iter()
        .filter(|(end, _)| *end > 0)
        .map(|(end, states)| (end, sentence_paths_from_states(states, max_sentences)))
        .collect()
}

fn make_abbreviation_sentences_by_end(
    graph: &WordGraph,
    max_sentences: usize,
    total_length: usize,
    grammar: &dyn Grammar,
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
                abbreviation_sentence_paths_from_states(states, max_sentences),
            )
        })
        .collect()
}

fn collect_sentence_states(
    graph: &WordGraph,
    max_sentences: usize,
    total_length: usize,
    grammar: &dyn Grammar,
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
                    next.weight += entry.weight
                        + grammar.query(
                            &source.grammar_context(),
                            &entry.text,
                            *end == total_length,
                        );
                    next.text.push_str(&entry.text);
                    next.word_lengths.push(end - start);
                    next.push_word(&entry.text);
                    insert_state(states.entry(*end).or_default(), next, max_sentences * 3);
                }
            }
        }
    }

    states
}

fn collect_abbreviation_sentence_states(
    graph: &WordGraph,
    max_sentences: usize,
    total_length: usize,
    grammar: &dyn Grammar,
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
                    next.weight += entry.weight
                        + grammar.query(
                            &source.grammar_context(),
                            &entry.text,
                            *end == total_length,
                        );
                    next.text.push_str(&entry.text);
                    next.word_lengths.push(end - start);
                    next.push_word(&entry.text);
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
) -> Vec<SentencePath> {
    states.sort_by(compare_path_state);
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
            word_lengths: state.word_lengths,
        });
        if paths.len() == max_sentences {
            break;
        }
    }
    paths
}

fn abbreviation_sentence_paths_from_states(
    mut states: Vec<PathState>,
    max_sentences: usize,
) -> Vec<SentencePath> {
    states.sort_by(compare_abbreviation_path_state);
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
            word_lengths: state.word_lengths,
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
    word_lengths: Vec<usize>,
    recent_words: Vec<String>,
}

impl PathState {
    fn grammar_context(&self) -> String {
        self.recent_words.concat()
    }

    fn push_word(&mut self, word: &str) {
        if self.recent_words.len() == 2 {
            self.recent_words.remove(0);
        }
        self.recent_words.push(word.to_owned());
    }
}

fn insert_state(states: &mut Vec<PathState>, candidate: PathState, beam_width: usize) {
    if let Some(existing_index) = states
        .iter()
        .position(|existing| has_same_future_grammar_state(existing, &candidate))
    {
        if compare_path_state(&candidate, &states[existing_index]) == Ordering::Less {
            states.remove(existing_index);
        } else {
            return;
        }
    }
    let index = states
        .binary_search_by(|existing| compare_path_state(existing, &candidate))
        .unwrap_or_else(|index| index);
    states.insert(index, candidate);
    if states.len() > beam_width {
        states.pop();
    }
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

fn compare_path_state(left: &PathState, right: &PathState) -> Ordering {
    right
        .weight
        .partial_cmp(&left.weight)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.word_lengths.len().cmp(&right.word_lengths.len()))
        .then_with(|| right.word_lengths.cmp(&left.word_lengths))
        .then_with(|| left.text.cmp(&right.text))
}

fn compare_abbreviation_path_state(left: &PathState, right: &PathState) -> Ordering {
    left.word_lengths
        .len()
        .cmp(&right.word_lengths.len())
        .then_with(|| {
            singleton_word_count(&left.word_lengths).cmp(&singleton_word_count(&right.word_lengths))
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
    grammar: &dyn Grammar,
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
        weight +=
            entry.weight + grammar.query(&recent_words.concat(), &entry.text, end == total_end);
        word_lengths.push(end - start);
        if recent_words.len() == 2 {
            recent_words.remove(0);
        }
        recent_words.push(entry.text.clone());
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

#[derive(Clone, Debug, Default)]
pub struct UpstreamSentenceModel {
    entries_by_code: Vec<ModelEntry>,
    entry_texts: ModelStringPool,
    entry_codes: ModelStringPool,
    lookup_index: SentenceLookupIndex,
    vocabulary: Vec<ModelVocabularyEntry>,
    vocabulary_first_codes: Vec<(String, usize)>,
    abbreviation_vocabulary: Vec<ModelVocabularyEntry>,
    abbreviation_vocabulary_first_codes: Vec<(String, usize)>,
    character_codes: HashMap<char, Vec<String>>,
    abbreviation_character_codes: HashMap<char, Vec<String>>,
    max_candidates: usize,
    grammar: GrammarProvider,
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
        )
    }

    fn from_model_entries(
        entries: impl IntoIterator<Item = OwnedModelEntry>,
        vocabulary: &[PresetVocabularyEntry],
        abbreviation_vocabulary: &[PresetVocabularyEntry],
        max_candidates: usize,
    ) -> Self {
        let mut owned_entries = Vec::new();
        let mut character_codes: HashMap<char, Vec<String>> = HashMap::new();
        let mut abbreviation_character_codes: HashMap<char, Vec<String>> = HashMap::new();
        for entry in entries {
            if entry.code.is_empty() {
                continue;
            }
            let mut chars = entry.text.chars();
            if let Some(ch) = chars.next() {
                if chars.next().is_none() {
                    character_codes
                        .entry(ch)
                        .or_default()
                        .push(entry.code.clone());
                    if entry.weight > 0.0 {
                        abbreviation_character_codes
                            .entry(ch)
                            .or_default()
                            .push(entry.code.clone());
                    }
                }
            }
            owned_entries.push(entry);
        }
        for codes in character_codes.values_mut() {
            codes.sort();
            codes.dedup();
        }
        for codes in abbreviation_character_codes.values_mut() {
            codes.sort();
            codes.dedup();
        }
        owned_entries.sort_by(compare_model_entry_by_code);
        let (entries_by_code, entry_texts, entry_codes) = pack_owned_model_entries(owned_entries);
        let index_start = crate::m37_metrics_enabled().then(Instant::now);
        let lookup_index = SentenceLookupIndex::build(&entries_by_code, &entry_codes);
        if let Some(index_start) = index_start {
            crate::m37_record_upstream_sentence_model_index_build(index_start.elapsed());
        }
        let (vocabulary, vocabulary_first_codes) =
            build_model_vocabulary_index(vocabulary, &character_codes);
        let (abbreviation_vocabulary, abbreviation_vocabulary_first_codes) =
            build_model_vocabulary_index(abbreviation_vocabulary, &abbreviation_character_codes);
        Self {
            entries_by_code,
            entry_texts,
            entry_codes,
            lookup_index,
            vocabulary,
            vocabulary_first_codes,
            abbreviation_vocabulary,
            abbreviation_vocabulary_first_codes,
            character_codes,
            abbreviation_character_codes,
            max_candidates: max_candidates.max(1),
            grammar: GrammarProvider::default(),
        }
    }

    #[must_use]
    pub fn with_grammar(mut self, grammar: impl Into<GrammarProvider>) -> Self {
        self.grammar = grammar.into();
        self
    }

    #[must_use]
    pub fn candidates_for_input(&self, input: &str) -> Vec<Candidate> {
        self.candidates_for_input_with_limit(input, self.max_candidates)
    }

    #[must_use]
    pub fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        let mut rows = vec![
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
                self.lookup_index.estimated_retained_bytes(),
                self.lookup_index.range_count(),
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
        ];
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
        self.entries_for_code(code).is_some()
    }

    fn candidates_for_graph_with_limit(
        &self,
        input: &str,
        graph: &WordGraph,
        max_candidates: usize,
    ) -> Vec<Candidate> {
        let max_candidates = max_candidates.max(1).min(self.max_candidates);
        let sentences_by_end =
            make_sentences_by_end(graph, max_candidates, input.len(), &self.grammar);
        let mut candidates = HashMap::new();
        for end in input
            .char_indices()
            .map(|(index, _)| index)
            .filter(|index| *index > 0)
            .chain(std::iter::once(input.len()))
        {
            let Some(sentences) = sentences_by_end.get(&end) else {
                continue;
            };
            for sentence in sentences {
                let candidate = Candidate {
                    text: sentence.text.clone(),
                    comment: String::new(),
                    preedit: None,
                    source: if end < input.len() {
                        CandidateSource::PartialTable {
                            consumed: end,
                            recompose_on_default: false,
                        }
                    } else {
                        CandidateSource::Sentence
                    },
                    quality: end as f32 * CODE_LENGTH_QUALITY_BAND + sentence.weight as f32,
                };
                match candidates.get(&candidate.text) {
                    Some(existing)
                        if compare_sentence_candidate(&candidate, existing) != Ordering::Less => {}
                    _ => {
                        candidates.insert(candidate.text.clone(), candidate);
                    }
                }
            }
        }
        let mut candidates = candidates.into_values().collect::<Vec<_>>();
        candidates.sort_by(compare_sentence_candidate);
        candidates.truncate(max_candidates);
        candidates
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
        let sentences_by_end =
            make_abbreviation_sentences_by_end(graph, sentence_limit, input.len(), &self.grammar);
        let total_end = input.len();
        let first_segment_end = sentences_by_end
            .get(&total_end)
            .and_then(|sentences| sentences.first())
            .and_then(|sentence| sentence.word_lengths.first())
            .copied()
            .filter(|end| *end < total_end);

        let mut ranked = Vec::<RankedSentence>::new();
        if let Some(sentence) = first_segment_end
            .and_then(|end| abbreviation_synthesized_sentence(graph, end, total_end, &self.grammar))
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
        let rebuild_start = crate::m37_metrics_enabled().then(Instant::now);
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
            let walk = self.lookup_index.walk_from(
                &self.entries_by_code,
                &self.entry_codes,
                input,
                &boundaries,
                start_index,
            );
            code_prefix_checks += walk.prefix_hits + walk.prefix_misses;
            lookup_metrics.prefix_filter_hits += walk.prefix_hits;
            lookup_metrics.prefix_filter_misses += walk.prefix_misses;
            lookup_metrics.prefix_filter_early_breaks += walk.prefix_early_breaks;
            lookup_metrics.exact_range_index_misses += walk.exact_range_misses;
            lookup_metrics.phrase_index_nodes_visited += walk.nodes_visited;
            lookup_metrics.phrase_index_entry_ranges_emitted += walk.entry_ranges_emitted;
            for span in walk.spans {
                let code = &input[start..span.end];
                let Some(entries) = self.entries_for_code(code) else {
                    lookup_metrics.exact_range_index_misses += 1;
                    lookup_metrics.partition_point_fallback_calls += 1;
                    continue;
                };
                lookup_metrics.exact_range_index_hits += 1;
                let bounded_entries = entries.iter().take(MAX_WORD_GRAPH_ENTRIES_PER_SPAN);
                table_entries_considered += entries.len().min(MAX_WORD_GRAPH_ENTRIES_PER_SPAN);
                let mut inserted_edge = false;
                for entry in bounded_entries {
                    graph
                        .entry(start)
                        .or_default()
                        .entry(span.end)
                        .or_default()
                        .push(WordGraphEntry::new(
                            entry.text(&self.entry_texts).to_owned(),
                            upstream_dictionary_weight(f64::from(entry.weight)),
                        ));
                    graph_edges += 1;
                    inserted_edge = true;
                }
                if inserted_edge {
                    reachable[span.end_index] = true;
                }
                let vocabulary_entries =
                    vocabulary_indices_for_first_code(&self.vocabulary_first_codes, code);
                for (_, index) in vocabulary_entries {
                    let vocabulary_entry = &self.vocabulary[*index];
                    if !self.vocabulary_entry_matches_input_prefix(vocabulary_entry, suffix, code) {
                        continue;
                    }
                    vocabulary_entries_considered += 1;
                    for phrase_code in
                        self.derive_matching_phrase_codes(vocabulary_entry, suffix, code)
                    {
                        let end = start + phrase_code.len();
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
                        if let Ok(end_index) = boundaries.binary_search(&end) {
                            reachable[end_index] = true;
                        }
                    }
                }
            }
        }
        for edges in graph.values_mut() {
            for entries in edges.values_mut() {
                entries.sort_by(compare_word_graph_entry);
                entries.truncate(MAX_WORD_GRAPH_ENTRIES_PER_SPAN);
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

    fn word_graph_for_code_spans(&self, input: &str, spans: &[SentenceCodeSpan]) -> WordGraph {
        let rebuild_start = crate::m37_metrics_enabled().then(Instant::now);
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
                let Some(entries) = self.entries_for_code(span.code) else {
                    lookup_metrics.exact_range_index_misses += 1;
                    lookup_metrics.partition_point_fallback_calls += 1;
                    continue;
                };
                lookup_metrics.exact_range_index_hits += 1;
                let bounded_entries = entries.iter().take(MAX_WORD_GRAPH_ENTRIES_PER_SPAN);
                table_entries_considered += entries.len().min(MAX_WORD_GRAPH_ENTRIES_PER_SPAN);
                let mut inserted_edge = false;
                for entry in bounded_entries {
                    graph
                        .entry(start)
                        .or_default()
                        .entry(span.end)
                        .or_default()
                        .push(WordGraphEntry::new(
                            entry.text(&self.entry_texts).to_owned(),
                            f64::from(entry.weight),
                        ));
                    graph_edges += 1;
                    inserted_edge = true;
                }
                if inserted_edge {
                    reachable[span.end_index] = true;
                }
                let vocabulary_entries = vocabulary_indices_for_first_code(
                    &self.abbreviation_vocabulary_first_codes,
                    span.code,
                );
                for (_, index) in vocabulary_entries {
                    let vocabulary_entry = &self.abbreviation_vocabulary[*index];
                    vocabulary_entries_considered += 1;
                    for (_phrase_code, phrase_end, phrase_end_index) in self
                        .derive_matching_phrase_codes_from_spans(
                            vocabulary_entry,
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
                                vocabulary_entry.text.clone(),
                                f64::from(vocabulary_entry.weight)
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
                entries.sort_by(compare_word_graph_entry);
                entries.truncate(MAX_WORD_GRAPH_ENTRIES_PER_SPAN);
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

    fn entries_for_code(&self, code: &str) -> Option<&[ModelEntry]> {
        self.lookup_index
            .entries_for_code(&self.entries_by_code, &self.entry_codes, code)
    }

    fn derive_matching_phrase_codes(
        &self,
        entry: &ModelVocabularyEntry,
        input: &str,
        first_code: &str,
    ) -> Vec<String> {
        let mut codes = Vec::new();
        let mut current = first_code.to_owned();
        self.derive_matching_phrase_codes_from(&entry.chars, input, 1, &mut current, &mut codes);
        codes.sort();
        codes.dedup();
        codes
    }

    fn vocabulary_entry_matches_input_prefix(
        &self,
        entry: &ModelVocabularyEntry,
        input: &str,
        first_code: &str,
    ) -> bool {
        self.vocabulary_chars_match_input_prefix_from(&entry.chars, input, 1, first_code.len())
    }

    fn vocabulary_chars_match_input_prefix_from(
        &self,
        chars: &[char],
        input: &str,
        index: usize,
        offset: usize,
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
        let Some(next_codes) = self.normal_phrase_character_codes().get(&chars[index]) else {
            return false;
        };
        next_codes.iter().any(|next_code| {
            remaining.starts_with(next_code)
                && self.vocabulary_chars_match_input_prefix_from(
                    chars,
                    input,
                    index + 1,
                    offset + next_code.len(),
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
    ) {
        if index == chars.len() {
            if input.starts_with(current.as_str()) {
                codes.push(current.clone());
            }
            return;
        }
        let Some(next_codes) = self.normal_phrase_character_codes().get(&chars[index]) else {
            return;
        };
        for next_code in next_codes {
            let original_len = current.len();
            current.push_str(next_code);
            if input.starts_with(current.as_str()) {
                self.derive_matching_phrase_codes_from(chars, input, index + 1, current, codes);
            }
            current.truncate(original_len);
        }
    }

    fn normal_phrase_character_codes(&self) -> &HashMap<char, Vec<String>> {
        match self.grammar {
            GrammarProvider::Octagram(_) => &self.abbreviation_character_codes,
            GrammarProvider::Null(_) => &self.character_codes,
        }
    }

    fn derive_matching_phrase_codes_from_spans(
        &self,
        entry: &ModelVocabularyEntry,
        spans_by_start: &[Vec<InputCodeSpan<'_>>],
        first_span: InputCodeSpan<'_>,
    ) -> Vec<(String, usize, usize)> {
        let mut codes = Vec::new();
        self.derive_matching_phrase_span_codes_from(
            &entry.chars,
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
        let Some(next_codes) = self.abbreviation_character_codes.get(&chars[state.index]) else {
            return;
        };
        let Some(spans) = spans_by_start.get(state.start_index) else {
            return;
        };
        for span in spans {
            if !next_codes.iter().any(|code| code == span.code) {
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

fn compare_sentence_candidate(left: &Candidate, right: &Candidate) -> Ordering {
    right
        .quality
        .partial_cmp(&left.quality)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.text.cmp(&right.text))
}

fn compare_word_graph_entry(left: &WordGraphEntry, right: &WordGraphEntry) -> Ordering {
    right
        .weight
        .partial_cmp(&left.weight)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.text.cmp(&right.text))
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
        .then_with(|| left.text.cmp(&right.text))
}
