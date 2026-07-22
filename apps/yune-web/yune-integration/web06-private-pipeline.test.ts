import { afterEach, describe, expect, it, vi } from "vitest";

import {
  WEB06_INJECTED_ERROR_CONFIG_ID,
  WEB06_INJECTED_ERROR_KEY,
  WEB06_INJECTED_ERROR_VALUE,
	WEB06_MINIMAL_RECEIPT_CAPACITY,
  web06ActionContract,
  web06AdapterProjectionFingerprint,
  web06AdapterProjectionFingerprintsEqual,
  web06ControlAction,
	web06DeferredControlAction,
  web06DeployCacheSnapshotDigest,
  web06EngineRawAdapterProjection,
  web06EngineRawFingerprint,
  web06InjectedAssetManifestDigest,
  web06PresentationFingerprintDigest,
	web06PresentationStateDigest,
  web06StableDigest,
  web06TerminalContract,
  web06UserdbSnapshotDigest,
} from "../src/yune-integration/private-protocol.js";
import { web06SchemaChangeFanout } from "../src/yune-integration/web06-app-action-map.js";

import type {
  Actions,
  RimeResult,
  Web06ActionIdentity,
	Web06MappedAction,
  Web06PresentationFingerprint,
  Web06WorkerReceipt,
  Web06WorkerResultSummary,
} from "../src/types.js";

const EMPTY_RAW_RESPONSE_JSON = JSON.stringify({
  handled: true,
  commits: [],
  context: null,
  status: null,
});

const COMPOSING_N_RAW_RESPONSE_JSON = JSON.stringify({
  handled: true,
  commits: [],
  context: {
    input: "n",
    preedit: "n",
    caret: 1,
    highlighted: -1,
    page_size: 6,
    page_no: 0,
    is_last_page: true,
    select_keys: null,
    select_labels: [],
    candidates: [],
  },
  status: null,
});

class MemoryStorage implements Storage {
  readonly #values = new Map<string, string>();

  get length() {
    return this.#values.size;
  }

  clear() {
    this.#values.clear();
  }

  getItem(key: string) {
    return this.#values.get(key) ?? null;
  }

  key(index: number) {
    return [...this.#values.keys()][index] ?? null;
  }

  removeItem(key: string) {
    this.#values.delete(key);
  }

  setItem(key: string, value: string) {
    this.#values.set(key, value);
  }
}

type WorkerMessage = Record<string, any>;

class ProtocolWorker {
  static latest: ProtocolWorker;
  readonly sent: WorkerMessage[] = [];
  readonly url: string;
  #messageListener?: (event: { data: WorkerMessage }) => void;

  constructor(url: string) {
    this.url = url;
    ProtocolWorker.latest = this;
  }

  addEventListener(type: string, listener: (event: { data: WorkerMessage }) => void) {
    if (type === "message") this.#messageListener = listener;
  }

  postMessage(message: WorkerMessage) {
    this.sent.push(message);
  }

  emit(data: WorkerMessage) {
    this.#messageListener?.({ data });
  }

  action(index: number) {
    return this.sent.filter(message => message.kind === "action")[index];
  }
}

interface DebugApi {
  protocolVersion: "web06-private-v1";
  mode: "off" | "minimal" | "full";
  modeProvenance:
    | "instrumented-default-minimal"
    | "instrumented-explicit-minimal"
    | "instrumented-explicit-full";
  status(): Record<string, any>;
  snapshot(): Record<string, any>;
  invalidations(): Record<string, any>[];
  resetReceipts(): void;
  prepareLearnedReloadContinuity(measurementId: string, actionId?: string): Record<string, any>;
  bindLearnedReloadWindow(measurementId: string, continuityNonce: string): Record<string, any>;
}

let randomCounter = 1;

async function loadProtocol(
  mode?: "minimal" | "full",
  sessionStorage = new MemoryStorage(),
  options: { autoRaf?: boolean } = {},
) {
  vi.resetModules();
  const dataset: Record<string, string> = {};
  const rafCallbacks: FrameRequestCallback[] = [];
  const localStorage = new MemoryStorage();
  const fakeWindow = {
    localStorage,
    sessionStorage,
    location: { reload: vi.fn() },
  };
  vi.stubGlobal("window", fakeWindow);
  vi.stubGlobal("location", {
    search: mode === undefined ? "" : `?yuneWeb06Mode=${mode}`,
    hostname: "localhost",
  });
  vi.stubGlobal("document", {
    documentElement: { dataset },
    activeElement: null,
    querySelector: () => null,
  });
  vi.stubGlobal("Worker", ProtocolWorker);
  vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
    if (options.autoRaf === false) rafCallbacks.push(callback);
    else callback(performance.now());
    return 1;
  });
  vi.stubGlobal("crypto", {
    randomUUID: () => `00000000-0000-4000-8000-${String(randomCounter++).padStart(12, "0")}`,
    getRandomValues: (bytes: Uint8Array) => {
      bytes.fill(randomCounter++ % 255 || 1);
      return bytes;
    },
  });

  const rime = await import("../src/rime.js");
  const debug = (fakeWindow as typeof fakeWindow & { __YUNE_WEB06__: DebugApi }).__YUNE_WEB06__;
  const flushRaf = () => rafCallbacks.shift()?.(performance.now());
  const flushAllRafs = () => {
    while (rafCallbacks.length > 0) flushRaf();
  };
  return {
    rime,
    debug,
    worker: ProtocolWorker.latest,
    sessionStorage,
    dataset,
    localStorage,
    pendingRafCount: () => rafCallbacks.length,
    flushRaf,
    flushAllRafs,
  };
}

function workerReceipt(summary?: Web06WorkerResultSummary, observerFailures: string[] = []): Web06WorkerReceipt {
  const now = performance.now();
  return {
    workerMessageReceivedAt: now,
    workerActionStartedAt: now,
    workerFinishedAt: now,
    runtimeSpans: [],
    adapterSpans: [],
    persistenceSpans: [],
    collectorSpans: [],
    engineRaw: {
      availability: "not-collected",
      action: "processKey",
      reason: "minimal-content-free",
    },
    observerFailures,
    lifecycleEffects: [],
    resultSummary: summary,
  };
}

function emitSuccess(
  worker: ProtocolWorker,
  actionIndex: number,
  result: unknown,
  summary: Web06WorkerResultSummary,
  lifecycleEffects: Web06WorkerReceipt["lifecycleEffects"] = [],
  engineRawJson = EMPTY_RAW_RESPONSE_JSON,
) {
  const envelope = worker.action(actionIndex);
  const receipt = workerReceipt(summary);
  receipt.lifecycleEffects = lifecycleEffects;
  if (envelope.mode === "minimal") {
    receipt.engineRaw = {
      availability: "not-collected",
      action: envelope.name,
      reason: "minimal-content-free",
    };
  }
  else if (summary.kind === "rime-result") {
    const rawFingerprint = web06EngineRawFingerprint(envelope.name, envelope.name, engineRawJson);
    const rawProjectionDigest = web06StableDigest(web06EngineRawAdapterProjection(rawFingerprint));
    const adapterProjection = web06AdapterProjectionFingerprint(result as RimeResult);
    const adapterProjectionDigest = web06StableDigest(adapterProjection);
    receipt.engineRawJson = engineRawJson;
    receipt.engineRawOperation = envelope.name;
    receipt.engineRaw = {
      availability: "captured",
      action: envelope.name,
      operation: envelope.name,
      jsonDigest: web06StableDigest(engineRawJson),
      rawFingerprintDigest: web06StableDigest(rawFingerprint),
      rawProjectionDigest,
      adapterProjectionDigest,
		projectionMatches: web06AdapterProjectionFingerprintsEqual(
			web06EngineRawAdapterProjection(rawFingerprint),
			adapterProjection,
		),
      adapterProjection,
      rawFingerprint,
    };
  }
  else {
    receipt.engineRaw = {
      availability: "not-applicable",
      action: envelope.name,
      reason: "action-has-no-runtime-response",
    };
  }
  worker.emit({
    protocolVersion: "web06-private-v1",
    kind: "action-result",
    mode: envelope.mode,
    modeProvenance: envelope.modeProvenance,
    identity: structuredClone(envelope.identity),
    resultType: "success",
    result,
    receipt,
  });
}

function emitListener(
  worker: ProtocolWorker,
  actionIndex: number,
  name: string,
  args: unknown[],
  associated = true,
) {
  const envelope = associated ? worker.action(actionIndex) : undefined;
  worker.emit({
    type: "listener",
    name,
    args,
    ...(associated ? {
      web06: {
        protocolVersion: "web06-private-v1",
        actionId: envelope!.identity.actionId,
        sequenceId: envelope!.identity.sequenceId,
      },
    } : {}),
  });
}

function fingerprint(sequenceId: number, input = "", textareaValue = ""): Web06PresentationFingerprint {
  return {
    sequenceId,
    input,
    page: 0,
    isLastPage: true,
    highlightedIndex: -1,
    candidates: [],
    status: null,
    textareaValue,
    selectionStart: textareaValue.length,
    selectionEnd: textareaValue.length,
  };
}

function recordPresentation(
  rime: Awaited<ReturnType<typeof loadProtocol>>["rime"],
  identity: Web06ActionIdentity,
  outcome: "painted" | "committed" | "processed-no-visual-change" | "barrier-completed" | "superseded" | "failure",
  adapterResult: RimeResult,
  before: Web06PresentationFingerprint,
  expected: Web06PresentationFingerprint,
  observed = expected,
  extra: Partial<{
    supersededBySequenceId: number;
    supersessionSequenceLag: number;
    paintObservedAt: number;
  }> = {},
) {
  const observedAt = performance.now();
  const adapterProjection = web06AdapterProjectionFingerprint(adapterResult);
  rime.recordWeb06ResponseMapping(identity, observedAt, observedAt);
  rime.recordWeb06PresentationOutcome({
    identity,
    outcome,
    stateUpdateScheduledAt: observedAt,
    stateCommittedAt: observedAt,
    firstRafAt: observedAt,
    terminalObservedAt: observedAt,
    ...(outcome === "painted" || outcome === "superseded"
      ? { paintObservedAt: extra.paintObservedAt ?? observedAt }
      : {}),
    beforePresentation: before,
    adapterProjection,
    adapterProjectionDigest: web06StableDigest(adapterProjection),
    presentationExpected: expected,
    domObserved: observed,
    presentationDigest: web06PresentationFingerprintDigest(observed),
    ...(extra.supersededBySequenceId === undefined ? {} : {
      supersededBySequenceId: extra.supersededBySequenceId,
    }),
    ...(extra.supersessionSequenceLag === undefined ? {} : {
      supersessionSequenceLag: extra.supersessionSequenceLag,
    }),
  });
}

function ownedAction<K extends keyof Actions>(
  rime: Awaited<ReturnType<typeof loadProtocol>>["rime"],
  name: K,
  args: Parameters<Actions[K]>,
  cause?: Web06ActionIdentity,
): ReturnType<Actions[K]> {
  return rime.withWeb06OwnedAction(
    `pipeline:${name}`,
    name,
    args,
    `pipeline:${name}`,
    cause,
    () => rime.default[name](...args as never),
  ) as ReturnType<Actions[K]>;
}

function mappedProcessAction(
  rime: Awaited<ReturnType<typeof loadProtocol>>["rime"],
  key: string,
  timeStamp: number,
) {
  const action: Web06MappedAction = {
    name: "processKey",
    args: [`{${key}}`],
    actionClass: "native-key",
    supersedable: true,
    boundary: "none",
  };
  const event = rime.recordWeb06DomEvent({
    type: "keydown",
    code: `Key${key.toUpperCase()}`,
    key,
    timeStamp,
    repeat: false,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
  }, {
    classification: "mapped-action(s)",
    reason: "test-printable-key",
    preventDefault: true,
    actions: [action],
  }, performance.now());
  const context = rime.web06MappedActionContext(event, action, 0, key);
  const promise = rime.withWeb06ActionContext(context, () => rime.default.processKey(`{${key}}`));
  return { action, event, context, promise, identity: rime.web06ActionIdentityFor(promise)! };
}

function composingResult(input: string): RimeResult {
  return {
    isComposing: true,
    success: true,
    inputBuffer: { before: "", active: input, after: "" },
    page: 0,
    isLastPage: true,
    highlightedIndex: -1,
    candidates: [],
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.resetModules();
});

describe("WEB-06 private main/worker pipeline", () => {
  it("ships the selector-omitted instrumentation lane as provenance-bound minimal", async () => {
    const { debug, worker } = await loadProtocol();
    expect(debug.mode).toBe("minimal");
    expect(debug.modeProvenance).toBe("instrumented-default-minimal");
    expect(worker.url).not.toContain("yuneWeb06Mode");
    expect(debug.snapshot().header).toMatchObject({
      mode: "minimal",
      modeProvenance: "instrumented-default-minimal",
    });

    const explicit = await loadProtocol("full");
    expect(explicit.worker.url).toContain("yuneWeb06Mode=full");
    expect(explicit.debug.modeProvenance).toBe("instrumented-explicit-full");
  });

  it("preserves real FIFO dispatch and emits non-render barrier/error lifecycle terminals", async () => {
    const { rime, debug, worker, flushAllRafs, pendingRafCount } = await loadProtocol(
      "full",
      new MemoryStorage(),
      { autoRaf: false },
    );
    const first = ownedAction(rime, "setOption", ["ascii_mode", true]);
    const firstIdentity = rime.web06ActionIdentityFor(first)!;
    const second = ownedAction(rime, "deploy", []);

    expect(worker.sent.filter(message => message.kind === "action")).toHaveLength(1);
    expect(debug.status()).toMatchObject({
      queueDepth: 2,
      runningActionId: firstIdentity.actionId,
      pendingTerminalActions: 0,
    });

    emitListener(worker, 0, "optionChanged", ["ascii_mode", true]);
    emitSuccess(worker, 0, undefined, {
      kind: "empty",
      resultDigest: "set-option",
      success: true,
      persistenceCompleted: false,
    }, [
		{ kind: "listener", name: "optionChanged", argsDigest: web06StableDigest(["ascii_mode", true]), args: ["ascii_mode", true], recordedAt: performance.now() },
      { kind: "engine-state", name: "setOption", resultDigest: "set-option", recordedAt: performance.now() },
    ]);
    expect(debug.status()).toMatchObject({ queueDepth: 1, pendingTerminalActions: 1 });
    expect(pendingRafCount()).toBe(1);
    flushAllRafs();
    expect(debug.status()).toMatchObject({ queueDepth: 1, pendingTerminalActions: 0 });
    await expect(first).resolves.toBeUndefined();
    expect(worker.sent.filter(message => message.kind === "action")).toHaveLength(2);

    emitListener(worker, 1, "deployStatusChanged", ["success"]);
    emitSuccess(worker, 1, true, {
      kind: "boolean",
      resultDigest: "deploy-true",
      success: true,
      persistenceCompleted: true,
    }, [
		{ kind: "listener", name: "deployStatusChanged", argsDigest: web06StableDigest(["success"]), args: ["success"], recordedAt: performance.now() },
      { kind: "engine-persistence", name: "deploy", resultDigest: "deploy-true", recordedAt: performance.now() },
    ]);
    expect(debug.status()).toMatchObject({ queueDepth: 0, pendingTerminalActions: 1 });
    flushAllRafs();
    expect(debug.status()).toMatchObject({ queueDepth: 0, pendingTerminalActions: 0 });
    await expect(second).resolves.toBe(true);

    const failed = ownedAction(rime, "processKey", ["{a}"]);
    const failedEnvelope = worker.action(2);
    worker.emit({
      protocolVersion: "web06-private-v1",
      kind: "action-result",
      mode: failedEnvelope.mode,
      modeProvenance: failedEnvelope.modeProvenance,
      identity: structuredClone(failedEnvelope.identity),
      resultType: "error",
      error: new Error("forced pipeline error"),
      receipt: workerReceipt(undefined, ["forced worker observer failure"]),
    });
    await expect(failed).rejects.toThrow("forced pipeline error");

    const snapshot = debug.snapshot();
    expect(snapshot.actions.map((action: any) => [action.name, action.lifecycle?.outcome])).toEqual([
      ["setOption", "barrier-completed"],
      ["deploy", "barrier-completed"],
      ["processKey", "failure"],
    ]);
    expect(snapshot.actions[0].lifecycle).toMatchObject({ effect: "listener", listenerEffectCount: 1 });
    expect(snapshot.actions[1].lifecycle).toMatchObject({ effect: "listener", persistenceCompleted: true });
    expect(snapshot.status).toMatchObject({
      valid: false,
      workerObserverFailureCount: 1,
      workerObserverFailuresAudited: true,
      queueDepth: 0,
    });
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("WORKER_OBSERVER_FAILURE");
  });

  it("binds presentations to exact DOM sequence tokens and keeps minimal receipts content-free", async () => {
    const { rime, debug, worker } = await loadProtocol("minimal");
    const first = ownedAction(rime, "processKey", ["{n}"]);
    const firstIdentity = rime.web06ActionIdentityFor(first)!;
    emitSuccess(worker, 0, { isComposing: true, success: true }, {
      kind: "rime-result",
      resultDigest: "n",
      success: true,
      persistenceCompleted: false,
    });
    await first;
    recordPresentation(
      rime,
      firstIdentity,
      "painted",
      { isComposing: true, success: true, inputBuffer: { before: "n", active: "", after: "" }, page: 0, isLastPage: true, highlightedIndex: -1, candidates: [] },
      fingerprint(firstIdentity.sequenceId),
      fingerprint(firstIdentity.sequenceId, "n"),
    );

    let snapshot = debug.snapshot();
    expect(snapshot.actions[0].presentation).toMatchObject({
      presentationExpectedDigest: expect.any(String),
      domObservedDigest: expect.any(String),
    });
    expect(snapshot.actions[0].presentation).not.toHaveProperty("presentationExpected");
    expect(snapshot.actions[0].presentation).not.toHaveProperty("domObserved");
    expect(snapshot.actions[0].worker).toMatchObject({
      runtimeSpans: [],
      adapterSpans: [],
      persistenceSpans: [],
      collectorSpans: [],
    });

    const stale = ownedAction(rime, "processKey", ["{i}"]);
    const staleIdentity = rime.web06ActionIdentityFor(stale)!;
    emitSuccess(worker, 1, { isComposing: true, success: true }, {
      kind: "rime-result",
      resultDigest: "ni",
      success: true,
      persistenceCompleted: false,
    });
    await stale;
    recordPresentation(
      rime,
      staleIdentity,
      "painted",
      { isComposing: true, success: true, inputBuffer: { before: "ni", active: "", after: "" }, page: 0, isLastPage: true, highlightedIndex: -1, candidates: [] },
      fingerprint(staleIdentity.sequenceId, "n"),
      fingerprint(staleIdentity.sequenceId, "ni"),
      fingerprint(firstIdentity.sequenceId, "ni"),
    );
    snapshot = debug.snapshot();
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("PRESENTATION_SEQUENCE_TOKEN_MISMATCH");
    expect(snapshot.mainObserverCallbacksMs.length).toBeGreaterThanOrEqual(6);
    expect(snapshot.mainObserverCallbacksMs.every((duration: number) => Number.isFinite(duration) && duration >= 0)).toBe(true);
		expect(snapshot.mainObserverCallbacks).toHaveLength(snapshot.mainObserverCallbacksMs.length);
		expect(snapshot.mainObserverCallbacks.every((callback: any, index: number) =>
			callback.callbackId === `web06-main-observer-${String(callback.sequenceId).padStart(8, "0")}`
			&& callback.operation.length > 0
			&& callback.finishedAt - callback.startedAt === callback.durationMs
			&& callback.durationMs === snapshot.mainObserverCallbacksMs[index]
		)).toBe(true);
  });

  it("closes every keyof Actions receipt with exactly one contract-owned terminal", async () => {
    const { rime, debug, worker } = await loadProtocol("full");
    const userdbSnapshot = {
      schemaId: "jyut6ping3",
      dictionaryId: "jyut6ping3",
      path: "/rime/jyut6ping3.userdb",
      exists: true,
      bytes: 0,
      updatedAt: null,
      rows: [],
      rawText: "",
      parseErrors: [],
    };
    const cacheSnapshot = {
      schemaId: "jyut6ping3",
      dictionaryId: "jyut6ping3",
      cacheFresh: true,
      deployedSchemaExists: true,
      actualStamp: null,
      expectedStamp: {},
    };
    const emptyResult: RimeResult = { isComposing: false, success: true };
    const composingResult: RimeResult = {
      isComposing: true,
      success: true,
      inputBuffer: { before: "n", active: "", after: "" },
      page: 0,
      isLastPage: true,
      highlightedIndex: -1,
      candidates: [],
    };
    const assetsManifest = { schemaId: "jyut6ping3" as const, assets: [] };
    const cases: Array<{
      name: keyof Actions;
      args: unknown[];
      result: unknown;
      summary: Web06WorkerResultSummary;
      listener?: { name: string; args: unknown[] };
      semanticEffect?: "engine-state" | "engine-persistence" | "cache-invalidation" | "snapshot-read";
      rawJson?: string;
      presentation?: {
        outcome: "painted" | "processed-no-visual-change" | "barrier-completed";
        before: Web06PresentationFingerprint;
        expected: Web06PresentationFingerprint;
      };
      ownerDigest?: string;
    }> = [
      { name: "setOption", args: ["ascii_mode", true], result: undefined, summary: { kind: "empty", resultDigest: "set-option", success: true, persistenceCompleted: false }, listener: { name: "optionChanged", args: ["ascii_mode", true] } },
      { name: "selectSchema", args: ["jyut6ping3"], result: true, summary: { kind: "boolean", resultDigest: "select-schema", success: true, persistenceCompleted: false }, listener: { name: "schemaChanged", args: ["jyut6ping3", "Jyutping"] } },
      { name: "getUserdbSnapshot", args: [], result: userdbSnapshot, summary: { kind: "userdb-snapshot", resultDigest: "userdb-read", success: true, persistenceCompleted: false, userdbDigest: "userdb-read", userdbRowCount: 0, userdbBytes: 0 }, ownerDigest: web06UserdbSnapshotDigest(userdbSnapshot) },
      { name: "importUserdb", args: ["# userdb"], result: userdbSnapshot, summary: { kind: "userdb-snapshot", resultDigest: "userdb-import", success: true, persistenceCompleted: true, userdbDigest: "userdb-import", userdbRowCount: 0, userdbBytes: 0 }, ownerDigest: web06UserdbSnapshotDigest(userdbSnapshot) },
      { name: "processKey", args: ["{n}"], result: composingResult, summary: { kind: "rime-result", resultDigest: "process", success: true, persistenceCompleted: false }, rawJson: COMPOSING_N_RAW_RESPONSE_JSON, presentation: { outcome: "painted", before: fingerprint(0), expected: fingerprint(0, "n") } },
      { name: "stageAi", args: [], result: emptyResult, summary: { kind: "rime-result", resultDigest: "ai", success: true, persistenceCompleted: false }, presentation: { outcome: "processed-no-visual-change", before: fingerprint(0), expected: fingerprint(0) } },
      { name: "selectCandidate", args: [0], result: emptyResult, summary: { kind: "rime-result", resultDigest: "select", success: true, persistenceCompleted: false }, presentation: { outcome: "barrier-completed", before: fingerprint(0), expected: fingerprint(0) } },
      { name: "deleteCandidate", args: [0], result: emptyResult, summary: { kind: "rime-result", resultDigest: "delete", success: true, persistenceCompleted: false }, presentation: { outcome: "barrier-completed", before: fingerprint(0), expected: fingerprint(0) } },
      { name: "flipPage", args: [false], result: emptyResult, summary: { kind: "rime-result", resultDigest: "page", success: true, persistenceCompleted: false }, presentation: { outcome: "barrier-completed", before: fingerprint(0), expected: fingerprint(0) } },
      { name: "customize", args: [{ enableLearning: true }], result: true, summary: { kind: "boolean", resultDigest: "customize", success: true, persistenceCompleted: true }, semanticEffect: "engine-persistence" },
      { name: "customizeValue", args: ["jyut6ping3.schema", "translator/enable_completion", "true"], result: true, summary: { kind: "boolean", resultDigest: "customize-value", success: true, persistenceCompleted: true } },
      { name: "deploy", args: [], result: true, summary: { kind: "boolean", resultDigest: "deploy", success: true, persistenceCompleted: true }, listener: { name: "deployStatusChanged", args: ["success"] } },
      { name: "deployCacheSnapshot", args: [], result: cacheSnapshot, summary: { kind: "deploy-cache-snapshot", resultDigest: "cache-read", success: true, persistenceCompleted: false }, ownerDigest: web06DeployCacheSnapshotDigest(cacheSnapshot) },
      { name: "invalidateDeployCache", args: [], result: cacheSnapshot, summary: { kind: "deploy-cache-snapshot", resultDigest: "cache-invalidated", success: true, persistenceCompleted: true }, ownerDigest: web06DeployCacheSnapshotDigest(cacheSnapshot) },
      { name: "injectedAssetsManifest", args: [], result: assetsManifest, summary: { kind: "asset-manifest", resultDigest: "assets-read", success: true, persistenceCompleted: false }, ownerDigest: web06InjectedAssetManifestDigest(assetsManifest) },
    ];

    for (const [index, testCase] of cases.entries()) {
      const promise = ownedAction(rime, testCase.name, testCase.args as never);
      const identity = rime.web06ActionIdentityFor(promise)!;
      const contract = web06TerminalContract(testCase.name);
      const effects: Web06WorkerReceipt["lifecycleEffects"] = [];
      if (testCase.listener !== undefined) {
        emitListener(worker, index, testCase.listener.name, testCase.listener.args);
        effects.push({
          kind: "listener",
          name: testCase.listener.name as any,
          argsDigest: web06StableDigest(testCase.listener.args),
		  args: testCase.listener.args,
          recordedAt: performance.now(),
        });
      }
      const semanticEffect = testCase.semanticEffect ?? contract.workerEffect;
      if (semanticEffect !== undefined) {
        effects.push({
          kind: semanticEffect,
          name: testCase.name,
          resultDigest: testCase.summary.resultDigest,
          recordedAt: performance.now(),
        });
      }
      emitSuccess(
        worker,
        index,
        testCase.result,
        testCase.summary,
        effects,
        testCase.rawJson,
      );
      await promise;

      if (contract.strategy === "presentation") {
        const presentation = testCase.presentation!;
        recordPresentation(
          rime,
          identity,
          presentation.outcome,
          testCase.result as RimeResult,
          { ...presentation.before, sequenceId: identity.sequenceId },
          { ...presentation.expected, sequenceId: identity.sequenceId },
        );
      }
      else if (contract.strategy === "owner-effect") {
        rime.recordWeb06OwnedResultEffect(promise, contract.ownerEffect!, {
			expectedState: { digest: testCase.ownerDigest! },
			readObservedState: () => ({ digest: testCase.ownerDigest! }),
        });
      }
    }

    const snapshot = debug.snapshot();
    expect(snapshot.actions).toHaveLength(cases.length);
    expect(snapshot.actions.every((action: any) =>
      Number(action.presentation !== undefined) + Number(action.lifecycle !== undefined) === 1
    )).toBe(true);
    expect(snapshot.actions.map((action: any) => action.name)).toEqual(cases.map(testCase => testCase.name));
    expect(snapshot.invalidations.map((entry: any) => entry.code)).not.toContain("MISSING_TERMINAL_OUTCOME");
    expect(snapshot.invalidations, JSON.stringify(snapshot.invalidations, null, 2)).toEqual([]);
    expect(snapshot.status).toMatchObject({ valid: true, pendingTerminalActions: 0 });
  });

  it("keeps the frozen injected failure contract-green and recovers later FIFO work exactly once", async () => {
    const { rime, debug, worker, flushAllRafs } = await loadProtocol(
      "full",
      new MemoryStorage(),
      { autoRaf: false },
    );
    const injected = ownedAction(rime, "customizeValue", [
      WEB06_INJECTED_ERROR_CONFIG_ID,
      WEB06_INJECTED_ERROR_KEY,
      WEB06_INJECTED_ERROR_VALUE,
    ]);
    const injectedIdentity = rime.web06ActionIdentityFor(injected)!;
    const recovered = ownedAction(rime, "setOption", ["ascii_mode", false]);
    const recoveredIdentity = rime.web06ActionIdentityFor(recovered)!;
    expect(debug.status()).toMatchObject({
      queueDepth: 2,
      runningActionId: injectedIdentity.actionId,
    });

    const failedEnvelope = worker.action(0);
    const failedReceipt = workerReceipt();
    failedReceipt.engineRaw = {
      availability: "not-applicable",
      action: "customizeValue",
      reason: "action-failed-before-runtime-response",
    };
    worker.emit({
      protocolVersion: "web06-private-v1",
      kind: "action-result",
      mode: failedEnvelope.mode,
      modeProvenance: failedEnvelope.modeProvenance,
      identity: structuredClone(failedEnvelope.identity),
      resultType: "error",
      error: new Error("Yune customizeValue requires a config ID and key"),
      receipt: failedReceipt,
    });
    await expect(injected).rejects.toThrow("Yune customizeValue requires a config ID and key");
    expect(debug.status()).toMatchObject({
      queueDepth: 1,
      runningActionId: recoveredIdentity.actionId,
      pendingTerminalActions: 0,
    });

    emitListener(worker, 1, "optionChanged", ["ascii_mode", false]);
    emitSuccess(worker, 1, undefined, {
      kind: "empty",
      resultDigest: web06StableDigest(null),
      success: true,
      persistenceCompleted: false,
    }, [
		{ kind: "listener", name: "optionChanged", argsDigest: web06StableDigest(["ascii_mode", false]), args: ["ascii_mode", false], recordedAt: performance.now() },
      { kind: "engine-state", name: "setOption", resultDigest: web06StableDigest(null), recordedAt: performance.now() },
    ]);
    flushAllRafs();
    await expect(recovered).resolves.toBeUndefined();

    const snapshot = debug.snapshot();
    expect(snapshot.status).toMatchObject({ valid: true, queueDepth: 0, pendingTerminalActions: 0 });
    expect(snapshot.actions.map((action: any) => ({
      sequenceId: action.identity.sequenceId,
      name: action.name,
      outcome: action.lifecycle?.outcome,
    }))).toEqual([
      { sequenceId: 1, name: "customizeValue", outcome: "failure" },
      { sequenceId: 2, name: "setOption", outcome: "barrier-completed" },
    ]);
    expect(snapshot.actions[0].lifecycle).toMatchObject({
      effect: "error",
      effectDigest: expect.stringMatching(/^[0-9a-f]{32}$/),
    });
    expect(snapshot.invalidations.map((entry: any) => entry.code)).not.toContain("UNEXPECTED_ACTION_FAILURE");
  });

  it("keeps the genuine customizeValue validation error identical in default, minimal, and full modes", async () => {
    const observations: unknown[] = [];
    for (const mode of [undefined, "minimal", "full"] as const) {
      const realm = await loadProtocol(mode);
      const action = ownedAction(realm.rime, "customizeValue", [
        WEB06_INJECTED_ERROR_CONFIG_ID,
        WEB06_INJECTED_ERROR_KEY,
        WEB06_INJECTED_ERROR_VALUE,
      ]);
      const envelope = realm.worker.action(0);
      const receipt = workerReceipt();
      receipt.engineRaw = envelope.mode === "full"
        ? {
            availability: "not-applicable",
            action: "customizeValue",
            reason: "action-failed-before-runtime-response",
          }
        : {
            availability: "not-collected",
            action: "customizeValue",
            reason: "minimal-content-free",
          };
      realm.worker.emit({
        protocolVersion: "web06-private-v1",
        kind: "action-result",
        mode: envelope.mode,
        modeProvenance: envelope.modeProvenance,
        identity: structuredClone(envelope.identity),
        resultType: "error",
        error: new Error("Yune customizeValue requires a config ID and key"),
        receipt,
      });
      await expect(action).rejects.toThrow("Yune customizeValue requires a config ID and key");
      const snapshot = realm.debug.snapshot();
      expect(snapshot.invalidations).toEqual([]);
      observations.push({
        publicArgs: envelope.args,
        publicError: "Yune customizeValue requires a config ID and key",
        actionName: snapshot.actions[0].name,
        outcome: snapshot.actions[0].lifecycle.outcome,
        queueDepth: snapshot.status.queueDepth,
        valid: snapshot.status.valid,
      });
    }
    expect(observations).toEqual(new Array(3).fill({
      publicArgs: ["", "", WEB06_INJECTED_ERROR_VALUE],
      publicError: "Yune customizeValue requires a config ID and key",
      actionName: "customizeValue",
      outcome: "failure",
      queueDepth: 0,
      valid: true,
    }));
  });

  it("rejects learned reload while a post-persistence two-rAF terminal is pending", async () => {
    const { rime, debug, worker, flushRaf, pendingRafCount } = await loadProtocol(
      "full",
      new MemoryStorage(),
      { autoRaf: false },
    );
    const customize = ownedAction(rime, "customizeValue", [
      "jyut6ping3.schema",
      "translator/enable_completion",
      "true",
    ]);
    emitSuccess(worker, 0, true, {
      kind: "boolean",
      resultDigest: web06StableDigest(true),
      success: true,
      persistenceCompleted: true,
    }, [{
      kind: "engine-persistence",
      name: "customizeValue",
      resultDigest: web06StableDigest(true),
      recordedAt: performance.now(),
    }]);
    await expect(customize).resolves.toBe(true);
    expect(debug.status()).toMatchObject({ queueDepth: 0, pendingTerminalActions: 1, valid: false });
    expect(pendingRafCount()).toBe(1);
    expect(() => debug.prepareLearnedReloadContinuity("pending-lifecycle-attempt")).toThrow(
      /RELOAD_CONTINUITY_NOT_IDLE/,
    );
    flushRaf();
    expect(debug.status()).toMatchObject({ pendingTerminalActions: 1 });
    flushRaf();
    expect(debug.status()).toMatchObject({ pendingTerminalActions: 0, valid: false });
    expect(debug.invalidations().map((entry: any) => entry.code)).toContain("RELOAD_CONTINUITY_NOT_IDLE");
  });

  it("requires direct presentation equality and action-specific no-visual legality", async () => {
    const printable = await loadProtocol("minimal");
    const printableResult: RimeResult = { isComposing: false, success: true };
    const printablePromise = ownedAction(printable.rime, "processKey", ["{a}"]);
    const printableIdentity = printable.rime.web06ActionIdentityFor(printablePromise)!;
    emitSuccess(printable.worker, 0, printableResult, {
      kind: "rime-result",
      resultDigest: "printable",
      success: true,
      persistenceCompleted: false,
    });
    await printablePromise;
    recordPresentation(
      printable.rime,
      printableIdentity,
      "processed-no-visual-change",
      printableResult,
      fingerprint(printableIdentity.sequenceId),
      fingerprint(printableIdentity.sequenceId),
    );
    let snapshot = printable.debug.snapshot();
    expect(snapshot.actions[0].presentation.outcome).toBe("failure");
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("ILLEGAL_PRESENTATION_OUTCOME");

    for (const [name, args] of [
      ["selectCandidate", [0]],
      ["deleteCandidate", [0]],
      ["flipPage", [false]],
    ] as const) {
      const realm = await loadProtocol("full");
      const result: RimeResult = { isComposing: false, success: true };
      const promise = ownedAction(realm.rime, name, args as never);
      const identity = realm.rime.web06ActionIdentityFor(promise)!;
      emitSuccess(realm.worker, 0, result, {
        kind: "rime-result",
        resultDigest: `${name}-result`,
        success: true,
        persistenceCompleted: false,
      });
      await promise;
      recordPresentation(
        realm.rime,
        identity,
        "processed-no-visual-change",
        result,
        fingerprint(identity.sequenceId),
        fingerprint(identity.sequenceId),
      );
      snapshot = realm.debug.snapshot();
      expect(snapshot.actions[0].presentation.outcome, name).toBe("failure");
      expect(snapshot.invalidations.map((entry: any) => entry.code), name).toContain(
        "ILLEGAL_PRESENTATION_OUTCOME",
      );
    }
  });

  it("chains queued before-state and accepts supersession only through an exact painted covering tuple", async () => {
    const realm = await loadProtocol("minimal");
    const first = mappedProcessAction(realm.rime, "a", 1);
    const second = mappedProcessAction(realm.rime, "b", 2);
    emitSuccess(realm.worker, 0, composingResult("a"), {
      kind: "rime-result",
      resultDigest: "a",
      success: true,
      persistenceCompleted: false,
    });
    await first.promise;
    emitSuccess(realm.worker, 1, composingResult("ab"), {
      kind: "rime-result",
      resultDigest: "ab",
      success: true,
      persistenceCompleted: false,
    });
    await second.promise;

    const firstExpected = fingerprint(first.identity.sequenceId, "a");
    const coveringBefore = fingerprint(first.identity.sequenceId, "a");
    const covering = fingerprint(second.identity.sequenceId, "ab");
    recordPresentation(
      realm.rime,
      second.identity,
      "painted",
      composingResult("ab"),
      coveringBefore,
      covering,
    );
    recordPresentation(
      realm.rime,
      first.identity,
      "superseded",
      composingResult("a"),
      fingerprint(0),
      firstExpected,
      covering,
      { supersededBySequenceId: second.identity.sequenceId, supersessionSequenceLag: 1 },
    );
    let snapshot = realm.debug.snapshot();
    expect(snapshot.invalidations).toEqual([]);
    expect(snapshot.actions[0].presentation.outcome).toBe("superseded");
    expect(snapshot.actions[1].presentation).toMatchObject({
      outcome: "painted",
      beforePresentationDigest: web06PresentationStateDigest(coveringBefore),
    });

    const unchanged = mappedProcessAction(realm.rime, "c", 3);
    emitSuccess(realm.worker, 2, composingResult("ab"), {
      kind: "rime-result",
      resultDigest: "unchanged-ab",
      success: true,
      persistenceCompleted: false,
    });
    await unchanged.promise;
    const unchangedExpected = fingerprint(unchanged.identity.sequenceId, "ab");
    recordPresentation(
      realm.rime,
      unchanged.identity,
      "painted",
      composingResult("ab"),
      covering,
      unchangedExpected,
    );
    snapshot = realm.debug.snapshot();
    expect(snapshot.actions[2].presentation.outcome).toBe("failure");
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("ILLEGAL_PRESENTATION_OUTCOME");
  });

  it("rejects supersession when the later candidate/status tuple is not exact", async () => {
    const realm = await loadProtocol("minimal");
    const first = mappedProcessAction(realm.rime, "a", 1);
    const second = mappedProcessAction(realm.rime, "b", 2);
    emitSuccess(realm.worker, 0, composingResult("a"), {
      kind: "rime-result",
      resultDigest: "a",
      success: true,
      persistenceCompleted: false,
    });
    await first.promise;
    emitSuccess(realm.worker, 1, composingResult("ab"), {
      kind: "rime-result",
      resultDigest: "ab",
      success: true,
      persistenceCompleted: false,
    });
    await second.promise;
    const expected = {
      ...fingerprint(second.identity.sequenceId, "ab"),
      candidates: [{ label: "1.", text: "甲", comment: "", source: "table" }],
      status: { schema_id: "jyut6ping3" },
    };
    const mutated = {
      ...expected,
      candidates: [{ ...expected.candidates[0]!, text: "乙" }],
      status: { schema_id: "luna_pinyin" },
    };
    recordPresentation(
      realm.rime,
      second.identity,
      "painted",
      composingResult("ab"),
      fingerprint(first.identity.sequenceId, "a"),
      expected,
      mutated,
    );
    recordPresentation(
      realm.rime,
      first.identity,
      "superseded",
      composingResult("a"),
      fingerprint(0),
      fingerprint(first.identity.sequenceId, "a"),
      mutated,
      { supersededBySequenceId: second.identity.sequenceId, supersessionSequenceLag: 1 },
    );
    const snapshot = realm.debug.snapshot();
    expect(snapshot.actions.map((action: any) => action.presentation.outcome)).toEqual(["failure", "failure"]);
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("ILLEGAL_PRESENTATION_OUTCOME");
  });

  it("accepts a finite exact paging paint and rejects a missing paint observation", async () => {
    const pagingRaw = JSON.stringify({
      handled: true,
      commits: [],
      context: {
        input: "n",
        preedit: "n",
        caret: 1,
        highlighted: 0,
        page_size: 6,
        page_no: 1,
        is_last_page: true,
        select_keys: null,
        select_labels: [],
        candidates: [],
      },
      status: null,
    });
    const result: RimeResult = {
      isComposing: true,
      success: true,
      inputBuffer: { before: "n", active: "", after: "" },
      page: 1,
      isLastPage: true,
      highlightedIndex: 0,
      candidates: [],
    };
    const green = await loadProtocol("full");
    const page = ownedAction(green.rime, "flipPage", [false]);
    const pageIdentity = green.rime.web06ActionIdentityFor(page)!;
    emitSuccess(green.worker, 0, result, {
      kind: "rime-result",
      resultDigest: "page-one",
      success: true,
      persistenceCompleted: false,
    }, [], pagingRaw);
    await page;
    recordPresentation(
      green.rime,
      pageIdentity,
      "painted",
      result,
      fingerprint(pageIdentity.sequenceId, "n"),
      { ...fingerprint(pageIdentity.sequenceId, "n"), page: 1, highlightedIndex: 0 },
    );
    let snapshot = green.debug.snapshot();
    expect(snapshot.status.valid).toBe(true);
    expect(snapshot.actions[0].presentation).toMatchObject({
      outcome: "painted",
      paintObservedAt: expect.any(Number),
    });

    const red = await loadProtocol("full");
    const missingPaint = ownedAction(red.rime, "flipPage", [false]);
    const missingIdentity = red.rime.web06ActionIdentityFor(missingPaint)!;
    emitSuccess(red.worker, 0, result, {
      kind: "rime-result",
      resultDigest: "page-one",
      success: true,
      persistenceCompleted: false,
    }, [], pagingRaw);
    await missingPaint;
    const observedAt = performance.now();
    const adapterProjection = web06AdapterProjectionFingerprint(result);
    const before = fingerprint(missingIdentity.sequenceId, "n");
    const expected = { ...before, page: 1, highlightedIndex: 0 };
    red.rime.recordWeb06ResponseMapping(missingIdentity, observedAt, observedAt);
    red.rime.recordWeb06PresentationOutcome({
      identity: missingIdentity,
      outcome: "painted",
      stateUpdateScheduledAt: observedAt,
      stateCommittedAt: observedAt,
      firstRafAt: observedAt,
      terminalObservedAt: observedAt,
      beforePresentation: before,
      adapterProjection,
      adapterProjectionDigest: web06StableDigest(adapterProjection),
      presentationExpected: expected,
      domObserved: expected,
      presentationDigest: web06PresentationFingerprintDigest(expected),
    });
    snapshot = red.debug.snapshot();
    expect(snapshot.actions[0].presentation.outcome).toBe("failure");
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("ILLEGAL_PRESENTATION_OUTCOME");
  });

  it("fails closed on owner readback mismatch and ignores unassociated background listeners", async () => {
    const background = await loadProtocol("full");
    emitListener(background.worker, 0, "initialized", [true], false);
    expect(background.dataset["yuneInitialized"]).toBe("true");
    expect(background.debug.snapshot().status.valid).toBe(true);

    const realm = await loadProtocol("full", new MemoryStorage(), { autoRaf: false });
    const snapshotResult = {
      schemaId: "jyut6ping3" as const,
      dictionaryId: "jyut6ping3",
      path: "/rime/jyut6ping3.userdb",
      exists: true,
      bytes: 0,
      updatedAt: null,
      rows: [],
      rawText: "",
      parseErrors: [],
    };
    const digest = web06UserdbSnapshotDigest(snapshotResult);
    const action = ownedAction(realm.rime, "getUserdbSnapshot", []);
    emitSuccess(realm.worker, 0, snapshotResult, {
      kind: "userdb-snapshot",
      resultDigest: digest,
      success: true,
      persistenceCompleted: false,
      userdbDigest: digest,
      userdbRowCount: 0,
      userdbBytes: 0,
    }, [{ kind: "snapshot-read", name: "getUserdbSnapshot", resultDigest: digest, recordedAt: performance.now() }]);
    await action;
    realm.rime.recordWeb06OwnedResultEffect(action, "ui-userdb-refresh", {
		expectedState: { digest },
		readObservedState: () => ({ digest: web06StableDigest("stale-view") }),
    });
    realm.flushAllRafs();
    const snapshot = realm.debug.snapshot();
    expect(snapshot.actions[0].lifecycle.outcome).toBe("failure");
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain(
      "LIFECYCLE_MAIN_EFFECT_MISMATCH",
    );
  });

  it("reports pending fanout work until its exact owner consumes and completes it", async () => {
    const { rime, debug, worker } = await loadProtocol("full");
    rime.withWeb06ControlEvent({ type: "change", timeStamp: 1 }, () => {
      rime.declareWeb06ControlFanout("pending-fanout", [{
        owner: "fanout-owner",
        action: web06ControlAction("setOption", ["ascii_mode", true]),
      }]);
    });
    expect(debug.status()).toMatchObject({ pendingFanoutActions: 1, valid: false });
    const action = rime.withWeb06OwnedAction(
      "fanout-owner",
      "setOption",
      ["ascii_mode", true],
      "pending-fanout",
      undefined,
      () => rime.default.setOption("ascii_mode", true),
    );
    expect(debug.status()).toMatchObject({ pendingFanoutActions: 0, queueDepth: 1 });
    emitListener(worker, 0, "optionChanged", ["ascii_mode", true]);
    emitSuccess(worker, 0, undefined, {
      kind: "empty",
      resultDigest: web06StableDigest(null),
      success: true,
      persistenceCompleted: false,
    }, [
		{ kind: "listener", name: "optionChanged", argsDigest: web06StableDigest(["ascii_mode", true]), args: ["ascii_mode", true], recordedAt: performance.now() },
      { kind: "engine-state", name: "setOption", resultDigest: web06StableDigest(null), recordedAt: performance.now() },
    ]);
    await action;
    expect(debug.snapshot().status).toMatchObject({
      valid: true,
      pendingFanoutActions: 0,
      pendingTerminalActions: 0,
      queueDepth: 0,
    });
  });

  it("maps one page-button click to one action, one result application, and one terminal", async () => {
    const { rime, debug, worker } = await loadProtocol("minimal");
    let promise!: ReturnType<Actions["flipPage"]>;
    rime.withWeb06ControlEvent({ type: "click", timeStamp: 1 }, () => {
      rime.declareWeb06ControlFanout("candidate-page-button", [{
        owner: "candidate-control",
        action: web06ControlAction("flipPage", [false]),
      }]);
      promise = rime.withWeb06OwnedAction(
        "candidate-control",
        "flipPage",
        [false],
        "candidate-page-button",
        undefined,
        () => rime.default.flipPage(false),
      );
    });
    expect(worker.sent.filter(message => message.kind === "action")).toHaveLength(1);
    const result: RimeResult = { isComposing: false, success: true };
    emitSuccess(worker, 0, result, {
      kind: "rime-result",
      resultDigest: "page-once",
      success: true,
      persistenceCompleted: false,
    });
    await promise;
    const identity = rime.web06ActionIdentityFor(promise)!;
    recordPresentation(
      rime,
      identity,
      "barrier-completed",
      result,
      fingerprint(0),
      fingerprint(identity.sequenceId),
    );
    const snapshot = debug.snapshot();
    expect(snapshot.events).toHaveLength(1);
    expect(snapshot.actions).toHaveLength(1);
    expect(snapshot.actions[0]).toMatchObject({
      name: "flipPage",
      resultType: "success",
      presentation: { outcome: "barrier-completed" },
    });
    expect(snapshot.invalidations).toEqual([]);
  });

  it("cancels an unconsumed long-press fanout without a phantom boundary and advances a consumed one exactly once", async () => {
    const cancelled = await loadProtocol("minimal");
	let cancelledEvent: any;
    cancelled.rime.withWeb06ControlEvent({ type: "mousedown", timeStamp: 1 }, () => {
      cancelledEvent = cancelled.rime.declareWeb06ControlFanout("candidate-long-press", [{
        owner: "candidate-delete",
        action: web06ControlAction("deleteCandidate", [0]),
      }]);
    });
    cancelled.rime.cancelWeb06EventFanout(cancelledEvent, "ordinary-click-cancelled-long-press");
    const afterCancel = ownedAction(cancelled.rime, "getUserdbSnapshot", []);
    const afterCancelIdentity = cancelled.rime.web06ActionIdentityFor(afterCancel)!;
    expect(afterCancelIdentity.supersessionSubRunId).toBe(1);
    expect(cancelled.debug.snapshot().events[0]).toMatchObject({
      identity: { classification: "frontend-consumed", mappedActionCount: 0 },
      mappedActions: [],
      linkedActionIds: [],
    });

    const consumed = await loadProtocol("minimal");
    let consumedEvent: any;
    consumed.rime.withWeb06ControlEvent({ type: "mousedown", timeStamp: 1 }, () => {
      consumedEvent = consumed.rime.declareWeb06ControlFanout("candidate-long-press", [{
        owner: "candidate-delete",
        action: web06ControlAction("deleteCandidate", [0]),
      }]);
    });
    const deletion = consumed.rime.withWeb06OwnedAction(
      "candidate-delete",
      "deleteCandidate",
      [0],
      "candidate-long-press",
      undefined,
      () => consumed.rime.default.deleteCandidate(0),
    );
    const deletionIdentity = consumed.rime.web06ActionIdentityFor(deletion)!;
    const next = ownedAction(consumed.rime, "getUserdbSnapshot", []);
    const nextIdentity = consumed.rime.web06ActionIdentityFor(next)!;
    expect(deletionIdentity.supersessionSubRunId).toBe(1);
    expect(nextIdentity.supersessionSubRunId).toBe(2);
    expect(consumedEvent.mappedActionCount).toBe(1);
  });

  it("keeps a partially consumed fanout mapped and rejects atomic cancellation", async () => {
    const { rime, debug } = await loadProtocol("minimal");
    const actions = Array.from({ length: 12 }, (_, index) => ({
      owner: `owner-${index}`,
      action: web06ControlAction("setOption", [`option_${index}`, true]),
    }));
    let event: any;
    rime.withWeb06ControlEvent({ type: "click", timeStamp: 1 }, () => {
      event = rime.declareWeb06ControlFanout("partial-live-options", actions);
    });
    rime.withWeb06OwnedAction(
      actions[0]!.owner,
      "setOption",
      actions[0]!.action.args,
      "partial-live-options",
      undefined,
      () => rime.default.setOption("option_0", true),
    );
    rime.cancelWeb06EventFanout(event, "must-not-partially-cancel");
    const snapshot = debug.snapshot();
    expect(snapshot.events[0]).toMatchObject({
      identity: { classification: "mapped-action(s)", mappedActionCount: 12 },
    });
    expect(snapshot.events[0].mappedActions).toHaveLength(12);
    expect(snapshot.events[0].linkedActionIds).toHaveLength(12);
    expect(snapshot.status.pendingFanoutActions).toBe(11);
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("CANCELLED_PARTIAL_FANOUT");
  });

  it("consumes the real mixed schema/deploy fanout in exact per-action boundary order", async () => {
    const { rime, debug } = await loadProtocol("minimal");
    const planned = web06SchemaChangeFanout({
      nextSchema: "luna_pinyin",
      applyDeployPreferences: true,
      deployPreferences: {
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
      },
      liveOptions: {
        isAsciiMode: false,
        isFullShape: false,
        outputStandard: "hong_kong_traditional",
        activeSchema: "jyut6ping3",
        isExtendedCharset: false,
        isDisabled: false,
      },
    });
    rime.withWeb06ControlEvent({ type: "change", timeStamp: 1 }, () => {
      rime.declareWeb06ControlFanout("schema-control", planned);
      for (const item of planned) {
        rime.withWeb06OwnedAction(
          item.owner,
          item.action.name,
          item.action.args,
          "schema-control",
          undefined,
          () => (rime.default[item.action.name] as (...args: any[]) => Promise<unknown>)(...item.action.args),
        );
      }
    });
    const snapshot = debug.snapshot();
    expect(snapshot.status.pendingFanoutActions).toBe(0);
    expect(snapshot.invalidations).toEqual([]);
    expect(snapshot.actions.map((receipt: any) => receipt.name)).toEqual(planned.map(item => item.action.name));
    expect(snapshot.actions.map((receipt: any) => receipt.identity.compositionEpochId)).toEqual(
      planned.map((_, index) => index + 1),
    );
  });

  it("rejects a mutated fanout argument by direct equality even in content-free minimal mode", async () => {
    const { rime, debug, worker } = await loadProtocol("minimal");
    rime.withWeb06ControlEvent({ type: "change", timeStamp: 1 }, () => {
      rime.declareWeb06ControlFanout("minimal-argument-negative", [{
        owner: "fanout-owner",
        action: web06ControlAction("setOption", ["ascii_mode", true]),
      }]);
    });
    rime.withWeb06OwnedAction(
      "fanout-owner",
      "setOption",
      ["ascii_mode", false],
      "minimal-argument-negative",
      undefined,
      () => rime.default.setOption("ascii_mode", false),
    );
    expect(worker.action(0).args).toEqual(["ascii_mode", false]);
    expect(debug.status()).toMatchObject({ pendingFanoutActions: 1, valid: false });
    expect(debug.invalidations().map(entry => entry.code)).toContain("FANOUT_OWNER_ACTION_MISMATCH");
    expect(JSON.stringify(debug.snapshot().events)).not.toContain("ascii_mode");
  });

  it("enforces deferred fanout arguments as an explicit one-shot state transition", async () => {
    const positive = await loadProtocol("full");
    let positiveEvent: any;
    positive.rime.withWeb06ControlEvent({ type: "change", timeStamp: 1 }, () => {
      positiveEvent = positive.rime.declareWeb06ControlFanout("deferred-positive", [{
        owner: "deferred-owner",
        action: web06DeferredControlAction("importUserdb", ["<pending-file-text>"]),
      }]);
    });
    expect(positive.rime.resolveWeb06DeferredFanoutAction(positiveEvent, 0, ["# resolved userdb"])).toBe(true);
    const positiveAction = positive.rime.withWeb06OwnedAction(
      "deferred-owner",
      "importUserdb",
      ["# resolved userdb"],
      "deferred-positive",
      undefined,
      () => positive.rime.default.importUserdb("# resolved userdb"),
    );
    expect(positive.worker.action(0).args).toEqual(["# resolved userdb"]);
    expect(positive.rime.web06ActionIdentityFor(positiveAction)).toMatchObject({
      originKind: "dom-event",
      eventId: positiveEvent.eventId,
      eventActionIndex: 0,
    });
    expect(positive.debug.status().pendingFanoutActions).toBe(0);
    expect(positive.rime.resolveWeb06DeferredFanoutAction(positiveEvent, 0, ["# too late"])).toBe(false);
    expect(positive.debug.invalidations().map(entry => entry.code)).toContain("DEFERRED_FANOUT_RESOLUTION_MISSING");

    const duplicate = await loadProtocol("minimal");
    let duplicateEvent: any;
    duplicate.rime.withWeb06ControlEvent({ type: "change", timeStamp: 2 }, () => {
      duplicateEvent = duplicate.rime.declareWeb06ControlFanout("deferred-duplicate", [{
        owner: "deferred-owner",
        action: web06DeferredControlAction("importUserdb", ["<pending-file-text>"]),
      }]);
    });
    expect(duplicate.rime.resolveWeb06DeferredFanoutAction(duplicateEvent, 0, ["# first"])).toBe(true);
    expect(duplicate.rime.resolveWeb06DeferredFanoutAction(duplicateEvent, 0, ["# second"])).toBe(false);
    expect(duplicate.debug.invalidations().map(entry => entry.code)).toContain("DEFERRED_FANOUT_ALREADY_RESOLVED");

    const ordinary = await loadProtocol("minimal");
    let ordinaryEvent: any;
    ordinary.rime.withWeb06ControlEvent({ type: "change", timeStamp: 3 }, () => {
      ordinaryEvent = ordinary.rime.declareWeb06ControlFanout("ordinary-control", [{
        owner: "ordinary-owner",
        action: web06ControlAction("setOption", ["ascii_mode", true]),
      }]);
    });
    expect(ordinary.rime.resolveWeb06DeferredFanoutAction(ordinaryEvent, 0, ["ascii_mode", false])).toBe(false);
    expect(ordinary.debug.invalidations().map(entry => entry.code)).toContain("DEFERRED_FANOUT_ACTION_NOT_DEFERRED");

    const unresolved = await loadProtocol("minimal");
    let unresolvedEvent: any;
    unresolved.rime.withWeb06ControlEvent({ type: "change", timeStamp: 4 }, () => {
      unresolvedEvent = unresolved.rime.declareWeb06ControlFanout("deferred-unresolved", [{
        owner: "deferred-owner",
        action: web06DeferredControlAction("importUserdb", ["<pending-file-text>"]),
      }]);
    });
    const unresolvedAction = unresolved.rime.withWeb06OwnedAction(
      "deferred-owner",
      "importUserdb",
      ["# unresolved userdb"],
      "deferred-unresolved",
      undefined,
      () => unresolved.rime.default.importUserdb("# unresolved userdb"),
    );
    expect(unresolved.rime.web06ActionIdentityFor(unresolvedAction)).toMatchObject({ originKind: "background" });
    expect(unresolved.debug.status().pendingFanoutActions).toBe(1);
    expect(unresolved.debug.invalidations().map(entry => entry.code)).toContain("UNRESOLVED_DEFERRED_FANOUT_ACTION");
  });

  it("rejects mutated listener arguments even when their provenance digest is forged to match", async () => {
    const { rime, debug, worker } = await loadProtocol("minimal");
    const action = ownedAction(rime, "setOption", ["ascii_mode", true]);
    emitListener(worker, 0, "optionChanged", ["ascii_mode", true]);
    emitSuccess(worker, 0, undefined, {
      kind: "empty",
      resultDigest: web06StableDigest(null),
      success: true,
      persistenceCompleted: false,
    }, [
      {
        kind: "listener",
        name: "optionChanged",
        argsDigest: web06StableDigest(["ascii_mode", true]),
        args: ["ascii_mode", false],
        recordedAt: performance.now(),
      },
      { kind: "engine-state", name: "setOption", resultDigest: web06StableDigest(null), recordedAt: performance.now() },
    ]);
    await action;
    const snapshot = debug.snapshot();
    expect(snapshot.actions[0].lifecycle.outcome).toBe("failure");
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("LISTENER_EFFECT_MISMATCH");
    expect(JSON.stringify(snapshot.actions[0].worker.lifecycleEffects)).not.toContain("ascii_mode");
  });

  it("bounds and reset-clears callback/fanout ledgers through one atomic snapshot", async () => {
    const { rime, debug } = await loadProtocol("minimal");
    const emptyMapping = {
      classification: "frontend-consumed" as const,
      reason: "pipeline-empty-event",
      preventDefault: false,
      actions: [],
    };
    for (let index = 0; index <= 2_048; index += 1) {
      rime.recordWeb06DomEvent({
        type: "keydown",
        key: "",
        code: "",
        timeStamp: index,
        repeat: false,
        ctrlKey: false,
        metaKey: false,
        altKey: false,
        shiftKey: false,
      }, emptyMapping, performance.now());
    }
    let snapshot = debug.snapshot();
    expect(snapshot.mainObserverCallbacksMs).toHaveLength(2_048);
		expect(snapshot.mainObserverCallbacks).toHaveLength(2_048);
    expect(snapshot.status).toMatchObject({
		mainObserverCallbackCount: 2_048,
      mainObserverCallbackCapacity: 2_048,
      mainObserverCallbackOverflowCount: 1,
      valid: false,
    });
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("MAIN_OBSERVER_CALLBACK_RING_OVERFLOW");

    debug.resetReceipts();
    snapshot = debug.snapshot();
    expect(snapshot.mainObserverCallbacksMs).toEqual([]);
		expect(snapshot.mainObserverCallbacks).toEqual([]);
		expect(snapshot.status).toMatchObject({
		mainObserverCallbackCount: 0,
		mainObserverCallbackOverflowCount: 0,
		valid: true,
	});

    const actions = Array.from({ length: 300 }, () => ({
      owner: "overflow-owner",
      action: web06ControlAction("setOption", ["ascii_mode", true]),
    }));
    rime.withWeb06ControlEvent({ type: "click", timeStamp: 3_000 }, () => {
      rime.declareWeb06ControlFanout("pipeline-fanout-overflow", actions);
    });
    snapshot = debug.snapshot();
    expect(snapshot.status).toMatchObject({ valid: false, pendingFanoutActions: 0 });
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("FANOUT_RING_OVERFLOW");
  });

  it("fails closed on a slow presentation/supersession observer without changing its result", async () => {
    const { rime, debug } = await loadProtocol("minimal");
    let clock = 100;
    const now = vi.spyOn(performance, "now").mockImplementation(() => {
      clock += 6;
      return clock;
    });
    const result = rime.observeWeb06Measurement(
      "presentation-terminal-and-supersession",
      undefined,
      () => "public-result-unchanged",
    );
    now.mockRestore();
    expect(result).toBe("public-result-unchanged");
    const snapshot = debug.snapshot();
    expect(snapshot.mainObserverCallbacksMs.some((duration: number) => duration >= 5)).toBe(true);
    expect(snapshot.invalidations.map((entry: any) => entry.code)).toContain("OBSERVER_CALLBACK_OVER_5MS");
  });

	it("times the real fanout, identity, promise, and transport bookkeeping paths without changing the public result", async () => {
		const { rime, debug, worker } = await loadProtocol("minimal");
		let clock = 100;
		const now = vi.spyOn(performance, "now").mockImplementation(() => {
			clock += 6;
			return clock;
		});
		let action!: ReturnType<Actions["setOption"]>;
		let event: any;
		try {
			rime.withWeb06ControlEvent({ type: "change", timeStamp: 1 }, () => {
				event = rime.declareWeb06ControlFanout("timed-real-fanout", [{
					owner: "timed-owner",
					action: web06ControlAction("setOption", ["ascii_mode", true]),
				}]);
				action = rime.withWeb06OwnedAction(
					"timed-owner",
					"setOption",
					["ascii_mode", true],
					"timed-real-fanout",
					undefined,
					() => rime.default.setOption("ascii_mode", true),
				);
			});
			expect(rime.web06ActionIdentityFor(action)).toMatchObject({
				originKind: "dom-event",
				eventId: event.eventId,
			});
			emitListener(worker, 0, "optionChanged", ["ascii_mode", true]);
			emitSuccess(worker, 0, undefined, {
				kind: "empty",
				resultDigest: web06StableDigest(null),
				success: true,
				persistenceCompleted: false,
			}, [
				{ kind: "listener", name: "optionChanged", argsDigest: web06StableDigest(["ascii_mode", true]), args: ["ascii_mode", true], recordedAt: performance.now() },
				{ kind: "engine-state", name: "setOption", resultDigest: web06StableDigest(null), recordedAt: performance.now() },
			]);
			expect(await action).toBeUndefined();
			const snapshot = debug.snapshot();
			const requiredOperations = [
				"declare-control-fanout",
				"register-event-fanout",
				"take-fanout-context",
				"advance-action-boundary",
				"create-action-identity",
				"mark-action-sent",
				"bind-action-promise",
				"lookup-action-promise",
			];
			const operations = snapshot.mainObserverCallbacks.map((callback: any) => callback.operation);
			expect(operations).toEqual(expect.arrayContaining(requiredOperations));
			expect(snapshot.mainObserverCallbacks.every((callback: any) =>
				callback.finishedAt - callback.startedAt === callback.durationMs
				&& callback.durationMs >= 5
			)).toBe(true);
			const slowOperations = snapshot.invalidations
				.filter((entry: any) => entry.code === "OBSERVER_CALLBACK_OVER_5MS")
				.map((entry: any) => entry.detail.split(" took ")[0]);
			expect(slowOperations).toEqual(expect.arrayContaining(requiredOperations));
		}
		finally {
			now.mockRestore();
		}
	});

  it("bounds pending presentation tokens to receipt capacity and retires evicted/stale terminals", async () => {
    const { rime, debug, worker } = await loadProtocol("minimal");
    let firstIdentity!: Web06ActionIdentity;
    let latestIdentity!: Web06ActionIdentity;
    const result = composingResult("z");
    for (let index = 0; index <= WEB06_MINIMAL_RECEIPT_CAPACITY; index += 1) {
      const promise = ownedAction(rime, "processKey", ["{z}"]);
      const identity = rime.web06ActionIdentityFor(promise)!;
      if (index === 0) firstIdentity = identity;
      latestIdentity = identity;
      emitSuccess(worker, index, result, {
        kind: "rime-result",
        resultDigest: `pending-${index}`,
        success: true,
        persistenceCompleted: false,
      });
      await promise;
    }
    expect(debug.status().pendingTerminalActions).toBe(WEB06_MINIMAL_RECEIPT_CAPACITY);
    expect(debug.invalidations().map(entry => entry.code)).toContain("PENDING_PRESENTATION_RING_OVERFLOW");

    recordPresentation(
      rime,
      firstIdentity,
      "painted",
      result,
      fingerprint(0),
      fingerprint(firstIdentity.sequenceId, "z"),
    );
    expect(debug.status().pendingTerminalActions).toBe(WEB06_MINIMAL_RECEIPT_CAPACITY);
    recordPresentation(
      rime,
      latestIdentity,
      "painted",
      result,
      fingerprint(0),
      fingerprint(latestIdentity.sequenceId, "z"),
    );
    expect(debug.status().pendingTerminalActions).toBe(WEB06_MINIMAL_RECEIPT_CAPACITY - 1);
  });

  it("performs an actual one-shot reload handoff with terminal, persistence, and userdb proof", async () => {
    const storage = new MemoryStorage();
    const firstRealm = await loadProtocol("full", storage);
    const committedResult: RimeResult = { isComposing: false, success: true, committed: "我係個" };
    const committedRawJson = JSON.stringify({
      handled: true,
      commits: ["我係個"],
      context: null,
      status: null,
    });
    const committed = ownedAction(firstRealm.rime, "processKey", ["{space}"]);
    const committedIdentity = firstRealm.rime.web06ActionIdentityFor(committed)!;
    emitSuccess(firstRealm.worker, 0, committedResult, {
      kind: "rime-result",
      resultDigest: "commit-result",
      success: true,
      persistenceCompleted: true,
      committedTextDigest: web06StableDigest("我係個"),
      committedUtf16Length: 3,
    }, [], committedRawJson);
    await committed;
    recordPresentation(
      firstRealm.rime,
      committedIdentity,
      "committed",
      committedResult,
      fingerprint(committedIdentity.sequenceId),
      fingerprint(committedIdentity.sequenceId, "", "我係個"),
    );

    const learnedSnapshot = {
      schemaId: "jyut6ping3" as const,
      dictionaryId: "jyut6ping3",
      path: "/rime/jyut6ping3.userdb",
      exists: true,
      bytes: 12,
      updatedAt: null,
      rows: [],
      rawText: "",
      parseErrors: [],
    };
    const learnedDigest = web06UserdbSnapshotDigest(learnedSnapshot);
    const userdb = ownedAction(firstRealm.rime, "getUserdbSnapshot", [], committedIdentity);
    emitSuccess(firstRealm.worker, 1, learnedSnapshot, {
      kind: "userdb-snapshot",
      resultDigest: learnedDigest,
      success: true,
      persistenceCompleted: false,
      userdbDigest: learnedDigest,
      userdbRowCount: 0,
      userdbBytes: 12,
    }, [{
      kind: "snapshot-read",
      name: "getUserdbSnapshot",
      resultDigest: learnedDigest,
      recordedAt: performance.now(),
    }]);
    await userdb;
    firstRealm.rime.recordWeb06OwnedResultEffect(userdb, "ui-userdb-refresh", {
		expectedState: { digest: learnedDigest },
		readObservedState: () => ({ digest: learnedDigest }),
    });

    const pre = firstRealm.debug.prepareLearnedReloadContinuity(
      "learned-row-attempt-01",
      committedIdentity.actionId,
    );
    expect(pre).toMatchObject({
      phase: "pre-reload",
      measurementId: "learned-row-attempt-01",
      terminal: {
        actionId: committedIdentity.actionId,
        persistenceCompleted: true,
      },
      userdb: { digest: learnedDigest, rowCount: 0, bytes: 12 },
      queueIdle: true,
      allActionsCompleted: true,
      storagePayloadKeys: ["measurementId", "continuityNonce"],
    });
    expect(Object.keys(JSON.parse(storage.getItem("__yune_web06_reload_continuity_v1__")!)).sort()).toEqual([
      "continuityNonce",
      "measurementId",
    ]);

    const secondRealm = await loadProtocol("full", storage);
    const arrived = secondRealm.debug.snapshot().reloadContinuity;
    expect(arrived).toMatchObject({
      phase: "post-reload-arrived",
      measurementId: pre.measurementId,
      continuityNonce: pre.continuityNonce,
      storageRemoved: true,
      oneShot: true,
    });
    expect(arrived.pageInstanceId).not.toBe(pre.pageInstanceId);
    expect(storage.getItem("__yune_web06_reload_continuity_v1__")).toBeNull();

    secondRealm.debug.resetReceipts();
    const bound = secondRealm.debug.bindLearnedReloadWindow(pre.measurementId, pre.continuityNonce);
    expect(bound).toMatchObject({
      phase: "post-reload-bound",
      measurementId: pre.measurementId,
      receiptWindowStartEventSequenceId: 1,
      receiptWindowStartActionSequenceId: 1,
      lastEventSequenceId: 0,
      lastActionSequenceId: 0,
      requiresFreshDriverPageCalibration: true,
      requiresFreshWorkerCalibration: true,
    });
    expect(() => secondRealm.debug.bindLearnedReloadWindow(pre.measurementId, pre.continuityNonce)).toThrow(
      /RELOAD_CONTINUITY_ARRIVAL_MISSING/,
    );

    const thirdRealm = await loadProtocol("full", storage);
    expect(thirdRealm.debug.snapshot()).not.toHaveProperty("reloadContinuity");
  });

  it("keeps protocol action contracts available in the integrated harness", () => {
    expect(web06ActionContract("setOption")).toEqual({ actionClass: "stateful-barrier", boundary: "option" });
  });
});
