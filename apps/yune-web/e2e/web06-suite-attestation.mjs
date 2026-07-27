import { createHash } from "node:crypto";
import { constants as fsConstants } from "node:fs";
import { lstat, open, readFile, realpath } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";

import {
  WEB06_BINDING_SCENARIO_ORDER,
  WEB06_EXTENDED_SCENARIO_RUNS,
  WEB06_PREVIEW_SCENARIOS as METRIC_PREVIEW_SCENARIOS,
  WEB06_RUNNER_TOOLING_PATHS as METRIC_RUNNER_TOOLING_PATHS,
  validateWeb06RunArtifactSchema,
} from "./web06-collector.mjs";
import {
  validateIndependentRecomputeSchema,
  verifyCollectorOutput,
} from "./web06-independent-verifier.mjs";
import {
  WEB06_BEHAVIOR_PREDICATE_VERSION,
  WEB06_METRIC_CONTRACT_VERSION,
  WEB06_OBSERVER_COUNTERBALANCE as METRIC_OBSERVER_COUNTERBALANCE,
  WEB06_SCENARIO_REGISTRY_VERSION,
} from "./web06-metric-contract.mjs";

export const WEB06_METRIC_VERSION = WEB06_METRIC_CONTRACT_VERSION;
export const WEB06_SCENARIO_VERSION = WEB06_SCENARIO_REGISTRY_VERSION;
export const WEB06_BEHAVIOR_VERSION = WEB06_BEHAVIOR_PREDICATE_VERSION;
export const WEB06_RUNNER_TOOLING_PATHS = METRIC_RUNNER_TOOLING_PATHS;
export const WEB06_PREVIEW_SCENARIOS = METRIC_PREVIEW_SCENARIOS;
export const WEB06_OBSERVER_COUNTERBALANCE = Object.freeze(
  Object.values(METRIC_OBSERVER_COUNTERBALANCE),
);
export const WEB06_BINDING_SCENARIOS = WEB06_BINDING_SCENARIO_ORDER;
export const WEB06_BRANCH_B_SCENARIOS = WEB06_EXTENDED_SCENARIO_RUNS;

const EXPECTATION_CONTRACTS = Object.freeze({
  OBSERVER: Object.freeze({
    mode: "PRODUCT/off+BASE/minimal+BASE/full",
    scenarios: Object.freeze(["rapid-long-jyutping"]),
    rounds: 5,
    minimumAttempts: 5,
    maximumAttempts: 7,
    selectedBranches: Object.freeze(["NONE"]),
    dispositions: Object.freeze(["DIAGNOSTIC"]),
  }),
  BASELINE: Object.freeze({
    mode: "BASE/full",
    scenarios: WEB06_BINDING_SCENARIOS,
    rounds: 5,
    minimumAttempts: 5,
    maximumAttempts: 7,
    selectedBranches: Object.freeze(["NONE"]),
    dispositions: Object.freeze(["SOURCE_CURRENT_BASELINE"]),
  }),
  FINAL: Object.freeze({
    mode: "FINAL/full",
    scenarios: WEB06_BINDING_SCENARIOS,
    rounds: 5,
    minimumAttempts: 5,
    maximumAttempts: 7,
    selectedBranches: Object.freeze(["A", "B", "C", "NONE"]),
    dispositions: Object.freeze(["PRODUCTION_REDUCTION", "MEASURED_NO_GO"]),
  }),
  PREVIEW: Object.freeze({
    mode: "FINAL/default-minimal",
    scenarios: WEB06_PREVIEW_SCENARIOS,
    rounds: 1,
    minimumAttempts: 1,
    maximumAttempts: 1,
    selectedBranches: Object.freeze(["A", "B", "C"]),
    dispositions: Object.freeze(["PRODUCTION_REDUCTION"]),
  }),
});

const SHA40 = /^[0-9a-f]{40}$/;
const SHA64 = /^[0-9a-f]{64}$/;
const MEASURED = new Set(["PASS", "RED"]);
const ATTEMPT_CLASSIFICATIONS = new Set([
  "PASS",
  "RED",
  "SETUP_INVALID",
  "NO_VERDICT_INVALID_CADENCE",
]);
const OBSERVER_CLASSIFICATIONS = new Set(["PASS", "RED", "SETUP_INVALID"]);
const OVERHEAD_LIMITS = Object.freeze({ medianMs: 1, p95Ms: 2, maxMs: 4 });

export const sha256Bytes = (bytes) =>
  createHash("sha256").update(bytes).digest("hex");

export function stableJsonValue(value) {
  if (Array.isArray(value)) return value.map(stableJsonValue);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.keys(value)
      .sort()
      .map((key) => [key, stableJsonValue(value[key])]),
  );
}

export const sha256StableJson = (value) =>
  sha256Bytes(Buffer.from(JSON.stringify(stableJsonValue(value)), "utf8"));

export async function createRunnerSourceManifest(
  repositoryRoot,
  sourceCommit,
  sourceTree,
) {
  assert(SHA40.test(sourceCommit ?? ""), "WEB06 runner source commit is invalid");
  assert(SHA40.test(sourceTree ?? ""), "WEB06 runner source tree is invalid");
  const canonicalRoot = await realpath(path.resolve(repositoryRoot));
  const files = [];
  for (const relativePath of WEB06_RUNNER_TOOLING_PATHS) {
    const candidate = path.join(canonicalRoot, ...relativePath.split("/"));
    const [metadata, canonicalCandidate] = await Promise.all([
      lstat(candidate),
      realpath(candidate),
    ]);
    assert(
      metadata.isFile() && !metadata.isSymbolicLink(),
      `WEB06 runner tooling must be a plain file: ${relativePath}`,
    );
    assert(
      canonicalCandidate === candidate,
      `WEB06 runner tooling path contains a symlink: ${relativePath}`,
    );
    files.push(Object.freeze({
      path: relativePath,
      sha256: sha256Bytes(await readFile(candidate)),
    }));
  }
  const tooling = Object.freeze({
    version: "web06-runner-tooling-v1",
    files: Object.freeze(files),
  });
  return Object.freeze({
    version: "web06-runner-source-v1",
    sourceCommit,
    sourceTree,
    sourceTreeState: "clean",
    tooling,
    toolingManifestSha256: sha256Bytes(Buffer.from(JSON.stringify(tooling))),
  });
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function exactArray(actual, expected, label) {
  assert(
    Array.isArray(actual) && JSON.stringify(actual) === JSON.stringify(expected),
    `${label} does not match the frozen WEB06 matrix`,
  );
}

function expectedScenarios(expectation, selectedBranch) {
  const contract = EXPECTATION_CONTRACTS[expectation];
  if (expectation === "FINAL" && selectedBranch === "B") {
    return [...contract.scenarios, ...WEB06_BRANCH_B_SCENARIOS];
  }
  return [...contract.scenarios];
}

function validateDisposition(payload, expectation) {
  const contract = EXPECTATION_CONTRACTS[expectation];
  assert(
    contract.dispositions.includes(payload.disposition),
    "WEB06 suite disposition is invalid for this expectation",
  );
  if (expectation === "FINAL") {
    if (payload.selectedBranch === "NONE") {
      assert(
        payload.disposition === "MEASURED_NO_GO",
        "WEB06 FINAL selectedBranch NONE requires MEASURED_NO_GO",
      );
    } else {
      assert(
        payload.disposition === "PRODUCTION_REDUCTION",
        "WEB06 FINAL production branch requires PRODUCTION_REDUCTION",
      );
    }
  }
}

function validateEvidenceReference(reference, expectedName, label) {
  assert(reference && typeof reference === "object" && !Array.isArray(reference), `${label} reference is missing`);
  assert(
    typeof reference.relativePath === "string" &&
      reference.relativePath !== "" &&
      !reference.relativePath.includes("\\") &&
      !path.posix.isAbsolute(reference.relativePath) &&
      reference.relativePath.split("/").every((part) => part !== "" && part !== "." && part !== ".."),
    `${label} path is unsafe`,
  );
  assert(reference.relativePath === expectedName, `${label} path is not canonical`);
  assert(Number.isSafeInteger(reference.bytes) && reference.bytes > 0, `${label} byte count is invalid`);
  assert(SHA64.test(reference.sha256 ?? ""), `${label} hash is invalid`);
}

function validateExecution(payload, scenarioCount) {
  const execution = payload.execution;
  assert(execution?.plannedScenarioCount === scenarioCount, "WEB06 planned scenario count is wrong");
  assert(execution?.executedScenarioCount === scenarioCount, "WEB06 executed scenario count is wrong");
  assert(execution?.skippedScenarioCount === 0, "WEB06 suite contains skipped scenarios");
  assert(execution?.unexpectedScenarioCount === 0, "WEB06 suite contains unexpected scenarios");
  assert(execution?.status === "completed", "WEB06 collector suite did not complete");
}

function finiteNonnegative(value, label) {
  assert(Number.isFinite(value) && value >= 0, `${label} must be finite and nonnegative`);
}

function validateObserverComparison(comparison, label) {
  assert(comparison && typeof comparison === "object", `${label} comparison is missing`);
  for (const side of ["left", "right"]) {
    for (const metric of Object.keys(OVERHEAD_LIMITS)) {
      finiteNonnegative(comparison[side]?.[metric], `${label} ${side} ${metric}`);
    }
  }
  let red = false;
  for (const [metric, limit] of Object.entries(OVERHEAD_LIMITS)) {
    const computed = comparison.right[metric] - comparison.left[metric];
    assert(
      Number.isFinite(comparison.deltaMs?.[metric]) &&
        Math.abs(comparison.deltaMs[metric] - computed) <= 1e-9,
      `${label} ${metric} delta was not recomputed from the retained summaries`,
    );
    if (Math.abs(computed) > limit) red = true;
  }
  assert(
    comparison.verdict === (red ? "RED" : "PASS"),
    `${label} comparison verdict does not match the frozen 1/2/4 ms ceilings`,
  );
  return red;
}

function validateObserverTriplet(attempt, index, expectedValidSlot, allContexts) {
  assert(attempt?.attemptId === `triplet-attempt-${index + 1}`, "WEB06 observer triplet IDs must be contiguous");
  assert(OBSERVER_CLASSIFICATIONS.has(attempt.classification), `${attempt.attemptId} has an invalid observer classification`);
  assert(attempt.validSlot === expectedValidSlot, `${attempt.attemptId} did not retain/repeat the expected valid slot`);
  assert(attempt.counterbalanceSlot === expectedValidSlot, `${attempt.attemptId} counterbalance slot is wrong`);
  exactArray(
    attempt.modeOrder,
    WEB06_OBSERVER_COUNTERBALANCE[expectedValidSlot - 1],
    `${attempt.attemptId} counterbalance order`,
  );
  assert(attempt.modeRuns && typeof attempt.modeRuns === "object" && !Array.isArray(attempt.modeRuns), `${attempt.attemptId} must retain three named mode runs`);
  exactArray(Object.keys(attempt.modeRuns), ["PRODUCT", "BASE_MINIMAL", "BASE_FULL"], `${attempt.attemptId} mode run identities`);
  for (const targetId of attempt.modeOrder) {
    const run = attempt.modeRuns[targetId];
    assert(run.targetId === targetId, `${attempt.attemptId}/${targetId} target identity mismatch`);
    assert(run.attemptId === attempt.attemptId, `${attempt.attemptId}/${run.targetId} attempt identity mismatch`);
    assert(
      typeof run.freshContextId === "string" && /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(run.freshContextId),
      `${attempt.attemptId}/${run.targetId} fresh context identity is invalid`,
    );
    assert(!allContexts.has(run.freshContextId), "WEB06 observer reused a browser context");
    allContexts.add(run.freshContextId);
    assert(SHA64.test(run.commonVerdictFingerprint ?? ""), `${attempt.attemptId}/${run.targetId} common verdict fingerprint is invalid`);
    assert(SHA64.test(run.commonCountFingerprint ?? ""), `${attempt.attemptId}/${run.targetId} common count fingerprint is invalid`);
  }
  if (attempt.classification === "SETUP_INVALID") {
    assert(attempt.completeTripletValid === false, `${attempt.attemptId} setup failure is marked valid`);
    assert(attempt.measurementStarted === false, `${attempt.attemptId} setup failure started measurement`);
    assert(attempt.validRedObserved === false, `${attempt.attemptId} setup failure hides a valid red`);
    assert(/^SETUP_[A-Z0-9_]+$/.test(attempt.setupFailureCode ?? ""), `${attempt.attemptId} setup failure code is invalid`);
    return false;
  }
  assert(attempt.completeTripletValid === true, `${attempt.attemptId} measured triplet is not marked valid`);
  assert(attempt.measurementStarted === true, `${attempt.attemptId} measured triplet never started`);
  assert(attempt.measurementCompleted === true, `${attempt.attemptId} measured triplet did not complete`);
  assert(
    Object.values(attempt.modeRuns).every((run) => run.commonVerdictFingerprint === attempt.modeRuns.PRODUCT.commonVerdictFingerprint),
    `${attempt.attemptId} common-surface verdicts disagree`,
  );
  assert(
    Object.values(attempt.modeRuns).every((run) => run.commonCountFingerprint === attempt.modeRuns.PRODUCT.commonCountFingerprint),
    `${attempt.attemptId} common-surface counts disagree`,
  );
  const minimal = attempt.modeRuns.BASE_MINIMAL;
  const full = attempt.modeRuns.BASE_FULL;
  assert(
    SHA64.test(minimal?.internalVerdictFingerprint ?? "") &&
      minimal.internalVerdictFingerprint === full?.internalVerdictFingerprint,
    `${attempt.attemptId} minimal/full internal verdicts disagree`,
  );
  assert(
    SHA64.test(minimal?.internalCountFingerprint ?? "") &&
      minimal.internalCountFingerprint === full?.internalCountFingerprint,
    `${attempt.attemptId} minimal/full internal counts disagree`,
  );
  finiteNonnegative(attempt.callbackMaxima?.sentinelCallbackSelfMs, `${attempt.attemptId} sentinel callback max`);
  finiteNonnegative(attempt.callbackMaxima?.sentinelTotalPerEventMs, `${attempt.attemptId} sentinel per-event total max`);
  finiteNonnegative(attempt.callbackMaxima?.mainCollectorCallbackMs, `${attempt.attemptId} main collector callback max`);
  finiteNonnegative(attempt.callbackMaxima?.workerCollectorCallbackMs, `${attempt.attemptId} worker collector callback max`);
  assert(Number.isSafeInteger(attempt.inWindowInstrumentationLongTaskCount) && attempt.inWindowInstrumentationLongTaskCount >= 0, `${attempt.attemptId} instrumentation Long Task count is invalid`);
  const overheadRed =
    validateObserverComparison(attempt.comparisons?.productVsMinimal, `${attempt.attemptId} PRODUCT/minimal`) ||
    validateObserverComparison(attempt.comparisons?.minimalVsFull, `${attempt.attemptId} minimal/full`);
  const callbackRed =
    attempt.callbackMaxima.sentinelCallbackSelfMs >= 0.5 ||
    attempt.callbackMaxima.sentinelTotalPerEventMs >= 1 ||
    attempt.callbackMaxima.mainCollectorCallbackMs >= 5 ||
    attempt.callbackMaxima.workerCollectorCallbackMs >= 5 ||
    attempt.inWindowInstrumentationLongTaskCount !== 0;
  const computed = overheadRed || callbackRed ? "RED" : "PASS";
  assert(attempt.classification === computed, `${attempt.attemptId} classification does not match retained overhead evidence`);
  assert(attempt.validRedObserved === (computed === "RED"), `${attempt.attemptId} valid-red marker is false`);
  return true;
}

function validateObserver(payload) {
  exactArray(payload.scenarios, ["rapid-long-jyutping"], "WEB06 observer scenarios");
  assert(Array.isArray(payload.observerTriplets), "WEB06 observer triplets are missing");
  assert(payload.observerTriplets.length >= 1 && payload.observerTriplets.length <= 7, "WEB06 observer triplet attempt count is outside 1..7");
  const contexts = new Set();
  let measured = 0;
  let passes = 0;
  let reds = 0;
  for (let index = 0; index < payload.observerTriplets.length; index += 1) {
    const attempt = payload.observerTriplets[index];
    if (validateObserverTriplet(attempt, index, measured + 1, contexts)) {
      measured += 1;
      if (attempt.classification === "PASS") passes += 1;
      else reds += 1;
    }
    if (measured === 5 && index !== payload.observerTriplets.length - 1) {
      throw new Error("WEB06 observer retained attempts after its terminal measured result");
    }
  }
  assert(payload.measuredTripletCount === measured, "WEB06 observer measured-triplet count is wrong");
  assert(payload.validTripletCount === measured, "WEB06 observer valid-triplet count is wrong");
  const verdict = measured === 5 ? (reds > 0 ? "RED" : "PASS") : "SETUP_NO_GO";
  assert(payload.verdict === verdict, "WEB06 observer verdict does not match retained triplets");
  if (verdict === "SETUP_NO_GO") {
    assert(payload.observerTriplets.length === 7, "WEB06 observer setup no-go did not exhaust seven attempts");
  } else {
    assert(measured === 5, "WEB06 observer did not retain exactly five valid triplets");
  }
  validateExecution(payload, 1);
}

function validateScenarioResult(result, contract, scenarioId) {
  const expectedTarget =
    contract.mode === "BASE/full"
      ? "BASE_FULL"
      : contract.mode === "FINAL/full"
        ? "FINAL_FULL"
        : "FINAL_MINIMAL";
  assert(result?.targetId === expectedTarget, `${scenarioId} target does not match ${contract.mode}`);
  assert(result?.scenarioId === scenarioId, `WEB06 scenario result is out of order: ${scenarioId}`);
  assert(Array.isArray(result.attempts), `${scenarioId} attempts are missing`);
  assert(
    result.attempts.length >= contract.minimumAttempts &&
      result.attempts.length <= contract.maximumAttempts,
    `${scenarioId} attempt count is outside the frozen limit`,
  );
  const attemptIds = new Set();
  for (let index = 0; index < result.attempts.length; index += 1) {
    const attempt = result.attempts[index];
    assert(
      attempt?.attemptId === `attempt-${index + 1}`,
      `${scenarioId} attempt IDs must be contiguous and retained`,
    );
    assert(!attemptIds.has(attempt.attemptId), `${scenarioId} has a duplicate attempt ID`);
    attemptIds.add(attempt.attemptId);
    assert(
      ATTEMPT_CLASSIFICATIONS.has(attempt.classification),
      `${scenarioId}/${attempt.attemptId} has an invalid classification`,
    );
    if (attempt.classification === "SETUP_INVALID") {
      assert(attempt.measurementStarted === false, `${scenarioId}/${attempt.attemptId} setup failure started measurement`);
      assert(attempt.validRedObserved === false, `${scenarioId}/${attempt.attemptId} setup failure hides a valid red`);
      assert(attempt.retryEligible === true, `${scenarioId}/${attempt.attemptId} setup failure is not explicitly retryable`);
      assert(/^SETUP_[A-Z0-9_]+$/.test(attempt.setupFailureCode ?? ""), `${scenarioId}/${attempt.attemptId} setup failure code is invalid`);
    } else if (attempt.classification === "NO_VERDICT_INVALID_CADENCE") {
      assert(attempt.measurementStarted === true, `${scenarioId}/${attempt.attemptId} cadence invalidation did not start measurement`);
      assert(attempt.validRedObserved === false, `${scenarioId}/${attempt.attemptId} cadence invalidation hides a valid red`);
      assert(attempt.retryEligible === true, `${scenarioId}/${attempt.attemptId} cadence invalidation is not explicitly retryable`);
    } else {
      assert(attempt.measurementStarted === true, `${scenarioId}/${attempt.attemptId} measured attempt never started`);
      assert(attempt.measurementCompleted === true, `${scenarioId}/${attempt.attemptId} measured attempt did not complete`);
      assert(attempt.retryEligible === false, `${scenarioId}/${attempt.attemptId} measured result is incorrectly retryable`);
      assert(attempt.validRedObserved === (attempt.classification === "RED"), `${scenarioId}/${attempt.attemptId} valid-red marker is false`);
    }
    for (const field of [
      "rawPacketSha256",
      "runnerRoundSummarySha256",
      "independentRoundSummarySha256",
    ]) {
      assert(SHA64.test(attempt[field] ?? ""), `${scenarioId}/${attempt.attemptId} ${field} is invalid`);
    }
    assert(
      attempt.runnerRoundSummarySha256 === attempt.independentRoundSummarySha256,
      `${scenarioId}/${attempt.attemptId} independent round recomputation differs`,
    );
    assert(["PASS", "RED", "NO_VERDICT"].includes(attempt.commonVerdict), `${scenarioId}/${attempt.attemptId} common verdict is invalid`);
    assert(["PASS", "RED", "NO_VERDICT"].includes(attempt.internalVerdict), `${scenarioId}/${attempt.attemptId} internal verdict is invalid`);
  }
  const measured = result.attempts.filter((attempt) =>
    MEASURED.has(attempt.classification),
  );
  assert(
    measured.length === contract.rounds &&
      result.measuredRoundCount === contract.rounds,
    `${scenarioId} does not retain the exact measured-round count`,
  );
  assert(
    MEASURED.has(result.attempts.at(-1)?.classification),
    `${scenarioId} contains attempts after its fifth measured round`,
  );
  const expectedVerdict = measured.some(
    (attempt) => attempt.classification === "RED",
  )
    ? "RED"
    : "PASS";
  assert(result.verdict === expectedVerdict, `${scenarioId} verdict does not match retained attempts`);
  for (const field of ["internalPoolSha256", "commonPoolSha256", "independentPoolSha256"]) {
    assert(SHA64.test(result[field] ?? ""), `${scenarioId} ${field} is invalid`);
  }
}

export function validateSuiteAttestation(payload, expected) {
  const schema = validateWeb06RunArtifactSchema(
    "suite-attestation.json",
    payload,
  );
  assert(
    schema.pass,
    `WEB06 suite attestation schema failed: ${schema.errors.join(",")}`,
  );
  const expectation = expected?.expectation ?? payload.expectation;
  assert(
    payload.expectation === expectation,
    "WEB06 suite expectation mismatch",
  );
  for (const [field, value] of [
    ["disposition", expected?.disposition],
    ["selectedBranch", expected?.selectedBranch],
    ["identityManifestSha256", expected?.identityManifestSha256],
    ["collectorContractSha256", expected?.collectorContractSha256],
    ["environmentManifestSha256", expected?.environmentManifestSha256],
    ["environmentId", expected?.environmentId],
  ]) {
    if (value !== undefined) {
      assert(
        payload[field] === value,
        `WEB06 suite ${field} does not match its source-bound expectation`,
      );
    }
  }

  const primaryRoleName =
    expectation === "OBSERVER" || expectation === "BASELINE"
      ? "BASE"
      : "FINAL";
  const primaryRole =
    payload.sourceArtifactRoles?.targetRoles?.[primaryRoleName];
  assert(primaryRole !== undefined, "WEB06 suite primary artifact role is missing");
  for (const [field, value] of [
    ["sourceCommit", expected?.sourceCommit],
    ["sourceTree", expected?.sourceTree],
    ["sourceTreeState", expected?.sourceTreeState],
    ["archiveSha256", expected?.archiveSha256],
    ["artifactManifestSha256", expected?.artifactManifestSha256],
    ["buildInfoSha256", expected?.buildInfoSha256],
  ]) {
    if (value !== undefined) {
      assert(
        primaryRole[field] === value,
        `WEB06 suite ${primaryRoleName} ${field} does not match its source-bound expectation`,
      );
    }
  }

  const runnerRole = payload.sourceArtifactRoles?.runnerSource;
  for (const [field, value] of [
    ["sourceCommit", expected?.sourceCommit],
    ["sourceTree", expected?.sourceTree],
    ["sourceTreeState", expected?.sourceTreeState],
  ]) {
    if (value !== undefined) {
      assert(
        runnerRole?.[field] === value,
        `WEB06 suite runner ${field} does not match its source-bound expectation`,
      );
    }
  }

  if (expected?.identityManifest !== undefined) {
    assert(
      sha256StableJson(expected.identityManifest) ===
        payload.identityManifestSha256,
      "WEB06 suite identity manifest bytes do not match its canonical hash",
    );
    assert(
      expected.identityManifest.collectorContractSha256 ===
        payload.collectorContractSha256,
      "WEB06 suite identity manifest collector contract mismatch",
    );
    for (const [roleName, pinned] of Object.entries(
      expected.identityManifest.roles ?? {},
    )) {
      const actual = payload.sourceArtifactRoles?.targetRoles?.[roleName];
      assert(actual !== undefined, `WEB06 identity role ${roleName} is missing`);
      for (const field of [
        "sourceCommit",
        "sourceTree",
        "sourceTreeState",
        "archiveSha256",
        "artifactManifestSha256",
        "buildInfoSha256",
        "selectedBranch",
        "disposition",
      ]) {
        assert(
          actual[field] === pinned[field],
          `WEB06 identity role ${roleName} ${field} mismatch`,
        );
      }
    }
  }

  if (expectation === "OBSERVER" && expected?.observerArtifacts !== undefined) {
    for (const roleName of ["PRODUCT", "BASE"]) {
      const actual = payload.sourceArtifactRoles?.targetRoles?.[roleName];
      const pinned = expected.observerArtifacts[roleName];
      assert(actual !== undefined && pinned !== undefined, `WEB06 observer ${roleName} role is missing`);
      for (const field of [
        "sourceCommit",
        "sourceTree",
        "sourceTreeState",
        "archiveSha256",
        "artifactManifestSha256",
        "buildInfoSha256",
      ]) {
        assert(
          actual[field] === pinned[field],
          `WEB06 observer ${roleName} ${field} mismatch`,
        );
      }
    }
  }

  if (payload.disposition === "PRODUCTION_REDUCTION") {
    assert(
      payload.measurementCompleted === true && payload.verdict === "PASS",
      "WEB06 production reduction requires a completed PASS suite",
    );
  }
  if (payload.disposition === "MEASURED_NO_GO") {
    assert(
      payload.selectedBranch === "NONE" &&
        payload.measurementCompleted === true &&
        payload.verdict === "RED",
      "WEB06 measured no-go requires a completed RED suite with no branch",
    );
  }
  return payload;
}

function assertDeepEqual(actual, expected, message) {
  assert(isDeepStrictEqual(actual, expected), message);
}

function safeRelativeSegments(relativePath, label) {
  assert(
    typeof relativePath === "string" &&
      relativePath !== "" &&
      !relativePath.includes("\\") &&
      !path.posix.isAbsolute(relativePath),
    `${label} path is unsafe`,
  );
  const segments = relativePath.split("/");
  assert(
    segments.every(
      (segment) =>
        segment !== "" &&
        segment !== "." &&
        segment !== ".." &&
        /^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(segment),
    ),
    `${label} path is unsafe`,
  );
  return segments;
}

async function canonicalEvidenceRoot(evidenceRoot) {
  assert(
    typeof evidenceRoot === "string" && evidenceRoot !== "",
    "WEB06 evidence root is missing",
  );
  const resolved = path.resolve(evidenceRoot);
  const metadata = await lstat(resolved);
  assert(
    metadata.isDirectory() && !metadata.isSymbolicLink(),
    "WEB06 evidence root must be a plain directory",
  );
  const canonical = await realpath(resolved);
  assert(
    canonical === resolved,
    "WEB06 evidence root must be canonical and contain no symlink",
  );
  return canonical;
}

function relativePathWithinRoot(canonicalRoot, candidate, label) {
  const resolved = path.resolve(candidate);
  const relative = path.relative(canonicalRoot, resolved);
  assert(
    relative !== "" &&
      relative !== ".." &&
      !relative.startsWith(`..${path.sep}`) &&
      !path.isAbsolute(relative),
    `${label} escaped its evidence root`,
  );
  return relative.split(path.sep);
}

async function readPlainJsonBeneathRoot(
  canonicalRoot,
  relativeSegments,
  label,
  reference,
) {
  let candidate = canonicalRoot;
  for (let index = 0; index < relativeSegments.length; index += 1) {
    candidate = path.join(candidate, relativeSegments[index]);
    const metadata = await lstat(candidate);
    const final = index === relativeSegments.length - 1;
    assert(!metadata.isSymbolicLink(), `${label} path contains a symlink`);
    assert(
      final ? metadata.isFile() : metadata.isDirectory(),
      `${label} path has an invalid file type`,
    );
    assert(
      (await realpath(candidate)) === candidate,
      `${label} path is not canonical`,
    );
  }
  const handle = await open(
    candidate,
    fsConstants.O_RDONLY | (fsConstants.O_NOFOLLOW ?? 0),
  );
  let bytes;
  try {
    const metadata = await handle.stat();
    assert(metadata.isFile(), `${label} must be a plain file`);
    bytes = await handle.readFile();
    const finalMetadata = await handle.stat();
    assert(
      finalMetadata.isFile() && finalMetadata.size === bytes.byteLength,
      `${label} changed while it was read`,
    );
  } finally {
    await handle.close();
  }
  const digest = sha256Bytes(bytes);
  if (reference !== undefined) {
    assert(
      bytes.byteLength === reference.bytes,
      `${label} byte count changed`,
    );
    assert(digest === reference.sha256, `${label} bytes changed`);
  }
  let payload;
  try {
    payload = JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new Error(`${label} is not valid JSON`);
  }
  return Object.freeze({
    path: candidate,
    payload,
    bytes,
    sha256: digest,
  });
}

function validateArtifactSchema(fileName, payload, label) {
  const schema = validateWeb06RunArtifactSchema(fileName, payload);
  assert(schema.pass, `${label} schema failed: ${schema.errors.join(",")}`);
}

function validateSharedArtifactIdentity(attestation, collector, independent) {
  for (const field of [
    "expectation",
    "disposition",
    "selectedBranch",
    "identityManifestSha256",
    "collectorContractSha256",
    "environmentManifestSha256",
    "environmentId",
  ]) {
    assert(
      collector[field] === attestation[field] &&
        independent[field] === attestation[field],
      `WEB06 artifact-set ${field} mismatch`,
    );
  }
}

function validateRunnerSourceBinding(attestation, collector) {
  const role = attestation.sourceArtifactRoles?.runnerSource;
  const manifest = collector.runnerSourceManifest;
  assert(role !== undefined && manifest !== undefined, "WEB06 runner source binding is missing");
  for (const field of ["sourceCommit", "sourceTree", "sourceTreeState"]) {
    assert(
      role[field] === manifest[field],
      `WEB06 runner source ${field} mismatch`,
    );
  }
  assert(
    role.toolingManifestSha256 === manifest.toolingManifestSha256,
    "WEB06 runner tooling manifest mismatch",
  );
  assert(
    role.sourceManifestSha256 === collector.runnerSourceManifestSha256 &&
      attestation.runnerSourceManifestSha256 ===
        collector.runnerSourceManifestSha256,
    "WEB06 runner source manifest mismatch",
  );
  assert(
    role.beforeObservationSha256 ===
        collector.runnerSourceObservationSha256 &&
      attestation.runnerSourceObservationSha256 ===
        collector.runnerSourceObservationSha256,
    "WEB06 runner source pre-observation mismatch",
  );
  assert(
    role.afterObservationSha256 ===
        collector.runnerSourcePostObservationSha256 &&
      attestation.runnerSourcePostObservationSha256 ===
        collector.runnerSourcePostObservationSha256,
    "WEB06 runner source post-observation mismatch",
  );
}

export async function verifyWeb06SuiteArtifactSet({
  attestationPath,
  evidenceRoot,
  expected = {},
  repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../.."),
  verifyCurrentSource = true,
}) {
  const canonicalRoot = await canonicalEvidenceRoot(evidenceRoot);
  const attestation = await readPlainJsonBeneathRoot(
    canonicalRoot,
    relativePathWithinRoot(
      canonicalRoot,
      attestationPath,
      "WEB06 suite attestation",
    ),
    "WEB06 suite attestation",
  );
  validateSuiteAttestation(attestation.payload, expected);
  validateEvidenceReference(
    attestation.payload.collectorOutput,
    "collector-output.json",
    "WEB06 collector output",
  );
  validateEvidenceReference(
    attestation.payload.independentRecompute,
    "independent-recompute.json",
    "WEB06 independent recompute",
  );
  const collector = await readPlainJsonBeneathRoot(
    canonicalRoot,
    safeRelativeSegments(
      attestation.payload.collectorOutput.relativePath,
      "WEB06 collector output",
    ),
    "WEB06 collector output",
    attestation.payload.collectorOutput,
  );
  const independent = await readPlainJsonBeneathRoot(
    canonicalRoot,
    safeRelativeSegments(
      attestation.payload.independentRecompute.relativePath,
      "WEB06 independent recompute",
    ),
    "WEB06 independent recompute",
    attestation.payload.independentRecompute,
  );

  validateArtifactSchema(
    "collector-output.json",
    collector.payload,
    "WEB06 collector output",
  );
  validateArtifactSchema(
    "independent-recompute.json",
    independent.payload,
    "WEB06 independent recompute",
  );
  const independentSchema = validateIndependentRecomputeSchema(
    independent.payload,
  );
  assert(
    independentSchema.pass,
    `WEB06 independent verifier schema failed: ${independentSchema.errors.join(",")}`,
  );
  validateSharedArtifactIdentity(
    attestation.payload,
    collector.payload,
    independent.payload,
  );
  validateRunnerSourceBinding(attestation.payload, collector.payload);
  assert(
    independent.payload.collectorOutputSha256 === collector.sha256,
    "WEB06 independent recompute is not bound to the collector bytes",
  );
  assertDeepEqual(
    attestation.payload.scenarios,
    collector.payload.scenarioRuns,
    "WEB06 attestation scenarios differ from collector output",
  );
  assertDeepEqual(
    attestation.payload.scenarioResults,
    collector.payload.scenarioResults,
    "WEB06 attestation scenario results differ from collector output",
  );
  assertDeepEqual(
    attestation.payload.observerTriplets,
    collector.payload.observerTriplets,
    "WEB06 attestation observer triplets differ from collector output",
  );
  assertDeepEqual(
    attestation.payload.execution,
    collector.payload.execution,
    "WEB06 attestation execution differs from collector output",
  );
  assert(
    attestation.payload.measurementStarted ===
        collector.payload.measurementStarted &&
      attestation.payload.measurementCompleted ===
        collector.payload.measurementCompleted,
    "WEB06 attestation measurement state differs from collector output",
  );

  const recomputed = await verifyCollectorOutput({
    evidenceRoot: canonicalRoot,
    collectorOutputPath: collector.path,
    repoRoot,
    verifyCurrentSource,
  });
  const recomputedSchema = validateIndependentRecomputeSchema(recomputed);
  assert(
    recomputedSchema.pass,
    `WEB06 recomputed verifier schema failed: ${recomputedSchema.errors.join(",")}`,
  );
  assertDeepEqual(
    independent.payload,
    recomputed,
    "WEB06 referenced independent recompute differs from verifier output",
  );
  assert(
    independent.bytes.equals(
      Buffer.from(`${JSON.stringify(recomputed, null, 2)}\n`, "utf8"),
    ),
    "WEB06 referenced independent recompute bytes are not canonical verifier output",
  );
  if (attestation.payload.expectation === "OBSERVER") {
    assertDeepEqual(
      collector.payload.observerTriplets,
      recomputed.observerTriplets,
      "WEB06 observer triplets differ from recomputed output",
    );
    assertDeepEqual(
      collector.payload.observerEvaluation,
      recomputed.observerEvaluation,
      "WEB06 observer evaluation differs from recomputed output",
    );
  }

  return Object.freeze({
    payload: attestation.payload,
    bytes: attestation.bytes,
    sha256: attestation.sha256,
    collector: Object.freeze({
      payload: collector.payload,
      bytes: collector.bytes,
      sha256: collector.sha256,
    }),
    independent: Object.freeze({
      payload: independent.payload,
      bytes: independent.bytes,
      sha256: independent.sha256,
    }),
    recomputed,
  });
}

export async function readAndValidateSuiteAttestation(file, expected) {
  assert(
    expected?.evidenceRoot !== undefined,
    "WEB06 suite artifact-set verification requires evidenceRoot",
  );
  return verifyWeb06SuiteArtifactSet({
    attestationPath: file,
    evidenceRoot: expected.evidenceRoot,
    expected,
    repoRoot: expected.repoRoot,
    verifyCurrentSource: expected.verifyCurrentSource ?? true,
  });
}
