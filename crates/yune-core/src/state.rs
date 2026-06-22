#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub text: String,
    pub comment: String,
    pub preedit: Option<String>,
    pub source: CandidateSource,
    pub quality: f32,
}

impl Candidate {
    #[must_use]
    pub fn commit_text_for_input(&self, input: &str) -> String {
        let Some(consumed) = self.source.partial_consumed_len() else {
            return self.text.clone();
        };
        if consumed > input.len() || !input.is_char_boundary(consumed) {
            return self.text.clone();
        }
        let suffix = &input[consumed..];
        if suffix.is_empty() {
            self.text.clone()
        } else {
            format!("{}{}", self.text, suffix)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateSource {
    Echo,
    Punctuation,
    Table,
    PartialTable {
        consumed: usize,
        recompose_on_default: bool,
    },
    UserTable,
    Completion,
    Sentence,
    ReverseLookup,
    History,
    Switch,
    Unfold,
    Schema,
    Ai {
        provider: String,
        confidence: AiConfidence,
    },
}

impl CandidateSource {
    #[must_use]
    pub fn ai(provider: impl Into<String>, confidence: AiConfidence) -> Self {
        Self::Ai {
            provider: provider.into(),
            confidence,
        }
    }

    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Echo => "echo",
            Self::Punctuation => "punct",
            Self::Table | Self::PartialTable { .. } => "table",
            Self::UserTable => "user_table",
            Self::Completion => "completion",
            Self::Sentence => "sentence",
            Self::ReverseLookup => "reverse_lookup",
            Self::History => "history",
            Self::Switch => "switch",
            Self::Unfold => "unfold",
            Self::Schema => "schema",
            Self::Ai { .. } => "ai",
        }
    }

    #[must_use]
    pub const fn is_ai(&self) -> bool {
        matches!(self, Self::Ai { .. })
    }

    #[must_use]
    pub const fn is_table_like(&self) -> bool {
        matches!(self, Self::Table | Self::PartialTable { .. })
    }

    #[must_use]
    pub const fn partial_consumed_len(&self) -> Option<usize> {
        match self {
            Self::PartialTable { consumed, .. } => Some(*consumed),
            _ => None,
        }
    }

    #[must_use]
    pub const fn recomposes_on_default(&self) -> bool {
        matches!(
            self,
            Self::PartialTable {
                recompose_on_default: true,
                ..
            }
        )
    }

    #[must_use]
    pub const fn ai_confidence(&self) -> Option<AiConfidence> {
        match self {
            Self::Ai { confidence, .. } => Some(*confidence),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AiConfidence {
    basis_points: u16,
}

impl AiConfidence {
    const MAX_BASIS_POINTS: u16 = 10_000;

    #[must_use]
    pub const fn from_basis_points(basis_points: u16) -> Self {
        Self {
            basis_points: if basis_points > Self::MAX_BASIS_POINTS {
                Self::MAX_BASIS_POINTS
            } else {
                basis_points
            },
        }
    }

    #[must_use]
    pub fn from_score(score: f32) -> Self {
        if !score.is_finite() || score <= 0.0 {
            return Self::from_basis_points(0);
        }
        if score >= 1.0 {
            return Self::from_basis_points(Self::MAX_BASIS_POINTS);
        }
        Self::from_basis_points((score * f32::from(Self::MAX_BASIS_POINTS)).round() as u16)
    }

    #[must_use]
    pub const fn basis_points(self) -> u16 {
        self.basis_points
    }

    #[must_use]
    pub fn as_score(self) -> f32 {
        f32::from(self.basis_points) / f32::from(Self::MAX_BASIS_POINTS)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PrivacyClass {
    #[default]
    Sensitive,
    Standard,
}

impl PrivacyClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sensitive => "sensitive",
            Self::Standard => "standard",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AiContext {
    pub app_id: Option<String>,
    pub field_id: Option<String>,
    pub preceding_text: Option<String>,
    pub privacy_class: PrivacyClass,
}

impl AiContext {
    #[must_use]
    pub fn standard() -> Self {
        Self {
            privacy_class: PrivacyClass::Standard,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn sensitive() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    #[must_use]
    pub fn with_field_id(mut self, field_id: impl Into<String>) -> Self {
        self.field_id = Some(field_id.into());
        self
    }

    #[must_use]
    pub fn with_preceding_text(mut self, preceding_text: impl Into<String>) -> Self {
        self.preceding_text = Some(preceding_text.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SegmentDebug {
    pub start: usize,
    pub end: usize,
    pub tag: String,
    pub source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterAuditRecord {
    pub name: String,
    pub before_count: usize,
    pub after_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpellingAlgebraDebug {
    pub translator: String,
    pub input: String,
    pub lookup_code: Option<String>,
    pub formulas: Vec<String>,
    pub expanded_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiStagingDebug {
    pub state: String,
    pub for_input: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngineInspectorSnapshot {
    pub segment_tags: Vec<String>,
    pub segments: Vec<SegmentDebug>,
    pub filter_pipeline: Vec<String>,
    pub filter_audit: Vec<FilterAuditRecord>,
    pub spelling_algebra: Vec<SpellingAlgebraDebug>,
    pub prediction_weight_threshold: Option<f32>,
    pub ai_staging: AiStagingDebug,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitRecord {
    pub candidate_type: String,
    pub text: String,
    pub input: String,
    pub segment_start: usize,
    pub segment_end: usize,
    pub tick: u64,
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Composition {
    pub input: String,
    pub caret: usize,
    pub preedit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub composition: Composition,
    pub segment_tags: Vec<String>,
    pub candidates: Vec<Candidate>,
    pub highlighted: usize,
    pub last_commit: Option<String>,
    pub commit_history: Vec<CommitRecord>,
    pub ai_context: AiContext,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            composition: Composition::default(),
            segment_tags: vec!["abc".to_owned()],
            candidates: Vec::new(),
            highlighted: 0,
            last_commit: None,
            commit_history: Vec::new(),
            ai_context: AiContext::default(),
        }
    }
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
