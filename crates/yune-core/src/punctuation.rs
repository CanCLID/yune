use super::{Candidate, CandidateSource, Context, Status, Translator};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PunctuationDefinition {
    Candidates(Vec<String>),
    Commit(String),
    ConfirmUnique(String),
    Pair([String; 2]),
}

impl PunctuationDefinition {
    #[must_use]
    pub fn candidate_texts(&self) -> Vec<String> {
        match self {
            Self::Candidates(values) => values.clone(),
            Self::Commit(value) => vec![value.clone()],
            Self::ConfirmUnique(value) => vec![value.clone()],
            Self::Pair(values) => values.to_vec(),
        }
    }
}

pub struct PunctuationProcessor {
    half_shape_definitions: HashMap<String, PunctuationDefinition>,
    full_shape_definitions: HashMap<String, PunctuationDefinition>,
    symbol_definitions: HashMap<String, PunctuationDefinition>,
    pair_oddness: HashMap<String, usize>,
}

pub(crate) enum PunctuationProcessorResult {
    Accepted,
    Preview(String),
    Candidates {
        input: String,
        texts: Vec<String>,
        highlighted: usize,
    },
    Commit(String),
}

impl PunctuationProcessor {
    #[must_use]
    pub fn with_shape_definitions(
        half_shape_definitions: impl IntoIterator<Item = (impl Into<String>, PunctuationDefinition)>,
        full_shape_definitions: impl IntoIterator<Item = (impl Into<String>, PunctuationDefinition)>,
        symbol_definitions: impl IntoIterator<Item = (impl Into<String>, PunctuationDefinition)>,
    ) -> Self {
        Self {
            half_shape_definitions: half_shape_definitions
                .into_iter()
                .map(|(key, definition)| (key.into(), definition))
                .collect(),
            full_shape_definitions: full_shape_definitions
                .into_iter()
                .map(|(key, definition)| (key.into(), definition))
                .collect(),
            symbol_definitions: symbol_definitions
                .into_iter()
                .map(|(key, definition)| (key.into(), definition))
                .collect(),
            pair_oddness: HashMap::new(),
        }
    }

    pub(crate) fn process_key(
        &mut self,
        key: &str,
        is_full_shape: bool,
        is_ascii_punct: bool,
        current_input: &str,
        candidates_len: usize,
        highlighted: usize,
    ) -> Option<PunctuationProcessorResult> {
        let definition = self.active_definition(key, is_full_shape).cloned()?;
        if is_ascii_punct {
            return Some(PunctuationProcessorResult::Accepted);
        }

        match definition {
            PunctuationDefinition::Commit(value) => Some(PunctuationProcessorResult::Commit(value)),
            PunctuationDefinition::ConfirmUnique(value) => {
                Some(PunctuationProcessorResult::Preview(value))
            }
            PunctuationDefinition::Pair(_) => self.pair_preview(key, is_full_shape, current_input),
            PunctuationDefinition::Candidates(values) => {
                if values.is_empty() {
                    return None;
                }
                let count = values.len();
                let next_index = if candidates_len > 0 && !current_input.is_empty() {
                    (highlighted + 1) % count
                } else {
                    0
                };
                Some(PunctuationProcessorResult::Candidates {
                    input: values[next_index].clone(),
                    texts: values,
                    highlighted: next_index,
                })
            }
        }
    }

    fn active_definition(&self, key: &str, is_full_shape: bool) -> Option<&PunctuationDefinition> {
        let shape_definitions = if is_full_shape {
            &self.full_shape_definitions
        } else {
            &self.half_shape_definitions
        };
        shape_definitions
            .get(key)
            .or_else(|| self.symbol_definitions.get(key))
    }

    fn pair_preview(
        &mut self,
        key: &str,
        is_full_shape: bool,
        current_input: &str,
    ) -> Option<PunctuationProcessorResult> {
        let shape_name = if is_full_shape {
            "full_shape"
        } else {
            "half_shape"
        };
        let (pair_name, pair) = if is_full_shape {
            self.full_shape_definitions
                .get(key)
                .and_then(|definition| match definition {
                    PunctuationDefinition::Pair(pair) => Some((shape_name, pair.clone())),
                    _ => None,
                })
        } else {
            self.half_shape_definitions
                .get(key)
                .and_then(|definition| match definition {
                    PunctuationDefinition::Pair(pair) => Some((shape_name, pair.clone())),
                    _ => None,
                })
        }
        .or_else(|| {
            self.symbol_definitions
                .get(key)
                .and_then(|definition| match definition {
                    PunctuationDefinition::Pair(pair) => Some(("symbols", pair.clone())),
                    _ => None,
                })
        })?;
        let oddness_key = format!("{pair_name}:{key}");
        let oddness = self.pair_oddness.entry(oddness_key).or_insert(0);
        let text = pair[*oddness % 2].clone();
        *oddness = 1 - (*oddness % 2);
        Some(PunctuationProcessorResult::Preview(format!(
            "{current_input}{text}"
        )))
    }
}

pub struct PunctuationTranslator {
    half_shape_entries: Vec<(String, Candidate)>,
    full_shape_entries: Vec<(String, Candidate)>,
    symbol_entries: Vec<(String, Candidate)>,
    required_tags: Option<Vec<String>>,
}

impl PunctuationTranslator {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        Self::with_shape_entries(entries, std::iter::empty::<(String, String)>())
    }

    #[must_use]
    pub fn with_shape_entries(
        half_shape_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        full_shape_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self::with_shape_and_symbol_entries(
            half_shape_entries,
            full_shape_entries,
            std::iter::empty::<(String, String)>(),
        )
    }

    #[must_use]
    pub fn with_shape_and_symbol_entries(
        half_shape_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        full_shape_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        symbol_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            half_shape_entries: punctuation_candidates(half_shape_entries),
            full_shape_entries: punctuation_candidates(full_shape_entries),
            symbol_entries: punctuation_candidates(symbol_entries),
            required_tags: None,
        }
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

    #[must_use]
    pub fn with_required_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let tags = tags.into_iter().map(Into::into).collect::<Vec<_>>();
        self.required_tags = (!tags.is_empty()).then_some(tags);
        self
    }
}

impl Translator for PunctuationTranslator {
    fn name(&self) -> &'static str {
        "punct_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        self.translate_with_entries(input, &self.half_shape_entries)
    }

    fn translate_with_status(&self, input: &str, status: &Status) -> Vec<Candidate> {
        let entries = if status.is_full_shape {
            &self.full_shape_entries
        } else {
            &self.half_shape_entries
        };
        self.translate_with_entries(input, entries)
    }

    fn translate_with_context(
        &self,
        input: &str,
        status: &Status,
        _options: &HashMap<String, bool>,
        context: &Context,
    ) -> Vec<Candidate> {
        if context
            .segment_tags
            .iter()
            .any(|segment_tag| segment_tag == "punct_number")
            && !input.is_empty()
        {
            let text = shape_formatted_ascii_text(input, status.is_full_shape);
            return vec![Candidate {
                comment: punctuation_candidate_comment(&text).to_owned(),
                text,
                preedit: None,
                source: CandidateSource::Punctuation,
                quality: 1.0,
            }];
        }
        if self.required_tags.as_ref().is_some_and(|required_tags| {
            !required_tags.iter().any(|tag| {
                context
                    .segment_tags
                    .iter()
                    .any(|segment_tag| segment_tag == tag)
            })
        }) {
            return Vec::new();
        }
        self.translate_with_status(input, status)
    }
}

impl PunctuationTranslator {
    fn translate_with_entries(
        &self,
        input: &str,
        shape_entries: &[(String, Candidate)],
    ) -> Vec<Candidate> {
        let shape_candidates = shape_entries
            .iter()
            .filter(|(key, _)| key == input)
            .map(|(_, candidate)| candidate.clone())
            .collect::<Vec<_>>();
        if !shape_candidates.is_empty() {
            return shape_candidates;
        }
        self.symbol_entries
            .iter()
            .filter(|(key, _)| key == input)
            .map(|(_, candidate)| candidate.clone())
            .collect()
    }
}

fn punctuation_candidates(
    entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
) -> Vec<(String, Candidate)> {
    entries
        .into_iter()
        .map(|(key, text)| {
            let key = key.into();
            let text = text.into();
            (
                key.clone(),
                Candidate {
                    comment: punctuation_candidate_comment(&text).to_owned(),
                    text,
                    preedit: None,
                    source: CandidateSource::Punctuation,
                    quality: 1.0,
                },
            )
        })
        .collect()
}

pub(crate) fn punctuation_candidate_comment(punct: &str) -> &'static str {
    let mut characters = punct.chars();
    let Some(ch) = characters.next() else {
        return "";
    };
    if characters.next().is_some() {
        return "";
    }

    if is_librime_half_shape_punct(ch) {
        "\u{3014}\u{534a}\u{89d2}\u{3015}"
    } else if is_librime_full_shape_punct(ch) {
        "\u{3014}\u{5168}\u{89d2}\u{3015}"
    } else {
        ""
    }
}

fn shape_formatted_ascii_text(text: &str, full_shape: bool) -> String {
    if !full_shape {
        return text.to_owned();
    }
    text.chars()
        .map(|ch| match ch {
            ' ' => '\u{3000}',
            '!'..='~' => char::from_u32(ch as u32 + 0xfee0)
                .expect("printable ASCII has a full-shape compatibility form"),
            _ => ch,
        })
        .collect()
}

fn is_librime_half_shape_punct(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x20..=0x7e
            | 0xff61..=0xff9f
            | 0xffa0..=0xffdc
            | 0x00a2
            | 0x00a3
            | 0x00a5
            | 0x00a6
            | 0x00ac
            | 0x00af
            | 0x2985
            | 0x2986
            | 0xffe8..=0xffee
    )
}

fn is_librime_full_shape_punct(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x3000
            | 0xff01..=0xff5e
            | 0x30a1..=0x30fc
            | 0x3001
            | 0x3002
            | 0x300c
            | 0x300d
            | 0x309b
            | 0x309c
            | 0x3131..=0x3164
            | 0xff5f
            | 0xff60
            | 0xffe0..=0xffe6
            | 0x2190..=0x2193
            | 0x2502
            | 0x25a0
            | 0x25cb
    )
}
