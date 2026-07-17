use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value as JsonValue};
use serde_yaml::Value;
use sha2::{Digest, Sha256};
use yune_core::SchemaBehaviorProfile;

use crate::{
    config_scalar_bool, config_scalar_string, deployment::deployed_config_root_with_trace,
    find_config_value, schema_behavior_profile_from_config, schema_component_prescription,
    ConfigDirectiveTraceEvent,
};

const AUDIT_VERSION: &str = "m60-reachability-v1";
const REACHABILITY_KEY: &str = "leading_syllable_reachability";
const PROBE_MARKER: &str = "__yune_m60_reachability_setting_source_probe__";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum AssetOrigin {
    Shared,
    User,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct AuditAsset {
    absolute: PathBuf,
    emitted: String,
    origin: AssetOrigin,
}

#[derive(Debug)]
struct AuditRoots {
    repo: PathBuf,
    shared: PathBuf,
    user: PathBuf,
    roots_equal: bool,
}

#[derive(Debug)]
struct ProbeTree {
    root: PathBuf,
    shared: PathBuf,
    user: PathBuf,
}

impl Drop for ProbeTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Produces the read-only M60 schema-reachability audit consumed by repository
/// governance tooling. This is intentionally a Rust-only tooling surface; it is
/// not exported through either C ABI table.
#[doc(hidden)]
pub fn schema_reachability_audit_json(
    repo_root: &Path,
    shared_data_dir: &Path,
    user_data_dir: &Path,
    schema_assets: &[PathBuf],
) -> Result<String, String> {
    let roots = validated_roots(repo_root, shared_data_dir, user_data_dir)?;
    if schema_assets.is_empty() {
        return Err("at least one --schema-asset is required".to_owned());
    }

    let mut requested = BTreeSet::new();
    let mut tuples = Vec::new();
    for schema_asset in schema_assets {
        let schema_asset = safe_relative_path(schema_asset, "schema asset")?;
        if !requested.insert(schema_asset.clone()) {
            return Err(format!(
                "duplicate schema asset {}",
                path_to_forward_slashes(&schema_asset)
            ));
        }
        tuples.extend(audit_schema_asset(&roots, &schema_asset)?);
    }
    tuples.sort_by(|left, right| tuple_sort_key(left).cmp(&tuple_sort_key(right)));

    serde_json::to_string(&json!({
        "version": AUDIT_VERSION,
        "tuples": tuples,
    }))
    .map_err(|error| format!("could not serialize audit output: {error}"))
}

fn validated_roots(
    repo_root: &Path,
    shared_data_dir: &Path,
    user_data_dir: &Path,
) -> Result<AuditRoots, String> {
    let repo = canonical_directory(repo_root, "repository root")?;
    let shared = canonical_directory(shared_data_dir, "shared data directory")?;
    let user = canonical_directory(user_data_dir, "user data directory")?;
    require_within(&repo, &shared, "shared data directory")?;
    require_within(&repo, &user, "user data directory")?;
    Ok(AuditRoots {
        roots_equal: shared == user,
        repo,
        shared,
        user,
    })
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("{label} {} is unavailable: {error}", path.display()))?;
    if !canonical.is_dir() {
        return Err(format!("{label} {} is not a directory", path.display()));
    }
    Ok(canonical)
}

fn require_within(root: &Path, path: &Path, label: &str) -> Result<(), String> {
    if path == root || path.starts_with(root) {
        Ok(())
    } else {
        Err(format!(
            "{label} {} escapes repository root {}",
            path.display(),
            root.display()
        ))
    }
}

fn safe_relative_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(format!(
            "{label} must be a non-empty repository-relative path"
        ));
    }
    if path.components().any(|component| {
        !matches!(component, Component::Normal(_))
            || matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
    }) {
        return Err(format!("{label} {} is unsafe", path.display()));
    }
    Ok(path.to_path_buf())
}

fn audit_schema_asset(roots: &AuditRoots, schema_asset: &Path) -> Result<Vec<JsonValue>, String> {
    let source = roots.repo.join(schema_asset);
    let canonical_source = source.canonicalize().map_err(|error| {
        format!(
            "schema asset {} is unavailable: {error}",
            path_to_forward_slashes(schema_asset)
        )
    })?;
    require_within(&roots.shared, &canonical_source, "schema asset")?;
    if !canonical_source.is_file() {
        return Err(format!(
            "schema asset {} is not a regular file",
            path_to_forward_slashes(schema_asset)
        ));
    }
    if source
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(format!(
            "schema asset {} must not be a symbolic link",
            path_to_forward_slashes(schema_asset)
        ));
    }
    let file_name = canonical_source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("schema asset {} has no UTF-8 file name", source.display()))?;
    if !file_name.ends_with(".schema.yaml") {
        return Err(format!(
            "schema asset {} must end in .schema.yaml",
            path_to_forward_slashes(schema_asset)
        ));
    }

    let (deployed, directive_trace) =
        deployed_config_root_with_trace(&canonical_source, file_name, &roots.shared, &roots.user)
            .ok_or_else(|| {
            format!(
                "schema asset {} has an unresolved or unsupported deployment directive",
                path_to_forward_slashes(schema_asset)
            )
        })?;
    let schema_id = find_config_value(&deployed, "schema/schema_id")
        .and_then(Value::as_str)
        .filter(|schema_id| !schema_id.is_empty())
        .ok_or_else(|| {
            format!(
                "schema asset {} has no deployed schema/schema_id",
                path_to_forward_slashes(schema_asset)
            )
        })?;
    let translators = deployed_translator_prescriptions(&deployed, schema_asset)?;
    let (source_trace, assets) = source_trace_and_assets(
        roots,
        schema_asset,
        file_name.trim_end_matches(".yaml"),
        &directive_trace,
    )?;
    let asset_hashes = asset_hashes(&assets)?;

    let mut tuples = Vec::new();
    for component in translators {
        let (component_name, namespace) = schema_component_prescription(&component);
        let Some(translator_arm) = translator_arm(component_name) else {
            continue;
        };
        let namespace = namespace.unwrap_or("translator");
        let config_path = format!("{namespace}/{REACHABILITY_KEY}");
        let literal = find_config_value(&deployed, &config_path);
        let effective = match literal {
            Some(value) => config_scalar_bool(value).ok_or_else(|| {
                format!(
                    "schema asset {} has unsupported boolean at {config_path}; expected YAML boolean or quoted true/false",
                    path_to_forward_slashes(schema_asset)
                )
            })?,
            None => true,
        };
        let setting_asset = if literal.is_some() {
            Some(find_setting_asset(
                roots,
                schema_asset,
                file_name,
                &assets,
                &config_path,
            )?)
        } else {
            None
        };
        let runtime_arm = runtime_arm(&deployed, component_name, namespace);
        tuples.push(json!({
            "schemaAsset": path_to_forward_slashes(schema_asset),
            "schemaId": schema_id,
            "component": component,
            "namespace": namespace,
            "configPath": config_path,
            "effective": effective,
            "settingAsset": setting_asset,
            "sourceTrace": source_trace,
            "assetHashes": asset_hashes,
            "translatorArm": translator_arm,
            "runtimeArm": runtime_arm,
        }));
    }
    Ok(tuples)
}

fn deployed_translator_prescriptions(
    deployed: &Value,
    schema_asset: &Path,
) -> Result<Vec<String>, String> {
    let value = find_config_value(deployed, "engine/translators").ok_or_else(|| {
        format!(
            "schema asset {} has no deployed engine/translators sequence",
            path_to_forward_slashes(schema_asset)
        )
    })?;
    let Value::Sequence(values) = value else {
        return Err(format!(
            "schema asset {} has non-sequence engine/translators",
            path_to_forward_slashes(schema_asset)
        ));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                format!(
                    "schema asset {} has non-string engine/translators/@{index}",
                    path_to_forward_slashes(schema_asset)
                )
            })
        })
        .collect()
}

fn translator_arm(component_name: &str) -> Option<&'static str> {
    match component_name {
        "script_translator" => Some("script"),
        "table_translator" => Some("table"),
        "r10n_translator" => Some("r10n"),
        _ => None,
    }
}

fn runtime_arm(deployed: &Value, component_name: &str, namespace: &str) -> &'static str {
    let enable_sentence = find_config_value(deployed, &format!("{namespace}/enable_sentence"))
        .and_then(config_scalar_bool)
        .unwrap_or(true);
    let standard = schema_behavior_profile_from_config(deployed) == SchemaBehaviorProfile::Standard;
    let explicit = find_config_value(deployed, &format!("{namespace}/yune_sentence_policy"))
        .and_then(config_scalar_string);
    match component_name {
        "table_translator"
            if enable_sentence && standard && explicit.as_deref() == Some("upstream_script") =>
        {
            "upstream-table-lazy-bounded"
        }
        "table_translator" => "table-standard",
        "script_translator" | "r10n_translator"
            if enable_sentence
                && standard
                && matches!(explicit.as_deref(), None | Some("upstream_script")) =>
        {
            "upstream-script"
        }
        "script_translator" | "r10n_translator" => "dictionary-standard",
        _ => "not-a-dictionary-translator",
    }
}

fn source_trace_and_assets(
    roots: &AuditRoots,
    schema_asset: &Path,
    source_resource_id: &str,
    directive_trace: &[ConfigDirectiveTraceEvent],
) -> Result<(Vec<JsonValue>, BTreeSet<AuditAsset>), String> {
    let source = roots.repo.join(schema_asset);
    let source_asset = audit_asset(roots, source, AssetOrigin::Shared)?;
    let mut resource_assets = BTreeMap::new();
    resource_assets.insert(source_resource_id.to_owned(), source_asset.clone());
    let mut assets = BTreeSet::from([source_asset.clone()]);
    let mut trace = vec![json!({
        "kind": "source",
        "asset": source_asset.emitted,
    })];

    for event in directive_trace {
        let target = if event.kind == "custom-patch" {
            resource_asset(roots, &event.referenced_resource_id, AssetOrigin::User)?
        } else if event.referenced_resource_id == event.source_resource_id {
            resource_assets
                .get(&event.source_resource_id)
                .cloned()
                .unwrap_or(resource_asset(
                    roots,
                    &event.referenced_resource_id,
                    AssetOrigin::Shared,
                )?)
        } else {
            resource_asset(roots, &event.referenced_resource_id, AssetOrigin::Shared)?
        };
        if target.absolute.is_file() {
            assets.insert(target.clone());
            resource_assets
                .entry(event.referenced_resource_id.clone())
                .or_insert_with(|| target.clone());
        }
        trace.push(json!({
            "kind": event.kind,
            "asset": target.emitted,
            "reference": event.reference,
            "optional": event.optional,
        }));
    }
    Ok((trace, assets))
}

fn resource_asset(
    roots: &AuditRoots,
    resource_id: &str,
    origin: AssetOrigin,
) -> Result<AuditAsset, String> {
    if resource_id.is_empty() {
        return Err("deployment trace contained an empty resource id".to_owned());
    }
    let root = match origin {
        AssetOrigin::Shared => &roots.shared,
        AssetOrigin::User => &roots.user,
    };
    audit_asset(roots, root.join(format!("{resource_id}.yaml")), origin)
}

fn audit_asset(
    roots: &AuditRoots,
    absolute: PathBuf,
    origin: AssetOrigin,
) -> Result<AuditAsset, String> {
    require_within(&roots.repo, &absolute, "audit asset")?;
    let relative = absolute
        .strip_prefix(&roots.repo)
        .map_err(|_| format!("audit asset {} escapes repository", absolute.display()))?;
    Ok(AuditAsset {
        emitted: path_to_forward_slashes(relative),
        absolute,
        origin,
    })
}

fn asset_hashes(assets: &BTreeSet<AuditAsset>) -> Result<Vec<JsonValue>, String> {
    let mut hashes = assets
        .iter()
        .map(|asset| {
            let metadata = asset.absolute.symlink_metadata().map_err(|error| {
                format!("audit asset {} is unavailable: {error}", asset.emitted)
            })?;
            if !metadata.file_type().is_file() {
                return Err(format!(
                    "audit asset {} is not a regular file",
                    asset.emitted
                ));
            }
            let bytes = fs::read(&asset.absolute)
                .map_err(|error| format!("audit asset {} is unreadable: {error}", asset.emitted))?;
            Ok(json!({
                "asset": asset.emitted,
                "sha256": format!("{:x}", Sha256::digest(bytes)),
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    hashes.sort_by(|left, right| {
        left["asset"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["asset"].as_str().unwrap_or_default())
    });
    Ok(hashes)
}

fn find_setting_asset(
    roots: &AuditRoots,
    schema_asset: &Path,
    file_name: &str,
    assets: &BTreeSet<AuditAsset>,
    config_path: &str,
) -> Result<String, String> {
    let probe = create_probe_tree(roots, assets)?;
    let source_relative = roots
        .repo
        .join(schema_asset)
        .strip_prefix(&roots.shared)
        .map_err(|_| {
            format!(
                "schema asset {} is not under the shared data root",
                path_to_forward_slashes(schema_asset)
            )
        })?
        .to_path_buf();
    let probe_source = probe.shared.join(source_relative);
    let mut owners = Vec::new();
    for asset in assets {
        let original = fs::read(&asset.absolute).map_err(|error| {
            format!("could not read {} for source probe: {error}", asset.emitted)
        })?;
        let mut yaml = serde_yaml::from_slice::<Value>(&original).map_err(|error| {
            format!(
                "could not parse {} for source probe: {error}",
                asset.emitted
            )
        })?;
        if replace_reachability_scalars(&mut yaml) == 0 {
            continue;
        }
        let probe_path = probe_asset_path(roots, &probe, asset)?;
        let marked = serde_yaml::to_string(&yaml).map_err(|error| {
            format!(
                "could not serialize {} source probe: {error}",
                asset.emitted
            )
        })?;
        fs::write(&probe_path, marked)
            .map_err(|error| format!("could not write {} source probe: {error}", asset.emitted))?;
        let marker_wins =
            deployed_config_root_with_trace(&probe_source, file_name, &probe.shared, &probe.user)
                .is_some_and(|(deployed, _)| {
                    find_config_value(&deployed, config_path).and_then(Value::as_str)
                        == Some(PROBE_MARKER)
                });
        fs::write(&probe_path, &original).map_err(|error| {
            format!("could not restore {} source probe: {error}", asset.emitted)
        })?;
        if marker_wins {
            owners.push(asset.emitted.clone());
        }
    }
    match owners.as_slice() {
        [owner] => Ok(owner.clone()),
        [] => Err(format!(
            "deployed explicit setting {config_path} has no attributable tracked source"
        )),
        _ => Err(format!(
            "deployed explicit setting {config_path} has ambiguous sources: {}",
            owners.join(", ")
        )),
    }
}

fn create_probe_tree(
    roots: &AuditRoots,
    assets: &BTreeSet<AuditAsset>,
) -> Result<ProbeTree, String> {
    static NEXT_PROBE: AtomicU64 = AtomicU64::new(0);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let root = std::env::temp_dir().join(format!(
        "yune-m60-reachability-probe-{}-{nonce}-{}",
        std::process::id(),
        NEXT_PROBE.fetch_add(1, Ordering::Relaxed)
    ));
    let shared = root.join("shared");
    let user = if roots.roots_equal {
        shared.clone()
    } else {
        root.join("user")
    };
    fs::create_dir_all(&shared)
        .map_err(|error| format!("could not create source-probe shared root: {error}"))?;
    fs::create_dir_all(&user)
        .map_err(|error| format!("could not create source-probe user root: {error}"))?;
    let probe = ProbeTree { root, shared, user };
    for asset in assets {
        let destination = probe_asset_path(roots, &probe, asset)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "could not create source-probe directory for {}: {error}",
                    asset.emitted
                )
            })?;
        }
        fs::copy(&asset.absolute, &destination).map_err(|error| {
            format!(
                "could not copy {} into source probe: {error}",
                asset.emitted
            )
        })?;
    }
    Ok(probe)
}

fn probe_asset_path(
    roots: &AuditRoots,
    probe: &ProbeTree,
    asset: &AuditAsset,
) -> Result<PathBuf, String> {
    let (source_root, probe_root) = match asset.origin {
        AssetOrigin::Shared => (&roots.shared, &probe.shared),
        AssetOrigin::User => (&roots.user, &probe.user),
    };
    let relative = asset.absolute.strip_prefix(source_root).map_err(|_| {
        format!(
            "audit asset {} does not belong to its declared {:?} root",
            asset.emitted, asset.origin
        )
    })?;
    Ok(probe_root.join(relative))
}

fn replace_reachability_scalars(value: &mut Value) -> usize {
    match value {
        Value::Sequence(sequence) => sequence.iter_mut().map(replace_reachability_scalars).sum(),
        Value::Mapping(mapping) => {
            let mut replaced = 0;
            for (key, value) in mapping {
                let is_reachability = key.as_str().is_some_and(|key| {
                    key == REACHABILITY_KEY
                        || key
                            .trim_end_matches("/=")
                            .trim_end_matches("/+")
                            .ends_with(&format!("/{REACHABILITY_KEY}"))
                });
                if is_reachability && config_scalar_bool(value).is_some() {
                    *value = Value::String(PROBE_MARKER.to_owned());
                    replaced += 1;
                } else {
                    replaced += replace_reachability_scalars(value);
                }
            }
            replaced
        }
        _ => 0,
    }
}

fn tuple_sort_key(tuple: &JsonValue) -> (&str, &str, &str) {
    (
        tuple["schemaAsset"].as_str().unwrap_or_default(),
        tuple["component"].as_str().unwrap_or_default(),
        tuple["namespace"].as_str().unwrap_or_default(),
    )
}

fn path_to_forward_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
