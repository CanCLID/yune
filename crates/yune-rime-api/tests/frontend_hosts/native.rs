use std::{
    ffi::{c_void, CStr, CString},
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{Bool, RimeApi, RimeSessionId, RimeTraits, FALSE, TRUE};

use super::{
    empty_commit, empty_context, empty_status, empty_traits, required_function, FrontendHostTrace,
    HostValidationBlocker, LOGICAL_SCHEMA_ID, NATIVE_SCENARIO, NATIVE_TARGET,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct NotificationEvent {
    handler: String,
    session_id: RimeSessionId,
    message_type: String,
    message_value: String,
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
    record_notification("handler_primary", session_id, message_type, message_value);
}

extern "C" fn record_notification_replacement(
    _context_object: *mut c_void,
    session_id: RimeSessionId,
    message_type: *const c_char,
    message_value: *const c_char,
) {
    record_notification(
        "handler_replacement",
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

pub(crate) fn run_native_host_lifecycle(
    api: &mut RimeApi,
) -> Result<FrontendHostTrace, HostValidationBlocker> {
    let mut trace = FrontendHostTrace::new(NATIVE_TARGET, NATIVE_SCENARIO);
    trace.call_text("rime_get_api", "resolved");
    trace.call_bool("validate_api_data_size", (api.data_size > 0) as Bool);

    let setup = required_function(&mut trace, "setup", api.setup)?;
    let set_notification_handler = required_function(
        &mut trace,
        "set_notification_handler",
        api.set_notification_handler,
    )?;
    let initialize = required_function(&mut trace, "initialize", api.initialize)?;
    let finalize = required_function(&mut trace, "finalize", api.finalize)?;
    let deployer_initialize =
        required_function(&mut trace, "deployer_initialize", api.deployer_initialize)?;
    let start_maintenance =
        required_function(&mut trace, "start_maintenance", api.start_maintenance)?;
    let is_maintenance_mode =
        required_function(&mut trace, "is_maintenance_mode", api.is_maintenance_mode)?;
    let join_maintenance_thread = required_function(
        &mut trace,
        "join_maintenance_thread",
        api.join_maintenance_thread,
    )?;
    let deploy = required_function(&mut trace, "deploy", api.deploy)?;
    let deploy_schema = required_function(&mut trace, "deploy_schema", api.deploy_schema)?;
    let create_session = required_function(&mut trace, "create_session", api.create_session)?;
    let find_session = required_function(&mut trace, "find_session", api.find_session)?;
    let select_schema = required_function(&mut trace, "select_schema", api.select_schema)?;
    let process_key = required_function(&mut trace, "process_key", api.process_key)?;
    let get_status = required_function(&mut trace, "get_status", api.get_status)?;
    let free_status = required_function(&mut trace, "free_status", api.free_status)?;
    let get_context = required_function(&mut trace, "get_context", api.get_context)?;
    let free_context = required_function(&mut trace, "free_context", api.free_context)?;
    let select_candidate_on_current_page = required_function(
        &mut trace,
        "select_candidate_on_current_page",
        api.select_candidate_on_current_page,
    )?;
    let get_commit = required_function(&mut trace, "get_commit", api.get_commit)?;
    let free_commit = required_function(&mut trace, "free_commit", api.free_commit)?;
    let destroy_session = required_function(&mut trace, "destroy_session", api.destroy_session)?;
    let cleanup_stale_sessions = required_function(
        &mut trace,
        "cleanup_stale_sessions",
        api.cleanup_stale_sessions,
    )?;
    let cleanup_all_sessions =
        required_function(&mut trace, "cleanup_all_sessions", api.cleanup_all_sessions)?;
    let set_option = required_function(&mut trace, "set_option", api.set_option)?;

    cleanup_all_sessions();
    trace.call_bool("cleanup_all_sessions", TRUE);

    let root = unique_temp_dir("native-host-runtime");
    let runtime = NativeRuntime::create(&root);
    write_minimal_schema(&runtime.shared);
    let traits = runtime.traits();

    // SAFETY: the C strings referenced by traits are owned by `runtime` and kept
    // alive until after all setup/initialize/deployer calls complete.
    unsafe { setup(&traits) };
    trace.call_bool("setup", TRUE);
    // SAFETY: same C string lifetime guarantee as setup.
    unsafe { deployer_initialize(&traits) };
    trace.call_bool("deployer_initialize", TRUE);

    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
    set_notification_handler(Some(record_notification_primary), ptr::null_mut());
    trace.call_text("set_notification_handler", "handler_primary");

    // SAFETY: the C strings referenced by traits are owned by `runtime` and kept
    // alive while initialize reads the host-provided runtime paths.
    unsafe { initialize(&traits) };
    trace.call_bool("initialize", TRUE);

    let maintenance = start_maintenance(TRUE);
    trace.call_bool("start_maintenance", maintenance);
    let maintenance_mode = is_maintenance_mode();
    trace.call_bool("is_maintenance_mode", maintenance_mode);
    join_maintenance_thread();
    trace.call_bool("join_maintenance_thread", TRUE);
    let deploy_result = deploy();
    trace.call_bool("deploy", deploy_result);
    let schema_id = CString::new(LOGICAL_SCHEMA_ID).expect("schema id should be valid");
    let deploy_schema_result = deploy_schema(schema_id.as_ptr());
    trace.call_bool("deploy_schema", deploy_schema_result);

    let session_id = create_session();
    assert_ne!(session_id, 0, "native host creates a non-zero session");
    trace.call_number("create_session", 1);
    let found = find_session(session_id);
    assert_eq!(found, TRUE, "native host finds a newly-created session");
    trace.call_bool("find_session", found);
    // SAFETY: `schema_id` is a valid NUL-terminated logical schema ID and lives
    // for the duration of the call.
    let selected = unsafe { select_schema(session_id, schema_id.as_ptr()) };
    assert_eq!(selected, TRUE, "native host selects dynamic_schema");
    trace.call_bool("select_schema", selected);

    let key_result = process_key(session_id, 'n' as c_int, 0);
    assert_eq!(key_result, TRUE, "native host processes key through ABI");
    trace.call_bool("process_key", key_result);

    let mut status = empty_status();
    // SAFETY: `status` points to caller-owned writable storage and is freed by
    // the matching `free_status` call before the object is discarded.
    let status_result = unsafe { get_status(session_id, &mut status) };
    assert_eq!(status_result, TRUE, "native host reads session status");
    trace.call_bool("get_status", status_result);
    assert_eq!(status.is_composing, TRUE);
    assert!(!status.schema_id.is_null());
    // SAFETY: successful get_status populated schema_id with a valid
    // NUL-terminated C string until free_status is called.
    let current_schema = unsafe { CStr::from_ptr(status.schema_id) };
    assert_eq!(current_schema.to_str(), Ok(LOGICAL_SCHEMA_ID));
    let status_ptr = &mut status as *mut _ as usize;
    // SAFETY: free_status receives the same caller-owned status object returned
    // by get_status.
    let free_status_result = unsafe { free_status(&mut status) };
    assert_eq!(free_status_result, TRUE, "native host frees status");
    trace.call_bool("free_status", free_status_result);
    trace.record_free_pair(
        "get_status",
        "free_status",
        status_ptr == &mut status as *mut _ as usize,
    );

    let mut context = empty_context();
    // SAFETY: `context` points to caller-owned writable storage and is freed by
    // the matching `free_context` call before pointer fields are discarded.
    let context_result = unsafe { get_context(session_id, &mut context) };
    assert_eq!(context_result, TRUE, "native host reads session context");
    assert_eq!(context.composition.length, 1);
    assert_eq!(context.menu.num_candidates, 1);
    trace.call_bool("get_context", context_result);
    let context_ptr = &mut context as *mut _ as usize;
    // SAFETY: free_context receives the same caller-owned context object returned
    // by get_context.
    let free_context_result = unsafe { free_context(&mut context) };
    assert_eq!(free_context_result, TRUE, "native host frees context");
    trace.call_bool("free_context", free_context_result);
    trace.record_free_pair(
        "get_context",
        "free_context",
        context_ptr == &mut context as *mut _ as usize,
    );

    let selected_candidate = select_candidate_on_current_page(session_id, 0);
    assert_eq!(
        selected_candidate, TRUE,
        "native host selects current candidate"
    );
    trace.call_bool("select_candidate_on_current_page", selected_candidate);
    let mut commit = empty_commit();
    // SAFETY: `commit` points to caller-owned writable storage and is freed by
    // the matching `free_commit` call before pointer fields are discarded.
    let commit_result = unsafe { get_commit(session_id, &mut commit) };
    assert_eq!(commit_result, TRUE, "native host reads commit");
    trace.call_bool("get_commit", commit_result);
    assert!(!commit.text.is_null());
    // SAFETY: successful get_commit populated commit.text with a valid
    // NUL-terminated C string until free_commit is called.
    let commit_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(commit_text.to_str(), Ok("n"));
    let commit_ptr = &mut commit as *mut _ as usize;
    // SAFETY: free_commit receives the same caller-owned commit object returned
    // by get_commit.
    let free_commit_result = unsafe { free_commit(&mut commit) };
    assert_eq!(free_commit_result, TRUE, "native host frees commit");
    trace.call_bool("free_commit", free_commit_result);
    trace.record_free_pair(
        "get_commit",
        "free_commit",
        commit_ptr == &mut commit as *mut _ as usize,
    );

    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    // SAFETY: `ascii_mode` is a valid NUL-terminated option ID and lives for the
    // duration of the call.
    unsafe { set_option(session_id, ascii_mode.as_ptr(), TRUE) };
    trace.call_bool("set_option_primary_handler", TRUE);
    set_notification_handler(Some(record_notification_replacement), ptr::null_mut());
    trace.call_text("set_notification_handler", "handler_replacement");
    // SAFETY: same valid option ID lifetime as above.
    unsafe { set_option(session_id, ascii_mode.as_ptr(), FALSE) };
    trace.call_bool("set_option_replacement_handler", TRUE);
    set_notification_handler(None, ptr::null_mut());
    trace.call_text("set_notification_handler", "cleared");
    // SAFETY: same valid option ID lifetime as above.
    unsafe { set_option(session_id, ascii_mode.as_ptr(), TRUE) };
    trace.call_bool("set_option_after_clear", TRUE);

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clone();
    assert!(events
        .iter()
        .any(|event| event.handler == "handler_primary"));
    assert!(events
        .iter()
        .any(|event| event.handler == "handler_replacement"));
    let event_count_after_clear = events.len();
    for event in &events {
        let session = if event.session_id == session_id {
            "session_primary".to_owned()
        } else {
            "session_global".to_owned()
        };
        trace.record_notification(
            &event.handler,
            &session,
            &event.message_type,
            &event.message_value,
        );
    }
    assert_eq!(
        notification_events()
            .lock()
            .expect("notification events should not be poisoned")
            .len(),
        event_count_after_clear,
        "cleared notification handler suppresses later events"
    );

    let destroyed = destroy_session(session_id);
    assert_eq!(destroyed, TRUE, "native host destroys primary session");
    trace.call_bool("destroy_session", destroyed);
    let stale_find_after_destroy = find_session(session_id);
    trace.record_stale_session(
        "destroy_session",
        "find_session",
        stale_find_after_destroy == TRUE,
    );
    assert_eq!(stale_find_after_destroy, FALSE);
    let stale_destroy_after_destroy = destroy_session(session_id);
    trace.record_stale_session(
        "destroy_session",
        "destroy_session",
        stale_destroy_after_destroy == TRUE,
    );
    assert_eq!(stale_destroy_after_destroy, FALSE);
    cleanup_stale_sessions();
    trace.call_bool("cleanup_stale_sessions", TRUE);
    cleanup_all_sessions();
    trace.call_bool("cleanup_all_sessions", TRUE);

    finalize();
    trace.call_bool("finalize", TRUE);
    let create_after_finalize = create_session();
    trace.record_stale_session("finalize", "create_session", create_after_finalize != 0);
    assert_eq!(create_after_finalize, 0);
    let find_after_finalize = find_session(session_id);
    trace.record_stale_session("finalize", "find_session", find_after_finalize == TRUE);
    assert_eq!(find_after_finalize, FALSE);

    // SAFETY: the C strings referenced by traits are owned by `runtime` and kept
    // alive while the repeated initialize reads the host-provided runtime paths.
    unsafe { initialize(&traits) };
    trace.call_bool("reinitialize", TRUE);
    let repeated_session = create_session();
    assert_ne!(repeated_session, 0, "reinitialize permits new sessions");
    trace.call_number("create_session_after_reinitialize", 2);
    assert_eq!(destroy_session(repeated_session), TRUE);
    trace.call_bool("destroy_session_after_reinitialize", TRUE);
    finalize();
    trace.call_bool("finalize_repeated", TRUE);
    let find_after_repeated_finalize = find_session(repeated_session);
    trace.record_stale_session(
        "finalize_repeated",
        "find_session",
        find_after_repeated_finalize == TRUE,
    );
    assert_eq!(find_after_repeated_finalize, FALSE);

    set_notification_handler(None, ptr::null_mut());
    trace.call_text("teardown_notification_handler", "cleared");
    let reset_traits = empty_traits();
    // SAFETY: null/default traits are accepted by setup to restore default
    // runtime paths after the host-shaped scenario finishes.
    unsafe { setup(&reset_traits) };
    trace.call_bool("teardown_setup_reset", TRUE);
    fs::remove_dir_all(&root).expect("temp dirs should be removed");
    trace.call_bool("teardown_remove_runtime", TRUE);

    trace.assert_sanitized();
    Ok(trace)
}

struct NativeRuntime {
    shared: PathBuf,
    _user: PathBuf,
    _prebuilt: PathBuf,
    _staging: PathBuf,
    shared_c: CString,
    user_c: CString,
    prebuilt_c: CString,
    staging_c: CString,
}

impl NativeRuntime {
    fn create(root: &Path) -> Self {
        let shared = root.join("shared");
        let user = root.join("user");
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
            _user: user,
            _prebuilt: prebuilt,
            _staging: staging,
            shared_c,
            user_c,
            prebuilt_c,
            staging_c,
        }
    }

    fn traits(&self) -> RimeTraits {
        let mut traits = empty_traits();
        traits.shared_data_dir = self.shared_c.as_ptr();
        traits.user_data_dir = self.user_c.as_ptr();
        traits.prebuilt_data_dir = self.prebuilt_c.as_ptr();
        traits.staging_dir = self.staging_c.as_ptr();
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

fn write_minimal_schema(shared: &Path) {
    fs::write(
        shared.join("default.yaml"),
        "config_version: frontend-host\nschema_list:\n  - schema: dynamic_schema\n",
    )
    .expect("frontend host default config should be written");
    fs::write(
        shared.join("dynamic_schema.schema.yaml"),
        "schema:\n  schema_id: dynamic_schema\n  name: Dynamic Schema\n",
    )
    .expect("frontend host schema config should be written");
}
