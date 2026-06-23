use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Read,
    os::raw::c_int,
    path::Path,
    sync::{Arc, Mutex, OnceLock},
    time::UNIX_EPOCH,
};

use regex::Regex;
use serde_yaml::{Mapping, Value};
use yune_core::{
    parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_table_bin_dictionary, rime_dict_source_checksum, rime_table_bin_dict_file_checksum,
    CharsetFilter, DictionaryLookupFilter, EchoTranslator, HistoryTranslator, ReverseLookupFilter,
    ReverseLookupTranslator, SchemaListTranslator, SimplifierFilter, SingleCharFilter,
    StaticTableTranslator, SwitchTranslator, TableDictionary, TaggedFilter, Translator,
    UniquifierFilter, TYPEDUCK_SENTENCE_WORD_PENALTY,
};

use crate::{
    config_scalar_bool, config_scalar_double, config_scalar_int, config_scalar_string,
    ends_with_ascii_digit, find_config_value, install_schema_punctuation_translator_from_config,
    load_runtime_config_root, resource_id::validate_data_resource_id, schema_folded_switch_options,
    schema_list_translator_config_for_current, schema_switch_translator_switches,
    selected_runtime_data_path, startup_trace, switch_scalar_field, AffixSegmentor, ConfigOpenKind,
    MatcherPattern, MatcherSegmentor, PunctSegmentor, RemainingGearDeferral, SessionState,
};

pub(crate) fn install_schema_translator_chain(session: &mut SessionState, schema_id: &str) {
    let schema_config = {
        let _trace = startup_trace::span("schema_config_load");
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed)
    };
    let Some(Value::Sequence(translators)) =
        find_config_value(&schema_config, "engine/translators")
    else {
        return;
    };
    let mut punctuation_translator_installed = false;
    let mut echo_translator_installed = false;

    for translator in translators.iter().filter_map(Value::as_str) {
        let (component_name, name_space) = schema_component_prescription(translator);
        match component_name {
            "punct_translator" if !punctuation_translator_installed => {
                install_schema_punctuation_translator_from_config(session, &schema_config);
                punctuation_translator_installed = true;
            }
            "table_translator" | "script_translator" | "r10n_translator" => {
                install_schema_dictionary_translator_from_config(
                    session,
                    &schema_config,
                    component_name,
                    name_space.unwrap_or("translator"),
                );
            }
            "reverse_lookup_translator" => install_schema_reverse_lookup_translator_from_config(
                session,
                &schema_config,
                match name_space {
                    Some("translator") | None => "reverse_lookup",
                    Some(name_space) => name_space,
                },
            ),
            "history_translator" => install_schema_history_translator_from_config(
                session,
                &schema_config,
                match name_space {
                    Some("translator") | None => "history",
                    Some(name_space) => name_space,
                },
            ),
            "switch_translator" => {
                install_schema_switch_translator_from_config(session, &schema_config);
            }
            "schema_list_translator" => {
                let schema_list_config = schema_list_translator_config_for_current(
                    session.engine.status().schema_id.as_str(),
                    &schema_config,
                );
                session.engine.add_translator(
                    SchemaListTranslator::new(schema_list_config.entries)
                        .with_hide_lone_schema(schema_list_config.hide_lone_schema),
                );
            }
            "echo_translator" if !echo_translator_installed => {
                session.engine.add_translator(EchoTranslator);
                echo_translator_installed = true;
            }
            "memory" => record_remaining_gear_deferral(
                session,
                "memory",
                "user dictionary memory and learning",
                "deferred because LevelDB/userdb learning is outside Phase 3",
                "05-userdb-and-learning",
            ),
            "poet" => record_remaining_gear_deferral(
                session,
                "poet",
                "grammar/model-assisted candidate scoring",
                "deferred because plugin/model behavior is outside Phase 3",
                "04-compiled-dictionary-data",
            ),
            "grammar" => record_remaining_gear_deferral(
                session,
                "grammar",
                "grammar/model-assisted candidate scoring",
                "deferred because plugin/model behavior is outside Phase 3",
                "04-compiled-dictionary-data",
            ),
            "contextual_translation" => record_remaining_gear_deferral(
                session,
                "contextual_translation",
                "context-aware translation using reverse/context data",
                "deferred because compiled reverse/context data is outside Phase 3",
                "04-compiled-dictionary-data",
            ),
            "unity_table_encoder" => record_remaining_gear_deferral(
                session,
                "unity_table_encoder",
                "encodes phrases into UniTE table data",
                "deferred because compiled UniTE/table payload support is outside Phase 3",
                "04-compiled-dictionary-data",
            ),
            _ => {}
        }
    }
}

pub(crate) fn schema_component_prescription(component: &str) -> (&str, Option<&str>) {
    let Some((component_name, name_space)) = component.split_once('@') else {
        return (component, None);
    };
    if component_name.is_empty() || name_space.is_empty() {
        (component, None)
    } else {
        (component_name, Some(name_space))
    }
}

type SharedTranslator = Arc<dyn Translator>;
const FULL_CONTENT_CACHE_HASH_LIMIT: u64 = 64 * 1024;
const HEADER_CACHE_READ_LIMIT: usize = 16 * 1024;

static DICTIONARY_TRANSLATOR_CACHE: OnceLock<Mutex<HashMap<String, SharedTranslator>>> =
    OnceLock::new();

fn dictionary_translator_cache() -> &'static Mutex<HashMap<String, SharedTranslator>> {
    DICTIONARY_TRANSLATOR_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn install_schema_dictionary_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    component_name: &str,
    name_space: &str,
) {
    let user_dict_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
        .and_then(config_scalar_string)
        .and_then(|name| validate_data_resource_id(&name));
    let is_typeduck_jyut6ping3_profile =
        is_typeduck_jyut6ping3_profile(schema_config, user_dict_name.as_deref());
    let is_upstream_luna_pinyin_profile =
        is_upstream_luna_pinyin_profile(schema_config, user_dict_name.as_deref(), component_name);
    let cache_key =
        schema_dictionary_translator_cache_key(schema_config, component_name, name_space);
    let enable_charset_filter = find_config_value(
        schema_config,
        &format!("{name_space}/enable_charset_filter"),
    )
    .and_then(config_scalar_bool)
    .unwrap_or(false);
    let enable_sentence =
        find_config_value(schema_config, &format!("{name_space}/enable_sentence"))
            .and_then(config_scalar_bool)
            .unwrap_or(true);
    let sentence_over_completion = find_config_value(
        schema_config,
        &format!("{name_space}/sentence_over_completion"),
    )
    .and_then(config_scalar_bool)
    .unwrap_or(false);
    let mut enable_completion =
        find_config_value(schema_config, &format!("{name_space}/enable_completion"))
            .and_then(config_scalar_bool)
            .unwrap_or(true);
    let enable_correction =
        find_config_value(schema_config, &format!("{name_space}/enable_correction"))
            .and_then(config_scalar_bool)
            .unwrap_or(false);
    if matches!(component_name, "script_translator" | "r10n_translator") {
        if let Some(enable_word_completion) = find_config_value(
            schema_config,
            &format!("{name_space}/enable_word_completion"),
        )
        .and_then(config_scalar_bool)
        {
            enable_completion = enable_word_completion;
        }
    }
    let combine_candidates =
        find_config_value(schema_config, &format!("{name_space}/combine_candidates"))
            .and_then(config_scalar_bool)
            .unwrap_or(matches!(
                component_name,
                "script_translator" | "r10n_translator"
            ));
    let prefix = find_config_value(schema_config, &format!("{name_space}/prefix"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let suffix = find_config_value(schema_config, &format!("{name_space}/suffix"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let has_affix = !prefix.is_empty();
    let show_full_code = find_config_value(schema_config, &format!("{name_space}/show_full_code"))
        .and_then(config_scalar_bool)
        .unwrap_or(!has_affix);
    let prediction_weight_threshold = find_config_value(
        schema_config,
        &format!("{name_space}/prediction_weight_threshold"),
    )
    .or_else(|| {
        find_config_value(
            schema_config,
            &format!("{name_space}/prediction_frequency_threshold"),
        )
    })
    .or_else(|| {
        find_config_value(
            schema_config,
            &format!("{name_space}/prediction/frequency_threshold"),
        )
    })
    .and_then(config_scalar_f32);
    let prediction_never_first = find_config_value(
        schema_config,
        &format!("{name_space}/prediction_never_first"),
    )
    .or_else(|| {
        find_config_value(
            schema_config,
            &format!("{name_space}/prediction-never-first"),
        )
    })
    .or_else(|| {
        find_config_value(
            schema_config,
            &format!("{name_space}/prediction/never_first"),
        )
    })
    .and_then(config_scalar_bool)
    .unwrap_or(false);
    let prediction_candidate_limit = find_config_value(
        schema_config,
        &format!("{name_space}/prediction_candidate_limit"),
    )
    .or_else(|| {
        find_config_value(
            schema_config,
            &format!("{name_space}/prediction/candidate_limit"),
        )
    })
    .or_else(|| find_config_value(schema_config, &format!("{name_space}/prediction_limit")))
    .and_then(config_scalar_int)
    .and_then(|limit| usize::try_from(limit).ok())
    // TypeDuck v1.1.2 calibrates jyut6ping3 to one long prediction before
    // single-character rows; see jyut6ping3-m21-prediction-ranking.json.
    .or_else(|| is_typeduck_jyut6ping3_profile.then_some(1));
    let prefix_fallback =
        find_config_value(schema_config, &format!("{name_space}/prefix_fallback"))
            .or_else(|| {
                find_config_value(
                    schema_config,
                    &format!("{name_space}/partial_parse_prefix_fallback"),
                )
            })
            .and_then(config_scalar_bool)
            .unwrap_or(is_typeduck_jyut6ping3_profile);
    let delimiters = find_config_value(schema_config, &format!("{name_space}/delimiter"))
        .or_else(|| find_config_value(schema_config, "speller/delimiter"))
        .and_then(config_scalar_string)
        .unwrap_or_else(|| " ".to_owned());
    let tags = schema_translator_tags(schema_config, name_space);
    let initial_quality =
        find_config_value(schema_config, &format!("{name_space}/initial_quality"))
            .and_then(config_scalar_f32)
            .unwrap_or(0.0);
    let comment_format = schema_comment_format(schema_config, name_space);
    let preedit_format = schema_preedit_format(schema_config, name_space);
    let dictionary_exclude =
        schema_string_list(schema_config, &format!("{name_space}/dictionary_exclude"));
    let spelling_algebra = spelling_algebra_for_dictionary(schema_config);
    if let Some(user_dict_name) = &user_dict_name {
        session.set_user_dict_name(user_dict_name.clone());
    }
    if prediction_never_first {
        session.engine.set_prediction_never_first(true);
    }
    if let Some(translator) = cache_key
        .as_ref()
        .and_then(|key| cached_dictionary_translator(key))
    {
        session.engine.add_shared_translator(translator);
        return;
    }

    let dictionary = match load_schema_table_dictionary(schema_config, name_space) {
        DictionaryLoadOutcome::Compiled(dictionary) => dictionary,
        DictionaryLoadOutcome::SourceFallback { dictionary, reason } => {
            record_dictionary_source_fallback(session, reason);
            dictionary
        }
        DictionaryLoadOutcome::NoUsablePath {
            dictionary_id,
            reason,
        } => {
            record_dictionary_load_failure(session, dictionary_id, reason);
            return;
        }
    };
    let mut translator = {
        let _trace = startup_trace::span("translator_index_build");
        StaticTableTranslator::from_dictionary(dictionary)
    }
    .with_completion(enable_completion)
    .with_correction(enable_correction)
    .with_dynamic_correction_lookup(is_typeduck_jyut6ping3_profile)
    .with_charset_filter(enable_charset_filter)
    .with_sentence(enable_sentence)
    .with_sentence_over_completion(sentence_over_completion)
    .with_delimiters(delimiters)
    .with_tags(tags)
    .with_initial_quality(initial_quality)
    .with_comment_format(&comment_format)
    .with_preedit_format(&preedit_format)
    .with_dictionary_exclude(dictionary_exclude)
    .with_combine_candidates(combine_candidates)
    .with_affix(prefix, suffix)
    .with_show_full_code(show_full_code)
    .with_prediction_never_first(prediction_never_first)
    .with_prefix_fallback(prefix_fallback);
    {
        let _trace = startup_trace::span("spelling_algebra_expand");
        translator = translator.with_spelling_algebra(&spelling_algebra);
    }
    if let Some(threshold) = prediction_weight_threshold {
        translator = translator.with_prediction_weight_threshold(threshold);
    }
    if let Some(limit) = prediction_candidate_limit {
        translator = translator.with_prediction_candidate_limit(limit);
    }
    if is_upstream_luna_pinyin_profile {
        translator = translator.with_upstream_sentence_model(100);
    }
    if is_typeduck_jyut6ping3_profile {
        translator = translator.with_sentence_word_penalty(TYPEDUCK_SENTENCE_WORD_PENALTY);
    }
    let translator: SharedTranslator = Arc::new(translator);
    if let Some(cache_key) = cache_key {
        dictionary_translator_cache()
            .lock()
            .expect("dictionary translator cache should not be poisoned")
            .insert(cache_key, Arc::clone(&translator));
    }
    session.engine.add_shared_translator(translator);
}

fn cached_dictionary_translator(cache_key: &str) -> Option<SharedTranslator> {
    dictionary_translator_cache()
        .lock()
        .expect("dictionary translator cache should not be poisoned")
        .get(cache_key)
        .map(Arc::clone)
}

fn schema_dictionary_translator_cache_key(
    schema_config: &Value,
    component_name: &str,
    name_space: &str,
) -> Option<String> {
    let raw_dictionary_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let dictionary_name = validate_data_resource_id(&raw_dictionary_name)?;
    let prism_name = find_config_value(schema_config, &format!("{name_space}/prism"))
        .and_then(config_scalar_string)
        .and_then(|name| validate_data_resource_id(&name))
        .unwrap_or_else(|| dictionary_name.clone());
    let schema_fingerprint = serde_yaml::to_string(schema_config)
        .ok()
        .map(|schema| stable_hash_bytes(schema.as_bytes()))
        .unwrap_or_default();
    let mut parts = vec![
        format!("component={component_name}"),
        format!("namespace={name_space}"),
        format!("schema={schema_fingerprint:016x}"),
        format!("dictionary={dictionary_name}"),
        format!("prism={prism_name}"),
    ];
    let mut visited_sources = HashSet::new();
    append_source_dictionary_cache_signature(&mut parts, &dictionary_name, &mut visited_sources);
    for pack in schema_dictionary_packs(schema_config, name_space) {
        append_source_dictionary_cache_signature(&mut parts, &pack, &mut visited_sources);
    }
    append_runtime_file_metadata_signature(
        &mut parts,
        "table",
        &format!("{dictionary_name}.table.bin"),
    );
    append_runtime_file_metadata_signature(&mut parts, "prism", &format!("{prism_name}.prism.bin"));
    append_runtime_file_metadata_signature(
        &mut parts,
        "reverse",
        &format!("{dictionary_name}.reverse.bin"),
    );
    Some(parts.join("\n"))
}

fn append_source_dictionary_cache_signature(
    parts: &mut Vec<String>,
    dictionary_name: &str,
    visited: &mut HashSet<String>,
) {
    let Some(dictionary_name) = validate_data_resource_id(dictionary_name) else {
        parts.push(format!("source:{dictionary_name}:invalid"));
        return;
    };
    if !visited.insert(dictionary_name.clone()) {
        return;
    }
    let resource_id = format!("{dictionary_name}.dict.yaml");
    let Some((path, len, modified, bytes)) =
        read_runtime_data_file_signature(&resource_id, HEADER_CACHE_READ_LIMIT)
    else {
        parts.push(format!("source:{resource_id}:missing"));
        return;
    };
    parts.push(format!(
        "source:{}:{len}:{modified}:{:016x}",
        path.display(),
        stable_hash_bytes(&bytes)
    ));
    let yaml = String::from_utf8_lossy(&bytes);
    let (imports, vocabularies) = source_dictionary_header_dependencies(&yaml);
    for import in imports {
        append_source_dictionary_cache_signature(parts, &import, visited);
    }
    for vocabulary in vocabularies {
        let Some(vocabulary) = validate_data_resource_id(&vocabulary) else {
            parts.push(format!("vocabulary:{vocabulary}:invalid"));
            continue;
        };
        append_runtime_file_content_signature(parts, "vocabulary", &format!("{vocabulary}.txt"));
    }
}

fn append_runtime_file_content_signature(parts: &mut Vec<String>, role: &str, resource_id: &str) {
    let Some((path, len, modified, bytes)) =
        read_runtime_data_file_signature(resource_id, HEADER_CACHE_READ_LIMIT)
    else {
        parts.push(format!("{role}:{resource_id}:missing"));
        return;
    };
    parts.push(format!(
        "{role}:{}:{len}:{modified}:{:016x}",
        path.display(),
        stable_hash_bytes(&bytes)
    ));
}

fn append_runtime_file_metadata_signature(parts: &mut Vec<String>, role: &str, resource_id: &str) {
    let Some(resource_id) = validate_data_resource_id(resource_id) else {
        parts.push(format!("{role}:{resource_id}:invalid"));
        return;
    };
    let Some(path) = selected_runtime_data_path(&resource_id) else {
        parts.push(format!("{role}:{resource_id}:missing"));
        return;
    };
    let Ok(metadata) = fs::metadata(&path) else {
        parts.push(format!("{role}:{}:metadata-unavailable", path.display()));
        return;
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    let prefix_hash = file_prefix_hash(&path, 96).unwrap_or_default();
    parts.push(format!(
        "{role}:{}:{}:{modified}:{prefix_hash:016x}",
        path.display(),
        metadata.len()
    ));
}

fn read_runtime_data_file_signature(
    resource_id: &str,
    prefix_limit: usize,
) -> Option<(std::path::PathBuf, u64, u128, Vec<u8>)> {
    let resource_id = validate_data_resource_id(resource_id)?;
    let path = selected_runtime_data_path(&resource_id)?;
    let metadata = fs::metadata(&path).ok()?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    let len = metadata.len();
    let read_limit = if len <= FULL_CONTENT_CACHE_HASH_LIMIT {
        usize::try_from(len).ok()?
    } else {
        prefix_limit
    };
    let mut file = fs::File::open(&path).ok()?;
    let mut bytes = vec![0; read_limit];
    let read = file.read(&mut bytes).ok()?;
    bytes.truncate(read);
    Some((path, len, modified, bytes))
}

fn file_prefix_hash(path: &Path, limit: usize) -> Option<u64> {
    let mut file = fs::File::open(path).ok()?;
    let mut bytes = vec![0; limit];
    let read = file.read(&mut bytes).ok()?;
    Some(stable_hash_bytes(&bytes[..read]))
}

fn source_dictionary_header_dependencies(input: &str) -> (Vec<String>, Vec<String>) {
    let mut imports = Vec::new();
    let mut vocabularies = Vec::new();
    let mut active_list: Option<&str> = None;
    for raw_line in input.lines() {
        let trimmed = raw_line.trim();
        if trimmed == "..." {
            break;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" {
            continue;
        }
        if !raw_line.chars().next().is_some_and(char::is_whitespace) {
            active_list = None;
        }
        if let Some(value) = header_value(trimmed, "import_tables") {
            collect_yaml_values(value, &mut imports);
            if value.trim().is_empty() {
                active_list = Some("import_tables");
            }
            continue;
        }
        if let Some(value) = header_value(trimmed, "vocabulary") {
            collect_yaml_values(value, &mut vocabularies);
            continue;
        }
        if let Some(value) = header_value(trimmed, "use_preset_vocabulary") {
            if yaml_bool(value) == Some(true) {
                vocabularies.push("essay".to_owned());
            }
            continue;
        }
        if let Some(target) = active_list {
            if let Some(value) = trimmed.strip_prefix('-') {
                if target == "import_tables" {
                    collect_yaml_values(value, &mut imports);
                }
            }
        }
    }
    imports.sort();
    imports.dedup();
    vocabularies.sort();
    vocabularies.dedup();
    (imports, vocabularies)
}

fn header_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(key)?.strip_prefix(':')
}

fn collect_yaml_values(value: &str, output: &mut Vec<String>) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    if let Some(list) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        for item in list.split(',') {
            if let Some(value) = yaml_scalar(item) {
                output.push(value);
            }
        }
    } else if let Some(value) = yaml_scalar(value) {
        output.push(value);
    }
}

fn yaml_scalar(value: &str) -> Option<String> {
    let value = value.trim().trim_matches('"').trim_matches('\'').trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn yaml_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn is_typeduck_jyut6ping3_profile(schema_config: &Value, dictionary_name: Option<&str>) -> bool {
    if dictionary_name != Some("jyut6ping3") {
        return false;
    }
    find_config_value(schema_config, "schema/schema_id")
        .and_then(config_scalar_string)
        .is_some_and(|schema_id| schema_id == "jyut6ping3" || schema_id.starts_with("jyut6ping3_"))
}

fn is_upstream_luna_pinyin_profile(
    schema_config: &Value,
    dictionary_name: Option<&str>,
    component_name: &str,
) -> bool {
    if component_name != "script_translator" || dictionary_name != Some("luna_pinyin") {
        return false;
    }
    find_config_value(schema_config, "schema/schema_id")
        .and_then(config_scalar_string)
        .is_some_and(|schema_id| schema_id == "luna_pinyin")
}

fn spelling_algebra_for_dictionary(schema_config: &Value) -> Vec<String> {
    schema_string_list(schema_config, "speller/algebra")
}

fn install_schema_reverse_lookup_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let raw_dictionary_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    if validate_data_resource_id(&raw_dictionary_name).is_none() {
        record_dictionary_load_failure(
            session,
            raw_dictionary_name,
            DictionaryLoadFailure::InvalidResourceId,
        );
        return;
    }
    let target_namespace = find_config_value(schema_config, &format!("{name_space}/target"))
        .and_then(config_scalar_string)
        .filter(|target| !target.is_empty())
        .unwrap_or_else(|| "translator".to_owned());
    let prefix = find_config_value(schema_config, &format!("{name_space}/prefix"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let suffix = find_config_value(schema_config, &format!("{name_space}/suffix"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let tag = find_config_value(schema_config, &format!("{name_space}/tag"))
        .and_then(config_scalar_string)
        .unwrap_or_else(|| "reverse_lookup".to_owned());
    let enable_completion =
        find_config_value(schema_config, &format!("{name_space}/enable_completion"))
            .and_then(config_scalar_bool)
            .unwrap_or(false);
    let comment_format = schema_comment_format(schema_config, name_space);
    let lazy_schema_config = schema_config.clone();
    let lazy_name_space = name_space.to_owned();
    let lazy_target_namespace = target_namespace;

    session.engine.add_translator(
        ReverseLookupTranslator::new_lazy(
            move || {
                let dictionary = dictionary_from_lazy_outcome(load_schema_table_dictionary(
                    &lazy_schema_config,
                    &lazy_name_space,
                ))?;
                let reverse_dictionary =
                    load_schema_reverse_dictionary(&lazy_schema_config, &lazy_target_namespace)
                        .or_else(|| {
                            Some(load_schema_table_dictionary(
                                &lazy_schema_config,
                                &lazy_target_namespace,
                            ))
                        })
                        .and_then(dictionary_from_lazy_outcome);
                Some((dictionary, reverse_dictionary))
            },
            prefix,
            suffix,
        )
        .with_tag(tag)
        .with_completion(enable_completion)
        .with_comment_format(&comment_format),
    );
}

fn dictionary_from_lazy_outcome(outcome: DictionaryLoadOutcome) -> Option<TableDictionary> {
    match outcome {
        DictionaryLoadOutcome::Compiled(dictionary)
        | DictionaryLoadOutcome::SourceFallback { dictionary, .. } => Some(dictionary),
        DictionaryLoadOutcome::NoUsablePath { .. } => None,
    }
}

fn install_schema_history_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let input = find_config_value(schema_config, &format!("{name_space}/input"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let size = find_config_value(schema_config, &format!("{name_space}/size"))
        .and_then(config_scalar_int)
        .and_then(|size| usize::try_from(size).ok())
        .unwrap_or(1);
    let initial_quality =
        find_config_value(schema_config, &format!("{name_space}/initial_quality"))
            .and_then(config_scalar_double)
            .map(|quality| quality as f32)
            .unwrap_or(1000.0);
    let tag = find_config_value(schema_config, &format!("{name_space}/tag"))
        .and_then(config_scalar_string)
        .unwrap_or_else(|| "abc".to_owned());

    session.engine.add_translator(
        HistoryTranslator::new(input)
            .with_size(size)
            .with_initial_quality(initial_quality)
            .with_tag(tag),
    );
}

fn install_schema_switch_translator_from_config(session: &mut SessionState, schema_config: &Value) {
    let switches = schema_switch_translator_switches(schema_config);
    if switches.is_empty() {
        return;
    }
    let fold_options = find_config_value(schema_config, "switcher/fold_options")
        .and_then(config_scalar_bool)
        .unwrap_or(false);
    session.engine.set_option("_fold_options", fold_options);
    session.engine.add_translator(
        SwitchTranslator::new(switches)
            .with_folded_options(schema_folded_switch_options(schema_config)),
    );
}

pub(crate) fn install_schema_filter_chain(session: &mut SessionState, schema_id: &str) {
    let schema_config = {
        let _trace = startup_trace::span("schema_config_load");
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed)
    };
    let Some(Value::Sequence(filters)) = find_config_value(&schema_config, "engine/filters") else {
        return;
    };
    for filter in filters.iter().filter_map(Value::as_str) {
        let (filter_name, name_space) = schema_component_prescription(filter);
        match filter_name {
            "reverse_lookup_filter" => install_schema_reverse_lookup_filter_from_config(
                session,
                &schema_config,
                match name_space {
                    Some("filter") | None => "reverse_lookup",
                    Some(name_space) => name_space,
                },
            ),
            "dictionary_lookup_filter" => install_schema_dictionary_lookup_filter_from_config(
                session,
                &schema_config,
                name_space.unwrap_or("dictionary_lookup_filter"),
            ),
            "simplifier" => install_schema_simplifier_filter_from_config(
                session,
                &schema_config,
                match name_space {
                    Some("filter") | None => "simplifier",
                    Some(name_space) => name_space,
                },
            ),
            "uniquifier" => session.engine.add_filter(UniquifierFilter),
            "single_char_filter" => session.engine.add_filter(SingleCharFilter),
            "charset_filter" | "cjk_minifier" => {
                let tags = schema_filter_tags(&schema_config, name_space.unwrap_or(filter_name));
                session
                    .engine
                    .add_filter(TaggedFilter::new(CharsetFilter, tags));
            }
            "memory" => record_remaining_gear_deferral(
                session,
                "memory",
                "user dictionary memory and learning",
                "deferred because LevelDB/userdb learning is outside Phase 3",
                "05-userdb-and-learning",
            ),
            "poet" => record_remaining_gear_deferral(
                session,
                "poet",
                "grammar/model-assisted candidate scoring",
                "deferred because plugin/model behavior is outside Phase 3",
                "04-compiled-dictionary-data",
            ),
            "grammar" => record_remaining_gear_deferral(
                session,
                "grammar",
                "grammar/model-assisted candidate scoring",
                "deferred because plugin/model behavior is outside Phase 3",
                "04-compiled-dictionary-data",
            ),
            "contextual_translation" => record_remaining_gear_deferral(
                session,
                "contextual_translation",
                "context-aware translation using reverse/context data",
                "deferred because compiled reverse/context data is outside Phase 3",
                "04-compiled-dictionary-data",
            ),
            "unity_table_encoder" => record_remaining_gear_deferral(
                session,
                "unity_table_encoder",
                "encodes phrases into UniTE table data",
                "deferred because compiled UniTE/table payload support is outside Phase 3",
                "04-compiled-dictionary-data",
            ),
            _ => {}
        }
    }
}

fn record_remaining_gear_deferral(
    session: &mut SessionState,
    gear: &str,
    observed_librime_role: &str,
    scope_decision: &str,
    target_phase: &str,
) {
    if session
        .remaining_gear_deferrals
        .iter()
        .any(|deferral| deferral.gear == gear)
    {
        return;
    }
    session
        .remaining_gear_deferrals
        .push(RemainingGearDeferral {
            gear: gear.to_owned(),
            observed_librime_role: observed_librime_role.to_owned(),
            current_yune_behavior: "recognized during schema installation as a deterministic no-op"
                .to_owned(),
            scope_decision: scope_decision.to_owned(),
            target_phase: target_phase.to_owned(),
        });
}

pub(crate) fn apply_schema_switch_resets(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(switches)) = find_config_value(&schema_config, "switches") else {
        return;
    };

    for the_switch in switches {
        let Value::Mapping(switch_map) = the_switch else {
            continue;
        };
        let Some(reset_value) = switch_reset_value(switch_map) else {
            continue;
        };

        if let Some(option_name) = switch_scalar_field(switch_map, "name") {
            session.engine.set_option(option_name, reset_value != 0);
            continue;
        }

        let Some(Value::Sequence(options)) = switch_map.get(Value::String("options".to_owned()))
        else {
            continue;
        };
        for (option_index, option) in options.iter().enumerate() {
            let Some(option_name) = config_scalar_string(option) else {
                continue;
            };
            session
                .engine
                .set_option(option_name, option_index as c_int == reset_value);
        }
    }
}

pub(crate) fn switch_reset_value(switch_map: &Mapping) -> Option<c_int> {
    let reset = switch_map.get(Value::String("reset".to_owned()))?;
    match reset {
        Value::Null | Value::Sequence(_) | Value::Mapping(_) => None,
        scalar => Some(config_scalar_int(scalar).unwrap_or(0)),
    }
}

fn schema_filter_tags(schema_config: &Value, name_space: &str) -> Vec<String> {
    schema_string_list(schema_config, &format!("{name_space}/tags"))
}

fn install_schema_reverse_lookup_filter_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let reverse_dictionary = match load_schema_reverse_dictionary(schema_config, name_space)
        .or_else(|| Some(load_schema_table_dictionary(schema_config, name_space)))
    {
        Some(DictionaryLoadOutcome::Compiled(dictionary)) => dictionary,
        Some(DictionaryLoadOutcome::SourceFallback { dictionary, reason }) => {
            record_dictionary_source_fallback(session, reason);
            dictionary
        }
        Some(DictionaryLoadOutcome::NoUsablePath {
            dictionary_id,
            reason,
        }) => {
            record_dictionary_load_failure(session, dictionary_id, reason);
            return;
        }
        None => return,
    };

    let overwrite_comment =
        find_config_value(schema_config, &format!("{name_space}/overwrite_comment"))
            .and_then(config_scalar_bool)
            .unwrap_or(false);
    let append_comment = find_config_value(schema_config, &format!("{name_space}/append_comment"))
        .and_then(config_scalar_bool)
        .unwrap_or(false);
    let comment_format = schema_comment_format(schema_config, name_space);

    let tags = schema_filter_tags(schema_config, name_space);
    session.engine.add_filter(TaggedFilter::new(
        ReverseLookupFilter::new(reverse_dictionary)
            .with_overwrite_comment(overwrite_comment)
            .with_append_comment(append_comment)
            .with_comment_format(&comment_format),
        tags,
    ));
}

fn install_schema_dictionary_lookup_filter_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let raw_dictionary_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let Some(dictionary_name) = validate_data_resource_id(&raw_dictionary_name) else {
        record_dictionary_load_failure(
            session,
            raw_dictionary_name,
            DictionaryLoadFailure::InvalidResourceId,
        );
        return;
    };
    let source_yaml = load_schema_source_dictionary_yaml(&dictionary_name);
    let dictionary = match source_yaml.as_deref() {
        Some(dictionary_yaml) if has_typeduck_lookup_source_rows(dictionary_yaml) => {
            match TableDictionary::parse_typeduck_lookup_dict_yaml(dictionary_yaml) {
                Ok(dictionary) => dictionary,
                Err(_) => {
                    record_dictionary_load_failure(
                        session,
                        dictionary_name,
                        DictionaryLoadFailure::SourceInvalid,
                    );
                    return;
                }
            }
        }
        _ => match load_schema_dictionary_by_name(
            schema_config,
            name_space,
            dictionary_name.clone(),
            false,
        ) {
            DictionaryLoadOutcome::Compiled(dictionary) => dictionary,
            DictionaryLoadOutcome::SourceFallback { dictionary, reason } => {
                record_dictionary_source_fallback(session, reason);
                dictionary
            }
            DictionaryLoadOutcome::NoUsablePath { reason, .. } => {
                record_dictionary_load_failure(session, dictionary_name, reason);
                return;
            }
        },
    };
    let tags = schema_filter_tags(schema_config, name_space);
    session.engine.add_filter(TaggedFilter::new(
        DictionaryLookupFilter::new(dictionary),
        tags,
    ));
}

fn install_schema_simplifier_filter_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let option_name = find_config_value(schema_config, &format!("{name_space}/option_name"))
        .and_then(config_scalar_string)
        .filter(|option_name| !option_name.is_empty())
        .unwrap_or_else(|| "simplification".to_owned());
    let tips = find_config_value(schema_config, &format!("{name_space}/tips"))
        .or_else(|| find_config_value(schema_config, &format!("{name_space}/tip")))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let opencc_config = find_config_value(schema_config, &format!("{name_space}/opencc_config"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    let show_in_comment =
        find_config_value(schema_config, &format!("{name_space}/show_in_comment"))
            .and_then(config_scalar_bool)
            .unwrap_or(false);
    let inherit_comment =
        find_config_value(schema_config, &format!("{name_space}/inherit_comment"))
            .and_then(config_scalar_bool)
            .unwrap_or(true);
    let comment_format = schema_comment_format(schema_config, name_space);
    let excluded_types = schema_string_list(schema_config, &format!("{name_space}/excluded_types"));

    let tags = schema_filter_tags(schema_config, name_space);
    session.engine.add_filter(TaggedFilter::new(
        SimplifierFilter::new()
            .with_option_name(option_name)
            .with_opencc_config(opencc_config)
            .with_tips(tips)
            .with_show_in_comment(show_in_comment)
            .with_inherit_comment(inherit_comment)
            .with_comment_format(&comment_format)
            .with_excluded_types(excluded_types),
        tags,
    ));
}

#[derive(Clone, Debug)]
enum DictionaryLoadOutcome {
    Compiled(TableDictionary),
    SourceFallback {
        dictionary: TableDictionary,
        reason: CompiledRejectReason,
    },
    NoUsablePath {
        dictionary_id: String,
        reason: DictionaryLoadFailure,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CompiledRejectReason {
    Missing,
    Stale,
    Invalid(String),
    Unsupported(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum DictionaryLoadFailure {
    InvalidResourceId,
    SourceMissing,
    SourceInvalid,
    CompiledRejected(CompiledRejectReason),
}

fn load_schema_table_dictionary(schema_config: &Value, name_space: &str) -> DictionaryLoadOutcome {
    let raw_dictionary_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
        .and_then(config_scalar_string)
        .unwrap_or_default();
    load_schema_dictionary_by_name(schema_config, name_space, raw_dictionary_name, true)
}

fn load_schema_reverse_dictionary(
    schema_config: &Value,
    name_space: &str,
) -> Option<DictionaryLoadOutcome> {
    let reverse_name =
        find_config_value(schema_config, &format!("{name_space}/reverse_dictionary"))
            .or_else(|| find_config_value(schema_config, &format!("{name_space}/dictionary")))
            .and_then(config_scalar_string)?;
    Some(load_schema_dictionary_by_name(
        schema_config,
        name_space,
        reverse_name,
        false,
    ))
}

fn load_schema_dictionary_by_name(
    schema_config: &Value,
    name_space: &str,
    raw_dictionary_name: String,
    require_prism: bool,
) -> DictionaryLoadOutcome {
    let Some(dictionary_name) = validate_data_resource_id(&raw_dictionary_name) else {
        return DictionaryLoadOutcome::NoUsablePath {
            dictionary_id: raw_dictionary_name,
            reason: DictionaryLoadFailure::InvalidResourceId,
        };
    };

    let source_yaml = load_schema_source_dictionary_yaml(&dictionary_name);
    let prism_name = find_config_value(schema_config, &format!("{name_space}/prism"))
        .and_then(config_scalar_string)
        .and_then(|name| validate_data_resource_id(&name))
        .unwrap_or_else(|| dictionary_name.clone());
    let compiled = load_schema_compiled_dictionary(
        &dictionary_name,
        &prism_name,
        source_yaml.as_ref(),
        require_prism,
    );
    match compiled {
        Ok(dictionary) => DictionaryLoadOutcome::Compiled(dictionary),
        Err(reason) => match source_yaml {
            Some(dictionary_yaml) => {
                let parsed = {
                    let _trace = startup_trace::span("source_dictionary_parse_if_any");
                    parse_schema_source_dictionary(schema_config, name_space, &dictionary_yaml)
                };
                match parsed {
                    Ok(dictionary) => DictionaryLoadOutcome::SourceFallback { dictionary, reason },
                    Err(_) => DictionaryLoadOutcome::NoUsablePath {
                        dictionary_id: dictionary_name,
                        reason: DictionaryLoadFailure::SourceInvalid,
                    },
                }
            }
            None => {
                let failure = if reason == CompiledRejectReason::Missing {
                    DictionaryLoadFailure::SourceMissing
                } else {
                    DictionaryLoadFailure::CompiledRejected(reason)
                };
                DictionaryLoadOutcome::NoUsablePath {
                    dictionary_id: dictionary_name,
                    reason: failure,
                }
            }
        },
    }
}

fn load_schema_compiled_dictionary(
    dictionary_name: &str,
    prism_name: &str,
    source_yaml: Option<&String>,
    require_prism: bool,
) -> Result<TableDictionary, CompiledRejectReason> {
    let table_name = validate_data_resource_id(&format!("{dictionary_name}.table.bin"))
        .ok_or_else(|| CompiledRejectReason::Invalid("invalid table resource id".to_owned()))?;
    let prism_name = validate_data_resource_id(&format!("{prism_name}.prism.bin"))
        .ok_or_else(|| CompiledRejectReason::Invalid("invalid prism resource id".to_owned()))?;
    let reverse_name = validate_data_resource_id(&format!("{dictionary_name}.reverse.bin"))
        .ok_or_else(|| CompiledRejectReason::Invalid("invalid reverse resource id".to_owned()))?;
    let Some(table_path) = selected_runtime_data_path(&table_name) else {
        return Err(CompiledRejectReason::Missing);
    };
    let prism_path = selected_runtime_data_path(&prism_name);
    if require_prism && prism_path.is_none() {
        return Err(CompiledRejectReason::Missing);
    }
    let Some(reverse_path) = selected_runtime_data_path(&reverse_name) else {
        return Err(CompiledRejectReason::Missing);
    };
    let table_bytes = {
        let _trace = startup_trace::span("compiled_table_load");
        fs::read(table_path)
            .map_err(|error| CompiledRejectReason::Invalid(format!("table read failed: {error}")))?
    };
    let prism_bytes = {
        let _trace = startup_trace::span("compiled_prism_load");
        prism_path
            .map(fs::read)
            .transpose()
            .map_err(|error| CompiledRejectReason::Invalid(format!("prism read failed: {error}")))?
    };
    let reverse_bytes = {
        let _trace = startup_trace::span("compiled_reverse_load");
        fs::read(reverse_path).map_err(|error| {
            CompiledRejectReason::Invalid(format!("reverse read failed: {error}"))
        })?
    };

    if let Some(source_yaml) = source_yaml {
        let source_checksum = rime_dict_source_checksum(0, [source_yaml.as_bytes()], None);
        if rime_table_bin_dict_file_checksum(&table_bytes) != Some(source_checksum) {
            return Err(CompiledRejectReason::Stale);
        }
    }

    let prism_payload = {
        let _trace = startup_trace::span("compiled_prism_load");
        prism_bytes
            .as_ref()
            .map(parse_rime_prism_bin_payload)
            .transpose()
            .map_err(|error| match error {
                yune_core::RimePrismBinParseError::UnsupportedSection { role } => {
                    CompiledRejectReason::Unsupported(role)
                }
                other => CompiledRejectReason::Invalid(format!("prism parse failed: {other:?}")),
            })?
    };
    let reverse_dictionary = {
        let _trace = startup_trace::span("compiled_reverse_load");
        parse_rime_reverse_bin_dictionary(&reverse_bytes).map_err(|error| match error {
            yune_core::RimeReverseBinParseError::UnsupportedSection { role } => {
                CompiledRejectReason::Unsupported(role)
            }
            other => CompiledRejectReason::Invalid(format!("reverse parse failed: {other:?}")),
        })?
    };
    {
        let _trace = startup_trace::span("compiled_table_load");
        parse_rime_table_bin_dictionary(&table_bytes)
    }
    .map(|dictionary| {
        let mut dictionary = dictionary.with_merged_advanced_data_from(&reverse_dictionary);
        if let Some(prism_payload) = prism_payload {
            dictionary =
                dictionary.with_merged_advanced_data_from(&TableDictionary::with_advanced_data(
                    Vec::new(),
                    yune_core::TableDictionaryAdvancedData {
                        corrections: prism_payload.corrections,
                        tolerance_rules: prism_payload.tolerance_rules,
                        ..yune_core::TableDictionaryAdvancedData::default()
                    },
                ));
        }
        dictionary
    })
    .map_err(|error| match error {
        yune_core::RimeTableBinParseError::UnsupportedSection { role } => {
            CompiledRejectReason::Unsupported(role)
        }
        other => CompiledRejectReason::Invalid(format!("table parse failed: {other:?}")),
    })
}

fn load_schema_source_dictionary_yaml(dictionary_name: &str) -> Option<String> {
    let dictionary_path = selected_runtime_data_path(&format!("{dictionary_name}.dict.yaml"))?;
    fs::read_to_string(dictionary_path).ok()
}

pub(crate) fn has_typeduck_lookup_source_rows(dictionary_yaml: &str) -> bool {
    let mut in_body = false;
    let mut comments_enabled = true;

    for line in dictionary_yaml.lines() {
        let line = line.trim_end();
        if !in_body {
            if line.trim() == "..." {
                in_body = true;
            }
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        if comments_enabled && line.starts_with('#') {
            if line == "# no comment" {
                comments_enabled = false;
            }
            continue;
        }

        let Some((payload, text)) = line.split_once('\t') else {
            continue;
        };
        return !text.is_empty() && payload.matches(',').count() >= 2;
    }

    false
}

fn parse_schema_source_dictionary(
    schema_config: &Value,
    name_space: &str,
    dictionary_yaml: &str,
) -> Result<TableDictionary, yune_core::TableDictionaryParseError> {
    let packs = schema_dictionary_packs(schema_config, name_space);
    TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        dictionary_yaml,
        packs,
        |import_table| {
            let import_table = validate_data_resource_id(import_table)?;
            selected_runtime_data_path(&format!("{import_table}.dict.yaml"))
                .and_then(|path| fs::read_to_string(path).ok())
        },
        |vocabulary| {
            let vocabulary = validate_data_resource_id(vocabulary)?;
            selected_runtime_data_path(&format!("{vocabulary}.txt"))
                .and_then(|path| fs::read_to_string(path).ok())
        },
    )
}

fn record_dictionary_source_fallback(session: &mut SessionState, reason: CompiledRejectReason) {
    if reason == CompiledRejectReason::Missing {
        return;
    }
    let current_yune_behavior = format!("source fallback after compiled reject: {reason:?}");
    if session
        .remaining_gear_deferrals
        .iter()
        .any(|deferral| deferral.current_yune_behavior == current_yune_behavior)
    {
        return;
    }
    session
        .remaining_gear_deferrals
        .push(RemainingGearDeferral {
        gear: "dictionary_source_fallback".to_owned(),
        observed_librime_role: "compiled dictionary reject with source fallback".to_owned(),
        current_yune_behavior,
        scope_decision:
            "prefer source dictionary when compiled data is missing, stale, unsupported, or invalid"
                .to_owned(),
        target_phase: "04-compiled-dictionary-data".to_owned(),
    });
}

fn record_dictionary_load_failure(
    session: &mut SessionState,
    dictionary_id: String,
    reason: DictionaryLoadFailure,
) {
    let current_yune_behavior =
        format!("NoUsablePath for dictionary '{dictionary_id}': {reason:?}");
    if session
        .remaining_gear_deferrals
        .iter()
        .any(|deferral| deferral.gear == "dictionary_load")
    {
        return;
    }
    session
        .remaining_gear_deferrals
        .push(RemainingGearDeferral {
            gear: "dictionary_load".to_owned(),
            observed_librime_role: "schema dictionary installation failure".to_owned(),
            current_yune_behavior,
            scope_decision:
                "record explicit dictionary load failure instead of installing an empty translator"
                    .to_owned(),
            target_phase: "04-compiled-dictionary-data".to_owned(),
        });
}

fn schema_dictionary_packs(schema_config: &Value, name_space: &str) -> Vec<String> {
    let Some(Value::Sequence(packs)) =
        find_config_value(schema_config, &format!("{name_space}/packs"))
    else {
        return Vec::new();
    };
    packs
        .iter()
        .filter_map(config_scalar_string)
        .filter_map(|pack| validate_data_resource_id(&pack))
        .collect()
}

fn schema_comment_format(schema_config: &Value, name_space: &str) -> Vec<String> {
    schema_string_list(schema_config, &format!("{name_space}/comment_format"))
}

fn schema_preedit_format(schema_config: &Value, name_space: &str) -> Vec<String> {
    schema_string_list(schema_config, &format!("{name_space}/preedit_format"))
}

fn schema_translator_tags(schema_config: &Value, name_space: &str) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(tag) = find_config_value(schema_config, &format!("{name_space}/tag"))
        .and_then(config_scalar_string)
    {
        tags.push(tag);
    }
    tags.extend(schema_string_list(
        schema_config,
        &format!("{name_space}/tags"),
    ));
    if tags.is_empty() {
        tags.push("abc".to_owned());
    }
    tags
}

pub(crate) fn schema_string_list(schema_config: &Value, key: &str) -> Vec<String> {
    let Some(Value::Sequence(formulas)) = find_config_value(schema_config, key) else {
        return Vec::new();
    };
    formulas.iter().filter_map(config_scalar_string).collect()
}

pub(crate) fn install_schema_segment_tags(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let mut tags = vec!["abc".to_owned()];
    session.affix_segmentors.clear();
    session.matcher_segmentor = None;
    session.ascii_segmentor_enabled = false;
    session.punct_segmentor = None;
    session.fallback_segmentor_enabled = false;

    if let Some(Value::Sequence(segmentors)) =
        find_config_value(&schema_config, "engine/segmentors")
    {
        tags.clear();
        session.ascii_segmentor_enabled = segmentors
            .iter()
            .filter_map(Value::as_str)
            .map(schema_component_prescription)
            .any(|(component_name, _)| component_name == "ascii_segmentor");
        if segmentors
            .iter()
            .filter_map(Value::as_str)
            .map(schema_component_prescription)
            .any(|(component_name, _)| component_name == "abc_segmentor")
        {
            tags.push("abc".to_owned());
            tags.extend(schema_string_list(
                &schema_config,
                "abc_segmentor/extra_tags",
            ));
        }
        if segmentors
            .iter()
            .filter_map(Value::as_str)
            .map(schema_component_prescription)
            .any(|(component_name, _)| component_name == "punct_segmentor")
        {
            session.punct_segmentor = Some(load_schema_punct_segmentor(&schema_config));
        }
        session.affix_segmentors = load_schema_affix_segmentors(&schema_config, segmentors);
        session.matcher_segmentor = load_schema_matcher_segmentor(&schema_config, segmentors);
        session.fallback_segmentor_enabled = segmentors
            .iter()
            .filter_map(Value::as_str)
            .map(schema_component_prescription)
            .any(|(component_name, _)| component_name == "fallback_segmentor");
    }
    session.base_segment_tags = tags;
    update_session_segment_tags(session);
}

fn load_schema_matcher_segmentor(
    schema_config: &Value,
    segmentors: &[Value],
) -> Option<MatcherSegmentor> {
    let name_space = segmentors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .find_map(|(component_name, name_space)| {
            (component_name == "matcher")
                .then(|| {
                    let name_space = name_space.unwrap_or("recognizer");
                    if name_space == "segmentor" {
                        "recognizer"
                    } else {
                        name_space
                    }
                })
                .filter(|name_space| !name_space.is_empty())
        })?;
    let patterns = load_schema_recognizer_patterns(schema_config, name_space);
    (!patterns.is_empty()).then_some(MatcherSegmentor { patterns })
}

fn load_schema_affix_segmentors(
    schema_config: &Value,
    segmentors: &[Value],
) -> Vec<AffixSegmentor> {
    segmentors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .filter_map(|(component_name, name_space)| {
            if component_name != "affix_segmentor" {
                return None;
            }
            let name_space = name_space.unwrap_or("segmentor");
            if name_space.is_empty() {
                return None;
            }
            let prefix = find_config_value(schema_config, &format!("{name_space}/prefix"))
                .and_then(config_scalar_string)
                .unwrap_or_default();
            if prefix.is_empty() {
                return None;
            }
            let tag = find_config_value(schema_config, &format!("{name_space}/tag"))
                .and_then(config_scalar_string)
                .filter(|tag| !tag.is_empty())
                .unwrap_or_else(|| "abc".to_owned());
            let suffix = find_config_value(schema_config, &format!("{name_space}/suffix"))
                .and_then(config_scalar_string)
                .unwrap_or_default();
            let tips = find_config_value(schema_config, &format!("{name_space}/tips"))
                .and_then(config_scalar_string)
                .unwrap_or_default();
            let extra_tags = schema_string_list(schema_config, &format!("{name_space}/extra_tags"));
            Some(AffixSegmentor {
                tag,
                prefix,
                suffix,
                tips,
                extra_tags,
            })
        })
        .collect()
}

fn load_schema_punct_segmentor(schema_config: &Value) -> PunctSegmentor {
    PunctSegmentor {
        half_shape_keys: punctuation_shape_segment_keys(schema_config, "half_shape"),
        full_shape_keys: punctuation_shape_segment_keys(schema_config, "full_shape"),
        digit_separators: find_config_value(schema_config, "punctuator/digit_separators")
            .and_then(config_scalar_string)
            .unwrap_or_else(|| ".:".to_owned()),
    }
}

fn punctuation_shape_segment_keys(schema_config: &Value, shape: &str) -> HashSet<String> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return HashSet::new();
    };
    mapping
        .keys()
        .filter_map(config_scalar_string)
        .filter(|key| {
            let mut chars = key.chars();
            chars
                .next()
                .is_some_and(|ch| ch.is_ascii() && !ch.is_ascii_control())
                && chars.next().is_none()
        })
        .collect()
}

pub(crate) fn load_schema_recognizer_patterns(
    schema_config: &Value,
    name_space: &str,
) -> Vec<MatcherPattern> {
    let Some(Value::Mapping(patterns)) =
        find_config_value(schema_config, &format!("{name_space}/patterns"))
    else {
        return Vec::new();
    };
    let mut patterns = patterns
        .iter()
        .filter_map(|(tag, pattern)| {
            let tag = config_scalar_string(tag)?;
            let pattern = config_scalar_string(pattern)?;
            Regex::new(&pattern)
                .ok()
                .map(|pattern| MatcherPattern { tag, pattern })
        })
        .collect::<Vec<_>>();
    patterns.sort_by(|left, right| left.tag.cmp(&right.tag));
    patterns
}

pub(crate) fn update_session_segment_tags(session: &mut SessionState) {
    let input = session.engine.context().composition.input.clone();
    if session.ascii_composer_inline_ascii && input.is_empty() {
        session.ascii_composer_inline_ascii = false;
        session.engine.set_option("ascii_mode", false);
    }
    if session.ascii_segmentor_enabled && session.engine.status().is_ascii_mode && !input.is_empty()
    {
        let raw_tags = vec!["raw".to_owned()];
        if session.engine.context().segment_tags != raw_tags {
            session.engine.set_segment_tags(raw_tags);
        }
        return;
    }
    if let Some(punct_segmentor) = &session.punct_segmentor {
        if let Some(tag) = punct_segmentor.tag_for_input(
            &input,
            session.engine.status().is_full_shape,
            session.engine.context().last_commit.as_deref(),
        ) {
            let punct_tags = vec![tag.to_owned()];
            if session.engine.context().segment_tags != punct_tags {
                session.engine.set_segment_tags(punct_tags);
            }
            return;
        }
    }
    let mut tags = session.base_segment_tags.clone();
    for affix_segmentor in &session.affix_segmentors {
        if affix_segmentor.matches(&input) {
            let mut affix_tags = vec![affix_segmentor.tag.clone()];
            for extra_tag in &affix_segmentor.extra_tags {
                if !affix_tags.iter().any(|existing| existing == extra_tag) {
                    affix_tags.push(extra_tag.clone());
                }
            }
            if session.engine.context().segment_tags != affix_tags {
                session.engine.set_segment_tags(affix_tags);
            }
            return;
        }
    }
    if let Some(matcher) = &session.matcher_segmentor {
        if let Some(tag) = matcher.match_tag(&input) {
            if !tags.iter().any(|existing| existing == tag) {
                tags.push(tag.to_owned());
            }
        }
    }
    if tags.is_empty() && session.fallback_segmentor_enabled && !input.is_empty() {
        tags.push("raw".to_owned());
    }
    if session.engine.context().segment_tags != tags {
        session.engine.set_segment_tags(tags);
    }
}

impl AffixSegmentor {
    pub(crate) fn prompt_preedit(&self, input: &str) -> Option<(String, usize)> {
        if self.tips.is_empty() {
            return None;
        }
        let code = self.stripped_code(input)?;
        let caret = code.len();
        Some((format!("{code}{}", self.tips), caret))
    }

    fn stripped_code<'a>(&self, input: &'a str) -> Option<&'a str> {
        let mut code = input.strip_prefix(&self.prefix)?;
        if code.is_empty() {
            return None;
        }
        if !self.suffix.is_empty() {
            code = code.strip_suffix(&self.suffix).unwrap_or(code);
        }
        if code.is_empty() {
            return None;
        }
        Some(code)
    }

    fn matches(&self, input: &str) -> bool {
        self.stripped_code(input).is_some()
    }
}
impl PunctSegmentor {
    fn tag_for_input(
        &self,
        input: &str,
        full_shape: bool,
        last_commit: Option<&str>,
    ) -> Option<&'static str> {
        if !self.accepts_input(input, full_shape) {
            return None;
        }
        if input
            .chars()
            .next()
            .is_some_and(|ch| self.digit_separators.contains(ch))
            && last_commit.is_some_and(ends_with_ascii_digit)
        {
            Some("punct_number")
        } else {
            Some("punct")
        }
    }

    fn accepts_input(&self, input: &str, full_shape: bool) -> bool {
        let keys = if full_shape {
            &self.full_shape_keys
        } else {
            &self.half_shape_keys
        };
        keys.contains(input)
    }
}

impl MatcherSegmentor {
    fn match_tag(&self, input: &str) -> Option<&str> {
        if input.is_empty() {
            return None;
        }
        self.patterns
            .iter()
            .find(|pattern| recognizer_pattern_matches(pattern, input))
            .map(|pattern| pattern.tag.as_str())
    }
}

pub(crate) fn recognizer_patterns_match(patterns: &[MatcherPattern], input: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| recognizer_pattern_matches(pattern, input))
}

fn recognizer_pattern_matches(pattern: &MatcherPattern, input: &str) -> bool {
    pattern
        .pattern
        .find(input)
        .is_some_and(|matched| matched.start() == 0 && matched.end() == input.len())
}

fn config_scalar_f32(value: &Value) -> Option<f32> {
    config_scalar_double(value).map(|number| number as f32)
}
