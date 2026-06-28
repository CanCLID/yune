// Evidence-only iOS-footprint proxy probe. Drives the native RimeApi (mmap
// path) with prebuilt assets supplied via `prebuilt_data_dir` and records this
// process's Windows working set, private bytes, peak working set, and a
// test-local allocator live/high-water estimate. Run one schema per process:
//   $env:YUNE_MEM_SCHEMA="jyut6ping3_mobile"
//   $env:YUNE_MEM_DEFAULT="jyut6ping3_mobile"
//   $env:YUNE_MEM_DISABLE_DICTIONARY_LOOKUP_RECORDS="1"
//   $env:YUNE_MEM_EVIDENCE_DIR="docs/reports/evidence/m47-ios-budget-native-memory-attribution-2026-06-28"
//   cargo test -p yune-rime-api --test native_memory_probe -- --ignored --exact native_memory_probe_reports_working_set --nocapture
use std::{
    alloc::{GlobalAlloc, Layout, System},
    ffi::{CStr, CString},
    fs,
    path::{Path, PathBuf},
    ptr,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use yune_rime_api::{
    begin_memory_probe, finish_memory_probe, memory_probe_mark, rime_get_api,
    yune_m37_metrics_free_string, yune_m43_memory_owner_profile_json, MemoryProbeEvent,
    MemoryProbeSample, RimeTraits, TRUE,
};

struct CountingAllocator;

static ALLOCATOR_LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static ALLOCATOR_HIGH_WATER_BYTES: AtomicU64 = AtomicU64::new(0);

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            add_live_bytes(layout.size() as u64);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        subtract_live_bytes(layout.size() as u64);
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };
        if !new_pointer.is_null() {
            let old_size = layout.size() as u64;
            let new_size = new_size as u64;
            if new_size >= old_size {
                add_live_bytes(new_size - old_size);
            } else {
                subtract_live_bytes(old_size - new_size);
            }
        }
        new_pointer
    }
}

fn add_live_bytes(bytes: u64) {
    let live = ALLOCATOR_LIVE_BYTES
        .fetch_add(bytes, Ordering::AcqRel)
        .saturating_add(bytes);
    update_high_water(live);
}

fn subtract_live_bytes(bytes: u64) {
    let _ = ALLOCATOR_LIVE_BYTES.fetch_update(Ordering::AcqRel, Ordering::Acquire, |live| {
        Some(live.saturating_sub(bytes))
    });
}

fn update_high_water(live: u64) {
    let mut current = ALLOCATOR_HIGH_WATER_BYTES.load(Ordering::Acquire);
    while live > current {
        match ALLOCATOR_HIGH_WATER_BYTES.compare_exchange(
            current,
            live,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

#[derive(Clone, Debug)]
struct PhaseSample {
    phase: String,
    sample: MemoryProbeSample,
    named_owner_bytes: Option<u64>,
    clean_mmap_file_backed_estimate_bytes: Option<u64>,
    conclusion: String,
}

#[derive(Clone, Copy, Debug, Default)]
struct OwnerStats {
    named_owner_bytes: u64,
    heap_owned_bytes: u64,
    mmap_file_backed_bytes: u64,
}

#[cfg(windows)]
#[repr(C)]
#[allow(non_snake_case)]
struct ProcessMemoryCountersEx {
    cb: u32,
    PageFaultCount: u32,
    PeakWorkingSetSize: usize,
    WorkingSetSize: usize,
    QuotaPeakPagedPoolUsage: usize,
    QuotaPagedPoolUsage: usize,
    QuotaPeakNonPagedPoolUsage: usize,
    QuotaNonPagedPoolUsage: usize,
    PagefileUsage: usize,
    PeakPagefileUsage: usize,
    PrivateUsage: usize,
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
}

#[cfg(windows)]
#[link(name = "psapi")]
unsafe extern "system" {
    fn GetProcessMemoryInfo(
        process: *mut std::ffi::c_void,
        counters: *mut ProcessMemoryCountersEx,
        cb: u32,
    ) -> i32;
}

#[cfg(windows)]
fn process_memory_sample() -> MemoryProbeSample {
    let mut counters = ProcessMemoryCountersEx {
        cb: std::mem::size_of::<ProcessMemoryCountersEx>() as u32,
        PageFaultCount: 0,
        PeakWorkingSetSize: 0,
        WorkingSetSize: 0,
        QuotaPeakPagedPoolUsage: 0,
        QuotaPagedPoolUsage: 0,
        QuotaPeakNonPagedPoolUsage: 0,
        QuotaNonPagedPoolUsage: 0,
        PagefileUsage: 0,
        PeakPagefileUsage: 0,
        PrivateUsage: 0,
    };
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<ProcessMemoryCountersEx>() as u32,
        )
    } != 0;
    let (working_set, peak_working_set, private_bytes) = if ok {
        (
            Some(counters.WorkingSetSize as u64),
            Some(counters.PeakWorkingSetSize as u64),
            Some(counters.PrivateUsage as u64),
        )
    } else {
        (None, None, None)
    };
    MemoryProbeSample {
        working_set_bytes: working_set,
        peak_working_set_bytes: peak_working_set,
        private_bytes,
        allocator_live_bytes: Some(ALLOCATOR_LIVE_BYTES.load(Ordering::Acquire)),
        allocator_high_water_bytes: Some(ALLOCATOR_HIGH_WATER_BYTES.load(Ordering::Acquire)),
    }
}

#[cfg(not(windows))]
fn process_memory_sample() -> MemoryProbeSample {
    MemoryProbeSample {
        working_set_bytes: None,
        peak_working_set_bytes: None,
        private_bytes: None,
        allocator_live_bytes: Some(ALLOCATOR_LIVE_BYTES.load(Ordering::Acquire)),
        allocator_high_water_bytes: Some(ALLOCATOR_HIGH_WATER_BYTES.load(Ordering::Acquire)),
    }
}

fn capture_phase(phase: impl Into<String>, conclusion: impl Into<String>) -> PhaseSample {
    let phase = phase.into();
    memory_probe_mark(format!("m47:phase:{phase}"));
    PhaseSample {
        phase,
        sample: process_memory_sample(),
        named_owner_bytes: None,
        clean_mmap_file_backed_estimate_bytes: None,
        conclusion: conclusion.into(),
    }
}

fn public_schema_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/yune-web/public/schema")
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("dir create");
    for entry in fs::read_dir(source).expect("read dir") {
        let entry = entry.expect("entry");
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(&from, &to).expect("copy");
        }
    }
}

fn empty_traits() -> RimeTraits {
    RimeTraits {
        data_size: std::mem::size_of::<RimeTraits>() as i32,
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

#[test]
#[ignore = "evidence-only: set YUNE_MEM_SCHEMA and run one schema per process"]
fn native_memory_probe_reports_working_set() {
    let schema =
        std::env::var("YUNE_MEM_SCHEMA").unwrap_or_else(|_| "jyut6ping3_mobile".to_owned());

    begin_memory_probe(Some(process_memory_sample));
    let mut phases = Vec::new();
    phases.push(capture_phase(
        "baseline_process_start",
        "Windows process baseline before temporary schema copy",
    ));

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("yune-mem-{schema}-{nanos}"));
    let shared = root.join("shared");
    let user = root.join("user");
    copy_tree(&public_schema_root(), &shared);
    fs::create_dir_all(user.join("build")).expect("user build");

    if let Ok(default_schema) = std::env::var("YUNE_MEM_DEFAULT") {
        let path = shared.join("default.yaml");
        if let Ok(text) = fs::read_to_string(&path) {
            let patched = text.replace(
                "- schema: jyut6ping3",
                &format!("- schema: {default_schema}"),
            );
            fs::write(&path, patched).expect("patch default");
        }
        let _ = fs::remove_file(shared.join("build").join("default.yaml"));
    }

    let disable_dictionary_lookup_records =
        std::env::var("YUNE_MEM_DISABLE_DICTIONARY_LOOKUP_RECORDS").is_ok();
    let disable_compact_lookup_records =
        std::env::var("YUNE_MEM_DISABLE_COMPACT_LOOKUP_RECORDS").is_ok();

    if std::env::var("YUNE_MEM_NOSENTENCE").is_ok() {
        for name in ["jyut6ping3.schema.yaml", "jyut6ping3_mobile.schema.yaml"] {
            let path = shared.join(name);
            if let Ok(text) = fs::read_to_string(&path) {
                let patched = text
                    .replace("enable_sentence: true", "enable_sentence: false")
                    .replace("enable_completion: true", "enable_completion: false");
                fs::write(&path, patched).expect("patch schema");
            }
            let _ = fs::remove_file(shared.join("build").join(name));
        }
    }
    phases.push(capture_phase(
        "after_temp_schema_copy",
        "temporary committed schema bundle copied into isolated runtime dirs",
    ));

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).unwrap();
    let user_c = CString::new(user.to_string_lossy().as_ref()).unwrap();
    let prebuilt_c = CString::new(shared.to_string_lossy().as_ref()).unwrap();
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    traits.prebuilt_data_dir = prebuilt_c.as_ptr();

    let api = unsafe { &*rime_get_api() };
    let setup = api.setup.expect("setup");
    let initialize = api.initialize.expect("initialize");
    let deploy = api.deploy.expect("deploy");
    let create_session = api.create_session.expect("create_session");
    let select_schema = api.select_schema.expect("select_schema");
    let process_key = api.process_key.expect("process_key");

    unsafe { setup(&traits) };
    phases.push(capture_phase("after_setup", "RimeApi setup complete"));
    unsafe { initialize(&traits) };
    phases.push(capture_phase(
        "after_initialize",
        "RimeApi initialize complete",
    ));
    assert_eq!(deploy(), TRUE, "deploy should succeed");
    if disable_dictionary_lookup_records {
        let mut patched = 0usize;
        for name in ["jyut6ping3.schema.yaml", "jyut6ping3_mobile.schema.yaml"] {
            let path = user.join("build").join(name);
            if path.exists() {
                patch_schema_disable_dictionary_lookup_records(&path);
                patched += 1;
            }
        }
        assert!(
            patched > 0,
            "YUNE_MEM_DISABLE_DICTIONARY_LOOKUP_RECORDS requested but no target deployed schema was present"
        );
    }
    if disable_compact_lookup_records {
        let mut patched = 0usize;
        for name in ["jyut6ping3.schema.yaml", "jyut6ping3_mobile.schema.yaml"] {
            let path = user.join("build").join(name);
            if path.exists() {
                patch_schema_disable_compact_lookup_records(&path);
                patched += 1;
            }
        }
        assert!(
            patched > 0,
            "YUNE_MEM_DISABLE_COMPACT_LOOKUP_RECORDS requested but no target deployed schema was present"
        );
    }
    phases.push(capture_phase(
        "after_deploy",
        "deploy reused prebuilt assets; create_session has not run",
    ));

    let session = create_session();
    assert_ne!(session, 0, "session");
    let create_session_owner_rows = memory_owner_rows();
    let create_session_owner_stats = owner_stats(&create_session_owner_rows);
    phases.push(capture_phase(
        "after_create_session",
        "create_session loaded the workspace default schema",
    ));

    let schema_c = CString::new(schema.as_str()).unwrap();
    assert_eq!(unsafe { select_schema(session, schema_c.as_ptr()) }, TRUE);
    phases.push(capture_phase(
        "after_select_schema",
        "select_schema applied requested active schema",
    ));

    for ch in "neihoumaa".chars() {
        let _ = process_key(session, ch as i32, 0);
    }
    phases.push(capture_phase(
        "after_typing_neihoumaa",
        "realistic short Jyutping typing path",
    ));
    let _ = process_key(session, 0xff1b, 0);
    phases.push(capture_phase(
        "steady_after_esc",
        "steady sample after ESC clear",
    ));

    let events = finish_memory_probe();
    let final_owner_rows = memory_owner_rows();
    let final_owner_stats = owner_stats(&final_owner_rows);
    for phase in &mut phases {
        match phase.phase.as_str() {
            "after_create_session" => {
                phase.named_owner_bytes = Some(create_session_owner_stats.named_owner_bytes);
                phase.clean_mmap_file_backed_estimate_bytes =
                    Some(create_session_owner_stats.mmap_file_backed_bytes);
            }
            "after_select_schema" | "after_typing_neihoumaa" | "steady_after_esc" => {
                phase.named_owner_bytes = Some(final_owner_stats.named_owner_bytes);
                phase.clean_mmap_file_backed_estimate_bytes =
                    Some(final_owner_stats.mmap_file_backed_bytes);
            }
            _ => {}
        }
    }
    print_memresults(
        &schema,
        &phases,
        create_session_owner_stats,
        final_owner_stats,
    );
    if let Ok(evidence_dir) = std::env::var("YUNE_MEM_EVIDENCE_DIR") {
        write_evidence(
            &evidence_path(&evidence_dir),
            &schema,
            &phases,
            &events,
            &create_session_owner_rows,
            &final_owner_rows,
            create_session_owner_stats,
            final_owner_stats,
        );
    }

    let _ = fs::remove_dir_all(&root);
}

fn patch_schema_disable_dictionary_lookup_records(path: &Path) {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "YUNE_MEM_DISABLE_DICTIONARY_LOOKUP_RECORDS requested but deployed schema {} could not be read: {error}",
            path.display()
        )
    });
    if text.contains("load_lookup_records: false") {
        return;
    }
    let marker = "dictionary_lookup_filter:\n  dictionary: jyut6ping3_scolar\n";
    assert!(
        text.contains(marker),
        "YUNE_MEM_DISABLE_DICTIONARY_LOOKUP_RECORDS requested but deployed schema {} does not contain the dictionary_lookup_filter block",
        path.display()
    );
    let patched = text.replace(marker, "dictionary_lookup_filter:\n  dictionary: jyut6ping3_scolar\n  load_lookup_records: false\n");
    fs::write(path, patched).expect("patch dictionary lookup filter config");
}

fn patch_schema_disable_compact_lookup_records(path: &Path) {
    let text = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "YUNE_MEM_DISABLE_COMPACT_LOOKUP_RECORDS requested but deployed schema {} could not be read: {error}",
            path.display()
        )
    });
    let mut patched = patch_namespace_disable_compact_lookup_records(&text, "translator");
    patched = patch_namespace_disable_compact_lookup_records(&patched, "luna_pinyin");
    fs::write(path, patched).expect("patch compact lookup record config");
}

fn patch_namespace_disable_compact_lookup_records(text: &str, namespace: &str) -> String {
    let marker = format!("{namespace}:\n");
    assert!(
        text.contains(&marker),
        "YUNE_MEM_DISABLE_COMPACT_LOOKUP_RECORDS requested but deployed schema does not contain the {namespace} block"
    );
    let block_start = text.find(&marker).expect("namespace marker should exist") + marker.len();
    let block_end = text[block_start..]
        .find("\n\n")
        .map(|offset| block_start + offset)
        .unwrap_or(text.len());
    if text[block_start..block_end].contains("load_lookup_records:") {
        return text.to_owned();
    }
    let insertion = format!("{marker}  load_lookup_records: false\n");
    text.replacen(&marker, &insertion, 1)
}

fn evidence_path(raw_path: &str) -> PathBuf {
    let path = PathBuf::from(raw_path);
    if path.is_absolute() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }
}

fn memory_owner_rows() -> Vec<Value> {
    let json = yune_m43_memory_owner_profile_json();
    assert!(!json.is_null(), "owner profile json should not be null");
    let text = unsafe { CStr::from_ptr(json) }
        .to_str()
        .expect("owner profile JSON should be UTF-8")
        .to_owned();
    unsafe { yune_m37_metrics_free_string(json) };
    serde_json::from_str::<Vec<Value>>(&text).expect("owner profile JSON should parse")
}

fn owner_stats(rows: &[Value]) -> OwnerStats {
    let mut stats = OwnerStats::default();
    for row in rows {
        let bytes = row["estimated_bytes"].as_u64().unwrap_or(0);
        stats.named_owner_bytes = stats.named_owner_bytes.saturating_add(bytes);
        match row["class"].as_str().unwrap_or_default() {
            "mmap_file_backed" => {
                stats.mmap_file_backed_bytes = stats.mmap_file_backed_bytes.saturating_add(bytes);
            }
            class if class.starts_with("heap_owned") => {
                stats.heap_owned_bytes = stats.heap_owned_bytes.saturating_add(bytes);
            }
            _ => {}
        }
    }
    stats
}

fn print_memresults(
    schema: &str,
    phases: &[PhaseSample],
    create_session_owner_stats: OwnerStats,
    final_owner_stats: OwnerStats,
) {
    println!("MEMRESULT schema={schema}");
    for phase in phases {
        println!(
            "MEMRESULT phase={} ws_mb={} private_mb={} peak_ws_mb={} alloc_live_mb={} alloc_high_mb={}",
            phase.phase,
            mb_string(phase.sample.working_set_bytes),
            mb_string(phase.sample.private_bytes),
            mb_string(phase.sample.peak_working_set_bytes),
            mb_string(phase.sample.allocator_live_bytes),
            mb_string(phase.sample.allocator_high_water_bytes)
        );
    }
    println!(
        "MEMRESULT after_create_session_owner_named_mb={} after_create_session_owner_heap_mb={} after_create_session_owner_mmap_file_backed_mb={}",
        mb(create_session_owner_stats.named_owner_bytes),
        mb(create_session_owner_stats.heap_owned_bytes),
        mb(create_session_owner_stats.mmap_file_backed_bytes)
    );
    println!(
        "MEMRESULT final_owner_named_mb={} final_owner_heap_mb={} final_owner_mmap_file_backed_mb={}",
        mb(final_owner_stats.named_owner_bytes),
        mb(final_owner_stats.heap_owned_bytes),
        mb(final_owner_stats.mmap_file_backed_bytes)
    );
}

fn write_evidence(
    evidence_dir: &Path,
    schema: &str,
    phases: &[PhaseSample],
    events: &[MemoryProbeEvent],
    create_session_owner_rows: &[Value],
    final_owner_rows: &[Value],
    create_session_owner_stats: OwnerStats,
    final_owner_stats: OwnerStats,
) {
    fs::create_dir_all(evidence_dir).expect("evidence dir should be created");
    fs::write(
        evidence_dir.join("phase-memory.csv"),
        phase_samples_csv(phases),
    )
    .expect("phase CSV should be written");
    fs::write(
        evidence_dir.join("phase-memory.json"),
        serde_json::to_string_pretty(&phase_samples_json(phases)).unwrap(),
    )
    .expect("phase JSON should be written");
    fs::write(
        evidence_dir.join("create-session-events.csv"),
        events_csv(events),
    )
    .expect("event CSV should be written");
    fs::write(
        evidence_dir.join("create-session-events.json"),
        serde_json::to_string_pretty(&events_json(events)).unwrap(),
    )
    .expect("event JSON should be written");
    fs::write(
        evidence_dir.join("owner-attribution.csv"),
        owner_rows_csv(&[
            (
                "after_create_session_owner_profile",
                create_session_owner_rows,
            ),
            ("final_owner_profile", final_owner_rows),
        ]),
    )
    .expect("owner CSV should be written");
    fs::write(
        evidence_dir.join("owner-attribution.json"),
        serde_json::to_string_pretty(&json!({
            "after_create_session_owner_profile": create_session_owner_rows,
            "final_owner_profile": final_owner_rows,
        }))
        .unwrap(),
    )
    .expect("owner JSON should be written");
    fs::write(
        evidence_dir.join("summary.json"),
        serde_json::to_string_pretty(&json!({
            "schema": schema,
            "harness": "Windows native RimeApi lean probe with GetProcessMemoryInfo counters",
            "working_set_metric": "PROCESS_MEMORY_COUNTERS_EX.WorkingSetSize, equivalent to Windows process working set",
            "private_bytes_metric": "PROCESS_MEMORY_COUNTERS_EX.PrivateUsage",
            "allocator_metric": "test-local #[global_allocator] wrapper over std::alloc::System; process-wide approximation",
            "after_create_session_named_owner_bytes": create_session_owner_stats.named_owner_bytes,
            "after_create_session_heap_owned_named_owner_bytes": create_session_owner_stats.heap_owned_bytes,
            "after_create_session_clean_mmap_file_backed_estimate_bytes": create_session_owner_stats.mmap_file_backed_bytes,
            "final_named_owner_bytes": final_owner_stats.named_owner_bytes,
            "final_heap_owned_named_owner_bytes": final_owner_stats.heap_owned_bytes,
            "final_clean_mmap_file_backed_estimate_bytes": final_owner_stats.mmap_file_backed_bytes,
            "phase_memory_csv": "phase-memory.csv",
            "phase_events_csv": "create-session-events.csv",
            "owner_attribution_csv": "owner-attribution.csv"
        }))
        .unwrap(),
    )
    .expect("summary JSON should be written");
}

fn phase_samples_json(phases: &[PhaseSample]) -> Vec<Value> {
    phases
        .iter()
        .map(|phase| {
            json!({
                "phase": phase.phase,
                "working_set_bytes": phase.sample.working_set_bytes,
                "working_set_mb": phase.sample.working_set_bytes.map(mb),
                "private_bytes": phase.sample.private_bytes,
                "private_mb": phase.sample.private_bytes.map(mb),
                "peak_working_set_bytes": phase.sample.peak_working_set_bytes,
                "peak_working_set_mb": phase.sample.peak_working_set_bytes.map(mb),
                "allocator_live_bytes": phase.sample.allocator_live_bytes,
                "allocator_live_mb": phase.sample.allocator_live_bytes.map(mb),
                "allocator_high_water_bytes": phase.sample.allocator_high_water_bytes,
                "allocator_high_water_mb": phase.sample.allocator_high_water_bytes.map(mb),
                "named_owner_bytes": phase.named_owner_bytes,
                "clean_mmap_file_backed_estimate_bytes": phase.clean_mmap_file_backed_estimate_bytes,
                "conclusion": phase.conclusion,
            })
        })
        .collect()
}

fn events_json(events: &[MemoryProbeEvent]) -> Vec<Value> {
    events
        .iter()
        .enumerate()
        .map(|(event_index, event)| {
            json!({
                "event_index": event_index,
                "phase": event.phase,
                "working_set_bytes": event.working_set_bytes,
                "working_set_mb": event.working_set_bytes.map(mb),
                "private_bytes": event.private_bytes,
                "private_mb": event.private_bytes.map(mb),
                "peak_working_set_bytes": event.peak_working_set_bytes,
                "peak_working_set_mb": event.peak_working_set_bytes.map(mb),
                "allocator_live_bytes": event.allocator_live_bytes,
                "allocator_live_mb": event.allocator_live_bytes.map(mb),
                "allocator_high_water_bytes": event.allocator_high_water_bytes,
                "allocator_high_water_mb": event.allocator_high_water_bytes.map(mb),
                "named_owner_bytes": Value::Null,
                "clean_mmap_file_backed_estimate_bytes": Value::Null,
                "conclusion": "",
            })
        })
        .collect()
}

fn phase_samples_csv(phases: &[PhaseSample]) -> String {
    let mut output = csv_header();
    for phase in phases {
        output.push_str(&csv_row(
            None,
            &phase.phase,
            phase.sample.working_set_bytes,
            phase.sample.private_bytes,
            phase.sample.peak_working_set_bytes,
            phase.sample.allocator_live_bytes,
            phase.sample.allocator_high_water_bytes,
            phase.named_owner_bytes,
            phase.clean_mmap_file_backed_estimate_bytes,
            &phase.conclusion,
        ));
    }
    output
}

fn events_csv(events: &[MemoryProbeEvent]) -> String {
    let mut output = csv_header();
    for (event_index, event) in events.iter().enumerate() {
        output.push_str(&csv_row(
            Some(event_index),
            &event.phase,
            event.working_set_bytes,
            event.private_bytes,
            event.peak_working_set_bytes,
            event.allocator_live_bytes,
            event.allocator_high_water_bytes,
            None,
            None,
            "",
        ));
    }
    output
}

fn csv_header() -> String {
    "event_index,phase,working_set_bytes,working_set_mb,private_bytes,private_mb,peak_working_set_bytes,peak_working_set_mb,allocator_live_bytes,allocator_live_mb,allocator_high_water_bytes,allocator_high_water_mb,named_owner_bytes,clean_mmap_file_backed_estimate_bytes,conclusion\n".to_owned()
}

fn csv_row(
    event_index: Option<usize>,
    phase: &str,
    working_set_bytes: Option<u64>,
    private_bytes: Option<u64>,
    peak_working_set_bytes: Option<u64>,
    allocator_live_bytes: Option<u64>,
    allocator_high_water_bytes: Option<u64>,
    named_owner_bytes: Option<u64>,
    clean_mmap_file_backed_estimate_bytes: Option<u64>,
    conclusion: &str,
) -> String {
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        event_index.map_or_else(String::new, |index| index.to_string()),
        csv_field(phase),
        option_u64(working_set_bytes),
        option_mb(working_set_bytes),
        option_u64(private_bytes),
        option_mb(private_bytes),
        option_u64(peak_working_set_bytes),
        option_mb(peak_working_set_bytes),
        option_u64(allocator_live_bytes),
        option_mb(allocator_live_bytes),
        option_u64(allocator_high_water_bytes),
        option_mb(allocator_high_water_bytes),
        option_u64(named_owner_bytes),
        option_u64(clean_mmap_file_backed_estimate_bytes),
        csv_field(conclusion),
    )
}

fn owner_rows_csv(snapshots: &[(&str, &[Value])]) -> String {
    let mut output =
        "phase,session_id,owner,class,estimated_bytes,item_count,storage,notes\n".to_owned();
    for (phase, rows) in snapshots {
        for row in *rows {
            output.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                csv_field(phase),
                option_u64(row["session_id"].as_u64()),
                csv_field(row["owner"].as_str().unwrap_or_default()),
                csv_field(row["class"].as_str().unwrap_or_default()),
                option_u64(row["estimated_bytes"].as_u64()),
                option_u64(row["item_count"].as_u64()),
                csv_field(row["storage"].as_str().unwrap_or_default()),
                csv_field(row["notes"].as_str().unwrap_or_default()),
            ));
        }
    }
    output
}

fn csv_field(value: &str) -> String {
    let escaped = value.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

fn option_u64(value: Option<u64>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn option_mb(value: Option<u64>) -> String {
    value.map_or_else(String::new, |value| mb(value).to_string())
}

fn mb_string(value: Option<u64>) -> String {
    value.map_or_else(|| "n/a".to_owned(), |value| mb(value).to_string())
}

fn mb(bytes: u64) -> f64 {
    ((bytes as f64 / (1024.0 * 1024.0)) * 10.0).round() / 10.0
}
