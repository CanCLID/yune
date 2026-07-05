use std::{
    ffi::{CStr, CString},
    fs, mem,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use yune_core::RimeDictArtifactStatus;
use yune_rime_api::{
    workspace_dictionary_rebuild_reports, RimeCleanupAllSessions, RimeContext, RimeCreateSession,
    RimeDeployerInitialize, RimeDestroySession, RimeFinalize, RimeFreeContext, RimeGetContext,
    RimeInitialize, RimeMenu, RimeProcessKey, RimeRunTask, RimeSelectSchema, RimeTraits, FALSE,
    TRUE,
};

const LUNA_BASIC_FIXTURE: &str =
    include_str!("../../yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json");
const TYPEDUCK_BOUNDARY_FIXTURE: &str = include_str!(
    "../../yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-windows-boundary-ngohaig.json"
);

#[test]
fn product_schema_cold_then_warm_starts_conform_from_isolated_build_output() {
    let _guard = test_guard();

    assert_product_path_conforms(ProductPath {
        deploy_schema_id: "luna_pinyin",
        runtime_schema_id: "luna_pinyin",
        dictionary_id: "luna_pinyin",
        fixture: LUNA_BASIC_FIXTURE,
        input: "ni",
        expected_count: 3,
    });
    assert_product_path_conforms(ProductPath {
        deploy_schema_id: "jyut6ping3",
        runtime_schema_id: "jyut6ping3_mobile",
        dictionary_id: "jyut6ping3",
        fixture: TYPEDUCK_BOUNDARY_FIXTURE,
        input: "ngohaig",
        expected_count: 4,
    });
}

struct ProductPath {
    deploy_schema_id: &'static str,
    runtime_schema_id: &'static str,
    dictionary_id: &'static str,
    fixture: &'static str,
    input: &'static str,
    expected_count: usize,
}

fn assert_product_path_conforms(product: ProductPath) {
    let runtime = PreparedRuntime::new(
        &format!("m56-product-cold-warm-start-{}", product.runtime_schema_id),
        product.deploy_schema_id,
    );
    setup_deployer(&runtime);
    let cold_reports = run_product_workspace_updates(product.deploy_schema_id, "cold");
    ExpectedArtifactStatus::Available.assert_for(&cold_reports, product.dictionary_id);

    setup_deployer(&runtime);
    let warm_reports = run_product_workspace_updates(product.deploy_schema_id, "warm");
    ExpectedArtifactStatus::NoRebuild.assert_for(&warm_reports, product.dictionary_id);
    initialize_runtime(&runtime);
    assert_candidates_match_fixture(
        product.runtime_schema_id,
        product.fixture,
        product.input,
        product.expected_count,
    );
    RimeCleanupAllSessions();
    RimeFinalize();
    runtime.remove();
}

fn run_product_workspace_updates(
    schema_id: &str,
    phase: &str,
) -> Vec<yune_rime_api::WorkspaceDictionaryRebuildReport> {
    let task_name = "workspace_update";
    let task = CString::new(task_name).expect("task name should not contain NUL");
    let task_result = RimeRunTask(task.as_ptr());
    let reports_after_task = workspace_dictionary_rebuild_reports();
    assert_eq!(
        task_result, TRUE,
        "{task_name} should deploy {schema_id} in isolated {phase} runtime: {reports_after_task:?}"
    );
    workspace_dictionary_rebuild_reports()
}

fn assert_candidates_match_fixture(
    schema_id: &str,
    fixture_json: &str,
    input: &str,
    expected_count: usize,
) {
    let expected = fixture_candidate_texts(fixture_json, input, expected_count);
    let actual = candidate_texts(schema_id, input);
    assert!(
        actual.len() >= expected.len(),
        "{schema_id} should produce at least {} candidates for {input:?}: {actual:?}",
        expected.len()
    );
    assert_eq!(
        &actual[..expected.len()],
        expected.as_slice(),
        "{schema_id} first candidates should match oracle fixture for {input:?}"
    );
}

fn fixture_candidate_texts(fixture_json: &str, input: &str, expected_count: usize) -> Vec<String> {
    let fixture: Value = serde_json::from_str(fixture_json).expect("fixture should parse");
    let case = fixture["cases"]
        .as_array()
        .expect("fixture cases should be an array")
        .iter()
        .find(|case| case["input"] == input)
        .unwrap_or_else(|| panic!("fixture should contain input {input:?}"));
    case["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array")
        .iter()
        .take(expected_count)
        .map(|candidate| {
            candidate["text"]
                .as_str()
                .expect("candidate text should be a string")
                .to_owned()
        })
        .collect()
}

fn candidate_texts(schema_id: &str, input: &str) -> Vec<String> {
    let session_id = RimeCreateSession();
    assert_ne!(session_id, 0, "session should be created");

    let schema_id = CString::new(schema_id).expect("schema id should not contain NUL");
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in input.chars() {
        assert_eq!(
            RimeProcessKey(session_id, ch as i32, 0),
            TRUE,
            "{input:?} key {ch:?} should be accepted"
        );
    }

    let mut context = empty_context();
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let texts = if context.menu.num_candidates <= 0 || context.menu.candidates.is_null() {
        Vec::new()
    } else {
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                usize::try_from(context.menu.num_candidates)
                    .expect("candidate count should fit usize"),
            )
        };
        candidates
            .iter()
            .map(|candidate| {
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);
    texts
}

#[derive(Clone, Copy)]
enum ExpectedArtifactStatus {
    Available,
    NoRebuild,
}

impl ExpectedArtifactStatus {
    fn assert_for(
        self,
        reports: &[yune_rime_api::WorkspaceDictionaryRebuildReport],
        dictionary_id: &str,
    ) {
        match self {
            Self::Available => assert!(
                reports.iter().any(|report| {
                    report.dictionary_id == dictionary_id
                        && report.report.table != RimeDictArtifactStatus::MissingSourceAndCompiled
                        && report.report.prism != RimeDictArtifactStatus::MissingSourceAndCompiled
                        && report.report.reverse != RimeDictArtifactStatus::MissingSourceAndCompiled
                }),
                "{dictionary_id} should report available table/prism/reverse artifacts: {reports:?}"
            ),
            Self::NoRebuild => assert!(
                reports.iter().any(|report| {
                    report.dictionary_id == dictionary_id
                        && report.report.table == RimeDictArtifactStatus::ReusedFresh
                        && report.report.prism == RimeDictArtifactStatus::ReusedFresh
                        && report.report.reverse == RimeDictArtifactStatus::ReusedFresh
                }),
                "{dictionary_id} should reuse existing table/prism/reverse artifacts: {reports:?}"
            ),
        }
    }
}

struct PreparedRuntime {
    root: PathBuf,
    shared_c: CString,
    user_c: CString,
}

impl PreparedRuntime {
    fn new(name: &str, default_schema_id: &str) -> Self {
        let root = unique_temp_dir(name);
        let shared = root.join("shared");
        let user = root.join("user");
        copy_launch_schema_assets(&browser_app_schema_root(), &shared);
        fs::write(
            shared.join("default.yaml"),
            format!("config_version: '1.0'\nschema_list:\n  - schema: {default_schema_id}\n"),
        )
        .expect("scoped default schema list should be written");
        fs::create_dir_all(&user).expect("user data dir should be created");

        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("shared path should be valid");
        let user_c =
            CString::new(user.to_string_lossy().as_ref()).expect("user path should be valid");
        Self {
            root,
            shared_c,
            user_c,
        }
    }

    fn remove(self) {
        fs::remove_dir_all(self.root).expect("runtime temp dir should be removed");
    }
}

fn setup_deployer(runtime: &PreparedRuntime) {
    RimeCleanupAllSessions();
    let mut traits = empty_traits();
    traits.shared_data_dir = runtime.shared_c.as_ptr();
    traits.user_data_dir = runtime.user_c.as_ptr();
    unsafe { RimeDeployerInitialize(&traits) };
}

fn initialize_runtime(runtime: &PreparedRuntime) {
    let mut traits = empty_traits();
    traits.shared_data_dir = runtime.shared_c.as_ptr();
    traits.user_data_dir = runtime.user_c.as_ptr();
    unsafe { RimeInitialize(&traits) };
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

fn empty_context() -> RimeContext {
    RimeContext {
        data_size: mem::size_of::<RimeContext>() as i32,
        composition: yune_rime_api::RimeComposition {
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

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("yune-{name}-{}-{nanos}", std::process::id()))
}

fn browser_app_schema_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/yune-web/public/schema")
}

fn copy_launch_schema_assets(source_root: &Path, destination_root: &Path) {
    for entry in fs::read_dir(source_root).expect("source schema dir should be readable") {
        let entry = entry.expect("source schema entry should be readable");
        let source = entry.path();
        let destination = destination_root.join(entry.file_name());
        if source.is_dir() {
            copy_launch_schema_assets(&source, &destination);
            continue;
        }
        if entry.file_name() == "default.custom.yaml" {
            continue;
        }
        if entry.file_name().to_string_lossy().ends_with(".poet.bin") {
            continue;
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("destination parent should be created");
        }
        fs::copy(&source, destination).expect("schema source file should be copied");
    }
}
