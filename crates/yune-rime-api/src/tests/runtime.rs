use super::*;

#[test]
fn maps_bool_to_rime_bool() {
    assert_eq!(bool_from(true), TRUE);
    assert_eq!(bool_from(false), FALSE);
}

#[test]
fn key_table_exposes_librime_style_modifier_and_key_name_lookup() {
    let shift = CString::new("Shift").expect("modifier name should be valid");
    let control = CString::new("Control").expect("modifier name should be valid");
    let alt = CString::new("Alt").expect("modifier name should be valid");
    let unknown = CString::new("NoSuchModifier").expect("modifier name should be valid");

    assert_eq!(unsafe { RimeGetModifierByName(shift.as_ptr()) }, 1);
    assert_eq!(unsafe { RimeGetModifierByName(control.as_ptr()) }, 1 << 2);
    assert_eq!(unsafe { RimeGetModifierByName(alt.as_ptr()) }, 1 << 3);
    assert_eq!(unsafe { RimeGetModifierByName(unknown.as_ptr()) }, 0);
    assert_eq!(unsafe { RimeGetModifierByName(std::ptr::null()) }, 0);

    assert_eq!(
        static_c_string(RimeGetModifierName(1 << 2)).as_deref(),
        Some("Control")
    );
    assert_eq!(
        static_c_string(RimeGetModifierName((1 << 2) | (1 << 3))).as_deref(),
        Some("Control")
    );
    assert_eq!(static_c_string(RimeGetModifierName(1 << 13)), None);

    let space = CString::new("space").expect("key name should be valid");
    let backspace = CString::new("BackSpace").expect("key name should be valid");
    let linefeed = CString::new("Linefeed").expect("key name should be valid");
    let clear = CString::new("Clear").expect("key name should be valid");
    let pause = CString::new("Pause").expect("key name should be valid");
    let sys_req = CString::new("Sys_Req").expect("key name should be valid");
    let left = CString::new("Left").expect("key name should be valid");
    let prior = CString::new("Prior").expect("key name should be valid");
    let next = CString::new("Next").expect("key name should be valid");
    let begin = CString::new("Begin").expect("key name should be valid");
    let cancel = CString::new("Cancel").expect("key name should be valid");
    let break_key = CString::new("Break").expect("key name should be valid");
    let hebrew_switch = CString::new("Hebrew_switch").expect("key name should be valid");
    let mode_switch = CString::new("Mode_switch").expect("key name should be valid");
    let num_lock = CString::new("Num_Lock").expect("key name should be valid");
    let kp_enter = CString::new("KP_Enter").expect("key name should be valid");
    let kp_page_up = CString::new("KP_Page_Up").expect("key name should be valid");
    let kp_prior = CString::new("KP_Prior").expect("key name should be valid");
    let kp_page_down = CString::new("KP_Page_Down").expect("key name should be valid");
    let kp_next = CString::new("KP_Next").expect("key name should be valid");
    let kp_9 = CString::new("KP_9").expect("key name should be valid");
    let kp_equal = CString::new("KP_Equal").expect("key name should be valid");
    let f1 = CString::new("F1").expect("key name should be valid");
    let f12 = CString::new("F12").expect("key name should be valid");
    let f13 = CString::new("F13").expect("key name should be valid");
    let f24 = CString::new("F24").expect("key name should be valid");
    let f35 = CString::new("F35").expect("key name should be valid");
    let shift_l = CString::new("Shift_L").expect("key name should be valid");
    let control_r = CString::new("Control_R").expect("key name should be valid");
    let caps_lock = CString::new("Caps_Lock").expect("key name should be valid");
    let alt_l = CString::new("Alt_L").expect("key name should be valid");
    let hyper_r = CString::new("Hyper_R").expect("key name should be valid");
    let void_symbol = CString::new("VoidSymbol").expect("key name should be valid");
    let nobreakspace = CString::new("nobreakspace").expect("key name should be valid");
    let yen = CString::new("yen").expect("key name should be valid");
    let eth = CString::new("Eth").expect("key name should be valid");
    let thorn = CString::new("thorn").expect("key name should be valid");
    let ydiaeresis = CString::new("ydiaeresis").expect("key name should be valid");
    let aogonek = CString::new("Aogonek").expect("key name should be valid");
    let lcaron = CString::new("Lcaron").expect("key name should be valid");
    let racute = CString::new("Racute").expect("key name should be valid");
    let tcedilla = CString::new("tcedilla").expect("key name should be valid");
    let abovedot = CString::new("abovedot").expect("key name should be valid");
    let hstroke = CString::new("Hstroke").expect("key name should be valid");
    let gbreve = CString::new("gbreve").expect("key name should be valid");
    let scircumflex = CString::new("scircumflex").expect("key name should be valid");
    let kappa = CString::new("kappa").expect("key name should be valid");
    let kra = CString::new("kra").expect("key name should be valid");
    let rcedilla = CString::new("Rcedilla").expect("key name should be valid");
    let eng = CString::new("ENG").expect("key name should be valid");
    let umacron = CString::new("umacron").expect("key name should be valid");
    let overline = CString::new("overline").expect("key name should be valid");
    let kana_fullstop = CString::new("kana_fullstop").expect("key name should be valid");
    let kana_middledot = CString::new("kana_middledot").expect("key name should be valid");
    let kana_tu = CString::new("kana_tu").expect("key name should be valid");
    let kana_chi = CString::new("kana_CHI").expect("key name should be valid");
    let kana_ti = CString::new("kana_TI").expect("key name should be valid");
    let kana_hu = CString::new("kana_HU").expect("key name should be valid");
    let semivoicedsound = CString::new("semivoicedsound").expect("key name should be valid");
    let arabic_comma = CString::new("Arabic_comma").expect("key name should be valid");
    let arabic_hamza = CString::new("Arabic_hamza").expect("key name should be valid");
    let arabic_ha = CString::new("Arabic_ha").expect("key name should be valid");
    let arabic_heh = CString::new("Arabic_heh").expect("key name should be valid");
    let arabic_sukun = CString::new("Arabic_sukun").expect("key name should be valid");
    let serbian_dje = CString::new("Serbian_dje").expect("key name should be valid");
    let ukrainian_ie = CString::new("Ukrainian_ie").expect("key name should be valid");
    let ukranian_je = CString::new("Ukranian_je").expect("key name should be valid");
    let cyrillic_je = CString::new("Cyrillic_je").expect("key name should be valid");
    let serbian_je = CString::new("Serbian_je").expect("key name should be valid");
    let byelorussian_shortu =
        CString::new("Byelorussian_shortu").expect("key name should be valid");
    let cyrillic_dzhe = CString::new("Cyrillic_dzhe").expect("key name should be valid");
    let serbian_dze = CString::new("Serbian_dze").expect("key name should be valid");
    let cyrillic_yu = CString::new("Cyrillic_yu").expect("key name should be valid");
    let cyrillic_ha = CString::new("Cyrillic_ha").expect("key name should be valid");
    let cyrillic_hardsign = CString::new("Cyrillic_hardsign").expect("key name should be valid");
    let cyrillic_yu_upper = CString::new("Cyrillic_YU").expect("key name should be valid");
    let cyrillic_hardsign_upper =
        CString::new("Cyrillic_HARDSIGN").expect("key name should be valid");
    let greek_alphaaccent = CString::new("Greek_ALPHAaccent").expect("key name should be valid");
    let greek_iotadieresis = CString::new("Greek_IOTAdieresis").expect("key name should be valid");
    let greek_iotadiaeresis =
        CString::new("Greek_IOTAdiaeresis").expect("key name should be valid");
    let greek_lambda_upper = CString::new("Greek_LAMBDA").expect("key name should be valid");
    let greek_lamda_upper = CString::new("Greek_LAMDA").expect("key name should be valid");
    let greek_omega_upper = CString::new("Greek_OMEGA").expect("key name should be valid");
    let greek_lambda = CString::new("Greek_lambda").expect("key name should be valid");
    let greek_lamda = CString::new("Greek_lamda").expect("key name should be valid");
    let greek_finalsmallsigma =
        CString::new("Greek_finalsmallsigma").expect("key name should be valid");
    let greek_omega = CString::new("Greek_omega").expect("key name should be valid");
    let leftradical = CString::new("leftradical").expect("key name should be valid");
    let topvertsummationconnector =
        CString::new("topvertsummationconnector").expect("key name should be valid");
    let lessthanequal = CString::new("lessthanequal").expect("key name should be valid");
    let infinity = CString::new("infinity").expect("key name should be valid");
    let leftarrow = CString::new("leftarrow").expect("key name should be valid");
    let blank = CString::new("blank").expect("key name should be valid");
    let lowrightcorner = CString::new("lowrightcorner").expect("key name should be valid");
    let vertbar = CString::new("vertbar").expect("key name should be valid");
    let emspace = CString::new("emspace").expect("key name should be valid");
    let ellipsis = CString::new("ellipsis").expect("key name should be valid");
    let trademark = CString::new("trademark").expect("key name should be valid");
    let leftsinglequotemark =
        CString::new("leftsinglequotemark").expect("key name should be valid");
    let dagger = CString::new("dagger").expect("key name should be valid");
    let cursor = CString::new("cursor").expect("key name should be valid");
    let leftcaret = CString::new("leftcaret").expect("key name should be valid");
    let overbar = CString::new("overbar").expect("key name should be valid");
    let circle = CString::new("circle").expect("key name should be valid");
    let righttack = CString::new("righttack").expect("key name should be valid");
    let hebrew_doublelowline =
        CString::new("hebrew_doublelowline").expect("key name should be valid");
    let hebrew_aleph = CString::new("hebrew_aleph").expect("key name should be valid");
    let hebrew_beth = CString::new("hebrew_beth").expect("key name should be valid");
    let hebrew_samekh = CString::new("hebrew_samekh").expect("key name should be valid");
    let hebrew_finalzadi = CString::new("hebrew_finalzadi").expect("key name should be valid");
    let hebrew_qoph = CString::new("hebrew_qoph").expect("key name should be valid");
    let hebrew_taw = CString::new("hebrew_taw").expect("key name should be valid");
    let thai_kokai = CString::new("Thai_kokai").expect("key name should be valid");
    let thai_dodek = CString::new("Thai_dodek").expect("key name should be valid");
    let thai_sarauu = CString::new("Thai_sarauu").expect("key name should be valid");
    let thai_maihanakat_maitho =
        CString::new("Thai_maihanakat_maitho").expect("key name should be valid");
    let thai_baht = CString::new("Thai_baht").expect("key name should be valid");
    let thai_leksun = CString::new("Thai_leksun").expect("key name should be valid");
    let thai_lekkao = CString::new("Thai_lekkao").expect("key name should be valid");
    let hangul_kiyeog = CString::new("Hangul_Kiyeog").expect("key name should be valid");
    let hangul_hieuh = CString::new("Hangul_Hieuh").expect("key name should be valid");
    let hangul_a = CString::new("Hangul_A").expect("key name should be valid");
    let hangul_i = CString::new("Hangul_I").expect("key name should be valid");
    let hangul_j_kiyeog = CString::new("Hangul_J_Kiyeog").expect("key name should be valid");
    let hangul_j_hieuh = CString::new("Hangul_J_Hieuh").expect("key name should be valid");
    let hangul_sunkyeongeumpieub =
        CString::new("Hangul_SunkyeongeumPieub").expect("key name should be valid");
    let hangul_j_yeorinhieuh =
        CString::new("Hangul_J_YeorinHieuh").expect("key name should be valid");
    let korean_won = CString::new("Korean_Won").expect("key name should be valid");
    let oe_upper = CString::new("OE").expect("key name should be valid");
    let oe_lower = CString::new("oe").expect("key name should be valid");
    let ydiaeresis_upper = CString::new("Ydiaeresis").expect("key name should be valid");
    let ecu_sign = CString::new("EcuSign").expect("key name should be valid");
    let rupee_sign = CString::new("RupeeSign").expect("key name should be valid");
    let euro_sign = CString::new("EuroSign").expect("key name should be valid");
    let ibm_3270_duplicate = CString::new("3270_Duplicate").expect("key name should be valid");
    let ibm_3270_erase_input = CString::new("3270_EraseInput").expect("key name should be valid");
    let ibm_3270_cursor_blink = CString::new("3270_CursorBlink").expect("key name should be valid");
    let ibm_3270_enter = CString::new("3270_Enter").expect("key name should be valid");
    let iso_lock = CString::new("ISO_Lock").expect("key name should be valid");
    let iso_level3_shift = CString::new("ISO_Level3_Shift").expect("key name should be valid");
    let iso_level5_shift = CString::new("ISO_Level5_Shift").expect("key name should be valid");
    let iso_last_group_lock =
        CString::new("ISO_Last_Group_Lock").expect("key name should be valid");
    let iso_left_tab = CString::new("ISO_Left_Tab").expect("key name should be valid");
    let iso_fast_cursor_down =
        CString::new("ISO_Fast_Cursor_Down").expect("key name should be valid");
    let iso_enter = CString::new("ISO_Enter").expect("key name should be valid");
    let dead_grave = CString::new("dead_grave").expect("key name should be valid");
    let dead_horn = CString::new("dead_horn").expect("key name should be valid");
    let dead_stroke = CString::new("dead_stroke").expect("key name should be valid");
    let accessx_enable = CString::new("AccessX_Enable").expect("key name should be valid");
    let audible_bell_enable = CString::new("AudibleBell_Enable").expect("key name should be valid");
    let first_virtual_screen =
        CString::new("First_Virtual_Screen").expect("key name should be valid");
    let pointer_left = CString::new("Pointer_Left").expect("key name should be valid");
    let pointer_button_dflt =
        CString::new("Pointer_Button_Dflt").expect("key name should be valid");
    let pointer_dblclick5 = CString::new("Pointer_DblClick5").expect("key name should be valid");
    let pointer_enable_keys = CString::new("Pointer_EnableKeys").expect("key name should be valid");
    let pointer_dflt_btn_prev =
        CString::new("Pointer_DfltBtnPrev").expect("key name should be valid");
    let pointer_drag5 = CString::new("Pointer_Drag5").expect("key name should be valid");
    let multi_key = CString::new("Multi_key").expect("key name should be valid");
    let henkan = CString::new("Henkan").expect("key name should be valid");
    let henkan_mode = CString::new("Henkan_Mode").expect("key name should be valid");
    let hiragana_katakana = CString::new("Hiragana_Katakana").expect("key name should be valid");
    let eisu_toggle = CString::new("Eisu_toggle").expect("key name should be valid");
    let hangul = CString::new("Hangul").expect("key name should be valid");
    let hangul_romaja = CString::new("Hangul_Romaja").expect("key name should be valid");
    let codeinput = CString::new("Codeinput").expect("key name should be valid");
    let multiple_candidate = CString::new("MultipleCandidate").expect("key name should be valid");
    let hangul_special = CString::new("Hangul_Special").expect("key name should be valid");
    let missing = CString::new("NoSuchKey").expect("key name should be valid");

    assert_eq!(unsafe { RimeGetKeycodeByName(space.as_ptr()) }, 0x20);
    assert_eq!(unsafe { RimeGetKeycodeByName(backspace.as_ptr()) }, 0xff08);
    assert_eq!(unsafe { RimeGetKeycodeByName(linefeed.as_ptr()) }, 0xff0a);
    assert_eq!(unsafe { RimeGetKeycodeByName(clear.as_ptr()) }, 0xff0b);
    assert_eq!(unsafe { RimeGetKeycodeByName(pause.as_ptr()) }, 0xff13);
    assert_eq!(unsafe { RimeGetKeycodeByName(sys_req.as_ptr()) }, 0xff15);
    assert_eq!(unsafe { RimeGetKeycodeByName(left.as_ptr()) }, 0xff51);
    assert_eq!(unsafe { RimeGetKeycodeByName(prior.as_ptr()) }, 0xff55);
    assert_eq!(unsafe { RimeGetKeycodeByName(next.as_ptr()) }, 0xff56);
    assert_eq!(unsafe { RimeGetKeycodeByName(begin.as_ptr()) }, 0xff58);
    assert_eq!(unsafe { RimeGetKeycodeByName(cancel.as_ptr()) }, 0xff69);
    assert_eq!(unsafe { RimeGetKeycodeByName(break_key.as_ptr()) }, 0xff6b);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_switch.as_ptr()) },
        0xff7e
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(mode_switch.as_ptr()) },
        0xff7e
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(num_lock.as_ptr()) }, 0xff7f);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_enter.as_ptr()) }, 0xff8d);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_page_up.as_ptr()) }, 0xff9a);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_prior.as_ptr()) }, 0xff9a);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(kp_page_down.as_ptr()) },
        0xff9b
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_next.as_ptr()) }, 0xff9b);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_9.as_ptr()) }, 0xffb9);
    assert_eq!(unsafe { RimeGetKeycodeByName(kp_equal.as_ptr()) }, 0xffbd);
    assert_eq!(unsafe { RimeGetKeycodeByName(f1.as_ptr()) }, 0xffbe);
    assert_eq!(unsafe { RimeGetKeycodeByName(f12.as_ptr()) }, 0xffc9);
    assert_eq!(unsafe { RimeGetKeycodeByName(f13.as_ptr()) }, 0xffca);
    assert_eq!(unsafe { RimeGetKeycodeByName(f24.as_ptr()) }, 0xffd5);
    assert_eq!(unsafe { RimeGetKeycodeByName(f35.as_ptr()) }, 0xffe0);
    assert_eq!(unsafe { RimeGetKeycodeByName(shift_l.as_ptr()) }, 0xffe1);
    assert_eq!(unsafe { RimeGetKeycodeByName(control_r.as_ptr()) }, 0xffe4);
    assert_eq!(unsafe { RimeGetKeycodeByName(caps_lock.as_ptr()) }, 0xffe5);
    assert_eq!(unsafe { RimeGetKeycodeByName(alt_l.as_ptr()) }, 0xffe9);
    assert_eq!(unsafe { RimeGetKeycodeByName(hyper_r.as_ptr()) }, 0xffee);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(void_symbol.as_ptr()) },
        0x00ff_ffff
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(nobreakspace.as_ptr()) }, 0xa0);
    assert_eq!(unsafe { RimeGetKeycodeByName(yen.as_ptr()) }, 0xa5);
    assert_eq!(unsafe { RimeGetKeycodeByName(eth.as_ptr()) }, 0xd0);
    assert_eq!(unsafe { RimeGetKeycodeByName(thorn.as_ptr()) }, 0xfe);
    assert_eq!(unsafe { RimeGetKeycodeByName(ydiaeresis.as_ptr()) }, 0xff);
    assert_eq!(unsafe { RimeGetKeycodeByName(aogonek.as_ptr()) }, 0x1a1);
    assert_eq!(unsafe { RimeGetKeycodeByName(lcaron.as_ptr()) }, 0x1a5);
    assert_eq!(unsafe { RimeGetKeycodeByName(racute.as_ptr()) }, 0x1c0);
    assert_eq!(unsafe { RimeGetKeycodeByName(tcedilla.as_ptr()) }, 0x1fe);
    assert_eq!(unsafe { RimeGetKeycodeByName(abovedot.as_ptr()) }, 0x1ff);
    assert_eq!(unsafe { RimeGetKeycodeByName(hstroke.as_ptr()) }, 0x2a1);
    assert_eq!(unsafe { RimeGetKeycodeByName(gbreve.as_ptr()) }, 0x2bb);
    assert_eq!(unsafe { RimeGetKeycodeByName(scircumflex.as_ptr()) }, 0x2fe);
    assert_eq!(unsafe { RimeGetKeycodeByName(kappa.as_ptr()) }, 0x3a2);
    assert_eq!(unsafe { RimeGetKeycodeByName(kra.as_ptr()) }, 0x3a2);
    assert_eq!(unsafe { RimeGetKeycodeByName(rcedilla.as_ptr()) }, 0x3a3);
    assert_eq!(unsafe { RimeGetKeycodeByName(eng.as_ptr()) }, 0x3bd);
    assert_eq!(unsafe { RimeGetKeycodeByName(umacron.as_ptr()) }, 0x3fe);
    assert_eq!(unsafe { RimeGetKeycodeByName(overline.as_ptr()) }, 0x47e);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(kana_fullstop.as_ptr()) },
        0x4a1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(kana_middledot.as_ptr()) },
        0x4a5
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(kana_tu.as_ptr()) }, 0x4af);
    assert_eq!(unsafe { RimeGetKeycodeByName(kana_chi.as_ptr()) }, 0x4c1);
    assert_eq!(unsafe { RimeGetKeycodeByName(kana_ti.as_ptr()) }, 0x4c1);
    assert_eq!(unsafe { RimeGetKeycodeByName(kana_hu.as_ptr()) }, 0x4cc);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(semivoicedsound.as_ptr()) },
        0x4df
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(arabic_comma.as_ptr()) },
        0x5ac
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(arabic_hamza.as_ptr()) },
        0x5c1
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(arabic_ha.as_ptr()) }, 0x5e7);
    assert_eq!(unsafe { RimeGetKeycodeByName(arabic_heh.as_ptr()) }, 0x5e7);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(arabic_sukun.as_ptr()) },
        0x5f2
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(serbian_dje.as_ptr()) }, 0x6a1);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ukrainian_ie.as_ptr()) },
        0x6a4
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(ukranian_je.as_ptr()) }, 0x6a4);
    assert_eq!(unsafe { RimeGetKeycodeByName(cyrillic_je.as_ptr()) }, 0x6a8);
    assert_eq!(unsafe { RimeGetKeycodeByName(serbian_je.as_ptr()) }, 0x6a8);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(byelorussian_shortu.as_ptr()) },
        0x6ae
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(cyrillic_dzhe.as_ptr()) },
        0x6af
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(serbian_dze.as_ptr()) }, 0x6af);
    assert_eq!(unsafe { RimeGetKeycodeByName(cyrillic_yu.as_ptr()) }, 0x6c0);
    assert_eq!(unsafe { RimeGetKeycodeByName(cyrillic_ha.as_ptr()) }, 0x6c8);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(cyrillic_hardsign.as_ptr()) },
        0x6df
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(cyrillic_yu_upper.as_ptr()) },
        0x6e0
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(cyrillic_hardsign_upper.as_ptr()) },
        0x6ff
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_alphaaccent.as_ptr()) },
        0x7a1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_iotadieresis.as_ptr()) },
        0x7a5
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_iotadiaeresis.as_ptr()) },
        0x7a5
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_lambda_upper.as_ptr()) },
        0x7cb
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_lamda_upper.as_ptr()) },
        0x7cb
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_omega_upper.as_ptr()) },
        0x7d9
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_lambda.as_ptr()) },
        0x7eb
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(greek_lamda.as_ptr()) }, 0x7eb);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(greek_finalsmallsigma.as_ptr()) },
        0x7f3
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(greek_omega.as_ptr()) }, 0x7f9);
    assert_eq!(unsafe { RimeGetKeycodeByName(leftradical.as_ptr()) }, 0x8a1);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(topvertsummationconnector.as_ptr()) },
        0x8b3
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(lessthanequal.as_ptr()) },
        0x8bc
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(infinity.as_ptr()) }, 0x8c2);
    assert_eq!(unsafe { RimeGetKeycodeByName(leftarrow.as_ptr()) }, 0x8fb);
    assert_eq!(unsafe { RimeGetKeycodeByName(blank.as_ptr()) }, 0x9df);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(lowrightcorner.as_ptr()) },
        0x9ea
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(vertbar.as_ptr()) }, 0x9f8);
    assert_eq!(unsafe { RimeGetKeycodeByName(emspace.as_ptr()) }, 0xaa1);
    assert_eq!(unsafe { RimeGetKeycodeByName(ellipsis.as_ptr()) }, 0xaae);
    assert_eq!(unsafe { RimeGetKeycodeByName(trademark.as_ptr()) }, 0xac9);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(leftsinglequotemark.as_ptr()) },
        0xad0
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(dagger.as_ptr()) }, 0xaf1);
    assert_eq!(unsafe { RimeGetKeycodeByName(cursor.as_ptr()) }, 0xaff);
    assert_eq!(unsafe { RimeGetKeycodeByName(leftcaret.as_ptr()) }, 0xba3);
    assert_eq!(unsafe { RimeGetKeycodeByName(overbar.as_ptr()) }, 0xbc0);
    assert_eq!(unsafe { RimeGetKeycodeByName(circle.as_ptr()) }, 0xbcf);
    assert_eq!(unsafe { RimeGetKeycodeByName(righttack.as_ptr()) }, 0xbfc);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_doublelowline.as_ptr()) },
        0xcdf
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_aleph.as_ptr()) },
        0xce0
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(hebrew_beth.as_ptr()) }, 0xce1);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_samekh.as_ptr()) },
        0xcf1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hebrew_finalzadi.as_ptr()) },
        0xcf5
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(hebrew_qoph.as_ptr()) }, 0xcf7);
    assert_eq!(unsafe { RimeGetKeycodeByName(hebrew_taw.as_ptr()) }, 0xcfa);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_kokai.as_ptr()) }, 0xda1);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_dodek.as_ptr()) }, 0xdb4);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_sarauu.as_ptr()) }, 0xdd9);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(thai_maihanakat_maitho.as_ptr()) },
        0xdde
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_baht.as_ptr()) }, 0xddf);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_leksun.as_ptr()) }, 0xdf0);
    assert_eq!(unsafe { RimeGetKeycodeByName(thai_lekkao.as_ptr()) }, 0xdf9);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_kiyeog.as_ptr()) },
        0xea1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_hieuh.as_ptr()) },
        0xebe
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(hangul_a.as_ptr()) }, 0xebf);
    assert_eq!(unsafe { RimeGetKeycodeByName(hangul_i.as_ptr()) }, 0xed3);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_j_kiyeog.as_ptr()) },
        0xed4
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_j_hieuh.as_ptr()) },
        0xeee
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_sunkyeongeumpieub.as_ptr()) },
        0xef1
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_j_yeorinhieuh.as_ptr()) },
        0xefa
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(korean_won.as_ptr()) }, 0xeff);
    assert_eq!(unsafe { RimeGetKeycodeByName(oe_upper.as_ptr()) }, 0x13bc);
    assert_eq!(unsafe { RimeGetKeycodeByName(oe_lower.as_ptr()) }, 0x13bd);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ydiaeresis_upper.as_ptr()) },
        0x13be
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(ecu_sign.as_ptr()) }, 0x20a0);
    assert_eq!(unsafe { RimeGetKeycodeByName(rupee_sign.as_ptr()) }, 0x20a8);
    assert_eq!(unsafe { RimeGetKeycodeByName(euro_sign.as_ptr()) }, 0x20ac);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ibm_3270_duplicate.as_ptr()) },
        0xfd01
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ibm_3270_erase_input.as_ptr()) },
        0xfd07
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ibm_3270_cursor_blink.as_ptr()) },
        0xfd0f
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(ibm_3270_enter.as_ptr()) },
        0xfd1e
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(iso_lock.as_ptr()) }, 0xfe01);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_level3_shift.as_ptr()) },
        0xfe03
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_level5_shift.as_ptr()) },
        0x00ff_ffff
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_last_group_lock.as_ptr()) },
        0xfe0f
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_left_tab.as_ptr()) },
        0xfe20
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(iso_fast_cursor_down.as_ptr()) },
        0xfe2f
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(iso_enter.as_ptr()) }, 0xfe34);
    assert_eq!(unsafe { RimeGetKeycodeByName(dead_grave.as_ptr()) }, 0xfe50);
    assert_eq!(unsafe { RimeGetKeycodeByName(dead_horn.as_ptr()) }, 0xfe62);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(dead_stroke.as_ptr()) },
        0x00ff_ffff
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(accessx_enable.as_ptr()) },
        0xfe70
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(audible_bell_enable.as_ptr()) },
        0xfe7a
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(first_virtual_screen.as_ptr()) },
        0xfed0
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_left.as_ptr()) },
        0xfee0
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_button_dflt.as_ptr()) },
        0xfee8
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_dblclick5.as_ptr()) },
        0xfef3
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_enable_keys.as_ptr()) },
        0xfef9
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_dflt_btn_prev.as_ptr()) },
        0xfefc
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(pointer_drag5.as_ptr()) },
        0xfefd
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(multi_key.as_ptr()) }, 0xff20);
    assert_eq!(unsafe { RimeGetKeycodeByName(henkan.as_ptr()) }, 0xff23);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(henkan_mode.as_ptr()) },
        0xff23
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hiragana_katakana.as_ptr()) },
        0xff27
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(eisu_toggle.as_ptr()) },
        0xff30
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(hangul.as_ptr()) }, 0xff31);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_romaja.as_ptr()) },
        0xff36
    );
    assert_eq!(unsafe { RimeGetKeycodeByName(codeinput.as_ptr()) }, 0xff37);
    assert_eq!(
        unsafe { RimeGetKeycodeByName(multiple_candidate.as_ptr()) },
        0xff3d
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(hangul_special.as_ptr()) },
        0xff3f
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(missing.as_ptr()) },
        0x00ff_ffff
    );
    assert_eq!(
        unsafe { RimeGetKeycodeByName(std::ptr::null()) },
        0x00ff_ffff
    );

    assert_eq!(
        static_c_string(RimeGetKeyName(0x20)).as_deref(),
        Some("space")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff08)).as_deref(),
        Some("BackSpace")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff0a)).as_deref(),
        Some("Linefeed")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff0b)).as_deref(),
        Some("Clear")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff13)).as_deref(),
        Some("Pause")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff15)).as_deref(),
        Some("Sys_Req")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff51)).as_deref(),
        Some("Left")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff55)).as_deref(),
        Some("Page_Up")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff56)).as_deref(),
        Some("Next")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff58)).as_deref(),
        Some("Begin")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff60)).as_deref(),
        Some("Select")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff69)).as_deref(),
        Some("Cancel")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff6b)).as_deref(),
        Some("Break")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff7e)).as_deref(),
        Some("Arabic_switch")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff7f)).as_deref(),
        Some("Num_Lock")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff8d)).as_deref(),
        Some("KP_Enter")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff9a)).as_deref(),
        Some("KP_Page_Up")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff9b)).as_deref(),
        Some("KP_Next")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffb9)).as_deref(),
        Some("KP_9")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffbd)).as_deref(),
        Some("KP_Equal")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffbe)).as_deref(),
        Some("F1")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffc9)).as_deref(),
        Some("F12")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffca)).as_deref(),
        Some("F13")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffd5)).as_deref(),
        Some("F24")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe0)).as_deref(),
        Some("F35")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe1)).as_deref(),
        Some("Shift_L")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe4)).as_deref(),
        Some("Control_R")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe5)).as_deref(),
        Some("Caps_Lock")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffe9)).as_deref(),
        Some("Alt_L")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xffee)).as_deref(),
        Some("Hyper_R")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xa0)).as_deref(),
        Some("nobreakspace")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xa5)).as_deref(),
        Some("yen")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xd0)).as_deref(),
        Some("ETH")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xde)).as_deref(),
        Some("THORN")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff)).as_deref(),
        Some("ydiaeresis")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1a1)).as_deref(),
        Some("Aogonek")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1a5)).as_deref(),
        Some("Lcaron")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1c0)).as_deref(),
        Some("Racute")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1fe)).as_deref(),
        Some("tcedilla")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x1ff)).as_deref(),
        Some("abovedot")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x2a1)).as_deref(),
        Some("Hstroke")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x2bb)).as_deref(),
        Some("gbreve")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x2fe)).as_deref(),
        Some("scircumflex")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x3a2)).as_deref(),
        Some("kappa")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x3a3)).as_deref(),
        Some("Rcedilla")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x3bd)).as_deref(),
        Some("ENG")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x3fe)).as_deref(),
        Some("umacron")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x47e)).as_deref(),
        Some("overline")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4a1)).as_deref(),
        Some("kana_fullstop")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4a5)).as_deref(),
        Some("kana_conjunctive")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4af)).as_deref(),
        Some("kana_tsu")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4c1)).as_deref(),
        Some("kana_CHI")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4cc)).as_deref(),
        Some("kana_FU")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x4df)).as_deref(),
        Some("semivoicedsound")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x5ac)).as_deref(),
        Some("Arabic_comma")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x5c1)).as_deref(),
        Some("Arabic_hamza")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x5e7)).as_deref(),
        Some("Arabic_ha")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x5f2)).as_deref(),
        Some("Arabic_sukun")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6a1)).as_deref(),
        Some("Serbian_dje")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6a4)).as_deref(),
        Some("Ukrainian_ie")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6a8)).as_deref(),
        Some("Cyrillic_je")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6ae)).as_deref(),
        Some("Byelorussian_shortu")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6af)).as_deref(),
        Some("Cyrillic_dzhe")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6c0)).as_deref(),
        Some("Cyrillic_yu")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6df)).as_deref(),
        Some("Cyrillic_hardsign")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6e0)).as_deref(),
        Some("Cyrillic_YU")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x6ff)).as_deref(),
        Some("Cyrillic_HARDSIGN")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7a1)).as_deref(),
        Some("Greek_ALPHAaccent")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7a5)).as_deref(),
        Some("Greek_IOTAdiaeresis")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7cb)).as_deref(),
        Some("Greek_LAMBDA")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7d9)).as_deref(),
        Some("Greek_OMEGA")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7eb)).as_deref(),
        Some("Greek_lambda")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7f3)).as_deref(),
        Some("Greek_finalsmallsigma")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x7f9)).as_deref(),
        Some("Greek_omega")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8a1)).as_deref(),
        Some("leftradical")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8b3)).as_deref(),
        Some("topvertsummationconnector")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8bc)).as_deref(),
        Some("lessthanequal")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8c2)).as_deref(),
        Some("infinity")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x8fb)).as_deref(),
        Some("leftarrow")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x9df)).as_deref(),
        Some("blank")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x9ea)).as_deref(),
        Some("lowrightcorner")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x9f8)).as_deref(),
        Some("vertbar")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xaa1)).as_deref(),
        Some("emspace")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xaae)).as_deref(),
        Some("ellipsis")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xac9)).as_deref(),
        Some("trademark")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xad0)).as_deref(),
        Some("leftsinglequotemark")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xaf1)).as_deref(),
        Some("dagger")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xaff)).as_deref(),
        Some("cursor")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xba3)).as_deref(),
        Some("leftcaret")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xbc0)).as_deref(),
        Some("overbar")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xbcf)).as_deref(),
        Some("circle")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xbfc)).as_deref(),
        Some("righttack")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcdf)).as_deref(),
        Some("hebrew_doublelowline")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xce0)).as_deref(),
        Some("hebrew_aleph")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xce1)).as_deref(),
        Some("hebrew_bet")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcf1)).as_deref(),
        Some("hebrew_samech")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcf5)).as_deref(),
        Some("hebrew_finalzade")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcf7)).as_deref(),
        Some("hebrew_kuf")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xcfa)).as_deref(),
        Some("hebrew_taf")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xda1)).as_deref(),
        Some("Thai_kokai")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdb4)).as_deref(),
        Some("Thai_dodek")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdd9)).as_deref(),
        Some("Thai_sarauu")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdde)).as_deref(),
        Some("Thai_maihanakat_maitho")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xddf)).as_deref(),
        Some("Thai_baht")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdf0)).as_deref(),
        Some("Thai_leksun")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xdf9)).as_deref(),
        Some("Thai_lekkao")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xea1)).as_deref(),
        Some("Hangul_Kiyeog")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xebe)).as_deref(),
        Some("Hangul_Hieuh")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xebf)).as_deref(),
        Some("Hangul_A")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xed3)).as_deref(),
        Some("Hangul_I")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xed4)).as_deref(),
        Some("Hangul_J_Kiyeog")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xeee)).as_deref(),
        Some("Hangul_J_Hieuh")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xef1)).as_deref(),
        Some("Hangul_SunkyeongeumPieub")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xefa)).as_deref(),
        Some("Hangul_J_YeorinHieuh")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xeff)).as_deref(),
        Some("Korean_Won")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x13bc)).as_deref(),
        Some("OE")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x13bd)).as_deref(),
        Some("oe")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x13be)).as_deref(),
        Some("Ydiaeresis")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x20a0)).as_deref(),
        Some("EcuSign")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x20a8)).as_deref(),
        Some("RupeeSign")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x20ac)).as_deref(),
        Some("EuroSign")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfd01)).as_deref(),
        Some("3270_Duplicate")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfd07)).as_deref(),
        Some("3270_EraseInput")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfd0f)).as_deref(),
        Some("3270_CursorBlink")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfd1e)).as_deref(),
        Some("3270_Enter")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe01)).as_deref(),
        Some("ISO_Lock")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe03)).as_deref(),
        Some("ISO_Level3_Shift")
    );
    assert_eq!(static_c_string(RimeGetKeyName(0xfe11)), None);
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe0f)).as_deref(),
        Some("ISO_Last_Group_Lock")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe20)).as_deref(),
        Some("ISO_Left_Tab")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe2f)).as_deref(),
        Some("ISO_Fast_Cursor_Down")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe34)).as_deref(),
        Some("ISO_Enter")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe50)).as_deref(),
        Some("dead_grave")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe62)).as_deref(),
        Some("dead_horn")
    );
    assert_eq!(static_c_string(RimeGetKeyName(0xfe63)), None);
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe70)).as_deref(),
        Some("AccessX_Enable")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfe7a)).as_deref(),
        Some("AudibleBell_Enable")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfed0)).as_deref(),
        Some("First_Virtual_Screen")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfed5)).as_deref(),
        Some("Terminate_Server")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfee0)).as_deref(),
        Some("Pointer_Left")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfee8)).as_deref(),
        Some("Pointer_Button_Dflt")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfef3)).as_deref(),
        Some("Pointer_DblClick5")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfef9)).as_deref(),
        Some("Pointer_EnableKeys")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfefc)).as_deref(),
        Some("Pointer_DfltBtnPrev")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xfefd)).as_deref(),
        Some("Pointer_Drag5")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff20)).as_deref(),
        Some("Multi_key")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff23)).as_deref(),
        Some("Henkan")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff27)).as_deref(),
        Some("Hiragana_Katakana")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff30)).as_deref(),
        Some("Eisu_toggle")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff31)).as_deref(),
        Some("Hangul")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff36)).as_deref(),
        Some("Hangul_Romaja")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff37)).as_deref(),
        Some("Codeinput")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff3d)).as_deref(),
        Some("MultipleCandidate")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0xff3f)).as_deref(),
        Some("Hangul_Special")
    );
    assert_eq!(
        static_c_string(RimeGetKeyName(0x00ff_ffff)).as_deref(),
        Some("VoidSymbol")
    );
}

#[test]
fn rime_get_api_exposes_current_function_table() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let api = rime_get_api();
    assert!(!api.is_null());
    // SAFETY: `rime_get_api` returns a process-lifetime pointer to an
    // initialized function table.
    let api = unsafe { &*api };
    assert_eq!(
        api.data_size,
        (std::mem::size_of::<RimeApi>() - std::mem::size_of::<i32>()) as i32
    );

    let create_session = api.create_session.expect("session API should be present");
    let find_session = api.find_session.expect("session API should be present");
    let process_key = api.process_key.expect("input API should be present");
    let get_commit = api.get_commit.expect("commit API should be present");
    let free_commit = api.free_commit.expect("commit API should be present");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("cleanup API should be present");

    assert!(api.schema_open.is_some());
    assert!(api.config_open.is_some());
    assert!(api.user_config_open.is_some());
    assert!(api.config_init.is_some());
    assert!(api.config_load_string.is_some());
    assert!(api.config_get_string.is_some());
    assert!(api.config_get_item.is_some());
    assert!(api.config_set_item.is_some());
    assert!(api.config_update_signature.is_some());
    assert!(api.config_begin_map.is_some());
    assert!(api.config_begin_list.is_some());
    assert!(api.config_next.is_some());
    assert!(api.config_end.is_some());
    assert!(api.commit_proto.is_none());
    assert!(api.get_state_label.is_some());
    assert!(api.get_state_label_abbreviated.is_some());

    let session_id = create_session();
    assert_eq!(find_session(session_id), TRUE);
    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, ' ' as i32, 0), TRUE);

    let mut commit = RimeCommit {
        data_size: std::mem::size_of::<RimeCommit>() as i32,
        text: std::ptr::null_mut(),
    };
    // SAFETY: commit points to valid writable storage.
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    // SAFETY: `get_commit` returned true and populated a valid C string.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    // SAFETY: commit.text was allocated by the shim above.
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    cleanup_all_sessions();
    assert_eq!(find_session(session_id), FALSE);
}

#[test]
fn registers_and_finds_modules_by_name() {
    let _guard = test_guard();
    crate::module_registry()
        .lock()
        .expect("module registry should not be poisoned")
        .modules_by_name
        .clear();
    let module_name = CString::new("sample_module_abi").expect("module name should be valid");
    let replacement_name = CString::new("sample_module_abi").expect("module name should be valid");
    let missing_name = CString::new("missing_module_abi").expect("module name should be valid");
    let mut module = RimeModule {
        data_size: std::mem::size_of::<RimeModule>() as i32,
        module_name: module_name.as_ptr(),
        initialize: Some(sample_module_initialize),
        finalize: Some(sample_module_finalize),
        get_api: Some(sample_module_get_api),
    };
    let mut replacement = RimeModule {
        data_size: std::mem::size_of::<RimeModule>() as i32,
        module_name: replacement_name.as_ptr(),
        initialize: None,
        finalize: None,
        get_api: None,
    };
    let mut unnamed = RimeModule {
        data_size: std::mem::size_of::<RimeModule>() as i32,
        module_name: std::ptr::null(),
        initialize: None,
        finalize: None,
        get_api: None,
    };

    // SAFETY: module names point to valid NUL-terminated strings and the
    // module storage lives through the lookups below.
    assert_eq!(unsafe { RimeRegisterModule(&mut module) }, TRUE);
    // SAFETY: lookup names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeFindModule(module_name.as_ptr()) },
        std::ptr::addr_of_mut!(module)
    );
    // SAFETY: lookup name is a valid NUL-terminated string.
    assert!(unsafe { RimeFindModule(missing_name.as_ptr()) }.is_null());

    // SAFETY: replacement module uses the same valid NUL-terminated name.
    assert_eq!(unsafe { RimeRegisterModule(&mut replacement) }, TRUE);
    // SAFETY: lookup name is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeFindModule(replacement_name.as_ptr()) },
        std::ptr::addr_of_mut!(replacement)
    );

    // SAFETY: null inputs are explicitly rejected without dereferencing.
    assert_eq!(unsafe { RimeRegisterModule(std::ptr::null_mut()) }, FALSE);
    // SAFETY: unnamed points to a valid module with a null module_name.
    assert_eq!(unsafe { RimeRegisterModule(&mut unnamed) }, FALSE);
    // SAFETY: null lookup names are explicitly rejected without dereferencing.
    assert!(unsafe { RimeFindModule(std::ptr::null()) }.is_null());

    crate::module_registry()
        .lock()
        .expect("module registry should not be poisoned")
        .modules_by_name
        .clear();
}

#[test]
fn notification_handler_receives_runtime_events_and_can_be_cleared() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("notification-events");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: sample_schema\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("sample_schema.schema.yaml"),
        "schema:\n  schema_id: sample_schema\n  name: Sample\n",
    )
    .expect("shared schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };
    notification_events_lock().clear();
    let session_id = RimeCreateSession();
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let property = CString::new("client_app").expect("property name should be valid");
    let property_value = CString::new("sample_console").expect("property value should be valid");
    let schema_id = CString::new("sample_schema").expect("schema id should be valid");
    let context_object = 0x5a_usize as *mut c_void;

    RimeSetNotificationHandler(Some(record_notification), context_object);
    // SAFETY: option, property, value, and schema strings are valid
    // NUL-terminated C strings.
    unsafe {
        RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE);
        RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE);
        RimeSetProperty(session_id, property.as_ptr(), property_value.as_ptr());
        assert_eq!(RimeSelectSchema(session_id, schema_id.as_ptr()), TRUE);
    }
    assert_eq!(RimeStartMaintenance(TRUE), TRUE);
    assert_eq!(RimeDeployWorkspace(), TRUE);

    let events = notification_events_lock();
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x5a,
                session_id,
                message_type: "option".to_owned(),
                message_value: "ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id,
                message_type: "option".to_owned(),
                message_value: "!ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id,
                message_type: "property".to_owned(),
                message_value: "client_app=sample_console".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id,
                message_type: "schema".to_owned(),
                message_value: "sample_schema/sample_schema".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "start".to_owned(),
            },
            NotificationEvent {
                context_object: 0x5a,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "success".to_owned(),
            },
        ]
    );
    drop(events);

    RimeSetNotificationHandler(None, std::ptr::null_mut());
    // SAFETY: option name is a valid NUL-terminated C string.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    assert_eq!(notification_events_lock().len(), 6);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn sets_and_gets_runtime_options() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let custom_toggle = CString::new("custom_toggle").expect("option name should be valid");
    let mut status = empty_status();

    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    // SAFETY: option names are valid nul-terminated C strings.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    // SAFETY: option names are valid nul-terminated C strings.
    unsafe { RimeSetOption(session_id, custom_toggle.as_ptr(), TRUE) };

    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, custom_toggle.as_ptr()) },
        TRUE
    );
    // SAFETY: `status` points to valid writable storage initialized with a
    // positive `data_size`.
    assert_eq!(unsafe { RimeGetStatus(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_ascii_mode, TRUE);
    // SAFETY: nested pointers were allocated by `RimeGetStatus` above.
    assert_eq!(unsafe { RimeFreeStatus(&mut status) }, TRUE);

    // SAFETY: option names are valid nul-terminated C strings.
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        FALSE
    );
    assert_eq!(unsafe { RimeGetOption(0, ascii_mode.as_ptr()) }, FALSE);
    assert_eq!(
        unsafe { RimeGetOption(session_id, std::ptr::null()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn sets_and_gets_runtime_properties() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    let property = CString::new("client_app").expect("property name should be valid");
    let value = CString::new("sample_console").expect("property value should be valid");
    let empty_value = CString::new("").expect("property value should be valid");
    let mut buffer = vec![0 as c_char; 32];

    // SAFETY: property name is valid and buffer points to writable storage.
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        FALSE
    );

    // SAFETY: property name and value are valid nul-terminated C strings.
    unsafe { RimeSetProperty(session_id, property.as_ptr(), value.as_ptr()) };
    // SAFETY: property name is valid and buffer points to writable storage.
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        TRUE
    );
    // SAFETY: `RimeGetProperty` returned true and wrote a trailing NUL into
    // the caller-owned buffer.
    let copied_value = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_value.to_str(), Ok("sample_console"));

    let mut zero_len_marker = b'!' as c_char;
    // SAFETY: librime's strncpy-based getter accepts a valid output pointer
    // with a zero-length buffer and reports the non-empty property as present.
    assert_eq!(
        unsafe { RimeGetProperty(session_id, property.as_ptr(), &mut zero_len_marker, 0,) },
        TRUE
    );
    assert_eq!(zero_len_marker, b'!' as c_char);

    let mut short_buffer = vec![0 as c_char; 7];
    // SAFETY: property name is valid and buffer points to writable storage.
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                short_buffer.as_mut_ptr(),
                short_buffer.len(),
            )
        },
        TRUE
    );
    // SAFETY: the raw byte view is bounded to the caller-owned buffer.
    let truncated_value = unsafe {
        std::slice::from_raw_parts(short_buffer.as_ptr().cast::<u8>(), short_buffer.len())
    };
    assert_eq!(truncated_value, b"sample_");

    // SAFETY: empty properties are accepted on set but rejected on get, as
    // librime treats empty property values as absent.
    unsafe { RimeSetProperty(session_id, property.as_ptr(), empty_value.as_ptr()) };
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                buffer.as_mut_ptr(),
                buffer.len(),
            )
        },
        FALSE
    );
    assert_eq!(
        unsafe {
            RimeGetProperty(
                session_id,
                property.as_ptr(),
                std::ptr::null_mut(),
                buffer.len(),
            )
        },
        FALSE
    );
    assert_eq!(
        unsafe { RimeGetProperty(session_id, std::ptr::null(), buffer.as_mut_ptr(), 0) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn simulates_librime_style_key_sequences() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let session_id = RimeCreateSession();
    {
        let mut registry = crate::sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let session = registry
            .sessions
            .get_mut(&session_id)
            .expect("session should exist");
        session
            .engine
            .add_translator(StaticTableTranslator::new([("ni", "你")]));
    }
    let sequence = CString::new("ni{space}").expect("key sequence should be valid");
    let noop_named_sequence = CString::new("{Tab}").expect("key sequence should be valid");
    let noop_control_sequence = CString::new("{Linefeed}{Clear}{Pause}{Scroll_Lock}{Sys_Req}")
        .expect("key sequence should be valid");
    let noop_misc_sequence = CString::new(
        "{Begin}{Select}{Print}{Execute}{Insert}{Undo}{Redo}{Menu}{Find}{Cancel}{Help}{Break}",
    )
    .expect("key sequence should be valid");
    let noop_switch_sequence = CString::new(
        "{Arabic_switch}{Greek_switch}{Hangul_switch}{Hebrew_switch}{ISO_Group_Shift}{Mode_switch}{kana_switch}{script_switch}{Num_Lock}",
    )
    .expect("key sequence should be valid");
    let noop_function_sequence =
        CString::new("{F1}{Alt+F4}{F12}{F13}{F35}").expect("key sequence should be valid");
    let noop_modifier_key_sequence = CString::new(
        "{Shift_L}{Shift_R}{Control_L}{Control_R}{Caps_Lock}{Shift_Lock}{Meta_L}{Meta_R}{Alt_L}{Alt_R}{Super_L}{Super_R}{Hyper_L}{Release+Hyper_R}",
    )
    .expect("key sequence should be valid");
    let noop_iso_key_sequence = CString::new(
        "{ISO_Lock}{ISO_Level2_Latch}{ISO_Level3_Shift}{ISO_Level3_Latch}{ISO_Level3_Lock}{ISO_Group_Latch}{ISO_Group_Lock}{ISO_Next_Group}{ISO_Next_Group_Lock}{ISO_Prev_Group}{ISO_Prev_Group_Lock}{ISO_First_Group}{ISO_First_Group_Lock}{ISO_Last_Group}{ISO_Last_Group_Lock}{ISO_Left_Tab}{ISO_Move_Line_Up}{ISO_Move_Line_Down}{ISO_Partial_Line_Up}{ISO_Partial_Line_Down}{ISO_Partial_Space_Left}{ISO_Partial_Space_Right}{ISO_Set_Margin_Left}{ISO_Set_Margin_Right}{ISO_Release_Margin_Left}{ISO_Release_Margin_Right}{ISO_Release_Both_Margins}{ISO_Fast_Cursor_Left}{ISO_Fast_Cursor_Right}{ISO_Fast_Cursor_Up}{ISO_Fast_Cursor_Down}{ISO_Continuous_Underline}{ISO_Discontinuous_Underline}{ISO_Emphasize}{ISO_Center_Object}{Release+ISO_Enter}",
    )
    .expect("key sequence should be valid");
    let noop_xkb_key_sequence = CString::new(concat!(
        "{dead_grave}{dead_acute}{dead_circumflex}{dead_tilde}{dead_macron}",
        "{dead_breve}{dead_abovedot}{dead_diaeresis}{dead_abovering}",
        "{dead_doubleacute}{dead_caron}{dead_cedilla}{dead_ogonek}",
        "{dead_iota}{dead_voiced_sound}{dead_semivoiced_sound}{dead_belowdot}",
        "{dead_hook}{dead_horn}{AccessX_Enable}{AccessX_Feedback_Enable}",
        "{RepeatKeys_Enable}{SlowKeys_Enable}{BounceKeys_Enable}",
        "{StickyKeys_Enable}{MouseKeys_Enable}{MouseKeys_Accel_Enable}",
        "{Overlay1_Enable}{Overlay2_Enable}{AudibleBell_Enable}",
        "{First_Virtual_Screen}{Prev_Virtual_Screen}{Next_Virtual_Screen}",
        "{Last_Virtual_Screen}{Terminate_Server}{Pointer_Left}{Pointer_Right}",
        "{Pointer_Up}{Pointer_Down}{Pointer_UpLeft}{Pointer_UpRight}",
        "{Pointer_DownLeft}{Pointer_DownRight}{Pointer_Button_Dflt}",
        "{Pointer_Button1}{Pointer_Button2}{Pointer_Button3}{Pointer_Button4}",
        "{Pointer_Button5}{Pointer_DblClick_Dflt}{Pointer_DblClick1}",
        "{Pointer_DblClick2}{Pointer_DblClick3}{Pointer_DblClick4}",
        "{Pointer_DblClick5}{Pointer_Drag_Dflt}{Pointer_Drag1}",
        "{Pointer_Drag2}{Pointer_Drag3}{Pointer_Drag4}{Pointer_EnableKeys}",
        "{Pointer_Accelerate}{Pointer_DfltBtnNext}{Pointer_DfltBtnPrev}",
        "{Release+Pointer_Drag5}",
    ))
    .expect("key sequence should be valid");
    let noop_input_method_key_sequence = CString::new(concat!(
        "{Multi_key}{Kanji}{Muhenkan}{Henkan}{Henkan_Mode}{Romaji}",
        "{Hiragana}{Katakana}{Hiragana_Katakana}{Zenkaku}{Hankaku}",
        "{Zenkaku_Hankaku}{Touroku}{Massyo}{Kana_Lock}{Kana_Shift}",
        "{Eisu_Shift}{Eisu_toggle}{Hangul}{Hangul_Start}{Hangul_End}",
        "{Hangul_Hanja}{Hangul_Jamo}{Hangul_Romaja}{Codeinput}",
        "{Hangul_Jeonja}{Hangul_Banja}{Hangul_PreHanja}{Hangul_PostHanja}",
        "{SingleCandidate}{MultipleCandidate}{PreviousCandidate}",
        "{Release+Hangul_Special}",
    ))
    .expect("key sequence should be valid");
    let noop_keypad_sequence = CString::new(concat!(
        "{KP_Space}{KP_Tab}{KP_F1}{KP_F2}{KP_F3}{KP_F4}{KP_Begin}",
        "{KP_Insert}{KP_Delete}{KP_Multiply}{KP_Add}{KP_Separator}",
        "{KP_Subtract}{KP_Decimal}{KP_Divide}{Release+KP_Equal}",
    ))
    .expect("key sequence should be valid");
    let noop_latin1_key_sequence =
        CString::new("{nobreakspace}{yen}{ETH}{Eth}{THORN}{Thorn}{division}{Release+ydiaeresis}")
            .expect("key sequence should be valid");
    let noop_latin2_key_sequence =
        CString::new("{Aogonek}{breve}{Lstroke}{Scaron}{Dstroke}{Odoubleacute}{Release+abovedot}")
            .expect("key sequence should be valid");
    let noop_latin3_key_sequence = CString::new(
        "{Hstroke}{Hcircumflex}{Iabovedot}{Gbreve}{Jcircumflex}{Scircumflex}{Release+scircumflex}",
    )
    .expect("key sequence should be valid");
    let noop_latin4_key_sequence = CString::new(
        "{kappa}{kra}{Rcedilla}{Itilde}{Lcedilla}{ENG}{Amacron}{Umacron}{Release+umacron}",
    )
    .expect("key sequence should be valid");
    let noop_kana_key_sequence = CString::new(
        "{overline}{kana_fullstop}{kana_conjunctive}{kana_middledot}{kana_tu}{kana_TI}{kana_HU}{voicedsound}{Release+semivoicedsound}",
    )
    .expect("key sequence should be valid");
    let noop_arabic_key_sequence = CString::new(
        "{Arabic_comma}{Arabic_semicolon}{Arabic_question_mark}{Arabic_hamza}{Arabic_hamzaonyeh}{Arabic_tatweel}{Arabic_ha}{Arabic_heh}{Release+Arabic_sukun}",
    )
    .expect("key sequence should be valid");
    let noop_cyrillic_key_sequence = CString::new(
        "{Serbian_dje}{Ukrainian_ie}{Ukranian_je}{Cyrillic_je}{Serbian_je}{Byelorussian_shortu}{Cyrillic_dzhe}{Serbian_dze}{numerosign}{Cyrillic_yu}{Cyrillic_hardsign}{Cyrillic_YU}{Release+Cyrillic_HARDSIGN}",
    )
    .expect("key sequence should be valid");
    let noop_greek_key_sequence = CString::new(
        "{Greek_ALPHAaccent}{Greek_IOTAdieresis}{Greek_IOTAdiaeresis}{Greek_LAMBDA}{Greek_LAMDA}{Greek_OMEGA}{Greek_lambda}{Greek_lamda}{Greek_finalsmallsigma}{Release+Greek_omega}",
    )
    .expect("key sequence should be valid");
    let noop_technical_key_sequence = CString::new(
        "{leftradical}{topleftradical}{topvertsummationconnector}{lessthanequal}{infinity}{leftarrow}{blank}{lowrightcorner}{Release+vertbar}",
    )
    .expect("key sequence should be valid");
    let noop_publishing_key_sequence = CString::new(
        "{emspace}{enspace}{signifblank}{ellipsis}{trademark}{leftsinglequotemark}{telephone}{leftcaret}{overbar}{Release+righttack}",
    )
    .expect("key sequence should be valid");
    let noop_hebrew_key_sequence = CString::new(
        "{hebrew_doublelowline}{hebrew_aleph}{hebrew_bet}{hebrew_beth}{hebrew_samech}{hebrew_samekh}{hebrew_kuf}{hebrew_qoph}{Release+hebrew_taw}",
    )
    .expect("key sequence should be valid");
    let named_ascii_sequence =
        CString::new("{exclam}{space}").expect("key sequence should be valid");
    let invalid_sequence =
        CString::new("x{Unknown}").expect("key sequence should be valid C string");
    let mut commit = RimeCommit {
        data_size: 0,
        text: std::ptr::null_mut(),
    };
    let mut context = empty_context();

    // SAFETY: sequence is a valid nul-terminated C string.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("你"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    // SAFETY: noop_named_sequence is a valid C string; librime parses known
    // key-table names such as Tab even when the engine does not handle them.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_named_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: Tab is a parsed no-op and should leave the context empty after
    // the previous commit.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_control_sequence is a valid C string; librime parses these
    // adjacent key-table names even when the engine ignores their events.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_control_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored named keys should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_misc_sequence is a valid C string; librime accepts these
    // named function keys in simulated sequences even when no processor handles
    // them in the active session.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_misc_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored named keys should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_switch_sequence is a valid C string; librime accepts
    // mode-switch aliases and Num_Lock as known key-table names.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_switch_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored named keys should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_function_sequence is a valid C string; librime accepts F1
    // through F35 via its key table even when the active engine ignores them.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_function_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored function keys should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_modifier_key_sequence is a valid C string; librime parses
    // physical modifier key names as key-table names even when ignored.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_modifier_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored modifier-key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_iso_key_sequence is a valid C string; librime parses the
    // ISO key-name block through its key table even when no processor handles
    // the resulting key events.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_iso_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored ISO key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_xkb_key_sequence is a valid C string; librime parses the
    // XKB/dead-key block through its key table even when no processor handles
    // the resulting key events.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_xkb_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored XKB key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_input_method_key_sequence is a valid C string; librime
    // parses input-method key names through its key table even when ignored.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_input_method_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored input-method key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_keypad_sequence is a valid C string; librime parses keypad
    // key-table names through its key table even when ignored by the engine.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_keypad_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored keypad key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_latin1_key_sequence is a valid C string; librime parses
    // Latin-1 key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_latin1_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Latin-1 key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_latin2_key_sequence is a valid C string; librime parses
    // Latin-2 key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_latin2_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Latin-2 key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_latin3_key_sequence is a valid C string; librime parses
    // Latin-3 key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_latin3_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Latin-3 key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_latin4_key_sequence is a valid C string; librime parses
    // Latin-4 key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_latin4_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Latin-4 key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_kana_key_sequence is a valid C string; librime parses
    // kana key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_kana_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored kana key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_arabic_key_sequence is a valid C string; librime parses
    // Arabic key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_arabic_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Arabic key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_cyrillic_key_sequence is a valid C string; librime parses
    // Cyrillic key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_cyrillic_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Cyrillic key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_greek_key_sequence is a valid C string; librime parses
    // Greek key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_greek_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Greek key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_technical_key_sequence is a valid C string; librime parses
    // technical-symbol key-table names even though the default editor/speller
    // ignore non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_technical_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored technical key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_publishing_key_sequence is a valid C string; librime parses
    // publishing/APL key-table names even though the default editor/speller
    // ignore non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_publishing_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored publishing/APL key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: noop_hebrew_key_sequence is a valid C string; librime parses
    // Hebrew key-table names even though the default editor/speller ignore
    // non-ASCII keycodes.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, noop_hebrew_key_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: ignored Hebrew key names should leave the context empty.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: named_ascii_sequence is a valid C string; librime parses ASCII
    // symbolic key names through its key table as printable key events.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, named_ascii_sequence.as_ptr()) },
        TRUE
    );
    // SAFETY: `commit` points to valid writable storage for this test.
    assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
    // SAFETY: `RimeGetCommit` returned true and populated `text`.
    let text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(text.to_str(), Ok("!"));
    // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
    assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);

    // SAFETY: invalid sequence is a valid C string but should fail parsing.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, invalid_sequence.as_ptr()) },
        FALSE
    );
    // SAFETY: parse failures should not partially apply the leading `x`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 0);
    assert_eq!(context.menu.num_candidates, 0);
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);

    // SAFETY: null and invalid sessions are explicitly rejected.
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id, std::ptr::null()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeSimulateKeySequence(session_id + 1, sequence.as_ptr()) },
        FALSE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
}
