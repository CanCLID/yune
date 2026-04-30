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
    UserTable,
    Completion,
    Sentence,
    ReverseLookup,
    History,
    Switch,
    Unfold,
    Schema,
    Ai,
}

impl CandidateSource {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Echo => "echo",
            Self::Punctuation => "punct",
            Self::Table => "table",
            Self::UserTable => "user_table",
            Self::Completion => "completion",
            Self::Sentence => "sentence",
            Self::ReverseLookup => "reverse_lookup",
            Self::History => "history",
            Self::Switch => "switch",
            Self::Unfold => "unfold",
            Self::Schema => "schema",
            Self::Ai => "ai",
        }
    }
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
