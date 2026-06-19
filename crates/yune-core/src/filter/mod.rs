use super::{
    Candidate, CandidateFilter, CandidateSource, CommentFormat, Context, DictionaryLookupRecord,
    TableDictionary,
};
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};

pub struct UniquifierFilter;

impl CandidateFilter for UniquifierFilter {
    fn name(&self) -> &'static str {
        "uniquifier"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        let mut seen = HashSet::new();
        candidates.retain(|candidate| seen.insert(candidate.text.clone()));
    }
}

pub struct SingleCharFilter;

impl CandidateFilter for SingleCharFilter {
    fn name(&self) -> &'static str {
        "single_char_filter"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        let table_prefix_len = candidates
            .iter()
            .position(|candidate| candidate.source != CandidateSource::Table)
            .unwrap_or(candidates.len());
        if table_prefix_len <= 1 {
            return;
        }

        let mut phrases = candidates.drain(..table_prefix_len).collect::<Vec<_>>();
        let mut single_chars = Vec::new();
        let mut multi_chars = Vec::new();
        for candidate in phrases.drain(..) {
            if candidate.text.chars().count() == 1 {
                single_chars.push(candidate);
            } else {
                multi_chars.push(candidate);
            }
        }
        single_chars.append(&mut multi_chars);
        candidates.splice(..0, single_chars);
    }
}

pub struct CharsetFilter;

impl CandidateFilter for CharsetFilter {
    fn name(&self) -> &'static str {
        "charset_filter"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        candidates.retain(|candidate| !contains_extended_cjk(&candidate.text));
    }

    fn apply_with_options(&self, candidates: &mut Vec<Candidate>, options: &HashMap<String, bool>) {
        if !options.get("extended_charset").copied().unwrap_or(false) {
            self.apply(candidates);
        }
    }
}

pub struct DictionaryLookupFilter {
    records_by_text: HashMap<String, Vec<DictionaryLookupRecord>>,
}

impl DictionaryLookupFilter {
    #[must_use]
    pub fn new(dictionary: TableDictionary) -> Self {
        Self {
            records_by_text: dictionary.lookup_records,
        }
    }

    fn comment_for_candidate(&self, candidate: &Candidate) -> Option<String> {
        if candidate.source == CandidateSource::Sentence {
            if let Some(records) = self.sentence_lookup_records(&candidate.text) {
                let mut comment = String::new();
                comment.push('\u{000c}');
                for record in records {
                    append_dictionary_lookup_record(&mut comment, true, record);
                }
                return Some(comment);
            }
        }

        let records = self.records_by_text.get(&candidate.text)?;
        if records.is_empty() {
            return None;
        }

        let (comment_prefix, lookup_comment) = dictionary_lookup_comment_parts(&candidate.comment);
        let lookup_codes = split_lookup_codes(lookup_comment);
        let primary_indices = records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| lookup_codes.contains(&record.code).then_some(index))
            .collect::<Vec<_>>();
        let mut comment = String::from(comment_prefix);
        comment.push('\u{000c}');
        if primary_indices.is_empty() {
            append_dictionary_lookup_record(&mut comment, true, &records[0]);
            for record in records.iter().skip(1) {
                append_dictionary_lookup_record(&mut comment, false, record);
            }
        } else {
            for index in &primary_indices {
                append_dictionary_lookup_record(&mut comment, true, &records[*index]);
            }
            for (index, record) in records.iter().enumerate() {
                if !primary_indices.contains(&index) {
                    append_dictionary_lookup_record(&mut comment, false, record);
                }
            }
        }
        Some(comment)
    }

    fn sentence_lookup_records(&self, text: &str) -> Option<Vec<&DictionaryLookupRecord>> {
        let mut records = Vec::new();
        if let Some(exact_records) = self.records_by_text.get(text) {
            records.extend(exact_records.iter());
        }

        let mut cursor = 0;
        while cursor < text.len() {
            let Some((prefix_len, prefix_records)) = self.longest_sentence_prefix(text, cursor)
            else {
                return (!records.is_empty()).then_some(records);
            };
            records.extend(prefix_records.iter());
            cursor += prefix_len;
        }

        (!records.is_empty()).then_some(records)
    }

    fn longest_sentence_prefix(
        &self,
        text: &str,
        cursor: usize,
    ) -> Option<(usize, &Vec<DictionaryLookupRecord>)> {
        let boundaries = text[cursor..]
            .char_indices()
            .map(|(offset, _)| cursor + offset)
            .chain(std::iter::once(text.len()))
            .skip(1)
            .filter(|end| !(*end == text.len() && cursor == 0))
            .collect::<Vec<_>>();
        boundaries.into_iter().rev().find_map(|end| {
            let prefix = &text[cursor..end];
            self.records_by_text
                .get(prefix)
                .map(|records| (prefix.len(), records))
        })
    }
}

fn dictionary_lookup_comment_parts(comment: &str) -> (&str, &str) {
    if let Some((prefix, lookup_comment)) = comment.split_once('\u{000c}') {
        return (prefix, lookup_comment);
    }
    if comment.starts_with('\u{000b}') {
        return (comment, "");
    }
    ("", comment)
}

fn split_lookup_codes(comment: &str) -> HashSet<String> {
    comment
        .split(['\u{000c}', ';', ' ', '\t'])
        .filter(|code| !code.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

impl CandidateFilter for DictionaryLookupFilter {
    fn name(&self) -> &'static str {
        "dictionary_lookup_filter"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        for candidate in candidates {
            if !matches!(
                candidate.source,
                CandidateSource::Table | CandidateSource::Completion | CandidateSource::Sentence
            ) {
                continue;
            }
            if let Some(comment) = self.comment_for_candidate(candidate) {
                candidate.comment = comment;
            }
        }
    }
}

fn append_dictionary_lookup_record(
    comment: &mut String,
    is_primary: bool,
    record: &DictionaryLookupRecord,
) {
    comment.push('\r');
    comment.push(if is_primary { '1' } else { '0' });
    comment.push(',');
    comment.push_str(&record.fields.join(","));
}

pub struct TaggedFilter {
    filter: Box<dyn CandidateFilter>,
    tags: Vec<String>,
}

impl TaggedFilter {
    #[must_use]
    pub fn new(
        filter: impl CandidateFilter + 'static,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            filter: Box::new(filter),
            tags: tags.into_iter().map(Into::into).collect(),
        }
    }

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        self.tags.is_empty()
            || self
                .tags
                .iter()
                .any(|tag| segment_tags.iter().any(|segment_tag| segment_tag == tag))
    }
}

impl CandidateFilter for TaggedFilter {
    fn name(&self) -> &'static str {
        self.filter.name()
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        self.filter.apply(candidates);
    }

    fn apply_with_options(&self, candidates: &mut Vec<Candidate>, options: &HashMap<String, bool>) {
        self.filter.apply_with_options(candidates, options);
    }

    fn apply_with_context(
        &self,
        candidates: &mut Vec<Candidate>,
        options: &HashMap<String, bool>,
        context: &Context,
    ) {
        if self.accepts_segment_tags(&context.segment_tags) {
            self.filter.apply_with_context(candidates, options, context);
        }
    }
}

pub(crate) fn contains_extended_cjk(text: &str) -> bool {
    text.chars().any(is_extended_cjk)
}

fn is_extended_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4dbf
            | 0x20000..=0x2a6df
            | 0x2a700..=0x2b73f
            | 0x2b740..=0x2b81f
            | 0x2b820..=0x2ceaf
            | 0x2ceb0..=0x2ebef
            | 0x30000..=0x3134f
            | 0x31350..=0x323af
            | 0x2ebf0..=0x2ee5f
            | 0x323b0..=0x3347f
            | 0x3300..=0x33ff
            | 0xfe30..=0xfe4f
            | 0xf900..=0xfaff
            | 0x2f800..=0x2fa1f
    )
}

pub struct SimplifierFilter {
    option_name: String,
    conversion: SimplifierConversion,
    tips_level: SimplifierTipsLevel,
    show_in_comment: bool,
    inherit_comment: bool,
    comment_format: CommentFormat,
    excluded_types: HashSet<String>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SimplifierConversion {
    None,
    TraditionalToSimplified,
    HongKongToSimplified,
    SimplifiedToTraditional,
    TraditionalToTaiwan,
    SimplifiedToTaiwan,
    TaiwanToSimplified,
    TaiwanToTraditional,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SimplifierTipsLevel {
    None,
    Char,
    All,
}

impl Default for SimplifierFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl SimplifierFilter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            option_name: "simplification".to_owned(),
            conversion: SimplifierConversion::TraditionalToSimplified,
            tips_level: SimplifierTipsLevel::None,
            show_in_comment: false,
            inherit_comment: true,
            comment_format: CommentFormat::default(),
            excluded_types: HashSet::new(),
        }
    }

    #[must_use]
    pub fn with_option_name(mut self, option_name: impl Into<String>) -> Self {
        let option_name = option_name.into();
        if !option_name.is_empty() {
            self.option_name = option_name;
        }
        self
    }

    #[must_use]
    pub fn with_opencc_config(mut self, opencc_config: impl AsRef<str>) -> Self {
        self.conversion = SimplifierConversion::from_opencc_config(opencc_config.as_ref());
        self
    }

    #[must_use]
    pub fn with_tips(mut self, tips: impl AsRef<str>) -> Self {
        self.tips_level = match tips.as_ref() {
            "char" => SimplifierTipsLevel::Char,
            "all" => SimplifierTipsLevel::All,
            _ => SimplifierTipsLevel::None,
        };
        self
    }

    #[must_use]
    pub fn with_show_in_comment(mut self, show_in_comment: bool) -> Self {
        self.show_in_comment = show_in_comment;
        self
    }

    #[must_use]
    pub fn with_inherit_comment(mut self, inherit_comment: bool) -> Self {
        self.inherit_comment = inherit_comment;
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self
    }

    #[must_use]
    pub fn with_excluded_types(mut self, excluded_types: impl IntoIterator<Item = String>) -> Self {
        self.excluded_types = excluded_types
            .into_iter()
            .filter(|candidate_type| !candidate_type.is_empty())
            .collect();
        self
    }
}

impl SimplifierConversion {
    fn from_opencc_config(opencc_config: &str) -> Self {
        let config_name = opencc_config
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(opencc_config)
            .to_ascii_lowercase();
        let config_stem = config_name.strip_suffix(".json").unwrap_or(&config_name);
        match config_stem {
            "" | "t2s" => Self::TraditionalToSimplified,
            "hk2s" => Self::HongKongToSimplified,
            "s2t" => Self::SimplifiedToTraditional,
            "t2tw" => Self::TraditionalToTaiwan,
            "s2tw" => Self::SimplifiedToTaiwan,
            "tw2s" => Self::TaiwanToSimplified,
            "tw2t" => Self::TaiwanToTraditional,
            _ if config_stem.ends_with(".ini") => Self::None,
            _ => Self::None,
        }
    }

    fn convert(self, text: &str) -> String {
        match self {
            Self::None => text.to_owned(),
            Self::TraditionalToSimplified => simplify_traditional_text(text),
            Self::HongKongToSimplified => simplify_hong_kong_text(text),
            Self::SimplifiedToTraditional => traditionalize_simplified_text(text),
            Self::TraditionalToTaiwan => traditional_to_taiwan_text(text),
            Self::SimplifiedToTaiwan => {
                traditional_to_taiwan_text(&traditionalize_simplified_text(text))
            }
            Self::TaiwanToSimplified => {
                simplify_traditional_text(&taiwan_to_traditional_text(text))
            }
            Self::TaiwanToTraditional => taiwan_to_traditional_text(text),
        }
    }
}

impl CandidateFilter for SimplifierFilter {
    fn name(&self) -> &'static str {
        "simplifier"
    }

    fn apply(&self, _candidates: &mut Vec<Candidate>) {}

    fn apply_with_options(&self, candidates: &mut Vec<Candidate>, options: &HashMap<String, bool>) {
        if !options.get(&self.option_name).copied().unwrap_or(false) {
            return;
        }

        for candidate in candidates {
            if self.excluded_types.contains(candidate.source.as_str()) {
                continue;
            }

            let original = candidate.text.clone();
            let simplified = self.conversion.convert(&original);
            if simplified == original {
                continue;
            }

            let show_tips = match self.tips_level {
                SimplifierTipsLevel::None => false,
                SimplifierTipsLevel::Char => original.chars().count() == 1,
                SimplifierTipsLevel::All => true,
            };

            if self.show_in_comment {
                if show_tips {
                    candidate.comment = self.comment_format.apply(&simplified);
                } else if !self.inherit_comment {
                    candidate.comment.clear();
                }
            } else {
                candidate.text = simplified;
                if show_tips {
                    let (comment, modified) = self.comment_format.apply_with_modified(&original);
                    candidate.comment = if modified {
                        comment
                    } else {
                        format!("〔{original}〕")
                    };
                } else if !self.inherit_comment {
                    candidate.comment.clear();
                }
            }
        }
    }
}

const HK_VARIANTS: &str = include_str!("../opencc/data/HKVariants.txt");
const HK_VARIANTS_REV_PHRASES: &str = include_str!("../opencc/data/HKVariantsRevPhrases.txt");
const TS_CHARACTERS: &str = include_str!("../opencc/data/TSCharacters.txt");
const TS_PHRASES: &str = include_str!("../opencc/data/TSPhrases.txt");

static HK2S_OPENCC_CHAIN: OnceLock<OpenCcChain> = OnceLock::new();
static T2S_OPENCC_CHAIN: OnceLock<OpenCcChain> = OnceLock::new();
static S2T_OPENCC_CHAIN: OnceLock<OpenCcChain> = OnceLock::new();

#[derive(Default)]
struct OpenCcStage {
    phrases: HashMap<String, String>,
    chars: HashMap<char, String>,
    max_phrase_chars: usize,
}

struct OpenCcChain {
    stages: Vec<OpenCcStage>,
}

impl OpenCcChain {
    fn convert(&self, text: &str) -> String {
        self.stages
            .iter()
            .fold(text.to_owned(), |converted, stage| {
                stage.convert(&converted)
            })
    }
}

impl OpenCcStage {
    fn convert(&self, text: &str) -> String {
        let mut converted = String::new();
        let mut cursor = 0;
        while cursor < text.len() {
            if let Some((matched_len, replacement)) = self.longest_phrase_match(text, cursor) {
                converted.push_str(replacement);
                cursor += matched_len;
                continue;
            }
            let Some(ch) = text[cursor..].chars().next() else {
                break;
            };
            if let Some(replacement) = self.chars.get(&ch) {
                converted.push_str(replacement);
            } else {
                converted.push(ch);
            }
            cursor += ch.len_utf8();
        }
        converted
    }

    fn longest_phrase_match<'a>(
        &'a self,
        text: &'a str,
        cursor: usize,
    ) -> Option<(usize, &'a str)> {
        if self.max_phrase_chars == 0 {
            return None;
        }
        let mut boundaries = text[cursor..]
            .char_indices()
            .map(|(offset, _)| cursor + offset)
            .chain(std::iter::once(text.len()))
            .skip(1)
            .take(self.max_phrase_chars)
            .collect::<Vec<_>>();
        boundaries.reverse();
        boundaries.into_iter().find_map(|end| {
            let phrase = &text[cursor..end];
            self.phrases
                .get(phrase)
                .map(|replacement| (phrase.len(), replacement.as_str()))
        })
    }
}

fn simplify_traditional_text(text: &str) -> String {
    t2s_opencc_chain().convert(text)
}

fn simplify_hong_kong_text(text: &str) -> String {
    hk2s_opencc_chain().convert(text)
}

fn traditionalize_simplified_text(text: &str) -> String {
    s2t_opencc_chain().convert(text)
}

fn hk2s_opencc_chain() -> &'static OpenCcChain {
    HK2S_OPENCC_CHAIN.get_or_init(|| OpenCcChain {
        stages: vec![
            OpenCcStage {
                phrases: parse_opencc_phrase_map(HK_VARIANTS_REV_PHRASES),
                chars: parse_reverse_opencc_char_map(HK_VARIANTS),
                max_phrase_chars: max_opencc_key_chars(HK_VARIANTS_REV_PHRASES),
            },
            OpenCcStage {
                phrases: parse_opencc_phrase_map(TS_PHRASES),
                chars: parse_opencc_char_map(TS_CHARACTERS),
                max_phrase_chars: max_opencc_key_chars(TS_PHRASES),
            },
        ],
    })
}

fn t2s_opencc_chain() -> &'static OpenCcChain {
    T2S_OPENCC_CHAIN.get_or_init(|| OpenCcChain {
        stages: vec![OpenCcStage {
            phrases: parse_opencc_phrase_map(TS_PHRASES),
            chars: parse_opencc_char_map(TS_CHARACTERS),
            max_phrase_chars: max_opencc_key_chars(TS_PHRASES),
        }],
    })
}

fn s2t_opencc_chain() -> &'static OpenCcChain {
    S2T_OPENCC_CHAIN.get_or_init(|| OpenCcChain {
        stages: vec![OpenCcStage {
            phrases: HashMap::new(),
            chars: parse_reverse_opencc_char_map(TS_CHARACTERS),
            max_phrase_chars: 0,
        }],
    })
}

fn parse_opencc_phrase_map(data: &str) -> HashMap<String, String> {
    parse_opencc_entries(data)
        .filter(|(key, _)| key.chars().count() > 1)
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

fn parse_opencc_char_map(data: &str) -> HashMap<char, String> {
    parse_opencc_entries(data)
        .filter_map(|(key, value)| {
            let mut key_chars = key.chars();
            let key = key_chars.next()?;
            key_chars.next().is_none().then(|| (key, value.to_owned()))
        })
        .collect()
}

fn parse_reverse_opencc_char_map(data: &str) -> HashMap<char, String> {
    let mut chars = HashMap::new();
    for (key, values) in parse_opencc_entries_with_values(data) {
        if key.chars().count() != 1 {
            continue;
        }
        for value in values {
            let mut value_chars = value.chars();
            let Some(value_ch) = value_chars.next() else {
                continue;
            };
            if value_chars.next().is_none() {
                chars.entry(value_ch).or_insert_with(|| key.to_owned());
            }
        }
    }
    chars
}

fn max_opencc_key_chars(data: &str) -> usize {
    parse_opencc_entries(data)
        .map(|(key, _)| key.chars().count())
        .max()
        .unwrap_or(0)
}

fn parse_opencc_entries(data: &str) -> impl Iterator<Item = (&str, &str)> {
    parse_opencc_entries_with_values(data)
        .filter_map(|(key, mut values)| values.next().map(|value| (key, value)))
}

fn parse_opencc_entries_with_values(
    data: &str,
) -> impl Iterator<Item = (&str, std::str::SplitWhitespace<'_>)> {
    data.lines().filter_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let mut parts = line.split_whitespace();
        let key = parts.next()?;
        Some((key, parts))
    })
}

fn traditional_to_taiwan_text(text: &str) -> String {
    text.chars().map(traditional_to_taiwan_char).collect()
}

fn traditional_to_taiwan_char(ch: char) -> char {
    match ch {
        '台' | '臺' => '臺',
        '裏' | '裡' => '裡',
        _ => ch,
    }
}

fn taiwan_to_traditional_text(text: &str) -> String {
    text.chars().map(taiwan_to_traditional_char).collect()
}

fn taiwan_to_traditional_char(ch: char) -> char {
    match ch {
        '裡' => '裏',
        _ => ch,
    }
}

pub struct ReverseLookupFilter {
    reverse_comments: HashMap<String, Vec<String>>,
    overwrite_comment: bool,
    append_comment: bool,
    comment_format: CommentFormat,
}

impl ReverseLookupFilter {
    #[must_use]
    pub fn new(reverse_dictionary: TableDictionary) -> Self {
        let mut reverse_comments: HashMap<String, Vec<String>> = HashMap::new();
        for entry in reverse_dictionary.entries {
            reverse_comments
                .entry(entry.text)
                .or_default()
                .push(entry.code);
        }

        Self {
            reverse_comments,
            overwrite_comment: false,
            append_comment: false,
            comment_format: CommentFormat::default(),
        }
    }

    #[must_use]
    pub fn with_overwrite_comment(mut self, overwrite_comment: bool) -> Self {
        self.overwrite_comment = overwrite_comment;
        self
    }

    #[must_use]
    pub fn with_append_comment(mut self, append_comment: bool) -> Self {
        self.append_comment = append_comment;
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self
    }
}

impl CandidateFilter for ReverseLookupFilter {
    fn name(&self) -> &'static str {
        "reverse_lookup_filter"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        for candidate in candidates {
            if !matches!(
                candidate.source,
                CandidateSource::Table | CandidateSource::Completion | CandidateSource::Sentence
            ) {
                continue;
            }
            if !(candidate.comment.is_empty() || self.overwrite_comment || self.append_comment) {
                continue;
            }

            let Some(comments) = self.reverse_comments.get(&candidate.text) else {
                continue;
            };
            if comments.is_empty() {
                continue;
            }

            let reverse_comment = self.comment_format.apply(&comments.join("; "));
            if self.overwrite_comment || candidate.comment.is_empty() {
                candidate.comment = reverse_comment;
            } else {
                candidate.comment = format!("{}; {reverse_comment}", candidate.comment);
            }
        }
    }
}
