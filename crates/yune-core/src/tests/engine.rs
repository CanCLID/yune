use crate::{Candidate, CandidateRanker, CandidateSource, Context, Engine, MockAiRanker, RerankResult, StaticTableTranslator, Translator};

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
                source: CandidateSource::Table,
                quality: 1.0,
            },
            Candidate {
                text: "呢".to_owned(),
                comment: "second-comment".to_owned(),
                source: CandidateSource::Table,
                quality: 1.0,
            },
        ]
    }
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

        fn try_rerank(
            &self,
            _context: &Context,
            _candidates: &[Candidate],
        ) -> RerankResult {
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
fn backspace_rebuilds_candidates() {
    let mut engine = Engine::new();

    engine.process_char('a');
    engine.process_char('b');
    engine.process_char('\u{8}');

    assert_eq!(engine.context().composition.input, "a");
    assert_eq!(engine.context().candidates[0].source, CandidateSource::Echo);
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
