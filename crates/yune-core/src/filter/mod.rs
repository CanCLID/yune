use super::{
    Candidate, CandidateFilter, CandidateSource, CommentFormat, Context, DictionaryLookupRecord,
    TableDictionary,
};
use std::collections::{HashMap, HashSet};

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
        let records = self.records_by_text.get(&candidate.text)?;
        if records.is_empty() {
            return None;
        }

        let (comment_prefix, lookup_comment) = candidate
            .comment
            .split_once('\u{000c}')
            .map_or(("", candidate.comment.as_str()), |(prefix, comment)| {
                (prefix, comment)
            });
        let lookup_code = lookup_comment
            .split('\u{000c}')
            .next()
            .unwrap_or_default()
            .replace([';', ' '], "");
        let primary_index = records
            .iter()
            .position(|record| !lookup_code.is_empty() && record.code == lookup_code)
            .unwrap_or(0);
        let mut comment = String::from(comment_prefix);
        comment.push('\u{000c}');
        append_dictionary_lookup_record(&mut comment, true, &records[primary_index]);
        for (index, record) in records.iter().enumerate() {
            if index != primary_index {
                append_dictionary_lookup_record(&mut comment, false, record);
            }
        }
        Some(comment)
    }
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
            "" | "t2s" | "hk2s" => Self::TraditionalToSimplified,
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

fn simplify_traditional_text(text: &str) -> String {
    text.chars().map(simplify_traditional_char).collect()
}

fn simplify_traditional_char(ch: char) -> char {
    match ch {
        '臺' | '檯' | '颱' => '台',
        '灣' => '湾',
        '龍' => '龙',
        '風' => '风',
        '雲' => '云',
        '馬' => '马',
        '門' => '门',
        '車' => '车',
        '書' => '书',
        '學' => '学',
        '國' => '国',
        '語' => '语',
        '體' => '体',
        '電' => '电',
        '腦' => '脑',
        '麵' => '面',
        '裏' | '裡' => '里',
        '後' => '后',
        '萬' => '万',
        '與' => '与',
        '為' => '为',
        '會' => '会',
        '個' => '个',
        '們' => '们',
        '來' => '来',
        '時' => '时',
        '對' => '对',
        '說' => '说',
        '這' => '这',
        '還' => '还',
        '過' => '过',
        '\u{934b}' => '\u{9505}',
        '開' => '开',
        '關' => '关',
        '見' => '见',
        '長' => '长',
        '發' => '发',
        '頭' => '头',
        '東' => '东',
        '廣' => '广',
        '愛' => '爱',
        '氣' => '气',
        '無' => '无',
        '點' => '点',
        '話' => '话',
        '機' => '机',
        '樂' => '乐',
        '貓' => '猫',
        '鳥' => '鸟',
        '魚' => '鱼',
        _ => ch,
    }
}

fn traditionalize_simplified_text(text: &str) -> String {
    text.chars().map(traditionalize_simplified_char).collect()
}

fn traditionalize_simplified_char(ch: char) -> char {
    match ch {
        '台' => '臺',
        '湾' => '灣',
        '龙' => '龍',
        '风' => '風',
        '云' => '雲',
        '马' => '馬',
        '门' => '門',
        '车' => '車',
        '书' => '書',
        '学' => '學',
        '国' => '國',
        '语' => '語',
        '体' => '體',
        '电' => '電',
        '脑' => '腦',
        '面' => '麵',
        '里' => '裏',
        '后' => '後',
        '万' => '萬',
        '与' => '與',
        '为' => '為',
        '会' => '會',
        '个' => '個',
        '们' => '們',
        '来' => '來',
        '时' => '時',
        '对' => '對',
        '说' => '說',
        '这' => '這',
        '还' => '還',
        '过' => '過',
        '\u{9505}' => '\u{934b}',
        '开' => '開',
        '关' => '關',
        '见' => '見',
        '长' => '長',
        '发' => '發',
        '头' => '頭',
        '东' => '東',
        '广' => '廣',
        '爱' => '愛',
        '气' => '氣',
        '无' => '無',
        '点' => '點',
        '话' => '話',
        '机' => '機',
        '乐' => '樂',
        '猫' => '貓',
        '鸟' => '鳥',
        '鱼' => '魚',
        _ => ch,
    }
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
