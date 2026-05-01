use std::{
    ffi::{c_void, CStr, CString},
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{rime_get_api, RimeApi, RimeContext, RimeSessionId, RimeTraits, FALSE, TRUE};

use super::{empty_commit, empty_context, empty_status, empty_traits, FrontendHostTrace};

const SQUIRREL_TARGET: &str = "squirrel_macos_source_model";
const SQUIRREL_SCENARIO: &str = "squirrel_lifecycle";
const SQUIRREL_SCHEMA: &str = "squirrel_luna";

#[derive(Clone, Debug, Eq, PartialEq)]
struct NotificationEvent {
    handler: String,
    session_id: RimeSessionId,
    message_type: String,
    message_value: String,
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("native frontend host test lock should not be poisoned")
}

fn notification_events() -> &'static Mutex<Vec<NotificationEvent>> {
    static NOTIFICATION_EVENTS: OnceLock<Mutex<Vec<NotificationEvent>>> = OnceLock::new();
    NOTIFICATION_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

extern "C" fn record_notification_primary(
    _context_object: *mut c_void,
    session_id: RimeSessionId,
    message_type: *const c_char,
    message_value: *const c_char,
) {
    record_notification(
        "squirrel_app_handler_primary",
        session_id,
        message_type,
        message_value,
    );
}

extern "C" fn record_notification_replacement(
    _context_object: *mut c_void,
    session_id: RimeSessionId,
    message_type: *const c_char,
    message_value: *const c_char,
) {
    record_notification(
        "squirrel_app_handler_replacement",
        session_id,
        message_type,
        message_value,
    );
}

fn record_notification(
    handler: &str,
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
            handler: handler.to_owned(),
            session_id,
            message_type,
            message_value,
        });
}

pub(crate) fn squirrel_lifecycle_is_source_modeled_through_yune_abi() {
    let _guard = test_guard();
    let api = unsafe {
        let api = rime_get_api();
        assert!(
            !api.is_null(),
            "Squirrel source model requires rime_get_api"
        );
        &*api
    };
    let trace = run_squirrel_lifecycle(api);

    assert_eq!(trace.target, SQUIRREL_TARGET);
    assert_eq!(trace.scenario, SQUIRREL_SCENARIO);
    assert_eq!(trace.resource_ids, vec![SQUIRREL_SCHEMA.to_owned()]);
    assert!(trace
        .calls
        .iter()
        .any(|call| call.name == "squirrel_focus_in_create_session"));
    assert!(trace
        .calls
        .iter()
        .any(|call| call.name == "squirrel_focus_out_clear_composition"));
    assert!(trace
        .calls
        .iter()
        .any(|call| call.name == "linux_followup.focus_reset_ordering"));
    assert!(trace.free_pairs.iter().any(|pair| {
        pair.get_call == "squirrel_get_status" && pair.free_call == "squirrel_free_status"
    }));
    assert!(trace.free_pairs.iter().any(|pair| {
        pair.get_call == "squirrel_get_context" && pair.free_call == "squirrel_free_context"
    }));
    assert!(trace.free_pairs.iter().any(|pair| {
        pair.get_call == "squirrel_get_commit" && pair.free_call == "squirrel_free_commit"
    }));
    trace.assert_sanitized();
}

pub(crate) fn squirrel_fixture_json() -> String {
    let _guard = test_guard();
    let api = unsafe {
        let api = rime_get_api();
        assert!(
            !api.is_null(),
            "Squirrel source model requires rime_get_api"
        );
        &*api
    };
    run_squirrel_lifecycle(api).to_json()
}

pub(crate) fn assert_squirrel_fixture_contract(fixture: &str) {
    super::assert_json_is_sanitized(fixture);
    assert!(fixture.contains("\"target\": \"squirrel_macos_source_model\""));
    assert!(fixture.contains("\"scenario\": \"squirrel_lifecycle\""));
    assert!(fixture.contains("\"source_model\": \"Squirrel macOS app and input-context lifecycle modeled at the RIME ABI boundary\""));
    assert!(fixture.contains("\"call_sequence\":"));
    assert!(fixture.contains("\"squirrel_focus_in_create_session\""));
    assert!(fixture.contains("\"squirrel_focus_out_clear_composition\""));
    assert!(fixture.contains("\"linux_followup.ibus_focus_reset_ordering\""));
    assert!(fixture.contains("\"linux_followup.fcitx_surrounding_text_scope\""));
    assert!(fixture.contains("\"blocker_or_gap\": \"documented_blocker"));
    assert!(
        fixture.contains("\"reproduction_status\": \"minimized_fixture_plus_documented_blocker\"")
    );
}

fn run_squirrel_lifecycle(api: &RimeApi) -> FrontendHostTrace {
    let mut trace = FrontendHostTrace::new(SQUIRREL_TARGET, SQUIRREL_SCENARIO);
    trace.resource_ids = vec![SQUIRREL_SCHEMA.to_owned()];
    trace.mismatch.expected_behavior = "Squirrel/macOS app-level setup, per-input-context sessions, key processing, context/status/commit reads, focus cleanup, notifications, stale-session rejection, and reinitialize teardown remain reproducible through Yune RimeApi calls".to_owned();
    trace.mismatch.observed_behavior = "direct Squirrel app integration was not made mandatory; the source-modeled ABI fixture completed through Yune RimeApi and preserved the direct-run blocker for later frontend verification".to_owned();
    trace.call_text(
        "source_model",
        "Squirrel macOS app and input-context lifecycle modeled at the RIME ABI boundary",
    );
    trace.call_text("attempted_environment", "macOS source-model regression without requiring installed Squirrel app, Xcode GUI automation, or input-method registration");
    trace.call_text("blocker_or_gap", "documented_blocker: direct Squirrel bundle integration is outside ordinary cargo test and requires local app/input-method packaging");
    trace.call_text(
        "reproduction_status",
        "minimized_fixture_plus_documented_blocker",
    );
    trace.call_text("rime_get_api", "resolved");
    trace.call_bool("validate_api_data_size", (api.data_size > 0) as c_int);

    let setup = require(&mut trace, "setup", api.setup);
    let set_notification_handler = require(
        &mut trace,
        "set_notification_handler",
        api.set_notification_handler,
    );
    let initialize = require(&mut trace, "initialize", api.initialize);
    let finalize = require(&mut trace, "finalize", api.finalize);
    let deployer_initialize = require(&mut trace, "deployer_initialize", api.deployer_initialize);
    let start_maintenance = require(&mut trace, "start_maintenance", api.start_maintenance);
    let join_maintenance_thread = require(
        &mut trace,
        "join_maintenance_thread",
        api.join_maintenance_thread,
    );
    let deploy = require(&mut trace, "deploy", api.deploy);
    let create_session = require(&mut trace, "create_session", api.create_session);
    let find_session = require(&mut trace, "find_session", api.find_session);
    let destroy_session = require(&mut trace, "destroy_session", api.destroy_session);
    let cleanup_all_sessions =
        require(&mut trace, "cleanup_all_sessions", api.cleanup_all_sessions);
    let select_schema = require(&mut trace, "select_schema", api.select_schema);
    let process_key = require(&mut trace, "process_key", api.process_key);
    let clear_composition = require(&mut trace, "clear_composition", api.clear_composition);
    let commit_composition = require(&mut trace, "commit_composition", api.commit_composition);
    let get_input = require(&mut trace, "get_input", api.get_input);
    let get_status = require(&mut trace, "get_status", api.get_status);
    let free_status = require(&mut trace, "free_status", api.free_status);
    let get_context = require(&mut trace, "get_context", api.get_context);
    let free_context = require(&mut trace, "free_context", api.free_context);
    let get_commit = require(&mut trace, "get_commit", api.get_commit);
    let free_commit = require(&mut trace, "free_commit", api.free_commit);
    let set_option = require(&mut trace, "set_option", api.set_option);
    let sync_user_data = require(&mut trace, "sync_user_data", api.sync_user_data);

    cleanup_all_sessions();
    trace.call_bool("cleanup_all_sessions", TRUE);

    let root = unique_temp_dir("squirrel-runtime");
    let runtime = SquirrelRuntime::create(&root);
    write_squirrel_schema(&runtime.shared, &runtime.staging);
    let traits = runtime.traits();

    // SAFETY: the C strings referenced by traits are owned by `runtime` and kept
    // alive until all setup/deployer/initialize calls finish.
    unsafe { setup(&traits) };
    trace.call_bool("squirrel_app_setup", TRUE);
    // SAFETY: same C string lifetime guarantee as setup.
    unsafe { deployer_initialize(&traits) };
    trace.call_bool("squirrel_deployer_initialize", TRUE);

    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
    set_notification_handler(Some(record_notification_primary), ptr::null_mut());
    trace.call_text("squirrel_set_notification_handler", "primary");

    // SAFETY: the C strings referenced by traits are owned by `runtime` and kept
    // alive while initialize reads the host-provided runtime paths.
    unsafe { initialize(&traits) };
    trace.call_bool("squirrel_app_initialize", TRUE);
    let maintenance_result = start_maintenance(TRUE);
    trace.call_bool("squirrel_start_maintenance", maintenance_result);
    join_maintenance_thread();
    trace.call_bool("squirrel_join_maintenance_thread", TRUE);
    let deploy_result = deploy();
    trace.call_bool("squirrel_deploy", deploy_result);

    let first_session = create_session();
    assert_ne!(first_session, 0, "Squirrel focus-in creates a session");
    trace.call_number("squirrel_focus_in_create_session", 1);
    trace.call_bool("squirrel_find_session", find_session(first_session));
    let schema_id = CString::new(SQUIRREL_SCHEMA).expect("schema id should be valid");
    // SAFETY: schema_id is a valid NUL-terminated logical schema ID and lives for
    // the duration of the call.
    let schema_selected = unsafe { select_schema(first_session, schema_id.as_ptr()) };
    assert_eq!(
        schema_selected, TRUE,
        "Squirrel source model selects schema"
    );
    trace.call_bool("squirrel_select_schema", schema_selected);

    let first_key = process_key(first_session, 'b' as c_int, 0);
    let second_key = process_key(first_session, 'a' as c_int, 0);
    assert_eq!(first_key, TRUE, "Squirrel source model processes first key");
    assert_eq!(
        second_key, TRUE,
        "Squirrel source model processes second key"
    );
    trace.call_bool("squirrel_process_key_b", first_key);
    trace.call_bool("squirrel_process_key_a", second_key);

    let input = get_input(first_session);
    assert!(!input.is_null(), "Squirrel source model reads input");
    // SAFETY: get_input returned a non-null session-owned C string that remains
    // valid until the session input buffer is replaced or the session is destroyed.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ba"));
    trace.call_text("squirrel_get_input", "ba");

    read_status(
        &mut trace,
        first_session,
        get_status,
        free_status,
        "squirrel_get_status",
        "squirrel_free_status",
        true,
    );
    read_context(
        &mut trace,
        first_session,
        ContextReadApi {
            get_context,
            free_context,
        },
        ContextReadExpectation {
            get_call: "squirrel_get_context",
            free_call: "squirrel_free_context",
            expected_page_size: 2,
            min_candidates: 1,
        },
    );

    let committed = commit_composition(first_session);
    assert_eq!(committed, TRUE, "Squirrel source model commits composition");
    trace.call_bool("squirrel_commit_composition", committed);
    read_commit(
        &mut trace,
        first_session,
        get_commit,
        free_commit,
        "squirrel_get_commit",
        "squirrel_free_commit",
        "八",
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: ascii_mode is a valid NUL-terminated option ID and lives for the
    // duration of the call.
    unsafe { set_option(first_session, ascii_mode.as_ptr(), TRUE) };
    trace.call_bool("squirrel_set_option_primary_handler", TRUE);
    set_notification_handler(Some(record_notification_replacement), ptr::null_mut());
    trace.call_text("squirrel_set_notification_handler", "replacement");
    // SAFETY: same valid option ID lifetime as above.
    unsafe { set_option(first_session, ascii_mode.as_ptr(), FALSE) };
    trace.call_bool("squirrel_set_option_replacement_handler", TRUE);
    set_notification_handler(None, ptr::null_mut());
    trace.call_text("squirrel_set_notification_handler", "cleared");
    // SAFETY: same valid option ID lifetime as above.
    unsafe { set_option(first_session, ascii_mode.as_ptr(), TRUE) };
    trace.call_bool("squirrel_set_option_after_clear", TRUE);

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clone();
    assert!(events
        .iter()
        .any(|event| event.handler == "squirrel_app_handler_primary"));
    assert!(events
        .iter()
        .any(|event| event.handler == "squirrel_app_handler_replacement"));
    for event in &events {
        let session = if event.session_id == first_session {
            "input_context_primary"
        } else {
            "app_global"
        };
        trace.record_notification(
            &event.handler,
            session,
            &event.message_type,
            &event.message_value,
        );
    }

    clear_composition(first_session);
    trace.call_bool("squirrel_focus_out_clear_composition", TRUE);
    read_status(
        &mut trace,
        first_session,
        get_status,
        free_status,
        "squirrel_focus_out_get_status",
        "squirrel_focus_out_free_status",
        false,
    );
    let destroyed_first = destroy_session(first_session);
    assert_eq!(destroyed_first, TRUE, "Squirrel focus-out destroys session");
    trace.call_bool("squirrel_focus_out_destroy_session", destroyed_first);
    let stale_find = find_session(first_session);
    trace.record_stale_session(
        "squirrel_focus_out_destroy_session",
        "find_session",
        stale_find == TRUE,
    );
    assert_eq!(stale_find, FALSE);
    let stale_context = unsafe {
        let mut context = empty_context();
        get_context(first_session, &mut context)
    };
    trace.record_stale_session(
        "squirrel_focus_out_destroy_session",
        "get_context",
        stale_context == TRUE,
    );
    assert_eq!(stale_context, FALSE);

    let second_session = create_session();
    assert_ne!(
        second_session, 0,
        "second Squirrel input context creates a session"
    );
    trace.call_number("squirrel_second_context_create_session", 2);
    // SAFETY: schema_id is still a valid NUL-terminated logical schema ID and
    // lives for the duration of the call.
    let selected_second = unsafe { select_schema(second_session, schema_id.as_ptr()) };
    assert_eq!(selected_second, TRUE, "second context selects schema");
    trace.call_bool("squirrel_second_context_select_schema", selected_second);
    let second_key = process_key(second_session, 'b' as c_int, 0);
    assert_eq!(second_key, TRUE, "second context processes key");
    trace.call_bool("squirrel_second_context_process_key", second_key);
    trace.call_text(
        "linux_followup.focus_reset_ordering",
        "source-modeled expectation: focus-out clears composition before session destroy; future ibus/fcitx runs must preserve call sequence before fixes",
    );
    trace.call_text(
        "linux_followup.ibus_focus_reset_ordering",
        "ibus-rime follow-up should validate focus-in/create, reset/clear, process-key/filter-key, status/context/commit reads, and destroy ordering under an ibus daemon",
    );
    trace.call_text(
        "linux_followup.fcitx_surrounding_text_scope",
        "fcitx-rime follow-up should validate focus-in/create, reset/clear, process-key, candidate UI reads, surrounding-text limitations, and notification propagation under fcitx5",
    );
    let destroyed_second = destroy_session(second_session);
    assert_eq!(destroyed_second, TRUE, "second context destroys session");
    trace.call_bool("squirrel_second_context_destroy_session", destroyed_second);

    let sync_result = sync_user_data();
    trace.call_bool("squirrel_sync_user_data", sync_result);
    let create_after_sync = create_session();
    trace.record_stale_session("sync_user_data", "create_session", create_after_sync != 0);
    assert_ne!(
        create_after_sync, 0,
        "sync cleanup preserves service startup for new sessions"
    );
    trace.call_number("squirrel_create_session_after_sync", 3);
    assert_eq!(destroy_session(create_after_sync), TRUE);
    trace.call_bool("squirrel_destroy_after_sync", TRUE);

    // SAFETY: traits strings remain valid for repeated initialize after sync cleanup.
    unsafe { initialize(&traits) };
    trace.call_bool("squirrel_reinitialize_after_sync", TRUE);
    let after_sync_session = create_session();
    assert_ne!(
        after_sync_session, 0,
        "reinitialize after sync permits sessions again"
    );
    trace.call_number("squirrel_create_session_after_reinitialize", 4);
    assert_eq!(destroy_session(after_sync_session), TRUE);
    trace.call_bool("squirrel_destroy_after_reinitialize", TRUE);
    finalize();
    trace.call_bool("squirrel_finalize", TRUE);
    let create_after_finalize = create_session();
    trace.record_stale_session("finalize", "create_session", create_after_finalize != 0);
    assert_eq!(create_after_finalize, 0);
    let find_after_finalize = find_session(after_sync_session);
    trace.record_stale_session("finalize", "find_session", find_after_finalize == TRUE);
    assert_eq!(find_after_finalize, FALSE);

    set_notification_handler(None, ptr::null_mut());
    trace.call_text("squirrel_teardown_notification_handler", "cleared");
    let reset_traits = empty_traits();
    // SAFETY: null/default traits are accepted by setup to restore default runtime paths.
    unsafe { setup(&reset_traits) };
    trace.call_bool("squirrel_teardown_setup_reset", TRUE);
    fs::remove_dir_all(&root).expect("temp dirs should be removed");
    trace.call_bool("squirrel_teardown_remove_runtime", TRUE);

    trace.assert_sanitized();
    trace
}

fn require<T>(trace: &mut FrontendHostTrace, name: &str, function: Option<T>) -> T {
    trace.record_function(name, function.is_some());
    function.unwrap_or_else(|| panic!("native frontend required RimeApi entry is missing: {name}"))
}

fn read_status(
    trace: &mut FrontendHostTrace,
    session_id: RimeSessionId,
    get_status: unsafe extern "C" fn(RimeSessionId, *mut yune_rime_api::RimeStatus) -> c_int,
    free_status: unsafe extern "C" fn(*mut yune_rime_api::RimeStatus) -> c_int,
    get_call: &str,
    free_call: &str,
    expected_composing: bool,
) {
    let mut status = empty_status();
    // SAFETY: status points to caller-owned writable storage and is freed by the
    // matching free_status call before the object is discarded.
    let status_result = unsafe { get_status(session_id, &mut status) };
    assert_eq!(
        status_result, TRUE,
        "native frontend source model reads status"
    );
    assert_eq!(status.is_composing == TRUE, expected_composing);
    let status_ptr = &mut status as *mut _ as usize;
    trace.call_bool(get_call, status_result);
    // SAFETY: free_status receives the same caller-owned status object returned by get_status.
    let free_status_result = unsafe { free_status(&mut status) };
    assert_eq!(
        free_status_result, TRUE,
        "native frontend source model frees status"
    );
    trace.call_bool(free_call, free_status_result);
    trace.record_free_pair(
        get_call,
        free_call,
        status_ptr == &mut status as *mut _ as usize,
    );
}

struct ContextReadApi {
    get_context: unsafe extern "C" fn(RimeSessionId, *mut RimeContext) -> c_int,
    free_context: unsafe extern "C" fn(*mut RimeContext) -> c_int,
}

struct ContextReadExpectation {
    get_call: &'static str,
    free_call: &'static str,
    expected_page_size: c_int,
    min_candidates: c_int,
}

fn read_context(
    trace: &mut FrontendHostTrace,
    session_id: RimeSessionId,
    api: ContextReadApi,
    expectation: ContextReadExpectation,
) {
    let mut context = empty_context();
    // SAFETY: context points to caller-owned writable storage and is freed by the
    // matching free_context call before pointer fields are discarded.
    let context_result = unsafe { (api.get_context)(session_id, &mut context) };
    assert_eq!(
        context_result, TRUE,
        "native frontend source model reads context"
    );
    assert_eq!(context.menu.page_size, expectation.expected_page_size);
    assert!(context.menu.num_candidates >= expectation.min_candidates);
    let context_ptr = &mut context as *mut _ as usize;
    trace.call_bool(expectation.get_call, context_result);
    // SAFETY: free_context receives the same caller-owned context object returned by get_context.
    let free_context_result = unsafe { (api.free_context)(&mut context) };
    assert_eq!(
        free_context_result, TRUE,
        "native frontend source model frees context"
    );
    trace.call_bool(expectation.free_call, free_context_result);
    trace.record_free_pair(
        expectation.get_call,
        expectation.free_call,
        context_ptr == &mut context as *mut _ as usize,
    );
}

fn read_commit(
    trace: &mut FrontendHostTrace,
    session_id: RimeSessionId,
    get_commit: unsafe extern "C" fn(RimeSessionId, *mut yune_rime_api::RimeCommit) -> c_int,
    free_commit: unsafe extern "C" fn(*mut yune_rime_api::RimeCommit) -> c_int,
    get_call: &str,
    free_call: &str,
    expected: &str,
) {
    let mut commit = empty_commit();
    // SAFETY: commit points to caller-owned writable storage and is freed by the
    // matching free_commit call before pointer fields are discarded.
    let commit_result = unsafe { get_commit(session_id, &mut commit) };
    assert_eq!(
        commit_result, TRUE,
        "native frontend source model reads commit"
    );
    assert!(!commit.text.is_null());
    // SAFETY: successful get_commit populated commit.text with a valid
    // NUL-terminated C string until free_commit is called.
    assert_eq!(
        unsafe { CStr::from_ptr(commit.text) }.to_str(),
        Ok(expected)
    );
    let commit_ptr = &mut commit as *mut _ as usize;
    trace.call_bool(get_call, commit_result);
    // SAFETY: free_commit receives the same caller-owned commit object returned by get_commit.
    let free_commit_result = unsafe { free_commit(&mut commit) };
    assert_eq!(
        free_commit_result, TRUE,
        "native frontend source model frees commit"
    );
    trace.call_bool(free_call, free_commit_result);
    trace.record_free_pair(
        get_call,
        free_call,
        commit_ptr == &mut commit as *mut _ as usize,
    );
}

struct SquirrelRuntime {
    shared: PathBuf,
    staging: PathBuf,
    _user: PathBuf,
    _prebuilt: PathBuf,
    shared_c: CString,
    user_c: CString,
    prebuilt_c: CString,
    staging_c: CString,
    distribution_name_c: CString,
    app_name_c: CString,
}

impl SquirrelRuntime {
    fn create(root: &Path) -> Self {
        let shared = root.join("shared");
        let user = root.join("squirrel-user");
        let prebuilt = shared.join("build");
        let staging = user.join("build");
        fs::create_dir_all(&shared).expect("shared dir should be created");
        fs::create_dir_all(&user).expect("user dir should be created");
        fs::create_dir_all(&prebuilt).expect("prebuilt dir should be created");
        fs::create_dir_all(&staging).expect("staging dir should be created");
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("shared path is valid");
        let user_c = CString::new(user.to_string_lossy().as_ref()).expect("user path is valid");
        let prebuilt_c =
            CString::new(prebuilt.to_string_lossy().as_ref()).expect("prebuilt path is valid");
        let staging_c =
            CString::new(staging.to_string_lossy().as_ref()).expect("staging path is valid");
        Self {
            shared,
            staging,
            _user: user,
            _prebuilt: prebuilt,
            shared_c,
            user_c,
            prebuilt_c,
            staging_c,
            distribution_name_c: CString::new("Squirrel macOS source model")
                .expect("distribution name is valid"),
            app_name_c: CString::new("squirrel").expect("app name is valid"),
        }
    }

    fn traits(&self) -> RimeTraits {
        let mut traits = empty_traits();
        traits.shared_data_dir = self.shared_c.as_ptr();
        traits.user_data_dir = self.user_c.as_ptr();
        traits.prebuilt_data_dir = self.prebuilt_c.as_ptr();
        traits.staging_dir = self.staging_c.as_ptr();
        traits.distribution_name = self.distribution_name_c.as_ptr();
        traits.app_name = self.app_name_c.as_ptr();
        traits
    }
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "yune-rime-api-frontend-host-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn write_squirrel_schema(shared: &Path, staging: &Path) {
    fs::write(
        staging.join("default.yaml"),
        "config_version: squirrel-source-model\nschema_list:\n  - schema: squirrel_luna\n",
    )
    .expect("Squirrel source model default config should be written");
    fs::write(
        staging.join("squirrel_luna.schema.yaml"),
        "\
schema:\n  schema_id: squirrel_luna\n  name: Squirrel Luna\nmenu:\n  page_size: 2\nswitches:\n  - name: ascii_mode\n    reset: 0\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: squirrel\n",
    )
    .expect("Squirrel source model schema config should be written");
    fs::write(
        shared.join("squirrel.dict.yaml"),
        "\
---\nname: squirrel\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nba\t八\t10\nba\t吧\t9\n",
    )
    .expect("Squirrel source model dictionary should be written");
}
