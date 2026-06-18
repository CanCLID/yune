use std::collections::HashMap;

mod ai;
mod comment_format;
mod dictionary;
mod engine;
mod filter;
mod key;
mod punctuation;
mod spelling_algebra;
mod state;
#[cfg(test)]
mod tests;
mod translator;
mod userdb;
pub use ai::{
    AiCandidateProvider, AiDecision, AiResult, AiWorker, MockAiProvider, StagedAiCandidates,
};
use comment_format::CommentFormat;
pub use dictionary::{
    parse_rime_prism_bin_metadata, parse_rime_prism_bin_payload, parse_rime_reverse_bin_dictionary,
    parse_rime_reverse_bin_metadata, parse_rime_table_bin_dictionary,
    parse_rime_table_bin_metadata, rime_checksum_bytes, rime_dict_rebuild_plan,
    rime_dict_source_checksum, rime_table_bin_dict_file_checksum, CodeCoords,
    DictionaryLookupRecord, RimeChecksumComputer, RimeCompiledMetadataError, RimeCorrectionEntry,
    RimeDictArtifactStatus, RimeDictRebuildError, RimeDictRebuildExecutionReport,
    RimeDictRebuildInput, RimeDictRebuildPlan, RimePrismBinMetadata, RimePrismBinParseError,
    RimePrismBinPayload, RimePrismChecksumMetadata, RimePrismSpellingDescriptor,
    RimeReverseBinMetadata, RimeReverseBinParseError, RimeTableBinMetadata, RimeTableBinParseError,
    RimeToleranceRule, TableDictionary, TableDictionaryAdvancedData, TableDictionaryParseError,
    TableEncoder, TableEncoderFormulaError, TableEncodingRule, TableEntry,
};
pub use engine::Engine;
pub use filter::{
    CharsetFilter, DictionaryLookupFilter, ReverseLookupFilter, SimplifierFilter, SingleCharFilter,
    TaggedFilter, UniquifierFilter,
};
pub use key::{parse_key_sequence, KeyCode, KeyEvent, KeyModifiers, KeySequenceParseError};
pub use punctuation::PunctuationTranslator;
pub use state::{
    AiConfidence, Candidate, CandidateSource, CommitRecord, Composition, Context, Snapshot, Status,
};
pub use translator::{
    EchoTranslator, FoldedSwitchOptions, HistoryTranslator, ReverseLookupTranslator,
    SchemaListTranslator, StaticTableTranslator, SwitchTranslator, SwitchTranslatorSwitch,
};
pub use userdb::{
    BackdatedScanPolicy, UserDb, UserDbCommitMetadata, UserDbLearnedEntry, UserDbLearningUpdate,
    UserDbLookupRequest, UserDbLookupResult, UserDbValue,
};

pub trait Translator: Send + Sync {
    fn name(&self) -> &'static str;

    fn translate(&self, input: &str) -> Vec<Candidate>;

    fn translate_with_status(&self, input: &str, _status: &Status) -> Vec<Candidate> {
        self.translate(input)
    }

    fn translate_with_state(
        &self,
        input: &str,
        status: &Status,
        _options: &HashMap<String, bool>,
    ) -> Vec<Candidate> {
        self.translate_with_status(input, status)
    }

    fn translate_with_context(
        &self,
        input: &str,
        status: &Status,
        options: &HashMap<String, bool>,
        _context: &Context,
    ) -> Vec<Candidate> {
        self.translate_with_state(input, status, options)
    }
}

pub trait CandidateRanker: Send + Sync {
    fn name(&self) -> &'static str;

    fn try_rerank(&self, context: &Context, candidates: &[Candidate]) -> RerankResult;
}

pub trait CandidateFilter: Send + Sync {
    fn name(&self) -> &'static str;

    fn apply(&self, candidates: &mut Vec<Candidate>);

    fn apply_with_options(
        &self,
        candidates: &mut Vec<Candidate>,
        _options: &HashMap<String, bool>,
    ) {
        self.apply(candidates);
    }

    fn apply_with_context(
        &self,
        candidates: &mut Vec<Candidate>,
        options: &HashMap<String, bool>,
        _context: &Context,
    ) {
        self.apply_with_options(candidates, options);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RerankResult {
    Pending,
    Ready(Vec<Candidate>),
}

pub struct MockAiRanker {
    preferred_texts: Vec<String>,
}

impl MockAiRanker {
    #[must_use]
    pub fn new(preferred_texts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            preferred_texts: preferred_texts.into_iter().map(Into::into).collect(),
        }
    }
}

impl CandidateRanker for MockAiRanker {
    fn name(&self) -> &'static str {
        "mock_ai_ranker"
    }

    fn try_rerank(&self, _context: &Context, candidates: &[Candidate]) -> RerankResult {
        if self.preferred_texts.is_empty() || candidates.is_empty() {
            return RerankResult::Pending;
        }

        let mut ranked = candidates.to_vec();
        ranked.sort_by_key(|candidate| {
            self.preferred_texts
                .iter()
                .position(|text| text == &candidate.text)
                .unwrap_or(self.preferred_texts.len())
        });
        RerankResult::Ready(ranked)
    }
}

#[cfg(test)]
mod facade_tests {
    use super::{
        parse_key_sequence, parse_rime_prism_bin_metadata, parse_rime_prism_bin_payload,
        parse_rime_reverse_bin_dictionary, parse_rime_reverse_bin_metadata,
        parse_rime_table_bin_dictionary, parse_rime_table_bin_metadata, rime_checksum_bytes,
        rime_dict_rebuild_plan, rime_dict_source_checksum, CodeCoords, KeyCode,
        RimeChecksumComputer, RimeCompiledMetadataError, RimeDictArtifactStatus,
        RimeDictRebuildError, RimeDictRebuildExecutionReport, RimeDictRebuildInput,
        RimeDictRebuildPlan, RimePrismBinMetadata, RimePrismBinParseError,
        RimePrismChecksumMetadata, RimeReverseBinMetadata, RimeReverseBinParseError,
        RimeTableBinMetadata, RimeTableBinParseError, TableDictionary, TableEncoder,
    };

    #[test]
    fn parses_librime_style_key_sequence_names() {
        let keys = parse_key_sequence(
            "zyx 123{Shift+space}ABC{Control+Alt+Return}{KP_Enter}{KP_2}{Tab}{Delete}{Escape}{Left}{Right}{KP_Left}{KP_Right}{Home}{KP_End}{Page_Down}{KP_Page_Up}{Down}{KP_Up}{Control+Up}{Control+Down}",
        )
        .expect("key sequence should parse");

        assert_eq!(keys.len(), 29);
        assert_eq!(keys[3].code, KeyCode::Character(' '));
        assert!(!keys[3].modifiers.shift);
        assert_eq!(keys[7].code, KeyCode::Character(' '));
        assert!(keys[7].modifiers.shift);
        assert_eq!(keys[11].code, KeyCode::Return);
        assert!(keys[11].modifiers.control);
        assert!(keys[11].modifiers.alt);
        assert_eq!(keys[12].code, KeyCode::KeypadEnter);
        assert_eq!(keys[13].code, KeyCode::KeypadDigit('2'));
        assert_eq!(keys[14].code, KeyCode::Tab);
        assert_eq!(keys[15].code, KeyCode::Delete);
        assert_eq!(keys[16].code, KeyCode::Escape);
        assert_eq!(keys[17].code, KeyCode::MoveCaretLeft);
        assert_eq!(keys[18].code, KeyCode::MoveCaretRight);
        assert_eq!(keys[19].code, KeyCode::MoveCaretLeftByChar);
        assert_eq!(keys[20].code, KeyCode::MoveCaretRightByChar);
        assert_eq!(keys[21].code, KeyCode::Home);
        assert_eq!(keys[22].code, KeyCode::End);
        assert_eq!(keys[23].code, KeyCode::NextPage);
        assert_eq!(keys[24].code, KeyCode::PreviousPage);
        assert_eq!(keys[25].code, KeyCode::NextCandidate);
        assert_eq!(keys[26].code, KeyCode::PreviousCandidate);
        assert_eq!(keys[27].code, KeyCode::MoveCaretLeftBySyllable);
        assert!(keys[27].modifiers.control);
        assert_eq!(keys[28].code, KeyCode::MoveCaretRightBySyllable);
        assert!(keys[28].modifiers.control);
    }

    #[test]
    fn parses_named_braces_for_literal_brace_keys() {
        let keys =
            parse_key_sequence("{braceleft}{braceright}").expect("key sequence should parse");

        assert_eq!(keys[0].code, KeyCode::Character('{'));
        assert_eq!(keys[1].code, KeyCode::Character('}'));
    }

    #[test]
    fn parses_librime_ascii_symbol_key_names_as_printable_characters() {
        let cases = [
            ("exclam", '!'),
            ("quotedbl", '"'),
            ("numbersign", '#'),
            ("dollar", '$'),
            ("percent", '%'),
            ("ampersand", '&'),
            ("apostrophe", '\''),
            ("quoteright", '\''),
            ("parenleft", '('),
            ("parenright", ')'),
            ("asterisk", '*'),
            ("plus", '+'),
            ("comma", ','),
            ("minus", '-'),
            ("period", '.'),
            ("slash", '/'),
            ("colon", ':'),
            ("semicolon", ';'),
            ("less", '<'),
            ("equal", '='),
            ("greater", '>'),
            ("question", '?'),
            ("at", '@'),
            ("bracketleft", '['),
            ("backslash", '\\'),
            ("bracketright", ']'),
            ("asciicircum", '^'),
            ("underscore", '_'),
            ("grave", '`'),
            ("quoteleft", '`'),
            ("braceleft", '{'),
            ("bar", '|'),
            ("braceright", '}'),
            ("asciitilde", '~'),
        ];
        let sequence = cases
            .iter()
            .map(|(name, _)| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), cases.len());
        for (key, (_, expected)) in keys.iter().zip(cases) {
            assert_eq!(key.code, KeyCode::Character(expected));
            assert!(key.modifiers.is_empty());
        }
    }

    #[test]
    fn parses_librime_known_noop_key_names() {
        let keys = parse_key_sequence(
            "{Linefeed}{Clear}{Pause}{Scroll_Lock}{Sys_Req}{Begin}{Select}{Print}{Execute}{Insert}{Undo}{Redo}{Menu}{Find}{Cancel}{Help}{Break}{Arabic_switch}{Greek_switch}{Hangul_switch}{Hebrew_switch}{ISO_Group_Shift}{Mode_switch}{kana_switch}{script_switch}{Num_Lock}{F1}{Alt+F4}{F12}{F13}{F35}{Shift_L}{Shift_R}{Control_L}{Control_R}{Caps_Lock}{Shift_Lock}{Meta_L}{Meta_R}{Alt_L}{Alt_R}{Super_L}{Super_R}{Hyper_L}{Release+Hyper_R}{ISO_Lock}{ISO_Level2_Latch}{ISO_Level3_Shift}{ISO_Level3_Latch}{ISO_Level3_Lock}{ISO_Group_Latch}{ISO_Group_Lock}{ISO_Next_Group}{ISO_Next_Group_Lock}{ISO_Prev_Group}{ISO_Prev_Group_Lock}{ISO_First_Group}{ISO_First_Group_Lock}{ISO_Last_Group}{ISO_Last_Group_Lock}{ISO_Left_Tab}{ISO_Move_Line_Up}{ISO_Move_Line_Down}{ISO_Partial_Line_Up}{ISO_Partial_Line_Down}{ISO_Partial_Space_Left}{ISO_Partial_Space_Right}{ISO_Set_Margin_Left}{ISO_Set_Margin_Right}{ISO_Release_Margin_Left}{ISO_Release_Margin_Right}{ISO_Release_Both_Margins}{ISO_Fast_Cursor_Left}{ISO_Fast_Cursor_Right}{ISO_Fast_Cursor_Up}{ISO_Fast_Cursor_Down}{ISO_Continuous_Underline}{ISO_Discontinuous_Underline}{ISO_Emphasize}{ISO_Center_Object}{Release+ISO_Enter}",
        )
        .expect("key sequence should parse");

        assert_eq!(keys.len(), 81);
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys.iter().enumerate().all(|(index, key)| index == 27
            || index == 44
            || index == 80
            || key.modifiers.is_empty()));
        assert!(keys[27].modifiers.alt);
        assert!(keys[44].modifiers.release);
        assert!(keys[80].modifiers.release);
    }

    #[test]
    fn parses_librime_xkb_noop_key_names() {
        let names = [
            "dead_grave",
            "dead_acute",
            "dead_circumflex",
            "dead_tilde",
            "dead_macron",
            "dead_breve",
            "dead_abovedot",
            "dead_diaeresis",
            "dead_abovering",
            "dead_doubleacute",
            "dead_caron",
            "dead_cedilla",
            "dead_ogonek",
            "dead_iota",
            "dead_voiced_sound",
            "dead_semivoiced_sound",
            "dead_belowdot",
            "dead_hook",
            "dead_horn",
            "AccessX_Enable",
            "AccessX_Feedback_Enable",
            "RepeatKeys_Enable",
            "SlowKeys_Enable",
            "BounceKeys_Enable",
            "StickyKeys_Enable",
            "MouseKeys_Enable",
            "MouseKeys_Accel_Enable",
            "Overlay1_Enable",
            "Overlay2_Enable",
            "AudibleBell_Enable",
            "First_Virtual_Screen",
            "Prev_Virtual_Screen",
            "Next_Virtual_Screen",
            "Last_Virtual_Screen",
            "Terminate_Server",
            "Pointer_Left",
            "Pointer_Right",
            "Pointer_Up",
            "Pointer_Down",
            "Pointer_UpLeft",
            "Pointer_UpRight",
            "Pointer_DownLeft",
            "Pointer_DownRight",
            "Pointer_Button_Dflt",
            "Pointer_Button1",
            "Pointer_Button2",
            "Pointer_Button3",
            "Pointer_Button4",
            "Pointer_Button5",
            "Pointer_DblClick_Dflt",
            "Pointer_DblClick1",
            "Pointer_DblClick2",
            "Pointer_DblClick3",
            "Pointer_DblClick4",
            "Pointer_DblClick5",
            "Pointer_Drag_Dflt",
            "Pointer_Drag1",
            "Pointer_Drag2",
            "Pointer_Drag3",
            "Pointer_Drag4",
            "Pointer_EnableKeys",
            "Pointer_Accelerate",
            "Pointer_DfltBtnNext",
            "Pointer_DfltBtnPrev",
            "Pointer_Drag5",
            "Release+Pointer_Drag5",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_input_method_noop_key_names() {
        let names = [
            "Multi_key",
            "Kanji",
            "Muhenkan",
            "Henkan",
            "Henkan_Mode",
            "Romaji",
            "Hiragana",
            "Katakana",
            "Hiragana_Katakana",
            "Zenkaku",
            "Hankaku",
            "Zenkaku_Hankaku",
            "Touroku",
            "Massyo",
            "Kana_Lock",
            "Kana_Shift",
            "Eisu_Shift",
            "Eisu_toggle",
            "Hangul",
            "Hangul_Start",
            "Hangul_End",
            "Hangul_Hanja",
            "Hangul_Jamo",
            "Hangul_Romaja",
            "Codeinput",
            "Hangul_Jeonja",
            "Hangul_Banja",
            "Hangul_PreHanja",
            "Hangul_PostHanja",
            "SingleCandidate",
            "MultipleCandidate",
            "PreviousCandidate",
            "Release+Hangul_Special",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_keypad_noop_key_names() {
        let names = [
            "KP_Space",
            "KP_Tab",
            "KP_F1",
            "KP_F2",
            "KP_F3",
            "KP_F4",
            "KP_Begin",
            "KP_Insert",
            "KP_Delete",
            "KP_Multiply",
            "KP_Add",
            "KP_Separator",
            "KP_Subtract",
            "KP_Decimal",
            "KP_Divide",
            "Release+KP_Equal",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_latin1_noop_key_names() {
        let names = [
            "nobreakspace",
            "exclamdown",
            "cent",
            "sterling",
            "currency",
            "yen",
            "brokenbar",
            "section",
            "diaeresis",
            "copyright",
            "ordfeminine",
            "guillemotleft",
            "notsign",
            "hyphen",
            "registered",
            "macron",
            "degree",
            "plusminus",
            "twosuperior",
            "threesuperior",
            "acute",
            "mu",
            "paragraph",
            "periodcentered",
            "cedilla",
            "onesuperior",
            "masculine",
            "guillemotright",
            "onequarter",
            "onehalf",
            "threequarters",
            "questiondown",
            "Agrave",
            "Aacute",
            "Acircumflex",
            "Atilde",
            "Adiaeresis",
            "Aring",
            "AE",
            "Ccedilla",
            "Egrave",
            "Eacute",
            "Ecircumflex",
            "Ediaeresis",
            "Igrave",
            "Iacute",
            "Icircumflex",
            "Idiaeresis",
            "ETH",
            "Eth",
            "Ntilde",
            "Ograve",
            "Oacute",
            "Ocircumflex",
            "Otilde",
            "Odiaeresis",
            "multiply",
            "Ooblique",
            "Ugrave",
            "Uacute",
            "Ucircumflex",
            "Udiaeresis",
            "Yacute",
            "THORN",
            "Thorn",
            "ssharp",
            "agrave",
            "aacute",
            "acircumflex",
            "atilde",
            "adiaeresis",
            "aring",
            "ae",
            "ccedilla",
            "egrave",
            "eacute",
            "ecircumflex",
            "ediaeresis",
            "igrave",
            "iacute",
            "icircumflex",
            "idiaeresis",
            "eth",
            "ntilde",
            "ograve",
            "oacute",
            "ocircumflex",
            "otilde",
            "odiaeresis",
            "division",
            "oslash",
            "ugrave",
            "uacute",
            "ucircumflex",
            "udiaeresis",
            "yacute",
            "thorn",
            "Release+ydiaeresis",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_latin2_noop_key_names() {
        let names = [
            "Aogonek",
            "breve",
            "Lstroke",
            "Lcaron",
            "Sacute",
            "Scaron",
            "Scedilla",
            "Tcaron",
            "Zacute",
            "Zcaron",
            "Zabovedot",
            "aogonek",
            "ogonek",
            "lstroke",
            "lcaron",
            "sacute",
            "caron",
            "scaron",
            "scedilla",
            "tcaron",
            "zacute",
            "doubleacute",
            "zcaron",
            "zabovedot",
            "Racute",
            "Abreve",
            "Lacute",
            "Cacute",
            "Ccaron",
            "Eogonek",
            "Ecaron",
            "Dcaron",
            "Dstroke",
            "Nacute",
            "Ncaron",
            "Odoubleacute",
            "Rcaron",
            "Uring",
            "Udoubleacute",
            "Tcedilla",
            "racute",
            "abreve",
            "lacute",
            "cacute",
            "ccaron",
            "eogonek",
            "ecaron",
            "dcaron",
            "dstroke",
            "nacute",
            "ncaron",
            "odoubleacute",
            "udoubleacute",
            "rcaron",
            "uring",
            "tcedilla",
            "Release+abovedot",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_latin3_noop_key_names() {
        let names = [
            "Hstroke",
            "Hcircumflex",
            "Iabovedot",
            "Gbreve",
            "Jcircumflex",
            "hstroke",
            "hcircumflex",
            "idotless",
            "gbreve",
            "jcircumflex",
            "Cabovedot",
            "Ccircumflex",
            "Gabovedot",
            "Gcircumflex",
            "Ubreve",
            "Scircumflex",
            "cabovedot",
            "ccircumflex",
            "gabovedot",
            "gcircumflex",
            "ubreve",
            "Release+scircumflex",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_latin4_noop_key_names() {
        let names = [
            "kappa",
            "kra",
            "Rcedilla",
            "Itilde",
            "Lcedilla",
            "Emacron",
            "Gcedilla",
            "Tslash",
            "rcedilla",
            "itilde",
            "lcedilla",
            "emacron",
            "gcedilla",
            "tslash",
            "ENG",
            "eng",
            "Amacron",
            "Iogonek",
            "Eabovedot",
            "Imacron",
            "Ncedilla",
            "Omacron",
            "Kcedilla",
            "Uogonek",
            "Utilde",
            "Umacron",
            "amacron",
            "iogonek",
            "eabovedot",
            "imacron",
            "ncedilla",
            "omacron",
            "kcedilla",
            "uogonek",
            "utilde",
            "Release+umacron",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_kana_noop_key_names() {
        let names = [
            "overline",
            "kana_fullstop",
            "kana_openingbracket",
            "kana_closingbracket",
            "kana_comma",
            "kana_conjunctive",
            "kana_middledot",
            "kana_WO",
            "kana_a",
            "kana_i",
            "kana_u",
            "kana_e",
            "kana_o",
            "kana_ya",
            "kana_yu",
            "kana_yo",
            "kana_tsu",
            "kana_tu",
            "prolongedsound",
            "kana_A",
            "kana_I",
            "kana_U",
            "kana_E",
            "kana_O",
            "kana_KA",
            "kana_KI",
            "kana_KU",
            "kana_KE",
            "kana_KO",
            "kana_SA",
            "kana_SHI",
            "kana_SU",
            "kana_SE",
            "kana_SO",
            "kana_TA",
            "kana_CHI",
            "kana_TI",
            "kana_TSU",
            "kana_TU",
            "kana_TE",
            "kana_TO",
            "kana_NA",
            "kana_NI",
            "kana_NU",
            "kana_NE",
            "kana_NO",
            "kana_HA",
            "kana_HI",
            "kana_FU",
            "kana_HU",
            "kana_HE",
            "kana_HO",
            "kana_MA",
            "kana_MI",
            "kana_MU",
            "kana_ME",
            "kana_MO",
            "kana_YA",
            "kana_YU",
            "kana_YO",
            "kana_RA",
            "kana_RI",
            "kana_RU",
            "kana_RE",
            "kana_RO",
            "kana_WA",
            "kana_N",
            "voicedsound",
            "Release+semivoicedsound",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_arabic_noop_key_names() {
        let names = [
            "Arabic_comma",
            "Arabic_semicolon",
            "Arabic_question_mark",
            "Arabic_hamza",
            "Arabic_maddaonalef",
            "Arabic_hamzaonalef",
            "Arabic_hamzaonwaw",
            "Arabic_hamzaunderalef",
            "Arabic_hamzaonyeh",
            "Arabic_alef",
            "Arabic_beh",
            "Arabic_tehmarbuta",
            "Arabic_teh",
            "Arabic_theh",
            "Arabic_jeem",
            "Arabic_hah",
            "Arabic_khah",
            "Arabic_dal",
            "Arabic_thal",
            "Arabic_ra",
            "Arabic_zain",
            "Arabic_seen",
            "Arabic_sheen",
            "Arabic_sad",
            "Arabic_dad",
            "Arabic_tah",
            "Arabic_zah",
            "Arabic_ain",
            "Arabic_ghain",
            "Arabic_tatweel",
            "Arabic_feh",
            "Arabic_qaf",
            "Arabic_kaf",
            "Arabic_lam",
            "Arabic_meem",
            "Arabic_noon",
            "Arabic_ha",
            "Arabic_heh",
            "Arabic_waw",
            "Arabic_alefmaksura",
            "Arabic_yeh",
            "Arabic_fathatan",
            "Arabic_dammatan",
            "Arabic_kasratan",
            "Arabic_fatha",
            "Arabic_damma",
            "Arabic_kasra",
            "Arabic_shadda",
            "Release+Arabic_sukun",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_cyrillic_noop_key_names() {
        let names = [
            "Serbian_dje",
            "Macedonia_gje",
            "Cyrillic_io",
            "Ukrainian_ie",
            "Ukranian_je",
            "Macedonia_dse",
            "Ukrainian_i",
            "Ukranian_i",
            "Ukrainian_yi",
            "Ukranian_yi",
            "Cyrillic_je",
            "Serbian_je",
            "Cyrillic_lje",
            "Serbian_lje",
            "Cyrillic_nje",
            "Serbian_nje",
            "Serbian_tshe",
            "Macedonia_kje",
            "Byelorussian_shortu",
            "Cyrillic_dzhe",
            "Serbian_dze",
            "numerosign",
            "Serbian_DJE",
            "Macedonia_GJE",
            "Cyrillic_IO",
            "Ukrainian_IE",
            "Ukranian_JE",
            "Macedonia_DSE",
            "Ukrainian_I",
            "Ukranian_I",
            "Ukrainian_YI",
            "Ukranian_YI",
            "Cyrillic_JE",
            "Serbian_JE",
            "Cyrillic_LJE",
            "Serbian_LJE",
            "Cyrillic_NJE",
            "Serbian_NJE",
            "Serbian_TSHE",
            "Macedonia_KJE",
            "Byelorussian_SHORTU",
            "Cyrillic_DZHE",
            "Serbian_DZE",
            "Cyrillic_yu",
            "Cyrillic_a",
            "Cyrillic_be",
            "Cyrillic_tse",
            "Cyrillic_de",
            "Cyrillic_ie",
            "Cyrillic_ef",
            "Cyrillic_ghe",
            "Cyrillic_ha",
            "Cyrillic_i",
            "Cyrillic_shorti",
            "Cyrillic_ka",
            "Cyrillic_el",
            "Cyrillic_em",
            "Cyrillic_en",
            "Cyrillic_o",
            "Cyrillic_pe",
            "Cyrillic_ya",
            "Cyrillic_er",
            "Cyrillic_es",
            "Cyrillic_te",
            "Cyrillic_u",
            "Cyrillic_zhe",
            "Cyrillic_ve",
            "Cyrillic_softsign",
            "Cyrillic_yeru",
            "Cyrillic_ze",
            "Cyrillic_sha",
            "Cyrillic_e",
            "Cyrillic_shcha",
            "Cyrillic_che",
            "Cyrillic_hardsign",
            "Cyrillic_YU",
            "Cyrillic_A",
            "Cyrillic_BE",
            "Cyrillic_TSE",
            "Cyrillic_DE",
            "Cyrillic_IE",
            "Cyrillic_EF",
            "Cyrillic_GHE",
            "Cyrillic_HA",
            "Cyrillic_I",
            "Cyrillic_SHORTI",
            "Cyrillic_KA",
            "Cyrillic_EL",
            "Cyrillic_EM",
            "Cyrillic_EN",
            "Cyrillic_O",
            "Cyrillic_PE",
            "Cyrillic_YA",
            "Cyrillic_ER",
            "Cyrillic_ES",
            "Cyrillic_TE",
            "Cyrillic_U",
            "Cyrillic_ZHE",
            "Cyrillic_VE",
            "Cyrillic_SOFTSIGN",
            "Cyrillic_YERU",
            "Cyrillic_ZE",
            "Cyrillic_SHA",
            "Cyrillic_E",
            "Cyrillic_SHCHA",
            "Cyrillic_CHE",
            "Release+Cyrillic_HARDSIGN",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_greek_noop_key_names() {
        let names = [
            "Greek_ALPHAaccent",
            "Greek_EPSILONaccent",
            "Greek_ETAaccent",
            "Greek_IOTAaccent",
            "Greek_IOTAdieresis",
            "Greek_IOTAdiaeresis",
            "Greek_OMICRONaccent",
            "Greek_UPSILONaccent",
            "Greek_UPSILONdieresis",
            "Greek_OMEGAaccent",
            "Greek_accentdieresis",
            "Greek_horizbar",
            "Greek_alphaaccent",
            "Greek_epsilonaccent",
            "Greek_etaaccent",
            "Greek_iotaaccent",
            "Greek_iotadieresis",
            "Greek_iotaaccentdieresis",
            "Greek_omicronaccent",
            "Greek_upsilonaccent",
            "Greek_upsilondieresis",
            "Greek_upsilonaccentdieresis",
            "Greek_omegaaccent",
            "Greek_ALPHA",
            "Greek_BETA",
            "Greek_GAMMA",
            "Greek_DELTA",
            "Greek_EPSILON",
            "Greek_ZETA",
            "Greek_ETA",
            "Greek_THETA",
            "Greek_IOTA",
            "Greek_KAPPA",
            "Greek_LAMBDA",
            "Greek_LAMDA",
            "Greek_MU",
            "Greek_NU",
            "Greek_XI",
            "Greek_OMICRON",
            "Greek_PI",
            "Greek_RHO",
            "Greek_SIGMA",
            "Greek_TAU",
            "Greek_UPSILON",
            "Greek_PHI",
            "Greek_CHI",
            "Greek_PSI",
            "Greek_OMEGA",
            "Greek_alpha",
            "Greek_beta",
            "Greek_gamma",
            "Greek_delta",
            "Greek_epsilon",
            "Greek_zeta",
            "Greek_eta",
            "Greek_theta",
            "Greek_iota",
            "Greek_kappa",
            "Greek_lambda",
            "Greek_lamda",
            "Greek_mu",
            "Greek_nu",
            "Greek_xi",
            "Greek_omicron",
            "Greek_pi",
            "Greek_rho",
            "Greek_sigma",
            "Greek_finalsmallsigma",
            "Greek_tau",
            "Greek_upsilon",
            "Greek_phi",
            "Greek_chi",
            "Greek_psi",
            "Release+Greek_omega",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_technical_noop_key_names() {
        let names = [
            "leftradical",
            "topleftradical",
            "horizconnector",
            "topintegral",
            "botintegral",
            "vertconnector",
            "topleftsqbracket",
            "botleftsqbracket",
            "toprightsqbracket",
            "botrightsqbracket",
            "topleftparens",
            "botleftparens",
            "toprightparens",
            "botrightparens",
            "leftmiddlecurlybrace",
            "rightmiddlecurlybrace",
            "topleftsummation",
            "botleftsummation",
            "topvertsummationconnector",
            "botvertsummationconnector",
            "toprightsummation",
            "botrightsummation",
            "rightmiddlesummation",
            "lessthanequal",
            "notequal",
            "greaterthanequal",
            "integral",
            "therefore",
            "variation",
            "infinity",
            "nabla",
            "approximate",
            "similarequal",
            "ifonlyif",
            "implies",
            "identical",
            "radical",
            "includedin",
            "includes",
            "intersection",
            "union",
            "logicaland",
            "logicalor",
            "partialderivative",
            "function",
            "leftarrow",
            "uparrow",
            "rightarrow",
            "downarrow",
            "blank",
            "soliddiamond",
            "checkerboard",
            "ht",
            "ff",
            "cr",
            "lf",
            "nl",
            "vt",
            "lowrightcorner",
            "uprightcorner",
            "upleftcorner",
            "lowleftcorner",
            "crossinglines",
            "horizlinescan1",
            "horizlinescan3",
            "horizlinescan5",
            "horizlinescan7",
            "horizlinescan9",
            "leftt",
            "rightt",
            "bott",
            "topt",
            "Release+vertbar",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_publishing_noop_key_names() {
        let names = [
            "emspace",
            "enspace",
            "em3space",
            "em4space",
            "digitspace",
            "punctspace",
            "thinspace",
            "hairspace",
            "emdash",
            "endash",
            "signifblank",
            "ellipsis",
            "doubbaselinedot",
            "onethird",
            "twothirds",
            "onefifth",
            "twofifths",
            "threefifths",
            "fourfifths",
            "onesixth",
            "fivesixths",
            "careof",
            "figdash",
            "leftanglebracket",
            "decimalpoint",
            "rightanglebracket",
            "marker",
            "oneeighth",
            "threeeighths",
            "fiveeighths",
            "seveneighths",
            "trademark",
            "signaturemark",
            "trademarkincircle",
            "leftopentriangle",
            "rightopentriangle",
            "emopencircle",
            "emopenrectangle",
            "leftsinglequotemark",
            "rightsinglequotemark",
            "leftdoublequotemark",
            "rightdoublequotemark",
            "prescription",
            "minutes",
            "seconds",
            "latincross",
            "hexagram",
            "filledrectbullet",
            "filledlefttribullet",
            "filledrighttribullet",
            "emfilledcircle",
            "emfilledrect",
            "enopencircbullet",
            "enopensquarebullet",
            "openrectbullet",
            "opentribulletup",
            "opentribulletdown",
            "openstar",
            "enfilledcircbullet",
            "enfilledsqbullet",
            "filledtribulletup",
            "filledtribulletdown",
            "leftpointer",
            "rightpointer",
            "club",
            "diamond",
            "heart",
            "maltesecross",
            "dagger",
            "doubledagger",
            "checkmark",
            "ballotcross",
            "musicalsharp",
            "musicalflat",
            "malesymbol",
            "femalesymbol",
            "telephone",
            "telephonerecorder",
            "phonographcopyright",
            "caret",
            "singlelowquotemark",
            "doublelowquotemark",
            "cursor",
            "leftcaret",
            "rightcaret",
            "downcaret",
            "upcaret",
            "overbar",
            "downtack",
            "upshoe",
            "downstile",
            "underbar",
            "jot",
            "quad",
            "uptack",
            "circle",
            "upstile",
            "downshoe",
            "rightshoe",
            "leftshoe",
            "lefttack",
            "Release+righttack",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_hebrew_noop_key_names() {
        let names = [
            "hebrew_doublelowline",
            "hebrew_aleph",
            "hebrew_bet",
            "hebrew_beth",
            "hebrew_gimel",
            "hebrew_gimmel",
            "hebrew_dalet",
            "hebrew_daleth",
            "hebrew_he",
            "hebrew_waw",
            "hebrew_zain",
            "hebrew_zayin",
            "hebrew_chet",
            "hebrew_het",
            "hebrew_tet",
            "hebrew_teth",
            "hebrew_yod",
            "hebrew_finalkaph",
            "hebrew_kaph",
            "hebrew_lamed",
            "hebrew_finalmem",
            "hebrew_mem",
            "hebrew_finalnun",
            "hebrew_nun",
            "hebrew_samech",
            "hebrew_samekh",
            "hebrew_ayin",
            "hebrew_finalpe",
            "hebrew_pe",
            "hebrew_finalzade",
            "hebrew_finalzadi",
            "hebrew_zade",
            "hebrew_zadi",
            "hebrew_kuf",
            "hebrew_qoph",
            "hebrew_resh",
            "hebrew_shin",
            "hebrew_taf",
            "Release+hebrew_taw",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn rime_checksum_computer_matches_librime_crc32_initial_remainder() {
        assert_eq!(rime_checksum_bytes(b"abc"), 0x359a_672f);

        let mut checksum = RimeChecksumComputer::new(0);
        checksum.process_bytes(b"ab");
        checksum.process_bytes(b"c");
        assert_eq!(checksum.checksum(), 0x359a_672f);

        let mut chained = RimeChecksumComputer::new(0x359a_672f);
        chained.process_bytes(b"def");
        assert_eq!(chained.checksum(), 0x050d_415e);
    }

    #[test]
    fn rime_dict_source_checksum_matches_librime_dict_compiler_ordering() {
        let checksum = rime_dict_source_checksum(
            0,
            [b"dict one\n".as_slice(), b"dict two\n".as_slice()],
            Some(b"vocab\n".as_slice()),
        );
        assert_eq!(checksum, 0x0300_9e82);

        let primary = rime_dict_source_checksum(0, [b"primary\n".as_slice()], None);
        let pack = rime_dict_source_checksum(primary, [b"pack\n".as_slice()], None);
        assert_eq!(pack, 0x9024_58b9);

        assert_eq!(
            rime_dict_source_checksum(
                0x1234_5678,
                std::iter::empty::<&[u8]>(),
                Some(b"ignored vocabulary\n".as_slice()),
            ),
            0x1234_5678
        );
    }

    #[test]
    fn rime_dict_rebuild_plan_marks_table_prism_reverse_and_report_statuses() {
        let input = RimeDictRebuildInput {
            source_available: true,
            source_dict_file_checksum: 0x1111_1111,
            pack_source_checksums: Vec::new(),
            schema_file_checksum: 0x2222_2222,
            table_dict_file_checksum: Some(0x1111_1111),
            prism: Some(RimePrismChecksumMetadata {
                dict_file_checksum: 0x1111_1111,
                schema_file_checksum: 0x2222_2222,
            }),
            reverse_dict_file_checksum: Some(0x1111_1111),
            prebuilt_table_available: false,
            prebuilt_prism_available: false,
            prebuilt_reverse_available: false,
            force_rebuild_table: false,
            force_rebuild_prism: false,
        };
        assert_eq!(
            rime_dict_rebuild_plan(input.clone()),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: false,
                rebuild_reverse: false,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::ReusedFresh,
                    prism: RimeDictArtifactStatus::ReusedFresh,
                    reverse: RimeDictArtifactStatus::ReusedFresh,
                },
            })
        );

        let changed_source = RimeDictRebuildInput {
            source_dict_file_checksum: 0x3333_3333,
            ..input.clone()
        };
        assert_eq!(
            rime_dict_rebuild_plan(changed_source),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x3333_3333,
                rebuild_table: true,
                rebuild_prism: true,
                rebuild_reverse: true,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::Rebuilt,
                    prism: RimeDictArtifactStatus::Rebuilt,
                    reverse: RimeDictArtifactStatus::Rebuilt,
                },
            })
        );

        let changed_schema = RimeDictRebuildInput {
            schema_file_checksum: 0x4444_4444,
            ..input.clone()
        };
        assert_eq!(
            rime_dict_rebuild_plan(changed_schema),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: true,
                rebuild_reverse: false,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::ReusedFresh,
                    prism: RimeDictArtifactStatus::Rebuilt,
                    reverse: RimeDictArtifactStatus::ReusedFresh,
                },
            })
        );

        let stale_reverse = RimeDictRebuildInput {
            reverse_dict_file_checksum: Some(0x5555_5555),
            ..input
        };
        assert_eq!(
            rime_dict_rebuild_plan(stale_reverse),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: true,
                rebuild_prism: false,
                rebuild_reverse: true,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::Rebuilt,
                    prism: RimeDictArtifactStatus::ReusedFresh,
                    reverse: RimeDictArtifactStatus::Rebuilt,
                },
            })
        );
    }

    #[test]
    fn rime_dict_rebuild_plan_reuses_prebuilt_when_source_is_missing() {
        let input = RimeDictRebuildInput {
            source_available: false,
            source_dict_file_checksum: 0,
            pack_source_checksums: Vec::new(),
            schema_file_checksum: 0x2222_2222,
            table_dict_file_checksum: Some(0x1111_1111),
            prism: Some(RimePrismChecksumMetadata {
                dict_file_checksum: 0x1111_1111,
                schema_file_checksum: 0x2222_2222,
            }),
            reverse_dict_file_checksum: Some(0x1111_1111),
            prebuilt_table_available: true,
            prebuilt_prism_available: true,
            prebuilt_reverse_available: true,
            force_rebuild_table: true,
            force_rebuild_prism: false,
        };
        assert_eq!(
            rime_dict_rebuild_plan(input.clone()),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: false,
                rebuild_reverse: false,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::ReusedPrebuilt,
                    prism: RimeDictArtifactStatus::ReusedPrebuilt,
                    reverse: RimeDictArtifactStatus::ReusedPrebuilt,
                },
            })
        );

        assert_eq!(
            rime_dict_rebuild_plan(RimeDictRebuildInput {
                table_dict_file_checksum: None,
                prebuilt_table_available: false,
                ..input
            }),
            Err(RimeDictRebuildError::MissingSourceAndCompiled)
        );
    }

    #[test]
    fn rime_dict_rebuild_plan_chains_pack_checksums_and_forced_flags() {
        let primary = rime_dict_source_checksum(0, [b"primary\n".as_slice()], None);
        let pack = rime_dict_source_checksum(primary, [b"pack\n".as_slice()], None);
        let input = RimeDictRebuildInput {
            source_available: true,
            source_dict_file_checksum: primary,
            pack_source_checksums: vec![pack],
            schema_file_checksum: 0x2222_2222,
            table_dict_file_checksum: Some(primary),
            prism: Some(RimePrismChecksumMetadata {
                dict_file_checksum: primary,
                schema_file_checksum: 0x2222_2222,
            }),
            reverse_dict_file_checksum: Some(primary),
            prebuilt_table_available: false,
            prebuilt_prism_available: false,
            prebuilt_reverse_available: false,
            force_rebuild_table: true,
            force_rebuild_prism: true,
        };

        assert_eq!(
            rime_dict_rebuild_plan(input),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: pack,
                rebuild_table: true,
                rebuild_prism: true,
                rebuild_reverse: true,
                report: RimeDictRebuildExecutionReport {
                    table: RimeDictArtifactStatus::Rebuilt,
                    prism: RimeDictArtifactStatus::Rebuilt,
                    reverse: RimeDictArtifactStatus::Rebuilt,
                },
            })
        );
    }

    #[test]
    fn parses_librime_compiled_table_prism_and_reverse_metadata() {
        let mut table = vec![0; 68];
        put_c_string(&mut table, 0, b"Rime::Table/4.0");
        put_u32_le(&mut table, 32, 0x1111_1111);
        put_u32_le(&mut table, 36, 7);
        put_u32_le(&mut table, 40, 11);
        put_u32_le(&mut table, 44, 0x40);
        put_u32_le(&mut table, 48, 0x44);
        put_u32_le(&mut table, 64, 13);
        assert_eq!(
            parse_rime_table_bin_metadata(&table),
            Ok(RimeTableBinMetadata {
                dict_file_checksum: 0x1111_1111,
                num_syllables: 7,
                num_entries: 11,
                string_table_size: 13,
            })
        );

        let mut prism = vec![0; 320];
        put_c_string(&mut prism, 0, b"Rime::Prism/4.0");
        put_u32_le(&mut prism, 32, 0x2222_2222);
        put_u32_le(&mut prism, 36, 0x3333_3333);
        put_u32_le(&mut prism, 40, 17);
        put_u32_le(&mut prism, 44, 19);
        put_u32_le(&mut prism, 48, 23);
        put_u32_le(&mut prism, 52, 0x50);
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Ok(RimePrismBinMetadata {
                dict_file_checksum: 0x2222_2222,
                schema_file_checksum: 0x3333_3333,
                num_syllables: 17,
                num_spellings: 19,
                double_array_size: 23,
            })
        );

        let mut reverse = vec![0; 64];
        put_c_string(&mut reverse, 0, b"Rime::Reverse/3.1");
        put_u32_le(&mut reverse, 32, 0x4444_4444);
        put_u32_le(&mut reverse, 52, 29);
        put_u32_le(&mut reverse, 60, 31);
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Ok(RimeReverseBinMetadata {
                dict_file_checksum: 0x4444_4444,
                key_trie_size: 29,
                value_trie_size: 31,
            })
        );
    }

    #[test]
    fn compiled_metadata_parser_matches_librime_load_rejection_cases() {
        let mut table = vec![0; 68];
        put_c_string(&mut table, 0, b"Rime::Table/3.0");
        assert_eq!(
            parse_rime_table_bin_metadata(&table),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut table, 0, b"Rime::Table/4.0");
        put_u32_le(&mut table, 44, 0x40);
        assert_eq!(
            parse_rime_table_bin_metadata(&table),
            Err(RimeCompiledMetadataError::MissingRequiredSection)
        );

        let mut prism = vec![0; 320];
        put_c_string(&mut prism, 0, b"Rime::Prism/3.9");
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut prism, 0, b"Rime::Prism/4.0");
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Err(RimeCompiledMetadataError::MissingRequiredSection)
        );

        let mut reverse = vec![0; 64];
        put_c_string(&mut reverse, 0, b"Rime::Reverse/2.9");
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut reverse, 0, b"Rime::Reverse/4.1");
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );

        let mut invalid = vec![0; 68];
        put_c_string(&mut invalid, 0, b"Rime::Wrong/4.0");
        assert_eq!(
            parse_rime_table_bin_metadata(&invalid),
            Err(RimeCompiledMetadataError::InvalidFormat)
        );
        assert_eq!(
            parse_rime_table_bin_metadata(&invalid[..20]),
            Err(RimeCompiledMetadataError::TooShort)
        );
    }

    fn put_c_string(bytes: &mut [u8], offset: usize, value: &[u8]) {
        bytes[offset..offset + value.len()].copy_from_slice(value);
    }

    fn put_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_i32_le(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_f32_le(bytes: &mut [u8], offset: usize, value: f32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_le_bytes());
    }

    fn put_f32_le_extend(bytes: &mut Vec<u8>, value: f32) {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }

    fn put_offset(bytes: &mut [u8], field_offset: usize, target: usize) {
        let raw = i32::try_from(target as isize - field_offset as isize)
            .expect("fixture offset should fit i32");
        put_i32_le(bytes, field_offset, raw);
    }

    fn append_c_string(bytes: &mut Vec<u8>, value: &str) -> usize {
        let offset = bytes.len();
        bytes.extend_from_slice(value.as_bytes());
        bytes.push(0);
        offset
    }

    fn compiled_table_fixture() -> Vec<u8> {
        let mut bytes = vec![0; 68];
        put_c_string(&mut bytes, 0, b"Rime::Table/4.0");
        put_u32_le(&mut bytes, 32, 0x1111_1111);
        put_u32_le(&mut bytes, 36, 1);
        put_u32_le(&mut bytes, 40, 2);
        let syllabary_offset = bytes.len();
        bytes.resize(syllabary_offset + 8, 0);
        put_u32_le(&mut bytes, syllabary_offset, 1);
        let code_offset = append_c_string(&mut bytes, "ba");
        put_offset(&mut bytes, syllabary_offset + 4, code_offset);
        let index_offset = bytes.len();
        bytes.resize(index_offset + 16, 0);
        put_u32_le(&mut bytes, index_offset, 1);
        put_u32_le(&mut bytes, index_offset + 4, 2);
        let entries_offset = bytes.len();
        bytes.resize(entries_offset + 16, 0);
        let ba_offset = append_c_string(&mut bytes, "八");
        let ba2_offset = append_c_string(&mut bytes, "爸");
        put_offset(&mut bytes, entries_offset, ba_offset);
        put_f32_le(&mut bytes, entries_offset + 4, 2.0);
        put_offset(&mut bytes, entries_offset + 8, ba2_offset);
        put_f32_le(&mut bytes, entries_offset + 12, 1.0);
        put_offset(&mut bytes, index_offset + 8, entries_offset);
        put_offset(&mut bytes, 44, syllabary_offset);
        put_offset(&mut bytes, 48, index_offset);
        bytes
    }

    fn compiled_prism_fixture() -> Vec<u8> {
        let mut bytes = vec![0; 320];
        put_c_string(&mut bytes, 0, b"Rime::Prism/4.0");
        put_u32_le(&mut bytes, 32, 0x2222_2222);
        put_u32_le(&mut bytes, 36, 0x3333_3333);
        put_u32_le(&mut bytes, 40, 1);
        put_u32_le(&mut bytes, 44, 1);
        let spelling_map_offset = bytes.len();
        bytes.resize(spelling_map_offset + 12, 0);
        put_u32_le(&mut bytes, spelling_map_offset, 1);
        put_u32_le(&mut bytes, spelling_map_offset + 4, 1);
        let descriptor_offset = bytes.len();
        bytes.resize(descriptor_offset + 16, 0);
        let tips_offset = append_c_string(&mut bytes, "tip");
        put_i32_le(&mut bytes, descriptor_offset, 7);
        put_i32_le(&mut bytes, descriptor_offset + 4, (1 << 30) | 2);
        put_f32_le(&mut bytes, descriptor_offset + 8, 0.5);
        put_offset(&mut bytes, descriptor_offset + 12, tips_offset);
        put_offset(&mut bytes, spelling_map_offset + 8, descriptor_offset);
        let correction_offset = bytes.len();
        bytes.extend_from_slice(b"YUNE-CORR\0");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "bq");
        put_len_string(&mut bytes, "ba");
        let tolerance_offset = bytes.len();
        bytes.extend_from_slice(b"YUNE-TOL\0");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "bz");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "ba");
        put_offset(&mut bytes, 56, spelling_map_offset);
        put_offset(&mut bytes, 60, correction_offset);
        put_offset(&mut bytes, 64, tolerance_offset);
        bytes
    }

    fn compiled_reverse_fixture() -> Vec<u8> {
        let mut bytes = vec![0; 64];
        put_c_string(&mut bytes, 0, b"Rime::Reverse/4.0");
        put_u32_le(&mut bytes, 32, 0x4444_4444);
        bytes.extend_from_slice(b"YUNE-REVERSE\0");
        put_u32_le_extend(&mut bytes, 2);
        put_len_string(&mut bytes, "ba");
        put_len_string(&mut bytes, "八");
        put_len_string(&mut bytes, "ba");
        put_len_string(&mut bytes, "爸");
        bytes
    }

    fn compiled_table_advanced_fixture() -> Vec<u8> {
        let mut bytes = compiled_table_fixture();
        bytes.extend_from_slice(b"YUNE-TABLE-ADV\0");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "明");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "a'b");
        put_u32_le_extend(&mut bytes, 2);
        put_len_string(&mut bytes, "您好");
        put_len_string(&mut bytes, "nh");
        put_f32_le_extend(&mut bytes, 11.0);
        put_len_string(&mut bytes, "你好");
        put_len_string(&mut bytes, "nh");
        put_f32_le_extend(&mut bytes, 6.0);
        put_u32_le_extend(&mut bytes, 1);
        put_u32_le_extend(&mut bytes, 2);
        put_len_string(&mut bytes, "AaBa");
        bytes
    }

    fn put_u32_le_extend(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn put_len_string(bytes: &mut Vec<u8>, value: &str) {
        put_u32_le_extend(bytes, value.len() as u32);
        bytes.extend_from_slice(value.as_bytes());
    }

    #[test]
    fn parses_compiled_table_fixture_into_dictionary_order() {
        let dictionary = parse_rime_table_bin_dictionary(compiled_table_fixture())
            .expect("compiled table should parse");
        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 2.0);
        assert_eq!(entries[1].text, "爸");
    }

    #[test]
    fn parses_compiled_table_advanced_payload_stems_vocabulary_and_encoder_entries() {
        let dictionary = parse_rime_table_bin_dictionary(compiled_table_advanced_fixture())
            .expect("advanced compiled table should parse");
        assert_eq!(dictionary.stems_for("明"), Some(&["a'b".to_owned()][..]));
        assert!(dictionary
            .entries()
            .iter()
            .any(|entry| entry.text == "您好" && entry.code == "nh" && entry.weight == 11.0));
        assert!(dictionary
            .entries()
            .iter()
            .any(|entry| entry.text == "你好" && entry.code == "nh"));
        assert!(dictionary.encoder().loaded());
    }

    #[test]
    fn parses_compiled_prism_fixture_payload() {
        let payload = parse_rime_prism_bin_payload(compiled_prism_fixture())
            .expect("compiled prism should parse");
        assert_eq!(payload.dict_file_checksum, 0x2222_2222);
        assert_eq!(payload.spelling_map.len(), 1);
        assert_eq!(payload.spelling_map[0][0].syllable_id, 7);
        assert_eq!(payload.spelling_map[0][0].spelling_type, 2);
        assert!(payload.spelling_map[0][0].is_correction);
        assert_eq!(payload.spelling_map[0][0].tips, "tip");
        assert_eq!(payload.corrections[0].observed_input, "bq");
        assert_eq!(payload.corrections[0].canonical_code, "ba");
        assert_eq!(payload.tolerance_rules[0].near_code, "bz");
        assert_eq!(
            payload.tolerance_rules[0].candidate_codes,
            ["ba".to_owned()]
        );
    }

    #[test]
    fn parses_compiled_reverse_fixture_into_dictionary() {
        let dictionary = parse_rime_reverse_bin_dictionary(compiled_reverse_fixture())
            .expect("compiled reverse should parse");
        let texts = dictionary
            .entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["八", "爸"]);
    }

    #[test]
    fn parses_compiled_reverse_dict_settings_and_stems() {
        let mut bytes = compiled_reverse_fixture();
        put_u32_le_extend(&mut bytes, 2);
        put_len_string(&mut bytes, "tail_anchor");
        put_len_string(&mut bytes, "'");
        put_len_string(&mut bytes, "rules/0/formula");
        put_len_string(&mut bytes, "AaBa");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "明");
        put_u32_le_extend(&mut bytes, 1);
        put_len_string(&mut bytes, "a'b");

        let dictionary = parse_rime_reverse_bin_dictionary(bytes)
            .expect("advanced compiled reverse should parse");
        assert_eq!(
            dictionary.dict_settings().get("tail_anchor"),
            Some(&"'".to_owned())
        );
        assert_eq!(
            dictionary.dict_settings().get("rules/0/formula"),
            Some(&"AaBa".to_owned())
        );
        assert_eq!(dictionary.stems_for("明"), Some(&["a'b".to_owned()][..]));
    }

    #[test]
    fn compiled_payload_readers_reject_malformed_bytes() {
        assert_eq!(
            parse_rime_table_bin_dictionary(&compiled_table_fixture()[..20]),
            Err(RimeTableBinParseError::TooShort)
        );
        let mut bad_version = compiled_table_fixture();
        put_c_string(&mut bad_version, 0, b"Rime::Table/3.0");
        assert_eq!(
            parse_rime_table_bin_dictionary(bad_version),
            Err(RimeTableBinParseError::UnsupportedVersion)
        );
        let mut missing_section = compiled_table_fixture();
        put_i32_le(&mut missing_section, 44, 0);
        assert_eq!(
            parse_rime_table_bin_dictionary(missing_section),
            Err(RimeTableBinParseError::MissingRequiredSection)
        );
        let mut bad_offset = compiled_table_fixture();
        put_i32_le(&mut bad_offset, 44, i32::MAX);
        assert_eq!(
            parse_rime_table_bin_dictionary(bad_offset),
            Err(RimeTableBinParseError::OutOfBounds)
        );
        let mut huge_count = compiled_table_fixture();
        let index_offset = 79;
        put_u32_le(&mut huge_count, index_offset, u32::MAX);
        assert_eq!(
            parse_rime_table_bin_dictionary(huge_count),
            Err(RimeTableBinParseError::InvalidCount)
        );
        let mut invalid_utf8 = compiled_table_fixture();
        let last = invalid_utf8.len() - 1;
        invalid_utf8[last - 1] = 0xff;
        assert_eq!(
            parse_rime_table_bin_dictionary(invalid_utf8),
            Err(RimeTableBinParseError::InvalidUtf8)
        );
        let mut unsupported = compiled_table_fixture();
        put_offset(&mut unsupported, 60, 68);
        assert!(matches!(
            parse_rime_table_bin_dictionary(unsupported),
            Err(RimeTableBinParseError::UnsupportedSection { .. })
        ));

        let mut prism_unsupported = compiled_prism_fixture();
        put_u32_le(&mut prism_unsupported, 48, 4);
        put_offset(&mut prism_unsupported, 52, 320);
        assert!(matches!(
            parse_rime_prism_bin_payload(prism_unsupported),
            Err(RimePrismBinParseError::UnsupportedSection { .. })
        ));

        let mut reverse_unsupported = compiled_reverse_fixture();
        put_u32_le(&mut reverse_unsupported, 52, 1);
        assert!(matches!(
            parse_rime_reverse_bin_dictionary(reverse_unsupported),
            Err(RimeReverseBinParseError::UnsupportedSection { .. })
        ));
    }

    #[test]
    fn parses_rime_dict_yaml_default_columns_and_weight_order() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

巴	ba	3193
爸	ba	3625
八	ba	6677
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "爸");
        assert_eq!(entries[2].text, "巴");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 6677.0);
    }

    #[test]
    fn parses_rime_dict_yaml_custom_columns_for_shape_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: cangjie_sample
version: "0.1"
sort: original
columns:
  - text
  - code
  - stem
...

明	ab
晭	abgr	ab'gr
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries[0].text, "明");
        assert_eq!(entries[0].code, "ab");
        assert_eq!(entries[1].text, "晭");
        assert_eq!(entries[1].code, "abgr");
        assert_eq!(
            dictionary.stems().get("晭").cloned(),
            Some(vec!["ab'gr".to_owned()])
        );
    }

    #[test]
    fn parses_rime_dict_yaml_stem_columns_like_librime_entry_collector() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stem_sample
version: "0.1"
sort: original
columns: [text, code, stem]
...

明	ab	a'b
明	ab	a'b
明	ac	a'c
未编码		ignored
"#,
        )
        .expect("dictionary should parse");

        assert_eq!(
            dictionary.stems().get("明").cloned(),
            Some(vec!["a'b".to_owned(), "a'c".to_owned()])
        );
        assert_eq!(
            dictionary.stems_for("明"),
            Some(&["a'b".to_owned(), "a'c".to_owned()][..])
        );
        assert!(!dictionary.stems().contains_key("未编码"));
        assert_eq!(dictionary.stems_for("未编码"), None);
    }

    #[test]
    fn parses_rime_dict_yaml_reverse_dict_settings_as_read_only_contract() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: reverse_settings_sample
version: "0.1"
dict_settings:
  use_rule_based_encoder: true
  tail_anchor: "'"
  rules:
    - length_equal: 2
      formula: "AaBa"
...

明	ab	1
"#,
        )
        .expect("dictionary should parse");

        assert_eq!(
            dictionary.dict_settings().get("use_rule_based_encoder"),
            Some(&"true".to_owned())
        );
        assert_eq!(
            dictionary.dict_settings().get("tail_anchor"),
            Some(&"'".to_owned())
        );
        assert_eq!(
            dictionary.dict_settings().get("rules/0/length_equal"),
            Some(&"2".to_owned())
        );
        assert_eq!(
            dictionary.dict_settings().get("rules/0/formula"),
            Some(&"AaBa".to_owned())
        );
    }

    #[test]
    fn parses_rime_dict_yaml_encoder_settings_like_librime_dict_settings() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: encoder_sample
version: "0.1"
sort: original
encoder:
  exclude_patterns:
    - '^x.*$'
  rules:
    - length_equal: 2
      formula: "AaAzBaBz"
    - length_in_range: [3, 5]
      formula: "AaBaCaZz"
  tail_anchor: "'"
...

甲	abc
乙	def
"#,
        )
        .expect("dictionary should parse");

        let encoder = dictionary.encoder();
        assert!(encoder.loaded());
        assert_eq!(encoder.max_phrase_length(), 5);
        assert_eq!(encoder.rules().len(), 2);
        assert_eq!(encoder.rules()[0].min_word_length, 2);
        assert_eq!(encoder.rules()[0].max_word_length, 2);
        assert_eq!(encoder.rules()[1].min_word_length, 3);
        assert_eq!(encoder.rules()[1].max_word_length, 5);
        assert!(encoder.is_code_excluded("xyz"));
        assert!(!encoder.is_code_excluded("axyz"));
        assert_eq!(
            encoder.encode(&["zyx'wvu'tsr", "qpo'nmlk'jih", "gfedcba"]),
            Some("zqga".to_owned())
        );
    }

    #[test]
    fn parses_rime_dict_yaml_rule_encoder_phrase_entries_like_librime_entry_collector() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            r#"
---
name: encoder_phrase_sample
version: "0.1"
sort: by_weight
use_preset_vocabulary: true
max_phrase_length: 2
min_phrase_weight: 10
encoder:
  rules:
    - length_equal: 2
      formula: "AaBa"
...

你	ni	10
好	hao	9
您	nin	8
你好		50%
"#,
            std::iter::empty::<&str>(),
            |_| None,
            |name| {
                (name == "essay").then(|| {
                    "\
你好\t12
您好\t11
你好啊\t20
低频\t9
"
                    .to_owned()
                })
            },
        )
        .expect("rule-based encoder phrases should parse");

        let entries = dictionary.entries();
        let encoded_source_phrase = entries
            .iter()
            .find(|entry| entry.text == "你好")
            .expect("source phrase should be encoded");
        assert_eq!(encoded_source_phrase.code, "nh");
        assert_eq!(encoded_source_phrase.weight, 6.0);
        assert!(!entries
            .iter()
            .any(|entry| entry.text == "你好" && entry.code.is_empty()));

        let injected_phrase = entries
            .iter()
            .find(|entry| entry.text == "您好")
            .expect("preset phrase should be injected when all characters are encodable");
        assert_eq!(injected_phrase.code, "nh");
        assert_eq!(injected_phrase.weight, 11.0);
        assert!(!entries.iter().any(|entry| entry.text == "你好啊"));
        assert!(!entries.iter().any(|entry| entry.text == "低频"));
    }

    #[test]
    fn table_encoder_parses_librime_formula_settings() {
        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAzBaBz")
            .expect("librime encoder formula should parse");
        encoder
            .add_length_equal_rule(3, "AaBaCaBz")
            .expect("librime encoder formula should parse");
        encoder
            .add_length_in_range_rule(4, 9, "AaBaCaZz")
            .expect("librime encoder formula should parse");

        assert!(encoder.loaded());
        assert_eq!(encoder.max_phrase_length(), 9);
        assert_eq!(encoder.rules().len(), 3);
        assert_eq!(encoder.rules()[0].min_word_length, 2);
        assert_eq!(encoder.rules()[0].max_word_length, 2);
        assert_eq!(
            encoder.rules()[0].coords,
            [
                CodeCoords {
                    char_index: 0,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 0,
                    code_index: -1
                },
                CodeCoords {
                    char_index: 1,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 1,
                    code_index: -1
                },
            ]
        );
        assert_eq!(
            encoder.rules()[2].coords,
            [
                CodeCoords {
                    char_index: 0,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 1,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 2,
                    code_index: 0
                },
                CodeCoords {
                    char_index: -1,
                    code_index: -1
                },
            ]
        );
    }

    #[test]
    fn table_encoder_matches_librime_raw_code_encoding_cases() {
        let code2 = ["abc", "def"];
        let code3 = ["abc", "def", "ghi"];

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAbBaBb")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("abde".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_in_range_rule(3, 5, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code3), Some("acdfgi".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("acdf".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAbZyZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("abef".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAaBbBbZzZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("aaeeff".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_in_range_rule(3, 5, "AzAzByByZaZa")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code3), Some("cceegg".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaBaYaZaZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("adf".to_owned()));
    }

    #[test]
    fn table_encoder_honors_librime_exclude_patterns_and_tail_anchor() {
        let mut encoder = TableEncoder::new();
        encoder
            .set_exclude_patterns(["^x.*$"])
            .expect("exclude regex should compile");
        assert!(encoder.is_code_excluded("x"));
        assert!(encoder.is_code_excluded("xyz"));
        assert!(!encoder.is_code_excluded("XYZ"));
        assert!(!encoder.is_code_excluded("ax"));

        let code = ["zyx'wvu'tsr", "qpo'nmlk'jih", "gfedcba"];

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zxqoga".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAbAcAzBwBxByBz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zyxuqpo".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAbAcAdAzBaBxByBz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zyxwuqpo".to_owned()));
    }

    #[test]
    fn parses_rime_dict_yaml_inline_custom_columns() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_columns_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	八	10
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].code, "ba");
        assert_eq!(entries[1].text, "吧");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_header_scalars() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: quoted_header_sample
version: "0.1"
sort: 'original'
columns:
  - 'code'
  - "text"
  - 'weight'
...

ba	八	1
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, 9.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_scalars_with_comments() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: commented_header_sample
version: "0.1" # dictionary version
sort: original # preserve source order
columns:
  - code # input code
  - text # candidate text
  - weight # absolute weight
...

ba	八	1
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, 9.0);
    }

    #[test]
    fn parses_rime_dict_yaml_block_lists_after_commented_keys() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: commented_list_key_primary
version: "0.1"
sort: original
columns: # dictionary field order
  - code
  - text
  - weight
import_tables: # extra tables
  - secondary
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
columns: # imported field order
  - code
  - text
  - weight
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts comments after block-list mapping keys");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
        assert_eq!(entries[1].weight, 2.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_columns_with_trailing_comment() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: commented_inline_columns_sample
version: "0.1"
sort: original
columns: [code, text, weight] # inline RIME config comment
...

ba	八	10
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "吧");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_empty_required_header_scalars() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: ""
version: ''
sort: original
...

八	ba	1
"#,
        )
        .expect("quoted empty required metadata is a present YAML scalar");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_text_only_entries_for_later_encoding() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: text_only_sample
version: "0.1"
sort: original
columns: [text, weight]
...

你好	10
你	1
"#,
        )
        .expect("dictionary with text-only entries should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "你好");
        assert_eq!(entries[0].code, "");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "你");
        assert_eq!(entries[1].code, "");
        assert_eq!(entries[1].weight, 1.0);
    }

    #[test]
    fn parses_rime_dict_yaml_preserves_raw_text_column_whitespace() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: spaced_text_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	 八 	10
"#,
        )
        .expect("RIME dictionary text fields should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, " 八 ");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_weight_numeric_prefixes() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: weight_prefix_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	八	10oops
ba	吧	-2.5x
ba	巴	abc
ba	把	50%
"#,
        )
        .expect("dictionary with librime-style row weights should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, -2.5);
        assert_eq!(entries[2].text, "巴");
        assert_eq!(entries[2].weight, 0.0);
        assert_eq!(entries[3].text, "把");
        assert_eq!(entries[3].weight, 0.0);
    }

    #[test]
    fn parses_rime_dict_yaml_no_comment_marker_as_literal_hash_entries() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: no_comment_sample
version: "0.1"
sort: original
columns: [text, code, weight]
...

# skipped comment
# no comment
#hash	ha	1
#literal	li	2
"#,
        )
        .expect("RIME dictionary '# no comment' marker should allow literal hash entries");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "#hash");
        assert_eq!(entries[0].code, "ha");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "#literal");
        assert_eq!(entries[1].code, "li");
        assert_eq!(entries[1].weight, 2.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_keys_with_space_before_colon() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name : spaced_colon_primary
version : "0.1"
sort : original
columns : [code, text, weight]
import_tables : [secondary]
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name : secondary
version : "0.1"
sort : original
columns : [code, text, weight]
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts whitespace before mapping-key colons");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_header_keys() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
"name": quoted_key_primary
'version': "0.1"
"sort": original
'columns': [code, text, weight]
"import_tables": [secondary]
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
'name': secondary
"version": "0.1"
"sort": original
'columns': [code, text, weight]
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts quoted dictionary header mapping keys");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_null_columns_as_default_columns() {
        for columns_header in ["columns:", "columns: null", "columns: ~"] {
            let dictionary = TableDictionary::parse_rime_dict_yaml(&format!(
                r#"
---
name: null_columns_sample
version: "0.1"
sort: original
{columns_header}
...

八	ba	10
"#
            ))
            .expect("null columns should use the default RIME column order");

            let entries = dictionary.entries();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].text, "八");
            assert_eq!(entries[0].code, "ba");
            assert_eq!(entries[0].weight, 10.0);
        }
    }

    #[test]
    fn parses_rime_dict_yaml_scalar_columns_as_explicit_empty_list() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: scalar_columns_sample
version: "0.1"
sort: original
columns: text
...

八	ba	10
"#,
        )
        .expect("scalar columns are non-null but not a ConfigList in librime");

        assert!(dictionary.entries().is_empty());
    }

    #[test]
    fn parses_rime_dict_yaml_null_column_items_as_placeholders() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: null_column_item_sample
version: "0.1"
sort: original
columns:
  -
  - text
  - code
  - ''
  - weight
...

ignored	八	ba	ignored	10
"#,
        )
        .expect("YAML-null column items should still occupy a column position");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_null_column_items_as_placeholders() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_null_column_item_sample
version: "0.1"
sort: original
columns: [, text, code, '', weight]
...

ignored	八	ba	ignored	10
"#,
        )
        .expect("inline YAML-null column items should still occupy column positions");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_quoted_commas_as_single_column_items() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_quoted_comma_column_sample
version: "0.1"
sort: original
columns: ['ignored,placeholder', text, code, weight]
...

ignored	八	ba	10
"#,
        )
        .expect("quoted commas in YAML flow lists should not split column items");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_without_document_start() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
name: no_document_start_sample
version: "0.1"
sort: original
...

八	ba	10
"#,
        )
        .expect("librime loads dictionary headers as YAML streams without requiring '---'");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_with_utf8_bom() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            "\u{feff}name: bom_header_sample\nversion: \"0.1\"\nsort: original\n...\n\n八\tba\t10\n",
        )
        .expect("yaml-cpp accepts a leading UTF-8 BOM before the dictionary header");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_import_tables_with_primary_sorting() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
import_tables:
  - primary
  - secondary
...

八	ba	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	爸	9
ba	吧	3
"#
                    .to_owned()
                })
            },
        )
        .expect("dictionary imports should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "八");
    }

    #[test]
    fn parses_rime_dict_yaml_schema_packs_as_optional_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_and_packs(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
...

爸	ba	1
八	ba	2
"#,
            ["pack", "missing_pack", "broken_pack"],
            |name| match name {
                "pack" => Some(
                    r#"
---
name: pack
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	爸	9
ba	吧	3
"#
                    .to_owned(),
                ),
                "broken_pack" => Some("name: broken\n".to_owned()),
                _ => None,
            },
        )
        .expect("dictionary packs should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "八");
        assert_eq!(entries[3].text, "爸");
        assert_eq!(entries[3].weight, 1.0);
    }

    #[test]
    fn parses_rime_dict_yaml_preset_vocabulary_weights() {
        let mut requested_vocabulary = Vec::new();
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
vocabulary: custom
import_tables:
  - secondary
...

八	ba
吧	ba	50%
白	bai	7
"#,
            std::iter::empty::<&str>(),
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

爸	ba
"#
                    .to_owned()
                })
            },
            |name| {
                requested_vocabulary.push(name.to_owned());
                (name == "custom").then(|| {
                    "\
八\t8
吧\t6
爸\t9
"
                    .to_owned()
                })
            },
        )
        .expect("dictionary with preset vocabulary weights should parse");

        let entries = dictionary.entries();
        assert_eq!(requested_vocabulary, ["custom"]);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[0].weight, 9.0);
        assert_eq!(entries[1].text, "八");
        assert_eq!(entries[1].weight, 8.0);
        assert_eq!(entries[2].text, "吧");
        assert_eq!(entries[2].weight, 3.0);
        assert_eq!(entries[3].text, "白");
        assert_eq!(entries[3].weight, 7.0);
    }

    #[test]
    fn parses_rime_dict_yaml_skips_null_import_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [null, ~, secondary, 'null']
...

八	ba	1
"#,
            |name| match name {
                "secondary" => Some(
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

吧	ba	2
"#
                    .to_owned(),
                ),
                "null" => Some(
                    r#"
---
name: 'null'
version: "0.1"
sort: original
...

爸	ba	3
"#
                    .to_owned(),
                ),
                _ => None,
            },
        )
        .expect("YAML-null import tables should be skipped like librime config nodes");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "爸");
    }

    #[test]
    fn parses_rime_dict_yaml_unescapes_quoted_import_table_names() {
        let mut requested_imports = Vec::new();
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: escaped_import_sample
version: "0.1"
sort: original
import_tables: ['sec''ondary', "third\"table", "hex\x5ftable", "unicode\u005ftable", "long\U0000005ftable"]
...

primary	pri	1
"#,
            |table| {
                requested_imports.push(table.to_owned());
                match table {
                    "sec'ondary" => Some(
                        r#"
---
name: "sec'ondary"
version: "0.1"
...

single quote	sq	2
"#
                        .to_owned(),
                    ),
                    "third\"table" => Some(
                        r#"
---
name: 'third"table'
version: "0.1"
...

double quote	dq	3
"#
                        .to_owned(),
                    ),
                    "hex_table" => Some(
                        r#"
---
name: hex_table
version: "0.1"
...

hex escape	he	4
"#
                        .to_owned(),
                    ),
                    "unicode_table" => Some(
                        r#"
---
name: unicode_table
version: "0.1"
...

unicode escape	ue	5
"#
                        .to_owned(),
                    ),
                    "long_table" => Some(
                        r#"
---
name: long_table
version: "0.1"
...

long unicode escape	le	6
"#
                        .to_owned(),
                    ),
                    _ => None,
                }
            },
        )
        .expect("quoted YAML import table names should be unescaped like yaml-cpp scalars");

        assert_eq!(
            requested_imports,
            [
                "sec'ondary",
                "third\"table",
                "hex_table",
                "unicode_table",
                "long_table"
            ]
        );
        let entries = dictionary.entries();
        assert_eq!(entries[0].text, "primary");
        assert_eq!(entries[1].text, "single quote");
        assert_eq!(entries[2].text, "double quote");
        assert_eq!(entries[3].text, "hex escape");
        assert_eq!(entries[4].text, "unicode escape");
        assert_eq!(entries[5].text, "long unicode escape");
    }

    #[test]
    fn parses_rime_dict_yaml_skips_collection_import_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [[ignored, missing], {name: skipped}, secondary, '[literal]']
...

八	ba	1
"#,
            |name| match name {
                "secondary" => Some(
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

吧	ba	2
"#
                    .to_owned(),
                ),
                "[literal]" => Some(
                    r#"
---
name: '[literal]'
version: "0.1"
sort: original
...

爸	ba	3
"#
                    .to_owned(),
                ),
                other => panic!("non-scalar import table should be skipped, got {other}"),
            },
        )
        .expect("non-scalar import table items should be skipped like librime config nodes");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "爸");
    }

    #[test]
    fn parses_rime_dict_yaml_drops_duplicate_word_code_definitions() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [secondary]
...

八	ba	1
八	ba	99
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

八	ba	88
吧	ba	3
"#
                    .to_owned()
                })
            },
        )
        .expect("dictionary imports should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_preserves_duplicate_phrase_code_definitions() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: phrase_duplicate_sample
version: "0.1"
sort: original
...

你好	ni hao	1
你好	ni hao	2
你	ni	3
你	ni	4
"#,
        )
        .expect("dictionary with duplicate phrase code definitions should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "你好");
        assert_eq!(entries[0].code, "nihao");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "你好");
        assert_eq!(entries[1].code, "nihao");
        assert_eq!(entries[1].weight, 2.0);
        assert_eq!(entries[2].text, "你");
        assert_eq!(entries[2].code, "ni");
        assert_eq!(entries[2].weight, 3.0);
    }

    #[test]
    fn rejects_rime_dict_yaml_with_incomplete_header() {
        let missing_name = TableDictionary::parse_rime_dict_yaml(
            r#"
---
version: "0.1"
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary without a name should be rejected");
        assert_eq!(
            missing_name.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let missing_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary without a version should be rejected");
        assert_eq!(
            missing_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let commented_blank_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
version: # dictionary version is missing
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with a blank commented version should be rejected");
        assert_eq!(
            commented_blank_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let null_name = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: null
version: "0.1"
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with YAML null name should be rejected");
        assert_eq!(
            null_name.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let null_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
version: ~
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with YAML null version should be rejected");
        assert_eq!(
            null_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );
    }
}
