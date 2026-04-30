# Phase 05: UserDB And Scaling Hardening - Pattern Map

**Mapped:** 2026-04-30
**Files analyzed:** 22
**Analogs found:** 22 / 22

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/yune-rime-api/src/userdb.rs` | ABI facade / service | request-response + file-I/O + CRUD | `crates/yune-rime-api/src/userdb.rs` | exact-current |
| `crates/yune-rime-api/src/userdb/mod.rs` | service / manager facade | request-response + file-I/O + CRUD | `crates/yune-rime-api/src/userdb.rs` | role-match |
| `crates/yune-rime-api/src/userdb/record.rs` | model / utility | transform | `crates/yune-core/src/dictionary/compiled.rs` | role-match |
| `crates/yune-rime-api/src/userdb/store.rs` | service trait / storage boundary | CRUD + batch | `crates/yune-rime-api/src/session.rs` + `crates/yune-core/src/lib.rs` | role-match |
| `crates/yune-rime-api/src/userdb/file_store.rs` | service / storage implementation | file-I/O + CRUD + batch | `crates/yune-rime-api/src/deployment.rs` | role-match |
| `crates/yune-rime-api/src/userdb/snapshot.rs` | utility / parser-writer | file-I/O + transform | `crates/yune-rime-api/src/userdb.rs` + `crates/yune-core/src/dictionary/compiled.rs` | role-match |
| `crates/yune-rime-api/src/userdb/sync.rs` | service | file-I/O + batch + transform | `crates/yune-rime-api/src/userdb.rs` | role-match |
| `crates/yune-rime-api/src/userdb/recovery.rs` | service | file-I/O + batch + recovery | `crates/yune-rime-api/src/deployment.rs` | role-match |
| `crates/yune-rime-api/src/runtime.rs` | config / runtime path service | request-response + file-I/O | `crates/yune-rime-api/src/runtime.rs` | exact-current |
| `crates/yune-rime-api/src/deployment.rs` | service / task dispatcher | request-response + batch + file-I/O | `crates/yune-rime-api/src/deployment.rs` | exact-current |
| `crates/yune-rime-api/src/resource_id.rs` | utility / validation | transform | `crates/yune-rime-api/src/resource_id.rs` | exact-current |
| `crates/yune-rime-api/src/session.rs` | store / provider | event-driven + request-response | `crates/yune-rime-api/src/session.rs` | exact-current |
| `crates/yune-rime-api/src/lib.rs` | ABI facade / orchestration glue | event-driven + request-response | `crates/yune-rime-api/src/lib.rs` | exact-current |
| `crates/yune-core/src/userdb.rs` | model / utility / translator support | transform + CRUD lookup | `crates/yune-core/src/dictionary/compiled.rs` + `crates/yune-core/src/translator/mod.rs` | role-match |
| `crates/yune-core/src/engine.rs` | service / engine | event-driven + request-response | `crates/yune-core/src/engine.rs` | exact-current |
| `crates/yune-core/src/state.rs` | model | event-driven state | `crates/yune-core/src/state.rs` | exact-current |
| `crates/yune-core/src/translator/mod.rs` | service / translator | transform + request-response | `crates/yune-core/src/translator/mod.rs` | exact-current |
| `crates/yune-core/src/lib.rs` | facade / trait exports / tests | transform + request-response | `crates/yune-core/src/lib.rs` | role-match |
| `crates/yune-rime-api/src/tests/userdb.rs` | test | file-I/O + request-response | `crates/yune-rime-api/src/tests/userdb.rs` | exact-current |
| `crates/yune-rime-api/src/tests/resource_id.rs` | test | validation + file-I/O | `crates/yune-rime-api/src/tests/resource_id.rs` | exact-current |
| `crates/yune-rime-api/src/tests/deployment.rs` | test | batch + file-I/O | `crates/yune-rime-api/src/tests/deployment.rs` | role-match |
| `crates/yune-rime-api/tests/frontend_client.rs` | integration test | request-response + ABI function-table | `crates/yune-rime-api/tests/frontend_client.rs` | exact-current |

## Pattern Assignments

### `crates/yune-rime-api/src/userdb.rs` (ABI facade/service, request-response + file-I/O)

**Analog:** `crates/yune-rime-api/src/userdb.rs`

**Imports pattern** (lines 0-13):
```rust
use std::{
    collections::HashSet,
    ffi::c_void,
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
};

use crate::{
    bool_from, clear_user_dict_iterator, cstring_from_lossless_str, optional_c_string,
    resource_id::validate_user_dict_name, runtime_paths, Bool, RimeUserDictIterator,
    UserDictListState, FALSE, TRUE,
};
```

**ABI pointer validation + safety-doc pattern** (lines 15-48):
```rust
/// Initializes an iterator over user dictionary names found in `user_data_dir`.
///
/// # Safety
///
/// `iterator` must be null or point to writable `RimeUserDictIterator` storage.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversUserDictIteratorInit(
    iterator: *mut RimeUserDictIterator,
) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }

    let names = deployed_user_dict_names()
        .into_iter()
        .map(|name| cstring_from_lossless_str(&name))
        .collect::<Vec<_>>();
    if names.is_empty() {
        return FALSE;
    }

    // SAFETY: `iterator` is non-null and owned by the caller; if it already
    // holds state from this shim, release it before replacing it. librime does
    // not touch an existing iterator when a new scan finds no dictionaries.
    unsafe { clear_user_dict_iterator(iterator) };
```

**C string boundary + Bool result pattern** (lines 97-108):
```rust
/// Backs up a plain file-backed user dictionary into the user sync directory.
///
/// # Safety
///
/// `dict_name` must be null or point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversBackupUserDict(dict_name: *const c_char) -> Bool {
    let Some(dict_name) = optional_c_string(dict_name) else {
        return FALSE;
    };
    bool_from(backup_plain_user_dict(&dict_name))
}
```

**Resource-ID-before-path-join pattern** (lines 259-267):
```rust
fn user_dict_path(dict_name: &str) -> Option<PathBuf> {
    let dict_name = validate_user_dict_name(dict_name)?;
    Some(runtime_user_data_dir().join(format!("{dict_name}.userdb")))
}

fn user_dict_snapshot_path(dict_name: &str) -> Option<PathBuf> {
    let dict_name = validate_user_dict_name(dict_name)?;
    Some(runtime_user_data_sync_dir().join(format!("{dict_name}.userdb.txt")))
}
```

**Current anti-pattern to replace, not copy** (lines 336-365):
```rust
fn merge_plain_user_dict_snapshot(dict_name: &str, snapshot: &Path) -> Result<(), std::io::Error> {
    let Some(destination) = user_dict_path(dict_name) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid user dictionary name",
        ));
    };
    if !destination.is_file() {
        fs::copy(snapshot, destination)?;
        return Ok(());
    }

    let destination_text = fs::read_to_string(&destination)?;
    let snapshot_text = fs::read_to_string(snapshot)?;
    let mut seen = destination_text
        .lines()
        .map(ToOwned::to_owned)
        .collect::<HashSet<_>>();
    let mut merged = destination_text;
    for line in snapshot_text.lines() {
        if line.trim().is_empty() || !seen.insert(line.to_owned()) {
            continue;
        }
```

**Planner instruction:** keep exported `RimeLevers*UserDict*` signatures, safety docs, pointer checks, and return shapes here. Move storage/snapshot/sync logic behind internal manager methods instead of growing this file.

---

### `crates/yune-rime-api/src/userdb/mod.rs` (manager facade, request-response + file-I/O)

**Analog:** `crates/yune-rime-api/src/userdb.rs`

**Runtime root acquisition pattern** (lines 245-257):
```rust
fn runtime_user_data_dir() -> PathBuf {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned())
}

pub(crate) fn runtime_user_data_sync_dir() -> PathBuf {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    PathBuf::from(paths.user_data_sync_dir.to_string_lossy().into_owned())
}
```

**Manager call sites to preserve** (lines 287-299):
```rust
pub(crate) fn sync_all_user_dicts() -> bool {
    let mut success = true;
    for dict_name in deployed_user_dict_names() {
        if !sync_plain_user_dict(&dict_name) {
            success = false;
        }
    }
    success
}

pub(crate) fn user_dict_upgrade() -> bool {
    true
}
```

**Planner instruction:** model `UserDbManager` as the internal owner of `backup`, `restore`, `import`, `export`, `sync_all`, `upgrade`, `recover`, and transaction orchestration. Preserve `bool`/`c_int` compatibility at public callers; let manager methods use `Result` internally where useful.

---

### `crates/yune-rime-api/src/userdb/record.rs` (model/utility, transform)

**Analog:** `crates/yune-core/src/dictionary/compiled.rs`

**Typed model + pure parser error pattern** (lines 65-97):
```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RimeTableBinMetadata {
    pub dict_file_checksum: u32,
    pub num_syllables: u32,
    pub num_entries: u32,
    pub string_table_size: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RimeCompiledMetadataError {
    TooShort,
    InvalidFormat,
    UnsupportedVersion,
    MissingRequiredSection,
}

pub fn parse_rime_table_bin_metadata(
    bytes: impl AsRef<[u8]>,
) -> Result<RimeTableBinMetadata, RimeCompiledMetadataError> {
```

**Small helper parse pattern** (lines 157-180):
```rust
fn ensure_len(bytes: &[u8], len: usize) -> Result<(), RimeCompiledMetadataError> {
    if bytes.len() < len {
        return Err(RimeCompiledMetadataError::TooShort);
    }
    Ok(())
}

pub(crate) fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, RimeCompiledMetadataError> {
    let end = offset
        .checked_add(4)
        .ok_or(RimeCompiledMetadataError::TooShort)?;
    let Some(value) = bytes.get(offset..end) else {
        return Err(RimeCompiledMetadataError::TooShort);
    };
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}
```

**Planner instruction:** copy this style for `UserDbValue`, `UserDbRecord`, `UserDbMetadata`, parse/pack errors, `formula_d`, `formula_p`, and key formatting/parsing. Keep it storage-neutral and free of C pointers.

---

### `crates/yune-rime-api/src/userdb/store.rs` (storage trait, CRUD + batch)

**Analog:** `crates/yune-core/src/lib.rs` trait pattern and `crates/yune-rime-api/src/session.rs` state/provider pattern

**Trait boundary pattern** (`crates/yune-core/src/lib.rs` lines 39-66):
```rust
pub trait Translator: Send + Sync {
    fn name(&self) -> &'static str;

    fn translate(&self, input: &str) -> Vec<Candidate>;

    fn translate_with_status(&self, input: &str, _status: &Status) -> Vec<Candidate> {
        self.translate(input)
    }

    fn translate_with_state(
        &self,
        input: &str,
        status: &Status,
        _options: &HashMap<String, bool>,
    ) -> Vec<Candidate> {
        self.translate_with_status(input, status)
    }
```

**Process-wide provider pattern** (`crates/yune-rime-api/src/session.rs` lines 150-158):
```rust
pub(crate) fn sessions() -> &'static Mutex<SessionRegistry> {
    static SESSIONS: OnceLock<Mutex<SessionRegistry>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(SessionRegistry::default()))
}

pub(crate) fn service_started() -> &'static AtomicBool {
    static SERVICE_STARTED: AtomicBool = AtomicBool::new(false);
    &SERVICE_STARTED
}
```

**Planner instruction:** define a `UserDbStore` trait with typed operations (`fetch`, `update`, `erase`, `query_prefix`, `query_all`, `begin_transaction`, `abort_transaction`, `commit_transaction`) and either pass concrete stores explicitly or expose a narrow internal provider. Do not expose filesystem paths or C ABI types to `yune-core`.

---

### `crates/yune-rime-api/src/userdb/file_store.rs` (storage implementation, file-I/O + CRUD + batch)

**Analog:** `crates/yune-rime-api/src/deployment.rs`

**Filesystem root extraction + early failure pattern** (lines 920-937):
```rust
fn runtime_data_roots() -> (PathBuf, PathBuf, PathBuf) {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    (
        PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned()),
        PathBuf::from(paths.staging_dir.to_string_lossy().into_owned()),
        PathBuf::from(paths.prebuilt_data_dir.to_string_lossy().into_owned()),
    )
}

fn copy_if_present(source: &Path, destination: &Path) -> Option<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    fs::copy(source, destination).ok()?;
    Some(())
}
```

**Write helper pattern** (lines 977-980, 992-995, 1008-1011):
```rust
if let Some(parent) = path.parent() {
    fs::create_dir_all(parent).ok()?;
}
fs::write(path, bytes).ok()
```

**Planner instruction:** use this `Option`/early-return style for low-level helpers, but add atomic temp-write/rename behavior instead of direct `fs::write` for durable userdb state. All writes must stage and validate before replacing the durable store.

---

### `crates/yune-rime-api/src/userdb/snapshot.rs` (parser/writer, file-I/O + transform)

**Analogs:** `crates/yune-rime-api/src/userdb.rs`, `crates/yune-core/src/dictionary/compiled.rs`

**Import/export count pattern to replace with table-format parser** (`userdb.rs` lines 377-388):
```rust
fn count_text_user_dict_entries(path: &Path) -> Result<c_int, std::io::Error> {
    let contents = fs::read_to_string(path)?;
    Ok(contents
        .lines()
        .filter(|line| {
            let line = line.trim();
            !line.is_empty() && !line.starts_with('#')
        })
        .count()
        .try_into()
        .unwrap_or(c_int::MAX))
}
```

**Parser result pattern** (`compiled.rs` lines 97-115):
```rust
pub fn parse_rime_table_bin_metadata(
    bytes: impl AsRef<[u8]>,
) -> Result<RimeTableBinMetadata, RimeCompiledMetadataError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 68)?;
    let version = parse_rime_format_version(bytes, b"Rime::Table/")?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimeCompiledMetadataError::UnsupportedVersion);
    }
```

**Planner instruction:** separate uniform `*.userdb.txt` snapshot parser/writer from table import/export parser/writer. Snapshot restore should validate metadata before mutation; table import/export should return entry counts as `c_int` at ABI boundary.

---

### `crates/yune-rime-api/src/userdb/sync.rs` (service, batch + conflict transform)

**Analog:** `crates/yune-rime-api/src/userdb.rs`

**Peer discovery pattern** (lines 314-333):
```rust
fn peer_user_dict_snapshots(dict_name: &str) -> Vec<PathBuf> {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    let sync_dir = PathBuf::from(paths.sync_dir.to_string_lossy().into_owned());
    drop(paths);

    let Ok(entries) = fs::read_dir(sync_dir) else {
        return Vec::new();
    };
    let snapshot_name = format!("{dict_name}.userdb.txt");
    let mut snapshots = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| path.join(&snapshot_name))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    snapshots.sort();
    snapshots
}
```

**Batch success accumulation pattern** (lines 287-294, 301-312):
```rust
pub(crate) fn sync_all_user_dicts() -> bool {
    let mut success = true;
    for dict_name in deployed_user_dict_names() {
        if !sync_plain_user_dict(&dict_name) {
            success = false;
        }
    }
    success
}

fn sync_plain_user_dict(dict_name: &str) -> bool {
    if validate_user_dict_name(dict_name).is_none() {
        return false;
    }
    let mut success = true;
```

**Planner instruction:** retain peer-directory scan and sorted deterministic order, but replace line dedupe with librime-shaped merge: max absolute commits, max decayed `dee`, max tick, deleted-entry handling, metadata update, and backup-after-merge.

---

### `crates/yune-rime-api/src/userdb/recovery.rs` (service, recovery + file-I/O)

**Analog:** `crates/yune-rime-api/src/deployment.rs`

**Cleanup predicate pattern** (lines 514-523):
```rust
fn should_cleanup_trash_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
        return false;
    };
    file_name == "rime.log"
        || file_name.ends_with(".bin")
        || file_name.ends_with(".reverse.kct")
        || file_name.ends_with(".userdb.kct.old")
        || file_name.ends_with(".userdb.kct.snapshot")
}
```

**Task integration pattern** (lines 135-164):
```rust
#[no_mangle]
pub extern "C" fn RimeRunTask(task_name: *const c_char) -> Bool {
    let Some(task_name) = optional_c_string(task_name) else {
        return FALSE;
    };
    if task_name == "user_dict_sync" {
        return bool_from(sync_all_user_dicts());
    }
    if task_name == "backup_config_files" {
        return bool_from(backup_config_files());
    }
```

**Planner instruction:** add recovery/upgrade cleanup as explicit manager methods called by existing `user_dict_upgrade()` and deployment tasks. Recovery must fail closed: validate current store/snapshot before replacing anything.

---

### `crates/yune-rime-api/src/runtime.rs` (runtime config, request-response + file-I/O)

**Analog:** `crates/yune-rime-api/src/runtime.rs`

**RuntimePaths fields to preserve** (lines 16-30):
```rust
pub(crate) struct RuntimePaths {
    pub(crate) shared_data_dir: CString,
    pub(crate) user_data_dir: CString,
    pub(crate) prebuilt_data_dir: CString,
    pub(crate) staging_dir: CString,
    pub(crate) sync_dir: CString,
    pub(crate) user_id: CString,
    pub(crate) user_data_sync_dir: CString,
    pub(crate) distribution_name: CString,
    pub(crate) distribution_code_name: CString,
    pub(crate) distribution_version: CString,
    pub(crate) app_name: CString,
    pub(crate) log_dir: CString,
    pub(crate) backup_config_files: bool,
}
```

**Installation metadata read pattern** (lines 162-185):
```rust
fn read_installation_settings(user_data_dir: &str) -> InstallationSettings {
    let path = Path::new(user_data_dir).join("installation.yaml");
    let Ok(text) = fs::read_to_string(path) else {
        return InstallationSettings::default();
    };
    let Ok(Value::Mapping(root)) = serde_yaml::from_str::<Value>(&text) else {
        return InstallationSettings::default();
    };

    InstallationSettings {
        loaded: true,
```

**Planner instruction:** userdb storage must continue to source `user_id`, `sync_dir`, and `user_data_sync_dir` from `RuntimePaths`; do not compute these from dict names or snapshot paths.

---

### `crates/yune-rime-api/src/deployment.rs` (task service, batch + file-I/O)

**Analog:** `crates/yune-rime-api/src/deployment.rs`

**Sync orchestration pattern** (lines 123-133):
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

**Maintenance task chain pattern** (lines 590-592):
```rust
pub(crate) fn run_workspace_maintenance_tasks() -> bool {
    workspace_update() && user_dict_upgrade() && cleanup_trash()
}
```

**Planner instruction:** keep sync/deploy notification behavior and task names stable. New userdb sync/upgrade internals should remain behind `sync_all_user_dicts()` and `user_dict_upgrade()` so deployment tests do not need new call paths.

---

### `crates/yune-rime-api/src/resource_id.rs` (validation utility, transform)

**Analog:** `crates/yune-rime-api/src/resource_id.rs`

**Userdb logical ID validation pattern** (lines 26-31):
```rust
pub(crate) fn validate_user_dict_name(id: &str) -> Option<String> {
    if id.ends_with(".userdb") || id.ends_with(".userdb.txt") {
        return None;
    }
    validate_logical_id(id)
}
```

**Shared logical ID rejection pattern** (lines 33-47):
```rust
fn validate_logical_id(id: &str) -> Option<String> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.starts_with('~')
        || id.contains('\0')
        || id.contains('/')
        || id.contains('\\')
        || has_windows_drive_prefix(id)
    {
        return None;
    }

    Some(id.to_owned())
}
```

**Planner instruction:** apply this before every userdb destination join and before accepting metadata-derived db names. Snapshot file paths may be file paths, but embedded `/db_name` must be validated as a logical user dict name.

---

### `crates/yune-rime-api/src/session.rs` (session store/provider, event-driven)

**Analog:** `crates/yune-rime-api/src/session.rs`

**Session state ownership pattern** (lines 74-103):
```rust
pub(crate) struct SessionState {
    pub(crate) engine: Engine,
    pub(crate) unread_commit: Option<String>,
    pub(crate) input_buffer: Option<CString>,
    pub(crate) key_binder: Option<KeyBinderProcessor>,
    pub(crate) speller: Option<SpellerProcessor>,
    pub(crate) editor_processor: Option<EditorProcessor>,
    pub(crate) editor_bindings: HashMap<KeyEvent, EditorBindingAction>,
```

**Session mutation helper pattern** (lines 206-222):
```rust
pub(crate) fn with_session(
    session_id: RimeSessionId,
    action: impl FnOnce(&mut SessionState) -> bool,
) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };

    bool_from(action(session))
}
```

**Planner instruction:** if a userdb learning adapter needs session-local schema/userdb context, store narrow typed fields in `SessionState`; do not store C strings, raw pointers, or concrete store handles in `yune-core`.

---

### `crates/yune-rime-api/src/lib.rs` (ABI orchestration glue, event-driven + request-response)

**Analog:** `crates/yune-rime-api/src/lib.rs`

**Module declaration/export pattern** (lines 19-39, 56-67):
```rust
mod abi;
mod api_table;
mod candidate_api;
mod config;
mod config_api;
mod config_compiler;
mod context_api;
mod deployment;
mod ffi_memory;
mod key_table;
mod levers;
mod modules;
mod notifications;
mod processors;
mod resource_id;
mod runtime;
mod schema_api;
mod schema_install;
mod schema_selection;
mod session;
mod userdb;
```

**Commit append seam pattern** (lines 1131-1137):
```rust
fn append_unread_commit(session: &mut SessionState, commit: String) {
    let commit = shape_formatted_commit_text(session, &commit);
    match &mut session.unread_commit {
        Some(buffer) => buffer.push_str(&commit),
        None => session.unread_commit = Some(commit),
    }
}
```

**Core event processing seam pattern** (lines 1471-1478):
```rust
let before_input = session.engine.context().composition.input.clone();
let before_highlighted = session.engine.context().highlighted;
let commit = session.engine.process_key_event(key_event);
update_key_binding_paging_state(session, key_event, &before_input, before_highlighted);
update_session_segment_tags(session);
if let Some(commit) = commit {
    return SessionKeyProcessResult::Commit(commit);
}
```

**Planner instruction:** if `lib.rs` changes, keep it to module declaration/re-export and narrow commit-learning adapter calls near existing commit seams. Do not add owned storage logic to this facade.

---

### `crates/yune-core/src/userdb.rs` (core model/utility/translator support, transform + lookup)

**Analogs:** `crates/yune-core/src/dictionary/compiled.rs`, `crates/yune-core/src/translator/mod.rs`

**Pure typed computation pattern** (`compiled.rs` lines 0-39):
```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RimeChecksumComputer {
    remainder: u32,
}

impl RimeChecksumComputer {
    const POLYNOMIAL: u32 = 0xedb8_8320;

    #[must_use]
    pub const fn new(initial_remainder: u32) -> Self {
        Self {
            remainder: initial_remainder,
        }
    }

    pub fn process_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        for byte in bytes.as_ref() {
            self.remainder ^= u32::from(*byte);
```

**Candidate construction from lookup pattern** (`translator/mod.rs` lines 252-266):
```rust
fn candidate_for_lookup(
    &self,
    entry_code: &str,
    candidate: &Candidate,
    lookup_code: &str,
) -> Candidate {
    let mut candidate = candidate.clone();
    candidate.comment = self.comment_format.apply(&candidate.comment);
    candidate.quality = candidate.quality.exp() + self.initial_quality;
    if entry_code != lookup_code {
        candidate.source = CandidateSource::Completion;
        candidate.quality -= 1.0;
    }
    candidate
}
```

**Planner instruction:** put storage-neutral learned-entry structs, ranking formulas, predictive comment generation, and userdb candidate conversion here if shared by translator/engine tests. No `PathBuf`, `CString`, runtime paths, or FFI types.

---

### `crates/yune-core/src/engine.rs` (engine service, event-driven)

**Analog:** `crates/yune-core/src/engine.rs`

**Translator/filter/ranker field pattern** (lines 10-18):
```rust
pub struct Engine {
    context: Context,
    status: Status,
    options: HashMap<String, bool>,
    properties: HashMap<String, String>,
    translators: Vec<Box<dyn Translator>>,
    filters: Vec<Box<dyn CandidateFilter>>,
    rankers: Vec<Box<dyn CandidateRanker>>,
}
```

**Commit metadata seam to extend** (lines 691-708):
```rust
fn commit_candidate(&mut self, candidate_index: usize) -> Option<String> {
    let (text, candidate_type) = self
        .context
        .candidates
        .get(candidate_index)
        .map(|candidate| (candidate.text.clone(), candidate.source.as_str().to_owned()))?;
    self.record_commit_with_type(candidate_type, text.clone());
    self.clear_composition();
    Some(text)
}

fn record_commit_with_type(&mut self, candidate_type: impl Into<String>, text: String) {
    self.context.last_commit = Some(text.clone());
    self.context.commit_history.push(CommitRecord {
        candidate_type: candidate_type.into(),
        text,
    });
}
```

**Candidate refresh order to preserve** (lines 710-735):
```rust
fn refresh_candidates(&mut self) {
    let input = self.context.composition.input.as_str();
    let mut candidates = self
        .translators
        .iter()
        .flat_map(|translator| {
            translator.translate_with_context(input, &self.status, &self.options, &self.context)
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .quality
            .partial_cmp(&left.quality)
            .unwrap_or(Ordering::Equal)
    });
    for filter in &self.filters {
        filter.apply_with_context(&mut candidates, &self.options, &self.context);
    }
    for ranker in &self.rankers {
```

**Planner instruction:** capture richer committed-candidate metadata before `clear_composition()`, and inject userdb candidates before optional rankers. Do not route classic userdb learning through `CandidateRanker`.

---

### `crates/yune-core/src/state.rs` (model, event-driven state)

**Analog:** `crates/yune-core/src/state.rs`

**Candidate and source pattern** (lines 0-21):
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub text: String,
    pub comment: String,
    pub source: CandidateSource,
    pub quality: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CandidateSource {
    Echo,
    Punctuation,
    Table,
    Completion,
    Sentence,
    ReverseLookup,
    History,
    Switch,
    Unfold,
    Schema,
    Ai,
}
```

**String label pattern** (lines 23-40):
```rust
impl CandidateSource {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Echo => "echo",
            Self::Punctuation => "punct",
            Self::Table => "table",
            Self::Completion => "completion",
```

**Commit record pattern** (lines 42-46):
```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommitRecord {
    pub candidate_type: String,
    pub text: String,
}
```

**Planner instruction:** if adding `CandidateSource::UserTable` or richer commit metadata, copy this enum/string-label style and keep names stable for tests.

---

### `crates/yune-core/src/translator/mod.rs` (translator service, transform + request-response)

**Analog:** `crates/yune-core/src/translator/mod.rs`

**Translator state/builder pattern** (lines 33-46, 116-126):
```rust
pub struct StaticTableTranslator {
    entries: Vec<(String, Candidate)>,
    enable_completion: bool,
    enable_charset_filter: bool,
    enable_sentence: bool,
    sentence_over_completion: bool,
    tags: Vec<String>,
    delimiters: String,
    initial_quality: f32,
    comment_format: CommentFormat,
    dictionary_exclude: HashSet<String>,
    corrections: Vec<RimeCorrectionEntry>,
    tolerance_rules: Vec<RimeToleranceRule>,
}

#[must_use]
pub fn with_completion(mut self, enable_completion: bool) -> Self {
    self.enable_completion = enable_completion;
    self
}
```

**Lookup/filter/candidate construction pattern** (lines 285-321):
```rust
let lookup_code = self.lookup_code(input);
let expanded_lookup_codes = self.expanded_lookup_codes(lookup_code);
let mut candidates = self
    .entries
    .iter()
    .filter_map(|(entry_code, candidate)| {
        let matched_lookup_code =
            expanded_lookup_codes.iter().find(|candidate_lookup_code| {
                self.matches_lookup_code(entry_code, candidate_lookup_code)
            })?;
        (self.is_dictionary_word_allowed(candidate)
            && (!filter_by_charset || !contains_extended_cjk(&candidate.text)))
        .then(|| self.candidate_for_lookup(entry_code, candidate, matched_lookup_code))
    })
    .collect::<Vec<_>>();
```

**Context-aware translation pattern** (lines 457-471):
```rust
fn translate_with_context(
    &self,
    input: &str,
    _status: &Status,
    options: &HashMap<String, bool>,
    context: &Context,
) -> Vec<Candidate> {
    let filter_by_charset = self.enable_charset_filter
        && !options.get("extended_charset").copied().unwrap_or(false);
    self.translated_candidates_for_segment(
        input,
        filter_by_charset,
        Some(&context.segment_tags),
    )
}
```

**Planner instruction:** implement userdb candidate generation as a classic translator or translator-adjacent source that follows this context-aware pattern and respects segment tags/options.

---

### `crates/yune-core/src/lib.rs` (core facade/exports/tests)

**Analog:** `crates/yune-core/src/lib.rs`

**Export pattern** (lines 30-37):
```rust
};
pub use key::{parse_key_sequence, KeyCode, KeyEvent, KeyModifiers, KeySequenceParseError};
pub use punctuation::PunctuationTranslator;
pub use state::{Candidate, CandidateSource, CommitRecord, Composition, Context, Snapshot, Status};
pub use translator::{
    EchoTranslator, FoldedSwitchOptions, HistoryTranslator, ReverseLookupTranslator,
    SchemaListTranslator, StaticTableTranslator, SwitchTranslator, SwitchTranslatorSwitch,
};
```

**Focused core behavior test pattern** (lines 3985-4013):
```rust
#[test]
fn history_translator_returns_recent_commits_for_configured_input() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你"), ("hao", "好")]));
    engine.add_translator(HistoryTranslator::new("his").with_size(2));

    engine.set_input("ni");
    assert_eq!(engine.commit_highlighted(), Some("你".to_owned()));
    engine.set_input("hao");
    assert_eq!(engine.commit_highlighted(), Some("好".to_owned()));
```

**Planner instruction:** add `mod userdb;` and `pub use` only if needed. Prefer new userdb unit tests near `crates/yune-core/src/userdb.rs`; do not add more large tests to `lib.rs` except minimal facade export coverage.

---

### `crates/yune-rime-api/src/tests/userdb.rs` (test, file-I/O + ABI request-response)

**Analog:** `crates/yune-rime-api/src/tests/userdb.rs`

**ABI test setup pattern** (lines 2-18):
```rust
#[test]
fn sync_user_data_merges_plain_userdb_snapshots_and_backs_up_current_state() {
    let _guard = test_guard();
    let root = unique_temp_dir("rime-sync-user-data");
    let user = root.join("user");
    let peer_sync = user.join("sync").join("peer");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::create_dir_all(&peer_sync).expect("peer sync dir should be created");
    struct CurrentDirGuard(PathBuf);
    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }
```

**Runtime setup + sync assertions pattern** (lines 36-61):
```rust
let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
let mut traits = empty_traits();
traits.user_data_dir = user_c.as_ptr();
// SAFETY: traits points to valid storage and strings live for the call.
unsafe { RimeSetup(&traits) };

let session_id = RimeCreateSession();
assert_eq!(RimeFindSession(session_id), TRUE);
assert_eq!(RimeSyncUserData(), TRUE);
assert_eq!(RimeFindSession(session_id), FALSE);

let merged =
    fs::read_to_string(user.join("luna_pinyin.userdb")).expect("dict should be readable");
assert_eq!(merged, "ni hao\t你好\t1\nzhong guo\t中国\t2\n");
```

**Planner instruction:** extend this module for userdb lifecycle, transaction, recovery, sync, import/export, rollback, and learning-through-ABI tests. Keep temp dirs unique and clean them up.

---

### `crates/yune-rime-api/src/tests/resource_id.rs` (test, validation + file-I/O)

**Analog:** `crates/yune-rime-api/src/tests/resource_id.rs`

**Logical userdb ID tests** (lines 103-139):
```rust
#[test]
fn user_dict_names_accept_logical_names_only() {
    assert_eq!(
        validate_user_dict_name("luna_pinyin"),
        Some("luna_pinyin".to_owned())
    );
    assert_eq!(
        validate_user_dict_name("default"),
        Some("default".to_owned())
    );
    assert_eq!(
        validate_user_dict_name("sample.user"),
        Some("sample.user".to_owned())
    );
}

#[test]
fn user_dict_names_reject_paths_and_userdb_suffixes() {
    for id in [
        "",
        ".",
```

**ABI path-vs-dict-name boundary test** (lines 240-293):
```rust
#[test]
fn userdb_apis_reject_unsafe_logical_dict_names_but_keep_file_paths() {
    let _guard = test_guard();
    let temp = unique_temp_dir("resource-id-userdb");
    let user = temp.join("user");
    let sync = temp.join("sync");
    fs::create_dir_all(&user).expect("create user dir");
    fs::create_dir_all(&sync).expect("create sync dir");
    fs::write(temp.join("input.txt"), "ni\t你\n").expect("write import source");
```

**Planner instruction:** add tests for snapshot metadata `db_name` validation, malformed snapshot rejection, `.userdb` suffix rejection, and no traversal-created files after failed restore/import/export.

---

### `crates/yune-rime-api/src/tests/deployment.rs` (test, batch + file-I/O)

**Analog:** `crates/yune-rime-api/src/tests/deployment.rs`

**Legacy artifact cleanup test pattern** (lines 2240-2300):
```rust
fn cleanup_trash_moves_librime_deployer_artifacts() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("cleanup-trash");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
```

**Deploy notification sync test pattern** (lines 2416-2457):
```rust
fn sync_user_data_emits_librime_deploy_notifications() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("sync-notification-events");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(user.join("default.yaml"), "config_version: test\n")
        .expect("user config should be written");
```

**Planner instruction:** keep deployment-level tests for task routing, notifications, cleanup artifacts, and `RimeSyncUserData` orchestration. Do not move userdb lifecycle assertions here unless testing deployment integration specifically.

---

### `crates/yune-rime-api/tests/frontend_client.rs` (integration test, ABI function-table request-response)

**Analog:** `crates/yune-rime-api/tests/frontend_client.rs`

**Function-table levers test pattern** (lines 740-806):
```rust
#[test]
fn frontend_style_api_table_can_manage_levers_user_dicts() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let find_module = api.find_module.expect("frontend requires find_module");
```

**Iterator and backup assertions pattern** (lines 807-831):
```rust
let mut iterator = empty_user_dict_iterator();
assert_eq!(unsafe { iterator_init(&mut iterator) }, TRUE);
assert!(!iterator.ptr.is_null());
assert_eq!(iterator.i, 0);
let first = unsafe { next_user_dict(&mut iterator) };
assert!(!first.is_null());
assert_eq!(unsafe { CStr::from_ptr(first) }.to_str(), Ok("essay"));
```

**Normal session commit flow pattern** (lines 1875-1911):
```rust
let session_id = create_session();
assert_ne!(session_id, 0);
assert_eq!(find_session(session_id), TRUE);
assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

let input = get_input(session_id);
assert!(!input.is_null());
let input = unsafe { CStr::from_ptr(input) };
assert_eq!(input.to_str(), Ok("ni"));
```

**Planner instruction:** use this file only when Phase 05 needs frontend-style function-table proof: levers API behavior, normal session commit learning, and ABI-visible candidate order/commit retrieval. Keep most lifecycle unit tests in `src/tests/userdb.rs`.

## Shared Patterns

### ABI boundaries: null checks, safety docs, C-string conversion, Bool/null returns

**Source:** `crates/yune-rime-api/src/userdb.rs` lines 97-108 and `crates/yune-rime-api/src/runtime.rs` lines 192-205

**Apply to:** `userdb.rs`, any new exported userdb functions, `lib.rs` commit adapter call sites

```rust
#[no_mangle]
pub unsafe extern "C" fn RimeLeversBackupUserDict(dict_name: *const c_char) -> Bool {
    let Some(dict_name) = optional_c_string(dict_name) else {
        return FALSE;
    };
    bool_from(backup_plain_user_dict(&dict_name))
}
```

### Logical resource ID validation before filesystem joins

**Source:** `crates/yune-rime-api/src/resource_id.rs` lines 26-47 and `crates/yune-rime-api/src/userdb.rs` lines 259-267

**Apply to:** all userdb destination paths, snapshot metadata db names, import/export dict names, recovery/upgrade cleanup

```rust
pub(crate) fn validate_user_dict_name(id: &str) -> Option<String> {
    if id.ends_with(".userdb") || id.ends_with(".userdb.txt") {
        return None;
    }
    validate_logical_id(id)
}
```

### Runtime path ownership through `RuntimePaths`

**Source:** `crates/yune-rime-api/src/runtime.rs` lines 16-30, 70-88, 187-190

**Apply to:** `userdb/mod.rs`, `file_store.rs`, `snapshot.rs`, `sync.rs`, `recovery.rs`, `deployment.rs`

```rust
pub(crate) fn runtime_paths() -> &'static Mutex<RuntimePaths> {
    static RUNTIME_PATHS: OnceLock<Mutex<RuntimePaths>> = OnceLock::new();
    RUNTIME_PATHS.get_or_init(|| Mutex::new(RuntimePaths::default()))
}
```

### Batch task success accumulation

**Source:** `crates/yune-rime-api/src/userdb.rs` lines 287-294 and `crates/yune-rime-api/src/deployment.rs` lines 123-133

**Apply to:** sync, backup-all, upgrade/recovery scans

```rust
let mut success = true;
for dict_name in deployed_user_dict_names() {
    if !sync_plain_user_dict(&dict_name) {
        success = false;
    }
}
success
```

### Core candidate ordering and userdb translator placement

**Source:** `crates/yune-core/src/engine.rs` lines 710-735

**Apply to:** `crates/yune-core/src/userdb.rs`, `engine.rs`, `translator/mod.rs`

```rust
let mut candidates = self
    .translators
    .iter()
    .flat_map(|translator| {
        translator.translate_with_context(input, &self.status, &self.options, &self.context)
    })
    .collect::<Vec<_>>();
candidates.sort_by(|left, right| {
    right
        .quality
        .partial_cmp(&left.quality)
        .unwrap_or(Ordering::Equal)
});
```

### Focused ABI test ownership

**Source:** `crates/yune-rime-api/src/tests/mod.rs` lines 96-106, 332-341

**Apply to:** all `crates/yune-rime-api/src/tests/*.rs` additions

```rust
fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned");
    let traits = empty_traits();
    // SAFETY: empty traits points to valid storage for the duration of the call.
    unsafe { RimeInitialize(&traits) };
    guard
}
```

## No Analog Found

No Phase 05 file is completely without an in-repository analog. The closest gaps are semantic, not structural:

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/yune-rime-api/src/userdb/store.rs` | storage trait | CRUD + batch | No existing userdb transaction store exists; copy trait/provider style, but use librime research for operations. |
| `crates/yune-rime-api/src/userdb/recovery.rs` | recovery service | file-I/O + recovery | No fail-closed userdb recovery exists; copy deployment cleanup/task style, but use research/librime targets for semantics. |
| `crates/yune-core/src/userdb.rs` | learned candidate model | transform + lookup | No persistent userdb learning model exists; copy typed pure-core and translator patterns, but use research/librime formulas. |

## Metadata

**Analog search scope:** `crates/yune-rime-api/src/**/*.rs`, `crates/yune-rime-api/tests/**/*.rs`, `crates/yune-core/src/**/*.rs`, project planning docs, `AGENTS.md`.

**Files scanned:** 80 Rust/Cargo source files listed under `crates/`; 14 analog files read or targeted for excerpts.

**Primary analog files read:**
- `crates/yune-rime-api/src/userdb.rs`
- `crates/yune-rime-api/src/runtime.rs`
- `crates/yune-rime-api/src/deployment.rs`
- `crates/yune-rime-api/src/resource_id.rs`
- `crates/yune-rime-api/src/session.rs`
- `crates/yune-rime-api/src/lib.rs`
- `crates/yune-core/src/engine.rs`
- `crates/yune-core/src/state.rs`
- `crates/yune-core/src/translator/mod.rs`
- `crates/yune-core/src/dictionary/compiled.rs`
- `crates/yune-core/src/lib.rs`
- `crates/yune-rime-api/src/tests/userdb.rs`
- `crates/yune-rime-api/src/tests/resource_id.rs`
- `crates/yune-rime-api/src/tests/deployment.rs`
- `crates/yune-rime-api/tests/frontend_client.rs`

**Pattern extraction date:** 2026-04-30
