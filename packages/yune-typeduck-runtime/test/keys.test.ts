import { describe, expect, it } from "vitest";

import { keyEventToRimeKey, RIME_KEY, RIME_MASK, TypeDuckKeyError } from "../src/keys.js";

describe("keyEventToRimeKey", () => {
  it("maps printable lowercase keys", () => {
    expect(keyEventToRimeKey({ key: "a" })).toEqual({ keycode: "a".charCodeAt(0), mask: 0 });
  });

  it("maps printable uppercase keys with Shift", () => {
    expect(keyEventToRimeKey({ key: "A", shiftKey: true })).toEqual({
      keycode: "A".charCodeAt(0),
      mask: RIME_MASK.Shift,
    });
  });

  it("maps digit selection keys", () => {
    expect(keyEventToRimeKey({ key: "1" })).toEqual({ keycode: 49, mask: 0 });
  });

  it("maps space key spellings", () => {
    expect(keyEventToRimeKey({ key: " " })).toEqual({ keycode: 0x20, mask: 0 });
    expect(keyEventToRimeKey({ key: "Space" })).toEqual({ keycode: 0x20, mask: 0 });
  });

  it("maps editing and navigation keys", () => {
    expect(keyEventToRimeKey({ key: "Backspace" }).keycode).toBe(0xff08);
    expect(keyEventToRimeKey({ key: "Tab" }).keycode).toBe(0xff09);
    expect(keyEventToRimeKey({ key: "Enter" }).keycode).toBe(0xff0d);
    expect(keyEventToRimeKey({ key: "Escape" }).keycode).toBe(0xff1b);
    expect(keyEventToRimeKey({ key: "Delete" }).keycode).toBe(0xffff);
    expect(keyEventToRimeKey({ key: "ArrowLeft" }).keycode).toBe(0xff51);
    expect(keyEventToRimeKey({ key: "ArrowUp" }).keycode).toBe(0xff52);
    expect(keyEventToRimeKey({ key: "ArrowRight" }).keycode).toBe(0xff53);
    expect(keyEventToRimeKey({ key: "ArrowDown" }).keycode).toBe(0xff54);
    expect(keyEventToRimeKey({ key: "PageUp" }).keycode).toBe(0xff55);
    expect(keyEventToRimeKey({ key: "PageDown" }).keycode).toBe(0xff56);
    expect(keyEventToRimeKey({ key: "Home" }).keycode).toBe(0xff50);
    expect(keyEventToRimeKey({ key: "End" }).keycode).toBe(0xff57);
  });

  it("exports the RIME key constants used by mapping", () => {
    expect(RIME_KEY).toMatchObject({
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
    });
  });

  it("maps modifier-only browser keys to native keycodes", () => {
    expect(keyEventToRimeKey({ key: "Shift" })).toEqual({ keycode: RIME_KEY.Shift, mask: 0 });
    expect(keyEventToRimeKey({ key: "Control" })).toEqual({ keycode: RIME_KEY.Control, mask: 0 });
    expect(keyEventToRimeKey({ key: "Alt" })).toEqual({ keycode: RIME_KEY.Alt, mask: 0 });
    expect(keyEventToRimeKey({ key: "Meta" })).toEqual({ keycode: RIME_KEY.Meta, mask: 0 });
    expect(keyEventToRimeKey({ key: "OS" })).toEqual({ keycode: RIME_KEY.Meta, mask: 0 });
    expect(keyEventToRimeKey({ key: "CapsLock" })).toEqual({ keycode: RIME_KEY.CapsLock, mask: 0 });
  });

  it("suppresses self modifier masks for real modifier keydowns", () => {
    expect(keyEventToRimeKey({ key: "Shift", shiftKey: true })).toEqual({ keycode: RIME_KEY.Shift, mask: 0 });
    expect(keyEventToRimeKey({ key: "Control", ctrlKey: true })).toEqual({ keycode: RIME_KEY.Control, mask: 0 });
    expect(keyEventToRimeKey({ key: "Alt", altKey: true })).toEqual({ keycode: RIME_KEY.Alt, mask: 0 });
    expect(keyEventToRimeKey({ key: "Meta", metaKey: true })).toEqual({ keycode: RIME_KEY.Meta, mask: 0 });
    expect(keyEventToRimeKey({ key: "OS", metaKey: true })).toEqual({ keycode: RIME_KEY.Meta, mask: 0 });
  });

  it("maps modifier-only key releases to native Release events", () => {
    expect(keyEventToRimeKey({ key: "Shift", shiftKey: true, type: "keyup" })).toEqual({
      keycode: RIME_KEY.Shift,
      mask: RIME_MASK.Release,
    });
    expect(keyEventToRimeKey({ key: "Control", ctrlKey: true, type: "keyup" })).toEqual({
      keycode: RIME_KEY.Control,
      mask: RIME_MASK.Release,
    });
    expect(keyEventToRimeKey({ key: "Alt", altKey: true, type: "keyup" })).toEqual({
      keycode: RIME_KEY.Alt,
      mask: RIME_MASK.Release,
    });
    expect(keyEventToRimeKey({ key: "Meta", metaKey: true, type: "keyup" })).toEqual({
      keycode: RIME_KEY.Meta,
      mask: RIME_MASK.Release,
    });
  });

  it("maps modifier flags to RIME masks", () => {
    expect(keyEventToRimeKey({ key: "a", shiftKey: true }).mask).toBe(1 << 0);
    expect(keyEventToRimeKey({ key: "a", ctrlKey: true }).mask).toBe(1 << 2);
    expect(keyEventToRimeKey({ key: "a", altKey: true }).mask).toBe(1 << 3);
    expect(keyEventToRimeKey({ key: "a", metaKey: true }).mask).toBe(RIME_MASK.Super);
    expect(keyEventToRimeKey({ key: "a", type: "keyup" }).mask).toBe(1 << 30);
  });

  it("combines modifier flags", () => {
    expect(
      keyEventToRimeKey({
        key: "a",
        shiftKey: true,
        ctrlKey: true,
        altKey: true,
        metaKey: true,
        type: "keyup",
      }).mask,
    ).toBe(RIME_MASK.Shift | RIME_MASK.Control | RIME_MASK.Alt | RIME_MASK.Super | RIME_MASK.Release);
  });

  it("throws a deterministic error for unknown multi-character keys", () => {
    expect(() => keyEventToRimeKey({ key: "UnidentifiedKey" })).toThrow(TypeDuckKeyError);
    expect(() => keyEventToRimeKey({ key: "UnidentifiedKey" })).toThrow(
      "Unsupported TypeDuck key: UnidentifiedKey",
    );
  });
});
