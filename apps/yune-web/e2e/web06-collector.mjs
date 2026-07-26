/**
 * WEB-06 source-bound browser collector helpers.
 *
 * This module deliberately keeps browser driving separate from metric policy:
 * the browser emits raw timestamps and identities, while the frozen parser
 * independently derives verdicts. Raw packets are written only beneath an
 * explicitly supplied external evidence root.
 */

import { createHash } from "node:crypto";
import { execFile } from "node:child_process";
import { lstat, mkdir, open, readFile, realpath, statfs } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { promisify } from "node:util";

import {
  EVENT_ACTION_RULES,
  SCENARIO_REGISTRY,
  SCENARIO_RUN_REGISTRY,
  WEB06_BEHAVIOR_PREDICATES,
  WEB06_BEHAVIOR_PREDICATE_VERSION,
  WEB06_METRIC_CONTRACT_VERSION,
  WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION,
  WEB06_OBSERVER_COUNTERBALANCE,
  WEB06_SCENARIO_REGISTRY_VERSION,
  WEB06_THRESHOLDS,
  evaluateAttemptSeries,
  evaluateObserverOverhead,
  expandScenarioExpectedTimeline,
  isSha256,
  resolveScenarioRun,
} from "./web06-metric-contract.mjs";
import {
  buildFiveRoundEvidenceSummary,
  buildRoundEvidenceSummary,
  normalizeWireActionArgs,
  publicEvidenceReceipt,
  validateAndRecomputeReceipt,
  validateCommonSurfaceReceipt,
  validatePublicFiveRoundSummarySchema,
  validatePublicEvidenceSchema,
  validatePublicRoundSummarySchema,
  validatePointerFreePrivacy,
} from "./web06-receipt-parser.mjs";

export const WEB06_PRIVATE_PROTOCOL_VERSION = "web06-private-v1";

export const WEB06_TARGET_PROTOCOL_MODES = Object.freeze({
  PRODUCT: "off",
  BASE_MINIMAL: "minimal",
  BASE_FULL: "full",
  FINAL_MINIMAL: "minimal",
  FINAL_FULL: "full",
});

export const WEB06_PREVIEW_SCENARIOS = Object.freeze([
  "existing-normal-guard",
  "rapid-jyutping",
]);

export const WEB06_PHASE0_OVERHEAD_SCENARIO = "rapid-long-jyutping";

const WEB06_BINDING_SCENARIO_IDS = Object.freeze([
  "existing-normal-guard",
  "rapid-jyutping",
  "rapid-long-jyutping",
  "rapid-luna",
  "burst-jyutping",
  "burst-luna",
  "correction",
  "selection-paging",
  "selection-paging-jyutping",
  "burst-action-map",
  "fifo-pressure-barriers",
  "learned-row",
  "fair-peer-short",
]);

export const WEB06_BINDING_SCENARIO_ORDER = Object.freeze(
  WEB06_BINDING_SCENARIO_IDS.flatMap((scenarioId) => Object.values(SCENARIO_RUN_REGISTRY)
    .filter((run) => run.scenarioId === scenarioId)
    .map((run) => run.runId)),
);

export const WEB06_EXTENDED_SCENARIO_RUNS = Object.freeze(
  Object.values(SCENARIO_RUN_REGISTRY)
    .filter((run) => run.scenarioId === "extended-scheduler-barriers")
    .map((run) => run.runId),
);

export const WEB06_COLLECTOR_CONTRACT_SHA256 = digestJson({
  metricContractVersion: WEB06_METRIC_CONTRACT_VERSION,
  scenarioRegistryVersion: WEB06_SCENARIO_REGISTRY_VERSION,
  behaviorPredicateVersion: WEB06_BEHAVIOR_PREDICATE_VERSION,
  thresholds: WEB06_THRESHOLDS,
  scenarios: SCENARIO_REGISTRY,
  behaviorPredicates: WEB06_BEHAVIOR_PREDICATES,
  eventActionRules: EVENT_ACTION_RULES,
});

export const WEB06_EXPECTATION_TARGET_ORDER = Object.freeze({
  BASELINE: Object.freeze(["BASE_FULL"]),
  FINAL: Object.freeze(["FINAL_FULL"]),
  PREVIEW: Object.freeze(["FINAL_MINIMAL"]),
  OBSERVER: Object.freeze(["PRODUCT", "BASE_MINIMAL", "BASE_FULL"]),
});

const COMMIT_RE = /^[0-9a-f]{40}$/;
const SAFE_SEGMENT_RE = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const REGISTRY_RUN_SEGMENT_RE = /^[A-Za-z0-9][A-Za-z0-9._@-]*$/;

function hasOwn(record, key) {
  return typeof key === "string" && Object.prototype.hasOwnProperty.call(record, key);
}

function safeEvidenceSegment(segment) {
  return typeof segment === "string" && (SAFE_SEGMENT_RE.test(segment)
    || (hasOwn(SCENARIO_RUN_REGISTRY, segment) && segment.includes("@")
      && REGISTRY_RUN_SEGMENT_RE.test(segment)));
}
const execFileAsync = promisify(execFile);

export const WEB06_RUNNER_TOOLING_PATHS = Object.freeze([
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

export const WEB06_FROZEN_RUN_ENVIRONMENT = Object.freeze({
  powerState: "AC",
  lowPowerMode: false,
  viewport: Object.freeze({ width: 1365, height: 900 }),
  displayRefreshHz: 60,
  displayLane: "declared-60hz",
  cacheRegime: "fresh-profile",
  locale: "zh-HK",
  workers: 1,
  retries: 0,
  pageSize: 6,
  aiEnabled: false,
  debugUi: "production-default",
  inspectorUi: "production-default",
});

export class Web06CollectorSetupError extends Error {
  constructor(code, detail = "") {
    super(`${code}${detail ? `:${detail}` : ""}`);
    this.name = "Web06CollectorSetupError";
    this.code = code;
  }
}

const WEB06_HARNESS_BEHAVIOR_FAILURE_CODES = new Set([
  "WEB06_ACTION_COMPLETION_COUNT",
  "WEB06_IMPORT_CONTINUATION_MARKER_TIMEOUT",
  "WEB06_LEARNED_PREPARE_CONTINUITY_INVALID",
  "WEB06_LEARNED_PROTOCOL_SEQUENCE_REUSE",
  "WEB06_LEARNED_RELOAD_ARRIVAL_INVALID",
  "WEB06_LEARNED_RELOAD_BIND_INVALID",
  "WEB06_PRESSURE_ACTION_MISSING",
  "WEB06_SAME_TASK_PAIR_REORDERED",
]);

const WEB06_HARNESS_SETUP_FAILURE_CODES = new Set([
  "WEB06_CONTROL_MISSING",
  "WEB06_EXTENDED_OPTION_CONTROL_MISSING",
  "WEB06_IMPORT_CONTROL_MISSING",
  "WEB06_LEARNED_BOUNDARY_MISSING",
  "WEB06_SETUP_LEARNED_PRIVATE_PROTOCOL_REQUIRED",
  "WEB06_LEARNED_STEP_SOURCE",
  "WEB06_LEARNED_WINDOW_DURATION_INVALID",
  "WEB06_MEASURED_UI_PAGE_SIZE_NOT_SIX",
  "WEB06_OBSERVER_COUNTERBALANCE",
  "WEB06_PAGE_SIZE_ENGINE_NOT_SEVEN",
  "WEB06_RUNNER_SOURCE_CHANGED_DURING_ATTEMPT",
  "WEB06_SCHEMA_CONTROL_MISSING",
  "WEB06_SETUP_AI_POSTURE",
  "WEB06_SETUP_CONFIGURATION_TRANSFER",
  "WEB06_SETUP_DEBUG_POSTURE",
  "WEB06_SETUP_DISPOSABLE_PROTOCOL_DIRTY",
  "WEB06_SETUP_FAILURE",
  "WEB06_SETUP_FOREGROUND_POSTURE",
  "WEB06_SETUP_INSPECTOR_POSTURE",
  "WEB06_SETUP_LOCALE_POSTURE",
  "WEB06_SETUP_PAGE_SIZE_POSTURE",
  "WEB06_SETUP_PREFLIGHT",
  "WEB06_SETUP_PROTOCOL_DIRTY",
  "WEB06_SETUP_VIEWPORT_POSTURE",
  "WEB06_SOURCE_REVIEWED_SCENARIOS_BLOCKED",
  "WEB06_UNSUPPORTED_REAL_CONTROL",
  "WEB06_UNSUPPORTED_REAL_UI_STEP",
  "WEB06_WINDOW_COUNT",
  "WEB06_WINDOW_DURATION_INVALID",
]);

const WEB06_HARNESS_FATAL_FAILURE_CODES = new Set([
  "WEB06_POST_MEASUREMENT_PACKAGING_FAILURE",
  "WEB06_RAW_PERSISTENCE_FATAL",
  "WEB06_RAW_RESERVATION_MISSING",
]);

function web06ErrorCode(error) {
  if (error && typeof error === "object" && typeof error.code === "string"
    && error.code.startsWith("WEB06_")) return error.code;
  const message = error instanceof Error ? error.message : String(error);
  return /^WEB06_[A-Z0-9_]+/.exec(message)?.[0] ?? "WEB06_UNCLASSIFIED_FAILURE";
}

/**
 * Frozen dimensional classification for errors which can escape the browser
 * driver. Unknown or browser-transport exceptions are setup-invalid; only the
 * enumerated coherent action/order/continuity failures are hard behavior REDs.
 */
export function classifyWeb06HarnessFailure(error) {
  const code = web06ErrorCode(error);
  if (error instanceof Web06CollectorSetupError || WEB06_HARNESS_SETUP_FAILURE_CODES.has(code)) {
    return Object.freeze({ code, dimension: "setup" });
  }
  if (WEB06_HARNESS_BEHAVIOR_FAILURE_CODES.has(code)) {
    return Object.freeze({ code, dimension: "behavior" });
  }
  if (WEB06_HARNESS_FATAL_FAILURE_CODES.has(code)) {
    return Object.freeze({ code, dimension: "fatal" });
  }
  return Object.freeze({ code, dimension: "setup" });
}

function setupAssert(condition, code, detail) {
  if (!condition) throw new Web06CollectorSetupError(code, detail);
}

function clone(value) {
  return value === undefined ? undefined : structuredClone(value);
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function isWithin(parent, child) {
  const relative = path.relative(parent, child);
  return relative !== "" && !relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative);
}

function parseJsonEnv(raw, name) {
  setupAssert(typeof raw === "string" && raw.length > 0, `WEB06_${name}_REQUIRED`);
  try {
    return JSON.parse(raw);
  } catch (error) {
    throw new Web06CollectorSetupError(`WEB06_${name}_INVALID_JSON`, error instanceof Error ? error.message : String(error));
  }
}

function validatePinnedIdentityRole(roleName, role) {
  setupAssert(role && typeof role === "object" && !Array.isArray(role), "WEB06_IDENTITY_ROLE_MISSING", roleName);
  setupAssert(exactKeys(role, ["sourceCommit", "sourceTree", "sourceTreeState", "archiveSha256",
    "artifactManifestSha256", "buildInfoSha256", "selectedBranch", "disposition"]),
  "WEB06_IDENTITY_ROLE_KEYS_INVALID", roleName);
  setupAssert(COMMIT_RE.test(role.sourceCommit ?? ""), "WEB06_IDENTITY_SOURCE_COMMIT_INVALID", roleName);
  setupAssert(COMMIT_RE.test(role.sourceTree ?? ""), "WEB06_IDENTITY_SOURCE_TREE_INVALID", roleName);
  setupAssert(role.sourceTreeState === "clean", "WEB06_IDENTITY_SOURCE_TREE_NOT_CLEAN", roleName);
  setupAssert(isSha256(role.archiveSha256), "WEB06_IDENTITY_ARCHIVE_HASH_INVALID", roleName);
  setupAssert(isSha256(role.artifactManifestSha256), "WEB06_IDENTITY_ARTIFACT_MANIFEST_HASH_INVALID", roleName);
  setupAssert(isSha256(role.buildInfoSha256), "WEB06_IDENTITY_BUILD_INFO_HASH_INVALID", roleName);
  setupAssert(["NONE", "A", "B", "C"].includes(role.selectedBranch), "WEB06_IDENTITY_BRANCH_INVALID", roleName);
  setupAssert(["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE", "PRODUCTION_REDUCTION", "MEASURED_NO_GO"].includes(role.disposition), "WEB06_IDENTITY_DISPOSITION_INVALID", roleName);
  return role;
}

function validateRunEnvironmentManifest(raw) {
  const manifest = parseJsonEnv(raw, "RUN_ENVIRONMENT_JSON");
  setupAssert(manifest?.version === "web06-run-environment-v1", "WEB06_RUN_ENVIRONMENT_VERSION_MISMATCH");
  setupAssert(isSha256(manifest.environmentId), "WEB06_RUN_ENVIRONMENT_ID_INVALID");
  const requiredToolchains = ["rust", "emscripten", "node", "npm", "playwright", "chromium", "chromiumExecutableSha256"];
  for (const field of requiredToolchains) {
    setupAssert(typeof manifest.toolchain?.[field] === "string" && manifest.toolchain[field].length > 0,
      "WEB06_RUN_ENVIRONMENT_TOOLCHAIN_INVALID", field);
  }
  for (const field of ["os", "osVersion", "osBuildVersion", "arch", "cpuModel", "powerState"]) {
    setupAssert(typeof manifest.host?.[field] === "string" && manifest.host[field].length > 0,
      "WEB06_RUN_ENVIRONMENT_HOST_INVALID", field);
  }
  setupAssert(Number.isSafeInteger(manifest.host?.logicalCores) && manifest.host.logicalCores > 0,
    "WEB06_RUN_ENVIRONMENT_LOGICAL_CORES_INVALID");
  setupAssert(Number.isSafeInteger(manifest.host?.memoryBytes) && manifest.host.memoryBytes > 0,
    "WEB06_RUN_ENVIRONMENT_MEMORY_INVALID");
  setupAssert(manifest.host.powerState === WEB06_FROZEN_RUN_ENVIRONMENT.powerState,
    "WEB06_RUN_ENVIRONMENT_AC_POWER_REQUIRED");
  setupAssert(manifest.host.lowPowerMode === WEB06_FROZEN_RUN_ENVIRONMENT.lowPowerMode,
    "WEB06_RUN_ENVIRONMENT_LOW_POWER_MODE_INVALID");
  setupAssert(manifest.browser?.viewport?.width === WEB06_FROZEN_RUN_ENVIRONMENT.viewport.width
    && manifest.browser?.viewport?.height === WEB06_FROZEN_RUN_ENVIRONMENT.viewport.height,
  "WEB06_RUN_ENVIRONMENT_VIEWPORT_INVALID");
  setupAssert(manifest.browser?.displayRefreshHz === WEB06_FROZEN_RUN_ENVIRONMENT.displayRefreshHz,
    "WEB06_RUN_ENVIRONMENT_REFRESH_INVALID");
  setupAssert(manifest.browser?.displayLane === WEB06_FROZEN_RUN_ENVIRONMENT.displayLane,
    "WEB06_RUN_ENVIRONMENT_DISPLAY_LANE_INVALID");
  setupAssert(manifest.browser?.cacheRegime === WEB06_FROZEN_RUN_ENVIRONMENT.cacheRegime,
    "WEB06_RUN_ENVIRONMENT_CACHE_REGIME_INVALID");
  setupAssert(manifest.browser?.locale === WEB06_FROZEN_RUN_ENVIRONMENT.locale,
    "WEB06_RUN_ENVIRONMENT_LOCALE_INVALID");
  setupAssert(manifest.runner?.workers === WEB06_FROZEN_RUN_ENVIRONMENT.workers
    && manifest.runner?.retries === WEB06_FROZEN_RUN_ENVIRONMENT.retries,
  "WEB06_RUN_ENVIRONMENT_RUNNER_CONCURRENCY_INVALID");
  setupAssert(manifest.ui?.pageSize === WEB06_FROZEN_RUN_ENVIRONMENT.pageSize
    && manifest.ui?.aiEnabled === WEB06_FROZEN_RUN_ENVIRONMENT.aiEnabled
    && manifest.ui?.debugUi === WEB06_FROZEN_RUN_ENVIRONMENT.debugUi
    && manifest.ui?.inspectorUi === WEB06_FROZEN_RUN_ENVIRONMENT.inspectorUi,
  "WEB06_RUN_ENVIRONMENT_UI_POSTURE_INVALID");
  const canonical = {
    version: manifest.version,
    toolchain: manifest.toolchain,
    host: manifest.host,
    browser: manifest.browser,
    runner: manifest.runner,
    ui: manifest.ui,
  };
  setupAssert(manifest.environmentId === digestJson(canonical), "WEB06_RUN_ENVIRONMENT_ID_CONTENT_MISMATCH");
  const privacy = validatePointerFreePrivacy(manifest);
  setupAssert(privacy.pass, "WEB06_RUN_ENVIRONMENT_PRIVACY_INVALID", privacy.errors.join(","));
  return Object.freeze({ manifest: clone(manifest), sha256: digestJson(manifest) });
}

function validateRunnerSourceManifest(manifest, expectedRole) {
  setupAssert(manifest && typeof manifest === "object" && !Array.isArray(manifest),
    "WEB06_RUNNER_SOURCE_MANIFEST_INVALID");
  setupAssert(manifest.version === "web06-runner-source-v1", "WEB06_RUNNER_SOURCE_VERSION_MISMATCH");
  setupAssert(COMMIT_RE.test(manifest.sourceCommit ?? ""), "WEB06_RUNNER_SOURCE_COMMIT_INVALID");
  setupAssert(COMMIT_RE.test(manifest.sourceTree ?? ""), "WEB06_RUNNER_SOURCE_TREE_INVALID");
  setupAssert(manifest.sourceTreeState === "clean", "WEB06_RUNNER_SOURCE_TREE_NOT_CLEAN");
  setupAssert(manifest.sourceCommit === expectedRole.sourceCommit, "WEB06_RUNNER_SOURCE_COMMIT_NOT_PINNED");
  setupAssert(manifest.sourceTree === expectedRole.sourceTree, "WEB06_RUNNER_SOURCE_TREE_NOT_PINNED");
  setupAssert(manifest.tooling?.version === "web06-runner-tooling-v1", "WEB06_RUNNER_TOOLING_VERSION_MISMATCH");
  setupAssert(Array.isArray(manifest.tooling?.files), "WEB06_RUNNER_TOOLING_FILES_INVALID");
  setupAssert(JSON.stringify(manifest.tooling.files.map((file) => file?.path)) === JSON.stringify(WEB06_RUNNER_TOOLING_PATHS),
    "WEB06_RUNNER_TOOLING_PATH_SET_MISMATCH");
  setupAssert(manifest.tooling.files.every((file) => isSha256(file?.sha256)),
    "WEB06_RUNNER_TOOLING_FILE_HASH_INVALID");
  setupAssert(isSha256(manifest.toolingManifestSha256)
    && manifest.toolingManifestSha256 === digestJson(manifest.tooling),
  "WEB06_RUNNER_TOOLING_MANIFEST_HASH_MISMATCH");
  return Object.freeze(clone(manifest));
}

function parseIdentityManifest(raw, requiredRoles) {
  const manifest = parseJsonEnv(raw, "IDENTITY_MANIFEST_JSON");
  setupAssert(manifest && typeof manifest === "object" && !Array.isArray(manifest), "WEB06_IDENTITY_MANIFEST_INVALID");
  setupAssert(exactKeys(manifest, ["version", "metricContractVersion", "scenarioRegistryVersion",
    "behaviorPredicateVersion", "collectorContractSha256", "roles"]),
  "WEB06_IDENTITY_MANIFEST_KEYS_INVALID");
  setupAssert(manifest.version === "web06-target-identities-v1", "WEB06_IDENTITY_MANIFEST_VERSION_MISMATCH");
  setupAssert(manifest.metricContractVersion === WEB06_METRIC_CONTRACT_VERSION, "WEB06_IDENTITY_METRIC_VERSION_MISMATCH");
  setupAssert(manifest.scenarioRegistryVersion === WEB06_SCENARIO_REGISTRY_VERSION, "WEB06_IDENTITY_SCENARIO_VERSION_MISMATCH");
  setupAssert(manifest.behaviorPredicateVersion === WEB06_BEHAVIOR_PREDICATE_VERSION, "WEB06_IDENTITY_BEHAVIOR_VERSION_MISMATCH");
  setupAssert(manifest.collectorContractSha256 === WEB06_COLLECTOR_CONTRACT_SHA256, "WEB06_IDENTITY_COLLECTOR_CONTRACT_HASH_MISMATCH");
  const roles = manifest.roles;
  setupAssert(roles && typeof roles === "object" && !Array.isArray(roles), "WEB06_IDENTITY_ROLES_INVALID");
  setupAssert(JSON.stringify(Object.keys(roles).sort()) === JSON.stringify([...requiredRoles].sort()),
    "WEB06_IDENTITY_ROLE_SET_MISMATCH");
  for (const roleName of requiredRoles) validatePinnedIdentityRole(roleName, roles[roleName]);
  const canonicalManifest = stableJsonValue(manifest);
  return Object.freeze({
    manifest: clone(canonicalManifest),
    sha256: digestJson(canonicalManifest),
  });
}

function identityRoleForTarget(id) {
  if (id === "PRODUCT") return "PRODUCT";
  if (id.startsWith("BASE_")) return "BASE";
  if (id.startsWith("FINAL_")) return "FINAL";
  throw new Web06CollectorSetupError("WEB06_TARGET_ID_INVALID", id);
}

function exactKeys(value, keys) {
  return value && typeof value === "object" && !Array.isArray(value)
    && JSON.stringify(Object.keys(value).sort()) === JSON.stringify([...keys].sort());
}

export function validateArtifactResponseGuard(guard, target, targetId = "target") {
  setupAssert(exactKeys(guard, ["version", "rootDocumentPath", "entries"])
    && guard.version === "web06-artifact-response-guard-v1"
    && guard.rootDocumentPath === "index.html" && Array.isArray(guard.entries),
  "WEB06_ARTIFACT_RESPONSE_GUARD_SHAPE", targetId);
  setupAssert(guard.entries.length >= 3, "WEB06_ARTIFACT_RESPONSE_GUARD_ENTRY_COUNT", targetId);
  const seen = new Set();
  for (const [index, entry] of guard.entries.entries()) {
    setupAssert(exactKeys(entry, ["path", "bytes", "sha256"])
      && typeof entry.path === "string" && entry.path.length > 0
      && !entry.path.startsWith("/") && !entry.path.includes("\\")
      && !entry.path.split("/").some((segment) => segment === "" || segment === "." || segment === "..")
      && Number.isSafeInteger(entry.bytes) && entry.bytes >= 0 && isSha256(entry.sha256),
    "WEB06_ARTIFACT_RESPONSE_GUARD_ENTRY", `${targetId}:${index + 1}`);
    setupAssert(!seen.has(entry.path), "WEB06_ARTIFACT_RESPONSE_GUARD_DUPLICATE_PATH", entry.path);
    seen.add(entry.path);
  }
  setupAssert(guard.entries[0].path === "build-info.json"
    && guard.entries[0].sha256 === target.buildInfoSha256,
  "WEB06_ARTIFACT_RESPONSE_GUARD_BUILD_INFO_IDENTITY", targetId);
  setupAssert(guard.entries[1].path === "public-artifact-manifest.json"
    && guard.entries[1].sha256 === target.artifactSha256,
  "WEB06_ARTIFACT_RESPONSE_GUARD_MANIFEST_IDENTITY", targetId);
  setupAssert(seen.has(guard.rootDocumentPath), "WEB06_ARTIFACT_RESPONSE_GUARD_ROOT_MISSING", targetId);
  return Object.freeze({ guard: clone(guard), sha256: digestJson(guard) });
}

export function evaluateArtifactResponseGuardObservations({ guard, guardSha256, observations,
  stage, additionalFailureCodes = [] }) {
  setupAssert(Array.isArray(observations) && Array.isArray(additionalFailureCodes),
    "WEB06_ARTIFACT_RESPONSE_OBSERVATIONS_SHAPE");
  const expected = new Map(guard.entries.map((entry) => [entry.path, entry]));
  const counts = new Map();
  const failures = [...additionalFailureCodes];
  for (const [index, observation] of observations.entries()) {
    setupAssert(exactKeys(observation, ["path", "status", "bytes", "sha256"])
      && typeof observation.path === "string" && Number.isSafeInteger(observation.status)
      && Number.isSafeInteger(observation.bytes) && observation.bytes >= 0 && isSha256(observation.sha256),
    "WEB06_ARTIFACT_RESPONSE_OBSERVATION", String(index + 1));
    counts.set(observation.path, (counts.get(observation.path) ?? 0) + 1);
    const row = expected.get(observation.path);
    if (!row) {
      failures.push(`UNKNOWN_SAME_ORIGIN_RESPONSE:${digestJson(observation.path)}`);
      continue;
    }
    if (observation.status < 200 || observation.status >= 300) {
      failures.push(`RESPONSE_STATUS:${observation.path}:${observation.status}`);
    }
    if (observation.bytes !== row.bytes) failures.push(`RESPONSE_BYTES:${observation.path}`);
    if (observation.sha256 !== row.sha256) failures.push(`RESPONSE_SHA256:${observation.path}`);
  }
  for (const entry of guard.entries) {
    if (!counts.has(entry.path)) failures.push(`MISSING_RESPONSE:${entry.path}`);
  }
  const distinctFailures = [...new Set(failures)];
  const summary = {
    version: "web06-artifact-response-guard-summary-v1",
    stage,
    expectedGuardSha256: guardSha256,
    expectedEntryCount: guard.entries.length,
    observedResponseCount: observations.length,
    observedUniquePathCount: counts.size,
    duplicateResponseCount: [...counts.values()].reduce((sum, count) => sum + Math.max(0, count - 1), 0),
    unknownPathCount: observations.filter((observation) => !expected.has(observation.path)).length,
    verifiedPathCountsSha256: digestJson(guard.entries.map((entry) => [entry.path, counts.get(entry.path) ?? 0])),
    observedPathCounts: guard.entries.map((entry) => ({ path: entry.path, count: counts.get(entry.path) ?? 0 })),
    failureCodes: distinctFailures,
    pass: distinctFailures.length === 0,
  };
  return Object.freeze({ ...summary, summarySha256: digestJson(summary) });
}

export function mergeWeb06LearnedSentinelSegments(pre, post, lifecycleMarker) {
  const expected = expandScenarioExpectedTimeline("learned-row");
  const boundary = expected.events.find((event) => event.stepId === "learned-reload-boundary");
  const preEvents = pre.events ?? [];
  setupAssert(boundary && preEvents.length === boundary.eventSequenceId - 1
    && (post.events ?? []).length === expected.events.length - boundary.eventSequenceId,
  "WEB06_LEARNED_SENTINEL_EVENT_PARTITION");
  setupAssert(lifecycleMarker?.eventSequenceId === boundary.eventSequenceId
    && lifecycleMarker?.stepId === boundary.stepId && lifecycleMarker?.type === boundary.type
    && lifecycleMarker?.key === boundary.key && lifecycleMarker?.code === boundary.code
    && lifecycleMarker?.classification === boundary.classification && lifecycleMarker?.reason === boundary.reason
    && exactKeys(lifecycleMarker, ["eventSequenceId", "pageInstanceId", "stepId", "type", "key", "code",
      "classification", "reason", "mappedActionIds", "eventTimestamp", "normalizedEventAt",
      "sentinelObservedAt", "originOwner"])
    && Array.isArray(lifecycleMarker.mappedActionIds) && lifecycleMarker.mappedActionIds.length === 0
    && lifecycleMarker.originOwner === "harness-browser-lifecycle"
    && Number.isFinite(lifecycleMarker.eventTimestamp)
    && lifecycleMarker.normalizedEventAt === lifecycleMarker.eventTimestamp
    && lifecycleMarker.sentinelObservedAt >= lifecycleMarker.normalizedEventAt,
  "WEB06_LEARNED_LIFECYCLE_MARKER_INVALID");
  const postEvents = (post.events ?? []).map((event, index) => ({
    ...event,
    eventSequenceId: boundary.eventSequenceId + index + 1,
  }));
  const postCallbackLedger = (post.callbackLedger ?? []).map((callback) => ({
    ...callback,
    ...(callback.eventSequenceId === undefined ? {} : {
      eventSequenceId: boundary.eventSequenceId + callback.eventSequenceId,
    }),
  }));
  const preObserver = pre.longTaskObserver;
  const postObserver = post.longTaskObserver;
  setupAssert(typeof preObserver?.pageInstanceId === "string" && typeof postObserver?.pageInstanceId === "string"
    && preObserver.pageInstanceId !== postObserver.pageInstanceId,
  "WEB06_LEARNED_SENTINEL_REALM_IDENTITY");
  const overflowKeys = new Set([
    ...Object.keys(pre.sentinelOverflowCounts ?? {}),
    ...Object.keys(post.sentinelOverflowCounts ?? {}),
  ]);
  return {
    ...post,
    events: [...preEvents, clone(lifecycleMarker), ...postEvents],
    auxiliaryEvents: [...(pre.auxiliaryEvents ?? []), ...(post.auxiliaryEvents ?? [])],
    unmatchedEvents: [...(pre.unmatchedEvents ?? []), ...(post.unmatchedEvents ?? [])],
    snapshots: [...(pre.snapshots ?? []), ...(post.snapshots ?? [])],
    interactionWindows: [...(pre.interactionWindows ?? []), ...(post.interactionWindows ?? [])],
    idleControlWindows: [...(pre.idleControlWindows ?? []), ...(post.idleControlWindows ?? [])],
    interactionFrameWindows: [...(pre.interactionFrameWindows ?? []), ...(post.interactionFrameWindows ?? [])],
    interactionFrameTimestamps: [...(pre.interactionFrameTimestamps ?? []), ...(post.interactionFrameTimestamps ?? [])],
    interactionFrameIntervalsMs: [...(pre.interactionFrameIntervalsMs ?? []), ...(post.interactionFrameIntervalsMs ?? [])],
    longTaskObserver: {
      supported: preObserver.supported === true && postObserver.supported === true,
      installedAt: Math.min(preObserver.installedAt, postObserver.installedAt),
      segments: [clone(preObserver), clone(postObserver)],
    },
    longTasks: [...(pre.longTasks ?? []), ...(post.longTasks ?? [])],
    focusVisibilitySamples: [...(pre.focusVisibilitySamples ?? []), ...(post.focusVisibilitySamples ?? [])],
    assetsRequestedDuringWindow: [...(pre.assetsRequestedDuringWindow ?? []), ...(post.assetsRequestedDuringWindow ?? [])],
    callbackLedger: [...(pre.callbackLedger ?? []), ...postCallbackLedger],
    callbackLedgerCapacity: (pre.callbackLedgerCapacity ?? 0) + (post.callbackLedgerCapacity ?? 0),
    callbackLedgerOverflowCount: (pre.callbackLedgerOverflowCount ?? 0) + (post.callbackLedgerOverflowCount ?? 0),
    sentinelOverflowCounts: Object.fromEntries([...overflowKeys].map((key) => [
      key,
      (pre.sentinelOverflowCounts?.[key] ?? 0) + (post.sentinelOverflowCounts?.[key] ?? 0),
    ])),
    sentinelCallbacksMs: [...(pre.sentinelCallbacksMs ?? []), ...(post.sentinelCallbacksMs ?? [])],
    unattributedInWindowCallbacksMs: [
      ...(pre.unattributedInWindowCallbacksMs ?? []), ...(post.unattributedInWindowCallbacksMs ?? []),
    ],
    sentinelTotalPerEventMs: [
      ...(pre.sentinelTotalPerEventMs ?? []), 0, ...(post.sentinelTotalPerEventMs ?? []),
    ],
    sentinelTotalPerWindowMs: [...(pre.sentinelTotalPerWindowMs ?? []), ...(post.sentinelTotalPerWindowMs ?? [])],
    sentinelAccountedCallbackCount:
      (pre.sentinelAccountedCallbackCount ?? 0) + (post.sentinelAccountedCallbackCount ?? 0),
  };
}

export function mergeWeb06LearnedProtocolSegments(pre, post, lifecycleMarker) {
  const expectedLifecycle = expandScenarioExpectedTimeline("learned-row").events
    .find((event) => event.stepId === "learned-reload-boundary");
  setupAssert(expectedLifecycle && lifecycleMarker?.eventSequenceId === expectedLifecycle.eventSequenceId
    && lifecycleMarker?.stepId === expectedLifecycle.stepId && lifecycleMarker?.type === expectedLifecycle.type
    && lifecycleMarker?.classification === expectedLifecycle.classification
    && lifecycleMarker?.reason === expectedLifecycle.reason
    && lifecycleMarker?.originOwner === "harness-browser-lifecycle",
  "WEB06_LEARNED_PROTOCOL_LIFECYCLE_MARKER_INVALID");
  const prePage = pre?.header?.pageInstanceId;
  const postPage = post?.header?.pageInstanceId;
  setupAssert(typeof prePage === "string" && typeof postPage === "string" && prePage !== postPage,
    "WEB06_LEARNED_PROTOCOL_REALM_IDENTITY");
  const events = [
    ...(pre?.events ?? []).map((event) => ({ ...event, web06PageInstanceId: prePage })),
    ...(post?.events ?? []).map((event) => ({ ...event, web06PageInstanceId: postPage })),
  ];
  const actions = [
    ...(pre?.actions ?? []).map((action) => ({ ...action, web06PageInstanceId: prePage })),
    ...(post?.actions ?? []).map((action) => ({ ...action, web06PageInstanceId: postPage })),
  ];
  const eventIds = events.map((event) => `${event.web06PageInstanceId}:${event.identity?.eventSequenceId}`);
  const actionIds = actions.map((action) => `${action.web06PageInstanceId}:${action.identity?.sequenceId}`);
  setupAssert(events.every((event) => Number.isSafeInteger(event.identity?.eventSequenceId))
    && actions.every((action) => Number.isSafeInteger(action.identity?.sequenceId))
    && new Set(eventIds).size === eventIds.length && new Set(actionIds).size === actionIds.length,
  "WEB06_LEARNED_PROTOCOL_SEQUENCE_REUSE");
  const protocolWindowSegments = [
    {
      pageInstanceId: prePage,
      receiptWindowStartEventSequenceId: pre?.status?.receiptWindowStartEventSequenceId,
      receiptWindowStartActionSequenceId: pre?.status?.receiptWindowStartActionSequenceId,
    },
    {
      pageInstanceId: postPage,
      receiptWindowStartEventSequenceId: post?.status?.receiptWindowStartEventSequenceId,
      receiptWindowStartActionSequenceId: post?.status?.receiptWindowStartActionSequenceId,
    },
  ];
  setupAssert(protocolWindowSegments.every((segment) =>
    Number.isSafeInteger(segment.receiptWindowStartEventSequenceId)
      && Number.isSafeInteger(segment.receiptWindowStartActionSequenceId)),
  "WEB06_LEARNED_PROTOCOL_WINDOW_START_MISSING");
  return {
    ...post,
    events,
    externalLifecycleEvents: [clone(lifecycleMarker)],
    protocolWindowSegments,
    actions,
    invalidations: [...(pre?.invalidations ?? []), ...(post?.invalidations ?? [])],
    mainObserverCallbacksMs: [...(pre?.mainObserverCallbacksMs ?? []), ...(post?.mainObserverCallbacksMs ?? [])],
    mainObserverCallbacks: [
      ...(pre?.mainObserverCallbacks ?? []).map((callback) => ({ ...callback, pageInstanceId: prePage })),
      ...(post?.mainObserverCallbacks ?? []).map((callback) => ({ ...callback, pageInstanceId: postPage })),
    ],
    status: {
      ...post?.status,
      mainObserverCallbackCount: (pre?.status?.mainObserverCallbackCount ?? 0)
        + (post?.status?.mainObserverCallbackCount ?? 0),
      mainObserverCallbackCapacity: (pre?.status?.mainObserverCallbackCapacity ?? 0)
        + (post?.status?.mainObserverCallbackCapacity ?? 0),
      mainObserverCallbackOverflowCount: (pre?.status?.mainObserverCallbackOverflowCount ?? 0)
        + (post?.status?.mainObserverCallbackOverflowCount ?? 0),
    },
  };
}

function validateTarget(id, target, identityManifest, expectation) {
  setupAssert(Object.hasOwn(WEB06_TARGET_PROTOCOL_MODES, id), "WEB06_TARGET_ID_INVALID", id);
  setupAssert(target && typeof target === "object" && !Array.isArray(target), "WEB06_TARGET_INVALID", id);
  let origin;
  try {
    origin = new URL(target.origin);
  } catch {
    throw new Web06CollectorSetupError("WEB06_TARGET_ORIGIN_INVALID", id);
  }
  setupAssert(origin.pathname === "/" && origin.username === "" && origin.password === ""
    && origin.search === "" && origin.hash === "", "WEB06_TARGET_ORIGIN_INVALID", id);
  if (expectation === "PREVIEW") {
    setupAssert(origin.protocol === "https:", "WEB06_PREVIEW_TARGET_HTTPS_REQUIRED", id);
  } else {
    setupAssert(origin.protocol === "http:" && ["127.0.0.1", "[::1]"].includes(origin.hostname)
      && origin.port !== "", "WEB06_TARGET_LOOPBACK_ORIGIN_REQUIRED", id);
  }
  setupAssert(COMMIT_RE.test(target.sourceCommit ?? ""), "WEB06_TARGET_SOURCE_COMMIT_INVALID", id);
  setupAssert(COMMIT_RE.test(target.sourceTree ?? ""), "WEB06_TARGET_SOURCE_TREE_INVALID", id);
  setupAssert(target.treeState === "clean", "WEB06_TARGET_TREE_NOT_CLEAN", id);
  setupAssert(isSha256(target.artifactSha256), "WEB06_TARGET_ARTIFACT_HASH_INVALID", id);
  setupAssert(isSha256(target.archiveSha256), "WEB06_TARGET_ARCHIVE_HASH_INVALID", id);
  setupAssert(isSha256(target.buildInfoSha256), "WEB06_TARGET_BUILD_INFO_HASH_INVALID", id);
  setupAssert(target.protocolMode === WEB06_TARGET_PROTOCOL_MODES[id], "WEB06_TARGET_PROTOCOL_MODE_INVALID", id);
  const expectedSelectorPolicy = id === "PRODUCT" || expectation === "PREVIEW" ? "omitted" : "explicit";
  setupAssert(target.selectorPolicy === expectedSelectorPolicy, "WEB06_TARGET_SELECTOR_POLICY_INVALID", id);
  const roleName = identityRoleForTarget(id);
  const pinned = validatePinnedIdentityRole(roleName, identityManifest.roles[roleName]);
  setupAssert(target.sourceCommit === pinned.sourceCommit, "WEB06_TARGET_SOURCE_COMMIT_NOT_PINNED", id);
  setupAssert(target.sourceTree === pinned.sourceTree, "WEB06_TARGET_SOURCE_TREE_NOT_PINNED", id);
  setupAssert(target.artifactSha256 === pinned.artifactManifestSha256, "WEB06_TARGET_ARTIFACT_NOT_PINNED", id);
  setupAssert(target.archiveSha256 === pinned.archiveSha256, "WEB06_TARGET_ARCHIVE_NOT_PINNED", id);
  setupAssert(target.buildInfoSha256 === pinned.buildInfoSha256, "WEB06_TARGET_BUILD_INFO_NOT_PINNED", id);
  const responseGuard = validateArtifactResponseGuard(target.artifactResponseGuard, target, id);
  return Object.freeze({
    id,
    origin: origin.toString(),
    sourceCommit: target.sourceCommit,
    sourceTree: target.sourceTree,
    treeState: target.treeState,
    artifactSha256: target.artifactSha256,
    archiveSha256: target.archiveSha256,
    buildInfoSha256: target.buildInfoSha256,
    artifactResponseGuard: responseGuard.guard,
    artifactResponseGuardSha256: responseGuard.sha256,
    protocolMode: target.protocolMode,
    selectorPolicy: target.selectorPolicy,
    identityRole: roleName,
    pinnedSelectedBranch: pinned.selectedBranch,
    pinnedDisposition: pinned.disposition,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
  });
}

/** Parse the opt-in lane. There are intentionally no in-repository defaults. */
export function parseWeb06CollectorEnvironment(env, { repoRoot }) {
  const evidenceRoot = env.YUNE_WEB06_EVIDENCE_ROOT;
  setupAssert(typeof evidenceRoot === "string" && path.isAbsolute(evidenceRoot), "WEB06_EVIDENCE_ROOT_ABSOLUTE_REQUIRED");
  const resolvedRepo = path.resolve(repoRoot);
  const resolvedEvidence = path.resolve(evidenceRoot);
  setupAssert(resolvedEvidence !== resolvedRepo && !isWithin(resolvedRepo, resolvedEvidence), "WEB06_EVIDENCE_ROOT_INSIDE_REPOSITORY");
  const outputPathSpecs = [
    ["collector", "YUNE_WEB06_COLLECTOR_OUTPUT_PATH", "collector-output.json"],
    ["independent", "YUNE_WEB06_INDEPENDENT_RECOMPUTE_PATH", "independent-recompute.json"],
    ["attestation", "YUNE_WEB06_SUITE_ATTESTATION_PATH", "suite-attestation.json"],
  ];
  const outputPaths = {};
  for (const [id, envName, fileName] of outputPathSpecs) {
    const value = env[envName];
    setupAssert(typeof value === "string" && path.isAbsolute(value), "WEB06_OUTPUT_PATH_ABSOLUTE_REQUIRED", envName);
    const resolved = path.resolve(value);
    setupAssert(isWithin(resolvedEvidence, resolved) && path.basename(resolved) === fileName,
      "WEB06_OUTPUT_PATH_INVALID", envName);
    outputPaths[id] = resolved;
  }
  setupAssert(new Set(Object.values(outputPaths)).size === outputPathSpecs.length,
    "WEB06_OUTPUT_PATH_DUPLICATE");

  const expectation = env.YUNE_WEB06_EXPECTATION;
  setupAssert(Object.hasOwn(WEB06_EXPECTATION_TARGET_ORDER, expectation ?? ""), "WEB06_EXPECTATION_INVALID");
  const runKind = env.YUNE_WEB06_RUN_KIND;
  setupAssert(["full", "preview-canary", "observer-overhead"].includes(runKind), "WEB06_RUN_KIND_INVALID");
  const expectedRunKind = expectation === "PREVIEW"
    ? "preview-canary"
    : expectation === "OBSERVER"
      ? "observer-overhead"
      : "full";
  setupAssert(runKind === expectedRunKind, "WEB06_EXPECTATION_RUN_KIND_MISMATCH");
  const branch = env.YUNE_WEB06_SELECTED_BRANCH;
  setupAssert(["NONE", "A", "B", "C"].includes(branch), "WEB06_SELECTED_BRANCH_INVALID");
  const disposition = env.YUNE_WEB06_DISPOSITION;
  setupAssert(["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE", "PRODUCTION_REDUCTION", "MEASURED_NO_GO"].includes(disposition), "WEB06_DISPOSITION_INVALID");
  if (["BASELINE", "OBSERVER"].includes(expectation)) {
    setupAssert(branch === "NONE", "WEB06_EXPECTATION_BRANCH_MISMATCH");
  }
  const expectedDisposition = expectation === "OBSERVER" ? "DIAGNOSTIC"
    : expectation === "BASELINE" ? "SOURCE_CURRENT_BASELINE"
      : branch === "NONE" ? "MEASURED_NO_GO" : "PRODUCTION_REDUCTION";
  setupAssert(disposition === expectedDisposition, "WEB06_EXPECTATION_DISPOSITION_MISMATCH");
  if (expectation === "PREVIEW") setupAssert(["A", "B", "C"].includes(branch), "WEB06_PREVIEW_PRODUCTION_BRANCH_REQUIRED");
  if (expectation === "FINAL") setupAssert(branch === "NONE" || ["A", "B", "C"].includes(branch), "WEB06_FINAL_BRANCH_INVALID");
  const requiredIdentityRoles = expectation === "OBSERVER"
    ? ["PRODUCT", "BASE"]
    : expectation === "BASELINE"
      ? ["BASE"]
      : ["FINAL"];
  const identity = parseIdentityManifest(env.YUNE_WEB06_IDENTITY_MANIFEST_JSON, requiredIdentityRoles);
  for (const roleName of requiredIdentityRoles) {
    const expectedBranch = roleName === "FINAL" ? branch : "NONE";
    setupAssert(identity.manifest.roles[roleName].selectedBranch === expectedBranch, "WEB06_IDENTITY_BRANCH_MISMATCH", roleName);
    const expectedRoleDisposition = roleName === "FINAL" ? disposition
      : roleName === "BASE" ? (expectation === "OBSERVER" ? "DIAGNOSTIC" : "SOURCE_CURRENT_BASELINE")
        : "DIAGNOSTIC";
    setupAssert(identity.manifest.roles[roleName].disposition === expectedRoleDisposition,
      "WEB06_IDENTITY_DISPOSITION_MISMATCH", roleName);
  }
  const runnerRole = identity.manifest.roles[expectation === "FINAL" || expectation === "PREVIEW" ? "FINAL" : "BASE"];
  const runnerSource = validateRunnerSourceManifest(
    parseJsonEnv(env.YUNE_WEB06_RUNNER_SOURCE_JSON, "RUNNER_SOURCE_JSON"),
    runnerRole,
  );
  const runEnvironment = validateRunEnvironmentManifest(env.YUNE_WEB06_RUN_ENVIRONMENT_JSON);
  const targetsValue = parseJsonEnv(env.YUNE_WEB06_TARGETS_JSON, "TARGETS_JSON");
  setupAssert(targetsValue && typeof targetsValue === "object" && !Array.isArray(targetsValue), "WEB06_TARGETS_INVALID");
  const targets = Object.fromEntries(Object.entries(targetsValue)
    .map(([id, target]) => [id, validateTarget(id, target, identity.manifest, expectation)]));

  const requiredTargets = parseJsonEnv(env.YUNE_WEB06_TARGET_ORDER_JSON, "TARGET_ORDER_JSON");
  setupAssert(Array.isArray(requiredTargets) && requiredTargets.length > 0, "WEB06_TARGET_ORDER_INVALID");
  setupAssert(requiredTargets.every((id) => typeof id === "string"), "WEB06_TARGET_ORDER_INVALID");
  setupAssert(new Set(requiredTargets).size === requiredTargets.length, "WEB06_TARGET_ORDER_DUPLICATE");
  for (const id of requiredTargets) setupAssert(hasOwn(targets, id), "WEB06_REQUIRED_TARGET_MISSING", id);
  const expectationTargets = WEB06_EXPECTATION_TARGET_ORDER[expectation];
  setupAssert(JSON.stringify(requiredTargets) === JSON.stringify(expectationTargets), "WEB06_EXPECTATION_TARGET_ORDER_MISMATCH");
  setupAssert(Object.keys(targets).length === expectationTargets.length
    && expectationTargets.every((id) => hasOwn(targets, id)), "WEB06_EXPECTATION_TARGET_SET_MISMATCH");

  const scenarioIds = parseJsonEnv(env.YUNE_WEB06_SCENARIOS_JSON, "SCENARIOS_JSON");
  setupAssert(Array.isArray(scenarioIds) && scenarioIds.length > 0, "WEB06_SCENARIOS_INVALID");
  setupAssert(new Set(scenarioIds).size === scenarioIds.length, "WEB06_SCENARIOS_DUPLICATE");
  for (const id of scenarioIds) {
    setupAssert(hasOwn(SCENARIO_RUN_REGISTRY, id), "WEB06_SCENARIO_RUN_UNKNOWN", String(id));
  }
  const blockedScenarios = parseJsonEnv(env.YUNE_WEB06_BLOCKED_SCENARIOS_JSON, "BLOCKED_SCENARIOS_JSON");
  setupAssert(Array.isArray(blockedScenarios), "WEB06_BLOCKED_SCENARIOS_INVALID");
  const blockedIds = [];
  for (const disposition of blockedScenarios) {
    setupAssert(disposition && typeof disposition === "object" && !Array.isArray(disposition), "WEB06_BLOCKED_DISPOSITION_INVALID");
    setupAssert(WEB06_BINDING_SCENARIO_ORDER.includes(disposition.scenarioRunId), "WEB06_BLOCKED_SCENARIO_UNKNOWN");
    setupAssert(disposition.disposition === "BLOCKED", "WEB06_BLOCKED_DISPOSITION_INVALID");
    setupAssert(COMMIT_RE.test(disposition.reviewCommit ?? ""), "WEB06_BLOCKED_REVIEW_COMMIT_INVALID");
    setupAssert(isSha256(disposition.planSha256), "WEB06_BLOCKED_PLAN_HASH_INVALID");
    setupAssert(typeof disposition.reasonCode === "string" && SAFE_SEGMENT_RE.test(disposition.reasonCode), "WEB06_BLOCKED_REASON_INVALID");
    blockedIds.push(disposition.scenarioRunId);
  }
  setupAssert(new Set(blockedIds).size === blockedIds.length, "WEB06_BLOCKED_SCENARIO_DUPLICATE");
  if (expectation === "PREVIEW") {
    setupAssert(JSON.stringify(scenarioIds) === JSON.stringify(WEB06_PREVIEW_SCENARIOS), "WEB06_PREVIEW_SCENARIOS_MISMATCH");
  }
  if (expectation === "OBSERVER") {
    setupAssert(JSON.stringify(scenarioIds) === JSON.stringify([WEB06_PHASE0_OVERHEAD_SCENARIO]), "WEB06_OBSERVER_SCENARIO_MISMATCH");
  }
  setupAssert(blockedScenarios.length === 0, "WEB06_BLOCKED_SCENARIOS_CANNOT_AUTHORIZE_EXECUTION");
  if (["BASELINE", "FINAL"].includes(expectation)) {
    const fullOrder = expectation === "FINAL" && branch === "B"
      ? [...WEB06_BINDING_SCENARIO_ORDER, ...WEB06_EXTENDED_SCENARIO_RUNS]
      : [...WEB06_BINDING_SCENARIO_ORDER];
    setupAssert(JSON.stringify(scenarioIds) === JSON.stringify(fullOrder), "WEB06_BINDING_SCENARIO_ORDER_MISMATCH");
  }

  if (scenarioIds.some((runId) => resolveScenarioRun(runId).scenarioId === "extended-scheduler-barriers")) {
    setupAssert(branch === "B", "WEB06_EXTENDED_SCENARIO_REQUIRES_BRANCH_B");
  }
  setupAssert(env.YUNE_WEB06_PLAYWRIGHT_RETRIES === "0", "WEB06_PLAYWRIGHT_RETRIES_MUST_BE_ZERO");
  setupAssert(env.YUNE_WEB06_PLAYWRIGHT_WORKERS === "1", "WEB06_PLAYWRIGHT_WORKERS_MUST_BE_ONE");

  const runId = env.YUNE_WEB06_RUN_ID;
  setupAssert(typeof runId === "string" && SAFE_SEGMENT_RE.test(runId), "WEB06_RUN_ID_INVALID");
  return Object.freeze({
    evidenceRoot: resolvedEvidence,
    repoRoot: resolvedRepo,
    runId,
    runKind,
    expectation,
    branch,
    disposition,
    identityManifest: Object.freeze(identity.manifest),
    identityManifestSha256: identity.sha256,
    runnerSource,
    runnerSourceManifestSha256: digestJson(runnerSource),
    scenarioIdsSha256: digestJson(scenarioIds),
    environmentManifest: Object.freeze(runEnvironment.manifest),
    environmentManifestSha256: runEnvironment.sha256,
    environmentId: runEnvironment.manifest.environmentId,
    targetOrder: [...requiredTargets],
    targets: Object.freeze(targets),
    scenarioIds: Object.freeze([...scenarioIds]),
    blockedScenarios: Object.freeze(clone(blockedScenarios)),
    outputPaths: Object.freeze(outputPaths),
  });
}

/**
 * Re-read the actual checkout immediately before browser setup. The release
 * runner's own HEAD/tree checks are defense in depth; this check binds the
 * exact spec/parser/verifier bytes that will interpret the measurement.
 */
export async function verifyWeb06RunnerSource(config) {
  const git = async (...args) => (await execFileAsync("git", args, {
    cwd: config.repoRoot,
    encoding: "utf8",
  })).stdout.trim();
  const [head, tree, status] = await Promise.all([
    git("rev-parse", "HEAD"),
    git("rev-parse", "HEAD^{tree}"),
    git("status", "--porcelain", "--untracked-files=all"),
  ]);
  setupAssert(head === config.runnerSource.sourceCommit, "WEB06_RUNNER_HEAD_MISMATCH");
  setupAssert(tree === config.runnerSource.sourceTree, "WEB06_RUNNER_TREE_MISMATCH");
  setupAssert(status === "", "WEB06_RUNNER_WORKTREE_NOT_CLEAN");
  const actualFiles = [];
  for (const expected of config.runnerSource.tooling.files) {
    const absolute = path.resolve(config.repoRoot, expected.path);
    setupAssert(isWithin(config.repoRoot, absolute), "WEB06_RUNNER_TOOLING_PATH_INVALID", expected.path);
    const bytes = await readFile(absolute);
    const actualSha256 = sha256(bytes);
    setupAssert(actualSha256 === expected.sha256, "WEB06_RUNNER_TOOLING_FILE_HASH_MISMATCH", expected.path);
    actualFiles.push({ path: expected.path, sha256: actualSha256 });
  }
  const snapshot = {
    version: "web06-runner-source-observation-v1",
    sourceCommit: head,
    sourceTree: tree,
    sourceTreeState: "clean",
    toolingManifestSha256: config.runnerSource.toolingManifestSha256,
    files: actualFiles,
  };
  return Object.freeze({ ...snapshot, observationSha256: digestJson(snapshot) });
}

export function validateObservedWeb06Environment(manifest, observed) {
  const expected = {
    toolchain: manifest.toolchain,
    host: manifest.host,
  };
  setupAssert(JSON.stringify(observed.toolchain) === JSON.stringify(expected.toolchain),
    "WEB06_OBSERVED_TOOLCHAIN_MISMATCH");
  setupAssert(JSON.stringify(observed.host) === JSON.stringify(expected.host),
    "WEB06_OBSERVED_HOST_MISMATCH");
  return true;
}

/** Observe the binding Mac rather than accepting a self-asserted environment JSON. */
export async function observeWeb06HostEnvironment(config, { browserVersion, browserExecutablePath }) {
  setupAssert(process.platform === "darwin", "WEB06_BINDING_HOST_NOT_MACOS");
  const command = async (program, args) => (await execFileAsync(program, args, {
    cwd: config.repoRoot,
    encoding: "utf8",
  })).stdout.trim();
  const [osVersion, osBuildVersion, cpuModel, logicalCores, memoryBytes, power, powerProfiles, rust, emscripten, npmVersion,
    playwrightPackage] = await Promise.all([
    command("sw_vers", ["-productVersion"]),
    command("sw_vers", ["-buildVersion"]),
    command("sysctl", ["-n", "machdep.cpu.brand_string"]),
    command("sysctl", ["-n", "hw.logicalcpu"]),
    command("sysctl", ["-n", "hw.memsize"]),
    command("pmset", ["-g", "batt"]),
    command("pmset", ["-g", "custom"]),
    command("rustc", ["--version"]),
    command("emcc", ["--version"]),
    command("npm", ["--version"]),
    readFile(path.join(config.repoRoot, "apps/yune-web/e2e/node_modules/@playwright/test/package.json"), "utf8"),
  ]);
  const acSection = /AC Power:\n([\s\S]*?)(?:\nBattery Power:|$)/.exec(powerProfiles)?.[1];
  setupAssert(typeof acSection === "string" && acSection.length > 0, "WEB06_OBSERVED_AC_POWER_PROFILE_MISSING");
  const lowPowerMatch = /\blowpowermode\s+([01])\b/.exec(acSection);
  setupAssert(lowPowerMatch !== null, "WEB06_OBSERVED_LOW_POWER_MODE_ROW_MISSING");
  setupAssert(typeof browserExecutablePath === "string" && path.isAbsolute(browserExecutablePath),
    "WEB06_CHROMIUM_EXECUTABLE_PATH_INVALID");
  const chromiumExecutableSha256 = sha256(await readFile(browserExecutablePath));
  const lowPowerMode = lowPowerMatch[1] === "1";
  const observed = {
    toolchain: {
      rust,
      emscripten: emscripten.split(/\r?\n/, 1)[0],
      node: process.version,
      npm: npmVersion,
      playwright: JSON.parse(playwrightPackage).version,
      chromium: browserVersion,
      chromiumExecutableSha256,
    },
    host: {
      os: "macOS",
      osVersion,
      osBuildVersion,
      arch: process.arch,
      cpuModel: cpuModel || os.cpus()[0]?.model,
      logicalCores: Number(logicalCores),
      memoryBytes: Number(memoryBytes),
      powerState: /AC Power/.test(power) ? "AC" : "battery",
      lowPowerMode,
    },
  };
  validateObservedWeb06Environment(config.environmentManifest, observed);
  const observation = {
    version: "web06-observed-environment-v1",
    ...observed,
  };
  return Object.freeze({ ...observation, observationSha256: digestJson(observation) });
}

async function ensureNoSymlinkDirectory(root, segments) {
  await mkdir(root, { recursive: true });
  const canonicalRoot = await realpath(root);
  let current = canonicalRoot;
  for (const segment of segments) {
    setupAssert(safeEvidenceSegment(segment), "WEB06_EVIDENCE_SEGMENT_INVALID", segment);
    current = path.join(current, segment);
    await mkdir(current, { recursive: true });
    const metadata = await lstat(current);
    setupAssert(!metadata.isSymbolicLink() && metadata.isDirectory(), "WEB06_EVIDENCE_SYMLINK_OR_NON_DIRECTORY", segment);
    const canonicalCurrent = await realpath(current);
    setupAssert(isWithin(canonicalRoot, canonicalCurrent), "WEB06_EVIDENCE_PATH_ESCAPE", segment);
    current = canonicalCurrent;
  }
  return { canonicalRoot, directory: current };
}

/** Reserve a create-new raw destination before the first measured interaction. */
export async function reserveRawEvidencePacket({ evidenceRoot, repoRoot, runId, sourceCommit, scenarioId, attemptId, mode,
  minimumFreeBytes = 16 * 1024 * 1024 }) {
  const canonicalRepo = await realpath(repoRoot);
  const rootInfo = await ensureNoSymlinkDirectory(evidenceRoot, []);
  setupAssert(rootInfo.canonicalRoot !== canonicalRepo && !isWithin(canonicalRepo, rootInfo.canonicalRoot), "WEB06_EVIDENCE_ROOT_INSIDE_REPOSITORY");
  setupAssert(COMMIT_RE.test(sourceCommit), "WEB06_RAW_SOURCE_COMMIT_INVALID");
  const { directory } = await ensureNoSymlinkDirectory(rootInfo.canonicalRoot, [sourceCommit, runId, scenarioId, attemptId]);
  setupAssert(SAFE_SEGMENT_RE.test(mode), "WEB06_RAW_MODE_INVALID", mode);
  const filesystem = await statfs(directory);
  const availableBytes = Number(filesystem.bavail) * Number(filesystem.bsize);
  setupAssert(Number.isFinite(availableBytes) && availableBytes >= minimumFreeBytes,
    "WEB06_RAW_EVIDENCE_FREE_SPACE_INSUFFICIENT");
  const destination = path.join(directory, `${mode}.raw.json`);
  const handle = await open(destination, "wx", 0o600);
  return {
    destination,
    evidenceRootCanonicalPath: rootInfo.canonicalRoot,
    handle,
    committed: false,
    availableBytes,
  };
}

const COMPLETED_RAW_ARRAY_FIELDS = Object.freeze([
  "events",
  "auxiliaryEvents",
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
]);
const UUID_V4_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

function runnerDenseArray(value) {
  if (!Array.isArray(value)) return false;
  for (let index = 0; index < value.length; index += 1) {
    if (!Object.prototype.hasOwnProperty.call(value, index)) return false;
  }
  return true;
}

function runnerClockBoundaryShape(boundary) {
  const count = WEB06_THRESHOLDS.calibration.exchangesPerBoundary;
  return runnerDenseArray(boundary) && boundary.length === count;
}

function runnerCalibrationShape(calibration) {
  return Boolean(calibration && typeof calibration === "object" && !Array.isArray(calibration)
    && runnerClockBoundaryShape(calibration.pre)
    && runnerClockBoundaryShape(calibration.post));
}

const COMPLETED_RAW_SENTINEL_ARRAY_FIELDS = Object.freeze([
  "events",
  "auxiliaryEvents",
  "unmatchedEvents",
  "snapshots",
  "interactionWindows",
  "idleControlWindows",
  "interactionFrameWindows",
  "interactionFrameTimestamps",
  "interactionFrameIntervalsMs",
  "longTasks",
  "focusVisibilitySamples",
  "assetsRequestedDuringWindow",
  "callbackLedger",
  "sentinelCallbacksMs",
  "unattributedInWindowCallbacksMs",
  "sentinelTotalPerEventMs",
  "sentinelTotalPerWindowMs",
]);

function runnerRawSentinelDecisionShape(sentinel) {
  if (!sentinel || typeof sentinel !== "object" || Array.isArray(sentinel)
    || COMPLETED_RAW_SENTINEL_ARRAY_FIELDS.some((field) => !runnerDenseArray(sentinel[field]))
    || !Number.isSafeInteger(sentinel.callbackLedgerCapacity) || sentinel.callbackLedgerCapacity < 0
    || !Number.isSafeInteger(sentinel.callbackLedgerOverflowCount) || sentinel.callbackLedgerOverflowCount < 0
    || !Number.isSafeInteger(sentinel.sentinelAccountedCallbackCount)
    || sentinel.sentinelAccountedCallbackCount < 0) return false;
  for (const field of ["events", "auxiliaryEvents", "unmatchedEvents", "snapshots",
    "interactionWindows", "idleControlWindows", "interactionFrameWindows", "longTasks",
    "focusVisibilitySamples", "assetsRequestedDuringWindow", "callbackLedger"]) {
    if (sentinel[field].some((row) => !row || typeof row !== "object" || Array.isArray(row))) return false;
  }
  if (sentinel.interactionFrameTimestamps.some((value) => !Number.isFinite(value))
    || sentinel.interactionFrameIntervalsMs.some((value) => !Number.isFinite(value))
    || sentinel.sentinelCallbacksMs.some((value) => !Number.isFinite(value))
    || sentinel.unattributedInWindowCallbacksMs.some((value) => !Number.isFinite(value))
    || sentinel.sentinelTotalPerEventMs.some((value) => !Number.isFinite(value))
    || sentinel.sentinelTotalPerWindowMs.some((value) => !Number.isFinite(value))) return false;
  const sentinelCallbackKeys = ["kind", "pageInstanceId", "startedAt", "finishedAt", "durationMs"];
  const sentinelCallbackOptionalKeys = ["eventSequenceId"];
  if (sentinel.callbackLedger.some((callback) =>
    sentinelCallbackKeys.some((key) => !Object.hasOwn(callback, key))
    || Object.keys(callback).some((key) =>
      !sentinelCallbackKeys.includes(key) && !sentinelCallbackOptionalKeys.includes(key))
    || typeof callback.kind !== "string" || callback.kind.length === 0
    || typeof callback.pageInstanceId !== "string" || callback.pageInstanceId.length === 0
    || (callback.eventSequenceId !== undefined
      && (!Number.isSafeInteger(callback.eventSequenceId) || callback.eventSequenceId <= 0))
    || !Number.isFinite(callback.startedAt) || !Number.isFinite(callback.finishedAt)
    || !Number.isFinite(callback.durationMs))) return false;
  return true;
}

export function sentinelLedgerIntegrityErrors(sentinel) {
  const errors = [];
  const callbacks = sentinel?.callbackLedger;
  const events = sentinel?.events;
  const windows = sentinel?.interactionWindows;
  if (!Array.isArray(callbacks) || !Array.isArray(events) || !Array.isArray(windows)) {
    return ["SENTINEL_CALLBACK_LEDGER_MISSING"];
  }
  if (!Number.isSafeInteger(sentinel.callbackLedgerCapacity)
    || sentinel.callbackLedgerCapacity < callbacks.length) {
    errors.push("SENTINEL_CALLBACK_CAPACITY_EXCEEDED");
  }
  if (!Number.isSafeInteger(sentinel.callbackLedgerOverflowCount)
    || sentinel.callbackLedgerOverflowCount !== 0) {
    errors.push("SENTINEL_CALLBACK_LEDGER_OVERFLOW");
  }
  if (callbacks.some((callback) => !Number.isFinite(callback?.startedAt)
    || !Number.isFinite(callback?.finishedAt)
    || !Number.isFinite(callback?.durationMs)
    || callback.startedAt < 0
    || callback.finishedAt < callback.startedAt
    || callback.durationMs < 0
    || callback.durationMs !== callback.finishedAt - callback.startedAt)) {
    errors.push("SENTINEL_CALLBACK_TIMING_INVALID");
  }
  if (JSON.stringify(sentinel.sentinelCallbacksMs)
    !== JSON.stringify(callbacks.map((callback) => callback?.durationMs))) {
    errors.push("SENTINEL_CALLBACK_DURATION_PROJECTION_MISMATCH");
  }
  const unattributed = callbacks
    .filter((callback) => callback?.eventSequenceId === undefined)
    .map((callback) => callback?.durationMs);
  if (JSON.stringify(sentinel.unattributedInWindowCallbacksMs) !== JSON.stringify(unattributed)) {
    errors.push("SENTINEL_UNATTRIBUTED_CALLBACK_PROJECTION_MISMATCH");
  }
  const eventOwners = new Map();
  for (const event of events) {
    const key = `${event?.pageInstanceId}:${event?.eventSequenceId}`;
    eventOwners.set(key, (eventOwners.get(key) ?? 0) + 1);
  }
  if ([...eventOwners.values()].some((count) => count !== 1)) {
    errors.push("SENTINEL_EVENT_OWNER_IDENTITY_INVALID");
  }
  if (callbacks.some((callback) => callback?.eventSequenceId !== undefined
    && eventOwners.get(`${callback?.pageInstanceId}:${callback.eventSequenceId}`) !== 1)) {
    errors.push("SENTINEL_CALLBACK_OWNER_MISSING");
  }
  if (sentinel.sentinelAccountedCallbackCount !== callbacks.length) {
    errors.push("SENTINEL_CALLBACK_COUNT_MISMATCH");
  }
  const eventTotals = events.map((event) => callbacks
    .filter((callback) => callback?.pageInstanceId === event?.pageInstanceId
      && callback?.eventSequenceId === event?.eventSequenceId)
    .reduce((sum, callback) => sum + callback.durationMs, 0));
  if (JSON.stringify(sentinel.sentinelTotalPerEventMs) !== JSON.stringify(eventTotals)) {
    errors.push("SENTINEL_EVENT_TOTAL_PROJECTION_MISMATCH");
  }
  const windowTotals = windows.map((window) => callbacks
    .filter((callback) => callback?.pageInstanceId === window?.pageInstanceId
      && callback?.startedAt >= window?.startedAt && callback?.startedAt <= window?.endedAt)
    .reduce((sum, callback) => sum + callback.durationMs, 0));
  if (JSON.stringify(sentinel.sentinelTotalPerWindowMs) !== JSON.stringify(windowTotals)) {
    errors.push("SENTINEL_WINDOW_TOTAL_PROJECTION_MISMATCH");
  }
  return [...new Set(errors)];
}

function runnerRawProtocolDecisionShape(envelope) {
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
    if (!runnerDenseArray(protocol[field])) return false;
  }
  for (const field of ["events", "actions", "mainObserverCallbacks", "protocolWindowSegments"]) {
    if (protocol[field].some((row) => !row || typeof row !== "object" || Array.isArray(row))) return false;
  }
  if (protocol.invalidations.some((value) => typeof value !== "string")
    || protocol.mainObserverCallbacksMs.some((value) => !Number.isFinite(value))) return false;
  const callbackKeys = ["callbackId", "sequenceId", "operation", "startedAt", "finishedAt", "durationMs"];
  const callbackOptionalKeys = ["actionId", "eventId", "pageInstanceId"];
  if (protocol.mainObserverCallbacks.some((callback) =>
    callbackKeys.some((key) => !Object.hasOwn(callback, key))
    || Object.keys(callback).some((key) =>
      !callbackKeys.includes(key) && !callbackOptionalKeys.includes(key))
    || typeof callback.callbackId !== "string" || callback.callbackId.length === 0
    || !Number.isSafeInteger(callback.sequenceId) || callback.sequenceId <= 0
    || typeof callback.operation !== "string" || callback.operation.length === 0
    || ["actionId", "eventId", "pageInstanceId"].some((key) =>
      callback[key] !== undefined && (typeof callback[key] !== "string" || callback[key].length === 0))
    || !Number.isFinite(callback.startedAt)
    || !Number.isFinite(callback.finishedAt) || !Number.isFinite(callback.durationMs))) return false;
  for (const action of protocol.actions) {
    if (!action.worker || typeof action.worker !== "object" || Array.isArray(action.worker)) return false;
    for (const field of ["runtimeSpans", "adapterSpans", "persistenceSpans", "collectorSpans",
      "observerFailures"]) {
      if (!runnerDenseArray(action.worker[field])) return false;
    }
    for (const field of ["runtimeSpans", "adapterSpans", "persistenceSpans", "collectorSpans"]) {
      if (action.worker[field].some((row) => !row || typeof row !== "object" || Array.isArray(row))) return false;
    }
    if (action.worker.collectorSpans.some((span) =>
      !Number.isFinite(span.startedAt) || !Number.isFinite(span.finishedAt))) return false;
    if (action.worker.observerFailures.some((value) => typeof value !== "string")) return false;
  }
  return true;
}

function runnerReceiptDecisionShape(receipt, scenarioId, scenarioRunId, schemaId, { common }) {
  if (!receipt || typeof receipt !== "object" || Array.isArray(receipt)
    || receipt.scenarioId !== scenarioId || receipt.scenarioRunId !== scenarioRunId
    || receipt.schemaId !== schemaId || receipt.measurementStarted !== true
    || receipt.measurementCompleted !== true
    || COMPLETED_RAW_ARRAY_FIELDS.some((field) => !runnerDenseArray(receipt[field]))
    || (common && (!runnerDenseArray(receipt.commonSamples)
      || !runnerDenseArray(receipt.unmatchedEvents) || !runnerDenseArray(receipt.actions)))
    || (!common && (!runnerDenseArray(receipt.actions)
      || !runnerDenseArray(receipt.pressureProofs) || !runnerDenseArray(receipt.burstRecoveries)))
    || !receipt.sentinelOverflowCounts || typeof receipt.sentinelOverflowCounts !== "object"
    || Array.isArray(receipt.sentinelOverflowCounts)) {
    return false;
  }
  for (const field of ["auxiliaryEvents", "unmatchedEvents"]) {
    if (receipt[field] !== undefined
      && receipt[field].some((row) => !row || typeof row !== "object" || Array.isArray(row))) return false;
  }
  if (receipt.cadenceGaps.some((gap) => !gap || typeof gap !== "object" || Array.isArray(gap)
    || typeof gap.stepId !== "string" || !Number.isFinite(gap.nominalGapMs)
    || !Number.isFinite(gap.actualDriverGapMs)
    || (gap.rebasedAfterLateHost !== undefined && typeof gap.rebasedAfterLateHost !== "boolean"))) return false;
  for (const field of ["idleFrameIntervalsMs", "interactionFrameIntervalsMs",
    "interactionFrameTimestamps"]) {
    if (receipt[field].some((value) => !Number.isFinite(value))) return false;
  }
  if (receipt.events.some((event) => !event || typeof event !== "object"
    || Array.isArray(event) || !Number.isSafeInteger(event.eventSequenceId)
    || !Number.isFinite(event.eventTimestamp) || !Number.isFinite(event.normalizedEventAt)
    || !UUID_V4_RE.test(event.pageInstanceId ?? ""))) return false;
  if (receipt.focusVisibilitySamples.some((sample) => !sample || typeof sample !== "object"
    || !Number.isFinite(sample.recordedAt) || typeof sample.focused !== "boolean"
    || typeof sample.visibilityState !== "string"
    || !UUID_V4_RE.test(sample.pageInstanceId ?? ""))) return false;
  for (const field of ["interactionFrameWindows", "interactionWindows", "idleControlWindows"]) {
    if (receipt[field].some((row) => !row || typeof row !== "object"
      || Array.isArray(row)
      || !UUID_V4_RE.test(row.pageInstanceId ?? ""))) return false;
  }
  if (receipt.interactionFrameWindows.some((row) => typeof row.windowId !== "string")
    || receipt.interactionWindows.some((row) => typeof row.windowId !== "string")) return false;
  if (receipt.interactionFrameWindows.some((row) =>
    !runnerDenseArray(row.timestamps) || !runnerDenseArray(row.intervalsMs)
    || row.timestamps.some((value) => !Number.isFinite(value))
    || row.intervalsMs.some((value) => !Number.isFinite(value)))) return false;
  if (receipt.interactionWindows.some((row) =>
    ["startedAt", "endedAt", "startBoundaryRafAt", "endBoundaryRafAt"]
      .some((field) => !Number.isFinite(row[field])))) return false;
  if (receipt.idleControlWindows.some((row) =>
    typeof row.controlId !== "string" || !Number.isFinite(row.startedAt)
    || !Number.isFinite(row.endedAt))) return false;
  if (receipt.assetsRequestedDuringWindow.some((row) => !row || typeof row !== "object"
    || Array.isArray(row) || typeof row.name !== "string" || !Number.isFinite(row.startTime))) return false;
  if (receipt.measurementProtocolBlockers.some((value) => typeof value !== "string")) return false;
  if (receipt.actions.some((action) => !action || typeof action !== "object" || Array.isArray(action))) return false;
  if (common && receipt.commonSamples.some((sample) => !sample || typeof sample !== "object"
    || Array.isArray(sample) || !Number.isFinite(sample.observedAt)
    || !sample.domObserved || typeof sample.domObserved !== "object"
    || Array.isArray(sample.domObserved)
    || (sample.firstDomObserved !== undefined
      && (!sample.firstDomObserved || typeof sample.firstDomObserved !== "object"
        || Array.isArray(sample.firstDomObserved))))) return false;
  if (!common && (receipt.pressureProofs.some((row) => !row || typeof row !== "object" || Array.isArray(row))
    || receipt.burstRecoveries.some((row) => !row || typeof row !== "object" || Array.isArray(row)))) {
    return false;
  }
  if (receipt.longTasks.some((task) => !task || typeof task !== "object"
    || Array.isArray(task) || !UUID_V4_RE.test(task.pageInstanceId ?? "")
    || !Number.isFinite(task.startTime) || !Number.isFinite(task.durationMs)
    || typeof task.overlapsInteractionWindow !== "boolean"
    || typeof task.overlapsIdleControl !== "boolean")) return false;
  if (!common && receipt.actions.some((action) => !action || typeof action !== "object"
    || ["presentationExpected", "domObserved"].some((field) =>
      action[field] !== undefined && action[field] !== null
      && (typeof action[field] !== "object" || Array.isArray(action[field]))))) return false;
  if (scenarioId === "learned-row") {
    for (const segmentName of ["preReload", "postReload"]) {
      const segment = receipt.calibrationSegments?.[segmentName];
      if (!runnerCalibrationShape(segment?.driver)
        || (!common && !runnerCalibrationShape(segment?.worker))) return false;
    }
    return true;
  }
  return runnerCalibrationShape(receipt.calibration?.driver)
    && (common || runnerCalibrationShape(receipt.calibration?.worker));
}

/**
 * Frozen producer-side structural gate for a completed raw attempt. Semantic
 * REDs remain parser-owned; malformed/coercive shapes become one setup issue
 * before raw persistence or summary construction.
 */
export function validateCompletedRawDecisionShape(envelope) {
  const scenarioId = envelope?.scenarioId;
  const scenarioRunId = envelope?.scenarioRunId;
  const schemaId = envelope?.schemaId;
  const run = hasOwn(SCENARIO_RUN_REGISTRY, scenarioRunId)
    ? SCENARIO_RUN_REGISTRY[scenarioRunId]
    : undefined;
  const pass = envelope?.version === "web06-raw-attempt-v1"
    && envelope.measurementStarted === true && envelope.measurementCompleted === true
    && hasOwn(SCENARIO_REGISTRY, scenarioId)
    && run?.scenarioId === scenarioId && run?.schema === schemaId
    && runnerRawSentinelDecisionShape(envelope.sentinel)
    && runnerRawProtocolDecisionShape(envelope)
    && runnerReceiptDecisionShape(envelope.commonReceipt, scenarioId, scenarioRunId, schemaId, { common: true })
    && (envelope.privateReceipt === undefined
      || runnerReceiptDecisionShape(envelope.privateReceipt, scenarioId, scenarioRunId, schemaId, { common: false }));
  return Object.freeze({
    pass,
    errors: pass ? [] : ["WEB06_COMPLETED_RAW_DECISION_SHAPE_INVALID"],
  });
}

function runnerRetainedReceiptBindingValid(envelope) {
  const target = envelope?.target ?? {};
  if (target.protocolMode === "off"
    && (envelope.privateReceipt !== undefined || envelope.protocolExport !== undefined)) return false;
  const expectedSource = {
    commit: target.sourceCommit,
    tree: target.sourceTree,
    treeState: target.treeState,
    archiveSha256: target.archiveSha256,
    buildInfoSha256: target.buildInfoSha256,
    artifactSha256: target.artifactSha256,
    artifactResponseGuardSha256: target.artifactResponseGuardSha256,
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
  const receipts = [envelope.commonReceipt, envelope.privateReceipt].filter((receipt) => receipt !== undefined);
  const roundIds = [];
  for (const receipt of receipts) {
    if (!receipt || typeof receipt !== "object" || Array.isArray(receipt)) return false;
    for (const [field, expected] of Object.entries({
      scenarioRunId: envelope.scenarioRunId,
      scenarioId: envelope.scenarioId,
      schemaId: envelope.schemaId,
      attemptId: envelope.attemptId,
      mode: target.id,
      measurementStarted: true,
      measurementCompleted: true,
    })) if (receipt[field] !== expected) return false;
    if (typeof receipt.roundId !== "string" || receipt.roundId.length === 0) return false;
    roundIds.push(receipt.roundId);
    if (Number.isSafeInteger(envelope.attemptNumber)
      && receipt.roundId !== `${envelope.scenarioId}-round-${envelope.attemptNumber}`) return false;
    if (!receipt.source || typeof receipt.source !== "object" || Array.isArray(receipt.source)) return false;
    for (const [field, expected] of Object.entries(expectedSource)) {
      if (receipt.source[field] !== expected) return false;
    }
    if (!isSha256(receipt.source.artifactResponseGuardSummarySha256)) return false;
  }
  if (new Set(roundIds).size > 1) return false;
  const sameProjection = (left, right) =>
    JSON.stringify(stableJsonValue(left)) === JSON.stringify(stableJsonValue(right));
  const common = envelope.commonReceipt;
  const internal = envelope.privateReceipt;
  const evidence = envelope.measurementEvidence;
  if (!common || !evidence || typeof evidence !== "object" || Array.isArray(evidence)
    || !sameProjection(common.eventClockProbe, evidence.eventClockProbe)
    || !sameProjection(common.eventClockSegments, evidence.eventClockSegments)
    || !sameProjection(common.calibration, { driver: evidence.calibration?.driver })
    || !sameProjection(common.calibrationSegments, evidence.calibrationSegments === undefined ? undefined : {
      preReload: { driver: evidence.calibrationSegments.preReload?.driver },
      postReload: { driver: evidence.calibrationSegments.postReload?.driver },
    })
    || !sameProjection(common.idleFrameIntervalsMs, evidence.idleFrameIntervalsMs)
    || !sameProjection(common.idleFrameSegments, evidence.idleFrameSegments)
    || !sameProjection(common.cadenceGaps, envelope.drive?.cadenceGaps)
    || !sameProjection(common.lifecycleContinuity, envelope.drive?.learned?.lifecycleContinuity)) return false;
  if (internal && (!sameProjection(internal.eventClockProbe, evidence.eventClockProbe)
    || !sameProjection(internal.eventClockSegments, evidence.eventClockSegments)
    || !sameProjection(internal.calibration, evidence.calibration)
    || !sameProjection(internal.calibrationSegments, evidence.calibrationSegments)
    || !sameProjection(internal.idleFrameIntervalsMs, evidence.idleFrameIntervalsMs)
    || !sameProjection(internal.idleFrameSegments, evidence.idleFrameSegments)
    || !sameProjection(internal.cadenceGaps, envelope.drive?.cadenceGaps)
    || !sameProjection(internal.burstRecoveries, envelope.drive?.burstRecoveries)
    || !sameProjection(internal.lifecycleContinuity, envelope.drive?.learned?.lifecycleContinuity))) return false;
  if (internal) {
    for (const field of ["auxiliaryEvents", "idleFrameIntervalsMs", "idleFrameSegments",
      "interactionFrameIntervalsMs", "interactionFrameTimestamps", "interactionFrameWindows",
      "interactionWindows", "idleControlWindows", "longTaskObserver", "longTasks",
      "focusVisibilitySamples", "assetsRequestedDuringWindow", "measurementProtocolBlockers",
      "lifecycleContinuity", "sentinelOverflowCounts"]) {
      if (!sameProjection(internal[field], common[field])) return false;
    }
  }
  return true;
}

export function retainedRawReceiptBehaviorErrors(envelope) {
  if (!runnerRetainedReceiptBindingValid(envelope)) return [];
  const scenarioId = envelope?.scenarioId;
  const scenarioRunId = envelope?.scenarioRunId;
  const schemaId = envelope?.schemaId;
  const common = envelope?.commonReceipt;
  const internal = envelope?.privateReceipt;
  if (!runnerReceiptDecisionShape(common, scenarioId, scenarioRunId, schemaId, { common: true })) return [];
  if (envelope?.target?.protocolMode !== "off"
    && !runnerReceiptDecisionShape(internal, scenarioId, scenarioRunId, schemaId, { common: false })) return [];
  const parsed = [
    validateCommonSurfaceReceipt(common),
    ...(internal === undefined ? [] : [validateAndRecomputeReceipt(internal)]),
  ];
  return [...new Set(parsed.flatMap((result) => result.behaviorErrors ?? []))];
}

export function buildIncompleteObserverModeProjection({
  rawPacket,
  measurementStarted,
  behaviorRedObserved,
}) {
  const behaviorRed = behaviorRedObserved === true;
  const verdict = behaviorRed ? "RED_BEHAVIOR" : "SETUP_INVALID";
  return {
    rawPacket,
    measurementStarted: measurementStarted === true,
    measurementCompleted: false,
    measurementValid: false,
    behaviorRedObserved: behaviorRed,
    hardRedBindingValid: false,
    hardRedObserved: behaviorRed,
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

/** Durably commit exact raw bytes to an already-exclusive reservation. */
export async function commitRawEvidencePacket({ reservation, packet }) {
  setupAssert(reservation?.handle && reservation.committed === false, "WEB06_RAW_RESERVATION_INVALID");
  const bytes = Buffer.from(`${JSON.stringify(packet, null, 2)}\n`, "utf8");
  const digest = sha256(bytes);
  try {
    await reservation.handle.writeFile(bytes);
    await reservation.handle.sync();
  } finally {
    await reservation.handle.close();
  }
  reservation.committed = true;
  return Object.freeze({
    rawPacketSha256: digest,
    rawPacketBytes: bytes.length,
    rawPath: reservation.destination,
    evidenceRootCanonicalPath: reservation.evidenceRootCanonicalPath,
  });
}

/** Build a stable public reference from the canonical root used for reservation. */
export function rawEvidencePacketReference(raw) {
  setupAssert(typeof raw?.evidenceRootCanonicalPath === "string"
    && path.isAbsolute(raw.evidenceRootCanonicalPath)
    && typeof raw?.rawPath === "string" && path.isAbsolute(raw.rawPath),
  "WEB06_RAW_REFERENCE_IDENTITY_INVALID");
  const relativePath = path.relative(raw.evidenceRootCanonicalPath, raw.rawPath).split(path.sep).join("/");
  setupAssert(relativePath && !relativePath.startsWith("../") && !path.isAbsolute(relativePath),
    "WEB06_RAW_REFERENCE_OUTSIDE_EVIDENCE_ROOT");
  return Object.freeze({ relativePath, bytes: raw.rawPacketBytes, sha256: raw.rawPacketSha256 });
}

/** Convenience wrapper retained for setup/tests; binding harnesses reserve first. */
export async function writeRawEvidencePacket(options) {
  const reservation = await reserveRawEvidencePacket(options);
  return commitRawEvidencePacket({ reservation, packet: options.packet });
}

export async function writeCompactEvidenceReceipt({ evidenceRoot, repoRoot, runId, sourceCommit, scenarioId, attemptId, mode, receipt }) {
  const privacy = validatePointerFreePrivacy(receipt);
  setupAssert(privacy.pass, "WEB06_PUBLIC_EVIDENCE_PRIVACY", privacy.errors.join(","));
  const canonicalRepo = await realpath(repoRoot);
  const rootInfo = await ensureNoSymlinkDirectory(evidenceRoot, []);
  setupAssert(rootInfo.canonicalRoot !== canonicalRepo && !isWithin(canonicalRepo, rootInfo.canonicalRoot), "WEB06_EVIDENCE_ROOT_INSIDE_REPOSITORY");
  const { directory } = await ensureNoSymlinkDirectory(rootInfo.canonicalRoot, [sourceCommit, runId, "compact", scenarioId, attemptId]);
  setupAssert(SAFE_SEGMENT_RE.test(mode), "WEB06_COMPACT_MODE_INVALID", mode);
  const destination = path.join(directory, `${mode}.public.json`);
  const bytes = Buffer.from(`${JSON.stringify(receipt, null, 2)}\n`, "utf8");
  const handle = await open(destination, "wx", 0o644);
  try {
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
  return Object.freeze({ publicReceiptSha256: sha256(bytes), publicReceiptBytes: bytes.length, publicPath: destination });
}

function schemaExactKeys(value, keys, errors, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    errors.push(`${label}:object`);
    return false;
  }
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) errors.push(`${label}:keys:${actual.join(",")}`);
  return true;
}

function schemaArtifactReference(value, errors, label, expectedName) {
  if (!schemaExactKeys(value, ["relativePath", "bytes", "sha256"], errors, label)) return;
  if (typeof value.relativePath !== "string" || path.isAbsolute(value.relativePath)
    || value.relativePath.split("/").some((segment) => segment === "..")
    || (expectedName && path.posix.basename(value.relativePath) !== expectedName)
    || !Number.isSafeInteger(value.bytes) || value.bytes <= 0 || !isSha256(value.sha256)) {
    errors.push(`${label}:values`);
  }
}

function schemaRunnerSource(value, errors, label) {
  if (!schemaExactKeys(value,
    ["version", "sourceCommit", "sourceTree", "sourceTreeState", "tooling", "toolingManifestSha256"], errors, label)) return;
  if (value.version !== "web06-runner-source-v1" || !COMMIT_RE.test(value.sourceCommit ?? "")
    || !COMMIT_RE.test(value.sourceTree ?? "") || value.sourceTreeState !== "clean"
    || !isSha256(value.toolingManifestSha256)) errors.push(`${label}:identity`);
  if (!schemaExactKeys(value.tooling, ["version", "files"], errors, `${label}.tooling`)) return;
  if (value.tooling.version !== "web06-runner-tooling-v1" || !Array.isArray(value.tooling.files)
    || JSON.stringify(value.tooling.files.map((file) => file?.path)) !== JSON.stringify(WEB06_RUNNER_TOOLING_PATHS)) {
    errors.push(`${label}.tooling:identity`);
    return;
  }
  value.tooling.files.forEach((file, index) => {
    if (!schemaExactKeys(file, ["path", "sha256"], errors, `${label}.tooling.files[${index}]`)) return;
    if (file.path !== WEB06_RUNNER_TOOLING_PATHS[index] || !isSha256(file.sha256)) {
      errors.push(`${label}.tooling.files[${index}]:values`);
    }
  });
}

function schemaRoundSummary(value, errors, label) {
  const result = validatePublicRoundSummarySchema(value);
  if (!result.pass) errors.push(...result.errors.map((error) => `${label}:${error}`));
}

function schemaFiveRoundSummary(value, errors, label) {
  const result = validatePublicFiveRoundSummarySchema(value);
  if (!result.pass) errors.push(...result.errors.map((error) => `${label}:${error}`));
}

function schemaSurfaceMap(value, errors, label, validator) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    errors.push(`${label}:object`);
    return;
  }
  const keys = Object.keys(value);
  if (keys.some((key) => !["common", "internal"].includes(key)) || new Set(keys).size !== keys.length) {
    errors.push(`${label}:surface-keys`);
  }
  for (const key of keys) validator(value[key], errors, `${label}.${key}`);
}

function schemaRunnerAttempt(value, errors, label) {
  const keys = ["attemptId", "measurementStarted", "measurementCompleted", "classification", "retainedMeasured",
    "retainedLogicalRound", "validForLatencyFrame", "retainedHardRed", "retryEligible", "validRedObserved",
    "rawPacket", "runnerSummaries"];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (typeof value.attemptId !== "string" || typeof value.classification !== "string"
    || !["PASS", "RED", "SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(value.classification)
    || ["measurementStarted", "measurementCompleted", "retainedMeasured", "retainedLogicalRound", "validForLatencyFrame",
      "retainedHardRed", "retryEligible", "validRedObserved"].some((key) => typeof value[key] !== "boolean")) {
    errors.push(`${label}:values`);
  }
  if (value.measurementCompleted && !value.measurementStarted) errors.push(`${label}:completed-without-start`);
  if (value.retainedMeasured !== value.retainedLogicalRound
    || (value.validForLatencyFrame && !value.retainedLogicalRound)
    || value.validRedObserved !== value.retainedHardRed
    || value.retryEligible !== ["SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(value.classification)
    || (value.retainedHardRed && value.classification !== "RED")) errors.push(`${label}:classification-facts`);
  schemaArtifactReference(value.rawPacket, errors, `${label}.rawPacket`);
  schemaSurfaceMap(value.runnerSummaries, errors, `${label}.runnerSummaries`, schemaRoundSummary);
  const summaryKeys = Object.keys(value.runnerSummaries ?? {}).sort();
  if (value.measurementCompleted === true && JSON.stringify(summaryKeys) !== JSON.stringify(["common", "internal"])) {
    errors.push(`${label}.runnerSummaries:completed-surfaces`);
  }
  if (value.measurementCompleted === false && summaryKeys.length !== 0) {
    errors.push(`${label}.runnerSummaries:partial-surfaces`);
  }
}

function schemaRunnerScenario(value, errors, label, expectation) {
  const keys = ["targetId", "scenarioRunId", "scenarioId", "schemaId", "measuredRoundCount",
    "validLatencyFrameRoundCount", "verdict",
    "preservedHardRedAttemptIds", "preservedHardRedObserved", "attempts", "runnerFiveRoundSummaries"];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (typeof value.targetId !== "string" || typeof value.scenarioRunId !== "string"
    || typeof value.scenarioId !== "string" || typeof value.schemaId !== "string"
    || !Number.isSafeInteger(value.measuredRoundCount) || value.measuredRoundCount < 0
    || !Number.isSafeInteger(value.validLatencyFrameRoundCount) || value.validLatencyFrameRoundCount < 0
    || !["PASS", "RED", "SETUP_NO_GO", "SETUP_INVALID"].includes(value.verdict)
    || typeof value.preservedHardRedObserved !== "boolean"
    || !Array.isArray(value.preservedHardRedAttemptIds)
    || value.preservedHardRedAttemptIds.some((id) => typeof id !== "string")
    || !Array.isArray(value.attempts)) errors.push(`${label}:values`);
  const run = hasOwn(SCENARIO_RUN_REGISTRY, value.scenarioRunId)
    ? SCENARIO_RUN_REGISTRY[value.scenarioRunId]
    : undefined;
  if (run?.scenarioId !== value.scenarioId || run?.schema !== value.schemaId) errors.push(`${label}:run-identity`);
  (value.attempts ?? []).forEach((attempt, index) => {
    schemaRunnerAttempt(attempt, errors, `${label}.attempts[${index}]`);
    if (attempt.attemptId !== `attempt-${index + 1}`) errors.push(`${label}.attempts[${index}]:attempt-id`);
  });
  const measuredCount = value.attempts?.filter((attempt) => attempt.retainedMeasured === true).length;
  const validLatencyFrameCount = value.attempts?.filter((attempt) => attempt.validForLatencyFrame === true).length;
  const hardRedIds = value.attempts?.filter((attempt) => attempt.retainedHardRed === true)
    .map((attempt) => attempt.attemptId) ?? [];
  if (value.measuredRoundCount !== measuredCount
    || value.validLatencyFrameRoundCount !== validLatencyFrameCount
    || value.validLatencyFrameRoundCount > value.measuredRoundCount
    || JSON.stringify(value.preservedHardRedAttemptIds) !== JSON.stringify(hardRedIds)
    || value.preservedHardRedObserved !== (hardRedIds.length > 0)) errors.push(`${label}:attempt-reconciliation`);
  schemaSurfaceMap(value.runnerFiveRoundSummaries, errors, `${label}.runnerFiveRoundSummaries`, schemaFiveRoundSummary);
  if (expectation === "PREVIEW" && Object.keys(value.runnerFiveRoundSummaries ?? {}).length !== 0) {
    errors.push(`${label}:preview-five-round-summary`);
  } else if (expectation === "PREVIEW" && value.attempts?.length !== 1) {
    errors.push(`${label}:preview-attempt-count`);
  } else if (expectation !== "PREVIEW") {
    const terminalIncompleteRed = value.attempts?.at(-1)?.measurementCompleted === false
      && value.attempts.at(-1).retainedHardRed === true;
    if (value.attempts?.length > WEB06_THRESHOLDS.attempts.maximum
      || (value.measuredRoundCount < WEB06_THRESHOLDS.attempts.requiredValid
        && value.attempts?.length !== WEB06_THRESHOLDS.attempts.maximum
        && !terminalIncompleteRed)) {
      errors.push(`${label}:attempt-count`);
    }
    const fiveKeys = Object.keys(value.runnerFiveRoundSummaries ?? {}).sort();
    if (value.validLatencyFrameRoundCount === WEB06_THRESHOLDS.attempts.requiredValid
      && JSON.stringify(fiveKeys) !== JSON.stringify(["common", "internal"])) {
      errors.push(`${label}.runnerFiveRoundSummaries:surfaces`);
    }
    if (value.validLatencyFrameRoundCount !== WEB06_THRESHOLDS.attempts.requiredValid && fiveKeys.length !== 0) {
      errors.push(`${label}.runnerFiveRoundSummaries:incomplete-surfaces`);
    }
  }
  const required = expectation === "PREVIEW" ? 1 : WEB06_THRESHOLDS.attempts.requiredValid;
  const expectedVerdict = value.measuredRoundCount === required
    ? (value.attempts?.some((attempt) => attempt.classification === "RED") ? "RED" : "PASS")
    : expectation === "PREVIEW"
      ? "SETUP_INVALID" : "SETUP_NO_GO";
  if (value.verdict !== expectedVerdict) errors.push(`${label}:verdict-reconciliation`);
}

const OBSERVER_MODE_BASE_KEYS = Object.freeze([
  "rawPacket", "measurementStarted", "measurementCompleted", "measurementValid",
  "behaviorRedObserved", "hardRedBindingValid", "hardRedObserved", "samples",
  "commonVerdict", "internalVerdict", "commonEventCount", "interactionWindowCount", "sentinelCallbacksMs",
  "sentinelTotalPerEventMs", "sentinelTotalPerWindowMs", "collectorCallbacksMs", "mainObserverCallbacksMs",
  "workerCollectorCallbacksMs", "callbackAttributionComplete", "callbackIntervals", "rawLongTasks",
  "underlyingLongTasksMs", "instrumentationAddedLongTasksMs",
]);

function observerModeHardRedExpected(value) {
  const atOrAbove = (values, threshold) => Array.isArray(values)
    && values.some((item) => Number.isFinite(item) && item >= threshold);
  const parserRed = [value?.commonVerdict, value?.internalVerdict]
    .some((verdict) => ["RED", "RED_BEHAVIOR"].includes(verdict));
  const localRed = atOrAbove(value?.sentinelCallbacksMs,
    WEB06_THRESHOLDS.observer.sentinelCallbackExclusiveMaxMs)
    || atOrAbove(value?.sentinelTotalPerEventMs,
      WEB06_THRESHOLDS.observer.sentinelTotalPerEventExclusiveMaxMs)
    || atOrAbove(value?.collectorCallbacksMs,
      WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(value?.mainObserverCallbacksMs,
      WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(value?.workerCollectorCallbacksMs,
      WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(value?.instrumentationAddedLongTasksMs,
      WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs);
  return value?.behaviorRedObserved === true
    || (value?.hardRedBindingValid === true && (parserRed || localRed));
}

function schemaNumberArray(value, errors, label, { allowSigned = false } = {}) {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "number"
    || !Number.isFinite(item) || (!allowSigned && item < 0))) {
    errors.push(`${label}:number-array`);
  }
}

function schemaStringArray(value, errors, label, { nonEmpty = false, unique = false } = {}) {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string" || (nonEmpty && item.length === 0))) {
    errors.push(`${label}:string-array`);
  } else if (unique && new Set(value).size !== value.length) errors.push(`${label}:duplicate`);
}

function schemaObserverCallback(value, errors, label, modeName) {
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
  const expectedPrivateClass = modeName === "minimal" ? "minimal-probe"
    : modeName === "full" ? "full-collector" : undefined;
  if (!sentinel && value.sourceClass !== expectedPrivateClass) errors.push(`${label}:source-class`);
  if (sentinel && (typeof value.kind !== "string" || value.kind.length === 0)) errors.push(`${label}.kind:string`);
  for (const key of ["callbackId", "sourceClass", ...(sentinel ? ["pageInstanceId"] : ["operation"])]) {
    if (typeof value[key] !== "string" || value[key].length === 0) errors.push(`${label}.${key}:string`);
  }
  for (const key of ["actionId", "eventId", "pageInstanceId"]) {
    if (key in value && (typeof value[key] !== "string" || value[key].length === 0)) errors.push(`${label}.${key}:string`);
  }
  if (!Number.isSafeInteger(value.sequenceId)
    || ("eventSequenceId" in value && !Number.isSafeInteger(value.eventSequenceId))
    || (sentinel && (!Number.isSafeInteger(value.windowIndex) || value.windowIndex < -1))) {
    errors.push(`${label}:sequence`);
  }
  if (!Number.isSafeInteger(value.sequenceId) || !Number.isFinite(value.startedAt)
    || !Number.isFinite(value.finishedAt) || !Number.isFinite(value.durationMs)) {
    errors.push(`${label}:timing`);
  }
}

function schemaObserverLongTask(value, errors, label) {
  const keys = ["startTime", "durationMs", "pageInstanceId", "overlapsInteractionWindow", "overlapsIdleControl", "locus"];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (!Number.isFinite(value.startTime) || !Number.isFinite(value.durationMs)
    || typeof value.pageInstanceId !== "string" || typeof value.overlapsInteractionWindow !== "boolean"
    || value.overlapsInteractionWindow !== true
    || typeof value.overlapsIdleControl !== "boolean" || typeof value.locus !== "string") errors.push(`${label}:values`);
}

function observerModeLedgerSemanticallyValid(value, modeName) {
  const intervals = value?.callbackIntervals;
  if (!Array.isArray(intervals)) return false;
  const sentinel = intervals.filter((row) => row?.sourceClass === "common-sentinel");
  const privateCallbacks = intervals.filter((row) => row?.sourceClass !== "common-sentinel");
  const finiteNonnegativeArray = (rows) => Array.isArray(rows)
    && rows.every((item) => Number.isFinite(item) && item >= 0);
  const rowsValid = intervals.every((row) => row && typeof row === "object" && !Array.isArray(row)
    && typeof row.callbackId === "string" && row.callbackId.length > 0
    && Number.isSafeInteger(row.sequenceId) && row.sequenceId > 0
    && Number.isFinite(row.startedAt) && row.startedAt >= 0
    && Number.isFinite(row.finishedAt) && row.finishedAt >= row.startedAt
    && Number.isFinite(row.durationMs) && row.durationMs >= 0
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
  const orderedUnique = (rows) => new Set(rows.map((row) => row.callbackId)).size === rows.length
    && new Set(rows.map((row) => row.sequenceId)).size === rows.length
    && rows.every((row, index) => index === 0
      || (row.sequenceId > rows[index - 1].sequenceId
        && row.finishedAt >= rows[index - 1].finishedAt));
  const sentinelValid = Number.isSafeInteger(value?.callbackLedgerCount)
    && value.callbackLedgerCount === sentinel.length
    && Number.isSafeInteger(value.callbackLedgerCapacity)
    && value.callbackLedgerCount <= value.callbackLedgerCapacity
    && value.sentinelAccountedCallbackCount === sentinel.length
    && value.callbackLedgerOverflowCount === 0
    && Number.isSafeInteger(value.commonEventCount) && value.commonEventCount >= 0
    && (value.commonEventCount === 0 || sentinel.length > 0)
    && finiteNonnegativeArray(value.sentinelCallbacksMs)
    && JSON.stringify(value.sentinelCallbacksMs) === JSON.stringify(sentinel.map((row) => row.durationMs));
  const privateValid = modeName === "product"
    ? privateCallbacks.length === 0
      && Array.isArray(value.mainObserverCallbacksMs) && value.mainObserverCallbacksMs.length === 0
      && Array.isArray(value.workerCollectorCallbacksMs) && value.workerCollectorCallbacksMs.length === 0
    : Number.isSafeInteger(value?.mainObserverCallbackCount)
      && Number.isSafeInteger(value?.mainObserverCallbackCapacity)
      && value.mainObserverCallbackCount === privateCallbacks.length
      && value.mainObserverCallbackCount <= value.mainObserverCallbackCapacity
      && value.mainObserverCallbackOverflowCount === 0
      && finiteNonnegativeArray(value.mainObserverCallbacksMs)
      && JSON.stringify(value.mainObserverCallbacksMs)
        === JSON.stringify(privateCallbacks.map((row) => row.durationMs));
  const rawLongTasksValid = Array.isArray(value?.rawLongTasks)
    && value.rawLongTasks.every((task) => Number.isFinite(task?.startTime) && task.startTime >= 0
      && Number.isFinite(task?.durationMs) && task.durationMs >= 0
      && task.overlapsInteractionWindow === true);
  if (!finiteNonnegativeArray(value?.workerCollectorCallbacksMs)
    || !finiteNonnegativeArray(value?.collectorCallbacksMs)
    || !finiteNonnegativeArray(value?.sentinelTotalPerEventMs)
    || !finiteNonnegativeArray(value?.sentinelTotalPerWindowMs)
    || !Number.isSafeInteger(value?.interactionWindowCount) || value.interactionWindowCount < 0) return false;
  const combinedCollector = [...value.mainObserverCallbacksMs, ...value.workerCollectorCallbacksMs]
    .sort((left, right) => left - right);
  const eventTotals = Array.from({ length: value.commonEventCount }, (_, index) => sentinel
    .filter((row) => row.eventSequenceId === index + 1)
    .reduce((sum, row) => sum + row.durationMs, 0));
  const windowTotals = Array.from({ length: value.interactionWindowCount }, (_, index) => sentinel
    .filter((row) => row.windowIndex === index)
    .reduce((sum, row) => sum + row.durationMs, 0));
  return rowsValid && rawLongTasksValid
    && new Set(intervals.map((row) => row.callbackId)).size === intervals.length
    && orderedUnique(sentinel) && orderedUnique(privateCallbacks)
    && sentinelValid && privateValid
    && JSON.stringify(combinedCollector)
      === JSON.stringify([...value.collectorCallbacksMs].sort((left, right) => left - right))
    && JSON.stringify(value.sentinelTotalPerEventMs) === JSON.stringify(eventTotals)
    && JSON.stringify(value.sentinelTotalPerWindowMs) === JSON.stringify(windowTotals);
}

function schemaObserverMode(value, errors, label, modeName) {
  const complete = value?.measurementCompleted === true;
  const required = [...OBSERVER_MODE_BASE_KEYS];
  if (complete) {
    required.push("commonEquivalenceDigest", "environmentManifestSha256", "environmentId", "callbackLedgerCount",
      "callbackLedgerCapacity",
      "sentinelAccountedCallbackCount", "callbackLedgerOverflowCount");
    if (modeName !== "product") required.push("internalEquivalenceDigest", "mainObserverCallbackCount",
      "mainObserverCallbackCapacity", "mainObserverCallbackOverflowCount");
  }
  if (!schemaExactKeys(value, required, errors, label)) return;
  schemaArtifactReference(value.rawPacket, errors, `${label}.rawPacket`);
  for (const key of ["measurementStarted", "measurementCompleted", "measurementValid",
    "behaviorRedObserved", "hardRedBindingValid", "hardRedObserved", "callbackAttributionComplete"]) {
    if (typeof value[key] !== "boolean") errors.push(`${label}.${key}:boolean`);
  }
  schemaNumberArray(value.samples, errors, `${label}.samples`);
  const allowSignedInvalidEvidence = complete && value.callbackAttributionComplete === false
    && value.measurementValid === false;
  for (const key of ["sentinelCallbacksMs", "sentinelTotalPerEventMs", "sentinelTotalPerWindowMs",
    "collectorCallbacksMs", "mainObserverCallbacksMs", "workerCollectorCallbacksMs"]) {
    schemaNumberArray(value[key], errors, `${label}.${key}`, { allowSigned: allowSignedInvalidEvidence });
  }
  for (const key of ["underlyingLongTasksMs", "instrumentationAddedLongTasksMs"]) {
    schemaNumberArray(value[key], errors, `${label}.${key}`);
  }
  if (!Array.isArray(value.callbackIntervals) || !Array.isArray(value.rawLongTasks)) errors.push(`${label}:ledgers`);
  else {
    value.callbackIntervals.forEach((row, index) =>
      schemaObserverCallback(row, errors, `${label}.callbackIntervals[${index}]`, modeName));
    value.rawLongTasks.forEach((row, index) => schemaObserverLongTask(row, errors, `${label}.rawLongTasks[${index}]`));
  }
  if (complete && Array.isArray(value.rawLongTasks)
    && Array.isArray(value.underlyingLongTasksMs) && Array.isArray(value.instrumentationAddedLongTasksMs)) {
    const thresholdDurations = value.rawLongTasks
      .filter((task) => Number.isFinite(task?.durationMs)
        && task.overlapsInteractionWindow === true
        && task.durationMs >= WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs)
      .map((task) => task.durationMs).sort((left, right) => left - right);
    const classifiedDurations = [...value.underlyingLongTasksMs, ...value.instrumentationAddedLongTasksMs]
      .sort((left, right) => left - right);
    if (JSON.stringify(thresholdDurations) !== JSON.stringify(classifiedDurations)) {
      errors.push(`${label}:long-task-attribution-conservation`);
    }
  }
  for (const key of ["commonEquivalenceDigest", "internalEquivalenceDigest", "environmentManifestSha256"]) {
    if (key in value && !isSha256(value[key])) errors.push(`${label}.${key}:hash`);
  }
  if ("environmentId" in value && !isSha256(value.environmentId)) {
    errors.push(`${label}.environmentId:hash`);
  }
  for (const key of ["commonEventCount", "interactionWindowCount", "callbackLedgerCount",
    "callbackLedgerCapacity", "sentinelAccountedCallbackCount", "callbackLedgerOverflowCount", "mainObserverCallbackCount",
    "mainObserverCallbackCapacity", "mainObserverCallbackOverflowCount"]) {
    if (key in value && (!Number.isSafeInteger(value[key]) || value[key] < 0)) errors.push(`${label}.${key}:count`);
  }
  for (const key of ["commonVerdict", "internalVerdict"]) {
    if (key in value && !["PASS", "RED", "RED_BEHAVIOR", "SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"]
      .includes(value[key])) errors.push(`${label}.${key}:verdict`);
  }
  if (value.measurementCompleted && !value.measurementStarted) errors.push(`${label}:completed-without-start`);
  if (!value.measurementCompleted && value.measurementValid) errors.push(`${label}:partial-marked-valid`);
  const hardRedExpected = observerModeHardRedExpected(value);
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
    !== observerModeLedgerSemanticallyValid(value, modeName)) {
    errors.push(`${label}:callback-attribution-semantic-link`);
  }
  if (complete && value.rawLongTasks?.some((task) =>
    Number.isFinite(task?.startTime) && Number.isFinite(task?.durationMs)
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
  const sentinelCallbacks = value.callbackIntervals?.filter((row) => row.sourceClass === "common-sentinel") ?? [];
  const privateCallbacks = value.callbackIntervals?.filter((row) => row.sourceClass !== "common-sentinel") ?? [];
  if (complete && value.callbackAttributionComplete === true
    && (value.callbackLedgerCount !== sentinelCallbacks.length
      || value.sentinelAccountedCallbackCount !== sentinelCallbacks.length)) {
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
    && (value.sentinelTotalPerEventMs.length !== value.commonEventCount
      || value.sentinelTotalPerWindowMs.length !== value.interactionWindowCount)) {
    errors.push(`${label}:sentinel-cardinality`);
  }
  if (complete && value.callbackAttributionComplete === true
    && JSON.stringify(value.mainObserverCallbacksMs)
    !== JSON.stringify(privateCallbacks.map((row) => row.durationMs))) errors.push(`${label}:private-duration-ledger`);
  const combinedCollectorDurations = [...value.mainObserverCallbacksMs, ...value.workerCollectorCallbacksMs]
    .sort((left, right) => left - right);
  if (complete && value.callbackAttributionComplete === true
    && JSON.stringify(combinedCollectorDurations)
    !== JSON.stringify([...value.collectorCallbacksMs].sort((left, right) => left - right))) {
    errors.push(`${label}:collector-duration-ledger`);
  }
}

function schemaObserverTriplet(value, errors, label) {
  const keys = ["attemptId", "valid", "counterbalanceSlot", "freshContextId", "modeContextIds", "modeOrder",
    "modeFixedBeforePageLoad", "product", "minimal", "full"];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (typeof value.attemptId !== "string" || typeof value.valid !== "boolean"
    || !Number.isSafeInteger(value.counterbalanceSlot)
    || !hasOwn(WEB06_OBSERVER_COUNTERBALANCE, String(value.counterbalanceSlot))
    || typeof value.freshContextId !== "string"
    || !Array.isArray(value.modeContextIds) || value.modeContextIds.some((id) => typeof id !== "string")
    || value.modeContextIds.length !== 3 || !Array.isArray(value.modeOrder) || value.modeOrder.length !== 3
    || value.modeOrder.some((id) => typeof id !== "string")
    || value.modeFixedBeforePageLoad !== true) errors.push(`${label}:values`);
  if (!/^triplet-attempt-[1-7]$/.test(value.attemptId ?? "")
    || value.modeContextIds?.some((id) => id.length === 0)
    || new Set(value.modeContextIds ?? []).size !== 3 || value.freshContextId !== value.modeContextIds?.join("+")) {
    errors.push(`${label}:context-identity`);
  }
  if (hasOwn(WEB06_OBSERVER_COUNTERBALANCE, String(value.counterbalanceSlot))
    && JSON.stringify(value.modeOrder) !== JSON.stringify(WEB06_OBSERVER_COUNTERBALANCE[value.counterbalanceSlot])) {
    errors.push(`${label}:counterbalance-order`);
  }
  for (const modeName of ["product", "minimal", "full"]) {
    schemaObserverMode(value[modeName], errors, `${label}.${modeName}`, modeName);
  }
  if (value.valid !== [value.product, value.minimal, value.full]
    .every((mode) => mode?.measurementValid === true)) errors.push(`${label}:validity`);
}

function schemaObserverEvaluation(value, errors, label) {
  const complete = Array.isArray(value?.comparisons);
  const keys = complete ? ["pass", "status", "comparisons", "violations"]
    : ["pass", "status", "violations", "preservedUnpairedReds"];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (typeof value.pass !== "boolean" || typeof value.status !== "string" || !Array.isArray(value.violations)
    || value.violations.some((row) => typeof row !== "string")
    || new Set(value.violations ?? []).size !== value.violations?.length) errors.push(`${label}:values`);
  if (complete && (!["PASS", "RED"].includes(value.status) || value.pass !== (value.status === "PASS")
    || value.comparisons.length !== 2)) errors.push(`${label}:status`);
  if (!complete && (!["INCOMPLETE", "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS"].includes(value.status)
    || value.pass !== false)) errors.push(`${label}:status`);
  if (!complete) schemaStringArray(value.preservedUnpairedReds, errors, `${label}.preservedUnpairedReds`, { unique: true });
  if (complete) value.comparisons.forEach((row, index) => {
    if (!schemaExactKeys(row, ["pair", "medianDelta", "p95Delta", "maxDelta"], errors,
      `${label}.comparisons[${index}]`)) return;
    if (row.pair !== ["product-vs-minimal", "minimal-vs-full"][index]
      || [row.medianDelta, row.p95Delta, row.maxDelta]
      .some((item) => !Number.isFinite(item) || item < 0)) errors.push(`${label}.comparisons[${index}]:values`);
  });
}

function schemaExecution(value, errors, label) {
  if (!schemaExactKeys(value, ["plannedScenarioCount", "executedScenarioCount", "skippedScenarioCount",
    "unexpectedScenarioCount", "status"], errors, label)) return;
  if ([value.plannedScenarioCount, value.executedScenarioCount, value.skippedScenarioCount, value.unexpectedScenarioCount]
    .some((item) => !Number.isSafeInteger(item) || item < 0) || !["completed", "incomplete"].includes(value.status)) {
    errors.push(`${label}:values`);
  }
}

function schemaExpectationIdentity(value, errors, label) {
  const expectation = value.expectation;
  const branch = value.selectedBranch;
  const disposition = value.disposition;
  if (!["BASELINE", "FINAL", "PREVIEW", "OBSERVER"].includes(expectation)
    || !["NONE", "A", "B", "C"].includes(branch)
    || !["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE", "PRODUCTION_REDUCTION", "MEASURED_NO_GO"].includes(disposition)) {
    errors.push(`${label}:expectation-identity`);
    return;
  }
  const expected = expectation === "OBSERVER" ? ["NONE", "DIAGNOSTIC"]
    : expectation === "BASELINE" ? ["NONE", "SOURCE_CURRENT_BASELINE"]
      : expectation === "PREVIEW" ? [branch, "PRODUCTION_REDUCTION"]
        : [branch, branch === "NONE" ? "MEASURED_NO_GO" : "PRODUCTION_REDUCTION"];
  if (expectation === "PREVIEW" && !["A", "B", "C"].includes(branch)) errors.push(`${label}:preview-branch`);
  if (branch !== expected[0] || disposition !== expected[1]) errors.push(`${label}:branch-disposition`);
}

function expectedScenarioRuns(expectation, branch) {
  if (expectation === "PREVIEW") return [...WEB06_PREVIEW_SCENARIOS];
  if (expectation === "OBSERVER") return [WEB06_PHASE0_OVERHEAD_SCENARIO];
  if (expectation === "FINAL" && branch === "B") {
    return [...WEB06_BINDING_SCENARIO_ORDER, ...WEB06_EXTENDED_SCENARIO_RUNS];
  }
  return [...WEB06_BINDING_SCENARIO_ORDER];
}

function schemaScenarioRuns(value, expectation, branch, errors, label) {
  schemaStringArray(value, errors, label, { nonEmpty: true, unique: true });
  if (Array.isArray(value)
    && JSON.stringify(value) !== JSON.stringify(expectedScenarioRuns(expectation, branch))) {
    errors.push(`${label}:order`);
  }
}

function expectedTargetId(expectation) {
  return expectation === "BASELINE" ? "BASE_FULL"
    : expectation === "FINAL" ? "FINAL_FULL"
      : expectation === "PREVIEW" ? "FINAL_MINIMAL" : undefined;
}

function schemaVersions(value, errors, label) {
  if (!schemaExactKeys(value, ["metric", "scenarioRegistry", "behaviorPredicate"], errors, label)
    || value.metric !== WEB06_METRIC_CONTRACT_VERSION || value.scenarioRegistry !== WEB06_SCENARIO_REGISTRY_VERSION
    || value.behaviorPredicate !== WEB06_BEHAVIOR_PREDICATE_VERSION) errors.push(`${label}:values`);
}

function schemaNestedEvidenceIdentityLinks(payload, errors, label) {
  const linkSummary = (summary, summaryLabel) => {
    if (!summary || typeof summary !== "object" || Array.isArray(summary)) return;
    if (summary.collectorContractSha256 !== undefined
      && summary.collectorContractSha256 !== payload.collectorContractSha256) {
      errors.push(`${summaryLabel}:collector-contract-link`);
    }
    if (summary.environmentManifestSha256 !== undefined
      && summary.environmentManifestSha256 !== payload.environmentManifestSha256) {
      errors.push(`${summaryLabel}:environment-manifest-link`);
    }
    if (summary.environmentId !== undefined && summary.environmentId !== payload.environmentId) {
      errors.push(`${summaryLabel}:environment-id-link`);
    }
    summary.roundSummaries?.forEach((round, index) =>
      linkSummary(round, `${summaryLabel}.roundSummaries[${index}]`));
  };
  payload.scenarioResults?.forEach((scenario, scenarioIndex) => {
    scenario.attempts?.forEach((attempt, attemptIndex) => {
      for (const [surface, summary] of Object.entries(attempt.runnerSummaries ?? {})) {
        linkSummary(summary,
          `${label}.scenarioResults[${scenarioIndex}].attempts[${attemptIndex}].runnerSummaries.${surface}`);
      }
    });
    for (const [surface, summary] of Object.entries(scenario.runnerFiveRoundSummaries ?? {})) {
      linkSummary(summary, `${label}.scenarioResults[${scenarioIndex}].runnerFiveRoundSummaries.${surface}`);
    }
    for (const [surface, summary] of Object.entries(scenario.fiveRoundSummaries ?? {})) {
      linkSummary(summary, `${label}.scenarioResults[${scenarioIndex}].fiveRoundSummaries.${surface}`);
    }
  });
  payload.observerTriplets?.forEach((triplet, tripletIndex) => {
    for (const modeName of ["product", "minimal", "full"]) {
      const mode = triplet?.[modeName];
      if (mode?.environmentManifestSha256 !== undefined
        && mode.environmentManifestSha256 !== payload.environmentManifestSha256) {
        errors.push(`${label}.observerTriplets[${tripletIndex}].${modeName}:environment-manifest-link`);
      }
      if (mode?.environmentId !== undefined && mode.environmentId !== payload.environmentId) {
        errors.push(`${label}.observerTriplets[${tripletIndex}].${modeName}:environment-id-link`);
      }
    }
  });
}

function validateCollectorOutputSchema(payload, errors) {
  const keys = ["version", "writeMode", "expectation", "disposition", "selectedBranch", "versions",
    "identityManifestSha256", "runnerSourceManifest", "runnerSourceManifestSha256", "runnerSourceObservationSha256",
    "runnerSourcePostObservationSha256", "collectorContractSha256", "environmentManifestSha256", "environmentId",
    "scenarioRuns", "execution", "measurementStarted", "measurementCompleted", "scenarioResults", "observerTriplets",
    ...(payload.expectation === "OBSERVER" ? ["observerEvaluation"] : [])];
  if (!schemaExactKeys(payload, keys, errors, "collector")) return;
  if (payload.version !== "web06-collector-output-v1" || payload.writeMode !== "create-new"
    || !["BASELINE", "FINAL", "PREVIEW", "OBSERVER"].includes(payload.expectation)) errors.push("collector:identity");
  schemaExpectationIdentity(payload, errors, "collector");
  schemaVersions(payload.versions, errors, "collector.versions");
  schemaRunnerSource(payload.runnerSourceManifest, errors, "collector.runnerSourceManifest");
  schemaExecution(payload.execution, errors, "collector.execution");
  for (const key of ["identityManifestSha256", "runnerSourceManifestSha256", "runnerSourceObservationSha256",
    "runnerSourcePostObservationSha256", "collectorContractSha256", "environmentManifestSha256"]) {
    if (!isSha256(payload[key])) errors.push(`collector.${key}:hash`);
  }
  schemaScenarioRuns(payload.scenarioRuns, payload.expectation, payload.selectedBranch, errors, "collector.scenarioRuns");
  if (typeof payload.environmentId !== "string" || payload.environmentId.length === 0
    || typeof payload.measurementStarted !== "boolean" || typeof payload.measurementCompleted !== "boolean"
    || !Array.isArray(payload.scenarioResults) || !Array.isArray(payload.observerTriplets)) errors.push("collector:arrays");
  if (payload.collectorContractSha256 !== WEB06_COLLECTOR_CONTRACT_SHA256) {
    errors.push("collector:collector-contract-link");
  }
  payload.scenarioResults?.forEach((row, index) => {
    schemaRunnerScenario(row, errors, `collector.scenarioResults[${index}]`, payload.expectation);
    if (row.targetId !== expectedTargetId(payload.expectation)
      || row.scenarioRunId !== payload.scenarioRuns?.[index]) errors.push(`collector.scenarioResults[${index}]:identity`);
  });
  payload.observerTriplets?.forEach((row, index) => schemaObserverTriplet(row, errors, `collector.observerTriplets[${index}]`));
  schemaNestedEvidenceIdentityLinks(payload, errors, "collector");
  if (payload.expectation === "OBSERVER") schemaObserverEvaluation(payload.observerEvaluation, errors, "collector.observerEvaluation");
  const observer = payload.expectation === "OBSERVER";
  if ((observer && (payload.scenarioResults?.length !== 0 || payload.observerTriplets?.length > 7))
    || (!observer && (payload.observerTriplets?.length !== 0
      || payload.scenarioResults?.length !== payload.scenarioRuns?.length))) errors.push("collector:lane-shape");
  if (payload.runnerSourceObservationSha256 !== payload.runnerSourcePostObservationSha256
    || payload.runnerSourceManifestSha256 !== digestJson(payload.runnerSourceManifest)) {
    errors.push("collector:source-reconciliation");
  }
  if (payload.measurementCompleted !== (payload.execution?.status === "completed")) {
    errors.push("collector:completion-reconciliation");
  }
}

function schemaIndependentDiagnosticBinding(value, errors, label, scope) {
  const fields = scope === "round"
    ? ["setupErrorCodes", "behaviorErrorCodes", "thresholdViolations"]
    : ["summaryErrors", "poolViolations"];
  const keys = ["version", "dimensions", ...(scope === "five-round" ? ["roundBindingsSha256"] : [])];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (value.version !== WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION
    || (scope === "five-round" && !isSha256(value.roundBindingsSha256))) errors.push(`${label}:identity`);
  if (!schemaExactKeys(value.dimensions, fields, errors, `${label}.dimensions`)) return;
  for (const field of fields) {
    const dimension = value.dimensions[field];
    if (!schemaExactKeys(dimension,
      ["rawCount", "rawSha256", "semanticCount", "semanticSha256"], errors, `${label}.dimensions.${field}`)) {
      continue;
    }
    if (!Number.isSafeInteger(dimension.rawCount) || dimension.rawCount < 0
      || !Number.isSafeInteger(dimension.semanticCount) || dimension.semanticCount < 0
      || dimension.semanticCount > dimension.rawCount
      || !isSha256(dimension.rawSha256) || !isSha256(dimension.semanticSha256)) {
      errors.push(`${label}.dimensions.${field}:values`);
    }
  }
}

function schemaIndependentDiagnosticBindingPair(value, errors, label, scope) {
  const keys = ["version", "scope", "runnerBinding", "independentBinding",
    "runnerSemanticProjectionSha256", "independentSemanticProjectionSha256",
    "runnerDecisionDimensionsSha256", "independentDecisionDimensionsSha256", "equivalent", "bindingPairSha256"];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (value.version !== "web06-independent-diagnostic-binding-pair-v1"
    || value.scope !== scope || value.equivalent !== true
    || !isSha256(value.runnerSemanticProjectionSha256)
    || value.runnerSemanticProjectionSha256 !== value.independentSemanticProjectionSha256
    || !isSha256(value.runnerDecisionDimensionsSha256)
    || value.runnerDecisionDimensionsSha256 !== value.independentDecisionDimensionsSha256
    || !isSha256(value.bindingPairSha256)) {
    errors.push(`${label}:equivalence`);
  }
  const { bindingPairSha256, ...pair } = value;
  if (bindingPairSha256 !== digestJson(stableJsonValue(pair))) errors.push(`${label}:binding-pair-hash`);
  schemaIndependentDiagnosticBinding(value.runnerBinding, errors, `${label}.runnerBinding`, scope);
  schemaIndependentDiagnosticBinding(value.independentBinding, errors, `${label}.independentBinding`, scope);
}

function schemaIndependentAttempt(value, errors, label) {
  const complete = value?.measurementCompleted === true;
  const keys = ["attemptId", "rawPacketSha256", "measurementStarted", "measurementCompleted", "classification",
    "retainedMeasured", "retainedLogicalRound", "validForLatencyFrame", "retainedHardRed", "retryEligible",
    "validRedObserved",
    ...(complete ? ["diagnosticBindingPairs", "recomputedSummarySha256"] : ["failureCode"])];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (!isSha256(value.rawPacketSha256) || (complete && !isSha256(value.recomputedSummarySha256))) errors.push(`${label}:hash`);
  if (typeof value.attemptId !== "string" || typeof value.classification !== "string"
    || !["PASS", "RED", "SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(value.classification)
    || ["measurementStarted", "measurementCompleted", "retainedMeasured", "retainedLogicalRound",
      "validForLatencyFrame",
      "retainedHardRed", "retryEligible", "validRedObserved"].some((key) => typeof value[key] !== "boolean")
    || (!complete && typeof value.failureCode !== "string")) errors.push(`${label}:values`);
  if (value.measurementCompleted && !value.measurementStarted
    || value.retainedMeasured !== value.retainedLogicalRound
    || (value.validForLatencyFrame && !value.retainedLogicalRound)
    || value.validRedObserved !== value.retainedHardRed
    || value.retryEligible !== ["SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(value.classification)
    || (value.retainedHardRed && value.classification !== "RED")) errors.push(`${label}:classification-facts`);
  if (complete) {
    schemaSurfaceMap(value.diagnosticBindingPairs, errors, `${label}.diagnosticBindingPairs`,
      (pair, pairErrors, pairLabel) => schemaIndependentDiagnosticBindingPair(pair, pairErrors, pairLabel, "round"));
    if (Object.keys(value.diagnosticBindingPairs ?? {}).length === 0) errors.push(`${label}:diagnostic-bindings-empty`);
  }
}

function schemaIndependentScenario(value, errors, label, expectation) {
  const keys = ["targetId", "scenarioRunId", "scenarioId", "schemaId", "measuredRoundCount",
    "validLatencyFrameRoundCount", "verdict",
    "preservedHardRedAttemptIds", "preservedHardRedObserved", "attemptResults", "fiveRoundSummaries",
    "fiveRoundDiagnosticBindingPairs", "fiveRoundSummarySha256"];
  if (!schemaExactKeys(value, keys, errors, label)) return;
  if (typeof value.targetId !== "string" || typeof value.scenarioRunId !== "string"
    || typeof value.scenarioId !== "string" || typeof value.schemaId !== "string"
    || !Number.isSafeInteger(value.measuredRoundCount) || value.measuredRoundCount < 0
    || !Number.isSafeInteger(value.validLatencyFrameRoundCount) || value.validLatencyFrameRoundCount < 0
    || !["PASS", "RED", "SETUP_NO_GO", "SETUP_INVALID"].includes(value.verdict)
    || typeof value.preservedHardRedObserved !== "boolean"
    || !Array.isArray(value.preservedHardRedAttemptIds)
    || !Array.isArray(value.attemptResults) || !isSha256(value.fiveRoundSummarySha256)) errors.push(`${label}:values`);
  const run = hasOwn(SCENARIO_RUN_REGISTRY, value.scenarioRunId)
    ? SCENARIO_RUN_REGISTRY[value.scenarioRunId]
    : undefined;
  if (run?.scenarioId !== value.scenarioId || run?.schema !== value.schemaId) errors.push(`${label}:run-identity`);
  value.attemptResults?.forEach((attempt, index) => {
    schemaIndependentAttempt(attempt, errors, `${label}.attemptResults[${index}]`);
    if (attempt.attemptId !== `attempt-${index + 1}`) errors.push(`${label}.attemptResults[${index}]:attempt-id`);
  });
  const measuredCount = value.attemptResults?.filter((attempt) => attempt.retainedMeasured === true).length;
  const validLatencyFrameCount = value.attemptResults
    ?.filter((attempt) => attempt.validForLatencyFrame === true).length;
  const hardRedIds = value.attemptResults?.filter((attempt) => attempt.retainedHardRed === true)
    .map((attempt) => attempt.attemptId) ?? [];
  if (value.measuredRoundCount !== measuredCount
    || value.validLatencyFrameRoundCount !== validLatencyFrameCount
    || value.validLatencyFrameRoundCount > value.measuredRoundCount
    || JSON.stringify(value.preservedHardRedAttemptIds) !== JSON.stringify(hardRedIds)
    || value.preservedHardRedObserved !== (hardRedIds.length > 0)
    || value.fiveRoundSummarySha256 !== digestJson(value.fiveRoundSummaries)) {
    errors.push(`${label}:reconciliation`);
  }
  schemaSurfaceMap(value.fiveRoundSummaries, errors, `${label}.fiveRoundSummaries`, schemaFiveRoundSummary);
  schemaSurfaceMap(value.fiveRoundDiagnosticBindingPairs, errors, `${label}.fiveRoundDiagnosticBindingPairs`,
    (pair, pairErrors, pairLabel) =>
      schemaIndependentDiagnosticBindingPair(pair, pairErrors, pairLabel, "five-round"));
  if (JSON.stringify(Object.keys(value.fiveRoundDiagnosticBindingPairs ?? {}).sort())
    !== JSON.stringify(Object.keys(value.fiveRoundSummaries ?? {}).sort())) {
    errors.push(`${label}:five-round-binding-surfaces`);
  }
  if (expectation === "PREVIEW" && Object.keys(value.fiveRoundSummaries ?? {}).length !== 0) {
    errors.push(`${label}:preview-five-round-summary`);
  } else if (expectation !== "PREVIEW") {
    const keys = Object.keys(value.fiveRoundSummaries ?? {}).sort();
    const expectedKeys = value.validLatencyFrameRoundCount === WEB06_THRESHOLDS.attempts.requiredValid
      ? ["common", "internal"] : [];
    if (JSON.stringify(keys) !== JSON.stringify(expectedKeys)) errors.push(`${label}:five-round-surfaces`);
  }
  const required = expectation === "PREVIEW" ? 1 : WEB06_THRESHOLDS.attempts.requiredValid;
  const expectedVerdict = value.measuredRoundCount === required
    ? (value.attemptResults?.some((attempt) => attempt.classification === "RED") ? "RED" : "PASS")
    : expectation === "PREVIEW" ? "SETUP_INVALID" : "SETUP_NO_GO";
  if (value.verdict !== expectedVerdict) errors.push(`${label}:verdict-reconciliation`);
}

function expectedSuiteTargetRoles(expectation) {
  if (!hasOwn(WEB06_EXPECTATION_TARGET_ORDER, expectation)) return [];
  return expectation === "OBSERVER" ? ["PRODUCT", "BASE"]
    : expectation === "BASELINE" ? ["BASE"] : ["FINAL"];
}

function schemaSuiteSourceArtifactRoles(value, errors, label, expectation) {
  if (!schemaExactKeys(value, ["version", "runnerSource", "targetRoles", "bindingsSha256"], errors, label)) return;
  if (value.version !== "web06-suite-source-artifact-roles-v1") errors.push(`${label}:version`);
  const { bindingsSha256, ...bindings } = value;
  if (!isSha256(bindingsSha256)
    || bindingsSha256 !== digestJson(stableJsonValue(bindings))) errors.push(`${label}:bindings-hash`);
  const runnerKeys = ["role", "identityRole", "sourceCommit", "sourceTree", "sourceTreeState",
    "sourceManifestSha256", "toolingManifestSha256", "beforeObservationSha256", "afterObservationSha256"];
  if (schemaExactKeys(value.runnerSource, runnerKeys, errors, `${label}.runnerSource`)) {
    const expectedIdentityRole = ["FINAL", "PREVIEW"].includes(expectation) ? "FINAL" : "BASE";
    if (value.runnerSource.role !== "RUNNER_SOURCE" || value.runnerSource.identityRole !== expectedIdentityRole
      || !COMMIT_RE.test(value.runnerSource.sourceCommit ?? "") || !COMMIT_RE.test(value.runnerSource.sourceTree ?? "")
      || value.runnerSource.sourceTreeState !== "clean"
      || ["sourceManifestSha256", "toolingManifestSha256", "beforeObservationSha256", "afterObservationSha256"]
        .some((key) => !isSha256(value.runnerSource[key]))
      || value.runnerSource.beforeObservationSha256 !== value.runnerSource.afterObservationSha256) {
      errors.push(`${label}.runnerSource:identity`);
    }
  }
  const expectedRoles = expectedSuiteTargetRoles(expectation);
  if (!schemaExactKeys(value.targetRoles, expectedRoles, errors, `${label}.targetRoles`)) return;
  for (const roleName of expectedRoles) {
    const role = value.targetRoles[roleName];
    const roleKeys = ["role", "sourceCommit", "sourceTree", "sourceTreeState", "archiveSha256",
      "artifactManifestSha256", "buildInfoSha256", "selectedBranch", "disposition", "targets"];
    if (!schemaExactKeys(role, roleKeys, errors, `${label}.targetRoles.${roleName}`)) continue;
    const expectationTargets = hasOwn(WEB06_EXPECTATION_TARGET_ORDER, expectation)
      ? WEB06_EXPECTATION_TARGET_ORDER[expectation]
      : [];
    const expectedTargets = expectationTargets
      .filter((targetId) => identityRoleForTarget(targetId) === roleName);
    if (role.role !== roleName || !COMMIT_RE.test(role.sourceCommit ?? "") || !COMMIT_RE.test(role.sourceTree ?? "")
      || role.sourceTreeState !== "clean"
      || ["archiveSha256", "artifactManifestSha256", "buildInfoSha256"].some((key) => !isSha256(role[key]))
      || !["NONE", "A", "B", "C"].includes(role.selectedBranch)
      || !["DIAGNOSTIC", "SOURCE_CURRENT_BASELINE", "PRODUCTION_REDUCTION", "MEASURED_NO_GO"]
        .includes(role.disposition)
      || !schemaExactKeys(role.targets, expectedTargets, errors, `${label}.targetRoles.${roleName}.targets`)) {
      errors.push(`${label}.targetRoles.${roleName}:identity`);
      continue;
    }
    for (const targetId of expectedTargets) {
      const target = role.targets[targetId];
      if (!schemaExactKeys(target,
        ["protocolMode", "selectorPolicy", "artifactResponseGuardSha256"], errors,
        `${label}.targetRoles.${roleName}.targets.${targetId}`)) continue;
      if (target.protocolMode !== WEB06_TARGET_PROTOCOL_MODES[targetId]
        || target.selectorPolicy !== (targetId === "PRODUCT" || expectation === "PREVIEW" ? "omitted" : "explicit")
        || !isSha256(target.artifactResponseGuardSha256)) {
        errors.push(`${label}.targetRoles.${roleName}.targets.${targetId}:identity`);
      }
    }
  }
}

function identityManifestFromSuiteSourceArtifactRoles(value, expectation) {
  const roles = Object.fromEntries(expectedSuiteTargetRoles(expectation).map((roleName) => {
    const source = value?.targetRoles?.[roleName] ?? {};
    return [roleName, {
      sourceCommit: source.sourceCommit,
      sourceTree: source.sourceTree,
      sourceTreeState: source.sourceTreeState,
      archiveSha256: source.archiveSha256,
      artifactManifestSha256: source.artifactManifestSha256,
      buildInfoSha256: source.buildInfoSha256,
      selectedBranch: source.selectedBranch,
      disposition: source.disposition,
    }];
  }));
  return {
    version: "web06-target-identities-v1",
    metricContractVersion: WEB06_METRIC_CONTRACT_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_REGISTRY_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_PREDICATE_VERSION,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    roles,
  };
}

function validateIndependentSchema(payload, errors) {
  const keys = ["version", "writeMode", "collectorOutputSha256", "expectation", "disposition", "selectedBranch",
    "identityManifestSha256", "collectorContractSha256", "environmentManifestSha256", "environmentId",
    "scenarioResults", "observerTriplets", ...(payload.expectation === "OBSERVER" ? ["observerEvaluation"] : []),
    "verificationStatus"];
  if (!schemaExactKeys(payload, keys, errors, "independent")) return;
  if (payload.version !== "web06-independent-recompute-v1" || payload.writeMode !== "create-new"
    || payload.verificationStatus !== "PASS" || !isSha256(payload.collectorOutputSha256)) errors.push("independent:identity");
  schemaExpectationIdentity(payload, errors, "independent");
  for (const key of ["identityManifestSha256", "collectorContractSha256", "environmentManifestSha256"]) {
    if (!isSha256(payload[key])) errors.push(`independent.${key}:hash`);
  }
  if (typeof payload.environmentId !== "string" || payload.environmentId.length === 0
    || !Array.isArray(payload.scenarioResults) || !Array.isArray(payload.observerTriplets)) {
    errors.push("independent:arrays");
  }
  if (payload.collectorContractSha256 !== WEB06_COLLECTOR_CONTRACT_SHA256) {
    errors.push("independent:collector-contract-link");
  }
  const runs = expectedScenarioRuns(payload.expectation, payload.selectedBranch);
  payload.scenarioResults?.forEach((row, index) => {
    schemaIndependentScenario(row, errors, `independent.scenarioResults[${index}]`, payload.expectation);
    if (row.targetId !== expectedTargetId(payload.expectation) || row.scenarioRunId !== runs[index]) {
      errors.push(`independent.scenarioResults[${index}]:identity`);
    }
  });
  payload.observerTriplets?.forEach((row, index) => {
    schemaObserverTriplet(row, errors, `independent.observerTriplets[${index}]`);
    if (row.attemptId !== `triplet-attempt-${index + 1}`) errors.push(`independent.observerTriplets[${index}]:attempt-id`);
  });
  schemaNestedEvidenceIdentityLinks(payload, errors, "independent");
  if (payload.expectation === "OBSERVER") schemaObserverEvaluation(payload.observerEvaluation, errors, "independent.observerEvaluation");
  if ((payload.expectation === "OBSERVER" && (payload.scenarioResults?.length !== 0 || payload.observerTriplets?.length > 7))
    || (payload.expectation !== "OBSERVER" && (payload.observerTriplets?.length !== 0
      || payload.scenarioResults?.length !== runs.length))) errors.push("independent:lane-shape");
}

function validateAttestationSchema(payload, errors) {
  const keys = ["version", "writeMode", "expectation", "disposition", "selectedBranch", "identityManifestSha256",
    "runnerSourceManifestSha256", "runnerSourceObservationSha256", "runnerSourcePostObservationSha256",
    "collectorContractSha256", "environmentManifestSha256", "environmentId", "versions", "scenarios", "execution",
    "collectorOutput", "independentRecompute", "measurementStarted", "measurementCompleted", "verdict",
    "sourceArtifactRoles", "scenarioResults", "observerTriplets", "privacy"];
  if (!schemaExactKeys(payload, keys, errors, "attestation")) return;
  if (payload.version !== "web06-suite-attestation-v1" || payload.writeMode !== "create-new") errors.push("attestation:identity");
  schemaExpectationIdentity(payload, errors, "attestation");
  schemaVersions(payload.versions, errors, "attestation.versions");
  schemaExecution(payload.execution, errors, "attestation.execution");
  schemaSuiteSourceArtifactRoles(payload.sourceArtifactRoles, errors, "attestation.sourceArtifactRoles",
    payload.expectation);
  if (digestJson(stableJsonValue(identityManifestFromSuiteSourceArtifactRoles(
    payload.sourceArtifactRoles,
    payload.expectation,
  ))) !== payload.identityManifestSha256) {
    errors.push("attestation:identity-manifest-role-link");
  }
  schemaScenarioRuns(payload.scenarios, payload.expectation, payload.selectedBranch, errors, "attestation.scenarios");
  schemaArtifactReference(payload.collectorOutput, errors, "attestation.collectorOutput", "collector-output.json");
  schemaArtifactReference(payload.independentRecompute, errors, "attestation.independentRecompute", "independent-recompute.json");
  for (const key of ["identityManifestSha256", "runnerSourceManifestSha256", "runnerSourceObservationSha256",
    "runnerSourcePostObservationSha256", "collectorContractSha256", "environmentManifestSha256"]) {
    if (!isSha256(payload[key])) errors.push(`attestation.${key}:hash`);
  }
  if (payload.runnerSourceObservationSha256 !== payload.runnerSourcePostObservationSha256
    || typeof payload.environmentId !== "string" || payload.environmentId.length === 0
    || typeof payload.measurementStarted !== "boolean" || typeof payload.measurementCompleted !== "boolean"
    || !["PASS", "RED", "SETUP_NO_GO", "SETUP_INVALID", "INCOMPLETE",
      "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS"].includes(payload.verdict)
    || !Array.isArray(payload.scenarioResults) || !Array.isArray(payload.observerTriplets)) {
    errors.push("attestation:values");
  }
  if (payload.collectorContractSha256 !== WEB06_COLLECTOR_CONTRACT_SHA256) {
    errors.push("attestation:collector-contract-link");
  }
  const runnerRole = payload.sourceArtifactRoles?.runnerSource;
  const targetRoles = payload.sourceArtifactRoles?.targetRoles;
  const runnerIdentityTargetRole = targetRoles && typeof targetRoles === "object"
    && hasOwn(targetRoles, runnerRole?.identityRole)
    ? targetRoles[runnerRole.identityRole]
    : undefined;
  if (runnerRole?.sourceManifestSha256 !== payload.runnerSourceManifestSha256
    || runnerRole?.beforeObservationSha256 !== payload.runnerSourceObservationSha256
    || runnerRole?.afterObservationSha256 !== payload.runnerSourcePostObservationSha256
    || runnerIdentityTargetRole?.sourceCommit !== runnerRole?.sourceCommit
    || runnerIdentityTargetRole?.sourceTree !== runnerRole?.sourceTree) {
    errors.push("attestation:runner-role-link");
  }
  for (const roleName of expectedSuiteTargetRoles(payload.expectation)) {
    const role = payload.sourceArtifactRoles?.targetRoles?.[roleName];
    const expectedBranch = roleName === "FINAL" ? payload.selectedBranch : "NONE";
    const expectedRoleDisposition = roleName === "FINAL" ? payload.disposition
      : roleName === "BASE" ? (payload.expectation === "OBSERVER" ? "DIAGNOSTIC" : "SOURCE_CURRENT_BASELINE")
        : "DIAGNOSTIC";
    if (role?.selectedBranch !== expectedBranch || role?.disposition !== expectedRoleDisposition) {
      errors.push(`attestation.sourceArtifactRoles.targetRoles.${roleName}:disposition-link`);
    }
  }
  payload.scenarioResults?.forEach((row, index) => {
    schemaRunnerScenario(row, errors, `attestation.scenarioResults[${index}]`, payload.expectation);
    if (row.targetId !== expectedTargetId(payload.expectation)
      || row.scenarioRunId !== payload.scenarios?.[index]) errors.push(`attestation.scenarioResults[${index}]:identity`);
  });
  payload.observerTriplets?.forEach((row, index) => {
    schemaObserverTriplet(row, errors, `attestation.observerTriplets[${index}]`);
    if (row.attemptId !== `triplet-attempt-${index + 1}`) errors.push(`attestation.observerTriplets[${index}]:attempt-id`);
  });
  schemaNestedEvidenceIdentityLinks(payload, errors, "attestation");
  if (payload.measurementCompleted !== (payload.execution?.status === "completed")) {
    errors.push("attestation:completion-reconciliation");
  }
  const observer = payload.expectation === "OBSERVER";
  const observerComplete = observer && (payload.observerTriplets?.filter((triplet) => triplet.valid === true).length
    === WEB06_THRESHOLDS.observer.requiredTriplets
    || payload.observerTriplets?.length === WEB06_THRESHOLDS.observer.maximumTripletAttempts);
  const scenarioComplete = !observer && (payload.expectation === "PREVIEW"
    ? payload.scenarioResults?.every((scenario) => scenario.attempts?.length === 1)
    : payload.scenarioResults?.every((scenario) => scenario.measuredRoundCount === WEB06_THRESHOLDS.attempts.requiredValid
      || scenario.attempts?.length === WEB06_THRESHOLDS.attempts.maximum
      || (scenario.attempts?.at(-1)?.measurementCompleted === false
        && scenario.attempts.at(-1).retainedHardRed === true)));
  const completed = observerComplete || scenarioComplete;
  const expectedPlanned = observer ? 1 : payload.scenarios?.length;
  const expectedExecuted = observer ? Number(completed) : payload.scenarioResults?.length;
  if (payload.execution?.plannedScenarioCount !== expectedPlanned
    || payload.execution?.executedScenarioCount !== expectedExecuted
    || payload.execution?.skippedScenarioCount !== 0 || payload.execution?.unexpectedScenarioCount !== 0
    || payload.execution?.status !== (completed ? "completed" : "incomplete")
    || payload.measurementCompleted !== completed) errors.push("attestation:execution-reconciliation");
  const expectedStarted = observer
    ? payload.observerTriplets?.some((triplet) => ["product", "minimal", "full"]
      .some((modeName) => triplet[modeName]?.measurementStarted === true))
    : payload.scenarioResults?.some((scenario) => scenario.attempts?.some((attempt) => attempt.measurementStarted === true));
  if (payload.measurementStarted !== expectedStarted) errors.push("attestation:measurement-start-reconciliation");
  let expectedVerdict;
  if (observer) {
    try {
      expectedVerdict = evaluateObserverOverhead(payload.observerTriplets ?? []).status;
    } catch {
      errors.push("attestation:observer-verdict-recompute");
    }
  } else if (payload.expectation === "PREVIEW") {
    expectedVerdict = payload.scenarioResults?.every((scenario) => scenario.verdict === "PASS") ? "PASS"
      : payload.scenarioResults?.some((scenario) => scenario.verdict === "RED") ? "RED" : "SETUP_INVALID";
  } else {
    expectedVerdict = payload.scenarioResults?.some((scenario) => scenario.verdict === "SETUP_NO_GO")
      ? "SETUP_NO_GO" : payload.scenarioResults?.some((scenario) => scenario.verdict !== "PASS") ? "RED" : "PASS";
  }
  if (expectedVerdict !== undefined && payload.verdict !== expectedVerdict) {
    errors.push("attestation:verdict-reconciliation");
  }
  if ((observer && (payload.scenarioResults?.length !== 0 || payload.observerTriplets?.length > 7))
    || (!observer && (payload.observerTriplets?.length !== 0
      || payload.scenarioResults?.length !== payload.scenarios?.length))) errors.push("attestation:lane-shape");
  if (!schemaExactKeys(payload.privacy, ["publicAllowlistVersion", "pass"], errors, "attestation.privacy")
    || payload.privacy.publicAllowlistVersion !== "web06-public-evidence-v1" || payload.privacy.pass !== true) {
    errors.push("attestation.privacy:values");
  }
}

/** Exact recursive schemas for every compact public run artifact. */
export function validateWeb06RunArtifactSchema(fileName, payload) {
  const errors = [];
  if (fileName === "collector-output.json") validateCollectorOutputSchema(payload, errors);
  else if (fileName === "independent-recompute.json") validateIndependentSchema(payload, errors);
  else if (fileName === "suite-attestation.json") validateAttestationSchema(payload, errors);
  else errors.push(`artifact:unknown-file:${fileName}`);
  const privacy = validatePointerFreePrivacy(payload);
  errors.push(...privacy.errors);
  return { pass: errors.length === 0, errors };
}

function outputRelativeToCanonicalEvidenceRoot({ evidenceRoot, canonicalRoot, outputPath, fileName }) {
  const candidate = path.resolve(outputPath);
  setupAssert(path.basename(candidate) === fileName, "WEB06_RUN_ARTIFACT_PATH_INVALID", fileName);
  const roots = [...new Set([path.resolve(evidenceRoot), canonicalRoot])];
  const relative = roots.map((root) => path.relative(root, candidate)).find((value) =>
    value && value !== ".." && !value.startsWith(`..${path.sep}`) && !path.isAbsolute(value));
  setupAssert(relative !== undefined, "WEB06_RUN_ARTIFACT_PATH_INVALID", fileName);
  return {
    relative,
    destination: path.resolve(canonicalRoot, relative),
  };
}

async function writeRunJsonArtifact({ evidenceRoot, repoRoot, runId, sourceCommit, fileName, outputPath, payload }) {
  const schema = validateWeb06RunArtifactSchema(fileName, payload);
  setupAssert(schema.pass, "WEB06_PUBLIC_EVIDENCE_SCHEMA", schema.errors.join(","));
  const canonicalRepo = await realpath(repoRoot);
  const rootInfo = await ensureNoSymlinkDirectory(evidenceRoot, []);
  setupAssert(rootInfo.canonicalRoot !== canonicalRepo && !isWithin(canonicalRepo, rootInfo.canonicalRoot),
    "WEB06_EVIDENCE_ROOT_INSIDE_REPOSITORY");
  setupAssert(COMMIT_RE.test(sourceCommit) && typeof runId === "string" && SAFE_SEGMENT_RE.test(runId),
    "WEB06_RUN_ARTIFACT_IDENTITY_INVALID");
  const { relative: relativeDestination, destination } = outputRelativeToCanonicalEvidenceRoot({
    evidenceRoot,
    canonicalRoot: rootInfo.canonicalRoot,
    outputPath,
    fileName,
  });
  const directorySegments = path.dirname(relativeDestination) === "." ? [] : path.dirname(relativeDestination).split(path.sep);
  const { directory } = await ensureNoSymlinkDirectory(rootInfo.canonicalRoot, directorySegments);
  setupAssert(path.join(directory, fileName) === destination, "WEB06_RUN_ARTIFACT_PATH_CANONICAL_MISMATCH", fileName);
  const bytes = Buffer.from(`${JSON.stringify(payload, null, 2)}\n`, "utf8");
  const handle = await open(destination, "wx", 0o644);
  try {
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
  return Object.freeze({ relativePath: relativeDestination.split(path.sep).join("/"),
    bytes: bytes.length, sha256: sha256(bytes), path: destination });
}

function validateScenarioResult(result, config) {
  setupAssert(result && typeof result === "object", "WEB06_SCENARIO_RESULT_INVALID");
  const run = resolveScenarioRun(result.scenarioRunId);
  setupAssert(result.scenarioId === run.scenarioId && result.schemaId === run.schema,
    "WEB06_SCENARIO_RESULT_RUN_IDENTITY_INVALID", result.scenarioRunId);
  setupAssert(config.scenarioIds.includes(result.scenarioRunId), "WEB06_SCENARIO_RESULT_UNDECLARED", result.scenarioRunId);
  const preview = config.expectation === "PREVIEW";
  setupAssert(Array.isArray(result.attempts) && result.attempts.length <= (preview ? 1 : WEB06_THRESHOLDS.attempts.maximum),
    "WEB06_SCENARIO_RESULT_ATTEMPTS_INVALID", result.scenarioRunId);
  setupAssert(result.attempts.every((attempt) => typeof attempt.attemptId === "string"
    && typeof attempt.measurementStarted === "boolean" && typeof attempt.measurementCompleted === "boolean"
    && typeof attempt.retryEligible === "boolean" && typeof attempt.validRedObserved === "boolean"
    && typeof attempt.retainedMeasured === "boolean" && typeof attempt.retainedLogicalRound === "boolean"
    && typeof attempt.validForLatencyFrame === "boolean" && typeof attempt.retainedHardRed === "boolean"
    && typeof attempt.rawPacket?.relativePath === "string" && !path.isAbsolute(attempt.rawPacket.relativePath)
    && isSha256(attempt.rawPacket.sha256) && Number.isSafeInteger(attempt.rawPacket.bytes)),
  "WEB06_SCENARIO_RESULT_ATTEMPT_SHAPE", result.scenarioRunId);
  setupAssert(Number.isSafeInteger(result.measuredRoundCount) && result.measuredRoundCount >= 0
    && result.measuredRoundCount <= (preview ? 1 : WEB06_THRESHOLDS.attempts.requiredValid),
  "WEB06_SCENARIO_RESULT_MEASURED_COUNT", result.scenarioRunId);
  setupAssert(Number.isSafeInteger(result.validLatencyFrameRoundCount)
    && result.validLatencyFrameRoundCount >= 0
    && result.validLatencyFrameRoundCount <= result.measuredRoundCount,
  "WEB06_SCENARIO_RESULT_VALID_LATENCY_FRAME_COUNT", result.scenarioRunId);
  if (preview) {
    setupAssert(result.attempts.length === 1, "WEB06_PREVIEW_ATTEMPT_COUNT", result.scenarioRunId);
  } else if (result.measuredRoundCount < WEB06_THRESHOLDS.attempts.requiredValid) {
    const terminalIncompleteRed = result.attempts.at(-1)?.measurementCompleted === false
      && result.attempts.at(-1).retainedHardRed === true
      && result.attempts.at(-1).retainedLogicalRound === false;
    setupAssert(result.attempts.length === WEB06_THRESHOLDS.attempts.maximum || terminalIncompleteRed,
      "WEB06_SCENARIO_RESULT_PREMATURE_SETUP_NO_GO", result.scenarioRunId);
  }
  return result;
}

function validateRawPacketReference(rawPacket, detail) {
  setupAssert(typeof rawPacket?.relativePath === "string" && !path.isAbsolute(rawPacket.relativePath)
    && !rawPacket.relativePath.split("/").includes("..")
    && isSha256(rawPacket.sha256) && Number.isSafeInteger(rawPacket.bytes) && rawPacket.bytes > 0,
  "WEB06_RAW_PACKET_REFERENCE_INVALID", detail);
}

function validateObserverTriplet(triplet, index) {
  setupAssert(triplet?.attemptId === `triplet-attempt-${index + 1}`,
    "WEB06_OBSERVER_TRIPLET_ATTEMPT_ID", String(index + 1));
  setupAssert(typeof triplet.valid === "boolean" && Number.isSafeInteger(triplet.counterbalanceSlot)
    && Array.isArray(triplet.modeContextIds) && triplet.modeContextIds.length === 3
    && Array.isArray(triplet.modeOrder) && triplet.modeOrder.length === 3
    && triplet.modeFixedBeforePageLoad === true,
  "WEB06_OBSERVER_TRIPLET_SHAPE", triplet.attemptId);
  for (const modeName of ["product", "minimal", "full"]) {
    const mode = triplet[modeName];
    setupAssert(mode && typeof mode === "object"
      && typeof mode.measurementStarted === "boolean"
      && typeof mode.measurementCompleted === "boolean"
      && typeof mode.measurementValid === "boolean",
    "WEB06_OBSERVER_MODE_SHAPE", `${triplet.attemptId}:${modeName}`);
    validateRawPacketReference(mode.rawPacket, `${triplet.attemptId}:${modeName}`);
  }
}

export function buildCollectorOutput({ config, scenarioResults = [], observerTriplets = [], observerEvaluation,
  runnerSourceBefore, runnerSourceAfter }) {
  const isObserver = config.expectation === "OBSERVER";
  setupAssert(isObserver ? scenarioResults.length === 0 : observerTriplets.length === 0,
    "WEB06_COLLECTOR_OUTPUT_LANE_SHAPE_INVALID");
  if (!isObserver) {
    setupAssert(scenarioResults.length === config.targetOrder.length * config.scenarioIds.length,
      "WEB06_COLLECTOR_OUTPUT_SCENARIO_RESULT_COUNT");
    for (const result of scenarioResults) validateScenarioResult(result, config);
  } else {
    setupAssert(observerEvaluation && typeof observerEvaluation.status === "string",
      "WEB06_OBSERVER_EVALUATION_MISSING");
    setupAssert(observerTriplets.length <= WEB06_THRESHOLDS.observer.maximumTripletAttempts,
      "WEB06_OBSERVER_TRIPLET_ATTEMPT_CAP");
    observerTriplets.forEach(validateObserverTriplet);
  }
  const executionCompleted = isObserver
    ? observerEvaluation.status !== "INCOMPLETE"
    : config.expectation === "PREVIEW"
      ? scenarioResults.every((result) => result.attempts.length === 1)
      : scenarioResults.every((result) => result.measuredRoundCount === WEB06_THRESHOLDS.attempts.requiredValid
        || result.attempts.length === WEB06_THRESHOLDS.attempts.maximum
        || (result.attempts.at(-1)?.measurementCompleted === false
          && result.attempts.at(-1).retainedHardRed === true
          && result.attempts.at(-1).retainedLogicalRound === false));
  return {
    version: "web06-collector-output-v1",
    writeMode: "create-new",
    expectation: config.expectation,
    disposition: config.disposition,
    selectedBranch: config.branch,
    versions: {
      metric: WEB06_METRIC_CONTRACT_VERSION,
      scenarioRegistry: WEB06_SCENARIO_REGISTRY_VERSION,
      behaviorPredicate: WEB06_BEHAVIOR_PREDICATE_VERSION,
    },
    identityManifestSha256: config.identityManifestSha256,
    runnerSourceManifest: clone(config.runnerSource),
    runnerSourceManifestSha256: config.runnerSourceManifestSha256,
    runnerSourceObservationSha256: runnerSourceBefore?.observationSha256,
    runnerSourcePostObservationSha256: runnerSourceAfter?.observationSha256,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    scenarioRuns: [...config.scenarioIds],
    execution: {
      plannedScenarioCount: isObserver ? 1 : config.targetOrder.length * config.scenarioIds.length,
      executedScenarioCount: isObserver ? Number(executionCompleted) : scenarioResults.length,
      skippedScenarioCount: 0,
      unexpectedScenarioCount: 0,
      status: executionCompleted ? "completed" : "incomplete",
    },
    measurementStarted: isObserver
      ? observerTriplets.some((triplet) => ["product", "minimal", "full"]
        .some((modeName) => triplet[modeName]?.measurementStarted === true))
      : scenarioResults.some((result) => result.attempts.some((attempt) => attempt.measurementStarted)),
    measurementCompleted: executionCompleted,
    scenarioResults: clone(scenarioResults),
    observerTriplets: clone(observerTriplets),
    ...(observerEvaluation === undefined ? {} : { observerEvaluation: clone(observerEvaluation) }),
  };
}

export async function writeCollectorOutput({ config, scenarioResults, observerTriplets, observerEvaluation,
  runnerSourceBefore, runnerSourceAfter }) {
  setupAssert(runnerSourceBefore?.observationSha256 === runnerSourceAfter?.observationSha256,
    "WEB06_RUNNER_SOURCE_PRE_POST_MISMATCH");
  const payload = buildCollectorOutput({ config, scenarioResults, observerTriplets, observerEvaluation,
    runnerSourceBefore, runnerSourceAfter });
  const sourceCommit = config.targets[config.targetOrder[0]].sourceCommit;
  return {
    payload,
    artifact: await writeRunJsonArtifact({
      evidenceRoot: config.evidenceRoot,
      repoRoot: config.repoRoot,
      runId: config.runId,
      sourceCommit,
      fileName: "collector-output.json",
      outputPath: config.outputPaths.collector,
      payload,
    }),
  };
}

/** Exact named runner and target source/artifact roles for release consumption. */
export function buildSuiteSourceArtifactRoles({ config, runnerSourceBefore, runnerSourceAfter }) {
  const targetRoles = Object.fromEntries(expectedSuiteTargetRoles(config.expectation).map((roleName) => {
    const pinned = config.identityManifest.roles[roleName];
    const targets = Object.fromEntries(config.targetOrder
      .filter((targetId) => config.targets[targetId].identityRole === roleName)
      .map((targetId) => {
        const target = config.targets[targetId];
        return [targetId, {
          protocolMode: target.protocolMode,
          selectorPolicy: target.selectorPolicy,
          artifactResponseGuardSha256: target.artifactResponseGuardSha256,
        }];
      }));
    return [roleName, {
      role: roleName,
      sourceCommit: pinned.sourceCommit,
      sourceTree: pinned.sourceTree,
      sourceTreeState: pinned.sourceTreeState,
      archiveSha256: pinned.archiveSha256,
      artifactManifestSha256: pinned.artifactManifestSha256,
      buildInfoSha256: pinned.buildInfoSha256,
      selectedBranch: pinned.selectedBranch,
      disposition: pinned.disposition,
      targets,
    }];
  }));
  const runnerIdentityRole = ["FINAL", "PREVIEW"].includes(config.expectation) ? "FINAL" : "BASE";
  const bindings = stableJsonValue({
    version: "web06-suite-source-artifact-roles-v1",
    runnerSource: {
      role: "RUNNER_SOURCE",
      identityRole: runnerIdentityRole,
      sourceCommit: config.runnerSource.sourceCommit,
      sourceTree: config.runnerSource.sourceTree,
      sourceTreeState: config.runnerSource.sourceTreeState,
      sourceManifestSha256: config.runnerSourceManifestSha256,
      toolingManifestSha256: config.runnerSource.toolingManifestSha256,
      beforeObservationSha256: runnerSourceBefore?.observationSha256,
      afterObservationSha256: runnerSourceAfter?.observationSha256,
    },
    targetRoles,
  });
  return stableJsonValue({
    ...bindings,
    bindingsSha256: digestJson(bindings),
  });
}

export async function writeSuiteAttestation({ config, collectorOutputArtifact, independentRecomputeArtifact,
  scenarioResults = [], observerTriplets = [], verdict, runnerSourceBefore, runnerSourceAfter }) {
  for (const artifact of [collectorOutputArtifact, independentRecomputeArtifact]) {
    setupAssert(artifact?.relativePath && isSha256(artifact.sha256) && Number.isSafeInteger(artifact.bytes),
      "WEB06_ATTESTATION_ARTIFACT_INVALID");
  }
  const observerComplete = config.expectation === "OBSERVER"
    && (observerTriplets.filter((triplet) => triplet.valid === true).length === WEB06_THRESHOLDS.observer.requiredTriplets
      || observerTriplets.length === WEB06_THRESHOLDS.observer.maximumTripletAttempts);
  const scenarioComplete = config.expectation !== "OBSERVER"
    && (config.expectation === "PREVIEW"
      ? scenarioResults.every((scenario) => scenario.attempts?.length === 1)
      : scenarioResults.every((scenario) => scenario.measuredRoundCount === WEB06_THRESHOLDS.attempts.requiredValid
        || scenario.attempts?.length === WEB06_THRESHOLDS.attempts.maximum
        || (scenario.attempts?.at(-1)?.measurementCompleted === false
          && scenario.attempts.at(-1).retainedHardRed === true)));
  const executionCompleted = observerComplete || scenarioComplete;
  const measurementStarted = config.expectation === "OBSERVER"
    ? observerTriplets.some((triplet) => ["product", "minimal", "full"]
      .some((modeName) => triplet[modeName]?.measurementStarted === true))
    : scenarioResults.some((scenario) => scenario.attempts?.some((attempt) => attempt.measurementStarted === true));
  const payload = {
    version: "web06-suite-attestation-v1",
    writeMode: "create-new",
    expectation: config.expectation,
    disposition: config.disposition,
    selectedBranch: config.branch,
    identityManifestSha256: config.identityManifestSha256,
    runnerSourceManifestSha256: config.runnerSourceManifestSha256,
    runnerSourceObservationSha256: runnerSourceBefore?.observationSha256,
    runnerSourcePostObservationSha256: runnerSourceAfter?.observationSha256,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    sourceArtifactRoles: buildSuiteSourceArtifactRoles({ config, runnerSourceBefore, runnerSourceAfter }),
    versions: {
      metric: WEB06_METRIC_CONTRACT_VERSION,
      scenarioRegistry: WEB06_SCENARIO_REGISTRY_VERSION,
      behaviorPredicate: WEB06_BEHAVIOR_PREDICATE_VERSION,
    },
    scenarios: [...config.scenarioIds],
    execution: {
      plannedScenarioCount: config.expectation === "OBSERVER" ? 1 : config.targetOrder.length * config.scenarioIds.length,
      executedScenarioCount: config.expectation === "OBSERVER"
        ? Number(executionCompleted)
        : scenarioResults.length,
      skippedScenarioCount: 0,
      unexpectedScenarioCount: 0,
      status: executionCompleted ? "completed" : "incomplete",
    },
    collectorOutput: {
      relativePath: collectorOutputArtifact.relativePath,
      bytes: collectorOutputArtifact.bytes,
      sha256: collectorOutputArtifact.sha256,
    },
    independentRecompute: {
      relativePath: independentRecomputeArtifact.relativePath,
      bytes: independentRecomputeArtifact.bytes,
      sha256: independentRecomputeArtifact.sha256,
    },
    measurementStarted,
    measurementCompleted: executionCompleted,
    verdict,
    scenarioResults: clone(scenarioResults),
    observerTriplets: clone(observerTriplets),
    privacy: { publicAllowlistVersion: "web06-public-evidence-v1", pass: true },
  };
  const sourceCommit = config.targets[config.targetOrder[0]].sourceCommit;
  return writeRunJsonArtifact({
    evidenceRoot: config.evidenceRoot,
    repoRoot: config.repoRoot,
    runId: config.runId,
    sourceCommit,
    fileName: "suite-attestation.json",
    outputPath: config.outputPaths.attestation,
    payload,
  });
}

export function advanceCadenceDeadline({ previousActualDispatchAt, nominalGapMs, nowMs }) {
  setupAssert(Number.isFinite(previousActualDispatchAt), "WEB06_CADENCE_PREVIOUS_NONFINITE");
  setupAssert(Number.isFinite(nominalGapMs) && nominalGapMs > 0, "WEB06_CADENCE_NOMINAL_NONFINITE");
  setupAssert(Number.isFinite(nowMs), "WEB06_CADENCE_NOW_NONFINITE");
  const phaseDeadline = previousActualDispatchAt + nominalGapMs;
  return Object.freeze({
    requestedDispatchAt: Math.max(phaseDeadline, nowMs),
    rebasedAfterLateHost: nowMs > phaseDeadline,
    phaseDeadline,
  });
}

export function combinedAttemptFacts({
  internalParsed,
  commonParsed,
  attemptId,
  measurementStarted = true,
  measurementCompleted = true,
}) {
  const parsedResults = [internalParsed, commonParsed].filter(Boolean);
  setupAssert(parsedResults.length > 0, "WEB06_ATTEMPT_PARSER_RESULT_MISSING", attemptId);
  const cadences = [...new Set(parsedResults.map((parsed) => parsed.cadence ?? "NOT_APPLICABLE"))];
  const validRedObserved = parsedResults.some((parsed) => ["RED", "RED_BEHAVIOR"].includes(parsed.status));
  const parserSetupInvalid = cadences.length !== 1 || parsedResults.some((parsed) =>
    parsed.status === "SETUP_INVALID"
      || (parsed.setupErrors?.length ?? 0) > 0
      || parsed.qualifiers?.setupInvalid === true);
  return {
    attemptId,
    measurementStarted,
    measurementCompleted,
    validRedObserved,
    setupInvalid: parserSetupInvalid,
    instrumentationAttributionInvalid: validRedObserved && parserSetupInvalid,
    cadence: cadences.length === 1 ? cadences[0]
      : validRedObserved ? (parsedResults.find((parsed) => ["RED", "RED_BEHAVIOR"].includes(parsed.status))?.cadence ?? "NOT_APPLICABLE")
        : "SETUP_INVALID_PARSER_CADENCE_DISAGREEMENT",
    behaviorRed: parsedResults.some((parsed) => (parsed.behaviorErrors?.length ?? 0) > 0
      || parsed.status === "RED_BEHAVIOR"),
    latencyRed: parsedResults.some((parsed) => (parsed.thresholdViolations?.length ?? 0) > 0),
    frameRed: parsedResults.some((parsed) => parsed.frameRed === true),
    longTaskRed: parsedResults.some((parsed) => parsed.longTaskRed === true),
  };
}

/**
 * Evaluate the full FINAL lane without turning an evidence-selected no-go into
 * an impossible "every scenario must fail" contract. A no-go still requires
 * at least one coherent measured RED, while unaffected rows must remain green
 * and setup/no-verdict rows always block closeout.
 */
export function evaluateFinalLaneDisposition({ disposition, scenarios }) {
  const violations = [];
  if (!Array.isArray(scenarios) || scenarios.length === 0) {
    return { pass: false, validRedObserved: false, violations: ["FINAL_SCENARIOS_MISSING"] };
  }
  if (!["PRODUCTION_REDUCTION", "MEASURED_NO_GO"].includes(disposition)) {
    return { pass: false, validRedObserved: false, violations: ["FINAL_DISPOSITION_INVALID"] };
  }
  let validRedObserved = false;
  for (const scenario of scenarios) {
    const label = scenario?.scenarioRunId ?? "unknown-scenario";
    if (!Number.isSafeInteger(scenario?.validLatencyFrameRoundCount)
      || scenario.validLatencyFrameRoundCount < 0
      || scenario.validLatencyFrameRoundCount > WEB06_THRESHOLDS.attempts.requiredValid) {
      violations.push(`${label}:FINAL_VALID_LATENCY_FRAME_COUNT_INVALID`);
      continue;
    }
    if (scenario.seriesStatus === "COMPLETE_GREEN") {
      if (scenario.validLatencyFrameRoundCount !== WEB06_THRESHOLDS.attempts.requiredValid
        || typeof scenario?.internalPoolPass !== "boolean"
        || typeof scenario?.commonPoolPass !== "boolean") {
        violations.push(`${label}:FINAL_POOL_STATUS_MISSING`);
        continue;
      }
      const poolsPass = scenario.internalPoolPass && scenario.commonPoolPass;
      if (!poolsPass) violations.push(`${label}:COMPLETE_GREEN_POOL_MISMATCH`);
      continue;
    }
    if (scenario.seriesStatus === "COMPLETE_WITH_RED") {
      validRedObserved = true;
      if (scenario.validLatencyFrameRoundCount === WEB06_THRESHOLDS.attempts.requiredValid) {
        if (typeof scenario?.internalPoolPass !== "boolean"
          || typeof scenario?.commonPoolPass !== "boolean") {
          violations.push(`${label}:FINAL_POOL_STATUS_MISSING`);
        } else if (scenario.internalPoolPass && scenario.commonPoolPass) {
          violations.push(`${label}:COMPLETE_RED_POOL_MISMATCH`);
        }
      } else if (scenario.internalPoolPass !== undefined || scenario.commonPoolPass !== undefined) {
        violations.push(`${label}:INCOMPLETE_RED_POOL_PRESENT`);
      }
      if (disposition === "PRODUCTION_REDUCTION") violations.push(`${label}:FINAL_NOT_GREEN`);
      continue;
    }
    violations.push(`${label}:SETUP_NO_GO:${scenario?.seriesStatus ?? "MISSING"}`);
  }
  if (disposition === "MEASURED_NO_GO" && !validRedObserved) {
    violations.push("MEASURED_NO_GO_WITHOUT_RED");
  }
  return { pass: violations.length === 0, validRedObserved, violations };
}

/**
 * Collect five measured rounds within seven retained attempts. PASS and RED
 * both count immediately; only setup/no-verdict attempts are replaceable.
 */
export async function collectFiveWithinSeven(runAttempt) {
  const attempts = [];
  const measuredReceipts = [];
  const measuredAttempts = [];
  const validLatencyFrameReceipts = [];
  const validLatencyFrameAttempts = [];
  for (let index = 1; index <= WEB06_THRESHOLDS.attempts.maximum; index += 1) {
    const attemptId = `attempt-${index}`;
    const result = await runAttempt({ attemptId, attemptNumber: index });
    setupAssert(result && typeof result === "object" && result.parsed, "WEB06_ATTEMPT_RESULT_INVALID", attemptId);
    const facts = combinedAttemptFacts({
      internalParsed: result.parsed,
      commonParsed: result.commonParsed,
      attemptId,
      measurementStarted: result.measurementStarted === true,
      measurementCompleted: result.measurementCompleted === true,
    });
    setupAssert(facts.attemptId === attemptId, "WEB06_ATTEMPT_ID_MISMATCH", attemptId);
    attempts.push({ ...result, attemptFacts: facts });
    const series = evaluateAttemptSeries(attempts.map((attempt) => attempt.attemptFacts));
    if (series.retained.at(-1).retainedLogicalRound === true) {
      measuredReceipts.push(result.receipt);
      measuredAttempts.push(result);
    }
    if (series.retained.at(-1).validForLatencyFrame === true) {
      validLatencyFrameReceipts.push(result.receipt);
      validLatencyFrameAttempts.push(result);
    }
    if (series.measuredCount === WEB06_THRESHOLDS.attempts.requiredValid) {
      return Object.freeze({
        attempts,
        measuredReceipts,
        measuredAttempts,
        validLatencyFrameReceipts,
        validLatencyFrameAttempts,
        series,
      });
    }
    if (series.retained.at(-1).verdict === "RED_INCOMPLETE_BEHAVIOR") {
      return Object.freeze({
        attempts,
        measuredReceipts,
        measuredAttempts,
        validLatencyFrameReceipts,
        validLatencyFrameAttempts,
        series,
      });
    }
  }
  return Object.freeze({
    attempts,
    measuredReceipts,
    measuredAttempts,
    validLatencyFrameReceipts,
    validLatencyFrameAttempts,
    series: evaluateAttemptSeries(attempts.map((attempt) => attempt.attemptFacts)),
  });
}

export function protocolCapabilityBlockers({ mode, scenarioId, protocol, status, invalidations = [], uiCapabilities = {}, selectedBranch = "NONE" }) {
  const blockers = [];
  if (mode === "off") {
    if (protocol !== undefined && protocol !== null) blockers.push("PRODUCT_PRIVATE_PROTOCOL_PRESENT");
    return blockers;
  }
  if (!protocol || typeof protocol !== "object") return ["PRIVATE_PROTOCOL_MISSING"];
  if (protocol.protocolVersion !== WEB06_PRIVATE_PROTOCOL_VERSION) blockers.push("PRIVATE_PROTOCOL_VERSION_MISMATCH");
  if (protocol.mode !== mode) blockers.push("PRIVATE_PROTOCOL_MODE_MISMATCH");
  if (!status || status.valid !== true) blockers.push("PRIVATE_PROTOCOL_INVALID");
  if (status?.queueDepth !== 0 || status?.runningActionId !== undefined) blockers.push("ACTION_QUEUE_NOT_IDLE");
  if (status?.pendingFanoutActions !== 0) blockers.push("PENDING_FANOUT_ACTIONS");
  if (status?.pendingTerminalActions !== 0) blockers.push("PENDING_TERMINAL_ACTIONS");
  if (!Number.isSafeInteger(status?.receiptWindowStartEventSequenceId)
    || !Number.isSafeInteger(status?.receiptWindowStartActionSequenceId)) blockers.push("PROTOCOL_WINDOW_START_MISSING");
  if (!Array.isArray(invalidations) || invalidations.length > 0) blockers.push("PRIVATE_PROTOCOL_INVALIDATIONS");
  if (scenarioId === "fifo-pressure-barriers" && uiCapabilities.importUserdbSameTask !== true) {
    blockers.push("FIFO_IMPORT_SAME_TASK_UI_UNSUPPORTED");
  }
  if (scenarioId === "learned-row" && uiCapabilities.backgroundCausality !== true) {
    blockers.push("BACKGROUND_CAUSALITY_UNPROVED");
  }
  if (scenarioId === "learned-row" && uiCapabilities.browserLifecycleContinuity !== true) {
    blockers.push("BROWSER_LIFECYCLE_PROTOCOL_CONTINUITY_UNPROVED");
  }
  if (scenarioId === "extended-scheduler-barriers") {
    if (selectedBranch !== "B") blockers.push("EXTENDED_SCENARIO_BRANCH_B_NOT_SELECTED");
    if (uiCapabilities.publicDeployControl !== true) blockers.push("PUBLIC_DEPLOY_CONTROL_HIDDEN");
    if (uiCapabilities.publicCustomizeValueControl !== true) blockers.push("PUBLIC_CUSTOMIZE_VALUE_CONTROL_HIDDEN");
  }
  return [...new Set(blockers)];
}

export function protocolHealthBlockers(protocolExport, { requireCallbackLedger = false } = {}) {
  if (!protocolExport || typeof protocolExport !== "object") return ["PRIVATE_PROTOCOL_EXPORT_MISSING"];
  const blockers = [];
  const status = protocolExport.status;
  if (status?.valid !== true) blockers.push("PRIVATE_PROTOCOL_INVALID");
  if (status?.queueDepth !== 0 || status?.runningActionId !== undefined) blockers.push("ACTION_QUEUE_NOT_IDLE");
  if (status?.pendingFanoutActions !== 0) blockers.push("PENDING_FANOUT_ACTIONS");
  if (status?.pendingTerminalActions !== 0) blockers.push("PENDING_TERMINAL_ACTIONS");
  if (!Array.isArray(protocolExport.invalidations) || protocolExport.invalidations.length > 0) {
    blockers.push("PRIVATE_PROTOCOL_INVALIDATIONS");
  }
  if (requireCallbackLedger) {
    const callbacks = protocolExport.mainObserverCallbacks;
    const durations = protocolExport.mainObserverCallbacksMs;
    if (!Array.isArray(callbacks) || !Array.isArray(durations)) {
      blockers.push("MAIN_OBSERVER_CALLBACK_LEDGER_MISSING");
    } else {
      const callbackIds = callbacks.map((callback) => callback?.callbackId);
      const sequenceIds = callbacks.map((callback) => callback?.sequenceId);
      const rowsValid = callbacks.every((callback) =>
        callback && typeof callback === "object" && !Array.isArray(callback)
        && typeof callback.callbackId === "string" && callback.callbackId.length > 0
        && Number.isSafeInteger(callback.sequenceId) && callback.sequenceId > 0
        && typeof callback.operation === "string" && callback.operation.length > 0
        && Number.isFinite(callback.startedAt) && Number.isFinite(callback.finishedAt)
        && callback.finishedAt >= callback.startedAt && Number.isFinite(callback.durationMs)
        && callback.durationMs >= 0
        && callback.durationMs === callback.finishedAt - callback.startedAt
        && (callback.actionId === undefined
          || (typeof callback.actionId === "string" && callback.actionId.length > 0))
        && (callback.eventId === undefined
          || (typeof callback.eventId === "string" && callback.eventId.length > 0)));
      const orderedUnique = new Set(callbackIds).size === callbackIds.length
        && new Set(sequenceIds).size === sequenceIds.length
        && sequenceIds.every((sequenceId, index) => index === 0 || sequenceId > sequenceIds[index - 1]);
      if (!rowsValid || !orderedUnique
        || durations.some((value) => !Number.isFinite(value) || value < 0)) {
        blockers.push("MAIN_OBSERVER_CALLBACK_LEDGER_INVALID");
      }
      if (JSON.stringify(durations) !== JSON.stringify(callbacks.map((callback) => callback?.durationMs))) {
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
  if (protocolExport.actions !== undefined) {
    if (!Array.isArray(protocolExport.actions)) {
      blockers.push("PRIVATE_PROTOCOL_ACTION_EXPORT_INVALID");
    } else {
      for (const action of protocolExport.actions) {
        if (!action.returnedIdentity) blockers.push("RETURNED_WIRE_IDENTITY_MISSING");
        if (!action.worker || !Array.isArray(action.worker.observerFailures)) {
          blockers.push("WORKER_OBSERVER_FAILURE_EXPORT_MISSING");
        } else if (action.worker.observerFailures.length > 0) {
          blockers.push("WORKER_OBSERVER_FAILURE");
        }
        const callbacks = action.worker?.collectorSpans;
        if (!Array.isArray(callbacks)) {
          blockers.push("WORKER_COLLECTOR_SPAN_EXPORT_MISSING");
        } else {
          if (callbacks.some((span) => !span || typeof span !== "object" || Array.isArray(span)
            || !Number.isFinite(span.startedAt) || !Number.isFinite(span.finishedAt)
            || span.finishedAt < span.startedAt)) blockers.push("COLLECTOR_CALLBACK_SPAN_INVALID");
          if (callbacks.some((span) => Number.isFinite(span?.startedAt) && Number.isFinite(span?.finishedAt)
            && span.finishedAt - span.startedAt
            >= WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)) {
            blockers.push("COLLECTOR_CALLBACK_CEILING");
          }
        }
      }
    }
  }
  return [...new Set(blockers)];
}

function strictPrefix(left, right) {
  return typeof left === "string" && typeof right === "string" && left.length < right.length && right.startsWith(left);
}

function terminalOwner(step, actions) {
  const owned = actions.filter((action) => action.stepId === step.id);
  const foreground = owned.filter((action) => action.background !== true);
  return foreground.length !== owned.length ? foreground[0] ?? owned[0] : foreground.at(-1);
}

export function expectedCommonSamples(scenarioId) {
  const row = hasOwn(SCENARIO_REGISTRY, scenarioId) ? SCENARIO_REGISTRY[scenarioId] : undefined;
  setupAssert(row !== undefined, "WEB06_SCENARIO_UNKNOWN", scenarioId);
  const expected = expandScenarioExpectedTimeline(scenarioId);
  let logicalInput = "";
  const samples = [];
  for (const step of row.steps) {
    for (const mapped of step.actions) {
      if (mapped.kind === "processKey") {
        const raw = mapped.args[0];
        const key = typeof raw === "string" && raw.startsWith("{") && raw.endsWith("}") ? raw.slice(1, -1) : "";
        if (key === "BackSpace") logicalInput = logicalInput.slice(0, -1);
        else if (["Escape", "space", "Return"].includes(key)) logicalInput = "";
        else if (!["Page_Down", "Page_Up", "Down", "Up"].includes(key) && key.length === 1) logicalInput += key;
      } else if (mapped.kind === "selectCandidate") {
        logicalInput = "";
      }
    }
    if (step.expectedLogicalInputAfter !== undefined) logicalInput = step.expectedLogicalInputAfter;
    if (step.source === "browser-lifecycle") logicalInput = "";
    if (step.sample === "none") continue;
    const owner = step.sample === "terminal"
      ? terminalOwner(step, expected.actions)
      : expected.actions.find((action) => action.stepId === step.id);
    samples.push({
      stepId: step.id,
      sampleKind: step.sample,
      eventSequenceId: owner?.eventSequenceId ?? owner?.causedByEventSequenceId,
      expectedInput: logicalInput,
      stressDeadline: owner?.stressDeadline === true,
    });
  }
  return samples;
}

export function resolveCommonSamples({ scenarioId, events, snapshots }) {
  const frozen = expectedCommonSamples(scenarioId);
  const resolved = [];
  const exactByIndex = new Map();
  for (let index = 0; index < frozen.length; index += 1) {
    const sample = frozen[index];
    const event = events.find((candidate) => candidate.eventSequenceId === sample.eventSequenceId);
    const exact = snapshots
      .filter((snapshot) => (event?.pageInstanceId === undefined || snapshot.pageInstanceId === event.pageInstanceId)
        && snapshot.observedAt >= (event?.normalizedEventAt ?? Infinity)
        && snapshot.stepId === sample.stepId
        && snapshot.stableDoubleRaf === true
        && snapshot.domObserved?.logicalInputProjection === sample.expectedInput)
      .sort((left, right) => left.observedAt - right.observedAt)[0];
    if (exact) exactByIndex.set(index, exact);
  }
  for (let index = 0; index < frozen.length; index += 1) {
    const sample = frozen[index];
    const exact = exactByIndex.get(index);
    if (exact) {
      resolved.push({
        ...sample,
        outcome: sample.sampleKind === "terminal" ? "terminal" : "painted",
        pageInstanceId: exact.pageInstanceId,
        observedAt: exact.observedAt,
        stableDoubleRaf: exact.stableDoubleRaf === true,
        firstDomObserved: clone(exact.firstDomObserved),
        domObserved: clone(exact.domObserved),
        domFingerprintSha256: digestJson(exact.domObserved),
      });
      continue;
    }
    let coveringIndex = -1;
    if (sample.sampleKind === "covering") {
      for (let candidate = index + 1; candidate <= Math.min(index + 2, frozen.length - 1); candidate += 1) {
        if (frozen[candidate].sampleKind === "covering"
          && strictPrefix(sample.expectedInput, frozen[candidate].expectedInput)
          && exactByIndex.has(candidate)) {
          coveringIndex = candidate;
          break;
        }
      }
    }
    if (coveringIndex >= 0) {
      const covering = exactByIndex.get(coveringIndex);
      resolved.push({
        ...sample,
        outcome: "superseded",
        supersededByStepId: frozen[coveringIndex].stepId,
        pageInstanceId: covering.pageInstanceId,
        observedAt: covering.observedAt,
        stableDoubleRaf: covering.stableDoubleRaf === true,
        firstDomObserved: clone(covering.firstDomObserved),
        domObserved: clone(covering.domObserved),
        domFingerprintSha256: digestJson(covering.domObserved),
      });
    } else {
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
      resolved.push({
        ...sample,
        outcome: "missing",
        pageInstanceId: event?.pageInstanceId,
        observedAt: events.find((candidate) => candidate.eventSequenceId === sample.eventSequenceId)?.normalizedEventAt ?? 0,
        stableDoubleRaf: false,
        firstDomObserved: clone(missingDom),
        domObserved: missingDom,
        domFingerprintSha256: digestJson(missingDom),
      });
    }
  }
  return resolved;
}

function digestJson(value) {
  return sha256(Buffer.from(JSON.stringify(value), "utf8"));
}

function stableJsonValue(value) {
  if (Array.isArray(value)) return value.map(stableJsonValue);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(Object.keys(value).sort().map((key) => [key, stableJsonValue(value[key])]));
}

function aggregateSpan(spans, predicate) {
  const selected = (spans ?? []).filter(predicate);
  if (!selected.length) return null;
  setupAssert(selected.every((span) => Number.isFinite(span.startedAt) && Number.isFinite(span.finishedAt)
    && span.finishedAt >= span.startedAt && ["success", "error"].includes(span.outcome)), "WEB06_WORKER_SPAN_INVALID");
  return {
    start: Math.min(...selected.map((span) => span.startedAt)),
    end: Math.max(...selected.map((span) => span.finishedAt)),
    outcomes: [...new Set(selected.map((span) => span.outcome))],
  };
}

function workerSpanRecord(worker, full) {
  if (!full) return { abi: null, responseExtract: null, jsonParse: null, adapterTranslate: null, persistence: null };
  return {
    abi: aggregateSpan(worker.runtimeSpans, (span) => span.stage === "abi-call"),
    responseExtract: aggregateSpan(worker.runtimeSpans, (span) => ["response-json-accessor", "response-byte-extraction"].includes(span.stage)),
    jsonParse: aggregateSpan(worker.runtimeSpans, (span) => ["response-json-parse", "response-shape-decode", "response-handled-accessor", "response-free"].includes(span.stage)),
    adapterTranslate: aggregateSpan(worker.adapterSpans, (span) => span.stage === "adapter-translation"),
    persistence: aggregateSpan(worker.persistenceSpans, () => true),
  };
}

function logicalInputFromRaw(rawJson, fallback) {
  try {
    const parsed = JSON.parse(rawJson);
    return typeof parsed?.context?.preedit === "string" ? parsed.context.preedit : fallback;
  } catch {
    return fallback;
  }
}

function commitFingerprint(presentation, previousTextareaValue) {
  const next = presentation.domObserved.textareaValue;
  let prefix = 0;
  while (prefix < previousTextareaValue.length && prefix < next.length && previousTextareaValue[prefix] === next[prefix]) prefix += 1;
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

/** Convert the complete private packet without discarding its wire identity. */
export function adaptPrivateProtocolReceipt({
  metadata,
  protocolWindow,
  wireEvents,
  wireActions,
  driverEvents,
  commonSurface,
  argumentCommitments = {},
  externalLifecycleEvents = [],
  protocolWindowSegments = [],
}) {
  const row = hasOwn(SCENARIO_REGISTRY, metadata.scenarioId)
    ? SCENARIO_REGISTRY[metadata.scenarioId]
    : undefined;
  setupAssert(row !== undefined, "WEB06_SCENARIO_UNKNOWN", metadata.scenarioId);
  setupAssert(metadata.mode !== "PRODUCT", "WEB06_PRODUCT_PRIVATE_ADAPTER_FORBIDDEN");
  const full = metadata.mode.endsWith("_FULL");
  const eventStart = protocolWindow.receiptWindowStartEventSequenceId;
  const actionStart = protocolWindow.receiptWindowStartActionSequenceId;
  setupAssert(Number.isSafeInteger(eventStart) && Number.isSafeInteger(actionStart), "WEB06_PROTOCOL_WINDOW_INVALID");
  const sourceWindows = protocolWindowSegments.length > 0 ? protocolWindowSegments : [{
    pageInstanceId: undefined,
    receiptWindowStartEventSequenceId: eventStart,
    receiptWindowStartActionSequenceId: actionStart,
  }];
  const windowForPage = (pageInstanceId) => sourceWindows.find((segment) =>
    segment.pageInstanceId === undefined || segment.pageInstanceId === pageInstanceId);
  setupAssert(sourceWindows.length === (metadata.scenarioId === "learned-row" ? 2 : 1)
    && sourceWindows.every((segment) => Number.isSafeInteger(segment.receiptWindowStartEventSequenceId)
      && Number.isSafeInteger(segment.receiptWindowStartActionSequenceId)),
  "WEB06_PROTOCOL_WINDOW_SEGMENTS_INVALID");
  const expected = expandScenarioExpectedTimeline(metadata.scenarioId);
  const ordinaryExpectedEvents = expected.events.filter((event) => event.type !== "browser-lifecycle");
  const expectedLifecycleEvents = expected.events.filter((event) => event.type === "browser-lifecycle");
  setupAssert(externalLifecycleEvents.length === expectedLifecycleEvents.length
    && (metadata.scenarioId !== "learned-row" || wireEvents.length === ordinaryExpectedEvents.length),
  "WEB06_PRIVATE_EVENT_SOURCE_CARDINALITY");
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
  const events = wireEvents.map((receipt, index) => {
    const identity = receipt.identity;
    const frozen = ordinaryExpectedEvents[index] ?? {};
    const localSequence = frozen.eventSequenceId;
    const driver = driverEvents[index] ?? {};
    const sourceWindow = windowForPage(receipt.web06PageInstanceId);
    const ordinal = eventOrdinalsByPage.get(receipt.web06PageInstanceId) ?? 0;
    setupAssert(sourceWindow && identity.eventSequenceId
      === sourceWindow.receiptWindowStartEventSequenceId + ordinal,
    "WEB06_PROTOCOL_EVENT_SEGMENT_SEQUENCE", String(index + 1));
    eventOrdinalsByPage.set(receipt.web06PageInstanceId, ordinal + 1);
    return {
      eventSequenceId: localSequence,
      pageInstanceId: receipt.web06PageInstanceId,
      stepId: frozen.stepId,
      type: identity.type,
      key: identity.key,
      code: identity.code,
      classification: identity.classification,
      reason: identity.reason,
      mappedActionIds: receipt.linkedActionIds.map((wireId) =>
        wireActionIdToLocal.get(compositeKey(receipt.web06PageInstanceId, wireId)) ?? `unmapped:${wireId}`),
      wireEventSequenceId: identity.eventSequenceId,
      wireEventId: identity.eventId,
      wireIdentity: clone(identity),
      wireLinkedActionIds: clone(receipt.linkedActionIds),
      eventTimestamp: identity.timeStamp,
      normalizedEventAt: identity.timeStamp,
      eventDeliveredAt: identity.eventDeliveredAt,
      requestedDriverDispatchAt: driver.requestedDriverDispatchAt,
      actualDriverDispatchAt: driver.actualDriverDispatchAt,
      modifiers: ["ctrlKey", "metaKey", "altKey", "shiftKey"].filter((field) => identity[field]),
    };
  });
  for (let index = 0; index < expectedLifecycleEvents.length; index += 1) {
    const frozen = expectedLifecycleEvents[index];
    const marker = externalLifecycleEvents[index];
    setupAssert(marker?.eventSequenceId === frozen.eventSequenceId && marker?.stepId === frozen.stepId
      && marker?.type === frozen.type && marker?.key === frozen.key && marker?.code === frozen.code
      && marker?.classification === frozen.classification && marker?.reason === frozen.reason
      && marker?.originOwner === "harness-browser-lifecycle" && Array.isArray(marker?.mappedActionIds)
      && marker.mappedActionIds.length === 0,
    "WEB06_PRIVATE_LIFECYCLE_MARKER_INVALID");
    events.push(clone(marker));
  }
  events.sort((left, right) => left.eventSequenceId - right.eventSequenceId);
  const eventByWireSequence = new Map(events.filter((event) => event.wireEventSequenceId !== undefined)
    .map((event) => [compositeKey(event.pageInstanceId, event.wireEventSequenceId), event]));
  let previousTextareaValue = commonSurface.initialDomObserved?.textareaValue ?? "";
  const actionOrdinalsByPage = new Map();
  const actions = wireActions.map((receipt, index) => {
    const identity = receipt.identity;
    const frozen = expected.actions[index] ?? {};
    const localSequence = frozen.sequenceId;
    const sourceWindow = windowForPage(receipt.web06PageInstanceId);
    const ordinal = actionOrdinalsByPage.get(receipt.web06PageInstanceId) ?? 0;
    setupAssert(sourceWindow && identity.sequenceId
      === sourceWindow.receiptWindowStartActionSequenceId + ordinal,
    "WEB06_PROTOCOL_ACTION_SEGMENT_SEQUENCE", String(index + 1));
    actionOrdinalsByPage.set(receipt.web06PageInstanceId, ordinal + 1);
    const localEventSequence = identity.eventSequenceId === undefined
      ? undefined : eventByWireSequence.get(compositeKey(receipt.web06PageInstanceId,
        identity.eventSequenceId))?.eventSequenceId;
    const causedByEventSequenceId = identity.causedByEventSequenceId === undefined
      ? undefined
      : eventByWireSequence.get(compositeKey(receipt.web06PageInstanceId,
        identity.causedByEventSequenceId))?.eventSequenceId;
    const event = eventByWireSequence.get(compositeKey(receipt.web06PageInstanceId,
      identity.eventSequenceId ?? identity.causedByEventSequenceId));
    const presentation = receipt.presentation;
    const lifecycle = receipt.lifecycle;
    const terminal = presentation ?? lifecycle;
    const terminalKind = presentation && lifecycle ? "ambiguous" : presentation ? "presentation" : lifecycle ? "lifecycle" : "missing";
    const worker = receipt.worker ?? {};
    const rawSequence = clone(identity.rawInputSequence);
    const stepCommitments = hasOwn(argumentCommitments, frozen.stepId)
      ? argumentCommitments[frozen.stepId]
      : undefined;
    const commitments = stepCommitments && typeof stepCommitments === "object"
      && hasOwn(stepCommitments, receipt.name)
      ? stepCommitments[receipt.name]
      : {};
    const normalizedArgs = normalizeWireActionArgs(receipt.name, receipt.args, commitments);
    const spans = workerSpanRecord(worker, full);
    const action = {
      actionId: `a${localSequence}`,
      sequenceId: localSequence,
      pageInstanceId: receipt.web06PageInstanceId,
      eventSequenceId: localEventSequence,
      causedByActionId: identity.causedByActionId === undefined
        ? undefined
        : wireActionIdToLocal.get(compositeKey(receipt.web06PageInstanceId, identity.causedByActionId)),
      causedBySequenceId: identity.causedBySequenceId === undefined ? undefined
        : wireActionSequenceToLocal.get(compositeKey(receipt.web06PageInstanceId, identity.causedBySequenceId)),
      causedByEventSequenceId,
      stepId: frozen.stepId,
      kind: receipt.name,
      stressDeadline: frozen.stressDeadline === true,
      args: normalizedArgs,
      classification: identity.actionClass,
      supersedable: identity.supersedable,
      compositionEpochId: identity.compositionEpochId,
      supersessionSubRunId: identity.supersessionSubRunId,
      originKind: identity.originKind,
      originReason: identity.originReason,
      wireSequenceId: identity.sequenceId,
      wireActionId: identity.actionId,
      wireArgs: clone(receipt.args),
      argumentCommitments: clone(commitments),
      wireIdentity: clone(identity),
      returnedWireIdentity: clone(receipt.returnedIdentity),
      driverDispatchAt: event?.actualDriverDispatchAt,
      actionEnqueuedAt: identity.actionEnqueuedAt,
      mainQueueDepth: identity.mainQueueDepthAtEnqueue,
      workerSentAt: identity.workerSentAt,
      workerDispatchDepth: identity.workerDispatchDepth,
      workerMessageReceivedAt: worker.workerMessageReceivedAt,
      workerActionStartedAt: worker.workerActionStartedAt,
      workerFinishedAt: worker.workerFinishedAt,
      mainResponseReceivedAt: receipt.mainResponseReceivedAt,
      responseMappingStartedAt: receipt.responseMappingStartedAt,
      responseMappingFinishedAt: receipt.responseMappingFinishedAt,
      terminalKind,
      stateUpdateScheduledAt: terminal?.stateUpdateScheduledAt,
      stateCommittedAt: presentation?.stateCommittedAt,
      stateAppliedAt: presentation?.stateCommittedAt ?? lifecycle?.terminalObservedAt,
      ...(presentation?.outcome === "painted" ? { paintObservedAt: presentation.paintObservedAt ?? presentation.terminalObservedAt } : {}),
      ...(["committed", "barrier-completed", "failure"].includes(terminal?.outcome)
        ? { terminalObservedAt: terminal?.terminalObservedAt }
        : {}),
      outcome: terminal?.outcome ?? "failure",
      supersededBySequenceId: presentation?.supersededBySequenceId === undefined
        ? undefined
        : wireActionSequenceToLocal.get(compositeKey(receipt.web06PageInstanceId,
          presentation.supersededBySequenceId)),
      rawActionSequence: rawSequence,
      logicalInput: presentation?.presentationExpected?.input ?? "",
      workerSpans: spans,
      presentationExpected: clone(presentation?.presentationExpected),
      domObserved: clone(presentation?.domObserved),
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
    action.engineRawProof = clone(worker.engineRaw);
    action.resultSummary = clone(worker.resultSummary);
    if (full) {
      const rawJson = worker.engineRawJson;
      action.persistenceRan = (worker.persistenceSpans?.length ?? 0) > 0;
      action.engineRaw = rawJson === undefined ? undefined : {
        actionKind: action.kind,
        compositionEpochId: action.compositionEpochId,
        supersessionSubRunId: action.supersessionSubRunId,
        rawActionSequence: clone(rawSequence),
        rawResponseJson: rawJson,
        rawResponseSha256: sha256(Buffer.from(rawJson, "utf8")),
      };
    }
    if (presentation?.outcome === "committed" && presentation.domObserved) {
      action.commitFingerprint = commitFingerprint(presentation, previousTextareaValue);
    }
    if (presentation?.domObserved?.textareaValue !== undefined) previousTextareaValue = presentation.domObserved.textareaValue;
    return action;
  });
  return {
    metricContractVersion: WEB06_METRIC_CONTRACT_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_REGISTRY_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_PREDICATE_VERSION,
    ...clone(metadata),
    protocolWindow: clone(protocolWindow),
    protocolWindowSegments: protocolWindowSegments.length ? clone(protocolWindowSegments) : undefined,
    eventClockProbe: clone(commonSurface.eventClockProbe),
    eventClockSegments: clone(commonSurface.eventClockSegments),
    calibration: clone(commonSurface.calibration),
    calibrationSegments: clone(commonSurface.calibrationSegments),
    events,
    actions,
    auxiliaryEvents: clone(commonSurface.auxiliaryEvents ?? []),
    cadenceGaps: clone(commonSurface.cadenceGaps ?? []),
    idleFrameIntervalsMs: clone(commonSurface.idleFrameIntervalsMs ?? []),
    idleFrameSegments: clone(commonSurface.idleFrameSegments),
    interactionFrameIntervalsMs: clone(commonSurface.interactionFrameIntervalsMs ?? []),
    interactionFrameTimestamps: clone(commonSurface.interactionFrameTimestamps ?? []),
    interactionFrameWindows: clone(commonSurface.interactionFrameWindows ?? []),
    interactionWindows: clone(commonSurface.interactionWindows ?? []),
    idleControlWindows: clone(commonSurface.idleControlWindows ?? []),
    longTaskObserver: clone(commonSurface.longTaskObserver),
    longTasks: clone(commonSurface.longTasks ?? []),
    focusVisibilitySamples: clone(commonSurface.focusVisibilitySamples ?? []),
    assetsRequestedDuringWindow: clone(commonSurface.assetsRequestedDuringWindow ?? []),
    measurementProtocolBlockers: clone(commonSurface.measurementProtocolBlockers ?? []),
    pressureProofs: clone(commonSurface.pressureProofs ?? []),
    burstRecoveries: clone(commonSurface.burstRecoveries ?? []),
    lifecycleContinuity: clone(commonSurface.lifecycleContinuity),
    sentinelOverflowCounts: clone(commonSurface.sentinelOverflowCounts ?? {}),
  };
}

export function makeCommonSurfaceReceipt({ metadata, commonSurface }) {
  return {
    metricContractVersion: WEB06_METRIC_CONTRACT_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_REGISTRY_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_PREDICATE_VERSION,
    ...clone(metadata),
    candidatePageSize: commonSurface.candidatePageSize,
    pageSizeSetup: clone(commonSurface.pageSizeSetup),
    eventClockProbe: clone(commonSurface.eventClockProbe),
    eventClockSegments: clone(commonSurface.eventClockSegments),
    calibration: { driver: clone(commonSurface.calibration?.driver) },
    calibrationSegments: commonSurface.calibrationSegments === undefined ? undefined : {
      preReload: { driver: clone(commonSurface.calibrationSegments.preReload?.driver) },
      postReload: { driver: clone(commonSurface.calibrationSegments.postReload?.driver) },
    },
    events: clone(commonSurface.events),
    auxiliaryEvents: clone(commonSurface.auxiliaryEvents ?? []),
    unmatchedEvents: clone(commonSurface.unmatchedEvents ?? []),
    measurementProtocolBlockers: clone(commonSurface.measurementProtocolBlockers ?? []),
    actions: [],
    commonSamples: resolveCommonSamples({
      scenarioId: metadata.scenarioId,
      events: commonSurface.events,
      snapshots: commonSurface.snapshots,
    }),
    cadenceGaps: clone(commonSurface.cadenceGaps ?? []),
    idleFrameIntervalsMs: clone(commonSurface.idleFrameIntervalsMs ?? []),
    idleFrameSegments: clone(commonSurface.idleFrameSegments),
    interactionFrameIntervalsMs: clone(commonSurface.interactionFrameIntervalsMs ?? []),
    interactionFrameWindows: clone(commonSurface.interactionFrameWindows ?? []),
    interactionWindows: clone(commonSurface.interactionWindows ?? []),
    idleControlWindows: clone(commonSurface.idleControlWindows ?? []),
    longTaskObserver: clone(commonSurface.longTaskObserver),
    longTasks: clone(commonSurface.longTasks ?? []),
    focusVisibilitySamples: clone(commonSurface.focusVisibilitySamples ?? []),
    assetsRequestedDuringWindow: clone(commonSurface.assetsRequestedDuringWindow ?? []),
    interactionFrameTimestamps: clone(commonSurface.interactionFrameTimestamps ?? []),
    lifecycleContinuity: clone(commonSurface.lifecycleContinuity),
    sentinelOverflowCounts: clone(commonSurface.sentinelOverflowCounts ?? {}),
  };
}

export function commonEndpointSequenceDigest(receipt) {
  setupAssert(Array.isArray(receipt?.commonSamples), "WEB06_COMMON_SAMPLE_SEQUENCE_MISSING");
  return digestJson(receipt.commonSamples.map((sample) => ({
    stepId: sample.stepId,
    sampleKind: sample.sampleKind,
    outcome: sample.outcome,
    supersededByStepId: sample.supersededByStepId,
    domFingerprintSha256: sample.domFingerprintSha256,
  })));
}

export function parseAndCompactPrivateReceipt(receipt, rawPacketSha256) {
  const parsed = validateAndRecomputeReceipt(receipt);
  return { parsed, publicReceipt: publicEvidenceReceipt({ receipt, parsed, rawPacketSha256 }) };
}

export function parseAndCompactCommonReceipt(receipt, rawPacketSha256) {
  setupAssert(isSha256(rawPacketSha256), "WEB06_RAW_PACKET_HASH_INVALID");
  const parsed = validateCommonSurfaceReceipt(receipt);
  const compact = {
    version: "web06-public-common-receipt-v1",
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
    commonSampleCount: receipt.commonSamples.length,
    thresholdViolations: [...parsed.thresholdViolations],
    behaviorErrorCodes: [...parsed.behaviorErrors],
    setupErrorCodes: [...parsed.setupErrors],
    rawPacketSha256,
    roundSummary: buildRoundEvidenceSummary(receipt, { surface: "common" }),
  };
  const schema = validatePublicEvidenceSchema(compact);
  setupAssert(schema.pass, "WEB06_PUBLIC_EVIDENCE_SCHEMA", schema.errors.join(","));
  return { parsed, publicReceipt: Object.freeze(compact) };
}

/** Projection is used only for raw-key ownership; the exact rendered bytes remain hashed. */
export function projectWeb06RenderedInput(renderedInput) {
  setupAssert(typeof renderedInput === "string", "WEB06_RENDERED_INPUT_INVALID");
  return renderedInput.replaceAll(" ", "");
}

export function web06DomFingerprintDigest(domObserved) {
  return digestJson(domObserved);
}

export const WEB06_SENTINEL_CAPACITIES = Object.freeze({
  events: 512,
  auxiliaryEvents: 64,
  unmatchedEvents: 64,
  snapshots: 512,
  frameTimestamps: 20_000,
  longTasks: 512,
  focus: 64,
  callbacks: 8_192,
  windows: 8,
  idleControls: 8,
  driverDispatchBindings: 512,
  pendingCaptures: 16,
});

/** Pure mirror of the sentinel capture-epoch filter for owning mutation tests. */
export function selectWeb06LongTaskEntries(entries, captureEpochStartedAt) {
  setupAssert(Array.isArray(entries) && Number.isFinite(captureEpochStartedAt),
    "WEB06_LONG_TASK_CAPTURE_EPOCH_INVALID");
  return entries.filter((entry) => Number.isFinite(entry?.startTime) && Number.isFinite(entry?.duration)
    && entry.duration >= 0 && entry.startTime >= captureEpochStartedAt)
    .map((entry) => ({ startTime: entry.startTime, durationMs: entry.duration }));
}

/**
 * External page sentinel. Pass this function itself to context.addInitScript;
 * it reads only DOM/browser APIs and never calls the private action protocol.
 */
export function installWeb06Sentinel() {
  if (window.__YUNE_WEB06_SENTINEL__ !== undefined) return;
  const eventTypes = ["keydown", "keyup", "blur", "change", "click", "submit", "mousedown", "touchstart"];
  const pageInstanceId = crypto.randomUUID();
  const capacities = {
    events: 512,
    auxiliaryEvents: 64,
    unmatchedEvents: 64,
    snapshots: 512,
    frameTimestamps: 20000,
    longTasks: 512,
    focus: 64,
    callbacks: 8192,
    windows: 8,
    idleControls: 8,
    driverDispatchBindings: 512,
    pendingCaptures: 16,
  };
  const state = {
    active: false,
    arm: undefined,
    events: [],
    auxiliaryEvents: [],
    unmatchedEvents: [],
    snapshots: [],
    frameTimestamps: [],
    longTasks: [],
    focus: [],
    callbackLedger: [],
    callbackLedgerOverflowCount: 0,
    pendingCaptures: 0,
    windows: [],
    idleControls: [],
    resourceBaseline: 0,
    driverDispatchByStep: new Map(),
    installedAt: performance.now(),
    captureEpochStartedAt: Infinity,
    overflowCounts: Object.fromEntries(Object.keys(capacities).map((name) => [name, 0])),
  };
  const pushBounded = (name, rows, value) => {
    if (rows.length >= capacities[name]) {
      state.overflowCounts[name] += 1;
      return undefined;
    }
    rows.push(value);
    return rows.length - 1;
  };
  const normalizedText = (element) => element?.textContent?.trim() ?? "";
  const projectRenderedInput = (value) => value.replaceAll(" ", "");
  const domFingerprint = () => {
    const panel = document.querySelector(".candidate-panel");
    const textarea = document.querySelector("textarea");
    const pageButtons = [...(panel?.querySelectorAll(".candidate-nav .page-nav") ?? [])];
    const rows = [...document.querySelectorAll(".candidate-panel .candidate-row")];
    const renderedInput = normalizedText(panel?.querySelector(".candidate-preedit"));
    return {
      input: renderedInput,
      renderedInput,
      logicalInputProjection: projectRenderedInput(renderedInput),
      candidates: rows.map((row) => ({
        label: normalizedText(row.querySelector(".candidate-index")),
        text: normalizedText(row.querySelector(".candidate-text")),
        comment: normalizedText(row.querySelector(".candidate-note")),
        source: row.dataset.source ?? "",
      })),
      pageShape: {
        previousDisabled: pageButtons[0]?.disabled ?? true,
        nextDisabled: pageButtons[1]?.disabled ?? true,
        highlightedIndex: rows.findIndex((row) => row.classList.contains("highlighted")),
        visibleCount: rows.length,
      },
      textareaValue: textarea?.value ?? "",
      selectionStart: textarea?.selectionStart ?? 0,
      selectionEnd: textarea?.selectionEnd ?? 0,
    };
  };
  const focusSample = (role = "in-window-event", windowId) => ({
    recordedAt: performance.now(),
    focused: document.hasFocus(),
    visibilityState: document.visibilityState,
    role,
    windowId,
    pageInstanceId,
  });
  const latestEventIndex = (stepId) => {
    for (let index = state.events.length - 1; index >= 0; index -= 1) {
      if (stepId === undefined || state.events[index].stepId === stepId) return index;
    }
    return undefined;
  };
  const recordCallback = (kind, startedAt, eventIndex) => {
    let entry;
    if (state.active) {
      entry = {
        kind,
        pageInstanceId,
        startedAt,
        finishedAt: startedAt,
        durationMs: 0,
        eventSequenceId: eventIndex === undefined ? undefined : eventIndex + 1,
      };
      if (pushBounded("callbacks", state.callbackLedger, entry) === undefined) state.callbackLedgerOverflowCount += 1;
    }
    let finishedAt = performance.now();
    // Include the ledger insertion and per-event accounting itself.  The final
    // timestamp/update is the only irreducible measurement write.
    const accountingFinishedAt = performance.now();
    finishedAt = accountingFinishedAt;
    const elapsed = finishedAt - startedAt;
    if (eventIndex !== undefined && state.events[eventIndex]) {
      state.events[eventIndex].sentinelTotalMs += elapsed;
    }
    if (entry) {
      entry.finishedAt = finishedAt;
      entry.durationMs = elapsed;
    }
    return elapsed;
  };
  const captureEndpoint = (stepId) => new Promise((resolve) => {
    if (state.pendingCaptures >= capacities.pendingCaptures) {
      state.overflowCounts.pendingCaptures += 1;
      resolve(undefined);
      return;
    }
    state.pendingCaptures += 1;
    requestAnimationFrame(() => {
      const firstRafStartedAt = performance.now();
      const eventIndex = latestEventIndex(stepId);
      const firstDomObserved = domFingerprint();
      const firstFingerprintJson = JSON.stringify(firstDomObserved);
      requestAnimationFrame(() => {
        const secondRafStartedAt = performance.now();
        const secondDomObserved = domFingerprint();
        const secondFingerprintJson = JSON.stringify(secondDomObserved);
        const snapshot = {
          stepId,
          pageInstanceId,
          observedAt: performance.now(),
          stableDoubleRaf: firstFingerprintJson === secondFingerprintJson,
          firstDomObserved,
          domObserved: secondDomObserved,
        };
        if (state.active) pushBounded("snapshots", state.snapshots, snapshot);
        recordCallback("endpoint-second-raf", secondRafStartedAt, eventIndex);
        state.pendingCaptures -= 1;
        resolve(snapshot);
      });
      recordCallback("endpoint-first-raf", firstRafStartedAt, eventIndex);
    });
  });
  const scheduleEndpoint = (stepId) => {
    void captureEndpoint(stepId);
  };
  const recordEvent = (event) => {
    if (!state.active) return;
    const callbackStartedAt = performance.now();
    const arm = state.arm?.queue?.[0] ?? state.arm;
    if (arm?.precursorTypes?.includes(event.type)) {
      pushBounded("auxiliaryEvents", state.auxiliaryEvents, {
        stepId: arm.stepId,
        pageInstanceId,
        type: event.type,
        key: "key" in event ? event.key : "",
        code: "code" in event ? event.code : "",
        eventTimestamp: event.timeStamp,
        sentinelObservedAt: callbackStartedAt,
      });
      arm.precursorTypes.splice(arm.precursorTypes.indexOf(event.type), 1);
      const callbackLedgerIndex = state.callbackLedger.length;
      const elapsed = recordCallback("auxiliary-event", callbackStartedAt, undefined);
      const callback = state.callbackLedger[callbackLedgerIndex];
      if (callback) {
        arm.precursorCallbacks = arm.precursorCallbacks ?? [];
        arm.precursorCallbacks.push(callback);
      }
      arm.precursorSelfMs = (arm.precursorSelfMs ?? 0) + elapsed;
      return;
    }
    if (!arm || !arm.expectedTypes.includes(event.type)) {
      pushBounded("unmatchedEvents", state.unmatchedEvents,
        { pageInstanceId, type: event.type, timeStamp: event.timeStamp, observedAt: callbackStartedAt });
      recordCallback("unmatched-event", callbackStartedAt, undefined);
      return;
    }
    const eventDeliveredAt = performance.now();
    const precursorSelfMs = arm.precursorSelfMs ?? 0;
    const eventIndex = pushBounded("events", state.events, {
      stepId: arm.stepId,
      pageInstanceId,
      type: event.type,
      key: "key" in event ? event.key : "",
      code: "code" in event ? event.code : "",
      eventTimestamp: event.timeStamp,
      normalizedEventAt: event.timeStamp,
      sentinelObservedAt: eventDeliveredAt,
      requestedDriverDispatchAt: state.driverDispatchByStep.get(arm.stepId)?.requestedDriverDispatchAt,
      actualDriverDispatchAt: state.driverDispatchByStep.get(arm.stepId)?.actualDriverDispatchAt,
      modifiers: ["ctrlKey", "metaKey", "altKey", "shiftKey"].filter((field) => event[field] === true),
      sentinelTotalMs: precursorSelfMs,
    });
    if (eventIndex !== undefined) {
      for (const callback of arm.precursorCallbacks ?? []) {
        callback.eventSequenceId = eventIndex + 1;
      }
      arm.precursorCallbacks = [];
      arm.precursorSelfMs = 0;
    }
    arm.expectedTypes.splice(arm.expectedTypes.indexOf(event.type), 1);
    if (arm.expectedTypes.length === 0) {
      if (state.arm?.queue) {
        state.arm.queue.shift();
        if (state.arm.queue.length === 0) state.arm = undefined;
      } else {
        state.arm = undefined;
      }
    }
    recordCallback("dom-event", callbackStartedAt, eventIndex);
  };
  for (const type of eventTypes) document.addEventListener(type, recordEvent, { capture: true, passive: true });
  const mutationObserver = new MutationObserver(() => {
    const startedAt = performance.now();
    const eventIndex = latestEventIndex();
    recordCallback("mutation", startedAt, eventIndex);
  });
  mutationObserver.observe(document, { subtree: true, childList: true, characterData: true, attributes: true });
  const supported = typeof PerformanceObserver === "function"
    && Array.isArray(PerformanceObserver.supportedEntryTypes)
    && PerformanceObserver.supportedEntryTypes.includes("longtask");
  let longTaskObserver;
  const ingestLongTaskEntries = (entries) => {
    for (const entry of entries) {
      // A buffered entry created before the current capture epoch is setup
      // activity, even when its observer callback arrives after reset.
      if (entry.startTime < state.captureEpochStartedAt) continue;
      pushBounded("longTasks", state.longTasks, { startTime: entry.startTime, durationMs: entry.duration });
    }
  };
  const drainLongTaskRecords = () => {
    if (longTaskObserver) ingestLongTaskEntries(longTaskObserver.takeRecords());
  };
  if (supported) {
    longTaskObserver = new PerformanceObserver((list) => {
      const startedAt = performance.now();
      ingestLongTaskEntries(list.getEntries());
      recordCallback("long-task-observer", startedAt, latestEventIndex());
    });
    longTaskObserver.observe({ type: "longtask", buffered: true });
  }
  const frame = (timestamp) => {
    const startedAt = performance.now();
    pushBounded("frameTimestamps", state.frameTimestamps, timestamp);
    requestAnimationFrame(frame);
    if (state.active) recordCallback("frame-observer", startedAt, latestEventIndex());
  };
  requestAnimationFrame(frame);
  const recordFocus = () => {
    const startedAt = performance.now();
    pushBounded("focus", state.focus, focusSample());
    recordCallback("focus-visibility", startedAt, latestEventIndex());
  };
  document.addEventListener("visibilitychange", recordFocus, { passive: true });
  window.addEventListener("focus", recordFocus, { passive: true });
  window.addEventListener("blur", recordFocus, { passive: true });
  window.__YUNE_WEB06_SENTINEL__ = {
    version: "web06-external-sentinel-v1",
    status() {
      return structuredClone({
        active: state.active,
        armPending: state.arm !== undefined,
        pendingCaptures: state.pendingCaptures,
        completedWindowCount: state.windows.filter((window) => Number.isFinite(window.endedAt)).length,
        snapshotCount: state.snapshots.length,
        eventCount: state.events.length,
        overflowCounts: state.overflowCounts,
      });
    },
    reset() {
      drainLongTaskRecords();
      state.active = false;
      state.arm = undefined;
      state.events.length = 0;
      state.auxiliaryEvents.length = 0;
      state.unmatchedEvents.length = 0;
      state.snapshots.length = 0;
      state.longTasks.length = 0;
      state.focus.length = 0;
      state.callbackLedger.length = 0;
      state.callbackLedgerOverflowCount = 0;
      state.pendingCaptures = 0;
      state.windows.length = 0;
      state.idleControls.length = 0;
      state.frameTimestamps.length = 0;
      state.driverDispatchByStep.clear();
      for (const name of Object.keys(state.overflowCounts)) state.overflowCounts[name] = 0;
      state.resourceBaseline = performance.getEntriesByType("resource").length;
      state.captureEpochStartedAt = performance.now();
    },
    armStep(value) {
      if (!state.active || state.arm !== undefined) throw new Error("WEB06_SENTINEL_ARM_OVERLAP");
      state.arm = {
        ...value,
        expectedTypes: [...value.expectedTypes],
        precursorTypes: [...(value.precursorTypes ?? [])],
      };
    },
    armSteps(values) {
      if (!state.active || state.arm !== undefined || !Array.isArray(values) || values.length < 2) {
        throw new Error("WEB06_SENTINEL_BATCH_ARM_INVALID");
      }
      state.arm = {
        queue: values.map((value) => ({
          ...value,
          expectedTypes: [...value.expectedTypes],
          precursorTypes: [...(value.precursorTypes ?? [])],
        })),
      };
    },
    bindDriverDispatch(stepId, requestedDriverDispatchAt, actualDriverDispatchAt) {
      if (!Number.isFinite(requestedDriverDispatchAt) || !Number.isFinite(actualDriverDispatchAt)
        || actualDriverDispatchAt < requestedDriverDispatchAt) {
        throw new Error("WEB06_SENTINEL_DRIVER_DISPATCH_INVALID");
      }
      if (!state.driverDispatchByStep.has(stepId)
        && state.driverDispatchByStep.size >= capacities.driverDispatchBindings) {
        state.overflowCounts.driverDispatchBindings += 1;
      } else {
        state.driverDispatchByStep.set(stepId, { requestedDriverDispatchAt, actualDriverDispatchAt });
      }
      for (const event of state.events) {
        if (event.stepId === stepId) {
          event.requestedDriverDispatchAt = requestedDriverDispatchAt;
          event.actualDriverDispatchAt = actualDriverDispatchAt;
        }
      }
    },
    scheduleEndpoint,
    captureEndpoint,
    async flushEndpoints() {
      while (state.pendingCaptures > 0) {
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
      return state.pendingCaptures;
    },
    latestSnapshot(stepId) {
      const snapshot = [...state.snapshots].reverse().find((candidate) => candidate.stepId === stepId);
      return snapshot === undefined ? undefined : structuredClone(snapshot);
    },
    async takeIdleIntervals(count) {
      const start = state.frameTimestamps.length;
      while (state.frameTimestamps.length - start < count + 1) {
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
      const values = state.frameTimestamps.slice(start, start + count + 1);
      return values.slice(1).map((value, index) => value - values[index]);
    },
    async startWindow(label) {
      if (state.active) throw new Error("WEB06_SENTINEL_WINDOW_OVERLAP");
      if (state.windows.length >= capacities.windows) {
        state.overflowCounts.windows += 1;
        throw new Error("WEB06_SENTINEL_WINDOW_CAPACITY");
      }
      const windowId = `${pageInstanceId}-window-${state.windows.length + 1}`;
      const preBoundaryFocus = focusSample("pre-boundary", windowId);
      pushBounded("focus", state.focus, preBoundaryFocus);
      return new Promise((resolve) => requestAnimationFrame((startBoundaryRafAt) => {
        const callbackStartedAt = performance.now();
        state.active = true;
        const window = {
          label,
          windowId,
          pageInstanceId,
          startedAt: startBoundaryRafAt,
          startBoundaryRafAt,
          preBoundaryFocusRecordedAt: preBoundaryFocus.recordedAt,
          frameStartIndex: state.frameTimestamps.length,
        };
        pushBounded("windows", state.windows, window);
        const value = {
          startedAt: startBoundaryRafAt,
          startBoundaryRafAt,
          preBoundaryFocus,
          initialDomObserved: domFingerprint(),
          frameIndex: state.frameTimestamps.length,
        };
        recordCallback("window-start", callbackStartedAt, undefined);
        resolve(value);
      }));
    },
    async endWindow() {
      if (!state.active || state.arm !== undefined) throw new Error("WEB06_SENTINEL_WINDOW_OR_ARM_INVALID");
      while (state.pendingCaptures > 0) {
        await new Promise((resolve) => requestAnimationFrame(resolve));
      }
      return new Promise((resolve) => requestAnimationFrame((endBoundaryRafAt) => {
        const callbackStartedAt = performance.now();
        const window = state.windows.at(-1);
        window.endedAt = endBoundaryRafAt;
        window.endBoundaryRafAt = endBoundaryRafAt;
        window.frameEndIndex = state.frameTimestamps.length;
        recordCallback("window-end", callbackStartedAt, latestEventIndex());
        state.active = false;
        const postBoundaryFocus = focusSample("post-boundary", window.windowId);
        pushBounded("focus", state.focus, postBoundaryFocus);
        window.postBoundaryFocusRecordedAt = postBoundaryFocus.recordedAt;
        resolve({ endedAt: endBoundaryRafAt, endBoundaryRafAt, postBoundaryFocus });
      }));
    },
    async captureIdleControl(label, durationMs) {
      if (state.active || !Number.isFinite(durationMs) || durationMs <= 0) {
        throw new Error("WEB06_SENTINEL_IDLE_CONTROL_INVALID");
      }
      if (state.idleControls.length >= capacities.idleControls) {
        state.overflowCounts.idleControls += 1;
        throw new Error("WEB06_SENTINEL_IDLE_CONTROL_CAPACITY");
      }
      return new Promise((resolve) => requestAnimationFrame((startedAt) => {
        const endedAt = startedAt + durationMs;
        const control = {
          label,
          controlId: `${pageInstanceId}-idle-control-${state.idleControls.length + 1}`,
          pageInstanceId,
          startedAt,
          endedAt,
        };
        pushBounded("idleControls", state.idleControls, control);
        const wait = (timestamp) => {
          if (timestamp >= endedAt) {
            requestAnimationFrame(() => resolve(structuredClone(control)));
          } else requestAnimationFrame(wait);
        };
        requestAnimationFrame(wait);
      }));
    },
    probeEventClock() {
      let eventTimestamp;
      const listener = (event) => { eventTimestamp = event.timeStamp; };
      document.addEventListener("web06-clock-probe", listener, { once: true });
      const beforeDispatchAt = performance.now();
      document.dispatchEvent(new Event("web06-clock-probe"));
      const afterDispatchAt = performance.now();
      return { pageInstanceId, beforeDispatchAt, eventTimestamp, afterDispatchAt };
    },
    snapshot() {
      drainLongTaskRecords();
      const completedWindows = state.windows.filter((window) => Number.isFinite(window.endedAt));
      const frameWindows = completedWindows.map((window) => {
        const timestamps = state.frameTimestamps.filter((value) => value >= window.startedAt && value <= window.endedAt);
        if (timestamps[0] !== window.startedAt) timestamps.unshift(window.startedAt);
        if (timestamps.at(-1) !== window.endedAt) timestamps.push(window.endedAt);
        return {
          windowId: window.windowId,
          pageInstanceId: window.pageInstanceId,
          timestamps,
          intervalsMs: timestamps.slice(1).map((value, index) => value - timestamps[index]),
        };
      });
      const frameTimes = frameWindows.flatMap((window) => window.timestamps);
      const frameIntervals = frameWindows.flatMap((window) => window.intervalsMs);
      const overlapsAnyWindow = (startTime, durationMs = 0) => completedWindows.some((window) =>
        startTime < window.endedAt && startTime + durationMs > window.startedAt);
      const overlapsAnyIdleControl = (startTime, durationMs = 0) => state.idleControls.some((control) =>
        startTime < control.endedAt && startTime + durationMs > control.startedAt);
      return structuredClone({
        events: state.events.map((event, index) => ({ ...event, eventSequenceId: index + 1 })),
        auxiliaryEvents: state.auxiliaryEvents,
        unmatchedEvents: state.unmatchedEvents,
        snapshots: state.snapshots,
        interactionWindows: completedWindows.map(({ label, frameStartIndex: _start, frameEndIndex: _end, ...window }) => window),
        idleControlWindows: state.idleControls,
        interactionFrameWindows: frameWindows,
        interactionFrameTimestamps: frameTimes,
        interactionFrameIntervalsMs: frameIntervals,
        longTaskObserver: { pageInstanceId, supported, installedAt: state.installedAt },
        longTasks: state.longTasks.map((task) => ({
          ...task,
          pageInstanceId,
          overlapsInteractionWindow: overlapsAnyWindow(task.startTime, task.durationMs),
          overlapsIdleControl: overlapsAnyIdleControl(task.startTime, task.durationMs),
        })),
        focusVisibilitySamples: state.focus,
        assetsRequestedDuringWindow: performance.getEntriesByType("resource")
          .slice(state.resourceBaseline)
          .filter((entry) => overlapsAnyWindow(entry.startTime, entry.duration))
          .map((entry) => ({ name: new URL(entry.name).pathname, startTime: entry.startTime })),
        callbackLedger: state.callbackLedger,
        callbackLedgerCapacity: capacities.callbacks,
        callbackLedgerOverflowCount: state.callbackLedgerOverflowCount,
        sentinelOverflowCounts: state.overflowCounts,
        sentinelCallbacksMs: state.callbackLedger.map((entry) => entry.durationMs),
        unattributedInWindowCallbacksMs: state.callbackLedger
          .filter((entry) => entry.eventSequenceId === undefined)
          .map((entry) => entry.durationMs),
        sentinelTotalPerEventMs: state.events.map((_event, index) => state.callbackLedger
          .filter((entry) => entry.eventSequenceId === index + 1)
          .reduce((sum, entry) => sum + entry.durationMs, 0)),
        sentinelTotalPerWindowMs: completedWindows.map((window) => state.callbackLedger
          .filter((entry) => entry.startedAt >= window.startedAt && entry.startedAt <= window.endedAt)
          .reduce((sum, entry) => sum + entry.durationMs, 0)),
        sentinelAccountedCallbackCount: state.callbackLedger.length,
      });
    },
  };
}
