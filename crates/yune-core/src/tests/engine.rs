use crate::{
    AiConfidence, AiContext, AiOffReason, AiResult, Candidate, CandidateRanker, CandidateSource,
    Context, Engine, MockAiRanker, RerankResult, StaticTableTranslator, Translator, UserDb,
};

struct CommentTranslator;

impl Translator for CommentTranslator {
    fn name(&self) -> &'static str {
        "comment_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        if input != "ni" {
            return Vec::new();
        }
        vec![
            Candidate {
                text: "你".to_owned(),
                comment: "first-comment".to_owned(),
                preedit: None,
                source: CandidateSource::Table,
                quality: 1.0,
            },
            Candidate {
                text: "呢".to_owned(),
                comment: "second-comment".to_owned(),
                preedit: None,
                source: CandidateSource::Table,
                quality: 1.0,
            },
        ]
    }
}

fn ai_candidate(text: &str) -> Candidate {
    ai_candidate_with_confidence(text, 0.62)
}

fn ai_candidate_with_confidence(text: &str, confidence: f32) -> Candidate {
    Candidate {
        text: text.to_owned(),
        comment: format!("ai:mock {confidence:.2}"),
        preedit: None,
        source: CandidateSource::ai("mock", AiConfidence::from_score(confidence)),
        quality: 0.0,
    }
}

fn visible_ai_candidate_index(engine: &Engine) -> usize {
    engine
        .context()
        .candidates
        .iter()
        .position(|candidate| candidate.source.is_ai())
        .expect("AI candidate should be visible")
}

#[test]
fn commits_table_candidate_before_echo_candidate() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.process_char('n');
    engine.process_char('i');

    assert_eq!(engine.context().composition.preedit, "ni");
    assert_eq!(engine.context().candidates[0].text, "你");
    assert_eq!(engine.context().candidates[1].text, "ni");

    let commit = engine.process_char(' ');
    assert_eq!(commit.as_deref(), Some("你"));
}

#[test]
fn numeric_selection_commits_candidate_on_current_page() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("ba2")
        .expect("key sequence should parse");

    assert_eq!(commits, ["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);
}

#[test]
fn keypad_numeric_selection_matches_librime_selector_without_text_input() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("{KP_1}ba{KP_2}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);
}

#[test]
fn shift_keypad_numeric_selection_matches_librime_selector() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("{Shift+KP_2}ba{Shift+KP_2}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);
}

#[test]
fn shift_ascii_numeric_selection_matches_librime_selector() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("ba{Shift+2}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);
}

#[test]
fn control_ascii_numeric_selection_matches_librime_selector() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("{Control+2}ba{Control+2}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);
}

#[test]
fn control_keypad_numeric_selection_matches_librime_selector() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("{Control+KP_2}ba{Control+KP_2}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);
}

#[test]
fn control_shift_numeric_selection_matches_librime_selector_digit_fallback() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("{Control+Shift+2}{Control+Shift+KP_2}ba{Control+Shift+2}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);

    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("ba{Control+Shift+KP_2}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);
}

#[test]
fn prediction_never_first_keeps_learned_prefix_prediction_behind_table_top() {
    let mut userdb = UserDb::default();
    userdb.learn_entry("ni hao", "LEARNED", 2, 2.0, 2);

    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "EXACT")]));
    engine.set_userdb(userdb);
    engine.set_prediction_never_first(true);
    engine.set_input("ni");

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(texts, ["EXACT", "LEARNED", "ni"]);
    assert_eq!(
        engine.context().candidates[1].source,
        CandidateSource::UserTable
    );
}

#[test]
fn bounded_refresh_expands_to_full_list_when_paging_past_window() {
    let dictionary = bounded_refresh_dictionary();
    let mut engine = Engine::new();
    engine.set_schema("luna_pinyin", "Luna Pinyin");
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(&dictionary)
            .expect("dictionary should parse")
            .with_completion(true)
            .with_sentence(false),
    );

    engine.set_input("n");

    let bounded_len = engine.context().candidates.len();
    assert!(
        bounded_len < 31,
        "initial refresh should retain a bounded window"
    );
    assert!(!engine.snapshot().candidate_list_complete);
    assert_eq!(engine.context().candidates[0].text, "candidate-00");

    assert!(engine.highlight_candidate(25));
    assert!(
        engine.context().candidates.len() > bounded_len,
        "out-of-window access should force a complete refresh"
    );
    assert!(engine.snapshot().candidate_list_complete);
    assert_eq!(engine.context().highlighted, 25);
    assert_eq!(engine.context().candidates[25].text, "candidate-25");
}

#[test]
fn page_snapshot_clones_only_visible_candidates_for_page_reads() {
    let dictionary = bounded_refresh_dictionary();
    let mut engine = Engine::new();
    engine.set_schema("luna_pinyin", "Luna Pinyin");
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(&dictionary)
            .expect("dictionary should parse")
            .with_completion(true)
            .with_sentence(false),
    );

    engine.set_input("n");

    let retained_len = engine.context().candidates.len();
    assert!(retained_len > 5, "test needs a retained surplus window");

    let page = engine.page_snapshot(5);

    assert_eq!(page.context.candidates.len(), 5);
    assert_eq!(page.context.candidates[0].text, "candidate-00");
    assert_eq!(page.context.highlighted, 0);
    assert!(!page.candidate_list_complete);
    assert_eq!(
        engine.context().candidates.len(),
        retained_len,
        "page snapshot must not force full-list expansion"
    );
}

#[test]
fn typeduck_product_refresh_keeps_ordinary_rows_bounded_until_full_access() {
    let dictionary = bounded_refresh_dictionary();
    let mut engine = Engine::new();
    engine.set_schema("jyut6ping3_mobile", "Jyutping");
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(&dictionary)
            .expect("dictionary should parse")
            .with_completion(true)
            .with_sentence(false)
            .with_prediction_candidate_limit(1)
            .with_prefix_fallback(true),
    );

    engine.set_input("n");

    let bounded_len = engine.context().candidates.len();
    assert!(
        bounded_len < 31,
        "product refresh should retain a bounded ordinary window"
    );
    assert!(!engine.snapshot().candidate_list_complete);
    assert_eq!(engine.context().candidates[0].text, "candidate-00");

    assert!(engine.highlight_candidate(25));
    assert!(
        engine.context().candidates.len() > bounded_len,
        "out-of-window product access should force a complete refresh"
    );
    assert!(engine.snapshot().candidate_list_complete);
}

#[test]
fn bounded_refresh_completes_before_candidate_deletion() {
    let dictionary = bounded_refresh_dictionary();
    let mut engine = Engine::new();
    engine.set_schema("luna_pinyin", "Luna Pinyin");
    engine.add_translator(
        StaticTableTranslator::parse_rime_dict_yaml(&dictionary)
            .expect("dictionary should parse")
            .with_completion(true)
            .with_sentence(false),
    );

    engine.set_input("n");

    let bounded_len = engine.context().candidates.len();
    assert!(
        bounded_len < 31,
        "initial refresh should retain a bounded window"
    );
    assert!(engine.delete_candidate(0));
    assert!(engine.snapshot().candidate_list_complete);
    assert!(!engine
        .context()
        .candidates
        .iter()
        .any(|candidate| candidate.text == "candidate-00"));

    assert!(engine.highlight_candidate(24));
    assert!(!engine
        .context()
        .candidates
        .iter()
        .any(|candidate| candidate.text == "candidate-00"));
}

fn bounded_refresh_dictionary() -> String {
    let mut dictionary = String::from(
        r#"
---
name: sample
version: "0.1"
sort: by_weight
...

"#,
    );
    for index in 0..30 {
        let first = char::from(b'a' + (index / 26) as u8);
        let second = char::from(b'a' + (index % 26) as u8);
        dictionary.push_str(&format!(
            "candidate-{index:02}\tn{first}{second}\t{}\n",
            100.0 - index as f32
        ));
    }
    dictionary
}

#[test]
fn escape_clears_composition_like_librime_editor_cancel() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    let commits = engine
        .process_key_sequence("ni{Escape}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert!(engine.context().composition.input.is_empty());
    assert!(engine.context().candidates.is_empty());
    assert_eq!(engine.context().last_commit, None);
    assert!(!engine.status().is_composing);
}

#[test]
fn shift_escape_ignores_shift_like_librime_editor_cancel_fallback() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    let commits = engine
        .process_key_sequence("ni{Shift+Escape}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert!(engine.context().composition.input.is_empty());
    assert!(engine.context().candidates.is_empty());
    assert_eq!(engine.context().last_commit, None);
    assert!(!engine.status().is_composing);
}

#[test]
fn delete_key_removes_input_at_caret_like_librime_editor_delete() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.set_input("nix");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["你"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    assert!(!engine.status().is_composing);

    engine.set_input("ni");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{Delete}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.input, "ni");
    assert_eq!(engine.context().composition.caret, 2);
}

#[test]
fn backspace_removes_input_before_caret_like_librime_editor_back() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.set_input("nxi");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["你"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    assert!(!engine.status().is_composing);

    engine.set_input("ni");
    engine.set_caret_pos(0);
    let commits = engine
        .process_key_sequence("{BackSpace}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.input, "ni");
    assert_eq!(engine.context().composition.caret, 0);
}

#[test]
fn deletion_clamps_caret_to_utf8_boundaries() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("n", "你")]));

    engine.set_input("é你n");
    engine.set_caret_pos(1);
    assert_eq!(engine.context().composition.caret, 0);
    engine.delete_input();
    assert_eq!(engine.context().composition.input, "你n");
    assert_eq!(engine.context().composition.caret, 0);

    engine.set_caret_pos(1);
    assert_eq!(engine.context().composition.caret, 0);
    engine.move_caret_right();
    assert_eq!(engine.context().composition.caret, "你".len());
    engine.back_to_previous_input();
    assert_eq!(engine.context().composition.input, "n");
    assert_eq!(engine.context().composition.caret, 0);
}

#[test]
fn control_backspace_falls_back_to_previous_input_like_librime_editor_back_syllable() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.set_input("nxi");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{Control+BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["你"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    assert!(!engine.status().is_composing);
}

#[test]
fn shift_backspace_uses_librime_editor_shift_as_control_fallback() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.set_input("nxi");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{Shift+BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["你"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    assert!(!engine.status().is_composing);
}

#[test]
fn control_return_commits_raw_input_like_librime_fluid_editor() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    let commits = engine
        .process_key_sequence("ni{Control+Return}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ni"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));
    assert!(!engine.status().is_composing);

    let commits = engine
        .process_key_sequence("{Control+Return}")
        .expect("key sequence should parse");
    assert!(commits.is_empty());
}

#[test]
fn shift_return_commits_script_text_like_librime_fluid_editor() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    let commits = engine
        .process_key_sequence("ni{Shift+Return}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ni"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));
    assert!(!engine.status().is_composing);

    let commits = engine
        .process_key_sequence("{Shift+Return}")
        .expect("key sequence should parse");
    assert!(commits.is_empty());
}

#[test]
fn shift_printable_keys_enter_input_and_shift_space_confirms_like_librime_editor() {
    let mut engine = Engine::new();

    let commits = engine
        .process_key_sequence("{Shift+A}b{Shift+space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["Ab"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("Ab"));
    assert!(!engine.status().is_composing);

    let commits = engine
        .process_key_sequence("{Shift+space}")
        .expect("key sequence should parse");
    assert!(commits.is_empty());
    assert_eq!(engine.context().last_commit.as_deref(), Some("Ab"));
}

#[test]
fn modified_keypad_enter_does_not_trigger_librime_return_only_editor_bindings() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    let commits = engine
        .process_key_sequence(
            "ni{Control+KP_Enter}{Shift+KP_Enter}{Control+Shift+KP_Enter}{KP_Enter}",
        )
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["你"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
}

#[test]
fn control_shift_return_commits_selected_comment_like_librime_fluid_editor() {
    let mut engine = Engine::new();
    engine.add_translator(CommentTranslator);

    let commits = engine
        .process_key_sequence("ni{Down}{Control+Shift+Return}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["second-comment"]);
    assert_eq!(
        engine.context().last_commit.as_deref(),
        Some("second-comment")
    );
    assert!(!engine.status().is_composing);

    let commits = engine
        .process_key_sequence("{Control+Shift+Return}")
        .expect("key sequence should parse");
    assert!(commits.is_empty());
}

#[test]
fn left_right_keys_move_caret_like_librime_navigator() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    let commits = engine
        .process_key_sequence("nix{Left}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["你"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    assert!(!engine.status().is_composing);

    engine.set_input("nix");
    engine.set_caret_pos(0);
    let commits = engine
        .process_key_sequence("{Right}{Right}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["你"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
}

#[test]
fn control_left_right_jump_across_simplified_syllable_span_like_librime_navigator() {
    let mut engine = Engine::new();

    engine.set_input("nix");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{Control+Left}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.caret, 0);

    let commits = engine
        .process_key_sequence("{Control+Right}{BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ni"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

    engine.set_input("nix");
    let commits = engine
        .process_key_sequence("{Control+Left}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ix"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
}

#[test]
fn shift_left_right_fall_back_to_control_syllable_jump_like_librime_navigator() {
    let mut engine = Engine::new();

    engine.set_input("nix");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{Shift+Left}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.caret, 0);

    let commits = engine
        .process_key_sequence("{Shift+Right}{BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ni"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

    engine.set_input("nix");
    let commits = engine
        .process_key_sequence("{Shift+Left}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ix"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
}

#[test]
fn control_up_down_jump_across_simplified_syllable_span_like_librime_vertical_navigator() {
    let mut engine = Engine::new();

    engine.set_input("nix");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{Control+Up}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.caret, 0);

    let commits = engine
        .process_key_sequence("{Control+Down}{BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ni"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

    engine.set_input("nix");
    let commits = engine
        .process_key_sequence("{Control+Up}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ix"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
}

#[test]
fn shift_up_down_fall_back_to_control_syllable_jump_like_librime_navigator() {
    let mut engine = Engine::new();

    engine.set_input("nix");
    engine.set_caret_pos(2);
    let commits = engine
        .process_key_sequence("{Shift+Up}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.caret, 0);

    let commits = engine
        .process_key_sequence("{Shift+Down}{BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ni"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

    engine.set_input("nix");
    let commits = engine
        .process_key_sequence("{Shift+Up}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ix"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
}

#[test]
fn keypad_left_right_keys_move_caret_by_char_with_librime_navigator_looping() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.set_input("nix");
    engine.set_caret_pos(0);
    let commits = engine
        .process_key_sequence("{KP_Left}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.caret, 3);
    let commits = engine
        .process_key_sequence("{KP_Left}{Delete}{space}")
        .expect("key sequence should parse");
    assert_eq!(commits, vec!["你"]);

    engine.set_input("nix");
    engine.set_caret_pos(3);
    let commits = engine
        .process_key_sequence("{KP_Right}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ix"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
}

#[test]
fn shift_keypad_left_right_ignore_shift_like_librime_navigator() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.set_input("nix");
    engine.set_caret_pos(0);
    let commits = engine
        .process_key_sequence("{Shift+KP_Left}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.caret, 3);
    let commits = engine
        .process_key_sequence("{Shift+KP_Left}{Delete}{space}")
        .expect("key sequence should parse");
    assert_eq!(commits, vec!["你"]);

    engine.set_input("nix");
    engine.set_caret_pos(3);
    let commits = engine
        .process_key_sequence("{Shift+KP_Right}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ix"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
}

#[test]
fn shift_keypad_up_down_ignore_shift_like_librime_navigator() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.set_input("nix");
    engine.set_caret_pos(0);
    let commits = engine
        .process_key_sequence("{Shift+KP_Up}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.caret, 3);
    let commits = engine
        .process_key_sequence("{Shift+KP_Up}{Delete}{space}")
        .expect("key sequence should parse");
    assert_eq!(commits, vec!["你"]);

    engine.set_input("nix");
    engine.set_caret_pos(3);
    let commits = engine
        .process_key_sequence("{Shift+KP_Down}{Delete}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["ix"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
}

#[test]
fn page_keys_move_candidate_page_like_librime_selector() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ba", "八"),
        ("ba", "吧"),
        ("ba", "爸"),
        ("ba", "巴"),
        ("ba", "把"),
        ("ba", "拔"),
    ]));

    let commits = engine
        .process_key_sequence("{Page_Down}ba{Page_Down}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().highlighted, 5);
    assert_eq!(engine.context().candidates[5].text, "拔");

    engine
        .process_key_sequence("{KP_Page_Up}")
        .expect("key sequence should parse");

    assert_eq!(engine.context().highlighted, 0);
    assert_eq!(engine.context().last_commit, None);
}

#[test]
fn up_down_keys_move_candidate_highlight_like_librime_selector() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ba", "八"),
        ("ba", "吧"),
        ("ba", "爸"),
    ]));

    let commits = engine
        .process_key_sequence("{Down}ba{Down}{KP_Down}{KP_Up}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["吧"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);
}

#[test]
fn raw_segments_do_not_select_candidates_like_librime_selector() {
    let mut engine = Engine::new();

    assert_eq!(engine.process_char('a'), None);
    engine.set_segment_tags(["raw"]);
    assert_eq!(engine.process_char('1'), None);

    assert_eq!(engine.context().composition.input, "a1");
    assert_eq!(engine.context().last_commit, None);
}

#[test]
fn home_end_keys_reset_candidate_highlight_like_librime_selector() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ba", "八"),
        ("ba", "吧"),
        ("ba", "爸"),
    ]));

    let commits = engine
        .process_key_sequence("ba{Down}{Down}{Home}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["八"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("八"));

    let commits = engine
        .process_key_sequence("ba{Down}{KP_End}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["八"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("八"));
}

#[test]
fn home_end_keys_fall_back_to_librime_navigator_caret_movement() {
    let mut engine = Engine::new();

    let commits = engine
        .process_key_sequence("nix{Home}{Delete}{End}{BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["i"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("i"));
}

#[test]
fn shift_home_end_keys_ignore_shift_like_librime_navigator() {
    let mut engine = Engine::new();

    engine.set_input("nix");
    let commits = engine
        .process_key_sequence("{Shift+Home}{Delete}{Shift+KP_End}{BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, vec!["i"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("i"));

    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
    engine
        .process_key_sequence("ba{Down}{Shift+Home}")
        .expect("key sequence should parse");

    assert_eq!(engine.context().highlighted, 1);
    assert_eq!(engine.context().composition.caret, 0);
}

#[test]
fn direct_candidate_selection_commits_by_global_or_page_index() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    engine
        .process_key_sequence("ba")
        .expect("key sequence should parse");
    assert_eq!(engine.select_candidate(1).as_deref(), Some("吧"));
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
    assert!(!engine.status().is_composing);

    engine
        .process_key_sequence("ba")
        .expect("key sequence should parse");
    assert_eq!(
        engine.select_candidate_on_current_page(0).as_deref(),
        Some("八")
    );
    assert_eq!(engine.context().last_commit.as_deref(), Some("八"));
}

#[test]
fn direct_candidate_highlighting_moves_selection_without_committing() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ba", "八"),
        ("ba", "吧"),
        ("ba", "爸"),
        ("ba", "巴"),
        ("ba", "把"),
        ("ba", "拔"),
    ]));

    engine
        .process_key_sequence("ba")
        .expect("key sequence should parse");
    assert!(engine.highlight_candidate(1));
    assert_eq!(engine.context().highlighted, 1);
    assert_eq!(engine.context().last_commit, None);
    assert!(!engine.highlight_candidate(99));
    assert_eq!(engine.context().highlighted, 1);

    assert!(engine.change_page(false));
    assert_eq!(engine.context().highlighted, 6);
    assert!(!engine.change_page(false));
    assert_eq!(engine.context().highlighted, 6);
    assert!(engine.highlight_candidate_on_current_page(0));
    assert_eq!(engine.context().highlighted, 5);
    assert!(!engine.highlight_candidate_on_current_page(5));
    assert_eq!(engine.context().highlighted, 5);
    assert!(engine.change_page(true));
    assert_eq!(engine.context().highlighted, 0);
    assert!(!engine.change_page(true));
    assert_eq!(engine.context().highlighted, 0);

    assert_eq!(engine.commit_composition().as_deref(), Some("八"));
}

#[test]
fn direct_candidate_deletion_removes_menu_items_without_committing() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ba", "八"),
        ("ba", "吧"),
        ("ba", "爸"),
        ("ba", "巴"),
        ("ba", "把"),
        ("ba", "拔"),
    ]));

    engine
        .process_key_sequence("ba")
        .expect("key sequence should parse");
    assert!(engine.delete_candidate(1));
    assert_eq!(engine.context().candidates[1].text, "爸");
    assert_eq!(engine.context().last_commit, None);
    assert!(!engine.delete_candidate(99));

    assert!(engine.change_page(false));
    assert!(engine.delete_candidate_on_current_page(0));
    assert_eq!(
        engine
            .context()
            .candidates
            .last()
            .map(|candidate| candidate.text.as_str()),
        Some("拔")
    );
    assert!(!engine.delete_candidate_on_current_page(5));
}

#[test]
fn control_delete_removes_highlighted_candidate_like_librime_editor_delete_candidate() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ba", "八"),
        ("ba", "吧"),
        ("ba", "爸"),
    ]));

    let commits = engine
        .process_key_sequence("ba{Down}{Control+Delete}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().candidates.len(), 3);
    assert_eq!(engine.context().candidates[1].text, "爸");
    assert_eq!(engine.context().highlighted, 1);
    assert_eq!(engine.context().last_commit, None);
}

#[test]
fn shift_delete_removes_highlighted_candidate_like_librime_editor_shift_as_control_fallback() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([
        ("ba", "八"),
        ("ba", "吧"),
        ("ba", "爸"),
    ]));

    let commits = engine
        .process_key_sequence("ba{Down}{Shift+Delete}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().candidates.len(), 3);
    assert_eq!(engine.context().candidates[1].text, "爸");
    assert_eq!(engine.context().highlighted, 1);
    assert_eq!(engine.context().last_commit, None);
}

#[test]
fn numeric_selection_consumes_out_of_page_digit_without_extending_input() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

    let commits = engine
        .process_key_sequence("ba0")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.input, "ba");
    assert_eq!(engine.context().candidates.len(), 3);
}

#[test]
fn table_translator_can_commit_rime_dictionary_phrase_codes() {
    let mut engine = Engine::new();
    let translator = StaticTableTranslator::parse_rime_dict_yaml(
        r#"
---
name: sample
version: "0.1"
sort: by_weight
...

你	ni	1
你好	ni hao	10
"#,
    )
    .expect("dictionary should parse");
    engine.add_translator(translator);

    let commits = engine
        .process_key_sequence("nihao{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["你好"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你好"));
}

#[test]
fn explicit_composition_control_commits_or_clears_active_input() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine
        .process_key_sequence("ni")
        .expect("key sequence should parse");
    assert_eq!(engine.commit_composition().as_deref(), Some("你"));
    assert!(!engine.status().is_composing);
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));

    engine
        .process_key_sequence("hao")
        .expect("key sequence should parse");
    engine.clear_composition();
    assert!(!engine.status().is_composing);
    assert!(engine.context().candidates.is_empty());
    assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    assert_eq!(engine.commit_composition(), None);
}

#[test]
fn direct_input_control_rebuilds_candidates_and_clamps_caret() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    engine.set_input("ni");

    assert_eq!(engine.context().composition.input, "ni");
    assert_eq!(engine.context().composition.preedit, "ni");
    assert_eq!(engine.context().composition.caret, 2);
    assert_eq!(engine.context().candidates[0].text, "你");

    engine.set_caret_pos(1);
    assert_eq!(engine.context().composition.caret, 1);
    engine.set_caret_pos(10);
    assert_eq!(engine.context().composition.caret, 2);
}

#[test]
fn runtime_options_update_status_flags_and_preserve_custom_values() {
    let mut engine = Engine::new();

    assert!(!engine.get_option("ascii_mode"));
    engine.set_option("ascii_mode", true);
    engine.set_option("custom_toggle", true);

    let status = engine.status();
    assert!(status.is_ascii_mode);
    assert!(engine.get_option("ascii_mode"));
    assert!(engine.get_option("custom_toggle"));

    engine.set_option("ascii_mode", false);
    assert!(!engine.status().is_ascii_mode);
    assert!(!engine.get_option("ascii_mode"));
    assert!(!engine.get_option("unknown_toggle"));
}

#[test]
fn runtime_properties_store_session_strings() {
    let mut engine = Engine::new();

    assert_eq!(engine.get_property("client_app"), None);

    engine.set_property("client_app", "sample_console");
    engine.set_property("inline_preedit", "");

    assert_eq!(engine.get_property("client_app"), Some("sample_console"));
    assert_eq!(engine.get_property("inline_preedit"), Some(""));
}

#[test]
fn mock_ai_ranker_can_reorder_ready_candidates() {
    let mut engine = Engine::new();
    let translator = StaticTableTranslator::parse_rime_dict_yaml(
        r#"
---
name: sample
version: "0.1"
sort: by_weight
...

把	ba	100
吧	ba	50
八	ba	10
"#,
    )
    .expect("dictionary should parse");
    engine.add_translator(translator);
    engine.add_ranker(MockAiRanker::new(["吧"]));

    engine
        .process_key_sequence("ba")
        .expect("keys should parse");

    assert_eq!(engine.context().candidates[0].text, "吧");
    assert_eq!(engine.context().candidates[1].text, "把");
    assert_eq!(engine.context().candidates[2].text, "八");
}

#[test]
fn pending_ranker_keeps_classic_candidate_order() {
    struct PendingRanker;

    impl CandidateRanker for PendingRanker {
        fn name(&self) -> &'static str {
            "pending_ranker"
        }

        fn try_rerank(&self, _context: &Context, _candidates: &[Candidate]) -> RerankResult {
            RerankResult::Pending
        }
    }

    let mut engine = Engine::new();
    let translator = StaticTableTranslator::parse_rime_dict_yaml(
        r#"
---
name: sample
version: "0.1"
sort: by_weight
...

把	ba	100
吧	ba	50
"#,
    )
    .expect("dictionary should parse");
    engine.add_translator(translator);
    engine.add_ranker(PendingRanker);

    engine
        .process_key_sequence("ba")
        .expect("keys should parse");

    assert_eq!(engine.context().candidates[0].text, "把");
    assert_eq!(engine.context().candidates[1].text, "吧");
}

#[test]
fn staged_ai_result_inserts_after_classic_top_candidate_and_preserves_top_candidate() {
    let mut engine = Engine::new();
    let translator = StaticTableTranslator::parse_rime_dict_yaml(
        r#"
---
name: sample
version: "0.1"
sort: by_weight
...

把	ba	100
吧	ba	50
"#,
    )
    .expect("dictionary should parse");
    engine.add_translator(translator);
    engine.set_input("ba");
    let baseline = engine.context().candidates.clone();

    let decision = engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![ai_candidate("吧呀")],
    });

    let candidates = &engine.context().candidates;
    assert_eq!(decision.as_str(), "ready");
    assert_eq!(candidates[0], baseline[0]);
    assert_eq!(
        candidates
            .get(1)
            .expect("AI row should follow classic top candidate")
            .text,
        "吧呀"
    );
    assert_eq!(
        candidates
            .get(1)
            .expect("AI row should follow classic top candidate")
            .source
            .as_str(),
        "ai"
    );
    assert_eq!(candidates[2], baseline[1]);
}

#[test]
fn staged_ai_merge_orders_ai_candidates_by_confidence_after_classic_top_candidate() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "把")]));
    engine.set_input("ba");

    let decision = engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![
            ai_candidate_with_confidence("low", 0.20),
            ai_candidate_with_confidence("high", 0.95),
            ai_candidate_with_confidence("middle", 0.50),
        ],
    });

    let texts = engine
        .context()
        .candidates
        .iter()
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(decision.as_str(), "ready");
    assert_eq!(texts, ["把", "high", "middle", "low", "ba"]);
}

#[test]
fn stale_or_pending_ai_result_does_not_change_classic_candidates() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "把"), ("ba", "吧")]));
    engine.set_input("ba");
    let baseline = engine.context().candidates.clone();

    let stale_decision = engine.stage_ai_result(AiResult::Ready {
        for_input: "ni".to_owned(),
        candidates: vec![ai_candidate("你呀")],
    });

    assert_eq!(stale_decision.as_str(), "pending");
    assert_eq!(engine.context().candidates, baseline);

    let pending_decision = engine.stage_ai_result(AiResult::pending("ba"));

    assert_eq!(pending_decision.as_str(), "pending");
    assert_eq!(engine.context().candidates, baseline);
}

#[test]
fn off_ai_result_clears_matching_staged_candidates() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "把")]));
    engine.set_input("ba");
    engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![ai_candidate("吧呀")],
    });
    assert!(engine
        .context()
        .candidates
        .iter()
        .any(|candidate| candidate.source.is_ai()));

    let decision = engine.stage_ai_result(AiResult::off("ba", AiOffReason::Privacy));

    assert_eq!(decision.as_str(), "off");
    assert!(engine
        .context()
        .candidates
        .iter()
        .all(|candidate| !candidate.source.is_ai()));
}

#[test]
fn default_confirm_does_not_auto_commit_highlighted_ai_candidate() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "把")]));
    engine.set_input("ba");
    engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![ai_candidate("吧呀")],
    });
    let ai_index = engine
        .context()
        .candidates
        .iter()
        .position(|candidate| candidate.source.is_ai())
        .expect("AI candidate should be visible");
    assert!(engine.highlight_candidate(ai_index));

    let commits = engine
        .process_key_sequence("{space}")
        .expect("key sequence should parse");

    assert!(commits.is_empty());
    assert_eq!(engine.context().composition.input, "ba");
    assert_eq!(engine.context().last_commit, None);
    assert_eq!(engine.take_pending_userdb_learning(), None);
}

#[test]
fn explicit_ai_commit_skips_librime_userdb_learning_but_classic_commit_still_stages_it() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "把")]));
    engine.set_input("ba");
    engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![ai_candidate("吧呀")],
    });
    let ai_index = engine
        .context()
        .candidates
        .iter()
        .position(|candidate| candidate.source.is_ai())
        .expect("AI candidate should be visible");

    assert_eq!(engine.select_candidate(ai_index).as_deref(), Some("吧呀"));
    assert!(engine.userdb().entries().is_empty());
    assert_eq!(engine.take_pending_userdb_learning(), None);
    assert!(engine.ai_memory().entries().is_empty());
    assert_eq!(engine.context().last_commit.as_deref(), Some("吧呀"));
    assert_eq!(
        engine
            .context()
            .commit_history
            .last()
            .expect("AI commit should be recorded")
            .candidate_type,
        "ai"
    );

    let mut classic_engine = Engine::new();
    classic_engine.add_translator(StaticTableTranslator::new([("ba", "把")]));
    classic_engine.set_input("ba");

    assert_eq!(classic_engine.select_candidate(0).as_deref(), Some("把"));
    let event = classic_engine
        .take_pending_userdb_learning()
        .expect("classic commit should stage userdb learning");
    assert_eq!(event.candidate_source, CandidateSource::Table);
    assert_eq!(event.candidate_type, "table");
}

#[test]
fn explicit_ai_commit_records_memory_for_standard_context() {
    let mut engine = Engine::new();
    engine.set_ai_context(
        AiContext::standard()
            .with_app_id("sample_cli")
            .with_field_id("message")
            .with_preceding_text("hello"),
    );
    engine.add_translator(StaticTableTranslator::new([("ba", "\u{628a}")]));
    engine.set_input("ba");
    engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![ai_candidate_with_confidence("\u{5427}\u{5440}", 0.62)],
    });
    let ai_index = visible_ai_candidate_index(&engine);

    assert_eq!(
        engine.select_candidate(ai_index).as_deref(),
        Some("\u{5427}\u{5440}")
    );

    assert_eq!(engine.take_pending_userdb_learning(), None);
    assert!(engine.userdb().entries().is_empty());
    let entries = engine.ai_memory().entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].input, "ba");
    assert_eq!(entries[0].selected_text, "\u{5427}\u{5440}");
    assert_eq!(entries[0].provider, "mock");
    assert_eq!(entries[0].confidence, AiConfidence::from_score(0.62));
    assert_eq!(entries[0].app_id.as_deref(), Some("sample_cli"));
    assert_eq!(entries[0].field_id.as_deref(), Some("message"));
    assert_eq!(entries[0].preceding_text.as_deref(), Some("hello"));
}

#[test]
fn sensitive_ai_commit_does_not_write_memory_or_userdb() {
    let mut engine = Engine::new();
    engine.add_translator(StaticTableTranslator::new([("ba", "\u{628a}")]));
    engine.set_input("ba");
    engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![ai_candidate("\u{5427}\u{5440}")],
    });
    let ai_index = visible_ai_candidate_index(&engine);

    assert_eq!(
        engine.select_candidate(ai_index).as_deref(),
        Some("\u{5427}\u{5440}")
    );

    assert_eq!(engine.take_pending_userdb_learning(), None);
    assert!(engine.userdb().entries().is_empty());
    assert!(engine.ai_memory().entries().is_empty());
}

#[test]
fn disabled_ai_memory_skips_standard_context_ai_commits() {
    let mut engine = Engine::new();
    engine.set_ai_context(AiContext::standard());
    engine.set_ai_memory_enabled(false);
    engine.add_translator(StaticTableTranslator::new([("ba", "\u{628a}")]));
    engine.set_input("ba");
    engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![ai_candidate("\u{5427}\u{5440}")],
    });
    let ai_index = visible_ai_candidate_index(&engine);

    assert_eq!(
        engine.select_candidate(ai_index).as_deref(),
        Some("\u{5427}\u{5440}")
    );

    assert!(engine.ai_memory().entries().is_empty());
    engine.set_ai_memory_enabled(true);
    engine.set_input("ba");
    engine.stage_ai_result(AiResult::Ready {
        for_input: "ba".to_owned(),
        candidates: vec![ai_candidate("\u{5427}\u{5440}")],
    });
    let ai_index = visible_ai_candidate_index(&engine);
    assert_eq!(
        engine.select_candidate(ai_index).as_deref(),
        Some("\u{5427}\u{5440}")
    );
    assert_eq!(engine.ai_memory().entries().len(), 1);
    engine.clear_ai_memory();
    assert!(engine.ai_memory().entries().is_empty());
}

#[test]
fn backspace_rebuilds_candidates() {
    let mut engine = Engine::new();

    engine.process_char('a');
    engine.process_char('b');
    engine.process_char('\u{8}');

    assert_eq!(engine.context().composition.input, "a");
    assert_eq!(engine.context().candidates[0].source, CandidateSource::Echo);
}

#[test]
fn clear_translators_removes_default_echo_fallback() {
    let mut engine = Engine::new();

    engine.process_char('x');
    assert_eq!(engine.context().candidates[0].source, CandidateSource::Echo);

    engine.clear_translators();

    assert_eq!(engine.context().composition.input, "x");
    assert!(engine.context().candidates.is_empty());
}

#[test]
fn sequence_collects_commits_and_snapshot_status() {
    let mut engine = Engine::new();
    engine.set_schema("sample", "Sample");
    engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

    let commits = engine.process_sequence("ni ");
    let snapshot = engine.snapshot();

    assert_eq!(commits, ["你"]);
    assert_eq!(snapshot.context.last_commit.as_deref(), Some("你"));
    assert_eq!(snapshot.status.schema_id, "sample");
    assert!(!snapshot.status.is_composing);
}

#[test]
fn key_sequence_processes_named_backspace_and_space() {
    let mut engine = Engine::new();

    let commits = engine
        .process_key_sequence("ni{BackSpace}{space}")
        .expect("key sequence should parse");

    assert_eq!(commits, ["n"]);
    assert_eq!(engine.context().last_commit.as_deref(), Some("n"));
    assert!(!engine.status().is_composing);
}
