use std::{
    collections::{HashMap, HashSet},
    ffi::{c_void, CString},
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, OnceLock},
};

use serde_yaml::{Mapping, Number, Value};

use crate::{
    bool_from, c_string_key, clear_schema_list, config_state_mut, cstring_from_lossless_str,
    ensure_mapping, find_config_value, free_schema_list_items, install_config_root,
    librime_signature_modified_time, load_runtime_config_root, non_empty_cstring_ptr,
    resource_id::validate_config_resource_id, runtime_paths, set_config_value, Bool,
    ConfigOpenKind, ConfigState, RimeConfig, RimeCustomSettings, RimeFreeSchemaList,
    RimeSchemaInfo, RimeSchemaList, RimeSchemaListItem, RimeSwitcherSettings, FALSE,
    RIME_VERSION_BYTES, TRUE,
};

struct LeverCustomSettings {
    config_id: String,
    generator_id: String,
    config: ConfigState,
    custom_config: ConfigState,
    modified: bool,
}

#[derive(Clone)]
pub(crate) struct LeverSchemaInfo {
    schema_id: CString,
    name: CString,
    version: Option<CString>,
    author: Option<CString>,
    description: Option<CString>,
    file_path: Option<CString>,
}

type SwitcherAvailableSchemaRegistry = HashMap<usize, Option<Vec<LeverSchemaInfo>>>;

fn switcher_selection_registry() -> &'static Mutex<HashMap<usize, Option<Vec<String>>>> {
    static SWITCHER_SELECTION_REGISTRY: OnceLock<Mutex<HashMap<usize, Option<Vec<String>>>>> =
        OnceLock::new();
    SWITCHER_SELECTION_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn switcher_hotkeys_registry() -> &'static Mutex<HashMap<usize, Option<CString>>> {
    static SWITCHER_HOTKEYS_REGISTRY: OnceLock<Mutex<HashMap<usize, Option<CString>>>> =
        OnceLock::new();
    SWITCHER_HOTKEYS_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn switcher_available_schema_registry() -> &'static Mutex<SwitcherAvailableSchemaRegistry> {
    static SWITCHER_AVAILABLE_SCHEMA_REGISTRY: OnceLock<Mutex<SwitcherAvailableSchemaRegistry>> =
        OnceLock::new();
    SWITCHER_AVAILABLE_SCHEMA_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

unsafe fn levers_schema_info_ptr(
    info: *mut RimeSchemaInfo,
    getter: impl FnOnce(&LeverSchemaInfo) -> Option<*const c_char>,
) -> *const c_char {
    if info.is_null() {
        return ptr::null();
    }

    // SAFETY: callers pass the opaque pointer stored in a levers schema-list
    // item's `reserved` field. That pointer is allocated as `LeverSchemaInfo`
    // and remains valid until the schema list is destroyed.
    let info = unsafe { &*info.cast::<LeverSchemaInfo>() };
    getter(info).unwrap_or(ptr::null())
}

#[no_mangle]
pub extern "C" fn RimeSwitcherSettingsInit() -> *mut RimeSwitcherSettings {
    let settings = Box::into_raw(Box::new(RimeSwitcherSettings { placeholder: 0 }));
    switcher_available_schema_registry()
        .lock()
        .expect("switcher available schema registry should not be poisoned")
        .insert(settings as usize, Some(deployed_levers_schema_infos()));
    switcher_selection_registry()
        .lock()
        .expect("switcher selection registry should not be poisoned")
        .insert(settings as usize, Some(deployed_selected_schema_ids()));
    switcher_hotkeys_registry()
        .lock()
        .expect("switcher hotkeys registry should not be poisoned")
        .insert(
            settings as usize,
            deployed_switcher_hotkeys().map(|hotkeys| cstring_from_lossless_str(&hotkeys)),
        );
    settings
}

/// Returns the deployed schema list through the librime levers module API.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `list` must be null or point to writable schema-list storage.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetAvailableSchemaList(
    settings: *mut RimeSwitcherSettings,
    list: *mut RimeSchemaList,
) -> Bool {
    if settings.is_null() || list.is_null() {
        return FALSE;
    }

    clear_schema_list(list);
    let available_schema_infos = switcher_available_schema_registry()
        .lock()
        .expect("switcher available schema registry should not be poisoned")
        .get(&(settings as usize))
        .cloned()
        .flatten()
        .unwrap_or_else(deployed_levers_schema_infos);
    populate_levers_schema_list(list, available_schema_infos)
}

/// Returns the deployed switcher selection through the librime levers module API.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `list` must be null or point to writable schema-list storage.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSelectedSchemaList(
    settings: *mut RimeSwitcherSettings,
    list: *mut RimeSchemaList,
) -> Bool {
    if settings.is_null() || list.is_null() {
        return FALSE;
    }

    clear_schema_list(list);
    let selected_schema_ids = switcher_selection_registry()
        .lock()
        .expect("switcher selection registry should not be poisoned")
        .get(&(settings as usize))
        .cloned()
        .flatten()
        .unwrap_or_else(deployed_selected_schema_ids);
    populate_schema_id_list(list, selected_schema_ids)
}

/// Returns the schema id from a levers schema-info pointer.
///
/// # Safety
///
/// `info` must be either null or a pointer returned in a levers available
/// schema-list item's `reserved` field while that list is still alive.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaId(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe { levers_schema_info_ptr(info, |info| non_empty_cstring_ptr(&info.schema_id)) }
}

/// Returns the schema name from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaName(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe { levers_schema_info_ptr(info, |info| non_empty_cstring_ptr(&info.name)) }
}

/// Returns the schema version from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaVersion(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe {
        levers_schema_info_ptr(info, |info| {
            info.version.as_ref().and_then(non_empty_cstring_ptr)
        })
    }
}

/// Returns the schema author from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaAuthor(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe {
        levers_schema_info_ptr(info, |info| {
            info.author.as_ref().and_then(non_empty_cstring_ptr)
        })
    }
}

/// Returns the schema description from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaDescription(
    info: *mut RimeSchemaInfo,
) -> *const c_char {
    unsafe {
        levers_schema_info_ptr(info, |info| {
            info.description.as_ref().and_then(non_empty_cstring_ptr)
        })
    }
}

/// Returns the schema config file path from a levers schema-info pointer.
///
/// # Safety
///
/// `info` follows the same lifetime rules as `RimeLeversGetSchemaId`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetSchemaFilePath(info: *mut RimeSchemaInfo) -> *const c_char {
    unsafe {
        levers_schema_info_ptr(info, |info| {
            info.file_path.as_ref().and_then(non_empty_cstring_ptr)
        })
    }
}

/// Selects schema IDs on the opaque switcher settings object.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `schema_id_list` must point to `count` valid NUL-terminated
/// strings when `count` is positive; non-positive counts select an empty list.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSelectSchemas(
    settings: *mut RimeSwitcherSettings,
    schema_id_list: *const *const c_char,
    count: c_int,
) -> Bool {
    if settings.is_null() || (count > 0 && schema_id_list.is_null()) {
        return FALSE;
    }

    let count = usize::try_from(count).unwrap_or(0);
    let mut selected_schema_ids = Vec::with_capacity(count);
    for index in 0..count {
        // SAFETY: callers promise `schema_id_list` has `count` readable entries
        // when count is positive.
        let schema_id = unsafe { *schema_id_list.add(index) };
        let Some(schema_id) = (unsafe { c_string_key(schema_id) }) else {
            return FALSE;
        };
        selected_schema_ids.push(schema_id);
    }

    switcher_selection_registry()
        .lock()
        .expect("switcher selection registry should not be poisoned")
        .insert(settings as usize, Some(selected_schema_ids));
    TRUE
}

/// Returns switcher hotkeys from the deployed default config.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversGetHotkeys(
    settings: *mut RimeSwitcherSettings,
) -> *const c_char {
    if settings.is_null() {
        return ptr::null();
    }

    switcher_hotkeys_registry()
        .lock()
        .expect("switcher hotkeys registry should not be poisoned")
        .get(&(settings as usize))
        .and_then(Option::as_ref)
        .map_or(ptr::null(), |hotkeys| hotkeys.as_ptr())
}

/// Matches librime's currently unimplemented switcher hotkey mutation path.
///
/// # Safety
///
/// `settings` must either be a pointer returned by `RimeSwitcherSettingsInit`
/// or null. `hotkeys`, when non-null, must point to a valid C string.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSetHotkeys(
    _settings: *mut RimeSwitcherSettings,
    _hotkeys: *const c_char,
) -> Bool {
    FALSE
}

/// Frees schema-list storage returned by levers schema-list APIs.
///
/// # Safety
///
/// `list` follows the same ownership rules as `RimeFreeSchemaList`.
#[no_mangle]
pub unsafe extern "C" fn RimeLeversSchemaListDestroy(list: *mut RimeSchemaList) {
    // SAFETY: ownership rules match `RimeFreeSchemaList`.
    unsafe { RimeFreeSchemaList(list) };
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
    let Some(config_id) = validate_config_resource_id(&config_id) else {
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

fn populate_levers_schema_list(
    schema_list: *mut RimeSchemaList,
    entries: Vec<LeverSchemaInfo>,
) -> Bool {
    if entries.is_empty() {
        return FALSE;
    }

    let mut list = Vec::with_capacity(entries.len());
    for entry in entries {
        let schema_id = entry.schema_id.as_c_str().to_owned().into_raw();
        let name = entry.name.as_c_str().to_owned().into_raw();
        let info = Box::into_raw(Box::new(entry)).cast::<c_void>();
        list.push(RimeSchemaListItem {
            schema_id,
            name,
            reserved: info,
        });
    }
    let size = list.len();
    let list_ptr = list.as_mut_ptr();
    std::mem::forget(list);

    // SAFETY: `schema_list` is non-null and points to caller-owned writable
    // storage; `list_ptr` owns `size` initialized schema-list items.
    unsafe {
        (*schema_list).size = size;
        (*schema_list).list = list_ptr;
    }
    TRUE
}

fn populate_schema_id_list(schema_list: *mut RimeSchemaList, schema_ids: Vec<String>) -> Bool {
    if schema_ids.is_empty() {
        return FALSE;
    }

    let mut list = Vec::with_capacity(schema_ids.len());
    for schema_id in schema_ids {
        let Ok(schema_id) = CString::new(schema_id) else {
            free_schema_list_items(&mut list);
            return FALSE;
        };
        list.push(RimeSchemaListItem {
            schema_id: schema_id.into_raw(),
            name: ptr::null_mut(),
            reserved: ptr::null_mut(),
        });
    }
    let size = list.len();
    let list_ptr = list.as_mut_ptr();
    std::mem::forget(list);

    // SAFETY: `schema_list` is non-null and points to caller-owned writable
    // storage; `list_ptr` owns `size` initialized schema-list items.
    unsafe {
        (*schema_list).size = size;
        (*schema_list).list = list_ptr;
    }
    TRUE
}

fn deployed_levers_schema_infos() -> Vec<LeverSchemaInfo> {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    let roots = [
        paths.shared_data_dir.to_string_lossy().into_owned(),
        paths.user_data_dir.to_string_lossy().into_owned(),
    ];
    drop(paths);

    let mut seen = HashSet::new();
    let mut infos = Vec::new();
    for root in roots {
        for path in schema_file_paths_in_dir(&root) {
            let Some((schema_id, schema_config)) = levers_schema_config_from_file(&path) else {
                continue;
            };
            if !seen.insert(schema_id.clone()) {
                continue;
            }
            infos.push(levers_schema_info(schema_id, schema_config, Some(path)));
        }
    }
    infos
}

fn schema_file_paths_in_dir(root: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().ends_with(".schema.yaml"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn levers_schema_config_from_file(path: &Path) -> Option<(String, Value)> {
    let yaml = fs::read_to_string(path).ok()?;
    let schema_config = serde_yaml::from_str::<Value>(&yaml).ok()?;
    let schema_id = find_config_value(&schema_config, "schema/schema_id")?
        .as_str()?
        .to_owned();
    find_config_value(&schema_config, "schema/name")?.as_str()?;
    Some((schema_id, schema_config))
}

fn levers_schema_info(
    schema_id: String,
    schema_config: Value,
    file_path: Option<PathBuf>,
) -> LeverSchemaInfo {
    let name = find_config_value(&schema_config, "schema/name")
        .and_then(Value::as_str)
        .unwrap_or(&schema_id)
        .to_owned();
    let version = find_config_value(&schema_config, "schema/version")
        .and_then(Value::as_str)
        .map(cstring_from_lossless_str);
    let author =
        levers_schema_author(&schema_config).map(|author| cstring_from_lossless_str(&author));
    let description = find_config_value(&schema_config, "schema/description")
        .and_then(Value::as_str)
        .map(cstring_from_lossless_str);
    let file_path =
        file_path.map(|path| cstring_from_lossless_str(path.to_string_lossy().as_ref()));

    LeverSchemaInfo {
        schema_id: cstring_from_lossless_str(&schema_id),
        name: cstring_from_lossless_str(&name),
        version,
        author,
        description,
        file_path,
    }
}

fn levers_schema_author(schema_config: &Value) -> Option<String> {
    let author = find_config_value(schema_config, "schema/author")?;
    match author {
        Value::Sequence(authors) => {
            let joined = authors
                .iter()
                .filter_map(Value::as_str)
                .filter(|author| !author.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        }
        Value::String(author) if !author.is_empty() => Some(author.clone()),
        _ => None,
    }
}

pub(crate) fn deployed_selected_schema_ids() -> Vec<String> {
    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Some(schema_list) = find_config_value(&default_config, "schema_list") else {
        return Vec::new();
    };
    let Value::Sequence(schema_list) = schema_list else {
        return Vec::new();
    };

    schema_list
        .iter()
        .filter_map(|entry| {
            let Value::Mapping(entry) = entry else {
                return None;
            };
            entry
                .get(Value::String("schema".to_owned()))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn deployed_switcher_hotkeys() -> Option<String> {
    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Value::Sequence(hotkeys) = find_config_value(&default_config, "switcher/hotkeys")? else {
        return None;
    };

    let hotkeys = hotkeys
        .iter()
        .filter_map(Value::as_str)
        .filter(|hotkey| !hotkey.is_empty())
        .collect::<Vec<_>>();
    if hotkeys.is_empty() {
        None
    } else {
        Some(hotkeys.join(", "))
    }
}
