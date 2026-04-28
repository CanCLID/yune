use std::collections::HashMap;

use serde_yaml::Value;
use yune_core::{KeyCode, KeyEvent};

use crate::{
    config_scalar_string, find_config_value, load_runtime_config_root,
    parse_single_key_binding_event, schema_engine_processors_include, ConfigOpenKind, EditorAction,
    EditorBindingAction, EditorCharHandler, EditorProcessor, SessionKeyProcessResult, SessionState,
};

pub(crate) fn install_schema_editor_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    if schema_engine_processors_include(&schema_config, "express_editor") {
        session.editor_processor = Some(EditorProcessor::Express);
        session.editor_char_handler = Some(EditorCharHandler::DirectCommit);
        session.engine.set_option("_auto_commit", true);
    } else if schema_engine_processors_include(&schema_config, "fluid_editor")
        || schema_engine_processors_include(&schema_config, "fluency_editor")
    {
        session.editor_processor = Some(EditorProcessor::Fluid);
        session.editor_char_handler = Some(EditorCharHandler::AddToInput);
        session.engine.set_option("_auto_commit", false);
    }
    if session.editor_processor.is_some() {
        load_editor_binding_section(&schema_config, &mut session.editor_bindings);
        if let Some(handler) = find_config_value(&schema_config, "editor/char_handler")
            .and_then(config_scalar_string)
            .and_then(|handler| editor_char_handler_from_name(&handler))
        {
            session.editor_char_handler = handler;
        }
    }
}

fn load_editor_binding_section(
    schema_config: &Value,
    bindings: &mut HashMap<KeyEvent, EditorBindingAction>,
) {
    let Some(Value::Mapping(config_bindings)) = find_config_value(schema_config, "editor/bindings")
    else {
        return;
    };

    for (key, action) in config_bindings {
        let Some(key) = config_scalar_string(key) else {
            continue;
        };
        let Some(key_event) = parse_single_key_binding_event(&key) else {
            continue;
        };
        let Some(action) = action.as_str().and_then(editor_binding_action_from_name) else {
            continue;
        };
        bindings.insert(key_event, action);
    }
}

fn editor_binding_action_from_name(action: &str) -> Option<EditorBindingAction> {
    let action = match action {
        "noop" => EditorBindingAction::Noop,
        "confirm" => EditorBindingAction::Action(EditorAction::Confirm),
        "toggle_selection" => EditorBindingAction::Action(EditorAction::ToggleSelection),
        "commit_comment" => EditorBindingAction::Action(EditorAction::CommitComment),
        "commit_raw_input" => EditorBindingAction::Action(EditorAction::CommitRawInput),
        "commit_script_text" => EditorBindingAction::Action(EditorAction::CommitScriptText),
        "commit_composition" => EditorBindingAction::Action(EditorAction::CommitComposition),
        "revert" => EditorBindingAction::Action(EditorAction::Revert),
        "back" => EditorBindingAction::Action(EditorAction::Back),
        "back_syllable" => EditorBindingAction::Action(EditorAction::BackSyllable),
        "delete_candidate" => EditorBindingAction::Action(EditorAction::DeleteCandidate),
        "delete" => EditorBindingAction::Action(EditorAction::Delete),
        "cancel" => EditorBindingAction::Action(EditorAction::Cancel),
        _ => return None,
    };
    Some(action)
}

fn editor_char_handler_from_name(handler: &str) -> Option<Option<EditorCharHandler>> {
    match handler {
        "direct_commit" => Some(Some(EditorCharHandler::DirectCommit)),
        "add_to_input" => Some(Some(EditorCharHandler::AddToInput)),
        "noop" => Some(None),
        _ => None,
    }
}

pub(crate) fn process_editor_processor(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<SessionKeyProcessResult> {
    if session.editor_processor.is_none() || key_event.modifiers.release {
        return None;
    }

    let is_composing = !session.engine.context().composition.input.is_empty();
    if is_composing {
        if let Some(action) = session.editor_bindings.get(&key_event).copied() {
            return match action {
                EditorBindingAction::Noop => Some(SessionKeyProcessResult::Accepted),
                EditorBindingAction::Action(action) => Some(apply_editor_action(session, action)),
            };
        }
    }

    if let Some(result) = process_editor_char_handler(session, key_event) {
        return Some(result);
    }

    if is_composing
        && session.editor_processor == Some(EditorProcessor::Express)
        && key_event.code == KeyCode::Return
    {
        if key_event.modifiers.is_empty() {
            let commit = session.engine.commit_raw_input();
            return Some(commit.map_or(
                SessionKeyProcessResult::Accepted,
                SessionKeyProcessResult::Commit,
            ));
        }

        if key_event.modifiers.control
            && !key_event.modifiers.shift
            && !key_event.modifiers.alt
            && !key_event.modifiers.super_key
            && !key_event.modifiers.hyper
            && !key_event.modifiers.meta
        {
            let commit = session.engine.commit_script_text();
            return Some(commit.map_or(
                SessionKeyProcessResult::Accepted,
                SessionKeyProcessResult::Commit,
            ));
        }
    }

    None
}

fn process_editor_char_handler(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<SessionKeyProcessResult> {
    if key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.hyper
        || key_event.modifiers.meta
    {
        return None;
    }
    let KeyCode::Character(ch) = key_event.code else {
        return None;
    };
    if !('\u{21}'..'\u{7f}').contains(&ch) {
        return None;
    }

    match session.editor_char_handler {
        Some(EditorCharHandler::AddToInput) => {
            let mut input = session.engine.context().composition.input.clone();
            input.push(ch);
            session.engine.set_input(input);
            Some(SessionKeyProcessResult::Accepted)
        }
        Some(EditorCharHandler::DirectCommit) => {
            let commit = session.engine.commit_composition();
            Some(commit.map_or(
                SessionKeyProcessResult::Noop,
                SessionKeyProcessResult::RejectedCommit,
            ))
        }
        None => Some(SessionKeyProcessResult::Noop),
    }
}

fn apply_editor_action(
    session: &mut SessionState,
    action: EditorAction,
) -> SessionKeyProcessResult {
    let commit = match action {
        EditorAction::Confirm | EditorAction::CommitComposition => {
            session.engine.commit_composition()
        }
        EditorAction::ToggleSelection => {
            session.engine.first_candidate();
            None
        }
        EditorAction::CommitComment => session.engine.commit_comment(),
        EditorAction::CommitRawInput => session.engine.commit_raw_input(),
        EditorAction::CommitScriptText => session.engine.commit_script_text(),
        EditorAction::Revert | EditorAction::Back | EditorAction::BackSyllable => {
            session.engine.back_to_previous_input();
            None
        }
        EditorAction::DeleteCandidate => {
            session
                .engine
                .delete_candidate(session.engine.context().highlighted);
            None
        }
        EditorAction::Delete => {
            session.engine.delete_input();
            None
        }
        EditorAction::Cancel => {
            session.engine.clear_composition();
            None
        }
    };
    commit.map_or(
        SessionKeyProcessResult::Accepted,
        SessionKeyProcessResult::Commit,
    )
}
