/**
 * Yune seam adapter for TypeDuck-Web
 *
 * This adapter bridges TypeDuck-Web's Actions interface to the Yune runtime
 * (@yune-ime/typeduck-runtime). It preserves one active TypeDuckRuntime per
 * Emscripten Module and handles lifecycle cleanup per D-05.
 *
 * Contract:
 * - Replaces upstream src/worker.ts librime/WASM binding
 * - Delegates to TypeDuckRuntime, keyEventToRimeKey, filesystem helpers per D-04
 * - Enforces one-live-runtime-per-Module constraint
 * - Translates TypeDuckResponse to RimeResult shape for upstream compatibility
 * - Calls cleanup() deterministically when worker tears down
 */

import {
  TypeDuckRuntime,
  type EmscriptenTypeDuckModule,
  type TypeDuckInitOptions,
  keyEventToRimeKey,
  type TypeDuckKeyboardEventLike,
  joinTypeDuckVirtualPath,
  prepareTypeDuckFilesystem,
  syncFromPersistenceBeforeInit,
  syncToPersistenceAfterMutation,
  syncAfterUserDataChange,
  type TypeDuckFilesystem,
  type TypeDuckFilesystemAssets,
  type PrepareTypeDuckFilesystemOptions,
} from "@yune-ime/typeduck-runtime";

import { translateResponse, type RimeResult } from "./response.js";

/**
 * Upstream Actions interface from src/types.ts
 */
export interface Actions {
  setOption(option: string, value: boolean): Promise<void>;
  processKey(input: string): Promise<RimeResult>;
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
let currentRuntime: TypeDuckRuntime | null = null;
let currentModule: EmscriptenTypeDuckModule | null = null;
let currentFs: TypeDuckFilesystem | null = null;
let currentSchemaId: string | null = null;
let currentPrepareOptions: PrepareTypeDuckFilesystemOptions | null = null;
let currentExtraSharedAssets: TypeDuckExtraSharedAsset[] = [];
let lastKeyResult: RimeResult = { isComposing: false, success: true };

export interface TypeDuckExtraSharedAsset {
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
  | "syncFromPersistenceBeforeInit:start"
  | "syncFromPersistenceBeforeInit:pass"
  | "syncToPersistenceAfterMutation:start"
  | "syncToPersistenceAfterMutation:pass"
  | "runtime:init";

interface PersistedConfigSnapshot {
  path: string;
  exists: boolean;
  pageSize?: string | null;
  bytes?: number;
  readError?: string;
}

export interface YunePersistenceDiagnostic {
  phase: PersistenceDiagnosticPhase;
  reason: PersistenceSyncReason;
  schemaId: string;
  userDataDir: string;
  timestamp: string;
  persistedConfig: PersistedConfigSnapshot;
}

/**
 * Initialize Yune runtime with Emscripten Module and filesystem
 *
 * Replaces upstream worker.ts loadRime() and init sequence
 */
export async function initYuneRuntime(
  module: EmscriptenTypeDuckModule,
  fs: TypeDuckFilesystem,
  options: TypeDuckInitOptions,
  assets: TypeDuckFilesystemAssets,
  dictionaryId: string,
  extraSharedAssets: TypeDuckExtraSharedAsset[] = [],
): Promise<void> {
  // Cleanup previous runtime if exists (one-active-runtime constraint)
  if (currentRuntime !== null) {
    currentRuntime.cleanup();
    currentRuntime = null;
  }

  currentModule = module;
  currentFs = fs;
  currentSchemaId = options.schemaId;
  lastKeyResult = { isComposing: false, success: true };

  // Prepare filesystem with explicit assets per D-06
  const prepareOptions: PrepareTypeDuckFilesystemOptions = {
    sharedDataDir: options.sharedDataDir,
    userDataDir: options.userDataDir,
    schemaId: options.schemaId,
    dictionaryId,
    assets,
  };
  currentPrepareOptions = prepareOptions;
  currentExtraSharedAssets = extraSharedAssets;

  // Load persisted user/build state before writing fresh app-owned assets.
  emitPersistenceDiagnostic(fs, prepareOptions, "syncFromPersistenceBeforeInit:start", "before-init");
  await syncFromPersistenceBeforeInit(fs);
  emitPersistenceDiagnostic(fs, prepareOptions, "syncFromPersistenceBeforeInit:pass", "before-init");

  prepareTypeDuckFilesystem(fs, prepareOptions);
  for (const asset of extraSharedAssets) {
    writeExtraSharedAsset(fs, options.sharedDataDir, asset);
  }

  // Initialize runtime
  currentRuntime = TypeDuckRuntime.init(module, options);
  emitPersistenceDiagnostic(fs, prepareOptions, "runtime:init", "after-init");

  // Sync after init to persist initial state
  await syncToPersistenceWithDiagnostic(fs, prepareOptions, "after-init");
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
  lastKeyResult = { isComposing: false, success: true };
}

/**
 * Parse upstream key sequence string to keyboard event-like object
 *
 * Upstream CandidatePanel.tsx sends strings like "{BackSpace}", "a", "{Release+Enter}"
 * This adapter translates to TypeDuckKeyboardEventLike for keyEventToRimeKey
 */
function parseKeySequence(input: string): TypeDuckKeyboardEventLike {
  // Release prefix
  const isRelease = input.startsWith("{Release+");
  const type = isRelease ? "keyup" : "keydown";

  // Extract key name
  let key: string;
  if (input.startsWith("{") && input.endsWith("}")) {
    // Special key wrapped in braces
    const inner = input.slice(1, -1);
    if (isRelease) {
      key = inner.replace("Release+", "");
    } else {
      key = inner;
    }
    // Normalize key names
    if (key === "BackSpace") key = "Backspace";
    if (key === "Page_Up") key = "PageUp";
    if (key === "Page_Down") key = "PageDown";
    if (key === "Return") key = "Enter";
    if (key === "Esc") key = "Escape";
    if (key === "Prior") key = "PageUp";
    if (key === "Next") key = "PageDown";
  } else {
    // Printable key sent directly
    key = input;
  }

  return { key, type };
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
    return lastKeyResult;
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

  prepareTypeDuckFilesystem(currentFs, currentPrepareOptions);
  for (const asset of currentExtraSharedAssets) {
    writeExtraSharedAsset(currentFs, currentPrepareOptions.sharedDataDir, asset);
  }

  const deployed = currentRuntime.deploy();
  await syncCurrentStateToPersistence("deploy");
  return deployed;
}

/**
 * Customize preferences using Yune runtime and sync persistence
 *
 * Replaces upstream Module.ccall("customize", ...)
 *
 * Note: Upstream customize uses pageSize and options bitmap.
 * Yune customize API accepts configId, key, value strings.
 * This adapter maps preferences to Yune customize calls.
 */
export async function customize(preferences: RimePreferences): Promise<boolean> {
  if (currentRuntime === null || currentFs === null || currentSchemaId === null) {
    throw new Error("Yune runtime not initialized");
  }

  // Map preferences to Yune customize calls
  let success = true;

  if (preferences.pageSize !== undefined) {
    const customized = currentRuntime.customize(currentSchemaId, "page_size", String(preferences.pageSize));
    await syncCurrentStateToPersistence("customize");
    success = success && customized;
  }

  // Note: options bitmap handling not implemented yet
  // Requires Yune adapter widening or explicit preference mapping

  return success;
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
  fs: TypeDuckFilesystem,
  sharedDataDir: string,
  asset: TypeDuckExtraSharedAsset,
): void {
  if (
    asset.path.length === 0 ||
    asset.path.startsWith("/") ||
    asset.path.includes("\\") ||
    asset.path.split("/").includes("..")
  ) {
    throw new Error(`Invalid TypeDuck shared asset path: ${asset.path}`);
  }

  const fullPath = joinTypeDuckVirtualPath(sharedDataDir, asset.path);
  ensureVirtualDirectory(fs, fullPath.split("/").slice(0, -1).join("/"));
  fs.writeFile(fullPath, asset.content, { flags: "w" });
}

function ensureVirtualDirectory(fs: TypeDuckFilesystem, path: string): void {
  if (fs.analyzePath(path).exists) {
    return;
  }
  if (fs.mkdirTree !== undefined) {
    fs.mkdirTree(path);
    return;
  }
  if (fs.mkdir === undefined) {
    throw new Error(`TypeDuck filesystem cannot create directory: ${path}`);
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
  fs: TypeDuckFilesystem,
  prepareOptions: PrepareTypeDuckFilesystemOptions,
  reason: PersistenceSyncReason,
): Promise<void> {
  emitPersistenceDiagnostic(fs, prepareOptions, "syncToPersistenceAfterMutation:start", reason);
  await syncToPersistenceAfterMutation(fs);
  emitPersistenceDiagnostic(fs, prepareOptions, "syncToPersistenceAfterMutation:pass", reason);
}

function emitPersistenceDiagnostic(
  fs: TypeDuckFilesystem,
  prepareOptions: PrepareTypeDuckFilesystemOptions,
  phase: PersistenceDiagnosticPhase,
  reason: PersistenceSyncReason,
): void {
  const diagnostic: YunePersistenceDiagnostic = {
    phase,
    reason,
    schemaId: prepareOptions.schemaId,
    userDataDir: prepareOptions.userDataDir,
    timestamp: new Date().toISOString(),
    persistedConfig: snapshotPersistedCustomConfig(fs, prepareOptions),
  };
  console.info(`YUNE_PERSISTENCE ${JSON.stringify(diagnostic)}`);
  const diagnosticGlobal = globalThis as typeof globalThis & {
    onYunePersistenceDiagnostic?: (marker: YunePersistenceDiagnostic) => void;
  };
  diagnosticGlobal.onYunePersistenceDiagnostic?.(diagnostic);
}

function snapshotPersistedCustomConfig(
  fs: TypeDuckFilesystem,
  prepareOptions: PrepareTypeDuckFilesystemOptions,
): PersistedConfigSnapshot {
  const path = joinTypeDuckVirtualPath(prepareOptions.userDataDir, `${prepareOptions.schemaId}.custom.yaml`);
  if (!fs.analyzePath(path).exists) {
    return { path, exists: false };
  }

  try {
    const file = fs.readFile(path, { encoding: "utf8" });
    const text = typeof file === "string" ? file : new TextDecoder().decode(file);
    const pageSize = /^\s*page_size:\s*(\S+)/m.exec(text)?.[1] ?? null;
    return {
      path,
      exists: true,
      pageSize,
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

/**
 * Get current runtime for testing/debugging
 */
export function getCurrentRuntime(): TypeDuckRuntime | null {
  return currentRuntime;
}
