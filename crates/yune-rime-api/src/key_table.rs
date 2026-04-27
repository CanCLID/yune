use std::{
    ffi::CStr,
    os::raw::{c_char, c_int},
    ptr,
};

const XK_VOID_SYMBOL: c_int = 0x00ff_ffff;
const XK_BACKSPACE: c_int = 0xff08;
const XK_TAB: c_int = 0xff09;
const XK_RETURN: c_int = 0xff0d;
const XK_ESCAPE: c_int = 0xff1b;
const XK_DELETE: c_int = 0xffff;
const XK_LEFT: c_int = 0xff51;
const XK_UP: c_int = 0xff52;
const XK_RIGHT: c_int = 0xff53;
const XK_DOWN: c_int = 0xff54;
const XK_HOME: c_int = 0xff50;
const XK_END: c_int = 0xff57;
const XK_PAGE_UP: c_int = 0xff55;
const XK_PAGE_DOWN: c_int = 0xff56;
const XK_INSERT: c_int = 0xff63;
const XK_KP_SPACE: c_int = 0xff80;
const XK_KP_TAB: c_int = 0xff89;
const XK_KP_ENTER: c_int = 0xff8d;
const XK_KP_F1: c_int = 0xff91;
const XK_KP_F2: c_int = 0xff92;
const XK_KP_F3: c_int = 0xff93;
const XK_KP_F4: c_int = 0xff94;
const XK_KP_HOME: c_int = 0xff95;
const XK_KP_LEFT: c_int = 0xff96;
const XK_KP_UP: c_int = 0xff97;
const XK_KP_RIGHT: c_int = 0xff98;
const XK_KP_DOWN: c_int = 0xff99;
const XK_KP_PAGE_UP: c_int = 0xff9a;
const XK_KP_PAGE_DOWN: c_int = 0xff9b;
const XK_KP_END: c_int = 0xff9c;
const XK_KP_BEGIN: c_int = 0xff9d;
const XK_KP_INSERT: c_int = 0xff9e;
const XK_KP_DELETE: c_int = 0xff9f;
const XK_KP_MULTIPLY: c_int = 0xffaa;
const XK_KP_ADD: c_int = 0xffab;
const XK_KP_SEPARATOR: c_int = 0xffac;
const XK_KP_SUBTRACT: c_int = 0xffad;
const XK_KP_DECIMAL: c_int = 0xffae;
const XK_KP_DIVIDE: c_int = 0xffaf;
const XK_KP_0: c_int = 0xffb0;
const XK_KP_1: c_int = 0xffb1;
const XK_KP_2: c_int = 0xffb2;
const XK_KP_3: c_int = 0xffb3;
const XK_KP_4: c_int = 0xffb4;
const XK_KP_5: c_int = 0xffb5;
const XK_KP_6: c_int = 0xffb6;
const XK_KP_7: c_int = 0xffb7;
const XK_KP_8: c_int = 0xffb8;
const XK_KP_9: c_int = 0xffb9;
const XK_KP_EQUAL: c_int = 0xffbd;
const XK_F1: c_int = 0xffbe;
const XK_F2: c_int = 0xffbf;
const XK_F3: c_int = 0xffc0;
const XK_F4: c_int = 0xffc1;
const XK_F5: c_int = 0xffc2;
const XK_F6: c_int = 0xffc3;
const XK_F7: c_int = 0xffc4;
const XK_F8: c_int = 0xffc5;
const XK_F9: c_int = 0xffc6;
const XK_F10: c_int = 0xffc7;
const XK_F11: c_int = 0xffc8;
const XK_F12: c_int = 0xffc9;

const MODIFIERS: &[(usize, &[u8])] = &[
    (0, b"Shift\0"),
    (1, b"Lock\0"),
    (2, b"Control\0"),
    (3, b"Alt\0"),
    (4, b"Mod2\0"),
    (5, b"Mod3\0"),
    (6, b"Mod4\0"),
    (7, b"Mod5\0"),
    (8, b"Button1\0"),
    (9, b"Button2\0"),
    (10, b"Button3\0"),
    (11, b"Button4\0"),
    (12, b"Button5\0"),
    (26, b"Super\0"),
    (27, b"Hyper\0"),
    (28, b"Meta\0"),
    (30, b"Release\0"),
];

const NAMED_KEYS: &[(&[u8], c_int)] = &[
    (b"BackSpace\0", XK_BACKSPACE),
    (b"Tab\0", XK_TAB),
    (b"Return\0", XK_RETURN),
    (b"Escape\0", XK_ESCAPE),
    (b"Delete\0", XK_DELETE),
    (b"Left\0", XK_LEFT),
    (b"Up\0", XK_UP),
    (b"Right\0", XK_RIGHT),
    (b"Down\0", XK_DOWN),
    (b"Home\0", XK_HOME),
    (b"End\0", XK_END),
    (b"Page_Up\0", XK_PAGE_UP),
    (b"Page_Down\0", XK_PAGE_DOWN),
    (b"Insert\0", XK_INSERT),
    (b"KP_Space\0", XK_KP_SPACE),
    (b"KP_Tab\0", XK_KP_TAB),
    (b"KP_Enter\0", XK_KP_ENTER),
    (b"KP_F1\0", XK_KP_F1),
    (b"KP_F2\0", XK_KP_F2),
    (b"KP_F3\0", XK_KP_F3),
    (b"KP_F4\0", XK_KP_F4),
    (b"KP_Home\0", XK_KP_HOME),
    (b"KP_Left\0", XK_KP_LEFT),
    (b"KP_Up\0", XK_KP_UP),
    (b"KP_Right\0", XK_KP_RIGHT),
    (b"KP_Down\0", XK_KP_DOWN),
    (b"KP_Page_Up\0", XK_KP_PAGE_UP),
    (b"KP_Prior\0", XK_KP_PAGE_UP),
    (b"KP_Next\0", XK_KP_PAGE_DOWN),
    (b"KP_Page_Down\0", XK_KP_PAGE_DOWN),
    (b"KP_End\0", XK_KP_END),
    (b"KP_Begin\0", XK_KP_BEGIN),
    (b"KP_Insert\0", XK_KP_INSERT),
    (b"KP_Delete\0", XK_KP_DELETE),
    (b"KP_Multiply\0", XK_KP_MULTIPLY),
    (b"KP_Add\0", XK_KP_ADD),
    (b"KP_Separator\0", XK_KP_SEPARATOR),
    (b"KP_Subtract\0", XK_KP_SUBTRACT),
    (b"KP_Decimal\0", XK_KP_DECIMAL),
    (b"KP_Divide\0", XK_KP_DIVIDE),
    (b"KP_0\0", XK_KP_0),
    (b"KP_1\0", XK_KP_1),
    (b"KP_2\0", XK_KP_2),
    (b"KP_3\0", XK_KP_3),
    (b"KP_4\0", XK_KP_4),
    (b"KP_5\0", XK_KP_5),
    (b"KP_6\0", XK_KP_6),
    (b"KP_7\0", XK_KP_7),
    (b"KP_8\0", XK_KP_8),
    (b"KP_9\0", XK_KP_9),
    (b"KP_Equal\0", XK_KP_EQUAL),
    (b"F1\0", XK_F1),
    (b"F2\0", XK_F2),
    (b"F3\0", XK_F3),
    (b"F4\0", XK_F4),
    (b"F5\0", XK_F5),
    (b"F6\0", XK_F6),
    (b"F7\0", XK_F7),
    (b"F8\0", XK_F8),
    (b"F9\0", XK_F9),
    (b"F10\0", XK_F10),
    (b"F11\0", XK_F11),
    (b"F12\0", XK_F12),
];

const ASCII_KEY_NAMES: &[(&[u8], c_int)] = &[
    (b"space\0", 0x20),
    (b"exclam\0", 0x21),
    (b"quotedbl\0", 0x22),
    (b"numbersign\0", 0x23),
    (b"dollar\0", 0x24),
    (b"percent\0", 0x25),
    (b"ampersand\0", 0x26),
    (b"apostrophe\0", 0x27),
    (b"quoteright\0", 0x27),
    (b"parenleft\0", 0x28),
    (b"parenright\0", 0x29),
    (b"asterisk\0", 0x2a),
    (b"plus\0", 0x2b),
    (b"comma\0", 0x2c),
    (b"minus\0", 0x2d),
    (b"period\0", 0x2e),
    (b"slash\0", 0x2f),
    (b"0\0", 0x30),
    (b"1\0", 0x31),
    (b"2\0", 0x32),
    (b"3\0", 0x33),
    (b"4\0", 0x34),
    (b"5\0", 0x35),
    (b"6\0", 0x36),
    (b"7\0", 0x37),
    (b"8\0", 0x38),
    (b"9\0", 0x39),
    (b"colon\0", 0x3a),
    (b"semicolon\0", 0x3b),
    (b"less\0", 0x3c),
    (b"equal\0", 0x3d),
    (b"greater\0", 0x3e),
    (b"question\0", 0x3f),
    (b"at\0", 0x40),
    (b"A\0", 0x41),
    (b"B\0", 0x42),
    (b"C\0", 0x43),
    (b"D\0", 0x44),
    (b"E\0", 0x45),
    (b"F\0", 0x46),
    (b"G\0", 0x47),
    (b"H\0", 0x48),
    (b"I\0", 0x49),
    (b"J\0", 0x4a),
    (b"K\0", 0x4b),
    (b"L\0", 0x4c),
    (b"M\0", 0x4d),
    (b"N\0", 0x4e),
    (b"O\0", 0x4f),
    (b"P\0", 0x50),
    (b"Q\0", 0x51),
    (b"R\0", 0x52),
    (b"S\0", 0x53),
    (b"T\0", 0x54),
    (b"U\0", 0x55),
    (b"V\0", 0x56),
    (b"W\0", 0x57),
    (b"X\0", 0x58),
    (b"Y\0", 0x59),
    (b"Z\0", 0x5a),
    (b"bracketleft\0", 0x5b),
    (b"backslash\0", 0x5c),
    (b"bracketright\0", 0x5d),
    (b"asciicircum\0", 0x5e),
    (b"underscore\0", 0x5f),
    (b"grave\0", 0x60),
    (b"quoteleft\0", 0x60),
    (b"a\0", 0x61),
    (b"b\0", 0x62),
    (b"c\0", 0x63),
    (b"d\0", 0x64),
    (b"e\0", 0x65),
    (b"f\0", 0x66),
    (b"g\0", 0x67),
    (b"h\0", 0x68),
    (b"i\0", 0x69),
    (b"j\0", 0x6a),
    (b"k\0", 0x6b),
    (b"l\0", 0x6c),
    (b"m\0", 0x6d),
    (b"n\0", 0x6e),
    (b"o\0", 0x6f),
    (b"p\0", 0x70),
    (b"q\0", 0x71),
    (b"r\0", 0x72),
    (b"s\0", 0x73),
    (b"t\0", 0x74),
    (b"u\0", 0x75),
    (b"v\0", 0x76),
    (b"w\0", 0x77),
    (b"x\0", 0x78),
    (b"y\0", 0x79),
    (b"z\0", 0x7a),
    (b"braceleft\0", 0x7b),
    (b"bar\0", 0x7c),
    (b"braceright\0", 0x7d),
    (b"asciitilde\0", 0x7e),
];

const LATIN1_KEY_NAMES: &[(&[u8], c_int)] = &[
    (b"nobreakspace\0", 0x0a0),
    (b"exclamdown\0", 0x0a1),
    (b"cent\0", 0x0a2),
    (b"sterling\0", 0x0a3),
    (b"currency\0", 0x0a4),
    (b"yen\0", 0x0a5),
    (b"brokenbar\0", 0x0a6),
    (b"section\0", 0x0a7),
    (b"diaeresis\0", 0x0a8),
    (b"copyright\0", 0x0a9),
    (b"ordfeminine\0", 0x0aa),
    (b"guillemotleft\0", 0x0ab),
    (b"notsign\0", 0x0ac),
    (b"hyphen\0", 0x0ad),
    (b"registered\0", 0x0ae),
    (b"macron\0", 0x0af),
    (b"degree\0", 0x0b0),
    (b"plusminus\0", 0x0b1),
    (b"twosuperior\0", 0x0b2),
    (b"threesuperior\0", 0x0b3),
    (b"acute\0", 0x0b4),
    (b"mu\0", 0x0b5),
    (b"paragraph\0", 0x0b6),
    (b"periodcentered\0", 0x0b7),
    (b"cedilla\0", 0x0b8),
    (b"onesuperior\0", 0x0b9),
    (b"masculine\0", 0x0ba),
    (b"guillemotright\0", 0x0bb),
    (b"onequarter\0", 0x0bc),
    (b"onehalf\0", 0x0bd),
    (b"threequarters\0", 0x0be),
    (b"questiondown\0", 0x0bf),
    (b"Agrave\0", 0x0c0),
    (b"Aacute\0", 0x0c1),
    (b"Acircumflex\0", 0x0c2),
    (b"Atilde\0", 0x0c3),
    (b"Adiaeresis\0", 0x0c4),
    (b"Aring\0", 0x0c5),
    (b"AE\0", 0x0c6),
    (b"Ccedilla\0", 0x0c7),
    (b"Egrave\0", 0x0c8),
    (b"Eacute\0", 0x0c9),
    (b"Ecircumflex\0", 0x0ca),
    (b"Ediaeresis\0", 0x0cb),
    (b"Igrave\0", 0x0cc),
    (b"Iacute\0", 0x0cd),
    (b"Icircumflex\0", 0x0ce),
    (b"Idiaeresis\0", 0x0cf),
    (b"ETH\0", 0x0d0),
    (b"Eth\0", 0x0d0),
    (b"Ntilde\0", 0x0d1),
    (b"Ograve\0", 0x0d2),
    (b"Oacute\0", 0x0d3),
    (b"Ocircumflex\0", 0x0d4),
    (b"Otilde\0", 0x0d5),
    (b"Odiaeresis\0", 0x0d6),
    (b"multiply\0", 0x0d7),
    (b"Ooblique\0", 0x0d8),
    (b"Ugrave\0", 0x0d9),
    (b"Uacute\0", 0x0da),
    (b"Ucircumflex\0", 0x0db),
    (b"Udiaeresis\0", 0x0dc),
    (b"Yacute\0", 0x0dd),
    (b"THORN\0", 0x0de),
    (b"Thorn\0", 0x0de),
    (b"ssharp\0", 0x0df),
    (b"agrave\0", 0x0e0),
    (b"aacute\0", 0x0e1),
    (b"acircumflex\0", 0x0e2),
    (b"atilde\0", 0x0e3),
    (b"adiaeresis\0", 0x0e4),
    (b"aring\0", 0x0e5),
    (b"ae\0", 0x0e6),
    (b"ccedilla\0", 0x0e7),
    (b"egrave\0", 0x0e8),
    (b"eacute\0", 0x0e9),
    (b"ecircumflex\0", 0x0ea),
    (b"ediaeresis\0", 0x0eb),
    (b"igrave\0", 0x0ec),
    (b"iacute\0", 0x0ed),
    (b"icircumflex\0", 0x0ee),
    (b"idiaeresis\0", 0x0ef),
    (b"eth\0", 0x0f0),
    (b"ntilde\0", 0x0f1),
    (b"ograve\0", 0x0f2),
    (b"oacute\0", 0x0f3),
    (b"ocircumflex\0", 0x0f4),
    (b"otilde\0", 0x0f5),
    (b"odiaeresis\0", 0x0f6),
    (b"division\0", 0x0f7),
    (b"oslash\0", 0x0f8),
    (b"ugrave\0", 0x0f9),
    (b"uacute\0", 0x0fa),
    (b"ucircumflex\0", 0x0fb),
    (b"udiaeresis\0", 0x0fc),
    (b"yacute\0", 0x0fd),
    (b"thorn\0", 0x0fe),
    (b"ydiaeresis\0", 0x0ff),
];

const LATIN2_KEY_NAMES: &[(&[u8], c_int)] = &[
    (b"Aogonek\0", 0x1a1),
    (b"breve\0", 0x1a2),
    (b"Lstroke\0", 0x1a3),
    (b"Lcaron\0", 0x1a5),
    (b"Sacute\0", 0x1a6),
    (b"Scaron\0", 0x1a9),
    (b"Scedilla\0", 0x1aa),
    (b"Tcaron\0", 0x1ab),
    (b"Zacute\0", 0x1ac),
    (b"Zcaron\0", 0x1ae),
    (b"Zabovedot\0", 0x1af),
    (b"aogonek\0", 0x1b1),
    (b"ogonek\0", 0x1b2),
    (b"lstroke\0", 0x1b3),
    (b"lcaron\0", 0x1b5),
    (b"sacute\0", 0x1b6),
    (b"caron\0", 0x1b7),
    (b"scaron\0", 0x1b9),
    (b"scedilla\0", 0x1ba),
    (b"tcaron\0", 0x1bb),
    (b"zacute\0", 0x1bc),
    (b"doubleacute\0", 0x1bd),
    (b"zcaron\0", 0x1be),
    (b"zabovedot\0", 0x1bf),
    (b"Racute\0", 0x1c0),
    (b"Abreve\0", 0x1c3),
    (b"Lacute\0", 0x1c5),
    (b"Cacute\0", 0x1c6),
    (b"Ccaron\0", 0x1c8),
    (b"Eogonek\0", 0x1ca),
    (b"Ecaron\0", 0x1cc),
    (b"Dcaron\0", 0x1cf),
    (b"Dstroke\0", 0x1d0),
    (b"Nacute\0", 0x1d1),
    (b"Ncaron\0", 0x1d2),
    (b"Odoubleacute\0", 0x1d5),
    (b"Rcaron\0", 0x1d8),
    (b"Uring\0", 0x1d9),
    (b"Udoubleacute\0", 0x1db),
    (b"Tcedilla\0", 0x1de),
    (b"racute\0", 0x1e0),
    (b"abreve\0", 0x1e3),
    (b"lacute\0", 0x1e5),
    (b"cacute\0", 0x1e6),
    (b"ccaron\0", 0x1e8),
    (b"eogonek\0", 0x1ea),
    (b"ecaron\0", 0x1ec),
    (b"dcaron\0", 0x1ef),
    (b"dstroke\0", 0x1f0),
    (b"nacute\0", 0x1f1),
    (b"ncaron\0", 0x1f2),
    (b"odoubleacute\0", 0x1f5),
    (b"udoubleacute\0", 0x1fb),
    (b"rcaron\0", 0x1f8),
    (b"uring\0", 0x1f9),
    (b"tcedilla\0", 0x1fe),
    (b"abovedot\0", 0x1ff),
];

const LATIN3_KEY_NAMES: &[(&[u8], c_int)] = &[
    (b"Hstroke\0", 0x2a1),
    (b"Hcircumflex\0", 0x2a6),
    (b"Iabovedot\0", 0x2a9),
    (b"Gbreve\0", 0x2ab),
    (b"Jcircumflex\0", 0x2ac),
    (b"hstroke\0", 0x2b1),
    (b"hcircumflex\0", 0x2b6),
    (b"idotless\0", 0x2b9),
    (b"gbreve\0", 0x2bb),
    (b"jcircumflex\0", 0x2bc),
    (b"Cabovedot\0", 0x2c5),
    (b"Ccircumflex\0", 0x2c6),
    (b"Gabovedot\0", 0x2d5),
    (b"Gcircumflex\0", 0x2d8),
    (b"Ubreve\0", 0x2dd),
    (b"Scircumflex\0", 0x2de),
    (b"cabovedot\0", 0x2e5),
    (b"ccircumflex\0", 0x2e6),
    (b"gabovedot\0", 0x2f5),
    (b"gcircumflex\0", 0x2f8),
    (b"ubreve\0", 0x2fd),
    (b"scircumflex\0", 0x2fe),
];

const LATIN4_KEY_NAMES: &[(&[u8], c_int)] = &[
    (b"kappa\0", 0x3a2),
    (b"kra\0", 0x3a2),
    (b"Rcedilla\0", 0x3a3),
    (b"Itilde\0", 0x3a5),
    (b"Lcedilla\0", 0x3a6),
    (b"Emacron\0", 0x3aa),
    (b"Gcedilla\0", 0x3ab),
    (b"Tslash\0", 0x3ac),
    (b"rcedilla\0", 0x3b3),
    (b"itilde\0", 0x3b5),
    (b"lcedilla\0", 0x3b6),
    (b"emacron\0", 0x3ba),
    (b"gcedilla\0", 0x3bb),
    (b"tslash\0", 0x3bc),
    (b"ENG\0", 0x3bd),
    (b"eng\0", 0x3bf),
    (b"Amacron\0", 0x3c0),
    (b"Iogonek\0", 0x3c7),
    (b"Eabovedot\0", 0x3cc),
    (b"Imacron\0", 0x3cf),
    (b"Ncedilla\0", 0x3d1),
    (b"Omacron\0", 0x3d2),
    (b"Kcedilla\0", 0x3d3),
    (b"Uogonek\0", 0x3d9),
    (b"Utilde\0", 0x3dd),
    (b"Umacron\0", 0x3de),
    (b"amacron\0", 0x3e0),
    (b"iogonek\0", 0x3e7),
    (b"eabovedot\0", 0x3ec),
    (b"imacron\0", 0x3ef),
    (b"ncedilla\0", 0x3f1),
    (b"omacron\0", 0x3f2),
    (b"kcedilla\0", 0x3f3),
    (b"uogonek\0", 0x3f9),
    (b"utilde\0", 0x3fd),
    (b"umacron\0", 0x3fe),
];

const KANA_KEY_NAMES: &[(&[u8], c_int)] = &[
    (b"overline\0", 0x47e),
    (b"kana_fullstop\0", 0x4a1),
    (b"kana_openingbracket\0", 0x4a2),
    (b"kana_closingbracket\0", 0x4a3),
    (b"kana_comma\0", 0x4a4),
    (b"kana_conjunctive\0", 0x4a5),
    (b"kana_middledot\0", 0x4a5),
    (b"kana_WO\0", 0x4a6),
    (b"kana_a\0", 0x4a7),
    (b"kana_i\0", 0x4a8),
    (b"kana_u\0", 0x4a9),
    (b"kana_e\0", 0x4aa),
    (b"kana_o\0", 0x4ab),
    (b"kana_ya\0", 0x4ac),
    (b"kana_yu\0", 0x4ad),
    (b"kana_yo\0", 0x4ae),
    (b"kana_tsu\0", 0x4af),
    (b"kana_tu\0", 0x4af),
    (b"prolongedsound\0", 0x4b0),
    (b"kana_A\0", 0x4b1),
    (b"kana_I\0", 0x4b2),
    (b"kana_U\0", 0x4b3),
    (b"kana_E\0", 0x4b4),
    (b"kana_O\0", 0x4b5),
    (b"kana_KA\0", 0x4b6),
    (b"kana_KI\0", 0x4b7),
    (b"kana_KU\0", 0x4b8),
    (b"kana_KE\0", 0x4b9),
    (b"kana_KO\0", 0x4ba),
    (b"kana_SA\0", 0x4bb),
    (b"kana_SHI\0", 0x4bc),
    (b"kana_SU\0", 0x4bd),
    (b"kana_SE\0", 0x4be),
    (b"kana_SO\0", 0x4bf),
    (b"kana_TA\0", 0x4c0),
    (b"kana_CHI\0", 0x4c1),
    (b"kana_TI\0", 0x4c1),
    (b"kana_TSU\0", 0x4c2),
    (b"kana_TU\0", 0x4c2),
    (b"kana_TE\0", 0x4c3),
    (b"kana_TO\0", 0x4c4),
    (b"kana_NA\0", 0x4c5),
    (b"kana_NI\0", 0x4c6),
    (b"kana_NU\0", 0x4c7),
    (b"kana_NE\0", 0x4c8),
    (b"kana_NO\0", 0x4c9),
    (b"kana_HA\0", 0x4ca),
    (b"kana_HI\0", 0x4cb),
    (b"kana_FU\0", 0x4cc),
    (b"kana_HU\0", 0x4cc),
    (b"kana_HE\0", 0x4cd),
    (b"kana_HO\0", 0x4ce),
    (b"kana_MA\0", 0x4cf),
    (b"kana_MI\0", 0x4d0),
    (b"kana_MU\0", 0x4d1),
    (b"kana_ME\0", 0x4d2),
    (b"kana_MO\0", 0x4d3),
    (b"kana_YA\0", 0x4d4),
    (b"kana_YU\0", 0x4d5),
    (b"kana_YO\0", 0x4d6),
    (b"kana_RA\0", 0x4d7),
    (b"kana_RI\0", 0x4d8),
    (b"kana_RU\0", 0x4d9),
    (b"kana_RE\0", 0x4da),
    (b"kana_RO\0", 0x4db),
    (b"kana_WA\0", 0x4dc),
    (b"kana_N\0", 0x4dd),
    (b"voicedsound\0", 0x4de),
    (b"semivoicedsound\0", 0x4df),
];

const ARABIC_KEY_NAMES: &[(&[u8], c_int)] = &[
    (b"Arabic_comma\0", 0x5ac),
    (b"Arabic_semicolon\0", 0x5bb),
    (b"Arabic_question_mark\0", 0x5bf),
    (b"Arabic_hamza\0", 0x5c1),
    (b"Arabic_maddaonalef\0", 0x5c2),
    (b"Arabic_hamzaonalef\0", 0x5c3),
    (b"Arabic_hamzaonwaw\0", 0x5c4),
    (b"Arabic_hamzaunderalef\0", 0x5c5),
    (b"Arabic_hamzaonyeh\0", 0x5c6),
    (b"Arabic_alef\0", 0x5c7),
    (b"Arabic_beh\0", 0x5c8),
    (b"Arabic_tehmarbuta\0", 0x5c9),
    (b"Arabic_teh\0", 0x5ca),
    (b"Arabic_theh\0", 0x5cb),
    (b"Arabic_jeem\0", 0x5cc),
    (b"Arabic_hah\0", 0x5cd),
    (b"Arabic_khah\0", 0x5ce),
    (b"Arabic_dal\0", 0x5cf),
    (b"Arabic_thal\0", 0x5d0),
    (b"Arabic_ra\0", 0x5d1),
    (b"Arabic_zain\0", 0x5d2),
    (b"Arabic_seen\0", 0x5d3),
    (b"Arabic_sheen\0", 0x5d4),
    (b"Arabic_sad\0", 0x5d5),
    (b"Arabic_dad\0", 0x5d6),
    (b"Arabic_tah\0", 0x5d7),
    (b"Arabic_zah\0", 0x5d8),
    (b"Arabic_ain\0", 0x5d9),
    (b"Arabic_ghain\0", 0x5da),
    (b"Arabic_tatweel\0", 0x5e0),
    (b"Arabic_feh\0", 0x5e1),
    (b"Arabic_qaf\0", 0x5e2),
    (b"Arabic_kaf\0", 0x5e3),
    (b"Arabic_lam\0", 0x5e4),
    (b"Arabic_meem\0", 0x5e5),
    (b"Arabic_noon\0", 0x5e6),
    (b"Arabic_ha\0", 0x5e7),
    (b"Arabic_heh\0", 0x5e7),
    (b"Arabic_waw\0", 0x5e8),
    (b"Arabic_alefmaksura\0", 0x5e9),
    (b"Arabic_yeh\0", 0x5ea),
    (b"Arabic_fathatan\0", 0x5eb),
    (b"Arabic_dammatan\0", 0x5ec),
    (b"Arabic_kasratan\0", 0x5ed),
    (b"Arabic_fatha\0", 0x5ee),
    (b"Arabic_damma\0", 0x5ef),
    (b"Arabic_kasra\0", 0x5f0),
    (b"Arabic_shadda\0", 0x5f1),
    (b"Arabic_sukun\0", 0x5f2),
];

/// Returns the librime modifier bit mask for a modifier name.
///
/// # Safety
///
/// `name` must be null or point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeGetModifierByName(name: *const c_char) -> c_int {
    let Some(name) = c_name(name) else {
        return 0;
    };
    MODIFIERS
        .iter()
        .find_map(|(index, modifier)| {
            (name == *modifier).then_some(1_i32.checked_shl(*index as u32).unwrap_or(0))
        })
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn RimeGetModifierName(modifier: c_int) -> *const c_char {
    if modifier == 0 {
        return ptr::null();
    }
    let first_bit = modifier.trailing_zeros() as usize;
    MODIFIERS
        .iter()
        .find_map(|(index, name)| (*index == first_bit).then_some(name.as_ptr().cast()))
        .unwrap_or(ptr::null())
}

/// Returns the X11 keysym for a librime key name.
///
/// # Safety
///
/// `name` must be null or point to a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn RimeGetKeycodeByName(name: *const c_char) -> c_int {
    let Some(name) = c_name(name) else {
        return XK_VOID_SYMBOL;
    };
    lookup_keycode(name).unwrap_or(XK_VOID_SYMBOL)
}

#[no_mangle]
pub extern "C" fn RimeGetKeyName(keycode: c_int) -> *const c_char {
    lookup_key_name(keycode).map_or(ptr::null(), |name| name.as_ptr().cast())
}

fn lookup_keycode(name: &[u8]) -> Option<c_int> {
    NAMED_KEYS
        .iter()
        .chain(ASCII_KEY_NAMES)
        .chain(LATIN1_KEY_NAMES)
        .chain(LATIN2_KEY_NAMES)
        .chain(LATIN3_KEY_NAMES)
        .chain(LATIN4_KEY_NAMES)
        .chain(KANA_KEY_NAMES)
        .chain(ARABIC_KEY_NAMES)
        .find_map(|(key_name, keycode)| (name == *key_name).then_some(*keycode))
}

fn lookup_key_name(keycode: c_int) -> Option<&'static [u8]> {
    NAMED_KEYS
        .iter()
        .chain(ASCII_KEY_NAMES)
        .chain(LATIN1_KEY_NAMES)
        .chain(LATIN2_KEY_NAMES)
        .chain(LATIN3_KEY_NAMES)
        .chain(LATIN4_KEY_NAMES)
        .chain(KANA_KEY_NAMES)
        .chain(ARABIC_KEY_NAMES)
        .find_map(|(name, candidate_keycode)| (*candidate_keycode == keycode).then_some(*name))
}

unsafe fn c_name<'a>(name: *const c_char) -> Option<&'a [u8]> {
    if name.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(name) }.to_bytes_with_nul();
    Some(bytes)
}
