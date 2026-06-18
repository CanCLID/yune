use std::time::Duration;

use yune_core::{
    parse_key_sequence, AiCandidateProvider, AiDecision, Engine, MockAiProvider,
    PunctuationTranslator, StaticTableTranslator,
};

use crate::args::AiProviderMode;
use crate::transcript::FixtureOutput;

pub(crate) const DEFAULT_SEQUENCE: &str = "nihao ";

const SAMPLE_DICT: &str = r#"
---
name: sample
version: "0.1"
sort: by_weight
...

你	ni	10
好	hao	10
你好	ni hao	100
"#;

const AI_PROVIDER_BUDGET: Duration = Duration::from_millis(50);

pub(crate) fn run_sequence(sequence: &str) -> Result<FixtureOutput, String> {
    run_sequence_with_ai_provider(sequence, AiProviderMode::None)
}

pub(crate) fn run_sequence_with_ai_provider(
    sequence: &str,
    ai_provider: AiProviderMode,
) -> Result<FixtureOutput, String> {
    let mut engine = Engine::new();
    engine.set_schema("sample", "Sample");
    engine.add_translator(PunctuationTranslator::default_half_shape());
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(SAMPLE_DICT)
            .map_err(|error| format!("invalid sample dictionary: {error}"))?,
    );
    let provider = (ai_provider == AiProviderMode::Mock).then_some(MockAiProvider);
    let mut ai_decision = provider.map(|_| AiDecision::Pending);
    let mut commits = Vec::new();
    for key_event in
        parse_key_sequence(sequence).map_err(|error| format!("invalid key sequence: {error}"))?
    {
        if let Some(commit) = engine.process_key_event(key_event) {
            commits.push(commit);
        }
        if let Some(provider) = provider {
            ai_decision = Some(
                engine.stage_ai_result(provider.provide(engine.context(), AI_PROVIDER_BUDGET)),
            );
        }
    }

    Ok(FixtureOutput {
        schema_id: "sample".to_owned(),
        sequence: sequence.to_owned(),
        commits,
        snapshot: engine.snapshot(),
        ai_decision,
    })
}

#[cfg(test)]
mod tests {
    use crate::args::AiProviderMode;

    use super::{run_sequence, run_sequence_with_ai_provider};

    #[test]
    fn default_run_omits_ai_decision_and_keeps_classic_output() {
        let output = run_sequence("ni").expect("sample sequence should run");
        let json = output.to_json();

        assert_eq!(output.ai_decision, None);
        assert!(!json.contains("\"ai_decision\""));
        assert_eq!(output.snapshot.context.candidates[0].text, "你");
        assert_eq!(
            output.snapshot.context.candidates[0].source.as_str(),
            "table"
        );
    }

    #[test]
    fn mock_provider_appends_source_labeled_ai_candidate() {
        let output = run_sequence_with_ai_provider("nihao", AiProviderMode::Mock)
            .expect("sample sequence should run");
        let candidates = &output.snapshot.context.candidates;
        let ai_candidate = candidates
            .last()
            .expect("mock AI candidate should be appended");

        assert_eq!(
            output.ai_decision.map(|decision| decision.as_str()),
            Some("ready")
        );
        assert_eq!(candidates[0].text, "你好");
        assert_eq!(candidates[0].source.as_str(), "table");
        assert_eq!(ai_candidate.text, "你好呀");
        assert_eq!(ai_candidate.comment, "ai:mock 0.62");
        assert_eq!(ai_candidate.source.as_str(), "ai");
        assert!(output.to_json().contains("\"ai_decision\": \"ready\""));
    }

    #[test]
    fn pending_mock_provider_preserves_classic_candidates() {
        let baseline = run_sequence("zz").expect("sample sequence should run");
        let pending = run_sequence_with_ai_provider("zz", AiProviderMode::Mock)
            .expect("sample sequence should run");

        assert_eq!(
            pending.ai_decision.map(|decision| decision.as_str()),
            Some("pending")
        );
        assert_eq!(
            pending.snapshot.context.candidates,
            baseline.snapshot.context.candidates
        );
        assert!(pending.to_json().contains("\"ai_decision\": \"pending\""));
    }
}
