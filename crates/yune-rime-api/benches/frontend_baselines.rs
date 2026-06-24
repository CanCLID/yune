use std::{
    collections::BTreeMap,
    ffi::{c_void, CString},
    fs, mem,
    os::raw::c_int,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_yaml::Value;
use yune_core::{
    parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_table_bin_dictionary, Engine, KeyEvent, RimePrismBinPayload, StaticTableTranslator,
    TableDictionary, TableDictionaryAdvancedData, TYPEDUCK_SENTENCE_WORD_PENALTY,
};
use yune_rime_api::{
    begin_startup_trace, finish_startup_trace, rime_get_api, Bool, RimeCommit, RimeComposition,
    RimeContext, RimeMenu, RimeSessionId, RimeStatus, RimeTraits, StartupTraceMemorySample, TRUE,
};

const PHASE_LABEL: &str = "06-04";
const XK_BACKSPACE: c_int = 0xff08;
const SESSION_ITERATIONS: u64 = 200;
const SIMPLE_KEY_ITERATIONS: u64 = 600;
const SCHEMA_LOOKUP_ITERATIONS: u64 = 200;
const DEPLOY_ITERATIONS: u64 = 20;
const USERDB_ITERATIONS: u64 = 80;
const USERDB_SEEDED_RECORDS: usize = 3;
const REAL_KEY_WARM_SAMPLES: usize = 16;
const REAL_STARTUP_SAMPLES: usize = 5;

fn main() {
    let _guard = bench_guard();
    let api = api_table();
    validate_api_table(api);

    let mut results = vec![
        run_m29_single_startup_memory(api, RealSchema::Jyut6ping3Mobile),
        run_session_lifecycle(api),
        run_simple_ascii_key(api),
        run_schema_loaded_key(api),
        run_deploy_dictionary_load(api),
        run_userdb_learning_sync(api),
        run_real_per_key_full_abi(api, "hai"),
        run_real_per_key_engine_only(RealSchema::Jyut6ping3Mobile, "hai", false),
        run_real_per_key_full_abi(api, "ngohaig"),
        run_real_per_key_engine_only(RealSchema::Jyut6ping3Mobile, "ngohaig", false),
        run_real_per_key_full_abi(api, "jigaajiusihaa"),
        run_real_per_key_engine_only(RealSchema::Jyut6ping3Mobile, "jigaajiusihaa", false),
        run_real_per_key_full_abi(api, "loengjathau"),
        run_real_per_key_engine_only(RealSchema::Jyut6ping3Mobile, "loengjathau", false),
        run_real_per_key_full_abi_with_correction(api, "jigaajiusihaa"),
        run_real_per_key_engine_only(RealSchema::Jyut6ping3Mobile, "jigaajiusihaa", true),
        run_real_per_key_full_abi_for_schema(api, RealSchema::LunaPinyin, "hao"),
        run_real_per_key_engine_only(RealSchema::LunaPinyin, "hao", false),
        run_real_per_key_full_abi_for_schema(api, RealSchema::LunaPinyin, "ni"),
        run_real_per_key_engine_only(RealSchema::LunaPinyin, "ni", false),
        run_real_per_key_full_abi_for_schema(api, RealSchema::LunaPinyin, "zhongguo"),
        run_real_per_key_engine_only(RealSchema::LunaPinyin, "zhongguo", false),
        run_real_per_key_full_abi_for_schema(api, RealSchema::Cangjie5, "a"),
        run_real_per_key_engine_only(RealSchema::Cangjie5, "a", false),
        run_real_startup_runtime_ready(api, RealSchema::Jyut6ping3Mobile),
        run_real_deploy_cache_hit(api, RealSchema::Jyut6ping3Mobile),
        run_real_startup_runtime_ready(api, RealSchema::LunaPinyin),
    ];
    results.extend(run_real_startup_trace_rows(
        api,
        RealSchema::Jyut6ping3Mobile,
    ));
    results.extend(run_real_startup_trace_rows(api, RealSchema::LunaPinyin));

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
    name: String,
    operations: u64,
    fixture: &'static str,
    schema_id: &'static str,
    asset_set: &'static str,
    data_size: String,
    total: Duration,
    stats: Option<BenchStats>,
    memory_notes: String,
}

impl BenchResult {
    fn per_operation_micros(&self) -> f64 {
        self.total.as_secs_f64() * 1_000_000.0 / self.operations as f64
    }
}

#[derive(Clone, Debug)]
struct BenchStats {
    median: f64,
    p95: f64,
    p99: f64,
    max: f64,
    cold_first: Option<f64>,
}

impl BenchStats {
    fn from_samples(samples: &[f64], cold_first_us: Option<f64>) -> Self {
        assert!(!samples.is_empty(), "benchmark samples should not be empty");
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        Self {
            median: percentile(&sorted, 0.50),
            p95: percentile(&sorted, 0.95),
            p99: percentile(&sorted, 0.99),
            max: *sorted.last().expect("samples should not be empty"),
            cold_first: cold_first_us,
        }
    }
}

#[derive(Clone, Debug)]
struct StartupSpan {
    name: &'static str,
    micros: u128,
    working_set_before: Option<u64>,
    working_set_after: Option<u64>,
    peak_working_set_after: Option<u64>,
}

#[derive(Clone, Debug)]
struct MemorySample {
    working_set_bytes: Option<u64>,
    peak_working_set_bytes: Option<u64>,
}

#[derive(Clone, Copy)]
enum RealSchema {
    Jyut6ping3Mobile,
    LunaPinyin,
    Cangjie5,
}

impl RealSchema {
    const fn schema_id(self) -> &'static str {
        match self {
            Self::Jyut6ping3Mobile => "jyut6ping3_mobile",
            Self::LunaPinyin => "luna_pinyin",
            Self::Cangjie5 => "cangjie5",
        }
    }

    const fn dictionary_id(self) -> &'static str {
        match self {
            Self::Jyut6ping3Mobile => "jyut6ping3",
            Self::LunaPinyin => "luna_pinyin",
            Self::Cangjie5 => "cangjie5",
        }
    }

    const fn prism_id(self) -> &'static str {
        match self {
            Self::Jyut6ping3Mobile => "jyut6ping3_mobile",
            Self::LunaPinyin => "luna_pinyin",
            Self::Cangjie5 => "cangjie5",
        }
    }

    const fn asset_set(self) -> &'static str {
        "apps/yune-web/source/public/schema"
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
        name: "session_create_destroy".to_owned(),
        operations: SESSION_ITERATIONS,
        fixture: "synthetic-basic-schema",
        schema_id: "baseline",
        asset_set: "synthetic",
        data_size: "sessions=200".to_owned(),
        total,
        stats: None,
        memory_notes: "not measured".to_owned(),
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
        name: "per_key_simple_ascii_rime_process_key".to_owned(),
        operations: SIMPLE_KEY_ITERATIONS,
        fixture: "default-echo-schema",
        schema_id: "baseline",
        asset_set: "synthetic",
        data_size: "keys=600 status/context/commit/free cycles=600".to_owned(),
        total,
        stats: None,
        memory_notes: "not measured".to_owned(),
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
        name: "per_key_schema_loaded_lookup_rime_process_key".to_owned(),
        operations: SCHEMA_LOOKUP_ITERATIONS * 2,
        fixture: "lookup-schema-table-dictionary",
        schema_id: "lookup",
        asset_set: "synthetic",
        data_size: "dictionary_entries=4 sessions=200 status/context/commit/free cycles=200"
            .to_owned(),
        total,
        stats: None,
        memory_notes: "not measured".to_owned(),
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
        name: "schema_deploy_dictionary_load".to_owned(),
        operations: DEPLOY_ITERATIONS,
        fixture: "lookup-schema-table-dictionary",
        schema_id: "lookup",
        asset_set: "synthetic",
        data_size: "dictionary_entries=4 deploy_cycles=20".to_owned(),
        total,
        stats: None,
        memory_notes: "not measured".to_owned(),
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
        name: "userdb_learning_sync".to_owned(),
        operations: USERDB_ITERATIONS,
        fixture: "learn-schema-synthetic-userdb",
        schema_id: "learn",
        asset_set: "synthetic",
        data_size: format!("seeded_userdb_records={USERDB_SEEDED_RECORDS} commits=80 syncs=80"),
        total,
        stats: None,
        memory_notes: "not measured".to_owned(),
    }
}

fn run_real_per_key_full_abi(api: &yune_rime_api::RimeApi, input: &'static str) -> BenchResult {
    run_real_per_key_full_abi_for_schema(api, RealSchema::Jyut6ping3Mobile, input)
}

fn run_real_per_key_full_abi_with_correction(
    api: &yune_rime_api::RimeApi,
    input: &'static str,
) -> BenchResult {
    run_real_per_key_full_abi_config(api, RealSchema::Jyut6ping3Mobile, input, true)
}

fn run_real_per_key_full_abi_for_schema(
    api: &yune_rime_api::RimeApi,
    schema: RealSchema,
    input: &'static str,
) -> BenchResult {
    run_real_per_key_full_abi_config(api, schema, input, false)
}

fn run_real_per_key_full_abi_config(
    api: &yune_rime_api::RimeApi,
    schema: RealSchema,
    input: &'static str,
    enable_correction: bool,
) -> BenchResult {
    let fixture = RuntimeFixture::new(&format!("real-{}-{}-full-abi", schema.schema_id(), input));
    let asset_metrics = write_real_schema_assets(&fixture);
    if enable_correction {
        enable_temp_common_correction(&fixture.shared);
    }
    setup_fixture(api, &fixture);
    if enable_correction {
        deploy_real_schema(api, &fixture, schema);
    }
    let cold_first_us = measure_full_abi_cold_first(api, schema, input);
    let (samples, total) = measure_full_abi_warm_samples(api, schema, input);
    reset_runtime(api);

    let key_count = input.chars().count() as u64;
    let operations = key_count * REAL_KEY_WARM_SAMPLES as u64;
    let scan_note = if enable_correction {
        "dynamic_correction_scan=enabled via temp common:/enable_correction"
    } else {
        "dynamic_correction_scan=profile-default"
    };
    BenchResult {
        name: real_per_key_name(schema, input, enable_correction, "full_abi"),
        operations,
        fixture: "typeduck-web-real-assets",
        schema_id: schema.schema_id(),
        asset_set: schema.asset_set(),
        data_size: format!(
            "input={input} keys_per_sample={key_count} warm_samples={REAL_KEY_WARM_SAMPLES} status/context/commit/free cycles={operations} asset_files={} asset_bytes={} {scan_note}",
            asset_metrics.files, asset_metrics.bytes
        ),
        total,
        stats: Some(BenchStats::from_samples(&samples, Some(cold_first_us))),
        memory_notes: format!(
            "RSS unavailable in std harness; copied_asset_bytes={} copied_asset_files={}",
            asset_metrics.bytes, asset_metrics.files
        ),
    }
}

fn measure_full_abi_cold_first(
    api: &yune_rime_api::RimeApi,
    schema: RealSchema,
    input: &str,
) -> f64 {
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
    let schema_id = CString::new(schema.schema_id()).expect("schema id should be valid");
    let session_id = create_session();
    assert_ne!(session_id, 0, "create_session returned 0");
    // SAFETY: schema_id points to a valid NUL-terminated logical schema ID.
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let first_key = input
        .chars()
        .next()
        .expect("real benchmark input should not be empty");
    let start = Instant::now();
    assert_eq!(process_key(session_id, first_key as c_int, 0), TRUE);
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
    let elapsed = start.elapsed();
    assert_eq!(destroy_session(session_id), TRUE);
    duration_micros(elapsed)
}

fn measure_full_abi_warm_samples(
    api: &yune_rime_api::RimeApi,
    schema: RealSchema,
    input: &str,
) -> (Vec<f64>, Duration) {
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
    let schema_id = CString::new(schema.schema_id()).expect("schema id should be valid");
    let session_id = create_session();
    assert_ne!(session_id, 0, "create_session returned 0");
    // SAFETY: schema_id points to a valid NUL-terminated logical schema ID.
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    let key_count = input.chars().count() as f64;
    let mut samples = Vec::with_capacity(REAL_KEY_WARM_SAMPLES);
    let mut total = Duration::ZERO;
    for _ in 0..REAL_KEY_WARM_SAMPLES {
        let start = Instant::now();
        for key in input.chars() {
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
        let elapsed = start.elapsed();
        total += elapsed;
        samples.push(duration_micros(elapsed) / key_count);
        clear_full_abi_input(process_key, session_id, input);
    }
    assert_eq!(destroy_session(session_id), TRUE);
    (samples, total)
}

fn clear_full_abi_input(
    process_key: extern "C" fn(RimeSessionId, c_int, c_int) -> Bool,
    session_id: RimeSessionId,
    input: &str,
) {
    for _ in input.chars() {
        assert_eq!(process_key(session_id, XK_BACKSPACE, 0), TRUE);
    }
}

fn run_real_per_key_engine_only(
    schema: RealSchema,
    input: &'static str,
    enable_correction: bool,
) -> BenchResult {
    let asset_metrics = real_asset_metrics();
    let mut engine = build_real_engine(schema, enable_correction);
    let first_key = input
        .chars()
        .next()
        .expect("real benchmark input should not be empty");
    let start = Instant::now();
    engine.process_key_event(KeyEvent::character(first_key));
    let cold_first_us = duration_micros(start.elapsed());
    engine.clear_composition();

    let key_count = input.chars().count() as f64;
    let mut samples = Vec::with_capacity(REAL_KEY_WARM_SAMPLES);
    let mut total = Duration::ZERO;
    for _ in 0..REAL_KEY_WARM_SAMPLES {
        let start = Instant::now();
        for key in input.chars() {
            engine.process_key_event(KeyEvent::character(key));
        }
        let elapsed = start.elapsed();
        total += elapsed;
        samples.push(duration_micros(elapsed) / key_count);
        engine.clear_composition();
    }
    let operations = input.chars().count() as u64 * REAL_KEY_WARM_SAMPLES as u64;
    let scan_note = if enable_correction {
        "dynamic_correction_scan=enabled; entries_by_code.keys() branch exercised"
    } else {
        "dynamic_correction_scan=profile-default"
    };
    BenchResult {
        name: real_per_key_name(schema, input, enable_correction, "engine_only"),
        operations,
        fixture: "typeduck-web-real-assets",
        schema_id: schema.schema_id(),
        asset_set: schema.asset_set(),
        data_size: format!(
            "input={input} keys_per_sample={} warm_samples={REAL_KEY_WARM_SAMPLES} engine_snapshot=excluded asset_files={} asset_bytes={} {scan_note}",
            input.chars().count(),
            asset_metrics.files,
            asset_metrics.bytes
        ),
        total,
        stats: Some(BenchStats::from_samples(&samples, Some(cold_first_us))),
        memory_notes: format!(
            "RSS unavailable in std harness; source_asset_bytes={} source_asset_files={}",
            asset_metrics.bytes, asset_metrics.files
        ),
    }
}

fn build_real_engine(schema: RealSchema, enable_correction: bool) -> Engine {
    let schema_config = real_schema_config(schema);
    let formulas = yaml_string_list(&schema_config, "speller/algebra");
    let delimiters =
        yaml_string(&schema_config, "speller/delimiter").unwrap_or_else(|| " ".to_owned());
    let enable_completion =
        yaml_bool(&schema_config, "translator/enable_completion").unwrap_or(true);
    let enable_sentence = yaml_bool(&schema_config, "translator/enable_sentence").unwrap_or(true);
    let combine_candidates =
        yaml_bool(&schema_config, "translator/combine_candidates").unwrap_or(false);
    let show_full_code = yaml_bool(&schema_config, "translator/show_full_code").unwrap_or(true);
    let prediction_never_first =
        yaml_bool(&schema_config, "translator/prediction_never_first").unwrap_or(false);
    let prediction_candidate_limit =
        yaml_usize(&schema_config, "translator/prediction_candidate_limit");
    let prefix_fallback = yaml_bool(&schema_config, "translator/prefix_fallback")
        .unwrap_or(matches!(schema, RealSchema::Jyut6ping3Mobile));

    let (dictionary, prism_payload) = load_real_dictionary_with_prism(schema);
    let mut translator = if matches!(schema, RealSchema::LunaPinyin) && prism_payload.is_some() {
        StaticTableTranslator::from_compact_dictionary(dictionary, prism_payload)
    } else {
        StaticTableTranslator::from_dictionary(dictionary)
    }
    .with_spelling_algebra(&formulas)
    .with_completion(enable_completion)
    .with_correction(enable_correction)
    .with_dynamic_correction_lookup(matches!(schema, RealSchema::Jyut6ping3Mobile))
    .with_sentence(enable_sentence)
    .with_delimiters(delimiters)
    .with_combine_candidates(combine_candidates)
    .with_show_full_code(show_full_code)
    .with_prediction_never_first(prediction_never_first)
    .with_prefix_fallback(prefix_fallback);
    if let Some(limit) = prediction_candidate_limit {
        translator = translator.with_prediction_candidate_limit(limit);
    }
    if matches!(schema, RealSchema::Jyut6ping3Mobile) {
        translator = translator.with_sentence_word_penalty(TYPEDUCK_SENTENCE_WORD_PENALTY);
    }

    let mut engine = Engine::new();
    engine.set_schema(schema.schema_id(), schema.schema_id());
    if prediction_never_first {
        engine.set_prediction_never_first(true);
    }
    engine.add_translator(translator);
    engine
}

fn load_real_dictionary_with_prism(
    schema: RealSchema,
) -> (TableDictionary, Option<RimePrismBinPayload>) {
    let schema_root = browser_app_schema_root();
    let dictionary_id = schema.dictionary_id();
    let table_bytes = fs::read(schema_root.join(format!("{dictionary_id}.table.bin")))
        .expect("real table bin should be readable");
    let reverse_bytes = fs::read(schema_root.join(format!("{dictionary_id}.reverse.bin")))
        .expect("real reverse bin should be readable");
    let mut dictionary = match (
        parse_rime_table_bin_dictionary(&table_bytes),
        parse_rime_reverse_bin_dictionary(&reverse_bytes),
    ) {
        (Ok(dictionary), Ok(reverse_dictionary)) => {
            dictionary.with_merged_advanced_data_from(&reverse_dictionary)
        }
        _ => load_source_dictionary(schema),
    };
    let prism_payload = real_prism_payload(schema);
    if let Some(prism_payload) = prism_payload.as_ref() {
        dictionary =
            dictionary.with_merged_advanced_data_from(&prism_advanced_dictionary(prism_payload));
    }
    (dictionary, prism_payload)
}

fn load_source_dictionary(schema: RealSchema) -> TableDictionary {
    let schema_root = browser_app_schema_root();
    let dictionary_id = schema.dictionary_id();
    let dictionary_yaml =
        fs::read_to_string(schema_root.join(format!("{dictionary_id}.dict.yaml")))
            .expect("real source dictionary should be readable");
    TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        &dictionary_yaml,
        std::iter::empty::<&str>(),
        |import_table| {
            fs::read_to_string(browser_app_schema_root().join(format!("{import_table}.dict.yaml")))
                .ok()
        },
        |vocabulary| {
            fs::read_to_string(browser_app_schema_root().join(format!("{vocabulary}.txt"))).ok()
        },
    )
    .expect("real source dictionary should parse")
}

fn real_prism_payload(schema: RealSchema) -> Option<RimePrismBinPayload> {
    let schema_root = browser_app_schema_root();
    let prism_path = schema_root.join(format!("{}.prism.bin", schema.prism_id()));
    if prism_path.is_file() {
        let prism_bytes = fs::read(prism_path).expect("real prism bin should be readable");
        return parse_rime_prism_bin_payload(&prism_bytes).ok();
    }
    None
}

fn prism_advanced_dictionary(prism_payload: &RimePrismBinPayload) -> TableDictionary {
    TableDictionary::with_advanced_data(
        Vec::<yune_core::TableEntry>::new(),
        TableDictionaryAdvancedData {
            corrections: prism_payload.corrections.clone(),
            tolerance_rules: prism_payload.tolerance_rules.clone(),
            ..TableDictionaryAdvancedData::default()
        },
    )
}

fn run_real_startup_runtime_ready(api: &yune_rime_api::RimeApi, schema: RealSchema) -> BenchResult {
    let mut samples = Vec::with_capacity(REAL_STARTUP_SAMPLES);
    let mut total = Duration::ZERO;
    let mut last_metrics = AssetMetrics::default();
    for _ in 0..REAL_STARTUP_SAMPLES {
        let fixture = RuntimeFixture::new(&format!("startup-{}", schema.schema_id()));
        last_metrics = write_real_schema_assets(&fixture);
        let start = Instant::now();
        setup_fixture(api, &fixture);
        let session_id = create_and_select_real_schema(api, schema);
        let destroy_session = require("destroy_session", api.destroy_session);
        assert_eq!(destroy_session(session_id), TRUE);
        let elapsed = start.elapsed();
        total += elapsed;
        samples.push(duration_micros(elapsed));
        reset_runtime(api);
    }
    BenchResult {
        name: format!("startup_real_{}_runtime_ready", schema.schema_id()),
        operations: REAL_STARTUP_SAMPLES as u64,
        fixture: "typeduck-web-real-assets",
        schema_id: schema.schema_id(),
        asset_set: schema.asset_set(),
        data_size: format!(
            "startup_samples={REAL_STARTUP_SAMPLES} asset_files={} asset_bytes={}",
            last_metrics.files, last_metrics.bytes
        ),
        total,
        stats: Some(BenchStats::from_samples(&samples, None)),
        memory_notes: format!(
            "RSS unavailable in std harness; copied_asset_bytes={} copied_asset_files={}",
            last_metrics.bytes, last_metrics.files
        ),
    }
}

fn run_m29_single_startup_memory(api: &yune_rime_api::RimeApi, schema: RealSchema) -> BenchResult {
    reset_runtime(api);
    let fixture = RuntimeFixture::new(&format!("m29-single-startup-memory-{}", schema.schema_id()));
    let before_assets = current_memory_sample();
    let asset_metrics = write_real_schema_assets(&fixture);
    let start = Instant::now();
    setup_fixture(api, &fixture);
    let session_id = create_and_select_real_schema(api, schema);
    let after_ready = current_memory_sample();
    let destroy_session = require("destroy_session", api.destroy_session);
    assert_eq!(destroy_session(session_id), TRUE);
    reset_runtime(api);
    let after_finalize = current_memory_sample();
    let elapsed = start.elapsed();
    let peak = [
        before_assets.peak_working_set_bytes,
        after_ready.peak_working_set_bytes,
        after_finalize.peak_working_set_bytes,
    ]
    .into_iter()
    .flatten()
    .max();

    BenchResult {
        name: format!("m29_single_startup_memory_{}", schema.schema_id()),
        operations: 1,
        fixture: "typeduck-web-real-assets",
        schema_id: schema.schema_id(),
        asset_set: schema.asset_set(),
        data_size: format!(
            "single_startup_memory_sample asset_files={} asset_bytes={}",
            asset_metrics.files, asset_metrics.bytes
        ),
        total: elapsed,
        stats: Some(BenchStats::from_samples(&[duration_micros(elapsed)], None)),
        memory_notes: format!(
            "before_bytes={} after_ready_bytes={} after_finalize_bytes={} peak_bytes={} ready_delta_bytes={} finalize_delta_bytes={}",
            format_optional_u64(before_assets.working_set_bytes),
            format_optional_u64(after_ready.working_set_bytes),
            format_optional_u64(after_finalize.working_set_bytes),
            format_optional_u64(peak),
            delta_optional_i128(before_assets.working_set_bytes, after_ready.working_set_bytes),
            delta_optional_i128(after_ready.working_set_bytes, after_finalize.working_set_bytes)
        ),
    }
}

fn run_real_deploy_cache_hit(api: &yune_rime_api::RimeApi, schema: RealSchema) -> BenchResult {
    let fixture = RuntimeFixture::new(&format!("deploy-cache-hit-{}", schema.schema_id()));
    let asset_metrics = write_real_schema_assets(&fixture);
    setup_fixture(api, &fixture);
    let mut samples = Vec::with_capacity(REAL_STARTUP_SAMPLES);
    let mut total = Duration::ZERO;
    for _ in 0..REAL_STARTUP_SAMPLES {
        let start = Instant::now();
        deploy_real_schema(api, &fixture, schema);
        let elapsed = start.elapsed();
        total += elapsed;
        samples.push(duration_micros(elapsed));
    }
    reset_runtime(api);
    BenchResult {
        name: format!("deploy_real_{}_cache_hit", schema.schema_id()),
        operations: REAL_STARTUP_SAMPLES as u64,
        fixture: "typeduck-web-real-assets",
        schema_id: schema.schema_id(),
        asset_set: schema.asset_set(),
        data_size: format!(
            "deploy_samples={REAL_STARTUP_SAMPLES} prebuilt_build_dir=true asset_files={} asset_bytes={}",
            asset_metrics.files, asset_metrics.bytes
        ),
        total,
        stats: Some(BenchStats::from_samples(&samples, None)),
        memory_notes: format!(
            "RSS unavailable in std harness; copied_asset_bytes={} copied_asset_files={}",
            asset_metrics.bytes, asset_metrics.files
        ),
    }
}

fn run_real_startup_trace_rows(
    api: &yune_rime_api::RimeApi,
    schema: RealSchema,
) -> Vec<BenchResult> {
    let mut samples_by_name: BTreeMap<&'static str, Vec<StartupSpan>> = BTreeMap::new();

    for _ in 0..REAL_STARTUP_SAMPLES {
        let mut sample_spans: BTreeMap<&'static str, StartupSpan> = BTreeMap::new();
        let fixture = RuntimeFixture::new(&format!("startup-trace-{}", schema.schema_id()));

        let (_, span) = measure_startup_span("write_real_schema_assets", || {
            write_real_schema_assets(&fixture)
        });
        merge_startup_span(&mut sample_spans, span);

        let ((), span) = measure_startup_span("teardown_or_finalize", || reset_runtime(api));
        merge_startup_span(&mut sample_spans, span);

        let traits = fixture.traits();
        let setup = require("setup", api.setup);
        let initialize = require("initialize", api.initialize);
        let ((), span) = measure_startup_span("setup", || {
            // SAFETY: fixture-owned C strings outlive setup.
            unsafe { setup(&traits) };
        });
        merge_startup_span(&mut sample_spans, span);
        let ((), span) = measure_startup_span("initialize", || {
            // SAFETY: fixture-owned C strings outlive initialize.
            unsafe { initialize(&traits) };
        });
        merge_startup_span(&mut sample_spans, span);

        let create_session = require("create_session", api.create_session);
        let (session_id, span) = measure_startup_span("create_session", || create_session());
        assert_ne!(session_id, 0, "create_session returned 0");
        merge_startup_span(&mut sample_spans, span);

        begin_startup_trace(Some(current_startup_trace_memory_sample));
        let select_schema = require("select_schema", api.select_schema);
        let schema_id = CString::new(schema.schema_id()).expect("schema id should be valid");
        let ((), span) = measure_startup_span("select_schema_total", || {
            // SAFETY: schema_id points to a valid NUL-terminated logical schema ID.
            assert_eq!(
                unsafe { select_schema(session_id, schema_id.as_ptr()) },
                TRUE
            );
        });
        let trace_events = finish_startup_trace();
        merge_startup_span(&mut sample_spans, span);
        for event in trace_events {
            merge_startup_span(
                &mut sample_spans,
                StartupSpan {
                    name: event.name,
                    micros: event.micros,
                    working_set_before: event.working_set_before,
                    working_set_after: event.working_set_after,
                    peak_working_set_after: event.peak_working_set_after,
                },
            );
        }

        let destroy_session = require("destroy_session", api.destroy_session);
        let ((), span) = measure_startup_span("destroy_session", || {
            assert_eq!(destroy_session(session_id), TRUE);
        });
        merge_startup_span(&mut sample_spans, span);

        let ((), span) = measure_startup_span("teardown_or_finalize", || reset_runtime(api));
        merge_startup_span(&mut sample_spans, span);

        for (name, span) in sample_spans {
            samples_by_name.entry(name).or_default().push(span);
        }
    }

    samples_by_name
        .into_iter()
        .map(|(name, spans)| startup_span_result(schema, name, &spans))
        .collect()
}

fn create_and_select_real_schema(
    api: &yune_rime_api::RimeApi,
    schema: RealSchema,
) -> RimeSessionId {
    let create_session = require("create_session", api.create_session);
    let select_schema = require("select_schema", api.select_schema);
    let schema_id = CString::new(schema.schema_id()).expect("schema id should be valid");
    let session_id = create_session();
    assert_ne!(session_id, 0, "create_session returned 0");
    // SAFETY: schema_id points to a valid NUL-terminated logical schema ID.
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    session_id
}

fn deploy_real_schema(api: &yune_rime_api::RimeApi, fixture: &RuntimeFixture, schema: RealSchema) {
    let deployer_initialize = require("deployer_initialize", api.deployer_initialize);
    let deploy = require("deploy", api.deploy);
    let deploy_schema = require("deploy_schema", api.deploy_schema);
    let traits = fixture.traits();
    let schema_file =
        CString::new(format!("{}.schema.yaml", schema.schema_id())).expect("schema file is valid");
    // SAFETY: fixture-owned C strings outlive the deployer initialization call.
    unsafe { deployer_initialize(&traits) };
    assert_eq!(deploy(), TRUE);
    assert_eq!(deploy_schema(schema_file.as_ptr()), TRUE);
}

#[derive(Clone, Copy, Debug, Default)]
struct AssetMetrics {
    files: usize,
    bytes: u64,
}

fn write_real_schema_assets(fixture: &RuntimeFixture) -> AssetMetrics {
    let schema_root = browser_app_schema_root();
    copy_dir_files(&schema_root, &fixture.shared)
        .expect("real browser schema assets should copy to shared dir");
    let build_root = schema_root.join("build");
    if build_root.is_dir() {
        copy_dir_files(&build_root, &fixture.user.join("build"))
            .expect("real browser build assets should copy to user build dir");
    }
    asset_metrics(&schema_root).expect("real asset metrics should be readable")
}

fn copy_dir_files(source_root: &Path, destination_root: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination_root)?;
    for entry in fs::read_dir(source_root)? {
        let entry = entry?;
        let source = entry.path();
        let destination = destination_root.join(entry.file_name());
        if source.is_dir() {
            copy_dir_files(&source, &destination)?;
        } else if source.is_file() {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}

fn asset_metrics(root: &Path) -> std::io::Result<AssetMetrics> {
    let mut metrics = AssetMetrics::default();
    collect_asset_metrics(root, &mut metrics)?;
    Ok(metrics)
}

fn collect_asset_metrics(root: &Path, metrics: &mut AssetMetrics) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_asset_metrics(&path, metrics)?;
        } else if path.is_file() {
            metrics.files += 1;
            metrics.bytes += entry.metadata()?.len();
        }
    }
    Ok(())
}

fn real_asset_metrics() -> AssetMetrics {
    asset_metrics(&browser_app_schema_root()).expect("real asset metrics should be readable")
}

fn enable_temp_common_correction(shared: &Path) {
    let path = shared.join("common.custom.yaml");
    let contents = fs::read_to_string(&path).expect("common.custom.yaml should be readable");
    let patched = contents.replace(
        "# - common:/enable_correction",
        "- common:/enable_correction",
    );
    fs::write(path, patched).expect("temporary common.custom.yaml should be patched");
}

fn real_schema_config(schema: RealSchema) -> Value {
    let path = browser_app_schema_root()
        .join("build")
        .join(format!("{}.schema.yaml", schema.schema_id()));
    let yaml = fs::read_to_string(path).expect("prebuilt real schema YAML should be readable");
    serde_yaml::from_str(&yaml).expect("prebuilt real schema YAML should parse")
}

fn yaml_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for component in path.split('/') {
        let mapping = current.as_mapping()?;
        current = mapping.get(Value::String(component.to_owned()))?;
    }
    Some(current)
}

fn yaml_string(value: &Value, path: &str) -> Option<String> {
    yaml_at_path(value, path)?.as_str().map(ToOwned::to_owned)
}

fn yaml_bool(value: &Value, path: &str) -> Option<bool> {
    yaml_at_path(value, path)?.as_bool()
}

fn yaml_usize(value: &Value, path: &str) -> Option<usize> {
    yaml_at_path(value, path)?
        .as_i64()
        .and_then(|value| usize::try_from(value).ok())
}

fn yaml_string_list(value: &Value, path: &str) -> Vec<String> {
    let Some(sequence) = yaml_at_path(value, path).and_then(Value::as_sequence) else {
        return Vec::new();
    };
    sequence
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn browser_app_schema_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/yune-web/source/public/schema")
}

fn real_per_key_name(
    schema: RealSchema,
    input: &str,
    enable_correction: bool,
    suffix: &str,
) -> String {
    let mut name = format!(
        "per_key_real_{}_{}_{}",
        schema.schema_id(),
        sanitize_benchmark_name(input),
        suffix
    );
    if enable_correction {
        name = format!(
            "per_key_real_{}_{}_correction_{}",
            schema.schema_id(),
            sanitize_benchmark_name(input),
            suffix
        );
    }
    name
}

fn sanitize_benchmark_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn duration_micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000_000.0
}

fn percentile(sorted_samples: &[f64], percentile: f64) -> f64 {
    assert!(
        !sorted_samples.is_empty(),
        "benchmark samples should not be empty"
    );
    let index = ((sorted_samples.len() - 1) as f64 * percentile).ceil() as usize;
    sorted_samples[index.min(sorted_samples.len() - 1)]
}

#[cfg(windows)]
fn current_memory_sample() -> MemorySample {
    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }

    #[link(name = "psapi")]
    extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    let mut counters = ProcessMemoryCounters {
        cb: mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            mem::size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    if ok == 0 {
        return MemorySample {
            working_set_bytes: None,
            peak_working_set_bytes: None,
        };
    }
    MemorySample {
        working_set_bytes: Some(counters.working_set_size as u64),
        peak_working_set_bytes: Some(counters.peak_working_set_size as u64),
    }
}

#[cfg(not(windows))]
fn current_memory_sample() -> MemorySample {
    MemorySample {
        working_set_bytes: None,
        peak_working_set_bytes: None,
    }
}

fn current_startup_trace_memory_sample() -> StartupTraceMemorySample {
    let sample = current_memory_sample();
    StartupTraceMemorySample {
        working_set_bytes: sample.working_set_bytes,
        peak_working_set_bytes: sample.peak_working_set_bytes,
    }
}

fn measure_startup_span<T>(name: &'static str, f: impl FnOnce() -> T) -> (T, StartupSpan) {
    let before = current_memory_sample();
    let start = Instant::now();
    let output = f();
    let elapsed = start.elapsed();
    let after = current_memory_sample();
    (
        output,
        StartupSpan {
            name,
            micros: elapsed.as_micros(),
            working_set_before: before.working_set_bytes,
            working_set_after: after.working_set_bytes,
            peak_working_set_after: after.peak_working_set_bytes,
        },
    )
}

fn merge_startup_span(spans: &mut BTreeMap<&'static str, StartupSpan>, span: StartupSpan) {
    spans
        .entry(span.name)
        .and_modify(|existing| {
            existing.micros = existing.micros.saturating_add(span.micros);
            if existing.working_set_before.is_none() {
                existing.working_set_before = span.working_set_before;
            }
            if span.working_set_after.is_some() {
                existing.working_set_after = span.working_set_after;
            }
            existing.peak_working_set_after =
                max_optional(existing.peak_working_set_after, span.peak_working_set_after);
        })
        .or_insert(span);
}

fn startup_span_result(
    schema: RealSchema,
    span_name: &'static str,
    spans: &[StartupSpan],
) -> BenchResult {
    let samples = spans
        .iter()
        .map(|span| span.micros as f64)
        .collect::<Vec<_>>();
    let total_micros = spans
        .iter()
        .fold(0_u128, |total, span| total.saturating_add(span.micros));
    BenchResult {
        name: format!("startup_trace_{}_{}", schema.schema_id(), span_name),
        operations: spans.len() as u64,
        fixture: "typeduck-web-real-assets",
        schema_id: schema.schema_id(),
        asset_set: schema.asset_set(),
        data_size: format!(
            "startup_trace_samples={} observed_samples={}",
            REAL_STARTUP_SAMPLES,
            spans.len()
        ),
        total: duration_from_micros(total_micros),
        stats: Some(BenchStats::from_samples(&samples, None)),
        memory_notes: startup_memory_notes(spans),
    }
}

fn duration_from_micros(micros: u128) -> Duration {
    Duration::from_micros(u64::try_from(micros).unwrap_or(u64::MAX))
}

fn startup_memory_notes(spans: &[StartupSpan]) -> String {
    let before = median_optional_u64(spans.iter().filter_map(|span| span.working_set_before));
    let after = median_optional_u64(spans.iter().filter_map(|span| span.working_set_after));
    let peak = spans
        .iter()
        .filter_map(|span| span.peak_working_set_after)
        .max();
    let delta = before
        .zip(after)
        .map(|(before, after)| i128::from(after) - i128::from(before));
    format!(
        "working_set_before_bytes={} working_set_after_bytes={} peak_working_set_after_bytes={} memory_delta_bytes={}",
        format_optional_u64(before),
        format_optional_u64(after),
        format_optional_u64(peak),
        delta.map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
    )
}

fn median_optional_u64(values: impl Iterator<Item = u64>) -> Option<u64> {
    let mut values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

fn max_optional(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn format_optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
}

fn delta_optional_i128(before: Option<u64>, after: Option<u64>) -> String {
    before.zip(after).map_or_else(
        || "unavailable".to_owned(),
        |(before, after)| (i128::from(after) - i128::from(before)).to_string(),
    )
}

fn require<T>(name: &str, function: Option<T>) -> T {
    function.unwrap_or_else(|| panic!("RimeApi missing required benchmark function: {name}"))
}

fn print_results(results: &[BenchResult]) {
    println!("frontend_baselines phase={PHASE_LABEL}");
    println!("profile=bench");
    println!("tool=dependency-free std::time harness");
    println!("unit=microseconds_per_operation");
    println!(
        "| benchmark | operations | fixture | schema_id | asset_set | data_size | total_ms | us_per_op | median_us | p95_us | p99_us | max_us | cold_first_us | memory_notes |"
    );
    println!("|---|---:|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|");
    for result in results {
        let (median, p95, p99, max, cold_first) = result.stats.as_ref().map_or_else(
            || {
                (
                    "-".to_owned(),
                    "-".to_owned(),
                    "-".to_owned(),
                    "-".to_owned(),
                    "-".to_owned(),
                )
            },
            |stats| {
                (
                    format!("{:.3}", stats.median),
                    format!("{:.3}", stats.p95),
                    format!("{:.3}", stats.p99),
                    format!("{:.3}", stats.max),
                    stats
                        .cold_first
                        .map_or_else(|| "-".to_owned(), |value| format!("{value:.3}")),
                )
            },
        );
        println!(
            "| {} | {} | {} | {} | {} | {} | {:.3} | {:.3} | {} | {} | {} | {} | {} | {} |",
            result.name,
            result.operations,
            result.fixture,
            result.schema_id,
            result.asset_set,
            result.data_size,
            result.total.as_secs_f64() * 1_000.0,
            result.per_operation_micros(),
            median,
            p95,
            p99,
            max,
            cold_first,
            result.memory_notes
        );
    }
}
