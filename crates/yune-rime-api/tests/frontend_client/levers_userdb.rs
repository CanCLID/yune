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
    let snapshot_text = fs::read_to_string(&snapshot).expect("snapshot should be readable");
    assert!(snapshot_text.contains("/db_name\tessay\n"));
    assert!(snapshot_text.contains("/db_type\tuserdb\n"));
    assert!(snapshot_text.contains("ni hao \t你好\tc=1 d=1 t=1\n"));
    assert!(snapshot_text.contains("zhong guo \t中国\tc=2 d=2 t=1\n"));

    let export_path = root.join("essay_export.tsv");
    let export_path_c =
        CString::new(export_path.to_string_lossy().as_ref()).expect("path is valid");
    assert_eq!(
        unsafe { export_user_dict(dict_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    assert_eq!(
        fs::read_to_string(&export_path).expect("export should be readable"),
        "你好\tni hao\t1\n中国\tzhong guo\t2\n"
    );

    fs::write(&export_path, "新\txin\t3\n词\tci\t4\n").expect("import source should be updated");
    let imported_name = CString::new("frontend_imported").expect("dict name is valid");
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    let imported = fs::read_to_string(user.join("frontend_imported.userdb"))
        .expect("imported dict should be readable");
    assert!(imported.contains("xin \t新\tc=3 d=3 t=1\n"));
    assert!(imported.contains("ci \t词\tc=4 d=4 t=1\n"));

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
