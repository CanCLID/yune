import { describe, expect, it, vi } from "vitest";

import {
  BoundedReceiptMap,
  WEB06_REDACTED_CUSTOMIZE_VALUE,
  WEB06_REDACTED_DICTIONARY_EXCLUDE,
  WEB06_REDACTED_USERDB_TEXT,
  mapWeb06KeyboardEvent,
  web06ActionContract,
  web06AsciiModeToggleActions,
  web06ActionIdentitiesEqual,
  web06CollectionMode,
  web06EnqueueThenSignal,
  web06PrivateActionArgs,
  web06ReceiptCapacity,
  web06StableDigest,
  web06TimestampsAreOrdered,
} from "../src/yune-integration/private-protocol.js";

import type {
  Actions,
  Web06ActionIdentity,
  Web06DomEventSnapshot,
} from "../src/types.js";

const actionNames = [
  "setOption",
  "selectSchema",
  "getUserdbSnapshot",
  "importUserdb",
  "processKey",
  "stageAi",
  "selectCandidate",
  "deleteCandidate",
  "flipPage",
  "customize",
  "customizeValue",
  "deploy",
  "deployCacheSnapshot",
  "invalidateDeployCache",
  "injectedAssetsManifest",
] as const satisfies readonly (keyof Actions)[];

function keyEvent(
  type: "keydown" | "keyup",
  key: string,
  code: string,
  modifiers: Partial<Pick<Web06DomEventSnapshot, "ctrlKey" | "metaKey" | "altKey" | "shiftKey" | "repeat">> = {},
): Web06DomEventSnapshot {
  return {
    type,
    key,
    code,
    timeStamp: 10,
    repeat: false,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
    ...modifiers,
  };
}

function context(overrides: Partial<Parameters<typeof mapWeb06KeyboardEvent>[1]> = {}) {
  return {
    capturedHasComposition: true,
    currentHasComposition: true,
    isAsciiMode: false,
    isInputFocused: true,
    visibleCandidateCount: 6,
    pendingAsciiModeShift: undefined,
    pendingAsciiModeShiftWasChorded: false,
    asciiModeToggleActions: web06AsciiModeToggleActions({
      nextAsciiMode: true,
      isFullShape: false,
      outputStandard: "hong_kong_traditional",
      activeSchema: "jyut6ping3",
      isExtendedCharset: false,
      isDisabled: false,
    }),
    ...overrides,
  };
}

describe("WEB-06 private protocol", () => {
  it("freezes collection modes and bounded capacities", () => {
    expect(web06CollectionMode("")).toBe("minimal");
    expect(web06CollectionMode("?yuneWeb06Mode=off")).toBe("off");
    expect(web06CollectionMode("?yuneWeb06Mode=full")).toBe("full");
    expect(() => web06CollectionMode("?yuneWeb06Mode=other")).toThrow(/Invalid yuneWeb06Mode/);
    expect([
      web06ReceiptCapacity("off"),
      web06ReceiptCapacity("minimal"),
      web06ReceiptCapacity("full"),
    ]).toEqual([0, 2_048, 8_192]);
  });

  it("classifies every public Action without changing its public shape", () => {
    expect(actionNames.map(name => [name, web06ActionContract(name)])).toEqual([
      ["setOption", { actionClass: "stateful-barrier", boundary: "option" }],
      ["selectSchema", { actionClass: "stateful-barrier", boundary: "schema" }],
      ["getUserdbSnapshot", { actionClass: "read-only", boundary: "none" }],
      ["importUserdb", { actionClass: "stateful-barrier", boundary: "persistence" }],
      ["processKey", { actionClass: "native-key", boundary: "none" }],
      ["stageAi", { actionClass: "adapter-only", boundary: "none" }],
      ["selectCandidate", { actionClass: "stateful-barrier", boundary: "selection" }],
      ["deleteCandidate", { actionClass: "stateful-barrier", boundary: "selection" }],
      ["flipPage", { actionClass: "stateful-barrier", boundary: "paging" }],
      ["customize", { actionClass: "stateful-barrier", boundary: "option" }],
      ["customizeValue", { actionClass: "stateful-barrier", boundary: "option" }],
      ["deploy", { actionClass: "stateful-barrier", boundary: "deploy" }],
      ["deployCacheSnapshot", { actionClass: "read-only", boundary: "none" }],
      ["invalidateDeployCache", { actionClass: "stateful-barrier", boundary: "deploy" }],
      ["injectedAssetsManifest", { actionClass: "read-only", boundary: "none" }],
    ]);
  });

  it("maps printable, correction, page, commit, cancel, and modifier keys exactly", () => {
    const cases = [
      [keyEvent("keydown", "a", "KeyA"), "{a}", "native-key", "none", true],
      [keyEvent("keydown", "Backspace", "Backspace"), "{BackSpace}", "stateful-barrier", "correction", false],
      [keyEvent("keydown", "Delete", "Delete"), "{Delete}", "stateful-barrier", "correction", false],
      [keyEvent("keydown", "ArrowDown", "ArrowDown"), "{Down}", "stateful-barrier", "paging", false],
      [keyEvent("keydown", "PageDown", "PageDown"), "{Page_Down}", "stateful-barrier", "paging", false],
      [keyEvent("keydown", " ", "Space"), "{space}", "stateful-barrier", "commit", false],
      [keyEvent("keydown", ".", "Period"), "{period}", "stateful-barrier", "commit", false],
      [keyEvent("keydown", "Escape", "Escape"), "{Escape}", "stateful-barrier", "cancel", false],
      [keyEvent("keydown", "Control", "ControlLeft", { ctrlKey: true }), "{Control_L}", "stateful-barrier", "modifier-release", false],
      [keyEvent("keyup", "Control", "ControlLeft"), "{Release+Control_L}", "stateful-barrier", "modifier-release", false],
    ] as const;

    for (const [event, input, actionClass, boundary, supersedable] of cases) {
      const mapped = mapWeb06KeyboardEvent(event, context());
      expect(mapped.actions, `${event.type}/${event.code}`).toEqual([expect.objectContaining({
        name: "processKey",
        args: [input],
        actionClass,
        boundary,
        supersedable,
      })]);
    }
  });

  it("maps an unmodified visible digit to selectCandidate and preserves digit keyup as zero-action", () => {
    expect(mapWeb06KeyboardEvent(keyEvent("keydown", "2", "Digit2"), context()).actions).toEqual([
      expect.objectContaining({
        name: "selectCandidate",
        args: [1],
        actionClass: "stateful-barrier",
        boundary: "selection",
      }),
    ]);
    expect(mapWeb06KeyboardEvent(keyEvent("keyup", "2", "Digit2"), context())).toMatchObject({
      classification: "frontend-consumed",
      actions: [],
    });
  });

  it("freezes Shift tap as zero keydown plus the exact 12-action live-option effect", () => {
    const keydown = mapWeb06KeyboardEvent(
      keyEvent("keydown", "Shift", "ShiftLeft", { shiftKey: true }),
      context(),
    );
    expect(keydown).toMatchObject({
      classification: "frontend-consumed",
      preventDefault: true,
      actions: [],
      shiftEffect: "start",
    });

    const keyup = mapWeb06KeyboardEvent(
      keyEvent("keyup", "Shift", "ShiftLeft"),
      context({ pendingAsciiModeShift: "ShiftLeft" }),
    );
    expect(keyup.actions.map(action => [action.name, action.args])).toEqual([
      ["setOption", ["soft_cursor", true]],
      ["setOption", ["ascii_mode", true]],
      ["setOption", ["full_shape", false]],
      ["setOption", ["traditionalization", false]],
      ["setOption", ["variants_hk", true]],
      ["setOption", ["trad_tw", false]],
      ["setOption", ["simplification", false]],
      ["setOption", ["zh_hans", false]],
      ["setOption", ["zh_hant_hk", false]],
      ["setOption", ["zh_hant_tw", false]],
      ["setOption", ["extended_charset", false]],
      ["setOption", ["disabled", false]],
    ]);
    expect(keyup.actions).toHaveLength(12);
    expect(keyup.actions.every(action =>
      action.deferred === true
      && action.actionClass === "stateful-barrier"
      && action.boundary === "modifier-release"
    )).toBe(true);

    expect(mapWeb06KeyboardEvent(
      keyEvent("keyup", "Shift", "ShiftLeft"),
      context({
        pendingAsciiModeShift: "ShiftLeft",
        pendingAsciiModeShiftWasChorded: true,
      }),
    )).toMatchObject({ actions: [], shiftEffect: "finish-without-toggle" });
  });

  it("keeps captured-state and current-state timing distinctions from the production handler", () => {
    const printable = keyEvent("keydown", "a", "KeyA");
    expect(mapWeb06KeyboardEvent(printable, context({
      capturedHasComposition: false,
      currentHasComposition: true,
      isAsciiMode: true,
    })).actions).toEqual([]);

    const digit = keyEvent("keydown", "1", "Digit1");
    expect(mapWeb06KeyboardEvent(digit, context({
      capturedHasComposition: false,
      currentHasComposition: true,
    })).actions[0]).toMatchObject({ name: "selectCandidate", args: [0] });

    const modifierRelease = keyEvent("keyup", "Control", "ControlLeft");
    expect(mapWeb06KeyboardEvent(modifierRelease, context({
      capturedHasComposition: false,
      currentHasComposition: true,
    })).actions).toEqual([]);
  });

  it("keeps off/minimal/full event decisions and public action arguments identical", () => {
    const event = keyEvent("keydown", "Backspace", "Backspace");
    const decisions = (["off", "minimal", "full"] as const).map(mode => ({
      mode,
      decision: mapWeb06KeyboardEvent(event, context()),
    }));
    expect(decisions.map(({ decision }) => decision)).toEqual([
      decisions[0].decision,
      decisions[0].decision,
      decisions[0].decision,
    ]);
  });

  it("bounds receipts, reports overflow, and retains deterministic order", () => {
    const ring = new BoundedReceiptMap<{ identity: Web06ActionIdentity; value: string }>(2);
    const identity = (sequenceId: number): Web06ActionIdentity => ({
      protocolVersion: "web06-private-v1",
      actionId: `a${sequenceId}`,
      sequenceId,
      compositionEpochId: 1,
      supersessionSubRunId: 1,
      actionClass: "native-key",
      supersedable: true,
      boundary: "none",
      rawInputSequence: [String(sequenceId)],
      originKind: "dom-event",
      originReason: "test",
      actionEnqueuedAt: sequenceId,
      mainQueueDepthAtEnqueue: 0,
    });
    expect(ring.set({ identity: identity(1), value: "one" })).toBe(false);
    expect(ring.set({ identity: identity(2), value: "two" })).toBe(false);
    expect(ring.set({ identity: identity(3), value: "three" })).toBe(true);
    expect(ring.values().map(receipt => receipt.value)).toEqual(["two", "three"]);
  });

  it("uses stable pointer-free digests", () => {
    expect(web06StableDigest({ b: 2, a: [1, "x"] })).toBe(
      web06StableDigest({ a: [1, "x"], b: 2 }),
    );
  });

  it("redacts every free-form action path without hashing or retaining source text", () => {
    const importSecret = "TOP-SECRET-userdb-row\tabc\tc=1";
    const customizeValueSecret = "TOP-SECRET-customize-value";
    const dictionarySecrets = ["TOP-SECRET-dictionary-one", "TOP-SECRET-dictionary-two"];
    const receipts = (["off", "minimal", "full"] as const).map(mode => ({
      mode,
      importArgs: web06PrivateActionArgs("importUserdb", [importSecret]),
      customizeValueArgs: web06PrivateActionArgs("customizeValue", [
        "default",
        "translator/example",
        customizeValueSecret,
      ]),
      customizeArgs: web06PrivateActionArgs("customize", [{
        enableLearning: true,
        dictionaryExclude: dictionarySecrets,
      }]),
    }));
    const exported = JSON.stringify(receipts);
    for (const secret of [importSecret, customizeValueSecret, ...dictionarySecrets]) {
      expect(exported).not.toContain(secret);
    }
    const normalizedReceipts = receipts.map(({ mode: _mode, ...receipt }) => receipt);
    expect(normalizedReceipts).toEqual([
      normalizedReceipts[0],
      normalizedReceipts[0],
      normalizedReceipts[0],
    ]);
    expect(receipts[0]).toMatchObject({
      importArgs: [WEB06_REDACTED_USERDB_TEXT],
      customizeValueArgs: ["default", "translator/example", WEB06_REDACTED_CUSTOMIZE_VALUE],
      customizeArgs: [{
        enableLearning: true,
        dictionaryExclude: {
          kind: WEB06_REDACTED_DICTIONARY_EXCLUDE,
          count: 2,
        },
      }],
    });
  });

  it("requires finite monotonic response, schedule, commit, rAF, and observation stamps", () => {
    expect(web06TimestampsAreOrdered(1, 2, 2, 3, 4)).toBe(true);
    expect(web06TimestampsAreOrdered(1, 3, undefined, 4, 5)).toBe(true);
    expect(web06TimestampsAreOrdered(1, 3, 2, 4, 5)).toBe(false);
    expect(web06TimestampsAreOrdered(1, Number.NaN, 3)).toBe(false);
  });

  it("requires the complete private identity to round-trip unchanged", () => {
    const identity: Web06ActionIdentity = {
      protocolVersion: "web06-private-v1",
      actionId: "web06-action-00000001",
      sequenceId: 1,
      eventId: "web06-event-00000001",
      eventSequenceId: 1,
      eventActionIndex: 0,
      compositionEpochId: 2,
      supersessionSubRunId: 3,
      actionClass: "native-key",
      supersedable: true,
      boundary: "none",
      rawInputSequence: ["n"],
      originKind: "dom-event",
      originReason: "printable-key",
      actionEnqueuedAt: 10,
      mainQueueDepthAtEnqueue: 1,
      workerSentAt: 11,
      workerDispatchDepth: 1,
    };
    expect(web06ActionIdentitiesEqual(identity, structuredClone(identity))).toBe(true);
    expect(web06ActionIdentitiesEqual(identity, { ...identity, boundary: "commit" })).toBe(false);
    expect(web06ActionIdentitiesEqual(identity, { ...identity, workerSentAt: 12 })).toBe(false);
  });

  it("signals the FIFO import continuation synchronously after the real enqueue", async () => {
    const order: string[] = [];
    const publicRawText = "fixed-empty-userdb\n";
    const { result, signalAccepted } = web06EnqueueThenSignal(
      () => {
        order.push(`enqueue:${publicRawText}`);
        return Promise.resolve("unchanged-result");
      },
      () => {
        order.push("marker");
        order.push("external-listener-dispatches-real-key");
        return true;
      },
    );
    expect(order).toEqual([
      `enqueue:${publicRawText}`,
      "marker",
      "external-listener-dispatches-real-key",
    ]);
    expect(signalAccepted).toBe(true);
    await expect(result).resolves.toBe("unchanged-result");
  });

  it("keeps actual WEB-06 debug exports private while worker envelopes retain public args", async () => {
    const importSecret = "DEBUG-EXPORT-SECRET-userdb";
    const customizeValueSecret = "DEBUG-EXPORT-SECRET-customize-value";
    const dictionarySecret = "DEBUG-EXPORT-SECRET-dictionary";
    const sent: unknown[] = [];
    let clock = 10;
    const now = vi.spyOn(performance, "now").mockImplementation(() => clock);
    class FakeWorker {
      addEventListener() {}
      postMessage(message: unknown) {
        sent.push(message);
      }
    }
    const fakeWindow = {
      localStorage: { getItem: () => null },
      sessionStorage: {},
      location: { reload: () => undefined },
    };
    vi.stubGlobal("window", fakeWindow);
    vi.stubGlobal("location", { search: "?yuneWeb06Mode=minimal", hostname: "localhost" });
    vi.stubGlobal("document", {
      documentElement: { dataset: {} },
      activeElement: null,
    });
    vi.stubGlobal("Worker", FakeWorker);

    const rime = await import("../src/rime.js");
    const control = (timeStamp: number) => ({
      type: "change",
      timeStamp,
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      shiftKey: false,
    });
    const runControlAction = (
      owner: string,
      name: keyof Actions,
      args: unknown[],
      timeStamp: number,
    ) => {
      let eventIdentity;
      rime.withWeb06ControlEvent(control(timeStamp), () => {
        eventIdentity = rime.declareWeb06ControlFanout("privacy-test", [{
          owner,
          action: web06PrivateAction(name, args),
        }]);
      });
      rime.withWeb06OwnedAction(
        owner,
        name,
        args,
        "privacy-test",
        undefined,
        () => rime.default[name](...args as never),
      );
      return eventIdentity;
    };

    let importEvent;
    rime.withWeb06ControlEvent({
      get nativeEvent() {
        clock = 20;
        return control(1);
      },
    }, () => {
      importEvent = rime.declareWeb06ControlFanout("privacy-test-import", [{
        owner: "privacy-import",
        action: web06PrivateAction("importUserdb", ["<pending-file-text>"]),
      }]);
    });
    expect(importEvent).toBeDefined();
    rime.resolveWeb06DeferredFanoutAction(importEvent!, 0, [importSecret]);
    rime.withWeb06OwnedAction(
      "privacy-import",
      "importUserdb",
      [importSecret],
      "privacy-test",
      undefined,
      () => rime.default.importUserdb(importSecret),
    );

    clock = 30;
    runControlAction(
      "privacy-customize-value",
      "customizeValue",
      ["default", "translator/example", customizeValueSecret],
      2,
    );
    clock = 40;
    runControlAction(
      "privacy-customize",
      "customize",
      [{ enableLearning: true, dictionaryExclude: [dictionarySecret] }],
      3,
    );

    const debug = (fakeWindow as typeof fakeWindow & {
      __YUNE_WEB06__: {
        events(): unknown[];
        actions(): unknown[];
        invalidations(): unknown[];
      };
    }).__YUNE_WEB06__;
    const exported = JSON.stringify({
      events: debug.events(),
      actions: debug.actions(),
      invalidations: debug.invalidations(),
    });
    for (const secret of [importSecret, customizeValueSecret, dictionarySecret]) {
      expect(exported).not.toContain(secret);
    }
    expect(debug.events()[0]).toMatchObject({ identity: { eventDeliveredAt: 10 } });
    expect(sent[0]).toMatchObject({ name: "importUserdb", args: [importSecret] });

    now.mockRestore();
    vi.unstubAllGlobals();
  });
});

function web06PrivateAction(name: keyof Actions, args: unknown[]) {
  const contract = web06ActionContract(name);
  return {
    name,
    args,
    actionClass: contract.actionClass,
    supersedable: false,
    boundary: contract.boundary,
    deferred: true,
  };
}
