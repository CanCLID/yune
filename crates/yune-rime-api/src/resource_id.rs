pub(crate) fn validate_config_resource_id(id: &str) -> Option<String> {
    let normalized = id
        .strip_suffix(".schema.yaml")
        .or_else(|| id.strip_suffix(".schema"))
        .or_else(|| id.strip_suffix(".yaml"))
        .unwrap_or(id);
    validate_logical_id(normalized)
}

pub(crate) fn validate_runtime_config_resource_id(id: &str) -> Option<String> {
    let normalized = id.strip_suffix(".yaml").unwrap_or(id);
    validate_logical_id(normalized)
}

pub(crate) fn validate_schema_config_resource_id(id: &str) -> Option<String> {
    let normalized = id
        .strip_suffix(".schema.yaml")
        .or_else(|| id.strip_suffix(".schema"))
        .unwrap_or(id);
    validate_logical_id(normalized).map(|id| format!("{id}.schema"))
}

pub(crate) fn validate_data_resource_id(id: &str) -> Option<String> {
    validate_logical_id(id)
}

pub(crate) fn validate_user_dict_name(id: &str) -> Option<String> {
    if id.ends_with(".userdb") || id.ends_with(".userdb.txt") {
        return None;
    }
    validate_logical_id(id)
}

fn validate_logical_id(id: &str) -> Option<String> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.starts_with('~')
        || id.contains('\0')
        || id.contains('/')
        || id.contains('\\')
        || has_windows_drive_prefix(id)
    {
        return None;
    }

    Some(id.to_owned())
}

fn has_windows_drive_prefix(id: &str) -> bool {
    let bytes = id.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}
