use super::*;

#[test]
fn config_load_string_and_scalar_accessors_work() {
    let _guard = test_guard();
    let mut config = empty_config();
    let yaml = CString::new(
            "\
schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\nswitches:\n  - name: ascii_mode\nmenu:\n  page_size: 9\nspeller:\n  algebra:\n    - xform/^([nl])ue$/$1ve/\nweights:\n  bias: 0.75\nenabled: true\n",
        )
        .expect("yaml should be valid");
    let mut enabled = FALSE;
    let mut page_size = 0;
    let mut bias = 0.0;
    let mut name_buffer = vec![0 as c_char; 16];

    // SAFETY: config points to writable storage and yaml is a valid C string.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );
    // SAFETY: keys and output pointers are valid for each call.
    assert_eq!(
        unsafe {
            RimeConfigGetBool(
                &mut config,
                CString::new("enabled").unwrap().as_ptr(),
                &mut enabled,
            )
        },
        TRUE
    );
    assert_eq!(enabled, TRUE);
    // SAFETY: keys and output pointers are valid for each call.
    assert_eq!(
        unsafe {
            RimeConfigGetInt(
                &mut config,
                CString::new("menu/page_size").unwrap().as_ptr(),
                &mut page_size,
            )
        },
        TRUE
    );
    assert_eq!(page_size, 9);
    // SAFETY: keys and output pointers are valid for each call.
    assert_eq!(
        unsafe {
            RimeConfigGetDouble(
                &mut config,
                CString::new("weights/bias").unwrap().as_ptr(),
                &mut bias,
            )
        },
        TRUE
    );
    assert_eq!(bias, 0.75);
    // SAFETY: keys and output pointers are valid for each call.
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                CString::new("schema/name").unwrap().as_ptr(),
                name_buffer.as_mut_ptr(),
                name_buffer.len(),
            )
        },
        TRUE
    );
    // SAFETY: the config API NUL-terminates successful string copies.
    assert_eq!(
        unsafe { CStr::from_ptr(name_buffer.as_ptr()) }.to_str(),
        Ok("Luna Pinyin")
    );
    // SAFETY: key is valid and the returned pointer is borrowed from config.
    let schema_id = unsafe {
        RimeConfigGetCString(
            &mut config,
            CString::new("schema/schema_id").unwrap().as_ptr(),
        )
    };
    assert!(!schema_id.is_null());
    // SAFETY: non-null pointer returned by the config API is a valid C string.
    assert_eq!(
        unsafe { CStr::from_ptr(schema_id) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: key and config are valid.
    assert_eq!(
        unsafe { RimeConfigListSize(&mut config, CString::new("switches").unwrap().as_ptr()) },
        1
    );
    // SAFETY: config was initialized by the API and is still open.
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
    assert!(config.ptr.is_null());
}

#[test]
fn config_load_string_initializes_before_librime_parse_failure() {
    let _guard = test_guard();
    let mut config = empty_config();
    let invalid_yaml = CString::new("schema: [unterminated").expect("yaml should be valid");
    let key = CString::new("schema/name").expect("key should be valid");
    let value = CString::new("Fallback").expect("value should be valid");
    let mut output = vec![0 as c_char; 16];

    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, invalid_yaml.as_ptr()) },
        FALSE
    );
    assert!(!config.ptr.is_null());
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(&mut config, key.as_ptr(), output.as_mut_ptr(), output.len())
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Fallback")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_open_apis_load_runtime_yaml_files() {
    let _guard = test_guard();
    let root = unique_temp_dir("config-open");
    let shared = root.join("shared");
    let user = root.join("user");
    let prebuilt = shared.join("build");
    let staging = user.join("build");
    fs::create_dir_all(&prebuilt).expect("prebuilt dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
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
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let mut config = empty_config();
    let default_id = CString::new("default").expect("config id should be valid");
    let default_file_id = CString::new("default.yaml").expect("config id should be valid");
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let user_id = CString::new("user").expect("config id should be valid");
    let missing_id = CString::new("missing").expect("config id should be valid");

    // SAFETY: config ids and output config pointers are valid.
    assert_eq!(
        unsafe { RimeConfigOpen(default_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "schema/name").as_deref(),
        Some("Staging Default")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    // SAFETY: config ids and output config pointers are valid.
    assert_eq!(
        unsafe { RimeConfigOpen(default_file_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "schema/name").as_deref(),
        Some("Staging Default")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    // SAFETY: schema id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeSchemaOpen(schema_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "schema/name").as_deref(),
        Some("Luna")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    // SAFETY: config id and output config pointer are valid.
    assert_eq!(
        unsafe { RimeUserConfigOpen(user_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "var/option").as_deref(),
        Some("custom")
    );

    // SAFETY: missing files still create a null config object, mirroring
    // librime's component-backed open behavior.
    assert_eq!(
        unsafe { RimeConfigOpen(missing_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(config_string(&mut config, "schema/name"), None);
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    let api = rime_get_api();
    assert!(!api.is_null());
    // SAFETY: function table pointer has process lifetime.
    let api = unsafe { &*api };
    assert!(api.schema_open.is_some());
    assert!(api.config_open.is_some());
    assert!(api.user_config_open.is_some());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn config_update_signature_writes_runtime_metadata() {
    let _guard = test_guard();
    let distribution_code_name =
        CString::new("yune-test").expect("distribution code name should be valid");
    let distribution_version =
        CString::new("2026.04").expect("distribution version should be valid");
    let mut traits = empty_traits();
    traits.distribution_code_name = distribution_code_name.as_ptr();
    traits.distribution_version = distribution_version.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let mut config = empty_config();
    let signer = CString::new("unit-test").expect("signer should be valid");
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigUpdateSignature(&mut config, signer.as_ptr()) },
        TRUE
    );

    assert_eq!(
        config_string(&mut config, "signature/generator").as_deref(),
        Some("unit-test")
    );
    assert_eq!(
        config_string(&mut config, "signature/distribution_code_name").as_deref(),
        Some("yune-test")
    );
    assert_eq!(
        config_string(&mut config, "signature/distribution_version").as_deref(),
        Some("2026.04")
    );
    assert!(config_string(&mut config, "signature/rime_version")
        .as_deref()
        .is_some_and(|value| value.starts_with("yune-rime-api ")));
    let modified_time =
        config_string(&mut config, "signature/modified_time").expect("signature time exists");
    assert_librime_ctime_shape(&modified_time);
    assert_eq!(
        unsafe { RimeConfigUpdateSignature(&mut config, std::ptr::null()) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
}

#[test]
fn config_iterators_expose_list_and_map_paths() {
    let _guard = test_guard();
    let mut config = empty_config();
    let yaml = CString::new(
            "\
switches:\n  - name: ascii_mode\n  - name: full_shape\nmenu:\n  page_size: 9\n  alternative_select_keys: ABC\n",
        )
        .expect("yaml should be valid");
    let switches = CString::new("switches").expect("key should be valid");
    let menu = CString::new("menu").expect("key should be valid");
    let missing = CString::new("missing").expect("key should be valid");
    let mut iterator = empty_config_iterator();

    // SAFETY: config points to writable storage and yaml is a valid C string.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );

    // SAFETY: iterator, config, and key pointers are valid.
    assert_eq!(
        unsafe { RimeConfigBeginList(&mut iterator, &mut config, switches.as_ptr()) },
        TRUE
    );
    assert_eq!(iterator.index, -1);
    assert!(!iterator.list.is_null());
    assert!(iterator.map.is_null());

    // SAFETY: iterator was initialized by RimeConfigBeginList.
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(iterator.index, 0);
    // SAFETY: iterator fields point to NUL-terminated strings owned by the iterator.
    assert_eq!(unsafe { CStr::from_ptr(iterator.key) }.to_str(), Ok("@0"));
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("switches/@0")
    );
    // SAFETY: same iterator remains valid.
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(iterator.index, 1);
    assert_eq!(unsafe { CStr::from_ptr(iterator.key) }.to_str(), Ok("@1"));
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("switches/@1")
    );
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 2);
    // SAFETY: iterator was initialized by this API and can be ended once.
    unsafe { RimeConfigEnd(&mut iterator) };
    assert!(iterator.list.is_null());
    assert!(iterator.key.is_null());

    // SAFETY: iterator, config, and key pointers are valid.
    assert_eq!(
        unsafe { RimeConfigBeginMap(&mut iterator, &mut config, menu.as_ptr()) },
        TRUE
    );
    // SAFETY: iterator was initialized by RimeConfigBeginMap.
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("alternative_select_keys")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/alternative_select_keys")
    );
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, TRUE);
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.key) }.to_str(),
        Ok("page_size")
    );
    assert_eq!(
        unsafe { CStr::from_ptr(iterator.path) }.to_str(),
        Ok("menu/page_size")
    );
    assert_eq!(unsafe { RimeConfigNext(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 2);
    unsafe { RimeConfigEnd(&mut iterator) };

    // SAFETY: missing/non-container paths should fail without initializing.
    iterator.list = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.map = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.index = 8;
    iterator.key = switches.as_ptr();
    iterator.path = switches.as_ptr();
    assert_eq!(
        unsafe { RimeConfigBeginList(&mut iterator, &mut config, missing.as_ptr()) },
        FALSE
    );
    assert!(iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(iterator.index, -1);
    assert!(iterator.key.is_null());
    assert!(iterator.path.is_null());

    iterator.list = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.map = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.index = 4;
    iterator.key = switches.as_ptr();
    iterator.path = switches.as_ptr();
    assert_eq!(
        unsafe { RimeConfigBeginMap(&mut iterator, &mut config, missing.as_ptr()) },
        FALSE
    );
    assert!(iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(iterator.index, -1);
    assert!(iterator.key.is_null());
    assert!(iterator.path.is_null());

    // librime performs the basic null-argument checks before clearing the
    // caller-visible iterator state.
    iterator.list = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.map = std::ptr::null_mut();
    iterator.index = 3;
    iterator.key = switches.as_ptr();
    iterator.path = switches.as_ptr();
    assert_eq!(
        unsafe { RimeConfigBeginList(&mut iterator, std::ptr::null_mut(), switches.as_ptr()) },
        FALSE
    );
    assert!(!iterator.list.is_null());
    assert!(iterator.map.is_null());
    assert_eq!(iterator.index, 3);
    assert_eq!(iterator.key, switches.as_ptr());
    assert_eq!(iterator.path, switches.as_ptr());

    iterator.list = std::ptr::null_mut();
    iterator.map = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    iterator.index = 5;
    iterator.key = menu.as_ptr();
    iterator.path = menu.as_ptr();
    assert_eq!(
        unsafe { RimeConfigBeginMap(&mut iterator, std::ptr::null_mut(), menu.as_ptr()) },
        FALSE
    );
    assert!(iterator.list.is_null());
    assert!(!iterator.map.is_null());
    assert_eq!(iterator.index, 5);
    assert_eq!(iterator.key, menu.as_ptr());
    assert_eq!(iterator.path, menu.as_ptr());

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_set_create_clear_and_close_work() {
    let _guard = test_guard();
    let mut config = empty_config();
    let schema_name = CString::new("schema/name").expect("key should be valid");
    let name = CString::new("Default").expect("value should be valid");
    let schema_id = CString::new("schema/schema_id").expect("key should be valid");
    let page_size = CString::new("menu/page_size").expect("key should be valid");
    let bias = CString::new("weights/bias").expect("key should be valid");
    let enabled = CString::new("enabled").expect("key should be valid");
    let switches = CString::new("switches").expect("key should be valid");
    let menu = CString::new("menu").expect("key should be valid");
    let mut output = vec![0 as c_char; 32];
    let mut int_output = 0;
    let mut double_output = 0.0;
    let mut bool_output = FALSE;

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    // SAFETY: all keys and values are valid C strings.
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, schema_name.as_ptr(), name.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, page_size.as_ptr(), 7) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetDouble(&mut config, bias.as_ptr(), 1.25) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetBool(&mut config, enabled.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigCreateList(&mut config, switches.as_ptr()) },
        TRUE
    );

    // SAFETY: all keys and output pointers are valid.
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                schema_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Default")
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, page_size.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 7);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, bias.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 1.25);
    assert_eq!(
        unsafe { RimeConfigGetBool(&mut config, enabled.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, TRUE);
    assert_eq!(
        unsafe { RimeConfigListSize(&mut config, switches.as_ptr()) },
        0
    );

    assert_eq!(
        unsafe { RimeConfigClear(&mut config, schema_name.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                schema_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigCreateMap(&mut config, menu.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, page_size.as_ptr(), &mut int_output) },
        FALSE
    );
    assert_eq!(
        unsafe {
            RimeConfigSetString(
                &mut config,
                schema_id.as_ptr(),
                CString::new("default").unwrap().as_ptr(),
            )
        },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_list_append_creates_and_extends_lists() {
    let _guard = test_guard();
    let mut config = empty_config();
    let languages = CString::new("display_languages").expect("key should be valid");
    let first_language = CString::new("display_languages/@0").expect("key should be valid");
    let second_language = CString::new("display_languages/@1").expect("key should be valid");
    let english = CString::new("en_US").expect("value should be valid");
    let cantonese = CString::new("zh_HK").expect("value should be valid");

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigListAppendString(&mut config, languages.as_ptr(), english.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigListAppendString(&mut config, languages.as_ptr(), cantonese.as_ptr()) },
        TRUE
    );

    assert_eq!(
        unsafe { RimeConfigListSize(&mut config, languages.as_ptr()) },
        2
    );
    assert_eq!(
        config_string(&mut config, first_language.to_str().unwrap()).as_deref(),
        Some("en_US")
    );
    assert_eq!(
        config_string(&mut config, second_language.to_str().unwrap()).as_deref(),
        Some("zh_HK")
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_list_append_scalar_variants_round_trip_through_accessors() {
    let _guard = test_guard();
    let mut config = empty_config();
    let items = CString::new("items").expect("key should be valid");
    let bool_item = CString::new("items/@0").expect("key should be valid");
    let int_item = CString::new("items/@1").expect("key should be valid");
    let double_item = CString::new("items/@2").expect("key should be valid");
    let string_item = CString::new("items/@3").expect("key should be valid");
    let label = CString::new("deploy").expect("value should be valid");
    let mut bool_output = FALSE;
    let mut int_output = 0;
    let mut double_output = 0.0;

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigListAppendBool(&mut config, items.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigListAppendInt(&mut config, items.as_ptr(), 7) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigListAppendDouble(&mut config, items.as_ptr(), 1.25) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigListAppendString(&mut config, items.as_ptr(), label.as_ptr()) },
        TRUE
    );

    assert_eq!(
        unsafe { RimeConfigListSize(&mut config, items.as_ptr()) },
        4
    );
    assert_eq!(
        unsafe { RimeConfigGetBool(&mut config, bool_item.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, TRUE);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, int_item.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 7);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, double_item.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 1.25);
    assert_eq!(
        config_string(&mut config, string_item.to_str().unwrap()).as_deref(),
        Some("deploy")
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_list_append_rejects_invalid_and_non_list_targets() {
    let _guard = test_guard();
    let mut config = empty_config();
    let scalar = CString::new("menu/page_size").expect("key should be valid");
    let list = CString::new("items").expect("key should be valid");
    let value = CString::new("value").expect("value should be valid");

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, scalar.as_ptr(), 7) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigListAppendString(&mut config, scalar.as_ptr(), value.as_ptr()) },
        FALSE
    );
    assert_eq!(
        config_string(&mut config, scalar.to_str().unwrap()).as_deref(),
        Some("7")
    );
    assert_eq!(
        unsafe { RimeConfigListAppendString(std::ptr::null_mut(), list.as_ptr(), value.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigListAppendString(&mut config, std::ptr::null(), value.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigListAppendString(&mut config, list.as_ptr(), std::ptr::null()) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn rime_api_exposes_config_list_append_contract() {
    let _guard = test_guard();
    let mut config = empty_config();
    let api = unsafe { &*rime_get_api() };
    let append_string = api
        .config_list_append_string
        .expect("TypeDuck-Windows requires config_list_append_string");
    let append_bool = api
        .config_list_append_bool
        .expect("TypeDuck-Windows requires config_list_append_bool");
    let append_int = api
        .config_list_append_int
        .expect("TypeDuck-Windows requires config_list_append_int");
    let append_double = api
        .config_list_append_double
        .expect("TypeDuck-Windows requires config_list_append_double");
    let list_size = api
        .config_list_size
        .expect("frontend requires config_list_size");
    let values = CString::new("deployer/options").expect("key should be valid");
    let label = CString::new("display_language").expect("value should be valid");

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { append_string(&mut config, values.as_ptr(), label.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { append_bool(&mut config, values.as_ptr(), FALSE) },
        TRUE
    );
    assert_eq!(
        unsafe { append_int(&mut config, values.as_ptr(), 42) },
        TRUE
    );
    assert_eq!(
        unsafe { append_double(&mut config, values.as_ptr(), 2.5) },
        TRUE
    );
    assert_eq!(unsafe { list_size(&mut config, values.as_ptr()) }, 4);
    assert_eq!(
        config_string(&mut config, "deployer/options/@0").as_deref(),
        Some("display_language")
    );
    assert_eq!(
        config_string(&mut config, "deployer/options/@1").as_deref(),
        Some("false")
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_get_string_allows_zero_length_buffers() {
    let _guard = test_guard();
    let mut config = empty_config();
    let key = CString::new("schema/name").expect("key should be valid");
    let value = CString::new("Default").expect("value should be valid");
    let missing = CString::new("schema/missing").expect("key should be valid");
    let mut output = 42 as c_char;

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigGetString(&mut config, key.as_ptr(), &mut output, 0) },
        TRUE
    );
    assert_eq!(output, 42 as c_char);
    assert_eq!(
        unsafe { RimeConfigGetString(&mut config, missing.as_ptr(), &mut output, 0) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigGetString(&mut config, key.as_ptr(), std::ptr::null_mut(), 0) },
        FALSE
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_get_string_uses_librime_strncpy_copy_semantics() {
    let _guard = test_guard();
    let mut config = empty_config();
    let long_key = CString::new("schema/name").expect("key should be valid");
    let long_value = CString::new("Default").expect("value should be valid");
    let short_key = CString::new("schema/id").expect("key should be valid");
    let short_value = CString::new("yo").expect("value should be valid");
    let mut truncated = [b'!' as c_char; 4];
    let mut padded = [b'!' as c_char; 4];

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, long_key.as_ptr(), long_value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, short_key.as_ptr(), short_value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                long_key.as_ptr(),
                truncated.as_mut_ptr(),
                truncated.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                short_key.as_ptr(),
                padded.as_mut_ptr(),
                padded.len(),
            )
        },
        TRUE
    );

    let truncated_bytes =
        unsafe { std::slice::from_raw_parts(truncated.as_ptr().cast::<u8>(), truncated.len()) };
    let padded_bytes =
        unsafe { std::slice::from_raw_parts(padded.as_ptr().cast::<u8>(), padded.len()) };
    assert_eq!(truncated_bytes, b"Defa");
    assert_eq!(padded_bytes, b"yo\0\0");
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_set_rejects_child_paths_under_existing_scalar_nodes() {
    let _guard = test_guard();
    let mut config = empty_config();
    let scalar = CString::new("zergs/going").expect("key should be valid");
    let child = CString::new("zergs/going/home").expect("key should be valid");
    let root = CString::new("").expect("key should be valid");
    let root_scalar = CString::new("root").expect("value should be valid");
    let root_child = CString::new("child").expect("key should be valid");
    let value = CString::new("home").expect("value should be valid");
    let mut output = vec![0 as c_char; 16];

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetBool(&mut config, scalar.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, child.as_ptr(), value.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                scalar.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("true")
    );

    assert_eq!(
        unsafe { RimeConfigClear(&mut config, scalar.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, child.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, root.as_ptr(), root_scalar.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, root_child.as_ptr(), value.as_ptr()) },
        FALSE
    );
    assert_eq!(config_string(&mut config, "").as_deref(), Some("root"));
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_paths_preserve_librime_empty_segments_after_leading_slashes() {
    let _guard = test_guard();
    let mut config = empty_config();
    let yaml = CString::new(
        r#"
foo:
  "":
    bar: empty
  bar: collapsed
"#,
    )
    .expect("yaml should be valid");
    let empty_segment = CString::new("foo//bar").expect("key should be valid");
    let collapsed = CString::new("foo/bar").expect("key should be valid");
    let leading_slash = CString::new("/foo//bar").expect("key should be valid");
    let triple_slash = CString::new("foo///bar").expect("key should be valid");
    let value = CString::new("written").expect("value should be valid");
    let mut output = vec![0 as c_char; 16];

    // SAFETY: config and YAML pointers are valid for the call.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                empty_segment.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("empty")
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                collapsed.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("collapsed")
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                leading_slash.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("empty")
    );

    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, triple_slash.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "foo///bar").as_deref(),
        Some("written")
    );
    assert_eq!(
        config_string(&mut config, "foo//bar").as_deref(),
        Some("empty")
    );
    assert_eq!(
        config_string(&mut config, "foo/bar").as_deref(),
        Some("collapsed")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_scalar_access_matches_librime_string_backed_values() {
    let _guard = test_guard();
    let mut config = empty_config();
    let page_size = CString::new("menu/page_size").expect("key should be valid");
    let enabled = CString::new("enabled").expect("key should be valid");
    let bias = CString::new("weights/bias").expect("key should be valid");
    let hex = CString::new("hex").expect("key should be valid");
    let flag = CString::new("flag").expect("key should be valid");
    let decimal = CString::new("decimal").expect("key should be valid");
    let floating = CString::new("floating").expect("key should be valid");
    let native_bool = CString::new("native_bool").expect("key should be valid");
    let native_int = CString::new("native_int").expect("key should be valid");
    let mut int_output = 0;
    let mut double_output = 0.0;
    let mut bool_output = TRUE;

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, page_size.as_ptr(), 7) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetBool(&mut config, enabled.as_ptr(), TRUE) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetDouble(&mut config, bias.as_ptr(), 1.25) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "menu/page_size").as_deref(),
        Some("7")
    );
    assert_eq!(
        config_string(&mut config, "enabled").as_deref(),
        Some("true")
    );
    assert_eq!(
        config_string(&mut config, "weights/bias").as_deref(),
        Some("1.250000")
    );
    // SAFETY: config and key pointers are valid.
    let borrowed = unsafe { RimeConfigGetCString(&mut config, page_size.as_ptr()) };
    assert!(!borrowed.is_null());
    // SAFETY: a non-null config C string is owned by the config cache.
    assert_eq!(unsafe { CStr::from_ptr(borrowed) }.to_str(), Ok("7"));
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    let yaml = CString::new(
        "\
hex: '0x10'\nflag: 'FALSE'\ndecimal: '42'\nfloating: '1.5'\nnative_bool: true\nnative_int: 8\n",
    )
    .expect("yaml should be valid");
    // SAFETY: config points to writable storage and yaml is a valid C string.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, hex.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 16);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, decimal.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 42);
    assert_eq!(
        unsafe { RimeConfigGetBool(&mut config, flag.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, FALSE);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, floating.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 1.5);
    assert_eq!(
        config_string(&mut config, "native_bool").as_deref(),
        Some("true")
    );
    assert_eq!(
        config_string(&mut config, "native_int").as_deref(),
        Some("8")
    );

    // SAFETY: native serde scalars remain readable through typed access.
    assert_eq!(
        unsafe { RimeConfigGetBool(&mut config, native_bool.as_ptr(), &mut bool_output) },
        TRUE
    );
    assert_eq!(bool_output, TRUE);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, native_int.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 8);

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_numeric_getters_accept_librime_stoi_stod_prefixes() {
    let _guard = test_guard();
    let mut config = empty_config();
    let decimal_suffix = CString::new("decimal_suffix").expect("key should be valid");
    let signed_spaced = CString::new("signed_spaced").expect("key should be valid");
    let malformed_hex_suffix = CString::new("malformed_hex_suffix").expect("key should be valid");
    let malformed_hex_empty = CString::new("malformed_hex_empty").expect("key should be valid");
    let spaced_hex = CString::new("spaced_hex").expect("key should be valid");
    let wrapped_hex = CString::new("wrapped_hex").expect("key should be valid");
    let invalid_int = CString::new("invalid_int").expect("key should be valid");
    let double_suffix = CString::new("double_suffix").expect("key should be valid");
    let exponent_suffix = CString::new("exponent_suffix").expect("key should be valid");
    let invalid_double = CString::new("invalid_double").expect("key should be valid");
    let mut int_output = 0;
    let mut double_output = 0.0;

    let yaml = CString::new(
            "\
decimal_suffix: '42abc'\nsigned_spaced: '  -7ms'\nmalformed_hex_suffix: '0x10tail'\nmalformed_hex_empty: '0x'\nspaced_hex: ' 0x10'\nwrapped_hex: '0xffffffff'\ninvalid_int: abc42\ndouble_suffix: '  2.5ms'\nexponent_suffix: '1e2hz'\ninvalid_double: hz1.5\n",
        )
        .expect("yaml should be valid");
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut config, yaml.as_ptr()) },
        TRUE
    );

    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, decimal_suffix.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 42);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, signed_spaced.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, -7);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, malformed_hex_suffix.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 0);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, malformed_hex_empty.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 0);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, spaced_hex.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, 0);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, wrapped_hex.as_ptr(), &mut int_output) },
        TRUE
    );
    assert_eq!(int_output, -1);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, invalid_int.as_ptr(), &mut int_output) },
        FALSE
    );

    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, double_suffix.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 2.5);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, exponent_suffix.as_ptr(), &mut double_output) },
        TRUE
    );
    assert_eq!(double_output, 100.0);
    assert_eq!(
        unsafe { RimeConfigGetDouble(&mut config, invalid_double.as_ptr(), &mut double_output) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_get_cstring_keeps_previous_read_only_borrows_alive() {
    let _guard = test_guard();
    let mut config = empty_config();
    let name_key = CString::new("schema/name").expect("key should be valid");
    let name_value = CString::new("Luna Pinyin").expect("value should be valid");
    let id_key = CString::new("schema/schema_id").expect("key should be valid");
    let id_value = CString::new("luna_pinyin").expect("value should be valid");

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, name_key.as_ptr(), name_value.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, id_key.as_ptr(), id_value.as_ptr()) },
        TRUE
    );

    let name = unsafe { RimeConfigGetCString(&mut config, name_key.as_ptr()) };
    let schema_id = unsafe { RimeConfigGetCString(&mut config, id_key.as_ptr()) };
    assert!(!name.is_null());
    assert!(!schema_id.is_null());
    assert_eq!(unsafe { CStr::from_ptr(name) }.to_str(), Ok("Luna Pinyin"));
    assert_eq!(
        unsafe { CStr::from_ptr(schema_id) }.to_str(),
        Ok("luna_pinyin")
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_set_supports_librime_list_key_paths() {
    let _guard = test_guard();
    let mut config = empty_config();
    let list = CString::new("list").expect("key should be valid");
    let next_id = CString::new("list/@next/id").expect("key should be valid");
    let last_value = CString::new("list/@last/value").expect("key should be valid");
    let before_first_id = CString::new("list/@before 0/id").expect("key should be valid");
    let first_value = CString::new("list/@0/value").expect("key should be valid");
    let after_last_id = CString::new("list/@after last/id").expect("key should be valid");
    let before_last_id = CString::new("list/@before last/id").expect("key should be valid");
    let value_at_0 = CString::new("list/@0/value").expect("key should be valid");
    let value_at_1 = CString::new("list/@1/value").expect("key should be valid");
    let value_at_2 = CString::new("list/@2/value").expect("key should be valid");
    let value_at_3 = CString::new("list/@3/value").expect("key should be valid");
    let last_id = CString::new("list/@last/id").expect("key should be valid");
    let mut output = 0;

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, next_id.as_ptr(), 1) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, last_value.as_ptr(), 100) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, next_id.as_ptr(), 2) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, last_value.as_ptr(), 200) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 2);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, value_at_0.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 100);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, value_at_1.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 200);

    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, before_first_id.as_ptr(), 3) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, first_value.as_ptr(), 50) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, after_last_id.as_ptr(), 4) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, last_value.as_ptr(), 400) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 4);
    for (path, expected) in [
        (&value_at_0, 50),
        (&value_at_1, 100),
        (&value_at_2, 200),
        (&value_at_3, 400),
    ] {
        assert_eq!(
            unsafe { RimeConfigGetInt(&mut config, path.as_ptr(), &mut output) },
            TRUE
        );
        assert_eq!(output, expected);
    }

    assert_eq!(
        unsafe { RimeConfigCreateList(&mut config, list.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, after_last_id.as_ptr(), 5) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 1);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, last_id.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 5);

    assert_eq!(
        unsafe { RimeConfigCreateList(&mut config, list.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, before_last_id.as_ptr(), 6) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 1);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, last_id.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 6);

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_list_references_follow_librime_strtoul_parsing() {
    let _guard = test_guard();
    let mut config = empty_config();
    let first_id = CString::new("list/@0/id").expect("key should be valid");
    let second_id = CString::new("list/@1/id").expect("key should be valid");
    let malformed_first = CString::new("list/@bogus/id").expect("key should be valid");
    let trailing_first = CString::new("list/@0bogus/id").expect("key should be valid");
    let trailing_after = CString::new("list/@after bogus/id").expect("key should be valid");
    let last_with_suffix = CString::new("list/@last bogus/id").expect("key should be valid");
    let list_value = CString::new("list/@/id").expect("key should be valid");
    let id = CString::new("id").expect("value should be valid");
    let mut output = 0;

    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, first_id.as_ptr(), 10) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, second_id.as_ptr(), 20) },
        TRUE
    );

    for path in [&malformed_first, &trailing_first] {
        assert_eq!(
            unsafe { RimeConfigGetInt(&mut config, path.as_ptr(), &mut output) },
            TRUE
        );
        assert_eq!(output, 10);
    }
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, trailing_after.as_ptr(), 30) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, second_id.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 30);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, last_with_suffix.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 20);

    assert_eq!(
        unsafe { RimeConfigSetString(&mut config, list_value.as_ptr(), id.as_ptr()) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_clear_uses_librime_null_write_semantics() {
    let _guard = test_guard();
    let mut config = empty_config();
    let list = CString::new("list").expect("key should be valid");
    let next_id = CString::new("list/@next/id").expect("key should be valid");
    let first_item = CString::new("list/@0").expect("key should be valid");
    let first_id = CString::new("list/@0/id").expect("key should be valid");
    let second_id = CString::new("list/@1/id").expect("key should be valid");
    let mut output = 0;

    // SAFETY: config points to writable storage.
    assert_eq!(unsafe { RimeConfigInit(&mut config) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, next_id.as_ptr(), 1) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetInt(&mut config, next_id.as_ptr(), 2) },
        TRUE
    );

    assert_eq!(
        unsafe { RimeConfigClear(&mut config, first_item.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 2);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, first_id.as_ptr(), &mut output) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, second_id.as_ptr(), &mut output) },
        TRUE
    );
    assert_eq!(output, 2);

    assert_eq!(
        unsafe { RimeConfigClear(&mut config, second_id.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { RimeConfigListSize(&mut config, list.as_ptr()) }, 2);
    assert_eq!(
        unsafe { RimeConfigGetInt(&mut config, second_id.as_ptr(), &mut output) },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
}

#[test]
fn config_get_and_set_item_copy_subtrees() {
    let _guard = test_guard();
    let mut source = empty_config();
    let mut item = empty_config();
    let mut destination = empty_config();
    let yaml = CString::new(
            "\
schema:\n  schema_id: luna_pinyin\n  name: Luna Pinyin\nswitches:\n  - name: ascii_mode\n  - name: full_shape\n",
        )
        .expect("yaml should be valid");
    let schema = CString::new("schema").expect("key should be valid");
    let copied_schema = CString::new("copied/schema").expect("key should be valid");
    let copied_name = CString::new("copied/schema/name").expect("key should be valid");
    let source_name = CString::new("schema/name").expect("key should be valid");
    let missing = CString::new("missing").expect("key should be valid");
    let mut output = vec![0 as c_char; 32];

    // SAFETY: config pointers and YAML string are valid.
    assert_eq!(
        unsafe { RimeConfigLoadString(&mut source, yaml.as_ptr()) },
        TRUE
    );
    // SAFETY: source, key, and destination item pointers are valid.
    assert_eq!(
        unsafe { RimeConfigGetItem(&mut source, schema.as_ptr(), &mut item) },
        TRUE
    );
    assert!(!item.ptr.is_null());
    // SAFETY: configs and keys are valid; item was initialized by get_item.
    assert_eq!(unsafe { RimeConfigInit(&mut destination) }, TRUE);
    assert_eq!(
        unsafe { RimeConfigSetItem(&mut destination, copied_schema.as_ptr(), &mut item) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut destination,
                copied_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    // SAFETY: successful string copies are NUL-terminated.
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("Luna Pinyin")
    );

    assert_eq!(
        unsafe {
            RimeConfigSetString(
                &mut item,
                source_name.as_ptr(),
                CString::new("Modified").unwrap().as_ptr(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut destination,
                copied_name.as_ptr(),
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

    // SAFETY: missing items copy as null configs and null values can be set.
    assert_eq!(
        unsafe { RimeConfigGetItem(&mut source, missing.as_ptr(), &mut item) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeConfigSetItem(&mut destination, copied_schema.as_ptr(), &mut item) },
        TRUE
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut destination,
                copied_name.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        FALSE
    );

    assert_eq!(unsafe { RimeConfigClose(&mut source) }, TRUE);
    assert_eq!(unsafe { RimeConfigClose(&mut item) }, TRUE);
    assert_eq!(unsafe { RimeConfigClose(&mut destination) }, TRUE);
}
