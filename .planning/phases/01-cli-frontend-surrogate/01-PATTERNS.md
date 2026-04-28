# Phase 01: CLI Frontend Surrogate - Pattern Map

**Mapped:** 2026-04-29
**Files analyzed:** 10 new/modified implementation, config, test, and fixture targets
**Analogs found:** 10 / 10

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/yune-cli/Cargo.toml` | config | transform | `crates/yune-rime-api/Cargo.toml` | exact |
| `crates/yune-cli/src/args.rs` | route/config | request-response | `crates/yune-cli/src/args.rs` | exact-existing |
| `crates/yune-cli/src/main.rs` | controller | request-response | `crates/yune-cli/src/main.rs` | exact-existing |
| `crates/yune-cli/src/rime_frontend.rs` | service | event-driven | `crates/yune-rime-api/tests/frontend_client.rs` | exact |
| `crates/yune-cli/src/transcript.rs` | utility/model | transform | `crates/yune-cli/src/transcript.rs` | exact-existing |
| `crates/yune-cli/src/render.rs` | utility | transform | `crates/yune-cli/src/render.rs` | exact-existing |
| `crates/yune-cli/src/fixture.rs` | service | file-I/O | `crates/yune-cli/src/fixture.rs` | exact-existing |
| `crates/yune-cli/src/sample_core.rs` | service | batch | `crates/yune-cli/src/sample_core.rs` | exact-existing |
| `crates/yune-cli/tests/frontend_surrogate.rs` or focused inline CLI tests | test | event-driven | `crates/yune-rime-api/tests/frontend_client.rs` | exact |
| `fixtures/frontend-*.json` ABI transcript fixtures | test fixture | file-I/O | `fixtures/sample-nihao.json` | exact |

## Pattern Assignments

### `crates/yune-cli/Cargo.toml` (config, transform)

**Analog:** `crates/yune-rime-api/Cargo.toml` and existing `crates/yune-cli/Cargo.toml`

**Dependency style pattern** (`crates/yune-rime-api/Cargo.toml` lines 8-12):
```toml
[dependencies]
libc = "0.2"
regex = "1"
serde_yaml = "0.9"
yune-core = { path = "../yune-core" }
```

**Existing CLI dependency block to extend** (`crates/yune-cli/Cargo.toml` lines 8-9):
```toml
[dependencies]
yune-core = { path = "../yune-core" }
```

**Apply:** Add `yune-rime-api = { path = "../yune-rime-api" }` beside the existing path dependency. Do not add serde or regex to `yune-cli` unless implementation directly needs them; the phase research expects the CLI to drive the ABI crate rather than reimplement ABI internals.

---

### `crates/yune-cli/src/args.rs` (route/config, request-response)

**Analog:** `crates/yune-cli/src/args.rs`

**Imports and command enum pattern** (lines 0-9):
```rust
use std::path::PathBuf;

use crate::default_sequence;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Run { sequence: String },
    Check { fixture: PathBuf },
    Help,
}
```

**Parser result/error pattern** (lines 11-33):
```rust
impl Command {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        match args.first().map(String::as_str) {
            None => Ok(Self::Run {
                sequence: default_sequence().to_owned(),
            }),
            Some("run") => Ok(Self::Run {
                sequence: args
                    .get(1)
                    .map_or_else(|| default_sequence().to_owned(), ToOwned::to_owned),
            }),
            Some("check") => {
                let fixture = args
                    .get(1)
                    .ok_or_else(|| "usage: yune-cli check <fixture.json>".to_owned())?;
                Ok(Self::Check {
                    fixture: PathBuf::from(fixture),
                })
            }
            Some("-h" | "--help" | "help") => Ok(Self::Help),
            Some(command) => Err(format!("unknown command: {command}\n\n{}", help_text())),
        }
    }
}
```

**Help text pattern** (lines 36-38):
```rust
pub(crate) fn help_text() -> &'static str {
    "usage:\n  yune-cli run [key-sequence]\n  yune-cli check <fixture.json>"
}
```

**Focused parser tests pattern** (lines 40-63):
```rust
#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn default_command_runs_default_sequence() {
        assert_eq!(
            Command::parse(&[]),
            Ok(Command::Run {
                sequence: "nihao ".to_owned()
            })
        );
    }

    #[test]
    fn parses_check_command() {
        assert_eq!(
            Command::parse(&["check".to_owned(), "fixture.json".to_owned()]),
            Ok(Command::Check {
                fixture: "fixture.json".into()
            })
        );
    }
}
```

**Apply:** Extend `Command` with explicit ABI/frontend variants that carry `shared_data_dir`, `user_data_dir`, optional `prebuilt_data_dir`/`staging_dir`/`log_dir`, `schema_id`, fixture/transcript path, output mode, and key sequence. Keep parsing in this file and return `Result<Self, String>` with UI-spec copy shaped as `error: {problem}. next: {action}.` for validation failures.

---

### `crates/yune-cli/src/main.rs` (controller, request-response)

**Analog:** `crates/yune-cli/src/main.rs`

**Module façade pattern** (lines 0-11):
```rust
use std::{env, process::ExitCode};

mod args;
mod fixture;
mod render;
mod rime_frontend;
mod sample_core;
mod transcript;

use args::Command;
use fixture::check_fixture;
use sample_core::{run_sequence, DEFAULT_SEQUENCE};
```

**Boundary error handling and dispatch pattern** (lines 13-35):
```rust
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
    match Command::parse(&args)? {
        Command::Run { sequence } => {
            let output = run_sequence(&sequence)?;
            println!("{}", output.to_json());
            Ok(())
        }
        Command::Check { fixture } => check_fixture(&fixture),
        Command::Help => {
            render::print_help();
            Ok(())
        }
    }
}
```

**Apply:** Keep `main.rs` orchestration-only. New frontend commands should dispatch to `rime_frontend`, `transcript`, `render`, or `fixture` helpers and still return `Result<(), String>`. Successful JSON mode should print only JSON to stdout; errors should be returned as one string for `main` to emit to stderr.

---

### `crates/yune-cli/src/rime_frontend.rs` (service, event-driven)

**Analog:** `crates/yune-rime-api/tests/frontend_client.rs`

**Imports pattern for ABI wrapper/client code** (`frontend_client.rs` lines 0-17):
```rust
use std::{
    ffi::{c_void, CStr, CString},
    fs, mem,
    os::raw::{c_char, c_int},
    path::PathBuf,
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{
    rime_get_api, RimeCandidate, RimeCandidateListIterator, RimeCommit, RimeComposition,
    RimeConfig, RimeConfigIterator, RimeContext, RimeCustomApi, RimeLeversApi, RimeMenu,
    RimeModule, RimeSchemaList, RimeSessionId, RimeStatus, RimeTraits, RimeUserDictIterator, FALSE,
    TRUE,
};
```

**Versioned empty ABI structs pattern** (`frontend_client.rs` lines 27-88):
```rust
fn empty_context() -> RimeContext {
    RimeContext {
        data_size: (mem::size_of::<RimeContext>() - mem::size_of::<i32>()) as i32,
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

fn empty_status() -> RimeStatus {
    RimeStatus {
        data_size: (mem::size_of::<RimeStatus>() - mem::size_of::<i32>()) as i32,
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

fn empty_traits() -> RimeTraits {
    RimeTraits {
        data_size: mem::size_of::<RimeTraits>() as i32,
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

fn empty_commit() -> RimeCommit {
    RimeCommit {
        data_size: (mem::size_of::<RimeCommit>() - mem::size_of::<i32>()) as i32,
        text: ptr::null_mut(),
    }
}
```

**Function-table lookup and setup pattern** (`frontend_client.rs` lines 201-269):
```rust
let api = rime_get_api();
assert!(!api.is_null());
let api = unsafe { &*api };

let setup = api.setup.expect("frontend requires setup");
let get_schema_list = api
    .get_schema_list
    .expect("frontend requires get_schema_list");
let free_schema_list = api
    .free_schema_list
    .expect("frontend requires free_schema_list");

let root = unique_temp_dir("schema-list-module");
let shared = root.join("shared");
let user = root.join("user");
let prebuilt = shared.join("build");
let staging = user.join("build");
fs::create_dir_all(&prebuilt).expect("prebuilt dir should be created");
fs::create_dir_all(&staging).expect("staging dir should be created");

let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
let mut traits = empty_traits();
traits.shared_data_dir = shared_c.as_ptr();
traits.user_data_dir = user_c.as_ptr();
unsafe { setup(&traits) };
```

**Deployment lifecycle pattern** (`frontend_client.rs` lines 1220-1316):
```rust
let api = rime_get_api();
assert!(!api.is_null());
let api = unsafe { &*api };

let deployer_initialize = api
    .deployer_initialize
    .expect("frontend requires deployer_initialize");
let start_maintenance = api
    .start_maintenance
    .expect("frontend requires start_maintenance");
let prebuild = api.prebuild.expect("frontend requires prebuild");
let deploy = api.deploy.expect("frontend requires deploy");
let deploy_schema = api.deploy_schema.expect("frontend requires deploy_schema");
let cleanup_all_sessions = api
    .cleanup_all_sessions
    .expect("frontend requires cleanup_all_sessions");
let create_session = api
    .create_session
    .expect("frontend requires create_session");
let find_session = api.find_session.expect("frontend requires find_session");

cleanup_all_sessions();
// ... build shared/user dirs and traits ...
unsafe { deployer_initialize(&traits) };

assert_eq!(start_maintenance(TRUE), TRUE);
assert_eq!(prebuild(), TRUE);
assert_eq!(deploy(), TRUE);
assert_eq!(deploy_schema(schema_file.as_ptr()), TRUE);

let session_id = create_session();
assert_eq!(find_session(session_id), TRUE);
cleanup_stale_sessions();
assert_eq!(find_session(session_id), TRUE);
assert_eq!(sync_user_data(), TRUE);
assert_eq!(find_session(session_id), FALSE);

let reset_traits = empty_traits();
unsafe { deployer_initialize(&reset_traits) };
```

**Basic session/key/context/status/commit pattern** (`frontend_client.rs` lines 1838-1914):
```rust
let api = rime_get_api();
assert!(!api.is_null());
let api = unsafe { &*api };

let cleanup_all_sessions = api
    .cleanup_all_sessions
    .expect("frontend requires cleanup_all_sessions");
cleanup_all_sessions();

let create_session = api
    .create_session
    .expect("frontend requires create_session");
let find_session = api.find_session.expect("frontend requires find_session");
let destroy_session = api
    .destroy_session
    .expect("frontend requires destroy_session");
let process_key = api.process_key.expect("frontend requires process_key");
let get_status = api.get_status.expect("frontend requires get_status");
let free_status = api.free_status.expect("frontend requires free_status");
let get_context = api.get_context.expect("frontend requires get_context");
let free_context = api.free_context.expect("frontend requires free_context");
let get_commit = api.get_commit.expect("frontend requires get_commit");
let free_commit = api.free_commit.expect("frontend requires free_commit");

let session_id = create_session();
assert_ne!(session_id, 0);
assert_eq!(find_session(session_id), TRUE);
assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

let mut status = empty_status();
assert_eq!(unsafe { get_status(session_id, &mut status) }, TRUE);
assert_eq!(status.is_composing, TRUE);
let schema_id = unsafe { CStr::from_ptr(status.schema_id) };
assert_eq!(schema_id.to_str(), Ok("default"));
assert_eq!(unsafe { free_status(&mut status) }, TRUE);

let mut context = empty_context();
assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
assert_eq!(context.composition.length, 2);
assert_eq!(context.menu.page_size, 5);
assert_eq!(context.menu.num_candidates, 1);
assert_eq!(context.menu.highlighted_candidate_index, 0);
let first_candidate = unsafe { *context.menu.candidates };
let first_candidate_text = unsafe { CStr::from_ptr(first_candidate.text) };
assert_eq!(first_candidate_text.to_str(), Ok("ni"));
assert_eq!(unsafe { free_context(&mut context) }, TRUE);

let mut commit = empty_commit();
assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
let commit_text = unsafe { CStr::from_ptr(commit.text) };
assert_eq!(commit_text.to_str(), Ok("ni"));
assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

assert_eq!(destroy_session(session_id), TRUE);
cleanup_all_sessions();
```

**Candidate page/select key/label pattern** (`frontend_client.rs` lines 1993-2038):
```rust
let mut context = empty_context();
assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
assert_eq!(context.menu.page_size, 2);
assert_eq!(context.menu.page_no, 1);
assert_eq!(context.menu.highlighted_candidate_index, 1);
assert_eq!(context.menu.num_candidates, 2);
assert_eq!(
    unsafe { CStr::from_ptr(context.menu.select_keys) }.to_str(),
    Ok("AB")
);
assert!(!context.select_labels.is_null());
let select_labels = unsafe {
    std::slice::from_raw_parts(context.select_labels, context.menu.page_size as usize)
};
assert_eq!(
    unsafe { CStr::from_ptr(select_labels[0]) }.to_str(),
    Ok("Alpha")
);
let candidates = unsafe {
    std::slice::from_raw_parts(
        context.menu.candidates,
        context.menu.num_candidates as usize,
    )
};
assert_eq!(
    unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
    Ok("巴")
);
assert_eq!(unsafe { free_context(&mut context) }, TRUE);
```

**Apply:** Implement a safe-ish CLI-facing wrapper in `rime_frontend.rs` that owns all `CString`, raw pointer reads, versioned struct initialization, function-table option extraction, and free pairing. Expose `Result<..., String>` functions to callers. Ensure `destroy_session`, `cleanup_all_sessions`, and `finalize` are called on every path that successfully initialized process-wide state or created a session.

---

### `crates/yune-cli/src/transcript.rs` (utility/model, transform)

**Analog:** `crates/yune-cli/src/transcript.rs`

**Stable top-level JSON ordering pattern** (lines 10-45):
```rust
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
```

**Context field ordering pattern** (lines 48-96):
```rust
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
```

**Candidate/status ordering and string escaping pattern** (lines 98-146, 148-217, 219-269):
```rust
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
```

**Apply:** Extend this module with per-event transcript structs and serializers. Preserve handcrafted JSON, two-space indentation, stable key order, no timestamps/paths, and `json_string` for every string crossing into JSON. Add event records without moving serialization into `main.rs` or `rime_frontend.rs`.

---

### `crates/yune-cli/src/render.rs` (utility, transform)

**Analog:** `crates/yune-cli/src/render.rs` plus `01-UI-SPEC.md`

**Existing render boundary pattern** (`render.rs` lines 0-4):
```rust
use crate::args::help_text;

pub(crate) fn print_help() {
    println!("{}", help_text());
}
```

**Apply:** Keep all human-facing output functions here. Add line-oriented render helpers for setup/session/key events that write only human transcript text on success. Use plain labels matching UI spec: `usage:`, `commands:`, `error:`, `next:`, `commit:`, `preedit:`, `caret:`, `highlighted:`, `candidates: none`, `selected: yes`. Do not emit ANSI color, cursor movement, spinners, tables, or JSON from this module.

---

### `crates/yune-cli/src/fixture.rs` (service, file-I/O)

**Analog:** `crates/yune-cli/src/fixture.rs`

**Read/compare/error pattern** (lines 4-20):
```rust
pub(crate) fn check_fixture(path: &Path) -> Result<(), String> {
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
```

**Minimal fixture field extraction pattern** (lines 22-35):
```rust
pub(crate) fn sequence_from_fixture(json: &str) -> Result<String, String> {
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
```

**Handwritten parser and normalization pattern** (lines 37-80):
```rust
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
```

**Fixture test pattern** (lines 82-118):
```rust
#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{check_fixture, sequence_from_fixture};

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
```

**Apply:** Preserve existing core fixture comparison while adding ABI-backed transcript comparison. Keep file I/O and expected-vs-actual normalization in `fixture.rs`; call into `rime_frontend` for actual ABI replay instead of duplicating setup/session code here.

---

### `crates/yune-cli/src/sample_core.rs` (service, batch)

**Analog:** `crates/yune-cli/src/sample_core.rs`

**Core fallback run pattern** (lines 0-36):
```rust
use yune_core::{Engine, PunctuationTranslator, StaticTableTranslator};

use crate::transcript::FixtureOutput;

pub(crate) const DEFAULT_SEQUENCE: &str = "nihao ";

pub(crate) fn run_sequence(sequence: &str) -> Result<FixtureOutput, String> {
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
```

**Apply:** Keep this as retained compatibility scaffolding. Do not route ABI/frontend commands through `yune_core::Engine` directly. If existing `Run`/`Check` remains core-backed in the first step, leave its shape intact and add separate frontend commands.

---

### `crates/yune-cli/tests/frontend_surrogate.rs` or focused inline CLI tests (test, event-driven)

**Analog:** `crates/yune-rime-api/tests/frontend_client.rs`

**Serialized process-wide test guard pattern** (`frontend_client.rs` lines 136-149):
```rust
fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned");
    let api = unsafe { &mut *rime_get_api() };
    let initialize = api
        .initialize
        .expect("frontend requires initialize for test setup");
    let traits = empty_traits();
    unsafe { initialize(&traits) };
    guard
}
```

**Unique temp runtime pattern** (`frontend_client.rs` lines 182-191):
```rust
fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "yune-rime-api-frontend-{label}-{}-{nanos}",
        std::process::id()
    ))
}
```

**Temp shared/user schema setup pattern** (`frontend_client.rs` lines 1952-1977):
```rust
let root = unique_temp_dir("schema-dictionary-paging");
let shared = root.join("shared");
let user = root.join("user");
let staging = user.join("build");
fs::create_dir_all(&shared).expect("shared dir should be created");
fs::create_dir_all(&staging).expect("staging dir should be created");
fs::write(
    staging.join("luna.schema.yaml"),
    "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n  alternative_select_labels: [Alpha, Beta]\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: frontend\n",
)
.expect("schema config should be written");
fs::write(
    shared.join("frontend.dict.yaml"),
    "\
---\nname: frontend\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nba\t八\t10\nba\t吧\t9\nba\t爸\t8\nba\t巴\t7\nba\t把\t6\nba\t拔\t5\n",
)
.expect("dictionary should be written");

let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
let mut traits = empty_traits();
traits.shared_data_dir = shared_c.as_ptr();
traits.user_data_dir = user_c.as_ptr();
unsafe { setup(&traits) };
```

**Apply:** Any new CLI frontend tests must serialize global RIME state, use unique temp `shared`/`user` dirs, write minimal YAML/dict fixtures, and clean sessions/runtime after each test. Use the same function-table path the CLI wrapper uses, not direct `yune_core` calls.

---

### `fixtures/frontend-*.json` ABI transcript fixtures (test fixture, file-I/O)

**Analog:** `fixtures/sample-nihao.json`

**Stable fixture JSON pattern** (`fixtures/sample-nihao.json` lines 0-24):
```json
{
  "schema_id": "sample",
  "sequence": "nihao ",
  "commits": ["你好"],
  "context": {
    "input": "",
    "caret": 0,
    "preedit": "",
    "highlighted": 0,
    "last_commit": "你好",
    "candidates": []
  },
  "status": {
    "schema_id": "sample",
    "schema_name": "Sample",
    "is_disabled": false,
    "is_composing": false,
    "is_ascii_mode": false,
    "is_full_shape": false,
    "is_simplified": false,
    "is_traditional": false,
    "is_ascii_punct": false
  }
}
```

**Apply:** New ABI fixtures should preserve deterministic ordering and two-space indentation. Per phase decisions, add per-event records for ABI replay; do not include timestamps, temp paths, session pointer values, or environment-specific directories.

## Shared Patterns

### ABI function table is the client contract

**Source:** `crates/yune-rime-api/src/api_table.rs` lines 62-163
**Apply to:** `crates/yune-cli/src/rime_frontend.rs`, frontend tests

```rust
fn build_rime_api() -> RimeApi {
    RimeApi {
        data_size: (std::mem::size_of::<RimeApi>() - std::mem::size_of::<c_int>()) as c_int,
        setup: Some(RimeSetup),
        set_notification_handler: Some(RimeSetNotificationHandler),
        initialize: Some(RimeInitialize),
        finalize: Some(RimeFinalize),
        start_maintenance: Some(RimeStartMaintenance),
        is_maintenance_mode: Some(RimeIsMaintenancing),
        join_maintenance_thread: Some(RimeJoinMaintenanceThread),
        deployer_initialize: Some(RimeDeployerInitialize),
        prebuild: Some(RimePrebuildAllSchemas),
        deploy: Some(RimeDeployWorkspace),
        deploy_schema: Some(RimeDeploySchema),
        deploy_config_file: Some(RimeDeployConfigFile),
        sync_user_data: Some(RimeSyncUserData),
        create_session: Some(RimeCreateSession),
        find_session: Some(RimeFindSession),
        destroy_session: Some(RimeDestroySession),
        cleanup_stale_sessions: Some(RimeCleanupStaleSessions),
        cleanup_all_sessions: Some(RimeCleanupAllSessions),
        process_key: Some(RimeProcessKey),
        commit_composition: Some(RimeCommitComposition),
        clear_composition: Some(RimeClearComposition),
        get_commit: Some(RimeGetCommit),
        free_commit: Some(RimeFreeCommit),
        get_context: Some(RimeGetContext),
        free_context: Some(RimeFreeContext),
        get_status: Some(RimeGetStatus),
        free_status: Some(RimeFreeStatus),
        get_schema_list: Some(RimeGetSchemaList),
        free_schema_list: Some(RimeFreeSchemaList),
        get_current_schema: Some(RimeGetCurrentSchema),
        select_schema: Some(RimeSelectSchema),
        // ... remaining ABI entries ...
        highlight_candidate: Some(RimeHighlightCandidate),
        highlight_candidate_on_current_page: Some(RimeHighlightCandidateOnCurrentPage),
        change_page: Some(RimeChangePage),
    }
}
```

### Runtime setup requires valid `RimeTraits` and explicit cleanup

**Source:** `crates/yune-rime-api/src/runtime.rs` lines 90-159 and `crates/yune-rime-api/src/deployment.rs` lines 21-37
**Apply to:** `rime_frontend.rs`, tests, CLI command validation

```rust
pub(crate) unsafe fn from_traits(traits: *const RimeTraits) -> Option<Self> {
    if traits.is_null() {
        return None;
    }

    let data_size = unsafe { (*traits).data_size };
    let provided_string = |member: *const *const c_char| {
        if rime_struct_has_member(traits, data_size, member) {
            unsafe { optional_c_string(*member) }
        } else {
            None
        }
    };

    let shared_data_dir = provided_string(unsafe { ptr::addr_of!((*traits).shared_data_dir) })
        .unwrap_or_else(|| ".".to_owned());
    let user_data_dir = provided_string(unsafe { ptr::addr_of!((*traits).user_data_dir) })
        .unwrap_or_else(|| ".".to_owned());
    let prebuilt_data_dir =
        provided_string(unsafe { ptr::addr_of!((*traits).prebuilt_data_dir) })
            .unwrap_or_else(|| path_join(&shared_data_dir, "build"));
    let staging_dir = provided_string(unsafe { ptr::addr_of!((*traits).staging_dir) })
        .unwrap_or_else(|| path_join(&user_data_dir, "build"));
    // ... build RuntimePaths ...
}

#[no_mangle]
pub unsafe extern "C" fn RimeInitialize(traits: *const RimeTraits) {
    unsafe { RimeSetup(traits) };
    service_started().store(true, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn RimeFinalize() {
    RimeCleanupAllSessions();
    service_started().store(false, Ordering::SeqCst);
}
```

### ABI reads allocate nested memory; callers must free with matching functions

**Source:** `crates/yune-rime-api/src/context_api.rs` lines 9-16, 44-52, 236-244 and `crates/yune-rime-api/src/ffi_memory.rs` lines 290-352
**Apply to:** `rime_frontend.rs` state capture

```rust
/// Copies the unread commit text for a session into a caller-provided commit.
///
/// # Safety
///
/// `commit` must be either null or a valid, writable pointer to a `RimeCommit`.
/// When this function returns `TRUE`, the caller must release `commit.text` by
/// passing the same commit object to `RimeFreeCommit`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCommit(session_id: RimeSessionId, commit: *mut RimeCommit) -> Bool {
    // ...
}

/// Copies the current composition and first candidate page into caller storage.
///
/// # Safety
///
/// `context` must be either null or a valid, writable pointer to a
/// `RimeContext` initialized with a positive `data_size`. When this function
/// returns `TRUE`, the caller must release nested strings and candidate memory
/// by passing the same context object to `RimeFreeContext`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetContext(
    session_id: RimeSessionId,
    context: *mut RimeContext,
) -> Bool {
    // ...
}

/// Releases nested allocations returned in a `RimeContext`.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeContext(context: *mut RimeContext) -> Bool {
    if context.is_null() {
        return FALSE;
    }
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    free_context_fields(context);
    clear_context(context);
    TRUE
}
```

### Session lifecycle and key processing go through API/session state

**Source:** `crates/yune-rime-api/src/session.rs` lines 149-185 and `crates/yune-rime-api/src/lib.rs` lines 317-492
**Apply to:** `rime_frontend.rs`, frontend tests

```rust
#[no_mangle]
pub extern "C" fn RimeCreateSession() -> RimeSessionId {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .create_session()
}

#[no_mangle]
pub extern "C" fn RimeDestroySession(session_id: RimeSessionId) -> Bool {
    bool_from(
        session_id != 0
            && sessions()
                .lock()
                .expect("session registry should not be poisoned")
                .sessions
                .remove(&session_id)
                .is_some(),
    )
}

#[no_mangle]
pub extern "C" fn RimeCleanupAllSessions() {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .sessions
        .clear();
}

#[no_mangle]
pub extern "C" fn RimeProcessKey(session_id: RimeSessionId, keycode: c_int, mask: c_int) -> Bool {
    if session_id == 0 || /* invalid mask/key cases */ false {
        return FALSE;
    }
    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    // process key and append unread commit as needed
    bool_from(
        accepted || matches!(key_event.code, KeyCode::Character(ch) if ch != ' ') || was_composing,
    )
}
```

### Schema selection resets and installs session processors

**Source:** `crates/yune-rime-api/src/schema_selection.rs` lines 40-80 and 82-125
**Apply to:** `rime_frontend.rs`, tests using explicit schema IDs

```rust
#[no_mangle]
pub unsafe extern "C" fn RimeSelectSchema(
    session_id: RimeSessionId,
    schema_id: *const c_char,
) -> Bool {
    if schema_id.is_null() {
        return FALSE;
    }
    let schema_id = unsafe { CStr::from_ptr(schema_id) }
        .to_string_lossy()
        .into_owned();

    let selected = with_session(session_id, |session| {
        apply_schema_to_session(session, &schema_id);
        true
    });
    if selected == TRUE {
        let status = sessions()
            .lock()
            .expect("session registry should not be poisoned")
            .sessions
            .get(&session_id)
            .map(|session| session.engine.status());
        if let Some(status) = status {
            notify(
                session_id,
                "schema",
                &format!("{}/{}", status.schema_id, status.schema_name),
            );
        }
    }
    selected
}

pub(crate) fn apply_schema_to_session(session: &mut SessionState, schema_id: &str) {
    let schema_name = deployed_schema_name(schema_id);
    session.engine.set_schema(schema_id.to_owned(), schema_name);
    session.engine.reset_translators();
    session.engine.reset_filters();
    session.key_binder = None;
    session.speller = None;
    session.editor_processor = None;
    session.editor_bindings.clear();
    // ... install processors/translators/filters ...
    session.engine.clear_composition();
    session.input_buffer = None;
    session.unread_commit = None;
}
```

### CLI errors use `Result<(), String>` at the boundary

**Source:** `crates/yune-cli/src/main.rs` lines 13-35, `crates/yune-cli/src/sample_core.rs` lines 18-35, `crates/yune-cli/src/fixture.rs` lines 4-20
**Apply to:** all new CLI command handlers

```rust
fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn run_sequence(sequence: &str) -> Result<FixtureOutput, String> {
    let mut engine = Engine::new();
    // ...
    let commits = engine
        .process_key_sequence(sequence)
        .map_err(|error| format!("invalid key sequence: {error}"))?;
    Ok(FixtureOutput { /* ... */ })
}
```

### Deterministic JSON is handcrafted, not serde-derived

**Source:** `crates/yune-cli/src/transcript.rs` lines 219-269
**Apply to:** `transcript.rs`, fixture generation

```rust
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
```

### Key sequence parsing can reuse `yune_core` semantics

**Source:** `crates/yune-core/src/engine.rs` lines 353-361 and `crates/yune-core/src/lib.rs` lines 26 export `parse_key_sequence`
**Apply to:** `rime_frontend.rs` replay loop

```rust
pub fn process_key_sequence(
    &mut self,
    input: &str,
) -> Result<Vec<String>, KeySequenceParseError> {
    Ok(parse_key_sequence(input)?
        .into_iter()
        .filter_map(|key_event| self.process_key_event(key_event))
        .collect())
}
```

**Apply:** For ABI replay, parse the same librime-style sequence, then translate each `KeyEvent` to RIME keycode/mask before calling `process_key`. This preserves CLI fixture syntax while exercising the ABI path.

## No Analog Found

All inferred files have close analogs in the current codebase. There are no unmatched files for this phase.

## Metadata

**Analog search scope:** `crates/yune-cli/src`, `crates/yune-rime-api/src`, `crates/yune-rime-api/tests`, `crates/yune-core/src`, `fixtures`, workspace Cargo manifests
**Files scanned:** 70+ Rust/config/fixture files via repository listing and targeted searches
**Pattern extraction date:** 2026-04-29
**Project instructions:** No `CLAUDE.md`, `.claude/skills/*/SKILL.md`, or `.agents/skills/*/SKILL.md` files were present in the active worktree.
