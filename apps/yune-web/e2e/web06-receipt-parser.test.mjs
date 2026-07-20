import assert from "node:assert/strict";
import test from "node:test";

import {
  PEER_LOGICAL_INPUT_IDS,
  computeBindingPeerRatio,
  evaluateCollectionEquivalence,
  evaluatePackageAlignment,
  publicEvidenceReceipt,
  validateAndRecomputeReceipt,
  validatePointerFreePrivacy,
  validateSupersessionGraph,
} from "./web06-receipt-parser.mjs";
import {
  WEB06_METRIC_CONTRACT_VERSION,
  WEB06_SCENARIO_REGISTRY_VERSION,
  expandScenarioExpectedTimeline,
} from "./web06-metric-contract.mjs";

const COMMIT = "0123456789abcdef0123456789abcdef01234567";
const HASH_A = "a".repeat(64);
const HASH_B = "b".repeat(64);

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
  return {
    ...frozen,
    compositionEpochId: "epoch-1",
    supersessionSubRunId: "subrun-1",
    driverDispatchAt: event.actualDriverDispatchAt,
    actionEnqueuedAt: start + 2,
    mainQueueDepth: 0,
    workerDispatchDepth: 0,
    workerSentAt: start + 3,
    workerMessageReceivedAt: start + 3.2,
    workerActionStartedAt: start + 3.4,
    workerFinishedAt: start + 4.4,
    mainResponseReceivedAt: start + 4.6,
    responseMappingStartedAt: start + 5,
    responseMappingFinishedAt: start + 6,
    stateUpdateScheduledAt: start + 6.5,
    stateCommittedAt: start + 7.5,
    ...(terminal ? { terminalObservedAt: start + 8.5 } : { paintObservedAt: start + 8.5 }),
    workerSpans: {
      abi: { start: start + 3.4, end: start + 3.8 },
      responseExtract: { start: start + 3.8, end: start + 4 },
      jsonParse: { start: start + 4, end: start + 4.2 },
      adapterTranslate: { start: start + 4.2, end: start + 4.4 },
      persistence: null,
    },
    persistenceRan: false,
    outcome,
    engineRaw: {
      logicalInput: rawActionSequence.filter((item) => item.length === 1).join(""),
      actionKind: frozen.kind,
      compositionEpochId: "epoch-1",
      supersessionSubRunId: "subrun-1",
      rawActionSequence,
      rawCandidateStatusSha256: HASH_A,
      pageSha256: HASH_B,
    },
    presentationExpected: { digest: `dom-${frozen.sequenceId}` },
    domObserved: { digest: `dom-${frozen.sequenceId}` },
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
  const events = timeline.events.map((frozen, index) => ({
    ...frozen,
    eventTimestamp: eventStarts[index],
    normalizedEventAt: eventStarts[index],
    eventDeliveredAt: eventStarts[index] + 1,
    requestedDriverDispatchAt: eventStarts[index] - 1,
    actualDriverDispatchAt: eventStarts[index] - 1,
    modifiers: [],
  }));
  const rawSequences = [["n"], ["n", "i"], ["n", "i", "selectCandidate:0"]];
  const actions = timeline.actions.map((frozen, index) => actionReceipt(
    frozen,
    events[frozen.eventSequenceId - 1],
    rawSequences[index],
    index === 2 ? "committed" : "painted",
  ));
  return {
    metricContractVersion: WEB06_METRIC_CONTRACT_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_REGISTRY_VERSION,
    scenarioId: "fair-peer-short",
    mode: "BASE_FULL",
    source: { commit: COMMIT, treeState: "clean", artifactSha256: HASH_A },
    roundId: "round-1",
    attemptId: "attempt-1",
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
    actions,
    cadenceGaps: [{ stepId: "peer-short-2", nominalGapMs: 60, actualDriverGapMs: 60 }],
    idleFrameIntervalsMs: Array(120).fill(16.67),
    interactionFrameIntervalsMs: Array(15).fill(16.67),
    longTaskObserverSupported: true,
    longTasks: [],
    focusVisibilitySamples: [{ focused: true, visibilityState: "visible" }],
    assetsRequestedDuringWindow: [],
    pressureProofs: [],
  };
}

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

test("missing, duplicate, and reordered IDs are non-retryable behavior REDs", () => {
  const missing = validReceipt();
  missing.events.splice(2, 1);
  const missingParsed = validateAndRecomputeReceipt(missing);
  assert.equal(missingParsed.status, "RED");
  assert.ok(missingParsed.behaviorErrors.some((error) => error.includes("EVENT_MISSING_ID")));

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
  assert.equal(validatePointerFreePrivacy({ authorization: "Bearer secret" }).pass, false);
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
