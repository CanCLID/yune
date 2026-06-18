use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{AiConfidence, Candidate, CandidateSource, Context};

pub trait AiCandidateProvider {
    fn name(&self) -> &'static str;

    fn provide(&self, ctx: &Context, budget: Duration) -> AiResult;
}

#[derive(Clone, Debug, PartialEq)]
pub enum AiResult {
    Pending {
        for_input: String,
    },
    Ready {
        for_input: String,
        candidates: Vec<Candidate>,
    },
}

impl AiResult {
    #[must_use]
    pub fn pending(for_input: impl Into<String>) -> Self {
        Self::Pending {
            for_input: for_input.into(),
        }
    }

    #[must_use]
    pub fn for_input(&self) -> &str {
        match self {
            Self::Pending { for_input } | Self::Ready { for_input, .. } => for_input,
        }
    }
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

enum AiWorkerRequest {
    Provide(Context),
    Shutdown,
}

pub struct AiWorker {
    request_tx: Sender<AiWorkerRequest>,
    result_rx: Receiver<AiResult>,
    handle: Option<JoinHandle<()>>,
}

impl AiWorker {
    #[must_use]
    pub fn spawn(provider: impl AiCandidateProvider + Send + 'static, budget: Duration) -> Self {
        let (request_tx, request_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = thread::spawn(move || worker_loop(provider, budget, request_rx, result_tx));
        Self {
            request_tx,
            result_rx,
            handle: Some(handle),
        }
    }

    pub fn request(&self, context: &Context) -> bool {
        self.request_tx
            .send(AiWorkerRequest::Provide(context.clone()))
            .is_ok()
    }

    #[must_use]
    pub fn try_recv_latest(&self) -> Option<AiResult> {
        let mut latest = None;
        loop {
            match self.result_rx.try_recv() {
                Ok(result) => latest = Some(result),
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return latest,
            }
        }
    }

    #[must_use]
    pub fn recv_matching_timeout(&self, input: &str, timeout: Duration) -> Option<AiResult> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(result) = self.try_recv_latest() {
                if result.for_input() == input {
                    return Some(result);
                }
            }

            let now = Instant::now();
            if now >= deadline {
                return None;
            }

            match self
                .result_rx
                .recv_timeout(deadline.saturating_duration_since(now))
            {
                Ok(result) if result.for_input() == input => return Some(result),
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return None,
            }
        }
    }
}

impl Drop for AiWorker {
    fn drop(&mut self) {
        let _ = self.request_tx.send(AiWorkerRequest::Shutdown);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn worker_loop(
    provider: impl AiCandidateProvider,
    budget: Duration,
    request_rx: Receiver<AiWorkerRequest>,
    result_tx: Sender<AiResult>,
) {
    while let Ok(request) = request_rx.recv() {
        let mut context = match request {
            AiWorkerRequest::Provide(context) => context,
            AiWorkerRequest::Shutdown => break,
        };
        while let Ok(next) = request_rx.try_recv() {
            match next {
                AiWorkerRequest::Provide(next_context) => context = next_context,
                AiWorkerRequest::Shutdown => return,
            }
        }
        if result_tx.send(provider.provide(&context, budget)).is_err() {
            break;
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
            return AiResult::pending(input);
        };

        AiResult::Ready {
            for_input: input.to_owned(),
            candidates: vec![Candidate {
                text: text.to_owned(),
                comment: "ai:mock 0.62".to_owned(),
                source: CandidateSource::ai("mock", AiConfidence::from_score(0.62)),
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

    use crate::{
        AiCandidateProvider, AiConfidence, AiResult, AiWorker, CandidateSource, Context,
        MockAiProvider,
    };

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
                assert_eq!(
                    candidates[0].source,
                    CandidateSource::ai("mock", AiConfidence::from_score(0.62))
                );
            }
            AiResult::Pending { .. } => {
                panic!("known mock input should produce a ready suggestion");
            }
        }
    }

    #[test]
    fn mock_ai_provider_is_pending_for_unknown_inputs() {
        let mut context = Context::default();
        context.composition.input = "unknown".to_owned();

        assert_eq!(
            MockAiProvider.provide(&context, Duration::from_millis(50)),
            AiResult::pending("unknown")
        );
    }

    #[test]
    fn ai_worker_returns_input_keyed_results_from_background_provider() {
        let mut context = Context::default();
        context.composition.input = "nihao".to_owned();
        let worker = AiWorker::spawn(MockAiProvider, Duration::from_millis(50));

        assert!(worker.request(&context));
        let result = worker
            .recv_matching_timeout("nihao", Duration::from_secs(1))
            .expect("mock worker should return a result");

        match result {
            AiResult::Ready {
                for_input,
                candidates,
            } => {
                assert_eq!(for_input, "nihao");
                assert_eq!(candidates[0].text, "你好呀");
            }
            AiResult::Pending { .. } => panic!("known input should be ready"),
        }
    }
}
