/**
 * Yune seam adapter for yune-web
 *
 * This adapter bridges the upstream-derived Actions interface to the Yune runtime
 * (@yune-ime/yune-web-runtime). It preserves one active YuneWebRuntime per
 * Emscripten Module and handles lifecycle cleanup per D-05.
 *
 * Contract:
 * - Replaces upstream src/worker.ts librime/WASM binding
 * - Delegates to YuneWebRuntime, keyEventToRimeKey, filesystem helpers per D-04
 * - Enforces one-live-runtime-per-Module constraint
 * - Translates YuneWebResponse to RimeResult shape for upstream compatibility
 * - Calls cleanup() deterministically when worker tears down
 */

import {
  YuneWebRuntime,
  type EmscriptenYuneWebModule,
  type YuneWebInitOptions,
  keyEventToRimeKey,
  type YuneWebKeyboardEventLike,
  joinYuneWebVirtualPath,
  prepareYuneWebFilesystem,
  syncFromPersistenceBeforeInit,
  syncToPersistenceAfterMutation,
  syncAfterUserDataChange,
  type YuneWebFilesystem,
  type YuneWebFilesystemAssets,
  type PrepareYuneWebFilesystemOptions,
} from "@yune-ime/yune-web-runtime";

import { translateResponse, type RimeResult } from "./response.js";

/**
 * Upstream Actions interface from src/types.ts
 */
export interface Actions {
  setOption(option: string, value: boolean): Promise<void>;
  processKey(input: string): Promise<RimeResult>;
  stageAi(): Promise<RimeResult>;
  selectCandidate(index: number): Promise<RimeResult>;
  deleteCandidate(index: number): Promise<RimeResult>;
  flipPage(backward: boolean): Promise<RimeResult>;
  customize(preferences: RimePreferences): Promise<boolean>;
  deploy(): Promise<boolean>;
}

/**
 * Upstream preferences shape from src/types.ts
 */
export interface RimePreferences {
  pageSize?: number;
  enableCompletion?: boolean;
  enableCorrection?: boolean;
  enableSentence?: boolean;
  enableLearning?: boolean;
  enableAI?: boolean;
  combineCandidates?: boolean;
  predictionNeverFirst?: boolean;
  predictionThreshold?: number;
  isCangjie5?: boolean;
  dictionaryExclude?: string[];
  /** Pre-2024 options encoding */
  options?: number;
}

/**
 * Upstream listener event types from src/types.ts
 */
export type ListenerEvent =
  | "deployStatusChanged"
  | "schemaChanged"
  | "optionChanged"
  | "initialized";

/**
 * Adapter state: one active runtime per Module
 */
let currentRuntime: YuneWebRuntime | null = null;
let currentModule: EmscriptenYuneWebModule | null = null;
let currentFs: YuneWebFilesystem | null = null;
let currentSchemaId: string | null = null;
let currentPrepareOptions: PrepareYuneWebFilesystemOptions | null = null;
let currentExtraSharedAssets: YuneWebExtraSharedAsset[] = [];
let currentAssetVersion = "unstamped";
let lastKeyResult: RimeResult = { isComposing: false, success: true };
const neutralKeyResult: RimeResult = { isComposing: false, success: true };
const passThroughModifierKeys = new Set([
  "Alt",
  "Alt_L",
  "Alt_R",
  "Control",
  "Control_L",
  "Control_R",
  "Meta",
  "Meta_L",
  "Meta_R",
  "Shift",
  "Shift_L",
  "Shift_R",
  "Super",
  "Super_L",
  "Super_R",
]);

type BooleanRimePreference =
  | "enableCompletion"
  | "enableCorrection"
  | "enableSentence"
  | "enableLearning";

const BOOLEAN_CUSTOMIZATION_KEYS: readonly {
  preference: BooleanRimePreference;
  keys: readonly string[];
}[] = [
  { preference: "enableCompletion", keys: ["translator/enable_completion"] },
  { preference: "enableCorrection", keys: ["translator/enable_correction"] },
  { preference: "enableSentence", keys: ["translator/enable_sentence"] },
  {
    preference: "enableLearning",
    keys: ["translator/enable_user_dict", "translator/encode_commit_history"],
  },
];

export interface YuneWebExtraSharedAsset {
  path: string;
  content: string | Uint8Array;
}

type PersistenceSyncReason =
  | "before-init"
  | "after-init"
  | "commit"
  | "select-candidate"
  | "delete-candidate"
  | "customize"
  | "deploy";

type PersistenceDiagnosticPhase =
  | "deploy:cache-hit"
  | "deploy:cache-miss"
  | "runtime:init:start"
  | "runtime:init:finish"
  | "rime:init:start"
  | "rime:init:finish"
  | "schema:deploy:start"
  | "schema:deploy:finish"
  | "syncFromPersistenceBeforeInit:start"
  | "syncFromPersistenceBeforeInit:pass"
  | "syncToPersistenceAfterMutation:start"
  | "syncToPersistenceAfterMutation:pass"
  | "runtime:init";

interface PersistedConfigSnapshot {
  path: string;
  exists: boolean;
  pageSize?: string | null;
  settings?: Record<string, string | null>;
  bytes?: number;
  readError?: string;
}

const PERSISTED_CUSTOM_CONFIG_KEYS = [
  "menu/page_size",
  "translator/enable_completion",
  "translator/enable_correction",
  "translator/enable_sentence",
  "translator/enable_user_dict",
  "translator/encode_commit_history",
  "translator/combine_candidates",
  "translator/prediction_never_first",
  "translator/prediction_weight_threshold",
  "translator/dictionary_exclude",
  "cangjie/dictionary",
  "cangjie/tips",
] as const;

export interface YunePersistenceDiagnostic {
  phase: PersistenceDiagnosticPhase;
  reason: PersistenceSyncReason;
  schemaId: string;
  userDataDir: string;
  timestamp: string;
  assetVersion: string;
  persistedConfig: PersistedConfigSnapshot;
  deployedConfig: PersistedConfigSnapshot;
}

interface DeployStamp {
  version: 1;
  assetVersion: string;
  schemaId: string;
  dictionaryId: string;
  assetSignature: string;
  customConfigSignature: string;
}

interface CustomPatchEntry {
  key: string;
  value: boolean | number | string | string[];
}

/**
 * Initialize Yune runtime with Emscripten Module and filesystem
 *
 * Replaces upstream worker.ts loadRime() and init sequence
 */
export async function initYuneRuntime(
  module: EmscriptenYuneWebModule,
  fs: YuneWebFilesystem,
  options: YuneWebInitOptions,
  assets: YuneWebFilesystemAssets,
  dictionaryId: string,
  extraSharedAssets: YuneWebExtraSharedAsset[] = [],
  preserveDeployedAssets = false,
  assetVersion = "unstamped",
): Promise<void> {
  // Cleanup previous runtime if exists (one-active-runtime constraint)
  if (currentRuntime !== null) {
    currentRuntime.cleanup();
    currentRuntime = null;
  }

  currentModule = module;
  currentFs = fs;
  currentSchemaId = options.schemaId;
  currentAssetVersion = assetVersion;
  lastKeyResult = { isComposing: false, success: true };

  // Prepare filesystem with explicit assets per D-06
  const prepareOptions: PrepareYuneWebFilesystemOptions = {
    sharedDataDir: options.sharedDataDir,
    userDataDir: options.userDataDir,
    schemaId: options.schemaId,
    dictionaryId,
    assets,
  };
  currentPrepareOptions = prepareOptions;
  currentExtraSharedAssets = extraSharedAssets;

  emitPersistenceDiagnostic(fs, prepareOptions, "runtime:init:start", "before-init");

  // Load persisted user/build state before writing fresh app-owned assets.
  emitPersistenceDiagnostic(fs, prepareOptions, "syncFromPersistenceBeforeInit:start", "before-init");
  await syncFromPersistenceBeforeInit(fs);
  emitPersistenceDiagnostic(fs, prepareOptions, "syncFromPersistenceBeforeInit:pass", "before-init");

  emitPersistenceDiagnostic(fs, prepareOptions, "schema:deploy:start", "before-init");
  const deployCacheFresh = isDeployCacheFresh(
    fs,
    prepareOptions,
    extraSharedAssets,
    currentAssetVersion,
  );
  if (preserveDeployedAssets || deployCacheFresh) {
    prepareYuneWebDeployFilesystem(fs, prepareOptions);
    if (deployCacheFresh) {
      emitPersistenceDiagnostic(fs, prepareOptions, "deploy:cache-hit", "before-init");
    }
  } else {
    prepareYuneWebFilesystem(fs, prepareOptions);
    emitPersistenceDiagnostic(fs, prepareOptions, "deploy:cache-miss", "before-init");
  }
  for (const asset of extraSharedAssets) {
    writeExtraSharedAsset(fs, options.sharedDataDir, asset);
  }
  emitPersistenceDiagnostic(fs, prepareOptions, "schema:deploy:finish", "before-init");

  // Initialize runtime
  emitPersistenceDiagnostic(fs, prepareOptions, "rime:init:start", "after-init");
  currentRuntime = YuneWebRuntime.init(module, options);
  emitPersistenceDiagnostic(fs, prepareOptions, "rime:init:finish", "after-init");
  emitPersistenceDiagnostic(fs, prepareOptions, "runtime:init", "after-init");

  // Sync after init to persist initial state
  await syncToPersistenceWithDiagnostic(fs, prepareOptions, "after-init");
  emitPersistenceDiagnostic(fs, prepareOptions, "runtime:init:finish", "after-init");
}

/**
 * Cleanup current runtime deterministically
 *
 * Call when worker tears down or before re-initialization
 */
export function cleanupYuneRuntime(): void {
  if (currentRuntime !== null) {
    currentRuntime.cleanup();
    currentRuntime = null;
  }
  currentModule = null;
  currentFs = null;
  currentSchemaId = null;
  currentPrepareOptions = null;
  currentExtraSharedAssets = [];
  currentAssetVersion = "unstamped";
  lastKeyResult = { isComposing: false, success: true };
}

/**
 * Parse upstream key sequence string to keyboard event-like object
 *
 * Upstream CandidatePanel.tsx sends strings like "{BackSpace}", "a", "{Release+Enter}"
 * This adapter translates to YuneWebKeyboardEventLike for keyEventToRimeKey
 */
function parseKeySequence(input: string): YuneWebKeyboardEventLike {
  // Release prefix
  const isRelease = input.startsWith("{Release+");
  const type = isRelease ? "keyup" : "keydown";

  // Extract key name
  let key: string;
  let shiftKey = false;
  let ctrlKey = false;
  let altKey = false;
  let metaKey = false;
  if (input.startsWith("{") && input.endsWith("}")) {
    // Special key wrapped in braces
    const inner = input.slice(1, -1);
    const parts = (isRelease ? inner.replace("Release+", "") : inner).split("+");
    key = parts.pop() ?? "";
    for (const modifier of parts) {
      switch (modifier) {
        case "Shift":
          shiftKey = true;
          break;
        case "Control":
          ctrlKey = true;
          break;
        case "Alt":
          altKey = true;
          break;
        case "Meta":
        case "Super":
          metaKey = true;
          break;
      }
    }
    // Normalize key names
    if (key === "BackSpace") key = "Backspace";
    if (key === "Page_Up") key = "PageUp";
    if (key === "Page_Down") key = "PageDown";
    if (key === "Return") key = "Enter";
    if (key === "space") key = " ";
    if (key === "Esc") key = "Escape";
    if (key === "Prior") key = "PageUp";
    if (key === "Next") key = "PageDown";
  } else {
    // Printable key sent directly
    key = input;
  }

  return { key, type, shiftKey, ctrlKey, altKey, metaKey };
}

/**
 * Process key event using Yune runtime
 *
 * Replaces upstream Module.ccall("process_key", ...)
 */
export async function processKey(input: string): Promise<RimeResult> {
  if (currentRuntime === null) {
    throw new Error("Yune runtime not initialized");
  }

  // Parse upstream key sequence to event-like object
  const eventLike = parseKeySequence(input);

  if (eventLike.type === "keyup") {
    return lastKeyResult.isComposing ? lastKeyResult : neutralKeyResult;
  }
  if (passThroughModifierKeys.has(eventLike.key)) {
    return lastKeyResult.isComposing ? lastKeyResult : neutralKeyResult;
  }

  // Delegate to Yune runtime via keyEventToRimeKey per D-04
  const response = currentRuntime.processKeyboardEvent(eventLike);

  // Translate to upstream RimeResult
  const result = translateResponse(response);
  lastKeyResult = result;

  // Sync persistence after commit
  if (result.committed && currentFs !== null) {
    await syncCurrentStateToPersistence("commit");
  }

  return result;
}

export async function stageAi(): Promise<RimeResult> {
  if (currentRuntime === null) {
    throw new Error("Yune runtime not initialized");
  }

  const response = currentRuntime.stageAi();
  const result = translateResponse(response);
  lastKeyResult = result;
  return result;
}

/**
 * Select candidate using Yune runtime
 *
 * Replaces upstream Module.ccall("select_candidate", ...)
 */
export async function selectCandidate(index: number): Promise<RimeResult> {
  if (currentRuntime === null) {
    throw new Error("Yune runtime not initialized");
  }

  const response = currentRuntime.selectCandidate(index);
  const result = translateResponse(response);

  // Sync persistence after commit
  if (result.committed && currentFs !== null) {
    await syncCurrentStateToPersistence("select-candidate");
  }

  return result;
}

/**
 * Delete candidate using Yune runtime
 *
 * Replaces upstream Module.ccall("delete_candidate", ...)
 */
export async function deleteCandidate(index: number): Promise<RimeResult> {
  if (currentRuntime === null) {
    throw new Error("Yune runtime not initialized");
  }

  const response = currentRuntime.deleteCandidate(index);
  const result = translateResponse(response);

  // Sync persistence after user data change
  if (currentFs !== null && currentPrepareOptions !== null) {
    emitPersistenceDiagnostic(currentFs, currentPrepareOptions, "syncToPersistenceAfterMutation:start", "delete-candidate");
    await syncAfterUserDataChange(currentFs);
    emitPersistenceDiagnostic(currentFs, currentPrepareOptions, "syncToPersistenceAfterMutation:pass", "delete-candidate");
  }

  return result;
}

/**
 * Flip page using Yune runtime
 *
 * Replaces upstream Module.ccall("flip_page", ...)
 */
export async function flipPage(backward: boolean): Promise<RimeResult> {
  if (currentRuntime === null) {
    throw new Error("Yune runtime not initialized");
  }

  const response = currentRuntime.flipPage(backward);
  return translateResponse(response);
}

/**
 * Deploy schema using Yune runtime and sync persistence
 *
 * Replaces upstream Module.ccall("deploy", ...)
 */
export async function deploy(): Promise<boolean> {
  if (currentRuntime === null || currentFs === null || currentPrepareOptions === null) {
    throw new Error("Yune runtime not initialized");
  }

  prepareYuneWebDeployFilesystem(currentFs, currentPrepareOptions);
  for (const asset of currentExtraSharedAssets) {
    writeExtraSharedAsset(currentFs, currentPrepareOptions.sharedDataDir, asset);
  }

  emitPersistenceDiagnostic(currentFs, currentPrepareOptions, "schema:deploy:start", "deploy");
  if (isDeployCacheFresh(currentFs, currentPrepareOptions, currentExtraSharedAssets, currentAssetVersion)) {
    emitPersistenceDiagnostic(currentFs, currentPrepareOptions, "deploy:cache-hit", "deploy");
    emitPersistenceDiagnostic(currentFs, currentPrepareOptions, "schema:deploy:finish", "deploy");
    return true;
  }

  emitPersistenceDiagnostic(currentFs, currentPrepareOptions, "deploy:cache-miss", "deploy");
  invalidateDeployedSchema(currentFs, currentPrepareOptions);
  const deployed = currentRuntime.deploy();
  if (deployed) {
    writeDeployStamp(currentFs, currentPrepareOptions, currentExtraSharedAssets, currentAssetVersion);
  }
  emitPersistenceDiagnostic(currentFs, currentPrepareOptions, "schema:deploy:finish", "deploy");
  await syncCurrentStateToPersistence("deploy");
  return deployed;
}

/**
 * Customize preferences using Yune runtime and sync persistence
 *
 * Replaces upstream Module.ccall("customize", ...)
 *
 * Note: upstream YuneWeb used pageSize and an options bitmap.
 * Yune customize API accepts configId, key, value strings.
 * This adapter maps preferences to Yune customize calls.
 */
export async function customize(preferences: RimePreferences): Promise<boolean> {
  if (currentRuntime === null || currentFs === null || currentSchemaId === null || currentPrepareOptions === null) {
    throw new Error("Yune runtime not initialized");
  }

  const runtime = currentRuntime;
  const fs = currentFs;
  const prepareOptions = currentPrepareOptions;
  const schemaId = currentSchemaId;

  // Map preferences to Yune customize calls
  let success = true;
  let customizedAny = false;
  const customPatchEntries: CustomPatchEntry[] = [];

  if (preferences.enableAI !== undefined) {
    success = runtime.setAiEnabled(preferences.enableAI) && success;
  }

  const customizeSetting = (
    key: string,
    value: string,
    patchValue: CustomPatchEntry["value"] = value,
  ): void => {
    customPatchEntries.push({ key, value: patchValue });
    if (persistedCustomizationMatches(fs, prepareOptions, key, value)) {
      return;
    }
    const customized = runtime.customize(`${schemaId}.schema`, key, value);
    success = success && customized;
    customizedAny = true;
  };

  if (preferences.pageSize !== undefined) {
    customizeSetting("menu/page_size", String(preferences.pageSize), preferences.pageSize);
  }

  for (const { preference, keys } of BOOLEAN_CUSTOMIZATION_KEYS) {
    const value = preferences[preference];
    if (value === undefined) {
      continue;
    }
    for (const key of keys) {
      customizeSetting(key, value ? "true" : "false", value);
    }
  }

  if (preferences.combineCandidates !== undefined) {
    customizeSetting(
      "translator/combine_candidates",
      preferences.combineCandidates ? "true" : "false",
      preferences.combineCandidates,
    );
  }

  if (preferences.predictionNeverFirst !== undefined) {
    customizeSetting(
      "translator/prediction_never_first",
      preferences.predictionNeverFirst ? "true" : "false",
      preferences.predictionNeverFirst,
    );
  }

  if (preferences.predictionThreshold !== undefined) {
    customizeSetting(
      "translator/prediction_weight_threshold",
      String(preferences.predictionThreshold),
      preferences.predictionThreshold,
    );
  }

  if (preferences.dictionaryExclude !== undefined) {
    customizeSetting(
      "translator/dictionary_exclude",
      JSON.stringify(preferences.dictionaryExclude),
      preferences.dictionaryExclude,
    );
  }

  if (preferences.isCangjie5 !== undefined) {
    customizeSetting(
      "cangjie/dictionary",
      preferences.isCangjie5 ? "cangjie5" : "cangjie3",
    );
    customizeSetting(
      "cangjie/tips",
      preferences.isCangjie5 ? "【倉頡五代】" : "【倉頡三代】",
    );
  }

  if (writeHarnessCustomConfig(fs, prepareOptions, customPatchEntries)) {
    customizedAny = true;
  }

  if (customizedAny) {
    await syncCurrentStateToPersistence("customize");
  }

  return success;
}

function writeHarnessCustomConfig(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
  entries: CustomPatchEntry[],
): boolean {
  if (entries.length === 0) {
    return false;
  }
  const path = joinYuneWebVirtualPath(options.userDataDir, `${options.schemaId}.custom.yaml`);
  const lines = [
    "# Rime custom settings",
    "# encoding: utf-8",
    "",
    "patch:",
    ...entries.flatMap(entry => yamlPatchEntryLines(entry)),
    "customization:",
    "  generator: yune-web",
    `  modified_time: ${Math.floor(Date.now() / 1000)}`,
    "",
  ];
  const text = lines.join("\n");
  const existing = fs.analyzePath(path).exists
    ? fs.readFile(path, { encoding: "utf8" })
    : null;
  const existingText = typeof existing === "string"
    ? existing
    : existing instanceof Uint8Array
    ? new TextDecoder().decode(existing)
    : null;
  if (existingText === text) {
    return false;
  }
  ensureVirtualDirectory(fs, path.split("/").slice(0, -1).join("/"));
  fs.writeFile(path, text, { flags: "w" });
  return true;
}

function yamlPatchEntryLines(entry: CustomPatchEntry): string[] {
  if (Array.isArray(entry.value)) {
    if (entry.value.length === 0) {
      return [`  ${entry.key}: []`];
    }
    return [
      `  ${entry.key}:`,
      ...entry.value.map(item => `    - ${yamlScalar(item)}`),
    ];
  }
  return [`  ${entry.key}: ${yamlScalar(entry.value)}`];
}

function yamlScalar(value: boolean | number | string): string {
  if (typeof value === "boolean") {
    return value ? "true" : "false";
  }
  if (typeof value === "number") {
    return String(value);
  }
  return JSON.stringify(value);
}

export async function setOption(option: string, value: boolean): Promise<void> {
  if (currentRuntime === null) {
    throw new Error("Yune runtime not initialized");
  }
  if (!currentRuntime.setOption(option, value)) {
    throw new Error(`Yune setOption failed: ${option}`);
  }
}

function writeExtraSharedAsset(
  fs: YuneWebFilesystem,
  sharedDataDir: string,
  asset: YuneWebExtraSharedAsset,
): void {
  if (
    asset.path.length === 0 ||
    asset.path.startsWith("/") ||
    asset.path.includes("\\") ||
    asset.path.split("/").includes("..")
  ) {
    throw new Error(`Invalid YuneWeb shared asset path: ${asset.path}`);
  }

  const fullPath = joinYuneWebVirtualPath(sharedDataDir, asset.path);
  ensureVirtualDirectory(fs, fullPath.split("/").slice(0, -1).join("/"));
  fs.writeFile(fullPath, asset.content, { flags: "w" });
}

function prepareYuneWebDeployFilesystem(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
): void {
  ensureVirtualDirectory(fs, options.sharedDataDir);
  ensureVirtualDirectory(fs, options.userDataDir);
  ensureVirtualDirectory(fs, joinYuneWebVirtualPath(options.userDataDir, "build"));
  fs.writeFile(joinYuneWebVirtualPath(options.sharedDataDir, "default.yaml"), options.assets.defaultYaml, {
    flags: "w",
  });
  fs.writeFile(
    joinYuneWebVirtualPath(options.sharedDataDir, `${options.schemaId}.schema.yaml`),
    options.assets.schemaYaml,
    { flags: "w" },
  );
  fs.writeFile(
    joinYuneWebVirtualPath(options.sharedDataDir, `${options.dictionaryId}.dict.yaml`),
    options.assets.dictionaryYaml,
    { flags: "w" },
  );
}

function invalidateDeployedSchema(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
): void {
  fs.writeFile(
    joinYuneWebVirtualPath(options.userDataDir, "build", `${options.schemaId}.schema.yaml`),
    "# stale before Yune deploy\n",
    { flags: "w" },
  );
}

function deployStampPath(options: PrepareYuneWebFilesystemOptions): string {
  return joinYuneWebVirtualPath(options.userDataDir, "build", ".yune-deploy-stamp.json");
}

function isDeployCacheFresh(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
  extraSharedAssets: YuneWebExtraSharedAsset[],
  assetVersion: string,
): boolean {
  const expected = expectedDeployStamp(fs, options, extraSharedAssets, assetVersion);
  const actual = readDeployStamp(fs, options);
  if (snapshotPersistedCustomConfig(fs, options).exists) {
    return false;
  }
  return actual !== null
    && actual.version === expected.version
    && actual.assetVersion === expected.assetVersion
    && actual.schemaId === expected.schemaId
    && actual.dictionaryId === expected.dictionaryId
    && actual.assetSignature === expected.assetSignature
    && actual.customConfigSignature === expected.customConfigSignature
    && deployedSchemaExists(fs, options)
    && deployedCustomSettingsMatch(fs, options);
}

function deployedCustomSettingsMatch(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
): boolean {
  const persisted = snapshotPersistedCustomConfig(fs, options).settings;
  if (persisted === undefined) {
    return true;
  }
  const deployed = snapshotDeployedSchemaConfig(fs, options).settings;
  if (deployed === undefined) {
    return false;
  }
  return PERSISTED_CUSTOM_CONFIG_KEYS.every((key) => {
    const persistedValue = persisted[key];
    return persistedValue === null || deployed[key] === persistedValue;
  });
}

function readDeployStamp(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
): DeployStamp | null {
  const path = deployStampPath(options);
  if (!fs.analyzePath(path).exists) {
    return null;
  }
  try {
    const raw = fs.readFile(path, { encoding: "utf8" });
    const text = typeof raw === "string" ? raw : new TextDecoder().decode(raw);
    const parsed = JSON.parse(text) as Partial<DeployStamp>;
    if (
      parsed.version === 1
      && typeof parsed.assetVersion === "string"
      && typeof parsed.schemaId === "string"
      && typeof parsed.dictionaryId === "string"
      && typeof parsed.assetSignature === "string"
      && typeof parsed.customConfigSignature === "string"
    ) {
      return parsed as DeployStamp;
    }
  } catch {
    return null;
  }
  return null;
}

function writeDeployStamp(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
  extraSharedAssets: YuneWebExtraSharedAsset[],
  assetVersion: string,
): void {
  const path = deployStampPath(options);
  ensureVirtualDirectory(fs, path.split("/").slice(0, -1).join("/"));
  fs.writeFile(
    path,
    `${JSON.stringify(expectedDeployStamp(fs, options, extraSharedAssets, assetVersion), null, 2)}\n`,
    { flags: "w" },
  );
}

function expectedDeployStamp(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
  extraSharedAssets: YuneWebExtraSharedAsset[],
  assetVersion: string,
): DeployStamp {
  return {
    version: 1,
    assetVersion,
    schemaId: options.schemaId,
    dictionaryId: options.dictionaryId,
    assetSignature: assetSignature(options, extraSharedAssets, assetVersion),
    customConfigSignature: customConfigSignature(fs, options),
  };
}

function deployedSchemaExists(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
): boolean {
  return fs.analyzePath(
    joinYuneWebVirtualPath(options.userDataDir, "build", `${options.schemaId}.schema.yaml`),
  ).exists;
}

function customConfigSignature(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
): string {
  const path = joinYuneWebVirtualPath(options.userDataDir, `${options.schemaId}.custom.yaml`);
  if (!fs.analyzePath(path).exists) {
    return "missing";
  }
  try {
    const raw = fs.readFile(path, { encoding: "utf8" });
    return contentSignature(typeof raw === "string" ? raw : raw);
  } catch (error) {
    return `error:${error instanceof Error ? error.message : String(error)}`;
  }
}

function persistedCustomizationMatches(
  fs: YuneWebFilesystem,
  options: PrepareYuneWebFilesystemOptions,
  key: string,
  value: string,
): boolean {
  const persistedValue = snapshotPersistedCustomConfig(fs, options).settings?.[key];
  return persistedValue === value;
}

function assetSignature(
  options: PrepareYuneWebFilesystemOptions,
  extraSharedAssets: YuneWebExtraSharedAsset[],
  assetVersion: string,
): string {
  const parts = [
    `assetVersion:${assetVersion}`,
    `schema:${options.schemaId}`,
    `dictionary:${options.dictionaryId}`,
    `default:${contentSignature(options.assets.defaultYaml)}`,
    `schemaYaml:${contentSignature(options.assets.schemaYaml)}`,
    `dictionaryYaml:${contentSignature(options.assets.dictionaryYaml)}`,
    `deployedDefault:${contentSignature(options.assets.deployedDefaultYaml ?? options.assets.defaultYaml)}`,
    `deployedSchema:${contentSignature(options.assets.deployedSchemaYaml ?? options.assets.schemaYaml)}`,
    ...extraSharedAssets
      .slice()
      .sort((left, right) => left.path.localeCompare(right.path))
      .map((asset) => `extra:${asset.path}:${contentSignature(asset.content)}`),
  ];
  return fnv1a(parts.join("\n"));
}

function contentSignature(content: string | Uint8Array): string {
  if (typeof content === "string") {
    return `${content.length}:${fnv1a(content)}`;
  }
  const sample = content.length <= 256
    ? content
    : new Uint8Array([
      ...content.slice(0, 128),
      ...content.slice(Math.max(0, content.length - 128)),
    ]);
  return `${content.length}:${fnv1aBytes(sample)}`;
}

function fnv1a(text: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

function fnv1aBytes(bytes: Uint8Array): string {
  let hash = 0x811c9dc5;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

function ensureVirtualDirectory(fs: YuneWebFilesystem, path: string): void {
  if (fs.analyzePath(path).exists) {
    return;
  }
  if (fs.mkdirTree !== undefined) {
    fs.mkdirTree(path);
    return;
  }
  if (fs.mkdir === undefined) {
    throw new Error(`YuneWeb filesystem cannot create directory: ${path}`);
  }
  const segments = path.split("/").filter((segment) => segment.length > 0);
  let current = path.startsWith("/") ? "/" : "";
  for (const segment of segments) {
    current = current === "/" || current === "" ? `${current}${segment}` : `${current}/${segment}`;
    if (!fs.analyzePath(current).exists) {
      fs.mkdir(current);
    }
  }
}

async function syncCurrentStateToPersistence(reason: PersistenceSyncReason): Promise<void> {
  if (currentFs === null || currentPrepareOptions === null) {
    throw new Error("Yune runtime not initialized");
  }
  await syncToPersistenceWithDiagnostic(currentFs, currentPrepareOptions, reason);
}

async function syncToPersistenceWithDiagnostic(
  fs: YuneWebFilesystem,
  prepareOptions: PrepareYuneWebFilesystemOptions,
  reason: PersistenceSyncReason,
): Promise<void> {
  emitPersistenceDiagnostic(fs, prepareOptions, "syncToPersistenceAfterMutation:start", reason);
  await syncToPersistenceAfterMutation(fs);
  emitPersistenceDiagnostic(fs, prepareOptions, "syncToPersistenceAfterMutation:pass", reason);
}

function emitPersistenceDiagnostic(
  fs: YuneWebFilesystem,
  prepareOptions: PrepareYuneWebFilesystemOptions,
  phase: PersistenceDiagnosticPhase,
  reason: PersistenceSyncReason,
): void {
  const diagnostic: YunePersistenceDiagnostic = {
    phase,
    reason,
    schemaId: prepareOptions.schemaId,
    userDataDir: prepareOptions.userDataDir,
    timestamp: new Date().toISOString(),
    assetVersion: currentAssetVersion,
    persistedConfig: snapshotPersistedCustomConfig(fs, prepareOptions),
    deployedConfig: snapshotDeployedSchemaConfig(fs, prepareOptions),
  };
  console.info(`YUNE_PERSISTENCE ${JSON.stringify(diagnostic)}`);
  const diagnosticGlobal = globalThis as typeof globalThis & {
    onYunePersistenceDiagnostic?: (marker: YunePersistenceDiagnostic) => void;
  };
  diagnosticGlobal.onYunePersistenceDiagnostic?.(diagnostic);
}

function snapshotDeployedSchemaConfig(
  fs: YuneWebFilesystem,
  prepareOptions: PrepareYuneWebFilesystemOptions,
): PersistedConfigSnapshot {
  const path = joinYuneWebVirtualPath(
    prepareOptions.userDataDir,
    "build",
    `${prepareOptions.schemaId}.schema.yaml`,
  );
  return snapshotYamlConfig(fs, path);
}

function snapshotPersistedCustomConfig(
  fs: YuneWebFilesystem,
  prepareOptions: PrepareYuneWebFilesystemOptions,
): PersistedConfigSnapshot {
  const path = joinYuneWebVirtualPath(prepareOptions.userDataDir, `${prepareOptions.schemaId}.custom.yaml`);
  return snapshotYamlConfig(fs, path);
}

function snapshotYamlConfig(
  fs: YuneWebFilesystem,
  path: string,
): PersistedConfigSnapshot {
  if (!fs.analyzePath(path).exists) {
    return { path, exists: false };
  }

  try {
    const file = fs.readFile(path, { encoding: "utf8" });
    const text = typeof file === "string" ? file : new TextDecoder().decode(file);
    const settings = Object.fromEntries(
      PERSISTED_CUSTOM_CONFIG_KEYS.map((key) => [key, readYamlSetting(text, key)]),
    );
    return {
      path,
      exists: true,
      pageSize: settings["menu/page_size"],
      settings,
      bytes: text.length,
    };
  } catch (error) {
    return {
      path,
      exists: true,
      readError: error instanceof Error ? `${error.name}: ${error.message}` : String(error),
    };
  }
}

function readYamlSetting(text: string, key: string): string | null {
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const rawValue = new RegExp(`^\\s*${escapedKey}:[^\\S\\r\\n]*(\\S+)`, "m").exec(text)?.[1] ?? null;
  if (rawValue !== null) {
    return rawValue.replace(/^['"](.+)['"]$/, "$1");
  }

  const sequenceHeader = new RegExp(`^\\s*${escapedKey}:\\s*$`, "m").exec(text);
  if (sequenceHeader === null) {
    const [section, child] = key.split("/", 2);
    if (section !== undefined && child !== undefined) {
      const nested = yamlSectionText(text, section);
      if (nested !== null) {
        return readYamlSetting(nested, child);
      }
    }
    return null;
  }
  const lines = text.slice(sequenceHeader.index).split(/\r?\n/).slice(1);
  const values: string[] = [];
  for (const line of lines) {
    if (/^\S/.test(line) || /^\s*[A-Za-z0-9_/.-]+:/.test(line)) {
      break;
    }
    const item = /^\s*-\s*(.+?)\s*$/.exec(line)?.[1];
    if (item !== undefined) {
      values.push(item.replace(/^['"](.+)['"]$/, "$1"));
    }
  }
  return values.length > 0 ? JSON.stringify(values) : "[]";
}

function yamlSectionText(text: string, section: string): string | null {
  const escapedSection = section.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const sectionHeader = new RegExp(`^${escapedSection}:\\s*$`, "m").exec(text);
  if (sectionHeader === null) {
    return null;
  }
  const lines = text.slice(sectionHeader.index).split(/\r?\n/).slice(1);
  const sectionLines: string[] = [];
  for (const line of lines) {
    if (/^\S/.test(line)) {
      break;
    }
    sectionLines.push(line);
  }
  return sectionLines.join("\n");
}

/**
 * Get current runtime for testing/debugging
 */
export function getCurrentRuntime(): YuneWebRuntime | null {
  return currentRuntime;
}
