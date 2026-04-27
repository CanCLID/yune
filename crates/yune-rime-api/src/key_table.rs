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
        .find_map(|(key_name, keycode)| (name == *key_name).then_some(*keycode))
}

fn lookup_key_name(keycode: c_int) -> Option<&'static [u8]> {
    NAMED_KEYS
        .iter()
        .chain(ASCII_KEY_NAMES)
        .find_map(|(name, candidate_keycode)| (*candidate_keycode == keycode).then_some(*name))
}

unsafe fn c_name<'a>(name: *const c_char) -> Option<&'a [u8]> {
    if name.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(name) }.to_bytes_with_nul();
    Some(bytes)
}
