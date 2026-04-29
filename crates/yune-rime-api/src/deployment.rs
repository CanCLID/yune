use std::{
    collections::HashSet,
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_yaml::{Mapping, Number, Value};

use crate::{
    apply_config_directives, apply_custom_patch, apply_legacy_preset_config_plugins, bool_from,
    cstring_from_lossless_str, find_config_value, load_runtime_config_root,
    normalize_config_resource_id, optional_c_string, path_join,
    resource_id::validate_data_resource_id, runtime_paths, runtime_user_data_sync_dir,
    service_started, set_build_info, set_config_value, source_modified_secs,
    source_uses_auto_custom_patch, sync_all_user_dicts, user_dict_upgrade, Bool, ConfigOpenKind,
    RimeCleanupAllSessions, RimeSetup, RimeTraits, FALSE, RIME_VERSION_BYTES, TRUE,
};

/// Initializes the runtime using the same trait handling as `RimeSetup`.
///
/// # Safety
///
/// `traits` follows the same preconditions as `RimeSetup`.
#[no_mangle]
pub unsafe extern "C" fn RimeInitialize(traits: *const RimeTraits) {
    // SAFETY: forwarded preconditions are identical to `RimeSetup`.
    unsafe { RimeSetup(traits) };
    service_started().store(true, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn RimeFinalize() {
    RimeCleanupAllSessions();
    service_started().store(false, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn RimeStartMaintenance(full_check: Bool) -> Bool {
    let _ = clean_old_log_files();
    if !run_installation_update() {
        return FALSE;
    }
    if full_check == FALSE && !detect_modifications() {
        return FALSE;
    }
    crate::notify(0, "deploy", "start");
    let success = run_workspace_maintenance_tasks();
    crate::notify(0, "deploy", if success { "success" } else { "failure" });
    bool_from(success)
}

#[no_mangle]
pub extern "C" fn RimeStartMaintenanceOnWorkspaceChange() -> Bool {
    RimeStartMaintenance(FALSE)
}

#[no_mangle]
pub extern "C" fn RimeIsMaintenancing() -> Bool {
    FALSE
}

#[no_mangle]
pub extern "C" fn RimeJoinMaintenanceThread() {}

/// Initializes deployer state using the same trait handling as `RimeSetup`.
///
/// # Safety
///
/// `traits` follows the same preconditions as `RimeSetup`.
#[no_mangle]
pub unsafe extern "C" fn RimeDeployerInitialize(traits: *const RimeTraits) {
    // SAFETY: forwarded preconditions are identical to `RimeSetup`.
    unsafe { RimeSetup(traits) };
}

#[no_mangle]
pub extern "C" fn RimePrebuildAllSchemas() -> Bool {
    bool_from(prebuild_all_schemas())
}

#[no_mangle]
pub extern "C" fn RimeDeployWorkspace() -> Bool {
    if !run_installation_update() {
        return FALSE;
    }
    if !run_workspace_maintenance_tasks() {
        return FALSE;
    }
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeDeploySchema(schema_file: *const c_char) -> Bool {
    let Some(schema_file) = optional_c_string(schema_file) else {
        return FALSE;
    };
    bool_from(deploy_schema_file(&schema_file))
}

#[no_mangle]
pub extern "C" fn RimeDeployConfigFile(
    file_name: *const c_char,
    version_key: *const c_char,
) -> Bool {
    let Some(file_name) = optional_c_string(file_name) else {
        return FALSE;
    };
    let Some(version_key) = optional_c_string(version_key) else {
        return FALSE;
    };
    bool_from(deploy_config_file(&file_name, &version_key))
}

#[no_mangle]
pub extern "C" fn RimeSyncUserData() -> Bool {
    RimeCleanupAllSessions();
    crate::notify(0, "deploy", "start");
    let installation_synced = run_installation_update();
    let configs_synced = backup_config_files();
    let user_dicts_synced = sync_all_user_dicts();
    let success = installation_synced && configs_synced && user_dicts_synced;
    crate::notify(0, "deploy", if success { "success" } else { "failure" });
    bool_from(success)
}

#[no_mangle]
pub extern "C" fn RimeRunTask(task_name: *const c_char) -> Bool {
    let Some(task_name) = optional_c_string(task_name) else {
        return FALSE;
    };
    if task_name == "user_dict_sync" {
        return bool_from(sync_all_user_dicts());
    }
    if task_name == "backup_config_files" {
        return bool_from(backup_config_files());
    }
    if task_name == "installation_update" {
        return bool_from(run_installation_update());
    }
    if task_name == "clean_old_log_files" {
        return bool_from(clean_old_log_files());
    }
    if task_name == "cleanup_trash" {
        return bool_from(cleanup_trash());
    }
    if task_name == "workspace_update" {
        return bool_from(workspace_update());
    }
    if task_name == "user_dict_upgrade" {
        return bool_from(user_dict_upgrade());
    }
    if task_name == "prebuild_all_schemas" {
        return bool_from(prebuild_all_schemas());
    }
    FALSE
}

pub(crate) fn run_installation_update() -> bool {
    let (
        user_data_dir,
        current_sync_dir,
        distribution_name,
        distribution_code_name,
        distribution_version,
    ) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
            paths.sync_dir.to_string_lossy().into_owned(),
            paths.distribution_name.to_string_lossy().into_owned(),
            paths.distribution_code_name.to_string_lossy().into_owned(),
            paths.distribution_version.to_string_lossy().into_owned(),
        )
    };

    if fs::create_dir_all(&user_data_dir).is_err() {
        return false;
    }

    let installation_path = user_data_dir.join("installation.yaml");
    let mut root = fs::read_to_string(&installation_path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .and_then(|value| match value {
            Value::Mapping(root) => Some(root),
            _ => None,
        })
        .unwrap_or_default();

    let installation_key = Value::String("installation_id".to_owned());
    let sync_key = Value::String("sync_dir".to_owned());
    let backup_key = Value::String("backup_config_files".to_owned());

    let existing_installation_id = root
        .get(&installation_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let installation_id = existing_installation_id.unwrap_or_else(|| {
        let generated = generate_installation_id();
        root.insert(installation_key.clone(), Value::String(generated.clone()));
        root.insert(
            Value::String("install_time".to_owned()),
            Value::String(current_unix_time_string()),
        );
        generated
    });

    let sync_dir = root
        .get(&sync_key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| {
            if current_sync_dir != "sync" {
                current_sync_dir
            } else {
                user_data_dir.join("sync").to_string_lossy().into_owned()
            }
        });
    let backup_config_files = root
        .get(&backup_key)
        .and_then(Value::as_bool)
        .unwrap_or(true);

    if root.contains_key(Value::String("install_time".to_owned())) {
        root.insert(
            Value::String("update_time".to_owned()),
            Value::String(current_unix_time_string()),
        );
    }
    if !distribution_name.is_empty() {
        root.insert(
            Value::String("distribution_name".to_owned()),
            Value::String(distribution_name.clone()),
        );
    }
    if !distribution_code_name.is_empty() {
        root.insert(
            Value::String("distribution_code_name".to_owned()),
            Value::String(distribution_code_name.clone()),
        );
    }
    if !distribution_version.is_empty() {
        root.insert(
            Value::String("distribution_version".to_owned()),
            Value::String(distribution_version.clone()),
        );
    }
    root.insert(
        Value::String("rime_version".to_owned()),
        Value::String(
            String::from_utf8_lossy(&RIME_VERSION_BYTES[..RIME_VERSION_BYTES.len() - 1])
                .into_owned(),
        ),
    );

    let yaml = match serde_yaml::to_string(&Value::Mapping(root)) {
        Ok(yaml) => yaml,
        Err(_) => return false,
    };
    if fs::write(&installation_path, yaml).is_err() {
        return false;
    }

    let user_data_sync_dir = path_join(&sync_dir, &installation_id);
    let mut paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    paths.user_id = cstring_from_lossless_str(&installation_id);
    paths.sync_dir = cstring_from_lossless_str(&sync_dir);
    paths.user_data_sync_dir = cstring_from_lossless_str(&user_data_sync_dir);
    paths.backup_config_files = backup_config_files;
    true
}

pub(crate) fn backup_config_files() -> bool {
    let (user_data_dir, backup_enabled) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
            paths.backup_config_files,
        )
    };
    if !backup_enabled {
        return true;
    }

    let Ok(entries) = fs::read_dir(&user_data_dir) else {
        return false;
    };

    let backup_dir = runtime_user_data_sync_dir();
    let mut success = true;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !should_backup_config_file(&path) {
            continue;
        }
        if fs::create_dir_all(&backup_dir).is_err() {
            return false;
        }
        let destination = backup_dir.join(entry.file_name());
        if fs::copy(&path, destination).is_err() {
            success = false;
        }
    }
    success
}

pub(crate) fn cleanup_trash() -> bool {
    let user_data_dir = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned())
    };
    let Ok(entries) = fs::read_dir(&user_data_dir) else {
        return false;
    };

    let trash_dir = user_data_dir.join("trash");
    let mut success = true;
    let mut trash_created = false;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !should_cleanup_trash_file(&path) {
            continue;
        }
        if !trash_created {
            if fs::create_dir_all(&trash_dir).is_err() {
                return false;
            }
            trash_created = true;
        }
        if fs::rename(&path, trash_dir.join(entry.file_name())).is_err() {
            success = false;
        }
    }
    success
}

pub(crate) fn clean_old_log_files() -> bool {
    let (app_name, log_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            paths.app_name.to_string_lossy().into_owned(),
            PathBuf::from(paths.log_dir.to_string_lossy().into_owned()),
        )
    };
    if app_name.is_empty() || log_dir.as_os_str().is_empty() {
        return true;
    }

    let Ok(entries) = fs::read_dir(&log_dir) else {
        return true;
    };
    let entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    let mut files_in_use = HashSet::new();
    for path in &entries {
        let Ok(metadata) = fs::symlink_metadata(path) else {
            continue;
        };
        if !metadata.file_type().is_symlink() {
            continue;
        }
        let Ok(target) = fs::read_link(path) else {
            continue;
        };
        let Some(target_file_name) = target.file_name().and_then(|file_name| file_name.to_str())
        else {
            continue;
        };
        if target_file_name.starts_with(&app_name) && target_file_name.ends_with(".log") {
            files_in_use.insert(target_file_name.to_owned());
        }
    }

    let today = current_log_date_marker();
    let mut success = true;
    for path in entries {
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            success = false;
            continue;
        };
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
            continue;
        };
        if !file_name.starts_with(&app_name) || !file_name.ends_with(".log") {
            continue;
        }
        if file_name.contains(&today) {
            continue;
        }
        if files_in_use.contains(file_name) {
            continue;
        }
        if fs::remove_file(&path).is_err() {
            success = false;
        }
    }
    success
}

pub(crate) fn detect_modifications() -> bool {
    let (shared_data_dir, user_data_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned()),
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
        )
    };

    let Some(last_modified) =
        latest_workspace_modified_time([user_data_dir.as_path(), shared_data_dir.as_path()])
    else {
        return true;
    };
    last_modified > user_last_build_time(&user_data_dir)
}

fn latest_workspace_modified_time<const N: usize>(data_dirs: [&Path; N]) -> Option<u64> {
    let mut last_modified = 0;
    for data_dir in data_dirs {
        last_modified = last_modified.max(file_modified_secs(data_dir)?);
        if data_dir.is_dir() {
            let entries = fs::read_dir(data_dir).ok()?;
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if !path.is_file() || !is_workspace_yaml_file(&path) {
                    continue;
                }
                last_modified = last_modified.max(file_modified_secs(&path)?);
            }
        }
    }
    Some(last_modified)
}

fn file_modified_secs(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn user_last_build_time(user_data_dir: &Path) -> u64 {
    let user_config_path = user_data_dir.join("user.yaml");
    fs::read_to_string(user_config_path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .and_then(|root| {
            find_config_value(&root, "var/last_build_time")
                .and_then(Value::as_i64)
                .and_then(|value| u64::try_from(value).ok())
        })
        .unwrap_or(0)
}

fn is_workspace_yaml_file(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("yaml")
        && path.file_name().and_then(|file_name| file_name.to_str()) != Some("user.yaml")
}

pub(crate) fn current_log_date_marker() -> String {
    let days_since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() / 86_400);
    let (year, month, day) = civil_from_days(days_since_epoch as i64);
    format!(".{year:04}{month:02}{day:02}")
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn should_cleanup_trash_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
        return false;
    };
    file_name == "rime.log"
        || file_name.ends_with(".bin")
        || file_name.ends_with(".reverse.kct")
        || file_name.ends_with(".userdb.kct.old")
        || file_name.ends_with(".userdb.kct.snapshot")
}

fn should_backup_config_file(path: &Path) -> bool {
    let extension = path.extension().and_then(|extension| extension.to_str());
    match extension {
        Some("txt") => true,
        Some("yaml") => !is_generated_customized_copy(path),
        _ => false,
    }
}

fn is_generated_customized_copy(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
        return false;
    };
    if file_name.ends_with(".custom.yaml") {
        return false;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    let Ok(Value::Mapping(root)) = serde_yaml::from_str::<Value>(&text) else {
        return false;
    };
    matches!(
        root.get(Value::String("customization".to_owned())),
        Some(Value::String(_))
    )
}

fn generate_installation_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("yune-{nanos}-{}", std::process::id())
}

fn current_unix_time_string() -> String {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or_else(
        |_| "0".to_owned(),
        |duration| duration.as_secs().to_string(),
    )
}

pub(crate) fn workspace_update() -> bool {
    if !deploy_config_file("default.yaml", "config_version") {
        return false;
    }
    let _ = symlink_prebuilt_dictionaries();

    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Some(schema_ids) = workspace_schema_ids(&default_config) else {
        return false;
    };

    let mut built = HashSet::new();
    let mut success = true;
    for schema_id in schema_ids {
        if !workspace_update_schema(&schema_id, false, &mut built) {
            success = false;
        }
    }

    write_last_build_time() && success
}

pub(crate) fn run_workspace_maintenance_tasks() -> bool {
    workspace_update() && user_dict_upgrade() && cleanup_trash()
}

fn workspace_schema_ids(default_config: &Value) -> Option<Vec<String>> {
    let Value::Sequence(schema_list) = find_config_value(default_config, "schema_list")? else {
        return None;
    };
    Some(
        schema_list
            .iter()
            .filter_map(|entry| {
                let Value::Mapping(entry) = entry else {
                    return None;
                };
                entry
                    .get(Value::String("schema".to_owned()))
                    .and_then(Value::as_str)
                    .and_then(validate_data_resource_id)
            })
            .collect(),
    )
}

fn workspace_update_schema(
    schema_id: &str,
    as_dependency: bool,
    built: &mut HashSet<String>,
) -> bool {
    if !built.insert(schema_id.to_owned()) {
        return true;
    }

    let schema_file = format!("{schema_id}.schema.yaml");
    if !deploy_schema_file(&schema_file) {
        return as_dependency;
    }

    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    if !as_dependency {
        for dependency_id in schema_dependencies(&schema_config) {
            if !workspace_update_schema(&dependency_id, true, built) {
                return false;
            }
        }
    }
    true
}

fn schema_dependencies(schema_config: &Value) -> Vec<String> {
    let Some(Value::Sequence(dependencies)) =
        find_config_value(schema_config, "schema/dependencies")
    else {
        return Vec::new();
    };
    dependencies
        .iter()
        .filter_map(Value::as_str)
        .filter_map(validate_data_resource_id)
        .collect()
}

fn write_last_build_time() -> bool {
    let user_data_dir = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned())
    };
    if fs::create_dir_all(&user_data_dir).is_err() {
        return false;
    }

    let user_config_path = user_data_dir.join("user.yaml");
    let mut user_config = fs::read_to_string(&user_config_path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| Value::Mapping(Mapping::new()));
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let timestamp = c_int::try_from(timestamp).unwrap_or(c_int::MAX);
    if !set_config_value(
        &mut user_config,
        "var/last_build_time",
        Value::Number(Number::from(timestamp)),
    ) {
        return false;
    }
    let Ok(yaml) = serde_yaml::to_string(&user_config) else {
        return false;
    };
    fs::write(user_config_path, yaml).is_ok()
}

fn symlink_prebuilt_dictionaries() -> bool {
    let (shared_data_dir, user_data_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned()),
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
        )
    };
    if !shared_data_dir.is_dir() || !user_data_dir.is_dir() {
        return false;
    }
    let Ok(shared_data_dir) = shared_data_dir.canonicalize() else {
        return false;
    };
    if user_data_dir
        .canonicalize()
        .is_ok_and(|user_data_dir| user_data_dir == shared_data_dir)
    {
        return false;
    }

    let Ok(entries) = fs::read_dir(&user_data_dir) else {
        return false;
    };
    let mut success = true;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            success = false;
            continue;
        };
        if !metadata.file_type().is_symlink() {
            continue;
        }

        let target_path = path.canonicalize();
        let bad_link = target_path.is_err();
        let linked_to_shared_data = target_path
            .ok()
            .and_then(|target_path| target_path.parent().map(Path::to_path_buf))
            .is_some_and(|parent| parent == shared_data_dir);
        if (bad_link || linked_to_shared_data) && fs::remove_file(&path).is_err() {
            success = false;
        }
    }
    success
}

pub(crate) fn deploy_config_file(file_name: &str, version_key: &str) -> bool {
    let Some(file_name) = validate_data_resource_id(file_name) else {
        return false;
    };
    if version_key.is_empty() {
        return false;
    }

    let (shared_data_dir, user_data_dir, staging_dir) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned()),
            PathBuf::from(paths.user_data_dir.to_string_lossy().into_owned()),
            PathBuf::from(paths.staging_dir.to_string_lossy().into_owned()),
        )
    };
    let source = shared_data_dir.join(&file_name);
    let destination = staging_dir.join(&file_name);
    if !source.is_file() {
        return false;
    }
    if source == destination {
        return true;
    }
    let user_copy = user_data_dir.join(&file_name);
    let trash_dir = user_data_dir.join("trash");
    let _ = trash_deprecated_user_copy(&source, &user_copy, version_key, &trash_dir);
    if !deployed_config_needs_update(&destination, &file_name, &shared_data_dir, &user_data_dir) {
        return true;
    }
    if let Some(parent) = destination.parent() {
        if fs::create_dir_all(parent).is_err() {
            return false;
        }
    }
    match deployed_config_yaml_with_build_info(
        &source,
        &file_name,
        &shared_data_dir,
        &user_data_dir,
    ) {
        Some(yaml) => fs::write(destination, yaml).is_ok(),
        None => fs::copy(source, destination).is_ok(),
    }
}

fn deployed_config_needs_update(
    destination: &Path,
    file_name: &str,
    shared_data_dir: &Path,
    user_data_dir: &Path,
) -> bool {
    let root = match fs::read_to_string(destination)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
    {
        Some(root) => root,
        None => return true,
    };
    let Some(Value::Mapping(timestamps)) = find_config_value(&root, "__build_info/timestamps")
    else {
        return true;
    };
    let Some(resource_id) = normalize_config_resource_id(file_name) else {
        return true;
    };
    if !timestamps.contains_key(Value::String(resource_id.clone())) {
        return true;
    }
    let custom_resource_id = custom_patch_resource_id(&resource_id);
    if source_uses_auto_custom_patch(&shared_data_dir.join(file_name))
        && !timestamps.contains_key(Value::String(custom_resource_id.clone()))
        && config_resource_path(shared_data_dir, user_data_dir, &custom_resource_id).exists()
    {
        return true;
    }
    for (resource_id, recorded_time) in timestamps {
        let Some(resource_id) = resource_id.as_str() else {
            return true;
        };
        let Some(recorded_time) = recorded_time.as_i64() else {
            return true;
        };
        let source = config_resource_path(shared_data_dir, user_data_dir, resource_id);
        if !source.exists() {
            if recorded_time != 0 {
                return true;
            }
            continue;
        }
        let Some(source_time) = source_modified_secs(&source) else {
            return true;
        };
        if recorded_time != i64::from(source_time) {
            return true;
        }
    }
    false
}

fn deployed_config_yaml_with_build_info(
    source: &Path,
    file_name: &str,
    shared_data_dir: &Path,
    user_data_dir: &Path,
) -> Option<String> {
    let mut root = fs::read_to_string(source)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())?;
    let resource_id = normalize_config_resource_id(file_name)?;
    let timestamp = source_modified_secs(source).unwrap_or(0);
    let mut patch_dependencies = Vec::new();
    let apply_auto_custom_patch =
        apply_config_directives(&mut root, shared_data_dir, &mut patch_dependencies)?;
    apply_legacy_preset_config_plugins(
        &mut root,
        &resource_id,
        shared_data_dir,
        &mut patch_dependencies,
    )?;
    set_build_info(&mut root, &resource_id, timestamp)?;

    if apply_auto_custom_patch {
        let custom_resource_id = custom_patch_resource_id(&resource_id);
        let custom_path = user_data_dir.join(format!("{custom_resource_id}.yaml"));
        if let Some(custom_root) = fs::read_to_string(&custom_path)
            .ok()
            .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
        {
            apply_custom_patch(
                &mut root,
                &custom_root,
                shared_data_dir,
                &mut patch_dependencies,
            )?;
            set_build_info(
                &mut root,
                &custom_resource_id,
                source_modified_secs(&custom_path).unwrap_or(0),
            )?;
        } else {
            set_build_info(&mut root, &custom_resource_id, 0)?;
        }
    }
    for (resource_id, timestamp) in patch_dependencies {
        set_build_info(&mut root, &resource_id, timestamp)?;
    }
    serde_yaml::to_string(&root).ok()
}

fn custom_patch_resource_id(resource_id: &str) -> String {
    let base = resource_id.strip_suffix(".schema").unwrap_or(resource_id);
    format!("{base}.custom")
}

fn config_resource_path(
    shared_data_dir: &Path,
    user_data_dir: &Path,
    resource_id: &str,
) -> PathBuf {
    let root = if resource_id.ends_with(".custom") {
        user_data_dir
    } else {
        shared_data_dir
    };
    root.join(format!("{resource_id}.yaml"))
}

fn trash_deprecated_user_copy(
    shared_copy: &Path,
    user_copy: &Path,
    version_key: &str,
    trash_dir: &Path,
) -> bool {
    if !shared_copy.exists()
        || !user_copy.exists()
        || paths_equivalent(shared_copy, user_copy).unwrap_or(false)
    {
        return false;
    }

    let mut shared_version = config_string_from_file(shared_copy, version_key).unwrap_or_default();
    let _ = remove_version_suffix(&mut shared_version, ".minimal");
    let mut user_version = config_string_from_file(user_copy, version_key).unwrap_or_default();
    let is_customized_user_copy = remove_version_suffix(&mut user_version, ".custom.");
    if compare_version_strings(&shared_version, &user_version).is_gt()
        || (shared_version == user_version && is_customized_user_copy)
    {
        if fs::create_dir_all(trash_dir).is_err() {
            return false;
        }
        return fs::rename(
            user_copy,
            trash_dir.join(user_copy.file_name().unwrap_or_default()),
        )
        .is_ok();
    }
    false
}

fn paths_equivalent(left: &Path, right: &Path) -> Option<bool> {
    Some(left.canonicalize().ok()? == right.canonicalize().ok()?)
}

fn config_string_from_file(path: &Path, key: &str) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .and_then(|root| {
            find_config_value(&root, key)
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn remove_version_suffix(version: &mut String, suffix: &str) -> bool {
    let Some(index) = version.find(suffix) else {
        return false;
    };
    version.truncate(index);
    true
}

fn compare_version_strings(left: &str, right: &str) -> std::cmp::Ordering {
    let mut left_parts = left.split('.');
    let mut right_parts = right.split('.');
    loop {
        match (left_parts.next(), right_parts.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (Some(part), None) => {
                return compare_version_part(part, "0").then(std::cmp::Ordering::Greater);
            }
            (None, Some(part)) => {
                return compare_version_part("0", part).then(std::cmp::Ordering::Less);
            }
            (Some(left), Some(right)) => {
                let ordering = compare_version_part(left, right);
                if !ordering.is_eq() {
                    return ordering;
                }
            }
        }
    }
}

fn compare_version_part(left: &str, right: &str) -> std::cmp::Ordering {
    match (left.parse::<u64>(), right.parse::<u64>()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

pub(crate) fn deploy_schema_file(schema_file: &str) -> bool {
    let Some(schema_file) = validate_data_resource_id(schema_file) else {
        return false;
    };

    let shared_data_dir = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned())
    };
    let source = shared_data_dir.join(&schema_file);
    if !source.is_file() {
        return false;
    }

    let schema_config = match fs::read_to_string(source)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
    {
        Some(schema_config) => schema_config,
        None => return false,
    };
    let Some(schema_id) = find_config_value(&schema_config, "schema/schema_id")
        .and_then(Value::as_str)
        .and_then(validate_data_resource_id)
    else {
        return false;
    };

    deploy_config_file(&format!("{schema_id}.schema.yaml"), "schema/version")
}

pub(crate) fn prebuild_all_schemas() -> bool {
    let shared_data_dir = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        PathBuf::from(paths.shared_data_dir.to_string_lossy().into_owned())
    };
    let Ok(entries) = fs::read_dir(&shared_data_dir) else {
        return false;
    };

    let mut success = true;
    for entry in entries {
        let Ok(entry) = entry else {
            success = false;
            continue;
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) else {
            continue;
        };
        if file_name.ends_with(".schema.yaml") && !deploy_schema_file(file_name) {
            success = false;
        }
    }
    success
}
