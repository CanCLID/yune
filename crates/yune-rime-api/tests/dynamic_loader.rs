use std::{
    ffi::{CStr, CString},
    mem,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, MutexGuard, OnceLock},
};

use libloading::Library;
use yune_rime_api::{
    Bool, RimeApi, RimeConfig, RimeTypeDuckProfileApi, RimeYuneWindowsProfileApi, TRUE,
};

#[path = "frontend_hosts/mod.rs"]
mod frontend_hosts;

type RimeGetApi = unsafe extern "C" fn() -> *mut yune_rime_api::RimeApi;
type RimeGetTypeDuckProfileApi = unsafe extern "C" fn() -> *mut RimeTypeDuckProfileApi;
type RimeGetYuneWindowsProfileApi = unsafe extern "C" fn() -> *mut RimeYuneWindowsProfileApi;

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

fn assert_upstream_api_data_size(api: &mut RimeApi, api_name: &str) {
    assert_eq!(
        api.data_size,
        (mem::size_of::<RimeApi>() - mem::size_of::<c_int>()) as c_int,
        "{api_name} must expose the upstream-shaped default RimeApi table"
    );
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
    assert_upstream_api_data_size(api, "rime_get_api");

    let trace = frontend_hosts::native::run_native_host_lifecycle(api)
        .unwrap_or_else(|blocker| panic!("native host validation blocker: {blocker:?}"));
    assert_eq!(
        trace.to_json(),
        frontend_hosts::BASELINE_TRACE_FIXTURE.replace("\r\n", "\n")
    );
}

#[test]
fn dynamic_loader_harness_loads_packaged_typeduck_profile_dll() {
    let Some(artifact) = std::env::var_os("YUNE_TYPEDUCK_PACKAGE_RIME_DLL") else {
        eprintln!("set YUNE_TYPEDUCK_PACKAGE_RIME_DLL to run the packaged TypeDuck profile smoke");
        return;
    };
    let _guard = test_guard();
    let artifact = PathBuf::from(artifact);
    assert!(
        artifact.is_file(),
        "packaged rime.dll should exist at {}",
        artifact.display()
    );

    // SAFETY: loading is restricted to the packaged DLL path provided by the
    // TypeDuck-Windows packaging script.
    let library = unsafe { Library::new(&artifact) }.unwrap_or_else(|error| {
        panic!(
            "failed to load packaged DLL {}: {error}",
            artifact.display()
        )
    });

    // SAFETY: the harness resolves only the exported null-terminated rime_get_api symbol.
    let get_api: libloading::Symbol<RimeGetApi> = unsafe { library.get(b"rime_get_api\0") }
        .unwrap_or_else(|error| panic!("missing packaged symbol rime_get_api: {error}"));
    // SAFETY: the resolved symbol follows the exported rime_get_api contract.
    let default_api = unsafe { get_api() };
    assert!(
        !default_api.is_null(),
        "packaged rime_get_api returned null"
    );
    // SAFETY: the table pointer was checked for null before dereference, and the
    // library is kept alive for the full duration of table use.
    let default_api = unsafe { &mut *default_api };
    assert_upstream_api_data_size(default_api, "packaged rime_get_api");
    assert_packaged_direct_call_symbols(&library);

    // SAFETY: the harness resolves only the exported null-terminated profile accessor.
    let get_profile_api: libloading::Symbol<RimeGetTypeDuckProfileApi> =
        unsafe { library.get(b"rime_get_typeduck_profile_api\0") }.unwrap_or_else(|error| {
            panic!("missing packaged symbol rime_get_typeduck_profile_api: {error}")
        });
    // SAFETY: the resolved symbol follows the exported profile accessor contract.
    let profile_api = unsafe { get_profile_api() };
    assert!(
        !profile_api.is_null(),
        "packaged rime_get_typeduck_profile_api returned null"
    );
    // SAFETY: the table pointer was checked for null before dereference, and the
    // library is kept alive for the full duration of table use.
    let profile_api = unsafe { &mut *profile_api };
    assert_eq!(
        profile_api.upstream.data_size,
        (mem::size_of::<RimeTypeDuckProfileApi>() - mem::size_of::<c_int>()) as c_int,
        "profile accessor must advertise the extended TypeDuck-profile table"
    );
    assert!(
        profile_api.upstream.data_size > default_api.data_size,
        "profile table must be opt-in and larger than the default table"
    );
    assert_profile_append_slots_round_trip(profile_api);

    let trace = frontend_hosts::native::run_native_host_lifecycle(&mut profile_api.upstream)
        .unwrap_or_else(|blocker| panic!("packaged profile lifecycle blocker: {blocker:?}"));
    assert_eq!(
        trace.to_json(),
        frontend_hosts::BASELINE_TRACE_FIXTURE.replace("\r\n", "\n")
    );
}

#[test]
fn dynamic_loader_harness_loads_packaged_yune_windows_profile_dll() {
    let Some(artifact) = std::env::var_os("YUNE_WINDOWS_PACKAGE_RIME_DLL") else {
        eprintln!(
            "set YUNE_WINDOWS_PACKAGE_RIME_DLL to run the packaged Yune Windows profile smoke"
        );
        return;
    };
    let _guard = test_guard();
    let artifact = PathBuf::from(artifact);
    assert!(
        artifact.is_file(),
        "packaged rime.dll should exist at {}",
        artifact.display()
    );

    // SAFETY: loading is restricted to the packaged DLL path provided by the
    // Yune Windows packaging script.
    let library = unsafe { Library::new(&artifact) }.unwrap_or_else(|error| {
        panic!(
            "failed to load packaged DLL {}: {error}",
            artifact.display()
        )
    });

    // SAFETY: the harness resolves only the exported null-terminated rime_get_api symbol.
    let get_api: libloading::Symbol<RimeGetApi> = unsafe { library.get(b"rime_get_api\0") }
        .unwrap_or_else(|error| panic!("missing packaged symbol rime_get_api: {error}"));
    // SAFETY: the resolved symbol follows the exported rime_get_api contract.
    let default_api = unsafe { get_api() };
    assert!(
        !default_api.is_null(),
        "packaged rime_get_api returned null"
    );
    // SAFETY: the table pointer was checked for null before dereference, and the
    // library is kept alive for the full duration of table use.
    let default_api = unsafe { &mut *default_api };
    assert_upstream_api_data_size(default_api, "packaged rime_get_api");
    assert_packaged_direct_call_symbols(&library);

    // SAFETY: the harness resolves only the exported null-terminated profile accessor.
    let get_profile_api: libloading::Symbol<RimeGetYuneWindowsProfileApi> =
        unsafe { library.get(b"rime_get_yune_windows_profile_api\0") }.unwrap_or_else(|error| {
            panic!("missing packaged symbol rime_get_yune_windows_profile_api: {error}")
        });
    // SAFETY: the resolved symbol follows the exported profile accessor contract.
    let profile_api = unsafe { get_profile_api() };
    assert!(
        !profile_api.is_null(),
        "packaged rime_get_yune_windows_profile_api returned null"
    );
    // SAFETY: the table pointer was checked for null before dereference, and the
    // library is kept alive for the full duration of table use.
    let profile_api = unsafe { &mut *profile_api };
    assert_eq!(
        profile_api.upstream.data_size,
        (mem::size_of::<RimeYuneWindowsProfileApi>() - mem::size_of::<c_int>()) as c_int,
        "profile accessor must advertise the extended Yune Windows table"
    );
    assert!(
        profile_api.upstream.data_size > default_api.data_size,
        "profile table must be opt-in and larger than the default table"
    );
    assert_profile_append_slots_round_trip(profile_api);

    let trace = frontend_hosts::native::run_native_host_lifecycle(&mut profile_api.upstream)
        .unwrap_or_else(|blocker| panic!("packaged profile lifecycle blocker: {blocker:?}"));
    assert_eq!(
        trace.to_json(),
        frontend_hosts::BASELINE_TRACE_FIXTURE.replace("\r\n", "\n")
    );
}

fn assert_packaged_direct_call_symbols(library: &Library) {
    const SYMBOLS: &[&[u8]] = &[
        b"RimeSetup\0",
        b"RimeInitialize\0",
        b"RimeFinalize\0",
        b"RimeGetContext\0",
        b"RimeConfigGetString\0",
    ];

    for symbol in SYMBOLS {
        // SAFETY: resolving a symbol does not call it; the placeholder function
        // type is used only to prove the export exists for legacy frontend code.
        unsafe { library.get::<unsafe extern "C" fn()>(symbol) }.unwrap_or_else(|error| {
            let name = String::from_utf8_lossy(&symbol[..symbol.len() - 1]);
            panic!("missing packaged direct-call symbol {name}: {error}");
        });
    }
}

fn assert_profile_append_slots_round_trip(profile_api: &mut RimeTypeDuckProfileApi) {
    let config_init = profile_api
        .upstream
        .config_init
        .expect("profile table should keep upstream config_init");
    let config_close = profile_api
        .upstream
        .config_close
        .expect("profile table should keep upstream config_close");
    let config_list_size = profile_api
        .upstream
        .config_list_size
        .expect("profile table should keep upstream config_list_size");
    let config_get_bool = profile_api
        .upstream
        .config_get_bool
        .expect("profile table should keep upstream config_get_bool");
    let config_get_int = profile_api
        .upstream
        .config_get_int
        .expect("profile table should keep upstream config_get_int");
    let config_get_double = profile_api
        .upstream
        .config_get_double
        .expect("profile table should keep upstream config_get_double");
    let config_get_string = profile_api
        .upstream
        .config_get_string
        .expect("profile table should keep upstream config_get_string");
    let append_bool = profile_api
        .config_list_append_bool
        .expect("profile table should expose config_list_append_bool");
    let append_int = profile_api
        .config_list_append_int
        .expect("profile table should expose config_list_append_int");
    let append_double = profile_api
        .config_list_append_double
        .expect("profile table should expose config_list_append_double");
    let append_string = profile_api
        .config_list_append_string
        .expect("profile table should expose config_list_append_string");

    let mut config = RimeConfig {
        ptr: std::ptr::null_mut(),
    };
    let items = CString::new("typeduck_profile_smoke").expect("key should be valid");
    let bool_item = CString::new("typeduck_profile_smoke/@0").expect("key should be valid");
    let int_item = CString::new("typeduck_profile_smoke/@1").expect("key should be valid");
    let double_item = CString::new("typeduck_profile_smoke/@2").expect("key should be valid");
    let string_item = CString::new("typeduck_profile_smoke/@3").expect("key should be valid");
    let string_value = CString::new("profile").expect("value should be valid");
    let mut bool_output: Bool = 0;
    let mut int_output: c_int = 0;
    let mut double_output = 0.0;
    let mut string_buffer = vec![0 as c_char; 64];

    assert_eq!(unsafe { config_init(&mut config) }, TRUE);
    assert_eq!(
        unsafe { append_bool(&mut config, items.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(unsafe { append_int(&mut config, items.as_ptr(), 7) }, TRUE);
    assert_eq!(
        unsafe { append_double(&mut config, items.as_ptr(), 1.25) },
        TRUE
    );
    assert_eq!(
        unsafe { append_string(&mut config, items.as_ptr(), string_value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { config_list_size(&mut config, items.as_ptr()) }, 4);
    assert_eq!(
        unsafe { config_get_bool(&mut config, bool_item.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, TRUE);
    assert_eq!(
        unsafe { config_get_int(&mut config, int_item.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 7);
    assert_eq!(
        unsafe { config_get_double(&mut config, double_item.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 1.25);
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                string_item.as_ptr(),
                string_buffer.as_mut_ptr(),
                string_buffer.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(string_buffer.as_ptr()) }.to_str(),
        Ok("profile")
    );
    assert_eq!(unsafe { config_close(&mut config) }, TRUE);
}
