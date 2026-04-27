use std::{env, fs, path::Path, process::ExitCode};

use yune_core::{
    Candidate, Context, Engine, PunctuationTranslator, Snapshot, StaticTableTranslator, Status,
};

const DEFAULT_SEQUENCE: &str = "nihao ";
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

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None => {
            let output = run_sequence(DEFAULT_SEQUENCE)?;
            println!("{}", output.to_json());
            Ok(())
        }
        Some("run") => {
            let sequence = args.get(1).map_or(DEFAULT_SEQUENCE, String::as_str);
            let output = run_sequence(sequence)?;
            println!("{}", output.to_json());
            Ok(())
        }
        Some("check") => {
            let fixture = args
                .get(1)
                .ok_or_else(|| "usage: yune-cli check <fixture.json>".to_owned())?;
            check_fixture(Path::new(fixture))
        }
        Some("-h" | "--help" | "help") => {
            print_help();
            Ok(())
        }
        Some(command) => Err(format!("unknown command: {command}\n\n{}", help_text())),
    }
}

fn run_sequence(sequence: &str) -> Result<FixtureOutput, String> {
    let mut engine = Engine::new();
    engine.set_schema("sample", "Sample");
    engine.add_translator(PunctuationTranslator::default_half_shape());
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(SAMPLE_DICT)
            .map_err(|error| format!("invalid sample dictionary: {error}"))?,
    );
    let commits = engine
        .process_key_sequence(sequence)
        .map_err(|error| format!("invalid key sequence: {error}"))?;

    Ok(FixtureOutput {
        schema_id: "sample".to_owned(),
        sequence: sequence.to_owned(),
        commits,
        snapshot: engine.snapshot(),
    })
}

fn check_fixture(path: &Path) -> Result<(), String> {
    let expected = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let sequence = sequence_from_fixture(&expected)?;
    let actual = run_sequence(&sequence)?.to_json();
    if normalize_json(&expected) == normalize_json(&actual) {
        println!("ok {}", path.display());
        return Ok(());
    }

    Err(format!(
        "fixture mismatch: {}\n\nexpected:\n{}\n\nactual:\n{}",
        path.display(),
        expected.trim(),
        actual
    ))
}

fn sequence_from_fixture(json: &str) -> Result<String, String> {
    let key = "\"sequence\"";
    let key_start = json
        .find(key)
        .ok_or_else(|| "fixture does not contain a top-level sequence field".to_owned())?;
    let after_key = &json[key_start + key.len()..];
    let colon = after_key
        .find(':')
        .ok_or_else(|| "fixture sequence field is missing ':'".to_owned())?;
    let after_colon = after_key[colon + 1..].trim_start();
    parse_json_string(after_colon)
        .map(|(sequence, _)| sequence)
        .map_err(|error| format!("invalid fixture sequence: {error}"))
}

fn parse_json_string(input: &str) -> Result<(String, usize), String> {
    let mut chars = input.char_indices();
    match chars.next() {
        Some((_, '"')) => {}
        _ => return Err("expected string".to_owned()),
    }

    let mut value = String::new();
    let mut escaped = false;
    for (index, ch) in chars {
        if escaped {
            match ch {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                '/' => value.push('/'),
                'b' => value.push('\u{8}'),
                'f' => value.push('\u{c}'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                'u' => {
                    return Err(
                        "unicode escapes are not supported in fixture sequence strings".to_owned(),
                    );
                }
                other => return Err(format!("unsupported escape: \\{other}")),
            }
            escaped = false;
            continue;
        }

        match ch {
            '\\' => escaped = true,
            '"' => return Ok((value, index + ch.len_utf8())),
            other => value.push(other),
        }
    }

    Err("unterminated string".to_owned())
}

fn normalize_json(input: &str) -> String {
    input.chars().filter(|ch| !ch.is_whitespace()).collect()
}

#[derive(Clone, Debug, PartialEq)]
struct FixtureOutput {
    schema_id: String,
    sequence: String,
    commits: Vec<String>,
    snapshot: Snapshot,
}

impl FixtureOutput {
    fn to_json(&self) -> String {
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

fn json_string(value: &str) -> String {
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

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "usage:\n  yune-cli run [key-sequence]\n  yune-cli check <fixture.json>"
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{check_fixture, json_string, sequence_from_fixture};

    #[test]
    fn escapes_json_strings() {
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }

    #[test]
    fn reads_sequence_from_fixture() {
        let fixture = "{ \"schema_id\": \"sample\", \"sequence\": \"nihao \" }";

        assert_eq!(sequence_from_fixture(fixture).as_deref(), Ok("nihao "));
    }

    #[test]
    fn checked_in_fixtures_match_cli_output() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = manifest_dir
            .parent()
            .and_then(Path::parent)
            .expect("CLI crate should live under workspace crates")
            .join("fixtures");
        let mut fixtures = std::fs::read_dir(&fixtures_dir)
            .expect("fixtures directory should be readable")
            .map(|entry| entry.expect("fixture entry should be readable").path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect::<Vec<_>>();
        fixtures.sort();

        assert!(!fixtures.is_empty());
        for fixture in fixtures {
            check_fixture(&fixture).unwrap_or_else(|error| panic!("{error}"));
        }
    }
}
