import { describe, expect, it } from "vitest";

import { keyEventToRimeKey } from "../src/keys";
import { bindTypeDuckModule, TYPEDUCK_EXPORTS, TypeDuckBindingError } from "../src/module";
import { TypeDuckLifecycleError, TypeDuckRuntime } from "../src/typeduck";
import { FakeTypeDuckModule } from "./fake-module";

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

function initializedRuntime(fake = new FakeTypeDuckModule()): TypeDuckRuntime {
  return TypeDuckRuntime.init(fake, {
    sharedDataDir: "/rime/shared",
    userDataDir: "/rime/user",
    schemaId: "typeduck_luna",
  });
}

describe("bindTypeDuckModule", () => {
  it("binds all canonical TypeDuck exports", () => {
    const cwrapped: string[] = [];
    const fake = new FakeTypeDuckModule();
    const original = fake.cwrap.bind(fake);
    fake.cwrap = (ident, returnType, argTypes) => {
      cwrapped.push(ident);
      return original(ident, returnType, argTypes);
    };

    bindTypeDuckModule(fake);

    expect(cwrapped).toEqual([...TYPEDUCK_EXPORTS]);
  });

  it("bound functions call the exact fake exports", () => {
    const fake = new FakeTypeDuckModule();
    const responsePtr = fake.response(responsePayload());
    fake.processKeyResult = responsePtr;
    const bindings = bindTypeDuckModule(fake);

    expect(bindings.init("/shared", "/user", "schema")).toBe(1);
    expect(bindings.processKey(7, 65, 4)).toBe(responsePtr);
    bindings.cleanup(7);
    bindings.freeResponse(responsePtr);

    expect(fake.calls("yune_typeduck_init")).toEqual([["/shared", "/user", "schema"]]);
    expect(fake.calls("yune_typeduck_process_key")).toEqual([[7, 65, 4]]);
    expect(fake.calls("yune_typeduck_cleanup")).toEqual([[7]]);
    expect(fake.calls("yune_typeduck_free_response")).toEqual([[responsePtr]]);
  });

  it("surfaces missing exports as TypeDuckBindingError", () => {
    const fake = new FakeTypeDuckModule();
    fake.remove("yune_typeduck_process_key");

    expect(() => bindTypeDuckModule(fake)).toThrow(TypeDuckBindingError);
    expect(() => bindTypeDuckModule(fake)).toThrow(
      "Missing TypeDuck export: yune_typeduck_process_key",
    );
  });

  it("exposes boolean-ish C returns as numeric values", () => {
    const fake = new FakeTypeDuckModule();
    fake.deployResult = 0;
    fake.customizeResult = 2;
    const bindings = bindTypeDuckModule(fake);

    expect(bindings.deploy(1)).toBe(0);
    expect(bindings.customize(1, "typeduck_luna.schema", "schema/name", "TypeDuck")).toBe(2);
  });
});

describe("TypeDuckRuntime operations", () => {
  it("initializes with shared/user directories and schema id", () => {
    const fake = new FakeTypeDuckModule();
    const runtime = initializedRuntime(fake);

    expect(runtime).toBeInstanceOf(TypeDuckRuntime);
    expect(fake.calls("yune_typeduck_init")).toEqual([
      ["/rime/shared", "/rime/user", "typeduck_luna"],
    ]);
  });

  it("throws on null init pointer without exposing a runtime", () => {
    const fake = new FakeTypeDuckModule();
    fake.initResult = 0;

    expect(() =>
      TypeDuckRuntime.init(fake, {
        sharedDataDir: "/rime/shared",
        userDataDir: "/rime/user",
        schemaId: "typeduck_luna",
      }),
    ).toThrow(new TypeDuckLifecycleError("TypeDuck adapter init failed"));
  });

  it("processKey forwards state, keycode, and explicit mask and frees the response", () => {
    const fake = new FakeTypeDuckModule();
    const responsePtr = fake.response(responsePayload({ commits: ["吧"] }));
    fake.processKeyResult = responsePtr;
    const runtime = initializedRuntime(fake);

    expect(runtime.processKey(0x62, 4)).toEqual(responsePayload({ commits: ["吧"] }));
    expect(fake.calls("yune_typeduck_process_key")).toEqual([[defaultInitPtr, 0x62, 4]]);
    expect(fake.freedResponses()).toEqual([responsePtr]);
  });

  it("processKey defaults mask to zero", () => {
    const fake = new FakeTypeDuckModule();
    fake.processKeyResult = fake.response(responsePayload());
    const runtime = initializedRuntime(fake);

    runtime.processKey(0x61);

    expect(fake.calls("yune_typeduck_process_key")).toEqual([[defaultInitPtr, 0x61, 0]]);
  });

  it("processKeyboardEvent maps through keyEventToRimeKey before processing", () => {
    const fake = new FakeTypeDuckModule();
    fake.processKeyResult = fake.response(responsePayload());
    const runtime = initializedRuntime(fake);
    const mapped = keyEventToRimeKey({ key: "A", shiftKey: true });

    runtime.processKeyboardEvent({ key: "A", shiftKey: true });

    expect(fake.calls("yune_typeduck_process_key")).toEqual([
      [defaultInitPtr, mapped.keycode, mapped.mask],
    ]);
  });

  it("forwards candidate actions and frees their responses", () => {
    const fake = new FakeTypeDuckModule();
    const selected = fake.response(responsePayload({ commits: ["选"] }));
    const deleted = fake.response(responsePayload({ commits: [] }));
    fake.selectCandidateResult = selected;
    fake.deleteCandidateResult = deleted;
    const runtime = initializedRuntime(fake);

    expect(runtime.selectCandidate(3).commits).toEqual(["选"]);
    expect(runtime.deleteCandidate(2).commits).toEqual([]);

    expect(fake.calls("yune_typeduck_select_candidate")).toEqual([[defaultInitPtr, 3]]);
    expect(fake.calls("yune_typeduck_delete_candidate")).toEqual([[defaultInitPtr, 2]]);
    expect(fake.freedResponses()).toEqual([selected, deleted]);
  });

  it("forwards page direction as adapter booleans", () => {
    const fake = new FakeTypeDuckModule();
    const forward = fake.response(responsePayload());
    const backward = fake.response(responsePayload());
    fake.flipPageResult = forward;
    const runtime = initializedRuntime(fake);

    runtime.flipPage(false);
    fake.flipPageResult = backward;
    runtime.flipPage(true);

    expect(fake.calls("yune_typeduck_flip_page")).toEqual([
      [defaultInitPtr, 0],
      [defaultInitPtr, 1],
    ]);
    expect(fake.freedResponses()).toEqual([forward, backward]);
  });

  it("returns booleans from deploy and customize numeric adapter returns", () => {
    const fake = new FakeTypeDuckModule();
    fake.deployResult = 1;
    fake.customizeResult = 0;
    const runtime = initializedRuntime(fake);

    expect(runtime.deploy()).toBe(true);
    expect(runtime.customize("typeduck_luna.schema", "schema/name", "TypeDuck Luna Web")).toBe(false);

    expect(fake.calls("yune_typeduck_deploy")).toEqual([[defaultInitPtr]]);
    expect(fake.calls("yune_typeduck_customize")).toEqual([
      [defaultInitPtr, "typeduck_luna.schema", "schema/name", "TypeDuck Luna Web"],
    ]);
  });
});

describe("TypeDuckRuntime lifecycle", () => {
  it("cleanup calls the adapter once and is idempotent", () => {
    const fake = new FakeTypeDuckModule();
    const runtime = initializedRuntime(fake);

    runtime.cleanup();
    runtime.cleanup();

    expect(fake.calls("yune_typeduck_cleanup")).toEqual([[defaultInitPtr]]);
  });

  it("non-cleanup methods throw after cleanup", () => {
    const fake = new FakeTypeDuckModule();
    const runtime = initializedRuntime(fake);
    runtime.cleanup();

    const cleanedUpMessage = "TypeDuck runtime has been cleaned up";
    expect(() => runtime.processKey(0x61)).toThrow(cleanedUpMessage);
    expect(() => runtime.processKeyboardEvent({ key: "a" })).toThrow(cleanedUpMessage);
    expect(() => runtime.selectCandidate(0)).toThrow(cleanedUpMessage);
    expect(() => runtime.deleteCandidate(0)).toThrow(cleanedUpMessage);
    expect(() => runtime.flipPage()).toThrow(cleanedUpMessage);
    expect(() => runtime.deploy()).toThrow(cleanedUpMessage);
    expect(() => runtime.customize("config", "key", "value")).toThrow(cleanedUpMessage);

    expect(fake.calls("yune_typeduck_process_key")).toEqual([]);
    expect(fake.calls("yune_typeduck_select_candidate")).toEqual([]);
    expect(fake.calls("yune_typeduck_delete_candidate")).toEqual([]);
    expect(fake.calls("yune_typeduck_flip_page")).toEqual([]);
    expect(fake.calls("yune_typeduck_deploy")).toEqual([]);
    expect(fake.calls("yune_typeduck_customize")).toEqual([]);
  });

  it("failed init cannot be cleaned up because no runtime object is returned", () => {
    const fake = new FakeTypeDuckModule();
    fake.initResult = 0;

    expect(() => initializedRuntime(fake)).toThrow("TypeDuck adapter init failed");
    expect(fake.calls("yune_typeduck_cleanup")).toEqual([]);
  });
});
