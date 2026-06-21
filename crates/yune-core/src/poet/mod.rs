use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use crate::{Candidate, CandidateSource, PresetVocabularyEntry, TableDictionary, TableEntry};

/// Upstream `grammar.h` null-grammar penalty (`ln(1e-6)`) used when no `.gram`
/// language model is configured.
pub const UPSTREAM_NO_GRAMMAR_PENALTY: f64 = -13.815510557964274;

const CODE_LENGTH_QUALITY_BAND: f32 = 10_000_000.0;
const MAX_WORD_GRAPH_ENTRIES_PER_SPAN: usize = 7;

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
pub struct WordGraphEntry {
    pub text: String,
    pub code: String,
    pub weight: f64,
}

impl WordGraphEntry {
    #[must_use]
    pub fn new(text: impl Into<String>, code: impl Into<String>, weight: f64) -> Self {
        Self {
            text: text.into(),
            code: code.into(),
            weight,
        }
    }
}

pub type WordGraph = BTreeMap<usize, BTreeMap<usize, Vec<WordGraphEntry>>>;

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

#[must_use]
pub fn make_sentence(graph: &WordGraph, total_length: usize) -> Option<SentencePath> {
    make_sentences(graph, total_length, 1).into_iter().next()
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
                    next.weight += null_grammar_score(entry.weight);
                    next.text.push_str(&entry.text);
                    next.word_lengths.push(end - start);
                    insert_state(states.entry(*end).or_default(), next, max_sentences * 3);
                }
            }
        }
    }

    let mut final_states = states.remove(&total_length).unwrap_or_default();
    final_states.sort_by(compare_path_state);
    final_states
        .into_iter()
        .take(max_sentences)
        .map(|state| SentencePath {
            text: state.text,
            weight: state.weight,
            word_lengths: state.word_lengths,
        })
        .collect()
}

#[derive(Clone, Debug, Default)]
struct PathState {
    text: String,
    weight: f64,
    word_lengths: Vec<usize>,
}

fn insert_state(states: &mut Vec<PathState>, candidate: PathState, beam_width: usize) {
    if let Some(existing_index) = states
        .iter()
        .position(|existing| existing.text == candidate.text)
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

fn compare_path_state(left: &PathState, right: &PathState) -> Ordering {
    right
        .weight
        .partial_cmp(&left.weight)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left.word_lengths.len().cmp(&right.word_lengths.len()))
        .then_with(|| right.word_lengths.cmp(&left.word_lengths))
        .then_with(|| left.text.cmp(&right.text))
}

#[derive(Clone, Debug, Default)]
pub struct UpstreamSentenceModel {
    entries: Vec<ModelEntry>,
    vocabulary: Vec<ModelVocabularyEntry>,
    character_codes: HashMap<char, Vec<String>>,
    max_candidates: usize,
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
        let table_entries = entries
            .iter()
            .filter(|entry| !entry.code.is_empty())
            .map(ModelEntry::from_table_entry)
            .collect::<Vec<_>>();
        let mut character_codes: HashMap<char, Vec<String>> = HashMap::new();
        for entry in entries {
            let mut chars = entry.text.chars();
            let Some(ch) = chars.next() else {
                continue;
            };
            if chars.next().is_none() && !entry.code.is_empty() {
                character_codes
                    .entry(ch)
                    .or_default()
                    .push(entry.code.clone());
            }
        }
        for codes in character_codes.values_mut() {
            codes.sort();
            codes.dedup();
        }
        let vocabulary = vocabulary
            .iter()
            .filter(|entry| entry.text.chars().count() > 1)
            .map(|entry| ModelVocabularyEntry {
                text: entry.text.clone(),
                weight: entry.weight,
            })
            .collect();
        Self {
            entries: table_entries,
            vocabulary,
            character_codes,
            max_candidates: max_candidates.max(1),
        }
    }

    #[must_use]
    pub fn candidates_for_input(&self, input: &str) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }

        let graph = self.word_graph_for_input(input);
        let mut candidates = HashMap::new();
        for end in input
            .char_indices()
            .map(|(index, _)| index)
            .filter(|index| *index > 0)
            .chain(std::iter::once(input.len()))
        {
            for sentence in make_sentences(&graph, end, self.max_candidates) {
                let candidate = Candidate {
                    text: sentence.text,
                    comment: String::new(),
                    preedit: None,
                    source: if end < input.len() {
                        CandidateSource::PartialTable { consumed: end }
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
        candidates.truncate(self.max_candidates);
        candidates
    }

    fn word_graph_for_input(&self, input: &str) -> WordGraph {
        let mut graph = WordGraph::new();
        for start in input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
        {
            if start >= input.len() {
                continue;
            }
            let suffix = &input[start..];
            let mut entries = self
                .entries
                .iter()
                .filter(|entry| suffix.starts_with(&entry.code))
                .cloned()
                .collect::<Vec<_>>();
            for vocabulary_entry in &self.vocabulary {
                for code in self.derive_matching_phrase_codes(&vocabulary_entry.text, suffix) {
                    entries.push(ModelEntry {
                        text: vocabulary_entry.text.clone(),
                        code,
                        weight: vocabulary_entry.weight,
                    });
                }
            }
            for entry in entries {
                let end = start + entry.code.len();
                graph
                    .entry(start)
                    .or_default()
                    .entry(end)
                    .or_default()
                    .push(WordGraphEntry::new(
                        entry.text,
                        entry.code,
                        f64::from(entry.weight),
                    ));
            }
        }
        for edges in graph.values_mut() {
            for entries in edges.values_mut() {
                entries.sort_by(compare_word_graph_entry);
                entries.truncate(MAX_WORD_GRAPH_ENTRIES_PER_SPAN);
            }
        }
        graph
    }

    fn derive_matching_phrase_codes(&self, text: &str, input: &str) -> Vec<String> {
        let chars = text.chars().collect::<Vec<_>>();
        let mut codes = Vec::new();
        self.derive_matching_phrase_codes_from(&chars, input, 0, String::new(), &mut codes);
        codes.sort();
        codes.dedup();
        codes
    }

    fn derive_matching_phrase_codes_from(
        &self,
        chars: &[char],
        input: &str,
        index: usize,
        current: String,
        codes: &mut Vec<String>,
    ) {
        if index == chars.len() {
            if input.starts_with(&current) {
                codes.push(current);
            }
            return;
        }
        let Some(next_codes) = self.character_codes.get(&chars[index]) else {
            return;
        };
        for next_code in next_codes {
            let next = format!("{current}{next_code}");
            if input.starts_with(&next) {
                self.derive_matching_phrase_codes_from(chars, input, index + 1, next, codes);
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ModelVocabularyEntry {
    text: String,
    weight: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct ModelEntry {
    text: String,
    code: String,
    weight: f32,
}

impl ModelEntry {
    fn from_table_entry(entry: &TableEntry) -> Self {
        Self {
            text: entry.text.clone(),
            code: entry.code.clone(),
            weight: entry.weight,
        }
    }
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
