use std::{
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
};

use serde_yaml::{Mapping, Number, Value};

use crate::{
    bool_from, c_string_key, config_state_mut, ensure_mapping, find_config_value,
    install_config_root, librime_signature_modified_time, load_runtime_config_root, runtime_paths,
    set_config_value, Bool, ConfigOpenKind, ConfigState, RimeConfig, RimeCustomSettings, FALSE,
    RIME_VERSION_BYTES, TRUE,
};

struct LeverCustomSettings {
    config_id: String,
    generator_id: String,
    config: ConfigState,
    custom_config: ConfigState,
    modified: bool,
}

/// Initializes levers custom settings for a deployed config id.
///
/// # Safety
///
/// `config_id` and `generator_id` must be valid NUL-terminated C strings.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomSettingsInit(
    config_id: *const c_char,
    generator_id: *const c_char,
) -> *mut RimeCustomSettings {
    let Some(config_id) = (unsafe { c_string_key(config_id) }) else {
        return ptr::null_mut();
    };
    let Some(generator_id) = (unsafe { c_string_key(generator_id) }) else {
        return ptr::null_mut();
    };

    Box::into_raw(Box::new(LeverCustomSettings {
        config_id,
        generator_id,
        config: ConfigState::default(),
        custom_config: ConfigState {
            root: Value::Null,
            cstring_borrows: Vec::new(),
        },
        modified: false,
    }))
    .cast::<RimeCustomSettings>()
}

/// Releases levers custom settings storage.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomSettingsDestroy(settings: *mut RimeCustomSettings) {
    if settings.is_null() {
        return;
    }
    // SAFETY: settings pointers are allocated by `RimeLeversCustomSettingsInit`.
    unsafe { drop(Box::from_raw(settings.cast::<LeverCustomSettings>())) };
}

/// Loads deployed and user custom config data for levers custom settings.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversLoadSettings(settings: *mut RimeCustomSettings) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };

    settings.config.root = load_runtime_config_root(&settings.config_id, ConfigOpenKind::Deployed);
    settings.config.cstring_borrows.clear();
    settings.modified = false;

    let path = custom_config_path(&settings.config_id);
    let loaded = fs::read_to_string(path)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok());
    match loaded {
        Some(root) => {
            settings.custom_config.root = root;
            settings.custom_config.cstring_borrows.clear();
            TRUE
        }
        None => {
            settings.custom_config.root = Value::Null;
            settings.custom_config.cstring_borrows.clear();
            FALSE
        }
    }
}

/// Saves modified levers custom settings to `<config>.custom.yaml`.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSaveSettings(settings: *mut RimeCustomSettings) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    if !settings.modified {
        return FALSE;
    }

    write_config_signature(
        &mut settings.custom_config.root,
        "customization",
        &settings.generator_id,
    );
    let path = custom_config_path(&settings.config_id);
    let Some(parent) = path.parent() else {
        return FALSE;
    };
    if fs::create_dir_all(parent).is_err() {
        return FALSE;
    }
    let Ok(yaml) = serde_yaml::to_string(&settings.custom_config.root) else {
        return FALSE;
    };
    if fs::write(path, yaml).is_err() {
        return FALSE;
    }

    settings.modified = false;
    TRUE
}

/// Writes a boolean levers custom setting under the literal `patch` key.
///
/// # Safety
///
/// `settings` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeBool(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: Bool,
) -> Bool {
    unsafe { levers_customize_value(settings, key, Value::Bool(value != FALSE)) }
}

/// Writes an integer levers custom setting under the literal `patch` key.
///
/// # Safety
///
/// `settings` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeInt(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: c_int,
) -> Bool {
    unsafe { levers_customize_value(settings, key, Value::Number(Number::from(value))) }
}

/// Writes a floating-point levers custom setting under the literal `patch` key.
///
/// # Safety
///
/// `settings` and `key` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeDouble(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: f64,
) -> Bool {
    let Ok(value) = serde_yaml::to_value(value) else {
        return FALSE;
    };
    unsafe { levers_customize_value(settings, key, value) }
}

/// Writes a string levers custom setting under the literal `patch` key.
///
/// # Safety
///
/// `settings`, `key`, and `value` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeString(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: *const c_char,
) -> Bool {
    let Some(value) = (unsafe { c_string_key(value) }) else {
        return FALSE;
    };
    unsafe { levers_customize_value(settings, key, Value::String(value)) }
}

/// Writes a list/map config item as a levers custom setting.
///
/// # Safety
///
/// `settings` and `key` must be valid pointers. `value` may be null or
/// uninitialized, in which case a null item is written.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversCustomizeItem(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: *mut RimeConfig,
) -> Bool {
    let item = if value.is_null() {
        Value::Null
    } else {
        match unsafe { config_state_mut(value) } {
            Some(value_state) => value_state.root.clone(),
            None => Value::Null,
        }
    };
    unsafe { levers_customize_value(settings, key, item) }
}

/// Reports whether the custom settings file has not yet been customized.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversIsFirstRun(settings: *mut RimeCustomSettings) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    let root = fs::read_to_string(custom_config_path(&settings.config_id))
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok());
    bool_from(
        root.as_ref()
            .and_then(|root| find_config_value(root, "customization"))
            .and_then(Value::as_mapping)
            .is_none(),
    )
}

/// Reports whether custom settings have unsaved mutations.
///
/// # Safety
///
/// `settings` must be null or a pointer returned by
/// `RimeLeversCustomSettingsInit`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSettingsIsModified(settings: *mut RimeCustomSettings) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    bool_from(settings.modified)
}

/// Copies the loaded deployed config into a caller-owned `RimeConfig`.
///
/// # Safety
///
/// `settings` and `config` must be valid pointers.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSettingsGetConfig(
    settings: *mut RimeCustomSettings,
    config: *mut RimeConfig,
) -> Bool {
    if config.is_null() {
        return FALSE;
    }
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    unsafe { install_config_root(config, settings.config.root.clone()) }
}

unsafe fn levers_custom_settings_mut(
    settings: *mut RimeCustomSettings,
) -> Option<&'static mut LeverCustomSettings> {
    if settings.is_null() {
        return None;
    }
    // SAFETY: levers custom settings pointers are allocated by
    // `RimeLeversCustomSettingsInit`.
    Some(unsafe { &mut *settings.cast::<LeverCustomSettings>() })
}

unsafe fn levers_customize_value(
    settings: *mut RimeCustomSettings,
    key: *const c_char,
    value: Value,
) -> Bool {
    let Some(settings) = (unsafe { levers_custom_settings_mut(settings) }) else {
        return FALSE;
    };
    let Some(key) = (unsafe { c_string_key(key) }) else {
        return FALSE;
    };

    let Value::Mapping(root) = ensure_mapping(&mut settings.custom_config.root) else {
        return FALSE;
    };
    let patch_key = Value::String("patch".to_owned());
    if !matches!(root.get(&patch_key), Some(Value::Mapping(_))) {
        root.insert(patch_key.clone(), Value::Mapping(Mapping::new()));
    }
    let Some(Value::Mapping(patch)) = root.get_mut(&patch_key) else {
        return FALSE;
    };
    patch.insert(Value::String(key), value);
    settings.custom_config.cstring_borrows.clear();
    settings.modified = true;
    TRUE
}

fn custom_config_path(config_id: &str) -> PathBuf {
    let config_name = config_id.strip_suffix(".schema").unwrap_or(config_id);
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    Path::new(paths.user_data_dir.to_string_lossy().as_ref())
        .join(format!("{config_name}.custom.yaml"))
}

fn write_config_signature(root: &mut Value, key: &str, generator: &str) {
    let modified_time = librime_signature_modified_time();
    let rime_version =
        String::from_utf8_lossy(&RIME_VERSION_BYTES[..RIME_VERSION_BYTES.len() - 1]).into_owned();
    let (distribution_code_name, distribution_version) = {
        let paths = runtime_paths()
            .lock()
            .expect("runtime paths should not be poisoned");
        (
            paths.distribution_code_name.to_string_lossy().into_owned(),
            paths.distribution_version.to_string_lossy().into_owned(),
        )
    };

    for (path, value) in [
        (format!("{key}/generator"), generator.to_owned()),
        (format!("{key}/modified_time"), modified_time),
        (
            format!("{key}/distribution_code_name"),
            distribution_code_name,
        ),
        (format!("{key}/distribution_version"), distribution_version),
        (format!("{key}/rime_version"), rime_version),
    ] {
        let _ = set_config_value(root, &path, Value::String(value));
    }
}
