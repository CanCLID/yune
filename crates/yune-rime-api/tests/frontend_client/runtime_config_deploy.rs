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
    let start_maintenance = api
        .start_maintenance
        .expect("frontend requires start_maintenance");
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
    assert_eq!(start_maintenance(TRUE), TRUE);
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
    let cleanup_stale_sessions = api
        .cleanup_stale_sessions
        .expect("frontend requires cleanup_stale_sessions");
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
    cleanup_stale_sessions();
    assert_eq!(find_session(session_id), TRUE);
    assert_eq!(sync_user_data(), TRUE);
    assert_eq!(find_session(session_id), FALSE);

    let reset_traits = empty_traits();
    unsafe { deployer_initialize(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
