use std::collections::HashMap;

use serde_yaml::Value;
use yune_core::{KeyCode, KeyEvent, PunctuationTranslator};

use crate::{
    config_scalar_bool, config_scalar_string, ends_with_ascii_digit, find_config_value,
    load_runtime_config_root, schema_engine_processors_include, schema_engine_translators_include,
    shape_formatted_ascii_text, ConfigOpenKind, PunctuationProcessResult, PunctuationProcessor,
    SessionState,
};

pub(crate) fn install_schema_punctuation_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
) {
    let half_shape_entries = punctuation_entries_from_config(schema_config, "half_shape");
    let full_shape_entries = punctuation_entries_from_config(schema_config, "full_shape");
    let symbol_entries = punctuation_entries_from_config(schema_config, "symbols");
    if half_shape_entries.is_empty() && full_shape_entries.is_empty() && symbol_entries.is_empty() {
        return;
    }
    let translator = PunctuationTranslator::with_shape_and_symbol_entries(
        half_shape_entries,
        full_shape_entries,
        symbol_entries,
    );
    let translator = if session.punct_segmentor.is_some() {
        translator.with_required_tags(["punct", "punct_number"])
    } else {
        translator
    };
    session.engine.add_translator(translator);
}

pub(crate) fn install_schema_punctuation_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    if !schema_engine_processors_include(&schema_config, "punctuator")
        || !schema_engine_translators_include(&schema_config, "punct_translator")
    {
        return;
    }

    let processor = PunctuationProcessor {
        use_space: find_config_value(&schema_config, "punctuator/use_space")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
        digit_separators: find_config_value(&schema_config, "punctuator/digit_separators")
            .and_then(config_scalar_string)
            .unwrap_or_else(|| ".:".to_owned()),
        digit_separator_commit: find_config_value(
            &schema_config,
            "punctuator/digit_separator_action",
        )
        .and_then(config_scalar_string)
        .is_some_and(|action| action == "commit"),
        half_shape_alternating_values: punctuation_alternating_values_from_config(
            &schema_config,
            "half_shape",
        ),
        full_shape_alternating_values: punctuation_alternating_values_from_config(
            &schema_config,
            "full_shape",
        ),
        symbol_alternating_values: punctuation_alternating_values_from_config(
            &schema_config,
            "symbols",
        ),
        half_shape_immediate_commits: punctuation_immediate_commits_from_config(
            &schema_config,
            "half_shape",
        ),
        full_shape_immediate_commits: punctuation_immediate_commits_from_config(
            &schema_config,
            "full_shape",
        ),
        symbol_immediate_commits: punctuation_immediate_commits_from_config(
            &schema_config,
            "symbols",
        ),
        half_shape_unique_commits: punctuation_unique_commits_from_config(
            &schema_config,
            "half_shape",
        ),
        full_shape_unique_commits: punctuation_unique_commits_from_config(
            &schema_config,
            "full_shape",
        ),
        symbol_unique_commits: punctuation_unique_commits_from_config(&schema_config, "symbols"),
        half_shape_pairs: punctuation_pairs_from_config(&schema_config, "half_shape"),
        full_shape_pairs: punctuation_pairs_from_config(&schema_config, "full_shape"),
        symbol_pairs: punctuation_pairs_from_config(&schema_config, "symbols"),
        pair_oddness: HashMap::new(),
        pending_digit_separator: None,
    };
    if processor.half_shape_unique_commits.is_empty()
        && processor.full_shape_unique_commits.is_empty()
        && processor.symbol_unique_commits.is_empty()
        && processor.half_shape_immediate_commits.is_empty()
        && processor.full_shape_immediate_commits.is_empty()
        && processor.symbol_immediate_commits.is_empty()
        && processor.half_shape_alternating_values.is_empty()
        && processor.full_shape_alternating_values.is_empty()
        && processor.symbol_alternating_values.is_empty()
        && processor.half_shape_pairs.is_empty()
        && processor.full_shape_pairs.is_empty()
        && processor.symbol_pairs.is_empty()
    {
        return;
    }
    session.punctuation_processor = Some(processor);
}

fn punctuation_entries_from_config(schema_config: &Value, shape: &str) -> Vec<(String, String)> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    for (key, definition) in mapping {
        let Some(key) = config_scalar_string(key) else {
            continue;
        };
        append_punctuation_definition(&mut entries, &key, definition);
    }
    entries
}

fn punctuation_unique_commits_from_config(
    schema_config: &Value,
    shape: &str,
) -> HashMap<String, String> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return HashMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, definition)| {
            let key = config_scalar_string(key)?;
            let text = punctuation_unique_commit(definition)?;
            Some((key, text))
        })
        .collect()
}

fn punctuation_immediate_commits_from_config(
    schema_config: &Value,
    shape: &str,
) -> HashMap<String, String> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return HashMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, definition)| {
            let key = config_scalar_string(key)?;
            let text = punctuation_immediate_commit(definition)?;
            Some((key, text))
        })
        .collect()
}

fn punctuation_alternating_values_from_config(
    schema_config: &Value,
    shape: &str,
) -> HashMap<String, Vec<String>> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return HashMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, definition)| {
            let key = config_scalar_string(key)?;
            let Value::Sequence(values) = definition else {
                return None;
            };
            let values = values
                .iter()
                .filter_map(config_scalar_string)
                .collect::<Vec<_>>();
            (!values.is_empty()).then_some((key, values))
        })
        .collect()
}

fn punctuation_pairs_from_config(
    schema_config: &Value,
    shape: &str,
) -> HashMap<String, [String; 2]> {
    let Some(Value::Mapping(mapping)) =
        find_config_value(schema_config, &format!("punctuator/{shape}"))
    else {
        return HashMap::new();
    };

    mapping
        .iter()
        .filter_map(|(key, definition)| {
            let key = config_scalar_string(key)?;
            let pair = punctuation_pair(definition)?;
            Some((key, pair))
        })
        .collect()
}

fn punctuation_unique_commit(definition: &Value) -> Option<String> {
    config_scalar_string(definition)
}

fn punctuation_immediate_commit(definition: &Value) -> Option<String> {
    let Value::Mapping(mapping) = definition else {
        return None;
    };
    mapping
        .get(Value::String("commit".to_owned()))
        .and_then(config_scalar_string)
}

fn punctuation_pair(definition: &Value) -> Option<[String; 2]> {
    let Value::Mapping(mapping) = definition else {
        return None;
    };
    let Some(Value::Sequence(pair)) = mapping.get(Value::String("pair".to_owned())) else {
        return None;
    };
    if pair.len() != 2 {
        return None;
    }
    let first = config_scalar_string(&pair[0])?;
    let second = config_scalar_string(&pair[1])?;
    Some([first, second])
}

fn append_punctuation_definition(
    entries: &mut Vec<(String, String)>,
    key: &str,
    definition: &Value,
) {
    if let Some(text) = config_scalar_string(definition) {
        entries.push((key.to_owned(), text));
        return;
    }

    match definition {
        Value::Sequence(values) => {
            for value in values {
                if let Some(text) = config_scalar_string(value) {
                    entries.push((key.to_owned(), text));
                }
            }
        }
        Value::Mapping(mapping) => {
            let commit_key = Value::String("commit".to_owned());
            if let Some(text) = mapping.get(&commit_key).and_then(config_scalar_string) {
                entries.push((key.to_owned(), text));
                return;
            }

            let pair_key = Value::String("pair".to_owned());
            let Some(Value::Sequence(pair)) = mapping.get(&pair_key) else {
                return;
            };
            if pair.len() != 2 {
                return;
            }
            for value in pair {
                if let Some(text) = config_scalar_string(value) {
                    entries.push((key.to_owned(), text));
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn process_punctuation_processor(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<PunctuationProcessResult> {
    if key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.release
    {
        return None;
    }

    let KeyCode::Character(ch) = key_event.code else {
        return None;
    };
    if !ch.is_ascii() || ch.is_ascii_control() {
        return None;
    }

    let key = ch.to_string();
    if session.engine.get_option("ascii_punct") {
        return active_punctuation_definition_exists(session, &key)
            .then_some(PunctuationProcessResult::Noop);
    }

    if let Some(result) = process_pending_digit_separator(session, ch, &ch.to_string()) {
        return Some(result);
    }

    let use_space = session.punctuation_processor.as_ref()?.use_space;
    if ch == ' ' && !use_space && !session.engine.context().composition.input.is_empty() {
        return None;
    }

    if let Some(result) = process_digit_separator(session, ch, &key) {
        return Some(result);
    }

    if process_alternating_punct(session, &key) {
        return Some(PunctuationProcessResult::Accepted);
    }

    if let Some(commit) = active_immediate_commit(session, &key) {
        return Some(PunctuationProcessResult::Commit(commit));
    }

    if let Some(preview) = active_pair_preview(session, &key) {
        session.engine.set_punctuation_preview(preview);
        return Some(PunctuationProcessResult::Accepted);
    }

    if let Some(preview) = active_unique_preview(session, &key) {
        session.engine.set_punctuation_preview(preview);
        return Some(PunctuationProcessResult::Accepted);
    }

    None
}

fn process_digit_separator(
    session: &mut SessionState,
    ch: char,
    key: &str,
) -> Option<PunctuationProcessResult> {
    let is_digit_separator = session
        .punctuation_processor
        .as_ref()
        .is_some_and(|processor| processor.digit_separators.contains(ch));
    if !is_digit_separator
        || !session.engine.context().composition.input.is_empty()
        || !session
            .engine
            .context()
            .last_commit
            .as_deref()
            .is_some_and(ends_with_ascii_digit)
        || !active_punctuation_definition_exists(session, key)
    {
        return None;
    }

    let full_shape = session.engine.status().is_full_shape;
    let digit_separator_commit = session
        .punctuation_processor
        .as_ref()
        .is_some_and(|processor| processor.digit_separator_commit);
    let punct = shape_formatted_ascii_text(key, full_shape);
    if digit_separator_commit {
        return Some(PunctuationProcessResult::Commit(punct));
    }

    if let Some(processor) = session.punctuation_processor.as_mut() {
        processor.pending_digit_separator = Some(key.to_owned());
    }
    session
        .engine
        .set_punctuation_composition(key.to_owned(), punct);
    Some(PunctuationProcessResult::Accepted)
}

fn process_pending_digit_separator(
    session: &mut SessionState,
    ch: char,
    key: &str,
) -> Option<PunctuationProcessResult> {
    let pending = session
        .punctuation_processor
        .as_ref()
        .and_then(|processor| processor.pending_digit_separator.as_deref())?;
    if session.engine.context().composition.input != pending {
        if let Some(processor) = session.punctuation_processor.as_mut() {
            processor.pending_digit_separator = None;
        }
        return None;
    }

    if ch.is_ascii_digit() || ch == ' ' {
        let commit = shape_formatted_ascii_text(
            &format!("{pending}{ch}"),
            session.engine.status().is_full_shape,
        );
        if let Some(processor) = session.punctuation_processor.as_mut() {
            processor.pending_digit_separator = None;
        }
        return Some(PunctuationProcessResult::Commit(commit));
    }

    if key == pending {
        if let Some(processor) = session.punctuation_processor.as_mut() {
            processor.pending_digit_separator = None;
        }
        session.engine.set_input(key.to_owned());
        return Some(PunctuationProcessResult::Accepted);
    }

    None
}

fn active_punctuation_definition_exists(session: &SessionState, key: &str) -> bool {
    let Some(processor) = session.punctuation_processor.as_ref() else {
        return false;
    };
    let (shape_unique_commits, shape_immediate_commits, shape_alternating_values, shape_pairs) =
        if session.engine.status().is_full_shape {
            (
                &processor.full_shape_unique_commits,
                &processor.full_shape_immediate_commits,
                &processor.full_shape_alternating_values,
                &processor.full_shape_pairs,
            )
        } else {
            (
                &processor.half_shape_unique_commits,
                &processor.half_shape_immediate_commits,
                &processor.half_shape_alternating_values,
                &processor.half_shape_pairs,
            )
        };

    shape_unique_commits.contains_key(key)
        || shape_immediate_commits.contains_key(key)
        || shape_alternating_values.contains_key(key)
        || shape_pairs.contains_key(key)
        || processor.symbol_unique_commits.contains_key(key)
        || processor.symbol_immediate_commits.contains_key(key)
        || processor.symbol_alternating_values.contains_key(key)
        || processor.symbol_pairs.contains_key(key)
}

fn active_immediate_commit(session: &SessionState, key: &str) -> Option<String> {
    let processor = session.punctuation_processor.as_ref()?;
    let shape_commits = if session.engine.status().is_full_shape {
        &processor.full_shape_immediate_commits
    } else {
        &processor.half_shape_immediate_commits
    };
    shape_commits
        .get(key)
        .or_else(|| processor.symbol_immediate_commits.get(key))
        .cloned()
}

fn active_unique_preview(session: &SessionState, key: &str) -> Option<String> {
    let processor = session.punctuation_processor.as_ref()?;
    let shape_entries = if session.engine.status().is_full_shape {
        &processor.full_shape_unique_commits
    } else {
        &processor.half_shape_unique_commits
    };
    shape_entries
        .get(key)
        .or_else(|| processor.symbol_unique_commits.get(key))
        .cloned()
}

fn active_pair_preview(session: &mut SessionState, key: &str) -> Option<String> {
    let is_full_shape = session.engine.status().is_full_shape;
    let current_input = session.engine.context().composition.input.clone();
    let processor = session.punctuation_processor.as_mut()?;
    let shape_name = if is_full_shape {
        "full_shape"
    } else {
        "half_shape"
    };
    let shape_pairs = if is_full_shape {
        &processor.full_shape_pairs
    } else {
        &processor.half_shape_pairs
    };
    let (pair_name, pair) = shape_pairs
        .get(key)
        .map(|pair| (shape_name, pair))
        .or_else(|| {
            processor
                .symbol_pairs
                .get(key)
                .map(|pair| ("symbols", pair))
        })?;

    let oddness_key = format!("{pair_name}:{key}");
    let oddness = processor.pair_oddness.entry(oddness_key).or_insert(0);
    let punct = pair[*oddness % 2].clone();
    *oddness = 1 - (*oddness % 2);
    Some(format!("{current_input}{punct}"))
}

fn process_alternating_punct(session: &mut SessionState, key: &str) -> bool {
    let Some(values) = active_alternating_punct_values(session, key).cloned() else {
        return false;
    };
    if values.is_empty() {
        return false;
    }
    let context = session.engine.context();
    let next_index = if context.candidates.is_empty() || context.composition.input.is_empty() {
        0
    } else {
        (context.highlighted + 1) % values.len()
    };
    let input = values[next_index].clone();
    session
        .engine
        .set_punctuation_candidate_list(input, values, next_index);
    true
}

fn active_alternating_punct_values<'a>(
    session: &'a SessionState,
    key: &str,
) -> Option<&'a Vec<String>> {
    let processor = session.punctuation_processor.as_ref()?;
    let shape_values = if session.engine.status().is_full_shape {
        &processor.full_shape_alternating_values
    } else {
        &processor.half_shape_alternating_values
    };
    shape_values
        .get(key)
        .or_else(|| processor.symbol_alternating_values.get(key))
}
