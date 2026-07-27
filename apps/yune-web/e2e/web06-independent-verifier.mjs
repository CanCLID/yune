#!/usr/bin/env node

/**
 * WEB-06 independent recomputation.
 *
 * Deliberately do not import the runner parser, validator, percentile helpers,
 * or summary builders.  This module consumes only frozen registry/version data
 * and implements its own clock calibration, timestamp arithmetic, attempt
 * retention, pooling, and byte comparison.
 */

import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { lstat, mkdir, open, readFile, realpath } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import {
  ACTION_REGISTRY,
  EVENT_ACTION_RULES,
  SCENARIO_REGISTRY,
  SCENARIO_RUN_REGISTRY,
  WEB06_BEHAVIOR_PREDICATES,
  WEB06_PRESSURE_PAIR_REGISTRY,
  WEB06_BEHAVIOR_PREDICATE_VERSION,
  WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION,
  WEB06_METRIC_CONTRACT_VERSION,
  WEB06_OBSERVER_COUNTERBALANCE,
  WEB06_SCENARIO_REGISTRY_VERSION,
  WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION,
  WEB06_THRESHOLDS,
} from "./web06-metric-contract.mjs";

const OUTCOMES = Object.freeze([
  "barrier-completed",
  "committed",
  "failure",
  "painted",
  "processed-no-visual-change",
  "superseded",
]);
const INDEPENDENT_OUTCOME_COUNT_KEYS = Object.freeze([...OUTCOMES, "unclassified"].sort());
const SHA64 = /^[0-9a-f]{64}$/;
const SHA40 = /^[0-9a-f]{40}$/;
const MEASURED = new Set(["PASS", "RED", "RED_BEHAVIOR"]);
const INDEPENDENT_TARGET_PROTOCOL_MODES = Object.freeze({
  PRODUCT: "off",
  BASE_MINIMAL: "minimal",
  BASE_FULL: "full",
  FINAL_MINIMAL: "minimal",
  FINAL_FULL: "full",
});
const execFileAsync = promisify(execFile);
const INDEPENDENT_RUNNER_TOOLING_PATHS = Object.freeze([
  "apps/yune-web/e2e/run-public-web06-gate.mjs",
  "apps/yune-web/e2e/run-web06-local-matrix.mjs",
  "apps/yune-web/e2e/playwright.web06.config.ts",
  "apps/yune-web/e2e/yune-web06-smoothness.spec.ts",
  "apps/yune-web/e2e/web06-collector.mjs",
  "apps/yune-web/e2e/web06-metric-contract.mjs",
  "apps/yune-web/e2e/web06-receipt-parser.mjs",
  "apps/yune-web/e2e/web06-independent-verifier.mjs",
  "apps/yune-web/e2e/web06-suite-attestation.mjs",
  "apps/yune-web/e2e/web06-sealed-artifact-server.mjs",
  "apps/yune-web/e2e/public-artifact-verifier.mjs",
  "scripts/evidence-output-path.py",
]);
const INDEPENDENT_BINDING_SCENARIO_IDS = Object.freeze([
  "existing-normal-guard", "rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping",
  "burst-luna", "correction", "selection-paging", "selection-paging-jyutping", "burst-action-map",
  "fifo-pressure-barriers", "learned-row", "fair-peer-short",
]);
const INDEPENDENT_BINDING_SCENARIO_RUNS = Object.freeze(INDEPENDENT_BINDING_SCENARIO_IDS.flatMap((scenarioId) =>
  Object.values(SCENARIO_RUN_REGISTRY).filter((run) => run.scenarioId === scenarioId).map((run) => run.runId)));
const INDEPENDENT_EXTENDED_SCENARIO_RUNS = Object.freeze(Object.values(SCENARIO_RUN_REGISTRY)
  .filter((run) => run.scenarioId === "extended-scheduler-barriers").map((run) => run.runId));
const SENTINEL_OVERFLOW_FIELDS = Object.freeze([
  "events", "auxiliaryEvents", "unmatchedEvents", "snapshots", "frameTimestamps", "longTasks", "focus",
  "callbacks", "windows", "idleControls", "driverDispatchBindings", "pendingCaptures",
]);
const INDEPENDENT_BEHAVIOR_FAILURE_CODES = new Set([
  "WEB06_ACTION_COMPLETION_COUNT",
  "WEB06_IMPORT_CONTINUATION_MARKER_TIMEOUT",
  "WEB06_LEARNED_PREPARE_CONTINUITY_INVALID",
  "WEB06_LEARNED_PROTOCOL_SEQUENCE_REUSE",
  "WEB06_LEARNED_RELOAD_ARRIVAL_INVALID",
  "WEB06_LEARNED_RELOAD_BIND_INVALID",
  "WEB06_PRESSURE_ACTION_MISSING",
  "WEB06_SAME_TASK_PAIR_REORDERED",
]);

function hasOwn(record, key) {
  return typeof key === "string" && Object.prototype.hasOwnProperty.call(record, key);
}

function independentRawAttemptOrdinalMatches(envelope) {
  const match = /^(?:triplet-)?attempt-([1-9][0-9]*)$/.exec(envelope?.attemptId ?? "");
  if (!match) return false;
  const ordinal = Number(match[1]);
  return Number.isSafeInteger(ordinal) && envelope.attemptNumber === ordinal;
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function exactJson(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function canonicalProjection(value) {
  if (Array.isArray(value)) return value.map(canonicalProjection);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.keys(value).filter((key) => value[key] !== undefined).sort()
    .map((key) => [key, canonicalProjection(value[key])]));
}

function equivalentProjection(left, right) {
  return JSON.stringify(canonicalProjection(left)) === JSON.stringify(canonicalProjection(right));
}

const INDEPENDENT_ROUND_DIAGNOSTIC_FIELDS = Object.freeze([
  "setupErrorCodes",
  "behaviorErrorCodes",
  "thresholdViolations",
]);
const INDEPENDENT_FIVE_ROUND_DIAGNOSTIC_FIELDS = Object.freeze([
  "summaryErrors",
  "poolViolations",
]);
const INDEPENDENT_TRUE_DIAGNOSTIC_SYNONYM_CODES = new Set([
  "BURST_RECOVERY_INVALID", "BURST_RECOVERY_IDENTITY",
  "BEHAVIOR_PAGE_VISIBLE_COUNT", "BEHAVIOR_VISIBLECOUNT",
  "BEHAVIOR_PAGE_PREVIOUS", "BEHAVIOR_PREVIOUSDISABLED",
  "BEHAVIOR_PAGE_NEXT", "BEHAVIOR_NEXTDISABLED",
  "BEHAVIOR_TEXTAREA_VALUE", "BEHAVIOR_TEXTAREAVALUE",
  "BEHAVIOR_SELECTION_START", "BEHAVIOR_SELECTIONSTART",
  "BEHAVIOR_SELECTION_END", "BEHAVIOR_SELECTIONEND",
]);

function independentCanonicalDiagnosticCode(value) {
  invariant(typeof value === "string" && value.length > 0,
    "WEB06_INDEPENDENT_SUMMARY_DIAGNOSTIC_CODE_INVALID");
  const [code, ...detail] = value.split(":");
  const suffix = detail.join(":");
  const withDetail = (family) => suffix.length === 0 ? family : `${family}:${suffix}`;

  if (["BURST_RECOVERY_INVALID", "BURST_RECOVERY_IDENTITY"].includes(code)) {
    return withDetail("BURST_RECOVERY_IDENTITY");
  }
  const aliases = {
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
  if (hasOwn(aliases, code)) return withDetail(aliases[code]);
  return value;
}

function independentCanonicalDiagnostics(values) {
  invariant(Array.isArray(values), "WEB06_INDEPENDENT_SUMMARY_DIAGNOSTIC_ARRAY_INVALID");
  const grouped = new Map();
  for (const raw of values) {
    const canonical = independentCanonicalDiagnosticCode(raw);
    const [code] = raw.split(":");
    const records = grouped.get(canonical) ?? [];
    records.push({ raw, code });
    grouped.set(canonical, records);
  }
  return [...grouped.entries()].sort(([left], [right]) => left.localeCompare(right))
    .flatMap(([canonical, records]) => {
      const codes = new Set(records.map((record) => record.code));
      const trueSynonymFamily = codes.size > 1
        && [...codes].every((code) => INDEPENDENT_TRUE_DIAGNOSTIC_SYNONYM_CODES.has(code));
      if (!trueSynonymFamily) return Array.from({ length: records.length }, () => canonical);
      const rawCounts = new Map();
      for (const record of records) rawCounts.set(record.raw, (rawCounts.get(record.raw) ?? 0) + 1);
      return Array.from({ length: Math.max(...rawCounts.values()) }, () => canonical);
    });
}

function independentSummaryDiagnosticFields(summary) {
  if (summary?.version === "web06-round-summary-v1") return INDEPENDENT_ROUND_DIAGNOSTIC_FIELDS;
  if (summary?.version === "web06-five-round-summary-v1") return INDEPENDENT_FIVE_ROUND_DIAGNOSTIC_FIELDS;
  invariant(false, "WEB06_INDEPENDENT_SUMMARY_VERSION_INVALID");
}

function independentDiagnosticBinding(summary) {
  const dimensions = Object.fromEntries(independentSummaryDiagnosticFields(summary).map((field) => {
    const raw = summary[field];
    const semantic = independentCanonicalDiagnostics(raw);
    return [field, {
      rawCount: raw.length,
      rawSha256: sha256(Buffer.from(JSON.stringify(raw), "utf8")),
      semanticCount: semantic.length,
      semanticSha256: sha256(Buffer.from(JSON.stringify(semantic), "utf8")),
    }];
  }));
  return canonicalProjection({
    version: WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION,
    dimensions,
    ...(summary.version === "web06-five-round-summary-v1" ? {
      roundBindingsSha256: sha256(Buffer.from(JSON.stringify(summary.roundSummaries.map((round) =>
        round.implementationDiagnosticBinding)), "utf8")),
    } : {}),
  });
}

function independentSemanticRoundProjection(summary) {
  const projected = structuredClone(summary);
  delete projected.implementationDiagnosticBinding;
  delete projected.semanticProjectionSha256;
  for (const field of INDEPENDENT_ROUND_DIAGNOSTIC_FIELDS) {
    projected[field] = independentCanonicalDiagnostics(projected[field]);
  }
  return canonicalProjection(projected);
}

function independentSummarySemanticProjection(summary) {
  let scope;
  let projected;
  if (summary?.version === "web06-round-summary-v1") {
    scope = "round";
    projected = independentSemanticRoundProjection(summary);
  } else if (summary?.version === "web06-five-round-summary-v1") {
    scope = "five-round";
    projected = structuredClone(summary);
    delete projected.implementationDiagnosticBinding;
    delete projected.semanticProjectionSha256;
    for (const field of INDEPENDENT_FIVE_ROUND_DIAGNOSTIC_FIELDS) {
      projected[field] = independentCanonicalDiagnostics(projected[field]);
    }
    projected.roundSummaries = projected.roundSummaries.map(independentSemanticRoundProjection);
    projected = canonicalProjection(projected);
  } else {
    invariant(false, "WEB06_INDEPENDENT_SUMMARY_VERSION_INVALID");
  }
  return canonicalProjection({
    version: WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION,
    scope,
    summary: projected,
  });
}

export function independentlyProjectSummarySemantics(summary) {
  return independentSummarySemanticProjection(summary);
}

function independentSemanticBytes(summary) {
  return Buffer.from(JSON.stringify(independentSummarySemanticProjection(summary)), "utf8");
}

function independentlyAssertSummaryIntegrity(summary, label) {
  invariant(summary?.semanticProjectionVersion === WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION,
    "WEB06_INDEPENDENT_SUMMARY_SEMANTIC_VERSION", label);
  invariant(exactJson(summary.implementationDiagnosticBinding, independentDiagnosticBinding(summary)),
    "WEB06_INDEPENDENT_SUMMARY_DIAGNOSTIC_BINDING", label);
  invariant(SHA64.test(summary.semanticProjectionSha256 ?? "")
    && summary.semanticProjectionSha256 === sha256(independentSemanticBytes(summary)),
  "WEB06_INDEPENDENT_SUMMARY_SEMANTIC_HASH", label);
}

function independentDecisionDimensions(summary) {
  const diagnostics = Object.fromEntries(independentSummaryDiagnosticFields(summary)
    .map((field) => [field, independentCanonicalDiagnostics(summary[field])]));
  return canonicalProjection({
    diagnostics,
    decision: summary.decision,
    status: summary.status,
    ...(summary.version === "web06-round-summary-v1" ? {
      retryEligible: summary.retryEligible,
    } : {}),
    validRedObserved: summary.validRedObserved,
  });
}

/**
 * Preserve both implementation-specific raw bindings while proving that only
 * the frozen semantic aliases differ. Duplicate raw occurrences survive the
 * projection unless they are distinct members of one explicit synonym family.
 */
export function independentlyBindSummaryDiagnostics(runnerSummary, independentSummary, label = "summary") {
  independentlyAssertSummaryIntegrity(runnerSummary, `${label}:runner`);
  independentlyAssertSummaryIntegrity(independentSummary, `${label}:independent`);
  const runnerDecisionDimensions = independentDecisionDimensions(runnerSummary);
  const independentDecisionProjection = independentDecisionDimensions(independentSummary);
  invariant(exactJson(runnerDecisionDimensions, independentDecisionProjection),
    "WEB06_INDEPENDENT_SUMMARY_DECISION_DIMENSIONS_MISMATCH", label);
  const runnerBytes = independentSemanticBytes(runnerSummary);
  const independentBytes = independentSemanticBytes(independentSummary);
  invariant(runnerBytes.equals(independentBytes), "WEB06_INDEPENDENT_SUMMARY_SEMANTIC_BYTE_MISMATCH", label);
  const pair = canonicalProjection({
    version: "web06-independent-diagnostic-binding-pair-v1",
    scope: runnerSummary.version === "web06-round-summary-v1" ? "round" : "five-round",
    runnerBinding: runnerSummary.implementationDiagnosticBinding,
    independentBinding: independentSummary.implementationDiagnosticBinding,
    runnerSemanticProjectionSha256: sha256(runnerBytes),
    independentSemanticProjectionSha256: sha256(independentBytes),
    runnerDecisionDimensionsSha256: sha256(Buffer.from(JSON.stringify(runnerDecisionDimensions), "utf8")),
    independentDecisionDimensionsSha256: sha256(Buffer.from(JSON.stringify(independentDecisionProjection), "utf8")),
    equivalent: true,
  });
  return canonicalProjection({
    ...pair,
    bindingPairSha256: sha256(Buffer.from(JSON.stringify(pair), "utf8")),
  });
}

function attachIndependentRoundIntegrity(summary, parsed) {
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
  summary.implementationDiagnosticBinding = independentDiagnosticBinding(summary);
  summary.semanticProjectionSha256 = sha256(independentSemanticBytes(summary));
  return summary;
}

function invariant(condition, code, detail = "") {
  if (!condition) throw new Error(`${code}${detail ? `:${detail}` : ""}`);
}

function finite(value) {
  return typeof value === "number" && Number.isFinite(value);
}

function independentClockCalibrationError(value) {
  return typeof value === "string"
    && (value.startsWith("SETUP_INVALID_CLOCK_CALIBRATION")
      || value === "SETUP_LEARNED_CALIBRATION_SEGMENTS_INVALID");
}

function independentDigestTokens(value, emit) {
  if (value === null) return emit("null;");
  if (value === undefined) return emit("undefined;");
  if (typeof value === "string") return emit(`s${value.length}:${value}`);
  if (typeof value === "number") return emit(`n${Object.is(value, -0) ? "-0" : String(value)};`);
  if (typeof value === "boolean") return emit(value ? "b1;" : "b0;");
  if (typeof value === "bigint") return emit(`i${String(value)};`);
  if (Array.isArray(value)) {
    emit(`a${value.length}:`);
    for (const item of value) independentDigestTokens(item, emit);
    return;
  }
  if (typeof value === "object") {
    const keys = Object.keys(value).sort();
    emit(`o${keys.length}:`);
    for (const key of keys) {
      independentDigestTokens(key, emit);
      independentDigestTokens(value[key], emit);
    }
    return;
  }
  throw new Error(`WEB06_INDEPENDENT_DIGEST_UNSUPPORTED:${typeof value}`);
}

/** Separate implementation used to reject mutated protocol fingerprints. */
export function independentWeb06StableDigest(value) {
  const hashes = [0x811c9dc5, 0x9e3779b9, 0x85ebca6b, 0xc2b2ae35];
  const multipliers = [0x01000193, 0x27d4eb2d, 0x165667b1, 0x85ebca77];
  independentDigestTokens(value, (token) => {
    for (let index = 0; index < token.length; index += 1) {
      for (let stream = 0; stream < hashes.length; stream += 1) {
        hashes[stream] = Math.imul(hashes[stream] ^ token.charCodeAt(index), multipliers[stream]);
      }
    }
  });
  return hashes.map((hash) => (hash >>> 0).toString(16).padStart(8, "0")).join("");
}

function nearestRank(sorted, p) {
  invariant(Array.isArray(sorted) && sorted.length > 0, "WEB06_INDEPENDENT_EMPTY_DISTRIBUTION");
  return sorted[Math.ceil((sorted.length - 1) * p)];
}

function distribution(values) {
  invariant(Array.isArray(values) && values.length > 0
    && values.every((value) => finite(value)), "WEB06_INDEPENDENT_DISTRIBUTION_INVALID");
  const sorted = [...values].sort((left, right) => left - right);
  return {
    count: sorted.length,
    median: nearestRank(sorted, 0.5),
    p95: nearestRank(sorted, 0.95),
    p99: nearestRank(sorted, 0.99),
    max: sorted.at(-1),
  };
}

/**
 * Verifier-owned threshold projection. Keep this implementation separate from
 * the runner helper so threshold labels, percentile coverage, and duplicate
 * per-sample violations are independently reproducible.
 */
export function independentlyEvaluateThresholdDistribution(values, ceilings, label) {
  const summary = distribution(values);
  const violations = [];
  for (const key of ["median", "p95", "p99", "max"]) {
    if (ceilings[key] !== undefined && summary[key] > ceilings[key]) {
      violations.push(`${label}:${key}:${summary[key]}>${ceilings[key]}`);
    }
  }
  return { pass: violations.length === 0, summary, violations };
}

function distributionOrNull(values) {
  return values.length ? distribution(values) : null;
}

function independentIntervalUnionOverlap(start, end, intervals) {
  const clipped = intervals.map((interval) => [Math.max(start, interval.startedAt), Math.min(end, interval.finishedAt)])
    .filter(([left, right]) => finite(left) && finite(right) && right > left)
    .sort((left, right) => left[0] - right[0]);
  let total = 0;
  let left;
  let right;
  for (const [nextLeft, nextRight] of clipped) {
    if (left === undefined || nextLeft > right) {
      if (left !== undefined) total += right - left;
      left = nextLeft;
      right = nextRight;
    } else right = Math.max(right, nextRight);
  }
  return total + (left === undefined ? 0 : right - left);
}

function independentObserverModeLedger(mode, modeName) {
  const semanticErrors = [];
  const intervals = mode?.callbackIntervals;
  if (!Array.isArray(intervals)) {
    return { errors: [`${modeName}:callback-intervals-shape`], intervals: [], attributionValid: false };
  }
  const ids = intervals.map((row) => row?.callbackId);
  if (!ids.every((id) => typeof id === "string" && id.length > 0)
    || new Set(ids).size !== ids.length) semanticErrors.push(`${modeName}:callback-interval-identity`);
  for (const [index, row] of intervals.entries()) {
    const timingValid = finite(row?.startedAt) && finite(row?.finishedAt)
      && row.startedAt >= 0 && row.finishedAt >= row.startedAt
      && finite(row.durationMs) && row.durationMs >= 0
      && row.durationMs === row.finishedAt - row.startedAt;
    const sentinelSource = row?.sourceClass === "common-sentinel";
    const sourceValid = sentinelSource
      ? typeof row?.kind === "string" && row.kind.length > 0
        && Number.isSafeInteger(row.windowIndex) && row.windowIndex >= -1
        && row.windowIndex < mode.interactionWindowCount
        && (row.eventSequenceId === undefined
          || (Number.isSafeInteger(row.eventSequenceId)
            && row.eventSequenceId > 0 && row.eventSequenceId <= mode.commonEventCount))
      : row?.sourceClass === (modeName === "minimal" ? "minimal-probe"
        : modeName === "full" ? "full-collector" : undefined)
        && typeof row?.operation === "string" && row.operation.length > 0;
    if (!Number.isSafeInteger(row?.sequenceId) || row.sequenceId <= 0 || !timingValid || !sourceValid) {
      semanticErrors.push(`${modeName}:callback-interval:${index + 1}`);
    }
  }
  const sentinel = intervals.filter((row) => row.sourceClass === "common-sentinel");
  const collector = intervals.filter((row) => row.sourceClass !== "common-sentinel");
  const orderedUnique = (rows) => new Set(rows.map((row) => row.sequenceId)).size === rows.length
    && rows.every((row, index) => index === 0
      || (row.sequenceId > rows[index - 1].sequenceId
        && row.finishedAt >= rows[index - 1].finishedAt));
  if (!orderedUnique(sentinel) || !orderedUnique(collector)) {
    semanticErrors.push(`${modeName}:callback-sequence-order`);
  }
  if (!Number.isSafeInteger(mode?.callbackLedgerCount)
    || mode.callbackLedgerCount !== sentinel.length
    || !Number.isSafeInteger(mode?.callbackLedgerCapacity)
    || mode.callbackLedgerCount > mode.callbackLedgerCapacity
    || mode.sentinelAccountedCallbackCount !== sentinel.length
    || mode.callbackLedgerOverflowCount !== 0
    || (!Number.isSafeInteger(mode?.commonEventCount) || mode.commonEventCount < 0)
    || (mode.commonEventCount > 0 && sentinel.length === 0)) {
    semanticErrors.push(`${modeName}:sentinel-callback-conservation`);
  }
  if (!exactJson(mode?.sentinelCallbacksMs ?? [], sentinel.map((row) => row.durationMs))) {
    semanticErrors.push(`${modeName}:sentinel-callback-duration-ledger`);
  }
  const recordedDurations = mode?.mainObserverCallbacksMs ?? [];
  const intervalDurations = collector.map((row) => row.durationMs);
  const numericArrays = [
    mode?.sentinelCallbacksMs,
    mode?.sentinelTotalPerEventMs,
    mode?.sentinelTotalPerWindowMs,
    mode?.collectorCallbacksMs,
    mode?.mainObserverCallbacksMs,
    mode?.workerCollectorCallbacksMs,
  ];
  if (numericArrays.some((values) => !Array.isArray(values)
    || values.some((value) => !finite(value) || value < 0))) {
    semanticErrors.push(`${modeName}:callback-duration-shape`);
  }
  if (!exactJson(recordedDurations, intervalDurations)) {
    semanticErrors.push(`${modeName}:main-callback-duration-ledger`);
  }
  const allCollectorDurations = [...recordedDurations, ...(mode?.workerCollectorCallbacksMs ?? [])];
  if (!exactJson([...allCollectorDurations].sort((a, b) => a - b),
    [...(mode?.collectorCallbacksMs ?? [])].sort((a, b) => a - b))) {
    semanticErrors.push(`${modeName}:collector-callback-duration-projection`);
  }
  if (modeName === "product") {
    if (collector.length || recordedDurations.length || (mode?.workerCollectorCallbacksMs ?? []).length) {
      semanticErrors.push("product:private-callback-present");
    }
  } else {
    if (!Number.isSafeInteger(mode?.mainObserverCallbackCount)
      || !Number.isSafeInteger(mode?.mainObserverCallbackCapacity)
      || mode.mainObserverCallbackCount !== collector.length
      || mode.mainObserverCallbackCount > mode.mainObserverCallbackCapacity
      || mode.mainObserverCallbackOverflowCount !== 0) {
      semanticErrors.push(`${modeName}:main-callback-conservation`);
    }
  }
  if (!Number.isSafeInteger(mode?.commonEventCount) || mode.commonEventCount < 0
    || !Array.isArray(mode?.sentinelTotalPerEventMs)
    || mode.sentinelTotalPerEventMs.length !== mode.commonEventCount
    || !Number.isSafeInteger(mode?.interactionWindowCount) || mode.interactionWindowCount < 0
    || !Array.isArray(mode?.sentinelTotalPerWindowMs)
    || mode.sentinelTotalPerWindowMs.length !== mode.interactionWindowCount) {
    semanticErrors.push(`${modeName}:sentinel-cardinality`);
  }
  const eventTotals = Number.isSafeInteger(mode?.commonEventCount) && mode.commonEventCount >= 0
    ? Array.from({ length: mode.commonEventCount }, (_, index) => sentinel
      .filter((row) => row.eventSequenceId === index + 1)
      .reduce((sum, row) => sum + row.durationMs, 0))
    : [];
  const windowTotals = Number.isSafeInteger(mode?.interactionWindowCount) && mode.interactionWindowCount >= 0
    ? Array.from({ length: mode.interactionWindowCount }, (_, index) => sentinel
      .filter((row) => row.windowIndex === index)
      .reduce((sum, row) => sum + row.durationMs, 0))
    : [];
  if (!exactJson(mode?.sentinelTotalPerEventMs, eventTotals)
    || !exactJson(mode?.sentinelTotalPerWindowMs, windowTotals)) {
    semanticErrors.push(`${modeName}:sentinel-total-projection`);
  }
  if (!Array.isArray(mode?.rawLongTasks)
    || mode.rawLongTasks.some((task) => !finite(task?.startTime) || task.startTime < 0
      || !finite(task?.durationMs) || task.durationMs < 0
      || task.overlapsInteractionWindow !== true)) {
    semanticErrors.push(`${modeName}:long-task-ledger`);
  }
  const attributionValid = semanticErrors.length === 0;
  const errors = [...semanticErrors];
  if (mode?.callbackAttributionComplete !== attributionValid) {
    errors.push(`${modeName}:callback-attribution-declaration`);
  }
  return {
    errors,
    intervals,
    attributionValid,
  };
}

function independentObserverModeLocalHardRed(mode) {
  if (mode?.hardRedBindingValid !== true) return false;
  const atOrAbove = (values, threshold) => Array.isArray(values)
    && values.some((value) => finite(value) && value >= threshold);
  return atOrAbove(mode.sentinelCallbacksMs, WEB06_THRESHOLDS.observer.sentinelCallbackExclusiveMaxMs)
    || atOrAbove(
      mode.sentinelTotalPerEventMs,
      WEB06_THRESHOLDS.observer.sentinelTotalPerEventExclusiveMaxMs,
    )
    || atOrAbove(mode.collectorCallbacksMs, WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(mode.mainObserverCallbacksMs, WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(mode.workerCollectorCallbacksMs, WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(
      mode.instrumentationAddedLongTasksMs,
      WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs,
    );
}

function independentObserverModeHardRedExpected(mode) {
  const parserRed = [mode?.commonVerdict, mode?.internalVerdict]
    .some((verdict) => ["RED", "RED_BEHAVIOR"].includes(verdict));
  return mode?.behaviorRedObserved === true
    || (mode?.hardRedBindingValid === true
      && (parserRed || independentObserverModeLocalHardRed(mode)));
}

/** Separate Long Task/callback attribution implementation used by the verifier. */
export function independentClassifyObserverLongTasks(inputModes) {
  const modes = structuredClone(inputModes);
  const errors = [];
  const threshold = WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs;
  const tasks = {};
  for (const modeName of ["product", "minimal", "full"]) {
    const mode = modes[modeName];
    const ledger = independentObserverModeLedger(mode, modeName);
    errors.push(...ledger.errors);
    const occurrences = new Map();
    tasks[modeName] = (mode?.rawLongTasks ?? [])
      .filter((task) => task?.overlapsInteractionWindow === true)
      .map((task, index) => {
      const locus = typeof task?.locus === "string" && task.locus ? task.locus : `invalid:${index}`;
      const occurrence = (occurrences.get(locus) ?? 0) + 1;
      occurrences.set(locus, occurrence);
      if (!finite(task?.startTime) || task.startTime < 0
        || !finite(task?.durationMs) || task.durationMs < 0) {
        errors.push(`${modeName}:long-task:${index + 1}`);
      }
      return { ...task, locus, locusKey: `${locus}#${occurrence}` };
    }).filter((task) => task.durationMs >= threshold);
    mode._verifiedIntervals = ledger.intervals;
    mode._verifiedAttributionValid = ledger.attributionValid;
  }
  const keys = Object.fromEntries(Object.entries(tasks)
    .map(([name, rows]) => [name, new Set(rows.map((row) => row.locusKey))]));
  for (const modeName of ["product", "minimal", "full"]) {
    const mode = modes[modeName];
    const underlying = [];
    const added = [];
    for (const task of tasks[modeName]) {
      const matched = keys.product.has(task.locusKey) && keys.minimal.has(task.locusKey) && keys.full.has(task.locusKey);
      const collectorIntervals = modeName === "product" ? []
        : mode._verifiedIntervals.filter((row) => row.sourceClass !== "common-sentinel");
      const overlap = mode._verifiedAttributionValid
        ? independentIntervalUnionOverlap(task.startTime, task.startTime + task.durationMs, collectorIntervals)
        : task.durationMs;
      const residualLowerBound = task.durationMs - overlap;
      if (modeName === "product"
        || (mode._verifiedAttributionValid && residualLowerBound >= threshold && matched)) {
        underlying.push(task.durationMs);
      }
      else if (modeName !== "product") added.push(task.durationMs);
    }
    if (!exactJson(underlying, mode.underlyingLongTasksMs ?? [])) errors.push(`${modeName}:underlying-attribution-mismatch`);
    if (!exactJson(added, mode.instrumentationAddedLongTasksMs ?? [])) errors.push(`${modeName}:added-attribution-mismatch`);
    delete mode._verifiedIntervals;
    delete mode._verifiedAttributionValid;
  }
  return { pass: errors.length === 0, errors, modes };
}

export function independentlyEvaluateObserverTriplets(attempts) {
  invariant(Array.isArray(attempts) && attempts.length <= WEB06_THRESHOLDS.observer.maximumTripletAttempts,
    "WEB06_INDEPENDENT_OBSERVER_ATTEMPT_CAP");
  const verified = [];
  const violations = [];
  const modeNames = ["product", "minimal", "full"];
  for (const attempt of attempts) {
    const complete = modeNames.every((name) => attempt[name]?.measurementCompleted === true);
    const attribution = complete ? independentClassifyObserverLongTasks({
      product: attempt.product,
      minimal: attempt.minimal,
      full: attempt.full,
    }) : { pass: true, errors: [] };
    const attributionMismatches = attribution.errors.filter((error) =>
      error.endsWith("attribution-mismatch"));
    if (attributionMismatches.length) {
      violations.push(...attributionMismatches.map((error) => `${attempt.attemptId}:${error}`));
    }
    const independentlyValid = complete && attributionMismatches.length === 0
      && modeNames.every((name) => attempt[name]?.measurementValid === true);
    if (attempt.valid !== independentlyValid) violations.push(`${attempt.attemptId}:validity-mismatch`);
    for (const modeName of modeNames) {
      if (attempt[modeName]
        && attempt[modeName].hardRedObserved !== independentObserverModeHardRedExpected(attempt[modeName])) {
        violations.push(`${attempt.attemptId}:${modeName}-hard-red-observation-mismatch`);
      }
    }
    verified.push({ ...attempt, valid: independentlyValid });
  }
  const valid = verified.filter((attempt) => attempt.valid);
  if (valid.length > WEB06_THRESHOLDS.observer.requiredTriplets) {
    throw new Error("WEB06_OBSERVER_VALID_TRIPLET_RETRY_FORBIDDEN");
  }
  if (valid.length < WEB06_THRESHOLDS.observer.requiredTriplets) {
    const preservedUnpairedReds = verified.flatMap((attempt) =>
      modeNames.filter((name) => independentObserverModeHardRedExpected(attempt[name]))
        .map((name) => `${attempt.attemptId}:${name}`));
    violations.push(...preservedUnpairedReds.map((identity) => `${identity}-unpaired-valid-red`));
    violations.sort();
    return {
      pass: false,
      status: attempts.length === WEB06_THRESHOLDS.observer.maximumTripletAttempts
        ? "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS" : "INCOMPLETE",
      violations,
      preservedUnpairedReds,
    };
  }
  const completeUnpairedReds = verified.filter((attempt) => !attempt.valid).flatMap((attempt) =>
    modeNames.filter((name) => independentObserverModeHardRedExpected(attempt[name]))
      .map((name) => `${attempt.attemptId}:${name}`));
  violations.push(...completeUnpairedReds.map((identity) => `${identity}-unpaired-valid-red`));
  const durationFields = [
    ["sentinel-callback", "sentinelCallbacksMs"],
    ["sentinel-total", "sentinelTotalPerEventMs"],
    ["collector-callback", "collectorCallbacksMs"],
    ["underlying-long-task", "underlyingLongTasksMs"],
    ["instrumentation-added-long-task", "instrumentationAddedLongTasksMs"],
  ];
  for (const attempt of valid) {
    for (const modeName of modeNames) {
      const mode = attempt[modeName];
      if (!mode || typeof mode !== "object") {
        violations.push(`${attempt.attemptId}:${modeName}-mode-shape`);
        continue;
      }
      if (mode.measurementValid !== true) violations.push(`${attempt.attemptId}:${modeName}-measurement-invalid`);
      if (!Array.isArray(mode.samples) || mode.samples.length === 0
        || mode.samples.some((value) => !finite(value) || value < 0)) {
        violations.push(`${attempt.attemptId}:${modeName}-samples-shape`);
      }
      for (const [label, field] of durationFields) {
        const values = mode[field];
        if (!Array.isArray(values) || (field === "sentinelCallbacksMs" && values.length === 0)
          || values.some((value) => !finite(value) || value < 0)) {
          violations.push(`${attempt.attemptId}:${modeName}-${label}-shape`);
        }
      }
      if (!Number.isSafeInteger(mode.commonEventCount) || mode.commonEventCount <= 0
        || !Array.isArray(mode.sentinelTotalPerEventMs)
        || mode.sentinelTotalPerEventMs.length !== mode.commonEventCount) {
        violations.push(`${attempt.attemptId}:${modeName}-sentinel-total-count`);
      }
      if (!Number.isSafeInteger(mode.interactionWindowCount) || mode.interactionWindowCount <= 0
        || !Array.isArray(mode.sentinelTotalPerWindowMs)
        || mode.sentinelTotalPerWindowMs.length !== mode.interactionWindowCount
        || mode.sentinelTotalPerWindowMs.some((value) => !finite(value) || value < 0)) {
        violations.push(`${attempt.attemptId}:${modeName}-sentinel-window-total-count`);
      }
      if (modeName === "product" && mode.instrumentationAddedLongTasksMs?.length !== 0) {
        violations.push(`${attempt.attemptId}:product-instrumentation-added-long-task-attribution`);
      }
      if (mode.callbackLedgerOverflowCount !== 0
        || !Number.isSafeInteger(mode.callbackLedgerCount)
        || mode.callbackLedgerCount !== mode.sentinelAccountedCallbackCount) {
        violations.push(`${attempt.attemptId}:${modeName}-sentinel-callback-conservation`);
      }
    }
  }
  const pooled = (modeName) => valid.flatMap((attempt) =>
    Array.isArray(attempt[modeName]?.samples) ? attempt[modeName].samples : []);
  const compare = (leftMode, rightMode) => {
    const left = distribution(pooled(leftMode));
    const right = distribution(pooled(rightMode));
    return {
      pair: `${leftMode}-vs-${rightMode}`,
      medianDelta: Math.abs(left.median - right.median),
      p95Delta: Math.abs(left.p95 - right.p95),
      maxDelta: Math.abs(left.max - right.max),
    };
  };
  const distributionsValid = modeNames.every((modeName) => pooled(modeName).length > 0
    && pooled(modeName).every((value) => finite(value) && value >= 0));
  const comparisons = distributionsValid
    ? [compare("product", "minimal"), compare("minimal", "full")]
    : [];
  for (const comparison of comparisons) {
    if (comparison.medianDelta > WEB06_THRESHOLDS.observer.absolutePooledMedianDeltaMs) {
      violations.push(`${comparison.pair}:median`);
    }
    if (comparison.p95Delta > WEB06_THRESHOLDS.observer.absolutePooledP95DeltaMs) {
      violations.push(`${comparison.pair}:p95`);
    }
    if (comparison.maxDelta > WEB06_THRESHOLDS.observer.absolutePooledMaxDeltaMs) {
      violations.push(`${comparison.pair}:max`);
    }
  }
  const slots = valid.map((attempt) => attempt.counterbalanceSlot);
  if (new Set(slots).size !== 5
    || slots.some((slot) => !hasOwn(WEB06_OBSERVER_COUNTERBALANCE, String(slot)))) {
    violations.push("counterbalance-slot-set");
  }
  const contexts = valid.map((attempt) => attempt.freshContextId);
  if (contexts.some((context) => typeof context !== "string" || !context)
    || new Set(contexts).size !== contexts.length) {
    violations.push("fresh-context-identity");
  }
  const allModeContexts = valid.flatMap((attempt) => attempt.modeContextIds ?? []);
  if (allModeContexts.length !== valid.length * 3
    || allModeContexts.some((context) => typeof context !== "string" || !context)
    || new Set(allModeContexts).size !== allModeContexts.length) {
    violations.push("fresh-mode-context-identities");
  }
  for (const attempt of valid) {
    if (!hasOwn(WEB06_OBSERVER_COUNTERBALANCE, String(attempt.counterbalanceSlot))
      || !exactJson(attempt.modeOrder, WEB06_OBSERVER_COUNTERBALANCE[attempt.counterbalanceSlot])) {
      violations.push(`${attempt.attemptId}:counterbalance-order`);
    }
    if (attempt.modeFixedBeforePageLoad !== true) violations.push(`${attempt.attemptId}:mode-not-fixed-before-load`);
    if (!Array.isArray(attempt.modeContextIds) || attempt.modeContextIds.length !== 3
      || new Set(attempt.modeContextIds).size !== 3) {
      violations.push(`${attempt.attemptId}:mode-context-identities`);
    }
    if (!(Array.isArray(attempt.product?.samples) && Array.isArray(attempt.minimal?.samples)
      && Array.isArray(attempt.full?.samples)
      && attempt.product.samples.length === attempt.minimal.samples.length
      && attempt.minimal.samples.length === attempt.full.samples.length)) {
      violations.push(`${attempt.attemptId}:common-sample-count`);
    }
    const commonDigests = modeNames.map((modeName) => attempt[modeName]?.commonEquivalenceDigest);
    if (!commonDigests.every((value) => SHA64.test(value ?? "")) || new Set(commonDigests).size !== 1) {
      violations.push(`${attempt.attemptId}:common-equivalence-digest`);
    }
    const internalDigests = [attempt.minimal?.internalEquivalenceDigest, attempt.full?.internalEquivalenceDigest];
    if (!internalDigests.every((value) => SHA64.test(value ?? "")) || internalDigests[0] !== internalDigests[1]) {
      violations.push(`${attempt.attemptId}:internal-equivalence-digest`);
    }
    const commonVerdicts = modeNames.map((modeName) => attempt[modeName]?.commonVerdict);
    if (!commonVerdicts.every((verdict) => ["PASS", "RED", "RED_BEHAVIOR"].includes(verdict))
      || new Set(commonVerdicts).size !== 1) {
      violations.push(`${attempt.attemptId}:common-verdict-disagreement`);
    }
    const internalVerdicts = [attempt.minimal?.internalVerdict, attempt.full?.internalVerdict];
    if (!internalVerdicts.every((verdict) => ["PASS", "RED", "RED_BEHAVIOR"].includes(verdict))
      || internalVerdicts[0] !== internalVerdicts[1]) {
      violations.push(`${attempt.attemptId}:internal-verdict-disagreement`);
    }
    const environments = modeNames.map((modeName) => attempt[modeName]?.environmentManifestSha256);
    if (!environments.every((value) => SHA64.test(value ?? "")) || new Set(environments).size !== 1) {
      violations.push(`${attempt.attemptId}:environment-drift`);
    }
    const environmentIds = modeNames.map((modeName) => attempt[modeName]?.environmentId);
    if (!environmentIds.every((value) => SHA64.test(value ?? "")) || new Set(environmentIds).size !== 1) {
      violations.push(`${attempt.attemptId}:environment-id-drift`);
    }
    for (const modeName of modeNames) {
      const mode = attempt[modeName];
      if (!mode || typeof mode !== "object") continue;
      if (Array.isArray(mode.sentinelCallbacksMs)
        && mode.sentinelCallbacksMs.some((value) =>
          value >= WEB06_THRESHOLDS.observer.sentinelCallbackExclusiveMaxMs)) {
        violations.push(`${attempt.attemptId}:${modeName}-sentinel-callback`);
      }
      if (Array.isArray(mode.sentinelTotalPerEventMs)
        && mode.sentinelTotalPerEventMs.some((value) =>
          value >= WEB06_THRESHOLDS.observer.sentinelTotalPerEventExclusiveMaxMs)) {
        violations.push(`${attempt.attemptId}:${modeName}-sentinel-total`);
      }
      if (Array.isArray(mode.collectorCallbacksMs)
        && mode.collectorCallbacksMs.some((value) =>
          value >= WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)) {
        violations.push(`${attempt.attemptId}:${modeName}-collector-callback`);
      }
      if (Array.isArray(mode.instrumentationAddedLongTasksMs)
        && mode.instrumentationAddedLongTasksMs.some((value) =>
          value >= WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs)) {
        violations.push(`${attempt.attemptId}:${modeName}-instrumentation-added-long-task`);
      }
    }
  }
  violations.sort();
  return { pass: violations.length === 0, status: violations.length ? "RED" : "PASS", comparisons, violations };
}

function independentlyClassifyHarnessFailure(code) {
  return INDEPENDENT_BEHAVIOR_FAILURE_CODES.has(code) ? "behavior" : "setup";
}

function independentRunnerSourceObservationErrors(observation, label) {
  const errors = [];
  const expectedKeys = ["version", "sourceCommit", "sourceTree", "sourceTreeState",
    "toolingManifestSha256", "files", "observationSha256"];
  if (!observation || typeof observation !== "object" || Array.isArray(observation)
    || !exactJson(Object.keys(observation).sort(), expectedKeys.sort())) {
    return [`${label}:shape`];
  }
  if (observation.version !== "web06-runner-source-observation-v1"
    || !SHA40.test(observation.sourceCommit ?? "") || !SHA40.test(observation.sourceTree ?? "")
    || observation.sourceTreeState !== "clean" || !SHA64.test(observation.toolingManifestSha256 ?? "")
    || !Array.isArray(observation.files) || observation.files.length === 0) {
    errors.push(`${label}:identity`);
  }
  const paths = new Set();
  for (const [index, file] of (observation.files ?? []).entries()) {
    if (!file || !exactJson(Object.keys(file).sort(), ["path", "sha256"])
      || typeof file.path !== "string" || !file.path || path.isAbsolute(file.path) || file.path.includes("\\")
      || file.path.split("/").some((segment) => !segment || segment === "." || segment === "..")
      || !SHA64.test(file.sha256 ?? "") || paths.has(file.path)) {
      errors.push(`${label}:file:${index + 1}`);
    }
    paths.add(file?.path);
  }
  const snapshot = {
    version: observation.version,
    sourceCommit: observation.sourceCommit,
    sourceTree: observation.sourceTree,
    sourceTreeState: observation.sourceTreeState,
    toolingManifestSha256: observation.toolingManifestSha256,
    files: observation.files,
  };
  if (observation.observationSha256 !== sha256(Buffer.from(JSON.stringify(snapshot), "utf8"))) {
    errors.push(`${label}:hash`);
  }
  return errors;
}

function independentObservedEnvironmentErrors(observation) {
  const errors = [];
  if (!observation || typeof observation !== "object" || Array.isArray(observation)
    || !exactJson(Object.keys(observation).sort(), ["host", "observationSha256", "toolchain", "version"])) {
    return ["raw-observed-environment:shape"];
  }
  const toolchainKeys = ["rust", "emscripten", "node", "npm", "playwright", "chromium",
    "chromiumExecutableSha256"];
  const hostKeys = ["os", "osVersion", "osBuildVersion", "arch", "cpuModel", "logicalCores",
    "memoryBytes", "powerState", "lowPowerMode"];
  if (observation.version !== "web06-observed-environment-v1"
    || !observation.toolchain || !exactJson(Object.keys(observation.toolchain).sort(), [...toolchainKeys].sort())
    || !observation.host || !exactJson(Object.keys(observation.host).sort(), [...hostKeys].sort())) {
    errors.push("raw-observed-environment:identity");
  } else {
    if (toolchainKeys.filter((key) => key !== "chromiumExecutableSha256")
      .some((key) => typeof observation.toolchain[key] !== "string" || !observation.toolchain[key])
      || !SHA64.test(observation.toolchain.chromiumExecutableSha256 ?? "")) {
      errors.push("raw-observed-environment:toolchain");
    }
    if (["os", "osVersion", "osBuildVersion", "arch", "cpuModel"]
      .some((key) => typeof observation.host[key] !== "string" || !observation.host[key])
      || observation.host.os !== "macOS" || observation.host.powerState !== "AC"
      || observation.host.lowPowerMode !== false
      || !Number.isSafeInteger(observation.host.logicalCores) || observation.host.logicalCores <= 0
      || !Number.isSafeInteger(observation.host.memoryBytes) || observation.host.memoryBytes <= 0) {
      errors.push("raw-observed-environment:host");
    }
  }
  const snapshot = {
    version: observation.version,
    toolchain: observation.toolchain,
    host: observation.host,
  };
  if (observation.observationSha256 !== sha256(Buffer.from(JSON.stringify(snapshot), "utf8"))) {
    errors.push("raw-observed-environment:hash");
  }
  return errors;
}

function independentRawObservationErrors(envelope, requireAttemptAfter) {
  const errors = [
    ...independentRunnerSourceObservationErrors(envelope.runnerSourceBefore, "raw-runner-source-before"),
    ...independentObservedEnvironmentErrors(envelope.observedEnvironment),
  ];
  if (envelope.attemptSourceBefore !== undefined) {
    errors.push(...independentRunnerSourceObservationErrors(
      envelope.attemptSourceBefore, "raw-attempt-source-before",
    ));
  } else if (requireAttemptAfter) errors.push("raw-attempt-source-before:missing");
  const sourceDriftFailure = envelope.partialAttempt?.failure?.code
    === "WEB06_RUNNER_SOURCE_CHANGED_DURING_ATTEMPT";
  if (envelope.attemptSourceAfter !== undefined) {
    errors.push(...independentRunnerSourceObservationErrors(
      envelope.attemptSourceAfter, "raw-attempt-source-after",
    ));
  } else if (requireAttemptAfter || sourceDriftFailure) errors.push("raw-attempt-source-after:missing");
  const sourceHashes = [envelope.runnerSourceBefore?.observationSha256,
    envelope.attemptSourceBefore?.observationSha256, envelope.attemptSourceAfter?.observationSha256]
    .filter((value) => value !== undefined);
  if (sourceDriftFailure) {
    if (envelope.attemptSourceBefore === undefined) errors.push("raw-attempt-source-before:missing");
    if (envelope.attemptSourceBefore?.observationSha256
      === envelope.attemptSourceAfter?.observationSha256) {
      errors.push("raw-runner-source-drift-not-observed");
    }
    if (envelope.runnerSourceBefore?.observationSha256
      !== envelope.attemptSourceBefore?.observationSha256) {
      errors.push("raw-runner-source-before-attempt-mismatch");
    }
  } else if (new Set(sourceHashes).size !== 1) {
    errors.push("raw-runner-source-observation-drift");
  }
  return errors;
}

function independentPartialReceiptBindingErrors(envelope) {
  const errors = [];
  const target = envelope?.target ?? {};
  const attemptNumberSafe = Number.isSafeInteger(envelope?.attemptNumber)
    && envelope.attemptNumber >= 1;
  const attemptNumberValid = attemptNumberSafe && independentRawAttemptOrdinalMatches(envelope);
  if (!attemptNumberSafe) errors.push("raw-partial-attempt-number");
  else if (!attemptNumberValid) errors.push("raw-partial-attempt-ordinal");
  if (target.protocolMode === "off"
    && (envelope.privateReceipt !== undefined || envelope.protocolExport !== undefined)) {
    errors.push("raw-partial-product-private-protocol-present");
  }
  const expectedSource = {
    commit: target.sourceCommit,
    tree: target.sourceTree,
    treeState: target.treeState,
    archiveSha256: target.archiveSha256,
    buildInfoSha256: target.buildInfoSha256,
    artifactSha256: target.artifactSha256,
    artifactResponseGuardSha256: target.artifactResponseGuardSha256,
    artifactResponseGuardSummarySha256: envelope.artifactResponseGuard?.summarySha256,
    identityManifestSha256: envelope.identityManifestSha256,
    runnerSourceManifestSha256: envelope.runnerSourceManifestSha256,
    runnerToolingManifestSha256: envelope.attemptSourceBefore?.toolingManifestSha256,
    runnerSourceObservationSha256: envelope.attemptSourceBefore?.observationSha256,
    runnerSourcePostObservationSha256: envelope.attemptSourceAfter?.observationSha256,
    observedEnvironmentSha256: envelope.observedEnvironment?.observationSha256,
    collectorContractSha256: target.collectorContractSha256,
    scenarioIdsSha256: envelope.scenarioIdsSha256,
    selectedBranch: target.pinnedSelectedBranch,
    disposition: target.pinnedDisposition,
    environmentManifestSha256: envelope.environmentManifestSha256,
    environmentId: envelope.environmentId,
  };
  const roundIds = [];
  for (const [surface, receipt] of [["common", envelope.commonReceipt], ["private", envelope.privateReceipt]]) {
    if (receipt === undefined) continue;
    if (!receipt || typeof receipt !== "object" || Array.isArray(receipt)) {
      errors.push(`raw-partial-${surface}-receipt-shape`);
      continue;
    }
    for (const [field, expected] of Object.entries({
      scenarioRunId: envelope.scenarioRunId,
      scenarioId: envelope.scenarioId,
      schemaId: envelope.schemaId,
      attemptId: envelope.attemptId,
      mode: target.id,
      measurementStarted: true,
      measurementCompleted: true,
    })) {
      if (receipt[field] !== expected) errors.push(`raw-partial-${surface}-receipt-metadata:${field}`);
    }
    if (typeof receipt.roundId !== "string" || receipt.roundId.length === 0) {
      errors.push(`raw-partial-${surface}-receipt-metadata:roundId`);
    } else {
      roundIds.push(receipt.roundId);
      if (!attemptNumberValid
        || receipt.roundId !== `${envelope.scenarioId}-round-${envelope.attemptNumber}`) {
        errors.push(`raw-partial-${surface}-receipt-metadata:roundId`);
      }
    }
    const source = receipt.source;
    if (!source || typeof source !== "object" || Array.isArray(source)) {
      errors.push(`raw-partial-${surface}-source-metadata`);
      continue;
    }
    for (const [field, expected] of Object.entries(expectedSource)) {
      if (source[field] !== expected) errors.push(`raw-partial-${surface}-source-metadata:${field}`);
    }
  }
  if (new Set(roundIds).size > 1) errors.push("raw-partial-receipt-round-mismatch");
  const common = envelope.commonReceipt;
  const internal = envelope.privateReceipt;
  const evidence = envelope.measurementEvidence;
  const sentinel = envelope.sentinel;
  if (common !== undefined && (!sentinel || typeof sentinel !== "object" || Array.isArray(sentinel)
    || !evidence || typeof evidence !== "object" || Array.isArray(evidence)
    || !equivalentProjection(common.eventClockProbe, evidence.eventClockProbe)
    || !equivalentProjection(common.eventClockSegments, evidence.eventClockSegments)
    || !equivalentProjection(common.calibration, { driver: evidence.calibration?.driver })
    || !equivalentProjection(common.calibrationSegments, evidence.calibrationSegments === undefined ? undefined : {
      preReload: { driver: evidence.calibrationSegments.preReload?.driver },
      postReload: { driver: evidence.calibrationSegments.postReload?.driver },
    })
    || !equivalentProjection(common.idleFrameIntervalsMs, evidence.idleFrameIntervalsMs)
    || !equivalentProjection(common.idleFrameSegments, evidence.idleFrameSegments)
    || !equivalentProjection(common.cadenceGaps, envelope.drive?.cadenceGaps)
    || !equivalentProjection(common.lifecycleContinuity, envelope.drive?.learned?.lifecycleContinuity))) {
    errors.push("raw-partial-common-projection");
  }
  if (common !== undefined && sentinel && typeof sentinel === "object" && !Array.isArray(sentinel)) {
    for (const field of ["events", "auxiliaryEvents", "unmatchedEvents", "interactionWindows",
      "idleControlWindows", "interactionFrameWindows", "interactionFrameTimestamps",
      "interactionFrameIntervalsMs", "longTasks", "focusVisibilitySamples",
      "assetsRequestedDuringWindow", "sentinelOverflowCounts"]) {
      if (!equivalentProjection(common[field], sentinel[field])) {
        errors.push(`raw-partial-common-sentinel-projection:${field}`);
      }
    }
    try {
      const expectedSamples = independentlyResolveCommonSamples({
        scenarioId: envelope.scenarioId,
        events: sentinel.events,
        snapshots: sentinel.snapshots,
      });
      if (!equivalentProjection(common.commonSamples, expectedSamples)) {
        errors.push("raw-partial-common-snapshot-projection");
      }
    } catch {
      errors.push("raw-partial-common-snapshot-projection");
    }
  }
  if (internal !== undefined && (!evidence || typeof evidence !== "object" || Array.isArray(evidence)
    || !equivalentProjection(internal.eventClockProbe, evidence.eventClockProbe)
    || !equivalentProjection(internal.eventClockSegments, evidence.eventClockSegments)
    || !equivalentProjection(internal.calibration, evidence.calibration)
    || !equivalentProjection(internal.calibrationSegments, evidence.calibrationSegments)
    || !equivalentProjection(internal.idleFrameIntervalsMs, evidence.idleFrameIntervalsMs)
    || !equivalentProjection(internal.idleFrameSegments, evidence.idleFrameSegments)
    || !equivalentProjection(internal.cadenceGaps, envelope.drive?.cadenceGaps)
    || !equivalentProjection(internal.burstRecoveries, envelope.drive?.burstRecoveries)
    || !equivalentProjection(internal.lifecycleContinuity, envelope.drive?.learned?.lifecycleContinuity))) {
    errors.push("raw-partial-private-projection");
  }
  if (common !== undefined && internal !== undefined) {
    for (const field of ["auxiliaryEvents", "idleFrameIntervalsMs", "idleFrameSegments",
      "interactionFrameIntervalsMs", "interactionFrameTimestamps", "interactionFrameWindows",
      "interactionWindows", "idleControlWindows", "longTaskObserver", "longTasks",
      "focusVisibilitySamples", "assetsRequestedDuringWindow", "measurementProtocolBlockers",
      "lifecycleContinuity", "sentinelOverflowCounts"]) {
      if (!equivalentProjection(internal[field], common[field])) {
        errors.push(`raw-partial-common-private-shared:${field}`);
      }
    }
  }
  return [...new Set(errors)];
}

function independentPartialRawErrors(envelope, expected) {
  const errors = [];
  const partial = envelope?.partialAttempt;
  if (!partial || partial.version !== "web06-partial-attempt-v1") {
    return ["raw-partial-version"];
  }
  if (partial.phase !== "failed" || partial.measurementCompleted !== false
    || envelope.measurementCompleted !== false || !envelope.setupFailure) {
    errors.push("raw-partial-completion-state");
  }
  if (partial.measurementStarted !== (envelope.measurementStarted === true)) {
    errors.push("raw-partial-measurement-state");
  }
  const protocolMode = envelope.target?.protocolMode;
  if (!hasOwn(INDEPENDENT_TARGET_PROTOCOL_MODES, envelope.target?.id)
    || INDEPENDENT_TARGET_PROTOCOL_MODES[envelope.target.id] !== protocolMode) {
    errors.push("raw-partial-target-protocol-mode");
  }
  for (const field of ["driverEvents", "cadenceGaps", "burstRecoveries", "pressureProofs"]) {
    if (!Array.isArray(partial[field])) errors.push(`raw-partial-${field}-shape`);
  }
  if (!partial.argumentCommitments || typeof partial.argumentCommitments !== "object"
    || Array.isArray(partial.argumentCommitments)) errors.push("raw-partial-argument-commitments-shape");
  const failure = partial.failure;
  if (!failure || typeof failure.code !== "string" || !failure.code.startsWith("WEB06_")
    || !["setup", "behavior"].includes(failure.dimension)) {
    errors.push("raw-partial-failure-shape");
  } else {
    if (failure.dimension !== independentlyClassifyHarnessFailure(failure.code)) {
      errors.push("raw-partial-failure-classification");
    }
    if (failure.dimension === "behavior" && envelope.measurementStarted !== true) {
      errors.push("raw-partial-premeasurement-behavior");
    }
    if (envelope.browserFailure?.messageCode !== failure.code) {
      errors.push("raw-partial-browser-failure-code");
    }
  }
  if (expected) {
    for (const [field, value] of Object.entries({
      scenarioRunId: expected.scenarioRunId,
      scenarioId: expected.scenarioId,
      schemaId: expected.schemaId,
      attemptId: expected.attemptId,
      targetId: expected.targetId,
    })) {
      const actual = field === "targetId" ? envelope.target?.id : envelope[field];
      if (value !== undefined && actual !== value) errors.push(`raw-partial-${field}-identity`);
    }
    if (expected.measurementStarted !== undefined
      && (envelope.measurementStarted === true) !== expected.measurementStarted) {
      errors.push("raw-partial-attempt-measurement-state");
    }
  }
  errors.push(...independentRawObservationErrors(envelope, false));
  errors.push(...independentPartialReceiptBindingErrors(envelope));
  const diagnosticScalar = (value) => value === undefined || value === null
    || ["number", "string", "boolean"].includes(typeof value);
  const boundedDenseDiagnosticArray = (value, maximum) =>
    independentArrayIsDense(value) && value.length <= maximum;
  const failureSentinel = partial.failureSentinel;
  if (failureSentinel !== undefined) {
    const callbacks = failureSentinel.callbackLedger;
    const diagnosticArrays = ["sentinelCallbacksMs", "unattributedInWindowCallbacksMs",
      "sentinelTotalPerEventMs", "sentinelTotalPerWindowMs"];
    if (!boundedDenseDiagnosticArray(failureSentinel.events, 512)
      || failureSentinel.events.some((row) => !row || typeof row !== "object" || Array.isArray(row))
      || !boundedDenseDiagnosticArray(callbacks, 8_192)
      || callbacks.some((callback) => !callback || typeof callback !== "object" || Array.isArray(callback))
      || diagnosticArrays.some((field) => failureSentinel[field] !== undefined
        && (!boundedDenseDiagnosticArray(failureSentinel[field], 8_192)
          || failureSentinel[field].some((value) => !diagnosticScalar(value))))
      || !diagnosticScalar(failureSentinel.callbackLedgerCapacity)
      || !diagnosticScalar(failureSentinel.callbackLedgerOverflowCount)
      || !diagnosticScalar(failureSentinel.sentinelAccountedCallbackCount)) {
      errors.push("raw-partial-sentinel-shape");
    }
  }
  const failureProtocol = partial.failureProtocolExport;
  if (protocolMode === "off" && failureProtocol !== undefined) {
    errors.push("raw-partial-product-private-protocol-present");
  }
  if (failureProtocol !== undefined) {
    const callbacks = failureProtocol.mainObserverCallbacks;
    const status = failureProtocol.status;
    if (!status || typeof status !== "object" || Array.isArray(status)
      || !boundedDenseDiagnosticArray(callbacks, 8_192)
      || callbacks.some((callback) => !callback || typeof callback !== "object" || Array.isArray(callback))
      || (failureProtocol.mainObserverCallbacksMs !== undefined
        && (!boundedDenseDiagnosticArray(failureProtocol.mainObserverCallbacksMs, 8_192)
          || failureProtocol.mainObserverCallbacksMs.some((value) => !diagnosticScalar(value))))
      || !diagnosticScalar(status.mainObserverCallbackCount)
      || !diagnosticScalar(status.mainObserverCallbackCapacity)
      || !diagnosticScalar(status.mainObserverCallbackOverflowCount)) {
      errors.push("raw-partial-main-callback-shape");
    }
    if (failureProtocol.header?.protocolVersion !== "web06-private-v1"
      || failureProtocol.header?.mode !== protocolMode) errors.push("raw-partial-private-protocol-header");
  }
  if (envelope.artifactResponseGuard !== undefined) {
    errors.push(...independentArtifactResponseGuardErrors(envelope, false));
  }
  return errors;
}

function independentRetainedRawReceiptBehaviorErrors(envelope) {
  if (envelope?.measurementStarted !== true) return [];
  if (independentPartialReceiptBindingErrors(envelope).length > 0) return [];
  const scenarioId = envelope?.scenarioId;
  const scenarioRunId = envelope?.scenarioRunId;
  const schemaId = envelope?.schemaId;
  const common = envelope?.commonReceipt;
  const internal = envelope?.privateReceipt;
  if (!independentReceiptIsDecisionShaped(common, scenarioId, scenarioRunId, schemaId, true)) return [];
  if (envelope?.target?.protocolMode !== "off"
    && !independentReceiptIsDecisionShaped(internal, scenarioId, scenarioRunId, schemaId, false)) return [];
  if (independentSharedRawProjectionErrors(envelope).length > 0) return [];
  if (internal !== undefined && independentPrivateProtocolProjectionErrors(envelope).length > 0) return [];
  const parsed = [
    deriveCommon(common),
    ...(internal === undefined ? [] : [deriveInternal(internal)]),
  ];
  return [...new Set(parsed.flatMap((result) => result.behaviorErrors ?? []))];
}

function independentArtifactResponseGuardErrors(envelope, requirePass) {
  const errors = [];
  const guard = envelope.target?.artifactResponseGuard;
  const target = envelope.target ?? {};
  if (!guard || JSON.stringify(Object.keys(guard).sort())
    !== JSON.stringify(["entries", "rootDocumentPath", "version"])) {
    return ["raw-artifact-response-guard-shape"];
  }
  if (guard.version !== "web06-artifact-response-guard-v1" || guard.rootDocumentPath !== "index.html"
    || !Array.isArray(guard.entries) || guard.entries.length < 3) errors.push("raw-artifact-response-guard-identity");
  const seen = new Set();
  for (const [index, entry] of (guard.entries ?? []).entries()) {
    if (!entry || JSON.stringify(Object.keys(entry).sort()) !== JSON.stringify(["bytes", "path", "sha256"])
      || typeof entry.path !== "string" || entry.path.startsWith("/") || entry.path.includes("\\")
      || entry.path.split("/").some((segment) => !segment || segment === "." || segment === "..")
      || !Number.isSafeInteger(entry.bytes) || entry.bytes < 0 || !SHA64.test(entry.sha256 ?? "")
      || seen.has(entry.path)) errors.push(`raw-artifact-response-guard-entry:${index + 1}`);
    seen.add(entry?.path);
  }
  const guardHash = sha256(Buffer.from(JSON.stringify(guard), "utf8"));
  if (guardHash !== target.artifactResponseGuardSha256
    || guardHash !== (envelope.commonReceipt ?? envelope.privateReceipt)?.source?.artifactResponseGuardSha256) {
    errors.push("raw-artifact-response-guard-hash-binding");
  }
  if (guard.entries?.[0]?.path !== "build-info.json" || guard.entries[0].sha256 !== target.buildInfoSha256
    || guard.entries?.[1]?.path !== "public-artifact-manifest.json"
    || guard.entries[1].sha256 !== target.artifactSha256 || !seen.has(guard.rootDocumentPath)) {
    errors.push("raw-artifact-response-guard-target-binding");
  }
  const summaries = [envelope.sourceProof?.artifactResponseGuard, envelope.artifactResponseGuard]
    .filter((summary) => summary !== undefined);
  if (requirePass && summaries.length !== 2) errors.push("raw-artifact-response-guard-summary-count");
  for (const [index, summary] of summaries.entries()) {
    const withoutHash = { ...summary };
    delete withoutHash.summarySha256;
    if (summary.version !== "web06-artifact-response-guard-summary-v1"
      || summary.summarySha256 !== sha256(Buffer.from(JSON.stringify(withoutHash), "utf8"))
      || summary.expectedGuardSha256 !== guardHash || summary.expectedEntryCount !== guard.entries.length
      || !Array.isArray(summary.observedPathCounts)
      || !exactJson(summary.observedPathCounts.map((row) => row.path), guard.entries.map((entry) => entry.path))
      || summary.observedPathCounts.some((row) => !Number.isSafeInteger(row.count) || row.count < 0)
      || summary.observedResponseCount !== summary.observedPathCounts.reduce((sum, row) => sum + row.count, 0)
        + (summary.unknownPathCount ?? 0)
      || summary.observedUniquePathCount !== summary.observedPathCounts.filter((row) => row.count > 0).length
        + (summary.unknownPathCount ?? 0)
      || summary.duplicateResponseCount !== summary.observedPathCounts.reduce((sum, row) => sum + Math.max(0, row.count - 1), 0)
      || summary.verifiedPathCountsSha256 !== sha256(Buffer.from(JSON.stringify(
        summary.observedPathCounts.map((row) => [row.path, row.count]),
      ), "utf8"))
      || !Array.isArray(summary.failureCodes)
      || summary.pass !== (summary.failureCodes.length === 0)) {
      errors.push(`raw-artifact-response-guard-summary:${index + 1}`);
    }
    if (requirePass && (summary.pass !== true || summary.unknownPathCount !== 0
      || summary.observedPathCounts.some((row) => row.count < 1))) {
      errors.push(`raw-artifact-response-guard-not-pass:${index + 1}`);
    }
  }
  const post = envelope.artifactResponseGuard;
  const sourceReceipt = envelope.commonReceipt ?? envelope.privateReceipt;
  if (requirePass && post?.summarySha256 !== sourceReceipt?.source?.artifactResponseGuardSummarySha256) {
    errors.push("raw-artifact-response-guard-summary-binding");
  }
  return errors;
}

const INDEPENDENT_COMPLETED_ARRAY_FIELDS = Object.freeze([
  "events", "auxiliaryEvents", "cadenceGaps", "idleFrameIntervalsMs", "interactionFrameIntervalsMs",
  "interactionFrameTimestamps", "interactionFrameWindows", "interactionWindows", "idleControlWindows",
  "longTasks", "focusVisibilitySamples", "assetsRequestedDuringWindow", "measurementProtocolBlockers",
]);
const INDEPENDENT_UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

function independentArrayIsDense(value) {
  if (!Array.isArray(value)) return false;
  for (let index = 0; index < value.length; index += 1) {
    if (!Object.prototype.hasOwnProperty.call(value, index)) return false;
  }
  return true;
}

function independentClockBoundaryIsShaped(rows) {
  return independentArrayIsDense(rows)
    && rows.length === WEB06_THRESHOLDS.calibration.exchangesPerBoundary;
}

function independentCalibrationIsShaped(value) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  return independentClockBoundaryIsShaped(value.pre)
    && independentClockBoundaryIsShaped(value.post);
}

function independentReceiptIsDecisionShaped(receipt, scenarioId, scenarioRunId, schemaId, common) {
  if (!receipt || typeof receipt !== "object" || Array.isArray(receipt)) return false;
  if (receipt.scenarioId !== scenarioId || receipt.scenarioRunId !== scenarioRunId
    || receipt.schemaId !== schemaId || receipt.measurementStarted !== true
    || receipt.measurementCompleted !== true) return false;
  for (const field of INDEPENDENT_COMPLETED_ARRAY_FIELDS) {
    if (!independentArrayIsDense(receipt[field])) return false;
  }
  if (common) {
    if (!independentArrayIsDense(receipt.commonSamples)
      || !independentArrayIsDense(receipt.unmatchedEvents)
      || !independentArrayIsDense(receipt.actions)) return false;
  } else if (!independentArrayIsDense(receipt.actions)
    || !independentArrayIsDense(receipt.pressureProofs)
    || !independentArrayIsDense(receipt.burstRecoveries)) return false;
  for (const action of receipt.actions) {
    if (!action || typeof action !== "object" || Array.isArray(action)) return false;
  }
  if (!receipt.sentinelOverflowCounts || typeof receipt.sentinelOverflowCounts !== "object"
    || Array.isArray(receipt.sentinelOverflowCounts)) return false;
  for (const field of ["auxiliaryEvents", "unmatchedEvents"]) {
    if (receipt[field] !== undefined) {
      for (const row of receipt[field]) {
        if (!row || typeof row !== "object" || Array.isArray(row)) return false;
      }
    }
  }
  for (const gap of receipt.cadenceGaps) {
    if (!gap || typeof gap !== "object" || Array.isArray(gap)
      || typeof gap.stepId !== "string" || !finite(gap.nominalGapMs)
      || !finite(gap.actualDriverGapMs)
      || (gap.rebasedAfterLateHost !== undefined
        && typeof gap.rebasedAfterLateHost !== "boolean")) return false;
  }
  for (const field of ["idleFrameIntervalsMs", "interactionFrameIntervalsMs",
    "interactionFrameTimestamps"]) {
    for (const value of receipt[field]) if (!finite(value)) return false;
  }
  for (const event of receipt.events) {
    if (!event || typeof event !== "object" || Array.isArray(event)
      || !Number.isSafeInteger(event.eventSequenceId) || !finite(event.eventTimestamp)
      || !finite(event.normalizedEventAt)
      || !INDEPENDENT_UUID_V4.test(event.pageInstanceId ?? "")) return false;
  }
  for (const sample of receipt.focusVisibilitySamples) {
    if (!sample || typeof sample !== "object" || !finite(sample.recordedAt)
      || typeof sample.focused !== "boolean" || typeof sample.visibilityState !== "string"
      || !INDEPENDENT_UUID_V4.test(sample.pageInstanceId ?? "")) return false;
  }
  for (const field of ["interactionFrameWindows", "interactionWindows", "idleControlWindows"]) {
    for (const row of receipt[field]) {
      if (!row || typeof row !== "object" || Array.isArray(row)
        || !INDEPENDENT_UUID_V4.test(row.pageInstanceId ?? "")) return false;
    }
  }
  if (receipt.interactionFrameWindows.some((row) => typeof row.windowId !== "string")
    || receipt.interactionWindows.some((row) => typeof row.windowId !== "string")) return false;
  for (const row of receipt.interactionFrameWindows) {
    if (!independentArrayIsDense(row.timestamps) || !independentArrayIsDense(row.intervalsMs)
      || row.timestamps.some((value) => !finite(value))
      || row.intervalsMs.some((value) => !finite(value))) return false;
  }
  for (const row of receipt.interactionWindows) {
    if (["startedAt", "endedAt", "startBoundaryRafAt", "endBoundaryRafAt"]
      .some((field) => !finite(row[field]))) return false;
  }
  for (const row of receipt.idleControlWindows) {
    if (typeof row.controlId !== "string" || !finite(row.startedAt) || !finite(row.endedAt)) return false;
  }
  for (const row of receipt.assetsRequestedDuringWindow) {
    if (!row || typeof row !== "object" || Array.isArray(row)
      || typeof row.name !== "string" || !finite(row.startTime)) return false;
  }
  if (receipt.measurementProtocolBlockers.some((value) => typeof value !== "string")) return false;
  if (common) {
    for (const sample of receipt.commonSamples) {
      if (!sample || typeof sample !== "object" || Array.isArray(sample)
        || !finite(sample.observedAt)
        || !sample.domObserved || typeof sample.domObserved !== "object"
        || Array.isArray(sample.domObserved)
        || (sample.firstDomObserved !== undefined
          && (!sample.firstDomObserved || typeof sample.firstDomObserved !== "object"
            || Array.isArray(sample.firstDomObserved)))) return false;
    }
  } else {
    for (const field of ["pressureProofs", "burstRecoveries"]) {
      for (const row of receipt[field]) {
        if (!row || typeof row !== "object" || Array.isArray(row)) return false;
      }
    }
  }
  for (const task of receipt.longTasks) {
    if (!task || typeof task !== "object" || Array.isArray(task)
      || !INDEPENDENT_UUID_V4.test(task.pageInstanceId ?? "")
      || !finite(task.startTime) || !finite(task.durationMs)
      || typeof task.overlapsInteractionWindow !== "boolean"
      || typeof task.overlapsIdleControl !== "boolean") return false;
  }
  if (!common) {
    for (const action of receipt.actions) {
      if (!action || typeof action !== "object") return false;
      for (const field of ["presentationExpected", "domObserved"]) {
        const projection = action[field];
        if (projection !== undefined && projection !== null
          && (typeof projection !== "object" || Array.isArray(projection))) return false;
      }
    }
  }
  if (scenarioId === "learned-row") {
    for (const name of ["preReload", "postReload"]) {
      const segment = receipt.calibrationSegments?.[name];
      if (!independentCalibrationIsShaped(segment?.driver)
        || (!common && !independentCalibrationIsShaped(segment?.worker))) return false;
    }
    return true;
  }
  return independentCalibrationIsShaped(receipt.calibration?.driver)
    && (common || independentCalibrationIsShaped(receipt.calibration?.worker));
}

const INDEPENDENT_RAW_SENTINEL_ARRAY_FIELDS = Object.freeze([
  "events", "auxiliaryEvents", "unmatchedEvents", "snapshots", "interactionWindows", "idleControlWindows",
  "interactionFrameWindows", "interactionFrameTimestamps", "interactionFrameIntervalsMs", "longTasks",
  "focusVisibilitySamples", "assetsRequestedDuringWindow", "callbackLedger", "sentinelCallbacksMs",
  "unattributedInWindowCallbacksMs", "sentinelTotalPerEventMs", "sentinelTotalPerWindowMs",
]);

function independentRawSentinelIsDecisionShaped(sentinel) {
  if (!sentinel || typeof sentinel !== "object" || Array.isArray(sentinel)
    || INDEPENDENT_RAW_SENTINEL_ARRAY_FIELDS.some((field) => !independentArrayIsDense(sentinel[field]))
    || !Number.isSafeInteger(sentinel.callbackLedgerCapacity) || sentinel.callbackLedgerCapacity < 0
    || !Number.isSafeInteger(sentinel.callbackLedgerOverflowCount) || sentinel.callbackLedgerOverflowCount < 0
    || !Number.isSafeInteger(sentinel.sentinelAccountedCallbackCount)
    || sentinel.sentinelAccountedCallbackCount < 0) return false;
  for (const field of ["events", "auxiliaryEvents", "unmatchedEvents", "snapshots", "interactionWindows",
    "idleControlWindows", "interactionFrameWindows", "longTasks", "focusVisibilitySamples",
    "assetsRequestedDuringWindow", "callbackLedger"]) {
    if (sentinel[field].some((row) => !row || typeof row !== "object" || Array.isArray(row))) return false;
  }
  if (sentinel.interactionFrameTimestamps.some((value) => !finite(value))
    || sentinel.interactionFrameIntervalsMs.some((value) => !finite(value))
    || sentinel.sentinelCallbacksMs.some((value) => !finite(value))
    || sentinel.unattributedInWindowCallbacksMs.some((value) => !finite(value))
    || sentinel.sentinelTotalPerEventMs.some((value) => !finite(value))
    || sentinel.sentinelTotalPerWindowMs.some((value) => !finite(value))) return false;
  const requiredCallbackKeys = ["kind", "pageInstanceId", "startedAt", "finishedAt", "durationMs"];
  const optionalCallbackKeys = ["eventSequenceId"];
  if (sentinel.callbackLedger.some((callback) =>
    requiredCallbackKeys.some((key) => !Object.hasOwn(callback, key))
    || Object.keys(callback).some((key) =>
      !requiredCallbackKeys.includes(key) && !optionalCallbackKeys.includes(key))
    || typeof callback.kind !== "string" || callback.kind.length === 0
    || typeof callback.pageInstanceId !== "string" || callback.pageInstanceId.length === 0
    || (callback.eventSequenceId !== undefined
      && (!Number.isSafeInteger(callback.eventSequenceId) || callback.eventSequenceId <= 0))
    || !finite(callback.startedAt) || !finite(callback.finishedAt) || !finite(callback.durationMs))) return false;
  return true;
}

function independentRawSentinelIntegrityErrors(sentinel) {
  const errors = [];
  const callbacks = sentinel?.callbackLedger;
  const events = sentinel?.events;
  const windows = sentinel?.interactionWindows;
  if (!Array.isArray(callbacks) || !Array.isArray(events) || !Array.isArray(windows)) {
    return ["raw-sentinel-callback-ledger-missing"];
  }
  if (!Number.isSafeInteger(sentinel.callbackLedgerCapacity)
    || callbacks.length > sentinel.callbackLedgerCapacity) {
    errors.push("raw-sentinel-callback-capacity");
  }
  if (!Number.isSafeInteger(sentinel.callbackLedgerOverflowCount)
    || sentinel.callbackLedgerOverflowCount !== 0) {
    errors.push("raw-sentinel-callback-overflow");
  }
  if (!Number.isSafeInteger(sentinel.sentinelAccountedCallbackCount)
    || sentinel.sentinelAccountedCallbackCount !== callbacks.length) {
    errors.push("raw-sentinel-callback-count");
  }
  if (callbacks.some((callback) => !finite(callback?.startedAt)
    || !finite(callback?.finishedAt)
    || !finite(callback?.durationMs)
    || callback.durationMs !== callback.finishedAt - callback.startedAt)) {
    errors.push("raw-sentinel-callback-timing");
  }
  if (!exactJson(sentinel.sentinelCallbacksMs, callbacks.map((callback) => callback.durationMs))) {
    errors.push("raw-sentinel-callback-duration-projection");
  }
  const unattributed = callbacks
    .filter((callback) => callback.eventSequenceId === undefined)
    .map((callback) => callback.durationMs);
  if (!exactJson(sentinel.unattributedInWindowCallbacksMs, unattributed)) {
    errors.push("raw-sentinel-unattributed-projection");
  }
  const ownerCounts = new Map();
  for (const event of events) {
    const key = `${event?.pageInstanceId}:${event?.eventSequenceId}`;
    ownerCounts.set(key, (ownerCounts.get(key) ?? 0) + 1);
  }
  if ([...ownerCounts.values()].some((count) => count !== 1)) {
    errors.push("raw-sentinel-event-owner-identity");
  }
  if (callbacks.some((callback) => callback.eventSequenceId !== undefined
    && ownerCounts.get(`${callback.pageInstanceId}:${callback.eventSequenceId}`) !== 1)) {
    errors.push("raw-sentinel-callback-owner");
  }
  const eventTotals = events.map((event) => callbacks
    .filter((callback) => callback.pageInstanceId === event.pageInstanceId
      && callback.eventSequenceId === event.eventSequenceId)
    .reduce((sum, callback) => sum + callback.durationMs, 0));
  if (!exactJson(sentinel.sentinelTotalPerEventMs, eventTotals)) {
    errors.push("raw-sentinel-event-total-projection");
  }
  const windowTotals = windows.map((window) => callbacks
    .filter((callback) => callback.pageInstanceId === window.pageInstanceId
      && callback.startedAt >= window.startedAt && callback.startedAt <= window.endedAt)
    .reduce((sum, callback) => sum + callback.durationMs, 0));
  if (!exactJson(sentinel.sentinelTotalPerWindowMs, windowTotals)) {
    errors.push("raw-sentinel-window-total-projection");
  }
  return [...new Set(errors)];
}

export function independentlyValidateRawSentinelIntegrity(sentinel) {
  const errors = independentRawSentinelIntegrityErrors(sentinel);
  return { pass: errors.length === 0, errors };
}

function independentRawProtocolIsDecisionShaped(envelope) {
  if (envelope.target?.protocolMode === "off") return envelope.protocolExport === undefined;
  const protocol = envelope.protocolExport;
  if (!protocol || typeof protocol !== "object" || Array.isArray(protocol)) return false;
  if (!protocol.status || typeof protocol.status !== "object" || Array.isArray(protocol.status)
    || !Number.isSafeInteger(protocol.status.mainObserverCallbackCount)
    || protocol.status.mainObserverCallbackCount < 0
    || !Number.isSafeInteger(protocol.status.mainObserverCallbackCapacity)
    || protocol.status.mainObserverCallbackCapacity < 0
    || !Number.isSafeInteger(protocol.status.mainObserverCallbackOverflowCount)
    || protocol.status.mainObserverCallbackOverflowCount < 0) return false;
  for (const field of ["events", "actions", "invalidations", "mainObserverCallbacks",
    "mainObserverCallbacksMs", "protocolWindowSegments"]) {
    if (!independentArrayIsDense(protocol[field])) return false;
  }
  for (const field of ["events", "actions", "mainObserverCallbacks", "protocolWindowSegments"]) {
    if (protocol[field].some((row) => !row || typeof row !== "object" || Array.isArray(row))) return false;
  }
  if (protocol.invalidations.some((value) => typeof value !== "string")
    || protocol.mainObserverCallbacksMs.some((value) => !finite(value))) return false;
  const requiredCallbackKeys = ["callbackId", "sequenceId", "operation", "startedAt", "finishedAt", "durationMs"];
  const optionalCallbackKeys = ["actionId", "eventId", "pageInstanceId"];
  if (protocol.mainObserverCallbacks.some((callback) =>
    requiredCallbackKeys.some((key) => !Object.hasOwn(callback, key))
    || Object.keys(callback).some((key) =>
      !requiredCallbackKeys.includes(key) && !optionalCallbackKeys.includes(key))
    || typeof callback.callbackId !== "string" || callback.callbackId.length === 0
    || !Number.isSafeInteger(callback.sequenceId) || callback.sequenceId <= 0
    || typeof callback.operation !== "string" || callback.operation.length === 0
    || ["actionId", "eventId", "pageInstanceId"].some((key) =>
      callback[key] !== undefined && (typeof callback[key] !== "string" || callback[key].length === 0))
    || !finite(callback.startedAt)
    || !finite(callback.finishedAt) || !finite(callback.durationMs))) return false;
  for (const action of protocol.actions) {
    if (!action.worker || typeof action.worker !== "object" || Array.isArray(action.worker)) return false;
    for (const field of ["runtimeSpans", "adapterSpans", "persistenceSpans", "collectorSpans",
      "observerFailures"]) {
      if (!independentArrayIsDense(action.worker[field])) return false;
    }
    for (const field of ["runtimeSpans", "adapterSpans", "persistenceSpans", "collectorSpans"]) {
      if (action.worker[field].some((row) => !row || typeof row !== "object" || Array.isArray(row))) return false;
    }
    if (action.worker.collectorSpans.some((span) =>
      !finite(span.startedAt) || !finite(span.finishedAt))) return false;
    if (action.worker.observerFailures.some((value) => typeof value !== "string")) return false;
  }
  return true;
}

/** Verifier-owned duplicate of the frozen completed raw decision-shape gate. */
export function independentlyValidateCompletedRawDecisionShape(envelope) {
  const scenarioId = envelope?.scenarioId;
  const scenarioRunId = envelope?.scenarioRunId;
  const schemaId = envelope?.schemaId;
  const run = hasOwn(SCENARIO_RUN_REGISTRY, scenarioRunId)
    ? SCENARIO_RUN_REGISTRY[scenarioRunId]
    : undefined;
  const pass = envelope?.version === "web06-raw-attempt-v1"
    && envelope.measurementStarted === true && envelope.measurementCompleted === true
    && independentRawAttemptOrdinalMatches(envelope)
    && hasOwn(SCENARIO_REGISTRY, scenarioId)
    && run?.scenarioId === scenarioId && run?.schema === schemaId
    && independentRawSentinelIsDecisionShaped(envelope.sentinel)
    && independentRawProtocolIsDecisionShaped(envelope)
    && independentReceiptIsDecisionShaped(envelope.commonReceipt, scenarioId, scenarioRunId, schemaId, true)
    && envelope.commonReceipt.roundId === `${scenarioId}-round-${envelope.attemptNumber}`
    && (envelope.privateReceipt === undefined
      || (independentReceiptIsDecisionShaped(envelope.privateReceipt, scenarioId, scenarioRunId, schemaId, false)
        && envelope.privateReceipt.roundId === `${scenarioId}-round-${envelope.attemptNumber}`));
  return {
    pass,
    errors: pass ? [] : ["WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID"],
  };
}

export function independentlyAuditRawEnvelope(envelope, expected) {
  const errors = [];
  if (envelope?.version !== "web06-raw-attempt-v1") return { pass: false, errors: ["raw-envelope-version"] };
  if (envelope.partialAttempt?.measurementCompleted === false || envelope.measurementCompleted === false) {
    errors.push(...independentPartialRawErrors(envelope, expected));
    return { pass: errors.length === 0, errors };
  }
  const decisionShape = independentlyValidateCompletedRawDecisionShape(envelope);
  if (!decisionShape.pass) return decisionShape;
  const sentinel = envelope.sentinel;
  if (!sentinel || typeof sentinel !== "object") return { pass: false, errors: ["raw-sentinel-missing"] };
  errors.push(...independentRawSentinelIntegrityErrors(sentinel));
  const protocolMode = envelope.target?.protocolMode;
  if (!hasOwn(INDEPENDENT_TARGET_PROTOCOL_MODES, envelope.target?.id)
    || INDEPENDENT_TARGET_PROTOCOL_MODES[envelope.target.id] !== protocolMode) {
    errors.push("raw-target-protocol-mode");
  }
  errors.push(...independentRawObservationErrors(envelope, true));
  if (!envelope.commonReceipt || typeof envelope.commonReceipt !== "object") errors.push("raw-common-receipt-missing");
  if (protocolMode === "off") {
    if (envelope.privateReceipt !== undefined || envelope.protocolExport !== undefined) {
      errors.push("raw-product-private-protocol-present");
    }
  } else if (!envelope.privateReceipt || !envelope.protocolExport
    || envelope.privateReceipt.mode !== envelope.target?.id
    || envelope.commonReceipt?.mode !== envelope.target?.id) {
    errors.push("raw-private-mode-completeness");
  }
  errors.push(...independentArtifactResponseGuardErrors(envelope, true));
  errors.push(...independentRawReceiptMetadataErrors(envelope));
  errors.push(...independentSharedRawProjectionErrors(envelope));
  const callbacks = sentinel.callbackLedger;
  if (!Array.isArray(callbacks) || !Number.isSafeInteger(sentinel.callbackLedgerCapacity)
    || !Number.isSafeInteger(sentinel.callbackLedgerOverflowCount)
    || !Number.isSafeInteger(sentinel.sentinelAccountedCallbackCount)) {
    errors.push("raw-sentinel-callback-shape");
  }
  if (!Array.isArray(sentinel.sentinelTotalPerEventMs)
    || sentinel.sentinelTotalPerEventMs.length !== (sentinel.events ?? []).length) {
    errors.push("raw-sentinel-event-total");
  }
  if (envelope.commonReceipt) {
    for (const field of ["events", "auxiliaryEvents", "unmatchedEvents", "interactionWindows", "idleControlWindows",
      "interactionFrameWindows",
      "interactionFrameTimestamps", "interactionFrameIntervalsMs", "longTasks", "focusVisibilitySamples",
      "assetsRequestedDuringWindow", "sentinelOverflowCounts"]) {
      if (!exactJson(envelope.commonReceipt[field], sentinel[field])) errors.push(`raw-common-projection:${field}`);
    }
    if (!Array.isArray(envelope.commonReceipt.actions) || envelope.commonReceipt.actions.length !== 0) {
      errors.push("raw-common-actions-present");
    }
    if (!exactJson(envelope.commonReceipt.pageSizeSetup, envelope.pageSizeSetup)
      || envelope.commonReceipt.candidatePageSize !== Number(envelope.pageSizeSetup?.measuredUiPageSize)) {
      errors.push("raw-common-page-size-projection");
    }
    try {
      const commonSamples = independentlyResolveCommonSamples({
        scenarioId: envelope.scenarioId,
        events: sentinel.events,
        snapshots: sentinel.snapshots,
      });
      if (!exactJson(commonSamples, envelope.commonReceipt.commonSamples)) {
        errors.push("raw-common-snapshot-projection");
      }
    } catch (error) {
      errors.push(error instanceof Error ? error.message : "raw-common-snapshot-projection");
    }
  }
  const protocol = envelope.protocolExport;
  if (envelope.privateReceipt) {
    errors.push(...independentPrivateProtocolProjectionErrors(envelope));
    if (!protocol || typeof protocol !== "object") errors.push("raw-private-protocol-missing");
  }
  const sourceReceipt = envelope.commonReceipt ?? envelope.privateReceipt;
  if (envelope.observedEnvironment?.observationSha256 !== sourceReceipt?.source?.observedEnvironmentSha256) {
    errors.push("raw-observed-environment-binding");
  }
  if (expected) {
    for (const [field, value] of Object.entries({
      scenarioRunId: expected.scenarioRunId,
      scenarioId: expected.scenarioId,
      schemaId: expected.schemaId,
      attemptId: expected.attemptId,
      targetId: expected.targetId,
    })) {
      const envelopeValue = field === "targetId" ? envelope.target?.id : envelope[field];
      const receiptValue = field === "targetId" ? sourceReceipt?.mode : sourceReceipt?.[field];
      if (value !== undefined && (envelopeValue !== value || receiptValue !== value)) {
        errors.push(`raw-${field}-identity`);
      }
    }
  }
  return { pass: errors.length === 0, errors };
}

function cadenceRange(nominalGapMs) {
  return nominalGapMs === 40 ? WEB06_THRESHOLDS.cadence.burst40
    : nominalGapMs === 120 ? WEB06_THRESHOLDS.cadence.burst120
      : nominalGapMs === 100 ? WEB06_THRESHOLDS.cadence.existing100
        : WEB06_THRESHOLDS.cadence.sustained60;
}

function independentOverflowErrors(receipt) {
  const counts = receipt.sentinelOverflowCounts;
  if (!counts || typeof counts !== "object" || Array.isArray(counts)
    || JSON.stringify(Object.keys(counts).sort()) !== JSON.stringify([...SENTINEL_OVERFLOW_FIELDS].sort())
    || Object.values(counts).some((value) => !Number.isSafeInteger(value) || value < 0)) {
    return ["SETUP_SENTINEL_OVERFLOW_COUNTERS_INVALID"];
  }
  return Object.entries(counts).filter(([, count]) => count > 0)
    .map(([field, count]) => `SETUP_SENTINEL_LEDGER_OVERFLOW:${field}:${count}`);
}

function independentIdentityErrors(receipt) {
  const errors = [];
  const source = receipt.source ?? {};
  const setup = (condition, code) => { if (condition) errors.push(code); };
  setup(receipt.metricContractVersion !== WEB06_METRIC_CONTRACT_VERSION, "SETUP_METRIC_VERSION_MISMATCH");
  setup(receipt.scenarioRegistryVersion !== WEB06_SCENARIO_REGISTRY_VERSION, "SETUP_SCENARIO_VERSION_MISMATCH");
  setup(receipt.behaviorPredicateVersion !== WEB06_BEHAVIOR_PREDICATE_VERSION,
    "SETUP_BEHAVIOR_PREDICATE_VERSION_MISMATCH");
  setup(!SHA40.test(source.commit ?? ""), "SETUP_SOURCE_COMMIT_INVALID");
  setup(!SHA40.test(source.tree ?? ""), "SETUP_SOURCE_TREE_INVALID");
  setup(source.treeState !== "clean", "SETUP_SOURCE_TREE_NOT_CLEAN");
  for (const [field, code] of [
    ["artifactSha256", "SETUP_ARTIFACT_HASH_INVALID"], ["archiveSha256", "SETUP_ARCHIVE_HASH_INVALID"],
    ["buildInfoSha256", "SETUP_BUILD_INFO_HASH_INVALID"],
    ["artifactResponseGuardSha256", "SETUP_ARTIFACT_RESPONSE_GUARD_HASH_INVALID"],
    ["artifactResponseGuardSummarySha256", "SETUP_ARTIFACT_RESPONSE_GUARD_SUMMARY_HASH_INVALID"],
    ["identityManifestSha256", "SETUP_IDENTITY_MANIFEST_HASH_INVALID"],
    ["runnerSourceManifestSha256", "SETUP_RUNNER_SOURCE_MANIFEST_HASH_INVALID"],
    ["runnerToolingManifestSha256", "SETUP_RUNNER_TOOLING_MANIFEST_HASH_INVALID"],
    ["runnerSourceObservationSha256", "SETUP_RUNNER_SOURCE_OBSERVATION_HASH_INVALID"],
    ["observedEnvironmentSha256", "SETUP_OBSERVED_ENVIRONMENT_HASH_INVALID"],
    ["collectorContractSha256", "SETUP_COLLECTOR_CONTRACT_HASH_INVALID"],
    ["scenarioIdsSha256", "SETUP_SCENARIO_SET_HASH_INVALID"],
    ["environmentManifestSha256", "SETUP_ENVIRONMENT_MANIFEST_HASH_INVALID"],
  ]) setup(!SHA64.test(source[field] ?? ""), code);
  setup(!SHA64.test(source.runnerSourcePostObservationSha256 ?? "")
    || source.runnerSourcePostObservationSha256 !== source.runnerSourceObservationSha256,
  "SETUP_RUNNER_SOURCE_POST_OBSERVATION_INVALID");
  setup(typeof source.environmentId !== "string" || !source.environmentId, "SETUP_ENVIRONMENT_ID_INVALID");
  setup(!["NONE", "A", "B", "C"].includes(source.selectedBranch), "SETUP_SELECTED_BRANCH_INVALID");
  setup(!["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE", "PRODUCTION_REDUCTION", "MEASURED_NO_GO"].includes(source.disposition),
    "SETUP_DISPOSITION_INVALID");
  setup(typeof receipt.roundId !== "string" || !receipt.roundId, "SETUP_ROUND_ID_INVALID");
  setup(typeof receipt.attemptId !== "string" || !receipt.attemptId, "SETUP_ATTEMPT_ID_INVALID");
  setup(receipt.measurementStarted !== true, "SETUP_MEASUREMENT_NOT_STARTED");
  setup(receipt.measurementCompleted !== true, "SETUP_MEASUREMENT_NOT_COMPLETED");
  const branchDispositionValid = source.selectedBranch === "NONE"
    ? ["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE", "MEASURED_NO_GO"].includes(source.disposition)
    : source.disposition === "PRODUCTION_REDUCTION";
  setup(!branchDispositionValid, "SETUP_BRANCH_DISPOSITION_MISMATCH");
  setup(!["PRODUCT", "BASE_MINIMAL", "BASE_FULL", "FINAL_MINIMAL", "FINAL_FULL"].includes(receipt.mode),
    "SETUP_MODE_INVALID");
  if (receipt.mode === "PRODUCT") {
    setup(source.selectedBranch !== "NONE" || source.disposition !== "DIAGNOSTIC", "SETUP_PRODUCT_DISPOSITION_INVALID");
  } else if (receipt.mode?.startsWith("BASE_")) {
    setup(source.selectedBranch !== "NONE" || !["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE"].includes(source.disposition),
      "SETUP_BASE_DISPOSITION_INVALID");
  } else if (receipt.mode?.startsWith("FINAL_")) {
    setup(source.selectedBranch === "NONE" ? source.disposition !== "MEASURED_NO_GO"
      : source.disposition !== "PRODUCTION_REDUCTION", "SETUP_FINAL_DISPOSITION_INVALID");
  }
  const run = hasOwn(SCENARIO_RUN_REGISTRY, receipt.scenarioRunId)
    ? SCENARIO_RUN_REGISTRY[receipt.scenarioRunId]
    : undefined;
  setup(run === undefined || run.scenarioId !== receipt.scenarioId || run.schema !== receipt.schemaId,
    "SETUP_SCENARIO_RUN_IDENTITY_MISMATCH");
  return errors;
}

function independentNormalizedArgs(action) {
  const raw = action.wireArgs;
  if (!Array.isArray(raw)) return undefined;
  if (action.kind === "importUserdb") {
    return raw.length === 1 && raw[0] === "<web06-redacted:userdb-text>"
      && SHA64.test(action.argumentCommitments?.userdbTextSha256 ?? "")
      ? [`sha256:${action.argumentCommitments.userdbTextSha256}`] : undefined;
  }
  if (action.kind === "customizeValue") {
    return raw.length === 3 && raw[2] === "<web06-redacted:customize-value>"
      && SHA64.test(action.argumentCommitments?.customizeValueSha256 ?? "")
      ? [raw[0], raw[1], `sha256:${action.argumentCommitments.customizeValueSha256}`] : undefined;
  }
  if (action.kind === "customize") {
    if (raw.length !== 1 || !raw[0] || typeof raw[0] !== "object" || Array.isArray(raw[0])) return undefined;
    const value = structuredClone(raw);
    const excluded = value[0].dictionaryExclude;
    if (excluded !== undefined) {
      if (excluded?.kind !== "web06-redacted:dictionary-exclude" || !Number.isSafeInteger(excluded.count)
        || excluded.count < 0
        || !SHA64.test(action.argumentCommitments?.dictionaryExcludeSha256 ?? "")) return undefined;
      value[0].dictionaryExclude = { kind: excluded.kind, count: excluded.count,
        sha256: action.argumentCommitments.dictionaryExcludeSha256 };
    }
    return value;
  }
  return structuredClone(raw);
}

function independentWireErrors(receipt) {
  const errors = [];
  const events = receipt.events ?? [];
  const actions = receipt.actions ?? [];
  const eventStart = receipt.protocolWindow?.receiptWindowStartEventSequenceId;
  const actionStart = receipt.protocolWindow?.receiptWindowStartActionSequenceId;
  if (!Number.isSafeInteger(eventStart) || eventStart < 1
    || !Number.isSafeInteger(actionStart) || actionStart < 1) {
    return ["WIRE_WINDOW_START_INVALID"];
  }
  const expected = independentExpandScenarioExpectedTimeline(receipt.scenarioId);
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
    return ["WIRE_WINDOW_SEGMENTS_INVALID"];
  }
  const windowForPage = (pageInstanceId) => protocolWindowSegments.find((segment) =>
    segment.pageInstanceId === undefined || segment.pageInstanceId === pageInstanceId);
  const actionByLocalId = new Map(actions.map((action) => [action.actionId, action]));
  const eventByLocalSequence = new Map(events.map((event) => [event.eventSequenceId, event]));
  const eventOrdinalsByPage = new Map();
  for (const actual of events) {
    const frozen = expected.events[actual.eventSequenceId - 1];
    if (frozen?.type === "browser-lifecycle") {
      if (actual.originOwner !== "harness-browser-lifecycle" || actual.wireEventSequenceId !== undefined
        || actual.wireEventId !== undefined || actual.wireIdentity !== undefined
        || actual.wireLinkedActionIds !== undefined) errors.push(`WIRE_EXTERNAL_EVENT_INVALID:${actual.eventSequenceId}`);
      continue;
    }
    const sourceWindow = windowForPage(actual.pageInstanceId);
    const ordinal = eventOrdinalsByPage.get(actual.pageInstanceId) ?? 0;
    if (!sourceWindow) {
      errors.push(`WIRE_EVENT_REALM:${actual.eventSequenceId}`);
      continue;
    }
    const wireSequence = sourceWindow.receiptWindowStartEventSequenceId + ordinal;
    eventOrdinalsByPage.set(actual.pageInstanceId, ordinal + 1);
    const wireId = `web06-event-${String(wireSequence).padStart(8, "0")}`;
    if (actual.wireEventSequenceId !== wireSequence || actual.wireEventId !== wireId
      || actual.wireIdentity?.eventSequenceId !== wireSequence || actual.wireIdentity?.eventId !== wireId) {
      errors.push(`WIRE_EVENT_OFFSET:${actual.eventSequenceId}`);
    }
    for (const field of ["type", "key", "code", "classification", "reason"]) {
      if (actual.wireIdentity?.[field] !== actual[field]) {
        errors.push(`WIRE_EVENT_IDENTITY:${actual.eventSequenceId}:${field}`);
      }
    }
    if (actual.wireIdentity?.timeStamp !== actual.eventTimestamp
      || actual.wireIdentity?.eventDeliveredAt !== actual.eventDeliveredAt) {
      errors.push(`WIRE_EVENT_TIMESTAMPS:${actual.eventSequenceId}`);
    }
    const expectedWireActions = (actual.mappedActionIds ?? [])
      .map((localId) => actionByLocalId.get(localId)?.wireActionId);
    if (!exactJson(actual.wireLinkedActionIds, expectedWireActions)) {
      errors.push(`WIRE_EVENT_ACTION_LINKS:${actual.eventSequenceId}`);
    }
  }
  const actionOrdinalsByPage = new Map();
  for (const actual of actions) {
    const sourceWindow = windowForPage(actual.pageInstanceId);
    const ordinal = actionOrdinalsByPage.get(actual.pageInstanceId) ?? 0;
    if (!sourceWindow) {
      errors.push(`WIRE_ACTION_REALM:${actual.sequenceId}`);
      continue;
    }
    const wireSequence = sourceWindow.receiptWindowStartActionSequenceId + ordinal;
    actionOrdinalsByPage.set(actual.pageInstanceId, ordinal + 1);
    const wireId = `web06-action-${String(wireSequence).padStart(8, "0")}`;
    const sourceEvent = actual.eventSequenceId === undefined
      ? undefined : eventByLocalSequence.get(actual.eventSequenceId);
    const sourceCauseAction = actual.causedByActionId === undefined
      ? undefined : actionByLocalId.get(actual.causedByActionId);
    const sourceCauseEvent = actual.causedByEventSequenceId === undefined
      ? undefined : eventByLocalSequence.get(actual.causedByEventSequenceId);
    if (actual.wireSequenceId !== wireSequence || actual.wireActionId !== wireId
      || actual.wireIdentity?.sequenceId !== wireSequence || actual.wireIdentity?.actionId !== wireId
      || actual.wireIdentity?.eventSequenceId !== sourceEvent?.wireEventSequenceId
      || actual.wireIdentity?.eventId !== sourceEvent?.wireEventId
      || actual.wireIdentity?.causedBySequenceId !== sourceCauseAction?.wireSequenceId
      || actual.wireIdentity?.causedByActionId !== sourceCauseAction?.wireActionId
      || actual.wireIdentity?.causedByEventSequenceId !== sourceCauseEvent?.wireEventSequenceId
      || actual.wireIdentity?.causedByEventId !== sourceCauseEvent?.wireEventId) {
      errors.push(`WIRE_ACTION_OFFSET:${actual.sequenceId}`);
    }
    if (!actual.returnedWireIdentity || !exactJson(actual.wireIdentity, actual.returnedWireIdentity)) {
      errors.push(`WIRE_RETURNED_IDENTITY:${actual.sequenceId}`);
    }
    for (const [wireField, localField] of [
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
    ]) {
      if (actual.wireIdentity?.[wireField] !== actual[localField]) {
        errors.push(`WIRE_ACTION_IDENTITY:${actual.sequenceId}:${wireField}`);
      }
    }
    if (!Array.isArray(actual.wireArgs)) {
      errors.push(`WIRE_ACTION_ARGS_MISSING:${actual.sequenceId}`);
    } else {
      const normalized = independentNormalizedArgs(actual);
      if (!normalized || !exactJson(normalized, actual.args)) {
        errors.push(`WIRE_ACTION_ARGS_NORMALIZATION:${actual.sequenceId}`);
      }
    }
  }
  return errors;
}

function independentTimelineErrors(receipt, row) {
  const errors = [];
  const expected = independentExpandScenarioExpectedTimeline(row.id);
  const events = receipt.events ?? [];
  const actions = receipt.actions ?? [];
  if (events.length !== row.expectedDomEventCount) errors.push(`EVENT_COUNT:${events.length}!=${row.expectedDomEventCount}`);
  if (actions.length !== row.expectedActionCount) errors.push(`ACTION_COUNT:${actions.length}!=${row.expectedActionCount}`);
  const contiguousErrors = (rows, field, prefix) => {
    const values = rows.map((item) => item[field]);
    if (new Set(values).size !== values.length) errors.push(`${prefix}_DUPLICATE_ID`);
    const invalidIndex = values.findIndex((value, index) => value !== index + 1);
    if (invalidIndex >= 0) {
      errors.push(`${prefix}_${values.includes(invalidIndex + 1) ? "REORDERED" : "MISSING"}_ID`);
    }
  };
  contiguousErrors(events, "eventSequenceId", "EVENT");
  contiguousErrors(actions, "sequenceId", "ACTION");
  if (new Set(actions.map((item) => item.actionId)).size !== actions.length) {
    errors.push("ACTION_DUPLICATE_ACTION_ID");
  }
  errors.push(...independentWireErrors(receipt));
  for (let index = 0; index < Math.min(events.length, expected.events.length); index += 1) {
    const actual = events[index];
    const frozen = expected.events[index];
    if (actual.stepId !== frozen.stepId || actual.type !== frozen.type) {
      errors.push(`EVENT_REORDERED:${index + 1}`);
    }
    if (actual.key !== frozen.key || actual.code !== frozen.code) {
      errors.push(`EVENT_IDENTITY:${index + 1}`);
    }
    if (actual.classification !== frozen.classification || actual.reason !== frozen.reason) {
      errors.push(`EVENT_CLASSIFICATION:${index + 1}`);
    }
    if (!exactJson(actual.mappedActionIds, frozen.mappedActionIds)) {
      errors.push(`EVENT_ACTION_CARDINALITY:${index + 1}`);
    }
  }
  for (let index = 0; index < Math.min(actions.length, expected.actions.length); index += 1) {
    const actual = actions[index];
    const frozen = expected.actions[index];
    if (actual.actionId !== frozen.actionId || actual.eventSequenceId !== frozen.eventSequenceId
      || actual.stepId !== frozen.stepId) errors.push(`ACTION_REORDERED:${index + 1}`);
    if (actual.kind !== frozen.kind || actual.classification !== frozen.classification) {
      errors.push(`ACTION_CLASSIFICATION:${index + 1}`);
    }
    if (actual.originKind !== frozen.originKind || actual.originReason !== frozen.originReason
      || actual.causedByActionId !== frozen.causedByActionId
      || actual.causedBySequenceId !== frozen.causedBySequenceId
      || actual.causedByEventSequenceId !== frozen.causedByEventSequenceId) {
      errors.push(`ACTION_ORIGIN:${index + 1}`);
    }
    if (!exactJson(actual.args, frozen.args)) errors.push(`ACTION_ARGUMENTS:${index + 1}`);
    if (!hasOwn(ACTION_REGISTRY, actual.kind)) errors.push(`ACTION_UNCLASSIFIED:${index + 1}`);
  }
  return errors;
}

function independentAuxiliaryErrors(receipt) {
  const errors = [];
  const expected = receipt.scenarioId === "extended-scheduler-barriers"
    ? [{ stepId: "extended-option-target", type: "click" }] : [];
  const actual = (receipt.auxiliaryEvents ?? []).map(({ stepId, type }) => ({ stepId, type }));
  if (!exactJson(actual, expected)) errors.push("AUXILIARY_DOM_EVENT_SEQUENCE");
  for (const event of receipt.auxiliaryEvents ?? []) {
    if (!finite(event.eventTimestamp) || !finite(event.sentinelObservedAt)
      || event.sentinelObservedAt < event.eventTimestamp) errors.push(`AUXILIARY_DOM_EVENT_TIME:${event.stepId ?? "unknown"}`);
  }
  return errors;
}

function independentReceiptPrivacyErrors(value, location = "", errors = []) {
  if (Array.isArray(value)) {
    value.forEach((child, index) => independentReceiptPrivacyErrors(child, `${location}[${index}]`, errors));
    return errors;
  }
  if (!value || typeof value !== "object") {
    if (typeof value === "string" && independentPrivacyStringInvalid(value)) {
      errors.push(`PUBLIC_PRIVACY_VALUE:${location}`);
    }
    return errors;
  }
  for (const [key, child] of Object.entries(value)) {
    const childLocation = location ? `${location}.${key}` : key;
    if (INDEPENDENT_FORBIDDEN_KEY.test(key)) errors.push(`PUBLIC_PRIVACY_KEY:${childLocation}`);
    independentReceiptPrivacyErrors(child, childLocation, errors);
  }
  return errors;
}

export function independentlyValidatePointerFreePrivacy(value) {
  const errors = independentReceiptPrivacyErrors(value);
  return { pass: errors.length === 0, errors };
}

function independentMissingArrayErrors(receipt, fields, prefix) {
  const errors = [];
  for (const field of fields) {
    if (!Array.isArray(receipt[field])) errors.push(`${prefix}_${field.toUpperCase()}_ARRAY_MISSING`);
  }
  return errors;
}

const INDEPENDENT_PROTOCOL_THRESHOLD_BLOCKERS = new Set([
  "MAIN_OBSERVER_CALLBACK_CEILING",
  "COLLECTOR_CALLBACK_CEILING",
]);

function independentMeasurementProtocolDisposition(blockers) {
  const setupErrors = [];
  const thresholdViolations = [];
  for (const blocker of blockers) {
    if (INDEPENDENT_PROTOCOL_THRESHOLD_BLOCKERS.has(blocker)) {
      thresholdViolations.push(`measurement-protocol:${blocker}`);
    } else {
      setupErrors.push(`SETUP_MEASUREMENT_PROTOCOL:${blocker}`);
    }
  }
  return { setupErrors, thresholdViolations };
}

function independentExpectedCommonSamples(row) {
  const expected = independentExpandScenarioExpectedTimeline(row.id);
  let input = "";
  const samples = [];
  for (const step of row.steps) {
    for (const action of step.actions) {
      if (action.kind === "processKey") {
        const raw = action.args[0];
        const key = typeof raw === "string" && raw.startsWith("{") && raw.endsWith("}")
          ? raw.slice(1, -1) : "";
        if (key === "BackSpace") input = input.slice(0, -1);
        else if (["Escape", "space", "Return"].includes(key)) input = "";
        else if (!["Page_Down", "Page_Up", "Down", "Up"].includes(key) && key.length === 1) input += key;
      } else if (action.kind === "selectCandidate") input = "";
    }
    if (step.expectedLogicalInputAfter !== undefined) input = step.expectedLogicalInputAfter;
    if (step.source === "browser-lifecycle") input = "";
    if (step.sample === "none") continue;
    const owners = expected.actions.filter((action) => action.stepId === step.id);
    const foreground = owners.filter((action) => action.background !== true);
    const owner = step.sample === "terminal"
      ? (foreground.length !== owners.length ? foreground[0] ?? owners[0] : foreground.at(-1)) : owners[0];
    samples.push({ stepId: step.id, sampleKind: step.sample,
      eventSequenceId: owner?.eventSequenceId ?? owner?.causedByEventSequenceId,
      expectedInput: input, stressDeadline: owner?.stressDeadline === true });
  }
  return samples;
}

/** Verifier-owned reconstruction of the public common endpoints from sentinel snapshots. */
export function independentlyResolveCommonSamples({ scenarioId, events, snapshots }) {
  invariant(hasOwn(SCENARIO_REGISTRY, scenarioId), "WEB06_INDEPENDENT_SCENARIO_UNKNOWN", scenarioId);
  const row = SCENARIO_REGISTRY[scenarioId];
  invariant(Array.isArray(events) && Array.isArray(snapshots), "WEB06_INDEPENDENT_COMMON_RAW_ARRAYS");
  const frozen = independentExpectedCommonSamples(row);
  const exactByIndex = new Map();
  for (let index = 0; index < frozen.length; index += 1) {
    const sample = frozen[index];
    const event = events.find((candidate) => candidate.eventSequenceId === sample.eventSequenceId);
    const exact = snapshots
      .filter((snapshot) => (event?.pageInstanceId === undefined || snapshot.pageInstanceId === event.pageInstanceId)
        && finite(snapshot.observedAt) && snapshot.observedAt >= (event?.normalizedEventAt ?? Number.POSITIVE_INFINITY)
        && snapshot.stepId === sample.stepId && snapshot.stableDoubleRaf === true
        && snapshot.domObserved?.logicalInputProjection === sample.expectedInput)
      .sort((left, right) => left.observedAt - right.observedAt)[0];
    if (exact) exactByIndex.set(index, exact);
  }
  const missingDom = {
    input: "",
    renderedInput: "",
    logicalInputProjection: "",
    candidates: [],
    pageShape: { previousDisabled: true, nextDisabled: true, highlightedIndex: -1, visibleCount: 0 },
    textareaValue: "",
    selectionStart: 0,
    selectionEnd: 0,
  };
  return frozen.map((sample, index) => {
    const event = events.find((candidate) => candidate.eventSequenceId === sample.eventSequenceId);
    const exact = exactByIndex.get(index);
    if (exact) {
      return {
        ...sample,
        outcome: sample.sampleKind === "terminal" ? "terminal" : "painted",
        pageInstanceId: exact.pageInstanceId,
        observedAt: exact.observedAt,
        stableDoubleRaf: exact.stableDoubleRaf === true,
        firstDomObserved: structuredClone(exact.firstDomObserved),
        domObserved: structuredClone(exact.domObserved),
        domFingerprintSha256: sha256(Buffer.from(JSON.stringify(exact.domObserved), "utf8")),
      };
    }
    let coveringIndex = -1;
    if (sample.sampleKind === "covering") {
      for (let candidate = index + 1; candidate <= Math.min(index + 2, frozen.length - 1); candidate += 1) {
        const covering = frozen[candidate];
        if (covering.sampleKind === "covering" && typeof sample.expectedInput === "string"
          && typeof covering.expectedInput === "string" && sample.expectedInput.length < covering.expectedInput.length
          && covering.expectedInput.startsWith(sample.expectedInput) && exactByIndex.has(candidate)) {
          coveringIndex = candidate;
          break;
        }
      }
    }
    if (coveringIndex >= 0) {
      const covering = exactByIndex.get(coveringIndex);
      return {
        ...sample,
        outcome: "superseded",
        supersededByStepId: frozen[coveringIndex].stepId,
        pageInstanceId: covering.pageInstanceId,
        observedAt: covering.observedAt,
        stableDoubleRaf: covering.stableDoubleRaf === true,
        firstDomObserved: structuredClone(covering.firstDomObserved),
        domObserved: structuredClone(covering.domObserved),
        domFingerprintSha256: sha256(Buffer.from(JSON.stringify(covering.domObserved), "utf8")),
      };
    }
    return {
      ...sample,
      outcome: "missing",
      pageInstanceId: event?.pageInstanceId,
      observedAt: event?.normalizedEventAt ?? 0,
      stableDoubleRaf: false,
      firstDomObserved: structuredClone(missingDom),
      domObserved: structuredClone(missingDom),
      domFingerprintSha256: sha256(Buffer.from(JSON.stringify(missingDom), "utf8")),
    };
  });
}

function independentNormalizeWireActionArgs(kind, wireArgs, commitments = {}) {
  invariant(Array.isArray(wireArgs), "WEB06_INDEPENDENT_WIRE_ARGS");
  if (kind === "importUserdb") {
    invariant(wireArgs.length === 1 && wireArgs[0] === "<web06-redacted:userdb-text>"
      && SHA64.test(commitments.userdbTextSha256 ?? ""), "WEB06_INDEPENDENT_IMPORT_COMMITMENT");
    return [`sha256:${commitments.userdbTextSha256}`];
  }
  if (kind === "customizeValue") {
    invariant(wireArgs.length === 3 && wireArgs[2] === "<web06-redacted:customize-value>"
      && SHA64.test(commitments.customizeValueSha256 ?? ""), "WEB06_INDEPENDENT_CUSTOMIZE_VALUE_COMMITMENT");
    return [wireArgs[0], wireArgs[1], `sha256:${commitments.customizeValueSha256}`];
  }
  if (kind === "customize") {
    invariant(wireArgs.length === 1 && wireArgs[0] && typeof wireArgs[0] === "object"
      && !Array.isArray(wireArgs[0]), "WEB06_INDEPENDENT_CUSTOMIZE_ARGS");
    const preferences = structuredClone(wireArgs[0]);
    if (preferences.dictionaryExclude !== undefined) {
      const excluded = preferences.dictionaryExclude;
      invariant(excluded && typeof excluded === "object" && !Array.isArray(excluded)
        && excluded.kind === "web06-redacted:dictionary-exclude"
        && Number.isSafeInteger(excluded.count) && excluded.count >= 0
        && SHA64.test(commitments.dictionaryExcludeSha256 ?? ""),
      "WEB06_INDEPENDENT_DICTIONARY_COMMITMENT");
      preferences.dictionaryExclude = { kind: excluded.kind, count: excluded.count,
        sha256: commitments.dictionaryExcludeSha256 };
    }
    return [preferences];
  }
  return structuredClone(wireArgs);
}

function independentAggregateSpan(spans, predicate) {
  const selected = (spans ?? []).filter(predicate);
  if (!selected.length) return null;
  invariant(selected.every((span) => finite(span.startedAt) && finite(span.finishedAt)
    && span.finishedAt >= span.startedAt && ["success", "error"].includes(span.outcome)),
  "WEB06_INDEPENDENT_RAW_WORKER_SPAN");
  return {
    start: Math.min(...selected.map((span) => span.startedAt)),
    end: Math.max(...selected.map((span) => span.finishedAt)),
    outcomes: [...new Set(selected.map((span) => span.outcome))],
  };
}

function independentWorkerSpanRecord(worker, full) {
  if (!full) return { abi: null, responseExtract: null, jsonParse: null, adapterTranslate: null, persistence: null };
  return {
    abi: independentAggregateSpan(worker.runtimeSpans, (span) => span.stage === "abi-call"),
    responseExtract: independentAggregateSpan(worker.runtimeSpans,
      (span) => ["response-json-accessor", "response-byte-extraction"].includes(span.stage)),
    jsonParse: independentAggregateSpan(worker.runtimeSpans,
      (span) => ["response-json-parse", "response-shape-decode", "response-handled-accessor", "response-free"]
        .includes(span.stage)),
    adapterTranslate: independentAggregateSpan(worker.adapterSpans, (span) => span.stage === "adapter-translation"),
    persistence: independentAggregateSpan(worker.persistenceSpans, () => true),
  };
}

function independentCommitFingerprint(presentation, previousTextareaValue) {
  const next = presentation.domObserved.textareaValue;
  let prefix = 0;
  while (prefix < previousTextareaValue.length && prefix < next.length
    && previousTextareaValue[prefix] === next[prefix]) prefix += 1;
  let suffix = 0;
  while (suffix < previousTextareaValue.length - prefix && suffix < next.length - prefix
    && previousTextareaValue.at(-1 - suffix) === next.at(-1 - suffix)) suffix += 1;
  return {
    exactCommitText: next.slice(prefix, next.length - suffix),
    textareaValue: next,
    selectionStart: presentation.domObserved.selectionStart,
    selectionEnd: presentation.domObserved.selectionEnd,
    visibleComposition: presentation.domObserved.input !== "",
  };
}

function independentPrivateProtocolProjectionErrors(envelope) {
  const errors = [];
  const receipt = envelope.privateReceipt;
  const protocol = envelope.protocolExport;
  if (!receipt || !protocol) return ["raw-private-protocol-missing"];
  const full = envelope.target?.protocolMode === "full";
  if (protocol.header?.protocolVersion !== "web06-private-v1"
    || protocol.header?.mode !== envelope.target?.protocolMode) errors.push("raw-private-protocol-header");
  const eventStart = receipt.protocolWindow?.receiptWindowStartEventSequenceId;
  const actionStart = receipt.protocolWindow?.receiptWindowStartActionSequenceId;
  const resetStatus = envelope.preflight?.protocolAfterReset?.status;
  if (!Number.isSafeInteger(eventStart) || !Number.isSafeInteger(actionStart)
    || resetStatus?.receiptWindowStartEventSequenceId !== eventStart
    || resetStatus?.receiptWindowStartActionSequenceId !== actionStart) errors.push("raw-private-protocol-window");
  const wireEvents = protocol.events;
  const wireActions = protocol.actions;
  const externalLifecycleEvents = protocol.externalLifecycleEvents ?? [];
  if (!Array.isArray(wireEvents) || !Array.isArray(wireActions) || !Array.isArray(externalLifecycleEvents)
    || wireEvents.length + externalLifecycleEvents.length !== (receipt.events ?? []).length
    || wireActions.length !== (receipt.actions ?? []).length) {
    return [...errors, "raw-private-protocol-cardinality"];
  }
  const expected = independentExpandScenarioExpectedTimeline(receipt.scenarioId);
  const ordinaryExpectedEvents = expected.events.filter((event) => event.type !== "browser-lifecycle");
  const expectedLifecycleEvents = expected.events.filter((event) => event.type === "browser-lifecycle");
  if (wireEvents.length !== ordinaryExpectedEvents.length
    || externalLifecycleEvents.length !== expectedLifecycleEvents.length) {
    return [...errors, "raw-private-protocol-source-cardinality"];
  }
  for (let index = 0; index < expectedLifecycleEvents.length; index += 1) {
    const frozen = expectedLifecycleEvents[index];
    const marker = externalLifecycleEvents[index];
    if (marker?.eventSequenceId !== frozen.eventSequenceId || marker?.stepId !== frozen.stepId
      || marker?.type !== frozen.type || marker?.key !== frozen.key || marker?.code !== frozen.code
      || marker?.classification !== frozen.classification || marker?.reason !== frozen.reason
      || !exactJson(marker?.mappedActionIds, frozen.mappedActionIds)
      || marker?.originOwner !== "harness-browser-lifecycle" || !finite(marker?.eventTimestamp)
      || marker?.normalizedEventAt !== marker?.eventTimestamp
      || !finite(marker?.sentinelObservedAt) || marker.sentinelObservedAt < marker.normalizedEventAt) {
      errors.push(`raw-private-lifecycle-marker:${index + 1}`);
    }
  }
  const protocolWindowSegments = protocol.protocolWindowSegments ?? [{
    pageInstanceId: undefined,
    receiptWindowStartEventSequenceId: eventStart,
    receiptWindowStartActionSequenceId: actionStart,
  }];
  if (!equivalentProjection(receipt.protocolWindowSegments, protocol.protocolWindowSegments)
    || protocolWindowSegments.length !== (receipt.scenarioId === "learned-row" ? 2 : 1)) {
    errors.push("raw-private-protocol-window-segments");
  }
  if (receipt.scenarioId === "learned-row") {
    const preProtocol = envelope.drive?.learned?.preSegment?.protocol;
    const expectedSegments = [
      {
        pageInstanceId: preProtocol?.header?.pageInstanceId,
        receiptWindowStartEventSequenceId: preProtocol?.status?.receiptWindowStartEventSequenceId,
        receiptWindowStartActionSequenceId: preProtocol?.status?.receiptWindowStartActionSequenceId,
      },
      {
        pageInstanceId: protocol?.header?.pageInstanceId,
        receiptWindowStartEventSequenceId: protocol?.status?.receiptWindowStartEventSequenceId,
        receiptWindowStartActionSequenceId: protocol?.status?.receiptWindowStartActionSequenceId,
      },
    ];
    if (!equivalentProjection(protocolWindowSegments, expectedSegments)) {
      errors.push("raw-private-protocol-window-segment-source");
    }
  }
  const windowForPage = (pageInstanceId) => protocolWindowSegments.find((segment) =>
    segment.pageInstanceId === undefined || segment.pageInstanceId === pageInstanceId);
  const compositeKey = (pageInstanceId, value) => `${pageInstanceId ?? "single-page"}:${value}`;
  const wireActionIdToLocal = new Map(wireActions.map((action, index) => [
    compositeKey(action.web06PageInstanceId, action.identity?.actionId),
    expected.actions[index]?.actionId,
  ]));
  const wireActionSequenceToLocal = new Map(wireActions.map((action, index) => [
    compositeKey(action.web06PageInstanceId, action.identity?.sequenceId),
    expected.actions[index]?.sequenceId,
  ]));
  const eventOrdinalsByPage = new Map();
  const projectedEvents = wireEvents.map((wire, index) => {
    const identity = wire.identity ?? {};
    const frozen = ordinaryExpectedEvents[index] ?? {};
    const localSequence = frozen.eventSequenceId;
    const driver = envelope.drive?.driverEvents?.[index] ?? {};
    const sourceWindow = windowForPage(wire.web06PageInstanceId);
    const ordinal = eventOrdinalsByPage.get(wire.web06PageInstanceId) ?? 0;
    if (!sourceWindow || identity.eventSequenceId
      !== sourceWindow.receiptWindowStartEventSequenceId + ordinal) {
      errors.push(`raw-private-protocol-event-segment-sequence:${index + 1}`);
    }
    eventOrdinalsByPage.set(wire.web06PageInstanceId, ordinal + 1);
    return {
      eventSequenceId: localSequence,
      pageInstanceId: wire.web06PageInstanceId,
      stepId: frozen.stepId,
      type: identity.type,
      key: identity.key,
      code: identity.code,
      classification: identity.classification,
      reason: identity.reason,
      mappedActionIds: (wire.linkedActionIds ?? [])
        .map((wireId) => wireActionIdToLocal.get(compositeKey(wire.web06PageInstanceId, wireId))
          ?? `unmapped:${wireId}`),
      wireEventSequenceId: identity.eventSequenceId,
      wireEventId: identity.eventId,
      wireIdentity: structuredClone(identity),
      wireLinkedActionIds: structuredClone(wire.linkedActionIds),
      eventTimestamp: identity.timeStamp,
      normalizedEventAt: identity.timeStamp,
      eventDeliveredAt: identity.eventDeliveredAt,
      requestedDriverDispatchAt: driver.requestedDriverDispatchAt,
      actualDriverDispatchAt: driver.actualDriverDispatchAt,
      modifiers: ["ctrlKey", "metaKey", "altKey", "shiftKey"].filter((field) => identity[field]),
    };
  });
  projectedEvents.push(...externalLifecycleEvents.map((marker) => structuredClone(marker)));
  projectedEvents.sort((left, right) => left.eventSequenceId - right.eventSequenceId);
  if (!equivalentProjection(projectedEvents, receipt.events)) {
    errors.push("raw-private-protocol-event-projection");
    projectedEvents.forEach((event, index) => {
      for (const key of new Set([...Object.keys(event), ...Object.keys(receipt.events?.[index] ?? {})])) {
        if (!equivalentProjection(event[key], receipt.events?.[index]?.[key])) {
          errors.push(`raw-private-protocol-event-projection:${index + 1}:${key}`);
        }
      }
    });
  }
  const eventByWireSequence = new Map(projectedEvents.filter((event) => event.wireEventSequenceId !== undefined)
    .map((event) => [compositeKey(event.pageInstanceId, event.wireEventSequenceId), event]));
  let previousTextareaValue = envelope.drive?.initialDomObserved?.textareaValue ?? "";
  const projectedActions = [];
  const actionOrdinalsByPage = new Map();
  try {
    for (let wireIndex = 0; wireIndex < wireActions.length; wireIndex += 1) {
      const wire = wireActions[wireIndex];
      const identity = wire.identity ?? {};
      const frozen = expected.actions[wireIndex] ?? {};
      const localSequence = frozen.sequenceId;
      const sourceWindow = windowForPage(wire.web06PageInstanceId);
      const ordinal = actionOrdinalsByPage.get(wire.web06PageInstanceId) ?? 0;
      if (!sourceWindow || identity.sequenceId
        !== sourceWindow.receiptWindowStartActionSequenceId + ordinal) {
        errors.push(`raw-private-protocol-action-segment-sequence:${wireIndex + 1}`);
      }
      actionOrdinalsByPage.set(wire.web06PageInstanceId, ordinal + 1);
      const localEventSequence = identity.eventSequenceId === undefined
        ? undefined : eventByWireSequence.get(compositeKey(wire.web06PageInstanceId,
          identity.eventSequenceId))?.eventSequenceId;
      const causedByEventSequenceId = identity.causedByEventSequenceId === undefined
        ? undefined : eventByWireSequence.get(compositeKey(wire.web06PageInstanceId,
          identity.causedByEventSequenceId))?.eventSequenceId;
      const event = eventByWireSequence.get(compositeKey(wire.web06PageInstanceId,
        identity.eventSequenceId ?? identity.causedByEventSequenceId));
      const presentation = wire.presentation;
      const lifecycle = wire.lifecycle;
      const terminal = presentation ?? lifecycle;
      const terminalKind = presentation && lifecycle ? "ambiguous"
        : presentation ? "presentation" : lifecycle ? "lifecycle" : "missing";
      const worker = wire.worker ?? {};
      const rawSequence = structuredClone(identity.rawInputSequence);
      const argumentCommitments = envelope.drive?.argumentCommitments;
      const stepCommitments = argumentCommitments && typeof argumentCommitments === "object"
        && hasOwn(argumentCommitments, frozen.stepId)
        ? argumentCommitments[frozen.stepId]
        : undefined;
      const commitments = stepCommitments && typeof stepCommitments === "object"
        && hasOwn(stepCommitments, wire.name)
        ? stepCommitments[wire.name]
        : {};
      const action = {
        actionId: `a${localSequence}`,
        sequenceId: localSequence,
        pageInstanceId: wire.web06PageInstanceId,
        eventSequenceId: localEventSequence,
        causedByActionId: identity.causedByActionId === undefined ? undefined
          : wireActionIdToLocal.get(compositeKey(wire.web06PageInstanceId, identity.causedByActionId)),
        causedBySequenceId: identity.causedBySequenceId === undefined
          ? undefined : wireActionSequenceToLocal.get(compositeKey(wire.web06PageInstanceId,
            identity.causedBySequenceId)),
        causedByEventSequenceId,
        stepId: frozen.stepId,
        kind: wire.name,
        stressDeadline: frozen.stressDeadline === true,
        args: independentNormalizeWireActionArgs(wire.name, wire.args, commitments),
        classification: identity.actionClass,
        supersedable: identity.supersedable,
        compositionEpochId: identity.compositionEpochId,
        supersessionSubRunId: identity.supersessionSubRunId,
        originKind: identity.originKind,
        originReason: identity.originReason,
        wireSequenceId: identity.sequenceId,
        wireActionId: identity.actionId,
        wireArgs: structuredClone(wire.args),
        argumentCommitments: structuredClone(commitments),
        wireIdentity: structuredClone(identity),
        returnedWireIdentity: structuredClone(wire.returnedIdentity),
        driverDispatchAt: event?.actualDriverDispatchAt,
        actionEnqueuedAt: identity.actionEnqueuedAt,
        mainQueueDepth: identity.mainQueueDepthAtEnqueue,
        workerSentAt: identity.workerSentAt,
        workerDispatchDepth: identity.workerDispatchDepth,
        workerMessageReceivedAt: worker.workerMessageReceivedAt,
        workerActionStartedAt: worker.workerActionStartedAt,
        workerFinishedAt: worker.workerFinishedAt,
        mainResponseReceivedAt: wire.mainResponseReceivedAt,
        responseMappingStartedAt: wire.responseMappingStartedAt,
        responseMappingFinishedAt: wire.responseMappingFinishedAt,
        terminalKind,
        stateUpdateScheduledAt: terminal?.stateUpdateScheduledAt,
        stateCommittedAt: presentation?.stateCommittedAt,
        stateAppliedAt: presentation?.stateCommittedAt ?? lifecycle?.terminalObservedAt,
        ...(presentation?.outcome === "painted"
          ? { paintObservedAt: presentation.paintObservedAt ?? presentation.terminalObservedAt } : {}),
        ...(["committed", "barrier-completed", "failure"].includes(terminal?.outcome)
          ? { terminalObservedAt: terminal?.terminalObservedAt } : {}),
        outcome: terminal?.outcome ?? "failure",
        supersededBySequenceId: presentation?.supersededBySequenceId === undefined
          ? undefined : wireActionSequenceToLocal.get(compositeKey(wire.web06PageInstanceId,
            presentation.supersededBySequenceId)),
        rawActionSequence: rawSequence,
        logicalInput: presentation?.presentationExpected?.input ?? "",
        workerSpans: independentWorkerSpanRecord(worker, full),
        presentationExpected: structuredClone(presentation?.presentationExpected),
        domObserved: structuredClone(presentation?.domObserved),
        beforeDomDigest: presentation?.beforePresentationDigest,
        afterDomDigest: presentation?.domObservedDigest,
        adapterProjectionDigest: presentation?.adapterProjectionDigest,
        presentationExpectedDigest: presentation?.presentationExpectedDigest,
        domObservedDigest: presentation?.domObservedDigest,
        presentationDigest: presentation?.presentationDigest,
        lifecycleEffect: lifecycle === undefined ? undefined : {
          effect: lifecycle.effect,
          effectDigest: lifecycle.effectDigest,
          workerEffectDigest: lifecycle.workerEffectDigest,
          mainEffectDigest: lifecycle.mainEffectDigest,
          listenerEffectCount: lifecycle.listenerEffectCount,
          persistenceCompleted: lifecycle.persistenceCompleted,
        },
      };
      action.engineRawProof = structuredClone(worker.engineRaw);
      action.resultSummary = structuredClone(worker.resultSummary);
      if (full) {
        const rawJson = worker.engineRawJson;
        action.persistenceRan = (worker.persistenceSpans?.length ?? 0) > 0;
        action.engineRaw = rawJson === undefined ? undefined : {
          actionKind: action.kind,
          compositionEpochId: action.compositionEpochId,
          supersessionSubRunId: action.supersessionSubRunId,
          rawActionSequence: structuredClone(rawSequence),
          rawResponseJson: rawJson,
          rawResponseSha256: sha256(Buffer.from(rawJson, "utf8")),
        };
      }
      if (presentation?.outcome === "committed" && presentation.domObserved) {
        action.commitFingerprint = independentCommitFingerprint(presentation, previousTextareaValue);
      }
      if (presentation?.domObserved?.textareaValue !== undefined) {
        previousTextareaValue = presentation.domObserved.textareaValue;
      }
      projectedActions.push(action);
    }
  } catch (error) {
    errors.push(error instanceof Error ? error.message : "raw-private-protocol-action-projection");
  }
  if (!equivalentProjection(projectedActions, receipt.actions)) {
    errors.push("raw-private-protocol-action-projection");
    projectedActions.forEach((action, index) => {
      for (const key of new Set([...Object.keys(action), ...Object.keys(receipt.actions?.[index] ?? {})])) {
        if (!equivalentProjection(action[key], receipt.actions?.[index]?.[key])) {
          errors.push(`raw-private-protocol-action-projection:${index + 1}:${key}`);
        }
      }
    });
  }
  return errors;
}

function independentRawReceiptMetadataErrors(envelope) {
  const errors = [];
  const target = envelope.target ?? {};
  const attemptNumberSafe = Number.isSafeInteger(envelope.attemptNumber)
    && envelope.attemptNumber >= 1;
  const attemptNumberValid = attemptNumberSafe && independentRawAttemptOrdinalMatches(envelope);
  if (!attemptNumberSafe) errors.push("raw-attempt-number");
  else if (!attemptNumberValid) errors.push("raw-attempt-ordinal");
  const expectedSource = {
    commit: target.sourceCommit,
    tree: target.sourceTree,
    treeState: target.treeState,
    archiveSha256: target.archiveSha256,
    buildInfoSha256: target.buildInfoSha256,
    artifactSha256: target.artifactSha256,
    artifactResponseGuardSha256: target.artifactResponseGuardSha256,
    artifactResponseGuardSummarySha256: envelope.artifactResponseGuard?.summarySha256,
    identityManifestSha256: envelope.identityManifestSha256,
    runnerSourceManifestSha256: envelope.runnerSourceManifestSha256,
    runnerToolingManifestSha256: envelope.attemptSourceBefore?.toolingManifestSha256,
    runnerSourceObservationSha256: envelope.attemptSourceBefore?.observationSha256,
    runnerSourcePostObservationSha256: envelope.attemptSourceAfter?.observationSha256,
    observedEnvironmentSha256: envelope.observedEnvironment?.observationSha256,
    collectorContractSha256: target.collectorContractSha256,
    scenarioIdsSha256: envelope.scenarioIdsSha256,
    selectedBranch: target.pinnedSelectedBranch,
    disposition: target.pinnedDisposition,
    environmentManifestSha256: envelope.environmentManifestSha256,
    environmentId: envelope.environmentId,
  };
  for (const [surface, receipt] of [["common", envelope.commonReceipt], ["private", envelope.privateReceipt]]) {
    if (receipt === undefined) continue;
    if (!equivalentProjection(receipt.source, expectedSource)) errors.push(`raw-${surface}-source-metadata-projection`);
    for (const [field, expected] of Object.entries({
      scenarioRunId: envelope.scenarioRunId,
      scenarioId: envelope.scenarioId,
      schemaId: envelope.schemaId,
      attemptId: envelope.attemptId,
      mode: target.id,
      measurementStarted: true,
      measurementCompleted: true,
    })) if (receipt[field] !== expected) errors.push(`raw-${surface}-receipt-metadata:${field}`);
    if (!attemptNumberValid
      || receipt.roundId !== `${envelope.scenarioId}-round-${envelope.attemptNumber}`) {
      errors.push(`raw-${surface}-receipt-metadata:roundId`);
    }
  }
  return errors;
}

function independentProtocolHealthBlockers(protocol, requireCallbackLedger = false) {
  if (!protocol || typeof protocol !== "object") return ["PRIVATE_PROTOCOL_EXPORT_MISSING"];
  const blockers = [];
  const status = protocol.status;
  if (status?.valid !== true) blockers.push("PRIVATE_PROTOCOL_INVALID");
  if (status?.queueDepth !== 0 || status?.runningActionId !== undefined) blockers.push("ACTION_QUEUE_NOT_IDLE");
  if (status?.pendingFanoutActions !== 0) blockers.push("PENDING_FANOUT_ACTIONS");
  if (status?.pendingTerminalActions !== 0) blockers.push("PENDING_TERMINAL_ACTIONS");
  if (!Array.isArray(protocol.invalidations) || protocol.invalidations.length > 0) blockers.push("PRIVATE_PROTOCOL_INVALIDATIONS");
  if (requireCallbackLedger) {
    const callbacks = protocol.mainObserverCallbacks;
    const durations = protocol.mainObserverCallbacksMs;
    if (!Array.isArray(callbacks) || !Array.isArray(durations)) {
      blockers.push("MAIN_OBSERVER_CALLBACK_LEDGER_MISSING");
    } else {
      const ids = callbacks.map((callback) => callback?.callbackId);
      const sequences = callbacks.map((callback) => callback?.sequenceId);
      const rowsValid = callbacks.every((callback) =>
        callback && typeof callback === "object" && !Array.isArray(callback)
        && typeof callback.callbackId === "string" && callback.callbackId.length > 0
        && Number.isSafeInteger(callback.sequenceId) && callback.sequenceId > 0
        && typeof callback.operation === "string" && callback.operation.length > 0
        && finite(callback.startedAt) && finite(callback.finishedAt)
        && callback.finishedAt >= callback.startedAt && finite(callback.durationMs)
        && callback.durationMs >= 0
        && callback.durationMs === callback.finishedAt - callback.startedAt
        && (callback.actionId === undefined
          || (typeof callback.actionId === "string" && callback.actionId.length > 0))
        && (callback.eventId === undefined
          || (typeof callback.eventId === "string" && callback.eventId.length > 0)));
      const orderedUnique = new Set(ids).size === ids.length
        && new Set(sequences).size === sequences.length
        && sequences.every((sequence, index) => index === 0 || sequence > sequences[index - 1]);
      if (!rowsValid || !orderedUnique || durations.some((value) => !finite(value) || value < 0)) {
        blockers.push("MAIN_OBSERVER_CALLBACK_LEDGER_INVALID");
      }
      if (!exactJson(durations, callbacks.map((callback) => callback?.durationMs))) {
        blockers.push("MAIN_OBSERVER_CALLBACK_DURATION_PROJECTION_MISMATCH");
      }
      if (durations.some((value) =>
        value >= WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)) {
        blockers.push("MAIN_OBSERVER_CALLBACK_CEILING");
      }
    }
    if (!Number.isSafeInteger(status?.mainObserverCallbackCount)
      || !Number.isSafeInteger(status?.mainObserverCallbackCapacity)
      || !Number.isSafeInteger(status?.mainObserverCallbackOverflowCount)
      || status.mainObserverCallbackCount < 0 || status.mainObserverCallbackCapacity < 0
      || status.mainObserverCallbackOverflowCount < 0) {
      blockers.push("MAIN_OBSERVER_CALLBACK_STATUS_INVALID");
    } else {
      if (Array.isArray(callbacks) && status.mainObserverCallbackCount !== callbacks.length) {
        blockers.push("MAIN_OBSERVER_CALLBACK_COUNT_MISMATCH");
      }
      if (status.mainObserverCallbackCount > status.mainObserverCallbackCapacity) {
        blockers.push("MAIN_OBSERVER_CALLBACK_CAPACITY_EXCEEDED");
      }
      if (status.mainObserverCallbackOverflowCount !== 0) {
        blockers.push("MAIN_OBSERVER_CALLBACK_OVERFLOW");
      }
    }
  }
  if (protocol.actions !== undefined) {
    if (!Array.isArray(protocol.actions)) blockers.push("PRIVATE_PROTOCOL_ACTION_EXPORT_INVALID");
    else for (const action of protocol.actions) {
      if (!action.returnedIdentity) blockers.push("RETURNED_WIRE_IDENTITY_MISSING");
      if (!action.worker || !Array.isArray(action.worker.observerFailures)) {
        blockers.push("WORKER_OBSERVER_FAILURE_EXPORT_MISSING");
      } else if (action.worker.observerFailures.length > 0) blockers.push("WORKER_OBSERVER_FAILURE");
      const callbacks = action.worker?.collectorSpans;
      if (!Array.isArray(callbacks)) {
        blockers.push("WORKER_COLLECTOR_SPAN_EXPORT_MISSING");
      } else {
        if (callbacks.some((span) => !span || typeof span !== "object" || Array.isArray(span)
          || !finite(span.startedAt) || !finite(span.finishedAt)
          || span.finishedAt < span.startedAt)) blockers.push("COLLECTOR_CALLBACK_SPAN_INVALID");
        if (callbacks.some((span) => finite(span?.startedAt) && finite(span?.finishedAt)
          && span.finishedAt - span.startedAt
          >= WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)) {
          blockers.push("COLLECTOR_CALLBACK_CEILING");
        }
      }
    }
  }
  return [...new Set(blockers)];
}

function independentSharedRawProjectionErrors(envelope) {
  const errors = [];
  const common = envelope.commonReceipt;
  const internal = envelope.privateReceipt;
  const evidence = envelope.measurementEvidence;
  if (!common || !evidence || typeof evidence !== "object") return ["raw-measurement-evidence-missing"];
  if (!equivalentProjection(common.eventClockProbe, evidence.eventClockProbe)
    || !equivalentProjection(common.eventClockSegments, evidence.eventClockSegments)
    || !equivalentProjection(common.calibration, { driver: evidence.calibration?.driver })
    || !equivalentProjection(common.calibrationSegments, evidence.calibrationSegments === undefined ? undefined : {
      preReload: { driver: evidence.calibrationSegments.preReload?.driver },
      postReload: { driver: evidence.calibrationSegments.postReload?.driver },
    })
    || !equivalentProjection(common.idleFrameIntervalsMs, evidence.idleFrameIntervalsMs)
    || !equivalentProjection(common.idleFrameSegments, evidence.idleFrameSegments)) {
    errors.push("raw-common-measurement-evidence-projection");
  }
  if (internal && (!equivalentProjection(internal.eventClockProbe, evidence.eventClockProbe)
    || !equivalentProjection(internal.eventClockSegments, evidence.eventClockSegments)
    || !equivalentProjection(internal.calibration, evidence.calibration)
    || !equivalentProjection(internal.calibrationSegments, evidence.calibrationSegments)
    || !equivalentProjection(internal.idleFrameIntervalsMs, evidence.idleFrameIntervalsMs)
    || !equivalentProjection(internal.idleFrameSegments, evidence.idleFrameSegments))) {
    errors.push("raw-private-measurement-evidence-projection");
  }
  for (const receipt of [common, internal].filter(Boolean)) {
    if (!equivalentProjection(receipt.cadenceGaps, envelope.drive?.cadenceGaps)) {
      errors.push(`raw-${receipt === common ? "common" : "private"}-drive-cadence-projection`);
    }
  }
  if (internal && (!equivalentProjection(internal.burstRecoveries, envelope.drive?.burstRecoveries)
    || !equivalentProjection(internal.lifecycleContinuity, envelope.drive?.learned?.lifecycleContinuity))) {
    errors.push("raw-private-drive-projection");
  }
  if (!equivalentProjection(common.lifecycleContinuity, envelope.drive?.learned?.lifecycleContinuity)) {
    errors.push("raw-common-lifecycle-projection");
  }
  if (envelope.scenarioId === "learned-row") {
    const commonLifecycle = (common.events ?? []).filter((event) => event.type === "browser-lifecycle");
    const privateLifecycle = (internal?.events ?? []).filter((event) => event.type === "browser-lifecycle");
    const protocolLifecycle = envelope.protocolExport?.externalLifecycleEvents ?? [];
    if (commonLifecycle.length !== 1 || privateLifecycle.length !== 1 || protocolLifecycle.length !== 1
      || !equivalentProjection(commonLifecycle[0], privateLifecycle[0])
      || !equivalentProjection(commonLifecycle[0], protocolLifecycle[0])
      || !equivalentProjection(commonLifecycle[0], envelope.drive?.learned?.lifecycleMarker)) {
      errors.push("raw-learned-lifecycle-marker-projection");
    }
  }
  if (internal) {
    for (const field of ["auxiliaryEvents", "idleFrameIntervalsMs", "idleFrameSegments", "interactionFrameIntervalsMs",
      "interactionFrameTimestamps", "interactionFrameWindows", "interactionWindows", "idleControlWindows",
      "longTaskObserver", "longTasks", "focusVisibilitySamples", "assetsRequestedDuringWindow",
      "measurementProtocolBlockers", "lifecycleContinuity", "sentinelOverflowCounts"]) {
      if (!equivalentProjection(internal[field], common[field])) errors.push(`raw-common-private-shared:${field}`);
    }
  }
  const row = hasOwn(SCENARIO_REGISTRY, envelope.scenarioId)
    ? SCENARIO_REGISTRY[envelope.scenarioId]
    : undefined;
  if (!row || envelope.completion?.timedOut !== false
    || envelope.completion?.expectedActionCount !== row.expectedActionCount
    || (internal && (envelope.completion?.observedActionCount !== row.expectedActionCount
      || envelope.protocolExport?.actions?.length !== row.expectedActionCount))) {
    errors.push("raw-completion-projection");
  }
  const expectedBlockers = envelope.target?.protocolMode === "off" ? [] : [
    ...(envelope.completion?.timedOut ? ["PROTOCOL_COMPLETION_TIMEOUT"] : []),
    ...independentProtocolHealthBlockers(envelope.protocolExport, true),
  ];
  if (!equivalentProjection(common.measurementProtocolBlockers, expectedBlockers)
    || (internal && !equivalentProjection(internal.measurementProtocolBlockers, expectedBlockers))) {
    errors.push("raw-measurement-protocol-blocker-projection");
  }
  return errors;
}

function independentBehaviorPredicateErrors(receipt) {
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
    const observed = sample.domObserved;
    const expected = predicate.expected;
    const texts = (observed?.candidates ?? []).map((candidate) => candidate.text);
    if (expected.renderedInput !== undefined && observed?.renderedInput !== expected.renderedInput) {
      errors.push(`BEHAVIOR_RENDERED_INPUT:${stepId}`);
    }
    if (expected.candidateTextsExact !== undefined && !exactJson(texts, expected.candidateTextsExact)) {
      errors.push(`BEHAVIOR_CANDIDATE_EXACT:${stepId}`);
    }
    if (expected.candidateTextsPrefix !== undefined
      && expected.candidateTextsPrefix.some((text, index) => texts[index] !== text)) {
      errors.push(`BEHAVIOR_CANDIDATE_PREFIX:${stepId}`);
    }
    if (expected.candidateTextsInclude !== undefined
      && expected.candidateTextsInclude.some((text) => !texts.includes(text))) {
      errors.push(`BEHAVIOR_CANDIDATE_INCLUDE:${stepId}`);
    }
    for (const [field, actual] of [
      ["visibleCount", observed?.pageShape?.visibleCount],
      ["previousDisabled", observed?.pageShape?.previousDisabled],
      ["nextDisabled", observed?.pageShape?.nextDisabled],
      ["textareaValue", observed?.textareaValue],
      ["selectionStart", observed?.selectionStart],
      ["selectionEnd", observed?.selectionEnd],
    ]) {
      if (expected[field] !== undefined && actual !== expected[field]) errors.push(`BEHAVIOR_${field.toUpperCase()}:${stepId}`);
    }
    if (expected.commitTextExact !== undefined && observed?.textareaValue !== expected.commitTextExact) {
      errors.push(`BEHAVIOR_COMMIT_TEXT:${stepId}`);
    }
    if (expected.visibleComposition !== undefined
      && (observed?.renderedInput !== "") !== expected.visibleComposition) errors.push(`BEHAVIOR_VISIBLE_COMPOSITION:${stepId}`);
    if (expected.persistenceCompleted === true
      && (typeof receipt.lifecycleContinuity?.measurementId !== "string"
        || !receipt.lifecycleContinuity.measurementId
        || receipt.lifecycleContinuity?.pre?.measurementId !== receipt.lifecycleContinuity.measurementId
        || receipt.lifecycleContinuity?.post?.measurementId !== receipt.lifecycleContinuity.measurementId
        || typeof receipt.lifecycleContinuity?.pre?.continuityNonce !== "string"
        || !receipt.lifecycleContinuity.pre.continuityNonce
        || receipt.lifecycleContinuity.pre.continuityNonce !== receipt.lifecycleContinuity?.post?.continuityNonce
        || receipt.lifecycleContinuity?.pre?.terminal?.persistenceCompleted !== true
        || receipt.lifecycleContinuity?.post?.phase !== "post-reload-bound")) {
      errors.push(`BEHAVIOR_PERSISTENCE_CONTINUITY:${stepId}`);
    }
  }
  return errors;
}

function independentCommonEndpointErrors(receipt, row) {
  const errors = [];
  errors.push(...independentAuxiliaryErrors(receipt));
  if (receipt.unmatchedEvents.length) errors.push("COMMON_UNMATCHED_DOM_EVENT");
  if (receipt.candidatePageSize !== 6) errors.push("COMMON_CANDIDATE_PAGE_SIZE_NOT_SIX");
  const pageSize = receipt.pageSizeSetup;
  if (!pageSize || !exactJson(pageSize.uiTransition, [6, 7, 6]) || pageSize.configuredPageSize !== 6
    || pageSize.sevenRows !== 7 || pageSize.restoredControlValue !== "6" || pageSize.realPreferencesControl !== true) {
    errors.push("COMMON_PAGE_SIZE_UI_TRANSITION_INVALID");
  }
  const expected = independentExpectedCommonSamples(row);
  const samples = receipt.commonSamples;
  if (samples.length !== expected.length) errors.push(`COMMON_SAMPLE_COUNT:${samples.length}!=${expected.length}`);
  const eventById = new Map(receipt.events.map((event) => [event.eventSequenceId, event]));
  for (let index = 0; index < Math.min(samples.length, expected.length); index += 1) {
    const sample = samples[index];
    const frozen = expected[index];
    const event = eventById.get(frozen.eventSequenceId);
    if (sample.stepId !== frozen.stepId || sample.sampleKind !== frozen.sampleKind
      || sample.eventSequenceId !== frozen.eventSequenceId
      || (event?.pageInstanceId !== undefined && sample.pageInstanceId !== event.pageInstanceId)) {
      errors.push(`COMMON_SAMPLE_IDENTITY:${index + 1}`);
    }
    if (!event || !finite(sample.observedAt) || sample.observedAt < event.normalizedEventAt) {
      errors.push(`COMMON_SAMPLE_TIME:${frozen.stepId}`);
      continue;
    }
    const coveringKind = frozen.sampleKind === "covering";
    if (!(coveringKind ? ["painted", "superseded"].includes(sample.outcome) : sample.outcome === "terminal")) {
      errors.push(`COMMON_SAMPLE_OUTCOME:${frozen.stepId}`);
    }
    let endpoint = sample;
    if (sample.outcome === "superseded") {
      const targetIndex = expected.findIndex((candidate) => candidate.stepId === sample.supersededByStepId);
      const target = samples[targetIndex];
      const targetExpected = expected[targetIndex];
      if (targetIndex <= index || targetIndex - index > WEB06_THRESHOLDS.sustained.supersessionSequenceLag.max
        || target?.outcome !== "painted" || target?.observedAt !== sample.observedAt
        || targetExpected?.sampleKind !== "covering" || targetExpected.expectedInput === frozen.expectedInput
        || !targetExpected.expectedInput.startsWith(frozen.expectedInput)) errors.push(`COMMON_SUPERSESSION_INVALID:${frozen.stepId}`);
      else endpoint = target;
    }
    const sampleStable = sample.firstDomObserved !== undefined && exactJson(sample.firstDomObserved, sample.domObserved);
    const endpointStable = endpoint.firstDomObserved !== undefined && exactJson(endpoint.firstDomObserved, endpoint.domObserved);
    const expectedInput = sample.outcome === "superseded"
      ? expected.find((candidate) => candidate.stepId === sample.supersededByStepId)?.expectedInput : frozen.expectedInput;
    if (sample.stableDoubleRaf !== sampleStable || endpoint.stableDoubleRaf !== endpointStable
      || !sampleStable || !endpointStable || endpoint.domObserved?.logicalInputProjection !== expectedInput) {
      errors.push(`COMMON_DOM_ENDPOINT:${frozen.stepId}`);
    }
    const dom = endpoint.domObserved;
    if (typeof dom?.renderedInput !== "string" || dom.input !== dom.renderedInput
      || dom.logicalInputProjection !== dom.renderedInput.replaceAll(" ", "")) errors.push(`COMMON_DOM_INPUT_PROJECTION:${frozen.stepId}`);
    const candidates = dom?.candidates;
    const page = dom?.pageShape;
    if (!Number.isSafeInteger(page?.visibleCount) || page.visibleCount < 0 || page.visibleCount > 6
      || !Array.isArray(candidates) || candidates.length !== page.visibleCount
      || candidates.some((candidate) => !candidate || typeof candidate.label !== "string"
        || typeof candidate.text !== "string" || typeof candidate.comment !== "string" || typeof candidate.source !== "string")
      || typeof page.previousDisabled !== "boolean" || typeof page.nextDisabled !== "boolean"
      || !Number.isSafeInteger(page.highlightedIndex) || typeof dom.textareaValue !== "string"
      || !Number.isSafeInteger(dom.selectionStart) || !Number.isSafeInteger(dom.selectionEnd)) {
      errors.push(`COMMON_PAGE_SHAPE:${frozen.stepId}`);
    }
    const digest = dom === undefined ? undefined : sha256(Buffer.from(JSON.stringify(dom), "utf8"));
    if (!SHA64.test(endpoint.domFingerprintSha256 ?? "") || endpoint.domFingerprintSha256 !== digest) {
      errors.push(`COMMON_DOM_FINGERPRINT:${frozen.stepId}`);
    }
    if (coveringKind && !(candidates?.length > 0)) errors.push(`COMMON_CANDIDATE_ENDPOINT:${frozen.stepId}`);
    if (!finite(event.actualDriverDispatchAt) || !finite(event.requestedDriverDispatchAt)) {
      errors.push(`COMMON_DRIVER_DISPATCH:${frozen.stepId}`);
    }
  }
  const firstEligible = samples.find((sample) => sample.sampleKind === "covering"
    && ["painted", "superseded"].includes(sample.outcome));
  const firstPositivePageShape = samples.map((sample) => sample.domObserved?.pageShape)
    .find((pageShape) => Number.isSafeInteger(pageShape?.visibleCount) && pageShape.visibleCount > 0);
  if (firstPositivePageShape?.visibleCount !== 6) {
    errors.push("COMMON_FIRST_ELIGIBLE_ENDPOINT_PAGE_SIZE_NOT_SIX");
  }
  const firstEndpoint = firstEligible?.outcome === "superseded"
    ? samples.find((sample) => sample.stepId === firstEligible.supersededByStepId) : firstEligible;
  if (firstEndpoint?.domObserved?.pageShape?.visibleCount !== 6) errors.push("COMMON_FIRST_MEASURED_ENDPOINT_NOT_SIX_ROWS");
  errors.push(...independentBehaviorPredicateErrors(receipt));
  return errors;
}

function independentProjectRawResponse(rawJson) {
  const root = JSON.parse(rawJson);
  invariant(root && typeof root === "object" && !Array.isArray(root) && typeof root.handled === "boolean"
    && Array.isArray(root.commits) && root.commits.every((value) => typeof value === "string")
    && (root.context === null || (root.context && typeof root.context === "object" && !Array.isArray(root.context)))
    && (root.status === null || (root.status && typeof root.status === "object" && !Array.isArray(root.status))),
  "WEB06_INDEPENDENT_RAW_RESPONSE_SHAPE");
  const committed = root.commits.length ? root.commits.join("") : undefined;
  if (!root.handled) return { success: false, input: "", page: 0, isLastPage: true, highlightedIndex: -1,
    candidates: [], status: null, ...(committed === undefined ? {} : { committed }) };
  if (root.context !== null && root.context.preedit !== "") {
    const context = root.context;
    invariant(typeof context.preedit === "string" && Number.isSafeInteger(context.page_no)
      && typeof context.is_last_page === "boolean" && Number.isSafeInteger(context.highlighted)
      && Array.isArray(context.select_labels) && Array.isArray(context.candidates), "WEB06_INDEPENDENT_RAW_CONTEXT_SHAPE");
    const candidates = context.candidates.map((candidate, index) => {
      invariant(candidate && typeof candidate === "object" && !Array.isArray(candidate)
        && typeof candidate.text === "string" && typeof candidate.comment === "string"
        && (candidate.source === undefined || typeof candidate.source === "string")
        && (context.select_labels[index] === undefined || typeof context.select_labels[index] === "string"),
      "WEB06_INDEPENDENT_RAW_CANDIDATE_SHAPE");
      return { ...(context.select_labels[index] === undefined ? {} : { label: context.select_labels[index] }),
        text: candidate.text, comment: candidate.comment,
        ...(candidate.source === undefined ? {} : { source: candidate.source }) };
    });
    return { success: true, input: context.preedit, page: context.page_no, isLastPage: context.is_last_page,
      highlightedIndex: context.highlighted, candidates, status: root.status,
      ...(committed === undefined ? {} : { committed }) };
  }
  return { success: true, input: "", page: 0, isLastPage: true, highlightedIndex: -1, candidates: [],
    status: root.status, ...(committed === undefined ? {} : { committed }) };
}

function independentPresentationProjection(value) {
  return !value ? undefined : { success: true, input: value.input, page: value.page,
    isLastPage: value.isLastPage, highlightedIndex: value.highlightedIndex, candidates: value.candidates,
    status: value.status ?? null };
}

function independentTerminalOwnerForStep(step, expectedActions) {
  const actions = expectedActions.filter((action) => action.stepId === step.id);
  const foreground = actions.filter((action) => action.background !== true);
  return foreground.length !== actions.length ? foreground[0] ?? actions[0] : foreground.at(-1);
}

function independentWorkerSpanErrors(action, minimal, full) {
  const errors = [];
  const spans = action.workerSpans ?? {};
  if (minimal) {
    if (Object.values(spans).some((span) => span !== null && span !== undefined)) {
      errors.push(`MINIMAL_RAW_WORKER_SPAN_PRESENT:${action.sequenceId}`);
    }
    if (action.persistenceRan !== undefined) {
      errors.push(`MINIMAL_RAW_PERSISTENCE_DISPOSITION_PRESENT:${action.sequenceId}`);
    }
  } else if (full) {
    for (const name of ["abi", "responseExtract", "jsonParse", "adapterTranslate", "persistence"]) {
      if (!Object.hasOwn(spans, name)) errors.push(`WORKER_SPAN_DECLARATION_MISSING:${action.sequenceId}:${name}`);
    }
    if (typeof action.persistenceRan !== "boolean") {
      errors.push(`WORKER_PERSISTENCE_DISPOSITION_MISSING:${action.sequenceId}`);
    } else if (action.persistenceRan !== Boolean(spans.persistence)) {
      errors.push(`WORKER_PERSISTENCE_SPAN_DISAGREES:${action.sequenceId}`);
    }
    const ordered = Object.entries(spans).filter(([, span]) => span !== null && span !== undefined)
      .map(([name, span]) => ({ name, ...span })).sort((left, right) => left.start - right.start);
    ordered.forEach((span, spanIndex) => {
      if (!finite(span.start) || !finite(span.end) || span.end < span.start) {
        errors.push(`WORKER_SPAN_INVALID:${action.sequenceId}:${span.name}`);
        return;
      }
      if (span.start < action.workerActionStartedAt || span.end > action.workerFinishedAt) {
        errors.push(`WORKER_SPAN_OUTSIDE_ACTION:${action.sequenceId}:${span.name}`);
      }
      if (spanIndex > 0 && span.start < ordered[spanIndex - 1].end) {
        errors.push(`WORKER_SPAN_OVERLAP:${action.sequenceId}:${ordered[spanIndex - 1].name}:${span.name}`);
      }
    });
  }
  return errors;
}

function independentInternalContentErrors(receipt, row) {
  const errors = [];
  errors.push(...independentAuxiliaryErrors(receipt));
  const expected = independentExpandScenarioExpectedTimeline(row.id);
  const minimal = ["BASE_MINIMAL", "FINAL_MINIMAL"].includes(receipt.mode);
  const full = ["BASE_FULL", "FINAL_FULL"].includes(receipt.mode);
  const responseBearing = new Set(["processKey", "stageAi", "selectCandidate", "deleteCandidate", "flipPage"]);
  for (const action of receipt.actions) {
    errors.push(...independentWorkerSpanErrors(action, minimal, full));
  }
  for (let index = 0; index < Math.min(receipt.actions.length, expected.actions.length); index += 1) {
    const action = receipt.actions[index];
    const frozen = expected.actions[index];
    const terminalStrategy = ACTION_REGISTRY[frozen.kind]?.terminalStrategy;
    if (!terminalStrategy || action.terminalKind !== terminalStrategy) errors.push(`ACTION_TERMINAL_KIND:${action.sequenceId}`);
    if (!OUTCOMES.includes(action.outcome)) errors.push(`ACTION_OUTCOME_UNKNOWN:${action.sequenceId}`);
    if (frozen.expectedOutcome === "failure") {
      if (action.outcome !== "failure" || terminalStrategy !== "lifecycle") {
        errors.push(`EXPECTED_ACTION_FAILURE_MISSING:${action.sequenceId}`);
      }
    } else if (action.outcome === "failure") {
      errors.push(`ACTION_EXPLICIT_FAILURE:${action.sequenceId}`);
    }
    if (terminalStrategy === "lifecycle" && !["barrier-completed", "failure"].includes(action.outcome)) {
      errors.push(`LIFECYCLE_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (terminalStrategy === "presentation" && action.outcome === "barrier-completed") {
      errors.push(`PRESENTATION_OUTCOME_INVALID:${action.sequenceId}`);
    }
    const step = row.steps.find((candidate) => candidate.id === frozen.stepId);
    if (step?.sample === "terminal"
      && independentTerminalOwnerForStep(step, expected.actions)?.sequenceId === frozen.sequenceId
      && !["painted", "committed", "barrier-completed", "failure"].includes(action.outcome)) {
      errors.push(`TERMINAL_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (frozen.classification === "stateful-barrier" && action.outcome === "superseded") {
      errors.push(`SUPERSEDED_BARRIER:${action.sequenceId}`);
    }
    if (frozen.supersedable === true && !["painted", "superseded"].includes(action.outcome)) {
      errors.push(`PRINTABLE_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (frozen.classification === "stateful-barrier"
      && !["painted", "committed", "barrier-completed", "failure"].includes(action.outcome)) {
      errors.push(`BARRIER_OUTCOME_INVALID:${action.sequenceId}`);
    }
    if (action.outcome === "processed-no-visual-change"
      && (frozen.classification === "stateful-barrier" || action.beforeDomDigest !== action.afterDomDigest)) {
      errors.push(`NO_VISUAL_CHANGE_INVALID:${action.sequenceId}`);
    }
    if (action.outcome === "superseded" && !frozen.supersedable) errors.push(`NONPRINTABLE_SUPERSEDED:${action.sequenceId}`);
    if (terminalStrategy === "presentation" && frozen.supersedable !== true
      && action.outcome === "processed-no-visual-change") errors.push(`INTERACTIVE_NO_VISUAL_CHANGE_FORBIDDEN:${action.sequenceId}`);
    const rawSequence = action.rawActionSequence ?? action.engineRaw?.rawActionSequence;
    if (!Array.isArray(rawSequence)) errors.push(`ACTION_RAW_SEQUENCE:${action.sequenceId}`);
    if (minimal) {
      if (action.engineRaw !== undefined) errors.push(`MINIMAL_ENGINE_RAW_PRESENT:${action.sequenceId}`);
      if (action.engineRawProof?.availability !== "not-collected"
        || action.engineRawProof?.reason !== "minimal-content-free" || action.engineRawProof?.action !== action.kind
        || action.engineRawProof?.rawFingerprint !== undefined) {
        errors.push(`MINIMAL_ENGINE_RAW_PROOF_INVALID:${action.sequenceId}`);
      }
      if (action.presentationExpected !== undefined || action.domObserved !== undefined) {
        errors.push(`MINIMAL_PRESENTATION_CONTENT_PRESENT:${action.sequenceId}`);
      }
    } else if (full) {
      const raw = action.engineRaw;
      const proof = action.engineRawProof;
      if (!proof || proof.action !== action.kind) errors.push(`ENGINE_RAW_PROOF_IDENTITY:${action.sequenceId}`);
      if (!responseBearing.has(action.kind)) {
        if (raw !== undefined || proof?.availability !== "not-applicable"
          || !["action-has-no-runtime-response", "action-failed-before-runtime-response"].includes(proof?.reason)) {
          errors.push(`ENGINE_RAW_NOT_APPLICABLE_INVALID:${action.sequenceId}`);
        }
      } else if (!raw || raw.actionKind !== action.kind || raw.compositionEpochId !== action.compositionEpochId
        || raw.supersessionSubRunId !== action.supersessionSubRunId) {
        errors.push(`ENGINE_RAW_IDENTITY:${action.sequenceId}`);
      } else if (!Array.isArray(raw.rawActionSequence)) {
        errors.push(`ENGINE_RAW_ACTION_SEQUENCE:${action.sequenceId}`);
      } else if (typeof raw.rawResponseJson !== "string" || !SHA64.test(raw.rawResponseSha256 ?? "")
        || sha256(Buffer.from(raw.rawResponseJson ?? "", "utf8")) !== raw.rawResponseSha256) {
        errors.push(`ENGINE_RAW_PREPROJECTION_BYTES:${action.sequenceId}`);
      } else {
        try {
          const projected = independentProjectRawResponse(raw.rawResponseJson);
          const committed = projected.committed;
          delete projected.committed;
          if (!exactJson(projected, independentPresentationProjection(action.presentationExpected))) {
            errors.push(`ENGINE_RAW_PROJECTION_DISAGREES:${action.sequenceId}`);
          }
          if (committed !== undefined && committed !== action.commitFingerprint?.exactCommitText) {
            errors.push(`ENGINE_RAW_COMMIT_DISAGREES:${action.sequenceId}`);
          }
        } catch {
          errors.push(`ENGINE_RAW_PREPROJECTION_JSON:${action.sequenceId}`);
        }
      }
      if (Array.isArray(action.rawActionSequence) && !exactJson(action.rawActionSequence, raw?.rawActionSequence)) {
        errors.push(`ENGINE_RAW_SEQUENCE_DISAGREES:${action.sequenceId}`);
      }
    }
    if (terminalStrategy === "presentation") {
      for (const field of ["beforeDomDigest", "adapterProjectionDigest", "presentationExpectedDigest",
        "domObservedDigest", "presentationDigest"]) {
        if (!/^[0-9a-f]{32}$/.test(action[field] ?? "")) errors.push(`PRESENTATION_DIGEST_INVALID:${action.sequenceId}:${field}`);
      }
      if (full) {
        const expectedPresentation = !action.presentationExpected
          || typeof action.presentationExpected !== "object" ? undefined : (() => {
          const { sequenceId: _sequenceId, ...rest } = action.presentationExpected;
          return rest;
        })();
        const observedPresentation = !action.domObserved
          || typeof action.domObserved !== "object" ? undefined : (() => {
          const { sequenceId: _sequenceId, ...rest } = action.domObserved;
          return rest;
        })();
        const expectedDigest = expectedPresentation === undefined
          ? undefined : independentWeb06StableDigest(expectedPresentation);
        const observedDigest = observedPresentation === undefined
          ? undefined : independentWeb06StableDigest(observedPresentation);
        if (expectedDigest !== action.presentationExpectedDigest
          || observedDigest !== action.domObservedDigest
          || action.presentationDigest !== action.domObservedDigest || action.afterDomDigest !== action.domObservedDigest) {
          errors.push(`PRESENTATION_DIGEST_RECOMPUTE:${action.sequenceId}`);
        }
        if (["painted", "committed", "barrier-completed"].includes(action.outcome)
          && (!action.presentationExpected || !action.domObserved
            || !exactJson(action.presentationExpected, action.domObserved))) {
          errors.push(`PRESENTATION_DOM_MISMATCH:${action.sequenceId}`);
        }
      }
    } else if (terminalStrategy === "lifecycle") {
      const lifecycle = action.lifecycleEffect;
      if (!lifecycle || !["listener", "engine-state", "engine-persistence", "ui-userdb-refresh",
        "ui-diagnostic-refresh", "cache-invalidation", "error"].includes(lifecycle.effect)
        || !/^[0-9a-f]{32}$/.test(lifecycle.effectDigest ?? "")
        || !/^[0-9a-f]{32}$/.test(lifecycle.workerEffectDigest ?? "")
        || (lifecycle.mainEffectDigest !== undefined && !/^[0-9a-f]{32}$/.test(lifecycle.mainEffectDigest))
        || !Number.isSafeInteger(lifecycle.listenerEffectCount) || lifecycle.listenerEffectCount < 0
        || typeof lifecycle.persistenceCompleted !== "boolean") {
        errors.push(`LIFECYCLE_EFFECT_INVALID:${action.sequenceId}`);
      }
      if (action.presentationExpected !== undefined || action.domObserved !== undefined) {
        errors.push(`LIFECYCLE_PRESENTATION_CONTENT_PRESENT:${action.sequenceId}`);
      }
    }
    if (action.outcome === "committed") {
      const commit = action.commitFingerprint;
      if (!commit || typeof commit.exactCommitText !== "string" || typeof commit.textareaValue !== "string"
        || !Number.isInteger(commit.selectionStart) || !Number.isInteger(commit.selectionEnd)
        || commit.visibleComposition !== false) errors.push(`COMMIT_FINGERPRINT_INVALID:${action.sequenceId}`);
    }
  }
  return errors;
}

function independentEventClockErrors(receipt, common) {
  const setupErrors = [];
  const behaviorErrors = [];
  const probes = receipt.eventClockSegments && typeof receipt.eventClockSegments === "object"
    ? Object.values(receipt.eventClockSegments) : [receipt.eventClockProbe];
  if (!probes.length || probes.some((probe) => !finite(probe?.beforeDispatchAt)
    || !finite(probe?.eventTimestamp) || !finite(probe?.afterDispatchAt))) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_PROBE");
  } else if (probes.some((probe) => probe.eventTimestamp < probe.beforeDispatchAt
    || probe.eventTimestamp > probe.afterDispatchAt)) {
    setupErrors.push("SETUP_INVALID_EVENT_CLOCK_ORIGIN");
  }
  const previousByRealm = new Map();
  for (const event of receipt.events ?? []) {
    const realm = event.pageInstanceId ?? "single-page";
    const previous = previousByRealm.get(realm) ?? -Infinity;
    if (!finite(event.eventTimestamp) || event.eventTimestamp < 0 || event.normalizedEventAt !== event.eventTimestamp) {
      setupErrors.push(`SETUP_INVALID_EVENT_TIMESTAMP:${event.eventSequenceId}`);
      continue;
    }
    if (event.eventTimestamp < previous) {
      setupErrors.push(`SETUP_DECREASING_EVENT_TIMESTAMP:${event.eventSequenceId}`);
    }
    previousByRealm.set(realm, event.eventTimestamp);
    const observed = common ? event.sentinelObservedAt : event.eventDeliveredAt;
    if (!finite(observed) || observed < event.normalizedEventAt) {
      behaviorErrors.push(`${common ? "COMMON_" : ""}EVENT_SAME_REALM_ORDER:${event.eventSequenceId}`);
    }
  }
  return { setupErrors, behaviorErrors };
}

function independentSameRealmActionErrors(actions) {
  const errors = [];
  for (const action of actions ?? []) {
    const eventTime = action.event?.eventDeliveredAt ?? action.eventDeliveredAt;
    const ordered = [
      ["actionEnqueuedAt", eventTime],
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
      if (!finite(action[field]) || action[field] < previous) {
        errors.push(`ACTION_SAME_REALM_ORDER:${action.sequenceId}:${field}`);
      }
    }
    if (action.outcome === "painted") {
      for (const field of ["stateUpdateScheduledAt", "stateCommittedAt", "paintObservedAt"]) {
        if (!finite(action[field])) errors.push(`PAINT_CHAIN_MISSING:${action.sequenceId}:${field}`);
      }
      if (finite(action.stateCommittedAt) && finite(action.paintObservedAt)
        && action.paintObservedAt < action.stateCommittedAt) {
        errors.push(`ACTION_SAME_REALM_ORDER:${action.sequenceId}:paintObservedAt`);
      }
    }
    if (!finite(action.event?.actualDriverDispatchAt) || !finite(action.event?.requestedDriverDispatchAt)
      || action.driverDispatchAt !== action.event?.actualDriverDispatchAt) {
      errors.push(`ACTION_DRIVER_IDENTITY:${action.sequenceId}`);
    }
    if (!Number.isInteger(action.mainQueueDepth) || action.mainQueueDepth < 0) {
      errors.push(`ACTION_QUEUE_DEPTH:${action.sequenceId}`);
    }
    if (!Number.isInteger(action.workerDispatchDepth) || action.workerDispatchDepth < 0) {
      errors.push(`ACTION_WORKER_DEPTH:${action.sequenceId}`);
    }
  }
  return errors;
}

function independentTimelineResidualErrors(actions) {
  const errors = [];
  for (const action of actions ?? []) {
    if (action.outcome !== "painted" || !finite(action.paintObservedAt)) continue;
    if (!finite(action.metrics?.timelineResidualMs)) {
      errors.push(`TIMELINE_RESIDUAL_NONFINITE:${action.sequenceId}`);
    } else if (Math.abs(action.metrics.timelineResidualMs)
      > WEB06_THRESHOLDS.metric.timelineResidualAbsoluteMaxMs) {
      errors.push(`TIMELINE_RESIDUAL:${action.sequenceId}:${action.metrics.timelineResidualMs}`);
    }
  }
  return errors;
}

/** Verifier-owned action timing audit, exported only for non-circular contract tests. */
export function independentlyValidateActionTiming(actions) {
  const errors = [
    ...independentSameRealmActionErrors(actions),
    ...independentTimelineResidualErrors(actions),
  ];
  return { pass: errors.length === 0, errors };
}

function independentStrictPrefix(left, right) {
  return Array.isArray(left) && Array.isArray(right) && left.length < right.length
    && left.every((value, index) => exactJson(value, right[index]));
}

function independentRawActionSequence(action) {
  return action.rawActionSequence ?? action.engineRaw?.rawActionSequence;
}

function independentLogicalInput(action) {
  return action.logicalInput ?? action.engineRaw?.logicalInput ?? action.presentationExpected?.input;
}

function independentlyResolveSupersession(actions, action, errors) {
  const target = actions.find((candidate) => candidate.sequenceId === action.supersededBySequenceId);
  if (!target || target.sequenceId <= action.sequenceId) {
    errors.push(`SUPERSESSION_ORPHAN_OR_BACKWARD:${action.sequenceId}`);
    return undefined;
  }
  if (target.compositionEpochId !== action.compositionEpochId
    || target.supersessionSubRunId !== action.supersessionSubRunId) {
    errors.push(`SUPERSESSION_CROSSES_BOUNDARY:${action.sequenceId}`);
  }
  if (target.outcome !== "painted" || !finite(target.paintObservedAt)) {
    errors.push(`SUPERSESSION_TARGET_NOT_PAINTED:${action.sequenceId}`);
  }
  if (target.classification !== "native-key" || target.supersedable !== true) {
    errors.push(`SUPERSESSION_TARGET_IS_BARRIER:${action.sequenceId}`);
  }
  if (!independentStrictPrefix(independentRawActionSequence(action), independentRawActionSequence(target))
    || !String(independentLogicalInput(target) ?? "")
      .startsWith(String(independentLogicalInput(action) ?? ""))) {
    errors.push(`SUPERSESSION_NON_PREFIX:${action.sequenceId}`);
  }
  const between = actions.filter((candidate) => candidate.sequenceId > action.sequenceId
    && candidate.sequenceId < target.sequenceId);
  if (between.some((candidate) => candidate.classification !== "native-key"
    || candidate.supersedable !== true || candidate.outcome === "failure")) {
    errors.push(`SUPERSESSION_SPANS_BARRIER_OR_FAILURE:${action.sequenceId}`);
  }
  const earliestPaint = actions.find((candidate) => candidate.sequenceId > action.sequenceId
    && candidate.compositionEpochId === action.compositionEpochId
    && candidate.supersessionSubRunId === action.supersessionSubRunId
    && candidate.outcome === "painted"
    && independentStrictPrefix(independentRawActionSequence(action), independentRawActionSequence(candidate)));
  if (earliestPaint && earliestPaint.sequenceId !== target.sequenceId) {
    errors.push(`SUPERSESSION_NOT_EARLIEST_COVERING_PAINT:${action.sequenceId}`);
  }
  const lag = target.sequenceId - action.sequenceId;
  if (lag > WEB06_THRESHOLDS.sustained.supersessionSequenceLag.max) {
    errors.push(`SUPERSESSION_LAG:${action.sequenceId}:${lag}`);
  }
  return target;
}

/** Verifier-owned supersession reconstruction, exported only for non-circular contract tests. */
export function independentlyValidateSupersessionGraph(actions) {
  const errors = [];
  for (const action of actions ?? []) {
    if (action.outcome === "superseded") independentlyResolveSupersession(actions, action, errors);
  }
  return { pass: errors.length === 0, errors };
}

function independentBurstFacts(receipt, row) {
  const behaviorErrors = [];
  const thresholdViolations = [];
  const expected = row.steps.filter((step) => step.declaredBurstPauseAfter === true);
  const recoveries = receipt.burstRecoveries ?? [];
  if (recoveries.length !== expected.length) behaviorErrors.push(`BURST_RECOVERY_COUNT:${recoveries.length}!=${expected.length}`);
  for (let index = 0; index < Math.min(recoveries.length, expected.length); index += 1) {
    const recovery = recoveries[index];
    const step = expected[index];
    const event = (receipt.events ?? []).find((candidate) => candidate.stepId === step.id && candidate.type === "keydown");
    if (!event || recovery.afterStepId !== step.id || !finite(recovery.latestPaintAt)
      || recovery.latestPaintAt < event.normalizedEventAt) {
      behaviorErrors.push(`BURST_RECOVERY_IDENTITY:${step.id}`);
      continue;
    }
    if (recovery.latestPaintAt - event.normalizedEventAt > 67) {
      thresholdViolations.push(`burst-recovery:${step.id}`);
    }
    const idle = recovery.idleSnapshot;
    if (!idle || idle.queueDepth !== 0 || idle.runningActionId !== null || idle.pendingFanoutActions !== 0
      || idle.pendingTerminalActions !== 0 || idle.pendingSentinelCaptures !== 0
      || !Number.isSafeInteger(recovery.expectedCompletedActionCount)
      || recovery.expectedCompletedActionCount < 0
      || !Number.isSafeInteger(idle.completedActionCount) || idle.completedActionCount < 0
      || idle.completedActionCount !== recovery.expectedCompletedActionCount) {
      behaviorErrors.push(`BURST_IDLE_SNAPSHOT_INVALID:${step.id}`);
    }
  }
  return { behaviorErrors, thresholdViolations };
}

/** Verifier-owned burst recovery audit, exported only for non-circular contract tests. */
export function independentlyVerifyBurstRecoveries(receipt, row) {
  return independentBurstFacts(receipt, row);
}

function cadenceFacts(receipt, row) {
  const counts = { total: 0, inRange: 0, tooShort: 0, tooLong: 0, delayedHost: 0 };
  const errors = [];
  const expected = row.steps.filter((step) => finite(step.nominalGapMs));
  const actual = receipt.cadenceGaps ?? [];
  if (actual.length !== expected.length) errors.push(`CADENCE_GAP_COUNT:${actual.length}!=${expected.length}`);
  for (const [index, gap] of actual.entries()) {
    counts.total += 1;
    if (gap.stepId !== expected[index]?.id || gap.nominalGapMs !== expected[index]?.nominalGapMs) {
      errors.push(`CADENCE_GAP_IDENTITY:${index + 1}`);
    }
    if (gap.rebasedAfterLateHost === true) counts.delayedHost += 1;
    const range = cadenceRange(gap.nominalGapMs);
    if (!finite(gap.actualDriverGapMs)) {
      errors.push("SETUP_NONFINITE_CADENCE");
      counts.inRange += 1;
      continue;
    }
    if (gap.actualDriverGapMs < range.minMs) counts.tooShort += 1;
    else if (gap.actualDriverGapMs > range.maxMs) counts.tooLong += 1;
    else counts.inRange += 1;
  }
  const cadence = row.cadence === "same-task-pressure" ? "NOT_APPLICABLE"
    : counts.tooShort > 0 ? "TOO_SHORT"
      : counts.tooLong > 0 ? "TOO_LONG" : "IN_RANGE";
  return { counts, cadence, errors };
}

function independentCadenceCountsProjection(receipt) {
  const counts = { total: 0, inRange: 0, tooShort: 0, tooLong: 0, delayedHost: 0 };
  for (const gap of receipt.cadenceGaps ?? []) {
    counts.total += 1;
    if (gap.rebasedAfterLateHost === true) counts.delayedHost += 1;
    const range = cadenceRange(gap.nominalGapMs);
    if (gap.actualDriverGapMs < range.minMs) counts.tooShort += 1;
    else if (gap.actualDriverGapMs > range.maxMs) counts.tooLong += 1;
    else counts.inRange += 1;
  }
  return counts;
}

function exchange(raw, kind) {
  const names = kind === "driver" ? ["d0", "m1", "m2", "d3"] : ["m0", "w1", "w2", "m3"];
  for (const name of names) {
    invariant(raw && finite(raw[name]), `SETUP_INVALID_CLOCK_CALIBRATION:${name}`);
  }
  const [a, b, c, d] = names.map((name) => raw[name]);
  const offset = ((b - a) + (c - d)) / 2;
  const netRtt = (d - a) - (c - b);
  invariant(finite(netRtt) && netRtt >= 0,
    `SETUP_INVALID_CLOCK_CALIBRATION:${kind}-net-rtt`);
  return { midpoint: (a + d) / 2, offset, uncertainty: netRtt / 2 };
}

function calibration(raw, kind) {
  const count = WEB06_THRESHOLDS.calibration.exchangesPerBoundary;
  invariant(Array.isArray(raw?.pre) && Array.isArray(raw?.post)
    && raw.pre.length === count && raw.post.length === count,
    "SETUP_INVALID_CLOCK_CALIBRATION:exchange-count");
  const select = (rows) => rows.map((row) => exchange(row, kind))
    .sort((left, right) => left.uncertainty - right.uncertainty || left.midpoint - right.midpoint)[0];
  const pre = select(raw.pre);
  const post = select(raw.post);
  invariant(pre.uncertainty <= WEB06_THRESHOLDS.calibration.uncertaintyMaxMs
    && post.uncertainty <= WEB06_THRESHOLDS.calibration.uncertaintyMaxMs,
  "SETUP_INVALID_CLOCK_CALIBRATION:uncertainty");
  invariant(Math.abs(post.offset - pre.offset) <= WEB06_THRESHOLDS.calibration.offsetDriftMaxMs,
    "SETUP_INVALID_CLOCK_CALIBRATION:offset-drift");
  invariant(post.midpoint > pre.midpoint, "SETUP_INVALID_CLOCK_CALIBRATION:boundary-order");
  return { pre, post };
}

function clockPoint(value, calibrationValue, referenceAt, direction) {
  invariant(finite(referenceAt), "SETUP_INVALID_CLOCK_CALIBRATION:interpolation-time");
  const { pre, post } = calibrationValue;
  const fraction = (referenceAt - pre.midpoint) / (post.midpoint - pre.midpoint);
  invariant(fraction >= 0 && fraction <= 1, "SETUP_INVALID_CLOCK_CALIBRATION:outside-boundary");
  const offset = pre.offset + (post.offset - pre.offset) * fraction;
  const uncertainty = pre.uncertainty + (post.uncertainty - pre.uncertainty) * fraction;
  invariant(uncertainty >= 0 && uncertainty <= WEB06_THRESHOLDS.calibration.uncertaintyMaxMs,
    "SETUP_INVALID_CLOCK_CALIBRATION:interpolated-uncertainty");
  return { correctedAt: value + direction * offset, uncertainty };
}

function independentCalibrations(receipt) {
  if (receipt.scenarioId !== "learned-row") return {
    driver: calibration(receipt.calibration?.driver, "driver"),
    worker: calibration(receipt.calibration?.worker, "worker"),
  };
  const pages = [...new Set((receipt.interactionWindows ?? []).map((window) => window.pageInstanceId))];
  invariant(pages.length === 2 && receipt.calibrationSegments?.preReload
    && receipt.calibrationSegments?.postReload,
  "SETUP_LEARNED_CALIBRATION_SEGMENTS_INVALID");
  const segment = (raw) => ({ driver: calibration(raw?.driver, "driver"), worker: calibration(raw?.worker, "worker") });
  const byPageInstance = {
    [pages[0]]: segment(receipt.calibrationSegments?.preReload),
    [pages[1]]: segment(receipt.calibrationSegments?.postReload),
  };
  return { ...byPageInstance[pages[0]], byPageInstance };
}

function independentCrossContextOrderingErrors(action, event, corrected) {
  const errors = [];
  if (action.workerMessageReceivedAt > action.workerActionStartedAt
    || action.workerActionStartedAt > action.workerFinishedAt) {
    errors.push(`SETUP_INVALID_CLOCK_CALIBRATION:worker-same-realm-order:${action.sequenceId}`);
  }
  if (action.workerSentAt > corrected.receive.correctedAt - corrected.receive.uncertainty) {
    errors.push(`SETUP_INVALID_CLOCK_CALIBRATION:send-receive-order:${action.sequenceId}`);
  }
  if (action.actionEnqueuedAt > corrected.start.correctedAt - corrected.start.uncertainty) {
    errors.push(`SETUP_INVALID_CLOCK_CALIBRATION:enqueue-start-order:${action.sequenceId}`);
  }
  if (corrected.finish.correctedAt + corrected.finish.uncertainty > action.mainResponseReceivedAt) {
    errors.push(`SETUP_INVALID_CLOCK_CALIBRATION:finish-response-order:${action.sequenceId}`);
  }
  if (corrected.driver
    && corrected.driver.correctedAt + corrected.driver.uncertainty > event.normalizedEventAt) {
    errors.push(`SETUP_INVALID_CLOCK_CALIBRATION:driver-event-order:${action.sequenceId}`);
  }
  return errors;
}

function independentCalibrationForPage(calibrations, pageInstanceId) {
  const lane = calibrations?.byPageInstance?.[pageInstanceId] ?? calibrations;
  return lane?.driver ?? lane;
}

function deriveInternal(receipt) {
  const row = hasOwn(SCENARIO_REGISTRY, receipt.scenarioId)
    ? SCENARIO_REGISTRY[receipt.scenarioId]
    : undefined;
  const setupErrors = [];
  const behaviorErrors = [];
  setupErrors.push(...independentIdentityErrors(receipt));
  if (!["BASE_MINIMAL", "BASE_FULL", "FINAL_MINIMAL", "FINAL_FULL"].includes(receipt.mode)) {
    setupErrors.push(receipt.mode === "PRODUCT"
      ? "SETUP_PRODUCT_INTERNAL_RECEIPT_FORBIDDEN"
      : "SETUP_INTERNAL_MODE_INVALID");
  }
  behaviorErrors.push(...independentReceiptPrivacyErrors(receipt));
  if (!row) {
    setupErrors.push("SETUP_UNKNOWN_SCENARIO");
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
  const missingArrays = independentMissingArrayErrors(receipt, [
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
  ], "SETUP_INTERNAL");
  setupErrors.push(...missingArrays);
  if (missingArrays.length > 0) {
    return {
      status: "SETUP_INVALID",
      setupErrors,
      behaviorErrors,
      thresholdViolations: [],
      metrics: { actions: [], covering: [], terminal: [] },
    };
  }
  setupErrors.push(...independentOverflowErrors(receipt));
  const protocolDisposition = independentMeasurementProtocolDisposition(
    receipt.measurementProtocolBlockers,
  );
  setupErrors.push(...protocolDisposition.setupErrors);
  const contentErrors = independentInternalContentErrors(receipt, row);
  setupErrors.push(...contentErrors.filter((error) => error.startsWith("SETUP_")));
  behaviorErrors.push(...contentErrors.filter((error) => !error.startsWith("SETUP_")));
  behaviorErrors.push(...independentTimelineErrors(receipt, row));
  const eventClock = independentEventClockErrors(receipt, false);
  setupErrors.push(...eventClock.setupErrors);
  behaviorErrors.push(...eventClock.behaviorErrors);
  let calibrations;
  try {
    calibrations = independentCalibrations(receipt);
  } catch (error) {
    setupErrors.push(error instanceof Error ? error.message : "WEB06_INDEPENDENT_CLOCK_INVALID");
  }

  const eventById = new Map((receipt.events ?? []).map((event) => [event.eventSequenceId, event]));
  const derived = [];
  let actionCalibrationFailed = false;
  const rawActionsWithEvents = (receipt.actions ?? []).flatMap((action) => {
    const event = eventById.get(action.eventSequenceId ?? action.causedByEventSequenceId);
    return event ? [{ ...action, event }] : [];
  });
  behaviorErrors.push(...independentSameRealmActionErrors(rawActionsWithEvents));
  if (calibrations) {
    for (const action of receipt.actions ?? []) {
      const event = eventById.get(action.eventSequenceId ?? action.causedByEventSequenceId);
      if (!event) {
        actionCalibrationFailed = true;
        continue;
      }
      const reference = (action.workerSentAt + action.mainResponseReceivedAt) / 2;
      const actionCalibrations = calibrations.byPageInstance?.[event.pageInstanceId] ?? calibrations;
      let receive;
      let start;
      let finish;
      let driver;
      try {
        receive = clockPoint(action.workerMessageReceivedAt, actionCalibrations.worker, reference, -1);
        start = clockPoint(action.workerActionStartedAt, actionCalibrations.worker, reference, -1);
        finish = clockPoint(action.workerFinishedAt, actionCalibrations.worker, reference, -1);
        driver = action.driverDispatchAt === undefined
          ? null
          : clockPoint(action.driverDispatchAt, actionCalibrations.driver, event.normalizedEventAt, 1);
      } catch (error) {
        setupErrors.push(`${error instanceof Error ? error.message : "WEB06_INDEPENDENT_CLOCK_ACTION"}:${action.sequenceId}`);
        actionCalibrationFailed = true;
        continue;
      }
      setupErrors.push(...independentCrossContextOrderingErrors(
        action,
        event,
        { receive, start, finish, driver },
      ));
      const metrics = {
        eventDeliveryMs: event.eventDeliveredAt - event.normalizedEventAt,
        eventHandlerEnqueueMs: action.actionEnqueuedAt - event.eventDeliveredAt,
        mainQueueWaitMs: action.workerSentAt - action.actionEnqueuedAt,
        workerMessageDeliveryMs: receive.correctedAt - action.workerSentAt,
        workerPreActionWaitMs: start.correctedAt - receive.correctedAt,
        workerDispatchWaitMs: start.correctedAt - action.workerSentAt,
        preServiceWaitMs: start.correctedAt - action.actionEnqueuedAt,
        preServiceWaitUpperBoundMs: start.correctedAt - action.actionEnqueuedAt + start.uncertainty,
        workerProcessMs: finish.correctedAt - start.correctedAt,
        workerRoundtripMs: action.mainResponseReceivedAt - action.workerSentAt,
        mainResponseDispatchMs: action.responseMappingStartedAt - action.mainResponseReceivedAt,
        responseMappingMs: action.responseMappingFinishedAt - action.responseMappingStartedAt,
      };
      for (const [name, span] of Object.entries(action.workerSpans ?? {})) {
        if (span) metrics[`worker${name[0].toUpperCase()}${name.slice(1)}Ms`] = span.end - span.start;
      }
      if (action.stateUpdateScheduledAt !== undefined) {
        metrics.stateScheduleMs = action.stateUpdateScheduledAt - action.responseMappingFinishedAt;
        metrics.reactCommitMs = action.stateCommittedAt - action.stateUpdateScheduledAt;
      }
      if (finite(action.paintObservedAt)) {
        metrics.paintProxyMs = action.paintObservedAt - action.stateCommittedAt;
        metrics.eventToCurrentPaintMs = action.paintObservedAt - event.normalizedEventAt;
        metrics.handlerToCurrentPaintMs = action.paintObservedAt - event.eventDeliveredAt;
        const sum = metrics.eventDeliveryMs + metrics.eventHandlerEnqueueMs + metrics.mainQueueWaitMs
          + metrics.workerRoundtripMs + metrics.mainResponseDispatchMs + metrics.responseMappingMs
          + metrics.stateScheduleMs + metrics.reactCommitMs + metrics.paintProxyMs;
        metrics.timelineResidualMs = metrics.eventToCurrentPaintMs - sum;
      }
      if (finite(action.terminalObservedAt)) {
        metrics.eventToTerminalObservationMs = action.terminalObservedAt - event.normalizedEventAt;
        if (driver) {
          metrics.driverDispatchToTerminalUpperBoundMs = action.terminalObservedAt
            - driver.correctedAt + driver.uncertainty;
        }
      }
      derived.push({ ...action, event, driver, corrected: { receive, start, finish, driver }, metrics });
    }
    for (const action of derived) {
      let coveringPaintAt;
      if (action.outcome === "painted") coveringPaintAt = action.paintObservedAt;
      if (action.outcome === "superseded") {
        const target = independentlyResolveSupersession(derived, action, behaviorErrors);
        if (target) {
          coveringPaintAt = target.paintObservedAt;
          action.metrics.supersessionSequenceLag = target.sequenceId - action.sequenceId;
          action.metrics.supersessionTimeMs = target.paintObservedAt - action.event.normalizedEventAt;
        }
      }
      if (finite(coveringPaintAt)) {
        action.metrics.eventToCoveringPaintMs = coveringPaintAt - action.event.normalizedEventAt;
        if (action.driver) {
          action.metrics.driverDispatchToCoveringPaintUpperBoundMs = coveringPaintAt
            - action.driver.correctedAt + action.driver.uncertainty;
        }
      }
    }
  }

  const covering = [];
  const terminal = [];
  const thresholdViolations = [...protocolDisposition.thresholdViolations];
  const timingComplete = calibrations !== undefined
    && !actionCalibrationFailed
    && derived.length === (receipt.actions ?? []).length
    && !setupErrors.some(independentClockCalibrationError);
  behaviorErrors.push(...independentTimelineResidualErrors(derived));
  if (timingComplete) {
    for (const step of row.steps) {
      const actions = derived.filter((action) => action.stepId === step.id);
      if (step.sample === "covering") {
        const metrics = actions[0]?.metrics;
        if (finite(metrics?.eventToCoveringPaintMs) && finite(metrics?.driverDispatchToCoveringPaintUpperBoundMs)) {
          covering.push(metrics);
        } else behaviorErrors.push(`COVERING_SAMPLE_MISSING:${step.id}`);
      }
      if (step.sample === "terminal") {
        const frozenOwner = independentTerminalOwnerForStep(step, independentExpandScenarioExpectedTimeline(row.id).actions);
        const owner = actions.find((action) => action.sequenceId === frozenOwner?.sequenceId);
        if (finite(owner?.metrics?.eventToTerminalObservationMs)
          && finite(owner?.metrics?.driverDispatchToTerminalUpperBoundMs)) {
          terminal.push({ ...owner.metrics, stressDeadline: frozenOwner?.stressDeadline === true });
        } else behaviorErrors.push(`TERMINAL_SAMPLE_MISSING:${step.id}`);
      }
    }
    if (covering.length !== row.expectedCoveringSamples) behaviorErrors.push(`COVERING_COUNT:${covering.length}`);
    if (terminal.length !== row.expectedTerminalSamples) behaviorErrors.push(`TERMINAL_COUNT:${terminal.length}`);
    thresholdViolations.push(...thresholdViolationsFor(row, covering, terminal));
    behaviorErrors.push(...verifyIndependentPressureProofs(receipt, row, derived));
  }
  const burst = independentBurstFacts(receipt, row);
  behaviorErrors.push(...burst.behaviorErrors);
  thresholdViolations.push(...burst.thresholdViolations);
  const frame = frameFacts(receipt, row, setupErrors, behaviorErrors, calibrations);
  const cadence = cadenceFacts(receipt, row);
  setupErrors.push(...cadence.errors.filter((error) => error.startsWith("SETUP_")));
  behaviorErrors.push(...cadence.errors.filter((error) => !error.startsWith("SETUP_")));
  const status = independentStatus({ setupErrors, behaviorErrors, thresholdViolations,
    frameRed: frame.frameRed, longTaskRed: frame.longTaskRed, cadence: cadence.cadence });
  return { status, setupErrors, behaviorErrors, thresholdViolations, frameRed: frame.frameRed,
    longTaskRed: frame.longTaskRed, cadence: cadence.cadence, cadenceCounts: cadence.counts,
    metrics: { actions: derived, covering, terminal } };
}

export function verifyIndependentPressureProofs(receipt, row, derivedActions) {
  if (!row.overlapRequired) return [];
  const errors = [];
  const expectedPairs = WEB06_PRESSURE_PAIR_REGISTRY[row.id] ?? [];
  const proofs = receipt.pressureProofs ?? [];
  if (proofs.length !== expectedPairs.length) errors.push("FIFO_PRESSURE_PROOF_COUNT");
  for (let index = 0; index < expectedPairs.length; index += 1) {
    const expected = expectedPairs[index];
    const proof = proofs[index];
    const earlier = derivedActions.find((action) => action.stepId === expected.earlierStepId
      && action.originKind !== "background");
    const later = derivedActions.find((action) => action.stepId === expected.laterStepId
      && action.originKind !== "background");
    const earlierObservedAt = earlier?.outcome === "painted" ? earlier.paintObservedAt
      : ["committed", "barrier-completed", "failure"].includes(earlier?.outcome)
        ? earlier?.terminalObservedAt : undefined;
    const laterAppliedAt = later?.terminalKind === "presentation" ? later?.stateCommittedAt : later?.terminalObservedAt;
    const laterStartLowerBound = later?.corrected?.start?.correctedAt - later?.corrected?.start?.uncertainty;
    const earlierEvent = earlier?.event;
    const laterEvent = later?.event;
    if (!proof || proof.subcase !== expected.subcase
      || proof.earlierStepId !== expected.earlierStepId || proof.laterStepId !== expected.laterStepId
      || proof.earlierSequenceId !== earlier?.sequenceId || proof.laterSequenceId !== later?.sequenceId
      || proof.dispatchContract !== "single-page-task-no-await"
      || !finite(earlierEvent?.actualDriverDispatchAt) || earlierEvent.actualDriverDispatchAt !== laterEvent?.actualDriverDispatchAt
      || !finite(earlier?.workerSentAt) || !finite(later?.actionEnqueuedAt) || !finite(earlier?.mainResponseReceivedAt)
      || !(earlier.workerSentAt <= later.actionEnqueuedAt && later.actionEnqueuedAt < earlier.mainResponseReceivedAt)
      || !Number.isSafeInteger(later?.mainQueueDepth) || later.mainQueueDepth < 1
      || !finite(laterStartLowerBound) || earlier.mainResponseReceivedAt > laterStartLowerBound
      || !finite(earlierObservedAt) || !finite(laterAppliedAt) || earlierObservedAt > laterAppliedAt) {
      errors.push(`FIFO_PRESSURE_NOT_PROVED:${expected.subcase}`);
    }
  }
  if (row.id === "extended-scheduler-barriers") {
    const failed = derivedActions.find((action) => action.stepId === "extended-error-target");
    const recovery = derivedActions.find((action) => action.stepId === "extended-error-reset");
    if (failed?.outcome !== "failure" || !recovery || recovery.outcome === "failure"
      || recovery.sequenceId <= failed.sequenceId || recovery.actionEnqueuedAt < failed.terminalObservedAt) {
      errors.push("EXPECTED_ERROR_BOUNDARY_RECOVERY_NOT_PROVED");
    }
  }
  return errors;
}

function deriveCommon(receipt) {
  const row = hasOwn(SCENARIO_REGISTRY, receipt.scenarioId)
    ? SCENARIO_REGISTRY[receipt.scenarioId]
    : undefined;
  const setupErrors = [];
  const behaviorErrors = [];
  setupErrors.push(...independentIdentityErrors(receipt));
  behaviorErrors.push(...independentReceiptPrivacyErrors(receipt));
  if (!row) {
    setupErrors.push("SETUP_UNKNOWN_SCENARIO");
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
  const missingArrays = independentMissingArrayErrors(receipt, [
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
  ], "SETUP_COMMON");
  setupErrors.push(...missingArrays);
  if (missingArrays.length > 0) {
    return {
      status: "SETUP_INVALID",
      setupErrors,
      behaviorErrors,
      thresholdViolations: [],
      metrics: { samples: [] },
    };
  }
  setupErrors.push(...independentOverflowErrors(receipt));
  const protocolDisposition = independentMeasurementProtocolDisposition(
    receipt.measurementProtocolBlockers,
  );
  setupErrors.push(...protocolDisposition.setupErrors);
  const endpointErrors = independentCommonEndpointErrors(receipt, row);
  setupErrors.push(...endpointErrors.filter((error) => error.startsWith("SETUP_")));
  behaviorErrors.push(...endpointErrors.filter((error) => !error.startsWith("SETUP_")));
  const eventClock = independentEventClockErrors(receipt, true);
  setupErrors.push(...eventClock.setupErrors);
  behaviorErrors.push(...eventClock.behaviorErrors);
  const samples = receipt.commonSamples ?? [];
  if ((receipt.events ?? []).length !== row.expectedDomEventCount) {
    behaviorErrors.push(`COMMON_EVENT_COUNT:${receipt.events?.length ?? 0}!=${row.expectedDomEventCount}`);
  }
  const commonEventValues = (receipt.events ?? []).map((event) => event.eventSequenceId);
  if (new Set(commonEventValues).size !== commonEventValues.length) behaviorErrors.push("COMMON_EVENT_DUPLICATE_ID");
  const invalidCommonEventIndex = commonEventValues.findIndex((value, index) => value !== index + 1);
  if (invalidCommonEventIndex >= 0) {
    behaviorErrors.push(`COMMON_EVENT_${commonEventValues.includes(invalidCommonEventIndex + 1)
      ? "REORDERED" : "MISSING"}_ID`);
  }
  const expectedTimeline = independentExpandScenarioExpectedTimeline(row.id);
  for (let index = 0; index < Math.min(receipt.events?.length ?? 0, expectedTimeline.events.length); index += 1) {
    const actual = receipt.events[index];
    const frozen = expectedTimeline.events[index];
    if (actual.stepId !== frozen.stepId || actual.type !== frozen.type) {
      behaviorErrors.push(`COMMON_EVENT_REORDERED:${index + 1}`);
    }
    if (actual.key !== frozen.key || actual.code !== frozen.code) {
      behaviorErrors.push(`COMMON_EVENT_IDENTITY:${index + 1}`);
    }
  }
  let driverCalibration;
  try {
    if (receipt.scenarioId === "learned-row") {
      const pages = [...new Set((receipt.interactionWindows ?? []).map((window) => window.pageInstanceId))];
      invariant(pages.length === 2, "SETUP_LEARNED_CALIBRATION_SEGMENTS_INVALID");
      const pre = calibration(receipt.calibrationSegments?.preReload?.driver, "driver");
      const post = calibration(receipt.calibrationSegments?.postReload?.driver, "driver");
      driverCalibration = { ...pre, byPageInstance: { [pages[0]]: pre, [pages[1]]: post } };
    } else driverCalibration = calibration(receipt.calibration?.driver, "driver");
  } catch (error) {
    setupErrors.push(error instanceof Error ? error.message : "WEB06_INDEPENDENT_CLOCK_INVALID");
  }
  const events = new Map(receipt.events.map((event) => [event.eventSequenceId, event]));
  const frozenSamples = independentExpectedCommonSamples(row);
  const metrics = [];
  if (driverCalibration) {
    for (let index = 0; index < Math.min(samples.length, frozenSamples.length); index += 1) {
      const sample = samples[index];
      const frozen = frozenSamples[index];
      const event = events.get(frozen.eventSequenceId);
      if (!event || !finite(sample.observedAt) || sample.observedAt < event.normalizedEventAt) continue;
      if (!finite(event.actualDriverDispatchAt) || !finite(event.requestedDriverDispatchAt)) continue;
      let corrected;
      try {
        corrected = clockPoint(event.actualDriverDispatchAt,
          driverCalibration.byPageInstance?.[event.pageInstanceId] ?? driverCalibration,
          event.normalizedEventAt, 1);
      } catch (error) {
        setupErrors.push(`${error instanceof Error ? error.message : "WEB06_INDEPENDENT_CLOCK_ACTION"}:${frozen.stepId}`);
        continue;
      }
      if (corrected.correctedAt + corrected.uncertainty > event.normalizedEventAt) {
        setupErrors.push(`SETUP_INVALID_CLOCK_CALIBRATION:driver-event-order:${frozen.stepId}`);
      }
      metrics.push({
        ...frozen,
        eventToObservationMs: sample.observedAt - event.normalizedEventAt,
        driverDispatchToObservationUpperBoundMs: sample.observedAt - corrected.correctedAt + corrected.uncertainty,
      });
    }
  }
  const covering = metrics.filter((sample) => sample.sampleKind === "covering");
  const terminal = metrics.filter((sample) => sample.sampleKind === "terminal");
  const timingComplete = driverCalibration !== undefined
    && metrics.length === row.expectedCoveringSamples + row.expectedTerminalSamples
    && !setupErrors.some(independentClockCalibrationError);
  if (timingComplete && covering.length !== row.expectedCoveringSamples) {
    behaviorErrors.push(`COMMON_COVERING_COUNT:${covering.length}!=${row.expectedCoveringSamples}`);
  }
  if (timingComplete && terminal.length !== row.expectedTerminalSamples) {
    behaviorErrors.push(`COMMON_TERMINAL_COUNT:${terminal.length}!=${row.expectedTerminalSamples}`);
  }
  const thresholdViolations = [...protocolDisposition.thresholdViolations];
  const binding = row.binding === true || row.binding === "branch-b-only";
  if (binding && timingComplete) {
    for (const sample of covering) {
      if (sample.eventToObservationMs > WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs.max) {
        thresholdViolations.push("common-covering-max");
      }
      if (sample.driverDispatchToObservationUpperBoundMs
        > WEB06_THRESHOLDS.sustained.driverDispatchToCoveringPaintUpperBoundMs.max) {
        thresholdViolations.push("common-driver-covering-max");
      }
    }
    if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)
      && covering.length) {
      thresholdViolations.push(...independentlyEvaluateThresholdDistribution(
        covering.map((sample) => sample.eventToObservationMs),
        WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs,
        "common-covering",
      ).violations);
    }
    for (const sample of terminal) {
      const ceiling = sample.stressDeadline
        ? WEB06_THRESHOLDS.terminal.persistenceStressCompletionMs.max
        : WEB06_THRESHOLDS.terminal.eventToTerminalObservationMs.max;
      if (sample.eventToObservationMs > ceiling) thresholdViolations.push("common-terminal-max");
      if (!sample.stressDeadline
        && sample.driverDispatchToObservationUpperBoundMs
          > WEB06_THRESHOLDS.terminal.driverDispatchToTerminalUpperBoundMs.max) {
        thresholdViolations.push("common-driver-terminal-max");
      }
    }
  }
  const burst = independentBurstFacts(receipt, row);
  behaviorErrors.push(...burst.behaviorErrors);
  thresholdViolations.push(...burst.thresholdViolations);
  const frame = frameFacts(receipt, row, setupErrors, behaviorErrors, driverCalibration);
  const cadence = cadenceFacts(receipt, row);
  setupErrors.push(...cadence.errors.filter((error) => error.startsWith("SETUP_")));
  behaviorErrors.push(...cadence.errors.filter((error) => !error.startsWith("SETUP_")));
  const status = independentStatus({ setupErrors, behaviorErrors, thresholdViolations,
    frameRed: frame.frameRed, longTaskRed: frame.longTaskRed, cadence: cadence.cadence });
  return { status, setupErrors, behaviorErrors, thresholdViolations, frameRed: frame.frameRed,
    longTaskRed: frame.longTaskRed, cadence: cadence.cadence, cadenceCounts: cadence.counts,
    metrics: { samples: metrics } };
}

function thresholdViolationsFor(row, covering, terminal) {
  const violations = [];
  if (!(row.binding === true || row.binding === "branch-b-only")) return violations;
  for (const sample of covering) {
    if (sample.eventToCoveringPaintMs > WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs.max) {
      violations.push("covering-max");
    }
    if (sample.driverDispatchToCoveringPaintUpperBoundMs
      > WEB06_THRESHOLDS.sustained.driverDispatchToCoveringPaintUpperBoundMs.max) {
      violations.push("driver-covering-max");
    }
  }
  if (covering.length) {
    violations.push(...independentlyEvaluateThresholdDistribution(
      covering.map((sample) => sample.preServiceWaitUpperBoundMs),
      WEB06_THRESHOLDS.sustained.preServiceWaitUpperBoundMs,
      "pre-service",
    ).violations);
  }
  if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)
    && covering.length) {
    violations.push(...independentlyEvaluateThresholdDistribution(
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
    if (!sample.stressDeadline
      && sample.driverDispatchToTerminalUpperBoundMs
        > WEB06_THRESHOLDS.terminal.driverDispatchToTerminalUpperBoundMs.max) {
      violations.push("driver-terminal-max");
    }
  }
  return violations;
}

function frameFacts(receipt, row, setupErrors, behaviorErrors, driverCalibrations) {
  const idle = receipt.idleFrameIntervalsMs ?? [];
  if (idle.some((value) => !finite(value) || value <= 0)) {
    setupErrors.push("SETUP_NONFINITE_IDLE_FRAME_INTERVAL");
  }
  if (idle.length < WEB06_THRESHOLDS.frame.requiredIdleIntervals) setupErrors.push("SETUP_IDLE_FRAME_COUNT");
  if (idle.length && idle.every((value) => finite(value) && value > 0)) {
    const median = distribution(idle).median;
    if (median < WEB06_THRESHOLDS.frame.idleMedianMs.min || median > WEB06_THRESHOLDS.frame.idleMedianMs.max) {
      setupErrors.push(`SETUP_IDLE_REFRESH_LANE:${median}`);
    }
  }
  const windows = receipt.interactionWindows ?? [];
  if (windows.length !== row.expectedInteractionWindowCount) {
    setupErrors.push(`SETUP_INTERACTION_WINDOW_INVALID:count:${windows.length}!=${row.expectedInteractionWindowCount}`);
  }
  const realms = new Map();
  const seenWindowIds = new Set();
  windows.forEach((window, index) => {
    const validWindowId = typeof window.windowId === "string" && window.windowId.length > 0;
    const instance = validWindowId ? window.windowId : `index-${index + 1}`;
    if (!finite(window.startedAt) || !finite(window.endedAt)
      || window.endedAt < window.startedAt || window.startedAt !== window.startBoundaryRafAt
      || window.endedAt !== window.endBoundaryRafAt) {
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
    const previous = realms.get(window.pageInstanceId);
    if (previous !== undefined && window.startedAt <= previous) {
      setupErrors.push(`SETUP_INTERACTION_WINDOW_INVALID:${instance}:realm-order`);
    }
    realms.set(window.pageInstanceId, window.endedAt);
  });
  if (row.id === "learned-row") {
    const segments = receipt.idleFrameSegments;
    const windowPages = windows.map((window) => window.pageInstanceId);
    if (!Array.isArray(segments) || segments.length !== 2
      || new Set(segments.map((segment) => segment?.pageInstanceId)).size !== 2
      || !exactJson(segments.map((segment) => segment?.pageInstanceId), windowPages)
      || !exactJson(segments.flatMap((segment) => segment?.intervalsMs ?? []), idle)
      || segments.some((segment) => !Array.isArray(segment?.intervalsMs)
        || segment.intervalsMs.length < WEB06_THRESHOLDS.frame.requiredIdleIntervals
        || segment.intervalsMs.some((value) => !finite(value) || value <= 0))) {
      setupErrors.push("SETUP_LEARNED_IDLE_FRAME_SEGMENTS_INVALID");
    } else for (const segment of segments) {
      const median = distribution(segment.intervalsMs).median;
      if (median < WEB06_THRESHOLDS.frame.idleMedianMs.min
        || median > WEB06_THRESHOLDS.frame.idleMedianMs.max) {
        setupErrors.push(`SETUP_LEARNED_IDLE_REFRESH_LANE:${segment.pageInstanceId}:${median}`);
      }
    }
    const continuity = receipt.lifecycleContinuity;
    const pre = continuity?.pre;
    const post = continuity?.post;
    if (new Set(windowPages).size !== 2 || continuity?.browserLifecycleEventCount !== 1
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
      || !exactJson(pre?.storagePayloadKeys, ["measurementId", "continuityNonce"])
      || post?.storageRemoved !== true || post?.oneShot !== true
      || post?.requiresFreshDriverPageCalibration !== true
      || post?.requiresFreshWorkerCalibration !== true) {
      behaviorErrors.push("LEARNED_REAL_RELOAD_CONTINUITY_INVALID");
    }
  }
  const idleControls = receipt.idleControlWindows ?? [];
  if (idleControls.length !== windows.length || idleControls.some((control, index) => {
    const measured = windows[index];
    return typeof control.controlId !== "string" || !control.controlId
      || typeof control.pageInstanceId !== "string" || !control.pageInstanceId
      || control.pageInstanceId !== measured?.pageInstanceId
      || !finite(control.startedAt) || !finite(control.endedAt) || control.endedAt <= control.startedAt
      || Math.abs((control.endedAt - control.startedAt) - (measured.endedAt - measured.startedAt)) > 0.001
      || windows.some((window) => window.pageInstanceId === control.pageInstanceId
        && control.startedAt < window.endedAt && control.endedAt > window.startedAt);
  })) setupErrors.push("SETUP_IDLE_LONG_TASK_CONTROL_INVALID");
  const observers = Array.isArray(receipt.longTaskObserver?.segments)
    ? receipt.longTaskObserver.segments : [receipt.longTaskObserver];
  const firstWindowAt = windows[0]?.startedAt;
  if (!finite(firstWindowAt) || windows.some((window) => !observers.some((observer) => observer?.supported === true
    && (observer.pageInstanceId === undefined || observer.pageInstanceId === window.pageInstanceId)
    && finite(observer.installedAt) && observer.installedAt <= window.startedAt))) {
    setupErrors.push("SETUP_LONG_TASK_OBSERVER_UNAVAILABLE");
  }
  const focus = receipt.focusVisibilitySamples ?? [];
  if (!Array.isArray(focus) || !focus.length || focus.some((sample) => !finite(sample.recordedAt)
    || sample.focused !== true || sample.visibilityState !== "visible"
    || typeof sample.pageInstanceId !== "string" || !sample.pageInstanceId)) {
    setupErrors.push("SETUP_PAGE_NOT_FOREGROUND");
  } else {
    for (const window of windows) {
      const pre = focus.filter((sample) => sample.role === "pre-boundary" && sample.windowId === window.windowId);
      const post = focus.filter((sample) => sample.role === "post-boundary" && sample.windowId === window.windowId);
      if (typeof window.windowId !== "string" || !window.windowId
        || typeof window.pageInstanceId !== "string" || !window.pageInstanceId
        || pre.length !== 1 || post.length !== 1 || pre[0]?.pageInstanceId !== window.pageInstanceId
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
  if (frames.some((value) => !finite(value) || value <= 0)) {
    setupErrors.push("SETUP_NONFINITE_INTERACTION_FRAME_INTERVAL");
  }
  const frameWindows = receipt.interactionFrameWindows ?? [];
  if (frameWindows.length !== windows.length) {
    setupErrors.push("SETUP_INTERACTION_FRAME_BOUNDARIES_MISSING");
  } else {
    const recomputedFrames = [];
    const recomputedTimestamps = [];
    for (let index = 0; index < frameWindows.length; index += 1) {
      const frameWindow = frameWindows[index];
      const window = windows[index];
      const timestamps = frameWindow?.timestamps;
      const declaredIntervals = frameWindow?.intervalsMs;
      if (frameWindow?.windowId !== window?.windowId || frameWindow?.pageInstanceId !== window?.pageInstanceId
        || !Array.isArray(timestamps) || timestamps.length < 2 || timestamps.some((value) => !finite(value))
        || !Array.isArray(declaredIntervals) || declaredIntervals.length !== timestamps.length - 1
        || timestamps[0] !== window.startedAt || timestamps.at(-1) !== window.endedAt) {
        setupErrors.push(`SETUP_INTERACTION_FRAME_BOUNDARY_MISMATCH:${index + 1}`);
        continue;
      }
      const calculatedIntervals = timestamps.slice(1).map((value, position) => value - timestamps[position]);
      if (!exactJson(calculatedIntervals, declaredIntervals)) {
        setupErrors.push(`SETUP_INTERACTION_FRAME_INTERVAL_MISMATCH:${index + 1}`);
      }
      recomputedTimestamps.push(...timestamps);
      recomputedFrames.push(...declaredIntervals);
    }
    if (!exactJson(recomputedTimestamps, receipt.interactionFrameTimestamps ?? [])
      || !exactJson(recomputedFrames, receipt.interactionFrameIntervalsMs ?? [])) {
      setupErrors.push("SETUP_INTERACTION_FRAME_FLATTENING_MISMATCH");
    }
    const eventsWithDispatch = (receipt.events ?? []).filter((event) =>
      finite(event.actualDriverDispatchAt) && finite(event.normalizedEventAt));
    if (driverCalibrations && eventsWithDispatch.length) {
      try {
        const corrected = eventsWithDispatch.map((event) => ({
          pageInstanceId: event.pageInstanceId,
          point: clockPoint(
            event.actualDriverDispatchAt,
            independentCalibrationForPage(driverCalibrations, event.pageInstanceId),
            event.normalizedEventAt,
            1,
          ),
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
      ...(receipt.commonSamples ?? []).map((sample) => ({
        at: sample.observedAt,
        pageInstanceId: sample.pageInstanceId,
      })),
      ...(receipt.actions ?? []).flatMap((action) => [
        { at: action.paintObservedAt, pageInstanceId: action.pageInstanceId },
        { at: action.terminalObservedAt, pageInstanceId: action.pageInstanceId },
      ]),
    ].filter(({ at }) => finite(at));
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
    const overlap = windows.some((window) => window.pageInstanceId === task.pageInstanceId
      && task.startTime < window.endedAt && task.startTime + task.durationMs > window.startedAt);
    const idleOverlap = idleControls.some((control) => control.pageInstanceId === task.pageInstanceId
      && task.startTime < control.endedAt && task.startTime + task.durationMs > control.startedAt);
    if (task.overlapsInteractionWindow !== overlap) behaviorErrors.push(`LONG_TASK_OVERLAP_MISMATCH:${index + 1}`);
    if (task.overlapsIdleControl !== idleOverlap) behaviorErrors.push(`LONG_TASK_IDLE_OVERLAP_MISMATCH:${index + 1}`);
    if (!overlap && !idleOverlap) behaviorErrors.push(`LONG_TASK_OUTSIDE_DECLARED_CONTROLS:${index + 1}`);
  }
  if ((receipt.assetsRequestedDuringWindow ?? []).length) behaviorErrors.push("ASSET_REQUEST_DURING_WINDOW");
  const binding = row.binding === true || row.binding === "branch-b-only";
  const validFrames = frames.filter((value) => finite(value) && value > 0);
  return {
    frameRed: binding && (validFrames.some((value) => value >= WEB06_THRESHOLDS.frame.rejectIntervalAtOrAboveMs)
      || (validFrames.length > 0 && distribution(validFrames).p99 > WEB06_THRESHOLDS.frame.p99Ms.max)),
    longTaskRed: binding && (receipt.longTasks ?? []).some((task) => windows.some((window) =>
      window.pageInstanceId === task.pageInstanceId && task.startTime < window.endedAt
        && task.startTime + task.durationMs > window.startedAt)
      && task.durationMs >= WEB06_THRESHOLDS.frame.rejectLongTaskAtOrAboveMs),
  };
}

function independentStatus({ setupErrors, behaviorErrors, thresholdViolations, frameRed, longTaskRed, cadence }) {
  const behaviorRed = behaviorErrors.length > 0;
  if (behaviorRed) return cadence === "TOO_SHORT" ? "RED_BEHAVIOR" : "RED";
  if (setupErrors.length) return "SETUP_INVALID";
  const numericRed = cadence !== "TOO_SHORT"
    && (thresholdViolations.length > 0 || frameRed || longTaskRed);
  if (numericRed) return "RED";
  if (cadence === "TOO_SHORT" || cadence === "TOO_LONG") return "NO_VERDICT_INVALID_CADENCE";
  return "PASS";
}

function componentValues(parsed, surface) {
  const rows = surface === "common" ? parsed.metrics.samples : parsed.metrics.actions.map((action) => action.metrics);
  const output = {};
  for (const row of rows) {
    for (const [field, value] of Object.entries(row ?? {})) {
      if (finite(value) && value >= 0 && INDEPENDENT_COMPONENT_KEYS.has(field)) {
        (output[field] ??= []).push(value);
      }
    }
  }
  return output;
}

function buildIndependentRoundSummary(receipt, surface) {
  const parsed = surface === "common" ? deriveCommon(receipt) : deriveInternal(receipt);
  const numericEvidenceBinding = parsed.setupErrors.length === 0 && parsed.cadence !== "TOO_SHORT";
  const components = Object.fromEntries(Object.entries(
    numericEvidenceBinding ? componentValues(parsed, surface) : {},
  )
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([field, values]) => [field, distribution(values)]));
  const outcomeCounts = Object.fromEntries(INDEPENDENT_OUTCOME_COUNT_KEYS.map((outcome) => [outcome, 0]));
  const rows = surface === "common" ? receipt.commonSamples ?? [] : receipt.actions ?? [];
  for (const row of rows) {
    const outcome = surface === "common" && row.outcome === "terminal" ? "committed" : row.outcome;
    if (OUTCOMES.includes(outcome)) outcomeCounts[outcome] += 1;
    else outcomeCounts.unclassified += 1;
  }
  const queueDepths = surface === "internal"
    ? (receipt.actions ?? []).map((action) => action.mainQueueDepth)
      .filter((value) => Number.isSafeInteger(value) && value >= 0) : [];
  const recoveryDepths = (receipt.burstRecoveries ?? []).map((recovery) => recovery.idleSnapshot?.queueDepth)
    .filter((value) => Number.isSafeInteger(value) && value >= 0);
  const burstLatencies = (receipt.burstRecoveries ?? []).flatMap((recovery) => {
    const event = (receipt.events ?? []).find((candidate) =>
      candidate.stepId === recovery.afterStepId && candidate.type === "keydown");
    const latency = finite(event?.normalizedEventAt) && finite(recovery.latestPaintAt)
      ? recovery.latestPaintAt - event.normalizedEventAt : undefined;
    return finite(latency) && latency >= 0 ? [latency] : [];
  });
  const frames = numericEvidenceBinding
    ? (receipt.interactionFrameIntervalsMs ?? []).filter((value) => finite(value) && value > 0)
    : [];
  const longTasks = numericEvidenceBinding
    ? (receipt.longTasks ?? []).filter((task) => finite(task.durationMs) && task.durationMs >= 0)
    : [];
  const overlapping = longTasks.filter((task) => task.overlapsInteractionWindow === true);
  const idleLongTasks = longTasks.filter((task) => task.overlapsIdleControl === true);
  const durationTotal = (rows) => rows.reduce((sum, task) => sum + task.durationMs, 0);
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
    cadence: independentCadenceCountsProjection(receipt),
    components,
    queue: {
      maxDepth: numericEvidenceBinding
        ? distributionOrNull([...queueDepths, ...recoveryDepths])?.max ?? null
        : null,
      endBurstDepth: numericEvidenceBinding ? recoveryDepths.at(-1) ?? null : null,
    },
    burst: { recoveryCount: (receipt.burstRecoveries ?? []).length,
      finalKeyToLatestPaintMs: numericEvidenceBinding ? distributionOrNull(burstLatencies) : null },
    frame: { intervals: distributionOrNull(frames),
      atOrAbove50MsCount: frames.filter((value) => value >= WEB06_THRESHOLDS.frame.rejectIntervalAtOrAboveMs).length },
    longTask: {
      count: longTasks.length,
      durationMs: distributionOrNull(longTasks.map((task) => task.durationMs)),
      overlapCount: overlapping.length,
      overlapDurationMs: distributionOrNull(overlapping.map((task) => task.durationMs)),
      idleControlCount: idleLongTasks.length,
      idleControlDurationMs: distributionOrNull(idleLongTasks.map((task) => task.durationMs)),
      interactionMinusIdleCount: overlapping.length - idleLongTasks.length,
      interactionMinusIdleDurationMs: durationTotal(overlapping) - durationTotal(idleLongTasks),
    },
  };
  return attachIndependentRoundIntegrity(summary, parsed);
}

/** Public test/review entry point for the verifier-owned round derivation. */
export function independentlyRecomputeRoundSummary(receipt, surface) {
  invariant(["internal", "common"].includes(surface), "WEB06_INDEPENDENT_SURFACE");
  return buildIndependentRoundSummary(receipt, surface);
}

function poolViolations(receipts, surface, parsed) {
  const scenarioId = receipts[0].scenarioId;
  invariant(hasOwn(SCENARIO_REGISTRY, scenarioId), "WEB06_INDEPENDENT_POOL_SCENARIO_UNKNOWN", scenarioId);
  const row = SCENARIO_REGISTRY[scenarioId];
  const binding = row.binding === true || row.binding === "branch-b-only";
  const covering = surface === "common"
    ? parsed.flatMap((item) => item.metrics.samples.filter((sample) => sample.sampleKind === "covering"))
    : parsed.flatMap((item) => item.metrics.covering);
  const terminal = surface === "common"
    ? parsed.flatMap((item) => item.metrics.samples.filter((sample) => sample.sampleKind === "terminal"))
    : parsed.flatMap((item) => item.metrics.terminal);
  const prefix = surface === "common" ? "pooled-common" : "pooled";
  const violations = [];
  const expectedCovering = row.expectedCoveringSamples * 5;
  const expectedTerminal = row.expectedTerminalSamples * 5;
  if (covering.length !== expectedCovering) violations.push(`${prefix}-covering-count:${covering.length}!=${expectedCovering}`);
  if (terminal.length !== expectedTerminal) violations.push(`${prefix}-terminal-count:${terminal.length}!=${expectedTerminal}`);
  const coveringValues = covering.map((sample) => surface === "common"
    ? sample.eventToObservationMs : sample.eventToCoveringPaintMs);
  if (["rapid-jyutping", "rapid-long-jyutping", "rapid-luna", "burst-jyutping", "burst-luna"].includes(row.id)) {
    if (!coveringValues.length) violations.push(`${prefix}-covering:missing`);
    else {
      const summary = distribution(coveringValues);
      for (const key of ["p95", "p99", "max"]) {
        const ceiling = WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs[key];
        if (summary[key] > ceiling) violations.push(`${prefix}-covering:${key}:${summary[key]}>${ceiling}`);
      }
    }
  }
  if (surface === "internal" && binding) {
    const pre = covering.map((sample) => sample.preServiceWaitUpperBoundMs);
    if (pre.length !== expectedCovering) violations.push(`pooled-pre-service-count:${pre.length}!=${expectedCovering}`);
    else if (pre.length) {
      const summary = distribution(pre);
      for (const key of ["p95", "max"]) {
        const ceiling = WEB06_THRESHOLDS.sustained.preServiceWaitUpperBoundMs[key];
        if (summary[key] > ceiling) violations.push(`pooled-pre-service:${key}:${summary[key]}>${ceiling}`);
      }
    }
  }
  const driverCovering = covering.map((sample) => surface === "common"
    ? sample.driverDispatchToObservationUpperBoundMs : sample.driverDispatchToCoveringPaintUpperBoundMs);
  if (binding && driverCovering.some((value) => value > WEB06_THRESHOLDS.sustained.driverDispatchToCoveringPaintUpperBoundMs.max)) {
    violations.push(surface === "common" ? "pooled-common-driver-covering:max" : "pooled-driver-covering:max");
  }
  const terminalValue = (sample) => surface === "common" ? sample.eventToObservationMs : sample.eventToTerminalObservationMs;
  if (binding && terminal.some((sample) => terminalValue(sample)
    > (sample.stressDeadline ? WEB06_THRESHOLDS.terminal.persistenceStressCompletionMs.max
      : WEB06_THRESHOLDS.terminal.eventToTerminalObservationMs.max))) {
    violations.push(surface === "common" ? "pooled-common-terminal:max" : "pooled-terminal:max");
  }
  const driverTerminal = terminal.filter((sample) => !sample.stressDeadline).map((sample) => surface === "common"
    ? sample.driverDispatchToObservationUpperBoundMs : sample.driverDispatchToTerminalUpperBoundMs);
  if (binding && driverTerminal.some((value) => value > WEB06_THRESHOLDS.terminal.driverDispatchToTerminalUpperBoundMs.max)) {
    violations.push(surface === "common" ? "pooled-common-driver-terminal:max" : "pooled-driver-terminal:max");
  }
  const frames = receipts.flatMap((receipt) => receipt.interactionFrameIntervalsMs ?? []);
  if (binding && frames.some((value) => value >= WEB06_THRESHOLDS.frame.rejectIntervalAtOrAboveMs)) {
    violations.push(surface === "common" ? "pooled-common-frame:max" : "pooled-frame:max");
  }
  if (binding && frames.length && distribution(frames).p99 > WEB06_THRESHOLDS.frame.p99Ms.max) {
    violations.push(surface === "common" ? "pooled-common-frame:p99" : "pooled-frame:p99");
  }
  return violations;
}

function buildIndependentFiveRoundSummary(receipts, surface) {
  invariant(receipts.length === 5, "WEB06_INDEPENDENT_FIVE_ROUND_COUNT");
  const identity = ({ metricContractVersion, scenarioRegistryVersion, behaviorPredicateVersion,
    scenarioRunId, scenarioId, schemaId, mode, source }) => JSON.stringify({
    metricContractVersion, scenarioRegistryVersion, behaviorPredicateVersion,
    scenarioRunId, scenarioId, schemaId, mode, source,
  });
  invariant(receipts.every((receipt) => identity(receipt) === identity(receipts[0])),
    "WEB06_INDEPENDENT_POOL_IDENTITY");
  for (const field of ["roundId", "attemptId"]) {
    const values = receipts.map((receipt) => receipt[field]);
    invariant(values.every((value) => typeof value === "string" && value)
      && new Set(values).size === receipts.length,
    `WEB06_INDEPENDENT_POOL_${field.toUpperCase()}_IDENTITY_INVALID`);
  }
  const parsed = receipts.map((receipt) => surface === "common" ? deriveCommon(receipt) : deriveInternal(receipt));
  const rounds = receipts.map((receipt) => buildIndependentRoundSummary(receipt, surface));
  const componentNames = [...new Set(rounds.flatMap((round) => Object.keys(round.components)))].sort();
  const summaryErrors = [];
  const pooledComponents = Object.fromEntries(componentNames.map((field) => {
    const values = parsed.flatMap((item) => componentValues(item, surface)[field] ?? []);
    if (!values.length) summaryErrors.push(`pooled-component-missing:${field}`);
    return [field, distributionOrNull(values)];
  }));
  const violations = poolViolations(receipts, surface, parsed);
  const frames = receipts.flatMap((receipt) => receipt.interactionFrameIntervalsMs ?? []);
  const status = parsed.every((item) => item.status === "PASS") && violations.length === 0
    && summaryErrors.length === 0 ? "PASS" : "RED";
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
    status,
    validRedObserved: rounds.some((round) => round.validRedObserved) || violations.length > 0 || summaryErrors.length > 0,
    roundSummaries: rounds,
    pooledComponents,
    pooledMetricsSha256: "",
    summaryErrors,
    poolViolations: violations,
    pooledFrame: distributionOrNull(frames),
    pooledLongTaskCount: receipts.reduce((sum, receipt) => sum
      + (receipt.longTasks ?? []).filter((task) => task.overlapsInteractionWindow === true).length, 0),
    pooledIdleLongTaskCount: receipts.reduce((sum, receipt) => sum
      + (receipt.longTasks ?? []).filter((task) => task.overlapsIdleControl === true).length, 0),
  };
  summary.pooledMetricsSha256 = sha256(Buffer.from(JSON.stringify({
    pooledComponents: summary.pooledComponents,
    pooledFrame: summary.pooledFrame,
    pooledLongTaskCount: summary.pooledLongTaskCount,
    pooledIdleLongTaskCount: summary.pooledIdleLongTaskCount,
  }), "utf8"));
  summary.semanticProjectionVersion = WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION;
  summary.decision = {
    roundRed: rounds.some((round) => round.status !== "PASS"),
    poolViolationRed: summary.poolViolations.length > 0,
    summaryErrorRed: summary.summaryErrors.length > 0,
    status: summary.status,
    validRedObserved: summary.validRedObserved,
  };
  summary.implementationDiagnosticBinding = independentDiagnosticBinding(summary);
  summary.semanticProjectionSha256 = sha256(independentSemanticBytes(summary));
  return summary;
}

/** Public test/review entry point for verifier-owned five-round pooling. */
export function independentlyRecomputeFiveRoundSummary(receipts, surface) {
  invariant(["internal", "common"].includes(surface), "WEB06_INDEPENDENT_SURFACE");
  return buildIndependentFiveRoundSummary(receipts, surface);
}

function classifySurfaces(recomputed) {
  const summaries = Object.values(recomputed);
  const validRedObserved = summaries.some((summary) => ["RED", "RED_BEHAVIOR"].includes(summary.status));
  const cadenceValid = summaries.every((summary) => ["IN_RANGE", "NOT_APPLICABLE"].includes(summary.cadenceVerdict));
  const setupClean = summaries.every((summary) => summary.setupErrorCodes.length === 0);
  if (validRedObserved) return { classification: "RED", retainedMeasured: true,
    retainedLogicalRound: true,
    validForLatencyFrame: cadenceValid && setupClean, retainedHardRed: true,
    retryEligible: false, validRedObserved: true };
  if (summaries.some((summary) => summary.status === "SETUP_INVALID")) {
    return { classification: "SETUP_INVALID", retainedMeasured: false, retainedLogicalRound: false,
      validForLatencyFrame: false,
      retainedHardRed: false, retryEligible: true, validRedObserved: false };
  }
  if (summaries.some((summary) => summary.status === "NO_VERDICT_INVALID_CADENCE")) {
    return { classification: "NO_VERDICT_INVALID_CADENCE", retainedMeasured: false,
      retainedLogicalRound: false, validForLatencyFrame: false,
      retainedHardRed: false, retryEligible: true, validRedObserved: false };
  }
  return { classification: "PASS", retainedMeasured: true, retainedLogicalRound: true,
    validForLatencyFrame: true,
    retainedHardRed: false, retryEligible: false, validRedObserved: false };
}

export function independentlyRecomputeIncompleteAttemptFacts(envelope, attempt) {
  const failure = envelope.partialAttempt?.failure;
  invariant(failure && typeof failure.code === "string", "WEB06_INDEPENDENT_PARTIAL_FAILURE_MISSING",
    attempt.attemptId);
  const dimension = independentlyClassifyHarnessFailure(failure.code);
  invariant(dimension === failure.dimension, "WEB06_INDEPENDENT_PARTIAL_FAILURE_CLASSIFICATION",
    attempt.attemptId);
  const retainedBehaviorRed = independentRetainedRawReceiptBehaviorErrors(envelope).length > 0;
  if (dimension === "behavior" || retainedBehaviorRed) {
    invariant(attempt.measurementStarted === true, "WEB06_INDEPENDENT_PARTIAL_BEHAVIOR_PRESTART", attempt.attemptId);
    return { classification: "RED", retainedMeasured: false, retainedLogicalRound: false,
      validForLatencyFrame: false,
      retainedHardRed: true, retryEligible: false, validRedObserved: true };
  }
  return { classification: "SETUP_INVALID", retainedMeasured: false, retainedLogicalRound: false,
    validForLatencyFrame: false,
    retainedHardRed: false, retryEligible: true, validRedObserved: false };
}

async function readRelativeRaw(root, relativePath) {
  invariant(typeof relativePath === "string" && !path.isAbsolute(relativePath), "WEB06_INDEPENDENT_RAW_PATH_INVALID");
  const canonicalRoot = await realpath(root);
  const candidate = path.resolve(canonicalRoot, relativePath);
  const relative = path.relative(canonicalRoot, candidate);
  invariant(relative && relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative),
    "WEB06_INDEPENDENT_RAW_PATH_ESCAPE");
  const canonicalCandidate = await realpath(candidate);
  const canonicalRelative = path.relative(canonicalRoot, canonicalCandidate);
  invariant(canonicalRelative && canonicalRelative !== ".."
    && !canonicalRelative.startsWith(`..${path.sep}`) && !path.isAbsolute(canonicalRelative),
  "WEB06_INDEPENDENT_RAW_PATH_ESCAPE");
  const metadata = await lstat(candidate);
  invariant(metadata.isFile() && !metadata.isSymbolicLink(), "WEB06_INDEPENDENT_RAW_PATH_NOT_REGULAR");
  return { bytes: await readFile(canonicalCandidate) };
}

export async function independentlyVerifyRunnerSource({ repoRoot, manifest }) {
  invariant(manifest?.version === "web06-runner-source-v1", "WEB06_INDEPENDENT_RUNNER_SOURCE_VERSION");
  invariant(exactJson(manifest.tooling?.files?.map((file) => file.path), INDEPENDENT_RUNNER_TOOLING_PATHS),
    "WEB06_INDEPENDENT_RUNNER_TOOLING_PATHS");
  invariant(manifest.toolingManifestSha256 === sha256(Buffer.from(JSON.stringify(manifest.tooling), "utf8")),
    "WEB06_INDEPENDENT_RUNNER_TOOLING_MANIFEST_HASH");
  const git = async (...args) => (await execFileAsync("git", args, { cwd: repoRoot, encoding: "utf8" })).stdout.trim();
  const [head, tree, status] = await Promise.all([
    git("rev-parse", "HEAD"), git("rev-parse", "HEAD^{tree}"), git("status", "--porcelain", "--untracked-files=all"),
  ]);
  invariant(head === manifest.sourceCommit, "WEB06_INDEPENDENT_RUNNER_HEAD");
  invariant(tree === manifest.sourceTree, "WEB06_INDEPENDENT_RUNNER_TREE");
  invariant(status === "", "WEB06_INDEPENDENT_RUNNER_WORKTREE_DIRTY");
  const files = [];
  for (const expected of manifest.tooling.files) {
    const absolute = path.resolve(repoRoot, expected.path);
    const relative = path.relative(repoRoot, absolute);
    invariant(relative && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative),
      "WEB06_INDEPENDENT_RUNNER_PATH_ESCAPE", expected.path);
    const actual = sha256(await readFile(absolute));
    invariant(actual === expected.sha256, "WEB06_INDEPENDENT_RUNNER_FILE_HASH", expected.path);
    files.push({ path: expected.path, sha256: actual });
  }
  const observation = { version: "web06-runner-source-observation-v1", sourceCommit: head, sourceTree: tree,
    sourceTreeState: "clean", toolingManifestSha256: manifest.toolingManifestSha256, files };
  return { ...observation, observationSha256: sha256(Buffer.from(JSON.stringify(observation), "utf8")) };
}

function independentObserverLongTaskLoci(receipt) {
  return (receipt?.longTasks ?? [])
    .filter((task) => task.overlapsInteractionWindow === true)
    .map((task) => {
    const windowIndex = (receipt.interactionWindows ?? []).findIndex((window) =>
      window.pageInstanceId === task.pageInstanceId
      && task.startTime < window.endedAt
      && task.startTime + task.durationMs > window.startedAt);
    const event = [...(receipt.events ?? [])]
      .filter((candidate) => candidate.pageInstanceId === task.pageInstanceId
        && candidate.normalizedEventAt <= task.startTime + task.durationMs)
      .at(-1);
      return `${windowIndex}:${event?.stepId ?? "no-event"}`;
    });
}

function independentObserverModeRawProjection(envelope) {
  const common = envelope.commonReceipt;
  invariant(common && envelope.sentinel, "WEB06_INDEPENDENT_OBSERVER_COMMON_RECEIPT_MISSING");
  const internal = envelope.privateReceipt ?? common;
  const commonParsed = deriveCommon(common);
  const internalParsed = envelope.privateReceipt ? deriveInternal(internal) : commonParsed;
  const commonEquivalence = {
    scenarioId: common.scenarioId,
    events: common.events?.map((event) => [event.stepId, event.type, event.key, event.code]),
    samples: common.commonSamples?.map((sample) => [
      sample.stepId,
      sample.sampleKind,
      sample.outcome,
      sample.supersededByStepId ?? null,
      sample.domFingerprintSha256,
    ]),
    candidatePageSize: common.candidatePageSize,
    cadenceIdentity: common.cadenceGaps?.map((gap) => [gap.stepId, gap.nominalGapMs]),
    interactionWindowCount: common.interactionWindows?.length,
    interactionFrameCount: common.interactionFrameIntervalsMs?.length,
    longTaskLoci: independentObserverLongTaskLoci(common),
    behaviorErrors: independentCanonicalDiagnostics(commonParsed.behaviorErrors),
    thresholdViolations: independentCanonicalDiagnostics(commonParsed.thresholdViolations),
    frameRed: commonParsed.frameRed,
    longTaskRed: commonParsed.longTaskRed,
  };
  const internalEquivalence = {
    actions: internal.actions?.map((action) => [
      action.stepId,
      action.kind,
      action.outcome,
      action.mainQueueDepth,
      action.workerDispatchDepth,
      action.terminalKind,
    ]),
    coveringCount: internalParsed.metrics?.covering?.length ?? 0,
    terminalCount: internalParsed.metrics?.terminal?.length ?? 0,
    behaviorErrors: independentCanonicalDiagnostics(internalParsed.behaviorErrors),
    thresholdViolations: independentCanonicalDiagnostics(internalParsed.thresholdViolations),
    frameRed: internalParsed.frameRed,
    longTaskRed: internalParsed.longTaskRed,
  };
  const sentinel = envelope.sentinel;
  const protocol = envelope.protocolExport;
  const mainCallbacks = protocol?.mainObserverCallbacks ?? [];
  const workerSpans = protocol?.actions?.flatMap((action) => action.worker?.collectorSpans ?? []) ?? [];
  const callbackIntervals = [
    ...(sentinel.callbackLedger ?? []).map((callback, index) => ({
      ...callback,
      callbackId: `${callback.pageInstanceId ?? "sentinel"}-sentinel-${index + 1}`,
      sequenceId: index + 1,
      sourceClass: "common-sentinel",
      windowIndex: (common.interactionWindows ?? []).findIndex((window) =>
        window.pageInstanceId === callback.pageInstanceId
        && callback.startedAt >= window.startedAt && callback.startedAt <= window.endedAt),
    })),
    ...mainCallbacks.map((callback) => ({
      ...callback,
      sourceClass: envelope.target?.protocolMode === "minimal" ? "minimal-probe" : "full-collector",
    })),
  ];
  const callbackIds = callbackIntervals.map((callback) => callback.callbackId);
  const intervalLedgerValid = callbackIds.every((id) => typeof id === "string" && id.length > 0)
    && new Set(callbackIds).size === callbackIds.length
    && callbackIntervals.every((callback) => finite(callback.startedAt) && finite(callback.finishedAt)
      && callback.finishedAt >= callback.startedAt && finite(callback.durationMs)
      && callback.durationMs === callback.finishedAt - callback.startedAt);
  const sentinelLedgerValid = sentinel.callbackLedgerOverflowCount === 0
    && sentinel.sentinelAccountedCallbackCount === (sentinel.callbackLedger?.length ?? -1);
  const baseCallbackAttributionComplete = intervalLedgerValid && sentinelLedgerValid
    && (envelope.target?.protocolMode === "off"
    || (Array.isArray(protocol?.mainObserverCallbacks)
      && protocol.mainObserverCallbacks.length === protocol?.status?.mainObserverCallbackCount
      && protocol.status.mainObserverCallbackCount <= protocol.status.mainObserverCallbackCapacity
      && protocol.status.mainObserverCallbackOverflowCount === 0
      && protocol.actions?.every((action) => Array.isArray(action.worker?.collectorSpans))));
  const setupClean = commonParsed.setupErrors.length === 0 && internalParsed.setupErrors.length === 0;
  const cadenceValid = [commonParsed.cadence, internalParsed.cadence]
    .every((value) => ["IN_RANGE", "NOT_APPLICABLE"].includes(value));
  const hardRedCadenceValid = [commonParsed.cadence, internalParsed.cadence]
    .every((value) => ["IN_RANGE", "NOT_APPLICABLE", "TOO_LONG"].includes(value));
  const behaviorRedObserved = commonParsed.behaviorErrors.length > 0
    || internalParsed.behaviorErrors.length > 0;
  const projection = {
    measurementStarted: true,
    measurementCompleted: true,
    measurementValid: false,
    behaviorRedObserved,
    hardRedBindingValid: false,
    hardRedObserved: false,
    samples: (commonParsed.metrics?.samples ?? []).map((sample) => sample.eventToObservationMs),
    commonEquivalenceDigest: sha256(Buffer.from(JSON.stringify(commonEquivalence), "utf8")),
    ...(envelope.privateReceipt ? {
      internalEquivalenceDigest: sha256(Buffer.from(JSON.stringify(internalEquivalence), "utf8")),
    } : {}),
    commonVerdict: commonParsed.status,
    internalVerdict: internalParsed.status,
    commonEventCount: common.events?.length ?? 0,
    environmentManifestSha256: common.source?.environmentManifestSha256,
    environmentId: common.source?.environmentId,
    interactionWindowCount: common.interactionWindows?.length ?? 0,
    sentinelCallbacksMs: sentinel.sentinelCallbacksMs ?? [],
    sentinelTotalPerEventMs: sentinel.sentinelTotalPerEventMs ?? [],
    sentinelTotalPerWindowMs: sentinel.sentinelTotalPerWindowMs ?? [],
    collectorCallbacksMs: [
      ...(protocol?.mainObserverCallbacksMs ?? []),
      ...workerSpans.map((span) => span.finishedAt - span.startedAt),
    ],
    mainObserverCallbacksMs: protocol?.mainObserverCallbacksMs ?? [],
    workerCollectorCallbacksMs: workerSpans.map((span) => span.finishedAt - span.startedAt),
    callbackLedgerCount: sentinel.callbackLedger?.length,
    callbackLedgerCapacity: sentinel.callbackLedgerCapacity,
    sentinelAccountedCallbackCount: sentinel.sentinelAccountedCallbackCount,
    callbackLedgerOverflowCount: sentinel.callbackLedgerOverflowCount,
    callbackAttributionComplete: baseCallbackAttributionComplete,
    ...(envelope.privateReceipt ? {
      mainObserverCallbackCount: protocol?.status?.mainObserverCallbackCount,
      mainObserverCallbackCapacity: protocol?.status?.mainObserverCallbackCapacity,
      mainObserverCallbackOverflowCount: protocol?.status?.mainObserverCallbackOverflowCount,
    } : {}),
    callbackIntervals,
    rawLongTasks: (sentinel.longTasks ?? [])
      .filter((task) => task.overlapsInteractionWindow === true)
      .map((task, index) => ({
        ...task,
        locus: independentObserverLongTaskLoci(common)[index],
      })),
  };
  const modeName = envelope.target?.id === "PRODUCT" ? "product"
    : envelope.target?.id === "BASE_MINIMAL" ? "minimal" : "full";
  const ledger = independentObserverModeLedger(projection, modeName);
  projection.callbackAttributionComplete = ledger.attributionValid;
  projection.measurementValid = cadenceValid && setupClean && ledger.attributionValid;
  projection.hardRedBindingValid = hardRedCadenceValid && setupClean && ledger.attributionValid;
  if (!ledger.attributionValid) {
    projection.commonVerdict = behaviorRedObserved ? "RED_BEHAVIOR" : "SETUP_INVALID";
    projection.internalVerdict = behaviorRedObserved ? "RED_BEHAVIOR" : "SETUP_INVALID";
  }
  projection.hardRedObserved = independentObserverModeHardRedExpected(projection);
  return {
    ...projection,
    initialMeasurementValid: projection.measurementValid,
  };
}

export function independentlyProjectObserverModeRawEvidence(envelope) {
  return independentObserverModeRawProjection(envelope);
}

function independentPartialObserverModeProjection(envelope) {
  const dimension = independentlyClassifyHarnessFailure(envelope.partialAttempt?.failure?.code);
  const hardRedObserved = dimension === "behavior"
    || independentRetainedRawReceiptBehaviorErrors(envelope).length > 0;
  const verdict = hardRedObserved ? "RED_BEHAVIOR" : "SETUP_INVALID";
  return {
    measurementStarted: envelope.measurementStarted === true,
    measurementCompleted: false,
    measurementValid: false,
    behaviorRedObserved: hardRedObserved,
    hardRedBindingValid: false,
    hardRedObserved,
    samples: [],
    commonVerdict: verdict,
    internalVerdict: verdict,
    commonEventCount: 0,
    interactionWindowCount: 0,
    sentinelCallbacksMs: [],
    sentinelTotalPerEventMs: [],
    sentinelTotalPerWindowMs: [],
    collectorCallbacksMs: [],
    mainObserverCallbacksMs: [],
    workerCollectorCallbacksMs: [],
    callbackAttributionComplete: false,
    callbackIntervals: [],
    rawLongTasks: [],
    underlyingLongTasksMs: [],
    instrumentationAddedLongTasksMs: [],
  };
}

function independentObserverIntervalLedgerValid(mode) {
  const intervals = mode.callbackIntervals;
  if (mode.callbackAttributionComplete !== true || !Array.isArray(intervals)) return false;
  const ids = intervals.map((callback) => callback.callbackId);
  return ids.every((id) => typeof id === "string" && id.length > 0)
    && new Set(ids).size === ids.length
    && intervals.every((callback) => finite(callback.startedAt) && finite(callback.finishedAt)
      && callback.finishedAt >= callback.startedAt
      && callback.durationMs === callback.finishedAt - callback.startedAt);
}

function independentlyAuditObserverTripletRaw(triplet, rawEnvelopesByPath) {
  const targetIds = { product: "PRODUCT", minimal: "BASE_MINIMAL", full: "BASE_FULL" };
  const rawProjections = {};
  for (const modeName of ["product", "minimal", "full"]) {
    const mode = triplet[modeName];
    invariant(mode?.rawPacket?.relativePath, "WEB06_INDEPENDENT_OBSERVER_RAW_REFERENCE",
      `${triplet.attemptId}:${modeName}`);
    const raw = rawEnvelopesByPath.get(mode.rawPacket.relativePath);
    invariant(raw, "WEB06_INDEPENDENT_OBSERVER_RAW_MISSING", `${triplet.attemptId}:${modeName}`);
    const audit = independentlyAuditRawEnvelope(raw.envelope, {
      targetId: targetIds[modeName],
      scenarioRunId: "rapid-long-jyutping",
      scenarioId: "rapid-long-jyutping",
      schemaId: "jyut6ping3",
      attemptId: triplet.attemptId,
      measurementStarted: mode.measurementStarted,
    });
    invariant(audit.pass, "WEB06_INDEPENDENT_OBSERVER_RAW_CONTRACT",
      `${triplet.attemptId}:${modeName}:${audit.errors.join(",")}`);
    if (mode.measurementCompleted !== true) {
      const projection = independentPartialObserverModeProjection(raw.envelope);
      for (const field of Object.keys(projection)) {
        invariant(exactJson(mode[field], projection[field]), "WEB06_INDEPENDENT_OBSERVER_PARTIAL_RAW_PROJECTION",
          `${triplet.attemptId}:${modeName}:${field}`);
      }
      rawProjections[modeName] = projection;
      continue;
    }
    const projection = independentObserverModeRawProjection(raw.envelope);
    for (const field of Object.keys(projection)
      .filter((field) => !["initialMeasurementValid", "hardRedObserved"].includes(field))) {
      invariant(exactJson(mode[field], projection[field]), "WEB06_INDEPENDENT_OBSERVER_RAW_PROJECTION",
        `${triplet.attemptId}:${modeName}:${field}`);
    }
    rawProjections[modeName] = { ...projection,
      underlyingLongTasksMs: mode.underlyingLongTasksMs,
      instrumentationAddedLongTasksMs: mode.instrumentationAddedLongTasksMs };
  }
  const attribution = independentClassifyObserverLongTasks(rawProjections);
  const mismatches = attribution.errors.filter((error) => error.endsWith("attribution-mismatch"));
  invariant(mismatches.length === 0, "WEB06_INDEPENDENT_OBSERVER_LONG_TASK_PROJECTION",
    `${triplet.attemptId}:${mismatches.join(",")}`);
  for (const modeName of ["product", "minimal", "full"]) {
    const mode = triplet[modeName];
    const projection = rawProjections[modeName];
    const expectedValid = mode.measurementCompleted === true
      && projection.initialMeasurementValid === true
      && independentObserverIntervalLedgerValid(projection);
    invariant(mode.measurementValid === expectedValid, "WEB06_INDEPENDENT_OBSERVER_MEASUREMENT_VALIDITY",
      `${triplet.attemptId}:${modeName}`);
    invariant(mode.hardRedObserved
      === independentObserverModeHardRedExpected(attribution.modes[modeName]),
    "WEB06_INDEPENDENT_OBSERVER_HARD_RED_PROJECTION", `${triplet.attemptId}:${modeName}`);
  }
}

function independentExpectedRunMatrix(expectation, branch) {
  if (expectation === "PREVIEW") return ["existing-normal-guard", "rapid-jyutping"];
  if (expectation === "OBSERVER") return ["rapid-long-jyutping"];
  if (expectation === "FINAL" && branch === "B") {
    return [...INDEPENDENT_BINDING_SCENARIO_RUNS, ...INDEPENDENT_EXTENDED_SCENARIO_RUNS];
  }
  return [...INDEPENDENT_BINDING_SCENARIO_RUNS];
}

function independentlyValidateCollectorMatrix(collector) {
  invariant(["BASELINE", "FINAL", "PREVIEW", "OBSERVER"].includes(collector.expectation),
    "WEB06_INDEPENDENT_EXPECTATION");
  invariant(["NONE", "A", "B", "C"].includes(collector.selectedBranch), "WEB06_INDEPENDENT_BRANCH");
  const expectedRuns = independentExpectedRunMatrix(collector.expectation, collector.selectedBranch);
  invariant(exactJson(collector.scenarioRuns, expectedRuns), "WEB06_INDEPENDENT_SCENARIO_MATRIX");
  const observer = collector.expectation === "OBSERVER";
  invariant(Array.isArray(collector.scenarioResults) && Array.isArray(collector.observerTriplets),
    "WEB06_INDEPENDENT_LANE_ARRAYS");
  if (observer) {
    invariant(collector.scenarioResults.length === 0
      && collector.observerTriplets.length <= WEB06_THRESHOLDS.observer.maximumTripletAttempts,
    "WEB06_INDEPENDENT_OBSERVER_MATRIX");
    return;
  }
  invariant(collector.observerTriplets.length === 0 && collector.scenarioResults.length === expectedRuns.length,
    "WEB06_INDEPENDENT_SCENARIO_RESULT_COUNT");
  const expectedTarget = collector.expectation === "BASELINE" ? "BASE_FULL"
    : collector.expectation === "FINAL" ? "FINAL_FULL" : "FINAL_MINIMAL";
  for (let index = 0; index < expectedRuns.length; index += 1) {
    const scenario = collector.scenarioResults[index];
    invariant(scenario?.scenarioRunId === expectedRuns[index] && scenario.targetId === expectedTarget,
      "WEB06_INDEPENDENT_SCENARIO_RESULT_IDENTITY", String(index + 1));
  }
}

function independentlyRecomputeCollectorProvenance(collector, rawEnvelopesByPath) {
  invariant(rawEnvelopesByPath instanceof Map && rawEnvelopesByPath.size > 0,
    "WEB06_INDEPENDENT_RAW_PROVENANCE_MISSING");
  invariant(collector.runnerSourceManifest && typeof collector.runnerSourceManifest === "object",
    "WEB06_INDEPENDENT_RUNNER_SOURCE_MANIFEST_MISSING");
  const runnerSourceManifest = collector.runnerSourceManifest;
  invariant(runnerSourceManifest.version === "web06-runner-source-v1"
    && SHA40.test(runnerSourceManifest.sourceCommit ?? "")
    && SHA40.test(runnerSourceManifest.sourceTree ?? "")
    && runnerSourceManifest.sourceTreeState === "clean"
    && runnerSourceManifest.tooling && typeof runnerSourceManifest.tooling === "object",
  "WEB06_INDEPENDENT_RUNNER_SOURCE_MANIFEST_IDENTITY");
  const runnerToolingManifestSha256 = sha256(Buffer.from(
    JSON.stringify(runnerSourceManifest.tooling), "utf8",
  ));
  invariant(runnerSourceManifest.toolingManifestSha256 === runnerToolingManifestSha256,
    "WEB06_INDEPENDENT_RUNNER_TOOLING_MANIFEST_HASH");
  const runnerSourceManifestSha256 = sha256(Buffer.from(JSON.stringify(runnerSourceManifest), "utf8"));
  const collectorContractSha256 = sha256(Buffer.from(JSON.stringify({
    metricContractVersion: WEB06_METRIC_CONTRACT_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_REGISTRY_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_PREDICATE_VERSION,
    thresholds: WEB06_THRESHOLDS,
    scenarios: SCENARIO_REGISTRY,
    behaviorPredicates: WEB06_BEHAVIOR_PREDICATES,
    eventActionRules: EVENT_ACTION_RULES,
  }), "utf8"));
  const scenarioIdsSha256 = sha256(Buffer.from(JSON.stringify(collector.scenarioRuns), "utf8"));
  let rawIdentity;
  for (const [rawPath, raw] of rawEnvelopesByPath) {
    const envelope = raw?.envelope;
    invariant(envelope && typeof envelope === "object", "WEB06_INDEPENDENT_RAW_PROVENANCE_ENVELOPE", rawPath);
    const target = envelope.target ?? {};
    const row = {
      expectation: envelope.expectation,
      disposition: target.pinnedDisposition,
      selectedBranch: target.pinnedSelectedBranch,
      identityManifestSha256: envelope.identityManifestSha256,
      runnerSourceManifestSha256: envelope.runnerSourceManifestSha256,
      collectorContractSha256: target.collectorContractSha256,
      environmentManifestSha256: envelope.environmentManifestSha256,
      environmentId: envelope.environmentId,
      scenarioIdsSha256: envelope.scenarioIdsSha256,
    };
    invariant([row.identityManifestSha256, row.runnerSourceManifestSha256,
      row.collectorContractSha256, row.environmentManifestSha256, row.scenarioIdsSha256]
      .every((value) => SHA64.test(value ?? "")), "WEB06_INDEPENDENT_RAW_PROVENANCE_HASH", rawPath);
    invariant(typeof row.environmentId === "string" && row.environmentId.length > 0,
      "WEB06_INDEPENDENT_RAW_PROVENANCE_ENVIRONMENT_ID", rawPath);
    if (rawIdentity === undefined) rawIdentity = row;
    else invariant(exactJson(row, rawIdentity), "WEB06_INDEPENDENT_RAW_PROVENANCE_DRIFT", rawPath);
    invariant(row.expectation === collector.expectation, "WEB06_INDEPENDENT_RAW_EXPECTATION", rawPath);
    invariant(row.disposition === collector.disposition, "WEB06_INDEPENDENT_RAW_DISPOSITION", rawPath);
    invariant(row.selectedBranch === collector.selectedBranch, "WEB06_INDEPENDENT_RAW_SELECTED_BRANCH", rawPath);
    invariant(row.identityManifestSha256 === collector.identityManifestSha256,
      "WEB06_INDEPENDENT_RAW_IDENTITY_MANIFEST", rawPath);
    invariant(row.runnerSourceManifestSha256 === runnerSourceManifestSha256
      && row.runnerSourceManifestSha256 === collector.runnerSourceManifestSha256,
    "WEB06_INDEPENDENT_RAW_RUNNER_SOURCE_MANIFEST", rawPath);
    invariant(row.collectorContractSha256 === collectorContractSha256
      && row.collectorContractSha256 === collector.collectorContractSha256,
    "WEB06_INDEPENDENT_RAW_COLLECTOR_CONTRACT", rawPath);
    invariant(row.environmentManifestSha256 === collector.environmentManifestSha256,
      "WEB06_INDEPENDENT_RAW_ENVIRONMENT_MANIFEST", rawPath);
    invariant(row.environmentId === collector.environmentId,
      "WEB06_INDEPENDENT_RAW_ENVIRONMENT_ID", rawPath);
    invariant(row.scenarioIdsSha256 === scenarioIdsSha256,
      "WEB06_INDEPENDENT_RAW_SCENARIO_IDS", rawPath);
    for (const [label, observation] of [
      ["runner-source-before", envelope.runnerSourceBefore],
      ["attempt-source-before", envelope.attemptSourceBefore],
      ["attempt-source-after", envelope.attemptSourceAfter],
    ]) {
      if (observation === undefined) continue;
      invariant(observation.sourceCommit === runnerSourceManifest.sourceCommit
        && observation.sourceTree === runnerSourceManifest.sourceTree
        && observation.sourceTreeState === runnerSourceManifest.sourceTreeState
        && observation.toolingManifestSha256 === runnerToolingManifestSha256,
      "WEB06_INDEPENDENT_RAW_RUNNER_SOURCE_IDENTITY", `${rawPath}:${label}`);
    }
    invariant(envelope.runnerSourceBefore?.observationSha256 === collector.runnerSourceObservationSha256,
      "WEB06_INDEPENDENT_RAW_RUNNER_SOURCE_BEFORE", rawPath);
    if (envelope.attemptSourceBefore?.observationSha256 !== undefined) {
      invariant(envelope.attemptSourceBefore.observationSha256 === collector.runnerSourceObservationSha256,
        "WEB06_INDEPENDENT_RAW_ATTEMPT_SOURCE_BEFORE", rawPath);
    }
    if (envelope.attemptSourceAfter?.observationSha256 !== undefined) {
      invariant(envelope.attemptSourceAfter.observationSha256 === collector.runnerSourcePostObservationSha256,
        "WEB06_INDEPENDENT_RAW_ATTEMPT_SOURCE_AFTER", rawPath);
    }
  }
  invariant(rawIdentity !== undefined, "WEB06_INDEPENDENT_RAW_PROVENANCE_MISSING");
  invariant(collector.runnerSourceObservationSha256 === collector.runnerSourcePostObservationSha256,
    "WEB06_INDEPENDENT_COLLECTOR_SOURCE_OBSERVATION_DRIFT");
  return {
    expectation: rawIdentity.expectation,
    disposition: rawIdentity.disposition,
    selectedBranch: rawIdentity.selectedBranch,
    identityManifestSha256: rawIdentity.identityManifestSha256,
    runnerSourceManifestSha256,
    collectorContractSha256,
    environmentManifestSha256: rawIdentity.environmentManifestSha256,
    environmentId: rawIdentity.environmentId,
  };
}

/** Side-effect-free recomputation over already parsed collector/raw envelopes. */
export function recomputeCollectorPayload({ collector, rawEnvelopesByPath }) {
  invariant(collector?.version === "web06-collector-output-v1", "WEB06_INDEPENDENT_COLLECTOR_VERSION");
  invariant(collector.writeMode === "create-new", "WEB06_INDEPENDENT_COLLECTOR_WRITE_MODE");
  independentlyValidateCollectorMatrix(collector);
  const provenance = independentlyRecomputeCollectorProvenance(collector, rawEnvelopesByPath);
  const scenarioResults = [];
  for (const scenario of collector.scenarioResults ?? []) {
    const requiredMeasuredCount = collector.expectation === "PREVIEW" ? 1 : 5;
    const maximumAttempts = collector.expectation === "PREVIEW" ? 1 : WEB06_THRESHOLDS.attempts.maximum;
    const run = hasOwn(SCENARIO_RUN_REGISTRY, scenario.scenarioRunId)
      ? SCENARIO_RUN_REGISTRY[scenario.scenarioRunId]
      : undefined;
    invariant(run?.scenarioId === scenario.scenarioId && run?.schema === scenario.schemaId,
      "WEB06_INDEPENDENT_SCENARIO_RUN_IDENTITY", scenario.scenarioRunId);
    const attemptResults = [];
    const retained = { internal: [], common: [] };
    let measuredCount = 0;
    let validLatencyFrameCount = 0;
    for (const [index, attempt] of (scenario.attempts ?? []).entries()) {
      invariant(attempt.attemptId === `attempt-${index + 1}`, "WEB06_INDEPENDENT_ATTEMPT_ID", scenario.scenarioRunId);
      invariant(attempt.rawPacket && typeof attempt.rawPacket.relativePath === "string",
        "WEB06_INDEPENDENT_RAW_REFERENCE", attempt.attemptId);
      const raw = rawEnvelopesByPath.get(attempt.rawPacket.relativePath);
      invariant(raw !== undefined, "WEB06_INDEPENDENT_RAW_MISSING", attempt.attemptId);
      const envelope = raw.envelope;
      const rawAudit = independentlyAuditRawEnvelope(envelope, {
        targetId: scenario.targetId,
        scenarioRunId: scenario.scenarioRunId,
        scenarioId: scenario.scenarioId,
        schemaId: scenario.schemaId,
        attemptId: attempt.attemptId,
        measurementStarted: attempt.measurementStarted,
      });
      invariant(rawAudit.pass, "WEB06_INDEPENDENT_RAW_CONTRACT", rawAudit.errors.join(","));
      if (attempt.measurementCompleted === false) {
        const facts = independentlyRecomputeIncompleteAttemptFacts(envelope, attempt);
        invariant(Object.keys(attempt.runnerSummaries ?? {}).length === 0,
          "WEB06_INDEPENDENT_PARTIAL_RUNNER_SUMMARY_PRESENT", attempt.attemptId);
        for (const field of Object.keys(facts)) {
          invariant(attempt[field] === facts[field],
            "WEB06_INDEPENDENT_ATTEMPT_FACT_MISMATCH", `${attempt.attemptId}:${field}`);
        }
        attemptResults.push({
          attemptId: attempt.attemptId,
          rawPacketSha256: raw.sha256,
          measurementStarted: attempt.measurementStarted,
          measurementCompleted: false,
          failureCode: envelope.partialAttempt.failure.code,
          ...facts,
        });
        if (facts.retainedHardRed) {
          invariant(index === scenario.attempts.length - 1,
            "WEB06_INDEPENDENT_ATTEMPT_AFTER_INCOMPLETE_RED", scenario.scenarioRunId);
        }
        continue;
      }
      invariant(attempt.measurementStarted === true, "WEB06_INDEPENDENT_COMPLETED_WITHOUT_START", attempt.attemptId);
      const recomputed = {};
      if (envelope.privateReceipt !== undefined) recomputed.internal = buildIndependentRoundSummary(envelope.privateReceipt, "internal");
      if (envelope.commonReceipt !== undefined) recomputed.common = buildIndependentRoundSummary(envelope.commonReceipt, "common");
      invariant(Object.keys(recomputed).length > 0, "WEB06_INDEPENDENT_RECEIPT_MISSING", attempt.attemptId);
      const diagnosticBindingPairs = {};
      for (const surface of Object.keys(recomputed)) {
        const runnerSummary = attempt.runnerSummaries?.[surface];
        diagnosticBindingPairs[surface] = independentlyBindSummaryDiagnostics(
          runnerSummary,
          recomputed[surface],
          `${attempt.attemptId}:${surface}`,
        );
      }
      const facts = classifySurfaces(recomputed);
      if (facts.retainedLogicalRound) measuredCount += 1;
      if (facts.validForLatencyFrame) validLatencyFrameCount += 1;
      invariant(measuredCount <= requiredMeasuredCount, "WEB06_INDEPENDENT_MEASURED_RETRY_FORBIDDEN", scenario.scenarioRunId);
      for (const field of Object.keys(facts)) {
        if (attempt[field] !== undefined) invariant(attempt[field] === facts[field],
          "WEB06_INDEPENDENT_ATTEMPT_FACT_MISMATCH", `${attempt.attemptId}:${field}`);
      }
      if (facts.validForLatencyFrame) {
        for (const surface of Object.keys(recomputed)) retained[surface].push(envelope[`${surface === "internal" ? "private" : "common"}Receipt`]);
      }
      attemptResults.push({
        attemptId: attempt.attemptId,
        rawPacketSha256: raw.sha256,
        measurementStarted: true,
        measurementCompleted: attempt.measurementCompleted === true,
        ...facts,
        diagnosticBindingPairs,
        recomputedSummarySha256: sha256(Buffer.from(JSON.stringify(recomputed), "utf8")),
      });
      if (measuredCount === requiredMeasuredCount) {
        invariant(index === scenario.attempts.length - 1, "WEB06_INDEPENDENT_ATTEMPT_AFTER_FIFTH_MEASURED", scenario.scenarioRunId);
      }
    }
    const fiveRoundSummaries = {};
    const fiveRoundDiagnosticBindingPairs = {};
    if (collector.expectation !== "PREVIEW" && validLatencyFrameCount === 5) {
      for (const surface of ["internal", "common"]) {
        if (!retained[surface].length) continue;
        invariant(retained[surface].length === 5, "WEB06_INDEPENDENT_SURFACE_ROUND_COUNT", `${scenario.scenarioRunId}:${surface}`);
        fiveRoundSummaries[surface] = buildIndependentFiveRoundSummary(retained[surface], surface);
        const runnerSummary = scenario.runnerFiveRoundSummaries?.[surface];
        fiveRoundDiagnosticBindingPairs[surface] = independentlyBindSummaryDiagnostics(
          runnerSummary,
          fiveRoundSummaries[surface],
          `${scenario.scenarioRunId}:${surface}:pool`,
        );
      }
    } else {
      if (collector.expectation !== "PREVIEW") {
        const terminalIncompleteRed = attemptResults.at(-1)?.measurementCompleted === false
          && attemptResults.at(-1)?.retainedHardRed === true;
        invariant(measuredCount === requiredMeasuredCount
          || (scenario.attempts ?? []).length === maximumAttempts
          || terminalIncompleteRed,
          "WEB06_INDEPENDENT_INCOMPLETE_BEFORE_ATTEMPT_CAP", scenario.scenarioRunId);
      } else {
        invariant(Object.keys(scenario.runnerFiveRoundSummaries ?? {}).length === 0,
          "WEB06_INDEPENDENT_PREVIEW_FIVE_ROUND_SUMMARY", scenario.scenarioRunId);
      }
    }
    const hardRedObserved = attemptResults.some((attempt) => attempt.classification === "RED");
    const verdict = measuredCount === requiredMeasuredCount
      ? (hardRedObserved ? "RED" : "PASS")
      : collector.expectation === "PREVIEW"
        ? "SETUP_INVALID"
        : "SETUP_NO_GO";
    const preservedHardRedAttemptIds = attemptResults
      .filter((attempt) => attempt.retainedHardRed === true)
      .map((attempt) => attempt.attemptId);
    if (scenario.verdict !== undefined) invariant(scenario.verdict === verdict,
      "WEB06_INDEPENDENT_SCENARIO_VERDICT", scenario.scenarioRunId);
    scenarioResults.push({
      targetId: scenario.targetId,
      scenarioRunId: scenario.scenarioRunId,
      scenarioId: scenario.scenarioId,
      schemaId: scenario.schemaId,
      measuredRoundCount: measuredCount,
      validLatencyFrameRoundCount: validLatencyFrameCount,
      verdict,
      preservedHardRedAttemptIds,
      preservedHardRedObserved: preservedHardRedAttemptIds.length > 0,
      attemptResults,
      fiveRoundSummaries,
      fiveRoundDiagnosticBindingPairs,
      fiveRoundSummarySha256: sha256(Buffer.from(JSON.stringify(fiveRoundSummaries), "utf8")),
    });
  }
  if (collector.expectation === "OBSERVER") {
    for (const triplet of collector.observerTriplets ?? []) {
      independentlyAuditObserverTripletRaw(triplet, rawEnvelopesByPath);
    }
  }
  const observerEvaluation = collector.expectation === "OBSERVER"
    ? independentlyEvaluateObserverTriplets(collector.observerTriplets ?? [])
    : undefined;
  if (collector.expectation === "OBSERVER") {
    invariant(exactJson(observerEvaluation, collector.observerEvaluation),
      "WEB06_INDEPENDENT_OBSERVER_EVALUATION_MISMATCH");
  } else {
    invariant((collector.observerTriplets ?? []).length === 0 && collector.observerEvaluation === undefined,
      "WEB06_INDEPENDENT_OBSERVER_LANE_CONTAMINATION");
  }
  return {
    version: "web06-independent-recompute-v1",
    writeMode: "create-new",
    collectorOutputSha256: collector.collectorOutputSha256,
    expectation: provenance.expectation,
    disposition: provenance.disposition,
    selectedBranch: provenance.selectedBranch,
    identityManifestSha256: provenance.identityManifestSha256,
    collectorContractSha256: provenance.collectorContractSha256,
    environmentManifestSha256: provenance.environmentManifestSha256,
    environmentId: provenance.environmentId,
    scenarioResults,
    observerTriplets: collector.observerTriplets ?? [],
    ...(observerEvaluation === undefined ? {} : { observerEvaluation }),
    verificationStatus: "PASS",
  };
}

const INDEPENDENT_ROUND_KEYS = Object.freeze([
  "version", "surface", "metricContractVersion", "scenarioRegistryVersion", "behaviorPredicateVersion",
  "sourceCommit", "sourceTree", "archiveSha256", "buildInfoSha256", "artifactManifestSha256",
  "artifactResponseGuardSha256", "artifactResponseGuardSummarySha256", "identityManifestSha256",
  "runnerSourceManifestSha256", "runnerToolingManifestSha256", "runnerSourceObservationSha256",
  "runnerSourcePostObservationSha256", "observedEnvironmentSha256", "collectorContractSha256",
  "environmentManifestSha256", "environmentId", "selectedBranch", "disposition", "scenarioRunId", "scenarioId",
  "schemaId", "roundId", "attemptId", "mode", "measurementStarted", "measurementCompleted", "status",
  "retryEligible", "validRedObserved", "setupErrorCodes", "behaviorErrorCodes", "thresholdViolations", "counts",
  "outcomeCounts", "cadenceVerdict", "cadence", "components", "queue", "burst", "frame", "longTask",
  "semanticProjectionVersion", "semanticProjectionSha256", "implementationDiagnosticBinding", "decision",
]);
const INDEPENDENT_COMPONENT_KEYS = new Set([
  "eventDeliveryMs", "eventHandlerEnqueueMs", "mainQueueWaitMs", "workerMessageDeliveryMs",
  "workerPreActionWaitMs", "workerDispatchWaitMs", "workerProcessMs", "workerRoundtripMs", "workerAbiMs",
  "workerAdapterTranslateMs", "workerJsonParseMs", "workerResponseExtractMs", "mainResponseDispatchMs",
  "responseMappingMs", "stateScheduleMs", "reactCommitMs", "paintProxyMs", "eventToCurrentPaintMs",
  "handlerToCurrentPaintMs", "timelineResidualMs", "preServiceWaitMs", "eventToObservationMs",
  "driverDispatchToObservationUpperBoundMs", "eventToCoveringPaintMs", "driverDispatchToCoveringPaintUpperBoundMs",
  "eventToTerminalObservationMs", "driverDispatchToTerminalUpperBoundMs", "preServiceWaitUpperBoundMs",
  "supersessionSequenceLag", "supersessionTimeMs",
]);
const INDEPENDENT_FORBIDDEN_KEY = /(?:(?:^|[_-])ptr(?:$|[_-])|pointer|address|authorization|cookie|password|secret|stack|browserProfile|userDataDir|(?:access|auth|bearer|api)Token)/i;
const INDEPENDENT_ABSOLUTE_PATH = /(?:^|[\s"'=(:,\[])(?:file:\/\/|\/(?:[^/\s()[\]{},;]+\/)*[^/\s()[\]{},;]+|[A-Za-z]:[\\/]|\\\\[^\\\s]+\\)/;
const INDEPENDENT_POINTER_VALUE = /(?:^|[\s"'=(:,\[])0x[0-9a-f]{6,}(?=$|[\s"',);}\]])/i;
const INDEPENDENT_SENSITIVE_VALUE = /(?:\bBearer[ \t]+[A-Za-z0-9._~+/=-]+|\bsk-proj-[A-Za-z0-9_-]+|https?:\/\/[^/\s:@]+:[^/\s@]+@)/i;
const INDEPENDENT_SUSPICIOUS_ENCODED_PRIVACY =
  /(?:https?%|file%|bearer%|sk(?:-|%(?:25)*2d)proj|%(?:25)*(?:2f|5c|3a|40))/i;
const INDEPENDENT_PRIVACY_PERCENT_DECODE_MAX_PASSES = 16;
const INDEPENDENT_PRIVACY_STRING_MAX_LENGTH = 65_536;

function independentPrivacyStringInvalid(value) {
  if (value.length > INDEPENDENT_PRIVACY_STRING_MAX_LENGTH) return true;
  const candidates = [value];
  let current = value;
  let malformedSuspiciousEncoding = false;
  let decodeBoundExceeded = false;
  for (let pass = 0; current.includes("%"); pass += 1) {
    if (pass >= INDEPENDENT_PRIVACY_PERCENT_DECODE_MAX_PASSES) {
      decodeBoundExceeded = true;
      break;
    }
    try {
      const decoded = decodeURIComponent(current);
      if (decoded === current || decoded.length >= current.length) {
        malformedSuspiciousEncoding = true;
        break;
      }
      candidates.push(decoded);
      current = decoded;
    } catch {
      malformedSuspiciousEncoding = INDEPENDENT_SUSPICIOUS_ENCODED_PRIVACY.test(value)
        || INDEPENDENT_SUSPICIOUS_ENCODED_PRIVACY.test(current)
        || (/%[0-9a-f]{2}/i.test(current) && /%(?![0-9a-f]{2})/i.test(current));
      break;
    }
  }
  if (!decodeBoundExceeded && current.includes("%") && (INDEPENDENT_SUSPICIOUS_ENCODED_PRIVACY.test(value)
    || INDEPENDENT_SUSPICIOUS_ENCODED_PRIVACY.test(current))) {
    malformedSuspiciousEncoding = true;
  }
  return decodeBoundExceeded || malformedSuspiciousEncoding || candidates.some((candidate) => {
    const webUrl = /^https?:\/\//i.test(candidate);
    return (!webUrl && INDEPENDENT_ABSOLUTE_PATH.test(candidate))
      || INDEPENDENT_POINTER_VALUE.test(candidate) || INDEPENDENT_SENSITIVE_VALUE.test(candidate);
  });
}

function independentEventRule(id) {
  const rule = EVENT_ACTION_RULES.find((candidate) => candidate.id === id);
  invariant(rule !== undefined, "WEB06_INDEPENDENT_EVENT_RULE_MISSING", id);
  return rule;
}

function independentKeyboardEventExpectation(step, type) {
  const primary = step.actions[0];
  if (primary !== undefined) {
    invariant(hasOwn(ACTION_REGISTRY, primary.kind),
      "WEB06_INDEPENDENT_ACTION_REGISTRY_MISSING", primary.kind);
  }
  if (step.code.startsWith("Shift")) {
    const rule = independentEventRule(type === "keydown" ? "modifier-tap-keydown" : "modifier-tap-keyup");
    return type === "keydown"
      ? { classification: rule.classification.startsWith("frontend-consumed")
        ? "frontend-consumed" : rule.classification, reason: "ascii-mode-shift-keydown" }
      : { classification: rule.classification, reason: "ascii-mode-shift-tap" };
  }
  if (type === "keyup") {
    if (/^[0-9]$/.test(step.key) && primary?.kind === "selectCandidate") {
      const rule = independentEventRule("digit-selection-keyup");
      return {
        classification: rule.classification.startsWith("frontend-consumed")
          ? "frontend-consumed" : rule.classification,
        reason: "composition-digit-keyup-follows-keydown",
      };
    }
    return {
      classification: independentEventRule("printable-keyup").classification,
      reason: "unmapped-keyup",
    };
  }
  if (primary?.kind === "selectCandidate") {
    return {
      classification: independentEventRule("digit-selection-keydown").classification,
      reason: "composition-digit-selection",
    };
  }
  if (primary?.kind === "processKey") {
    const rule = independentEventRule(primary.supersedable
      ? "printable-keydown"
      : primary.args[0] === "{Escape}" ? "escape-cancel-keydown"
        : primary.args[0] === "{BackSpace}" || primary.args[0] === "{Delete}"
          ? "backspace-delete-keydown"
          : primary.args[0] === "{Page_Down}" || primary.args[0] === "{Page_Up}"
            || primary.args[0] === "{Down}" || primary.args[0] === "{Up}"
            ? "arrow-page-keydown" : "punctuation-commit-keydown");
    if (primary.supersedable) {
      return { classification: rule.classification, reason: "printable-key" };
    }
    const rimeKey = String(primary.args[0] ?? "").replace(/^\{|\}$/g, "");
    const boundary = rimeKey === "BackSpace" || rimeKey === "Delete"
      ? "correction"
      : rimeKey === "Page_Down" || rimeKey === "Page_Up" || rimeKey === "Down" || rimeKey === "Up"
        ? "paging"
        : rimeKey === "space" || rimeKey === "Return"
          ? "commit"
          : rimeKey === "Escape"
            ? "cancel"
            : "none";
    return { classification: rule.classification, reason: `rime-key:${boundary}` };
  }
  return { classification: "browser-pass-through", reason: "unmapped-keydown" };
}

export function independentExpandScenarioExpectedTimeline(scenarioId) {
  invariant(hasOwn(SCENARIO_REGISTRY, scenarioId), `WEB06_UNKNOWN_SCENARIO:${scenarioId}`);
  const row = SCENARIO_REGISTRY[scenarioId];
  let eventSequenceId = 0;
  let sequenceId = 0;
  const events = [];
  const actions = [];
  for (const step of row.steps) {
    const addAction = (expected, eventId, eventReason) => {
      invariant(hasOwn(ACTION_REGISTRY, expected.kind),
        "WEB06_INDEPENDENT_ACTION_REGISTRY_MISSING", expected.kind);
      sequenceId += 1;
      const actionId = `a${sequenceId}`;
      const causedBy = expected.background === true
        ? [...actions].reverse().find((candidate) =>
          candidate.stepId === step.id && candidate.background !== true)
        : undefined;
      actions.push({
        ...expected,
        actionId,
        sequenceId,
        eventSequenceId: expected.background === true ? undefined : eventId,
        originKind: expected.background === true ? "background" : "dom-event",
        originReason: expected.background === true ? expected.originReason : eventReason,
        causedByActionId: causedBy?.actionId,
        causedBySequenceId: causedBy?.sequenceId,
        causedByEventSequenceId: expected.background === true ? eventId : undefined,
        stepId: step.id,
      });
      return { actionId, background: expected.background === true };
    };
    if (step.source === "keyboard") {
      const keydownId = ++eventSequenceId;
      const keyupId = ++eventSequenceId;
      const actionEventId = step.code === "ShiftLeft" || step.code === "ShiftRight" ? keyupId : keydownId;
      const keydown = independentKeyboardEventExpectation(step, "keydown");
      const keyup = independentKeyboardEventExpectation(step, "keyup");
      events.push({
        eventSequenceId: keydownId,
        stepId: step.id,
        type: "keydown",
        key: step.key,
        code: step.code,
        classification: keydown.classification,
        reason: keydown.reason,
        mappedActionIds: [],
      }, {
        eventSequenceId: keyupId,
        stepId: step.id,
        type: "keyup",
        key: step.key,
        code: step.code,
        classification: keyup.classification,
        reason: keyup.reason,
        mappedActionIds: [],
      });
      for (const expected of step.actions) {
        const added = addAction(expected, actionEventId, events[actionEventId - 1].reason);
        if (!added.background) events[actionEventId - 1].mappedActionIds.push(added.actionId);
      }
      continue;
    }
    const eventId = ++eventSequenceId;
    const event = {
      eventSequenceId: eventId,
      stepId: step.id,
      type: step.domEventType ?? step.source,
      key: "",
      code: "",
      classification: step.actions.length ? "mapped-action(s)" : "browser-pass-through",
      reason: step.eventReason ?? step.id,
      mappedActionIds: [],
    };
    events.push(event);
    for (const expected of step.actions) {
      const added = addAction(expected, eventId, event.reason);
      if (!added.background) event.mappedActionIds.push(added.actionId);
    }
  }
  return { events, actions };
}

function independentSchemaExact(value, keys, errors, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    errors.push(`${label}:object`);
    return false;
  }
  if (!exactJson(Object.keys(value).sort(), [...keys].sort())) errors.push(`${label}:keys`);
  return true;
}

function independentSchemaStrings(value, errors, label) {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) errors.push(`${label}:strings`);
}

function independentSchemaDiagnosticBinding(value, errors, label, scope) {
  const fields = scope === "round"
    ? INDEPENDENT_ROUND_DIAGNOSTIC_FIELDS
    : INDEPENDENT_FIVE_ROUND_DIAGNOSTIC_FIELDS;
  const keys = ["version", "dimensions", ...(scope === "five-round" ? ["roundBindingsSha256"] : [])];
  if (!independentSchemaExact(value, keys, errors, label)) return;
  if (value.version !== WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION
    || (scope === "five-round" && !SHA64.test(value.roundBindingsSha256 ?? ""))) {
    errors.push(`${label}:identity`);
  }
  if (!independentSchemaExact(value.dimensions, fields, errors, `${label}.dimensions`)) return;
  for (const field of fields) {
    const dimension = value.dimensions[field];
    if (!independentSchemaExact(dimension,
      ["rawCount", "rawSha256", "semanticCount", "semanticSha256"], errors, `${label}.dimensions.${field}`)) {
      continue;
    }
    if (!Number.isSafeInteger(dimension.rawCount) || dimension.rawCount < 0
      || !Number.isSafeInteger(dimension.semanticCount) || dimension.semanticCount < 0
      || dimension.semanticCount > dimension.rawCount
      || !SHA64.test(dimension.rawSha256 ?? "") || !SHA64.test(dimension.semanticSha256 ?? "")) {
      errors.push(`${label}.dimensions.${field}:values`);
    }
  }
}

function independentSchemaDiagnosticBindingPair(value, errors, label, expectedScope) {
  const keys = ["version", "scope", "runnerBinding", "independentBinding",
    "runnerSemanticProjectionSha256", "independentSemanticProjectionSha256",
    "runnerDecisionDimensionsSha256", "independentDecisionDimensionsSha256", "equivalent", "bindingPairSha256"];
  if (!independentSchemaExact(value, keys, errors, label)) return;
  if (value.version !== "web06-independent-diagnostic-binding-pair-v1"
    || value.scope !== expectedScope || value.equivalent !== true
    || !SHA64.test(value.runnerSemanticProjectionSha256 ?? "")
    || value.runnerSemanticProjectionSha256 !== value.independentSemanticProjectionSha256
    || !SHA64.test(value.runnerDecisionDimensionsSha256 ?? "")
    || value.runnerDecisionDimensionsSha256 !== value.independentDecisionDimensionsSha256
    || !SHA64.test(value.bindingPairSha256 ?? "")) {
    errors.push(`${label}:equivalence`);
  }
  const { bindingPairSha256, ...pair } = value;
  if (bindingPairSha256 !== sha256(Buffer.from(JSON.stringify(canonicalProjection(pair)), "utf8"))) {
    errors.push(`${label}:binding-pair-hash`);
  }
  independentSchemaDiagnosticBinding(value.runnerBinding, errors, `${label}.runnerBinding`, expectedScope);
  independentSchemaDiagnosticBinding(value.independentBinding, errors, `${label}.independentBinding`, expectedScope);
}

/** Owning schema check for the raw runner/independent binding pair. */
export function validateIndependentDiagnosticBindingPairSchema(value, scope) {
  const errors = [];
  if (!["round", "five-round"].includes(scope)) {
    errors.push("diagnosticBindingPair:scope");
  } else {
    independentSchemaDiagnosticBindingPair(value, errors, "diagnosticBindingPair", scope);
  }
  independentSchemaPrivacy(value, "diagnosticBindingPair", errors);
  return { pass: errors.length === 0, errors };
}

function independentSchemaDistribution(value, errors, label, nullable = true) {
  if (value === null && nullable) return;
  if (!independentSchemaExact(value, ["count", "median", "p95", "p99", "max"], errors, label)) return;
  if (!Number.isSafeInteger(value.count) || value.count <= 0
    || [value.median, value.p95, value.p99, value.max].some((item) => !finite(item) || item < 0)) {
    errors.push(`${label}:values`);
  } else if (!(value.median <= value.p95 && value.p95 <= value.p99 && value.p99 <= value.max)) {
    errors.push(`${label}:order`);
  }
}

function independentSchemaRoundSummary(value, errors, label) {
  if (!independentSchemaExact(value, INDEPENDENT_ROUND_KEYS, errors, label)) return;
  if (value.version !== "web06-round-summary-v1" || !["common", "internal"].includes(value.surface)
    || value.metricContractVersion !== WEB06_METRIC_CONTRACT_VERSION
    || value.scenarioRegistryVersion !== WEB06_SCENARIO_REGISTRY_VERSION
    || value.behaviorPredicateVersion !== WEB06_BEHAVIOR_PREDICATE_VERSION
    || !["PASS", "RED", "RED_BEHAVIOR", "SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(value.status)
    || !["IN_RANGE", "TOO_SHORT", "TOO_LONG", "NOT_APPLICABLE"].includes(value.cadenceVerdict)) {
    errors.push(`${label}:identity`);
  }
  try {
    independentlyAssertSummaryIntegrity(value, label);
  } catch {
    errors.push(`${label}:summary-integrity`);
  }
  for (const key of ["sourceCommit", "sourceTree"]) if (!SHA40.test(value[key] ?? "")) errors.push(`${label}.${key}:hash`);
  for (const key of ["archiveSha256", "buildInfoSha256", "artifactManifestSha256", "artifactResponseGuardSha256",
    "artifactResponseGuardSummarySha256", "identityManifestSha256", "runnerSourceManifestSha256",
    "runnerToolingManifestSha256", "runnerSourceObservationSha256", "runnerSourcePostObservationSha256",
    "observedEnvironmentSha256", "collectorContractSha256", "environmentManifestSha256"]) {
    if (!SHA64.test(value[key] ?? "")) errors.push(`${label}.${key}:hash`);
  }
  for (const key of ["measurementStarted", "measurementCompleted", "retryEligible", "validRedObserved"]) {
    if (typeof value[key] !== "boolean") errors.push(`${label}.${key}:boolean`);
  }
  for (const key of ["setupErrorCodes", "behaviorErrorCodes", "thresholdViolations"]) {
    independentSchemaStrings(value[key], errors, `${label}.${key}`);
  }
  if (independentSchemaExact(value.counts,
    ["events", "actions", "coveringSamples", "terminalSamples", "unclassifiedSamples", "interactionWindows"],
  errors, `${label}.counts`)
    && Object.values(value.counts).some((item) => !Number.isSafeInteger(item) || item < 0)) errors.push(`${label}.counts:values`);
  if (independentSchemaExact(value.outcomeCounts, INDEPENDENT_OUTCOME_COUNT_KEYS,
    errors, `${label}.outcomeCounts`)
    && Object.values(value.outcomeCounts).some((item) => !Number.isSafeInteger(item) || item < 0)) {
    errors.push(`${label}.outcomeCounts:values`);
  }
  if (independentSchemaExact(value.cadence,
    ["total", "inRange", "tooShort", "tooLong", "delayedHost"], errors, `${label}.cadence`)
    && Object.values(value.cadence).some((item) => !Number.isSafeInteger(item) || item < 0)) {
    errors.push(`${label}.cadence:values`);
  }
  if (!value.components || typeof value.components !== "object" || Array.isArray(value.components)) {
    errors.push(`${label}.components:object`);
  } else for (const [name, summary] of Object.entries(value.components)) {
    if (!INDEPENDENT_COMPONENT_KEYS.has(name)) errors.push(`${label}.components.${name}:unknown`);
    independentSchemaDistribution(summary, errors, `${label}.components.${name}`, false);
  }
  if (independentSchemaExact(value.queue, ["maxDepth", "endBurstDepth"], errors, `${label}.queue`)
    && Object.values(value.queue).some((item) => item !== null && (!Number.isSafeInteger(item) || item < 0))) {
    errors.push(`${label}.queue:values`);
  }
  if (independentSchemaExact(value.burst, ["recoveryCount", "finalKeyToLatestPaintMs"], errors, `${label}.burst`)) {
    if (!Number.isSafeInteger(value.burst.recoveryCount) || value.burst.recoveryCount < 0) errors.push(`${label}.burst:count`);
    independentSchemaDistribution(value.burst.finalKeyToLatestPaintMs, errors, `${label}.burst.finalKeyToLatestPaintMs`);
  }
  if (independentSchemaExact(value.frame, ["intervals", "atOrAbove50MsCount"], errors, `${label}.frame`)) {
    independentSchemaDistribution(value.frame.intervals, errors, `${label}.frame.intervals`);
    if (!Number.isSafeInteger(value.frame.atOrAbove50MsCount) || value.frame.atOrAbove50MsCount < 0) {
      errors.push(`${label}.frame:count`);
    }
  }
  if (independentSchemaExact(value.longTask, ["count", "durationMs", "overlapCount", "overlapDurationMs",
    "idleControlCount", "idleControlDurationMs", "interactionMinusIdleCount", "interactionMinusIdleDurationMs"],
  errors, `${label}.longTask`)) {
    for (const key of ["count", "overlapCount", "idleControlCount", "interactionMinusIdleCount"]) {
      if (!Number.isSafeInteger(value.longTask[key])) errors.push(`${label}.longTask.${key}:integer`);
    }
    for (const key of ["durationMs", "overlapDurationMs", "idleControlDurationMs"]) {
      independentSchemaDistribution(value.longTask[key], errors, `${label}.longTask.${key}`);
    }
    if (!finite(value.longTask.interactionMinusIdleDurationMs)) errors.push(`${label}.longTask:delta`);
  }
  const row = hasOwn(SCENARIO_REGISTRY, value.scenarioId)
    ? SCENARIO_REGISTRY[value.scenarioId]
    : undefined;
  if (!row) {
    errors.push(`${label}:scenario`);
    return;
  }
  const binding = row.binding === true || row.binding === "branch-b-only";
  const frameRed = binding && ((value.frame?.atOrAbove50MsCount ?? 0) > 0
    || (value.frame?.intervals?.p99 ?? -Infinity) > WEB06_THRESHOLDS.frame.p99Ms.max);
  const longTaskRed = binding
    && (value.longTask?.overlapDurationMs?.max ?? -Infinity) >= WEB06_THRESHOLDS.frame.rejectLongTaskAtOrAboveMs;
  const expectedStatus = independentStatus({
    setupErrors: value.setupErrorCodes ?? [],
    behaviorErrors: value.behaviorErrorCodes ?? [],
    thresholdViolations: value.thresholdViolations ?? [],
    frameRed,
    longTaskRed,
    cadence: value.cadenceVerdict,
  });
  const expectedDecision = {
    setupInvalid: (value.setupErrorCodes?.length ?? 0) > 0,
    behaviorRed: (value.behaviorErrorCodes?.length ?? 0) > 0,
    thresholdRed: (value.setupErrorCodes?.length ?? 0) === 0
      && value.cadenceVerdict !== "TOO_SHORT"
      && (value.thresholdViolations?.length ?? 0) > 0,
    frameRed,
    longTaskRed,
    cadenceInvalid: ["TOO_SHORT", "TOO_LONG"].includes(value.cadenceVerdict),
    status: expectedStatus,
    retryEligible: ["SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(expectedStatus),
    validRedObserved: ["RED", "RED_BEHAVIOR"].includes(expectedStatus),
  };
  if (!exactJson(value.decision, expectedDecision)
    || value.status !== expectedDecision.status
    || value.retryEligible !== expectedDecision.retryEligible
    || value.validRedObserved !== expectedDecision.validRedObserved) {
    errors.push(`${label}:decision-link`);
  }
}

function independentSchemaFiveRound(value, errors, label) {
  const keys = ["version", "surface", "scenarioRunId", "scenarioId", "schemaId", "mode",
    "environmentManifestSha256", "environmentId", "roundCount", "status", "validRedObserved",
    "roundSummaries", "pooledComponents", "pooledMetricsSha256", "summaryErrors", "poolViolations", "pooledFrame",
    "pooledLongTaskCount", "pooledIdleLongTaskCount",
    "semanticProjectionVersion", "semanticProjectionSha256", "implementationDiagnosticBinding", "decision"];
  if (!independentSchemaExact(value, keys, errors, label)) return;
  if (value.version !== "web06-five-round-summary-v1" || !["common", "internal"].includes(value.surface)
    || value.roundCount !== 5 || !["PASS", "RED"].includes(value.status)
    || typeof value.validRedObserved !== "boolean" || !SHA64.test(value.environmentManifestSha256 ?? "")
    || !SHA64.test(value.pooledMetricsSha256 ?? "")
    || typeof value.environmentId !== "string" || value.environmentId.length === 0) errors.push(`${label}:identity`);
  try {
    independentlyAssertSummaryIntegrity(value, label);
  } catch {
    errors.push(`${label}:summary-integrity`);
  }
  if (!Array.isArray(value.roundSummaries) || value.roundSummaries.length !== 5) errors.push(`${label}.roundSummaries:count`);
  else value.roundSummaries.forEach((row, index) => {
    independentSchemaRoundSummary(row, errors, `${label}.roundSummaries[${index}]`);
    if (row.surface !== value.surface || row.scenarioRunId !== value.scenarioRunId
      || row.scenarioId !== value.scenarioId || row.schemaId !== value.schemaId || row.mode !== value.mode
      || row.environmentManifestSha256 !== value.environmentManifestSha256
      || row.environmentId !== value.environmentId) errors.push(`${label}.roundSummaries[${index}]:identity-link`);
  });
  if (!value.pooledComponents || typeof value.pooledComponents !== "object" || Array.isArray(value.pooledComponents)) {
    errors.push(`${label}.pooledComponents:object`);
  } else for (const [name, summary] of Object.entries(value.pooledComponents)) {
    if (!INDEPENDENT_COMPONENT_KEYS.has(name)) errors.push(`${label}.pooledComponents.${name}:unknown`);
    independentSchemaDistribution(summary, errors, `${label}.pooledComponents.${name}`, false);
  }
  independentSchemaStrings(value.summaryErrors, errors, `${label}.summaryErrors`);
  independentSchemaStrings(value.poolViolations, errors, `${label}.poolViolations`);
  independentSchemaDistribution(value.pooledFrame, errors, `${label}.pooledFrame`);
  if (!Number.isSafeInteger(value.pooledLongTaskCount) || value.pooledLongTaskCount < 0
    || !Number.isSafeInteger(value.pooledIdleLongTaskCount) || value.pooledIdleLongTaskCount < 0) {
    errors.push(`${label}:long-task-counts`);
  }
  const expectedStatus = value.roundSummaries?.every((round) => round.status === "PASS")
    && value.summaryErrors?.length === 0 && value.poolViolations?.length === 0 ? "PASS" : "RED";
  const expectedValidRed = value.roundSummaries?.some((round) => round.validRedObserved === true)
    || (value.summaryErrors?.length ?? 0) > 0 || (value.poolViolations?.length ?? 0) > 0;
  if (value.status !== expectedStatus || value.validRedObserved !== expectedValidRed) {
    errors.push(`${label}:disposition-link`);
  }
  const expectedDecision = {
    roundRed: value.roundSummaries?.some((round) => round.status !== "PASS") === true,
    poolViolationRed: (value.poolViolations?.length ?? 0) > 0,
    summaryErrorRed: (value.summaryErrors?.length ?? 0) > 0,
    status: expectedStatus,
    validRedObserved: expectedValidRed,
  };
  if (!exactJson(value.decision, expectedDecision)) errors.push(`${label}:decision-link`);
  const expectedPooledHash = sha256(Buffer.from(JSON.stringify({
    pooledComponents: value.pooledComponents,
    pooledFrame: value.pooledFrame,
    pooledLongTaskCount: value.pooledLongTaskCount,
    pooledIdleLongTaskCount: value.pooledIdleLongTaskCount,
  }), "utf8"));
  if (value.pooledMetricsSha256 !== expectedPooledHash) errors.push(`${label}:pooled-metrics-hash`);
}

function independentSchemaObserverCallback(value, errors, label, modeName) {
  const sentinel = value?.sourceClass === "common-sentinel";
  const required = sentinel
    ? ["kind", "pageInstanceId", "callbackId", "sequenceId", "windowIndex",
      "startedAt", "finishedAt", "durationMs", "sourceClass"]
    : ["callbackId", "sequenceId", "operation", "startedAt", "finishedAt", "durationMs", "sourceClass"];
  const optional = sentinel ? ["eventSequenceId"] : ["actionId", "eventId", "pageInstanceId"];
  if (!value || typeof value !== "object" || Array.isArray(value)
    || required.some((key) => !Object.hasOwn(value, key))
    || Object.keys(value).some((key) => !required.includes(key) && !optional.includes(key))) {
    errors.push(`${label}:keys`);
    return;
  }
  const privateClass = modeName === "minimal" ? "minimal-probe" : modeName === "full" ? "full-collector" : undefined;
  if ((!sentinel && value.sourceClass !== privateClass) || (sentinel && modeName === undefined)) errors.push(`${label}:source-class`);
  for (const key of ["callbackId", "sourceClass", ...(sentinel ? ["kind", "pageInstanceId"] : ["operation"])]) {
    if (typeof value[key] !== "string" || value[key].length === 0) errors.push(`${label}.${key}:string`);
  }
  for (const key of ["actionId", "eventId", "pageInstanceId"]) {
    if (key in value && (typeof value[key] !== "string" || value[key].length === 0)) errors.push(`${label}.${key}:string`);
  }
  if (!Number.isSafeInteger(value.sequenceId)
    || ("eventSequenceId" in value && !Number.isSafeInteger(value.eventSequenceId))
    || (sentinel && (!Number.isSafeInteger(value.windowIndex) || value.windowIndex < -1))
    || !finite(value.startedAt) || !finite(value.finishedAt)
    || !finite(value.durationMs)) errors.push(`${label}:values`);
}

function independentObserverLedgerSemanticValidity(value, modeName) {
  if (!Array.isArray(value?.callbackIntervals)) return false;
  const sentinelRows = value.callbackIntervals.filter((row) => row?.sourceClass === "common-sentinel");
  const privateRows = value.callbackIntervals.filter((row) => row?.sourceClass !== "common-sentinel");
  const nonnegativeNumbers = (rows) => Array.isArray(rows)
    && rows.every((item) => finite(item) && item >= 0);
  const rowSemantics = value.callbackIntervals.every((row) =>
    row && typeof row === "object" && !Array.isArray(row)
    && typeof row.callbackId === "string" && row.callbackId.length > 0
    && Number.isSafeInteger(row.sequenceId) && row.sequenceId > 0
    && finite(row.startedAt) && row.startedAt >= 0
    && finite(row.finishedAt) && row.finishedAt >= row.startedAt
    && finite(row.durationMs) && row.durationMs >= 0
    && row.durationMs === row.finishedAt - row.startedAt
    && (row.sourceClass === "common-sentinel"
      ? typeof row.kind === "string" && row.kind.length > 0
        && Number.isSafeInteger(row.windowIndex) && row.windowIndex >= -1
        && row.windowIndex < value.interactionWindowCount
        && (row.eventSequenceId === undefined
          || (Number.isSafeInteger(row.eventSequenceId)
            && row.eventSequenceId > 0 && row.eventSequenceId <= value.commonEventCount))
      : row.sourceClass === (modeName === "minimal" ? "minimal-probe"
        : modeName === "full" ? "full-collector" : undefined)
        && typeof row.operation === "string" && row.operation.length > 0));
  const ordered = (rows) => new Set(rows.map((row) => row.callbackId)).size === rows.length
    && new Set(rows.map((row) => row.sequenceId)).size === rows.length
    && rows.every((row, index) => index === 0
      || (row.sequenceId > rows[index - 1].sequenceId
        && row.finishedAt >= rows[index - 1].finishedAt));
  const sentinelConserved = Number.isSafeInteger(value?.callbackLedgerCount)
    && value.callbackLedgerCount === sentinelRows.length
    && Number.isSafeInteger(value.callbackLedgerCapacity)
    && value.callbackLedgerCount <= value.callbackLedgerCapacity
    && value.sentinelAccountedCallbackCount === sentinelRows.length
    && value.callbackLedgerOverflowCount === 0
    && Number.isSafeInteger(value.commonEventCount) && value.commonEventCount >= 0
    && (value.commonEventCount === 0 || sentinelRows.length > 0)
    && nonnegativeNumbers(value.sentinelCallbacksMs)
    && exactJson(value.sentinelCallbacksMs, sentinelRows.map((row) => row.durationMs));
  const privateConserved = modeName === "product"
    ? privateRows.length === 0 && Array.isArray(value.mainObserverCallbacksMs)
      && value.mainObserverCallbacksMs.length === 0
      && Array.isArray(value.workerCollectorCallbacksMs)
      && value.workerCollectorCallbacksMs.length === 0
    : Number.isSafeInteger(value?.mainObserverCallbackCount)
      && Number.isSafeInteger(value?.mainObserverCallbackCapacity)
      && value.mainObserverCallbackCount === privateRows.length
      && value.mainObserverCallbackCount <= value.mainObserverCallbackCapacity
      && value.mainObserverCallbackOverflowCount === 0
      && nonnegativeNumbers(value.mainObserverCallbacksMs)
      && exactJson(value.mainObserverCallbacksMs, privateRows.map((row) => row.durationMs));
  const rawTaskSemantics = Array.isArray(value?.rawLongTasks)
    && value.rawLongTasks.every((task) => finite(task?.startTime) && task.startTime >= 0
      && finite(task?.durationMs) && task.durationMs >= 0
      && task.overlapsInteractionWindow === true);
  if (!nonnegativeNumbers(value?.workerCollectorCallbacksMs)
    || !nonnegativeNumbers(value?.collectorCallbacksMs)
    || !nonnegativeNumbers(value?.sentinelTotalPerEventMs)
    || !nonnegativeNumbers(value?.sentinelTotalPerWindowMs)
    || !Number.isSafeInteger(value?.interactionWindowCount) || value.interactionWindowCount < 0) return false;
  const combinedDurations = [...value.mainObserverCallbacksMs, ...value.workerCollectorCallbacksMs]
    .sort((left, right) => left - right);
  const eventTotals = Array.from({ length: value.commonEventCount }, (_, index) => sentinelRows
    .filter((row) => row.eventSequenceId === index + 1)
    .reduce((sum, row) => sum + row.durationMs, 0));
  const windowTotals = Array.from({ length: value.interactionWindowCount }, (_, index) => sentinelRows
    .filter((row) => row.windowIndex === index)
    .reduce((sum, row) => sum + row.durationMs, 0));
  return rowSemantics && rawTaskSemantics
    && new Set(value.callbackIntervals.map((row) => row.callbackId)).size === value.callbackIntervals.length
    && ordered(sentinelRows) && ordered(privateRows)
    && sentinelConserved && privateConserved
    && exactJson(combinedDurations, [...value.collectorCallbacksMs].sort((left, right) => left - right))
    && exactJson(value.sentinelTotalPerEventMs, eventTotals)
    && exactJson(value.sentinelTotalPerWindowMs, windowTotals);
}

function independentSchemaObserverMode(value, errors, label, modeName) {
  const complete = value?.measurementCompleted === true;
  const keys = ["rawPacket", "measurementStarted", "measurementCompleted", "measurementValid",
    "behaviorRedObserved", "hardRedBindingValid", "hardRedObserved",
    "samples", ...(complete ? ["commonEquivalenceDigest"] : []), ...(complete && modeName !== "product"
      ? ["internalEquivalenceDigest"] : []), "commonVerdict", "internalVerdict",
    "commonEventCount", ...(complete ? ["environmentManifestSha256", "environmentId"] : []),
    "interactionWindowCount",
    "sentinelCallbacksMs", "sentinelTotalPerEventMs", "sentinelTotalPerWindowMs", "collectorCallbacksMs",
    "mainObserverCallbacksMs", "workerCollectorCallbacksMs", ...(complete ? ["callbackLedgerCount",
      "callbackLedgerCapacity", "sentinelAccountedCallbackCount", "callbackLedgerOverflowCount"] : []),
    "callbackAttributionComplete",
    ...(complete && modeName !== "product" ? ["mainObserverCallbackCount", "mainObserverCallbackCapacity",
      "mainObserverCallbackOverflowCount"] : []), "callbackIntervals", "rawLongTasks", "underlyingLongTasksMs",
    "instrumentationAddedLongTasksMs"];
  if (!independentSchemaExact(value, keys, errors, label)) return;
  if (!independentSchemaExact(value.rawPacket, ["relativePath", "bytes", "sha256"], errors, `${label}.rawPacket`)) return;
  if (typeof value.rawPacket.relativePath !== "string" || path.isAbsolute(value.rawPacket.relativePath)
    || value.rawPacket.relativePath.split("/").includes("..") || !Number.isSafeInteger(value.rawPacket.bytes)
    || value.rawPacket.bytes <= 0 || !SHA64.test(value.rawPacket.sha256 ?? "")) errors.push(`${label}.rawPacket:values`);
  for (const key of ["measurementStarted", "measurementCompleted", "measurementValid",
    "behaviorRedObserved", "hardRedBindingValid", "hardRedObserved", "callbackAttributionComplete"]) {
    if (typeof value[key] !== "boolean") errors.push(`${label}.${key}:boolean`);
  }
  if (!Array.isArray(value.samples) || value.samples.some((item) => !finite(item) || item < 0)) {
    errors.push(`${label}.samples:numbers`);
  }
  const signedInvalidEvidence = complete && value.callbackAttributionComplete === false
    && value.measurementValid === false;
  for (const key of ["sentinelCallbacksMs", "sentinelTotalPerEventMs", "sentinelTotalPerWindowMs",
    "collectorCallbacksMs", "mainObserverCallbacksMs", "workerCollectorCallbacksMs"]) {
    if (!Array.isArray(value[key])
      || value[key].some((item) => !finite(item) || (!signedInvalidEvidence && item < 0))) {
      errors.push(`${label}.${key}:numbers`);
    }
  }
  for (const key of ["underlyingLongTasksMs", "instrumentationAddedLongTasksMs"]) {
    if (!Array.isArray(value[key]) || value[key].some((item) => !finite(item) || item < 0)) {
      errors.push(`${label}.${key}:numbers`);
    }
  }
  for (const key of ["commonEventCount", "interactionWindowCount", "callbackLedgerCount", "callbackLedgerCapacity",
    "sentinelAccountedCallbackCount",
    "callbackLedgerOverflowCount", "mainObserverCallbackCount", "mainObserverCallbackCapacity",
    "mainObserverCallbackOverflowCount"]) {
    if (key in value && (!Number.isSafeInteger(value[key]) || value[key] < 0)) errors.push(`${label}.${key}:count`);
  }
  for (const key of ["commonEquivalenceDigest", "internalEquivalenceDigest", "environmentManifestSha256"]) {
    if (key in value && !SHA64.test(value[key] ?? "")) errors.push(`${label}.${key}:hash`);
  }
  if ("environmentId" in value && !SHA64.test(value.environmentId ?? "")) {
    errors.push(`${label}.environmentId:hash`);
  }
  for (const key of ["commonVerdict", "internalVerdict"]) {
    if (!["PASS", "RED", "RED_BEHAVIOR", "SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"]
      .includes(value[key])) errors.push(`${label}.${key}:verdict`);
  }
  if (!Array.isArray(value.callbackIntervals) || !Array.isArray(value.rawLongTasks)) errors.push(`${label}:ledgers`);
  else {
    value.callbackIntervals.forEach((row, index) => independentSchemaObserverCallback(row, errors,
      `${label}.callbackIntervals[${index}]`, modeName));
    value.rawLongTasks.forEach((row, index) => {
      if (!independentSchemaExact(row,
        ["startTime", "durationMs", "pageInstanceId", "overlapsInteractionWindow", "overlapsIdleControl", "locus"],
      errors, `${label}.rawLongTasks[${index}]`)) return;
      if (!finite(row.startTime) || !finite(row.durationMs)
        || typeof row.pageInstanceId !== "string" || typeof row.locus !== "string"
        || row.overlapsInteractionWindow !== true || typeof row.overlapsIdleControl !== "boolean") {
        errors.push(`${label}.rawLongTasks[${index}]:values`);
      }
    });
  }
  if (complete && !value.measurementStarted) errors.push(`${label}:measurement-start`);
  if (complete && value.callbackAttributionComplete === true
    && (value.sentinelTotalPerEventMs.length !== value.commonEventCount
      || value.sentinelTotalPerWindowMs.length !== value.interactionWindowCount)) {
    errors.push(`${label}:cardinality`);
  }
  const sentinelCallbacks = value.callbackIntervals?.filter((row) => row.sourceClass === "common-sentinel") ?? [];
  const privateCallbacks = value.callbackIntervals?.filter((row) => row.sourceClass !== "common-sentinel") ?? [];
  if (complete && (value.callbackLedgerCount !== sentinelCallbacks.length
    || value.sentinelAccountedCallbackCount !== sentinelCallbacks.length)
    && value.callbackAttributionComplete === true) {
    errors.push(`${label}:sentinel-callback-conservation`);
  }
  if (modeName === "product" && privateCallbacks.length !== 0) errors.push(`${label}:private-callback-present`);
  if (complete && value.callbackAttributionComplete === true && modeName !== "product"
    && (value.mainObserverCallbackCount !== privateCallbacks.length
      || value.mainObserverCallbackCount > value.mainObserverCallbackCapacity)) {
    errors.push(`${label}:private-callback-conservation`);
  }
  if (complete && (value.callbackLedgerOverflowCount > 0
    || (modeName !== "product" && value.mainObserverCallbackOverflowCount > 0))
    && value.callbackAttributionComplete !== false) {
    errors.push(`${label}:overflow-validity-link`);
  }
  if (complete && value.callbackAttributionComplete === true
    && !exactJson(value.mainObserverCallbacksMs, privateCallbacks.map((row) => row.durationMs))) {
    errors.push(`${label}:private-duration-ledger`);
  }
  if (complete && value.callbackAttributionComplete === true
    && !exactJson([...value.collectorCallbacksMs].sort((left, right) => left - right),
    [...value.mainObserverCallbacksMs, ...value.workerCollectorCallbacksMs].sort((left, right) => left - right))) {
    errors.push(`${label}:collector-duration-ledger`);
  }
  if (!complete && value.measurementValid) errors.push(`${label}:partial-valid`);
  const hardRedExpected = independentObserverModeHardRedExpected(value);
  if (value.hardRedObserved !== hardRedExpected) errors.push(`${label}:hard-red-verdict-link`);
  if (value.behaviorRedObserved === true
    && ![value.commonVerdict, value.internalVerdict]
      .some((verdict) => ["RED", "RED_BEHAVIOR"].includes(verdict))) {
    errors.push(`${label}:behavior-red-verdict-link`);
  }
  if ([value.commonVerdict, value.internalVerdict].includes("RED_BEHAVIOR")
    && value.behaviorRedObserved !== true) {
    errors.push(`${label}:behavior-red-observation-link`);
  }
  if (complete && value.hardRedBindingValid === true
    && (value.callbackAttributionComplete !== true
      || [value.commonVerdict, value.internalVerdict].includes("SETUP_INVALID"))) {
    errors.push(`${label}:hard-red-binding-validity-link`);
  }
  if (complete && value.measurementValid === true && value.hardRedBindingValid !== true) {
    errors.push(`${label}:measurement-hard-red-binding-link`);
  }
  if (complete && value.callbackAttributionComplete === false) {
    const expectedInvalidVerdict = value.behaviorRedObserved === true ? "RED_BEHAVIOR" : "SETUP_INVALID";
    if (value.commonVerdict !== expectedInvalidVerdict || value.internalVerdict !== expectedInvalidVerdict) {
      errors.push(`${label}:invalid-attribution-verdict-link`);
    }
  }
  if (complete && value.commonVerdict === "PASS" && value.internalVerdict === "PASS"
    && value.callbackAttributionComplete === true && value.measurementValid !== true) {
    errors.push(`${label}:clean-completed-validity-link`);
  }
  if (complete && value.callbackAttributionComplete === false && value.measurementValid !== false) {
    errors.push(`${label}:attribution-validity-link`);
  }
  if (complete && value.callbackAttributionComplete
    !== independentObserverLedgerSemanticValidity(value, modeName)) {
    errors.push(`${label}:callback-attribution-semantic-link`);
  }
  if (complete && value.rawLongTasks?.some((task) =>
    finite(task?.startTime) && finite(task?.durationMs)
    && (task.startTime < 0 || task.durationMs < 0))) {
    const expectedInvalidVerdict = value.behaviorRedObserved === true ? "RED_BEHAVIOR" : "SETUP_INVALID";
    if (value.commonVerdict !== expectedInvalidVerdict || value.internalVerdict !== expectedInvalidVerdict) {
      errors.push(`${label}:invalid-long-task-verdict-link`);
    }
  }
  if (complete && [value.commonVerdict, value.internalVerdict]
    .some((verdict) => ["SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(verdict))
    && value.measurementValid !== false) errors.push(`${label}:invalid-completed-validity-link`);
  if (!complete && (value.commonVerdict !== value.internalVerdict || value.callbackAttributionComplete !== false
    || value.hardRedBindingValid !== false
    || value.commonEventCount !== 0 || value.interactionWindowCount !== 0
    || ["samples", "sentinelCallbacksMs", "sentinelTotalPerEventMs", "sentinelTotalPerWindowMs",
      "collectorCallbacksMs", "mainObserverCallbacksMs", "workerCollectorCallbacksMs", "callbackIntervals",
      "rawLongTasks", "underlyingLongTasksMs", "instrumentationAddedLongTasksMs"]
      .some((key) => value[key]?.length !== 0))) errors.push(`${label}:partial-projection`);
  if (complete && Array.isArray(value.rawLongTasks)
    && Array.isArray(value.underlyingLongTasksMs) && Array.isArray(value.instrumentationAddedLongTasksMs)) {
    const thresholdDurations = value.rawLongTasks
      .filter((task) => finite(task?.durationMs)
        && task.overlapsInteractionWindow === true
        && task.durationMs >= WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs)
      .map((task) => task.durationMs).sort((left, right) => left - right);
    const classifiedDurations = [...value.underlyingLongTasksMs, ...value.instrumentationAddedLongTasksMs]
      .sort((left, right) => left - right);
    if (!exactJson(thresholdDurations, classifiedDurations)) {
      errors.push(`${label}:long-task-attribution-conservation`);
    }
  }
}

/** Verifier-owned exact nested schema for a compact observer-mode projection. */
export function independentlyValidateObserverModeProjectionSchema(value, modeName) {
  const errors = [];
  try {
    if (!["product", "minimal", "full"].includes(modeName)) errors.push("observer-mode:mode");
    else independentSchemaObserverMode(value, errors, "observer-mode", modeName);
  } catch {
    errors.push("observer-mode:schema-exception");
  }
  try {
    independentSchemaPrivacy(value, "", errors);
  } catch {
    errors.push("observer-mode:privacy-exception");
  }
  return { pass: errors.length === 0, errors };
}

function independentSchemaObserverTriplet(value, errors, label, index) {
  if (!independentSchemaExact(value, ["attemptId", "valid", "counterbalanceSlot", "freshContextId", "modeContextIds",
    "modeOrder", "modeFixedBeforePageLoad", "product", "minimal", "full"], errors, label)) return;
  if (value.attemptId !== `triplet-attempt-${index + 1}` || typeof value.valid !== "boolean"
    || !Number.isSafeInteger(value.counterbalanceSlot)
    || !hasOwn(WEB06_OBSERVER_COUNTERBALANCE, String(value.counterbalanceSlot))
    || typeof value.freshContextId !== "string" || !Array.isArray(value.modeContextIds)
    || value.modeContextIds.length !== 3 || value.modeContextIds.some((id) => typeof id !== "string" || !id)
    || new Set(value.modeContextIds).size !== 3 || value.freshContextId !== value.modeContextIds.join("+")
    || !exactJson(value.modeOrder, WEB06_OBSERVER_COUNTERBALANCE[value.counterbalanceSlot])
    || value.modeFixedBeforePageLoad !== true) errors.push(`${label}:identity`);
  for (const modeName of ["product", "minimal", "full"]) {
    independentSchemaObserverMode(value[modeName], errors, `${label}.${modeName}`, modeName);
  }
  if (value.valid !== [value.product, value.minimal, value.full].every((mode) => mode?.measurementValid === true)) {
    errors.push(`${label}:validity`);
  }
}

function independentSchemaObserverEvaluation(value, errors, label) {
  const complete = Array.isArray(value?.comparisons);
  const keys = complete ? ["pass", "status", "comparisons", "violations"]
    : ["pass", "status", "violations", "preservedUnpairedReds"];
  if (!independentSchemaExact(value, keys, errors, label)) return;
  if (typeof value.pass !== "boolean" || !Array.isArray(value.violations)
    || value.violations.some((row) => typeof row !== "string")) errors.push(`${label}:values`);
  if (complete) {
    if (!["PASS", "RED"].includes(value.status) || value.pass !== (value.status === "PASS")
      || value.comparisons.length !== 2) errors.push(`${label}:status`);
    value.comparisons.forEach((row, index) => {
      if (!independentSchemaExact(row, ["pair", "medianDelta", "p95Delta", "maxDelta"], errors,
        `${label}.comparisons[${index}]`)) return;
      if (row.pair !== ["product-vs-minimal", "minimal-vs-full"][index]
        || [row.medianDelta, row.p95Delta, row.maxDelta].some((item) => !finite(item) || item < 0)) {
        errors.push(`${label}.comparisons[${index}]:values`);
      }
    });
  } else {
    if (!["INCOMPLETE", "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS"].includes(value.status) || value.pass !== false) {
      errors.push(`${label}:status`);
    }
    independentSchemaStrings(value.preservedUnpairedReds, errors, `${label}.preservedUnpairedReds`);
  }
}

function independentSchemaPrivacy(value, location, errors) {
  if (Array.isArray(value)) {
    value.forEach((child, index) => independentSchemaPrivacy(child, `${location}[${index}]`, errors));
    return;
  }
  if (!value || typeof value !== "object") {
    if (typeof value === "string" && independentPrivacyStringInvalid(value)) {
      errors.push(`privacy:value:${location}`);
    }
    return;
  }
  for (const [key, child] of Object.entries(value)) {
    const childLocation = location ? `${location}.${key}` : key;
    if (INDEPENDENT_FORBIDDEN_KEY.test(key)) errors.push(`privacy:key:${childLocation}`);
    independentSchemaPrivacy(child, childLocation, errors);
  }
}

/** Verifier-owned exact recursive allowlist for its public output bytes. */
export function validateIndependentRecomputeSchema(value) {
  const errors = [];
  try {
  const observer = value?.expectation === "OBSERVER";
  const keys = ["version", "writeMode", "collectorOutputSha256", "expectation", "disposition", "selectedBranch",
    "identityManifestSha256", "collectorContractSha256", "environmentManifestSha256", "environmentId",
    "scenarioResults", "observerTriplets", ...(observer ? ["observerEvaluation"] : []), "verificationStatus"];
  if (!independentSchemaExact(value, keys, errors, "independent")) return { pass: false, errors };
  if (value.version !== "web06-independent-recompute-v1" || value.writeMode !== "create-new"
    || value.verificationStatus !== "PASS" || !["BASELINE", "FINAL", "PREVIEW", "OBSERVER"].includes(value.expectation)
    || !["NONE", "A", "B", "C"].includes(value.selectedBranch)
    || !["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE", "PRODUCTION_REDUCTION", "MEASURED_NO_GO"].includes(value.disposition)
    || typeof value.environmentId !== "string" || !value.environmentId) errors.push("independent:identity");
  const expectedDisposition = value.expectation === "OBSERVER" ? "DIAGNOSTIC"
    : value.expectation === "BASELINE" ? "SOURCE_CURRENT_BASELINE"
      : value.selectedBranch === "NONE" ? "MEASURED_NO_GO" : "PRODUCTION_REDUCTION";
  if ((["OBSERVER", "BASELINE"].includes(value.expectation) && value.selectedBranch !== "NONE")
    || (value.expectation === "PREVIEW" && !["A", "B", "C"].includes(value.selectedBranch))
    || value.disposition !== expectedDisposition) errors.push("independent:branch-disposition");
  for (const key of ["collectorOutputSha256", "identityManifestSha256", "collectorContractSha256",
    "environmentManifestSha256"]) if (!SHA64.test(value[key] ?? "")) errors.push(`independent.${key}:hash`);
  if (!Array.isArray(value.scenarioResults) || !Array.isArray(value.observerTriplets)) errors.push("independent:arrays");
  const expectedRuns = independentExpectedRunMatrix(value.expectation, value.selectedBranch);
  const expectedTarget = value.expectation === "BASELINE" ? "BASE_FULL"
    : value.expectation === "FINAL" ? "FINAL_FULL" : value.expectation === "PREVIEW" ? "FINAL_MINIMAL" : undefined;
  value.scenarioResults?.forEach((scenario, scenarioIndex) => {
    if (!independentSchemaExact(scenario, ["targetId", "scenarioRunId", "scenarioId", "schemaId",
      "measuredRoundCount", "validLatencyFrameRoundCount", "verdict",
      "preservedHardRedAttemptIds", "preservedHardRedObserved", "attemptResults",
      "fiveRoundSummaries", "fiveRoundDiagnosticBindingPairs", "fiveRoundSummarySha256"],
    errors, `independent.scenarioResults[${scenarioIndex}]`)) return;
    if (!Number.isSafeInteger(scenario.measuredRoundCount) || scenario.measuredRoundCount < 0
      || !Number.isSafeInteger(scenario.validLatencyFrameRoundCount)
      || scenario.validLatencyFrameRoundCount < 0
      || scenario.validLatencyFrameRoundCount > scenario.measuredRoundCount
      || !["PASS", "RED", "SETUP_NO_GO", "SETUP_INVALID"].includes(scenario.verdict)
      || typeof scenario.preservedHardRedObserved !== "boolean" || !Array.isArray(scenario.attemptResults)
      || !SHA64.test(scenario.fiveRoundSummarySha256 ?? "")
      || scenario.fiveRoundSummarySha256 !== sha256(Buffer.from(JSON.stringify(scenario.fiveRoundSummaries), "utf8"))) {
      errors.push(`independent.scenarioResults[${scenarioIndex}]:identity`);
    }
    const run = hasOwn(SCENARIO_RUN_REGISTRY, scenario.scenarioRunId)
      ? SCENARIO_RUN_REGISTRY[scenario.scenarioRunId]
      : undefined;
    if (scenario.targetId !== expectedTarget || scenario.scenarioRunId !== expectedRuns[scenarioIndex]
      || run?.scenarioId !== scenario.scenarioId || run?.schema !== scenario.schemaId) {
      errors.push(`independent.scenarioResults[${scenarioIndex}]:matrix-identity`);
    }
    independentSchemaStrings(scenario.preservedHardRedAttemptIds, errors,
      `independent.scenarioResults[${scenarioIndex}].preservedHardRedAttemptIds`);
    scenario.attemptResults?.forEach((attempt, attemptIndex) => {
      const complete = attempt?.measurementCompleted === true;
      const attemptKeys = ["attemptId", "rawPacketSha256", "measurementStarted", "measurementCompleted",
        "classification", "retainedMeasured", "retainedLogicalRound", "validForLatencyFrame",
        "retainedHardRed", "retryEligible",
        "validRedObserved", ...(complete ? ["diagnosticBindingPairs", "recomputedSummarySha256"] : ["failureCode"])];
      if (!independentSchemaExact(attempt, attemptKeys, errors,
        `independent.scenarioResults[${scenarioIndex}].attemptResults[${attemptIndex}]`)) return;
      if (attempt.attemptId !== `attempt-${attemptIndex + 1}` || !SHA64.test(attempt.rawPacketSha256 ?? "")
        || (complete && !SHA64.test(attempt.recomputedSummarySha256 ?? ""))
        || (!complete && (typeof attempt.failureCode !== "string" || !attempt.failureCode))
        || !["PASS", "RED", "SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(attempt.classification)
        || ["measurementStarted", "measurementCompleted", "retainedMeasured", "retainedLogicalRound",
          "validForLatencyFrame",
          "retainedHardRed", "retryEligible", "validRedObserved"].some((key) => typeof attempt[key] !== "boolean")) {
        errors.push(`independent.scenarioResults[${scenarioIndex}].attemptResults[${attemptIndex}]:values`);
      }
      if (complete) {
        if (!attempt.diagnosticBindingPairs || typeof attempt.diagnosticBindingPairs !== "object"
          || Array.isArray(attempt.diagnosticBindingPairs)
          || Object.keys(attempt.diagnosticBindingPairs).length === 0
          || Object.keys(attempt.diagnosticBindingPairs).some((surface) => !["common", "internal"].includes(surface))) {
          errors.push(`independent.scenarioResults[${scenarioIndex}].attemptResults[${attemptIndex}].diagnosticBindingPairs:keys`);
        } else for (const [surface, pair] of Object.entries(attempt.diagnosticBindingPairs)) {
          independentSchemaDiagnosticBindingPair(pair, errors,
            `independent.scenarioResults[${scenarioIndex}].attemptResults[${attemptIndex}].diagnosticBindingPairs.${surface}`,
            "round");
        }
      }
    });
    if (!scenario.fiveRoundSummaries || typeof scenario.fiveRoundSummaries !== "object"
      || Array.isArray(scenario.fiveRoundSummaries)
      || Object.keys(scenario.fiveRoundSummaries).some((surface) => !["common", "internal"].includes(surface))) {
      errors.push(`independent.scenarioResults[${scenarioIndex}].fiveRoundSummaries:keys`);
    } else for (const [surface, summary] of Object.entries(scenario.fiveRoundSummaries)) {
      independentSchemaFiveRound(summary, errors,
        `independent.scenarioResults[${scenarioIndex}].fiveRoundSummaries.${surface}`);
      if (summary?.surface !== surface) errors.push(`independent.scenarioResults[${scenarioIndex}]:surface-link`);
    }
    if (!scenario.fiveRoundDiagnosticBindingPairs
      || typeof scenario.fiveRoundDiagnosticBindingPairs !== "object"
      || Array.isArray(scenario.fiveRoundDiagnosticBindingPairs)
      || !exactJson(Object.keys(scenario.fiveRoundDiagnosticBindingPairs).sort(),
        Object.keys(scenario.fiveRoundSummaries ?? {}).sort())) {
      errors.push(`independent.scenarioResults[${scenarioIndex}].fiveRoundDiagnosticBindingPairs:keys`);
    } else for (const [surface, pair] of Object.entries(scenario.fiveRoundDiagnosticBindingPairs)) {
      independentSchemaDiagnosticBindingPair(pair, errors,
        `independent.scenarioResults[${scenarioIndex}].fiveRoundDiagnosticBindingPairs.${surface}`, "five-round");
    }
  });
  value.observerTriplets?.forEach((triplet, index) =>
    independentSchemaObserverTriplet(triplet, errors, `independent.observerTriplets[${index}]`, index));
  if (observer) independentSchemaObserverEvaluation(value.observerEvaluation, errors, "independent.observerEvaluation");
  if ((observer && (value.scenarioResults?.length !== 0
    || value.observerTriplets?.length > WEB06_THRESHOLDS.observer.maximumTripletAttempts))
    || (!observer && (value.observerTriplets?.length !== 0
      || value.scenarioResults?.length !== expectedRuns.length))) {
    errors.push("independent:lane-shape");
  }
  } catch {
    errors.push("independent:schema-exception");
  }
  try {
    independentSchemaPrivacy(value, "", errors);
  } catch {
    errors.push("independent:privacy-exception");
  }
  return { pass: errors.length === 0, errors };
}

export async function verifyCollectorOutput({ evidenceRoot, collectorOutputPath,
  repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../.."), verifyCurrentSource = true }) {
  const collectorBytes = await readFile(collectorOutputPath);
  const collector = JSON.parse(collectorBytes.toString("utf8"));
  if (verifyCurrentSource) {
    const observed = await independentlyVerifyRunnerSource({ repoRoot, manifest: collector.runnerSourceManifest });
    invariant(observed.observationSha256 === collector.runnerSourceObservationSha256,
      "WEB06_INDEPENDENT_RUNNER_OBSERVATION_HASH");
  }
  const rawEnvelopesByPath = new Map();
  for (const scenario of collector.scenarioResults ?? []) {
    for (const attempt of scenario.attempts ?? []) {
      const raw = await readRelativeRaw(evidenceRoot, attempt.rawPacket?.relativePath);
      invariant(raw.bytes.length === attempt.rawPacket.bytes, "WEB06_INDEPENDENT_RAW_SIZE", attempt.attemptId);
      const digest = sha256(raw.bytes);
      invariant(digest === attempt.rawPacket.sha256, "WEB06_INDEPENDENT_RAW_HASH", attempt.attemptId);
      rawEnvelopesByPath.set(attempt.rawPacket.relativePath, {
        bytes: raw.bytes,
        sha256: digest,
        envelope: JSON.parse(raw.bytes.toString("utf8")),
      });
    }
  }
  for (const triplet of collector.observerTriplets ?? []) {
    for (const modeName of ["product", "minimal", "full"]) {
      const mode = triplet?.[modeName];
      const raw = await readRelativeRaw(evidenceRoot, mode?.rawPacket?.relativePath);
      invariant(raw.bytes.length === mode.rawPacket.bytes, "WEB06_INDEPENDENT_RAW_SIZE",
        `${triplet.attemptId}:${modeName}`);
      const digest = sha256(raw.bytes);
      invariant(digest === mode.rawPacket.sha256, "WEB06_INDEPENDENT_RAW_HASH",
        `${triplet.attemptId}:${modeName}`);
      rawEnvelopesByPath.set(mode.rawPacket.relativePath, {
        bytes: raw.bytes,
        sha256: digest,
        envelope: JSON.parse(raw.bytes.toString("utf8")),
      });
    }
  }
  const payload = recomputeCollectorPayload({
    collector: { ...collector, collectorOutputSha256: sha256(collectorBytes) },
    rawEnvelopesByPath,
  });
  const schema = validateIndependentRecomputeSchema(payload);
  invariant(schema.pass, "WEB06_INDEPENDENT_PUBLIC_SCHEMA", schema.errors.join(","));
  return payload;
}

export async function writeIndependentRecompute({ evidenceRoot, collectorOutputPath, outputPath }) {
  const output = await verifyCollectorOutput({ evidenceRoot, collectorOutputPath });
  const bytes = Buffer.from(`${JSON.stringify(output, null, 2)}\n`, "utf8");
  const canonicalRoot = await realpath(evidenceRoot);
  const candidate = path.resolve(outputPath);
  invariant(path.basename(candidate) === "independent-recompute.json",
    "WEB06_INDEPENDENT_OUTPUT_PATH_INVALID");
  const roots = [...new Set([path.resolve(evidenceRoot), canonicalRoot])];
  const relative = roots.map((root) => path.relative(root, candidate)).find((value) =>
    value && value !== ".." && !value.startsWith(`..${path.sep}`) && !path.isAbsolute(value));
  invariant(relative !== undefined, "WEB06_INDEPENDENT_OUTPUT_PATH_INVALID");
  const destination = path.resolve(canonicalRoot, relative);
  let directory = canonicalRoot;
  const segments = path.dirname(relative) === "." ? [] : path.dirname(relative).split(path.sep);
  for (const segment of segments) {
    invariant(/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(segment), "WEB06_INDEPENDENT_OUTPUT_SEGMENT", segment);
    directory = path.join(directory, segment);
    await mkdir(directory, { recursive: true });
    const metadata = await lstat(directory);
    invariant(metadata.isDirectory() && !metadata.isSymbolicLink(), "WEB06_INDEPENDENT_OUTPUT_SYMLINK", segment);
    const canonical = await realpath(directory);
    invariant(path.relative(canonicalRoot, canonical) !== ".."
      && !path.relative(canonicalRoot, canonical).startsWith(`..${path.sep}`),
    "WEB06_INDEPENDENT_OUTPUT_PATH_ESCAPE", segment);
    directory = canonical;
  }
  invariant(path.join(directory, path.basename(destination)) === destination,
    "WEB06_INDEPENDENT_OUTPUT_CANONICAL_MISMATCH");
  const handle = await open(destination, "wx", 0o644);
  try {
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
  return { output, relativePath: relative.split(path.sep).join("/"),
    bytes: bytes.length, sha256: sha256(bytes), path: destination };
}

function parseArguments(argv) {
  const values = Object.create(null);
  const allowed = new Set(["evidence-root", "collector-output", "output"]);
  invariant(argv.length % 2 === 0, "WEB06_INDEPENDENT_ARGUMENTS_INVALID");
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    invariant(key?.startsWith("--") && value !== undefined, "WEB06_INDEPENDENT_ARGUMENTS_INVALID");
    const name = key.slice(2);
    invariant(allowed.has(name) && !hasOwn(values, name), "WEB06_INDEPENDENT_ARGUMENTS_INVALID", name);
    values[name] = value;
  }
  for (const field of ["evidence-root", "collector-output", "output"]) {
    invariant(typeof values[field] === "string" && path.isAbsolute(values[field]),
      "WEB06_INDEPENDENT_ABSOLUTE_ARGUMENT_REQUIRED", field);
  }
  return values;
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  const args = parseArguments(process.argv.slice(2));
  await writeIndependentRecompute({
    evidenceRoot: args["evidence-root"],
    collectorOutputPath: args["collector-output"],
    outputPath: args.output,
  });
}

// Freeze the only imported mutable-looking values at module evaluation and
// make accidental use of runner helpers visible in source review.
invariant(WEB06_METRIC_CONTRACT_VERSION === "web06-metric-v1"
  && WEB06_SCENARIO_REGISTRY_VERSION === "web06-scenarios-v1"
  && WEB06_BEHAVIOR_PREDICATE_VERSION === "web06-behavior-predicates-v1"
  && SHA64.test("0".repeat(64)), "WEB06_INDEPENDENT_VERSION_MISMATCH");
