import { describe, expect, it } from "vitest";

import { bindYuneWebModule } from "../src/module.js";
import { readYuneWebResponse, YuneWebResponseError } from "../src/response.js";
import { FakeYuneWebModule } from "./fake-module.js";

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
      schema_id: "yune_web_luna",
      schema_name: "YuneWeb Luna",
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

function bindings(fake: FakeYuneWebModule) {
  return bindYuneWebModule(fake);
}

describe("readYuneWebResponse", () => {
  it("parses a valid adapter response object", () => {
    const fake = new FakeYuneWebModule();
    const ptr = fake.response(responsePayload({ error: "visible adapter error" }), true);

    expect(readYuneWebResponse(ptr, bindings(fake))).toEqual(
      responsePayload({ error: "visible adapter error" }),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("parses optional candidate source labels", () => {
    const fake = new FakeYuneWebModule();
    const payload = responsePayload({
      context: {
        ...responsePayload().context,
        candidates: [
          { text: "你", comment: "" },
          { text: "你啊", comment: "ai:local-model 0.83", source: "ai:local" },
        ],
      },
    });
    const ptr = fake.response(payload, true);

    expect(readYuneWebResponse(ptr, bindings(fake))).toEqual(payload);
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("parses opt-in inspector debug fields", () => {
    const fake = new FakeYuneWebModule();
    const payload = responsePayload({
      context: {
        ...responsePayload().context,
        candidates: [
          {
            text: "nei",
            comment: "nei",
            source: "table",
            quality: 10,
            preedit: "nei",
          },
          {
            text: "nei aa",
            comment: "ai",
            source: "ai:local",
            quality: 0.83,
            ai_confidence: 0.83,
          },
        ],
        debug: {
          segment_tags: ["abc"],
          segments: [{ start: 0, end: 3, tag: "abc", source: "context.segment_tags" }],
          filter_pipeline: ["uniquifier"],
          filter_audit: [{ name: "uniquifier", before_count: 3, after_count: 2 }],
          spelling_algebra: [
            {
              translator: "static_table_translator",
              input: "nei",
              lookup_code: "nei",
              formulas: ["derive/\\d//"],
              expanded_codes: ["nei"],
            },
          ],
          prediction: {
            weight_threshold: 0,
            candidates: [
              {
                index: 0,
                text: "nei",
                source: "table",
                quality: 10,
                threshold: 0,
                above_threshold: true,
              },
            ],
          },
          ai_staging: { state: "off", for_input: null },
        },
      },
    });
    const ptr = fake.response(payload, true);

    expect(readYuneWebResponse(ptr, bindings(fake))).toEqual(payload);
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("treats null candidate source labels as classic candidates", () => {
    const fake = new FakeYuneWebModule();
    const payload = responsePayload({
      context: {
        ...responsePayload().context,
        candidates: [
          { text: "ä½ ", comment: "", source: null },
          { text: "ä½ å•Š", comment: "ai:local-model 0.83", source: "ai:local" },
        ],
      },
    });
    const ptr = fake.response(payload, true);

    expect(readYuneWebResponse(ptr, bindings(fake)).context?.candidates).toEqual([
      { text: "ä½ ", comment: "" },
      { text: "ä½ å•Š", comment: "ai:local-model 0.83", source: "ai:local" },
    ]);
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("uses response_handled as the authoritative handled value", () => {
    const fake = new FakeYuneWebModule();
    const ptr = fake.response(responsePayload({ handled: false }), true);

    expect(readYuneWebResponse(ptr, bindings(fake)).handled).toBe(true);
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws for null response pointer and does not free pointer zero", () => {
    const fake = new FakeYuneWebModule();

    expect(() => readYuneWebResponse(0, bindings(fake))).toThrow(
      new YuneWebResponseError("YuneWeb adapter returned null response"),
    );
    expect(fake.freedResponses()).toEqual([]);
  });

  it("throws for null response JSON and still frees the response", () => {
    const fake = new FakeYuneWebModule();
    const ptr = fake.responseWithJsonPointer(0, true);

    expect(() => readYuneWebResponse(ptr, bindings(fake))).toThrow(
      new YuneWebResponseError("YuneWeb adapter returned null response JSON"),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws a deterministic error for malformed JSON and still frees the response", () => {
    const fake = new FakeYuneWebModule();
    const ptr = fake.responseText("{not json", true);

    let thrown: unknown;
    try {
      readYuneWebResponse(ptr, bindings(fake));
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(YuneWebResponseError);
    expect(thrown).toHaveProperty("message", "YuneWeb adapter returned malformed response JSON");
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws a deterministic error for non-object JSON and still frees the response", () => {
    const fake = new FakeYuneWebModule();
    const ptr = fake.response(["not", "object"], true);

    expect(() => readYuneWebResponse(ptr, bindings(fake))).toThrow(
      new YuneWebResponseError("YuneWeb response must be an object"),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws when handled is not boolean and still frees the response", () => {
    const fake = new FakeYuneWebModule();
    const ptr = fake.response(responsePayload({ handled: "yes" }), true);

    expect(() => readYuneWebResponse(ptr, bindings(fake))).toThrow(
      new YuneWebResponseError("YuneWeb response handled field must be boolean"),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("throws when commits is not an array and still frees the response", () => {
    const fake = new FakeYuneWebModule();
    const ptr = fake.response(responsePayload({ commits: "你" }), true);

    expect(() => readYuneWebResponse(ptr, bindings(fake))).toThrow(
      new YuneWebResponseError("YuneWeb response commits field must be a string array"),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });

  it("allows nullable context and status fields", () => {
    const fake = new FakeYuneWebModule();
    const ptr = fake.response(responsePayload({ context: null, status: null }), true);

    expect(readYuneWebResponse(ptr, bindings(fake))).toEqual(
      responsePayload({ context: null, status: null }),
    );
    expect(fake.freedResponses()).toEqual([ptr]);
  });
});
