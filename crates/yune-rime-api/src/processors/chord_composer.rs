use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde_yaml::Value;
use yune_core::{parse_key_sequence, KeyCode, KeyEvent, KeyModifiers};

use crate::{
    config_scalar_bool, config_scalar_string, find_config_value, load_runtime_config_root,
    parse_single_key_binding_event, schema_engine_processors_include, schema_string_list,
    ConfigOpenKind, SessionKeyProcessResult, SessionState,
};

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

    pub(crate) fn has_binding(&self, key_event: &KeyEvent) -> bool {
        self.bindings.contains_key(key_event)
    }

    pub(crate) fn prompt(&self) -> Option<&str> {
        self.prompt.as_deref()
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

pub(crate) fn session_chord_composer_accepts_printable(
    session: &SessionState,
    key_event: KeyEvent,
) -> bool {
    let Some(composer) = session.chord_composer.as_ref() else {
        return false;
    };
    let KeyCode::Character(ch) = key_event.code else {
        return false;
    };
    composer.alphabet.contains(&ch)
        && chord_composer_allows_modifiers(composer, key_event.modifiers)
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

pub(crate) fn sync_chord_composer_context_update(session: &mut SessionState) {
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

pub(crate) fn process_chord_composer_processor(
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
