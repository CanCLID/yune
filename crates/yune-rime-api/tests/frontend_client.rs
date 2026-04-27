use std::{
    ffi::{c_void, CStr, CString},
    fs, mem,
    os::raw::{c_char, c_int},
    path::PathBuf,
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use yune_rime_api::{
    rime_get_api, RimeCandidate, RimeCandidateListIterator, RimeCommit, RimeComposition,
    RimeConfig, RimeConfigIterator, RimeContext, RimeCustomApi, RimeLeversApi, RimeMenu,
    RimeModule, RimeSchemaList, RimeSessionId, RimeStatus, RimeTraits, RimeUserDictIterator, FALSE,
    TRUE,
};

use serde_yaml::Value;

#[derive(Debug, PartialEq, Eq)]
struct NotificationEvent {
    context_object: usize,
    session_id: RimeSessionId,
    message_type: String,
    message_value: String,
}

fn empty_context() -> RimeContext {
    RimeContext {
        data_size: (mem::size_of::<RimeContext>() - mem::size_of::<i32>()) as i32,
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
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        },
        commit_text_preview: ptr::null_mut(),
        select_labels: ptr::null_mut(),
    }
}

fn empty_status() -> RimeStatus {
    RimeStatus {
        data_size: (mem::size_of::<RimeStatus>() - mem::size_of::<i32>()) as i32,
        schema_id: ptr::null_mut(),
        schema_name: ptr::null_mut(),
        is_disabled: FALSE,
        is_composing: FALSE,
        is_ascii_mode: FALSE,
        is_full_shape: FALSE,
        is_simplified: FALSE,
        is_traditional: FALSE,
        is_ascii_punct: FALSE,
    }
}

fn empty_traits() -> RimeTraits {
    RimeTraits {
        data_size: mem::size_of::<RimeTraits>() as i32,
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

fn empty_commit() -> RimeCommit {
    RimeCommit {
        data_size: (mem::size_of::<RimeCommit>() - mem::size_of::<i32>()) as i32,
        text: ptr::null_mut(),
    }
}

fn empty_candidate_list_iterator() -> RimeCandidateListIterator {
    RimeCandidateListIterator {
        ptr: ptr::null_mut(),
        index: 0,
        candidate: RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        },
    }
}

fn empty_schema_list() -> RimeSchemaList {
    RimeSchemaList {
        size: 0,
        list: ptr::null_mut(),
    }
}

fn empty_config() -> RimeConfig {
    RimeConfig {
        ptr: ptr::null_mut(),
    }
}

fn empty_user_dict_iterator() -> RimeUserDictIterator {
    RimeUserDictIterator {
        ptr: ptr::null_mut(),
        i: 0,
    }
}

fn yaml_mapping_value<'a>(mapping: &'a serde_yaml::Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_owned()))
}

fn empty_config_iterator() -> RimeConfigIterator {
    RimeConfigIterator {
        list: ptr::null_mut(),
        map: ptr::null_mut(),
        index: 0,
        key: ptr::null(),
        path: ptr::null(),
    }
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned")
}

fn notification_events() -> &'static Mutex<Vec<NotificationEvent>> {
    static NOTIFICATION_EVENTS: OnceLock<Mutex<Vec<NotificationEvent>>> = OnceLock::new();
    NOTIFICATION_EVENTS.get_or_init(|| Mutex::new(Vec::new()))
}

extern "C" fn record_notification(
    context_object: *mut c_void,
    session_id: RimeSessionId,
    message_type: *const c_char,
    message_value: *const c_char,
) {
    // SAFETY: the shim invokes handlers with valid NUL-terminated message
    // strings for the duration of the callback.
    let message_type = unsafe { CStr::from_ptr(message_type) }
        .to_string_lossy()
        .into_owned();
    // SAFETY: same as above.
    let message_value = unsafe { CStr::from_ptr(message_value) }
        .to_string_lossy()
        .into_owned();
    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .push(NotificationEvent {
            context_object: context_object as usize,
            session_id,
            message_type,
            message_value,
        });
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "yune-rime-api-frontend-{label}-{}-{nanos}",
        std::process::id()
    ))
}

extern "C" fn frontend_module_initialize() {}

extern "C" fn frontend_module_finalize() {}

extern "C" fn frontend_module_get_api() -> *mut RimeCustomApi {
    ptr::null_mut()
}

#[test]
fn frontend_style_api_table_can_read_schema_lists_and_modules() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let get_schema_list = api
        .get_schema_list
        .expect("frontend requires get_schema_list");
    let free_schema_list = api
        .free_schema_list
        .expect("frontend requires free_schema_list");
    let register_module = api
        .register_module
        .expect("frontend requires register_module");
    let find_module = api.find_module.expect("frontend requires find_module");

    let root = unique_temp_dir("schema-list-module");
    let shared = root.join("shared");
    let user = root.join("user");
    let prebuilt = shared.join("build");
    let staging = user.join("build");
    fs::create_dir_all(&prebuilt).expect("prebuilt dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        prebuilt.join("default.yaml"),
        "\
schema_list:
  - schema: prebuilt_only
",
    )
    .expect("prebuilt default config should be written");
    fs::write(
        staging.join("default.yaml"),
        "\
schema_list:
  - schema: luna_pinyin
  - schema: cangjie5
    case: [conditions/include_cangjie]
  - schema: hidden
    case: [conditions/include_hidden]
  - schema: missing_name
  - not_schema: ignored
conditions:
  include_cangjie: true
  include_hidden: false
",
    )
    .expect("staging default config should be written");
    fs::write(
        staging.join("luna_pinyin.schema.yaml"),
        "schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\n",
    )
    .expect("luna schema config should be written");
    fs::write(
        prebuilt.join("cangjie5.schema.yaml"),
        "schema:\n  schema_id: cangjie5\n  name: Cangjie 5\n",
    )
    .expect("cangjie schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let mut schema_list = empty_schema_list();
    assert_eq!(unsafe { get_schema_list(&mut schema_list) }, TRUE);
    assert_eq!(schema_list.size, 4);
    assert!(!schema_list.list.is_null());

    let mut actual = Vec::new();
    for index in 0..schema_list.size {
        let item = unsafe { *schema_list.list.add(index) };
        let schema_id = unsafe { CStr::from_ptr(item.schema_id) };
        let name = unsafe { CStr::from_ptr(item.name) };
        actual.push((
            schema_id.to_string_lossy().into_owned(),
            name.to_string_lossy().into_owned(),
        ));
        assert!(item.reserved.is_null());
    }
    assert_eq!(
        actual,
        vec![
            ("luna_pinyin".to_owned(), "Luna Pinyin".to_owned()),
            ("cangjie5".to_owned(), "Cangjie 5".to_owned()),
            ("hidden".to_owned(), "hidden".to_owned()),
            ("missing_name".to_owned(), "missing_name".to_owned()),
        ]
    );

    unsafe { free_schema_list(&mut schema_list) };
    assert_eq!(schema_list.size, 0);
    assert!(schema_list.list.is_null());
    assert_eq!(unsafe { get_schema_list(ptr::null_mut()) }, FALSE);
    unsafe { free_schema_list(ptr::null_mut()) };

    let module_name = CString::new(format!(
        "frontend_module_{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after Unix epoch")
            .as_nanos()
    ))
    .expect("module name should be valid");
    let module_name_ptr = module_name.into_raw();
    let module = Box::leak(Box::new(RimeModule {
        data_size: mem::size_of::<RimeModule>() as i32,
        module_name: module_name_ptr,
        initialize: Some(frontend_module_initialize),
        finalize: Some(frontend_module_finalize),
        get_api: Some(frontend_module_get_api),
    }));
    let module_ptr = module as *mut RimeModule;
    assert_eq!(unsafe { register_module(module_ptr) }, TRUE);
    assert_eq!(unsafe { find_module(module_name_ptr) }, module_ptr);
    assert!(module.initialize.is_some());
    assert!(module.finalize.is_some());
    assert_eq!(
        module.get_api.expect("module api getter exists")(),
        ptr::null_mut()
    );

    let missing_module = CString::new("frontend_missing_module").expect("literal should be valid");
    assert!(unsafe { find_module(missing_module.as_ptr()) }.is_null());
    assert_eq!(unsafe { register_module(ptr::null_mut()) }, FALSE);
    assert!(unsafe { find_module(ptr::null()) }.is_null());

    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_use_builtin_levers_module() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let find_module = api.find_module.expect("frontend requires find_module");

    let root = unique_temp_dir("builtin-levers");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
schema_list:
  - schema: luna_pinyin
  - schema: cangjie5
switcher:
  hotkeys:
    - Control+grave
    - F4
    - ''
",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("luna_pinyin.schema.yaml"),
        "\
schema:
  schema_id: luna_pinyin
  name: Luna Pinyin
  version: '1.0'
  author:
    - Author One
    - Author Two
  description: Sample schema
",
    )
    .expect("luna schema config should be written");
    fs::write(
        shared.join("cangjie5.schema.yaml"),
        "schema:\n  schema_id: cangjie5\n  name: Cangjie 5\n",
    )
    .expect("cangjie schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    let module = unsafe { find_module(levers_name.as_ptr()) };
    assert!(!module.is_null());
    let module = unsafe { &*module };
    let get_api = module.get_api.expect("levers module should expose get_api");
    let levers_api = get_api().cast::<RimeLeversApi>();
    assert!(!levers_api.is_null());
    let levers_api = unsafe { &*levers_api };
    assert_eq!(
        levers_api.data_size,
        (mem::size_of::<RimeLeversApi>() - mem::size_of::<i32>()) as i32
    );

    let switcher_settings_init = levers_api
        .switcher_settings_init
        .expect("levers API should expose switcher settings init");
    let get_available = levers_api
        .get_available_schema_list
        .expect("levers API should expose available schema list");
    let get_selected = levers_api
        .get_selected_schema_list
        .expect("levers API should expose selected schema list");
    let get_schema_author = levers_api
        .get_schema_author
        .expect("levers API should expose schema author getter");
    let select_schemas = levers_api
        .select_schemas
        .expect("levers API should expose schema selection");
    let get_hotkeys = levers_api
        .get_hotkeys
        .expect("levers API should expose hotkey lookup");
    let set_hotkeys = levers_api
        .set_hotkeys
        .expect("levers API should expose hotkey mutation");
    let destroy = levers_api
        .schema_list_destroy
        .expect("levers API should expose schema list destroy");

    let settings = switcher_settings_init();
    assert!(!settings.is_null());
    let mut available = empty_schema_list();
    assert_eq!(unsafe { get_available(settings, &mut available) }, TRUE);
    assert_eq!(available.size, 2);
    let first_available = unsafe { *available.list };
    assert!(!first_available.reserved.is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(first_available.schema_id) }.to_str(),
        Ok("cangjie5")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(first_available.name) }.to_str(),
        Ok("Cangjie 5")
    );
    let second_available = unsafe { *available.list.add(1) };
    assert_eq!(
        unsafe { CStr::from_ptr(second_available.schema_id) }.to_str(),
        Ok("luna_pinyin")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(second_available.name) }.to_str(),
        Ok("Luna Pinyin")
    );
    let author = unsafe { get_schema_author(second_available.reserved.cast()) };
    assert_eq!(
        unsafe { CStr::from_ptr(author) }.to_str(),
        Ok("Author One\nAuthor Two")
    );

    let mut selected = empty_schema_list();
    assert_eq!(unsafe { get_selected(settings, &mut selected) }, TRUE);
    assert_eq!(selected.size, 2);
    let first_selected = unsafe { *selected.list };
    assert_eq!(
        unsafe { CStr::from_ptr(first_selected.schema_id) }.to_str(),
        Ok("luna_pinyin")
    );
    assert!(first_selected.name.is_null());
    assert!(first_selected.reserved.is_null());

    let hotkeys = unsafe { get_hotkeys(settings) };
    assert!(!hotkeys.is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(hotkeys) }.to_str(),
        Ok("Control+grave, F4")
    );

    let selected_cangjie = CString::new("cangjie5").expect("schema id should be valid");
    let selected_luna = CString::new("luna_pinyin").expect("schema id should be valid");
    let schema_ids = [selected_cangjie.as_ptr(), selected_luna.as_ptr()];
    assert_eq!(
        unsafe { select_schemas(settings, schema_ids.as_ptr(), schema_ids.len() as c_int) },
        TRUE
    );
    let mut overridden_selected = empty_schema_list();
    assert_eq!(
        unsafe { get_selected(settings, &mut overridden_selected) },
        TRUE
    );
    assert_eq!(overridden_selected.size, 2);
    let overridden_first = unsafe { *overridden_selected.list };
    let overridden_second = unsafe { *overridden_selected.list.add(1) };
    assert_eq!(
        unsafe { CStr::from_ptr(overridden_first.schema_id) }.to_str(),
        Ok("cangjie5")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(overridden_second.schema_id) }.to_str(),
        Ok("luna_pinyin")
    );
    assert!(overridden_first.name.is_null());
    assert!(overridden_first.reserved.is_null());
    assert!(overridden_second.name.is_null());
    assert!(overridden_second.reserved.is_null());
    let new_hotkeys = CString::new("Alt+space").expect("hotkeys should be valid");
    assert_eq!(
        unsafe { set_hotkeys(settings, new_hotkeys.as_ptr()) },
        FALSE
    );
    assert!(unsafe { get_hotkeys(ptr::null_mut()) }.is_null());
    assert_eq!(unsafe { select_schemas(settings, ptr::null(), 1) }, FALSE);

    unsafe { destroy(&mut overridden_selected) };
    assert_eq!(overridden_selected.size, 0);
    assert!(overridden_selected.list.is_null());
    unsafe { destroy(&mut selected) };
    assert_eq!(selected.size, 0);
    assert!(selected.list.is_null());
    unsafe { destroy(&mut available) };
    assert_eq!(available.size, 0);
    assert!(available.list.is_null());
    unsafe { drop(Box::from_raw(settings)) };

    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_customize_levers_settings() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let find_module = api.find_module.expect("frontend requires find_module");
    let config_get_int = api
        .config_get_int
        .expect("frontend requires config_get_int");
    let config_get_string = api
        .config_get_string
        .expect("frontend requires config_get_string");
    let config_load_string = api
        .config_load_string
        .expect("frontend requires config_load_string");
    let config_close = api.config_close.expect("frontend requires config_close");

    let root = unique_temp_dir("levers-custom-settings");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna_pinyin.schema.yaml"),
        "\
schema:
  schema_id: luna_pinyin
  name: Luna Pinyin
menu:
  page_size: 5
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    traits.distribution_code_name = c"frontend_dist".as_ptr();
    traits.distribution_version = c"2026.04".as_ptr();
    unsafe { setup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    let module = unsafe { find_module(levers_name.as_ptr()) };
    assert!(!module.is_null());
    let module = unsafe { &*module };
    let get_api = module.get_api.expect("levers module should expose get_api");
    let levers_api = get_api().cast::<RimeLeversApi>();
    assert!(!levers_api.is_null());
    let levers_api = unsafe { &*levers_api };

    let custom_settings_init = levers_api
        .custom_settings_init
        .expect("levers API should expose custom settings init");
    let custom_settings_destroy = levers_api
        .custom_settings_destroy
        .expect("levers API should expose custom settings destroy");
    let load_settings = levers_api
        .load_settings
        .expect("levers API should expose load settings");
    let save_settings = levers_api
        .save_settings
        .expect("levers API should expose save settings");
    let customize_bool = levers_api
        .customize_bool
        .expect("levers API should expose bool customization");
    let customize_int = levers_api
        .customize_int
        .expect("levers API should expose int customization");
    let customize_string = levers_api
        .customize_string
        .expect("levers API should expose string customization");
    let customize_item = levers_api
        .customize_item
        .expect("levers API should expose item customization");
    let is_first_run = levers_api
        .is_first_run
        .expect("levers API should expose first-run state");
    let settings_is_modified = levers_api
        .settings_is_modified
        .expect("levers API should expose modified state");
    let settings_get_config = levers_api
        .settings_get_config
        .expect("levers API should expose deployed config access");

    let config_id = CString::new("luna_pinyin.schema").expect("config id should be valid");
    let generator = CString::new("frontend-client").expect("generator should be valid");
    let settings = unsafe { custom_settings_init(config_id.as_ptr(), generator.as_ptr()) };
    assert!(!settings.is_null());

    assert_eq!(unsafe { load_settings(settings) }, FALSE);
    assert_eq!(unsafe { is_first_run(settings) }, TRUE);
    assert_eq!(unsafe { settings_is_modified(settings) }, FALSE);

    let mut loaded_config = empty_config();
    assert_eq!(
        unsafe { settings_get_config(settings, &mut loaded_config) },
        TRUE
    );
    let schema_name_key = CString::new("schema/name").expect("config key should be valid");
    let page_size_key = CString::new("menu/page_size").expect("config key should be valid");
    let mut string_output = [0 as c_char; 64];
    assert_eq!(
        unsafe {
            config_get_string(
                &mut loaded_config,
                schema_name_key.as_ptr(),
                string_output.as_mut_ptr(),
                string_output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(string_output.as_ptr()) }.to_str(),
        Ok("Luna Pinyin")
    );
    let mut int_output = 0;
    assert_eq!(
        unsafe { config_get_int(&mut loaded_config, page_size_key.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 5);

    let bool_key = CString::new("switches/@0/reset").expect("custom key should be valid");
    let int_key = CString::new("menu/page_size").expect("custom key should be valid");
    let string_key = CString::new("schema/name").expect("custom key should be valid");
    let string_value = CString::new("Frontend Luna").expect("custom value should be valid");
    assert_eq!(
        unsafe { customize_bool(settings, bool_key.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { customize_int(settings, int_key.as_ptr(), 9) },
        TRUE
    );
    assert_eq!(
        unsafe { customize_string(settings, string_key.as_ptr(), string_value.as_ptr()) },
        TRUE
    );

    let mut hotkey_config = empty_config();
    let hotkey_yaml = CString::new("- Control+grave\n- F4\n").expect("yaml should be valid");
    assert_eq!(
        unsafe { config_load_string(&mut hotkey_config, hotkey_yaml.as_ptr()) },
        TRUE
    );
    let hotkey_key = CString::new("switcher/hotkeys").expect("custom key should be valid");
    assert_eq!(
        unsafe { customize_item(settings, hotkey_key.as_ptr(), &mut hotkey_config) },
        TRUE
    );
    assert_eq!(unsafe { settings_is_modified(settings) }, TRUE);
    assert_eq!(unsafe { save_settings(settings) }, TRUE);
    assert_eq!(unsafe { settings_is_modified(settings) }, FALSE);
    assert_eq!(unsafe { save_settings(settings) }, FALSE);
    assert_eq!(unsafe { is_first_run(settings) }, FALSE);

    let saved = fs::read_to_string(user.join("luna_pinyin.custom.yaml"))
        .expect("custom settings should be saved without .schema suffix");
    let saved_root: Value = serde_yaml::from_str(&saved).expect("custom settings should parse");
    let patch = saved_root
        .get("patch")
        .and_then(Value::as_mapping)
        .expect("patch map should be present");
    assert_eq!(
        yaml_mapping_value(patch, "switches/@0/reset").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        yaml_mapping_value(patch, "menu/page_size").and_then(Value::as_i64),
        Some(9)
    );
    assert_eq!(
        yaml_mapping_value(patch, "schema/name").and_then(Value::as_str),
        Some("Frontend Luna")
    );
    assert!(matches!(
        yaml_mapping_value(patch, "switcher/hotkeys"),
        Some(Value::Sequence(values)) if values.len() == 2
    ));
    let customization = saved_root
        .get("customization")
        .and_then(Value::as_mapping)
        .expect("customization signature should be present");
    assert_eq!(
        yaml_mapping_value(customization, "generator").and_then(Value::as_str),
        Some("frontend-client")
    );
    assert_eq!(
        yaml_mapping_value(customization, "distribution_code_name").and_then(Value::as_str),
        Some("frontend_dist")
    );

    assert_eq!(unsafe { config_close(&mut loaded_config) }, TRUE);
    assert_eq!(unsafe { config_close(&mut hotkey_config) }, TRUE);
    unsafe { custom_settings_destroy(settings) };
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_manage_levers_user_dicts() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let find_module = api.find_module.expect("frontend requires find_module");

    let root = unique_temp_dir("levers-user-dicts");
    let user = root.join("user");
    let sync = root.join("sync");
    fs::create_dir_all(user.join("luna_pinyin.userdb"))
        .expect("leveldb-style user dict dir should be created");
    fs::write(
        user.join("essay.userdb"),
        "# comment\nni hao\t你好\t1\n\nzhong guo\t中国\t2\n",
    )
    .expect("plain user dict should be written");
    fs::write(user.join("legacy.userdb.txt"), "")
        .expect("legacy text snapshot should not be listed");
    fs::write(
        user.join("installation.yaml"),
        format!(
            "installation_id: frontend-device\nsync_dir: '{}'\n",
            sync.to_string_lossy()
        ),
    )
    .expect("installation metadata should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    let module = unsafe { find_module(levers_name.as_ptr()) };
    assert!(!module.is_null());
    let module = unsafe { &*module };
    let get_api = module.get_api.expect("levers module should expose get_api");
    let levers_api = get_api().cast::<RimeLeversApi>();
    assert!(!levers_api.is_null());
    let levers_api = unsafe { &*levers_api };

    let iterator_init = levers_api
        .user_dict_iterator_init
        .expect("levers API should expose user dict iterator init");
    let iterator_destroy = levers_api
        .user_dict_iterator_destroy
        .expect("levers API should expose user dict iterator destroy");
    let next_user_dict = levers_api
        .next_user_dict
        .expect("levers API should expose next user dict");
    let backup_user_dict = levers_api
        .backup_user_dict
        .expect("levers API should expose user dict backup");
    let restore_user_dict = levers_api
        .restore_user_dict
        .expect("levers API should expose user dict restore");
    let export_user_dict = levers_api
        .export_user_dict
        .expect("levers API should expose user dict export");
    let import_user_dict = levers_api
        .import_user_dict
        .expect("levers API should expose user dict import");

    let mut iterator = empty_user_dict_iterator();
    assert_eq!(unsafe { iterator_init(&mut iterator) }, TRUE);
    assert!(!iterator.ptr.is_null());
    assert_eq!(iterator.i, 0);
    let first = unsafe { next_user_dict(&mut iterator) };
    assert!(!first.is_null());
    assert_eq!(unsafe { CStr::from_ptr(first) }.to_str(), Ok("essay"));
    let second = unsafe { next_user_dict(&mut iterator) };
    assert!(!second.is_null());
    assert_eq!(
        unsafe { CStr::from_ptr(second) }.to_str(),
        Ok("luna_pinyin")
    );
    assert!(unsafe { next_user_dict(&mut iterator) }.is_null());
    unsafe { iterator_destroy(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert_eq!(iterator.i, 0);

    let dict_name = CString::new("essay").expect("dict name is valid");
    assert_eq!(unsafe { backup_user_dict(dict_name.as_ptr()) }, TRUE);
    let snapshot = sync.join("frontend-device").join("essay.userdb.txt");
    assert_eq!(
        fs::read_to_string(&snapshot).expect("snapshot should be readable"),
        fs::read_to_string(user.join("essay.userdb")).expect("user dict should be readable")
    );

    let export_path = root.join("essay_export.tsv");
    let export_path_c =
        CString::new(export_path.to_string_lossy().as_ref()).expect("path is valid");
    assert_eq!(
        unsafe { export_user_dict(dict_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    assert_eq!(
        fs::read_to_string(&export_path).expect("export should be readable"),
        fs::read_to_string(user.join("essay.userdb")).expect("user dict should be readable")
    );

    fs::write(&export_path, "xin\t新\t3\nci\t词\t4\n").expect("import source should be updated");
    let imported_name = CString::new("frontend_imported").expect("dict name is valid");
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    assert_eq!(
        fs::read_to_string(user.join("frontend_imported.userdb"))
            .expect("imported dict should be readable"),
        "xin\t新\t3\nci\t词\t4\n"
    );

    let snapshot_c = CString::new(snapshot.to_string_lossy().as_ref()).expect("path is valid");
    fs::remove_file(user.join("essay.userdb")).expect("user dict should be removable");
    assert_eq!(unsafe { restore_user_dict(snapshot_c.as_ptr()) }, TRUE);
    assert!(user.join("essay.userdb").is_file());

    assert_eq!(unsafe { iterator_init(ptr::null_mut()) }, FALSE);
    assert!(unsafe { next_user_dict(ptr::null_mut()) }.is_null());
    unsafe { iterator_destroy(ptr::null_mut()) };
    let missing_name = CString::new("missing").expect("dict name is valid");
    let missing_snapshot = root.join("missing.userdb.txt");
    let missing_snapshot_c =
        CString::new(missing_snapshot.to_string_lossy().as_ref()).expect("path is valid");
    assert_eq!(unsafe { backup_user_dict(ptr::null()) }, FALSE);
    assert_eq!(unsafe { backup_user_dict(missing_name.as_ptr()) }, FALSE);
    assert_eq!(unsafe { restore_user_dict(ptr::null()) }, FALSE);
    assert_eq!(
        unsafe { restore_user_dict(missing_snapshot_c.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { export_user_dict(ptr::null(), export_path_c.as_ptr()) },
        -1
    );
    assert_eq!(
        unsafe { export_user_dict(missing_name.as_ptr(), export_path_c.as_ptr()) },
        -1
    );
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), ptr::null()) },
        -1
    );
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), missing_snapshot_c.as_ptr()) },
        -1
    );

    fs::remove_file(user.join("essay.userdb")).expect("restored user dict should be removable");
    fs::remove_file(user.join("frontend_imported.userdb"))
        .expect("imported user dict should be removable");
    fs::remove_dir_all(user.join("luna_pinyin.userdb"))
        .expect("leveldb-style user dict dir should be removable");
    let mut empty_iterator = empty_user_dict_iterator();
    empty_iterator.i = 7;
    assert_eq!(unsafe { iterator_init(&mut empty_iterator) }, FALSE);
    assert!(empty_iterator.ptr.is_null());
    assert_eq!(empty_iterator.i, 7);

    fs::write(user.join("cached.userdb"), "").expect("cached user dict should be written");
    let mut cached_iterator = empty_user_dict_iterator();
    assert_eq!(unsafe { iterator_init(&mut cached_iterator) }, TRUE);
    assert!(!cached_iterator.ptr.is_null());
    assert_eq!(cached_iterator.i, 0);
    fs::remove_file(user.join("cached.userdb")).expect("cached user dict should be removed");
    assert_eq!(unsafe { iterator_init(&mut cached_iterator) }, FALSE);
    assert!(!cached_iterator.ptr.is_null());
    assert_eq!(cached_iterator.i, 0);
    let cached = unsafe { next_user_dict(&mut cached_iterator) };
    assert!(!cached.is_null());
    assert_eq!(unsafe { CStr::from_ptr(cached) }.to_str(), Ok("cached"));
    unsafe { iterator_destroy(&mut cached_iterator) };

    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_receive_runtime_notifications() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let set_notification_handler = api
        .set_notification_handler
        .expect("frontend requires set_notification_handler");
    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    let set_option = api.set_option.expect("frontend requires set_option");
    let set_property = api.set_property.expect("frontend requires set_property");
    let select_schema = api.select_schema.expect("frontend requires select_schema");
    let deploy = api.deploy.expect("frontend requires deploy");

    cleanup_all_sessions();
    let root = unique_temp_dir("notifications");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: sample_schema\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("sample_schema.schema.yaml"),
        "schema:\n  schema_id: sample_schema\n  name: Sample\n",
    )
    .expect("shared schema should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    notification_events()
        .lock()
        .expect("notification events should not be poisoned")
        .clear();
    let session_id = create_session();
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let property = CString::new("client_app").expect("property name should be valid");
    let property_value = CString::new("frontend_client").expect("property value should be valid");
    let schema_id = CString::new("sample_schema").expect("schema id should be valid");
    let context_object = 0x7b_usize as *mut c_void;

    set_notification_handler(Some(record_notification), context_object);
    unsafe {
        set_option(session_id, ascii_mode.as_ptr(), TRUE);
        set_option(session_id, ascii_mode.as_ptr(), FALSE);
        set_property(session_id, property.as_ptr(), property_value.as_ptr());
        assert_eq!(select_schema(session_id, schema_id.as_ptr()), TRUE);
    }
    assert_eq!(deploy(), TRUE);

    let events = notification_events()
        .lock()
        .expect("notification events should not be poisoned");
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x7b,
                session_id,
                message_type: "option".to_owned(),
                message_value: "ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id,
                message_type: "option".to_owned(),
                message_value: "!ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id,
                message_type: "property".to_owned(),
                message_value: "client_app=frontend_client".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id,
                message_type: "schema".to_owned(),
                message_value: "sample_schema/sample_schema".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "start".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "success".to_owned(),
            },
        ]
    );
    drop(events);

    set_notification_handler(None, ptr::null_mut());
    unsafe { set_option(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(
        notification_events()
            .lock()
            .expect("notification events should not be poisoned")
            .len(),
        6
    );

    assert_eq!(destroy_session(session_id), TRUE);
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_open_runtime_configs() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let config_open = api.config_open.expect("frontend requires config_open");
    let schema_open = api.schema_open.expect("frontend requires schema_open");
    let user_config_open = api
        .user_config_open
        .expect("frontend requires user_config_open");
    let config_get_string = api
        .config_get_string
        .expect("frontend requires config_get_string");
    let config_get_int = api
        .config_get_int
        .expect("frontend requires config_get_int");
    let config_close = api.config_close.expect("frontend requires config_close");

    let root = unique_temp_dir("config-open");
    let shared = root.join("shared");
    let user = root.join("user");
    let prebuilt = shared.join("build");
    let staging = user.join("build");
    fs::create_dir_all(&prebuilt).expect("prebuilt dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        prebuilt.join("default.yaml"),
        "schema:\n  name: Prebuilt Default\nmenu:\n  page_size: 5\n",
    )
    .expect("prebuilt config should be written");
    fs::write(
        staging.join("default.yaml"),
        "schema:\n  name: Staging Default\nmenu:\n  page_size: 7\n",
    )
    .expect("staging config should be written");
    fs::write(
        staging.join("luna.schema.yaml"),
        "schema:\n  schema_id: luna\n  name: Luna\n",
    )
    .expect("schema config should be written");
    fs::write(user.join("user.yaml"), "var:\n  option: custom\n")
        .expect("user config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let mut config = empty_config();
    let default_id = CString::new("default").expect("literal should not contain NUL");
    let default_file_id = CString::new("default.yaml").expect("literal should not contain NUL");
    let schema_id = CString::new("luna").expect("literal should not contain NUL");
    let user_id = CString::new("user").expect("literal should not contain NUL");
    let missing_id = CString::new("missing").expect("literal should not contain NUL");
    let schema_name_key = CString::new("schema/name").expect("literal should not contain NUL");
    let page_size_key = CString::new("menu/page_size").expect("literal should not contain NUL");
    let option_key = CString::new("var/option").expect("literal should not contain NUL");
    let mut buffer = vec![0 as c_char; 32];

    assert_eq!(
        unsafe { config_open(default_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                schema_name_key.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        TRUE
    );
    let schema_name = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(schema_name.to_str(), Ok("Staging Default"));
    let mut page_size = 0;
    assert_eq!(
        unsafe { config_get_int(&mut config, page_size_key.as_ptr(), &mut page_size) },
        TRUE
    );
    assert_eq!(page_size, 7);
    assert_eq!(unsafe { config_close(&mut config) }, TRUE);

    assert_eq!(
        unsafe { config_open(default_file_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        unsafe { config_get_int(&mut config, page_size_key.as_ptr(), &mut page_size) },
        TRUE
    );
    assert_eq!(page_size, 7);
    assert_eq!(unsafe { config_close(&mut config) }, TRUE);

    assert_eq!(
        unsafe { schema_open(schema_id.as_ptr(), &mut config) },
        TRUE
    );
    buffer.fill(0);
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                schema_name_key.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        TRUE
    );
    let schema_name = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(schema_name.to_str(), Ok("Luna"));
    assert_eq!(unsafe { config_close(&mut config) }, TRUE);

    assert_eq!(
        unsafe { user_config_open(user_id.as_ptr(), &mut config) },
        TRUE
    );
    buffer.fill(0);
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                option_key.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        TRUE
    );
    let user_option = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(user_option.to_str(), Ok("custom"));
    assert_eq!(unsafe { config_close(&mut config) }, TRUE);

    assert_eq!(
        unsafe { config_open(missing_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                schema_name_key.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        FALSE
    );
    assert_eq!(unsafe { config_close(&mut config) }, TRUE);

    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_run_deployment_and_maintenance() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let deployer_initialize = api
        .deployer_initialize
        .expect("frontend requires deployer_initialize");
    let start_maintenance = api
        .start_maintenance
        .expect("frontend requires start_maintenance");
    let is_maintenance_mode = api
        .is_maintenance_mode
        .expect("frontend requires is_maintenance_mode");
    let join_maintenance_thread = api
        .join_maintenance_thread
        .expect("frontend requires join_maintenance_thread");
    let prebuild = api.prebuild.expect("frontend requires prebuild");
    let deploy = api.deploy.expect("frontend requires deploy");
    let deploy_schema = api.deploy_schema.expect("frontend requires deploy_schema");
    let deploy_config_file = api
        .deploy_config_file
        .expect("frontend requires deploy_config_file");
    let run_task = api.run_task.expect("frontend requires run_task");
    let sync_user_data = api
        .sync_user_data
        .expect("frontend requires sync_user_data");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let find_session = api.find_session.expect("frontend requires find_session");

    cleanup_all_sessions();
    let root = unique_temp_dir("deployment");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n  version: test\n",
    )
    .expect("shared schema should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let schema_file = CString::new("default.schema.yaml").expect("literal should be valid");
    let config_file = CString::new("default.yaml").expect("literal should be valid");
    let version_key = CString::new("config_version").expect("literal should be valid");
    let task_name = CString::new("workspace_update").expect("literal should be valid");
    let unknown_task = CString::new("no_such_task").expect("literal should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { deployer_initialize(&traits) };

    assert_eq!(start_maintenance(TRUE), TRUE);
    assert_eq!(start_maintenance(FALSE), FALSE);
    assert_eq!(is_maintenance_mode(), FALSE);
    join_maintenance_thread();
    assert!(user.join("build").join("default.yaml").is_file());
    assert!(user.join("build").join("default.schema.yaml").is_file());

    assert_eq!(prebuild(), TRUE);
    assert_eq!(deploy(), TRUE);
    assert_eq!(deploy_schema(schema_file.as_ptr()), TRUE);
    assert_eq!(deploy_schema(ptr::null()), FALSE);
    assert_eq!(
        deploy_config_file(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    assert_eq!(deploy_config_file(config_file.as_ptr(), ptr::null()), FALSE);
    assert_eq!(run_task(task_name.as_ptr()), TRUE);
    assert_eq!(run_task(unknown_task.as_ptr()), FALSE);
    assert_eq!(run_task(ptr::null()), FALSE);

    let session_id = create_session();
    assert_eq!(find_session(session_id), TRUE);
    assert_eq!(sync_user_data(), TRUE);
    assert_eq!(find_session(session_id), FALSE);

    let reset_traits = empty_traits();
    unsafe { deployer_initialize(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_read_in_memory_configs() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let config_init = api.config_init.expect("frontend requires config_init");
    let config_load_string = api
        .config_load_string
        .expect("frontend requires config_load_string");
    let config_get_bool = api
        .config_get_bool
        .expect("frontend requires config_get_bool");
    let config_get_int = api
        .config_get_int
        .expect("frontend requires config_get_int");
    let config_get_double = api
        .config_get_double
        .expect("frontend requires config_get_double");
    let config_get_string = api
        .config_get_string
        .expect("frontend requires config_get_string");
    let config_get_cstring = api
        .config_get_cstring
        .expect("frontend requires config_get_cstring");
    let config_list_size = api
        .config_list_size
        .expect("frontend requires config_list_size");
    let config_begin_list = api
        .config_begin_list
        .expect("frontend requires config_begin_list");
    let config_begin_map = api
        .config_begin_map
        .expect("frontend requires config_begin_map");
    let config_next = api.config_next.expect("frontend requires config_next");
    let config_end = api.config_end.expect("frontend requires config_end");
    let config_close = api.config_close.expect("frontend requires config_close");

    let mut config = empty_config();
    let yaml = CString::new(
        "\
schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\nswitches:\n  - name: ascii_mode\n  - name: full_shape\nmenu:\n  page_size: 9\n  alternative_select_keys: ABC\nweights:\n  bias: 0.75\nenabled: true\n",
    )
    .expect("yaml should not contain NUL");
    let enabled_key = CString::new("enabled").expect("literal should not contain NUL");
    let page_size_key = CString::new("menu/page_size").expect("literal should not contain NUL");
    let bias_key = CString::new("weights/bias").expect("literal should not contain NUL");
    let schema_name_key = CString::new("schema/name").expect("literal should not contain NUL");
    let schema_id_key = CString::new("schema/schema_id").expect("literal should not contain NUL");
    let switches_key = CString::new("switches").expect("literal should not contain NUL");
    let menu_key = CString::new("menu").expect("literal should not contain NUL");
    let missing_key = CString::new("missing").expect("literal should not contain NUL");

    assert_eq!(unsafe { config_init(&mut config) }, TRUE);
    assert!(!config.ptr.is_null());
    assert_eq!(unsafe { config_init(&mut config) }, FALSE);
    assert_eq!(
        unsafe { config_load_string(&mut config, yaml.as_ptr()) },
        TRUE
    );

    let mut enabled = FALSE;
    let mut page_size: c_int = 0;
    let mut bias = 0.0;
    let mut schema_name_buffer = vec![0 as c_char; 16];
    assert_eq!(
        unsafe { config_get_bool(&mut config, enabled_key.as_ptr(), &mut enabled) },
        TRUE
    );
    assert_eq!(enabled, TRUE);
    assert_eq!(
        unsafe { config_get_int(&mut config, page_size_key.as_ptr(), &mut page_size) },
        TRUE
    );
    assert_eq!(page_size, 9);
    assert_eq!(
        unsafe { config_get_double(&mut config, bias_key.as_ptr(), &mut bias) },
        TRUE
    );
    assert_eq!(bias, 0.75);
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                schema_name_key.as_ptr(),
                schema_name_buffer.as_mut_ptr(),
                schema_name_buffer.len(),
            )
        },
        TRUE
    );
    let schema_name = unsafe { CStr::from_ptr(schema_name_buffer.as_ptr()) };
    assert_eq!(schema_name.to_str(), Ok("Luna Pinyin"));
    let schema_id = unsafe { config_get_cstring(&mut config, schema_id_key.as_ptr()) };
    assert!(!schema_id.is_null());
    let schema_id = unsafe { CStr::from_ptr(schema_id) };
    assert_eq!(schema_id.to_str(), Ok("luna_pinyin"));
    assert_eq!(
        unsafe { config_list_size(&mut config, switches_key.as_ptr()) },
        2
    );

    let mut iterator = empty_config_iterator();
    assert_eq!(
        unsafe { config_begin_list(&mut iterator, &mut config, switches_key.as_ptr()) },
        TRUE
    );
    assert_eq!(iterator.index, -1);
    assert!(!iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(unsafe { config_next(&mut iterator) }, TRUE);
    assert_eq!(iterator.index, 0);
    assert_eq!(unsafe { CStr::from_ptr(iterator.key) }.to_str(), Ok("@0"));
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("switches/@0")
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, TRUE);
    assert_eq!(iterator.index, 1);
    assert_eq!(unsafe { CStr::from_ptr(iterator.key) }.to_str(), Ok("@1"));
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("switches/@1")
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 2);
    unsafe { config_end(&mut iterator) };
    assert!(iterator.list.is_null());
    assert!(iterator.key.is_null());

    assert_eq!(
        unsafe { config_begin_map(&mut iterator, &mut config, menu_key.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("alternative_select_keys")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/alternative_select_keys")
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("page_size")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/page_size")
    );
    assert_eq!(unsafe { config_next(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 2);
    unsafe { config_end(&mut iterator) };

    assert_eq!(
        unsafe { config_begin_list(&mut iterator, &mut config, missing_key.as_ptr()) },
        FALSE
    );
    assert!(iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(iterator.index, -1);
    assert!(iterator.key.is_null());
    assert!(iterator.path.is_null());

    assert_eq!(unsafe { config_close(&mut config) }, TRUE);
    assert!(config.ptr.is_null());
}

#[test]
fn frontend_style_api_table_can_mutate_in_memory_configs() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let config_init = api.config_init.expect("frontend requires config_init");
    let config_load_string = api
        .config_load_string
        .expect("frontend requires config_load_string");
    let config_set_bool = api
        .config_set_bool
        .expect("frontend requires config_set_bool");
    let config_set_int = api
        .config_set_int
        .expect("frontend requires config_set_int");
    let config_set_double = api
        .config_set_double
        .expect("frontend requires config_set_double");
    let config_set_string = api
        .config_set_string
        .expect("frontend requires config_set_string");
    let config_get_bool = api
        .config_get_bool
        .expect("frontend requires config_get_bool");
    let config_get_int = api
        .config_get_int
        .expect("frontend requires config_get_int");
    let config_get_double = api
        .config_get_double
        .expect("frontend requires config_get_double");
    let config_get_string = api
        .config_get_string
        .expect("frontend requires config_get_string");
    let config_create_list = api
        .config_create_list
        .expect("frontend requires config_create_list");
    let config_create_map = api
        .config_create_map
        .expect("frontend requires config_create_map");
    let config_list_size = api
        .config_list_size
        .expect("frontend requires config_list_size");
    let config_get_item = api
        .config_get_item
        .expect("frontend requires config_get_item");
    let config_set_item = api
        .config_set_item
        .expect("frontend requires config_set_item");
    let config_clear = api.config_clear.expect("frontend requires config_clear");
    let config_close = api.config_close.expect("frontend requires config_close");

    let mut source = empty_config();
    let mut item = empty_config();
    let mut destination = empty_config();
    let schema_key = CString::new("schema").expect("literal should not contain NUL");
    let schema_name_key = CString::new("schema/name").expect("literal should not contain NUL");
    let copied_schema_key = CString::new("copied/schema").expect("literal should not contain NUL");
    let copied_name_key =
        CString::new("copied/schema/name").expect("literal should not contain NUL");
    let page_size_key = CString::new("menu/page_size").expect("literal should not contain NUL");
    let bias_key = CString::new("weights/bias").expect("literal should not contain NUL");
    let enabled_key = CString::new("enabled").expect("literal should not contain NUL");
    let switches_key = CString::new("switches").expect("literal should not contain NUL");
    let menu_key = CString::new("menu").expect("literal should not contain NUL");
    let name_value = CString::new("Default").expect("literal should not contain NUL");
    let replacement_value = CString::new("Modified").expect("literal should not contain NUL");
    let yaml = CString::new(
        "\
schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\n",
    )
    .expect("yaml should not contain NUL");

    assert_eq!(unsafe { config_init(&mut destination) }, TRUE);
    assert_eq!(
        unsafe {
            config_set_string(
                &mut destination,
                schema_name_key.as_ptr(),
                name_value.as_ptr(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { config_set_int(&mut destination, page_size_key.as_ptr(), 7) },
        TRUE
    );
    assert_eq!(
        unsafe { config_set_double(&mut destination, bias_key.as_ptr(), 1.25) },
        TRUE
    );
    assert_eq!(
        unsafe { config_set_bool(&mut destination, enabled_key.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { config_create_list(&mut destination, switches_key.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { config_create_map(&mut destination, menu_key.as_ptr()) },
        TRUE
    );

    let mut output = vec![0 as c_char; 32];
    let mut int_output: c_int = 0;
    let mut double_output = 0.0;
    let mut bool_output = FALSE;
    assert_eq!(
        unsafe {
            config_get_string(
                &mut destination,
                schema_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Default")
    );
    assert_eq!(
        unsafe { config_get_int(&mut destination, page_size_key.as_ptr(), &mut int_output) },
        FALSE
    );
    assert_eq!(
        unsafe { config_get_double(&mut destination, bias_key.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 1.25);
    assert_eq!(
        unsafe { config_get_bool(&mut destination, enabled_key.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, TRUE);
    assert_eq!(
        unsafe { config_list_size(&mut destination, switches_key.as_ptr()) },
        0
    );

    assert_eq!(
        unsafe { config_load_string(&mut source, yaml.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { config_get_item(&mut source, schema_key.as_ptr(), &mut item) },
        TRUE
    );
    assert!(!item.ptr.is_null());
    assert_eq!(
        unsafe { config_set_item(&mut destination, copied_schema_key.as_ptr(), &mut item) },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut destination,
                copied_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Luna Pinyin")
    );

    assert_eq!(
        unsafe {
            config_set_string(
                &mut item,
                schema_name_key.as_ptr(),
                replacement_value.as_ptr(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut destination,
                copied_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Luna Pinyin")
    );

    assert_eq!(
        unsafe { config_clear(&mut destination, copied_name_key.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut destination,
                copied_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        FALSE
    );

    assert_eq!(unsafe { config_close(&mut source) }, TRUE);
    assert_eq!(unsafe { config_close(&mut item) }, TRUE);
    assert_eq!(unsafe { config_close(&mut destination) }, TRUE);
}

#[test]
fn frontend_style_api_table_can_update_config_signatures() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let config_init = api.config_init.expect("frontend requires config_init");
    let config_update_signature = api
        .config_update_signature
        .expect("frontend requires config_update_signature");
    let config_get_string = api
        .config_get_string
        .expect("frontend requires config_get_string");
    let config_close = api.config_close.expect("frontend requires config_close");

    let distribution_code_name =
        CString::new("frontend-test").expect("distribution code name should be valid");
    let distribution_version =
        CString::new("2026.04").expect("distribution version should be valid");
    let mut traits = empty_traits();
    traits.distribution_code_name = distribution_code_name.as_ptr();
    traits.distribution_version = distribution_version.as_ptr();
    unsafe { setup(&traits) };

    let mut config = empty_config();
    let signer = CString::new("frontend-client").expect("signer should be valid");
    let generator_key =
        CString::new("signature/generator").expect("literal should not contain NUL");
    let distribution_code_name_key =
        CString::new("signature/distribution_code_name").expect("literal should not contain NUL");
    let distribution_version_key =
        CString::new("signature/distribution_version").expect("literal should not contain NUL");
    let rime_version_key =
        CString::new("signature/rime_version").expect("literal should not contain NUL");
    let modified_time_key =
        CString::new("signature/modified_time").expect("literal should not contain NUL");
    let mut output = vec![0 as c_char; 64];

    assert_eq!(unsafe { config_init(&mut config) }, TRUE);
    assert_eq!(
        unsafe { config_update_signature(&mut config, signer.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                generator_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("frontend-client")
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                distribution_code_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("frontend-test")
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                distribution_version_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("2026.04")
    );
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                rime_version_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert!(unsafe { CStr::from_ptr(output.as_ptr()) }
        .to_str()
        .is_ok_and(|value| value.starts_with("yune-rime-api ")));
    assert_eq!(
        unsafe {
            config_get_string(
                &mut config,
                modified_time_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert!(unsafe { CStr::from_ptr(output.as_ptr()) }
        .to_str()
        .is_ok_and(|value| value.len() >= 20 && value.contains(':') && !value.ends_with('\n')));
    assert_eq!(
        unsafe { config_update_signature(&mut config, ptr::null()) },
        FALSE
    );

    assert_eq!(unsafe { config_close(&mut config) }, TRUE);
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
}

#[test]
fn frontend_style_api_table_can_drive_basic_composition_flow() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    assert_eq!(
        api.data_size,
        (mem::size_of_val(api) - mem::size_of::<i32>()) as i32
    );

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let find_session = api.find_session.expect("frontend requires find_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let get_input = api.get_input.expect("frontend requires get_input");
    let get_status = api.get_status.expect("frontend requires get_status");
    let free_status = api.free_status.expect("frontend requires free_status");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(find_session(session_id), TRUE);
    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let input = get_input(session_id);
    assert!(!input.is_null());
    let input = unsafe { CStr::from_ptr(input) };
    assert_eq!(input.to_str(), Ok("ni"));

    let mut status = empty_status();
    assert_eq!(unsafe { get_status(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    let schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    assert_eq!(schema_id.to_str(), Ok("default"));
    assert_eq!(unsafe { free_status(&mut status) }, TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert_eq!(context.menu.page_size, 5);
    assert_eq!(context.menu.num_candidates, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    let first_candidate = unsafe { *context.menu.candidates };
    let first_candidate_text = unsafe { CStr::from_ptr(first_candidate.text) };
    assert_eq!(first_candidate_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(select_candidate_on_current_page(session_id, 0), TRUE);

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    let commit_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(commit_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_page_schema_dictionary_candidates() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let select_schema = api.select_schema.expect("frontend requires select_schema");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");
    let highlight_candidate = api
        .highlight_candidate
        .expect("frontend requires highlight_candidate");
    let highlight_candidate_on_current_page = api
        .highlight_candidate_on_current_page
        .expect("frontend requires highlight_candidate_on_current_page");
    let change_page = api.change_page.expect("frontend requires change_page");
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");

    let root = unique_temp_dir("schema-dictionary-paging");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nmenu:\n  page_size: 2\ntranslator:\n  dictionary: frontend\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("frontend.dict.yaml"),
        "\
---\nname: frontend\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nba\t八\t10\nba\t吧\t9\nba\t爸\t8\nba\t巴\t7\nba\t把\t6\nba\t拔\t5\n",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("luna").expect("schema id should be valid");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(highlight_candidate(session_id, 0), FALSE);
    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(highlight_candidate(session_id, 3), TRUE);

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.menu.page_size, 2);
    assert_eq!(context.menu.page_no, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 1);
    assert_eq!(context.menu.num_candidates, 2);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    assert_eq!(
        unsafe { CStr::from_ptr(candidates[1].text) }.to_str(),
        Ok("巴")
    );
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(highlight_candidate_on_current_page(session_id, 0), TRUE);
    assert_eq!(change_page(session_id, FALSE), TRUE);
    assert_eq!(select_candidate_on_current_page(session_id, 1), TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    assert_eq!(unsafe { CStr::from_ptr(commit.text) }.to_str(), Ok("拔"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
}

#[test]
fn frontend_style_api_table_can_simulate_key_sequences() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let simulate_key_sequence = api
        .simulate_key_sequence
        .expect("frontend requires simulate_key_sequence");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let sequence = CString::new("ni{space}").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { simulate_key_sequence(session_id, sequence.as_ptr()) },
        TRUE
    );

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    let commit_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(commit_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_edit_input_and_caret() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let get_version = api.get_version.expect("frontend requires get_version");
    let version = get_version();
    assert!(!version.is_null());
    let version = unsafe { CStr::from_ptr(version) };
    assert_eq!(version.to_str(), Ok("yune-rime-api 0.1.0"));

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let get_input = api.get_input.expect("frontend requires get_input");
    let get_caret_pos = api.get_caret_pos.expect("frontend requires get_caret_pos");
    let set_caret_pos = api.set_caret_pos.expect("frontend requires set_caret_pos");
    let set_input = api.set_input.expect("frontend requires set_input");

    assert!(get_input(0).is_null());
    assert_eq!(get_caret_pos(0), 0);

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let input = CString::new("nihao").expect("literal should not contain NUL");
    assert_eq!(unsafe { set_input(session_id, input.as_ptr()) }, TRUE);
    assert_eq!(get_caret_pos(session_id), 5);

    let current_input = get_input(session_id);
    assert!(!current_input.is_null());
    let current_input = unsafe { CStr::from_ptr(current_input) };
    assert_eq!(current_input.to_str(), Ok("nihao"));

    set_caret_pos(session_id, 2);
    assert_eq!(get_caret_pos(session_id), 2);
    set_caret_pos(session_id, 99);
    assert_eq!(get_caret_pos(session_id), 5);

    assert_eq!(unsafe { set_input(session_id, ptr::null()) }, FALSE);
    assert_eq!(unsafe { set_input(session_id + 1, input.as_ptr()) }, FALSE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_commit_clear_and_delete_composition() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let commit_composition = api
        .commit_composition
        .expect("frontend requires commit_composition");
    let clear_composition = api
        .clear_composition
        .expect("frontend requires clear_composition");
    let delete_candidate = api
        .delete_candidate
        .expect("frontend requires delete_candidate");
    let delete_candidate_on_current_page = api
        .delete_candidate_on_current_page
        .expect("frontend requires delete_candidate_on_current_page");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    assert_eq!(commit_composition(0), FALSE);
    assert_eq!(delete_candidate(0, 0), FALSE);

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(commit_composition(session_id), FALSE);
    assert_eq!(delete_candidate(session_id, 0), FALSE);

    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(commit_composition(session_id), TRUE);

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    let committed_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(committed_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(process_key(session_id, 'h' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    clear_composition(session_id);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(process_key(session_id, 'b' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'a' as i32, 0), TRUE);
    assert_eq!(delete_candidate(session_id, 1), FALSE);
    assert_eq!(delete_candidate_on_current_page(session_id, 0), TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_manage_runtime_state() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let set_option = api.set_option.expect("frontend requires set_option");
    let get_option = api.get_option.expect("frontend requires get_option");
    let set_property = api.set_property.expect("frontend requires set_property");
    let get_property = api.get_property.expect("frontend requires get_property");
    let get_current_schema = api
        .get_current_schema
        .expect("frontend requires get_current_schema");
    let select_schema = api.select_schema.expect("frontend requires select_schema");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let get_status = api.get_status.expect("frontend requires get_status");
    let free_status = api.free_status.expect("frontend requires free_status");

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let ascii_mode = CString::new("ascii_mode").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { get_option(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    unsafe { set_option(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(unsafe { get_option(session_id, ascii_mode.as_ptr()) }, TRUE);

    let mut status = empty_status();
    assert_eq!(unsafe { get_status(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_ascii_mode, TRUE);
    assert_eq!(unsafe { free_status(&mut status) }, TRUE);

    let property = CString::new("client_app").expect("literal should not contain NUL");
    let property_value = CString::new("frontend_client").expect("literal should not contain NUL");
    let mut property_buffer = vec![0 as c_char; 32];
    assert_eq!(
        unsafe {
            get_property(
                session_id,
                property.as_ptr(),
                property_buffer.as_mut_ptr(),
                property_buffer.len(),
            )
        },
        FALSE
    );
    unsafe { set_property(session_id, property.as_ptr(), property_value.as_ptr()) };
    assert_eq!(
        unsafe {
            get_property(
                session_id,
                property.as_ptr(),
                property_buffer.as_mut_ptr(),
                property_buffer.len(),
            )
        },
        TRUE
    );
    let copied_property = unsafe { CStr::from_ptr(property_buffer.as_ptr()) };
    assert_eq!(copied_property.to_str(), Ok("frontend_client"));

    let mut schema_buffer = vec![0 as c_char; 32];
    assert_eq!(
        unsafe { get_current_schema(session_id, schema_buffer.as_mut_ptr(), schema_buffer.len()) },
        TRUE
    );
    let current_schema = unsafe { CStr::from_ptr(schema_buffer.as_ptr()) };
    assert_eq!(current_schema.to_str(), Ok("default"));

    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let schema_id = CString::new("sample_schema").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { get_current_schema(session_id, schema_buffer.as_mut_ptr(), schema_buffer.len()) },
        TRUE
    );
    let selected_schema = unsafe { CStr::from_ptr(schema_buffer.as_ptr()) };
    assert_eq!(selected_schema.to_str(), Ok("sample_schema"));

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_read_runtime_paths() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let root = unique_temp_dir("runtime-paths");
    let shared = root.join("shared");
    let user = root.join("user");
    let prebuilt = root.join("prebuilt");
    let staging = root.join("stage");
    let sync = root.join("sync");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        user.join("installation.yaml"),
        format!(
            "installation_id: frontend-user\nsync_dir: '{}'\n",
            sync.to_string_lossy()
        ),
    )
    .expect("installation metadata should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let prebuilt_c = CString::new(prebuilt.to_string_lossy().as_ref()).expect("path is valid");
    let staging_c = CString::new(staging.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    traits.prebuilt_data_dir = prebuilt_c.as_ptr();
    traits.staging_dir = staging_c.as_ptr();
    unsafe { setup(&traits) };

    let get_shared_data_dir = api
        .get_shared_data_dir
        .expect("frontend requires get_shared_data_dir");
    let get_user_data_dir = api
        .get_user_data_dir
        .expect("frontend requires get_user_data_dir");
    let get_prebuilt_data_dir = api
        .get_prebuilt_data_dir
        .expect("frontend requires get_prebuilt_data_dir");
    let get_staging_dir = api
        .get_staging_dir
        .expect("frontend requires get_staging_dir");
    let get_sync_dir = api.get_sync_dir.expect("frontend requires get_sync_dir");
    let get_user_id = api.get_user_id.expect("frontend requires get_user_id");
    let get_shared_data_dir_s = api
        .get_shared_data_dir_s
        .expect("frontend requires get_shared_data_dir_s");
    let get_user_data_dir_s = api
        .get_user_data_dir_s
        .expect("frontend requires get_user_data_dir_s");
    let get_prebuilt_data_dir_s = api
        .get_prebuilt_data_dir_s
        .expect("frontend requires get_prebuilt_data_dir_s");
    let get_staging_dir_s = api
        .get_staging_dir_s
        .expect("frontend requires get_staging_dir_s");
    let get_sync_dir_s = api
        .get_sync_dir_s
        .expect("frontend requires get_sync_dir_s");
    let get_user_data_sync_dir = api
        .get_user_data_sync_dir
        .expect("frontend requires get_user_data_sync_dir");

    let shared_path = shared.to_string_lossy();
    let user_path = user.to_string_lossy();
    let prebuilt_path = prebuilt.to_string_lossy();
    let staging_path = staging.to_string_lossy();
    let sync_path = sync.to_string_lossy();
    let user_sync_path = sync.join("frontend-user");
    let user_sync_path = user_sync_path.to_string_lossy();

    let raw_shared = unsafe { CStr::from_ptr(get_shared_data_dir()) };
    assert_eq!(raw_shared.to_str(), Ok(shared_path.as_ref()));
    let raw_user = unsafe { CStr::from_ptr(get_user_data_dir()) };
    assert_eq!(raw_user.to_str(), Ok(user_path.as_ref()));
    let raw_prebuilt = unsafe { CStr::from_ptr(get_prebuilt_data_dir()) };
    assert_eq!(raw_prebuilt.to_str(), Ok(prebuilt_path.as_ref()));
    let raw_staging = unsafe { CStr::from_ptr(get_staging_dir()) };
    assert_eq!(raw_staging.to_str(), Ok(staging_path.as_ref()));
    let raw_sync = unsafe { CStr::from_ptr(get_sync_dir()) };
    assert_eq!(raw_sync.to_str(), Ok(sync_path.as_ref()));
    let raw_user_id = unsafe { CStr::from_ptr(get_user_id()) };
    assert_eq!(raw_user_id.to_str(), Ok("frontend-user"));

    let mut buffer = vec![0 as c_char; 256];
    unsafe { get_shared_data_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_shared = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_shared.to_str(), Ok(shared_path.as_ref()));

    unsafe { get_user_data_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_user = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_user.to_str(), Ok(user_path.as_ref()));

    unsafe { get_prebuilt_data_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_prebuilt = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_prebuilt.to_str(), Ok(prebuilt_path.as_ref()));

    unsafe { get_staging_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_staging = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_staging.to_str(), Ok(staging_path.as_ref()));

    unsafe { get_sync_dir_s(buffer.as_mut_ptr(), buffer.len()) };
    let copied_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_sync.to_str(), Ok(sync_path.as_ref()));

    unsafe { get_user_data_sync_dir(buffer.as_mut_ptr(), buffer.len()) };
    let copied_user_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_user_sync.to_str(), Ok(user_sync_path.as_ref()));

    let mut short_buffer = vec![0 as c_char; 8];
    unsafe { get_sync_dir_s(short_buffer.as_mut_ptr(), short_buffer.len()) };
    let truncated_sync = unsafe {
        std::slice::from_raw_parts(short_buffer.as_ptr().cast::<u8>(), short_buffer.len())
    };
    assert_eq!(truncated_sync, &sync_path.as_bytes()[..short_buffer.len()]);

    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_read_schema_state_labels() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let setup = api.setup.expect("frontend requires setup");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let root = unique_temp_dir("state-label");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
switches:
  - name: ascii_mode
    states: [Native, Ascii]
    abbrev: [N, A]
  - options: [simplification, traditional]
    states: [简体, 繁體]
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { setup(&traits) };

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let select_schema = api.select_schema.expect("frontend requires select_schema");
    let get_state_label = api
        .get_state_label
        .expect("frontend requires get_state_label");
    let get_state_label_abbreviated = api
        .get_state_label_abbreviated
        .expect("frontend requires get_state_label_abbreviated");

    let session_id = create_session();
    assert_ne!(session_id, 0);
    let schema_id = CString::new("luna").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { select_schema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let ascii_mode = CString::new("ascii_mode").expect("literal should not contain NUL");
    let full_label = unsafe { get_state_label(session_id, ascii_mode.as_ptr(), TRUE) };
    assert!(!full_label.is_null());
    let full_label = unsafe { CStr::from_ptr(full_label) };
    assert_eq!(full_label.to_str(), Ok("Ascii"));

    let abbreviated =
        unsafe { get_state_label_abbreviated(session_id, ascii_mode.as_ptr(), TRUE, TRUE) };
    assert_eq!(abbreviated.length, 1);
    assert!(!abbreviated.str.is_null());
    let abbreviated_label = unsafe { CStr::from_ptr(abbreviated.str) };
    assert_eq!(abbreviated_label.to_str(), Ok("A"));

    let simplification = CString::new("simplification").expect("literal should not contain NUL");
    let radio =
        unsafe { get_state_label_abbreviated(session_id, simplification.as_ptr(), TRUE, TRUE) };
    assert_eq!(radio.length, "简".len());
    assert!(!radio.str.is_null());
    let radio_slice = unsafe { std::slice::from_raw_parts(radio.str.cast::<u8>(), radio.length) };
    assert_eq!(std::str::from_utf8(radio_slice), Ok("简"));

    let hidden_radio =
        unsafe { get_state_label_abbreviated(session_id, simplification.as_ptr(), FALSE, TRUE) };
    assert!(hidden_radio.str.is_null());
    assert_eq!(hidden_radio.length, 0);

    let missing = CString::new("missing").expect("literal should not contain NUL");
    assert!(unsafe { get_state_label(session_id, missing.as_ptr(), TRUE) }.is_null());
    assert!(unsafe { get_state_label(0, ascii_mode.as_ptr(), TRUE) }.is_null());
    assert!(unsafe { get_state_label(session_id, ptr::null(), TRUE) }.is_null());

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    let reset_traits = empty_traits();
    unsafe { setup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn frontend_style_api_table_can_iterate_candidates() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let candidate_list_begin = api
        .candidate_list_begin
        .expect("frontend requires candidate_list_begin");
    let candidate_list_next = api
        .candidate_list_next
        .expect("frontend requires candidate_list_next");
    let candidate_list_end = api
        .candidate_list_end
        .expect("frontend requires candidate_list_end");

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let mut iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_begin(session_id, &mut iterator) },
        TRUE
    );
    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, TRUE);

    let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    let comment = unsafe { CStr::from_ptr(iterator.candidate.comment) };
    assert_eq!(comment.to_str(), Ok("echo"));

    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 1);
    let preserved_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(preserved_text.to_str(), Ok("ni"));

    unsafe { candidate_list_end(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert!(iterator.candidate.text.is_null());
    assert!(iterator.candidate.comment.is_null());

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_iterate_candidates_from_index() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let candidate_list_from_index = api
        .candidate_list_from_index
        .expect("frontend requires candidate_list_from_index");
    let candidate_list_next = api
        .candidate_list_next
        .expect("frontend requires candidate_list_next");
    let candidate_list_end = api
        .candidate_list_end
        .expect("frontend requires candidate_list_end");

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let mut empty_iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut empty_iterator, 0) },
        FALSE
    );
    assert!(empty_iterator.ptr.is_null());

    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let mut iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut iterator, 0) },
        TRUE
    );
    assert_eq!(iterator.index, -1);
    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, TRUE);

    let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    unsafe { candidate_list_end(&mut iterator) };

    let mut past_end_iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut past_end_iterator, 1) },
        TRUE
    );
    assert_eq!(past_end_iterator.index, 0);
    assert_eq!(
        unsafe { candidate_list_next(&mut past_end_iterator) },
        FALSE
    );
    assert_eq!(past_end_iterator.index, 1);
    assert!(past_end_iterator.candidate.text.is_null());
    unsafe { candidate_list_end(&mut past_end_iterator) };

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}
