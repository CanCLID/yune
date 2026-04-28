use std::collections::HashMap;

use serde_yaml::Value;
use yune_core::{parse_key_sequence, Engine, KeyCode, KeyEvent};

use crate::{
    apply_schema_to_session, config_scalar_string, deployed_selected_schema_ids, find_config_value,
    load_runtime_config_root, notifications::notify, parse_single_key_binding_event,
    process_session_key_event, schema_engine_processors_include, switch_reset_value,
    switch_scalar_field, ConfigOpenKind, RimeSessionId, SessionKeyProcessResult, SessionState,
};

pub(crate) struct KeyBinderProcessor {
    bindings: HashMap<KeyEvent, Vec<KeyBinding>>,
    redirecting: bool,
    last_key: Option<char>,
}

impl KeyBinderProcessor {
    pub(crate) fn has_binding(&self, key_event: &KeyEvent) -> bool {
        self.bindings.contains_key(key_event)
    }
}

struct KeyBinding {
    condition: KeyBindingCondition,
    action: KeyBindingAction,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum KeyBindingCondition {
    Always,
    Composing,
    HasMenu,
    Paging,
}

#[derive(Clone)]
enum KeyBindingAction {
    Send(Vec<KeyEvent>),
    Toggle(String),
    SetOption { option: String, value: bool },
    SelectSchema(String),
}

struct KeyBindingSwitchOption {
    options: Vec<String>,
    option_index: usize,
    reset_index: usize,
}

enum KeyBindingSwitchTarget {
    Toggle(String),
    Radio(KeyBindingSwitchOption),
}

pub(crate) fn install_schema_key_binder_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    if !schema_engine_processors_include(&schema_config, "key_binder") {
        return;
    }
    let Some(Value::Sequence(bindings)) = find_config_value(&schema_config, "key_binder/bindings")
    else {
        return;
    };

    let mut processor = KeyBinderProcessor {
        bindings: HashMap::new(),
        redirecting: false,
        last_key: None,
    };
    for binding in bindings {
        let Value::Mapping(binding) = binding else {
            continue;
        };
        let Some(condition) = binding
            .get(Value::String("when".to_owned()))
            .and_then(config_scalar_string)
            .and_then(|condition| key_binding_condition(&condition))
        else {
            continue;
        };
        let Some(accept) = binding
            .get(Value::String("accept".to_owned()))
            .and_then(config_scalar_string)
        else {
            continue;
        };
        let Some(key_event) = parse_single_key_binding_event(&accept) else {
            continue;
        };
        let action = if let Some(send) = binding
            .get(Value::String("send".to_owned()))
            .and_then(config_scalar_string)
        {
            let Some(target) = parse_single_key_binding_event(&send) else {
                continue;
            };
            KeyBindingAction::Send(vec![target])
        } else if let Some(send_sequence) = binding
            .get(Value::String("send_sequence".to_owned()))
            .and_then(config_scalar_string)
        {
            let Ok(targets) = parse_key_sequence(&send_sequence) else {
                continue;
            };
            KeyBindingAction::Send(targets)
        } else if let Some(toggle) = binding
            .get(Value::String("toggle".to_owned()))
            .and_then(config_scalar_string)
        {
            KeyBindingAction::Toggle(toggle)
        } else if let Some(option) = binding
            .get(Value::String("set_option".to_owned()))
            .and_then(config_scalar_string)
        {
            KeyBindingAction::SetOption {
                option,
                value: true,
            }
        } else if let Some(option) = binding
            .get(Value::String("unset_option".to_owned()))
            .and_then(config_scalar_string)
        {
            KeyBindingAction::SetOption {
                option,
                value: false,
            }
        } else if let Some(schema) = binding
            .get(Value::String("select".to_owned()))
            .and_then(config_scalar_string)
        {
            KeyBindingAction::SelectSchema(schema)
        } else {
            continue;
        };
        insert_key_binding(
            processor.bindings.entry(key_event).or_default(),
            KeyBinding { condition, action },
        );
    }

    if !processor.bindings.is_empty() {
        session.key_binder = Some(processor);
    }
}

fn insert_key_binding(bindings: &mut Vec<KeyBinding>, binding: KeyBinding) {
    let rank = key_binding_condition_rank(binding.condition);
    let insertion_index = bindings
        .iter()
        .position(|existing| key_binding_condition_rank(existing.condition) >= rank)
        .unwrap_or(bindings.len());
    bindings.insert(insertion_index, binding);
}

fn key_binding_condition(condition: &str) -> Option<KeyBindingCondition> {
    match condition {
        "always" => Some(KeyBindingCondition::Always),
        "composing" => Some(KeyBindingCondition::Composing),
        "has_menu" => Some(KeyBindingCondition::HasMenu),
        "paging" => Some(KeyBindingCondition::Paging),
        _ => None,
    }
}

fn key_binding_condition_rank(condition: KeyBindingCondition) -> usize {
    match condition {
        KeyBindingCondition::Paging => 1,
        KeyBindingCondition::HasMenu => 2,
        KeyBindingCondition::Composing => 3,
        KeyBindingCondition::Always => 4,
    }
}

pub(crate) fn process_key_binder_processor(
    session_id: RimeSessionId,
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<Vec<String>> {
    {
        let processor = session.key_binder.as_mut()?;
        if processor.redirecting {
            return None;
        }
        if reinterpret_key_binding_paging_key(processor, &mut session.engine, key_event) {
            return None;
        }
    }

    let processor = session.key_binder.as_ref()?;
    let bindings = processor.bindings.get(&key_event)?;
    let binding_index = bindings
        .iter()
        .position(|binding| key_binding_condition_matches(session, binding.condition))?;

    let action = bindings[binding_index].action.clone();
    match action {
        KeyBindingAction::Send(events) => {
            Some(redirect_key_binding_events(session_id, session, events))
        }
        KeyBindingAction::Toggle(option) => {
            toggle_key_binding_option(session, &option);
            Some(Vec::new())
        }
        KeyBindingAction::SetOption { option, value } => {
            set_key_binding_option(session, &option, value);
            Some(Vec::new())
        }
        KeyBindingAction::SelectSchema(schema) => {
            select_key_binding_schema(session_id, session, &schema);
            Some(Vec::new())
        }
    }
}

fn reinterpret_key_binding_paging_key(
    processor: &mut KeyBinderProcessor,
    engine: &mut Engine,
    key_event: KeyEvent,
) -> bool {
    if key_event.modifiers.release {
        return false;
    }

    let ch = if key_event.modifiers.is_empty() {
        match key_event.code {
            KeyCode::Character(ch) => Some(ch),
            _ => None,
        }
    } else {
        None
    };

    if ch == Some('.') && matches!(processor.last_key, Some('.') | Some(',')) {
        processor.last_key = None;
        return false;
    }

    let mut reinterpreted = false;
    if processor.last_key == Some('.') && matches!(ch, Some('a'..='z')) {
        let input = &engine.context().composition.input;
        if !input.is_empty() && !input.ends_with('.') {
            engine.process_char('.');
            reinterpreted = true;
        }
    }

    processor.last_key = ch;
    reinterpreted
}

fn redirect_key_binding_events(
    session_id: RimeSessionId,
    session: &mut SessionState,
    events: Vec<KeyEvent>,
) -> Vec<String> {
    if let Some(processor) = session.key_binder.as_mut() {
        processor.redirecting = true;
    }
    let mut commits = Vec::new();
    for event in events {
        match process_session_key_event(session_id, session, event) {
            SessionKeyProcessResult::Commit(commit)
            | SessionKeyProcessResult::RejectedCommit(commit) => commits.push(commit),
            SessionKeyProcessResult::Noop | SessionKeyProcessResult::Accepted => {}
        }
    }
    if let Some(processor) = session.key_binder.as_mut() {
        processor.redirecting = false;
    }
    commits
}

fn key_binding_condition_matches(session: &SessionState, condition: KeyBindingCondition) -> bool {
    match condition {
        KeyBindingCondition::Always => true,
        KeyBindingCondition::Composing => !session.engine.context().composition.input.is_empty(),
        KeyBindingCondition::HasMenu => {
            !session.engine.status().is_ascii_mode
                && !session.engine.context().candidates.is_empty()
        }
        KeyBindingCondition::Paging => {
            session.paging && !session.engine.context().composition.input.is_empty()
        }
    }
}

pub(crate) fn update_key_binding_paging_state(
    session: &mut SessionState,
    key_event: KeyEvent,
    before_input: &str,
    before_highlighted: usize,
) {
    let context = session.engine.context();
    if context.composition.input.is_empty() {
        session.paging = false;
        return;
    }
    if context.composition.input != before_input {
        session.paging = false;
    }
    if matches!(
        key_event.code,
        KeyCode::PreviousPage
            | KeyCode::NextPage
            | KeyCode::PreviousCandidate
            | KeyCode::NextCandidate
    ) && context.highlighted != before_highlighted
    {
        session.paging = true;
    }
}

fn toggle_key_binding_option(session: &mut SessionState, option: &str) {
    if let Some(the_option) = key_binding_switch_by_index(session, option) {
        match the_option {
            KeyBindingSwitchTarget::Toggle(option) => {
                session
                    .engine
                    .set_option(option.clone(), !session.engine.get_option(&option));
            }
            KeyBindingSwitchTarget::Radio(the_option) => {
                toggle_key_binding_radio_option(session, &the_option);
            }
        }
        return;
    }

    if let Some(the_option) = key_binding_switch_option(session, option) {
        toggle_key_binding_radio_option(session, &the_option);
        return;
    }

    session
        .engine
        .set_option(option, !session.engine.get_option(option));
}

fn toggle_key_binding_radio_option(
    session: &mut SessionState,
    the_option: &KeyBindingSwitchOption,
) {
    let selected_index = the_option
        .options
        .iter()
        .position(|option| session.engine.get_option(option));
    let next_index = selected_index
        .map(|index| (index + 1) % the_option.options.len())
        .unwrap_or(the_option.option_index);
    select_key_binding_radio_option(session, &the_option.options, next_index);
}

fn set_key_binding_option(session: &mut SessionState, option: &str, value: bool) {
    if let Some(the_option) = key_binding_switch_option(session, option) {
        if value {
            select_key_binding_radio_option(session, &the_option.options, the_option.option_index);
        } else if session.engine.get_option(option) {
            select_key_binding_radio_option(session, &the_option.options, the_option.reset_index);
        }
        return;
    }

    session.engine.set_option(option, value);
}

fn select_key_binding_schema(session_id: RimeSessionId, session: &mut SessionState, schema: &str) {
    let selected_schema = if schema == ".next" {
        next_key_binding_schema(session)
    } else {
        Some(schema.to_owned())
    };
    let Some(selected_schema) = selected_schema else {
        return;
    };
    apply_schema_to_session(session, &selected_schema);
    let status = session.engine.status();
    notify(
        session_id,
        "schema",
        &format!("{}/{}", status.schema_id, status.schema_name),
    );
}

fn next_key_binding_schema(session: &SessionState) -> Option<String> {
    let current_schema = &session.engine.status().schema_id;
    deployed_selected_schema_ids()
        .into_iter()
        .find(|schema_id| schema_id != current_schema)
}

pub(crate) fn select_key_binding_radio_option(
    session: &mut SessionState,
    options: &[String],
    selected_index: usize,
) {
    if selected_index >= options.len() {
        return;
    }
    for (option_index, option) in options.iter().enumerate() {
        session
            .engine
            .set_option(option.clone(), option_index == selected_index);
    }
}

fn key_binding_switch_option(
    session: &SessionState,
    option_name: &str,
) -> Option<KeyBindingSwitchOption> {
    let schema_id = &session.engine.status().schema_id;
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Value::Sequence(switches) = find_config_value(&schema_config, "switches")? else {
        return None;
    };

    for the_switch in switches {
        let Value::Mapping(switch_map) = the_switch else {
            continue;
        };
        let Some(Value::Sequence(options)) = switch_map.get(Value::String("options".to_owned()))
        else {
            continue;
        };
        let options = options
            .iter()
            .filter_map(config_scalar_string)
            .collect::<Vec<_>>();
        let Some(option_index) = options.iter().position(|option| option == option_name) else {
            continue;
        };
        let reset_index = switch_reset_value(switch_map)
            .and_then(|reset| usize::try_from(reset).ok())
            .unwrap_or(0);
        return Some(KeyBindingSwitchOption {
            options,
            option_index,
            reset_index,
        });
    }
    None
}

fn key_binding_switch_by_index(
    session: &SessionState,
    option_name: &str,
) -> Option<KeyBindingSwitchTarget> {
    let switch_index = option_name.strip_prefix('@')?.parse::<usize>().ok()?;
    let schema_id = &session.engine.status().schema_id;
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Value::Sequence(switches) = find_config_value(&schema_config, "switches")? else {
        return None;
    };
    let Value::Mapping(switch_map) = switches.get(switch_index)? else {
        return None;
    };

    if let Some(option_name) = switch_scalar_field(switch_map, "name") {
        return Some(KeyBindingSwitchTarget::Toggle(option_name));
    }

    let Some(Value::Sequence(options)) = switch_map.get(Value::String("options".to_owned())) else {
        return None;
    };
    let options = options
        .iter()
        .filter_map(config_scalar_string)
        .collect::<Vec<_>>();
    if options.is_empty() {
        return None;
    }
    let reset_index = switch_reset_value(switch_map)
        .and_then(|reset| usize::try_from(reset).ok())
        .unwrap_or(0);
    Some(KeyBindingSwitchTarget::Radio(KeyBindingSwitchOption {
        options,
        option_index: 0,
        reset_index,
    }))
}
