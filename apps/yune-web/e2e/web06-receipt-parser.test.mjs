import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  PEER_LOGICAL_INPUT_IDS,
  buildFiveRoundEvidenceSummary,
  buildRoundEvidenceSummary,
  buildSummarySemanticProjection,
  canonicalizeSummaryDiagnosticCodes,
  computeBindingPeerRatio,
  evaluateCollectionEquivalence,
  evaluateFiveRoundCommonPool,
  evaluateFiveRoundPool,
  evaluatePackageAlignment,
  normalizeWireActionArgs,
  publicEvidenceReceipt,
  terminalOwnerSequenceId,
  summarySemanticProjectionBytes,
  validateAndRecomputeReceipt,
  validateCommonSurfaceReceipt,
  validatePublicEvidenceSchema,
  validatePublicFiveRoundSummarySchema,
  validatePublicRoundSummarySchema,
  validatePointerFreePrivacy,
  validateSupersessionGraph,
  web06StableDigest,
} from "./web06-receipt-parser.mjs";
import {
  WEB06_BEHAVIOR_PREDICATE_VERSION,
  WEB06_METRIC_CONTRACT_VERSION,
  WEB06_SCENARIO_REGISTRY_VERSION,
  WEB06_THRESHOLDS,
  SCENARIO_REGISTRY,
  SCENARIO_RUN_REGISTRY,
  WEB06_PRESSURE_PAIR_REGISTRY,
  evaluateAttemptSeries,
  expandScenarioExpectedTimeline,
} from "./web06-metric-contract.mjs";
import {
  independentWeb06StableDigest,
  independentlyBindSummaryDiagnostics,
  independentClassifyObserverLongTasks,
  independentExpandScenarioExpectedTimeline,
  independentlyAuditRawEnvelope,
  independentlyValidateCompletedRawDecisionShape,
  independentlyRecomputeFiveRoundSummary,
  independentlyRecomputeRoundSummary,
  independentlyProjectSummarySemantics,
  independentlyProjectObserverModeRawEvidence,
  independentlyRecomputeIncompleteAttemptFacts,
  independentlyValidateObserverModeProjectionSchema,
  independentlyValidatePointerFreePrivacy,
  independentlyValidateRawSentinelIntegrity,
  independentlyValidateActionTiming,
  independentlyValidateSupersessionGraph,
  independentlyVerifyBurstRecoveries,
  validateIndependentDiagnosticBindingPairSchema,
  validateIndependentRecomputeSchema,
  verifyCollectorOutput,
  verifyIndependentPressureProofs,
} from "./web06-independent-verifier.mjs";
import {
  WEB06_COLLECTOR_CONTRACT_SHA256,
  WEB06_BINDING_SCENARIO_ORDER,
  buildIncompleteObserverModeProjection,
  combinedAttemptFacts,
  parseAndCompactCommonReceipt,
  retainedRawReceiptBehaviorErrors,
  resolveCommonSamples,
  sentinelLedgerIntegrityErrors,
  validateCompletedRawDecisionShape,
  validateWeb06ObserverModeProjectionSchema,
} from "./web06-collector.mjs";

const COMMIT = "0123456789abcdef0123456789abcdef01234567";
const TREE = "1234567890abcdef1234567890abcdef12345678";
const HASH_A = "a".repeat(64);
const HASH_B = "b".repeat(64);
const PAGE_ID = "00000000-0000-4000-8000-000000000001";
const ZERO_SENTINEL_OVERFLOWS = Object.freeze(Object.fromEntries([
  "events", "auxiliaryEvents", "unmatchedEvents", "snapshots", "frameTimestamps", "longTasks", "focus",
  "callbacks", "windows", "idleControls", "driverDispatchBindings", "pendingCaptures",
].map((field) => [field, 0])));

function runnerSourceObservation({ toolingManifestSha256 = HASH_B } = {}) {
  const snapshot = {
    version: "web06-runner-source-observation-v1",
    sourceCommit: COMMIT,
    sourceTree: TREE,
    sourceTreeState: "clean",
    toolingManifestSha256,
    files: [{ path: "apps/yune-web/e2e/fixture.mjs", sha256: HASH_A }],
  };
  return { ...snapshot, observationSha256: createHash("sha256")
    .update(JSON.stringify(snapshot), "utf8").digest("hex") };
}

function refreshRunnerSourceObservationHash(observation) {
  const snapshot = {
    version: observation.version,
    sourceCommit: observation.sourceCommit,
    sourceTree: observation.sourceTree,
    sourceTreeState: observation.sourceTreeState,
    toolingManifestSha256: observation.toolingManifestSha256,
    files: observation.files,
  };
  observation.observationSha256 = createHash("sha256")
    .update(JSON.stringify(snapshot), "utf8").digest("hex");
}

function refreshSemanticProjectionHash(summary) {
  summary.semanticProjectionSha256 = createHash("sha256")
    .update(summarySemanticProjectionBytes(summary), "utf8").digest("hex");
  return summary;
}

function refreshDiagnosticBinding(summary) {
  const fields = summary.version === "web06-round-summary-v1"
    ? ["setupErrorCodes", "behaviorErrorCodes", "thresholdViolations"]
    : ["summaryErrors", "poolViolations"];
  for (const field of fields) {
    const raw = summary[field];
    const semantic = canonicalizeSummaryDiagnosticCodes(raw);
    summary.implementationDiagnosticBinding.dimensions[field] = {
      rawCount: raw.length,
      rawSha256: createHash("sha256").update(JSON.stringify(raw), "utf8").digest("hex"),
      semanticCount: semantic.length,
      semanticSha256: createHash("sha256").update(JSON.stringify(semantic), "utf8").digest("hex"),
    };
  }
  if (summary.version === "web06-five-round-summary-v1") {
    summary.implementationDiagnosticBinding.roundBindingsSha256 = createHash("sha256")
      .update(JSON.stringify(summary.roundSummaries.map((round) => round.implementationDiagnosticBinding)), "utf8")
      .digest("hex");
  }
  return refreshSemanticProjectionHash(summary);
}

function observedEnvironmentObservation() {
  const observation = {
    version: "web06-observed-environment-v1",
    toolchain: {
      rust: "rustc test",
      emscripten: "emcc test",
      node: "v24.test",
      npm: "11.test",
      playwright: "1.test",
      chromium: "test-revision",
      chromiumExecutableSha256: HASH_A,
    },
    host: {
      os: "macOS",
      osVersion: "test",
      osBuildVersion: "test-build",
      arch: "arm64",
      cpuModel: "Apple test",
      logicalCores: 10,
      memoryBytes: 34359738368,
      powerState: "AC",
      lowPowerMode: false,
    },
  };
  return { ...observation, observationSha256: createHash("sha256")
    .update(JSON.stringify(observation), "utf8").digest("hex") };
}

test("stable 128-bit protocol digests match frozen cross-implementation vectors", () => {
  const vectors = [
    [null, "ff7c0b4d9a24ea69c423dc8b3c5e47dd"],
    ["", "91a327e4e6e90ab0befeb8725118cf54"],
    [true, "307d80851ec876c9e02e9cc3ac5c2dad"],
    [0, "dbcbef783228625428aa862e6d3c8ce0"],
    [-0, "49525db528343f39be8cea6311da7c5d"],
    [[], "e8cc5d6a1e8a6a2aa2b15a802a27a932"],
    [{}, "ee838c706da2363c6347602e8fde29b0"],
    [[1, "x"], "b1962c704e6edd643a9d7f66266e1610"],
    [{ b: 2, a: [1, "x"] }, "f6620c5bbcc21097c4902945fc2e885b"],
    ["我係個", "6ca3b4ed879a762f52719df95c624289"],
  ];
  for (const [value, expected] of vectors) {
    assert.equal(web06StableDigest(value), expected);
    assert.equal(independentWeb06StableDigest(value), expected);
  }
  assert.notEqual(web06StableDigest({ a: 1 }), web06StableDigest({ a: 2 }));
});

test("verifier-owned timeline expansion matches every frozen scenario event and action", () => {
  for (const scenarioId of Object.keys(SCENARIO_REGISTRY)) {
    assert.deepEqual(independentExpandScenarioExpectedTimeline(scenarioId),
      expandScenarioExpectedTimeline(scenarioId), scenarioId);
  }
  for (const scenarioId of ["toString", "__proto__", "constructor"]) {
    assert.throws(() => independentExpandScenarioExpectedTimeline(scenarioId), /WEB06_UNKNOWN_SCENARIO/);
  }
});

test("semantic diagnostic projection collapses only same-predicate aliases and preserves multiplicity", () => {
  for (const code of ["toString", "__proto__", "constructor"]) {
    assert.deepEqual(canonicalizeSummaryDiagnosticCodes([code]), [code]);
    const summary = buildRoundEvidenceSummary(validReceipt(), { surface: "internal" });
    summary.behaviorErrorCodes = [code];
    assert.deepEqual(independentlyProjectSummarySemantics(summary).summary.behaviorErrorCodes, [code]);
  }
  assert.deepEqual(canonicalizeSummaryDiagnosticCodes([
    "BEHAVIOR_PAGE_VISIBLE_COUNT:step-1",
    "BEHAVIOR_VISIBLECOUNT:step-1",
  ]), ["BEHAVIOR_VISIBLE_COUNT:step-1"]);
  assert.deepEqual(canonicalizeSummaryDiagnosticCodes([
    "BEHAVIOR_PAGE_VISIBLE_COUNT:step-1",
    "BEHAVIOR_PAGE_VISIBLE_COUNT:step-1",
  ]), [
    "BEHAVIOR_VISIBLE_COUNT:step-1",
    "BEHAVIOR_VISIBLE_COUNT:step-1",
  ], "same-label multiplicity cannot disappear");
  assert.deepEqual(canonicalizeSummaryDiagnosticCodes([
    "BURST_RECOVERY_INVALID:burst-1",
    "BURST_RECOVERY_IDENTITY:burst-1",
    "BURST_RECOVERY_INVALID:burst-2",
  ]), [
    "BURST_RECOVERY_IDENTITY:burst-1",
    "BURST_RECOVERY_IDENTITY:burst-2",
  ], "per-instance identity survives true synonym collapse");
  const divergentSuffixes = [
    "BURST_RECOVERY_INVALID:burst-1:expected-a",
    "BURST_RECOVERY_IDENTITY:burst-1:observed-b",
    "BEHAVIOR_PAGE_VISIBLE_COUNT:step-1:expected-6",
    "BEHAVIOR_VISIBLECOUNT:step-1:observed-7",
  ];
  assert.deepEqual(canonicalizeSummaryDiagnosticCodes(divergentSuffixes), [
    "BEHAVIOR_VISIBLE_COUNT:step-1:expected-6",
    "BEHAVIOR_VISIBLE_COUNT:step-1:observed-7",
    "BURST_RECOVERY_IDENTITY:burst-1:expected-a",
    "BURST_RECOVERY_IDENTITY:burst-1:observed-b",
  ], "aliasing preserves every diagnostic suffix component");
  const divergentSummary = buildRoundEvidenceSummary(validReceipt(), { surface: "internal" });
  divergentSummary.behaviorErrorCodes = divergentSuffixes;
  assert.deepEqual(
    independentlyProjectSummarySemantics(divergentSummary).summary.behaviorErrorCodes,
    canonicalizeSummaryDiagnosticCodes(divergentSuffixes),
    "independent and producer alias grammars preserve divergent suffixes identically",
  );
  assert.deepEqual(canonicalizeSummaryDiagnosticCodes([
    "EVENT_REORDERED:1",
    "EVENT_CLASSIFICATION:1",
    "EVENT_ACTION_CARDINALITY:1",
    "WORKER_SPAN_INVALID:1:abi",
    "WORKER_SPAN_OUTSIDE_ACTION:1:abi",
  ]), [
    "EVENT_ACTION_CARDINALITY:1",
    "EVENT_CLASSIFICATION:1",
    "EVENT_REORDERED:1",
    "WORKER_SPAN_INVALID:1:abi",
    "WORKER_SPAN_OUTSIDE_ACTION:1:abi",
  ], "materially distinct contract predicates never alias");
});

test("independent pressure verifier recomputes every raw inequality and same-task dispatch operand", () => {
  const pairs = WEB06_PRESSURE_PAIR_REGISTRY["fifo-pressure-barriers"];
  const actions = pairs.flatMap((pair, index) => {
    const base = index * 100;
    const earlierSequenceId = index * 2 + 1;
    return [{
      stepId: pair.earlierStepId,
      sequenceId: earlierSequenceId,
      originKind: "dom-event",
      outcome: "barrier-completed",
      workerSentAt: base + 1,
      mainResponseReceivedAt: base + 10,
      terminalObservedAt: base + 20,
      event: { actualDriverDispatchAt: base + 0.5 },
    }, {
      stepId: pair.laterStepId,
      sequenceId: earlierSequenceId + 1,
      originKind: "dom-event",
      outcome: "painted",
      terminalKind: "presentation",
      actionEnqueuedAt: base + 5,
      mainQueueDepth: 1,
      stateCommittedAt: base + 21,
      corrected: { start: { correctedAt: base + 12, uncertainty: 1 } },
      event: { actualDriverDispatchAt: base + 0.5 },
    }];
  });
  const receipt = { pressureProofs: pairs.map((pair, index) => ({
    ...pair,
    earlierSequenceId: index * 2 + 1,
    laterSequenceId: index * 2 + 2,
    dispatchContract: "single-page-task-no-await",
  })) };
  const row = SCENARIO_REGISTRY["fifo-pressure-barriers"];
  assert.deepEqual(verifyIndependentPressureProofs(receipt, row, actions), []);
  const mutations = [
    (r, _a) => { r.pressureProofs[0].dispatchContract = "microtask-separated"; },
    (_r, a) => { a[0].workerSentAt = 6; },
    (_r, a) => { a[1].actionEnqueuedAt = 10; },
    (_r, a) => { a[1].mainQueueDepth = 0; },
    (_r, a) => { a[1].corrected.start.correctedAt = 10; },
    (_r, a) => { a[0].terminalObservedAt = 22; },
    (_r, a) => { a[1].event.actualDriverDispatchAt = 0.75; },
    (r, _a) => { r.pressureProofs[0].earlierSequenceId = 99; },
  ];
  for (const mutate of mutations) {
    const mutatedReceipt = structuredClone(receipt);
    const mutatedActions = structuredClone(actions);
    mutate(mutatedReceipt, mutatedActions);
    assert.ok(verifyIndependentPressureProofs(mutatedReceipt, row, mutatedActions)
      .includes("FIFO_PRESSURE_NOT_PROVED:commit-then-type"));
  }
});

test("independent Long Task verifier requires complete callback intervals and conservative same-locus residuals", () => {
  const sentinel = {
    callbackId: "sentinel-1", sequenceId: 1, kind: "dom-event",
    pageInstanceId: PAGE_ID, eventSequenceId: 1, windowIndex: 0,
    startedAt: 90, finishedAt: 91, durationMs: 1, sourceClass: "common-sentinel",
  };
  const mode = (name) => ({
    callbackAttributionComplete: true,
    callbackLedgerCount: 1,
    callbackLedgerCapacity: 8,
    sentinelAccountedCallbackCount: 1,
    callbackLedgerOverflowCount: 0,
    commonEventCount: 1,
    interactionWindowCount: 1,
    sentinelCallbacksMs: [1],
    sentinelTotalPerEventMs: [1],
    sentinelTotalPerWindowMs: [1],
    collectorCallbacksMs: name === "product" ? [] : [5],
    mainObserverCallbacksMs: name === "product" ? [] : [5],
    workerCollectorCallbacksMs: [],
    callbackIntervals: name === "product" ? [sentinel] : [sentinel, {
      callbackId: "web06-main-observer-00000001",
      sequenceId: 1,
      operation: "capture",
      startedAt: 100,
      finishedAt: 105,
      durationMs: 5,
      sourceClass: name === "minimal" ? "minimal-probe" : "full-collector",
    }],
    mainObserverCallbackCount: name === "product" ? undefined : 1,
    mainObserverCallbackCapacity: name === "product" ? undefined : 64,
    mainObserverCallbackOverflowCount: name === "product" ? undefined : 0,
    rawLongTasks: [{
      startTime: 100,
      durationMs: 60,
      pageInstanceId: PAGE_ID,
      overlapsInteractionWindow: true,
      overlapsIdleControl: false,
      locus: "0:rapid-1",
    }],
    underlyingLongTasksMs: [60],
    instrumentationAddedLongTasksMs: [],
  });
  const valid = { product: mode("product"), minimal: mode("minimal"), full: mode("full") };
  assert.equal(independentClassifyObserverLongTasks(valid).pass, true);
  const mutations = [
    (m) => { m.minimal.callbackIntervals[1].sourceClass = "common-sentinel"; },
    (m) => { m.minimal.callbackIntervals.pop(); },
    (m) => {
      Object.assign(m.minimal.callbackIntervals[1], { startedAt: 130, finishedAt: 145, durationMs: 15 });
      m.minimal.collectorCallbacksMs = [15];
      m.minimal.mainObserverCallbacksMs = [15];
    },
    (m) => {
      m.minimal.callbackIntervals.push(structuredClone(m.minimal.callbackIntervals[1]));
      m.minimal.collectorCallbacksMs.push(5);
      m.minimal.mainObserverCallbacksMs.push(5);
      m.minimal.mainObserverCallbackCount = 2;
    },
    (m) => {
      Object.assign(m.minimal.callbackIntervals[1], { startedAt: 109, finishedAt: 120, durationMs: 11 });
      m.minimal.collectorCallbacksMs = [11];
      m.minimal.mainObserverCallbacksMs = [11];
    },
    (m) => { m.full.rawLongTasks[0].locus = "1:different"; },
  ];
  for (const mutate of mutations) {
    const changed = structuredClone(valid);
    mutate(changed);
    assert.equal(independentClassifyObserverLongTasks(changed).pass, false);
  }
  const workerClockMutation = structuredClone(valid);
  workerClockMutation.minimal.workerCollectorCallbacksMs = [10_000];
  workerClockMutation.minimal.collectorCallbacksMs = [5, 10_000];
  assert.equal(independentClassifyObserverLongTasks(workerClockMutation).pass, true,
    "worker-clock spans remain ceiling evidence but never overlap a page Long Task");
  assert.deepEqual(workerClockMutation.minimal.underlyingLongTasksMs, [60]);
});

function presentationDigest(value) {
  const { sequenceId: _sequenceId, ...state } = value;
  return web06StableDigest(state);
}

function driverExchange(base, transit = 0.1, service = 0.02) {
  return {
    d0: base,
    m1: base + transit,
    m2: base + transit + service,
    d3: base + transit * 2 + service,
  };
}

function workerExchange(base, transit = 0.1, service = 0.02) {
  return {
    m0: base,
    w1: base + transit,
    w2: base + transit + service,
    m3: base + transit * 2 + service,
  };
}

function actionReceipt(frozen, event, rawActionSequence, outcome) {
  const start = event.normalizedEventAt;
  const terminal = outcome === "committed";
  const logicalInput = rawActionSequence.filter((item) => item.length === 1).join("");
  const rawResponseJson = JSON.stringify({
    handled: true,
    commits: terminal ? ["你"] : [],
    context: terminal ? null : {
      input: logicalInput,
      preedit: logicalInput,
      caret: logicalInput.length,
      page_no: 0,
      page_size: 6,
      is_last_page: true,
      highlighted: 0,
      select_labels: ["1."],
      candidates: [{ text: "你", comment: "", source: "" }],
    },
    status: null,
  });
  const presentation = {
    sequenceId: frozen.sequenceId,
    input: terminal ? "" : logicalInput,
    page: 0,
    isLastPage: true,
    highlightedIndex: terminal ? -1 : 0,
    candidates: terminal ? [] : [{ label: "1.", text: "你", comment: "", source: "" }],
    status: null,
    textareaValue: terminal ? "你" : "",
    selectionStart: terminal ? 1 : 0,
    selectionEnd: terminal ? 1 : 0,
  };
  const presentationSha = presentationDigest(presentation);
  const wireIdentity = {
    actionId: `web06-action-${String(frozen.sequenceId).padStart(8, "0")}`,
    sequenceId: frozen.sequenceId,
    eventId: `web06-event-${String(frozen.eventSequenceId).padStart(8, "0")}`,
    eventSequenceId: frozen.eventSequenceId,
    compositionEpochId: "epoch-1",
    supersessionSubRunId: "subrun-1",
    actionClass: frozen.classification,
    supersedable: frozen.supersedable,
    originKind: frozen.originKind,
    originReason: frozen.originReason,
    causedByActionId: frozen.causedByActionId,
    causedBySequenceId: frozen.causedBySequenceId,
    causedByEventSequenceId: frozen.causedByEventSequenceId,
    rawInputSequence: rawActionSequence,
    actionEnqueuedAt: start + 2,
    mainQueueDepthAtEnqueue: 0,
    workerSentAt: start + 3,
    workerDispatchDepth: 0,
  };
  return {
    ...frozen,
    inputClass: undefined,
    stressDeadline: frozen.stressDeadline === true,
    terminalKind: "presentation",
    pageInstanceId: event.pageInstanceId,
    compositionEpochId: "epoch-1",
    supersessionSubRunId: "subrun-1",
    driverDispatchAt: event.actualDriverDispatchAt,
    actionEnqueuedAt: start + 2,
    mainQueueDepth: 0,
    workerDispatchDepth: 0,
    wireSequenceId: frozen.sequenceId,
    wireActionId: wireIdentity.actionId,
    wireArgs: structuredClone(frozen.args),
    argumentCommitments: {},
    wireIdentity,
    returnedWireIdentity: structuredClone(wireIdentity),
    workerSentAt: start + 3,
    workerMessageReceivedAt: start + 3.2,
    workerActionStartedAt: start + 3.4,
    workerFinishedAt: start + 4.4,
    mainResponseReceivedAt: start + 4.6,
    responseMappingStartedAt: start + 5,
    responseMappingFinishedAt: start + 6,
    stateUpdateScheduledAt: start + 6.5,
    stateCommittedAt: start + 7.5,
    stateAppliedAt: start + 7.5,
    ...(terminal ? { terminalObservedAt: start + 8.5 } : { paintObservedAt: start + 8.5 }),
    workerSpans: {
      abi: { start: start + 3.4, end: start + 3.8, outcomes: ["success"] },
      responseExtract: { start: start + 3.8, end: start + 4, outcomes: ["success"] },
      jsonParse: { start: start + 4, end: start + 4.2, outcomes: ["success"] },
      adapterTranslate: { start: start + 4.2, end: start + 4.4, outcomes: ["success"] },
      persistence: null,
    },
    persistenceRan: false,
    outcome,
    rawActionSequence,
    logicalInput: terminal ? "" : logicalInput,
    engineRawProof: {
      availability: "captured",
      action: frozen.kind,
      operation: frozen.kind,
      jsonDigest: "d".repeat(32),
      rawFingerprintDigest: "d".repeat(32),
      rawProjectionDigest: "d".repeat(32),
      adapterProjectionDigest: "d".repeat(32),
      projectionMatches: true,
    },
    resultSummary: {
      kind: "rime-result",
      resultDigest: "d".repeat(32),
      success: true,
      persistenceCompleted: false,
    },
    engineRaw: {
      actionKind: frozen.kind,
      compositionEpochId: "epoch-1",
      supersessionSubRunId: "subrun-1",
      rawActionSequence,
      rawResponseJson,
      rawResponseSha256: createHash("sha256").update(rawResponseJson, "utf8").digest("hex"),
    },
    presentationExpected: structuredClone(presentation),
    domObserved: structuredClone(presentation),
    beforeDomDigest: "0".repeat(32),
    adapterProjectionDigest: "1".repeat(32),
    presentationExpectedDigest: presentationSha,
    domObservedDigest: presentationSha,
    presentationDigest: presentationSha,
    afterDomDigest: presentationSha,
    ...(terminal ? {
      commitFingerprint: {
        exactCommitText: "你",
        textareaValue: "你",
        selectionStart: 1,
        selectionEnd: 1,
        visibleComposition: false,
      },
    } : {}),
  };
}

function validReceipt() {
  const timeline = expandScenarioExpectedTimeline("fair-peer-short");
  const eventStarts = [100, 101, 160, 161, 220, 221];
  const events = timeline.events.map((frozen, index) => {
    const eventTimestamp = eventStarts[index];
    const wireEventId = `web06-event-${String(frozen.eventSequenceId).padStart(8, "0")}`;
    return {
      ...frozen,
      wireEventSequenceId: frozen.eventSequenceId,
      wireEventId,
      wireIdentity: {
        eventSequenceId: frozen.eventSequenceId,
        eventId: wireEventId,
        type: frozen.type,
        key: frozen.key,
        code: frozen.code,
        classification: frozen.classification,
        reason: frozen.reason,
        timeStamp: eventTimestamp,
        eventDeliveredAt: eventTimestamp + 1,
      },
      eventTimestamp,
      normalizedEventAt: eventTimestamp,
      eventDeliveredAt: eventTimestamp + 1,
      pageInstanceId: PAGE_ID,
      requestedDriverDispatchAt: eventTimestamp - 1,
      actualDriverDispatchAt: eventTimestamp - 1,
      modifiers: [],
    };
  });
  const rawSequences = [["n"], ["n", "i"], ["n", "i", "space"]];
  const actions = timeline.actions.map((frozen, index) => actionReceipt(
    frozen,
    events[frozen.eventSequenceId - 1],
    rawSequences[index],
    index === 2 ? "committed" : "painted",
  ));
  for (const event of events) {
    event.wireLinkedActionIds = event.mappedActionIds.map((actionId) =>
      actions.find((action) => action.actionId === actionId)?.wireActionId);
  }
  return {
    metricContractVersion: WEB06_METRIC_CONTRACT_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_REGISTRY_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_PREDICATE_VERSION,
    scenarioId: "fair-peer-short",
    scenarioRunId: "fair-peer-short",
    schemaId: "luna_pinyin",
    mode: "BASE_FULL",
    source: {
      commit: COMMIT,
      tree: TREE,
      treeState: "clean",
      archiveSha256: HASH_B,
      artifactSha256: HASH_A,
      buildInfoSha256: HASH_B,
      artifactResponseGuardSha256: HASH_A,
      artifactResponseGuardSummarySha256: HASH_B,
      identityManifestSha256: HASH_A,
      runnerSourceManifestSha256: HASH_A,
      runnerToolingManifestSha256: HASH_B,
      runnerSourceObservationSha256: HASH_A,
      runnerSourcePostObservationSha256: HASH_A,
      observedEnvironmentSha256: HASH_B,
      collectorContractSha256: HASH_B,
      scenarioIdsSha256: HASH_A,
      selectedBranch: "NONE",
      disposition: "SOURCE_CURRENT_BASELINE",
      environmentManifestSha256: HASH_B,
      environmentId: "macbook-ac-60hz-clean-cache",
    },
    roundId: "round-1",
    attemptId: "attempt-1",
    measurementStarted: true,
    measurementCompleted: true,
    protocolWindow: {
      receiptWindowStartEventSequenceId: 1,
      receiptWindowStartActionSequenceId: 1,
    },
    eventClockProbe: { beforeDispatchAt: 0, eventTimestamp: 0.05, afterDispatchAt: 0.1 },
    calibration: {
      driver: {
        pre: Array.from({ length: 9 }, (_, index) => driverExchange(index, 0.1 + index * 0.01)),
        post: Array.from({ length: 9 }, (_, index) => driverExchange(1000 + index, 0.1 + index * 0.01)),
      },
      worker: {
        pre: Array.from({ length: 9 }, (_, index) => workerExchange(index, 0.1 + index * 0.01)),
        post: Array.from({ length: 9 }, (_, index) => workerExchange(1000 + index, 0.1 + index * 0.01)),
      },
    },
    events,
    auxiliaryEvents: [],
    actions,
    cadenceGaps: [{ stepId: "peer-short-2", nominalGapMs: 60, actualDriverGapMs: 60 }],
    idleFrameIntervalsMs: Array(120).fill(16.67),
    interactionFrameIntervalsMs: Array(15).fill(10),
    interactionFrameTimestamps: Array.from({ length: 16 }, (_, index) => 90 + index * 10),
    interactionFrameWindows: [{
      windowId: "web06-window-1",
      pageInstanceId: PAGE_ID,
      timestamps: Array.from({ length: 16 }, (_, index) => 90 + index * 10),
      intervalsMs: Array(15).fill(10),
    }],
    interactionWindows: [{
      windowId: "web06-window-1",
      pageInstanceId: PAGE_ID,
      startedAt: 90,
      endedAt: 240,
      startBoundaryRafAt: 90,
      endBoundaryRafAt: 240,
      preBoundaryFocusRecordedAt: 89.9,
      postBoundaryFocusRecordedAt: 240.1,
    }],
    idleControlWindows: [{
      controlId: "web06-idle-control-1",
      pageInstanceId: PAGE_ID,
      startedAt: 300,
      endedAt: 450,
    }],
    longTaskObserver: { supported: true, installedAt: 80 },
    longTasks: [],
    focusVisibilitySamples: [
      {
        recordedAt: 89.9,
        focused: true,
        visibilityState: "visible",
        role: "pre-boundary",
        windowId: "web06-window-1",
        pageInstanceId: PAGE_ID,
      },
      {
        recordedAt: 240.1,
        focused: true,
        visibilityState: "visible",
        role: "post-boundary",
        windowId: "web06-window-1",
        pageInstanceId: PAGE_ID,
      },
    ],
    assetsRequestedDuringWindow: [],
    measurementProtocolBlockers: [],
    sentinelOverflowCounts: { ...ZERO_SENTINEL_OVERFLOWS },
    pressureProofs: [],
    burstRecoveries: [],
  };
}

function validRawEnvelope(privateReceipt) {
  privateReceipt.roundId = `${privateReceipt.scenarioId}-round-1`;
  const runnerObservation = runnerSourceObservation({
    toolingManifestSha256: privateReceipt.source.runnerToolingManifestSha256,
  });
  const environmentObservation = observedEnvironmentObservation();
  privateReceipt.source.runnerSourceObservationSha256 = runnerObservation.observationSha256;
  privateReceipt.source.runnerSourcePostObservationSha256 = runnerObservation.observationSha256;
  privateReceipt.source.observedEnvironmentSha256 = environmentObservation.observationSha256;
  const guard = {
    version: "web06-artifact-response-guard-v1",
    rootDocumentPath: "index.html",
    entries: [
      { path: "build-info.json", bytes: 10, sha256: privateReceipt.source.buildInfoSha256 },
      { path: "public-artifact-manifest.json", bytes: 20, sha256: privateReceipt.source.artifactSha256 },
      { path: "index.html", bytes: 30, sha256: HASH_A },
    ],
  };
  const guardSha256 = createHash("sha256").update(JSON.stringify(guard), "utf8").digest("hex");
  const guardSummary = (stage) => {
    const summary = {
      version: "web06-artifact-response-guard-summary-v1",
      stage,
      expectedGuardSha256: guardSha256,
      expectedEntryCount: 3,
      observedResponseCount: 3,
      observedUniquePathCount: 3,
      duplicateResponseCount: 0,
      unknownPathCount: 0,
      verifiedPathCountsSha256: createHash("sha256").update(JSON.stringify(
        guard.entries.map((entry) => [entry.path, 1]),
      ), "utf8").digest("hex"),
      observedPathCounts: guard.entries.map((entry) => ({ path: entry.path, count: 1 })),
      failureCodes: [],
      pass: true,
    };
    return { ...summary, summarySha256: createHash("sha256").update(JSON.stringify(summary), "utf8").digest("hex") };
  };
  const sourceProofGuard = guardSummary("source-proof");
  const postGuard = guardSummary("post-measurement");
  privateReceipt.source.artifactResponseGuardSha256 = guardSha256;
  privateReceipt.source.artifactResponseGuardSummarySha256 = postGuard.summarySha256;
  const sentinelEvents = privateReceipt.events.map((event) => ({
    eventSequenceId: event.eventSequenceId,
    stepId: event.stepId,
    type: event.type,
    key: event.key,
    code: event.code,
    eventTimestamp: event.eventTimestamp,
    normalizedEventAt: event.normalizedEventAt,
    sentinelObservedAt: event.eventDeliveredAt,
    pageInstanceId: event.pageInstanceId,
    requestedDriverDispatchAt: event.requestedDriverDispatchAt,
    actualDriverDispatchAt: event.actualDriverDispatchAt,
  }));
  const sampleDoms = [
    commonDom("n", ["你", "擬", "尼", "泥", "呢", "妳"]),
    commonDom("ni", ["你", "擬", "尼", "泥", "呢", "妳"]),
    commonDom("", [], "你"),
  ];
  const sampleFacts = [
    ["peer-short-1", "covering", 1, "n", 108.5],
    ["peer-short-2", "covering", 3, "ni", 168.5],
    ["peer-short-commit", "terminal", 5, "", 228.5],
  ];
  const snapshots = sampleFacts.map(([stepId, _kind, _eventSequenceId, _input, observedAt], index) => ({
    stepId,
    pageInstanceId: PAGE_ID,
    observedAt,
    stableDoubleRaf: true,
    firstDomObserved: structuredClone(sampleDoms[index]),
    domObserved: structuredClone(sampleDoms[index]),
  }));
  const commonSamples = sampleFacts.map(([stepId, sampleKind, eventSequenceId, expectedInput, observedAt], index) => ({
    stepId,
    sampleKind,
    eventSequenceId,
    expectedInput,
    stressDeadline: false,
    outcome: sampleKind === "terminal" ? "terminal" : "painted",
    pageInstanceId: PAGE_ID,
    observedAt,
    stableDoubleRaf: true,
    firstDomObserved: structuredClone(sampleDoms[index]),
    domObserved: structuredClone(sampleDoms[index]),
    domFingerprintSha256: createHash("sha256").update(JSON.stringify(sampleDoms[index]), "utf8").digest("hex"),
  }));
  const pageSizeSetup = { measuredUiPageSize: "6" };
  const commonReceipt = {
    metricContractVersion: privateReceipt.metricContractVersion,
    scenarioRegistryVersion: privateReceipt.scenarioRegistryVersion,
    behaviorPredicateVersion: privateReceipt.behaviorPredicateVersion,
    scenarioId: privateReceipt.scenarioId,
    scenarioRunId: privateReceipt.scenarioRunId,
    schemaId: privateReceipt.schemaId,
    mode: privateReceipt.mode,
    source: structuredClone(privateReceipt.source),
    roundId: privateReceipt.roundId,
    attemptId: privateReceipt.attemptId,
    measurementStarted: true,
    measurementCompleted: true,
    candidatePageSize: 6,
    pageSizeSetup,
    eventClockProbe: structuredClone(privateReceipt.eventClockProbe),
    calibration: { driver: structuredClone(privateReceipt.calibration.driver) },
    events: sentinelEvents,
    auxiliaryEvents: [],
    unmatchedEvents: [],
    measurementProtocolBlockers: [],
    actions: [],
    commonSamples,
    cadenceGaps: structuredClone(privateReceipt.cadenceGaps),
    idleFrameIntervalsMs: structuredClone(privateReceipt.idleFrameIntervalsMs),
    interactionFrameIntervalsMs: structuredClone(privateReceipt.interactionFrameIntervalsMs),
    interactionFrameWindows: structuredClone(privateReceipt.interactionFrameWindows),
    interactionWindows: structuredClone(privateReceipt.interactionWindows),
    idleControlWindows: structuredClone(privateReceipt.idleControlWindows),
    longTaskObserver: structuredClone(privateReceipt.longTaskObserver),
    longTasks: structuredClone(privateReceipt.longTasks),
    focusVisibilitySamples: structuredClone(privateReceipt.focusVisibilitySamples),
    assetsRequestedDuringWindow: [],
    interactionFrameTimestamps: structuredClone(privateReceipt.interactionFrameTimestamps),
    sentinelOverflowCounts: structuredClone(privateReceipt.sentinelOverflowCounts),
  };
  const protocolActions = privateReceipt.actions.map((action) => ({
    identity: structuredClone(action.wireIdentity),
    returnedIdentity: structuredClone(action.returnedWireIdentity),
    name: action.kind,
    args: structuredClone(action.wireArgs),
    mainResponseReceivedAt: action.mainResponseReceivedAt,
    responseMappingStartedAt: action.responseMappingStartedAt,
    responseMappingFinishedAt: action.responseMappingFinishedAt,
    worker: {
      workerMessageReceivedAt: action.workerMessageReceivedAt,
      workerActionStartedAt: action.workerActionStartedAt,
      workerFinishedAt: action.workerFinishedAt,
      runtimeSpans: [
        { stage: "abi-call", startedAt: action.workerSpans.abi.start,
          finishedAt: action.workerSpans.abi.end, outcome: "success" },
        { stage: "response-byte-extraction", startedAt: action.workerSpans.responseExtract.start,
          finishedAt: action.workerSpans.responseExtract.end, outcome: "success" },
        { stage: "response-json-parse", startedAt: action.workerSpans.jsonParse.start,
          finishedAt: action.workerSpans.jsonParse.end, outcome: "success" },
      ],
      adapterSpans: [{ stage: "adapter-translation", startedAt: action.workerSpans.adapterTranslate.start,
        finishedAt: action.workerSpans.adapterTranslate.end, outcome: "success" }],
      persistenceSpans: [],
      collectorSpans: [],
      observerFailures: [],
      engineRaw: structuredClone(action.engineRawProof),
      engineRawJson: action.engineRaw.rawResponseJson,
      resultSummary: structuredClone(action.resultSummary),
    },
    presentation: {
      outcome: action.outcome,
      stateUpdateScheduledAt: action.stateUpdateScheduledAt,
      stateCommittedAt: action.stateCommittedAt,
      ...(action.paintObservedAt === undefined ? {} : { paintObservedAt: action.paintObservedAt }),
      ...(action.terminalObservedAt === undefined ? {} : { terminalObservedAt: action.terminalObservedAt }),
      presentationExpected: structuredClone(action.presentationExpected),
      domObserved: structuredClone(action.domObserved),
      beforePresentationDigest: action.beforeDomDigest,
      adapterProjectionDigest: action.adapterProjectionDigest,
      presentationExpectedDigest: action.presentationExpectedDigest,
      domObservedDigest: action.domObservedDigest,
      presentationDigest: action.presentationDigest,
    },
  }));
  const protocolWindow = privateReceipt.protocolWindow;
  const protocolWindowSegments = [{
    pageInstanceId: PAGE_ID,
    receiptWindowStartEventSequenceId: protocolWindow.receiptWindowStartEventSequenceId,
    receiptWindowStartActionSequenceId: protocolWindow.receiptWindowStartActionSequenceId,
  }];
  privateReceipt.protocolWindowSegments = structuredClone(protocolWindowSegments);
  return {
    version: "web06-raw-attempt-v1",
    target: {
      id: privateReceipt.mode,
      protocolMode: "full",
      sourceCommit: privateReceipt.source.commit,
      sourceTree: privateReceipt.source.tree,
      treeState: privateReceipt.source.treeState,
      archiveSha256: privateReceipt.source.archiveSha256,
      buildInfoSha256: privateReceipt.source.buildInfoSha256,
      artifactSha256: privateReceipt.source.artifactSha256,
      artifactResponseGuard: guard,
      artifactResponseGuardSha256: guardSha256,
      collectorContractSha256: privateReceipt.source.collectorContractSha256,
      pinnedSelectedBranch: privateReceipt.source.selectedBranch,
      pinnedDisposition: privateReceipt.source.disposition,
    },
    scenarioRunId: privateReceipt.scenarioRunId,
    scenarioId: privateReceipt.scenarioId,
    schemaId: privateReceipt.schemaId,
    attemptId: privateReceipt.attemptId,
    attemptNumber: 1,
    identityManifestSha256: privateReceipt.source.identityManifestSha256,
    runnerSourceManifestSha256: privateReceipt.source.runnerSourceManifestSha256,
    scenarioIdsSha256: privateReceipt.source.scenarioIdsSha256,
    environmentManifestSha256: privateReceipt.source.environmentManifestSha256,
    environmentId: privateReceipt.source.environmentId,
    measurementStarted: true,
    measurementCompleted: true,
    pageSizeSetup,
    preflight: { protocolAfterReset: { status: structuredClone(protocolWindow) } },
    drive: {
      initialDomObserved: { textareaValue: "" },
      driverEvents: privateReceipt.events.map((event) => ({
        requestedDriverDispatchAt: event.requestedDriverDispatchAt,
        actualDriverDispatchAt: event.actualDriverDispatchAt,
      })),
      argumentCommitments: {},
      cadenceGaps: structuredClone(privateReceipt.cadenceGaps),
      burstRecoveries: structuredClone(privateReceipt.burstRecoveries),
    },
    measurementEvidence: {
      eventClockProbe: structuredClone(privateReceipt.eventClockProbe),
      calibration: structuredClone(privateReceipt.calibration),
      idleFrameIntervalsMs: structuredClone(privateReceipt.idleFrameIntervalsMs),
    },
    completion: {
      timedOut: false,
      expectedActionCount: privateReceipt.actions.length,
      observedActionCount: privateReceipt.actions.length,
    },
    sentinel: {
      events: structuredClone(sentinelEvents),
      auxiliaryEvents: [],
      unmatchedEvents: [],
      snapshots,
      interactionWindows: structuredClone(privateReceipt.interactionWindows),
      idleControlWindows: structuredClone(privateReceipt.idleControlWindows),
      interactionFrameWindows: structuredClone(privateReceipt.interactionFrameWindows),
      interactionFrameTimestamps: structuredClone(privateReceipt.interactionFrameTimestamps),
      interactionFrameIntervalsMs: structuredClone(privateReceipt.interactionFrameIntervalsMs),
      longTasks: structuredClone(privateReceipt.longTasks),
      focusVisibilitySamples: structuredClone(privateReceipt.focusVisibilitySamples),
      assetsRequestedDuringWindow: structuredClone(privateReceipt.assetsRequestedDuringWindow),
      sentinelOverflowCounts: structuredClone(privateReceipt.sentinelOverflowCounts),
      callbackLedger: [],
      callbackLedgerCapacity: 8192,
      callbackLedgerOverflowCount: 0,
      sentinelAccountedCallbackCount: 0,
      sentinelCallbacksMs: [],
      unattributedInWindowCallbacksMs: [],
      sentinelTotalPerEventMs: Array(privateReceipt.events.length).fill(0),
      sentinelTotalPerWindowMs: Array(privateReceipt.interactionWindows.length).fill(0),
    },
    protocolExport: {
      header: { protocolVersion: "web06-private-v1", mode: "full" },
      status: { valid: true, queueDepth: 0, pendingFanoutActions: 0, pendingTerminalActions: 0,
        mainObserverCallbackCount: 0, mainObserverCallbackCapacity: 8192,
        mainObserverCallbackOverflowCount: 0 },
      invalidations: [],
      mainObserverCallbacks: [],
      mainObserverCallbacksMs: [],
      events: privateReceipt.events.map((event) => ({
        web06PageInstanceId: event.pageInstanceId,
        identity: structuredClone(event.wireIdentity),
        linkedActionIds: structuredClone(event.wireLinkedActionIds),
      })),
      actions: protocolActions.map((action) => ({ ...action, web06PageInstanceId: PAGE_ID })),
      protocolWindowSegments: structuredClone(protocolWindowSegments),
    },
    runnerSourceBefore: structuredClone(runnerObservation),
    attemptSourceBefore: structuredClone(runnerObservation),
    attemptSourceAfter: structuredClone(runnerObservation),
    observedEnvironment: environmentObservation,
    sourceProof: { artifactResponseGuard: sourceProofGuard },
    artifactResponseGuard: postGuard,
    commonReceipt,
    privateReceipt,
  };
}

test("completed raw decision shapes are rejected once before either semantic summary", () => {
  const baseline = validRawEnvelope(validReceipt());
  assert.deepEqual(validateCompletedRawDecisionShape(baseline), { pass: true, errors: [] });
  assert.deepEqual(independentlyValidateCompletedRawDecisionShape(baseline), { pass: true, errors: [] });
  const mutations = [
    ["missing required array", (copy) => { delete copy.commonReceipt.events; }],
    ["sparse event array", (copy) => { delete copy.commonReceipt.events[0]; }],
    ["null common action", (copy) => { copy.commonReceipt.actions = [null]; }],
    ["primitive private action", (copy) => { copy.privateReceipt.actions[0] = "action"; }],
    ["null cadence gap", (copy) => { copy.commonReceipt.cadenceGaps[0] = null; }],
    ["null interaction window", (copy) => { copy.commonReceipt.interactionWindows[0] = null; }],
    ["null sentinel callback", (copy) => { copy.sentinel.callbackLedger = [null]; }],
    ["sparse sentinel callback", (copy) => {
      copy.sentinel.callbackLedger = [{ startedAt: 1, finishedAt: 1.1, durationMs: 0.1 }];
      delete copy.sentinel.callbackLedger[0];
    }],
    ["null sentinel snapshot", (copy) => { copy.sentinel.snapshots[0] = null; }],
    ["sparse sentinel snapshot", (copy) => { delete copy.sentinel.snapshots[0]; }],
    ["null sentinel auxiliary event", (copy) => { copy.sentinel.auxiliaryEvents = [null]; }],
    ["null sentinel unmatched event", (copy) => { copy.sentinel.unmatchedEvents = [null]; }],
    ["null sentinel interaction window", (copy) => { copy.sentinel.interactionWindows[0] = null; }],
    ["null sentinel idle control", (copy) => { copy.sentinel.idleControlWindows[0] = null; }],
    ["null sentinel frame window", (copy) => { copy.sentinel.interactionFrameWindows[0] = null; }],
    ["null sentinel long task", (copy) => { copy.sentinel.longTasks = [null]; }],
    ["null sentinel focus row", (copy) => { copy.sentinel.focusVisibilitySamples[0] = null; }],
    ["null sentinel asset row", (copy) => { copy.sentinel.assetsRequestedDuringWindow = [null]; }],
    ["primitive sentinel event total", (copy) => { copy.sentinel.sentinelTotalPerEventMs[0] = "0"; }],
    ["null protocol action", (copy) => { copy.protocolExport.actions[0] = null; }],
    ["sparse protocol event", (copy) => { delete copy.protocolExport.events[0]; }],
    ["null protocol callback", (copy) => { copy.protocolExport.mainObserverCallbacks = [null]; }],
    ["null protocol window segment", (copy) => { copy.protocolExport.protocolWindowSegments[0] = null; }],
    ["primitive protocol invalidation", (copy) => { copy.protocolExport.invalidations = [null]; }],
    ["primitive protocol callback duration", (copy) => { copy.protocolExport.mainObserverCallbacksMs = ["0"]; }],
    ["null protocol worker span", (copy) => { copy.protocolExport.actions[0].worker.runtimeSpans[0] = null; }],
    ["primitive protocol observer failure", (copy) => {
      copy.protocolExport.actions[0].worker.observerFailures = [null];
    }],
    ["null common sample", (copy) => { copy.commonReceipt.commonSamples[0] = null; }],
    ["null private pressure proof", (copy) => { copy.privateReceipt.pressureProofs = [null]; }],
    ["null private burst recovery", (copy) => { copy.privateReceipt.burstRecoveries = [null]; }],
    ["null private long task", (copy) => { copy.privateReceipt.longTasks = [null]; }],
    ["sparse calibration", (copy) => { delete copy.privateReceipt.calibration.worker.pre[0]; }],
    ["nonboolean focus", (copy) => { copy.commonReceipt.focusVisibilitySamples[0].focused = "true"; }],
    ["mixed page identity", (copy) => { copy.privateReceipt.events[0].pageInstanceId = "single-page"; }],
    ["truthy primitive presentation", (copy) => { copy.privateReceipt.actions[0].presentationExpected = "state"; }],
    ["nonfinite event timestamp", (copy) => { copy.privateReceipt.events[0].eventTimestamp = Number.NaN; }],
    ["unknown scenario", (copy) => { copy.scenarioId = "unknown-scenario"; }],
    ["inherited scenario name", (copy) => { copy.scenarioId = "toString"; }],
    ["prototype scenario name", (copy) => { copy.scenarioId = "__proto__"; }],
    ["unknown scenario run", (copy) => { copy.scenarioRunId = "unknown-run"; }],
    ["inherited scenario run", (copy) => { copy.scenarioRunId = "toString"; }],
    ["prototype scenario run", (copy) => { copy.scenarioRunId = "__proto__"; }],
    ["constructor scenario run", (copy) => { copy.scenarioRunId = "constructor"; }],
    ["missing attempt number", (copy) => { delete copy.attemptNumber; }],
    ["mutually forged attempt ordinal", (copy) => {
      copy.attemptNumber = 7;
      copy.commonReceipt.roundId = `${copy.scenarioId}-round-7`;
      copy.privateReceipt.roundId = `${copy.scenarioId}-round-7`;
    }],
    ["mutually forged round", (copy) => {
      copy.commonReceipt.roundId = "forged-unrelated-round";
      copy.privateReceipt.roundId = "forged-unrelated-round";
    }],
    ["schema identity mismatch", (copy) => {
      copy.schemaId = copy.schemaId === "luna_pinyin" ? "jyut6ping3" : "luna_pinyin";
    }],
    ["missing private learned worker segment", (copy) => {
      copy.scenarioId = "learned-row";
      copy.scenarioRunId = "learned-row";
      copy.commonReceipt.scenarioId = "learned-row";
      copy.commonReceipt.scenarioRunId = "learned-row";
      copy.privateReceipt.scenarioId = "learned-row";
      copy.privateReceipt.scenarioRunId = "learned-row";
      copy.commonReceipt.calibrationSegments = {
        preReload: { driver: structuredClone(copy.commonReceipt.calibration.driver) },
        postReload: { driver: structuredClone(copy.commonReceipt.calibration.driver) },
      };
      copy.privateReceipt.calibrationSegments = {
        preReload: {
          driver: structuredClone(copy.privateReceipt.calibration.driver),
          worker: structuredClone(copy.privateReceipt.calibration.worker),
        },
        postReload: {
          driver: structuredClone(copy.privateReceipt.calibration.driver),
          worker: structuredClone(copy.privateReceipt.calibration.worker),
        },
      };
      delete copy.privateReceipt.calibrationSegments.postReload.worker;
    }],
  ];
  for (const [label, mutate] of mutations) {
    const changed = structuredClone(baseline);
    mutate(changed);
    const expected = { pass: false, errors: ["WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID"] };
    assert.deepEqual(validateCompletedRawDecisionShape(changed), expected, `${label}:runner`);
    assert.deepEqual(independentlyValidateCompletedRawDecisionShape(changed), expected, `${label}:independent`);
  }
});

test("completed sentinel callback evidence is structurally finite and semantically exact", () => {
  const exact = validRawEnvelope(validReceipt());
  const expectedIdentity = {
    targetId: "BASE_FULL",
    scenarioRunId: "fair-peer-short",
    scenarioId: "fair-peer-short",
    schemaId: "luna_pinyin",
    attemptId: "attempt-1",
    measurementStarted: true,
  };
  exact.sentinel.callbackLedger = [{
    kind: "dom-event",
    pageInstanceId: PAGE_ID,
    eventSequenceId: 1,
    startedAt: 100,
    finishedAt: 100.25,
    durationMs: 0.25,
  }];
  exact.sentinel.sentinelCallbacksMs = [0.25];
  exact.sentinel.unattributedInWindowCallbacksMs = [];
  exact.sentinel.sentinelTotalPerEventMs = [0.25, 0, 0, 0, 0, 0];
  exact.sentinel.sentinelTotalPerWindowMs = [0.25];
  exact.sentinel.sentinelAccountedCallbackCount = 1;
  assert.deepEqual(validateCompletedRawDecisionShape(exact), { pass: true, errors: [] });
  assert.deepEqual(independentlyValidateCompletedRawDecisionShape(exact), { pass: true, errors: [] });
  assert.deepEqual(sentinelLedgerIntegrityErrors(exact.sentinel), []);
  assert.deepEqual(independentlyValidateRawSentinelIntegrity(exact.sentinel), { pass: true, errors: [] });
  assert.deepEqual(independentlyAuditRawEnvelope(exact, expectedIdentity), { pass: true, errors: [] });

  const signed = structuredClone(exact);
  Object.assign(signed.sentinel.callbackLedger[0], {
    startedAt: 100,
    finishedAt: 99,
    durationMs: -1,
  });
  signed.sentinel.sentinelCallbacksMs = [-1];
  signed.sentinel.sentinelTotalPerEventMs = [-1, 0, 0, 0, 0, 0];
  signed.sentinel.sentinelTotalPerWindowMs = [-1];
  const completePageSizeSetup = {
    measuredUiPageSize: "6",
    uiTransition: [6, 7, 6],
    configuredPageSize: 6,
    sevenRows: 7,
    restoredControlValue: "6",
    realPreferencesControl: true,
  };
  signed.pageSizeSetup = structuredClone(completePageSizeSetup);
  signed.commonReceipt.pageSizeSetup = structuredClone(completePageSizeSetup);
  assert.ok(sentinelLedgerIntegrityErrors(signed.sentinel)
    .includes("SENTINEL_CALLBACK_TIMING_INVALID"));
  assert.deepEqual(independentlyValidateRawSentinelIntegrity(signed.sentinel),
    { pass: true, errors: [] }, "finite signed evidence remains reconcilable");
  assert.deepEqual(independentlyAuditRawEnvelope(signed, expectedIdentity),
    { pass: true, errors: [] }, "finite signed evidence remains source-bound");
  const signedProjection = independentlyProjectObserverModeRawEvidence(signed);
  assert.equal(signedProjection.callbackAttributionComplete, false);
  assert.equal(signedProjection.measurementValid, false);
  assert.equal(signedProjection.hardRedBindingValid, false);
  assert.equal(signedProjection.commonVerdict, "SETUP_INVALID");
  assert.equal(signedProjection.internalVerdict, "SETUP_INVALID");
  assert.equal(signedProjection.hardRedObserved, false);

  const signedBehavior = structuredClone(signed);
  signedBehavior.sentinel.snapshots[0].domObserved.candidates[0].text = "forged-candidate";
  signedBehavior.commonReceipt.commonSamples = resolveCommonSamples({
    scenarioId: signedBehavior.scenarioId,
    events: signedBehavior.sentinel.events,
    snapshots: signedBehavior.sentinel.snapshots,
  });
  assert.deepEqual(independentlyAuditRawEnvelope(signedBehavior, expectedIdentity),
    { pass: true, errors: [] });
  const signedBehaviorProjection = independentlyProjectObserverModeRawEvidence(signedBehavior);
  assert.equal(signedBehaviorProjection.callbackAttributionComplete, false);
  assert.equal(signedBehaviorProjection.measurementValid, false);
  assert.equal(signedBehaviorProjection.commonVerdict, "RED_BEHAVIOR");
  assert.equal(signedBehaviorProjection.internalVerdict, "RED_BEHAVIOR");
  assert.equal(signedBehaviorProjection.hardRedObserved, true);

  for (const [label, mutate, expectedError] of [
    ["count mismatch", (copy) => { copy.sentinel.sentinelAccountedCallbackCount = 0; },
      "SENTINEL_CALLBACK_COUNT_MISMATCH"],
    ["capacity exceeded", (copy) => { copy.sentinel.callbackLedgerCapacity = 0; },
      "SENTINEL_CALLBACK_CAPACITY_EXCEEDED"],
    ["positive overflow", (copy) => { copy.sentinel.callbackLedgerOverflowCount = 1; },
      "SENTINEL_CALLBACK_LEDGER_OVERFLOW"],
    ["callback duration projection", (copy) => { copy.sentinel.sentinelCallbacksMs[0] = 0.24; },
      "SENTINEL_CALLBACK_DURATION_PROJECTION_MISMATCH"],
    ["callback timing arithmetic", (copy) => { copy.sentinel.callbackLedger[0].finishedAt = 100.2; },
      "SENTINEL_CALLBACK_TIMING_INVALID"],
    ["owner missing", (copy) => { copy.sentinel.callbackLedger[0].eventSequenceId = 999; },
      "SENTINEL_CALLBACK_OWNER_MISSING"],
    ["event total underreported by one millisecond",
      (copy) => { copy.sentinel.sentinelTotalPerEventMs[0] -= 1; },
      "SENTINEL_EVENT_TOTAL_PROJECTION_MISMATCH"],
    ["window total mismatch", (copy) => { copy.sentinel.sentinelTotalPerWindowMs[0] = 0; },
      "SENTINEL_WINDOW_TOTAL_PROJECTION_MISMATCH"],
  ]) {
    const changed = structuredClone(exact);
    mutate(changed);
    assert.deepEqual(validateCompletedRawDecisionShape(changed), { pass: true, errors: [] },
      `${label}: finite semantic mismatch remains completed`);
    assert.deepEqual(independentlyValidateCompletedRawDecisionShape(changed), { pass: true, errors: [] },
      `${label}: independent finite semantic mismatch remains completed`);
    assert.ok(sentinelLedgerIntegrityErrors(changed.sentinel).includes(expectedError), label);
    assert.equal(independentlyValidateRawSentinelIntegrity(changed.sentinel).pass, false,
      `${label}: independent reconciliation`);
    assert.equal(independentlyAuditRawEnvelope(changed, expectedIdentity).pass, false,
      `${label}: completed raw audit`);
  }

  const unattributed = structuredClone(exact);
  delete unattributed.sentinel.callbackLedger[0].eventSequenceId;
  unattributed.sentinel.unattributedInWindowCallbacksMs = [0.25];
  unattributed.sentinel.sentinelTotalPerEventMs = Array(6).fill(0);
  assert.deepEqual(sentinelLedgerIntegrityErrors(unattributed.sentinel), []);
  assert.deepEqual(independentlyValidateRawSentinelIntegrity(unattributed.sentinel),
    { pass: true, errors: [] });

  const wrongPageWithForgedTotals = structuredClone(exact);
  wrongPageWithForgedTotals.sentinel.callbackLedger[0].pageInstanceId = "wrong-page";
  wrongPageWithForgedTotals.sentinel.sentinelTotalPerEventMs = Array(6).fill(0);
  wrongPageWithForgedTotals.sentinel.sentinelTotalPerWindowMs = [0];
  assert.ok(sentinelLedgerIntegrityErrors(wrongPageWithForgedTotals.sentinel)
    .includes("SENTINEL_CALLBACK_OWNER_MISSING"));
  assert.ok(independentlyValidateRawSentinelIntegrity(wrongPageWithForgedTotals.sentinel).errors
    .includes("raw-sentinel-callback-owner"));
  assert.equal(independentlyAuditRawEnvelope(wrongPageWithForgedTotals, expectedIdentity).pass, false);

  const forgedUnattributed = structuredClone(exact);
  forgedUnattributed.sentinel.unattributedInWindowCallbacksMs = [0.25];
  assert.ok(sentinelLedgerIntegrityErrors(forgedUnattributed.sentinel)
    .includes("SENTINEL_UNATTRIBUTED_CALLBACK_PROJECTION_MISMATCH"));
  assert.ok(independentlyValidateRawSentinelIntegrity(forgedUnattributed.sentinel).errors
    .includes("raw-sentinel-unattributed-projection"));
  assert.equal(independentlyAuditRawEnvelope(forgedUnattributed, expectedIdentity).pass, false);

  for (const [label, mutate] of [
    ["missing callback durations", (copy) => { delete copy.sentinel.sentinelCallbacksMs; }],
    ["missing unattributed callbacks", (copy) => { delete copy.sentinel.unattributedInWindowCallbacksMs; }],
    ["missing per-window totals", (copy) => { delete copy.sentinel.sentinelTotalPerWindowMs; }],
    ["sparse callback durations", (copy) => { delete copy.sentinel.sentinelCallbacksMs[0]; }],
    ["nonfinite callback durations", (copy) => { copy.sentinel.sentinelCallbacksMs[0] = Number.NaN; }],
    ["nonfinite per-window totals", (copy) => { copy.sentinel.sentinelTotalPerWindowMs[0] = Infinity; }],
  ]) {
    const changed = structuredClone(exact);
    mutate(changed);
    const expected = { pass: false, errors: ["WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID"] };
    assert.deepEqual(validateCompletedRawDecisionShape(changed), expected, `${label}:runner`);
    assert.deepEqual(independentlyValidateCompletedRawDecisionShape(changed), expected, `${label}:independent`);
  }
});

test("inherited action names remain completed behavior REDs instead of structural setup failures", () => {
  for (const kind of ["toString", "__proto__", "constructor"]) {
    const envelope = validRawEnvelope(validReceipt());
    envelope.privateReceipt.actions[0].kind = kind;
    assert.deepEqual(validateCompletedRawDecisionShape(envelope), { pass: true, errors: [] }, `${kind}:runner-gate`);
    assert.deepEqual(independentlyValidateCompletedRawDecisionShape(envelope),
      { pass: true, errors: [] }, `${kind}:independent-gate`);
    const runner = buildRoundEvidenceSummary(envelope.privateReceipt, { surface: "internal" });
    const independent = independentlyRecomputeRoundSummary(envelope.privateReceipt, "internal");
    assert.equal(runner.status, "RED", `${kind}:runner-status`);
    assert.equal(independent.status, "RED", `${kind}:independent-status`);
    assert.ok(runner.behaviorErrorCodes.includes("ACTION_UNCLASSIFIED:1"), `${kind}:runner-code`);
    assert.ok(independent.behaviorErrorCodes.includes("ACTION_UNCLASSIFIED:1"), `${kind}:independent-code`);
    assert.doesNotThrow(() => independentlyBindSummaryDiagnostics(runner, independent, `action-kind:${kind}`));
  }
});

test("calibration value failures remain semantic and cannot erase a completed behavior RED", () => {
  for (const invalidValue of [null, { m0: Number.NaN, w1: 1, w2: 2, m3: 3 }]) {
    const envelope = validRawEnvelope(validReceipt());
    envelope.privateReceipt.calibration.worker.pre[0] = structuredClone(invalidValue);
    assert.equal(validateCompletedRawDecisionShape(envelope).pass, true);
    const persistedEnvelope = JSON.parse(JSON.stringify(envelope));
    assert.equal(independentlyValidateCompletedRawDecisionShape(persistedEnvelope).pass, true);
    for (const summary of [
      buildRoundEvidenceSummary(envelope.privateReceipt, { surface: "internal" }),
      independentlyRecomputeRoundSummary(persistedEnvelope.privateReceipt, "internal"),
    ]) {
      assert.equal(summary.status, "SETUP_INVALID");
      assert.equal(summary.validRedObserved, false);
      assert.equal(summary.thresholdViolations.length, 0);
    }

    envelope.privateReceipt.events[0].stepId = "wrong-step";
    persistedEnvelope.privateReceipt.events[0].stepId = "wrong-step";
    const runner = buildRoundEvidenceSummary(envelope.privateReceipt, { surface: "internal" });
    const independent = independentlyRecomputeRoundSummary(persistedEnvelope.privateReceipt, "internal");
    assert.equal(runner.status, "RED");
    assert.equal(independent.status, "RED");
    assert.equal(runner.validRedObserved, true);
    assert.equal(independent.validRedObserved, true);
    assert.equal(runner.thresholdViolations.length, 0);
    assert.equal(independent.thresholdViolations.length, 0);
    assert.doesNotThrow(() =>
      independentlyBindSummaryDiagnostics(runner, independent, "calibration-plus-behavior"));
    const parsedProjection = {
      status: runner.status,
      cadence: runner.cadenceVerdict,
      setupErrors: runner.setupErrorCodes,
      behaviorErrors: runner.behaviorErrorCodes,
      thresholdViolations: runner.thresholdViolations,
      frameRed: runner.frame?.red === true,
      longTaskRed: runner.longTask?.red === true,
    };
    const facts = combinedAttemptFacts({
      internalParsed: parsedProjection,
      attemptId: "attempt-1",
      measurementStarted: true,
      measurementCompleted: true,
    });
    const series = evaluateAttemptSeries([facts]);
    assert.equal(series.retained[0].retainedLogicalRound, true);
    assert.equal(series.retained[0].validForLatencyFrame, false);
  }
});

test("a source-bound pointer-free raw receipt recomputes to PASS", () => {
  const receipt = validReceipt();
  const parsed = validateAndRecomputeReceipt(receipt);
  assert.equal(parsed.status, "PASS");
  assert.deepEqual(parsed.setupErrors, []);
  assert.deepEqual(parsed.behaviorErrors, []);

  assert.deepEqual(parsed.thresholdViolations, []);
  assert.equal(parsed.metrics.covering.length, 2);
  assert.equal(parsed.metrics.terminal.length, 1);
  assert.equal(parsed.metrics.covering[0].eventToCoveringPaintMs, 8.5);
  assert.ok(parsed.metrics.covering[0].preServiceWaitUpperBoundMs > 1.4);
});

function commonDom(renderedInput, candidateTexts, textareaValue = "") {
  return {
    input: renderedInput,
    renderedInput,
    logicalInputProjection: renderedInput.replaceAll(" ", ""),
    candidates: candidateTexts.map((text, index) => ({
      label: `${index + 1}.`,
      text,
      comment: "",
      source: "",
    })),
    pageShape: {
      previousDisabled: true,
      nextDisabled: true,
      highlightedIndex: candidateTexts.length ? 0 : -1,
      visibleCount: candidateTexts.length,
    },
    textareaValue,
    selectionStart: textareaValue.length,
    selectionEnd: textareaValue.length,
  };
}

test("minimal receipts require queue/presentation metadata but reject full raw diagnostics", () => {
  const minimal = validReceipt();
  minimal.mode = "BASE_MINIMAL";
  for (const action of minimal.actions) {
    delete action.workerSpans;
    delete action.persistenceRan;
    delete action.engineRaw;
    delete action.presentationExpected;
    delete action.domObserved;
    action.engineRawProof = {
      availability: "not-collected",
      action: action.kind,
      reason: "minimal-content-free",
    };
  }
  assert.equal(validateAndRecomputeReceipt(minimal).status, "PASS");

  minimal.actions[0].engineRaw = { actionKind: "processKey" };
  const leakedRaw = validateAndRecomputeReceipt(minimal);
  assert.equal(leakedRaw.status, "RED");
  assert.ok(leakedRaw.behaviorErrors.includes("MINIMAL_ENGINE_RAW_PRESENT:1"));

  delete minimal.actions[0].engineRaw;
  minimal.actions[0].workerSpans = { abi: { start: 103.4, end: 103.8 } };
  const leakedSpan = validateAndRecomputeReceipt(minimal);
  assert.equal(leakedSpan.status, "RED");
  assert.ok(leakedSpan.behaviorErrors.includes("MINIMAL_RAW_WORKER_SPAN_PRESENT:1"));
});

test("full receipts fail closed without raw spans or preprojection engine bytes", () => {
  const missing = validReceipt();
  delete missing.actions[0].engineRaw;
  delete missing.actions[0].workerSpans;
  const parsed = validateAndRecomputeReceipt(missing);
  assert.equal(parsed.status, "RED");
  assert.ok(parsed.behaviorErrors.includes("ENGINE_RAW_IDENTITY:1"));
  assert.ok(parsed.behaviorErrors.includes("WORKER_SPAN_DECLARATION_MISSING:1:abi"));
});

test("PRODUCT is accepted only through the common external surface", () => {
  const internal = validReceipt();
  internal.mode = "PRODUCT";
  assert.ok(validateAndRecomputeReceipt(internal).setupErrors.includes("SETUP_PRODUCT_INTERNAL_RECEIPT_FORBIDDEN"));

  const source = validReceipt();
  source.source.disposition = "DIAGNOSTIC";
  const common = {
    metricContractVersion: source.metricContractVersion,
    scenarioRegistryVersion: source.scenarioRegistryVersion,
    behaviorPredicateVersion: source.behaviorPredicateVersion,
    scenarioId: source.scenarioId,
    scenarioRunId: source.scenarioRunId,
    schemaId: source.schemaId,
    mode: "PRODUCT",
    source: source.source,
    roundId: source.roundId,
    attemptId: source.attemptId,
    measurementStarted: true,
    measurementCompleted: true,
    eventClockProbe: source.eventClockProbe,
    calibration: { driver: source.calibration.driver },
    candidatePageSize: 6,
    pageSizeSetup: {
      uiTransition: [6, 7, 6],
      configuredPageSize: 6,
      sevenRows: 7,
      restoredControlValue: "6",
      realPreferencesControl: true,
    },
    unmatchedEvents: [],
    measurementProtocolBlockers: [],
    sentinelOverflowCounts: { ...ZERO_SENTINEL_OVERFLOWS },
    events: source.events.map((event) => ({
      eventSequenceId: event.eventSequenceId,
      stepId: event.stepId,
      type: event.type,
      key: event.key,
      code: event.code,
      eventTimestamp: event.eventTimestamp,
      normalizedEventAt: event.normalizedEventAt,
      sentinelObservedAt: event.eventDeliveredAt,
      requestedDriverDispatchAt: event.requestedDriverDispatchAt,
      actualDriverDispatchAt: event.actualDriverDispatchAt,
    })),
    auxiliaryEvents: [],
    commonSamples: [
      {
        stepId: "peer-short-1",
        sampleKind: "covering",
        eventSequenceId: 1,
        outcome: "painted",
        observedAt: 108.5,
        stableDoubleRaf: true,
        domObserved: commonDom("n", ["你", "擬", "尼", "泥", "呢", "妳"]),
      },
      {
        stepId: "peer-short-2",
        sampleKind: "covering",
        eventSequenceId: 3,
        outcome: "painted",
        observedAt: 168.5,
        stableDoubleRaf: true,
        domObserved: commonDom("ni", ["你", "擬", "尼", "泥", "呢", "妳"]),
      },
      {
        stepId: "peer-short-commit",
        sampleKind: "terminal",
        eventSequenceId: 5,
        outcome: "terminal",
        observedAt: 228.5,
        stableDoubleRaf: true,
        domObserved: commonDom("", [], "你"),
      },
    ],
    cadenceGaps: source.cadenceGaps,
    idleFrameIntervalsMs: source.idleFrameIntervalsMs,
    interactionFrameIntervalsMs: source.interactionFrameIntervalsMs,
    interactionFrameTimestamps: source.interactionFrameTimestamps,
    interactionFrameWindows: source.interactionFrameWindows,
    interactionWindows: source.interactionWindows,
    idleControlWindows: source.idleControlWindows,
    longTaskObserver: source.longTaskObserver,
    longTasks: source.longTasks,
    focusVisibilitySamples: source.focusVisibilitySamples,
    assetsRequestedDuringWindow: [],
  };
  for (const sample of common.commonSamples) {
    sample.firstDomObserved = structuredClone(sample.domObserved);
    sample.domFingerprintSha256 = createHash("sha256")
      .update(JSON.stringify(sample.domObserved), "utf8")
      .digest("hex");
  }
  const parsed = validateCommonSurfaceReceipt(common);
  assert.equal(parsed.status, "PASS");
  assert.deepEqual(parsed.setupErrors, []);
  assert.deepEqual(parsed.behaviorErrors, []);

  const fiveAtFirstEligibleEndpoint = structuredClone(common);
  const first = fiveAtFirstEligibleEndpoint.commonSamples[0];
  first.domObserved.candidates = first.domObserved.candidates.slice(0, 5);
  first.domObserved.pageShape.visibleCount = 5;
  first.firstDomObserved = structuredClone(first.domObserved);
  first.domFingerprintSha256 = createHash("sha256")
    .update(JSON.stringify(first.domObserved), "utf8").digest("hex");
  assert.ok(validateCommonSurfaceReceipt(fiveAtFirstEligibleEndpoint).behaviorErrors
    .includes("COMMON_FIRST_ELIGIBLE_ENDPOINT_PAGE_SIZE_NOT_SIX"));

  const commonRounds = Array.from({ length: 5 }, (_, index) => ({
    ...structuredClone(common),
    roundId: `common-round-${index + 1}`,
    attemptId: `common-attempt-${index + 1}`,
  }));
  const commonPool = evaluateFiveRoundCommonPool(commonRounds);
  assert.equal(commonPool.pass, true);
  assert.equal(commonPool.pooledCovering.count, 10);
  assert.equal(commonPool.pooledTerminal.count, 5);
  const mixedSource = structuredClone(commonRounds);
  mixedSource[4].source.artifactSha256 = HASH_B;
  assert.throws(() => evaluateFiveRoundCommonPool(mixedSource), /POOL_SOURCE_OR_MODE_IDENTITY_MISMATCH/);

  const superseded = structuredClone(common);
  superseded.commonSamples[0].outcome = "superseded";
  superseded.commonSamples[0].supersededByStepId = "peer-short-2";
  superseded.commonSamples[0].observedAt = 165.8;
  superseded.commonSamples[1].observedAt = 165.8;
  const supersededParsed = validateCommonSurfaceReceipt(superseded);
  assert.equal(supersededParsed.status, "PASS");
  assert.ok(!supersededParsed.behaviorErrors.some((error) => error.includes("SUPERSESSION")));

  common.commonSamples[1].domObserved.input = "n";
  common.commonSamples[1].domObserved.logicalInputProjection = "n";
  assert.ok(validateCommonSurfaceReceipt(common).behaviorErrors.includes("COMMON_DOM_ENDPOINT:peer-short-2"));
});

test("raw userdb action bytes normalize only by an independently recomputed digest", () => {
  assert.throws(() => normalizeWireActionArgs("importUserdb", [""]), /RAW_BYTES_FORBIDDEN/);
  assert.deepEqual(normalizeWireActionArgs("processKey", ["{n}"]), ["{n}"]);
  assert.deepEqual(normalizeWireActionArgs(
    "importUserdb",
    ["<web06-redacted:userdb-text>"],
    { userdbTextSha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" },
  ), [`sha256:${"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}`]);
  assert.throws(
    () => normalizeWireActionArgs("importUserdb", ["<web06-redacted:userdb-text>"]),
    /COMMITMENT_MISSING/,
  );
  assert.deepEqual(normalizeWireActionArgs(
    "customizeValue",
    ["default", "web06/injected_error", "<web06-redacted:customize-value>"],
    { customizeValueSha256: "b5bea41b6c623f7c09f1bf24dcae58ebab3c0cdd90ad966bc43a45b44867e12b" },
  ), ["default", "web06/injected_error", "sha256:b5bea41b6c623f7c09f1bf24dcae58ebab3c0cdd90ad966bc43a45b44867e12b"]);
  assert.deepEqual(normalizeWireActionArgs(
    "customize",
    [{ pageSize: 6, dictionaryExclude: { kind: "web06-redacted:dictionary-exclude", count: 0 } }],
    { dictionaryExcludeSha256: "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945" },
  ), [{
    pageSize: 6,
    dictionaryExclude: {
      kind: "web06-redacted:dictionary-exclude",
      count: 0,
      sha256: "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945",
    },
  }]);
  assert.throws(
    () => normalizeWireActionArgs("customizeValue", ["default", "key", "raw-value"], { customizeValueSha256: HASH_A }),
    /COMMITMENT_INVALID/,
  );
  assert.throws(() => normalizeWireActionArgs("importUserdb", [42]), /IMPORT_USERDB_ARGS_INVALID/);
});

test("painted actions require a finite complete schedule/commit/paint chain", () => {
  const missing = validReceipt();
  delete missing.actions[0].stateCommittedAt;
  const parsed = validateAndRecomputeReceipt(missing);
  assert.equal(parsed.status, "RED");
  assert.ok(parsed.behaviorErrors.includes("PAINT_CHAIN_MISSING:1:stateCommittedAt"));
  assert.ok(parsed.behaviorErrors.includes("TIMELINE_RESIDUAL_NONFINITE:1"));
});

test("terminal ownership waits for Shift fan-out but not learned background refresh", () => {
  const shiftTimeline = expandScenarioExpectedTimeline("burst-action-map");
  const shiftActions = shiftTimeline.actions.filter((action) => action.stepId === "action-map-shift-tap");
  assert.equal(shiftActions.length, 12);
  assert.ok(shiftActions.every((action) => action.kind === "setOption" && action.classification === "stateful-barrier"));
  assert.equal(
    terminalOwnerSequenceId("burst-action-map", "action-map-shift-tap"),
    shiftActions.at(-1).sequenceId,
  );

  const learnedTimeline = expandScenarioExpectedTimeline("learned-row");
  const learnedActions = learnedTimeline.actions.filter((action) => action.stepId === "learned-commit");
  assert.deepEqual(learnedActions.map((action) => [action.kind, action.background === true]), [
    ["selectCandidate", false],
    ["getUserdbSnapshot", true],
  ]);
  assert.deepEqual(
    learnedActions.map((action) => ({
      originKind: action.originKind,
      eventSequenceId: action.eventSequenceId,
      causedByActionId: action.causedByActionId,
      causedByEventSequenceId: action.causedByEventSequenceId,
    })),
    [
      { originKind: "dom-event", eventSequenceId: learnedActions[0].eventSequenceId, causedByActionId: undefined, causedByEventSequenceId: undefined },
      { originKind: "background", eventSequenceId: undefined, causedByActionId: learnedActions[0].actionId, causedByEventSequenceId: learnedActions[0].eventSequenceId },
    ],
  );
  assert.equal(
    terminalOwnerSequenceId("learned-row", "learned-commit"),
    learnedActions[0].sequenceId,
  );
});

test("foreground, observer installation, and finite frame proof span the timed window", () => {
  const emptyFocus = validReceipt();
  emptyFocus.focusVisibilitySamples = [];
  assert.ok(validateAndRecomputeReceipt(emptyFocus).setupErrors.includes("SETUP_PAGE_NOT_FOREGROUND"));

  const lateObserver = validReceipt();
  lateObserver.longTaskObserver.installedAt = 91;
  assert.ok(validateAndRecomputeReceipt(lateObserver).setupErrors.includes("SETUP_LONG_TASK_OBSERVER_UNAVAILABLE"));

  const nonfiniteFrame = validReceipt();
  nonfiniteFrame.interactionFrameIntervalsMs[0] = Number.NaN;
  assert.ok(validateAndRecomputeReceipt(nonfiniteFrame).setupErrors.includes("SETUP_NONFINITE_INTERACTION_FRAME_INTERVAL"));
});

test("idle Long Tasks are retained in an equal-duration control and never left unclassified", () => {
  const receipt = validReceipt();
  receipt.longTasks.push({
    pageInstanceId: PAGE_ID,
    startTime: 320,
    durationMs: 55,
    overlapsInteractionWindow: false,
    overlapsIdleControl: true,
  });
  const parsed = validateAndRecomputeReceipt(receipt);
  assert.equal(parsed.status, "PASS");
  const summary = buildRoundEvidenceSummary(receipt, { surface: "internal" });
  assert.equal(summary.longTask.count, 1);
  assert.equal(summary.longTask.overlapCount, 0);
  assert.equal(summary.longTask.idleControlCount, 1);
  assert.equal(summary.longTask.interactionMinusIdleCount, -1);
  assert.equal(summary.longTask.interactionMinusIdleDurationMs, -55);
  const independentSummary = independentlyRecomputeRoundSummary(receipt, "internal");
  assert.equal(independentSummary.longTask.interactionMinusIdleCount, -1);
  assert.equal(independentSummary.longTask.interactionMinusIdleDurationMs, -55);

  const misclassified = structuredClone(receipt);
  misclassified.longTasks[0].overlapsIdleControl = false;
  assert.ok(validateAndRecomputeReceipt(misclassified).behaviorErrors
    .includes("LONG_TASK_IDLE_OVERLAP_MISMATCH:1"));

  const outside = structuredClone(receipt);
  outside.longTasks[0] = {
    pageInstanceId: PAGE_ID,
    startTime: 500,
    durationMs: 55,
    overlapsInteractionWindow: false,
    overlapsIdleControl: false,
  };
  assert.ok(validateAndRecomputeReceipt(outside).behaviorErrors
    .includes("LONG_TASK_OUTSIDE_DECLARED_CONTROLS:1"));
});

test("every bounded sentinel ledger overflows fail closed without silent shifting", () => {
  for (const field of Object.keys(ZERO_SENTINEL_OVERFLOWS)) {
    const receipt = validReceipt();
    receipt.sentinelOverflowCounts[field] = 1;
    const parsed = validateAndRecomputeReceipt(receipt);
    assert.equal(parsed.status, "SETUP_INVALID", field);
    assert.ok(parsed.setupErrors.includes(`SETUP_SENTINEL_LEDGER_OVERFLOW:${field}:1`), field);
  }
  const unknown = validReceipt();
  unknown.sentinelOverflowCounts.unknownLedger = 0;
  assert.ok(validateAndRecomputeReceipt(unknown).setupErrors
    .includes("SETUP_SENTINEL_OVERFLOW_COUNTERS_INVALID"));
});

test("five-round pooling reports and binds frame plus endpoint distributions", () => {
  const green = Array.from({ length: 5 }, (_, index) => {
    const receipt = validReceipt();
    receipt.roundId = `round-${index + 1}`;
    receipt.attemptId = `attempt-${index + 1}`;
    return receipt;
  });
  const pooled = evaluateFiveRoundPool(green);
  assert.equal(pooled.pass, true);
  assert.equal(pooled.pooledCovering.count, 10);
  assert.equal(pooled.pooledTerminal.count, 5);
  assert.equal(pooled.pooledFrames.count, 75);
  assert.equal(pooled.pooledLongTaskCount, 0);

  const red = structuredClone(green);
  red[0].interactionFrameIntervalsMs = [50, ...Array(14).fill(10)];
  red[0].interactionFrameTimestamps = red[0].interactionFrameIntervalsMs.reduce(
    (timestamps, interval) => [...timestamps, timestamps.at(-1) + interval],
    [90],
  );
  red[0].interactionFrameWindows[0].timestamps = structuredClone(red[0].interactionFrameTimestamps);
  red[0].interactionFrameWindows[0].intervalsMs = structuredClone(red[0].interactionFrameIntervalsMs);
  red[0].interactionWindows[0].endedAt = 280;
  red[0].interactionWindows[0].endBoundaryRafAt = 280;
  red[0].interactionWindows[0].postBoundaryFocusRecordedAt = 280.1;
  red[0].idleControlWindows[0].endedAt = 490;
  red[0].focusVisibilitySamples[1].recordedAt = 280.1;
  const redRound = buildRoundEvidenceSummary(red[0], { surface: "internal" });
  assert.equal(redRound.status, "RED");
  assert.equal(redRound.validRedObserved, true);
  assert.equal(redRound.decision.frameRed, true);
  const redPool = evaluateFiveRoundPool(red);
  assert.equal(redPool.pass, false);
  assert.ok(redPool.violations.includes("pooled-frame:max"));
  const redFive = buildFiveRoundEvidenceSummary(red, { surface: "internal" });
  const independentRedFive = independentlyRecomputeFiveRoundSummary(red, "internal");
  assert.equal(redFive.status, "RED");
  assert.equal(redFive.validRedObserved, true);
  assert.ok(redFive.poolViolations.includes("pooled-frame:max"));
  assert.equal(independentRedFive.status, "RED");
  assert.equal(independentRedFive.validRedObserved, true);
  assert.doesNotThrow(() => independentlyBindSummaryDiagnostics(
    redFive,
    independentRedFive,
    "five-round:pooled-frame",
  ));
  const driverTerminalRed = structuredClone(green);
  const terminal = driverTerminalRed[0].actions.at(-1);
  terminal.terminalObservedAt = terminal.driverDispatchAt + 68;
  driverTerminalRed[0].interactionFrameIntervalsMs = [
    ...Array(5).fill(14),
    ...Array(10).fill(13),
  ];
  driverTerminalRed[0].interactionFrameTimestamps =
    driverTerminalRed[0].interactionFrameIntervalsMs.reduce(
      (timestamps, interval) => [...timestamps, timestamps.at(-1) + interval],
      [90],
    );
  driverTerminalRed[0].interactionFrameWindows[0].timestamps =
    structuredClone(driverTerminalRed[0].interactionFrameTimestamps);
  driverTerminalRed[0].interactionFrameWindows[0].intervalsMs =
    structuredClone(driverTerminalRed[0].interactionFrameIntervalsMs);
  driverTerminalRed[0].interactionWindows[0].endedAt = 290;
  driverTerminalRed[0].interactionWindows[0].endBoundaryRafAt = 290;
  driverTerminalRed[0].interactionWindows[0].postBoundaryFocusRecordedAt = 290.1;
  driverTerminalRed[0].idleControlWindows[0].endedAt = 500;
  driverTerminalRed[0].focusVisibilitySamples[1].recordedAt = 290.1;
  const driverTerminalParsed = validateAndRecomputeReceipt(driverTerminalRed[0]);
  assert.equal(driverTerminalParsed.status, "RED");
  const driverTerminalFive = buildFiveRoundEvidenceSummary(driverTerminalRed, { surface: "internal" });
  const independentDriverTerminalFive = independentlyRecomputeFiveRoundSummary(driverTerminalRed, "internal");
  assert.equal(driverTerminalFive.status, "RED");
  assert.equal(driverTerminalFive.validRedObserved, true);
  assert.ok(driverTerminalFive.poolViolations.some((code) => code.startsWith("pooled-driver-terminal")));
  assert.doesNotThrow(() => independentlyBindSummaryDiagnostics(
    driverTerminalFive,
    independentDriverTerminalFive,
    "five-round:driver-terminal",
  ));
});

test("missing, duplicate, and reordered IDs are non-retryable behavior REDs", () => {
  const missing = validReceipt();
  missing.events.splice(2, 1);
  const missingParsed = validateAndRecomputeReceipt(missing);
  assert.equal(missingParsed.status, "RED");
  assert.ok(missingParsed.behaviorErrors.some((error) => error.includes("EVENT_MISSING_ID")));

  const missingActionCompletion = validReceipt();
  missingActionCompletion.measurementProtocolBlockers.push("PROTOCOL_COMPLETION_TIMEOUT");
  missingActionCompletion.actions.splice(1, 1);
  const completionParsed = validateAndRecomputeReceipt(missingActionCompletion);
  assert.equal(completionParsed.status, "RED");
  assert.ok(completionParsed.setupErrors
    .includes("SETUP_MEASUREMENT_PROTOCOL:PROTOCOL_COMPLETION_TIMEOUT"));
  assert.ok(completionParsed.behaviorErrors.some((error) => error.startsWith("ACTION_COUNT:")));

  const timeoutOnly = validReceipt();
  timeoutOnly.measurementProtocolBlockers.push("PROTOCOL_COMPLETION_TIMEOUT");
  const timeoutOnlyParsed = validateAndRecomputeReceipt(timeoutOnly);
  assert.equal(timeoutOnlyParsed.status, "SETUP_INVALID");
  assert.equal(timeoutOnlyParsed.validRedObserved, false);
  assert.equal(timeoutOnlyParsed.retryEligible, true);
  assert.deepEqual(timeoutOnlyParsed.behaviorErrors, []);

  const duplicate = validReceipt();
  duplicate.actions[1].sequenceId = 1;
  const duplicateParsed = validateAndRecomputeReceipt(duplicate);
  assert.equal(duplicateParsed.status, "RED");
  assert.ok(duplicateParsed.behaviorErrors.includes("ACTION_DUPLICATE_ID"));

  const reordered = validReceipt();
  [reordered.actions[0], reordered.actions[1]] = [reordered.actions[1], reordered.actions[0]];
  const reorderedParsed = validateAndRecomputeReceipt(reordered);
  assert.equal(reorderedParsed.status, "RED");
  assert.ok(reorderedParsed.behaviorErrors.includes("ACTION_REORDERED_ID"));

  const wrongCode = validReceipt();
  wrongCode.events[4].code = "Key1";
  const wrongCodeParsed = validateAndRecomputeReceipt(wrongCode);
  assert.equal(wrongCodeParsed.status, "RED");
  assert.ok(wrongCodeParsed.behaviorErrors.includes("EVENT_IDENTITY:5"));

  const wireOffset = validReceipt();
  wireOffset.actions[1].wireSequenceId += 1;
  const wireOffsetParsed = validateAndRecomputeReceipt(wireOffset);
  assert.equal(wireOffsetParsed.status, "RED");
  assert.ok(wireOffsetParsed.behaviorErrors.includes("WIRE_ACTION_OFFSET:2"));

  const returnedEnvelope = validReceipt();
  returnedEnvelope.actions[0].returnedWireIdentity.workerSentAt += 0.01;
  const returnedParsed = validateAndRecomputeReceipt(returnedEnvelope);
  assert.equal(returnedParsed.status, "RED");
  assert.ok(returnedParsed.behaviorErrors.includes("WIRE_RETURNED_IDENTITY:1"));
});

test("raw threshold arithmetic rejects an unrounded 67.000001 ms sample", () => {
  const receipt = validReceipt();
  const first = receipt.actions[0];
  const event = receipt.events[first.eventSequenceId - 1];
  first.paintObservedAt = event.normalizedEventAt + 67.000001;
  const parsed = validateAndRecomputeReceipt(receipt);
  assert.equal(parsed.status, "RED");
  assert.ok(parsed.thresholdViolations.includes("covering-max"));
});

test("protocol callback ceilings alone are threshold REDs while protocol defects remain setup-invalid", () => {
  for (const blocker of ["MAIN_OBSERVER_CALLBACK_CEILING", "COLLECTOR_CALLBACK_CEILING"]) {
    const receipt = validReceipt();
    receipt.measurementProtocolBlockers = [blocker];
    const parsed = validateAndRecomputeReceipt(receipt);
    assert.equal(parsed.status, "RED", blocker);
    assert.equal(parsed.validRedObserved, true, blocker);
    assert.equal(parsed.retryEligible, false, blocker);
    assert.deepEqual(parsed.setupErrors, [], blocker);
    assert.deepEqual(parsed.thresholdViolations, [`measurement-protocol:${blocker}`], blocker);
  }

  const malformed = validReceipt();
  malformed.measurementProtocolBlockers = ["MAIN_OBSERVER_CALLBACK_LEDGER_INVALID"];
  const malformedParsed = validateAndRecomputeReceipt(malformed);
  assert.equal(malformedParsed.status, "SETUP_INVALID");
  assert.equal(malformedParsed.validRedObserved, false);
  assert.equal(malformedParsed.retryEligible, true);
  assert.deepEqual(malformedParsed.thresholdViolations, []);
});

test("setup invalidity gates numeric REDs but cannot erase independent behavior REDs", () => {
  const receipt = validReceipt();
  receipt.source.treeState = "dirty";
  receipt.actions[0].paintObservedAt = receipt.events[0].normalizedEventAt + 67.000001;
  const parsed = validateAndRecomputeReceipt(receipt);
  assert.equal(parsed.status, "SETUP_INVALID");
  assert.equal(parsed.validRedObserved, false);
  assert.equal(parsed.retryEligible, true);
  assert.equal(parsed.qualifiers.setupInvalid, true);
  assert.equal(parsed.qualifiers.instrumentationAttributionInvalid, false);
  assert.ok(parsed.setupErrors.includes("SETUP_SOURCE_TREE_NOT_CLEAN"));
  assert.ok(parsed.thresholdViolations.includes("covering-max"));

  const behavior = validReceipt();
  behavior.source.treeState = "dirty";
  behavior.actions[0].outcome = "forged";
  const behaviorParsed = validateAndRecomputeReceipt(behavior);
  assert.equal(behaviorParsed.status, "RED");
  assert.equal(behaviorParsed.validRedObserved, true);
  assert.equal(behaviorParsed.retryEligible, false);
  assert.ok(behaviorParsed.behaviorErrors.length > 0);
});

test("too-short cadence retains numeric and jank observations without counting a measured red", () => {
  const latency = validReceipt();
  latency.cadenceGaps[0].actualDriverGapMs = 20;
  latency.actions[0].paintObservedAt = latency.events[0].normalizedEventAt + 67.000001;
  const latencyParsed = validateAndRecomputeReceipt(latency);
  assert.equal(latencyParsed.status, "NO_VERDICT_INVALID_CADENCE");
  assert.equal(latencyParsed.validRedObserved, false);
  assert.equal(latencyParsed.retryEligible, true);
  assert.equal(latencyParsed.qualifiers.nonbindingThresholdObservationRed, true);
  assert.ok(latencyParsed.thresholdViolations.includes("covering-max"));

  const jank = validReceipt();
  jank.cadenceGaps[0].actualDriverGapMs = 20;
  jank.longTasks.push({
    pageInstanceId: PAGE_ID,
    startTime: 120,
    durationMs: 55,
    overlapsInteractionWindow: true,
    overlapsIdleControl: false,
  });
  const jankParsed = validateAndRecomputeReceipt(jank);
  assert.equal(jankParsed.status, "NO_VERDICT_INVALID_CADENCE");
  assert.equal(jankParsed.qualifiers.nonbindingLongTaskObservationRed, true);

  const behavior = validReceipt();
  behavior.cadenceGaps[0].actualDriverGapMs = 20;
  behavior.actions.reverse();
  assert.equal(validateAndRecomputeReceipt(behavior).status, "RED_BEHAVIOR");
});

test("too-long cadence preserves threshold, jank, and behavior REDs but not clean observations", () => {
  const numeric = validReceipt();
  numeric.cadenceGaps[0].actualDriverGapMs = 200;
  numeric.actions[0].paintObservedAt = numeric.events[0].normalizedEventAt + 67.000001;
  for (const parsed of [
    validateAndRecomputeReceipt(numeric),
    independentlyRecomputeRoundSummary(numeric, "internal"),
  ]) {
    assert.equal(parsed.status, "RED");
    assert.equal(parsed.validRedObserved, true);
    assert.equal(parsed.retryEligible, false);
  }

  const jank = validReceipt();
  jank.cadenceGaps[0].actualDriverGapMs = 200;
  jank.longTasks.push({
    pageInstanceId: PAGE_ID,
    startTime: 120,
    durationMs: 55,
    overlapsInteractionWindow: true,
    overlapsIdleControl: false,
  });
  assert.equal(validateAndRecomputeReceipt(jank).status, "RED");
  assert.equal(independentlyRecomputeRoundSummary(jank, "internal").status, "RED");

  const clean = validReceipt();
  clean.cadenceGaps[0].actualDriverGapMs = 200;
  const cleanParsed = validateAndRecomputeReceipt(clean);
  assert.equal(cleanParsed.status, "NO_VERDICT_INVALID_CADENCE");
  assert.equal(cleanParsed.validRedObserved, false);
  assert.equal(cleanParsed.retryEligible, true);

  const behavior = validReceipt();
  behavior.cadenceGaps[0].actualDriverGapMs = 200;
  behavior.actions.reverse();
  assert.equal(validateAndRecomputeReceipt(behavior).status, "RED");

  const setupInvalid = structuredClone(numeric);
  setupInvalid.source.treeState = "dirty";
  const setupParsed = validateAndRecomputeReceipt(setupInvalid);
  assert.equal(setupParsed.status, "SETUP_INVALID");
  assert.equal(setupParsed.validRedObserved, false);
  assert.equal(setupParsed.retryEligible, true);
});

test("pooled cardinality and schema identity fail closed without a smaller denominator", () => {
  const rounds = Array.from({ length: 5 }, (_, index) => ({
    ...structuredClone(validReceipt()),
    roundId: `round-${index + 1}`,
    attemptId: `attempt-${index + 1}`,
  }));
  const crossSchema = structuredClone(rounds);
  crossSchema[4].schemaId = "jyut6ping3";
  assert.throws(() => evaluateFiveRoundPool(crossSchema), /POOL_SOURCE_OR_MODE_IDENTITY_MISMATCH/);

  const commonRounds = rounds.map((round) => {
    const common = {
      ...structuredClone(round),
      mode: "PRODUCT",
      actions: [],
      commonSamples: [],
      unmatchedEvents: [],
      candidatePageSize: 6,
      pageSizeSetup: {
        uiTransition: [6, 7, 6],
        configuredPageSize: 6,
        sevenRows: 7,
        restoredControlValue: "6",
        realPreferencesControl: true,
      },
      calibration: { driver: structuredClone(round.calibration.driver) },
    };
    return common;
  });
  const pool = evaluateFiveRoundCommonPool(commonRounds);
  assert.equal(pool.pass, false);
  assert.ok(pool.violations.includes("pooled-common-covering-count:0!=10"));
  assert.ok(pool.violations.includes("pooled-common-terminal-count:0!=5"));
});

test("cadence identity is frozen and cannot be relabelled after capture", () => {
  const receipt = validReceipt();
  receipt.cadenceGaps[0].stepId = "wrong-step";
  const parsed = validateAndRecomputeReceipt(receipt);
  assert.equal(parsed.status, "RED");
  assert.ok(parsed.behaviorErrors.includes("CADENCE_GAP_IDENTITY:1"));
});

test("supersession accepts append-only n to ni and rejects correction/barrier links", () => {
  const appendOnly = [
    {
      sequenceId: 1,
      classification: "native-key",
      supersedable: true,
      outcome: "superseded",
      supersededBySequenceId: 2,
      compositionEpochId: "e1",
      supersessionSubRunId: "s1",
      engineRaw: { logicalInput: "n", rawActionSequence: ["n"] },
    },
    {
      sequenceId: 2,
      classification: "native-key",
      supersedable: true,
      outcome: "painted",
      paintObservedAt: 60,
      compositionEpochId: "e1",
      supersessionSubRunId: "s1",
      engineRaw: { logicalInput: "ni", rawActionSequence: ["n", "i"] },
    },
  ];
  assert.deepEqual(validateSupersessionGraph(appendOnly), { pass: true, errors: [] });

  const nonPrefix = structuredClone(appendOnly);
  nonPrefix[0].engineRaw = { logicalInput: "ni", rawActionSequence: ["n", "i"] };
  nonPrefix[1].engineRaw = { logicalInput: "na", rawActionSequence: ["n", "a"] };
  assert.equal(validateSupersessionGraph(nonPrefix).pass, false);
  assert.ok(validateSupersessionGraph(nonPrefix).errors.some((error) => error.includes("NON_PREFIX")));

  const intoBackspace = structuredClone(appendOnly);
  intoBackspace[1] = {
    ...intoBackspace[1],
    classification: "stateful-barrier",
    supersedable: false,
    engineRaw: { logicalInput: "", rawActionSequence: ["n", "BackSpace"] },
  };
  const barrierResult = validateSupersessionGraph(intoBackspace);
  assert.equal(barrierResult.pass, false);
  assert.ok(barrierResult.errors.some((error) => error.includes("TARGET_IS_BARRIER")));
});

test("independent supersession reconstruction proves every prefix, boundary, target, and lag invariant", () => {
  const base = () => [
    {
      sequenceId: 1,
      classification: "native-key",
      supersedable: true,
      outcome: "superseded",
      supersededBySequenceId: 2,
      compositionEpochId: "epoch-1",
      supersessionSubRunId: "subrun-1",
      rawActionSequence: ["n"],
      logicalInput: "n",
    },
    {
      sequenceId: 2,
      classification: "native-key",
      supersedable: true,
      outcome: "painted",
      paintObservedAt: 60,
      compositionEpochId: "epoch-1",
      supersessionSubRunId: "subrun-1",
      rawActionSequence: ["n", "i"],
      logicalInput: "ni",
    },
  ];
  assert.deepEqual(independentlyValidateSupersessionGraph(base()), { pass: true, errors: [] });

  const intermediate = (sequenceId, overrides = {}) => ({
    sequenceId,
    classification: "native-key",
    supersedable: true,
    outcome: "processed-no-visual-change",
    compositionEpochId: "epoch-1",
    supersessionSubRunId: "subrun-1",
    rawActionSequence: ["n", String(sequenceId)],
    logicalInput: `n${sequenceId}`,
    ...overrides,
  });
  const cases = [
    ["orphan/backward", (actions) => { actions[0].supersededBySequenceId = 1; },
      "SUPERSESSION_ORPHAN_OR_BACKWARD:1"],
    ["composition boundary", (actions) => { actions[1].compositionEpochId = "epoch-2"; },
      "SUPERSESSION_CROSSES_BOUNDARY:1"],
    ["unpainted target", (actions) => { actions[1].outcome = "processed-no-visual-change"; },
      "SUPERSESSION_TARGET_NOT_PAINTED:1"],
    ["barrier target", (actions) => {
      actions[1].classification = "stateful-barrier";
      actions[1].supersedable = false;
    }, "SUPERSESSION_TARGET_IS_BARRIER:1"],
    ["non-prefix target", (actions) => {
      actions[1].rawActionSequence = ["x"];
      actions[1].logicalInput = "x";
    }, "SUPERSESSION_NON_PREFIX:1"],
    ["intervening barrier", (actions) => {
      actions[0].supersededBySequenceId = 3;
      actions[1].sequenceId = 3;
      actions.splice(1, 0, intermediate(2, { classification: "stateful-barrier", supersedable: false }));
    }, "SUPERSESSION_SPANS_BARRIER_OR_FAILURE:1"],
    ["earlier covering paint", (actions) => {
      actions[0].supersededBySequenceId = 3;
      actions[1].sequenceId = 3;
      actions.splice(1, 0, intermediate(2, {
        outcome: "painted",
        paintObservedAt: 55,
        rawActionSequence: ["n", "a"],
        logicalInput: "na",
      }));
    }, "SUPERSESSION_NOT_EARLIEST_COVERING_PAINT:1"],
    ["sequence lag", (actions) => {
      actions[0].supersededBySequenceId = 4;
      actions[1].sequenceId = 4;
      actions.splice(1, 0, intermediate(2), intermediate(3));
    }, "SUPERSESSION_LAG:1:3"],
  ];
  for (const [label, mutate, expected] of cases) {
    const actions = base();
    mutate(actions);
    assert.ok(independentlyValidateSupersessionGraph(actions).errors.includes(expected), label);
  }
});

function provedPeerManifest() {
  const compiledHashes = {
    table: HASH_A,
    prism: HASH_A,
    reverse: HASH_A,
    "data-model": HASH_A,
    runtime: HASH_A,
  };
  const reproducible = {
    sourceTreeState: "clean",
    generatedManifestSha256: HASH_A,
    completeArtifactManifestSha256: HASH_B,
    packageManager: {
      name: "npm",
      version: "11.4.2",
      lockSha256: HASH_A,
      integrityManifestSha256: HASH_B,
    },
    toolchain: {
      nodeVersion: "v22.16.0",
      emscriptenVersion: "4.0.23",
      emscriptenCommit: COMMIT,
      compilerVersion: "clang 21",
    },
    resolvedRecipes: [{
      id: "luna-package",
      repository: "https://example.invalid/luna",
      commit: COMMIT,
      logicalBytesSha256: HASH_A,
    }],
  };
  return {
    version: "web06-peer-data-v1",
    yune: {
      ...reproducible,
      repositoryCommit: COMMIT,
      artifactSha256: HASH_A,
      buildCommand: "sealed-yune-build-v1",
      compiledHashes: { ...compiledHashes },
    },
    peer: {
      ...reproducible,
      repositoryCommit: "fedcba9876543210fedcba9876543210fedcba98",
      artifactSha256: HASH_B,
      buildCommand: "sealed-my-rime-build-v1",
      compiledHashes: { ...compiledHashes, runtime: HASH_B },
    },
    logicalInputs: PEER_LOGICAL_INPUT_IDS.map((id) => id === "grammar-model"
      ? { id, yuneSha256: "none", peerSha256: "none", explicitNone: true }
      : { id, yuneSha256: HASH_A, peerSha256: HASH_A }),
    effectiveConfiguration: { yuneSha256: HASH_B, peerSha256: HASH_B },
    freshEmptyUserdb: true,
    sameEndpointObserver: true,
  };
}

test("packageAlignment is row-level and essay negative control refuses a ratio", () => {
  const manifest = provedPeerManifest();
  assert.deepEqual(evaluatePackageAlignment(manifest), { packageAlignment: "PROVED", reasons: [] });
  const ratio = computeBindingPeerRatio({ yuneMs: [10, 11, 12, 13, 14], peerMs: [11, 12, 13, 14, 15], manifest });
  assert.equal(ratio.packageAlignment, "PROVED");
  assert.equal(ratio.matchesOrBeats, true);

  const negative = structuredClone(manifest);
  negative.logicalInputs.find((item) => item.id === "essay").peerSha256 = HASH_B;
  const alignment = evaluatePackageAlignment(negative);
  assert.equal(alignment.packageAlignment, "DATA_CONFOUNDED");
  assert.ok(alignment.reasons.includes("logical-input-different:essay"));
  assert.throws(
    () => computeBindingPeerRatio({ yuneMs: [10, 11, 12, 13, 14], peerMs: [11, 12, 13, 14, 15], manifest: negative }),
    /BINDING_RATIO_REFUSED/,
  );
});

test("collection modes preserve public shapes, calls, JSON, and pointer ownership", () => {
  const mode = () => ({
    publicActionResultShapeSha256: HASH_A,
    publicErrorShapeSha256: HASH_A,
    nativeBindingCallSequenceSha256: HASH_B,
    decodedJsonSha256: HASH_B,
    nativeBindingCallCount: 3,
    responsePointerAudit: {
      nonzeroResponseCount: 3,
      freeCount: 3,
      duplicateFreeCount: 0,
      zeroFreeCount: 0,
    },
    collectorExceptionIsolation: {
      publicResultUnchanged: true,
      publicErrorUnchanged: true,
      measurementFailsClosed: true,
    },
    publicTrace: { actionSequenceDigest: HASH_A },
  });
  const modes = { product: mode(), minimal: mode(), full: mode() };
  assert.deepEqual(evaluateCollectionEquivalence(modes), { pass: true, violations: [] });
  modes.full.responsePointerAudit.duplicateFreeCount = 1;
  modes.minimal.decodedJsonSha256 = HASH_A;
  const red = evaluateCollectionEquivalence(modes);
  assert.equal(red.pass, false);
  assert.ok(red.violations.includes("full:pointer-free-ownership"));
  assert.ok(red.violations.includes("decodedJsonSha256:different"));
});

test("privacy contract rejects pointers, secrets, and local absolute paths", () => {
  assert.equal(validatePointerFreePrivacy({ digest: HASH_A, values: [1, 2] }).pass, true);
  const pointer = validatePointerFreePrivacy({ responsePointer: "0x1234abcd" });
  assert.equal(pointer.pass, false);
  assert.ok(pointer.errors.some((error) => error.includes("PRIVACY_KEY")));
  assert.equal(validatePointerFreePrivacy({ note: "/Users/alice/private/raw.json" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ note: "/private/tmp/web06/raw.json" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ note: "/var/folders/ab/cd/T/raw.json" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ note: "/tmp/web06/raw.json" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ note: "/secret.txt" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ servedPath: "/assets/yune-web.js" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ note: "/Volumes/build/raw.json" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ note: "/opt/web06/raw.json" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ note: "D:\\work\\raw.json" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ note: "\\\\server\\share\\raw.json" }).pass, false);
  assert.equal(validatePointerFreePrivacy({ servedUrl: "https://example.test/tmp/yune-web.js" }).pass, true);
  assert.equal(validatePointerFreePrivacy({ authorization: "Bearer secret" }).pass, false);
  const assertPrivate = (secret) => {
    const producer = validatePointerFreePrivacy({ note: secret });
    assert.equal(producer.pass, false, secret);
    const independentlyChecked = independentlyValidatePointerFreePrivacy({ note: secret });
    assert.equal(independentlyChecked.pass, false, secret);
    assert.ok(independentlyChecked.errors.some((error) => error.includes("PUBLIC_PRIVACY_VALUE")), secret);
  };
  const sensitiveValues = [
    "failure(/Users/alice/private/raw.json)",
    "error(path:/private/tmp/web06/raw.json)",
    "failure(/opt/web06/raw.json)",
    "file:///Users/alice/private/raw.json",
    "request failed: Bearer eyJhbGciOi.test-token",
    "request failed: sk-proj-test_secret",
    "https://alice:password@example.test/evidence",
  ];
  for (const secret of sensitiveValues) {
    assertPrivate(secret);
    assertPrivate(encodeURIComponent(secret));
    assertPrivate(encodeURIComponent(encodeURIComponent(secret)));
  }
  const encodeEveryByte = (value) => [...Buffer.from(value, "utf8")]
    .map((byte) => `%${byte.toString(16).padStart(2, "0")}`)
    .join("");
  const nestEncoding = (value, additionalPasses) =>
    Array.from({ length: additionalPasses }, () => undefined)
      .reduce((encoded) => encodeURIComponent(encoded), value);
  for (const secret of [
    "Bearer eyJhbGciOi.test-token",
    "sk-proj-test_secret",
  ]) {
    assertPrivate(
      nestEncoding(encodeEveryByte(secret), 3),
    );
  }
  assertPrivate(
    nestEncoding("progress%20complete", 20),
  );
  assertPrivate("%41".repeat(22_000));
  for (const encoded of [
    "https%3A%2F%2Fuser%3Apassword%40example.test%2F",
    "%2FUsers%2Falice%2Fraw.json",
    "%2Fprivate%2Ftmp%2Fraw.json",
    "file%3A%2F%2F%2FUsers%2Falice%2Fraw.json",
    "Bearer%20abcdefghijklmnop",
    "sk%2Dproj%2Dtest_secret",
    "%30%78%31%32%33%34%61%62%63%64",
    "%2530%2578%2531%2532%2533%2534%2561%2562%2563%2564",
    "https%253A%252F%252Fuser%253Apassword%2540example.test%252F",
    "%252FUsers%252Falice%252Fraw.json",
    "https%3A%2F%2Fuser%ZZ",
    "%2FUsers%2Falice%ZZ",
    "Bearer%20abcdefghijklmnop%ZZ",
    "sk%2Dproj%2Dtest%ZZ",
  ]) {
    assertPrivate(encoded);
  }
  assert.equal(validatePointerFreePrivacy({ note: "progress%20complete" }).pass, true);
  assert.equal(independentlyValidatePointerFreePrivacy({ note: "progress%20complete" }).pass, true);
  const fourLayerInnocuous = nestEncoding(encodeURIComponent("progress complete"), 3);
  assert.equal(validatePointerFreePrivacy({ note: fourLayerInnocuous }).pass, true);
  assert.equal(independentlyValidatePointerFreePrivacy({ note: fourLayerInnocuous }).pass, true);
  assert.equal(validatePointerFreePrivacy({ note: "100% ready" }).pass, true);
  assert.equal(independentlyValidatePointerFreePrivacy({ note: "100% ready" }).pass, true);
});

test("compact public receipt exposes only source/count/verdict digests", () => {
  const receipt = validReceipt();
  const parsed = validateAndRecomputeReceipt(receipt);
  const compact = publicEvidenceReceipt({ receipt, parsed, rawPacketSha256: HASH_B });
  assert.equal(compact.version, "web06-public-receipt-v1");
  assert.equal(compact.verdict, "PASS");
  assert.equal(compact.eventCount, 6);
  assert.equal(compact.actionCount, 3);
  assert.equal(validatePointerFreePrivacy(compact).pass, true);
  assert.equal("events" in compact, false);
  assert.equal("actions" in compact, false);
});

test("producer and verifier bind raw diagnostics separately while comparing exact semantic bytes", () => {
  const common = validRawEnvelope(validReceipt()).commonReceipt;
  common.commonSamples[0].domObserved.pageShape.visibleCount = 5;
  common.commonSamples[0].domFingerprintSha256 = createHash("sha256")
    .update(JSON.stringify(common.commonSamples[0].domObserved), "utf8").digest("hex");
  const runner = buildRoundEvidenceSummary(common, { surface: "common" });
  const independent = independentlyRecomputeRoundSummary(common, "common");
  const pair = independentlyBindSummaryDiagnostics(runner, independent, "common-alias");
  assert.notDeepEqual(runner.behaviorErrorCodes, independent.behaviorErrorCodes,
    "implementation-specific labels remain present in their raw summaries");
  assert.notEqual(
    pair.runnerBinding.dimensions.behaviorErrorCodes.rawSha256,
    pair.independentBinding.dimensions.behaviorErrorCodes.rawSha256,
  );
  assert.equal(
    pair.runnerBinding.dimensions.behaviorErrorCodes.semanticSha256,
    pair.independentBinding.dimensions.behaviorErrorCodes.semanticSha256,
  );
  assert.deepEqual(buildSummarySemanticProjection(runner), independentlyProjectSummarySemantics(independent));
  assert.equal(Buffer.from(summarySemanticProjectionBytes(runner), "utf8")
    .equals(Buffer.from(summarySemanticProjectionBytes(independent), "utf8")), true);
  assert.equal(pair.runnerSemanticProjectionSha256, pair.independentSemanticProjectionSha256);
  assert.deepEqual(runner.decision, independent.decision);
  assert.equal(runner.status, independent.status);
  assert.equal(runner.retryEligible, independent.retryEligible);
  assert.equal(runner.validRedObserved, independent.validRedObserved);
});

test("producer-valid diagnostic mutation families remain an exact runner-verifier bijection", () => {
  const bind = (receipt, surface, label) => {
    const runner = buildRoundEvidenceSummary(receipt, { surface });
    const independent = independentlyRecomputeRoundSummary(receipt, surface);
    assert.doesNotThrow(() => independentlyBindSummaryDiagnostics(runner, independent, label), label);
    assert.deepEqual(buildSummarySemanticProjection(runner),
      independentlyProjectSummarySemantics(independent), label);
  };
  const internalCases = [
    ["event identity and order", (receipt) => {
      [receipt.events[0], receipt.events[1]] = [receipt.events[1], receipt.events[0]];
    }],
    ["wire window start", (receipt) => { receipt.protocolWindow.receiptWindowStartEventSequenceId = 0; }],
    ["same-realm ordering", (receipt) => {
      receipt.events[0].eventDeliveredAt = receipt.events[0].normalizedEventAt - 1;
    }],
    ["worker span", (receipt) => {
      receipt.actions[0].workerSpans.abi.start = receipt.actions[0].workerActionStartedAt - 1;
    }],
    ["engine raw JSON", (receipt) => {
      receipt.actions[0].engineRaw.rawResponseJson = "{";
      receipt.actions[0].engineRaw.rawResponseSha256 = createHash("sha256").update("{", "utf8").digest("hex");
    }],
    ["presentation null", (receipt) => {
      receipt.actions[0].presentationExpected = null;
      receipt.actions[0].domObserved = null;
    }],
    ["actual lifecycle for frozen presentation", (receipt) => {
      receipt.actions[0].terminalKind = "lifecycle";
      receipt.actions[0].lifecycleEffect = {
        effect: "engine-state",
        effectDigest: "0".repeat(32),
        workerEffectDigest: "1".repeat(32),
        listenerEffectCount: 0,
        persistenceCompleted: true,
      };
    }],
    ["foreground false", (receipt) => { receipt.focusVisibilitySamples[0].focused = false; }],
    ["finite calibration failure plus behavior RED", (receipt) => {
      receipt.calibration.worker.pre[0].m3 = receipt.calibration.worker.pre[0].m0 - 1;
      receipt.events[0].stepId = "wrong-step";
    }],
    ["terminal owner missing", (receipt) => { delete receipt.actions.at(-1).terminalObservedAt; }],
    ["threshold violation", (receipt) => {
      receipt.actions[0].paintObservedAt = receipt.events[0].normalizedEventAt + 67.000001;
    }],
    ["frame RED", (receipt) => {
      receipt.interactionFrameIntervalsMs = [50, ...Array(14).fill(10)];
      receipt.interactionFrameTimestamps = receipt.interactionFrameIntervalsMs.reduce(
        (timestamps, interval) => [...timestamps, timestamps.at(-1) + interval],
        [90],
      );
      receipt.interactionFrameWindows[0].timestamps = structuredClone(receipt.interactionFrameTimestamps);
      receipt.interactionFrameWindows[0].intervalsMs = structuredClone(receipt.interactionFrameIntervalsMs);
      receipt.interactionWindows[0].endedAt = 280;
      receipt.interactionWindows[0].endBoundaryRafAt = 280;
      receipt.interactionWindows[0].postBoundaryFocusRecordedAt = 280.1;
      receipt.idleControlWindows[0].endedAt = 490;
      receipt.focusVisibilitySamples[1].recordedAt = 280.1;
    }],
    ["Long Task RED", (receipt) => {
      receipt.longTasks.push({
        pageInstanceId: PAGE_ID,
        startTime: 120,
        durationMs: 55,
        overlapsInteractionWindow: true,
        overlapsIdleControl: false,
      });
    }],
    ["excess action", (receipt) => {
      const extra = structuredClone(receipt.actions.at(-1));
      extra.sequenceId = 4;
      extra.actionId = "a4";
      receipt.actions.push(extra);
    }],
  ];
  for (const [label, mutate] of internalCases) {
    const receipt = validReceipt();
    mutate(receipt);
    bind(receipt, "internal", `internal:${label}`);
  }
  const frozenLifecycleActualPresentation = validReceipt();
  frozenLifecycleActualPresentation.scenarioId = "extended-scheduler-barriers";
  frozenLifecycleActualPresentation.scenarioRunId = "extended-scheduler-barriers@luna_pinyin";
  bind(frozenLifecycleActualPresentation, "internal",
    "internal:frozen lifecycle with actual presentation");

  const commonCases = [
    ["event identity", (receipt) => { receipt.events[0].stepId = "wrong-step"; }],
    ["same-realm ordering", (receipt) => {
      receipt.events[0].sentinelObservedAt = receipt.events[0].normalizedEventAt - 1;
    }],
    ["missing driver dispatch", (receipt) => { delete receipt.events[0].actualDriverDispatchAt; }],
    ["endpoint fingerprint", (receipt) => { receipt.commonSamples[0].domFingerprintSha256 = undefined; }],
    ["endpoint observation ordering", (receipt) => {
      receipt.commonSamples[0].observedAt = receipt.events[0].normalizedEventAt - 1;
    }],
    ["frame cardinality", (receipt) => { receipt.interactionFrameWindows[0].timestamps.pop(); }],
    ["idle control page", (receipt) => { receipt.idleControlWindows[0].pageInstanceId = "other-page"; }],
    ["observer installation", (receipt) => {
      receipt.longTaskObserver.installedAt = receipt.interactionWindows[0].startedAt + 1;
    }],
    ["focus window identity", (receipt) => {
      receipt.focusVisibilitySamples[0].windowId = "wrong-window";
    }],
    ["cadence too short", (receipt) => { receipt.cadenceGaps[0].actualDriverGapMs = 1; }],
    ["covering threshold RED", (receipt) => {
      receipt.commonSamples[0].observedAt = receipt.events[0].normalizedEventAt + 67.000001;
    }],
    ["missing common sample", (receipt) => { receipt.commonSamples.pop(); }],
  ];
  for (const [label, mutate] of commonCases) {
    const receipt = validRawEnvelope(validReceipt()).commonReceipt;
    mutate(receipt);
    bind(receipt, "common", `common:${label}`);
  }
  const thresholdRed = validRawEnvelope(validReceipt()).commonReceipt;
  thresholdRed.commonSamples[0].observedAt = thresholdRed.events[0].normalizedEventAt + 67.000001;
  const runnerThreshold = buildRoundEvidenceSummary(thresholdRed, { surface: "common" });
  const independentThreshold = independentlyRecomputeRoundSummary(thresholdRed, "common");
  assert.equal(runnerThreshold.status, "RED");
  assert.equal(runnerThreshold.validRedObserved, true);
  assert.ok(runnerThreshold.thresholdViolations.some((code) => code.startsWith("common-covering-")));
  assert.equal(independentThreshold.status, "RED");
  assert.equal(independentThreshold.validRedObserved, true);
  assert.doesNotThrow(() => independentlyBindSummaryDiagnostics(
    runnerThreshold,
    independentThreshold,
    "common:covering-threshold-measured-red",
  ));
});

test("diagnostic binding rejects decision drift and round or five-round semantic byte drift", () => {
  const receipt = validReceipt();
  const runnerRound = buildRoundEvidenceSummary(receipt, { surface: "internal" });
  const independentRound = independentlyRecomputeRoundSummary(receipt, "internal");
  const roundPair = independentlyBindSummaryDiagnostics(runnerRound, independentRound);
  assert.equal(roundPair.scope, "round");
  assert.deepEqual(validateIndependentDiagnosticBindingPairSchema(roundPair, "round").errors, []);
  const tamperedRoundPair = structuredClone(roundPair);
  tamperedRoundPair.runnerBinding.dimensions.behaviorErrorCodes.rawCount += 1;
  assert.equal(validateIndependentDiagnosticBindingPairSchema(tamperedRoundPair, "round").pass, false);

  const multiplicityDrift = structuredClone(runnerRound);
  multiplicityDrift.behaviorErrorCodes.push("EVENT_REORDERED:1", "EVENT_REORDERED:1");
  refreshDiagnosticBinding(multiplicityDrift);
  assert.throws(() => independentlyBindSummaryDiagnostics(multiplicityDrift, independentRound),
    /WEB06_INDEPENDENT_SUMMARY_DECISION_DIMENSIONS_MISMATCH/);

  const decisionDrift = structuredClone(runnerRound);
  decisionDrift.status = "RED";
  decisionDrift.validRedObserved = true;
  decisionDrift.decision.status = "RED";
  decisionDrift.decision.validRedObserved = true;
  refreshSemanticProjectionHash(decisionDrift);
  assert.throws(() => independentlyBindSummaryDiagnostics(decisionDrift, independentRound),
    /WEB06_INDEPENDENT_SUMMARY_DECISION_DIMENSIONS_MISMATCH/);

  const roundByteDrift = structuredClone(runnerRound);
  roundByteDrift.counts.events += 1;
  refreshSemanticProjectionHash(roundByteDrift);
  assert.throws(() => independentlyBindSummaryDiagnostics(roundByteDrift, independentRound),
    /WEB06_INDEPENDENT_SUMMARY_SEMANTIC_BYTE_MISMATCH/);

  const receipts = Array.from({ length: 5 }, (_, index) => {
    const value = validReceipt();
    value.roundId = `round-${index + 1}`;
    value.attemptId = `attempt-${index + 1}`;
    return value;
  });
  const runnerFive = buildFiveRoundEvidenceSummary(receipts, { surface: "internal" });
  const independentFive = independentlyRecomputeFiveRoundSummary(receipts, "internal");
  const fivePair = independentlyBindSummaryDiagnostics(runnerFive, independentFive);
  assert.equal(fivePair.scope, "five-round");
  assert.deepEqual(validateIndependentDiagnosticBindingPairSchema(fivePair, "five-round").errors, []);
  const tamperedFivePair = structuredClone(fivePair);
  tamperedFivePair.independentBinding.roundBindingsSha256 = "0".repeat(64);
  assert.equal(validateIndependentDiagnosticBindingPairSchema(tamperedFivePair, "five-round").pass, false);
  assert.equal(fivePair.runnerBinding.roundBindingsSha256,
    runnerFive.implementationDiagnosticBinding.roundBindingsSha256);
  assert.equal(fivePair.independentBinding.roundBindingsSha256,
    independentFive.implementationDiagnosticBinding.roundBindingsSha256);
  const fiveByteDrift = structuredClone(runnerFive);
  fiveByteDrift.pooledLongTaskCount += 1;
  refreshSemanticProjectionHash(fiveByteDrift);
  assert.throws(() => independentlyBindSummaryDiagnostics(fiveByteDrift, independentFive),
    /WEB06_INDEPENDENT_SUMMARY_SEMANTIC_BYTE_MISMATCH/);
});

test("clock calibration setup failures do not manufacture count or latency REDs", () => {
  const internalReceipt = validReceipt();
  internalReceipt.calibration.worker.pre = [];
  internalReceipt.longTasks.push({
    pageInstanceId: PAGE_ID,
    startTime: 120,
    durationMs: 55,
    overlapsInteractionWindow: true,
    overlapsIdleControl: false,
  });
  for (const summary of [
    buildRoundEvidenceSummary(internalReceipt, { surface: "internal" }),
    independentlyRecomputeRoundSummary(internalReceipt, "internal"),
  ]) {
    assert.equal(summary.status, "SETUP_INVALID");
    assert.equal(summary.retryEligible, true);
    assert.equal(summary.validRedObserved, false);
    assert.equal(summary.thresholdViolations.length, 0);
    assert.equal(summary.behaviorErrorCodes.some((code) =>
      /(?:COVERING|TERMINAL)_(?:SAMPLE_MISSING|COUNT)/.test(code)), false);
    assert.deepEqual(summary.counts, {
      events: 6,
      actions: 3,
      coveringSamples: 0,
      terminalSamples: 0,
      unclassifiedSamples: 0,
      interactionWindows: 1,
    });
    assert.deepEqual(summary.components, {});
    assert.equal(summary.frame.intervals, null);
    assert.equal(summary.longTask.count, 0);
    assert.equal(summary.longTask.durationMs, null);
  }

  const commonReceipt = validRawEnvelope(validReceipt()).commonReceipt;
  commonReceipt.pageSizeSetup = {
    uiTransition: [6, 7, 6],
    configuredPageSize: 6,
    sevenRows: 7,
    restoredControlValue: "6",
    realPreferencesControl: true,
  };
  commonReceipt.calibration.driver.pre = [];
  for (const summary of [
    buildRoundEvidenceSummary(commonReceipt, { surface: "common" }),
    independentlyRecomputeRoundSummary(commonReceipt, "common"),
  ]) {
    assert.equal(summary.status, "SETUP_INVALID");
    assert.equal(summary.retryEligible, true);
    assert.equal(summary.validRedObserved, false);
    assert.equal(summary.thresholdViolations.length, 0);
    assert.equal(summary.behaviorErrorCodes.some((code) =>
      /COMMON_(?:COVERING|TERMINAL)_COUNT/.test(code)), false);
    assert.deepEqual(summary.components, {});
    assert.equal(summary.frame.intervals, null);
    assert.equal(summary.longTask.count, 0);
  }

  const compactCommon = structuredClone(commonReceipt);
  compactCommon.commonSamples[0].sampleKind = "unknown-sample-kind";
  compactCommon.commonSamples[0].outcome = "unknown-outcome";
  const { publicReceipt } = parseAndCompactCommonReceipt(compactCommon, HASH_B);
  assert.equal(publicReceipt.eventCount, 6);
  assert.equal(publicReceipt.commonSampleCount, 3);
  assert.deepEqual(publicReceipt.roundSummary.counts, {
    events: 6,
    actions: 0,
    coveringSamples: 1,
    terminalSamples: 1,
    unclassifiedSamples: 1,
    interactionWindows: 1,
  });
  assert.equal(publicReceipt.roundSummary.outcomeCounts.unclassified, 1);
  assert.deepEqual(publicReceipt.roundSummary.components, {});
  assert.equal(publicReceipt.roundSummary.frame.intervals, null);
  assert.equal(publicReceipt.roundSummary.longTask.count, 0);
});

test("public receipt and pooled summaries reject recursive schema and identity mutations", () => {
  const receipts = Array.from({ length: 5 }, (_, index) => {
    const receipt = validReceipt();
    receipt.roundId = `round-${index + 1}`;
    receipt.attemptId = `attempt-${index + 1}`;
    return receipt;
  });
  const parsed = validateAndRecomputeReceipt(receipts[0]);
  const compact = publicEvidenceReceipt({ receipt: receipts[0], parsed, rawPacketSha256: HASH_B });
  const round = buildRoundEvidenceSummary(receipts[0], { surface: "internal" });
  const pooled = buildFiveRoundEvidenceSummary(receipts, { surface: "internal" });
  const commonReceipt = validRawEnvelope(validReceipt()).commonReceipt;
  const commonCompact = parseAndCompactCommonReceipt(commonReceipt, HASH_B).publicReceipt;
  assert.deepEqual(validatePublicEvidenceSchema(compact).errors, []);
  assert.deepEqual(validatePublicEvidenceSchema(commonCompact).errors, []);
  assert.deepEqual(validatePublicRoundSummarySchema(round).errors, []);
  assert.deepEqual(validatePublicFiveRoundSummarySchema(pooled).errors, []);
  for (const [value, validate, mutate] of [
    [compact, validatePublicEvidenceSchema, (copy) => { copy.version = "forged"; }],
    [compact, validatePublicEvidenceSchema, (copy) => { copy.actionCount += 1; }],
    [compact, validatePublicEvidenceSchema, (copy) => { copy.coveringSampleCount += 1; }],
    [commonCompact, validatePublicEvidenceSchema, (copy) => { copy.commonSampleCount += 1; }],
    [compact, validatePublicEvidenceSchema, (copy) => { copy.setupErrorCodes.push("SETUP_FORGED"); }],
    [compact, validatePublicEvidenceSchema, (copy) => { copy.artifactSha256 = "c".repeat(64); }],
    [compact, validatePublicEvidenceSchema, (copy) => { copy.selectedBranch = "A"; }],
    [compact, validatePublicEvidenceSchema, (copy) => { copy.disposition = "PRODUCTION_REDUCTION"; }],
    [compact, validatePublicEvidenceSchema, (copy) => {
      copy.verdict = "RED";
      copy.roundSummary.status = "RED";
    }],
    [compact, validatePublicEvidenceSchema, (copy) => { copy.roundSummary.status = "forged"; }],
    [round, validatePublicRoundSummarySchema, (copy) => { copy.counts.events = 1.5; }],
    [round, validatePublicRoundSummarySchema, (copy) => { copy.validRedObserved = true; }],
    [pooled, validatePublicFiveRoundSummarySchema,
      (copy) => { copy.roundSummaries[0].environmentId = "different-environment"; }],
    [pooled, validatePublicFiveRoundSummarySchema, (copy) => { copy.status = "RED"; }],
    [pooled, validatePublicFiveRoundSummarySchema, (copy) => { copy.validRedObserved = true; }],
    [pooled, validatePublicFiveRoundSummarySchema,
      (copy) => { copy.pooledComponents.eventToCoveringPaintMs.max += 1; }],
    [pooled, validatePublicFiveRoundSummarySchema,
      (copy) => { copy.pooledComponents.eventToCoveringPaintMs.unknown = true; }],
  ]) {
    const copy = structuredClone(value);
    mutate(copy);
    assert.equal(validate(copy).pass, false);
  }
});

test("independent verifier audits preserved premeasurement and partial behavior packets", async () => {
  const partialEnvelope = (measurementStarted, code, dimension,
    target = { id: "BASE_FULL", protocolMode: "full" }) => ({
    version: "web06-raw-attempt-v1",
    target,
    scenarioRunId: "fair-peer-short",
    scenarioId: "fair-peer-short",
    schemaId: "luna_pinyin",
    attemptId: "attempt-1",
    attemptNumber: 1,
    measurementStarted,
    measurementCompleted: false,
    runnerSourceBefore: runnerSourceObservation(),
    attemptSourceBefore: runnerSourceObservation(),
    observedEnvironment: observedEnvironmentObservation(),
    setupFailure: { name: "Error", message: `${code}:fixture` },
    browserFailure: { pageClosed: false, messageCode: code },
    partialAttempt: {
      version: "web06-partial-attempt-v1",
      phase: "failed",
      measurementStarted,
      measurementCompleted: false,
      driverEvents: [],
      cadenceGaps: [],
      burstRecoveries: [],
      pressureProofs: [],
      argumentCommitments: {},
      failure: { code, dimension },
    },
  });
  const expected = { targetId: "BASE_FULL", scenarioRunId: "fair-peer-short",
    scenarioId: "fair-peer-short", schemaId: "luna_pinyin", attemptId: "attempt-1" };
  assert.deepEqual(independentlyAuditRawEnvelope(
    partialEnvelope(false, "WEB06_SETUP_FAILURE", "setup"),
    { ...expected, measurementStarted: false },
  ), { pass: true, errors: [] });
  assert.deepEqual(independentlyAuditRawEnvelope(
    partialEnvelope(true, "WEB06_ACTION_COMPLETION_COUNT", "behavior"),
    { ...expected, measurementStarted: true },
  ), { pass: true, errors: [] });
  const relabelled = partialEnvelope(true, "WEB06_ACTION_COMPLETION_COUNT", "setup");
  assert.equal(independentlyAuditRawEnvelope(relabelled, expected).pass, false);
  const missingSource = partialEnvelope(false, "WEB06_SETUP_FAILURE", "setup");
  delete missingSource.runnerSourceBefore;
  delete missingSource.attemptSourceBefore;
  assert.equal(independentlyAuditRawEnvelope(missingSource, expected).pass, false);

  const productExpected = { ...expected, targetId: "PRODUCT" };
  const product = partialEnvelope(false, "WEB06_SETUP_FAILURE", "setup",
    { id: "PRODUCT", protocolMode: "off" });
  assert.deepEqual(independentlyAuditRawEnvelope(product,
    { ...productExpected, measurementStarted: false }), { pass: true, errors: [] });
  product.partialAttempt.failureProtocolExport = {
    header: { protocolVersion: "web06-private-v1", mode: "off" },
    status: {
      mainObserverCallbackCount: 0,
      mainObserverCallbackCapacity: 1,
      mainObserverCallbackOverflowCount: 0,
    },
    mainObserverCallbacks: [],
  };
  const productPrivate = independentlyAuditRawEnvelope(product, productExpected);
  assert.equal(productPrivate.pass, false);
  assert.ok(productPrivate.errors.includes("raw-partial-product-private-protocol-present"));
  const productTopLevelProtocol = partialEnvelope(false, "WEB06_SETUP_FAILURE", "setup",
    { id: "PRODUCT", protocolMode: "off" });
  productTopLevelProtocol.protocolExport = structuredClone(product.partialAttempt.failureProtocolExport);
  assert.ok(independentlyAuditRawEnvelope(
    productTopLevelProtocol,
    { ...productExpected, measurementStarted: false },
  ).errors.includes("raw-partial-product-private-protocol-present"));
  const productTopLevelReceipt = partialEnvelope(false, "WEB06_SETUP_FAILURE", "setup",
    { id: "PRODUCT", protocolMode: "off" });
  productTopLevelReceipt.privateReceipt = { retained: "private" };
  assert.ok(independentlyAuditRawEnvelope(
    productTopLevelReceipt,
    { ...productExpected, measurementStarted: false },
  ).errors.includes("raw-partial-product-private-protocol-present"));
  assert.deepEqual(retainedRawReceiptBehaviorErrors(productTopLevelProtocol), []);
  assert.deepEqual(retainedRawReceiptBehaviorErrors(productTopLevelReceipt), []);

  const minimal = partialEnvelope(true, "WEB06_ACTION_COMPLETION_COUNT", "behavior",
    { id: "BASE_MINIMAL", protocolMode: "minimal" });
  minimal.partialAttempt.failureProtocolExport = {
    header: { protocolVersion: "web06-private-v1", mode: "full" },
    status: {
      mainObserverCallbackCount: 0,
      mainObserverCallbackCapacity: 1,
      mainObserverCallbackOverflowCount: 0,
    },
    mainObserverCallbacks: [],
  };
  const mismatchedHeader = independentlyAuditRawEnvelope(minimal,
    { ...expected, targetId: "BASE_MINIMAL", measurementStarted: true });
  assert.equal(mismatchedHeader.pass, false);
  assert.ok(mismatchedHeader.errors.includes("raw-partial-private-protocol-header"));

  const malformedDiagnostics = partialEnvelope(
    true,
    "WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID",
    "setup",
  );
  malformedDiagnostics.partialAttempt.failureSentinel = {
    events: [],
    callbackLedger: [{
      callbackId: "partially-written",
      startedAt: null,
      finishedAt: Number.NaN,
      durationMs: "malformed",
    }],
    callbackLedgerCapacity: "malformed",
    callbackLedgerOverflowCount: null,
    sentinelAccountedCallbackCount: Number.NaN,
    sentinelCallbacksMs: [null, Number.NaN, "malformed"],
    unattributedInWindowCallbacksMs: [null],
    sentinelTotalPerEventMs: [Number.NaN],
    sentinelTotalPerWindowMs: ["malformed"],
  };
  malformedDiagnostics.partialAttempt.failureProtocolExport = {
    header: { protocolVersion: "web06-private-v1", mode: "full" },
    status: {
      mainObserverCallbackCount: "malformed",
      mainObserverCallbackCapacity: null,
      mainObserverCallbackOverflowCount: Number.NaN,
    },
    mainObserverCallbacks: [{
      callbackId: "partially-written",
      startedAt: null,
      finishedAt: Number.NaN,
      durationMs: "malformed",
    }],
    mainObserverCallbacksMs: [null, Number.NaN, "malformed"],
  };
  assert.deepEqual(independentlyAuditRawEnvelope(
    malformedDiagnostics,
    { ...expected, measurementStarted: true },
  ), { pass: true, errors: [] }, "partial callback diagnostics retain malformed scalar evidence");
  const sparseDiagnostics = structuredClone(malformedDiagnostics);
  delete sparseDiagnostics.partialAttempt.failureSentinel.callbackLedger[0];
  assert.ok(independentlyAuditRawEnvelope(
    sparseDiagnostics,
    { ...expected, measurementStarted: true },
  ).errors.includes("raw-partial-sentinel-shape"));
  const nonObjectDiagnostics = structuredClone(malformedDiagnostics);
  nonObjectDiagnostics.partialAttempt.failureProtocolExport.mainObserverCallbacks[0] = null;
  assert.ok(independentlyAuditRawEnvelope(
    nonObjectDiagnostics,
    { ...expected, measurementStarted: true },
  ).errors.includes("raw-partial-main-callback-shape"));
  const overCapacityDiagnostics = structuredClone(malformedDiagnostics);
  overCapacityDiagnostics.partialAttempt.failureSentinel.callbackLedger = Array.from(
    { length: 8_193 },
    () => ({ callbackId: "retained" }),
  );
  assert.ok(independentlyAuditRawEnvelope(
    overCapacityDiagnostics,
    { ...expected, measurementStarted: true },
  ).errors.includes("raw-partial-sentinel-shape"));

  const drift = partialEnvelope(
    true,
    "WEB06_RUNNER_SOURCE_CHANGED_DURING_ATTEMPT",
    "setup",
  );
  drift.attemptSourceAfter = structuredClone(drift.attemptSourceBefore);
  drift.attemptSourceAfter.sourceTree = "f".repeat(40);
  refreshRunnerSourceObservationHash(drift.attemptSourceAfter);
  assert.deepEqual(independentlyAuditRawEnvelope(
    drift,
    { ...expected, measurementStarted: true },
  ), { pass: true, errors: [] }, "a positive before/after source drift is preserved");
  const equalDrift = structuredClone(drift);
  equalDrift.attemptSourceAfter = structuredClone(equalDrift.attemptSourceBefore);
  assert.ok(independentlyAuditRawEnvelope(
    equalDrift,
    { ...expected, measurementStarted: true },
  ).errors.includes("raw-runner-source-drift-not-observed"));
  const missingAfterDrift = structuredClone(drift);
  delete missingAfterDrift.attemptSourceAfter;
  assert.ok(independentlyAuditRawEnvelope(
    missingAfterDrift,
    { ...expected, measurementStarted: true },
  ).errors.includes("raw-attempt-source-after:missing"));

  const partialFromCompleted = (receipt) => {
    const envelope = validRawEnvelope(receipt);
    const failure = partialEnvelope(
      true,
      "WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID",
      "setup",
    );
    envelope.measurementCompleted = false;
    envelope.attemptNumber = 1;
    envelope.commonReceipt.roundId = `${envelope.scenarioId}-round-1`;
    envelope.privateReceipt.roundId = envelope.commonReceipt.roundId;
    envelope.setupFailure = failure.setupFailure;
    envelope.browserFailure = failure.browserFailure;
    envelope.partialAttempt = structuredClone(malformedDiagnostics.partialAttempt);
    envelope.commonReceipt.pageSizeSetup = {
      uiTransition: [6, 7, 6],
      configuredPageSize: 6,
      sevenRows: 7,
      restoredControlValue: "6",
      realPreferencesControl: true,
    };
    return envelope;
  };
  const addCausativeCommonBehavior = (envelope) => {
    envelope.sentinel.snapshots[0].domObserved.candidates[0].text = "forged-candidate";
    envelope.commonReceipt.commonSamples = resolveCommonSamples({
      scenarioId: envelope.scenarioId,
      events: envelope.sentinel.events,
      snapshots: envelope.sentinel.snapshots,
    });
  };
  const receiptOnlyBehavior = partialFromCompleted(validReceipt());
  receiptOnlyBehavior.commonReceipt.commonSamples[0].outcome = "forged-outcome";
  assert.equal(independentlyAuditRawEnvelope(
    receiptOnlyBehavior,
    { ...expected, measurementStarted: true },
  ).pass, false, "receipt-only behavior cannot diverge from the raw sentinel snapshots");
  assert.deepEqual(retainedRawReceiptBehaviorErrors(receiptOnlyBehavior), []);
  assert.equal(independentlyRecomputeIncompleteAttemptFacts(receiptOnlyBehavior, {
    attemptId: "attempt-1",
    measurementStarted: true,
  }).classification, "SETUP_INVALID");

  const retainedBehavior = partialFromCompleted(validReceipt());
  addCausativeCommonBehavior(retainedBehavior);
  const producerBehaviorErrors = retainedRawReceiptBehaviorErrors(retainedBehavior);
  assert.ok(producerBehaviorErrors.length > 0);
  assert.deepEqual(independentlyRecomputeIncompleteAttemptFacts(retainedBehavior, {
    attemptId: "attempt-1",
    measurementStarted: true,
  }), {
    classification: "RED",
    retainedMeasured: false,
    retainedLogicalRound: false,
    validForLatencyFrame: false,
    retainedHardRed: true,
    retryEligible: false,
    validRedObserved: true,
  }, "raw-causative common behavior evidence survives a malformed callback snapshot");

  for (const [label, mutate] of [
    ["attempt", (copy) => { copy.commonReceipt.attemptId = "attempt-forged"; }],
    ["mode", (copy) => { copy.privateReceipt.mode = "BASE_MINIMAL"; }],
    ["source", (copy) => { copy.commonReceipt.source.commit = "f".repeat(40); }],
    ["round", (copy) => { copy.privateReceipt.roundId = "fair-peer-short-round-2"; }],
    ["mutually forged round", (copy) => {
      copy.commonReceipt.roundId = "forged-unrelated-round";
      copy.privateReceipt.roundId = "forged-unrelated-round";
    }],
    ["missing attempt number", (copy) => { delete copy.attemptNumber; }],
    ["mutually forged attempt ordinal", (copy) => {
      copy.attemptNumber = 7;
      copy.commonReceipt.roundId = `${copy.scenarioId}-round-7`;
      copy.privateReceipt.roundId = `${copy.scenarioId}-round-7`;
    }],
    ["envelope attempt", (copy) => { copy.attemptId = "attempt-forged"; }],
    ["guard summary", (copy) => {
      copy.commonReceipt.source.artifactResponseGuardSummarySha256 = "f".repeat(64);
      copy.privateReceipt.source.artifactResponseGuardSummarySha256 = "f".repeat(64);
    }],
    ["measurement projection", (copy) => { copy.measurementEvidence.idleFrameIntervalsMs = [999]; }],
    ["shared projection", (copy) => {
      copy.commonReceipt.assetsRequestedDuringWindow = [{
        name: "forged.js",
        startTime: 1,
      }];
    }],
  ]) {
    const unbound = structuredClone(retainedBehavior);
    mutate(unbound);
    assert.equal(independentlyAuditRawEnvelope(
      unbound,
      { ...expected, measurementStarted: true },
    ).pass, false, `${label}: independent raw audit`);
    assert.deepEqual(retainedRawReceiptBehaviorErrors(unbound), [],
      `${label}: producer refuses an unbound retained receipt`);
    assert.deepEqual(independentlyRecomputeIncompleteAttemptFacts(unbound, {
      attemptId: "attempt-1",
      measurementStarted: true,
    }), {
      classification: "SETUP_INVALID",
      retainedMeasured: false,
      retainedLogicalRound: false,
      validForLatencyFrame: false,
      retainedHardRed: false,
      retryEligible: true,
      validRedObserved: false,
    }, `${label}: independent recompute refuses an unbound retained receipt`);
  }

  const persistCausativeShapeFailure = (receipt, mutateCompleted) => {
    const completed = validRawEnvelope(receipt);
    completed.attemptNumber = 1;
    completed.commonReceipt.roundId = `${completed.scenarioId}-round-1`;
    completed.privateReceipt.roundId = completed.commonReceipt.roundId;
    completed.commonReceipt.pageSizeSetup = {
      uiTransition: [6, 7, 6],
      configuredPageSize: 6,
      sevenRows: 7,
      restoredControlValue: "6",
      realPreferencesControl: true,
    };
    mutateCompleted(completed);
    assert.equal(validateCompletedRawDecisionShape(completed).pass, false);
    assert.equal(independentlyValidateCompletedRawDecisionShape(completed).pass, false);
    const partial = {
      ...completed,
      measurementCompleted: false,
      setupFailure: {
        name: "Error",
        message: "WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID:fixture",
      },
      browserFailure: {
        pageClosed: false,
        messageCode: "WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID",
      },
      partialAttempt: {
        version: "web06-partial-attempt-v1",
        phase: "failed",
        measurementStarted: true,
        measurementCompleted: false,
        driverEvents: structuredClone(completed.drive.driverEvents),
        cadenceGaps: structuredClone(completed.drive.cadenceGaps),
        burstRecoveries: structuredClone(completed.drive.burstRecoveries),
        pressureProofs: [],
        argumentCommitments: structuredClone(completed.drive.argumentCommitments),
        failureSentinel: structuredClone(completed.sentinel),
        failureProtocolExport: structuredClone(completed.protocolExport),
        failure: {
          code: "WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID",
          dimension: "setup",
        },
      },
    };
    return JSON.parse(JSON.stringify(partial));
  };
  const causativeBehaviorReceipt = validReceipt();
  const causativeBehavior = persistCausativeShapeFailure(causativeBehaviorReceipt, (completed) => {
    addCausativeCommonBehavior(completed);
    completed.sentinel.callbackLedger = [{
      kind: "event",
      pageInstanceId: PAGE_ID,
      startedAt: 1,
      finishedAt: 2,
      durationMs: "malformed-causative-duration",
    }];
    completed.sentinel.sentinelCallbacksMs = ["malformed-causative-duration"];
  });
  assert.equal(causativeBehavior.partialAttempt.failureSentinel
    .callbackLedger[0].durationMs, "malformed-causative-duration");
  assert.deepEqual(independentlyAuditRawEnvelope(
    causativeBehavior,
    { ...expected, measurementStarted: true },
  ), { pass: true, errors: [] }, "causative malformed sentinel bytes survive partial persistence");
  const causativeBehaviorErrors = retainedRawReceiptBehaviorErrors(causativeBehavior);
  assert.ok(causativeBehaviorErrors.length > 0);
  const causativeBehaviorFacts = independentlyRecomputeIncompleteAttemptFacts(causativeBehavior, {
    attemptId: "attempt-1",
    measurementStarted: true,
  });
  assert.equal(causativeBehaviorFacts.classification, "RED");
  const behaviorCompact = buildIncompleteObserverModeProjection({
    rawPacket: { relativePath: "causative-behavior.raw.json", bytes: 1, sha256: HASH_A },
    measurementStarted: true,
    behaviorRedObserved: causativeBehaviorFacts.validRedObserved,
  });
  assert.equal(behaviorCompact.commonVerdict, "RED_BEHAVIOR");
  assert.equal(behaviorCompact.internalVerdict, "RED_BEHAVIOR");
  assert.equal(behaviorCompact.hardRedObserved, true);
  assert.deepEqual(
    validateWeb06ObserverModeProjectionSchema(behaviorCompact, "full"),
    { pass: true, errors: [] },
    "producer public schema preserves causative malformed behavior as a hard RED",
  );
  assert.deepEqual(
    independentlyValidateObserverModeProjectionSchema(behaviorCompact, "full"),
    { pass: true, errors: [] },
    "independent public schema preserves causative malformed behavior as a hard RED",
  );

  const causativeNumericReceipt = validReceipt();
  causativeNumericReceipt.actions[0].paintObservedAt =
    causativeNumericReceipt.actions[0].driverDispatchAt + 68;
  const causativeNumeric = persistCausativeShapeFailure(causativeNumericReceipt, (completed) => {
    completed.protocolExport.mainObserverCallbacksMs = ["malformed-causative-duration"];
  });
  assert.equal(causativeNumeric.partialAttempt.failureProtocolExport
    .mainObserverCallbacksMs[0], "malformed-causative-duration");
  assert.deepEqual(independentlyAuditRawEnvelope(
    causativeNumeric,
    { ...expected, measurementStarted: true },
  ), { pass: true, errors: [] }, "causative malformed protocol bytes survive partial persistence");
  assert.deepEqual(retainedRawReceiptBehaviorErrors(causativeNumeric), []);
  const causativeNumericFacts = independentlyRecomputeIncompleteAttemptFacts(causativeNumeric, {
    attemptId: "attempt-1",
    measurementStarted: true,
  });
  assert.equal(causativeNumericFacts.classification, "SETUP_INVALID");
  const numericCompact = buildIncompleteObserverModeProjection({
    rawPacket: { relativePath: "causative-numeric.raw.json", bytes: 1, sha256: HASH_A },
    measurementStarted: true,
    behaviorRedObserved: causativeNumericFacts.validRedObserved,
  });
  assert.equal(numericCompact.commonVerdict, "SETUP_INVALID");
  assert.equal(numericCompact.internalVerdict, "SETUP_INVALID");
  assert.equal(numericCompact.hardRedObserved, false);
  assert.deepEqual(
    validateWeb06ObserverModeProjectionSchema(numericCompact, "full"),
    { pass: true, errors: [] },
    "producer public schema preserves causative malformed numeric evidence as setup-invalid",
  );
  assert.deepEqual(
    independentlyValidateObserverModeProjectionSchema(numericCompact, "full"),
    { pass: true, errors: [] },
    "independent public schema preserves causative malformed numeric evidence as setup-invalid",
  );

  const verifyBaselineFixture = async ({
    fairEnvelope,
    fairAttemptFacts,
    fairAttemptCount,
    fairRunnerSummaries = () => ({}),
  }) => {
    const root = await mkdtemp(path.join(os.tmpdir(), "web06-baseline-causative-"));
    try {
      const rawDirectory = path.join(root, "raw");
      await mkdir(rawDirectory);
      const scenarioRuns = [...WEB06_BINDING_SCENARIO_ORDER];
      const scenarioIdsSha256 = createHash("sha256")
        .update(JSON.stringify(scenarioRuns), "utf8").digest("hex");
      const runnerTooling = {
        version: "web06-runner-tooling-v1",
        files: [{ path: "apps/yune-web/e2e/fixture.mjs", sha256: HASH_A }],
      };
      const runnerToolingManifestSha256 = createHash("sha256")
        .update(JSON.stringify(runnerTooling), "utf8").digest("hex");
      const runnerSourceManifest = {
        version: "web06-runner-source-v1",
        sourceCommit: COMMIT,
        sourceTree: TREE,
        sourceTreeState: "clean",
        tooling: runnerTooling,
        toolingManifestSha256: runnerToolingManifestSha256,
      };
      const runnerSourceManifestSha256 = createHash("sha256")
        .update(JSON.stringify(runnerSourceManifest), "utf8").digest("hex");
      const sourceObservation = runnerSourceObservation({
        toolingManifestSha256: runnerToolingManifestSha256,
      });
      const environmentObservation = observedEnvironmentObservation();
      const environmentId = "macbook-ac-60hz-clean-cache";
      const bindBaselineProvenance = (envelope, run) => {
        envelope.expectation = "BASELINE";
        envelope.scenarioRunId = run.runId;
        envelope.scenarioId = run.scenarioId;
        envelope.schemaId = run.schema;
        envelope.identityManifestSha256 = HASH_A;
        envelope.runnerSourceManifestSha256 = runnerSourceManifestSha256;
        envelope.scenarioIdsSha256 = scenarioIdsSha256;
        envelope.environmentManifestSha256 = HASH_B;
        envelope.environmentId = environmentId;
        envelope.runnerSourceBefore = structuredClone(sourceObservation);
        envelope.attemptSourceBefore = structuredClone(sourceObservation);
        if (envelope.attemptSourceAfter !== undefined) {
          envelope.attemptSourceAfter = structuredClone(sourceObservation);
        }
        envelope.observedEnvironment = structuredClone(environmentObservation);
        envelope.target = {
          ...envelope.target,
          id: "BASE_FULL",
          protocolMode: "full",
          sourceCommit: COMMIT,
          sourceTree: TREE,
          treeState: "clean",
          archiveSha256: envelope.target?.archiveSha256 ?? HASH_B,
          buildInfoSha256: envelope.target?.buildInfoSha256 ?? HASH_B,
          artifactSha256: envelope.target?.artifactSha256 ?? HASH_A,
          artifactResponseGuardSha256: envelope.target?.artifactResponseGuardSha256 ?? HASH_A,
          collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
          pinnedSelectedBranch: "NONE",
          pinnedDisposition: "SOURCE_CURRENT_BASELINE",
        };
        for (const receipt of [envelope.commonReceipt, envelope.privateReceipt]
          .filter((value) => value !== undefined)) {
          Object.assign(receipt, {
            scenarioRunId: run.runId,
            scenarioId: run.scenarioId,
            schemaId: run.schema,
            attemptId: envelope.attemptId,
            mode: "BASE_FULL",
          });
          Object.assign(receipt.source, {
            commit: COMMIT,
            tree: TREE,
            treeState: "clean",
            archiveSha256: envelope.target.archiveSha256,
            buildInfoSha256: envelope.target.buildInfoSha256,
            artifactSha256: envelope.target.artifactSha256,
            artifactResponseGuardSha256: envelope.target.artifactResponseGuardSha256,
            artifactResponseGuardSummarySha256: envelope.artifactResponseGuard?.summarySha256,
            identityManifestSha256: HASH_A,
            runnerSourceManifestSha256,
            runnerToolingManifestSha256,
            runnerSourceObservationSha256: sourceObservation.observationSha256,
            runnerSourcePostObservationSha256: sourceObservation.observationSha256,
            observedEnvironmentSha256: environmentObservation.observationSha256,
            collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
            scenarioIdsSha256,
            selectedBranch: "NONE",
            disposition: "SOURCE_CURRENT_BASELINE",
            environmentManifestSha256: HASH_B,
            environmentId,
          });
        }
        return envelope;
      };
      const setupInvalidFacts = {
        classification: "SETUP_INVALID",
        retainedMeasured: false,
        retainedLogicalRound: false,
        validForLatencyFrame: false,
        retainedHardRed: false,
        retryEligible: true,
        validRedObserved: false,
      };
      const writeAttempt = async (scenarioIndex, attemptNumber, envelope) => {
        const relativePath = `raw/${scenarioIndex + 1}-${attemptNumber}.json`;
        const bytes = Buffer.from(`${JSON.stringify(envelope)}\n`, "utf8");
        await writeFile(path.join(root, relativePath), bytes);
        return {
          relativePath,
          bytes: bytes.length,
          sha256: createHash("sha256").update(bytes).digest("hex"),
        };
      };
      const scenarioResults = [];
      for (const [scenarioIndex, scenarioRunId] of scenarioRuns.entries()) {
        const run = SCENARIO_RUN_REGISTRY[scenarioRunId];
        const isFairPeer = scenarioRunId === "fair-peer-short";
        const attemptCount = isFairPeer
          ? fairAttemptCount
          : WEB06_THRESHOLDS.attempts.maximum;
        const attempts = [];
        for (let attemptNumber = 1; attemptNumber <= attemptCount; attemptNumber += 1) {
          const attemptId = `attempt-${attemptNumber}`;
          const isFairSeed = isFairPeer && attemptNumber === 1;
          const envelope = isFairSeed
            ? structuredClone(fairEnvelope)
            : partialEnvelope(false, "WEB06_SETUP_FAILURE", "setup");
          envelope.attemptId = attemptId;
          if (!isFairSeed) envelope.attemptNumber = attemptNumber;
          bindBaselineProvenance(envelope, run);
          const rawPacket = await writeAttempt(scenarioIndex, attemptNumber, envelope);
          const facts = isFairSeed ? fairAttemptFacts : setupInvalidFacts;
          attempts.push({
            attemptId,
            measurementStarted: envelope.measurementStarted === true,
            measurementCompleted: envelope.measurementCompleted === true,
            ...facts,
            rawPacket,
            runnerSummaries: isFairSeed ? fairRunnerSummaries(envelope) : {},
          });
        }
        const measuredRoundCount = attempts.filter((attempt) => attempt.retainedMeasured === true).length;
        const validLatencyFrameRoundCount =
          attempts.filter((attempt) => attempt.validForLatencyFrame === true).length;
        const preservedHardRedAttemptIds =
          attempts.filter((attempt) => attempt.retainedHardRed === true).map((attempt) => attempt.attemptId);
        scenarioResults.push({
          targetId: "BASE_FULL",
          scenarioRunId,
          scenarioId: run.scenarioId,
          schemaId: run.schema,
          verdict: "SETUP_NO_GO",
          attempts,
          measuredRoundCount,
          validLatencyFrameRoundCount,
          preservedHardRedAttemptIds,
          preservedHardRedObserved: preservedHardRedAttemptIds.length > 0,
          runnerFiveRoundSummaries: {},
        });
      }
      const collector = {
        version: "web06-collector-output-v1",
        writeMode: "create-new",
        expectation: "BASELINE",
        disposition: "SOURCE_CURRENT_BASELINE",
        selectedBranch: "NONE",
        identityManifestSha256: HASH_A,
        runnerSourceManifest,
        runnerSourceManifestSha256,
        runnerSourceObservationSha256: sourceObservation.observationSha256,
        runnerSourcePostObservationSha256: sourceObservation.observationSha256,
        collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
        environmentManifestSha256: HASH_B,
        environmentId,
        scenarioRuns,
        observerTriplets: [],
        scenarioResults,
      };
      const collectorPath = path.join(root, "collector-output.json");
      await writeFile(collectorPath, `${JSON.stringify(collector, null, 2)}\n`);
      return await verifyCollectorOutput({
        evidenceRoot: root,
        collectorOutputPath: collectorPath,
        verifyCurrentSource: false,
      });
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  };

  const behaviorVerified = await verifyBaselineFixture({
    fairEnvelope: causativeBehavior,
    fairAttemptFacts: {
      classification: "RED",
      retainedMeasured: false,
      retainedLogicalRound: false,
      validForLatencyFrame: false,
      retainedHardRed: true,
      retryEligible: false,
      validRedObserved: true,
    },
    fairAttemptCount: 1,
  });
  assert.equal(behaviorVerified.verificationStatus, "PASS");
  const behaviorScenario = behaviorVerified.scenarioResults
    .find((scenario) => scenario.scenarioRunId === "fair-peer-short");
  assert.deepEqual({
    classification: behaviorScenario.attemptResults[0].classification,
    preservedHardRedAttemptIds: behaviorScenario.preservedHardRedAttemptIds,
    preservedHardRedObserved: behaviorScenario.preservedHardRedObserved,
  }, {
    classification: "RED",
    preservedHardRedAttemptIds: ["attempt-1"],
    preservedHardRedObserved: true,
  });

  const numericVerified = await verifyBaselineFixture({
    fairEnvelope: causativeNumeric,
    fairAttemptFacts: {
      classification: "SETUP_INVALID",
      retainedMeasured: false,
      retainedLogicalRound: false,
      validForLatencyFrame: false,
      retainedHardRed: false,
      retryEligible: true,
      validRedObserved: false,
    },
    fairAttemptCount: WEB06_THRESHOLDS.attempts.maximum,
  });
  assert.equal(numericVerified.verificationStatus, "PASS");
  const numericScenario = numericVerified.scenarioResults
    .find((scenario) => scenario.scenarioRunId === "fair-peer-short");
  assert.equal(numericScenario.attemptResults.length, WEB06_THRESHOLDS.attempts.maximum);
  assert.deepEqual({
    classification: numericScenario.attemptResults[0].classification,
    preservedHardRedAttemptIds: numericScenario.preservedHardRedAttemptIds,
    preservedHardRedObserved: numericScenario.preservedHardRedObserved,
  }, {
    classification: "SETUP_INVALID",
    preservedHardRedAttemptIds: [],
    preservedHardRedObserved: false,
  });

  const completedOrdinalForgery = validRawEnvelope(validReceipt());
  completedOrdinalForgery.attemptNumber = 7;
  completedOrdinalForgery.commonReceipt.roundId = `${completedOrdinalForgery.scenarioId}-round-7`;
  completedOrdinalForgery.privateReceipt.roundId = `${completedOrdinalForgery.scenarioId}-round-7`;
  await assert.rejects(() => verifyBaselineFixture({
    fairEnvelope: completedOrdinalForgery,
    fairAttemptFacts: {
      classification: "PASS",
      retainedMeasured: true,
      retainedLogicalRound: true,
      validForLatencyFrame: true,
      retainedHardRed: false,
      retryEligible: false,
      validRedObserved: false,
    },
    fairAttemptCount: WEB06_THRESHOLDS.attempts.maximum,
    fairRunnerSummaries: (envelope) => ({
      internal: buildRoundEvidenceSummary(envelope.privateReceipt, { surface: "internal" }),
      common: buildRoundEvidenceSummary(envelope.commonReceipt, { surface: "common" }),
    }),
  }), /WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID/,
  "file-level verification rejects a completed attempt-1 mutually forged as ordinal seven");

  const partialOrdinalForgery = structuredClone(causativeNumeric);
  partialOrdinalForgery.attemptNumber = 7;
  partialOrdinalForgery.commonReceipt.roundId = `${partialOrdinalForgery.scenarioId}-round-7`;
  partialOrdinalForgery.privateReceipt.roundId = `${partialOrdinalForgery.scenarioId}-round-7`;
  await assert.rejects(() => verifyBaselineFixture({
    fairEnvelope: partialOrdinalForgery,
    fairAttemptFacts: {
      classification: "SETUP_INVALID",
      retainedMeasured: false,
      retainedLogicalRound: false,
      validForLatencyFrame: false,
      retainedHardRed: false,
      retryEligible: true,
      validRedObserved: false,
    },
    fairAttemptCount: WEB06_THRESHOLDS.attempts.maximum,
  }), /raw-partial-attempt-ordinal/,
  "file-level verification rejects a partial attempt-1 mutually forged as ordinal seven");

  const numericOnly = partialFromCompleted(validReceipt());
  numericOnly.privateReceipt.actions[0].paintObservedAt =
    numericOnly.privateReceipt.actions[0].driverDispatchAt + 68;
  const numericParsed = validateAndRecomputeReceipt(numericOnly.privateReceipt);
  assert.equal(numericParsed.behaviorErrors.length, 0);
  assert.ok(numericParsed.thresholdViolations.length > 0);
  assert.deepEqual(retainedRawReceiptBehaviorErrors(numericOnly), []);
  assert.deepEqual(independentlyRecomputeIncompleteAttemptFacts(numericOnly, {
    attemptId: "attempt-1",
    measurementStarted: true,
  }), {
    classification: "SETUP_INVALID",
    retainedMeasured: false,
    retainedLogicalRound: false,
    validForLatencyFrame: false,
    retainedHardRed: false,
    retryEligible: true,
    validRedObserved: false,
  }, "numeric-only evidence is nonbinding when the callback snapshot is incomplete");
});

test("standalone independent verifier enforces the exact preview matrix and preserved raw bytes", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "web06-independent-"));
  try {
    const rawDirectory = path.join(root, "raw");
    await mkdir(rawDirectory);
    const runFacts = [
      ["existing-normal-guard", "existing-normal-guard", "jyut6ping3"],
      ["rapid-jyutping", "rapid-jyutping", "jyut6ping3"],
    ];
    const scenarioRuns = runFacts.map(([runId]) => runId);
    const scenarioIdsSha256 = createHash("sha256").update(JSON.stringify(scenarioRuns), "utf8").digest("hex");
    const runnerTooling = {
      version: "web06-runner-tooling-v1",
      files: [{ path: "apps/yune-web/e2e/fixture.mjs", sha256: HASH_A }],
    };
    const runnerToolingManifestSha256 = createHash("sha256")
      .update(JSON.stringify(runnerTooling), "utf8").digest("hex");
    const runnerSourceManifest = {
      version: "web06-runner-source-v1",
      sourceCommit: COMMIT,
      sourceTree: TREE,
      sourceTreeState: "clean",
      tooling: runnerTooling,
      toolingManifestSha256: runnerToolingManifestSha256,
    };
    const runnerSourceManifestSha256 = createHash("sha256")
      .update(JSON.stringify(runnerSourceManifest), "utf8").digest("hex");
    const sourceObservation = runnerSourceObservation({ toolingManifestSha256: runnerToolingManifestSha256 });
    const environmentObservation = observedEnvironmentObservation();
    const scenarioResults = [];
    const rawEnvelopes = new Map();
    for (const [scenarioRunId, scenarioId, schemaId] of runFacts) {
      const envelope = {
        version: "web06-raw-attempt-v1",
        target: {
          id: "FINAL_MINIMAL",
          protocolMode: "minimal",
          collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
          pinnedSelectedBranch: "A",
          pinnedDisposition: "PRODUCTION_REDUCTION",
        },
        expectation: "PREVIEW",
        scenarioRunId,
        scenarioId,
        schemaId,
        attemptId: "attempt-1",
        attemptNumber: 1,
        identityManifestSha256: HASH_A,
        runnerSourceManifestSha256,
        scenarioIdsSha256,
        environmentManifestSha256: HASH_B,
        environmentId: "test-environment",
        measurementStarted: false,
        measurementCompleted: false,
        runnerSourceBefore: structuredClone(sourceObservation),
        attemptSourceBefore: structuredClone(sourceObservation),
        observedEnvironment: structuredClone(environmentObservation),
        setupFailure: { name: "Error", message: "WEB06_SETUP_FAILURE:fixture" },
        browserFailure: { pageClosed: false, messageCode: "WEB06_SETUP_FAILURE" },
        partialAttempt: {
          version: "web06-partial-attempt-v1",
          phase: "failed",
          measurementStarted: false,
          measurementCompleted: false,
          driverEvents: [],
          cadenceGaps: [],
          burstRecoveries: [],
          pressureProofs: [],
          argumentCommitments: {},
          failure: { code: "WEB06_SETUP_FAILURE", dimension: "setup" },
        },
      };
      const relativePath = `raw/${scenarioRunId}.json`;
      const bytes = Buffer.from(`${JSON.stringify(envelope)}\n`, "utf8");
      await writeFile(path.join(root, relativePath), bytes);
      rawEnvelopes.set(relativePath, structuredClone(envelope));
      scenarioResults.push({
        targetId: "FINAL_MINIMAL",
        scenarioRunId,
        scenarioId,
        schemaId,
        verdict: "SETUP_INVALID",
        attempts: [{
          attemptId: "attempt-1",
          measurementStarted: false,
          measurementCompleted: false,
          classification: "SETUP_INVALID",
          retainedMeasured: false,
          retainedLogicalRound: false,
          validForLatencyFrame: false,
          retainedHardRed: false,
          retryEligible: true,
          validRedObserved: false,
          rawPacket: { relativePath, bytes: bytes.length, sha256: createHash("sha256").update(bytes).digest("hex") },
          runnerSummaries: {},
        }],
        measuredRoundCount: 0,
        validLatencyFrameRoundCount: 0,
        runnerFiveRoundSummaries: {},
      });
    }
    const collector = {
      version: "web06-collector-output-v1",
      writeMode: "create-new",
      expectation: "PREVIEW",
      disposition: "PRODUCTION_REDUCTION",
      selectedBranch: "A",
      identityManifestSha256: HASH_A,
      runnerSourceManifest,
      runnerSourceManifestSha256,
      runnerSourceObservationSha256: sourceObservation.observationSha256,
      runnerSourcePostObservationSha256: sourceObservation.observationSha256,
      collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
      environmentManifestSha256: HASH_B,
      environmentId: "test-environment",
      scenarioRuns,
      observerTriplets: [],
      scenarioResults,
    };
    const collectorPath = path.join(root, "collector-output.json");
    const persistCollector = (value) => writeFile(collectorPath, `${JSON.stringify(value, null, 2)}\n`);
    const persistRawEnvelope = async (collectorValue, scenarioIndex, envelope) => {
      const reference = collectorValue.scenarioResults[scenarioIndex].attempts[0].rawPacket;
      const bytes = Buffer.from(`${JSON.stringify(envelope)}\n`, "utf8");
      await writeFile(path.join(root, reference.relativePath), bytes);
      reference.bytes = bytes.length;
      reference.sha256 = createHash("sha256").update(bytes).digest("hex");
    };
    const restoreRawEnvelopes = async () => {
      for (const [index, result] of collector.scenarioResults.entries()) {
        await persistRawEnvelope(collector, index, rawEnvelopes.get(result.attempts[0].rawPacket.relativePath));
      }
    };
    await persistCollector(collector);
    const verified = await verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
      verifyCurrentSource: false });
    assert.equal(verified.verificationStatus, "PASS");
    assert.equal(verified.scenarioResults.length, 2);
    assert.ok(verified.scenarioResults.every((scenario) => scenario.attemptResults.length === 1));
    assert.deepEqual(validateIndependentRecomputeSchema(verified).errors, []);
    for (const mutate of [
      (copy) => { copy.unknown = true; },
      (copy) => { copy.scenarioResults[0].attemptResults[0].retainedMeasured = "true"; },
      (copy) => { copy.scenarioResults[0].fiveRoundSummaries.unknown = true; },
      (copy) => { copy.environmentId = "/tmp/private/web06"; },
      (copy) => { copy.environmentId = "/secret.txt"; },
    ]) {
      const copy = structuredClone(verified);
      mutate(copy);
      assert.equal(validateIndependentRecomputeSchema(copy).pass, false);
    }

    for (const [label, mutate, expectedError] of [
      ["identity", (copy) => { copy.identityManifestSha256 = "c".repeat(64); },
        /WEB06_INDEPENDENT_RAW_IDENTITY_MANIFEST/],
      ["contract", (copy) => { copy.collectorContractSha256 = "c".repeat(64); },
        /WEB06_INDEPENDENT_RAW_COLLECTOR_CONTRACT/],
      ["environment manifest", (copy) => { copy.environmentManifestSha256 = "c".repeat(64); },
        /WEB06_INDEPENDENT_RAW_ENVIRONMENT_MANIFEST/],
      ["environment id", (copy) => { copy.environmentId = "forged-environment"; },
        /WEB06_INDEPENDENT_RAW_ENVIRONMENT_ID/],
      ["runner manifest", (copy) => { copy.runnerSourceManifestSha256 = "c".repeat(64); },
        /WEB06_INDEPENDENT_RAW_RUNNER_SOURCE_MANIFEST/],
      ["runner observation", (copy) => { copy.runnerSourceObservationSha256 = "c".repeat(64); },
        /WEB06_INDEPENDENT_RAW_RUNNER_SOURCE_BEFORE/],
    ]) {
      const forged = structuredClone(collector);
      mutate(forged);
      await persistCollector(forged);
      await assert.rejects(() => verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
        verifyCurrentSource: false }), expectedError, label);
    }
    await persistCollector(collector);

    const brokenNestedToolingHash = structuredClone(collector);
    brokenNestedToolingHash.runnerSourceManifest.tooling.files[0].sha256 = "c".repeat(64);
    await persistCollector(brokenNestedToolingHash);
    await assert.rejects(() => verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
      verifyCurrentSource: false }), /WEB06_INDEPENDENT_RUNNER_TOOLING_MANIFEST_HASH/,
    "the independent verifier recomputes the nested runner tooling manifest hash");
    await persistCollector(collector);

    for (const [label, mutate, expectedError] of [
      ["observed environment payload", (envelope) => { envelope.observedEnvironment.host.cpuModel = "forged"; },
        /raw-observed-environment:hash/],
      ["runner source observation payload", (envelope) => {
        envelope.runnerSourceBefore.sourceTree = "f".repeat(40);
      }, /WEB06_INDEPENDENT_RAW_RUNNER_SOURCE_IDENTITY/],
    ]) {
      const forgedNested = structuredClone(collector);
      const firstPath = forgedNested.scenarioResults[0].attempts[0].rawPacket.relativePath;
      const envelope = structuredClone(rawEnvelopes.get(firstPath));
      mutate(envelope);
      await persistRawEnvelope(forgedNested, 0, envelope);
      await persistCollector(forgedNested);
      await assert.rejects(() => verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
        verifyCurrentSource: false }), expectedError, label);
      await restoreRawEnvelopes();
      await persistCollector(collector);
    }

    const forgedObservationIdentity = structuredClone(collector);
    let forgedObservationSha256;
    for (const [index, result] of forgedObservationIdentity.scenarioResults.entries()) {
      const envelope = structuredClone(rawEnvelopes.get(result.attempts[0].rawPacket.relativePath));
      for (const observation of [envelope.runnerSourceBefore, envelope.attemptSourceBefore]) {
        observation.toolingManifestSha256 = "c".repeat(64);
        refreshRunnerSourceObservationHash(observation);
      }
      forgedObservationSha256 = envelope.runnerSourceBefore.observationSha256;
      await persistRawEnvelope(forgedObservationIdentity, index, envelope);
    }
    forgedObservationIdentity.runnerSourceObservationSha256 = forgedObservationSha256;
    forgedObservationIdentity.runnerSourcePostObservationSha256 = forgedObservationSha256;
    await persistCollector(forgedObservationIdentity);
    await assert.rejects(() => verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
      verifyCurrentSource: false }), /WEB06_INDEPENDENT_RAW_RUNNER_SOURCE_IDENTITY/,
    "rehashing every raw observation cannot detach it from the pinned runner tooling manifest");
    await restoreRawEnvelopes();
    await persistCollector(collector);

    const forgedEverywhere = structuredClone(collector);
    forgedEverywhere.collectorContractSha256 = "c".repeat(64);
    for (const [index, result] of forgedEverywhere.scenarioResults.entries()) {
      const envelope = structuredClone(rawEnvelopes.get(result.attempts[0].rawPacket.relativePath));
      envelope.target.collectorContractSha256 = forgedEverywhere.collectorContractSha256;
      await persistRawEnvelope(forgedEverywhere, index, envelope);
    }
    await persistCollector(forgedEverywhere);
    await assert.rejects(() => verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
      verifyCurrentSource: false }), /WEB06_INDEPENDENT_RAW_COLLECTOR_CONTRACT/,
    "collector contract is independently recomputed instead of trusting mutually forged raw and collector bytes");
    await restoreRawEnvelopes();
    await persistCollector(collector);

    const crossRawDrift = structuredClone(collector);
    const secondPath = crossRawDrift.scenarioResults[1].attempts[0].rawPacket.relativePath;
    const secondEnvelope = structuredClone(rawEnvelopes.get(secondPath));
    secondEnvelope.environmentId = "second-raw-environment";
    await persistRawEnvelope(crossRawDrift, 1, secondEnvelope);
    await persistCollector(crossRawDrift);
    await assert.rejects(() => verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
      verifyCurrentSource: false }), /WEB06_INDEPENDENT_RAW_PROVENANCE_DRIFT/,
    "every raw envelope must carry the same source/environment identity");
    await restoreRawEnvelopes();
    await persistCollector(collector);

    const outside = await mkdtemp(path.join(os.tmpdir(), "web06-independent-outside-"));
    try {
      await writeFile(path.join(outside, "escape.json"), "{}\n");
      await symlink(outside, path.join(root, "linked-outside"));
      const escaped = structuredClone(collector);
      escaped.scenarioResults[0].attempts[0].rawPacket.relativePath = "linked-outside/escape.json";
      await writeFile(collectorPath, `${JSON.stringify(escaped, null, 2)}\n`);
      await assert.rejects(() => verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
        verifyCurrentSource: false }), /WEB06_INDEPENDENT_RAW_PATH_ESCAPE/);
    } finally {
      await rm(outside, { recursive: true, force: true });
    }

    const emptyFinal = structuredClone(collector);
    emptyFinal.expectation = "FINAL";
    emptyFinal.scenarioRuns = [];
    emptyFinal.scenarioResults = [];
    await writeFile(collectorPath, `${JSON.stringify(emptyFinal, null, 2)}\n`);
    await assert.rejects(() => verifyCollectorOutput({ evidenceRoot: root, collectorOutputPath: collectorPath,
      verifyCurrentSource: false }), /WEB06_INDEPENDENT_SCENARIO_MATRIX/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("independent raw audit and derivation reject deep source, endpoint, protocol, and behavior mutations", () => {
  const receipt = validReceipt();
  const envelope = validRawEnvelope(receipt);
  const expected = { targetId: "BASE_FULL", scenarioRunId: "fair-peer-short", scenarioId: "fair-peer-short",
    schemaId: "luna_pinyin", attemptId: "attempt-1", measurementStarted: true };
  assert.deepEqual(independentlyAuditRawEnvelope(envelope, expected), { pass: true, errors: [] });
  const baseline = buildRoundEvidenceSummary(receipt, { surface: "internal" });
  assert.deepEqual(independentlyRecomputeRoundSummary(receipt, "internal"), baseline);
    const rawMutations = [
      ["source target", (copy) => { copy.target.sourceTree = "f".repeat(40); }],
      ["receipt source", (copy) => { copy.privateReceipt.source.treeState = "dirty"; }],
      ["runner observation content", (copy) => { copy.runnerSourceBefore.sourceTree = "f".repeat(40); }],
      ["environment observation content", (copy) => { copy.observedEnvironment.host.cpuModel = "forged"; }],
      ["raw cadence", (copy) => { copy.drive.cadenceGaps[0].actualDriverGapMs = 1; }],
      ["protocol status", (copy) => { copy.protocolExport.status.valid = false; }],
      ["protocol invalidation", (copy) => { copy.protocolExport.invalidations.push("forged"); }],
      ["worker observer failure", (copy) => { copy.protocolExport.actions[0].worker.observerFailures.push("forged"); }],
      ["completion timeout", (copy) => { copy.completion.timedOut = true; }],
      ["completion count", (copy) => { copy.completion.observedActionCount = 0; }],
      ["private calibration", (copy) => { copy.privateReceipt.calibration.driver.pre[0].m1 += 1; }],
      ["measurement calibration", (copy) => { copy.measurementEvidence.calibration.driver.pre[0].m1 += 1; }],
      ["shared focus", (copy) => { copy.privateReceipt.focusVisibilitySamples.pop(); }],
      ["version", (copy) => { copy.privateReceipt.metricContractVersion = "forged"; }],
      ["event clock", (copy) => { copy.privateReceipt.eventClockProbe.eventTimestamp = 5; }],
      ["common snapshot", (copy) => { copy.sentinel.snapshots[0].domObserved.candidates[0].text = "forged"; }],
      ["common candidate", (copy) => { copy.commonReceipt.commonSamples[0].domObserved.candidates[0].text = "forged"; }],
      ["event identity", (copy) => { copy.privateReceipt.events[0].key = "x"; }],
      ["protocol event", (copy) => { copy.protocolExport.events[0].identity.key = "x"; }],
      ["action args", (copy) => { copy.privateReceipt.actions[0].args = ["{x}"]; }],
      ["protocol action", (copy) => { copy.protocolExport.actions[0].identity.sequenceId = 99; }],
      ["engine raw", (copy) => { copy.protocolExport.actions[0].worker.engineRawJson = "{}"; }],
      ["presentation", (copy) => { copy.protocolExport.actions[0].presentation.domObserved.input = "x"; }],
      ["wire identity", (copy) => { copy.privateReceipt.actions[0].wireSequenceId = 99; }],
      ["same realm", (copy) => { copy.privateReceipt.actions[0].responseMappingStartedAt = 1; }],
      ["cross clock", (copy) => { copy.privateReceipt.actions[0].workerActionStartedAt = 500; }],
      ["idle proof", (copy) => { copy.privateReceipt.idleFrameIntervalsMs.pop(); }],
      ["observer", (copy) => { copy.privateReceipt.longTaskObserver.supported = false; }],
      ["focus", (copy) => { copy.privateReceipt.focusVisibilitySamples[0].focused = false; }],
      ["frame window", (copy) => { copy.privateReceipt.interactionFrameWindows[0].timestamps[1] += 1; }],
      ["asset request", (copy) => { copy.privateReceipt.assetsRequestedDuringWindow.push({ name: "/late.wasm", startTime: 100 }); }],
      ["burst recovery", (copy) => { copy.privateReceipt.burstRecoveries = [{ afterStepId: "forged" }]; }],
      ["cadence identity", (copy) => { copy.privateReceipt.cadenceGaps[0].stepId = "forged"; }],
      ["callback ceiling", (copy) => {
        copy.sentinel.callbackLedger.push({ callbackId: "s1", startedAt: 1, finishedAt: 11, durationMs: 10 });
        copy.sentinel.sentinelAccountedCallbackCount = 1;
      }],
    ];
    for (const [label, mutate] of rawMutations) {
      const copy = validRawEnvelope(validReceipt());
      mutate(copy);
      const audit = independentlyAuditRawEnvelope(copy, expected);
      const summary = independentlyRecomputeRoundSummary(copy.privateReceipt, "internal");
      assert.ok(!audit.pass || JSON.stringify(summary) !== JSON.stringify(baseline), label);
    }
  const forgedRunner = structuredClone(baseline);
  forgedRunner.counts.events += 1;
  assert.notDeepEqual(independentlyRecomputeRoundSummary(receipt, "internal"), forgedRunner);
});

test("independent timing and frame proofs fail closed on every cross-realm, paint-chain, and window gap", () => {
  assert.equal(independentlyRecomputeRoundSummary(validReceipt(), "internal").status, "PASS");

  const internalSetupCases = [
    ["worker same-realm ordering", (receipt) => {
      receipt.actions[0].workerMessageReceivedAt = receipt.actions[0].workerActionStartedAt + 0.01;
    }, "SETUP_INVALID_CLOCK_CALIBRATION:worker-same-realm-order:1"],
    ["send/receive uncertainty", (receipt) => {
      receipt.actions[0].workerMessageReceivedAt = receipt.actions[0].workerSentAt + 0.05;
    }, "SETUP_INVALID_CLOCK_CALIBRATION:send-receive-order:1"],
    ["enqueue/start uncertainty", (receipt) => {
      receipt.actions[0].actionEnqueuedAt = receipt.actions[0].workerActionStartedAt - 0.05;
      receipt.actions[0].wireIdentity.actionEnqueuedAt = receipt.actions[0].actionEnqueuedAt;
      receipt.actions[0].returnedWireIdentity.actionEnqueuedAt = receipt.actions[0].actionEnqueuedAt;
    }, "SETUP_INVALID_CLOCK_CALIBRATION:enqueue-start-order:1"],
    ["finish/response uncertainty", (receipt) => {
      receipt.actions[0].workerFinishedAt = receipt.actions[0].mainResponseReceivedAt - 0.05;
    }, "SETUP_INVALID_CLOCK_CALIBRATION:finish-response-order:1"],
    ["driver/event uncertainty", (receipt) => {
      receipt.events[0].actualDriverDispatchAt = 99.95;
      receipt.actions[0].driverDispatchAt = 99.95;
    }, "SETUP_INVALID_CLOCK_CALIBRATION:driver-event-order:1"],
    ["dispatch outside interaction window", (receipt) => {
      receipt.events[0].actualDriverDispatchAt = 80;
      receipt.events[0].requestedDriverDispatchAt = 80;
      receipt.actions[0].driverDispatchAt = 80;
    }, "SETUP_DISPATCH_OUTSIDE_INTERACTION_WINDOWS"],
    ["terminal observation outside interaction window", (receipt) => {
      receipt.actions[2].terminalObservedAt = 241;
    }, "SETUP_OBSERVATION_OUTSIDE_INTERACTION_WINDOWS"],
    ["hidden mid-window sample", (receipt) => {
      receipt.focusVisibilitySamples.push({
        recordedAt: 150,
        focused: false,
        visibilityState: "hidden",
        role: "mid-window",
        windowId: "web06-window-1",
        pageInstanceId: PAGE_ID,
      });
    }, "SETUP_PAGE_NOT_FOREGROUND"],
  ];
  for (const [label, mutate, expected] of internalSetupCases) {
    const receipt = validReceipt();
    mutate(receipt);
    const summary = independentlyRecomputeRoundSummary(receipt, "internal");
    assert.ok(summary.setupErrorCodes.includes(expected), label);
  }

  const commonDispatchOutside = validRawEnvelope(validReceipt()).commonReceipt;
  commonDispatchOutside.events[0].actualDriverDispatchAt = 80;
  commonDispatchOutside.events[0].requestedDriverDispatchAt = 80;
  assert.ok(independentlyRecomputeRoundSummary(commonDispatchOutside, "common").setupErrorCodes
    .includes("SETUP_DISPATCH_OUTSIDE_INTERACTION_WINDOWS"));

  for (const [field, expected] of [
    ["stateUpdateScheduledAt", "PAINT_CHAIN_MISSING:1:stateUpdateScheduledAt"],
    ["stateCommittedAt", "PAINT_CHAIN_MISSING:1:stateCommittedAt"],
  ]) {
    const receipt = validReceipt();
    delete receipt.actions[0][field];
    const summary = independentlyRecomputeRoundSummary(receipt, "internal");
    assert.ok(summary.behaviorErrorCodes.includes(expected));
    assert.ok(summary.behaviorErrorCodes.includes("TIMELINE_RESIDUAL_NONFINITE:1"));
  }

  const residual = independentlyValidateActionTiming([{
    sequenceId: 1,
    outcome: "painted",
    eventDeliveredAt: 0,
    actionEnqueuedAt: 1,
    workerSentAt: 2,
    mainResponseReceivedAt: 3,
    responseMappingStartedAt: 4,
    responseMappingFinishedAt: 5,
    stateUpdateScheduledAt: 6,
    stateCommittedAt: 7,
    paintObservedAt: 8,
    metrics: { timelineResidualMs: 0.100001 },
  }]);
  assert.ok(residual.errors.includes("TIMELINE_RESIDUAL:1:0.100001"));
});

test("independent burst recovery rejects mutually forged negative queue accounting", () => {
  const row = SCENARIO_REGISTRY["burst-action-map"];
  const step = row.steps.find((candidate) => candidate.declaredBurstPauseAfter === true);
  assert.ok(step);
  const receipt = {
    events: [{ stepId: step.id, type: "keydown", normalizedEventAt: 100 }],
    burstRecoveries: [{
      afterStepId: step.id,
      latestPaintAt: 110,
      expectedCompletedActionCount: 0,
      idleSnapshot: {
        queueDepth: 0,
        runningActionId: null,
        pendingFanoutActions: 0,
        pendingTerminalActions: 0,
        pendingSentinelCaptures: 0,
        completedActionCount: 0,
      },
    }],
  };
  assert.deepEqual(independentlyVerifyBurstRecoveries(receipt, row), {
    behaviorErrors: [],
    thresholdViolations: [],
  });

  receipt.burstRecoveries[0].expectedCompletedActionCount = -1;
  receipt.burstRecoveries[0].idleSnapshot.completedActionCount = -1;
  assert.ok(independentlyVerifyBurstRecoveries(receipt, row).behaviorErrors
    .includes(`BURST_IDLE_SNAPSHOT_INVALID:${step.id}`));
});

test("independent five-round pool enforces driver-dispatch terminal maximum", () => {
  const receipts = Array.from({ length: 5 }, (_, index) => {
    const row = validReceipt();
    row.roundId = `round-${index + 1}`;
    row.attemptId = `attempt-${index + 1}`;
    row.events[4].actualDriverDispatchAt = 100;
    row.actions[2].driverDispatchAt = 100;
    return row;
  });
  const independent = independentlyRecomputeFiveRoundSummary(receipts, "internal");
  assert.ok(independent.poolViolations.includes("pooled-driver-terminal:max"));

  for (const [field, invalidValue, suffix] of [
    ["roundId", receipts[0].roundId, "ROUNDID"],
    ["attemptId", receipts[0].attemptId, "ATTEMPTID"],
    ["attemptId", "", "ATTEMPTID"],
  ]) {
    const invalid = structuredClone(receipts);
    invalid[4][field] = invalidValue;
    assert.throws(() => evaluateFiveRoundPool(invalid),
      new RegExp(`WEB06_POOL_${suffix}_IDENTITY_INVALID`));
    assert.throws(() => independentlyRecomputeFiveRoundSummary(invalid, "internal"),
      new RegExp(`WEB06_INDEPENDENT_POOL_${suffix}_IDENTITY_INVALID`));
  }
});
