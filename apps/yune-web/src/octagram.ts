import type { RimeSchemaId, YuneWebMemorySnapshot } from "./types";

export const OCTAGRAM_MODEL_ID = "zh-hant-t-essay-bgw";
export const OCTAGRAM_MODEL_FILENAME = `${OCTAGRAM_MODEL_ID}.gram`;
export const OCTAGRAM_DEV_MODEL_ASSET_PATH = `dev/octagram/${OCTAGRAM_MODEL_FILENAME}`;
export const OCTAGRAM_MODEL_SHA256 = "574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3";
export const OCTAGRAM_MODEL_BYTES = 10513408;
// Source pin lives in docs/reports/evidence/m54-native-octagram-grammar-support/external-pins.json.

export interface GrammarModelRequest {
  modelId: string;
  expectedSha256: string;
  assetPath: string;
  sharedDataPath: string;
}

export interface GrammarModelDiagnostic {
  requestedSchemaId: RimeSchemaId;
  effectiveSchemaId: RimeSchemaId;
  delivered: boolean;
  modelId: string | null;
  expectedSha256: string | null;
  actualSha256?: string;
  bytes?: number;
  sourcePath?: string;
  fallback: boolean;
  reason?: string;
  memoryBeforeBytes?: number;
  memoryAfterBytes?: number;
  memoryDeltaBytes?: number;
}

export function grammarModelRequestForSchema(schemaId: RimeSchemaId): GrammarModelRequest | null {
  if (schemaId !== "luna_pinyin_octagram") {
    return null;
  }
  return {
    modelId: OCTAGRAM_MODEL_ID,
    expectedSha256: OCTAGRAM_MODEL_SHA256,
    assetPath: OCTAGRAM_DEV_MODEL_ASSET_PATH,
    sharedDataPath: OCTAGRAM_MODEL_FILENAME,
  };
}

export function grammarDiagnosticForSchema(
  schemaId: RimeSchemaId,
  fields: Partial<Omit<GrammarModelDiagnostic, "requestedSchemaId" | "effectiveSchemaId">> & {
    effectiveSchemaId?: RimeSchemaId;
  } = {},
): GrammarModelDiagnostic {
  const request = grammarModelRequestForSchema(schemaId);
  const effectiveSchemaId = fields.effectiveSchemaId ?? schemaId;
  return {
    requestedSchemaId: schemaId,
    effectiveSchemaId,
    delivered: fields.delivered ?? false,
    modelId: fields.modelId ?? request?.modelId ?? null,
    expectedSha256: fields.expectedSha256 ?? request?.expectedSha256 ?? null,
    actualSha256: fields.actualSha256,
    bytes: fields.bytes,
    sourcePath: fields.sourcePath ?? request?.assetPath,
    fallback: fields.fallback ?? false,
    reason: fields.reason,
    memoryBeforeBytes: fields.memoryBeforeBytes,
    memoryAfterBytes: fields.memoryAfterBytes,
    memoryDeltaBytes: fields.memoryDeltaBytes,
  };
}

export function grammarMemoryDelta(
  before: YuneWebMemorySnapshot | undefined,
  after: YuneWebMemorySnapshot | undefined,
): Pick<GrammarModelDiagnostic, "memoryBeforeBytes" | "memoryAfterBytes" | "memoryDeltaBytes"> {
  const memoryBeforeBytes = before?.wasmHeapBytes;
  const memoryAfterBytes = after?.wasmHeapBytes;
  return {
    memoryBeforeBytes,
    memoryAfterBytes,
    memoryDeltaBytes: memoryBeforeBytes === undefined || memoryAfterBytes === undefined
      ? undefined
      : memoryAfterBytes - memoryBeforeBytes,
  };
}
