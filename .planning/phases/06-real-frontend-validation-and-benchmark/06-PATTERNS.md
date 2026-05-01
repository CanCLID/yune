# Phase 6: Real Frontend Validation And Benchmark - Pattern Map

**Mapped:** 2026-05-01
**Files analyzed:** 9
**Analogs found:** 9 / 9

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/yune-rime-api/tests/dynamic_loader.rs` | test / validation harness | request-response + file-I/O + event-driven | `crates/yune-rime-api/tests/dynamic_loader.rs` | exact-existing |
| `crates/yune-rime-api/tests/frontend_hosts/mod.rs` | test utility / host trace model | request-response + event-driven + transform | `crates/yune-cli/src/rime_frontend.rs` | role-match |
| `crates/yune-rime-api/tests/frontend_hosts/native.rs` | test / host scenario | request-response + event-driven + file-I/O | `crates/yune-rime-api/tests/dynamic_loader.rs` | exact |
| `crates/yune-rime-api/tests/frontend_hosts/typeduck_web.rs` | test / source-modeled fixture | request-response + event-driven + file-I/O | `crates/yune-rime-api/tests/frontend_client.rs` | role-match |
| `crates/yune-rime-api/tests/frontend_hosts/native_frontends.rs` | test / source-modeled fixture | request-response + event-driven | `crates/yune-rime-api/src/tests/lifecycle_safety.rs` | role-match |
| `fixtures/frontend-traces/*.json` or `docs/frontend-validation/*.md` | fixture / documentation artifact | file-I/O + event-driven trace | `crates/yune-cli/src/transcript.rs` | role-match |
| `crates/yune-rime-api/benches/frontend_baselines.rs` or equivalent benchmark target | benchmark harness | batch + request-response + file-I/O | `crates/yune-rime-api/tests/frontend_client.rs` | role-match |
| `crates/yune-rime-api/Cargo.toml` | config | build config | `crates/yune-rime-api/Cargo.toml` | exact-existing |
| `docs/real-frontend-validation-plan.md` | documentation | transform / decision record | `docs/compat-foundation-summary.md` | role-match |

## Pattern Assignments

### `crates/yune-rime-api/tests/dynamic_loader.rs` (test / validation harness, request-response + file-I/O + event-driven)

**Analog:** `crates/yune-rime-api/tests/dynamic_loader.rs`

**Imports pattern** (lines 0-15):
```rust
use std::{
    ffi::{c_void, CStr, CString},
    fs, mem,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    process::Command,
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use libloading::Library;
use yune_rime_api::{
    Bool, RimeCommit, RimeComposition, RimeContext, RimeMenu, RimeSessionId, RimeStatus,
    RimeTraits, FALSE, TRUE,
};
```

**Dynamic cdylib discovery pattern** (lines 140-234):
```rust
fn dynamic_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yune_rime_api.dll"
    } else if cfg!(target_os = "macos") {
        "libyune_rime_api.dylib"
    } else {
        "libyune_rime_api.so"
    }
}

fn artifact_candidates() -> Result<Vec<PathBuf>, String> {
    let target_dir = target_dir()?;
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_owned());
    Ok(vec![
        target_dir.join(&profile).join(dynamic_library_name()),
        target_dir.join("debug").join(dynamic_library_name()),
        target_dir.join("release").join(dynamic_library_name()),
    ])
}

fn build_dynamic_artifact() -> Result<(), String> {
    let manifest = manifest_dir()?.join("Cargo.toml");
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .arg("build")
        .arg("-p")
        .arg("yune-rime-api")
        .arg("--manifest-path")
        .arg(manifest);
    if let Some(target_dir) = std::env::var_os("CARGO_TARGET_DIR") {
        command.arg("--target-dir").arg(target_dir);
    }

    let output = command
        .output()
        .map_err(|error| format!("failed to run cargo build for dynamic artifact: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo build -p yune-rime-api failed with status {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
```

**Host API table pattern** (lines 261-313):
```rust
#[test]
fn dynamic_loader_harness_loads_cargo_cdylib_and_api_table() {
    let _guard = test_guard();
    let artifact =
        discover_dynamic_artifact().unwrap_or_else(|message| panic!("missing artifact: {message}"));

    // SAFETY: loading is restricted to the Cargo-built yune-rime-api artifact
    // discovered under the active target directory.
    let library = unsafe { Library::new(&artifact) }.unwrap_or_else(|error| {
        panic!(
            "failed to load dynamic artifact {}: {error}",
            artifact.display()
        )
    });

    // SAFETY: the harness resolves only the exported null-terminated rime_get_api symbol.
    let get_api: libloading::Symbol<RimeGetApi> = unsafe { library.get(b"rime_get_api\0") }
        .unwrap_or_else(|error| panic!("missing dynamic symbol rime_get_api: {error}"));
    // SAFETY: the resolved symbol follows the exported rime_get_api contract.
    let api = unsafe { get_api() };
    assert!(!api.is_null(), "null API table returned by rime_get_api");
    // SAFETY: the table pointer was checked for null before dereference, and the library
    // is kept alive for the full duration of table use.
    let api = unsafe { &mut *api };
    assert_eq!(
        api.data_size,
        (mem::size_of_val(api) - mem::size_of::<c_int>()) as c_int,
        "runtime behavior failure: unexpected RimeApi data_size"
    );

    let setup = require("setup", api.setup);
    let initialize = require("initialize", api.initialize);
    let finalize = require("finalize", api.finalize);
    let set_notification_handler =
        require("set_notification_handler", api.set_notification_handler);
    let deploy = require("deploy", api.deploy);
    let create_session = require("create_session", api.create_session);
```

**Lifecycle + notification + free pairing pattern** (lines 327-433):
```rust
let shared_c =
    CString::new(shared.to_string_lossy().as_ref()).expect("shared path should be valid");
let user_c = CString::new(user.to_string_lossy().as_ref()).expect("user path should be valid");
let prebuilt_c =
    CString::new(prebuilt.to_string_lossy().as_ref()).expect("prebuilt path should be valid");
let staging_c =
    CString::new(staging.to_string_lossy().as_ref()).expect("staging path should be valid");
let mut traits = empty_traits();
traits.shared_data_dir = shared_c.as_ptr();
traits.user_data_dir = user_c.as_ptr();
traits.prebuilt_data_dir = prebuilt_c.as_ptr();
traits.staging_dir = staging_c.as_ptr();

// SAFETY: the C strings referenced by traits are kept alive through setup/initialize.
unsafe { setup(&traits) };
notification_events()
    .lock()
    .expect("notification events should not be poisoned")
    .clear();
set_notification_handler(Some(record_notification), 0x42_usize as *mut c_void);
// SAFETY: the C strings referenced by traits are kept alive through setup/initialize.
unsafe { initialize(&traits) };

assert_eq!(
    deploy(),
    TRUE,
    "runtime behavior failure: deploy returned {}",
    bool_name(deploy())
);

let session_id = create_session();
assert_ne!(
    session_id, 0,
    "runtime behavior failure: create_session returned 0"
);
assert_eq!(
    find_session(session_id),
    TRUE,
    "runtime behavior failure: find_session could not find newly created session"
);

let schema_id = CString::new("dynamic_schema").expect("schema id should be valid");
assert_eq!(
    unsafe { select_schema(session_id, schema_id.as_ptr()) },
    TRUE,
    "runtime behavior failure: select_schema(dynamic_schema) failed"
);
assert_eq!(
    process_key(session_id, 'n' as c_int, 0),
    TRUE,
    "runtime behavior failure: process_key('n') failed"
);

let mut status = empty_status();
assert_eq!(unsafe { get_status(session_id, &mut status) }, TRUE);
assert_eq!(unsafe { free_status(&mut status) }, TRUE);

let mut context = empty_context();
assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
assert_eq!(unsafe { free_context(&mut context) }, TRUE);

let mut commit = empty_commit();
assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

assert_eq!(destroy_session(session_id), TRUE);
cleanup_all_sessions();
set_notification_handler(None, ptr::null_mut());
finalize();
```

**Apply to Phase 6:** extend this file or extract shared helpers, but preserve the `libloading` boundary, explicit `require()` checks for every needed function pointer, `CString` lifetime comments, serialized test guard, and temp runtime cleanup.

---

### `crates/yune-rime-api/tests/frontend_hosts/mod.rs` (test utility / host trace model, request-response + event-driven + transform)

**Analog:** `crates/yune-cli/src/rime_frontend.rs`

**Trace data model pattern** (lines 43-106):
```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrontendRun {
    pub(crate) schema_id: String,
    pub(crate) sequence: String,
    pub(crate) events: Vec<FrontendEvent>,
    pub(crate) commits: Vec<String>,
    pub(crate) context: FrontendContext,
    pub(crate) status: FrontendStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrontendEvent {
    pub(crate) key: String,
    pub(crate) keycode: c_int,
    pub(crate) mask: c_int,
    pub(crate) handled: bool,
    pub(crate) commits: Vec<String>,
    pub(crate) context: FrontendContext,
    pub(crate) status: FrontendStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrontendContext {
    pub(crate) input: String,
    pub(crate) caret: usize,
    pub(crate) preedit: String,
    pub(crate) highlighted: usize,
    pub(crate) last_commit: Option<String>,
    pub(crate) candidates: Vec<FrontendCandidate>,
    pub(crate) page_size: usize,
    pub(crate) page_no: usize,
    pub(crate) is_last_page: bool,
    pub(crate) select_keys: Option<String>,
    pub(crate) select_labels: Vec<String>,
}
```

**Corrective error pattern** (lines 108-129):
```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FrontendError {
    problem: String,
    next: String,
}

impl FrontendError {
    fn new(problem: impl Into<String>, next: impl Into<String>) -> Self {
        Self {
            problem: problem.into(),
            next: next.into(),
        }
    }
}

impl std::fmt::Display for FrontendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "error: {}. next: {}.", self.problem, self.next)
    }
}

impl std::error::Error for FrontendError {}
```

**API-table missing-function validation pattern** (lines 152-175, 585-590):
```rust
// SAFETY: `rime_get_api` returns a static function table pointer owned by the
// ABI crate for the duration of the process; null is checked before use.
let api = unsafe {
    let api = rime_get_api();
    if api.is_null() {
        return Err(FrontendError::new(
            "RimeApi function table is unavailable",
            "ensure yune-rime-api is linked into yune-cli",
        ));
    }
    &*api
};

let setup = api.setup.ok_or_else(|| missing_api("setup"))?;
let initialize = api.initialize.ok_or_else(|| missing_api("initialize"))?;
let deploy = api.deploy.ok_or_else(|| missing_api("deploy"))?;
let create_session = api
    .create_session
    .ok_or_else(|| missing_api("create_session"))?;
let process_key = api.process_key.ok_or_else(|| missing_api("process_key"))?;
let select_schema = api
    .select_schema
    .ok_or_else(|| missing_api("select_schema"))?;

fn missing_api(name: &str) -> FrontendError {
    FrontendError::new(
        format!("RimeApi missing {name}"),
        "ensure yune-rime-api exposes the required frontend function table entry",
    )
}
```

**Cleanup guard pattern** (lines 655-695):
```rust
struct CleanupGuard<'api> {
    api: &'api yune_rime_api::RimeApi,
    session_id: Option<RimeSessionId>,
    initialized: bool,
}

impl<'api> CleanupGuard<'api> {
    fn new(api: &'api yune_rime_api::RimeApi) -> Self {
        Self {
            api,
            session_id: None,
            initialized: false,
        }
    }
}

impl Drop for CleanupGuard<'_> {
    fn drop(&mut self) {
        if let Some(session_id) = self.session_id.take() {
            if let Some(destroy_session) = self.api.destroy_session {
                destroy_session(session_id);
            }
        }
        if let Some(cleanup_all_sessions) = self.api.cleanup_all_sessions {
            cleanup_all_sessions();
        }
        if self.initialized {
            if let Some(finalize) = self.api.finalize {
                finalize();
            }
        }
    }
}
```

**Apply to Phase 6:** host trace helpers should return structured results/errors rather than panicking when used by optional real-frontend attempts. Keep panic/assert style in mandatory tests; use `FrontendError`-style results for blocker capture and manual validation helpers.

---

### `crates/yune-rime-api/tests/frontend_hosts/native.rs` (test / host scenario, request-response + event-driven + file-I/O)

**Analog:** `crates/yune-rime-api/tests/dynamic_loader.rs`

**Shared state and notification capture pattern** (lines 19-25, 90-119):
```rust
#[derive(Debug, PartialEq, Eq)]
struct NotificationEvent {
    context_object: usize,
    session_id: RimeSessionId,
    message_type: String,
    message_value: String,
}

fn notification_events() -> &'static Mutex<Vec<NotificationEvent>> {
    static NOTIFICATION_EVENTS: OnceLock<Mutex<Vec<NotificationEvent>>> = OnceLock::new();
    NOTIFICATION_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

extern "C" fn record_notification(
    context_object: *mut c_void,
    session_id: RimeSessionId,
    message_type: *const c_char,
    message_value: *const c_char,
) {
    // SAFETY: the ABI shim invokes handlers with valid NUL-terminated message
    // strings for the duration of the callback.
    let message_type = unsafe { CStr::from_ptr(message_type) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: same as above.
    let message_value = unsafe { CStr::from_ptr(message_value) }
        .to_string_lossy()
        .into_owned();
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .push(NotificationEvent {
            context_object: context_object as usize,
            session_id,
            message_type,
            message_value,
        });
}
```

**Test serialization and temp runtime pattern** (lines 121-138, 240-251):
```rust
fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "yune-rime-api-dynamic-loader-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("dynamic loader test lock should not be poisoned")
}

fn write_minimal_schema(shared: &Path) {
    fs::write(
        shared.join("default.yaml"),
        "config_version: dynamic-loader\nschema_list:\n  - schema: dynamic_schema\n",
    )
    .expect("dynamic loader default config should be written");
    fs::write(
        shared.join("dynamic_schema.schema.yaml"),
        "schema:\n  schema_id: dynamic_schema\n  name: Dynamic Schema\n",
    )
    .expect("dynamic loader schema config should be written");
}
```

**Required function pointer pattern** (lines 236-238):
```rust
fn require<T>(name: &str, function: Option<T>) -> T {
    function.unwrap_or_else(|| panic!("null required RimeApi function pointer: {name}"))
}
```

**Apply to Phase 6:** native host scenario should build an ordered call trace around this exact lifecycle, adding stale sessions, notification handler replacement, repeated initialize/finalize, maintenance/deploy, and context/status/commit/free events. Do not call `yune-core` directly for validation.

---

### `crates/yune-rime-api/tests/frontend_hosts/typeduck_web.rs` (test / source-modeled fixture, request-response + event-driven + file-I/O)

**Analog:** `crates/yune-rime-api/tests/frontend_client.rs`

**Frontend wrapper call coverage pattern** (lines 1839-1914):
```rust
#[test]
fn frontend_style_api_table_can_drive_basic_composition_flow() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    assert_eq!(
        api.data_size,
        (mem::size_of_val(api) - mem::size_of::<i32>()) as i32
    );

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
    let get_input = api.get_input.expect("frontend requires get_input");
    let get_status = api.get_status.expect("frontend requires get_status");
    let free_status = api.free_status.expect("frontend requires free_status");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");
```

**Candidate paging / selection wrapper pattern** (lines 2031-2065, 2093-2147):
```rust
let highlight_candidate = api
    .highlight_candidate
    .expect("frontend requires highlight_candidate");
let highlight_candidate_on_current_page = api
    .highlight_candidate_on_current_page
    .expect("frontend requires highlight_candidate_on_current_page");
let change_page = api.change_page.expect("frontend requires change_page");
let select_candidate_on_current_page = api
    .select_candidate_on_current_page
    .expect("frontend requires select_candidate_on_current_page");

let session_id = create_session();
assert_ne!(session_id, 0);
let schema_id = CString::new("luna").expect("schema id should be valid");
assert_eq!(
    unsafe { select_schema(session_id, schema_id.as_ptr()) },
    TRUE
);
assert_eq!(highlight_candidate(session_id, 0), FALSE);
assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
assert_eq!(highlight_candidate(session_id, 3), TRUE);

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
assert_eq!(unsafe { free_context(&mut context) }, TRUE);

assert_eq!(highlight_candidate_on_current_page(session_id, 0), TRUE);
assert_eq!(change_page(session_id, FALSE), TRUE);
assert_eq!(select_candidate_on_current_page(session_id, 1), TRUE);
assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("拔"));
assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
```

**Levers customization pattern for TypeDuck-Web custom settings** (lines 588-633, 661-691):
```rust
let custom_settings_init = levers_api
    .custom_settings_init
    .expect("levers API should expose custom settings init");
let custom_settings_destroy = levers_api
    .custom_settings_destroy
    .expect("levers API should expose custom settings destroy");
let load_settings = levers_api
    .load_settings
    .expect("levers API should expose load settings");
let save_settings = levers_api
    .save_settings
    .expect("levers API should expose save settings");
let customize_bool = levers_api
    .customize_bool
    .expect("levers API should expose bool customization");
let customize_int = levers_api
    .customize_int
    .expect("levers API should expose int customization");
let customize_string = levers_api
    .customize_string
    .expect("levers API should expose string customization");

let config_id = CString::new("luna_pinyin.schema").expect("config id should be valid");
let generator = CString::new("frontend-client").expect("generator should be valid");
let settings = unsafe { custom_settings_init(config_id.as_ptr(), generator.as_ptr()) };
assert!(!settings.is_null());

assert_eq!(unsafe { load_settings(settings) }, FALSE);
assert_eq!(unsafe { is_first_run(settings) }, TRUE);
assert_eq!(unsafe { settings_is_modified(settings) }, FALSE);

let bool_key = CString::new("switches/@0/reset").expect("custom key should be valid");
let int_key = CString::new("menu/page_size").expect("custom key should be valid");
let string_key = CString::new("schema/name").expect("custom key should be valid");
let string_value = CString::new("Frontend Luna").expect("custom value should be valid");
assert_eq!(
    unsafe { customize_bool(settings, bool_key.as_ptr(), TRUE) },
    TRUE
);
assert_eq!(
    unsafe { customize_int(settings, int_key.as_ptr(), 9) },
    TRUE
);
assert_eq!(
    unsafe { customize_string(settings, string_key.as_ptr(), string_value.as_ptr()) },
    TRUE
);
assert_eq!(unsafe { settings_is_modified(settings) }, TRUE);
assert_eq!(unsafe { save_settings(settings) }, TRUE);
assert_eq!(unsafe { settings_is_modified(settings) }, FALSE);
```

**Notification/deploy pattern** (lines 923-1054): use `frontend_style_api_table_can_receive_runtime_notifications` as the model for TypeDuck-Web worker notification handler replacement and deploy/maintenance observations.

**Apply to Phase 6:** model TypeDuck-Web through ABI calls Yune owns. Do not vendor TypeDuck-Web unless a later plan explicitly justifies it; use wrapper-shaped call sequences and capture browser/WASM-specific blockers as trace/blocker artifacts.

---

### `crates/yune-rime-api/tests/frontend_hosts/native_frontends.rs` (test / source-modeled fixture, request-response + event-driven)

**Analog:** `crates/yune-rime-api/src/tests/lifecycle_safety.rs`

**Repeated initialize/finalize pattern** (lines 4-44):
```rust
#[test]
fn lifecycle_safety_repeated_setup_initialize_finalize_is_deterministic() {
    let _guard = test_guard();

    for iteration in 0..3 {
        let root = unique_temp_dir(&format!("lifecycle-repeated-{iteration}"));
        let shared = root.join("shared");
        let user = root.join("user");
        fs::create_dir_all(&shared).expect("shared dir should be created");
        fs::create_dir_all(&user).expect("user dir should be created");

        let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
        let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
        let mut traits = empty_traits();
        traits.shared_data_dir = shared_c.as_ptr();
        traits.user_data_dir = user_c.as_ptr();

        unsafe { RimeSetup(&traits) };
        unsafe { RimeInitialize(&traits) };
        let session_id = RimeCreateSession();
        assert_ne!(
            session_id, 0,
            "iteration {iteration} creates a usable session"
        );
        assert_eq!(RimeFindSession(session_id), TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        assert_eq!(RimeFindSession(session_id), FALSE);

        RimeFinalize();
        assert_eq!(
            RimeCreateSession(),
            0,
            "finalize stops new session creation"
        );
        assert_eq!(RimeFindSession(session_id), FALSE);
        fs::remove_dir_all(root).expect("temp dirs should be removed");
    }

    let reset_traits = empty_traits();
    unsafe { RimeInitialize(&reset_traits) };
}
```

**Stale handle pattern** (lines 46-75):
```rust
#[test]
fn lifecycle_safety_repeated_session_destroy_and_cleanup_reject_stale_handles() {
    let _guard = test_guard();

    for iteration in 0..3 {
        RimeCleanupAllSessions();
        let first = RimeCreateSession();
        assert_ne!(first, 0, "iteration {iteration} creates first session");
        assert_eq!(RimeFindSession(first), TRUE);
        assert_eq!(RimeDestroySession(first), TRUE);
        assert_eq!(RimeDestroySession(first), FALSE);
        assert_eq!(RimeFindSession(first), FALSE);

        let second = RimeCreateSession();
        assert_ne!(second, 0, "iteration {iteration} creates second session");
        assert_eq!(RimeFindSession(second), TRUE);
        RimeCleanupAllSessions();
        assert_eq!(RimeFindSession(second), FALSE);

        let after_cleanup = RimeCreateSession();
        assert_ne!(
            after_cleanup, 0,
            "iteration {iteration} can create after cleanup-all"
        );
        assert_ne!(after_cleanup, first, "stale handles are not reused");
        assert_ne!(after_cleanup, second, "cleanup handles are not reused");
        assert_eq!(RimeFindSession(after_cleanup), TRUE);
        assert_eq!(RimeDestroySession(after_cleanup), TRUE);
    }
}
```

**Notification replacement pattern** (lines 182-223):
```rust
#[test]
fn lifecycle_safety_notification_handler_replacement_and_clearing_are_deterministic() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();

    let session_id = RimeCreateSession();
    let ascii_mode = CString::new("ascii_mode").expect("option name is valid");

    RimeSetNotificationHandler(Some(record_notification), 0x11_usize as *mut c_void);
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    RimeSetNotificationHandler(Some(record_notification), 0x22_usize as *mut c_void);
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
    RimeSetNotificationHandler(None, ptr::null_mut());
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned");
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x11,
                session_id,
                message_type: "option".to_owned(),
                message_value: "ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x22,
                session_id,
                message_type: "option".to_owned(),
                message_value: "!ascii_mode".to_owned(),
            },
        ]
    );
    drop(events);
    assert_eq!(RimeDestroySession(session_id), TRUE);
}
```

**Apply to Phase 6:** Squirrel/macOS and Linux source-modeled tests should start as lifecycle fixtures around these exact behaviors: app-level setup/init/finalize, per-input-context sessions, stale-session rejection, focus/reset ordering if observed, and notification callback replacement.

---

### `fixtures/frontend-traces/*.json` or `docs/frontend-validation/*.md` (fixture / documentation artifact, file-I/O + event-driven trace)

**Analog:** `crates/yune-cli/src/transcript.rs`

**Deterministic transcript shape pattern** (lines 25-72):
```rust
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
```

**Event JSON pattern** (lines 284-342):
```rust
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
```

**Environment-independent assertion pattern** (lines 677-705):
```rust
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
```

**Apply to Phase 6:** trace fixtures should omit local paths, timestamps, process IDs, raw pointers, Cargo env vars, and personal user data. If markdown blocker notes are used instead, keep the same fields: target, attempted command/environment, call sequence, expected behavior, observed behavior, blocker, and reproduction status.

---

### `crates/yune-rime-api/benches/frontend_baselines.rs` or equivalent benchmark target (benchmark harness, batch + request-response + file-I/O)

**Analog:** `crates/yune-rime-api/tests/frontend_client.rs` plus `crates/yune-rime-api/src/tests/userdb.rs`

**ABI lifecycle setup pattern for benchmarks** (lines 1219-1316 in `frontend_client.rs`):
```rust
#[test]
fn frontend_style_api_table_can_run_deployment_and_maintenance() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let deployer_initialize = api
        .deployer_initialize
        .expect("frontend requires deployer_initialize");
    let start_maintenance = api
        .start_maintenance
        .expect("frontend requires start_maintenance");
    let is_maintenance_mode = api
        .is_maintenance_mode
        .expect("frontend requires is_maintenance_mode");
    let join_maintenance_thread = api
        .join_maintenance_thread
        .expect("frontend requires join_maintenance_thread");
    let prebuild = api.prebuild.expect("frontend requires prebuild");
    let deploy = api.deploy.expect("frontend requires deploy");
    let deploy_schema = api.deploy_schema.expect("frontend requires deploy_schema");
    let deploy_config_file = api
        .deploy_config_file
        .expect("frontend requires deploy_config_file");
    let run_task = api.run_task.expect("frontend requires run_task");
    let sync_user_data = api
        .sync_user_data
        .expect("frontend requires sync_user_data");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    let cleanup_stale_sessions = api
        .cleanup_stale_sessions
        .expect("frontend requires cleanup_stale_sessions");
    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let find_session = api.find_session.expect("frontend requires find_session");
```

**Userdb learning/sync benchmark scenario pattern** (lines 290-392 in `src/tests/userdb.rs`):
```rust
#[test]
fn userdb_sync_merges_plain_snapshots_and_backs_up_current_state() {
    let _guard = test_guard();
    let root = unique_temp_dir("rime-sync-user-data");
    let user = root.join("user");
    let peer_sync = user.join("sync").join("peer");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::create_dir_all(&peer_sync).expect("peer sync dir should be created");

    fs::write(
        user.join("luna_pinyin.userdb"),
        "ni hao\t你好\t1\nshuo\t说\t1\n",
    )
    .expect("local user dict should be written");
    fs::write(
        peer_sync.join("luna_pinyin.userdb.txt"),
        "# Rime user dictionary\n/db_name\tluna_pinyin\n/db_type\tuserdb\n/tick\t5\n/user_id\tpeer\nni hao\t你好\tc=4 d=4 t=2\nshuo\t说\tc=-7 d=7 t=3\nzhong guo\t中国\tc=2 d=2 t=5\n",
    )
    .expect("peer snapshot should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    assert_eq!(RimeFindSession(session_id), TRUE);
    assert_eq!(RimeSyncUserData(), TRUE);
    assert_eq!(RimeFindSession(session_id), FALSE);
```

**Config dependency guidance:** `crates/yune-rime-api/Cargo.toml` already has `libloading = "0.8"` as a dev-dependency (lines 17-18). If adding Criterion, verify Rust 1.76 compatibility first; otherwise follow the repository's dependency-free test style and write a small timing harness with `std::time` and deterministic output.

**Apply to Phase 6:** benchmark the ABI layer (`rime_get_api` / `RimeApi` function table), not `Engine::process_key_event` directly. Include session create/destroy, per-key simple ASCII, schema-loaded dictionary lookup with context/status/commit/free, deploy/dictionary loading, and userdb learning/sync. Serialize global-state benchmarks with a test/bench guard.

---

### `crates/yune-rime-api/Cargo.toml` (config, build config)

**Analog:** `crates/yune-rime-api/Cargo.toml`

**Current crate-type and dev-dependency pattern** (lines 8-18):
```toml
[lib]
crate-type = ["rlib", "cdylib"]

[dependencies]
libc = "0.2"
regex = "1"
serde_yaml = "0.9"
yune-core = { path = "../yune-core" }

[dev-dependencies]
libloading = "0.8"
```

**Workspace MSRV pattern:** root `Cargo.toml` lines 9-13:
```toml
[workspace.package]
edition = "2021"
license = "BSD-3-Clause"
repository = "https://github.com/yune-ime/yune"
rust-version = "1.76"
```

**Apply to Phase 6:** any benchmark dependency must respect Rust 1.76. Prefer no dependency if Criterion's current MSRV is incompatible. Keep `crate-type = ["rlib", "cdylib"]` intact because native loader validation depends on the cdylib artifact.

---

### `docs/real-frontend-validation-plan.md` (documentation, transform / decision record)

**Analog:** `docs/compat-foundation-summary.md`

**Concise status summary pattern** (lines 4-10):
```markdown
## Completed scope

- CLI frontend surrogate: `yune-cli` can drive setup, deployment, schema selection, session lifecycle, key processing, rendering, and transcript replay through the RIME ABI path.
- Native ABI validation: a dynamic-loader integration path exercises the built cdylib and locks struct layout, API table, runtime lifecycle, notification, module, and session behavior.
- Schema pipeline depth: processor, segmentor, translator, filter, spelling algebra, correction/tolerance, OpenCC, and larger schema-chain behaviors have focused compatibility coverage or explicit deferrals.
- Compiled dictionary data: runtime dictionary loading and rebuild behavior consume compiled table, prism, reverse, stem, dict settings, preset vocabulary, encoder, correction, and tolerance data where the current compatibility slice requires it.
- User dictionary behavior: userdb storage, typed records, backup/restore, sync, recovery, transaction rollback, runtime learning, frequency updates, predictive lookup, and frontend-style persistence are implemented and covered by focused tests.
```

**Explicit boundaries pattern** (lines 12-18):
```markdown
## Explicit boundaries

- Yune is not a full librime clone yet; the implemented surface is the measured compatibility foundation needed for the current Rust ABI and frontend-surrogate workflows.
- The userdb implementation is a typed file-backed compatibility abstraction, not full LevelDB binary compatibility.
- The C++ plugin ABI, Lua, octagram, predict, proto, and broader librime plugin ecosystem remain out of scope.
- A real native frontend integration is not complete; current validation uses CLI and native frontend-like loader paths.
- AI-native candidates, ranking, context policy, memory policy, and privacy controls are intentionally deferred to a separate product layer.
```

**Apply to Phase 6:** update docs only with evidence/outcomes: TypeDuck-Web runnable reproduction or blocker, Squirrel/macOS attempt or blocker, Linux scope, benchmark baseline summary, and AI-native go/no-go. Keep AI-native implementation out of this doc.

## Shared Patterns

### Global-state test serialization
**Source:** `crates/yune-rime-api/tests/frontend_client.rs` lines 136-149 and `crates/yune-rime-api/tests/dynamic_loader.rs` lines 132-138
**Apply to:** All host-shaped validation tests and benchmarks
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

### ABI memory ownership and free pairing
**Source:** `crates/yune-rime-api/src/context_api.rs` lines 9-16, 44-52, 236-243 and `crates/yune-rime-api/src/ffi_memory.rs` lines 290-351
**Apply to:** Every scenario that calls `get_commit`, `get_context`, `get_status`, schema lists, candidate iterators, or levers iterators
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
```

```rust
#[no_mangle]
pub unsafe extern "C" fn RimeFreeContext(context: *mut RimeContext) -> Bool {
    if context.is_null() {
        return FALSE;
    }
    // SAFETY: `context` is non-null and points to caller-owned storage.
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    free_context_fields(context);
    clear_context(context);
    TRUE
}
```

### C ABI safety comments
**Source:** `crates/yune-rime-api/src/lib.rs` lines 560-588 and `crates/yune-rime-api/tests/dynamic_loader.rs` lines 267-284
**Apply to:** Any new unsafe pointer dereference, `CStr::from_ptr`, dynamic library load, or symbol resolution
```rust
/// Sets the current raw composition input for a session.
///
/// # Safety
///
/// `input` must be either null or a valid NUL-terminated C string. Null input
/// is rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeSetInput(session_id: RimeSessionId, input: *const c_char) -> Bool {
    if session_id == 0 || input.is_null() {
        return FALSE;
    }

    // SAFETY: `input` is non-null and caller promises a valid NUL-terminated C
    // string.
    let Ok(input) = unsafe { CStr::from_ptr(input) }.to_str() else {
        return FALSE;
    };
```

### Logical resource IDs, not filesystem paths
**Source:** `crates/yune-rime-api/src/userdb/mod.rs` lines 152-176 and `crates/yune-rime-api/src/deployment.rs` lines 1174-1179, 1428-1458
**Apply to:** Any validation fixture that names schema/config/dictionary/userdb IDs
```rust
pub(crate) fn user_dict_path(dict_name: &str) -> Option<PathBuf> {
    let dict_name = validate_user_dict_name(dict_name)?;
    Some(runtime_user_data_dir().join(format!("{dict_name}.userdb")))
}

pub(crate) fn open_store(dict_name: &str) -> io::Result<FileUserDbStore> {
    let Some(dict_name) = validate_user_dict_name(dict_name) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid user dictionary name",
        ));
    };
```

```rust
pub(crate) fn deploy_config_file(file_name: &str, version_key: &str) -> bool {
    let Some(file_name) = validate_data_resource_id(file_name) else {
        return false;
    };
    if version_key.is_empty() {
        return false;
    }
```

### Function table construction and required entries
**Source:** `crates/yune-rime-api/src/api_table.rs` lines 62-163
**Apply to:** Host trace required-function lists and benchmark setup
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
```

### Deployment and userdb benchmark boundaries
**Source:** `crates/yune-rime-api/src/deployment.rs` lines 123-133 and `crates/yune-rime-api/src/userdb/sync.rs` lines 14-35
**Apply to:** Benchmark and validation scenarios for deploy, dictionary loading, sync, and learning
```rust
#[no_mangle]
pub extern "C" fn RimeSyncUserData() -> Bool {
    RimeCleanupAllSessions();
    crate::notify(0, "deploy", "start");
    let installation_synced = run_installation_update();
    let configs_synced = backup_config_files();
    let user_dicts_synced = sync_all_user_dicts();
    let success = installation_synced && configs_synced && user_dicts_synced;
    crate::notify(0, "deploy", if success { "success" } else { "failure" });
    bool_from(success)
}
```

```rust
pub(crate) fn sync_user_dict(dict_name: &str) -> bool {
    if validate_user_dict_name(dict_name).is_none() {
        return false;
    }
    let mut success = true;
    for snapshot in peer_user_dict_snapshots(dict_name) {
        if restore_snapshot(&snapshot).is_err() {
            success = false;
        }
    }
    backup_user_dict(dict_name) && success
}
```

## No Analog Found

All likely Phase 6 files have close in-repository analogs. There is no existing real TypeDuck-Web, Squirrel, ibus, or fcitx integration file; planner should use the source-modeled ABI fixture patterns above plus `06-RESEARCH.md` for external wrapper details rather than expecting an in-repo frontend adapter.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| External TypeDuck-Web/Squirrel/ibus/fcitx integration scripts, if added | optional external validation artifact | file-I/O + event-driven | No existing real frontend integration scripts exist; normal tests should remain source-modeled and not require external frontend toolchains. |

## Metadata

**Analog search scope:** `/Users/trenton/Projects/yune/crates`, `/Users/trenton/Projects/yune/docs`, `/Users/trenton/Projects/yune/fixtures`, `/Users/trenton/Projects/yune/.planning/codebase`
**Files scanned:** 80+ source/test/doc/fixture paths listed under the repository source tree
**Pattern extraction date:** 2026-05-01
