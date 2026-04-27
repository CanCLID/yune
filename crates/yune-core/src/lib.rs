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
    Tab,
    Ignored,
    Backspace,
    Delete,
    Escape,
    MoveCaretLeft,
    MoveCaretRight,
    MoveCaretLeftByChar,
    MoveCaretRightByChar,
    MoveCaretLeftBySyllable,
    MoveCaretRightBySyllable,
    Home,
    End,
    PreviousCandidate,
    NextCandidate,
    FirstCandidate,
    PreviousPage,
    NextPage,
    Return,
    KeypadEnter,
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
            let code = if is_exact_control_modifier(modifiers) || is_exact_shift_modifier(modifiers)
            {
                match token {
                    "Up" => KeyCode::MoveCaretLeftBySyllable,
                    "Down" => KeyCode::MoveCaretRightBySyllable,
                    _ => key_code_from_name(token)?,
                }
            } else {
                key_code_from_name(token)?
            };
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
    if name.chars().count() == 1 {
        return Ok(KeyCode::Character(
            name.chars()
                .next()
                .expect("single-character key name should contain a char"),
        ));
    }

    let code = match name {
        "space" => KeyCode::Character(' '),
        "Tab" => KeyCode::Tab,
        "Linefeed" | "Clear" | "Pause" | "Scroll_Lock" | "Sys_Req" | "Begin" | "Select"
        | "Print" | "Execute" | "Insert" | "Undo" | "Redo" | "Menu" | "Find" | "Cancel"
        | "Help" | "Break" | "Arabic_switch" | "Greek_switch" | "Hangul_switch"
        | "Hebrew_switch" | "ISO_Group_Shift" | "Mode_switch" | "kana_switch" | "script_switch"
        | "Num_Lock" | "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10"
        | "F11" | "F12" | "F13" | "F14" | "F15" | "F16" | "F17" | "F18" | "F19" | "F20" | "F21"
        | "F22" | "F23" | "F24" | "F25" | "F26" | "F27" | "F28" | "F29" | "F30" | "F31" | "F32"
        | "F33" | "F34" | "F35" | "Shift_L" | "Shift_R" | "Control_L" | "Control_R"
        | "Caps_Lock" | "Shift_Lock" | "Meta_L" | "Meta_R" | "Alt_L" | "Alt_R" | "Super_L"
        | "Super_R" | "Hyper_L" | "Hyper_R" => KeyCode::Ignored,
        _ if is_librime_iso_key_name(name) => KeyCode::Ignored,
        "BackSpace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Escape" => KeyCode::Escape,
        "Left" => KeyCode::MoveCaretLeft,
        "Right" => KeyCode::MoveCaretRight,
        "KP_Left" => KeyCode::MoveCaretLeftByChar,
        "KP_Right" => KeyCode::MoveCaretRightByChar,
        "Up" | "KP_Up" => KeyCode::PreviousCandidate,
        "Down" | "KP_Down" => KeyCode::NextCandidate,
        "Home" | "KP_Home" => KeyCode::Home,
        "End" | "KP_End" => KeyCode::End,
        "Page_Up" | "Prior" | "KP_Page_Up" | "KP_Prior" => KeyCode::PreviousPage,
        "Page_Down" | "Next" | "KP_Page_Down" | "KP_Next" => KeyCode::NextPage,
        "Return" => KeyCode::Return,
        "KP_Enter" => KeyCode::KeypadEnter,
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

fn is_librime_iso_key_name(name: &str) -> bool {
    matches!(
        name,
        "ISO_Lock"
            | "ISO_Level2_Latch"
            | "ISO_Level3_Shift"
            | "ISO_Level3_Latch"
            | "ISO_Level3_Lock"
            | "ISO_Group_Latch"
            | "ISO_Group_Lock"
            | "ISO_Next_Group"
            | "ISO_Next_Group_Lock"
            | "ISO_Prev_Group"
            | "ISO_Prev_Group_Lock"
            | "ISO_First_Group"
            | "ISO_First_Group_Lock"
            | "ISO_Last_Group"
            | "ISO_Last_Group_Lock"
            | "ISO_Left_Tab"
            | "ISO_Move_Line_Up"
            | "ISO_Move_Line_Down"
            | "ISO_Partial_Line_Up"
            | "ISO_Partial_Line_Down"
            | "ISO_Partial_Space_Left"
            | "ISO_Partial_Space_Right"
            | "ISO_Set_Margin_Left"
            | "ISO_Set_Margin_Right"
            | "ISO_Release_Margin_Left"
            | "ISO_Release_Margin_Right"
            | "ISO_Release_Both_Margins"
            | "ISO_Fast_Cursor_Left"
            | "ISO_Fast_Cursor_Right"
            | "ISO_Fast_Cursor_Up"
            | "ISO_Fast_Cursor_Down"
            | "ISO_Continuous_Underline"
            | "ISO_Discontinuous_Underline"
            | "ISO_Emphasize"
            | "ISO_Center_Object"
            | "ISO_Enter"
    )
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
        if is_exact_control_shift_modifier(key_event.modifiers) && key_event.code == KeyCode::Return
        {
            return self.commit_comment();
        }
        if is_exact_control_shift_modifier(key_event.modifiers) {
            match key_event.code {
                KeyCode::Character(ch)
                    if ch.is_ascii_digit() && !self.context.candidates.is_empty() =>
                {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                KeyCode::KeypadDigit(ch) if !self.context.candidates.is_empty() => {
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
                KeyCode::KeypadDigit(ch) if !self.context.candidates.is_empty() => {
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
                    if ch.is_ascii_digit() && !self.context.candidates.is_empty() =>
                {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                KeyCode::KeypadDigit(ch) if !self.context.candidates.is_empty() => {
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
            KeyCode::KeypadDigit(ch) if !self.context.candidates.is_empty() => {
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
        let next_index = next_index.min(self.context.candidates.len() - 1);
        if current_index == next_index {
            return false;
        }
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

    pub fn move_caret_left(&mut self) -> bool {
        if self.context.composition.caret == 0 {
            return false;
        }
        self.context.composition.caret -= 1;
        true
    }

    pub fn move_caret_right(&mut self) -> bool {
        if self.context.composition.caret >= self.context.composition.input.len() {
            return false;
        }
        self.context.composition.caret += 1;
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
        if self.context.composition.caret == 0 {
            return None;
        }
        self.context.composition.caret -= 1;
        self.context
            .composition
            .input
            .remove(self.context.composition.caret);
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

    fn commit_raw_input(&mut self) -> Option<String> {
        if self.context.composition.input.is_empty() {
            return None;
        }
        let text = self.context.composition.input.clone();
        self.context.last_commit = Some(text.clone());
        self.clear_composition();
        Some(text)
    }

    fn commit_script_text(&mut self) -> Option<String> {
        if self.context.composition.preedit.is_empty() {
            return None;
        }
        let text = self.context.composition.preedit.clone();
        self.context.last_commit = Some(text.clone());
        self.clear_composition();
        Some(text)
    }

    fn commit_comment(&mut self) -> Option<String> {
        let text = self
            .context
            .candidates
            .get(self.context.highlighted)
            .and_then(|candidate| {
                (!candidate.comment.is_empty()).then(|| candidate.comment.clone())
            })?;
        self.context.last_commit = Some(text.clone());
        self.clear_composition();
        Some(text)
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

#[cfg(test)]
mod tests {
    use super::{
        parse_key_sequence, Candidate, CandidateRanker, CandidateSource, Context, Engine, KeyCode,
        MockAiRanker, PunctuationTranslator, RerankResult, StaticTableTranslator, TableDictionary,
        Translator,
    };

    struct CommentTranslator;

    impl Translator for CommentTranslator {
        fn name(&self) -> &'static str {
            "comment_translator"
        }

        fn translate(&self, input: &str) -> Vec<Candidate> {
            if input != "ni" {
                return Vec::new();
            }
            vec![
                Candidate {
                    text: "你".to_owned(),
                    comment: "first-comment".to_owned(),
                    source: CandidateSource::Table,
                    quality: 1.0,
                },
                Candidate {
                    text: "呢".to_owned(),
                    comment: "second-comment".to_owned(),
                    source: CandidateSource::Table,
                    quality: 1.0,
                },
            ]
        }
    }

    #[test]
    fn parses_librime_style_key_sequence_names() {
        let keys = parse_key_sequence(
            "zyx 123{Shift+space}ABC{Control+Alt+Return}{KP_Enter}{KP_2}{Tab}{Delete}{Escape}{Left}{Right}{KP_Left}{KP_Right}{Home}{KP_End}{Page_Down}{KP_Page_Up}{Down}{KP_Up}{Control+Up}{Control+Down}",
        )
        .expect("key sequence should parse");

        assert_eq!(keys.len(), 29);
        assert_eq!(keys[3].code, KeyCode::Character(' '));
        assert!(!keys[3].modifiers.shift);
        assert_eq!(keys[7].code, KeyCode::Character(' '));
        assert!(keys[7].modifiers.shift);
        assert_eq!(keys[11].code, KeyCode::Return);
        assert!(keys[11].modifiers.control);
        assert!(keys[11].modifiers.alt);
        assert_eq!(keys[12].code, KeyCode::KeypadEnter);
        assert_eq!(keys[13].code, KeyCode::KeypadDigit('2'));
        assert_eq!(keys[14].code, KeyCode::Tab);
        assert_eq!(keys[15].code, KeyCode::Delete);
        assert_eq!(keys[16].code, KeyCode::Escape);
        assert_eq!(keys[17].code, KeyCode::MoveCaretLeft);
        assert_eq!(keys[18].code, KeyCode::MoveCaretRight);
        assert_eq!(keys[19].code, KeyCode::MoveCaretLeftByChar);
        assert_eq!(keys[20].code, KeyCode::MoveCaretRightByChar);
        assert_eq!(keys[21].code, KeyCode::Home);
        assert_eq!(keys[22].code, KeyCode::End);
        assert_eq!(keys[23].code, KeyCode::NextPage);
        assert_eq!(keys[24].code, KeyCode::PreviousPage);
        assert_eq!(keys[25].code, KeyCode::NextCandidate);
        assert_eq!(keys[26].code, KeyCode::PreviousCandidate);
        assert_eq!(keys[27].code, KeyCode::MoveCaretLeftBySyllable);
        assert!(keys[27].modifiers.control);
        assert_eq!(keys[28].code, KeyCode::MoveCaretRightBySyllable);
        assert!(keys[28].modifiers.control);
    }

    #[test]
    fn parses_named_braces_for_literal_brace_keys() {
        let keys =
            parse_key_sequence("{braceleft}{braceright}").expect("key sequence should parse");

        assert_eq!(keys[0].code, KeyCode::Character('{'));
        assert_eq!(keys[1].code, KeyCode::Character('}'));
    }

    #[test]
    fn parses_librime_known_noop_key_names() {
        let keys = parse_key_sequence(
            "{Linefeed}{Clear}{Pause}{Scroll_Lock}{Sys_Req}{Begin}{Select}{Print}{Execute}{Insert}{Undo}{Redo}{Menu}{Find}{Cancel}{Help}{Break}{Arabic_switch}{Greek_switch}{Hangul_switch}{Hebrew_switch}{ISO_Group_Shift}{Mode_switch}{kana_switch}{script_switch}{Num_Lock}{F1}{Alt+F4}{F12}{F13}{F35}{Shift_L}{Shift_R}{Control_L}{Control_R}{Caps_Lock}{Shift_Lock}{Meta_L}{Meta_R}{Alt_L}{Alt_R}{Super_L}{Super_R}{Hyper_L}{Release+Hyper_R}{ISO_Lock}{ISO_Level2_Latch}{ISO_Level3_Shift}{ISO_Level3_Latch}{ISO_Level3_Lock}{ISO_Group_Latch}{ISO_Group_Lock}{ISO_Next_Group}{ISO_Next_Group_Lock}{ISO_Prev_Group}{ISO_Prev_Group_Lock}{ISO_First_Group}{ISO_First_Group_Lock}{ISO_Last_Group}{ISO_Last_Group_Lock}{ISO_Left_Tab}{ISO_Move_Line_Up}{ISO_Move_Line_Down}{ISO_Partial_Line_Up}{ISO_Partial_Line_Down}{ISO_Partial_Space_Left}{ISO_Partial_Space_Right}{ISO_Set_Margin_Left}{ISO_Set_Margin_Right}{ISO_Release_Margin_Left}{ISO_Release_Margin_Right}{ISO_Release_Both_Margins}{ISO_Fast_Cursor_Left}{ISO_Fast_Cursor_Right}{ISO_Fast_Cursor_Up}{ISO_Fast_Cursor_Down}{ISO_Continuous_Underline}{ISO_Discontinuous_Underline}{ISO_Emphasize}{ISO_Center_Object}{Release+ISO_Enter}",
        )
        .expect("key sequence should parse");

        assert_eq!(keys.len(), 81);
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys.iter().enumerate().all(|(index, key)| index == 27
            || index == 44
            || index == 80
            || key.modifiers.is_empty()));
        assert!(keys[27].modifiers.alt);
        assert!(keys[44].modifiers.release);
        assert!(keys[80].modifiers.release);
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
    fn shift_keypad_numeric_selection_matches_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{Shift+KP_2}ba{Shift+KP_2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn shift_ascii_numeric_selection_matches_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("ba{Shift+2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn control_ascii_numeric_selection_matches_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{Control+2}ba{Control+2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn control_keypad_numeric_selection_matches_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{Control+KP_2}ba{Control+KP_2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn control_shift_numeric_selection_matches_librime_selector_digit_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{Control+Shift+2}{Control+Shift+KP_2}ba{Control+Shift+2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);

        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("ba{Control+Shift+KP_2}")
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
    fn shift_escape_ignores_shift_like_librime_editor_cancel_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("ni{Shift+Escape}")
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
    fn backspace_removes_input_before_caret_like_librime_editor_back() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nxi");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);

        engine.set_input("ni");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{BackSpace}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.input, "ni");
        assert_eq!(engine.context().composition.caret, 0);
    }

    #[test]
    fn control_backspace_falls_back_to_previous_input_like_librime_editor_back_syllable() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nxi");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Control+BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn shift_backspace_uses_librime_editor_shift_as_control_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nxi");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Shift+BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn control_return_commits_raw_input_like_librime_fluid_editor() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("ni{Control+Return}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));
        assert!(!engine.status().is_composing);

        let commits = engine
            .process_key_sequence("{Control+Return}")
            .expect("key sequence should parse");
        assert!(commits.is_empty());
    }

    #[test]
    fn shift_return_commits_script_text_like_librime_fluid_editor() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("ni{Shift+Return}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));
        assert!(!engine.status().is_composing);

        let commits = engine
            .process_key_sequence("{Shift+Return}")
            .expect("key sequence should parse");
        assert!(commits.is_empty());
    }

    #[test]
    fn shift_printable_keys_enter_input_and_shift_space_confirms_like_librime_editor() {
        let mut engine = Engine::new();

        let commits = engine
            .process_key_sequence("{Shift+A}b{Shift+space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["Ab"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("Ab"));
        assert!(!engine.status().is_composing);

        let commits = engine
            .process_key_sequence("{Shift+space}")
            .expect("key sequence should parse");
        assert!(commits.is_empty());
        assert_eq!(engine.context().last_commit.as_deref(), Some("Ab"));
    }

    #[test]
    fn modified_keypad_enter_does_not_trigger_librime_return_only_editor_bindings() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence(
                "ni{Control+KP_Enter}{Shift+KP_Enter}{Control+Shift+KP_Enter}{KP_Enter}",
            )
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    }

    #[test]
    fn control_shift_return_commits_selected_comment_like_librime_fluid_editor() {
        let mut engine = Engine::new();
        engine.add_translator(CommentTranslator);

        let commits = engine
            .process_key_sequence("ni{Down}{Control+Shift+Return}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["second-comment"]);
        assert_eq!(
            engine.context().last_commit.as_deref(),
            Some("second-comment")
        );
        assert!(!engine.status().is_composing);

        let commits = engine
            .process_key_sequence("{Control+Shift+Return}")
            .expect("key sequence should parse");
        assert!(commits.is_empty());
    }

    #[test]
    fn left_right_keys_move_caret_like_librime_navigator() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("nix{Left}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);

        engine.set_input("nix");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{Right}{Right}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    }

    #[test]
    fn control_left_right_jump_across_simplified_syllable_span_like_librime_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Control+Left}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 0);

        let commits = engine
            .process_key_sequence("{Control+Right}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Control+Left}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn shift_left_right_fall_back_to_control_syllable_jump_like_librime_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Shift+Left}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 0);

        let commits = engine
            .process_key_sequence("{Shift+Right}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Shift+Left}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn control_up_down_jump_across_simplified_syllable_span_like_librime_vertical_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Control+Up}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 0);

        let commits = engine
            .process_key_sequence("{Control+Down}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Control+Up}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn shift_up_down_fall_back_to_control_syllable_jump_like_librime_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Shift+Up}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 0);

        let commits = engine
            .process_key_sequence("{Shift+Down}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Shift+Up}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn keypad_left_right_keys_move_caret_by_char_with_librime_navigator_looping() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nix");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{KP_Left}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 3);
        let commits = engine
            .process_key_sequence("{KP_Left}{Delete}{space}")
            .expect("key sequence should parse");
        assert_eq!(commits, vec!["你"]);

        engine.set_input("nix");
        engine.set_caret_pos(3);
        let commits = engine
            .process_key_sequence("{KP_Right}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn shift_keypad_left_right_ignore_shift_like_librime_navigator() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nix");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{Shift+KP_Left}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 3);
        let commits = engine
            .process_key_sequence("{Shift+KP_Left}{Delete}{space}")
            .expect("key sequence should parse");
        assert_eq!(commits, vec!["你"]);

        engine.set_input("nix");
        engine.set_caret_pos(3);
        let commits = engine
            .process_key_sequence("{Shift+KP_Right}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn shift_keypad_up_down_ignore_shift_like_librime_navigator() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nix");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{Shift+KP_Up}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 3);
        let commits = engine
            .process_key_sequence("{Shift+KP_Up}{Delete}{space}")
            .expect("key sequence should parse");
        assert_eq!(commits, vec!["你"]);

        engine.set_input("nix");
        engine.set_caret_pos(3);
        let commits = engine
            .process_key_sequence("{Shift+KP_Down}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
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
    fn home_end_keys_fall_back_to_librime_navigator_caret_movement() {
        let mut engine = Engine::new();

        let commits = engine
            .process_key_sequence("nix{Home}{Delete}{End}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["i"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("i"));
    }

    #[test]
    fn shift_home_end_keys_ignore_shift_like_librime_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Shift+Home}{Delete}{Shift+KP_End}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["i"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("i"));

        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
        engine
            .process_key_sequence("ba{Down}{Shift+Home}")
            .expect("key sequence should parse");

        assert_eq!(engine.context().highlighted, 1);
        assert_eq!(engine.context().composition.caret, 0);
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
        assert!(!engine.change_page(false));
        assert_eq!(engine.context().highlighted, 6);
        assert!(engine.highlight_candidate_on_current_page(0));
        assert_eq!(engine.context().highlighted, 5);
        assert!(!engine.highlight_candidate_on_current_page(5));
        assert_eq!(engine.context().highlighted, 5);
        assert!(engine.change_page(true));
        assert_eq!(engine.context().highlighted, 0);
        assert!(!engine.change_page(true));
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
    fn control_delete_removes_highlighted_candidate_like_librime_editor_delete_candidate() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));

        let commits = engine
            .process_key_sequence("ba{Down}{Control+Delete}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().candidates.len(), 3);
        assert_eq!(engine.context().candidates[1].text, "爸");
        assert_eq!(engine.context().highlighted, 1);
        assert_eq!(engine.context().last_commit, None);
    }

    #[test]
    fn shift_delete_removes_highlighted_candidate_like_librime_editor_shift_as_control_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));

        let commits = engine
            .process_key_sequence("ba{Down}{Shift+Delete}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().candidates.len(), 3);
        assert_eq!(engine.context().candidates[1].text, "爸");
        assert_eq!(engine.context().highlighted, 1);
        assert_eq!(engine.context().last_commit, None);
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
