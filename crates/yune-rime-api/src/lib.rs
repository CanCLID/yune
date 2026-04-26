use std::{
    collections::HashMap,
    ffi::{c_void, CStr, CString},
    os::raw::{c_char, c_int},
    path::Path,
    ptr, slice,
    sync::{Mutex, OnceLock},
};

use yune_core::{parse_key_sequence, Engine, KeyCode, KeyEvent, KeyModifiers};

pub type RimeSessionId = usize;
pub type Bool = c_int;

pub const FALSE: Bool = 0;
pub const TRUE: Bool = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeTraits {
    pub data_size: c_int,
    pub shared_data_dir: *const c_char,
    pub user_data_dir: *const c_char,
    pub distribution_name: *const c_char,
    pub distribution_code_name: *const c_char,
    pub distribution_version: *const c_char,
    pub app_name: *const c_char,
    pub modules: *const *const c_char,
    pub min_log_level: c_int,
    pub log_dir: *const c_char,
    pub prebuilt_data_dir: *const c_char,
    pub staging_dir: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCommit {
    pub data_size: c_int,
    pub text: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeComposition {
    pub length: c_int,
    pub cursor_pos: c_int,
    pub sel_start: c_int,
    pub sel_end: c_int,
    pub preedit: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCandidate {
    pub text: *mut c_char,
    pub comment: *mut c_char,
    pub reserved: *mut std::ffi::c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeMenu {
    pub page_size: c_int,
    pub page_no: c_int,
    pub is_last_page: Bool,
    pub highlighted_candidate_index: c_int,
    pub num_candidates: c_int,
    pub candidates: *mut RimeCandidate,
    pub select_keys: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeContext {
    pub data_size: c_int,
    pub composition: RimeComposition,
    pub menu: RimeMenu,
    pub commit_text_preview: *mut c_char,
    pub select_labels: *mut *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeStatus {
    pub data_size: c_int,
    pub schema_id: *mut c_char,
    pub schema_name: *mut c_char,
    pub is_disabled: Bool,
    pub is_composing: Bool,
    pub is_ascii_mode: Bool,
    pub is_full_shape: Bool,
    pub is_simplified: Bool,
    pub is_traditional: Bool,
    pub is_ascii_punct: Bool,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCandidateListIterator {
    pub ptr: *mut c_void,
    pub index: c_int,
    pub candidate: RimeCandidate,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeSchemaListItem {
    pub schema_id: *mut c_char,
    pub name: *mut c_char,
    pub reserved: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeSchemaList {
    pub size: usize,
    pub list: *mut RimeSchemaListItem,
}

const XK_BACKSPACE: c_int = 0xff08;
const XK_RETURN: c_int = 0xff0d;
const DEFAULT_PAGE_SIZE: usize = 5;
const DEFAULT_SCHEMA_ID: &str = "default";
const DEFAULT_SCHEMA_NAME: &str = "Default";
const RIME_VERSION_BYTES: &[u8] =
    concat!("yune-rime-api ", env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[derive(Default)]
struct SessionRegistry {
    next_id: RimeSessionId,
    sessions: HashMap<RimeSessionId, SessionState>,
}

impl SessionRegistry {
    fn create_session(&mut self) -> RimeSessionId {
        self.next_id = self.next_id.saturating_add(1).max(1);
        let session_id = self.next_id;
        self.sessions.insert(session_id, SessionState::default());
        session_id
    }
}

#[derive(Default)]
struct SessionState {
    engine: Engine,
    unread_commit: Option<String>,
    input_buffer: Option<CString>,
}

struct CandidateListState {
    candidates: Vec<yune_core::Candidate>,
}

struct RuntimePaths {
    shared_data_dir: CString,
    user_data_dir: CString,
    prebuilt_data_dir: CString,
    staging_dir: CString,
    sync_dir: CString,
    user_id: CString,
    user_data_sync_dir: CString,
}

impl Default for RuntimePaths {
    fn default() -> Self {
        Self::new(".", ".", "build", "build", "sync", "unknown")
    }
}

impl RuntimePaths {
    fn new(
        shared_data_dir: &str,
        user_data_dir: &str,
        prebuilt_data_dir: &str,
        staging_dir: &str,
        sync_dir: &str,
        user_id: &str,
    ) -> Self {
        let user_data_sync_dir = path_join(sync_dir, user_id);
        Self {
            shared_data_dir: cstring_from_lossless_str(shared_data_dir),
            user_data_dir: cstring_from_lossless_str(user_data_dir),
            prebuilt_data_dir: cstring_from_lossless_str(prebuilt_data_dir),
            staging_dir: cstring_from_lossless_str(staging_dir),
            sync_dir: cstring_from_lossless_str(sync_dir),
            user_id: cstring_from_lossless_str(user_id),
            user_data_sync_dir: cstring_from_lossless_str(&user_data_sync_dir),
        }
    }

    unsafe fn from_traits(traits: *const RimeTraits) -> Option<Self> {
        if traits.is_null() {
            return None;
        }

        // SAFETY: callers promise that `traits`, when non-null, points to a
        // valid `RimeTraits` object whose optional C strings are NUL-terminated.
        let traits = unsafe { &*traits };
        let shared_data_dir =
            optional_c_string(traits.shared_data_dir).unwrap_or_else(|| ".".to_owned());
        let user_data_dir =
            optional_c_string(traits.user_data_dir).unwrap_or_else(|| ".".to_owned());
        let prebuilt_data_dir = optional_c_string(traits.prebuilt_data_dir)
            .unwrap_or_else(|| path_join(&shared_data_dir, "build"));
        let staging_dir = optional_c_string(traits.staging_dir)
            .unwrap_or_else(|| path_join(&user_data_dir, "build"));

        Some(Self::new(
            &shared_data_dir,
            &user_data_dir,
            &prebuilt_data_dir,
            &staging_dir,
            "sync",
            "unknown",
        ))
    }
}

fn sessions() -> &'static Mutex<SessionRegistry> {
    static SESSIONS: OnceLock<Mutex<SessionRegistry>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(SessionRegistry::default()))
}

fn runtime_paths() -> &'static Mutex<RuntimePaths> {
    static RUNTIME_PATHS: OnceLock<Mutex<RuntimePaths>> = OnceLock::new();
    RUNTIME_PATHS.get_or_init(|| Mutex::new(RuntimePaths::default()))
}

#[must_use]
pub const fn bool_from(value: bool) -> Bool {
    if value {
        TRUE
    } else {
        FALSE
    }
}

/// Stores process-wide runtime traits for later path queries.
///
/// # Safety
///
/// `traits` must be either null or a valid pointer to a `RimeTraits` object.
/// Any non-null string pointers in the traits object must be valid
/// NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn RimeSetup(traits: *const RimeTraits) {
    if let Some(paths) = unsafe { RuntimePaths::from_traits(traits) } {
        *runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned") = paths;
    }
}

#[no_mangle]
pub extern "C" fn RimeSetupLogging(_app_name: *const c_char) {}

/// Initializes the runtime using the same trait handling as `RimeSetup`.
///
/// # Safety
///
/// `traits` follows the same preconditions as `RimeSetup`.
#[no_mangle]
pub unsafe extern "C" fn RimeInitialize(traits: *const RimeTraits) {
    // SAFETY: forwarded preconditions are identical to `RimeSetup`.
    unsafe { RimeSetup(traits) };
}

#[no_mangle]
pub extern "C" fn RimeFinalize() {
    RimeCleanupAllSessions();
}

#[no_mangle]
pub extern "C" fn RimeStartMaintenance(_full_check: Bool) -> Bool {
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeStartMaintenanceOnWorkspaceChange() -> Bool {
    RimeStartMaintenance(FALSE)
}

#[no_mangle]
pub extern "C" fn RimeIsMaintenancing() -> Bool {
    FALSE
}

#[no_mangle]
pub extern "C" fn RimeJoinMaintenanceThread() {}

/// Initializes deployer state using the same trait handling as `RimeSetup`.
///
/// # Safety
///
/// `traits` follows the same preconditions as `RimeSetup`.
#[no_mangle]
pub unsafe extern "C" fn RimeDeployerInitialize(traits: *const RimeTraits) {
    // SAFETY: forwarded preconditions are identical to `RimeSetup`.
    unsafe { RimeSetup(traits) };
}

#[no_mangle]
pub extern "C" fn RimePrebuildAllSchemas() -> Bool {
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeDeployWorkspace() -> Bool {
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeDeploySchema(schema_file: *const c_char) -> Bool {
    bool_from(!schema_file.is_null())
}

#[no_mangle]
pub extern "C" fn RimeDeployConfigFile(
    file_name: *const c_char,
    version_key: *const c_char,
) -> Bool {
    bool_from(!file_name.is_null() && !version_key.is_null())
}

#[no_mangle]
pub extern "C" fn RimeSyncUserData() -> Bool {
    RimeCleanupAllSessions();
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeRunTask(task_name: *const c_char) -> Bool {
    bool_from(!task_name.is_null())
}

#[no_mangle]
pub extern "C" fn RimeGetVersion() -> *const c_char {
    RIME_VERSION_BYTES.as_ptr().cast::<c_char>()
}

#[no_mangle]
pub extern "C" fn RimeGetSharedDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.shared_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetUserDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.user_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetPrebuiltDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.prebuilt_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetStagingDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.staging_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetSyncDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.sync_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetUserId() -> *const c_char {
    runtime_path_ptr(|paths| &paths.user_id)
}

/// Copies the shared data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSharedDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.shared_data_dir, dir, buffer_size);
}

/// Copies the user data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetUserDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.user_data_dir, dir, buffer_size);
}

/// Copies the prebuilt data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetPrebuiltDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.prebuilt_data_dir, dir, buffer_size);
}

/// Copies the staging directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStagingDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.staging_dir, dir, buffer_size);
}

/// Copies the sync directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSyncDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.sync_dir, dir, buffer_size);
}

/// Copies the user-specific sync directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetUserDataSyncDir(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.user_data_sync_dir, dir, buffer_size);
}

#[no_mangle]
pub extern "C" fn RimeCreateSession() -> RimeSessionId {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .create_session()
}

#[no_mangle]
pub extern "C" fn RimeFindSession(session_id: RimeSessionId) -> Bool {
    bool_from(
        session_id != 0
            && sessions()
                .lock()
                .expect("session registry should not be poisoned")
                .sessions
                .contains_key(&session_id),
    )
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
pub extern "C" fn RimeCleanupStaleSessions() {}

#[no_mangle]
pub extern "C" fn RimeProcessKey(session_id: RimeSessionId, keycode: c_int, mask: c_int) -> Bool {
    if session_id == 0 || mask != 0 {
        return FALSE;
    }
    let Some(key_event) = key_event_from_rime_keycode(keycode) else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };

    let was_composing = !session.engine.context().composition.input.is_empty();
    if let Some(commit) = session.engine.process_key_event(key_event) {
        session.unread_commit = Some(commit);
        return TRUE;
    }

    bool_from(matches!(key_event.code, KeyCode::Character(ch) if ch != ' ') || was_composing)
}

#[no_mangle]
pub extern "C" fn RimeCommitComposition(session_id: RimeSessionId) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    let Some(commit) = session.engine.commit_composition() else {
        return FALSE;
    };

    session.unread_commit = Some(commit);
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeClearComposition(session_id: RimeSessionId) {
    if session_id == 0 {
        return;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    if let Some(session) = registry.sessions.get_mut(&session_id) {
        session.engine.clear_composition();
    }
}

#[no_mangle]
pub extern "C" fn RimeGetInput(session_id: RimeSessionId) -> *const c_char {
    if session_id == 0 {
        return ptr::null();
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return ptr::null();
    };
    let Ok(input) = CString::new(session.engine.context().composition.input.as_str()) else {
        return ptr::null();
    };
    session.input_buffer = Some(input);
    session
        .input_buffer
        .as_ref()
        .map_or(ptr::null(), |input| input.as_ptr())
}

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

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    session.engine.set_input(input);
    session.input_buffer = None;
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeGetCaretPos(session_id: RimeSessionId) -> usize {
    if session_id == 0 {
        return 0;
    }

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    registry
        .sessions
        .get(&session_id)
        .map_or(0, |session| session.engine.context().composition.caret)
}

#[no_mangle]
pub extern "C" fn RimeSetCaretPos(session_id: RimeSessionId, caret_pos: usize) {
    if session_id == 0 {
        return;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    if let Some(session) = registry.sessions.get_mut(&session_id) {
        session.engine.set_caret_pos(caret_pos);
    }
}

/// Sets a session-scoped runtime option.
///
/// # Safety
///
/// `option` must be either null or point to a valid, nul-terminated C string.
/// Null option names are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeSetOption(
    session_id: RimeSessionId,
    option: *const c_char,
    value: Bool,
) {
    if option.is_null() {
        return;
    }
    // SAFETY: callers promise that `option` is a valid nul-terminated string.
    let option = unsafe { CStr::from_ptr(option) }
        .to_string_lossy()
        .into_owned();
    let _ = with_session(session_id, |session| {
        session.engine.set_option(option, value != FALSE);
        true
    });
}

/// Returns the current value of a session-scoped runtime option.
///
/// # Safety
///
/// `option` must be either null or point to a valid, nul-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeGetOption(session_id: RimeSessionId, option: *const c_char) -> Bool {
    if option.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `option` is a valid nul-terminated string.
    let option = unsafe { CStr::from_ptr(option) };
    with_session(session_id, |session| {
        session.engine.get_option(&option.to_string_lossy())
    })
}

/// Sets a session-scoped string property.
///
/// # Safety
///
/// `property` and `value` must be either null or point to valid,
/// nul-terminated C strings. Null inputs are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeSetProperty(
    session_id: RimeSessionId,
    property: *const c_char,
    value: *const c_char,
) {
    if property.is_null() || value.is_null() {
        return;
    }
    // SAFETY: callers promise that both pointers are valid nul-terminated
    // strings.
    let property = unsafe { CStr::from_ptr(property) }
        .to_string_lossy()
        .into_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    let _ = with_session(session_id, |session| {
        session.engine.set_property(property, value);
        true
    });
}

/// Copies a session-scoped string property into caller-provided storage.
///
/// # Safety
///
/// `property` must point to a valid nul-terminated C string, and `value` must
/// point to writable storage of `buffer_size` bytes. Null or empty values are
/// rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeGetProperty(
    session_id: RimeSessionId,
    property: *const c_char,
    value: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if property.is_null() || value.is_null() || buffer_size == 0 {
        return FALSE;
    }
    // SAFETY: callers promise that `property` is a valid nul-terminated string.
    let property = unsafe { CStr::from_ptr(property) };

    with_session(session_id, |session| {
        let Some(property_value) = session.engine.get_property(&property.to_string_lossy()) else {
            return false;
        };
        if property_value.is_empty() {
            return false;
        }
        copy_c_string_to_buffer(property_value, value, buffer_size);
        true
    })
}

/// Copies the current session schema id into caller-provided storage.
///
/// # Safety
///
/// `schema_id` must point to writable storage of `buffer_size` bytes. Null or
/// empty buffers are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCurrentSchema(
    session_id: RimeSessionId,
    schema_id: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if schema_id.is_null() || buffer_size == 0 {
        return FALSE;
    }

    with_session(session_id, |session| {
        let current_schema = session.engine.status().schema_id;
        copy_c_string_to_buffer(&current_schema, schema_id, buffer_size);
        true
    })
}

/// Selects the active schema id for a session.
///
/// # Safety
///
/// `schema_id` must be either null or point to a valid nul-terminated C string.
/// Null schema ids are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeSelectSchema(
    session_id: RimeSessionId,
    schema_id: *const c_char,
) -> Bool {
    if schema_id.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `schema_id` is a valid nul-terminated
    // string.
    let schema_id = unsafe { CStr::from_ptr(schema_id) }
        .to_string_lossy()
        .into_owned();

    with_session(session_id, |session| {
        session.engine.set_schema(schema_id.clone(), schema_id);
        session.engine.clear_composition();
        session.input_buffer = None;
        session.unread_commit = None;
        true
    })
}

/// Returns the currently available schema list.
///
/// # Safety
///
/// `schema_list` must be either null or point to writable storage. When this
/// function returns `TRUE`, the caller must release nested allocations with
/// `RimeFreeSchemaList`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSchemaList(schema_list: *mut RimeSchemaList) -> Bool {
    if schema_list.is_null() {
        return FALSE;
    }

    clear_schema_list(schema_list);

    let Ok(schema_id) = CString::new(DEFAULT_SCHEMA_ID) else {
        return FALSE;
    };
    let Ok(name) = CString::new(DEFAULT_SCHEMA_NAME) else {
        return FALSE;
    };
    let mut list = vec![RimeSchemaListItem {
        schema_id: schema_id.into_raw(),
        name: name.into_raw(),
        reserved: ptr::null_mut(),
    }];
    let size = list.len();
    let list_ptr = list.as_mut_ptr();
    std::mem::forget(list);

    // SAFETY: `schema_list` is non-null and points to caller-owned writable
    // storage; `list_ptr` owns `size` initialized schema-list items.
    unsafe {
        (*schema_list).size = size;
        (*schema_list).list = list_ptr;
    }
    TRUE
}

/// Frees nested allocations populated by `RimeGetSchemaList`.
///
/// # Safety
///
/// `schema_list` must be either null or a valid pointer. Nested pointers, when
/// non-null, must have been returned by `RimeGetSchemaList` and not already
/// freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeSchemaList(schema_list: *mut RimeSchemaList) {
    if schema_list.is_null() {
        return;
    }

    free_schema_list_fields(schema_list);
    clear_schema_list(schema_list);
}

/// Processes a librime-style key sequence against a session.
///
/// # Safety
///
/// `key_sequence` must be either null or point to a valid, nul-terminated C
/// string. Null or unparsable sequences are rejected without mutating the
/// session.
#[no_mangle]
pub unsafe extern "C" fn RimeSimulateKeySequence(
    session_id: RimeSessionId,
    key_sequence: *const c_char,
) -> Bool {
    if session_id == 0 || key_sequence.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `key_sequence` is a valid nul-terminated
    // string.
    let Ok(key_sequence) = unsafe { CStr::from_ptr(key_sequence) }.to_str() else {
        return FALSE;
    };
    let Ok(key_events) = parse_key_sequence(key_sequence) else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };

    for key_event in key_events {
        if let Some(commit) = session.engine.process_key_event(key_event) {
            session.unread_commit = Some(commit);
        }
    }
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeSelectCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    commit_selected_candidate(session_id, |session| session.engine.select_candidate(index))
}

#[no_mangle]
pub extern "C" fn RimeSelectCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    commit_selected_candidate(session_id, |session| {
        session.engine.select_candidate_on_current_page(index)
    })
}

#[no_mangle]
pub extern "C" fn RimeHighlightCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    with_session(session_id, |session| {
        session.engine.highlight_candidate(index)
    })
}

#[no_mangle]
pub extern "C" fn RimeHighlightCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    with_session(session_id, |session| {
        session.engine.highlight_candidate_on_current_page(index)
    })
}

#[no_mangle]
pub extern "C" fn RimeChangePage(session_id: RimeSessionId, backward: Bool) -> Bool {
    with_session(session_id, |session| {
        session.engine.change_page(backward != FALSE)
    })
}

/// Copies the unread commit text for a session into a caller-provided commit.
///
/// # Safety
///
/// `commit` must be either null or a valid, writable pointer to a `RimeCommit`.
/// When this function returns `TRUE`, the caller must release `commit.text` by
/// passing the same commit object to `RimeFreeCommit`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCommit(session_id: RimeSessionId, commit: *mut RimeCommit) -> Bool {
    if commit.is_null() {
        return FALSE;
    }

    clear_commit(commit);

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    let Some(text) = session.unread_commit.take() else {
        return FALSE;
    };
    let Ok(text) = CString::new(text) else {
        return FALSE;
    };

    // SAFETY: `commit` is non-null and points to caller-owned writable storage.
    unsafe {
        (*commit).data_size = std::mem::size_of::<RimeCommit>() as c_int;
        (*commit).text = text.into_raw();
    }
    TRUE
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
    if context.is_null() {
        return FALSE;
    }
    // SAFETY: `context` is non-null and points to caller-owned storage.
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    clear_context(context);

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get(&session_id) else {
        return FALSE;
    };

    let snapshot = session.engine.snapshot();
    let composition = snapshot.context.composition;
    if !composition.input.is_empty() {
        let Ok(preedit) = CString::new(composition.preedit) else {
            return FALSE;
        };
        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `preedit` is converted into owned C storage for the caller.
        unsafe {
            (*context).composition.length = composition.input.len() as c_int;
            (*context).composition.cursor_pos = composition.caret as c_int;
            (*context).composition.sel_start = 0;
            (*context).composition.sel_end = composition.input.len() as c_int;
            (*context).composition.preedit = preedit.into_raw();
        }
    }

    let candidates = snapshot.context.candidates;
    if !candidates.is_empty() {
        let highlighted = snapshot.context.highlighted;
        let page_no = highlighted / DEFAULT_PAGE_SIZE;
        let page_start = page_no * DEFAULT_PAGE_SIZE;
        let page_end = (page_start + DEFAULT_PAGE_SIZE).min(candidates.len());
        let page_candidates = &candidates[page_start..page_end];

        let mut rime_candidates = Vec::with_capacity(page_candidates.len());
        for candidate in page_candidates {
            let Ok(text) = CString::new(candidate.text.as_str()) else {
                free_rime_candidates(&mut rime_candidates);
                return FALSE;
            };
            let comment = if candidate.comment.is_empty() {
                ptr::null_mut()
            } else {
                let Ok(comment) = CString::new(candidate.comment.as_str()) else {
                    free_rime_candidates(&mut rime_candidates);
                    return FALSE;
                };
                comment.into_raw()
            };
            rime_candidates.push(RimeCandidate {
                text: text.into_raw(),
                comment,
                reserved: ptr::null_mut(),
            });
        }

        let num_candidates = rime_candidates.len();
        let candidates_ptr = rime_candidates.as_mut_ptr();
        std::mem::forget(rime_candidates);

        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `candidates_ptr` owns `num_candidates` initialized entries.
        unsafe {
            (*context).menu.page_size = DEFAULT_PAGE_SIZE as c_int;
            (*context).menu.page_no = page_no as c_int;
            (*context).menu.is_last_page = bool_from(page_end == candidates.len());
            (*context).menu.highlighted_candidate_index =
                (highlighted - page_start).min(num_candidates.saturating_sub(1)) as c_int;
            (*context).menu.num_candidates = num_candidates as c_int;
            (*context).menu.candidates = candidates_ptr;
        }
    }

    TRUE
}

/// Copies current session status into caller storage.
///
/// # Safety
///
/// `status` must be either null or a valid, writable pointer to a
/// `RimeStatus` initialized with a positive `data_size`. When this function
/// returns `TRUE`, the caller must release nested strings by passing the same
/// status object to `RimeFreeStatus`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStatus(session_id: RimeSessionId, status: *mut RimeStatus) -> Bool {
    if status.is_null() {
        return FALSE;
    }
    // SAFETY: `status` is non-null and points to caller-owned storage.
    if unsafe { (*status).data_size } <= 0 {
        return FALSE;
    }

    clear_status(status);

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get(&session_id) else {
        return FALSE;
    };
    let snapshot = session.engine.status();
    let Ok(schema_id) = CString::new(snapshot.schema_id) else {
        return FALSE;
    };
    let Ok(schema_name) = CString::new(snapshot.schema_name) else {
        return FALSE;
    };

    // SAFETY: `status` is non-null and points to caller-owned writable storage;
    // schema strings are converted into owned C storage for the caller.
    unsafe {
        (*status).schema_id = schema_id.into_raw();
        (*status).schema_name = schema_name.into_raw();
        (*status).is_disabled = bool_from(snapshot.is_disabled);
        (*status).is_composing = bool_from(snapshot.is_composing);
        (*status).is_ascii_mode = bool_from(snapshot.is_ascii_mode);
        (*status).is_full_shape = bool_from(snapshot.is_full_shape);
        (*status).is_simplified = bool_from(snapshot.is_simplified);
        (*status).is_traditional = bool_from(snapshot.is_traditional);
        (*status).is_ascii_punct = bool_from(snapshot.is_ascii_punct);
    }
    TRUE
}

/// Initializes an iterator over the current candidate list from the first item.
///
/// # Safety
///
/// `iterator` must be either null or a valid, writable pointer to a
/// `RimeCandidateListIterator`. When this function returns `TRUE`, the caller
/// must eventually pass the same iterator to `RimeCandidateListEnd`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListBegin(
    session_id: RimeSessionId,
    iterator: *mut RimeCandidateListIterator,
) -> Bool {
    // SAFETY: forwarded preconditions are identical to
    // `RimeCandidateListFromIndex` with a zero start index.
    unsafe { RimeCandidateListFromIndex(session_id, iterator, 0) }
}

/// Initializes an iterator over the current candidate list from `index`.
///
/// # Safety
///
/// `iterator` must be either null or a valid, writable pointer to a
/// `RimeCandidateListIterator`. When this function returns `TRUE`, the caller
/// must eventually pass the same iterator to `RimeCandidateListEnd`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListFromIndex(
    session_id: RimeSessionId,
    iterator: *mut RimeCandidateListIterator,
    index: c_int,
) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get(&session_id) else {
        return FALSE;
    };
    let candidates = session.engine.context().candidates.clone();
    if candidates.is_empty() {
        return FALSE;
    }

    let state = Box::new(CandidateListState { candidates });
    // SAFETY: `iterator` is non-null and points to caller-owned writable
    // storage. The boxed state is released by `RimeCandidateListEnd`.
    unsafe {
        (*iterator).ptr = Box::into_raw(state).cast::<c_void>();
        (*iterator).index = index.saturating_sub(1);
        (*iterator).candidate = RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        };
    }
    TRUE
}

/// Advances a candidate list iterator and copies the current candidate.
///
/// # Safety
///
/// `iterator` must be either null or a valid pointer previously initialized by
/// `RimeCandidateListBegin` or `RimeCandidateListFromIndex`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListNext(iterator: *mut RimeCandidateListIterator) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }
    // SAFETY: `iterator` is non-null and points to caller-owned storage.
    let state = unsafe { (*iterator).ptr.cast::<CandidateListState>().as_ref() };
    let Some(state) = state else {
        return FALSE;
    };

    // SAFETY: `iterator` is non-null and any current candidate strings were
    // allocated by this API during an earlier successful `Next`.
    unsafe {
        free_candidate_fields(&mut (*iterator).candidate);
        (*iterator).index = (*iterator).index.saturating_add(1);
        if (*iterator).index < 0 {
            return FALSE;
        }
    }

    // SAFETY: index was checked non-negative above.
    let candidate_index = unsafe { (*iterator).index as usize };
    let Some(candidate) = state.candidates.get(candidate_index) else {
        return FALSE;
    };
    let Ok(text) = CString::new(candidate.text.as_str()) else {
        return FALSE;
    };
    let comment = if candidate.comment.is_empty() {
        ptr::null_mut()
    } else {
        let Ok(comment) = CString::new(candidate.comment.as_str()) else {
            return FALSE;
        };
        comment.into_raw()
    };

    // SAFETY: `iterator` is non-null and points to caller-owned writable
    // storage; strings are now owned by the iterator until next/end.
    unsafe {
        (*iterator).candidate = RimeCandidate {
            text: text.into_raw(),
            comment,
            reserved: ptr::null_mut(),
        };
    }
    TRUE
}

/// Frees a candidate list iterator initialized by this API.
///
/// # Safety
///
/// `iterator` must be either null or a valid pointer. Any non-null nested
/// pointers must have been returned by candidate-list iterator APIs.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListEnd(iterator: *mut RimeCandidateListIterator) {
    if iterator.is_null() {
        return;
    }

    // SAFETY: `iterator` is non-null and nested pointers are owned by this API
    // when populated by candidate-list iterator calls.
    unsafe {
        if !(*iterator).ptr.is_null() {
            drop(Box::from_raw((*iterator).ptr.cast::<CandidateListState>()));
        }
        free_candidate_fields(&mut (*iterator).candidate);
        (*iterator).ptr = ptr::null_mut();
        (*iterator).index = 0;
        (*iterator).candidate = RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        };
    }
}

/// Frees nested allocations populated by `RimeGetContext`.
///
/// # Safety
///
/// `context` must be either null or a valid, writable pointer to a
/// `RimeContext`. Nested pointers, when non-null, must have been returned by
/// `RimeGetContext` and not already freed.
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

/// Frees nested allocations populated by `RimeGetStatus`.
///
/// # Safety
///
/// `status` must be either null or a valid, writable pointer to a
/// `RimeStatus`. Nested pointers, when non-null, must have been returned by
/// `RimeGetStatus` and not already freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeStatus(status: *mut RimeStatus) -> Bool {
    if status.is_null() {
        return FALSE;
    }
    // SAFETY: `status` is non-null and points to caller-owned storage.
    if unsafe { (*status).data_size } <= 0 {
        return FALSE;
    }

    free_status_fields(status);
    clear_status(status);
    TRUE
}

/// Frees a commit object populated by `RimeGetCommit`.
///
/// # Safety
///
/// `commit` must be either null or a valid, writable pointer to a `RimeCommit`.
/// If `commit.text` is non-null, it must be a pointer previously returned by
/// `RimeGetCommit` and not already freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeCommit(commit: *mut RimeCommit) -> Bool {
    if commit.is_null() {
        return FALSE;
    }

    // SAFETY: `commit` is non-null and any non-null `text` pointer is owned by
    // this API because it was returned from `CString::into_raw` in `RimeGetCommit`.
    unsafe {
        if !(*commit).text.is_null() {
            drop(CString::from_raw((*commit).text));
        }
    }
    clear_commit(commit);
    TRUE
}

fn key_event_from_rime_keycode(keycode: c_int) -> Option<KeyEvent> {
    let code = match keycode {
        XK_BACKSPACE => KeyCode::Backspace,
        XK_RETURN => KeyCode::Return,
        0x20..=0x7e => KeyCode::Character(char::from_u32(keycode as u32)?),
        _ => return None,
    };

    Some(KeyEvent {
        code,
        modifiers: KeyModifiers::default(),
    })
}

fn commit_selected_candidate(
    session_id: RimeSessionId,
    select: impl FnOnce(&mut SessionState) -> Option<String>,
) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    let Some(commit) = select(session) else {
        return FALSE;
    };

    session.unread_commit = Some(commit);
    TRUE
}

fn with_session(session_id: RimeSessionId, action: impl FnOnce(&mut SessionState) -> bool) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };

    bool_from(action(session))
}

fn optional_c_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }

    // SAFETY: callers validate that non-null optional runtime trait strings are
    // valid NUL-terminated C strings before reaching this helper.
    Some(
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn cstring_from_lossless_str(value: &str) -> CString {
    CString::new(value).expect("values derived from C strings or literals cannot contain NUL bytes")
}

fn path_join(base: &str, child: &str) -> String {
    Path::new(base).join(child).to_string_lossy().into_owned()
}

fn runtime_path_ptr(select: impl FnOnce(&RuntimePaths) -> &CString) -> *const c_char {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    select(&paths).as_ptr()
}

fn copy_runtime_path_to_buffer(
    select: impl FnOnce(&RuntimePaths) -> &CString,
    output: *mut c_char,
    buffer_size: usize,
) {
    if output.is_null() || buffer_size == 0 {
        return;
    }

    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    let value = select(&paths).to_string_lossy();
    copy_c_string_to_buffer(&value, output, buffer_size);
}

fn clear_commit(commit: *mut RimeCommit) {
    // SAFETY: callers only pass non-null pointers to this helper; fields are
    // plain integers/pointers and assigning null mirrors librime's clear macro.
    unsafe {
        (*commit).data_size = 0;
        (*commit).text = ptr::null_mut();
    }
}

fn clear_context(context: *mut RimeContext) {
    // SAFETY: callers only pass non-null pointers to this helper; this mirrors
    // librime's versioned struct clear by preserving `data_size`.
    unsafe {
        (*context).composition = RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: ptr::null_mut(),
        };
        (*context).menu = RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        };
        (*context).commit_text_preview = ptr::null_mut();
        (*context).select_labels = ptr::null_mut();
    }
}

fn clear_status(status: *mut RimeStatus) {
    // SAFETY: callers only pass non-null pointers to this helper; this mirrors
    // librime's versioned struct clear by preserving `data_size`.
    unsafe {
        (*status).schema_id = ptr::null_mut();
        (*status).schema_name = ptr::null_mut();
        (*status).is_disabled = FALSE;
        (*status).is_composing = FALSE;
        (*status).is_ascii_mode = FALSE;
        (*status).is_full_shape = FALSE;
        (*status).is_simplified = FALSE;
        (*status).is_traditional = FALSE;
        (*status).is_ascii_punct = FALSE;
    }
}

fn clear_schema_list(schema_list: *mut RimeSchemaList) {
    // SAFETY: callers only pass non-null pointers to this helper; fields are
    // plain integers/pointers and assigning null mirrors librime cleanup.
    unsafe {
        (*schema_list).size = 0;
        (*schema_list).list = ptr::null_mut();
    }
}

fn free_context_fields(context: *mut RimeContext) {
    // SAFETY: `context` is non-null and nested pointers are owned by this API
    // when populated by `RimeGetContext`.
    unsafe {
        if !(*context).composition.preedit.is_null() {
            drop(CString::from_raw((*context).composition.preedit));
        }
        if !(*context).menu.candidates.is_null() && (*context).menu.num_candidates > 0 {
            let num_candidates = (*context).menu.num_candidates as usize;
            let mut candidates =
                Vec::from_raw_parts((*context).menu.candidates, num_candidates, num_candidates);
            free_rime_candidates(&mut candidates);
        }
        if !(*context).menu.select_keys.is_null() {
            drop(CString::from_raw((*context).menu.select_keys));
        }
        if !(*context).commit_text_preview.is_null() {
            drop(CString::from_raw((*context).commit_text_preview));
        }
        if !(*context).select_labels.is_null() {
            let page_size = (*context).menu.page_size.max(0) as usize;
            let labels = Vec::from_raw_parts((*context).select_labels, page_size, page_size);
            for label in labels {
                if !label.is_null() {
                    drop(CString::from_raw(label));
                }
            }
        }
    }
}

fn free_schema_list_fields(schema_list: *mut RimeSchemaList) {
    // SAFETY: `schema_list` is non-null and nested pointers are owned by this
    // API when populated by `RimeGetSchemaList`.
    unsafe {
        if (*schema_list).list.is_null() {
            return;
        }
        let size = (*schema_list).size;
        let list = Vec::from_raw_parts((*schema_list).list, size, size);
        for item in list {
            if !item.schema_id.is_null() {
                drop(CString::from_raw(item.schema_id));
            }
            if !item.name.is_null() {
                drop(CString::from_raw(item.name));
            }
        }
    }
}

fn free_status_fields(status: *mut RimeStatus) {
    // SAFETY: `status` is non-null and nested pointers are owned by this API
    // when populated by `RimeGetStatus`.
    unsafe {
        if !(*status).schema_id.is_null() {
            drop(CString::from_raw((*status).schema_id));
        }
        if !(*status).schema_name.is_null() {
            drop(CString::from_raw((*status).schema_name));
        }
    }
}

fn free_rime_candidates(candidates: &mut Vec<RimeCandidate>) {
    for mut candidate in candidates.drain(..) {
        free_candidate_fields(&mut candidate);
    }
}

fn free_candidate_fields(candidate: &mut RimeCandidate) {
    if !candidate.text.is_null() {
        // SAFETY: candidate text pointers were returned by CString::into_raw
        // while populating a RimeContext or candidate-list iterator.
        unsafe {
            drop(CString::from_raw(candidate.text));
        }
        candidate.text = ptr::null_mut();
    }
    if !candidate.comment.is_null() {
        // SAFETY: candidate comment pointers were returned by CString::into_raw
        // while populating a RimeContext or candidate-list iterator.
        unsafe {
            drop(CString::from_raw(candidate.comment));
        }
        candidate.comment = ptr::null_mut();
    }
    candidate.reserved = ptr::null_mut();
}

fn copy_c_string_to_buffer(value: &str, output: *mut c_char, buffer_size: usize) {
    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(buffer_size.saturating_sub(1));
    // SAFETY: callers pass writable storage of `buffer_size` bytes, and
    // `copy_len` is bounded to leave room for a trailing NUL.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), output, copy_len);
        slice::from_raw_parts_mut(output.cast::<u8>(), buffer_size)[copy_len] = 0;
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    use yune_core::StaticTableTranslator;

    use super::{
        bool_from, RimeCandidateListBegin, RimeCandidateListEnd, RimeCandidateListFromIndex,
        RimeCandidateListIterator, RimeCandidateListNext, RimeChangePage, RimeCleanupAllSessions,
        RimeCleanupStaleSessions, RimeClearComposition, RimeCommit, RimeCommitComposition,
        RimeContext, RimeCreateSession, RimeDeployConfigFile, RimeDeploySchema,
        RimeDeployWorkspace, RimeDeployerInitialize, RimeDestroySession, RimeFinalize,
        RimeFindSession, RimeFreeCommit, RimeFreeContext, RimeFreeStatus, RimeGetCaretPos,
        RimeGetCommit, RimeGetContext, RimeGetCurrentSchema, RimeGetInput, RimeGetOption,
        RimeGetPrebuiltDataDir, RimeGetPrebuiltDataDirSecure, RimeGetProperty, RimeGetSchemaList,
        RimeGetSharedDataDir, RimeGetSharedDataDirSecure, RimeGetStagingDir,
        RimeGetStagingDirSecure, RimeGetStatus, RimeGetSyncDir, RimeGetSyncDirSecure,
        RimeGetUserDataDir, RimeGetUserDataDirSecure, RimeGetUserDataSyncDir, RimeGetUserId,
        RimeGetVersion, RimeHighlightCandidate, RimeHighlightCandidateOnCurrentPage,
        RimeInitialize, RimeIsMaintenancing, RimeJoinMaintenanceThread, RimePrebuildAllSchemas,
        RimeProcessKey, RimeRunTask, RimeSelectCandidate, RimeSelectCandidateOnCurrentPage,
        RimeSelectSchema, RimeSetCaretPos, RimeSetInput, RimeSetOption, RimeSetProperty, RimeSetup,
        RimeSetupLogging, RimeSimulateKeySequence, RimeStartMaintenance,
        RimeStartMaintenanceOnWorkspaceChange, RimeStatus, RimeSyncUserData, RimeTraits, FALSE,
        TRUE,
    };

    fn test_guard() -> MutexGuard<'static, ()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("test lock should not be poisoned")
    }

    fn empty_context() -> RimeContext {
        RimeContext {
            data_size: (std::mem::size_of::<RimeContext>() - std::mem::size_of::<i32>()) as i32,
            composition: super::RimeComposition {
                length: 0,
                cursor_pos: 0,
                sel_start: 0,
                sel_end: 0,
                preedit: std::ptr::null_mut(),
            },
            menu: super::RimeMenu {
                page_size: 0,
                page_no: 0,
                is_last_page: FALSE,
                highlighted_candidate_index: 0,
                num_candidates: 0,
                candidates: std::ptr::null_mut(),
                select_keys: std::ptr::null_mut(),
            },
            commit_text_preview: std::ptr::null_mut(),
            select_labels: std::ptr::null_mut(),
        }
    }

    fn empty_status() -> RimeStatus {
        RimeStatus {
            data_size: (std::mem::size_of::<RimeStatus>() - std::mem::size_of::<i32>()) as i32,
            schema_id: std::ptr::null_mut(),
            schema_name: std::ptr::null_mut(),
            is_disabled: FALSE,
            is_composing: FALSE,
            is_ascii_mode: FALSE,
            is_full_shape: FALSE,
            is_simplified: FALSE,
            is_traditional: FALSE,
            is_ascii_punct: FALSE,
        }
    }

    fn empty_candidate_list_iterator() -> RimeCandidateListIterator {
        RimeCandidateListIterator {
            ptr: std::ptr::null_mut(),
            index: 0,
            candidate: super::RimeCandidate {
                text: std::ptr::null_mut(),
                comment: std::ptr::null_mut(),
                reserved: std::ptr::null_mut(),
            },
        }
    }

    fn empty_schema_list() -> super::RimeSchemaList {
        super::RimeSchemaList {
            size: 0,
            list: std::ptr::null_mut(),
        }
    }

    fn empty_traits() -> RimeTraits {
        RimeTraits {
            data_size: std::mem::size_of::<RimeTraits>() as i32,
            shared_data_dir: std::ptr::null(),
            user_data_dir: std::ptr::null(),
            distribution_name: std::ptr::null(),
            distribution_code_name: std::ptr::null(),
            distribution_version: std::ptr::null(),
            app_name: std::ptr::null(),
            modules: std::ptr::null(),
            min_log_level: 0,
            log_dir: std::ptr::null(),
            prebuilt_data_dir: std::ptr::null(),
            staging_dir: std::ptr::null(),
        }
    }

    #[test]
    fn maps_bool_to_rime_bool() {
        assert_eq!(bool_from(true), TRUE);
        assert_eq!(bool_from(false), FALSE);
    }

    #[test]
    fn setup_and_initialize_expose_runtime_metadata_paths() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let shared = CString::new("/tmp/yune-shared").expect("path should be valid");
        let user = CString::new("/tmp/yune-user").expect("path should be valid");
        let staging = CString::new("/tmp/yune-stage").expect("path should be valid");
        let mut traits = empty_traits();
        traits.shared_data_dir = shared.as_ptr();
        traits.user_data_dir = user.as_ptr();
        traits.staging_dir = staging.as_ptr();
        let mut buffer = vec![0 as c_char; 64];
        let mut short_buffer = vec![0 as c_char; 10];

        // SAFETY: traits points to a valid RimeTraits object with valid strings.
        unsafe { RimeSetup(&traits) };

        let version = RimeGetVersion();
        assert!(!version.is_null());
        // SAFETY: version is a static NUL-terminated C string.
        let version = unsafe { CStr::from_ptr(version) };
        assert_eq!(version.to_str(), Ok("yune-rime-api 0.1.0"));

        // SAFETY: runtime path getters return stable process-owned C strings.
        let shared_dir = unsafe { CStr::from_ptr(RimeGetSharedDataDir()) };
        assert_eq!(shared_dir.to_str(), Ok("/tmp/yune-shared"));
        // SAFETY: runtime path getters return stable process-owned C strings.
        let user_dir = unsafe { CStr::from_ptr(RimeGetUserDataDir()) };
        assert_eq!(user_dir.to_str(), Ok("/tmp/yune-user"));
        // SAFETY: runtime path getters return stable process-owned C strings.
        let prebuilt_dir = unsafe { CStr::from_ptr(RimeGetPrebuiltDataDir()) };
        assert_eq!(prebuilt_dir.to_str(), Ok("/tmp/yune-shared/build"));
        // SAFETY: runtime path getters return stable process-owned C strings.
        let staging_dir = unsafe { CStr::from_ptr(RimeGetStagingDir()) };
        assert_eq!(staging_dir.to_str(), Ok("/tmp/yune-stage"));
        // SAFETY: runtime path getters return stable process-owned C strings.
        let sync_dir = unsafe { CStr::from_ptr(RimeGetSyncDir()) };
        assert_eq!(sync_dir.to_str(), Ok("sync"));
        // SAFETY: runtime path getters return stable process-owned C strings.
        let user_id = unsafe { CStr::from_ptr(RimeGetUserId()) };
        assert_eq!(user_id.to_str(), Ok("unknown"));

        // SAFETY: buffers point to writable storage.
        unsafe { RimeGetPrebuiltDataDirSecure(buffer.as_mut_ptr(), buffer.len()) };
        // SAFETY: secure getter wrote a trailing NUL into the buffer.
        let copied_prebuilt = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        assert_eq!(copied_prebuilt.to_str(), Ok("/tmp/yune-shared/build"));

        // SAFETY: buffers point to writable storage.
        unsafe { RimeGetSharedDataDirSecure(short_buffer.as_mut_ptr(), short_buffer.len()) };
        // SAFETY: secure getter wrote a trailing NUL into the short buffer.
        let truncated_shared = unsafe { CStr::from_ptr(short_buffer.as_ptr()) };
        assert_eq!(truncated_shared.to_str(), Ok("/tmp/yune"));

        // SAFETY: buffers point to writable storage.
        unsafe { RimeGetUserDataDirSecure(buffer.as_mut_ptr(), buffer.len()) };
        // SAFETY: secure getter wrote a trailing NUL into the buffer.
        let copied_user = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        assert_eq!(copied_user.to_str(), Ok("/tmp/yune-user"));

        // SAFETY: buffers point to writable storage.
        unsafe { RimeGetStagingDirSecure(buffer.as_mut_ptr(), buffer.len()) };
        // SAFETY: secure getter wrote a trailing NUL into the buffer.
        let copied_staging = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        assert_eq!(copied_staging.to_str(), Ok("/tmp/yune-stage"));

        // SAFETY: buffers point to writable storage.
        unsafe { RimeGetSyncDirSecure(buffer.as_mut_ptr(), buffer.len()) };
        // SAFETY: secure getter wrote a trailing NUL into the buffer.
        let copied_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        assert_eq!(copied_sync.to_str(), Ok("sync"));

        // SAFETY: buffers point to writable storage.
        unsafe { RimeGetUserDataSyncDir(buffer.as_mut_ptr(), buffer.len()) };
        // SAFETY: secure getter wrote a trailing NUL into the buffer.
        let copied_user_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        assert_eq!(copied_user_sync.to_str(), Ok("sync/unknown"));

        let prebuilt = CString::new("/tmp/yune-prebuilt").expect("path should be valid");
        traits.prebuilt_data_dir = prebuilt.as_ptr();
        // SAFETY: traits points to a valid RimeTraits object with valid strings.
        unsafe { RimeInitialize(&traits) };
        // SAFETY: runtime path getters return stable process-owned C strings.
        let prebuilt_dir = unsafe { CStr::from_ptr(RimeGetPrebuiltDataDir()) };
        assert_eq!(prebuilt_dir.to_str(), Ok("/tmp/yune-prebuilt"));

        RimeFinalize();
    }

    #[test]
    fn maintenance_and_deployment_shims_are_deterministic() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let shared = CString::new("/tmp/yune-deployer-shared").expect("path should be valid");
        let schema_file = CString::new("default.schema.yaml").expect("file should be valid");
        let config_file = CString::new("default.yaml").expect("file should be valid");
        let version_key = CString::new("config_version").expect("key should be valid");
        let task_name = CString::new("workspace_update").expect("task should be valid");
        let mut traits = empty_traits();
        traits.shared_data_dir = shared.as_ptr();

        RimeSetupLogging(task_name.as_ptr());
        assert_eq!(RimeStartMaintenance(TRUE), TRUE);
        assert_eq!(RimeStartMaintenanceOnWorkspaceChange(), TRUE);
        assert_eq!(RimeIsMaintenancing(), FALSE);
        RimeJoinMaintenanceThread();

        // SAFETY: traits points to a valid RimeTraits object with valid strings.
        unsafe { RimeDeployerInitialize(&traits) };
        // SAFETY: runtime path getters return stable process-owned C strings.
        let shared_dir = unsafe { CStr::from_ptr(RimeGetSharedDataDir()) };
        assert_eq!(shared_dir.to_str(), Ok("/tmp/yune-deployer-shared"));

        assert_eq!(RimePrebuildAllSchemas(), TRUE);
        assert_eq!(RimeDeployWorkspace(), TRUE);
        assert_eq!(RimeDeploySchema(schema_file.as_ptr()), TRUE);
        assert_eq!(RimeDeploySchema(std::ptr::null()), FALSE);
        assert_eq!(
            RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
            TRUE
        );
        assert_eq!(
            RimeDeployConfigFile(config_file.as_ptr(), std::ptr::null()),
            FALSE
        );
        assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);
        assert_eq!(RimeRunTask(std::ptr::null()), FALSE);

        let session_id = RimeCreateSession();
        assert_eq!(RimeFindSession(session_id), TRUE);
        RimeCleanupStaleSessions();
        assert_eq!(RimeFindSession(session_id), TRUE);
        assert_eq!(RimeSyncUserData(), TRUE);
        assert_eq!(RimeFindSession(session_id), FALSE);
    }

    #[test]
    fn creates_finds_and_destroys_sessions() {
        let _guard = test_guard();
        RimeCleanupAllSessions();

        let session_id = RimeCreateSession();

        assert_ne!(session_id, 0);
        assert_eq!(RimeFindSession(session_id), TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        assert_eq!(RimeFindSession(session_id), FALSE);
    }

    #[test]
    fn processes_ascii_keys_and_returns_unread_commit_once() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };

        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
        assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated `text` with a
        // valid NUL-terminated C string owned by the commit object.
        let text = unsafe { CStr::from_ptr(commit.text) };
        assert_eq!(text.to_str(), Ok("ni"));
        // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        assert!(commit.text.is_null());
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn process_key_commits_numeric_candidate_selection() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        {
            let mut registry = super::sessions()
                .lock()
                .expect("session registry should not be poisoned");
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("session should exist");
            session
                .engine
                .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
        }
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };

        assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, '2' as i32, 0), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated `text` with a
        // valid NUL-terminated C string owned by the commit object.
        let text = unsafe { CStr::from_ptr(commit.text) };
        assert_eq!(text.to_str(), Ok("吧"));
        // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn select_candidate_apis_commit_current_candidates() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        {
            let mut registry = super::sessions()
                .lock()
                .expect("session registry should not be poisoned");
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("session should exist");
            session
                .engine
                .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
        }
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };

        assert_eq!(RimeSelectCandidate(session_id, 0), FALSE);
        assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
        assert_eq!(RimeSelectCandidate(session_id, 1), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated `text`.
        let text = unsafe { CStr::from_ptr(commit.text) };
        assert_eq!(text.to_str(), Ok("吧"));
        // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

        assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
        assert_eq!(RimeSelectCandidateOnCurrentPage(session_id, 0), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated `text`.
        let text = unsafe { CStr::from_ptr(commit.text) };
        assert_eq!(text.to_str(), Ok("八"));
        // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn highlight_candidate_apis_move_selection_without_commit() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        {
            let mut registry = super::sessions()
                .lock()
                .expect("session registry should not be poisoned");
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("session should exist");
            session.engine.add_translator(StaticTableTranslator::new([
                ("ba", "八"),
                ("ba", "吧"),
                ("ba", "爸"),
                ("ba", "巴"),
                ("ba", "把"),
                ("ba", "拔"),
            ]));
        }
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        let mut context = empty_context();

        assert_eq!(RimeHighlightCandidate(session_id, 0), FALSE);
        assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
        assert_eq!(RimeHighlightCandidate(session_id, 1), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
        // SAFETY: `context` points to writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.menu.page_no, 0);
        assert_eq!(context.menu.highlighted_candidate_index, 1);
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

        assert_eq!(RimeChangePage(session_id, FALSE), TRUE);
        // SAFETY: `context` points to writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.menu.page_no, 1);
        assert_eq!(context.menu.highlighted_candidate_index, 1);
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

        assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 0), TRUE);
        assert_eq!(RimeHighlightCandidateOnCurrentPage(session_id, 5), FALSE);
        // SAFETY: `context` points to writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.menu.page_no, 1);
        assert_eq!(context.menu.highlighted_candidate_index, 0);
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

        assert_eq!(RimeChangePage(session_id, TRUE), TRUE);
        assert_eq!(RimeSelectCandidate(session_id, 0), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated `text`.
        let text = unsafe { CStr::from_ptr(commit.text) };
        assert_eq!(text.to_str(), Ok("八"));
        // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn commits_composition_explicitly_and_returns_unread_commit() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        {
            let mut registry = super::sessions()
                .lock()
                .expect("session registry should not be poisoned");
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("session should exist");
            session
                .engine
                .add_translator(StaticTableTranslator::new([("ni", "你")]));
        }
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        let mut context = empty_context();

        assert_eq!(RimeCommitComposition(session_id), FALSE);
        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
        assert_eq!(RimeCommitComposition(session_id), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated `text`.
        let text = unsafe { CStr::from_ptr(commit.text) };
        assert_eq!(text.to_str(), Ok("你"));
        // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

        // SAFETY: `context` points to writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.composition.length, 0);
        assert_eq!(context.menu.num_candidates, 0);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn clears_composition_without_creating_commit() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        let mut context = empty_context();

        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
        RimeClearComposition(session_id);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
        // SAFETY: `context` points to writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.composition.length, 0);
        assert_eq!(context.menu.num_candidates, 0);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn gets_and_sets_input_and_caret_position() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        {
            let mut registry = super::sessions()
                .lock()
                .expect("session registry should not be poisoned");
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("session should exist");
            session
                .engine
                .add_translator(StaticTableTranslator::new([("ni", "你")]));
        }
        let mut context = empty_context();

        assert_eq!(RimeGetInput(0), std::ptr::null());
        assert_eq!(RimeGetCaretPos(0), 0);
        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
        assert_eq!(RimeGetCaretPos(session_id), 2);

        let input = RimeGetInput(session_id);
        assert!(!input.is_null());
        // SAFETY: `RimeGetInput` returned a non-null session-owned C string.
        let input = unsafe { CStr::from_ptr(input) };
        assert_eq!(input.to_str(), Ok("ni"));

        RimeSetCaretPos(session_id, 1);
        assert_eq!(RimeGetCaretPos(session_id), 1);
        RimeSetCaretPos(session_id, 10);
        assert_eq!(RimeGetCaretPos(session_id), 2);

        let new_input = CString::new("ni").expect("literal should not contain NUL");
        // SAFETY: `new_input` is a valid NUL-terminated C string.
        assert_eq!(
            unsafe { RimeSetInput(session_id, new_input.as_ptr()) },
            TRUE
        );
        assert_eq!(RimeGetCaretPos(session_id), 2);
        // SAFETY: `context` points to writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.menu.num_candidates, 2);
        // SAFETY: `context.menu.candidates` points to initialized candidates.
        let first_candidate = unsafe { *context.menu.candidates };
        // SAFETY: candidate text is a valid NUL-terminated string owned by the
        // context object.
        let first_candidate_text = unsafe { CStr::from_ptr(first_candidate.text) };
        assert_eq!(first_candidate_text.to_str(), Ok("你"));
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

        // SAFETY: null pointers are explicitly rejected.
        assert_eq!(unsafe { RimeSetInput(session_id, std::ptr::null()) }, FALSE);
        // SAFETY: `new_input` is a valid NUL-terminated C string.
        assert_eq!(
            unsafe { RimeSetInput(session_id + 1, new_input.as_ptr()) },
            FALSE
        );

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn returns_context_with_preedit_and_candidate_page() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut context = empty_context();

        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);

        // SAFETY: `context` points to valid writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.composition.length, 2);
        assert_eq!(context.composition.cursor_pos, 2);
        assert_eq!(context.composition.sel_start, 0);
        assert_eq!(context.composition.sel_end, 2);
        // SAFETY: `RimeGetContext` returned true and populated owned C strings.
        let preedit = unsafe { CStr::from_ptr(context.composition.preedit) };
        assert_eq!(preedit.to_str(), Ok("ni"));

        assert_eq!(context.menu.page_size, 5);
        assert_eq!(context.menu.page_no, 0);
        assert_eq!(context.menu.is_last_page, TRUE);
        assert_eq!(context.menu.highlighted_candidate_index, 0);
        assert_eq!(context.menu.num_candidates, 1);
        assert!(!context.menu.candidates.is_null());
        // SAFETY: `context.menu.candidates` points to one initialized candidate.
        let candidate = unsafe { *context.menu.candidates };
        // SAFETY: candidate strings are valid NUL-terminated strings owned by
        // the context object.
        let candidate_text = unsafe { CStr::from_ptr(candidate.text) };
        assert_eq!(candidate_text.to_str(), Ok("ni"));
        // SAFETY: the echo candidate includes a non-null comment.
        let candidate_comment = unsafe { CStr::from_ptr(candidate.comment) };
        assert_eq!(candidate_comment.to_str(), Ok("echo"));

        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert!(context.composition.preedit.is_null());
        assert!(context.menu.candidates.is_null());
        assert_eq!(context.menu.num_candidates, 0);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn iterates_candidate_list_from_current_context() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        {
            let mut registry = super::sessions()
                .lock()
                .expect("session registry should not be poisoned");
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("session should exist");
            session
                .engine
                .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
        }
        let mut iterator = empty_candidate_list_iterator();

        assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
        // SAFETY: `iterator` points to valid writable storage.
        assert_eq!(
            unsafe { RimeCandidateListBegin(session_id, &mut iterator) },
            TRUE
        );
        // SAFETY: `iterator` was initialized by `RimeCandidateListBegin`.
        assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
        // SAFETY: `RimeCandidateListNext` populated a valid C string.
        let first_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
        assert_eq!(first_text.to_str(), Ok("八"));
        // SAFETY: current candidate includes a non-null comment.
        let first_comment = unsafe { CStr::from_ptr(iterator.candidate.comment) };
        assert_eq!(first_comment.to_str(), Ok("ba"));
        // SAFETY: `iterator` remains valid and owns the current candidate.
        assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
        // SAFETY: `RimeCandidateListNext` populated a valid C string.
        let second_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
        assert_eq!(second_text.to_str(), Ok("吧"));
        // SAFETY: `iterator` was initialized by this API and can be ended once.
        unsafe { RimeCandidateListEnd(&mut iterator) };
        assert!(iterator.ptr.is_null());
        assert!(iterator.candidate.text.is_null());
        assert!(iterator.candidate.comment.is_null());

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn candidate_list_can_start_from_index_and_rejects_empty_menu() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        {
            let mut registry = super::sessions()
                .lock()
                .expect("session registry should not be poisoned");
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("session should exist");
            session
                .engine
                .add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
        }
        let mut iterator = empty_candidate_list_iterator();

        // SAFETY: `iterator` points to valid writable storage.
        assert_eq!(
            unsafe { RimeCandidateListBegin(session_id, &mut iterator) },
            FALSE
        );
        assert_eq!(RimeProcessKey(session_id, 'b' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'a' as i32, 0), TRUE);
        // SAFETY: `iterator` points to valid writable storage.
        assert_eq!(
            unsafe { RimeCandidateListFromIndex(session_id, &mut iterator, 1) },
            TRUE
        );
        // SAFETY: `iterator` was initialized by `RimeCandidateListFromIndex`.
        assert_eq!(unsafe { RimeCandidateListNext(&mut iterator) }, TRUE);
        // SAFETY: `RimeCandidateListNext` populated a valid C string.
        let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
        assert_eq!(text.to_str(), Ok("吧"));
        // SAFETY: `iterator` was initialized by this API and can be ended once.
        unsafe { RimeCandidateListEnd(&mut iterator) };

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn returns_status_with_schema_and_composing_flags() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut status = empty_status();

        // SAFETY: `status` points to valid writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
        // SAFETY: `RimeGetStatus` returned true and populated owned C strings.
        let schema_id = unsafe { CStr::from_ptr(status.schema_id) };
        // SAFETY: `RimeGetStatus` returned true and populated owned C strings.
        let schema_name = unsafe { CStr::from_ptr(status.schema_name) };
        assert_eq!(schema_id.to_str(), Ok("default"));
        assert_eq!(schema_name.to_str(), Ok("Default"));
        assert_eq!(status.is_composing, FALSE);
        assert_eq!(status.is_ascii_mode, FALSE);
        // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
        assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);
        assert!(status.schema_id.is_null());
        assert!(status.schema_name.is_null());

        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        // SAFETY: `status` points to valid writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
        assert_eq!(status.is_composing, TRUE);
        // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
        assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn sets_and_gets_runtime_options() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
        let custom_toggle = CString::new("custom_toggle").expect("option name should be valid");
        let mut status = empty_status();

        assert_eq!(
            unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
            FALSE
        );
        // SAFETY: option names are valid nul-terminated C strings.
        unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
        // SAFETY: option names are valid nul-terminated C strings.
        unsafe { RimeSetOption(session_id, custom_toggle.as_ptr(), TRUE) };

        assert_eq!(
            unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
            TRUE
        );
        assert_eq!(
            unsafe { RimeGetOption(session_id, custom_toggle.as_ptr()) },
            TRUE
        );
        // SAFETY: `status` points to valid writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
        assert_eq!(status.is_ascii_mode, TRUE);
        // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
        assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

        // SAFETY: option names are valid nul-terminated C strings.
        unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
        assert_eq!(
            unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
            FALSE
        );
        assert_eq!(unsafe { RimeGetOption(0, ascii_mode.as_ptr()) }, FALSE);
        assert_eq!(
            unsafe { RimeGetOption(session_id, std::ptr::null()) },
            FALSE
        );

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn sets_and_gets_runtime_properties() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let property = CString::new("client_app").expect("property name should be valid");
        let value = CString::new("sample_console").expect("property value should be valid");
        let empty_value = CString::new("").expect("property value should be valid");
        let mut buffer = vec![0 as c_char; 32];

        // SAFETY: property name is valid and buffer points to writable storage.
        assert_eq!(
            unsafe {
                RimeGetProperty(
                    session_id,
                    property.as_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                )
            },
            FALSE
        );

        // SAFETY: property name and value are valid nul-terminated C strings.
        unsafe { RimeSetProperty(session_id, property.as_ptr(), value.as_ptr()) };
        // SAFETY: property name is valid and buffer points to writable storage.
        assert_eq!(
            unsafe {
                RimeGetProperty(
                    session_id,
                    property.as_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                )
            },
            TRUE
        );
        // SAFETY: `RimeGetProperty` returned true and wrote a trailing NUL into
        // the caller-owned buffer.
        let copied_value = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        assert_eq!(copied_value.to_str(), Ok("sample_console"));

        let mut short_buffer = vec![0 as c_char; 7];
        // SAFETY: property name is valid and buffer points to writable storage.
        assert_eq!(
            unsafe {
                RimeGetProperty(
                    session_id,
                    property.as_ptr(),
                    short_buffer.as_mut_ptr(),
                    short_buffer.len(),
                )
            },
            TRUE
        );
        // SAFETY: the shim always NUL-terminates non-empty buffers.
        let truncated_value = unsafe { CStr::from_ptr(short_buffer.as_ptr()) };
        assert_eq!(truncated_value.to_str(), Ok("sample"));

        // SAFETY: empty properties are accepted on set but rejected on get, as
        // librime treats empty property values as absent.
        unsafe { RimeSetProperty(session_id, property.as_ptr(), empty_value.as_ptr()) };
        assert_eq!(
            unsafe {
                RimeGetProperty(
                    session_id,
                    property.as_ptr(),
                    buffer.as_mut_ptr(),
                    buffer.len(),
                )
            },
            FALSE
        );
        assert_eq!(
            unsafe {
                RimeGetProperty(
                    session_id,
                    property.as_ptr(),
                    std::ptr::null_mut(),
                    buffer.len(),
                )
            },
            FALSE
        );
        assert_eq!(
            unsafe { RimeGetProperty(session_id, std::ptr::null(), buffer.as_mut_ptr(), 0) },
            FALSE
        );

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn gets_and_selects_current_schema() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let schema_id = CString::new("sample_schema").expect("schema id should be valid");
        let mut buffer = vec![0 as c_char; 32];
        let mut short_buffer = vec![0 as c_char; 8];
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        let mut context = empty_context();
        let mut status = empty_status();

        // SAFETY: buffer points to writable storage.
        assert_eq!(
            unsafe { RimeGetCurrentSchema(session_id, buffer.as_mut_ptr(), buffer.len()) },
            TRUE
        );
        // SAFETY: `RimeGetCurrentSchema` wrote a trailing NUL into buffer.
        let current_schema = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        assert_eq!(current_schema.to_str(), Ok("default"));

        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
        // SAFETY: schema id is a valid nul-terminated C string.
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
            TRUE
        );

        // SAFETY: selecting a schema clears unread composition state.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
        // SAFETY: context points to writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.composition.length, 0);
        assert_eq!(context.menu.num_candidates, 0);

        // SAFETY: buffer points to writable storage.
        assert_eq!(
            unsafe {
                RimeGetCurrentSchema(session_id, short_buffer.as_mut_ptr(), short_buffer.len())
            },
            TRUE
        );
        // SAFETY: `RimeGetCurrentSchema` wrote a trailing NUL into buffer.
        let truncated_schema = unsafe { CStr::from_ptr(short_buffer.as_ptr()) };
        assert_eq!(truncated_schema.to_str(), Ok("sample_"));

        // SAFETY: status points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
        // SAFETY: `RimeGetStatus` populated owned C strings.
        let status_schema_id = unsafe { CStr::from_ptr(status.schema_id) };
        // SAFETY: `RimeGetStatus` populated owned C strings.
        let status_schema_name = unsafe { CStr::from_ptr(status.schema_name) };
        assert_eq!(status_schema_id.to_str(), Ok("sample_schema"));
        assert_eq!(status_schema_name.to_str(), Ok("sample_schema"));
        // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
        assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

        assert_eq!(
            unsafe { RimeGetCurrentSchema(session_id, std::ptr::null_mut(), 0) },
            FALSE
        );
        assert_eq!(
            unsafe { RimeSelectSchema(session_id, std::ptr::null()) },
            FALSE
        );
        assert_eq!(
            unsafe { RimeSelectSchema(session_id + 1, schema_id.as_ptr()) },
            FALSE
        );

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn gets_and_frees_available_schema_list() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let mut schema_list = empty_schema_list();

        // SAFETY: schema_list points to valid writable storage.
        assert_eq!(unsafe { RimeGetSchemaList(&mut schema_list) }, TRUE);
        assert_eq!(schema_list.size, 1);
        assert!(!schema_list.list.is_null());

        // SAFETY: `RimeGetSchemaList` returned true and populated one item.
        let item = unsafe { *schema_list.list };
        // SAFETY: schema strings are valid NUL-terminated strings owned by the
        // schema-list object.
        let schema_id = unsafe { CStr::from_ptr(item.schema_id) };
        // SAFETY: schema strings are valid NUL-terminated strings owned by the
        // schema-list object.
        let name = unsafe { CStr::from_ptr(item.name) };
        assert_eq!(schema_id.to_str(), Ok("default"));
        assert_eq!(name.to_str(), Ok("Default"));
        assert!(item.reserved.is_null());

        // SAFETY: nested pointers were allocated by `RimeGetSchemaList` above.
        unsafe { super::RimeFreeSchemaList(&mut schema_list) };
        assert_eq!(schema_list.size, 0);
        assert!(schema_list.list.is_null());

        // SAFETY: null pointers are explicitly rejected/no-oped.
        assert_eq!(unsafe { RimeGetSchemaList(std::ptr::null_mut()) }, FALSE);
        unsafe { super::RimeFreeSchemaList(std::ptr::null_mut()) };
    }

    #[test]
    fn simulates_librime_style_key_sequences() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        {
            let mut registry = super::sessions()
                .lock()
                .expect("session registry should not be poisoned");
            let session = registry
                .sessions
                .get_mut(&session_id)
                .expect("session should exist");
            session
                .engine
                .add_translator(StaticTableTranslator::new([("ni", "你")]));
        }
        let sequence = CString::new("ni{space}").expect("key sequence should be valid");
        let invalid_sequence =
            CString::new("x{Unknown}").expect("key sequence should be valid C string");
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };
        let mut context = empty_context();

        // SAFETY: sequence is a valid nul-terminated C string.
        assert_eq!(
            unsafe { RimeSimulateKeySequence(session_id, sequence.as_ptr()) },
            TRUE
        );
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated `text`.
        let text = unsafe { CStr::from_ptr(commit.text) };
        assert_eq!(text.to_str(), Ok("你"));
        // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

        // SAFETY: invalid sequence is a valid C string but should fail parsing.
        assert_eq!(
            unsafe { RimeSimulateKeySequence(session_id, invalid_sequence.as_ptr()) },
            FALSE
        );
        // SAFETY: parse failures should not partially apply the leading `x`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.composition.length, 0);
        assert_eq!(context.menu.num_candidates, 0);
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

        // SAFETY: null and invalid sessions are explicitly rejected.
        assert_eq!(
            unsafe { RimeSimulateKeySequence(session_id, std::ptr::null()) },
            FALSE
        );
        assert_eq!(
            unsafe { RimeSimulateKeySequence(session_id + 1, sequence.as_ptr()) },
            FALSE
        );

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn rejects_invalid_context_requests() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut context = empty_context();
        context.data_size = 0;

        // SAFETY: `context` points to writable storage but has invalid
        // librime-style data_size metadata.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, FALSE);
        // SAFETY: null pointers are explicitly rejected.
        assert_eq!(
            unsafe { RimeGetContext(session_id, std::ptr::null_mut()) },
            FALSE
        );
        // SAFETY: `context` points to writable storage but has invalid
        // librime-style data_size metadata.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, FALSE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn rejects_invalid_status_requests() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut status = empty_status();
        status.data_size = 0;

        // SAFETY: `status` points to writable storage but has invalid
        // librime-style data_size metadata.
        assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, FALSE);
        // SAFETY: null pointers are explicitly rejected.
        assert_eq!(
            unsafe { RimeGetStatus(session_id, std::ptr::null_mut()) },
            FALSE
        );
        // SAFETY: `status` points to writable storage but has invalid
        // librime-style data_size metadata.
        assert_eq!(unsafe { RimeFreeStatus(&mut status) }, FALSE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn rejects_unknown_sessions_and_modified_keys() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();

        assert_eq!(RimeProcessKey(0, 'a' as i32, 0), FALSE);
        assert_eq!(RimeProcessKey(session_id + 1, 'a' as i32, 0), FALSE);
        assert_eq!(RimeProcessKey(session_id, 'a' as i32, 1), FALSE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }
}
