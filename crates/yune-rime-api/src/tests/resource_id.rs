use std::ffi::CString;
use std::fs;

use crate::{
    deploy_config_file, deploy_schema_file, install_schema_translator_chain,
    load_runtime_config_root,
    resource_id::{
        validate_config_resource_id, validate_data_resource_id, validate_user_dict_name,
    },
    selected_runtime_config_path, selected_runtime_data_path, ConfigOpenKind,
    RimeLeversBackupUserDict, RimeLeversExportUserDict, RimeLeversImportUserDict, SessionState,
};

use super::*;

#[test]
fn config_resource_ids_accept_logical_names_and_expected_suffixes() {
    assert_eq!(
        validate_config_resource_id("sample"),
        Some("sample".to_owned())
    );
    assert_eq!(
        validate_config_resource_id("sample.yaml"),
        Some("sample".to_owned())
    );
    assert_eq!(
        validate_config_resource_id("sample.schema"),
        Some("sample.schema".to_owned())
    );
    assert_eq!(
        validate_config_resource_id("sample.schema.yaml"),
        Some("sample.schema".to_owned())
    );
    assert_eq!(
        validate_config_resource_id("default.custom"),
        Some("default.custom".to_owned())
    );
}

#[test]
fn config_resource_ids_reject_filesystem_syntax() {
    for id in [
        "",
        ".",
        "..",
        "../evil",
        "..\\evil",
        "/tmp/evil",
        "\\tmp\\evil",
        "C:evil",
        "C:\\evil",
        "a/b",
        "a\\b",
        "~/evil",
        "evil\0id",
    ] {
        assert_eq!(validate_config_resource_id(id), None, "{id:?}");
    }
}

#[test]
fn data_resource_ids_accept_logical_file_names() {
    assert_eq!(
        validate_data_resource_id("sample"),
        Some("sample".to_owned())
    );
    assert_eq!(
        validate_data_resource_id("sample_schema"),
        Some("sample_schema".to_owned())
    );
    assert_eq!(
        validate_data_resource_id("luna_pinyin.dict.yaml"),
        Some("luna_pinyin.dict.yaml".to_owned())
    );
    assert_eq!(
        validate_data_resource_id("essay.txt"),
        Some("essay.txt".to_owned())
    );
}

#[test]
fn data_resource_ids_reject_filesystem_syntax() {
    for id in [
        "",
        ".",
        "..",
        "../evil.dict.yaml",
        "..\\evil.dict.yaml",
        "/tmp/evil.dict.yaml",
        "\\tmp\\evil.dict.yaml",
        "C:evil.dict.yaml",
        "C:\\evil.dict.yaml",
        "a/b.dict.yaml",
        "a\\b.dict.yaml",
        "~/evil.dict.yaml",
        "evil\0id.dict.yaml",
    ] {
        assert_eq!(validate_data_resource_id(id), None, "{id:?}");
    }
}

#[test]
fn user_dict_names_accept_logical_names_only() {
    assert_eq!(
        validate_user_dict_name("luna_pinyin"),
        Some("luna_pinyin".to_owned())
    );
    assert_eq!(
        validate_user_dict_name("default"),
        Some("default".to_owned())
    );
    assert_eq!(
        validate_user_dict_name("sample.user"),
        Some("sample.user".to_owned())
    );
}

#[test]
fn user_dict_names_reject_paths_and_userdb_suffixes() {
    for id in [
        "",
        ".",
        "..",
        "../evil",
        "..\\evil",
        "/tmp/evil",
        "\\tmp\\evil",
        "C:evil",
        "C:\\evil",
        "a/b",
        "a\\b",
        "~/evil",
        "evil\0id",
        "luna_pinyin.userdb",
        "luna_pinyin.userdb.txt",
    ] {
        assert_eq!(validate_user_dict_name(id), None, "{id:?}");
    }
}

#[test]
fn config_api_rejects_unsafe_resource_ids() {
    let _guard = test_guard();
    let mut config = empty_config();
    let config_id = CString::new("../evil").expect("C string");

    // SAFETY: config_id and config point to valid storage for the call.
    let opened = unsafe { RimeConfigOpen(config_id.as_ptr(), &mut config) };

    assert_eq!(opened, FALSE);
    assert!(config.ptr.is_null());
}

#[test]
fn runtime_path_helpers_reject_unsafe_resource_ids() {
    let _guard = test_guard();

    assert!(selected_runtime_config_path("../evil", ConfigOpenKind::Deployed).is_none());
    assert!(selected_runtime_data_path("../evil.dict.yaml").is_none());
    assert_eq!(
        load_runtime_config_root("../evil", ConfigOpenKind::Deployed),
        Value::Null
    );
}

#[test]
fn deployment_rejects_unsafe_logical_filenames() {
    let _guard = test_guard();

    assert!(!deploy_config_file("../evil.yaml", "config_version"));
    assert!(!deploy_schema_file("../evil.schema.yaml"));
}

#[test]
fn schema_dictionary_loading_rejects_unsafe_dictionary_name() {
    let _guard = test_guard();
    let temp = unique_temp_dir("resource-id-schema-dict");
    let shared = temp.join("shared");
    let staging = temp.join("staging");
    let user = temp.join("user");
    fs::create_dir_all(&shared).expect("create shared dir");
    fs::create_dir_all(&staging).expect("create staging dir");
    fs::create_dir_all(&user).expect("create user dir");
    fs::write(
        staging.join("sample.schema.yaml"),
        "schema:\n  schema_id: sample\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: ../evil\n",
    )
    .expect("write schema");
    let traits = RimeTraits {
        shared_data_dir: CString::new(shared.to_string_lossy().as_ref())
            .expect("shared path")
            .into_raw(),
        user_data_dir: CString::new(user.to_string_lossy().as_ref())
            .expect("user path")
            .into_raw(),
        staging_dir: CString::new(staging.to_string_lossy().as_ref())
            .expect("staging path")
            .into_raw(),
        ..empty_traits()
    };
    // SAFETY: traits contains valid C strings for the duration of setup.
    unsafe { RimeSetup(&traits) };
    // SAFETY: reclaim setup strings after RimeSetup copied path values.
    unsafe {
        drop(CString::from_raw(traits.shared_data_dir as *mut c_char));
        drop(CString::from_raw(traits.user_data_dir as *mut c_char));
        drop(CString::from_raw(traits.staging_dir as *mut c_char));
    }

    let mut session = SessionState::default();
    session.engine.set_schema("sample", "sample");
    install_schema_translator_chain(&mut session, "sample");

    assert!(session.engine.context().candidates.is_empty());
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn userdb_apis_reject_unsafe_logical_dict_names_but_keep_file_paths() {
    let _guard = test_guard();
    let temp = unique_temp_dir("resource-id-userdb");
    let user = temp.join("user");
    let sync = temp.join("sync");
    fs::create_dir_all(&user).expect("create user dir");
    fs::create_dir_all(&sync).expect("create sync dir");
    fs::write(temp.join("input.txt"), "ni\t你\n").expect("write import source");
    fs::write(
        user.join("installation.yaml"),
        format!(
            "installation_id: resource-id-test\nsync_dir: {}\n",
            sync.to_string_lossy()
        ),
    )
    .expect("write installation");
    let traits = RimeTraits {
        user_data_dir: CString::new(user.to_string_lossy().as_ref())
            .expect("user path")
            .into_raw(),
        ..empty_traits()
    };
    // SAFETY: traits contains valid C strings for the duration of setup.
    unsafe { RimeSetup(&traits) };
    // SAFETY: reclaim setup strings after RimeSetup copied path values.
    unsafe {
        drop(CString::from_raw(traits.user_data_dir as *mut c_char));
    }

    let dict_name = CString::new("../evil").expect("dict C string");
    let text_file =
        CString::new(temp.join("input.txt").to_string_lossy().as_ref()).expect("path C string");

    // SAFETY: pointers reference valid C strings for the calls.
    assert_eq!(
        unsafe { RimeLeversBackupUserDict(dict_name.as_ptr()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeLeversExportUserDict(dict_name.as_ptr(), text_file.as_ptr()) },
        -1
    );
    assert_eq!(
        unsafe { RimeLeversImportUserDict(dict_name.as_ptr(), text_file.as_ptr()) },
        -1
    );

    let safe_name = CString::new("safe").expect("safe dict C string");
    assert_eq!(
        unsafe { RimeLeversImportUserDict(safe_name.as_ptr(), text_file.as_ptr()) },
        1
    );
    assert!(user.join("safe.userdb").is_file());
    assert!(!user.join("..").join("evil.userdb").exists());

    let _ = fs::remove_dir_all(temp);
}
