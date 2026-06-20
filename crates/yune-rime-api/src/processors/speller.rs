use regex::Regex;
use yune_core::{Candidate, CandidateSource, KeyCode, KeyEvent};

use crate::{
    config_scalar_bool, config_scalar_int, config_scalar_string, find_config_value,
    load_runtime_config_root, schema_engine_processors_include, ConfigOpenKind, SessionState,
    SpellerAutoClear, SpellerProcessResult, SpellerProcessor,
};

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

pub(crate) fn process_speller_processor(
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
        auto_select || (!session.engine.get_option("_auto_commit") && ch != ' '),
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
        .is_some_and(|candidate| candidate.source.is_table_like())
}

fn speller_previous_match_backup(
    session: &SessionState,
    auto_select: bool,
    max_code_length: usize,
    auto_select_pattern: Option<&Regex>,
) -> Option<(String, usize, Candidate)> {
    if !auto_select || max_code_length > 0 || auto_select_pattern.is_some() {
        return None;
    }
    let context = session.engine.context();
    if !speller_context_has_menu(context) {
        return None;
    }
    let candidate = context.candidates.get(context.highlighted)?;
    candidate.source.is_table_like().then(|| {
        (
            context.composition.input.clone(),
            context.highlighted,
            candidate.clone(),
        )
    })
}

fn speller_auto_select_previous_match(
    session: &mut SessionState,
    previous_match: Option<(String, usize, Candidate)>,
    appended_input: &str,
    delimiters: &str,
) -> Option<String> {
    if speller_context_has_menu(session.engine.context()) {
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
    if session.engine.get_option("_auto_commit") {
        let commit = session.engine.commit_composition();
        if commit.is_some() {
            session.engine.set_input(rest);
        } else {
            session.engine.set_input(appended_input.to_owned());
        }
        return commit;
    }

    session.engine.set_input(rest);
    None
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
        .filter(|candidate| candidate.source.is_table_like());
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
