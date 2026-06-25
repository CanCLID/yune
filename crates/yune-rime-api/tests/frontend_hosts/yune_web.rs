use std::{
    ffi::{c_void, CStr, CString},
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{rime_get_api, RimeApi, RimeLeversApi, RimeSessionId, RimeTraits, FALSE, TRUE};

use super::{empty_commit, empty_context, empty_status, empty_traits, FrontendHostTrace};

const YUNE_WEB_TARGET: &str = "yune_web_browser_wasm_wrapper";
const YUNE_WEB_SCENARIO: &str = "yune_web_basic_lifecycle";
const YUNE_WEB_SCHEMA: &str = "yune_web_luna";

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
        .expect("yune-web host test lock should not be poisoned")
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
        "worker_handler_primary",
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
        "worker_handler_replacement",
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

pub(crate) fn yune_web_wrapper_lifecycle_is_validated_through_yune_abi() {
    let _guard = test_guard();
    let api = unsafe {
        let api = rime_get_api();
        assert!(!api.is_null(), "yune-web wrapper requires rime_get_api");
        &*api
    };
    let trace = run_yune_web_lifecycle(api);

    assert_eq!(trace.target, YUNE_WEB_TARGET);
    assert_eq!(trace.scenario, YUNE_WEB_SCENARIO);
    assert_eq!(trace.resource_ids, vec![YUNE_WEB_SCHEMA.to_owned()]);
    assert!(trace
        .calls
        .iter()
        .any(|call| call.name == "simulate_key_sequence"));
    assert!(trace
        .calls
        .iter()
        .any(|call| call.name == "candidate_list_begin"));
    assert!(trace
        .calls
        .iter()
        .any(|call| call.name == "delete_candidate_on_current_page"));
    assert!(trace
        .calls
        .iter()
        .any(|call| call.name == "levers.custom_settings_init"));
    assert!(trace.free_pairs.iter().any(|pair| {
        pair.get_call == "candidate_list_begin" && pair.free_call == "candidate_list_end"
    }));
    assert!(trace.free_pairs.iter().any(|pair| {
        pair.get_call == "levers.custom_settings_init"
            && pair.free_call == "levers.custom_settings_destroy"
    }));
    trace.assert_sanitized();
}

pub(crate) fn yune_web_basic_fixture_json() -> String {
    let _guard = test_guard();
    let api = unsafe {
        let api = rime_get_api();
        assert!(!api.is_null(), "yune-web wrapper requires rime_get_api");
        &*api
    };
    run_yune_web_lifecycle(api).to_json()
}

pub(crate) fn assert_yune_web_fixture_contract(fixture: &str) {
    super::assert_json_is_sanitized(fixture);
    assert!(fixture.contains("\"target\": \"yune_web_browser_wasm_wrapper\""));
    assert!(fixture.contains("\"scenario\": \"yune_web_basic_lifecycle\""));
    assert!(fixture.contains("\"resource_ids\": [\"yune_web_luna\"]"));
    assert!(fixture.contains("\"simulate_key_sequence\""));
    assert!(fixture.contains("\"browser_wasm_limit.emscripten_worker_lifecycle\""));
    assert!(fixture.contains("\"browser_wasm_limit.idbfs_persistence\""));
    assert!(fixture.contains("\"browser_wasm_limit.native_dynamic_loading\""));
    assert!(fixture.contains("\"classification\": \"match\""));
    assert!(fixture.contains("\"reproduction_status\": \"minimized_fixture\""));
}

fn run_yune_web_lifecycle(api: &RimeApi) -> FrontendHostTrace {
    let mut trace = FrontendHostTrace::new(YUNE_WEB_TARGET, YUNE_WEB_SCENARIO);
    trace.resource_ids = vec![YUNE_WEB_SCHEMA.to_owned()];
    trace.mismatch.expected_behavior = "yune-web-style browser/WebAssembly wrapper lifecycle maps to Yune-owned RimeApi calls without vendored frontend source".to_owned();
    trace.mismatch.observed_behavior = "minimized source-modeled setup, initialize, worker notification, maintenance, global session, key simulation, context/status/commit, candidate, levers, cleanup, and finalize calls completed through Yune RimeApi".to_owned();
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
    let _process_key = require(&mut trace, "process_key", api.process_key);
    let simulate_key_sequence = require(
        &mut trace,
        "simulate_key_sequence",
        api.simulate_key_sequence,
    );
    let get_status = require(&mut trace, "get_status", api.get_status);
    let free_status = require(&mut trace, "free_status", api.free_status);
    let get_context = require(&mut trace, "get_context", api.get_context);
    let free_context = require(&mut trace, "free_context", api.free_context);
    let get_commit = require(&mut trace, "get_commit", api.get_commit);
    let free_commit = require(&mut trace, "free_commit", api.free_commit);
    let get_input = require(&mut trace, "get_input", api.get_input);
    let select_candidate_on_current_page = require(
        &mut trace,
        "select_candidate_on_current_page",
        api.select_candidate_on_current_page,
    );
    let highlight_candidate = require(&mut trace, "highlight_candidate", api.highlight_candidate);
    let highlight_candidate_on_current_page = require(
        &mut trace,
        "highlight_candidate_on_current_page",
        api.highlight_candidate_on_current_page,
    );
    let change_page = require(&mut trace, "change_page", api.change_page);
    let delete_candidate_on_current_page = require(
        &mut trace,
        "delete_candidate_on_current_page",
        api.delete_candidate_on_current_page,
    );
    let candidate_list_begin =
        require(&mut trace, "candidate_list_begin", api.candidate_list_begin);
    let candidate_list_next = require(&mut trace, "candidate_list_next", api.candidate_list_next);
    let candidate_list_end = require(&mut trace, "candidate_list_end", api.candidate_list_end);

    cleanup_all_sessions();
    trace.call_bool("cleanup_all_sessions", TRUE);

    let root = unique_temp_dir("yune-web-runtime");
    let runtime = YuneWebRuntime::create(&root);
    write_yune_web_schema(&runtime.shared, &runtime.staging);
    let traits = runtime.traits();

    // SAFETY: the C strings referenced by traits are owned by `runtime` and kept
    // alive until setup, deployer_initialize, and initialize have finished.
    unsafe { setup(&traits) };
    trace.call_bool("setup_browser_virtual_paths", TRUE);
    // SAFETY: same C string lifetime guarantee as setup.
    unsafe { deployer_initialize(&traits) };
    trace.call_bool("deployer_initialize", TRUE);

    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
    set_notification_handler(Some(record_notification_primary), ptr::null_mut());
    trace.call_text("set_notification_handler", "worker_handler_primary");

    // SAFETY: the C strings referenced by traits are owned by `runtime` and kept
    // alive while initialize reads the browser-modeled runtime paths.
    unsafe { initialize(&traits) };
    trace.call_bool("initialize", TRUE);
    trace.call_text(
        "browser_wasm_limit.emscripten_worker_lifecycle",
        "modeled_only",
    );
    trace.call_text("browser_wasm_limit.idbfs_persistence", "modeled_only");
    trace.call_text(
        "browser_wasm_limit.native_dynamic_loading",
        "unavailable_in_browser",
    );

    let maintenance = start_maintenance(TRUE);
    trace.call_bool("start_maintenance", maintenance);
    join_maintenance_thread();
    trace.call_bool("join_maintenance_thread", TRUE);
    let deploy_result = deploy();
    trace.call_bool("deploy", deploy_result);

    let session_id = create_session();
    assert_ne!(session_id, 0, "yune-web wrapper creates one global session");
    trace.call_number("create_global_session", 1);
    let found = find_session(session_id);
    assert_eq!(found, TRUE, "global session is findable");
    trace.call_bool("find_session", found);

    let schema_id = CString::new(YUNE_WEB_SCHEMA).expect("schema id should be valid");
    // SAFETY: `schema_id` is a valid NUL-terminated logical schema ID and lives
    // for the duration of the call.
    let selected = unsafe { select_schema(session_id, schema_id.as_ptr()) };
    assert_eq!(selected, TRUE, "yune-web wrapper selects yune_web_luna");
    trace.call_bool("select_schema", selected);

    let sequence = CString::new("ba").expect("key sequence should be valid");
    // SAFETY: `sequence` is a valid NUL-terminated librime-style key sequence
    // and lives for the duration of the call.
    let simulated = unsafe { simulate_key_sequence(session_id, sequence.as_ptr()) };
    assert_eq!(simulated, TRUE, "yune-web wrapper simulates key sequence");
    trace.call_bool("simulate_key_sequence", simulated);
    let input = get_input(session_id);
    assert!(
        !input.is_null(),
        "input pointer should be available while composing"
    );
    // SAFETY: get_input returns a valid NUL-terminated pointer owned by the
    // session for immediate read-only observation.
    assert_eq!(unsafe { CStr::from_ptr(input) }.to_str(), Ok("ba"));
    trace.call_text("get_input", "ba");

    read_status(&mut trace, session_id, get_status, free_status);
    read_context(&mut trace, session_id, get_context, free_context, 2, 1);

    let mut iterator = super::empty_candidate_list_iterator();
    // SAFETY: `iterator` points to caller-owned writable storage. A successful
    // begin is paired with `candidate_list_end` before `iterator` is discarded.
    let iterator_started = unsafe { candidate_list_begin(session_id, &mut iterator) };
    assert_eq!(iterator_started, TRUE, "candidate list iterator starts");
    trace.call_bool("candidate_list_begin", iterator_started);
    // SAFETY: iterator storage remains valid between begin and end.
    let iterator_next = unsafe { candidate_list_next(&mut iterator) };
    assert_eq!(iterator_next, TRUE, "candidate list iterator advances");
    trace.call_bool("candidate_list_next", iterator_next);
    // SAFETY: `candidate_list_end` receives the same iterator object returned by begin.
    unsafe { candidate_list_end(&mut iterator) };
    trace.call_bool("candidate_list_end", TRUE);
    trace.record_free_pair("candidate_list_begin", "candidate_list_end", true);

    assert_eq!(highlight_candidate(session_id, 3), TRUE);
    trace.call_bool("highlight_candidate", TRUE);
    read_context(&mut trace, session_id, get_context, free_context, 2, 2);
    assert_eq!(highlight_candidate_on_current_page(session_id, 0), TRUE);
    trace.call_bool("highlight_candidate_on_current_page", TRUE);
    assert_eq!(change_page(session_id, FALSE), TRUE);
    trace.call_bool("change_page", TRUE);
    assert_eq!(delete_candidate_on_current_page(session_id, 0), TRUE);
    trace.call_bool("delete_candidate_on_current_page", TRUE);
    read_context(&mut trace, session_id, get_context, free_context, 2, 1);

    assert_eq!(select_candidate_on_current_page(session_id, 0), TRUE);
    trace.call_bool("select_candidate_on_current_page", TRUE);
    read_commit(&mut trace, session_id, get_commit, free_commit, "拔");

    run_levers_customization(&mut trace, api, &runtime.user);

    // SAFETY: same valid option ID lifetime as the CString call scope.
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    set_notification_handler(Some(record_notification_replacement), ptr::null_mut());
    trace.call_text("set_notification_handler", "worker_handler_replacement");
    if let Some(set_option) = api.set_option {
        unsafe { set_option(session_id, ascii_mode.as_ptr(), TRUE) };
        trace.call_bool("set_option_replacement_handler", TRUE);
    }
    set_notification_handler(None, ptr::null_mut());
    trace.call_text("set_notification_handler", "cleared");
    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clone();
    assert!(events
        .iter()
        .any(|event| event.handler == "worker_handler_replacement"));
    for event in &events {
        trace.record_notification(
            &event.handler,
            if event.session_id == session_id {
                "global_session"
            } else {
                "runtime"
            },
            &event.message_type,
            &event.message_value,
        );
    }

    let destroyed = destroy_session(session_id);
    assert_eq!(destroyed, TRUE, "yune-web wrapper destroys global session");
    trace.call_bool("destroy_session", destroyed);
    cleanup_all_sessions();
    trace.call_bool("cleanup_all_sessions", TRUE);
    finalize();
    trace.call_bool("finalize", TRUE);
    let reset_traits = empty_traits();
    // SAFETY: null/default traits restore default runtime paths after the host
    // scenario finishes.
    unsafe { setup(&reset_traits) };
    trace.call_bool("teardown_setup_reset", TRUE);
    fs::remove_dir_all(&root).expect("temp dirs should be removed");
    trace.call_bool("teardown_remove_runtime", TRUE);

    trace.assert_sanitized();
    trace
}

fn require<T>(trace: &mut FrontendHostTrace, name: &str, function: Option<T>) -> T {
    trace.record_function(name, function.is_some());
    function.unwrap_or_else(|| panic!("yune-web required RimeApi entry is missing: {name}"))
}

fn read_status(
    trace: &mut FrontendHostTrace,
    session_id: RimeSessionId,
    get_status: unsafe extern "C" fn(RimeSessionId, *mut yune_rime_api::RimeStatus) -> c_int,
    free_status: unsafe extern "C" fn(*mut yune_rime_api::RimeStatus) -> c_int,
) {
    let mut status = empty_status();
    // SAFETY: `status` points to caller-owned writable storage and is freed by
    // the matching `free_status` call before the object is discarded.
    let status_result = unsafe { get_status(session_id, &mut status) };
    assert_eq!(status_result, TRUE, "yune-web wrapper reads status");
    assert_eq!(status.is_composing, TRUE);
    let status_ptr = &mut status as *mut _ as usize;
    trace.call_bool("get_status", status_result);
    // SAFETY: free_status receives the same caller-owned status object returned
    // by get_status.
    let free_status_result = unsafe { free_status(&mut status) };
    assert_eq!(free_status_result, TRUE, "yune-web wrapper frees status");
    trace.call_bool("free_status", free_status_result);
    trace.record_free_pair(
        "get_status",
        "free_status",
        status_ptr == &mut status as *mut _ as usize,
    );
}

fn read_context(
    trace: &mut FrontendHostTrace,
    session_id: RimeSessionId,
    get_context: unsafe extern "C" fn(RimeSessionId, *mut yune_rime_api::RimeContext) -> c_int,
    free_context: unsafe extern "C" fn(*mut yune_rime_api::RimeContext) -> c_int,
    expected_page_size: c_int,
    min_candidates: c_int,
) {
    let mut context = empty_context();
    // SAFETY: `context` points to caller-owned writable storage and is freed by
    // the matching `free_context` call before pointer fields are discarded.
    let context_result = unsafe { get_context(session_id, &mut context) };
    assert_eq!(context_result, TRUE, "yune-web wrapper reads context");
    assert_eq!(context.menu.page_size, expected_page_size);
    assert!(context.menu.num_candidates >= min_candidates);
    let context_ptr = &mut context as *mut _ as usize;
    trace.call_bool("get_context", context_result);
    // SAFETY: free_context receives the same caller-owned context object returned
    // by get_context.
    let free_context_result = unsafe { free_context(&mut context) };
    assert_eq!(free_context_result, TRUE, "yune-web wrapper frees context");
    trace.call_bool("free_context", free_context_result);
    trace.record_free_pair(
        "get_context",
        "free_context",
        context_ptr == &mut context as *mut _ as usize,
    );
}

fn read_commit(
    trace: &mut FrontendHostTrace,
    session_id: RimeSessionId,
    get_commit: unsafe extern "C" fn(RimeSessionId, *mut yune_rime_api::RimeCommit) -> c_int,
    free_commit: unsafe extern "C" fn(*mut yune_rime_api::RimeCommit) -> c_int,
    expected: &str,
) {
    let mut commit = empty_commit();
    // SAFETY: `commit` points to caller-owned writable storage and is freed by
    // the matching `free_commit` call before pointer fields are discarded.
    let commit_result = unsafe { get_commit(session_id, &mut commit) };
    assert_eq!(commit_result, TRUE, "yune-web wrapper reads commit");
    assert!(!commit.text.is_null());
    // SAFETY: successful get_commit populated commit.text with a valid
    // NUL-terminated C string until free_commit is called.
    assert_eq!(
        unsafe { CStr::from_ptr(commit.text) }.to_str(),
        Ok(expected)
    );
    let commit_ptr = &mut commit as *mut _ as usize;
    trace.call_bool("get_commit", commit_result);
    // SAFETY: free_commit receives the same caller-owned commit object returned
    // by get_commit.
    let free_commit_result = unsafe { free_commit(&mut commit) };
    assert_eq!(free_commit_result, TRUE, "yune-web wrapper frees commit");
    trace.call_bool("free_commit", free_commit_result);
    trace.record_free_pair(
        "get_commit",
        "free_commit",
        commit_ptr == &mut commit as *mut _ as usize,
    );
}

fn run_levers_customization(trace: &mut FrontendHostTrace, api: &RimeApi, user: &Path) {
    let find_module = require(trace, "find_module", api.find_module);
    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: levers_name is a valid NUL-terminated module name and lives for the call.
    let module = unsafe { find_module(levers_name.as_ptr()) };
    assert!(!module.is_null(), "levers module should be registered");
    let module = unsafe { &*module };
    let get_api = module.get_api.expect("levers module should expose get_api");
    let levers_api = get_api().cast::<RimeLeversApi>();
    assert!(
        !levers_api.is_null(),
        "levers API pointer should be available"
    );
    let levers_api = unsafe { &*levers_api };

    let custom_settings_init = require_levers(
        trace,
        "levers.custom_settings_init",
        levers_api.custom_settings_init,
    );
    let custom_settings_destroy = require_levers(
        trace,
        "levers.custom_settings_destroy",
        levers_api.custom_settings_destroy,
    );
    let load_settings = require_levers(trace, "levers.load_settings", levers_api.load_settings);
    let save_settings = require_levers(trace, "levers.save_settings", levers_api.save_settings);
    let customize_bool = require_levers(trace, "levers.customize_bool", levers_api.customize_bool);
    let customize_int = require_levers(trace, "levers.customize_int", levers_api.customize_int);
    let customize_string = require_levers(
        trace,
        "levers.customize_string",
        levers_api.customize_string,
    );
    let is_first_run = require_levers(trace, "levers.is_first_run", levers_api.is_first_run);
    let settings_is_modified = require_levers(
        trace,
        "levers.settings_is_modified",
        levers_api.settings_is_modified,
    );

    let config_id = CString::new("yune_web_luna.schema").expect("config id should be valid");
    let generator = CString::new("yune-web").expect("generator should be valid");
    // SAFETY: config_id and generator are valid NUL-terminated strings and live
    // for the duration of the call. Returned settings are destroyed below.
    let settings = unsafe { custom_settings_init(config_id.as_ptr(), generator.as_ptr()) };
    assert!(
        !settings.is_null(),
        "levers custom settings should initialize"
    );
    trace.call_bool("levers.custom_settings_init", TRUE);

    // SAFETY: settings pointer was allocated by custom_settings_init.
    trace.call_bool("levers.load_settings", unsafe { load_settings(settings) });
    // SAFETY: same settings pointer lifetime.
    assert_eq!(unsafe { is_first_run(settings) }, TRUE);
    trace.call_bool("levers.is_first_run", TRUE);
    // SAFETY: same settings pointer lifetime.
    assert_eq!(unsafe { settings_is_modified(settings) }, FALSE);
    trace.call_bool("levers.settings_is_modified_initial", FALSE);

    let bool_key = CString::new("switches/@0/reset").expect("custom key should be valid");
    let int_key = CString::new("menu/page_size").expect("custom key should be valid");
    let string_key = CString::new("schema/name").expect("custom key should be valid");
    let string_value = CString::new("Yune Web Luna").expect("custom value should be valid");
    // SAFETY: settings and C strings are valid for each customization call.
    assert_eq!(
        unsafe { customize_bool(settings, bool_key.as_ptr(), TRUE) },
        TRUE
    );
    trace.call_bool("levers.customize_bool", TRUE);
    // SAFETY: same settings and key lifetime guarantee.
    assert_eq!(
        unsafe { customize_int(settings, int_key.as_ptr(), 7) },
        TRUE
    );
    trace.call_bool("levers.customize_int", TRUE);
    // SAFETY: same settings and key/value lifetime guarantee.
    assert_eq!(
        unsafe { customize_string(settings, string_key.as_ptr(), string_value.as_ptr()) },
        TRUE
    );
    trace.call_bool("levers.customize_string", TRUE);
    // SAFETY: settings pointer remains valid until destroy below.
    assert_eq!(unsafe { settings_is_modified(settings) }, TRUE);
    trace.call_bool("levers.settings_is_modified_after_customization", TRUE);
    // SAFETY: settings pointer remains valid until destroy below.
    assert_eq!(unsafe { save_settings(settings) }, TRUE);
    trace.call_bool("levers.save_settings", TRUE);
    // SAFETY: destroy receives the same settings pointer returned by init.
    unsafe { custom_settings_destroy(settings) };
    trace.call_bool("levers.custom_settings_destroy", TRUE);
    trace.record_free_pair(
        "levers.custom_settings_init",
        "levers.custom_settings_destroy",
        true,
    );
    assert!(user.join("yune_web_luna.custom.yaml").exists());
}

fn require_levers<T>(trace: &mut FrontendHostTrace, name: &str, function: Option<T>) -> T {
    trace.record_function(name, function.is_some());
    function.unwrap_or_else(|| panic!("yune-web required levers entry is missing: {name}"))
}

struct YuneWebRuntime {
    shared: PathBuf,
    user: PathBuf,
    staging: PathBuf,
    _prebuilt: PathBuf,
    shared_c: CString,
    user_c: CString,
    prebuilt_c: CString,
    staging_c: CString,
    distribution_name_c: CString,
    app_name_c: CString,
}

impl YuneWebRuntime {
    fn create(root: &Path) -> Self {
        let shared = root.join("shared");
        let user = root.join("browser-user");
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
            user,
            staging,
            _prebuilt: prebuilt,
            shared_c,
            user_c,
            prebuilt_c,
            staging_c,
            distribution_name_c: CString::new("yune-web modeled wrapper")
                .expect("distribution name is valid"),
            app_name_c: CString::new("yune-web").expect("app name is valid"),
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

fn write_yune_web_schema(shared: &Path, staging: &Path) {
    fs::write(
        staging.join("default.yaml"),
        "config_version: yune-web\nschema_list:\n  - schema: yune_web_luna\n",
    )
    .expect("yune-web default config should be written");
    fs::write(
        staging.join("yune_web_luna.schema.yaml"),
        "\
schema:\n  schema_id: yune_web_luna\n  name: Yune Web Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n  alternative_select_labels: [Alpha, Beta]\nswitches:\n  - name: ascii_mode\n    reset: 0\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: yune_web\n",
    )
    .expect("yune-web schema config should be written");
    fs::write(
        shared.join("yune_web.dict.yaml"),
        "\
---\nname: yune_web\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nba\t八\t10\nba\t吧\t9\nba\t爸\t8\nba\t巴\t7\nba\t把\t6\nba\t拔\t5\n",
    )
    .expect("yune-web dictionary should be written");
}
