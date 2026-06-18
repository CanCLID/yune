use crate::rime_frontend::{
    FrontendCandidate, FrontendContext, FrontendEvent, FrontendRun, FrontendStatus,
};
use yune_core::{AiDecision, Candidate, Context, Snapshot, Status};

// Owns deterministic frontend transcript comparison against librime-visible
// per-key state; no paths, timestamps, process IDs, or native frontend claims.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FixtureOutput {
    pub(crate) schema_id: String,
    pub(crate) sequence: String,
    pub(crate) commits: Vec<String>,
    pub(crate) snapshot: Snapshot,
    pub(crate) ai_decision: Option<AiDecision>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrontendTranscript<'run> {
    run: &'run FrontendRun,
}

impl<'run> FrontendTranscript<'run> {
    pub(crate) fn new(run: &'run FrontendRun) -> Self {
        Self { run }
    }

    pub(crate) fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        push_field(
            &mut json,
            1,
            "schema_id",
            &json_string(&self.run.schema_id),
            true,
        );
        push_field(
            &mut json,
            1,
            "sequence",
            &json_string(&self.run.sequence),
            true,
        );
        push_field(
            &mut json,
            1,
            "events",
            &frontend_events_json(&self.run.events, 1),
            true,
        );
        push_field(
            &mut json,
            1,
            "commits",
            &json_string_array(&self.run.commits),
            true,
        );
        push_field(
            &mut json,
            1,
            "context",
            &frontend_context_json(&self.run.context, 1),
            true,
        );
        push_field(
            &mut json,
            1,
            "status",
            &frontend_status_json(&self.run.status, 1),
            false,
        );
        json.push_str("}\n");
        json
    }
}

impl FixtureOutput {
    pub(crate) fn to_json(&self) -> String {
        let mut json = String::new();
        json.push_str("{\n");
        push_field(
            &mut json,
            1,
            "schema_id",
            &json_string(&self.schema_id),
            true,
        );
        push_field(&mut json, 1, "sequence", &json_string(&self.sequence), true);
        push_field(
            &mut json,
            1,
            "commits",
            &json_string_array(&self.commits),
            true,
        );
        if let Some(ai_decision) = self.ai_decision {
            push_field(
                &mut json,
                1,
                "ai_decision",
                &json_string(ai_decision.as_str()),
                true,
            );
        }
        push_field(
            &mut json,
            1,
            "context",
            &context_json(&self.snapshot.context, 1),
            true,
        );
        push_field(
            &mut json,
            1,
            "status",
            &status_json(&self.snapshot.status, 1),
            false,
        );
        json.push_str("}\n");
        json
    }
}

fn context_json(context: &Context, depth: usize) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    push_field(
        &mut json,
        depth + 1,
        "input",
        &json_string(&context.composition.input),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "caret",
        &context.composition.caret.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "preedit",
        &json_string(&context.composition.preedit),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "highlighted",
        &context.highlighted.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "last_commit",
        &optional_string_json(context.last_commit.as_deref()),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "candidates",
        &candidates_json(&context.candidates, depth + 1),
        false,
    );
    push_indent(&mut json, depth);
    json.push('}');
    json
}

fn candidates_json(candidates: &[Candidate], depth: usize) -> String {
    if candidates.is_empty() {
        return "[]".to_owned();
    }

    let mut json = String::new();
    json.push_str("[\n");
    for (index, candidate) in candidates.iter().enumerate() {
        push_indent(&mut json, depth + 1);
        json.push_str("{\n");
        push_field(
            &mut json,
            depth + 2,
            "text",
            &json_string(&candidate.text),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "comment",
            &json_string(&candidate.comment),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "source",
            &json_string(candidate.source.as_str()),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "quality",
            &candidate.quality.to_string(),
            false,
        );
        push_indent(&mut json, depth + 1);
        json.push('}');
        if index + 1 != candidates.len() {
            json.push(',');
        }
        json.push('\n');
    }
    push_indent(&mut json, depth);
    json.push(']');
    json
}

fn status_json(status: &Status, depth: usize) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    push_field(
        &mut json,
        depth + 1,
        "schema_id",
        &json_string(&status.schema_id),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "schema_name",
        &json_string(&status.schema_name),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_disabled",
        &status.is_disabled.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_composing",
        &status.is_composing.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_ascii_mode",
        &status.is_ascii_mode.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_full_shape",
        &status.is_full_shape.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_simplified",
        &status.is_simplified.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_traditional",
        &status.is_traditional.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_ascii_punct",
        &status.is_ascii_punct.to_string(),
        false,
    );
    push_indent(&mut json, depth);
    json.push('}');
    json
}

fn frontend_events_json(events: &[FrontendEvent], depth: usize) -> String {
    if events.is_empty() {
        return "[]".to_owned();
    }

    let mut json = String::new();
    json.push_str("[\n");
    for (index, event) in events.iter().enumerate() {
        push_indent(&mut json, depth + 1);
        json.push_str("{\n");
        push_field(&mut json, depth + 2, "index", &index.to_string(), true);
        push_field(&mut json, depth + 2, "key", &json_string(&event.key), true);
        push_field(
            &mut json,
            depth + 2,
            "keycode",
            &event.keycode.to_string(),
            true,
        );
        push_field(&mut json, depth + 2, "mask", &event.mask.to_string(), true);
        push_field(
            &mut json,
            depth + 2,
            "handled",
            &event.handled.to_string(),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "commits",
            &json_string_array(&event.commits),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "context",
            &frontend_context_json(&event.context, depth + 2),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "status",
            &frontend_status_json(&event.status, depth + 2),
            false,
        );
        push_indent(&mut json, depth + 1);
        json.push('}');
        if index + 1 != events.len() {
            json.push(',');
        }
        json.push('\n');
    }
    push_indent(&mut json, depth);
    json.push(']');
    json
}

fn frontend_context_json(context: &FrontendContext, depth: usize) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    push_field(
        &mut json,
        depth + 1,
        "input",
        &json_string(&context.input),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "caret",
        &context.caret.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "preedit",
        &json_string(&context.preedit),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "highlighted",
        &context.highlighted.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "last_commit",
        &optional_string_json(context.last_commit.as_deref()),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "page_size",
        &context.page_size.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "page_no",
        &context.page_no.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_last_page",
        &context.is_last_page.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "select_keys",
        &optional_string_json(context.select_keys.as_deref()),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "select_labels",
        &json_string_array(&context.select_labels),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "candidates",
        &frontend_candidates_json(&context.candidates, depth + 1),
        false,
    );
    push_indent(&mut json, depth);
    json.push('}');
    json
}

fn frontend_candidates_json(candidates: &[FrontendCandidate], depth: usize) -> String {
    if candidates.is_empty() {
        return "[]".to_owned();
    }

    let mut json = String::new();
    json.push_str("[\n");
    for (index, candidate) in candidates.iter().enumerate() {
        push_indent(&mut json, depth + 1);
        json.push_str("{\n");
        push_field(
            &mut json,
            depth + 2,
            "text",
            &json_string(&candidate.text),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "comment",
            &json_string(&candidate.comment),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "source",
            &json_string(&candidate.source),
            true,
        );
        push_field(
            &mut json,
            depth + 2,
            "quality",
            &candidate.quality.to_string(),
            false,
        );
        push_indent(&mut json, depth + 1);
        json.push('}');
        if index + 1 != candidates.len() {
            json.push(',');
        }
        json.push('\n');
    }
    push_indent(&mut json, depth);
    json.push(']');
    json
}

fn frontend_status_json(status: &FrontendStatus, depth: usize) -> String {
    let mut json = String::new();
    json.push_str("{\n");
    push_field(
        &mut json,
        depth + 1,
        "schema_id",
        &json_string(&status.schema_id),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "schema_name",
        &json_string(&status.schema_name),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_disabled",
        &status.is_disabled.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_composing",
        &status.is_composing.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_ascii_mode",
        &status.is_ascii_mode.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_full_shape",
        &status.is_full_shape.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_simplified",
        &status.is_simplified.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_traditional",
        &status.is_traditional.to_string(),
        true,
    );
    push_field(
        &mut json,
        depth + 1,
        "is_ascii_punct",
        &status.is_ascii_punct.to_string(),
        false,
    );
    push_indent(&mut json, depth);
    json.push('}');
    json
}

fn push_field(json: &mut String, depth: usize, key: &str, value: &str, comma: bool) {
    push_indent(json, depth);
    json.push('"');
    json.push_str(key);
    json.push_str("\": ");
    json.push_str(value);
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

fn json_string_array(values: &[String]) -> String {
    let values = values
        .iter()
        .map(|value| json_string(value))
        .collect::<Vec<_>>();
    format!("[{}]", values.join(", "))
}

fn optional_string_json(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), json_string)
}

pub(crate) fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{8}' => output.push_str("\\b"),
            '\u{c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control.is_control() => {
                output.push_str(&format!("\\u{:04x}", u32::from(control)));
            }
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

#[cfg(test)]
mod tests {
    use super::{json_string, FrontendTranscript};
    use crate::rime_frontend::{
        FrontendCandidate, FrontendContext, FrontendEvent, FrontendRun, FrontendStatus,
    };

    fn status(is_composing: bool) -> FrontendStatus {
        FrontendStatus {
            schema_id: "luna".to_owned(),
            schema_name: "Luna".to_owned(),
            is_disabled: false,
            is_composing,
            is_ascii_mode: false,
            is_full_shape: false,
            is_simplified: true,
            is_traditional: false,
            is_ascii_punct: true,
        }
    }

    fn context() -> FrontendContext {
        FrontendContext {
            input: "ni".to_owned(),
            caret: 2,
            preedit: "ni".to_owned(),
            highlighted: 1,
            last_commit: Some("你".to_owned()),
            candidates: vec![FrontendCandidate {
                text: "你".to_owned(),
                comment: "ni".to_owned(),
                source: "table".to_owned(),
                quality: 10,
            }],
            page_size: 5,
            page_no: 0,
            is_last_page: true,
            select_keys: Some("12345".to_owned()),
            select_labels: vec!["1".to_owned(), "2".to_owned()],
        }
    }

    #[test]
    fn escapes_json_strings() {
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }

    #[test]
    fn serializes_frontend_transcript_with_stable_top_level_order() {
        let run = FrontendRun {
            schema_id: "luna".to_owned(),
            sequence: "ni".to_owned(),
            events: vec![FrontendEvent {
                key: "n".to_owned(),
                keycode: 110,
                mask: 0,
                handled: true,
                commits: vec!["你".to_owned()],
                context: context(),
                status: status(true),
            }],
            commits: vec!["你".to_owned()],
            context: context(),
            status: status(true),
        };

        let json = FrontendTranscript::new(&run).to_json();

        assert_eq!(
            json,
            "{\n  \"schema_id\": \"luna\",\n  \"sequence\": \"ni\",\n  \"events\": [\n    {\n      \"index\": 0,\n      \"key\": \"n\",\n      \"keycode\": 110,\n      \"mask\": 0,\n      \"handled\": true,\n      \"commits\": [\"你\"],\n      \"context\": {\n        \"input\": \"ni\",\n        \"caret\": 2,\n        \"preedit\": \"ni\",\n        \"highlighted\": 1,\n        \"last_commit\": \"你\",\n        \"page_size\": 5,\n        \"page_no\": 0,\n        \"is_last_page\": true,\n        \"select_keys\": \"12345\",\n        \"select_labels\": [\"1\", \"2\"],\n        \"candidates\": [\n          {\n            \"text\": \"你\",\n            \"comment\": \"ni\",\n            \"source\": \"table\",\n            \"quality\": 10\n          }\n        ]\n      },\n      \"status\": {\n        \"schema_id\": \"luna\",\n        \"schema_name\": \"Luna\",\n        \"is_disabled\": false,\n        \"is_composing\": true,\n        \"is_ascii_mode\": false,\n        \"is_full_shape\": false,\n        \"is_simplified\": true,\n        \"is_traditional\": false,\n        \"is_ascii_punct\": true\n      }\n    }\n  ],\n  \"commits\": [\"你\"],\n  \"context\": {\n    \"input\": \"ni\",\n    \"caret\": 2,\n    \"preedit\": \"ni\",\n    \"highlighted\": 1,\n    \"last_commit\": \"你\",\n    \"page_size\": 5,\n    \"page_no\": 0,\n    \"is_last_page\": true,\n    \"select_keys\": \"12345\",\n    \"select_labels\": [\"1\", \"2\"],\n    \"candidates\": [\n      {\n        \"text\": \"你\",\n        \"comment\": \"ni\",\n        \"source\": \"table\",\n        \"quality\": 10\n      }\n    ]\n  },\n  \"status\": {\n    \"schema_id\": \"luna\",\n    \"schema_name\": \"Luna\",\n    \"is_disabled\": false,\n    \"is_composing\": true,\n    \"is_ascii_mode\": false,\n    \"is_full_shape\": false,\n    \"is_simplified\": true,\n    \"is_traditional\": false,\n    \"is_ascii_punct\": true\n  }\n}\n"
        );
    }

    #[test]
    fn frontend_transcript_omits_environment_dependent_values() {
        let run = FrontendRun {
            schema_id: "luna".to_owned(),
            sequence: "n".to_owned(),
            events: vec![],
            commits: vec![],
            context: FrontendContext {
                input: String::new(),
                caret: 0,
                preedit: String::new(),
                highlighted: 0,
                last_commit: None,
                candidates: vec![],
                page_size: 0,
                page_no: 0,
                is_last_page: false,
                select_keys: None,
                select_labels: vec![],
            },
            status: status(false),
        };

        let json = FrontendTranscript::new(&run).to_json();

        assert!(!json.contains("/tmp/"));
        assert!(!json.contains("0x"));
        assert!(!json.contains("timestamp"));
        assert!(!json.contains("duration"));
    }
}
