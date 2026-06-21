use std::{fs, os::raw::c_int, path::Path, time::UNIX_EPOCH};

use serde_yaml::{Mapping, Number, Value};

use crate::{
    ensure_mapping, find_config_value, find_config_value_mut, normalize_config_resource_id,
    set_config_value, RIME_VERSION_BYTES,
};

pub(crate) fn source_uses_auto_custom_patch(source: &Path) -> bool {
    fs::read_to_string(source)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
        .map_or(true, |root| find_config_value(&root, "__patch").is_none())
}

pub(crate) fn apply_config_directives(
    root: &mut Value,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
) -> Option<bool> {
    let local_reference_root = root.clone();
    let root_has_patch = apply_config_directives_inner(
        root,
        shared_data_dir,
        patch_dependencies,
        true,
        &local_reference_root,
    )?;
    Some(!root_has_patch)
}

pub(crate) fn apply_config_directives_inner(
    root: &mut Value,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
    is_root: bool,
    local_reference_root: &Value,
) -> Option<bool> {
    let mut root_has_patch = false;
    match root {
        Value::Sequence(sequence) => {
            for value in sequence {
                apply_config_directives_inner(
                    value,
                    shared_data_dir,
                    patch_dependencies,
                    false,
                    local_reference_root,
                )?;
            }
        }
        Value::Mapping(mapping) => {
            let keys = mapping.keys().cloned().collect::<Vec<_>>();
            for key in keys {
                if matches!(key.as_str(), Some("__include" | "__patch")) {
                    continue;
                }
                if let Some(value) = mapping.get_mut(&key) {
                    apply_config_directives_inner(
                        value,
                        shared_data_dir,
                        patch_dependencies,
                        false,
                        local_reference_root,
                    )?;
                }
            }
            apply_node_include_directive(
                root,
                shared_data_dir,
                patch_dependencies,
                local_reference_root,
            )?;
            let node_has_patch = apply_node_patch_directive(
                root,
                shared_data_dir,
                patch_dependencies,
                local_reference_root,
            )?;
            root_has_patch = is_root && node_has_patch;
        }
        _ => {}
    }
    Some(root_has_patch)
}

pub(crate) fn apply_node_include_directive(
    root: &mut Value,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
    local_reference_root: &Value,
) -> Option<()> {
    let include = {
        let Value::Mapping(mapping) = root else {
            return Some(());
        };
        mapping.remove(Value::String("__include".to_owned()))
    };
    let Some(Value::String(reference)) = include else {
        return include.is_none().then_some(());
    };
    apply_include_reference(
        root,
        &reference,
        shared_data_dir,
        patch_dependencies,
        local_reference_root,
    )
}

pub(crate) fn apply_include_reference(
    root: &mut Value,
    reference: &str,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
    local_reference_root: &Value,
) -> Option<()> {
    let (reference, optional) = reference
        .strip_suffix('?')
        .map_or((reference, false), |reference| (reference, true));
    let (resource, path) = if let Some((resource, path)) = reference.split_once(':') {
        (resource, path)
    } else {
        ("", reference)
    };
    let included = if resource.is_empty() {
        find_config_value(root, path)
            .cloned()
            .or_else(|| find_config_value(local_reference_root, path).cloned())
    } else {
        load_external_config_reference(
            resource,
            path,
            optional,
            shared_data_dir,
            patch_dependencies,
        )?
    };
    let Some(included) = included else {
        return optional.then_some(());
    };
    let Value::Mapping(overrides) = std::mem::replace(root, included) else {
        return Some(());
    };
    merge_config_value(root, Value::Mapping(overrides)).then_some(())
}

pub(crate) fn apply_node_patch_directive(
    root: &mut Value,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
    local_reference_root: &Value,
) -> Option<bool> {
    let (patch, directive_only_node) = {
        let Value::Mapping(mapping) = root else {
            return Some(false);
        };
        let patch = mapping.remove(Value::String("__patch".to_owned()));
        (patch, mapping.is_empty())
    };
    let Some(patch) = patch else {
        return Some(false);
    };
    if directive_only_node {
        *root = Value::Null;
    }
    apply_patch_directive(
        root,
        &patch,
        shared_data_dir,
        patch_dependencies,
        local_reference_root,
    )?;
    Some(true)
}

pub(crate) fn apply_patch_directive(
    root: &mut Value,
    patch: &Value,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
    local_reference_root: &Value,
) -> Option<()> {
    match patch {
        Value::Mapping(patch) => apply_patch_map(
            root,
            patch,
            shared_data_dir,
            patch_dependencies,
            local_reference_root,
        ),
        Value::String(reference) => apply_patch_reference(
            root,
            reference,
            shared_data_dir,
            patch_dependencies,
            local_reference_root,
        ),
        Value::Sequence(patches) => {
            for patch in patches {
                apply_patch_directive(
                    root,
                    patch,
                    shared_data_dir,
                    patch_dependencies,
                    local_reference_root,
                )?;
            }
            Some(())
        }
        _ => None,
    }
}

pub(crate) fn apply_patch_reference(
    root: &mut Value,
    reference: &str,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
    local_reference_root: &Value,
) -> Option<()> {
    let (reference, optional) = reference
        .strip_suffix('?')
        .map_or((reference, false), |reference| (reference, true));
    let (resource, path) = if let Some((resource, path)) = reference.split_once(':') {
        (resource, path)
    } else {
        ("", reference)
    };
    if !resource.is_empty() {
        let Some(referenced) = load_external_config_reference(
            resource,
            path,
            optional,
            shared_data_dir,
            patch_dependencies,
        )?
        else {
            return Some(());
        };
        return match referenced {
            Value::Mapping(patch) => {
                let patch = if optional {
                    patch
                } else {
                    normalize_external_named_patch(reference, patch)
                };
                apply_patch_map(
                    root,
                    &patch,
                    shared_data_dir,
                    patch_dependencies,
                    local_reference_root,
                )
            }
            _ => None,
        };
    }
    match find_config_value(root, path)
        .cloned()
        .or_else(|| find_config_value(local_reference_root, path).cloned())
    {
        Some(Value::Mapping(patch)) => apply_patch_map(
            root,
            &patch,
            shared_data_dir,
            patch_dependencies,
            local_reference_root,
        ),
        Some(_) => None,
        None => optional.then_some(()),
    }
}

pub(crate) fn load_external_config_reference(
    resource: &str,
    path: &str,
    optional: bool,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
) -> Option<Option<Value>> {
    let resource_id = normalize_config_resource_id(resource)?;
    let resource_path = shared_data_dir.join(format!("{resource_id}.yaml"));
    let timestamp = if resource_path.exists() {
        source_modified_secs(&resource_path).unwrap_or(0)
    } else {
        0
    };
    patch_dependencies.push((resource_id, timestamp));
    let Some(resource_root) = fs::read_to_string(&resource_path)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
    else {
        return optional.then_some(None);
    };
    match find_config_value(&resource_root, path).cloned() {
        Some(mut value) => {
            apply_config_directives_inner(
                &mut value,
                shared_data_dir,
                patch_dependencies,
                false,
                &resource_root,
            )?;
            Some(Some(value))
        }
        None => optional.then_some(None),
    }
}

fn normalize_external_named_patch(reference: &str, patch: Mapping) -> Mapping {
    let Some(name) = reference.rsplit('/').find(|part| !part.is_empty()) else {
        return patch;
    };
    if patch.len() != 1 {
        return patch;
    }
    let key = Value::String(name.to_owned());
    if let Some(Value::Mapping(inner)) = patch.get(&key) {
        return inner.clone();
    }
    patch
}

pub(crate) fn apply_custom_patch(
    root: &mut Value,
    custom_root: &Value,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
) -> Option<()> {
    let Some(Value::Mapping(patch)) = find_config_value(custom_root, "patch") else {
        return Some(());
    };
    apply_patch_map(
        root,
        patch,
        shared_data_dir,
        patch_dependencies,
        custom_root,
    )
}

pub(crate) fn apply_legacy_preset_config_plugins(
    root: &mut Value,
    resource_id: &str,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
) -> Option<()> {
    if !resource_id.ends_with(".schema") {
        return Some(());
    }

    apply_legacy_key_binder_import_preset(root, shared_data_dir, patch_dependencies)?;
    apply_legacy_import_preset(root, "punctuator", shared_data_dir, patch_dependencies)?;
    apply_legacy_import_preset(root, "recognizer", shared_data_dir, patch_dependencies)
}

fn apply_legacy_key_binder_import_preset(
    root: &mut Value,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
) -> Option<()> {
    let preset = match find_config_value(root, "key_binder/import_preset").cloned() {
        Some(Value::String(preset)) => preset,
        Some(_) => return None,
        None => return Some(()),
    };
    let mut overrides = match find_config_value(root, "key_binder").cloned()? {
        Value::Mapping(overrides) => overrides,
        _ => return None,
    };
    if let Some(bindings) = overrides.remove(Value::String("bindings".to_owned())) {
        overrides.insert(Value::String("bindings/+".to_owned()), bindings);
    }

    let included = load_external_config_reference(
        &preset,
        "key_binder",
        false,
        shared_data_dir,
        patch_dependencies,
    )??;
    let target = find_config_value_mut(root, "key_binder")?;
    *target = included;
    merge_config_value(target, Value::Mapping(overrides)).then_some(())
}

fn apply_legacy_import_preset(
    root: &mut Value,
    section: &str,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
) -> Option<()> {
    let preset = match find_config_value(root, &format!("{section}/import_preset")).cloned() {
        Some(Value::String(preset)) => preset,
        Some(_) => return None,
        None => return Some(()),
    };
    let target = find_config_value_mut(root, section)?;
    let included = load_external_config_reference(
        &preset,
        section,
        false,
        shared_data_dir,
        patch_dependencies,
    )??;
    let Value::Mapping(overrides) = std::mem::replace(target, included) else {
        return Some(());
    };
    merge_literal_config_value(target, Value::Mapping(overrides));
    Some(())
}

fn merge_literal_config_value(target: &mut Value, value: Value) {
    match (target, value) {
        (Value::Mapping(target), Value::Mapping(value)) => {
            for (key, value) in value {
                if let Some(target_value) = target.get_mut(&key) {
                    merge_literal_config_value(target_value, value);
                } else {
                    target.insert(key, value);
                }
            }
        }
        (target, value) => {
            *target = value;
        }
    }
}

pub(crate) fn apply_patch_map(
    root: &mut Value,
    patch: &Mapping,
    shared_data_dir: &Path,
    patch_dependencies: &mut Vec<(String, c_int)>,
    local_reference_root: &Value,
) -> Option<()> {
    for (key, value) in patch {
        let key = key.as_str()?;
        let mut value = value.clone();
        apply_config_directives_inner(
            &mut value,
            shared_data_dir,
            patch_dependencies,
            false,
            local_reference_root,
        )?;
        if !apply_patch_entry(root, key, value, false) {
            return None;
        }
    }
    Some(())
}

pub(crate) fn apply_patch_entry(
    root: &mut Value,
    key: &str,
    value: Value,
    merge_tree: bool,
) -> bool {
    let appending = key == "__append" || key.ends_with("/+");
    let merging = key == "__merge"
        || key.ends_with("/+")
        || (merge_tree && matches!(value, Value::Null | Value::Mapping(_)) && !key.ends_with("/="));
    let path = if key == "__append" || key == "__merge" {
        ""
    } else if appending || merging {
        key.strip_suffix("/+")
            .or_else(|| key.strip_suffix("/="))
            .unwrap_or(key)
    } else {
        key.strip_suffix("/=").unwrap_or(key)
    };

    if appending || merging {
        if path.is_empty() {
            if !root.is_null() {
                return value.is_null()
                    || (appending && append_config_value(root, value.clone()))
                    || (merging && merge_config_value(root, value));
            }
        } else if find_config_value(root, path).is_some_and(|value| !value.is_null()) {
            let target = find_config_value_mut(root, path).expect("target was just found");
            return value.is_null()
                || (appending && append_config_value(target, value.clone()))
                || (merging && merge_config_value(target, value));
        }
    }

    set_config_value(root, path, value)
}

pub(crate) fn append_config_value(target: &mut Value, value: Value) -> bool {
    match target {
        Value::String(existing) => {
            let Value::String(value) = value else {
                return false;
            };
            existing.push_str(&value);
            true
        }
        Value::Sequence(existing) => {
            let Value::Sequence(mut value) = value else {
                return false;
            };
            existing.append(&mut value);
            true
        }
        Value::Null => {
            *target = value;
            true
        }
        _ => false,
    }
}

pub(crate) fn merge_config_value(target: &mut Value, value: Value) -> bool {
    let Value::Mapping(patch) = value else {
        return false;
    };
    for (key, value) in patch {
        let Some(key) = key.as_str() else {
            return false;
        };
        if !apply_patch_entry(target, key, value, true) {
            return false;
        }
    }
    true
}

pub(crate) fn source_modified_secs(source: &Path) -> Option<c_int> {
    source
        .metadata()
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| c_int::try_from(duration.as_secs()).unwrap_or(c_int::MAX))
}

pub(crate) fn set_build_info(root: &mut Value, resource_id: &str, timestamp: c_int) -> Option<()> {
    let Value::Mapping(root) = root else {
        return None;
    };
    let build_info = root
        .entry(Value::String("__build_info".to_owned()))
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    let Value::Mapping(build_info) = ensure_mapping(build_info) else {
        return None;
    };
    build_info.insert(
        Value::String("rime_version".to_owned()),
        Value::String(
            String::from_utf8_lossy(&RIME_VERSION_BYTES[..RIME_VERSION_BYTES.len() - 1])
                .into_owned(),
        ),
    );
    let timestamps = build_info
        .entry(Value::String("timestamps".to_owned()))
        .or_insert_with(|| Value::Mapping(Mapping::new()));
    let Value::Mapping(timestamps) = ensure_mapping(timestamps) else {
        return None;
    };
    timestamps.insert(
        Value::String(resource_id.to_owned()),
        Value::Number(Number::from(timestamp)),
    );
    Some(())
}
