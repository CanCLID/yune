use crate::resource_id::{
    validate_config_resource_id, validate_data_resource_id, validate_user_dict_name,
};

#[test]
fn config_resource_ids_accept_logical_names_and_expected_suffixes() {
    assert_eq!(validate_config_resource_id("sample"), Some("sample".to_owned()));
    assert_eq!(
        validate_config_resource_id("sample.yaml"),
        Some("sample".to_owned())
    );
    assert_eq!(
        validate_config_resource_id("sample.schema"),
        Some("sample".to_owned())
    );
    assert_eq!(
        validate_config_resource_id("sample.schema.yaml"),
        Some("sample".to_owned())
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
    assert_eq!(validate_data_resource_id("sample"), Some("sample".to_owned()));
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
    assert_eq!(validate_user_dict_name("default"), Some("default".to_owned()));
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
