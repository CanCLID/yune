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
