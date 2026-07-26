import {
  ACTION_REGISTRY,
  SCENARIO_REGISTRY,
  SCENARIO_RUN_REGISTRY,
  WEB06_BEHAVIOR_PREDICATES,
  WEB06_BEHAVIOR_PREDICATE_VERSION,
  WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION,
  WEB06_METRIC_CONTRACT_VERSION,
  WEB06_SCENARIO_REGISTRY_VERSION,
  WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION,
  WEB06_THRESHOLDS,
  WEB06_DISPOSITIONS,
  WEB06_PRESSURE_PAIR_REGISTRY,
  WEB06_SELECTED_BRANCHES,
  buildClockCalibration,
  correctDriverTimestamp,
  correctWorkerTimestamp,
  distributionSummary,
  evaluateThresholdDistribution,
  expandScenarioExpectedTimeline,
  isSha256,
  resolveScenarioRun,
} from "./web06-metric-contract.mjs";
import { createHash } from "node:crypto";

const COMMIT_SHA_RE = /^[0-9a-f]{40}$/;
const FORBIDDEN_KEY_RE = /(?:(?:^|[_-])ptr(?:$|[_-])|pointer|address|authorization|cookie|password|secret|stack|browserProfile|userDataDir|(?:access|auth|bearer|api)Token)/i;
const ABSOLUTE_PATH_RE = /(?:^|[\s"'=(:,\[])(?:file:\/\/|\/(?:[^/\s()[\]{},;]+\/)*[^/\s()[\]{},;]+|[A-Za-z]:[\\/]|\\\\[^\\\s]+\\)/;
const POINTER_VALUE_RE = /(?:^|[\s"'=(:,\[])0x[0-9a-f]{6,}(?=$|[\s"',);}\]])/i;
const SENSITIVE_VALUE_RE = /(?:\bBearer[ \t]+[A-Za-z0-9._~+/=-]+|\bsk-proj-[A-Za-z0-9_-]+|https?:\/\/[^/\s:@]+:[^/\s@]+@)/i;
const ALLOWED_OUTCOMES = new Set([
  "painted",
  "superseded",
  "committed",
  "processed-no-visual-change",
  "barrier-completed",
  "failure",
]);
const PUBLIC_OUTCOME_COUNT_KEYS = Object.freeze([...ALLOWED_OUTCOMES, "unclassified"].sort());
const INTERNAL_RECEIPT_MODES = new Set([
  "BASE_MINIMAL",
  "BASE_FULL",
  "FINAL_MINIMAL",
  "FINAL_FULL",
]);
const FULL_RECEIPT_MODES = new Set(["BASE_FULL", "FINAL_FULL"]);
const MINIMAL_RECEIPT_MODES = new Set(["BASE_MINIMAL", "FINAL_MINIMAL"]);

function sameJson(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function hasOwn(record, key) {
  return typeof key === "string" && Object.prototype.hasOwnProperty.call(record, key);
}

function stableDigestTokens(value, emit) {
  if (value === null) return emit("null;");
  if (value === undefined) return emit("undefined;");
  if (typeof value === "string") return emit(`s${value.length}:${value}`);
  if (typeof value === "number") return emit(`n${Object.is(value, -0) ? "-0" : String(value)};`);
  if (typeof value === "boolean") return emit(value ? "b1;" : "b0;");
  if (typeof value === "bigint") return emit(`i${String(value)};`);
  if (Array.isArray(value)) {
    emit(`a${value.length}:`);
    for (const item of value) stableDigestTokens(item, emit);
    return;
  }
  if (typeof value === "object") {
    const keys = Object.keys(value).sort();
    emit(`o${keys.length}:`);
    for (const key of keys) {
      stableDigestTokens(key, emit);
      stableDigestTokens(value[key], emit);
    }
    return;
  }
  throw new Error(`WEB06_STABLE_DIGEST_UNSUPPORTED:${typeof value}`);
}

/** Independent implementation of the private protocol's frozen 128-bit digest. */
export function web06StableDigest(value) {
  const hashes = [0x811c9dc5, 0x9e3779b9, 0x85ebca6b, 0xc2b2ae35];
  const multipliers = [0x01000193, 0x27d4eb2d, 0x165667b1, 0x85ebca77];
  stableDigestTokens(value, (token) => {
    for (let index = 0; index < token.length; index += 1) {
      const codeUnit = token.charCodeAt(index);
      for (let stream = 0; stream < hashes.length; stream += 1) {
        hashes[stream] = Math.imul(hashes[stream] ^ codeUnit, multipliers[stream]);
      }
    }
  });
  return hashes.map((hash) => (hash >>> 0).toString(16).padStart(8, "0")).join("");
}

function presentationStateDigest(value) {
  if (!value || typeof value !== "object") return undefined;
  const { sequenceId: _sequenceId, ...state } = value;
  return web06StableDigest(state);
}

function strictPrefix(left, right) {
  return Array.isArray(left)
    && Array.isArray(right)
    && left.length < right.length
    && left.every((value, index) => sameJson(value, right[index]));
}

function pushIf(errors, condition, code) {
  if (condition) errors.push(code);
}

function finite(value) {
  return typeof value === "number" && Number.isFinite(value);
}

function isClockCalibrationSetupError(value) {
  return typeof value === "string"
    && (value.startsWith("SETUP_INVALID_CLOCK_CALIBRATION")
      || value === "SETUP_LEARNED_CALIBRATION_SEGMENTS_INVALID");
}

function requireArrayFields(receipt, fields, setupErrors, prefix) {
  for (const field of fields) {
    if (!Array.isArray(receipt[field])) setupErrors.push(`${prefix}_${field.toUpperCase()}_ARRAY_MISSING`);
  }
}

const MEASUREMENT_PROTOCOL_THRESHOLD_BLOCKERS = new Set([
  "MAIN_OBSERVER_CALLBACK_CEILING",
  "COLLECTOR_CALLBACK_CEILING",
]);

function classifyMeasurementProtocolBlockers(blockers) {
  const setupErrors = [];
  const thresholdViolations = [];
  for (const blocker of blockers) {
    if (MEASUREMENT_PROTOCOL_THRESHOLD_BLOCKERS.has(blocker)) {
      thresholdViolations.push(`measurement-protocol:${blocker}`);
    } else {
      setupErrors.push(`SETUP_MEASUREMENT_PROTOCOL:${blocker}`);
    }
  }
  return { setupErrors, thresholdViolations };
}

const SENTINEL_OVERFLOW_FIELDS = Object.freeze([
  "events",
  "auxiliaryEvents",
  "unmatchedEvents",
  "snapshots",
  "frameTimestamps",
  "longTasks",
  "focus",
  "callbacks",
  "windows",
  "idleControls",
  "driverDispatchBindings",
  "pendingCaptures",
]);

function validateSentinelOverflows(receipt, setupErrors) {
  const counts = receipt.sentinelOverflowCounts;
  if (!counts || typeof counts !== "object" || Array.isArray(counts)
    || !sameJson(Object.keys(counts).sort(), [...SENTINEL_OVERFLOW_FIELDS].sort())
    || Object.values(counts).some((value) => !Number.isSafeInteger(value) || value < 0)) {
    setupErrors.push("SETUP_SENTINEL_OVERFLOW_COUNTERS_INVALID");
    return;
  }
  for (const [field, count] of Object.entries(counts)) {
    if (count > 0) setupErrors.push(`SETUP_SENTINEL_LEDGER_OVERFLOW:${field}:${count}`);
  }
}

function validateAuxiliaryDomEvents(receipt, behaviorErrors) {
  const expected = receipt.scenarioId === "extended-scheduler-barriers"
    ? [{ stepId: "extended-option-target", type: "click" }]
    : [];
  const actual = receipt.auxiliaryEvents.map(({ stepId, type }) => ({ stepId, type }));
  if (!sameJson(actual, expected)) behaviorErrors.push("AUXILIARY_DOM_EVENT_SEQUENCE");
  for (const event of receipt.auxiliaryEvents) {
    if (!finite(event.eventTimestamp) || !finite(event.sentinelObservedAt)
      || event.sentinelObservedAt < event.eventTimestamp) {
      behaviorErrors.push(`AUXILIARY_DOM_EVENT_TIME:${event.stepId ?? "unknown"}`);
    }
  }
}

function validateContiguousIds(rows, field, start, prefix, errors) {
  const values = rows.map((row) => row[field]);
  if (new Set(values).size !== values.length) errors.push(`${prefix}_DUPLICATE_ID`);
  for (let index = 0; index < values.length; index += 1) {
    if (values[index] !== start + index) {
      errors.push(`${prefix}_${values.includes(start + index) ? "REORDERED" : "MISSING"}_ID`);
      break;
    }
  }
}

function validatePrivacy(value, path, errors) {
  if (Array.isArray(value)) {
    value.forEach((child, index) => validatePrivacy(child, `${path}[${index}]`, errors));
    return;
  }
  if (!value || typeof value !== "object") {
    const webUrl = typeof value === "string" && /^https?:\/\//i.test(value);
    if (typeof value === "string" && ((!webUrl && ABSOLUTE_PATH_RE.test(value))
      || POINTER_VALUE_RE.test(value) || SENSITIVE_VALUE_RE.test(value))) {
      errors.push(`PUBLIC_PRIVACY_VALUE:${path}`);
    }
    return;
  }
  for (const [key, child] of Object.entries(value)) {
    const childPath = path ? `${path}.${key}` : key;
    if (FORBIDDEN_KEY_RE.test(key)) errors.push(`PUBLIC_PRIVACY_KEY:${childPath}`);
    validatePrivacy(child, childPath, errors);
  }
}

export function validatePointerFreePrivacy(value) {
  const errors = [];
  validatePrivacy(value, "", errors);
  return { pass: errors.length === 0, errors };
}

function validateIdentity(receipt, setupErrors) {
  pushIf(setupErrors, receipt.metricContractVersion !== WEB06_METRIC_CONTRACT_VERSION, "SETUP_METRIC_VERSION_MISMATCH");
  pushIf(setupErrors, receipt.scenarioRegistryVersion !== WEB06_SCENARIO_REGISTRY_VERSION, "SETUP_SCENARIO_VERSION_MISMATCH");
  pushIf(setupErrors, receipt.behaviorPredicateVersion !== WEB06_BEHAVIOR_PREDICATE_VERSION, "SETUP_BEHAVIOR_PREDICATE_VERSION_MISMATCH");
  pushIf(setupErrors, !COMMIT_SHA_RE.test(receipt.source?.commit ?? ""), "SETUP_SOURCE_COMMIT_INVALID");
  pushIf(setupErrors, !COMMIT_SHA_RE.test(receipt.source?.tree ?? ""), "SETUP_SOURCE_TREE_INVALID");
  pushIf(setupErrors, receipt.source?.treeState !== "clean", "SETUP_SOURCE_TREE_NOT_CLEAN");
  pushIf(setupErrors, !isSha256(receipt.source?.artifactSha256), "SETUP_ARTIFACT_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.archiveSha256), "SETUP_ARCHIVE_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.buildInfoSha256), "SETUP_BUILD_INFO_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.artifactResponseGuardSha256),
    "SETUP_ARTIFACT_RESPONSE_GUARD_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.artifactResponseGuardSummarySha256),
    "SETUP_ARTIFACT_RESPONSE_GUARD_SUMMARY_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.identityManifestSha256), "SETUP_IDENTITY_MANIFEST_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.runnerSourceManifestSha256), "SETUP_RUNNER_SOURCE_MANIFEST_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.runnerToolingManifestSha256), "SETUP_RUNNER_TOOLING_MANIFEST_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.runnerSourceObservationSha256), "SETUP_RUNNER_SOURCE_OBSERVATION_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.runnerSourcePostObservationSha256)
    || receipt.source?.runnerSourcePostObservationSha256 !== receipt.source?.runnerSourceObservationSha256,
  "SETUP_RUNNER_SOURCE_POST_OBSERVATION_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.observedEnvironmentSha256), "SETUP_OBSERVED_ENVIRONMENT_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.collectorContractSha256), "SETUP_COLLECTOR_CONTRACT_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.scenarioIdsSha256), "SETUP_SCENARIO_SET_HASH_INVALID");
  pushIf(setupErrors, !isSha256(receipt.source?.environmentManifestSha256), "SETUP_ENVIRONMENT_MANIFEST_HASH_INVALID");
  pushIf(setupErrors, typeof receipt.source?.environmentId !== "string" || !receipt.source.environmentId,
    "SETUP_ENVIRONMENT_ID_INVALID");
  pushIf(setupErrors, !WEB06_SELECTED_BRANCHES.includes(receipt.source?.selectedBranch), "SETUP_SELECTED_BRANCH_INVALID");
  pushIf(setupErrors, !WEB06_DISPOSITIONS.includes(receipt.source?.disposition), "SETUP_DISPOSITION_INVALID");
  if (receipt.source?.selectedBranch === "NONE") {
    pushIf(setupErrors, !["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE", "MEASURED_NO_GO"].includes(receipt.source?.disposition), "SETUP_BRANCH_DISPOSITION_MISMATCH");
  } else {
    pushIf(setupErrors, receipt.source?.disposition !== "PRODUCTION_REDUCTION", "SETUP_BRANCH_DISPOSITION_MISMATCH");
  }
  pushIf(setupErrors, !["PRODUCT", "BASE_MINIMAL", "BASE_FULL", "FINAL_MINIMAL", "FINAL_FULL"].includes(receipt.mode), "SETUP_MODE_INVALID");
  const run = hasOwn(SCENARIO_RUN_REGISTRY, receipt.scenarioRunId)
    ? SCENARIO_RUN_REGISTRY[receipt.scenarioRunId]
    : undefined;
  pushIf(setupErrors, run === undefined || run.scenarioId !== receipt.scenarioId || run.schema !== receipt.schemaId,
    "SETUP_SCENARIO_RUN_IDENTITY_MISMATCH");
  if (receipt.mode === "PRODUCT") {
    pushIf(setupErrors, receipt.source?.selectedBranch !== "NONE" || receipt.source?.disposition !== "DIAGNOSTIC",
      "SETUP_PRODUCT_DISPOSITION_INVALID");
  } else if (receipt.mode?.startsWith("BASE_")) {
    pushIf(setupErrors, receipt.source?.selectedBranch !== "NONE"
      || !["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE"].includes(receipt.source?.disposition),
    "SETUP_BASE_DISPOSITION_INVALID");
  } else if (receipt.mode?.startsWith("FINAL_")) {
    const finalPairValid = receipt.source?.selectedBranch === "NONE"
      ? receipt.source?.disposition === "MEASURED_NO_GO"
      : receipt.source?.disposition === "PRODUCTION_REDUCTION";
    pushIf(setupErrors, !finalPairValid, "SETUP_FINAL_DISPOSITION_INVALID");
  }
  pushIf(setupErrors, typeof receipt.roundId !== "string" || !receipt.roundId, "SETUP_ROUND_ID_INVALID");
  pushIf(setupErrors, typeof receipt.attemptId !== "string" || !receipt.attemptId, "SETUP_ATTEMPT_ID_INVALID");
  pushIf(setupErrors, receipt.measurementStarted !== true, "SETUP_MEASUREMENT_NOT_STARTED");
  pushIf(setupErrors, receipt.measurementCompleted !== true, "SETUP_MEASUREMENT_NOT_COMPLETED");
}

function validateInternalMode(receipt, setupErrors) {
  if (!INTERNAL_RECEIPT_MODES.has(receipt.mode)) {
    setupErrors.push(receipt.mode === "PRODUCT"
      ? "SETUP_PRODUCT_INTERNAL_RECEIPT_FORBIDDEN"
      : "SETUP_INTERNAL_MODE_INVALID");
  }
}

function validateEventClock(receipt, setupErrors, behaviorErrors) {
  const probes = receipt.eventClockSegments && typeof receipt.eventClockSegments === "object"
    ? Object.values(receipt.eventClockSegments) : [receipt.eventClockProbe];
  if (!probes.length || probes.some((probe) => !probe || !finite(probe.beforeDispatchAt)
    || !finite(probe.eventTimestamp) || !finite(probe.afterDispatchAt))) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_PROBE");
  } else if (probes.some((probe) => probe.eventTimestamp < probe.beforeDispatchAt
    || probe.eventTimestamp > probe.afterDispatchAt)) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_ORIGIN");
  }
  const previousByPage = new Map();
  for (const event of receipt.events ?? []) {
    const pageInstanceId = event.pageInstanceId ?? "single-page";
    const previousTimestamp = previousByPage.get(pageInstanceId) ?? -Infinity;
    if (!finite(event.eventTimestamp) || event.eventTimestamp < 0 || event.normalizedEventAt !== event.eventTimestamp) {
      setupErrors.push(`SETUP_INVALID_EVENT_TIMESTAMP:${event.eventSequenceId}`);
      continue;
    }
    if (event.eventTimestamp < previousTimestamp) setupErrors.push(`SETUP_DECREASING_EVENT_TIMESTAMP:${event.eventSequenceId}`);
    previousByPage.set(pageInstanceId, event.eventTimestamp);
    if (!finite(event.eventDeliveredAt) || event.eventDeliveredAt < event.normalizedEventAt) {
      behaviorErrors.push(`EVENT_SAME_REALM_ORDER:${event.eventSequenceId}`);
    }
  }
}

function expectedCommonSamples(row, expected) {
  let logicalInput = "";
  const samples = [];
  for (const step of row.steps) {
    for (const mapped of step.actions) {
      if (mapped.kind === "processKey") {
        const raw = mapped.args[0];
        const key = typeof raw === "string" && raw.startsWith("{") && raw.endsWith("}")
          ? raw.slice(1, -1)
          : "";
        if (key === "BackSpace") logicalInput = logicalInput.slice(0, -1);
        else if (key === "Escape" || key === "space" || key === "Return") logicalInput = "";
        else if (!["Page_Down", "Page_Up", "Down", "Up"].includes(key) && key.length === 1) logicalInput += key;
      } else if (mapped.kind === "selectCandidate") {
        logicalInput = "";
      }
    }
    if (step.expectedLogicalInputAfter !== undefined) logicalInput = step.expectedLogicalInputAfter;
    if (step.source === "browser-lifecycle") logicalInput = "";
    if (step.sample === "none") continue;
    const expectedActions = expected.actions.filter((action) => action.stepId === step.id);
    const owner = step.sample === "terminal"
      ? terminalOwnerForStep(step, expected.actions)
      : expectedActions[0];
    samples.push({
      stepId: step.id,
      sampleKind: step.sample,
      eventSequenceId: owner?.eventSequenceId,
      expectedInput: logicalInput,
      stressDeadline: owner?.stressDeadline === true,
    });
  }
  return samples;
}

function candidateTexts(domObserved) {
  return (domObserved?.candidates ?? []).map((candidate) => candidate.text);
}

function matchesPrefix(actual, expected) {
  return expected.every((value, index) => actual[index] === value);
}

/** Independently bind coherent DOM endpoints to frozen external predicates. */
export function validateBehaviorPredicateSamples(receipt) {
  const errors = [];
  for (const [identity, predicate] of Object.entries(WEB06_BEHAVIOR_PREDICATES)) {
    if (!identity.startsWith(`${receipt.scenarioId}:`)) continue;
    const stepId = identity.slice(receipt.scenarioId.length + 1);
    const sample = (receipt.commonSamples ?? []).find((candidate) => candidate.stepId === stepId);
    if (!sample) {
      errors.push(`BEHAVIOR_PREDICATE_SAMPLE_MISSING:${stepId}`);
      continue;
    }
    if (sample.outcome === "superseded") errors.push(`BEHAVIOR_PREDICATE_SUPERSEDED:${stepId}`);
    const expected = predicate.expected;
    const observed = sample.domObserved;
    const texts = candidateTexts(observed);
    if (expected.renderedInput !== undefined && observed?.renderedInput !== expected.renderedInput) {
      errors.push(`BEHAVIOR_RENDERED_INPUT:${stepId}`);
    }
    if (expected.candidateTextsExact !== undefined && !sameJson(texts, expected.candidateTextsExact)) {
      errors.push(`BEHAVIOR_CANDIDATE_EXACT:${stepId}`);
    }
    if (expected.candidateTextsPrefix !== undefined && !matchesPrefix(texts, expected.candidateTextsPrefix)) {
      errors.push(`BEHAVIOR_CANDIDATE_PREFIX:${stepId}`);
    }
    if (expected.candidateTextsInclude !== undefined
      && expected.candidateTextsInclude.some((value) => !texts.includes(value))) {
      errors.push(`BEHAVIOR_CANDIDATE_INCLUDE:${stepId}`);
    }
    if (expected.visibleCount !== undefined && observed?.pageShape?.visibleCount !== expected.visibleCount) {
      errors.push(`BEHAVIOR_PAGE_VISIBLE_COUNT:${stepId}`);
    }
    if (expected.previousDisabled !== undefined
      && observed?.pageShape?.previousDisabled !== expected.previousDisabled) {
      errors.push(`BEHAVIOR_PAGE_PREVIOUS:${stepId}`);
    }
    if (expected.nextDisabled !== undefined && observed?.pageShape?.nextDisabled !== expected.nextDisabled) {
      errors.push(`BEHAVIOR_PAGE_NEXT:${stepId}`);
    }
    if (expected.textareaValue !== undefined && observed?.textareaValue !== expected.textareaValue) {
      errors.push(`BEHAVIOR_TEXTAREA_VALUE:${stepId}`);
    }
    if (expected.commitTextExact !== undefined && observed?.textareaValue !== expected.commitTextExact) {
      errors.push(`BEHAVIOR_COMMIT_TEXT:${stepId}`);
    }
    if (expected.selectionStart !== undefined && observed?.selectionStart !== expected.selectionStart) {
      errors.push(`BEHAVIOR_SELECTION_START:${stepId}`);
    }
    if (expected.selectionEnd !== undefined && observed?.selectionEnd !== expected.selectionEnd) {
      errors.push(`BEHAVIOR_SELECTION_END:${stepId}`);
    }
    if (expected.visibleComposition !== undefined
      && (observed?.renderedInput !== "") !== expected.visibleComposition) {
      errors.push(`BEHAVIOR_VISIBLE_COMPOSITION:${stepId}`);
    }
    if (expected.persistenceCompleted === true) {
      const continuity = receipt.lifecycleContinuity;
      if (typeof continuity?.measurementId !== "string" || !continuity.measurementId
        || continuity?.pre?.measurementId !== continuity.measurementId
        || continuity?.post?.measurementId !== continuity.measurementId
        || typeof continuity?.pre?.continuityNonce !== "string" || !continuity.pre.continuityNonce
        || continuity.pre.continuityNonce !== continuity?.post?.continuityNonce
        || continuity?.pre?.terminal?.persistenceCompleted !== true
        || continuity?.post?.phase !== "post-reload-bound") {
        errors.push(`BEHAVIOR_PERSISTENCE_CONTINUITY:${stepId}`);
      }
    }
  }
  return errors;
}

function validateCommonEventClock(receipt, setupErrors, behaviorErrors) {
  const probes = receipt.eventClockSegments && typeof receipt.eventClockSegments === "object"
    ? Object.values(receipt.eventClockSegments) : [receipt.eventClockProbe];
  if (!probes.length || probes.some((probe) => !probe || !finite(probe.beforeDispatchAt)
    || !finite(probe.eventTimestamp) || !finite(probe.afterDispatchAt))) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_PROBE");
  } else if (probes.some((probe) => probe.eventTimestamp < probe.beforeDispatchAt
    || probe.eventTimestamp > probe.afterDispatchAt)) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_ORIGIN");
  }
  const previousByPage = new Map();
  for (const event of receipt.events ?? []) {
    const pageInstanceId = event.pageInstanceId ?? "single-page";
    const previous = previousByPage.get(pageInstanceId) ?? -Infinity;
    if (!finite(event.eventTimestamp) || event.eventTimestamp < 0 || event.normalizedEventAt !== event.eventTimestamp) {
      setupErrors.push(`SETUP_INVALID_EVENT_TIMESTAMP:${event.eventSequenceId}`);
      continue;
    }
    if (event.eventTimestamp < previous) setupErrors.push(`SETUP_DECREASING_EVENT_TIMESTAMP:${event.eventSequenceId}`);
    previousByPage.set(pageInstanceId, event.eventTimestamp);
    if (!finite(event.sentinelObservedAt) || event.sentinelObservedAt < event.normalizedEventAt) {
      behaviorErrors.push(`COMMON_EVENT_SAME_REALM_ORDER:${event.eventSequenceId}`);
    }
  }
}

function finalAttemptStatus({
  setupErrors,
  behaviorErrors,
  cadence,
  thresholdViolations = [],
  frameRed = false,
  longTaskRed = false,
}) {
  const behaviorRed = behaviorErrors.length > 0;
  const numericOrJankRed = thresholdViolations.length > 0 || frameRed || longTaskRed;
  // Numeric/jank evidence is binding only when setup and attribution are
  // valid. Independently observable behavior/order failures remain evidence
  // even when setup defects coexist.
  const validRedObserved = behaviorRed
    || (setupErrors.length === 0 && cadence !== "TOO_SHORT" && numericOrJankRed);
  let status;
  if (behaviorRed) status = cadence === "TOO_SHORT" ? "RED_BEHAVIOR" : "RED";
  else if (setupErrors.length > 0) status = "SETUP_INVALID";
  else if (validRedObserved) status = "RED";
  else if (cadence === "TOO_SHORT" || cadence === "TOO_LONG") status = "NO_VERDICT_INVALID_CADENCE";
  else status = "PASS";
  return {
    status,
    validRedObserved,
    retryEligible: !validRedObserved && ["SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(status),
    qualifiers: {
      setupInvalid: setupErrors.length > 0,
      cadenceInvalid: cadence === "TOO_SHORT" || cadence === "TOO_LONG",
      instrumentationAttributionInvalid: validRedObserved && setupErrors.length > 0,
      nonbindingThresholdObservationRed: cadence === "TOO_SHORT" && thresholdViolations.length > 0,
      nonbindingFrameObservationRed: cadence === "TOO_SHORT" && frameRed,
      nonbindingLongTaskObservationRed: cadence === "TOO_SHORT" && longTaskRed,
    },
  };
}

/**
 * PRODUCT has no internal WEB-06 protocol. This parser deliberately validates
 * only evidence observed by the identical Playwright sentinel in every mode.
 */
export function validateCommonSurfaceReceipt(receipt) {
  const setupErrors = [];
  const behaviorErrors = [];
  validateIdentity(receipt, setupErrors);
  const row = hasOwn(SCENARIO_REGISTRY, receipt.scenarioId)
    ? SCENARIO_REGISTRY[receipt.scenarioId]
    : undefined;
  if (!row) setupErrors.push("SETUP_UNKNOWN_SCENARIO");
  behaviorErrors.push(...validatePointerFreePrivacy(receipt).errors);
  if (!row) {
    return {
      status: "SETUP_INVALID",
      setupErrors,
      behaviorErrors,
      cadence: "NOT_APPLICABLE",
      frameRed: false,
      longTaskRed: false,
      thresholdViolations: [],
      metrics: { samples: [] },
    };
  }
  requireArrayFields(receipt, [
    "events",
    "auxiliaryEvents",
    "unmatchedEvents",
    "commonSamples",
    "cadenceGaps",
    "idleFrameIntervalsMs",
    "interactionFrameIntervalsMs",
    "interactionFrameTimestamps",
    "interactionFrameWindows",
    "interactionWindows",
    "idleControlWindows",
    "longTasks",
    "focusVisibilitySamples",
    "assetsRequestedDuringWindow",
    "measurementProtocolBlockers",
  ], setupErrors, "SETUP_COMMON");
  if (setupErrors.some((error) => error.startsWith("SETUP_COMMON_"))) {
    return { status: "SETUP_INVALID", setupErrors, behaviorErrors, thresholdViolations: [], metrics: { samples: [] } };
  }
  validateSentinelOverflows(receipt, setupErrors);
  const protocolDisposition = classifyMeasurementProtocolBlockers(receipt.measurementProtocolBlockers);
  setupErrors.push(...protocolDisposition.setupErrors);
  validateAuxiliaryDomEvents(receipt, behaviorErrors);

  const expected = expandScenarioExpectedTimeline(row.id);
  const events = receipt.events ?? [];
  if ((receipt.unmatchedEvents ?? []).length > 0) behaviorErrors.push("COMMON_UNMATCHED_DOM_EVENT");
  if (receipt.candidatePageSize !== 6) behaviorErrors.push("COMMON_CANDIDATE_PAGE_SIZE_NOT_SIX");
  const pageSizeSetup = receipt.pageSizeSetup;
  if (!pageSizeSetup || !sameJson(pageSizeSetup.uiTransition, [6, 7, 6])
    || pageSizeSetup.configuredPageSize !== 6
    || pageSizeSetup.sevenRows !== 7 || pageSizeSetup.restoredControlValue !== "6"
    || pageSizeSetup.realPreferencesControl !== true) {
    behaviorErrors.push("COMMON_PAGE_SIZE_UI_TRANSITION_INVALID");
  }
  const firstEligiblePageShape = (receipt.commonSamples ?? [])
    .map((sample) => sample.domObserved?.pageShape)
    .find((pageShape) => Number.isSafeInteger(pageShape?.visibleCount) && pageShape.visibleCount > 0);
  if (firstEligiblePageShape?.visibleCount !== 6) {
    behaviorErrors.push("COMMON_FIRST_ELIGIBLE_ENDPOINT_PAGE_SIZE_NOT_SIX");
  }
  if (events.length !== row.expectedDomEventCount) {
    behaviorErrors.push(`COMMON_EVENT_COUNT:${events.length}!=${row.expectedDomEventCount}`);
  }
  validateContiguousIds(events, "eventSequenceId", 1, "COMMON_EVENT", behaviorErrors);
  for (let index = 0; index < Math.min(events.length, expected.events.length); index += 1) {
    const actual = events[index];
    const frozen = expected.events[index];
    if (actual.stepId !== frozen.stepId || actual.type !== frozen.type) behaviorErrors.push(`COMMON_EVENT_REORDERED:${index + 1}`);
    if (actual.key !== frozen.key || actual.code !== frozen.code) behaviorErrors.push(`COMMON_EVENT_IDENTITY:${index + 1}`);
  }
  validateCommonEventClock(receipt, setupErrors, behaviorErrors);

  let driverCalibration;
  try {
    if (row.id === "learned-row") {
      const pages = [...new Set((receipt.interactionWindows ?? []).map((window) => window.pageInstanceId))];
      if (pages.length !== 2) throw new Error("SETUP_LEARNED_CALIBRATION_SEGMENTS_INVALID");
      const pre = buildClockCalibration(receipt.calibrationSegments?.preReload?.driver?.pre,
        receipt.calibrationSegments?.preReload?.driver?.post, "driver-page");
      const post = buildClockCalibration(receipt.calibrationSegments?.postReload?.driver?.pre,
        receipt.calibrationSegments?.postReload?.driver?.post, "driver-page");
      driverCalibration = { driver: pre, byPageInstance: {
        [pages[0]]: { driver: pre }, [pages[1]]: { driver: post },
      } };
    } else {
      driverCalibration = buildClockCalibration(
        receipt.calibration?.driver?.pre,
        receipt.calibration?.driver?.post,
        "driver-page",
      );
    }
  } catch (error) {
    setupErrors.push(error instanceof Error ? error.message : "SETUP_INVALID_CLOCK_CALIBRATION");
  }

  const frozenSamples = expectedCommonSamples(row, expected);
  const samples = receipt.commonSamples ?? [];
  if (samples.length !== frozenSamples.length) {
    behaviorErrors.push(`COMMON_SAMPLE_COUNT:${samples.length}!=${frozenSamples.length}`);
  }
  const metrics = [];
  for (let index = 0; index < Math.min(samples.length, frozenSamples.length); index += 1) {
    const sample = samples[index];
    const frozen = frozenSamples[index];
    const event = events.find((candidate) => candidate.eventSequenceId === frozen.eventSequenceId);
    if (sample.stepId !== frozen.stepId || sample.sampleKind !== frozen.sampleKind
      || sample.eventSequenceId !== frozen.eventSequenceId
      || (event?.pageInstanceId !== undefined && sample.pageInstanceId !== event.pageInstanceId)) {
      behaviorErrors.push(`COMMON_SAMPLE_IDENTITY:${index + 1}`);
    }
    if (!event || !finite(sample.observedAt) || sample.observedAt < event?.normalizedEventAt) {
      behaviorErrors.push(`COMMON_SAMPLE_TIME:${frozen.stepId}`);
      continue;
    }
    const isCovering = frozen.sampleKind === "covering";
    const validOutcome = isCovering
      ? sample.outcome === "painted" || sample.outcome === "superseded"
      : sample.outcome === "terminal";
    if (!validOutcome) behaviorErrors.push(`COMMON_SAMPLE_OUTCOME:${frozen.stepId}`);
    let covering = sample;
    if (sample.outcome === "superseded") {
      const targetIndex = frozenSamples.findIndex((candidate) => candidate.stepId === sample.supersededByStepId);
      const target = samples[targetIndex];
      const targetFrozen = frozenSamples[targetIndex];
      if (targetIndex <= index || targetIndex - index > WEB06_THRESHOLDS.sustained.supersessionSequenceLag.max
        || target?.outcome !== "painted" || target?.observedAt !== sample.observedAt
        || targetFrozen?.sampleKind !== "covering"
        || !targetFrozen.expectedInput.startsWith(frozen.expectedInput)
        || targetFrozen.expectedInput === frozen.expectedInput) {
        behaviorErrors.push(`COMMON_SUPERSESSION_INVALID:${frozen.stepId}`);
      } else {
        covering = target;
      }
    }
    const sampleStable = sample.firstDomObserved !== undefined
      && sameJson(sample.firstDomObserved, sample.domObserved);
    const coveringStable = covering.firstDomObserved !== undefined
      && sameJson(covering.firstDomObserved, covering.domObserved);
    if (sample.stableDoubleRaf !== sampleStable || covering.stableDoubleRaf !== coveringStable
      || !sampleStable || !coveringStable
      || covering.domObserved?.logicalInputProjection !== (sample.outcome === "superseded"
        ? frozenSamples.find((candidate) => candidate.stepId === sample.supersededByStepId)?.expectedInput
        : frozen.expectedInput)) {
      behaviorErrors.push(`COMMON_DOM_ENDPOINT:${frozen.stepId}`);
    }
    const renderedInput = covering.domObserved?.renderedInput;
    if (typeof renderedInput !== "string" || covering.domObserved?.input !== renderedInput
      || covering.domObserved?.logicalInputProjection !== renderedInput.replaceAll(" ", "")) {
      behaviorErrors.push(`COMMON_DOM_INPUT_PROJECTION:${frozen.stepId}`);
    }
    const visibleCount = covering.domObserved?.pageShape?.visibleCount;
    const candidates = covering.domObserved?.candidates;
    if (!Number.isSafeInteger(visibleCount) || visibleCount < 0 || visibleCount > 6
      || !Array.isArray(candidates) || visibleCount !== candidates.length
      || candidates.some((candidate) => !candidate || typeof candidate.label !== "string"
        || typeof candidate.text !== "string" || typeof candidate.comment !== "string"
        || typeof candidate.source !== "string")
      || typeof covering.domObserved?.pageShape?.previousDisabled !== "boolean"
      || typeof covering.domObserved?.pageShape?.nextDisabled !== "boolean"
      || !Number.isSafeInteger(covering.domObserved?.pageShape?.highlightedIndex)
      || typeof covering.domObserved?.textareaValue !== "string"
      || !Number.isSafeInteger(covering.domObserved?.selectionStart)
      || !Number.isSafeInteger(covering.domObserved?.selectionEnd)) {
      behaviorErrors.push(`COMMON_PAGE_SHAPE:${frozen.stepId}`);
    }
    const observedDigest = covering.domObserved === undefined
      ? undefined
      : createHash("sha256").update(JSON.stringify(covering.domObserved), "utf8").digest("hex");
    if (!isSha256(covering.domFingerprintSha256) || covering.domFingerprintSha256 !== observedDigest) {
      behaviorErrors.push(`COMMON_DOM_FINGERPRINT:${frozen.stepId}`);
    }
    if (isCovering && !(covering.domObserved?.candidates?.length > 0)) {
      behaviorErrors.push(`COMMON_CANDIDATE_ENDPOINT:${frozen.stepId}`);
    }
    if (!finite(event.actualDriverDispatchAt) || !finite(event.requestedDriverDispatchAt)) {
      behaviorErrors.push(`COMMON_DRIVER_DISPATCH:${frozen.stepId}`);
      continue;
    }
    if (!driverCalibration) continue;
    let correctedDriver;
    try {
      correctedDriver = correctDriverTimestamp(event.actualDriverDispatchAt,
        calibrationForPage(driverCalibration, event.pageInstanceId).driver ?? driverCalibration,
        event.normalizedEventAt);
    } catch (error) {
      setupErrors.push(`${error instanceof Error ? error.message : "SETUP_INVALID_CLOCK_CALIBRATION"}:${frozen.stepId}`);
      continue;
    }
    if (correctedDriver.correctedAt + correctedDriver.uncertainty > event.normalizedEventAt) {
      setupErrors.push(`SETUP_INVALID_CLOCK_CALIBRATION:driver-event-order:${frozen.stepId}`);
    }
    metrics.push({
      ...frozen,
      eventToObservationMs: sample.observedAt - event.normalizedEventAt,
      driverDispatchToObservationUpperBoundMs:
        sample.observedAt - correctedDriver.correctedAt + correctedDriver.uncertainty,
    });
  }
  const firstEligible = samples.find((sample) => sample.sampleKind === "covering"
    && ["painted", "superseded"].includes(sample.outcome));
  const firstEligibleTarget = firstEligible?.outcome === "superseded"
    ? samples.find((sample) => sample.stepId === firstEligible.supersededByStepId)
    : firstEligible;
  if (firstEligibleTarget?.domObserved?.pageShape?.visibleCount !== 6) {
    behaviorErrors.push("COMMON_FIRST_MEASURED_ENDPOINT_NOT_SIX_ROWS");
  }
  const metricCoveringCount = metrics.filter((sample) => sample.sampleKind === "covering").length;
  const metricTerminalCount = metrics.filter((sample) => sample.sampleKind === "terminal").length;
  const timingComplete = driverCalibration !== undefined
    && !setupErrors.some(isClockCalibrationSetupError)
    && metrics.length === frozenSamples.length;
  if (timingComplete && metricCoveringCount !== row.expectedCoveringSamples) {
    behaviorErrors.push(`COMMON_COVERING_COUNT:${metricCoveringCount}!=${row.expectedCoveringSamples}`);
  }
  if (timingComplete && metricTerminalCount !== row.expectedTerminalSamples) {
    behaviorErrors.push(`COMMON_TERMINAL_COUNT:${metricTerminalCount}!=${row.expectedTerminalSamples}`);
  }

  behaviorErrors.push(...validateBehaviorPredicateSamples(receipt));

  const cadence = cadenceVerdict(receipt, row, behaviorErrors, setupErrors);
  const frames = frameVerdict(receipt, row, behaviorErrors, setupErrors, driverCalibration);
  const burst = burstRecoveryVerdict(receipt, row);
  behaviorErrors.push(...burst.behaviorErrors);
  const violations = [...protocolDisposition.thresholdViolations, ...burst.violations];
  const binding = row.binding === true || row.binding === "branch-b-only";
  if (binding && timingComplete) {
    const covering = metrics.filter((sample) => sample.sampleKind === "covering");
    const terminal = metrics.filter((sample) => sample.sampleKind === "terminal");
    for (const sample of covering) {
      if (sample.eventToObservationMs > WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs.max) violations.push("common-covering-max");
      if (sample.driverDispatchToObservationUpperBoundMs > WEB06_THRESHOLDS.sustained.driverDispatchToCoveringPaintUpperBoundMs.max) {
        violations.push("common-driver-covering-max");
      }
    }
    if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)
      && covering.length) {
      violations.push(...evaluateThresholdDistribution(
        covering.map((sample) => sample.eventToObservationMs),
        WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs,
        "common-covering",
      ).violations);
    }
    for (const sample of terminal) {
      const ceiling = sample.stressDeadline
        ? WEB06_THRESHOLDS.terminal.persistenceStressCompletionMs.max
        : WEB06_THRESHOLDS.terminal.eventToTerminalObservationMs.max;
      if (sample.eventToObservationMs > ceiling) violations.push("common-terminal-max");
      if (!sample.stressDeadline
        && sample.driverDispatchToObservationUpperBoundMs > WEB06_THRESHOLDS.terminal.driverDispatchToTerminalUpperBoundMs.max) {
        violations.push("common-driver-terminal-max");
      }
    }
  }
  const disposition = finalAttemptStatus({
    setupErrors,
    behaviorErrors,
    cadence,
    thresholdViolations: violations,
    frameRed: frames.frameRed,
    longTaskRed: frames.longTaskRed,
  });
  return {
    ...disposition,
    setupErrors,
    behaviorErrors,
    cadence,
    frameRed: frames.frameRed,
    longTaskRed: frames.longTaskRed,
    thresholdViolations: violations,
    metrics: { samples: metrics },
    calibration: { driver: driverCalibration },
  };
}

function validateTimelineShape(receipt, row, behaviorErrors) {
  const expected = expandScenarioExpectedTimeline(row.id);
  const events = receipt.events ?? [];
  const actions = receipt.actions ?? [];
  if (events.length !== row.expectedDomEventCount) behaviorErrors.push(`EVENT_COUNT:${events.length}!=${row.expectedDomEventCount}`);
  if (actions.length !== row.expectedActionCount) behaviorErrors.push(`ACTION_COUNT:${actions.length}!=${row.expectedActionCount}`);
  validateContiguousIds(events, "eventSequenceId", 1, "EVENT", behaviorErrors);
  validateContiguousIds(actions, "sequenceId", 1, "ACTION", behaviorErrors);
  if (new Set(actions.map((item) => item.actionId)).size !== actions.length) behaviorErrors.push("ACTION_DUPLICATE_ACTION_ID");
  validateWireIdentityNormalization(receipt, behaviorErrors);

  for (let index = 0; index < Math.min(events.length, expected.events.length); index += 1) {
    const actual = events[index];
    const frozen = expected.events[index];
    if (actual.stepId !== frozen.stepId || actual.type !== frozen.type) behaviorErrors.push(`EVENT_REORDERED:${index + 1}`);
    if (actual.key !== frozen.key || actual.code !== frozen.code) behaviorErrors.push(`EVENT_IDENTITY:${index + 1}`);
    if (actual.classification !== frozen.classification || actual.reason !== frozen.reason) {
      behaviorErrors.push(`EVENT_CLASSIFICATION:${index + 1}`);
    }
    if (!sameJson(actual.mappedActionIds, frozen.mappedActionIds)) behaviorErrors.push(`EVENT_ACTION_CARDINALITY:${index + 1}`);
  }
  for (let index = 0; index < Math.min(actions.length, expected.actions.length); index += 1) {
    const actual = actions[index];
    const frozen = expected.actions[index];
    if (actual.actionId !== frozen.actionId || actual.eventSequenceId !== frozen.eventSequenceId || actual.stepId !== frozen.stepId) {
      behaviorErrors.push(`ACTION_REORDERED:${index + 1}`);
    }
    if (actual.kind !== frozen.kind || actual.classification !== frozen.classification) behaviorErrors.push(`ACTION_CLASSIFICATION:${index + 1}`);
    if (actual.originKind !== frozen.originKind || actual.originReason !== frozen.originReason
      || actual.causedByActionId !== frozen.causedByActionId
      || actual.causedBySequenceId !== frozen.causedBySequenceId
      || actual.causedByEventSequenceId !== frozen.causedByEventSequenceId) {
      behaviorErrors.push(`ACTION_ORIGIN:${index + 1}`);
    }
    if (!sameJson(actual.args, frozen.args)) behaviorErrors.push(`ACTION_ARGUMENTS:${index + 1}`);
    if (!hasOwn(ACTION_REGISTRY, actual.kind)) behaviorErrors.push(`ACTION_UNCLASSIFIED:${index + 1}`);
  }
  return expected;
}

function wireId(prefix, sequenceId) {
  return `web06-${prefix}-${String(sequenceId).padStart(8, "0")}`;
}

export function normalizeWireActionArgs(kind, wireArgs, commitments = {}) {
  if (!Array.isArray(wireArgs)) throw new Error("WEB06_WIRE_ACTION_ARGS_INVALID");
  if (kind === "importUserdb") {
    if (wireArgs.length !== 1 || typeof wireArgs[0] !== "string") {
      throw new Error("WEB06_IMPORT_USERDB_ARGS_INVALID");
    }
    if (wireArgs[0] !== "<web06-redacted:userdb-text>") {
      throw new Error("WEB06_IMPORT_USERDB_RAW_BYTES_FORBIDDEN");
    }
    if (!isSha256(commitments.userdbTextSha256)) throw new Error("WEB06_IMPORT_USERDB_COMMITMENT_MISSING");
    return [`sha256:${commitments.userdbTextSha256}`];
  }
  if (kind === "customizeValue") {
    if (wireArgs.length !== 3 || wireArgs[2] !== "<web06-redacted:customize-value>"
      || !isSha256(commitments.customizeValueSha256)) {
      throw new Error("WEB06_CUSTOMIZE_VALUE_COMMITMENT_INVALID");
    }
    return [wireArgs[0], wireArgs[1], `sha256:${commitments.customizeValueSha256}`];
  }
  if (kind === "customize") {
    if (wireArgs.length !== 1 || !wireArgs[0] || typeof wireArgs[0] !== "object" || Array.isArray(wireArgs[0])) {
      throw new Error("WEB06_CUSTOMIZE_ARGS_INVALID");
    }
    const preferences = structuredClone(wireArgs[0]);
    const excluded = preferences.dictionaryExclude;
    if (excluded !== undefined) {
      if (!excluded || typeof excluded !== "object" || Array.isArray(excluded)
        || excluded.kind !== "web06-redacted:dictionary-exclude"
        || !Number.isSafeInteger(excluded.count) || excluded.count < 0
        || !isSha256(commitments.dictionaryExcludeSha256)) {
        throw new Error("WEB06_DICTIONARY_EXCLUDE_COMMITMENT_INVALID");
      }
      preferences.dictionaryExclude = {
        kind: excluded.kind,
        count: excluded.count,
        sha256: commitments.dictionaryExcludeSha256,
      };
    }
    return [preferences];
  }
  return structuredClone(wireArgs);
}

function validateNormalizedArgs(action, behaviorErrors) {
  if (!Array.isArray(action.wireArgs)) {
    behaviorErrors.push(`WIRE_ACTION_ARGS_MISSING:${action.sequenceId}`);
    return;
  }
  let normalized;
  try {
    normalized = normalizeWireActionArgs(action.kind, action.wireArgs, action.argumentCommitments);
  } catch {
    behaviorErrors.push(`WIRE_ACTION_ARGS_NORMALIZATION:${action.sequenceId}`);
    return;
  }
  if (!sameJson(normalized, action.args)) {
    behaviorErrors.push(`WIRE_ACTION_ARGS_NORMALIZATION:${action.sequenceId}`);
  }
}

function validateWireIdentityNormalization(receipt, behaviorErrors) {
  const eventStart = receipt.protocolWindow?.receiptWindowStartEventSequenceId;
  const actionStart = receipt.protocolWindow?.receiptWindowStartActionSequenceId;
  if (!Number.isSafeInteger(eventStart) || eventStart < 1
    || !Number.isSafeInteger(actionStart) || actionStart < 1) {
    behaviorErrors.push("WIRE_WINDOW_START_INVALID");
    return;
  }
  const expected = expandScenarioExpectedTimeline(receipt.scenarioId);
  const protocolWindowSegments = receipt.protocolWindowSegments ?? [{
    pageInstanceId: undefined,
    receiptWindowStartEventSequenceId: eventStart,
    receiptWindowStartActionSequenceId: actionStart,
  }];
  if (!Array.isArray(protocolWindowSegments)
    || protocolWindowSegments.length !== (receipt.scenarioId === "learned-row" ? 2 : 1)
    || protocolWindowSegments.some((segment) => !Number.isSafeInteger(segment.receiptWindowStartEventSequenceId)
      || !Number.isSafeInteger(segment.receiptWindowStartActionSequenceId))
    || (receipt.scenarioId === "learned-row"
      && new Set(protocolWindowSegments.map((segment) => segment.pageInstanceId)).size !== 2)) {
    behaviorErrors.push("WIRE_WINDOW_SEGMENTS_INVALID");
    return;
  }
  const windowForPage = (pageInstanceId) => protocolWindowSegments.find((segment) =>
    segment.pageInstanceId === undefined || segment.pageInstanceId === pageInstanceId);
  const actionByLocalId = new Map((receipt.actions ?? []).map((action) => [action.actionId, action]));
  const eventByLocalSequence = new Map((receipt.events ?? []).map((event) => [event.eventSequenceId, event]));
  const eventOrdinalsByPage = new Map();
  for (const event of receipt.events ?? []) {
    const frozen = expected.events[event.eventSequenceId - 1];
    if (frozen?.type === "browser-lifecycle") {
      if (event.originOwner !== "harness-browser-lifecycle" || event.wireEventSequenceId !== undefined
        || event.wireEventId !== undefined || event.wireIdentity !== undefined
        || event.wireLinkedActionIds !== undefined) {
        behaviorErrors.push(`WIRE_EXTERNAL_EVENT_INVALID:${event.eventSequenceId}`);
      }
      continue;
    }
    const sourceWindow = windowForPage(event.pageInstanceId);
    const ordinal = eventOrdinalsByPage.get(event.pageInstanceId) ?? 0;
    if (!sourceWindow) {
      behaviorErrors.push(`WIRE_EVENT_REALM:${event.eventSequenceId}`);
      continue;
    }
    const sequenceId = sourceWindow.receiptWindowStartEventSequenceId + ordinal;
    eventOrdinalsByPage.set(event.pageInstanceId, ordinal + 1);
    const eventId = wireId("event", sequenceId);
    if (event.wireEventSequenceId !== sequenceId || event.wireEventId !== eventId
      || event.wireIdentity?.eventSequenceId !== sequenceId || event.wireIdentity?.eventId !== eventId) {
      behaviorErrors.push(`WIRE_EVENT_OFFSET:${event.eventSequenceId}`);
    }
    for (const field of ["type", "key", "code", "classification", "reason"]) {
      if (event.wireIdentity?.[field] !== event[field]) behaviorErrors.push(`WIRE_EVENT_IDENTITY:${event.eventSequenceId}:${field}`);
    }
    if (event.wireIdentity?.timeStamp !== event.eventTimestamp
      || event.wireIdentity?.eventDeliveredAt !== event.eventDeliveredAt) {
      behaviorErrors.push(`WIRE_EVENT_TIMESTAMPS:${event.eventSequenceId}`);
    }
    const expectedWireActions = (event.mappedActionIds ?? []).map((localId) => actionByLocalId.get(localId)?.wireActionId);
    if (!sameJson(event.wireLinkedActionIds, expectedWireActions)) {
      behaviorErrors.push(`WIRE_EVENT_ACTION_LINKS:${event.eventSequenceId}`);
    }
  }
  const actionOrdinalsByPage = new Map();
  for (const action of receipt.actions ?? []) {
    const sourceWindow = windowForPage(action.pageInstanceId);
    const ordinal = actionOrdinalsByPage.get(action.pageInstanceId) ?? 0;
    if (!sourceWindow) {
      behaviorErrors.push(`WIRE_ACTION_REALM:${action.sequenceId}`);
      continue;
    }
    const sequenceId = sourceWindow.receiptWindowStartActionSequenceId + ordinal;
    actionOrdinalsByPage.set(action.pageInstanceId, ordinal + 1);
    const actionId = wireId("action", sequenceId);
    const sourceEvent = action.eventSequenceId === undefined ? undefined : eventByLocalSequence.get(action.eventSequenceId);
    const wireEventSequenceId = sourceEvent?.wireEventSequenceId;
    const wireEventId = sourceEvent?.wireEventId;
    const sourceCauseAction = action.causedByActionId === undefined ? undefined : actionByLocalId.get(action.causedByActionId);
    const wireCauseSequenceId = sourceCauseAction?.wireSequenceId;
    const wireCauseActionId = sourceCauseAction?.wireActionId;
    const sourceCauseEvent = action.causedByEventSequenceId === undefined
      ? undefined : eventByLocalSequence.get(action.causedByEventSequenceId);
    const wireCauseEventSequenceId = sourceCauseEvent?.wireEventSequenceId;
    const wireCauseEventId = sourceCauseEvent?.wireEventId;
    if (action.wireSequenceId !== sequenceId || action.wireActionId !== actionId
      || action.wireIdentity?.sequenceId !== sequenceId || action.wireIdentity?.actionId !== actionId
      || action.wireIdentity?.eventSequenceId !== wireEventSequenceId || action.wireIdentity?.eventId !== wireEventId
      || action.wireIdentity?.causedBySequenceId !== wireCauseSequenceId
      || action.wireIdentity?.causedByActionId !== wireCauseActionId
      || action.wireIdentity?.causedByEventSequenceId !== wireCauseEventSequenceId
      || action.wireIdentity?.causedByEventId !== wireCauseEventId) {
      behaviorErrors.push(`WIRE_ACTION_OFFSET:${action.sequenceId}`);
    }
    if (!action.returnedWireIdentity || !sameJson(action.wireIdentity, action.returnedWireIdentity)) {
      behaviorErrors.push(`WIRE_RETURNED_IDENTITY:${action.sequenceId}`);
    }
    const fields = [
      ["compositionEpochId", "compositionEpochId"],
      ["supersessionSubRunId", "supersessionSubRunId"],
      ["actionClass", "classification"],
      ["supersedable", "supersedable"],
      ["actionEnqueuedAt", "actionEnqueuedAt"],
      ["mainQueueDepthAtEnqueue", "mainQueueDepth"],
      ["workerSentAt", "workerSentAt"],
      ["workerDispatchDepth", "workerDispatchDepth"],
      ["originKind", "originKind"],
      ["originReason", "originReason"],
    ];
    for (const [wireField, localField] of fields) {
      if (action.wireIdentity?.[wireField] !== action[localField]) {
        behaviorErrors.push(`WIRE_ACTION_IDENTITY:${action.sequenceId}:${wireField}`);
      }
    }
    validateNormalizedArgs(action, behaviorErrors);
  }
}

function calibrationFor(receipt, setupErrors) {
  try {
    if (receipt.scenarioId === "learned-row") {
      const pageInstances = [...new Set((receipt.interactionWindows ?? []).map((window) => window.pageInstanceId))];
      if (pageInstances.length !== 2 || !receipt.calibrationSegments?.preReload || !receipt.calibrationSegments?.postReload) {
        throw new Error("SETUP_LEARNED_CALIBRATION_SEGMENTS_INVALID");
      }
      const segment = (value) => ({
        driver: buildClockCalibration(value?.driver?.pre, value?.driver?.post, "driver-page"),
        worker: buildClockCalibration(value?.worker?.pre, value?.worker?.post, "main-worker"),
      });
      const byPageInstance = {
        [pageInstances[0]]: segment(receipt.calibrationSegments.preReload),
        [pageInstances[1]]: segment(receipt.calibrationSegments.postReload),
      };
      return { ...byPageInstance[pageInstances[0]], byPageInstance };
    }
    const base = {
      driver: buildClockCalibration(receipt.calibration?.driver?.pre, receipt.calibration?.driver?.post, "driver-page"),
      worker: buildClockCalibration(receipt.calibration?.worker?.pre, receipt.calibration?.worker?.post, "main-worker"),
    };
    return base;
  } catch (error) {
    setupErrors.push(error instanceof Error ? error.message : "SETUP_INVALID_CLOCK_CALIBRATION");
    return null;
  }
}

function calibrationForPage(calibrations, pageInstanceId) {
  return calibrations?.byPageInstance?.[pageInstanceId] ?? calibrations;
}

function crossContextAction(action, event, calibrations, setupErrors) {
  try {
    const workerReference = (action.workerSentAt + action.mainResponseReceivedAt) / 2;
    const receive = correctWorkerTimestamp(action.workerMessageReceivedAt, calibrations.worker, workerReference);
    const start = correctWorkerTimestamp(action.workerActionStartedAt, calibrations.worker, workerReference);
    const finish = correctWorkerTimestamp(action.workerFinishedAt, calibrations.worker, workerReference);
    const driver = action.driverDispatchAt === undefined
      ? null
      : correctDriverTimestamp(action.driverDispatchAt, calibrations.driver, event.normalizedEventAt);
    if (action.workerMessageReceivedAt > action.workerActionStartedAt || action.workerActionStartedAt > action.workerFinishedAt) {
      setupErrors.push(`SETUP_INVALID_CLOCK_CALIBRATION:worker-same-realm-order:${action.sequenceId}`);
    }
    if (action.workerSentAt > receive.correctedAt - receive.uncertainty) {
      setupErrors.push(`SETUP_INVALID_CLOCK_CALIBRATION:send-receive-order:${action.sequenceId}`);
    }
    if (action.actionEnqueuedAt > start.correctedAt - start.uncertainty) {
      setupErrors.push(`SETUP_INVALID_CLOCK_CALIBRATION:enqueue-start-order:${action.sequenceId}`);
    }
    if (finish.correctedAt + finish.uncertainty > action.mainResponseReceivedAt) {
      setupErrors.push(`SETUP_INVALID_CLOCK_CALIBRATION:finish-response-order:${action.sequenceId}`);
    }
    return { receive, start, finish, driver };
  } catch (error) {
    setupErrors.push(`${error instanceof Error ? error.message : "SETUP_INVALID_CLOCK_CALIBRATION"}:${action.sequenceId}`);
    return null;
  }
}

function validateWorkerSpans(action, mode, behaviorErrors) {
  const spanRecord = action.workerSpans ?? {};
  if (MINIMAL_RECEIPT_MODES.has(mode)) {
    if (Object.values(spanRecord).some((span) => span !== null && span !== undefined)) {
      behaviorErrors.push(`MINIMAL_RAW_WORKER_SPAN_PRESENT:${action.sequenceId}`);
    }
    if (action.persistenceRan !== undefined) {
      behaviorErrors.push(`MINIMAL_RAW_PERSISTENCE_DISPOSITION_PRESENT:${action.sequenceId}`);
    }
    return;
  }
  if (!FULL_RECEIPT_MODES.has(mode)) return;
  for (const required of ["abi", "responseExtract", "jsonParse", "adapterTranslate", "persistence"]) {
    if (!Object.hasOwn(spanRecord, required)) behaviorErrors.push(`WORKER_SPAN_DECLARATION_MISSING:${action.sequenceId}:${required}`);
  }
  if (typeof action.persistenceRan !== "boolean") {
    behaviorErrors.push(`WORKER_PERSISTENCE_DISPOSITION_MISSING:${action.sequenceId}`);
  } else if (action.persistenceRan !== Boolean(spanRecord.persistence)) {
    behaviorErrors.push(`WORKER_PERSISTENCE_SPAN_DISAGREES:${action.sequenceId}`);
  }
  const spans = Object.entries(spanRecord)
    .filter(([, span]) => span !== null && span !== undefined)
    .map(([name, span]) => ({ name, ...span }))
    .sort((left, right) => left.start - right.start);
  for (let index = 0; index < spans.length; index += 1) {
    const span = spans[index];
    if (!finite(span.start) || !finite(span.end) || span.end < span.start) {
      behaviorErrors.push(`WORKER_SPAN_INVALID:${action.sequenceId}:${span.name}`);
      continue;
    }
    if (span.start < action.workerActionStartedAt || span.end > action.workerFinishedAt) {
      behaviorErrors.push(`WORKER_SPAN_OUTSIDE_ACTION:${action.sequenceId}:${span.name}`);
    }
    if (index > 0 && span.start < spans[index - 1].end) {
      behaviorErrors.push(`WORKER_SPAN_OVERLAP:${action.sequenceId}:${spans[index - 1].name}:${span.name}`);
    }
  }
}

function validateSameRealmAction(action, event, behaviorErrors) {
  const ordered = [
    ["actionEnqueuedAt", event.eventDeliveredAt],
    ["workerSentAt", action.actionEnqueuedAt],
    ["mainResponseReceivedAt", action.workerSentAt],
    ["responseMappingStartedAt", action.mainResponseReceivedAt],
    ["responseMappingFinishedAt", action.responseMappingStartedAt],
  ];
  if (action.stateUpdateScheduledAt !== undefined) {
    ordered.push(["stateUpdateScheduledAt", action.responseMappingFinishedAt]);
    ordered.push(["stateCommittedAt", action.stateUpdateScheduledAt]);
  }
  for (const [field, previous] of ordered) {
    if (!finite(action[field]) || action[field] < previous) behaviorErrors.push(`ACTION_SAME_REALM_ORDER:${action.sequenceId}:${field}`);
  }
  if (action.outcome === "painted") {
    for (const field of ["stateUpdateScheduledAt", "stateCommittedAt", "paintObservedAt"]) {
      if (!finite(action[field])) behaviorErrors.push(`PAINT_CHAIN_MISSING:${action.sequenceId}:${field}`);
    }
    if (finite(action.stateCommittedAt) && finite(action.paintObservedAt)
      && action.paintObservedAt < action.stateCommittedAt) {
      behaviorErrors.push(`ACTION_SAME_REALM_ORDER:${action.sequenceId}:paintObservedAt`);
    }
  }
  if (!finite(event.actualDriverDispatchAt) || !finite(event.requestedDriverDispatchAt)
    || action.driverDispatchAt !== event.actualDriverDispatchAt) {
    behaviorErrors.push(`ACTION_DRIVER_IDENTITY:${action.sequenceId}`);
  }
  pushIf(behaviorErrors, !Number.isInteger(action.mainQueueDepth) || action.mainQueueDepth < 0, `ACTION_QUEUE_DEPTH:${action.sequenceId}`);
  pushIf(behaviorErrors, !Number.isInteger(action.workerDispatchDepth) || action.workerDispatchDepth < 0, `ACTION_WORKER_DEPTH:${action.sequenceId}`);
}

function rawActionSequence(action) {
  return action.rawActionSequence ?? action.engineRaw?.rawActionSequence;
}

function logicalInput(action) {
  return action.logicalInput ?? action.engineRaw?.logicalInput ?? action.presentationExpected?.input;
}

const RESPONSE_BEARING_ACTIONS = new Set([
  "processKey",
  "stageAi",
  "selectCandidate",
  "deleteCandidate",
  "flipPage",
]);

function rawRecord(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`WEB06_RAW_${label}_OBJECT`);
  return value;
}

function independentlyProjectEngineRaw(rawJson) {
  const root = rawRecord(JSON.parse(rawJson), "RESPONSE");
  if (typeof root.handled !== "boolean" || !Array.isArray(root.commits)
    || root.commits.some((value) => typeof value !== "string")
    || !(root.context === null || (root.context && typeof root.context === "object" && !Array.isArray(root.context)))
    || !(root.status === null || (root.status && typeof root.status === "object" && !Array.isArray(root.status)))) {
    throw new Error("WEB06_RAW_RESPONSE_SHAPE");
  }
  const committed = root.commits.length ? root.commits.join("") : undefined;
  if (!root.handled) {
    return { success: false, input: "", page: 0, isLastPage: true, highlightedIndex: -1, candidates: [], status: null,
      ...(committed === undefined ? {} : { committed }) };
  }
  const context = root.context;
  if (context !== null && context.preedit !== "") {
    if (typeof context.preedit !== "string" || !Number.isSafeInteger(context.page_no)
      || typeof context.is_last_page !== "boolean" || !Number.isSafeInteger(context.highlighted)
      || !Array.isArray(context.select_labels) || !Array.isArray(context.candidates)) {
      throw new Error("WEB06_RAW_CONTEXT_SHAPE");
    }
    const candidates = context.candidates.map((candidate, index) => {
      const value = rawRecord(candidate, `CANDIDATE_${index}`);
      if (typeof value.text !== "string" || typeof value.comment !== "string"
        || (value.source !== undefined && typeof value.source !== "string")) {
        throw new Error("WEB06_RAW_CANDIDATE_SHAPE");
      }
      const label = context.select_labels[index];
      if (label !== undefined && typeof label !== "string") throw new Error("WEB06_RAW_LABEL_SHAPE");
      return { ...(label === undefined ? {} : { label }), text: value.text, comment: value.comment,
        ...(value.source === undefined ? {} : { source: value.source }) };
    });
    return {
      success: true,
      input: context.preedit,
      page: context.page_no,
      isLastPage: context.is_last_page,
      highlightedIndex: context.highlighted,
      candidates,
      status: root.status,
      ...(committed === undefined ? {} : { committed }),
    };
  }
  return { success: true, input: "", page: 0, isLastPage: true, highlightedIndex: -1, candidates: [],
    status: root.status, ...(committed === undefined ? {} : { committed }) };
}

function presentationProjection(value) {
  if (!value) return undefined;
  return {
    success: true,
    input: value.input,
    page: value.page,
    isLastPage: value.isLastPage,
    highlightedIndex: value.highlightedIndex,
    candidates: value.candidates,
    status: value.status ?? null,
  };
}

function validateFingerprints(action, mode, expectedTerminalKind, behaviorErrors) {
  if (!Array.isArray(rawActionSequence(action))) {
    behaviorErrors.push(`ACTION_RAW_SEQUENCE:${action.sequenceId}`);
  }
  const raw = action.engineRaw;
  if (MINIMAL_RECEIPT_MODES.has(mode)) {
    if (raw !== undefined) behaviorErrors.push(`MINIMAL_ENGINE_RAW_PRESENT:${action.sequenceId}`);
    if (action.engineRawProof?.availability !== "not-collected"
      || action.engineRawProof?.reason !== "minimal-content-free"
      || action.engineRawProof?.action !== action.kind
      || action.engineRawProof?.rawFingerprint !== undefined) {
      behaviorErrors.push(`MINIMAL_ENGINE_RAW_PROOF_INVALID:${action.sequenceId}`);
    }
    if (action.presentationExpected !== undefined || action.domObserved !== undefined) {
      behaviorErrors.push(`MINIMAL_PRESENTATION_CONTENT_PRESENT:${action.sequenceId}`);
    }
  } else if (FULL_RECEIPT_MODES.has(mode)) {
    const responseBearing = RESPONSE_BEARING_ACTIONS.has(action.kind);
    const proof = action.engineRawProof;
    if (!proof || proof.action !== action.kind) behaviorErrors.push(`ENGINE_RAW_PROOF_IDENTITY:${action.sequenceId}`);
    if (!responseBearing) {
      if (raw !== undefined || proof?.availability !== "not-applicable"
        || !["action-has-no-runtime-response", "action-failed-before-runtime-response"].includes(proof?.reason)) {
        behaviorErrors.push(`ENGINE_RAW_NOT_APPLICABLE_INVALID:${action.sequenceId}`);
      }
    } else if (!raw || raw.actionKind !== action.kind || raw.compositionEpochId !== action.compositionEpochId
      || raw.supersessionSubRunId !== action.supersessionSubRunId) {
      behaviorErrors.push(`ENGINE_RAW_IDENTITY:${action.sequenceId}`);
    } else if (!Array.isArray(raw.rawActionSequence)) {
      behaviorErrors.push(`ENGINE_RAW_ACTION_SEQUENCE:${action.sequenceId}`);
    } else if (typeof raw.rawResponseJson !== "string" || !isSha256(raw.rawResponseSha256)
      || createHash("sha256").update(raw.rawResponseJson, "utf8").digest("hex") !== raw.rawResponseSha256) {
      behaviorErrors.push(`ENGINE_RAW_PREPROJECTION_BYTES:${action.sequenceId}`);
    } else {
      try {
        const projected = independentlyProjectEngineRaw(raw.rawResponseJson);
        const presentation = presentationProjection(action.presentationExpected);
        const projectedWithoutCommit = { ...projected };
        delete projectedWithoutCommit.committed;
        if (!sameJson(projectedWithoutCommit, presentation)) {
          behaviorErrors.push(`ENGINE_RAW_PROJECTION_DISAGREES:${action.sequenceId}`);
        }
        if (projected.committed !== undefined
          && projected.committed !== action.commitFingerprint?.exactCommitText) {
          behaviorErrors.push(`ENGINE_RAW_COMMIT_DISAGREES:${action.sequenceId}`);
        }
      } catch {
        behaviorErrors.push(`ENGINE_RAW_PREPROJECTION_JSON:${action.sequenceId}`);
      }
    }
    if (Array.isArray(action.rawActionSequence) && !sameJson(action.rawActionSequence, raw?.rawActionSequence)) {
      behaviorErrors.push(`ENGINE_RAW_SEQUENCE_DISAGREES:${action.sequenceId}`);
    }
  }
  if (expectedTerminalKind === "presentation") {
    for (const field of ["beforeDomDigest", "adapterProjectionDigest", "presentationExpectedDigest", "domObservedDigest", "presentationDigest"]) {
      if (!/^[0-9a-f]{32}$/.test(action[field] ?? "")) behaviorErrors.push(`PRESENTATION_DIGEST_INVALID:${action.sequenceId}:${field}`);
    }
    if (FULL_RECEIPT_MODES.has(mode)) {
      if (presentationStateDigest(action.presentationExpected) !== action.presentationExpectedDigest
        || presentationStateDigest(action.domObserved) !== action.domObservedDigest
        || action.presentationDigest !== action.domObservedDigest
        || action.afterDomDigest !== action.domObservedDigest) {
        behaviorErrors.push(`PRESENTATION_DIGEST_RECOMPUTE:${action.sequenceId}`);
      }
    }
  } else if (expectedTerminalKind === "lifecycle") {
    const lifecycle = action.lifecycleEffect;
    if (!lifecycle || !["listener", "engine-state", "engine-persistence", "ui-userdb-refresh",
      "ui-diagnostic-refresh", "cache-invalidation", "error"].includes(lifecycle.effect)
      || !/^[0-9a-f]{32}$/.test(lifecycle.effectDigest ?? "")
      || !/^[0-9a-f]{32}$/.test(lifecycle.workerEffectDigest ?? "")
      || (lifecycle.mainEffectDigest !== undefined && !/^[0-9a-f]{32}$/.test(lifecycle.mainEffectDigest))
      || !Number.isSafeInteger(lifecycle.listenerEffectCount) || lifecycle.listenerEffectCount < 0
      || typeof lifecycle.persistenceCompleted !== "boolean") {
      behaviorErrors.push(`LIFECYCLE_EFFECT_INVALID:${action.sequenceId}`);
    }
    if (action.presentationExpected !== undefined || action.domObserved !== undefined) {
      behaviorErrors.push(`LIFECYCLE_PRESENTATION_CONTENT_PRESENT:${action.sequenceId}`);
    }
  }
  if (["painted", "committed", "barrier-completed"].includes(action.outcome)) {
    if (expectedTerminalKind === "presentation" && FULL_RECEIPT_MODES.has(mode)
      && (!action.presentationExpected || !action.domObserved || !sameJson(action.presentationExpected, action.domObserved))) {
      behaviorErrors.push(`PRESENTATION_DOM_MISMATCH:${action.sequenceId}`);
    }
  }
  if (action.outcome === "committed") {
    const commit = action.commitFingerprint;
    if (!commit || typeof commit.exactCommitText !== "string" || typeof commit.textareaValue !== "string"
      || !Number.isInteger(commit.selectionStart) || !Number.isInteger(commit.selectionEnd)
      || commit.visibleComposition !== false) {
      behaviorErrors.push(`COMMIT_FINGERPRINT_INVALID:${action.sequenceId}`);
    }
  }
}

function validateOutcomes(actions, expected, row, mode, behaviorErrors) {
  for (let index = 0; index < Math.min(actions.length, expected.actions.length); index += 1) {
    const action = actions[index];
    const frozen = expected.actions[index];
    const step = row.steps.find((candidate) => candidate.id === frozen.stepId);
    const terminalStrategy = ACTION_REGISTRY[frozen.kind]?.terminalStrategy;
    if (action.terminalKind !== terminalStrategy) {
      behaviorErrors.push(`ACTION_TERMINAL_KIND:${action.sequenceId}`);
    }
    if (!ALLOWED_OUTCOMES.has(action.outcome)) behaviorErrors.push(`ACTION_OUTCOME_UNKNOWN:${action.sequenceId}`);
    if (frozen.expectedOutcome === "failure") {
      if (action.outcome !== "failure" || terminalStrategy !== "lifecycle") {
        behaviorErrors.push(`EXPECTED_ACTION_FAILURE_MISSING:${action.sequenceId}`);
      }
    } else if (action.outcome === "failure") {
      behaviorErrors.push(`ACTION_EXPLICIT_FAILURE:${action.sequenceId}`);
    }
    if (terminalStrategy === "lifecycle" && !["barrier-completed", "failure"].includes(action.outcome)) {
      behaviorErrors.push(`LIFECYCLE_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (terminalStrategy === "presentation" && ["barrier-completed"].includes(action.outcome)) {
      behaviorErrors.push(`PRESENTATION_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (frozen.supersedable === true && !["painted", "superseded"].includes(action.outcome)) {
      behaviorErrors.push(`PRINTABLE_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (step?.sample === "terminal" && isTerminalOwner(step, frozen, expected.actions)
      && !["painted", "committed", "barrier-completed", "failure"].includes(action.outcome)) {
      behaviorErrors.push(`TERMINAL_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (frozen.classification === "stateful-barrier" && action.outcome === "superseded") {
      behaviorErrors.push(`SUPERSEDED_BARRIER:${action.sequenceId}`);
    }
    if (frozen.classification === "stateful-barrier"
      && !["painted", "committed", "barrier-completed", "failure"].includes(action.outcome)) {
      behaviorErrors.push(`BARRIER_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (action.outcome === "processed-no-visual-change") {
      if (frozen.classification === "stateful-barrier" || action.beforeDomDigest !== action.afterDomDigest) {
        behaviorErrors.push(`NO_VISUAL_CHANGE_INVALID:${action.sequenceId}`);
      }
    }
    if (action.outcome === "superseded" && !frozen.supersedable) {
      behaviorErrors.push(`NONPRINTABLE_SUPERSEDED:${action.sequenceId}`);
    }
    if (terminalStrategy === "presentation" && frozen.supersedable !== true
      && action.outcome === "processed-no-visual-change") {
      behaviorErrors.push(`INTERACTIVE_NO_VISUAL_CHANGE_FORBIDDEN:${action.sequenceId}`);
    }
    validateFingerprints(action, mode, terminalStrategy, behaviorErrors);
  }
}

function terminalOwnerForStep(step, expectedActions) {
  const actions = expectedActions.filter((action) => action.stepId === step.id);
  if (actions.length === 0) return undefined;
  const foreground = actions.filter((action) => action.background !== true);
  if (foreground.length !== actions.length) return foreground[0] ?? actions[0];
  return foreground.at(-1);
}

function isTerminalOwner(step, action, expectedActions) {
  return terminalOwnerForStep(step, expectedActions)?.sequenceId === action.sequenceId;
}

export function terminalOwnerSequenceId(scenarioId, stepId) {
  if (!hasOwn(SCENARIO_REGISTRY, scenarioId)) throw new Error(`WEB06_UNKNOWN_SCENARIO:${scenarioId}`);
  const row = SCENARIO_REGISTRY[scenarioId];
  const step = row.steps.find((candidate) => candidate.id === stepId);
  if (!step || step.sample !== "terminal") throw new Error(`WEB06_UNKNOWN_TERMINAL_STEP:${stepId}`);
  const expected = expandScenarioExpectedTimeline(scenarioId);
  const owner = terminalOwnerForStep(step, expected.actions);
  if (!owner) throw new Error(`WEB06_TERMINAL_OWNER_MISSING:${stepId}`);
  return owner.sequenceId;
}

function resolveSupersession(actions, action, behaviorErrors) {
  const target = actions.find((candidate) => candidate.sequenceId === action.supersededBySequenceId);
  if (!target || target.sequenceId <= action.sequenceId) {
    behaviorErrors.push(`SUPERSESSION_ORPHAN_OR_BACKWARD:${action.sequenceId}`);
    return null;
  }
  if (target.compositionEpochId !== action.compositionEpochId || target.supersessionSubRunId !== action.supersessionSubRunId) {
    behaviorErrors.push(`SUPERSESSION_CROSSES_BOUNDARY:${action.sequenceId}`);
  }
  if (target.outcome !== "painted" || !finite(target.paintObservedAt)) behaviorErrors.push(`SUPERSESSION_TARGET_NOT_PAINTED:${action.sequenceId}`);
  if (target.classification !== "native-key" || target.supersedable !== true) {
    behaviorErrors.push(`SUPERSESSION_TARGET_IS_BARRIER:${action.sequenceId}`);
  }
  if (!strictPrefix(rawActionSequence(action), rawActionSequence(target))
    || !String(logicalInput(target) ?? "").startsWith(String(logicalInput(action) ?? ""))) {
    behaviorErrors.push(`SUPERSESSION_NON_PREFIX:${action.sequenceId}`);
  }
  const between = actions.filter((candidate) => candidate.sequenceId > action.sequenceId && candidate.sequenceId < target.sequenceId);
  if (between.some((candidate) => candidate.classification !== "native-key"
    || candidate.supersedable !== true || candidate.outcome === "failure")) {
    behaviorErrors.push(`SUPERSESSION_SPANS_BARRIER_OR_FAILURE:${action.sequenceId}`);
  }
  const firstExactPaint = actions.find((candidate) =>
    candidate.sequenceId > action.sequenceId
    && candidate.compositionEpochId === action.compositionEpochId
    && candidate.supersessionSubRunId === action.supersessionSubRunId
    && candidate.outcome === "painted"
    && strictPrefix(rawActionSequence(action), rawActionSequence(candidate)));
  if (firstExactPaint && firstExactPaint.sequenceId !== target.sequenceId) behaviorErrors.push(`SUPERSESSION_NOT_EARLIEST_COVERING_PAINT:${action.sequenceId}`);
  const lag = target.sequenceId - action.sequenceId;
  if (lag > WEB06_THRESHOLDS.sustained.supersessionSequenceLag.max) behaviorErrors.push(`SUPERSESSION_LAG:${action.sequenceId}:${lag}`);
  return target;
}

export function validateSupersessionGraph(actions) {
  const errors = [];
  for (const action of actions) {
    if (action.outcome === "superseded") resolveSupersession(actions, action, errors);
    if (action.classification === "stateful-barrier" && action.outcome === "superseded") {
      errors.push(`SUPERSEDED_BARRIER:${action.sequenceId}`);
    }
  }
  return { pass: errors.length === 0, errors };
}

function recomputeActions(receipt, row, expected, calibrations, setupErrors, behaviorErrors) {
  const eventById = new Map(receipt.events.map((event) => [event.eventSequenceId, event]));
  const derivedActions = [];
  for (let index = 0; index < receipt.actions.length; index += 1) {
    const action = receipt.actions[index];
    const frozen = expected.actions[index];
    const event = eventById.get(action.eventSequenceId ?? action.causedByEventSequenceId);
    if (!event) continue;
    const corrected = crossContextAction(action, event, calibrationForPage(calibrations, event.pageInstanceId), setupErrors);
    if (!corrected) continue;
    if (corrected.driver && corrected.driver.correctedAt + corrected.driver.uncertainty > event.normalizedEventAt) {
      setupErrors.push(`SETUP_INVALID_CLOCK_CALIBRATION:driver-event-order:${action.sequenceId}`);
    }
    const values = {
      eventDeliveryMs: event.eventDeliveredAt - event.normalizedEventAt,
      eventHandlerEnqueueMs: action.actionEnqueuedAt - event.eventDeliveredAt,
      mainQueueWaitMs: action.workerSentAt - action.actionEnqueuedAt,
      workerMessageDeliveryMs: corrected.receive.correctedAt - action.workerSentAt,
      workerPreActionWaitMs: corrected.start.correctedAt - corrected.receive.correctedAt,
      workerDispatchWaitMs: corrected.start.correctedAt - action.workerSentAt,
      preServiceWaitMs: corrected.start.correctedAt - action.actionEnqueuedAt,
      preServiceWaitUpperBoundMs: corrected.start.correctedAt - action.actionEnqueuedAt + corrected.start.uncertainty,
      workerProcessMs: corrected.finish.correctedAt - corrected.start.correctedAt,
      workerRoundtripMs: action.mainResponseReceivedAt - action.workerSentAt,
      mainResponseDispatchMs: action.responseMappingStartedAt - action.mainResponseReceivedAt,
      responseMappingMs: action.responseMappingFinishedAt - action.responseMappingStartedAt,
    };
    for (const [name, span] of Object.entries(action.workerSpans ?? {})) {
      if (span) values[`worker${name[0].toUpperCase()}${name.slice(1)}Ms`] = span.end - span.start;
    }
    if (action.stateUpdateScheduledAt !== undefined) {
      values.stateScheduleMs = action.stateUpdateScheduledAt - action.responseMappingFinishedAt;
      values.reactCommitMs = action.stateCommittedAt - action.stateUpdateScheduledAt;
    }
    if (finite(action.paintObservedAt)) {
      values.paintProxyMs = action.paintObservedAt - action.stateCommittedAt;
      values.eventToCurrentPaintMs = action.paintObservedAt - event.normalizedEventAt;
      values.handlerToCurrentPaintMs = action.paintObservedAt - event.eventDeliveredAt;
      const sum = values.eventDeliveryMs + values.eventHandlerEnqueueMs + values.mainQueueWaitMs
        + values.workerRoundtripMs + values.mainResponseDispatchMs + values.responseMappingMs
        + values.stateScheduleMs + values.reactCommitMs + values.paintProxyMs;
      values.timelineResidualMs = values.eventToCurrentPaintMs - sum;
      if (action.outcome === "painted") {
        if (!finite(values.timelineResidualMs)) {
          behaviorErrors.push(`TIMELINE_RESIDUAL_NONFINITE:${action.sequenceId}`);
        } else if (Math.abs(values.timelineResidualMs) > WEB06_THRESHOLDS.metric.timelineResidualAbsoluteMaxMs) {
          behaviorErrors.push(`TIMELINE_RESIDUAL:${action.sequenceId}:${values.timelineResidualMs}`);
        }
      }
    }
    if (finite(action.terminalObservedAt)) {
      values.eventToTerminalObservationMs = action.terminalObservedAt - event.normalizedEventAt;
      if (corrected.driver) {
        values.driverDispatchToTerminalUpperBoundMs = action.terminalObservedAt - corrected.driver.correctedAt + corrected.driver.uncertainty;
      }
    }
    derivedActions.push({ ...action, frozen, event, corrected, metrics: values });
  }

  for (const derived of derivedActions) {
    if (derived.outcome === "superseded") {
      const target = resolveSupersession(derivedActions, derived, behaviorErrors);
      if (target) {
        derived.coveringPaintAt = target.paintObservedAt;
        derived.metrics.supersessionSequenceLag = target.sequenceId - derived.sequenceId;
        derived.metrics.supersessionTimeMs = target.paintObservedAt - derived.event.normalizedEventAt;
      }
    } else if (derived.outcome === "painted") {
      derived.coveringPaintAt = derived.paintObservedAt;
    }
    if (finite(derived.coveringPaintAt)) {
      derived.metrics.eventToCoveringPaintMs = derived.coveringPaintAt - derived.event.normalizedEventAt;
      if (derived.corrected.driver) {
        derived.metrics.driverDispatchToCoveringPaintUpperBoundMs = derived.coveringPaintAt
          - derived.corrected.driver.correctedAt + derived.corrected.driver.uncertainty;
      }
    }
  }
  return derivedActions;
}

function cadenceVerdict(receipt, row, behaviorErrors, setupErrors) {
  let tooShort = false;
  let tooLong = false;
  const expectedGaps = row.steps.filter((step) => Number.isFinite(step.nominalGapMs));
  const actualGaps = receipt.cadenceGaps ?? [];
  if (actualGaps.length !== expectedGaps.length) behaviorErrors.push(`CADENCE_GAP_COUNT:${actualGaps.length}!=${expectedGaps.length}`);
  for (let index = 0; index < actualGaps.length; index += 1) {
    const gap = actualGaps[index];
    const expected = expectedGaps[index];
    if (!expected || gap.stepId !== expected.id || gap.nominalGapMs !== expected.nominalGapMs) {
      behaviorErrors.push(`CADENCE_GAP_IDENTITY:${index + 1}`);
    }
    if (!finite(gap.actualDriverGapMs)) {
      setupErrors.push("SETUP_NONFINITE_CADENCE");
      continue;
    }
    const range = gap.nominalGapMs === 40
      ? WEB06_THRESHOLDS.cadence.burst40
      : gap.nominalGapMs === 120
        ? WEB06_THRESHOLDS.cadence.burst120
        : gap.nominalGapMs === 100
          ? WEB06_THRESHOLDS.cadence.existing100
          : WEB06_THRESHOLDS.cadence.sustained60;
    if (gap.actualDriverGapMs < range.minMs) tooShort = true;
    if (gap.actualDriverGapMs > range.maxMs) tooLong = true;
  }
  if (row.cadence === "same-task-pressure") return "NOT_APPLICABLE";
  return tooShort ? "TOO_SHORT" : tooLong ? "TOO_LONG" : "IN_RANGE";
}

function frameVerdict(receipt, row, behaviorErrors, setupErrors, driverCalibrations) {
  const idle = receipt.idleFrameIntervalsMs ?? [];
  if (idle.some((value) => !finite(value) || value <= 0)) setupErrors.push("SETUP_NONFINITE_IDLE_FRAME_INTERVAL");
  if (idle.length < WEB06_THRESHOLDS.frame.requiredIdleIntervals) setupErrors.push("SETUP_IDLE_FRAME_COUNT");
  if (idle.length && idle.every((value) => finite(value) && value > 0)) {
    const median = distributionSummary(idle).median;
    if (median < WEB06_THRESHOLDS.frame.idleMedianMs.min || median > WEB06_THRESHOLDS.frame.idleMedianMs.max) {
      setupErrors.push(`SETUP_IDLE_REFRESH_LANE:${median}`);
    }
  }
  const windows = receipt.interactionWindows ?? [];
  if (windows.length !== row.expectedInteractionWindowCount) {
    setupErrors.push(`SETUP_INTERACTION_WINDOW_INVALID:count:${windows.length}!=${row.expectedInteractionWindowCount}`);
  }
  const previousWindowEndByPage = new Map();
  const seenWindowIds = new Set();
  windows.forEach((window, index) => {
    const validWindowId = typeof window.windowId === "string" && window.windowId.length > 0;
    const instance = validWindowId ? window.windowId : `index-${index + 1}`;
    if (!finite(window.startedAt) || !finite(window.endedAt) || window.endedAt < window.startedAt
      || window.startBoundaryRafAt !== window.startedAt || window.endBoundaryRafAt !== window.endedAt) {
      setupErrors.push(`SETUP_INTERACTION_WINDOW_INVALID:${instance}:bounds`);
    }
    if (!validWindowId) {
      setupErrors.push(`SETUP_INTERACTION_WINDOW_IDENTITY_INVALID:${instance}:window-id`);
    } else if (seenWindowIds.has(window.windowId)) {
      setupErrors.push(`SETUP_INTERACTION_WINDOW_IDENTITY_INVALID:${instance}:duplicate-window-id`);
    }
    seenWindowIds.add(window.windowId);
    if (typeof window.pageInstanceId !== "string" || window.pageInstanceId.length === 0) {
      setupErrors.push(`SETUP_INTERACTION_WINDOW_IDENTITY_INVALID:${instance}:page-instance-id`);
      return;
    }
    const previousEnd = previousWindowEndByPage.get(window.pageInstanceId);
    if (previousEnd !== undefined && window.startedAt <= previousEnd) {
      setupErrors.push(`SETUP_INTERACTION_WINDOW_INVALID:${instance}:realm-order`);
    }
    previousWindowEndByPage.set(window.pageInstanceId, window.endedAt);
  });
  if (row.id === "learned-row") {
    const segments = receipt.idleFrameSegments;
    const windowPages = windows.map((window) => window.pageInstanceId);
    if (!Array.isArray(segments) || segments.length !== 2
      || new Set(segments.map((segment) => segment?.pageInstanceId)).size !== 2
      || !sameJson(segments.map((segment) => segment?.pageInstanceId), windowPages)
      || !sameJson(segments.flatMap((segment) => segment?.intervalsMs ?? []), idle)
      || segments.some((segment) => !Array.isArray(segment?.intervalsMs)
        || segment.intervalsMs.length < WEB06_THRESHOLDS.frame.requiredIdleIntervals
        || segment.intervalsMs.some((value) => !finite(value) || value <= 0))) {
      setupErrors.push("SETUP_LEARNED_IDLE_FRAME_SEGMENTS_INVALID");
    } else {
      for (const segment of segments) {
        const median = distributionSummary(segment.intervalsMs).median;
        if (median < WEB06_THRESHOLDS.frame.idleMedianMs.min
          || median > WEB06_THRESHOLDS.frame.idleMedianMs.max) {
          setupErrors.push(`SETUP_LEARNED_IDLE_REFRESH_LANE:${segment.pageInstanceId}:${median}`);
        }
      }
    }
  }
  const idleControls = receipt.idleControlWindows ?? [];
  if (idleControls.length !== windows.length || idleControls.some((control, index) => {
    const measured = windows[index];
    return !finite(control.startedAt) || !finite(control.endedAt) || control.endedAt <= control.startedAt
      || typeof control.controlId !== "string" || !control.controlId
      || typeof control.pageInstanceId !== "string" || !control.pageInstanceId
      || control.pageInstanceId !== measured?.pageInstanceId
      || Math.abs((control.endedAt - control.startedAt) - (measured.endedAt - measured.startedAt)) > 0.001
      || windows.some((window) => window.pageInstanceId === control.pageInstanceId
        && control.startedAt < window.endedAt && control.endedAt > window.startedAt);
  })) {
    setupErrors.push("SETUP_IDLE_LONG_TASK_CONTROL_INVALID");
  }
  if (row.id === "learned-row") {
    const continuity = receipt.lifecycleContinuity;
    const pre = continuity?.pre;
    const post = continuity?.post;
    if (new Set(windows.map((window) => window.pageInstanceId)).size !== 2
      || continuity?.browserLifecycleEventCount !== 1
      || typeof continuity?.measurementId !== "string" || !continuity.measurementId
      || pre?.phase !== "pre-reload" || post?.phase !== "post-reload-bound"
      || pre?.protocolVersion !== "web06-private-v1" || post?.protocolVersion !== "web06-private-v1"
      || pre?.measurementId !== continuity.measurementId || post?.measurementId !== continuity.measurementId
      || typeof pre?.continuityNonce !== "string" || !pre.continuityNonce
      || pre.continuityNonce !== post?.continuityNonce
      || pre?.pageInstanceId !== windows[0]?.pageInstanceId
      || post?.pageInstanceId !== windows[1]?.pageInstanceId
      || pre?.terminal?.persistenceCompleted !== true || pre?.queueIdle !== true
      || pre?.allActionsCompleted !== true
      || !sameJson(pre?.storagePayloadKeys, ["measurementId", "continuityNonce"])
      || post?.storageRemoved !== true || post?.oneShot !== true
      || post?.requiresFreshDriverPageCalibration !== true
      || post?.requiresFreshWorkerCalibration !== true) {
      behaviorErrors.push("LEARNED_REAL_RELOAD_CONTINUITY_INVALID");
    }
  }
  const observer = receipt.longTaskObserver;
  const firstWindowAt = windows[0]?.startedAt;
  const observerSegments = Array.isArray(observer?.segments) ? observer.segments : [observer];
  if (!finite(firstWindowAt) || windows.some((window) => !observerSegments.some((segment) =>
    segment?.supported === true && finite(segment.installedAt)
      && (segment.pageInstanceId === undefined || segment.pageInstanceId === window.pageInstanceId)
      && segment.installedAt <= window.startedAt))) {
    setupErrors.push("SETUP_LONG_TASK_OBSERVER_UNAVAILABLE");
  }
  const focusSamples = receipt.focusVisibilitySamples ?? [];
  if (!Array.isArray(focusSamples) || !focusSamples.length || focusSamples.some((sample) =>
    !finite(sample.recordedAt) || sample.focused !== true || sample.visibilityState !== "visible"
    || typeof sample.pageInstanceId !== "string" || !sample.pageInstanceId)) {
    setupErrors.push("SETUP_PAGE_NOT_FOREGROUND");
  } else if (windows.length) {
    for (const window of windows) {
      const pre = focusSamples.filter((sample) => sample.role === "pre-boundary" && sample.windowId === window.windowId);
      const post = focusSamples.filter((sample) => sample.role === "post-boundary" && sample.windowId === window.windowId);
      if (typeof window.windowId !== "string" || !window.windowId
        || typeof window.pageInstanceId !== "string" || !window.pageInstanceId
        || pre.length !== 1 || post.length !== 1
        || pre[0]?.pageInstanceId !== window.pageInstanceId
        || post[0]?.pageInstanceId !== window.pageInstanceId
        || pre[0]?.recordedAt !== window.preBoundaryFocusRecordedAt
        || post[0]?.recordedAt !== window.postBoundaryFocusRecordedAt
        || pre[0]?.recordedAt > window.startedAt || post[0]?.recordedAt < window.endedAt) {
        setupErrors.push("SETUP_FOREGROUND_BOUNDARY_IDENTITY_INVALID");
      }
    }
  }
  const frames = receipt.interactionFrameIntervalsMs ?? [];
  if (!frames.length) setupErrors.push("SETUP_INTERACTION_FRAMES_MISSING");
  if (frames.some((value) => !finite(value) || value <= 0)) setupErrors.push("SETUP_NONFINITE_INTERACTION_FRAME_INTERVAL");
  const frameTimestamps = receipt.interactionFrameTimestamps ?? [];
  const frameWindows = receipt.interactionFrameWindows ?? [];
  if (frameWindows.length !== windows.length) {
    setupErrors.push("SETUP_INTERACTION_FRAME_BOUNDARIES_MISSING");
  } else {
    const flattenedTimestamps = [];
    const flattenedIntervals = [];
    for (let index = 0; index < frameWindows.length; index += 1) {
      const frameWindow = frameWindows[index];
      const window = windows[index];
      const timestamps = frameWindow?.timestamps;
      const intervals = frameWindow?.intervalsMs;
      if (frameWindow?.windowId !== window?.windowId || frameWindow?.pageInstanceId !== window?.pageInstanceId
        || !Array.isArray(timestamps) || timestamps.length < 2 || timestamps.some((value) => !finite(value))
        || !Array.isArray(intervals) || intervals.length !== timestamps.length - 1
        || timestamps[0] !== window.startedAt || timestamps.at(-1) !== window.endedAt) {
        setupErrors.push(`SETUP_INTERACTION_FRAME_BOUNDARY_MISMATCH:${index + 1}`);
        continue;
      }
      const recomputed = timestamps.slice(1).map((value, position) => value - timestamps[position]);
      if (!sameJson(recomputed, intervals)) setupErrors.push(`SETUP_INTERACTION_FRAME_INTERVAL_MISMATCH:${index + 1}`);
      flattenedTimestamps.push(...timestamps);
      flattenedIntervals.push(...intervals);
    }
    if (!sameJson(flattenedTimestamps, frameTimestamps) || !sameJson(flattenedIntervals, frames)) {
      setupErrors.push("SETUP_INTERACTION_FRAME_FLATTENING_MISMATCH");
    }
    const eventsWithDispatch = (receipt.events ?? []).filter((event) =>
      finite(event.actualDriverDispatchAt) && finite(event.normalizedEventAt));
    if (driverCalibrations && eventsWithDispatch.length) {
      try {
        const corrected = eventsWithDispatch.map((event) => ({
          pageInstanceId: event.pageInstanceId,
          point: correctDriverTimestamp(event.actualDriverDispatchAt,
            calibrationForPage(driverCalibrations, event.pageInstanceId).driver ?? driverCalibrations,
            event.normalizedEventAt),
        }));
        if (corrected.some(({ point, pageInstanceId }) => !windows.some((window) =>
          (pageInstanceId === undefined || window.pageInstanceId === pageInstanceId)
          && point.correctedAt - point.uncertainty >= window.startedAt
            && point.correctedAt + point.uncertainty <= window.endedAt))) {
          setupErrors.push("SETUP_DISPATCH_OUTSIDE_INTERACTION_WINDOWS");
        }
      } catch (error) {
        setupErrors.push(error instanceof Error ? error.message : "SETUP_INVALID_CLOCK_CALIBRATION");
      }
    }
    const observations = [
      ...(receipt.commonSamples ?? []).map((sample) => ({ at: sample.observedAt, pageInstanceId: sample.pageInstanceId })),
      ...(receipt.actions ?? []).flatMap((action) => [
        { at: action.paintObservedAt, pageInstanceId: action.pageInstanceId },
        { at: action.terminalObservedAt, pageInstanceId: action.pageInstanceId },
      ]),
    ].filter((item) => finite(item.at));
    if (observations.some(({ at, pageInstanceId }) => !windows.some((window) =>
      (pageInstanceId === undefined || window.pageInstanceId === pageInstanceId)
        && at >= window.startedAt && at <= window.endedAt))) {
      setupErrors.push("SETUP_OBSERVATION_OUTSIDE_INTERACTION_WINDOWS");
    }
  }
  const longTasks = receipt.longTasks ?? [];
  if (longTasks.some((task) => !finite(task.startTime) || task.startTime < 0
    || !finite(task.durationMs) || task.durationMs < 0)) {
    setupErrors.push("SETUP_NONFINITE_LONG_TASK");
  }
  for (const [index, task] of longTasks.entries()) {
    if (!finite(task.startTime) || task.startTime < 0
      || !finite(task.durationMs) || task.durationMs < 0) continue;
    const recomputedOverlap = windows.some((window) => window.pageInstanceId === task.pageInstanceId
      &&
      task.startTime < window.endedAt && task.startTime + task.durationMs > window.startedAt);
    const recomputedIdleOverlap = idleControls.some((control) => control.pageInstanceId === task.pageInstanceId
      &&
      task.startTime < control.endedAt && task.startTime + task.durationMs > control.startedAt);
    if (task.overlapsInteractionWindow !== recomputedOverlap) {
      behaviorErrors.push(`LONG_TASK_OVERLAP_MISMATCH:${index + 1}`);
    }
    if (task.overlapsIdleControl !== recomputedIdleOverlap) {
      behaviorErrors.push(`LONG_TASK_IDLE_OVERLAP_MISMATCH:${index + 1}`);
    }
    if (!recomputedOverlap && !recomputedIdleOverlap) {
      behaviorErrors.push(`LONG_TASK_OUTSIDE_DECLARED_CONTROLS:${index + 1}`);
    }
  }
  const binding = row.binding === true || row.binding === "branch-b-only";
  const finiteFrames = frames.filter((value) => finite(value) && value > 0);
  const frameRed = binding && (finiteFrames.some((value) => value >= WEB06_THRESHOLDS.frame.rejectIntervalAtOrAboveMs)
    || (finiteFrames.length && distributionSummary(finiteFrames).p99 > WEB06_THRESHOLDS.frame.p99Ms.max));
  const longTaskRed = binding && longTasks.some((task) => {
    const overlaps = windows.some((window) => window.pageInstanceId === task.pageInstanceId
      &&
      task.startTime < window.endedAt && task.startTime + task.durationMs > window.startedAt);
    return overlaps && task.durationMs >= WEB06_THRESHOLDS.frame.rejectLongTaskAtOrAboveMs;
  });
  if ((receipt.assetsRequestedDuringWindow ?? []).length) behaviorErrors.push("ASSET_REQUEST_DURING_WINDOW");
  return { frameRed: Boolean(frameRed), longTaskRed };
}

function thresholdVerdict(receipt, row, derivedActions, behaviorErrors) {
  const covering = [];
  const terminal = [];
  const preService = [];
  const frozenActions = expandScenarioExpectedTimeline(row.id).actions;
  for (const step of row.steps) {
    const stepActions = derivedActions.filter((action) => action.stepId === step.id);
    if (step.sample === "covering") {
      const item = stepActions[0];
      if (!item || !finite(item.metrics.eventToCoveringPaintMs) || !finite(item.metrics.driverDispatchToCoveringPaintUpperBoundMs)) {
        behaviorErrors.push(`COVERING_SAMPLE_MISSING:${step.id}`);
      } else {
        covering.push(item.metrics);
        preService.push(item.metrics.preServiceWaitUpperBoundMs);
      }
    }
    if (step.sample === "terminal") {
      const owner = terminalOwnerForStep(step, frozenActions);
      const primary = stepActions.find((action) => action.sequenceId === owner?.sequenceId);
      if (!primary || !finite(primary.metrics.eventToTerminalObservationMs) || !finite(primary.metrics.driverDispatchToTerminalUpperBoundMs)) {
        behaviorErrors.push(`TERMINAL_SAMPLE_MISSING:${step.id}`);
      } else terminal.push({ ...primary.metrics, stressDeadline: primary.frozen.stressDeadline === true });
    }
  }
  if (covering.length !== row.expectedCoveringSamples) behaviorErrors.push(`COVERING_COUNT:${covering.length}`);
  if (terminal.length !== row.expectedTerminalSamples) behaviorErrors.push(`TERMINAL_COUNT:${terminal.length}`);
  const violations = [];
  if (row.binding === true || row.binding === "branch-b-only") {
    for (const sample of covering) {
      if (sample.eventToCoveringPaintMs > WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs.max) violations.push("covering-max");
      if (sample.driverDispatchToCoveringPaintUpperBoundMs > WEB06_THRESHOLDS.sustained.driverDispatchToCoveringPaintUpperBoundMs.max) violations.push("driver-covering-max");
    }
    if (preService.length) {
      violations.push(...evaluateThresholdDistribution(preService, WEB06_THRESHOLDS.sustained.preServiceWaitUpperBoundMs, "pre-service").violations);
    }
    if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)
      && covering.length) {
      violations.push(...evaluateThresholdDistribution(
        covering.map((sample) => sample.eventToCoveringPaintMs),
        WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs,
        "covering",
      ).violations);
    }
    for (const sample of terminal) {
      const ceiling = sample.stressDeadline
        ? WEB06_THRESHOLDS.terminal.persistenceStressCompletionMs.max
        : WEB06_THRESHOLDS.terminal.eventToTerminalObservationMs.max;
      if (sample.eventToTerminalObservationMs > ceiling) violations.push("terminal-max");
      if (!sample.stressDeadline && sample.driverDispatchToTerminalUpperBoundMs > WEB06_THRESHOLDS.terminal.driverDispatchToTerminalUpperBoundMs.max) {
        violations.push("driver-terminal-max");
      }
    }
  }
  if (row.overlapRequired) {
    const expectedPairs = WEB06_PRESSURE_PAIR_REGISTRY[row.id] ?? [];
    const proofs = receipt.pressureProofs ?? [];
    if (proofs.length !== expectedPairs.length) behaviorErrors.push("FIFO_PRESSURE_PROOF_COUNT");
    for (let index = 0; index < expectedPairs.length; index += 1) {
      const expectedPair = expectedPairs[index];
      const proof = proofs[index];
      const earlier = derivedActions.find((action) => action.stepId === expectedPair.earlierStepId
        && action.originKind !== "background");
      const later = derivedActions.find((action) => action.stepId === expectedPair.laterStepId
        && action.originKind !== "background");
      const earlierObservedAt = earlier?.outcome === "painted" ? earlier.paintObservedAt
        : ["committed", "barrier-completed", "failure"].includes(earlier?.outcome) ? earlier?.terminalObservedAt : undefined;
      const laterAppliedAt = later?.terminalKind === "presentation" ? later?.stateCommittedAt : later?.terminalObservedAt;
      const laterWorkerStartedAtMainClock = later?.corrected?.start?.correctedAt - later?.corrected?.start?.uncertainty;
      if (!proof || proof.subcase !== expectedPair.subcase
        || proof.earlierStepId !== expectedPair.earlierStepId || proof.laterStepId !== expectedPair.laterStepId
        || proof.earlierSequenceId !== earlier?.sequenceId || proof.laterSequenceId !== later?.sequenceId
        || proof.dispatchContract !== "single-page-task-no-await"
        || !finite(earlier?.event?.actualDriverDispatchAt)
        || earlier.event.actualDriverDispatchAt !== later?.event?.actualDriverDispatchAt
        || !finite(earlier?.workerSentAt) || !finite(later?.actionEnqueuedAt) || !finite(earlier?.mainResponseReceivedAt)
        || !(earlier.workerSentAt <= later.actionEnqueuedAt && later.actionEnqueuedAt < earlier.mainResponseReceivedAt)
        || !Number.isSafeInteger(later?.mainQueueDepth) || later.mainQueueDepth < 1
        || !finite(laterWorkerStartedAtMainClock)
        || earlier.mainResponseReceivedAt > laterWorkerStartedAtMainClock
        || !finite(earlierObservedAt) || !finite(laterAppliedAt) || earlierObservedAt > laterAppliedAt) {
        behaviorErrors.push(`FIFO_PRESSURE_NOT_PROVED:${expectedPair.subcase}`);
      }
    }
    const errorTarget = derivedActions.find((action) => action.stepId === "extended-error-target");
    if (row.id === "extended-scheduler-barriers") {
      const recovery = derivedActions.find((action) => action.stepId === "extended-error-reset");
      if (errorTarget?.outcome !== "failure" || !recovery || recovery.outcome === "failure"
        || recovery.sequenceId <= errorTarget.sequenceId || recovery.actionEnqueuedAt < errorTarget.terminalObservedAt) {
        behaviorErrors.push("EXPECTED_ERROR_BOUNDARY_RECOVERY_NOT_PROVED");
      }
    }
  }
  return { covering, terminal, violations, latencyRed: violations.length > 0 };
}

function burstRecoveryVerdict(receipt, row) {
  const behaviorErrors = [];
  const violations = [];
  const expectedRecoveries = row.steps.filter((step) => step.declaredBurstPauseAfter === true);
  const recoveries = receipt.burstRecoveries ?? [];
  if (recoveries.length !== expectedRecoveries.length) {
    behaviorErrors.push(`BURST_RECOVERY_COUNT:${recoveries.length}!=${expectedRecoveries.length}`);
  }
  for (let index = 0; index < Math.min(recoveries.length, expectedRecoveries.length); index += 1) {
    const recovery = recoveries[index];
    const expectedStep = expectedRecoveries[index];
    const event = (receipt.events ?? []).find((candidate) =>
      candidate.stepId === expectedStep.id && candidate.type === "keydown");
    if (!event || recovery.afterStepId !== expectedStep.id || !finite(recovery.latestPaintAt)
      || recovery.latestPaintAt < event.normalizedEventAt) {
      behaviorErrors.push(`BURST_RECOVERY_INVALID:${expectedStep.id}`);
      continue;
    }
    if (recovery.latestPaintAt - event.normalizedEventAt > 67) violations.push(`burst-recovery:${expectedStep.id}`);
    const idle = recovery.idleSnapshot;
    if (!idle || idle.queueDepth !== 0 || idle.runningActionId !== null
      || idle.pendingFanoutActions !== 0 || idle.pendingTerminalActions !== 0
      || idle.pendingSentinelCaptures !== 0 || idle.completedActionCount !== recovery.expectedCompletedActionCount
      || !Number.isSafeInteger(idle.completedActionCount) || idle.completedActionCount < 0) {
      behaviorErrors.push(`BURST_IDLE_SNAPSHOT_INVALID:${expectedStep.id}`);
    }
  }
  return { behaviorErrors, violations };
}

export function validateAndRecomputeReceipt(receipt) {
  const setupErrors = [];
  const behaviorErrors = [];
  validateIdentity(receipt, setupErrors);
  validateInternalMode(receipt, setupErrors);
  const row = hasOwn(SCENARIO_REGISTRY, receipt.scenarioId)
    ? SCENARIO_REGISTRY[receipt.scenarioId]
    : undefined;
  if (!row) setupErrors.push("SETUP_UNKNOWN_SCENARIO");
  const privacy = validatePointerFreePrivacy(receipt);
  behaviorErrors.push(...privacy.errors);
  if (!row) {
    return {
      status: "SETUP_INVALID",
      setupErrors,
      behaviorErrors,
      cadence: "NOT_APPLICABLE",
      frameRed: false,
      longTaskRed: false,
      thresholdViolations: [],
      metrics: { actions: [], covering: [], terminal: [] },
    };
  }
  requireArrayFields(receipt, [
    "events",
    "auxiliaryEvents",
    "actions",
    "cadenceGaps",
    "idleFrameIntervalsMs",
    "interactionFrameIntervalsMs",
    "interactionFrameTimestamps",
    "interactionFrameWindows",
    "interactionWindows",
    "idleControlWindows",
    "longTasks",
    "focusVisibilitySamples",
    "assetsRequestedDuringWindow",
    "measurementProtocolBlockers",
  ], setupErrors, "SETUP_INTERNAL");
  if (setupErrors.some((error) => error.startsWith("SETUP_INTERNAL_"))) {
    return {
      status: "SETUP_INVALID",
      setupErrors,
      behaviorErrors,
      thresholdViolations: [],
      metrics: { actions: [], covering: [], terminal: [] },
    };
  }
  validateSentinelOverflows(receipt, setupErrors);
  const protocolDisposition = classifyMeasurementProtocolBlockers(receipt.measurementProtocolBlockers);
  setupErrors.push(...protocolDisposition.setupErrors);
  validateAuxiliaryDomEvents(receipt, behaviorErrors);
  const expected = validateTimelineShape(receipt, row, behaviorErrors);
  validateEventClock(receipt, setupErrors, behaviorErrors);
  const eventById = new Map(receipt.events.map((event) => [event.eventSequenceId, event]));
  for (const action of receipt.actions) {
    const event = eventById.get(action.eventSequenceId ?? action.causedByEventSequenceId);
    if (event) validateSameRealmAction(action, event, behaviorErrors);
    validateWorkerSpans(action, receipt.mode, behaviorErrors);
  }
  const calibrations = calibrationFor(receipt, setupErrors);
  validateOutcomes(receipt.actions ?? [], expected, row, receipt.mode, behaviorErrors);
  let derivedActions = [];
  let timingComplete = false;
  if (calibrations) {
    derivedActions = recomputeActions(receipt, row, expected, calibrations, setupErrors, behaviorErrors);
    timingComplete = derivedActions.length === receipt.actions.length
      && !setupErrors.some(isClockCalibrationSetupError);
  }
  const cadence = cadenceVerdict(receipt, row, behaviorErrors, setupErrors);
  const frames = frameVerdict(receipt, row, behaviorErrors, setupErrors, calibrations);
  const burst = burstRecoveryVerdict(receipt, row);
  behaviorErrors.push(...burst.behaviorErrors);
  const timingThresholds = timingComplete
    ? thresholdVerdict(receipt, row, derivedActions, behaviorErrors)
    : { covering: [], terminal: [], violations: [], latencyRed: false };
  const thresholds = {
    ...timingThresholds,
    violations: [
      ...protocolDisposition.thresholdViolations,
      ...timingThresholds.violations,
      ...burst.violations,
    ],
  };
  const disposition = finalAttemptStatus({
    setupErrors,
    behaviorErrors,
    cadence,
    thresholdViolations: thresholds.violations,
    frameRed: frames.frameRed,
    longTaskRed: frames.longTaskRed,
  });
  return {
    ...disposition,
    setupErrors,
    behaviorErrors,
    cadence,
    frameRed: frames.frameRed,
    longTaskRed: frames.longTaskRed,
    thresholdViolations: thresholds.violations,
    metrics: { actions: derivedActions, covering: thresholds.covering, terminal: thresholds.terminal },
    calibration: calibrations,
  };
}

function validateFiveRoundPoolIdentity(receipts) {
  if (!Array.isArray(receipts) || receipts.length !== WEB06_THRESHOLDS.attempts.requiredValid) {
    throw new Error("WEB06_POOL_REQUIRES_EXACTLY_FIVE_ROUNDS");
  }
  const first = receipts[0];
  const identity = (receipt) => JSON.stringify({
    metricContractVersion: receipt.metricContractVersion,
    scenarioRegistryVersion: receipt.scenarioRegistryVersion,
    behaviorPredicateVersion: receipt.behaviorPredicateVersion,
    scenarioRunId: receipt.scenarioRunId,
    scenarioId: receipt.scenarioId,
    schemaId: receipt.schemaId,
    mode: receipt.mode,
    source: receipt.source,
  });
  if (receipts.some((receipt) => identity(receipt) !== identity(first))) {
    throw new Error("WEB06_POOL_SOURCE_OR_MODE_IDENTITY_MISMATCH");
  }
  const run = hasOwn(SCENARIO_RUN_REGISTRY, first.scenarioRunId)
    ? SCENARIO_RUN_REGISTRY[first.scenarioRunId]
    : undefined;
  if (run === undefined || run.scenarioId !== first.scenarioId || run.schema !== first.schemaId) {
    throw new Error("WEB06_POOL_SCENARIO_RUN_IDENTITY_MISMATCH");
  }
  for (const field of ["roundId", "attemptId"]) {
    const values = receipts.map((receipt) => receipt[field]);
    if (values.some((value) => typeof value !== "string" || !value) || new Set(values).size !== receipts.length) {
      throw new Error(`WEB06_POOL_${field.toUpperCase()}_IDENTITY_INVALID`);
    }
  }
}

export function evaluateFiveRoundPool(receipts) {
  validateFiveRoundPoolIdentity(receipts);
  const parsed = receipts.map(validateAndRecomputeReceipt);
  if (parsed.some((result) => result.status === "SETUP_INVALID" || result.status === "NO_VERDICT_INVALID_CADENCE")) {
    throw new Error("WEB06_POOL_CONTAINS_INVALID_ROUND");
  }
  const scenarioId = receipts[0].scenarioId;
  const row = hasOwn(SCENARIO_REGISTRY, scenarioId) ? SCENARIO_REGISTRY[scenarioId] : undefined;
  if (!row) throw new Error("WEB06_POOL_UNKNOWN_SCENARIO");
  const pooledCovering = parsed.flatMap((result) => result.metrics.covering.map((sample) => sample.eventToCoveringPaintMs));
  const pooledPreService = parsed.flatMap((result) => result.metrics.covering.map((sample) => sample.preServiceWaitUpperBoundMs));
  const pooledDriverCovering = parsed.flatMap((result) =>
    result.metrics.covering.map((sample) => sample.driverDispatchToCoveringPaintUpperBoundMs));
  const pooledTerminalSamples = parsed.flatMap((result) => result.metrics.terminal);
  const pooledTerminal = pooledTerminalSamples.map((sample) => sample.eventToTerminalObservationMs);
  const pooledDriverTerminal = parsed.flatMap((result) =>
    result.metrics.terminal.filter((sample) => !sample.stressDeadline)
      .map((sample) => sample.driverDispatchToTerminalUpperBoundMs));
  const pooledFrames = receipts.flatMap((receipt) => receipt.interactionFrameIntervalsMs ?? []);
  const violations = [];
  const expectedCoveringCount = row.expectedCoveringSamples * WEB06_THRESHOLDS.attempts.requiredValid;
  const expectedTerminalCount = row.expectedTerminalSamples * WEB06_THRESHOLDS.attempts.requiredValid;
  if (pooledCovering.length !== expectedCoveringCount) {
    violations.push(`pooled-covering-count:${pooledCovering.length}!=${expectedCoveringCount}`);
  }
  if (pooledTerminal.length !== expectedTerminalCount) {
    violations.push(`pooled-terminal-count:${pooledTerminal.length}!=${expectedTerminalCount}`);
  }
  if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)) {
    if (pooledCovering.length === 0) violations.push("pooled-covering:missing");
    else violations.push(...evaluateThresholdDistribution(pooledCovering, WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs, "pooled-covering").violations);
  }
  if (row.binding === true || row.binding === "branch-b-only") {
    if (pooledPreService.length !== expectedCoveringCount) {
      violations.push(`pooled-pre-service-count:${pooledPreService.length}!=${expectedCoveringCount}`);
    } else if (pooledPreService.length) {
      violations.push(...evaluateThresholdDistribution(pooledPreService, WEB06_THRESHOLDS.sustained.preServiceWaitUpperBoundMs, "pooled-pre-service").violations);
    }
  }
  if (row.binding === true || row.binding === "branch-b-only") {
    if (pooledDriverCovering.some((value) => value > WEB06_THRESHOLDS.sustained.driverDispatchToCoveringPaintUpperBoundMs.max)) {
      violations.push("pooled-driver-covering:max");
    }
    if (pooledTerminalSamples.some((sample) => sample.eventToTerminalObservationMs
      > (sample.stressDeadline
        ? WEB06_THRESHOLDS.terminal.persistenceStressCompletionMs.max
        : WEB06_THRESHOLDS.terminal.eventToTerminalObservationMs.max))) {
      violations.push("pooled-terminal:max");
    }
    if (pooledDriverTerminal.some((value) => value > WEB06_THRESHOLDS.terminal.driverDispatchToTerminalUpperBoundMs.max)) {
      violations.push("pooled-driver-terminal:max");
    }
    if (pooledFrames.some((value) => value >= WEB06_THRESHOLDS.frame.rejectIntervalAtOrAboveMs)) {
      violations.push("pooled-frame:max");
    }
    if (pooledFrames.length && distributionSummary(pooledFrames).p99 > WEB06_THRESHOLDS.frame.p99Ms.max) {
      violations.push("pooled-frame:p99");
    }
  }
  return {
    pass: parsed.every((result) => result.status === "PASS") && violations.length === 0,
    parsed,
    pooledCovering: pooledCovering.length ? distributionSummary(pooledCovering) : null,
    pooledPreService: pooledPreService.length ? distributionSummary(pooledPreService) : null,
    pooledDriverCovering: pooledDriverCovering.length ? distributionSummary(pooledDriverCovering) : null,
    pooledTerminal: pooledTerminal.length ? distributionSummary(pooledTerminal) : null,
    pooledDriverTerminal: pooledDriverTerminal.length ? distributionSummary(pooledDriverTerminal) : null,
    pooledFrames: pooledFrames.length ? distributionSummary(pooledFrames) : null,
    pooledLongTaskCount: receipts.reduce((sum, receipt) => sum
      + (receipt.longTasks ?? []).filter((task) => task.overlapsInteractionWindow === true).length, 0),
    violations,
  };
}

/** Pool the identical external-sentinel surface used by PRODUCT/minimal/full. */
export function evaluateFiveRoundCommonPool(receipts) {
  validateFiveRoundPoolIdentity(receipts);
  const parsed = receipts.map(validateCommonSurfaceReceipt);
  if (parsed.some((result) => result.status === "SETUP_INVALID" || result.status === "NO_VERDICT_INVALID_CADENCE")) {
    throw new Error("WEB06_COMMON_POOL_CONTAINS_INVALID_ROUND");
  }
  const scenarioId = receipts[0].scenarioId;
  const row = hasOwn(SCENARIO_REGISTRY, scenarioId) ? SCENARIO_REGISTRY[scenarioId] : undefined;
  if (!row) throw new Error("WEB06_COMMON_POOL_UNKNOWN_SCENARIO");
  const samples = parsed.flatMap((result) => result.metrics.samples);
  const coveringSamples = samples.filter((sample) => sample.sampleKind === "covering");
  const terminalSamples = samples.filter((sample) => sample.sampleKind === "terminal");
  const pooledCovering = coveringSamples.map((sample) => sample.eventToObservationMs);
  const pooledDriverCovering = coveringSamples.map((sample) => sample.driverDispatchToObservationUpperBoundMs);
  const pooledTerminal = terminalSamples.map((sample) => sample.eventToObservationMs);
  const pooledDriverTerminal = terminalSamples.filter((sample) => !sample.stressDeadline)
    .map((sample) => sample.driverDispatchToObservationUpperBoundMs);
  const pooledFrames = receipts.flatMap((receipt) => receipt.interactionFrameIntervalsMs ?? []);
  const violations = [];
  const binding = row.binding === true || row.binding === "branch-b-only";
  const expectedCoveringCount = row.expectedCoveringSamples * WEB06_THRESHOLDS.attempts.requiredValid;
  const expectedTerminalCount = row.expectedTerminalSamples * WEB06_THRESHOLDS.attempts.requiredValid;
  if (pooledCovering.length !== expectedCoveringCount) {
    violations.push(`pooled-common-covering-count:${pooledCovering.length}!=${expectedCoveringCount}`);
  }
  if (pooledTerminal.length !== expectedTerminalCount) {
    violations.push(`pooled-common-terminal-count:${pooledTerminal.length}!=${expectedTerminalCount}`);
  }
  if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)) {
    if (pooledCovering.length === 0) violations.push("pooled-common-covering:missing");
    else violations.push(...evaluateThresholdDistribution(
      pooledCovering,
      WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs,
      "pooled-common-covering",
    ).violations);
  }
  if (binding) {
    if (pooledDriverCovering.some((value) => value > WEB06_THRESHOLDS.sustained.driverDispatchToCoveringPaintUpperBoundMs.max)) {
      violations.push("pooled-common-driver-covering:max");
    }
    if (terminalSamples.some((sample) => sample.eventToObservationMs
      > (sample.stressDeadline
        ? WEB06_THRESHOLDS.terminal.persistenceStressCompletionMs.max
        : WEB06_THRESHOLDS.terminal.eventToTerminalObservationMs.max))) {
      violations.push("pooled-common-terminal:max");
    }
    if (pooledDriverTerminal.some((value) => value > WEB06_THRESHOLDS.terminal.driverDispatchToTerminalUpperBoundMs.max)) {
      violations.push("pooled-common-driver-terminal:max");
    }
    if (pooledFrames.some((value) => value >= WEB06_THRESHOLDS.frame.rejectIntervalAtOrAboveMs)) {
      violations.push("pooled-common-frame:max");
    }
    if (pooledFrames.length && distributionSummary(pooledFrames).p99 > WEB06_THRESHOLDS.frame.p99Ms.max) {
      violations.push("pooled-common-frame:p99");
    }
  }
  return {
    pass: parsed.every((result) => result.status === "PASS") && violations.length === 0,
    parsed,
    pooledCovering: pooledCovering.length ? distributionSummary(pooledCovering) : null,
    pooledDriverCovering: pooledDriverCovering.length ? distributionSummary(pooledDriverCovering) : null,
    pooledTerminal: pooledTerminal.length ? distributionSummary(pooledTerminal) : null,
    pooledDriverTerminal: pooledDriverTerminal.length ? distributionSummary(pooledDriverTerminal) : null,
    pooledFrames: pooledFrames.length ? distributionSummary(pooledFrames) : null,
    pooledLongTaskCount: receipts.reduce((sum, receipt) => sum
      + (receipt.longTasks ?? []).filter((task) => task.overlapsInteractionWindow === true).length, 0),
    violations,
  };
}

function distributionOrNull(values) {
  return values.length ? distributionSummary(values) : null;
}

const ROUND_DIAGNOSTIC_FIELDS = Object.freeze([
  "setupErrorCodes",
  "behaviorErrorCodes",
  "thresholdViolations",
]);
const FIVE_ROUND_DIAGNOSTIC_FIELDS = Object.freeze([
  "summaryErrors",
  "poolViolations",
]);
const TRUE_DIAGNOSTIC_SYNONYM_CODES = new Set([
  "BURST_RECOVERY_INVALID", "BURST_RECOVERY_IDENTITY",
  "BEHAVIOR_PAGE_VISIBLE_COUNT", "BEHAVIOR_VISIBLECOUNT",
  "BEHAVIOR_PAGE_PREVIOUS", "BEHAVIOR_PREVIOUSDISABLED",
  "BEHAVIOR_PAGE_NEXT", "BEHAVIOR_NEXTDISABLED",
  "BEHAVIOR_TEXTAREA_VALUE", "BEHAVIOR_TEXTAREAVALUE",
  "BEHAVIOR_SELECTION_START", "BEHAVIOR_SELECTIONSTART",
  "BEHAVIOR_SELECTION_END", "BEHAVIOR_SELECTIONEND",
]);

function stableSemanticValue(value) {
  if (Array.isArray(value)) return value.map(stableSemanticValue);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.keys(value).filter((key) => value[key] !== undefined).sort()
    .map((key) => [key, stableSemanticValue(value[key])]));
}

function canonicalDiagnosticCode(value) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error("WEB06_SUMMARY_DIAGNOSTIC_CODE_INVALID");
  }
  const [code, ...detail] = value.split(":");
  const suffix = detail.join(":");
  const withDetail = (family) => suffix.length === 0 ? family : `${family}:${suffix}`;

  if (["BURST_RECOVERY_INVALID", "BURST_RECOVERY_IDENTITY"].includes(code)) {
    return withDetail("BURST_RECOVERY_IDENTITY");
  }
  const behaviorAliases = {
    BEHAVIOR_PAGE_VISIBLE_COUNT: "BEHAVIOR_VISIBLE_COUNT",
    BEHAVIOR_VISIBLECOUNT: "BEHAVIOR_VISIBLE_COUNT",
    BEHAVIOR_PAGE_PREVIOUS: "BEHAVIOR_PREVIOUS_DISABLED",
    BEHAVIOR_PREVIOUSDISABLED: "BEHAVIOR_PREVIOUS_DISABLED",
    BEHAVIOR_PAGE_NEXT: "BEHAVIOR_NEXT_DISABLED",
    BEHAVIOR_NEXTDISABLED: "BEHAVIOR_NEXT_DISABLED",
    BEHAVIOR_TEXTAREA_VALUE: "BEHAVIOR_TEXTAREA_VALUE",
    BEHAVIOR_TEXTAREAVALUE: "BEHAVIOR_TEXTAREA_VALUE",
    BEHAVIOR_SELECTION_START: "BEHAVIOR_SELECTION_START",
    BEHAVIOR_SELECTIONSTART: "BEHAVIOR_SELECTION_START",
    BEHAVIOR_SELECTION_END: "BEHAVIOR_SELECTION_END",
    BEHAVIOR_SELECTIONEND: "BEHAVIOR_SELECTION_END",
  };
  if (hasOwn(behaviorAliases, code)) return withDetail(behaviorAliases[code]);
  return value;
}

export function canonicalizeSummaryDiagnosticCodes(values) {
  if (!Array.isArray(values)) throw new Error("WEB06_SUMMARY_DIAGNOSTIC_ARRAY_INVALID");
  const grouped = new Map();
  for (const raw of values) {
    const canonical = canonicalDiagnosticCode(raw);
    const [code] = raw.split(":");
    const records = grouped.get(canonical) ?? [];
    records.push({ raw, code });
    grouped.set(canonical, records);
  }
  return [...grouped.entries()].sort(([left], [right]) => left.localeCompare(right))
    .flatMap(([canonical, records]) => {
      const codes = new Set(records.map((record) => record.code));
      const trueSynonymFamily = codes.size > 1
        && [...codes].every((code) => TRUE_DIAGNOSTIC_SYNONYM_CODES.has(code));
      if (!trueSynonymFamily) return Array.from({ length: records.length }, () => canonical);
      const rawCounts = new Map();
      for (const record of records) rawCounts.set(record.raw, (rawCounts.get(record.raw) ?? 0) + 1);
      return Array.from({ length: Math.max(...rawCounts.values()) }, () => canonical);
    });
}

function diagnosticFieldsForSummary(summary) {
  if (summary?.version === "web06-round-summary-v1") return ROUND_DIAGNOSTIC_FIELDS;
  if (summary?.version === "web06-five-round-summary-v1") return FIVE_ROUND_DIAGNOSTIC_FIELDS;
  throw new Error("WEB06_SUMMARY_VERSION_INVALID");
}

function sha256Json(value) {
  return createHash("sha256").update(JSON.stringify(value), "utf8").digest("hex");
}

function buildImplementationDiagnosticBinding(summary) {
  const dimensions = Object.fromEntries(diagnosticFieldsForSummary(summary).map((field) => {
    const raw = summary[field];
    const semantic = canonicalizeSummaryDiagnosticCodes(raw);
    return [field, {
      rawCount: raw.length,
      rawSha256: sha256Json(raw),
      semanticCount: semantic.length,
      semanticSha256: sha256Json(semantic),
    }];
  }));
  const binding = {
    version: WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION,
    dimensions,
    ...(summary.version === "web06-five-round-summary-v1" ? {
      roundBindingsSha256: sha256Json(summary.roundSummaries.map((round) =>
        round.implementationDiagnosticBinding)),
    } : {}),
  };
  return stableSemanticValue(binding);
}

function semanticRoundProjection(summary) {
  const projected = structuredClone(summary);
  delete projected.implementationDiagnosticBinding;
  delete projected.semanticProjectionSha256;
  for (const field of ROUND_DIAGNOSTIC_FIELDS) {
    projected[field] = canonicalizeSummaryDiagnosticCodes(projected[field]);
  }
  return stableSemanticValue(projected);
}

/** Frozen decision-semantic projection used for the literal Phase-1 byte comparison. */
export function buildSummarySemanticProjection(summary) {
  let projected;
  let scope;
  if (summary?.version === "web06-round-summary-v1") {
    scope = "round";
    projected = semanticRoundProjection(summary);
  } else if (summary?.version === "web06-five-round-summary-v1") {
    scope = "five-round";
    projected = structuredClone(summary);
    delete projected.implementationDiagnosticBinding;
    delete projected.semanticProjectionSha256;
    for (const field of FIVE_ROUND_DIAGNOSTIC_FIELDS) {
      projected[field] = canonicalizeSummaryDiagnosticCodes(projected[field]);
    }
    projected.roundSummaries = projected.roundSummaries.map(semanticRoundProjection);
    projected = stableSemanticValue(projected);
  } else {
    throw new Error("WEB06_SUMMARY_VERSION_INVALID");
  }
  return stableSemanticValue({
    version: WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION,
    scope,
    summary: projected,
  });
}

export function summarySemanticProjectionBytes(summary) {
  return JSON.stringify(buildSummarySemanticProjection(summary));
}

function attachRoundSummaryIntegrity(summary, parsed) {
  const numericEvidenceBinding = parsed.setupErrors.length === 0 && parsed.cadence !== "TOO_SHORT";
  summary.semanticProjectionVersion = WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION;
  summary.decision = {
    setupInvalid: parsed.setupErrors.length > 0,
    behaviorRed: parsed.behaviorErrors.length > 0,
    thresholdRed: numericEvidenceBinding && parsed.thresholdViolations.length > 0,
    frameRed: numericEvidenceBinding && parsed.frameRed === true,
    longTaskRed: numericEvidenceBinding && parsed.longTaskRed === true,
    cadenceInvalid: ["TOO_SHORT", "TOO_LONG"].includes(parsed.cadence),
    status: parsed.status,
    retryEligible: ["SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(parsed.status),
    validRedObserved: ["RED", "RED_BEHAVIOR"].includes(parsed.status),
  };
  summary.implementationDiagnosticBinding = buildImplementationDiagnosticBinding(summary);
  summary.semanticProjectionSha256 = createHash("sha256")
    .update(summarySemanticProjectionBytes(summary), "utf8").digest("hex");
  return summary;
}

function componentValues(parsed, surface) {
  const rows = surface === "common" ? parsed.metrics.samples : parsed.metrics.actions.map((action) => action.metrics);
  const values = {};
  for (const row of rows) {
    for (const [field, value] of Object.entries(row ?? {})) {
      if (finite(value) && value >= 0 && PUBLIC_COMPONENT_KEYS.has(field)) (values[field] ??= []).push(value);
    }
  }
  return values;
}

function cadenceCounts(receipt) {
  const counts = { total: 0, inRange: 0, tooShort: 0, tooLong: 0, delayedHost: 0 };
  for (const gap of receipt.cadenceGaps ?? []) {
    counts.total += 1;
    if (gap.rebasedAfterLateHost === true) counts.delayedHost += 1;
    const range = gap.nominalGapMs === 40 ? WEB06_THRESHOLDS.cadence.burst40
      : gap.nominalGapMs === 120 ? WEB06_THRESHOLDS.cadence.burst120
        : gap.nominalGapMs === 100 ? WEB06_THRESHOLDS.cadence.existing100
          : WEB06_THRESHOLDS.cadence.sustained60;
    if (gap.actualDriverGapMs < range.minMs) counts.tooShort += 1;
    else if (gap.actualDriverGapMs > range.maxMs) counts.tooLong += 1;
    else counts.inRange += 1;
  }
  return counts;
}

export function buildRoundEvidenceSummary(receipt, { surface = "internal" } = {}) {
  if (!["internal", "common"].includes(surface)) throw new Error("WEB06_SUMMARY_SURFACE_INVALID");
  const parsed = surface === "common" ? validateCommonSurfaceReceipt(receipt) : validateAndRecomputeReceipt(receipt);
  const numericEvidenceBinding = parsed.setupErrors.length === 0 && parsed.cadence !== "TOO_SHORT";
  const components = Object.fromEntries(Object.entries(
    numericEvidenceBinding ? componentValues(parsed, surface) : {},
  )
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([field, values]) => [field, distributionSummary(values)]));
  const outcomeCounts = Object.fromEntries(PUBLIC_OUTCOME_COUNT_KEYS.map((outcome) => [outcome, 0]));
  if (surface === "internal") {
    for (const action of receipt.actions ?? []) {
      if (ALLOWED_OUTCOMES.has(action.outcome)) outcomeCounts[action.outcome] += 1;
      else outcomeCounts.unclassified += 1;
    }
  } else {
    for (const sample of receipt.commonSamples ?? []) {
      const mapped = sample.outcome === "terminal" ? "committed" : sample.outcome;
      if (ALLOWED_OUTCOMES.has(mapped)) outcomeCounts[mapped] += 1;
      else outcomeCounts.unclassified += 1;
    }
  }
  const burstLatencies = (receipt.burstRecoveries ?? []).flatMap((recovery) => {
    const event = (receipt.events ?? []).find((item) => item.stepId === recovery.afterStepId && item.type === "keydown");
    const latency = finite(event?.normalizedEventAt) && finite(recovery.latestPaintAt)
      ? recovery.latestPaintAt - event.normalizedEventAt
      : undefined;
    return finite(latency) && latency >= 0
      ? [latency]
      : [];
  });
  const queueDepths = surface === "internal"
    ? (receipt.actions ?? []).map((action) => action.mainQueueDepth)
      .filter((value) => Number.isSafeInteger(value) && value >= 0)
    : [];
  const recoveryDepths = (receipt.burstRecoveries ?? []).map((recovery) => recovery.idleSnapshot?.queueDepth)
    .filter((value) => Number.isSafeInteger(value) && value >= 0);
  const frames = numericEvidenceBinding
    ? (receipt.interactionFrameIntervalsMs ?? []).filter((value) => finite(value) && value > 0)
    : [];
  const longTasks = numericEvidenceBinding
    ? (receipt.longTasks ?? []).filter((task) => finite(task.durationMs) && task.durationMs >= 0)
    : [];
  const overlappingLongTasks = longTasks.filter((task) => task.overlapsInteractionWindow === true);
  const idleLongTasks = longTasks.filter((task) => task.overlapsIdleControl === true);
  const longTaskDuration = (rows) => rows.reduce((sum, task) => sum + task.durationMs, 0);
  const commonSamples = receipt.commonSamples ?? [];
  const summary = {
    version: "web06-round-summary-v1",
    surface,
    metricContractVersion: receipt.metricContractVersion,
    scenarioRegistryVersion: receipt.scenarioRegistryVersion,
    behaviorPredicateVersion: receipt.behaviorPredicateVersion,
    sourceCommit: receipt.source?.commit,
    sourceTree: receipt.source?.tree,
    archiveSha256: receipt.source?.archiveSha256,
    buildInfoSha256: receipt.source?.buildInfoSha256,
    artifactManifestSha256: receipt.source?.artifactSha256,
    artifactResponseGuardSha256: receipt.source?.artifactResponseGuardSha256,
    artifactResponseGuardSummarySha256: receipt.source?.artifactResponseGuardSummarySha256,
    identityManifestSha256: receipt.source?.identityManifestSha256,
    runnerSourceManifestSha256: receipt.source?.runnerSourceManifestSha256,
    runnerToolingManifestSha256: receipt.source?.runnerToolingManifestSha256,
    runnerSourceObservationSha256: receipt.source?.runnerSourceObservationSha256,
    runnerSourcePostObservationSha256: receipt.source?.runnerSourcePostObservationSha256,
    observedEnvironmentSha256: receipt.source?.observedEnvironmentSha256,
    collectorContractSha256: receipt.source?.collectorContractSha256,
    environmentManifestSha256: receipt.source?.environmentManifestSha256,
    environmentId: receipt.source?.environmentId,
    selectedBranch: receipt.source?.selectedBranch,
    disposition: receipt.source?.disposition,
    scenarioRunId: receipt.scenarioRunId,
    scenarioId: receipt.scenarioId,
    schemaId: receipt.schemaId,
    roundId: receipt.roundId,
    attemptId: receipt.attemptId,
    mode: receipt.mode,
    measurementStarted: receipt.measurementStarted === true,
    measurementCompleted: receipt.measurementCompleted === true,
    status: parsed.status,
    retryEligible: ["SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(parsed.status),
    validRedObserved: ["RED", "RED_BEHAVIOR"].includes(parsed.status),
    setupErrorCodes: [...parsed.setupErrors],
    behaviorErrorCodes: [...parsed.behaviorErrors],
    thresholdViolations: [...parsed.thresholdViolations],
    counts: {
      events: (receipt.events ?? []).length,
      actions: surface === "internal" ? (receipt.actions ?? []).length : 0,
      coveringSamples: surface === "common"
        ? commonSamples.filter((sample) => sample.sampleKind === "covering").length
        : parsed.metrics.covering.length,
      terminalSamples: surface === "common"
        ? commonSamples.filter((sample) => sample.sampleKind === "terminal").length
        : parsed.metrics.terminal.length,
      unclassifiedSamples: surface === "common"
        ? commonSamples.filter((sample) => !["covering", "terminal"].includes(sample.sampleKind)).length
        : 0,
      interactionWindows: (receipt.interactionWindows ?? []).length,
    },
    outcomeCounts,
    cadenceVerdict: parsed.cadence,
    cadence: cadenceCounts(receipt),
    components,
    queue: {
      maxDepth: numericEvidenceBinding
        ? distributionOrNull([...queueDepths, ...recoveryDepths])?.max ?? null
        : null,
      endBurstDepth: numericEvidenceBinding ? recoveryDepths.at(-1) ?? null : null,
    },
    burst: {
      recoveryCount: (receipt.burstRecoveries ?? []).length,
      finalKeyToLatestPaintMs: numericEvidenceBinding ? distributionOrNull(burstLatencies) : null,
    },
    frame: {
      intervals: distributionOrNull(frames),
      atOrAbove50MsCount: frames.filter((value) => value >= WEB06_THRESHOLDS.frame.rejectIntervalAtOrAboveMs).length,
    },
    longTask: {
      count: longTasks.length,
      durationMs: distributionOrNull(longTasks.map((task) => task.durationMs)),
      overlapCount: overlappingLongTasks.length,
      overlapDurationMs: distributionOrNull(overlappingLongTasks.map((task) => task.durationMs)),
      idleControlCount: idleLongTasks.length,
      idleControlDurationMs: distributionOrNull(idleLongTasks.map((task) => task.durationMs)),
      interactionMinusIdleCount: overlappingLongTasks.length - idleLongTasks.length,
      interactionMinusIdleDurationMs:
        longTaskDuration(overlappingLongTasks) - longTaskDuration(idleLongTasks),
    },
  };
  return attachRoundSummaryIntegrity(summary, parsed);
}

export function buildFiveRoundEvidenceSummary(receipts, { surface = "internal" } = {}) {
  validateFiveRoundPoolIdentity(receipts);
  const rounds = receipts.map((receipt) => buildRoundEvidenceSummary(receipt, { surface }));
  const componentNames = [...new Set(rounds.flatMap((round) => Object.keys(round.components)))].sort();
  const summaryErrors = [];
  const pooledComponents = Object.fromEntries(componentNames.map((field) => {
    const values = receipts.flatMap((receipt) => {
      const parsed = surface === "common" ? validateCommonSurfaceReceipt(receipt) : validateAndRecomputeReceipt(receipt);
      return componentValues(parsed, surface)[field] ?? [];
    });
    if (values.length === 0) summaryErrors.push(`pooled-component-missing:${field}`);
    return [field, distributionOrNull(values)];
  }));
  const pool = surface === "common" ? evaluateFiveRoundCommonPool(receipts) : evaluateFiveRoundPool(receipts);
  const summary = {
    version: "web06-five-round-summary-v1",
    surface,
    scenarioRunId: receipts[0].scenarioRunId,
    scenarioId: receipts[0].scenarioId,
    schemaId: receipts[0].schemaId,
    mode: receipts[0].mode,
    environmentManifestSha256: receipts[0].source.environmentManifestSha256,
    environmentId: receipts[0].source.environmentId,
    roundCount: rounds.length,
    status: pool.pass && summaryErrors.length === 0 ? "PASS" : "RED",
    validRedObserved: rounds.some((round) => round.validRedObserved)
      || pool.violations.length > 0 || summaryErrors.length > 0,
    roundSummaries: rounds,
    pooledComponents,
    pooledMetricsSha256: "",
    summaryErrors,
    poolViolations: [...pool.violations],
    pooledFrame: pool.pooledFrames,
    pooledLongTaskCount: pool.pooledLongTaskCount,
    pooledIdleLongTaskCount: receipts.reduce((sum, receipt) => sum
      + (receipt.longTasks ?? []).filter((task) => task.overlapsIdleControl === true).length, 0),
  };
  summary.pooledMetricsSha256 = publicPooledMetricsSha256(summary);
  summary.semanticProjectionVersion = WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION;
  summary.decision = {
    roundRed: rounds.some((round) => round.status !== "PASS"),
    poolViolationRed: summary.poolViolations.length > 0,
    summaryErrorRed: summary.summaryErrors.length > 0,
    status: summary.status,
    validRedObserved: summary.validRedObserved,
  };
  summary.implementationDiagnosticBinding = buildImplementationDiagnosticBinding(summary);
  summary.semanticProjectionSha256 = createHash("sha256")
    .update(summarySemanticProjectionBytes(summary), "utf8").digest("hex");
  return summary;
}

export const PEER_LOGICAL_INPUT_IDS = Object.freeze([
  "resolved-schema-includes-patches",
  "dictionary-and-imports",
  "essay",
  "grammar-model",
  "speller-algebra",
  "filters-and-options",
  "page-size-and-comments",
  "fresh-empty-userdb",
]);

const PEER_COMPILED_HASH_IDS = Object.freeze([
  "table",
  "prism",
  "reverse",
  "data-model",
  "runtime",
]);

function validateReproducibleSide(sideName, side, reasons) {
  if (!COMMIT_SHA_RE.test(side?.repositoryCommit ?? "")) reasons.push(`${sideName}-repository-commit`);
  if (side?.sourceTreeState !== "clean") reasons.push(`${sideName}-source-tree-state`);
  if (!isSha256(side?.artifactSha256)) reasons.push(`${sideName}-artifact-hash`);
  if (!isSha256(side?.generatedManifestSha256)) reasons.push(`${sideName}-generated-manifest-hash`);
  if (!isSha256(side?.completeArtifactManifestSha256)) reasons.push(`${sideName}-complete-artifact-manifest-hash`);
  if (typeof side?.buildCommand !== "string" || !side.buildCommand) reasons.push(`${sideName}-build-command`);
  const packageManager = side?.packageManager;
  if (!packageManager || !["npm", "pnpm"].includes(packageManager.name)
    || typeof packageManager.version !== "string" || !packageManager.version
    || !isSha256(packageManager.lockSha256)
    || !isSha256(packageManager.integrityManifestSha256)) {
    reasons.push(`${sideName}-dependency-resolution`);
  }
  const toolchain = side?.toolchain;
  if (!toolchain || typeof toolchain.nodeVersion !== "string" || !toolchain.nodeVersion
    || typeof toolchain.emscriptenVersion !== "string" || !toolchain.emscriptenVersion
    || !COMMIT_SHA_RE.test(toolchain.emscriptenCommit ?? "")
    || typeof toolchain.compilerVersion !== "string" || !toolchain.compilerVersion) {
    reasons.push(`${sideName}-toolchain`);
  }
  const recipes = side?.resolvedRecipes;
  if (!Array.isArray(recipes) || recipes.length === 0) {
    reasons.push(`${sideName}-resolved-recipes`);
  } else {
    const recipeIds = recipes.map((recipe) => recipe.id);
    if (new Set(recipeIds).size !== recipeIds.length) reasons.push(`${sideName}-resolved-recipes-duplicate`);
    for (const recipe of recipes) {
      if (typeof recipe.id !== "string" || !recipe.id
        || typeof recipe.repository !== "string" || !recipe.repository
        || !COMMIT_SHA_RE.test(recipe.commit ?? "")
        || !isSha256(recipe.logicalBytesSha256)) {
        reasons.push(`${sideName}-resolved-recipe:${recipe.id ?? "unknown"}`);
      }
    }
  }
  for (const hashId of PEER_COMPILED_HASH_IDS) {
    if (!isSha256(side?.compiledHashes?.[hashId])) reasons.push(`${sideName}-compiled-${hashId}`);
  }
}

export function evaluatePackageAlignment(manifest) {
  const reasons = [];
  if (!manifest || manifest.version !== "web06-peer-data-v1") reasons.push("manifest-version");
  for (const side of ["yune", "peer"]) {
    validateReproducibleSide(side, manifest?.[side], reasons);
  }
  const logical = new Map((manifest?.logicalInputs ?? []).map((item) => [item.id, item]));
  for (const id of PEER_LOGICAL_INPUT_IDS) {
    const item = logical.get(id);
    if (!item) {
      reasons.push(`logical-input-missing:${id}`);
      continue;
    }
    if (id === "grammar-model" && item.explicitNone === true) {
      if (item.yuneSha256 !== "none" || item.peerSha256 !== "none") reasons.push("grammar-explicit-none-mismatch");
    } else if (!isSha256(item.yuneSha256) || !isSha256(item.peerSha256)) {
      reasons.push(`logical-input-hash:${id}`);
    }
    if (item.yuneSha256 !== item.peerSha256) reasons.push(`logical-input-different:${id}`);
  }
  if (logical.size !== PEER_LOGICAL_INPUT_IDS.length) reasons.push("logical-input-extra-or-duplicate");
  if (manifest?.effectiveConfiguration?.yuneSha256 !== manifest?.effectiveConfiguration?.peerSha256
    || !isSha256(manifest?.effectiveConfiguration?.yuneSha256)) reasons.push("effective-configuration-different");
  if (manifest?.freshEmptyUserdb !== true) reasons.push("fresh-empty-userdb-not-proved");
  if (manifest?.sameEndpointObserver !== true) reasons.push("same-endpoint-observer-not-proved");
  return { packageAlignment: reasons.length ? "DATA_CONFOUNDED" : "PROVED", reasons };
}

export function evaluateCollectionEquivalence(modes) {
  const violations = [];
  const ordered = [modes?.product, modes?.minimal, modes?.full];
  if (ordered.some((mode) => !mode)) return { pass: false, violations: ["missing-mode"] };
  const digestFields = [
    "publicActionResultShapeSha256",
    "publicErrorShapeSha256",
    "nativeBindingCallSequenceSha256",
    "decodedJsonSha256",
  ];
  for (const field of digestFields) {
    if (ordered.some((mode) => !isSha256(mode[field]))) violations.push(`${field}:invalid`);
    if (new Set(ordered.map((mode) => mode[field])).size !== 1) violations.push(`${field}:different`);
  }
  if (ordered.some((mode) => !Number.isInteger(mode.nativeBindingCallCount) || mode.nativeBindingCallCount < 0)
    || new Set(ordered.map((mode) => mode.nativeBindingCallCount)).size !== 1) {
    violations.push("native-binding-call-count");
  }
  for (const [index, mode] of ordered.entries()) {
    const audit = mode.responsePointerAudit;
    const label = ["product", "minimal", "full"][index];
    if (!audit || !Number.isInteger(audit.nonzeroResponseCount) || !Number.isInteger(audit.freeCount)
      || audit.nonzeroResponseCount !== audit.freeCount || audit.duplicateFreeCount !== 0 || audit.zeroFreeCount !== 0) {
      violations.push(`${label}:pointer-free-ownership`);
    }
    const isolation = mode.collectorExceptionIsolation;
    if (!isolation || isolation.publicResultUnchanged !== true || isolation.publicErrorUnchanged !== true
      || isolation.measurementFailsClosed !== true) {
      violations.push(`${label}:collector-exception-isolation`);
    }
    const privacy = validatePointerFreePrivacy(mode.publicTrace);
    if (!privacy.pass) violations.push(`${label}:public-trace-privacy`);
  }
  return { pass: violations.length === 0, violations };
}

export function computeBindingPeerRatio({ yuneMs, peerMs, manifest }) {
  const alignment = evaluatePackageAlignment(manifest);
  if (alignment.packageAlignment !== "PROVED") {
    throw new Error(`WEB06_BINDING_RATIO_REFUSED:${alignment.reasons.join(",")}`);
  }
  if (yuneMs?.length !== 5 || peerMs?.length !== 5) throw new Error("WEB06_PEER_RATIO_REQUIRES_FIVE_ROUNDS");
  const yuneP95 = distributionSummary(yuneMs).p95;
  const peerP95 = distributionSummary(peerMs).p95;
  if (!(peerP95 > 0)) throw new Error("WEB06_PEER_RATIO_INVALID_DENOMINATOR");
  const ratio = yuneP95 / peerP95;
  return {
    packageAlignment: "PROVED",
    yuneP95,
    peerP95,
    ratio,
    matchesOrBeats: ratio <= WEB06_THRESHOLDS.peer.matchesOrBeatsP95RatioMax,
  };
}

const PUBLIC_ROUND_KEYS = Object.freeze([
  "version", "surface", "metricContractVersion", "scenarioRegistryVersion", "behaviorPredicateVersion",
  "sourceCommit", "sourceTree", "archiveSha256", "buildInfoSha256", "artifactManifestSha256",
  "artifactResponseGuardSha256", "artifactResponseGuardSummarySha256",
  "identityManifestSha256", "runnerSourceManifestSha256", "runnerToolingManifestSha256",
  "runnerSourceObservationSha256", "runnerSourcePostObservationSha256", "observedEnvironmentSha256",
  "collectorContractSha256", "environmentManifestSha256", "environmentId", "selectedBranch", "disposition",
  "scenarioRunId", "scenarioId", "schemaId", "roundId", "attemptId", "mode", "measurementStarted",
  "measurementCompleted", "status", "retryEligible", "validRedObserved", "setupErrorCodes",
  "behaviorErrorCodes", "thresholdViolations", "counts", "outcomeCounts", "cadenceVerdict", "cadence",
  "components", "queue", "burst", "frame", "longTask", "semanticProjectionVersion",
  "semanticProjectionSha256", "implementationDiagnosticBinding", "decision",
]);
const PUBLIC_DISTRIBUTION_KEYS = Object.freeze(["count", "median", "p95", "p99", "max"]);
const PUBLIC_COMPONENT_KEYS = new Set([
  "eventDeliveryMs", "eventHandlerEnqueueMs", "mainQueueWaitMs",
  "workerMessageDeliveryMs", "workerPreActionWaitMs", "workerDispatchWaitMs",
  "workerProcessMs", "workerRoundtripMs", "workerAbiMs", "workerAdapterTranslateMs",
  "workerJsonParseMs", "workerResponseExtractMs",
  "mainResponseDispatchMs", "responseMappingMs", "stateScheduleMs", "reactCommitMs",
  "paintProxyMs", "eventToCurrentPaintMs", "handlerToCurrentPaintMs", "timelineResidualMs",
  "preServiceWaitMs",
  "eventToObservationMs", "driverDispatchToObservationUpperBoundMs",
  "eventToCoveringPaintMs", "driverDispatchToCoveringPaintUpperBoundMs",
  "eventToTerminalObservationMs", "driverDispatchToTerminalUpperBoundMs",
  "preServiceWaitUpperBoundMs", "supersessionSequenceLag", "supersessionTimeMs",
]);

function publicExactKeys(value, allowed, errors, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    errors.push(`${label}:object`);
    return false;
  }
  const actual = Object.keys(value).sort();
  const expected = [...allowed].sort();
  if (!sameJson(actual, expected)) errors.push(`${label}:keys:${actual.join(",")}`);
  return true;
}

function publicStringArray(value, errors, label) {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) errors.push(`${label}:string-array`);
}

function validatePublicDiagnosticBinding(summary, errors, label) {
  const binding = summary.implementationDiagnosticBinding;
  const expectedKeys = summary.version === "web06-five-round-summary-v1"
    ? ["version", "dimensions", "roundBindingsSha256"]
    : ["version", "dimensions"];
  if (!publicExactKeys(binding, expectedKeys, errors, `${label}.implementationDiagnosticBinding`)) return;
  if (binding.version !== WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION) {
    errors.push(`${label}.implementationDiagnosticBinding:version`);
  }
  const fields = diagnosticFieldsForSummary(summary);
  if (!publicExactKeys(binding.dimensions, fields, errors, `${label}.implementationDiagnosticBinding.dimensions`)) return;
  for (const field of fields) {
    const dimension = binding.dimensions[field];
    if (!publicExactKeys(dimension, ["rawCount", "rawSha256", "semanticCount", "semanticSha256"],
      errors, `${label}.implementationDiagnosticBinding.dimensions.${field}`)) continue;
    const raw = summary[field];
    if (!Array.isArray(raw)) {
      errors.push(`${label}.implementationDiagnosticBinding.dimensions.${field}:raw-array`);
      continue;
    }
    const semantic = canonicalizeSummaryDiagnosticCodes(raw);
    if (!Number.isSafeInteger(dimension.rawCount) || dimension.rawCount < 0
      || !Number.isSafeInteger(dimension.semanticCount) || dimension.semanticCount < 0
      || !isSha256(dimension.rawSha256) || !isSha256(dimension.semanticSha256)
      || dimension.rawCount !== raw.length || dimension.rawSha256 !== sha256Json(raw)
      || dimension.semanticCount !== semantic.length || dimension.semanticSha256 !== sha256Json(semantic)) {
      errors.push(`${label}.implementationDiagnosticBinding.dimensions.${field}:binding`);
    }
  }
  if (summary.version === "web06-five-round-summary-v1") {
    const expected = sha256Json(summary.roundSummaries.map((round) => round.implementationDiagnosticBinding));
    if (binding.roundBindingsSha256 !== expected) {
      errors.push(`${label}.implementationDiagnosticBinding:round-bindings`);
    }
  }
}

function publicDistribution(value, errors, label, nullable = true) {
  if (value === null && nullable) return;
  if (!publicExactKeys(value, PUBLIC_DISTRIBUTION_KEYS, errors, label)) return;
  if (!Number.isSafeInteger(value.count) || value.count <= 0
    || [value.median, value.p95, value.p99, value.max].some((item) => !finite(item) || item < 0)) {
    errors.push(`${label}:values`);
  } else if (!(value.median <= value.p95 && value.p95 <= value.p99 && value.p99 <= value.max)) {
    errors.push(`${label}:order`);
  }
}

function publicPooledMetricsSha256(summary) {
  return createHash("sha256").update(JSON.stringify({
    pooledComponents: summary.pooledComponents,
    pooledFrame: summary.pooledFrame,
    pooledLongTaskCount: summary.pooledLongTaskCount,
    pooledIdleLongTaskCount: summary.pooledIdleLongTaskCount,
  }), "utf8").digest("hex");
}

function validatePublicRoundSemantics(summary, errors, label) {
  const outcomeTotal = Object.values(summary.outcomeCounts ?? {}).reduce((sum, count) => sum + count, 0);
  const expectedOutcomeTotal = summary.surface === "internal"
    ? summary.counts?.actions
    : (summary.counts?.coveringSamples ?? 0) + (summary.counts?.terminalSamples ?? 0)
      + (summary.counts?.unclassifiedSamples ?? 0);
  if (outcomeTotal !== expectedOutcomeTotal) errors.push(`${label}:outcome-count-link`);

  const cadence = summary.cadence ?? {};
  if (cadence.total !== cadence.inRange + cadence.tooShort + cadence.tooLong
    || cadence.delayedHost > cadence.total) errors.push(`${label}:cadence-count-link`);
  const row = hasOwn(SCENARIO_REGISTRY, summary.scenarioId)
    ? SCENARIO_REGISTRY[summary.scenarioId]
    : undefined;
  if (!row) {
    errors.push(`${label}:scenario`);
    return;
  }
  const expectedCadenceVerdict = row.cadence === "same-task-pressure"
    ? "NOT_APPLICABLE"
    : cadence.tooShort > 0 ? "TOO_SHORT" : cadence.tooLong > 0 ? "TOO_LONG" : "IN_RANGE";
  if (summary.cadenceVerdict !== expectedCadenceVerdict) errors.push(`${label}:cadence-verdict-link`);

  const binding = row.binding === true || row.binding === "branch-b-only";
  const frameRed = binding && ((summary.frame?.atOrAbove50MsCount ?? 0) > 0
    || (summary.frame?.intervals?.p99 ?? -Infinity) > WEB06_THRESHOLDS.frame.p99Ms.max);
  const longTaskRed = binding
    && (summary.longTask?.overlapDurationMs?.max ?? -Infinity) >= WEB06_THRESHOLDS.frame.rejectLongTaskAtOrAboveMs;
  const disposition = finalAttemptStatus({
    setupErrors: summary.setupErrorCodes ?? [],
    behaviorErrors: summary.behaviorErrorCodes ?? [],
    cadence: summary.cadenceVerdict,
    thresholdViolations: summary.thresholdViolations ?? [],
    frameRed,
    longTaskRed,
  });
  if (summary.status !== disposition.status || summary.validRedObserved !== disposition.validRedObserved
    || summary.retryEligible !== disposition.retryEligible) errors.push(`${label}:disposition-link`);
  const expectedDecision = {
    setupInvalid: (summary.setupErrorCodes?.length ?? 0) > 0,
    behaviorRed: (summary.behaviorErrorCodes?.length ?? 0) > 0,
    thresholdRed: (summary.setupErrorCodes?.length ?? 0) === 0
      && summary.cadenceVerdict !== "TOO_SHORT"
      && (summary.thresholdViolations?.length ?? 0) > 0,
    frameRed,
    longTaskRed,
    cadenceInvalid: ["TOO_SHORT", "TOO_LONG"].includes(summary.cadenceVerdict),
    status: disposition.status,
    retryEligible: disposition.retryEligible,
    validRedObserved: disposition.validRedObserved,
  };
  if (!sameJson(summary.decision, expectedDecision)) errors.push(`${label}:decision-link`);
  if (summary.measurementStarted !== true || summary.measurementCompleted !== true) {
    errors.push(`${label}:measurement-completion`);
  }
  if ((summary.frame?.atOrAbove50MsCount ?? 0) > (summary.frame?.intervals?.count ?? 0)) {
    errors.push(`${label}:frame-count-link`);
  }
  const longTask = summary.longTask ?? {};
  if ((longTask.durationMs === null ? 0 : longTask.durationMs?.count) !== longTask.count
    || (longTask.overlapDurationMs === null ? 0 : longTask.overlapDurationMs?.count) !== longTask.overlapCount
    || (longTask.idleControlDurationMs === null ? 0 : longTask.idleControlDurationMs?.count)
      !== longTask.idleControlCount
    || longTask.interactionMinusIdleCount !== longTask.overlapCount - longTask.idleControlCount) {
    errors.push(`${label}:long-task-count-link`);
  }
}

function validatePublicRoundSummary(summary, errors, label = "roundSummary") {
  if (!publicExactKeys(summary, PUBLIC_ROUND_KEYS, errors, label)) return;
  if (summary.version !== "web06-round-summary-v1" || !["common", "internal"].includes(summary.surface)
    || summary.metricContractVersion !== WEB06_METRIC_CONTRACT_VERSION
    || summary.scenarioRegistryVersion !== WEB06_SCENARIO_REGISTRY_VERSION
    || summary.behaviorPredicateVersion !== WEB06_BEHAVIOR_PREDICATE_VERSION
    || !WEB06_SELECTED_BRANCHES.includes(summary.selectedBranch)
    || !WEB06_DISPOSITIONS.includes(summary.disposition)
    || !["PASS", "RED", "RED_BEHAVIOR", "SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(summary.status)
    || !["IN_RANGE", "TOO_SHORT", "TOO_LONG", "NOT_APPLICABLE"].includes(summary.cadenceVerdict)) {
    errors.push(`${label}:identity`);
  }
  let semanticProjectionValid = summary.semanticProjectionVersion === WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION
    && isSha256(summary.semanticProjectionSha256);
  try {
    semanticProjectionValid = semanticProjectionValid
      && summary.semanticProjectionSha256 === createHash("sha256")
        .update(summarySemanticProjectionBytes(summary), "utf8").digest("hex");
  } catch {
    semanticProjectionValid = false;
  }
  if (!semanticProjectionValid) {
    errors.push(`${label}:semantic-projection`);
  }
  try {
    validatePublicDiagnosticBinding(summary, errors, label);
  } catch {
    errors.push(`${label}:diagnostic-binding`);
  }
  for (const key of ["environmentId", "scenarioRunId", "scenarioId", "schemaId", "roundId", "attemptId", "mode"]) {
    if (typeof summary[key] !== "string" || summary[key].length === 0) errors.push(`${label}.${key}:string`);
  }
  for (const key of ["sourceCommit", "sourceTree", "archiveSha256", "buildInfoSha256", "artifactManifestSha256",
    "artifactResponseGuardSha256", "artifactResponseGuardSummarySha256",
    "identityManifestSha256", "runnerSourceManifestSha256", "runnerToolingManifestSha256",
    "runnerSourceObservationSha256", "runnerSourcePostObservationSha256", "observedEnvironmentSha256",
    "collectorContractSha256", "environmentManifestSha256"]) {
    const valid = key === "sourceCommit" || key === "sourceTree" ? COMMIT_SHA_RE.test(summary[key] ?? "") : isSha256(summary[key]);
    if (!valid) errors.push(`${label}.${key}:hash`);
  }
  if (typeof summary.measurementStarted !== "boolean" || typeof summary.measurementCompleted !== "boolean"
    || typeof summary.retryEligible !== "boolean" || typeof summary.validRedObserved !== "boolean") {
    errors.push(`${label}:booleans`);
  }
  for (const key of ["setupErrorCodes", "behaviorErrorCodes", "thresholdViolations"]) {
    publicStringArray(summary[key], errors, `${label}.${key}`);
  }
  if (publicExactKeys(summary.counts,
    ["events", "actions", "coveringSamples", "terminalSamples", "unclassifiedSamples", "interactionWindows"],
  errors, `${label}.counts`)
    && Object.values(summary.counts).some((item) => !Number.isSafeInteger(item) || item < 0)) errors.push(`${label}.counts:values`);
  if (publicExactKeys(summary.outcomeCounts, PUBLIC_OUTCOME_COUNT_KEYS, errors, `${label}.outcomeCounts`)
    && Object.values(summary.outcomeCounts).some((item) => !Number.isSafeInteger(item) || item < 0)) {
    errors.push(`${label}.outcomeCounts:values`);
  }
  if (publicExactKeys(summary.cadence,
    ["total", "inRange", "tooShort", "tooLong", "delayedHost"], errors, `${label}.cadence`)
    && Object.values(summary.cadence).some((item) => !Number.isSafeInteger(item) || item < 0)) errors.push(`${label}.cadence:values`);
  if (!summary.components || typeof summary.components !== "object" || Array.isArray(summary.components)) {
    errors.push(`${label}.components:object`);
  } else {
    for (const [key, value] of Object.entries(summary.components)) {
      if (!PUBLIC_COMPONENT_KEYS.has(key)) errors.push(`${label}.components.${key}:unknown`);
      publicDistribution(value, errors, `${label}.components.${key}`, false);
    }
  }
  if (publicExactKeys(summary.queue, ["maxDepth", "endBurstDepth"], errors, `${label}.queue`)
    && [summary.queue.maxDepth, summary.queue.endBurstDepth].some((item) =>
      item !== null && (!Number.isSafeInteger(item) || item < 0))) errors.push(`${label}.queue:values`);
  if (publicExactKeys(summary.burst, ["recoveryCount", "finalKeyToLatestPaintMs"], errors, `${label}.burst`)) {
    if (!Number.isSafeInteger(summary.burst.recoveryCount) || summary.burst.recoveryCount < 0) errors.push(`${label}.burst:count`);
    publicDistribution(summary.burst.finalKeyToLatestPaintMs, errors, `${label}.burst.finalKeyToLatestPaintMs`);
  }
  if (publicExactKeys(summary.frame, ["intervals", "atOrAbove50MsCount"], errors, `${label}.frame`)) {
    publicDistribution(summary.frame.intervals, errors, `${label}.frame.intervals`);
    if (!Number.isSafeInteger(summary.frame.atOrAbove50MsCount) || summary.frame.atOrAbove50MsCount < 0) errors.push(`${label}.frame:count`);
  }
  if (publicExactKeys(summary.longTask, ["count", "durationMs", "overlapCount", "overlapDurationMs",
    "idleControlCount", "idleControlDurationMs", "interactionMinusIdleCount", "interactionMinusIdleDurationMs"],
  errors, `${label}.longTask`)) {
    for (const key of ["count", "overlapCount", "idleControlCount", "interactionMinusIdleCount"]) {
      if (!Number.isSafeInteger(summary.longTask[key])) errors.push(`${label}.longTask.${key}:integer`);
    }
    for (const key of ["durationMs", "overlapDurationMs", "idleControlDurationMs"]) {
      publicDistribution(summary.longTask[key], errors, `${label}.longTask.${key}`);
    }
    if (!finite(summary.longTask.interactionMinusIdleDurationMs)) errors.push(`${label}.longTask:duration-delta`);
  }
  validatePublicRoundSemantics(summary, errors, label);
}

export function validatePublicRoundSummarySchema(summary) {
  const errors = [];
  validatePublicRoundSummary(summary, errors);
  const privacy = validatePointerFreePrivacy(summary);
  errors.push(...privacy.errors);
  return { pass: errors.length === 0, errors };
}

export function validatePublicFiveRoundSummarySchema(summary) {
  const errors = [];
  const keys = [
    "version", "surface", "scenarioRunId", "scenarioId", "schemaId", "mode",
    "environmentManifestSha256", "environmentId", "roundCount", "status", "validRedObserved",
    "roundSummaries", "pooledComponents", "pooledMetricsSha256", "summaryErrors", "poolViolations", "pooledFrame",
    "pooledLongTaskCount", "pooledIdleLongTaskCount",
    "semanticProjectionVersion", "semanticProjectionSha256", "implementationDiagnosticBinding", "decision",
  ];
  if (!publicExactKeys(summary, keys, errors, "fiveRoundSummary")) return { pass: false, errors };
  if (summary.version !== "web06-five-round-summary-v1" || !["common", "internal"].includes(summary.surface)
    || summary.roundCount !== 5 || !["PASS", "RED"].includes(summary.status)
    || typeof summary.validRedObserved !== "boolean" || !isSha256(summary.environmentManifestSha256)
    || !isSha256(summary.pooledMetricsSha256)
    || typeof summary.environmentId !== "string" || !summary.environmentId
    || ["scenarioRunId", "scenarioId", "schemaId", "mode"]
      .some((key) => typeof summary[key] !== "string" || summary[key].length === 0)) {
    errors.push("fiveRoundSummary:identity");
  }
  let semanticProjectionValid = summary.semanticProjectionVersion === WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION
    && isSha256(summary.semanticProjectionSha256);
  try {
    semanticProjectionValid = semanticProjectionValid
      && summary.semanticProjectionSha256 === createHash("sha256")
        .update(summarySemanticProjectionBytes(summary), "utf8").digest("hex");
  } catch {
    semanticProjectionValid = false;
  }
  if (!semanticProjectionValid) {
    errors.push("fiveRoundSummary:semantic-projection");
  }
  if (!Array.isArray(summary.roundSummaries) || summary.roundSummaries.length !== 5) {
    errors.push("fiveRoundSummary.roundSummaries:count");
  } else {
    summary.roundSummaries.forEach((round, index) => {
      validatePublicRoundSummary(round, errors, `fiveRoundSummary.roundSummaries[${index}]`);
      if (round.surface !== summary.surface || round.scenarioRunId !== summary.scenarioRunId
        || round.scenarioId !== summary.scenarioId || round.schemaId !== summary.schemaId
        || round.mode !== summary.mode || round.environmentManifestSha256 !== summary.environmentManifestSha256
        || round.environmentId !== summary.environmentId) errors.push(`fiveRoundSummary.roundSummaries[${index}]:identity-link`);
    });
  }
  if (!summary.pooledComponents || typeof summary.pooledComponents !== "object"
    || Array.isArray(summary.pooledComponents)) {
    errors.push("fiveRoundSummary.pooledComponents:object");
  } else {
    for (const [key, value] of Object.entries(summary.pooledComponents)) {
      if (!PUBLIC_COMPONENT_KEYS.has(key)) errors.push(`fiveRoundSummary.pooledComponents.${key}:unknown`);
      publicDistribution(value, errors, `fiveRoundSummary.pooledComponents.${key}`, false);
    }
  }
  publicStringArray(summary.summaryErrors, errors, "fiveRoundSummary.summaryErrors");
  publicStringArray(summary.poolViolations, errors, "fiveRoundSummary.poolViolations");
  publicDistribution(summary.pooledFrame, errors, "fiveRoundSummary.pooledFrame");
  if (!Number.isSafeInteger(summary.pooledLongTaskCount) || summary.pooledLongTaskCount < 0
    || !Number.isSafeInteger(summary.pooledIdleLongTaskCount) || summary.pooledIdleLongTaskCount < 0) {
    errors.push("fiveRoundSummary.longTaskCounts");
  }
  const expectedStatus = summary.roundSummaries?.every((round) => round.status === "PASS")
    && summary.summaryErrors?.length === 0 && summary.poolViolations?.length === 0 ? "PASS" : "RED";
  const expectedValidRed = summary.roundSummaries?.some((round) => round.validRedObserved === true)
    || (summary.summaryErrors?.length ?? 0) > 0 || (summary.poolViolations?.length ?? 0) > 0;
  if (summary.status !== expectedStatus || summary.validRedObserved !== expectedValidRed) {
    errors.push("fiveRoundSummary:disposition-link");
  }
  const expectedDecision = {
    roundRed: summary.roundSummaries?.some((round) => round.status !== "PASS") === true,
    poolViolationRed: (summary.poolViolations?.length ?? 0) > 0,
    summaryErrorRed: (summary.summaryErrors?.length ?? 0) > 0,
    status: expectedStatus,
    validRedObserved: expectedValidRed,
  };
  if (!sameJson(summary.decision, expectedDecision)) errors.push("fiveRoundSummary:decision-link");
  try {
    validatePublicDiagnosticBinding(summary, errors, "fiveRoundSummary");
  } catch {
    errors.push("fiveRoundSummary:diagnostic-binding");
  }
  if (summary.pooledMetricsSha256 !== publicPooledMetricsSha256(summary)) {
    errors.push("fiveRoundSummary:pooled-metrics-hash");
  }
  const privacy = validatePointerFreePrivacy(summary);
  errors.push(...privacy.errors);
  return { pass: errors.length === 0, errors };
}

/** Exact recursive allowlist for committed compact per-round evidence. */
export function validatePublicEvidenceSchema(compact) {
  const errors = [];
  const common = compact?.version === "web06-public-common-receipt-v1";
  const baseKeys = [
    "version", "sourceCommit", "sourceTree", "sourceArchiveSha256", "buildInfoSha256", "artifactSha256",
    "artifactResponseGuardSha256", "artifactResponseGuardSummarySha256",
    "identityManifestSha256", "runnerSourceManifestSha256", "runnerToolingManifestSha256",
    "runnerSourceObservationSha256", "runnerSourcePostObservationSha256", "observedEnvironmentSha256",
    "collectorContractSha256", "scenarioIdsSha256", "environmentManifestSha256", "environmentId",
    "selectedBranch", "disposition", "metricContractVersion", "scenarioRegistryVersion",
    "behaviorPredicateVersion", "scenarioId", "scenarioRunId", "schemaId", "roundId", "attemptId", "mode",
    "verdict", "eventCount", common ? "commonSampleCount" : "actionCount",
    ...(common ? [] : ["coveringSampleCount", "terminalSampleCount"]),
    "thresholdViolations", "behaviorErrorCodes", "setupErrorCodes", "rawPacketSha256", "roundSummary",
  ];
  if (!publicExactKeys(compact, baseKeys, errors, "publicReceipt")) return { pass: false, errors };
  if (!["web06-public-receipt-v1", "web06-public-common-receipt-v1"].includes(compact.version)
    || !COMMIT_SHA_RE.test(compact.sourceCommit ?? "") || !COMMIT_SHA_RE.test(compact.sourceTree ?? "")
    || compact.metricContractVersion !== WEB06_METRIC_CONTRACT_VERSION
    || compact.scenarioRegistryVersion !== WEB06_SCENARIO_REGISTRY_VERSION
    || compact.behaviorPredicateVersion !== WEB06_BEHAVIOR_PREDICATE_VERSION
    || !WEB06_SELECTED_BRANCHES.includes(compact.selectedBranch)
    || !WEB06_DISPOSITIONS.includes(compact.disposition)
    || !["PASS", "RED", "RED_BEHAVIOR", "SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(compact.verdict)
    || ["environmentId", "scenarioId", "scenarioRunId", "schemaId", "roundId", "attemptId", "mode"]
      .some((key) => typeof compact[key] !== "string" || compact[key].length === 0)) {
    errors.push("publicReceipt:identity");
  }
  for (const key of ["sourceArchiveSha256", "buildInfoSha256", "artifactSha256", "identityManifestSha256",
    "artifactResponseGuardSha256", "artifactResponseGuardSummarySha256",
    "runnerSourceManifestSha256", "runnerToolingManifestSha256", "runnerSourceObservationSha256",
    "runnerSourcePostObservationSha256", "observedEnvironmentSha256", "collectorContractSha256",
    "scenarioIdsSha256", "environmentManifestSha256", "rawPacketSha256"]) {
    if (!isSha256(compact[key])) errors.push(`publicReceipt.${key}:hash`);
  }
  for (const key of ["eventCount", common ? "commonSampleCount" : "actionCount",
    ...(common ? [] : ["coveringSampleCount", "terminalSampleCount"])]) {
    if (!Number.isSafeInteger(compact[key]) || compact[key] < 0) errors.push(`publicReceipt.${key}:count`);
  }
  for (const key of ["thresholdViolations", "behaviorErrorCodes", "setupErrorCodes"]) {
    publicStringArray(compact[key], errors, `publicReceipt.${key}`);
  }
  validatePublicRoundSummary(compact.roundSummary, errors);
  const roundLinks = {
    sourceCommit: "sourceCommit",
    sourceTree: "sourceTree",
    sourceArchiveSha256: "archiveSha256",
    buildInfoSha256: "buildInfoSha256",
    artifactSha256: "artifactManifestSha256",
    artifactResponseGuardSha256: "artifactResponseGuardSha256",
    artifactResponseGuardSummarySha256: "artifactResponseGuardSummarySha256",
    identityManifestSha256: "identityManifestSha256",
    runnerSourceManifestSha256: "runnerSourceManifestSha256",
    runnerToolingManifestSha256: "runnerToolingManifestSha256",
    runnerSourceObservationSha256: "runnerSourceObservationSha256",
    runnerSourcePostObservationSha256: "runnerSourcePostObservationSha256",
    observedEnvironmentSha256: "observedEnvironmentSha256",
    collectorContractSha256: "collectorContractSha256",
    environmentManifestSha256: "environmentManifestSha256",
    environmentId: "environmentId",
    selectedBranch: "selectedBranch",
    disposition: "disposition",
    metricContractVersion: "metricContractVersion",
    scenarioRegistryVersion: "scenarioRegistryVersion",
    behaviorPredicateVersion: "behaviorPredicateVersion",
    scenarioRunId: "scenarioRunId",
    scenarioId: "scenarioId",
    schemaId: "schemaId",
    roundId: "roundId",
    attemptId: "attemptId",
    mode: "mode",
    verdict: "status",
    setupErrorCodes: "setupErrorCodes",
    behaviorErrorCodes: "behaviorErrorCodes",
    thresholdViolations: "thresholdViolations",
  };
  const linked = Object.entries(roundLinks).every(([compactKey, summaryKey]) =>
    sameJson(compact[compactKey], compact.roundSummary?.[summaryKey]));
  if (compact.roundSummary?.surface !== (common ? "common" : "internal")
    || compact.roundSummary?.counts?.events !== compact.eventCount
    || (common && compact.commonSampleCount !== (compact.roundSummary?.counts?.coveringSamples ?? 0)
      + (compact.roundSummary?.counts?.terminalSamples ?? 0)
      + (compact.roundSummary?.counts?.unclassifiedSamples ?? 0))
    || (!common && (compact.roundSummary?.counts?.actions !== compact.actionCount
      || compact.roundSummary?.counts?.coveringSamples !== compact.coveringSampleCount
      || compact.roundSummary?.counts?.terminalSamples !== compact.terminalSampleCount))
    || !linked) errors.push("publicReceipt:round-summary-link");
  const privacy = validatePointerFreePrivacy(compact);
  errors.push(...privacy.errors);
  return { pass: errors.length === 0, errors };
}

/** Compact evidence deliberately carries digests and verdicts, never raw spans. */
export function publicEvidenceReceipt({ receipt, parsed, rawPacketSha256 }) {
  if (!isSha256(rawPacketSha256)) throw new Error("WEB06_RAW_PACKET_HASH_INVALID");
  const compact = {
    version: "web06-public-receipt-v1",
    sourceCommit: receipt.source.commit,
    sourceTree: receipt.source.tree,
    sourceArchiveSha256: receipt.source.archiveSha256,
    buildInfoSha256: receipt.source.buildInfoSha256,
    artifactSha256: receipt.source.artifactSha256,
    artifactResponseGuardSha256: receipt.source.artifactResponseGuardSha256,
    artifactResponseGuardSummarySha256: receipt.source.artifactResponseGuardSummarySha256,
    identityManifestSha256: receipt.source.identityManifestSha256,
    runnerSourceManifestSha256: receipt.source.runnerSourceManifestSha256,
    runnerToolingManifestSha256: receipt.source.runnerToolingManifestSha256,
    runnerSourceObservationSha256: receipt.source.runnerSourceObservationSha256,
    runnerSourcePostObservationSha256: receipt.source.runnerSourcePostObservationSha256,
    observedEnvironmentSha256: receipt.source.observedEnvironmentSha256,
    collectorContractSha256: receipt.source.collectorContractSha256,
    scenarioIdsSha256: receipt.source.scenarioIdsSha256,
    environmentManifestSha256: receipt.source.environmentManifestSha256,
    environmentId: receipt.source.environmentId,
    selectedBranch: receipt.source.selectedBranch,
    disposition: receipt.source.disposition,
    metricContractVersion: receipt.metricContractVersion,
    scenarioRegistryVersion: receipt.scenarioRegistryVersion,
    behaviorPredicateVersion: receipt.behaviorPredicateVersion,
    scenarioId: receipt.scenarioId,
    scenarioRunId: receipt.scenarioRunId,
    schemaId: receipt.schemaId,
    roundId: receipt.roundId,
    attemptId: receipt.attemptId,
    mode: receipt.mode,
    verdict: parsed.status,
    eventCount: receipt.events.length,
    actionCount: receipt.actions.length,
    coveringSampleCount: parsed.metrics.covering.length,
    terminalSampleCount: parsed.metrics.terminal.length,
    thresholdViolations: [...parsed.thresholdViolations],
    behaviorErrorCodes: [...parsed.behaviorErrors],
    setupErrorCodes: [...parsed.setupErrors],
    rawPacketSha256,
    roundSummary: buildRoundEvidenceSummary(receipt, { surface: "internal" }),
  };
  const schema = validatePublicEvidenceSchema(compact);
  if (!schema.pass) throw new Error(`WEB06_PUBLIC_EVIDENCE_SCHEMA:${schema.errors.join(",")}`);
  return compact;
}
