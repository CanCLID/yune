import { describe, expect, it } from "vitest";

import { bindTypeDuckModule } from "../src/module.js";
import { readTypeDuckResponse, TypeDuckResponseError } from "../src/response.js";
import { FakeTypeDuckModule } from "./fake-module.js";

function responsePayload(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    handled: true,
    commits: ["你"],
    context: {
      input: "ni",
      preedit: "ni",
      caret: 2,
      highlighted: 0,
      page_size: 5,
      page_no: 0,
      is_last_page: false,
      select_keys: "12345",
      select_labels: ["1", "2"],
      candidates: [{ text: "你", comment: "" }],
    },
    status: {
      schema_id: "typeduck_luna",
      schema_name: "TypeDuck Luna",
      is_disabled: false,
      is_composing: true,
      is_ascii_mode: false,
      is_full_shape: false,
      is_simplified: false,
      is_traditional: false,
      is_ascii_punct: false,
    },
    ...overrides,
  };
}

function bindings(fake: FakeTypeDuckModule) {
  return bindTypeDuckModule(fake);
}

describe("readTypeDuckResponse", () => {
  it("parses a valid adapter response object", () => {
    const fake = new FakeTypeDuckModule();
    const ptr = fake.response(responsePayload({ error: "visible adapter error" }), true);

    expect(readTypeDuckResponse(ptr, bindings(fake))).toEqual(
      responsePayload({ error: "visible adapter error" }),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("uses response_handled as the authoritative handled value", () => {
    const fake = new FakeTypeDuckModule();
    const ptr = fake.response(responsePayload({ handled: false }), true);

    expect(readTypeDuckResponse(ptr, bindings(fake)).handled).toBe(true);
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws for null response pointer and does not free pointer zero", () => {
    const fake = new FakeTypeDuckModule();

    expect(() => readTypeDuckResponse(0, bindings(fake))).toThrow(
      new TypeDuckResponseError("TypeDuck adapter returned null response"),
    );
    expect(fake.freedResponses()).toEqual([]);
  });

  it("throws for null response JSON and still frees the response", () => {
    const fake = new FakeTypeDuckModule();
    const ptr = fake.responseWithJsonPointer(0, true);

    expect(() => readTypeDuckResponse(ptr, bindings(fake))).toThrow(
      new TypeDuckResponseError("TypeDuck adapter returned null response JSON"),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws a deterministic error for malformed JSON and still frees the response", () => {
    const fake = new FakeTypeDuckModule();
    const ptr = fake.responseText("{not json", true);

    let thrown: unknown;
    try {
      readTypeDuckResponse(ptr, bindings(fake));
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(TypeDuckResponseError);
    expect(thrown).toHaveProperty("message", "TypeDuck adapter returned malformed response JSON");
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws a deterministic error for non-object JSON and still frees the response", () => {
    const fake = new FakeTypeDuckModule();
    const ptr = fake.response(["not", "object"], true);

    expect(() => readTypeDuckResponse(ptr, bindings(fake))).toThrow(
      new TypeDuckResponseError("TypeDuck response must be an object"),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws when handled is not boolean and still frees the response", () => {
    const fake = new FakeTypeDuckModule();
    const ptr = fake.response(responsePayload({ handled: "yes" }), true);

    expect(() => readTypeDuckResponse(ptr, bindings(fake))).toThrow(
      new TypeDuckResponseError("TypeDuck response handled field must be boolean"),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws when commits is not an array and still frees the response", () => {
    const fake = new FakeTypeDuckModule();
    const ptr = fake.response(responsePayload({ commits: "你" }), true);

    expect(() => readTypeDuckResponse(ptr, bindings(fake))).toThrow(
      new TypeDuckResponseError("TypeDuck response commits field must be a string array"),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("allows nullable context and status fields", () => {
    const fake = new FakeTypeDuckModule();
    const ptr = fake.response(responsePayload({ context: null, status: null }), true);

    expect(readTypeDuckResponse(ptr, bindings(fake))).toEqual(
      responsePayload({ context: null, status: null }),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });
});
