/**
 * yune-web worker with Yune runtime integration
 *
 * This patch replaces librime/WASM binding with Yune runtime adapter
 * from @yune-ime/yune-web-runtime while preserving Actions interface
 * and message handling logic.
 */

import type { Actions, ListenerArgsMap, Message, RimeResult, RimePreferences, RimeNotification, RimeDeployStatus, RimeSchemaId, YuneWebUserdbParseError, YuneWebUserdbRow, YuneWebUserdbSnapshot } from "./types";

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
  setOption,
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
  type EmscriptenYuneWebModule,
  type YuneWebFilesystem,
} from "@yune-ime/yune-web-runtime";

interface YuneWebBrowserModule extends EmscriptenYuneWebModule {
  FS: YuneWebFilesystem;
  IDBFS: unknown;
}

interface CreateYuneWebModuleOptions {
  printErr: (message: string) => void;
  locateFile: (path: string, prefix: string) => string;
  noInitialRun?: boolean;
}

type CreateYuneWebModule = (options: CreateYuneWebModuleOptions) => Promise<YuneWebBrowserModule>;

interface PlaygroundSchema {
  id: RimeSchemaId;
  name: string;
  dictionaryId: string;
}

interface StartupMarker {
  phase: string;
  ms: number;
}

interface PublicAssetManifestEntry {
  path: string;
  sha256: string;
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
  async processKey(input) {
    const result = await processKey(input);
    // Persistence sync handled by adapter
    return result;
  },
  async stageAi() {
    const result = await stageAi();
    return result;
  },
  async selectCandidate(index) {
    const result = await selectCandidate(index);
    return result;
  },
  async deleteCandidate(index) {
    const result = await deleteCandidate(index);
    return result;
  },
  async flipPage(backward) {
    const result = await flipPage(backward);
    return result;
  },
  async customize(preferences) {
    const result = await customize(preferences);
    return result;
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
};

function activeUserdbSnapshot(): YuneWebUserdbSnapshot {
  const module = yuneModule;
  if (module === null) {
    throw new Error("Yune module is not loaded");
  }
  const schema = PLAYGROUND_SCHEMAS[activeSchemaId];
  const dictionaryId = schema.dictionaryId;
  const path = joinYuneWebVirtualPath(RIME_USER_DIR, `${dictionaryId}.userdb`);
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
const DEFAULT_SCHEMA_ID: RimeSchemaId = "jyut6ping3_mobile";
const YUNE_WEB_ASSET_VERSION = "m27-startup-v2";
const YUNE_WEB_WASM_BUILD_PROFILE = "release";
const YUNE_WEB_M27_EVIDENCE_VERSION = "m27-startup-v1";
const YUNE_WEB_M31_EVIDENCE_VERSION = "m31-yune-web-public-demo-v1";
const YUNE_PUBLIC_DEMO = typeof YUNE_PUBLIC_DEMO_BUILD !== "undefined" && YUNE_PUBLIC_DEMO_BUILD === true;
const YUNE_WEB_PUBLIC_SHARED_ASSETS = [
  "default.custom.yaml",
  "common.yaml",
  "common.custom.yaml",
  "include.yaml",
  "template.yaml",
  "jyut6ping3.schema.yaml",
  "jyut6ping3_mobile.schema.yaml",
  "jyut6ping3_scolar.schema.yaml",
  "jyut6ping3_scolar.dict.yaml",
  "luna_pinyin.schema.yaml",
  "luna_pinyin.dict.yaml",
  "luna_pinyin_yune_reverse.dict.yaml",
  "opencc/hk2s.json",
  "opencc/HKVariantsRev.ocd2",
  "opencc/HKVariantsRevPhrases.ocd2",
  "opencc/TSCharacters.ocd2",
  "opencc/TSPhrases.ocd2",
  "jyut6ping3.table.bin",
  "jyut6ping3.reverse.bin",
  "jyut6ping3_mobile.prism.bin",
  "jyut6ping3_scolar.table.bin",
  "jyut6ping3_scolar.reverse.bin",
  "jyut6ping3_scolar.prism.bin",
  "luna_pinyin.table.bin",
  "luna_pinyin.reverse.bin",
  "luna_pinyin.prism.bin",
] as const;
const PLAYGROUND_SCHEMAS: Record<RimeSchemaId, PlaygroundSchema> = {
  jyut6ping3_mobile: {
    id: "jyut6ping3_mobile",
    name: "Jyutping",
    dictionaryId: "jyut6ping3",
  },
  cangjie5: {
    id: "cangjie5",
    name: "Cangjie 5",
    dictionaryId: "cangjie5",
  },
  luna_pinyin: {
    id: "luna_pinyin",
    name: "Luna Pinyin",
    dictionaryId: "luna_pinyin",
  },
};
let yuneModule: YuneWebBrowserModule | null = null;
let loadedExtraSharedAssets: { path: string; content: string | Uint8Array }[] = [];
let activeSchemaId: RimeSchemaId = DEFAULT_SCHEMA_ID;
let publicAssetManifest: PublicAssetManifest | null = null;
let publicAssetCacheStats: PublicAssetCacheStats = { hits: 0, misses: 0, unavailable: false };

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
  const markStartup = (phase: string) => {
    startupMarkers.push({ phase, ms: Math.round(performance.now() - startupStartedAt) });
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
    markStartup("module:created");

    if (module.IDBFS === undefined || module.IDBFS === null) {
      throw new Error("Yune Emscripten module missing IDBFS runtime method");
    }

    markStartup("filesystem:mount:start");
    mountYuneWebPersistence(module.FS, module.IDBFS, {}, RIME_USER_DIR);
    yuneModule = module;
    markStartup("filesystem:mount:finish");
    markStartup("persistence:mounted");

    markStartup("assets:load:start");
    if (YUNE_PUBLIC_DEMO) {
      publicAssetCacheStats = { hits: 0, misses: 0, unavailable: false };
      loadedExtraSharedAssets = await loadPublicSharedAssets();
    } else {
      loadedExtraSharedAssets = await loadExtraSharedAssets([
        "default.custom.yaml",
        "common.yaml",
        "common.custom.yaml",
        "include.yaml",
        "template.yaml",
        "jyut6ping3.schema.yaml",
        "jyut6ping3_mobile_longpress.schema.yaml",
        "jyut6ping3_mobile_10keys.schema.yaml",
        "jyut6ping3_scolar.schema.yaml",
        "jyut6ping3_scolar.dict.yaml",
        "luna_pinyin.schema.yaml",
        "luna_pinyin.dict.yaml",
        "luna_pinyin_yune_reverse.dict.yaml",
        "loengfan.schema.yaml",
        "loengfan.dict.yaml",
        "loengfan_longpress.schema.yaml",
        "cangjie3.schema.yaml",
        "cangjie3.dict.yaml",
        "cangjie5.schema.yaml",
        "cangjie5.dict.yaml",
        "opencc/hk2s.json",
        "opencc/HKVariantsRev.ocd2",
        "opencc/HKVariantsRevPhrases.ocd2",
        "opencc/TSCharacters.ocd2",
        "opencc/TSPhrases.ocd2",
      ]);
      loadedExtraSharedAssets.push(...await loadExtraSharedAssets([
        "jyut6ping3.table.bin",
        "jyut6ping3.reverse.bin",
        "jyut6ping3_mobile.prism.bin",
        "jyut6ping3_scolar.table.bin",
        "jyut6ping3_scolar.reverse.bin",
        "jyut6ping3_scolar.prism.bin",
        "cangjie5.table.bin",
        "cangjie5.reverse.bin",
        "cangjie5.prism.bin",
        "luna_pinyin.table.bin",
        "luna_pinyin.reverse.bin",
        "luna_pinyin.prism.bin",
      ], true));
    }
    markStartup("assets:load:finish");
    markStartup("assets:loaded");

    markStartup("schema:select:start");
    await selectYuneSchema(DEFAULT_SCHEMA_ID);
    markStartup("schema:select:finish");
    markStartup("runtime:init:finish");
    markStartup("runtime:initialized");

    loading = false;
    dispatch("initialized", true);
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
        schema: DEFAULT_SCHEMA_ID,
        wasmBuildProfile: YUNE_WEB_WASM_BUILD_PROFILE,
        wasmGlue: "yune-web.js",
        wasmBinary: "yune-web.wasm",
        assetCache: YUNE_PUBLIC_DEMO ? publicAssetCacheStats : undefined,
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
  const assets = await Promise.all(
    YUNE_WEB_PUBLIC_SHARED_ASSETS.map(async (path) => ({
      path,
      content: await loadPublicSchemaAsset(path),
    })),
  );
  return assets;
}

async function loadPublicSchemaAsset(path: string): Promise<string | Uint8Array> {
  const entry = await publicAssetManifestEntry(path);
  const sourceUrl = `schema/${path}`;
  if (typeof caches === "undefined") {
    publicAssetCacheStats.unavailable = true;
    return loadAssetContent({ type: "url", url: `${sourceUrl}?sha256=${entry.sha256}` });
  }

  const cache = await caches.open(`yune-web-assets-${YUNE_WEB_M31_EVIDENCE_VERSION}`);
  const cacheUrl = new URL(sourceUrl, location.href);
  cacheUrl.searchParams.set("sha256", entry.sha256);
  const cacheRequest = new Request(cacheUrl.toString());
  const cached = await cache.match(cacheRequest);
  if (cached !== undefined) {
    publicAssetCacheStats.hits += 1;
    return responseAssetContent(cached, path);
  }

  const response = await fetch(sourceUrl, { cache: "force-cache" });
  if (!response.ok) {
    throw new Error(`Asset URL loading failed: ${sourceUrl} (${response.status})`);
  }
  await cache.put(cacheRequest, response.clone());
  publicAssetCacheStats.misses += 1;
  return responseAssetContent(response, path);
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

async function selectYuneSchema(schemaId: RimeSchemaId, preserveDeployedAssets = false): Promise<void> {
  const module = yuneModule;
  if (module === null) {
    throw new Error("Yune module is not loaded");
  }
  if (YUNE_PUBLIC_DEMO && schemaId !== DEFAULT_SCHEMA_ID) {
    throw new Error(`yune-web public demo does not expose schema: ${schemaId}`);
  }
  const schema = PLAYGROUND_SCHEMAS[schemaId];
  if (schema === undefined) {
    throw new Error(`Unknown Yune schema: ${schemaId}`);
  }
  const assetsConfig: ExplicitYuneWebAssets = {
    defaultYaml: await schemaAssetSource("default.yaml"),
    schemaYaml: await schemaAssetSource(`${schema.id}.schema.yaml`),
    dictionaryYaml: await schemaAssetSource(`${schema.dictionaryId}.dict.yaml`),
    deployedDefaultYaml: await schemaAssetSource("build/default.yaml"),
    deployedSchemaYaml: await schemaAssetSource(`build/${schema.id}.schema.yaml`),
  };
  const assets = await loadExplicitAssets(assetsConfig);
  validateExplicitAssets(assets);
  await initYuneRuntime(
    module,
    module.FS,
    {
      sharedDataDir: RIME_SHARED_DIR,
      userDataDir: RIME_USER_DIR,
      schemaId: schema.id,
    },
    assets,
    schema.dictionaryId,
    loadedExtraSharedAssets,
    preserveDeployedAssets,
    YUNE_WEB_ASSET_VERSION,
  );
  activeSchemaId = schema.id;
  dispatch("schemaChanged", schema.id, schema.name);
}

async function schemaAssetSource(path: string): Promise<AssetSource> {
  if (!YUNE_PUBLIC_DEMO) {
    return { type: "url", url: `schema/${path}` };
  }
  return { type: "content", content: await loadPublicSchemaAsset(path) };
}

addEventListener("message", async ({ data: { name, args } }: MessageEvent<Message>) => {
  if (loading) await loadRime;
  const workerStartedAt = nowMs();
  try {
    // @ts-expect-error Unactionable
    const result = await actions[name](...args);
    const workerFinishedAt = nowMs();
    postMessage({ type: "success", result, elapsedMs: Math.round(workerFinishedAt - workerStartedAt), workerStartedAt, workerFinishedAt });
  }
  catch (error) {
    const workerFinishedAt = nowMs();
    postMessage({ type: "error", error, elapsedMs: Math.round(workerFinishedAt - workerStartedAt), workerStartedAt, workerFinishedAt });
  }
});

// Cleanup on worker termination
addEventListener("unload", () => {
  cleanupYuneRuntime();
});
