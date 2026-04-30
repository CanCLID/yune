use crate::{Candidate, CandidateSource};

const DECAY_WINDOW: f64 = 200.0;
const USER_PHRASE_BONUS: f32 = 30_000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct UserDbCommitMetadata {
    pub input: String,
    pub selected_text: String,
    pub candidate_type: String,
    pub candidate_source: CandidateSource,
    pub segment_start: usize,
    pub segment_end: usize,
    pub tick: u64,
}

impl UserDbCommitMetadata {
    #[must_use]
    pub fn new(
        input: impl Into<String>,
        selected_text: impl Into<String>,
        candidate_source: CandidateSource,
        segment_start: usize,
        segment_end: usize,
        tick: u64,
    ) -> Self {
        Self {
            input: input.into(),
            selected_text: selected_text.into(),
            candidate_type: candidate_source.as_str().to_owned(),
            candidate_source,
            segment_start,
            segment_end,
            tick,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserDbValue {
    pub commits: i32,
    pub dee: f64,
    pub tick: u64,
}

impl Default for UserDbValue {
    fn default() -> Self {
        Self {
            commits: 0,
            dee: 0.0,
            tick: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserDbLearnedEntry {
    pub code: String,
    pub text: String,
    pub value: UserDbValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserDbLookupRequest {
    pub input: String,
    pub predictive: bool,
    pub limit: usize,
}

impl UserDbLookupRequest {
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            predictive: false,
            limit: 0,
        }
    }

    #[must_use]
    pub const fn with_predictive(mut self, predictive: bool) -> Self {
        self.predictive = predictive;
        self
    }

    #[must_use]
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserDbLookupResult {
    pub code: String,
    pub text: String,
    pub comment: String,
    pub source: CandidateSource,
    pub quality: f32,
    pub value: UserDbValue,
}

impl UserDbLookupResult {
    #[must_use]
    pub fn candidate(&self) -> Candidate {
        Candidate {
            text: self.text.clone(),
            comment: self.comment.clone(),
            source: self.source.clone(),
            quality: self.quality,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserDbLearningUpdate {
    pub input: String,
    pub selected_text: String,
    pub candidate_type: String,
    pub value: UserDbValue,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UserDb {
    entries: Vec<UserDbLearnedEntry>,
}

impl UserDb {
    pub fn record_commit(&mut self, metadata: &UserDbCommitMetadata) -> UserDbLearningUpdate {
        let code = normalize_code(&metadata.input);
        let position = self
            .entries
            .iter()
            .position(|entry| entry.code == code && entry.text == metadata.selected_text);
        let index = match position {
            Some(index) => index,
            None => {
                self.entries.push(UserDbLearnedEntry {
                    code: code.clone(),
                    text: metadata.selected_text.clone(),
                    value: UserDbValue::default(),
                });
                self.entries.len() - 1
            }
        };
        let value = &mut self.entries[index].value;
        if value.commits < 0 {
            value.commits = -value.commits;
        }
        value.commits += 1;
        let next_tick = value.tick.max(metadata.tick).saturating_add(1);
        value.dee = formula_d(1.0, next_tick as f64, value.dee, value.tick as f64);
        value.tick = next_tick;
        UserDbLearningUpdate {
            input: metadata.input.clone(),
            selected_text: metadata.selected_text.clone(),
            candidate_type: metadata.candidate_type.clone(),
            value: value.clone(),
        }
    }

    pub fn learn_entry(
        &mut self,
        code: impl AsRef<str>,
        text: impl Into<String>,
        commits: i32,
        dee: f64,
        tick: u64,
    ) {
        self.entries.push(UserDbLearnedEntry {
            code: normalize_code(code.as_ref()),
            text: text.into(),
            value: UserDbValue { commits, dee, tick },
        });
    }

    #[must_use]
    pub fn entries(&self) -> &[UserDbLearnedEntry] {
        &self.entries
    }

    #[must_use]
    pub fn into_entries(self) -> Vec<UserDbLearnedEntry> {
        self.entries
    }

    #[must_use]
    pub fn lookup(&self, request: &UserDbLookupRequest) -> Vec<UserDbLookupResult> {
        let input_code = normalize_code(&request.input);
        let mut exact = Vec::new();
        let mut predictive = Vec::new();
        let present_tick = self
            .entries
            .iter()
            .map(|entry| entry.value.tick)
            .max()
            .unwrap_or(0)
            .saturating_add(1);

        for entry in &self.entries {
            if entry.value.commits < 0 {
                continue;
            }
            if entry.code.trim_end() == request.input {
                exact.push(lookup_result(entry, "", present_tick));
                continue;
            }
            if request.predictive && entry.code.starts_with(&input_code) {
                let remaining = entry.code[input_code.len()..].trim_end().to_owned();
                predictive.push(lookup_result(entry, &format!("~{remaining}"), present_tick));
            }
        }
        exact.sort_by(quality_order);
        predictive.sort_by(quality_order);
        exact.extend(predictive);
        if request.limit > 0 && exact.len() > request.limit {
            exact.truncate(request.limit);
        }
        exact
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackdatedScanPolicy {
    pub scans_commit_records: bool,
    pub scans_current_composition: bool,
    pub scans_history_translator: bool,
    pub scans_ai_ranker_memory: bool,
}

impl BackdatedScanPolicy {
    #[must_use]
    pub const fn normal_runtime_context_only() -> Self {
        Self {
            scans_commit_records: true,
            scans_current_composition: true,
            scans_history_translator: false,
            scans_ai_ranker_memory: false,
        }
    }

    #[must_use]
    pub fn scan_commit_event(&self, event: &UserDbCommitMetadata) -> UserDbCommitMetadata {
        event.clone()
    }
}

#[must_use]
pub fn formula_d(d: f64, t: f64, da: f64, ta: f64) -> f64 {
    d + da * ((ta - t) / DECAY_WINDOW).exp()
}

#[must_use]
pub fn formula_p(s: f64, u: f64, t: f64, d: f64) -> f64 {
    let k_m = 1.0 / (1.0 - (-0.005_f64).exp());
    let m = s - (s - u) * (1.0 - (-t / 10_000.0).exp()).powi(10);
    if d < 20.0 {
        m + (0.5 - m) * (d / k_m)
    } else {
        m + (1.0 - m) * (4.0_f64.powf(d / k_m) - 1.0) / 3.0
    }
}

#[must_use]
pub fn normalize_code(code: &str) -> String {
    let mut normalized = code.trim_end().to_owned();
    normalized.push(' ');
    normalized
}

fn lookup_result(
    entry: &UserDbLearnedEntry,
    comment: &str,
    present_tick: u64,
) -> UserDbLookupResult {
    let mut value = entry.value.clone();
    if value.tick < present_tick {
        value.dee = formula_d(0.0, present_tick as f64, value.dee, value.tick as f64);
    }
    let usage = if present_tick == 0 {
        0.0
    } else {
        f64::from(value.commits.max(0)) / present_tick as f64
    };
    let probability = formula_p(0.0, usage, present_tick as f64, value.dee).max(f64::EPSILON);
    UserDbLookupResult {
        code: entry.code.clone(),
        text: entry.text.clone(),
        comment: comment.to_owned(),
        source: CandidateSource::UserTable,
        quality: probability.ln() as f32 + 10.0 + USER_PHRASE_BONUS
            - (entry.code.len() as f32 * 0.5),
        value,
    }
}

fn quality_order(left: &UserDbLookupResult, right: &UserDbLookupResult) -> std::cmp::Ordering {
    right
        .quality
        .partial_cmp(&left.quality)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.text.len().cmp(&right.text.len()))
        .then_with(|| left.text.cmp(&right.text))
}

#[cfg(test)]
mod tests {
    use crate::{
        BackdatedScanPolicy, CandidateSource, Engine, StaticTableTranslator, UserDb,
        UserDbLookupRequest,
    };

    #[test]
    fn userdb_learning_commit_records_metadata_before_clear() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));
        engine.set_input("ni");

        assert_eq!(engine.commit_composition(), Some("你".to_owned()));
        assert!(engine.context().composition.input.is_empty());

        let event = engine
            .take_pending_userdb_learning()
            .expect("commit should expose a pending learning event");
        assert_eq!(event.input, "ni");
        assert_eq!(event.selected_text, "你");
        assert_eq!(event.candidate_type, "table");
        assert_eq!(event.candidate_source, CandidateSource::Table);
        assert_eq!(event.segment_start, 0);
        assert_eq!(event.segment_end, 2);
        assert_eq!(event.tick, 1);
    }

    #[test]
    fn userdb_learning_repeated_commits_increase_quality_and_emit_updates() {
        let mut db = UserDb::default();
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        for _ in 0..2 {
            engine.set_input("ni");
            assert_eq!(engine.commit_composition(), Some("你".to_owned()));
            let event = engine
                .take_pending_userdb_learning()
                .expect("commit should expose learning metadata");
            let update = db.record_commit(&event);
            assert_eq!(update.input, "ni");
            assert_eq!(update.selected_text, "你");
        }

        let request = UserDbLookupRequest::new("ni");
        let learned = db.lookup(&request);
        assert_eq!(learned[0].text, "你");
        assert_eq!(learned[0].value.commits, 2);
        assert!(learned[0].quality > 1.5);
    }

    #[test]
    fn predictive_userdb_lookup_returns_prefix_matches_before_optional_rankers() {
        let mut db = UserDb::default();
        db.learn_entry("ni hao", "你好", 2, 2.0, 2);
        db.learn_entry("ni", "你", 1, 1.0, 1);

        let predictive = db.lookup(&UserDbLookupRequest::new("ni").with_predictive(true));
        assert_eq!(predictive[0].text, "你");
        assert_eq!(predictive[0].source, CandidateSource::UserTable);
        assert_eq!(predictive[1].text, "你好");
        assert_eq!(predictive[1].comment, "~hao");

        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "尼")]).with_initial_quality(0.0));
        engine.set_userdb(db);
        engine.set_input("ni");

        assert_eq!(
            engine.context().candidates[0].source,
            CandidateSource::UserTable
        );
        assert_eq!(engine.context().candidates[0].text, "你");
    }

    #[test]
    fn backdated_scan_scope_is_explicit_and_excludes_history_or_ai_memory() {
        let policy = BackdatedScanPolicy::normal_runtime_context_only();
        assert!(policy.scans_commit_records);
        assert!(policy.scans_current_composition);
        assert!(!policy.scans_history_translator);
        assert!(!policy.scans_ai_ranker_memory);

        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("hao", "好")]));
        engine.set_input("hao");
        assert_eq!(engine.commit_composition(), Some("好".to_owned()));
        let event = engine
            .take_pending_userdb_learning()
            .expect("normal commit context should be scanable");
        let scanned = policy.scan_commit_event(&event);
        assert_eq!(scanned.input, "hao");
        assert_eq!(scanned.selected_text, "好");
    }
}
