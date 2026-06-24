use std::{
    collections::BTreeMap,
    ffi::CString,
    fs, mem,
    os::raw::{c_int, c_void},
    path::{Path, PathBuf},
    ptr,
    time::{Duration, Instant},
};

use libloading::Library;
use yune_core::{
    parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_table_bin_dictionary, rime_dict_source_checksum, rime_table_bin_dict_file_checksum,
};
use yune_rime_api::{
    RimeApi, RimeComposition, RimeContext, RimeMenu, RimeSessionId, RimeStatus, RimeTraits, TRUE,
};

type RimeGetApi = unsafe extern "C" fn() -> *mut RimeApi;

const DEFAULT_ITERATIONS: usize = 9;
const DEFAULT_SESSION_ITERATIONS: usize = 60;
const DEFAULT_KEY_ITERATIONS: usize = 80;
const KEY_WARMUPS: usize = 5;

fn main() {
    let options = Options::parse();
    fs::create_dir_all(&options.output).expect("output directory should be created");
    let engine =
        LoadedRime::load(&options.dll).unwrap_or_else(|error| panic!("load rime failed: {error}"));
    if options.deploy_before_benchmark {
        deploy_workspace(&engine, &options);
    }
    let samples = run_benchmark(&engine, &options);
    write_samples(&options.output.join("samples.csv"), &samples);
    write_summary(&options.output.join("summary.csv"), &samples);
    write_product_path_status(&options.output.join("product_path_status.csv"), &options);
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
    _library: Library,
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
        Ok(Self {
            _library: library,
            api,
        })
    }

    fn api(&self) -> &RimeApi {
        unsafe { &*self.api }
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
}

fn run_benchmark(engine: &LoadedRime, options: &Options) -> Vec<Sample> {
    let mut samples = Vec::new();
    run_startup(engine, options, &mut samples);
    run_session(engine, options, &mut samples);
    for input in &options.inputs {
        run_key_workload(engine, options, input, &mut samples);
    }
    samples
}

fn run_startup(engine: &LoadedRime, options: &Options, samples: &mut Vec<Sample>) {
    for index in 0..options.iterations {
        let api = engine.api();
        let traits = TraitsBundle::new(options);
        let before = current_memory_sample();
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

fn run_session(engine: &LoadedRime, options: &Options, samples: &mut Vec<Sample>) {
    with_service(engine, options, |api| {
        for index in 0..options.session_iterations {
            let before = current_memory_sample();
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
        let session_id = require("create_session", api.create_session)();
        assert_ne!(session_id, 0, "create_session returned 0");
        select_schema(api, session_id, &options.schema);
        set_default_options(api, session_id);
        for _ in 0..KEY_WARMUPS {
            process_input_with_context(api, session_id, input);
        }
        for index in 0..options.key_iterations {
            let before = current_memory_sample();
            let start = Instant::now();
            process_input_with_context(api, session_id, input);
            let elapsed = start.elapsed();
            let after = current_memory_sample();
            samples.push(Sample::new(
                options,
                "key_sequence_process_with_context",
                input,
                index,
                input.chars().count(),
                elapsed,
                before,
                after,
                None,
            ));
        }
        assert_eq!(
            require("destroy_session", api.destroy_session)(session_id),
            TRUE
        );
    });
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
    let mut output = String::from("engine,track,schema_id,dictionary_id,prism_id,source_path,table_path,prism_path,reverse_path,source_checksum,table_checksum,checksum_status,table_parse,prism_parse,reverse_parse,compiled_ready\n");
    if options.track == "track-b-product" {
        for (dictionary_id, prism_id) in product_dictionary_requests(&options.schema) {
            let status = ProductPathStatus::inspect(options, dictionary_id, prism_id);
            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
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
                status.compiled_ready
            ));
        }
    }
    fs::write(path, output).expect("product path status CSV should be written");
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
        let checksum_status = match (source_checksum, table_checksum) {
            (Some(source), Some(table)) if source == table => "fresh",
            (Some(_), Some(_)) => "stale",
            (None, _) => "missing_source",
            (_, None) => "missing_table_checksum",
        }
        .to_owned();
        let table_parse = table_bytes
            .as_ref()
            .map(|bytes| parse_status(parse_rime_table_bin_dictionary(bytes)))
            .unwrap_or_else(|| "missing".to_owned());
        let prism_parse = prism_bytes
            .as_ref()
            .map(|bytes| parse_status(parse_rime_prism_bin_payload(bytes)))
            .unwrap_or_else(|| "missing".to_owned());
        let reverse_parse = reverse_bytes
            .as_ref()
            .map(|bytes| parse_status(parse_rime_reverse_bin_dictionary(bytes)))
            .unwrap_or_else(|| "missing".to_owned());
        let compiled_ready = checksum_status == "fresh"
            && table_parse == "ok"
            && prism_parse == "ok"
            && reverse_parse == "ok";

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
        }
    }
}

fn selected_data_path(options: &Options, file_name: &str) -> Option<PathBuf> {
    [
        options.build.join(file_name),
        options.shared.join(file_name),
    ]
    .into_iter()
    .find(|path| path.is_file())
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
