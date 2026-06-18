use std::time::Duration;

use crate::{
    AiCandidateProvider, AiConfidence, AiProviderKind, AiResult, Candidate, CandidateSource,
    Context, MemoryStore,
};

pub const LOCAL_MODEL_PROVIDER_NAME: &str = "local-model";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalModelRule {
    input: String,
    text: String,
    confidence: AiConfidence,
    preceding_suffix: Option<String>,
    app_id: Option<String>,
    field_id: Option<String>,
}

impl LocalModelRule {
    #[must_use]
    pub fn new(
        input: impl Into<String>,
        text: impl Into<String>,
        confidence: AiConfidence,
    ) -> Self {
        Self {
            input: input.into(),
            text: text.into(),
            confidence,
            preceding_suffix: None,
            app_id: None,
            field_id: None,
        }
    }

    #[must_use]
    pub fn with_preceding_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.preceding_suffix = Some(suffix.into());
        self
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
    pub fn input(&self) -> &str {
        &self.input
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn confidence(&self) -> AiConfidence {
        self.confidence
    }

    fn matches(&self, context: &Context) -> bool {
        self.input == context.composition.input
            && optional_context_matches(
                self.preceding_suffix.as_deref(),
                context.ai_context.preceding_text.as_deref(),
                ContextMatch::Suffix,
            )
            && optional_context_matches(
                self.app_id.as_deref(),
                context.ai_context.app_id.as_deref(),
                ContextMatch::Exact,
            )
            && optional_context_matches(
                self.field_id.as_deref(),
                context.ai_context.field_id.as_deref(),
                ContextMatch::Exact,
            )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalModelProvider {
    rules: Vec<LocalModelRule>,
    memory: MemoryStore,
}

impl LocalModelProvider {
    #[must_use]
    pub fn new(rules: impl IntoIterator<Item = LocalModelRule>) -> Self {
        Self {
            rules: rules.into_iter().collect(),
            memory: MemoryStore::default(),
        }
    }

    #[must_use]
    pub fn sample() -> Self {
        Self::new([
            LocalModelRule::new(
                "nihao",
                "\u{4f60}\u{597d}\u{5566}",
                AiConfidence::from_basis_points(8_200),
            ),
            LocalModelRule::new(
                "nihao",
                "\u{4f60}\u{597d}\u{5440}",
                AiConfidence::from_basis_points(6_400),
            ),
            LocalModelRule::new(
                "hao",
                "\u{597d}\u{5440}",
                AiConfidence::from_basis_points(7_100),
            )
            .with_preceding_suffix("\u{4f60}"),
        ])
    }

    #[must_use]
    pub fn with_memory(mut self, memory: MemoryStore) -> Self {
        self.memory = memory;
        self
    }

    #[must_use]
    pub fn rules(&self) -> &[LocalModelRule] {
        &self.rules
    }

    #[must_use]
    pub fn memory(&self) -> &MemoryStore {
        &self.memory
    }
}

impl Default for LocalModelProvider {
    fn default() -> Self {
        Self::sample()
    }
}

impl AiCandidateProvider for LocalModelProvider {
    fn name(&self) -> &'static str {
        LOCAL_MODEL_PROVIDER_NAME
    }

    fn kind(&self) -> AiProviderKind {
        AiProviderKind::Local
    }

    fn provide(&self, context: &Context, budget: Duration) -> AiResult {
        let input = context.composition.input.as_str();
        if input.is_empty() || budget.is_zero() {
            return AiResult::pending(input);
        }

        let mut candidates = Vec::new();
        for entry in self.memory.entries() {
            if entry.input == input
                && memory_context_matches(
                    entry.app_id.as_deref(),
                    context.ai_context.app_id.as_deref(),
                )
                && memory_context_matches(
                    entry.field_id.as_deref(),
                    context.ai_context.field_id.as_deref(),
                )
            {
                push_or_upgrade_candidate(
                    &mut candidates,
                    entry.selected_text.clone(),
                    boosted_memory_confidence(entry.confidence, entry.commits),
                );
            }
        }
        for rule in self.rules.iter().filter(|rule| rule.matches(context)) {
            push_or_upgrade_candidate(&mut candidates, rule.text.clone(), rule.confidence);
        }

        if candidates.is_empty() {
            AiResult::pending(input)
        } else {
            candidates.sort_by(ai_candidate_order);
            AiResult::Ready {
                for_input: input.to_owned(),
                candidates,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContextMatch {
    Exact,
    Suffix,
}

fn optional_context_matches(
    expected: Option<&str>,
    actual: Option<&str>,
    match_kind: ContextMatch,
) -> bool {
    let Some(expected) = expected else {
        return true;
    };
    match (actual, match_kind) {
        (Some(actual), ContextMatch::Exact) => actual == expected,
        (Some(actual), ContextMatch::Suffix) => actual.ends_with(expected),
        (None, _) => false,
    }
}

fn memory_context_matches(expected: Option<&str>, actual: Option<&str>) -> bool {
    expected.is_none() || expected == actual
}

fn boosted_memory_confidence(confidence: AiConfidence, commits: u32) -> AiConfidence {
    let boost = commits.min(10) as u16 * 100;
    AiConfidence::from_basis_points(confidence.basis_points().saturating_add(boost))
}

fn push_or_upgrade_candidate(
    candidates: &mut Vec<Candidate>,
    text: String,
    confidence: AiConfidence,
) {
    if let Some(candidate) = candidates
        .iter_mut()
        .find(|candidate| candidate.text == text)
    {
        if Some(confidence) > candidate.source.ai_confidence() {
            candidate.comment = local_model_comment(confidence);
            candidate.source = CandidateSource::ai(LOCAL_MODEL_PROVIDER_NAME, confidence);
            candidate.quality = confidence.as_score();
        }
        return;
    }
    candidates.push(Candidate {
        text,
        comment: local_model_comment(confidence),
        source: CandidateSource::ai(LOCAL_MODEL_PROVIDER_NAME, confidence),
        quality: confidence.as_score(),
    });
}

fn local_model_comment(confidence: AiConfidence) -> String {
    format!(
        "ai:{LOCAL_MODEL_PROVIDER_NAME} {:.2}",
        confidence.as_score()
    )
}

fn ai_candidate_order(left: &Candidate, right: &Candidate) -> std::cmp::Ordering {
    right
        .source
        .ai_confidence()
        .cmp(&left.source.ai_confidence())
        .then_with(|| left.text.cmp(&right.text))
}

#[cfg(test)]
mod tests {
    use crate::{
        AiCandidateProvider, AiConfidence, AiContext, AiResult, AiWorker, CandidateSource, Context,
        LocalModelProvider, LocalModelRule, MemoryStore, UserDbCommitMetadata,
    };

    use std::time::Duration;

    #[test]
    fn local_model_provider_returns_weighted_source_labeled_candidates() {
        let mut context = Context::default();
        context.composition.input = "nihao".to_owned();

        let result = LocalModelProvider::sample().provide(&context, Duration::from_millis(50));

        match result {
            AiResult::Ready {
                for_input,
                candidates,
            } => {
                assert_eq!(for_input, "nihao");
                assert_eq!(candidates.len(), 2);
                assert_eq!(candidates[0].text, "\u{4f60}\u{597d}\u{5566}");
                assert_eq!(candidates[0].comment, "ai:local-model 0.82");
                assert_eq!(
                    candidates[0].source,
                    CandidateSource::ai("local-model", AiConfidence::from_basis_points(8_200))
                );
                assert_eq!(candidates[1].text, "\u{4f60}\u{597d}\u{5440}");
            }
            AiResult::Pending { .. } | AiResult::Off { .. } => {
                panic!("sample local model should produce candidates")
            }
        }
    }

    #[test]
    fn local_model_provider_uses_contextual_preceding_text_rules() {
        let provider = LocalModelProvider::new([LocalModelRule::new(
            "hao",
            "\u{597d}\u{5440}",
            AiConfidence::from_basis_points(7_100),
        )
        .with_preceding_suffix("\u{4f60}")]);
        let mut context = Context::default();
        context.composition.input = "hao".to_owned();

        assert_eq!(
            provider.provide(&context, Duration::from_millis(50)),
            AiResult::pending("hao")
        );

        context.ai_context = AiContext::standard().with_preceding_text("\u{4f60}");
        match provider.provide(&context, Duration::from_millis(50)) {
            AiResult::Ready { candidates, .. } => {
                assert_eq!(candidates[0].text, "\u{597d}\u{5440}");
            }
            AiResult::Pending { .. } | AiResult::Off { .. } => {
                panic!("preceding-text rule should produce a candidate")
            }
        }
    }

    #[test]
    fn local_model_provider_uses_ai_memory_entries() {
        let mut memory = MemoryStore::default();
        let mut learning_context = Context {
            ai_context: AiContext::standard().with_app_id("sample_cli"),
            ..Context::default()
        };
        learning_context.composition.input = "nihao".to_owned();
        let event = UserDbCommitMetadata::new(
            "nihao",
            "\u{4f60}\u{597d}\u{5462}",
            CandidateSource::ai("local-model", AiConfidence::from_basis_points(8_000)),
            0,
            5,
            1,
        );
        memory.record_commit(&learning_context, &event);
        let provider = LocalModelProvider::new([]).with_memory(memory);
        let context = Context {
            composition: learning_context.composition.clone(),
            ai_context: AiContext::standard().with_app_id("sample_cli"),
            ..Context::default()
        };

        match provider.provide(&context, Duration::from_millis(50)) {
            AiResult::Ready { candidates, .. } => {
                assert_eq!(candidates[0].text, "\u{4f60}\u{597d}\u{5462}");
                assert_eq!(candidates[0].comment, "ai:local-model 0.81");
            }
            AiResult::Pending { .. } | AiResult::Off { .. } => {
                panic!("memory-backed local model should produce a candidate")
            }
        }
    }

    #[test]
    fn local_model_worker_is_allowed_in_sensitive_context() {
        let mut context = Context::default();
        context.composition.input = "nihao".to_owned();
        let worker = AiWorker::spawn(LocalModelProvider::sample(), Duration::from_millis(50));

        assert!(worker.request(&context));
        let result = worker
            .recv_matching_timeout("nihao", Duration::from_secs(1))
            .expect("local model should return a result");

        match result {
            AiResult::Ready { candidates, .. } => {
                assert_eq!(candidates[0].source.as_str(), "ai");
            }
            AiResult::Pending { .. } | AiResult::Off { .. } => {
                panic!("local providers should be allowed in sensitive contexts")
            }
        }
    }

    #[test]
    fn zero_budget_local_model_falls_back_to_pending() {
        let mut context = Context::default();
        context.composition.input = "nihao".to_owned();

        assert_eq!(
            LocalModelProvider::sample().provide(&context, Duration::ZERO),
            AiResult::pending("nihao")
        );
    }
}
