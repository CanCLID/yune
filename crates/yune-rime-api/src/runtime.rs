use std::{
    ffi::CString,
    fs,
    os::raw::c_char,
    path::Path,
    ptr,
    sync::{Mutex, OnceLock},
};

use serde_yaml::Value;

use crate::{
    copy_c_string_with_strncpy_semantics, cstring_from_lossless_str, optional_c_string,
    rime_struct_has_member, RimeTraits,
};

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

pub(crate) struct RuntimePathArgs<'a> {
    pub(crate) shared_data_dir: &'a str,
    pub(crate) user_data_dir: &'a str,
    pub(crate) prebuilt_data_dir: &'a str,
    pub(crate) staging_dir: &'a str,
    pub(crate) sync_dir: &'a str,
    pub(crate) user_id: &'a str,
    pub(crate) distribution: (&'a str, &'a str, &'a str),
    pub(crate) app_name: &'a str,
    pub(crate) log_dir: &'a str,
    pub(crate) backup_config_files: bool,
}

#[derive(Default)]
struct InstallationSettings {
    loaded: bool,
    installation_id: Option<String>,
    sync_dir: Option<String>,
    backup_config_files: Option<bool>,
}

impl Default for RuntimePaths {
    fn default() -> Self {
        Self::new(RuntimePathArgs {
            shared_data_dir: ".",
            user_data_dir: ".",
            prebuilt_data_dir: "build",
            staging_dir: "build",
            sync_dir: "sync",
            user_id: "unknown",
            distribution: ("", "", ""),
            app_name: "",
            log_dir: "",
            backup_config_files: true,
        })
    }
}

impl RuntimePaths {
    pub(crate) fn new(args: RuntimePathArgs<'_>) -> Self {
        let user_data_sync_dir = path_join(args.sync_dir, args.user_id);
        Self {
            shared_data_dir: cstring_from_lossless_str(args.shared_data_dir),
            user_data_dir: cstring_from_lossless_str(args.user_data_dir),
            prebuilt_data_dir: cstring_from_lossless_str(args.prebuilt_data_dir),
            staging_dir: cstring_from_lossless_str(args.staging_dir),
            sync_dir: cstring_from_lossless_str(args.sync_dir),
            user_id: cstring_from_lossless_str(args.user_id),
            user_data_sync_dir: cstring_from_lossless_str(&user_data_sync_dir),
            distribution_name: cstring_from_lossless_str(args.distribution.0),
            distribution_code_name: cstring_from_lossless_str(args.distribution.1),
            distribution_version: cstring_from_lossless_str(args.distribution.2),
            app_name: cstring_from_lossless_str(args.app_name),
            log_dir: cstring_from_lossless_str(args.log_dir),
            backup_config_files: args.backup_config_files,
        }
    }

    pub(crate) unsafe fn from_traits(traits: *const RimeTraits) -> Option<Self> {
        if traits.is_null() {
            return None;
        }

        // SAFETY: callers promise that `traits`, when non-null, points to at
        // least the leading `data_size` field of a `RimeTraits` object.
        let data_size = unsafe { (*traits).data_size };
        let provided_string = |member: *const *const c_char| {
            if rime_struct_has_member(traits, data_size, member) {
                // SAFETY: the field is covered by `data_size`; callers promise
                // that provided non-null strings are NUL-terminated.
                unsafe { optional_c_string(*member) }
            } else {
                None
            }
        };

        let shared_data_dir = provided_string(unsafe { ptr::addr_of!((*traits).shared_data_dir) })
            .unwrap_or_else(|| ".".to_owned());
        let user_data_dir = provided_string(unsafe { ptr::addr_of!((*traits).user_data_dir) })
            .unwrap_or_else(|| ".".to_owned());
        let prebuilt_data_dir =
            provided_string(unsafe { ptr::addr_of!((*traits).prebuilt_data_dir) })
                .unwrap_or_else(|| path_join(&shared_data_dir, "build"));
        let staging_dir = provided_string(unsafe { ptr::addr_of!((*traits).staging_dir) })
            .unwrap_or_else(|| path_join(&user_data_dir, "build"));
        let distribution_name =
            provided_string(unsafe { ptr::addr_of!((*traits).distribution_name) })
                .unwrap_or_default();
        let distribution_code_name =
            provided_string(unsafe { ptr::addr_of!((*traits).distribution_code_name) })
                .unwrap_or_default();
        let distribution_version =
            provided_string(unsafe { ptr::addr_of!((*traits).distribution_version) })
                .unwrap_or_default();
        let app_name =
            provided_string(unsafe { ptr::addr_of!((*traits).app_name) }).unwrap_or_default();
        let log_dir =
            provided_string(unsafe { ptr::addr_of!((*traits).log_dir) }).unwrap_or_default();
        let installation = read_installation_settings(&user_data_dir);
        let sync_dir = if let Some(sync_dir) = installation.sync_dir {
            sync_dir
        } else if installation.loaded {
            path_join(&user_data_dir, "sync")
        } else {
            "sync".to_owned()
        };
        let user_id = installation
            .installation_id
            .unwrap_or_else(|| "unknown".to_owned());
        let backup_config_files = installation.backup_config_files.unwrap_or(true);

        Some(Self::new(RuntimePathArgs {
            shared_data_dir: &shared_data_dir,
            user_data_dir: &user_data_dir,
            prebuilt_data_dir: &prebuilt_data_dir,
            staging_dir: &staging_dir,
            sync_dir: &sync_dir,
            user_id: &user_id,
            distribution: (
                &distribution_name,
                &distribution_code_name,
                &distribution_version,
            ),
            app_name: &app_name,
            log_dir: &log_dir,
            backup_config_files,
        }))
    }
}

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
        installation_id: root
            .get(Value::String("installation_id".to_owned()))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        sync_dir: root
            .get(Value::String("sync_dir".to_owned()))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        backup_config_files: root
            .get(Value::String("backup_config_files".to_owned()))
            .and_then(Value::as_bool),
    }
}

pub(crate) fn runtime_paths() -> &'static Mutex<RuntimePaths> {
    static RUNTIME_PATHS: OnceLock<Mutex<RuntimePaths>> = OnceLock::new();
    RUNTIME_PATHS.get_or_init(|| Mutex::new(RuntimePaths::default()))
}

/// Stores process-wide runtime traits for later path queries.
///
/// # Safety
///
/// `traits` must be either null or a valid pointer to a `RimeTraits` object.
/// Any non-null string pointers in the traits object must be valid
/// NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn RimeSetup(traits: *const RimeTraits) {
    let _trace = crate::startup_trace::span("runtime_setup");
    if let Some(paths) = unsafe { RuntimePaths::from_traits(traits) } {
        *runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned") = paths;
    }
}

#[no_mangle]
pub extern "C" fn RimeSetupLogging(app_name: *const c_char) {
    let Some(app_name) = optional_c_string(app_name) else {
        return;
    };
    runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned")
        .app_name = cstring_from_lossless_str(&app_name);
}

#[no_mangle]
pub extern "C" fn RimeGetSharedDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.shared_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetUserDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.user_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetPrebuiltDataDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.prebuilt_data_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetStagingDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.staging_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetSyncDir() -> *const c_char {
    runtime_path_ptr(|paths| &paths.sync_dir)
}

#[no_mangle]
pub extern "C" fn RimeGetUserId() -> *const c_char {
    runtime_path_ptr(|paths| &paths.user_id)
}

/// Copies the shared data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSharedDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.shared_data_dir, dir, buffer_size);
}

/// Copies the user data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetUserDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.user_data_dir, dir, buffer_size);
}

/// Copies the prebuilt data directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetPrebuiltDataDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.prebuilt_data_dir, dir, buffer_size);
}

/// Copies the staging directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStagingDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.staging_dir, dir, buffer_size);
}

/// Copies the sync directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSyncDirSecure(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.sync_dir, dir, buffer_size);
}

/// Copies the user-specific sync directory into caller-provided storage.
///
/// # Safety
///
/// `dir` must point to writable storage of `buffer_size` bytes. Null or empty
/// buffers are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeGetUserDataSyncDir(dir: *mut c_char, buffer_size: usize) {
    copy_runtime_path_to_buffer(|paths| &paths.user_data_sync_dir, dir, buffer_size);
}

pub(crate) fn path_join(base: &str, child: &str) -> String {
    Path::new(base).join(child).to_string_lossy().into_owned()
}

pub(crate) fn runtime_path_ptr(select: impl FnOnce(&RuntimePaths) -> &CString) -> *const c_char {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    select(&paths).as_ptr()
}

pub(crate) fn copy_runtime_path_to_buffer(
    select: impl FnOnce(&RuntimePaths) -> &CString,
    output: *mut c_char,
    buffer_size: usize,
) {
    if output.is_null() || buffer_size == 0 {
        return;
    }

    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    let value = select(&paths).to_string_lossy();
    copy_c_string_with_strncpy_semantics(&value, output, buffer_size);
}
