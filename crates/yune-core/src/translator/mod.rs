use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::comment_format::CommentFormat;
use crate::dictionary::normalize_table_code;
use crate::filter::contains_extended_cjk;
use crate::spelling_algebra::SpellingAlgebra;
use crate::{
    Candidate, CandidateSource, Context, RimeCorrectionEntry, RimeToleranceRule, Status,
    TableDictionary, TableDictionaryParseError, TableEntry, Translator,
};

const TYPEDUCK_CORRECTION_CREDIBILITY: f32 = -16.118_095; // log(1e-7)
const TYPEDUCK_CORRECTION_MAX_DISTANCE: usize = 4;
// Yune-internal heuristic calibrated to the M21 TypeDuck v1.1.2 sentence-composition fixture; re-validate that fixture if changed.
const TYPEDUCK_SENTENCE_WORD_PENALTY: f32 = 21.0;

#[derive(Clone, Debug, Eq, PartialEq)]
struct LookupCodeSpec {
    code: String,
    correction_distance: Option<usize>,
}

impl LookupCodeSpec {
    fn exact(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            correction_distance: None,
        }
    }

    fn correction(code: impl Into<String>, distance: usize) -> Self {
        Self {
            code: code.into(),
            correction_distance: Some(distance),
        }
    }
}

#[derive(Clone)]
struct PendingLookupCandidate {
    entry_code: String,
    lookup_code: String,
    candidate: Candidate,
    correction_distance: Option<usize>,
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

fn sentence_piece_quality(raw_quality: f32) -> f32 {
    raw_quality.max(1.0).ln() - TYPEDUCK_SENTENCE_WORD_PENALTY
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

pub struct StaticTableTranslator {
    entries: Vec<(String, Candidate)>,
    entries_by_code: BTreeMap<String, Vec<Candidate>>,
    normal_codes: HashSet<String>,
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
        let normal_codes = normal_codes(&entries);
        Self {
            entries,
            entries_by_code,
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
        }
    }

    #[must_use]
    pub fn from_dictionary(dictionary: TableDictionary) -> Self {
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
        let normal_codes = normal_codes(&entries);
        Self {
            entries,
            entries_by_code,
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
        let algebra = SpellingAlgebra::parse(formulas);
        if !algebra.is_empty() {
            let (entries, normal_codes, has_single_letter_abbreviations) =
                algebra.expand_entries_with_normal_codes(self.entries);
            self.entries = entries;
            self.entries_by_code = entries_by_code(&self.entries);
            self.normal_codes = normal_codes;
            self.single_letter_sentence_guard_enabled = has_single_letter_abbreviations;
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

    fn expanded_lookup_specs(&self, lookup_code: &str) -> Vec<LookupCodeSpec> {
        let mut specs = vec![LookupCodeSpec::exact(lookup_code)];
        if self.enable_correction {
            let mut corrections = Vec::new();
            for correction in &self.corrections {
                if correction.observed_input != lookup_code
                    || !self.normal_codes.contains(&correction.canonical_code)
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
            if self.dynamic_correction_lookup {
                for canonical_code in &self.normal_codes {
                    if canonical_code == lookup_code {
                        continue;
                    }
                    let distance = typeduck_restricted_distance(
                        canonical_code,
                        lookup_code,
                        TYPEDUCK_CORRECTION_MAX_DISTANCE,
                    );
                    if distance == 0 || distance > TYPEDUCK_CORRECTION_MAX_DISTANCE {
                        continue;
                    }
                    corrections.push((canonical_code.clone(), distance));
                }
            }
            if let Some(min_distance) = corrections.iter().map(|(_, distance)| *distance).min() {
                for (code, distance) in corrections {
                    if distance == min_distance && !specs.iter().any(|spec| spec.code == code) {
                        specs.push(LookupCodeSpec::correction(code, distance));
                    }
                }
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

    fn is_dictionary_word_allowed(&self, candidate: &Candidate) -> bool {
        !self.dictionary_exclude.contains(&candidate.text)
    }

    fn is_prediction_candidate_allowed(&self, lookup_code: &str, candidate: &Candidate) -> bool {
        match self.prediction_weight_threshold {
            Some(threshold) if candidate.quality < threshold => return false,
            _ => {}
        }
        if self.prediction_candidate_limit.is_some()
            && complete_syllable_prefix_count(&candidate.comment, lookup_code).is_none()
        {
            return false;
        }
        true
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
        let mut candidate = candidate.clone();
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

    fn candidates_for_lookup_codes(
        &self,
        lookup_specs: &[LookupCodeSpec],
        filter_by_charset: bool,
    ) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        for lookup_spec in lookup_specs {
            let lookup_code = lookup_spec.code.as_str();
            let mut pending = Vec::new();
            if let Some(exact_candidates) = self.entries_by_code.get(lookup_code) {
                pending.extend(
                    exact_candidates
                        .iter()
                        .filter(|candidate| {
                            self.is_dictionary_word_allowed(candidate)
                                && (!filter_by_charset || !contains_extended_cjk(&candidate.text))
                        })
                        .map(|candidate| PendingLookupCandidate {
                            entry_code: lookup_code.to_owned(),
                            lookup_code: lookup_code.to_owned(),
                            candidate: candidate.clone(),
                            correction_distance: lookup_spec.correction_distance,
                        }),
                );
            }
            if lookup_spec.correction_distance.is_none()
                && self.enable_completion
                && !lookup_code.is_empty()
            {
                let mut completion_candidates = Vec::new();
                for (entry_code, entry_candidates) in
                    self.entries_by_code.range(lookup_code.to_owned()..)
                {
                    if !entry_code.starts_with(lookup_code) {
                        break;
                    }
                    if entry_code == lookup_code {
                        continue;
                    }
                    completion_candidates.extend(
                        entry_candidates
                            .iter()
                            .filter(|candidate| {
                                self.is_dictionary_word_allowed(candidate)
                                    && self.is_prediction_candidate_allowed(lookup_code, candidate)
                                    && (!filter_by_charset
                                        || !contains_extended_cjk(&candidate.text))
                            })
                            .map(|candidate| PendingLookupCandidate {
                                entry_code: entry_code.clone(),
                                lookup_code: lookup_code.to_owned(),
                                candidate: candidate.clone(),
                                correction_distance: lookup_spec.correction_distance,
                            }),
                    );
                }
                if let Some(limit) = self.prediction_candidate_limit {
                    completion_candidates.sort_by(|left, right| {
                        right
                            .raw_quality()
                            .partial_cmp(&left.raw_quality())
                            .unwrap_or(Ordering::Equal)
                            .then_with(|| left.entry_code.cmp(&right.entry_code))
                            .then_with(|| left.candidate.text.cmp(&right.candidate.text))
                    });
                    completion_candidates.truncate(limit);
                }
                pending.extend(completion_candidates);
            }
            if self.prediction_candidate_limit.is_some() {
                pending.sort_by(|left, right| {
                    right
                        .raw_quality()
                        .partial_cmp(&left.raw_quality())
                        .unwrap_or(Ordering::Equal)
                        .then_with(|| left.entry_code.cmp(&right.entry_code))
                        .then_with(|| left.candidate.text.cmp(&right.candidate.text))
                });
            }
            candidates.extend(pending.into_iter().map(|pending| {
                self.candidate_for_lookup(
                    &pending.entry_code,
                    &pending.candidate,
                    &pending.lookup_code,
                    pending.correction_distance,
                )
            }));
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
        let Some((prefix, consumed_lookup_len)) = self.longest_valid_lookup_prefix(lookup_code)
        else {
            return Vec::new();
        };
        let Some(prefix_candidates) = self.entries_by_code.get(prefix) else {
            return Vec::new();
        };
        let consumed_input_len = input
            .len()
            .saturating_sub(lookup_code.len())
            .saturating_add(consumed_lookup_len);
        let mut seen_texts = existing_candidates
            .iter()
            .map(|candidate| candidate.text.clone())
            .collect::<HashSet<_>>();
        let mut candidates = Vec::new();
        for candidate in prefix_candidates.iter().filter(|candidate| {
            self.is_dictionary_word_allowed(candidate)
                && original_code_allows_prefix_fallback(&candidate.comment, prefix)
                && (!filter_by_charset || !contains_extended_cjk(&candidate.text))
        }) {
            let mut candidate = self.candidate_for_lookup(prefix, candidate, prefix, None);
            if !seen_texts.insert(candidate.text.clone()) {
                continue;
            }
            candidate.source = CandidateSource::PartialTable {
                consumed: consumed_input_len,
            };
            candidates.push(candidate);
        }
        candidates
    }

    fn longest_valid_lookup_prefix<'a>(&self, lookup_code: &'a str) -> Option<(&'a str, usize)> {
        let mut boundaries = lookup_code
            .char_indices()
            .map(|(index, _)| index)
            .filter(|index| *index > 0)
            .collect::<Vec<_>>();
        boundaries.reverse();
        for end in boundaries {
            let prefix = &lookup_code[..end];
            if self.entries_by_code.contains_key(prefix) {
                return Some((prefix, end));
            }
        }
        None
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
            let should_add_prefix_fallback =
                candidates.is_empty() || (!used_sentence && has_full_exact_candidate);
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

    fn sentence_candidate(
        &self,
        input: &str,
        filter_by_charset: bool,
        priority_floor: Option<f32>,
    ) -> Option<Candidate> {
        if input.is_empty() {
            return None;
        }

        #[derive(Clone)]
        struct SentencePath {
            quality: f32,
            raw_quality: f32,
            pieces: Vec<String>,
        }

        let mut paths: Vec<Option<SentencePath>> = vec![None; input.len() + 1];
        paths[0] = Some(SentencePath {
            quality: 0.0,
            raw_quality: 0.0,
            pieces: Vec::new(),
        });
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
                let is_final_segment = end == input.len();
                // In abbreviation-bearing schemas, generated one-letter aliases are lookup
                // shortcuts, not stable interior sentence boundaries.
                if !is_final_segment
                    && self.single_letter_sentence_guard_enabled
                    && entry_code.len() == 1
                {
                    continue;
                }
                let mut entry_matches = Vec::new();
                if let Some(candidates) = self.entries_by_code.get(entry_code) {
                    entry_matches.push(candidates.as_slice());
                }
                if is_final_segment && self.enable_completion && !entry_code.is_empty() {
                    for (completion_code, completion_candidates) in
                        self.entries_by_code.range(entry_code.to_owned()..)
                    {
                        if !completion_code.starts_with(entry_code) {
                            break;
                        }
                        if completion_code == entry_code {
                            continue;
                        }
                        entry_matches.push(completion_candidates.as_slice());
                    }
                }
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
                for candidates in entry_matches {
                    for candidate in candidates {
                        if !self.is_dictionary_word_allowed(candidate)
                            || (filter_by_charset && contains_extended_cjk(&candidate.text))
                        {
                            continue;
                        }
                        let mut next_path = path.clone();
                        next_path.quality += sentence_piece_quality(candidate.quality);
                        next_path.raw_quality += candidate.quality;
                        next_path.pieces.push(candidate.text.clone());
                        if is_final_segment && next_path.pieces.len() <= 1 {
                            continue;
                        }
                        let replace = match paths[end_pos].as_ref() {
                            Some(existing) => match next_path
                                .quality
                                .partial_cmp(&existing.quality)
                                .unwrap_or(Ordering::Equal)
                            {
                                Ordering::Greater => true,
                                Ordering::Equal => next_path.raw_quality > existing.raw_quality,
                                Ordering::Less => false,
                            },
                            None => true,
                        };
                        if replace {
                            paths[end_pos] = Some(next_path);
                        }
                    }
                }
            }
        }

        let path = paths[input.len()].take()?;
        if path.pieces.len() <= 1 {
            return None;
        }
        let quality = priority_floor
            .map(|floor| floor + 1.0)
            .unwrap_or(path.quality.max(1.0) + self.initial_quality);
        Some(Candidate {
            text: path.pieces.join(""),
            comment: " ☯ ".to_owned(),
            preedit: None,
            source: CandidateSource::Sentence,
            quality,
        })
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
}

pub struct ReverseLookupTranslator {
    entries: Vec<TableEntry>,
    reverse_comments: HashMap<String, Vec<String>>,
    prefix: String,
    suffix: String,
    tag: String,
    enable_completion: bool,
    comment_format: CommentFormat,
}

impl ReverseLookupTranslator {
    #[must_use]
    pub fn new(
        dictionary: TableDictionary,
        reverse_dictionary: Option<TableDictionary>,
        prefix: impl Into<String>,
        suffix: impl Into<String>,
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
            prefix: prefix.into(),
            suffix: suffix.into(),
            tag: "reverse_lookup".to_owned(),
            enable_completion: false,
            comment_format: CommentFormat::default(),
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

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        segment_tags
            .iter()
            .any(|segment_tag| segment_tag == &self.tag)
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

        self.entries
            .iter()
            .filter(|entry| {
                if self.enable_completion {
                    entry.code.starts_with(&code)
                } else {
                    entry.code == code
                }
            })
            .map(|entry| {
                let comment = self
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

fn options_get_bool(options: &HashMap<String, bool>, option: &str) -> bool {
    options.get(option).copied().unwrap_or(false)
}
