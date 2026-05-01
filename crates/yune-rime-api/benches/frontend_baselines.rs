use std::{
    ffi::CString,
    fs, mem,
    os::raw::c_int,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{
    rime_get_api, Bool, RimeCommit, RimeComposition, RimeContext, RimeMenu, RimeSessionId,
    RimeStatus, RimeTraits, TRUE,
};

const PHASE_LABEL: &str = "06-04";
const SESSION_ITERATIONS: u64 = 200;
const SIMPLE_KEY_ITERATIONS: u64 = 600;
const SCHEMA_LOOKUP_ITERATIONS: u64 = 200;
const DEPLOY_ITERATIONS: u64 = 20;
const USERDB_ITERATIONS: u64 = 80;
const USERDB_SEEDED_RECORDS: usize = 3;

fn main() {
    let _guard = bench_guard();
    let api = api_table();
    validate_api_table(api);

    let results = vec![
        run_session_lifecycle(api),
        run_simple_ascii_key(api),
        run_schema_loaded_key(api),
        run_deploy_dictionary_load(api),
        run_userdb_learning_sync(api),
    ];

    print_results(&results);
}

fn bench_guard() -> MutexGuard<'static, ()> {
    static BENCH_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    BENCH_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("frontend benchmark lock should not be poisoned")
}

fn api_table() -> &'static yune_rime_api::RimeApi {
    let api = rime_get_api();
    assert!(!api.is_null(), "rime_get_api returned null");
    // SAFETY: the pointer was checked for null and remains valid for this process.
    unsafe { &*api }
}

fn validate_api_table(api: &yune_rime_api::RimeApi) {
    assert_eq!(
        api.data_size,
        (mem::size_of_val(api) - mem::size_of::<c_int>()) as c_int,
        "unexpected RimeApi data_size"
    );
}

#[derive(Clone, Debug)]
struct BenchResult {
    name: &'static str,
    operations: u64,
    fixture: &'static str,
    data_size: String,
    total: Duration,
}

impl BenchResult {
    fn per_operation_micros(&self) -> f64 {
        self.total.as_secs_f64() * 1_000_000.0 / self.operations as f64
    }
}

struct RuntimeFixture {
    root: PathBuf,
    shared: PathBuf,
    user: PathBuf,
    shared_c: CString,
    user_c: CString,
}

impl RuntimeFixture {
    fn new(label: &str) -> Self {
        let root = unique_temp_dir(label);
        let shared = root.join("shared");
        let user = root.join("user");
        fs::create_dir_all(&shared).expect("shared fixture dir should be created");
        fs::create_dir_all(&user).expect("user fixture dir should be created");
        let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
        let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
        Self {
            root,
            shared,
            user,
            shared_c,
            user_c,
        }
    }

    fn traits(&self) -> RimeTraits {
        let mut traits = empty_traits();
        traits.shared_data_dir = self.shared_c.as_ptr();
        traits.user_data_dir = self.user_c.as_ptr();
        traits
    }
}

impl Drop for RuntimeFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "yune-rime-api-frontend-baseline-{label}-{}-{nanos}",
        std::process::id()
    ))
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

fn empty_status() -> RimeStatus {
    RimeStatus {
        data_size: (mem::size_of::<RimeStatus>() - mem::size_of::<i32>()) as i32,
        schema_id: ptr::null_mut(),
        schema_name: ptr::null_mut(),
        is_disabled: 0,
        is_composing: 0,
        is_ascii_mode: 0,
        is_full_shape: 0,
        is_simplified: 0,
        is_traditional: 0,
        is_ascii_punct: 0,
    }
}

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
            is_last_page: 0,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        },
        commit_text_preview: ptr::null_mut(),
        select_labels: ptr::null_mut(),
    }
}

fn empty_commit() -> RimeCommit {
    RimeCommit {
        data_size: (mem::size_of::<RimeCommit>() - mem::size_of::<i32>()) as i32,
        text: ptr::null_mut(),
    }
}

fn reset_runtime(api: &yune_rime_api::RimeApi) {
    if let Some(cleanup_all_sessions) = api.cleanup_all_sessions {
        cleanup_all_sessions();
    }
    if let Some(finalize) = api.finalize {
        finalize();
    }
    let reset = empty_traits();
    let setup = require("setup", api.setup);
    let initialize = require("initialize", api.initialize);
    // SAFETY: reset traits contain only null pointers and a valid data_size.
    unsafe { setup(&reset) };
    // SAFETY: reset traits contain only null pointers and a valid data_size.
    unsafe { initialize(&reset) };
}

fn setup_fixture(api: &yune_rime_api::RimeApi, fixture: &RuntimeFixture) {
    let setup = require("setup", api.setup);
    let initialize = require("initialize", api.initialize);
    reset_runtime(api);
    let traits = fixture.traits();
    // SAFETY: fixture-owned C strings outlive setup and initialize calls.
    unsafe { setup(&traits) };
    // SAFETY: fixture-owned C strings outlive setup and initialize calls.
    unsafe { initialize(&traits) };
}

fn write_basic_schema(shared: &Path, schema: &str) {
    fs::write(
        shared.join("default.yaml"),
        format!("config_version: baseline\nschema_list:\n  - schema: {schema}\n"),
    )
    .expect("default config should be written");
    fs::write(
        shared.join(format!("{schema}.schema.yaml")),
        format!("schema:\n  schema_id: {schema}\n  name: Baseline {schema}\n"),
    )
    .expect("schema config should be written");
}

fn write_lookup_schema(shared: &Path, user: &Path) {
    let staging = user.join("build");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: baseline\nschema_list:\n  - schema: lookup\n",
    )
    .expect("default config should be written");
    let lookup_schema = "schema:\n  schema_id: lookup\n  name: Lookup\nmenu:\n  page_size: 5\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: lookup\n";
    fs::write(staging.join("lookup.schema.yaml"), lookup_schema)
        .expect("staging lookup schema should be written");
    fs::write(shared.join("lookup.schema.yaml"), lookup_schema)
        .expect("shared lookup schema should be written");
    fs::write(
        shared.join("lookup.dict.yaml"),
        "---\nname: lookup\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nba\t八\t10\nba\t吧\t9\nba\t爸\t8\nba\t巴\t7\n",
    )
    .expect("lookup dictionary should be written");
}

fn write_userdb_schema(shared: &Path, user: &Path) {
    let staging = user.join("build");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("learn.schema.yaml"),
        "schema:\n  schema_id: learn\n  name: Learn\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: learn\n",
    )
    .expect("learning schema should be written");
    fs::write(
        shared.join("learn.dict.yaml"),
        "---\nname: learn\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nni\t你\t10\nni hao\t你好\t9\n",
    )
    .expect("learning dictionary should be written");
    fs::write(
        user.join("learn.userdb"),
        "# yune userdb\n/db_name\tlearn\n/db_type\tuserdb\n/tick\t3\nni hao \t你好\tc=2 d=2 t=1\nni men \t你们\tc=1 d=1 t=2\n",
    )
    .expect("synthetic userdb should be written");
    let sync = user.join("sync");
    fs::create_dir_all(&sync).expect("sync dir should be created");
    fs::write(
        user.join("installation.yaml"),
        format!(
            "installation_id: baseline-device\nsync_dir: '{}'\n",
            sync.to_string_lossy()
        ),
    )
    .expect("installation metadata should be written");
}

fn run_session_lifecycle(api: &yune_rime_api::RimeApi) -> BenchResult {
    let fixture = RuntimeFixture::new("session-lifecycle");
    write_basic_schema(&fixture.shared, "baseline");
    setup_fixture(api, &fixture);
    let cleanup_all_sessions = require("cleanup_all_sessions", api.cleanup_all_sessions);
    let create_session = require("create_session", api.create_session);
    let destroy_session = require("destroy_session", api.destroy_session);

    let start = Instant::now();
    for _ in 0..SESSION_ITERATIONS {
        let session_id = create_session();
        assert_ne!(session_id, 0, "create_session returned 0");
        assert_eq!(destroy_session(session_id), TRUE, "destroy_session failed");
    }
    let total = start.elapsed();
    cleanup_all_sessions();
    reset_runtime(api);

    BenchResult {
        name: "session_create_destroy",
        operations: SESSION_ITERATIONS,
        fixture: "synthetic-basic-schema",
        data_size: "sessions=200".to_owned(),
        total,
    }
}

fn run_simple_ascii_key(api: &yune_rime_api::RimeApi) -> BenchResult {
    let fixture = RuntimeFixture::new("simple-ascii-key");
    write_basic_schema(&fixture.shared, "baseline");
    setup_fixture(api, &fixture);
    let cleanup_all_sessions = require("cleanup_all_sessions", api.cleanup_all_sessions);
    let create_session = require("create_session", api.create_session);
    let destroy_session = require("destroy_session", api.destroy_session);
    let process_key = require("process_key", api.process_key);
    let get_status = require("get_status", api.get_status);
    let free_status = require("free_status", api.free_status);
    let get_context = require("get_context", api.get_context);
    let free_context = require("free_context", api.free_context);
    let get_commit = require("get_commit", api.get_commit);
    let free_commit = require("free_commit", api.free_commit);

    let session_id = create_session();
    assert_ne!(session_id, 0, "create_session returned 0");
    let start = Instant::now();
    for index in 0..SIMPLE_KEY_ITERATIONS {
        let key = if index % 2 == 0 { 'a' } else { ' ' };
        assert_eq!(process_key(session_id, key as c_int, 0), TRUE);
        read_status_context_commit(
            api,
            session_id,
            get_status,
            free_status,
            get_context,
            free_context,
            get_commit,
            free_commit,
        );
    }
    let total = start.elapsed();
    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    reset_runtime(api);

    BenchResult {
        name: "per_key_simple_ascii_rime_process_key",
        operations: SIMPLE_KEY_ITERATIONS,
        fixture: "default-echo-schema",
        data_size: "keys=600 status/context/commit/free cycles=600".to_owned(),
        total,
    }
}

#[allow(clippy::too_many_arguments)]
fn read_status_context_commit(
    _api: &yune_rime_api::RimeApi,
    session_id: RimeSessionId,
    get_status: unsafe extern "C" fn(RimeSessionId, *mut RimeStatus) -> Bool,
    free_status: unsafe extern "C" fn(*mut RimeStatus) -> Bool,
    get_context: unsafe extern "C" fn(RimeSessionId, *mut RimeContext) -> Bool,
    free_context: unsafe extern "C" fn(*mut RimeContext) -> Bool,
    get_commit: unsafe extern "C" fn(RimeSessionId, *mut RimeCommit) -> Bool,
    free_commit: unsafe extern "C" fn(*mut RimeCommit) -> Bool,
) {
    let mut status = empty_status();
    // SAFETY: status points to valid caller-owned storage for the ABI call.
    if unsafe { get_status(session_id, &mut status) } == TRUE {
        // SAFETY: status was successfully populated by get_status and must be freed.
        assert_eq!(unsafe { free_status(&mut status) }, TRUE);
    }
    let mut context = empty_context();
    // SAFETY: context points to valid caller-owned storage for the ABI call.
    if unsafe { get_context(session_id, &mut context) } == TRUE {
        // SAFETY: context was successfully populated by get_context and must be freed.
        assert_eq!(unsafe { free_context(&mut context) }, TRUE);
    }
    let mut commit = empty_commit();
    // SAFETY: commit points to valid caller-owned storage for the ABI call.
    if unsafe { get_commit(session_id, &mut commit) } == TRUE {
        // SAFETY: commit was successfully populated by get_commit and must be freed.
        assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    }
}

fn run_schema_loaded_key(api: &yune_rime_api::RimeApi) -> BenchResult {
    let fixture = RuntimeFixture::new("schema-loaded-key");
    write_lookup_schema(&fixture.shared, &fixture.user);
    setup_fixture(api, &fixture);
    let cleanup_all_sessions = require("cleanup_all_sessions", api.cleanup_all_sessions);
    let create_session = require("create_session", api.create_session);
    let destroy_session = require("destroy_session", api.destroy_session);
    let select_schema = require("select_schema", api.select_schema);
    let process_key = require("process_key", api.process_key);
    let get_status = require("get_status", api.get_status);
    let free_status = require("free_status", api.free_status);
    let get_context = require("get_context", api.get_context);
    let free_context = require("free_context", api.free_context);
    let get_commit = require("get_commit", api.get_commit);
    let free_commit = require("free_commit", api.free_commit);
    let schema_id = CString::new("lookup").expect("schema id should be valid");

    let start = Instant::now();
    for _ in 0..SCHEMA_LOOKUP_ITERATIONS {
        let session_id = create_session();
        assert_ne!(session_id, 0, "create_session returned 0");
        // SAFETY: schema_id points to a valid NUL-terminated logical schema ID.
        assert_eq!(
            unsafe { select_schema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(process_key(session_id, 'b' as c_int, 0), TRUE);
        assert_eq!(process_key(session_id, 'a' as c_int, 0), TRUE);
        read_status_context_commit(
            api,
            session_id,
            get_status,
            free_status,
            get_context,
            free_context,
            get_commit,
            free_commit,
        );
        assert_eq!(destroy_session(session_id), TRUE);
    }
    let total = start.elapsed();
    cleanup_all_sessions();
    reset_runtime(api);

    BenchResult {
        name: "per_key_schema_loaded_lookup_rime_process_key",
        operations: SCHEMA_LOOKUP_ITERATIONS * 2,
        fixture: "lookup-schema-table-dictionary",
        data_size: "dictionary_entries=4 sessions=200 status/context/commit/free cycles=200"
            .to_owned(),
        total,
    }
}

fn run_deploy_dictionary_load(api: &yune_rime_api::RimeApi) -> BenchResult {
    let fixture = RuntimeFixture::new("deploy-dictionary-load");
    write_lookup_schema(&fixture.shared, &fixture.user);
    setup_fixture(api, &fixture);
    let deployer_initialize = require("deployer_initialize", api.deployer_initialize);
    let deploy = require("deploy", api.deploy);
    let deploy_schema = require("deploy_schema", api.deploy_schema);
    let create_session = require("create_session", api.create_session);
    let destroy_session = require("destroy_session", api.destroy_session);
    let select_schema = require("select_schema", api.select_schema);
    let cleanup_all_sessions = require("cleanup_all_sessions", api.cleanup_all_sessions);
    let schema_file = CString::new("lookup.schema.yaml").expect("schema file should be valid");
    let schema_id = CString::new("lookup").expect("schema id should be valid");
    let traits = fixture.traits();

    let start = Instant::now();
    for _ in 0..DEPLOY_ITERATIONS {
        // SAFETY: fixture-owned C strings outlive the deployer initialization call.
        unsafe { deployer_initialize(&traits) };
        assert_eq!(deploy(), TRUE);
        assert_eq!(deploy_schema(schema_file.as_ptr()), TRUE);
        let session_id = create_session();
        assert_ne!(session_id, 0, "create_session returned 0");
        // SAFETY: schema_id points to a valid NUL-terminated logical schema ID.
        assert_eq!(
            unsafe { select_schema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(destroy_session(session_id), TRUE);
    }
    let total = start.elapsed();
    cleanup_all_sessions();
    reset_runtime(api);

    BenchResult {
        name: "schema_deploy_dictionary_load",
        operations: DEPLOY_ITERATIONS,
        fixture: "lookup-schema-table-dictionary",
        data_size: "dictionary_entries=4 deploy_cycles=20".to_owned(),
        total,
    }
}

fn run_userdb_learning_sync(api: &yune_rime_api::RimeApi) -> BenchResult {
    let fixture = RuntimeFixture::new("userdb-learning-sync");
    write_userdb_schema(&fixture.shared, &fixture.user);
    setup_fixture(api, &fixture);
    let cleanup_all_sessions = require("cleanup_all_sessions", api.cleanup_all_sessions);
    let create_session = require("create_session", api.create_session);
    let destroy_session = require("destroy_session", api.destroy_session);
    let process_key = require("process_key", api.process_key);
    let select_schema = require("select_schema", api.select_schema);
    let commit_composition = require("commit_composition", api.commit_composition);
    let sync_user_data = require("sync_user_data", api.sync_user_data);
    let schema_id = CString::new("learn").expect("schema id should be valid");

    let start = Instant::now();
    for _ in 0..USERDB_ITERATIONS {
        let session_id = create_session();
        assert_ne!(session_id, 0, "create_session returned 0");
        // SAFETY: schema_id points to a valid NUL-terminated logical schema ID.
        assert_eq!(
            unsafe { select_schema(session_id, schema_id.as_ptr()) },
            TRUE
        );
        assert_eq!(process_key(session_id, 'n' as c_int, 0), TRUE);
        assert_eq!(process_key(session_id, 'i' as c_int, 0), TRUE);
        assert_eq!(commit_composition(session_id), TRUE);
        assert_eq!(destroy_session(session_id), TRUE);
        assert_eq!(sync_user_data(), TRUE);
    }
    let total = start.elapsed();
    cleanup_all_sessions();
    reset_runtime(api);

    BenchResult {
        name: "userdb_learning_sync",
        operations: USERDB_ITERATIONS,
        fixture: "learn-schema-synthetic-userdb",
        data_size: format!("seeded_userdb_records={USERDB_SEEDED_RECORDS} commits=80 syncs=80"),
        total,
    }
}

fn require<T>(name: &str, function: Option<T>) -> T {
    function.unwrap_or_else(|| panic!("RimeApi missing required benchmark function: {name}"))
}

fn print_results(results: &[BenchResult]) {
    println!("frontend_baselines phase={PHASE_LABEL}");
    println!("profile=bench");
    println!("tool=dependency-free std::time harness");
    println!("unit=microseconds_per_operation");
    println!("| benchmark | operations | fixture | data_size | total_ms | us_per_op |");
    println!("|---|---:|---|---|---:|---:|");
    for result in results {
        println!(
            "| {} | {} | {} | {} | {:.3} | {:.3} |",
            result.name,
            result.operations,
            result.fixture,
            result.data_size,
            result.total.as_secs_f64() * 1_000.0,
            result.per_operation_micros()
        );
    }
}
