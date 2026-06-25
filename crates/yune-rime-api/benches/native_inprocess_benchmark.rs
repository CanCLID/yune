use std::{
    collections::BTreeMap,
    ffi::{CStr, CString},
    fmt, fs, mem,
    os::raw::{c_char, c_int, c_void},
    path::{Path, PathBuf},
    ptr,
    sync::Arc,
    time::{Duration, Instant},
};

use libloading::Library;
use yune_core::{
    parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_table_bin_advanced_data, parse_rime_table_bin_dictionary, rime_dict_source_checksum,
    rime_table_bin_dict_file_checksum, CompactMarisaStringTable, CompactTableByteSource,
    CompactTableStore, RimeTableBinParseError,
};
use yune_rime_api::{
    Bool, RimeApi, RimeComposition, RimeContext, RimeMenu, RimeSessionId, RimeStatus, RimeTraits,
    FALSE, TRUE,
};

type RimeGetApi = unsafe extern "C" fn() -> *mut RimeApi;
type YuneM37MetricsEnable = unsafe extern "C" fn(Bool);
type YuneM37MetricsReset = unsafe extern "C" fn();
type YuneM37MetricsSnapshotJson = unsafe extern "C" fn() -> *mut c_char;
type YuneM37MetricsFreeString = unsafe extern "C" fn(*mut c_char);
type YuneStartupTraceBegin = unsafe extern "C" fn();
type YuneStartupTraceFinishJson = unsafe extern "C" fn() -> *mut c_char;

const DEFAULT_ITERATIONS: usize = 9;
const DEFAULT_SESSION_ITERATIONS: usize = 60;
const DEFAULT_KEY_ITERATIONS: usize = 80;
const KEY_WARMUPS: usize = 5;
const M37_METRIC_FIELDS: [&str; 44] = [
    "process_key_calls",
    "process_key_ns",
    "translator_calls",
    "translator_ns",
    "lookup_views_visited",
    "owned_candidates_materialized",
    "candidates_sorted",
    "candidate_sort_ns",
    "userdb_merge_ns",
    "filter_pipeline_ns",
    "ranker_pipeline_ns",
    "ai_merge_ns",
    "candidates_stored",
    "context_full_snapshot_candidates_cloned",
    "context_page_snapshot_candidates_cloned",
    "abi_get_context_calls",
    "abi_get_context_ns",
    "abi_candidates_exported",
    "abi_free_context_calls",
    "abi_free_context_ns",
    "candidate_request_bounded_calls",
    "candidate_request_unbounded_calls",
    "candidate_request_page_limit_total",
    "candidate_request_surplus_total",
    "bounded_iterator_calls",
    "bounded_iterator_limit_total",
    "bounded_iterator_selected_total",
    "bounded_iterator_full_count_total",
    "full_list_translation_calls",
    "full_list_fallback_count",
    "exact_lookup_calls",
    "exact_lookup_ns",
    "exact_lookup_candidates",
    "prefix_lookup_calls",
    "prefix_lookup_ns",
    "prefix_lookup_candidates",
    "heap_exact_lookup_calls",
    "heap_prefix_lookup_calls",
    "no_marisa_compact_exact_lookup_calls",
    "no_marisa_compact_prefix_lookup_calls",
    "rsmarisa_exact_lookup_calls",
    "rsmarisa_prefix_lookup_calls",
    "abi_c_string_allocations",
    "abi_c_string_bytes",
];

fn main() {
    let options = Options::parse();
    fs::create_dir_all(&options.output).expect("output directory should be created");
    let engine =
        LoadedRime::load(&options.dll).unwrap_or_else(|error| panic!("load rime failed: {error}"));
    if options.deploy_before_benchmark {
        deploy_workspace(&engine, &options);
    }
    let (samples, startup_traces) = run_benchmark(&engine, &options);
    write_samples(&options.output.join("samples.csv"), &samples);
    write_summary(&options.output.join("summary.csv"), &samples);
    write_m37_metrics(
        &options.output.join("m37_metrics.csv"),
        &samples
            .iter()
            .filter_map(|sample| sample.m37_metrics.as_ref())
            .cloned()
            .collect::<Vec<_>>(),
    );
    write_startup_session_trace(
        &options.output.join("startup_session_trace.csv"),
        &startup_traces,
    );
    write_product_path_status(&options.output.join("product_path_status.csv"), &options);
    write_raw_lookup_microbench(
        &options.output.join("raw_lookup_microbench.csv"),
        &options,
        &samples,
    );
    write_metadata(&options.output.join("metadata.txt"), &options);
    println!("engine={}", options.engine);
    println!("schema={}", options.schema);
    println!("track={}", options.track);
    println!("samples={}", samples.len());
    println!("summary={}", options.output.join("summary.csv").display());
}

#[derive(Debug)]
struct Options {
    engine: String,
    track: String,
    schema: String,
    dll: PathBuf,
    shared: PathBuf,
    user: PathBuf,
    build: PathBuf,
    output: PathBuf,
    inputs: Vec<String>,
    iterations: usize,
    session_iterations: usize,
    key_iterations: usize,
    deploy_before_benchmark: bool,
}

impl Options {
    fn parse() -> Self {
        let mut args = std::env::args().skip(1).collect::<Vec<_>>();
        assert!(
            !args.is_empty(),
            "native_inprocess_benchmark requires --engine, --dll, --shared, --user, --build, --output, and --schema"
        );
        Self {
            engine: take_arg(&mut args, "--engine"),
            track: take_arg_default(&mut args, "--track", "track-a"),
            schema: take_arg(&mut args, "--schema"),
            dll: PathBuf::from(take_arg(&mut args, "--dll")),
            shared: PathBuf::from(take_arg(&mut args, "--shared")),
            user: PathBuf::from(take_arg(&mut args, "--user")),
            build: PathBuf::from(take_arg(&mut args, "--build")),
            output: PathBuf::from(take_arg(&mut args, "--output")),
            inputs: take_arg_default(&mut args, "--inputs", "ni,hao,zhongguo")
                .split(',')
                .filter(|input| !input.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            iterations: take_arg_default(
                &mut args,
                "--iterations",
                &DEFAULT_ITERATIONS.to_string(),
            )
            .parse()
            .expect("iterations should be usize"),
            session_iterations: take_arg_default(
                &mut args,
                "--session-iterations",
                &DEFAULT_SESSION_ITERATIONS.to_string(),
            )
            .parse()
            .expect("session iterations should be usize"),
            key_iterations: take_arg_default(
                &mut args,
                "--key-iterations",
                &DEFAULT_KEY_ITERATIONS.to_string(),
            )
            .parse()
            .expect("key iterations should be usize"),
            deploy_before_benchmark: take_flag(&mut args, "--deploy-before-benchmark"),
        }
    }
}

fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return false;
    };
    args.remove(index);
    true
}

fn take_arg(args: &mut Vec<String>, name: &str) -> String {
    take_arg_default(args, name, "").tap(|value| {
        assert!(!value.is_empty(), "missing required argument {name}");
    })
}

fn take_arg_default(args: &mut Vec<String>, name: &str, default: &str) -> String {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return default.to_owned();
    };
    args.remove(index);
    assert!(index < args.len(), "missing value for {name}");
    args.remove(index)
}

trait Tap: Sized {
    fn tap(self, f: impl FnOnce(&Self)) -> Self {
        f(&self);
        self
    }
}

impl<T> Tap for T {}

struct LoadedRime {
    library: Library,
    api: *mut RimeApi,
}

impl LoadedRime {
    fn load(path: &Path) -> Result<Self, String> {
        let library = unsafe { Library::new(path) }
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let get_api: libloading::Symbol<RimeGetApi> = unsafe { library.get(b"rime_get_api\0") }
            .map_err(|error| format!("missing rime_get_api: {error}"))?;
        let api = unsafe { get_api() };
        if api.is_null() {
            return Err("rime_get_api returned null".to_owned());
        }
        Ok(Self { library, api })
    }

    fn api(&self) -> &RimeApi {
        unsafe { &*self.api }
    }

    fn m37_metrics(&self) -> Option<M37MetricsExports> {
        unsafe {
            Some(M37MetricsExports {
                enable: *self.library.get(b"yune_m37_metrics_enable\0").ok()?,
                reset: *self.library.get(b"yune_m37_metrics_reset\0").ok()?,
                snapshot_json: *self.library.get(b"yune_m37_metrics_snapshot_json\0").ok()?,
                free_string: *self.library.get(b"yune_m37_metrics_free_string\0").ok()?,
            })
        }
    }

    fn startup_trace(&self) -> Option<StartupTraceExports> {
        unsafe {
            Some(StartupTraceExports {
                begin: *self.library.get(b"yune_startup_trace_begin\0").ok()?,
                finish_json: *self.library.get(b"yune_startup_trace_finish_json\0").ok()?,
                free_string: *self.library.get(b"yune_m37_metrics_free_string\0").ok()?,
            })
        }
    }
}

#[derive(Clone, Copy)]
struct M37MetricsExports {
    enable: YuneM37MetricsEnable,
    reset: YuneM37MetricsReset,
    snapshot_json: YuneM37MetricsSnapshotJson,
    free_string: YuneM37MetricsFreeString,
}

impl M37MetricsExports {
    fn reset_and_enable(self) {
        unsafe {
            (self.reset)();
            (self.enable)(TRUE);
        }
    }

    fn disable_and_snapshot(self) -> BTreeMap<String, u64> {
        unsafe {
            (self.enable)(FALSE);
            let value = (self.snapshot_json)();
            if value.is_null() {
                return BTreeMap::new();
            }
            let text = CStr::from_ptr(value).to_string_lossy().into_owned();
            (self.free_string)(value);
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
                return BTreeMap::new();
            };
            M37_METRIC_FIELDS
                .iter()
                .map(|field| {
                    (
                        (*field).to_owned(),
                        json.get(field)
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0),
                    )
                })
                .collect()
        }
    }
}

#[derive(Clone, Copy)]
struct StartupTraceExports {
    begin: YuneStartupTraceBegin,
    finish_json: YuneStartupTraceFinishJson,
    free_string: YuneM37MetricsFreeString,
}

impl StartupTraceExports {
    fn begin(self) {
        unsafe { (self.begin)() };
    }

    fn finish(self) -> Vec<StartupTraceEvent> {
        unsafe {
            let value = (self.finish_json)();
            if value.is_null() {
                return Vec::new();
            }
            let text = CStr::from_ptr(value).to_string_lossy().into_owned();
            (self.free_string)(value);
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
                return Vec::new();
            };
            json.as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .filter_map(|(index, event)| {
                    Some(StartupTraceEvent {
                        event_index: index,
                        name: event.get("name")?.as_str()?.to_owned(),
                        micros: event.get("micros")?.as_u64()?,
                        working_set_before: event
                            .get("working_set_before")
                            .and_then(serde_json::Value::as_u64),
                        working_set_after: event
                            .get("working_set_after")
                            .and_then(serde_json::Value::as_u64),
                        peak_working_set_after: event
                            .get("peak_working_set_after")
                            .and_then(serde_json::Value::as_u64),
                    })
                })
                .collect()
        }
    }
}

#[derive(Clone, Debug)]
struct Sample {
    engine: String,
    track: String,
    schema: String,
    workload: &'static str,
    input: String,
    index: usize,
    operation_count: usize,
    total_us: f64,
    us_per_operation: f64,
    before_working_set_bytes: Option<u64>,
    after_ready_working_set_bytes: Option<u64>,
    after_finalize_working_set_bytes: Option<u64>,
    peak_working_set_bytes: Option<u64>,
    m37_metrics: Option<M37MetricSample>,
}

#[derive(Clone, Debug)]
struct M37MetricSample {
    engine: String,
    track: String,
    schema: String,
    workload: &'static str,
    input: String,
    index: usize,
    operation_count: usize,
    metrics: BTreeMap<String, u64>,
}

#[derive(Clone, Debug)]
struct StartupTraceSample {
    engine: String,
    track: String,
    schema: String,
    workload: &'static str,
    input: String,
    sample_index: usize,
    event: StartupTraceEvent,
}

#[derive(Clone, Debug)]
struct StartupTraceEvent {
    event_index: usize,
    name: String,
    micros: u64,
    working_set_before: Option<u64>,
    working_set_after: Option<u64>,
    peak_working_set_after: Option<u64>,
}

fn run_benchmark(engine: &LoadedRime, options: &Options) -> (Vec<Sample>, Vec<StartupTraceSample>) {
    let mut samples = Vec::new();
    let mut startup_traces = Vec::new();
    run_startup(engine, options, &mut samples, &mut startup_traces);
    run_session(engine, options, &mut samples, &mut startup_traces);
    for input in &options.inputs {
        run_key_workload(engine, options, input, &mut samples);
    }
    (samples, startup_traces)
}

fn run_startup(
    engine: &LoadedRime,
    options: &Options,
    samples: &mut Vec<Sample>,
    startup_traces: &mut Vec<StartupTraceSample>,
) {
    for index in 0..options.iterations {
        let api = engine.api();
        let traits = TraitsBundle::new(options);
        let before = current_memory_sample();
        let trace = engine.startup_trace();
        if let Some(trace) = trace {
            trace.begin();
        }
        let start = Instant::now();
        unsafe {
            require("setup", api.setup)(&traits.traits);
            require("initialize", api.initialize)(&traits.traits);
        }
        let create_session = require("create_session", api.create_session);
        let session_id = create_session();
        assert_ne!(session_id, 0, "create_session returned 0");
        select_schema(api, session_id, &options.schema);
        read_status(api, session_id);
        let ready = current_memory_sample();
        let elapsed = start.elapsed();
        assert_eq!(
            require("destroy_session", api.destroy_session)(session_id),
            TRUE
        );
        require("finalize", api.finalize)();
        let finalized = current_memory_sample();
        if let Some(trace) = trace {
            push_startup_trace_samples(
                startup_traces,
                options,
                "startup_warm_shared_assets_runtime_ready",
                "",
                index,
                trace.finish(),
            );
        }
        samples.push(Sample::new(
            options,
            "startup_warm_shared_assets_runtime_ready",
            "",
            index,
            1,
            elapsed,
            before,
            ready,
            Some(finalized),
        ));
    }
}

fn run_session(
    engine: &LoadedRime,
    options: &Options,
    samples: &mut Vec<Sample>,
    startup_traces: &mut Vec<StartupTraceSample>,
) {
    with_service(engine, options, |api| {
        for index in 0..options.session_iterations {
            let before = current_memory_sample();
            let trace = engine.startup_trace();
            if let Some(trace) = trace {
                trace.begin();
            }
            let start = Instant::now();
            let session_id = require("create_session", api.create_session)();
            assert_ne!(session_id, 0, "create_session returned 0");
            select_schema(api, session_id, &options.schema);
            assert_eq!(
                require("destroy_session", api.destroy_session)(session_id),
                TRUE
            );
            let elapsed = start.elapsed();
            let after = current_memory_sample();
            if let Some(trace) = trace {
                push_startup_trace_samples(
                    startup_traces,
                    options,
                    "session_create_select_destroy",
                    "",
                    index,
                    trace.finish(),
                );
            }
            samples.push(Sample::new(
                options,
                "session_create_select_destroy",
                "",
                index,
                1,
                elapsed,
                before,
                after,
                None,
            ));
        }
    });
}

fn run_key_workload(
    engine: &LoadedRime,
    options: &Options,
    input: &str,
    samples: &mut Vec<Sample>,
) {
    with_service(engine, options, |api| {
        let m37_metrics = engine.m37_metrics();
        let session_id = require("create_session", api.create_session)();
        assert_ne!(session_id, 0, "create_session returned 0");
        select_schema(api, session_id, &options.schema);
        set_default_options(api, session_id);
        for _ in 0..KEY_WARMUPS {
            process_input_with_context(api, session_id, input);
        }
        for index in 0..options.key_iterations {
            let before = current_memory_sample();
            if let Some(metrics) = m37_metrics {
                metrics.reset_and_enable();
            }
            let start = Instant::now();
            process_input_with_context(api, session_id, input);
            let elapsed = start.elapsed();
            let metrics = m37_metrics.map(M37MetricsExports::disable_and_snapshot);
            let after = current_memory_sample();
            let mut sample = Sample::new(
                options,
                "key_sequence_process_with_context",
                input,
                index,
                input.chars().count(),
                elapsed,
                before,
                after,
                None,
            );
            if let Some(metrics) = metrics {
                sample.m37_metrics = Some(M37MetricSample {
                    engine: options.engine.clone(),
                    track: options.track.clone(),
                    schema: options.schema.clone(),
                    workload: "key_sequence_process_with_context",
                    input: input.to_owned(),
                    index,
                    operation_count: input.chars().count(),
                    metrics,
                });
            }
            samples.push(sample);
        }
        assert_eq!(
            require("destroy_session", api.destroy_session)(session_id),
            TRUE
        );
    });
}

fn push_startup_trace_samples(
    startup_traces: &mut Vec<StartupTraceSample>,
    options: &Options,
    workload: &'static str,
    input: &str,
    sample_index: usize,
    events: Vec<StartupTraceEvent>,
) {
    startup_traces.extend(events.into_iter().map(|event| StartupTraceSample {
        engine: options.engine.clone(),
        track: options.track.clone(),
        schema: options.schema.clone(),
        workload,
        input: input.to_owned(),
        sample_index,
        event,
    }));
}

fn with_service(engine: &LoadedRime, options: &Options, action: impl FnOnce(&RimeApi)) {
    let api = engine.api();
    let traits = TraitsBundle::new(options);
    unsafe {
        require("setup", api.setup)(&traits.traits);
        require("initialize", api.initialize)(&traits.traits);
    }
    action(api);
    require("finalize", api.finalize)();
}

fn deploy_workspace(engine: &LoadedRime, options: &Options) {
    let api = engine.api();
    let traits = TraitsBundle::new(options);
    unsafe {
        require("deployer_initialize", api.deployer_initialize)(&traits.traits);
    }
    assert_eq!(require("deploy", api.deploy)(), TRUE);
    let schema_file =
        CString::new(format!("{}.schema.yaml", options.schema)).expect("schema file is valid");
    assert_eq!(
        require("deploy_schema", api.deploy_schema)(schema_file.as_ptr()),
        TRUE
    );
    let workspace_update =
        CString::new(format!("workspace_update:{}", options.schema)).expect("task name is valid");
    assert_eq!(
        require("run_task", api.run_task)(workspace_update.as_ptr()),
        TRUE
    );
    require("finalize", api.finalize)();
}

struct TraitsBundle {
    _shared: CString,
    _user: CString,
    _build: CString,
    _distribution_name: CString,
    _distribution_code_name: CString,
    _distribution_version: CString,
    _app_name: CString,
    _modules: CString,
    _module_ptrs: Box<[*const i8]>,
    _log_dir: CString,
    traits: RimeTraits,
}

impl TraitsBundle {
    fn new(options: &Options) -> Self {
        let shared = cstring_path(&options.shared);
        let user = cstring_path(&options.user);
        let build = cstring_path(&options.build);
        let distribution_name = CString::new(options.engine.as_str()).expect("valid engine name");
        let distribution_code_name =
            CString::new(options.engine.as_str()).expect("valid engine code name");
        let distribution_version = CString::new("m36-native-benchmark").expect("valid version");
        let app_name = CString::new("yune.m36.native_inprocess_benchmark").expect("valid app");
        let modules = CString::new("default").expect("valid module");
        let module_ptrs = vec![modules.as_ptr(), ptr::null()].into_boxed_slice();
        let log_dir = CString::new("").expect("valid log dir");
        let traits = RimeTraits {
            data_size: (mem::size_of::<RimeTraits>() - mem::size_of::<c_int>()) as c_int,
            shared_data_dir: shared.as_ptr(),
            user_data_dir: user.as_ptr(),
            distribution_name: distribution_name.as_ptr(),
            distribution_code_name: distribution_code_name.as_ptr(),
            distribution_version: distribution_version.as_ptr(),
            app_name: app_name.as_ptr(),
            modules: module_ptrs.as_ptr(),
            min_log_level: 2,
            log_dir: log_dir.as_ptr(),
            prebuilt_data_dir: build.as_ptr(),
            staging_dir: build.as_ptr(),
        };
        Self {
            _shared: shared,
            _user: user,
            _build: build,
            _distribution_name: distribution_name,
            _distribution_code_name: distribution_code_name,
            _distribution_version: distribution_version,
            _app_name: app_name,
            _modules: modules,
            _module_ptrs: module_ptrs,
            _log_dir: log_dir,
            traits,
        }
    }
}

fn cstring_path(path: &Path) -> CString {
    CString::new(path.to_string_lossy().as_bytes()).expect("path should not contain NUL")
}

fn select_schema(api: &RimeApi, session_id: RimeSessionId, schema: &str) {
    let schema = CString::new(schema).expect("schema id should be valid");
    let select_schema = require("select_schema", api.select_schema);
    assert_eq!(unsafe { select_schema(session_id, schema.as_ptr()) }, TRUE);
}

fn set_default_options(api: &RimeApi, session_id: RimeSessionId) {
    let set_option = require("set_option", api.set_option);
    for option in ["ascii_mode", "full_shape", "ascii_punct", "zh_hans"] {
        let option = CString::new(option).expect("option should be valid");
        unsafe { set_option(session_id, option.as_ptr(), 0) };
    }
}

fn process_input_with_context(api: &RimeApi, session_id: RimeSessionId, input: &str) {
    require("clear_composition", api.clear_composition)(session_id);
    let process_key = require("process_key", api.process_key);
    for ch in input.chars() {
        assert_ne!(
            process_key(session_id, ch as c_int, 0),
            0,
            "process_key failed for {input}"
        );
    }
    read_context(api, session_id);
}

fn read_context(api: &RimeApi, session_id: RimeSessionId) {
    let mut context = RimeContext {
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
            is_last_page: 0,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        },
        commit_text_preview: ptr::null_mut(),
        select_labels: ptr::null_mut(),
    };
    assert_eq!(
        unsafe { require("get_context", api.get_context)(session_id, &mut context) },
        TRUE
    );
    assert_eq!(
        unsafe { require("free_context", api.free_context)(&mut context) },
        TRUE
    );
}

fn read_status(api: &RimeApi, session_id: RimeSessionId) {
    let mut status = RimeStatus {
        data_size: (mem::size_of::<RimeStatus>() - mem::size_of::<c_int>()) as c_int,
        schema_id: ptr::null_mut(),
        schema_name: ptr::null_mut(),
        is_disabled: 0,
        is_composing: 0,
        is_ascii_mode: 0,
        is_full_shape: 0,
        is_simplified: 0,
        is_traditional: 0,
        is_ascii_punct: 0,
    };
    assert_eq!(
        unsafe { require("get_status", api.get_status)(session_id, &mut status) },
        TRUE
    );
    assert_eq!(
        unsafe { require("free_status", api.free_status)(&mut status) },
        TRUE
    );
}

impl Sample {
    #[allow(clippy::too_many_arguments)]
    fn new(
        options: &Options,
        workload: &'static str,
        input: &str,
        sample_index: usize,
        operation_count: usize,
        elapsed: Duration,
        before: MemorySample,
        after_ready: MemorySample,
        after_finalize: Option<MemorySample>,
    ) -> Self {
        let total_us = duration_micros(elapsed);
        Self {
            engine: options.engine.clone(),
            track: options.track.clone(),
            schema: options.schema.clone(),
            workload,
            input: input.to_owned(),
            index: sample_index,
            operation_count,
            total_us,
            us_per_operation: total_us / operation_count as f64,
            before_working_set_bytes: before.working_set_bytes,
            after_ready_working_set_bytes: after_ready.working_set_bytes,
            after_finalize_working_set_bytes: after_finalize
                .and_then(|sample| sample.working_set_bytes),
            peak_working_set_bytes: max_optional(
                after_ready.peak_working_set_bytes,
                after_finalize.and_then(|sample| sample.peak_working_set_bytes),
            ),
            m37_metrics: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct MemorySample {
    working_set_bytes: Option<u64>,
    peak_working_set_bytes: Option<u64>,
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
        return MemorySample::default();
    }
    MemorySample {
        working_set_bytes: Some(counters.working_set_size as u64),
        peak_working_set_bytes: Some(counters.peak_working_set_size as u64),
    }
}

#[cfg(not(windows))]
fn current_memory_sample() -> MemorySample {
    MemorySample::default()
}

fn write_samples(path: &PathBuf, samples: &[Sample]) {
    let mut output = String::from("engine,track,schema_id,workload,input,sample_index,operation_count,total_us,us_per_operation,before_working_set_bytes,after_ready_working_set_bytes,after_finalize_working_set_bytes,peak_working_set_bytes\n");
    for sample in samples {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{:.3},{:.3},{},{},{},{}\n",
            csv(&sample.engine),
            csv(&sample.track),
            csv(&sample.schema),
            csv(sample.workload),
            csv(&sample.input),
            sample.index,
            sample.operation_count,
            sample.total_us,
            sample.us_per_operation,
            optional_u64(sample.before_working_set_bytes),
            optional_u64(sample.after_ready_working_set_bytes),
            optional_u64(sample.after_finalize_working_set_bytes),
            optional_u64(sample.peak_working_set_bytes)
        ));
    }
    fs::write(path, output).expect("samples CSV should be written");
}

fn write_summary(path: &PathBuf, samples: &[Sample]) {
    let mut groups = BTreeMap::<(&str, &str, &str, &str, &str), Vec<&Sample>>::new();
    for sample in samples {
        groups
            .entry((
                sample.engine.as_str(),
                sample.track.as_str(),
                sample.schema.as_str(),
                sample.workload,
                sample.input.as_str(),
            ))
            .or_default()
            .push(sample);
    }
    let mut output = String::from("engine,track,schema_id,workload,input,samples,operations,median_us,p95_us,p99_us,max_us,median_working_set_bytes,max_peak_working_set_bytes\n");
    for ((engine, track, schema, workload, input), samples) in groups {
        let mut latencies = samples
            .iter()
            .map(|sample| sample.us_per_operation)
            .collect::<Vec<_>>();
        latencies.sort_by(f64::total_cmp);
        let mut working_sets = samples
            .iter()
            .filter_map(|sample| sample.after_ready_working_set_bytes)
            .collect::<Vec<_>>();
        working_sets.sort_unstable();
        let peak = samples
            .iter()
            .filter_map(|sample| sample.peak_working_set_bytes)
            .max();
        let operations = samples
            .iter()
            .map(|sample| sample.operation_count)
            .sum::<usize>();
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{},{}\n",
            csv(engine),
            csv(track),
            csv(schema),
            csv(workload),
            csv(input),
            samples.len(),
            operations,
            percentile(&latencies, 0.50),
            percentile(&latencies, 0.95),
            percentile(&latencies, 0.99),
            latencies.last().copied().unwrap_or(0.0),
            working_sets
                .get(working_sets.len().saturating_sub(1) / 2)
                .map_or_else(|| "unavailable".to_owned(), ToString::to_string),
            optional_u64(peak)
        ));
    }
    fs::write(path, output).expect("summary CSV should be written");
}

fn write_m37_metrics(path: &PathBuf, samples: &[M37MetricSample]) {
    let mut header =
        String::from("engine,track,schema_id,workload,input,sample_index,operation_count");
    for field in M37_METRIC_FIELDS {
        header.push(',');
        header.push_str(field);
    }
    header.push('\n');
    let mut output = header;
    for sample in samples {
        output.push_str(&format!(
            "{},{},{},{},{},{},{}",
            csv(&sample.engine),
            csv(&sample.track),
            csv(&sample.schema),
            csv(sample.workload),
            csv(&sample.input),
            sample.index,
            sample.operation_count,
        ));
        for field in M37_METRIC_FIELDS {
            output.push(',');
            output.push_str(
                &sample
                    .metrics
                    .get(field)
                    .copied()
                    .unwrap_or_default()
                    .to_string(),
            );
        }
        output.push('\n');
    }
    fs::write(path, output).expect("M37 metrics CSV should be written");
}

fn write_startup_session_trace(path: &PathBuf, samples: &[StartupTraceSample]) {
    let mut output = String::from("engine,track,schema_id,workload,input,sample_index,event_index,name,micros,working_set_before,working_set_after,peak_working_set_after\n");
    for sample in samples {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv(&sample.engine),
            csv(&sample.track),
            csv(&sample.schema),
            csv(sample.workload),
            csv(&sample.input),
            sample.sample_index,
            sample.event.event_index,
            csv(&sample.event.name),
            sample.event.micros,
            optional_u64(sample.event.working_set_before),
            optional_u64(sample.event.working_set_after),
            optional_u64(sample.event.peak_working_set_after),
        ));
    }
    fs::write(path, output).expect("startup/session trace CSV should be written");
}

fn write_metadata(path: &PathBuf, options: &Options) {
    let metadata = [
        format!("engine={}", options.engine),
        format!("track={}", options.track),
        format!("schema={}", options.schema),
        format!("dll={}", options.dll.display()),
        format!("shared={}", options.shared.display()),
        format!("user={}", options.user.display()),
        format!("build={}", options.build.display()),
        format!("inputs={}", options.inputs.join(",")),
        format!("iterations={}", options.iterations),
        format!("session_iterations={}", options.session_iterations),
        format!("key_iterations={}", options.key_iterations),
        format!(
            "deploy_before_benchmark={}",
            options.deploy_before_benchmark
        ),
        "managed_runtime=false".to_owned(),
    ]
    .join("\n");
    fs::write(path, format!("{metadata}\n")).expect("metadata should be written");
}

fn write_product_path_status(path: &PathBuf, options: &Options) {
    let mut output = String::from("engine,track,schema_id,dictionary_id,prism_id,source_path,table_path,prism_path,reverse_path,source_checksum,table_checksum,checksum_status,table_parse,prism_parse,reverse_parse,compiled_ready,selected_storage,table_format,table_mapping_mode,prism_mapping_mode,source_fallback,byte_source_len,stored_entries,table_heap_mirror_bytes,prism_heap_mirror_bytes,rsmarisa_probe_path,rsmarisa_status,rsmarisa_mapping_mode,rsmarisa_num_tries,rsmarisa_num_keys,rsmarisa_sample_key\n");
    for (dictionary_id, prism_id) in status_dictionary_requests(options) {
        let status = ProductPathStatus::inspect(options, dictionary_id, prism_id);
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv(&options.engine),
            csv(&options.track),
            csv(&options.schema),
            csv(status.dictionary_id),
            csv(status.prism_id),
            csv(&display_optional_path(status.source_path.as_ref())),
            csv(&display_optional_path(status.table_path.as_ref())),
            csv(&display_optional_path(status.prism_path.as_ref())),
            csv(&display_optional_path(status.reverse_path.as_ref())),
            status.source_checksum.map_or_else(
                || "unavailable".to_owned(),
                |value| format!("{value:#010x}")
            ),
            status.table_checksum.map_or_else(
                || "unavailable".to_owned(),
                |value| format!("{value:#010x}")
            ),
            csv(&status.checksum_status),
            csv(&status.table_parse),
            csv(&status.prism_parse),
            csv(&status.reverse_parse),
            status.compiled_ready,
            csv(&status.selected_storage),
            csv(&status.table_format),
            csv(&status.table_mapping_mode),
            csv(&status.prism_mapping_mode),
            status.source_fallback,
            status.byte_source_len,
            status.stored_entries,
            status.table_heap_mirror_bytes,
            status.prism_heap_mirror_bytes,
            csv(&display_optional_path(status.rsmarisa_probe_path.as_ref())),
            csv(&status.rsmarisa_status),
            csv(&status.rsmarisa_mapping_mode),
            status.rsmarisa_num_tries
                .map_or_else(|| "unavailable".to_owned(), |value| value.to_string()),
            status.rsmarisa_num_keys
                .map_or_else(|| "unavailable".to_owned(), |value| value.to_string()),
            csv(&status.rsmarisa_sample_key)
        ));
    }
    fs::write(path, output).expect("product path status CSV should be written");
}

fn status_dictionary_requests(options: &Options) -> Vec<(&'static str, &'static str)> {
    if options.engine != "yune" {
        return Vec::new();
    }
    if options.track == "track-a-comparison" && options.schema == "luna_pinyin" {
        return vec![("luna_pinyin", "luna_pinyin")];
    }
    if options.track == "track-b-product" {
        return product_dictionary_requests(&options.schema);
    }
    Vec::new()
}

fn product_dictionary_requests(schema: &str) -> Vec<(&'static str, &'static str)> {
    if schema == "jyut6ping3_mobile" || schema == "jyut6ping3" {
        vec![
            ("jyut6ping3", "jyut6ping3_mobile"),
            ("jyut6ping3_scolar", "jyut6ping3_scolar"),
        ]
    } else {
        Vec::new()
    }
}

struct ProductPathStatus<'a> {
    dictionary_id: &'a str,
    prism_id: &'a str,
    source_path: Option<PathBuf>,
    table_path: Option<PathBuf>,
    prism_path: Option<PathBuf>,
    reverse_path: Option<PathBuf>,
    source_checksum: Option<u32>,
    table_checksum: Option<u32>,
    checksum_status: String,
    table_parse: String,
    prism_parse: String,
    reverse_parse: String,
    compiled_ready: bool,
    selected_storage: String,
    table_format: String,
    table_mapping_mode: String,
    prism_mapping_mode: String,
    source_fallback: bool,
    byte_source_len: usize,
    stored_entries: usize,
    table_heap_mirror_bytes: usize,
    prism_heap_mirror_bytes: usize,
    rsmarisa_probe_path: Option<PathBuf>,
    rsmarisa_status: String,
    rsmarisa_mapping_mode: String,
    rsmarisa_num_tries: Option<usize>,
    rsmarisa_num_keys: Option<usize>,
    rsmarisa_sample_key: String,
}

impl<'a> ProductPathStatus<'a> {
    fn inspect(options: &Options, dictionary_id: &'a str, prism_id: &'a str) -> Self {
        let source_path = selected_data_path(options, &format!("{dictionary_id}.dict.yaml"));
        let table_path = selected_data_path(options, &format!("{dictionary_id}.table.bin"));
        let prism_path = selected_data_path(options, &format!("{prism_id}.prism.bin"));
        let reverse_path = selected_data_path(options, &format!("{dictionary_id}.reverse.bin"));

        let source = source_path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok());
        let table_bytes = table_path.as_ref().and_then(|path| fs::read(path).ok());
        let prism_bytes = prism_path.as_ref().and_then(|path| fs::read(path).ok());
        let reverse_bytes = reverse_path.as_ref().and_then(|path| fs::read(path).ok());

        let source_checksum = source
            .as_ref()
            .map(|source| rime_dict_source_checksum(0, [source.as_bytes()], None));
        let table_checksum = table_bytes
            .as_ref()
            .and_then(rime_table_bin_dict_file_checksum);
        let table_has_marisa = table_bytes
            .as_ref()
            .is_some_and(|bytes| string_table_range(bytes).is_some());
        let accepts_upstream_marisa_checksum = options.engine == "yune"
            && options.track == "track-a-comparison"
            && dictionary_id == "luna_pinyin"
            && table_has_marisa
            && source_checksum.is_some()
            && table_checksum.is_some();
        let checksum_status = match (source_checksum, table_checksum) {
            (Some(source), Some(table)) if source == table => "fresh",
            (Some(_), Some(_)) if accepts_upstream_marisa_checksum => {
                "accepted_upstream_marisa_import_checksum"
            }
            (Some(_), Some(_)) => "stale",
            (None, _) => "missing_source",
            (_, None) => "missing_table_checksum",
        }
        .to_owned();
        let table_parse = table_bytes
            .as_ref()
            .map(|bytes| parse_status(parse_rime_table_bin_dictionary(bytes)))
            .unwrap_or_else(|| "missing".to_owned());
        let table_format = table_bytes
            .as_ref()
            .map_or_else(|| "missing".to_owned(), |bytes| table_format_status(bytes));
        let prism_parse = prism_bytes
            .as_ref()
            .map(|bytes| parse_status(parse_rime_prism_bin_payload(bytes)))
            .unwrap_or_else(|| "missing".to_owned());
        let reverse_parse = reverse_bytes
            .as_ref()
            .map(|bytes| parse_status(parse_rime_reverse_bin_dictionary(bytes)))
            .unwrap_or_else(|| "missing".to_owned());
        let rsmarisa = inspect_rsmarisa_string_table(options, dictionary_id);
        let upstream_marisa_luna = options.engine == "yune"
            && options.track == "track-a-comparison"
            && dictionary_id == "luna_pinyin"
            && table_has_marisa;
        let checksum_accepted = checksum_status == "fresh"
            || checksum_status == "accepted_upstream_marisa_import_checksum";
        let table_ready = if upstream_marisa_luna {
            rsmarisa.status == "ok"
        } else {
            table_parse == "ok"
        };
        let reverse_ready = reverse_parse == "ok"
            || (upstream_marisa_luna && reverse_parse.contains("UnsupportedSection"));
        let compiled_ready =
            checksum_accepted && table_ready && prism_parse == "ok" && reverse_ready;
        let (
            selected_storage,
            table_mapping_mode,
            prism_mapping_mode,
            byte_source_len,
            stored_entries,
            table_heap_mirror_bytes,
            prism_heap_mirror_bytes,
        ) = if compiled_ready {
            (
                if upstream_marisa_luna {
                    "rsmarisa_byte_backed".to_owned()
                } else {
                    "byte_backed".to_owned()
                },
                "mmap".to_owned(),
                "mmap".to_owned(),
                table_bytes.as_ref().map_or(0, Vec::len),
                table_bytes
                    .as_ref()
                    .map_or(0, |bytes| table_entry_count(bytes).unwrap_or(0)),
                0,
                0,
            )
        } else {
            (
                "unavailable".to_owned(),
                "unavailable".to_owned(),
                "unavailable".to_owned(),
                0,
                0,
                0,
                0,
            )
        };

        Self {
            dictionary_id,
            prism_id,
            source_path,
            table_path,
            prism_path,
            reverse_path,
            source_checksum,
            table_checksum,
            checksum_status,
            table_parse,
            prism_parse,
            reverse_parse,
            compiled_ready,
            selected_storage,
            table_format,
            table_mapping_mode,
            prism_mapping_mode,
            source_fallback: !compiled_ready,
            byte_source_len,
            stored_entries,
            table_heap_mirror_bytes,
            prism_heap_mirror_bytes,
            rsmarisa_probe_path: rsmarisa.payload_path,
            rsmarisa_status: rsmarisa.status,
            rsmarisa_mapping_mode: rsmarisa.mapping_mode,
            rsmarisa_num_tries: rsmarisa.num_tries,
            rsmarisa_num_keys: rsmarisa.num_keys,
            rsmarisa_sample_key: rsmarisa.sample_key,
        }
    }
}

struct RsmarisaProbeStatus {
    payload_path: Option<PathBuf>,
    status: String,
    mapping_mode: String,
    num_tries: Option<usize>,
    num_keys: Option<usize>,
    sample_key: String,
}

fn inspect_rsmarisa_string_table(options: &Options, dictionary_id: &str) -> RsmarisaProbeStatus {
    let Some(table_path) = selected_data_path(options, &format!("{dictionary_id}.table.bin"))
    else {
        return RsmarisaProbeStatus {
            payload_path: None,
            status: "missing_table".to_owned(),
            mapping_mode: "unavailable".to_owned(),
            num_tries: None,
            num_keys: None,
            sample_key: String::new(),
        };
    };
    let Some(table_bytes) = fs::read(&table_path).ok() else {
        return RsmarisaProbeStatus {
            payload_path: None,
            status: "table_read_failed".to_owned(),
            mapping_mode: "unavailable".to_owned(),
            num_tries: None,
            num_keys: None,
            sample_key: String::new(),
        };
    };
    let Some((offset, size)) = string_table_range(&table_bytes) else {
        return RsmarisaProbeStatus {
            payload_path: None,
            status: "missing_string_table".to_owned(),
            mapping_mode: "unavailable".to_owned(),
            num_tries: None,
            num_keys: None,
            sample_key: String::new(),
        };
    };
    let Some(payload) = table_bytes.get(offset..offset + size) else {
        return RsmarisaProbeStatus {
            payload_path: None,
            status: "string_table_out_of_bounds".to_owned(),
            mapping_mode: "unavailable".to_owned(),
            num_tries: None,
            num_keys: None,
            sample_key: String::new(),
        };
    };

    let payload_path = options
        .output
        .join(format!("rsmarisa-{dictionary_id}-string-table.marisa"));
    if let Err(error) = fs::write(&payload_path, payload) {
        return RsmarisaProbeStatus {
            payload_path: Some(payload_path),
            status: format!("payload_write_failed:{error}"),
            mapping_mode: "unavailable".to_owned(),
            num_tries: None,
            num_keys: None,
            sample_key: String::new(),
        };
    }
    let Some(payload_path_str) = payload_path.to_str() else {
        return RsmarisaProbeStatus {
            payload_path: Some(payload_path),
            status: "payload_path_not_utf8".to_owned(),
            mapping_mode: "unavailable".to_owned(),
            num_tries: None,
            num_keys: None,
            sample_key: String::new(),
        };
    };
    let mut trie = rsmarisa::Trie::new();
    if let Err(error) = trie.mmap(payload_path_str) {
        return RsmarisaProbeStatus {
            payload_path: Some(payload_path),
            status: format!("mmap_failed:{error}"),
            mapping_mode: "mmap_failed".to_owned(),
            num_tries: None,
            num_keys: None,
            sample_key: String::new(),
        };
    }
    let num_tries = trie.num_tries();
    let num_keys = trie.num_keys();
    let sample_key = if num_keys == 0 {
        String::new()
    } else {
        let mut agent = rsmarisa::Agent::new();
        agent.set_query_id(0);
        trie.reverse_lookup(&mut agent);
        agent.key().as_str().to_owned()
    };
    RsmarisaProbeStatus {
        payload_path: Some(payload_path),
        status: "ok".to_owned(),
        mapping_mode: "mmap".to_owned(),
        num_tries: Some(num_tries),
        num_keys: Some(num_keys),
        sample_key,
    }
}

fn table_format_status(bytes: &[u8]) -> String {
    match string_table_range(bytes) {
        Some((_, size)) => format!("rime_marisa_string_table:{size}"),
        None if parse_rime_table_bin_dictionary(bytes).is_ok() => {
            "yune_no_marisa_compact".to_owned()
        }
        None => "unknown".to_owned(),
    }
}

fn table_entry_count(bytes: &[u8]) -> Option<usize> {
    read_u32_at(bytes, 40).and_then(|value| usize::try_from(value).ok())
}

fn string_table_range(bytes: &[u8]) -> Option<(usize, usize)> {
    let offset = read_offset_ptr_at(bytes, 60)?;
    let size = usize::try_from(read_u32_at(bytes, 64)?).ok()?;
    (size != 0).then_some((offset, size))
}

fn read_offset_ptr_at(bytes: &[u8], field_offset: usize) -> Option<usize> {
    let raw = i32::from_le_bytes(bytes.get(field_offset..field_offset + 4)?.try_into().ok()?);
    if raw == 0 {
        return None;
    }
    field_offset.checked_add_signed(raw as isize)
}

fn read_u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn selected_data_path(options: &Options, file_name: &str) -> Option<PathBuf> {
    [
        options.build.join(file_name),
        options.shared.join(file_name),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn write_raw_lookup_microbench(path: &PathBuf, options: &Options, samples: &[Sample]) {
    let mut output = String::from("engine,track,schema_id,input,iterations,table_path,prism_path,selected_storage,table_mapping_mode,prism_mapping_mode,raw_prism_median_us,raw_prism_code_count,raw_table_median_us,raw_table_lookup_codes,raw_table_candidate_count,translator_median_us,context_export_median_us,rsmarisa_exact_lookup_calls_per_op,rsmarisa_prefix_lookup_calls_per_op,owned_candidates_per_op,context_page_candidates_per_op,abi_c_string_allocations_per_op,abi_c_string_bytes_per_op\n");
    if options.engine != "yune"
        || options.track != "track-a-comparison"
        || options.schema != "luna_pinyin"
    {
        fs::write(path, output).expect("raw lookup microbench CSV should be written");
        return;
    }

    match RawLookupFixture::load(options) {
        Ok(fixture) => {
            for input in &options.inputs {
                let row = run_raw_lookup_microbench_row(options, samples, &fixture, input);
                output.push_str(&row);
            }
        }
        Err(error) => {
            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                csv(&options.engine),
                csv(&options.track),
                csv(&options.schema),
                "unavailable",
                0,
                "missing",
                "missing",
                "unavailable",
                "unavailable",
                "unavailable",
                "unavailable",
                0,
                "unavailable",
                csv(&error),
                0,
                "unavailable",
                "unavailable",
                "unavailable",
                "unavailable",
                "unavailable",
                "unavailable",
                "unavailable",
                "unavailable",
            ));
        }
    }

    fs::write(path, output).expect("raw lookup microbench CSV should be written");
}

struct RawLookupFixture {
    table_path: PathBuf,
    prism_path: PathBuf,
    store: CompactTableStore,
    prism: yune_core::RimePrismBinPayload,
    selected_storage: &'static str,
    table_mapping_mode: &'static str,
    prism_mapping_mode: &'static str,
}

impl RawLookupFixture {
    fn load(options: &Options) -> Result<Self, String> {
        let table_path = selected_data_path(options, "luna_pinyin.table.bin")
            .ok_or_else(|| "missing luna_pinyin.table.bin".to_owned())?;
        let prism_path = selected_data_path(options, "luna_pinyin.prism.bin")
            .ok_or_else(|| "missing luna_pinyin.prism.bin".to_owned())?;
        let table_source = BenchMappedTableBytes::load(&table_path)?;
        let advanced = parse_rime_table_bin_advanced_data(table_source.bytes())
            .map_err(|error| format!("table advanced parse failed: {error:?}"))?;
        let store = CompactTableStore::from_table_bin_byte_source(Arc::new(table_source), advanced)
            .map_err(|error| format!("compact table parse failed: {error:?}"))?;
        let prism_bytes = BenchMappedDataBytes::load(&prism_path, "prism")?;
        let prism = parse_rime_prism_bin_payload(prism_bytes.bytes())
            .map_err(|error| format!("prism parse failed: {error:?}"))?;
        let selected_storage = store.storage_label();
        let table_mapping_mode = store.mapping_mode();
        Ok(Self {
            table_path,
            prism_path,
            store,
            prism,
            selected_storage,
            table_mapping_mode,
            prism_mapping_mode: prism_bytes.mapping_mode(),
        })
    }
}

fn run_raw_lookup_microbench_row(
    options: &Options,
    samples: &[Sample],
    fixture: &RawLookupFixture,
    input: &str,
) -> String {
    let iterations = options.key_iterations.max(1);
    let lookup_codes = raw_lookup_codes(fixture, input);
    let mut prism_latencies = Vec::with_capacity(iterations);
    let mut table_latencies = Vec::with_capacity(iterations);
    let mut prism_code_count = 0usize;
    let mut table_candidate_count = 0usize;

    for _ in 0..iterations {
        let prism_start = Instant::now();
        let prism_codes = fixture
            .prism
            .lookup_canonical_codes(input, fixture.store.syllabary_codes());
        prism_latencies.push(duration_micros(prism_start.elapsed()));
        prism_code_count = prism_codes.len();

        let table_start = Instant::now();
        table_candidate_count = lookup_codes
            .iter()
            .map(|code| fixture.store.exact_candidate_count(code))
            .sum();
        table_latencies.push(duration_micros(table_start.elapsed()));
    }
    prism_latencies.sort_by(f64::total_cmp);
    table_latencies.sort_by(f64::total_cmp);

    let key_metrics = median_key_metrics(samples, input);
    format!(
        "{},{},{},{},{},{},{},{},{},{},{:.3},{},{:.3},{},{},{},{},{},{},{},{},{},{}\n",
        csv(&options.engine),
        csv(&options.track),
        csv(&options.schema),
        csv(input),
        iterations,
        csv(&fixture.table_path.display().to_string()),
        csv(&fixture.prism_path.display().to_string()),
        csv(fixture.selected_storage),
        csv(fixture.table_mapping_mode),
        csv(fixture.prism_mapping_mode),
        percentile(&prism_latencies, 0.50),
        prism_code_count,
        percentile(&table_latencies, 0.50),
        csv(&lookup_codes.join("|")),
        table_candidate_count,
        optional_f64(key_metrics.translator_median_us),
        optional_f64(key_metrics.context_export_median_us),
        optional_f64(key_metrics.rsmarisa_exact_lookup_calls_per_op),
        optional_f64(key_metrics.rsmarisa_prefix_lookup_calls_per_op),
        optional_f64(key_metrics.owned_candidates_per_op),
        optional_f64(key_metrics.context_page_candidates_per_op),
        optional_f64(key_metrics.abi_c_string_allocations_per_op),
        optional_f64(key_metrics.abi_c_string_bytes_per_op),
    )
}

fn raw_lookup_codes(fixture: &RawLookupFixture, input: &str) -> Vec<String> {
    let mut codes = vec![input.to_owned()];
    for lookup in fixture
        .prism
        .lookup_canonical_codes(input, fixture.store.syllabary_codes())
    {
        if !codes.iter().any(|code| code == lookup.code)
            && fixture.store.exact_candidate_count(lookup.code) > 0
        {
            codes.push(lookup.code.to_owned());
        }
    }
    codes
}

#[derive(Default)]
struct KeyOwnerMetrics {
    translator_median_us: Option<f64>,
    context_export_median_us: Option<f64>,
    rsmarisa_exact_lookup_calls_per_op: Option<f64>,
    rsmarisa_prefix_lookup_calls_per_op: Option<f64>,
    owned_candidates_per_op: Option<f64>,
    context_page_candidates_per_op: Option<f64>,
    abi_c_string_allocations_per_op: Option<f64>,
    abi_c_string_bytes_per_op: Option<f64>,
}

fn median_key_metrics(samples: &[Sample], input: &str) -> KeyOwnerMetrics {
    fn metric_per_operation(
        samples: &[Sample],
        input: &str,
        field: &str,
        denominator_field: Option<&str>,
    ) -> Option<f64> {
        let mut values = samples
            .iter()
            .filter(|sample| {
                sample.workload == "key_sequence_process_with_context" && sample.input == input
            })
            .filter_map(|sample| {
                let metrics = sample.m37_metrics.as_ref()?;
                let value = *metrics.metrics.get(field)? as f64;
                let denominator = denominator_field
                    .and_then(|name| metrics.metrics.get(name).copied())
                    .unwrap_or(sample.operation_count as u64);
                (denominator > 0).then_some(value / denominator as f64)
            })
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        (!values.is_empty()).then(|| percentile(&values, 0.50))
    }

    KeyOwnerMetrics {
        translator_median_us: metric_per_operation(samples, input, "translator_ns", None)
            .map(|value| value / 1000.0),
        context_export_median_us: metric_per_operation(
            samples,
            input,
            "abi_get_context_ns",
            Some("abi_get_context_calls"),
        )
        .map(|value| value / 1000.0),
        rsmarisa_exact_lookup_calls_per_op: metric_per_operation(
            samples,
            input,
            "rsmarisa_exact_lookup_calls",
            None,
        ),
        rsmarisa_prefix_lookup_calls_per_op: metric_per_operation(
            samples,
            input,
            "rsmarisa_prefix_lookup_calls",
            None,
        ),
        owned_candidates_per_op: metric_per_operation(
            samples,
            input,
            "owned_candidates_materialized",
            None,
        ),
        context_page_candidates_per_op: metric_per_operation(
            samples,
            input,
            "context_page_snapshot_candidates_cloned",
            None,
        ),
        abi_c_string_allocations_per_op: metric_per_operation(
            samples,
            input,
            "abi_c_string_allocations",
            None,
        ),
        abi_c_string_bytes_per_op: metric_per_operation(samples, input, "abi_c_string_bytes", None),
    }
}

struct BenchMappedTableBytes {
    mmap: Arc<memmap2::Mmap>,
}

impl BenchMappedTableBytes {
    fn load(path: &Path) -> Result<Self, String> {
        let file = fs::File::open(path)
            .map_err(|error| format!("table open failed for {}: {error}", path.display()))?;
        let mmap = {
            // SAFETY: the benchmark run copies deployed artifacts into an isolated
            // work directory before loading them and does not mutate them afterward.
            unsafe { memmap2::MmapOptions::new().map(&file) }
        }
        .map_err(|error| format!("table mmap failed for {}: {error}", path.display()))?;
        Ok(Self {
            mmap: Arc::new(mmap),
        })
    }
}

impl CompactTableByteSource for BenchMappedTableBytes {
    fn bytes(&self) -> &[u8] {
        self.mmap.as_ref()
    }

    fn storage_label(&self) -> &'static str {
        "byte_backed"
    }

    fn mapping_mode(&self) -> &'static str {
        "mmap"
    }

    fn marisa_string_table(
        &self,
        offset: usize,
        size: usize,
    ) -> Result<Box<dyn CompactMarisaStringTable>, RimeTableBinParseError> {
        let end = offset
            .checked_add(size)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let payload = self
            .bytes()
            .get(offset..end)
            .ok_or(RimeTableBinParseError::OutOfBounds)?;
        let mut trie = rsmarisa::Trie::new();
        // SAFETY: the returned table stores an Arc to this exact mmap and drops
        // the trie before that owner, so the mapped slice remains valid for every
        // trie access.
        let payload = unsafe { std::mem::transmute::<&[u8], &'static [u8]>(payload) };
        trie.map(payload)
            .map_err(|_| RimeTableBinParseError::InvalidFormat)?;
        let num_keys = trie.num_keys();
        Ok(Box::new(BenchMappedMarisaStringTable {
            trie,
            payload_range: offset..end,
            num_keys,
            _mmap: Arc::clone(&self.mmap),
        }))
    }
}

impl fmt::Debug for BenchMappedTableBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BenchMappedTableBytes")
            .field("len", &self.mmap.len())
            .field("mapping_mode", &"mmap")
            .finish()
    }
}

struct BenchMappedDataBytes {
    mmap: memmap2::Mmap,
    mapping_mode: &'static str,
}

impl BenchMappedDataBytes {
    fn load(path: &Path, role: &str) -> Result<Self, String> {
        let file = fs::File::open(path)
            .map_err(|error| format!("{role} open failed for {}: {error}", path.display()))?;
        let mmap = {
            // SAFETY: the benchmark run copies deployed artifacts into an isolated
            // work directory before loading them and does not mutate them afterward.
            unsafe { memmap2::MmapOptions::new().map(&file) }
        }
        .map_err(|error| format!("{role} mmap failed for {}: {error}", path.display()))?;
        Ok(Self {
            mmap,
            mapping_mode: "mmap",
        })
    }

    fn bytes(&self) -> &[u8] {
        &self.mmap
    }

    fn mapping_mode(&self) -> &'static str {
        self.mapping_mode
    }
}

struct BenchMappedMarisaStringTable {
    trie: rsmarisa::Trie,
    payload_range: std::ops::Range<usize>,
    num_keys: usize,
    _mmap: Arc<memmap2::Mmap>,
}

impl fmt::Debug for BenchMappedMarisaStringTable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BenchMappedMarisaStringTable")
            .field("payload_range", &self.payload_range)
            .field("num_keys", &self.num_keys)
            .field("mapping_mode", &"mmap_embedded_payload")
            .finish_non_exhaustive()
    }
}

impl CompactMarisaStringTable for BenchMappedMarisaStringTable {
    fn get(&self, id: u32) -> Option<String> {
        if id as usize >= self.num_keys {
            return None;
        }
        let mut agent = rsmarisa::Agent::new();
        agent.set_query_id(id as usize);
        self.trie.reverse_lookup(&mut agent);
        Some(agent.key().as_str().to_owned())
    }

    fn num_keys(&self) -> usize {
        self.num_keys
    }

    fn mapping_mode(&self) -> &'static str {
        "mmap_embedded_payload"
    }
}

fn parse_status<T, E: std::fmt::Debug>(result: Result<T, E>) -> String {
    match result {
        Ok(_) => "ok".to_owned(),
        Err(error) => format!("{error:?}"),
    }
}

fn display_optional_path(path: Option<&PathBuf>) -> String {
    path.map_or_else(|| "missing".to_owned(), |path| path.display().to_string())
}

fn require<T>(name: &str, function: Option<T>) -> T {
    function.unwrap_or_else(|| panic!("RimeApi missing required function: {name}"))
}

fn duration_micros(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000_000.0
}

fn percentile(sorted_samples: &[f64], percentile: f64) -> f64 {
    if sorted_samples.is_empty() {
        return 0.0;
    }
    let index = ((sorted_samples.len() - 1) as f64 * percentile).ceil() as usize;
    sorted_samples[index.min(sorted_samples.len() - 1)]
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |value| value.to_string())
}

fn optional_f64(value: Option<f64>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |value| format!("{value:.3}"))
}

fn max_optional(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn csv(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}
