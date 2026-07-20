/**
 * yune-web worker with Yune runtime integration
 *
 * This patch replaces librime/WASM binding with Yune runtime adapter
 * from @yune-ime/yune-web-runtime while preserving Actions interface
 * and message handling logic.
 */

import type {
  Actions,
  ListenerArgsMap,
  RimeResult,
  RimePreferences,
  RimeNotification,
  RimeDeployStatus,
  RimeSchemaId,
  Web06WorkerActionEnvelope,
  Web06WorkerReceipt,
  Web06WorkerRequestEnvelope,
  YuneWebMemorySnapshot,
  YuneWebUserdbParseError,
  YuneWebUserdbRow,
  YuneWebUserdbSnapshot,
} from "./types";
import { WEB06_PRIVATE_PROTOCOL_VERSION } from "./types";
import { web06CollectionMode } from "./yune-integration/private-protocol";
import {
  OCTAGRAM_MODEL_BYTES,
  grammarDiagnosticForSchema,
  grammarMemoryDelta,
  grammarModelRequestForSchema,
  type GrammarModelDiagnostic,
} from "./octagram";

// Yune integration imports
import {
  initYuneRuntime,
  cleanupYuneRuntime,
  processKey,
  stageAi,
  selectCandidate,
  deleteCandidate,
  flipPage,
  deploy as deployYuneRuntime,
  customize,
  customizeValue,
  deployCacheSnapshot,
  invalidateDeployCache,
  setOption,
  type Web06AdapterObservation,
  type YunePersistenceDiagnostic,
} from "./yune-integration/adapter.js";

import {
  loadExplicitAssets,
  loadAssetContent,
  validateExplicitAssets,
  type ExplicitYuneWebAssets,
  type AssetSource,
} from "./yune-integration/assets.js";

import {
  joinYuneWebVirtualPath,
  mountYuneWebPersistence,
  syncAfterUserDataChange,
  type EmscriptenYuneWebModule,
  type YuneWebFilesystem,
} from "@yune-ime/yune-web-runtime";

interface YuneWebBrowserModule extends EmscriptenYuneWebModule {
  FS: YuneWebFilesystem;
  IDBFS: unknown;
  HEAP8?: Int8Array;
  HEAPU8?: Uint8Array;
  wasmMemory?: WebAssembly.Memory;
}

interface CreateYuneWebModuleOptions {
  printErr: (message: string) => void;
  locateFile: (path: string, prefix: string) => string;
  noInitialRun?: boolean;
}

type CreateYuneWebModule = (options: CreateYuneWebModuleOptions) => Promise<YuneWebBrowserModule>;

function isRimeSchemaId(value: string | null): value is RimeSchemaId {
  return value === "jyut6ping3"
    || value === "cangjie5"
    || value === "luna_pinyin"
    || value === "luna_pinyin_octagram";
}

interface PlaygroundSchema {
  runtimeId: RimeSchemaId | "jyut6ping3_mobile";
  name: string;
  dictionaryId: string;
  deployedDefaultPath?: string;
  deployedSchemaPath?: string;
}

interface StartupMarker {
  phase: string;
  ms: number;
  wasmMemory?: StartupWasmMemorySnapshot;
}

interface StartupWasmMemorySnapshot {
  currentBytes: number;
  peakBytes: number;
}

interface PublicAssetManifestEntry {
  path: string;
  sha256: string;
  bytes: number;
  tier: "shared" | "explicit";
  required?: boolean;
}

interface PublicAssetManifest {
  version: string;
  generatedFor: "yune-web";
  assets: PublicAssetManifestEntry[];
}

interface PublicAssetCacheStats {
  hits: number;
  misses: number;
  unavailable: boolean;
}

interface YuneWebFilesystemStat {
  size?: number;
  mtime?: Date | number | string;
  mtimeMs?: number;
}

type YuneWebFilesystemWithStat = YuneWebFilesystem & {
  stat?(path: string): YuneWebFilesystemStat;
};

declare const globalThis: {
  onRimeNotification<T extends keyof RimeNotification>(type: T, value: RimeNotification[T]): void;
  onYunePersistenceDiagnostic?: (marker: YunePersistenceDiagnostic) => void;
  createYuneWebModule?: CreateYuneWebModule;
  createYuneTypeduckModule?: CreateYuneWebModule;
  crypto?: Crypto;
};

declare function importScripts(...urls: string[]): void;
declare const YUNE_PUBLIC_DEMO_BUILD: boolean | undefined;

// Preserve upstream notification dispatch
globalThis.onRimeNotification = (type, value) => {
  switch (type) {
    case "deploy":
      dispatch("deployStatusChanged", value as RimeDeployStatus);
      break;
    case "schema":
      dispatch("schemaChanged", ...value.split("/") as [string, string]);
      break;
    case "option": {
      const disabled = value[0] === "!";
      dispatch("optionChanged", value.slice(+disabled), !disabled);
      break;
    }
  }
};

globalThis.onYunePersistenceDiagnostic = (marker) => {
  postMessage({ type: "diagnostic", source: "yune-persistence", marker });
};
setTimeout(() => {
  postMessage({
    type: "diagnostic",
    source: "yune-persistence",
    marker: { phase: "worker:diagnostic-ready", timestamp: new Date().toISOString() },
  });
}, 0);

function dispatch<K extends keyof ListenerArgsMap>(name: K, ...args: ListenerArgsMap[K]) {
  postMessage({ type: "listener", name, args });
}

// Yune adapter Actions implementation
const actions: Actions = {
  async setOption(option, value) {
    await setOption(option, value);
  },
  async selectSchema(schemaId) {
    dispatch("deployStatusChanged", "start");
    try {
      await selectYuneSchema(schemaId);
      dispatch("deployStatusChanged", "success");
      return true;
    } catch (error) {
      dispatch("deployStatusChanged", "failure");
      throw error;
    }
  },
  async getUserdbSnapshot() {
    return activeUserdbSnapshot();
  },
  async importUserdb(rawText) {
    return importActiveUserdb(rawText);
  },
  async processKey(input) {
    const result = await processKey(input);
    // Persistence sync handled by adapter
    return withMemorySnapshot(result);
  },
  async stageAi() {
    const result = await stageAi();
    return withMemorySnapshot(result);
  },
  async selectCandidate(index) {
    const result = await selectCandidate(index);
    return withMemorySnapshot(result);
  },
  async deleteCandidate(index) {
    const result = await deleteCandidate(index);
    return withMemorySnapshot(result);
  },
  async flipPage(backward) {
    const result = await flipPage(backward);
    return withMemorySnapshot(result);
  },
  async customize(preferences) {
    const result = await customize(preferences);
    return result;
  },
  async customizeValue(configId, key, value) {
    return customizeValue(configId, key, value);
  },
  async deploy() {
    dispatch("deployStatusChanged", "start");
    try {
      const result = await deployYuneRuntime();
      if (result) {
        await selectYuneSchema(activeSchemaId, true);
      }
      dispatch("deployStatusChanged", result ? "success" : "failure");
      return result;
    } catch (error) {
      dispatch("deployStatusChanged", "failure");
      throw error;
    }
  },
  async deployCacheSnapshot() {
    return deployCacheSnapshot();
  },
  async invalidateDeployCache() {
    return invalidateDeployCache();
  },
  async injectedAssetsManifest() {
    return {
      schemaId: activeSchemaId,
      assets: loadedExtraSharedAssets.map((asset) => ({
        path: asset.path,
        bytes: typeof asset.content === "string"
          ? new TextEncoder().encode(asset.content).byteLength
          : asset.content.length,
        kind: typeof asset.content === "string" ? "text" : "binary",
      })),
    };
  },
};

function activeUserdbSnapshot(): YuneWebUserdbSnapshot {
  const module = yuneModule;
  if (module === null) {
    throw new Error("Yune module is not loaded");
  }
  const { dictionaryId, path } = activeUserdbPath();
  const base = {
    schemaId: activeSchemaId,
    dictionaryId,
    path,
  };
  if (!module.FS.analyzePath(path).exists) {
    return {
      ...base,
      exists: false,
      bytes: 0,
      updatedAt: null,
      rows: [],
      rawText: "",
      parseErrors: [],
    };
  }

  const raw = module.FS.readFile(path, { encoding: "utf8" });
  const rawText = typeof raw === "string" ? raw : new TextDecoder().decode(raw);
  const { rows, parseErrors } = parseUserdbRows(rawText);
  return {
    ...base,
    exists: true,
    bytes: new TextEncoder().encode(rawText).byteLength,
    updatedAt: userdbUpdatedAt(module.FS, path),
    rows,
    rawText,
    parseErrors,
  };
}

async function importActiveUserdb(rawText: string): Promise<YuneWebUserdbSnapshot> {
  const module = yuneModule;
  if (module === null) {
    throw new Error("Yune module is not loaded");
  }
  const { path } = activeUserdbPath();
  module.FS.writeFile(path, rawText);
  await syncAfterUserDataChange(module.FS);
  await selectYuneSchema(activeSchemaId, true);
  return activeUserdbSnapshot();
}

function activeUserdbPath() {
  const schema = PLAYGROUND_SCHEMAS[activeSchemaId];
  const dictionaryId = schema.dictionaryId;
  const path = joinYuneWebVirtualPath(RIME_USER_DIR, `${dictionaryId}.userdb`);
  return { dictionaryId, path };
}

function userdbUpdatedAt(fs: YuneWebFilesystem, path: string): string | null {
  const stat = (fs as YuneWebFilesystemWithStat).stat?.(path);
  const raw = stat?.mtime ?? stat?.mtimeMs;
  if (raw === undefined || raw === null) {
    return null;
  }
  const date = raw instanceof Date ? raw : new Date(raw);
  return Number.isNaN(date.getTime()) ? null : date.toISOString();
}

function parseUserdbRows(rawText: string): { rows: YuneWebUserdbRow[]; parseErrors: YuneWebUserdbParseError[] } {
  const rows: YuneWebUserdbRow[] = [];
  const parseErrors: YuneWebUserdbParseError[] = [];
  rawText.split(/\r?\n/).forEach((line, index) => {
    const lineNumber = index + 1;
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#") || trimmed.startsWith("/")) {
      return;
    }
    const columns = line.split("\t");
    if (columns.length < 2 || !columns[0] || !columns[1]) {
      parseErrors.push({ line: lineNumber, raw: line, reason: "expected code<TAB>text<TAB>metadata" });
      return;
    }
    const value = parseUserdbValue(columns.slice(2).join("\t"));
    rows.push({
      code: columns[0].trimEnd(),
      text: columns[1],
      commits: value.commits,
      dee: value.dee,
      tick: value.tick,
      raw: line,
    });
    if (value.error !== undefined) {
      parseErrors.push({ line: lineNumber, raw: line, reason: value.error });
    }
  });
  return { rows, parseErrors };
}

function parseUserdbValue(value: string): {
  commits: number | null;
  dee: number | null;
  tick: number | null;
  error?: string;
} {
  const trimmed = value.trim();
  if (!trimmed) {
    return { commits: null, dee: null, tick: null };
  }
  const packed = Object.fromEntries(
    trimmed
      .split(/\s+/)
      .map(field => field.split("=", 2))
      .filter((pair): pair is [string, string] => pair.length === 2 && pair[0].length > 0),
  );
  if (Object.keys(packed).length > 0) {
    return {
      commits: parseNullableNumber(packed["c"]),
      dee: parseNullableNumber(packed["d"]),
      tick: parseNullableNumber(packed["t"]),
      error: packed["c"] === undefined && packed["d"] === undefined && packed["t"] === undefined
        ? "metadata has no c/d/t fields"
        : undefined,
    };
  }
  const commits = Number(trimmed);
  if (Number.isFinite(commits)) {
    return { commits, dee: Math.abs(commits), tick: 1 };
  }
  return { commits: null, dee: null, tick: null, error: "metadata is not parseable" };
}

function parseNullableNumber(value: string | undefined): number | null {
  if (value === undefined) {
    return null;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

let loading = true;
const RIME_SHARED_DIR = "/usr/share/rime-data";
const RIME_USER_DIR = "/rime";
const DEFAULT_SCHEMA_ID: RimeSchemaId = "jyut6ping3";
const INITIAL_SCHEMA_ID: RimeSchemaId = initialSchemaFromWorkerUrl();
const YUNE_WEB_ASSET_VERSION = "yune-web-wasm-heap-v2";
const YUNE_WEB_WASM_BUILD_PROFILE = "release";
const YUNE_WEB_M27_EVIDENCE_VERSION = "m27-startup-v1";
const YUNE_WEB_M31_EVIDENCE_VERSION = "web03-three-schema-launch-v1";
const YUNE_PUBLIC_DEMO = typeof YUNE_PUBLIC_DEMO_BUILD !== "undefined" && YUNE_PUBLIC_DEMO_BUILD === true;
const LATENCY_WORKER_ACTION_MULTIPLIER = latencyWorkerActionMultiplier();
const WEB06_COLLECTION_MODE = web06CollectionMode(location.search);
let activeWeb06WorkerReceipt: Web06WorkerReceipt | null = null;
let web06ExpectedActionSequenceId = 1;
const web06SeenActionIds = new Set<string>();
const web06SeenActionIdOrder: string[] = [];

const web06AdapterObservation: Web06AdapterObservation = {
  mode: WEB06_COLLECTION_MODE,
  now: () => performance.now(),
  onSpan(span) {
    recordWeb06CollectorCallback(`span:${span.component}:${span.stage}`, () => {
      const receipt = activeWeb06WorkerReceipt;
      if (receipt === null) return;
      switch (span.component) {
        case "runtime":
          receipt.runtimeSpans.push(span);
          break;
        case "adapter":
          receipt.adapterSpans.push(span);
          break;
        case "persistence":
          receipt.persistenceSpans.push(span);
          break;
        case "collector":
          receipt.collectorSpans.push(span);
          break;
      }
    });
  },
  onEngineRawJson(copy) {
    recordWeb06CollectorCallback(`engine-raw:${copy.operation}`, () => {
      const receipt = activeWeb06WorkerReceipt;
      if (receipt === null) return;
      if (receipt.engineRawJson !== undefined) {
        receipt.observerFailures.push(`duplicate engineRaw JSON for ${copy.operation}`);
        return;
      }
      receipt.engineRawJson = copy.json;
    });
  },
  onFailure(failure) {
    try {
      const detail = stringifyWeb06Failure(failure);
      if (activeWeb06WorkerReceipt !== null) {
        activeWeb06WorkerReceipt.observerFailures.push(detail);
      }
      else {
        postMessage({ type: "diagnostic", source: "web06-observer-failure", marker: detail });
      }
    } catch {
      // Measurement failure cannot replace the runtime product result/error.
    }
  },
};

function recordWeb06CollectorCallback(operation: string, callback: () => void): void {
  const receipt = activeWeb06WorkerReceipt;
  const startedAt = performance.now();
  try {
    callback();
  } catch (error) {
    receipt?.observerFailures.push(`${operation}: ${stringifyWeb06Failure(error)}`);
  } finally {
    const finishedAt = performance.now();
    receipt?.collectorSpans.push({
      component: "collector",
      operation,
      stage: "callback",
      startedAt,
      finishedAt,
      outcome: "success",
    });
  }
}

function stringifyWeb06Failure(value: unknown): string {
  if (value instanceof Error) {
    return `${value.name}: ${value.message}`;
  }
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

function latencyWorkerActionMultiplier() {
  const raw = new URLSearchParams(location.search).get("latencyWorkerActionMultiplier");
  if (raw === null) {
    return 1;
  }
  const value = Number(raw);
  const loopback = ["127.0.0.1", "localhost", "::1", "[::1]"].includes(location.hostname);
  if (!YUNE_PUBLIC_DEMO || !loopback || value !== 4) {
    throw new Error(`Invalid public latency worker-action multiplier: ${raw}`);
  }
  return value;
}

function latencyMultiplierForAction(name: keyof Actions, args: unknown[]) {
  const input = name === "processKey" ? args[0] : undefined;
  return typeof input === "string" && /^\{[A-Za-z]\}$/.test(input)
    ? LATENCY_WORKER_ACTION_MULTIPLIER
    : 1;
}

function amplifyWorkerAction(workerStartedAt: number, multiplier: number) {
  const baseFinishedAt = nowMs();
  const workerBaseElapsedMs = Math.max(0, baseFinishedAt - workerStartedAt);
  const targetAmplificationMs = workerBaseElapsedMs * (multiplier - 1);
  const amplificationStartedAt = performance.now();
  let value = 0x811c9dc5;
  while (performance.now() - amplificationStartedAt < targetAmplificationMs) {
    for (let index = 0; index < 1_000; index += 1) {
      value = (Math.imul(value ^ index, 16777619) + 1013904223) | 0;
    }
  }
  if (!Number.isFinite(value)) {
    throw new Error("Latency worker-action amplification failed");
  }
  return {
    workerBaseElapsedMs,
    workerAmplificationMs: performance.now() - amplificationStartedAt,
    workerActionMultiplier: multiplier,
  };
}
// Cloudflare Pages caps a single deployed asset at 25 MiB, so the public build
// (public-demo/build.mjs) splits oversized schema payloads into ordered
// `<path>.partN` chunks. These bounds must match that build script so the byte
// ranges the worker reassembles line up exactly with what was written.
const PAGES_MAX_ASSET_BYTES = 25 * 1024 * 1024;
const SPLIT_CHUNK_BYTES = 20 * 1024 * 1024;
type WasmAttributionAssetFamily =
  | "luna-core"
  | "jyutping-core"
  | "jyutping-scolar"
  | "reverse-lookup"
  | "opencc"
  | "extras"
  | "full-jyutping";
const YUNE_WEB_WASM_ATTRIBUTION_FAMILY = wasmAttributionAssetFamilyFromWorkerUrl();
const YUNE_WEB_COMMON_SHARED_ASSETS = [
  "default.custom.yaml",
  "common.yaml",
  "common.custom.yaml",
  "include.yaml",
  "template.yaml",
] as const;
const YUNE_WEB_OPENCC_SHARED_ASSETS = [
  "opencc/t2hkf.json",
  "opencc/HKVariantsFull.txt",
  "opencc/t2s.json",
  "opencc/t2tw.json",
  "opencc/hk2s.json",
  "opencc/HKVariantsRev.ocd2",
  "opencc/HKVariantsRevPhrases.ocd2",
  "opencc/TSCharacters.ocd2",
  "opencc/TSPhrases.ocd2",
] as const;
const YUNE_WEB_LUNA_OPENCC_SHARED_ASSETS = [
  "opencc/t2hk.json",
  "opencc/t2s.json",
  "opencc/t2tw.json",
  "opencc/HKVariants.ocd2",
  "opencc/TSCharacters.ocd2",
  "opencc/TSPhrases.ocd2",
  "opencc/TWVariants.ocd2",
] as const;
const YUNE_WEB_LUNA_SHARED_ASSETS = [
  "default.custom.yaml",
  "pinyin.yaml",
  "key_bindings.yaml",
  "punctuation.yaml",
  "symbols.yaml",
  "essay.txt",
  "luna_pinyin.schema.yaml",
  "luna_pinyin.dict.yaml",
  "luna_pinyin.table.bin",
  "luna_pinyin.reverse.bin",
  "luna_pinyin.prism.bin",
  "stroke.schema.yaml",
  "stroke.dict.yaml",
  "stroke.table.bin",
  "stroke.reverse.bin",
  "stroke.prism.bin",
  ...YUNE_WEB_LUNA_OPENCC_SHARED_ASSETS,
] as const;
const YUNE_WEB_CANGJIE_SHARED_ASSETS = [
  ...YUNE_WEB_COMMON_SHARED_ASSETS,
  "cangjie5.schema.yaml",
  "cangjie5.dict.yaml",
  "cangjie5.table.bin",
  "cangjie5.reverse.bin",
  "cangjie5.prism.bin",
  "jyut6ping3.dict.yaml",
  "jyut6ping3.table.bin",
  "jyut6ping3.reverse.bin",
  "jyut6ping3_scolar.schema.yaml",
  "jyut6ping3_scolar.dict.yaml",
  "jyut6ping3_scolar.table.bin",
  "jyut6ping3_scolar.reverse.bin",
  "jyut6ping3_scolar.prism.bin",
  ...YUNE_WEB_OPENCC_SHARED_ASSETS,
] as const;
const YUNE_WEB_JYUTPING_SHARED_ASSETS = [
  ...YUNE_WEB_COMMON_SHARED_ASSETS,
  "jyut6ping3.schema.yaml",
  "jyut6ping3_mobile.schema.yaml",
  "jyut6ping3_scolar.schema.yaml",
  "jyut6ping3_scolar.dict.yaml",
  "loengfan.schema.yaml",
  "loengfan.dict.yaml",
  "cangjie3.schema.yaml",
  "cangjie3.dict.yaml",
  "cangjie5.schema.yaml",
  "cangjie5.dict.yaml",
  "luna_pinyin.dict.yaml",
  "luna_pinyin_yune_reverse.dict.yaml",
  "luna_pinyin_yune_reverse.table.bin",
  "luna_pinyin_yune_reverse.reverse.bin",
  "luna_pinyin_yune_reverse.prism.bin",
  ...YUNE_WEB_OPENCC_SHARED_ASSETS,
  "jyut6ping3.table.bin",
  "jyut6ping3.reverse.bin",
  "jyut6ping3_mobile.prism.bin",
  "jyut6ping3_scolar.table.bin",
  "jyut6ping3_scolar.reverse.bin",
  "jyut6ping3_scolar.prism.bin",
] as const;
const YUNE_WEB_JYUTPING_CORE_SHARED_ASSETS = [
  ...YUNE_WEB_COMMON_SHARED_ASSETS,
  "jyut6ping3.schema.yaml",
  "jyut6ping3_mobile.schema.yaml",
  "jyut6ping3.table.bin",
  "jyut6ping3.reverse.bin",
  "jyut6ping3_mobile.prism.bin",
] as const;
const YUNE_WEB_JYUTPING_SCOLAR_SHARED_ASSETS = [
  ...YUNE_WEB_JYUTPING_CORE_SHARED_ASSETS,
  "jyut6ping3_scolar.schema.yaml",
  "jyut6ping3_scolar.dict.yaml",
  "jyut6ping3_scolar.table.bin",
  "jyut6ping3_scolar.reverse.bin",
  "jyut6ping3_scolar.prism.bin",
] as const;
const YUNE_WEB_REVERSE_LOOKUP_SHARED_ASSETS = [
  ...YUNE_WEB_JYUTPING_CORE_SHARED_ASSETS,
  "loengfan.schema.yaml",
  "loengfan.dict.yaml",
  "cangjie3.schema.yaml",
  "cangjie3.dict.yaml",
  "cangjie5.schema.yaml",
  "cangjie5.dict.yaml",
  "luna_pinyin.dict.yaml",
  "luna_pinyin_yune_reverse.dict.yaml",
] as const;
const YUNE_WEB_OPENCC_ATTRIBUTION_SHARED_ASSETS = [
  ...YUNE_WEB_JYUTPING_CORE_SHARED_ASSETS,
  ...YUNE_WEB_OPENCC_SHARED_ASSETS,
] as const;
const PLAYGROUND_SCHEMAS: Record<RimeSchemaId, PlaygroundSchema> = {
  jyut6ping3: {
    runtimeId: "jyut6ping3_mobile",
    name: "Jyutping",
    dictionaryId: "jyut6ping3",
    deployedDefaultPath: "build/default.yaml",
    deployedSchemaPath: "build/jyut6ping3_mobile.schema.yaml",
  },
  cangjie5: {
    runtimeId: "cangjie5",
    name: "Cangjie 5",
    dictionaryId: "cangjie5",
  },
  luna_pinyin: {
    runtimeId: "luna_pinyin",
    name: "Luna Pinyin",
    dictionaryId: "luna_pinyin",
  },
  luna_pinyin_octagram: {
    runtimeId: "luna_pinyin_octagram",
    name: "Luna Pinyin + Octagram",
    dictionaryId: "luna_pinyin",
  },
};
let yuneModule: YuneWebBrowserModule | null = null;
let loadedExtraSharedAssets: { path: string; content: string | Uint8Array }[] = [];
let activeSchemaId: RimeSchemaId = INITIAL_SCHEMA_ID;
let publicAssetManifest: PublicAssetManifest | null = null;
let publicAssetCacheStats: PublicAssetCacheStats = { hits: 0, misses: 0, unavailable: false };
let peakWasmHeapBytes = 0;
const verifiedOctagramModelCache = new Map<string, Uint8Array>();

function withMemorySnapshot(result: RimeResult): RimeResult {
  const memory = activeWasmMemorySnapshot();
  return memory === undefined ? result : { ...result, memory };
}

function activeWasmMemorySnapshot(): YuneWebMemorySnapshot | undefined {
  const module = yuneModule;
  if (module === null) {
    return undefined;
  }
  return wasmMemorySnapshot(module);
}

function wasmMemorySnapshot(module: YuneWebBrowserModule): YuneWebMemorySnapshot | undefined {
  const wasmHeapBytes = wasmHeapByteLength(module);
  if (wasmHeapBytes === undefined) {
    return undefined;
  }
  peakWasmHeapBytes = Math.max(peakWasmHeapBytes, wasmHeapBytes);
  return { wasmHeapBytes, peakWasmHeapBytes };
}

function startupWasmMemorySnapshot(memory: YuneWebMemorySnapshot | undefined): StartupWasmMemorySnapshot | undefined {
  return memory === undefined
    ? undefined
    : { currentBytes: memory.wasmHeapBytes, peakBytes: memory.peakWasmHeapBytes };
}

function wasmHeapByteLength(module: YuneWebBrowserModule): number | undefined {
  const buffer =
    module.HEAPU8?.buffer ?? module.HEAP8?.buffer ?? module.wasmMemory?.buffer;
  return buffer instanceof ArrayBuffer ? buffer.byteLength : undefined;
}

function nowMs(): number {
  return performance.timeOrigin + performance.now();
}

function resolveYuneWebModuleFactory(): CreateYuneWebModule {
  if (typeof globalThis.createYuneWebModule === "function") {
    return globalThis.createYuneWebModule;
  }
  if (typeof globalThis.createYuneTypeduckModule === "function") {
    return globalThis.createYuneTypeduckModule;
  }
  throw new Error("Yune Emscripten module factory is unavailable");
}

// Yune runtime initialization
const loadRime = (async () => {
  const startupStartedAt = performance.now();
  const startupMarkers: StartupMarker[] = [];
  const markStartup = (phase: string, moduleForMemory: YuneWebBrowserModule | null = yuneModule) => {
    const marker: StartupMarker = { phase, ms: Math.round(performance.now() - startupStartedAt) };
    const memory = moduleForMemory === null ? undefined : startupWasmMemorySnapshot(wasmMemorySnapshot(moduleForMemory));
    if (memory !== undefined) {
      marker.wasmMemory = memory;
    }
    startupMarkers.push(marker);
  };
  try {
    markStartup("runtime:init:start");
    markStartup("worker:start");
    importScripts(`yune-web.js?v=${YUNE_WEB_ASSET_VERSION}`);
    markStartup("wasm-glue:loaded");
    markStartup("wasm:module:create:start");
    const module = await resolveYuneWebModuleFactory()({
      printErr,
      noInitialRun: true,
      locateFile(path) {
        if (path.endsWith(".wasm")) {
          return `yune-web.wasm?v=${YUNE_WEB_ASSET_VERSION}`;
        }
        return path;
      },
    });
    markStartup("wasm:module:create:finish");
    markStartup("module:created", module);

    if (module.IDBFS === undefined || module.IDBFS === null) {
      throw new Error("Yune Emscripten module missing IDBFS runtime method");
    }

    markStartup("filesystem:mount:start");
    mountYuneWebPersistence(module.FS, module.IDBFS, {}, RIME_USER_DIR);
    yuneModule = module;
    markStartup("filesystem:mount:finish", module);
    markStartup("persistence:mounted", module);

    markStartup("assets:load:start");
    publicAssetCacheStats = { hits: 0, misses: 0, unavailable: false };
    loadedExtraSharedAssets = await loadSharedAssetsForSchema(INITIAL_SCHEMA_ID);
    markStartup("assets:load:finish", module);
    markStartup("assets:loaded", module);

    markStartup("schema:select:start");
    await selectYuneSchema(INITIAL_SCHEMA_ID);
    markStartup("schema:select:finish", module);
    markStartup("startup-defaults:customize:start");
    await customize(defaultStartupDeployPreferences());
    markStartup("startup-defaults:customize:finish", module);
    markStartup("runtime:init:finish", module);
    markStartup("runtime:initialized", module);
    const startupMemory = activeWasmMemorySnapshot();

    loading = false;
    dispatch("initialized", true, startupMemory);
    postMessage({
      type: "diagnostic",
      source: "yune-startup",
      marker: {
        phase: "startup:complete",
        totalMs: Math.round(performance.now() - startupStartedAt),
        markers: startupMarkers,
        m27EvidenceVersion: YUNE_WEB_M27_EVIDENCE_VERSION,
        m31EvidenceVersion: YUNE_PUBLIC_DEMO ? YUNE_WEB_M31_EVIDENCE_VERSION : undefined,
        publicDemo: YUNE_PUBLIC_DEMO,
        assetVersion: YUNE_WEB_ASSET_VERSION,
        schema: PLAYGROUND_SCHEMAS[INITIAL_SCHEMA_ID].runtimeId,
        wasmMemory: startupWasmMemorySnapshot(startupMemory),
        wasmBuildProfile: YUNE_WEB_WASM_BUILD_PROFILE,
        wasmAttributionAssetFamily: YUNE_WEB_WASM_ATTRIBUTION_FAMILY ?? undefined,
        wasmGlue: "yune-web.js",
        wasmBinary: "yune-web.wasm",
        assetCache: YUNE_PUBLIC_DEMO ? publicAssetCacheStats : undefined,
        loadedExplicitAssets: [
          "default.yaml",
          `${PLAYGROUND_SCHEMAS[INITIAL_SCHEMA_ID].runtimeId}.schema.yaml`,
          `${PLAYGROUND_SCHEMAS[INITIAL_SCHEMA_ID].dictionaryId}.dict.yaml`,
        ],
        loadedSharedAssets: loadedExtraSharedAssets.map((asset) => asset.path),
      },
    });
  } catch (error) {
    console.error("Yune runtime initialization failed", error);
    loading = false;
    dispatch("initialized", false);
    postMessage({
      type: "error",
      error: error instanceof Error ? `${error.name}: ${error.message}` : String(error),
    });
    throw error;
  }
})();

function printErr(message: string): void {
  if (!YUNE_PUBLIC_DEMO || location.search === "?debug") {
    const match = /^([IWEF])\S+ \S+ \S+ (.*)$/.exec(message);
    if (match) {
      console[({ I: "info", W: "warn", E: "error", F: "error" } as const)[match[1] as "I" | "W" | "E" | "F"]](`[${match[2]}`);
    }
    else {
      console.error(message);
    }
  }
}

function initialSchemaFromWorkerUrl(): RimeSchemaId {
  try {
    const raw = new URL(location.href).searchParams.get("schema");
    if (raw === "jyut6ping3_mobile") {
      return "jyut6ping3";
    }
    if (isRimeSchemaId(raw)) {
      return raw;
    }
  } catch {
    // Fall through to the app default below.
  }
  return DEFAULT_SCHEMA_ID;
}

function wasmAttributionAssetFamilyFromWorkerUrl(): WasmAttributionAssetFamily | null {
  try {
    const raw = new URL(location.href).searchParams.get("assetFamily");
    return isWasmAttributionAssetFamily(raw) ? raw : null;
  } catch {
    return null;
  }
}

function isWasmAttributionAssetFamily(value: string | null): value is WasmAttributionAssetFamily {
  return value === "luna-core"
    || value === "jyutping-core"
    || value === "jyutping-scolar"
    || value === "reverse-lookup"
    || value === "opencc"
    || value === "extras"
    || value === "full-jyutping";
}

function defaultStartupDeployPreferences(): Partial<RimePreferences> {
  return {
    pageSize: 6,
    enableCompletion: true,
    enableCorrection: false,
    enableSentence: true,
    enableLearning: true,
    combineCandidates: true,
    predictionNeverFirst: true,
    predictionThreshold: 0,
    dictionaryExclude: [],
    isCangjie5: true,
  };
}

async function loadPublicAssetManifest(): Promise<PublicAssetManifest> {
  if (publicAssetManifest !== null) {
    return publicAssetManifest;
  }
  const response = await fetch(`schema-asset-manifest.json?v=${YUNE_WEB_M31_EVIDENCE_VERSION}`, {
    cache: "no-cache",
  });
  if (!response.ok) {
    throw new Error(`yune-web public asset manifest failed to load (${response.status})`);
  }
  const manifest = await response.json() as PublicAssetManifest;
  if (manifest.generatedFor !== "yune-web" || manifest.version !== YUNE_WEB_M31_EVIDENCE_VERSION) {
    throw new Error(`Unexpected yune-web asset manifest ${manifest.generatedFor}/${manifest.version}`);
  }
  publicAssetManifest = manifest;
  return manifest;
}

async function publicAssetManifestEntry(path: string): Promise<PublicAssetManifestEntry> {
  const manifest = await loadPublicAssetManifest();
  const entry = manifest.assets.find((asset) => asset.path === path);
  if (entry === undefined) {
    throw new Error(`yune-web public asset ${path} is missing from schema-asset-manifest.json`);
  }
  return entry;
}

async function loadPublicSharedAssets() {
  return loadPublicSharedAssetPaths(sharedAssetPathsForSchema(INITIAL_SCHEMA_ID));
}

async function loadPublicSharedAssetPaths(paths: readonly string[]) {
  const assets = await Promise.all(
    paths.map(async (path) => ({
      path,
      content: await loadPublicSchemaAsset(path),
    })),
  );
  return assets;
}

async function loadPublicSchemaAsset(path: string): Promise<string | Uint8Array> {
  const entry = await publicAssetManifestEntry(path);
  if (entry.bytes > PAGES_MAX_ASSET_BYTES) {
    return loadSplitPublicSchemaAsset(path, entry);
  }
  const response = await fetchPublicAsset(`schema/${path}`, entry.sha256);
  return responseAssetContent(response, path);
}

async function fetchPublicAsset(sourceUrl: string, sha256: string): Promise<Response> {
  if (typeof caches === "undefined") {
    publicAssetCacheStats.unavailable = true;
    const uncachedUrl = `${sourceUrl}?sha256=${sha256}`;
    const response = await fetch(uncachedUrl, { cache: "force-cache" });
    if (!response.ok) {
      throw new Error(`Asset URL loading failed: ${uncachedUrl} (${response.status})`);
    }
    return response;
  }

  const cache = await caches.open(`yune-web-assets-${YUNE_WEB_M31_EVIDENCE_VERSION}`);
  const cacheUrl = new URL(sourceUrl, location.href);
  cacheUrl.searchParams.set("sha256", sha256);
  const cacheRequest = new Request(cacheUrl.toString());
  const cached = await cache.match(cacheRequest);
  if (cached !== undefined) {
    publicAssetCacheStats.hits += 1;
    return cached;
  }

  const versionedSourceUrl = cacheUrl.toString();
  const response = await fetch(versionedSourceUrl, { cache: "force-cache" });
  if (!response.ok) {
    throw new Error(`Asset URL loading failed: ${versionedSourceUrl} (${response.status})`);
  }
  await cache.put(cacheRequest, response.clone());
  publicAssetCacheStats.misses += 1;
  return response;
}

// Oversized schema payloads are split into `<path>.partN` chunks by the public
// build to stay under Cloudflare Pages' 25 MiB per-asset cap. Fetch every part,
// concatenate them in order, and verify the reconstructed bytes against the
// manifest before handing them to the WASM filesystem.
async function loadSplitPublicSchemaAsset(
  path: string,
  entry: PublicAssetManifestEntry,
): Promise<Uint8Array> {
  const partCount = Math.ceil(entry.bytes / SPLIT_CHUNK_BYTES);
  const partPromises: Promise<Uint8Array>[] = [];
  for (let index = 0; index < partCount; index += 1) {
    partPromises.push(
      fetchPublicAsset(`schema/${path}.part${index}`, entry.sha256).then(
        async (response) => new Uint8Array(await response.arrayBuffer()),
      ),
    );
  }
  const parts = await Promise.all(partPromises);

  const combined = new Uint8Array(entry.bytes);
  let offset = 0;
  for (const part of parts) {
    if (offset + part.byteLength > entry.bytes) {
      throw new Error(`Split asset ${path} overflowed ${entry.bytes} expected bytes while reassembling`);
    }
    combined.set(part, offset);
    offset += part.byteLength;
  }
  if (offset !== entry.bytes) {
    throw new Error(`Split asset ${path} reassembled ${offset} bytes, expected ${entry.bytes}`);
  }

  if (globalThis.crypto?.subtle) {
    const actualSha256 = await sha256Hex(combined);
    if (actualSha256 !== entry.sha256) {
      throw new Error(`Split asset ${path} sha256 mismatch: expected ${entry.sha256}, got ${actualSha256}`);
    }
  }
  return combined;
}

async function responseAssetContent(response: Response, path: string): Promise<string | Uint8Array> {
  if (isBinarySchemaAsset(path)) {
    return new Uint8Array(await response.arrayBuffer());
  }
  return response.text();
}

function isBinarySchemaAsset(path: string): boolean {
  return /\.(?:bin|ocd2)$/i.test(path);
}

async function loadExtraSharedAssets(paths: string[], optional = false) {
  const assets = await Promise.all(
    paths.map(async (path) => {
      const source: AssetSource = { type: "url", url: `schema/${path}` };
      try {
        return {
          path,
          content: await loadAssetContent(source),
        };
      } catch (error) {
        if (optional) {
          return null;
        }
        throw error;
      }
    }),
  );
  return assets.filter((asset): asset is { path: string; content: string | Uint8Array } => asset !== null);
}

async function loadSharedAssetsForSchema(schemaId: RimeSchemaId) {
  const paths = sharedAssetPathsForSchema(schemaId);
  if (YUNE_PUBLIC_DEMO) {
    return loadPublicSharedAssetPaths(paths);
  }
  return loadExtraSharedAssets([...paths], true);
}

async function ensureSharedAssetsForSchema(schemaId: RimeSchemaId): Promise<void> {
  const loadedPaths = new Set(loadedExtraSharedAssets.map((asset) => asset.path));
  const missing = sharedAssetPathsForSchema(schemaId).filter((path) => !loadedPaths.has(path));
  if (missing.length === 0) {
    return;
  }
  loadedExtraSharedAssets.push(...await (YUNE_PUBLIC_DEMO
    ? loadPublicSharedAssetPaths(missing)
    : loadExtraSharedAssets(missing, true)));
}

function sharedAssetPathsForSchema(schemaId: RimeSchemaId): readonly string[] {
  if (YUNE_WEB_WASM_ATTRIBUTION_FAMILY !== null) {
    return uniqueSharedAssetPaths(sharedAssetPathsForAttributionFamily(YUNE_WEB_WASM_ATTRIBUTION_FAMILY));
  }
  switch (schemaId) {
    case "luna_pinyin":
    case "luna_pinyin_octagram":
      return uniqueSharedAssetPaths(YUNE_WEB_LUNA_SHARED_ASSETS);
    case "cangjie5":
      return uniqueSharedAssetPaths(YUNE_WEB_CANGJIE_SHARED_ASSETS);
    case "jyut6ping3":
    default:
      return uniqueSharedAssetPaths(YUNE_WEB_JYUTPING_SHARED_ASSETS);
  }
}

function sharedAssetPathsForAttributionFamily(family: WasmAttributionAssetFamily): readonly string[] {
  switch (family) {
    case "luna-core":
      return YUNE_WEB_LUNA_SHARED_ASSETS;
    case "jyutping-core":
      return YUNE_WEB_JYUTPING_CORE_SHARED_ASSETS;
    case "jyutping-scolar":
      return YUNE_WEB_JYUTPING_SCOLAR_SHARED_ASSETS;
    case "reverse-lookup":
      return YUNE_WEB_REVERSE_LOOKUP_SHARED_ASSETS;
    case "opencc":
      return YUNE_WEB_OPENCC_ATTRIBUTION_SHARED_ASSETS;
    case "extras":
      return [];
    case "full-jyutping":
      return YUNE_WEB_JYUTPING_SHARED_ASSETS;
  }
}

function uniqueSharedAssetPaths(paths: readonly string[]): string[] {
  return [...new Set(paths)];
}

async function loadOctagramAssetForSchema(schemaId: RimeSchemaId): Promise<{
  effectiveSchemaId: RimeSchemaId;
  asset: { path: string; content: Uint8Array } | null;
  diagnostic: GrammarModelDiagnostic;
}> {
  const request = grammarModelRequestForSchema(schemaId);
  if (request === null) {
    return {
      effectiveSchemaId: schemaId,
      asset: null,
      diagnostic: grammarDiagnosticForSchema(schemaId, {
        reason: "schema has no grammar/language",
      }),
    };
  }

  try {
    const cached = verifiedOctagramModelCache.get(request.expectedSha256);
    if (cached !== undefined) {
      return {
        effectiveSchemaId: schemaId,
        asset: { path: request.sharedDataPath, content: cached },
        diagnostic: grammarDiagnosticForSchema(schemaId, {
          delivered: true,
          modelId: request.modelId,
          expectedSha256: request.expectedSha256,
          actualSha256: request.expectedSha256,
          bytes: cached.byteLength,
          sourcePath: request.assetPath,
          fallback: false,
          reason: "verified browser delivery cache hit; ranking rows prove engine grammar use",
        }),
      };
    }
    const content = await loadAssetContent({
      type: "url",
      url: `schema/${request.assetPath}`,
    });
    if (!(content instanceof Uint8Array)) {
      throw new Error(`octagram model ${request.assetPath} did not load as bytes`);
    }
    if (content.byteLength !== OCTAGRAM_MODEL_BYTES) {
      throw new Error(`octagram model size mismatch: expected ${OCTAGRAM_MODEL_BYTES}, got ${content.byteLength}`);
    }
    const actualSha256 = await sha256Hex(content);
    if (actualSha256 !== request.expectedSha256) {
      throw new Error(`octagram model checksum mismatch: expected ${request.expectedSha256}, got ${actualSha256}`);
    }
    verifiedOctagramModelCache.set(request.expectedSha256, content);
    return {
      effectiveSchemaId: schemaId,
      asset: { path: request.sharedDataPath, content },
      diagnostic: grammarDiagnosticForSchema(schemaId, {
        delivered: true,
        modelId: request.modelId,
        expectedSha256: request.expectedSha256,
        actualSha256,
        bytes: content.byteLength,
        sourcePath: request.assetPath,
        fallback: false,
      }),
    };
  } catch (error) {
    return {
      effectiveSchemaId: "luna_pinyin",
      asset: null,
      diagnostic: grammarDiagnosticForSchema(schemaId, {
        effectiveSchemaId: "luna_pinyin",
        delivered: false,
        modelId: request.modelId,
        expectedSha256: request.expectedSha256,
        sourcePath: request.assetPath,
        fallback: true,
        reason: error instanceof Error ? `${error.name}: ${error.message}` : String(error),
      }),
    };
  }
}

async function sha256Hex(bytes: Uint8Array): Promise<string> {
  if (!globalThis.crypto?.subtle) {
    throw new Error("octagram checksum verification requires a secure context (localhost or HTTPS)");
  }
  const source =
    bytes.buffer instanceof ArrayBuffer
      ? new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength)
      : Uint8Array.from(bytes);
  const digest = await globalThis.crypto.subtle.digest("SHA-256", source);
  return [...new Uint8Array(digest)]
    .map(byte => byte.toString(16).padStart(2, "0"))
    .join("");
}

async function selectYuneSchema(schemaId: RimeSchemaId, preserveDeployedAssets = false): Promise<void> {
  const module = yuneModule;
  if (module === null) {
    throw new Error("Yune module is not loaded");
  }
  if (PLAYGROUND_SCHEMAS[schemaId] === undefined) {
    throw new Error(`Unknown Yune schema: ${schemaId}`);
  }
  const memoryBefore = activeWasmMemorySnapshot();
  const [octagram] = await Promise.all([
    loadOctagramAssetForSchema(schemaId),
    ensureSharedAssetsForSchema(schemaId),
  ]);
  const effectiveSchemaId = octagram.effectiveSchemaId;
  const schema = PLAYGROUND_SCHEMAS[effectiveSchemaId];
  const perSchemaExtraSharedAssets = octagram.asset === null
    ? loadedExtraSharedAssets
    : [...loadedExtraSharedAssets, octagram.asset];
  const assetsConfig: ExplicitYuneWebAssets = {
    defaultYaml: await schemaAssetSource("default.yaml"),
    schemaYaml: await schemaAssetSource(`${schema.runtimeId}.schema.yaml`),
    dictionaryYaml: await schemaAssetSource(`${schema.dictionaryId}.dict.yaml`),
    ...(schema.deployedDefaultPath === undefined ? {} : {
      deployedDefaultYaml: await schemaAssetSource(schema.deployedDefaultPath),
    }),
    ...(schema.deployedSchemaPath === undefined ? {} : {
      deployedSchemaYaml: await schemaAssetSource(schema.deployedSchemaPath),
    }),
  };
  const assets = await loadExplicitAssets(assetsConfig);
  validateExplicitAssets(assets);
  await initYuneRuntime(
    module,
    module.FS,
    {
      sharedDataDir: RIME_SHARED_DIR,
      userDataDir: RIME_USER_DIR,
      schemaId: schema.runtimeId,
    },
    assets,
    schema.dictionaryId,
      perSchemaExtraSharedAssets,
      preserveDeployedAssets,
      YUNE_WEB_ASSET_VERSION,
      web06AdapterObservation,
  );
  activeSchemaId = schemaId;
  const activeGrammarDiagnostic = {
    ...octagram.diagnostic,
    ...grammarMemoryDelta(memoryBefore, activeWasmMemorySnapshot()),
  };
  dispatch("schemaChanged", effectiveSchemaId, schema.name);
  dispatch("grammarDiagnosticChanged", activeGrammarDiagnostic);
}

async function schemaAssetSource(path: string): Promise<AssetSource> {
  if (!YUNE_PUBLIC_DEMO) {
    return { type: "url", url: `schema/${path}` };
  }
  return { type: "content", content: await loadPublicSchemaAsset(path) };
}

addEventListener("message", async (event: MessageEvent<Web06WorkerRequestEnvelope>) => {
  const workerMessageReceivedAt = performance.now();
  const { data } = event;
  if (data.kind === "clock-ping") {
    const workerSentAt = performance.now();
    postMessage({
      protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
      kind: "clock-echo",
      pingId: data.pingId,
      mainSentAt: data.mainSentAt,
      workerReceivedAt: workerMessageReceivedAt,
      workerSentAt,
    });
    return;
  }
  await handleWeb06WorkerAction(data, workerMessageReceivedAt);
});

async function handleWeb06WorkerAction(
  envelope: Web06WorkerActionEnvelope,
  workerMessageReceivedAt: number,
): Promise<void> {
  const protocolFailures: string[] = [];
  if (envelope.protocolVersion !== WEB06_PRIVATE_PROTOCOL_VERSION) {
    protocolFailures.push(`protocol ${envelope.protocolVersion}`);
  }
  if (envelope.mode !== WEB06_COLLECTION_MODE) {
    protocolFailures.push(`mode ${envelope.mode}, expected ${WEB06_COLLECTION_MODE}`);
  }
  if (web06SeenActionIds.has(envelope.identity.actionId)) {
    protocolFailures.push(`duplicate actionId ${envelope.identity.actionId}`);
  }
  if (envelope.identity.sequenceId !== web06ExpectedActionSequenceId) {
    protocolFailures.push(
      `sequence ${envelope.identity.sequenceId}, expected ${web06ExpectedActionSequenceId}`,
    );
  }
  web06SeenActionIds.add(envelope.identity.actionId);
  web06SeenActionIdOrder.push(envelope.identity.actionId);
  if (web06SeenActionIdOrder.length > 8_192) {
    protocolFailures.push("worker action-ID ring exceeded 8192 records");
    const evicted = web06SeenActionIdOrder.shift();
    if (evicted !== undefined) web06SeenActionIds.delete(evicted);
  }
  web06ExpectedActionSequenceId = Math.max(
    web06ExpectedActionSequenceId,
    envelope.identity.sequenceId + 1,
  );

  if (loading) await loadRime;
  const workerActionStartedAt = performance.now();
  const workerStartedAt = nowMs();
  const receipt: Web06WorkerReceipt = {
    workerMessageReceivedAt,
    workerActionStartedAt,
    workerFinishedAt: workerActionStartedAt,
    runtimeSpans: [],
    adapterSpans: [],
    persistenceSpans: [],
    collectorSpans: [],
    observerFailures: protocolFailures,
  };
  if (activeWeb06WorkerReceipt !== null) {
    receipt.observerFailures.push("worker received an action while another receipt was active");
  }
  activeWeb06WorkerReceipt = receipt;
  try {
    // The private envelope is stripped here. The public action receives only
    // its original arguments and returns its original result shape.
    // @ts-expect-error The discriminated action tuple is validated by the main-thread registry.
    const result = await actions[envelope.name](...envelope.args);
    const amplification = amplifyWorkerAction(
      workerStartedAt,
      latencyMultiplierForAction(envelope.name, envelope.args),
    );
    const workerFinishedAt = nowMs();
    receipt.workerFinishedAt = performance.now();
    postMessage({
      protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
      kind: "action-result",
      mode: WEB06_COLLECTION_MODE,
      identity: envelope.identity,
      resultType: "success",
      result,
      receipt,
      elapsedMs: Math.round(workerFinishedAt - workerStartedAt),
      workerStartedAt,
      workerFinishedAt,
      ...amplification,
    });
  }
  catch (error) {
    const amplification = amplifyWorkerAction(
      workerStartedAt,
      latencyMultiplierForAction(envelope.name, envelope.args),
    );
    const workerFinishedAt = nowMs();
    receipt.workerFinishedAt = performance.now();
    postMessage({
      protocolVersion: WEB06_PRIVATE_PROTOCOL_VERSION,
      kind: "action-result",
      mode: WEB06_COLLECTION_MODE,
      identity: envelope.identity,
      resultType: "error",
      error,
      receipt,
      elapsedMs: Math.round(workerFinishedAt - workerStartedAt),
      workerStartedAt,
      workerFinishedAt,
      ...amplification,
    });
  }
  finally {
    activeWeb06WorkerReceipt = null;
  }
}

// Cleanup on worker termination
addEventListener("unload", () => {
  cleanupYuneRuntime();
});
