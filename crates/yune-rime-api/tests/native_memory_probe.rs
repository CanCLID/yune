// Evidence-only iOS-footprint probe. Drives the native RimeApi (mmap path) with
// prebuilt assets supplied via `prebuilt_data_dir` (the ship-prebuilt scenario an
// iOS keyboard uses) and reads this process's own working set at baseline, after
// load+session (steady), and the lifetime peak. Run one schema per process:
//   YUNE_MEM_SCHEMA=jyut6ping3_mobile cargo test -p yune-rime-api \
//     --test native_memory_probe -- --ignored --exact native_memory_probe_reports_working_set
use std::{
    ffi::CString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    ptr,
    time::{SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{rime_get_api, RimeTraits, TRUE};

fn read_metric_mb(metric: &str) -> f64 {
    let pid = std::process::id();
    let script = format!("$p=Get-Process -Id {pid}; $p.Refresh(); [int64]$p.{metric}");
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .expect("powershell should run");
    let text = String::from_utf8_lossy(&output.stdout);
    let bytes: f64 = text.trim().parse().unwrap_or(0.0);
    (bytes / (1024.0 * 1024.0) * 10.0).round() / 10.0
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
    let baseline = read_metric_mb("WorkingSet64");

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("yune-mem-{schema}-{nanos}"));
    let shared = root.join("shared");
    let user = root.join("user");
    copy_tree(&public_schema_root(), &shared);
    fs::create_dir_all(user.join("build")).expect("user build");

    // A/B lever: when set, rewrite the workspace default schema_list so create_session
    // loads only the named schema (isolates per-schema create_session memory).
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

    // A/B lever: when set, disable sentence + completion in the source schemas and
    // force config recompile (drop precompiled build/*.schema.yaml) while keeping the
    // prebuilt dictionary .bin. Isolates the upstream sentence model's memory cost.
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

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).unwrap();
    let user_c = CString::new(user.to_string_lossy().as_ref()).unwrap();
    // prebuilt_data_dir points at the committed compiled assets => deploy reuses + mmaps
    // them instead of rebuilding, exactly like an iOS keyboard loading bundle assets.
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
    unsafe { initialize(&traits) };
    let after_setup = read_metric_mb("WorkingSet64");
    assert_eq!(deploy(), TRUE, "deploy should succeed");
    let after_deploy = read_metric_mb("WorkingSet64");

    let session = create_session();
    assert_ne!(session, 0, "session");
    let after_session = read_metric_mb("WorkingSet64");
    let schema_c = CString::new(schema.as_str()).unwrap();
    assert_eq!(unsafe { select_schema(session, schema_c.as_ptr()) }, TRUE);
    let after_select = read_metric_mb("WorkingSet64");
    // realistic short typing session
    for ch in "neihoumaa".chars() {
        let _ = process_key(session, ch as i32, 0);
    }
    let after_typing = read_metric_mb("WorkingSet64");
    let _ = process_key(session, 0xff1b, 0); // ESC clear
    println!("MEMRESULT after_session_ws_mb={after_session}");
    println!("MEMRESULT after_select_ws_mb={after_select}");
    println!("MEMRESULT after_typing_ws_mb={after_typing}");

    let steady = read_metric_mb("WorkingSet64");
    let peak = read_metric_mb("PeakWorkingSet64");

    println!("MEMRESULT schema={schema}");
    println!("MEMRESULT baseline_ws_mb={baseline}");
    println!("MEMRESULT after_setup_ws_mb={after_setup}");
    println!("MEMRESULT after_deploy_ws_mb={after_deploy}");
    println!("MEMRESULT steady_ws_mb={steady}");
    println!("MEMRESULT peak_ws_mb={peak}");
    println!(
        "MEMRESULT engine_steady_delta_mb={}",
        ((steady - baseline) * 10.0).round() / 10.0
    );
    println!(
        "MEMRESULT deploy_transient_mb={}",
        ((peak - steady) * 10.0).round() / 10.0
    );

    let _ = fs::remove_dir_all(&root);
}
