use std::time::Duration;

use crate::{Candidate, CandidateSource, Context};

pub trait AiCandidateProvider {
    fn name(&self) -> &'static str;

    fn provide(&self, ctx: &Context, budget: Duration) -> AiResult;
}

#[derive(Clone, Debug, PartialEq)]
pub enum AiResult {
    Pending,
    Ready {
        for_input: String,
        candidates: Vec<Candidate>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct StagedAiCandidates {
    pub for_input: String,
    pub candidates: Vec<Candidate>,
}

impl StagedAiCandidates {
    #[must_use]
    pub fn matches_input(&self, input: &str) -> bool {
        self.for_input == input
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiDecision {
    Off,
    Pending,
    Ready,
}

impl AiDecision {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Pending => "pending",
            Self::Ready => "ready",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MockAiProvider;

impl AiCandidateProvider for MockAiProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn provide(&self, ctx: &Context, _budget: Duration) -> AiResult {
        let input = ctx.composition.input.as_str();
        let Some(text) = mock_suggestion(input) else {
            return AiResult::Pending;
        };

        AiResult::Ready {
            for_input: input.to_owned(),
            candidates: vec![Candidate {
                text: text.to_owned(),
                comment: "ai:mock 0.62".to_owned(),
                source: CandidateSource::Ai,
                quality: 0.0,
            }],
        }
    }
}

fn mock_suggestion(input: &str) -> Option<&'static str> {
    match input {
        "ni" => Some("你呀"),
        "hao" => Some("好呀"),
        "nihao" => Some("你好呀"),
        "ba" => Some("吧呀"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{AiCandidateProvider, AiResult, CandidateSource, Context, MockAiProvider};

    #[test]
    fn mock_ai_provider_returns_source_labeled_candidates_for_known_inputs() {
        let mut context = Context::default();
        context.composition.input = "nihao".to_owned();

        let result = MockAiProvider.provide(&context, Duration::from_millis(50));

        match result {
            AiResult::Ready {
                for_input,
                candidates,
            } => {
                assert_eq!(for_input, "nihao");
                assert_eq!(candidates.len(), 1);
                assert_eq!(candidates[0].text, "你好呀");
                assert_eq!(candidates[0].comment, "ai:mock 0.62");
                assert_eq!(candidates[0].source, CandidateSource::Ai);
            }
            AiResult::Pending => panic!("known mock input should produce a ready suggestion"),
        }
    }

    #[test]
    fn mock_ai_provider_is_pending_for_unknown_inputs() {
        let mut context = Context::default();
        context.composition.input = "unknown".to_owned();

        assert_eq!(
            MockAiProvider.provide(&context, Duration::from_millis(50)),
            AiResult::Pending
        );
    }
}
