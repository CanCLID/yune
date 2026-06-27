use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::punctuation::{
    punctuation_candidate_comment, PunctuationProcessor, PunctuationProcessorResult,
};
use crate::AiContext;
use crate::{
    parse_key_sequence, AiDecision, AiResult, Candidate, CandidateFilter, CandidateRanker,
    CandidateRequest, CandidateSource, CommitRecord, Composition, Context, EchoTranslator,
    EngineInspectorSnapshot, FilterAuditRecord, KeyCode, KeyEvent, KeyModifiers,
    KeySequenceParseError, MemoryOwnerRow, MemoryStore, PageSnapshot, RerankResult, SegmentDebug,
    Snapshot, StagedAiCandidates, Status, Translator, UserDb, UserDbCommitMetadata,
    UserDbLookupRequest, UserDbLookupResult,
};

pub struct Engine {
    context: Context,
    status: Status,
    options: HashMap<String, bool>,
    properties: HashMap<String, String>,
    translators: Vec<Arc<dyn Translator>>,
    punctuation_processor: Option<PunctuationProcessor>,
    filters: Vec<Box<dyn CandidateFilter>>,
    last_filter_audit: Vec<FilterAuditRecord>,
    rankers: Vec<Box<dyn CandidateRanker>>,
    staged_ai_result: Option<StagedAiCandidates>,
    ai_memory: MemoryStore,
    userdb: UserDb,
    prediction_never_first: bool,
    candidate_list_complete: bool,
    pending_userdb_learning: Option<UserDbCommitMetadata>,
    commit_tick: u64,
}

const DEFAULT_PAGE_SIZE: usize = 5;
const BOUNDED_REFRESH_SURPLUS: usize = 15;
const M44_SHORT_KEY_REFRESH_SURPLUS: usize = 2;
const TYPEDUCK_E_SQUARED: f32 = 7.389_056;
const TYPEDUCK_EXP_E_SQUARED: f32 = 1618.178;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommitIntent {
    DefaultConfirm,
    ExplicitSelection,
}

fn clamp_to_char_boundary(input: &str, caret: usize) -> usize {
    let mut caret = caret.min(input.len());
    while caret > 0 && !input.is_char_boundary(caret) {
        caret -= 1;
    }
    caret
}

fn previous_char_boundary(input: &str, caret: usize) -> Option<usize> {
    let caret = clamp_to_char_boundary(input, caret);
    input[..caret].char_indices().last().map(|(index, _)| index)
}

fn next_char_boundary(input: &str, caret: usize) -> Option<usize> {
    let caret = clamp_to_char_boundary(input, caret);
    input[caret..]
        .chars()
        .next()
        .map(|character| caret + character.len_utf8())
}

fn learning_code_for_candidate(input: &str, candidate: &Candidate) -> String {
    let primary_records = primary_dictionary_lookup_records(&candidate.comment);
    if primary_records.is_empty() {
        compact_dictionary_lookup_code(&candidate.comment).unwrap_or_else(|| input.to_owned())
    } else if candidate.source == CandidateSource::Sentence {
        let component_records = primary_records
            .iter()
            .filter(|record| !record.is_composition)
            .collect::<Vec<_>>();
        let exact_codes = primary_records
            .iter()
            .filter(|record| record.text == candidate.text && !record.is_composition)
            .map(|record| record.code.as_str())
            .collect::<Vec<_>>();
        if exact_codes.is_empty() {
            component_records
                .iter()
                .map(|record| record.code.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            exact_codes.join(" ")
        }
    } else {
        primary_records[0].code.clone()
    }
}

fn compact_dictionary_lookup_code(comment: &str) -> Option<String> {
    let code = comment
        .strip_prefix('\u{000c}')?
        .split('\r')
        .find(|part| !part.is_empty())?
        .trim();
    (!code.is_empty() && !code.contains(',')).then(|| code.to_owned())
}

fn partial_consumed_len_for_commit(
    input: &str,
    candidate: &Candidate,
    intent: CommitIntent,
) -> Option<usize> {
    let consumed = match intent {
        CommitIntent::ExplicitSelection => candidate.source.partial_consumed_len()?,
        CommitIntent::DefaultConfirm if candidate.source.recomposes_on_default() => {
            candidate.source.partial_consumed_len()?
        }
        CommitIntent::DefaultConfirm => return None,
    };
    if consumed == 0 || consumed >= input.len() || !input.is_char_boundary(consumed) {
        return None;
    }
    Some(consumed)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrimaryDictionaryLookupRecord {
    text: String,
    code: String,
    is_composition: bool,
}

fn primary_dictionary_lookup_records(comment: &str) -> Vec<PrimaryDictionaryLookupRecord> {
    let Some((_, lookup_comment)) = comment.split_once('\u{000c}') else {
        return Vec::new();
    };
    lookup_comment
        .split('\r')
        .filter(|record| !record.is_empty())
        .filter_map(|record| {
            let (flag, fields) = record.split_once(',')?;
            if flag != "1" {
                return None;
            }
            let fields = fields.split(',').collect::<Vec<_>>();
            let text = fields.first()?.trim();
            let code = fields.get(1)?.trim();
            let is_composition = fields.get(7).is_some_and(|field| *field == "composition");
            (!code.is_empty()).then(|| PrimaryDictionaryLookupRecord {
                text: text.to_owned(),
                code: code.to_owned(),
                is_composition,
            })
        })
        .collect()
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            context: Context::default(),
            status: Status::default(),
            options: HashMap::new(),
            properties: HashMap::new(),
            translators: vec![Arc::new(EchoTranslator)],
            punctuation_processor: None,
            filters: Vec::new(),
            last_filter_audit: Vec::new(),
            rankers: Vec::new(),
            staged_ai_result: None,
            ai_memory: MemoryStore::default(),
            userdb: UserDb::default(),
            prediction_never_first: false,
            candidate_list_complete: true,
            pending_userdb_learning: None,
            commit_tick: 0,
        }
    }
}

impl Engine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_translator(&mut self, translator: impl Translator + 'static) {
        self.add_shared_translator(Arc::new(translator));
    }

    pub fn add_shared_translator(&mut self, translator: Arc<dyn Translator>) {
        let insert_at = self
            .translators
            .iter()
            .position(|existing| existing.name() == "echo_translator")
            .unwrap_or(self.translators.len());
        self.translators.insert(insert_at, translator);
        self.refresh_candidates();
    }

    pub fn set_punctuation_processor(&mut self, processor: PunctuationProcessor) {
        self.punctuation_processor = Some(processor);
    }

    pub fn reset_translators(&mut self) {
        self.translators = vec![Arc::new(EchoTranslator)];
        self.refresh_candidates();
    }

    pub fn clear_translators(&mut self) {
        self.translators.clear();
        self.refresh_candidates();
    }

    pub fn add_filter(&mut self, filter: impl CandidateFilter + 'static) {
        self.filters.push(Box::new(filter));
        self.refresh_candidates();
    }

    pub fn reset_filters(&mut self) {
        self.filters.clear();
        self.refresh_candidates();
    }

    pub fn add_ranker(&mut self, ranker: impl CandidateRanker + 'static) {
        self.rankers.push(Box::new(ranker));
        self.refresh_candidates();
    }

    pub fn stage_ai_result(&mut self, result: AiResult) -> AiDecision {
        let decision = match result {
            AiResult::Off { for_input, .. } => {
                if for_input == self.context.composition.input {
                    self.staged_ai_result = None;
                    AiDecision::Off
                } else {
                    self.ai_decision_for_current_input()
                }
            }
            AiResult::Pending { for_input } => {
                if for_input == self.context.composition.input {
                    self.staged_ai_result = None;
                    AiDecision::Pending
                } else {
                    self.ai_decision_for_current_input()
                }
            }
            AiResult::Ready {
                for_input,
                candidates,
            } => {
                let staged = StagedAiCandidates {
                    for_input,
                    candidates,
                };
                let decision = if staged.matches_input(&self.context.composition.input) {
                    AiDecision::Ready
                } else {
                    AiDecision::Pending
                };
                self.staged_ai_result = Some(staged);
                decision
            }
        };
        self.refresh_candidates();
        decision
    }

    fn ai_decision_for_current_input(&self) -> AiDecision {
        self.staged_ai_result
            .as_ref()
            .map_or(AiDecision::Off, |staged| {
                if staged.matches_input(&self.context.composition.input) {
                    AiDecision::Ready
                } else {
                    AiDecision::Pending
                }
            })
    }

    pub fn set_schema(&mut self, id: impl Into<String>, name: impl Into<String>) {
        self.status.schema_id = id.into();
        self.status.schema_name = name.into();
    }

    pub fn set_userdb(&mut self, userdb: UserDb) {
        self.userdb = userdb;
        self.refresh_candidates();
    }

    #[must_use]
    pub fn userdb(&self) -> &UserDb {
        &self.userdb
    }

    pub fn set_prediction_never_first(&mut self, prediction_never_first: bool) {
        self.prediction_never_first = prediction_never_first;
        self.refresh_candidates();
    }

    pub fn take_pending_userdb_learning(&mut self) -> Option<UserDbCommitMetadata> {
        self.pending_userdb_learning.take()
    }

    #[must_use]
    pub fn ai_memory(&self) -> &MemoryStore {
        &self.ai_memory
    }

    #[must_use]
    pub fn memory_owner_rows(&self) -> Vec<MemoryOwnerRow> {
        self.translators
            .iter()
            .flat_map(|translator| translator.memory_owner_rows())
            .collect()
    }

    pub fn set_ai_memory(&mut self, memory_store: MemoryStore) {
        self.ai_memory = memory_store;
    }

    pub fn set_ai_memory_enabled(&mut self, enabled: bool) {
        self.ai_memory.set_enabled(enabled);
    }

    pub fn clear_ai_memory(&mut self) {
        self.ai_memory.clear();
    }

    pub fn set_option(&mut self, option: impl Into<String>, value: bool) {
        let option = option.into();
        match option.as_str() {
            "disabled" => self.status.is_disabled = value,
            "ascii_mode" => self.status.is_ascii_mode = value,
            "full_shape" => self.status.is_full_shape = value,
            "simplification" | "simplified" => self.status.is_simplified = value,
            "traditionalization" | "traditional" => self.status.is_traditional = value,
            "ascii_punct" => self.status.is_ascii_punct = value,
            _ => {}
        }
        self.options.insert(option, value);
        self.refresh_candidates();
    }

    #[must_use]
    pub fn get_option(&self, option: &str) -> bool {
        match option {
            "disabled" => self.status.is_disabled,
            "ascii_mode" => self.status.is_ascii_mode,
            "full_shape" => self.status.is_full_shape,
            "simplification" | "simplified" => self.status.is_simplified,
            "traditionalization" | "traditional" => self.status.is_traditional,
            "ascii_punct" => self.status.is_ascii_punct,
            _ => self.options.get(option).copied().unwrap_or(false),
        }
    }

    pub fn set_property(&mut self, property: impl Into<String>, value: impl Into<String>) {
        self.properties.insert(property.into(), value.into());
    }

    pub fn set_segment_tags(&mut self, tags: impl IntoIterator<Item = impl Into<String>>) {
        self.context.segment_tags = tags.into_iter().map(Into::into).collect();
        if self.context.segment_tags.is_empty() {
            self.context.segment_tags.push("abc".to_owned());
        }
        self.refresh_candidates();
    }

    pub fn set_ai_context(&mut self, ai_context: AiContext) {
        self.context.ai_context = ai_context;
    }

    pub fn clear_ai_context(&mut self) {
        self.context.ai_context = AiContext::default();
    }

    #[must_use]
    pub fn get_property(&self, property: &str) -> Option<&str> {
        self.properties.get(property).map(String::as_str)
    }

    pub fn process_char(&mut self, ch: char) -> Option<String> {
        match ch {
            '\u{8}' | '\u{7f}' => self.backspace(),
            ' ' => self.commit_highlighted(),
            '0'..='9' if self.has_selectable_candidates() => {
                self.commit_candidate_at_page_index(select_index_from_digit(ch))
            }
            _ if !ch.is_control() => {
                self.context.composition.input.push(ch);
                self.context.composition.caret = self.context.composition.input.len();
                self.context.composition.preedit = self.context.composition.input.clone();
                self.refresh_candidates();
                None
            }
            _ => None,
        }
    }

    pub fn process_key_event(&mut self, key_event: KeyEvent) -> Option<String> {
        if is_exact_control_shift_modifier(key_event.modifiers) && key_event.code == KeyCode::Return
        {
            return self.commit_comment();
        }
        if is_exact_control_shift_modifier(key_event.modifiers) {
            match key_event.code {
                KeyCode::Character(ch)
                    if ch.is_ascii_digit() && self.has_selectable_candidates() =>
                {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                KeyCode::KeypadDigit(ch) if self.has_selectable_candidates() => {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                _ => {}
            }
        }

        if is_exact_shift_modifier(key_event.modifiers) {
            match key_event.code {
                KeyCode::Return => {
                    return self.commit_script_text();
                }
                KeyCode::Backspace => {
                    return self.backspace();
                }
                KeyCode::Delete => {
                    self.delete_candidate(self.context.highlighted);
                    return None;
                }
                KeyCode::Escape => {
                    self.clear_composition();
                    return None;
                }
                KeyCode::MoveCaretLeft => {
                    self.move_caret_left_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretRight => {
                    self.move_caret_right_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretLeftBySyllable => {
                    self.move_caret_left_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretRightBySyllable => {
                    self.move_caret_right_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretLeftByChar => {
                    self.move_caret_left_by_char();
                    return None;
                }
                KeyCode::MoveCaretRightByChar => {
                    self.move_caret_right_by_char();
                    return None;
                }
                KeyCode::PreviousCandidate => {
                    self.move_caret_left_by_char();
                    return None;
                }
                KeyCode::NextCandidate => {
                    self.move_caret_right_by_char();
                    return None;
                }
                KeyCode::Home => {
                    self.move_caret_home();
                    return None;
                }
                KeyCode::End => {
                    self.move_caret_end();
                    return None;
                }
                KeyCode::Character(ch) if ch == ' ' || is_printable_ascii(ch) => {
                    return self.process_char(ch);
                }
                KeyCode::KeypadDigit(ch) if self.has_selectable_candidates() => {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                _ => {}
            }
        }

        if is_exact_control_modifier(key_event.modifiers) {
            match key_event.code {
                KeyCode::Backspace => {
                    return self.backspace();
                }
                KeyCode::Delete => {
                    self.delete_candidate(self.context.highlighted);
                    return None;
                }
                KeyCode::Return => {
                    return self.commit_raw_input();
                }
                KeyCode::MoveCaretLeft => {
                    self.move_caret_left_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretRight => {
                    self.move_caret_right_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretLeftBySyllable => {
                    self.move_caret_left_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretRightBySyllable => {
                    self.move_caret_right_by_syllable();
                    return None;
                }
                KeyCode::Character(ch)
                    if ch.is_ascii_digit() && self.has_selectable_candidates() =>
                {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                KeyCode::KeypadDigit(ch) if self.has_selectable_candidates() => {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                _ => {}
            }
        }

        if !key_event.modifiers.is_empty() {
            return None;
        }

        if let Some(result) = self.process_punctuation_processor(key_event) {
            return match result {
                PunctuationProcessorResult::Accepted => None,
                PunctuationProcessorResult::Preview(text) => {
                    self.set_punctuation_preview(text);
                    None
                }
                PunctuationProcessorResult::Candidates {
                    input,
                    texts,
                    highlighted,
                } => {
                    self.set_punctuation_candidate_list(input, texts, highlighted);
                    None
                }
                PunctuationProcessorResult::Commit(commit) => Some(self.record_commit(commit)),
            };
        }

        match key_event.code {
            KeyCode::Character(ch) => self.process_char(ch),
            KeyCode::KeypadDigit(ch) if self.has_selectable_candidates() => {
                self.commit_candidate_at_page_index(select_index_from_digit(ch))
            }
            KeyCode::KeypadDigit(_) => None,
            KeyCode::Tab => None,
            KeyCode::Ignored => None,
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete_at_caret(),
            KeyCode::Escape => {
                self.clear_composition();
                None
            }
            KeyCode::MoveCaretLeft => {
                self.move_caret_left();
                None
            }
            KeyCode::MoveCaretRight => {
                self.move_caret_right();
                None
            }
            KeyCode::MoveCaretLeftByChar => {
                self.move_caret_left_by_char();
                None
            }
            KeyCode::MoveCaretRightByChar => {
                self.move_caret_right_by_char();
                None
            }
            KeyCode::MoveCaretLeftBySyllable => {
                self.move_caret_left_by_syllable();
                None
            }
            KeyCode::MoveCaretRightBySyllable => {
                self.move_caret_right_by_syllable();
                None
            }
            KeyCode::Home => {
                if !self.first_candidate() {
                    self.move_caret_home();
                }
                None
            }
            KeyCode::End => {
                if self.context.composition.caret < self.context.composition.input.len()
                    || !self.first_candidate()
                {
                    self.move_caret_end();
                }
                None
            }
            KeyCode::PreviousCandidate => {
                self.previous_candidate();
                None
            }
            KeyCode::NextCandidate => {
                self.next_candidate();
                None
            }
            KeyCode::FirstCandidate => {
                self.first_candidate();
                None
            }
            KeyCode::PreviousPage => {
                self.change_page(true);
                None
            }
            KeyCode::NextPage => {
                self.change_page(false);
                None
            }
            KeyCode::Return | KeyCode::KeypadEnter => self.commit_highlighted(),
        }
    }

    fn process_punctuation_processor(
        &mut self,
        key_event: KeyEvent,
    ) -> Option<PunctuationProcessorResult> {
        let KeyCode::Character(ch) = key_event.code else {
            return None;
        };
        if !is_printable_ascii(ch) {
            return None;
        }
        let key = ch.to_string();
        self.punctuation_processor.as_mut()?.process_key(
            &key,
            self.status.is_full_shape,
            self.status.is_ascii_punct,
            &self.context.composition.input,
            self.context.candidates.len(),
            self.context.highlighted,
        )
    }

    pub fn process_sequence(&mut self, input: &str) -> Vec<String> {
        input
            .chars()
            .filter_map(|ch| self.process_char(ch))
            .collect()
    }

    pub fn process_key_sequence(
        &mut self,
        input: &str,
    ) -> Result<Vec<String>, KeySequenceParseError> {
        Ok(parse_key_sequence(input)?
            .into_iter()
            .filter_map(|key_event| self.process_key_event(key_event))
            .collect())
    }

    pub fn commit_composition(&mut self) -> Option<String> {
        self.commit_highlighted()
    }

    pub fn commit_raw_input(&mut self) -> Option<String> {
        self.commit_raw_input_text()
    }

    pub fn select_candidate(&mut self, index: usize) -> Option<String> {
        if index >= self.context.candidates.len() && !self.candidate_list_complete {
            self.ensure_complete_candidates();
        }
        self.commit_candidate(index, CommitIntent::ExplicitSelection)
    }

    pub fn select_candidate_on_current_page(&mut self, index: usize) -> Option<String> {
        self.commit_candidate_at_page_index(index)
    }

    pub fn highlight_candidate(&mut self, index: usize) -> bool {
        if index >= self.context.candidates.len() && !self.candidate_list_complete {
            self.ensure_complete_candidates();
        }
        if index >= self.context.candidates.len() {
            return false;
        }
        self.context.highlighted = index;
        true
    }

    pub fn highlight_candidate_on_current_page(&mut self, index: usize) -> bool {
        if index >= DEFAULT_PAGE_SIZE {
            return false;
        }
        let page_start = (self.context.highlighted / DEFAULT_PAGE_SIZE) * DEFAULT_PAGE_SIZE;
        self.highlight_candidate(page_start + index)
    }

    pub fn delete_candidate(&mut self, index: usize) -> bool {
        if !self.candidate_list_complete {
            self.ensure_complete_candidates();
        }
        if index >= self.context.candidates.len() {
            return false;
        }
        self.context.candidates.remove(index);
        if self.context.candidates.is_empty() {
            self.context.highlighted = 0;
        } else if index < self.context.highlighted {
            self.context.highlighted -= 1;
        } else if self.context.highlighted >= self.context.candidates.len() {
            self.context.highlighted = self.context.candidates.len() - 1;
        }
        true
    }

    pub fn delete_candidate_on_current_page(&mut self, index: usize) -> bool {
        if index >= DEFAULT_PAGE_SIZE {
            return false;
        }
        let page_start = (self.context.highlighted / DEFAULT_PAGE_SIZE) * DEFAULT_PAGE_SIZE;
        self.delete_candidate(page_start + index)
    }

    pub fn change_page(&mut self, backward: bool) -> bool {
        self.change_page_by(DEFAULT_PAGE_SIZE, backward)
    }

    pub fn change_page_by(&mut self, page_size: usize, backward: bool) -> bool {
        if !self.has_selectable_candidates() {
            return false;
        }

        let page_size = page_size.max(1);
        if !backward && !self.candidate_list_complete {
            self.ensure_complete_candidates();
        }
        let current_index = self.context.highlighted;
        let next_index = if backward {
            current_index.saturating_sub(page_size)
        } else {
            current_index + page_size
        };
        let next_index = next_index.min(self.context.candidates.len() - 1);
        if current_index == next_index {
            return false;
        }
        self.highlight_candidate(next_index)
    }

    pub fn previous_candidate(&mut self) -> bool {
        if !self.has_selectable_candidates() {
            return false;
        }
        if self.context.highlighted == 0 {
            return true;
        }
        self.highlight_candidate(self.context.highlighted - 1)
    }

    pub fn next_candidate(&mut self) -> bool {
        if !self.has_selectable_candidates() {
            return false;
        }
        let next_index = self.context.highlighted + 1;
        if next_index >= self.context.candidates.len() && !self.candidate_list_complete {
            self.ensure_complete_candidates();
        }
        if next_index >= self.context.candidates.len() {
            return true;
        }
        self.highlight_candidate(next_index)
    }

    pub fn first_candidate(&mut self) -> bool {
        if !self.has_selectable_candidates() {
            return false;
        }
        if self.context.highlighted == 0 {
            return false;
        }
        self.highlight_candidate(0)
    }

    fn has_selectable_candidates(&self) -> bool {
        !self.context.candidates.is_empty()
            && !self.context.segment_tags.iter().any(|tag| tag == "raw")
    }

    pub fn clear_composition(&mut self) {
        self.context.composition = Composition::default();
        self.context.candidates.clear();
        self.context.highlighted = 0;
        self.candidate_list_complete = true;
        self.staged_ai_result = None;
    }

    pub fn set_input(&mut self, input: impl Into<String>) {
        let input = input.into();
        self.staged_ai_result = None;
        self.context.composition.input = input.clone();
        self.context.composition.caret = input.len();
        self.context.composition.preedit = input;
        self.refresh_candidates();
    }

    pub fn set_punctuation_composition(
        &mut self,
        input: impl Into<String>,
        text: impl Into<String>,
    ) {
        let input = input.into();
        let text = text.into();
        self.staged_ai_result = None;
        self.context.composition.input = input.clone();
        self.context.composition.caret = input.len();
        self.context.composition.preedit = input;
        self.context.candidates = vec![Candidate {
            comment: punctuation_candidate_comment(&text).to_owned(),
            text,
            preedit: None,
            source: CandidateSource::Punctuation,
            quality: 1.0,
        }];
        self.context.highlighted = 0;
        self.candidate_list_complete = true;
    }

    pub fn set_punctuation_preview(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.staged_ai_result = None;
        self.context.composition.input = text.clone();
        self.context.composition.caret = text.len();
        self.context.composition.preedit = text;
        self.context.candidates.clear();
        self.context.highlighted = 0;
        self.candidate_list_complete = true;
    }

    pub fn set_punctuation_candidate_list(
        &mut self,
        input: impl Into<String>,
        texts: impl IntoIterator<Item = impl Into<String>>,
        highlighted: usize,
    ) {
        let input = input.into();
        self.staged_ai_result = None;
        self.context.composition.input = input.clone();
        self.context.composition.caret = input.len();
        self.context.composition.preedit = input;
        self.context.candidates = texts
            .into_iter()
            .map(|text| {
                let text = text.into();
                Candidate {
                    comment: punctuation_candidate_comment(&text).to_owned(),
                    text,
                    preedit: None,
                    source: CandidateSource::Punctuation,
                    quality: 1.0,
                }
            })
            .collect();
        self.context.highlighted = highlighted.min(self.context.candidates.len().saturating_sub(1));
        self.candidate_list_complete = true;
    }

    pub fn record_commit(&mut self, text: impl Into<String>) -> String {
        let text = text.into();
        self.record_commit_with_type("raw", text.clone(), String::new());
        self.clear_composition();
        text
    }

    pub fn set_caret_pos(&mut self, caret_pos: usize) {
        self.context.composition.caret =
            clamp_to_char_boundary(&self.context.composition.input, caret_pos);
    }

    pub fn move_caret_left(&mut self) -> bool {
        let Some(previous) = previous_char_boundary(
            &self.context.composition.input,
            self.context.composition.caret,
        ) else {
            return false;
        };
        self.context.composition.caret = previous;
        true
    }

    pub fn move_caret_right(&mut self) -> bool {
        let Some(next) = next_char_boundary(
            &self.context.composition.input,
            self.context.composition.caret,
        ) else {
            return false;
        };
        self.context.composition.caret = next;
        true
    }

    pub fn move_caret_left_by_char(&mut self) -> bool {
        if self.move_caret_left() {
            return true;
        }
        if self.context.composition.input.is_empty()
            || self.context.composition.caret == self.context.composition.input.len()
        {
            return false;
        }
        self.context.composition.caret = self.context.composition.input.len();
        true
    }

    pub fn move_caret_right_by_char(&mut self) -> bool {
        if self.move_caret_right() {
            return true;
        }
        if self.context.composition.input.is_empty() || self.context.composition.caret == 0 {
            return false;
        }
        self.context.composition.caret = 0;
        true
    }

    pub fn move_caret_left_by_syllable(&mut self) -> bool {
        if self.context.composition.input.is_empty() || self.context.composition.caret == 0 {
            return false;
        }
        self.context.composition.caret = 0;
        true
    }

    pub fn move_caret_right_by_syllable(&mut self) -> bool {
        if self.context.composition.caret >= self.context.composition.input.len() {
            return false;
        }
        self.context.composition.caret = self.context.composition.input.len();
        true
    }

    pub fn move_caret_home(&mut self) -> bool {
        if self.context.composition.caret == 0 {
            return false;
        }
        self.context.composition.caret = 0;
        true
    }

    pub fn move_caret_end(&mut self) -> bool {
        if self.context.composition.caret >= self.context.composition.input.len() {
            return false;
        }
        self.context.composition.caret = self.context.composition.input.len();
        true
    }

    #[must_use]
    pub fn context(&self) -> &Context {
        &self.context
    }

    #[must_use]
    pub fn status(&self) -> Status {
        let mut status = self.status.clone();
        status.is_composing = !self.context.composition.input.is_empty();
        status
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        crate::m37_record_context_full_snapshot_clone(self.context.candidates.len());
        Snapshot {
            context: self.context.clone(),
            status: self.status(),
            candidate_list_complete: self.candidate_list_complete,
        }
    }

    #[must_use]
    pub fn page_snapshot(&self, page_size: usize) -> PageSnapshot {
        let page_size = page_size.max(1);
        let highlighted = self.context.highlighted;
        let page_no = highlighted / page_size;
        let page_start = page_no * page_size;
        let page_end = (page_start + page_size).min(self.context.candidates.len());
        let page_candidates = self.context.candidates[page_start..page_end].to_vec();
        crate::m37_record_context_page_snapshot_clone(page_candidates.len());
        let page_highlighted = highlighted
            .saturating_sub(page_start)
            .min(page_candidates.len().saturating_sub(1));
        let context = Context {
            composition: self.context.composition.clone(),
            segment_tags: self.context.segment_tags.clone(),
            candidates: page_candidates,
            highlighted: page_highlighted,
            last_commit: self.context.last_commit.clone(),
            commit_history: self.context.commit_history.clone(),
            ai_context: self.context.ai_context.clone(),
        };
        PageSnapshot {
            context,
            status: self.status(),
            candidate_list_complete: self.candidate_list_complete,
            page_size,
            page_no,
            page_start,
            retained_candidate_count: self.context.candidates.len(),
        }
    }

    #[must_use]
    pub fn candidate_list_complete(&self) -> bool {
        self.candidate_list_complete
    }

    pub fn ensure_complete_candidate_list(&mut self) {
        self.ensure_complete_candidates();
    }

    #[must_use]
    pub fn inspector_snapshot(&self) -> EngineInspectorSnapshot {
        let input = &self.context.composition.input;
        let segment_tags = self.context.segment_tags.clone();
        let segments = if input.is_empty() {
            Vec::new()
        } else {
            segment_tags
                .iter()
                .map(|tag| SegmentDebug {
                    start: 0,
                    end: input.len(),
                    tag: tag.clone(),
                    source: "context.segment_tags".to_owned(),
                })
                .collect()
        };
        let filter_pipeline = self
            .filters
            .iter()
            .map(|filter| filter.name().to_owned())
            .collect();
        let spelling_algebra = self
            .translators
            .iter()
            .filter_map(|translator| translator.spelling_algebra_debug(input))
            .collect();
        let prediction_weight_threshold = self
            .translators
            .iter()
            .find_map(|translator| translator.prediction_weight_threshold());

        EngineInspectorSnapshot {
            segment_tags,
            segments,
            filter_pipeline,
            filter_audit: self.last_filter_audit.clone(),
            spelling_algebra,
            prediction_weight_threshold,
            ai_staging: crate::AiStagingDebug {
                state: self.ai_decision_for_current_input().as_str().to_owned(),
                for_input: self
                    .staged_ai_result
                    .as_ref()
                    .map(|staged| staged.for_input.clone()),
            },
        }
    }

    fn backspace(&mut self) -> Option<String> {
        let previous = previous_char_boundary(
            &self.context.composition.input,
            self.context.composition.caret,
        )?;
        self.context.composition.input.remove(previous);
        self.context.composition.caret = previous;
        self.context.composition.preedit = self.context.composition.input.clone();
        self.refresh_candidates();
        None
    }

    fn delete_at_caret(&mut self) -> Option<String> {
        let caret = clamp_to_char_boundary(
            &self.context.composition.input,
            self.context.composition.caret,
        );
        if caret < self.context.composition.input.len() {
            self.context.composition.caret = caret;
            self.context.composition.input.remove(caret);
            self.context.composition.preedit = self.context.composition.input.clone();
            self.refresh_candidates();
        }
        None
    }

    pub(crate) fn commit_highlighted(&mut self) -> Option<String> {
        self.commit_candidate(self.context.highlighted, CommitIntent::DefaultConfirm)
    }

    fn commit_raw_input_text(&mut self) -> Option<String> {
        if self.context.composition.input.is_empty() {
            return None;
        }
        let text = self.context.composition.input.clone();
        self.record_commit_with_type("raw", text.clone(), text.clone());
        self.clear_composition();
        Some(text)
    }

    pub fn commit_script_text(&mut self) -> Option<String> {
        if self.context.composition.preedit.is_empty() {
            return None;
        }
        let text = self.context.composition.preedit.clone();
        self.record_commit_with_type("raw", text.clone(), text.clone());
        self.clear_composition();
        Some(text)
    }

    pub fn commit_comment(&mut self) -> Option<String> {
        let text = self
            .context
            .candidates
            .get(self.context.highlighted)
            .and_then(|candidate| {
                (!candidate.comment.is_empty()).then(|| candidate.comment.clone())
            })?;
        self.record_commit_with_type("raw", text.clone(), text.clone());
        self.clear_composition();
        Some(text)
    }

    pub fn back_to_previous_input(&mut self) -> Option<String> {
        self.backspace()
    }

    pub fn delete_input(&mut self) -> Option<String> {
        self.delete_at_caret()
    }

    fn commit_candidate_at_page_index(&mut self, page_index: usize) -> Option<String> {
        if page_index >= DEFAULT_PAGE_SIZE {
            return None;
        }
        let page_start = (self.context.highlighted / DEFAULT_PAGE_SIZE) * DEFAULT_PAGE_SIZE;
        if page_start + page_index >= self.context.candidates.len() && !self.candidate_list_complete
        {
            self.ensure_complete_candidates();
        }
        self.commit_candidate(page_start + page_index, CommitIntent::ExplicitSelection)
    }

    fn commit_candidate(&mut self, candidate_index: usize, intent: CommitIntent) -> Option<String> {
        let input = self.context.composition.input.clone();
        let segment_start = 0;
        let candidate = self.context.candidates.get(candidate_index).cloned()?;
        let candidate_source = candidate.source.clone();
        if intent == CommitIntent::DefaultConfirm && candidate_source.is_ai() {
            return None;
        }
        let partial_consumed = partial_consumed_len_for_commit(&input, &candidate, intent);
        let (text, learning_input, segment_end) = if let Some(consumed) = partial_consumed {
            (
                candidate.text.clone(),
                input[..consumed].to_owned(),
                consumed,
            )
        } else {
            (
                candidate.commit_text_for_input(&input),
                input.clone(),
                input.len(),
            )
        };
        let code = learning_code_for_candidate(&learning_input, &candidate);
        self.commit_tick = self.commit_tick.saturating_add(1);
        let learning = UserDbCommitMetadata::new(
            learning_input,
            text.clone(),
            candidate_source.clone(),
            segment_start,
            segment_end,
            self.commit_tick,
        )
        .with_code(code);
        if candidate_source.is_ai() {
            self.pending_userdb_learning = None;
            self.ai_memory.record_commit(&self.context, &learning);
        } else {
            self.pending_userdb_learning = Some(learning.clone());
        }
        self.record_commit_with_metadata(learning);
        if let Some(consumed) = partial_consumed {
            let remaining = input[consumed..].to_owned();
            self.staged_ai_result = None;
            self.context.composition.input = remaining.clone();
            self.context.composition.caret = remaining.len();
            self.context.composition.preedit = remaining;
            self.refresh_candidates();
        } else {
            self.clear_composition();
        }
        Some(text)
    }

    fn record_commit_with_type(
        &mut self,
        candidate_type: impl Into<String>,
        text: String,
        input: String,
    ) {
        self.commit_tick = self.commit_tick.saturating_add(1);
        let segment_end = input.len();
        let metadata = UserDbCommitMetadata {
            code: input.clone(),
            input,
            selected_text: text,
            candidate_type: candidate_type.into(),
            candidate_source: CandidateSource::Echo,
            segment_start: 0,
            segment_end,
            tick: self.commit_tick,
        };
        self.record_commit_with_metadata(metadata);
    }

    fn record_commit_with_metadata(&mut self, metadata: UserDbCommitMetadata) {
        self.context.last_commit = Some(metadata.selected_text.clone());
        self.context.commit_history.push(CommitRecord {
            candidate_type: metadata.candidate_type,
            text: metadata.selected_text,
            input: metadata.input,
            segment_start: metadata.segment_start,
            segment_end: metadata.segment_end,
            tick: metadata.tick,
        });
    }

    fn refresh_candidates(&mut self) {
        self.refresh_candidates_with_request(self.candidate_request_for_refresh());
    }

    fn ensure_complete_candidates(&mut self) {
        if self.candidate_list_complete {
            return;
        }
        let highlighted = self.context.highlighted;
        self.refresh_candidates_with_request(CandidateRequest::unbounded());
        if !self.context.candidates.is_empty() {
            self.context.highlighted = highlighted.min(self.context.candidates.len() - 1);
        }
    }

    fn candidate_request_for_refresh(&self) -> CandidateRequest {
        if !self.can_use_bounded_refresh() {
            return CandidateRequest::unbounded();
        }
        let filter_extended_cjk = self
            .filters
            .iter()
            .any(|filter| filter.name() == "charset_filter")
            && !self
                .options
                .get("extended_charset")
                .copied()
                .unwrap_or(false);
        let input = self.context.composition.input.as_str();
        let has_filter_surplus_risk = self.filters.iter().any(|filter| {
            matches!(
                filter.name(),
                "charset_filter" | "simplifier" | "uniquifier"
            )
        });
        let surplus = if self.status.schema_id == "luna_pinyin"
            && crate::translator::is_m44_track_a_short_key_prefix(input)
        {
            if has_filter_surplus_risk {
                M44_SHORT_KEY_REFRESH_SURPLUS
            } else {
                0
            }
        } else {
            BOUNDED_REFRESH_SURPLUS
        };
        CandidateRequest::bounded(DEFAULT_PAGE_SIZE + surplus)
            .with_filter_extended_cjk(filter_extended_cjk)
    }

    fn can_use_bounded_refresh(&self) -> bool {
        let schema_allows_bounded = if self.status.schema_id == "luna_pinyin" {
            !self.context.composition.input.is_empty()
        } else {
            self.status.schema_id.starts_with("jyut6ping3")
        };
        schema_allows_bounded
            && self.rankers.is_empty()
            && self.userdb.entries().is_empty()
            && self.filters.iter().all(|filter| {
                matches!(
                    filter.name(),
                    "charset_filter" | "dictionary_lookup_filter" | "simplifier" | "uniquifier"
                )
            })
    }

    fn refresh_candidates_with_request(&mut self, request: CandidateRequest) {
        let input = self.context.composition.input.clone();
        if request.limit.is_some() {
            crate::m37_record_candidate_request_bounded(
                DEFAULT_PAGE_SIZE,
                request
                    .limit
                    .unwrap_or(DEFAULT_PAGE_SIZE)
                    .saturating_sub(DEFAULT_PAGE_SIZE),
            );
        } else {
            crate::m37_record_candidate_request_unbounded();
        }
        let (mut candidates, candidate_list_complete) = if request.limit.is_some() {
            let mut candidates = Vec::new();
            let mut candidate_list_complete = true;
            for translator in &self.translators {
                let translator_start = Instant::now();
                let result = translator.translate_with_context_and_request(
                    &input,
                    &self.status,
                    &self.options,
                    &self.context,
                    request,
                );
                crate::m37_record_translator(translator_start.elapsed());
                candidate_list_complete &= result.is_complete;
                candidates.extend(result.candidates);
            }
            (candidates, candidate_list_complete)
        } else {
            crate::m37_record_full_list_translation();
            (
                self.translators
                    .iter()
                    .flat_map(|translator| {
                        let translator_start = Instant::now();
                        let candidates = translator.translate_with_context(
                            &input,
                            &self.status,
                            &self.options,
                            &self.context,
                        );
                        crate::m37_record_translator(translator_start.elapsed());
                        candidates
                    })
                    .collect::<Vec<_>>(),
                true,
            )
        };
        let sort_start = Instant::now();
        candidates.sort_by(|left, right| {
            right
                .quality
                .partial_cmp(&left.quality)
                .unwrap_or(Ordering::Equal)
        });
        crate::m37_record_candidates_sorted(candidates.len());
        crate::m37_record_candidate_sort(sort_start.elapsed());
        let userdb_start = Instant::now();
        merge_userdb_candidates(
            &input,
            &mut candidates,
            self.userdb
                .lookup(&UserDbLookupRequest::new(input.as_str()).with_predictive(true)),
            self.prediction_never_first,
        );
        crate::m37_record_userdb_merge(userdb_start.elapsed());
        let filter_start = Instant::now();
        let mut filter_audit = Vec::with_capacity(self.filters.len());
        for filter in &self.filters {
            let before_count = candidates.len();
            filter.apply_with_context(&mut candidates, &self.options, &self.context);
            filter_audit.push(FilterAuditRecord {
                name: filter.name().to_owned(),
                before_count,
                after_count: candidates.len(),
            });
        }
        crate::m37_record_filter_pipeline(filter_start.elapsed());
        self.last_filter_audit = filter_audit;
        let ranker_start = Instant::now();
        for ranker in &self.rankers {
            if let RerankResult::Ready(ranked) = ranker.try_rerank(&self.context, &candidates) {
                candidates = ranked;
            }
        }
        crate::m37_record_ranker_pipeline(ranker_start.elapsed());
        let ai_start = Instant::now();
        self.context.candidates =
            merge_classic_and_staged_ai(&input, candidates, self.staged_ai_result.as_ref());
        crate::m37_record_ai_merge(ai_start.elapsed());
        crate::m37_record_candidates_stored(self.context.candidates.len());
        self.context.highlighted = 0;
        self.candidate_list_complete = candidate_list_complete;
    }
}

fn merge_userdb_candidates(
    input: &str,
    candidates: &mut Vec<Candidate>,
    userdb_results: Vec<UserDbLookupResult>,
    prediction_never_first: bool,
) {
    let input_code_len = comparable_userdb_code_len(input);
    for result in userdb_results {
        let user_code_len = result.comparable_code_len();
        let user_candidate = result.candidate();
        let mut insertion_index = if user_code_len > input_code_len {
            let mut index = candidates
                .iter()
                .position(|candidate| candidate.source != CandidateSource::UserTable)
                .unwrap_or(candidates.len());
            if prediction_never_first && index == 0 && !candidates.is_empty() {
                index = 1;
            }
            index
        } else if result.is_multi_segment_code() && user_code_len == input_code_len {
            candidates
                .iter()
                .position(|candidate| {
                    candidate_comparable_code_len(candidate, input_code_len) < user_code_len
                })
                .unwrap_or(candidates.len())
        } else {
            equal_code_user_phrase_insert_index(user_candidate.quality, candidates.len())
        };
        while insertion_index < candidates.len()
            && candidates[insertion_index].source == CandidateSource::UserTable
        {
            insertion_index += 1;
        }
        candidates.insert(insertion_index, user_candidate);
    }
}

fn equal_code_user_phrase_insert_index(user_quality: f32, candidates_len: usize) -> usize {
    let probability = (user_quality - 0.5).max(0.0);
    let threshold = (TYPEDUCK_E_SQUARED - probability * TYPEDUCK_EXP_E_SQUARED)
        .max(0.0)
        .ceil() as usize;
    threshold.min(candidates_len)
}

fn candidate_comparable_code_len(candidate: &Candidate, input_code_len: usize) -> usize {
    if candidate.source == CandidateSource::Sentence {
        input_code_len
    } else {
        comparable_userdb_code_len(&candidate.comment)
    }
}

fn comparable_userdb_code_len(code: &str) -> usize {
    code.chars()
        .filter(|ch| !ch.is_ascii_digit() && !ch.is_whitespace())
        .count()
}

fn merge_classic_and_staged_ai(
    input: &str,
    mut classic: Vec<Candidate>,
    staged_ai_result: Option<&StagedAiCandidates>,
) -> Vec<Candidate> {
    if let Some(staged) = staged_ai_result {
        if staged.matches_input(input) {
            let mut ai_candidates = staged
                .candidates
                .iter()
                .cloned()
                .enumerate()
                .collect::<Vec<_>>();
            ai_candidates.sort_by(|(left_index, left), (right_index, right)| {
                right
                    .source
                    .ai_confidence()
                    .cmp(&left.source.ai_confidence())
                    .then_with(|| left_index.cmp(right_index))
            });
            let insertion_index = classic.len().min(1);
            classic.splice(
                insertion_index..insertion_index,
                ai_candidates.into_iter().map(|(_, candidate)| candidate),
            );
        }
    }
    classic
}

const fn is_exact_control_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.control
        && !modifiers.shift
        && !modifiers.lock
        && !modifiers.alt
        && !modifiers.super_key
        && !modifiers.hyper
        && !modifiers.meta
        && !modifiers.release
}

const fn is_exact_shift_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.shift
        && !modifiers.lock
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.super_key
        && !modifiers.hyper
        && !modifiers.meta
        && !modifiers.release
}

const fn is_exact_control_shift_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.control
        && modifiers.shift
        && !modifiers.lock
        && !modifiers.alt
        && !modifiers.super_key
        && !modifiers.hyper
        && !modifiers.meta
        && !modifiers.release
}

const fn is_printable_ascii(ch: char) -> bool {
    matches!(ch, '!'..='~')
}

const fn select_index_from_digit(ch: char) -> usize {
    match ch {
        '1'..='9' => ch as usize - '1' as usize,
        '0' => 9,
        _ => 0,
    }
}
