import {
  ACTION_REGISTRY,
  SCENARIO_REGISTRY,
  WEB06_METRIC_CONTRACT_VERSION,
  WEB06_SCENARIO_REGISTRY_VERSION,
  WEB06_THRESHOLDS,
  buildClockCalibration,
  correctDriverTimestamp,
  correctWorkerTimestamp,
  distributionSummary,
  evaluateThresholdDistribution,
  expandScenarioExpectedTimeline,
  isSha256,
} from "./web06-metric-contract.mjs";
import { createHash } from "node:crypto";

const COMMIT_SHA_RE = /^[0-9a-f]{40}$/;
const FORBIDDEN_KEY_RE = /(?:ptr|pointer|address|authorization|cookie|password|secret|stack|browserProfile|userDataDir|(?:access|auth|bearer|api)Token)/i;
const ABSOLUTE_PATH_RE = /(?:file:\/\/|\/Users\/|\/home\/|[A-Za-z]:\\Users\\)/;
const POINTER_VALUE_RE = /(?:^|\s)0x[0-9a-f]{6,}(?:$|\s)/i;
const ALLOWED_OUTCOMES = new Set([
  "painted",
  "superseded",
  "committed",
  "processed-no-visual-change",
  "barrier-completed",
  "failure",
]);
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
    if (typeof value === "string" && (ABSOLUTE_PATH_RE.test(value) || POINTER_VALUE_RE.test(value))) {
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
  pushIf(setupErrors, !COMMIT_SHA_RE.test(receipt.source?.commit ?? ""), "SETUP_SOURCE_COMMIT_INVALID");
  pushIf(setupErrors, receipt.source?.treeState !== "clean", "SETUP_SOURCE_TREE_NOT_CLEAN");
  pushIf(setupErrors, !isSha256(receipt.source?.artifactSha256), "SETUP_ARTIFACT_HASH_INVALID");
  pushIf(setupErrors, !["PRODUCT", "BASE_MINIMAL", "BASE_FULL", "FINAL_MINIMAL", "FINAL_FULL"].includes(receipt.mode), "SETUP_MODE_INVALID");
  pushIf(setupErrors, typeof receipt.roundId !== "string" || !receipt.roundId, "SETUP_ROUND_ID_INVALID");
  pushIf(setupErrors, typeof receipt.attemptId !== "string" || !receipt.attemptId, "SETUP_ATTEMPT_ID_INVALID");
}

function validateInternalMode(receipt, setupErrors) {
  if (!INTERNAL_RECEIPT_MODES.has(receipt.mode)) {
    setupErrors.push(receipt.mode === "PRODUCT"
      ? "SETUP_PRODUCT_INTERNAL_RECEIPT_FORBIDDEN"
      : "SETUP_INTERNAL_MODE_INVALID");
  }
}

function validateEventClock(receipt, setupErrors, behaviorErrors) {
  const probe = receipt.eventClockProbe;
  if (!probe || !finite(probe.beforeDispatchAt) || !finite(probe.eventTimestamp) || !finite(probe.afterDispatchAt)) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_PROBE");
  } else if (probe.eventTimestamp < probe.beforeDispatchAt || probe.eventTimestamp > probe.afterDispatchAt) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_ORIGIN");
  }
  let previousTimestamp = -Infinity;
  for (const event of receipt.events ?? []) {
    if (!finite(event.eventTimestamp) || event.eventTimestamp < 0 || event.normalizedEventAt !== event.eventTimestamp) {
      setupErrors.push(`SETUP_INVALID_EVENT_TIMESTAMP:${event.eventSequenceId}`);
      continue;
    }
    if (event.eventTimestamp < previousTimestamp) setupErrors.push(`SETUP_DECREASING_EVENT_TIMESTAMP:${event.eventSequenceId}`);
    previousTimestamp = event.eventTimestamp;
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

function validateCommonEventClock(receipt, setupErrors, behaviorErrors) {
  const probe = receipt.eventClockProbe;
  if (!probe || !finite(probe.beforeDispatchAt) || !finite(probe.eventTimestamp) || !finite(probe.afterDispatchAt)) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_PROBE");
  } else if (probe.eventTimestamp < probe.beforeDispatchAt || probe.eventTimestamp > probe.afterDispatchAt) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_ORIGIN");
  }
  let previous = -Infinity;
  for (const event of receipt.events ?? []) {
    if (!finite(event.eventTimestamp) || event.eventTimestamp < 0 || event.normalizedEventAt !== event.eventTimestamp) {
      setupErrors.push(`SETUP_INVALID_EVENT_TIMESTAMP:${event.eventSequenceId}`);
      continue;
    }
    if (event.eventTimestamp < previous) setupErrors.push(`SETUP_DECREASING_EVENT_TIMESTAMP:${event.eventSequenceId}`);
    previous = event.eventTimestamp;
    if (!finite(event.sentinelObservedAt) || event.sentinelObservedAt < event.normalizedEventAt) {
      behaviorErrors.push(`COMMON_EVENT_SAME_REALM_ORDER:${event.eventSequenceId}`);
    }
  }
}

/**
 * PRODUCT has no internal WEB-06 protocol. This parser deliberately validates
 * only evidence observed by the identical Playwright sentinel in every mode.
 */
export function validateCommonSurfaceReceipt(receipt) {
  const setupErrors = [];
  const behaviorErrors = [];
  validateIdentity(receipt, setupErrors);
  const row = SCENARIO_REGISTRY[receipt.scenarioId];
  if (!row) setupErrors.push("SETUP_UNKNOWN_SCENARIO");
  behaviorErrors.push(...validatePointerFreePrivacy(receipt).errors);
  if (!row) return { status: "SETUP_INVALID", setupErrors, behaviorErrors };

  const expected = expandScenarioExpectedTimeline(row.id);
  const events = receipt.events ?? [];
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
    driverCalibration = buildClockCalibration(
      receipt.calibration?.driver?.pre,
      receipt.calibration?.driver?.post,
      "driver-page",
    );
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
      || sample.eventSequenceId !== frozen.eventSequenceId) {
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
    if (sample.stableDoubleRaf !== true || covering.stableDoubleRaf !== true
      || covering.domObserved?.input !== (sample.outcome === "superseded"
        ? frozenSamples.find((candidate) => candidate.stepId === sample.supersededByStepId)?.expectedInput
        : frozen.expectedInput)) {
      behaviorErrors.push(`COMMON_DOM_ENDPOINT:${frozen.stepId}`);
    }
    if (isCovering && !(covering.domObserved?.candidates?.length > 0)) {
      behaviorErrors.push(`COMMON_CANDIDATE_ENDPOINT:${frozen.stepId}`);
    }
    if (!finite(event.actualDriverDispatchAt) || !finite(event.requestedDriverDispatchAt)) {
      behaviorErrors.push(`COMMON_DRIVER_DISPATCH:${frozen.stepId}`);
      continue;
    }
    let correctedDriver;
    try {
      correctedDriver = driverCalibration
        ? correctDriverTimestamp(event.actualDriverDispatchAt, driverCalibration, event.actualDriverDispatchAt)
        : undefined;
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

  const cadence = cadenceVerdict(receipt, row, behaviorErrors, setupErrors);
  const frames = frameVerdict(receipt, row, behaviorErrors, setupErrors);
  const violations = [];
  const binding = row.binding === true || row.binding === "branch-b-only";
  if (binding) {
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
  const hardRed = behaviorErrors.length > 0 || violations.length > 0 || frames.frameRed || frames.longTaskRed;
  let status;
  if (setupErrors.length) status = "SETUP_INVALID";
  else if (cadence === "TOO_SHORT") status = behaviorErrors.length ? "RED_BEHAVIOR" : "NO_VERDICT_INVALID_CADENCE";
  else if (cadence === "TOO_LONG") status = hardRed ? "RED" : "NO_VERDICT_INVALID_CADENCE";
  else status = hardRed ? "RED" : "PASS";
  return {
    status,
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
    if (!ACTION_REGISTRY[actual.kind]) behaviorErrors.push(`ACTION_UNCLASSIFIED:${index + 1}`);
  }
  return expected;
}

function wireId(prefix, sequenceId) {
  return `web06-${prefix}-${String(sequenceId).padStart(8, "0")}`;
}

export function normalizeWireActionArgs(kind, wireArgs) {
  if (!Array.isArray(wireArgs)) throw new Error("WEB06_WIRE_ACTION_ARGS_INVALID");
  if (kind === "importUserdb") {
    if (wireArgs.length !== 1 || typeof wireArgs[0] !== "string") {
      throw new Error("WEB06_IMPORT_USERDB_ARGS_INVALID");
    }
    return [`sha256:${createHash("sha256").update(wireArgs[0], "utf8").digest("hex")}`];
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
    normalized = normalizeWireActionArgs(action.kind, action.wireArgs);
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
  const actionByLocalId = new Map((receipt.actions ?? []).map((action) => [action.actionId, action]));
  for (const event of receipt.events ?? []) {
    const sequenceId = eventStart + event.eventSequenceId - 1;
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
  for (const action of receipt.actions ?? []) {
    const sequenceId = actionStart + action.sequenceId - 1;
    const actionId = wireId("action", sequenceId);
    const wireEventSequenceId = action.eventSequenceId === undefined
      ? undefined
      : eventStart + action.eventSequenceId - 1;
    const wireEventId = wireEventSequenceId === undefined ? undefined : wireId("event", wireEventSequenceId);
    const wireCauseSequenceId = action.causedBySequenceId === undefined
      ? undefined
      : actionStart + action.causedBySequenceId - 1;
    const wireCauseActionId = wireCauseSequenceId === undefined ? undefined : wireId("action", wireCauseSequenceId);
    const wireCauseEventSequenceId = action.causedByEventSequenceId === undefined
      ? undefined
      : eventStart + action.causedByEventSequenceId - 1;
    const wireCauseEventId = wireCauseEventSequenceId === undefined ? undefined : wireId("event", wireCauseEventSequenceId);
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
    return {
      driver: buildClockCalibration(receipt.calibration?.driver?.pre, receipt.calibration?.driver?.post, "driver-page"),
      worker: buildClockCalibration(receipt.calibration?.worker?.pre, receipt.calibration?.worker?.post, "main-worker"),
    };
  } catch (error) {
    setupErrors.push(error instanceof Error ? error.message : "SETUP_INVALID_CLOCK_CALIBRATION");
    return null;
  }
}

function crossContextAction(action, calibrations, setupErrors) {
  try {
    const workerReference = (action.workerSentAt + action.mainResponseReceivedAt) / 2;
    const receive = correctWorkerTimestamp(action.workerMessageReceivedAt, calibrations.worker, workerReference);
    const start = correctWorkerTimestamp(action.workerActionStartedAt, calibrations.worker, workerReference);
    const finish = correctWorkerTimestamp(action.workerFinishedAt, calibrations.worker, workerReference);
    const driver = action.driverDispatchAt === undefined
      ? null
      : correctDriverTimestamp(action.driverDispatchAt, calibrations.driver, action.driverDispatchAt);
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

function validateFingerprints(action, mode, behaviorErrors) {
  if (!Array.isArray(rawActionSequence(action))) {
    behaviorErrors.push(`ACTION_RAW_SEQUENCE:${action.sequenceId}`);
  }
  const raw = action.engineRaw;
  if (MINIMAL_RECEIPT_MODES.has(mode)) {
    if (raw !== undefined) behaviorErrors.push(`MINIMAL_ENGINE_RAW_PRESENT:${action.sequenceId}`);
  } else if (FULL_RECEIPT_MODES.has(mode)) {
    if (!raw || raw.actionKind !== action.kind || raw.compositionEpochId !== action.compositionEpochId || raw.supersessionSubRunId !== action.supersessionSubRunId) {
      behaviorErrors.push(`ENGINE_RAW_IDENTITY:${action.sequenceId}`);
    }
    if (!Array.isArray(raw?.rawActionSequence)) behaviorErrors.push(`ENGINE_RAW_ACTION_SEQUENCE:${action.sequenceId}`);
    if (typeof raw?.rawResponseJson !== "string" || !isSha256(raw?.rawResponseSha256)
      || (typeof raw?.rawResponseJson === "string"
        && createHash("sha256").update(raw.rawResponseJson, "utf8").digest("hex") !== raw.rawResponseSha256)) {
      behaviorErrors.push(`ENGINE_RAW_PREPROJECTION_BYTES:${action.sequenceId}`);
    } else {
      try {
        JSON.parse(raw.rawResponseJson);
      } catch {
        behaviorErrors.push(`ENGINE_RAW_PREPROJECTION_JSON:${action.sequenceId}`);
      }
    }
    if (Array.isArray(action.rawActionSequence) && !sameJson(action.rawActionSequence, raw?.rawActionSequence)) {
      behaviorErrors.push(`ENGINE_RAW_SEQUENCE_DISAGREES:${action.sequenceId}`);
    }
  }
  if (["painted", "committed", "barrier-completed"].includes(action.outcome)) {
    if (!action.presentationExpected || !action.domObserved || !sameJson(action.presentationExpected, action.domObserved)) {
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
    if (!ALLOWED_OUTCOMES.has(action.outcome)) behaviorErrors.push(`ACTION_OUTCOME_UNKNOWN:${action.sequenceId}`);
    if (action.outcome === "failure") behaviorErrors.push(`ACTION_EXPLICIT_FAILURE:${action.sequenceId}`);
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
    validateFingerprints(action, mode, behaviorErrors);
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
  const row = SCENARIO_REGISTRY[scenarioId];
  if (!row) throw new Error(`WEB06_UNKNOWN_SCENARIO:${scenarioId}`);
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
  if (between.some((candidate) => candidate.classification !== "native-key" || !candidate.supersedable || candidate.outcome === "failure")) {
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
    validateSameRealmAction(action, event, behaviorErrors);
    validateWorkerSpans(action, receipt.mode, behaviorErrors);
    const corrected = crossContextAction(action, calibrations, setupErrors);
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

function frameVerdict(receipt, row, behaviorErrors, setupErrors) {
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
  if (!windows.length || windows.some((window) =>
    !finite(window.startedAt) || !finite(window.endedAt) || window.endedAt < window.startedAt)) {
    setupErrors.push("SETUP_INTERACTION_WINDOW_INVALID");
  }
  const observer = receipt.longTaskObserver;
  const firstWindowAt = windows.length ? Math.min(...windows.map((window) => window.startedAt)) : undefined;
  if (observer?.supported !== true || !finite(observer.installedAt)
    || !finite(firstWindowAt) || observer.installedAt > firstWindowAt) {
    setupErrors.push("SETUP_LONG_TASK_OBSERVER_UNAVAILABLE");
  }
  const focusSamples = receipt.focusVisibilitySamples ?? [];
  if (!focusSamples.length || focusSamples.some((sample) =>
    !finite(sample.recordedAt) || !sample.focused || sample.visibilityState !== "visible")) {
    setupErrors.push("SETUP_PAGE_NOT_FOREGROUND");
  } else if (windows.length) {
    const lastWindowAt = Math.max(...windows.map((window) => window.endedAt));
    const recorded = focusSamples.map((sample) => sample.recordedAt);
    if (Math.min(...recorded) > firstWindowAt || Math.max(...recorded) < lastWindowAt) {
      setupErrors.push("SETUP_FOREGROUND_PROOF_DOES_NOT_SPAN_WINDOW");
    }
  }
  const frames = receipt.interactionFrameIntervalsMs ?? [];
  if (!frames.length) setupErrors.push("SETUP_INTERACTION_FRAMES_MISSING");
  if (frames.some((value) => !finite(value) || value <= 0)) setupErrors.push("SETUP_NONFINITE_INTERACTION_FRAME_INTERVAL");
  if ((receipt.longTasks ?? []).some((task) =>
    !finite(task.startTime) || !finite(task.durationMs) || task.durationMs < 0)) {
    setupErrors.push("SETUP_NONFINITE_LONG_TASK");
  }
  const binding = row.binding === true || row.binding === "branch-b-only";
  const finiteFrames = frames.filter((value) => finite(value) && value > 0);
  const frameRed = binding && (finiteFrames.some((value) => value >= WEB06_THRESHOLDS.frame.rejectIntervalAtOrAboveMs)
    || (finiteFrames.length && distributionSummary(finiteFrames).p99 > WEB06_THRESHOLDS.frame.p99Ms.max));
  const longTaskRed = binding && (receipt.longTasks ?? []).some((task) =>
    task.overlapsInteractionWindow && task.durationMs >= WEB06_THRESHOLDS.frame.rejectLongTaskAtOrAboveMs);
  if ((receipt.assetsRequestedDuringWindow ?? []).length) behaviorErrors.push("ASSET_REQUEST_DURING_WINDOW");
  return { frameRed: Boolean(frameRed), longTaskRed };
}

function thresholdVerdict(receipt, row, derivedActions, behaviorErrors) {
  const covering = [];
  const terminal = [];
  const preService = [];
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
      const owner = terminalOwnerForStep(step, stepActions.map((action) => action.frozen));
      const primary = stepActions.find((action) => action.sequenceId === owner?.sequenceId);
      if (!primary || !finite(primary.metrics.eventToTerminalObservationMs) || !finite(primary.metrics.driverDispatchToTerminalUpperBoundMs)) {
        behaviorErrors.push(`TERMINAL_SAMPLE_MISSING:${step.id}`);
      } else terminal.push({ ...primary.metrics, stressDeadline: primary.frozen.stressDeadline === true });
    }
  }
  if (covering.length !== row.expectedCoveringSamples) behaviorErrors.push(`COVERING_COUNT:${covering.length}`);
  if (terminal.length !== row.expectedTerminalSamples) behaviorErrors.push(`TERMINAL_COUNT:${terminal.length}`);
  const violations = [];
  const expectedRecoveries = row.steps.filter((step) => step.declaredBurstPauseAfter === true);
  const recoveries = receipt.burstRecoveries ?? [];
  if (recoveries.length !== expectedRecoveries.length) {
    behaviorErrors.push(`BURST_RECOVERY_COUNT:${recoveries.length}!=${expectedRecoveries.length}`);
  }
  for (let index = 0; index < Math.min(recoveries.length, expectedRecoveries.length); index += 1) {
    const recovery = recoveries[index];
    const expectedStep = expectedRecoveries[index];
    const event = derivedActions.find((action) => action.stepId === expectedStep.id)?.event;
    if (!event || recovery.afterStepId !== expectedStep.id || !finite(recovery.latestPaintAt)
      || recovery.latestPaintAt < event.normalizedEventAt) {
      behaviorErrors.push(`BURST_RECOVERY_INVALID:${expectedStep.id}`);
      continue;
    }
    if (recovery.latestPaintAt - event.normalizedEventAt > 67) violations.push(`burst-recovery:${expectedStep.id}`);
    if (recovery.queueDepthBeforeNextBurst !== 0) violations.push(`burst-queue-not-zero:${expectedStep.id}`);
  }
  if (row.binding === true || row.binding === "branch-b-only") {
    for (const sample of covering) {
      if (sample.eventToCoveringPaintMs > WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs.max) violations.push("covering-max");
      if (sample.driverDispatchToCoveringPaintUpperBoundMs > WEB06_THRESHOLDS.sustained.driverDispatchToCoveringPaintUpperBoundMs.max) violations.push("driver-covering-max");
    }
    if (preService.length) {
      violations.push(...evaluateThresholdDistribution(preService, WEB06_THRESHOLDS.sustained.preServiceWaitUpperBoundMs, "pre-service").violations);
    }
    if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)) {
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
    for (const proof of receipt.pressureProofs ?? []) {
      const earlier = derivedActions.find((action) => action.sequenceId === proof.earlierSequenceId);
      const later = derivedActions.find((action) => action.sequenceId === proof.laterSequenceId);
      if (!earlier || !later
        || !(earlier.workerSentAt <= later.actionEnqueuedAt && later.actionEnqueuedAt < earlier.mainResponseReceivedAt)
        || later.mainQueueDepth < 1
        || earlier.mainResponseReceivedAt > later.workerActionStartedAtMainClock
        || earlier.terminalObservedAt > later.stateAppliedAt) {
        behaviorErrors.push(`FIFO_PRESSURE_NOT_PROVED:${proof.subcase ?? "unknown"}`);
      }
    }
    if ((receipt.pressureProofs ?? []).length !== new Set(row.steps.map((step) => step.subcase).filter(Boolean)).size) {
      behaviorErrors.push("FIFO_PRESSURE_PROOF_COUNT");
    }
  }
  return { covering, terminal, violations, latencyRed: violations.length > 0 };
}

export function validateAndRecomputeReceipt(receipt) {
  const setupErrors = [];
  const behaviorErrors = [];
  validateIdentity(receipt, setupErrors);
  validateInternalMode(receipt, setupErrors);
  const row = SCENARIO_REGISTRY[receipt.scenarioId];
  if (!row) setupErrors.push("SETUP_UNKNOWN_SCENARIO");
  const privacy = validatePointerFreePrivacy(receipt);
  behaviorErrors.push(...privacy.errors);
  if (!row) return { status: "SETUP_INVALID", setupErrors, behaviorErrors };
  const expected = validateTimelineShape(receipt, row, behaviorErrors);
  validateEventClock(receipt, setupErrors, behaviorErrors);
  const calibrations = calibrationFor(receipt, setupErrors);
  validateOutcomes(receipt.actions ?? [], expected, row, receipt.mode, behaviorErrors);
  let derivedActions = [];
  if (calibrations) {
    derivedActions = recomputeActions(receipt, row, expected, calibrations, setupErrors, behaviorErrors);
  }
  const cadence = cadenceVerdict(receipt, row, behaviorErrors, setupErrors);
  const frames = frameVerdict(receipt, row, behaviorErrors, setupErrors);
  const thresholds = calibrations
    ? thresholdVerdict(receipt, row, derivedActions, behaviorErrors)
    : { covering: [], terminal: [], violations: [], latencyRed: false };
  const behaviorRed = behaviorErrors.length > 0;
  const hardRed = behaviorRed || thresholds.latencyRed || frames.frameRed || frames.longTaskRed;
  let status;
  if (setupErrors.length) status = "SETUP_INVALID";
  else if (cadence === "TOO_SHORT") status = behaviorRed ? "RED_BEHAVIOR" : "NO_VERDICT_INVALID_CADENCE";
  else if (cadence === "TOO_LONG") status = hardRed ? "RED" : "NO_VERDICT_INVALID_CADENCE";
  else status = hardRed ? "RED" : "PASS";
  return {
    status,
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

export function evaluateFiveRoundPool(receipts) {
  if (!Array.isArray(receipts) || receipts.length !== WEB06_THRESHOLDS.attempts.requiredValid) {
    throw new Error("WEB06_POOL_REQUIRES_EXACTLY_FIVE_ROUNDS");
  }
  const parsed = receipts.map(validateAndRecomputeReceipt);
  if (parsed.some((result) => result.status === "SETUP_INVALID" || result.status === "NO_VERDICT_INVALID_CADENCE")) {
    throw new Error("WEB06_POOL_CONTAINS_INVALID_ROUND");
  }
  const scenarioId = receipts[0].scenarioId;
  if (receipts.some((receipt) => receipt.scenarioId !== scenarioId)) throw new Error("WEB06_CROSS_SCENARIO_POOL_FORBIDDEN");
  const row = SCENARIO_REGISTRY[scenarioId];
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
  if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)) {
    violations.push(...evaluateThresholdDistribution(pooledCovering, WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs, "pooled-covering").violations);
  }
  if ((row.binding === true || row.binding === "branch-b-only") && pooledPreService.length) {
    violations.push(...evaluateThresholdDistribution(pooledPreService, WEB06_THRESHOLDS.sustained.preServiceWaitUpperBoundMs, "pooled-pre-service").violations);
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
    pooledLongTaskCount: receipts.reduce((sum, receipt) => sum + (receipt.longTasks ?? []).filter((task) => task.overlapsInteractionWindow).length, 0),
    violations,
  };
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

/** Compact evidence deliberately carries digests and verdicts, never raw spans. */
export function publicEvidenceReceipt({ receipt, parsed, rawPacketSha256 }) {
  if (!isSha256(rawPacketSha256)) throw new Error("WEB06_RAW_PACKET_HASH_INVALID");
  const compact = {
    version: "web06-public-receipt-v1",
    sourceCommit: receipt.source.commit,
    artifactSha256: receipt.source.artifactSha256,
    metricContractVersion: receipt.metricContractVersion,
    scenarioRegistryVersion: receipt.scenarioRegistryVersion,
    scenarioId: receipt.scenarioId,
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
  };
  const privacy = validatePointerFreePrivacy(compact);
  if (!privacy.pass) throw new Error(`WEB06_PUBLIC_EVIDENCE_PRIVACY:${privacy.errors.join(",")}`);
  return compact;
}
