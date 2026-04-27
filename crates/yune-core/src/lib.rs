use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub text: String,
    pub comment: String,
    pub source: CandidateSource,
    pub quality: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateSource {
    Echo,
    Punctuation,
    Table,
    Ai,
}

impl CandidateSource {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Echo => "echo",
            Self::Punctuation => "punct",
            Self::Table => "table",
            Self::Ai => "ai",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyModifiers {
    pub shift: bool,
    pub lock: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
    pub hyper: bool,
    pub meta: bool,
    pub release: bool,
}

impl KeyModifiers {
    #[must_use]
    pub const fn is_empty(self) -> bool {
        !self.shift
            && !self.lock
            && !self.control
            && !self.alt
            && !self.super_key
            && !self.hyper
            && !self.meta
            && !self.release
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyCode {
    Character(char),
    KeypadDigit(char),
    Backspace,
    Delete,
    Escape,
    PreviousCandidate,
    NextCandidate,
    FirstCandidate,
    PreviousPage,
    NextPage,
    Return,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyEvent {
    #[must_use]
    pub const fn character(ch: char) -> Self {
        Self {
            code: KeyCode::Character(ch),
            modifiers: KeyModifiers {
                shift: false,
                lock: false,
                control: false,
                alt: false,
                super_key: false,
                hyper: false,
                meta: false,
                release: false,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeySequenceParseError {
    message: String,
}

impl KeySequenceParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for KeySequenceParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for KeySequenceParseError {}

pub fn parse_key_sequence(input: &str) -> Result<Vec<KeyEvent>, KeySequenceParseError> {
    let mut events = Vec::new();
    let mut index = 0;

    while index < input.len() {
        let ch = input[index..]
            .chars()
            .next()
            .expect("index should be at a character boundary");
        if ch == '{' && index + ch.len_utf8() < input.len() {
            let start = index + ch.len_utf8();
            let end = input[start..].find('}').map(|offset| start + offset);
            let end = end.ok_or_else(|| {
                KeySequenceParseError::new(format!(
                    "unmatched '{{' in key sequence at byte offset {index}"
                ))
            })?;
            let repr = &input[start..end];
            events.push(parse_key_event_repr(repr)?);
            index = end + '}'.len_utf8();
        } else {
            events.push(KeyEvent::character(ch));
            index += ch.len_utf8();
        }
    }

    Ok(events)
}

fn parse_key_event_repr(repr: &str) -> Result<KeyEvent, KeySequenceParseError> {
    if repr.is_empty() {
        return Err(KeySequenceParseError::new("empty key name in key sequence"));
    }
    if repr.chars().count() == 1 {
        return Ok(KeyEvent::character(repr.chars().next().expect(
            "single-character key representation should contain a char",
        )));
    }

    let mut tokens = repr.split('+').peekable();
    let mut modifiers = KeyModifiers::default();
    while let Some(token) = tokens.next() {
        if tokens.peek().is_none() {
            let code = key_code_from_name(token)?;
            return Ok(KeyEvent { code, modifiers });
        }
        apply_modifier(&mut modifiers, token)?;
    }

    Err(KeySequenceParseError::new("empty key representation"))
}

fn apply_modifier(modifiers: &mut KeyModifiers, token: &str) -> Result<(), KeySequenceParseError> {
    match token {
        "Shift" => modifiers.shift = true,
        "Lock" => modifiers.lock = true,
        "Control" => modifiers.control = true,
        "Alt" => modifiers.alt = true,
        "Super" => modifiers.super_key = true,
        "Hyper" => modifiers.hyper = true,
        "Meta" => modifiers.meta = true,
        "Release" => modifiers.release = true,
        _ => {
            return Err(KeySequenceParseError::new(format!(
                "unrecognized key modifier: {token}"
            )));
        }
    }
    Ok(())
}

fn key_code_from_name(name: &str) -> Result<KeyCode, KeySequenceParseError> {
    let code = match name {
        "space" => KeyCode::Character(' '),
        "BackSpace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Escape" => KeyCode::Escape,
        "Up" | "KP_Up" => KeyCode::PreviousCandidate,
        "Down" | "KP_Down" => KeyCode::NextCandidate,
        "Home" | "End" | "KP_Home" | "KP_End" => KeyCode::FirstCandidate,
        "Page_Up" | "Prior" | "KP_Page_Up" | "KP_Prior" => KeyCode::PreviousPage,
        "Page_Down" | "Next" | "KP_Page_Down" | "KP_Next" => KeyCode::NextPage,
        "Return" => KeyCode::Return,
        "KP_Enter" => KeyCode::Return,
        "KP_0" => KeyCode::KeypadDigit('0'),
        "KP_1" => KeyCode::KeypadDigit('1'),
        "KP_2" => KeyCode::KeypadDigit('2'),
        "KP_3" => KeyCode::KeypadDigit('3'),
        "KP_4" => KeyCode::KeypadDigit('4'),
        "KP_5" => KeyCode::KeypadDigit('5'),
        "KP_6" => KeyCode::KeypadDigit('6'),
        "KP_7" => KeyCode::KeypadDigit('7'),
        "KP_8" => KeyCode::KeypadDigit('8'),
        "KP_9" => KeyCode::KeypadDigit('9'),
        "braceleft" => KeyCode::Character('{'),
        "braceright" => KeyCode::Character('}'),
        "plus" => KeyCode::Character('+'),
        "comma" => KeyCode::Character(','),
        "period" => KeyCode::Character('.'),
        "minus" => KeyCode::Character('-'),
        "underscore" => KeyCode::Character('_'),
        _ => {
            return Err(KeySequenceParseError::new(format!(
                "unrecognized key name: {name}"
            )));
        }
    };
    Ok(code)
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Composition {
    pub input: String,
    pub caret: usize,
    pub preedit: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    pub composition: Composition,
    pub candidates: Vec<Candidate>,
    pub highlighted: usize,
    pub last_commit: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Status {
    pub schema_id: String,
    pub schema_name: String,
    pub is_disabled: bool,
    pub is_composing: bool,
    pub is_ascii_mode: bool,
    pub is_full_shape: bool,
    pub is_simplified: bool,
    pub is_traditional: bool,
    pub is_ascii_punct: bool,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            schema_id: "default".to_owned(),
            schema_name: "Default".to_owned(),
            is_disabled: false,
            is_composing: false,
            is_ascii_mode: false,
            is_full_shape: false,
            is_simplified: false,
            is_traditional: false,
            is_ascii_punct: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub context: Context,
    pub status: Status,
}

pub trait Translator: Send + Sync {
    fn name(&self) -> &'static str;

    fn translate(&self, input: &str) -> Vec<Candidate>;
}

pub trait CandidateRanker: Send + Sync {
    fn name(&self) -> &'static str;

    fn try_rerank(&self, context: &Context, candidates: &[Candidate]) -> RerankResult;
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
            source: CandidateSource::Echo,
            quality: 0.0,
        }]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableEntry {
    pub code: String,
    pub text: String,
    pub weight: f32,
}

impl TableEntry {
    #[must_use]
    pub fn new(code: impl Into<String>, text: impl Into<String>, weight: f32) -> Self {
        Self {
            code: normalize_table_code(&code.into()),
            text: text.into(),
            weight,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableDictionary {
    entries: Vec<TableEntry>,
}

impl TableDictionary {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = TableEntry>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
        }
    }

    pub fn parse_rime_dict_yaml(input: &str) -> Result<Self, TableDictionaryParseError> {
        let mut metadata = RimeTableMetadata::default();
        let mut in_header = false;
        let mut body_start = None;

        for (line_index, line) in input.lines().enumerate() {
            let trimmed = line.trim();
            if !in_header {
                if trimmed == "---" {
                    in_header = true;
                }
                continue;
            }

            if trimmed == "..." {
                body_start = Some(line_index + 1);
                break;
            }
            metadata.read_header_line(line);
        }

        let body_start = body_start.ok_or_else(|| {
            TableDictionaryParseError::new("RIME dictionary header is missing terminating '...'")
        })?;
        let mut entries = Vec::new();
        let mut comments_enabled = true;

        for line in input.lines().skip(body_start) {
            let line = line.trim_end();
            if line.is_empty() {
                continue;
            }
            if comments_enabled && line.starts_with('#') {
                if line == "# no comment" {
                    comments_enabled = false;
                }
                continue;
            }

            if let Some(entry) = metadata.parse_entry(line) {
                entries.push(entry);
            }
        }

        if metadata.sort_by_weight {
            entries.sort_by(|left, right| {
                left.code.cmp(&right.code).then_with(|| {
                    right
                        .weight
                        .partial_cmp(&left.weight)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            });
        }

        Ok(Self { entries })
    }

    #[must_use]
    pub fn entries(&self) -> &[TableEntry] {
        &self.entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableDictionaryParseError {
    message: String,
}

impl TableDictionaryParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TableDictionaryParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TableDictionaryParseError {}

#[derive(Clone, Debug)]
struct RimeTableMetadata {
    columns: Vec<String>,
    reading_columns: bool,
    sort_by_weight: bool,
}

impl Default for RimeTableMetadata {
    fn default() -> Self {
        Self {
            columns: vec!["text".to_owned(), "code".to_owned(), "weight".to_owned()],
            reading_columns: false,
            sort_by_weight: true,
        }
    }
}

impl RimeTableMetadata {
    fn read_header_line(&mut self, line: &str) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return;
        }

        if self.reading_columns {
            if let Some(column) = trimmed.strip_prefix("- ") {
                self.columns.push(column.trim().to_owned());
                return;
            }
            self.reading_columns = false;
        }

        if trimmed == "columns:" {
            self.columns.clear();
            self.reading_columns = true;
            return;
        }

        if let Some(sort_order) = trimmed.strip_prefix("sort:") {
            self.sort_by_weight = sort_order.trim() != "original";
        }
    }

    fn parse_entry(&self, line: &str) -> Option<TableEntry> {
        let fields = line.split('\t').collect::<Vec<_>>();
        let text_column = self.column_index("text")?;
        let code_column = self.column_index("code")?;
        let text = fields.get(text_column)?.trim();
        let code = fields.get(code_column)?.trim();
        if text.is_empty() || code.is_empty() {
            return None;
        }

        let weight = self
            .column_index("weight")
            .and_then(|column| fields.get(column))
            .and_then(|value| value.trim().parse::<f32>().ok())
            .unwrap_or(0.0);
        Some(TableEntry::new(code, text, weight))
    }

    fn column_index(&self, label: &str) -> Option<usize> {
        self.columns.iter().position(|column| column == label)
    }
}

fn normalize_table_code(code: &str) -> String {
    code.split_whitespace().collect()
}

pub struct StaticTableTranslator {
    entries: Vec<(String, Candidate)>,
}

impl StaticTableTranslator {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        let entries = entries
            .into_iter()
            .map(|(code, text)| {
                let code = code.into();
                let text = text.into();
                (
                    code.clone(),
                    Candidate {
                        text,
                        comment: code,
                        source: CandidateSource::Table,
                        quality: 1.0,
                    },
                )
            })
            .collect();
        Self { entries }
    }

    #[must_use]
    pub fn from_dictionary(dictionary: TableDictionary) -> Self {
        let entries = dictionary
            .entries
            .into_iter()
            .map(|entry| {
                let candidate = Candidate {
                    text: entry.text,
                    comment: entry.code.clone(),
                    source: CandidateSource::Table,
                    quality: entry.weight,
                };
                (entry.code, candidate)
            })
            .collect();
        Self { entries }
    }

    pub fn parse_rime_dict_yaml(input: &str) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml(input).map(Self::from_dictionary)
    }
}

impl Translator for StaticTableTranslator {
    fn name(&self) -> &'static str {
        "static_table_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        self.entries
            .iter()
            .filter(|(code, _)| code == input)
            .map(|(_, candidate)| candidate.clone())
            .collect()
    }
}

pub struct PunctuationTranslator {
    entries: Vec<(String, Candidate)>,
}

impl PunctuationTranslator {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        let entries = entries
            .into_iter()
            .map(|(key, text)| {
                let key = key.into();
                let text = text.into();
                (
                    key.clone(),
                    Candidate {
                        text,
                        comment: "punct".to_owned(),
                        source: CandidateSource::Punctuation,
                        quality: 1.0,
                    },
                )
            })
            .collect();
        Self { entries }
    }

    #[must_use]
    pub fn default_half_shape() -> Self {
        Self::new([
            (",", "，"),
            (".", "。"),
            ("?", "？"),
            ("!", "！"),
            (";", "；"),
            (":", "："),
        ])
    }
}

impl Translator for PunctuationTranslator {
    fn name(&self) -> &'static str {
        "punct_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        self.entries
            .iter()
            .filter(|(key, _)| key == input)
            .map(|(_, candidate)| candidate.clone())
            .collect()
    }
}

pub struct Engine {
    context: Context,
    status: Status,
    options: HashMap<String, bool>,
    properties: HashMap<String, String>,
    translators: Vec<Box<dyn Translator>>,
    rankers: Vec<Box<dyn CandidateRanker>>,
}

const DEFAULT_PAGE_SIZE: usize = 5;

impl Default for Engine {
    fn default() -> Self {
        Self {
            context: Context::default(),
            status: Status::default(),
            options: HashMap::new(),
            properties: HashMap::new(),
            translators: vec![Box::new(EchoTranslator)],
            rankers: Vec::new(),
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

    pub fn add_ranker(&mut self, ranker: impl CandidateRanker + 'static) {
        self.rankers.push(Box::new(ranker));
        self.refresh_candidates();
    }

    pub fn set_schema(&mut self, id: impl Into<String>, name: impl Into<String>) {
        self.status.schema_id = id.into();
        self.status.schema_name = name.into();
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

    #[must_use]
    pub fn get_property(&self, property: &str) -> Option<&str> {
        self.properties.get(property).map(String::as_str)
    }

    pub fn process_char(&mut self, ch: char) -> Option<String> {
        match ch {
            '\u{8}' | '\u{7f}' => self.backspace(),
            ' ' => self.commit_highlighted(),
            '0'..='9' if !self.context.candidates.is_empty() => {
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
        if !key_event.modifiers.is_empty() {
            return None;
        }

        match key_event.code {
            KeyCode::Character(ch) => self.process_char(ch),
            KeyCode::KeypadDigit(ch) if !self.context.candidates.is_empty() => {
                self.commit_candidate_at_page_index(select_index_from_digit(ch))
            }
            KeyCode::KeypadDigit(_) => None,
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete_at_caret(),
            KeyCode::Escape => {
                self.clear_composition();
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
            KeyCode::Return => self.commit_highlighted(),
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

    pub fn select_candidate(&mut self, index: usize) -> Option<String> {
        self.commit_candidate(index)
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
        if self.context.candidates.is_empty() {
            return false;
        }

        let page_size = page_size.max(1);
        let current_index = self.context.highlighted;
        let next_index = if backward {
            current_index.saturating_sub(page_size)
        } else {
            current_index + page_size
        };
        self.highlight_candidate(next_index)
    }

    pub fn previous_candidate(&mut self) -> bool {
        if self.context.candidates.is_empty() {
            return false;
        }
        if self.context.highlighted == 0 {
            return true;
        }
        self.highlight_candidate(self.context.highlighted - 1)
    }

    pub fn next_candidate(&mut self) -> bool {
        if self.context.candidates.is_empty() {
            return false;
        }
        let next_index = self.context.highlighted + 1;
        if next_index >= self.context.candidates.len() {
            return true;
        }
        self.highlight_candidate(next_index)
    }

    pub fn first_candidate(&mut self) -> bool {
        if self.context.candidates.is_empty() {
            return false;
        }
        if self.context.highlighted == 0 {
            return false;
        }
        self.highlight_candidate(0)
    }

    pub fn clear_composition(&mut self) {
        self.context.composition = Composition::default();
        self.context.candidates.clear();
        self.context.highlighted = 0;
    }

    pub fn set_input(&mut self, input: impl Into<String>) {
        let input = input.into();
        self.context.composition.input = input.clone();
        self.context.composition.caret = input.len();
        self.context.composition.preedit = input;
        self.refresh_candidates();
    }

    pub fn set_caret_pos(&mut self, caret_pos: usize) {
        self.context.composition.caret = caret_pos.min(self.context.composition.input.len());
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
        self.context.composition.input.pop();
        self.context.composition.caret = self.context.composition.input.len();
        self.context.composition.preedit = self.context.composition.input.clone();
        self.refresh_candidates();
        None
    }

    fn delete_at_caret(&mut self) -> Option<String> {
        if self.context.composition.caret < self.context.composition.input.len() {
            self.context
                .composition
                .input
                .remove(self.context.composition.caret);
            self.context.composition.preedit = self.context.composition.input.clone();
            self.refresh_candidates();
        }
        None
    }

    fn commit_highlighted(&mut self) -> Option<String> {
        self.commit_candidate(self.context.highlighted)
    }

    fn commit_candidate_at_page_index(&mut self, page_index: usize) -> Option<String> {
        if page_index >= DEFAULT_PAGE_SIZE {
            return None;
        }
        let page_start = (self.context.highlighted / DEFAULT_PAGE_SIZE) * DEFAULT_PAGE_SIZE;
        self.commit_candidate(page_start + page_index)
    }

    fn commit_candidate(&mut self, candidate_index: usize) -> Option<String> {
        let text = self
            .context
            .candidates
            .get(candidate_index)
            .map(|candidate| candidate.text.clone())?;
        self.context.last_commit = Some(text.clone());
        self.clear_composition();
        Some(text)
    }

    fn refresh_candidates(&mut self) {
        let input = self.context.composition.input.as_str();
        let mut candidates = self
            .translators
            .iter()
            .flat_map(|translator| translator.translate(input))
            .collect::<Vec<_>>();
        for ranker in &self.rankers {
            if let RerankResult::Ready(ranked) = ranker.try_rerank(&self.context, &candidates) {
                candidates = ranked;
            }
        }
        self.context.candidates = candidates;
        self.context.highlighted = 0;
    }
}

const fn select_index_from_digit(ch: char) -> usize {
    match ch {
        '1'..='9' => ch as usize - '1' as usize,
        '0' => 9,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_key_sequence, CandidateRanker, CandidateSource, Context, Engine, KeyCode,
        MockAiRanker, PunctuationTranslator, RerankResult, StaticTableTranslator, TableDictionary,
    };

    #[test]
    fn parses_librime_style_key_sequence_names() {
        let keys = parse_key_sequence(
            "zyx 123{Shift+space}ABC{Control+Alt+Return}{KP_Enter}{KP_2}{Delete}{Escape}{Home}{KP_End}{Page_Down}{KP_Page_Up}{Down}{KP_Up}",
        )
        .expect("key sequence should parse");

        assert_eq!(keys.len(), 22);
        assert_eq!(keys[3].code, KeyCode::Character(' '));
        assert!(!keys[3].modifiers.shift);
        assert_eq!(keys[7].code, KeyCode::Character(' '));
        assert!(keys[7].modifiers.shift);
        assert_eq!(keys[11].code, KeyCode::Return);
        assert!(keys[11].modifiers.control);
        assert!(keys[11].modifiers.alt);
        assert_eq!(keys[12].code, KeyCode::Return);
        assert_eq!(keys[13].code, KeyCode::KeypadDigit('2'));
        assert_eq!(keys[14].code, KeyCode::Delete);
        assert_eq!(keys[15].code, KeyCode::Escape);
        assert_eq!(keys[16].code, KeyCode::FirstCandidate);
        assert_eq!(keys[17].code, KeyCode::FirstCandidate);
        assert_eq!(keys[18].code, KeyCode::NextPage);
        assert_eq!(keys[19].code, KeyCode::PreviousPage);
        assert_eq!(keys[20].code, KeyCode::NextCandidate);
        assert_eq!(keys[21].code, KeyCode::PreviousCandidate);
    }

    #[test]
    fn parses_named_braces_for_literal_brace_keys() {
        let keys =
            parse_key_sequence("{braceleft}{braceright}").expect("key sequence should parse");

        assert_eq!(keys[0].code, KeyCode::Character('{'));
        assert_eq!(keys[1].code, KeyCode::Character('}'));
    }

    #[test]
    fn commits_table_candidate_before_echo_candidate() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.process_char('n');
        engine.process_char('i');

        assert_eq!(engine.context().composition.preedit, "ni");
        assert_eq!(engine.context().candidates[0].text, "你");
        assert_eq!(engine.context().candidates[1].text, "ni");

        let commit = engine.process_char(' ');
        assert_eq!(commit.as_deref(), Some("你"));
    }

    #[test]
    fn numeric_selection_commits_candidate_on_current_page() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("ba2")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn keypad_numeric_selection_matches_librime_selector_without_text_input() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{KP_1}ba{KP_2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn escape_clears_composition_like_librime_editor_cancel() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("ni{Escape}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert!(engine.context().composition.input.is_empty());
        assert!(engine.context().candidates.is_empty());
        assert_eq!(engine.context().last_commit, None);
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn delete_key_removes_input_at_caret_like_librime_editor_delete() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);

        engine.set_input("ni");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Delete}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.input, "ni");
        assert_eq!(engine.context().composition.caret, 2);
    }

    #[test]
    fn page_keys_move_candidate_page_like_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));

        let commits = engine
            .process_key_sequence("{Page_Down}ba{Page_Down}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().highlighted, 5);
        assert_eq!(engine.context().candidates[5].text, "拔");

        engine
            .process_key_sequence("{KP_Page_Up}")
            .expect("key sequence should parse");

        assert_eq!(engine.context().highlighted, 0);
        assert_eq!(engine.context().last_commit, None);
    }

    #[test]
    fn up_down_keys_move_candidate_highlight_like_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));

        let commits = engine
            .process_key_sequence("{Down}ba{Down}{KP_Down}{KP_Up}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn home_end_keys_reset_candidate_highlight_like_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));

        let commits = engine
            .process_key_sequence("ba{Down}{Down}{Home}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["八"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("八"));

        let commits = engine
            .process_key_sequence("ba{Down}{KP_End}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["八"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("八"));
    }

    #[test]
    fn direct_candidate_selection_commits_by_global_or_page_index() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");
        assert_eq!(engine.select_candidate(1).as_deref(), Some("吧"));
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");
        assert_eq!(
            engine.select_candidate_on_current_page(0).as_deref(),
            Some("八")
        );
        assert_eq!(engine.context().last_commit.as_deref(), Some("八"));
    }

    #[test]
    fn direct_candidate_highlighting_moves_selection_without_committing() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");
        assert!(engine.highlight_candidate(1));
        assert_eq!(engine.context().highlighted, 1);
        assert_eq!(engine.context().last_commit, None);
        assert!(!engine.highlight_candidate(99));
        assert_eq!(engine.context().highlighted, 1);

        assert!(engine.change_page(false));
        assert_eq!(engine.context().highlighted, 6);
        assert!(engine.highlight_candidate_on_current_page(0));
        assert_eq!(engine.context().highlighted, 5);
        assert!(!engine.highlight_candidate_on_current_page(5));
        assert_eq!(engine.context().highlighted, 5);
        assert!(engine.change_page(true));
        assert_eq!(engine.context().highlighted, 0);
        assert!(engine.change_page(true));
        assert_eq!(engine.context().highlighted, 0);

        assert_eq!(engine.commit_composition().as_deref(), Some("八"));
    }

    #[test]
    fn direct_candidate_deletion_removes_menu_items_without_committing() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");
        assert!(engine.delete_candidate(1));
        assert_eq!(engine.context().candidates[1].text, "爸");
        assert_eq!(engine.context().last_commit, None);
        assert!(!engine.delete_candidate(99));

        assert!(engine.change_page(false));
        assert!(engine.delete_candidate_on_current_page(0));
        assert_eq!(
            engine
                .context()
                .candidates
                .last()
                .map(|candidate| candidate.text.as_str()),
            Some("拔")
        );
        assert!(!engine.delete_candidate_on_current_page(5));
    }

    #[test]
    fn numeric_selection_consumes_out_of_page_digit_without_extending_input() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("ba0")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.input, "ba");
        assert_eq!(engine.context().candidates.len(), 3);
    }

    #[test]
    fn parses_rime_dict_yaml_default_columns_and_weight_order() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

巴	ba	3193
爸	ba	3625
八	ba	6677
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "爸");
        assert_eq!(entries[2].text, "巴");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 6677.0);
    }

    #[test]
    fn parses_rime_dict_yaml_custom_columns_for_shape_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: cangjie_sample
version: "0.1"
sort: original
columns:
  - text
  - code
  - stem
...

明	ab
晭	abgr	ab'gr
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries[0].text, "明");
        assert_eq!(entries[0].code, "ab");
        assert_eq!(entries[1].text, "晭");
        assert_eq!(entries[1].code, "abgr");
    }

    #[test]
    fn table_translator_can_commit_rime_dictionary_phrase_codes() {
        let mut engine = Engine::new();
        let translator = StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

你	ni	1
你好	ni hao	10
"#,
        )
        .expect("dictionary should parse");
        engine.add_translator(translator);

        let commits = engine
            .process_key_sequence("nihao{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["你好"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你好"));
    }

    #[test]
    fn explicit_composition_control_commits_or_clears_active_input() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine
            .process_key_sequence("ni")
            .expect("key sequence should parse");
        assert_eq!(engine.commit_composition().as_deref(), Some("你"));
        assert!(!engine.status().is_composing);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));

        engine
            .process_key_sequence("hao")
            .expect("key sequence should parse");
        engine.clear_composition();
        assert!(!engine.status().is_composing);
        assert!(engine.context().candidates.is_empty());
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert_eq!(engine.commit_composition(), None);
    }

    #[test]
    fn direct_input_control_rebuilds_candidates_and_clamps_caret() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("ni");

        assert_eq!(engine.context().composition.input, "ni");
        assert_eq!(engine.context().composition.preedit, "ni");
        assert_eq!(engine.context().composition.caret, 2);
        assert_eq!(engine.context().candidates[0].text, "你");

        engine.set_caret_pos(1);
        assert_eq!(engine.context().composition.caret, 1);
        engine.set_caret_pos(10);
        assert_eq!(engine.context().composition.caret, 2);
    }

    #[test]
    fn runtime_options_update_status_flags_and_preserve_custom_values() {
        let mut engine = Engine::new();

        assert!(!engine.get_option("ascii_mode"));
        engine.set_option("ascii_mode", true);
        engine.set_option("custom_toggle", true);

        let status = engine.status();
        assert!(status.is_ascii_mode);
        assert!(engine.get_option("ascii_mode"));
        assert!(engine.get_option("custom_toggle"));

        engine.set_option("ascii_mode", false);
        assert!(!engine.status().is_ascii_mode);
        assert!(!engine.get_option("ascii_mode"));
        assert!(!engine.get_option("unknown_toggle"));
    }

    #[test]
    fn runtime_properties_store_session_strings() {
        let mut engine = Engine::new();

        assert_eq!(engine.get_property("client_app"), None);

        engine.set_property("client_app", "sample_console");
        engine.set_property("inline_preedit", "");

        assert_eq!(engine.get_property("client_app"), Some("sample_console"));
        assert_eq!(engine.get_property("inline_preedit"), Some(""));
    }

    #[test]
    fn mock_ai_ranker_can_reorder_ready_candidates() {
        let mut engine = Engine::new();
        let translator = StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

把	ba	100
吧	ba	50
八	ba	10
"#,
        )
        .expect("dictionary should parse");
        engine.add_translator(translator);
        engine.add_ranker(MockAiRanker::new(["吧"]));

        engine
            .process_key_sequence("ba")
            .expect("keys should parse");

        assert_eq!(engine.context().candidates[0].text, "吧");
        assert_eq!(engine.context().candidates[1].text, "把");
        assert_eq!(engine.context().candidates[2].text, "八");
    }

    #[test]
    fn pending_ranker_keeps_classic_candidate_order() {
        struct PendingRanker;

        impl CandidateRanker for PendingRanker {
            fn name(&self) -> &'static str {
                "pending_ranker"
            }

            fn try_rerank(
                &self,
                _context: &Context,
                _candidates: &[super::Candidate],
            ) -> RerankResult {
                RerankResult::Pending
            }
        }

        let mut engine = Engine::new();
        let translator = StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

把	ba	100
吧	ba	50
"#,
        )
        .expect("dictionary should parse");
        engine.add_translator(translator);
        engine.add_ranker(PendingRanker);

        engine
            .process_key_sequence("ba")
            .expect("keys should parse");

        assert_eq!(engine.context().candidates[0].text, "把");
        assert_eq!(engine.context().candidates[1].text, "吧");
    }

    #[test]
    fn punctuation_translator_offers_half_shape_candidates_before_echo() {
        let mut engine = Engine::new();
        engine.add_translator(PunctuationTranslator::default_half_shape());

        engine.process_char('.');

        assert_eq!(engine.context().composition.input, ".");
        assert_eq!(engine.context().candidates[0].text, "。");
        assert_eq!(
            engine.context().candidates[0].source,
            CandidateSource::Punctuation
        );
        assert_eq!(engine.context().candidates[1].text, ".");
    }

    #[test]
    fn punctuation_candidate_commits_through_selection_key() {
        let mut engine = Engine::new();
        engine.add_translator(PunctuationTranslator::default_half_shape());

        let commits = engine
            .process_key_sequence(".{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["。"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("。"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn backspace_rebuilds_candidates() {
        let mut engine = Engine::new();

        engine.process_char('a');
        engine.process_char('b');
        engine.process_char('\u{8}');

        assert_eq!(engine.context().composition.input, "a");
        assert_eq!(engine.context().candidates[0].source, CandidateSource::Echo);
    }

    #[test]
    fn sequence_collects_commits_and_snapshot_status() {
        let mut engine = Engine::new();
        engine.set_schema("sample", "Sample");
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine.process_sequence("ni ");
        let snapshot = engine.snapshot();

        assert_eq!(commits, ["你"]);
        assert_eq!(snapshot.context.last_commit.as_deref(), Some("你"));
        assert_eq!(snapshot.status.schema_id, "sample");
        assert!(!snapshot.status.is_composing);
    }

    #[test]
    fn key_sequence_processes_named_backspace_and_space() {
        let mut engine = Engine::new();

        let commits = engine
            .process_key_sequence("ni{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["n"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("n"));
        assert!(!engine.status().is_composing);
    }
}
