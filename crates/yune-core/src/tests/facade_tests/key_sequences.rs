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
