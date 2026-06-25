export interface YuneWebKeyboardEventLike {
  key: string;
  shiftKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
  type?: string;
}

export interface RimeKey {
  keycode: number;
  mask: number;
}

export class YuneWebKeyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "YuneWebKeyError";
  }
}

export const RIME_KEY = {
  Backspace: 0xff08,
  Tab: 0xff09,
  Enter: 0xff0d,
  Escape: 0xff1b,
  Delete: 0xffff,
  ArrowLeft: 0xff51,
  ArrowUp: 0xff52,
  ArrowRight: 0xff53,
  ArrowDown: 0xff54,
  PageUp: 0xff55,
  PageDown: 0xff56,
  Home: 0xff50,
  End: 0xff57,
  Space: 0x20,
  Shift: 0xffe1,
  Control: 0xffe3,
  CapsLock: 0xffe5,
  Alt: 0xffe9,
  Meta: 0xffeb,
} as const;

export const RIME_MASK = {
  Shift: 1 << 0,
  Control: 1 << 2,
  Alt: 1 << 3,
  Super: 1 << 26,
  Hyper: 1 << 27,
  Meta: 1 << 28,
  Release: 1 << 30,
} as const;

const NAMED_KEYCODES: Readonly<Record<string, number>> = {
  Backspace: RIME_KEY.Backspace,
  BackSpace: RIME_KEY.Backspace,
  Tab: RIME_KEY.Tab,
  Enter: RIME_KEY.Enter,
  Return: RIME_KEY.Enter,
  Escape: RIME_KEY.Escape,
  Esc: RIME_KEY.Escape,
  Delete: RIME_KEY.Delete,
  ArrowLeft: RIME_KEY.ArrowLeft,
  Left: RIME_KEY.ArrowLeft,
  ArrowUp: RIME_KEY.ArrowUp,
  Up: RIME_KEY.ArrowUp,
  ArrowRight: RIME_KEY.ArrowRight,
  Right: RIME_KEY.ArrowRight,
  ArrowDown: RIME_KEY.ArrowDown,
  Down: RIME_KEY.ArrowDown,
  PageUp: RIME_KEY.PageUp,
  Page_Up: RIME_KEY.PageUp,
  Prior: RIME_KEY.PageUp,
  PageDown: RIME_KEY.PageDown,
  Page_Down: RIME_KEY.PageDown,
  Next: RIME_KEY.PageDown,
  Home: RIME_KEY.Home,
  End: RIME_KEY.End,
  Space: RIME_KEY.Space,
  " ": RIME_KEY.Space,
  Shift: RIME_KEY.Shift,
  Control: RIME_KEY.Control,
  CapsLock: RIME_KEY.CapsLock,
  Alt: RIME_KEY.Alt,
  Meta: RIME_KEY.Meta,
  OS: RIME_KEY.Meta,
  asciitilde: "~".charCodeAt(0),
  quoteleft: "`".charCodeAt(0),
  exclam: "!".charCodeAt(0),
  at: "@".charCodeAt(0),
  numbersign: "#".charCodeAt(0),
  dollar: "$".charCodeAt(0),
  percent: "%".charCodeAt(0),
  asciicircum: "^".charCodeAt(0),
  ampersand: "&".charCodeAt(0),
  asterisk: "*".charCodeAt(0),
  parenleft: "(".charCodeAt(0),
  parenright: ")".charCodeAt(0),
  minus: "-".charCodeAt(0),
  underscore: "_".charCodeAt(0),
  plus: "+".charCodeAt(0),
  equal: "=".charCodeAt(0),
  braceleft: "{".charCodeAt(0),
  bracketleft: "[".charCodeAt(0),
  braceright: "}".charCodeAt(0),
  bracketright: "]".charCodeAt(0),
  colon: ":".charCodeAt(0),
  semicolon: ";".charCodeAt(0),
  quotedbl: "\"".charCodeAt(0),
  apostrophe: "'".charCodeAt(0),
  bar: "|".charCodeAt(0),
  backslash: "\\".charCodeAt(0),
  less: "<".charCodeAt(0),
  comma: ",".charCodeAt(0),
  greater: ">".charCodeAt(0),
  period: ".".charCodeAt(0),
  question: "?".charCodeAt(0),
  slash: "/".charCodeAt(0),
  space: RIME_KEY.Space,
};

export function keyEventToRimeKey(event: YuneWebKeyboardEventLike): RimeKey {
  const keycode = keyToCodePoint(event.key);
  return {
    keycode,
    mask: eventToMask(event) & ~selfModifierMask(event.key),
  };
}

function keyToCodePoint(key: string): number {
  const keypadDigit = /^KP_([0-9])$/.exec(key);
  if (keypadDigit !== null) {
    return 0xffb0 + Number(keypadDigit[1]);
  }

  const named = NAMED_KEYCODES[key];
  if (named !== undefined) {
    return named;
  }

  if ([...key].length === 1) {
    const codePoint = key.codePointAt(0);
    if (codePoint !== undefined) {
      return codePoint;
    }
  }

  throw new YuneWebKeyError(`Unsupported YuneWeb key: ${key}`);
}

function eventToMask(event: YuneWebKeyboardEventLike): number {
  let mask = 0;
  if (event.shiftKey === true) {
    mask |= RIME_MASK.Shift;
  }
  if (event.ctrlKey === true) {
    mask |= RIME_MASK.Control;
  }
  if (event.altKey === true) {
    mask |= RIME_MASK.Alt;
  }
  if (event.metaKey === true) {
    mask |= RIME_MASK.Super;
  }
  if (event.type === "keyup") {
    mask |= RIME_MASK.Release;
  }
  return mask;
}

function selfModifierMask(key: string): number {
  switch (key) {
    case "Shift":
      return RIME_MASK.Shift;
    case "Control":
      return RIME_MASK.Control;
    case "Alt":
      return RIME_MASK.Alt;
    case "Meta":
    case "OS":
      return RIME_MASK.Super;
    default:
      return 0;
  }
}
