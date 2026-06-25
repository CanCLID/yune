use std::{collections::HashMap, os::raw::c_int};

use serde_yaml::Value;
use yune_core::{KeyCode, KeyEvent};

use crate::{
    config_scalar_string, find_config_value, load_runtime_config_root,
    parse_single_key_binding_event, session_menu_page_size, ConfigOpenKind, SelectorBindingAction,
    SelectorLayoutAction, SessionState, XK_DOWN, XK_KP_DOWN, XK_KP_LEFT, XK_KP_PAGE_DOWN,
    XK_KP_PAGE_UP, XK_KP_RIGHT, XK_KP_UP, XK_LEFT, XK_PAGE_DOWN, XK_PAGE_UP, XK_RIGHT, XK_UP,
};

pub(crate) fn process_selector_layout_key(
    session: &mut SessionState,
    key_event: KeyEvent,
    keycode: c_int,
    mask: c_int,
) -> Option<bool> {
    if session.engine.context().composition.input.is_empty()
        || session.engine.context().candidates.is_empty()
        || session
            .engine
            .context()
            .segment_tags
            .iter()
            .any(|tag| tag == "raw")
    {
        return None;
    }

    let is_vertical = session.engine.get_option("_vertical");
    let is_linear =
        session.engine.get_option("_linear") || session.engine.get_option("_horizontal");
    if let Some(action) = selector_configured_action(session, is_vertical, is_linear, key_event) {
        return match action {
            SelectorBindingAction::Noop => Some(false),
            SelectorBindingAction::Action(action) => {
                apply_selector_layout_action(session, action, is_linear)
            }
        };
    }

    if mask != 0 {
        return None;
    }

    let action = selector_layout_action(is_vertical, is_linear, keycode)?;
    apply_selector_layout_action(session, action, is_linear)
}

pub(crate) fn selector_configured_action(
    session: &SessionState,
    is_vertical: bool,
    is_linear: bool,
    key_event: KeyEvent,
) -> Option<SelectorBindingAction> {
    let bindings = match (is_vertical, is_linear) {
        (false, false) => &session.selector_bindings.horizontal_stacked,
        (false, true) => &session.selector_bindings.horizontal_linear,
        (true, false) => &session.selector_bindings.vertical_stacked,
        (true, true) => &session.selector_bindings.vertical_linear,
    };
    bindings.get(&key_event).copied()
}

fn selector_layout_action(
    is_vertical: bool,
    is_linear: bool,
    keycode: c_int,
) -> Option<SelectorLayoutAction> {
    use SelectorLayoutAction::{NextCandidate, NextPage, PreviousCandidate, PreviousPage};

    match (is_vertical, is_linear, keycode) {
        (false, false, XK_UP | XK_KP_UP) => Some(PreviousCandidate),
        (false, false, XK_DOWN | XK_KP_DOWN) => Some(NextCandidate),
        (false, true, XK_LEFT | XK_KP_LEFT) => Some(PreviousCandidate),
        (false, true, XK_RIGHT | XK_KP_RIGHT) => Some(NextCandidate),
        (false, true, XK_UP | XK_KP_UP) => Some(PreviousPage),
        (false, true, XK_DOWN | XK_KP_DOWN) => Some(NextPage),
        (true, false, XK_RIGHT | XK_KP_RIGHT) => Some(PreviousCandidate),
        (true, false, XK_LEFT | XK_KP_LEFT) => Some(NextCandidate),
        (true, true, XK_UP | XK_KP_UP) => Some(PreviousCandidate),
        (true, true, XK_DOWN | XK_KP_DOWN) => Some(NextCandidate),
        (true, true, XK_RIGHT | XK_KP_RIGHT) => Some(PreviousPage),
        (true, true, XK_LEFT | XK_KP_LEFT) => Some(NextPage),
        (_, _, XK_PAGE_UP | XK_KP_PAGE_UP) => Some(PreviousPage),
        (_, _, XK_PAGE_DOWN | XK_KP_PAGE_DOWN) => Some(NextPage),
        _ => None,
    }
}

fn apply_selector_layout_action(
    session: &mut SessionState,
    action: SelectorLayoutAction,
    is_linear: bool,
) -> Option<bool> {
    match action {
        SelectorLayoutAction::PreviousCandidate => {
            selector_previous_candidate_like_librime(session, is_linear)
        }
        SelectorLayoutAction::NextCandidate => {
            selector_next_candidate_like_librime(session, is_linear)
        }
        SelectorLayoutAction::PreviousPage => {
            selector_previous_page_like_librime(session);
            Some(true)
        }
        SelectorLayoutAction::NextPage => {
            selector_next_page_like_librime(session);
            Some(true)
        }
        SelectorLayoutAction::Home => selector_home_like_librime(session),
        SelectorLayoutAction::End => selector_end_like_librime(session),
    }
}

fn selector_previous_candidate_like_librime(
    session: &mut SessionState,
    is_linear: bool,
) -> Option<bool> {
    let context = session.engine.context();
    if is_linear && context.composition.caret < context.composition.input.len() {
        return None;
    }
    let highlighted = context.highlighted;
    if highlighted == 0 {
        return (!is_linear).then_some(true);
    }
    session.engine.highlight_candidate(highlighted - 1);
    session.paging = true;
    Some(true)
}

fn selector_next_candidate_like_librime(
    session: &mut SessionState,
    is_linear: bool,
) -> Option<bool> {
    let context = session.engine.context();
    if is_linear && context.composition.caret < context.composition.input.len() {
        return None;
    }
    let next_index = context.highlighted + 1;
    if next_index >= context.candidates.len() {
        return Some(true);
    }
    session.engine.highlight_candidate(next_index);
    session.paging = true;
    Some(true)
}

fn selector_previous_page_like_librime(session: &mut SessionState) {
    let page_size = session_menu_page_size(session);
    let selected_index = session.engine.context().highlighted;
    let index = selected_index.saturating_sub(page_size);
    session.engine.highlight_candidate(index);
    session.paging = true;
}

fn selector_next_page_like_librime(session: &mut SessionState) {
    let page_size = session_menu_page_size(session);
    let context = session.engine.context();
    let index = context.highlighted + page_size;
    let page_start = (index / page_size) * page_size;
    if context.candidates.len() <= page_start {
        return;
    }
    let index = index.min(context.candidates.len() - 1);
    session.engine.highlight_candidate(index);
    session.paging = true;
}

fn selector_home_like_librime(session: &mut SessionState) -> Option<bool> {
    if session.engine.context().highlighted == 0 {
        return None;
    }
    session.engine.highlight_candidate(0);
    Some(true)
}

fn selector_end_like_librime(session: &mut SessionState) -> Option<bool> {
    let context = session.engine.context();
    if context.composition.caret < context.composition.input.len() {
        return None;
    }
    selector_home_like_librime(session)
}

pub(crate) fn install_schema_selector_bindings(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    load_selector_binding_section(
        &schema_config,
        "selector",
        &mut session.selector_bindings.horizontal_stacked,
    );
    load_selector_binding_section(
        &schema_config,
        "selector/linear",
        &mut session.selector_bindings.horizontal_linear,
    );
    load_selector_binding_section(
        &schema_config,
        "selector/vertical",
        &mut session.selector_bindings.vertical_stacked,
    );
    load_selector_binding_section(
        &schema_config,
        "selector/vertical/linear",
        &mut session.selector_bindings.vertical_linear,
    );
}

fn load_selector_binding_section(
    schema_config: &Value,
    section: &str,
    bindings: &mut HashMap<KeyEvent, SelectorBindingAction>,
) {
    let Some(Value::Mapping(config_bindings)) =
        find_config_value(schema_config, &format!("{section}/bindings"))
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
        let Some(action) = action.as_str().and_then(selector_binding_action_from_name) else {
            continue;
        };
        bindings.insert(key_event, action);
    }
}

fn selector_binding_action_from_name(action: &str) -> Option<SelectorBindingAction> {
    let action = match action {
        "noop" => SelectorBindingAction::Noop,
        "previous_candidate" => {
            SelectorBindingAction::Action(SelectorLayoutAction::PreviousCandidate)
        }
        "next_candidate" => SelectorBindingAction::Action(SelectorLayoutAction::NextCandidate),
        "previous_page" => SelectorBindingAction::Action(SelectorLayoutAction::PreviousPage),
        "next_page" => SelectorBindingAction::Action(SelectorLayoutAction::NextPage),
        "home" => SelectorBindingAction::Action(SelectorLayoutAction::Home),
        "end" => SelectorBindingAction::Action(SelectorLayoutAction::End),
        _ => return None,
    };
    Some(action)
}

pub(crate) fn process_alternative_select_key(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<Option<String>> {
    if key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.release
        || session.engine.context().candidates.is_empty()
    {
        return None;
    }
    let KeyCode::Character(ch) = key_event.code else {
        return None;
    };
    if !ch.is_ascii() || !('\u{20}'..='\u{7e}').contains(&ch) {
        return None;
    }

    let menu_settings = &session.menu_settings;
    let select_keys = menu_settings.select_keys.as_deref()?;
    let Some(index) = select_keys
        .bytes()
        .position(|select_key| select_key == ch as u8)
    else {
        return ch.is_ascii_digit().then_some(None);
    };
    if index >= menu_settings.page_size {
        return Some(None);
    }

    let page_start =
        (session.engine.context().highlighted / menu_settings.page_size) * menu_settings.page_size;
    Some(session.engine.select_candidate(page_start + index))
}
