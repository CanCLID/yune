use super::*;

#[test]
fn built_in_levers_module_exposes_available_schema_list() {
    let _guard = test_guard();
    crate::module_registry()
        .lock()
        .expect("module registry should not be poisoned")
        .modules_by_name
        .clear();
    let root = unique_temp_dir("levers-schema-list");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
show_extra_schema: false
schema_list:
  - schema: luna_pinyin
  - schema: extra_schema
    case:
      - show_extra_schema
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
    .expect("schema config should be written");
    fs::write(
        user.join("ignored_missing_name.schema.yaml"),
        "schema:\n  schema_id: ignored_missing_name\n",
    )
    .expect("invalid schema config should be written");
    fs::write(
        user.join("terra_pinyin.schema.yaml"),
        "\
schema:
  schema_id: terra_pinyin
  name: Terra Pinyin
  version: ''
  description: ''
",
    )
    .expect("user schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    assert!(module.get_api.is_some());
    let get_api = module.get_api.expect("levers get_api should be set");
    let api = get_api().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    assert_eq!(
        api.data_size,
        (std::mem::size_of::<RimeLeversApi>() - std::mem::size_of::<i32>()) as i32
    );
    assert!(api.custom_settings_init.is_some());
    assert!(api.custom_settings_destroy.is_some());
    assert!(api.load_settings.is_some());
    assert!(api.save_settings.is_some());
    assert!(api.customize_bool.is_some());
    assert!(api.customize_int.is_some());
    assert!(api.customize_double.is_some());
    assert!(api.customize_string.is_some());
    assert!(api.customize_item.is_some());
    assert!(api.is_first_run.is_some());
    assert!(api.settings_is_modified.is_some());
    assert!(api.settings_get_config.is_some());
    assert!(api.switcher_settings_init.is_some());
    assert!(api.get_available_schema_list.is_some());
    assert!(api.get_selected_schema_list.is_some());
    assert!(api.schema_list_destroy.is_some());
    assert!(api.get_schema_id.is_some());
    assert!(api.get_schema_name.is_some());
    assert!(api.get_schema_version.is_some());
    assert!(api.get_schema_author.is_some());
    assert!(api.get_schema_description.is_some());
    assert!(api.get_schema_file_path.is_some());
    assert!(api.select_schemas.is_some());
    assert!(api.user_dict_iterator_init.is_some());
    assert!(api.user_dict_iterator_destroy.is_some());
    assert!(api.next_user_dict.is_some());
    assert!(api.backup_user_dict.is_some());
    assert!(api.restore_user_dict.is_some());
    assert!(api.export_user_dict.is_some());
    assert!(api.import_user_dict.is_some());

    let settings = (api
        .switcher_settings_init
        .expect("switcher settings init should be available"))();
    assert!(!settings.is_null());
    let mut schema_list = empty_schema_list();
    let get_available = api
        .get_available_schema_list
        .expect("available schema list should be available");
    // SAFETY: settings and schema_list are valid for the call.
    assert_eq!(unsafe { get_available(settings, &mut schema_list) }, TRUE);
    assert_eq!(schema_list.size, 2);
    // SAFETY: the levers API populated one schema-list item.
    let item = unsafe { *schema_list.list };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let schema_id = unsafe { CStr::from_ptr(item.schema_id) };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let name = unsafe { CStr::from_ptr(item.name) };
    assert_eq!(schema_id.to_str(), Ok("luna_pinyin"));
    assert_eq!(name.to_str(), Ok("Luna Pinyin"));
    assert!(!item.reserved.is_null());
    // SAFETY: the second item is in bounds because size is 2.
    let user_item = unsafe { *schema_list.list.add(1) };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let user_schema_id = unsafe { CStr::from_ptr(user_item.schema_id) };
    // SAFETY: schema-list strings are valid NUL-terminated strings.
    let user_name = unsafe { CStr::from_ptr(user_item.name) };
    assert_eq!(user_schema_id.to_str(), Ok("terra_pinyin"));
    assert_eq!(user_name.to_str(), Ok("Terra Pinyin"));
    assert!(!user_item.reserved.is_null());

    let get_schema_id = api.get_schema_id.expect("schema id getter should be set");
    let get_schema_name = api
        .get_schema_name
        .expect("schema name getter should be set");
    let get_schema_version = api
        .get_schema_version
        .expect("schema version getter should be set");
    let get_schema_author = api
        .get_schema_author
        .expect("schema author getter should be set");
    let get_schema_description = api
        .get_schema_description
        .expect("schema description getter should be set");
    let get_schema_file_path = api
        .get_schema_file_path
        .expect("schema file path getter should be set");
    let schema_info = item.reserved.cast();
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_id(schema_info)) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_name(schema_info)) }.to_str(),
        Ok("Luna Pinyin")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_version(schema_info)) }.to_str(),
        Ok("1.0")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_author(schema_info)) }.to_str(),
        Ok("Author One\nAuthor Two")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(get_schema_description(schema_info)) }.to_str(),
        Ok("Sample schema")
    );
    // SAFETY: item.reserved points to levers-owned schema info while the
    // schema list is alive.
    let file_path = unsafe { CStr::from_ptr(get_schema_file_path(schema_info)) };
    assert_eq!(
        file_path.to_string_lossy(),
        shared.join("luna_pinyin.schema.yaml").to_string_lossy()
    );
    let user_schema_info = user_item.reserved.cast();
    // SAFETY: empty optional schema metadata should be exposed as null, matching
    // librime's schema-info getters.
    assert!(unsafe { get_schema_version(user_schema_info) }.is_null());
    // SAFETY: same as above.
    assert!(unsafe { get_schema_description(user_schema_info) }.is_null());
    // SAFETY: null schema info is explicitly rejected.
    assert!(unsafe { get_schema_id(std::ptr::null_mut()) }.is_null());

    let mut selected_list = empty_schema_list();
    let get_selected = api
        .get_selected_schema_list
        .expect("selected schema list should be available");
    // SAFETY: settings and selected_list are valid for the call.
    assert_eq!(unsafe { get_selected(settings, &mut selected_list) }, TRUE);
    assert_eq!(selected_list.size, 2);
    // SAFETY: the levers API populated two selected schema-list items.
    let selected_first = unsafe { *selected_list.list };
    // SAFETY: the second item is in bounds because size is 2.
    let selected_second = unsafe { *selected_list.list.add(1) };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    let selected_first_id = unsafe { CStr::from_ptr(selected_first.schema_id) };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    let selected_second_id = unsafe { CStr::from_ptr(selected_second.schema_id) };
    assert_eq!(selected_first_id.to_str(), Ok("luna_pinyin"));
    assert_eq!(selected_second_id.to_str(), Ok("extra_schema"));
    assert!(selected_first.name.is_null());
    assert!(selected_first.reserved.is_null());
    assert!(selected_second.name.is_null());
    assert!(selected_second.reserved.is_null());

    let destroy = api
        .schema_list_destroy
        .expect("schema-list destroy should be available");
    fs::write(
        staging.join("default.yaml"),
        "\
schema_list:
  - schema: terra_pinyin
",
    )
    .expect("default config should be rewritten");
    let mut stale_selected_list = empty_schema_list();
    // SAFETY: existing settings keep their initialized selected-schema state.
    assert_eq!(
        unsafe { get_selected(settings, &mut stale_selected_list) },
        TRUE
    );
    assert_eq!(stale_selected_list.size, 2);
    // SAFETY: the levers API populated two selected schema-list items.
    let stale_first = unsafe { *stale_selected_list.list };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    assert_eq!(
        unsafe { CStr::from_ptr(stale_first.schema_id) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: stale_selected_list was populated by the levers API above.
    unsafe { destroy(&mut stale_selected_list) };
    fs::write(
        shared.join("new_schema.schema.yaml"),
        "\
schema:
  schema_id: new_schema
  name: New Schema
",
    )
    .expect("new schema config should be written");
    let mut stale_available_list = empty_schema_list();
    // SAFETY: existing settings keep their initialized available-schema state.
    assert_eq!(
        unsafe { get_available(settings, &mut stale_available_list) },
        TRUE
    );
    assert_eq!(stale_available_list.size, 2);
    // SAFETY: stale_available_list was populated by the levers API above.
    unsafe { destroy(&mut stale_available_list) };
    let new_settings = (api
        .switcher_settings_init
        .expect("switcher settings init should be available"))();
    assert!(!new_settings.is_null());
    let mut refreshed_available_list = empty_schema_list();
    // SAFETY: a new settings object sees the updated schema directory snapshot.
    assert_eq!(
        unsafe { get_available(new_settings, &mut refreshed_available_list) },
        TRUE
    );
    assert_eq!(refreshed_available_list.size, 3);
    // SAFETY: the second item is in bounds because size is 3 and shared-dir
    // schemas sort before user-dir schemas.
    let refreshed_available_second = unsafe { *refreshed_available_list.list.add(1) };
    // SAFETY: schema-list ids are valid NUL-terminated strings.
    assert_eq!(
        unsafe { CStr::from_ptr(refreshed_available_second.schema_id) }.to_str(),
        Ok("new_schema")
    );
    // SAFETY: refreshed_available_list was populated by the levers API above.
    unsafe { destroy(&mut refreshed_available_list) };
    let mut refreshed_selected_list = empty_schema_list();
    // SAFETY: a new settings object sees the updated deployed default config.
    assert_eq!(
        unsafe { get_selected(new_settings, &mut refreshed_selected_list) },
        TRUE
    );
    assert_eq!(refreshed_selected_list.size, 1);
    // SAFETY: the levers API populated one selected schema-list item.
    let refreshed_first = unsafe { *refreshed_selected_list.list };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    assert_eq!(
        unsafe { CStr::from_ptr(refreshed_first.schema_id) }.to_str(),
        Ok("terra_pinyin")
    );
    // SAFETY: refreshed_selected_list was populated by the levers API above.
    unsafe { destroy(&mut refreshed_selected_list) };
    // SAFETY: new_settings was allocated by this shim's switcher init function.
    unsafe { drop(Box::from_raw(new_settings)) };

    // SAFETY: selected_list was populated by the levers API above.
    unsafe { destroy(&mut selected_list) };
    assert_eq!(selected_list.size, 0);
    assert!(selected_list.list.is_null());

    let select_schemas = api
        .select_schemas
        .expect("select_schemas should be available");
    let selected_luna = CString::new("luna_pinyin").expect("schema id should be valid");
    let selected_terra = CString::new("terra_pinyin").expect("schema id should be valid");
    let schema_ids = [selected_terra.as_ptr(), selected_luna.as_ptr()];
    // SAFETY: settings, schema_ids, and each C string are valid for the call.
    assert_eq!(
        unsafe { select_schemas(settings, schema_ids.as_ptr(), schema_ids.len() as i32) },
        TRUE
    );
    let mut overridden_selected_list = empty_schema_list();
    // SAFETY: settings and selected list output are valid.
    assert_eq!(
        unsafe { get_selected(settings, &mut overridden_selected_list) },
        TRUE
    );
    assert_eq!(overridden_selected_list.size, 2);
    // SAFETY: the levers API populated two selected schema-list items.
    let overridden_first = unsafe { *overridden_selected_list.list };
    // SAFETY: the second item is in bounds because size is 2.
    let overridden_second = unsafe { *overridden_selected_list.list.add(1) };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    let overridden_first_id = unsafe { CStr::from_ptr(overridden_first.schema_id) };
    // SAFETY: selected schema-list ids are valid NUL-terminated strings.
    let overridden_second_id = unsafe { CStr::from_ptr(overridden_second.schema_id) };
    assert_eq!(overridden_first_id.to_str(), Ok("terra_pinyin"));
    assert_eq!(overridden_second_id.to_str(), Ok("luna_pinyin"));
    assert!(overridden_first.name.is_null());
    assert!(overridden_first.reserved.is_null());
    assert!(overridden_second.name.is_null());
    assert!(overridden_second.reserved.is_null());

    // SAFETY: librime's levers API treats negative counts like an empty
    // selection because its signed loop never executes.
    assert_eq!(
        unsafe { select_schemas(settings, std::ptr::null(), -1) },
        TRUE
    );
    let mut negative_count_selected_list = empty_schema_list();
    // SAFETY: settings and selected list output are valid.
    assert_eq!(
        unsafe { get_selected(settings, &mut negative_count_selected_list) },
        FALSE
    );
    assert_eq!(negative_count_selected_list.size, 0);
    assert!(negative_count_selected_list.list.is_null());

    // Restore a non-empty override before the remaining null-input checks.
    // SAFETY: settings, schema_ids, and each C string are valid for the call.
    assert_eq!(
        unsafe { select_schemas(settings, schema_ids.as_ptr(), schema_ids.len() as i32) },
        TRUE
    );
    // SAFETY: null settings and null schema arrays are rejected.
    assert_eq!(
        unsafe { select_schemas(std::ptr::null_mut(), schema_ids.as_ptr(), 1) },
        FALSE
    );
    assert_eq!(
        unsafe { select_schemas(settings, std::ptr::null(), 1) },
        FALSE
    );
    // SAFETY: overridden_selected_list was populated by the levers API above.
    unsafe { destroy(&mut overridden_selected_list) };
    assert_eq!(overridden_selected_list.size, 0);
    assert!(overridden_selected_list.list.is_null());

    // SAFETY: schema_list was populated by the levers API above.
    unsafe { destroy(&mut schema_list) };
    assert_eq!(schema_list.size, 0);
    assert!(schema_list.list.is_null());
    // SAFETY: settings was allocated by this shim's switcher init function.
    unsafe { drop(Box::from_raw(settings)) };

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn levers_user_dict_iterator_lists_userdb_entries() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-user-dicts");
    let user = root.join("user");
    fs::create_dir_all(user.join("luna_pinyin.userdb"))
        .expect("leveldb-style user dict dir should be created");
    fs::write(user.join("essay.userdb"), "").expect("user dict file should be written");
    fs::write(user.join("legacy.userdb.txt"), "")
        .expect("plain legacy user dict should not match current userdb extension");
    fs::write(user.join("default.yaml"), "").expect("unrelated file should be ignored");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    let iterator_init = api
        .user_dict_iterator_init
        .expect("user dict iterator init should be available");
    let iterator_destroy = api
        .user_dict_iterator_destroy
        .expect("user dict iterator destroy should be available");
    let next_user_dict = api
        .next_user_dict
        .expect("next user dict should be available");

    let mut iterator = crate::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 0,
    };
    // SAFETY: iterator points to writable storage.
    assert_eq!(unsafe { iterator_init(&mut iterator) }, TRUE);
    assert!(!iterator.ptr.is_null());
    assert_eq!(iterator.i, 0);

    // SAFETY: iterator was initialized by the levers API.
    let first = unsafe { next_user_dict(&mut iterator) };
    assert!(!first.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(unsafe { CStr::from_ptr(first) }.to_str(), Ok("essay"));
    // SAFETY: iterator remains initialized.
    let second = unsafe { next_user_dict(&mut iterator) };
    assert!(!second.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(
        unsafe { CStr::from_ptr(second) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: iterator is exhausted but valid.
    assert!(unsafe { next_user_dict(&mut iterator) }.is_null());

    // SAFETY: iterator was initialized by this shim.
    unsafe { iterator_destroy(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert_eq!(iterator.i, 0);

    // SAFETY: null inputs are explicitly rejected/no-oped.
    assert_eq!(unsafe { iterator_init(std::ptr::null_mut()) }, FALSE);
    assert!(unsafe { next_user_dict(std::ptr::null_mut()) }.is_null());
    unsafe { iterator_destroy(std::ptr::null_mut()) };

    fs::remove_file(user.join("essay.userdb")).expect("user dict file should be removed");
    fs::remove_dir_all(user.join("luna_pinyin.userdb")).expect("user dict dir should be removed");
    let mut empty_iterator = crate::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 7,
    };
    // SAFETY: iterator points to writable storage; no .userdb entries remain.
    assert_eq!(unsafe { iterator_init(&mut empty_iterator) }, FALSE);
    assert!(empty_iterator.ptr.is_null());
    assert_eq!(empty_iterator.i, 7);

    fs::write(user.join("cached.userdb"), "").expect("cached user dict should be written");
    let mut cached_iterator = crate::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 0,
    };
    // SAFETY: iterator points to writable storage.
    assert_eq!(unsafe { iterator_init(&mut cached_iterator) }, TRUE);
    assert!(!cached_iterator.ptr.is_null());
    assert_eq!(cached_iterator.i, 0);
    fs::remove_file(user.join("cached.userdb")).expect("cached user dict should be removed");
    // SAFETY: librime leaves an existing iterator untouched when a re-scan
    // finds no user dictionaries.
    assert_eq!(unsafe { iterator_init(&mut cached_iterator) }, FALSE);
    assert!(!cached_iterator.ptr.is_null());
    assert_eq!(cached_iterator.i, 0);
    // SAFETY: cached_iterator still owns the previous snapshot.
    let cached = unsafe { next_user_dict(&mut cached_iterator) };
    assert!(!cached.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(unsafe { CStr::from_ptr(cached) }.to_str(), Ok("cached"));
    // SAFETY: cached_iterator was initialized by this shim.
    unsafe { iterator_destroy(&mut cached_iterator) };

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn levers_user_dict_file_operations_handle_plain_userdb_files() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-user-dict-files");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    struct CurrentDirGuard(PathBuf);
    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }
    let current_dir_guard =
        CurrentDirGuard(env::current_dir().expect("current dir should be available"));
    env::set_current_dir(&root).expect("test cwd should move under temp root");
    fs::write(
        user.join("luna_pinyin.userdb"),
        "# comment\nni hao\t你好\t1\n\nzhong guo\t中国\t2\n",
    )
    .expect("plain user dict should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    let backup_user_dict = api
        .backup_user_dict
        .expect("backup user dict should be available");
    let restore_user_dict = api
        .restore_user_dict
        .expect("restore user dict should be available");
    let export_user_dict = api
        .export_user_dict
        .expect("export user dict should be available");
    let import_user_dict = api
        .import_user_dict
        .expect("import user dict should be available");

    let dict_name = CString::new("luna_pinyin").expect("dict name is valid");
    // SAFETY: dict name is a valid NUL-terminated string.
    assert_eq!(unsafe { backup_user_dict(dict_name.as_ptr()) }, TRUE);
    let snapshot = root
        .join("sync")
        .join("unknown")
        .join("luna_pinyin.userdb.txt");
    let snapshot_text = fs::read_to_string(&snapshot).expect("snapshot should be readable");
    assert!(snapshot_text.contains("/db_name\tluna_pinyin\n"));
    assert!(snapshot_text.contains("/db_type\tuserdb\n"));
    assert!(snapshot_text.contains("ni hao \t你好\tc=1 d=1 t=1\n"));
    assert!(snapshot_text.contains("zhong guo \t中国\tc=2 d=2 t=1\n"));

    let export_path = root.join("luna_export.tsv");
    let export_path_c =
        CString::new(export_path.to_string_lossy().as_ref()).expect("path is valid");
    // SAFETY: pointers are valid NUL-terminated strings.
    assert_eq!(
        unsafe { export_user_dict(dict_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    assert_eq!(
        fs::read_to_string(&export_path).expect("export should be readable"),
        "你好\tni hao\t1\n中国\tzhong guo\t2\n"
    );

    fs::write(&export_path, "新\txin\t3\n词\tci\t4\n").expect("import file should be updated");
    let imported_name = CString::new("imported").expect("dict name is valid");
    // SAFETY: pointers are valid NUL-terminated strings.
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    let imported =
        fs::read_to_string(user.join("imported.userdb")).expect("import should be readable");
    assert!(imported.contains("xin \t新\tc=3 d=3 t=1\n"));
    assert!(imported.contains("ci \t词\tc=4 d=4 t=1\n"));

    let snapshot_c = CString::new(snapshot.to_string_lossy().as_ref()).expect("path is valid");
    fs::remove_file(user.join("luna_pinyin.userdb"))
        .expect("user dict should be removable before restore");
    // SAFETY: snapshot path is a valid NUL-terminated string.
    assert_eq!(unsafe { restore_user_dict(snapshot_c.as_ptr()) }, TRUE);
    assert!(user.join("luna_pinyin.userdb").is_file());

    let missing = CString::new("missing").expect("dict name is valid");
    // SAFETY: null and missing inputs are explicitly rejected.
    assert_eq!(unsafe { backup_user_dict(std::ptr::null()) }, FALSE);
    assert_eq!(unsafe { backup_user_dict(missing.as_ptr()) }, FALSE);
    assert_eq!(unsafe { restore_user_dict(std::ptr::null()) }, FALSE);
    assert_eq!(
        unsafe { export_user_dict(std::ptr::null(), export_path_c.as_ptr()) },
        -1
    );
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), std::ptr::null()) },
        -1
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    drop(current_dir_guard);
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn levers_custom_settings_load_modify_and_save_custom_yaml() {
    let _guard = test_guard();
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
    traits.distribution_code_name = c"test_dist".as_ptr();
    traits.distribution_version = c"2026.04".as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };

    let config_id = CString::new("luna_pinyin.schema").expect("config id should be valid");
    let generator = CString::new("yune_test").expect("generator should be valid");
    let init = api
        .custom_settings_init
        .expect("custom settings init should be available");
    let destroy = api
        .custom_settings_destroy
        .expect("custom settings destroy should be available");
    let load = api
        .load_settings
        .expect("load_settings should be available");
    let save = api
        .save_settings
        .expect("save_settings should be available");
    let is_first_run = api.is_first_run.expect("is_first_run should be available");
    let is_modified = api
        .settings_is_modified
        .expect("settings_is_modified should be available");
    let get_config = api
        .settings_get_config
        .expect("settings_get_config should be available");

    // SAFETY: config id and generator are valid C strings.
    let settings = unsafe { init(config_id.as_ptr(), generator.as_ptr()) };
    assert!(!settings.is_null());
    // SAFETY: settings is valid for each call.
    assert_eq!(unsafe { load(settings) }, FALSE);
    assert_eq!(unsafe { is_first_run(settings) }, TRUE);
    assert_eq!(unsafe { is_modified(settings) }, FALSE);

    let mut loaded_config = empty_config();
    // SAFETY: settings and config output are valid.
    assert_eq!(unsafe { get_config(settings, &mut loaded_config) }, TRUE);
    assert_eq!(
        config_string(&mut loaded_config, "schema/name").as_deref(),
        Some("Luna Pinyin")
    );

    let custom_bool_key = CString::new("switches/@0/reset").expect("custom key should be valid");
    let custom_int_key = CString::new("menu/page_size").expect("custom key should be valid");
    let custom_double_key = CString::new("weights/bias").expect("custom key should be valid");
    let custom_string_key = CString::new("schema/name").expect("custom key should be valid");
    let custom_string_value = CString::new("Custom Luna").expect("value should be valid");
    let customize_bool = api
        .customize_bool
        .expect("customize_bool should be available");
    let customize_int = api
        .customize_int
        .expect("customize_int should be available");
    let customize_double = api
        .customize_double
        .expect("customize_double should be available");
    let customize_string = api
        .customize_string
        .expect("customize_string should be available");
    // SAFETY: settings and keys are valid for each customization call.
    assert_eq!(
        unsafe { customize_bool(settings, custom_bool_key.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { customize_int(settings, custom_int_key.as_ptr(), 9) },
        TRUE
    );
    assert_eq!(
        unsafe { customize_double(settings, custom_double_key.as_ptr(), 0.25) },
        TRUE
    );
    assert_eq!(
        unsafe {
            customize_string(
                settings,
                custom_string_key.as_ptr(),
                custom_string_value.as_ptr(),
            )
        },
        TRUE
    );

    let mut item_config = empty_config();
    let item_yaml = CString::new("- Control+grave\n- F4\n").expect("yaml should be valid");
    // SAFETY: item_config and YAML string are valid.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut item_config, item_yaml.as_ptr()) },
        TRUE
    );
    let customize_item = api
        .customize_item
        .expect("customize_item should be available");
    let item_key = CString::new("switcher/hotkeys").expect("item key should be valid");
    // SAFETY: settings, key, and item config are valid.
    assert_eq!(
        unsafe { customize_item(settings, item_key.as_ptr(), &mut item_config) },
        TRUE
    );
    assert_eq!(unsafe { is_modified(settings) }, TRUE);
    assert_eq!(unsafe { save(settings) }, TRUE);
    assert_eq!(unsafe { is_modified(settings) }, FALSE);
    assert_eq!(unsafe { save(settings) }, FALSE);
    assert_eq!(unsafe { is_first_run(settings) }, FALSE);

    let saved = fs::read_to_string(user.join("luna_pinyin.custom.yaml"))
        .expect("custom settings should be saved without .schema suffix");
    let saved_root: Value = serde_yaml::from_str(&saved).expect("saved YAML should parse");
    let patch = find_config_value(&saved_root, "patch")
        .and_then(Value::as_mapping)
        .expect("patch map should be present");
    assert_eq!(
        patch
            .get(Value::String("switches/@0/reset".to_owned()))
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        patch
            .get(Value::String("menu/page_size".to_owned()))
            .and_then(Value::as_i64),
        Some(9)
    );
    assert_eq!(
        patch
            .get(Value::String("weights/bias".to_owned()))
            .and_then(Value::as_f64),
        Some(0.25)
    );
    assert_eq!(
        patch
            .get(Value::String("schema/name".to_owned()))
            .and_then(Value::as_str),
        Some("Custom Luna")
    );
    assert!(matches!(
        patch.get(Value::String("switcher/hotkeys".to_owned())),
        Some(Value::Sequence(values)) if values.len() == 2
    ));
    assert_eq!(
        find_config_value(&saved_root, "customization/generator").and_then(Value::as_str),
        Some("yune_test")
    );
    assert_eq!(
        find_config_value(&saved_root, "customization/distribution_code_name")
            .and_then(Value::as_str),
        Some("test_dist")
    );
    let customization_modified_time = find_config_value(&saved_root, "customization/modified_time")
        .and_then(Value::as_str)
        .expect("customization signature should include modified time");
    assert_librime_ctime_shape(customization_modified_time);

    // SAFETY: configs and settings were initialized by this API.
    unsafe {
        assert_eq!(RimeConfigClose(&mut loaded_config), TRUE);
        assert_eq!(RimeConfigClose(&mut item_config), TRUE);
        destroy(settings);
    }
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn levers_hotkeys_are_read_from_deployed_default_config() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-hotkeys");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("default.yaml"),
        "\
switcher:
  hotkeys:
    - Control+grave
    - F4
    - ''
",
    )
    .expect("default config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let settings = crate::RimeSwitcherSettingsInit();
    assert!(!settings.is_null());
    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    let get_hotkeys = api.get_hotkeys.expect("get_hotkeys should be available");
    let set_hotkeys = api.set_hotkeys.expect("set_hotkeys should be available");

    // SAFETY: settings is a valid pointer returned by the shim.
    let hotkeys = unsafe { get_hotkeys(settings) };
    assert!(!hotkeys.is_null());
    // SAFETY: get_hotkeys returns a process-owned NUL-terminated C string.
    assert_eq!(
        unsafe { CStr::from_ptr(hotkeys) }.to_str(),
        Ok("Control+grave, F4")
    );
    fs::write(
        staging.join("default.yaml"),
        "\
switcher:
  hotkeys:
    - Alt+space
",
    )
    .expect("updated default config should be written");
    // SAFETY: settings is a valid pointer returned by the shim. librime keeps
    // the hotkeys loaded into the switcher settings object at init time.
    let original_hotkeys = unsafe { get_hotkeys(settings) };
    assert!(!original_hotkeys.is_null());
    // SAFETY: returned pointer is valid while the settings object is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(original_hotkeys) }.to_str(),
        Ok("Control+grave, F4")
    );
    let new_settings = crate::RimeSwitcherSettingsInit();
    assert!(!new_settings.is_null());
    // SAFETY: new_settings is a valid pointer returned after the config update.
    let updated_hotkeys = unsafe { get_hotkeys(new_settings) };
    assert!(!updated_hotkeys.is_null());
    // SAFETY: returned pointer is valid while the new settings object is alive.
    assert_eq!(
        unsafe { CStr::from_ptr(updated_hotkeys) }.to_str(),
        Ok("Alt+space")
    );
    // SAFETY: null settings are rejected without dereferencing.
    assert!(unsafe { get_hotkeys(std::ptr::null_mut()) }.is_null());

    let new_hotkeys = CString::new("Alt+space").expect("hotkeys should be valid");
    // SAFETY: settings and hotkeys are valid pointers; mutation is currently unsupported.
    assert_eq!(
        unsafe { set_hotkeys(settings, new_hotkeys.as_ptr()) },
        FALSE
    );

    // SAFETY: new_settings was allocated by this shim's switcher init function.
    unsafe { drop(Box::from_raw(new_settings)) };
    // SAFETY: settings was allocated by this shim's switcher init function.
    unsafe { drop(Box::from_raw(settings)) };
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
