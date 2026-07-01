import { describe, expect, it } from "vitest";

import {
  OCTAGRAM_DEV_MODEL_ASSET_PATH,
  OCTAGRAM_MODEL_ID,
  OCTAGRAM_MODEL_SHA256,
  grammarDiagnosticForSchema,
  grammarModelRequestForSchema,
} from "../src/octagram.js";
import {
  isLunaOutputSchema,
  isRimeSchemaId,
} from "../src/consts.js";

describe("WEB-04 octagram browser profile policy", () => {
  it("keeps the plain Luna profile default-off", () => {
    expect(grammarModelRequestForSchema("luna_pinyin")).toBeNull();
    expect(isLunaOutputSchema("luna_pinyin")).toBe(true);
  });

  it("loads the pinned lotem model only for the dedicated octagram profile", () => {
    expect(grammarModelRequestForSchema("luna_pinyin_octagram")).toEqual({
      modelId: OCTAGRAM_MODEL_ID,
      expectedSha256: OCTAGRAM_MODEL_SHA256,
      assetPath: OCTAGRAM_DEV_MODEL_ASSET_PATH,
      sharedDataPath: `${OCTAGRAM_MODEL_ID}.gram`,
    });
    expect(isLunaOutputSchema("luna_pinyin_octagram")).toBe(true);
  });

  it("accepts the octagram schema id without sweeping arbitrary Luna-family names", () => {
    expect(isRimeSchemaId("luna_pinyin_octagram")).toBe(true);
    expect(isRimeSchemaId("luna_pinyin_experimental")).toBe(false);
  });

  it("reports fail-closed expected model identity in diagnostics", () => {
    expect(grammarDiagnosticForSchema("luna_pinyin_octagram")).toMatchObject({
      requestedSchemaId: "luna_pinyin_octagram",
      effectiveSchemaId: "luna_pinyin_octagram",
      delivered: false,
      modelId: OCTAGRAM_MODEL_ID,
      expectedSha256: OCTAGRAM_MODEL_SHA256,
      fallback: false,
    });
  });
});
