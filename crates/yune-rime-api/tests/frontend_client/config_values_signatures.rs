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
