mod local_model;
mod memory;

use std::{
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{AiConfidence, Candidate, CandidateSource, Context, PrivacyClass, Status};

pub use local_model::{LocalModelProvider, LocalModelRule, LOCAL_MODEL_PROVIDER_NAME};
pub use memory::{
    memory_store_file_name, memory_store_snapshot_file_name, validate_memory_store_id,
    AiMemoryEntry, AiMemoryRecordResult, AiMemorySkipReason, AiMemorySnapshotError, MemoryStore,
    MEMORY_STORE_FILE_SUFFIX, MEMORY_STORE_SNAPSHOT_SUFFIX,
};

pub trait AiCandidateProvider {
    fn name(&self) -> &'static str;

    fn kind(&self) -> AiProviderKind {
        AiProviderKind::Local
    }

    fn provide(&self, ctx: &Context, budget: Duration) -> AiResult;
}

pub trait AiContextProvider {
    fn capture(&self, context: &Context, status: &Status) -> AiContextSnapshot;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiProviderKind {
    Mock,
    Local,
    Remote,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiContextSnapshot {
    pub app_id: Option<String>,
    pub field_id: Option<String>,
    pub preceding_text: Option<String>,
    pub privacy_class: PrivacyClass,
    pub input: String,
    pub cursor: usize,
    pub schema_id: String,
    pub schema_name: String,
    pub candidate_count: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EngineAiContextProvider;

impl AiContextProvider for EngineAiContextProvider {
    fn capture(&self, context: &Context, status: &Status) -> AiContextSnapshot {
        AiContextSnapshot {
            app_id: context.ai_context.app_id.clone(),
            field_id: context.ai_context.field_id.clone(),
            preceding_text: context.ai_context.preceding_text.clone(),
            privacy_class: context.ai_context.privacy_class,
            input: context.composition.input.clone(),
            cursor: context.composition.caret,
            schema_id: status.schema_id.clone(),
            schema_name: status.schema_name.clone(),
            candidate_count: context.candidates.len(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AiPrivacyPolicy;

impl AiPrivacyPolicy {
    #[must_use]
    pub fn allows_provider(self, context: &Context, provider_kind: AiProviderKind) -> bool {
        match provider_kind {
            AiProviderKind::Mock | AiProviderKind::Local => true,
            AiProviderKind::Remote => context.ai_context.privacy_class != PrivacyClass::Sensitive,
        }
    }

    #[must_use]
    pub fn allows_learning(self, context: &Context) -> bool {
        context.ai_context.privacy_class != PrivacyClass::Sensitive
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum AiResult {
    Off {
        for_input: String,
        reason: AiOffReason,
    },
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
    pub fn off(for_input: impl Into<String>, reason: AiOffReason) -> Self {
        Self::Off {
            for_input: for_input.into(),
            reason,
        }
    }

    #[must_use]
    pub fn pending(for_input: impl Into<String>) -> Self {
        Self::Pending {
            for_input: for_input.into(),
        }
    }

    #[must_use]
    pub fn for_input(&self) -> &str {
        match self {
            Self::Off { for_input, .. }
            | Self::Pending { for_input }
            | Self::Ready { for_input, .. } => for_input,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AiOffReason {
    Privacy,
}

impl AiOffReason {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Privacy => "privacy",
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
    Provide(Box<Context>),
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
        let provider_kind = provider.kind();
        let (request_tx, request_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            worker_loop(provider, provider_kind, budget, request_rx, result_tx);
        });
        Self {
            request_tx,
            result_rx,
            handle: Some(handle),
        }
    }

    pub fn request(&self, context: &Context) -> bool {
        self.request_tx
            .send(AiWorkerRequest::Provide(Box::new(context.clone())))
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
    provider_kind: AiProviderKind,
    budget: Duration,
    request_rx: Receiver<AiWorkerRequest>,
    result_tx: Sender<AiResult>,
) {
    while let Ok(request) = request_rx.recv() {
        let mut context = match request {
            AiWorkerRequest::Provide(context) => *context,
            AiWorkerRequest::Shutdown => break,
        };
        while let Ok(next) = request_rx.try_recv() {
            match next {
                AiWorkerRequest::Provide(next_context) => context = *next_context,
                AiWorkerRequest::Shutdown => return,
            }
        }
        if !AiPrivacyPolicy.allows_provider(&context, provider_kind) {
            let input = context.composition.input.clone();
            if result_tx
                .send(AiResult::off(input, AiOffReason::Privacy))
                .is_err()
            {
                break;
            }
            continue;
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

    fn kind(&self) -> AiProviderKind {
        AiProviderKind::Mock
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
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use crate::{
        AiCandidateProvider, AiConfidence, AiContext, AiContextProvider, AiOffReason,
        AiPrivacyPolicy, AiProviderKind, AiResult, AiWorker, Candidate, CandidateSource, Context,
        EngineAiContextProvider, MockAiProvider, PrivacyClass, Status,
    };

    #[derive(Clone, Debug)]
    struct RecordingRemoteProvider {
        calls: Arc<AtomicUsize>,
    }

    impl RecordingRemoteProvider {
        fn new() -> Self {
            Self {
                calls: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl AiCandidateProvider for RecordingRemoteProvider {
        fn name(&self) -> &'static str {
            "recording_remote"
        }

        fn kind(&self) -> AiProviderKind {
            AiProviderKind::Remote
        }

        fn provide(&self, ctx: &Context, _budget: Duration) -> AiResult {
            self.calls.fetch_add(1, Ordering::SeqCst);
            AiResult::Ready {
                for_input: ctx.composition.input.clone(),
                candidates: vec![Candidate {
                    text: "remote".to_owned(),
                    comment: "ai:remote 0.80".to_owned(),
                    source: CandidateSource::ai("remote", AiConfidence::from_score(0.80)),
                    quality: 0.0,
                }],
            }
        }
    }

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
            AiResult::Off { .. } => panic!("mock provider should not be privacy-blocked"),
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
            AiResult::Off { .. } => panic!("mock provider should not be privacy-blocked"),
        }
    }

    #[test]
    fn default_context_is_sensitive_and_blocks_remote_provider_calls() {
        let mut context = Context::default();
        context.composition.input = "nihao".to_owned();
        let provider = RecordingRemoteProvider::new();
        let calls = provider.clone();
        let worker = AiWorker::spawn(provider, Duration::from_millis(50));

        assert_eq!(context.ai_context.privacy_class, PrivacyClass::Sensitive);
        assert!(worker.request(&context));
        let result = worker
            .recv_matching_timeout("nihao", Duration::from_secs(1))
            .expect("privacy block should return a result");

        assert_eq!(calls.call_count(), 0);
        assert_eq!(result, AiResult::off("nihao", AiOffReason::Privacy));
    }

    #[test]
    fn standard_context_allows_remote_provider_calls() {
        let mut context = Context::default();
        context.composition.input = "nihao".to_owned();
        context.ai_context = AiContext::standard();
        let provider = RecordingRemoteProvider::new();
        let calls = provider.clone();
        let worker = AiWorker::spawn(provider, Duration::from_millis(50));

        assert!(worker.request(&context));
        let result = worker
            .recv_matching_timeout("nihao", Duration::from_secs(1))
            .expect("remote provider should return a result");

        assert_eq!(calls.call_count(), 1);
        match result {
            AiResult::Ready {
                for_input,
                candidates,
            } => {
                assert_eq!(for_input, "nihao");
                assert_eq!(candidates[0].source.as_str(), "ai");
            }
            AiResult::Pending { .. } | AiResult::Off { .. } => {
                panic!("standard context should allow remote provider")
            }
        }
    }

    #[test]
    fn privacy_policy_disables_learning_for_sensitive_contexts() {
        let sensitive = Context::default();
        let mut standard = Context::default();
        standard.ai_context = AiContext::standard();

        assert!(!AiPrivacyPolicy.allows_learning(&sensitive));
        assert!(AiPrivacyPolicy.allows_learning(&standard));
    }

    #[test]
    fn engine_context_provider_captures_explicit_shareable_fields() {
        let mut context = Context::default();
        context.composition.input = "nihao".to_owned();
        context.composition.caret = 5;
        context.ai_context = AiContext::standard()
            .with_app_id("sample_cli")
            .with_field_id("demo")
            .with_preceding_text("hello ");
        context.candidates.push(Candidate {
            text: "你好".to_owned(),
            comment: "nihao".to_owned(),
            source: CandidateSource::Table,
            quality: 1.0,
        });
        let status = Status {
            schema_id: "sample".to_owned(),
            schema_name: "Sample".to_owned(),
            ..Status::default()
        };

        let snapshot = EngineAiContextProvider.capture(&context, &status);

        assert_eq!(snapshot.app_id.as_deref(), Some("sample_cli"));
        assert_eq!(snapshot.field_id.as_deref(), Some("demo"));
        assert_eq!(snapshot.preceding_text.as_deref(), Some("hello "));
        assert_eq!(snapshot.privacy_class, PrivacyClass::Standard);
        assert_eq!(snapshot.input, "nihao");
        assert_eq!(snapshot.cursor, 5);
        assert_eq!(snapshot.schema_id, "sample");
        assert_eq!(snapshot.schema_name, "Sample");
        assert_eq!(snapshot.candidate_count, 1);
    }
}
