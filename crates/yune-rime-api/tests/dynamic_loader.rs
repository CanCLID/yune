use std::{
    ffi::{c_void, CStr, CString},
    fs, mem,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use libloading::Library;
use yune_rime_api::{
    Bool, RimeCommit, RimeComposition, RimeContext, RimeMenu, RimeSessionId, RimeStatus,
    RimeTraits, FALSE, TRUE,
};

type RimeGetApi = unsafe extern "C" fn() -> *mut yune_rime_api::RimeApi;

#[derive(Debug, PartialEq, Eq)]
struct NotificationEvent {
    context_object: usize,
    session_id: RimeSessionId,
    message_type: String,
    message_value: String,
}

fn empty_traits() -> RimeTraits {
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

fn empty_context() -> RimeContext {
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

fn empty_status() -> RimeStatus {
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

fn empty_commit() -> RimeCommit {
    RimeCommit {
        data_size: (mem::size_of::<RimeCommit>() - mem::size_of::<c_int>()) as c_int,
        text: ptr::null_mut(),
    }
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

fn dynamic_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yune_rime_api.dll"
    } else if cfg!(target_os = "macos") {
        "libyune_rime_api.dylib"
    } else {
        "libyune_rime_api.so"
    }
}

fn target_dir() -> Result<PathBuf, String> {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .and_then(|manifest_dir| manifest_dir.parent()?.parent().map(Path::to_path_buf))
                .map(|workspace| workspace.join("target"))
        })
        .ok_or_else(|| "missing CARGO_MANIFEST_DIR; cannot locate Cargo target directory".to_owned())
}

fn discover_dynamic_artifact() -> Result<PathBuf, String> {
    let target_dir = target_dir()?;
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_owned());
    let candidates = [
        target_dir.join(&profile).join(dynamic_library_name()),
        target_dir.join("debug").join(dynamic_library_name()),
        target_dir.join("release").join(dynamic_library_name()),
    ];
    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
        .ok_or_else(|| {
            let checked = candidates
                .iter()
                .map(|candidate| candidate.display().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "missing Cargo-built dynamic artifact {}; checked {checked}",
                dynamic_library_name()
            )
        })
}

fn require<T>(name: &str, function: Option<T>) -> T {
    function.unwrap_or_else(|| panic!("null required RimeApi function pointer: {name}"))
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

fn bool_name(value: Bool) -> &'static str {
    if value == TRUE { "TRUE" } else { "FALSE" }
}

#[test]
fn dynamic_loader_harness_loads_cargo_cdylib_and_api_table() {
    let _guard = test_guard();
    let artifact = discover_dynamic_artifact().unwrap_or_else(|message| panic!("missing artifact: {message}"));

    // SAFETY: loading is restricted to the Cargo-built yune-rime-api artifact
    // discovered under the active target directory.
    let library = unsafe { Library::new(&artifact) }
        .unwrap_or_else(|error| panic!("failed to load dynamic artifact {}: {error}", artifact.display()));

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
    let set_notification_handler = require("set_notification_handler", api.set_notification_handler);
    let deploy = require("deploy", api.deploy);
    let create_session = require("create_session", api.create_session);
    let find_session = require("find_session", api.find_session);
    let select_schema = require("select_schema", api.select_schema);
    let process_key = require("process_key", api.process_key);
    let get_status = require("get_status", api.get_status);
    let free_status = require("free_status", api.free_status);
    let get_context = require("get_context", api.get_context);
    let free_context = require("free_context", api.free_context);
    let select_candidate_on_current_page = require(
        "select_candidate_on_current_page",
        api.select_candidate_on_current_page,
    );
    let get_commit = require("get_commit", api.get_commit);
    let free_commit = require("free_commit", api.free_commit);
    let destroy_session = require("destroy_session", api.destroy_session);
    let cleanup_all_sessions = require("cleanup_all_sessions", api.cleanup_all_sessions);

    cleanup_all_sessions();

    let root = unique_temp_dir("runtime");
    let shared = root.join("shared");
    let user = root.join("user");
    let prebuilt = shared.join("build");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::create_dir_all(&prebuilt).expect("prebuilt dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    write_minimal_schema(&shared);

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("shared path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("user path should be valid");
    let prebuilt_c = CString::new(prebuilt.to_string_lossy().as_ref()).expect("prebuilt path should be valid");
    let staging_c = CString::new(staging.to_string_lossy().as_ref()).expect("staging path should be valid");
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
    assert_ne!(session_id, 0, "runtime behavior failure: create_session returned 0");
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
    assert_eq!(
        unsafe { get_status(session_id, &mut status) },
        TRUE,
        "runtime behavior failure: get_status failed"
    );
    assert_eq!(status.is_composing, TRUE);
    let current_schema = unsafe { CStr::from_ptr(status.schema_id) };
    assert_eq!(current_schema.to_str(), Ok("dynamic_schema"));
    assert_eq!(unsafe { free_status(&mut status) }, TRUE);

    let mut context = empty_context();
    assert_eq!(
        unsafe { get_context(session_id, &mut context) },
        TRUE,
        "runtime behavior failure: get_context failed"
    );
    assert_eq!(context.composition.length, 1);
    assert_eq!(context.menu.num_candidates, 1);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(select_candidate_on_current_page(session_id, 0), TRUE);
    let mut commit = empty_commit();
    assert_eq!(
        unsafe { get_commit(session_id, &mut commit) },
        TRUE,
        "runtime behavior failure: get_commit failed after candidate selection"
    );
    let commit_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(commit_text.to_str(), Ok("n"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned");
    assert!(
        events.iter().any(|event| {
            event.context_object == 0x42
                && event.session_id == session_id
                && event.message_type == "schema"
                && event.message_value == "dynamic_schema/Dynamic Schema"
        }),
        "runtime behavior failure: schema notification was not recorded"
    );
    drop(events);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    set_notification_handler(None, ptr::null_mut());
    finalize();
    let reset_traits = empty_traits();
    // SAFETY: null/default traits are accepted by setup to restore default runtime paths.
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
