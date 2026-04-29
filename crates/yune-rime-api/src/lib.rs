use std::{
    collections::{HashMap, HashSet},
    ffi::{c_void, CStr, CString},
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
};

use regex::Regex;
use resource_id::{
    validate_config_resource_id, validate_data_resource_id, validate_runtime_config_resource_id,
};
use serde_yaml::{Mapping, Value};
use yune_core::{
    parse_key_sequence, CandidateSource, FoldedSwitchOptions, KeyCode, KeyEvent, KeyModifiers,
    SwitchTranslatorSwitch,
};

mod abi;
mod api_table;
mod candidate_api;
mod config;
mod config_api;
mod config_compiler;
mod context_api;
mod deployment;
mod ffi_memory;
mod key_table;
mod levers;
mod modules;
mod notifications;
mod processors;
mod resource_id;
mod runtime;
mod schema_api;
mod schema_install;
mod schema_selection;
mod session;
mod userdb;
pub use abi::*;
use api_table::state_label_cache;
pub use api_table::{rime_get_api, rime_levers_get_api};
pub use candidate_api::*;
use config::*;
pub use config_api::*;
use config_compiler::*;
pub use context_api::*;
pub use deployment::*;
pub use ffi_memory::*;
pub use key_table::*;
pub use levers::*;
pub use modules::*;
use notifications::notify;
pub use notifications::RimeSetNotificationHandler;
pub(crate) use processors::*;
pub use runtime::*;
pub use schema_api::*;
pub(crate) use schema_install::{
    apply_schema_switch_resets, install_schema_filter_chain, install_schema_segment_tags,
    install_schema_translator_chain, load_schema_recognizer_patterns, recognizer_patterns_match,
    schema_component_prescription, schema_string_list, switch_reset_value,
    update_session_segment_tags,
};
pub(crate) use schema_selection::apply_schema_to_session;
pub use schema_selection::{RimeGetCurrentSchema, RimeSelectSchema};
pub use session::*;
pub use userdb::*;

const XK_BACKSPACE: c_int = 0xff08;
const XK_ESCAPE: c_int = 0xff1b;
const XK_RETURN: c_int = 0xff0d;
const XK_DELETE: c_int = 0xffff;
const XK_HOME: c_int = 0xff50;
pub(crate) const XK_LEFT: c_int = 0xff51;
pub(crate) const XK_UP: c_int = 0xff52;
pub(crate) const XK_RIGHT: c_int = 0xff53;
pub(crate) const XK_DOWN: c_int = 0xff54;
pub(crate) const XK_PAGE_UP: c_int = 0xff55;
pub(crate) const XK_PAGE_DOWN: c_int = 0xff56;
const XK_END: c_int = 0xff57;
const XK_KP_ENTER: c_int = 0xff8d;
const XK_KP_HOME: c_int = 0xff95;
pub(crate) const XK_KP_LEFT: c_int = 0xff96;
pub(crate) const XK_KP_UP: c_int = 0xff97;
pub(crate) const XK_KP_RIGHT: c_int = 0xff98;
pub(crate) const XK_KP_DOWN: c_int = 0xff99;
pub(crate) const XK_KP_PAGE_UP: c_int = 0xff9a;
pub(crate) const XK_KP_PAGE_DOWN: c_int = 0xff9b;
const XK_KP_END: c_int = 0xff9c;
const XK_KP_0: c_int = 0xffb0;
const XK_KP_9: c_int = 0xffb9;
const XK_EISU_TOGGLE: c_int = 0xff30;
pub(crate) const XK_SHIFT_L: c_int = 0xffe1;
pub(crate) const XK_SHIFT_R: c_int = 0xffe2;
pub(crate) const XK_CONTROL_L: c_int = 0xffe3;
pub(crate) const XK_CONTROL_R: c_int = 0xffe4;
pub(crate) const XK_CAPS_LOCK: c_int = 0xffe5;
pub(crate) const XK_ALT_L: c_int = 0xffe9;
pub(crate) const XK_ALT_R: c_int = 0xffea;
pub(crate) const XK_SUPER_L: c_int = 0xffeb;
pub(crate) const XK_SUPER_R: c_int = 0xffec;
const K_SHIFT_MASK: c_int = 1 << 0;
const K_LOCK_MASK: c_int = 1 << 1;
const K_CONTROL_MASK: c_int = 1 << 2;
const K_ALT_MASK: c_int = 1 << 3;
const K_SUPER_MASK: c_int = 1 << 26;
pub(crate) const K_RELEASE_MASK: c_int = 1 << 30;
const DEFAULT_PAGE_SIZE: usize = 5;
pub(crate) const RIME_VERSION_BYTES: &[u8] =
    concat!("yune-rime-api ", env!("CARGO_PKG_VERSION"), "\0").as_bytes();

pub(crate) struct SpellerProcessor {
    pub(crate) alphabet: String,
    pub(crate) delimiters: String,
    pub(crate) initials: String,
    pub(crate) finals: String,
    pub(crate) max_code_length: usize,
    pub(crate) auto_select: bool,
    pub(crate) auto_select_pattern: Option<Regex>,
    pub(crate) auto_clear: SpellerAutoClear,
    pub(crate) use_space: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum SpellerAutoClear {
    None,
    Auto,
    Manual,
    MaxLength,
}

#[derive(Clone, Copy)]
pub(crate) enum EditorBindingAction {
    Noop,
    Action(EditorAction),
}

#[derive(Clone, Copy)]
pub(crate) enum EditorAction {
    Confirm,
    ToggleSelection,
    CommitComment,
    CommitRawInput,
    CommitScriptText,
    CommitComposition,
    Revert,
    Back,
    BackSyllable,
    DeleteCandidate,
    Delete,
    Cancel,
}

#[derive(Default)]
pub(crate) struct SelectorBindings {
    pub(crate) horizontal_stacked: HashMap<KeyEvent, SelectorBindingAction>,
    pub(crate) horizontal_linear: HashMap<KeyEvent, SelectorBindingAction>,
    pub(crate) vertical_stacked: HashMap<KeyEvent, SelectorBindingAction>,
    pub(crate) vertical_linear: HashMap<KeyEvent, SelectorBindingAction>,
}

#[derive(Clone, Copy)]
pub(crate) enum SelectorBindingAction {
    Noop,
    Action(SelectorLayoutAction),
}

#[derive(Default)]
pub(crate) struct NavigatorBindings {
    pub(crate) horizontal: HashMap<KeyEvent, NavigatorBindingAction>,
    pub(crate) vertical: HashMap<KeyEvent, NavigatorBindingAction>,
}

#[derive(Clone, Copy)]
pub(crate) enum NavigatorBindingAction {
    Noop,
    Action(NavigatorAction),
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum NavigatorSyllableJumpPosition {
    AfterDelimiter,
    BeforeDelimiter,
}

#[derive(Clone)]
enum SwitchSelectionCommand {
    Toggle(String),
    Radio {
        options: Vec<String>,
        option_index: usize,
    },
}

pub(crate) struct MatcherSegmentor {
    pub(crate) patterns: Vec<MatcherPattern>,
}

pub(crate) struct AffixSegmentor {
    pub(crate) tag: String,
    pub(crate) prefix: String,
    pub(crate) suffix: String,
    pub(crate) extra_tags: Vec<String>,
}

pub(crate) struct PunctSegmentor {
    pub(crate) half_shape_keys: HashSet<String>,
    pub(crate) full_shape_keys: HashSet<String>,
    pub(crate) digit_separators: String,
}

pub(crate) struct RecognizerProcessor {
    pub(crate) use_space: bool,
    pub(crate) patterns: Vec<MatcherPattern>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AsciiComposerProcessResult {
    Noop,
    Accepted(Option<String>),
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AsciiModeSwitchStyle {
    InlineAscii,
    CommitText,
    CommitCode,
    Clear,
    SetAsciiMode,
    UnsetAsciiMode,
}

pub(crate) struct MatcherPattern {
    pub(crate) tag: String,
    pub(crate) pattern: Regex,
}

pub(crate) struct PunctuationProcessor {
    pub(crate) use_space: bool,
    pub(crate) digit_separators: String,
    pub(crate) digit_separator_commit: bool,
    pub(crate) half_shape_alternating_counts: HashMap<String, usize>,
    pub(crate) full_shape_alternating_counts: HashMap<String, usize>,
    pub(crate) symbol_alternating_counts: HashMap<String, usize>,
    pub(crate) half_shape_unique_commits: HashMap<String, String>,
    pub(crate) full_shape_unique_commits: HashMap<String, String>,
    pub(crate) symbol_unique_commits: HashMap<String, String>,
    pub(crate) half_shape_pairs: HashMap<String, [String; 2]>,
    pub(crate) full_shape_pairs: HashMap<String, [String; 2]>,
    pub(crate) symbol_pairs: HashMap<String, [String; 2]>,
    pub(crate) pair_oddness: HashMap<String, usize>,
    pub(crate) pending_digit_separator: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EditorProcessor {
    Express,
    Fluid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EditorCharHandler {
    DirectCommit,
    AddToInput,
}

pub(crate) enum PunctuationProcessResult {
    Accepted,
    Commit(String),
}

pub(crate) struct SpellerProcessResult {
    pub(crate) accepted: bool,
    pub(crate) commit: Option<String>,
}

pub(crate) enum SessionKeyProcessResult {
    Noop,
    Accepted,
    Commit(String),
    RejectedCommit(String),
}

pub(crate) struct UserDictListState {
    names: Vec<CString>,
}

struct StateLabel {
    value: String,
    length: usize,
}

pub(crate) struct ContextMenuSettings {
    pub(crate) page_size: usize,
    pub(crate) select_keys: Option<String>,
    pub(crate) select_labels: Vec<String>,
}

#[derive(Clone, Copy)]
pub(crate) enum ConfigOpenKind {
    Deployed,
    User,
}

#[must_use]
pub const fn bool_from(value: bool) -> Bool {
    if value {
        TRUE
    } else {
        FALSE
    }
}

#[no_mangle]
pub extern "C" fn RimeGetVersion() -> *const c_char {
    RIME_VERSION_BYTES.as_ptr().cast::<c_char>()
}

#[no_mangle]
pub extern "C" fn RimeProcessKey(session_id: RimeSessionId, keycode: c_int, mask: c_int) -> Bool {
    if session_id == 0
        || (mask != 0
            && !((mask == K_CONTROL_MASK
                && (matches!(
                    keycode,
                    XK_BACKSPACE | XK_DELETE | XK_LEFT | XK_RIGHT | XK_UP | XK_DOWN | XK_RETURN
                ) || (('0' as c_int)..=('9' as c_int)).contains(&keycode)
                    || (0x20..=0x7e).contains(&keycode)
                    || (XK_KP_0..=XK_KP_9).contains(&keycode)))
                || (mask == K_SHIFT_MASK && keycode == XK_RETURN)
                || (mask == K_SHIFT_MASK && keycode == XK_BACKSPACE)
                || (mask == K_SHIFT_MASK && keycode == XK_DELETE)
                || (mask == K_SHIFT_MASK && keycode == XK_ESCAPE)
                || (mask == K_SHIFT_MASK
                    && matches!(keycode, XK_LEFT | XK_RIGHT | XK_UP | XK_DOWN))
                || (mask == K_SHIFT_MASK
                    && matches!(keycode, XK_KP_LEFT | XK_KP_RIGHT | XK_KP_UP | XK_KP_DOWN))
                || (mask == K_SHIFT_MASK && (XK_KP_0..=XK_KP_9).contains(&keycode))
                || (mask == K_SHIFT_MASK
                    && matches!(keycode, XK_HOME | XK_END | XK_KP_HOME | XK_KP_END))
                || (mask == K_SHIFT_MASK && (0x20..=0x7e).contains(&keycode))
                || (mask == K_LOCK_MASK && (0x20..=0x7e).contains(&keycode))
                || (mask == K_ALT_MASK && (0x20..=0x7e).contains(&keycode))
                || (mask == K_SUPER_MASK && (0x20..=0x7e).contains(&keycode))
                || (mask == (K_CONTROL_MASK | K_SHIFT_MASK)
                    && (keycode == XK_RETURN
                        || (('0' as c_int)..=('9' as c_int)).contains(&keycode)
                        || (XK_KP_0..=XK_KP_9).contains(&keycode)
                        || (0x20..=0x7e).contains(&keycode)))
                || ((mask & K_RELEASE_MASK) != 0
                    && (mask
                        & !(K_RELEASE_MASK
                            | K_CONTROL_MASK
                            | K_SHIFT_MASK
                            | K_LOCK_MASK
                            | K_ALT_MASK
                            | K_SUPER_MASK))
                        == 0
                    && (0x20..=0x7e).contains(&keycode))
                || (mask == K_RELEASE_MASK && (0x20..=0x7e).contains(&keycode))
                || (mask == K_RELEASE_MASK && is_ascii_composer_modifier_key(keycode))))
    {
        return FALSE;
    }
    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };

    if is_ascii_composer_modifier_key(keycode) && (mask == 0 || mask == K_RELEASE_MASK) {
        if let Some(commit) = process_ascii_composer_modifier_switch_key(session, keycode, mask) {
            append_unread_commit(session, commit);
        }
        update_session_segment_tags(session);
        return FALSE;
    }
    if session.ascii_composer_pressed_switch_key.is_some() {
        session.ascii_composer_pressed_switch_key = None;
    }

    if keycode == XK_EISU_TOGGLE && mask == 0 {
        if let Some(commit) = process_ascii_composer_switch_key(session, keycode) {
            if let Some(commit) = commit {
                append_unread_commit(session, commit);
            }
            update_session_segment_tags(session);
            return TRUE;
        }
        return FALSE;
    }
    if keycode == XK_CAPS_LOCK && mask == 0 {
        if let Some(commit) = process_ascii_composer_caps_lock_switch_key(session) {
            if let Some(commit) = commit {
                append_unread_commit(session, commit);
            }
            update_session_segment_tags(session);
            return TRUE;
        }
        return FALSE;
    }

    let Some(key_event) = key_event_from_rime_keycode(keycode, mask) else {
        return FALSE;
    };
    if (mask == K_CONTROL_MASK || mask == K_LOCK_MASK || mask == K_ALT_MASK || mask == K_SUPER_MASK)
        && (0x20..=0x7e).contains(&keycode)
        && !(('0' as c_int)..=('9' as c_int)).contains(&keycode)
        && !session_has_modified_printable_binding(session, key_event)
        && !session_chord_composer_accepts_printable(session, key_event)
    {
        return FALSE;
    }

    match process_ascii_composer_processor(session, key_event) {
        AsciiComposerProcessResult::Noop => {}
        AsciiComposerProcessResult::Accepted(commit) => {
            if let Some(commit) = commit {
                append_unread_commit(session, commit);
            }
            update_session_segment_tags(session);
            return TRUE;
        }
        AsciiComposerProcessResult::Rejected => return FALSE,
    }

    if let Some(commits) = process_key_binder_processor(session_id, session, key_event) {
        for commit in commits {
            append_unread_commit(session, commit);
        }
        return TRUE;
    }

    let was_composing = !session.engine.context().composition.input.is_empty();
    if !was_composing
        && (mask == K_CONTROL_MASK || mask == (K_CONTROL_MASK | K_SHIFT_MASK))
        && matches!(
            key_event.code,
            KeyCode::Character('0'..='9') | KeyCode::KeypadDigit('0'..='9')
        )
    {
        return FALSE;
    }
    let mut accepted = false;
    if let Some(selector_accepted) = process_selector_layout_key(session, key_event, keycode, mask)
    {
        accepted = selector_accepted;
    } else if let Some(navigator_accepted) = process_navigator_configured_key(session, key_event) {
        accepted = navigator_accepted;
    } else if let Some(navigator_accepted) = process_navigator_delimiter_key(session, key_event) {
        accepted = navigator_accepted;
    } else {
        match key_event.code {
            KeyCode::PreviousPage => {
                let page_size = session_menu_page_size(session);
                if session.engine.change_page_by(page_size, true) {
                    session.paging = true;
                    accepted = true;
                }
            }
            KeyCode::NextPage => {
                let page_size = session_menu_page_size(session);
                if session.engine.change_page_by(page_size, false) {
                    session.paging = true;
                    accepted = true;
                }
            }
            _ => match process_session_key_event(session_id, session, key_event) {
                SessionKeyProcessResult::Noop => {
                    if let Some(commit) = process_shape_processor(session, key_event) {
                        append_unread_commit(session, commit);
                        return TRUE;
                    }
                    return FALSE;
                }
                SessionKeyProcessResult::Accepted => accepted = true,
                SessionKeyProcessResult::Commit(commit) => {
                    append_unread_commit(session, commit);
                    return TRUE;
                }
                SessionKeyProcessResult::RejectedCommit(commit) => {
                    append_unread_commit(session, commit);
                    return FALSE;
                }
            },
        }
    }

    apply_visible_switch_radio_defaults(session);
    bool_from(
        accepted || matches!(key_event.code, KeyCode::Character(ch) if ch != ' ') || was_composing,
    )
}

#[no_mangle]
pub extern "C" fn RimeCommitComposition(session_id: RimeSessionId) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    let Some(commit) = session.engine.commit_composition() else {
        return FALSE;
    };

    session.paging = false;
    update_session_segment_tags(session);
    sync_chord_composer_context_update(session);
    append_unread_commit(session, commit);
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeClearComposition(session_id: RimeSessionId) {
    if session_id == 0 {
        return;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    if let Some(session) = registry.get_session_mut(session_id) {
        session.engine.clear_composition();
        session.paging = false;
        update_session_segment_tags(session);
        sync_chord_composer_context_update(session);
    }
}

#[no_mangle]
pub extern "C" fn RimeGetInput(session_id: RimeSessionId) -> *const c_char {
    if session_id == 0 {
        return ptr::null();
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return ptr::null();
    };
    let Ok(input) = CString::new(session.engine.context().composition.input.as_str()) else {
        return ptr::null();
    };
    session.input_buffer = Some(input);
    session
        .input_buffer
        .as_ref()
        .map_or(ptr::null(), |input| input.as_ptr())
}

/// Sets the current raw composition input for a session.
///
/// # Safety
///
/// `input` must be either null or a valid NUL-terminated C string. Null input
/// is rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeSetInput(session_id: RimeSessionId, input: *const c_char) -> Bool {
    if session_id == 0 || input.is_null() {
        return FALSE;
    }

    // SAFETY: `input` is non-null and caller promises a valid NUL-terminated C
    // string.
    let Ok(input) = unsafe { CStr::from_ptr(input) }.to_str() else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    session.engine.set_input(input);
    session.input_buffer = None;
    update_session_segment_tags(session);
    sync_chord_composer_context_update(session);
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeGetCaretPos(session_id: RimeSessionId) -> usize {
    if session_id == 0 {
        return 0;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    registry
        .get_session_mut(session_id)
        .map_or(0, |session| session.engine.context().composition.caret)
}

#[no_mangle]
pub extern "C" fn RimeSetCaretPos(session_id: RimeSessionId, caret_pos: usize) {
    if session_id == 0 {
        return;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    if let Some(session) = registry.get_session_mut(session_id) {
        session.engine.set_caret_pos(caret_pos);
    }
}

/// Sets a session-scoped runtime option.
///
/// # Safety
///
/// `option` must be either null or point to a valid, nul-terminated C string.
/// Null option names are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeSetOption(
    session_id: RimeSessionId,
    option: *const c_char,
    value: Bool,
) {
    if option.is_null() {
        return;
    }
    // SAFETY: callers promise that `option` is a valid nul-terminated string.
    let option = unsafe { CStr::from_ptr(option) }
        .to_string_lossy()
        .into_owned();
    if with_session(session_id, |session| {
        session.engine.set_option(option.clone(), value != FALSE);
        update_session_segment_tags(session);
        true
    }) == TRUE
    {
        let message_value = if value != FALSE {
            option
        } else {
            format!("!{option}")
        };
        notify(session_id, "option", &message_value);
    }
}

/// Returns the current value of a session-scoped runtime option.
///
/// # Safety
///
/// `option` must be either null or point to a valid, nul-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeGetOption(session_id: RimeSessionId, option: *const c_char) -> Bool {
    if option.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `option` is a valid nul-terminated string.
    let option = unsafe { CStr::from_ptr(option) };
    with_session(session_id, |session| {
        session.engine.get_option(&option.to_string_lossy())
    })
}

/// Sets a session-scoped string property.
///
/// # Safety
///
/// `property` and `value` must be either null or point to valid,
/// nul-terminated C strings. Null inputs are ignored.
#[no_mangle]
pub unsafe extern "C" fn RimeSetProperty(
    session_id: RimeSessionId,
    property: *const c_char,
    value: *const c_char,
) {
    if property.is_null() || value.is_null() {
        return;
    }
    // SAFETY: callers promise that both pointers are valid nul-terminated
    // strings.
    let property = unsafe { CStr::from_ptr(property) }
        .to_string_lossy()
        .into_owned();
    let value = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    if with_session(session_id, |session| {
        session.engine.set_property(property.clone(), value.clone());
        true
    }) == TRUE
    {
        notify(session_id, "property", &format!("{property}={value}"));
    }
}

/// Copies a session-scoped string property into caller-provided storage.
///
/// # Safety
///
/// `property` must point to a valid nul-terminated C string, and `value` must
/// point to writable storage of `buffer_size` bytes. Null pointers and empty
/// property values are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeGetProperty(
    session_id: RimeSessionId,
    property: *const c_char,
    value: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if property.is_null() || value.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `property` is a valid nul-terminated string.
    let property = unsafe { CStr::from_ptr(property) };

    with_session(session_id, |session| {
        let Some(property_value) = session.engine.get_property(&property.to_string_lossy()) else {
            return false;
        };
        if property_value.is_empty() {
            return false;
        }
        copy_c_string_with_strncpy_semantics(property_value, value, buffer_size);
        true
    })
}

/// Returns a switch state label from the selected schema config.
///
/// # Safety
///
/// `option_name` must be either null or a valid NUL-terminated C string. The
/// returned pointer is process-owned and remains valid until the next
/// state-label query.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStateLabel(
    session_id: RimeSessionId,
    option_name: *const c_char,
    state: Bool,
) -> *const c_char {
    // SAFETY: forwarded preconditions match `RimeGetStateLabelAbbreviated`.
    unsafe { RimeGetStateLabelAbbreviated(session_id, option_name, state, FALSE).str }
}

/// Returns a switch state label slice from the selected schema config.
///
/// # Safety
///
/// `option_name` must be either null or a valid NUL-terminated C string. The
/// returned pointer is process-owned and remains valid until the next
/// state-label query.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStateLabelAbbreviated(
    session_id: RimeSessionId,
    option_name: *const c_char,
    state: Bool,
    abbreviated: Bool,
) -> RimeStringSlice {
    let Some(option_name) = (unsafe { c_string_key(option_name) }) else {
        return empty_string_slice();
    };
    let Some(label) =
        state_label_for_session(session_id, &option_name, state, abbreviated != FALSE)
    else {
        return empty_string_slice();
    };
    let Ok(cached_label) = CString::new(label.value) else {
        return empty_string_slice();
    };

    let mut cache = state_label_cache()
        .lock()
        .expect("state label cache should not be poisoned");
    *cache = Some(cached_label);
    let Some(cached_label) = cache.as_ref() else {
        return empty_string_slice();
    };
    RimeStringSlice {
        str: cached_label.as_ptr(),
        length: label.length,
    }
}

/// Processes a librime-style key sequence against a session.
///
/// # Safety
///
/// `key_sequence` must be either null or point to a valid, nul-terminated C
/// string. Null or unparsable sequences are rejected without mutating the
/// session.
#[no_mangle]
pub unsafe extern "C" fn RimeSimulateKeySequence(
    session_id: RimeSessionId,
    key_sequence: *const c_char,
) -> Bool {
    if session_id == 0 || key_sequence.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `key_sequence` is a valid nul-terminated
    // string.
    let Ok(key_sequence) = unsafe { CStr::from_ptr(key_sequence) }.to_str() else {
        return FALSE;
    };
    let Ok(key_events) = parse_key_sequence(key_sequence) else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };

    for key_event in key_events {
        match process_session_key_event(session_id, session, key_event) {
            SessionKeyProcessResult::Commit(commit)
            | SessionKeyProcessResult::RejectedCommit(commit) => {
                append_unread_commit(session, commit)
            }
            SessionKeyProcessResult::Noop | SessionKeyProcessResult::Accepted => {}
        }
    }
    TRUE
}

#[no_mangle]
pub extern "C" fn RimeSelectCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    select_candidate_or_switch(session_id, |_session| Some(index))
}

#[no_mangle]
pub extern "C" fn RimeSelectCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    select_candidate_or_switch(session_id, |session| {
        candidate_index_on_current_page(session, index)
    })
}

#[no_mangle]
pub extern "C" fn RimeDeleteCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    with_session(session_id, |session| session.engine.delete_candidate(index))
}

#[no_mangle]
pub extern "C" fn RimeDeleteCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    with_session(session_id, |session| {
        let Some(global_index) = candidate_index_on_current_page(session, index) else {
            return false;
        };
        session.engine.delete_candidate(global_index)
    })
}

#[no_mangle]
pub extern "C" fn RimeHighlightCandidate(session_id: RimeSessionId, index: usize) -> Bool {
    with_session(session_id, |session| {
        highlight_candidate_clamped_like_librime(session, index)
    })
}

#[no_mangle]
pub extern "C" fn RimeHighlightCandidateOnCurrentPage(
    session_id: RimeSessionId,
    index: usize,
) -> Bool {
    with_session(session_id, |session| {
        let Some(global_index) = candidate_index_on_current_page(session, index) else {
            return false;
        };
        highlight_candidate_clamped_like_librime(session, global_index)
    })
}

#[no_mangle]
pub extern "C" fn RimeChangePage(session_id: RimeSessionId, backward: Bool) -> Bool {
    with_session(session_id, |session| {
        if session.engine.context().candidates.is_empty() {
            return false;
        }

        let page_size = session_menu_page_size(session);
        let current_index = session.engine.context().highlighted;
        let next_index = if backward != FALSE {
            current_index.saturating_sub(page_size)
        } else {
            current_index + page_size
        };
        let changed = highlight_candidate_clamped_like_librime(session, next_index);
        if changed {
            session.paging = true;
        }
        changed
    })
}

fn key_event_from_rime_keycode(keycode: c_int, mask: c_int) -> Option<KeyEvent> {
    let code = match keycode {
        XK_BACKSPACE => KeyCode::Backspace,
        XK_DELETE => KeyCode::Delete,
        XK_ESCAPE => KeyCode::Escape,
        XK_LEFT => KeyCode::MoveCaretLeft,
        XK_RIGHT => KeyCode::MoveCaretRight,
        XK_UP if mask == K_CONTROL_MASK || mask == K_SHIFT_MASK => KeyCode::MoveCaretLeftBySyllable,
        XK_DOWN if mask == K_CONTROL_MASK || mask == K_SHIFT_MASK => {
            KeyCode::MoveCaretRightBySyllable
        }
        XK_KP_LEFT => KeyCode::MoveCaretLeftByChar,
        XK_KP_RIGHT => KeyCode::MoveCaretRightByChar,
        XK_UP | XK_KP_UP => KeyCode::PreviousCandidate,
        XK_DOWN | XK_KP_DOWN => KeyCode::NextCandidate,
        XK_HOME | XK_KP_HOME => KeyCode::Home,
        XK_END | XK_KP_END => KeyCode::End,
        XK_PAGE_UP | XK_KP_PAGE_UP => KeyCode::PreviousPage,
        XK_PAGE_DOWN | XK_KP_PAGE_DOWN => KeyCode::NextPage,
        XK_RETURN => KeyCode::Return,
        XK_KP_ENTER => KeyCode::KeypadEnter,
        XK_KP_0..=XK_KP_9 => {
            KeyCode::KeypadDigit(char::from_u32(('0' as u32) + (keycode - XK_KP_0) as u32)?)
        }
        0x20..=0x7e => KeyCode::Character(char::from_u32(keycode as u32)?),
        _ => return None,
    };
    let modifiers = key_modifiers_from_rime_mask(mask)?;

    Some(KeyEvent { code, modifiers })
}

fn key_modifiers_from_rime_mask(mask: c_int) -> Option<KeyModifiers> {
    let supported_mask =
        K_SHIFT_MASK | K_LOCK_MASK | K_CONTROL_MASK | K_ALT_MASK | K_SUPER_MASK | K_RELEASE_MASK;
    if mask & !supported_mask != 0 {
        return None;
    }

    Some(KeyModifiers {
        shift: mask & K_SHIFT_MASK != 0,
        lock: mask & K_LOCK_MASK != 0,
        control: mask & K_CONTROL_MASK != 0,
        alt: mask & K_ALT_MASK != 0,
        super_key: mask & K_SUPER_MASK != 0,
        release: mask & K_RELEASE_MASK != 0,
        ..KeyModifiers::default()
    })
}

fn select_candidate_or_switch(
    session_id: RimeSessionId,
    index: impl FnOnce(&SessionState) -> Option<usize>,
) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    let Some(index) = index(session) else {
        return FALSE;
    };
    if apply_schema_list_candidate(session, index) || apply_switch_candidate(session, index) {
        session.paging = false;
        update_session_segment_tags(session);
        return TRUE;
    }

    let Some(commit) = session.engine.select_candidate(index) else {
        return FALSE;
    };
    session.paging = false;
    update_session_segment_tags(session);
    append_unread_commit(session, commit);
    TRUE
}

fn apply_schema_list_candidate(session: &mut SessionState, candidate_index: usize) -> bool {
    let Some(candidate) = session.engine.context().candidates.get(candidate_index) else {
        return false;
    };
    if candidate.source != CandidateSource::Schema {
        return false;
    }
    let schema_index = session.engine.context().candidates[..=candidate_index]
        .iter()
        .filter(|candidate| candidate.source == CandidateSource::Schema)
        .count()
        - 1;
    let Some(schema_id) = schema_list_selection_commands(session)
        .get(schema_index)
        .cloned()
    else {
        return false;
    };
    apply_schema_to_session(session, &schema_id);
    true
}

fn schema_list_selection_commands(session: &SessionState) -> Vec<String> {
    let current_schema = session.engine.status().schema_id;
    let mut schema_ids = vec![current_schema.clone()];
    let schema_config = load_runtime_config_root(
        &format!("{current_schema}.schema"),
        ConfigOpenKind::Deployed,
    );
    schema_ids.extend(
        schema_list_translator_entries_for_current(&current_schema, &schema_config)
            .into_iter()
            .map(|(schema_id, _)| schema_id),
    );
    schema_ids
}

fn apply_switch_candidate(session: &mut SessionState, candidate_index: usize) -> bool {
    let Some(candidate) = session.engine.context().candidates.get(candidate_index) else {
        return false;
    };
    if candidate.source == CandidateSource::Unfold {
        session.engine.set_option("_fold_options", false);
        return true;
    }
    if candidate.source != CandidateSource::Switch {
        return false;
    }
    let switch_index = session.engine.context().candidates[..=candidate_index]
        .iter()
        .filter(|candidate| candidate.source == CandidateSource::Switch)
        .count()
        - 1;
    let schema_id = &session.engine.status().schema_id;
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(selection) = schema_switch_selection_commands(&schema_config)
        .get(switch_index)
        .cloned()
    else {
        return false;
    };

    match selection {
        SwitchSelectionCommand::Toggle(option_name) => {
            let target_state = !session.engine.get_option(&option_name);
            session.engine.set_option(option_name.clone(), target_state);
            persist_switcher_saved_option(&schema_config, &option_name, target_state);
        }
        SwitchSelectionCommand::Radio {
            options,
            option_index,
        } => {
            select_key_binding_radio_option(session, &options, option_index);
            for (index, option) in options.iter().enumerate() {
                persist_switcher_saved_option(&schema_config, option, index == option_index);
            }
        }
    }
    session.engine.clear_composition();
    true
}

fn persist_switcher_saved_option(schema_config: &Value, option_name: &str, value: bool) {
    if !schema_string_list(schema_config, "switcher/save_options")
        .iter()
        .any(|saved_option| saved_option == option_name)
    {
        return;
    }
    let Some(user_config_path) = selected_runtime_config_path("user", ConfigOpenKind::User) else {
        return;
    };
    if let Some(user_config_dir) = user_config_path.parent() {
        if fs::create_dir_all(user_config_dir).is_err() {
            return;
        }
    }
    let mut user_config = fs::read_to_string(&user_config_path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| Value::Mapping(Mapping::new()));
    if !set_config_value(
        &mut user_config,
        &format!("var/option/{option_name}"),
        Value::Bool(value),
    ) {
        return;
    }
    let Ok(yaml) = serde_yaml::to_string(&user_config) else {
        return;
    };
    let _ = fs::write(user_config_path, yaml);
}

pub(crate) fn apply_visible_switch_radio_defaults(session: &mut SessionState) {
    if !session.engine.context().candidates.iter().any(|candidate| {
        matches!(
            candidate.source,
            CandidateSource::Switch | CandidateSource::Unfold
        )
    }) {
        return;
    }

    let schema_id = &session.engine.status().schema_id;
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    for options in schema_switch_radio_option_groups(&schema_config) {
        let selected_index = options
            .iter()
            .position(|option| session.engine.get_option(option))
            .unwrap_or(0);
        for (option_index, option) in options.iter().enumerate() {
            let selected = option_index == selected_index;
            if session.engine.get_option(option) != selected {
                session.engine.set_option(option.clone(), selected);
            }
        }
    }
}

fn append_unread_commit(session: &mut SessionState, commit: String) {
    let commit = shape_formatted_commit_text(session, &commit);
    match &mut session.unread_commit {
        Some(buffer) => buffer.push_str(&commit),
        None => session.unread_commit = Some(commit),
    }
}

fn shape_formatted_commit_text(session: &SessionState, text: &str) -> String {
    shape_formatted_ascii_text(text, session.engine.status().is_full_shape)
}

pub(crate) fn session_menu_page_size(session: &SessionState) -> usize {
    context_menu_settings(&session.engine.status().schema_id).page_size
}

#[derive(Clone, Copy)]
pub(crate) enum SelectorLayoutAction {
    PreviousCandidate,
    NextCandidate,
    PreviousPage,
    NextPage,
    Home,
    End,
}

#[derive(Clone, Copy)]
pub(crate) enum NavigatorAction {
    Rewind,
    Forward,
    LeftByChar,
    RightByChar,
    LeftBySyllable,
    RightBySyllable,
    LeftByCharNoLoop,
    RightByCharNoLoop,
    LeftBySyllableNoLoop,
    RightBySyllableNoLoop,
    Home,
    End,
}

fn session_has_modified_printable_binding(session: &SessionState, key_event: KeyEvent) -> bool {
    if session
        .key_binder
        .as_ref()
        .is_some_and(|processor| processor.has_binding(&key_event))
    {
        return true;
    }

    let is_vertical = session.engine.get_option("_vertical");
    let is_linear =
        session.engine.get_option("_linear") || session.engine.get_option("_horizontal");
    selector_configured_action(session, is_vertical, is_linear, key_event).is_some()
        || navigator_configured_action(session, is_vertical, key_event).is_some()
        || session.editor_bindings.contains_key(&key_event)
        || session
            .chord_composer
            .as_ref()
            .is_some_and(|processor| processor.has_binding(&key_event))
}

pub(crate) fn parse_single_key_binding_event(pattern: &str) -> Option<KeyEvent> {
    let sequence = if pattern.chars().count() == 1 {
        pattern.to_owned()
    } else {
        format!("{{{pattern}}}")
    };
    let mut events = parse_key_sequence(&sequence).ok()?;
    (events.len() == 1).then(|| events.remove(0))
}

fn schema_engine_translators_include(schema_config: &Value, translator_name: &str) -> bool {
    !schema_engine_translator_namespaces(schema_config, translator_name, translator_name).is_empty()
}

fn schema_engine_translator_namespaces(
    schema_config: &Value,
    translator_name: &str,
    default_name_space: &str,
) -> Vec<String> {
    let Some(Value::Sequence(translators)) = find_config_value(schema_config, "engine/translators")
    else {
        return Vec::new();
    };
    schema_engine_component_namespaces(translators, translator_name, default_name_space)
}

fn schema_engine_component_namespaces(
    components: &[Value],
    component_name: &str,
    default_name_space: &str,
) -> Vec<String> {
    let component_prefix = format!("{component_name}@");
    components
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|component| {
            if component == component_name {
                Some(default_name_space.to_owned())
            } else {
                component
                    .strip_prefix(&component_prefix)
                    .filter(|name_space| !name_space.is_empty())
                    .map(ToOwned::to_owned)
            }
        })
        .collect()
}

fn schema_engine_processors_include(schema_config: &Value, processor_name: &str) -> bool {
    let Some(Value::Sequence(processors)) = find_config_value(schema_config, "engine/processors")
    else {
        return false;
    };
    processors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .any(|(component_name, _)| component_name == processor_name)
}

pub(crate) fn switch_scalar_field(switch_map: &Mapping, key: &str) -> Option<String> {
    switch_map
        .get(Value::String(key.to_owned()))
        .and_then(config_scalar_string)
}

pub(crate) fn schema_switch_translator_switches(
    schema_config: &Value,
) -> Vec<SwitchTranslatorSwitch> {
    let Some(Value::Sequence(switches)) = find_config_value(schema_config, "switches") else {
        return Vec::new();
    };

    switches
        .iter()
        .filter_map(|the_switch| {
            let Value::Mapping(switch_map) = the_switch else {
                return None;
            };
            if let Some(option_name) = switch_scalar_field(switch_map, "name") {
                let state0 = switch_label_value(switch_map, "states", 0)?;
                let state1 = switch_label_value(switch_map, "states", 1).unwrap_or_default();
                return Some(
                    SwitchTranslatorSwitch::toggle(option_name, state0, state1)
                        .with_abbrev(switch_abbrev_values(switch_map)),
                );
            }

            let Value::Sequence(options) = switch_map.get(Value::String("options".to_owned()))?
            else {
                return None;
            };
            let options = options
                .iter()
                .filter_map(config_scalar_string)
                .collect::<Vec<_>>();
            let states = schema_switch_label_values(switch_map, "states");
            if options.is_empty() || states.is_empty() {
                None
            } else {
                Some(
                    SwitchTranslatorSwitch::radio(options, states)
                        .with_abbrev(switch_abbrev_values(switch_map)),
                )
            }
        })
        .collect()
}

pub(crate) fn schema_folded_switch_options(schema_config: &Value) -> FoldedSwitchOptions {
    FoldedSwitchOptions {
        prefix: find_config_value(schema_config, "switcher/option_list_prefix")
            .and_then(config_scalar_string)
            .unwrap_or_default(),
        suffix: find_config_value(schema_config, "switcher/option_list_suffix")
            .and_then(config_scalar_string)
            .unwrap_or_default(),
        separator: find_config_value(schema_config, "switcher/option_list_separator")
            .and_then(config_scalar_string)
            .unwrap_or_else(|| " ".to_owned()),
        abbreviate_options: find_config_value(schema_config, "switcher/abbreviate_options")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
    }
}

fn schema_switch_selection_commands(schema_config: &Value) -> Vec<SwitchSelectionCommand> {
    let Some(Value::Sequence(switches)) = find_config_value(schema_config, "switches") else {
        return Vec::new();
    };

    let mut commands = Vec::new();
    for the_switch in switches {
        let Value::Mapping(switch_map) = the_switch else {
            continue;
        };
        if let Some(option_name) = switch_scalar_field(switch_map, "name") {
            if switch_label_value(switch_map, "states", 0).is_some() {
                commands.push(SwitchSelectionCommand::Toggle(option_name));
            }
            continue;
        }

        let Some(Value::Sequence(options)) = switch_map.get(Value::String("options".to_owned()))
        else {
            continue;
        };
        let options = options
            .iter()
            .filter_map(config_scalar_string)
            .collect::<Vec<_>>();
        if options.is_empty() || switch_label_value(switch_map, "states", 0).is_none() {
            continue;
        }
        for option_index in 0..options.len() {
            if switch_label_value(switch_map, "states", option_index).is_some() {
                commands.push(SwitchSelectionCommand::Radio {
                    options: options.clone(),
                    option_index,
                });
            }
        }
    }
    commands
}

fn schema_switch_radio_option_groups(schema_config: &Value) -> Vec<Vec<String>> {
    let Some(Value::Sequence(switches)) = find_config_value(schema_config, "switches") else {
        return Vec::new();
    };

    switches
        .iter()
        .filter_map(|the_switch| {
            let Value::Mapping(switch_map) = the_switch else {
                return None;
            };
            if switch_scalar_field(switch_map, "name").is_some()
                || switch_label_value(switch_map, "states", 0).is_none()
            {
                return None;
            }
            let Value::Sequence(options) = switch_map.get(Value::String("options".to_owned()))?
            else {
                return None;
            };
            let options = options
                .iter()
                .filter_map(config_scalar_string)
                .collect::<Vec<_>>();
            (!options.is_empty()).then_some(options)
        })
        .collect()
}

fn schema_switch_label_values(switch_map: &Mapping, key: &str) -> Vec<String> {
    let Some(Value::Sequence(values)) = switch_map.get(Value::String(key.to_owned())) else {
        return Vec::new();
    };
    values
        .iter()
        .map(|value| config_scalar_string(value).unwrap_or_default())
        .collect()
}

fn switch_label_value(switch_map: &Mapping, key: &str, state_index: usize) -> Option<String> {
    let Value::Sequence(values) = switch_map.get(Value::String(key.to_owned()))? else {
        return None;
    };
    values.get(state_index).and_then(config_scalar_string)
}

fn switch_abbrev_values(switch_map: &Mapping) -> Vec<Option<String>> {
    let Some(Value::Sequence(values)) = switch_map.get(Value::String("abbrev".to_owned())) else {
        return Vec::new();
    };
    values.iter().map(config_scalar_string).collect()
}

pub(crate) fn process_session_key_event(
    session_id: RimeSessionId,
    session: &mut SessionState,
    key_event: KeyEvent,
) -> SessionKeyProcessResult {
    if let Some(result) = process_chord_composer_processor(session, key_event) {
        update_session_segment_tags(session);
        sync_chord_composer_context_update(session);
        return result;
    }
    if let Some(commits) = process_key_binder_processor(session_id, session, key_event) {
        update_session_segment_tags(session);
        return if commits.is_empty() {
            SessionKeyProcessResult::Accepted
        } else {
            SessionKeyProcessResult::Commit(commits.concat())
        };
    }
    if key_event.modifiers.release {
        return SessionKeyProcessResult::Noop;
    }
    if process_recognizer_processor(session, key_event) {
        update_session_segment_tags(session);
        return SessionKeyProcessResult::Accepted;
    }
    if let Some(result) = process_punctuation_processor(session, key_event) {
        let commit = match result {
            PunctuationProcessResult::Accepted => None,
            PunctuationProcessResult::Commit(commit) => Some(session.engine.record_commit(commit)),
        };
        update_session_segment_tags(session);
        return commit.map_or(
            SessionKeyProcessResult::Accepted,
            SessionKeyProcessResult::Commit,
        );
    }
    if let Some(commit) = process_alternative_select_key(session, key_event) {
        update_session_segment_tags(session);
        return commit.map_or(
            SessionKeyProcessResult::Accepted,
            SessionKeyProcessResult::Commit,
        );
    }
    if let Some(result) = process_speller_processor(session, key_event) {
        update_session_segment_tags(session);
        if let Some(commit) = result.commit {
            return SessionKeyProcessResult::Commit(commit);
        }
        return if result.accepted {
            SessionKeyProcessResult::Accepted
        } else {
            SessionKeyProcessResult::Noop
        };
    }
    if let Some(result) = process_editor_processor(session, key_event) {
        update_session_segment_tags(session);
        return result;
    }
    let before_input = session.engine.context().composition.input.clone();
    let before_highlighted = session.engine.context().highlighted;
    let commit = session.engine.process_key_event(key_event);
    update_key_binding_paging_state(session, key_event, &before_input, before_highlighted);
    update_session_segment_tags(session);
    if let Some(commit) = commit {
        return SessionKeyProcessResult::Commit(commit);
    }
    let context = session.engine.context();
    if !before_input.is_empty()
        || context.composition.input != before_input
        || context.highlighted != before_highlighted
    {
        SessionKeyProcessResult::Accepted
    } else {
        SessionKeyProcessResult::Noop
    }
}

pub(crate) fn ends_with_ascii_digit(text: &str) -> bool {
    text.as_bytes()
        .last()
        .is_some_and(|byte| byte.is_ascii_digit())
}

fn candidate_index_on_current_page(session: &SessionState, index: usize) -> Option<usize> {
    let page_size = session_menu_page_size(session);
    if index >= page_size || session.engine.context().candidates.is_empty() {
        return None;
    }

    let page_start = (session.engine.context().highlighted / page_size) * page_size;
    Some(page_start + index)
}

fn highlight_candidate_clamped_like_librime(session: &mut SessionState, index: usize) -> bool {
    let candidate_count = session.engine.context().candidates.len();
    if candidate_count == 0 {
        return false;
    }

    let new_index = index.min(candidate_count - 1);
    if new_index == session.engine.context().highlighted {
        return false;
    }

    session.engine.highlight_candidate(new_index)
}

pub(crate) fn open_runtime_config(
    config_id: &str,
    kind: ConfigOpenKind,
    config: *mut RimeConfig,
) -> Bool {
    if config.is_null() || validate_config_resource_id(config_id).is_none() {
        return FALSE;
    }

    let root = load_runtime_config_root(config_id, kind);
    unsafe { install_config_root(config, root) }
}

pub(crate) unsafe fn install_config_root(config: *mut RimeConfig, root: Value) -> Bool {
    if config.is_null() {
        return FALSE;
    }
    let state = Box::new(ConfigState {
        root,
        cstring_borrows: Vec::new(),
    });

    // SAFETY: `config` is non-null and points to caller-owned writable storage.
    // If it already owns config state from this shim, replace it to avoid
    // leaking when callers reuse a `RimeConfig` slot.
    unsafe {
        if !(*config).ptr.is_null() {
            drop(Box::from_raw((*config).ptr.cast::<ConfigState>()));
        }
        (*config).ptr = Box::into_raw(state).cast::<c_void>();
    }
    TRUE
}

pub(crate) fn load_runtime_config_root(config_id: &str, kind: ConfigOpenKind) -> Value {
    let Some(selected_path) = selected_runtime_config_path(config_id, kind) else {
        return Value::Null;
    };

    fs::read_to_string(selected_path)
        .ok()
        .and_then(|yaml| serde_yaml::from_str::<Value>(&yaml).ok())
        .unwrap_or(Value::Null)
}

fn runtime_config_roots(kind: ConfigOpenKind) -> Vec<String> {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    match kind {
        ConfigOpenKind::Deployed => vec![
            paths.staging_dir.to_string_lossy().into_owned(),
            paths.prebuilt_data_dir.to_string_lossy().into_owned(),
        ],
        ConfigOpenKind::User => vec![paths.user_data_dir.to_string_lossy().into_owned()],
    }
}

pub(crate) fn selected_runtime_config_path(
    resource_id: &str,
    kind: ConfigOpenKind,
) -> Option<PathBuf> {
    let resource_id = validate_runtime_config_resource_id(resource_id)?;
    let roots = runtime_config_roots(kind);
    roots
        .iter()
        .map(|root| config_file_path(root, &resource_id))
        .find(|path| path.exists())
        .or_else(|| {
            roots
                .first()
                .map(|root| config_file_path(root, &resource_id))
        })
}

pub(crate) fn selected_runtime_data_path(file_name: &str) -> Option<PathBuf> {
    let file_name = validate_data_resource_id(file_name)?;
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    [
        paths.staging_dir.to_string_lossy().into_owned(),
        paths.prebuilt_data_dir.to_string_lossy().into_owned(),
        paths.shared_data_dir.to_string_lossy().into_owned(),
    ]
    .into_iter()
    .map(|root| Path::new(&root).join(&file_name))
    .find(|path| path.is_file())
}

pub(crate) fn normalize_config_resource_id(config_id: &str) -> Option<String> {
    validate_config_resource_id(config_id)
}

fn config_file_path(root: &str, resource_id: &str) -> PathBuf {
    Path::new(root).join(format!("{resource_id}.yaml"))
}

pub(crate) fn deployed_schema_list_entries() -> Vec<(String, String)> {
    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Some(schema_list) = find_config_value(&default_config, "schema_list") else {
        return Vec::new();
    };
    let Value::Sequence(schema_list) = schema_list else {
        return Vec::new();
    };

    schema_list
        .iter()
        .filter_map(deployed_schema_list_entry)
        .map(|schema_id| {
            let schema_config =
                load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
            let name = find_config_value(&schema_config, "schema/name")
                .and_then(Value::as_str)
                .unwrap_or(&schema_id)
                .to_owned();
            (schema_id, name)
        })
        .collect()
}

pub(crate) fn schema_list_translator_entries_for_current(
    current_schema: &str,
    schema_config: &Value,
) -> Vec<(String, String)> {
    let mut entries = deployed_schema_list_entries()
        .into_iter()
        .filter(|(schema_id, _)| schema_id != current_schema)
        .map(|(schema_id, schema_name)| {
            let access_time = schema_access_time_quality(&schema_id);
            (schema_id, schema_name, access_time)
        })
        .collect::<Vec<_>>();

    let fix_order = find_config_value(schema_config, "switcher/fix_schema_list_order")
        .and_then(config_scalar_bool)
        .unwrap_or(false);
    if !fix_order {
        entries.sort_by(|(_, _, x), (_, _, y)| y.cmp(x));
    }

    entries
        .into_iter()
        .map(|(schema_id, schema_name, _)| (schema_id, schema_name))
        .collect()
}

fn schema_access_time_quality(schema_id: &str) -> i64 {
    let user_config = load_runtime_config_root("user", ConfigOpenKind::User);
    let Some(timestamp) =
        find_config_value(&user_config, &format!("var/schema_access_time/{schema_id}"))
            .and_then(config_scalar_int)
    else {
        return 0;
    };
    let timestamp = i64::from(timestamp);
    if timestamp <= session_activity_now() as i64 {
        timestamp
    } else {
        0
    }
}

pub(crate) fn deployed_schema_name(schema_id: &str) -> String {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    find_config_value(&schema_config, "schema/name")
        .and_then(Value::as_str)
        .unwrap_or(schema_id)
        .to_owned()
}

fn deployed_schema_list_entry(entry: &Value) -> Option<String> {
    let Value::Mapping(entry) = entry else {
        return None;
    };
    entry
        .get(Value::String("schema".to_owned()))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn state_label_for_session(
    session_id: RimeSessionId,
    option_name: &str,
    state: Bool,
    abbreviated: bool,
) -> Option<StateLabel> {
    if session_id == 0 {
        return None;
    }
    let schema_id = {
        let mut registry = sessions()
            .lock()
            .expect("session registry should not be poisoned");
        registry
            .get_session_mut(session_id)
            .map(|session| session.engine.status().schema_id)?
    };
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    switch_state_label(&schema_config, option_name, state, abbreviated)
}

pub(crate) fn context_menu_settings(schema_id: &str) -> ContextMenuSettings {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let page_size = find_config_value(&schema_config, "menu/page_size")
        .and_then(config_scalar_int)
        .filter(|page_size| *page_size > 0)
        .map(|page_size| page_size as usize)
        .unwrap_or(DEFAULT_PAGE_SIZE);
    let select_keys = find_config_value(&schema_config, "menu/alternative_select_keys")
        .and_then(Value::as_str)
        .filter(|select_keys| !select_keys.is_empty())
        .map(ToOwned::to_owned);
    let select_labels = match find_config_value(&schema_config, "menu/alternative_select_labels") {
        Some(Value::Sequence(labels)) => labels
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect(),
        _ => Vec::new(),
    };

    ContextMenuSettings {
        page_size,
        select_keys,
        select_labels,
    }
}

fn switch_state_label(
    schema_config: &Value,
    option_name: &str,
    state: Bool,
    abbreviated: bool,
) -> Option<StateLabel> {
    let switches = find_config_value(schema_config, "switches")?;
    let Value::Sequence(switches) = switches else {
        return None;
    };

    for the_switch in switches {
        let Value::Mapping(switch_map) = the_switch else {
            continue;
        };
        if switch_scalar_field(switch_map, "name").is_some_and(|name| name == option_name) {
            let state_index = usize::try_from(state).ok()?;
            return label_from_switch(switch_map, state_index, abbreviated);
        }

        let Some(options) = switch_map.get(Value::String("options".to_owned())) else {
            continue;
        };
        let Value::Sequence(options) = options else {
            continue;
        };
        for (option_index, option) in options.iter().enumerate() {
            if config_scalar_string(option).is_some_and(|name| name == option_name) {
                return (state != FALSE)
                    .then(|| label_from_switch(switch_map, option_index, abbreviated))
                    .flatten();
            }
        }
    }
    None
}

fn label_from_switch(
    switch_map: &Mapping,
    state_index: usize,
    abbreviated: bool,
) -> Option<StateLabel> {
    if abbreviated {
        if let Some(label) = label_list_value(switch_map, "abbrev", state_index) {
            let length = label.len();
            return Some(StateLabel {
                value: label,
                length,
            });
        }
    }

    let label = label_list_value(switch_map, "states", state_index)?;
    let length = if abbreviated {
        first_unicode_byte_length(&label)
    } else {
        label.len()
    };
    Some(StateLabel {
        value: label,
        length,
    })
}

fn label_list_value(switch_map: &Mapping, key: &str, state_index: usize) -> Option<String> {
    let Value::Sequence(values) = switch_map.get(Value::String(key.to_owned()))? else {
        return None;
    };
    values.get(state_index).and_then(config_scalar_string)
}

fn first_unicode_byte_length(value: &str) -> usize {
    value.chars().next().map_or(0, |first| first.len_utf8())
}

#[cfg(unix)]
pub(crate) fn librime_signature_modified_time() -> String {
    // librime's Signature::Sign stores a trimmed ctime(3) string.
    let now = unsafe { libc::time(ptr::null_mut()) };
    let mut buffer = [0 as c_char; 64];
    let written = unsafe { libc::ctime_r(&now, buffer.as_mut_ptr()) };
    if written.is_null() {
        return "0".to_owned();
    }
    unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .trim()
        .to_owned()
}

#[cfg(not(unix))]
pub(crate) fn librime_signature_modified_time() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or_else(
            |_| "0".to_owned(),
            |duration| duration.as_secs().to_string(),
        )
}

#[cfg(test)]
mod tests;
