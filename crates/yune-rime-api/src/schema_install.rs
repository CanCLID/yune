use std::{collections::HashSet, fs, os::raw::c_int};

use regex::Regex;
use serde_yaml::{Mapping, Value};
use yune_core::{
    parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_table_bin_dictionary, rime_dict_source_checksum, rime_table_bin_dict_file_checksum,
    CharsetFilter, HistoryTranslator, ReverseLookupFilter, ReverseLookupTranslator,
    SchemaListTranslator, SimplifierFilter, SingleCharFilter, StaticTableTranslator,
    SwitchTranslator, TableDictionary, TaggedFilter, UniquifierFilter,
};

use crate::{
    config_scalar_bool, config_scalar_double, config_scalar_int, config_scalar_string,
    ends_with_ascii_digit, find_config_value, install_schema_punctuation_translator_from_config,
    load_runtime_config_root, resource_id::validate_data_resource_id, schema_folded_switch_options,
    schema_list_translator_entries_for_current, schema_switch_translator_switches,
    selected_runtime_data_path, switch_scalar_field, AffixSegmentor, ConfigOpenKind,
    MatcherPattern, MatcherSegmentor, PunctSegmentor, RemainingGearDeferral, SessionState,
};

pub(crate) fn install_schema_translator_chain(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(translators)) =
        find_config_value(&schema_config, "engine/translators")
    else {
        return;
    };
    let mut punctuation_translator_installed = false;

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
                let entries = schema_list_translator_entries_for_current(
                    session.engine.status().schema_id.as_str(),
                    &schema_config,
                );
                session
                    .engine
                    .add_translator(SchemaListTranslator::new(entries));
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

fn install_schema_dictionary_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    component_name: &str,
    name_space: &str,
) {
    let dictionary = match load_schema_table_dictionary(schema_config, name_space) {
        DictionaryLoadOutcome::Compiled(dictionary) => dictionary,
        DictionaryLoadOutcome::SourceFallback { dictionary, reason } => {
            record_dictionary_source_fallback(session, reason);
            dictionary
        }
        DictionaryLoadOutcome::NoUsablePath { dictionary_id, reason } => {
            record_dictionary_load_failure(session, dictionary_id, reason);
            return;
        }
    };
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
    let dictionary_exclude =
        schema_string_list(schema_config, &format!("{name_space}/dictionary_exclude"));
    let spelling_algebra = schema_string_list(schema_config, "speller/algebra");
    session.engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_spelling_algebra(&spelling_algebra)
            .with_completion(enable_completion)
            .with_charset_filter(enable_charset_filter)
            .with_sentence(enable_sentence)
            .with_sentence_over_completion(sentence_over_completion)
            .with_delimiters(delimiters)
            .with_tags(tags)
            .with_initial_quality(initial_quality)
            .with_comment_format(&comment_format)
            .with_dictionary_exclude(dictionary_exclude),
    );
}

fn install_schema_reverse_lookup_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    name_space: &str,
) {
    let dictionary = match load_schema_table_dictionary(schema_config, name_space) {
        DictionaryLoadOutcome::Compiled(dictionary) => dictionary,
        DictionaryLoadOutcome::SourceFallback { dictionary, reason } => {
            record_dictionary_source_fallback(session, reason);
            dictionary
        }
        DictionaryLoadOutcome::NoUsablePath { dictionary_id, reason } => {
            record_dictionary_load_failure(session, dictionary_id, reason);
            return;
        }
    };
    let target_namespace = find_config_value(schema_config, &format!("{name_space}/target"))
        .and_then(config_scalar_string)
        .filter(|target| !target.is_empty())
        .unwrap_or_else(|| "translator".to_owned());
    let reverse_dictionary = match load_schema_table_dictionary(schema_config, &target_namespace) {
        DictionaryLoadOutcome::Compiled(dictionary) => Some(dictionary),
        DictionaryLoadOutcome::SourceFallback { dictionary, reason } => {
            record_dictionary_source_fallback(session, reason);
            Some(dictionary)
        }
        DictionaryLoadOutcome::NoUsablePath { dictionary_id, reason } => {
            record_dictionary_load_failure(session, dictionary_id, reason);
            None
        }
    };
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

    session.engine.add_translator(
        ReverseLookupTranslator::new(dictionary, reverse_dictionary, prefix, suffix)
            .with_tag(tag)
            .with_completion(enable_completion)
            .with_comment_format(&comment_format),
    );
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
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
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
    let reverse_dictionary = match load_schema_table_dictionary(schema_config, name_space) {
        DictionaryLoadOutcome::Compiled(dictionary) => dictionary,
        DictionaryLoadOutcome::SourceFallback { dictionary, reason } => {
            record_dictionary_source_fallback(session, reason);
            dictionary
        }
        DictionaryLoadOutcome::NoUsablePath { dictionary_id, reason } => {
            record_dictionary_load_failure(session, dictionary_id, reason);
            return;
        }
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
    let Some(dictionary_name) = validate_data_resource_id(&raw_dictionary_name) else {
        return DictionaryLoadOutcome::NoUsablePath {
            dictionary_id: raw_dictionary_name,
            reason: DictionaryLoadFailure::InvalidResourceId,
        };
    };

    let source = load_schema_source_dictionary(schema_config, name_space, &dictionary_name);
    let compiled = load_schema_compiled_dictionary(&dictionary_name, source.as_ref().map(|(yaml, _)| yaml));
    match compiled {
        Ok(dictionary) => DictionaryLoadOutcome::Compiled(dictionary),
        Err(reason) => match source {
            Some((_, Ok(dictionary))) => DictionaryLoadOutcome::SourceFallback { dictionary, reason },
            Some((_, Err(_))) => DictionaryLoadOutcome::NoUsablePath {
                dictionary_id: dictionary_name,
                reason: DictionaryLoadFailure::SourceInvalid,
            },
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
    source_yaml: Option<&String>,
) -> Result<TableDictionary, CompiledRejectReason> {
    let table_name = validate_data_resource_id(&format!("{dictionary_name}.table.bin"))
        .ok_or_else(|| CompiledRejectReason::Invalid("invalid table resource id".to_owned()))?;
    let prism_name = validate_data_resource_id(&format!("{dictionary_name}.prism.bin"))
        .ok_or_else(|| CompiledRejectReason::Invalid("invalid prism resource id".to_owned()))?;
    let reverse_name = validate_data_resource_id(&format!("{dictionary_name}.reverse.bin"))
        .ok_or_else(|| CompiledRejectReason::Invalid("invalid reverse resource id".to_owned()))?;
    let Some(table_path) = selected_runtime_data_path(&table_name) else {
        return Err(CompiledRejectReason::Missing);
    };
    let Some(prism_path) = selected_runtime_data_path(&prism_name) else {
        return Err(CompiledRejectReason::Missing);
    };
    let Some(reverse_path) = selected_runtime_data_path(&reverse_name) else {
        return Err(CompiledRejectReason::Missing);
    };
    let table_bytes = fs::read(table_path)
        .map_err(|error| CompiledRejectReason::Invalid(format!("table read failed: {error}")))?;
    let prism_bytes = fs::read(prism_path)
        .map_err(|error| CompiledRejectReason::Invalid(format!("prism read failed: {error}")))?;
    let reverse_bytes = fs::read(reverse_path)
        .map_err(|error| CompiledRejectReason::Invalid(format!("reverse read failed: {error}")))?;

    if let Some(source_yaml) = source_yaml {
        let source_checksum = rime_dict_source_checksum(0, [source_yaml.as_bytes()], None);
        if rime_table_bin_dict_file_checksum(&table_bytes) != Some(source_checksum) {
            return Err(CompiledRejectReason::Stale);
        }
    }

    parse_rime_prism_bin_payload(&prism_bytes).map_err(|error| match error {
        yune_core::RimePrismBinParseError::UnsupportedSection { role } => {
            CompiledRejectReason::Unsupported(role)
        }
        other => CompiledRejectReason::Invalid(format!("prism parse failed: {other:?}")),
    })?;
    parse_rime_reverse_bin_dictionary(&reverse_bytes).map_err(|error| match error {
        yune_core::RimeReverseBinParseError::UnsupportedSection { role } => {
            CompiledRejectReason::Unsupported(role)
        }
        other => CompiledRejectReason::Invalid(format!("reverse parse failed: {other:?}")),
    })?;
    parse_rime_table_bin_dictionary(&table_bytes).map_err(|error| match error {
        yune_core::RimeTableBinParseError::UnsupportedSection { role } => {
            CompiledRejectReason::Unsupported(role)
        }
        other => CompiledRejectReason::Invalid(format!("table parse failed: {other:?}")),
    })
}

fn load_schema_source_dictionary(
    schema_config: &Value,
    name_space: &str,
    dictionary_name: &str,
) -> Option<(String, Result<TableDictionary, yune_core::TableDictionaryParseError>)> {
    let dictionary_path = selected_runtime_data_path(&format!("{dictionary_name}.dict.yaml"))?;
    let dictionary_yaml = fs::read_to_string(dictionary_path).ok()?;
    let packs = schema_dictionary_packs(schema_config, name_space);
    let result = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        &dictionary_yaml,
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
    );
    Some((dictionary_yaml, result))
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
    session.remaining_gear_deferrals.push(RemainingGearDeferral {
        gear: "dictionary_source_fallback".to_owned(),
        observed_librime_role: "compiled dictionary reject with source fallback".to_owned(),
        current_yune_behavior,
        scope_decision: "prefer source dictionary when compiled data is missing, stale, unsupported, or invalid"
            .to_owned(),
        target_phase: "04-compiled-dictionary-data".to_owned(),
    });
}

fn record_dictionary_load_failure(
    session: &mut SessionState,
    dictionary_id: String,
    reason: DictionaryLoadFailure,
) {
    let current_yune_behavior = format!("NoUsablePath for dictionary '{dictionary_id}': {reason:?}");
    if session
        .remaining_gear_deferrals
        .iter()
        .any(|deferral| deferral.gear == "dictionary_load")
    {
        return;
    }
    session.remaining_gear_deferrals.push(RemainingGearDeferral {
        gear: "dictionary_load".to_owned(),
        observed_librime_role: "schema dictionary installation failure".to_owned(),
        current_yune_behavior,
        scope_decision: "record explicit dictionary load failure instead of installing an empty translator"
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
            let extra_tags = schema_string_list(schema_config, &format!("{name_space}/extra_tags"));
            Some(AffixSegmentor {
                tag,
                prefix,
                suffix,
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
    fn matches(&self, input: &str) -> bool {
        let Some(mut code) = input.strip_prefix(&self.prefix) else {
            return false;
        };
        if code.is_empty() {
            return false;
        }
        if !self.suffix.is_empty() {
            code = code.strip_suffix(&self.suffix).unwrap_or(code);
        }
        !code.is_empty()
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
