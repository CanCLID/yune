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
    Backspace,
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
        "Return" => KeyCode::Return,
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
    translators: Vec<Box<dyn Translator>>,
    rankers: Vec<Box<dyn CandidateRanker>>,
}

const DEFAULT_PAGE_SIZE: usize = 5;

impl Default for Engine {
    fn default() -> Self {
        Self {
            context: Context::default(),
            status: Status::default(),
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
            KeyCode::Backspace => self.backspace(),
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
        self.context.composition = Composition::default();
        self.context.candidates.clear();
        self.context.highlighted = 0;
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
        let keys = parse_key_sequence("zyx 123{Shift+space}ABC{Control+Alt+Return}")
            .expect("key sequence should parse");

        assert_eq!(keys.len(), 12);
        assert_eq!(keys[3].code, KeyCode::Character(' '));
        assert!(!keys[3].modifiers.shift);
        assert_eq!(keys[7].code, KeyCode::Character(' '));
        assert!(keys[7].modifiers.shift);
        assert_eq!(keys[11].code, KeyCode::Return);
        assert!(keys[11].modifiers.control);
        assert!(keys[11].modifiers.alt);
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
