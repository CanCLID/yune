use std::{fmt::Write as _, mem, os::raw::c_int, ptr};

use yune_rime_api::{
    Bool, RimeCommit, RimeComposition, RimeContext, RimeMenu, RimeStatus, RimeTraits, FALSE,
};

pub(crate) mod native;

pub(crate) const BASELINE_TRACE_FIXTURE: &str =
    include_str!("../../../../fixtures/frontend-traces/native-host-lifecycle.json");
pub(crate) const NATIVE_TARGET: &str = "cargo_cdylib_dynamic_loader";
pub(crate) const NATIVE_SCENARIO: &str = "native_host_lifecycle";
pub(crate) const LOGICAL_SCHEMA_ID: &str = "dynamic_schema";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrontendHostTrace {
    pub(crate) target: String,
    pub(crate) scenario: String,
    pub(crate) resource_ids: Vec<String>,
    pub(crate) required_functions: Vec<FunctionAvailability>,
    pub(crate) calls: Vec<TraceCall>,
    pub(crate) notifications: Vec<TraceNotification>,
    pub(crate) free_pairs: Vec<FreePairObservation>,
    pub(crate) stale_sessions: Vec<StaleSessionObservation>,
    pub(crate) mismatch: MismatchRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionAvailability {
    pub(crate) name: String,
    pub(crate) available: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TraceCall {
    pub(crate) name: String,
    pub(crate) result: TraceValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TraceValue {
    Bool(bool),
    Number(i64),
    Text(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TraceNotification {
    pub(crate) handler: String,
    pub(crate) session: String,
    pub(crate) message_type: String,
    pub(crate) message_value: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FreePairObservation {
    pub(crate) get_call: String,
    pub(crate) free_call: String,
    pub(crate) same_object: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StaleSessionObservation {
    pub(crate) after: String,
    pub(crate) operation: String,
    pub(crate) accepted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MismatchRecord {
    pub(crate) expected_behavior: String,
    pub(crate) observed_behavior: String,
    pub(crate) classification: MismatchClassification,
    pub(crate) reproduction_status: ReproductionStatus,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MismatchClassification {
    Match,
    Mismatch,
    Blocker,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReproductionStatus {
    Reproduced,
    RunnableReproduction,
    MinimizedFixture,
    DocumentedBlocker,
}

impl FrontendHostTrace {
    pub(crate) fn new(target: &str, scenario: &str) -> Self {
        Self {
            target: target.to_owned(),
            scenario: scenario.to_owned(),
            resource_ids: vec![LOGICAL_SCHEMA_ID.to_owned()],
            required_functions: Vec::new(),
            calls: Vec::new(),
            notifications: Vec::new(),
            free_pairs: Vec::new(),
            stale_sessions: Vec::new(),
            mismatch: MismatchRecord {
                expected_behavior: "host-shaped native lifecycle matches required RIME ABI surface".to_owned(),
                observed_behavior: "all required native lifecycle calls completed through the dynamic RimeApi table".to_owned(),
                classification: MismatchClassification::Match,
                reproduction_status: ReproductionStatus::MinimizedFixture,
            },
        }
    }

    pub(crate) fn record_function(&mut self, name: &str, available: bool) {
        self.required_functions.push(FunctionAvailability {
            name: name.to_owned(),
            available,
        });
        if !available {
            self.mismatch = MismatchRecord {
                expected_behavior: format!("RimeApi exposes required function pointer {name}"),
                observed_behavior: format!(
                    "RimeApi returned null required function pointer {name}"
                ),
                classification: MismatchClassification::Blocker,
                reproduction_status: ReproductionStatus::DocumentedBlocker,
            };
        }
    }

    pub(crate) fn call_bool(&mut self, name: &str, value: Bool) {
        self.calls.push(TraceCall {
            name: name.to_owned(),
            result: TraceValue::Bool(value != FALSE),
        });
    }

    pub(crate) fn call_number(&mut self, name: &str, value: impl Into<i64>) {
        self.calls.push(TraceCall {
            name: name.to_owned(),
            result: TraceValue::Number(value.into()),
        });
    }

    pub(crate) fn call_text(&mut self, name: &str, value: impl Into<String>) {
        self.calls.push(TraceCall {
            name: name.to_owned(),
            result: TraceValue::Text(value.into()),
        });
    }

    pub(crate) fn record_notification(
        &mut self,
        handler: &str,
        session: &str,
        message_type: &str,
        message_value: &str,
    ) {
        self.notifications.push(TraceNotification {
            handler: handler.to_owned(),
            session: session.to_owned(),
            message_type: message_type.to_owned(),
            message_value: message_value.to_owned(),
        });
    }

    pub(crate) fn record_free_pair(&mut self, get_call: &str, free_call: &str, same_object: bool) {
        self.free_pairs.push(FreePairObservation {
            get_call: get_call.to_owned(),
            free_call: free_call.to_owned(),
            same_object,
        });
    }

    pub(crate) fn record_stale_session(&mut self, after: &str, operation: &str, accepted: bool) {
        self.stale_sessions.push(StaleSessionObservation {
            after: after.to_owned(),
            operation: operation.to_owned(),
            accepted,
        });
    }

    pub(crate) fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        push_field(&mut json, 1, "target", &json_string(&self.target), true);
        push_field(&mut json, 1, "scenario", &json_string(&self.scenario), true);
        push_field(
            &mut json,
            1,
            "resource_ids",
            &json_string_array(&self.resource_ids),
            true,
        );
        push_field(
            &mut json,
            1,
            "required_functions",
            &required_functions_json(&self.required_functions, 1),
            true,
        );
        push_field(
            &mut json,
            1,
            "ordered_calls",
            &calls_json(&self.calls, 1),
            true,
        );
        push_field(
            &mut json,
            1,
            "notifications",
            &notifications_json(&self.notifications, 1),
            true,
        );
        push_field(
            &mut json,
            1,
            "free_pairs",
            &free_pairs_json(&self.free_pairs, 1),
            true,
        );
        push_field(
            &mut json,
            1,
            "stale_sessions",
            &stale_sessions_json(&self.stale_sessions, 1),
            true,
        );
        push_field(
            &mut json,
            1,
            "mismatch",
            &mismatch_json(&self.mismatch, 1),
            false,
        );
        json.push_str("}\n");
        json
    }

    pub(crate) fn assert_sanitized(&self) {
        assert_json_is_sanitized(&self.to_json());
        for resource_id in &self.resource_ids {
            assert_logical_resource_id(resource_id);
        }
    }
}

pub(crate) fn required_function<T>(
    trace: &mut FrontendHostTrace,
    name: &str,
    function: Option<T>,
) -> Result<T, HostValidationBlocker> {
    trace.record_function(name, function.is_some());
    function.ok_or_else(|| HostValidationBlocker::missing_function(name))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HostValidationBlocker {
    pub(crate) mismatch: MismatchRecord,
}

impl HostValidationBlocker {
    fn missing_function(name: &str) -> Self {
        Self {
            mismatch: MismatchRecord {
                expected_behavior: format!("RimeApi exposes required function pointer {name}"),
                observed_behavior: format!(
                    "RimeApi returned null required function pointer {name}"
                ),
                classification: MismatchClassification::Blocker,
                reproduction_status: ReproductionStatus::DocumentedBlocker,
            },
        }
    }
}

pub(crate) fn empty_traits() -> RimeTraits {
    RimeTraits {
        data_size: mem::size_of::<RimeTraits>() as c_int,
        shared_data_dir: ptr::null(),
        user_data_dir: ptr::null(),
        distribution_name: ptr::null(),
        distribution_code_name: ptr::null(),
        distribution_version: ptr::null(),
        app_name: ptr::null(),
        modules: ptr::null(),
        min_log_level: 0,
        log_dir: ptr::null(),
        prebuilt_data_dir: ptr::null(),
        staging_dir: ptr::null(),
    }
}

pub(crate) fn empty_context() -> RimeContext {
    RimeContext {
        data_size: (mem::size_of::<RimeContext>() - mem::size_of::<c_int>()) as c_int,
        composition: RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: ptr::null_mut(),
        },
        menu: RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        },
        commit_text_preview: ptr::null_mut(),
        select_labels: ptr::null_mut(),
    }
}

pub(crate) fn empty_status() -> RimeStatus {
    RimeStatus {
        data_size: (mem::size_of::<RimeStatus>() - mem::size_of::<c_int>()) as c_int,
        schema_id: ptr::null_mut(),
        schema_name: ptr::null_mut(),
        is_disabled: FALSE,
        is_composing: FALSE,
        is_ascii_mode: FALSE,
        is_full_shape: FALSE,
        is_simplified: FALSE,
        is_traditional: FALSE,
        is_ascii_punct: FALSE,
    }
}

pub(crate) fn empty_commit() -> RimeCommit {
    RimeCommit {
        data_size: (mem::size_of::<RimeCommit>() - mem::size_of::<c_int>()) as c_int,
        text: ptr::null_mut(),
    }
}

#[allow(dead_code)]
pub(crate) fn assert_baseline_fixture_is_sanitized() {
    assert_json_is_sanitized(BASELINE_TRACE_FIXTURE);
    assert!(BASELINE_TRACE_FIXTURE.contains("\"target\": \"cargo_cdylib_dynamic_loader\""));
    assert!(BASELINE_TRACE_FIXTURE.contains("\"scenario\": \"native_host_lifecycle\""));
    assert!(BASELINE_TRACE_FIXTURE.contains("\"classification\": \"match\""));
    assert!(BASELINE_TRACE_FIXTURE.contains("\"reproduction_status\": \"minimized_fixture\""));
}

pub(crate) fn assert_json_is_sanitized(json: &str) {
    for forbidden in [
        "/tmp/",
        "/var/",
        "target/debug",
        "target/release",
        "0x",
        "timestamp",
        "duration",
        "process_id",
        "CARGO_",
        "Users/",
        "\\\\",
    ] {
        assert!(
            !json.contains(forbidden),
            "frontend host trace fixture contains environment-dependent token {forbidden:?}"
        );
    }
}

fn assert_logical_resource_id(resource_id: &str) {
    assert!(!resource_id.is_empty());
    assert!(!resource_id.starts_with('/'));
    assert!(!resource_id.contains('/'));
    assert!(!resource_id.contains('\\'));
    assert!(!resource_id.contains(".."));
    assert!(!resource_id.contains(':'));
}

fn required_functions_json(functions: &[FunctionAvailability], depth: usize) -> String {
    if functions.is_empty() {
        return "[]".to_owned();
    }
    let mut json = String::new();
    json.push_str("[\n");
    for (index, function) in functions.iter().enumerate() {
        push_indent(&mut json, depth + 1);
        json.push_str("{\n");
        push_field(
            &mut json,
            depth + 2,
            "name",
            &json_string(&function.name),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "available",
            &function.available.to_string(),
            false,
        );
        push_indent(&mut json, depth + 1);
        json.push('}');
        if index + 1 != functions.len() {
            json.push(',');
        }
        json.push('\n');
    }
    push_indent(&mut json, depth);
    json.push(']');
    json
}

fn calls_json(calls: &[TraceCall], depth: usize) -> String {
    if calls.is_empty() {
        return "[]".to_owned();
    }
    let mut json = String::new();
    json.push_str("[\n");
    for (index, call) in calls.iter().enumerate() {
        push_indent(&mut json, depth + 1);
        json.push_str("{\n");
        push_field(&mut json, depth + 2, "name", &json_string(&call.name), true);
        push_field(
            &mut json,
            depth + 2,
            "result",
            &trace_value_json(&call.result),
            false,
        );
        push_indent(&mut json, depth + 1);
        json.push('}');
        if index + 1 != calls.len() {
            json.push(',');
        }
        json.push('\n');
    }
    push_indent(&mut json, depth);
    json.push(']');
    json
}

fn notifications_json(notifications: &[TraceNotification], depth: usize) -> String {
    if notifications.is_empty() {
        return "[]".to_owned();
    }
    let mut json = String::new();
    json.push_str("[\n");
    for (index, event) in notifications.iter().enumerate() {
        push_indent(&mut json, depth + 1);
        json.push_str("{\n");
        push_field(
            &mut json,
            depth + 2,
            "handler",
            &json_string(&event.handler),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "session",
            &json_string(&event.session),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "message_type",
            &json_string(&event.message_type),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "message_value",
            &json_string(&event.message_value),
            false,
        );
        push_indent(&mut json, depth + 1);
        json.push('}');
        if index + 1 != notifications.len() {
            json.push(',');
        }
        json.push('\n');
    }
    push_indent(&mut json, depth);
    json.push(']');
    json
}

fn free_pairs_json(pairs: &[FreePairObservation], depth: usize) -> String {
    if pairs.is_empty() {
        return "[]".to_owned();
    }
    let mut json = String::new();
    json.push_str("[\n");
    for (index, pair) in pairs.iter().enumerate() {
        push_indent(&mut json, depth + 1);
        json.push_str("{\n");
        push_field(
            &mut json,
            depth + 2,
            "get_call",
            &json_string(&pair.get_call),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "free_call",
            &json_string(&pair.free_call),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "same_object",
            &pair.same_object.to_string(),
            false,
        );
        push_indent(&mut json, depth + 1);
        json.push('}');
        if index + 1 != pairs.len() {
            json.push(',');
        }
        json.push('\n');
    }
    push_indent(&mut json, depth);
    json.push(']');
    json
}

fn stale_sessions_json(stale_sessions: &[StaleSessionObservation], depth: usize) -> String {
    if stale_sessions.is_empty() {
        return "[]".to_owned();
    }
    let mut json = String::new();
    json.push_str("[\n");
    for (index, stale) in stale_sessions.iter().enumerate() {
        push_indent(&mut json, depth + 1);
        json.push_str("{\n");
        push_field(
            &mut json,
            depth + 2,
            "after",
            &json_string(&stale.after),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "operation",
            &json_string(&stale.operation),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "accepted",
            &stale.accepted.to_string(),
            false,
        );
        push_indent(&mut json, depth + 1);
        json.push('}');
        if index + 1 != stale_sessions.len() {
            json.push(',');
        }
        json.push('\n');
    }
    push_indent(&mut json, depth);
    json.push(']');
    json
}

fn mismatch_json(mismatch: &MismatchRecord, depth: usize) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    push_field(
        &mut json,
        depth + 1,
        "expected_behavior",
        &json_string(&mismatch.expected_behavior),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "observed_behavior",
        &json_string(&mismatch.observed_behavior),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "classification",
        &json_string(mismatch.classification.as_str()),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "reproduction_status",
        &json_string(mismatch.reproduction_status.as_str()),
        false,
    );
    push_indent(&mut json, depth);
    json.push('}');
    json
}

impl MismatchClassification {
    fn as_str(self) -> &'static str {
        match self {
            Self::Match => "match",
            Self::Mismatch => "mismatch",
            Self::Blocker => "blocker",
        }
    }
}

impl ReproductionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Reproduced => "reproduced",
            Self::RunnableReproduction => "runnable_reproduction",
            Self::MinimizedFixture => "minimized_fixture",
            Self::DocumentedBlocker => "documented_blocker",
        }
    }
}

fn trace_value_json(value: &TraceValue) -> String {
    match value {
        TraceValue::Bool(value) => value.to_string(),
        TraceValue::Number(value) => value.to_string(),
        TraceValue::Text(value) => json_string(value),
    }
}

fn json_string_array(values: &[String]) -> String {
    if values.is_empty() {
        return "[]".to_owned();
    }
    let mut json = String::new();
    json.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            json.push_str(", ");
        }
        json.push_str(&json_string(value));
    }
    json.push(']');
    json
}

fn json_string(value: &str) -> String {
    let mut out = String::new();
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                write!(&mut out, "\\u{:04x}", ch as u32).expect("write to String cannot fail");
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn push_field(json: &mut String, depth: usize, key: &str, value: &str, comma: bool) {
    push_indent(json, depth);
    write!(json, "\"{key}\": {value}").expect("write to String cannot fail");
    if comma {
        json.push(',');
    }
    json.push('\n');
}

fn push_indent(json: &mut String, depth: usize) {
    for _ in 0..depth {
        json.push_str("  ");
    }
}
