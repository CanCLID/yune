use std::{
    mem,
    os::raw::c_int,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard, OnceLock},
};

use libloading::Library;

#[path = "frontend_hosts/mod.rs"]
mod frontend_hosts;

type RimeGetApi = unsafe extern "C" fn() -> *mut yune_rime_api::RimeApi;

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

fn manifest_dir() -> Result<PathBuf, String> {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "missing CARGO_MANIFEST_DIR; cannot locate crate manifest".to_owned())
}

fn workspace_dir() -> Result<PathBuf, String> {
    manifest_dir()?
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "CARGO_MANIFEST_DIR is not under a workspace root".to_owned())
}

fn target_dir() -> Result<PathBuf, String> {
    if let Some(target_dir) = std::env::var_os("CARGO_TARGET_DIR") {
        Ok(PathBuf::from(target_dir))
    } else {
        Ok(workspace_dir()?.join("target"))
    }
}

fn artifact_candidates() -> Result<Vec<PathBuf>, String> {
    let target_dir = target_dir()?;
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_owned());
    Ok(vec![
        target_dir.join(&profile).join(dynamic_library_name()),
        target_dir.join("debug").join(dynamic_library_name()),
        target_dir.join("release").join(dynamic_library_name()),
    ])
}

fn find_dynamic_artifact() -> Result<Option<PathBuf>, String> {
    Ok(artifact_candidates()?
        .into_iter()
        .find(|candidate| candidate.is_file()))
}

fn build_dynamic_artifact() -> Result<(), String> {
    let manifest = manifest_dir()?.join("Cargo.toml");
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command
        .arg("build")
        .arg("-p")
        .arg("yune-rime-api")
        .arg("--manifest-path")
        .arg(manifest);
    if let Some(target_dir) = std::env::var_os("CARGO_TARGET_DIR") {
        command.arg("--target-dir").arg(target_dir);
    }

    let output = command
        .output()
        .map_err(|error| format!("failed to run cargo build for dynamic artifact: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo build -p yune-rime-api failed with status {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn discover_dynamic_artifact() -> Result<PathBuf, String> {
    if let Some(artifact) = find_dynamic_artifact()? {
        return Ok(artifact);
    }

    build_dynamic_artifact()?;
    if let Some(artifact) = find_dynamic_artifact()? {
        return Ok(artifact);
    }

    let checked = artifact_candidates()?
        .iter()
        .map(|candidate| candidate.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "missing Cargo-built dynamic artifact {}; checked {checked}",
        dynamic_library_name()
    ))
}

#[test]
fn dynamic_loader_harness_loads_cargo_cdylib_and_api_table() {
    let _guard = test_guard();
    let artifact =
        discover_dynamic_artifact().unwrap_or_else(|message| panic!("missing artifact: {message}"));

    // SAFETY: loading is restricted to the Cargo-built yune-rime-api artifact
    // discovered under the active target directory.
    let library = unsafe { Library::new(&artifact) }.unwrap_or_else(|error| {
        panic!(
            "failed to load dynamic artifact {}: {error}",
            artifact.display()
        )
    });

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

    let trace = frontend_hosts::native::run_native_host_lifecycle(api)
        .unwrap_or_else(|blocker| panic!("native host validation blocker: {blocker:?}"));
    assert_eq!(trace.to_json(), frontend_hosts::BASELINE_TRACE_FIXTURE);
}
