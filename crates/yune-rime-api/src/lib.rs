use std::{
    collections::{HashMap, HashSet},
    ffi::{c_void, CStr, CString},
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    ptr,
};

use regex::Regex;
use serde_yaml::{Mapping, Value};
use yune_core::{
    parse_key_sequence, CandidateSource, Engine, FoldedSwitchOptions, KeyCode, KeyEvent,
    KeyModifiers, PunctuationTranslator, SwitchTranslatorSwitch,
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
pub use runtime::*;
pub use schema_api::*;
pub(crate) use schema_install::{
    install_schema_filter_chain, install_schema_segment_tags, install_schema_translator_chain,
    load_schema_recognizer_patterns, recognizer_patterns_match, schema_component_prescription,
    schema_string_list, update_session_segment_tags,
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
const XK_LEFT: c_int = 0xff51;
const XK_UP: c_int = 0xff52;
const XK_RIGHT: c_int = 0xff53;
const XK_DOWN: c_int = 0xff54;
const XK_PAGE_UP: c_int = 0xff55;
const XK_PAGE_DOWN: c_int = 0xff56;
const XK_END: c_int = 0xff57;
const XK_KP_ENTER: c_int = 0xff8d;
const XK_KP_HOME: c_int = 0xff95;
const XK_KP_LEFT: c_int = 0xff96;
const XK_KP_UP: c_int = 0xff97;
const XK_KP_RIGHT: c_int = 0xff98;
const XK_KP_DOWN: c_int = 0xff99;
const XK_KP_PAGE_UP: c_int = 0xff9a;
const XK_KP_PAGE_DOWN: c_int = 0xff9b;
const XK_KP_END: c_int = 0xff9c;
const XK_KP_0: c_int = 0xffb0;
const XK_KP_9: c_int = 0xffb9;
const XK_EISU_TOGGLE: c_int = 0xff30;
const XK_SHIFT_L: c_int = 0xffe1;
const XK_SHIFT_R: c_int = 0xffe2;
const XK_CONTROL_L: c_int = 0xffe3;
const XK_CONTROL_R: c_int = 0xffe4;
const XK_CAPS_LOCK: c_int = 0xffe5;
const XK_ALT_L: c_int = 0xffe9;
const XK_ALT_R: c_int = 0xffea;
const XK_SUPER_L: c_int = 0xffeb;
const XK_SUPER_R: c_int = 0xffec;
const K_SHIFT_MASK: c_int = 1 << 0;
const K_LOCK_MASK: c_int = 1 << 1;
const K_CONTROL_MASK: c_int = 1 << 2;
const K_ALT_MASK: c_int = 1 << 3;
const K_SUPER_MASK: c_int = 1 << 26;
const K_RELEASE_MASK: c_int = 1 << 30;
const DEFAULT_PAGE_SIZE: usize = 5;
pub(crate) const RIME_VERSION_BYTES: &[u8] =
    concat!("yune-rime-api ", env!("CARGO_PKG_VERSION"), "\0").as_bytes();

pub(crate) struct KeyBinderProcessor {
    bindings: HashMap<KeyEvent, Vec<KeyBinding>>,
    redirecting: bool,
    last_key: Option<char>,
}

struct KeyBinding {
    condition: KeyBindingCondition,
    action: KeyBindingAction,
}

pub(crate) struct SpellerProcessor {
    alphabet: String,
    delimiters: String,
    initials: String,
    finals: String,
    max_code_length: usize,
    auto_select: bool,
    auto_select_pattern: Option<Regex>,
    auto_clear: SpellerAutoClear,
    use_space: bool,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum SpellerAutoClear {
    None,
    Auto,
    Manual,
    MaxLength,
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

#[derive(Clone, Copy)]
pub(crate) enum EditorBindingAction {
    Noop,
    Action(EditorAction),
}

#[derive(Clone, Copy)]
enum EditorAction {
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
    horizontal_stacked: HashMap<KeyEvent, SelectorBindingAction>,
    horizontal_linear: HashMap<KeyEvent, SelectorBindingAction>,
    vertical_stacked: HashMap<KeyEvent, SelectorBindingAction>,
    vertical_linear: HashMap<KeyEvent, SelectorBindingAction>,
}

#[derive(Clone, Copy)]
enum SelectorBindingAction {
    Noop,
    Action(SelectorLayoutAction),
}

#[derive(Default)]
pub(crate) struct NavigatorBindings {
    horizontal: HashMap<KeyEvent, NavigatorBindingAction>,
    vertical: HashMap<KeyEvent, NavigatorBindingAction>,
}

#[derive(Clone, Copy)]
enum NavigatorBindingAction {
    Noop,
    Action(NavigatorAction),
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum NavigatorSyllableJumpPosition {
    AfterDelimiter,
    BeforeDelimiter,
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
    use_space: bool,
    patterns: Vec<MatcherPattern>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AsciiComposerProcessResult {
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
    use_space: bool,
    digit_separators: String,
    digit_separator_commit: bool,
    half_shape_alternating_counts: HashMap<String, usize>,
    full_shape_alternating_counts: HashMap<String, usize>,
    symbol_alternating_counts: HashMap<String, usize>,
    half_shape_unique_commits: HashMap<String, String>,
    full_shape_unique_commits: HashMap<String, String>,
    symbol_unique_commits: HashMap<String, String>,
    half_shape_pairs: HashMap<String, [String; 2]>,
    full_shape_pairs: HashMap<String, [String; 2]>,
    symbol_pairs: HashMap<String, [String; 2]>,
    pair_oddness: HashMap<String, usize>,
    pending_digit_separator: Option<String>,
}

pub(crate) struct ChordComposerProcessor {
    alphabet: Vec<char>,
    algebra: ChordProjection,
    output_format: ChordProjection,
    prompt_format: ChordProjection,
    bindings: HashMap<KeyEvent, ChordComposerBindingAction>,
    use_control: bool,
    use_alt: bool,
    use_shift: bool,
    use_super: bool,
    use_caps: bool,
    raw_sequence: String,
    pressed_keys: HashSet<char>,
    recognized_chord: HashSet<char>,
    prompt: Option<String>,
    finish_on_first_release: bool,
    was_composing: bool,
}

impl ChordComposerProcessor {
    fn clear_chord_state(&mut self) {
        self.raw_sequence.clear();
        self.pressed_keys.clear();
        self.recognized_chord.clear();
        self.prompt = None;
    }
}

#[derive(Clone, Copy)]
enum ChordComposerBindingAction {
    CommitRawInput,
}

#[derive(Clone, Default)]
struct ChordProjection {
    formulas: Vec<ChordProjectionFormula>,
}

#[derive(Clone)]
enum ChordProjectionFormula {
    Transliterate(Vec<(char, char)>),
    Transform { pattern: Regex, replacement: String },
    Erase(Regex),
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

enum PunctuationProcessResult {
    Accepted,
    Commit(String),
}

struct SpellerProcessResult {
    accepted: bool,
    commit: Option<String>,
}

enum SessionKeyProcessResult {
    Noop,
    Accepted,
    Commit(String),
    RejectedCommit(String),
}

impl ChordProjection {
    fn parse(formulas: &[String]) -> Self {
        let mut parsed = Vec::new();
        for formula in formulas {
            let Some(parsed_formula) = ChordProjectionFormula::parse(formula) else {
                return Self::default();
            };
            parsed.push(parsed_formula);
        }
        Self { formulas: parsed }
    }

    fn apply(&self, value: &mut String) {
        for formula in &self.formulas {
            formula.apply(value);
            if value.is_empty() {
                break;
            }
        }
    }
}

impl ChordProjectionFormula {
    fn parse(definition: &str) -> Option<Self> {
        let separator = definition.chars().find(|ch| !ch.is_ascii_lowercase())?;
        let args = definition.split(separator).collect::<Vec<_>>();
        match args.first().copied()? {
            "xlit" => Self::parse_xlit(&args),
            "xform" => Self::parse_xform(&args),
            "erase" => Self::parse_erase(&args),
            _ => None,
        }
    }

    fn parse_xlit(args: &[&str]) -> Option<Self> {
        if args.len() < 3 {
            return None;
        }
        let left = args[1].chars().collect::<Vec<_>>();
        let right = args[2].chars().collect::<Vec<_>>();
        if left.len() != right.len() {
            return None;
        }
        Some(Self::Transliterate(left.into_iter().zip(right).collect()))
    }

    fn parse_xform(args: &[&str]) -> Option<Self> {
        if args.len() < 3 || args[1].is_empty() {
            return None;
        }
        Some(Self::Transform {
            pattern: Regex::new(args[1]).ok()?,
            replacement: args[2].to_owned(),
        })
    }

    fn parse_erase(args: &[&str]) -> Option<Self> {
        if args.len() < 2 || args[1].is_empty() {
            return None;
        }
        Some(Self::Erase(Regex::new(args[1]).ok()?))
    }

    fn apply(&self, value: &mut String) {
        match self {
            Self::Transliterate(char_map) => {
                let transformed = value
                    .chars()
                    .map(|ch| {
                        char_map
                            .iter()
                            .find_map(|(source, replacement)| {
                                (*source == ch).then_some(*replacement)
                            })
                            .unwrap_or(ch)
                    })
                    .collect::<String>();
                *value = transformed;
            }
            Self::Transform {
                pattern,
                replacement,
            } => {
                *value = pattern
                    .replace_all(value, replacement.as_str())
                    .into_owned();
            }
            Self::Erase(pattern) => {
                if pattern.is_match(value) {
                    value.clear();
                }
            }
        }
    }
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

fn session_menu_page_size(session: &SessionState) -> usize {
    context_menu_settings(&session.engine.status().schema_id).page_size
}

#[derive(Clone, Copy)]
enum SelectorLayoutAction {
    PreviousCandidate,
    NextCandidate,
    PreviousPage,
    NextPage,
    Home,
    End,
}

#[derive(Clone, Copy)]
enum NavigatorAction {
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

fn process_selector_layout_key(
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

fn selector_configured_action(
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

fn session_has_modified_printable_binding(session: &SessionState, key_event: KeyEvent) -> bool {
    if session
        .key_binder
        .as_ref()
        .is_some_and(|processor| processor.bindings.contains_key(&key_event))
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
            .is_some_and(|processor| processor.bindings.contains_key(&key_event))
}

fn session_chord_composer_accepts_printable(session: &SessionState, key_event: KeyEvent) -> bool {
    let Some(composer) = session.chord_composer.as_ref() else {
        return false;
    };
    let KeyCode::Character(ch) = key_event.code else {
        return false;
    };
    composer.alphabet.contains(&ch)
        && chord_composer_allows_modifiers(composer, key_event.modifiers)
}

fn process_navigator_configured_key(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<bool> {
    if session.engine.context().composition.input.is_empty() || key_event.modifiers.release {
        return None;
    }
    let is_vertical = session.engine.get_option("_vertical");
    let action = navigator_configured_action(session, is_vertical, key_event)?;
    match action {
        NavigatorBindingAction::Noop => Some(false),
        NavigatorBindingAction::Action(action) => {
            apply_navigator_action(session, action);
            Some(true)
        }
    }
}

fn process_navigator_delimiter_key(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<bool> {
    if session.engine.context().composition.input.is_empty() || key_event.modifiers.release {
        return None;
    }
    let input = &session.engine.context().composition.input;
    if !input
        .chars()
        .any(|ch| session.navigator_delimiters.contains(ch))
    {
        return None;
    }
    let action = default_navigator_syllable_action(key_event)?;
    apply_navigator_action(session, action);
    Some(true)
}

fn default_navigator_syllable_action(key_event: KeyEvent) -> Option<NavigatorAction> {
    let exact_control_or_shift = (key_event.modifiers.control ^ key_event.modifiers.shift)
        && !key_event.modifiers.lock
        && !key_event.modifiers.alt
        && !key_event.modifiers.super_key
        && !key_event.modifiers.hyper
        && !key_event.modifiers.meta
        && !key_event.modifiers.release;
    if !exact_control_or_shift {
        return None;
    }

    match key_event.code {
        KeyCode::MoveCaretLeft | KeyCode::MoveCaretLeftBySyllable => {
            Some(NavigatorAction::LeftBySyllable)
        }
        KeyCode::MoveCaretRight | KeyCode::MoveCaretRightBySyllable => {
            Some(NavigatorAction::RightBySyllable)
        }
        _ => None,
    }
}

fn navigator_configured_action(
    session: &SessionState,
    is_vertical: bool,
    key_event: KeyEvent,
) -> Option<NavigatorBindingAction> {
    let bindings = if is_vertical {
        &session.navigator_bindings.vertical
    } else {
        &session.navigator_bindings.horizontal
    };
    bindings.get(&key_event).copied()
}

fn apply_navigator_action(session: &mut SessionState, action: NavigatorAction) {
    match action {
        NavigatorAction::Rewind => {
            session.engine.move_caret_left();
        }
        NavigatorAction::Forward => {
            session.engine.move_caret_right();
        }
        NavigatorAction::LeftByChar => {
            session.engine.move_caret_left_by_char();
        }
        NavigatorAction::RightByChar => {
            session.engine.move_caret_right_by_char();
        }
        NavigatorAction::LeftBySyllable | NavigatorAction::LeftBySyllableNoLoop => {
            let loop_at_boundary = matches!(action, NavigatorAction::LeftBySyllable);
            if !move_caret_left_by_delimited_syllable(session, loop_at_boundary) {
                session.engine.move_caret_left_by_syllable();
            }
        }
        NavigatorAction::RightBySyllable | NavigatorAction::RightBySyllableNoLoop => {
            let loop_at_boundary = matches!(action, NavigatorAction::RightBySyllable);
            if !move_caret_right_by_delimited_syllable(session, loop_at_boundary) {
                session.engine.move_caret_right_by_syllable();
            }
        }
        NavigatorAction::LeftByCharNoLoop => {
            session.engine.move_caret_left();
        }
        NavigatorAction::RightByCharNoLoop => {
            session.engine.move_caret_right();
        }
        NavigatorAction::Home => {
            session.engine.move_caret_home();
        }
        NavigatorAction::End => {
            session.engine.move_caret_end();
        }
    }
}

fn move_caret_left_by_delimited_syllable(
    session: &mut SessionState,
    loop_at_boundary: bool,
) -> bool {
    let context = session.engine.context();
    let input = &context.composition.input;
    let caret = context.composition.caret.min(input.len());
    if input.is_empty() || !input.is_ascii() {
        return false;
    }

    let stops = navigator_syllable_stops(
        input,
        &session.navigator_delimiters,
        session.navigator_syllable_jump_position,
    );
    let next_caret = stops
        .iter()
        .rev()
        .copied()
        .find(|stop| *stop < caret)
        .or_else(|| {
            loop_at_boundary
                .then(|| stops.iter().rev().copied().find(|stop| *stop < input.len()))
                .flatten()
        });
    let Some(next_caret) = next_caret else {
        return false;
    };
    if next_caret == caret {
        return false;
    }
    session.engine.set_caret_pos(next_caret);
    true
}

fn move_caret_right_by_delimited_syllable(
    session: &mut SessionState,
    loop_at_boundary: bool,
) -> bool {
    let context = session.engine.context();
    let input = &context.composition.input;
    let caret = context.composition.caret.min(input.len());
    if input.is_empty() || !input.is_ascii() {
        return false;
    }

    let stops = navigator_syllable_stops(
        input,
        &session.navigator_delimiters,
        session.navigator_syllable_jump_position,
    );
    let next_caret = stops
        .iter()
        .copied()
        .find(|stop| *stop > caret)
        .or_else(|| {
            loop_at_boundary
                .then(|| stops.iter().copied().find(|stop| *stop > 0))
                .flatten()
        });
    let Some(next_caret) = next_caret else {
        return false;
    };
    if next_caret == caret {
        return false;
    }
    session.engine.set_caret_pos(next_caret);
    true
}

fn navigator_syllable_stops(
    input: &str,
    delimiters: &str,
    jump_position: NavigatorSyllableJumpPosition,
) -> Vec<usize> {
    let mut stops = vec![0, input.len()];
    let mut delimiter_run_start = None;
    for (index, ch) in input.char_indices() {
        if delimiters.contains(ch) {
            delimiter_run_start.get_or_insert(index);
            continue;
        }

        if let Some(start) = delimiter_run_start.take() {
            stops.push(match jump_position {
                NavigatorSyllableJumpPosition::AfterDelimiter => index,
                NavigatorSyllableJumpPosition::BeforeDelimiter => start,
            });
        }
    }
    if let Some(start) = delimiter_run_start {
        stops.push(match jump_position {
            NavigatorSyllableJumpPosition::AfterDelimiter => input.len(),
            NavigatorSyllableJumpPosition::BeforeDelimiter => start,
        });
    }
    stops.sort_unstable();
    stops.dedup();
    stops
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

pub(crate) fn install_schema_ascii_composer_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(processors)) = find_config_value(&schema_config, "engine/processors")
    else {
        return;
    };
    session.ascii_composer_enabled = processors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .any(|(component_name, _)| component_name == "ascii_composer");
    if !session.ascii_composer_enabled {
        return;
    }

    session.ascii_composer_switch_bindings = load_ascii_composer_switch_bindings(&schema_config);
    if session.ascii_composer_switch_bindings.is_empty() {
        let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
        session.ascii_composer_switch_bindings =
            load_ascii_composer_switch_bindings(&default_config);
    }
}

fn load_ascii_composer_switch_bindings(
    schema_config: &Value,
) -> HashMap<c_int, AsciiModeSwitchStyle> {
    let Some(Value::Mapping(bindings)) =
        find_config_value(schema_config, "ascii_composer/switch_key")
    else {
        return HashMap::new();
    };

    bindings
        .iter()
        .filter_map(|(key, style)| {
            let key = config_scalar_string(key)?;
            let style =
                config_scalar_string(style).and_then(|style| ascii_mode_switch_style(&style))?;
            let key_c = CString::new(key).ok()?;
            // SAFETY: `key_c` is a valid NUL-terminated key-name string.
            let keycode = unsafe { RimeGetKeycodeByName(key_c.as_ptr()) };
            (keycode != 0x00ff_ffff).then_some((keycode, style))
        })
        .collect()
}

fn ascii_mode_switch_style(style: &str) -> Option<AsciiModeSwitchStyle> {
    match style {
        "inline_ascii" => Some(AsciiModeSwitchStyle::InlineAscii),
        "commit_text" => Some(AsciiModeSwitchStyle::CommitText),
        "commit_code" => Some(AsciiModeSwitchStyle::CommitCode),
        "clear" => Some(AsciiModeSwitchStyle::Clear),
        "set_ascii_mode" => Some(AsciiModeSwitchStyle::SetAsciiMode),
        "unset_ascii_mode" => Some(AsciiModeSwitchStyle::UnsetAsciiMode),
        _ => None,
    }
}

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

pub(crate) fn install_schema_chord_composer_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    if !schema_engine_processors_include(&schema_config, "chord_composer") {
        return;
    }

    let alphabet = find_config_value(&schema_config, "chord_composer/alphabet")
        .and_then(config_scalar_string)
        .unwrap_or_default()
        .chars()
        .collect::<Vec<_>>();
    if alphabet.is_empty() {
        return;
    }

    session.engine.set_option("_chord_typing", true);
    session.chord_composer = Some(ChordComposerProcessor {
        alphabet,
        algebra: ChordProjection::parse(&schema_string_list(
            &schema_config,
            "chord_composer/algebra",
        )),
        output_format: ChordProjection::parse(&schema_string_list(
            &schema_config,
            "chord_composer/output_format",
        )),
        prompt_format: ChordProjection::parse(&schema_string_list(
            &schema_config,
            "chord_composer/prompt_format",
        )),
        bindings: load_chord_composer_bindings(&schema_config),
        use_control: find_config_value(&schema_config, "chord_composer/use_control")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
        use_alt: find_config_value(&schema_config, "chord_composer/use_alt")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
        use_shift: find_config_value(&schema_config, "chord_composer/use_shift")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
        use_super: find_config_value(&schema_config, "chord_composer/use_super")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
        use_caps: find_config_value(&schema_config, "chord_composer/use_caps")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
        raw_sequence: String::new(),
        pressed_keys: HashSet::new(),
        recognized_chord: HashSet::new(),
        prompt: None,
        finish_on_first_release: find_config_value(
            &schema_config,
            "chord_composer/finish_chord_on_first_key_release",
        )
        .and_then(config_scalar_bool)
        .unwrap_or(false),
        was_composing: false,
    });
}

fn load_chord_composer_bindings(
    schema_config: &Value,
) -> HashMap<KeyEvent, ChordComposerBindingAction> {
    let Some(Value::Mapping(config_bindings)) =
        find_config_value(schema_config, "chord_composer/bindings")
    else {
        return HashMap::new();
    };

    let mut bindings = HashMap::new();
    for (key, action) in config_bindings {
        let Some(key) = config_scalar_string(key) else {
            continue;
        };
        let Some(key_event) = parse_single_key_binding_event(&key) else {
            continue;
        };
        let Some(action) = action.as_str() else {
            continue;
        };
        match action {
            "commit_raw_input" => {
                bindings.insert(key_event, ChordComposerBindingAction::CommitRawInput);
            }
            "noop" => {
                bindings.remove(&key_event);
            }
            _ => {}
        }
    }
    bindings
}

pub(crate) fn install_schema_speller_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    if !schema_engine_processors_include(&schema_config, "speller") {
        return;
    }

    let alphabet = find_config_value(&schema_config, "speller/alphabet")
        .and_then(config_scalar_string)
        .unwrap_or_else(|| "zyxwvutsrqponmlkjihgfedcba".to_owned());
    let initials = find_config_value(&schema_config, "speller/initials")
        .and_then(config_scalar_string)
        .filter(|initials| !initials.is_empty())
        .unwrap_or_else(|| alphabet.clone());
    session.speller = Some(SpellerProcessor {
        alphabet,
        delimiters: find_config_value(&schema_config, "speller/delimiter")
            .and_then(config_scalar_string)
            .unwrap_or_default(),
        initials,
        finals: find_config_value(&schema_config, "speller/finals")
            .and_then(config_scalar_string)
            .unwrap_or_default(),
        max_code_length: find_config_value(&schema_config, "speller/max_code_length")
            .and_then(config_scalar_int)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0),
        auto_select: find_config_value(&schema_config, "speller/auto_select")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
        auto_select_pattern: find_config_value(&schema_config, "speller/auto_select_pattern")
            .and_then(config_scalar_string)
            .and_then(|pattern| Regex::new(&pattern).ok()),
        auto_clear: find_config_value(&schema_config, "speller/auto_clear")
            .and_then(config_scalar_string)
            .and_then(|value| match value.as_str() {
                "auto" => Some(SpellerAutoClear::Auto),
                "manual" => Some(SpellerAutoClear::Manual),
                "max_length" => Some(SpellerAutoClear::MaxLength),
                _ => None,
            })
            .unwrap_or(SpellerAutoClear::None),
        use_space: find_config_value(&schema_config, "speller/use_space")
            .and_then(config_scalar_bool)
            .unwrap_or(false),
    });
}

pub(crate) fn install_schema_recognizer_processor(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let Some(Value::Sequence(processors)) = find_config_value(&schema_config, "engine/processors")
    else {
        return;
    };
    let Some(name_space) = processors
        .iter()
        .filter_map(Value::as_str)
        .map(schema_component_prescription)
        .find_map(|(component_name, name_space)| {
            (component_name == "recognizer")
                .then(|| {
                    let name_space = name_space.unwrap_or("recognizer");
                    if name_space == "processor" {
                        "recognizer"
                    } else {
                        name_space
                    }
                })
                .filter(|name_space| !name_space.is_empty())
        })
    else {
        return;
    };
    let patterns = load_schema_recognizer_patterns(&schema_config, name_space);
    if patterns.is_empty() {
        return;
    }
    let use_space = find_config_value(&schema_config, &format!("{name_space}/use_space"))
        .and_then(config_scalar_bool)
        .unwrap_or(false);
    session.recognizer_processor = Some(RecognizerProcessor {
        use_space,
        patterns,
    });
}

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

pub(crate) fn install_schema_navigator_bindings(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    session.navigator_delimiters = find_config_value(&schema_config, "speller/delimiter")
        .and_then(config_scalar_string)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| " ".to_owned());
    session.navigator_syllable_jump_position =
        match find_config_value(&schema_config, "navigator/syllable_jump_position")
            .and_then(config_scalar_string)
            .as_deref()
        {
            Some("before_delimiter") => NavigatorSyllableJumpPosition::BeforeDelimiter,
            _ => NavigatorSyllableJumpPosition::AfterDelimiter,
        };
    load_navigator_binding_section(
        &schema_config,
        "navigator",
        &mut session.navigator_bindings.horizontal,
    );
    load_navigator_binding_section(
        &schema_config,
        "navigator/vertical",
        &mut session.navigator_bindings.vertical,
    );
}

fn load_navigator_binding_section(
    schema_config: &Value,
    section: &str,
    bindings: &mut HashMap<KeyEvent, NavigatorBindingAction>,
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
        let Some(action) = action.as_str().and_then(navigator_binding_action_from_name) else {
            continue;
        };
        bindings.insert(key_event, action);
    }
}

fn navigator_binding_action_from_name(action: &str) -> Option<NavigatorBindingAction> {
    let action = match action {
        "noop" => NavigatorBindingAction::Noop,
        "rewind" => NavigatorBindingAction::Action(NavigatorAction::Rewind),
        "forward" => NavigatorBindingAction::Action(NavigatorAction::Forward),
        "left_by_char" => NavigatorBindingAction::Action(NavigatorAction::LeftByChar),
        "right_by_char" => NavigatorBindingAction::Action(NavigatorAction::RightByChar),
        "left_by_syllable" => NavigatorBindingAction::Action(NavigatorAction::LeftBySyllable),
        "right_by_syllable" => NavigatorBindingAction::Action(NavigatorAction::RightBySyllable),
        "left_by_char_no_loop" => NavigatorBindingAction::Action(NavigatorAction::LeftByCharNoLoop),
        "right_by_char_no_loop" => {
            NavigatorBindingAction::Action(NavigatorAction::RightByCharNoLoop)
        }
        "left_by_syllable_no_loop" => {
            NavigatorBindingAction::Action(NavigatorAction::LeftBySyllableNoLoop)
        }
        "right_by_syllable_no_loop" => {
            NavigatorBindingAction::Action(NavigatorAction::RightBySyllableNoLoop)
        }
        "home" => NavigatorBindingAction::Action(NavigatorAction::Home),
        "end" => NavigatorBindingAction::Action(NavigatorAction::End),
        _ => return None,
    };
    Some(action)
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

fn parse_single_key_binding_event(pattern: &str) -> Option<KeyEvent> {
    let sequence = if pattern.chars().count() == 1 {
        pattern.to_owned()
    } else {
        format!("{{{pattern}}}")
    };
    let mut events = parse_key_sequence(&sequence).ok()?;
    (events.len() == 1).then(|| events.remove(0))
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
        half_shape_alternating_counts: punctuation_alternating_counts_from_config(
            &schema_config,
            "half_shape",
        ),
        full_shape_alternating_counts: punctuation_alternating_counts_from_config(
            &schema_config,
            "full_shape",
        ),
        symbol_alternating_counts: punctuation_alternating_counts_from_config(
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
        && processor.half_shape_alternating_counts.is_empty()
        && processor.full_shape_alternating_counts.is_empty()
        && processor.symbol_alternating_counts.is_empty()
        && processor.half_shape_pairs.is_empty()
        && processor.full_shape_pairs.is_empty()
        && processor.symbol_pairs.is_empty()
    {
        return;
    }
    session.punctuation_processor = Some(processor);
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

fn switch_reset_value(switch_map: &Mapping) -> Option<c_int> {
    let reset = switch_map.get(Value::String("reset".to_owned()))?;
    match reset {
        Value::Null | Value::Sequence(_) | Value::Mapping(_) => None,
        scalar => Some(config_scalar_int(scalar).unwrap_or(0)),
    }
}

fn switch_scalar_field(switch_map: &Mapping, key: &str) -> Option<String> {
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

fn punctuation_alternating_counts_from_config(
    schema_config: &Value,
    shape: &str,
) -> HashMap<String, usize> {
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
            let count = values.iter().filter_map(config_scalar_string).count();
            (count > 0).then_some((key, count))
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
    if let Some(text) = config_scalar_string(definition) {
        return Some(text);
    }
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

fn process_session_key_event(
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

fn sync_chord_composer_context_update(session: &mut SessionState) {
    let Some(composer) = session.chord_composer.as_mut() else {
        return;
    };
    let is_composing =
        !session.engine.context().composition.input.is_empty() || composer.prompt.is_some();
    if is_composing {
        composer.was_composing = true;
    } else if composer.was_composing {
        composer.was_composing = false;
        composer.raw_sequence.clear();
    }
}

fn process_chord_composer_processor(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<SessionKeyProcessResult> {
    if session.engine.get_option("ascii_mode") {
        return None;
    }
    let composer = session.chord_composer.as_ref()?;

    if let Some(action) = composer.bindings.get(&key_event).copied() {
        return Some(apply_chord_composer_binding(session, action));
    }

    if !key_event.modifiers.release
        && matches!(key_event.code, KeyCode::Backspace | KeyCode::Escape)
    {
        if let Some(composer) = session.chord_composer.as_mut() {
            composer.clear_chord_state();
        }
        return None;
    }

    let KeyCode::Character(ch) = key_event.code else {
        if let Some(composer) = session.chord_composer.as_mut() {
            composer.clear_chord_state();
        }
        return None;
    };
    let composer = session.chord_composer.as_mut()?;

    if !chord_composer_allows_modifiers(composer, key_event.modifiers) {
        composer.clear_chord_state();
        return None;
    }

    if !composer.alphabet.contains(&ch) {
        composer.clear_chord_state();
        return None;
    }

    if key_event.modifiers.release {
        let was_pressed = composer.pressed_keys.remove(&ch);
        if !was_pressed {
            return Some(SessionKeyProcessResult::Noop);
        }
        if !composer.recognized_chord.is_empty()
            && (composer.finish_on_first_release || composer.pressed_keys.is_empty())
        {
            let mut code = serialize_chord_composer_code(composer);
            composer.recognized_chord.clear();
            composer.prompt = None;
            return Some(feed_chord_composer_output(session, &mut code));
        }
        return Some(SessionKeyProcessResult::Accepted);
    }

    let should_buffer_raw = !key_event.modifiers.control
        && !key_event.modifiers.alt
        && !key_event.modifiers.super_key
        && !key_event.modifiers.lock;
    if should_buffer_raw
        && (session.engine.context().composition.input.is_empty()
            || !composer.raw_sequence.is_empty())
    {
        composer.raw_sequence.push(ch);
    }
    composer.pressed_keys.insert(ch);
    composer.recognized_chord.insert(ch);
    composer.prompt = chord_composer_prompt(composer);
    Some(SessionKeyProcessResult::Accepted)
}

fn chord_composer_allows_modifiers(
    composer: &ChordComposerProcessor,
    modifiers: KeyModifiers,
) -> bool {
    (!modifiers.control || composer.use_control)
        && (!modifiers.alt || composer.use_alt)
        && (!modifiers.shift || composer.use_shift)
        && (!modifiers.super_key || composer.use_super)
        && (!modifiers.lock || composer.use_caps)
        && !modifiers.hyper
        && !modifiers.meta
}

fn apply_chord_composer_binding(
    session: &mut SessionState,
    action: ChordComposerBindingAction,
) -> SessionKeyProcessResult {
    match action {
        ChordComposerBindingAction::CommitRawInput => {
            let raw_sequence = session
                .chord_composer
                .as_mut()
                .map(|composer| {
                    composer.prompt = None;
                    std::mem::take(&mut composer.raw_sequence)
                })
                .unwrap_or_default();
            if raw_sequence.is_empty() {
                return SessionKeyProcessResult::Noop;
            }
            session.engine.set_input(raw_sequence);
            session.engine.commit_raw_input().map_or(
                SessionKeyProcessResult::Accepted,
                SessionKeyProcessResult::Commit,
            )
        }
    }
}

fn serialize_chord_composer_code(composer: &ChordComposerProcessor) -> String {
    let mut code = composer
        .alphabet
        .iter()
        .filter(|ch| composer.recognized_chord.contains(ch))
        .collect::<String>();
    composer.algebra.apply(&mut code);
    composer.output_format.apply(&mut code);
    code
}

fn chord_composer_prompt(composer: &ChordComposerProcessor) -> Option<String> {
    if composer.recognized_chord.is_empty()
        || (composer.recognized_chord.len() == 1 && composer.recognized_chord.contains(&' '))
    {
        return None;
    }
    let mut prompt = composer
        .alphabet
        .iter()
        .filter(|ch| composer.recognized_chord.contains(ch))
        .collect::<String>();
    composer.algebra.apply(&mut prompt);
    composer.prompt_format.apply(&mut prompt);
    (!prompt.is_empty()).then_some(prompt)
}

fn feed_chord_composer_output(
    session: &mut SessionState,
    code: &mut str,
) -> SessionKeyProcessResult {
    if code.is_empty() {
        return SessionKeyProcessResult::Accepted;
    }
    let Ok(events) = parse_key_sequence(code) else {
        return SessionKeyProcessResult::Accepted;
    };

    let mut commits = Vec::new();
    for event in events {
        let before_input = session.engine.context().composition.input.clone();
        let before_highlighted = session.engine.context().highlighted;
        if let Some(commit) = session.engine.process_key_event(event) {
            commits.push(commit);
            continue;
        }
        let context = session.engine.context();
        if context.composition.input == before_input
            && context.highlighted == before_highlighted
            && event.modifiers.is_empty()
        {
            if let KeyCode::Character(ch) = event.code {
                if let Some(composer) = session.chord_composer.as_mut() {
                    composer.raw_sequence.clear();
                }
                commits.push(session.engine.record_commit(ch.to_string()));
            }
        }
    }

    if commits.is_empty() {
        SessionKeyProcessResult::Accepted
    } else {
        SessionKeyProcessResult::Commit(commits.concat())
    }
}

fn process_editor_processor(
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

fn process_ascii_composer_processor(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> AsciiComposerProcessResult {
    if !session.ascii_composer_enabled
        || key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.hyper
        || key_event.modifiers.meta
        || key_event.modifiers.release
    {
        return AsciiComposerProcessResult::Noop;
    }
    if key_event.modifiers.shift && matches!(key_event.code, KeyCode::Character(' ')) {
        return AsciiComposerProcessResult::Noop;
    }
    if !session.engine.status().is_ascii_mode {
        return AsciiComposerProcessResult::Noop;
    }
    let KeyCode::Character(ch) = key_event.code else {
        return AsciiComposerProcessResult::Noop;
    };
    if !(('\u{20}'..'\u{80}').contains(&ch)) {
        return AsciiComposerProcessResult::Noop;
    }
    if session.engine.context().composition.input.is_empty() {
        return AsciiComposerProcessResult::Rejected;
    }

    let mut input = session.engine.context().composition.input.clone();
    input.push(ch);
    session.engine.set_input(input);
    AsciiComposerProcessResult::Accepted(None)
}

fn is_ascii_composer_modifier_key(keycode: c_int) -> bool {
    matches!(
        keycode,
        XK_SHIFT_L
            | XK_SHIFT_R
            | XK_CONTROL_L
            | XK_CONTROL_R
            | XK_ALT_L
            | XK_ALT_R
            | XK_SUPER_L
            | XK_SUPER_R
    )
}

fn process_ascii_composer_modifier_switch_key(
    session: &mut SessionState,
    keycode: c_int,
    mask: c_int,
) -> Option<String> {
    if !session.ascii_composer_enabled {
        return None;
    }
    if mask == K_RELEASE_MASK {
        let pressed = session.ascii_composer_pressed_switch_key.take();
        if pressed == Some(keycode) {
            return switch_ascii_mode_with_key(session, keycode);
        }
        return None;
    }

    match session.ascii_composer_pressed_switch_key {
        None => session.ascii_composer_pressed_switch_key = Some(keycode),
        Some(pressed) if pressed != keycode => session.ascii_composer_pressed_switch_key = None,
        Some(_) => {}
    }
    None
}

fn process_ascii_composer_switch_key(
    session: &mut SessionState,
    keycode: c_int,
) -> Option<Option<String>> {
    if !session.ascii_composer_enabled {
        return None;
    }
    Some(switch_ascii_mode_with_key(session, keycode))
}

fn switch_ascii_mode_with_key(session: &mut SessionState, keycode: c_int) -> Option<String> {
    let style = *session.ascii_composer_switch_bindings.get(&keycode)?;
    let old_mode = session.engine.status().is_ascii_mode;
    let new_mode = match style {
        AsciiModeSwitchStyle::SetAsciiMode => true,
        AsciiModeSwitchStyle::UnsetAsciiMode => false,
        AsciiModeSwitchStyle::InlineAscii
        | AsciiModeSwitchStyle::CommitText
        | AsciiModeSwitchStyle::CommitCode
        | AsciiModeSwitchStyle::Clear => !old_mode,
    };
    if old_mode == new_mode {
        return None;
    }
    switch_ascii_mode(session, new_mode, style)
}

fn process_ascii_composer_caps_lock_switch_key(
    session: &mut SessionState,
) -> Option<Option<String>> {
    if !session.ascii_composer_enabled {
        return None;
    }
    let mut style = *session.ascii_composer_switch_bindings.get(&XK_CAPS_LOCK)?;
    if matches!(
        style,
        AsciiModeSwitchStyle::InlineAscii
            | AsciiModeSwitchStyle::SetAsciiMode
            | AsciiModeSwitchStyle::UnsetAsciiMode
    ) {
        style = AsciiModeSwitchStyle::Clear;
    }
    Some(switch_ascii_mode(session, true, style))
}

fn switch_ascii_mode(
    session: &mut SessionState,
    ascii_mode: bool,
    style: AsciiModeSwitchStyle,
) -> Option<String> {
    let mut commit = None;
    let was_composing = !session.engine.context().composition.input.is_empty();
    if was_composing {
        match style {
            AsciiModeSwitchStyle::InlineAscii => {}
            AsciiModeSwitchStyle::CommitText => {
                commit = session.engine.commit_composition();
            }
            AsciiModeSwitchStyle::CommitCode => {
                commit = session.engine.commit_raw_input();
            }
            AsciiModeSwitchStyle::Clear
            | AsciiModeSwitchStyle::SetAsciiMode
            | AsciiModeSwitchStyle::UnsetAsciiMode => {
                session.engine.clear_composition();
            }
        }
    }
    session.ascii_composer_inline_ascii =
        was_composing && ascii_mode && style == AsciiModeSwitchStyle::InlineAscii;
    session.engine.set_option("ascii_mode", ascii_mode);
    commit
}

fn process_recognizer_processor(session: &mut SessionState, key_event: KeyEvent) -> bool {
    if key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.release
    {
        return false;
    }
    let KeyCode::Character(ch) = key_event.code else {
        return false;
    };
    if !((ch == ' '
        && session
            .recognizer_processor
            .as_ref()
            .is_some_and(|processor| processor.use_space))
        || (ch > '\u{20}' && ch < '\u{80}'))
    {
        return false;
    }
    let Some(processor) = &session.recognizer_processor else {
        return false;
    };

    let mut input = session.engine.context().composition.input.clone();
    input.push(ch);
    if !recognizer_patterns_match(&processor.patterns, &input) {
        return false;
    }
    session.engine.set_input(input);
    true
}

fn process_shape_processor(session: &SessionState, key_event: KeyEvent) -> Option<String> {
    if !session.engine.status().is_full_shape
        || key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.release
    {
        return None;
    }
    let KeyCode::Character(ch) = key_event.code else {
        return None;
    };
    if !('\u{20}'..='\u{7e}').contains(&ch) {
        return None;
    }
    Some(shape_formatted_ascii_text(&ch.to_string(), true))
}

fn process_speller_processor(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<SpellerProcessResult> {
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
    if !('\u{20}'..'\u{7f}').contains(&ch) {
        return None;
    }
    let Some(speller) = &session.speller else {
        return None;
    };
    if ch == ' ' {
        if !speller.use_space || key_event.modifiers.shift {
            return None;
        }
    } else {
        let is_alphabet = speller.alphabet.contains(ch);
        let is_delimiter = speller.delimiters.contains(ch);
        if !is_alphabet && !is_delimiter {
            let can_select_candidate =
                ch.is_ascii_digit() && !session.engine.context().candidates.is_empty();
            return if can_select_candidate {
                None
            } else {
                Some(SpellerProcessResult {
                    accepted: false,
                    commit: None,
                })
            };
        }
        let is_initial = speller.initials.contains(ch);
        if !is_initial
            && speller.expecting_initial(
                session.engine.context().composition.caret,
                &session.engine.context().composition.input,
            )
        {
            return Some(SpellerProcessResult {
                accepted: false,
                commit: None,
            });
        }
    }

    let auto_clear = speller.auto_clear;
    let max_code_length = speller.max_code_length;
    let auto_select = speller.auto_select;
    let auto_select_pattern = speller.auto_select_pattern.clone();
    let is_initial = ch != ' ' && speller.initials.contains(ch);
    let delimiters = speller.delimiters.clone();
    let commit = if is_initial
        && speller_auto_select_at_max_code_length(session, max_code_length, &delimiters)
    {
        session.engine.commit_composition()
    } else {
        None
    };
    if matches!(
        auto_clear,
        SpellerAutoClear::Manual | SpellerAutoClear::MaxLength
    ) && speller_auto_clear_condition(session, auto_clear, max_code_length)
    {
        session.engine.clear_composition();
    }
    let previous_match = speller_previous_match_backup(
        session,
        auto_select,
        max_code_length,
        auto_select_pattern.as_ref(),
    );

    let mut input = session.engine.context().composition.input.clone();
    input.push(ch);
    let appended_input = input.clone();
    session.engine.set_input(input);
    let commit = commit
        .or_else(|| {
            speller_auto_select_previous_match(
                session,
                previous_match,
                &appended_input,
                &delimiters,
            )
        })
        .or_else(|| {
            auto_select
                .then(|| {
                    speller_auto_select_unique_candidate(
                        session,
                        max_code_length,
                        auto_select_pattern.as_ref(),
                        &delimiters,
                    )
                })
                .flatten()
        });
    if auto_clear == SpellerAutoClear::Auto
        && speller_auto_clear_condition(session, auto_clear, max_code_length)
    {
        session.engine.clear_composition();
    }
    Some(SpellerProcessResult {
        accepted: true,
        commit,
    })
}

impl SpellerProcessor {
    fn expecting_initial(&self, caret_pos: usize, input: &str) -> bool {
        if caret_pos == 0 {
            return true;
        }
        let previous_char = input[..caret_pos].chars().last();
        previous_char.map_or(true, |ch| {
            self.finals.contains(ch) || !self.alphabet.contains(ch)
        })
    }
}

fn speller_auto_clear_condition(
    session: &SessionState,
    auto_clear: SpellerAutoClear,
    max_code_length: usize,
) -> bool {
    let context = session.engine.context();
    if speller_context_has_menu(context) || context.composition.input.is_empty() {
        return false;
    }
    auto_clear != SpellerAutoClear::MaxLength
        || max_code_length == 0
        || context.composition.input.len() >= max_code_length
}

fn speller_auto_select_at_max_code_length(
    session: &SessionState,
    max_code_length: usize,
    delimiters: &str,
) -> bool {
    if max_code_length == 0 {
        return false;
    }
    let context = session.engine.context();
    let input = &context.composition.input;
    if input.len() < max_code_length || input.contains(|ch| delimiters.contains(ch)) {
        return false;
    }
    context
        .candidates
        .get(context.highlighted)
        .is_some_and(|candidate| candidate.source == CandidateSource::Table)
}

fn speller_previous_match_backup(
    session: &SessionState,
    auto_select: bool,
    max_code_length: usize,
    auto_select_pattern: Option<&Regex>,
) -> Option<(String, usize, yune_core::Candidate)> {
    if !auto_select || max_code_length > 0 || auto_select_pattern.is_some() {
        return None;
    }
    let context = session.engine.context();
    if !speller_context_has_menu(context) {
        return None;
    }
    let candidate = context.candidates.get(context.highlighted)?;
    (candidate.source == CandidateSource::Table).then(|| {
        (
            context.composition.input.clone(),
            context.highlighted,
            candidate.clone(),
        )
    })
}

fn speller_auto_select_previous_match(
    session: &mut SessionState,
    previous_match: Option<(String, usize, yune_core::Candidate)>,
    appended_input: &str,
    delimiters: &str,
) -> Option<String> {
    if !session.engine.get_option("_auto_commit")
        || speller_context_has_menu(session.engine.context())
    {
        return None;
    }
    let (previous_input, previous_highlighted, previous_candidate) = previous_match?;
    if previous_input.is_empty()
        || !appended_input.starts_with(&previous_input)
        || previous_input.contains(|ch| delimiters.contains(ch))
    {
        return None;
    }

    let rest = appended_input[previous_input.len()..].to_owned();
    session.engine.set_input(previous_input);
    let still_matches_previous = session
        .engine
        .context()
        .candidates
        .get(previous_highlighted)
        .is_some_and(|candidate| {
            candidate.source == previous_candidate.source
                && candidate.text == previous_candidate.text
        });
    if !still_matches_previous || !session.engine.highlight_candidate(previous_highlighted) {
        session.engine.set_input(appended_input.to_owned());
        return None;
    }
    let commit = session.engine.commit_composition();
    if commit.is_some() {
        session.engine.set_input(rest);
    } else {
        session.engine.set_input(appended_input.to_owned());
    }
    commit
}

fn speller_auto_select_unique_candidate(
    session: &mut SessionState,
    max_code_length: usize,
    auto_select_pattern: Option<&Regex>,
    delimiters: &str,
) -> Option<String> {
    let context = session.engine.context();
    let input = &context.composition.input;
    if input.is_empty() || input.contains(|ch| delimiters.contains(ch)) {
        return None;
    }
    let matches_auto_select_rule = if let Some(pattern) = auto_select_pattern {
        pattern
            .find(input)
            .is_some_and(|matched| matched.start() == 0 && matched.end() == input.len())
    } else {
        max_code_length == 0 || input.len() >= max_code_length
    };
    if !matches_auto_select_rule {
        return None;
    }
    let mut table_candidates = context
        .candidates
        .iter()
        .filter(|candidate| candidate.source == CandidateSource::Table);
    let _ = table_candidates.next()?;
    if table_candidates.next().is_some() {
        return None;
    }
    if context
        .candidates
        .iter()
        .filter(|candidate| candidate.source != CandidateSource::Echo)
        .count()
        != 1
    {
        return None;
    }
    session.engine.commit_composition()
}

fn speller_context_has_menu(context: &yune_core::Context) -> bool {
    context
        .candidates
        .iter()
        .any(|candidate| candidate.source != CandidateSource::Echo)
}

fn process_key_binder_processor(
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

fn update_key_binding_paging_state(
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

fn select_key_binding_radio_option(
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

fn process_punctuation_processor(
    session: &mut SessionState,
    key_event: KeyEvent,
) -> Option<PunctuationProcessResult> {
    if key_event.modifiers.control
        || key_event.modifiers.alt
        || key_event.modifiers.super_key
        || key_event.modifiers.release
        || session.engine.get_option("ascii_punct")
    {
        return None;
    }

    let KeyCode::Character(ch) = key_event.code else {
        return None;
    };
    if !ch.is_ascii() || ch.is_ascii_control() {
        return None;
    }

    if let Some(result) = process_pending_digit_separator(session, ch, &ch.to_string()) {
        return Some(result);
    }

    let use_space = session.punctuation_processor.as_ref()?.use_space;
    if ch == ' ' && !use_space && !session.engine.context().composition.input.is_empty() {
        return None;
    }

    let key = ch.to_string();
    if let Some(result) = process_digit_separator(session, ch, &key) {
        return Some(result);
    }

    if let Some(count) = active_alternating_punct_count(session, &key) {
        let highlighted = session.engine.context().highlighted;
        let next_index = (highlighted + 1) % count;
        session.engine.highlight_candidate(next_index);
        return Some(PunctuationProcessResult::Accepted);
    }

    if let Some(commit) = active_pair_commit(session, &key) {
        return Some(PunctuationProcessResult::Commit(commit));
    }

    let processor = session.punctuation_processor.as_ref()?;
    let shape_entries = if session.engine.status().is_full_shape {
        &processor.full_shape_unique_commits
    } else {
        &processor.half_shape_unique_commits
    };

    shape_entries
        .get(&key)
        .or_else(|| processor.symbol_unique_commits.get(&key))
        .cloned()
        .map(PunctuationProcessResult::Commit)
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
    let (shape_unique_commits, shape_alternating_counts, shape_pairs) =
        if session.engine.status().is_full_shape {
            (
                &processor.full_shape_unique_commits,
                &processor.full_shape_alternating_counts,
                &processor.full_shape_pairs,
            )
        } else {
            (
                &processor.half_shape_unique_commits,
                &processor.half_shape_alternating_counts,
                &processor.half_shape_pairs,
            )
        };

    shape_unique_commits.contains_key(key)
        || shape_alternating_counts.contains_key(key)
        || shape_pairs.contains_key(key)
        || processor.symbol_unique_commits.contains_key(key)
        || processor.symbol_alternating_counts.contains_key(key)
        || processor.symbol_pairs.contains_key(key)
}

pub(crate) fn ends_with_ascii_digit(text: &str) -> bool {
    text.as_bytes()
        .last()
        .is_some_and(|byte| byte.is_ascii_digit())
}

fn shape_formatted_ascii_text(text: &str, full_shape: bool) -> String {
    if !full_shape {
        return text.to_owned();
    }
    text.chars()
        .map(|ch| match ch {
            ' ' => '\u{3000}',
            '!'..='~' => char::from_u32(ch as u32 + 0xfee0)
                .expect("printable ASCII has a full-shape compatibility form"),
            _ => ch,
        })
        .collect()
}

fn active_pair_commit(session: &mut SessionState, key: &str) -> Option<String> {
    let processor = session.punctuation_processor.as_mut()?;
    let is_full_shape = session.engine.status().is_full_shape;
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
    let commit = pair[*oddness % 2].clone();
    *oddness = 1 - (*oddness % 2);
    Some(commit)
}

fn active_alternating_punct_count(session: &SessionState, key: &str) -> Option<usize> {
    let context = session.engine.context();
    if context.composition.input != key || context.candidates.is_empty() {
        return None;
    }

    let processor = session.punctuation_processor.as_ref()?;
    let shape_counts = if session.engine.status().is_full_shape {
        &processor.full_shape_alternating_counts
    } else {
        &processor.half_shape_alternating_counts
    };
    shape_counts
        .get(key)
        .or_else(|| processor.symbol_alternating_counts.get(key))
        .copied()
        .filter(|count| *count > 0)
        .map(|count| count.min(context.candidates.len()))
}

fn process_alternative_select_key(
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

    let menu_settings = context_menu_settings(&session.engine.status().schema_id);
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
    if config.is_null() {
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
    let resource_id = normalize_config_resource_id(config_id);
    let selected_path = selected_runtime_config_path(&resource_id, kind);

    selected_path
        .and_then(|path| fs::read_to_string(path).ok())
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
    let roots = runtime_config_roots(kind);
    roots
        .iter()
        .map(|root| config_file_path(root, resource_id))
        .find(|path| path.exists())
        .or_else(|| {
            roots
                .first()
                .map(|root| config_file_path(root, resource_id))
        })
}

pub(crate) fn selected_runtime_data_path(file_name: &str) -> Option<PathBuf> {
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    [
        paths.staging_dir.to_string_lossy().into_owned(),
        paths.prebuilt_data_dir.to_string_lossy().into_owned(),
        paths.shared_data_dir.to_string_lossy().into_owned(),
    ]
    .into_iter()
    .map(|root| Path::new(&root).join(file_name))
    .find(|path| path.is_file())
}

fn normalize_config_resource_id(config_id: &str) -> String {
    config_id
        .strip_suffix(".yaml")
        .unwrap_or(config_id)
        .to_owned()
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
