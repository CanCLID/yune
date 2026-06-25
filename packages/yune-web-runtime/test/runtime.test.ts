import { describe, expect, it } from "vitest";

import { keyEventToRimeKey, RIME_KEY, RIME_MASK } from "../src/keys.js";
import { bindYuneWebModule, YUNE_WEB_EXPORTS, YuneWebBindingError } from "../src/module.js";
import { YuneWebLifecycleError, YuneWebRuntime } from "../src/runtime.js";
import { FakeYuneWebModule } from "./fake-module.js";

const statePtr = 42;
const defaultInitPtr = 1;

function responsePayload(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    handled: true,
    commits: ["你"],
    context: null,
    status: null,
    ...overrides,
  };
}

function initializedRuntime(fake = new FakeYuneWebModule()): YuneWebRuntime {
  return YuneWebRuntime.init(fake, {
    sharedDataDir: "/rime/shared",
    userDataDir: "/rime/user",
    schemaId: "yune_web_luna",
  });
}

describe("bindYuneWebModule", () => {
  it("binds all canonical YuneWeb exports", () => {
    const cwrapped: string[] = [];
    const fake = new FakeYuneWebModule();
    const original = fake.cwrap.bind(fake);
    fake.cwrap = (ident, returnType, argTypes) => {
      cwrapped.push(ident);
      return original(ident, returnType, argTypes);
    };

    bindYuneWebModule(fake);

    expect(cwrapped).toEqual([...YUNE_WEB_EXPORTS]);
  });

  it("bound functions call the exact fake exports", () => {
    const fake = new FakeYuneWebModule();
    const responsePtr = fake.response(responsePayload());
    fake.processKeyResult = responsePtr;
    const bindings = bindYuneWebModule(fake);

    expect(bindings.init("/shared", "/user", "schema")).toBe(1);
    expect(bindings.processKey(7, 65, 4)).toBe(responsePtr);
    bindings.cleanup(7);
    bindings.freeResponse(responsePtr);

    expect(fake.calls("yune_web_init")).toEqual([["/shared", "/user", "schema"]]);
    expect(fake.calls("yune_web_process_key")).toEqual([[7, 65, 4]]);
    expect(fake.calls("yune_web_cleanup")).toEqual([[7]]);
    expect(fake.calls("yune_web_free_response")).toEqual([[responsePtr]]);
  });

  it("surfaces missing exports as YuneWebBindingError", () => {
    const fake = new FakeYuneWebModule();
    fake.remove("yune_web_process_key");

    expect(() => bindYuneWebModule(fake)).toThrow(YuneWebBindingError);
    expect(() => bindYuneWebModule(fake)).toThrow(
      "Missing YuneWeb export: yune_web_process_key",
    );
  });

  it("exposes boolean-ish C returns as numeric values", () => {
    const fake = new FakeYuneWebModule();
    fake.deployResult = 0;
    fake.customizeResult = 2;
    fake.setOptionResult = 1;
    fake.setAiEnabledResult = 1;
    const bindings = bindYuneWebModule(fake);

    expect(bindings.deploy(1)).toBe(0);
    expect(bindings.customize(1, "yune_web_luna.schema", "schema/name", "YuneWeb")).toBe(2);
    expect(bindings.setOption(1, "ascii_mode", 1)).toBe(1);
    expect(bindings.setAiEnabled(1, 1)).toBe(1);
  });
});

describe("YuneWebRuntime operations", () => {
  it("initializes with shared/user directories and schema id", () => {
    const fake = new FakeYuneWebModule();
    const runtime = initializedRuntime(fake);

    expect(runtime).toBeInstanceOf(YuneWebRuntime);
    expect(fake.calls("yune_web_init")).toEqual([
      ["/rime/shared", "/rime/user", "yune_web_luna"],
    ]);
  });

  it("throws on null init pointer without exposing a runtime", () => {
    const fake = new FakeYuneWebModule();
    fake.initResult = 0;

    expect(() =>
      YuneWebRuntime.init(fake, {
        sharedDataDir: "/rime/shared",
        userDataDir: "/rime/user",
        schemaId: "yune_web_luna",
      }),
    ).toThrow(new YuneWebLifecycleError("YuneWeb adapter init failed"));
  });

  it("processKey forwards state, keycode, and explicit mask and frees the response", () => {
    const fake = new FakeYuneWebModule();
    const responsePtr = fake.response(responsePayload({ commits: ["吧"] }));
    fake.processKeyResult = responsePtr;
    const runtime = initializedRuntime(fake);

    expect(runtime.processKey(0x62, 4)).toEqual(responsePayload({ commits: ["吧"] }));
    expect(fake.calls("yune_web_process_key")).toEqual([[defaultInitPtr, 0x62, 4]]);
    expect(fake.freedResponses()).toEqual([responsePtr]);
  });

  it("processKey defaults mask to zero", () => {
    const fake = new FakeYuneWebModule();
    fake.processKeyResult = fake.response(responsePayload());
    const runtime = initializedRuntime(fake);

    runtime.processKey(0x61);

    expect(fake.calls("yune_web_process_key")).toEqual([[defaultInitPtr, 0x61, 0]]);
  });

  it("processKeyboardEvent maps through keyEventToRimeKey before processing", () => {
    const fake = new FakeYuneWebModule();
    fake.processKeyResult = fake.response(responsePayload());
    const runtime = initializedRuntime(fake);
    const mapped = keyEventToRimeKey({ key: "A", shiftKey: true });

    runtime.processKeyboardEvent({ key: "A", shiftKey: true });

    expect(fake.calls("yune_web_process_key")).toEqual([
      [defaultInitPtr, mapped.keycode, mapped.mask],
    ]);
  });

  it("processKeyboardEvent forwards browser metaKey as the supported Super mask", () => {
    const fake = new FakeYuneWebModule();
    fake.processKeyResult = fake.response(responsePayload());
    const runtime = initializedRuntime(fake);
    const mapped = keyEventToRimeKey({ key: "x", metaKey: true });

    runtime.processKeyboardEvent({ key: "x", metaKey: true });

    expect(fake.calls("yune_web_process_key")).toEqual([
      [defaultInitPtr, mapped.keycode, mapped.mask],
    ]);
  });

  it("processKeyboardEvent forwards modifier-only press and release events", () => {
    const fake = new FakeYuneWebModule();
    const shiftDown = fake.response(responsePayload());
    const shiftUp = fake.response(responsePayload());
    fake.processKeyResult = shiftDown;
    const runtime = initializedRuntime(fake);

    runtime.processKeyboardEvent({ key: "Shift", shiftKey: true });
    fake.processKeyResult = shiftUp;
    runtime.processKeyboardEvent({ key: "Shift", shiftKey: true, type: "keyup" });

    expect(fake.calls("yune_web_process_key")).toEqual([
      [defaultInitPtr, RIME_KEY.Shift, 0],
      [defaultInitPtr, RIME_KEY.Shift, RIME_MASK.Release],
    ]);
  });

  it("forwards candidate actions and frees their responses", () => {
    const fake = new FakeYuneWebModule();
    const selected = fake.response(responsePayload({ commits: ["选"] }));
    const deleted = fake.response(responsePayload({ commits: [] }));
    fake.selectCandidateResult = selected;
    fake.deleteCandidateResult = deleted;
    const runtime = initializedRuntime(fake);

    expect(runtime.selectCandidate(3).commits).toEqual(["选"]);
    expect(runtime.deleteCandidate(2).commits).toEqual([]);

    expect(fake.calls("yune_web_select_candidate")).toEqual([[defaultInitPtr, 3]]);
    expect(fake.calls("yune_web_delete_candidate")).toEqual([[defaultInitPtr, 2]]);
    expect(fake.freedResponses()).toEqual([selected, deleted]);
  });

  it("forwards page direction as adapter booleans", () => {
    const fake = new FakeYuneWebModule();
    const forward = fake.response(responsePayload());
    const backward = fake.response(responsePayload());
    fake.flipPageResult = forward;
    const runtime = initializedRuntime(fake);

    runtime.flipPage(false);
    fake.flipPageResult = backward;
    runtime.flipPage(true);

    expect(fake.calls("yune_web_flip_page")).toEqual([
      [defaultInitPtr, 0],
      [defaultInitPtr, 1],
    ]);
    expect(fake.freedResponses()).toEqual([forward, backward]);
  });

  it("returns booleans from deploy and customize numeric adapter returns", () => {
    const fake = new FakeYuneWebModule();
    fake.deployResult = 1;
    fake.customizeResult = 0;
    fake.setOptionResult = 1;
    fake.setAiEnabledResult = 1;
    const runtime = initializedRuntime(fake);

    expect(runtime.deploy()).toBe(true);
    expect(runtime.customize("yune_web_luna.schema", "schema/name", "YuneWeb Luna Web")).toBe(false);
    expect(runtime.setOption("ascii_mode", true)).toBe(true);
    expect(runtime.setAiEnabled(true)).toBe(true);

    expect(fake.calls("yune_web_deploy")).toEqual([[defaultInitPtr]]);
    expect(fake.calls("yune_web_customize")).toEqual([
      [defaultInitPtr, "yune_web_luna.schema", "schema/name", "YuneWeb Luna Web"],
    ]);
    expect(fake.calls("yune_web_set_option")).toEqual([[defaultInitPtr, "ascii_mode", 1]]);
    expect(fake.calls("yune_web_set_ai_enabled")).toEqual([[defaultInitPtr, 1]]);
  });

  it("stageAi returns the second-pass response and frees it", () => {
    const fake = new FakeYuneWebModule();
    const staged = fake.response(responsePayload({ commits: [], context: null }));
    fake.stageAiResult = staged;
    const runtime = initializedRuntime(fake);

    expect(runtime.stageAi()).toEqual(responsePayload({ commits: [], context: null }));

    expect(fake.calls("yune_web_stage_ai")).toEqual([[defaultInitPtr]]);
    expect(fake.freedResponses()).toEqual([staged]);
  });
});

describe("YuneWebRuntime lifecycle", () => {
  it("cleanup calls the adapter once and is idempotent", () => {
    const fake = new FakeYuneWebModule();
    const runtime = initializedRuntime(fake);

    runtime.cleanup();
    runtime.cleanup();

    expect(fake.calls("yune_web_cleanup")).toEqual([[defaultInitPtr]]);
  });

  it("non-cleanup methods throw after cleanup", () => {
    const fake = new FakeYuneWebModule();
    const runtime = initializedRuntime(fake);
    runtime.cleanup();

    const cleanedUpMessage = "YuneWeb runtime has been cleaned up";
    expect(() => runtime.processKey(0x61)).toThrow(cleanedUpMessage);
    expect(() => runtime.processKeyboardEvent({ key: "a" })).toThrow(cleanedUpMessage);
    expect(() => runtime.selectCandidate(0)).toThrow(cleanedUpMessage);
    expect(() => runtime.deleteCandidate(0)).toThrow(cleanedUpMessage);
    expect(() => runtime.flipPage()).toThrow(cleanedUpMessage);
    expect(() => runtime.deploy()).toThrow(cleanedUpMessage);
    expect(() => runtime.customize("config", "key", "value")).toThrow(cleanedUpMessage);
    expect(() => runtime.setOption("ascii_mode", true)).toThrow(cleanedUpMessage);
    expect(() => runtime.setAiEnabled(true)).toThrow(cleanedUpMessage);
    expect(() => runtime.stageAi()).toThrow(cleanedUpMessage);

    expect(fake.calls("yune_web_process_key")).toEqual([]);
    expect(fake.calls("yune_web_select_candidate")).toEqual([]);
    expect(fake.calls("yune_web_delete_candidate")).toEqual([]);
    expect(fake.calls("yune_web_flip_page")).toEqual([]);
    expect(fake.calls("yune_web_deploy")).toEqual([]);
    expect(fake.calls("yune_web_customize")).toEqual([]);
    expect(fake.calls("yune_web_set_option")).toEqual([]);
    expect(fake.calls("yune_web_set_ai_enabled")).toEqual([]);
    expect(fake.calls("yune_web_stage_ai")).toEqual([]);
  });

  it("failed init cannot be cleaned up because no runtime object is returned", () => {
    const fake = new FakeYuneWebModule();
    fake.initResult = 0;

    expect(() => initializedRuntime(fake)).toThrow("YuneWeb adapter init failed");
    expect(fake.calls("yune_web_cleanup")).toEqual([]);
  });
});
