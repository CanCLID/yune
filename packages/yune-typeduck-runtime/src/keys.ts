export interface TypeDuckKeyboardEventLike {
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

export class TypeDuckKeyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeDuckKeyError";
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
  Prior: RIME_KEY.PageUp,
  PageDown: RIME_KEY.PageDown,
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
};

export function keyEventToRimeKey(event: TypeDuckKeyboardEventLike): RimeKey {
  const keycode = keyToCodePoint(event.key);
  return {
    keycode,
    mask: eventToMask(event) & ~selfModifierMask(event.key),
  };
}

function keyToCodePoint(key: string): number {
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

  throw new TypeDuckKeyError(`Unsupported TypeDuck key: ${key}`);
}

function eventToMask(event: TypeDuckKeyboardEventLike): number {
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
