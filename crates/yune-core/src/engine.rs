use std::cmp::Ordering;
use std::collections::HashMap;

use crate::punctuation::punctuation_candidate_comment;
use crate::{
    parse_key_sequence, AiDecision, AiResult, Candidate, CandidateFilter, CandidateRanker,
    CandidateSource, CommitRecord, Composition, Context, EchoTranslator, KeyCode, KeyEvent,
    KeyModifiers, KeySequenceParseError, RerankResult, Snapshot, StagedAiCandidates, Status,
    Translator, UserDb, UserDbCommitMetadata, UserDbLookupRequest,
};

pub struct Engine {
    context: Context,
    status: Status,
    options: HashMap<String, bool>,
    properties: HashMap<String, String>,
    translators: Vec<Box<dyn Translator>>,
    filters: Vec<Box<dyn CandidateFilter>>,
    rankers: Vec<Box<dyn CandidateRanker>>,
    staged_ai_result: Option<StagedAiCandidates>,
    userdb: UserDb,
    pending_userdb_learning: Option<UserDbCommitMetadata>,
    commit_tick: u64,
}

const DEFAULT_PAGE_SIZE: usize = 5;

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

impl Default for Engine {
    fn default() -> Self {
        Self {
            context: Context::default(),
            status: Status::default(),
            options: HashMap::new(),
            properties: HashMap::new(),
            translators: vec![Box::new(EchoTranslator)],
            filters: Vec::new(),
            rankers: Vec::new(),
            staged_ai_result: None,
            userdb: UserDb::default(),
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
        let insert_at = self
            .translators
            .iter()
            .position(|existing| existing.name() == "echo_translator")
            .unwrap_or(self.translators.len());
        self.translators.insert(insert_at, Box::new(translator));
        self.refresh_candidates();
    }

    pub fn reset_translators(&mut self) {
        self.translators = vec![Box::new(EchoTranslator)];
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

    pub fn take_pending_userdb_learning(&mut self) -> Option<UserDbCommitMetadata> {
        self.pending_userdb_learning.take()
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
        self.commit_candidate(index, CommitIntent::ExplicitSelection)
    }

    pub fn select_candidate_on_current_page(&mut self, index: usize) -> Option<String> {
        self.commit_candidate_at_page_index(index)
    }

    pub fn highlight_candidate(&mut self, index: usize) -> bool {
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
            source: CandidateSource::Punctuation,
            quality: 1.0,
        }];
        self.context.highlighted = 0;
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
        Snapshot {
            context: self.context.clone(),
            status: self.status(),
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
        self.commit_candidate(page_start + page_index, CommitIntent::ExplicitSelection)
    }

    fn commit_candidate(&mut self, candidate_index: usize, intent: CommitIntent) -> Option<String> {
        let input = self.context.composition.input.clone();
        let segment_start = 0;
        let segment_end = input.len();
        let (text, candidate_source) = self
            .context
            .candidates
            .get(candidate_index)
            .map(|candidate| (candidate.text.clone(), candidate.source.clone()))?;
        if intent == CommitIntent::DefaultConfirm && candidate_source.is_ai() {
            return None;
        }
        self.commit_tick = self.commit_tick.saturating_add(1);
        let learning = UserDbCommitMetadata::new(
            input.clone(),
            text.clone(),
            candidate_source.clone(),
            segment_start,
            segment_end,
            self.commit_tick,
        );
        if candidate_source.is_ai() {
            self.pending_userdb_learning = None;
        } else {
            self.pending_userdb_learning = Some(learning.clone());
        }
        self.record_commit_with_metadata(learning);
        self.clear_composition();
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
        let input = self.context.composition.input.clone();
        let mut candidates = self
            .translators
            .iter()
            .flat_map(|translator| {
                translator.translate_with_context(
                    &input,
                    &self.status,
                    &self.options,
                    &self.context,
                )
            })
            .collect::<Vec<_>>();
        candidates.extend(
            self.userdb
                .lookup(&UserDbLookupRequest::new(input.as_str()).with_predictive(true))
                .into_iter()
                .map(|result| result.candidate()),
        );
        candidates.sort_by(|left, right| {
            right
                .quality
                .partial_cmp(&left.quality)
                .unwrap_or(Ordering::Equal)
        });
        for filter in &self.filters {
            filter.apply_with_context(&mut candidates, &self.options, &self.context);
        }
        for ranker in &self.rankers {
            if let RerankResult::Ready(ranked) = ranker.try_rerank(&self.context, &candidates) {
                candidates = ranked;
            }
        }
        self.context.candidates =
            merge_classic_and_staged_ai(&input, candidates, self.staged_ai_result.as_ref());
        self.context.highlighted = 0;
    }
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
            classic.extend(ai_candidates.into_iter().map(|(_, candidate)| candidate));
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
